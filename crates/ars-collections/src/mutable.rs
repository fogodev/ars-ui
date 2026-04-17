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

use alloc::vec::Vec;
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

    /// An item's data was replaced in-place (key unchanged).
    Replace {
        /// Key of the replaced item.
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

    /// Remove items with the given keys, returning their owned values in
    /// iteration order. Emits [`CollectionChange::Remove`] carrying the keys
    /// that actually matched an item — unknown keys are silently skipped by
    /// the inner collection, so they are excluded from the change event to
    /// keep the change log truthful. Returns an empty `Vec` (and emits no
    /// event) when no key in `keys` matches.
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        let removed = self.inner.remove_by_keys(keys);

        if !removed.is_empty() {
            self.pending_changes.push(CollectionChange::Remove {
                keys: removed.iter().map(|t| t.key().clone()).collect(),
            });
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
    /// insert (unknown parent key). `CollectionChange::Insert` is emitted
    /// only on success, so the change log never references a phantom node.
    pub fn insert_child(&mut self, parent: Option<&Key>, index: usize, item: T) -> Option<usize> {
        let flat_index = self.inner.insert_child(parent, index, item)?;

        self.pending_changes.push(CollectionChange::Insert {
            index: flat_index,
            count: 1,
        });

        Some(flat_index)
    }

    /// Remove the listed nodes (and their descendants), returning their
    /// owned values. Emits [`CollectionChange::Remove`] carrying only the
    /// keys that actually matched an item — unknown keys are silently
    /// skipped by the inner collection and excluded from the change event.
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        let removed = self.inner.remove_by_keys(keys);

        if !removed.is_empty() {
            self.pending_changes.push(CollectionChange::Remove {
                keys: removed.iter().map(|t| t.key().clone()).collect(),
            });
        }

        removed
    }

    /// Move a node (with its subtree) under a new parent at the given
    /// sibling index.
    ///
    /// Returns `Some((from_flat_index, to_flat_index))` on success, or
    /// `None` when the inner [`TreeCollection::reparent`] rejected the
    /// move (unknown key, unknown `new_parent`, or `new_parent` is a
    /// descendant of `key`). `CollectionChange::Move` is emitted only on
    /// success.
    pub fn reparent(
        &mut self,
        key: &Key,
        new_parent: Option<&Key>,
        index: usize,
    ) -> Option<(usize, usize)> {
        let (from_index, to_index) = self.inner.reparent(key, new_parent, index)?;

        self.pending_changes.push(CollectionChange::Move {
            key: key.clone(),
            from_index,
            to_index,
        });

        Some((from_index, to_index))
    }

    /// Reorder a node among its existing siblings (same parent).
    ///
    /// Returns `Some((from_flat_index, to_flat_index))` on success, or
    /// `None` when `key` does not exist in the tree. `CollectionChange::Move`
    /// is emitted only on success.
    pub fn reorder(&mut self, key: &Key, to_sibling_index: usize) -> Option<(usize, usize)> {
        let (from_index, to_index) = self.inner.reorder_sibling(key, to_sibling_index)?;

        self.pending_changes.push(CollectionChange::Move {
            key: key.clone(),
            from_index,
            to_index,
        });

        Some((from_index, to_index))
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
