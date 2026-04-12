// ars-collections/src/tree_collection.rs

use alloc::{collections::BTreeSet, string::String, vec::Vec};
use core::fmt::{self, Debug};

use indexmap::IndexMap;

use crate::{
    collection::{Collection, CollectionItem},
    key::Key,
    node::{Node, NodeType},
};

/// Configuration for a single tree item during construction.
pub struct TreeItemConfig<T> {
    /// Stable identity of the tree item.
    pub key: Key,
    /// Plain-text representation for type-ahead and ARIA fallback.
    pub text_value: String,
    /// The user's data value.
    pub value: T,
    /// Child items nested under this item.
    pub children: Vec<TreeItemConfig<T>>,
    /// Whether the item starts expanded. Default `false`.
    pub default_expanded: bool,
}

/// Manual `Debug` avoids requiring `T: Debug`.
impl<T> Debug for TreeItemConfig<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeItemConfig")
            .field("key", &self.key)
            .field("text_value", &self.text_value)
            .field("children", &self.children.len())
            .field("default_expanded", &self.default_expanded)
            .finish()
    }
}

/// A collection for hierarchical data.
///
/// Maintains a flat `Vec<Node<T>>` in DFS pre-order. Collapsed subtrees are
/// present in the `nodes` vec (for key lookups and selection persistence) but
/// are excluded from the *visible* iteration exposed to components via
/// `nodes()` and `keys()`.
///
/// Call `set_expanded` to expand/collapse a subtree. This produces a new
/// `TreeCollection` (functional update) rather than mutating in place, so
/// it integrates cleanly with reactive signals in `ars-leptos`/`ars-dioxus`.
///
/// # Stack safety
///
/// Tree depth is capped at [`MAX_TREE_DEPTH`] (32 levels). This limit is
/// intentionally conservative to stay within the default WASM stack size
/// (typically 64 KiB–1 MiB) where each recursive `insert_item` frame
/// consumes stack space. The constant is checked at insertion time and
/// panics immediately if violated, rather than risking a silent stack
/// overflow at runtime.
pub struct TreeCollection<T> {
    /// All nodes, including those inside collapsed subtrees.
    all_nodes: Vec<Node<T>>,

    /// Subset of flat indices that are currently visible (not inside a
    /// collapsed ancestor).
    visible_indices: Vec<usize>,

    /// Map from Key to flat index in `all_nodes`.
    key_to_index: IndexMap<Key, usize>,

    /// Set of keys whose subtrees are currently expanded.
    expanded_keys: BTreeSet<Key>,

    /// Cached `all_nodes` index of the first visible focusable item.
    first_focusable_visible: Option<usize>,

    /// Cached `all_nodes` index of the last visible focusable item.
    last_focusable_visible: Option<usize>,
}

impl<T: Clone> Clone for TreeCollection<T> {
    fn clone(&self) -> Self {
        Self {
            all_nodes: self.all_nodes.clone(),
            visible_indices: self.visible_indices.clone(),
            key_to_index: self.key_to_index.clone(),
            expanded_keys: self.expanded_keys.clone(),
            first_focusable_visible: self.first_focusable_visible,
            last_focusable_visible: self.last_focusable_visible,
        }
    }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints node counts only,
/// since the payload `T` is opaque to the machine layer.
impl<T> Debug for TreeCollection<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeCollection")
            .field("total_nodes", &self.all_nodes.len())
            .field("visible_nodes", &self.visible_indices.len())
            .finish()
    }
}

/// Structural equality: two tree collections are equal when they contain the
/// same nodes in the same order with the same hierarchy and expansion state.
/// Extends `Node::PartialEq` (which compares key, type, text, value) with
/// hierarchy fields (`level`, `parent_key`) that are significant for trees.
impl<T: Clone + PartialEq> PartialEq for TreeCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        self.all_nodes.len() == other.all_nodes.len()
            && self.expanded_keys == other.expanded_keys
            && self
                .all_nodes
                .iter()
                .zip(other.all_nodes.iter())
                .all(|(a, b)| a == b && a.level == b.level && a.parent_key == b.parent_key)
    }
}

/// Maximum nesting depth for tree items. Enforced during construction to
/// guarantee WASM stack safety — the default WASM stack (64 KiB–1 MiB)
/// cannot support unbounded recursion.
const MAX_TREE_DEPTH: usize = 32;

impl<T> TreeCollection<T> {
    /// Compute which flat indices are visible given the current expansion set,
    /// along with cached first/last focusable visible indices.
    fn compute_visible(all_nodes: &[Node<T>]) -> (Vec<usize>, Option<usize>, Option<usize>) {
        let mut visible = Vec::with_capacity(all_nodes.len());
        let mut first_focusable = None;
        let mut last_focusable = None;
        let mut skip_until_level = None::<usize>;

        for node in all_nodes {
            // If we're skipping a collapsed subtree, check whether we've
            // exited it (returned to the same or higher level).
            if let Some(skip_level) = skip_until_level {
                if node.level <= skip_level {
                    skip_until_level = None;
                } else {
                    continue; // still inside a collapsed subtree
                }
            }

            visible.push(node.index);

            if node.is_focusable() {
                if first_focusable.is_none() {
                    first_focusable = Some(node.index);
                }
                last_focusable = Some(node.index);
            }

            // If this node has children and is not expanded, skip children.
            if node.has_children && node.is_expanded != Some(true) {
                skip_until_level = Some(node.level);
            }
        }
        (visible, first_focusable, last_focusable)
    }
}

impl<T: Clone> Default for TreeCollection<T> {
    fn default() -> Self {
        Self::new([])
    }
}

impl<T: Clone> TreeCollection<T> {
    /// Build a `TreeCollection` from a list of root-level items.
    pub fn new(roots: impl IntoIterator<Item = TreeItemConfig<T>>) -> Self {
        let mut all_nodes = Vec::new();
        let mut key_to_index = IndexMap::new();
        let mut expanded_keys = BTreeSet::new();

        // Recursive DFS insertion.
        fn insert_item<T: Clone>(
            item: TreeItemConfig<T>,
            level: usize,
            parent_key: Option<Key>,
            all_nodes: &mut Vec<Node<T>>,
            key_to_index: &mut IndexMap<Key, usize>,
            expanded_keys: &mut BTreeSet<Key>,
        ) {
            assert!(
                level <= MAX_TREE_DEPTH,
                "TreeCollection: nesting depth {level} exceeds MAX_TREE_DEPTH ({MAX_TREE_DEPTH}). \
                 Deep nesting risks stack overflow in WASM targets.",
            );
            let has_children = !item.children.is_empty();
            let index = all_nodes.len();
            if item.default_expanded && has_children {
                expanded_keys.insert(item.key.clone());
            }
            key_to_index.insert(item.key.clone(), index);
            all_nodes.push(Node {
                key: item.key.clone(),
                node_type: NodeType::Item,
                value: Some(item.value),
                text_value: item.text_value,
                level,
                has_children,
                is_expanded: if has_children {
                    Some(item.default_expanded)
                } else {
                    None
                },
                parent_key,
                index,
            });
            for child in item.children {
                insert_item(
                    child,
                    level + 1,
                    Some(item.key.clone()),
                    all_nodes,
                    key_to_index,
                    expanded_keys,
                );
            }
        }

        for root in roots {
            insert_item(
                root,
                0,
                None,
                &mut all_nodes,
                &mut key_to_index,
                &mut expanded_keys,
            );
        }

        let (visible_indices, first_focusable_visible, last_focusable_visible) =
            Self::compute_visible(&all_nodes);
        Self {
            all_nodes,
            visible_indices,
            key_to_index,
            expanded_keys,
            first_focusable_visible,
            last_focusable_visible,
        }
    }

    /// Expand or collapse the subtree rooted at `key`.
    /// Returns a new `TreeCollection` with updated visibility.
    #[must_use]
    pub fn set_expanded(&self, key: &Key, expanded: bool) -> Self {
        // Only modify expansion state for nodes that have children.
        // Leaf nodes use is_expanded == None and must not be altered.
        let is_expandable = self
            .key_to_index
            .get(key)
            .is_some_and(|&i| self.all_nodes[i].has_children);

        let mut new_expanded = self.expanded_keys.clone();
        if is_expandable {
            if expanded {
                new_expanded.insert(key.clone());
            } else {
                new_expanded.remove(key);
            }
        }

        // Update the is_expanded field on the node.
        let mut new_nodes = self.all_nodes.clone();
        if is_expandable {
            if let Some(&idx) = self.key_to_index.get(key) {
                new_nodes[idx].is_expanded = Some(expanded);
            }
        }

        let (visible_indices, first_focusable_visible, last_focusable_visible) =
            Self::compute_visible(&new_nodes);
        Self {
            all_nodes: new_nodes,
            visible_indices,
            key_to_index: self.key_to_index.clone(),
            expanded_keys: new_expanded,
            first_focusable_visible,
            last_focusable_visible,
        }
    }

