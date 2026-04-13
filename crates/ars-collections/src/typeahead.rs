// ars-collections/src/typeahead.rs

//! Type-ahead / type-select state machine for keyboard search in collections.
//!
//! Type-ahead allows users to jump to items by typing text matching item labels.
//! It is consumed by any component that renders a list with keyboard navigation
//! (Listbox, Select, Menu, Combobox, TreeView).
//!
//! The [`State`](State) struct accumulates keystrokes into a search buffer and
//! finds matching items by prefix. When the user pauses typing for longer than
//! [`TYPEAHEAD_TIMEOUT`](TYPEAHEAD_TIMEOUT), the buffer resets automatically on
//! the next keystroke.

use alloc::{string::String, vec::Vec};
use core::time::Duration;

#[cfg(feature = "i18n")]
use ars_i18n::Locale;

use crate::{Collection, key::Key};

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
    /// `Clock` trait (see `01-architecture.md §1.4` on `no_std` Timer/Clock).
    pub last_key_time_ms: u64,

    /// The key that was focused when the current search started. Used as the
    /// starting point for wrap-around: if we reach the end of the list without
    /// a match, we wrap to the beginning and continue searching up to (but not
    /// including) the start key.
    pub search_start_key: Option<Key>,
}

impl State {
    /// Process a new character from a keydown event (non-i18n fallback using
    /// ASCII case folding).
    ///
    /// - If the elapsed time since the last keypress exceeds
    ///   [`TYPEAHEAD_TIMEOUT`], the search string is reset before appending the
    ///   new character.
    /// - Returns `(new_state, Some(key))` if a match was found, or
    ///   `(new_state, None)` otherwise.
    #[cfg(not(feature = "i18n"))]
    pub fn process_char<T, C: Collection<T>>(
        &self,
        ch: char,
        now_ms: u64,
        current_focus: Option<&Key>,
        collection: &C,
    ) -> (Self, Option<Key>) {
        let timed_out = Duration::from_millis(now_ms.saturating_sub(self.last_key_time_ms))
            >= TYPEAHEAD_TIMEOUT;

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

        let found = Self::find_match(&search, current_focus, collection);

        let new_state = Self {
            search,
            last_key_time_ms: now_ms,
            search_start_key: search_start,
        };

        (new_state, found)
    }

    /// Find the first item whose `text_value` starts with `search` using ASCII
    /// case folding, beginning the search from the item *after* `current_focus`
    /// and wrapping around.
    ///
    /// Single-character searches wrap; multi-character searches do not (they
    /// stay within the current alphabetical run to avoid disorienting jumps).
    #[cfg(not(feature = "i18n"))]
    fn find_match<T, C: Collection<T>>(
        search: &str,
        current_focus: Option<&Key>,
        collection: &C,
    ) -> Option<Key> {
        let query = search.to_lowercase();

        let single_char = query.chars().count() == 1;

        let all_item_keys: Vec<Key> = collection
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.key.clone())
            .collect();

        if all_item_keys.is_empty() {
            return None;
        }

        // Find the starting position.
        let start_pos = current_focus
            .and_then(|k| all_item_keys.iter().position(|ik| ik == k))
            .map_or(0, |p| (p + 1) % all_item_keys.len());

        // Scan in two passes for wrap-around (single char only).
        let scan_len = if single_char {
            all_item_keys.len()
        } else {
            all_item_keys.len().saturating_sub(start_pos)
        };

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

