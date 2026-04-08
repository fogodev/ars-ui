//! Outside-interaction configuration and event types.
//!
//! This module defines the adapter-facing data model for "interact outside"
//! behavior used by overlays and dismissable surfaces. It intentionally stays
//! free of DOM or framework types so the detection policy can be tested with
//! pure unit tests.

use std::{string::String, vec::Vec};

use ars_core::{Callback, PointerType};

/// Configuration for outside-interaction detection.
///
/// This composable configuration controls whether detection is enabled and
/// whether focus transitions outside the boundary should also be reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InteractOutsideConfig {
    /// Whether outside-interaction detection is disabled.
    pub disabled: bool,

    /// Whether focus moving outside the boundary should also be reported.
    ///
    /// When `false`, only pointer-originated outside interactions are
    /// considered. Overlay components typically set this to `true`.
    pub detect_focus: bool,
}

/// Standalone registration payload for outside-interaction detection.
///
/// Framework adapters use this value to register an interaction boundary,
/// additional portal-owner IDs that should still count as "inside" for
/// teleported content, and the callback to invoke when an outside interaction
/// is detected.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractOutsideStandalone {
    /// The ID of the primary element whose boundary is being monitored.
    pub target_id: String,

    /// Portal-owner IDs that should be treated as inside.
    ///
    /// These IDs correspond to `data-ars-portal-owner` markers applied by the
    /// adapter to teleported content. They are not arbitrary DOM element IDs.
    pub portal_owner_ids: Vec<String>,

    /// Callback invoked when an outside interaction is detected.
    pub on_interact_outside: Option<Callback<dyn Fn(InteractOutsideEvent)>>,

    /// Whether outside-interaction detection is active for this registration.
    pub enabled: bool,

    /// Optional grace period in milliseconds before pointer-outside dismissal.
    ///
    /// Adapters may use this for submenu-style pointer grace handling.
    pub pointer_gracing: Option<u32>,
}

/// A normalized outside-interaction event.
#[derive(Clone, Debug, PartialEq)]
pub enum InteractOutsideEvent {
    /// A pointer interaction occurred outside the registered boundary.
    PointerOutside {
        /// Client-space X coordinate of the pointer event.
        client_x: f64,
        /// Client-space Y coordinate of the pointer event.
        client_y: f64,
        /// The type of pointer that triggered the event.
        pointer_type: PointerType,
    },

    /// Focus moved outside the registered boundary.
    FocusOutside,

    /// The Escape key was pressed while the overlay had focus.
    EscapeKey,
}

/// Returns whether outside-interaction detection should currently be active.
#[cfg(test)]
#[must_use]
fn detection_is_active(
    config: &InteractOutsideConfig,
    standalone: &InteractOutsideStandalone,
) -> bool {
    !config.disabled && standalone.enabled
}

/// Returns whether a resolved portal-owner ID should count as inside.
#[cfg(test)]
#[must_use]
fn portal_owner_id_is_inside(
    standalone: &InteractOutsideStandalone,
    resolved_owner_id: Option<&str>,
) -> bool {
    let Some(resolved_owner_id) = resolved_owner_id else {
        return false;
    };

    standalone
        .portal_owner_ids
        .iter()
        .any(|owner_id| owner_id == resolved_owner_id)
}

#[cfg(test)]
mod tests {
    use ars_core::PointerType;

    use super::*;

    fn sample_standalone() -> InteractOutsideStandalone {
        InteractOutsideStandalone {
            target_id: "popover-1".into(),
            portal_owner_ids: vec!["portal-1".into(), "portal-2".into()],
            on_interact_outside: None,
            enabled: true,
            pointer_gracing: Some(250),
        }
    }

    #[test]
    fn interact_outside_config_defaults() {
        let config = InteractOutsideConfig::default();
        assert!(!config.disabled);
        assert!(!config.detect_focus);
    }

    #[test]
    fn pointer_outside_event_compares_fields() {
        let event = InteractOutsideEvent::PointerOutside {
            client_x: 10.0,
            client_y: 20.0,
            pointer_type: PointerType::Mouse,
        };

        assert_eq!(
            event,
            InteractOutsideEvent::PointerOutside {
                client_x: 10.0,
                client_y: 20.0,
                pointer_type: PointerType::Mouse,
            }
        );
    }