    /// Whether the item with `key` is currently expanded.
    pub fn is_expanded(&self, key: &Key) -> bool {
        self.expanded_keys.contains(key)
    }

    /// Return visible keys using an external expanded-keys set.
    /// This avoids cloning the entire tree when only the expanded set changes.
    pub fn visible_keys_with_expanded(&self, expanded: &BTreeSet<Key>) -> Vec<Key> {
        let mut visible = Vec::new();
        let mut skip_until_level = None::<usize>;

        for node in &self.all_nodes {
            // If we're skipping children of a collapsed node, check if we've
            // returned to the same or shallower level.
            if let Some(level) = skip_until_level {
                if node.level > level {
                    continue;
                }
                skip_until_level = None;
            }

            visible.push(node.key.clone());

            // If this node has children and is NOT in the expanded set, skip its children.
            if node.has_children && !expanded.contains(&node.key) {
                skip_until_level = Some(node.level);
            }
        }
        visible
    }

    /// Check if a single key is visible given an external expanded-keys set.
    pub fn is_visible_with_expanded(&self, key: &Key, expanded: &BTreeSet<Key>) -> bool {
        // Find the node for this key
        let Some(&node_index) = self.key_to_index.get(key) else {
            return false;
        };
        let node = &self.all_nodes[node_index];

        // Root-level nodes are always visible.
        if node.level == 0 {
            return true;
        }

        // Walk backwards to find ancestors and check each is in the expanded set.
        let mut current_level = node.level;
        for i in (0..node_index).rev() {
            let ancestor = &self.all_nodes[i];
            if ancestor.level < current_level {
                // This is a direct ancestor at a shallower level.
                if !expanded.contains(&ancestor.key) {
                    return false;
                }
                current_level = ancestor.level;
                if current_level == 0 {
                    break;
                }
            }
        }
        true
    }
}

impl<T: Clone> Collection<T> for TreeCollection<T> {
    fn size(&self) -> usize {
        self.visible_indices.len()
    }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        self.key_to_index.get(key).map(|&i| &self.all_nodes[i])
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.visible_indices.get(index).map(|&i| &self.all_nodes[i])
    }

    fn first_key(&self) -> Option<&Key> {
        self.first_focusable_visible.map(|i| &self.all_nodes[i].key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable_visible.map(|i| &self.all_nodes[i].key)
    }

    fn key_after(&self, key: &Key) -> Option<&Key> {
        self.key_after_no_wrap(key).or_else(|| self.first_key())
    }

    fn key_before(&self, key: &Key) -> Option<&Key> {
        self.key_before_no_wrap(key).or_else(|| self.last_key())
    }

    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
        let current = *self.key_to_index.get(key)?;
        // Find position of `current` in visible_indices, then search forward.
        let pos = self.visible_indices.iter().position(|&i| i == current)?;
        self.visible_indices[pos + 1..]
            .iter()
            .find(|&&i| self.all_nodes[i].is_focusable())
            .map(|&i| &self.all_nodes[i].key)
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let current = *self.key_to_index.get(key)?;
        let pos = self.visible_indices.iter().position(|&i| i == current)?;
        self.visible_indices[..pos]
            .iter()
            .rfind(|&&i| self.all_nodes[i].is_focusable())
            .map(|&i| &self.all_nodes[i].key)
    }

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.visible_indices.iter().map(|&i| &self.all_nodes[i].key)
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        self.visible_indices.iter().map(|&i| &self.all_nodes[i])
    }

    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a,
    {
        // Returns direct children from all_nodes (not just visible ones).
        self.all_nodes
            .iter()
            .filter(move |n| n.parent_key.as_ref() == Some(parent_key))
    }
}

/// Mutation methods used by `MutableTreeData`. These operate on the internal
/// flat `all_nodes` vec, `key_to_index` map, and `visible_indices` cache.
///
/// After structural mutations (insert, remove, reparent), `rebuild_indices()`
/// recomputes `key_to_index`, per-node `index` fields, and `visible_indices`.
impl<T: CollectionItem> TreeCollection<T> {
    /// Insert a child node under `parent` at the given sibling index.
    ///
    /// `sibling_index` is the position among the parent's direct children
    /// (or among root nodes when `parent` is `None`). The node is inserted
    /// into `all_nodes` at the correct DFS position and indices are rebuilt.
    ///
    /// If `parent` is `Some` but the key does not exist in the tree, the
    /// operation is a no-op to avoid creating dangling parent references.
    pub fn insert_child(&mut self, parent: Option<&Key>, sibling_index: usize, item: T) {
        // Reject inserts under a nonexistent parent.
        if let Some(pk) = parent {
            if !self.key_to_index.contains_key(pk) {
                return;
            }
        }

        let key = item.key().clone();
        let (level, parent_key_owned) = match parent {
            Some(pk) => {
                let parent_level = self.all_nodes[self.key_to_index[pk]].level;
                (parent_level + 1, Some(pk.clone()))
            }
            None => (0, None),
        };
        let text_value = item.text_value().to_string();
        let node = Node {
            key: key.clone(),
            node_type: NodeType::Item,
            value: Some(item),
            text_value,
            level,
            has_children: false,
            is_expanded: None,
            parent_key: parent_key_owned.clone(),
            index: 0, // recomputed by rebuild_indices
        };

        // Determine flat insertion position: after the parent's existing
        // children at `sibling_index`, or among roots.
        let flat_pos = self.flat_insert_position(parent_key_owned.as_ref(), sibling_index);
        self.all_nodes.insert(flat_pos, node);

        // If inserting under a parent, mark it as having children.
        if let Some(pk) = parent_key_owned.as_ref() {
            if let Some(&pi) = self.key_to_index.get(pk) {
                // pi may have shifted if flat_pos <= pi, adjust
                let adj = if flat_pos <= pi { pi + 1 } else { pi };
                self.all_nodes[adj].has_children = true;
                if self.all_nodes[adj].is_expanded.is_none() {
                    self.all_nodes[adj].is_expanded = Some(false);
                }
            }
        }

        self.rebuild_indices();
    }

    /// Remove items by key (and their entire subtrees).
    pub fn remove_by_keys(&mut self, keys: &[Key]) -> Vec<T> {
        let mut removed = Vec::new();
        for key in keys {
            // Collect the subtree rooted at `key` (DFS order in all_nodes).
            if let Some(&start) = self.key_to_index.get(key) {
                let parent_key = self.all_nodes[start].parent_key.clone();
                let root_level = self.all_nodes[start].level;
                let mut end = start + 1;
                while end < self.all_nodes.len() && self.all_nodes[end].level > root_level {
                    end += 1;
                }
                // Drain the range [start..end] from all_nodes.
                let drained = self.all_nodes.drain(start..end).collect::<Vec<_>>();
                for node in &drained {
                    self.expanded_keys.remove(&node.key);
                }
                for node in drained {
                    if let Some(val) = node.value {
                        removed.push(val);
                    }
                }

                // If the removed node's former parent has no remaining children,
                // reset it to leaf state (has_children = false, is_expanded = None).
                if let Some(pk) = &parent_key {
                    if let Some(&pi) = self.key_to_index.get(pk) {
                        let still_has_children = self
                            .all_nodes
                            .iter()
                            .any(|n| n.parent_key.as_ref() == Some(pk));
                        if !still_has_children {
                            self.all_nodes[pi].has_children = false;
                            self.all_nodes[pi].is_expanded = None;
                            self.expanded_keys.remove(pk);
                        }
                    }
                }

                // Rebuild after each drain so subsequent key lookups use
                // valid indices (drain shifts all_nodes in place).
                self.rebuild_indices();
            }
        }
        removed
    }

    /// Get the flat index of a node by key.
    pub fn flat_index_of(&self, key: &Key) -> Option<usize> {
        self.key_to_index.get(key).copied()
    }

