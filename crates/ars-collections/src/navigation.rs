use alloc::collections::BTreeSet;

use crate::{Collection, DisabledBehavior, key::Key};

/// Returns the next focusable key, skipping disabled items when configured.
#[must_use]
pub fn next_enabled_key<T, C: Collection<T>>(
    collection: &C,
    current: &Key,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
    wrap: bool,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        let next = if wrap {
            collection.key_after(current)
        } else {
            collection.key_after_no_wrap(current)
        };
        return next.cloned();
    }

    let mut candidate = if wrap {
        collection.key_after(current)
    } else {
        collection.key_after_no_wrap(current)
    };
    let start = candidate.cloned();

    loop {
        match candidate {
            None => return None,
            Some(key) if !disabled_keys.contains(key) => return Some(key.clone()),
            Some(key) => {
                candidate = if wrap {
                    collection.key_after(key)
                } else {
                    collection.key_after_no_wrap(key)
                };
                if candidate.cloned() == start {
                    return None;
                }
            }
        }
    }
}

/// Returns the previous focusable key, skipping disabled items when configured.
#[must_use]
pub fn prev_enabled_key<T, C: Collection<T>>(
    collection: &C,
    current: &Key,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
    wrap: bool,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        let previous = if wrap {
            collection.key_before(current)
        } else {
            collection.key_before_no_wrap(current)
        };
        return previous.cloned();
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
            Some(key) if !disabled_keys.contains(key) => return Some(key.clone()),
            Some(key) => {
                candidate = if wrap {
                    collection.key_before(key)
                } else {
                    collection.key_before_no_wrap(key)
                };
                if candidate.cloned() == start {
                    return None;
                }
            }
        }
    }
}

/// Returns the first enabled focusable key in the collection.
#[must_use]
pub fn first_enabled_key<T, C: Collection<T>>(
    collection: &C,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        return collection.first_key().cloned();
    }

    let mut candidate = collection.first_key();
    while let Some(key) = candidate {
        if !disabled_keys.contains(key) {
            return Some(key.clone());
        }
        candidate = collection.key_after_no_wrap(key);
    }

    None
}

