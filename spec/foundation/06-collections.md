# Collections and Selection Specification

> Cross-references: [01-architecture.md](./01-architecture.md) for crate structure, `Machine` trait, `Bindable`, and `AttrMap`; [04-internationalization.md](./04-internationalization.md) for locale-aware collation used in sort comparators.

The `ars-collections` crate lives at layer three of the dependency graph, above `ars-core`, `ars-a11y`, and `ars-i18n`, and below `ars-forms`, `ars-dom`, and the framework adapters. It provides the unified data layer consumed by every component that renders a list of items: Listbox, Select, Menu, Combobox, Autocomplete, Table, TreeView, Tabs, TagGroup, and GridList.

```text
                ars-core (no_std)
                  /      |      \
                 /       |       \
           ars-a11y  ars-i18n  ars-interactions
                \       |      /
                 \      |     /
                ars-collections
                        |
                    ars-forms
                        |
                    ars-dom
                    /       \
            ars-leptos    ars-dioxus
```

---

## 1. Collection Trait

### 1.1 Collection Ownership Model

`Collection<T>` takes **owned data** — it does not use interior mutability (`RefCell`, `Cell`) internally. This makes collections safe to share across closures and effects without borrow-checker conflicts.

**Reactivity pattern:** To make a collection reactive, wrap it in a signal rather than adding interior mutability to the collection itself:

```rust
// CORRECT: RwSignal wraps the entire collection
let items = RwSignal::new(StaticCollection::new([item1, item2]));

// WRONG: Do not use Signal<Rc<RefCell<Collection>>>
// let items = RwSignal::new(Rc::new(RefCell::new(StaticCollection::new(...))));
```

**Async loading safe pattern:** For collections populated asynchronously, use `Signal<Option<Collection<T>>>` to represent the loading state, or use `AsyncCollection<T>` (§5) which manages loading state internally:

```rust
// Safe async loading pattern
let items: RwSignal<Option<ListCollection<MyItem>>> = RwSignal::new(None);

// After async load completes:
items.set(Some(StaticCollection::new(loaded_data)));
```

When the signal updates, the framework adapter triggers a re-render with the new collection. The old collection is dropped when no longer referenced.

**Iteration safety:** `Collection<T>` uses exterior mutability only. Internal data must not be wrapped in `RefCell` or similar interior mutability primitives. All mutations go through the state machine via events (`UpdateItems`, etc.). This ensures iterators are never invalidated by concurrent modification. Adapters must not expose mutable references to collection internals.

### 1.2 Purpose

The `Collection<T>` trait is the single abstraction that all list-rendering components operate against. Instead of each component accepting `Vec<T>` and re-implementing traversal, they receive any type that implements `Collection<T>`. This enables:

- **Static collections** built once from a `Vec`.
- **Dynamic/reactive collections** that diff efficiently on change.
- **Sectioned collections** with groups, headers, and separators.
- **Tree collections** with arbitrary depth, exposing a flattened iteration order.
- **Async/paginated collections** that load on demand.
- **Virtualized collections** that only materialise visible items.

> **Design intent**: `Collection<T>` is designed for **in-memory datasets** that are fully materialized. For server-driven pagination and infinite scrolling, use `AsyncCollection<T>` (§5), which wraps a `Collection<T>` with cursor-based pagination metadata and loading state. The core `Collection<T>` trait is stable; async/paginated support builds on top without breaking changes.

#### 1.2.1 IntersectionObserver for Lazy Rendering & Virtual Scroll

Collections supporting lazy rendering or virtual scrolling MUST use `IntersectionObserver` to determine element visibility and trigger data loading.

**Configuration**:

Browser-equivalent JavaScript (for reference only — Rust implementation uses `web_sys::IntersectionObserver`):

```text
let observer = new IntersectionObserver(callback, {
  root: scroll_container, // null for viewport
  rootMargin: "200px 0px", // preload buffer (200px above/below viewport)
  threshold: [0, 0.5, 1.0], // visibility thresholds
});
```

**Sentinel Element Pattern**: For paginated or infinite-scroll collections, place a sentinel `<div>` element after the last rendered item. When the sentinel enters the root margin, trigger the next page load:

1. Observer detects sentinel at threshold 0 (entering margin)
2. Emit `Event::LoadNextPage` or call `AsyncCollection::load_more()`
3. After new items render, sentinel moves to new end position
4. Observer automatically tracks the repositioned sentinel

**Pagination Trigger Debounce**: To prevent redundant page loads during rapid scrolling, the observer callback MUST debounce page-load requests with a minimum 100ms interval between triggers.

**Virtual Scroll Integration**: For virtualized lists, the observer tracks which item slots are visible. Items outside the observed range are replaced with spacer elements of estimated height. The observer callback updates the `visible_range: Range<usize>` in the collection's rendering context.

**Cleanup**: Observer MUST be disconnected on component unmount via `observer.disconnect()`. Sentinel elements MUST be unobserved before removal from DOM.

The trait itself is pure Rust — no `web_sys`, no framework imports — and is `no_std` compatible when the `alloc` feature is active (all `Vec` and `String` usage comes from `alloc`).

### 1.3 Key Type

Every node in a collection is identified by a `Key`. The key must be cheaply cloneable, hashable, and orderable for use in `BTreeSet`-backed selection state.

```rust
// ars-collections/src/key.rs

use alloc::string::String;

/// The identifier for a node within a collection.
///
/// Keys are stable across re-renders. Framework adapters commonly derive
/// keys from item index (for static slices), from a database primary key
/// (for server data), or from a user-supplied `id` prop.
///
/// The `String` variant covers most real-world use-cases including numeric
/// IDs rendered as strings. The `Int` variant is a zero-allocation fast
/// path for purely numeric identifiers (e.g., row IDs from a `u64` database
/// primary key). The `Uuid` variant (requires the `uuid` feature) provides
/// a zero-allocation path for UUID-based identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// String key — the universal fallback.
    String(String),

    /// Integer key — allocation-free for numeric identifiers.
    Int(u64),

    /// UUID key — allocation-free for UUID-based identifiers.
    ///
    /// Available only when the `uuid` feature is enabled. Provides a
    /// 16-byte `Copy` key without heap allocation, compared to the 36-byte
    /// `String` representation of a UUID.
    #[cfg(feature = "uuid")]
    Uuid(uuid::Uuid),
}

/// Manual `PartialOrd` / `Ord` implementation: `Int` keys sort before `String`
/// keys so that numeric identifiers (common in database-driven collections)
/// cluster together at the front of `BTreeSet<Key>` used by `selection::State`.
/// Within each variant the natural ordering applies (`u64::cmp` for `Int`,
/// lexicographic for `String`).
///
/// When the `uuid` feature is enabled, the ordering is `Int < Uuid < String`:
/// numeric IDs first, then UUIDs (also structured identifiers), then
/// arbitrary strings.
///
/// **Note on mixed-key ordering**: When a collection contains
/// both `Key::Int` and `Key::String` keys, all `Int` keys sort before all
/// `String` keys. This is intentional for database-backed collections where
/// numeric IDs and string IDs are not intermixed. If your use case requires a
/// single unified ordering, normalize all keys to `Key::String` (e.g., via
/// `Key::str(id.to_string())`). For database-sourced numeric IDs, prefer
/// `Key::from_database_id(u64)` (alias for `Key::Int`) to make the sort
/// behavior explicit.
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Variant ordering: Int (0) < Uuid (1) < String (2).
        // Within the same variant: natural ordering.
        match (self, other) {
            (Key::Int(a), Key::Int(b)) => a.cmp(b),
            (Key::String(a), Key::String(b)) => a.cmp(b),
            (Key::Int(_), Key::String(_)) => core::cmp::Ordering::Less,
            (Key::String(_), Key::Int(_)) => core::cmp::Ordering::Greater,
            #[cfg(feature = "uuid")]
            (Key::Uuid(a), Key::Uuid(b)) => a.cmp(b),
            #[cfg(feature = "uuid")]
            (Key::Int(_), Key::Uuid(_)) => core::cmp::Ordering::Less,
            #[cfg(feature = "uuid")]
            (Key::Uuid(_), Key::Int(_)) => core::cmp::Ordering::Greater,
            #[cfg(feature = "uuid")]
            (Key::Uuid(_), Key::String(_)) => core::cmp::Ordering::Less,
            #[cfg(feature = "uuid")]
            (Key::String(_), Key::Uuid(_)) => core::cmp::Ordering::Greater,
        }
    }
}

impl Key {
    /// Construct a string key.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Self {
        Key::String(s.into())
    }

    /// Construct an integer key.
    #[must_use]
    pub const fn int(n: u64) -> Self {
        Key::Int(n)
    }

    /// Construct a key from a database numeric ID.
    ///
    /// Alias for `Key::Int`. Exists to make the ordering behavior explicit:
    /// database ID keys sort before string keys in `BTreeSet<Key>`.
    #[must_use]
    pub const fn from_database_id(n: u64) -> Self {
        Key::Int(n)
    }

    /// Construct a UUID key.
    ///
    /// Available only when the `uuid` feature is enabled. Provides a
    /// zero-allocation key for UUID-based identifiers.
    #[cfg(feature = "uuid")]
    #[must_use]
    pub const fn uuid(id: uuid::Uuid) -> Self {
        Key::Uuid(id)
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self { Key::String(s.into()) }
}

impl From<String> for Key {
    fn from(s: String) -> Self { Key::String(s) }
}

impl From<u64> for Key {
    fn from(n: u64) -> Self { Key::Int(n) }
}

impl From<u32> for Key {
    fn from(n: u32) -> Self { Key::Int(u64::from(n)) }
}

impl From<usize> for Key {
    fn from(n: usize) -> Self { Key::Int(n as u64) }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for Key {
    fn from(id: uuid::Uuid) -> Self { Key::Uuid(id) }
}

impl Default for Key {
    fn default() -> Self { Key::Int(0) }
}

impl core::fmt::Display for Key {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Key::String(s) => f.write_str(s),
            Key::Int(n)    => write!(f, "{n}"),
            #[cfg(feature = "uuid")]
            Key::Uuid(id)  => write!(f, "{id}"),
        }
    }
}
```

**Key parsing:** `Key::parse(s: &str)` attempts integer parsing first; if it fails, returns `Key::String(s.to_owned())`. To avoid ambiguity, prefer explicit constructors: `Key::Int(42)` or `Key::String("42abc".into())`. Display implementation: `Int` variants format as the number, `String` variants format as the string value, `Uuid` variants format as the standard hyphenated UUID string.

### 1.4 NodeType

```rust
// ars-collections/src/node.rs

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
```

### 1.5 Node Struct

```rust
// ars-collections/src/node.rs

use alloc::{string::String, vec::Vec};
use crate::key::Key;

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
    pub const fn is_structural(&self) -> bool {
        !matches!(self.node_type, NodeType::Item)
    }

    /// Construct an `Item` node from a `CollectionItem` value.
    #[must_use]
    pub fn item(key: Key, index: usize, item: T) -> Self
    where
        T: CollectionItem,
    {
        let text_value = item.text_value().to_string();
        Self {
            key,
            node_type: NodeType::Item,
            value: Some(item),
            text_value,
            level: 0,
            has_children: false,
            is_expanded: None,
            parent_key: None,
            index,
        }
    }
}

/// Structural equality: compares identity fields (`key`, `node_type`,
/// `text_value`) without requiring `T: PartialEq`. Available on all
/// `Node<T>` regardless of `T` bounds.
///
/// **Design Note**: `Node<T>` does NOT implement `PartialEq` as a blanket
/// impl. Instead, `PartialEq` is implemented only when `T: PartialEq`,
/// and `Eq` is implemented only when `T: Eq`. This preserves composability
/// (allowing `Node<T>` in `BTreeSet`/`HashSet` when `T` supports it) and
/// avoids violating `Eq`'s substitutability requirement.
///
/// For code that needs key/type-only comparison without a `T: PartialEq`
/// bound (e.g., the diffing algorithm matching nodes by identity), use
/// `structural_eq()`.
impl<T> Node<T> {
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
```

#### 1.5.1 Node Equality Semantics

`Node<T>` intentionally does NOT derive `PartialEq`. The auto-derived equality would compare only structural fields (`key`, `children`, metadata) but exclude `value: T`, leading to confusing semantics.

`Node<T>` provides two comparison methods:

- **`structural_eq(&self, other: &Self) -> bool`**: Available on all `Node<T>` (no `T` bounds). Compares `key`, `node_type`, and `text_value` only. Used by the diffing algorithm to detect structural changes (additions, removals, reordering) without requiring `T: PartialEq`.
- **`PartialEq` (via `==`)** where `T: PartialEq`: Compares all fields including `value`. This is the standard equality operator and includes full value comparison.
- **`Eq`** where `T: Eq`: Implemented because `PartialEq` compares all fields, satisfying `Eq`'s substitutability requirement. Enables `Node<T>` in `BTreeSet` and `HashSet` when `T: Eq`.

Consumers should use `==` for full value-aware comparisons and `structural_eq()` for layout-only diffing where `T: PartialEq` is unavailable or unnecessary.

### 1.6 Collection Trait

````rust
// ars-collections/src/collection.rs

use crate::{key::Key, node::Node};

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
/// Trait for items stored in collections. Provides the key and
/// optional text value used for typeahead matching and collation.
pub trait CollectionItem {
    /// The unique key identifying this item within a collection.
    fn key(&self) -> &Key;

