//! Hover interaction types and state machine.
//!
//! The hover interaction tracks pointer position over an element. It applies
//! only to mouse and pen devices; touch and keyboard have no hover concept.
//! Hover state is suppressed while a press is active, preventing false hover
//! during touch interactions that fire both pointer and mouse events.

use ars_core::{AttrMap, Callback, HtmlAttr, ModalityContext, SharedState};

use crate::PointerType;

// ---------------------------------------------------------------------------
// HoverState
// ---------------------------------------------------------------------------

/// The current state of the hover state machine.
///
/// A simple two-state machine: the pointer is either over the element
/// (`Hovered`) or not (`NotHovered`). Only mouse and pen pointers produce
/// hover transitions; touch and keyboard are ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoverState {
    /// Pointer is not over the element.
    #[default]
    NotHovered,
    /// Pointer is over the element.
    Hovered,
}

impl HoverState {
    /// Returns `true` when the pointer is over the element.
    #[must_use]
    pub fn is_hovered(&self) -> bool {
        matches!(self, HoverState::Hovered)
    }
}

// ---------------------------------------------------------------------------
// HoverEventType
// ---------------------------------------------------------------------------

/// The kind of hover event being dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverEventType {
    /// The pointer entered the element.
    HoverStart,
    /// The pointer left the element.
    HoverEnd,
}

// ---------------------------------------------------------------------------
// HoverEvent
// ---------------------------------------------------------------------------

/// A normalized hover event. Only produced for Mouse and Pen pointer types;
/// touch and keyboard do not produce hover events.
#[derive(Clone, Debug)]
pub struct HoverEvent {
    /// Always Mouse or Pen; never Touch, Keyboard, or Virtual.
    pub pointer_type: PointerType,

    /// The type of hover event.
    pub event_type: HoverEventType,
}

// ---------------------------------------------------------------------------
// HoverConfig
// ---------------------------------------------------------------------------

/// Configuration for hover interaction behavior.
///
/// Controls how the hover interaction responds to pointer enter/leave events.
/// Callbacks use [`Callback`] for automatic platform-appropriate pointer type
/// (`Rc` on wasm, `Arc` on native) and built-in `Clone`, `Debug`, and
/// `PartialEq` (by pointer identity).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HoverConfig {
    /// Whether the element is disabled. Disabled elements receive no hover events.
    pub disabled: bool,

    /// Called when the pointer enters the element.
    pub on_hover_start: Option<Callback<dyn Fn(HoverEvent)>>,

    /// Called when the pointer leaves the element.
    pub on_hover_end: Option<Callback<dyn Fn(HoverEvent)>>,

    /// Called whenever hover state changes.
    pub on_hover_change: Option<Callback<dyn Fn(bool)>>,
}

// ---------------------------------------------------------------------------
// Integration helpers (spec §3.4)
// ---------------------------------------------------------------------------

/// Returns `true` when hover should be cleared because a global press is active.
///
/// Hover integration reads the shared modality snapshot instead of a thread-local.
/// Framework adapters call this to decide whether to suppress hover state when a
/// press begins (see spec §3.4).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "adapter integration hook; called by framework adapters, not core. See spec §3.4"
    )
)]
pub(crate) fn should_clear_hover(modality: &dyn ModalityContext) -> bool {
    modality.is_global_press_active()
}

/// Returns whether the modality context has recorded a pointer interaction.
///
/// Used by programmatic focus to decide whether a preceding interaction came
/// from a pointer device (see spec §3.4).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "adapter integration hook; called by framework adapters, not core. See spec §3.4"
    )
)]
pub(crate) fn had_pointer_interaction(modality: &dyn ModalityContext) -> bool {
    modality.had_pointer_interaction()
}

// ---------------------------------------------------------------------------
// HoverResult
// ---------------------------------------------------------------------------

/// The output of [`use_hover`], providing live attribute generation and state access.
///
/// `HoverResult` attrs are **reactive, not one-shot snapshots**. Use
/// [`current_attrs()`](Self::current_attrs) inside the component's `connect()`
/// method to ensure attributes reflect the current state at DOM reconciliation.
#[derive(Debug)]
pub struct HoverResult {
    /// Whether the element is currently hovered (reactive signal in adapter).
    pub hovered: bool,
    /// Internal state handle — use [`current_attrs()`](Self::current_attrs) to
    /// produce a live `AttrMap`.
    state: SharedState<HoverState>,
}

