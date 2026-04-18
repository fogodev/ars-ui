//! Mutable collection wrappers with change tracking for DOM reconciliation.
//!
//! Adapters (Leptos, Dioxus, …) need to perform targeted DOM updates when a
//! collection mutates, rather than re-rendering every item. The wrappers in
//! this module — [`MutableListData`] and [`MutableTreeData`] — delegate all
//! read-only [`Collection`] access to an inner [`StaticCollection`] or
//! [`TreeCollection`] while recording each mutation as a [`CollectionChange`]
//! in an internal buffer.
//!
//! After an update cycle, the adapter calls [`MutableListData::drain_changes`]
//! / [`MutableTreeData::drain_changes`] to retrieve and clear the buffered
//! events, and then applies them as granular DOM mutations (insert / remove /
//! move / replace / reset).
//!
//! Spec reference: `spec/foundation/06-collections.md` §1.8 "Mutable
//! Collections".

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    mem,
};

use crate::{
    collection::{Collection, CollectionItem},
    key::Key,
    node::Node,
    static_collection::StaticCollection,
    tree_collection::TreeCollection,
};

// ---------------------------------------------------------------------------
// CollectionChange
// ---------------------------------------------------------------------------

/// A granular change event emitted when a mutable collection is modified.
///
/// Adapters consume a sequence of these events to perform targeted DOM
/// updates instead of re-rendering the entire list. The generic key type `K`
/// is parameterised so tests and alternative collection implementations can
/// use keys other than [`Key`]; production wrappers always use
/// `CollectionChange<Key>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "K: Clone + serde::Serialize",
        deserialize = "K: Clone + serde::de::DeserializeOwned"
    ))
)]
pub enum CollectionChange<K: Clone> {
    /// New items inserted at the given flat index.
    ///
    /// `count` is always `1` for single-item mutations but is kept as a field
    /// so future bulk inserts can share the variant without a breaking API
    /// change.
    Insert {
        /// Flat insertion index in the collection's iteration order.
        index: usize,

        /// Number of items inserted starting at `index`.
        count: usize,
    },

    /// Items with the given keys were removed.
    Remove {
        /// Keys of the removed items, in the order the caller supplied them.
        keys: Vec<K>,
    },

    /// An item moved from one flat index to another.
    Move {
        /// Key of the moved item.
        key: K,

        /// Flat index the item occupied before the move.
        from_index: usize,

        /// Flat index the item occupies after the move.
        to_index: usize,
    },

    /// The node at this key needs to be re-rendered in place — its
    /// position in the iteration is unchanged.
    ///
    /// Emitted in two situations:
    ///
    /// * `replace()` swapped the item's payload `T`, leaving the key
    ///   and surrounding structure untouched.
    /// * A non-payload mutation flipped the node's rendered state
    ///   while leaving the iteration order intact — for example,
    ///   inserting a child under a previously-leaf tree node turns
    ///   that parent into a collapsed branch (`has_children: false →
    ///   true`, `is_expanded: None → Some(false)`), which adapters
    ///   key DOM off (expander chevron, `aria-expanded`).
    ///
    /// Adapters handle both cases identically: re-fetch the node by
    /// key and re-render it in place.
    Replace {
        /// Key of the node whose rendered state changed.
        key: K,
    },

    /// The entire collection was reset (e.g. bulk replacement or clear).
    ///
    /// Adapters should re-render all items instead of trying to reconcile
    /// individual changes.
    Reset,
}

// ---------------------------------------------------------------------------
// MutableListData
// ---------------------------------------------------------------------------

/// A mutable flat-list collection that tracks granular changes.
///
/// Wraps a [`StaticCollection`] and records every mutation as a
/// [`CollectionChange`] in `pending_changes`. The adapter drains this buffer
/// each update cycle via [`MutableListData::drain_changes`].
pub struct MutableListData<T: CollectionItem> {
    /// The inner collection holding the canonical item data.
    inner: StaticCollection<T>,

    /// Pending change events to be drained by the adapter layer.
    pending_changes: Vec<CollectionChange<Key>>,
}

/// Manual `Debug` avoids requiring `T: Debug`, matching [`StaticCollection`]'s
/// approach. Prints the inner collection and pending-change count only.
impl<T: CollectionItem> Debug for MutableListData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutableListData")
            .field("inner", &self.inner)
            .field("pending_changes", &self.pending_changes.len())
            .finish()
    }
}

impl<T: CollectionItem> MutableListData<T> {
    /// Wrap an existing [`StaticCollection`] so subsequent mutations emit
    /// change events. The change buffer starts empty — construction itself
    /// is not reported as a change.
    #[must_use]
    pub const fn new(collection: StaticCollection<T>) -> Self {
        Self {
            inner: collection,
            pending_changes: Vec::new(),
        }
    }

    /// Append an item to the end of the collection. Emits
    /// [`CollectionChange::Insert`] with `count: 1` at the appended index.
    pub fn push(&mut self, item: T) {
        let index = self.inner.len();

        self.inner.insert(index, item);

        self.pending_changes
            .push(CollectionChange::Insert { index, count: 1 });
    }

    /// Insert an item at the given flat index, shifting subsequent items.
    /// Emits [`CollectionChange::Insert`] with `count: 1`.
    pub fn insert(&mut self, index: usize, item: T) {
        self.inner.insert(index, item);

        self.pending_changes
            .push(CollectionChange::Insert { index, count: 1 });
    }

    /// Remove items with the given keys, returning their owned item values
    /// in iteration order. Emits [`CollectionChange::Remove`] carrying every
    /// **input key that actually existed** in the collection (deduped, in
    /// input order) — unknown keys are silently skipped by the inner
    /// collection, so they are excluded from the change event to keep the
    /// change log truthful. Returns an empty `Vec` of payloads (and emits
    /// no event) when no key in `keys` matches.
    ///
    /// The change event must be derived from existence-before-mutation, not
    /// from the returned `Vec<T>`: [`StaticCollection::remove_by_keys`]
    /// drops structural nodes ([`Section`], [`Header`], [`Separator`])
    /// silently from its return because they carry no payload, even though
    /// removing one mutates the collection and reindexes every later
    /// position. Without the snapshot, an adapter reconciling from
    /// `CollectionChange` would leave a phantom row in the DOM.
    ///
    /// [`Section`]: crate::node::NodeType::Section
    /// [`Header`]: crate::node::NodeType::Header
    /// [`Separator`]: crate::node::NodeType::Separator
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        // Snapshot which input keys are genuinely going to mutate the
        // collection. We dedup as we go so a duplicated input key only
        // appears once in the change event — that mirrors the inner
        // method's behaviour (the second `remove_by_keys` pass over an
        // already-removed key is a no-op).
        let mut seen = BTreeSet::<Key>::new();

        let matched_keys = keys
            .iter()
            .filter(|k| seen.insert((*k).clone()) && self.inner.index_of(k).is_some())
            .cloned()
            .collect::<Vec<_>>();

        let removed = self.inner.remove_by_keys(keys);

        if !matched_keys.is_empty() {
            self.pending_changes
                .push(CollectionChange::Remove { keys: matched_keys });
        }

        removed
    }

    /// Move an item identified by `key` to the given flat index.
    ///
    /// Returns `Some((from_flat_index, to_flat_index))` on success, or
    /// `None` if `key` is not present in the collection.
    /// [`CollectionChange::Move`] is emitted only on success, so the change
    /// log never references a key the collection does not contain.
    pub fn move_item(&mut self, key: &Key, to_index: usize) -> Option<(usize, usize)> {
        let from_index = self.inner.index_of(key)?;

        self.inner.move_item(from_index, to_index);

        self.pending_changes.push(CollectionChange::Move {
            key: key.clone(),
            from_index,
            to_index,
        });

        Some((from_index, to_index))
    }

    /// Replace an item's data in place.
    ///
    /// Returns the previous value at `item.key()` on success, or `None` if
    /// the key is not present. The `Replace` change event is emitted only
    /// on success, so the change log never references a key the collection
    /// does not contain.
    pub fn replace(&mut self, item: T) -> Option<T> {
        let key = item.key().clone();

        let old = self.inner.replace(item);

        if old.is_some() {
            self.pending_changes.push(CollectionChange::Replace { key });
        }

        old
    }

    /// Remove every item and emit a single [`CollectionChange::Reset`].
    ///
    /// Any earlier pending events (`Insert`, `Remove`, `Move`, `Replace`)
    /// are discarded before pushing `Reset` so the buffer is always
    /// `[Reset]` after a clear. Coalescing matters because:
    ///
    /// * `Insert { index, count }` does not carry the inserted payload —
    ///   replaying a stale `Insert` against the now-empty inner collection
    ///   would let the adapter reference an item that no longer exists.
    /// * `Reset` is by definition a full-rebuild signal, so any change
    ///   that preceded it within the same update cycle is invisible to the
    ///   adapter anyway.
    ///
    /// Adapters treat the resulting `Reset` as a hint to re-render the
    /// full list from scratch.
    pub fn clear(&mut self) {
        self.inner.clear();

        // Drop stale events queued earlier in this cycle: they describe
        // intermediate states the adapter never observes.
        self.pending_changes.clear();
        self.pending_changes.push(CollectionChange::Reset);
    }

    /// Take the pending change log, leaving the buffer empty. Called by the
    /// adapter once per update cycle.
    pub fn drain_changes(&mut self) -> Vec<CollectionChange<Key>> {
        mem::take(&mut self.pending_changes)
    }
}

// The `Collection<T>` delegation requires `T: Clone` because the inner
// `StaticCollection<T>` only implements `Collection<T>` when `T: Clone`
// (see `static_collection.rs`). Mutation methods above are available for
// any `T: CollectionItem`.
impl<T: CollectionItem + Clone> Collection<T> for MutableListData<T> {
    fn size(&self) -> usize {
        self.inner.size()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.inner.get(key)
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.inner.get_by_index(index)
    }

    fn first_key(&self) -> Option<&Key> {
        self.inner.first_key()
    }

    fn last_key(&self) -> Option<&Key> {
        self.inner.last_key()
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after(key)
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before(key)
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after_no_wrap(key)
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before_no_wrap(key)
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.inner.keys()
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.nodes()
    }

    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.children_of(parent_key)
    }
}

