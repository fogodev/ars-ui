//! Keyboard-navigation test helpers for focus-zone and keyboard event testing.

use alloc::{collections::BTreeSet, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};

use ars_core::KeyboardKey;

use crate::{DomEvent, FocusZone, FocusZoneOptions};

/// A simulated keyboard event for use in unit tests.
#[derive(Debug)]
pub struct SimulatedKeyEvent {
    /// The DOM `key` value exposed by the event.
    pub key: &'static str,

    /// Whether the Shift modifier is pressed.
    pub shift: bool,

    /// Whether the Ctrl modifier is pressed.
    pub ctrl: bool,

    /// Whether the Meta/Cmd modifier is pressed.
    pub meta: bool,

    /// Whether the Alt/Option modifier is pressed.
    pub alt: bool,

    /// Whether `prevent_default()` has been called.
    pub default_prevented: AtomicBool,

    /// Whether `stop_propagation()` has been called.
    pub propagation_stopped: AtomicBool,
}

impl SimulatedKeyEvent {
    /// Creates a `keydown` event with the provided key and no modifiers.
    #[must_use]
    pub const fn key(key: &'static str) -> Self {
        Self {
            key,
            shift: false,
            ctrl: false,
            meta: false,
            alt: false,
            default_prevented: AtomicBool::new(false),
            propagation_stopped: AtomicBool::new(false),
        }
    }

    /// Marks the event as having the Shift modifier pressed.
    #[must_use]
    pub const fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Marks the event as having the Ctrl modifier pressed.
    #[must_use]
    pub const fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Marks the event as having the Meta/Cmd modifier pressed.
    #[must_use]
    pub const fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }

    /// Marks the event as having the Alt/Option modifier pressed.
    #[must_use]
    pub const fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

// `AtomicBool` does not implement `Clone`, so the spec's derived `Clone`
// example must be implemented manually to preserve the same public fields.
impl Clone for SimulatedKeyEvent {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            shift: self.shift,
            ctrl: self.ctrl,
            meta: self.meta,
            alt: self.alt,
            default_prevented: AtomicBool::new(self.default_prevented.load(Ordering::Relaxed)),
            propagation_stopped: AtomicBool::new(self.propagation_stopped.load(Ordering::Relaxed)),
        }
    }
}

impl DomEvent for SimulatedKeyEvent {
    fn key(&self) -> Option<&str> {
        Some(self.key)
    }

    fn shift_key(&self) -> bool {
        self.shift
    }

    fn ctrl_key(&self) -> bool {
        self.ctrl
    }

    fn meta_key(&self) -> bool {
        self.meta
    }

    fn alt_key(&self) -> bool {
        self.alt
    }

    fn event_type(&self) -> &str {
        "keydown"
    }

    fn prevent_default(&self) {
        self.default_prevented.store(true, Ordering::Relaxed);
    }

    fn stop_propagation(&self) {
        self.propagation_stopped.store(true, Ordering::Relaxed);
    }
}

/// A recorder that captures keyboard-navigation side effects in unit tests.
#[derive(Debug)]
pub struct NavigationRecorder {
    /// The ordered list of recorded navigation events.
    pub events: Vec<NavigationEvent>,
}

/// A keyboard-navigation side effect emitted by a test harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationEvent {
    /// Focus moved from one item index to another.
    FocusMoved {
        /// The previously focused index.
        from: usize,
        /// The newly focused index.
        to: usize,
    },

    /// Selection changed to the provided item index.
    SelectionChanged {
        /// The selected index.
        index: usize,
    },

    /// The currently focused item was activated.
    Activated {
        /// The activated index.
        index: usize,
    },

    /// Navigation escaped the current composite widget.
    Escaped,
}

impl NavigationRecorder {
    /// Creates an empty navigation recorder.
    #[must_use]
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Records a focus transition between two item indices.
    pub fn record_focus_move(&mut self, from: usize, to: usize) {
        self.events.push(NavigationEvent::FocusMoved { from, to });
    }

