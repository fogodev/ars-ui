// ars-collections/src/node.rs

use alloc::string::String;

use crate::key::Key;

/// The structural role of a node within a collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    /// A selectable/focusable item (the common case).
    Item,

    /// A logical grouping of items. Sections are not themselves focusable.
    /// Their `child_nodes` iterator yields the items they contain.
    Section,

    /// A non-interactive heading rendered above a section's items.
    Header,

    /// A decorative or semantic divider between items or sections.
    /// Never focusable, never selectable.
    Separator,
}

/// A single node within a collection, wrapping the user's item value `T`
/// together with structural metadata used by components for rendering,
/// keyboard navigation, and ARIA attribute generation.
#[derive(Clone, Debug)]
pub struct Node<T> {
    /// Stable identity of this node.
    pub key: Key,

    /// Structural role: Item, Section, Header, or Separator.
    pub node_type: NodeType,

    /// The user's data value. `None` for structural nodes (Header, Separator)
    /// that carry no domain data.
    pub value: Option<T>,

    /// Plain-text representation used for type-ahead matching and ARIA
    /// `aria-label` fallback when no visible label is provided.
    /// Derived from the item's display text by the collection builder.
    pub text_value: String,

    /// Nesting depth. Root-level items have `level = 0`. Each additional
    /// level of nesting (tree children) increments by 1.
    pub level: usize,

    /// Whether this node has child nodes. Always `false` for items at the
    /// leaf level. Always `true` for Section nodes with content.
    /// For tree items: `true` even when the item is currently collapsed,
    /// as the children exist but are not in the flattened iteration.
    pub has_children: bool,

    /// Whether this tree item is currently expanded. `None` for non-tree
    /// collections and for items that are not themselves parents.
    pub is_expanded: Option<bool>,

    /// The key of the parent Section or tree Item, if any.
    pub parent_key: Option<Key>,

    /// Index of this node within its flat iteration order (0-based).
    /// Set by the collection after building. Used by virtualization.
    pub index: usize,
}

impl<T> Node<T> {
    /// Returns `true` if this node can receive focus during keyboard navigation.
    /// Items are focusable; Sections, Headers, and Separators are not.
    #[must_use]
    pub fn is_focusable(&self) -> bool {
        self.node_type == NodeType::Item
    }

    /// Returns `true` if this node represents a structural boundary
    /// (Section, Header, or Separator) that is never selectable.
    #[must_use]
    pub fn is_structural(&self) -> bool {
        !matches!(self.node_type, NodeType::Item)
    }

    /// Compare two nodes by structural identity only: `key`, `node_type`,
    /// and `text_value`. Does NOT compare `value`. Does NOT require
    /// `T: PartialEq`. Used by the collection diffing algorithm to detect
    /// structural changes (additions, removals, reordering) without
    /// inspecting payloads.
    #[must_use]
    pub fn structural_eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.node_type == other.node_type
            && self.text_value == other.text_value
    }
}

/// `PartialEq` compares ALL fields including `value`, so it is only
/// available when `T: PartialEq`. Two nodes are equal when they have
/// the same key, type, text, AND value.
impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.node_type == other.node_type
            && self.text_value == other.text_value
            && self.value == other.value
    }
}

