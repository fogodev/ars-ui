//! Long-press interaction types and state machine.
//!
//! Long press tracks a held pointer or keyboard activation across a threshold
//! duration. It exposes current long-pressing data attributes, accessibility
//! description wiring, and the adapter-facing configuration needed to integrate
//! timing and cross-interaction cancellation with `Press`.

use alloc::{rc::Rc, string::String};
use core::{cell::RefCell, time::Duration};

use ars_core::{AttrMap, Callback, ComponentIds, HtmlAttr, MessageFn, SharedState, TimerHandle};
use ars_i18n::Locale;

use crate::{KeyModifiers, PointerType};

const MOVE_CANCEL_THRESHOLD_PX: f64 = 10.0;

/// The current state of the long-press state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LongPressState {
    /// No active long-press gesture is in progress.
    #[default]
    Idle,

    /// A press is being held while the long-press threshold timer is pending.
    Timing {
        /// The modality that started the hold.
        pointer_type: PointerType,

        /// The x-coordinate where the hold began, if available.
        origin_x: Option<f64>,

        /// The y-coordinate where the hold began, if available.
        origin_y: Option<f64>,

        /// Handle for the pending threshold timer.
        timer_handle: TimerHandle,
    },

    /// The threshold elapsed and the long press already fired.
    LongPressed {
        /// The modality that started the hold.
        pointer_type: PointerType,
    },
}

/// The kind of long-press event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongPressEventType {
    /// The hold began and the interaction entered the pending timing state.
    LongPressStart,

    /// The threshold elapsed while the hold was still active.
    LongPress,

    /// The long press was cancelled before reaching the threshold.
    LongPressCancel,
}

/// A normalized long-press event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LongPressEvent {
    /// How the interaction was initiated.
    pub pointer_type: PointerType,

    /// The type of long-press event.
    pub event_type: LongPressEventType,

    /// Client-space X coordinate. `None` for keyboard events.
    pub client_x: Option<f64>,

    /// Client-space Y coordinate. `None` for keyboard events.
    pub client_y: Option<f64>,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,
}

/// Configuration for long-press interaction behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct LongPressConfig {
    /// Whether the element is disabled.
    pub disabled: bool,

    /// Duration the pointer or key must be held before a long press is detected.
    /// Defaults to 500ms (matching iOS long-press behavior).
    pub threshold: Duration,

    /// Accessibility description of the long-press action.
    ///
    /// This text is rendered into a visually-hidden description element and
    /// linked to the interactive element via `aria-describedby`.
    pub accessibility_description: Option<String>,

    /// Localized live-announcement text emitted when the long press fires.
    ///
    /// The adapter should dispatch this message with assertive priority.
    pub long_press_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Called when the hold begins and the interaction enters `Timing`.
    pub on_long_press_start: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Called when the threshold elapses while the hold is still active.
    pub on_long_press: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Called when the interaction is cancelled before reaching the threshold.
    pub on_long_press_cancel: Option<Callback<dyn Fn(LongPressEvent)>>,

    /// Shared state used to suppress the co-located `Press` activation after a
    /// completed long press.
    ///
    /// The long-press threshold stores `Some(pointer_type)` for the modality
    /// that fired. The matching `Press` release consumes that value and
    /// suppresses only the originating activation.
    pub long_press_cancel_flag: Option<SharedState<Option<PointerType>>>,
}

impl Default for LongPressConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            threshold: Duration::from_millis(500),
            accessibility_description: None,
            long_press_announcement: MessageFn::static_str("Long press activated"),
            on_long_press_start: None,
            on_long_press: None,
            on_long_press_cancel: None,
            long_press_cancel_flag: None,
        }
    }
}

/// The output of [`use_long_press`], providing live attribute generation and state access.
#[derive(Debug)]
pub struct LongPressResult {
    /// Whether the element is currently timing or has already fired a long press.
    pub is_long_pressing: bool,

    state: Rc<RefCell<LongPressState>>,
    held_modifiers: Rc<RefCell<KeyModifiers>>,
    config: LongPressConfig,
}

