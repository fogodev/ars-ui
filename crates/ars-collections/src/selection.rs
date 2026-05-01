use alloc::{boxed::Box, collections::BTreeSet};

use ars_core::Callback;

use crate::{Collection, key::Key};

/// Whether and how many items can be selected simultaneously.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Mode {
    /// No items can be selected. Focus still moves through the list.
    #[default]
    None,

    /// Exactly one item can be selected at a time.
    Single,

    /// Any number of items may be selected independently.
    Multiple,
}

/// Controls how pointer and keyboard selection affect the current selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Behavior {
    /// Toggling an item preserves the rest of the current selection.
    #[default]
    Toggle,

    /// Selecting an item replaces the current selection.
    Replace,
}

/// The set of currently selected keys.
///
/// `All` represents "every item is selected", including items not yet loaded
/// in async or paginated collections.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Set {
    /// No items are selected.
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
    /// Returns `true` when `key` is part of this selection.
    #[must_use]
    pub fn contains(&self, key: &Key) -> bool {
        match self {
            Self::Empty => false,
            Self::Single(selected) => selected == key,
            Self::Multiple(selected) => selected.contains(key),
            Self::All => true,
        }
    }

    /// Returns `true` when no items are selected.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns `true` when all items are selected.
    #[must_use]
    pub const fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns the first selected key when one is available.
    #[must_use]
    pub fn first(&self) -> Option<&Key> {
        match self {
            Self::Single(key) => Some(key),
            Self::Multiple(keys) => keys.first(),
            Self::Empty | Self::All => None,
        }
    }

    /// Returns the number of selected items, or `None` for `All`.
    #[must_use]
    pub fn count(&self) -> Option<usize> {
        match self {
            Self::Empty => Some(0),
            Self::Single(_) => Some(1),
            Self::Multiple(keys) => Some(keys.len()),
            Self::All => None,
        }
    }

    /// Returns the number of selected items.
    ///
    /// `All` returns `0`; callers that need to distinguish that case should
    /// prefer [`Self::count`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.count().unwrap_or(0)
    }

    /// Iterates over the selected keys.
    ///
    /// `All` returns an empty iterator because it must be resolved against a
    /// concrete collection.
    #[must_use]
    pub fn keys(&self) -> Box<dyn Iterator<Item = &Key> + '_> {
        match self {
            Self::Empty | Self::All => Box::new(core::iter::empty()),
            Self::Single(key) => Box::new(core::iter::once(key)),
            Self::Multiple(keys) => Box::new(keys.iter()),
        }
    }
}

/// Controls how disabled items behave in selection contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DisabledBehavior {
    /// Disabled items are skipped during keyboard navigation.
    #[default]
    Skip,

    /// Disabled items are focusable but not selectable.
    FocusOnly,
}

/// Callback for item action (Enter, double-click, tap in replace mode).
///
/// Distinct from selection change: action activates the item associated with
/// the provided [`Key`].
pub type OnAction = Option<Callback<dyn Fn(Key) + Send + Sync>>;

/// The full selection state for a collection-based component.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct State {
    /// Which items are currently selected.
    pub selected_keys: Set,

    /// The anchor for range selection.
    pub anchor_key: Option<Key>,

    /// The item that currently has focus.
    pub focused_key: Option<Key>,

    /// Keys that are disabled and excluded from selection operations.
    pub disabled_keys: BTreeSet<Key>,

    /// Controls how disabled items behave.
    pub disabled_behavior: DisabledBehavior,

    /// Selection mode for this instance.
    pub mode: Mode,

    /// Selection behavior for this instance.
    pub behavior: Behavior,

    /// Whether touch-based selection mode is currently active.
    pub selection_mode_active: bool,

    /// When set, further selections are blocked once the limit is reached.
    pub max_selection: Option<usize>,
}

impl State {
    /// Creates a new selection state with the given mode and behavior.
    #[must_use]
    pub fn new(mode: Mode, behavior: Behavior) -> Self {
        Self {
            mode,
            behavior,
            ..Self::default()
        }
    }

