// ars-collections/src/builder.rs

use alloc::{format, string::String, vec::Vec};

use indexmap::IndexMap;

use crate::{
    key::Key,
    node::{Node, NodeType},
    static_collection::StaticCollection,
};

/// Builds a [`StaticCollection`] from items added imperatively or from an
/// iterator.
///
/// # Example
///
/// ```rust,ignore
/// let collection = CollectionBuilder::new()
///     .item(Key::int(1), "Apple",  apple_data)
///     .item(Key::int(2), "Banana", banana_data)
///     .item(Key::int(3), "Cherry", cherry_data)
///     .build();
/// ```
pub struct CollectionBuilder<T> {
    /// Nodes accumulated so far in flat iteration order.
    nodes: Vec<Node<T>>,
    /// Maps each key to its flat index for O(1) duplicate detection.
    key_to_index: IndexMap<Key, usize>,
    /// The key of the currently open section, if any.
    current_section: Option<Key>,
}

// The main impl block requires only `T` — no `Clone` bound needed for building.
// Only `build()` requires `T: Clone` (because `StaticCollection` needs it).
impl<T> CollectionBuilder<T> {
    /// Create an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            key_to_index: IndexMap::new(),
            current_section: None,
        }
    }

    /// Add a focusable item.
    ///
    /// `text_value` is used for type-ahead matching and ARIA label fallback.
    #[must_use]
    pub fn item(mut self, key: impl Into<Key>, text_value: impl Into<String>, value: T) -> Self {
        let key = key.into();
        let index = self.nodes.len();
        let node = Node {
            key: key.clone(),
            node_type: NodeType::Item,
            value: Some(value),
            text_value: text_value.into(),
            level: if self.current_section.is_some() { 1 } else { 0 },
            has_children: false,
            is_expanded: None,
            parent_key: self.current_section.clone(),
            index,
        };
        debug_assert!(
            !self.key_to_index.contains_key(&key),
            "CollectionBuilder: duplicate key {key:?}"
        );
        self.key_to_index.insert(key, index);
        self.nodes.push(node);
        self
    }

    /// Begin a named section. All subsequent `item` calls until the next
    /// `end_section` (or another `section`) belong to this section.
    #[must_use]
    pub fn section(mut self, key: impl Into<Key>, header_text: impl Into<String>) -> Self {
        let key = key.into();
        let header_key = Key::str(format!("{key}-header"));

        // Push the Section node itself.
        let section_index = self.nodes.len();
        self.key_to_index.insert(key.clone(), section_index);
        self.nodes.push(Node {
            key: key.clone(),
            node_type: NodeType::Section,
            value: None,
            text_value: header_text.into(),
            level: 0,
            has_children: true,
            is_expanded: None,
            parent_key: None,
            index: section_index,
        });

        // Push a Header node for the visible label.
        let header_index = self.nodes.len();
        self.key_to_index.insert(header_key.clone(), header_index);
        self.nodes.push(Node {
            key: header_key,
            node_type: NodeType::Header,
            value: None,
            text_value: self.nodes[section_index].text_value.clone(),
            level: 0,
            has_children: false,
            is_expanded: None,
            parent_key: Some(key.clone()),
            index: header_index,
        });

        self.current_section = Some(key);
        self
    }

    /// End the current section. Items added after this call are top-level.
    #[must_use]
    pub fn end_section(mut self) -> Self {
        self.current_section = None;
        self
    }

    /// Add a visual separator.
    #[must_use]
    pub fn separator(mut self) -> Self {
        let index = self.nodes.len();
        let key = Key::str(format!("separator-{index}"));
        self.key_to_index.insert(key.clone(), index);
        self.nodes.push(Node {
            key,
            node_type: NodeType::Separator,
            value: None,
            text_value: String::new(),
            level: 0,
            has_children: false,
            is_expanded: None,
            parent_key: self.current_section.clone(),
            index,
        });
        self
    }
}

impl<T: Clone> CollectionBuilder<T> {
    /// Consume the builder and produce a [`StaticCollection`].
    ///
    /// This is the only method that requires `T: Clone`, because
    /// `StaticCollection<T>` has a `Clone` bound on its impl.
    #[must_use]
    pub fn build(self) -> StaticCollection<T> {
        StaticCollection::from_parts(self.nodes, self.key_to_index)
    }
}