impl LongPressResult {
    /// Produce a fresh [`AttrMap`] reflecting the current long-press state.
    ///
    /// Call this inside `connect()` so the returned attributes stay aligned with
    /// the latest interaction state.
    #[must_use]
    pub fn current_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        if is_long_pressing(*self.state.borrow()) {
            attrs.set_bool(HtmlAttr::Data("ars-long-pressing"), true);
        }

        attrs
    }

    /// Returns data attributes for an associated visually-hidden description element.
    ///
    /// The interactive element's `aria-describedby` linkage is applied by the
    /// component connect function, not by `current_attrs()`.
    #[must_use]
    pub fn description_attrs(&self, ids: &ComponentIds) -> Option<AttrMap> {
        self.config.accessibility_description.as_ref().map(|_desc| {
            let mut desc = AttrMap::new();

            desc.set(HtmlAttr::Id, ids.part("long-press-desc"));

            desc
        })
    }

    /// Returns the current long-press state snapshot.
    #[must_use]
    pub fn current_state(&self) -> LongPressState {
        *self.state.borrow()
    }

    /// Returns the pending timer handle while the interaction is in `Timing`.
    #[must_use]
    pub fn pending_timer_handle(&self) -> Option<TimerHandle> {
        match self.current_state() {
            LongPressState::Timing { timer_handle, .. } => Some(timer_handle),
            LongPressState::Idle | LongPressState::LongPressed { .. } => None,
        }
    }

    /// Begins a long-press interaction and records the pending threshold timer.
    pub fn begin_long_press(
        &mut self,
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
        timer_handle: TimerHandle,
    ) {
        let mut state = self.state.borrow_mut();

        let mut held_modifiers = self.held_modifiers.borrow_mut();

        reduce_long_press(
            &mut state,
            &mut held_modifiers,
            &self.config,
            InternalEvent::Start {
                pointer_type,
                client_x,
                client_y,
                modifiers,
                timer_handle,
            },
            None,
        );

        self.is_long_pressing = is_long_pressing(*state);
    }

    /// Fires the pending long-press threshold and returns the live announcement.
    ///
    /// Adapters should send the returned message through the provider's live
    /// announcer with assertive priority when `Some`.
    #[must_use]
    pub fn fire_long_press(&mut self, locale: &Locale) -> Option<String> {
        let mut state = self.state.borrow_mut();

        let mut held_modifiers = self.held_modifiers.borrow_mut();

        let announcement = reduce_long_press(
            &mut state,
            &mut held_modifiers,
            &self.config,
            InternalEvent::TimerFired,
            Some(locale),
        );

        self.is_long_pressing = is_long_pressing(*state);

        announcement
    }

    /// Ends the active long press, cancelling it if the threshold has not fired.
    pub fn end_long_press(&mut self, client_x: Option<f64>, client_y: Option<f64>) {
        let mut state = self.state.borrow_mut();

        let mut held_modifiers = self.held_modifiers.borrow_mut();

        reduce_long_press(
            &mut state,
            &mut held_modifiers,
            &self.config,
            InternalEvent::Release { client_x, client_y },
            None,
        );

        self.is_long_pressing = is_long_pressing(*state);
    }

    /// Cancels the active long press without firing the threshold action.
    pub fn cancel_long_press(&mut self, client_x: Option<f64>, client_y: Option<f64>) {
        let mut state = self.state.borrow_mut();

        let mut held_modifiers = self.held_modifiers.borrow_mut();

        reduce_long_press(
            &mut state,
            &mut held_modifiers,
            &self.config,
            InternalEvent::Cancel { client_x, client_y },
            None,
        );

        self.is_long_pressing = is_long_pressing(*state);
    }

    /// Updates the held pointer position, cancelling when movement exceeds the dead-zone.
    pub fn move_long_press(&mut self, client_x: f64, client_y: f64) {
        let mut state = self.state.borrow_mut();

        let mut held_modifiers = self.held_modifiers.borrow_mut();

        reduce_long_press(
            &mut state,
            &mut held_modifiers,
            &self.config,
            InternalEvent::Move { client_x, client_y },
            None,
        );

        self.is_long_pressing = is_long_pressing(*state);
    }
}