    /// Human-readable text for typeahead matching and collation sorting.
    /// Returns `""` by default for items without meaningful text content.
    fn text_value(&self) -> &str { "" }
}

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
    /// ```rust
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

    // NOTE: Methods below use explicit lifetime parameters with `T: 'a`
    // bounds. Rust 2024 edition's RPITIT lifetime capture rules require
    // them: the opaque return type captures `T`, so the compiler must
    // know `T` outlives the borrow.

    /// An iterator over all node keys in flat iteration order.
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key> where T: 'a;

    /// An iterator over all nodes in flat iteration order.
    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>> where T: 'a;

    /// An iterator over only focusable item keys, skipping structural nodes.
    fn item_keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
    where
        T: 'a,
    {
        self.nodes()
            .filter(|n| n.is_focusable())
            .map(|n| &n.key)
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
````

#### 1.6.1 Collection Change Announcements

When a collection's contents change dynamically (items added, removed, filtered, or sorted), the change MUST be announced to screen readers via a live region. Components that mutate collections at runtime — including Combobox, Autocomplete, TagsInput, and Table — MUST emit structured announcements.

```rust
/// Describes a change to a collection for screen reader announcement.
pub enum CollectionChangeAnnouncement {
    /// Items were added. Message via `CollectionMessages::items_added`.
    ItemsAdded { count: usize },
    /// Items were removed. Message via `CollectionMessages::items_removed`.
    ItemsRemoved { count: usize },
    /// Collection was filtered. Message via `CollectionMessages::filtered`.
    Filtered { matching_count: usize },
    /// Collection was sorted. Message via `CollectionMessages::sorted`.
    /// `SortDirection` is defined in §7.2 (`SortedCollection`).
    Sorted { column: String, direction: SortDirection },
    /// Collection is empty after operation. Message via `CollectionMessages::empty`.
    Empty,
    /// Async load completed. Message via `CollectionMessages::loaded`.
    Loaded { count: usize },
}

/// Localizable message functions for collection change announcements.
/// Uses CLDR plural rules via `plural_category(count, locale)` (from ars-i18n §4.3)
/// for count-dependent messages.
///
/// Default English messages are provided. Override via Props to localize.
// Uses `MessageFn` and `Locale` from ars-i18n (04-internationalization.md §7.1).
use ars_i18n::{Locale, MessageFn};

/// All closure fields use `MessageFn::new()` which delegates to the cfg-gated `From` impls
/// defined in `04-internationalization.md` §7.1 — `Rc` on WASM, `Arc` on native.
/// Trait objects include `+ Send + Sync` on all targets (see design note in §7.1).

pub struct CollectionMessages {
    /// Message for items added. Receives (count, locale) for plural-aware formatting.
    /// Default (en): "1 item added" / "{count} items added"
    pub items_added: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Message for items removed. Receives (count, locale).
    /// Default (en): "1 item removed" / "{count} items removed"
    pub items_removed: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Message for filtered results. Receives (count, locale).
    /// Default (en): "No results found" / "1 result available" / "{count} results available"
    pub filtered: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Message for sorted collection. Receives (column, direction, locale).
    /// `SortDirection` is defined in this crate's `sorted_collection` module (§7).
    /// Default (en): "Sorted by {column}, {direction}"
    pub sorted: MessageFn<dyn Fn(&str, SortDirection, &Locale) -> String + Send + Sync>,
    /// Message for empty collection. Receives locale for localization.
    /// Default (en): "No items"
    pub empty: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Message for async load completion. Receives (count, locale).
    /// Default (en): "1 item loaded" / "{count} items loaded"
    pub loaded: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for CollectionMessages {
    fn default() -> Self {
        Self {
            items_added: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 { "1 item added".into() }
                else { format!("{count} items added") }
            }),
            items_removed: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 { "1 item removed".into() }
                else { format!("{count} items removed") }
            }),
            filtered: MessageFn::new(|count: usize, _locale: &Locale| match count {
                0 => "No results found".into(),
                1 => "1 result available".into(),
                n => format!("{n} results available"),
            }),
            // Note: `SortDirection` is defined in §7.2 and imported at the crate level.
            sorted: MessageFn::new(|col: &str, dir: SortDirection, _locale: &Locale| {
                format!("Sorted by {col}, {dir}")
            }),
            empty: MessageFn::new(|_locale: &Locale| "No items".into()),
            loaded: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 { "1 item loaded".into() }
                else { format!("{count} items loaded") }
            }),
        }
    }
}

impl core::fmt::Debug for CollectionMessages {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CollectionMessages {{ .. }}")
    }
}
```

**Adapter Requirements**:

- Maintain a dedicated `aria-live="polite"` region per collection component
- Render the announcement text into the live region after the DOM update completes
- **Debounce**: Coalesce rapid successive changes (e.g., keystroke-driven filtering) with a **minimum 150ms debounce** before announcing. Only the final state is announced. The 150ms floor ensures that intermediate states during rapid updates (e.g., collection going from 100 to 10 to 5 items within 300ms) do not produce multiple announcements.
- **Identical announcement suppression**: If the computed announcement text is identical to the most recently announced text, suppress the duplicate. This prevents redundant announcements when a filter keystroke produces the same result count (e.g., typing "ab" and "abc" both yield "5 results available").
- **Concurrent operation batching**: When multiple collection operations occur within the same debounce window (e.g., a filter followed immediately by a sort), batch them into a single announcement describing the final state. Do not announce intermediate states. For example, a filter reducing to 5 items followed by a sort within the same 150ms window produces one announcement: `"5 results available, sorted by name, ascending"` rather than two separate announcements.
- For Combobox/Autocomplete: announce `Filtered` result count after each filter operation (post-debounce): e.g., `"5 results available"` or `"No results found"`
- For TagsInput: announce `ItemsRemoved` when a tag is deleted, `ItemsAdded` when a tag is added

**Timing**: Announcements fire AFTER the DOM has been updated with new items, not before. This ensures screen readers can navigate to the announced items immediately after hearing the announcement.

> **SortedCollection.** A collection variant that maintains items in a
> user-defined sort order. Wraps a `StaticCollection<T>` with a comparator function.
> Re-sorts on item add/update. Used by Table column sorting and any sorted list view.
> See §7.2.
>
> **MutableListData / MutableTreeData.** Mutable collection types that
> support dynamic add, remove, reorder, and move operations. Emit granular change events
> for efficient DOM reconciliation. See §1.8.

### 1.7 Collection Trait Adoption Requirements

All list-rendering components **MUST** use `Collection<T>` (or a concrete implementation such as `StaticCollection<T>`, `TreeCollection<T>`, `FilteredCollection<T>`) for their item data. This ensures consistent typeahead, disabled-aware navigation, and keyboard interaction patterns across the library.

**Required adopters (use `StaticCollection<T>` or `TreeCollection<T>`):**

| Component      | Collection Type                    | Current Status                                                                              |
| -------------- | ---------------------------------- | ------------------------------------------------------------------------------------------- |
| `Select`       | `StaticCollection<select::Item>`   | Adopted                                                                                     |
| `Combobox`     | `StaticCollection<combobox::Item>` | Adopted                                                                                     |
| `Listbox`      | `StaticCollection<listbox::Item>`  | Adopted                                                                                     |
| `Menu`         | `StaticCollection<menu::Item>`     | Adopted                                                                                     |
| `MenuBar`      | `StaticCollection<menu_bar::Menu>` | Adopted                                                                                     |
| `Autocomplete` | via `FilteredCollection<T>`        | Adopted                                                                                     |
| `Table`        | `StaticCollection<row::Item>`      | Adopted — Table spec defines `StaticCollection<row::Item>` with column-aware key extraction |
| `TreeView`     | `TreeCollection<TreeItem>`         | **Adopted** — Props accepts `TreeCollection<TreeItem>`, navigation uses collection API      |
| `GridList`     | `StaticCollection<GridItemDef>`    | Adopted                                                                                     |
| `TagGroup`     | `StaticCollection<TagItemDef>`     | Adopted                                                                                     |

**Exempt from Collection trait** (items are not list-rendered or have fixed-size enumerable sets):

| Component       | Reason                                                                                   |
| --------------- | ---------------------------------------------------------------------------------------- |
| `Tabs`          | Items are panel IDs registered dynamically; DOM-order discovery, not data-driven         |
| `Accordion`     | Same as Tabs — items self-register at mount                                              |
| `TagsInput`     | Tags are user-typed strings with insertion-order semantics; `Vec<String>` is appropriate |
| `CheckboxGroup` | Child checkboxes register themselves; not a list-rendering component                     |

All list-rendering components have been migrated to the `Collection` trait. See §3 for `selection::Set` integration requirements.

### 1.8 Mutable Collections

Mutable collection wrappers support dynamic add, remove, reorder, and move operations. Each mutation emits a `CollectionChange` event for efficient DOM reconciliation by the adapter layer.

#### 1.8.1 Change Events

```rust
// ars-collections/src/mutable.rs

/// A granular change event emitted when a mutable collection is modified.
/// Adapters use these to perform targeted DOM updates instead of full re-renders.
#[derive(Clone, Debug, PartialEq)]
pub enum CollectionChange<K: Clone> {
    /// New items inserted at the given index.
    Insert { index: usize, count: usize },
    /// Items with the given keys were removed.
    Remove { keys: Vec<K> },
    /// An item moved from one index to another.
    Move { key: K, from_index: usize, to_index: usize },
    /// An item's data was replaced in-place (key unchanged).
    Replace { key: K },
    /// The entire collection was reset (e.g., bulk replacement).
    /// Adapters should re-render all items.
    Reset,
}
```

#### 1.8.2 MutableListData

`MutableListData<T>` wraps a `StaticCollection<T>` with mutation methods. It maintains an internal change log that the adapter drains after each update cycle.

```rust
/// A mutable flat-list collection that tracks granular changes.
pub struct MutableListData<T: CollectionItem> {
    inner: StaticCollection<T>,
    pending_changes: Vec<CollectionChange<Key>>,
}

impl<T: CollectionItem> MutableListData<T> {
    /// Create from an existing static collection.
    pub fn new(collection: StaticCollection<T>) -> Self {
        Self { inner: collection, pending_changes: Vec::new() }
    }

    /// Append an item to the end of the collection.
    pub fn push(&mut self, item: T) {
        let index = self.inner.len();
        self.inner.insert(index, item);
        self.pending_changes.push(CollectionChange::Insert { index, count: 1 });
    }

    /// Insert an item at the given index.
    pub fn insert(&mut self, index: usize, item: T) {
        self.inner.insert(index, item);
        self.pending_changes.push(CollectionChange::Insert { index, count: 1 });
    }

    /// Remove items by key. Returns the removed items.
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        let removed = self.inner.remove_by_keys(keys);
        self.pending_changes.push(CollectionChange::Remove { keys: keys.to_vec() });
        removed
    }

    /// Move an item from one index to another.
    pub fn move_item(&mut self, key: &Key, to_index: usize) {
        if let Some(from_index) = self.inner.index_of(key) {
            self.inner.move_item(from_index, to_index);
            self.pending_changes.push(CollectionChange::Move {
                key: key.clone(),
                from_index,
                to_index,
            });
        }
    }

    /// Replace an item's data in-place (key must match).
    pub fn replace(&mut self, item: T) {
        let key = item.key().clone();
        self.inner.replace(item);
        self.pending_changes.push(CollectionChange::Replace { key });
    }

    /// Remove all items.
    pub fn clear(&mut self) {
        self.inner.clear();
        self.pending_changes.push(CollectionChange::Reset);
    }

    /// Drain pending changes. Called by the adapter after processing.
    pub fn drain_changes(&mut self) -> Vec<CollectionChange<Key>> {
        core::mem::take(&mut self.pending_changes)
    }
}

impl<T: CollectionItem> Collection<T> for MutableListData<T> {
    // Delegates all Collection trait methods to self.inner (StaticCollection<T>).
    fn size(&self) -> usize { self.inner.size() }
    fn get(&self, key: &Key) -> Option<&Node<T>> { self.inner.get(key) }
    fn get_by_index(&self, index: usize) -> Option<&Node<T>> { self.inner.get_by_index(index) }
    fn first_key(&self) -> Option<&Key> { self.inner.first_key() }
    fn last_key(&self) -> Option<&Key> { self.inner.last_key() }
    fn key_after(&self, key: &Key) -> Option<&Key> { self.inner.key_after(key) }
    fn key_before(&self, key: &Key) -> Option<&Key> { self.inner.key_before(key) }
    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_after_no_wrap(key) }
    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_before_no_wrap(key) }
    fn keys(&self) -> impl Iterator<Item = &Key> { self.inner.keys() }
    fn nodes(&self) -> impl Iterator<Item = &Node<T>> { self.inner.nodes() }
    fn children_of(&self, parent_key: &Key) -> impl Iterator<Item = &Node<T>> { self.inner.children_of(parent_key) }
}
```

#### 1.8.3 MutableTreeData

`MutableTreeData<T>` wraps a `TreeCollection<T>` with tree-specific mutations including reparenting.

```rust
/// A mutable tree collection that tracks granular changes.
pub struct MutableTreeData<T: CollectionItem> {
    inner: TreeCollection<T>,
    pending_changes: Vec<CollectionChange<Key>>,
}

impl<T: CollectionItem> MutableTreeData<T> {
    /// Create from an existing tree collection.
    pub fn new(collection: TreeCollection<T>) -> Self {
        Self { inner: collection, pending_changes: Vec::new() }
    }

    /// Insert a child under the given parent at the specified index.
    /// Use `parent: None` for root-level insertion.
    pub fn insert_child(&mut self, parent: Option<&Key>, index: usize, item: T) {
        let flat_index = self.inner.insert_child(parent, index, item);
        self.pending_changes.push(CollectionChange::Insert { index: flat_index, count: 1 });
    }

    /// Remove a node and all its descendants.
    pub fn remove(&mut self, keys: &[Key]) -> Vec<T> {
        let removed = self.inner.remove_by_keys(keys);
        self.pending_changes.push(CollectionChange::Remove { keys: keys.to_vec() });
        removed
    }

    /// Move a node to a new parent (reparent). The node keeps its subtree.
    pub fn reparent(&mut self, key: &Key, new_parent: Option<&Key>, index: usize) {
        let from_index = self.inner.flat_index_of(key).expect("key must exist");
        self.inner.reparent(key, new_parent, index);
        let to_index = self.inner.flat_index_of(key).expect("key must exist after reparent");
        self.pending_changes.push(CollectionChange::Move {
            key: key.clone(),
            from_index,
            to_index,
        });
    }

    /// Reorder a node among its siblings (same parent).
    pub fn reorder(&mut self, key: &Key, to_sibling_index: usize) {
        let from_index = self.inner.flat_index_of(key).expect("key must exist");
        self.inner.reorder_sibling(key, to_sibling_index);
        let to_index = self.inner.flat_index_of(key).expect("key must exist after reorder");
        self.pending_changes.push(CollectionChange::Move {
            key: key.clone(),
            from_index,
            to_index,
        });
    }

    /// Replace a node's data in-place (key must match, children preserved).
    pub fn replace(&mut self, item: T) {
        let key = item.key().clone();
        self.inner.replace(item);
        self.pending_changes.push(CollectionChange::Replace { key });
    }

    /// Drain pending changes.
    pub fn drain_changes(&mut self) -> Vec<CollectionChange<Key>> {
        core::mem::take(&mut self.pending_changes)
    }
}

impl<T: CollectionItem> Collection<T> for MutableTreeData<T> {
    // Delegates all Collection trait methods to self.inner (TreeCollection<T>).
    fn size(&self) -> usize { self.inner.size() }
    fn get(&self, key: &Key) -> Option<&Node<T>> { self.inner.get(key) }
    fn get_by_index(&self, index: usize) -> Option<&Node<T>> { self.inner.get_by_index(index) }
    fn first_key(&self) -> Option<&Key> { self.inner.first_key() }
    fn last_key(&self) -> Option<&Key> { self.inner.last_key() }
    fn key_after(&self, key: &Key) -> Option<&Key> { self.inner.key_after(key) }
    fn key_before(&self, key: &Key) -> Option<&Key> { self.inner.key_before(key) }
    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_after_no_wrap(key) }
    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_before_no_wrap(key) }
    fn keys(&self) -> impl Iterator<Item = &Key> { self.inner.keys() }
    fn nodes(&self) -> impl Iterator<Item = &Node<T>> { self.inner.nodes() }
    fn children_of(&self, parent_key: &Key) -> impl Iterator<Item = &Node<T>> { self.inner.children_of(parent_key) }
}
```

#### 1.8.4 DnD Integration

`CollectionDndEvent` (§10) maps directly to mutable collection operations:

| DnD Event                            | MutableListData                   | MutableTreeData  |
| ------------------------------------ | --------------------------------- | ---------------- |
| `Reorder { keys, target, position }` | `move_item()`                     | `reorder()`      |
| `Move { keys, target, position }`    | `insert()` (+ remove from source) | `reparent()`     |
| `Insert { items, target, position }` | `insert()`                        | `insert_child()` |

The adapter handles the event by calling the appropriate mutation method, then drains changes via `drain_changes()` to perform targeted DOM updates.

---

## 2. Collection Building

### 2.1 CollectionBuilder

The `CollectionBuilder` is the primary way to construct a static collection from an iterator or an explicit item list. It is generic over `T` and accepts items via a fluent API. The builder enforces that keys are unique; duplicate keys cause a panic in debug builds and silently overwrite in release.

````rust
// ars-collections/src/builder.rs

use alloc::{string::String, vec::Vec};
use indexmap::IndexMap;
use crate::{key::Key, node::{Node, NodeType}, StaticCollection};

/// Builds a [`StaticCollection`] from items added imperatively or from an
/// iterator.
///
/// # Example
///
/// ```rust
/// let collection = CollectionBuilder::new()
///     .item(Key::int(1), "Apple",  apple_data)
///     .item(Key::int(2), "Banana", banana_data)
///     .item(Key::int(3), "Cherry", cherry_data)
///     .build();
/// ```
pub struct CollectionBuilder<T> {
    nodes: Vec<Node<T>>,
    key_to_index: IndexMap<Key, usize>,
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
    pub fn item(
        mut self,
        key: impl Into<Key>,
        text_value: impl Into<String>,
        value: T,
    ) -> Self {
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
    pub fn section(
        mut self,
        key: impl Into<Key>,
        header_text: impl Into<String>,
    ) -> Self {
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
    fn default() -> Self { Self::new() }
}

/// Manual `Debug` avoids requiring `T: Debug`. Prints item count only.
impl<T> core::fmt::Debug for CollectionBuilder<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CollectionBuilder")
            .field("items", &self.nodes.len())
            .finish()
    }
}
````

#### 2.1.1 Trait Bound Propagation Strategy

Trait bounds on `T` are layered by access level:

- **Read-only access** (`Collection<T>` trait): has no bounds on `T`. Components that only iterate or display items should bound on `C: Collection<T>` without requiring `T: Clone`.
- **Mutation access** (`CollectionBuilder` layer): requires `T: Clone`. Clone is needed because mutation operations (insert, update) may need to copy items for diffing or rollback. `T: PartialEq` is additionally needed only for collection equality comparison (`StaticCollection`'s `PartialEq` impl), not for building or mutation.
- **Keyed access** (`KeyedCollection<T, K>` trait): additionally requires `K: Ord + Clone`.

This layering ensures that consumers pay only for the trait bounds their access pattern requires. Components consuming collections in read-only mode should bound on `C: Collection<T>` to maximize flexibility for callers.

### 2.2 StaticCollection

```rust
// ars-collections/src/static_collection.rs

use alloc::vec::Vec;
use indexmap::IndexMap;
use crate::{key::Key, node::{Node, NodeType}, Collection};

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
        let last_focusable  = nodes.iter().rposition(Node::is_focusable);
        Self { nodes, key_to_index, first_focusable, last_focusable }
    }

    /// Construct from a `Vec` of `(Key, text_value, T)` tuples.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = (Key, String, T)>) -> Self {
        Self::from_iter(items)
    }
}

/// Construct an empty collection.
impl<T: Clone> Default for StaticCollection<T> {
    fn default() -> Self {
        Self::new([])
    }
}

/// Construct from a `Vec` of `(Key, text_value, T)` tuples.
impl<T: Clone> From<Vec<(Key, String, T)>> for StaticCollection<T> {
    fn from(items: Vec<(Key, String, T)>) -> Self {
        Self::from_iter(items)
    }
}

/// Enables `iterator.collect::<StaticCollection<T>>()` as the idiomatic way
/// to build a collection from an iterator of `(Key, text_value, T)` tuples.
impl<T: Clone> FromIterator<(Key, String, T)> for StaticCollection<T> {
    fn from_iter<I: IntoIterator<Item = (Key, String, T)>>(iter: I) -> Self {
        let mut builder = CollectionBuilder::new();
        for (key, text, value) in iter {
            builder = builder.item(key, text, value);
        }
        builder.build()
    }
}

// Note: Clone bound on the trait impl is intentional — StaticCollection stores owned T values
// and its constructors (new, from_parts) require Clone for ergonomic initialization.
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

    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key> where T: 'a {
        self.nodes.iter().map(|n| &n.key)
    }

    fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<T>> where T: 'a {
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
            && self.nodes.iter().zip(other.nodes.iter()).all(|(a, b)| a == b)
    }
}

/// Mutation methods used by `MutableListData`. These operate on the internal
/// `IndexMap` and `Vec` and are O(1) amortized for append, O(n) for mid-list insert.
impl<T: CollectionItem> StaticCollection<T> {
    /// Number of items.
    #[must_use]
    pub fn len(&self) -> usize { self.nodes.len() }

    /// Returns `true` when the collection contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

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
```

### 2.3 Tree Collection

Tree collections represent hierarchical data (TreeView, nested menus) as a flat iteration order while tracking parent-child relationships and expansion state. The flattened order is a depth-first pre-order traversal: a parent node appears before its children.

```rust
// ars-collections/src/tree_collection.rs

use alloc::{collections::BTreeSet, string::String, vec::Vec};
use indexmap::IndexMap;
use crate::{key::Key, node::{Node, NodeType}, collection::{Collection, CollectionItem}};

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
impl<T> core::fmt::Debug for TreeItemConfig<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
impl<T> core::fmt::Debug for TreeCollection<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
                .all(|(a, b)| {
                    a == b && a.level == b.level && a.parent_key == b.parent_key
                })
    }
}

/// Maximum nesting depth for tree items. Enforced during construction to
/// guarantee WASM stack safety — the default WASM stack (64 KiB–1 MiB)
/// cannot support unbounded recursion.
const MAX_TREE_DEPTH: usize = 32;

impl<T> TreeCollection<T> {
    /// Compute which flat indices are visible given the current expansion set,
    /// along with cached first/last focusable visible indices.
    fn compute_visible(
        all_nodes: &[Node<T>],
    ) -> (Vec<usize>, Option<usize>, Option<usize>) {
        let mut visible = Vec::with_capacity(all_nodes.len());
        let mut first_focusable = None;
        let mut last_focusable = None;
        let mut skip_until_level: Option<usize> = None;

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
            insert_item(root, 0, None, &mut all_nodes, &mut key_to_index, &mut expanded_keys);
        }

        let (visible_indices, first_focusable_visible, last_focusable_visible) =
            Self::compute_visible(&all_nodes);
        Self {
            all_nodes, visible_indices, key_to_index, expanded_keys,
            first_focusable_visible, last_focusable_visible,
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
        let mut skip_until_level: Option<usize> = None;

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
        self.first_focusable_visible
            .map(|i| &self.all_nodes[i].key)
    }

    fn last_key(&self) -> Option<&Key> {
        self.last_focusable_visible
            .map(|i| &self.all_nodes[i].key)
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
                let drained: Vec<Node<T>> = self.all_nodes.drain(start..end).collect();
                for node in &drained {
                    self.expanded_keys.remove(&node.key);
                }
                for node in drained {
                    if let Some(val) = node.value { removed.push(val); }
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
        // Validate new_parent exists before extraction.
        if let Some(pk) = new_parent {
            if !self.key_to_index.contains_key(pk) {
                return;
            }
        }

        // Save old parent key before extraction for metadata cleanup.
        let old_parent_key = self.parent_of(key).cloned();

        // Extract the subtree.
        let subtree = self.extract_subtree(key);
        if subtree.is_empty() { return; }

        // Reject reparenting under a descendant of the moved node.
        // After extraction the descendant is no longer in the tree.
        if let Some(pk) = new_parent {
            if !self.key_to_index.contains_key(pk) {
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

        // Mark the new parent as having children (if it was a leaf).
        if let Some(pk) = new_parent {
            if let Some(&pi) = self.key_to_index.get(pk) {
                self.all_nodes[pi].has_children = true;
                if self.all_nodes[pi].is_expanded.is_none() {
                    self.all_nodes[pi].is_expanded = Some(false);
                }
            }
        }

        // Recompute levels relative to new parent.
        let new_level = match new_parent {
            Some(pk) => self.key_to_index.get(pk)
                .map_or(0, |&i| self.all_nodes[i].level + 1),
            None => 0,
        };
        let old_level = subtree[0].level;
        let level_delta = new_level as isize - old_level as isize;

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
        if subtree.is_empty() { return; }

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
        self.key_to_index.get(key)
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
        let children: Vec<usize> = match parent {
            Some(pk) => {
                let parent_level = self.key_to_index.get(pk)
                    .map_or(0, |&i| self.all_nodes[i].level);
                self.all_nodes.iter()
                    .enumerate()
                    .filter(|(_, n)| n.parent_key.as_ref() == Some(pk) && n.level == parent_level + 1)
                    .map(|(i, _)| i)
                    .collect()
            }
            None => self.all_nodes.iter()
                .enumerate()
                .filter(|(_, n)| n.parent_key.is_none() && n.level == 0)
                .map(|(i, _)| i)
                .collect(),
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
                match parent {
                    Some(pk) => self.key_to_index.get(pk).map_or(self.all_nodes.len(), |&i| i + 1),
                    None => self.all_nodes.len(),
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
```

#### 2.3.1 Optimized Tree Expand/Collapse

For large trees, splitting the tree structure signal from the expanded-keys signal avoids O(n) clones on every expand/collapse:

**Dioxus signal-splitting pattern:**

```rust
// Structure signal (rarely changes) + expanded-keys signal (changes on toggle)
let tree_structure = use_signal(|| build_tree(items));
let expanded_keys = use_signal(|| BTreeSet::new());

// Per-item memo — only re-renders when THIS item's visibility changes
let is_visible = use_memo(move || {
    let expanded = expanded_keys.read();
    tree_structure.read().is_visible_with_expanded(&item_key, &expanded)
});
```

**Leptos separate-signals pattern:**

```rust
let (tree_structure, _) = signal(build_tree(items));
let (expanded_keys, set_expanded_keys) = signal(BTreeSet::new());

// Derived signal per item
let is_visible = Memo::new(move |_| {
    let expanded = expanded_keys.get();
    tree_structure.get().is_visible_with_expanded(&item_key, &expanded)
});
```

---

## 3. Selection Model

### 3.1 `selection::Mode` and `selection::Behavior`

```rust
// ars-collections/src/selection.rs

/// Whether and how many items can be selected simultaneously.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// No items can be selected. Focus still moves through the list.
    #[default]
    None,

    /// Exactly one item can be selected at a time. Selecting a new item
    /// deselects the previous one unless `selection::Behavior::Toggle` is
    /// used.
    Single,

    /// Any number of items may be selected independently.
    Multiple,
}

// `Behavior` — defined in `shared/selection-patterns.md`
```

> **Action vs Selection**: In `Replace` mode, item activation (Enter key, double-click,
> or single tap on touch) should trigger an `Action(Key)` callback distinct from
> `on_selection_change`. Components should expose an `on_action` callback for this purpose.
> On touch devices, tap triggers the action while long-press enters selection mode.

```rust

/// Callback for item action (Enter, double-click, tap in Replace mode).
/// Distinct from selection change — action activates the item.
/// Uses the same cfg-gated pattern as `Callback<T>`: `Rc` on WASM, `Arc` on native,
/// ensuring cross-platform safety for multi-threaded native runtimes.
#[cfg(target_arch = "wasm32")]
pub type OnAction = Option<Rc<dyn Fn(Key)>>;

#[cfg(not(target_arch = "wasm32"))]
pub type OnAction = Option<Arc<dyn Fn(Key) + Send + Sync>>;
```

> **Capture semantics**: OnAction callbacks are invoked synchronously during event processing.
> They must not mutate the collection directly; instead, dispatch a separate update signal.

### 3.2 `selection::Set`

The internal representation of the selected keys. A special `All` variant supports "select all" semantics on collections whose total size may be unknown at selection time (e.g., server-paginated lists).

```rust
// ars-collections/src/selection.rs

use alloc::collections::BTreeSet;
use crate::key::Key;

/// The set of currently selected keys.
///
/// `All` represents "every item is selected", including items not yet
/// loaded in async/paginated collections. When transitioning from `All`
/// to a specific set (e.g., the user deselects one item), the caller
/// must supply the full known key set to compute the complement.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Set {
    /// No items selected.
    #[default]
    Empty,

    /// Exactly one item is selected.
    Single(Key),

    /// Multiple items are selected.
    Multiple(BTreeSet<Key>),

    /// All items are selected.
    All,
}

impl Set {
    /// Returns `true` if `key` is in this selection.
    pub fn contains(&self, key: &Key) -> bool {
        match self {
            Self::Empty       => false,
            Self::All         => true,
            Self::Single(k)   => k == key,
            Self::Multiple(s) => s.contains(key),
        }
    }

    /// Returns `true` if no items are selected.
    pub fn is_empty(&self) -> bool { matches!(self, Self::Empty) }

    /// Returns `true` if all items are selected.
    pub fn is_all(&self) -> bool { matches!(self, Self::All) }

    /// Returns the single selected key, if exactly one item is selected.
    pub fn first(&self) -> Option<&Key> {
        match self {
            Self::Single(k) => Some(k),
            Self::Multiple(set) => set.first(),
            _ => None,
        }
    }

    /// Returns the count of selected items, or `None` for `All`.
    pub fn count(&self) -> Option<usize> {
        match self {
            Self::Empty => Some(0),
            Self::Single(_) => Some(1),
            Self::Multiple(set) => Some(set.len()),
            Self::All => None,
        }
    }

    /// Returns the number of selected items (0 for `All` — use `count()` for Option semantics).
    pub fn len(&self) -> usize {
        self.count().unwrap_or(0)
    }

    /// Iterate over selected keys. Returns an empty iterator for `All` —
    /// callers needing all keys must resolve against the full collection.
    pub fn keys(&self) -> Box<dyn Iterator<Item = &Key> + '_> {
        match self {
            Self::Empty | Self::All => Box::new(core::iter::empty()),
            Self::Single(k) => Box::new(core::iter::once(k)),
            Self::Multiple(set) => Box::new(set.iter()),
        }
    }
}
```

Consumers should prefer these methods over pattern matching to remain forward-compatible with future `selection::Set` variants.

`selection::Set` provides helper methods: `contains(key: &Key) -> bool`, `keys() -> impl Iterator<Item = &Key>`, `len() -> usize`, `count() -> Option<usize>`, `is_empty() -> bool`, `is_all() -> bool`, `first() -> Option<&Key>`. External code should prefer these helpers over exhaustive pattern matching. Adding new `selection::Set` variants is a breaking change covered by semver.

### 3.3 DisabledBehavior

When items are in the `disabled_keys` set, the `DisabledBehavior` enum controls what "disabled" means in practice. By default (`Skip`), disabled items are skipped during keyboard navigation: not focusable, not selectable, and not actionable. This matches the behavior of native `<option disabled>` in HTML `<select>` elements. However, some design systems require that disabled items remain focusable so that screen reader users can discover them and understand _why_ they are disabled (e.g., a tooltip or `aria-describedby` explanation). The `FocusOnly` variant enables this pattern: the item can receive focus and its label/description is announced, but selection and activation are blocked.

```rust
// ars-collections/src/selection.rs

/// Controls how disabled items behave in selection contexts.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum DisabledBehavior {
    /// Disabled items are skipped during keyboard navigation (not focusable, not selectable).
    #[default]
    Skip,
    /// Disabled items are focusable but not selectable (default for ARIA Listbox pattern).
    FocusOnly,
}
```

When `DisabledBehavior::FocusOnly` is active:

- Keyboard navigation (Arrow keys) **does** land on disabled items.
- The item receives `aria-disabled="true"` but retains `tabindex="-1"` (or `tabindex="0"` when focused under roving tabindex).
- Pressing Space/Enter on a disabled item is a no-op (no selection, no action).
- Mouse click on a disabled item moves focus to it but does not select it.

When `DisabledBehavior::Skip` is active (default):

- Keyboard navigation skips disabled items entirely (same as structural nodes).
- Disabled items are not reachable via mouse click focus.

#### 3.3.1 Structural vs Disabled Non-Focusability

Two distinct mechanisms prevent a node from receiving focus:

- **Structural** non-focusability is a property of the **Collection** — `key_after` / `key_before` always skip `Section`, `Header`, and `Separator` nodes. This is baked into the `Collection` trait implementation and cannot be overridden.
- **Disabled** non-focusability is **behavioral** — tracked in `selection::State::disabled_keys` and respected by the `next_enabled_key` / `prev_enabled_key` helpers (§3.3.2) only when `DisabledBehavior::Skip`. When `DisabledBehavior::FocusOnly`, disabled items remain navigable.

Components should use the `next_enabled_key` / `prev_enabled_key` helpers for all keyboard navigation so that both mechanisms are honored transparently.

**Cross-component consistency requirement (PARITY-v25-15):** All collection-based components with keyboard navigation — Combobox, Listbox, Menu, and TreeView — MUST use the shared `next_enabled_key()` / `prev_enabled_key()` / `first_enabled_key()` / `last_enabled_key()` utilities from `ars-collections/src/navigation.rs` for their arrow-key handlers. Components MUST NOT implement their own disabled-key skipping logic. Each component's keyboard navigation section should reference `DisabledBehavior` and these shared helpers:

- **Combobox** (`components/selection/combobox.md`): Arrow key handlers in the listbox portion use `next_enabled_key` / `prev_enabled_key` with the component's `DisabledBehavior` setting.
- **Listbox** (`components/selection/listbox.md`): All arrow, Home, End handlers delegate to `next_enabled_key` / `prev_enabled_key` / `first_enabled_key` / `last_enabled_key`.
- **Menu** (`components/selection/menu.md`): Arrow key handlers use `next_enabled_key` / `prev_enabled_key` to skip disabled menu items per `DisabledBehavior`.
- **TreeView** (`components/navigation/tree-view.md`): Arrow key handlers use `next_enabled_key` / `prev_enabled_key` over the flattened tree iteration order, respecting `DisabledBehavior`.

#### 3.3.2 Disabled-Aware Navigation Helpers

The `Collection` trait's `key_after` / `key_before` skip structural nodes but do **not** consider disabled keys (which live outside the collection in `selection::State`). The following free functions layer disabled-key awareness on top:

```rust
// ars-collections/src/navigation.rs

use alloc::collections::BTreeSet;
use crate::{key::Key, Collection, DisabledBehavior};

/// Navigate forward from `current`, skipping disabled keys when
/// `disabled_behavior` is `DisabledBehavior::Skip`.
///
/// When `wrap` is `true` and `current` is the last enabled item,
/// wraps to the first enabled item. When `false`, returns `None`.
pub fn next_enabled_key<T, C: Collection<T>>(
    collection: &C,
    current: &Key,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
    wrap: bool,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        // Disabled items are navigable — delegate directly.
        let next = if wrap {
            collection.key_after(current)
        } else {
            collection.key_after_no_wrap(current)
        };
        return next.cloned();
    }

    // DisabledBehavior::Skip — skip disabled keys.
    let mut candidate = if wrap {
        collection.key_after(current)
    } else {
        collection.key_after_no_wrap(current)
    };
    let start = candidate.cloned();
    loop {
        match candidate {
            None => return None,
            Some(k) if !disabled_keys.contains(k) => return Some(k.clone()),
            Some(k) => {
                candidate = if wrap {
                    collection.key_after(k)
                } else {
                    collection.key_after_no_wrap(k)
                };
                // Guard against infinite loop when all items are disabled.
                if candidate.cloned() == start {
                    return None;
                }
            }
        }
    }
}

/// Navigate backward — mirror of `next_enabled_key`.
pub fn prev_enabled_key<T, C: Collection<T>>(
    collection: &C,
    current: &Key,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
    wrap: bool,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        let prev = if wrap {
            collection.key_before(current)
        } else {
            collection.key_before_no_wrap(current)
        };
        return prev.cloned();
    }

    let mut candidate = if wrap {
        collection.key_before(current)
    } else {
        collection.key_before_no_wrap(current)
    };
    let start = candidate.cloned();
    loop {
        match candidate {
            None => return None,
            Some(k) if !disabled_keys.contains(k) => return Some(k.clone()),
            Some(k) => {
                candidate = if wrap {
                    collection.key_before(k)
                } else {
                    collection.key_before_no_wrap(k)
                };
                if candidate.cloned() == start {
                    return None;
                }
            }
        }
    }
}

/// The first enabled focusable key in the collection.
pub fn first_enabled_key<T, C: Collection<T>>(
    collection: &C,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        return collection.first_key().cloned();
    }
    // Walk from the first focusable key, skipping disabled.
    let mut candidate = collection.first_key();
    while let Some(k) = candidate {
        if !disabled_keys.contains(k) {
            return Some(k.clone());
        }
        candidate = collection.key_after_no_wrap(k);
    }
    None
}

/// The last enabled focusable key in the collection.
pub fn last_enabled_key<T, C: Collection<T>>(
    collection: &C,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        return collection.last_key().cloned();
    }
    let mut candidate = collection.last_key();
    while let Some(k) = candidate {
        if !disabled_keys.contains(k) {
            return Some(k.clone());
        }
        candidate = collection.key_before_no_wrap(k);
    }
    None
}
```

### 3.4 State

The full selection machine state. Used as part of component contexts (e.g., `listbox::Context`, `table::Context`) and kept in `Bindable<selection::Set>` for controlled/uncontrolled support (see `01-architecture.md §2.6`).

```rust
// ars-collections/src/selection.rs

use crate::{key::Key, Collection, selection};
use alloc::collections::BTreeSet;

/// The full selection state for a collection-based component.
///
/// All mutating methods return a new `State` (functional update),
/// enabling straightforward use inside state machine `Action::UpdateContext`
/// closures and reactive signals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Which items are currently selected.
    pub selected_keys: selection::Set,

    /// The anchor for Shift+Click / Arrow+Shift range extension.
    /// Updated whenever the user initiates a new selection without Shift.
    pub anchor_key: Option<Key>,

    /// The item that currently has focus (aria-activedescendant target).
    /// Distinct from `selected_keys` — focus and selection are orthogonal.
    pub focused_key: Option<Key>,

    /// Keys that are disabled and must be skipped during navigation and
    /// excluded from selection operations.
    pub disabled_keys: BTreeSet<Key>,

    /// Controls how disabled items behave (fully inert vs. focusable-only).
    /// See `DisabledBehavior` for details.
    pub disabled_behavior: DisabledBehavior,

    /// Selection mode for this instance.
    pub mode: selection::Mode,

    /// Selection behavior for this instance.
    pub behavior: selection::Behavior,

    /// Whether touch-based selection mode is currently active.
    /// On touch devices with `selection::Behavior::Replace`, tapping an item
    /// triggers its primary action by default. A long press enters selection
    /// mode (sets this to `true`), after which taps toggle selection.
    /// Deselecting all items exits selection mode (resets to `false`).
    pub selection_mode_active: bool,

    /// When set, further selections are blocked once the limit is reached.
    /// The adapter should disable unchecked checkboxes or provide a
    /// replace-oldest strategy.
    pub max_selection: Option<usize>,
}

impl State {
    /// Create a new `State` with the given mode and behavior.
    pub fn new(mode: selection::Mode, behavior: selection::Behavior) -> Self {
        Self {
            mode,
            behavior,
            ..Default::default()
        }
    }

    // ------------------------------------------------------------------ //
    // Key predicate helpers                                               //
    // ------------------------------------------------------------------ //

    /// Whether `key` is currently selected.
    pub fn is_selected(&self, key: &Key) -> bool {
        self.selected_keys.contains(key)
    }

    /// Whether `key` is disabled (not selectable, skipped in navigation).
    pub fn is_disabled(&self, key: &Key) -> bool {
        self.disabled_keys.contains(key)
    }

    // ------------------------------------------------------------------ //
    // Primitive mutators (return new state)                              //
    // ------------------------------------------------------------------ //

    /// Select a single key, respecting mode.
    ///
    /// - `None` mode: no-op.
    /// - `Single` mode: replaces entire selection.
    /// - `Multiple` / `Toggle` behavior: adds to the set.
    /// - `Multiple` / `Replace` behavior: replaces entire selection.
    pub fn select(&self, key: Key) -> Self {
        if self.mode == selection::Mode::None || self.is_disabled(&key) {
            return self.clone();
        }
        let selected_keys = match self.mode {
            selection::Mode::None => return self.clone(),
            selection::Mode::Single => {
                selection::Set::Single(key.clone())
            }
            selection::Mode::Multiple => match self.behavior {
                selection::Behavior::Toggle => {
                    let mut s = match &self.selected_keys {
                        selection::Set::Multiple(existing) => existing.clone(),
                        selection::Set::Single(k) => {
                            let mut s = BTreeSet::new();
                            s.insert(k.clone());
                            s
                        }
                        selection::Set::All => return self.clone(),
                        selection::Set::Empty => BTreeSet::new(),
                    };
                    s.insert(key.clone());
                    selection::Set::Multiple(s)
                }
                selection::Behavior::Replace => {
                    let mut s = BTreeSet::new();
                    s.insert(key.clone());
                    selection::Set::Multiple(s)
                }
            },
        };
        Self {
            selected_keys,
            anchor_key: Some(key),
            ..self.clone()
        }
    }

    /// Deselect a single key. No-op if not selected.
    pub fn deselect(&self, key: &Key) -> Self {
        let selected_keys = match &self.selected_keys {
            selection::Set::All => return self.clone(), // cannot deselect from All without the full key set
            selection::Set::Empty => return self.clone(),
            selection::Set::Single(k) => if *k == *key { selection::Set::Empty } else { return self.clone(); },
            selection::Set::Multiple(s) => {
                let mut new_s = s.clone();
                new_s.remove(key);
                if new_s.is_empty() {
                    selection::Set::Empty
                } else {
                    selection::Set::Multiple(new_s)
                }
            }
        };
        Self { selected_keys, ..self.clone() }
    }

    /// Deselect a single key when all items are currently selected.
    /// Requires the collection to compute the complement set.
    pub fn deselect_from_all<T, C: Collection<T>>(&self, key: &Key, collection: &C) -> Self {
        match &self.selected_keys {
            selection::Set::All => {
                let remaining: BTreeSet<Key> = collection.item_keys()
                    .filter(|k| *k != key)
                    .cloned()
                    .collect();
                Self {
                    selected_keys: selection::Set::Multiple(remaining),
                    ..self.clone()
                }
            }
            _ => self.deselect(key),
        }
    }

    /// Toggle the selection state of `key`.
    /// Accepts a `collection` reference so that toggling off from `All`
    /// can compute the complement set via `deselect_from_all`.
    pub fn toggle<T, C: Collection<T>>(&self, key: Key, collection: &C) -> Self {
        if self.is_selected(&key) {
            match &self.selected_keys {
                selection::Set::All => self.deselect_from_all(&key, collection),
                _ => self.deselect(&key),
            }
        } else {
            self.select(key)
        }
    }

    /// Select all items. Sets the `All` variant.
    ///
    /// No-op when `mode != Multiple`.
    pub fn select_all(&self) -> Self {
        if self.mode != selection::Mode::Multiple {
            return self.clone();
        }
        Self {
            selected_keys: selection::Set::All,
            ..self.clone()
        }
    }

    /// Clear the entire selection.
    pub fn clear(&self) -> Self {
        Self {
            selected_keys: selection::Set::Empty,
            anchor_key: None,
            selection_mode_active: false,
            ..self.clone()
        }
    }

    /// Extend the selection from `anchor_key` to `key` (Shift+Click /
    /// Arrow+Shift). Computes the contiguous range using the collection's
    /// iteration order.
    ///
    /// For `Single` mode this behaves identically to `select`.
    /// For `None` mode this is a no-op.
    pub fn extend_selection<T: Clone, C: Collection<T>>(
        &self,
        key: Key,
        collection: &C,
    ) -> Self {
        if self.mode == selection::Mode::None {
            return self.clone();
        }
        if self.mode == selection::Mode::Single {
            return self.select(key);
        }

        let anchor = match &self.anchor_key {
            Some(a) => a.clone(),
            None    => return self.select(key),
        };

        // Collect the range [anchor, key] or [key, anchor] in iteration order.
        let mut in_range = false;
        let mut range_keys: BTreeSet<Key> = BTreeSet::new();

        for node in collection.nodes() {
            if !node.is_focusable() { continue; }
            let is_anchor = &node.key == &anchor;
            let is_target = &node.key == &key;

            if is_anchor || is_target {
                in_range = !in_range;
                // Include both endpoints.
                if !self.is_disabled(&node.key) {
                    range_keys.insert(node.key.clone());
                }
            } else if in_range && !self.is_disabled(&node.key) {
                range_keys.insert(node.key.clone());
            }

            if is_anchor && is_target { break; } // same key
        }

        // Merge with the existing selection (range extension, not replacement).
        let existing = match &self.selected_keys {
            selection::Set::Multiple(s) => s.clone(),
            _ => BTreeSet::new(),
        };
        let merged = existing.into_iter().chain(range_keys).collect();

        Self {
            selected_keys: selection::Set::Multiple(merged),
            focused_key: Some(key),
            // anchor_key intentionally unchanged during range extension
            ..self.clone()
        }
    }

    /// Set focus to `key` without changing selection.
    pub fn set_focus(&self, key: Key) -> Self {
        Self { focused_key: Some(key), ..self.clone() }
    }

    /// Replace the disabled key set.
    pub fn with_disabled(self, disabled_keys: BTreeSet<Key>) -> Self {
        Self { disabled_keys, ..self }
    }
}
```

#### 3.4.1 Collection Update Invariant

When `Event::UpdateItems(new_items)` is processed: (1) Highlighted key is validated — if not present in `new_items`, reset to first item or `None`. (2) Selected keys are intersected with `new_items` keys; removed keys are dropped silently. (3) `UpdateItems` is idempotent — sending the same items again produces no state change. (4) If `UpdateItems` and a user navigation event (ArrowKey) arrive in the same batch, `UpdateItems` processes first, then navigation applies to the updated list.

#### 3.4.2 Multi-Item Selection Anchor Key Invalidation

When `anchor_key` is invalidated (deleted via `UpdateItems`), it resets to the current `focused_key`. If `focused_key` is also invalid, `anchor_key` resets to `None`. Guards must prevent `range_select()` when `anchor_key` is `None` — the operation becomes a single-select on the target key instead.

**Stale `anchor_key` fallback during range selection:** Even after `UpdateItems` validation, a race condition can cause `anchor_key` to reference a key that no longer exists in the collection (e.g., concurrent async item removal). The range selection algorithm MUST handle this gracefully:

1. Before computing the range, verify that `anchor_key` exists in the current collection via `collection.contains_key()`.
2. If `anchor_key` references a non-existent key, **fall back to single-item Replace selection** on the target key.
3. Update `anchor_key` to the new selection target so subsequent range operations have a valid anchor.

```rust
// In selection::State::extend_selection() / range_select():
// If anchor_key is stale (deleted between UpdateItems and the Shift+Click),
// degrade gracefully to a single-select Replace operation.
if let Some(ref anchor) = self.anchor_key {
    if !collection.contains_key(anchor) {
        // Anchor references a deleted item — treat as single-item Replace.
        return self.select(target_key);
    }
}
```

#### 3.4.3 Focus Strategy Interaction

The `focused_key` field in `selection::State` has different semantics depending on the component's `FocusStrategy` (see `03-accessibility.md`):

**Under `FocusStrategy::RovingTabindex` (default)**:

- `focused_key` tracks which item has `tabindex="0"` and receives actual DOM focus
- Arrow keys call `element.focus()` on the target item, moving DOM focus
- Each item renders: `tabindex={if key == focused_key { "0" } else { "-1" }}`

**Under `FocusStrategy::ActiveDescendant`**:

- `focused_key` tracks which item is the "active descendant" (visually highlighted but not DOM-focused)
- The container element keeps DOM focus and sets `aria-activedescendant="{focused_item_id}"`
- Arrow keys update `focused_key` without calling `element.focus()` on individual items
- Only the container has `tabindex="0"`; items have no `tabindex`

### 3.5 `selection::Set` Adoption Requirements

All components that support item selection **MUST** use `selection::Set` (not `BTreeSet<String>`) for their selection state. This is required for:

- **"Select All" semantics** — the `selection::Set::All` variant supports selecting all items including unloaded pages in async collections, without materializing the full key set.
- **Consistent API surface** — consumers learn one selection model across all selectable components.
- **`selection::State` integration** — the `select`, `deselect`, `toggle`, `extend_selection`, `select_all`, and `clear` methods on `selection::State` all operate on `selection::Set`.

**Components that must migrate from `BTreeSet<String>` to `selection::Set`:**

| Component  | Current                                          | Required                                                      |
| ---------- | ------------------------------------------------ | ------------------------------------------------------------- |
| `Table`    | ~~`Bindable<BTreeSet<String>>`~~                 | `Bindable<selection::Set>` + `selection::State` — **Adopted** |
| `GridList` | `Bindable<BTreeSet<String>>` for `selected_keys` | `Bindable<selection::Set>` + `selection::State`               |
| `TagGroup` | `Bindable<BTreeSet<String>>` for `selected_keys` | `Bindable<selection::Set>` + `selection::State`               |
| `TreeView` | ~~`Bindable<BTreeSet<String>>`~~                 | `Bindable<selection::Set>` + `selection::State` — **Adopted** |

**Also**: `TreeView` previously defined its own `selection::Mode` enum (None/Single/Multiple) — this has been replaced with the canonical `ars_collections::selection::Mode`.

### 3.6 Controlled vs Uncontrolled Selection

Selection state integrates with the `Bindable<T>` pattern from `01-architecture.md §2.6`:

```rust
// Inside a component context, e.g., listbox::Context:

use ars_collections::selection;

pub struct Context {
    /// The controlled/uncontrolled selection binding.
    /// The inner `selection::Set` is the source of truth.
    pub selection: Bindable<selection::Set>,

    /// The internal `selection::State` holds mode, behavior, anchor, focus,
    /// and disabled_keys. Its `selected_keys` field is always derived from
    /// `self.selection.get()` during state machine transitions.
    pub selection_state: selection::State,

    // ... other context fields
}
```

When a component operates in **controlled** mode, selection mutations call `Bindable::set` which invokes the `on_change` callback without updating internal state. The external source of truth re-flows the new `selection::Set` back via prop update. When **uncontrolled**, `Bindable::set` updates the internal value directly.

### 3.7 Selection Interactions Reference

| User Action            | Effect                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| Click item             | `replace_behavior`: `select(key)`, clearing others. `toggle_behavior`: `toggle(key)`     |
| Shift+Click            | `extend_selection(key, collection)` from anchor                                          |
| Ctrl/Cmd+Click         | `toggle(key)`, anchor updates to `key`                                                   |
| Space                  | Same as Click for the focused item                                                       |
| Enter                  | Confirm focused item (component-specific; often closes overlay)                          |
| ArrowDown / ArrowRight | `key_after(focused)`, move focus only (no selection in `Replace` mode unless Shift held) |
| ArrowUp / ArrowLeft    | `key_before(focused)`                                                                    |
| Shift+ArrowDown        | `extend_selection(key_after(focused))`                                                   |
| Shift+ArrowUp          | `extend_selection(key_before(focused))`                                                  |
| Ctrl+A / Cmd+A         | `select_all()` (Multiple mode only)                                                      |
| Escape                 | `clear()` selection or close overlay (component-specific)                                |
| Home                   | Focus first item                                                                         |
| End                    | Focus last item                                                                          |
| Shift+Home             | Extend selection to first item                                                           |
| Shift+End              | Extend selection to last item                                                            |

Navigation always skips disabled items (those in `selection::State::disabled_keys`) and structural nodes (Section, Header, Separator). When `DisabledBehavior::FocusOnly` is active, disabled items are _not_ skipped during navigation but are still excluded from selection operations.

### 3.8 Touch Device Behavior

When `selection::Behavior::Replace` is active on touch devices:

| Interaction             | Effect                                           |
| ----------------------- | ------------------------------------------------ |
| Tap                     | Triggers item's primary action — does NOT select |
| Long press (500ms)      | Enters selection mode; selects pressed item      |
| Tap (in selection mode) | Toggles item selection                           |
| Deselect all items      | Exits selection mode                             |

This requires integration with `use_long_press` (see `05-interactions.md §5`). The `selection_mode_active: bool` field on `selection::State` tracks whether the user has entered touch selection mode. When `selection_mode_active` is `true`, tap behaves like `toggle(key)` instead of triggering the primary action. When the last selected item is deselected (i.e., `selected_keys` becomes `Empty`), `selection_mode_active` is automatically reset to `false`.

Components should render a visual indicator (e.g., checkboxes on each item) when `selection_mode_active` is `true` so the user understands they are in selection mode.

### 3.9 Empty State Messages

Collection-rendering components (Table, GridList, Select, Combobox, etc.) **MUST** include an `empty_label` field in their Messages struct:

```rust
/// Displayed when the collection contains zero items. Screen readers
/// announce this via `aria-live`.
pub empty_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
```

The adapter renders this message inside the `empty-state` anatomy part (see `components/data-display/table.md` §Empty State Handling). The `empty-state` container carries `aria-live="polite"` so screen readers announce when a previously populated collection becomes empty (e.g., after filtering).

---

## 4. Type-Ahead / Type-Select

Type-ahead allows users to jump to items by typing text matching item labels. It is implemented in `ars-collections` and consumed by any component that renders a list with keyboard navigation (Listbox, Select, Menu, Combobox, TreeView).

### 4.1 State

```rust
// ars-collections/src/typeahead.rs

use alloc::{collections::BTreeSet, string::String};
use core::time::Duration;
#[cfg(feature = "i18n")]
use ars_i18n::Locale;
use crate::{key::Key, Collection};

/// Default time window for accumulating multi-character type-ahead queries.
pub const TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(500);

/// The accumulated type-ahead search state.
///
/// Lives inside the component's `Context` struct alongside `selection::State`.
/// Updated on every `keydown` event that produces a printable character.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The accumulated search string, e.g. `"ban"` after typing B, A, N.
    pub search: String,

    /// Timestamp (in milliseconds since epoch) of the last keypress that
    /// contributed to `search`. Used to detect timeout and reset.
    ///
    /// The component's state machine obtains this timestamp from an abstract
    /// `Clock` trait (see `01-architecture.md §1.4` on no_std Timer/Clock).
    pub last_key_time_ms: u64,

    /// The key that was focused when the current search started. Used as the
    /// starting point for wrap-around: if we reach the end of the list without
    /// a match, we wrap to the beginning and continue searching up to (but not
    /// including) the start key.
    pub search_start_key: Option<Key>,
}

impl State {
    /// Process a new character from a keydown event.
    ///
    /// - If `now_ms - last_key_time_ms >= TYPEAHEAD_TIMEOUT`, the search
    ///   string is reset before appending the new character.
    /// - Returns `Some(key)` if a match was found, `None` otherwise.
    #[cfg(feature = "i18n")]
    pub fn process_char<T, C: Collection<T>>(
        &self,
        ch: char,
        now_ms: u64,
        current_focus: Option<&Key>,
        collection: &C,
        locale: &Locale,
        disabled_keys: &BTreeSet<Key>,
        disabled_behavior: DisabledBehavior,
    ) -> (Self, Option<Key>) {
        // Determine whether to reset the accumulated search.
        let timed_out = Duration::from_millis(now_ms.saturating_sub(self.last_key_time_ms)) >= TYPEAHEAD_TIMEOUT;
        let mut search = if timed_out {
            String::new()
        } else {
            self.search.clone()
        };
        search.push(ch);

        let search_start = if timed_out || self.search_start_key.is_none() {
            current_focus.cloned()
        } else {
            self.search_start_key.clone()
        };

        let found = Self::find_match(&search, current_focus, collection, locale, disabled_keys, disabled_behavior);

        let new_state = Self {
            search,
            last_key_time_ms: now_ms,
            search_start_key: search_start,
        };

        (new_state, found)
    }

    /// Process a new character (non-i18n fallback using ASCII case folding).
    #[cfg(not(feature = "i18n"))]
    pub fn process_char<T, C: Collection<T>>(
        &self,
        ch: char,
        now_ms: u64,
        current_focus: Option<&Key>,
        collection: &C,
        disabled_keys: &BTreeSet<Key>,
        disabled_behavior: DisabledBehavior,
    ) -> (Self, Option<Key>) {
        let timed_out = Duration::from_millis(now_ms.saturating_sub(self.last_key_time_ms)) >= TYPEAHEAD_TIMEOUT;
        let mut search = if timed_out {
            String::new()
        } else {
            self.search.clone()
        };
        search.push(ch);

        let search_start = if timed_out || self.search_start_key.is_none() {
            current_focus.cloned()
        } else {
            self.search_start_key.clone()
        };

        let found = Self::find_match(&search, current_focus, collection, disabled_keys, disabled_behavior);

        let new_state = Self {
            search,
            last_key_time_ms: now_ms,
            search_start_key: search_start,
        };

        (new_state, found)
    }

    /// Find the first item whose `text_value` starts with `search`
    /// (locale-aware case folding via ICU4X `CaseMapper`), beginning the
    /// search from the item *after* `current_focus` (single-char, cycling)
    /// or *at* `current_focus` (multi-char, refining).
    ///
    /// Single-character searches wrap; multi-character searches do not (they
    /// stay within the current alphabetical run to avoid disorienting jumps).
    #[cfg(feature = "i18n")]
    fn find_match<T, C: Collection<T>>(
        search: &str,
        current_focus: Option<&Key>,
        collection: &C,
        locale: &Locale,
        disabled_keys: &BTreeSet<Key>,
        disabled_behavior: DisabledBehavior,
    ) -> Option<Key> {
        // CaseMapper::new() returns CaseMapperBorrowed<'static> which is Copy —
        // no caching needed, can be constructed freely.
        let case_mapper = icu::casemap::CaseMapper::new();
        // single_char from raw input — case mapping can expand one char to many.
        let single_char = search.chars().count() == 1;
        let query = case_mapper.lowercase_to_string(search, &locale.language_identifier());

        let skip_disabled = disabled_behavior == DisabledBehavior::Skip;
        let all_item_keys: alloc::vec::Vec<Key> = collection
            .nodes()
            .filter(|n| n.is_focusable() && (!skip_disabled || !disabled_keys.contains(&n.key)))
            .map(|n| n.key.clone())
            .collect();

        if all_item_keys.is_empty() {
            return None;
        }

        // Single-char: start AFTER current_focus (cycling to next match).
        // Multi-char: start AT current_focus (refining keeps current match viable).
        let start_pos = current_focus
            .and_then(|k| all_item_keys.iter().position(|ik| ik == k))
            .map_or(0, |p| if single_char { (p + 1) % all_item_keys.len() } else { p });

        // Single-char wraps around the full list; multi-char scans forward only.
        let scan_len = if single_char { all_item_keys.len() } else { all_item_keys.len().saturating_sub(start_pos) };

        for offset in 0..scan_len {
            let idx = (start_pos + offset) % all_item_keys.len();
            let key = &all_item_keys[idx];
            if let Some(text) = collection.text_value_of(key) {
                if case_mapper.lowercase_to_string(text, &locale.language_identifier()).starts_with(&query) {
                    return Some(key.clone());
                }
            }
        }

        None
    }

    #[cfg(not(feature = "i18n"))]
    fn find_match<T, C: Collection<T>>(
        search: &str,
        current_focus: Option<&Key>,
        collection: &C,
        disabled_keys: &BTreeSet<Key>,
        disabled_behavior: DisabledBehavior,
    ) -> Option<Key> {
        // single_char from raw input — case mapping can expand one char to many.
        let single_char = search.chars().count() == 1;
        let query = search.to_lowercase();

        let skip_disabled = disabled_behavior == DisabledBehavior::Skip;
        let all_item_keys: alloc::vec::Vec<Key> = collection
            .nodes()
            .filter(|n| n.is_focusable() && (!skip_disabled || !disabled_keys.contains(&n.key)))
            .map(|n| n.key.clone())
            .collect();

        if all_item_keys.is_empty() {
            return None;
        }

        // Single-char: start AFTER current_focus (cycling to next match).
        // Multi-char: start AT current_focus (refining keeps current match viable).
        let start_pos = current_focus
            .and_then(|k| all_item_keys.iter().position(|ik| ik == k))
            .map_or(0, |p| if single_char { (p + 1) % all_item_keys.len() } else { p });

        // Single-char wraps around the full list; multi-char scans forward only.
        let scan_len = if single_char { all_item_keys.len() } else { all_item_keys.len().saturating_sub(start_pos) };

        for offset in 0..scan_len {
            let idx = (start_pos + offset) % all_item_keys.len();
            let key = &all_item_keys[idx];
            if let Some(text) = collection.text_value_of(key) {
                if text.to_lowercase().starts_with(&query) {
                    return Some(key.clone());
                }
            }
        }

        None
    }

    /// Reset the type-ahead state (e.g., when the user presses Escape or
    /// the component loses focus).
    #[must_use]
    pub fn reset() -> Self {
        Self::default()
    }
}
```

### 4.2 Integration in a Component's Event Handler

Inside the component's `transition` function (see `01-architecture.md §2.7`):

```rust
// Pseudocode inside a Listbox::transition match arm:

Event::KeyDown(key_event) => {
    let ch = key_event.key_as_char()?; // returns None for non-printable keys
    let now_ms = ctx.clock.now_ms();
    let (new_typeahead, found_key) = ctx.typeahead.process_char(
        ch, now_ms, ctx.focused_key.as_ref(), &ctx.collection, &ctx.locale,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    );
    if let Some(target_key) = found_key {
        Some(TransitionPlan::context_only(move |ctx| {
            ctx.typeahead = new_typeahead;
            ctx.selection_state = ctx.selection_state.set_focus(target_key);
        }))
    } else {
        Some(TransitionPlan::context_only(move |ctx| {
            ctx.typeahead = new_typeahead;
        }))
    }
}
```

---

## 5. Async Loading

`AsyncCollection<T>` is the pagination layer built on top of the core `Collection<T>` trait. It handles cursor-based pagination, infinite scroll, and retry semantics for server-driven data sources.

### 5.1 AsyncLoadingState

```rust
// ars-collections/src/async_collection.rs

/// The current loading phase of an async collection or page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum AsyncLoadingState {
    /// No load has been initiated yet.
    #[default]
    Idle,

    /// The initial load is in progress. The collection is empty.
    Loading,

    /// Additional pages are being fetched. The collection already has items.
    LoadingMore,

    /// Data is fully loaded. No more pages.
    Loaded,

    /// A load failed. The error message is surfaced for display or retry.
    Error(alloc::string::String),
}

impl AsyncLoadingState {
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self, Self::Loading | Self::LoadingMore)
    }

    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        if let Self::Error(msg) = self {
            Some(msg.as_str())
        } else {
            None
        }
    }
}
```

### 5.2 AsyncCollection

`AsyncCollection<T>` wraps a `StaticCollection<T>` (the items loaded so far) with cursor-based pagination metadata and loading state. It is not itself reactive — the component's machine context holds it in a `Signal` or similar framework primitive.

```rust
// ars-collections/src/async_collection.rs

use alloc::{string::String, vec::Vec};
use crate::{Key, Node, Collection, StaticCollection, AsyncLoadingState};

/// A collection that grows over time as pages are fetched.
///
/// The component machine drives loading: when the sentinel element (the last
/// rendered item or a dedicated loading indicator) becomes visible, the
/// machine emits a `LoadMore` event, which triggers the async fetch effect.
/// When the fetch completes, the machine merges new items via `append_page`.
pub struct AsyncCollection<T: Clone> {
    /// Items loaded so far.
    inner: StaticCollection<T>,

    /// Opaque cursor for the next page request. `None` means either the
    /// collection has not started loading or all pages are exhausted.
    pub next_cursor: Option<String>,

    /// Whether all pages have been fetched.
    pub has_more: bool,

    /// Current loading phase.
    pub loading_state: AsyncLoadingState,

    /// Total item count if known from the server (e.g., from a `total`
    /// field in the API response). `None` when unknown.
    pub total_count: Option<usize>,
}

impl<T: Clone> AsyncCollection<T> {
    /// Create an empty async collection ready for its first load.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: StaticCollection::default(),
            next_cursor: None,
            has_more: true,
            loading_state: AsyncLoadingState::Idle,
            total_count: None,
        }
    }

    /// Transition to the loading state before a fetch begins.
    #[must_use]
    pub fn begin_load(&self) -> Self {
        let state = if self.inner.is_empty() {
            AsyncLoadingState::Loading
        } else {
            AsyncLoadingState::LoadingMore
        };
        Self {
            loading_state: state,
            ..self.clone_meta()
        }
    }

    /// Append a new page of items, updating cursor and has_more.
    #[must_use]
    pub fn append_page(
        &self,
        new_items: Vec<(Key, String, T)>,
        next_cursor: Option<String>,
    ) -> Self {
        let has_more = next_cursor.is_some();
        // Merge existing items with the new page.
        let mut merged: Vec<(Key, String, T)> = self
            .inner
            .nodes()
            .filter_map(|n| {
                n.value.as_ref().map(|v| {
                    (n.key.clone(), n.text_value.clone(), v.clone())
                })
            })
            .collect();
        merged.extend(new_items);

        Self {
            inner: StaticCollection::new(merged),
            next_cursor,
            has_more,
            loading_state: AsyncLoadingState::Loaded,
            total_count: self.total_count,
        }
    }

    /// Record a load error.
    #[must_use]
    pub fn set_error(&self, message: impl Into<String>) -> Self {
        Self {
            loading_state: AsyncLoadingState::Error(message.into()),
            ..self.clone_meta()
        }
    }

    fn clone_meta(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            next_cursor: self.next_cursor.clone(),
            has_more: self.has_more,
            loading_state: self.loading_state.clone(),
            total_count: self.total_count,
        }
    }
}

impl<T: Clone> Default for AsyncCollection<T> {
    fn default() -> Self { Self::new() }
}

// Delegate Collection<T> to the inner StaticCollection.
impl<T: Clone> Collection<T> for AsyncCollection<T> {
    fn size(&self) -> usize { self.inner.size() }
    fn get(&self, key: &Key) -> Option<&Node<T>> { self.inner.get(key) }
    fn get_by_index(&self, index: usize) -> Option<&Node<T>> { self.inner.get_by_index(index) }
    fn first_key(&self) -> Option<&Key> { self.inner.first_key() }
    fn last_key(&self) -> Option<&Key> { self.inner.last_key() }
    fn key_after(&self, key: &Key) -> Option<&Key> { self.inner.key_after(key) }
    fn key_before(&self, key: &Key) -> Option<&Key> { self.inner.key_before(key) }
    fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_after_no_wrap(key) }
    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> { self.inner.key_before_no_wrap(key) }
    fn keys(&self) -> impl Iterator<Item = &Key> { self.inner.keys() }
    fn nodes(&self) -> impl Iterator<Item = &Node<T>> { self.inner.nodes() }
    fn children_of(&self, parent_key: &Key) -> impl Iterator<Item = &Node<T>> {
        self.inner.children_of(parent_key)
    }
}

impl<T: Clone> Clone for AsyncCollection<T> {
    fn clone(&self) -> Self { self.clone_meta() }
}
```

#### 5.2.1 AsyncLoader Trait and LoadResult

While `AsyncCollection<T>` holds the accumulated state, the _loading logic_ is defined by the `AsyncLoader<T>` trait. Each component that uses async data implements (or is provided) a loader that knows how to fetch a single page.

````rust
// ars-collections/src/async_loader.rs

use alloc::{string::String, vec::Vec};

/// The result of fetching a single page of items.
#[derive(Clone, Debug)]
pub struct LoadResult<T> {
    /// The items returned by this page.
    pub items: Vec<T>,

    /// Opaque cursor for the next page. `None` signals that no more pages
    /// exist. The `AsyncCollection` stores this and passes it back on the
    /// next `load_page` call.
    pub next_cursor: Option<String>,

    /// Total number of items across all pages, if the server provides it.
    /// Used to set `aria-setsize` on virtualized items before all pages
    /// have been fetched.
    pub total_count: Option<usize>,
}

/// Error type returned by async page loads.
#[derive(Clone, Debug)]
pub struct CollectionError {
    /// A human-readable error message (not shown to end users by default;
    /// used for logging and the retry UI).
    pub message: String,

    /// Whether the caller should retry the request. Set to `false` for
    /// permanent failures (e.g., 404 Not Found).
    pub retryable: bool,
}

impl core::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Defines how to fetch a single page of data for an `AsyncCollection`.
///
/// Implementations are provided by the application layer. The component
/// machine calls `load_page` inside a framework-managed async effect
/// (e.g., Leptos `create_resource`, Dioxus `use_future`).
///
/// ```rust
/// struct UserLoader { search_query: String }
///
/// impl AsyncLoader<User> for UserLoader {
///     type Fut = Pin<Box<dyn Future<Output = Result<LoadResult<User>, CollectionError>>>>;
///
///     fn load_page(&self, cursor: Option<&str>) -> Self::Fut {
///         let query = self.search_query.clone();
///         let cursor = cursor.map(String::from);
///         Box::pin(async move {
///             let resp = api::search_users(&query, cursor.as_deref()).await?;
///             Ok(LoadResult {
///                 items: resp.users,
///                 next_cursor: resp.next_cursor,
///                 total_count: Some(resp.total),
///             })
///         })
///     }
/// }
/// ```
pub trait AsyncLoader<T> {
    /// The future type returned by `load_page`. This is an associated type
    /// rather than `async fn` to support `no_std` environments and to give
    /// callers control over boxing and pinning.
    type Fut: Future<Output = Result<LoadResult<T>, CollectionError>>;

    /// Fetch a single page of items starting from `cursor`.
    ///
    /// - `cursor` is `None` for the initial load and `Some(...)` for
    ///   subsequent pages. The value comes from `LoadResult::next_cursor`
    ///   returned by the previous call.
    /// - The returned future **must be cancel-safe**: dropping it before
    ///   completion cancels the in-flight request with no side effects.
    ///   This is the standard cancellation semantic — if the component
    ///   unmounts or the user navigates away, the adapter drops the future
    ///   and no further callbacks fire.
    ///
    /// # Errors
    ///
    /// Returns `CollectionError` when the page load fails. The `retryable`
    /// flag indicates whether the caller should offer a retry.
    fn load_page(&self, cursor: Option<&str>) -> Self::Fut;
}
````

#### 5.2.2 Error Recovery

When a page load fails, `AsyncCollection` transitions to `AsyncLoadingState::Error`. The component renders a retry affordance (button or link). Recovery follows this sequence:

1. The user activates the retry trigger.
2. The machine calls `collection.begin_load()` to transition back to `Loading` / `LoadingMore`.
3. The machine re-invokes `loader.load_page(collection.next_cursor.as_deref())` with the **same cursor** that failed, effectively retrying the failed page.

For automated retry with exponential backoff, the adapter layer (not the core collection) can wrap the loader:

```rust
/// Wraps an `AsyncLoader` with retry logic. The core collection library
/// does not include this — it lives in the adapter or application layer.
///
/// Backoff schedule: 200ms, 400ms, 800ms, 1600ms, capped at 5s.
/// Maximum attempts: 3 (configurable).
pub struct RetryLoader<T, L: AsyncLoader<T>> {
    inner: L,
    max_attempts: usize,
    _marker: core::marker::PhantomData<T>,
}
```

#### 5.2.3 Cancellation Semantics

Async page loads are cancelled by dropping the future returned by `AsyncLoader::load_page`. This happens automatically when:

- The component unmounts (the adapter drops all pending futures).
- A new load is triggered before the previous one completes (e.g., the user scrolls quickly past multiple sentinel elements). The machine drops the old future and starts a new `load_page` call.
- The search query changes in a filtered async list — the entire collection is replaced and pending loads for the old query are dropped.

No explicit `cancel()` method is needed. The Rust ownership model ensures cleanup.

#### 5.2.4 Selection Preservation Across Page Loads

Selected keys are maintained in the `selection::State` (see §4), not in the `AsyncCollection` itself. When a new page is appended via `append_page`:

- Existing `selection::State` keys remain valid because previously loaded items keep their `Key` values.
- The adapter does **not** clear or rebuild the selection set on page load.
- If a selected key refers to an item that has not yet been loaded (possible when selection state is restored from a URL or external store), the selection is considered _pending_. Components treat pending selections as valid — the item will appear selected once its page loads.

#### 5.2.5 Debounce and Throttle for Scroll-Triggered Loads

When the loading sentinel enters the viewport, the adapter debounces `LoadMore` events to prevent duplicate requests during fast scrolling:

- **Throttle window**: At most one `LoadMore` event per 150ms (configurable via `load_throttle_ms` on the component props).
- **Guard**: The machine ignores `LoadMore` if `loading_state.is_loading()` is already `true`.
- **Scroll direction**: The sentinel only triggers when scrolling _toward_ it (downward for vertical lists). Scrolling away does not trigger a load.

### 5.3 Infinite Scroll with Sentinel

The sentinel pattern places a non-interactive "loading more" element after the last rendered item. When it enters the viewport, the component fires a `LoadMore` event.

In the connect API:

```rust
impl<'a> Api<'a> {
    /// Props for the sentinel element rendered after all items when
    /// `has_more` is true. The framework adapter attaches an
    /// `IntersectionObserver` to this element.
    ///
    /// Attributes include `data-ars-part="loading-sentinel"` and
    /// `aria-hidden="true"` (it carries no content for screen readers;
    /// loading progress is announced via `LiveAnnouncer`).
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap> {
        if !self.ctx.collection.has_more { return None; }
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Data("ars-scope"), "listbox");
        attrs.set(HtmlAttr::Data("ars-part"), "loading-sentinel");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::TabIndex, "-1");
        Some(attrs)
    }
}

/// The full listbox Part enum is defined in `components/selection/listbox.md`.
/// The `LoadingSentinel` variant is added here for infinite scroll support.
impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            // ... other listbox parts ...
            Part::LoadingSentinel => self.loading_sentinel_attrs()
                .unwrap_or_else(AttrMap::new),
        }
    }
}
```

### 5.4 Retry Pattern

When `loading_state` is `Error`, the component renders a retry trigger. Clicking it re-emits the `LoadMore` event, which calls `begin_load()` and re-runs the fetch effect.

---

## 6. Virtualization

> **React Aria mapping**: The `Virtualizer` struct below corresponds to React Aria's `Virtualizer` component.
> Components that support virtualization (Listbox, Select, Combobox, Table, TreeView, GridList) integrate
> `Virtualizer` via their machine context. See individual component specs for integration details.

Virtualization renders only the visible subset of a potentially large collection, mapping each visible item to an absolute position in a scroll container. The virtualizer is maintained in the component's machine context and updated whenever the viewport scrolls or the collection changes.

The Virtualizer supports both vertical and horizontal scrolling via the `orientation` and `dir` fields. Vertical is the default; horizontal mode is used by Carousel and horizontal GridList. RTL locales reverse the horizontal scroll direction.

### 6.1 Layout Strategies

```rust
// ars-collections/src/virtualization.rs

/// How item sizes are determined for layout calculations.
#[derive(Clone, Debug)]
pub enum LayoutStrategy {
    /// Every item has the same pixel height. O(1) scroll position math.
    /// The most efficient strategy; use when items are uniform.
    FixedHeight { item_height: f64 },

    /// Items have varying heights, measured after first render and cached.
    /// Estimated height used for items not yet measured.
    VariableHeight { estimated_item_height: f64 },

    /// Items are arranged in a grid with fixed column count.
    /// `item_height` is the row height; `columns` is the number of columns.
    Grid { item_height: f64, columns: NonZero<usize> },

    /// Grid layout: equal-sized items in responsive columns.
    /// Column count is computed from `viewport_width / min_item_width`, then
    /// items are sized equally within `[min_item_width, max_item_width]`.
    /// Essential for GridList virtualization.
    GridLayout {
        min_item_width: f64,
        max_item_width: f64,
        min_item_height: f64,
        max_item_height: Option<f64>,
        gap: f64,
    },

    /// Waterfall/masonry layout: variable-height items placed in the shortest column.
    /// Column count is derived the same way as GridLayout. Requires knowing item
    /// heights ahead of time or measuring after first render.
    WaterfallLayout {
        min_item_width: f64,
        max_item_width: f64,
        min_item_height: f64,
        gap: f64,
    },

    /// Table layout: column-aware positioning for virtualized tables.
    /// Supports sticky headers, column resize, and per-column widths.
    /// Matches React Aria's `TableLayout` for Virtualizer integration with Table.
    TableLayout {
        /// Height of each data row (pixels). For variable-height rows, this is
        /// the estimated height; actual heights are measured and cached like
        /// `VariableHeight`.
        row_height: f64,
        /// Height of the sticky header row (pixels).
        header_height: f64,
        /// Column widths (pixels). Length equals number of visible columns.
        /// Updated by the Table's column resize interaction.
        column_widths: Vec<f64>,
        /// Vertical gap between rows (pixels).
        row_gap: f64,
    },
}

impl LayoutStrategy {
    /// Estimated height of a single item (used before actual measurement).
    pub fn estimated_item_height(&self) -> f64 {
        match self {
            LayoutStrategy::FixedHeight { item_height }         => *item_height,
            LayoutStrategy::VariableHeight { estimated_item_height } => *estimated_item_height,
            LayoutStrategy::Grid { item_height, .. }            => *item_height,
            LayoutStrategy::GridLayout { min_item_height, .. }  => *min_item_height,
            LayoutStrategy::WaterfallLayout { min_item_height, .. } => *min_item_height,
            LayoutStrategy::TableLayout { row_height, .. }      => *row_height,
        }
    }

    /// Estimated size of a single item along the active scroll axis.
    pub fn estimated_item_extent(&self, orientation: Orientation) -> f64 {
        match orientation {
            Orientation::Vertical => self.estimated_item_height(),
            Orientation::Horizontal => match self {
                LayoutStrategy::GridLayout { min_item_width, .. }
                | LayoutStrategy::WaterfallLayout { min_item_width, .. } => *min_item_width,
                LayoutStrategy::TableLayout { row_height, column_widths, .. } => {
                    column_widths.first().copied().unwrap_or(*row_height)
                }
                _ => self.estimated_item_height(),
            },
        }
    }
}
```

#### 6.1.1 Variable-Height Item Measurement

For variable-height items, the virtualizer maintains a height cache (`BTreeMap<usize, f64>`, keyed by flat index). Visible items are measured via `ResizeObserver`; measured heights update the cache. Off-screen items use `estimated_item_height: f64` (required prop for variable-height mode). On scroll, the virtualizer recalculates positions from cached/estimated heights. Total scroll height updates incrementally as items are measured.

### 6.2 Focused Element Persistence (React Aria Parity)

When using virtualization with keyboard navigation, the currently focused item MUST remain
in the DOM even when scrolled out of the visible range. This ensures:

- Screen readers can still access the focused element
- `aria-activedescendant` references remain valid
- Keyboard navigation (arrow keys) continues to work from the correct position

The Virtualizer's `visible_range()` method always includes the focused item's index in the
returned range, even if that index falls outside the scroll-determined visible window. The
adapter renders the focused item at its correct absolute position but with `visibility: hidden`
(or `opacity: 0`) when it is off-screen, preserving DOM presence without visual artifacts.

### 6.3 Virtualizer Struct

```rust
// ars-collections/src/virtualization.rs

use alloc::collections::BTreeMap;
use crate::key::Key;

/// The number of items to render beyond the visible range on each side.
/// Reduces blank flicker during fast scrolling at the cost of more DOM nodes.
pub const DEFAULT_OVERSCAN: usize = 5;

/// Computes the visible item range, absolute item positions, and total scroll
/// height for a virtualized list.
///
/// The virtualizer is purely a calculation structure — it has no DOM access.
/// The component's machine context holds a `Virtualizer` instance and updates
/// it in response to scroll events and collection changes. Framework adapters
/// read `visible_range` to decide which items to render, and apply
/// `item_offset_px` as a `transform: translateY(...)` or `top` style.
/// Clone deep-copies all state including the measured height cache.
/// Each clone operates independently — mutations to one do not affect the other.
#[derive(Debug, Clone)]
pub struct Virtualizer {
    /// Total number of items in the collection (the full logical count,
    /// including pages not yet loaded for async collections).
    pub total_count: usize,

    /// Layout strategy controlling how heights are calculated.
    pub layout: LayoutStrategy,

    /// Height of the scroll container viewport in pixels.
    pub viewport_height: f64,

    /// Width of the scroll container viewport in pixels.
    /// Required by `GridLayout` and `WaterfallLayout` to compute column count
    /// from `viewport_width / min_item_width`. For single-column layouts
    /// (`FixedHeight`, `VariableHeight`), this field is unused.
    pub viewport_width: f64,

    /// Current vertical scroll offset in pixels (distance scrolled from the top).
    pub scroll_top: f64,

    /// Current horizontal scroll offset in pixels (distance scrolled from the inline-start edge).
    /// Used when `orientation` is `Horizontal`. For RTL locales, the adapter normalizes
    /// the browser's `scrollLeft` value before setting this field (see §6.4).
    pub scroll_left: f64,

    /// Scroll axis for virtualization. `Vertical` (default) virtualizes along
    /// the Y-axis; `Horizontal` virtualizes along the X-axis (used by Carousel,
    /// horizontal GridList).
    pub orientation: Orientation,

    /// Text direction. When `Rtl` and `orientation` is `Horizontal`, scroll
    /// position is measured from the inline-end edge. See §6.4 for browser
    /// normalization.
    pub dir: Direction,

    /// Extra items rendered outside the visible range on each side.
    pub overscan: usize,

    /// Measured heights for variable-height items, keyed by flat index.
    /// Populated after each render by calling `report_item_height`.
    measured_heights: BTreeMap<usize, f64>,

    /// Index of the currently focused item, if any. When set, `visible_range()`
    /// includes this index in the rendered range even if it falls outside the
    /// scroll-determined visible window. This ensures the focused element remains
    /// in the DOM for screen readers and keyboard navigation (see §6.2).
    pub focused_index: Option<usize>,
}
```

#### 6.3.1 VirtualLayout Trait

The `VirtualLayout` trait abstracts vertical layout calculations so that components can swap between fixed-height, variable-height, and grid strategies without changing their scroll-handling logic. Custom layout implementations (e.g., masonry, sticky headers) implement this trait and plug into the `Virtualizer`.

> **Note:** The built-in `LayoutStrategy` enum variants use inline implementations within the `Virtualizer` methods. The `VirtualLayout` trait is an extension point for custom vertical layouts that don't fit the built-in strategies. Layouts that also support horizontal virtualization implement the separate `HorizontalVirtualLayout` trait instead of relying on panic-prone default methods.

````rust
// ars-collections/src/virtual_layout.rs

use core::ops::Range;

/// A layout algorithm that maps a flat collection of items to vertical pixel
/// positions within a scroll container. The `Virtualizer` delegates all
/// vertical geometric queries to its `VirtualLayout` implementation.
pub trait VirtualLayout {
    /// Returns the range of item indices `[start, end)` that are visible
    /// (or partially visible) given the current scroll state.
    ///
    /// `scroll_offset` is the number of pixels scrolled from the top of the
    /// container. `viewport_height` is the visible height of the scroll
    /// container. The returned range does **not** include overscan — the
    /// `Virtualizer` adds overscan padding around the range.
    ///
    /// ```rust
    /// let range = layout.visible_range(320.0, 600.0);
    /// // e.g., 8..23 — items 8 through 22 overlap the viewport.
    /// ```
    fn visible_range(&self, scroll_offset: f64, viewport_height: f64) -> Range<usize>;

    /// Returns the Y-axis pixel offset for the item at `index`, measured from
    /// the top of the scroll content area. Adapters use this value as
    /// `transform: translateY({offset}px)` or `top: {offset}px` on each
    /// rendered item.
    fn item_offset(&self, index: usize) -> f64;

    /// Returns the total scrollable height of the content area. The adapter
    /// sets this as the `height` of the inner scroll sentinel element so
    /// that the browser renders a correctly-sized scrollbar track.
    fn total_height(&self) -> f64;

    /// Reports the actual measured pixel height of the item at `index`.
    ///
    /// For variable-height layouts, the adapter calls this after each render
    /// pass once the DOM has been laid out (typically inside a
    /// `requestAnimationFrame` or `ResizeObserver` callback). The layout
    /// caches measured heights and uses them in subsequent `visible_range`,
    /// `item_offset`, and `total_height` calculations. For items that have
    /// not been measured, the layout falls back to an estimated height.
    ///
    /// Fixed-height layouts may ignore this call.
    fn report_item_height(&mut self, index: usize, height: f64);

    /// Returns the `scroll_top` value that would bring the item at `index`
    /// into view, aligned to the top of the viewport.
    ///
    /// Callers that need center or bottom alignment adjust the returned
    /// value using the item's height and the viewport height. This method
    /// is the foundation for `Virtualizer::scroll_to_index`.
    fn scroll_to_index(&self, index: usize) -> f64 {
        self.item_offset(index)
    }

    /// The total number of items known to the layout. Must match the
    /// collection size (or estimated total for async collections).
    fn item_count(&self) -> usize;
}

/// Optional horizontal extension implemented by layouts that support
/// inline-axis virtualization.
pub trait HorizontalVirtualLayout: VirtualLayout {

    /// Returns the range of item indices visible given horizontal scroll state.
    fn visible_range_horizontal(&self, scroll_offset: f64, viewport_width: f64) -> Range<usize>;

    /// Returns the X-axis pixel offset for the item at `index`, measured from
    /// the inline-start edge of the scroll content area.
    fn item_offset_x(&self, index: usize) -> f64;

    /// Returns the total scrollable width of the content area.
    fn total_width(&self) -> f64;

    /// Reports the actual measured pixel width of the item at `index`.
    fn report_item_width(&mut self, index: usize, width: f64) {
        let _ = (index, width);
        // No-op for layouts that don't use horizontal variable widths.
    }
}
````

**Scroll anchoring.** When items above the viewport change height (e.g., after images load or content expands), the visible items would shift downward, causing a jarring jump. To prevent this, the adapter tracks an _anchor item_ — the first item whose top edge is at or below `scroll_top`. After a layout recalculation triggered by `report_item_height`:

1. Compute the new `item_offset` for the anchor item.
2. Calculate the delta: `new_offset - old_offset`.
3. Adjust `scroll_top` by the delta so the anchor item remains at the same visual position.

This logic lives in the adapter layer (not in `VirtualLayout` itself) because it requires writing to the DOM's `scrollTop` property.

**Integration with `aria-activedescendant`.** Virtualized lists use `aria-activedescendant` on the scroll container to point at the currently focused option. Because the focused item may be scrolled out of view and removed from the DOM, the `Virtualizer` keeps the focused item in the rendered range via `focused_index` (see the `Virtualizer` struct above). The adapter must:

- Set `aria-activedescendant` on the list container to the `id` of the focused item element.
- Ensure the focused item element is present in the DOM at all times (guaranteed by `visible_range()` including `focused_index`).
- On keyboard navigation (Arrow keys), update `focused_index` _before_ re-rendering so the new target is included in the visible range.

**Performance considerations.**

- **Estimated heights for unmeasured items.** Variable-height layouts must provide a reasonable `estimated_item_height` (typically the average measured height so far or a static default). Poor estimates cause scrollbar thumb jitter — the total height changes as items are measured. Implementations should periodically recompute the estimate as `sum(measured) / count(measured)`.
- **Height cache eviction.** For very large collections (100k+ items), consider capping the measured-height cache and evicting entries far from the current scroll position. Evicted items revert to the estimated height.
- **Batched height reports.** Adapters should batch multiple `report_item_height` calls into a single layout recalculation rather than recalculating after each individual report.

````rust
impl Virtualizer {
    /// Create a new virtualizer with the given layout strategy.
    pub fn new(total_count: usize, layout: LayoutStrategy) -> Self {
        Self {
            total_count,
            layout,
            viewport_height: 0.0,
            viewport_width: 0.0,
            scroll_top: 0.0,
            scroll_left: 0.0,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            overscan: DEFAULT_OVERSCAN,
            measured_heights: BTreeMap::new(),
            focused_index: None,
        }
    }

    /// Update scroll position and viewport dimensions in-place.
    /// O(1) — suitable for scroll-event-driven updates at 60fps.
    /// For horizontal virtualization, pass the normalized scroll_left value (see §6.4).
    pub fn set_scroll_state_mut(
        &mut self, scroll_top: f64, scroll_left: f64,
        viewport_height: f64, viewport_width: f64,
    ) {
        self.scroll_top = scroll_top;
        self.scroll_left = scroll_left;
        self.viewport_height = viewport_height;
        self.viewport_width = viewport_width;
    }

    /// Records a measured height for a specific item (mutable, zero-copy).
    /// This is the preferred API for frequent per-item updates.
    pub fn report_item_height_mut(&mut self, index: usize, height: f64) {
        self.measured_heights.insert(index, height);
    }

    /// Record the measured pixel height of item at `index` (for variable
    /// height layouts). Returns a new `Virtualizer`; does not mutate.
    /// Prefer `report_item_height_mut` for per-item callbacks to avoid O(n) cloning.
    pub fn report_item_height(&self, index: usize, height: f64) -> Self {
        let mut new = self.clone();
        new.report_item_height_mut(index, height);
        new
    }

    /// Applies a collection update that may have changed flat item indices.
    ///
    /// Because measured heights and focus are keyed by flat index, adapters
    /// must call this on inserts, removals, filtering, and reorders before
    /// reusing the `Virtualizer` for the updated collection. The method
    /// updates `total_count`, clears the measured-height cache, and clears
    /// `focused_index`.
    pub fn apply_collection_change_mut(&mut self, total_count: usize) {
        self.total_count = total_count;
        self.measured_heights.clear();
        self.focused_index = None;
    }

    /// Immutable wrapper around `apply_collection_change_mut`.
    pub fn apply_collection_change(&self, total_count: usize) -> Self {
        let mut new = self.clone();
        new.apply_collection_change_mut(total_count);
        new
    }

    /// The range of flat indices [start, end) that should be rendered,
    /// including overscan. Components iterate `start..end` and render
    /// `collection.get_by_index(i)` for each `i`.
    pub fn visible_range(&self) -> core::ops::Range<usize> {
        let viewport_extent = match self.orientation {
            Orientation::Vertical => self.viewport_height,
            Orientation::Horizontal => self.viewport_width,
        };

        if self.total_count == 0 || viewport_extent == 0.0 {
            return 0..0;
        }

        let max_scroll = (self.total_main_axis_extent() - viewport_extent).max(0.0);
        let scroll_offset = match self.orientation {
            Orientation::Vertical => self.scroll_top,
            Orientation::Horizontal => self.scroll_left,
        }.clamp(0.0, max_scroll);

        let (first_visible, last_visible) = match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => {
                let first = (scroll_offset / item_height).floor() as usize;
                let last  = ((scroll_offset + viewport_extent) / item_height).ceil() as usize;
                (first, last)
            }
            LayoutStrategy::VariableHeight { estimated_item_height } => {
                self.variable_height_range(*estimated_item_height, scroll_offset, viewport_extent)
            }
            LayoutStrategy::Grid { item_height, columns } => {
                let cols = columns.get();
                let row_start = (scroll_offset / item_height).floor() as usize;
                let row_end   = ((scroll_offset + viewport_extent) / item_height).ceil() as usize;
                (row_start * cols, (row_end * cols).min(self.total_count))
            }
            LayoutStrategy::GridLayout { min_item_width, min_item_height, gap, .. } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);
                let row_stride = main_size + gap;
                let row_start = (scroll_offset / row_stride).floor() as usize;
                let row_end   = ((scroll_offset + viewport_extent) / row_stride).ceil() as usize;
                (row_start * cols, (row_end * cols).min(self.total_count))
            }
            LayoutStrategy::WaterfallLayout { min_item_width, min_item_height, gap, .. } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let positions = self.waterfall_positions(cross_size, main_size, *gap);
                let mut first = self.total_count;
                let mut last  = 0;
                for (i, &y) in positions.iter().enumerate() {
                    let h = self.measured_heights.get(&i).copied().unwrap_or(main_size);
                    if y + h > scroll_offset && y < scroll_offset + viewport_extent {
                        first = first.min(i);
                        last  = last.max(i + 1);
                    }
                }
                if first > last { first = 0; last = 0; }
                (first, last)
            }
            LayoutStrategy::TableLayout { row_height, header_height, column_widths, row_gap } => {
                match self.orientation {
                    Orientation::Vertical => {
                        let row_stride = row_height + row_gap;
                        let data_offset = (scroll_offset - header_height).max(0.0);
                        let data_end    = (scroll_offset + viewport_extent - header_height).max(0.0);
                        let first = (data_offset / row_stride).floor() as usize;
                        let last  = (data_end / row_stride).ceil() as usize;
                        (first, last.min(self.total_count))
                    }
                    Orientation::Horizontal => {
                        let mut cumulative = 0.0_f64;
                        let mut first = column_widths.len();
                        let mut found_first = false;
                        let mut last = column_widths.len();
                        for (i, &w) in column_widths.iter().enumerate() {
                            if cumulative + w > scroll_offset && !found_first {
                                first = i; found_first = true;
                            }
                            cumulative += w;
                            if cumulative >= scroll_offset + viewport_extent {
                                last = i + 1; break;
                            }
                        }
                        (first, last.min(column_widths.len()))
                    }
                }
            }
        };

        let mut start = first_visible.saturating_sub(self.overscan);
        let mut end   = last_visible
            .saturating_add(self.overscan)
            .min(self.total_count);

        // Ensure the focused item is always included in the rendered range,
        // even when scrolled out of view (see §6.2 Focused Element Persistence).
        if let Some(fi) = self.focused_index {
            if fi < self.total_count {
                start = start.min(fi);
                end   = end.max(fi + 1);
            }
        }

        start..end
    }

    /// Set the focused item index. The adapter calls this when keyboard focus
    /// changes to a virtualized item. Pass `None` when focus leaves the
    /// virtualized container. Returns a new `Virtualizer`; does not mutate.
    pub fn set_focused_index(&self, index: Option<usize>) -> Self {
        Self {
            focused_index: index,
            ..self.clone()
        }
    }

    /// The pixel offset from the top of the scroll container for item at
    /// `index`. Used as `transform: translateY({offset}px)` or `top: {offset}px`.
    pub fn item_offset_px(&self, index: usize) -> f64 {
        match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => {
                index as f64 * item_height
            }
            LayoutStrategy::VariableHeight { estimated_item_height } => {
                (0..index).map(|i| {
                    self.measured_heights
                        .get(&i)
                        .copied()
                        .unwrap_or(*estimated_item_height)
                }).sum()
            }
            LayoutStrategy::Grid { item_height, columns } => {
                (index / columns.get()) as f64 * item_height
            }
            LayoutStrategy::GridLayout { min_item_width, min_item_height, gap, .. } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);
                (index / cols) as f64 * (main_size + gap)
            }
            LayoutStrategy::WaterfallLayout { min_item_width, min_item_height, gap, .. } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let positions = self.waterfall_positions(cross_size, main_size, *gap);
                positions.get(index).copied().unwrap_or(0.0)
            }
            LayoutStrategy::TableLayout { row_height, header_height, column_widths, row_gap } => {
                match self.orientation {
                    Orientation::Vertical => header_height + index as f64 * (row_height + row_gap),
                    Orientation::Horizontal => column_widths.iter().take(index).sum(),
                }
            }
        }
    }

    /// Total scroll extent of the list on the active axis. Adapters use this
    /// to size the inner spacer so the browser renders the correct scrollbar.
    ///
    /// **Performance note:** For `VariableHeight`, this method iterates all items O(n).
    /// Callers in scroll handlers should cache the result rather than calling per-frame.
    pub fn total_height_px(&self) -> f64 {
        match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => {
                self.total_count as f64 * item_height
            }
            LayoutStrategy::VariableHeight { estimated_item_height } => {
                (0..self.total_count).map(|i| {
                    self.measured_heights
                        .get(&i)
                        .copied()
                        .unwrap_or(*estimated_item_height)
                }).sum()
            }
            LayoutStrategy::Grid { item_height, columns } => {
                let cols = columns.get();
                let rows = (self.total_count + cols - 1) / cols;
                rows as f64 * item_height
            }
            LayoutStrategy::GridLayout { min_item_width, min_item_height, gap, .. } => {
                if self.total_count == 0 { return 0.0; }
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);
                let rows = (self.total_count + cols - 1) / cols;
                rows as f64 * (main_size + gap) - gap
            }
            LayoutStrategy::WaterfallLayout { min_item_width, min_item_height, gap, .. } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                self.waterfall_total_height(cross_size, main_size, *gap)
            }
            LayoutStrategy::TableLayout { row_height, header_height, row_gap, .. } => {
                if self.total_count == 0 { return *header_height; }
                header_height + self.total_count as f64 * (row_height + row_gap) - row_gap
            }
        }
    }

    /// Compute the scroll offset needed to bring item `index` into view.
    /// Used by `scroll_to_index` and `scroll_to_key` (after resolving key →
    /// index via the collection).
    ///
    /// `align` controls which edge of the item aligns with the viewport edge
    /// when the item is not already visible.
    pub fn scroll_top_for_index(&self, index: usize, align: ScrollAlign) -> f64 {
        let offset = self.item_offset_px(index);
        let extent = match &self.layout {
            LayoutStrategy::FixedHeight { item_height }              => *item_height,
            LayoutStrategy::Grid { item_height, .. }                 => *item_height,
            LayoutStrategy::VariableHeight { estimated_item_height } => {
                self.measured_heights.get(&index).copied().unwrap_or(*estimated_item_height)
            }
            LayoutStrategy::GridLayout { min_item_width, min_item_height, .. } => {
                self.axis_sizes(*min_item_width, *min_item_height).0
            }
            LayoutStrategy::WaterfallLayout { min_item_width, min_item_height, .. } => {
                let main_size = self.axis_sizes(*min_item_width, *min_item_height).0;
                self.measured_heights.get(&index).copied().unwrap_or(main_size)
            }
            LayoutStrategy::TableLayout { row_height, column_widths, .. } => {
                match self.orientation {
                    Orientation::Vertical => *row_height,
                    Orientation::Horizontal => column_widths.get(index).copied().unwrap_or(*row_height),
                }
            }
        };
        let viewport_extent = self.viewport_extent();
        let max_scroll = (self.total_main_axis_extent() - viewport_extent).max(0.0);
        let clamped_scroll_offset = self.clamped_scroll_offset(viewport_extent);
        let item_end = offset + extent;

        let target_offset = match align {
            ScrollAlign::Auto => {
                if offset < clamped_scroll_offset {
                    offset
                } else if item_end > clamped_scroll_offset + viewport_extent {
                    item_end - viewport_extent
                } else {
                    clamped_scroll_offset
                }
            }
            ScrollAlign::Top    => offset,
            ScrollAlign::Bottom => (item_end - viewport_extent).max(0.0),
            ScrollAlign::Center => (offset - (viewport_extent - extent) / 2.0).max(0.0),
        };

        target_offset.clamp(0.0, max_scroll)
    }

    /// Programmatically scroll to the item at `index` with the given alignment.
    ///
    /// Returns the computed scroll position. The adapter MUST apply this value
    /// to the scroll container's `scrollTop` (or equivalent). This is a
    /// convenience wrapper around `scroll_top_for_index`.
    ///
    /// ```rust
    /// let scroll_pos = virtualizer.scroll_to_index(42, ScrollAlign::Center);
    /// // Adapter applies: container.set_scroll_top(scroll_pos);
    /// ```
    pub fn scroll_to_index(&self, index: usize, align: ScrollAlign) -> f64 {
        self.scroll_top_for_index(index, align)
    }

    /// Programmatically scroll to the item with the given `key`.
    ///
    /// Resolves the key to a flat index via the provided `key_to_index`
    /// lookup function, then delegates to `scroll_to_index`. Returns `None`
    /// if the key is not found in the collection.
    ///
    /// ```rust
    /// let scroll_pos = virtualizer.scroll_to_key(&Key::from("item-7"), ScrollAlign::Auto, |key| {
    ///     collection.get(key).map(|n| n.index)
    /// });
    /// ```
    pub fn scroll_to_key(
        &self,
        key: &Key,
        align: ScrollAlign,
        key_to_index: impl Fn(&Key) -> Option<usize>,
    ) -> Option<f64> {
        key_to_index(key).map(|index| self.scroll_to_index(index, align))
    }

    /// Computes the scroll adjustment needed to keep `anchor_index` at the
    /// same visual position after a layout change (see §6.6).
    ///
    /// `old_offset` is `item_offset_px(anchor_index)` recorded **before** the change.
    pub fn scroll_adjustment_for_anchor(&self, anchor_index: usize, old_offset: f64) -> f64 {
        self.item_offset_px(anchor_index) - old_offset
    }

    fn viewport_extent(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.viewport_height,
            Orientation::Horizontal => self.viewport_width,
        }
    }

    fn scroll_offset(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.scroll_top,
            Orientation::Horizontal => self.scroll_left,
        }
    }

    fn total_main_axis_extent(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.total_height_px(),
            Orientation::Horizontal => match &self.layout {
                LayoutStrategy::TableLayout { column_widths, .. } => {
                    column_widths.iter().sum()
                }
                _ => self.total_height_px(),
            },
        }
    }

    fn variable_height_range(
        &self,
        estimated: f64,
        scroll_offset: f64,
        viewport_extent: f64,
    ) -> (usize, usize) {
        let mut cumulative = 0.0_f64;
        let mut first = 0;
        let mut found_first = false;
        let mut last  = self.total_count;

        for i in 0..self.total_count {
            let h = self.measured_heights.get(&i).copied().unwrap_or(estimated);
            if cumulative + h > scroll_offset && !found_first {
                first = i;
                found_first = true;
            }
            cumulative += h;
            if cumulative >= scroll_offset + viewport_extent {
                last = i + 1;
                break;
            }
        }
        (first, last)
    }

    /// Returns `(main_axis_item_size, cross_axis_item_size)` for a responsive
    /// grid or waterfall layout based on the current orientation.
    /// Vertical: main = height, cross = width.  Horizontal: main = width, cross = height.
    const fn axis_sizes(&self, min_item_width: f64, min_item_height: f64) -> (f64, f64) {
        match self.orientation {
            Orientation::Vertical  => (min_item_height, min_item_width),
            Orientation::Horizontal => (min_item_width, min_item_height),
        }
    }

    /// Computes the responsive column count for `GridLayout` and
    /// `WaterfallLayout` from the cross-axis viewport extent.
    /// `cross_item_size` is the item dimension along the cross axis.
    fn responsive_columns(&self, cross_item_size: f64, gap: f64) -> usize {
        let cross = match self.orientation {
            Orientation::Vertical => self.viewport_width,
            Orientation::Horizontal => self.viewport_height,
        };
        let stride = cross_item_size + gap;
        if cross <= 0.0 || stride <= 0.0 { return 1; }
        ((cross + gap) / stride).floor().max(1.0) as usize
    }

    /// Computes the main-axis offset for every item in a waterfall (masonry)
    /// layout. `cross_item_size` and `main_item_size` are the orientation-
    /// resolved item dimensions (call `axis_sizes` first).
    fn waterfall_positions(
        &self, cross_item_size: f64, main_item_size: f64, gap: f64,
    ) -> Vec<f64> {
        let columns = self.responsive_columns(cross_item_size, gap);
        let mut col_heights = vec![0.0_f64; columns];
        let mut positions   = Vec::with_capacity(self.total_count);

        for i in 0..self.total_count {
            let (min_col, _) = col_heights.iter().enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));
            let y = col_heights[min_col];
            positions.push(y);
            let h = self.measured_heights.get(&i).copied().unwrap_or(main_item_size);
            col_heights[min_col] = y + h + gap;
        }
        positions
    }

    /// Total main-axis extent of a waterfall layout (tallest column minus trailing gap).
    fn waterfall_total_height(
        &self, cross_item_size: f64, main_item_size: f64, gap: f64,
    ) -> f64 {
        if self.total_count == 0 { return 0.0; }
        let columns = self.responsive_columns(cross_item_size, gap);
        let mut col_heights = vec![0.0_f64; columns];
        for i in 0..self.total_count {
            let (min_col, _) = col_heights.iter().enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));
            let h = self.measured_heights.get(&i).copied().unwrap_or(main_item_size);
            col_heights[min_col] += h + gap;
        }
        let tallest = col_heights.iter().copied().fold(0.0_f64, f64::max);
        (tallest - gap).max(0.0)
    }
}