impl<T> Default for CollectionBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints item count only.
impl<T> core::fmt::Debug for CollectionBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CollectionBuilder")
            .field("items", &self.nodes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Collection;

    #[test]
    fn builder_empty() {
        let c = CollectionBuilder::<String>::new().build();
        assert_eq!(c.size(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn builder_single_item() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "apple_data")
            .build();

        assert_eq!(c.size(), 1);
        let node = c.get(&Key::int(1)).expect("should find key 1");
        assert_eq!(node.node_type, NodeType::Item);
        assert_eq!(node.text_value, "Apple");
        assert_eq!(node.value, Some("apple_data"));
        assert_eq!(node.index, 0);
        assert_eq!(node.level, 0);
        assert_eq!(node.parent_key, None);
    }

    #[test]
    fn builder_multiple_items_ordering() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .item(Key::int(3), "Cherry", "c")
            .build();

        assert_eq!(c.size(), 3);
        let keys = c.keys().collect::<Vec<_>>();
        assert_eq!(keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);

        // Verify indices
        assert_eq!(c.get(&Key::int(1)).expect("key 1").index, 0);
        assert_eq!(c.get(&Key::int(2)).expect("key 2").index, 1);
        assert_eq!(c.get(&Key::int(3)).expect("key 3").index, 2);
    }

    #[test]
    fn builder_section_creates_section_and_header_nodes() {
        let c = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "a")
            .item(Key::int(2), "Banana", "b")
            .end_section()
            .build();

        // Section + Header + 2 items = 4 nodes
        assert_eq!(c.size(), 4);

        // Section node
        let section = c.get(&Key::str("fruits")).expect("section");
        assert_eq!(section.node_type, NodeType::Section);
        assert_eq!(section.text_value, "Fruits");
        assert!(section.has_children);

        // Header node
        let header = c.get(&Key::str("fruits-header")).expect("header");
        assert_eq!(header.node_type, NodeType::Header);
        assert_eq!(header.text_value, "Fruits");
        assert_eq!(header.parent_key, Some(Key::str("fruits")));

        // Items have level 1 and parent_key
        let apple = c.get(&Key::int(1)).expect("apple");
        assert_eq!(apple.level, 1);
        assert_eq!(apple.parent_key, Some(Key::str("fruits")));
    }

    #[test]
    fn builder_end_section_resets_level() {
        let c = CollectionBuilder::new()
            .section(Key::str("sec"), "Section")
            .item(Key::int(1), "Inside", "in")
            .end_section()
            .item(Key::int(2), "Outside", "out")
            .build();

        let inside = c.get(&Key::int(1)).expect("inside");
        assert_eq!(inside.level, 1);
        assert_eq!(inside.parent_key, Some(Key::str("sec")));

        let outside = c.get(&Key::int(2)).expect("outside");
        assert_eq!(outside.level, 0);
        assert_eq!(outside.parent_key, None);
    }

    #[test]
    fn builder_separator() {
        let c = CollectionBuilder::new()
            .item(Key::int(1), "A", "a")
            .separator()
            .item(Key::int(2), "B", "b")
            .build();

        assert_eq!(c.size(), 3);

        // The separator is at index 1, so its key is "separator-1"
        let sep = c.get(&Key::str("separator-1")).expect("separator");
        assert_eq!(sep.node_type, NodeType::Separator);
        assert!(sep.text_value.is_empty());
        assert!(!sep.is_focusable());
    }

    #[test]
    fn builder_default_equals_new() {
        let a = CollectionBuilder::<String>::new();
        let b = CollectionBuilder::<String>::default();
        assert_eq!(a.build().size(), b.build().size());
    }

    #[test]
    fn builder_debug_format() {
        let builder =
            CollectionBuilder::new()
                .item(Key::int(1), "A", "a")
                .item(Key::int(2), "B", "b");
        let debug = alloc::format!("{builder:?}");
        assert!(debug.contains("CollectionBuilder"));
        assert!(debug.contains("2"));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "CollectionBuilder: duplicate key")]
    fn builder_duplicate_key_panics_in_debug() {
        drop(
            CollectionBuilder::new()
                .item(Key::int(1), "A", "a")
                .item(Key::int(1), "B", "b")
                .build(),
        );
    }
}
