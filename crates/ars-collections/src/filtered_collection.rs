// ars-collections/src/filtered_collection.rs

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use crate::{
    Collection,
    key::Key,
    node::{Node, NodeType},
};

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
/// ## Coordinate systems
///
/// This wrapper maintains two coordinate systems:
/// - **Base indices** (`node.index`): the stable identity from the underlying
///   base collection, used for O(log n) membership checks in [`get`] and
///   [`children_of`].
/// - **Wrapper positions**: positions into `inner`'s iteration order (i.e.,
///   arguments to `inner.get_by_index()`), used for [`get_by_index`],
///   [`nodes`], and navigation. This distinction is critical when `inner`
///   is itself a wrapper (e.g., [`SortedCollection`]) whose traversal order
///   differs from base index order.
///
/// The predicate is consumed during `new()` and not retained.
pub struct FilteredCollection<'a, T, C>
where
    C: Collection<T>,
{
    inner: &'a C,

    /// Base `node.index` values of visible nodes.
    /// Used for O(log n) membership checks in `get()` and `children_of()`.
    visible_base_indices: BTreeSet<usize>,

    /// Wrapper positions (into `inner`) of visible nodes, in traversal order.
    /// Used by `get_by_index()` and `nodes()` — these are the coordinates
    /// that `inner.get_by_index()` expects.
    visible_positions: Vec<usize>,

    /// Maps base `node.index` → index into `visible_positions`, for O(log n)
    /// lookup when navigating from a key.
    base_to_visible_pos: BTreeMap<usize, usize>,

    /// Cached index into `visible_positions` of the first focusable item.
    first_focusable: Option<usize>,

    /// Cached index into `visible_positions` of the last focusable item.
    last_focusable: Option<usize>,

    _phantom: PhantomData<T>,
}

impl<'a, T: Clone, C: Collection<T>> FilteredCollection<'a, T, C> {
    /// Apply `predicate` to each item node in `inner`.
    ///
    /// Section/Header/Separator nodes are included only when at least one of
    /// their children passes the predicate.
    pub fn new(inner: &'a C, predicate: impl Fn(&Node<T>) -> bool) -> Self {
        // First pass: find all item nodes that pass, keyed by base index.
        let passing = inner
            .nodes()
            .filter(|n| n.is_focusable() && predicate(n))
            .map(|n| n.index)
            .collect::<BTreeSet<_>>();

        // Second pass: collect (wrapper_position, base_index) for visible nodes.
        // Structural nodes are included based on the scope they belong to:
        // - Section nodes: scope is their direct children.
        // - Header/Separator inside a section: scope is the parent section's
        //   children (they have no children of their own).
        // - Top-level Header/Separator: scope is all top-level items. Without
        //   this branch a no-op predicate (`|_| true`) would drop top-level
        //   separators and silently change the collection shape.
        let visible_data = inner
            .nodes()
            .enumerate()
            .filter(|(_, n)| {
                if n.is_focusable() {
                    return passing.contains(&n.index);
                }
                match (&n.parent_key, n.node_type) {
                    (_, NodeType::Section) => inner
                        .children_of(&n.key)
                        .any(|child| passing.contains(&child.index)),

                    (Some(pk), _) => inner
                        .children_of(pk)
                        .any(|child| passing.contains(&child.index)),

                    (None, _) => inner.nodes().any(|m| {
                        m.is_focusable() && m.parent_key.is_none() && passing.contains(&m.index)
                    }),
                }
            })
            .map(|(wrapper_pos, n)| (wrapper_pos, n.index))
            .collect::<Vec<_>>();

        let visible_positions = visible_data.iter().map(|&(wp, _)| wp).collect::<Vec<_>>();

        let visible_base_indices = visible_data
            .iter()
            .map(|&(_, bi)| bi)
            .collect::<BTreeSet<_>>();

        let base_to_visible_pos = visible_data
            .iter()
            .enumerate()
            .map(|(vis_idx, &(_, base_idx))| (base_idx, vis_idx))
            .collect::<BTreeMap<_, _>>();

        let first_focusable = visible_positions.iter().enumerate().find_map(|(vi, &wp)| {
            inner
                .get_by_index(wp)
                .filter(|n| n.is_focusable())
                .map(|_| vi)
        });

        let last_focusable = visible_positions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(vi, &wp)| {
                inner
                    .get_by_index(wp)
                    .filter(|n| n.is_focusable())
                    .map(|_| vi)
            });

