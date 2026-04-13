// ars-collections/src/sorted_collection.rs

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{
    cmp,
    fmt::{self, Debug, Display},
    marker::PhantomData,
};

use crate::{
    Collection,
    key::Key,
    node::{Node, NodeType},
};

/// The direction of a sort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Sort in ascending order (smallest first).
    Ascending,
    /// Sort in descending order (largest first).
    Descending,
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortDirection::Ascending => f.write_str("ascending"),
            SortDirection::Descending => f.write_str("descending"),
        }
    }
}

/// Unified sort state for table columns.
///
/// `K` is the column key type — typically the same `Key` used by the
/// collection, but any type works (e.g., a string column identifier).
#[derive(Clone, Debug, PartialEq)]
pub struct SortDescriptor<K> {
    /// The column (or field) being sorted.
    pub column: K,
    /// Whether the sort is ascending or descending.
    pub direction: SortDirection,
}

/// A read-only view over another collection with a comparator applied to item
/// nodes. Structural nodes (sections, headers, separators) retain their
/// relative order with respect to the items following them.
///
/// For locale-aware sorting, integrate `ars_i18n::StringCollator` as the comparator:
///
/// ```rust,ignore
/// let collator = ars_i18n::StringCollator::new(&locale, Default::default());
/// let sorted = SortedCollection::new(&collection, |a, b| {
///     collator.compare(&a.text_value, &b.text_value)
/// });
/// ```
///
/// ## Coordinate systems
///
/// This wrapper maintains two coordinate systems:
/// - **Base indices** (`node.index`): the stable identity from the underlying
///   base collection, used for key→position lookups during navigation.
/// - **Wrapper positions**: positions into `inner`'s iteration order (i.e.,
///   arguments to `inner.get_by_index()`), stored in `sorted_positions`.
///   This distinction is critical when `inner` is itself a wrapper (e.g.,
///   [`FilteredCollection`]) whose `get_by_index` expects dense positions,
///   not sparse base indices.
///
/// The comparator is consumed at construction time and not retained.
pub struct SortedCollection<'a, T, C>
where
    C: Collection<T>,
{
    inner: &'a C,

    /// Wrapper positions (into `inner`) in sorted traversal order.
    /// These are the coordinates that `inner.get_by_index()` expects.
    sorted_positions: Vec<usize>,

    /// Maps base `node.index` → index into `sorted_positions`, for O(log n)
    /// lookup when navigating from a key.
    base_to_sorted_pos: BTreeMap<usize, usize>,

    /// Cached index into `sorted_positions` of the first focusable item.
    first_focusable: Option<usize>,

    /// Cached index into `sorted_positions` of the last focusable item.
    last_focusable: Option<usize>,

    _phantom: PhantomData<T>,
}