    /// Returns `true` when `key` is currently selected.
    #[must_use]
    pub fn is_selected(&self, key: &Key) -> bool {
        self.selected_keys.contains(key)
    }

    /// Returns `true` when `key` is disabled.
    #[must_use]
    pub fn is_disabled(&self, key: &Key) -> bool {
        self.disabled_keys.contains(key)
    }

    /// Selects `key`, respecting the current mode and behavior.
    #[must_use]
    pub fn select(&self, key: Key) -> Self {
        if self.mode == Mode::None || self.is_disabled(&key) {
            return self.clone();
        }

        let selected_keys = match self.mode {
            Mode::None => return self.clone(),

            Mode::Single => Set::Single(key.clone()),

            Mode::Multiple => match self.behavior {
                Behavior::Toggle => {
                    let mut selected = match &self.selected_keys {
                        Set::Multiple(existing) => existing.clone(),

                        Set::Single(existing) => {
                            let mut keys = BTreeSet::new();

                            keys.insert(existing.clone());

                            keys
                        }

                        Set::All => return self.clone(),

                        Set::Empty => BTreeSet::new(),
                    };

                    selected.insert(key.clone());

                    Set::Multiple(selected)
                }

                Behavior::Replace => {
                    let mut selected = BTreeSet::new();

                    selected.insert(key.clone());

                    Set::Multiple(selected)
                }
            },
        };

        Self {
            selected_keys,
            anchor_key: Some(key),
            ..self.clone()
        }
    }

    /// Deselects `key` if it is currently selected.
    #[must_use]
    pub fn deselect(&self, key: &Key) -> Self {
        let selected_keys = match &self.selected_keys {
            Set::All | Set::Empty => return self.clone(),

            Set::Single(selected) => {
                if selected == key {
                    Set::Empty
                } else {
                    return self.clone();
                }
            }

            Set::Multiple(selected) => {
                let mut next = selected.clone();

                next.remove(key);

                if next.is_empty() {
                    Set::Empty
                } else {
                    Set::Multiple(next)
                }
            }
        };

        Self {
            selected_keys: selected_keys.clone(),
            selection_mode_active: self.selection_mode_active && !selected_keys.is_empty(),
            ..self.clone()
        }
    }

    /// Deselects `key` when the current selection is [`Set::All`].
    #[must_use]
    pub fn deselect_from_all<T, C: Collection<T>>(&self, key: &Key, collection: &C) -> Self {
        match &self.selected_keys {
            Set::All => {
                let remaining = collection
                    .item_keys()
                    .filter(|candidate| *candidate != key)
                    .cloned()
                    .collect();

                Self {
                    selected_keys: Set::Multiple(remaining),
                    ..self.clone()
                }
            }

            _ => self.deselect(key),
        }
    }

    /// Toggles the selection state of `key`.
    #[must_use]
    pub fn toggle<T, C: Collection<T>>(&self, key: Key, collection: &C) -> Self {
        if self.is_selected(&key) {
            match &self.selected_keys {
                Set::All => self.deselect_from_all(&key, collection),

                _ => self.deselect(&key),
            }
        } else {
            self.select(key)
        }
    }

    /// Selects all items when the mode is [`Mode::Multiple`].
    #[must_use]
    pub fn select_all(&self) -> Self {
        if self.mode != Mode::Multiple {
            return self.clone();
        }

        Self {
            selected_keys: Set::All,
            ..self.clone()
        }
    }

    /// Clears the selection.
    #[must_use]
    pub fn clear(&self) -> Self {
        Self {
            selected_keys: Set::Empty,
            anchor_key: None,
            selection_mode_active: false,
            ..self.clone()
        }
    }