        Self {
            inner,
            visible_base_indices,
            visible_positions,
            base_to_visible_pos,
            first_focusable,
            last_focusable,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Clone, C: Collection<T>> Collection<T> for FilteredCollection<'a, T, C> {
    fn size(&self) -> usize {
        self.visible_positions.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        let node = self.inner.get(key)?;
        if self.visible_base_indices.contains(&node.index) {
            Some(node)
        } else {
            None
        }
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.visible_positions
            .get(index)
            .and_then(|&wp| self.inner.get_by_index(wp))
    }

    fn first_key(&self) -> Option<&Key> {
        self.first_focusable
            .and_then(|vi| self.visible_positions.get(vi))
            .and_then(|&wp| self.inner.get_by_index(wp))
            .map(|n| &n.key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable
            .and_then(|vi| self.visible_positions.get(vi))
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

        let vis_pos = *self.base_to_visible_pos.get(&node.index)?;

        self.visible_positions[vis_pos + 1..]
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

        let vis_pos = *self.base_to_visible_pos.get(&node.index)?;

        self.visible_positions[..vis_pos]
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
        self.visible_positions
            .iter()
            .filter_map(|&wp| self.inner.get_by_index(wp))
    }

    fn children_of<'b>(&'b self, parent_key: &Key) -> impl Iterator<Item = &'b Node<T>>
    where
        T: 'b,
    {
        self.inner
            .children_of(parent_key)
            .filter(|n| self.visible_base_indices.contains(&n.index))
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints visible count only.
impl<T, C: Collection<T>> Debug for FilteredCollection<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilteredCollection")
            .field("visible", &self.visible_positions.len())
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

    // ------------------------------------------------------------------ //
    // Composition: FilteredCollection over SortedCollection               //
    // ------------------------------------------------------------------ //

    #[test]
    fn filter_over_sorted_preserves_sorted_order() {
        use crate::SortedCollection;

        let base = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&base, |a, b| a.text_value.cmp(&b.text_value));
        // Sorted order: Apple(2), Banana(3), Cherry(1)

        // Filter out Banana — should keep Apple, Cherry in sorted order.
        let filtered = FilteredCollection::new(&sorted, |n| n.text_value != "Banana");

        assert_eq!(filtered.size(), 2);

        // Apple(2), Cherry(1) — Banana(3) was filtered out.
        let keys = filtered.keys().collect::<Vec<_>>();
        assert_eq!(keys, vec![&Key::int(2), &Key::int(1)]);

        // Positional access follows filtered sorted order.
        assert_eq!(filtered.get_by_index(0).expect("idx 0").text_value, "Apple");
        assert_eq!(
            filtered.get_by_index(1).expect("idx 1").text_value,
            "Cherry"
        );

        // Navigation follows filtered sorted order.
        assert_eq!(filtered.first_key(), Some(&Key::int(2))); // Apple
        assert_eq!(filtered.last_key(), Some(&Key::int(1))); // Cherry
        assert_eq!(filtered.key_after(&Key::int(2)), Some(&Key::int(1))); // Apple → Cherry
        assert_eq!(filtered.key_after(&Key::int(1)), Some(&Key::int(2))); // Cherry wraps → Apple
    }

    #[test]
    fn filter_all_pass_over_sorted_matches_sorted() {
        use crate::SortedCollection;

        let base = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", "c")
            .item(Key::int(2), "Apple", "a")
            .item(Key::int(3), "Banana", "b")
            .build();

        let sorted = SortedCollection::new(&base, |a, b| a.text_value.cmp(&b.text_value));
        let filtered = FilteredCollection::new(&sorted, |_| true);

        // Should produce identical order as sorted: Apple, Banana, Cherry
        let texts = filtered
            .nodes()
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["Apple", "Banana", "Cherry"]);

        assert_eq!(filtered.size(), 3);
    }

    #[test]
    fn no_op_filter_preserves_top_level_separator() {
        // A no-op predicate must not change the collection's shape, including
        // top-level separators (which have no children and no parent section).
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .separator()
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| true);

        assert_eq!(filtered.size(), 3);
        let types = filtered.nodes().map(|n| n.node_type).collect::<Vec<_>>();
        assert_eq!(
            types,
            vec![NodeType::Item, NodeType::Separator, NodeType::Item]
        );
    }

    #[test]
    fn top_level_separator_preserved_when_top_level_item_passes() {
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .separator()
            .item(Key::int(2), "Banana", "b")
            .build();

        // Filter out Banana; Apple still passes — separator stays.
        let filtered = FilteredCollection::new(&inner, |n| n.text_value == "Apple");

        assert_eq!(filtered.size(), 2);
        let types = filtered.nodes().map(|n| n.node_type).collect::<Vec<_>>();
        assert_eq!(types, vec![NodeType::Item, NodeType::Separator]);
    }

    #[test]
    fn top_level_separator_dropped_when_no_top_level_items_pass() {
        // When no top-level items pass, the separator has no scope to belong
        // to and is correctly excluded.
        let inner = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .separator()
            .item(Key::int(2), "Banana", "b")
            .build();

        let filtered = FilteredCollection::new(&inner, |_| false);

        assert_eq!(filtered.size(), 0);
    }
}
