//! Focus and focus-within interaction types and state machine.
//!
//! Focus interaction provides normalized focus/blur events and determines
//! whether focus is "visible" — i.e., whether a focus ring should be displayed.
//! A focus ring appears for keyboard navigation but is unnecessary and visually
//! noisy for pointer interactions.
//!
//! [`FocusWithin`](FocusWithinResult) extends focus tracking to container
//! elements: the container is marked as focus-containing when any descendant has
//! focus, matching CSS `:focus-within` but exposed as a data attribute.

use std::{cell::RefCell, rc::Rc};

use ars_core::{ArsRc, AttrMap, Callback, HtmlAttr, ModalityContext};

use crate::PointerType;

// ────────────────────────────────────────────────────────────────────
// FocusState
// ────────────────────────────────────────────────────────────────────

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
    pub const fn is_focused(&self) -> bool {
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

// ────────────────────────────────────────────────────────────────────
// FocusEventType + FocusEvent
// ────────────────────────────────────────────────────────────────────

/// The kind of focus event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEventType {
    /// The element received focus.
    Focus,
    /// The element lost focus.
    Blur,
    /// A descendant within the container received focus.
    FocusWithin,
    /// Focus left the container entirely.
    BlurWithin,
}

/// A normalized focus event, independent of input modality.
#[derive(Clone, Debug)]
pub struct FocusEvent {
    /// The type of focus event.
    pub event_type: FocusEventType,

    /// The pointer type that triggered this focus, or `None` if focus was moved
    /// programmatically (e.g., via `element.focus()`).
    pub pointer_type: Option<PointerType>,
}

// ────────────────────────────────────────────────────────────────────
// FocusConfig
// ────────────────────────────────────────────────────────────────────

/// Configuration for focus interaction on a single element.
///
/// Callbacks use [`Callback`] for automatic platform-appropriate pointer
/// type (`Rc` on wasm, `Arc` on native) and built-in `Clone`, `Debug`, and
/// `PartialEq` (by pointer identity).
#[derive(Clone, Debug, PartialEq)]
pub struct FocusConfig {
    /// Whether the element is disabled. Disabled elements receive no focus events.
    pub disabled: bool,

    /// Shared modality context for the current provider root.
    pub modality: ArsRc<dyn ModalityContext>,

    /// Called when the element receives focus.
    pub on_focus: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when the element loses focus.
    pub on_blur: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus-visible state changes.
    pub on_focus_visible_change: Option<Callback<dyn Fn(bool)>>,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            modality: ArsRc::from_modality(ars_core::DefaultModalityContext::new()),
            on_focus: None,
            on_blur: None,
            on_focus_visible_change: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// FocusWithinConfig
// ────────────────────────────────────────────────────────────────────

/// Configuration for focus-within tracking on a container element.
///
/// Callbacks use [`Callback`] for automatic platform-appropriate pointer
/// type (`Rc` on wasm, `Arc` on native) and built-in `Clone`, `Debug`, and
/// `PartialEq` (by pointer identity).
#[derive(Clone, Debug, PartialEq)]
pub struct FocusWithinConfig {
    /// Whether the container is disabled.
    pub disabled: bool,

    /// Shared modality context for the current provider root.
    pub modality: ArsRc<dyn ModalityContext>,

    /// Called when focus enters the container (any descendant focused).
    pub on_focus_within: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus leaves the container entirely.
    pub on_blur_within: Option<Callback<dyn Fn(FocusEvent)>>,

