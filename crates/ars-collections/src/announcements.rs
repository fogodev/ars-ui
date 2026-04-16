//! Localizable announcement message helpers for collection mutations.

use alloc::{format, string::String, sync::Arc};
use core::fmt::{self, Debug};

use ars_core::{Locale, MessageFn};

use crate::SortDirection;

type CountLocaleMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;
type SortedLocaleMessage = dyn Fn(&str, SortDirection, &Locale) -> String + Send + Sync;
type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

/// Describes a change to a collection for screen reader announcement.
#[derive(Clone, Debug, PartialEq)]
pub enum CollectionChangeAnnouncement {
    /// Items were added. Message via [`CollectionMessages::items_added`].
    ItemsAdded {
        /// The number of items added to the collection.
        count: usize,
    },

    /// Items were removed. Message via [`CollectionMessages::items_removed`].
    ItemsRemoved {
        /// The number of items removed from the collection.
        count: usize,
    },

    /// Collection was filtered. Message via [`CollectionMessages::filtered`].
    Filtered {
        /// The number of items matching the active filter.
        matching_count: usize,
    },

    /// Collection was sorted. Message via [`CollectionMessages::sorted`].
    Sorted {
        /// The column used for sorting.
        column: String,

        /// The sort direction applied to `column`.
        direction: SortDirection,
    },

    /// Collection is empty after the operation. Message via [`CollectionMessages::empty`].
    Empty,

    /// Async load completed. Message via [`CollectionMessages::loaded`].
    Loaded {
        /// The number of items loaded by the async operation.
        count: usize,
    },
}

/// Localizable message functions for collection change announcements.
pub struct CollectionMessages {
    /// Message for items added. Receives `(count, locale)`.
    pub items_added: MessageFn<CountLocaleMessage>,

    /// Message for items removed. Receives `(count, locale)`.
    pub items_removed: MessageFn<CountLocaleMessage>,

    /// Message for filtered results. Receives `(count, locale)`.
    pub filtered: MessageFn<CountLocaleMessage>,

    /// Message for sorted collection. Receives `(column, direction, locale)`.
    pub sorted: MessageFn<SortedLocaleMessage>,

    /// Message for an empty collection. Receives `locale`.
    pub empty: MessageFn<LocaleMessage>,

    /// Message for async load completion. Receives `(count, locale)`.
    pub loaded: MessageFn<CountLocaleMessage>,
}

impl Default for CollectionMessages {
    fn default() -> Self {
        Self {
            items_added: MessageFn::new(Arc::new(|count: usize, _locale: &Locale| {
                if count == 1 {
                    String::from("1 item added")
                } else {
                    format!("{count} items added")
                }
            }) as Arc<CountLocaleMessage>),

            items_removed: MessageFn::new(Arc::new(|count: usize, _locale: &Locale| {
                if count == 1 {
                    String::from("1 item removed")
                } else {
                    format!("{count} items removed")
                }
            }) as Arc<CountLocaleMessage>),

            filtered: MessageFn::new(Arc::new(|count: usize, _locale: &Locale| match count {
                0 => String::from("No results found"),
                1 => String::from("1 result available"),
                n => format!("{n} results available"),
            }) as Arc<CountLocaleMessage>),

            sorted: MessageFn::new(Arc::new(
                |column: &str, direction: SortDirection, _locale: &Locale| {
                    format!("Sorted by {column}, {direction}")
                },
            ) as Arc<SortedLocaleMessage>),

            empty: MessageFn::new(
                Arc::new(|_locale: &Locale| String::from("No items")) as Arc<LocaleMessage>
            ),

            loaded: MessageFn::new(Arc::new(|count: usize, _locale: &Locale| {
                if count == 1 {
                    String::from("1 item loaded")
                } else {
                    format!("{count} items loaded")
                }
            }) as Arc<CountLocaleMessage>),
        }
    }
}

impl Debug for CollectionMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CollectionMessages { .. }")
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use ars_core::Locale;

    use super::*;

    fn locale() -> Locale {
        Locale::parse("en-US").expect("en-US must parse")
    }

    #[test]
    fn collection_change_announcement_variants_construct() {
        let items_added = CollectionChangeAnnouncement::ItemsAdded { count: 1 };

        let items_removed = CollectionChangeAnnouncement::ItemsRemoved { count: 2 };

        let filtered = CollectionChangeAnnouncement::Filtered { matching_count: 3 };

        let sorted = CollectionChangeAnnouncement::Sorted {
            column: String::from("Name"),
            direction: SortDirection::Ascending,
        };

        let empty = CollectionChangeAnnouncement::Empty;

        let loaded = CollectionChangeAnnouncement::Loaded { count: 4 };

        assert_eq!(
            items_added,
            CollectionChangeAnnouncement::ItemsAdded { count: 1 }
        );
        assert_eq!(
            items_removed,
            CollectionChangeAnnouncement::ItemsRemoved { count: 2 }
        );
        assert_eq!(
            filtered,
            CollectionChangeAnnouncement::Filtered { matching_count: 3 }
        );
        assert_eq!(
            sorted,
            CollectionChangeAnnouncement::Sorted {
                column: String::from("Name"),
                direction: SortDirection::Ascending,
            }
        );
        assert_eq!(empty, CollectionChangeAnnouncement::Empty);
        assert_eq!(loaded, CollectionChangeAnnouncement::Loaded { count: 4 });
    }

    #[test]
    fn collection_change_announcement_derives_clone_debug_and_partial_eq() {
        let announcement = CollectionChangeAnnouncement::Sorted {
            column: String::from("Status"),
            direction: SortDirection::Descending,
        };

        assert_eq!(announcement.clone(), announcement);
        assert_eq!(
            format!("{announcement:?}"),
            "Sorted { column: \"Status\", direction: Descending }"
        );
    }

    #[test]
    fn default_items_added_messages_match_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!((messages.items_added)(1, &locale), "1 item added");
        assert_eq!((messages.items_added)(5, &locale), "5 items added");
    }

    #[test]
    fn default_items_removed_messages_match_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!((messages.items_removed)(1, &locale), "1 item removed");
        assert_eq!((messages.items_removed)(5, &locale), "5 items removed");
    }

    #[test]
    fn default_filtered_messages_match_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!((messages.filtered)(0, &locale), "No results found");
        assert_eq!((messages.filtered)(1, &locale), "1 result available");
        assert_eq!((messages.filtered)(5, &locale), "5 results available");
    }

    #[test]
    fn default_sorted_messages_match_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!(
            (messages.sorted)("Name", SortDirection::Ascending, &locale),
            "Sorted by Name, ascending"
        );
        assert_eq!(
            (messages.sorted)("Name", SortDirection::Descending, &locale),
            "Sorted by Name, descending"
        );
    }

    #[test]
    fn default_empty_message_matches_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!((messages.empty)(&locale), "No items");
    }

    #[test]
    fn default_loaded_messages_match_spec() {
        let locale = locale();

        let messages = CollectionMessages::default();

        assert_eq!((messages.loaded)(1, &locale), "1 item loaded");
        assert_eq!((messages.loaded)(5, &locale), "5 items loaded");
    }

    #[test]
    fn collection_messages_debug_is_redacted() {
        assert_eq!(
            format!("{:?}", CollectionMessages::default()),
            "CollectionMessages { .. }"
        );
    }
}