    /// Asserts that the recorded focus transitions match `expected` exactly.
    pub fn assert_focus_sequence(&self, expected: &[(usize, usize)]) {
        let actual: Vec<(usize, usize)> = self
            .events
            .iter()
            .filter_map(|event| match event {
                NavigationEvent::FocusMoved { from, to } => Some((*from, *to)),
                NavigationEvent::SelectionChanged { .. }
                | NavigationEvent::Activated { .. }
                | NavigationEvent::Escaped => None,
            })
            .collect();

        assert_eq!(actual, expected, "Focus navigation sequence mismatch");
    }
}

impl Default for NavigationRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// A `FocusZone` test harness that records focus movement after each key press.
///
/// This helper defaults `is_rtl` to `false`; callers that need explicit RTL
/// behavior should test `FocusZone::handle_key()` directly.
#[derive(Debug)]
pub struct FocusZoneTestHarness {
    /// The focus zone under test.
    pub zone: FocusZone,

    /// The current focused item index tracked by the harness.
    pub current_index: usize,

    /// The recorder storing navigation side effects.
    pub recorder: NavigationRecorder,

    /// Disabled item indices skipped by keyboard navigation when configured.
    pub disabled_indices: BTreeSet<usize>,
}

impl FocusZoneTestHarness {
    /// Creates a new focus-zone harness starting at item index `0`.
    #[must_use]
    pub const fn new(options: FocusZoneOptions, item_count: usize) -> Self {
        Self {
            zone: FocusZone::new(options, item_count),
            current_index: 0,
            recorder: NavigationRecorder::new(),
            disabled_indices: BTreeSet::new(),
        }
    }

    /// Marks an item index as disabled for subsequent navigation.
    pub fn disable(&mut self, index: usize) {
        self.disabled_indices.insert(index);
    }

    /// Sends a key to the underlying `FocusZone`.
    ///
    /// Returns `true` when the key changed focus and `false` when the key was
    /// not handled.
    pub fn send_key(&mut self, key: KeyboardKey) -> bool {
        let is_disabled = |index: usize| self.disabled_indices.contains(&index);

        if let Some(next) = self.zone.handle_key(key, false, is_disabled) {
            self.recorder.record_focus_move(self.current_index, next);
            self.current_index = next;
            self.zone.active_index = next;

            true
        } else {
            false
        }
    }

    /// Asserts that the harness focus is currently at `expected_index`.
    pub fn assert_at(&self, expected_index: usize) {
        assert_eq!(
            self.current_index, expected_index,
            "Expected focus at index {}, but was at {}",
            expected_index, self.current_index
        );
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::{boxed::Box, string::String, vec};
    use core::{any::Any, panic::AssertUnwindSafe, sync::atomic::Ordering};
    use std::{
        panic::{catch_unwind, set_hook, take_hook},
        sync::{Mutex, OnceLock},
    };

    use super::*;
    use crate::{
        AriaAttribute, AriaRole, AriaValidationError, AriaValidator, FocusZoneDirection,
        LiveAnnouncer,
    };

    fn panic_message(payload: Box<dyn Any + Send>) -> String {
        match payload.downcast::<String>() {
            Ok(message) => *message,
            Err(payload) => match payload.downcast::<&'static str>() {
                Ok(message) => String::from(*message),
                Err(_) => String::from("non-string panic payload"),
            },
        }
    }

    fn catch_panic_silently(
        f: impl FnOnce() + core::panic::UnwindSafe,
    ) -> Result<(), Box<dyn Any + Send>> {
        static PANIC_HOOK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        let lock = PANIC_HOOK_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().expect("panic hook lock poisoned");
        let previous_hook = take_hook();

        set_hook(Box::new(|_| {}));
        let result = catch_unwind(f);
        set_hook(previous_hook);

        result
    }

    #[test]
    fn simulated_key_event_key_constructor_sets_defaults() {
        let event = SimulatedKeyEvent::key("Tab");

        assert_eq!(event.key, "Tab");
        assert!(!event.shift);
        assert!(!event.ctrl);
        assert!(!event.meta);
        assert!(!event.alt);
        assert!(!event.default_prevented.load(Ordering::Relaxed));
        assert!(!event.propagation_stopped.load(Ordering::Relaxed));
    }