    /// Called when focus-within-visible state changes.
    pub on_focus_within_visible_change: Option<Callback<dyn Fn(bool)>>,
}

impl Default for FocusWithinConfig {
    fn default() -> Self {
        Self {
            disabled: false,
            modality: ArsRc::from_modality(ars_core::DefaultModalityContext::new()),
            on_focus_within: None,
            on_blur_within: None,
            on_focus_within_visible_change: None,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// FocusResult
// ────────────────────────────────────────────────────────────────────

/// The output of [`use_focus`], providing live attribute generation and state access.
///
/// `FocusResult` attrs are **reactive, not one-shot snapshots**. Use
/// [`current_attrs()`](Self::current_attrs) inside the component's `connect()`
/// method to ensure attributes reflect the current state at DOM reconciliation.
#[derive(Debug)]
pub struct FocusResult {
    /// Internal state handle — use [`current_attrs()`](Self::current_attrs) to
    /// produce a live `AttrMap`.
    state: Rc<RefCell<FocusState>>,

    /// Whether the element is currently focused (reactive signal in adapter).
    pub focused: bool,

    /// Whether the focus ring should be visible (reactive signal in adapter).
    pub focus_visible: bool,
}

impl FocusResult {
    /// Produce a fresh [`AttrMap`] reflecting the current focus state.
    ///
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    #[must_use]
    pub fn current_attrs(&self, config: &FocusConfig) -> AttrMap {
        let state = self.state.borrow();
        let mut attrs = AttrMap::new();
        if state.is_focused() {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        if state.is_focus_visible(config.modality.as_ref()) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }
}

// ────────────────────────────────────────────────────────────────────
// FocusWithinResult
// ────────────────────────────────────────────────────────────────────

/// The output of [`use_focus_within`], providing live attribute generation for
/// container focus tracking.
#[derive(Debug)]
pub struct FocusWithinResult {
    /// Whether any descendant within the container has focus.
    pub focus_within: bool,

    /// Whether focus-within should show a visible indicator.
    pub is_focus_within_visible: bool,

    /// Tracks whether focus is within the container.
    state: Rc<RefCell<bool>>,

    /// Tracks whether focus-within is visible.
    visible: Rc<RefCell<bool>>,
}

impl FocusWithinResult {
    /// Produce a fresh [`AttrMap`] reflecting the current focus-within state.
    ///
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    #[must_use]
    pub fn current_attrs(&self, config: &FocusWithinConfig) -> AttrMap {
        let _config = config;
        let mut attrs = AttrMap::new();
        if *self.state.borrow() {
            attrs.set_bool(HtmlAttr::Data("ars-focus-within"), true);
        }
        if *self.visible.borrow() {
            attrs.set_bool(HtmlAttr::Data("ars-focus-within-visible"), true);
        }
        attrs
    }
}

// ────────────────────────────────────────────────────────────────────
// Factory functions
// ────────────────────────────────────────────────────────────────────

/// Creates a focus interaction state container with the given configuration.
///
/// Returns a [`FocusResult`] holding the initial `Unfocused` state. Event
/// handlers are registered as typed methods on the component's `Api` struct
/// by the framework adapter — this factory only creates the core state container.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "spec API takes ownership; adapters will consume the config for event handler registration"
)]
pub fn use_focus(config: FocusConfig) -> FocusResult {
    let state = Rc::new(RefCell::new(FocusState::Unfocused));
    let focused = state.borrow().is_focused();
    let focus_visible = state.borrow().is_focus_visible(config.modality.as_ref());

    FocusResult {
        state,
        focused,
        focus_visible,
    }
}

/// Creates a focus-within interaction state container with the given configuration.
///
/// Returns a [`FocusWithinResult`] tracking whether any descendant has focus.
/// Event handlers are registered as typed methods on the component's `Api` struct
/// by the framework adapter.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "spec API takes ownership; adapters will consume the config for event handler registration"
)]
pub fn use_focus_within(config: FocusWithinConfig) -> FocusWithinResult {
    let state = Rc::new(RefCell::new(false));
    let visible = Rc::new(RefCell::new(false));

    let _is_disabled = config.disabled;
    let focus_within = *state.borrow();
    let is_focus_within_visible = *visible.borrow();

    FocusWithinResult {
        focus_within,
        is_focus_within_visible,
        state,
        visible,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use ars_core::{
        ArsRc, AttrValue, Callback, DefaultModalityContext, HtmlAttr, KeyModifiers, KeyboardKey,
        ModalityContext, NullModalityContext, PointerType,
    };

    use super::*;

    // ── FocusState tests ────────────────────────────────────────────

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
    fn focus_state_default_is_unfocused() {
        assert_eq!(FocusState::default(), FocusState::Unfocused);
    }

    // ── State transition tests (issue requirements) ──────────────────

    #[test]
    fn unfocused_to_focused_by_keyboard_when_last_modality_is_keyboard() {
        let modality = DefaultModalityContext::new();
        modality.on_key_down(KeyboardKey::Tab, KeyModifiers::default());

        // When modality is Keyboard, focus should transition to FocusedByKeyboard
        let last = modality.last_pointer_type();
        assert_eq!(last, Some(PointerType::Keyboard));

        let state = FocusState::FocusedByKeyboard;
        assert!(state.is_focused());
        assert!(state.is_focus_visible(&modality));
    }

    #[test]
    fn unfocused_to_focused_by_pointer_when_last_modality_is_mouse() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Mouse);

        let state = FocusState::FocusedByPointer;
        assert!(state.is_focused());
        assert!(!state.is_focus_visible(&modality));
    }

