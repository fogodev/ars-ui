//! Move interaction types and state machine.
//!
//! Move tracks continuous pointer or keyboard movement for controls such as
//! sliders, splitters, and drag handles. It normalizes pointer deltas, keyboard
//! arrow/home/end/page movement, and live movement attributes into a single
//! framework-agnostic interaction surface.

use alloc::{rc::Rc, vec::Vec};
use core::cell::RefCell;

use ars_core::{AttrMap, Callback, HtmlAttr, KeyModifiers, KeyboardKey, ResolvedDirection};

use crate::PointerType;

/// Configuration for move interaction behavior.
///
/// Callbacks use [`Callback`] so the interaction surface stays cloneable and
/// consistent with the rest of `ars-interactions`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MoveConfig {
    /// Whether the element is disabled.
    pub disabled: bool,

    /// Called when movement begins.
    pub on_move_start: Option<Callback<dyn Fn(MoveEvent) + Send + Sync>>,

    /// Called for each movement delta.
    pub on_move: Option<Callback<dyn Fn(MoveEvent) + Send + Sync>>,

    /// Called when movement ends or is cancelled.
    pub on_move_end: Option<Callback<dyn Fn(MoveEvent) + Send + Sync>>,
}

/// A normalized move event describing a positional delta.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveEvent {
    /// How the movement was initiated.
    pub pointer_type: PointerType,

    /// The kind of move event being dispatched.
    pub event_type: MoveEventType,

    /// Horizontal delta in CSS pixels or logical keyboard units.
    pub delta_x: f64,

    /// Vertical delta in CSS pixels or logical keyboard units.
    pub delta_y: f64,

    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,
}

/// The kind of move event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveEventType {
    /// Movement has started.
    MoveStart,

    /// A movement delta is being delivered.
    Move,

    /// Movement has ended.
    MoveEnd,
}

/// The current state of the move interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MoveState {
    /// No movement is active.
    #[default]
    Idle,

    /// Movement is active and tracking the last seen pointer position.
    Moving {
        /// Input modality that initiated the movement.
        pointer_type: PointerType,

        /// Last seen client X for delta computation.
        last_x: f64,

        /// Last seen client Y for delta computation.
        last_y: f64,

        /// Cached horizontal CSS scale factor for inverse delta correction.
        scale_x: f64,

        /// Cached vertical CSS scale factor for inverse delta correction.
        scale_y: f64,
    },
}

/// The output of [`use_move`], providing live attribute generation and
/// adapter-facing transition helpers.
#[derive(Debug)]
pub struct MoveResult {
    state: Rc<RefCell<MoveState>>,
    config: MoveConfig,
    active_move_keys: Vec<KeyboardKey>,
}

impl MoveResult {
    /// Returns the current attributes for the move target element.
    ///
    /// The `ars-touch-none` class is always present so touch-driven move
    /// interactions are not intercepted by browser gesture handling.
    #[must_use]
    pub fn current_attrs(&self) -> AttrMap {
        let moving = matches!(*self.state.borrow(), MoveState::Moving { .. });

        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Class, "ars-touch-none");

        if moving {
            attrs.set_bool(HtmlAttr::Data("ars-moving"), true);
        }