// ---------------------------------------------------------------------------
// MutableTreeData
// ---------------------------------------------------------------------------

/// A mutable tree collection that tracks granular changes.
///
/// Wraps a [`TreeCollection`] and records every mutation — including
/// reparenting and sibling reordering — as a [`CollectionChange`] using
/// flat DFS indices. The adapter drains this buffer each update cycle via
/// [`MutableTreeData::drain_changes`].
pub struct MutableTreeData<T: CollectionItem> {
    /// The inner tree holding the canonical hierarchy.
    inner: TreeCollection<T>,

    /// Pending change events to be drained by the adapter layer.
    pending_changes: Vec<CollectionChange<Key>>,
}

/// Manual `Debug` avoids requiring `T: Debug`, matching [`TreeCollection`]'s
/// approach. Prints the inner tree and pending-change count only.
impl<T: CollectionItem> Debug for MutableTreeData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutableTreeData")
            .field("inner", &self.inner)
            .field("pending_changes", &self.pending_changes.len())
            .finish()
    }
}

impl<T: CollectionItem> MutableTreeData<T> {
    /// Wrap an existing [`TreeCollection`] so subsequent mutations emit
    /// change events. The change buffer starts empty.
    #[must_use]
    pub const fn new(collection: TreeCollection<T>) -> Self {
        Self {
            inner: collection,
            pending_changes: Vec::new(),
        }
    }

    /// Insert a child under `parent` at the given sibling index (or at the
    /// root when `parent` is `None`).
    ///
    /// Returns the flat DFS index of the inserted node on success, or
    /// `None` if the inner [`TreeCollection::insert_child`] rejected the
    /// insert (unknown parent key).
    ///
    /// # Visibility-aware change events
    ///
    /// The wrapper emits at most one of:
    ///
    /// * `Insert { index: visible_index, count: 1 }` — when the new
    ///   child lands in the visible iteration. A hidden insert (one
    ///   under a collapsed ancestor) emits no `Insert`; the hidden
    ///   node surfaces naturally when an ancestor later expands and
    ///   the adapter re-fetches the expanded subtree.
    /// * `Replace { key: parent_key }` — when the parent flipped from
    ///   leaf to collapsed branch. [`TreeCollection::insert_child`]
    ///   sets `has_children = true` and `is_expanded = Some(false)` on
    ///   a previously-leaf parent, which adapters key DOM off
    ///   (expander chevron, `aria-expanded`). The transition only
    ///   needs surfacing when the parent is itself visible — a hidden
    ///   parent's row is not rendered, so the metadata change is
    ///   invisible until an ancestor expands and the adapter re-reads
    ///   the parent.
    ///
    /// The two events are mutually exclusive: a leaf parent always
    /// becomes collapsed, so its new child is hidden and never
    /// produces an `Insert`. A parent that was already an expanded
    /// branch keeps its visible state and surfaces the new child via
    /// `Insert` alone.
    pub fn insert_child(&mut self, parent: Option<&Key>, index: usize, item: T) -> Option<usize> {
        let key = item.key().clone();

        // Snapshot whether the parent (if any) is currently a visible
        // leaf, before the inner call flips its `has_children` /
        // `is_expanded` metadata. We only care about visible leaves —
        // hidden leaves transition the same way internally, but their
        // row isn't rendered so the metadata change has no DOM impact
        // until an ancestor expands.
        let visible_leaf_parent = parent
            .filter(|p| self.inner.visible_index_of(p).is_some() && !self.inner.has_children(p));

        let flat_index = self.inner.insert_child(parent, index, item)?;

        if let Some(visible_index) = self.inner.visible_index_of(&key) {
            // The new child is part of the visible iteration. The
            // parent therefore couldn't have been a leaf (a leaf
            // parent would have collapsed the new child), so no
            // Replace fires alongside.
            self.pending_changes.push(CollectionChange::Insert {
                index: visible_index,
                count: 1,
            });
        } else if let Some(parent_key) = visible_leaf_parent {
            // Hidden child + previously-visible-leaf parent → the
            // parent is now a collapsed branch and needs its rendered
            // chevron / `aria-expanded` updated. Emit Replace so a
            // reconciler that consumes only `CollectionChange` picks
            // up the new metadata.
            self.pending_changes.push(CollectionChange::Replace {
                key: parent_key.clone(),
            });
        }

        Some(flat_index)
    }

    /// Remove the listed nodes (and their descendants), returning their
    /// owned item values.
    ///
    /// Emits up to two kinds of event, in DFS order:
    ///
    /// * [`CollectionChange::Remove`] — carrying every key that
    ///   disappeared from the **visible iteration** as a result of the
    ///   call. That includes:
    ///     - the input keys that actually matched a node,
    ///     - cascade-removed descendants that were rendered, and
    ///     - structural nodes ([`Section`], [`Header`], [`Separator`]),
    ///       which carry no payload and would never appear in the
    ///       returned `Vec<T>`.
    ///
    ///   Hidden nodes (those inside a collapsed ancestor) were never
    ///   rendered, so they stay out of the change event — emitting them
    ///   would force the adapter to no-op and pollute the truthful
    ///   change log.
    ///
    /// * [`CollectionChange::Replace`] — one event per still-visible
    ///   parent whose [`has_children`] flipped from `true` to `false`
    ///   as a side effect of the removal. A parent transitions from a
    ///   branch to a leaf when the removal drains its last remaining
    ///   child (visible *or* hidden); the inner tree flips
    ///   `has_children: true → false` and `is_expanded: Some(_) → None`
    ///   on that node. Adapters key expander chevrons and
    ///   `aria-expanded` off those flags, so the parent row must be
    ///   re-rendered even when none of its visible children appeared in
    ///   the `Remove` event. Parents that are themselves hidden (inside
    ///   a collapsed ancestor) are skipped: their row isn't rendered,
    ///   so the metadata change is invisible until an ancestor expands
    ///   and the adapter re-reads them.
    ///
    /// Remove events precede Replace events in the drained buffer, and
    /// Replace events are emitted in pre-mutation DFS order so the
    /// event trace is stable for keyed reconcilers.
    ///
    /// The Remove diff is computed against `visible_keys()` before and
    /// after the inner mutation rather than from the returned `Vec<T>`,
    /// both to capture cascaded descendants and to surface structural
    /// removals that have no payload to enumerate. The branch→leaf
    /// detection reuses the same pre-mutation snapshot.
    ///
    /// [`has_children`]: crate::node::Node::has_children
    /// [`Section`]: crate::node::NodeType::Section
    /// [`Header`]: crate::node::NodeType::Header
    /// [`Separator`]: crate::node::NodeType::Separator
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        // `visible_keys` is the inherent (Clone-free) twin of
        // `Collection::keys`, available because `MutableTreeData::remove`
        // only requires `T: CollectionItem`. Capturing the iteration
        // order in a `Vec` (not a `BTreeSet`) lets us preserve DFS
        // order in the change event. The parallel `BTreeMap` snapshot
        // records branch/leaf state for each visible key so the
        // post-mutation loop can detect parents whose last child was
        // drained by this call.
        let visible_before = self.inner.visible_keys().cloned().collect::<Vec<_>>();

        let visible_before_has_children = visible_before
            .iter()
            .map(|k| (k.clone(), self.inner.has_children(k)))
            .collect::<BTreeMap<_, _>>();

        let removed = self.inner.remove_by_keys(keys);

        let visible_after = self.inner.visible_keys().cloned().collect::<BTreeSet<_>>();

        let visible_removed_keys = visible_before
            .iter()
            .filter(|k| !visible_after.contains(*k))
            .cloned()
            .collect::<Vec<_>>();

        if !visible_removed_keys.is_empty() {
            self.pending_changes.push(CollectionChange::Remove {
                keys: visible_removed_keys,
            });
        }

        // Emit Replace for any still-visible parent whose `has_children`
        // flipped from true to false. Removal can only lose children,
        // never gain them, so the leaf→branch direction is not
        // considered here. Iterating `visible_before` keeps the
        // emission order DFS-stable and matches how the Remove event
        // itself is ordered.
        for k in &visible_before {
            if !visible_after.contains(k) {
                continue;
            }

            let was_branch = visible_before_has_children.get(k).copied().unwrap_or(false);

            if was_branch && !self.inner.has_children(k) {
                self.pending_changes
                    .push(CollectionChange::Replace { key: k.clone() });
            }
        }

