// ars-collections/src/filtered_collection.rs

use alloc::{collections::BTreeSet, vec::Vec};
use core::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use crate::{Collection, key::Key, node::Node};

/// A read-only view over another collection with a predicate applied.
///
/// Items that do not satisfy the predicate are excluded from all iteration
/// and navigation methods. Selection state (held by the component context)
/// is unaffected by filtering — selected keys that are currently hidden by
/// the filter remain in `selection::State::selected_keys`. When the filter is
/// cleared, those items are still shown as selected.
///
/// **Highlight state behavior**: When the filtered set changes (e.g., user
/// types in a Combobox), the component's `focused_key` may point to an
/// item that is no longer visible. Components MUST reset `focused_key`
/// to `filtered.first_key()` whenever the filter predicate changes and the
/// current highlight is not in the new visible set. This prevents
/// `aria-activedescendant` from referencing a hidden DOM element.
///
/// Clone: the struct stores only indices and a reference — no closures.
/// The predicate is consumed during `new()` and not retained.
pub struct FilteredCollection<'a, T, C>
where
    C: Collection<T>,
{
    inner: &'a C,

    /// Flat indices of inner nodes that pass the predicate.
    /// Uses `BTreeSet` for O(log n) `contains()` in `get()`.
    visible_indices: BTreeSet<usize>,

    /// Ordered list of visible indices for positional access in `get_by_index()`.
    visible_order: Vec<usize>,

    /// Cached index of the first visible focusable item.
    first_focusable: Option<usize>,

    /// Cached index of the last visible focusable item.
    last_focusable: Option<usize>,

    _phantom: PhantomData<T>,
}

impl<'a, T: Clone, C: Collection<T>> FilteredCollection<'a, T, C> {
    /// Apply `predicate` to each item node in `inner`.
    ///
    /// Section/Header/Separator nodes are included only when at least one of
    /// their children passes the predicate.
    pub fn new(inner: &'a C, predicate: impl Fn(&Node<T>) -> bool) -> Self {
        // First pass: find all item nodes that pass.
        // Uses node.index (the stable flat index from the inner collection)
        // rather than iterator position, so this works correctly when the
        // inner collection is itself a wrapper (e.g., SortedCollection) whose
        // traversal order differs from index order.
        let passing = inner
            .nodes()
            .filter(|n| n.is_focusable() && predicate(n))
            .map(|n| n.index)
            .collect::<BTreeSet<_>>();

        // Second pass: include structural nodes whose section group has passing items.
        // - Section nodes: included when at least one direct child passes.
        // - Header/Separator nodes inside a section: included when their parent
        //   section has at least one passing child (they have no children of their
        //   own, so checking `children_of(&header_key)` would always be empty).
        let visible_order = inner
            .nodes()
            .filter(|n| {
                if n.is_focusable() {
                    passing.contains(&n.index)
                } else {
                    // Section nodes own the children — check directly.
                    inner
                        .children_of(&n.key)
                        .any(|child| passing.contains(&child.index))
                        // Header/Separator nodes: check their parent section's children.
                        || n.parent_key.as_ref().is_some_and(|pk| {
                            inner
                                .children_of(pk)
                                .any(|child| passing.contains(&child.index))
                        })
                }
            })
            .map(|n| n.index)
            .collect::<Vec<_>>();

        let visible_indices = visible_order.iter().copied().collect::<BTreeSet<_>>();

        let first_focusable = visible_order
            .iter()
            .copied()
            .find(|&i| inner.get_by_index(i).is_some_and(Node::is_focusable));

        let last_focusable = visible_order
            .iter()
            .rev()
            .copied()
            .find(|&i| inner.get_by_index(i).is_some_and(Node::is_focusable));

        Self {
            inner,
            visible_indices,
            visible_order,
            first_focusable,
            last_focusable,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Clone, C: Collection<T>> Collection<T> for FilteredCollection<'a, T, C> {
    fn size(&self) -> usize {
        self.visible_indices.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        // Only return nodes that are in the visible set.
        let node = self.inner.get(key)?;
        if self.visible_indices.contains(&node.index) {
            Some(node)
        } else {
            None
        }
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.visible_order
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
        let current_index = self.inner.get(key)?.index;

        let pos = self
            .visible_order
            .iter()
            .position(|&i| i == current_index)?;

        self.visible_order[pos + 1..].iter().find_map(|&i| {
            self.inner
                .get_by_index(i)
                .filter(|n| n.is_focusable())
                .map(|n| &n.key)
        })
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let current_index = self.inner.get(key)?.index;

        let pos = self
            .visible_order
            .iter()
            .position(|&i| i == current_index)?;

        self.visible_order[..pos].iter().rev().find_map(|&i| {
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
        // Iterate visible_order (Vec) to preserve the inner collection's
        // traversal order, not visible_indices (BTreeSet) which re-sorts by
        // numeric index. This ensures nodes()/keys() agree with
        // get_by_index()/key_after() when wrapping a SortedCollection.
        self.visible_order
            .iter()
            .filter_map(|&i| self.inner.get_by_index(i))
    }

    fn children_of<'b>(&'b self, parent_key: &Key) -> impl Iterator<Item = &'b Node<T>>
    where
        T: 'b,
    {
        self.inner
            .children_of(parent_key)
            .filter(|n| self.visible_indices.contains(&n.index))
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints visible count only.
impl<T, C: Collection<T>> Debug for FilteredCollection<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilteredCollection")
            .field("visible", &self.visible_indices.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builder::CollectionBuilder, key::Key, node::NodeType};

    // ------------------------------------------------------------------ //
    // Construction                                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn filter_empty_collection() {
        let inner = CollectionBuilder::<&str>::new().build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.size(), 0);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_all_pass() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.size(), 3);
    }

    #[test]
    fn filter_none_pass() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| false);