/// Creates a long-press interaction state container with the given configuration.
///
/// Event handlers and timer wiring are adapter-owned; the module-level helpers
/// below provide the state-machine logic used by unit tests and future adapter
/// integration.
#[must_use]
pub fn use_long_press(config: LongPressConfig) -> LongPressResult {
    LongPressResult {
        is_long_pressing: false,
        state: Rc::new(RefCell::new(LongPressState::Idle)),
        held_modifiers: Rc::new(RefCell::new(KeyModifiers::default())),
        config,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InternalEvent {
    Start {
        pointer_type: PointerType,
        client_x: Option<f64>,
        client_y: Option<f64>,
        modifiers: KeyModifiers,
        timer_handle: TimerHandle,
    },

    TimerFired,

    Release {
        client_x: Option<f64>,
        client_y: Option<f64>,
    },

    Cancel {
        client_x: Option<f64>,
        client_y: Option<f64>,
    },

    Move {
        client_x: f64,
        client_y: f64,
    },
}

fn reduce_long_press(
    state: &mut LongPressState,
    held_modifiers: &mut KeyModifiers,
    config: &LongPressConfig,
    event: InternalEvent,
    locale: Option<&Locale>,
) -> Option<String> {
    match event {
        InternalEvent::Start {
            pointer_type,
            client_x,
            client_y,
            modifiers,
            timer_handle,
        } => {
            if config.disabled || !matches!(state, LongPressState::Idle) {
                return None;
            }

            if let Some(flag) = &config.long_press_cancel_flag {
                flag.set(None);
            }

            *held_modifiers = modifiers;

            *state = LongPressState::Timing {
                pointer_type,
                origin_x: client_x,
                origin_y: client_y,
                timer_handle,
            };

            if let Some(callback) = &config.on_long_press_start {
                callback(LongPressEvent {
                    pointer_type,
                    event_type: LongPressEventType::LongPressStart,
                    client_x,
                    client_y,
                    modifiers,
                });
            }

            None
        }
        InternalEvent::TimerFired => {
            let LongPressState::Timing {
                pointer_type,
                origin_x,
                origin_y,
                ..
            } = *state
            else {
                return None;
            };

            *state = LongPressState::LongPressed { pointer_type };

            if let Some(flag) = &config.long_press_cancel_flag {
                flag.set(Some(pointer_type));
            }

            let event = LongPressEvent {
                pointer_type,
                event_type: LongPressEventType::LongPress,
                client_x: origin_x,
                client_y: origin_y,
                modifiers: *held_modifiers,
            };

            if let Some(callback) = &config.on_long_press {
                callback(event);
            }

            locale.map(|active_locale| (config.long_press_announcement)(active_locale))
        }
        InternalEvent::Release { client_x, client_y } => {
            let LongPressState::Timing {
                pointer_type,
                origin_x,
                origin_y,
                ..
            } = *state
            else {
                *state = LongPressState::Idle;
                *held_modifiers = KeyModifiers::default();

                return None;
            };

            *state = LongPressState::Idle;

            let event = LongPressEvent {
                pointer_type,
                event_type: LongPressEventType::LongPressCancel,
                client_x: client_x.or(origin_x),
                client_y: client_y.or(origin_y),
                modifiers: *held_modifiers,
            };

            *held_modifiers = KeyModifiers::default();

            if let Some(callback) = &config.on_long_press_cancel {
                callback(event);
            }

            None
        }
        InternalEvent::Cancel { client_x, client_y } => {
            let LongPressState::Timing {
                pointer_type,
                origin_x,
                origin_y,
                ..
            } = *state
            else {
                *state = LongPressState::Idle;

                *held_modifiers = KeyModifiers::default();

                return None;
            };

            *state = LongPressState::Idle;

            let event = LongPressEvent {
                pointer_type,
                event_type: LongPressEventType::LongPressCancel,
                client_x: client_x.or(origin_x),
                client_y: client_y.or(origin_y),
                modifiers: *held_modifiers,
            };

            *held_modifiers = KeyModifiers::default();

            if let Some(callback) = &config.on_long_press_cancel {
                callback(event);
            }

            None
        }
        InternalEvent::Move { client_x, client_y } => {
            let LongPressState::Timing {
                pointer_type,
                origin_x,
                origin_y,
                ..
            } = *state
            else {
                return None;
            };

            let origin_x = origin_x?;
            let origin_y = origin_y?;

            if exceeds_move_threshold(origin_x, origin_y, client_x, client_y) {
                *state = LongPressState::Idle;

                let event = LongPressEvent {
                    pointer_type,
                    event_type: LongPressEventType::LongPressCancel,
                    client_x: Some(client_x),
                    client_y: Some(client_y),
                    modifiers: *held_modifiers,
                };

                *held_modifiers = KeyModifiers::default();

                if let Some(callback) = &config.on_long_press_cancel {
                    callback(event);
                }
            }

            None
        }
    }
}

const fn is_long_pressing(state: LongPressState) -> bool {
    matches!(
        state,
        LongPressState::Timing { .. } | LongPressState::LongPressed { .. }
    )
}

fn exceeds_move_threshold(origin_x: f64, origin_y: f64, client_x: f64, client_y: f64) -> bool {
    let delta_x = client_x - origin_x;
    let delta_y = client_y - origin_y;

    (delta_x * delta_x) + (delta_y * delta_y) > MOVE_CANCEL_THRESHOLD_PX * MOVE_CANCEL_THRESHOLD_PX
}

#[cfg(test)]
mod tests {
    use alloc::{format, sync::Arc, vec, vec::Vec};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use ars_core::{AttrValue, HtmlAttr};
    use ars_i18n::locales;

    use super::*;

    #[test]
    fn long_press_config_default_values() {
        let config = LongPressConfig::default();

        assert!(!config.disabled);
        assert_eq!(config.threshold, Duration::from_millis(500));
        assert!(config.accessibility_description.is_none());
        assert_eq!(
            (config.long_press_announcement)(&locales::en_us()),
            "Long press activated"
        );
        assert!(config.on_long_press_start.is_none());
        assert!(config.on_long_press.is_none());
        assert!(config.on_long_press_cancel.is_none());
        assert!(config.long_press_cancel_flag.is_none());
    }

    #[test]
    fn long_press_config_debug_shows_callbacks_and_message() {
        let config = LongPressConfig {
            on_long_press_start: Some(Callback::new(|_: LongPressEvent| {})),
            on_long_press: Some(Callback::new(|_: LongPressEvent| {})),
            on_long_press_cancel: Some(Callback::new(|_: LongPressEvent| {})),
            ..LongPressConfig::default()
        };

        let debug = format!("{config:?}");

        assert!(debug.contains("on_long_press_start: Some(Callback(..))"));
        assert!(debug.contains("on_long_press: Some(Callback(..))"));
        assert!(debug.contains("on_long_press_cancel: Some(Callback(..))"));
        assert!(debug.contains("long_press_announcement: <closure>"));
    }

    #[test]
    fn long_press_event_type_variants_are_distinct() {
        assert_ne!(
            LongPressEventType::LongPressStart,
            LongPressEventType::LongPress
        );
        assert_ne!(
            LongPressEventType::LongPress,
            LongPressEventType::LongPressCancel
        );
    }

    #[test]
    fn long_press_result_current_attrs_idle_is_empty() {
        let result = LongPressResult {
            is_long_pressing: false,
            state: Rc::new(RefCell::new(LongPressState::Idle)),
            held_modifiers: Rc::new(RefCell::new(KeyModifiers::default())),
            config: LongPressConfig::default(),
        };

        let attrs = result.current_attrs();

        assert!(!attrs.contains(&HtmlAttr::Data("ars-long-pressing")));
    }

    #[test]
    fn long_press_result_current_attrs_sets_data_attr_for_timing_and_fired_states() {
        let config = LongPressConfig::default();

        let timing = LongPressResult {
            is_long_pressing: true,
            state: Rc::new(RefCell::new(LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(10.0),
                origin_y: Some(12.0),
                timer_handle: TimerHandle::new(1),
            })),
            held_modifiers: Rc::new(RefCell::new(KeyModifiers::default())),
            config: config.clone(),
        };

        let timing_attrs = timing.current_attrs();

        assert_eq!(
            timing_attrs.get_value(&HtmlAttr::Data("ars-long-pressing")),
            Some(&AttrValue::Bool(true))
        );

        let fired = LongPressResult {
            is_long_pressing: true,
            state: Rc::new(RefCell::new(LongPressState::LongPressed {
                pointer_type: PointerType::Keyboard,
            })),
            held_modifiers: Rc::new(RefCell::new(KeyModifiers::default())),
            config,
        };

        assert!(
            fired
                .current_attrs()
                .contains(&HtmlAttr::Data("ars-long-pressing"))
        );
    }

    #[test]
    fn long_press_description_attrs_returns_predictable_id() {
        let ids = ComponentIds::from_id("btn-1");

        let result = use_long_press(LongPressConfig {
            accessibility_description: Some(String::from("Long press for more options")),
            ..LongPressConfig::default()
        });

        let desc_attrs = result
            .description_attrs(&ids)
            .expect("description attrs should exist");

        assert_eq!(desc_attrs.get(&HtmlAttr::Id), Some("btn-1-long-press-desc"));
    }

    #[test]
    fn long_press_description_attrs_returns_none_without_description() {
        let ids = ComponentIds::from_id("btn-2");

        let result = use_long_press(LongPressConfig::default());

        assert!(result.description_attrs(&ids).is_none());
    }

    #[test]
    fn use_long_press_returns_idle_state_and_not_pressing() {
        let result = use_long_press(LongPressConfig::default());

        assert_eq!(*result.state.borrow(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
        assert_eq!(result.pending_timer_handle(), None);
    }

    #[test]
    fn begin_long_press_public_method_enters_timing_and_tracks_timer() {
        let mut result = use_long_press(LongPressConfig::default());

        result.begin_long_press(
            PointerType::Touch,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
            TimerHandle::new(21),
        );

        assert_eq!(
            result.current_state(),
            LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(3.0),
                origin_y: Some(4.0),
                timer_handle: TimerHandle::new(21),
            }
        );
        assert_eq!(result.pending_timer_handle(), Some(TimerHandle::new(21)));
        assert!(result.is_long_pressing);
    }

    #[test]
    fn begin_long_press_public_method_ignores_disabled_config() {
        let mut result = use_long_press(LongPressConfig {
            disabled: true,
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Touch,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
            TimerHandle::new(31),
        );

        assert_eq!(result.current_state(), LongPressState::Idle);
        assert_eq!(result.pending_timer_handle(), None);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn repeated_begin_long_press_is_ignored_while_active() {
        let start_calls = Arc::new(AtomicUsize::new(0));

        let mut result = use_long_press(LongPressConfig {
            on_long_press_start: Some({
                let start_calls = Arc::clone(&start_calls);
                Callback::new(move |_: LongPressEvent| {
                    start_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Touch,
            Some(3.0),
            Some(4.0),
            KeyModifiers::default(),
            TimerHandle::new(21),
        );

        result.begin_long_press(
            PointerType::Mouse,
            Some(9.0),
            Some(10.0),
            KeyModifiers::default(),
            TimerHandle::new(99),
        );

        assert_eq!(
            result.current_state(),
            LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(3.0),
                origin_y: Some(4.0),
                timer_handle: TimerHandle::new(21),
            }
        );
        assert_eq!(start_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn fire_long_press_public_method_returns_announcement_and_sets_cancel_flag() {
        let shared_flag = SharedState::new(None);

        let mut result = use_long_press(LongPressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers {
                shift: false,
                ctrl: true,
                alt: false,
                meta: false,
            },
            TimerHandle::new(22),
        );

        let announcement = result.fire_long_press(&locales::en_us());

        assert_eq!(announcement.as_deref(), Some("Long press activated"));
        assert_eq!(
            result.current_state(),
            LongPressState::LongPressed {
                pointer_type: PointerType::Keyboard,
            }
        );
        assert!(result.is_long_pressing);
        assert_eq!(shared_flag.get(), Some(PointerType::Keyboard));
        assert_eq!(result.pending_timer_handle(), None);
    }

    #[test]
    fn fire_long_press_public_method_from_idle_returns_none() {
        let mut result = use_long_press(LongPressConfig::default());

        let announcement = result.fire_long_press(&locales::en_us());

        assert!(announcement.is_none());
        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn end_long_press_public_method_returns_to_idle_after_fire() {
        let mut result = use_long_press(LongPressConfig::default());

        result.begin_long_press(
            PointerType::Mouse,
            Some(10.0),
            Some(12.0),
            KeyModifiers::default(),
            TimerHandle::new(23),
        );

        let _announcement = result.fire_long_press(&locales::en_us());

        result.end_long_press(Some(10.0), Some(12.0));

        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn end_long_press_public_method_before_threshold_fires_cancel() {
        let cancel_count = Arc::new(AtomicUsize::new(0));

        let mut result = use_long_press(LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_count = Arc::clone(&cancel_count);
                Callback::new(move |_: LongPressEvent| {
                    cancel_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Mouse,
            Some(10.0),
            Some(12.0),
            KeyModifiers::default(),
            TimerHandle::new(24),
        );

        result.end_long_press(Some(11.0), Some(13.0));

        assert_eq!(cancel_count.load(Ordering::SeqCst), 1);
        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn cancel_long_press_public_method_before_threshold_fires_cancel() {
        let cancel_count = Arc::new(AtomicUsize::new(0));

        let mut result = use_long_press(LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_count = Arc::clone(&cancel_count);
                Callback::new(move |_: LongPressEvent| {
                    cancel_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Pen,
            Some(1.0),
            Some(2.0),
            KeyModifiers::default(),
            TimerHandle::new(25),
        );

        result.cancel_long_press(Some(3.0), Some(4.0));

        assert_eq!(cancel_count.load(Ordering::SeqCst), 1);
        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn cancel_long_press_public_method_after_fire_is_noop() {
        let cancel_count = Arc::new(AtomicUsize::new(0));

        let mut result = use_long_press(LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_count = Arc::clone(&cancel_count);
                Callback::new(move |_: LongPressEvent| {
                    cancel_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Keyboard,
            None,
            None,
            KeyModifiers::default(),
            TimerHandle::new(26),
        );

        let _announcement = result.fire_long_press(&locales::en_us());

        result.cancel_long_press(None, None);

        assert_eq!(cancel_count.load(Ordering::SeqCst), 0);
        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn move_long_press_public_method_cancels_when_threshold_exceeded() {
        let cancel_count = Arc::new(AtomicUsize::new(0));

        let mut result = use_long_press(LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_count = Arc::clone(&cancel_count);
                Callback::new(move |_: LongPressEvent| {
                    cancel_count.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        });

        result.begin_long_press(
            PointerType::Touch,
            Some(100.0),
            Some(100.0),
            KeyModifiers::default(),
            TimerHandle::new(27),
        );

        result.move_long_press(120.0, 100.0);

        assert_eq!(cancel_count.load(Ordering::SeqCst), 1);
        assert_eq!(result.current_state(), LongPressState::Idle);
        assert!(!result.is_long_pressing);
    }

    #[test]
    fn move_long_press_public_method_within_threshold_keeps_active() {
        let mut result = use_long_press(LongPressConfig::default());

        result.begin_long_press(
            PointerType::Touch,
            Some(100.0),
            Some(100.0),
            KeyModifiers::default(),
            TimerHandle::new(28),
        );

        result.move_long_press(105.0, 103.0);

        assert_eq!(
            result.current_state(),
            LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(100.0),
                origin_y: Some(100.0),
                timer_handle: TimerHandle::new(28),
            }
        );
        assert!(result.is_long_pressing);
    }

    #[test]
    fn start_event_enters_timing_and_fires_start_callback() {
        let start_events = Arc::new(Mutex::new(Vec::<LongPressEvent>::new()));

        let config = LongPressConfig {
            on_long_press_start: Some({
                let start_events = Arc::clone(&start_events);
                Callback::new(move |event: LongPressEvent| {
                    start_events.lock().expect("poisoned").push(event);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Idle;

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Start {
                pointer_type: PointerType::Touch,
                client_x: Some(25.0),
                client_y: Some(40.0),
                modifiers: KeyModifiers {
                    shift: true,
                    ctrl: false,
                    alt: false,
                    meta: false,
                },
                timer_handle: TimerHandle::new(7),
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(
            state,
            LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(25.0),
                origin_y: Some(40.0),
                timer_handle: TimerHandle::new(7),
            }
        );
        assert_eq!(
            *start_events.lock().expect("poisoned"),
            vec![LongPressEvent {
                pointer_type: PointerType::Touch,
                event_type: LongPressEventType::LongPressStart,
                client_x: Some(25.0),
                client_y: Some(40.0),
                modifiers: KeyModifiers {
                    shift: true,
                    ctrl: false,
                    alt: false,
                    meta: false,
                },
            }]
        );
    }

    #[test]
    fn timer_fire_enters_long_pressed_sets_cancel_flag_and_returns_announcement() {
        let long_press_events = Arc::new(Mutex::new(Vec::<LongPressEvent>::new()));

        let shared_flag = SharedState::new(None);

        let config = LongPressConfig {
            long_press_cancel_flag: Some(shared_flag.clone()),
            on_long_press: Some({
                let long_press_events = Arc::clone(&long_press_events);
                Callback::new(move |event: LongPressEvent| {
                    long_press_events.lock().expect("poisoned").push(event);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Keyboard,
            origin_x: None,
            origin_y: None,
            timer_handle: TimerHandle::new(9),
        };

        let mut modifiers = KeyModifiers {
            shift: false,
            ctrl: true,
            alt: false,
            meta: false,
        };

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::TimerFired,
            Some(&locales::en_us()),
        );

        assert_eq!(
            state,
            LongPressState::LongPressed {
                pointer_type: PointerType::Keyboard,
            }
        );
        assert_eq!(shared_flag.get(), Some(PointerType::Keyboard));
        assert_eq!(announcement.as_deref(), Some("Long press activated"));
        assert_eq!(
            *long_press_events.lock().expect("poisoned"),
            vec![LongPressEvent {
                pointer_type: PointerType::Keyboard,
                event_type: LongPressEventType::LongPress,
                client_x: None,
                client_y: None,
                modifiers: KeyModifiers {
                    shift: false,
                    ctrl: true,
                    alt: false,
                    meta: false,
                },
            }]
        );
    }

    #[test]
    fn timer_fire_from_idle_is_ignored() {
        let config = LongPressConfig::default();

        let mut state = LongPressState::Idle;

        let mut modifiers = KeyModifiers {
            shift: true,
            ctrl: false,
            alt: true,
            meta: false,
        };

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::TimerFired,
            Some(&locales::en_us()),
        );

        assert!(announcement.is_none());
        assert_eq!(state, LongPressState::Idle);
        assert_eq!(
            modifiers,
            KeyModifiers {
                shift: true,
                ctrl: false,
                alt: true,
                meta: false,
            }
        );
    }

    #[test]
    fn release_before_threshold_cancels_and_resets_state() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Mouse,
            origin_x: Some(4.0),
            origin_y: Some(6.0),
            timer_handle: TimerHandle::new(3),
        };

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Release {
                client_x: Some(5.0),
                client_y: Some(7.0),
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(state, LongPressState::Idle);
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 1);
        assert_eq!(modifiers, KeyModifiers::default());
    }

    #[test]
    fn cancel_after_long_press_is_noop_and_resets_modifiers() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::LongPressed {
            pointer_type: PointerType::Pen,
        };

        let mut modifiers = KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
            meta: false,
        };

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Cancel {
                client_x: Some(4.0),
                client_y: Some(9.0),
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(state, LongPressState::Idle);
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 0);
        assert_eq!(modifiers, KeyModifiers::default());
    }

    #[test]
    fn cancel_event_before_threshold_resets_state_and_fires_cancel_callback() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Pen,
            origin_x: Some(3.0),
            origin_y: Some(8.0),
            timer_handle: TimerHandle::new(4),
        };

        let mut modifiers = KeyModifiers::default();

        reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Cancel {
                client_x: Some(4.0),
                client_y: Some(9.0),
            },
            None,
        );

        assert_eq!(state, LongPressState::Idle);
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 1);
        assert_eq!(modifiers, KeyModifiers::default());
    }

    #[test]
    fn move_within_threshold_keeps_timing_state() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Touch,
            origin_x: Some(100.0),
            origin_y: Some(100.0),
            timer_handle: TimerHandle::new(15),
        };

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Move {
                client_x: 105.0,
                client_y: 104.0,
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(
            state,
            LongPressState::Timing {
                pointer_type: PointerType::Touch,
                origin_x: Some(100.0),
                origin_y: Some(100.0),
                timer_handle: TimerHandle::new(15),
            }
        );
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn move_beyond_threshold_cancels_before_timer_fire() {
        let cancel_events = Arc::new(Mutex::new(Vec::<LongPressEvent>::new()));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_events = Arc::clone(&cancel_events);
                Callback::new(move |event: LongPressEvent| {
                    cancel_events.lock().expect("poisoned").push(event);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Touch,
            origin_x: Some(100.0),
            origin_y: Some(100.0),
            timer_handle: TimerHandle::new(5),
        };

        let mut modifiers = KeyModifiers::default();

        reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Move {
                client_x: 115.0,
                client_y: 100.0,
            },
            None,
        );

        assert_eq!(state, LongPressState::Idle);
        assert_eq!(
            *cancel_events.lock().expect("poisoned"),
            vec![LongPressEvent {
                pointer_type: PointerType::Touch,
                event_type: LongPressEventType::LongPressCancel,
                client_x: Some(115.0),
                client_y: Some(100.0),
                modifiers: KeyModifiers::default(),
            }]
        );
    }

    #[test]
    fn move_without_origin_coordinates_is_ignored() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Timing {
            pointer_type: PointerType::Keyboard,
            origin_x: None,
            origin_y: None,
            timer_handle: TimerHandle::new(16),
        };

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Move {
                client_x: 1.0,
                client_y: 2.0,
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(
            state,
            LongPressState::Timing {
                pointer_type: PointerType::Keyboard,
                origin_x: None,
                origin_y: None,
                timer_handle: TimerHandle::new(16),
            }
        );
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn move_from_idle_is_ignored() {
        let mut state = LongPressState::Idle;

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &LongPressConfig::default(),
            InternalEvent::Move {
                client_x: 1.0,
                client_y: 2.0,
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(state, LongPressState::Idle);
    }

    #[test]
    fn release_after_long_press_returns_to_idle_without_cancel_callback() {
        let cancel_calls = Arc::new(AtomicUsize::new(0));

        let config = LongPressConfig {
            on_long_press_cancel: Some({
                let cancel_calls = Arc::clone(&cancel_calls);
                Callback::new(move |_: LongPressEvent| {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                })
            }),
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::LongPressed {
            pointer_type: PointerType::Pen,
        };

        let mut modifiers = KeyModifiers::default();

        let announcement = reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Release {
                client_x: Some(1.0),
                client_y: Some(2.0),
            },
            None,
        );

        assert!(announcement.is_none());
        assert_eq!(state, LongPressState::Idle);
        assert_eq!(cancel_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn disabled_long_press_ignores_start_events() {
        let config = LongPressConfig {
            disabled: true,
            ..LongPressConfig::default()
        };

        let mut state = LongPressState::Idle;

        let mut modifiers = KeyModifiers::default();

        reduce_long_press(
            &mut state,
            &mut modifiers,
            &config,
            InternalEvent::Start {
                pointer_type: PointerType::Mouse,
                client_x: Some(0.0),
                client_y: Some(0.0),
                modifiers: KeyModifiers::default(),
                timer_handle: TimerHandle::new(11),
            },
            None,
        );

        assert_eq!(state, LongPressState::Idle);
    }
}
