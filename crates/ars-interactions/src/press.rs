//! Press interaction types and state machine.
//!
//! The press interaction is the fundamental activation primitive that unifies
//! mouse click, touch tap, keyboard Enter/Space, and virtual cursor activation
//! into a single consistent model. It tracks whether the element is currently
//! being pressed and whether the pointer is within the element's bounds.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ars_core::{AttrMap, Callback, HtmlAttr, SharedFlag};

use crate::{KeyModifiers, PointerType};

/// The current state of the press state machine.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PressState {
    /// No active press.
    #[default]
    Idle,

    /// Press has begun; position relative to element not yet resolved.
    /// Transient zero-duration state; resolves within same event tick to
    /// `PressedInside` or `PressedOutside`.
    Pressing {
        /// The pointer modality that initiated the press.
        pointer_type: PointerType,
    },

    /// Press is active and the pointer is within the element bounds.
    PressedInside {
        /// The pointer modality that initiated the press.
        pointer_type: PointerType,
        /// The x-coordinate where the press originated, if available.
        origin_x: Option<f64>,
        /// The y-coordinate where the press originated, if available.
        origin_y: Option<f64>,
    },

    /// Press is active but the pointer has moved outside the element bounds.
    PressedOutside {
        /// The pointer modality that initiated the press.
        pointer_type: PointerType,
    },
}

impl PressState {
    /// Returns `true` when the element is in a committed pressed state.
    ///
    /// The transient `Pressing` state (which resolves within the same event tick
    /// before any render) returns `false` to prevent `data-ars-pressed` from flashing.
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        matches!(
            self,
            PressState::PressedInside { .. } | PressState::PressedOutside { .. }
        )
    }

    /// Returns `true` when pressed and the pointer is within element bounds.
    #[must_use]
    pub const fn is_pressed_inside(&self) -> bool {
        matches!(self, PressState::PressedInside { .. })
    }

    /// Returns `true` when the press interaction is disabled.
    ///
    /// This is an associated function, not a method — it does not depend on
    /// press state. Callers may also use `config.disabled` directly.
    #[must_use]
    pub const fn is_disabled(config: &PressConfig) -> bool {
        config.disabled
    }
}

// ---------------------------------------------------------------------------
// PressConfig
// ---------------------------------------------------------------------------

/// Configuration for press interaction behavior.
///
/// Controls how the press interaction responds to pointer, touch, and keyboard
/// input. Callbacks use [`Callback`] for automatic platform-appropriate pointer
/// type (`Rc` on wasm, `Arc` on native) and built-in `Clone`, `Debug`, and
/// `PartialEq` (by pointer identity).
#[derive(Clone, Debug, PartialEq)]
pub struct PressConfig {
    /// Whether the element is disabled. Disabled elements receive no press events.
    pub disabled: bool,

    /// Prevent text selection on press-and-hold. Defaults to `true` for
    /// button-like elements, `false` for text content.
    pub prevent_text_selection: bool,

    /// Whether to allow the press to continue when the pointer leaves the
    /// element while still pressed (useful for sliders, scroll pickers).
    /// When `false` (default), leaving the element while pressed transitions
    /// to `PressedOutside` and will not fire `on_press` on release.
    pub allow_press_on_exit: bool,

    /// Touch scroll cancellation threshold in pixels. If touch displacement
    /// exceeds this value before `touchend`, the press is cancelled (user
    /// intended to scroll). Default: 10 (matching React Aria).
    pub scroll_threshold_px: u16,

    /// Called when the element is pressed (pointer down AND within element).
    pub on_press_start: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called when press ends (pointer up, key up, or cancellation).
    pub on_press_end: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called on activation: pointer released inside the element, or Enter/Space
    /// released after having been pressed on this element.
    pub on_press: Option<Callback<dyn Fn(PressEvent)>>,

    /// Called when the pointer's inside/outside state changes while a press is
    /// active. `true` = pointer re-entered the element; `false` = pointer exited.
    pub on_press_change: Option<Callback<dyn Fn(bool)>>,

    /// Fired when a press is released (pointer up / key up / touch end),
    /// regardless of whether the release was inside or outside the element.
    /// Distinct from `on_press_end` (fires on any press conclusion) and
    /// `on_press` (fires only for activations inside the element).
    pub on_press_up: Option<Callback<dyn Fn(PressEvent)>>,

    /// Maximum duration to hold pointer capture before automatically releasing.
    /// Prevents stuck capture states caused by missed `pointerup` events.
    /// Defaults to 5000ms. Set to `None` to disable the timeout entirely.
    pub pointer_capture_timeout: Option<Duration>,