        removed
    }

    /// Move a node (with its subtree) under a new parent at the given
    /// sibling index.
    ///
    /// Returns `Some((from_flat_index, to_flat_index))` on success, or
    /// `None` when the inner [`TreeCollection::reparent`] rejected the
    /// move (unknown key, unknown `new_parent`, or `new_parent` is a
    /// descendant of `key`).
    ///
    /// # Visibility-aware change events
    ///
    /// The visibility of the moved node — and of every descendant in
    /// its subtree — may differ between the old and new locations
    /// (e.g. moving an expanded subtree under a collapsed parent hides
    /// every node it contained, while moving a hidden but internally
    /// expanded subtree into a visible location surfaces all of it at
    /// once). A single `Move` event is therefore not always sufficient
    /// to describe the DOM impact. The wrapper picks the event shape
    /// that matches the subtree-visibility transition:
    ///
    /// | From visible | To visible | Event emitted                                                                    |
    /// | ------------ | ---------- | -------------------------------------------------------------------------------- |
    /// | yes          | yes        | `Move { key, from: vis_from, to: vis_to }`                                       |
    /// | yes          | no         | `Remove { keys: <previously-visible subtree keys, DFS order> }`                  |
    /// | no           | yes        | `Insert { index: vis_to, count: <number of newly-visible subtree nodes> }`       |
    /// | no           | no         | *(no subtree event — neither location is rendered)*                              |
    ///
    /// In all cases the indices are **visible iteration indices**, not
    /// flat DFS indices, so the adapter can apply them directly against
    /// [`Collection::get_by_index`]. The `Insert` always spans
    /// `count` consecutive positions starting at `to_index`.
    ///
    /// # Parent metadata transitions
    ///
    /// Reparenting can also flip [`has_children`] on the old and new
    /// parents independently of the moved subtree's own visibility:
    ///
    /// * the **old parent** transitions branch → leaf when the moved
    ///   node was its only remaining child (`has_children: true →
    ///   false`, `is_expanded: Some(_) → None`);
    /// * the **new parent** transitions leaf → collapsed branch when it
    ///   had no children before (`has_children: false → true`,
    ///   `is_expanded: None → Some(false)`).
    ///
    /// These transitions change the parent row's rendered state
    /// (expander chevron, `aria-expanded`) without changing its
    /// iteration position, so they are surfaced as
    /// [`CollectionChange::Replace`] events keyed by the affected
    /// parent — one per still-visible parent that flipped. The
    /// transition is silent when the affected parent is itself hidden
    /// inside a collapsed ancestor, since its row is not rendered.
    ///
    /// The subtree event (if any) is emitted first, followed by the
    /// parent `Replace` events in pre-mutation DFS order. The moved
    /// key itself cannot flip `has_children` through a reparent — the
    /// subtree moves as a unit — so it never triggers a `Replace` of
    /// its own even when it appears in both the pre- and post-mutation
    /// visible set.
    ///
    /// [`has_children`]: crate::node::Node::has_children
    pub fn reparent(
        &mut self,
        key: &Key,
        new_parent: Option<&Key>,
        index: usize,
    ) -> Option<(usize, usize)> {
        // Snapshot the pre-mutation visible-key set in DFS order and
        // the branch/leaf flag for each visible key.
        //
        // Both visibility-crossing subtree branches need to compare
        // pre/post visibility to capture every subtree descendant that
        // flipped — only the moved subtree's visibility can change, so
        // a set difference against the full visible iteration is
        // exactly the subtree delta. The same snapshot doubles as a
        // membership set for the hidden→visible count and as the
        // source of pre-mutation `has_children` state for the parent
        // metadata-transition detection.
        let visible_before = self.inner.visible_keys().cloned().collect::<Vec<_>>();

        let visible_before_has_children = visible_before
            .iter()
            .map(|k| (k.clone(), self.inner.has_children(k)))
            .collect::<BTreeMap<_, _>>();

        let from_visible = self.inner.visible_index_of(key);

        let (from_flat, to_flat) = self.inner.reparent(key, new_parent, index)?;

        let to_visible = self.inner.visible_index_of(key);

        let visible_after = self.inner.visible_keys().cloned().collect::<BTreeSet<_>>();

        match (from_visible, to_visible) {
            (Some(from_index), Some(to_index)) => {
                self.pending_changes.push(CollectionChange::Move {
                    key: key.clone(),
                    from_index,
                    to_index,
                });
            }

            (Some(_), None) => {
                // The moved subtree's root and every visible descendant
                // disappear together under the collapsed new parent.
                // Emit Remove with all of them in pre-mutation DFS
                // order so a keyed reconciler can drop the matching
                // DOM rows in one pass — emitting only `key` would
                // leave orphan descendants in the DOM/state.
                let removed_keys = visible_before
                    .iter()
                    .filter(|k| !visible_after.contains(*k))
                    .cloned()
                    .collect::<Vec<_>>();

                self.pending_changes
                    .push(CollectionChange::Remove { keys: removed_keys });
            }

            (None, Some(to_index)) => {
                // The root and any descendants whose own expansion
                // state keeps them visible under the new parent appear
                // in `count` consecutive visible positions starting at
                // `to_index`. Emitting `count: 1` would leave adapters
                // that honour `count` under-applying the change and
                // drifting subsequent indices. Reuse
                // `visible_before_has_children` as a membership set —
                // `contains_key` is O(log n) and avoids a second
                // allocation.
                let count = self
                    .inner
                    .visible_keys()
                    .filter(|k| !visible_before_has_children.contains_key(*k))
                    .count();

                self.pending_changes.push(CollectionChange::Insert {
                    index: to_index,
                    count,
                });
            }

            (None, None) => {
                // Both subtree endpoints hidden — no subtree event.
                // Parent-metadata transitions may still fire below.
            }
        }

        // Emit Replace for any still-visible key whose `has_children`
        // flipped in either direction. Iteration follows pre-mutation
        // DFS order so the event trace is stable and matches the
        // ordering used elsewhere.
        //
        // This loop is what surfaces `old_parent` losing its last
        // child (branch → leaf) and `new_parent` gaining its first
        // child (leaf → branch). The moved `key` itself cannot flip
        // here — its subtree moves as a unit, so its own
        // `has_children` is invariant under reparent — which means
        // even if it appears in both visible sets the branch-state
        // comparison short-circuits.
        for k in visible_before {
            if !visible_after.contains(&k) {
                continue;
            }

            let was_branch = visible_before_has_children
                .get(&k)
                .copied()
                .unwrap_or(false);

            let is_branch = self.inner.has_children(&k);

            if was_branch != is_branch {
                self.pending_changes
                    .push(CollectionChange::Replace { key: k });
            }
        }

        Some((from_flat, to_flat))
    }

    /// Reorder a node among its existing siblings (same parent).
    ///
    /// Returns `Some((from_flat_index, to_flat_index))` on success, or
    /// `None` when `key` does not exist in the tree.
    ///
    /// # Visibility-aware change events
    ///
    /// Reordering preserves the parent and therefore preserves
    /// visibility — either both endpoints are visible (the parent is
    /// expanded) or both are hidden (the parent is collapsed). In the
    /// visible case `CollectionChange::Move` is emitted using **visible
    /// iteration indices**; in the hidden case no event is emitted.
    pub fn reorder(&mut self, key: &Key, to_sibling_index: usize) -> Option<(usize, usize)> {
        let from_visible = self.inner.visible_index_of(key);

        let (from_flat, to_flat) = self.inner.reorder_sibling(key, to_sibling_index)?;

        if let (Some(from_index), Some(to_index)) = (from_visible, self.inner.visible_index_of(key))
        {
            self.pending_changes.push(CollectionChange::Move {
                key: key.clone(),
                from_index,
                to_index,
            });
        }

        Some((from_flat, to_flat))
    }

    /// Replace a node's data in place. Children are preserved.
    ///
    /// Returns the previous value at `item.key()` on success, or `None` if
    /// the key is not present. The `Replace` change event is emitted only
    /// on success, so the change log never references a key the tree does
    /// not contain.
    pub fn replace(&mut self, item: T) -> Option<T> {
        let key = item.key().clone();

        let old = self.inner.replace(item);

        if old.is_some() {
            self.pending_changes.push(CollectionChange::Replace { key });
        }

        old
    }

    /// Take the pending change log, leaving the buffer empty.
    pub fn drain_changes(&mut self) -> Vec<CollectionChange<Key>> {
        mem::take(&mut self.pending_changes)
    }
}

// The `Collection<T>` delegation requires `T: Clone` for the same reason as
// `MutableListData` — see that impl's note.
impl<T: CollectionItem + Clone> Collection<T> for MutableTreeData<T> {
    fn size(&self) -> usize {
        self.inner.size()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.inner.get(key)
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.inner.get_by_index(index)
    }

    fn first_key(&self) -> Option<&Key> {
        self.inner.first_key()
    }

    fn last_key(&self) -> Option<&Key> {
        self.inner.last_key()
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after(key)
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before(key)
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_after_no_wrap(key)
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        self.inner.key_before_no_wrap(key)
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.inner.keys()
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.nodes()
    }

    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.inner.children_of(parent_key)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use super::*;
    use crate::{builder::CollectionBuilder, tree_collection::TreeItemConfig};

    // -----------------------------------------------------------------------
    // Fixtures
    // -----------------------------------------------------------------------

    #[derive(Clone, Debug, PartialEq)]
    struct Item {
        id: Key,
        label: String,
    }

    impl Item {
        fn new(id: u64, label: &str) -> Self {
            Self {
                id: Key::int(id),
                label: label.to_string(),
            }
        }
    }

    impl CollectionItem for Item {
        fn key(&self) -> &Key {
            &self.id
        }

        fn text_value(&self) -> &str {
            &self.label
        }
    }

    fn list_of_three() -> MutableListData<Item> {
        MutableListData::new(
            CollectionBuilder::new()
                .item(Key::int(1), "Apple", Item::new(1, "Apple"))
                .item(Key::int(2), "Banana", Item::new(2, "Banana"))
                .item(Key::int(3), "Cherry", Item::new(3, "Cherry"))
                .build(),
        )
    }

    fn tree_config(
        id: u64,
        label: &str,
        children: Vec<TreeItemConfig<Item>>,
    ) -> TreeItemConfig<Item> {
        TreeItemConfig {
            key: Key::int(id),
            text_value: label.to_string(),
            value: Item::new(id, label),
            children,
            default_expanded: true,
        }
    }

    /// Build a small tree:
    ///
    /// ```text
    /// 1 Root A
    /// ├── 11 Child A1
    /// └── 12 Child A2
    /// 2 Root B
    /// └── 21 Child B1
    /// ```
    ///
    /// Flat DFS order: [1, 11, 12, 2, 21] → indices 0, 1, 2, 3, 4.
    fn sample_tree() -> MutableTreeData<Item> {
        MutableTreeData::new(TreeCollection::new([
            tree_config(
                1,
                "Root A",
                vec![
                    tree_config(11, "Child A1", vec![]),
                    tree_config(12, "Child A2", vec![]),
                ],
            ),
            tree_config(2, "Root B", vec![tree_config(21, "Child B1", vec![])]),
        ]))
    }

    // -----------------------------------------------------------------------
    // MutableListData — single mutations
    // -----------------------------------------------------------------------