    /// Extends the selection from the current anchor to `key`.
    #[must_use]
    pub fn extend_selection<T: Clone, C: Collection<T>>(&self, key: Key, collection: &C) -> Self {
        if self.mode == Mode::None {
            return self.clone();
        }

        if self.mode == Mode::Single {
            return self.select(key);
        }

        let anchor = match &self.anchor_key {
            Some(anchor) => anchor.clone(),
            None => return self.select(key),
        };

        if !collection.contains_key(&anchor) {
            return self.select(key);
        }

        let mut in_range = false;
        let mut range_keys = BTreeSet::new();

        for node in collection.nodes() {
            if !node.is_focusable() {
                continue;
            }

            let is_anchor = node.key == anchor;
            let is_target = node.key == key;

            if is_anchor || is_target {
                in_range = !in_range;

                if !self.is_disabled(&node.key) {
                    range_keys.insert(node.key.clone());
                }
            } else if in_range && !self.is_disabled(&node.key) {
                range_keys.insert(node.key.clone());
            }

            if is_anchor && is_target {
                break;
            }
        }

        let existing = match &self.selected_keys {
            Set::Multiple(selected) => selected.clone(),
            _ => BTreeSet::new(),
        };

        let merged = existing.into_iter().chain(range_keys).collect();

        Self {
            selected_keys: Set::Multiple(merged),
            focused_key: Some(key),
            ..self.clone()
        }
    }

    /// Sets the focused key without changing the selection.
    #[must_use]
    pub fn set_focus(&self, key: Key) -> Self {
        Self {
            focused_key: Some(key),
            ..self.clone()
        }
    }