/// How to align the target item within the viewport when scrolling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScrollAlign {
    /// Only scroll if the item is not already visible. Prefer minimal movement.
    #[default]
    Auto,
    /// Align the item's top edge with the viewport top.
    Top,
    /// Align the item's bottom edge with the viewport bottom.
    Bottom,
    /// Center the item in the viewport.
    Center,
}

````

### 6.4 Virtualizer Integration API

Components that support virtualization (Listbox, Select, Combobox, Table,
TreeView, GridList) integrate the `Virtualizer` through these patterns:

**Container and item height setup:**

- The adapter measures the scroll container's height via a `ResizeObserver` and
  calls `virtualizer.set_scroll_state_mut(scroll_top, 0.0, viewport_height, viewport_width)`.
- For `FixedHeight`, the item height is a prop (e.g., `item_height: 48.0`).
- For `VariableHeight`, estimated height is a prop; actual heights are reported
  after render via `virtualizer.report_item_height(index, measured_px)`.

**Visible range calculation:**

- On each scroll event, the adapter calls `virtualizer.visible_range()` to get
  the `Range<usize>` of item indices to render.
- The adapter iterates `range.start..range.end` and renders
  `collection.get_by_index(i)` for each `i`, positioned at
  `virtualizer.item_offset_px(i)`.