        attrs
    }

    /// Returns whether movement is currently active.
    #[must_use]
    pub fn is_moving(&self) -> bool {
        matches!(*self.state.borrow(), MoveState::Moving { .. })
    }

    /// Starts pointer-driven movement and records the pointer scale factors
    /// that should be used for subsequent delta correction.
    pub fn begin_pointer_move(
        &mut self,
        pointer_type: PointerType,
        client_x: f64,
        client_y: f64,
        modifiers: KeyModifiers,
        scale_x: f64,
        scale_y: f64,
    ) {
        if self.config.disabled {
            return;
        }

        let started = {
            let mut state = self.state.borrow_mut();

            match *state {
                MoveState::Idle => {
                    self.active_move_keys.clear();
                    *state = MoveState::Moving {
                        pointer_type,
                        last_x: client_x,
                        last_y: client_y,
                        scale_x: sanitize_scale(scale_x),
                        scale_y: sanitize_scale(scale_y),
                    };
                    true
                }

                MoveState::Moving { .. } => false,
            }
        };

        if started {
            self.dispatch_event(MoveEventType::MoveStart, pointer_type, 0.0, 0.0, modifiers);
        }
    }

    /// Applies a pointer-move delta using the scale captured at movement start.
    pub fn update_pointer_move(&mut self, client_x: f64, client_y: f64, modifiers: KeyModifiers) {
        let event = {
            let mut state = self.state.borrow_mut();

            match *state {
                MoveState::Idle
                | MoveState::Moving {
                    pointer_type: PointerType::Keyboard,
                    ..
                } => None,

                MoveState::Moving {
                    pointer_type,
                    ref mut last_x,
                    ref mut last_y,
                    scale_x,
                    scale_y,
                } => {
                    let raw_dx = client_x - *last_x;
                    let raw_dy = client_y - *last_y;

                    *last_x = client_x;
                    *last_y = client_y;

                    Some(MoveEvent {
                        pointer_type,
                        event_type: MoveEventType::Move,
                        delta_x: raw_dx / scale_x,
                        delta_y: raw_dy / scale_y,
                        modifiers,
                    })
                }
            }
        };

        if let Some(event) = event {
            self.dispatch_callback(self.config.on_move.as_ref(), event);
        }
    }

    /// Ends pointer-driven movement.
    pub fn end_pointer_move(&mut self, modifiers: KeyModifiers) {
        self.finish_pointer_move(modifiers);
    }

    /// Cancels pointer-driven movement.
    pub fn cancel_pointer_move(&mut self, modifiers: KeyModifiers) {
        self.finish_pointer_move(modifiers);
    }

    /// Handles keyboard-driven movement on keydown.
    ///
    /// Returns `true` when the key produced a movement delta.
    #[must_use]
    pub fn handle_key_down(
        &mut self,
        key: KeyboardKey,
        direction: ResolvedDirection,
        modifiers: KeyModifiers,
    ) -> bool {
        if self.config.disabled {
            return false;
        }

        let Some((delta_x, delta_y)) = key_to_delta(key, direction, modifiers) else {
            return false;
        };

        let should_emit_start = {
            let mut state = self.state.borrow_mut();

            match *state {
                MoveState::Idle => {
                    *state = MoveState::Moving {
                        pointer_type: PointerType::Keyboard,
                        last_x: 0.0,
                        last_y: 0.0,
                        scale_x: 1.0,
                        scale_y: 1.0,
                    };
                    true
                }

                MoveState::Moving {
                    pointer_type: PointerType::Keyboard,
                    ..
                } => false,

                MoveState::Moving { .. } => return false,
            }
        };

        self.track_active_move_key(key);

        if should_emit_start {
            self.dispatch_event(
                MoveEventType::MoveStart,
                PointerType::Keyboard,
                0.0,
                0.0,
                modifiers,
            );
        }

        self.dispatch_event(
            MoveEventType::Move,
            PointerType::Keyboard,
            delta_x,
            delta_y,
            modifiers,
        );

        true
    }

    /// Handles keyboard-driven movement completion on keyup.
    ///
    /// Returns `true` when the released key ended an active keyboard move
    /// session because no other move keys remain pressed.
    #[must_use]
    pub fn handle_key_up(&mut self, key: KeyboardKey, modifiers: KeyModifiers) -> bool {
        if !is_move_key(key) {
            return false;
        }

        if !self.untrack_active_move_key(key) {
            return false;
        }

        let ended = {
            let mut state = self.state.borrow_mut();

            match *state {
                MoveState::Moving {
                    pointer_type: PointerType::Keyboard,
                    ..
                } if self.active_move_keys.is_empty() => {
                    *state = MoveState::Idle;

                    true
                }

                MoveState::Idle | MoveState::Moving { .. } => false,
            }
        };

        if ended {
            self.dispatch_event(
                MoveEventType::MoveEnd,
                PointerType::Keyboard,
                0.0,
                0.0,
                modifiers,
            );
        }

        ended
    }

    fn finish_pointer_move(&mut self, modifiers: KeyModifiers) {
        let pointer_type = {
            let mut state = self.state.borrow_mut();

            match *state {
                MoveState::Moving {
                    pointer_type: PointerType::Keyboard,
                    ..
                }
                | MoveState::Idle => None,

                MoveState::Moving { pointer_type, .. } => {
                    *state = MoveState::Idle;

                    self.active_move_keys.clear();

                    Some(pointer_type)
                }
            }
        };

        if let Some(pointer_type) = pointer_type {
            self.dispatch_event(MoveEventType::MoveEnd, pointer_type, 0.0, 0.0, modifiers);
        }
    }

    fn dispatch_event(
        &self,
        event_type: MoveEventType,
        pointer_type: PointerType,
        delta_x: f64,
        delta_y: f64,
        modifiers: KeyModifiers,
    ) {
        let event = MoveEvent {
            pointer_type,
            event_type,
            delta_x,
            delta_y,
            modifiers,
        };

        match event_type {
            MoveEventType::MoveStart => {
                self.dispatch_callback(self.config.on_move_start.as_ref(), event);
            }

            MoveEventType::Move => self.dispatch_callback(self.config.on_move.as_ref(), event),

            MoveEventType::MoveEnd => {
                self.dispatch_callback(self.config.on_move_end.as_ref(), event);
            }
        }
    }

    fn dispatch_callback(
        &self,
        callback: Option<&Callback<dyn Fn(MoveEvent) + Send + Sync>>,
        event: MoveEvent,
    ) {
        if let Some(callback) = callback {
            callback(event);
        }
    }

    fn track_active_move_key(&mut self, key: KeyboardKey) {
        if !self.active_move_keys.contains(&key) {
            self.active_move_keys.push(key);
        }
    }

    fn untrack_active_move_key(&mut self, key: KeyboardKey) -> bool {
        let original_len = self.active_move_keys.len();

        self.active_move_keys
            .retain(|active_key| *active_key != key);

        self.active_move_keys.len() != original_len
    }
}

