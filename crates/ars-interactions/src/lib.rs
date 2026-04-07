//! Input interaction state types and attribute merging utilities.
//!
//! This crate defines the shared interaction states (press, focus) used across
//! components and provides [`compose::merge_attrs`] for merging attribute maps
//! from multiple interaction sources into a single [`ars_core::AttrMap`].

pub mod compose;
pub mod direction;
pub mod hover;
pub mod press;

pub use ars_core::{
    Callback, DefaultModalityContext, KeyModifiers, KeyboardKey, ModalityContext, ModalitySnapshot,
    NullModalityContext, PointerType, SharedFlag, SharedState,
};
pub use compose::merge_attrs;
pub use direction::{LogicalDirection, resolve_arrow_key};
pub use hover::{HoverConfig, HoverEvent, HoverEventType, HoverResult, HoverState, use_hover};
pub use press::{PressConfig, PressEvent, PressEventType, PressResult, PressState, use_press};

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

    /// Returns whether the current focus state should render a visible indicator.
    #[must_use]
    pub fn is_focus_visible(&self, modality: &dyn ModalityContext) -> bool {
        match self {
            Self::FocusedByKeyboard => true,
            Self::FocusedProgrammatic => !modality.had_pointer_interaction(),
            Self::FocusedByPointer | Self::Unfocused => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn focus_state_programmatic_visibility_uses_injected_modality() {
        let modality = DefaultModalityContext::new();

        assert!(FocusState::FocusedProgrammatic.is_focus_visible(&modality));

        modality.on_pointer_down(PointerType::Mouse);
        assert!(!FocusState::FocusedProgrammatic.is_focus_visible(&modality));

        modality.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        assert!(FocusState::FocusedProgrammatic.is_focus_visible(&modality));
    }

    #[test]
    fn focus_state_keyboard_visibility_is_always_true() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Mouse);

        assert!(FocusState::FocusedByKeyboard.is_focus_visible(&modality));
        assert!(!FocusState::FocusedByPointer.is_focus_visible(&modality));
        assert!(!FocusState::Unfocused.is_focus_visible(&modality));
    }
}