**Collection updates:**

- Before reusing a `Virtualizer` after inserts, removals, filtering, or reorders,
  the adapter calls `virtualizer.apply_collection_change_mut(new_total_count)`.
- This invalidates index-based measured heights and the stored focused index so
  stale row measurements and stale focus targets are not applied to different
  items after flat indices shift.

**Scroll event handling:**

- The adapter attaches a scroll listener to the container and calls
  `virtualizer.set_scroll_state_mut(container.scroll_top(), container.scroll_left(), container.client_height(), container.client_width())`
  on every scroll event (throttled via `requestAnimationFrame`).

**ARIA `aria-setsize` / `aria-posinset` computation:**

- Each rendered item receives `aria-setsize` = `virtualizer.total_count` and
  `aria-posinset` = `index + 1` (1-based). This tells screen readers the total
  list size even though only a subset is in the DOM.

```rust
// Example: Listbox adapter wiring (pseudocode)
fn render_virtualized_listbox(ctx: &ListboxContext) {
    let range = ctx.virtualizer.visible_range();
    let total = ctx.virtualizer.total_count;
    for i in range {
        let node = ctx.collection.get_by_index(i);
        let offset = ctx.virtualizer.item_offset_px(i);
        // render item at absolute position `offset` with:
        //   aria-setsize = total
        //   aria-posinset = i + 1
    }
}
```