impl<'a, T: Clone, C: Collection<T>> SortedCollection<'a, T, C> {
    /// Construct a sorted view.
    ///
    /// `comparator` is called only for `Item` nodes. Section, Header, and
    /// Separator nodes are left in their original relative positions within
    /// their grouping. For flat (non-sectioned) collections, all nodes are
    /// reordered.
    pub fn new(inner: &'a C, comparator: impl Fn(&Node<T>, &Node<T>) -> cmp::Ordering) -> Self {
        // Collect item wrapper positions and sort them by comparator.
        let mut item_positions = inner
            .nodes()
            .enumerate()
            .filter(|(_, n)| n.is_focusable())
            .map(|(pos, _)| pos)
            .collect::<Vec<_>>();

        item_positions.sort_by(|&a, &b| {
            let node_a = inner
                .get_by_index(a)
                .expect("sort index must be within collection bounds");

            let node_b = inner
                .get_by_index(b)
                .expect("sort index must be within collection bounds");

            comparator(node_a, node_b)
        });

        // Interleave structural nodes back into sorted order.
        //
        // For flat (non-sectioned) collections: all nodes are Items, so
        // item_positions IS the final sorted order.
        //
        // For sectioned or mixed collections: items are sorted within each
        // contiguous *run* of same-parent items. A new run starts when a
        // Section node is encountered or an Item's parent_key differs from
        // the previous Item's parent. Header and Separator nodes do not
        // break runs — they are emitted in their original positions.
        //
        // This run-aware grouping prevents items from migrating across
        // structural boundaries when top-level items appear in multiple
        // runs separated by sections.
        let has_sections = inner.nodes().any(Node::is_structural);

        let sorted_positions = if has_sections {
            // Phase 1: assign each item to a contiguous-run group, keyed by
            // wrapper position.
            let mut item_to_group = BTreeMap::<usize, usize>::new();
            let mut next_group = 0;
            let mut current_parent = None::<Option<Key>>;

            for (pos, node) in inner.nodes().enumerate() {
                match node.node_type {
                    NodeType::Section | NodeType::Separator => {
                        // Section and Separator both act as hard run boundaries.
                        // Items on opposite sides MUST NOT merge into one sorted group,
                        // even when they share the same parent_key, or sorting would
                        // pull items across the visual divider.
                        current_parent = None;
                    }

                    NodeType::Header => {
                        // Headers appear immediately after their Section and are
                        // never run boundaries — the Section already reset scope.
                    }

                    NodeType::Item => {
                        let pk = node.parent_key.clone();
                        match &current_parent {
                            Some(existing_pk) if *existing_pk == pk => {}
                            _ => {
                                next_group += 1;
                                current_parent = Some(pk);
                            }
                        }
                        item_to_group.insert(pos, next_group);
                    }
                }
            }

            // Phase 2: distribute sorted item wrapper positions into per-group buckets.
            // Every wrapper position in item_positions was added to item_to_group
            // in Phase 1 (both iterate focusable items), so the map lookup always
            // succeeds — `.expect` documents the invariant.
            let mut groups = BTreeMap::<usize, Vec<usize>>::new();
            for &wp in &item_positions {
                let group = *item_to_group
                    .get(&wp)
                    .expect("item position must have been assigned a group");
                groups.entry(group).or_default().push(wp);
            }

            // Phase 3: walk original order, emit structural nodes in place,
            // emit each group's sorted items on first encounter. Both map
            // lookups are guaranteed by Phase 1/2 construction invariants.
            let mut group_emitted = BTreeSet::<usize>::new();
            let mut result = Vec::with_capacity(inner.size());

            for (pos, node) in inner.nodes().enumerate() {
                match node.node_type {
                    NodeType::Section | NodeType::Header | NodeType::Separator => {
                        result.push(pos);
                    }
                    NodeType::Item => {
                        let group = *item_to_group
                            .get(&pos)
                            .expect("item position must have been assigned a group");
                        if group_emitted.insert(group) {
                            let items = groups
                                .get(&group)
                                .expect("group must have been populated in Phase 2");
                            result.extend_from_slice(items);
                        }
                    }
                }
            }

            result
        } else {
            // Fast path: flat collection — sorted items are the full order.
            item_positions
        };

        // Build reverse map: base node.index → position in sorted_positions.
        let base_to_sorted_pos = sorted_positions
            .iter()
            .enumerate()
            .filter_map(|(sp_idx, &wp)| inner.get_by_index(wp).map(|n| (n.index, sp_idx)))
            .collect::<BTreeMap<_, _>>();

        let first_focusable = sorted_positions.iter().enumerate().find_map(|(si, &wp)| {
            inner
                .get_by_index(wp)
                .filter(|n| n.is_focusable())
                .map(|_| si)
        });

        let last_focusable = sorted_positions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(si, &wp)| {
                inner
                    .get_by_index(wp)
                    .filter(|n| n.is_focusable())
                    .map(|_| si)
            });

        Self {
            inner,
            sorted_positions,
            base_to_sorted_pos,
            first_focusable,
            last_focusable,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, C: Collection<T>> Collection<T> for SortedCollection<'a, T, C> {
    fn size(&self) -> usize {
        self.sorted_positions.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.inner.get(key)
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.sorted_positions
            .get(index)
            .and_then(|&wp| self.inner.get_by_index(wp))
    }

    fn first_key(&self) -> Option<&Key> {
        self.first_focusable
            .and_then(|si| self.sorted_positions.get(si))
            .and_then(|&wp| self.inner.get_by_index(wp))
            .map(|n| &n.key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable
            .and_then(|si| self.sorted_positions.get(si))
            .and_then(|&wp| self.inner.get_by_index(wp))
            .map(|n| &n.key)
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.key_after_no_wrap(key).or_else(|| self.first_key())
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.key_before_no_wrap(key).or_else(|| self.last_key())
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        let node = self.inner.get(key)?;

        let sorted_pos = *self.base_to_sorted_pos.get(&node.index)?;

        self.sorted_positions[sorted_pos + 1..]
            .iter()
            .find_map(|&wp| {
                self.inner
                    .get_by_index(wp)
                    .filter(|n| n.is_focusable())
                    .map(|n| &n.key)
            })
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let node = self.inner.get(key)?;

        let sorted_pos = *self.base_to_sorted_pos.get(&node.index)?;

        self.sorted_positions[..sorted_pos]
            .iter()
            .rev()
            .find_map(|&wp| {
                self.inner
                    .get_by_index(wp)
                    .filter(|n| n.is_focusable())
                    .map(|n| &n.key)
            })
    }

    fn keys<'b>(&'b self) -> impl Iterator<Item = &'b Key>
    where
        T: 'b,
    {
        self.nodes().map(|n| &n.key)
    }

    fn nodes<'b>(&'b self) -> impl Iterator<Item = &'b Node<T>>
    where
        T: 'b,
    {
        self.sorted_positions
            .iter()
            .filter_map(|&wp| self.inner.get_by_index(wp))
    }