    #[test]
    fn unfocused_to_focused_by_pointer_when_last_modality_is_touch() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Touch);

        let state = FocusState::FocusedByPointer;
        assert!(state.is_focused());
        assert!(!state.is_focus_visible(&modality));
    }

    #[test]
    fn unfocused_to_focused_by_pointer_when_last_modality_is_pen() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Pen);

        let state = FocusState::FocusedByPointer;
        assert!(state.is_focused());
        assert!(!state.is_focus_visible(&modality));
    }

    #[test]
    fn unfocused_to_focused_programmatic_when_no_prior_modality() {
        let modality = DefaultModalityContext::new();

        // No prior interaction — programmatic focus
        assert_eq!(modality.last_pointer_type(), None);

        let state = FocusState::FocusedProgrammatic;
        assert!(state.is_focused());
        // No prior pointer interaction → focus ring shown
        assert!(state.is_focus_visible(&modality));
    }

    // ── is_focus_visible tests ──────────────────────────────────────

    #[test]
    fn focus_visible_true_only_for_keyboard() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Mouse);

        assert!(FocusState::FocusedByKeyboard.is_focus_visible(&modality));
        assert!(!FocusState::FocusedByPointer.is_focus_visible(&modality));
        assert!(!FocusState::Unfocused.is_focus_visible(&modality));
    }

    #[test]
    fn programmatic_visibility_uses_injected_modality() {
        let modality = DefaultModalityContext::new();

        // No prior interaction → visible
        assert!(FocusState::FocusedProgrammatic.is_focus_visible(&modality));

        // After pointer → not visible
        modality.on_pointer_down(PointerType::Mouse);
        assert!(!FocusState::FocusedProgrammatic.is_focus_visible(&modality));

        // After keyboard → visible again
        modality.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        assert!(FocusState::FocusedProgrammatic.is_focus_visible(&modality));
    }

    // ── FocusEventType tests ────────────────────────────────────────

    #[test]
    fn focus_event_type_variants_are_distinct() {
        assert_ne!(FocusEventType::Focus, FocusEventType::Blur);
        assert_ne!(FocusEventType::FocusWithin, FocusEventType::BlurWithin);
        assert_ne!(FocusEventType::Focus, FocusEventType::FocusWithin);
    }

    #[test]
    fn focus_event_type_is_copy() {
        let t = FocusEventType::Focus;
        let t2 = t;
        assert_eq!(t, t2);
    }

    // ── FocusEvent tests ────────────────────────────────────────────

    #[test]
    fn focus_event_clone_preserves_fields() {
        let event = FocusEvent {
            event_type: FocusEventType::Focus,
            pointer_type: Some(PointerType::Mouse),
        };
        let cloned = event.clone();
        assert_eq!(cloned.event_type, FocusEventType::Focus);
        assert_eq!(cloned.pointer_type, Some(PointerType::Mouse));
    }

    #[test]
    fn focus_event_debug_output() {
        let event = FocusEvent {
            event_type: FocusEventType::Blur,
            pointer_type: None,
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("Blur"));
    }

    // ── FocusConfig tests ───────────────────────────────────────────

    #[test]
    fn focus_config_default_values() {
        let config = FocusConfig::default();
        assert!(!config.disabled);
        assert!(config.on_focus.is_none());
        assert!(config.on_blur.is_none());
        assert!(config.on_focus_visible_change.is_none());
    }

    #[test]
    fn focus_config_debug_default_shows_none_callbacks() {
        let config = FocusConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("disabled: false"));
        assert!(debug.contains("on_focus: None"));
        assert!(debug.contains("on_blur: None"));
        assert!(debug.contains("on_focus_visible_change: None"));
    }

    #[test]
    fn focus_config_debug_with_callbacks_shows_callback() {
        let config = FocusConfig {
            on_focus: Some(Callback::new(|_: FocusEvent| {})),
            on_blur: Some(Callback::new(|_: FocusEvent| {})),
            ..FocusConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("on_focus: Some(Callback(..))"));
        assert!(debug.contains("on_blur: Some(Callback(..))"));
    }

    #[test]
    fn focus_config_clone_shares_modality() {
        let config = FocusConfig::default();
        let cloned = config.clone();
        assert_eq!(config.modality, cloned.modality);
    }

    #[test]
    fn focus_config_partial_eq_same_modality() {
        let config1 = FocusConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn focus_config_partial_eq_different_modality() {
        let config1 = FocusConfig::default();
        let config2 = FocusConfig {
            modality: ArsRc::from_modality(NullModalityContext),
            ..FocusConfig::default()
        };
        assert_ne!(config1, config2);
    }

    // ── FocusWithinConfig tests ─────────────────────────────────────

    #[test]
    fn focus_within_config_default_values() {
        let config = FocusWithinConfig::default();
        assert!(!config.disabled);
        assert!(config.on_focus_within.is_none());
        assert!(config.on_blur_within.is_none());
        assert!(config.on_focus_within_visible_change.is_none());
    }

    #[test]
    fn focus_within_config_debug_output() {
        let config = FocusWithinConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("disabled: false"));
        assert!(debug.contains("on_focus_within: None"));
    }

    // ── FocusResult / current_attrs tests ───────────────────────────

    #[test]
    fn focus_result_current_attrs_unfocused_is_empty() {
        let result = FocusResult {
            state: Rc::new(RefCell::new(FocusState::Unfocused)),
            focused: false,
            focus_visible: false,
        };
        let config = FocusConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focused")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    #[test]
    fn focus_result_current_attrs_keyboard_sets_both() {
        let result = FocusResult {
            state: Rc::new(RefCell::new(FocusState::FocusedByKeyboard)),
            focused: true,
            focus_visible: true,
        };
        let config = FocusConfig::default();
        let attrs = result.current_attrs(&config);
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focused")),
            Some(&AttrValue::Bool(true))
        );
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-visible")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn focus_result_current_attrs_pointer_sets_only_focused() {
        let result = FocusResult {
            state: Rc::new(RefCell::new(FocusState::FocusedByPointer)),
            focused: true,
            focus_visible: false,
        };
        let config = FocusConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-focused")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    #[test]
    fn focus_result_current_attrs_programmatic_visible_depends_on_modality() {
        let config = FocusConfig {
            modality: ArsRc::from_modality(DefaultModalityContext::new()),
            ..FocusConfig::default()
        };

        let result = FocusResult {
            state: Rc::new(RefCell::new(FocusState::FocusedProgrammatic)),
            focused: true,
            focus_visible: true,
        };

        // No prior pointer → focus-visible should be true
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-visible")));

        // After pointer interaction on config's modality → not visible
        config.modality.on_pointer_down(PointerType::Mouse);
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-visible")));

        // After keyboard → visible again
        config
            .modality
            .on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    // ── FocusWithinResult / current_attrs tests ─────────────────────

    #[test]
    fn focus_within_result_current_attrs_no_focus_is_empty() {
        let result = FocusWithinResult {
            focus_within: false,
            is_focus_within_visible: false,
            state: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(false)),
        };
        let config = FocusWithinConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within-visible")));
    }

    #[test]
    fn focus_within_result_current_attrs_focus_within_sets_attr() {
        let result = FocusWithinResult {
            focus_within: true,
            is_focus_within_visible: false,
            state: Rc::new(RefCell::new(true)),
            visible: Rc::new(RefCell::new(false)),
        };
        let config = FocusWithinConfig::default();
        let attrs = result.current_attrs(&config);
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-within")),
            Some(&AttrValue::Bool(true))
        );
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within-visible")));
    }

    #[test]
    fn focus_within_result_current_attrs_visible_sets_attr() {
        let result = FocusWithinResult {
            focus_within: false,
            is_focus_within_visible: true,
            state: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(true)),
        };
        let config = FocusWithinConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within")));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-within-visible")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn focus_within_result_current_attrs_both_set() {
        let result = FocusWithinResult {
            focus_within: true,
            is_focus_within_visible: true,
            state: Rc::new(RefCell::new(true)),
            visible: Rc::new(RefCell::new(true)),
        };
        let config = FocusWithinConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-within")));
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-within-visible")));
    }

    // ── Factory function tests ──────────────────────────────────────

    #[test]
    fn use_focus_returns_unfocused_state() {
        let result = use_focus(FocusConfig::default());
        assert_eq!(*result.state.borrow(), FocusState::Unfocused);
    }

    #[test]
    fn use_focus_returns_focused_false() {
        let result = use_focus(FocusConfig::default());
        assert!(!result.focused);
    }

    #[test]
    fn use_focus_returns_focus_visible_false() {
        let result = use_focus(FocusConfig::default());
        assert!(!result.focus_visible);
    }

    #[test]
    fn use_focus_disabled_config_still_creates_result() {
        let config = FocusConfig {
            disabled: true,
            ..FocusConfig::default()
        };
        let result = use_focus(config);
        assert_eq!(*result.state.borrow(), FocusState::Unfocused);
        assert!(!result.focused);
    }

    #[test]
    fn use_focus_within_returns_initial_false() {
        let result = use_focus_within(FocusWithinConfig::default());
        assert!(!result.focus_within);
    }

    #[test]
    fn use_focus_within_returns_visible_false() {
        let result = use_focus_within(FocusWithinConfig::default());
        assert!(!result.is_focus_within_visible);
    }

    // ── FocusState derive coverage ─────────────────────────────────

    #[test]
    fn focus_state_is_copy() {
        let a = FocusState::FocusedByKeyboard;
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn focus_state_clone_matches_copy() {
        let a = FocusState::FocusedByPointer;
        #[expect(clippy::clone_on_copy, reason = "explicitly testing Clone impl")]
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ── FocusResult / FocusWithinResult Debug coverage ──────────────

    #[test]
    fn focus_result_debug_output() {
        let result = use_focus(FocusConfig::default());
        let debug = format!("{result:?}");
        assert!(debug.contains("FocusResult"));
        assert!(debug.contains("focused: false"));
        assert!(debug.contains("focus_visible: false"));
    }

    #[test]
    fn focus_within_result_debug_output() {
        let result = use_focus_within(FocusWithinConfig::default());
        let debug = format!("{result:?}");
        assert!(debug.contains("FocusWithinResult"));
        assert!(debug.contains("focus_within: false"));
    }

    // ── FocusWithinConfig derive coverage ────────────────────────────

    #[test]
    fn focus_within_config_clone_shares_modality() {
        let config = FocusWithinConfig::default();
        let cloned = config.clone();
        assert_eq!(config.modality, cloned.modality);
    }

    #[test]
    fn focus_within_config_partial_eq_same() {
        let config1 = FocusWithinConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn focus_within_config_partial_eq_different_modality() {
        let config1 = FocusWithinConfig::default();
        let config2 = FocusWithinConfig {
            modality: ArsRc::from_modality(NullModalityContext),
            ..FocusWithinConfig::default()
        };
        assert_ne!(config1, config2);
    }

    // ── FocusEvent None pointer_type clone ───────────────────────────

    #[test]
    fn focus_event_clone_with_none_pointer_type() {
        let event = FocusEvent {
            event_type: FocusEventType::FocusWithin,
            pointer_type: None,
        };
        let cloned = event.clone();
        assert_eq!(cloned.event_type, FocusEventType::FocusWithin);
        assert_eq!(cloned.pointer_type, None);
    }

    // ── use_focus_within disabled config ─────────────────────────────

    #[test]
    fn use_focus_within_disabled_config_still_creates_result() {
        let config = FocusWithinConfig {
            disabled: true,
            ..FocusWithinConfig::default()
        };
        let result = use_focus_within(config);
        assert!(!result.focus_within);
        assert!(!result.is_focus_within_visible);
    }

    // ── FocusWithin tracking test ───────────────────────────────────

    #[test]
    fn focus_within_tracking_child_focus_propagates() {
        let result = use_focus_within(FocusWithinConfig::default());

        // Simulate child focus: adapter sets state to true
        *result.state.borrow_mut() = true;
        *result.visible.borrow_mut() = true;

        let config = FocusWithinConfig::default();
        let attrs = result.current_attrs(&config);
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-within")));
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-within-visible")));

        // Simulate child blur: adapter sets state to false
        *result.state.borrow_mut() = false;
        *result.visible.borrow_mut() = false;

        let attrs = result.current_attrs(&config);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-within-visible")));
    }
}