### 6.5 Keyboard Navigation with Virtualization

When the user presses Arrow keys, the focused item may not currently be rendered. The component must:

1. Compute the next key via `collection.key_after(focused_key)` or `key_before`.
2. Look up the flat index of the new key via `collection.get(new_key).map(|n| n.index)`.
3. Call `virtualizer.scroll_top_for_index(new_index, ScrollAlign::Auto)` to compute the required scroll position.
4. Emit a `ScrollTo { scroll_top }` effect that sets the DOM scroll container's `scrollTop` via `ars-dom`.
5. After the scroll settles, the item is now rendered and can receive DOM focus.

This means keyboard navigation through unrendered items produces a scroll rather than a direct `focus()` call. The machine tracks the _intended_ focused key even before the DOM element exists.

```rust
// Pseudocode: inside Listbox::transition for ArrowDown

Event::KeyDown(KeyboardEvent { key: KeyboardKey::ArrowDown, .. }) => {
    let next_key = ctx.collection.key_after_no_wrap(&ctx.focused_key?)?;
    if ctx.selection_state.is_disabled(&next_key) {
        // Skip disabled — recurse or stop.
        return None;
    }
    let next_index = ctx.collection.get(&next_key)?.index;
    let scroll_top = ctx.virtualizer.scroll_top_for_index(next_index, ScrollAlign::Auto);
    let need_scroll = (scroll_top - ctx.virtualizer.scroll_top).abs() > 0.5;

    let plan = TransitionPlan::context_only(move |ctx| {
        ctx.selection_state = ctx.selection_state.set_focus(next_key.clone());
        if need_scroll {
            ctx.virtualizer.set_scroll_state_mut(scroll_top, ctx.virtualizer.scroll_left, ctx.virtualizer.viewport_height, ctx.virtualizer.viewport_width);
        }
    });
    Some(if need_scroll {
        plan.with_effect(PendingEffect::new("scroll-to-focused", move |ctx, _props, _send| {
            let platform = use_platform_effects();
            platform.set_scroll_top(&ctx.scroll_container_id, scroll_top);
            no_cleanup()
        }))
    } else {
        plan
    })
}
```

