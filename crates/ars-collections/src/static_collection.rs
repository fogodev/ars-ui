// ars-collections/src/static_collection.rs

use alloc::{string::String, vec::Vec};

use indexmap::IndexMap;

use crate::{
    builder::CollectionBuilder,
    collection::{Collection, CollectionItem},
    key::Key,
    node::Node,
};

/// A collection built once from in-memory data. Traversal is O(1) for key
/// lookup (via `IndexMap`) and O(1) for index lookup (via `Vec` backing).
pub struct StaticCollection<T> {
    /// Nodes in flat iteration order.
    nodes: Vec<Node<T>>,

    /// Maps Key → flat index for O(1) `get`.
    key_to_index: IndexMap<Key, usize>,

    /// Cached index of the first focusable item.
    first_focusable: Option<usize>,

    /// Cached index of the last focusable item.
    last_focusable: Option<usize>,
}

impl<T: Clone> StaticCollection<T> {
    /// Construct from pre-built parts (used by `CollectionBuilder`).
    pub(crate) fn from_parts(nodes: Vec<Node<T>>, key_to_index: IndexMap<Key, usize>) -> Self {
        let first_focusable = nodes.iter().position(Node::is_focusable);
        let last_focusable = nodes.iter().rposition(Node::is_focusable);
        Self {
            nodes,
            key_to_index,
            first_focusable,
            last_focusable,
        }
    }

    /// Construct from a `Vec` of `(Key, text_value, T)` tuples.
    #[must_use]
    pub fn new(items: Vec<(Key, String, T)>) -> Self {
        Self::from_vec(items)
    }

    /// Convenience constructor from a `Vec` of `(Key, label, data)` tuples.
    #[must_use]
    pub fn from_vec(items: Vec<(Key, String, T)>) -> Self {
        let mut builder = CollectionBuilder::new();
        for (key, text, value) in items {
            builder = builder.item(key, text, value);
        }
        builder.build()
    }
}

/// Enables `iterator.collect::<StaticCollection<T>>()` as the idiomatic way
/// to build a collection from an iterator of `(Key, text_value, T)` tuples.
impl<T: Clone> FromIterator<(Key, String, T)> for StaticCollection<T> {
    fn from_iter<I: IntoIterator<Item = (Key, String, T)>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

// Note: Clone bound on the trait impl is intentional — StaticCollection stores owned T values
// and its constructors (from_vec, from_parts) require Clone for ergonomic initialization.
// The Collection<T> trait itself has no bounds on T (read-only access), but this impl
// inherits the Clone bound from the struct's constructors for simplicity.
impl<T: Clone> Collection<T> for StaticCollection<T> {
    fn size(&self) -> usize {
        self.nodes.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.key_to_index.get(key).map(|&i| &self.nodes[i])
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.nodes.get(index)
    }

    fn first_key(&self) -> Option<&Key> {
        self.first_focusable.map(|i| &self.nodes[i].key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable.map(|i| &self.nodes[i].key)
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.key_after_no_wrap(key).or_else(|| self.first_key())
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.key_before_no_wrap(key).or_else(|| self.last_key())
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        let start = *self.key_to_index.get(key)? + 1;
        self.nodes[start..]
            .iter()
            .find(|n| n.is_focusable())
            .map(|n| &n.key)
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let end = *self.key_to_index.get(key)?;
        self.nodes[..end]
            .iter()
            .rfind(|n| n.is_focusable())
            .map(|n| &n.key)
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.nodes.iter().map(|n| &n.key)
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.nodes.iter()
    }

    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.nodes
            .iter()
            .filter(move |n| n.parent_key.as_ref() == Some(parent_key))
    }
}

/// Manual `Clone` — all fields are `Clone` when `T: Clone`.
impl<T: Clone> Clone for StaticCollection<T> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            key_to_index: self.key_to_index.clone(),
            first_focusable: self.first_focusable,
            last_focusable: self.last_focusable,
        }
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints size only, since the
/// payload `T` is opaque to the machine layer.
impl<T> core::fmt::Debug for StaticCollection<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StaticCollection")
            .field("size", &self.nodes.len())
            .finish()
    }
}