        assert_eq!(filtered.size(), 0);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_some_items() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value.starts_with('A'));

        assert_eq!(filtered.size(), 1);

        let keys = filtered.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec![&Key::int(1)]);
    }

    #[test]
    fn filter_preserves_structural_with_passing_children() {
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .build();

        // Only Apple passes — section and header should still be included.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        // Section + Header + Apple = 3
        assert_eq!(filtered.size(), 3);
        assert!(filtered.get(&Key::str("fruits")).is_some());
        assert!(filtered.get(&Key::str("fruits-header")).is_some());
        assert!(filtered.get(&Key::int(1)).is_some());
        assert!(filtered.get(&Key::int(2)).is_none());
    }

    #[test]
    fn filter_removes_structural_without_passing_children() {
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .section(Key::str("vegs"), "Vegetables")
            .item(Key::int(2), "Carrot", "c")
            .end_section()
            .build();

        // Only Apple passes — "Vegetables" section should be removed entirely.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        assert!(filtered.get(&Key::str("fruits")).is_some());
        assert!(filtered.get(&Key::str("vegs")).is_none());
        assert!(filtered.get(&Key::int(1)).is_some());
        assert!(filtered.get(&Key::int(2)).is_none());
    }

    // ------------------------------------------------------------------ //
    // Random access                                                       //
    // ------------------------------------------------------------------ //

    #[test]
    fn get_returns_none_for_hidden_key() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        assert!(filtered.get(&Key::int(1)).is_some());
        assert!(filtered.get(&Key::int(2)).is_none());
    }

    #[test]
    fn get_by_index_maps_to_visible_order() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        // Keep only Apple and Cherry.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value != "Banana");

        assert_eq!(filtered.size(), 2);

        let first = filtered.get_by_index(0).expect("index 0");

        assert_eq!(first.key, Key::int(1));

        let second = filtered.get_by_index(1).expect("index 1");

        assert_eq!(second.key, Key::int(3));
        assert!(filtered.get_by_index(2).is_none());
    }

    #[test]
    fn contains_key_visible_and_hidden() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        assert!(filtered.contains_key(&Key::int(1)));
        assert!(!filtered.contains_key(&Key::int(2)));
    }

    // ------------------------------------------------------------------ //
    // Boundary navigation                                                 //
    // ------------------------------------------------------------------ //

    #[test]
    fn first_key_skips_structural() {
        let inner = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        // first_key should be the item, not section or header.
        assert_eq!(filtered.first_key(), Some(&Key::int(1)));
    }

    #[test]
    fn last_key_skips_structural() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .separator()
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.last_key(), Some(&Key::int(1)));
    }

    #[test]
    fn first_last_key_on_empty_filtered() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| false);

        assert_eq!(filtered.first_key(), None);
        assert_eq!(filtered.last_key(), None);
    }

    // ------------------------------------------------------------------ //
    // Sequential navigation                                               //
    // ------------------------------------------------------------------ //

    #[test]
    fn key_after_in_filtered_set() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        // Keep Apple and Cherry only.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value != "Banana");

        assert_eq!(filtered.key_after_no_wrap(&Key::int(1)), Some(&Key::int(3)));
    }

    #[test]
    fn key_after_wraps() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.key_after(&Key::int(2)), Some(&Key::int(1)));
    }

    #[test]
    fn key_before_wraps() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.key_before(&Key::int(1)), Some(&Key::int(2)));
    }

    #[test]
    fn key_after_no_wrap_at_end() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.key_after_no_wrap(&Key::int(2)), None);
    }

    #[test]
    fn key_before_no_wrap_at_start() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.key_before_no_wrap(&Key::int(1)), None);
    }

    #[test]
    fn key_before_no_wrap_finds_previous() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        // Keep Apple and Cherry only.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value != "Banana");

        assert_eq!(
            filtered.key_before_no_wrap(&Key::int(3)),
            Some(&Key::int(1))
        );
    }

    // ------------------------------------------------------------------ //
    // Iteration                                                           //
    // ------------------------------------------------------------------ //

    #[test]
    fn keys_yields_visible_only() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value != "Banana");

        let keys = filtered.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec![&Key::int(1), &Key::int(3)]);
    }

    #[test]
    fn nodes_yields_visible_only() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Banana");

        let nodes = filtered.nodes().collect::<Vec<_>>();

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].key, Key::int(2));
    }

    #[test]
    fn children_of_filtered() {
        let inner = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .build();

        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        let children = filtered
            .children_of(&Key::str("fruits"))
            .collect::<Vec<_>>();

        // Header + Apple = 2 (Banana is filtered out).
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].node_type, NodeType::Header);
        assert_eq!(children[1].key, Key::int(1));
    }

    #[test]
    fn item_keys_filters_structural_from_visible() {
        let inner = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        let item_keys = filtered.item_keys().collect::<Vec<_>>();

        // Should only include item keys, not section/header.
        assert_eq!(item_keys, vec![&Key::int(1), &Key::int(2)]);
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

        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        let debug = alloc::format!("{filtered:?}");

        assert!(debug.contains("FilteredCollection"));
        assert!(debug.contains("1"));
    }
}