impl HoverResult {
    /// Produce a fresh [`AttrMap`] reflecting the current hover state.
    ///
    /// Call this inside `connect()` — not once at init time — to ensure
    /// the returned attributes are always up to date.
    #[must_use]
    pub fn current_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        self.state.with(|s| {
            if s.is_hovered() {
                attrs.set_bool(HtmlAttr::Data("ars-hovered"), true);
            }
        });
        attrs
    }
}

// ---------------------------------------------------------------------------
// use_hover factory
// ---------------------------------------------------------------------------

/// Creates a hover interaction state machine with the given configuration.
///
/// Returns a [`HoverResult`] holding the initial `NotHovered` state. Event
/// handlers are registered as typed methods on the component's `Api` struct
/// by the framework adapter — this factory only creates the core state container.
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "spec API takes ownership; adapters will consume the config for event handler registration"
)]
pub fn use_hover(config: HoverConfig) -> HoverResult {
    let state = SharedState::new(HoverState::NotHovered);
    let _is_disabled = config.disabled;

    let hovered = state.get().is_hovered();

    HoverResult { hovered, state }
}

#[cfg(test)]
mod tests {
    use ars_core::{AttrValue, DefaultModalityContext, HtmlAttr, PointerType};

    use super::*;

    // --- HoverConfig tests ---

    #[test]
    fn hover_config_default_values() {
        let config = HoverConfig::default();
        assert!(!config.disabled);
        assert!(config.on_hover_start.is_none());
        assert!(config.on_hover_end.is_none());
        assert!(config.on_hover_change.is_none());
    }