/// `Eq` is safe to implement when `T: Eq` because `PartialEq` now
/// compares all fields (including `value`), satisfying the
/// substitutability requirement. This allows `Node<T>` to be used
/// in `BTreeSet`, `HashSet`, and other collections requiring `Eq`.
impl<T: Eq> Eq for Node<T> {}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    /// Helper to create a minimal item node for testing.
    fn item_node(key: Key, value: &str) -> Node<String> {
        Node {
            key,
            node_type: NodeType::Item,
            value: Some(value.to_string()),
            text_value: value.to_string(),
            level: 0,
            has_children: false,
            is_expanded: None,
            parent_key: None,
            index: 0,
        }
    }

    /// Helper to create a structural node for testing.
    fn structural_node(key: Key, node_type: NodeType) -> Node<String> {
        Node {
            key,
            node_type,
            value: None,
            text_value: String::new(),
            level: 0,
            has_children: false,
            is_expanded: None,
            parent_key: None,
            index: 0,
        }
    }

    // --- is_focusable ---

    #[test]
    fn item_is_focusable() {
        let node = item_node(Key::int(1), "apple");
        assert!(node.is_focusable());
    }

    #[test]
    fn section_not_focusable() {
        let node = structural_node(Key::int(1), NodeType::Section);
        assert!(!node.is_focusable());
    }

    #[test]
    fn header_not_focusable() {
        let node = structural_node(Key::int(1), NodeType::Header);
        assert!(!node.is_focusable());
    }

    #[test]
    fn separator_not_focusable() {
        let node = structural_node(Key::int(1), NodeType::Separator);
        assert!(!node.is_focusable());
    }

    // --- is_structural ---

    #[test]
    fn item_not_structural() {
        let node = item_node(Key::int(1), "apple");
        assert!(!node.is_structural());
    }

    #[test]
    fn section_is_structural() {
        let node = structural_node(Key::int(1), NodeType::Section);
        assert!(node.is_structural());
    }

    #[test]
    fn header_is_structural() {
        let node = structural_node(Key::int(1), NodeType::Header);
        assert!(node.is_structural());
    }

    #[test]
    fn separator_is_structural() {
        let node = structural_node(Key::int(1), NodeType::Separator);
        assert!(node.is_structural());
    }

    // --- structural_eq ---

    #[test]
    fn structural_eq_matches() {
        let a = item_node(Key::str("x"), "hello");
        let b = item_node(Key::str("x"), "hello");
        assert!(a.structural_eq(&b));
    }

    #[test]
    fn structural_eq_ignores_value() {
        let a = Node {
            value: Some("alpha".to_string()),
            ..item_node(Key::str("x"), "hello")
        };
        let b = Node {
            value: Some("beta".to_string()),
            ..item_node(Key::str("x"), "hello")
        };
        // structural_eq ignores value — same key+type+text → true
        assert!(a.structural_eq(&b));
        // but PartialEq sees different values → false
        assert_ne!(a, b);
    }

    #[test]
    fn structural_eq_differs_on_key() {
        let a = item_node(Key::str("x"), "hello");
        let b = item_node(Key::str("y"), "hello");
        assert!(!a.structural_eq(&b));
    }

    #[test]
    fn structural_eq_differs_on_node_type() {
        let a = item_node(Key::int(1), "hello");
        let mut b = item_node(Key::int(1), "hello");
        b.node_type = NodeType::Header;
        assert!(!a.structural_eq(&b));
    }

    #[test]
    fn structural_eq_differs_on_text_value() {
        let a = item_node(Key::int(1), "hello");
        let b = item_node(Key::int(1), "world");
        assert!(!a.structural_eq(&b));
    }

    // --- PartialEq ---

    #[test]
    fn partial_eq_compares_value() {
        let a = Node {
            value: Some(1u32),
            ..Node {
                key: Key::int(1),
                node_type: NodeType::Item,
                value: Some(1u32),
                text_value: "one".to_string(),
                level: 0,
                has_children: false,
                is_expanded: None,
                parent_key: None,
                index: 0,
            }
        };
        let b = Node {
            value: Some(2u32),
            ..Node {
                key: Key::int(1),
                node_type: NodeType::Item,
                value: Some(2u32),
                text_value: "one".to_string(),
                level: 0,
                has_children: false,
                is_expanded: None,
                parent_key: None,
                index: 0,
            }
        };
        // Same identity (key+type+text) but different value → not equal
        assert_ne!(a, b);
    }

    #[test]
    fn partial_eq_equal_nodes() {
        let a = item_node(Key::int(1), "apple");
        let b = item_node(Key::int(1), "apple");
        assert_eq!(a, b);
    }

    // --- Node field coverage ---

    #[test]
    fn node_with_tree_fields() {
        let node: Node<String> = Node {
            key: Key::str("child-1"),
            node_type: NodeType::Item,
            value: Some("child".to_string()),
            text_value: "child".to_string(),
            level: 2,
            has_children: true,
            is_expanded: Some(false),
            parent_key: Some(Key::str("parent")),
            index: 5,
        };
        assert_eq!(node.level, 2);
        assert!(node.has_children);
        assert_eq!(node.is_expanded, Some(false));
        assert_eq!(node.parent_key, Some(Key::str("parent")));
        assert_eq!(node.index, 5);
        assert!(node.is_focusable());
    }

    #[test]
    fn node_clone() {
        let node = item_node(Key::int(1), "apple");
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    #[test]
    fn node_type_copy() {
        let t = NodeType::Item;
        let t2 = t;
        assert_eq!(t, t2);
    }

    // --- PartialEq short-circuit paths ---

    #[test]
    fn partial_eq_differs_on_key() {
        let a = item_node(Key::int(1), "apple");
        let b = item_node(Key::int(2), "apple");
        assert_ne!(a, b);
    }

    #[test]
    fn partial_eq_differs_on_node_type() {
        let a = item_node(Key::int(1), "apple");
        let mut b = item_node(Key::int(1), "apple");
        b.node_type = NodeType::Section;
        assert_ne!(a, b);
    }

    #[test]
    fn partial_eq_differs_on_text_value() {
        let a = item_node(Key::int(1), "apple");
        let b = item_node(Key::int(1), "banana");
        assert_ne!(a, b);
    }

    // --- structural_eq with None values ---

    #[test]
    fn structural_eq_with_none_values() {
        let a = structural_node(Key::str("sep-1"), NodeType::Separator);
        let b = structural_node(Key::str("sep-1"), NodeType::Separator);
        // Both have value: None — structural_eq ignores value, should match
        assert!(a.structural_eq(&b));
        // PartialEq also matches since None == None
        assert_eq!(a, b);
    }

    #[test]
    fn structural_eq_none_vs_some() {
        let a = structural_node(Key::int(1), NodeType::Item);
        let b = item_node(Key::int(1), "");
        // Same key, same node_type — but different text_value ("" vs "")
        // a.text_value is "" (from structural_node), b.text_value is "" (value="")
        // Actually both are "" — so structural_eq matches
        assert!(a.structural_eq(&b));
        // But PartialEq sees None vs Some("") → not equal
        assert_ne!(a, b);
    }
}