    /// When set, the press handler checks this flag on `pointerup`. If `true`,
    /// the press activation (`on_press`) is suppressed because a long-press
    /// already fired. See spec §8.7 Cross-Interaction Cancellation Protocol.
    pub long_press_cancel_flag: Option<SharedFlag>,
}

impl Default for PressConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            prevent_text_selection: true,
            allow_press_on_exit: false,
            scroll_threshold_px: 10,
            on_press_start: None,
            on_press_end: None,
            on_press: None,
            on_press_change: None,
            on_press_up: None,
            pointer_capture_timeout: Some(Duration::from_millis(5000)),
            long_press_cancel_flag: None,
        }
    }
}

// ---------------------------------------------------------------------------
// PressEventType
// ---------------------------------------------------------------------------

/// The kind of press event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressEventType {
    /// The press started (pointer down / key down within the element).
    PressStart,
    /// The press ended (any conclusion: release, cancel, blur).
    PressEnd,
    /// Activation: the press was released inside the element.
    Press,
    /// Fired when a press is released (pointer up / key up / touch end),
    /// regardless of whether the release was inside or outside the element.
    /// Distinct from `PressEnd` (fires on any press conclusion) and `Press`
    /// (fires only for activations inside the element).
    PressUp,
}

// ---------------------------------------------------------------------------
// PressEvent
// ---------------------------------------------------------------------------

/// A normalized press event, independent of input modality.
///
/// **Clone semantics:** Uses [`SharedFlag`] for propagation control so that
/// cloned events share the same propagation flag. `SharedFlag` uses
/// [`AtomicBool`](std::sync::atomic::AtomicBool) for the value (zero overhead
/// on wasm) and [`Arc`](std::sync::Arc) for shared ownership. Calling
/// [`continue_propagation()`](Self::continue_propagation) on any clone
/// affects the original and all other clones.
#[derive(Clone, Debug)]
pub struct PressEvent {
    /// How the press was initiated.
    pub pointer_type: PointerType,

    /// The type of event this represents.
    pub event_type: PressEventType,

    /// Client-space X coordinate. `None` for keyboard/virtual events.
    pub client_x: Option<f64>,

    /// Client-space Y coordinate. `None` for keyboard/virtual events.
    pub client_y: Option<f64>,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,

    /// Whether the pointer was within the element when this event fired.
    /// `false` when the press started inside but pointer moved outside.
    pub is_within_element: bool,

    /// When called, prevents the event handler from stopping propagation.
    /// By default, press events stop propagation. Call
    /// [`continue_propagation()`](Self::continue_propagation) to allow parent
    /// handlers to also receive the event.
    ///
    /// Uses [`SharedFlag`] so cloned events share propagation state across
    /// threads on native targets.
    pub continue_propagation: SharedFlag,
}

impl PressEvent {
    /// Allow the event to propagate to parent handlers.
    pub fn continue_propagation(&self) {
        self.continue_propagation.set(true);
    }

    /// Check whether propagation was allowed.
    #[must_use]
    pub fn should_propagate(&self) -> bool {
        self.continue_propagation.get()
    }