    #[test]
    fn focus_outside_event_is_constructible() {
        assert_eq!(
            InteractOutsideEvent::FocusOutside,
            InteractOutsideEvent::FocusOutside
        );
    }

    #[test]
    fn escape_key_event_is_constructible() {
        assert_eq!(
            InteractOutsideEvent::EscapeKey,
            InteractOutsideEvent::EscapeKey
        );
    }

    #[test]
    fn standalone_clone_preserves_fields_and_callback_identity() {
        let standalone = InteractOutsideStandalone {
            on_interact_outside: Some(Callback::new(|_: InteractOutsideEvent| {})),
            ..sample_standalone()
        };

        let cloned = standalone.clone();

        assert_eq!(standalone.target_id, cloned.target_id);
        assert_eq!(standalone.portal_owner_ids, cloned.portal_owner_ids);
        assert_eq!(standalone.enabled, cloned.enabled);
        assert_eq!(standalone.pointer_gracing, cloned.pointer_gracing);
        assert_eq!(standalone.on_interact_outside, cloned.on_interact_outside);
    }

    #[test]
    fn standalone_debug_redacts_callback_body() {
        let standalone = InteractOutsideStandalone {
            on_interact_outside: Some(Callback::new(|_: InteractOutsideEvent| {})),
            ..sample_standalone()
        };

        let debug = format!("{standalone:?}");
        assert!(debug.contains("target_id: \"popover-1\""));
        assert!(debug.contains("on_interact_outside: Some(Callback(..))"));
        assert!(debug.contains("pointer_gracing: Some(250)"));
    }

    #[test]
    fn standalone_partial_eq_uses_callback_pointer_identity() {
        let callback = Callback::new(|_: InteractOutsideEvent| {});
        let left = InteractOutsideStandalone {
            on_interact_outside: Some(callback.clone()),
            ..sample_standalone()
        };
        let right = InteractOutsideStandalone {
            on_interact_outside: Some(callback),
            ..sample_standalone()
        };
        let different = InteractOutsideStandalone {
            on_interact_outside: Some(Callback::new(|_: InteractOutsideEvent| {})),
            ..sample_standalone()
        };

        assert_eq!(left, right);
        assert_ne!(left, different);
    }

    #[test]
    fn detection_is_inactive_when_config_is_disabled() {
        let config = InteractOutsideConfig {
            disabled: true,
            detect_focus: true,
        };

        assert!(!detection_is_active(&config, &sample_standalone()));
    }

    #[test]
    fn detection_is_inactive_when_standalone_is_disabled() {
        let config = InteractOutsideConfig::default();
        let standalone = InteractOutsideStandalone {
            enabled: false,
            ..sample_standalone()
        };

        assert!(!detection_is_active(&config, &standalone));
    }

    #[test]
    fn detection_is_active_when_both_config_and_registration_are_enabled() {
        assert!(detection_is_active(
            &InteractOutsideConfig::default(),
            &sample_standalone()
        ));
    }

    #[test]
    fn target_id_is_not_treated_as_a_portal_owner_id() {
        let standalone = sample_standalone();
        assert!(!portal_owner_id_is_inside(&standalone, Some("popover-1")));
    }

    #[test]
    fn portal_owner_ids_count_as_inside() {
        let standalone = sample_standalone();
        assert!(portal_owner_id_is_inside(&standalone, Some("portal-2")));
    }

    #[test]
    fn unrelated_owner_id_is_outside() {
        let standalone = sample_standalone();
        assert!(!portal_owner_id_is_inside(
            &standalone,
            Some("other-overlay")
        ));
    }

    #[test]
    fn missing_owner_id_is_outside() {
        let standalone = sample_standalone();
        assert!(!portal_owner_id_is_inside(&standalone, None));
    }

    #[test]
    fn pointer_gracing_is_preserved_without_affecting_boundary_matching() {
        let standalone = sample_standalone();

        assert_eq!(standalone.pointer_gracing, Some(250));
        assert!(portal_owner_id_is_inside(&standalone, Some("portal-1")));
        assert!(!portal_owner_id_is_inside(&standalone, Some("outside")));
    }
}