    /// Move a node (and its subtree) to a new parent at the given child index.
    ///
    /// If `new_parent` is `Some` but the key does not exist in the tree
    /// (or is a descendant of the node being moved), the operation is a
    /// no-op to avoid creating dangling parent references.
    pub fn reparent(&mut self, key: &Key, new_parent: Option<&Key>, sibling_index: usize) {
        // Validate new_parent exists before extraction. Checking after
        // extraction is insufficient because the target might be a descendant
        // of the moved node (and thus removed by extract_subtree).
        if let Some(pk) = new_parent {
            if !self.key_to_index.contains_key(pk) {
                return;
            }
        }

        // Save old parent key before extraction for metadata cleanup.
        let old_parent_key = self.parent_of(key).cloned();

        // Extract the subtree.
        let subtree = self.extract_subtree(key);
        if subtree.is_empty() {
            return;
        }

        // Reject reparenting under a descendant of the moved node.
        // After extraction the descendant is no longer in the tree.
        if let Some(pk) = new_parent {
            if !self.key_to_index.contains_key(pk) {
                // Descendant was part of the extracted subtree — re-insert
                // at the original flat position to preserve tree integrity.
                let insert_pos = self.all_nodes.len().min(subtree[0].index);
                for (offset, node) in subtree.into_iter().enumerate() {
                    self.all_nodes.insert(insert_pos + offset, node);
                }
                self.rebuild_indices();
                return;
            }
        }

        // Reset old parent to leaf state if it has no remaining children.
        if let Some(pk) = &old_parent_key {
            if let Some(&pi) = self.key_to_index.get(pk) {
                let still_has_children = self
                    .all_nodes
                    .iter()
                    .any(|n| n.parent_key.as_ref() == Some(pk));
                if !still_has_children {
                    self.all_nodes[pi].has_children = false;
                    self.all_nodes[pi].is_expanded = None;
                    self.expanded_keys.remove(pk);
                }
            }
        }

        // Rebuild indices after extraction so that parent lookups and
        // flat_insert_position operate on valid index state.
        self.rebuild_indices();

        // Recompute levels relative to new parent.
        let new_level = match new_parent {
            Some(pk) => self
                .key_to_index
                .get(pk)
                .map_or(0, |&i| self.all_nodes[i].level + 1),
            None => 0,
        };
        let old_level = subtree[0].level;
        let level_delta = new_level as isize - old_level as isize;

        // Mark the new parent as having children (if it was a leaf).
        if let Some(pk) = new_parent {
            if let Some(&pi) = self.key_to_index.get(pk) {
                self.all_nodes[pi].has_children = true;
                if self.all_nodes[pi].is_expanded.is_none() {
                    self.all_nodes[pi].is_expanded = Some(false);
                }
            }
        }

        let flat_pos = self.flat_insert_position(new_parent, sibling_index);
        for (offset, mut node) in subtree.into_iter().enumerate() {
            node.level = (node.level as isize + level_delta) as usize;
            if offset == 0 {
                node.parent_key = new_parent.cloned();
            }
            self.all_nodes.insert(flat_pos + offset, node);
        }
        self.rebuild_indices();
    }

    /// Reorder a node among its siblings to the given sibling index.
    pub fn reorder_sibling(&mut self, key: &Key, to_sibling_index: usize) {
        let parent_key = self.parent_of(key).cloned();
        let subtree = self.extract_subtree(key);
        if subtree.is_empty() {
            return;
        }

        // Rebuild indices after extraction so flat_insert_position uses valid state.
        self.rebuild_indices();

        let flat_pos = self.flat_insert_position(parent_key.as_ref(), to_sibling_index);
        for (offset, node) in subtree.into_iter().enumerate() {
            self.all_nodes.insert(flat_pos + offset, node);
        }
        self.rebuild_indices();
    }

    /// Replace an item's data (matched by key).
    pub fn replace(&mut self, item: T) {
        let key = item.key().clone();
        if let Some(&idx) = self.key_to_index.get(&key) {
            self.all_nodes[idx].value = Some(item);
        }
    }

    /// Return the parent key of a node, if any.
    fn parent_of(&self, key: &Key) -> Option<&Key> {
        self.key_to_index
            .get(key)
            .and_then(|&i| self.all_nodes[i].parent_key.as_ref())
    }

    /// Extract a node and its subtree from `all_nodes`, returning the
    /// removed nodes. After extraction, indices are stale until
    /// `rebuild_indices()` is called.
    fn extract_subtree(&mut self, key: &Key) -> Vec<Node<T>> {
        if let Some(&start) = self.key_to_index.get(key) {
            let root_level = self.all_nodes[start].level;
            let mut end = start + 1;
            while end < self.all_nodes.len() && self.all_nodes[end].level > root_level {
                end += 1;
            }
            self.all_nodes.drain(start..end).collect()
        } else {
            Vec::new()
        }
    }

