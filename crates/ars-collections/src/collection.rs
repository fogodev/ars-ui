// ars-collections/src/collection.rs

use crate::{key::Key, node::Node};

/// Trait for items stored in collections. Provides the key and
/// optional text value used for typeahead matching and collation.
pub trait CollectionItem {
    /// The unique key identifying this item within a collection.
    fn key(&self) -> &Key;

    /// Human-readable text for typeahead matching and collation sorting.
    /// Returns `""` by default for items without meaningful text content.
    fn text_value(&self) -> &str {
        ""
    }
}

/// A read-only, ordered collection of typed nodes.
///
/// # Design rationale — RPITIT over `dyn Iterator`
///
/// Several methods on this trait (`keys`, `nodes`, `item_keys`, `children_of`)
/// return `impl Iterator<…>` via Rust's *Return Position Impl Trait In Traits*
/// (RPITIT, stabilised in Rust 1.75). This choice has two consequences:
///
/// 1. **No heap allocation per call**: each implementor can return its own
///    concrete iterator type (`slice::Iter`, `indexmap::Keys`, a filter
///    adapter, etc.) without boxing.
/// 2. **`Collection<T>` is NOT object-safe**: you cannot write
///    `dyn Collection<T>`. This is an intentional trade-off — every
///    component that needs a collection is generic over `C: Collection<T>`
///    and monomorphised at compile time, which is the common path for
///    headless UI libraries.
///
/// For the rare case where type-erasure *is* needed (e.g., plugin systems
/// or heterogeneous collection stores), the companion trait
/// `AnyCollection<T>` provides a `dyn`-safe wrapper by boxing the
/// iterators. Blanket `impl<C: Collection<T>> AnyCollection<T> for C`
/// is supplied so that any concrete collection can be used as
/// `&dyn AnyCollection<T>` with no additional work.
///
/// Implementations must guarantee:
/// - All keys returned by iterators and navigation methods are stable
///   for the lifetime of the collection.
/// - `key_after` / `key_before` skip non-focusable nodes (Sections,
///   Headers, Separators) so that callers never need to filter manually.
/// - `get` and the navigation methods are O(1) or O(log n). Implementations
///   backed by `IndexMap` or `Vec` with an index satisfy this.
// Note: The Collection trait does not require Send + Sync — these bounds
// are not needed by any framework adapter.
pub trait Collection<T> {
    // ------------------------------------------------------------------ //
    // Size                                                                //
    // ------------------------------------------------------------------ //

    /// Total number of nodes in the flat iteration order, including
    /// structural nodes (sections, headers, separators).
    fn size(&self) -> usize;

    /// Returns `true` when the collection contains no nodes.
    fn is_empty(&self) -> bool {
        self.size() == 0
    }

    // ------------------------------------------------------------------ //
    // Random access                                                       //
    // ------------------------------------------------------------------ //

    /// Retrieve a node by its key, or `None` if not found.
    fn get(&self, key: &Key) -> Option<&Node<T>>;

    /// Returns `true` if the collection contains an item with the given key.
    fn contains_key(&self, key: &Key) -> bool {
        self.get(key).is_some()
    }

    /// Retrieve a node by its flat index, or `None` if out of range.
    fn get_by_index(&self, index: usize) -> Option<&Node<T>>;

    // ------------------------------------------------------------------ //
    // Boundary navigation                                                 //
    // ------------------------------------------------------------------ //

    /// The key of the first focusable item in the collection, or `None`
    /// if the collection is empty or contains only structural nodes.
    ///
    /// **Key Reference Lifetime.**
    /// The returned `&Key` reference is valid only for the duration of the
    /// borrow on `&self`. If the collection is mutated (items added, removed,
    /// or reordered), previously returned `&Key` references are invalidated
    /// by Rust's borrow checker (the mutable borrow required for mutation
    /// conflicts with the outstanding shared borrow).
    ///
    /// **Callers that need to store keys across mutations MUST clone:**
    /// ```rust,ignore
    /// let key: Key = collection.first_key().cloned().expect("collection is non-empty");
    /// // `key` is now owned — safe across mutations
    /// ```
    ///
    /// **Validation after mutation:** To check if a previously-stored key
    /// is still valid after a mutation, use `collection.get(&key).is_some()`.
    fn first_key(&self) -> Option<&Key>;