/// Returns the last enabled focusable key in the collection.
#[must_use]
pub fn last_enabled_key<T, C: Collection<T>>(
    collection: &C,
    disabled_keys: &BTreeSet<Key>,
    disabled_behavior: DisabledBehavior,
) -> Option<Key> {
    if disabled_behavior == DisabledBehavior::FocusOnly {
        return collection.last_key().cloned();
    }

    let mut candidate = collection.last_key();
    while let Some(key) = candidate {
        if !disabled_keys.contains(key) {
            return Some(key.clone());
        }
        candidate = collection.key_before_no_wrap(key);
    }

    None
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use super::*;
    use crate::CollectionBuilder;

    fn fixture_collection() -> crate::StaticCollection<&'static str> {
        CollectionBuilder::new()
            .item(Key::int(1), "One", "one")
            .section(Key::str("group"), "Group")
            .item(Key::int(2), "Two", "two")
            .item(Key::int(3), "Three", "three")
            .separator()
            .item(Key::int(4), "Four", "four")
            .end_section()
            .item(Key::int(5), "Five", "five")
            .build()
    }

    #[test]
    fn next_enabled_key_skips_disabled_with_wrap() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3)]);

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(2),
                &disabled,
                DisabledBehavior::Skip,
                true
            ),
            Some(Key::int(4))
        );
    }

    #[test]
    fn next_enabled_key_respects_no_wrap() {
        let collection = fixture_collection();

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(5),
                &BTreeSet::new(),
                DisabledBehavior::Skip,
                false,
            ),
            None
        );
    }

    #[test]
    fn next_enabled_key_focus_only_allows_disabled_targets() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3)]);

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(2),
                &disabled,
                DisabledBehavior::FocusOnly,
                true,
            ),
            Some(Key::int(3))
        );
    }

    #[test]
    fn next_enabled_key_focus_only_respects_no_wrap() {
        let collection = fixture_collection();

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(5),
                &BTreeSet::from([Key::int(5)]),
                DisabledBehavior::FocusOnly,
                false,
            ),
            None
        );
    }

    #[test]
    fn next_enabled_key_skip_no_wrap_stops_after_disabled_tail() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3), Key::int(4), Key::int(5)]);

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(2),
                &disabled,
                DisabledBehavior::Skip,
                false,
            ),
            None
        );
    }

    #[test]
    fn prev_enabled_key_skips_disabled_with_wrap() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3)]);

        assert_eq!(
            prev_enabled_key(
                &collection,
                &Key::int(4),
                &disabled,
                DisabledBehavior::Skip,
                true
            ),
            Some(Key::int(2))
        );
    }

    #[test]
    fn prev_enabled_key_focus_only_allows_disabled_targets() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3)]);

        assert_eq!(
            prev_enabled_key(
                &collection,
                &Key::int(4),
                &disabled,
                DisabledBehavior::FocusOnly,
                true,
            ),
            Some(Key::int(3))
        );
    }

    #[test]
    fn prev_enabled_key_focus_only_respects_no_wrap() {
        let collection = fixture_collection();

        assert_eq!(
            prev_enabled_key(
                &collection,
                &Key::int(1),
                &BTreeSet::from([Key::int(1)]),
                DisabledBehavior::FocusOnly,
                false,
            ),
            None
        );
    }

    #[test]
    fn prev_enabled_key_skip_no_wrap_stops_after_disabled_head() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(1), Key::int(2), Key::int(3)]);

        assert_eq!(
            prev_enabled_key(
                &collection,
                &Key::int(4),
                &disabled,
                DisabledBehavior::Skip,
                false,
            ),
            None
        );
    }

    #[test]
    fn first_enabled_key_skips_disabled_and_structural_nodes() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(1), Key::int(2)]);

        assert_eq!(
            first_enabled_key(&collection, &disabled, DisabledBehavior::Skip),
            Some(Key::int(3))
        );
    }

    #[test]
    fn first_enabled_key_focus_only_returns_first_focusable_key() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(1), Key::int(2), Key::int(3)]);

        assert_eq!(
            first_enabled_key(&collection, &disabled, DisabledBehavior::FocusOnly),
            Some(Key::int(1))
        );
    }

    #[test]
    fn last_enabled_key_skips_disabled_and_structural_nodes() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(5), Key::int(4)]);

        assert_eq!(
            last_enabled_key(&collection, &disabled, DisabledBehavior::Skip),
            Some(Key::int(3))
        );
    }

    #[test]
    fn last_enabled_key_focus_only_returns_last_focusable_key() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([Key::int(3), Key::int(4), Key::int(5)]);

        assert_eq!(
            last_enabled_key(&collection, &disabled, DisabledBehavior::FocusOnly),
            Some(Key::int(5))
        );
    }

    #[test]
    fn all_items_disabled_returns_none_without_looping() {
        let collection = fixture_collection();
        let disabled = BTreeSet::from([
            Key::int(1),
            Key::int(2),
            Key::int(3),
            Key::int(4),
            Key::int(5),
        ]);

        assert_eq!(
            next_enabled_key(
                &collection,
                &Key::int(1),
                &disabled,
                DisabledBehavior::Skip,
                true
            ),
            None
        );
        assert_eq!(
            prev_enabled_key(
                &collection,
                &Key::int(5),
                &disabled,
                DisabledBehavior::Skip,
                true
            ),
            None
        );
        assert_eq!(
            first_enabled_key(&collection, &disabled, DisabledBehavior::Skip),
            None
        );
        assert_eq!(
            last_enabled_key(&collection, &disabled, DisabledBehavior::Skip),
            None
        );
    }
}