    /// Process a new character from a keydown event (locale-aware case folding
    /// via ICU4X `CaseMapper`).
    ///
    /// - If the elapsed time since the last keypress exceeds
    ///   [`TYPEAHEAD_TIMEOUT`], the search string is reset before appending the
    ///   new character.
    /// - Returns `(new_state, Some(key))` if a match was found, or
    ///   `(new_state, None)` otherwise.
    #[cfg(feature = "i18n")]
    pub fn process_char<T, C: Collection<T>>(
        &self,
        ch: char,
        now_ms: u64,
        current_focus: Option<&Key>,
        collection: &C,
        locale: &Locale,
    ) -> (Self, Option<Key>) {
        let timed_out = Duration::from_millis(now_ms.saturating_sub(self.last_key_time_ms))
            >= TYPEAHEAD_TIMEOUT;

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

        let found = Self::find_match(&search, current_focus, collection, locale);

        let new_state = Self {
            search,
            last_key_time_ms: now_ms,
            search_start_key: search_start,
        };

        (new_state, found)
    }

    /// Find the first item whose `text_value` starts with `search` using
    /// locale-aware case folding via ICU4X `CaseMapper`, beginning the search
    /// from the item *after* `current_focus` and wrapping around.
    ///
    /// Single-character searches wrap; multi-character searches do not (they
    /// stay within the current alphabetical run to avoid disorienting jumps).
    #[cfg(feature = "i18n")]
    fn find_match<T, C: Collection<T>>(
        search: &str,
        current_focus: Option<&Key>,
        collection: &C,
        locale: &Locale,
    ) -> Option<Key> {
        // CaseMapper::new() returns CaseMapperBorrowed<'static> which is Copy —
        // no caching needed, can be constructed freely.
        let case_mapper = icu::casemap::CaseMapper::new();

        let langid = locale.language_identifier();

        let query = case_mapper.lowercase_to_string(search, langid);

        let single_char = query.chars().count() == 1;

        let all_item_keys: Vec<Key> = collection
            .nodes()
            .filter(|n| n.is_focusable())
            .map(|n| n.key.clone())
            .collect();

        if all_item_keys.is_empty() {
            return None;
        }

        // Find the starting position.
        let start_pos = current_focus
            .and_then(|k| all_item_keys.iter().position(|ik| ik == k))
            .map_or(0, |p| (p + 1) % all_item_keys.len());

        // Scan forward only; single-char wraps, multi-char does not.
        let scan_len = if single_char {
            all_item_keys.len()
        } else {
            all_item_keys.len().saturating_sub(start_pos)
        };

        for offset in 0..scan_len {
            let idx = (start_pos + offset) % all_item_keys.len();

            let key = &all_item_keys[idx];

            if let Some(text) = collection.text_value_of(key) {
                if case_mapper
                    .lowercase_to_string(text, langid)
                    .starts_with(query.as_ref())
                {
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

#[cfg(all(test, not(feature = "i18n")))]
mod tests {
    use super::*;
    use crate::{builder::CollectionBuilder, key::Key};

    /// Build a simple fruit collection for testing.
    fn fruit_collection() -> crate::StaticCollection<&'static str> {
        CollectionBuilder::new()
            .item(Key::int(1), "Apple", "apple")
            .item(Key::int(2), "Banana", "banana")
            .item(Key::int(3), "Blueberry", "blueberry")
            .item(Key::int(4), "Cherry", "cherry")
            .item(Key::int(5), "Date", "date")
            .build()
    }

    /// Build a collection with structural nodes (section, separator) to verify
    /// they are skipped during matching.
    fn collection_with_structural() -> crate::StaticCollection<&'static str> {
        CollectionBuilder::new()
            .section(Key::str("fruits"), "Fruits")
            .item(Key::int(1), "Apple", "apple")
            .item(Key::int(2), "Banana", "banana")
            .separator()
            .item(Key::int(3), "Cherry", "cherry")
            .build()
    }

    // ------------------------------------------------------------------ //
    // Character accumulation                                              //
    // ------------------------------------------------------------------ //

    #[test]
    fn character_accumulation() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'b' at t=100
        let (state, _) = state.process_char('b', 100, None, &c);
        assert_eq!(state.search, "b");

        // Type 'a' at t=200 (within timeout)
        let (state, _) = state.process_char('a', 200, None, &c);
        assert_eq!(state.search, "ba");

        // Type 'n' at t=300 (within timeout)
        let (state, _) = state.process_char('n', 300, None, &c);
        assert_eq!(state.search, "ban");
    }

    // ------------------------------------------------------------------ //
    // Timeout resets search buffer                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn timeout_resets_search() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'a' at t=100
        let (state, _) = state.process_char('a', 100, None, &c);
        assert_eq!(state.search, "a");

        // Type 'b' at t=700 (600ms later, > 500ms timeout)
        let (state, _) = state.process_char('b', 700, None, &c);
        assert_eq!(state.search, "b");
    }

    #[test]
    fn within_timeout_preserves_buffer() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'a' at t=100
        let (state, _) = state.process_char('a', 100, None, &c);

        // Type 'p' at t=599 (499ms later, < 500ms timeout)
        let (state, _) = state.process_char('p', 599, None, &c);
        assert_eq!(state.search, "ap");
    }