    #[test]
    fn simulated_key_event_builder_sets_modifiers() {
        let event = SimulatedKeyEvent::key("a").with_shift().with_ctrl();

        assert!(event.shift);
        assert!(event.ctrl);
        assert!(!event.meta);
        assert!(!event.alt);
    }

    #[test]
    fn simulated_key_event_builder_can_enable_all_modifiers() {
        let event = SimulatedKeyEvent::key("a")
            .with_shift()
            .with_ctrl()
            .with_meta()
            .with_alt();

        assert!(event.shift);
        assert!(event.ctrl);
        assert!(event.meta);
        assert!(event.alt);
    }

    #[test]
    fn simulated_key_event_implements_dom_event() {
        let event = SimulatedKeyEvent::key("Tab");

        assert_eq!(event.key(), Some("Tab"));
        assert_eq!(event.event_type(), "keydown");
        assert!(!event.shift_key());
        assert!(!event.ctrl_key());
        assert!(!event.meta_key());
        assert!(!event.alt_key());
    }

    #[test]
    fn simulated_key_event_prevent_default_and_stop_propagation_set_flags() {
        let event = SimulatedKeyEvent::key("Enter");

        event.prevent_default();
        event.stop_propagation();

        assert!(event.default_prevented.load(Ordering::Relaxed));
        assert!(event.propagation_stopped.load(Ordering::Relaxed));
    }

    #[test]
    fn navigation_recorder_new_starts_empty() {
        let recorder = NavigationRecorder::new();

        assert!(recorder.events.is_empty());
    }

    #[test]
    fn navigation_recorder_default_starts_empty() {
        let recorder = NavigationRecorder::default();

        assert!(recorder.events.is_empty());
    }

    #[test]
    fn record_focus_move_adds_focus_moved_event() {
        let mut recorder = NavigationRecorder::new();

        recorder.record_focus_move(0, 1);

        assert_eq!(
            recorder.events,
            vec![NavigationEvent::FocusMoved { from: 0, to: 1 }]
        );
    }

    #[test]
    fn assert_focus_sequence_matches_exact_sequence() {
        let recorder = NavigationRecorder {
            events: vec![
                NavigationEvent::SelectionChanged { index: 0 },
                NavigationEvent::FocusMoved { from: 0, to: 1 },
                NavigationEvent::Activated { index: 1 },
                NavigationEvent::FocusMoved { from: 1, to: 2 },
            ],
        };

        recorder.assert_focus_sequence(&[(0, 1), (1, 2)]);
    }

    #[test]
    fn assert_focus_sequence_panics_on_mismatch() {
        let recorder = NavigationRecorder {
            events: vec![NavigationEvent::FocusMoved { from: 0, to: 1 }],
        };

        let panic = catch_panic_silently(AssertUnwindSafe(|| {
            recorder.assert_focus_sequence(&[(0, 2)]);
        }))
        .expect_err("mismatched focus sequence should panic");

        assert!(panic_message(panic).contains("Focus navigation sequence mismatch"));
    }

    #[test]
    fn focus_zone_test_harness_new_starts_at_zero() {
        let harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);

