// ars-collections/src/sorted_collection.rs

use alloc::{collections::BTreeMap, vec::Vec};
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
/// Clone — this struct holds only a reference and sorted indices, no closures.
/// The comparator is consumed at construction time and not retained.
pub struct SortedCollection<'a, T, C>
where
    C: Collection<T>,
{
    inner: &'a C,
    /// Sorted flat indices of inner nodes in the new traversal order.
    sorted_indices: Vec<usize>,
    /// Cached index of the first focusable item in sorted order.
    first_focusable: Option<usize>,
    /// Cached index of the last focusable item in sorted order.
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
        let mut item_indices = inner
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.index)
            .collect::<Vec<_>>();

        item_indices.sort_by(|&a, &b| {
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
        // item_indices IS the final sorted order.
        //
        // For sectioned collections: sort items within each section while
        // preserving section order and structural node positions. Algorithm:
        // 1. Group sorted items by parent_key (section membership).
        // 2. Walk the original node order. Structural nodes (Section, Header,
        //    Separator) are emitted in their original positions. When items
        //    belonging to a section are encountered, emit the section's sorted
        //    items instead, consuming from the group.
        let has_sections = inner.nodes().any(Node::is_structural);

        let sorted_indices = if has_sections {
            // Group sorted item indices by parent_key.
            let mut section_items = BTreeMap::<Option<Key>, Vec<usize>>::new();
            for &idx in &item_indices {
                let node = inner
                    .get_by_index(idx)
                    .expect("sorted index must be within bounds");
                section_items
                    .entry(node.parent_key.clone())
                    .or_default()
                    .push(idx);
            }
            // Track consumption position per section.
            let mut section_cursors = BTreeMap::<Option<Key>, usize>::new();

            let mut result = Vec::with_capacity(inner.size());
            let mut current_section = None::<Key>;
            let mut items_emitted_for_section = false;

            for node in inner.nodes() {
                match node.node_type {
                    NodeType::Section => {
                        current_section = Some(node.key.clone());
                        items_emitted_for_section = false;
                        result.push(node.index);
                    }
                    NodeType::Header | NodeType::Separator => {
                        // Emit structural nodes in their original position.
                        result.push(node.index);
                    }
                    NodeType::Item => {
                        // On first item of this section, emit all sorted items for it.
                        let section_key = node.parent_key.clone();
                        if !items_emitted_for_section || section_key != current_section {
                            if let Some(items) = section_items.get(&section_key) {
                                let cursor =
                                    section_cursors.entry(section_key.clone()).or_insert(0);
                                if *cursor == 0 {
                                    // First encounter: emit all sorted items for this section.
                                    result.extend_from_slice(items);
                                    *cursor = items.len();
                                }
                            }
                            items_emitted_for_section = true;
                            current_section = section_key;
                        }
                        // Skip original item indices — already emitted via sorted group.
                    }
                }
            }

            result
        } else {
            // Fast path: flat collection — sorted items are the full order.
            item_indices
        };

        let first_focusable = sorted_indices
            .iter()
            .copied()
            .find(|&i| inner.get_by_index(i).is_some_and(Node::is_focusable));
        let last_focusable = sorted_indices
            .iter()
            .rev()
            .copied()
            .find(|&i| inner.get_by_index(i).is_some_and(Node::is_focusable));

        Self {
            inner,
            sorted_indices,
            first_focusable,
            last_focusable,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, C: Collection<T>> Collection<T> for SortedCollection<'a, T, C> {
    fn size(&self) -> usize {
        self.sorted_indices.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.inner.get(key)
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.sorted_indices
            .get(index)
            .and_then(|&i| self.inner.get_by_index(i))
    }

    fn first_key(&self) -> Option<&Key> {
        self.first_focusable
            .and_then(|i| self.inner.get_by_index(i))
            .map(|n| &n.key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable
            .and_then(|i| self.inner.get_by_index(i))
            .map(|n| &n.key)
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.key_after_no_wrap(key).or_else(|| self.first_key())
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.key_before_no_wrap(key).or_else(|| self.last_key())
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        let current = self.inner.get(key)?.index;
        let pos = self.sorted_indices.iter().position(|&i| i == current)?;
        self.sorted_indices[pos + 1..].iter().find_map(|&i| {
            self.inner
                .get_by_index(i)
                .filter(|n| n.is_focusable())
                .map(|n| &n.key)
        })
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let current = self.inner.get(key)?.index;
        let pos = self.sorted_indices.iter().position(|&i| i == current)?;
        self.sorted_indices[..pos].iter().rev().find_map(|&i| {
            self.inner
                .get_by_index(i)
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
        self.sorted_indices
            .iter()
            .filter_map(|&i| self.inner.get_by_index(i))
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
            .field("sorted_count", &self.sorted_indices.len())
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
}