### 6.6 Scroll Position Maintenance

When the collection changes (items added, reordered, or filtered) while the user is scrolled to a non-zero position, the virtualizer must adjust `scroll_top` to keep the previously focused item visible. The strategy:

1. Before applying the collection change, record the focused item's `item_offset_px(focused_index)`.
2. After applying the change, look up the focused item's new index.
3. Compute the delta: `new_offset - old_offset`.
4. Adjust `scroll_top` by the delta.

This prevents content jumping when, for example, new items are prepended (infinite scroll upward) or when a filter narrows the collection.

### 6.7 RTL Scroll Normalization

Browser implementations of `scrollLeft` for RTL content are inconsistent:

| Browser      | `scrollLeft` range for RTL                               |
| ------------ | -------------------------------------------------------- |
| Chrome, Edge | `0` (far right) to `-maxScroll` (far left)               |
| Firefox      | `0` (far right) to `-maxScroll` (far left)               |
| Safari       | `0` (far left) to `maxScroll` (far right) — **reversed** |

The adapter MUST normalize `scrollLeft` to a consistent `0..maxScroll` range (measuring from the inline-start edge) before passing it to `Virtualizer.scroll_left`:

```rust
// ars-collections/src/virtualization.rs

/// Browser convention for RTL `scrollLeft` values.
///
/// Adapters detect the convention once at startup (e.g., by writing a
/// known `scrollLeft` to a hidden RTL element and reading back the sign)
/// and reuse the result for all subsequent normalization calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtlScrollMode {
    /// Chrome, Edge, Firefox: `scrollLeft` is `0` at inline-start
    /// (far-right), negative toward inline-end (far-left).
    /// Range: `-max..0`.
    Negative,
    /// Safari: `scrollLeft` is `0` at far-left (inline-end in RTL),
    /// positive toward far-right (inline-start).
    /// Range: `0..max`.
    Positive,
}

/// Normalizes a browser's `scrollLeft` value for RTL content to a
/// consistent `0..max_scroll` range measured from the inline-start edge.
///
/// `mode` identifies the browser's scroll convention (see `RtlScrollMode`).
/// The adapter detects this once and passes it on every scroll event.
///
/// For LTR content, `scrollLeft` is already `0..max` and does not need
/// normalization.
pub fn normalize_scroll_left_rtl(
    raw: f64, scroll_width: f64, client_width: f64, mode: RtlScrollMode,
) -> f64 {
    let max_scroll = (scroll_width - client_width).max(0.0);
    match mode {
        RtlScrollMode::Negative => {
            // Chrome/Firefox: raw is -max..0. Negate to get 0..max.
            raw.abs().clamp(0.0, max_scroll)
        }
        RtlScrollMode::Positive => {
            // Safari: raw is 0 (inline-end) to max (inline-start).
            // Convert to inline-start distance: max - raw.
            (max_scroll - raw).clamp(0.0, max_scroll)
        }
    }
}
```