    fn children_of<'b>(&'b self, parent_key: &Key) -> impl Iterator<Item = &'b Node<T>>
    where
        T: 'b,
    {
        self.inner.children_of(parent_key)
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints sorted count only.
impl<T, C: Collection<T>> Debug for SortedCollection<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SortedCollection")
            .field("sorted_count", &self.sorted_positions.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builder::CollectionBuilder, key::Key};

    // ------------------------------------------------------------------ //
    // SortDirection                                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn ascending_display() {
        assert_eq!(alloc::format!("{}", SortDirection::Ascending), "ascending");
    }

    #[test]
    fn descending_display() {
        assert_eq!(
            alloc::format!("{}", SortDirection::Descending),
            "descending"
        );
    }

    #[test]
    fn sort_direction_clone_copy_eq() {
        let a = SortDirection::Ascending;

        let b = a;

        assert_eq!(a, b);
    }

    // ------------------------------------------------------------------ //
    // SortDescriptor                                                      //
    // ------------------------------------------------------------------ //

    #[test]
    fn sort_descriptor_creation() {
        let desc = SortDescriptor {
            column: Key::str("name"),
            direction: SortDirection::Ascending,
        };

        assert_eq!(desc.column, Key::str("name"));
        assert_eq!(desc.direction, SortDirection::Ascending);
    }

    #[test]
    fn sort_descriptor_clone_eq() {
        let a = SortDescriptor {
            column: "name",
            direction: SortDirection::Descending,
        };

        let b = a.clone();

        assert_eq!(a, b);
    }

    // ------------------------------------------------------------------ //
    // Construction (flat collections)                                      //
    // ------------------------------------------------------------------ //

    #[test]
    fn sort_empty_collection() {
        let inner = CollectionBuilder::<&str>::new().build();

        let sorted = SortedCollection::new(&inner, |_, _| cmp::Ordering::Equal);

        assert_eq!(sorted.size(), 0);
        assert!(sorted.is_empty());
    }

    #[test]
    fn sort_single_item() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.size(), 1);
        assert_eq!(sorted.first_key(), Some(&Key::int(1)));
    }

    #[test]
    fn sort_already_sorted() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let keys = sorted.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);
    }

    #[test]
    fn sort_reverse_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let keys = sorted.keys().collect::<Vec<_>>();

        // Apple (3), Banana (2), Cherry (1)
        assert_eq!(keys, vec![&Key::int(3), &Key::int(2), &Key::int(1)]);
    }

    #[test]
    fn sort_alphabetical_by_text() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Delta", "d")
            .item(Key::int(2), "Alpha", "a")
            .item(Key::int(3), "Charlie", "c")
            .item(Key::int(4), "Bravo", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let texts = sorted
            .nodes()
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["Alpha", "Bravo", "Charlie", "Delta"]);
    }

    // ------------------------------------------------------------------ //
    // Construction (sectioned collections)                                //
    // ------------------------------------------------------------------ //

    #[test]
    fn sort_preserves_section_structure() {
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .end_section()
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Section + Header + 2 items = 4
        assert_eq!(sorted.size(), 4);

        // Check node order: Section, Header, Apple, Cherry
        let node_types = sorted.nodes().map(|n| n.node_type).collect::<Vec<_>>();

        assert_eq!(
            node_types,
            vec![
                NodeType::Section,
                NodeType::Header,
                NodeType::Item,
                NodeType::Item,
            ]
        );

        let texts = sorted
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["Apple", "Cherry"]);
    }

    #[test]
    fn sort_within_multiple_sections() {
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .section(Key::str("vegs"), "Vegetables")
            .item(Key::int(4), "Carrot", "c")
            .item(Key::int(3), "Artichoke", "a")
            .end_section()
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Each section's items sorted independently.
        let item_texts = sorted
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(item_texts, vec!["Apple", "Banana", "Artichoke", "Carrot"]);
    }

    #[test]
    fn sort_section_with_separator() {
        let inner = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .separator()
            .item(Key::int(3), "Standalone", "s")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Separator stays in its original position.
        let types = sorted.nodes().map(|n| n.node_type).collect::<Vec<_>>();

        assert_eq!(
            types,
            vec![
                NodeType::Section,
                NodeType::Header,
                NodeType::Item, // Apple (sorted)
                NodeType::Item, // Banana (sorted)
                NodeType::Separator,
                NodeType::Item, // Standalone
            ]
        );
    }

    #[test]
    fn sort_disjoint_top_level_runs_stay_in_place() {
        // Top-level items separated by a section must NOT merge across
        // the structural boundary. Each run is sorted independently.
        //
        // Original order:
        //   Item D (top-level)   ← run 1
        //   Item B (top-level)   ← run 1
        //   Section "Fruits"
        //     Item Cherry
        //     Item Apple
        //   Item C (top-level)   ← run 2
        //   Item A (top-level)   ← run 2
        let inner = CollectionBuilder::new()
            .item(Key::int(4), "D", "d")
            .item(Key::int(2), "B", "b")
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(13), "Cherry", "cherry")
            .item(Key::int(11), "Apple", "apple")
            .end_section()
            .item(Key::int(3), "C", "c")
            .item(Key::int(1), "A", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let texts = sorted
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();

        // Run 1 sorted: B, D. Section items sorted: Apple, Cherry. Run 2 sorted: A, C.
        assert_eq!(texts, vec!["B", "D", "Apple", "Cherry", "A", "C"]);

        // Structural nodes stay in their original positions.
        let types = sorted.nodes().map(|n| n.node_type).collect::<Vec<_>>();
        assert_eq!(
            types,
            vec![
                NodeType::Item,    // B (run 1)
                NodeType::Item,    // D (run 1)
                NodeType::Section, // Fruits
                NodeType::Header,  // Fruits header
                NodeType::Item,    // Apple (section)
                NodeType::Item,    // Cherry (section)
                NodeType::Item,    // A (run 2)
                NodeType::Item,    // C (run 2)
            ]
        );
    }

    // ------------------------------------------------------------------ //
    // Random access                                                       //
    // ------------------------------------------------------------------ //

    #[test]
    fn get_by_key_unchanged() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // get() by key is unaffected by sort order.
        let node = sorted.get(&Key::int(1)).expect("key 1");

        assert_eq!(node.text_value, "Cherry");
    }

    #[test]
    fn get_by_index_reflects_sorted_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Index 0 should be Apple after sorting.
        let first = sorted.get_by_index(0).expect("index 0");

        assert_eq!(first.text_value, "Apple");

        let second = sorted.get_by_index(1).expect("index 1");

        assert_eq!(second.text_value, "Cherry");
    }

    // ------------------------------------------------------------------ //
    // Boundary navigation                                                 //
    // ------------------------------------------------------------------ //

    #[test]
    fn first_key_is_smallest_after_sort() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.first_key(), Some(&Key::int(2))); // Apple
    }

    #[test]
    fn last_key_is_largest_after_sort() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.last_key(), Some(&Key::int(1))); // Cherry
    }

    #[test]
    fn first_key_empty_collection() {
        let inner = CollectionBuilder::<&str>::new().build();

        let sorted = SortedCollection::new(&inner, |_, _| cmp::Ordering::Equal);

        assert_eq!(sorted.first_key(), None);
    }

    // ------------------------------------------------------------------ //
    // Sequential navigation                                               //
    // ------------------------------------------------------------------ //

    #[test]
    fn key_after_follows_sorted_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Sorted: Apple(2), Banana(3), Cherry(1)
        assert_eq!(sorted.key_after_no_wrap(&Key::int(2)), Some(&Key::int(3))); // Apple -> Banana
        assert_eq!(sorted.key_after_no_wrap(&Key::int(3)), Some(&Key::int(1))); // Banana -> Cherry
    }

    #[test]
    fn key_after_wraps() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Sorted: Apple(2), Cherry(1). After Cherry wraps to Apple.
        assert_eq!(sorted.key_after(&Key::int(1)), Some(&Key::int(2)));
    }

    #[test]
    fn key_before_wraps() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Sorted: Apple(2), Cherry(1). Before Apple wraps to Cherry.
        assert_eq!(sorted.key_before(&Key::int(2)), Some(&Key::int(1)));
    }

    #[test]
    fn key_after_no_wrap_at_end() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.key_after_no_wrap(&Key::int(1)), None); // Cherry is last
    }

    #[test]
    fn key_before_no_wrap_at_start() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.key_before_no_wrap(&Key::int(2)), None); // Apple is first
    }

    #[test]
    fn key_before_no_wrap_finds_previous() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // Sorted: Apple(2), Banana(3), Cherry(1). Before Cherry is Banana.
        assert_eq!(sorted.key_before_no_wrap(&Key::int(1)), Some(&Key::int(3)));
    }

    // ------------------------------------------------------------------ //
    // Iteration                                                           //
    // ------------------------------------------------------------------ //

    #[test]
    fn keys_in_sorted_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let keys = sorted.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec![&Key::int(2), &Key::int(3), &Key::int(1)]);
    }

    #[test]
    fn nodes_in_sorted_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let texts = sorted
            .nodes()
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(texts, vec!["Apple", "Cherry"]);
    }

    #[test]
    fn children_of_unchanged() {
        let inner = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        // children_of delegates to inner — original order (Header + 2 items).

        let children = sorted.children_of(&Key::str("sec")).collect::<Vec<_>>();

        assert_eq!(children.len(), 3);
    }

    // ------------------------------------------------------------------ //
    // Debug                                                               //
    // ------------------------------------------------------------------ //

    #[test]
    fn debug_format() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let debug = alloc::format!("{sorted:?}");

        assert!(debug.contains("SortedCollection"));
        assert!(debug.contains("2"));
    }

    // ------------------------------------------------------------------ //
    // Composition: SortedCollection over FilteredCollection               //
    // ------------------------------------------------------------------ //

    #[test]
    fn sort_over_filtered_uses_wrapper_positions() {
        use crate::FilteredCollection;

        let base = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .item(Key::int(4), "Date", "d")
            .build();

        // Filter out Apple — leaves Cherry(1), Banana(3), Date(4).
        let filtered = FilteredCollection::new(&base, |n| n.text_value != "Apple");

        // Sort the filtered view alphabetically.
        let sorted = SortedCollection::new(&filtered, |a, b| a.text_value.cmp(&b.text_value));

        assert_eq!(sorted.size(), 3);

        let texts = sorted
            .nodes()
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["Banana", "Cherry", "Date"]);

        // Positional access follows sorted order.
        assert_eq!(sorted.get_by_index(0).expect("idx 0").text_value, "Banana");
        assert_eq!(sorted.get_by_index(1).expect("idx 1").text_value, "Cherry");
        assert_eq!(sorted.get_by_index(2).expect("idx 2").text_value, "Date");

        // Navigation follows sorted order.
        assert_eq!(sorted.first_key(), Some(&Key::int(3))); // Banana
        assert_eq!(sorted.last_key(), Some(&Key::int(4))); // Date
        assert_eq!(sorted.key_after_no_wrap(&Key::int(3)), Some(&Key::int(1))); // Banana → Cherry
        assert_eq!(sorted.key_before_no_wrap(&Key::int(1)), Some(&Key::int(3))); // Cherry → Banana
    }

    #[test]
    fn sort_respects_separator_as_run_boundary() {
        // Top-level items with a separator between them must NOT merge into
        // one group. Each side sorts within itself, separator stays put.
        //
        // Original order:
        //   D, B, Separator, C, A  (all top-level)
        let inner = CollectionBuilder::new()
            .item(Key::int(4), "D", "d")
            .item(Key::int(2), "B", "b")
            .separator()
            .item(Key::int(3), "C", "c")
            .item(Key::int(1), "A", "a")
            .build();

        let sorted = SortedCollection::new(&inner, |a, b| a.text_value.cmp(&b.text_value));

        let types = sorted.nodes().map(|n| n.node_type).collect::<Vec<_>>();
        assert_eq!(
            types,
            vec![
                NodeType::Item,      // run 1
                NodeType::Item,      // run 1
                NodeType::Separator, // stays in place
                NodeType::Item,      // run 2
                NodeType::Item,      // run 2
            ]
        );

        let texts = sorted
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();
        // Run 1 (D, B) sorted → [B, D]. Run 2 (C, A) sorted → [A, C].
        assert_eq!(texts, vec!["B", "D", "A", "C"]);
    }
}