    /// Creates a child event sharing propagation state with the parent.
    /// The child event's `continue_propagation` points to the same flag.
    #[must_use]
    pub fn create_child_event(&self) -> PressEvent {
        PressEvent {
            pointer_type: self.pointer_type,
            event_type: self.event_type,
            client_x: self.client_x,
            client_y: self.client_y,
            modifiers: self.modifiers,
            is_within_element: self.is_within_element,
            continue_propagation: self.continue_propagation.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// PressResult
// ---------------------------------------------------------------------------

/// The output of [`use_press`], providing live attribute generation and state access.
///
/// `PressResult` attrs are **reactive, not one-shot snapshots**. Use
/// [`current_attrs()`](Self::current_attrs) inside the component's `connect()`
/// method to ensure attributes reflect the current state at DOM reconciliation.
#[derive(Debug)]
pub struct PressResult {
    /// Internal state handle — use [`current_attrs()`](Self::current_attrs) to
    /// produce a live `AttrMap`.
    state: Rc<RefCell<PressState>>,

    /// Whether the element is currently being pressed (reactive signal in adapter).
    pub pressed: bool,
}

impl PressResult {
    /// Produce a fresh [`AttrMap`] reflecting the current press state.
    ///
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    #[must_use]
    pub fn current_attrs(&self, config: &PressConfig) -> AttrMap {
        let state = self.state.borrow();
        let mut attrs = AttrMap::new();
        if state.is_pressed_inside() {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }
        if PressState::is_disabled(config) {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            // Disabled elements use aria-disabled="true" instead of HTML disabled
            // to allow tooltip on hover. Do NOT set pointer-events:none here.
            // Adapters prevent interaction via event handler removal.
        }
        // Note: data-ars-active is managed by the adapter layer (not current_attrs)
        // because it requires RAF-deferred removal after pointerup.
        attrs
    }
}

// ---------------------------------------------------------------------------
// use_press factory
// ---------------------------------------------------------------------------

/// Creates a press interaction state machine with the given configuration.
///
/// Returns a [`PressResult`] holding the initial `Idle` state. Event handlers
/// are registered as typed methods on the component's `Api` struct by the
/// framework adapter — this factory only creates the core state container.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "spec API takes ownership; adapters will consume the config for event handler registration"
)]
pub fn use_press(config: PressConfig) -> PressResult {
    let state = Rc::new(RefCell::new(PressState::Idle));
    let _is_disabled = config.disabled;
    let pressed = state.borrow().is_pressed_inside();

    PressResult { state, pressed }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use ars_core::{AttrValue, HtmlAttr};

    use super::*;

    // --- PressState tests ---

    #[test]
    fn press_state_idle_is_not_pressed() {
        assert!(!PressState::Idle.is_pressed());
        assert!(!PressState::Idle.is_pressed_inside());
    }

    #[test]
    fn press_state_pressed_inside_is_pressed() {
        let state = PressState::PressedInside {
            pointer_type: PointerType::Mouse,
            origin_x: Some(10.0),
            origin_y: Some(20.0),
        };
        assert!(state.is_pressed());
        assert!(state.is_pressed_inside());
    }

    #[test]
    fn press_state_pressing_is_not_committed() {
        let state = PressState::Pressing {
            pointer_type: PointerType::Touch,
        };
        assert!(!state.is_pressed());
    }

    // --- PressConfig tests ---

    #[test]
    fn press_config_debug_default_shows_none_callbacks() {
        let config = PressConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("disabled: false"));
        assert!(debug.contains("on_press_start: None"));
        assert!(debug.contains("on_press: None"));
        assert!(debug.contains("pointer_capture_timeout: Some(5s)"));
        assert!(debug.contains("long_press_cancel_flag: None"));
    }