For LTR content, `scrollLeft` is always `0..maxScroll` across all browsers and can be passed directly.

---

## 7. Filtering and Sorting

### 7.1 FilteredCollection

Filtering wraps an existing `Collection<T>` implementation and applies a predicate, producing a view that delegates all traversal to a subset of the inner collection's nodes. The inner collection is not modified.

```rust
// ars-collections/src/filtered_collection.rs

use alloc::vec::Vec;
use crate::{key::Key, node::{Node, NodeType}, Collection};

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

    /// Base `node.index` values of visible nodes.
    /// Used for O(log n) membership checks in `get()` and `children_of()`.
    visible_base_indices: alloc::collections::BTreeSet<usize>,

    /// Wrapper positions (into `inner`) of visible nodes, in traversal order.
    /// Used by `get_by_index()` and `nodes()` — these are the coordinates
    /// that `inner.get_by_index()` expects.
    visible_positions: Vec<usize>,

    /// Maps base `node.index` → index into `visible_positions`, for O(log n)
    /// lookup when navigating from a key.
    base_to_visible_pos: alloc::collections::BTreeMap<usize, usize>,

    /// Cached index into `visible_positions` of the first focusable item.
    first_focusable: Option<usize>,

    /// Cached index into `visible_positions` of the last focusable item.
    last_focusable: Option<usize>,

    _phantom: core::marker::PhantomData<T>,
}

impl<'a, T: Clone, C: Collection<T>> FilteredCollection<'a, T, C> {
    /// Apply `predicate` to each item node in `inner`.
    ///
    /// Section/Header/Separator nodes are included only when at least one of
    /// their children passes the predicate.
    pub fn new(inner: &'a C, predicate: impl Fn(&Node<T>) -> bool) -> Self {
        // First pass: find all item nodes that pass, keyed by base index.
        let passing: alloc::collections::BTreeSet<usize> = inner
            .nodes()
            .filter(|n| n.is_focusable() && predicate(n))
            .map(|n| n.index)
            .collect();

        // Second pass: collect (wrapper_position, base_index) for visible nodes.
        // Structural nodes are included based on the scope they belong to:
        // - Section nodes: scope is their direct children.
        // - Header/Separator inside a section: scope is the parent section's
        //   children (they have no children of their own).
        // - Top-level Header/Separator: scope is all top-level items. Without
        //   this branch a no-op predicate (`|_| true`) would drop top-level
        //   separators and silently change the collection shape.
        let visible_data: Vec<(usize, usize)> = inner
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
                        m.is_focusable()
                            && m.parent_key.is_none()
                            && passing.contains(&m.index)
                    }),
                }
            })
            .map(|(wrapper_pos, n)| (wrapper_pos, n.index))
            .collect();

        let visible_positions: Vec<usize> = visible_data.iter().map(|&(wp, _)| wp).collect();
        let visible_base_indices = visible_data.iter().map(|&(_, bi)| bi).collect();
        let base_to_visible_pos = visible_data.iter().enumerate()
            .map(|(vis_idx, &(_, base_idx))| (base_idx, vis_idx)).collect();

        let first_focusable = visible_positions.iter().enumerate()
            .find_map(|(vi, &wp)| inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|_| vi));
        let last_focusable = visible_positions.iter().enumerate().rev()
            .find_map(|(vi, &wp)| inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|_| vi));

        Self { inner, visible_base_indices, visible_positions, base_to_visible_pos,
               first_focusable, last_focusable, _phantom: core::marker::PhantomData }
    }
}

impl<'a, T: Clone, C: Collection<T>> Collection<T> for FilteredCollection<'a, T, C> {
    fn size(&self) -> usize { self.visible_positions.len() }

    fn get(&self, key: &Key) -> Option<&Node<T>> {
        let node = self.inner.get(key)?;
        if self.visible_base_indices.contains(&node.index) {
            Some(node)
        } else {
            None
        }
    }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.visible_positions.get(index).and_then(|&wp| self.inner.get_by_index(wp))
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
        let &vis_pos = self.base_to_visible_pos.get(&node.index)?;
        self.visible_positions[vis_pos + 1..]
            .iter()
            .find_map(|&wp| self.inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|n| &n.key))
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let node = self.inner.get(key)?;
        let &vis_pos = self.base_to_visible_pos.get(&node.index)?;
        self.visible_positions[..vis_pos]
            .iter()
            .rev()
            .find_map(|&wp| self.inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|n| &n.key))
    }

    fn keys(&self) -> impl Iterator<Item = &Key> {
        self.nodes().map(|n| &n.key)
    }

    fn nodes(&self) -> impl Iterator<Item = &Node<T>> {
        self.visible_positions
            .iter()
            .filter_map(|&wp| self.inner.get_by_index(wp))
    }

    fn children_of(&self, parent_key: &Key) -> impl Iterator<Item = &Node<T>> {
        self.inner.children_of(parent_key)
            .filter(|n| self.visible_base_indices.contains(&n.index))
    }
}
```

### 7.2 SortedCollection

Sorting produces a view that presents nodes in a different order without modifying the source. Locale-aware string comparison uses the `StringCollator` from `ars-i18n` (see `04-internationalization.md` §8).

````rust
// ars-collections/src/sorted_collection.rs

use alloc::vec::Vec;
use crate::{key::Key, node::{Node, NodeType}, Collection};

/// The direction of a sort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Sort in ascending order (smallest first).
    Ascending,
    /// Sort in descending order (largest first).
    Descending,
}

impl core::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
/// ```rust
/// let collator = ars_i18n::StringCollator::new(&locale, Default::default());
/// let sorted = SortedCollection::new(&collection, |a, b| {
///     collator.compare(&a.text_value, &b.text_value)
/// });
/// ```
/// Clone — this struct holds only a reference and sorted indices, no closures.
/// The comparator is consumed at construction time and not retained.
pub struct SortedCollection<'a, T, C>
where
    C: Collection<T>,
{
    inner: &'a C,
    /// Wrapper positions (into `inner`) in sorted traversal order.
    sorted_positions: Vec<usize>,
    /// Maps base `node.index` → index into `sorted_positions`.
    base_to_sorted_pos: alloc::collections::BTreeMap<usize, usize>,
    /// Cached index into `sorted_positions` of the first focusable item.
    first_focusable: Option<usize>,
    /// Cached index into `sorted_positions` of the last focusable item.
    last_focusable: Option<usize>,
    _phantom: core::marker::PhantomData<T>,
}