    // ------------------------------------------------------------------ //
    // Prefix matching                                                     //
    // ------------------------------------------------------------------ //

    #[test]
    fn find_match_prefix() {
        let c = fruit_collection();

        let state = State::default();

        // Type "ban" — should match "Banana"
        let (state, _) = state.process_char('b', 100, None, &c);
        let (state, _) = state.process_char('a', 200, None, &c);
        let (_, found) = state.process_char('n', 300, None, &c);

        assert_eq!(found, Some(Key::int(2))); // Banana
    }

    #[test]
    fn find_match_case_insensitive() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'B' (uppercase) — should still match "Banana" or "Blueberry"
        let (_, found) = state.process_char('B', 100, None, &c);
        assert!(found.is_some());
    }

    #[test]
    fn no_match_returns_none() {
        let c = fruit_collection();

        let state = State::default();

        // Type "xyz" — no match
        let (state, _) = state.process_char('x', 100, None, &c);
        let (state, _) = state.process_char('y', 200, None, &c);
        let (_, found) = state.process_char('z', 300, None, &c);

        assert_eq!(found, None);
    }

    // ------------------------------------------------------------------ //
    // Wrap-around for single-char queries                                 //
    // ------------------------------------------------------------------ //

    #[test]
    fn single_char_wraps_around() {
        let c = fruit_collection();

        let state = State::default();

        // Focus on Cherry (key=4), type 'a' — should wrap to Apple (key=1)
        let (_, found) = state.process_char('a', 100, Some(&Key::int(4)), &c);
        assert_eq!(found, Some(Key::int(1)));
    }

    // ------------------------------------------------------------------ //
    // Multi-char queries do NOT wrap                                      //
    // ------------------------------------------------------------------ //

    #[test]
    fn multi_char_does_not_wrap() {
        let c = fruit_collection();

        let state = State::default();

        // Focus on Cherry (key=4, index=3). start_pos = 4, scan_len = 5-4 = 1.
        // Only Date is scanned. "ap" doesn't match Date, and Apple (index 0)
        // is behind the scan window — multi-char must NOT wrap back to it.
        let (state, _) = state.process_char('a', 100, Some(&Key::int(4)), &c);
        let (_, found) = state.process_char('p', 200, Some(&Key::int(4)), &c);

        assert_eq!(found, None);
    }

    // ------------------------------------------------------------------ //
    // Match cycling (repeated same character)                             //
    // ------------------------------------------------------------------ //

    #[test]
    fn match_cycling_same_char() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'b' — should match Banana (key=2, first 'b' item)
        let (state, found) = state.process_char('b', 100, None, &c);
        assert_eq!(found, Some(Key::int(2))); // Banana

        // Type 'b' again after timeout — should cycle to Blueberry (key=3)
        let (_, found) = state.process_char('b', 700, Some(&Key::int(2)), &c);
        assert_eq!(found, Some(Key::int(3))); // Blueberry
    }

    #[test]
    fn match_cycling_wraps_to_first() {
        let c = fruit_collection();

        let state = State::default();

        // Focus on Blueberry (key=3), type 'b' after timeout — should wrap
        // back to Banana (key=2)
        let (_, found) = state.process_char('b', 100, Some(&Key::int(3)), &c);
        assert_eq!(found, Some(Key::int(2)));
    }

    // ------------------------------------------------------------------ //
    // Structural nodes are skipped                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn skips_structural_nodes() {
        let c = collection_with_structural();

        let state = State::default();

        // Type 'f' — "Fruits" is a section header, should NOT match.
        // No focusable item starts with 'f', so result is None.
        let (_, found) = state.process_char('f', 100, None, &c);
        assert_eq!(found, None);
    }

    // ------------------------------------------------------------------ //
    // Empty collection                                                    //
    // ------------------------------------------------------------------ //

    #[test]
    fn empty_collection_returns_none() {
        let c: crate::StaticCollection<&str> = CollectionBuilder::new().build();

        let state = State::default();

        let (_, found) = state.process_char('a', 100, None, &c);
        assert_eq!(found, None);
    }

    // ------------------------------------------------------------------ //
    // Reset                                                               //
    // ------------------------------------------------------------------ //

    #[test]
    fn reset_clears_state() {
        let reset_state = State::reset();

        assert_eq!(reset_state, State::default());
        assert!(reset_state.search.is_empty());
        assert_eq!(reset_state.last_key_time_ms, 0);
        assert_eq!(reset_state.search_start_key, None);
    }

    // ------------------------------------------------------------------ //
    // search_start_key preserved within timeout                           //
    // ------------------------------------------------------------------ //

    #[test]
    fn search_start_key_preserved_within_timeout() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'b' at t=100 with focus on Apple (key=1)
        let (state, _) = state.process_char('b', 100, Some(&Key::int(1)), &c);
        assert_eq!(state.search_start_key, Some(Key::int(1)));

        // Type 'a' at t=200 — same search session, start key preserved
        let (state, _) = state.process_char('a', 200, Some(&Key::int(2)), &c);
        assert_eq!(state.search_start_key, Some(Key::int(1)));
    }

    #[test]
    fn search_start_key_resets_on_timeout() {
        let c = fruit_collection();

        let state = State::default();

        // Type 'b' at t=100 with focus on Apple
        let (state, _) = state.process_char('b', 100, Some(&Key::int(1)), &c);
        assert_eq!(state.search_start_key, Some(Key::int(1)));

        // Type 'c' at t=700 (after timeout) with focus on Banana
        let (state, _) = state.process_char('c', 700, Some(&Key::int(2)), &c);
        assert_eq!(state.search_start_key, Some(Key::int(2)));
    }
}

