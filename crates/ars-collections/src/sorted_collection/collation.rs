// ars-collections/src/sorted_collection/collation.rs

//! Locale-aware collation integration for collection types.
//!
//! This module connects [`ars_i18n::StringCollator`] to collection types via
//! [`CollationTarget`] and [`CollationSupport`] traits, plus a [`CollatorCache`]
//! for reuse across repeated sort operations.

use alloc::collections::BTreeMap;
use core::fmt::{self, Debug};

use ars_i18n::{CollationOptions, CollationStrength, Locale, StringCollator};

use super::SortedCollection;
use crate::{
    Collection, collection::CollectionItem, filtered_collection::FilteredCollection,
    static_collection::StaticCollection, tree_collection::TreeCollection,
};

// ────────────────────────────────────────────────────────────────────────── //
// CollationTarget                                                           //
// ────────────────────────────────────────────────────────────────────────── //

/// Helper trait to associate the item type for [`CollationSupport`].
///
/// Implemented for collection types that store typed items. The associated
/// `Item` type is passed through to the `text_fn` closure in
/// [`CollationSupport::with_collation`].
pub trait CollationTarget {
    /// The user data type stored in the collection's nodes.
    type Item;
}

/// Blanket impl so `&StaticCollection<T>` etc. satisfy the [`CollationTarget`]
/// supertrait required by [`CollationSupport`] without duplicating impls.
impl<T: CollationTarget> CollationTarget for &T {
    type Item = T::Item;
}

impl<T: CollectionItem + Clone> CollationTarget for StaticCollection<T> {
    type Item = T;
}

impl<T: CollectionItem + Clone> CollationTarget for TreeCollection<T> {
    type Item = T;
}

impl<'a, T: CollectionItem + Clone, C: Collection<T>> CollationTarget
    for FilteredCollection<'a, T, C>
{
    type Item = T;
}

// ────────────────────────────────────────────────────────────────────────── //
// CollationSupport                                                          //
// ────────────────────────────────────────────────────────────────────────── //

/// Locale-aware sorting support for collection types.
///
/// Requires the `i18n` feature flag (depends on `ars-i18n` for
/// [`StringCollator`]). Wraps the collection in a [`SortedCollection`]
/// using the provided collator for locale-correct string ordering.
///
/// The collator is borrowed — callers may reuse a [`CollatorCache`] across
/// repeated sort operations without reconstructing the collator each time.
pub trait CollationSupport: Sized + CollationTarget {
    /// The output type after applying collation (typically a [`SortedCollection`] wrapper).
    type Output;

    /// Apply locale-aware sorting using the given collator and text extraction function.
    ///
    /// `text_fn` extracts the sortable text from each item. The collator is
    /// only used during construction (sorting happens eagerly), so neither
    /// the collator nor `text_fn` need to outlive the returned collection.
    fn with_collation<F>(self, collator: &StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&<Self as CollationTarget>::Item) -> &str;
}

impl<'a, T: CollectionItem + Clone> CollationSupport for &'a StaticCollection<T> {
    type Output = SortedCollection<'a, T, StaticCollection<T>>;

