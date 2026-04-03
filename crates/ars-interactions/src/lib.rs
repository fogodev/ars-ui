//! Input interaction state types and attribute merging utilities.
//!
//! This crate defines the shared interaction states (press, focus) used across
//! components and provides a helper for merging attribute maps from multiple sources.

use ars_core::AttrMap;

/// The input modality that initiated an interaction.
///
/// Matches the values exposed by the Pointer Events API, extended with
/// virtual activation from screen readers and scripted events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerType {
    /// Physical mouse or trackpad.
    Mouse,
    /// Finger on a touchscreen.
    Touch,
    /// Stylus or digital pen.
    Pen,
    /// Keyboard (Enter, Space, or arrow key).
    Keyboard,
    /// Programmatic / screen reader virtual cursor activation.
    Virtual,
}

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
    pub fn is_pressed(&self) -> bool {
        matches!(
            self,
            PressState::PressedInside { .. } | PressState::PressedOutside { .. }
        )
    }

    /// Returns `true` when pressed and the pointer is within element bounds.
    #[must_use]
    pub fn is_pressed_inside(&self) -> bool {
        matches!(self, PressState::PressedInside { .. })
    }
}

/// The focus state of a focusable element.
///
/// Tracks both whether the element has focus and the modality that caused it,
/// enabling correct focus-ring visibility decisions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusState {
    /// Element does not have focus.
    #[default]
    Unfocused,

    /// Element has focus, received via pointer interaction.
    /// Focus ring should NOT be shown.
    FocusedByPointer,

    /// Element has focus, received via keyboard navigation.
    /// Focus ring SHOULD be shown.
    FocusedByKeyboard,

    /// Element has focus, received via programmatic `.focus()` call.
    /// Focus ring shown only if the document's previous modality was keyboard.
    FocusedProgrammatic,
}

impl FocusState {
    /// Returns `true` if the element currently has focus regardless of modality.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        !matches!(self, FocusState::Unfocused)
    }
}

/// Merges two attribute maps, with `overlay` values taking precedence over `base`.
///
/// Returns a new [`AttrMap`] containing all entries from both maps. When both maps
/// contain the same key, the value from `overlay` wins.
#[must_use]
pub fn merge_attrs(base: &AttrMap, overlay: &AttrMap) -> AttrMap {
    let mut merged = base.clone();
    for (key, value) in overlay {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_attrs_prefers_overlay_values() {
        let mut base = AttrMap::new();
        base.insert("role".into(), "button".into());
        let mut overlay = AttrMap::new();
        overlay.insert("role".into(), "switch".into());
        overlay.insert("data-state".into(), "on".into());

        let merged = merge_attrs(&base, &overlay);
        assert_eq!(merged.get("role").map(String::as_str), Some("switch"));
        assert_eq!(merged.get("data-state").map(String::as_str), Some("on"));
    }

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

    #[test]
    fn focus_state_unfocused_is_not_focused() {
        assert!(!FocusState::Unfocused.is_focused());
    }

    #[test]
    fn focus_state_keyboard_is_focused() {
        assert!(FocusState::FocusedByKeyboard.is_focused());
    }

    #[test]
    fn focus_state_all_focused_variants_report_focused() {
        assert!(FocusState::FocusedByPointer.is_focused());
        assert!(FocusState::FocusedByKeyboard.is_focused());
        assert!(FocusState::FocusedProgrammatic.is_focused());
    }
}