impl<'a, T: Clone, C: Collection<T>> SortedCollection<'a, T, C> {
    /// Construct a sorted view.
    ///
    /// `comparator` is called only for `Item` nodes. Section, Header, and
    /// Separator nodes are left in their original relative positions within
    /// their grouping. For flat (non-sectioned) collections, all nodes are
    /// reordered.
    pub fn new(
        inner: &'a C,
        comparator: impl Fn(&Node<T>, &Node<T>) -> core::cmp::Ordering,
    ) -> Self {
        // Collect item wrapper positions and sort them by comparator.
        let mut item_positions: Vec<usize> = inner
            .nodes()
            .enumerate()
            .filter(|(_, n)| n.is_focusable())
            .map(|(pos, _)| pos)
            .collect();

        item_positions.sort_by(|&a, &b| {
            let na = inner.get_by_index(a)
                .expect("sort index must be within collection bounds");
            let nb = inner.get_by_index(b)
                .expect("sort index must be within collection bounds");
            comparator(na, nb)
        });

        // Interleave structural nodes back into sorted order.
        //
        // For flat (non-sectioned) collections: all nodes are Items, so
        // item_indices IS the final sorted order.
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
        let has_sections = inner.nodes().any(|n| n.is_structural());

        let sorted_positions = if has_sections {
            // Phase 1: assign each item to a contiguous-run group, keyed by
            // wrapper position.
            let mut item_to_group: alloc::collections::BTreeMap<usize, usize> =
                alloc::collections::BTreeMap::new();
            let mut next_group: usize = 0;
            let mut current_parent: Option<Option<Key>> = None;

            for (pos, node) in inner.nodes().enumerate() {
                match node.node_type {
                    NodeType::Section | NodeType::Separator => {
                        // Section and Separator both act as hard run boundaries.
                        // Items on opposite sides MUST NOT merge into one sorted
                        // group, even when they share the same parent_key, or
                        // sorting would pull items across the visual divider.
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
            let mut groups: alloc::collections::BTreeMap<usize, Vec<usize>> =
                alloc::collections::BTreeMap::new();
            for &wp in &item_positions {
                let group = *item_to_group.get(&wp)
                    .expect("item position must have been assigned a group");
                groups.entry(group).or_default().push(wp);
            }

            // Phase 3: walk original order, emit structural nodes in place,
            // emit each group's sorted items on first encounter. Both map
            // lookups are guaranteed by Phase 1/2 construction invariants.
            let mut group_emitted: alloc::collections::BTreeSet<usize> =
                alloc::collections::BTreeSet::new();
            let mut result = Vec::with_capacity(inner.size());

            for (pos, node) in inner.nodes().enumerate() {
                match node.node_type {
                    NodeType::Section | NodeType::Header | NodeType::Separator => {
                        result.push(pos);
                    }
                    NodeType::Item => {
                        let group = *item_to_group.get(&pos)
                            .expect("item position must have been assigned a group");
                        if group_emitted.insert(group) {
                            let items = groups.get(&group)
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
        let base_to_sorted_pos: alloc::collections::BTreeMap<usize, usize> = sorted_positions
            .iter().enumerate()
            .filter_map(|(sp_idx, &wp)| inner.get_by_index(wp).map(|n| (n.index, sp_idx)))
            .collect();

        let first_focusable = sorted_positions.iter().enumerate()
            .find_map(|(si, &wp)| inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|_| si));
        let last_focusable = sorted_positions.iter().enumerate().rev()
            .find_map(|(si, &wp)| inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|_| si));

        Self { inner, sorted_positions, base_to_sorted_pos, first_focusable, last_focusable, _phantom: core::marker::PhantomData }
    }
}

impl<'a, T, C: Collection<T>> Collection<T> for SortedCollection<'a, T, C> {
    fn size(&self) -> usize { self.sorted_positions.len() }

    fn get(&self, key: &Key) -> Option<&Node<T>> { self.inner.get(key) }

    fn get_by_index(&self, index: usize) -> Option<&Node<T>> {
        self.sorted_positions.get(index).and_then(|&wp| self.inner.get_by_index(wp))
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
        let &sorted_pos = self.base_to_sorted_pos.get(&node.index)?;
        self.sorted_positions[sorted_pos + 1..].iter().find_map(|&wp| {
            self.inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|n| &n.key)
        })
    }

    fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
        let node = self.inner.get(key)?;
        let &sorted_pos = self.base_to_sorted_pos.get(&node.index)?;
        self.sorted_positions[..sorted_pos].iter().rev().find_map(|&wp| {
            self.inner.get_by_index(wp).filter(|n| n.is_focusable()).map(|n| &n.key)
        })
    }

    fn keys(&self) -> impl Iterator<Item = &Key> {
        self.nodes().map(|n| &n.key)
    }

    fn nodes(&self) -> impl Iterator<Item = &Node<T>> {
        self.sorted_positions.iter().filter_map(|&wp| self.inner.get_by_index(wp))
    }

    fn children_of(&self, parent_key: &Key) -> impl Iterator<Item = &Node<T>> {
        self.inner.children_of(parent_key)
    }
}
````

### 7.3 Locale-Aware Collation

Sort comparators for string data should use `ars_i18n::StringCollator` rather than Rust's default byte-order `Ord` on `str`. `StringCollator` uses ICU4X under the hood and respects the user's locale, producing correct ordering for accented characters, ligatures, Han ideographs (pinyin vs. stroke vs. radical), and scripts with locale-specific rules.

#### 7.3.1 StringCollator Integration with Collection Trait

The `CollationSupport` trait adds locale-aware sorting directly to collection types. It wraps the collection in a `SortedCollection` using the provided `StringCollator`.

```rust
/// Locale-aware sorting support for collection types.
/// Requires `i18n` feature flag (depends on `ars-i18n` for `StringCollator`).
#[cfg(feature = "i18n")]
pub trait CollationSupport: Sized + CollationTarget {
    /// The output type after applying collation (typically a SortedCollection wrapper).
    type Output;

    /// Apply locale-aware sorting using the given collator and text extraction function.
    /// `text_fn` extracts the sortable text from each item.
    fn with_collation<F>(self, collator: StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&<Self as CollationTarget>::Item) -> &str + 'static;
}

/// Helper trait to associate the item type for CollationSupport.
#[cfg(feature = "i18n")]
pub trait CollationTarget {
    type Item;
}

/// Blanket impl so `&StaticCollection<T>` etc. satisfy the `CollationTarget`
/// supertrait required by `CollationSupport` without duplicating impls.
#[cfg(feature = "i18n")]
impl<T: CollationTarget> CollationTarget for &T {
    type Item = T::Item;
}

#[cfg(feature = "i18n")]
impl<T: CollectionItem + Clone> CollationTarget for StaticCollection<T> {
    type Item = T;
}

#[cfg(feature = "i18n")]
impl<T: CollectionItem + Clone> CollationTarget for TreeCollection<T> {
    type Item = T;
}

#[cfg(feature = "i18n")]
impl<'a, T: CollectionItem + Clone, C: Collection<T>> CollationTarget
    for FilteredCollection<'a, T, C>
{
    type Item = T;
}

#[cfg(feature = "i18n")]
impl<'a, T: CollectionItem + Clone> CollationSupport for &'a StaticCollection<T> {
    type Output = SortedCollection<'a, T, StaticCollection<T>>;

    fn with_collation<F>(self, collator: StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str + 'static,
    {
        // SortedCollection::new comparator receives &Node<T>; extract &T via value.
        SortedCollection::new(self, move |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}

#[cfg(feature = "i18n")]
impl<'a, T: CollectionItem + Clone> CollationSupport for &'a TreeCollection<T> {
    type Output = SortedCollection<'a, T, TreeCollection<T>>;

    /// Sorts the flattened iteration order. For per-level sibling sorting,
    /// use SortedCollection with a depth-aware comparator instead.
    fn with_collation<F>(self, collator: StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str + 'static,
    {
        SortedCollection::new(self, move |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}

#[cfg(feature = "i18n")]
impl<'a, T: CollectionItem + Clone, C: Collection<T>> CollationSupport
    for &'a FilteredCollection<'a, T, C>
{
    type Output = SortedCollection<'a, T, FilteredCollection<'a, T, C>>;

    fn with_collation<F>(self, collator: StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str + 'static,
    {
        SortedCollection::new(self, move |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}
```

**Collator caching.** Creating a `StringCollator` involves loading ICU4X locale data. Components that re-sort frequently (e.g., Table on column header click) should cache collator instances per `(locale, strength)` pair:

```rust
/// Cache for StringCollator instances, keyed by (locale, strength).
/// Uses `BTreeMap` (requires `Locale: Ord` and `CollationStrength: Ord`)
/// for deterministic iteration order and to avoid `HashMap`'s `Hash` bound.
#[cfg(feature = "i18n")]
pub struct CollatorCache {
    entries: BTreeMap<(Locale, CollationStrength), StringCollator>,
}

#[cfg(feature = "i18n")]
impl CollatorCache {
    pub const fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn get_or_create(
        &mut self,
        locale: &Locale,
        strength: CollationStrength,
    ) -> &StringCollator {
        self.entries.entry((locale.clone(), strength)).or_insert_with(|| {
            let options = CollationOptions { strength, ..CollationOptions::default() };
            StringCollator::new(locale, options)
        })
    }
}

#[cfg(feature = "i18n")]
impl Default for CollatorCache {
    fn default() -> Self { Self::new() }
}

#[cfg(feature = "i18n")]
impl Debug for CollatorCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollatorCache")
            .field("entries", &self.entries.len())
            .finish()
    }
}
```

#### 7.3.2 Collation Levels

ICU4X collation supports multiple comparison levels that control sensitivity:

> `CollationStrength` — defined in `04-internationalization.md` (cfg-gated: re-exports `ars_i18n::CollationStrength` when `i18n` feature is enabled, provides local enum otherwise)

Components that perform string sorting (Table, Combobox with autocomplete, Listbox) accept a `collation_strength: CollationStrength` prop to control comparison sensitivity:

| Component | Default Level | Rationale                                           |
| --------- | ------------- | --------------------------------------------------- |
| Table     | `Secondary`   | Column sort should group accented variants together |
| Combobox  | `Primary`     | Autocomplete matching should be accent-insensitive  |
| Listbox   | `Secondary`   | List sort groups accented variants                  |

```rust
// Example: sorting a Listbox by text_value, locale-aware

use ars_i18n::{StringCollator, CollationOptions, Locale};
use ars_collections::SortedCollection;

fn sorted_listbox<T: Clone, C: Collection<T>>(
    base: &C,
    locale: &Locale,
    direction: SortDirection,
) -> SortedCollection<'_, T, C> {
    let mut options = CollationOptions::default();
    options.strength = CollationStrength::Secondary;
    let collator = StringCollator::new(locale, options);
    SortedCollection::new(base, move |a, b| {
        let ord = collator.compare(&a.text_value, &b.text_value);
        if direction == SortDirection::Descending { ord.reverse() } else { ord }
    })
}
```

#### 7.3.3 Collation Configuration

String sorting in collections uses locale-aware collation via ICU4X `StringCollator`. Configuration: (1) Collation locale is derived from the nearest `ArsProvider` context. (2) Collation strength: Primary (base letters), Secondary (+accents), Tertiary (+case) — configurable per column via `collation_strength` prop. (3) `StringCollator` instances are cached per `(locale, strength)` pair and shared across columns. (4) Non-string types use their `Ord` implementation; mixed-type columns define a custom comparator.

### 7.4 Selection Stability Through Filter and Sort Changes

When the user applies a filter or sort, the `selection::State::selected_keys` `BTreeSet<Key>` is unchanged. Keys are stable regardless of display order or visibility. Components should:

- Render selected items with `data-ars-selected` even if they are currently hidden by a filter.
- Preserve `anchor_key` across filter changes so Shift+Click range operations remain coherent.
- When the `All` selection variant is active and a filter is applied, the semantics are "all currently visible items are selected". The component is responsible for computing the effective selected set as the intersection of `All` with the current filter's visible item keys when communicating the selection outward (e.g., in `on_selection_change` callbacks).

---

## 8. Crate Layout

```file-tree
crates/ars-collections/
  Cargo.toml
  src/
    lib.rs                  # Re-exports: Collection, Key, Node, NodeType, ...
    key.rs                  # Key enum, From impls
    node.rs                 # Node<T>, NodeType
    collection.rs           # Collection<T> trait
    builder.rs              # CollectionBuilder<T>
    static_collection.rs    # StaticCollection<T>
    tree_collection.rs      # TreeCollection<T>, TreeItemConfig<T>
    selection.rs            # selection::Mode, selection::Behavior, selection::Set,
                            #   selection::State
    typeahead.rs            # typeahead::State, TYPEAHEAD_TIMEOUT_MS
    navigation.rs           # next_enabled_key, prev_enabled_key, first_enabled_key, last_enabled_key
    async_collection.rs     # AsyncLoadingState, AsyncCollection<T>
    virtualization.rs       # Virtualizer, LayoutStrategy, ScrollAlign
    filtered_collection.rs  # FilteredCollection<T>
    sorted_collection.rs    # SortedCollection<T>, SortDirection
```

```toml
# crates/ars-collections/Cargo.toml
[package]
name    = "ars-collections"
version = "0.1.0"
edition = "2021"

[dependencies]
ars-core  = { path = "../ars-core" }
ars-a11y  = { path = "../ars-a11y" }
ars-i18n  = { path = "../ars-i18n", optional = true }
# indexmap v2 provides no_std + alloc support by default when "std" is disabled;
# there is no separate "alloc" feature.
indexmap  = { version = "2", default-features = false }
uuid     = { version = "1", default-features = false, optional = true }

[features]
default  = ["std"]
std      = ["indexmap/std"]
i18n     = ["dep:ars-i18n"]          # Enables locale-aware collation helpers
uuid     = ["dep:uuid"]              # Enables Key::Uuid variant (zero-allocation UUID keys)
serde    = ["dep:serde", "indexmap/serde"] # Serializable selection::Set, Key
```

---

## 9. Usage Examples

### 9.1 Static Listbox Collection

```rust
use ars_collections::{CollectionBuilder, Key, selection};

let collection = CollectionBuilder::new()
    .item(Key::str("apple"),  "Apple",  FruitData { color: "red" })
    .item(Key::str("banana"), "Banana", FruitData { color: "yellow" })
    .separator()
    .section(Key::str("berries"), "Berries")
        .item(Key::str("strawberry"), "Strawberry", FruitData { color: "red" })
        .item(Key::str("blueberry"),  "Blueberry",  FruitData { color: "blue" })
    .end_section()
    .build();

let mut selection = selection::State::new(selection::Mode::Single, selection::Behavior::Replace);
selection = selection.select(Key::str("apple"));
assert!(selection.is_selected(&Key::str("apple")));
assert!(!selection.is_selected(&Key::str("banana")));
```

### 9.2 Async Paginated Collection

```rust
use ars_collections::{AsyncCollection, Key};

// Initial state — no items yet.
let mut col: AsyncCollection<UserRecord> = AsyncCollection::new();

// Before fetch:
col = col.begin_load();
assert!(col.loading_state.is_loading());

// After first page arrives:
let page_one = vec![
    (Key::int(1), "Alice".into(), UserRecord { id: 1, name: "Alice" }),
    (Key::int(2), "Bob".into(),   UserRecord { id: 2, name: "Bob" }),
];
col = col.append_page(page_one, Some("cursor_abc".into()));
assert_eq!(col.size(), 2);
assert!(col.has_more);

// After second (final) page:
let page_two = vec![
    (Key::int(3), "Carol".into(), UserRecord { id: 3, name: "Carol" }),
];
col = col.append_page(page_two, None);
assert_eq!(col.size(), 3);
assert!(!col.has_more);
```

### 9.3 Tree with Expand/Collapse

```rust
use ars_collections::{TreeCollection, TreeItemConfig, Key};

let tree = TreeCollection::new(vec![
    TreeItemConfig {
        key: Key::str("fruits"),
        text_value: "Fruits".into(),
        value: CategoryData { label: "Fruits" },
        default_expanded: true,
        children: vec![
            TreeItemConfig {
                key: Key::str("apple"),
                text_value: "Apple".into(),
                value: CategoryData { label: "Apple" },
                default_expanded: false,
                children: vec![],
            },
        ],
    },
]);

// "apple" is visible because "fruits" is expanded.
assert!(tree.get(&Key::str("apple")).is_some());

// Collapse "fruits":
let collapsed = tree.set_expanded(&Key::str("fruits"), false);
// "apple" is now hidden from iteration but still in the full node set.
assert!(collapsed.get(&Key::str("apple")).is_some()); // key lookup still works
assert!(collapsed.nodes().all(|n| n.key != Key::str("apple"))); // not in visible iteration
```

### 9.4 Type-Ahead Navigation

```rust
use ars_collections::{typeahead, StaticCollection, Key};

let collection = StaticCollection::new([
    (Key::int(0), "Apple".into(),      ()),
    (Key::int(1), "Avocado".into(),    ()),
    (Key::int(2), "Banana".into(),     ()),
    (Key::int(3), "Blueberry".into(),  ()),
    (Key::int(4), "Cherry".into(),     ()),
]);

let state = typeahead::State::default();
let focus = Some(Key::int(0));

// Type "b" — should jump to Banana (the first item starting with "b" after Apple).
let locale = Locale::parse("en-US").expect("valid locale");
let (state, found) = state.process_char('b', 1000, focus.as_ref(), &collection, &locale);
assert_eq!(found, Some(Key::int(2))); // Banana

// Type "l" quickly (within 500 ms) — accumulated query is "bl" → Blueberry.
let (_, found) = state.process_char('l', 1200, Some(Key::int(2)).as_ref(), &collection, &locale);
assert_eq!(found, Some(Key::int(3))); // Blueberry
```

### 9.5 Virtualizer Scroll Math

```rust
use ars_collections::{Virtualizer, LayoutStrategy, ScrollAlign};

let mut virt = Virtualizer::new(10_000, LayoutStrategy::FixedHeight { item_height: 48.0 });
virt.set_scroll_state_mut(0.0, 0.0, 600.0, 800.0); // scrolled to top-left, 600px tall, 800px wide viewport

// Should render items 0..18: 0 overscan before (clamped), 13 visible (ceil(600/48)), 5 overscan after.
let range = virt.visible_range();
assert_eq!(range.start, 0);
assert_eq!(range.end, 13 + 5); // 18 — ceil(600/48)=13 visible + 5 overscan

// Scroll to item 500.
let scroll_to = virt.scroll_top_for_index(500, ScrollAlign::Auto);
assert_eq!(scroll_to, 500.0 * 48.0); // 24000.0

// Total scroll height.
assert_eq!(virt.total_height_px(), 10_000.0 * 48.0); // 480000.0
```

---

## 10. Drag-and-Drop Collection Integration

> Cross-references: Equivalent to React Aria `useDraggableCollection`, `useDroppableCollection`.

### 10.1 Purpose

Collection-level drag-and-drop integration enables automatic DnD behavior for collection components (Listbox, GridList, Table, TreeView) without requiring manual wiring per item. These traits extend the DnD primitives from `05-interactions.md` §7 with collection-aware semantics:

- Automatically make collection items draggable
- Compute drop indicators between items
- Fire collection-level reorder events
- Integrate with the `selection::State` (drag all selected items)
- Provide consistent drop position computation (before, after, on)

### 10.2 Drop Position

```rust
// ars-collections/src/dnd.rs

use crate::key::Key;

/// Where an item is being dropped relative to a target item in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPosition {
    /// Before the target item in reading order.
    /// - Vertical list: above the target.
    /// - Horizontal list (LTR): left of the target.
    /// - Horizontal list (RTL): right of the target (inline-start side).
    Before,

    /// After the target item in reading order.
    /// - Vertical list: below the target.
    /// - Horizontal list (LTR): right of the target.
    /// - Horizontal list (RTL): left of the target (inline-end side).
    After,

    /// On top of the target item (e.g., dropping into a folder in a tree).
    On,
}

impl core::fmt::Display for DropPosition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Before => write!(f, "before"),
            Self::After => write!(f, "after"),
            Self::On => write!(f, "on"),
        }
    }
}

/// A resolved drop target within a collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionDropTarget {
    /// The key of the item nearest to the drop point.
    pub key: Key,

    /// Where relative to that item the drop will occur.
    pub position: DropPosition,
}
```

### 10.3 Collection DnD Events

```rust
// ars-collections/src/dnd.rs

use alloc::vec::Vec;
use crate::key::Key;

/// Events fired by collection-level drag-and-drop operations.
#[derive(Clone, Debug, PartialEq)]
pub enum CollectionDndEvent {
    /// Items within the same collection are being reordered.
    ///
    /// `keys`: The keys of the items being moved (may be multiple if selection is dragged).
    /// `target`: The drop target key.
    /// `position`: Where relative to the target the items should be placed.
    Reorder {
        keys: Vec<Key>,
        target: Key,
        position: DropPosition,
    },

    /// Items are being moved from one collection to another.
    ///
    /// `keys`: The keys in the source collection.
    /// `target`: The drop target key in this (destination) collection.
    /// `position`: Where relative to the target.
    Move {
        keys: Vec<Key>,
        target: Key,
        position: DropPosition,
    },

    /// External items (from outside any collection, or from a different data source)
    /// are being inserted into this collection.
    ///
    /// `items`: Serialized drag data for each dragged item.
    /// `target`: The drop target key.
    /// `position`: Where relative to the target.
    Insert {
        items: Vec<DragItem>,
        target: Key,
        position: DropPosition,
    },

    /// A drag operation has started. Fired on the source collection.
    DragStart {
        keys: Vec<Key>,
    },

    /// A drag operation has ended (completed or cancelled).
    DragEnd {
        keys: Vec<Key>,
        /// Whether the drag completed successfully (dropped on valid target).
        success: bool,
    },
}

// Note: `CollectionDndEvent` is a callback event type, not a Machine::Event.
// It is passed to user-supplied handler callbacks (e.g., `on_reorder`, `on_insert`)
// rather than being dispatched through the Machine's `transition()` function.
// The `Clone + Debug + PartialEq` derives are for testing and logging convenience.

// `DragItem` — defined in `05-interactions.md`
```

### 10.4 DraggableCollection Trait

```rust
// ars-collections/src/dnd.rs

use alloc::collections::BTreeMap;
use crate::{Collection, key::Key, selection};

/// Extends a collection with drag-source behavior.
///
/// When applied to a collection component, this trait:
/// 1. Makes all non-disabled items draggable (or a configured subset)
/// 2. When a selected item is dragged, includes all selected items in the drag
/// 3. Provides drag preview data derived from item text values
/// 4. Fires `DragStart` / `DragEnd` events
pub trait DraggableCollection<T>: Collection<T> {
    /// Returns whether the item with the given key can be dragged.
    /// Default: all focusable items are draggable.
    fn is_draggable(&self, key: &Key) -> bool {
        self.get(key).map_or(false, |n| n.is_focusable())
    }

    /// Returns the drag data for the given keys.
    /// Default: uses text_value as "text/plain" data.
    fn drag_data(&self, keys: &[Key]) -> Vec<DragItem> {
        keys.iter()
            .filter_map(|k| {
                self.text_value_of(k).map(|text| {
                    let mut data = BTreeMap::new();
                    data.insert("text/plain".into(), text.to_owned());
                    DragItem { data }
                })
            })
            .collect()
    }

    /// Returns the current selection state.
    fn selection(&self) -> &selection::State;

    /// Returns the set of keys to include in a drag operation when `key`
    /// is the drag handle. If `key` is part of the current selection,
    /// returns all selected keys; otherwise returns just `key`.
    /// When selection is `Set::All`, iterates the collection's item keys.
    fn drag_keys(&self) -> Vec<Key> {
        match &self.selection().selected_keys {
            selection::Set::All => self.item_keys().cloned().collect(),
            other => other.keys().cloned().collect(),
        }
    }
}
```

### 10.5 DroppableCollection Trait

```rust
// ars-collections/src/dnd.rs

use crate::{Collection, key::Key};

/// Extends a collection with drop-target behavior.
///
/// When applied to a collection component, this trait:
/// 1. Computes the nearest drop target based on pointer position
/// 2. Renders drop indicators between items
/// 3. Validates whether a drop is accepted
/// 4. Fires `Reorder`, `Move`, or `Insert` events on drop
pub trait DroppableCollection<T>: Collection<T> {
    /// The set of MIME types this collection accepts for external drops.
    /// Empty means only internal reorder is supported.
    fn accepted_types(&self) -> &[&str] {
        &[]
    }

    /// Returns whether an item at `key` can receive a drop "on" it
    /// (e.g., dropping into a folder). Default: false (only between-item drops).
    fn allows_drop_on(&self, _key: &Key) -> bool {
        false
    }

    /// Compute the drop target for a given pointer position.
    /// The adapter translates screen coordinates into a `CollectionDropTarget`
    /// based on item bounding boxes and the current writing direction.
    ///
    /// - `pointer_x` and `pointer_y` are viewport-relative coordinates.
    /// - `direction` is needed for horizontal collections to correctly resolve
    ///   `Before`/`After` in RTL layouts (where the inline-start side is on the right).
    ///
    /// This method is implemented by the adapter layer, not in pure Rust,
    /// because it requires DOM measurements.
    fn compute_drop_target(
        &self,
        _pointer_x: f64,
        _pointer_y: f64,
        _direction: Direction,
    ) -> Option<CollectionDropTarget> {
        None // Adapter override required
    }

    /// Validate whether a proposed drop should be accepted.
    /// Return `false` to show a "not allowed" indicator and reject the drop.
    ///
    /// Default: accept all drops.
    fn is_drop_valid(
        &self,
        _target: &CollectionDropTarget,
        _items: &[DragItem],
    ) -> bool {
        true
    }
}
```

### 10.6 Draggable Item Accessibility

When `dnd_enabled == true`, draggable items receive these ARIA attributes:

| Attribute              | Value                | Notes                                                                                                                                                                                       |
| ---------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aria-roledescription` | `messages.draggable` | Locale-aware string (default: `"draggable"`). Screen readers announce the affordance.                                                                                                       |
| `aria-grabbed`         | `"false"` / `"true"` | `"true"` when item is actively being dragged during keyboard DnD. Note: deprecated in ARIA 1.2 but required for older AT compatibility (per `03-accessibility.md` §AriaAttribute::Grabbed). |

The optional `DragHandle` button receives:

| Attribute    | Value                              | Notes                                        |
| ------------ | ---------------------------------- | -------------------------------------------- |
| `role`       | `"button"`                         | Drag handle is an interactive button         |
| `aria-label` | `messages.drag_handle(text_value)` | Locale-aware label, e.g., "Drag {item name}" |
| `tabindex`   | `"0"`                              | Focusable for keyboard DnD initiation        |

Pointer-based DnD completion announcements: When a `CollectionDndEvent::Reorder` fires from pointer interaction, the adapter posts a completion announcement to `LiveAnnouncer` with `AriaLive::Polite`: `messages.reorder_complete(item_name, position, total)`.

### 10.7 Drop Indicator Anatomy

Collection components that support DnD rendering must include the following anatomy part:

| Part              | HTML Element | Description                                                                                                             |
| ----------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------- |
| **DropIndicator** | `<div>`      | Visual indicator showing where a dragged item will be placed. Positioned between items based on `CollectionDropTarget`. |

The adapter renders the `DropIndicator` at the computed position during a drag-over:

- `data-drop-position="before"` or `"after"` or `"on"`
- `data-drop-target="{key}"` identifies the target item
- `aria-hidden="true"` (visual-only; screen readers receive live region announcements)

### 10.8 Integration with Existing Components

After defining these traits, the following components gain optional DnD support:

| Component    | DnD Anatomy Parts                                | Events                                                                  |
| ------------ | ------------------------------------------------ | ----------------------------------------------------------------------- |
| **Listbox**  | `DragHandle` (optional), `DropIndicator`         | `CollectionDndEvent::Reorder`                                           |
| **GridList** | `DragHandle` (optional), `DropIndicator`         | `CollectionDndEvent::Reorder`                                           |
| **Table**    | `DragHandle` (optional per-row), `DropIndicator` | `CollectionDndEvent::Reorder`                                           |
| **TreeView** | `DragHandle` (optional), `DropIndicator`         | `CollectionDndEvent::Reorder`, `CollectionDndEvent::Move` (reparenting) |

Each component's state machine gains an optional `dnd_enabled: bool` config field. When enabled, the adapter wires up the `DraggableCollection` and `DroppableCollection` trait implementations.

### 10.9 I18n — CollectionDndMessages

```rust
/// Localizable messages for collection-level drag-and-drop operations.
/// Follows the ComponentMessages pattern from `04-internationalization.md` §7.1.
/// `CollectionDndMessages` uses `MessageFn` (cfg-gated: `Rc` on WASM, `Arc` on native)
/// for closure fields, consistent with the Messages convention in `04-internationalization.md` §7.1.
///
/// `MessageFn::new(closure)` accepts any closure matching the field's function signature.
/// The `MessageFn<dyn Fn(A, B, ...) -> String + Send + Sync>` wrapper provides a
/// type-erased, cfg-gated (Rc on WASM / Arc on native) container. Each closure signature
/// requires a corresponding `From` impl in the `MessageFn` infrastructure (see
/// `04-internationalization.md` §7.1).
// Manual Debug impl prints closure fields as "<closure>".
// PartialEq is not derived (closures are not comparable).
#[derive(Clone)]
pub struct CollectionDndMessages {
    /// Announced when a single item drag starts.
    /// Example: "{item_label}. Press Tab to move to a drop target, Escape to cancel."
    pub drag_start: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,

    /// Announced when a multi-item drag starts (all selected items dragged).
    /// Must use plural rules via `format_plural`.
    /// Example: "Dragging {count} items."
    pub drag_start_multi: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,

    /// Announced when hovering over a drop target.
    /// Receives target item label and drop position.
    /// Example: "Drop available: before {target_label}"
    pub drop_target_enter: MessageFn<dyn Fn(&str, DropPosition, &Locale) -> String + Send + Sync>,

    /// Announced on successful drop completion.
    /// Example: "Dropped {item_label} before {target_label}."
    pub drop_complete: MessageFn<dyn Fn(&str, &str, DropPosition, &Locale) -> String + Send + Sync>,

    /// Announced on pointer-based reorder completion.
    /// Example: "Reordered: {item_label} moved to position {pos} of {total}."
    pub reorder_complete: MessageFn<dyn Fn(&str, usize, usize, &Locale) -> String + Send + Sync>,

    /// Announced when drag is cancelled.
    /// Example: "Drop cancelled."
    pub drop_cancelled: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// The role description for draggable items.
    /// Default: "draggable"
    pub draggable: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Template for drag handle aria-label.
    /// Example: "Drag {item_name}"
    pub drag_handle: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for CollectionDndMessages {
    fn default() -> Self {
        Self {
            drag_start: MessageFn::new(|label: &str, _locale: &Locale| {
                format!("{label}. Press Tab to move to a drop target, Escape to cancel.")
            }),
            drag_start_multi: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("Dragging {count} items.")
            }),
            drop_target_enter: MessageFn::new(|target: &str, pos: DropPosition, _locale: &Locale| {
                format!("Drop available: {pos} {target}")
            }),
            drop_complete: MessageFn::new(|item: &str, target: &str, pos: DropPosition, _locale: &Locale| {
                format!("Dropped {item} {pos} {target}.")
            }),
            reorder_complete: MessageFn::new(|item: &str, pos: usize, total: usize, _locale: &Locale| {
                format!("Reordered: {item} moved to position {pos} of {total}.")
            }),
            drop_cancelled: MessageFn::new(|_locale: &Locale| "Drop cancelled.".to_string()),
            draggable: MessageFn::new(|_locale: &Locale| "draggable".to_string()),
            drag_handle: MessageFn::new(|item: &str, _locale: &Locale| {
                format!("Drag {item}")
            }),
        }
    }
}

/// Accessible announcement templates for DnD operations, posted to
/// `LiveAnnouncer` at each stage of the drag lifecycle.
pub struct DndAnnouncements {
    /// Announced when drag starts. Template: "Started dragging {item}. {position_hint}"
    pub drag_start: MessageFn<dyn Fn(DndAnnouncementData, &Locale) -> String + Send + Sync>,
    /// Announced when dragged over a drop target. Template: "Over {target}. Drop to {action}"
    pub drag_over: MessageFn<dyn Fn(DndAnnouncementData, &Locale) -> String + Send + Sync>,
    /// Announced when dropped. Template: "Dropped {item} {result}"
    pub drop: MessageFn<dyn Fn(DndAnnouncementData, &Locale) -> String + Send + Sync>,
    /// Announced when drag is cancelled. Template: "Cancelled dragging {item}"
    pub drag_cancel: MessageFn<dyn Fn(DndAnnouncementData, &Locale) -> String + Send + Sync>,
}

/// Data passed to DnD announcement templates.
pub struct DndAnnouncementData {
    /// Label of the item being dragged.
    pub item_label: String,
    /// Label of the drop target (if applicable).
    pub target_label: Option<String>,
    /// Position hint (e.g., "position 3 of 10").
    pub position_hint: Option<String>,
    /// Action description (e.g., "reorder", "move into folder").
    pub action: Option<String>,
    /// Result description (e.g., "at position 3").
    pub result: Option<String>,
}
```