    /// Replaces the disabled key set.
    #[must_use]
    pub fn with_disabled(self, disabled_keys: BTreeSet<Key>) -> Self {
        Self {
            disabled_keys,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, sync::Arc, vec, vec::Vec};
    use core::sync::atomic::{AtomicBool, Ordering};

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

    fn multiple_toggle_state() -> State {
        State::new(Mode::Multiple, Behavior::Toggle)
    }

    #[test]
    fn mode_default_is_none() {
        assert_eq!(Mode::default(), Mode::None);
    }

    #[test]
    fn behavior_default_is_toggle() {
        assert_eq!(Behavior::default(), Behavior::Toggle);
    }

    #[test]
    fn disabled_behavior_default_is_skip() {
        assert_eq!(DisabledBehavior::default(), DisabledBehavior::Skip);
    }

    #[test]
    fn set_helpers_cover_all_variants() {
        let mut selected = BTreeSet::new();

        selected.insert(Key::int(1));
        selected.insert(Key::int(2));

        let empty = Set::Empty;

        let single = Set::Single(Key::int(3));

        let multiple = Set::Multiple(selected);

        let all = Set::All;

        assert!(!empty.contains(&Key::int(1)));
        assert!(single.contains(&Key::int(3)));
        assert!(multiple.contains(&Key::int(2)));
        assert!(all.contains(&Key::int(999)));

        assert!(empty.is_empty());
        assert!(!single.is_empty());
        assert!(!empty.is_all());
        assert!(!single.is_all());
        assert!(!multiple.is_all());
        assert!(all.is_all());

        assert_eq!(empty.first(), None);
        assert_eq!(single.first(), Some(&Key::int(3)));
        assert_eq!(multiple.first(), Some(&Key::int(1)));
        assert_eq!(all.first(), None);

        assert_eq!(empty.count(), Some(0));
        assert_eq!(single.count(), Some(1));
        assert_eq!(multiple.count(), Some(2));
        assert_eq!(all.count(), None);

        assert_eq!(all.len(), 0);
        assert_eq!(single.len(), 1);

        let empty_keys = empty.keys().cloned().collect::<Vec<_>>();

        let single_keys = single.keys().cloned().collect::<Vec<_>>();

        let multiple_keys = multiple.keys().cloned().collect::<Vec<_>>();

        let all_keys = all.keys().cloned().collect::<Vec<_>>();

        assert!(empty_keys.is_empty());
        assert_eq!(single_keys, vec![Key::int(3)]);
        assert_eq!(multiple_keys, vec![Key::int(1), Key::int(2)]);
        assert!(all_keys.is_empty());
    }

    #[test]
    fn state_new_sets_mode_and_behavior() {
        let state = State::new(Mode::Single, Behavior::Replace);

        assert_eq!(state.mode, Mode::Single);
        assert_eq!(state.behavior, Behavior::Replace);
        assert_eq!(state.selected_keys, Set::Empty);
        assert_eq!(state.anchor_key, None);
        assert_eq!(state.focused_key, None);
        assert_eq!(state.disabled_behavior, DisabledBehavior::Skip);
        assert!(!state.selection_mode_active);
        assert_eq!(state.max_selection, None);
    }

    #[test]
    fn select_is_noop_in_none_mode() {
        let state = State::new(Mode::None, Behavior::Toggle);

        assert_eq!(state.select(Key::int(1)), state);
    }

    #[test]
    fn select_replaces_in_single_mode() {
        let state = State::new(Mode::Single, Behavior::Toggle).select(Key::int(1));

        let next = state.select(Key::int(2));

        assert_eq!(next.selected_keys, Set::Single(Key::int(2)));
        assert_eq!(next.anchor_key, Some(Key::int(2)));
    }

    #[test]
    fn select_accumulates_in_multiple_toggle_mode() {
        let state = multiple_toggle_state()
            .select(Key::int(1))
            .select(Key::int(2));

        let Set::Multiple(selected) = state.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(
            selected.into_iter().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2)]
        );
    }

    #[test]
    fn select_materializes_single_into_multiple_toggle_mode() {
        let state = State {
            selected_keys: Set::Single(Key::int(1)),
            ..multiple_toggle_state()
        };

        let next = state.select(Key::int(2));

        let Set::Multiple(selected) = next.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(
            selected.into_iter().collect::<Vec<_>>(),
            vec![Key::int(1), Key::int(2)]
        );
    }

    #[test]
    fn select_is_noop_when_all_is_already_selected() {
        let state = State {
            selected_keys: Set::All,
            anchor_key: Some(Key::int(1)),
            ..multiple_toggle_state()
        };

        assert_eq!(state.select(Key::int(3)), state);
    }

    #[test]
    fn select_replaces_in_multiple_replace_mode() {
        let state = State::new(Mode::Multiple, Behavior::Replace)
            .select(Key::int(1))
            .select(Key::int(2));

        let Set::Multiple(selected) = state.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(selected.into_iter().collect::<Vec<_>>(), vec![Key::int(2)]);
    }

    #[test]
    fn select_is_noop_for_disabled_key() {
        let state = State::new(Mode::Multiple, Behavior::Toggle).with_disabled({
            let mut disabled = BTreeSet::new();

            disabled.insert(Key::int(2));

            disabled
        });

        assert_eq!(state.select(Key::int(2)), state);
        assert!(state.is_disabled(&Key::int(2)));
        assert!(!state.is_disabled(&Key::int(1)));
    }

    #[test]
    fn deselect_clears_selection_mode_when_last_item_removed() {
        let state = State {
            selected_keys: Set::Single(Key::int(1)),
            selection_mode_active: true,
            ..State::new(Mode::Multiple, Behavior::Toggle)
        };

        let next = state.deselect(&Key::int(1));

        assert_eq!(next.selected_keys, Set::Empty);
        assert!(!next.selection_mode_active);
    }

    #[test]
    fn deselect_is_noop_for_empty_and_all_selection() {
        let empty = multiple_toggle_state();

        assert_eq!(empty.deselect(&Key::int(1)), empty);

        let all = State {
            selected_keys: Set::All,
            ..multiple_toggle_state()
        };

        assert_eq!(all.deselect(&Key::int(1)), all);
    }

    #[test]
    fn deselect_is_noop_for_nonmatching_single_selection() {
        let state = State {
            selected_keys: Set::Single(Key::int(1)),
            ..multiple_toggle_state()
        };

        assert_eq!(state.deselect(&Key::int(2)), state);
    }

    #[test]
    fn deselect_removes_key_from_multiple_selection() {
        let state = State {
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(1), Key::int(2)])),
            selection_mode_active: true,
            ..multiple_toggle_state()
        };

        let next = state.deselect(&Key::int(2));

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(1)]))
        );
        assert!(next.selection_mode_active);
    }

    #[test]
    fn deselect_can_empty_multiple_selection() {
        let state = State {
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(2)])),
            selection_mode_active: true,
            ..multiple_toggle_state()
        };

        let next = state.deselect(&Key::int(2));

        assert_eq!(next.selected_keys, Set::Empty);
        assert!(!next.selection_mode_active);
    }

    #[test]
    fn select_in_none_mode_with_non_default_state_is_identity() {
        // Even when anchor_key, focused_key, and selection_mode_active are all
        // populated, a `Mode::None` state must return an identical clone from
        // `select()` — the early-return at the top of the function preserves
        // every field, not just the mode.
        let state = State {
            mode: Mode::None,
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(2)])),
            anchor_key: Some(Key::int(2)),
            focused_key: Some(Key::int(2)),
            selection_mode_active: true,
            ..multiple_toggle_state()
        };

        assert_eq!(state.select(Key::int(1)), state);
    }

    #[test]
    fn deselect_from_all_delegates_to_deselect_for_multiple_set() {
        // When `selected_keys` is not `Set::All`, `deselect_from_all` must
        // delegate to `deselect()` — covering the `_` arm of the inner match.
        let collection = fixture_collection();

        let state = State {
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(1), Key::int(2)])),
            selection_mode_active: true,
            ..multiple_toggle_state()
        };

        let next = state.deselect_from_all(&Key::int(1), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(2)]))
        );
        assert!(next.selection_mode_active);
    }

    #[test]
    fn toggle_from_all_uses_collection_complement() {
        let collection = fixture_collection();

        let state = State {
            selected_keys: Set::All,
            ..multiple_toggle_state()
        };

        let next = state.toggle(Key::int(3), &collection);

        let Set::Multiple(selected) = next.selected_keys else {
            panic!("expected concrete selection");
        };

        assert!(!selected.contains(&Key::int(3)));
        assert_eq!(selected.len(), 4);
    }

    #[test]
    fn toggle_deselects_selected_key_from_concrete_selection() {
        let collection = fixture_collection();

        let state = State {
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(1), Key::int(2)])),
            ..multiple_toggle_state()
        };

        let next = state.toggle(Key::int(2), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(1)]))
        );
    }

    #[test]
    fn toggle_selects_unselected_key() {
        let collection = fixture_collection();

        let state = multiple_toggle_state().select(Key::int(1));

        let next = state.toggle(Key::int(2), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(1), Key::int(2)]))
        );
    }

    #[test]
    fn select_all_only_applies_in_multiple_mode() {
        let single = State::new(Mode::Single, Behavior::Toggle).select_all();

        let multiple = multiple_toggle_state().select_all();

        assert_eq!(single.selected_keys, Set::Empty);
        assert_eq!(multiple.selected_keys, Set::All);
    }

    #[test]
    fn clear_resets_anchor_and_selection_mode() {
        let state = State {
            selected_keys: Set::Single(Key::int(1)),
            anchor_key: Some(Key::int(1)),
            selection_mode_active: true,
            ..multiple_toggle_state()
        };

        let next = state.clear();

        assert_eq!(next.selected_keys, Set::Empty);
        assert_eq!(next.anchor_key, None);
        assert!(!next.selection_mode_active);
        assert_eq!(next.mode, Mode::Multiple);
        assert_eq!(next.behavior, Behavior::Toggle);
    }

    #[test]
    fn extend_selection_collects_focusable_range() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(2)),
            selected_keys: Set::Single(Key::int(2)),
            ..multiple_toggle_state()
        };

        let next = state.extend_selection(Key::int(5), &collection);

        let Set::Multiple(selected) = next.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(
            selected.into_iter().collect::<Vec<_>>(),
            vec![Key::int(2), Key::int(3), Key::int(4), Key::int(5)]
        );
        assert_eq!(next.focused_key, Some(Key::int(5)));
        assert_eq!(next.anchor_key, Some(Key::int(2)));
    }

    #[test]
    fn extend_selection_is_noop_in_none_mode() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(2)),
            ..State::new(Mode::None, Behavior::Toggle)
        };

        assert_eq!(state.extend_selection(Key::int(5), &collection), state);
    }

    #[test]
    fn extend_selection_delegates_to_select_in_single_mode() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(2)),
            ..State::new(Mode::Single, Behavior::Toggle)
        };

        let next = state.extend_selection(Key::int(5), &collection);

        assert_eq!(next.selected_keys, Set::Single(Key::int(5)));
        assert_eq!(next.anchor_key, Some(Key::int(5)));
    }

    #[test]
    fn extend_selection_skips_disabled_keys() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(2)),
            disabled_keys: BTreeSet::from([Key::int(3)]),
            selected_keys: Set::Single(Key::int(2)),
            ..multiple_toggle_state()
        };

        let next = state.extend_selection(Key::int(4), &collection);

        let Set::Multiple(selected) = next.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(
            selected.into_iter().collect::<Vec<_>>(),
            vec![Key::int(2), Key::int(4)]
        );
    }

    #[test]
    fn extend_selection_same_key_yields_single_item_range() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(3)),
            ..multiple_toggle_state()
        };

        let next = state.extend_selection(Key::int(3), &collection);

        let Set::Multiple(selected) = next.selected_keys else {
            panic!("expected multiple selection");
        };

        assert_eq!(selected.into_iter().collect::<Vec<_>>(), vec![Key::int(3)]);
    }

    #[test]
    fn extend_selection_merges_range_with_existing_multiple_selection() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(3)),
            selected_keys: Set::Multiple(BTreeSet::from([Key::int(1)])),
            ..multiple_toggle_state()
        };

        let next = state.extend_selection(Key::int(5), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([
                Key::int(1),
                Key::int(3),
                Key::int(4),
                Key::int(5),
            ]))
        );
    }

    #[test]
    fn extend_selection_falls_back_when_anchor_is_stale() {
        let collection = fixture_collection();

        let state = State {
            anchor_key: Some(Key::int(999)),
            ..multiple_toggle_state()
        };

        let next = state.extend_selection(Key::int(4), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(4)]))
        );
        assert_eq!(next.anchor_key, Some(Key::int(4)));
    }

    #[test]
    fn extend_selection_without_anchor_falls_back_to_select() {
        let collection = fixture_collection();

        let state = multiple_toggle_state();

        let next = state.extend_selection(Key::int(1), &collection);

        assert_eq!(
            next.selected_keys,
            Set::Multiple(BTreeSet::from([Key::int(1)]))
        );
        assert_eq!(next.anchor_key, Some(Key::int(1)));
    }

    #[test]
    fn set_focus_updates_only_focus() {
        let state = multiple_toggle_state().set_focus(Key::int(4));

        assert_eq!(state.focused_key, Some(Key::int(4)));
        assert_eq!(state.selected_keys, Set::Empty);
    }

    #[test]
    fn on_action_uses_callback_abstraction() {
        let called = Arc::new(AtomicBool::new(false));

        let callback = {
            let called = Arc::clone(&called);
            Some(Callback::new(move |key| {
                if key == Key::int(7) {
                    called.store(true, Ordering::Relaxed);
                }
            }))
        };

        let on_action: OnAction = callback.clone();

        assert!(on_action.is_some());

        on_action.expect("OnAction should exist")(Key::int(7));

        assert!(called.load(Ordering::Relaxed));
    }
}