    /// The key of the last focusable item in the collection.
    /// Same lifetime semantics as `first_key()` — see note above.
    fn last_key(&self) -> Option<&Key>;

    // ------------------------------------------------------------------ //
    // Sequential navigation                                               //
    // ------------------------------------------------------------------ //

    /// The key of the next focusable item after `key`, wrapping to
    /// `first_key()` when `key` is the last item.
    ///
    /// Returns `None` only when the collection has no focusable items.
    fn key_after(&self, key: &Key) -> Option<&Key>;

    /// The key of the previous focusable item before `key`, wrapping to
    /// `last_key()` when `key` is the first item.
    ///
    /// Returns `None` only when the collection has no focusable items.
    fn key_before(&self, key: &Key) -> Option<&Key>;

    /// The key of the next focusable item after `key` without wrapping.
    /// Returns `None` when `key` is the last item.
    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key>;

    /// The key of the previous focusable item before `key` without wrapping.
    /// Returns `None` when `key` is the first item.
    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key>;

    // ------------------------------------------------------------------ //
    // Iteration                                                           //
    // ------------------------------------------------------------------ //

    // NOTE: The methods below use explicit lifetime parameters with `T: 'a`
    // bounds. The spec omits these, but Rust 2024 edition's RPITIT lifetime
    // capture rules require them: the opaque return type captures `T`, so
    // the compiler must know `T` outlives the borrow.

    /// An iterator over all node keys in flat iteration order.
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a;

    /// An iterator over all nodes in flat iteration order.
    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a;

    /// An iterator over only focusable item keys, skipping structural nodes.
    fn item_keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.nodes().filter(|n| n.is_focusable()).map(|n| &n.key)
    }

    // ------------------------------------------------------------------ //
    // Children (Sections / Trees)                                        //
    // ------------------------------------------------------------------ //

    /// Returns an iterator over the direct children of a given parent key.
    /// For flat collections this always returns an empty iterator.
    fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<T>>
    where
        T: 'a;

    // ------------------------------------------------------------------ //
    // Text value access                                                   //
    // ------------------------------------------------------------------ //

    /// The plain-text representation of the item with the given key.
    /// Used by type-ahead matching. Returns `None` if the key is unknown
    /// or the node is structural.
    fn text_value_of<'a>(&'a self, key: &Key) -> Option<&'a str>
    where
        T: 'a,
    {
        self.get(key).map(|n| n.text_value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test struct implementing `CollectionItem` with custom text.
    struct LabeledItem {
        id: Key,
        label: &'static str,
    }

    impl CollectionItem for LabeledItem {
        fn key(&self) -> &Key {
            &self.id
        }

        fn text_value(&self) -> &str {
            self.label
        }
    }

    /// Test struct using the default `text_value` implementation.
    struct BareItem {
        id: Key,
    }

    impl CollectionItem for BareItem {
        fn key(&self) -> &Key {
            &self.id
        }
    }

    #[test]
    fn collection_item_key() {
        let item = LabeledItem {
            id: Key::int(1),
            label: "apple",
        };
        assert_eq!(item.key(), &Key::int(1));
    }

    #[test]
    fn collection_item_text_value_custom() {
        let item = LabeledItem {
            id: Key::int(1),
            label: "apple",
        };
        assert_eq!(item.text_value(), "apple");
    }

    #[test]
    fn collection_item_text_value_default() {
        let item = BareItem { id: Key::int(1) };
        assert_eq!(item.text_value(), "");
    }
}