/// Creates a move interaction state machine with the given configuration.
#[must_use]
pub fn use_move(config: MoveConfig) -> MoveResult {
    MoveResult {
        state: Rc::new(RefCell::new(MoveState::Idle)),
        config,
        active_move_keys: Vec::new(),
    }
}

fn key_to_delta(
    key: KeyboardKey,
    direction: ResolvedDirection,
    modifiers: KeyModifiers,
) -> Option<(f64, f64)> {
    let step = if modifiers.shift { 10.0 } else { 1.0 };

    let h_step = if direction.is_rtl() { -step } else { step };

    match key {
        KeyboardKey::ArrowRight => Some((h_step, 0.0)),

        KeyboardKey::ArrowLeft => Some((-h_step, 0.0)),

        KeyboardKey::ArrowDown => Some((0.0, step)),

        KeyboardKey::ArrowUp => Some((0.0, -step)),

        KeyboardKey::Home => {
            let home = if direction.is_rtl() {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };

            Some((home, 0.0))
        }

        KeyboardKey::End => {
            let end = if direction.is_rtl() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };

            Some((end, 0.0))
        }

        KeyboardKey::PageUp => Some((0.0, -step * 10.0)),

        KeyboardKey::PageDown => Some((0.0, step * 10.0)),

        _ => None,
    }
}

const fn is_move_key(key: KeyboardKey) -> bool {
    matches!(
        key,
        KeyboardKey::ArrowRight
            | KeyboardKey::ArrowLeft
            | KeyboardKey::ArrowDown
            | KeyboardKey::ArrowUp
            | KeyboardKey::Home
            | KeyboardKey::End
            | KeyboardKey::PageUp
            | KeyboardKey::PageDown
    )
}