/// Structural equality: two collections are equal when they contain the same
/// nodes in the same order (compared via `Node<T>::PartialEq`, which checks
/// all fields including `key`, `node_type`, `text_value`, and `value: T`).
impl<T: Clone + PartialEq> PartialEq for StaticCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        self.nodes.len() == other.nodes.len()
            && self
                .nodes
                .iter()
                .zip(other.nodes.iter())
                .all(|(a, b)| a == b)
    }
}

/// Mutation methods used by `MutableListData`. These operate on the internal
/// `IndexMap` and `Vec` and are O(1) amortized for append, O(n) for mid-list insert.
impl<T: CollectionItem> StaticCollection<T> {
    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` when the collection contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Insert an item at the given index, shifting subsequent items.
    pub fn insert(&mut self, index: usize, item: T) {
        let key = item.key().clone();
        let node = Node::item(key.clone(), index, item);
        self.nodes.insert(index, node);
        self.key_to_index.insert(key, index);
        self.reindex_from(index + 1);
    }

    /// Remove items by key. Returns the removed item values.
    pub fn remove_by_keys(&mut self, keys: &[Key]) -> Vec<T> {
        let mut removed = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(idx) = self.key_to_index.shift_remove(key) {
                let node = self.nodes.remove(idx);
                if let Some(val) = node.value {
                    removed.push(val);
                }
                self.reindex_from(idx);
            }
        }
        removed
    }

    /// Replace an item's data (matched by key).
    pub fn replace(&mut self, item: T) {
        let key = item.key().clone();
        if let Some(&idx) = self.key_to_index.get(&key) {
            self.nodes[idx].value = Some(item);
        }
    }

    /// Remove all items.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.key_to_index.clear();
    }

    /// Move an item from one index to another.
    pub fn move_item(&mut self, from: usize, to: usize) {
        let node = self.nodes.remove(from);
        self.nodes.insert(to, node);
        let start = from.min(to);
        self.reindex_from(start);
    }

    /// Get the index of an item by key.
    #[must_use]
    pub fn index_of(&self, key: &Key) -> Option<usize> {
        self.key_to_index.get(key).copied()
    }

    /// Recompute `key_to_index` and `Node::index` from a starting position.
    fn reindex_from(&mut self, start: usize) {
        for i in start..self.nodes.len() {
            self.nodes[i].index = i;
            let key = &self.nodes[i].key;
            self.key_to_index.insert(key.clone(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString, vec};

    use super::*;
    use crate::node::NodeType;

    // ---------------------------------------------------------------
    // Test helper: a simple CollectionItem impl for mutation tests
    // ---------------------------------------------------------------

    #[derive(Clone, Debug, PartialEq)]
    struct Fruit {
        id: Key,
        name: String,
    }

    impl Fruit {
        fn new(id: u64, name: &str) -> Self {
            Self {
                id: Key::int(id),
                name: name.to_string(),
            }
        }
    }

    impl CollectionItem for Fruit {
        fn key(&self) -> &Key {
            &self.id
        }

        fn text_value(&self) -> &str {
            &self.name
        }
    }

    // ---------------------------------------------------------------
    // Helper to build a simple 3-item collection
    // ---------------------------------------------------------------

    fn three_items() -> StaticCollection<&'static str> {
        StaticCollection::from_vec(vec![
            (Key::int(1), "Apple".to_string(), "a"),
            (Key::int(2), "Banana".to_string(), "b"),
            (Key::int(3), "Cherry".to_string(), "c"),
        ])
    }

    // ---------------------------------------------------------------
    // Construction tests
    // ---------------------------------------------------------------

    #[test]
    fn from_vec_basic() {
        let c = three_items();
        assert_eq!(c.size(), 3);
        assert!(!c.is_empty());
    }

    #[test]
    fn from_vec_empty() {
        let c = StaticCollection::<String>::from_vec(vec![]);
        assert_eq!(c.size(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn new_delegates_to_from_vec() {
        let a = StaticCollection::new(vec![
            (Key::int(1), "X".to_string(), 10),
            (Key::int(2), "Y".to_string(), 20),
        ]);
        let b = StaticCollection::from_vec(vec![
            (Key::int(1), "X".to_string(), 10),
            (Key::int(2), "Y".to_string(), 20),
        ]);
        assert_eq!(a, b);
    }

    #[test]
    fn from_iterator() {
        let items = vec![
            (Key::int(1), "A".to_string(), "a"),
            (Key::int(2), "B".to_string(), "b"),
        ];
        let c: StaticCollection<&str> = items.into_iter().collect();
        assert_eq!(c.size(), 2);
    }

    // ---------------------------------------------------------------
    // Collection trait — size
    // ---------------------------------------------------------------

    #[test]
    fn size_includes_structural_nodes() {
        let c = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "A", "a")
            .end_section()
            .separator()
            .item(Key::int(2), "B", "b")
            .build();
        // Section + Header + item + separator + item = 5
        assert_eq!(c.size(), 5);
    }

    // ---------------------------------------------------------------
    // Collection trait — random access
    // ---------------------------------------------------------------

    #[test]
    fn get_existing() {
        let c = three_items();
        let node = c.get(&Key::int(2)).expect("key 2 should exist");
        assert_eq!(node.text_value, "Banana");
        assert_eq!(node.value, Some("b"));
    }

    #[test]
    fn get_missing() {
        let c = three_items();
        assert!(c.get(&Key::int(99)).is_none());
    }

    #[test]
    fn contains_key_true() {
        let c = three_items();
        assert!(c.contains_key(&Key::int(1)));
    }

    #[test]
    fn contains_key_false() {
        let c = three_items();
        assert!(!c.contains_key(&Key::int(99)));
    }

    #[test]
    fn get_by_index_valid() {
        let c = three_items();
        let node = c.get_by_index(1).expect("index 1");
        assert_eq!(node.key, Key::int(2));
    }

    #[test]
    fn get_by_index_out_of_range() {
        let c = three_items();
        assert!(c.get_by_index(100).is_none());
    }

    // ---------------------------------------------------------------
    // Collection trait — boundary navigation
    // ---------------------------------------------------------------

    #[test]
    fn first_key_with_items() {
        let c = three_items();
        assert_eq!(c.first_key(), Some(&Key::int(1)));
    }

    #[test]
    fn first_key_empty() {
        let c = StaticCollection::<String>::from_vec(vec![]);
        assert_eq!(c.first_key(), None);
    }

    #[test]
    fn first_key_skips_structural() {
        let c = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "Apple", "a")
            .end_section()
            .build();
        // Section and Header are structural; first focusable is the item
        assert_eq!(c.first_key(), Some(&Key::int(1)));
    }

    #[test]
    fn last_key_with_items() {
        let c = three_items();
        assert_eq!(c.last_key(), Some(&Key::int(3)));
    }

    #[test]
    fn last_key_skips_structural() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "A", "a")
            .separator()
            .build();
        // Separator is structural; last focusable is item 1
        assert_eq!(c.last_key(), Some(&Key::int(1)));
    }

    #[test]
    fn first_and_last_key_only_structural() {
        let c: StaticCollection<String> = CollectionBuilder::new().separator().build();
        assert_eq!(c.first_key(), None);
        assert_eq!(c.last_key(), None);
    }

    // ---------------------------------------------------------------
    // Collection trait — wrapping navigation
    // ---------------------------------------------------------------

    #[test]
    fn key_after_middle() {
        let c = three_items();
        assert_eq!(c.key_after(&Key::int(1)), Some(&Key::int(2)));
    }

    #[test]
    fn key_after_wraps_at_end() {
        let c = three_items();
        // key_after the last item wraps to first
        assert_eq!(c.key_after(&Key::int(3)), Some(&Key::int(1)));
    }

    #[test]
    fn key_before_middle() {
        let c = three_items();
        assert_eq!(c.key_before(&Key::int(3)), Some(&Key::int(2)));
    }

    #[test]
    fn key_before_wraps_at_start() {
        let c = three_items();
        // key_before the first item wraps to last
        assert_eq!(c.key_before(&Key::int(1)), Some(&Key::int(3)));
    }

    #[test]
    fn key_after_unknown_key() {
        let c = three_items();
        // Unknown key → key_after_no_wrap returns None → or_else(first_key)
        // But wait — key_to_index.get returns None, so key_after_no_wrap returns None,
        // then or_else calls first_key which returns Some(1).
        // Actually, for an unknown key, the wrapping version still returns first_key.
        // This is spec-correct: key_after returns None only when no focusable items.
        assert_eq!(c.key_after(&Key::int(99)), Some(&Key::int(1)));
    }

    #[test]
    fn key_before_unknown_key() {
        let c = three_items();
        assert_eq!(c.key_before(&Key::int(99)), Some(&Key::int(3)));
    }

    // ---------------------------------------------------------------
    // Collection trait — non-wrapping navigation
    // ---------------------------------------------------------------

    #[test]
    fn key_after_no_wrap_middle() {
        let c = three_items();
        assert_eq!(c.key_after_no_wrap(&Key::int(1)), Some(&Key::int(2)));
    }

    #[test]
    fn key_after_no_wrap_at_end() {
        let c = three_items();
        assert_eq!(c.key_after_no_wrap(&Key::int(3)), None);
    }

    #[test]
    fn key_before_no_wrap_middle() {
        let c = three_items();
        assert_eq!(c.key_before_no_wrap(&Key::int(3)), Some(&Key::int(2)));
    }

    #[test]
    fn key_before_no_wrap_at_start() {
        let c = three_items();
        assert_eq!(c.key_before_no_wrap(&Key::int(1)), None);
    }

    #[test]
    fn key_after_no_wrap_unknown_key() {
        let c = three_items();
        assert_eq!(c.key_after_no_wrap(&Key::int(99)), None);
    }

    #[test]
    fn key_before_no_wrap_unknown_key() {
        let c = three_items();
        assert_eq!(c.key_before_no_wrap(&Key::int(99)), None);
    }

    // ---------------------------------------------------------------
    // Navigation skips structural nodes
    // ---------------------------------------------------------------

    #[test]
    fn key_after_skips_structural() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "A", "a")
            .separator()
            .item(Key::int(2), "B", "b")
            .build();
        // key_after(1) should skip separator and land on 2
        assert_eq!(c.key_after(&Key::int(1)), Some(&Key::int(2)));
    }

    #[test]
    fn key_before_skips_structural() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "A", "a")
            .separator()
            .item(Key::int(2), "B", "b")
            .build();
        // key_before(2) should skip separator and land on 1
        assert_eq!(c.key_before(&Key::int(2)), Some(&Key::int(1)));
    }

    #[test]
    fn navigation_with_sections() {
        let c = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .section(Key::str("vegs"), "Vegetables")
            .item(Key::int(3), "Carrot", "c")
            .end_section()
            .build();

        // Navigation skips Section and Header nodes
        assert_eq!(c.first_key(), Some(&Key::int(1)));
        assert_eq!(c.key_after(&Key::int(2)), Some(&Key::int(3)));
        assert_eq!(c.key_before(&Key::int(3)), Some(&Key::int(2)));
    }

    // ---------------------------------------------------------------
    // Collection trait — iteration
    // ---------------------------------------------------------------

    #[test]
    fn keys_iterator() {
        let c = three_items();
        let keys: Vec<_> = c.keys().collect();
        assert_eq!(keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);
    }

    #[test]
    fn nodes_iterator() {
        let c = three_items();
        let text_values: Vec<_> = c.nodes().map(|n| n.text_value.as_str()).collect();
        assert_eq!(text_values, vec!["Apple", "Banana", "Cherry"]);
    }

    #[test]
    fn item_keys_filters_structural() {
        let c = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "A", "a")
            .end_section()
            .separator()
            .item(Key::int(2), "B", "b")
            .build();
        // item_keys should only return focusable items
        let item_keys: Vec<_> = c.item_keys().collect();
        assert_eq!(item_keys, vec![&Key::int(1), &Key::int(2)]);
    }

    // ---------------------------------------------------------------
    // Collection trait — children
    // ---------------------------------------------------------------

    #[test]
    fn children_of_section() {
        let c = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .item(Key::int(3), "Other", "o")
            .build();

        let children: Vec<_> = c.children_of(&Key::str("fruits")).collect();
        // Header + 2 items are children of the section
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].node_type, NodeType::Header);
        assert_eq!(children[1].key, Key::int(1));
        assert_eq!(children[2].key, Key::int(2));
    }

    #[test]
    fn children_of_no_match() {
        let c = three_items();
        let children: Vec<_> = c.children_of(&Key::str("nonexistent")).collect();
        assert!(children.is_empty());
    }

    // ---------------------------------------------------------------
    // Collection trait — text value
    // ---------------------------------------------------------------

    #[test]
    fn text_value_of_existing() {
        let c = three_items();
        assert_eq!(c.text_value_of(&Key::int(2)), Some("Banana"));
    }

    #[test]
    fn text_value_of_missing() {
        let c = three_items();
        assert_eq!(c.text_value_of(&Key::int(99)), None);
    }

    // ---------------------------------------------------------------
    // Manual trait impls
    // ---------------------------------------------------------------

    #[test]
    fn clone_produces_equal_collection() {
        let c = three_items();
        let cloned = c.clone();
        assert_eq!(c, cloned);
    }

    #[test]
    fn debug_contains_size() {
        let c = three_items();
        let debug = format!("{c:?}");
        assert!(debug.contains("StaticCollection"));
        assert!(debug.contains("3"));
    }

    #[test]
    fn partial_eq_equal() {
        let a = three_items();
        let b = three_items();
        assert_eq!(a, b);
    }

    #[test]
    fn partial_eq_different_size() {
        let a = three_items();
        let b = StaticCollection::from_vec(vec![(Key::int(1), "Apple".to_string(), "a")]);
        assert_ne!(a, b);
    }

    #[test]
    fn partial_eq_different_values() {
        let a = StaticCollection::from_vec(vec![(Key::int(1), "Apple".to_string(), "a")]);
        let b = StaticCollection::from_vec(vec![(Key::int(1), "Apple".to_string(), "z")]);
        assert_ne!(a, b);
    }

    // ---------------------------------------------------------------
    // Mutation tests (require T: CollectionItem)
    // ---------------------------------------------------------------

    fn fruit_collection() -> StaticCollection<Fruit> {
        let mut c = CollectionBuilder::new().build();
        c.insert(0, Fruit::new(1, "Apple"));
        c.insert(1, Fruit::new(2, "Banana"));
        c.insert(2, Fruit::new(3, "Cherry"));
        c
    }

    #[test]
    fn mutation_len() {
        let c = fruit_collection();
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn mutation_insert_at_beginning() {
        let mut c = fruit_collection();
        c.insert(0, Fruit::new(0, "Avocado"));
        assert_eq!(c.len(), 4);
        assert_eq!(c.get_by_index(0).expect("index 0").key, Key::int(0));
        // Original items shifted
        assert_eq!(c.get_by_index(1).expect("index 1").key, Key::int(1));
        // Indices recomputed
        assert_eq!(c.index_of(&Key::int(0)), Some(0));
        assert_eq!(c.index_of(&Key::int(1)), Some(1));
    }

    #[test]
    fn mutation_insert_at_end() {
        let mut c = fruit_collection();
        c.insert(3, Fruit::new(4, "Dragonfruit"));
        assert_eq!(c.len(), 4);
        assert_eq!(c.get_by_index(3).expect("index 3").key, Key::int(4));
    }

    #[test]
    fn mutation_remove_by_keys() {
        let mut c = fruit_collection();
        let removed = c.remove_by_keys(&[Key::int(2)]);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].name, "Banana");
        assert_eq!(c.len(), 2);
        assert!(!c.contains_key(&Key::int(2)));
    }

    #[test]
    fn mutation_remove_missing_key() {
        let mut c = fruit_collection();
        let removed = c.remove_by_keys(&[Key::int(99)]);
        assert!(removed.is_empty());
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn mutation_replace_existing() {
        let mut c = fruit_collection();
        c.replace(Fruit::new(2, "Blueberry"));
        let node = c.get(&Key::int(2)).expect("key 2");
        assert_eq!(node.value.as_ref().expect("value").name, "Blueberry");
    }

    #[test]
    fn mutation_replace_missing() {
        let mut c = fruit_collection();
        c.replace(Fruit::new(99, "Unknown"));
        // No change — key 99 doesn't exist
        assert_eq!(c.len(), 3);
        assert!(!c.contains_key(&Key::int(99)));
    }

    #[test]
    fn mutation_clear() {
        let mut c = fruit_collection();
        c.clear();
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn mutation_move_item() {
        let mut c = fruit_collection();
        // Move Cherry (index 2) to index 0
        c.move_item(2, 0);
        assert_eq!(c.get_by_index(0).expect("index 0").key, Key::int(3));
        assert_eq!(c.get_by_index(1).expect("index 1").key, Key::int(1));
        assert_eq!(c.get_by_index(2).expect("index 2").key, Key::int(2));
        // Indices updated
        assert_eq!(c.index_of(&Key::int(3)), Some(0));
        assert_eq!(c.index_of(&Key::int(1)), Some(1));
        assert_eq!(c.index_of(&Key::int(2)), Some(2));
    }

    #[test]
    fn mutation_index_of() {
        let c = fruit_collection();
        assert_eq!(c.index_of(&Key::int(1)), Some(0));
        assert_eq!(c.index_of(&Key::int(2)), Some(1));
        assert_eq!(c.index_of(&Key::int(3)), Some(2));
        assert_eq!(c.index_of(&Key::int(99)), None);
    }

    // ---------------------------------------------------------------
    // Navigation on single-item collection (edge case)
    // ---------------------------------------------------------------

    #[test]
    fn single_item_wrapping_navigation() {
        let c = StaticCollection::from_vec(vec![(Key::int(1), "Only".to_string(), "x")]);
        // key_after wraps to itself
        assert_eq!(c.key_after(&Key::int(1)), Some(&Key::int(1)));
        // key_before wraps to itself
        assert_eq!(c.key_before(&Key::int(1)), Some(&Key::int(1)));
        // no_wrap returns None (no other focusable item)
        assert_eq!(c.key_after_no_wrap(&Key::int(1)), None);
        assert_eq!(c.key_before_no_wrap(&Key::int(1)), None);
    }

    // ---------------------------------------------------------------
    // Empty collection navigation
    // ---------------------------------------------------------------

    #[test]
    fn empty_collection_navigation() {
        let c = StaticCollection::<String>::from_vec(vec![]);
        assert_eq!(c.first_key(), None);
        assert_eq!(c.last_key(), None);
        assert_eq!(c.key_after(&Key::int(1)), None);
        assert_eq!(c.key_before(&Key::int(1)), None);
        assert_eq!(c.key_after_no_wrap(&Key::int(1)), None);
        assert_eq!(c.key_before_no_wrap(&Key::int(1)), None);
    }
}
