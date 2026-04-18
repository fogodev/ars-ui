//! Press interaction types and state machine.
//!
//! The press interaction is the fundamental activation primitive that unifies
//! mouse click, touch tap, keyboard Enter/Space, and virtual cursor activation
//! into a single consistent model. It tracks whether the element is currently
//! being pressed and whether the pointer is within the element's bounds.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ars_core::{AttrMap, Callback, HtmlAttr, SharedFlag, SharedState};

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
    /// Returns `true` when the element is actively pressed within its bounds.
    ///
    /// The transient `Pressing` state (which resolves within the same event tick
    /// before any render) returns `false` to prevent `data-ars-pressed` from flashing.
    /// `PressedOutside` also returns `false`, matching the styling and activation
    /// semantics for presses that have left the element.
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        matches!(self, PressState::PressedInside { .. })
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActivePress {
    pointer_type: PointerType,
    origin_x: Option<f64>,
    origin_y: Option<f64>,
    is_within_element: bool,
}

// ---------------------------------------------------------------------------
// PressConfig
// ---------------------------------------------------------------------------

/// Configuration for press interaction behavior.
///
/// Controls how the press interaction responds to pointer, touch, and keyboard
/// input. Callbacks use [`Callback`] for shared Arc-backed ownership plus
/// built-in `Clone`, `Debug`, and `PartialEq` (by pointer identity).
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

    /// When set, the press handler checks this shared state on release.
    ///
    /// `Some(pointer_type)` suppresses the matching modality's activation
    /// because a long press already fired for that press. `None` means no
    /// pending long-press suppression. See spec §8.7 Cross-Interaction
    /// Cancellation Protocol.
    pub long_press_cancel_flag: Option<SharedState<Option<PointerType>>>,
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

    /// Per-modality active press bookkeeping used to support simultaneous inputs.
    active_presses: Rc<RefCell<Vec<ActivePress>>>,

    /// Stored press configuration used by the adapter-facing transition helpers.
    config: PressConfig,

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

    /// Returns the current press state snapshot.
    #[must_use]
    pub fn current_state(&self) -> PressState {
        self.state.borrow().clone()
    }

    /// Begins a press interaction and resolves the initial inside/outside state.
    ///
    /// Adapters call this from pointer-down, touch-start, keyboard-down, or
    /// virtual activation handlers after filtering unsupported events.
    pub fn begin_press(
        &mut self,
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
        within_element: bool,
    ) {
        if self.config.disabled {
            return;
        }

        let (next_state, pressed) = {
            let mut active_presses = self.active_presses.borrow_mut();

            if active_presses
                .iter()
                .any(|press| press.pointer_type == pointer_type)
            {
                return;
            }

            if active_presses.is_empty() {
                if let Some(flag) = &self.config.long_press_cancel_flag {
                    flag.set(None);
                }
            }

            *self.state.borrow_mut() = PressState::Pressing { pointer_type };

            active_presses.push(ActivePress {
                pointer_type,
                origin_x: client_x,
                origin_y: client_y,
                is_within_element: within_element,
            });

            (
                derive_press_state(&active_presses),
                active_presses.iter().any(|press| press.is_within_element),
            )
        };

        *self.state.borrow_mut() = next_state;

        self.pressed = pressed;

        if within_element {
            self.fire_press_event(
                PressEventType::PressStart,
                pointer_type,
                client_x,
                client_y,
                modifiers,
                true,
            );
        } else if let Some(callback) = &self.config.on_press_change {
            callback(false);
        }
    }

    /// Updates whether the active press is currently inside the element bounds.
    ///
    /// Adapters call this from pointer-enter, pointer-leave, or hit-tested move
    /// handlers while the press is active.
    pub fn update_pressed_bounds(
        &mut self,
        pointer_type: PointerType,
        within_element: bool,
        client_x: Option<f64>,
        client_y: Option<f64>,
    ) {
        let (changed, next_state, pressed) = {
            let mut active_presses = self.active_presses.borrow_mut();

            let Some(active_press) = active_presses
                .iter_mut()
                .find(|press| press.pointer_type == pointer_type)
            else {
                return;
            };

            let changed = active_press.is_within_element != within_element;

            active_press.is_within_element = within_element;

            if active_press.origin_x.is_none() {
                active_press.origin_x = client_x;
            }

            if active_press.origin_y.is_none() {
                active_press.origin_y = client_y;
            }

            (
                changed,
                derive_press_state(&active_presses),
                active_presses.iter().any(|press| press.is_within_element),
            )
        };

        *self.state.borrow_mut() = next_state;

        self.pressed = pressed;

        if changed {
            if let Some(callback) = &self.config.on_press_change {
                callback(within_element);
            }
        }
    }

    /// Ends the current press interaction.
    ///
    /// Returns `true` when activation fired (`on_press` ran) and `false`
    /// otherwise. Adapters can use the boolean to suppress duplicate native
    /// click synthesis after a completed long press.
    #[must_use]
    pub fn end_press(
        &mut self,
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
    ) -> bool {
        let (is_within_element, activation_candidate, suppress_activation, next_state, pressed) = {
            let mut active_presses = self.active_presses.borrow_mut();

            let Some(index) = active_presses
                .iter()
                .position(|press| press.pointer_type == pointer_type)
            else {
                return false;
            };

            let released_press = active_presses.remove(index);

            let is_within_element = released_press.is_within_element;

            let activation_candidate = is_within_element
                || (self.config.allow_press_on_exit && !released_press.is_within_element);

            let long_press_canceled = self.consume_long_press_cancel_flag(pointer_type);

            let suppress_activation = activation_candidate && long_press_canceled;

            let next_state = derive_press_state(&active_presses);

            let pressed = active_presses.iter().any(|press| press.is_within_element);

            (
                is_within_element,
                activation_candidate,
                suppress_activation,
                next_state,
                pressed,
            )
        };

        *self.state.borrow_mut() = next_state;

        self.pressed = pressed;

        self.fire_press_event(
            PressEventType::PressUp,
            pointer_type,
            client_x,
            client_y,
            modifiers,
            is_within_element,
        );

        self.fire_press_event(
            PressEventType::PressEnd,
            pointer_type,
            client_x,
            client_y,
            modifiers,
            is_within_element,
        );

        if activation_candidate && !suppress_activation {
            self.fire_press_event(
                PressEventType::Press,
                pointer_type,
                client_x,
                client_y,
                modifiers,
                is_within_element,
            );
            true
        } else {
            false
        }
    }

    /// Cancels the current press without firing activation.
    ///
    /// Adapters call this from pointer-cancel, blur, drag-start, or scroll
    /// cancellation paths.
    pub fn cancel_press(
        &mut self,
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
    ) {
        let (is_within_element, next_state, pressed) = {
            let mut active_presses = self.active_presses.borrow_mut();

            let Some(index) = active_presses
                .iter()
                .position(|press| press.pointer_type == pointer_type)
            else {
                return;
            };

            let cancelled_press = active_presses.remove(index);

            (
                cancelled_press.is_within_element,
                derive_press_state(&active_presses),
                active_presses.iter().any(|press| press.is_within_element),
            )
        };

        *self.state.borrow_mut() = next_state;

        self.pressed = pressed;

        self.clear_long_press_cancel_flag(pointer_type);

        self.fire_press_event(
            PressEventType::PressEnd,
            pointer_type,
            client_x,
            client_y,
            modifiers,
            is_within_element,
        );
    }

    fn consume_long_press_cancel_flag(&self, pointer_type: PointerType) -> bool {
        let Some(flag) = &self.config.long_press_cancel_flag else {
            return false;
        };

        let should_cancel = flag.get() == Some(pointer_type);

        if should_cancel {
            flag.set(None);
        }

        should_cancel
    }

    fn clear_long_press_cancel_flag(&self, pointer_type: PointerType) {
        let Some(flag) = &self.config.long_press_cancel_flag else {
            return;
        };

        if flag.get() == Some(pointer_type) {
            flag.set(None);
        }
    }

    fn fire_press_event(
        &self,
        event_type: PressEventType,
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
        is_within_element: bool,
    ) {
        let event = PressEvent {
            pointer_type,
            event_type,
            client_x,
            client_y,
            modifiers,
            is_within_element,
            continue_propagation: SharedFlag::new(false),
        };

        match event_type {
            PressEventType::PressStart => {
                if let Some(callback) = &self.config.on_press_start {
                    callback(event);
                }
            }

            PressEventType::PressEnd => {
                if let Some(callback) = &self.config.on_press_end {
                    callback(event);
                }
            }

            PressEventType::Press => {
                if let Some(callback) = &self.config.on_press {
                    callback(event);
                }
            }

            PressEventType::PressUp => {
                if let Some(callback) = &self.config.on_press_up {
                    callback(event);
                }
            }
        }
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
pub fn use_press(config: PressConfig) -> PressResult {
    let state = Rc::new(RefCell::new(PressState::Idle));

    let pressed = state.borrow().is_pressed_inside();

    PressResult {
        state,
        active_presses: Rc::default(),
        config,
        pressed,
    }
}

fn derive_press_state(active_presses: &[ActivePress]) -> PressState {
    if let Some(active_press) = active_presses
        .iter()
        .rev()
        .find(|press| press.is_within_element)
    {
        return PressState::PressedInside {
            pointer_type: active_press.pointer_type,
            origin_x: active_press.origin_x,
            origin_y: active_press.origin_y,
        };
    }

    if let Some(active_press) = active_presses.last() {
        return PressState::PressedOutside {
            pointer_type: active_press.pointer_type,
        };
    }

    PressState::Idle
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
    fn press_state_pressed_outside_is_not_pressed() {
        let state = PressState::PressedOutside {
            pointer_type: PointerType::Touch,
        };

        assert!(!state.is_pressed());
        assert!(!state.is_pressed_inside());
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
            long_press_cancel_flag: Some(SharedState::new(Some(PointerType::Touch))),
            ..PressConfig::default()
        };

        let debug = format!("{config:?}");

        assert!(debug.contains("on_press: Some(Callback(..))"));
        assert!(debug.contains("on_press_change: Some(Callback(..))"));
        assert!(debug.contains("long_press_cancel_flag: Some(SharedState(Some(Touch)))"));

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
            active_presses: Rc::new(RefCell::new(Vec::new())),
            config: PressConfig::default(),
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
            active_presses: Rc::new(RefCell::new(vec![ActivePress {
                pointer_type: PointerType::Mouse,
                origin_x: Some(10.0),
                origin_y: Some(20.0),
                is_within_element: true,
            }])),
            config: PressConfig::default(),
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
            active_presses: Rc::new(RefCell::new(Vec::new())),
            config: PressConfig::default(),
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
            active_presses: Rc::new(RefCell::new(vec![ActivePress {
                pointer_type: PointerType::Touch,
                origin_x: None,
                origin_y: None,
                is_within_element: true,
            }])),
            config: PressConfig::default(),
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
            active_presses: Rc::new(RefCell::new(vec![ActivePress {
                pointer_type: PointerType::Mouse,
                origin_x: Some(0.0),
                origin_y: Some(0.0),
                is_within_element: false,
            }])),
            config: PressConfig::default(),
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

    #[test]
    fn begin_press_enters_pressed_inside_and_fires_start_callback() {
        let start_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let config = PressConfig {
            on_press_start: Some({
                let start_events = Arc::clone(&start_events);
                Callback::new(move |event: PressEvent| {
                    start_events.lock().expect("poisoned").push(event);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(12.0),
            Some(24.0),
            KeyModifiers {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            },
            true,
        );

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Mouse,
                origin_x: Some(12.0),
                origin_y: Some(24.0),
            }
        );
        assert!(result.pressed);
        assert_eq!(start_events.lock().expect("poisoned").len(), 1);

        let start_events = start_events.lock().expect("poisoned");

        let event = &start_events[0];

        assert_eq!(event.pointer_type, PointerType::Mouse);
        assert_eq!(event.event_type, PressEventType::PressStart);
        assert_eq!(event.client_x, Some(12.0));
        assert_eq!(event.client_y, Some(24.0));
        assert_eq!(
            event.modifiers,
            KeyModifiers {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            }
        );
        assert!(event.is_within_element);
        assert!(!event.should_propagate());
    }

    #[test]
    fn begin_press_disabled_is_ignored() {
        let start_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            disabled: true,
            on_press_start: Some({
                let start_count = Arc::clone(&start_count);

                Callback::new(move |_: PressEvent| {
                    start_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(7.0),
            Some(8.0),
            KeyModifiers::default(),
            true,
        );

        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
        assert_eq!(start_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn duplicate_begin_press_for_same_modality_is_ignored() {
        let start_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            on_press_start: Some({
                let start_count = Arc::clone(&start_count);

                Callback::new(move |_: PressEvent| {
                    start_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(1.0),
            Some(2.0),
            KeyModifiers::default(),
            true,
        );

        result.begin_press(
            PointerType::Touch,
            Some(9.0),
            Some(10.0),
            KeyModifiers::default(),
            false,
        );

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Touch,
                origin_x: Some(1.0),
                origin_y: Some(2.0),
            }
        );
        assert!(result.pressed);
        assert_eq!(start_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn update_pressed_bounds_emits_change_events_for_leave_and_reenter() {
        let changes = Arc::new(std::sync::Mutex::new(Vec::<bool>::new()));

        let config = PressConfig {
            on_press_change: Some({
                let changes = Arc::clone(&changes);

                Callback::new(move |inside: bool| {
                    changes.lock().expect("poisoned").push(inside);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(1.0),
            Some(2.0),
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Touch, false, Some(10.0), Some(11.0));

        assert_eq!(
            result.current_state(),
            PressState::PressedOutside {
                pointer_type: PointerType::Touch,
            }
        );
        assert!(!result.pressed);

        result.update_pressed_bounds(PointerType::Touch, true, Some(12.0), Some(13.0));

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Touch,
                origin_x: Some(1.0),
                origin_y: Some(2.0),
            }
        );
        assert!(result.pressed);
        assert_eq!(*changes.lock().expect("poisoned"), vec![false, true]);
    }

    #[test]
    fn update_pressed_bounds_with_unknown_modality_is_noop() {
        let changes = Arc::new(std::sync::Mutex::new(Vec::<bool>::new()));

        let config = PressConfig {
            on_press_change: Some({
                let changes = Arc::clone(&changes);

                Callback::new(move |inside: bool| {
                    changes.lock().expect("poisoned").push(inside);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(2.0),
            Some(3.0),
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Keyboard, false, None, None);

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Mouse,
                origin_x: Some(2.0),
                origin_y: Some(3.0),
            }
        );
        assert!(result.pressed);
        assert!(changes.lock().expect("poisoned").is_empty());
    }

    #[test]
    fn update_pressed_bounds_without_change_does_not_emit_callback() {
        let changes = Arc::new(std::sync::Mutex::new(Vec::<bool>::new()));

        let config = PressConfig {
            on_press_change: Some({
                let changes = Arc::clone(&changes);

                Callback::new(move |inside: bool| {
                    changes.lock().expect("poisoned").push(inside);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Pen,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Pen, true, Some(8.0), Some(9.0));

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Pen,
                origin_x: Some(3.0),
                origin_y: Some(4.0),
            }
        );
        assert!(changes.lock().expect("poisoned").is_empty());
    }

    #[test]
    fn update_pressed_bounds_backfills_origin_when_begin_had_no_coordinates() {
        let config = PressConfig::default();

        let mut result = use_press(config);

        // Begin press with no coordinates (keyboard-initiated press).
        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Keyboard,
                origin_x: None,
                origin_y: None,
            }
        );

        // Now update_pressed_bounds with coordinates — this backfills the origin.
        result.update_pressed_bounds(PointerType::Keyboard, true, Some(42.0), Some(84.0));

        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Keyboard,
                origin_x: Some(42.0),
                origin_y: Some(84.0),
            }
        );
    }

    #[test]
    fn begin_press_outside_emits_initial_press_change_false() {
        let changes = Arc::new(std::sync::Mutex::new(Vec::<bool>::new()));

        let config = PressConfig {
            on_press_change: Some({
                let changes = Arc::clone(&changes);

                Callback::new(move |inside: bool| {
                    changes.lock().expect("poisoned").push(inside);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(9.0),
            Some(10.0),
            KeyModifiers::default(),
            false,
        );

        assert_eq!(
            result.current_state(),
            PressState::PressedOutside {
                pointer_type: PointerType::Mouse,
            }
        );
        assert!(!result.pressed);
        assert_eq!(*changes.lock().expect("poisoned"), vec![false]);
    }

    #[test]
    fn begin_press_from_idle_clears_stale_long_press_flag() {
        let shared_flag = SharedState::new(Some(PointerType::Mouse));

        let mut result = use_press(PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            ..PressConfig::default()
        });

        result.begin_press(
            PointerType::Mouse,
            Some(1.0),
            Some(2.0),
            KeyModifiers::default(),
            true,
        );

        assert_eq!(shared_flag.get(), None);
        assert!(result.pressed);
    }

    #[test]
    fn end_press_fires_activation_when_inside() {
        let end_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let press_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let up_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let config = PressConfig {
            on_press_end: Some({
                let end_events = Arc::clone(&end_events);

                Callback::new(move |event: PressEvent| {
                    end_events.lock().expect("poisoned").push(event);
                })
            }),
            on_press: Some({
                let press_events = Arc::clone(&press_events);
                Callback::new(move |event: PressEvent| {
                    press_events.lock().expect("poisoned").push(event);
                })
            }),
            on_press_up: Some({
                let up_events = Arc::clone(&up_events);
                Callback::new(move |event: PressEvent| {
                    up_events.lock().expect("poisoned").push(event);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        let activated =
            result.end_press(PointerType::Keyboard, None, None, KeyModifiers::default());

        assert!(activated);
        assert_eq!(result.current_state(), PressState::Idle);
        assert_eq!(up_events.lock().expect("poisoned").len(), 1);
        assert_eq!(end_events.lock().expect("poisoned").len(), 1);
        assert_eq!(press_events.lock().expect("poisoned").len(), 1);
        assert_eq!(
            press_events.lock().expect("poisoned")[0].event_type,
            PressEventType::Press
        );
    }

    #[test]
    fn end_press_outside_without_allow_press_on_exit_does_not_activate() {
        let up_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let end_events = Arc::new(std::sync::Mutex::new(Vec::<PressEvent>::new()));

        let press_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            on_press_up: Some({
                let up_events = Arc::clone(&up_events);

                Callback::new(move |event: PressEvent| {
                    up_events.lock().expect("poisoned").push(event);
                })
            }),
            on_press_end: Some({
                let end_events = Arc::clone(&end_events);
                Callback::new(move |event: PressEvent| {
                    end_events.lock().expect("poisoned").push(event);
                })
            }),
            on_press: Some({
                let press_count = Arc::clone(&press_count);
                Callback::new(move |_: PressEvent| {
                    press_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(5.0),
            Some(6.0),
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Mouse, false, Some(20.0), Some(21.0));

        let activated = result.end_press(
            PointerType::Mouse,
            Some(20.0),
            Some(21.0),
            KeyModifiers::default(),
        );

        assert!(!activated);
        assert_eq!(press_count.load(Ordering::SeqCst), 0);
        assert_eq!(up_events.lock().expect("poisoned").len(), 1);
        assert_eq!(end_events.lock().expect("poisoned").len(), 1);
        assert!(!up_events.lock().expect("poisoned")[0].is_within_element);
        assert!(!end_events.lock().expect("poisoned")[0].is_within_element);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn end_press_outside_activates_when_allow_press_on_exit_is_enabled() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            allow_press_on_exit: true,
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);
                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(5.0),
            Some(6.0),
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Mouse, false, Some(20.0), Some(21.0));

        let activated = result.end_press(
            PointerType::Mouse,
            Some(20.0),
            Some(21.0),
            KeyModifiers::default(),
        );

        assert!(activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 1);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn end_press_suppresses_activation_when_long_press_flag_is_set() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let shared_flag = SharedState::new(Some(PointerType::Touch));

        let config = PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.active_presses.borrow_mut().push(ActivePress {
            pointer_type: PointerType::Touch,
            origin_x: Some(8.0),
            origin_y: Some(9.0),
            is_within_element: true,
        });

        *result.state.borrow_mut() = derive_press_state(&result.active_presses.borrow());

        result.pressed = true;

        let activated = result.end_press(
            PointerType::Touch,
            Some(8.0),
            Some(9.0),
            KeyModifiers::default(),
        );

        assert!(!activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 0);
        assert_eq!(shared_flag.get(), None);
        assert_eq!(result.current_state(), PressState::Idle);
    }

    #[test]
    fn end_press_with_unknown_modality_returns_false() {
        let mut result = use_press(PressConfig::default());

        result.begin_press(
            PointerType::Touch,
            Some(8.0),
            Some(9.0),
            KeyModifiers::default(),
            true,
        );

        let activated =
            result.end_press(PointerType::Keyboard, None, None, KeyModifiers::default());

        assert!(!activated);
        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Touch,
                origin_x: Some(8.0),
                origin_y: Some(9.0),
            }
        );
        assert!(result.pressed);
    }

    #[test]
    fn cancel_press_resets_state_and_fires_end_without_activation() {
        let end_count = Arc::new(AtomicUsize::new(0));

        let press_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            on_press_end: Some({
                let end_count = Arc::clone(&end_count);

                Callback::new(move |_: PressEvent| {
                    end_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            on_press: Some({
                let press_count = Arc::clone(&press_count);

                Callback::new(move |_: PressEvent| {
                    press_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Pen,
            Some(2.0),
            Some(3.0),
            KeyModifiers::default(),
            true,
        );

        result.cancel_press(
            PointerType::Pen,
            Some(2.0),
            Some(3.0),
            KeyModifiers::default(),
        );

        assert_eq!(result.current_state(), PressState::Idle);
        assert_eq!(end_count.load(Ordering::SeqCst), 1);
        assert_eq!(press_count.load(Ordering::SeqCst), 0);
        assert!(!result.pressed);
    }

    #[test]
    fn cancel_press_with_unknown_modality_is_noop() {
        let end_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            on_press_end: Some({
                let end_count = Arc::clone(&end_count);

                Callback::new(move |_: PressEvent| {
                    end_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Pen,
            Some(2.0),
            Some(3.0),
            KeyModifiers::default(),
            true,
        );

        result.cancel_press(
            PointerType::Mouse,
            Some(9.0),
            Some(10.0),
            KeyModifiers::default(),
        );

        assert_eq!(end_count.load(Ordering::SeqCst), 0);
        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Pen,
                origin_x: Some(2.0),
                origin_y: Some(3.0),
            }
        );
        assert!(result.pressed);
    }

    #[test]
    fn simultaneous_pointer_and_keyboard_presses_are_tracked_independently() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let config = PressConfig {
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(5.0),
            Some(6.0),
            KeyModifiers::default(),
            true,
        );

        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        let mouse_activated = result.end_press(
            PointerType::Mouse,
            Some(5.0),
            Some(6.0),
            KeyModifiers::default(),
        );

        assert!(mouse_activated);
        assert!(result.pressed);
        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Keyboard,
                origin_x: None,
                origin_y: None,
            }
        );

        let keyboard_activated =
            result.end_press(PointerType::Keyboard, None, None, KeyModifiers::default());

        assert!(keyboard_activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 2);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn second_modality_begin_does_not_clear_pending_long_press_suppression() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let shared_flag = SharedState::new(None);

        let config = PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(4.0),
            Some(5.0),
            KeyModifiers::default(),
            true,
        );

        shared_flag.set(Some(PointerType::Touch));

        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        let activated = result.end_press(
            PointerType::Touch,
            Some(4.0),
            Some(5.0),
            KeyModifiers::default(),
        );

        assert!(!activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 0);
        assert_eq!(shared_flag.get(), None);
        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Keyboard,
                origin_x: None,
                origin_y: None,
            }
        );
        assert!(result.pressed);
    }

    #[test]
    fn outside_release_consumes_long_press_suppression_before_other_modality_activates() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let shared_flag = SharedState::new(None);

        let config = PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(4.0),
            Some(5.0),
            KeyModifiers::default(),
            true,
        );

        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Touch, false, Some(40.0), Some(50.0));

        shared_flag.set(Some(PointerType::Touch));

        let touch_activated = result.end_press(
            PointerType::Touch,
            Some(40.0),
            Some(50.0),
            KeyModifiers::default(),
        );

        let keyboard_activated =
            result.end_press(PointerType::Keyboard, None, None, KeyModifiers::default());

        assert!(!touch_activated);
        assert!(keyboard_activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 1);
        assert_eq!(shared_flag.get(), None);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn long_press_suppression_is_consumed_only_by_matching_modality_release() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let shared_flag = SharedState::new(None);

        let config = PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(4.0),
            Some(5.0),
            KeyModifiers::default(),
            true,
        );

        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        shared_flag.set(Some(PointerType::Touch));

        let keyboard_activated =
            result.end_press(PointerType::Keyboard, None, None, KeyModifiers::default());

        let touch_activated = result.end_press(
            PointerType::Touch,
            Some(4.0),
            Some(5.0),
            KeyModifiers::default(),
        );

        assert!(keyboard_activated);
        assert!(!touch_activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 1);
        assert_eq!(shared_flag.get(), None);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn cancel_press_clears_matching_long_press_suppression() {
        let activation_count = Arc::new(AtomicUsize::new(0));

        let shared_flag = SharedState::new(None);

        let config = PressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_press: Some({
                let activation_count = Arc::clone(&activation_count);

                Callback::new(move |_: PressEvent| {
                    activation_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Touch,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
            true,
        );

        shared_flag.set(Some(PointerType::Touch));

        result.cancel_press(
            PointerType::Touch,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
        );

        assert_eq!(shared_flag.get(), None);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);

        result.begin_press(
            PointerType::Touch,
            Some(7.0),
            Some(8.0),
            KeyModifiers::default(),
            true,
        );

        let activated = result.end_press(
            PointerType::Touch,
            Some(7.0),
            Some(8.0),
            KeyModifiers::default(),
        );

        assert!(activated);
        assert_eq!(activation_count.load(Ordering::SeqCst), 1);
        assert_eq!(shared_flag.get(), None);
        assert_eq!(result.current_state(), PressState::Idle);
        assert!(!result.pressed);
    }

    #[test]
    fn pointer_exit_does_not_clear_pressed_while_keyboard_press_remains_active() {
        let changes = Arc::new(std::sync::Mutex::new(Vec::<bool>::new()));

        let config = PressConfig {
            on_press_change: Some({
                let changes = Arc::clone(&changes);

                Callback::new(move |inside: bool| {
                    changes.lock().expect("poisoned").push(inside);
                })
            }),
            ..PressConfig::default()
        };

        let mut result = use_press(config);

        result.begin_press(
            PointerType::Mouse,
            Some(1.0),
            Some(1.0),
            KeyModifiers::default(),
            true,
        );
        result.begin_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            true,
        );

        result.update_pressed_bounds(PointerType::Mouse, false, Some(8.0), Some(9.0));

        assert!(result.pressed);
        assert_eq!(
            result.current_state(),
            PressState::PressedInside {
                pointer_type: PointerType::Keyboard,
                origin_x: None,
                origin_y: None,
            }
        );
        assert_eq!(*changes.lock().expect("poisoned"), vec![false]);
    }
}