        assert_eq!(harness.current_index, 0);
        assert_eq!(harness.zone.active_index, 0);
        assert!(harness.recorder.events.is_empty());
        assert!(harness.disabled_indices.is_empty());
    }

    #[test]
    fn send_key_moves_vertical_focus_zone_and_returns_true() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);

        assert!(harness.send_key(KeyboardKey::ArrowDown));
        harness.assert_at(1);
        harness.recorder.assert_focus_sequence(&[(0, 1)]);
    }

    #[test]
    fn send_key_returns_false_for_unhandled_key() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);

        assert!(!harness.send_key(KeyboardKey::Tab));
        harness.assert_at(0);
        assert!(harness.recorder.events.is_empty());
    }

    #[test]
    fn send_key_skips_disabled_items() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);
        harness.disable(1);
        harness.disable(2);

        assert!(harness.send_key(KeyboardKey::ArrowDown));
        harness.assert_at(3);
        harness.recorder.assert_focus_sequence(&[(0, 3)]);
    }

    #[test]
    fn send_key_returns_false_when_no_enabled_target_exists() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 3);
        harness.disable(1);
        harness.disable(2);

        assert!(!harness.send_key(KeyboardKey::ArrowDown));
        harness.assert_at(0);
        assert!(harness.recorder.events.is_empty());
    }

    #[test]
    fn assert_at_panics_with_descriptive_message() {
        let harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);

        let panic = catch_panic_silently(AssertUnwindSafe(|| {
            harness.assert_at(1);
        }))
        .expect_err("assert_at should panic on mismatch");

        let message = panic_message(panic);

        assert!(message.contains("Expected focus at index 1, but was at 0"));
    }

    #[test]
    fn vertical_zone_wraps() {
        let options = FocusZoneOptions {
            direction: FocusZoneDirection::Vertical,
            wrap: true,
            ..FocusZoneOptions::default()
        };
        let mut harness = FocusZoneTestHarness::new(options, 3);

        harness.send_key(KeyboardKey::ArrowDown);
        harness.send_key(KeyboardKey::ArrowDown);
        harness.send_key(KeyboardKey::ArrowDown);

        harness.assert_at(0);
        harness
            .recorder
            .assert_focus_sequence(&[(0, 1), (1, 2), (2, 0)]);
    }

    #[test]
    fn zone_skips_disabled_items() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);
        harness.disable(1);
        harness.disable(2);

        harness.send_key(KeyboardKey::ArrowDown);

        harness.assert_at(3);
    }

    #[test]
    fn home_end_navigation() {
        let mut harness = FocusZoneTestHarness::new(FocusZoneOptions::default(), 5);

        harness.send_key(KeyboardKey::End);
        harness.assert_at(4);

        harness.send_key(KeyboardKey::Home);
        harness.assert_at(0);
    }

    #[test]
    fn aria_validator_catches_abstract_role() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Widget, &[], false, &[]);

        assert!(validator.has_errors());
        assert!(matches!(
            validator.errors()[0],
            AriaValidationError::AbstractRoleUsed { .. }
        ));
    }

    #[test]
    fn aria_validator_catches_missing_required_attr() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Slider, &[], false, &[]);

        assert!(validator.errors().iter().any(|error| matches!(
            error,
            AriaValidationError::MissingRequiredAttribute {
                missing_attr: "aria-valuenow",
                ..
            }
        )));
    }

    #[test]
    fn live_announcer_deduplicates_voiceover() {
        let mut announcer = LiveAnnouncer::new();

        announcer.announce("Test message");
        announcer.notify_announced();
        announcer.announce("Test message");
        announcer.notify_announced();
        announcer.announce("Test message");
    }

    #[test]
    fn simulated_key_event_clone_preserves_flags() {
        let event = SimulatedKeyEvent::key("Escape")
            .with_shift()
            .with_meta()
            .with_alt();

        event.prevent_default();
        event.stop_propagation();

        let cloned = event.clone();

        assert_eq!(cloned.key, "Escape");
        assert!(cloned.shift);
        assert!(cloned.meta);
        assert!(cloned.alt);
        assert!(cloned.default_prevented.load(Ordering::Relaxed));
        assert!(cloned.propagation_stopped.load(Ordering::Relaxed));
    }

    #[test]
    fn aria_validator_example_uses_real_attribute_shape() {
        let mut validator = AriaValidator::new();

        validator.check_role(
            AriaRole::Slider,
            &[AriaAttribute::ValueNow(42.0)],
            false,
            &[],
        );

        assert!(validator.errors().iter().any(|error| matches!(
            error,
            AriaValidationError::MissingRequiredAttribute {
                missing_attr: "aria-valuemin",
                ..
            }
        )));
    }
}