    fn with_collation<F>(self, collator: &StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str,
    {
        // SortedCollection::new comparator receives &Node<T>; extract &T via value.
        SortedCollection::new(self, |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}

impl<'a, T: CollectionItem + Clone> CollationSupport for &'a TreeCollection<T> {
    type Output = SortedCollection<'a, T, TreeCollection<T>>;

    /// Sorts the flattened iteration order. For per-level sibling sorting,
    /// use [`SortedCollection`] with a depth-aware comparator instead.
    fn with_collation<F>(self, collator: &StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str,
    {
        SortedCollection::new(self, |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}

impl<'a, T: CollectionItem + Clone, C: Collection<T>> CollationSupport
    for &'a FilteredCollection<'a, T, C>
{
    type Output = SortedCollection<'a, T, FilteredCollection<'a, T, C>>;

    fn with_collation<F>(self, collator: &StringCollator, text_fn: F) -> Self::Output
    where
        F: Fn(&T) -> &str,
    {
        SortedCollection::new(self, |a, b| {
            let a_text = text_fn(a.value.as_ref().expect("item node"));
            let b_text = text_fn(b.value.as_ref().expect("item node"));
            collator.compare(a_text, b_text)
        })
    }
}

// ────────────────────────────────────────────────────────────────────────── //
// CollatorCache                                                             //
// ────────────────────────────────────────────────────────────────────────── //

/// Cache for [`StringCollator`] instances, keyed by `(Locale, CollationStrength)`.
///
/// Uses [`BTreeMap`] (requires `Locale: Ord` and `CollationStrength: Ord`)
/// for deterministic iteration order and to avoid `HashMap`'s `Hash` bound.
///
/// Components that re-sort frequently (e.g., Table on column header click)
/// should use a `CollatorCache` to avoid repeated ICU4X locale data loading.
pub struct CollatorCache {
    entries: BTreeMap<(Locale, CollationStrength), StringCollator>,
}

impl CollatorCache {
    /// Create an empty collator cache.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Retrieve a cached collator or create one for the given locale and strength.
    pub fn get_or_create(
        &mut self,
        locale: &Locale,
        strength: CollationStrength,
    ) -> &StringCollator {
        self.entries
            .entry((locale.clone(), strength))
            .or_insert_with(|| {
                StringCollator::new(
                    locale,
                    CollationOptions {
                        strength,
                        ..CollationOptions::default()
                    },
                )
            })
    }
}

impl Default for CollatorCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Manual `Debug` prints entry count to avoid verbose collator output.
impl Debug for CollatorCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CollatorCache")
            .field("entries", &self.entries.len())
            .finish()
    }
}

// ────────────────────────────────────────────────────────────────────────── //
// Tests                                                                     //
// ────────────────────────────────────────────────────────────────────────── //

#[cfg(test)]
mod tests {
    use alloc::{
        string::{String, ToString},
        vec::Vec,
    };

    use ars_i18n::{CollationStrength, locales};

    use super::*;
    use crate::{builder::CollectionBuilder, key::Key, node::Node};

    /// Minimal item type implementing [`CollectionItem`] for collation tests.
    #[derive(Clone, Debug)]
    struct TextItem {
        id: Key,
        label: String,
    }

    impl TextItem {
        fn new(id: u64, label: &str) -> Self {
            Self {
                id: Key::int(id),
                label: label.to_string(),
            }
        }
    }

    impl CollectionItem for TextItem {
        fn key(&self) -> &Key {
            &self.id
        }

        fn text_value(&self) -> &str {
            &self.label
        }
    }

    // ------------------------------------------------------------------ //
    // CollationSupport on StaticCollection                                //
    // ------------------------------------------------------------------ //

    #[test]
    fn collation_support_static_collection_sorts_by_locale() {
        let collection = CollectionBuilder::new()
            .item(Key::int(1), "Ärger", TextItem::new(1, "Ärger"))
            .item(Key::int(2), "Banana", TextItem::new(2, "Banana"))
            .item(Key::int(3), "Art", TextItem::new(3, "Art"))
            .build();

        let locale = locales::de();
        let collator = StringCollator::new(&locale, Default::default());

        let sorted = (&collection).with_collation(&collator, |item: &TextItem| &item.label);

        let texts: Vec<_> = sorted
            .nodes()
            .filter(|n: &&Node<TextItem>| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect();

        // German locale: Ä has the same primary weight as A.
        // Primary comparison: Är-g-er vs Ar-t → g < t, so Ärger < Art.
        assert_eq!(texts, vec!["Ärger", "Art", "Banana"]);
    }

    // ------------------------------------------------------------------ //
    // CollationSupport on TreeCollection                                   //
    // ------------------------------------------------------------------ //

    #[test]
    fn collation_support_tree_collection_sorts_by_locale() {
        use crate::tree_collection::TreeItemConfig;

        let tree = TreeCollection::new([
            TreeItemConfig {
                key: Key::int(1),
                text_value: "Cherry".to_string(),
                value: TextItem::new(1, "Cherry"),
                children: alloc::vec![],
                default_expanded: false,
            },
            TreeItemConfig {
                key: Key::int(2),
                text_value: "Apple".to_string(),
                value: TextItem::new(2, "Apple"),
                children: alloc::vec![],
                default_expanded: false,
            },
            TreeItemConfig {
                key: Key::int(3),
                text_value: "Banana".to_string(),
                value: TextItem::new(3, "Banana"),
                children: alloc::vec![],
                default_expanded: false,
            },
        ]);

        let locale = locales::en();
        let collator = StringCollator::new(&locale, Default::default());

        let sorted = (&tree).with_collation(&collator, |item: &TextItem| &item.label);

        let texts: Vec<_> = sorted
            .nodes()
            .filter(|n: &&Node<TextItem>| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect();

        assert_eq!(texts, vec!["Apple", "Banana", "Cherry"]);
    }

    // ------------------------------------------------------------------ //
    // CollationSupport on FilteredCollection                              //
    // ------------------------------------------------------------------ //

    #[test]
    fn collation_support_filtered_collection_sorts_by_locale() {
        let collection = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", TextItem::new(1, "Cherry"))
            .item(Key::int(2), "Apple", TextItem::new(2, "Apple"))
            .item(Key::int(3), "Banana", TextItem::new(3, "Banana"))
            .item(Key::int(4), "Date", TextItem::new(4, "Date"))
            .build();

        // Filter out "Date".
        let filtered = FilteredCollection::new(&collection, |n| n.text_value != "Date");

        let locale = locales::en();
        let collator = StringCollator::new(&locale, Default::default());

        let sorted = (&filtered).with_collation(&collator, |item: &TextItem| &item.label);

        let texts: Vec<_> = sorted
            .nodes()
            .filter(|n: &&Node<TextItem>| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect();

        // Date is filtered out; remaining sorted: Apple, Banana, Cherry.
        assert_eq!(texts, vec!["Apple", "Banana", "Cherry"]);
    }

    // ------------------------------------------------------------------ //
    // CollatorCache                                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn collator_cache_new_is_empty() {
        let cache = CollatorCache::new();

        // Debug output should show zero entries.
        let debug = alloc::format!("{cache:?}");
        assert!(debug.contains("CollatorCache"));
        assert!(debug.contains('0'));
    }

    #[test]
    fn collator_cache_returns_same_instance() {
        let mut cache = CollatorCache::new();
        let locale = locales::de();

        let first = cache.get_or_create(&locale, CollationStrength::Secondary);
        let first_ptr = first as *const _;

        let second = cache.get_or_create(&locale, CollationStrength::Secondary);
        let second_ptr = second as *const _;

        assert_eq!(
            first_ptr, second_ptr,
            "same (locale, strength) must return cached instance"
        );
    }

    #[test]
    fn collator_cache_different_strength_creates_new() {
        let mut cache = CollatorCache::new();
        let locale = locales::en();

        let primary = cache.get_or_create(&locale, CollationStrength::Primary);
        let primary_ptr = primary as *const _;

        let tertiary = cache.get_or_create(&locale, CollationStrength::Tertiary);
        let tertiary_ptr = tertiary as *const _;

        assert_ne!(
            primary_ptr, tertiary_ptr,
            "different strengths must produce different instances"
        );
    }

    #[test]
    fn collator_cache_default_equals_new() {
        let from_new = CollatorCache::new();
        let from_default = CollatorCache::default();

        let new_debug = alloc::format!("{from_new:?}");
        let default_debug = alloc::format!("{from_default:?}");

        assert_eq!(new_debug, default_debug);
    }

    // ------------------------------------------------------------------ //
    // Edge cases                                                           //
    // ------------------------------------------------------------------ //

    #[test]
    fn collation_support_empty_collection() {
        let collection = CollectionBuilder::<TextItem>::new().build();

        let locale = locales::en();
        let collator = StringCollator::new(&locale, Default::default());

        let sorted = (&collection).with_collation(&collator, |item: &TextItem| &item.label);

        assert_eq!(sorted.size(), 0);
        assert!(sorted.first_key().is_none());
    }

    #[test]
    fn collation_support_sectioned_collection_preserves_sections() {
        let collection = CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(2), "Cherry", TextItem::new(2, "Cherry"))
            .item(Key::int(1), "Apple", TextItem::new(1, "Apple"))
            .end_section()
            .section(Key::str("vegs"), "Vegetables")
            .item(Key::int(4), "Carrot", TextItem::new(4, "Carrot"))
            .item(Key::int(3), "Artichoke", TextItem::new(3, "Artichoke"))
            .end_section()
            .build();

        let locale = locales::en();
        let collator = StringCollator::new(&locale, Default::default());

        let sorted = (&collection).with_collation(&collator, |item: &TextItem| &item.label);

        // Items within each section are sorted independently.
        let item_texts: Vec<_> = sorted
            .nodes()
            .filter(|n: &&Node<TextItem>| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect();

        assert_eq!(item_texts, vec!["Apple", "Cherry", "Artichoke", "Carrot"]);
    }

    #[test]
    fn collator_cache_composes_with_collation_support() {
        let collection = CollectionBuilder::new()
            .item(Key::int(1), "Cherry", TextItem::new(1, "Cherry"))
            .item(Key::int(2), "Apple", TextItem::new(2, "Apple"))
            .item(Key::int(3), "Banana", TextItem::new(3, "Banana"))
            .build();

        let mut cache = CollatorCache::new();
        let locale = locales::en();

        // Use cached collator with the convenience trait.
        let collator = cache.get_or_create(&locale, CollationStrength::Tertiary);
        let sorted = (&collection).with_collation(collator, |item: &TextItem| &item.label);

        let texts: Vec<_> = sorted
            .nodes()
            .filter(|n: &&Node<TextItem>| n.is_focusable())
            .map(|n| n.text_value.as_str())
            .collect();

        assert_eq!(texts, vec!["Apple", "Banana", "Cherry"]);
    }
}