    /// Compute the flat insertion position for a new child at `sibling_index`
    /// under the given `parent` (or among roots when `parent` is `None`).
    fn flat_insert_position(&self, parent: Option<&Key>, sibling_index: usize) -> usize {
        let children = if let Some(pk) = parent {
            let parent_level = self
                .key_to_index
                .get(pk)
                .map_or(0, |&i| self.all_nodes[i].level);

            self.all_nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.parent_key.as_ref() == Some(pk) && n.level == parent_level + 1)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else {
            self.all_nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.parent_key.is_none() && n.level == 0)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };
        if sibling_index >= children.len() {
            // Append after the last sibling's subtree.
            if let Some(&last_child_idx) = children.last() {
                let root_level = self.all_nodes[last_child_idx].level;
                let mut end = last_child_idx + 1;
                while end < self.all_nodes.len() && self.all_nodes[end].level > root_level {
                    end += 1;
                }
                end
            } else {
                // No existing children — insert right after the parent.
                if let Some(pk) = parent {
                    self.key_to_index
                        .get(pk)
                        .map_or(self.all_nodes.len(), |&i| i + 1)
                } else {
                    self.all_nodes.len()
                }
            }
        } else {
            children[sibling_index]
        }
    }

    /// Rebuild `key_to_index`, per-node `index` fields, and `visible_indices`
    /// after a structural mutation.
    fn rebuild_indices(&mut self) {
        self.key_to_index.clear();
        for (i, node) in self.all_nodes.iter_mut().enumerate() {
            node.index = i;
            self.key_to_index.insert(node.key.clone(), i);
        }

        let (visible_indices, first_focusable_visible, last_focusable_visible) =
            Self::compute_visible(&self.all_nodes);

        self.visible_indices = visible_indices;
        self.first_focusable_visible = first_focusable_visible;
        self.last_focusable_visible = last_focusable_visible;
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, format, string::ToString, vec, vec::Vec};

    use super::*;

    // ---------------------------------------------------------------
    // Test helper: a CollectionItem impl for mutation tests
    // ---------------------------------------------------------------

    #[derive(Clone, Debug, PartialEq)]
    struct TreeFruit {
        id: Key,
        name: String,
    }

    impl TreeFruit {
        fn new(id: u64, name: &str) -> Self {
            Self {
                id: Key::int(id),
                name: name.to_string(),
            }
        }
    }

    impl CollectionItem for TreeFruit {
        fn key(&self) -> &Key {
            &self.id
        }

        fn text_value(&self) -> &str {
            &self.name
        }
    }

    // ---------------------------------------------------------------
    // Test helpers: tree construction
    // ---------------------------------------------------------------

    fn leaf(key: u64, name: &'static str) -> TreeItemConfig<&'static str> {
        TreeItemConfig {
            key: Key::int(key),
            text_value: name.to_string(),
            value: name,
            children: Vec::new(),
            default_expanded: false,
        }
    }

    fn branch(
        key: u64,
        name: &'static str,
        expanded: bool,
        children: Vec<TreeItemConfig<&'static str>>,
    ) -> TreeItemConfig<&'static str> {
        TreeItemConfig {
            key: Key::int(key),
            text_value: name.to_string(),
            value: name,
            children,
            default_expanded: expanded,
        }
    }

    /// Sample tree:
    /// ```text
    /// 1: Fruits (expanded)
    ///   2: Apple
    ///   3: Banana
    /// 4: Vegetables (collapsed)
    ///   5: Carrot
    ///   6: Daikon
    /// 7: Grains
    /// ```
    fn sample_tree() -> TreeCollection<&'static str> {
        TreeCollection::new(vec![
            branch(1, "Fruits", true, vec![leaf(2, "Apple"), leaf(3, "Banana")]),
            branch(
                4,
                "Vegetables",
                false,
                vec![leaf(5, "Carrot"), leaf(6, "Daikon")],
            ),
            leaf(7, "Grains"),
        ])
    }

    // ---------------------------------------------------------------
    // Construction tests
    // ---------------------------------------------------------------

    #[test]
    fn new_empty() {
        let tree = TreeCollection::<&str>::new(Vec::new());
        assert_eq!(tree.size(), 0);
        assert!(tree.is_empty());
    }

    #[test]
    fn new_single_root() {
        let tree = TreeCollection::new(vec![leaf(1, "Root")]);
        assert_eq!(tree.size(), 1);
        let node = tree.get(&Key::int(1)).expect("root node");
        assert_eq!(node.level, 0);
        assert!(!node.has_children);
        assert_eq!(node.is_expanded, None);
        assert_eq!(node.parent_key, None);
    }

    #[test]
    fn new_flat_roots() {
        let tree = TreeCollection::new(vec![leaf(1, "A"), leaf(2, "B"), leaf(3, "C")]);
        assert_eq!(tree.size(), 3);
        // DFS order: 1, 2, 3
        let keys = tree.keys().collect::<Vec<_>>();
        assert_eq!(keys, vec![&Key::int(1), &Key::int(2), &Key::int(3)]);
    }

    #[test]
    fn new_nested_dfs_order() {
        let tree = sample_tree();
        // All nodes in DFS pre-order: 1, 2, 3, 4, 5, 6, 7
        assert_eq!(tree.all_nodes.len(), 7);
        assert_eq!(tree.all_nodes[0].key, Key::int(1));
        assert_eq!(tree.all_nodes[1].key, Key::int(2));
        assert_eq!(tree.all_nodes[2].key, Key::int(3));
        assert_eq!(tree.all_nodes[3].key, Key::int(4));
        assert_eq!(tree.all_nodes[4].key, Key::int(5));
        assert_eq!(tree.all_nodes[5].key, Key::int(6));
        assert_eq!(tree.all_nodes[6].key, Key::int(7));
    }

    #[test]
    fn new_nested_levels() {
        let tree = sample_tree();
        assert_eq!(tree.all_nodes[0].level, 0); // Fruits
        assert_eq!(tree.all_nodes[1].level, 1); // Apple
        assert_eq!(tree.all_nodes[2].level, 1); // Banana
        assert_eq!(tree.all_nodes[3].level, 0); // Vegetables
        assert_eq!(tree.all_nodes[4].level, 1); // Carrot
        assert_eq!(tree.all_nodes[5].level, 1); // Daikon
        assert_eq!(tree.all_nodes[6].level, 0); // Grains
    }

    #[test]
    fn new_nested_parent_keys() {
        let tree = sample_tree();
        assert_eq!(tree.all_nodes[0].parent_key, None); // Fruits
        assert_eq!(tree.all_nodes[1].parent_key, Some(Key::int(1))); // Apple → Fruits
        assert_eq!(tree.all_nodes[2].parent_key, Some(Key::int(1))); // Banana → Fruits
        assert_eq!(tree.all_nodes[3].parent_key, None); // Vegetables
        assert_eq!(tree.all_nodes[4].parent_key, Some(Key::int(4))); // Carrot → Vegetables
        assert_eq!(tree.all_nodes[5].parent_key, Some(Key::int(4))); // Daikon → Vegetables
        assert_eq!(tree.all_nodes[6].parent_key, None); // Grains
    }

    #[test]
    fn new_has_children() {
        let tree = sample_tree();
        assert!(tree.all_nodes[0].has_children); // Fruits
        assert!(!tree.all_nodes[1].has_children); // Apple
        assert!(!tree.all_nodes[2].has_children); // Banana
        assert!(tree.all_nodes[3].has_children); // Vegetables
        assert!(!tree.all_nodes[6].has_children); // Grains
    }

    #[test]
    fn new_default_expanded() {
        let tree = sample_tree();
        // Fruits is default_expanded=true
        assert!(tree.is_expanded(&Key::int(1)));
        assert_eq!(tree.all_nodes[0].is_expanded, Some(true));
        // Vegetables is default_expanded=false
        assert!(!tree.is_expanded(&Key::int(4)));
        assert_eq!(tree.all_nodes[3].is_expanded, Some(false));
        // Grains is a leaf — is_expanded is None
        assert_eq!(tree.all_nodes[6].is_expanded, None);
    }

    #[test]
    fn new_depth_at_limit() {
        // Build a chain of depth exactly MAX_TREE_DEPTH (levels 0..=32)
        fn chain(depth: usize) -> TreeItemConfig<u64> {
            if depth >= MAX_TREE_DEPTH {
                // Leaf at exactly MAX_TREE_DEPTH — should pass the assert
                TreeItemConfig {
                    key: Key::int(depth as u64),
                    text_value: format!("level-{depth}"),
                    value: depth as u64,
                    children: Vec::new(),
                    default_expanded: false,
                }
            } else {
                TreeItemConfig {
                    key: Key::int(depth as u64),
                    text_value: format!("level-{depth}"),
                    value: depth as u64,
                    children: vec![chain(depth + 1)],
                    default_expanded: true,
                }
            }
        }
        // Levels 0..=32 = 33 nodes, all within MAX_TREE_DEPTH
        let tree = TreeCollection::new(vec![chain(0)]);
        assert_eq!(tree.all_nodes.len(), MAX_TREE_DEPTH + 1);
    }

    #[test]
    #[should_panic(expected = "exceeds MAX_TREE_DEPTH")]
    fn new_depth_exceeded() {
        // Build a chain deeper than MAX_TREE_DEPTH
        fn chain(depth: usize) -> TreeItemConfig<u64> {
            if depth > MAX_TREE_DEPTH + 1 {
                TreeItemConfig {
                    key: Key::int(depth as u64),
                    text_value: format!("level-{depth}"),
                    value: depth as u64,
                    children: Vec::new(),
                    default_expanded: false,
                }
            } else {
                TreeItemConfig {
                    key: Key::int(depth as u64),
                    text_value: format!("level-{depth}"),
                    value: depth as u64,
                    children: vec![chain(depth + 1)],
                    default_expanded: false,
                }
            }
        }
        // This should panic — depth 33 exceeds MAX_TREE_DEPTH (32)
        TreeCollection::new(vec![chain(0)]);
    }

    // ---------------------------------------------------------------
    // Expand / Collapse tests
    // ---------------------------------------------------------------

    #[test]
    fn set_expanded_collapse() {
        let tree = sample_tree();
        // Fruits is expanded — Apple and Banana are visible
        assert!(tree.keys().any(|k| k == &Key::int(2))); // Apple visible

        // Collapse Fruits
        let collapsed = tree.set_expanded(&Key::int(1), false);
        // Apple and Banana should be hidden
        assert!(!collapsed.keys().any(|k| k == &Key::int(2)));
        assert!(!collapsed.keys().any(|k| k == &Key::int(3)));
        // Fruits itself is still visible
        assert!(collapsed.keys().any(|k| k == &Key::int(1)));
    }

    #[test]
    fn set_expanded_expand() {
        let tree = sample_tree();
        // Vegetables is collapsed — Carrot not visible
        assert!(!tree.keys().any(|k| k == &Key::int(5)));

        // Expand Vegetables
        let expanded = tree.set_expanded(&Key::int(4), true);
        assert!(expanded.keys().any(|k| k == &Key::int(5))); // Carrot visible
        assert!(expanded.keys().any(|k| k == &Key::int(6))); // Daikon visible
    }

    #[test]
    fn set_expanded_returns_new_instance() {
        let tree = sample_tree();
        let original_size = tree.size();

        // Collapse Fruits — original should be unchanged
        let collapsed = tree.set_expanded(&Key::int(1), false);
        assert_eq!(tree.size(), original_size);
        assert_ne!(tree.size(), collapsed.size());
    }

    #[test]
    fn is_expanded_true_false() {
        let tree = sample_tree();
        assert!(tree.is_expanded(&Key::int(1))); // Fruits expanded
        assert!(!tree.is_expanded(&Key::int(4))); // Vegetables collapsed
        assert!(!tree.is_expanded(&Key::int(7))); // Grains is a leaf
        assert!(!tree.is_expanded(&Key::int(99))); // nonexistent
    }

    #[test]
    fn set_expanded_nonexistent_key() {
        let tree = sample_tree();
        let result = tree.set_expanded(&Key::int(99), true);
        // Should not panic; tree should be functionally equivalent
        assert_eq!(result.all_nodes.len(), tree.all_nodes.len());
    }

    #[test]
    fn collapse_already_collapsed() {
        let tree = sample_tree();
        // Vegetables is already collapsed
        let result = tree.set_expanded(&Key::int(4), false);
        assert!(!result.is_expanded(&Key::int(4)));
        // Visible set should be the same
        let orig_keys = tree.keys().collect::<Vec<_>>();
        let result_keys = result.keys().collect::<Vec<_>>();
        assert_eq!(orig_keys, result_keys);
    }

    #[test]
    fn expand_leaf_node() {
        let tree = sample_tree();
        // Grains is a leaf — expanding it is a no-op
        let result = tree.set_expanded(&Key::int(7), true);
        let orig_keys = tree.keys().collect::<Vec<_>>();
        let result_keys = result.keys().collect::<Vec<_>>();
        assert_eq!(orig_keys, result_keys);
    }

    // ---------------------------------------------------------------
    // Visibility tests
    // ---------------------------------------------------------------

    #[test]
    fn visibility_with_mixed_expansion() {
        let tree = sample_tree();
        // Fruits expanded, Vegetables collapsed
        // Visible: 1(Fruits), 2(Apple), 3(Banana), 4(Vegetables), 7(Grains)
        assert_eq!(tree.size(), 5);
        let visible_keys = tree.keys().collect::<Vec<_>>();
        assert_eq!(
            visible_keys,
            vec![
                &Key::int(1),
                &Key::int(2),
                &Key::int(3),
                &Key::int(4),
                &Key::int(7),
            ]
        );
    }

    #[test]
    fn visibility_all_expanded() {
        let tree = sample_tree();
        let fully_expanded = tree.set_expanded(&Key::int(4), true);
        // All 7 nodes visible
        assert_eq!(fully_expanded.size(), 7);
    }

    #[test]
    fn visibility_all_collapsed() {
        let tree = sample_tree();
        let collapsed = tree.set_expanded(&Key::int(1), false);
        // Only roots visible: 1(Fruits), 4(Vegetables), 7(Grains)
        assert_eq!(collapsed.size(), 3);
    }

    #[test]
    fn visible_keys_with_expanded_external_set() {
        let tree = sample_tree();
        // External expanded set: only Vegetables expanded
        let mut expanded = BTreeSet::new();
        expanded.insert(Key::int(4));

        let visible = tree.visible_keys_with_expanded(&expanded);
        // Fruits collapsed (children hidden), Vegetables expanded (children shown)
        assert_eq!(
            visible,
            vec![
                Key::int(1),
                Key::int(4),
                Key::int(5),
                Key::int(6),
                Key::int(7),
            ]
        );
    }

    #[test]
    fn is_visible_with_expanded_root() {
        let tree = sample_tree();
        let expanded = BTreeSet::new(); // nothing expanded
        // Root nodes are always visible
        assert!(tree.is_visible_with_expanded(&Key::int(1), &expanded));
        assert!(tree.is_visible_with_expanded(&Key::int(4), &expanded));
        assert!(tree.is_visible_with_expanded(&Key::int(7), &expanded));
    }

    #[test]
    fn is_visible_with_expanded_child_of_collapsed() {
        let tree = sample_tree();
        let expanded = BTreeSet::new(); // nothing expanded
        // Apple is child of Fruits which is not expanded
        assert!(!tree.is_visible_with_expanded(&Key::int(2), &expanded));
    }

    #[test]
    fn is_visible_with_expanded_child_of_expanded() {
        let tree = sample_tree();
        let mut expanded = BTreeSet::new();
        expanded.insert(Key::int(1)); // Fruits expanded
        assert!(tree.is_visible_with_expanded(&Key::int(2), &expanded)); // Apple visible
    }

    #[test]
    fn is_visible_with_expanded_nonexistent_key() {
        let tree = sample_tree();
        let expanded = BTreeSet::new();
        assert!(!tree.is_visible_with_expanded(&Key::int(99), &expanded));
    }

    #[test]
    fn is_visible_with_expanded_deeply_nested() {
        // Build a 3-level tree: root → mid → leaf
        let tree = TreeCollection::new(vec![branch(
            1,
            "Root",
            true,
            vec![branch(2, "Mid", true, vec![leaf(3, "Leaf")])],
        )]);

        // All expanded — leaf is visible
        let mut all_expanded = BTreeSet::new();
        all_expanded.insert(Key::int(1));
        all_expanded.insert(Key::int(2));
        assert!(tree.is_visible_with_expanded(&Key::int(3), &all_expanded));

        // Only root expanded, mid collapsed — leaf is NOT visible
        let mut root_only = BTreeSet::new();
        root_only.insert(Key::int(1));
        assert!(!tree.is_visible_with_expanded(&Key::int(3), &root_only));

        // Nothing expanded — mid is NOT visible
        let none_expanded = BTreeSet::new();
        assert!(!tree.is_visible_with_expanded(&Key::int(2), &none_expanded));
    }

    // ---------------------------------------------------------------
    // Collection trait — size
    // ---------------------------------------------------------------

    #[test]
    fn size_equals_visible_count() {
        let tree = sample_tree();
        // 5 visible nodes (Fruits expanded, Vegetables collapsed)
        assert_eq!(tree.size(), 5);
    }

    // ---------------------------------------------------------------
    // Collection trait — random access
    // ---------------------------------------------------------------

    #[test]
    fn get_returns_any_node() {
        let tree = sample_tree();
        // Carrot (key 5) is inside collapsed Vegetables — still accessible via get
        let node = tree.get(&Key::int(5)).expect("Carrot should exist");
        assert_eq!(node.text_value, "Carrot");
    }

    #[test]
    fn get_missing_key() {
        let tree = sample_tree();
        assert!(tree.get(&Key::int(99)).is_none());
    }

    #[test]
    fn get_by_index_uses_visible() {
        let tree = sample_tree();
        // Visible order: 1(Fruits), 2(Apple), 3(Banana), 4(Vegetables), 7(Grains)
        let node = tree.get_by_index(0).expect("index 0");
        assert_eq!(node.key, Key::int(1)); // Fruits
        let node = tree.get_by_index(3).expect("index 3");
        assert_eq!(node.key, Key::int(4)); // Vegetables
        let node = tree.get_by_index(4).expect("index 4");
        assert_eq!(node.key, Key::int(7)); // Grains
    }

    #[test]
    fn get_by_index_out_of_range() {
        let tree = sample_tree();
        assert!(tree.get_by_index(100).is_none());
    }

    // ---------------------------------------------------------------
    // Collection trait — boundary navigation
    // ---------------------------------------------------------------

    #[test]
    fn first_key_and_last_key() {
        let tree = sample_tree();
        assert_eq!(tree.first_key(), Some(&Key::int(1)));
        assert_eq!(tree.last_key(), Some(&Key::int(7)));
    }

    #[test]
    fn first_key_empty() {
        let tree = TreeCollection::<&str>::new(Vec::new());
        assert_eq!(tree.first_key(), None);
        assert_eq!(tree.last_key(), None);
    }

    // ---------------------------------------------------------------
    // Collection trait — wrapping navigation
    // ---------------------------------------------------------------

    #[test]
    fn key_after_middle() {
        let tree = sample_tree();
        // After Apple(2) → Banana(3)
        assert_eq!(tree.key_after(&Key::int(2)), Some(&Key::int(3)));
    }

    #[test]
    fn key_after_wraps_at_end() {
        let tree = sample_tree();
        // After Grains(7) wraps to Fruits(1)
        assert_eq!(tree.key_after(&Key::int(7)), Some(&Key::int(1)));
    }

    #[test]
    fn key_before_middle() {
        let tree = sample_tree();
        // Before Banana(3) → Apple(2)
        assert_eq!(tree.key_before(&Key::int(3)), Some(&Key::int(2)));
    }

    #[test]
    fn key_before_wraps_at_start() {
        let tree = sample_tree();
        // Before Fruits(1) wraps to Grains(7)
        assert_eq!(tree.key_before(&Key::int(1)), Some(&Key::int(7)));
    }

    // ---------------------------------------------------------------
    // Collection trait — non-wrapping navigation
    // ---------------------------------------------------------------

    #[test]
    fn key_after_no_wrap_middle() {
        let tree = sample_tree();
        assert_eq!(tree.key_after_no_wrap(&Key::int(2)), Some(&Key::int(3)));
    }

    #[test]
    fn key_after_no_wrap_at_end() {
        let tree = sample_tree();
        assert_eq!(tree.key_after_no_wrap(&Key::int(7)), None);
    }

    #[test]
    fn key_before_no_wrap_at_start() {
        let tree = sample_tree();
        assert_eq!(tree.key_before_no_wrap(&Key::int(1)), None);
    }

    #[test]
    fn key_after_no_wrap_skips_collapsed() {
        let tree = sample_tree();
        // After Banana(3) → Vegetables(4), skipping collapsed children 5,6
        assert_eq!(tree.key_after_no_wrap(&Key::int(3)), Some(&Key::int(4)));
        // After Vegetables(4) → Grains(7), because 5,6 are not visible
        assert_eq!(tree.key_after_no_wrap(&Key::int(4)), Some(&Key::int(7)));
    }

    #[test]
    fn key_after_no_wrap_unknown_key() {
        let tree = sample_tree();
        assert_eq!(tree.key_after_no_wrap(&Key::int(99)), None);
    }

    // ---------------------------------------------------------------
    // Collection trait — iteration
    // ---------------------------------------------------------------

    #[test]
    fn keys_iterator_only_visible() {
        let tree = sample_tree();
        let keys = tree.keys().collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![
                &Key::int(1),
                &Key::int(2),
                &Key::int(3),
                &Key::int(4),
                &Key::int(7),
            ]
        );
    }

    #[test]
    fn nodes_iterator_only_visible() {
        let tree = sample_tree();
        let text_values = tree
            .nodes()
            .map(|n| n.text_value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            text_values,
            vec!["Fruits", "Apple", "Banana", "Vegetables", "Grains"]
        );
    }

    #[test]
    fn item_keys_filters_visible_focusable() {
        let tree = sample_tree();
        // All visible nodes are Item type, so item_keys == keys for this tree
        let item_keys = tree.item_keys().collect::<Vec<_>>();
        assert_eq!(
            item_keys,
            vec![
                &Key::int(1),
                &Key::int(2),
                &Key::int(3),
                &Key::int(4),
                &Key::int(7),
            ]
        );
    }

    // ---------------------------------------------------------------
    // Collection trait — children
    // ---------------------------------------------------------------

    #[test]
    fn children_of_returns_all_including_collapsed() {
        let tree = sample_tree();
        // Vegetables(4) is collapsed but children_of should still return its children
        let children = tree.children_of(&Key::int(4)).collect::<Vec<_>>();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].key, Key::int(5)); // Carrot
        assert_eq!(children[1].key, Key::int(6)); // Daikon
    }

    #[test]
    fn children_of_leaf() {
        let tree = sample_tree();
        let children = tree.children_of(&Key::int(7)).collect::<Vec<_>>();
        assert!(children.is_empty());
    }

    #[test]
    fn children_of_nonexistent() {
        let tree = sample_tree();
        let children = tree.children_of(&Key::int(99)).collect::<Vec<_>>();
        assert!(children.is_empty());
    }

    // ---------------------------------------------------------------
    // Collection trait — text value
    // ---------------------------------------------------------------

    #[test]
    fn text_value_of_existing() {
        let tree = sample_tree();
        assert_eq!(tree.text_value_of(&Key::int(2)), Some("Apple"));
    }

    #[test]
    fn text_value_of_missing() {
        let tree = sample_tree();
        assert_eq!(tree.text_value_of(&Key::int(99)), None);
    }

    // ---------------------------------------------------------------
    // Mutation tests (require T: CollectionItem)
    // ---------------------------------------------------------------

    fn fruit_tree() -> TreeCollection<TreeFruit> {
        TreeCollection::new(vec![
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Fruits".to_string(),
                value: TreeFruit::new(1, "Fruits"),
                children: vec![
                    TreeItemConfig {
                        key: Key::int(2),
                        text_value: "Apple".to_string(),
                        value: TreeFruit::new(2, "Apple"),
                        children: Vec::new(),
                        default_expanded: false,
                    },
                    TreeItemConfig {
                        key: Key::int(3),
                        text_value: "Banana".to_string(),
                        value: TreeFruit::new(3, "Banana"),
                        children: Vec::new(),
                        default_expanded: false,
                    },
                ],
                default_expanded: true,
            },
            TreeItemConfig {
                key: Key::int(4),
                text_value: "Other".to_string(),
                value: TreeFruit::new(4, "Other"),
                children: Vec::new(),
                default_expanded: false,
            },
        ])
    }

    #[test]
    fn insert_child_at_root() {
        let mut tree = fruit_tree();
        let original_size = tree.all_nodes.len();
        tree.insert_child(None, 0, TreeFruit::new(10, "Root Item"));
        assert_eq!(tree.all_nodes.len(), original_size + 1);
        // Inserted at root level, sibling_index 0 — should be first node
        assert_eq!(tree.all_nodes[0].key, Key::int(10));
        assert_eq!(tree.all_nodes[0].level, 0);
        assert_eq!(tree.all_nodes[0].parent_key, None);
    }

    #[test]
    fn insert_child_under_parent() {
        let mut tree = fruit_tree();
        // Insert Cherry under Fruits at sibling_index 2 (after Banana)
        tree.insert_child(Some(&Key::int(1)), 2, TreeFruit::new(5, "Cherry"));
        // Cherry should be a child of Fruits at level 1
        let cherry = tree.get(&Key::int(5)).expect("Cherry");
        assert_eq!(cherry.level, 1);
        assert_eq!(cherry.parent_key, Some(Key::int(1)));
    }

    #[test]
    fn insert_child_updates_parent_has_children() {
        let mut tree = fruit_tree();
        // Other(4) is a leaf
        assert!(!tree.get(&Key::int(4)).expect("Other").has_children);

        // Insert a child under Other
        tree.insert_child(Some(&Key::int(4)), 0, TreeFruit::new(10, "Sub"));
        assert!(tree.get(&Key::int(4)).expect("Other").has_children);
    }

    #[test]
    fn remove_by_keys_removes_subtree() {
        let mut tree = fruit_tree();
        // Remove Fruits(1) — should also remove Apple(2) and Banana(3)
        let removed = tree.remove_by_keys(&[Key::int(1)]);
        assert_eq!(removed.len(), 3); // Fruits + Apple + Banana
        assert!(!tree.contains_key(&Key::int(1)));
        assert!(!tree.contains_key(&Key::int(2)));
        assert!(!tree.contains_key(&Key::int(3)));
        // Other(4) should still be there
        assert!(tree.contains_key(&Key::int(4)));
    }

    #[test]
    fn remove_by_keys_missing() {
        let mut tree = fruit_tree();
        let removed = tree.remove_by_keys(&[Key::int(99)]);
        assert!(removed.is_empty());
        assert_eq!(tree.all_nodes.len(), 4);
    }

    #[test]
    fn flat_index_of_returns_correct_index() {
        let tree = fruit_tree();
        assert_eq!(tree.flat_index_of(&Key::int(1)), Some(0));
        assert_eq!(tree.flat_index_of(&Key::int(2)), Some(1));
        assert_eq!(tree.flat_index_of(&Key::int(3)), Some(2));
        assert_eq!(tree.flat_index_of(&Key::int(4)), Some(3));
        assert_eq!(tree.flat_index_of(&Key::int(99)), None);
    }

    #[test]
    fn reparent_to_root() {
        let mut tree = fruit_tree();
        // Move Apple(2) from under Fruits(1) to root level
        tree.reparent(&Key::int(2), None, 0);
        let apple = tree.get(&Key::int(2)).expect("Apple");
        assert_eq!(apple.level, 0);
        assert_eq!(apple.parent_key, None);
    }

    #[test]
    fn reparent_to_different_parent() {
        let mut tree = fruit_tree();
        // Move Apple(2) from under Fruits(1) to under Other(4)
        tree.reparent(&Key::int(2), Some(&Key::int(4)), 0);
        let apple = tree.get(&Key::int(2)).expect("Apple");
        assert_eq!(apple.parent_key, Some(Key::int(4)));
        // Level should adjust: Other is level 0, so Apple is level 1
        assert_eq!(apple.level, 1);
    }

    #[test]
    fn reorder_sibling() {
        let mut tree = fruit_tree();
        // Banana(3) is sibling index 1 under Fruits. Move to index 0.
        tree.reorder_sibling(&Key::int(3), 0);
        // Now Banana should come before Apple in DFS
        let children = tree.children_of(&Key::int(1)).collect::<Vec<_>>();
        assert_eq!(children[0].key, Key::int(3)); // Banana first
        assert_eq!(children[1].key, Key::int(2)); // Apple second
    }

    #[test]
    fn replace_value() {
        let mut tree = fruit_tree();
        tree.replace(TreeFruit::new(2, "Green Apple"));
        let node = tree.get(&Key::int(2)).expect("Apple");
        assert_eq!(node.value.as_ref().expect("value").name, "Green Apple");
    }

    #[test]
    fn replace_missing_key() {
        let mut tree = fruit_tree();
        tree.replace(TreeFruit::new(99, "Unknown"));
        // No change — key 99 doesn't exist
        assert!(!tree.contains_key(&Key::int(99)));
    }

    // ---------------------------------------------------------------
    // Manual trait impl tests
    // ---------------------------------------------------------------

    #[test]
    fn clone_produces_equal_collection() {
        let tree = sample_tree();
        let cloned = tree.clone();
        assert_eq!(tree, cloned);
    }

    #[test]
    fn debug_contains_name_and_counts() {
        let tree = sample_tree();
        let debug = format!("{tree:?}");
        assert!(debug.contains("TreeCollection"));
        assert!(debug.contains("7")); // total_nodes
        assert!(debug.contains("5")); // visible_nodes
    }

    #[test]
    fn partial_eq_equal() {
        let a = sample_tree();
        let b = sample_tree();
        assert_eq!(a, b);
    }

    #[test]
    fn partial_eq_different_expansion() {
        let a = sample_tree();
        let b = a.set_expanded(&Key::int(4), true);
        assert_ne!(a, b);
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn single_item_wrapping_navigation() {
        let tree = TreeCollection::new(vec![leaf(1, "Only")]);
        assert_eq!(tree.key_after(&Key::int(1)), Some(&Key::int(1)));
        assert_eq!(tree.key_before(&Key::int(1)), Some(&Key::int(1)));
        assert_eq!(tree.key_after_no_wrap(&Key::int(1)), None);
        assert_eq!(tree.key_before_no_wrap(&Key::int(1)), None);
    }

    #[test]
    fn empty_tree_navigation() {
        let tree = TreeCollection::<&str>::new(Vec::new());
        assert_eq!(tree.first_key(), None);
        assert_eq!(tree.last_key(), None);
        assert_eq!(tree.key_after(&Key::int(1)), None);
        assert_eq!(tree.key_before(&Key::int(1)), None);
    }

    #[test]
    fn contains_key_in_collapsed_subtree() {
        let tree = sample_tree();
        // Carrot(5) is inside collapsed Vegetables — contains_key should still find it
        assert!(tree.contains_key(&Key::int(5)));
    }

    #[test]
    fn nested_collapse_hides_only_subtree() {
        // Tree:
        //   1: A (expanded)
        //     2: B (collapsed)
        //       3: C
        //     4: D
        let tree = TreeCollection::new(vec![branch(
            1,
            "A",
            true,
            vec![branch(2, "B", false, vec![leaf(3, "C")]), leaf(4, "D")],
        )]);
        // Visible: A, B, D (C is inside collapsed B)
        assert_eq!(tree.size(), 3);
        let visible_keys = tree.keys().collect::<Vec<_>>();
        assert_eq!(visible_keys, vec![&Key::int(1), &Key::int(2), &Key::int(4)]);
    }

    #[test]
    fn node_index_fields_correct() {
        let tree = sample_tree();
        for (i, node) in tree.all_nodes.iter().enumerate() {
            assert_eq!(node.index, i, "Node at position {i} has wrong index field");
        }
    }

    // ---------------------------------------------------------------
    // Branch coverage: navigation from non-visible (collapsed) node
    // ---------------------------------------------------------------

    #[test]
    fn key_after_no_wrap_from_collapsed_node() {
        let tree = sample_tree();
        // Carrot(5) is inside collapsed Vegetables — it exists in key_to_index
        // but its flat index is NOT in visible_indices. The position() call
        // returns None, so key_after_no_wrap returns None.
        assert_eq!(tree.key_after_no_wrap(&Key::int(5)), None);
    }

    #[test]
    fn key_before_no_wrap_from_collapsed_node() {
        let tree = sample_tree();
        // Same for key_before_no_wrap on a non-visible key
        assert_eq!(tree.key_before_no_wrap(&Key::int(5)), None);
    }

    #[test]
    fn insert_child_at_index_zero_before_parent() {
        let mut tree = fruit_tree();
        // Insert as first child of Fruits(1). Fruits is at flat index 0.
        // The first child slot (sibling_index=0) is at flat index 1 (after parent).
        // This exercises the case where flat_pos > pi (insertion after parent).
        tree.insert_child(Some(&Key::int(1)), 0, TreeFruit::new(10, "Mango"));
        let mango = tree.get(&Key::int(10)).expect("Mango");
        assert_eq!(mango.parent_key, Some(Key::int(1)));
        assert_eq!(mango.level, 1);
    }

    #[test]
    fn reparent_subtree_adjusts_child_levels() {
        // Tree: root(1) -> mid(2) -> leaf(3)
        let mut tree = TreeCollection::new(vec![TreeItemConfig {
            key: Key::int(1),
            text_value: "Root".to_string(),
            value: TreeFruit::new(1, "Root"),
            children: vec![TreeItemConfig {
                key: Key::int(2),
                text_value: "Mid".to_string(),
                value: TreeFruit::new(2, "Mid"),
                children: vec![TreeItemConfig {
                    key: Key::int(3),
                    text_value: "Leaf".to_string(),
                    value: TreeFruit::new(3, "Leaf"),
                    children: Vec::new(),
                    default_expanded: false,
                }],
                default_expanded: true,
            }],
            default_expanded: true,
        }]);

        // Reparent Mid(2) + Leaf(3) to root level
        tree.reparent(&Key::int(2), None, 1);
        let mid = tree.get(&Key::int(2)).expect("Mid");
        assert_eq!(mid.level, 0);
        assert_eq!(mid.parent_key, None);
        // Leaf should also adjust: was level 2, now level 1
        let leaf_node = tree.get(&Key::int(3)).expect("Leaf");
        assert_eq!(leaf_node.level, 1);
    }

    #[test]
    fn reorder_sibling_nonexistent_key() {
        let mut tree = fruit_tree();
        // Reorder a nonexistent key — should be a no-op
        tree.reorder_sibling(&Key::int(99), 0);
        assert_eq!(tree.all_nodes.len(), 4);
    }

    #[test]
    fn reparent_nonexistent_key() {
        let mut tree = fruit_tree();
        tree.reparent(&Key::int(99), None, 0);
        assert_eq!(tree.all_nodes.len(), 4);
    }

    #[test]
    fn tree_item_config_debug() {
        let config = leaf(1, "Apple");
        let debug = format!("{config:?}");
        assert!(debug.contains("TreeItemConfig"));
        assert!(debug.contains("Apple"));
    }

    #[test]
    fn insert_child_into_empty_tree() {
        // Covers flat_insert_position: parent=None, no existing root children
        let mut tree = TreeCollection::default();
        tree.insert_child(None, 0, TreeFruit::new(1, "First"));
        assert_eq!(tree.all_nodes.len(), 1);
        assert_eq!(tree.all_nodes[0].key, Key::int(1));
    }

    #[test]
    fn insert_child_after_sibling_with_subtree() {
        // Covers flat_insert_position: append after last sibling whose subtree
        // must be skipped (the while end += 1 loop body).
        let mut tree = fruit_tree();
        // Fruits(1) has children Apple(2), Banana(3).
        // Insert at sibling_index=999 (past end) → appends after Banana's subtree.
        // But Banana has no children so the while loop doesn't execute.
        // Instead, give Banana a child first, then insert after it.
        tree.insert_child(Some(&Key::int(3)), 0, TreeFruit::new(30, "Baby Banana"));
        // Now Banana(3) has child Baby Banana(30). Insert new sibling after Banana
        // under Fruits at sibling_index=999 (past end of 2 children).
        tree.insert_child(Some(&Key::int(1)), 999, TreeFruit::new(5, "Cherry"));
        // Cherry should appear after Banana's subtree (after Baby Banana)
        let cherry = tree.get(&Key::int(5)).expect("Cherry");
        assert_eq!(cherry.parent_key, Some(Key::int(1)));
        assert_eq!(cherry.level, 1);
        // Verify DFS order: Cherry comes after Baby Banana
        let cherry_idx = tree.flat_index_of(&Key::int(5)).expect("cherry index");
        let baby_idx = tree.flat_index_of(&Key::int(30)).expect("baby index");
        assert!(cherry_idx > baby_idx);
    }

    // ---------------------------------------------------------------
    // Cached first_key / last_key correctness
    // ---------------------------------------------------------------

    #[test]
    fn first_last_key_update_on_expand_collapse() {
        // All collapsed: visible = [Fruits(1), Vegetables(4), Grains(7)]
        let all_collapsed = sample_tree().set_expanded(&Key::int(1), false);
        assert_eq!(all_collapsed.first_key(), Some(&Key::int(1)));
        assert_eq!(all_collapsed.last_key(), Some(&Key::int(7)));

        // Expand Vegetables: visible adds Carrot(5), Daikon(6)
        let veg_expanded = all_collapsed.set_expanded(&Key::int(4), true);
        assert_eq!(veg_expanded.first_key(), Some(&Key::int(1)));
        assert_eq!(veg_expanded.last_key(), Some(&Key::int(7)));

        // Collapse everything except Vegetables → last visible focusable is Daikon(6)
        // Actually Grains(7) is always visible (root), so last stays 7.
        // Better test: build a tree with a single root that has children.
        let single = TreeCollection::new(vec![branch(
            1,
            "Root",
            true,
            vec![leaf(2, "A"), leaf(3, "B")],
        )]);
        assert_eq!(single.first_key(), Some(&Key::int(1)));
        assert_eq!(single.last_key(), Some(&Key::int(3)));

        let collapsed = single.set_expanded(&Key::int(1), false);
        assert_eq!(collapsed.first_key(), Some(&Key::int(1)));
        assert_eq!(collapsed.last_key(), Some(&Key::int(1))); // only root visible
    }

    #[test]
    fn first_last_key_update_after_insert() {
        let mut tree = fruit_tree();
        let orig_first = tree.first_key().cloned();
        // Insert at root position 0 — new node becomes first
        tree.insert_child(None, 0, TreeFruit::new(0, "Zzz First"));
        assert_eq!(tree.first_key(), Some(&Key::int(0)));
        // Original first is now second
        assert_ne!(tree.first_key().cloned(), orig_first);
    }

    #[test]
    fn first_last_key_update_after_remove() {
        let mut tree = fruit_tree();
        assert_eq!(tree.last_key(), Some(&Key::int(4))); // Other
        tree.remove_by_keys(&[Key::int(4)]);
        // After removing Other(4), last focusable is Banana(3)
        assert_eq!(tree.last_key(), Some(&Key::int(3)));
    }

    #[test]
    fn first_last_key_empty_after_clear() {
        let mut tree = fruit_tree();
        // Multi-key removal is safe — rebuild_indices runs after each drain.
        tree.remove_by_keys(&[Key::int(1), Key::int(4)]);
        assert_eq!(tree.first_key(), None);
        assert_eq!(tree.last_key(), None);
    }

    // ---------------------------------------------------------------
    // PR review fixes
    // ---------------------------------------------------------------

    #[test]
    fn remove_by_keys_multi_key_safe() {
        // Previously panicked due to stale key_to_index after first drain.
        let mut tree = fruit_tree();
        // Fruits(1) has children Apple(2), Banana(3); Other(4) is a root.
        // Remove both roots in a single call.
        let removed = tree.remove_by_keys(&[Key::int(1), Key::int(4)]);
        assert_eq!(removed.len(), 4); // Fruits + Apple + Banana + Other
        assert!(tree.is_empty());
    }

    #[test]
    fn remove_by_keys_cleans_expanded_keys() {
        let tree = sample_tree();
        // Fruits(1) is expanded
        assert!(tree.is_expanded(&Key::int(1)));

        // Convert to mutable tree via CollectionItem
        let mut tree = fruit_tree();
        // Expand Fruits in fruit_tree (it's already default_expanded=true)
        assert!(tree.is_expanded(&Key::int(1)));

        tree.remove_by_keys(&[Key::int(1)]);
        // Stale expanded key should be cleaned up
        assert!(!tree.is_expanded(&Key::int(1)));
    }

    #[test]
    fn reparent_marks_new_parent_has_children() {
        let mut tree = fruit_tree();
        // Other(4) is a leaf — has_children == false
        assert!(!tree.get(&Key::int(4)).expect("Other").has_children);

        // Reparent Apple(2) under Other(4)
        tree.reparent(&Key::int(2), Some(&Key::int(4)), 0);
        let other = tree.get(&Key::int(4)).expect("Other after reparent");
        assert!(other.has_children);
        assert_eq!(other.is_expanded, Some(false));
    }

    #[test]
    fn reparent_collapse_new_parent_hides_children() {
        let mut tree = fruit_tree();
        // Reparent Apple(2) under Other(4)
        tree.reparent(&Key::int(2), Some(&Key::int(4)), 0);
        // Other is now a parent with is_expanded=Some(false), so Apple is hidden
        assert!(!tree.keys().any(|k| k == &Key::int(2)));

        // Expand Other — Apple becomes visible
        let expanded = tree.set_expanded(&Key::int(4), true);
        assert!(expanded.keys().any(|k| k == &Key::int(2)));
    }

    #[test]
    fn set_expanded_on_leaf_is_noop() {
        let tree = sample_tree();
        // Grains(7) is a leaf
        let node = tree.get(&Key::int(7)).expect("Grains");
        assert_eq!(node.is_expanded, None);
        assert!(!node.has_children);

        // Attempting to expand a leaf should not change anything
        let after = tree.set_expanded(&Key::int(7), true);
        let node = after.get(&Key::int(7)).expect("Grains after");
        assert_eq!(node.is_expanded, None); // still None, not Some(true)
        assert!(!after.is_expanded(&Key::int(7))); // not in expanded_keys
    }

    #[test]
    fn set_expanded_on_missing_key_is_noop() {
        let tree = sample_tree();
        let after = tree.set_expanded(&Key::int(99), true);
        // Should not insert phantom key into expanded_keys
        assert!(!after.is_expanded(&Key::int(99)));
    }

    #[test]
    fn partial_eq_detects_hierarchy_change() {
        let tree_a = fruit_tree();
        let mut tree_b = fruit_tree();
        // Reparent Apple(2) to root level — same nodes, different hierarchy
        tree_b.reparent(&Key::int(2), None, 0);
        assert_ne!(tree_a, tree_b);
    }

    #[test]
    fn reparent_to_nonexistent_parent_is_noop() {
        let mut tree = fruit_tree();
        let before_keys: Vec<_> = tree.keys().cloned().collect();
        // Reparent Apple(2) under nonexistent key 99 — should be a no-op
        tree.reparent(&Key::int(2), Some(&Key::int(99)), 0);
        let after_keys: Vec<_> = tree.keys().cloned().collect();
        // Tree should be unchanged
        assert_eq!(before_keys, after_keys);
        // Apple should still exist with original parent
        let apple = tree.get(&Key::int(2)).expect("Apple");
        assert_eq!(apple.parent_key, Some(Key::int(1)));
    }

    #[test]
    fn remove_last_child_resets_parent_to_leaf() {
        let mut tree = fruit_tree();
        // Fruits(1) has children Apple(2) and Banana(3)
        assert!(tree.get(&Key::int(1)).expect("Fruits").has_children);

        // Remove both children
        tree.remove_by_keys(&[Key::int(2)]);
        tree.remove_by_keys(&[Key::int(3)]);

        // Fruits should now be a leaf
        let fruits = tree.get(&Key::int(1)).expect("Fruits after");
        assert!(!fruits.has_children);
        assert_eq!(fruits.is_expanded, None);
        assert!(!tree.is_expanded(&Key::int(1)));
    }

    #[test]
    fn remove_one_child_keeps_parent_as_branch() {
        let mut tree = fruit_tree();
        // Remove only Apple(2), Banana(3) still under Fruits
        tree.remove_by_keys(&[Key::int(2)]);
        let fruits = tree.get(&Key::int(1)).expect("Fruits");
        assert!(fruits.has_children); // still has Banana
    }

    #[test]
    fn insert_child_under_missing_parent_is_noop() {
        let mut tree = fruit_tree();
        let before_len = tree.all_nodes.len();
        // Attempt to insert under nonexistent key 99
        tree.insert_child(Some(&Key::int(99)), 0, TreeFruit::new(10, "Orphan"));
        assert_eq!(tree.all_nodes.len(), before_len);
        assert!(!tree.contains_key(&Key::int(10)));
    }

    #[test]
    fn reparent_resets_old_parent_to_leaf() {
        let mut tree = fruit_tree();
        // Fruits(1) has Apple(2) and Banana(3)
        assert!(tree.get(&Key::int(1)).expect("Fruits").has_children);

        // Move Apple to root
        tree.reparent(&Key::int(2), None, 0);
        // Fruits still has Banana
        assert!(tree.get(&Key::int(1)).expect("Fruits").has_children);

        // Move Banana to root too — Fruits has no children left
        tree.reparent(&Key::int(3), None, 0);
        let fruits = tree.get(&Key::int(1)).expect("Fruits after");
        assert!(!fruits.has_children);
        assert_eq!(fruits.is_expanded, None);
    }
}