#[cfg(all(test, feature = "i18n"))]
mod tests_i18n {
    use ars_i18n::Locale;

    use super::*;
    use crate::{builder::CollectionBuilder, key::Key};

    /// Build a simple fruit collection for testing.
    fn fruit_collection() -> crate::StaticCollection<&'static str> {
        CollectionBuilder::new()
            .item(Key::int(1), "Apple", "apple")
            .item(Key::int(2), "Banana", "banana")
            .item(Key::int(3), "Blueberry", "blueberry")
            .item(Key::int(4), "Cherry", "cherry")
            .item(Key::int(5), "Date", "date")
            .build()
    }

    fn en_locale() -> Locale {
        Locale::parse("en").expect("valid locale")
    }

    fn tr_locale() -> Locale {
        Locale::parse("tr").expect("valid locale")
    }

    // ------------------------------------------------------------------ //
    // Basic i18n: accumulation and prefix matching                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn i18n_character_accumulation() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        let (state, _) = state.process_char('b', 100, None, &c, &locale);
        assert_eq!(state.search, "b");

        let (state, _) = state.process_char('a', 200, None, &c, &locale);
        assert_eq!(state.search, "ba");
    }

    #[test]
    fn i18n_timeout_resets_search() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        let (state, _) = state.process_char('a', 100, None, &c, &locale);
        assert_eq!(state.search, "a");

        // 600ms later, > 500ms timeout
        let (state, _) = state.process_char('b', 700, None, &c, &locale);
        assert_eq!(state.search, "b");
    }

    #[test]
    fn i18n_prefix_match() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        let (state, _) = state.process_char('b', 100, None, &c, &locale);
        let (state, _) = state.process_char('a', 200, None, &c, &locale);
        let (_, found) = state.process_char('n', 300, None, &c, &locale);

        assert_eq!(found, Some(Key::int(2))); // Banana
    }

    #[test]
    fn i18n_case_insensitive() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        // Uppercase 'B' should still match
        let (_, found) = state.process_char('B', 100, None, &c, &locale);
        assert!(found.is_some());
    }

    #[test]
    fn i18n_single_char_wraps() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        // Focus on Cherry (key=4), type 'a' — wraps to Apple (key=1)
        let (_, found) = state.process_char('a', 100, Some(&Key::int(4)), &c, &locale);
        assert_eq!(found, Some(Key::int(1)));
    }

    #[test]
    fn i18n_no_match_returns_none() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        // Type "xyz" — no match
        let (state, _) = state.process_char('x', 100, None, &c, &locale);
        let (state, _) = state.process_char('y', 200, None, &c, &locale);
        let (_, found) = state.process_char('z', 300, None, &c, &locale);

        assert_eq!(found, None);
    }

    #[test]
    fn i18n_empty_collection_returns_none() {
        let c: crate::StaticCollection<&str> = CollectionBuilder::new().build();

        let locale = en_locale();

        let state = State::default();

        let (_, found) = state.process_char('a', 100, None, &c, &locale);
        assert_eq!(found, None);
    }

    #[test]
    fn i18n_search_start_key_preserved_within_timeout() {
        let c = fruit_collection();

        let locale = en_locale();

        let state = State::default();

        // Type 'b' at t=100 with focus on Apple (key=1)
        let (state, _) = state.process_char('b', 100, Some(&Key::int(1)), &c, &locale);
        assert_eq!(state.search_start_key, Some(Key::int(1)));

        // Type 'a' at t=200 — same search session, start key preserved
        let (state, _) = state.process_char('a', 200, Some(&Key::int(2)), &c, &locale);
        assert_eq!(state.search_start_key, Some(Key::int(1)));
    }

    #[test]
    fn i18n_reset() {
        let reset_state = State::reset();

        assert_eq!(reset_state, State::default());
    }

    // ------------------------------------------------------------------ //
    // Locale-specific case folding                                        //
    // ------------------------------------------------------------------ //

    #[test]
    fn i18n_turkish_dotless_i() {
        // Turkish locale: lowercase 'I' → 'ı' (dotless i), not 'i'.
        // A collection item with text "ılık" should NOT match 'I' under Turkish rules
        // because Turkish lowercases 'I' to 'ı', and "ılık" does start with 'ı'.
        let c = CollectionBuilder::new()
            .item(Key::int(1), "ılık", "warm")
            .item(Key::int(2), "igloo", "igloo")
            .build();

        let locale = tr_locale();

        let state = State::default();

        // Under Turkish locale, 'I' lowercases to 'ı', matching "ılık"
        let (_, found) = state.process_char('I', 100, None, &c, &locale);
        assert_eq!(found, Some(Key::int(1))); // ılık, not igloo
    }

    #[test]
    fn i18n_turkish_dotted_i() {
        // Turkish locale: lowercase 'İ' (U+0130, capital dotted I) → 'i'.
        let c = CollectionBuilder::new()
            .item(Key::int(1), "igloo", "igloo")
            .item(Key::int(2), "ılık", "warm")
            .build();

        let locale = tr_locale();

        let state = State::default();

        // Under Turkish locale, 'İ' (dotted capital I) lowercases to 'i',
        // matching "igloo"
        let (_, found) = state.process_char('İ', 100, None, &c, &locale);
        assert_eq!(found, Some(Key::int(1))); // igloo
    }
}