fn sanitize_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, sync::Arc, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{AttrValue, ResolvedDirection};

    use super::*;

    #[test]
    fn move_config_defaults_are_disabled_false_and_callbacks_none() {
        let config = MoveConfig::default();

        assert!(!config.disabled);
        assert!(config.on_move_start.is_none());
        assert!(config.on_move.is_none());
        assert!(config.on_move_end.is_none());
    }

    #[test]
    fn use_move_starts_idle_with_touch_none_class() {
        let result = use_move(MoveConfig::default());

        assert!(!result.is_moving());
        assert_eq!(*result.state.borrow(), MoveState::Idle);

        let attrs = result.current_attrs();

        assert_eq!(
            attrs.get_value(&HtmlAttr::Class),
            Some(&AttrValue::String(String::from("ars-touch-none"))),
        );
        assert!(!attrs.contains(&HtmlAttr::Data("ars-moving")));
    }

    #[test]
    fn pointer_move_transitions_idle_to_moving_to_idle() {
        let starts = Arc::new(Mutex::new(Vec::new()));
        let moves = Arc::new(Mutex::new(Vec::new()));
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Mouse,
            10.0,
            20.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );

        assert!(result.is_moving());
        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Mouse,
                last_x: 10.0,
                last_y: 20.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );

        let moving_attrs = result.current_attrs();

        assert!(moving_attrs.contains(&HtmlAttr::Data("ars-moving")));

        result.update_pointer_move(13.0, 27.0, KeyModifiers::default());
        result.end_pointer_move(KeyModifiers::default());

        assert!(!result.is_moving());
        assert_eq!(*result.state.borrow(), MoveState::Idle);

        let starts = starts.lock().expect("starts");

        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].event_type, MoveEventType::MoveStart);
        assert_eq!(starts[0].pointer_type, PointerType::Mouse);
        assert_eq!(starts[0].delta_x, 0.0);
        assert_eq!(starts[0].delta_y, 0.0);

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].event_type, MoveEventType::Move);
        assert_eq!(moves[0].delta_x, 3.0);
        assert_eq!(moves[0].delta_y, 7.0);

        let ends = ends.lock().expect("ends");

        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].event_type, MoveEventType::MoveEnd);
        assert_eq!(ends[0].pointer_type, PointerType::Mouse);
        assert_eq!(ends[0].delta_x, 0.0);
        assert_eq!(ends[0].delta_y, 0.0);
    }

    #[test]
    fn pointer_move_applies_inverse_css_scale_to_deltas() {
        let moves = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Touch,
            100.0,
            100.0,
            KeyModifiers::default(),
            2.0,
            4.0,
        );
        result.update_pointer_move(120.0, 140.0, KeyModifiers::default());

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].pointer_type, PointerType::Touch);
        assert_eq!(moves[0].delta_x, 10.0);
        assert_eq!(moves[0].delta_y, 10.0);
    }

    #[test]
    fn key_to_delta_maps_arrows_shift_home_end_and_page_keys() {
        assert_eq!(
            key_to_delta(
                KeyboardKey::ArrowRight,
                ResolvedDirection::Ltr,
                KeyModifiers::default(),
            ),
            Some((1.0, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::ArrowRight,
                ResolvedDirection::Rtl,
                KeyModifiers::default(),
            ),
            Some((-1.0, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::ArrowUp,
                ResolvedDirection::Ltr,
                KeyModifiers {
                    shift: true,
                    ..KeyModifiers::default()
                },
            ),
            Some((0.0, -10.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::Home,
                ResolvedDirection::Ltr,
                KeyModifiers::default(),
            ),
            Some((f64::NEG_INFINITY, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::End,
                ResolvedDirection::Rtl,
                KeyModifiers::default(),
            ),
            Some((f64::NEG_INFINITY, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::PageDown,
                ResolvedDirection::Ltr,
                KeyModifiers {
                    shift: true,
                    ..KeyModifiers::default()
                },
            ),
            Some((0.0, 100.0)),
        );
    }

    #[test]
    fn keyboard_move_starts_on_keydown_and_ends_on_keyup() {
        let starts = Arc::new(Mutex::new(Vec::new()));
        let moves = Arc::new(Mutex::new(Vec::new()));
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        assert!(result.handle_key_down(
            KeyboardKey::ArrowLeft,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Keyboard,
                last_x: 0.0,
                last_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
        assert!(result.handle_key_up(KeyboardKey::ArrowLeft, KeyModifiers::default()));
        assert_eq!(*result.state.borrow(), MoveState::Idle);

        let starts = starts.lock().expect("starts");

        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].event_type, MoveEventType::MoveStart);
        assert_eq!(starts[0].pointer_type, PointerType::Keyboard);

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].event_type, MoveEventType::Move);
        assert_eq!(moves[0].delta_x, -1.0);
        assert_eq!(moves[0].delta_y, 0.0);

        let ends = ends.lock().expect("ends");

        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].event_type, MoveEventType::MoveEnd);
        assert_eq!(ends[0].pointer_type, PointerType::Keyboard);
    }

    #[test]
    fn disabled_move_ignores_pointer_and_keyboard_transitions() {
        let starts = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            disabled: true,
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Mouse,
            1.0,
            2.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );

        assert_eq!(*result.state.borrow(), MoveState::Idle);
        assert!(!result.handle_key_down(
            KeyboardKey::ArrowRight,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(starts.lock().expect("starts").is_empty());
    }

    #[test]
    fn begin_pointer_move_is_noop_when_already_moving() {
        let starts = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Mouse,
            10.0,
            10.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );
        result.begin_pointer_move(
            PointerType::Pen,
            20.0,
            20.0,
            KeyModifiers::default(),
            3.0,
            3.0,
        );

        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Mouse,
                last_x: 10.0,
                last_y: 10.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
        assert_eq!(starts.lock().expect("starts").len(), 1);
    }

    #[test]
    fn update_pointer_move_is_noop_when_idle_or_keyboard_driven() {
        let moves = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.update_pointer_move(5.0, 6.0, KeyModifiers::default());

        assert!(moves.lock().expect("moves").is_empty());

        assert!(result.handle_key_down(
            KeyboardKey::ArrowDown,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));

        result.update_pointer_move(8.0, 9.0, KeyModifiers::default());

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].pointer_type, PointerType::Keyboard);
        assert_eq!(moves[0].delta_x, 0.0);
        assert_eq!(moves[0].delta_y, 1.0);
    }

    #[test]
    fn cancel_pointer_move_emits_end_and_resets_scale() {
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Pen,
            4.0,
            5.0,
            KeyModifiers::default(),
            2.0,
            3.0,
        );

        result.cancel_pointer_move(KeyModifiers {
            alt: true,
            ..KeyModifiers::default()
        });

        assert_eq!(*result.state.borrow(), MoveState::Idle);

        let ends = ends.lock().expect("ends");

        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].pointer_type, PointerType::Pen);
        assert!(ends[0].modifiers.alt);
    }

    #[test]
    fn cancel_and_key_up_are_noops_when_no_matching_active_move_exists() {
        let mut result = use_move(MoveConfig::default());

        result.cancel_pointer_move(KeyModifiers::default());

        assert_eq!(*result.state.borrow(), MoveState::Idle);

        assert!(!result.handle_key_up(KeyboardKey::Tab, KeyModifiers::default()));
        assert!(!result.handle_key_up(KeyboardKey::ArrowLeft, KeyModifiers::default()));

        result.begin_pointer_move(
            PointerType::Touch,
            1.0,
            1.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );

        assert!(!result.handle_key_up(KeyboardKey::ArrowLeft, KeyModifiers::default()));
        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Touch,
                last_x: 1.0,
                last_y: 1.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
    }

    #[test]
    fn handle_key_up_ignores_non_move_key_even_if_tracked() {
        let mut result = use_move(MoveConfig::default());

        *result.state.borrow_mut() = MoveState::Moving {
            pointer_type: PointerType::Keyboard,
            last_x: 0.0,
            last_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        result.active_move_keys.push(KeyboardKey::Tab);

        assert!(!result.handle_key_up(KeyboardKey::Tab, KeyModifiers::default()));
        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Keyboard,
                last_x: 0.0,
                last_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
    }

    #[test]
    fn handle_key_down_reuses_keyboard_session_and_rejects_non_move_or_pointer_conflicts() {
        let starts = Arc::new(Mutex::new(Vec::new()));
        let moves = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        assert!(!result.handle_key_down(
            KeyboardKey::Enter,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(result.handle_key_down(
            KeyboardKey::ArrowRight,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(result.handle_key_down(
            KeyboardKey::ArrowLeft,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));

        assert_eq!(starts.lock().expect("starts").len(), 1);

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].delta_x, 1.0);
        assert_eq!(moves[1].delta_x, -1.0);

        drop(moves);

        assert!(!result.handle_key_up(KeyboardKey::ArrowLeft, KeyModifiers::default()));
        assert!(result.handle_key_up(KeyboardKey::ArrowRight, KeyModifiers::default()));

        result.begin_pointer_move(
            PointerType::Mouse,
            5.0,
            5.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );

        assert!(!result.handle_key_down(
            KeyboardKey::ArrowUp,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
    }

    #[test]
    fn keyboard_move_only_ends_when_last_active_move_key_is_released() {
        let starts = Arc::new(Mutex::new(Vec::new()));
        let moves = Arc::new(Mutex::new(Vec::new()));
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_start: Some({
                let starts = Arc::clone(&starts);
                Callback::new(move |event: MoveEvent| {
                    starts.lock().expect("start events").push(event);
                })
            }),
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        assert!(result.handle_key_down(
            KeyboardKey::ArrowRight,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(result.handle_key_down(
            KeyboardKey::ArrowUp,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(!result.handle_key_up(KeyboardKey::ArrowRight, KeyModifiers::default()));
        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Keyboard,
                last_x: 0.0,
                last_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
        assert!(ends.lock().expect("ends").is_empty());

        assert!(result.handle_key_up(KeyboardKey::ArrowUp, KeyModifiers::default()));
        assert_eq!(*result.state.borrow(), MoveState::Idle);

        let starts = starts.lock().expect("starts");
        let moves = moves.lock().expect("moves");
        let ends = ends.lock().expect("ends");

        assert_eq!(starts.len(), 1);
        assert_eq!(moves.len(), 2);
        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].event_type, MoveEventType::MoveEnd);
    }

    #[test]
    fn rejected_pointer_start_does_not_forget_active_keyboard_move_keys() {
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        assert!(result.handle_key_down(
            KeyboardKey::ArrowRight,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(result.handle_key_down(
            KeyboardKey::ArrowUp,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));

        result.begin_pointer_move(
            PointerType::Mouse,
            10.0,
            10.0,
            KeyModifiers::default(),
            1.0,
            1.0,
        );

        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Keyboard,
                last_x: 0.0,
                last_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
        assert!(!result.handle_key_up(KeyboardKey::ArrowRight, KeyModifiers::default()));
        assert!(ends.lock().expect("ends").is_empty());
        assert!(result.handle_key_up(KeyboardKey::ArrowUp, KeyModifiers::default()));
        assert_eq!(*result.state.borrow(), MoveState::Idle);
        assert_eq!(ends.lock().expect("ends").len(), 1);
    }

    #[test]
    fn ignored_pointer_end_does_not_forget_active_keyboard_move_keys() {
        let ends = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move_end: Some({
                let ends = Arc::clone(&ends);
                Callback::new(move |event: MoveEvent| {
                    ends.lock().expect("end events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        assert!(result.handle_key_down(
            KeyboardKey::ArrowRight,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));
        assert!(result.handle_key_down(
            KeyboardKey::ArrowUp,
            ResolvedDirection::Ltr,
            KeyModifiers::default(),
        ));

        result.end_pointer_move(KeyModifiers::default());
        result.cancel_pointer_move(KeyModifiers::default());

        assert_eq!(
            *result.state.borrow(),
            MoveState::Moving {
                pointer_type: PointerType::Keyboard,
                last_x: 0.0,
                last_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            },
        );
        assert!(!result.handle_key_up(KeyboardKey::ArrowRight, KeyModifiers::default()));
        assert!(ends.lock().expect("ends").is_empty());
        assert!(result.handle_key_up(KeyboardKey::ArrowUp, KeyModifiers::default()));
        assert_eq!(*result.state.borrow(), MoveState::Idle);
        assert_eq!(ends.lock().expect("ends").len(), 1);
    }

    #[test]
    fn key_to_delta_covers_remaining_keys_and_non_move_input() {
        assert_eq!(
            key_to_delta(
                KeyboardKey::ArrowDown,
                ResolvedDirection::Ltr,
                KeyModifiers::default(),
            ),
            Some((0.0, 1.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::Home,
                ResolvedDirection::Rtl,
                KeyModifiers::default(),
            ),
            Some((f64::INFINITY, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::End,
                ResolvedDirection::Ltr,
                KeyModifiers::default(),
            ),
            Some((f64::INFINITY, 0.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::PageUp,
                ResolvedDirection::Ltr,
                KeyModifiers::default(),
            ),
            Some((0.0, -10.0)),
        );
        assert_eq!(
            key_to_delta(
                KeyboardKey::Tab,
                ResolvedDirection::Ltr,
                KeyModifiers::default()
            ),
            None,
        );
    }

    #[test]
    fn invalid_pointer_scale_falls_back_to_one() {
        let moves = Arc::new(Mutex::new(Vec::new()));

        let config = MoveConfig {
            on_move: Some({
                let moves = Arc::clone(&moves);
                Callback::new(move |event: MoveEvent| {
                    moves.lock().expect("move events").push(event);
                })
            }),
            ..MoveConfig::default()
        };

        let mut result = use_move(config);

        result.begin_pointer_move(
            PointerType::Mouse,
            10.0,
            10.0,
            KeyModifiers::default(),
            0.0,
            f64::NAN,
        );
        result.update_pointer_move(15.0, 18.0, KeyModifiers::default());

        let moves = moves.lock().expect("moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].delta_x, 5.0);
        assert_eq!(moves[0].delta_y, 8.0);
    }
}