    #[test]
    fn hover_config_debug_default_shows_none_callbacks() {
        let config = HoverConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("disabled: false"));
        assert!(debug.contains("on_hover_start: None"));
        assert!(debug.contains("on_hover_end: None"));
        assert!(debug.contains("on_hover_change: None"));
    }

    #[test]
    fn hover_config_debug_with_callbacks_shows_callback() {
        let config = HoverConfig {
            on_hover_start: Some(Callback::new(|_: HoverEvent| {})),
            on_hover_end: Some(Callback::new(|_: HoverEvent| {})),
            on_hover_change: Some(Callback::new(|_: bool| {})),
            ..HoverConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("on_hover_start: Some(Callback(..))"));
        assert!(debug.contains("on_hover_end: Some(Callback(..))"));
        assert!(debug.contains("on_hover_change: Some(Callback(..))"));
    }

    #[test]
    fn hover_config_clone_preserves_disabled_and_shares_callbacks() {
        let config = HoverConfig {
            disabled: true,
            on_hover_start: Some(Callback::new(|_: HoverEvent| {})),
            ..HoverConfig::default()
        };
        let cloned = config.clone();
        assert!(cloned.disabled);
        // Callback clone shares the same allocation (pointer identity)
        assert_eq!(config.on_hover_start, cloned.on_hover_start);
    }

    #[test]
    fn hover_config_partial_eq_uses_pointer_identity_for_callbacks() {
        let cb = Callback::new(|_: HoverEvent| {});
        let config1 = HoverConfig {
            on_hover_start: Some(cb.clone()),
            ..HoverConfig::default()
        };
        let config2 = HoverConfig {
            on_hover_start: Some(cb),
            ..HoverConfig::default()
        };
        // Same callback allocation → equal
        assert_eq!(config1, config2);

        // Different callback allocation → not equal (even if same closure body)
        let config3 = HoverConfig {
            on_hover_start: Some(Callback::new(|_: HoverEvent| {})),
            ..HoverConfig::default()
        };
        assert_ne!(config1, config3);
    }

    // --- HoverEventType tests ---

    #[test]
    fn hover_event_type_variants_are_distinct() {
        assert_ne!(HoverEventType::HoverStart, HoverEventType::HoverEnd);
    }

    #[test]
    fn hover_event_type_is_copy() {
        let t = HoverEventType::HoverStart;
        let t2 = t;
        assert_eq!(t, t2);
    }

    // --- HoverEvent tests ---

    #[test]
    fn hover_event_construction_with_mouse() {
        let event = HoverEvent {
            pointer_type: PointerType::Mouse,
            event_type: HoverEventType::HoverStart,
        };
        assert_eq!(event.pointer_type, PointerType::Mouse);
        assert_eq!(event.event_type, HoverEventType::HoverStart);
    }

    #[test]
    fn hover_event_construction_with_pen() {
        let event = HoverEvent {
            pointer_type: PointerType::Pen,
            event_type: HoverEventType::HoverEnd,
        };
        assert_eq!(event.pointer_type, PointerType::Pen);
        assert_eq!(event.event_type, HoverEventType::HoverEnd);
    }

    #[test]
    fn hover_event_clone() {
        let event = HoverEvent {
            pointer_type: PointerType::Mouse,
            event_type: HoverEventType::HoverStart,
        };
        let cloned = event.clone();
        assert_eq!(cloned.pointer_type, PointerType::Mouse);
        assert_eq!(cloned.event_type, HoverEventType::HoverStart);
    }

    // --- HoverState tests ---

    #[test]
    fn hover_state_default_is_not_hovered() {
        assert_eq!(HoverState::default(), HoverState::NotHovered);
    }

    #[test]
    fn hover_state_not_hovered_is_not_hovered() {
        assert!(!HoverState::NotHovered.is_hovered());
    }

    #[test]
    fn hover_state_hovered_is_hovered() {
        assert!(HoverState::Hovered.is_hovered());
    }

    // --- HoverResult::current_attrs tests ---

    #[test]
    fn hover_result_current_attrs_not_hovered_is_empty() {
        let result = HoverResult {
            state: SharedState::new(HoverState::NotHovered),
            hovered: false,
        };
        let attrs = result.current_attrs();
        assert!(!attrs.contains(&HtmlAttr::Data("ars-hovered")));
    }

    #[test]
    fn hover_result_current_attrs_hovered_sets_data_ars_hovered() {
        let result = HoverResult {
            state: SharedState::new(HoverState::Hovered),
            hovered: true,
        };
        let attrs = result.current_attrs();
        assert!(attrs.contains(&HtmlAttr::Data("ars-hovered")));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-hovered")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn hover_result_current_attrs_reflects_live_state_mutation() {
        let state = SharedState::new(HoverState::NotHovered);
        let result = HoverResult {
            state: state.clone(),
            hovered: false,
        };

        // Initially not hovered
        assert!(
            !result
                .current_attrs()
                .contains(&HtmlAttr::Data("ars-hovered"))
        );

        // Mutate shared state to Hovered (simulating adapter pointer-enter handler)
        state.set(HoverState::Hovered);

        // current_attrs() must reflect the live state, not a stale snapshot
        let attrs = result.current_attrs();
        assert!(attrs.contains(&HtmlAttr::Data("ars-hovered")));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-hovered")),
            Some(&AttrValue::Bool(true))
        );

        // Mutate back to NotHovered
        state.set(HoverState::NotHovered);
        assert!(
            !result
                .current_attrs()
                .contains(&HtmlAttr::Data("ars-hovered"))
        );
    }

    // --- use_hover tests ---

    #[test]
    fn use_hover_returns_not_hovered_state() {
        let result = use_hover(HoverConfig::default());
        assert_eq!(result.state.get(), HoverState::NotHovered);
    }

    #[test]
    fn use_hover_returns_hovered_false() {
        let result = use_hover(HoverConfig::default());
        assert!(!result.hovered);
    }

    #[test]
    fn use_hover_disabled_config_still_creates_result() {
        let config = HoverConfig {
            disabled: true,
            ..HoverConfig::default()
        };
        let result = use_hover(config);
        assert_eq!(result.state.get(), HoverState::NotHovered);
        assert!(!result.hovered);
    }

    // --- Integration helper tests ---

    #[test]
    fn should_clear_hover_false_when_no_press() {
        let modality = DefaultModalityContext::new();
        assert!(!should_clear_hover(&modality));
    }

    #[test]
    fn should_clear_hover_true_when_press_active() {
        let modality = DefaultModalityContext::new();
        modality.set_global_press_active(true);
        assert!(should_clear_hover(&modality));
    }

    #[test]
    fn had_pointer_interaction_false_initially() {
        let modality = DefaultModalityContext::new();
        assert!(!had_pointer_interaction(&modality));
    }

    #[test]
    fn had_pointer_interaction_true_after_pointer_down() {
        let modality = DefaultModalityContext::new();
        modality.on_pointer_down(PointerType::Mouse);
        assert!(had_pointer_interaction(&modality));
    }
}