    #[test]
    fn push_appends_and_emits_insert_change() {
        let mut list = list_of_three();

        list.push(Item::new(4, "Date"));

        assert_eq!(list.size(), 4);
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Insert { index: 3, count: 1 }]
        );

        // The appended item is retrievable under its key.
        let node = list.get(&Key::int(4)).expect("pushed item is present");

        assert_eq!(node.key, Key::int(4));
    }

    #[test]
    fn insert_at_index_emits_insert_change() {
        let mut list = list_of_three();

        list.insert(1, Item::new(42, "Mango"));

        assert_eq!(list.size(), 4);
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Insert { index: 1, count: 1 }]
        );

        // The inserted item lands at flat index 1.
        let node = list.get_by_index(1).expect("index 1");

        assert_eq!(node.key, Key::int(42));
    }

    #[test]
    fn remove_emits_remove_change_with_keys() {
        let mut list = list_of_three();

        let keys = [Key::int(1), Key::int(3)];

        let removed = list.remove(&keys);

        assert_eq!(removed.len(), 2);
        assert_eq!(list.size(), 1);
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Remove {
                keys: keys.to_vec(),
            }]
        );
    }

    #[test]
    fn remove_change_contains_only_matched_keys() {
        let mut list = list_of_three();

        // Mix of valid (1) and unknown (999) keys.
        let removed = list.remove(&[Key::int(1), Key::int(999)]);

        assert_eq!(removed.len(), 1);
        assert_eq!(list.size(), 2);

        // The change event reflects only the key that actually matched.
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Remove {
                keys: vec![Key::int(1)],
            }]
        );
    }

    #[test]
    fn remove_all_unknown_keys_emits_no_event() {
        let mut list = list_of_three();

        let removed = list.remove(&[Key::int(998), Key::int(999)]);

        assert!(removed.is_empty());
        assert_eq!(list.size(), 3);
        assert!(list.drain_changes().is_empty());
    }

    #[test]
    fn remove_separator_emits_remove_event_despite_empty_payload() {
        // A separator (and Section/Header) is a structural node — it
        // appears in `Collection::keys` and adapters render a row for
        // it, but it carries no `T` payload. `StaticCollection::
        // remove_by_keys` drops structural nodes silently from its
        // returned `Vec<T>` even though it does mutate the collection
        // (the node is gone, every later index has shifted). The
        // wrapper must still emit `Remove` for that key, otherwise the
        // change log lies and a reconciler keyed off `CollectionChange`
        // leaves a phantom row in the DOM.
        let mut list = MutableListData::new(
            CollectionBuilder::<Item>::new()
                .item(Key::int(1), "Apple", Item::new(1, "Apple"))
                .separator()
                .item(Key::int(2), "Banana", Item::new(2, "Banana"))
                .build(),
        );

        // `separator()` derives its key from the insertion index (1
        // here, between the two items) — see `CollectionBuilder`.
        let separator_key = Key::str("separator-1");

        assert_eq!(list.size(), 3, "two items + one separator");
        assert!(
            list.contains_key(&separator_key),
            "separator must be part of the iteration",
        );

        let removed = list.remove(core::slice::from_ref(&separator_key));

        // The separator carries no payload; the inner method returns
        // an empty `Vec<T>` even though the node was removed.
        assert!(removed.is_empty(), "structural nodes never carry a payload",);
        assert_eq!(
            list.size(),
            2,
            "separator was removed even though no payload was returned",
        );

        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Remove {
                keys: vec![separator_key],
            }],
            "structural-key removal must still emit Remove so adapters reconcile",
        );
    }

    #[test]
    fn move_item_emits_move_change_and_returns_indices() {
        let mut list = list_of_three();

        let indices = list.move_item(&Key::int(1), 2);

        assert_eq!(indices, Some((0, 2)));
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Move {
                key: Key::int(1),
                from_index: 0,
                to_index: 2,
            }]
        );

        // After the move, the item previously at flat 0 is at flat 2.
        let node = list.get_by_index(2).expect("index 2");

        assert_eq!(node.key, Key::int(1));
    }

    #[test]
    fn move_item_unknown_key_returns_none_and_emits_no_event() {
        let mut list = list_of_three();

        let indices = list.move_item(&Key::int(999), 0);

        assert_eq!(indices, None);
        assert!(list.drain_changes().is_empty());
    }

    #[test]
    fn replace_emits_replace_change_and_returns_old_value() {
        let mut list = list_of_three();

        let old = list.replace(Item::new(2, "Blueberry"));

        assert_eq!(old.expect("old value").label, "Banana");
        assert_eq!(
            list.drain_changes(),
            vec![CollectionChange::Replace { key: Key::int(2) }]
        );

        // Value is replaced in place; key is unchanged.
        let node = list.get(&Key::int(2)).expect("key 2");

        assert_eq!(node.value.as_ref().expect("value").label, "Blueberry");
    }

    #[test]
    fn replace_unknown_key_returns_none_and_emits_no_event() {
        let mut list = list_of_three();

        let old = list.replace(Item::new(999, "Phantom"));

        assert!(old.is_none());
        assert_eq!(list.size(), 3);
        assert!(list.drain_changes().is_empty());
    }

    #[test]
    fn clear_emits_reset_change() {
        let mut list = list_of_three();

        list.clear();

        assert_eq!(list.size(), 0);
        assert_eq!(list.drain_changes(), vec![CollectionChange::Reset]);
    }

    #[test]
    fn clear_discards_pending_events_and_emits_only_reset() {
        // Regression test: prior behaviour was to append `Reset` to the
        // existing buffer, leaving entries like `[Insert, Reset]` for the
        // adapter to drain. `Insert { index, count }` carries no payload,
        // so an adapter cannot replay a stale insert against the cleared
        // collection. After `clear`, the buffer must be exactly `[Reset]`
        // regardless of what was queued earlier in the cycle.
        let mut list = list_of_three();

        // Queue every other variant before clearing.
        list.push(Item::new(4, "Date")); // Insert
        list.remove(&[Key::int(2)]); // Remove
        list.move_item(&Key::int(1), 1); // Move
        list.replace(Item::new(3, "Cherry-2")); // Replace

        // Sanity check: four events queued before clear.
        assert_eq!(
            list.drain_changes().len(),
            4,
            "precondition: four events queued before clear"
        );

        // Re-queue more events, then clear.
        list.push(Item::new(5, "Elderberry"));
        list.push(Item::new(6, "Fig"));

        list.clear();

        // After clear, the buffer must contain exactly one Reset event —
        // the prior Insert events would reference indices the adapter
        // can no longer replay.
        let drained = list.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Reset],
            "clear must coalesce all pending events into a single Reset; got {drained:?}"
        );

        // Inner state is also empty.
        assert_eq!(list.size(), 0);
    }

    #[test]
    fn clear_on_empty_buffer_still_emits_single_reset() {
        // No prior events queued — clear must still emit exactly `[Reset]`.
        let mut list = list_of_three();

        list.clear();

        assert_eq!(list.drain_changes(), vec![CollectionChange::Reset]);
    }

    #[test]
    fn replace_on_structural_node_returns_none_and_emits_no_event() {
        // Regression test: a Section node shares the key namespace with
        // items, so a caller could supply an item whose key collides with
        // a Section. `MutableListData::replace` must return `None` (the
        // inner `StaticCollection::replace` refuses to mutate structural
        // nodes) AND must NOT emit a `Replace` event for the structural
        // key — otherwise the adapter would try to re-render a Section
        // as if it were an item.
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", Item::new(1, "Apple"))
            .end_section()
            .build();

        let mut list = MutableListData::new(inner);

        // Drain the empty initial buffer to make assertions unambiguous.
        drop(list.drain_changes());

        // Build an Item whose key collides with the Section's key.
        let intruder = Item {
            id: Key::str("fruits"),
            label: "Pineapple".to_string(),
        };

        let old = list.replace(intruder);

        assert!(old.is_none(), "replace must report failure on Section key");

        let drained = list.drain_changes();

        assert!(
            drained.is_empty(),
            "no Replace event should be emitted for structural-node keys; got {drained:?}"
        );

        // The Section is still a Section — no silent promotion to an Item.
        let section = list
            .get(&Key::str("fruits"))
            .expect("section still present");

        assert!(section.value.is_none(), "Section value must remain None");
    }

    #[test]
    fn drain_changes_returns_and_clears_buffer() {
        let mut list = list_of_three();

        list.push(Item::new(4, "Date"));
        list.push(Item::new(5, "Elderberry"));

        let first = list.drain_changes();

        assert_eq!(first.len(), 2);

        // Buffer is now empty.
        let second = list.drain_changes();

        assert!(second.is_empty());
    }

    #[test]
    fn list_delegates_collection_methods() {
        let mut list = list_of_three();

        assert_eq!(list.size(), 3);
        assert!(!list.is_empty());
        assert!(list.get(&Key::int(2)).is_some());
        assert!(list.get(&Key::int(99)).is_none());
        assert_eq!(list.get_by_index(1).expect("idx 1").key, Key::int(2));
        assert_eq!(list.first_key(), Some(&Key::int(1)));
        assert_eq!(list.last_key(), Some(&Key::int(3)));
        assert_eq!(list.key_after(&Key::int(1)), Some(&Key::int(2)));
        assert_eq!(list.key_before(&Key::int(2)), Some(&Key::int(1)));
        assert_eq!(
            list.key_after_no_wrap(&Key::int(3)),
            None,
            "no wrap past last"
        );
        assert_eq!(
            list.key_before_no_wrap(&Key::int(1)),
            None,
            "no wrap past first"
        );

        let keys = list.keys().cloned().collect::<Vec<_>>();

        assert_eq!(keys, vec![Key::int(1), Key::int(2), Key::int(3)]);

        let node_count = list.nodes().count();

        assert_eq!(node_count, 3);

        // children_of on a flat list yields nothing.
        assert_eq!(list.children_of(&Key::int(1)).count(), 0);

        // Drain sanity — no reads should have mutated the change buffer.
        assert!(list.drain_changes().is_empty());
    }

    // -----------------------------------------------------------------------
    // MutableTreeData — single mutations
    // -----------------------------------------------------------------------

    #[test]
    fn tree_insert_child_emits_insert_with_flat_index() {
        let mut tree = sample_tree();

        // Insert a new root as the third root sibling (sibling_index 2).
        // Expected flat index after insert: 5 (after the original 5 nodes).
        let root_flat = tree.insert_child(None, 2, Item::new(3, "Root C"));

        assert_eq!(root_flat, Some(5));
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Insert { index: 5, count: 1 }]
        );

        // Insert a new child under Root A (key 1) at sibling_index 0.
        // Expected flat index: 1 (just after Root A).
        let child_flat = tree.insert_child(Some(&Key::int(1)), 0, Item::new(10, "Child A0"));

        assert_eq!(child_flat, Some(1));
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Insert { index: 1, count: 1 }]
        );
    }

    #[test]
    fn tree_insert_child_with_invalid_parent_returns_none_and_emits_no_event() {
        let mut tree = sample_tree();

        let flat = tree.insert_child(Some(&Key::int(999)), 0, Item::new(77, "Orphan"));

        assert_eq!(flat, None, "wrapper returns None on rejected insert");
        assert_eq!(tree.size(), 5, "tree size is unchanged");
        assert!(
            tree.drain_changes().is_empty(),
            "no change event for rejected insert"
        );
    }

    #[test]
    fn tree_remove_emits_remove_change() {
        let mut tree = sample_tree();

        let keys = [Key::int(11)];

        let removed = tree.remove(&keys);

        assert_eq!(removed.len(), 1);
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Remove {
                keys: keys.to_vec(),
            }]
        );
    }

    #[test]
    fn tree_remove_change_contains_only_matched_keys() {
        let mut tree = sample_tree();

        // Mix of valid (11) and unknown (999) keys.
        let removed = tree.remove(&[Key::int(11), Key::int(999)]);

        assert_eq!(removed.len(), 1);
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Remove {
                keys: vec![Key::int(11)],
            }]
        );
    }

    #[test]
    fn tree_remove_all_unknown_keys_emits_no_event() {
        let mut tree = sample_tree();

        let removed = tree.remove(&[Key::int(998), Key::int(999)]);

        assert!(removed.is_empty());
        assert_eq!(tree.size(), 5);
        assert!(tree.drain_changes().is_empty());
    }

    #[test]
    fn tree_reparent_emits_move_with_correct_flat_indices() {
        let mut tree = sample_tree();

        // Move Child A1 (flat index 1) to be the first child of Root B.
        // Before: [1, 11, 12, 2, 21] — Child A1 at index 1.
        // After reparent under key 2 at sibling_index 0:
        //   [1, 12, 2, 11, 21] — Child A1 now at flat index 3.
        let indices = tree.reparent(&Key::int(11), Some(&Key::int(2)), 0);

        assert_eq!(indices, Some((1, 3)));
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Move {
                key: Key::int(11),
                from_index: 1,
                to_index: 3,
            }]
        );
    }

    #[test]
    fn tree_reparent_unknown_key_returns_none_and_emits_no_event() {
        let mut tree = sample_tree();

        let indices = tree.reparent(&Key::int(999), None, 0);

        assert_eq!(indices, None);
        assert!(tree.drain_changes().is_empty());
    }

    #[test]
    fn tree_reparent_unknown_new_parent_returns_none_and_emits_no_event() {
        let mut tree = sample_tree();

        let indices = tree.reparent(&Key::int(11), Some(&Key::int(999)), 0);

        assert_eq!(indices, None);
        assert!(tree.drain_changes().is_empty());
    }

    #[test]
    fn tree_reorder_emits_move_with_correct_flat_indices() {
        let mut tree = sample_tree();

        // Move Child A1 (flat index 1) after Child A2 among its siblings.
        // Before: [1, 11, 12, 2, 21] — Child A1 at 1, Child A2 at 2.
        // After reorder_sibling(11, 1):
        //   [1, 12, 11, 2, 21] — Child A1 now at flat index 2.
        let indices = tree.reorder(&Key::int(11), 1);

        assert_eq!(indices, Some((1, 2)));
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Move {
                key: Key::int(11),
                from_index: 1,
                to_index: 2,
            }]
        );
    }

    #[test]
    fn tree_reorder_unknown_key_returns_none_and_emits_no_event() {
        let mut tree = sample_tree();

        let indices = tree.reorder(&Key::int(999), 0);

        assert_eq!(indices, None);
        assert!(tree.drain_changes().is_empty());
    }

    #[test]
    fn tree_replace_emits_replace_change_and_returns_old_value() {
        let mut tree = sample_tree();

        let old = tree.replace(Item::new(11, "Renamed A1"));

        assert_eq!(old.expect("old value").label, "Child A1");
        assert_eq!(
            tree.drain_changes(),
            vec![CollectionChange::Replace { key: Key::int(11) }]
        );

        let node = tree.get(&Key::int(11)).expect("key 11");

        assert_eq!(node.value.as_ref().expect("value").label, "Renamed A1");
    }

    #[test]
    fn tree_replace_unknown_key_returns_none_and_emits_no_event() {
        let mut tree = sample_tree();

        let old = tree.replace(Item::new(999, "Phantom"));

        assert!(old.is_none());
        assert_eq!(tree.size(), 5);
        assert!(tree.drain_changes().is_empty());
    }

    #[test]
    fn tree_drain_changes_returns_and_clears_buffer() {
        let mut tree = sample_tree();

        tree.replace(Item::new(11, "X"));
        tree.replace(Item::new(12, "Y"));

        let first = tree.drain_changes();

        assert_eq!(first.len(), 2);

        let second = tree.drain_changes();

        assert!(second.is_empty());
    }

    // -----------------------------------------------------------------------
    // MutableTreeData — visibility-aware change events
    //
    // The events `MutableTreeData` emits must describe *visible* iteration
    // changes, not raw `all_nodes` mutations. Hidden subtrees never appear
    // in `Collection::nodes`, so adapters relying on `get_by_index` would
    // mis-place DOM updates if the events carried flat DFS indices that
    // pointed inside a collapsed ancestor.
    // -----------------------------------------------------------------------

    /// Helper: build a tree with one collapsed root parent and one expanded
    /// root parent, plus a leaf root at the end. After construction:
    ///
    /// ```text
    /// 1 Root A   (collapsed)         flat 0, visible 0
    /// ├── 11 Child A1                flat 1, visible — (hidden)
    /// └── 12 Child A2                flat 2, visible — (hidden)
    /// 2 Root B   (expanded)          flat 3, visible 1
    /// └── 21 Child B1                flat 4, visible 2
    /// 3 Root C   (leaf)              flat 5, visible 3
    /// ```
    fn mixed_visibility_tree() -> MutableTreeData<Item> {
        MutableTreeData::new(TreeCollection::new([
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Root A".to_string(),
                value: Item::new(1, "Root A"),
                children: vec![
                    TreeItemConfig {
                        key: Key::int(11),
                        text_value: "Child A1".to_string(),
                        value: Item::new(11, "Child A1"),
                        children: vec![],
                        default_expanded: true,
                    },
                    TreeItemConfig {
                        key: Key::int(12),
                        text_value: "Child A2".to_string(),
                        value: Item::new(12, "Child A2"),
                        children: vec![],
                        default_expanded: true,
                    },
                ],
                default_expanded: false, // <-- A is collapsed
            },
            TreeItemConfig {
                key: Key::int(2),
                text_value: "Root B".to_string(),
                value: Item::new(2, "Root B"),
                children: vec![TreeItemConfig {
                    key: Key::int(21),
                    text_value: "Child B1".to_string(),
                    value: Item::new(21, "Child B1"),
                    children: vec![],
                    default_expanded: true,
                }],
                default_expanded: true,
            },
            TreeItemConfig {
                key: Key::int(3),
                text_value: "Root C".to_string(),
                value: Item::new(3, "Root C"),
                children: vec![],
                default_expanded: true,
            },
        ]))
    }

    #[test]
    fn tree_visibility_fixture_has_expected_visible_iteration() {
        // Sanity check: confirm that `mixed_visibility_tree` actually
        // hides the children of Root A, so the visibility-aware tests
        // below exercise the real "hidden" branches.
        let tree = mixed_visibility_tree();

        let visible = tree.keys().cloned().collect::<Vec<_>>();

        assert_eq!(
            visible,
            vec![Key::int(1), Key::int(2), Key::int(21), Key::int(3)],
            "Children of collapsed Root A must be hidden from visible iteration"
        );
    }

    #[test]
    fn tree_insert_child_under_collapsed_parent_emits_no_event() {
        // Insert a new child under the collapsed Root A. The child sits
        // inside a hidden subtree, so adapters have nothing to render.
        // Per the visibility-aware contract no event is emitted.
        let mut tree = mixed_visibility_tree();

        let flat = tree.insert_child(Some(&Key::int(1)), 0, Item::new(99, "Hidden Child"));

        assert!(
            flat.is_some(),
            "insert succeeded — Some(_) returned even though no event is emitted"
        );

        // The node IS in the inner tree (just not in the visible iteration).
        assert!(
            tree.get(&Key::int(99)).is_some(),
            "hidden node is still reachable by key"
        );

        let drained = tree.drain_changes();

        assert!(
            drained.is_empty(),
            "no event for insert under collapsed parent; got {drained:?}"
        );
    }

    #[test]
    fn tree_insert_child_under_visible_leaf_parent_emits_replace_for_parent() {
        // Adding a child under a visible *leaf* flips that parent from
        // leaf to collapsed branch:
        //   has_children: false → true
        //   is_expanded:  None  → Some(false)
        //
        // The new child lands inside the now-collapsed parent and is
        // therefore hidden from the visible iteration, so no Insert
        // event fires for the child. But the parent's rendered state
        // genuinely changed — adapters key DOM off `has_children` and
        // `is_expanded` (expander chevron, `aria-expanded`), so a
        // reconciler that consumes only `CollectionChange` would leave
        // the parent row stale unless we emit `Replace` for it.
        let mut tree = mixed_visibility_tree();

        // Root C (id 3) is a visible leaf in the fixture.
        let root_c_before = tree
            .get(&Key::int(3))
            .expect("fixture: Root C must be present before the insert");

        assert!(!root_c_before.has_children, "Root C starts as a leaf");
        assert_eq!(
            root_c_before.is_expanded, None,
            "a leaf has no expansion state",
        );

        let flat = tree.insert_child(Some(&Key::int(3)), 0, Item::new(99, "New Child"));

        assert!(flat.is_some(), "insert succeeded");

        // The child is in the inner tree but hidden under newly-
        // collapsed Root C.
        assert!(
            tree.get(&Key::int(99)).is_some(),
            "child is reachable by key",
        );
        assert!(
            !tree.keys().any(|k| *k == Key::int(99)),
            "child must NOT be in the visible iteration",
        );

        // Confirm the leaf→branch transition actually happened on Root
        // C, since that is the precondition for the Replace event.
        let root_c = tree.get(&Key::int(3)).expect("Root C still in tree");
        assert!(root_c.has_children, "Root C now has children");
        assert_eq!(root_c.is_expanded, Some(false), "Root C is collapsed");

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Replace { key: Key::int(3) }],
            "leaf→branch transition on a visible parent must emit Replace so adapters re-render",
        );
    }

    #[test]
    fn tree_insert_child_under_hidden_leaf_parent_emits_no_event() {
        // Same leaf→branch transition mechanics as the previous test,
        // but the parent itself is hidden inside a collapsed ancestor.
        // Its rendered state can't be stale because the row isn't
        // rendered at all, so no event must fire.
        let mut tree = mixed_visibility_tree();

        // Child A1 is a hidden leaf (under collapsed Root A).
        let child_a1_before = tree.get(&Key::int(11)).expect("fixture: Child A1 present");

        assert!(!child_a1_before.has_children, "Child A1 starts as a leaf");
        assert_eq!(
            child_a1_before.is_expanded, None,
            "Child A1 is a leaf, not an expanded branch",
        );

        let flat = tree.insert_child(Some(&Key::int(11)), 0, Item::new(111, "Hidden Grand"));

        assert!(flat.is_some(), "insert succeeded");

        // Inner state changed (Child A1 is now a collapsed branch),
        // but neither it nor the new grandchild is rendered.
        let child_a1_after = tree
            .get(&Key::int(11))
            .expect("Child A1 still present after insert");

        assert!(
            child_a1_after.has_children,
            "Child A1 flipped leaf→branch internally",
        );
        assert_eq!(
            child_a1_after.is_expanded,
            Some(false),
            "Child A1 is now a collapsed branch",
        );

        let drained = tree.drain_changes();

        assert!(
            drained.is_empty(),
            "no event for leaf→branch on a hidden parent; got {drained:?}",
        );
    }

    #[test]
    fn tree_insert_child_emits_visible_index_when_visibility_skews_flat() {
        // Insert a new root after Root B. Because Root A's subtree is
        // collapsed, the new root's flat DFS index differs from its
        // visible iteration index — the event must use the visible one.
        //
        // Before insert (flat / visible):
        //   1 Root A    flat 0 / visible 0
        //   11/12       flat 1,2 / hidden
        //   2 Root B    flat 3 / visible 1
        //   21          flat 4 / visible 2
        //   3 Root C    flat 5 / visible 3
        //
        // After inserting a new root at sibling_index 2 (between B and C):
        //   New root    flat 5 / visible 3
        let mut tree = mixed_visibility_tree();

        let flat = tree.insert_child(None, 2, Item::new(50, "New Root"));

        assert_eq!(flat, Some(5), "flat DFS index after the existing nodes");

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Insert {
                index: 3, // visible position, not flat position
                count: 1,
            }],
            "Insert event must carry visible iteration index, not flat DFS"
        );

        // Sanity check: the visible iteration now includes the new root
        // at position 3.
        let visible_at_3 = tree.get_by_index(3).expect("visible index 3");

        assert_eq!(visible_at_3.key, Key::int(50));
    }

    #[test]
    fn tree_remove_hidden_non_last_child_emits_no_event() {
        // Remove one hidden child of collapsed Root A. Root A still has
        // its other hidden child afterwards, so `has_children` stays
        // `true` (no branch→leaf flip), *and* no visible keys are
        // removed. With nothing to render differently, the adapter gets
        // an empty drain. (Removing *both* children would drain Root A's
        // child list and emit `Replace { Root A }` — see
        // `tree_remove_hidden_last_children_emits_replace_for_visible_parent`.)
        let mut tree = mixed_visibility_tree();

        let removed = tree.remove(&[Key::int(11)]);

        assert_eq!(removed.len(), 1, "inner removal returned Child A1");

        // Root A is still a collapsed branch (Child A2 remains).
        let root_a = tree.get(&Key::int(1)).expect("Root A still in tree");

        assert!(root_a.has_children, "Root A still has Child A2");
        assert_eq!(
            root_a.is_expanded,
            Some(false),
            "Root A is still a collapsed branch",
        );

        let drained = tree.drain_changes();

        assert!(
            drained.is_empty(),
            "no event when removed node was hidden and parent stayed a branch; \
             got {drained:?}"
        );
    }

    #[test]
    fn tree_remove_filters_event_to_visible_keys() {
        // Remove a mix: Root B (visible, plus its visible child 21) and
        // Child A1 (hidden inside collapsed Root A).
        // The Remove event must list ONLY the previously-visible keys.
        let mut tree = mixed_visibility_tree();

        let removed = tree.remove(&[Key::int(2), Key::int(11)]);

        // Root B + Child B1 + Child A1 = 3 inner removals.
        assert_eq!(removed.len(), 3);

        let drained = tree.drain_changes();

        // Order of keys in the event mirrors the order returned by the
        // inner `remove_by_keys` (subtree-by-subtree).
        let expected = vec![CollectionChange::Remove {
            keys: vec![Key::int(2), Key::int(21)],
        }];

        assert_eq!(
            drained, expected,
            "Remove event must include only previously-visible keys"
        );
    }

    #[test]
    fn tree_remove_hidden_last_children_emits_replace_for_visible_parent() {
        // Remove *both* hidden children of collapsed Root A. None of the
        // removed keys were visible, so `visible_removed_keys` is empty —
        // but `TreeCollection::remove_by_keys` still flips Root A's
        // metadata from a collapsed branch to a leaf:
        //   has_children: true  → false
        //   is_expanded:  Some(false) → None
        // Root A is *visible*, and adapters key expander chevron /
        // `aria-expanded` off those flags. A reconciler consuming only
        // `CollectionChange` would leave Root A rendered as a collapsed
        // branch forever unless the wrapper emits `Replace`.
        let mut tree = mixed_visibility_tree();

        // Sanity: Root A starts as a collapsed branch.
        let root_a_before = tree
            .get(&Key::int(1))
            .expect("fixture: Root A present before remove");

        assert!(root_a_before.has_children, "Root A starts with children");
        assert_eq!(
            root_a_before.is_expanded,
            Some(false),
            "Root A starts collapsed",
        );

        let removed = tree.remove(&[Key::int(11), Key::int(12)]);

        assert_eq!(
            removed.len(),
            2,
            "inner removal still returns both payloads"
        );

        // Confirm the branch→leaf transition actually landed, since
        // that is the precondition for the Replace event.
        let root_a = tree.get(&Key::int(1)).expect("Root A still present");

        assert!(!root_a.has_children, "Root A lost its last child");
        assert_eq!(root_a.is_expanded, None, "a leaf has no expansion state",);

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Replace { key: Key::int(1) }],
            "branch→leaf transition on a visible parent must emit Replace, \
             even though no visible descendants were removed",
        );
    }

    #[test]
    fn tree_remove_visible_last_child_emits_replace_alongside_remove() {
        // Remove Child B1, the only visible child of Root B. The visible
        // iteration loses Child B1 (→ Remove event) *and* Root B flips
        // from an expanded branch to a leaf:
        //   has_children: true → false
        //   is_expanded:  Some(true) → None
        // Root B is still visible at the same row, so its chevron /
        // `aria-expanded` must be refreshed in the same update cycle.
        // The Replace for Root B fires alongside the Remove for Child B1.
        let mut tree = mixed_visibility_tree();

        let root_b_before = tree
            .get(&Key::int(2))
            .expect("fixture: Root B present before remove");

        assert!(root_b_before.has_children, "Root B starts with children");
        assert_eq!(
            root_b_before.is_expanded,
            Some(true),
            "Root B starts expanded",
        );

        let removed = tree.remove(&[Key::int(21)]);

        assert_eq!(removed.len(), 1, "Child B1 payload returned");

        let root_b = tree.get(&Key::int(2)).expect("Root B still present");

        assert!(!root_b.has_children, "Root B lost its last child");
        assert_eq!(root_b.is_expanded, None, "Root B is now a leaf");

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![
                CollectionChange::Remove {
                    keys: vec![Key::int(21)],
                },
                CollectionChange::Replace { key: Key::int(2) },
            ],
            "visible child removal that flips the visible parent must emit \
             Remove for the child and Replace for the parent in that order",
        );
    }

    #[test]
    fn tree_reparent_visible_to_hidden_emits_remove() {
        // Move Root C (visible leaf) under collapsed Root A.
        // From the adapter's DOM perspective, Root C disappears.
        let mut tree = mixed_visibility_tree();

        let indices = tree.reparent(&Key::int(3), Some(&Key::int(1)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        let visible_keys = tree.keys().cloned().collect::<Vec<_>>();

        assert!(
            !visible_keys.contains(&Key::int(3)),
            "Root C is no longer visible after moving under collapsed parent; \
             visible: {visible_keys:?}"
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Remove {
                keys: vec![Key::int(3)],
            }],
            "visible→hidden reparent must emit Remove, not Move"
        );
    }

    #[test]
    fn tree_reparent_visible_to_hidden_emits_full_subtree_remove() {
        // Move Root B (expanded, with visible Child B1) under collapsed
        // Root A. Both Root B AND its visible descendant disappear from
        // the iteration — the Remove event must list every previously
        // visible key in the moved subtree, not just the moved root,
        // otherwise keyed reconcilers leave orphan rows in the DOM.
        let mut tree = mixed_visibility_tree();

        // Sanity-check the pre-mutation visible iteration so the regression
        // is obvious if the fixture changes.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2), Key::int(21), Key::int(3)],
            "fixture: Root A collapsed, Root B expanded with Child B1 visible",
        );

        let indices = tree.reparent(&Key::int(2), Some(&Key::int(1)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        let visible_after = tree.keys().cloned().collect::<Vec<_>>();
        assert_eq!(
            visible_after,
            vec![Key::int(1), Key::int(3)],
            "Root B and Child B1 must both be hidden under collapsed Root A",
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Remove {
                keys: vec![Key::int(2), Key::int(21)],
            }],
            "Remove must include every previously-visible subtree key in DFS order"
        );
    }

    #[test]
    fn tree_reparent_hidden_to_visible_emits_insert() {
        // Move Child A1 (hidden inside collapsed Root A) out to be a
        // root sibling. From the adapter's DOM perspective, Child A1
        // appears for the first time.
        let mut tree = mixed_visibility_tree();

        // Reparent Child A1 to root, between Root B and Root C.
        let indices = tree.reparent(&Key::int(11), None, 2);

        assert!(indices.is_some(), "reparent succeeded");

        let drained = tree.drain_changes();

        // Visible iteration after reparent:
        //   1 Root A    visible 0 (still collapsed; only Child A2 hidden)
        //   2 Root B    visible 1
        //   21 Child B1 visible 2
        //   11 Child A1 visible 3 (newly inserted)
        //   3 Root C    visible 4
        assert_eq!(
            drained,
            vec![CollectionChange::Insert { index: 3, count: 1 }],
            "hidden→visible reparent must emit Insert at the visible position"
        );
    }

    #[test]
    fn tree_reparent_hidden_to_visible_emits_subtree_insert_count() {
        // Move Child A1 (hidden inside collapsed Root A, but itself
        // expanded with an expanded grandchild) out to root level.
        // The whole previously-hidden subtree (Child A1 + Grand A11)
        // becomes visible at once — `count` must reflect the full
        // subtree size, not 1, otherwise adapters that honour `count`
        // will under-apply the change and drift indices.
        //
        // Child A1 is Root A's *only* child in this fixture, so the
        // move also drains Root A: it flips branch → leaf while
        // staying visible at row 0. The wrapper emits the subtree
        // `Insert` first, then a `Replace` for Root A's metadata flip,
        // in that order.
        let mut tree = MutableTreeData::new(TreeCollection::new([
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Root A".to_string(),
                value: Item::new(1, "Root A"),
                children: vec![TreeItemConfig {
                    key: Key::int(11),
                    text_value: "Child A1".to_string(),
                    value: Item::new(11, "Child A1"),
                    children: vec![TreeItemConfig {
                        key: Key::int(111),
                        text_value: "Grand A11".to_string(),
                        value: Item::new(111, "Grand A11"),
                        children: vec![],
                        default_expanded: true,
                    }],
                    default_expanded: true,
                }],
                default_expanded: false, // Root A collapsed → subtree hidden
            },
            TreeItemConfig {
                key: Key::int(2),
                text_value: "Root B".to_string(),
                value: Item::new(2, "Root B"),
                children: vec![],
                default_expanded: true,
            },
        ]));

        // Sanity: only the two roots are visible to start.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2)],
            "fixture: Child A1 / Grand A11 hidden under collapsed Root A",
        );

        // Move Child A1 to root sibling-index 1 (between Root A and Root B).
        let indices = tree.reparent(&Key::int(11), None, 1);

        assert!(indices.is_some(), "reparent succeeded");

        // Visible iteration after reparent:
        //   1   Root A    visible 0 (now leaf)
        //   11  Child A1  visible 1 (newly visible, expanded)
        //   111 Grand A11 visible 2 (newly visible alongside its parent)
        //   2   Root B    visible 3
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(11), Key::int(111), Key::int(2)],
            "Grand A11 must surface alongside Child A1 because the subtree was already expanded",
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![
                CollectionChange::Insert { index: 1, count: 2 },
                CollectionChange::Replace { key: Key::int(1) },
            ],
            "Insert count must reflect every newly-visible subtree node, not \
             just the root; Replace must fire for Root A's branch→leaf flip",
        );
    }

    #[test]
    fn tree_reparent_hidden_to_hidden_flips_both_visible_roots_emits_replace_for_each() {
        // Two collapsed visible roots (Box 1, Box 2) with a single
        // hidden item under Box 1. Moving the item between them keeps
        // both subtree endpoints hidden (a `(None, None)` case — no
        // `Move` / `Insert` / `Remove` for the subtree itself), but
        // both roots *are* visible and both flip their branch/leaf
        // state:
        //   Box 1: has_children true → false, is_expanded Some(false) → None
        //   Box 2: has_children false → true, is_expanded None → Some(false)
        // The wrapper must emit one `Replace` per affected visible
        // parent in pre-mutation DFS order, otherwise adapters leave
        // both chevrons stale.
        let mut tree = MutableTreeData::new(TreeCollection::new([
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Box 1".to_string(),
                value: Item::new(1, "Box 1"),
                children: vec![TreeItemConfig {
                    key: Key::int(11),
                    text_value: "Item".to_string(),
                    value: Item::new(11, "Item"),
                    children: vec![],
                    default_expanded: true,
                }],
                default_expanded: false,
            },
            TreeItemConfig {
                key: Key::int(2),
                text_value: "Box 2".to_string(),
                value: Item::new(2, "Box 2"),
                children: vec![],
                default_expanded: false,
            },
        ]));

        // Sanity: both roots are visible; the item is hidden.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2)],
            "fixture: only the two roots are visible",
        );

        let indices = tree.reparent(&Key::int(11), Some(&Key::int(2)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        // Confirm both transitions landed.
        let box_1 = tree.get(&Key::int(1)).expect("Box 1 still present");

        assert!(!box_1.has_children, "Box 1 lost its only child");
        assert_eq!(box_1.is_expanded, None, "Box 1 is now a leaf");

        let box_2 = tree.get(&Key::int(2)).expect("Box 2 still present");

        assert!(box_2.has_children, "Box 2 gained the item");
        assert_eq!(
            box_2.is_expanded,
            Some(false),
            "Box 2 is now a collapsed branch",
        );

        // The visible iteration is unchanged — the item is hidden
        // under the newly-collapsed Box 2.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2)],
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![
                CollectionChange::Replace { key: Key::int(1) },
                CollectionChange::Replace { key: Key::int(2) },
            ],
            "both visible parents flipped metadata; one Replace per parent, \
             pre-mutation DFS order",
        );
    }

    #[test]
    fn tree_reparent_hidden_to_hidden_emits_replace_for_visible_leaf_destination() {
        // Move Child A1 (hidden under collapsed Root A) to be the first
        // child of Root C (a visible leaf). The moved subtree is hidden
        // before *and* after (Root C flips into a collapsed branch the
        // moment it gains a child, so Child A1 lands inside a collapsed
        // ancestor). That makes this a `(None, None)` subtree-visibility
        // case — no `Move` / `Insert` / `Remove` fires for the subtree —
        // yet Root C's *visible row* transitioned:
        //   has_children: false → true
        //   is_expanded:  None  → Some(false)
        // Adapters key expander chevron / `aria-expanded` off those
        // flags, so a reconciler consuming only `CollectionChange`
        // would leave Root C rendered as a leaf unless the wrapper
        // emits `Replace` for it. Root A still has Child A2, so its
        // `has_children` flag is unchanged — no Replace for Root A.
        let mut tree = mixed_visibility_tree();

        // Sanity: Root C starts as a visible leaf.
        let root_c_before = tree
            .get(&Key::int(3))
            .expect("fixture: Root C present before reparent");

        assert!(!root_c_before.has_children, "Root C starts as a leaf");
        assert_eq!(
            root_c_before.is_expanded, None,
            "a leaf has no expansion state",
        );

        let indices = tree.reparent(&Key::int(11), Some(&Key::int(3)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        // Confirm the leaf→branch transition landed on Root C.
        let root_c = tree.get(&Key::int(3)).expect("Root C still present");

        assert!(root_c.has_children, "Root C gained a child");
        assert_eq!(
            root_c.is_expanded,
            Some(false),
            "Root C is now a collapsed branch",
        );

        // Child A1 is hidden under newly-collapsed Root C; the visible
        // iteration length is unchanged (still [Root A, Root B, Child
        // B1, Root C]).
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2), Key::int(21), Key::int(3)],
            "Child A1 is hidden under the newly-collapsed Root C",
        );

        // Root A still has Child A2 and remains a branch.
        let root_a_after = tree
            .get(&Key::int(1))
            .expect("Root A present after reparent");

        assert!(root_a_after.has_children, "Root A kept Child A2");
        assert_eq!(
            root_a_after.is_expanded,
            Some(false),
            "Root A is still a collapsed branch",
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Replace { key: Key::int(3) }],
            "leaf→branch transition on a visible destination must emit \
             Replace even when neither subtree endpoint is visible",
        );
    }

    #[test]
    fn tree_reparent_visible_move_flips_src_parent_emits_move_and_replace() {
        // Both endpoints visible (Move case). Root A has only Child A1,
        // so moving Child A1 away drains Root A's child list:
        //   has_children: true → false
        //   is_expanded:  Some(true) → None
        // Root A stays visible at row 0, so the wrapper must emit both
        // the subtree `Move` *and* a `Replace` for Root A's branch→leaf
        // flip in the same drain cycle.
        let mut tree = MutableTreeData::new(TreeCollection::new([
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Root A".to_string(),
                value: Item::new(1, "Root A"),
                children: vec![TreeItemConfig {
                    key: Key::int(11),
                    text_value: "Child A1".to_string(),
                    value: Item::new(11, "Child A1"),
                    children: vec![],
                    default_expanded: true,
                }],
                default_expanded: true,
            },
            TreeItemConfig {
                key: Key::int(2),
                text_value: "Root B".to_string(),
                value: Item::new(2, "Root B"),
                children: vec![TreeItemConfig {
                    key: Key::int(21),
                    text_value: "Child B1".to_string(),
                    value: Item::new(21, "Child B1"),
                    children: vec![],
                    default_expanded: true,
                }],
                default_expanded: true,
            },
        ]));

        // Sanity: everything is visible, Root A is an expanded branch.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(11), Key::int(2), Key::int(21),],
        );

        // Move Child A1 under Root B at sibling-index 0.
        let indices = tree.reparent(&Key::int(11), Some(&Key::int(2)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        // Root A now has no children (branch→leaf) while Root B still
        // has two (Root B's branch state is unchanged).
        let root_a = tree.get(&Key::int(1)).expect("Root A still present");

        assert!(!root_a.has_children, "Root A lost its only child");
        assert_eq!(root_a.is_expanded, None, "Root A is now a leaf");

        let root_b = tree.get(&Key::int(2)).expect("Root B still present");

        assert!(root_b.has_children, "Root B still a branch");
        assert_eq!(root_b.is_expanded, Some(true), "Root B still expanded",);

        // Post-move visible order: Root A (leaf), Root B, Child A1, Child B1.
        assert_eq!(
            tree.keys().cloned().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2), Key::int(11), Key::int(21),],
        );

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![
                CollectionChange::Move {
                    key: Key::int(11),
                    from_index: 1,
                    to_index: 2,
                },
                CollectionChange::Replace { key: Key::int(1) },
            ],
            "visible move that drains src_parent's last child must emit \
             Move then Replace for the src_parent's branch→leaf flip",
        );
    }

    #[test]
    fn tree_reparent_hidden_to_hidden_with_hidden_parents_emits_no_event() {
        // Deeply nested fixture: a single visible collapsed root that
        // hides two sub-parents (a collapsed branch and a leaf) with
        // their own hidden item. Moving the hidden item between the
        // two hidden sub-parents flips `has_children` on both — but
        // since neither sub-parent is itself rendered (both are hidden
        // under the outer collapsed root), the metadata change is
        // invisible and no event should fire. This is the negative
        // guard for the leaf→branch / branch→leaf Replace emissions in
        // `reparent`: the transitions only matter when the affected
        // parent is actually part of the visible iteration.
        let mut tree = MutableTreeData::new(TreeCollection::new([TreeItemConfig {
            key: Key::int(1),
            text_value: "Outer".to_string(),
            value: Item::new(1, "Outer"),
            children: vec![
                TreeItemConfig {
                    key: Key::int(11),
                    text_value: "Box 1".to_string(),
                    value: Item::new(11, "Box 1"),
                    children: vec![TreeItemConfig {
                        key: Key::int(111),
                        text_value: "Item".to_string(),
                        value: Item::new(111, "Item"),
                        children: vec![],
                        default_expanded: true,
                    }],
                    default_expanded: false,
                },
                TreeItemConfig {
                    key: Key::int(12),
                    text_value: "Box 2".to_string(),
                    value: Item::new(12, "Box 2"),
                    children: vec![],
                    default_expanded: false,
                },
            ],
            default_expanded: false,
        }]));

        // Sanity: only Outer is visible. Everything below is hidden.
        assert_eq!(tree.keys().cloned().collect::<Vec<_>>(), vec![Key::int(1)],);

        let indices = tree.reparent(&Key::int(111), Some(&Key::int(12)), 0);

        assert!(indices.is_some(), "reparent succeeded");

        // Box 1 (hidden) transitioned branch→leaf; Box 2 (hidden)
        // transitioned leaf→branch. Both are invisible under the
        // collapsed Outer, so no Replace is emitted.
        let box_1 = tree.get(&Key::int(11)).expect("Box 1 present");

        assert!(!box_1.has_children, "Box 1 flipped branch→leaf internally");
        assert_eq!(box_1.is_expanded, None, "Box 1 has no expansion state");

        let box_2 = tree.get(&Key::int(12)).expect("Box 2 present");

        assert!(box_2.has_children, "Box 2 flipped leaf→branch internally");
        assert_eq!(
            box_2.is_expanded,
            Some(false),
            "Box 2 is now a collapsed branch",
        );

        assert!(
            tree.drain_changes().is_empty(),
            "no event when every affected parent is hidden; got {:?}",
            tree.drain_changes(),
        );
    }

    #[test]
    fn tree_reorder_under_collapsed_parent_emits_no_event() {
        // Reorder hidden siblings under the collapsed Root A.
        // Visibility is preserved (still hidden) so no event.
        let mut tree = mixed_visibility_tree();

        let indices = tree.reorder(&Key::int(11), 1);

        assert!(indices.is_some(), "reorder succeeded internally");

        let drained = tree.drain_changes();

        assert!(
            drained.is_empty(),
            "no event when reordering hidden siblings; got {drained:?}"
        );
    }

    #[test]
    fn tree_reorder_visible_emits_visible_index_move() {
        // In the mixed-visibility fixture, the only expanded subtree
        // with multiple siblings would be a manufactured one. Build a
        // small tree where Root B has two visible children we can swap.
        let mut tree = MutableTreeData::new(TreeCollection::new([TreeItemConfig {
            key: Key::int(2),
            text_value: "Root B".to_string(),
            value: Item::new(2, "Root B"),
            children: vec![
                TreeItemConfig {
                    key: Key::int(21),
                    text_value: "Child B1".to_string(),
                    value: Item::new(21, "Child B1"),
                    children: vec![],
                    default_expanded: true,
                },
                TreeItemConfig {
                    key: Key::int(22),
                    text_value: "Child B2".to_string(),
                    value: Item::new(22, "Child B2"),
                    children: vec![],
                    default_expanded: true,
                },
            ],
            default_expanded: true,
        }]));

        // Visible: [Root B (0), Child B1 (1), Child B2 (2)].
        // Move Child B1 to sibling_index 1 → it becomes the second
        // visible child (visible index 2).
        let indices = tree.reorder(&Key::int(21), 1);

        assert_eq!(indices, Some((1, 2)), "flat indices match visible here");

        let drained = tree.drain_changes();

        assert_eq!(
            drained,
            vec![CollectionChange::Move {
                key: Key::int(21),
                from_index: 1,
                to_index: 2,
            }],
            "visible reorder uses visible iteration indices"
        );
    }

    #[test]
    fn tree_delegates_collection_methods() {
        let mut tree = sample_tree();

        assert_eq!(tree.size(), 5);
        assert!(!tree.is_empty());
        assert!(tree.get(&Key::int(11)).is_some());
        assert!(tree.get(&Key::int(999)).is_none());
        assert_eq!(tree.get_by_index(0).expect("idx 0").key, Key::int(1));
        assert_eq!(tree.first_key(), Some(&Key::int(1)));
        assert_eq!(tree.last_key(), Some(&Key::int(21)));
        assert_eq!(tree.key_after(&Key::int(1)), Some(&Key::int(11)));
        assert_eq!(tree.key_before(&Key::int(11)), Some(&Key::int(1)));

        // No-wrap variants: past-the-end / before-the-start return None.
        assert_eq!(tree.key_after_no_wrap(&Key::int(21)), None);
        assert_eq!(tree.key_before_no_wrap(&Key::int(1)), None);
        // And behave like their wrapping counterparts mid-collection.
        assert_eq!(tree.key_after_no_wrap(&Key::int(1)), Some(&Key::int(11)),);
        assert_eq!(tree.key_before_no_wrap(&Key::int(21)), Some(&Key::int(2)),);

        let keys = tree.keys().cloned().collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                Key::int(1),
                Key::int(11),
                Key::int(12),
                Key::int(2),
                Key::int(21),
            ],
        );
        assert_eq!(tree.nodes().count(), 5);

        // Direct children of Root A (key 1).
        let child_keys = tree
            .children_of(&Key::int(1))
            .map(|n| n.key.clone())
            .collect::<Vec<_>>();

        assert_eq!(child_keys, vec![Key::int(11), Key::int(12)]);

        // Reads do not populate the change buffer.
        assert!(tree.drain_changes().is_empty());
    }

    // -----------------------------------------------------------------------
    // Debug impls (coverage + smoke test for non-Debug payload types)
    // -----------------------------------------------------------------------

    #[test]
    fn list_debug_includes_type_name_and_pending_count() {
        let mut list = list_of_three();

        // Queue one pending change so the debug count isn't 0.
        list.push(Item::new(4, "Date"));

        let debug = format!("{list:?}");

        assert!(debug.contains("MutableListData"), "got: {debug}");
        assert!(
            debug.contains("pending_changes"),
            "debug should show pending_changes field: {debug}"
        );
        assert!(debug.contains("StaticCollection"), "got: {debug}");
    }

    #[test]
    fn tree_debug_includes_type_name_and_pending_count() {
        let mut tree = sample_tree();

        tree.replace(Item::new(11, "X"));

        let debug = format!("{tree:?}");

        assert!(debug.contains("MutableTreeData"), "got: {debug}");
        assert!(
            debug.contains("pending_changes"),
            "debug should show pending_changes field: {debug}"
        );
        assert!(debug.contains("TreeCollection"), "got: {debug}");
    }

    // -----------------------------------------------------------------------
    // reparent cycle rejection — the inner's descendant-of-moved check
    // runs after extraction and triggers a subtree restore, which must not
    // emit a change event from the wrapper.
    // -----------------------------------------------------------------------

    #[test]
    fn tree_reparent_to_descendant_returns_none_and_restores_tree() {
        // Build a tree with a 3-level chain so we can try to reparent a node
        // under its own descendant:
        //   1 Root
        //   └── 2 Mid
        //       └── 3 Leaf
        let inner = TreeCollection::new([tree_config(
            1,
            "Root",
            vec![tree_config(2, "Mid", vec![tree_config(3, "Leaf", vec![])])],
        )]);

        let mut tree = MutableTreeData::new(inner);

        // Try to reparent Mid(2) under Leaf(3) — a descendant of Mid —
        // which would create a cycle. The inner detects this after
        // extraction and restores the subtree.
        let indices = tree.reparent(&Key::int(2), Some(&Key::int(3)), 0);

        assert_eq!(indices, None, "cycle-creating reparent must return None");
        assert_eq!(tree.size(), 3, "tree size unchanged after restore");
        assert!(
            tree.drain_changes().is_empty(),
            "no change event for rejected reparent"
        );

        // The tree structure is intact: Root -> Mid -> Leaf still in place.
        let keys = tree.keys().cloned().collect::<Vec<_>>();

        assert_eq!(keys, vec![Key::int(1), Key::int(2), Key::int(3)]);
    }

    // -----------------------------------------------------------------------
    // CollectionChange — derives
    // -----------------------------------------------------------------------

    #[test]
    fn collection_change_derives_clone_debug_partial_eq() {
        // Exercise Clone + PartialEq + Debug for every variant so the
        // derives are actually instantiated for each shape.
        let variants = [
            CollectionChange::Insert { index: 2, count: 1 },
            CollectionChange::Remove {
                keys: vec![Key::int(1)],
            },
            CollectionChange::Move {
                key: Key::int(1),
                from_index: 0,
                to_index: 1,
            },
            CollectionChange::Replace { key: Key::int(1) },
            CollectionChange::Reset,
        ];

        let expected_debug = ["Insert", "Remove", "Move", "Replace", "Reset"];

        for (variant, tag) in variants.iter().zip(expected_debug.iter()) {
            assert_eq!(variant.clone(), *variant, "PartialEq + Clone roundtrip");

            let debug = format!("{variant:?}");

            assert!(
                debug.contains(tag),
                "Debug for {tag} includes variant name; got {debug}"
            );
        }
    }
}