    #[test]
    fn press_config_debug_with_callbacks_shows_callback() {
        let press_calls = Arc::new(AtomicUsize::new(0));
        let change_calls = Arc::new(AtomicUsize::new(0));
        let config = PressConfig {
            on_press: Some({
                let press_calls = Arc::clone(&press_calls);
                Callback::new(move |_: PressEvent| {
                    press_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_press_change: Some({
                let change_calls = Arc::clone(&change_calls);
                Callback::new(move |_: bool| {
                    change_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            long_press_cancel_flag: Some(SharedFlag::new(true)),
            ..PressConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("on_press: Some(Callback(..))"));
        assert!(debug.contains("on_press_change: Some(Callback(..))"));
        assert!(debug.contains("long_press_cancel_flag: Some(SharedFlag(true))"));
        let event = PressEvent {
            pointer_type: PointerType::Mouse,
            event_type: PressEventType::Press,
            client_x: Some(1.0),
            client_y: Some(2.0),
            modifiers: KeyModifiers::default(),
            is_within_element: true,
            continue_propagation: SharedFlag::new(false),
        };
        config.on_press.as_ref().expect("callback")(event);
        config.on_press_change.as_ref().expect("callback")(true);
        assert_eq!(press_calls.load(Ordering::SeqCst), 1);
        assert_eq!(change_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn press_config_default_values() {
        let config = PressConfig::default();
        assert!(!config.disabled);
        assert!(config.prevent_text_selection);
        assert!(!config.allow_press_on_exit);
        assert_eq!(config.scroll_threshold_px, 10);
        assert!(config.on_press_start.is_none());
        assert!(config.on_press_end.is_none());
        assert!(config.on_press.is_none());
        assert!(config.on_press_change.is_none());
        assert!(config.on_press_up.is_none());
        assert_eq!(
            config.pointer_capture_timeout,
            Some(Duration::from_millis(5000))
        );
        assert!(config.long_press_cancel_flag.is_none());
    }

    // --- PressEventType tests ---

    #[test]
    fn press_event_type_variants_are_distinct() {
        assert_ne!(PressEventType::PressStart, PressEventType::PressEnd);
        assert_ne!(PressEventType::Press, PressEventType::PressUp);
        assert_ne!(PressEventType::PressStart, PressEventType::Press);
    }

    #[test]
    fn press_event_type_is_copy() {
        let t = PressEventType::Press;
        let t2 = t;
        assert_eq!(t, t2);
    }

    // --- PressEvent tests ---

    #[test]
    fn press_event_stops_propagation_by_default() {
        let event = PressEvent {
            pointer_type: PointerType::Mouse,
            event_type: PressEventType::Press,
            client_x: Some(100.0),
            client_y: Some(200.0),
            modifiers: KeyModifiers::default(),
            is_within_element: true,
            continue_propagation: SharedFlag::new(false),
        };
        assert!(!event.should_propagate());
    }

    #[test]
    fn press_event_continue_propagation_enables_propagation() {
        let event = PressEvent {
            pointer_type: PointerType::Touch,
            event_type: PressEventType::PressStart,
            client_x: None,
            client_y: None,
            modifiers: KeyModifiers::default(),
            is_within_element: true,
            continue_propagation: SharedFlag::new(false),
        };
        event.continue_propagation();
        assert!(event.should_propagate());
    }

    #[test]
    fn press_event_create_child_shares_propagation_flag() {
        let parent = PressEvent {
            pointer_type: PointerType::Keyboard,
            event_type: PressEventType::PressEnd,
            client_x: None,
            client_y: None,
            modifiers: KeyModifiers::default(),
            is_within_element: true,
            continue_propagation: SharedFlag::new(false),
        };
        let child = parent.create_child_event();

        // Mutating child affects parent
        child.continue_propagation();
        assert!(parent.should_propagate());
    }

    #[test]
    fn press_event_should_propagate_reflects_shared_state() {
        let flag = SharedFlag::new(false);
        let event1 = PressEvent {
            pointer_type: PointerType::Mouse,
            event_type: PressEventType::Press,
            client_x: Some(10.0),
            client_y: Some(20.0),
            modifiers: KeyModifiers::default(),
            is_within_element: true,
            continue_propagation: flag.clone(),
        };
        let event2 = event1.clone();

        // Both start with propagation stopped
        assert!(!event1.should_propagate());
        assert!(!event2.should_propagate());

        // Setting on clone affects both
        event2.continue_propagation();
        assert!(event1.should_propagate());
        assert!(event2.should_propagate());
    }

    // --- PressState::is_disabled tests ---

    #[test]
    fn press_state_is_disabled_reads_config() {
        let enabled = PressConfig::default();
        assert!(!PressState::is_disabled(&enabled));

        let disabled = PressConfig {
            disabled: true,
            ..PressConfig::default()
        };
        assert!(PressState::is_disabled(&disabled));
    }

    // --- PressResult tests ---

    #[test]
    fn press_result_current_attrs_idle_is_empty() {
        let result = PressResult {
            state: Rc::new(RefCell::new(PressState::Idle)),
            pressed: false,
        };
        let config = PressConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-pressed")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-disabled")));
    }

    #[test]
    fn press_result_current_attrs_pressed_inside_sets_data_ars_pressed() {
        let result = PressResult {
            state: Rc::new(RefCell::new(PressState::PressedInside {
                pointer_type: PointerType::Mouse,
                origin_x: Some(10.0),
                origin_y: Some(20.0),
            })),
            pressed: true,
        };
        let config = PressConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-pressed")));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-pressed")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn press_result_current_attrs_disabled_sets_data_ars_disabled() {
        let result = PressResult {
            state: Rc::new(RefCell::new(PressState::Idle)),
            pressed: false,
        };
        let config = PressConfig {
            disabled: true,
            ..PressConfig::default()
        };
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-disabled")));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn press_result_current_attrs_pressed_and_disabled_sets_both() {
        let result = PressResult {
            state: Rc::new(RefCell::new(PressState::PressedInside {
                pointer_type: PointerType::Touch,
                origin_x: None,
                origin_y: None,
            })),
            pressed: true,
        };
        let config = PressConfig {
            disabled: true,
            ..PressConfig::default()
        };
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-pressed")));
        assert!(attrs.contains(&HtmlAttr::Data("ars-disabled")));
    }

    #[test]
    fn press_result_current_attrs_pressed_outside_no_data_ars_pressed() {
        let result = PressResult {
            state: Rc::new(RefCell::new(PressState::PressedOutside {
                pointer_type: PointerType::Mouse,
            })),
            pressed: false,
        };
        let config = PressConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-pressed")));
    }

    // --- use_press tests ---

    #[test]
    fn use_press_returns_idle_state() {
        let result = use_press(PressConfig::default());
        assert_eq!(*result.state.borrow(), PressState::Idle);
    }

    #[test]
    fn use_press_returns_pressed_false() {
        let result = use_press(PressConfig::default());
        assert!(!result.pressed);
    }

    #[test]
    fn use_press_disabled_config_still_creates_result() {
        let config = PressConfig {
            disabled: true,
            ..PressConfig::default()
        };
        let result = use_press(config);
        assert_eq!(*result.state.borrow(), PressState::Idle);
        assert!(!result.pressed);
    }
}
