//! Contract tests for standardized component misuse errors.

use ars_core::ComponentError;

#[test]
fn component_error_missing_id_display_is_actionable() {
    let error = ComponentError::MissingId {
        component: "Dialog",
        part: "root",
    };

    assert!(matches!(
        error,
        ComponentError::MissingId {
            component: "Dialog",
            part: "root"
        }
    ));
    assert!(error.to_string().contains("[ars-ui:Dialog]"));
    assert!(error.to_string().contains("Missing required `id`"));
    assert!(error.to_string().contains("Provide an explicit ID"));
}

#[test]
fn component_error_disabled_gate_display_is_actionable() {
    let error = ComponentError::DisabledGate {
        component: "Button",
        event: "press".to_owned(),
    };

    assert!(matches!(
        error,
        ComponentError::DisabledGate {
            component: "Button",
            ..
        }
    ));
    assert!(error.to_string().contains("[ars-ui:Button]"));
    assert!(error.to_string().contains("disabled component"));
    assert!(error.to_string().contains("Check `disabled` prop"));
}

#[test]
fn component_error_invalid_prop_combination_display_is_actionable() {
    let error = ComponentError::InvalidPropCombination {
        component: "Select",
        props: vec!["open", "default_open"],
        reason: "controlled and uncontrolled open state cannot both be set".to_owned(),
    };

    assert!(matches!(
        error,
        ComponentError::InvalidPropCombination {
            component: "Select",
            ..
        }
    ));
    assert!(error.to_string().contains("[ars-ui:Select]"));
    assert!(error.to_string().contains("Invalid prop combination"));
    assert!(error.to_string().contains("controlled and uncontrolled"));
}

#[test]
fn component_error_invalid_state_transition_display_is_actionable() {
    let error = ComponentError::InvalidStateTransition {
        component: "Dialog",
        current_state: "Closed".to_owned(),
        event: "Close".to_owned(),
    };

    assert!(matches!(
        error,
        ComponentError::InvalidStateTransition {
            component: "Dialog",
            ..
        }
    ));
    assert!(error.to_string().contains("[ars-ui:Dialog]"));
    assert!(error.to_string().contains("Cannot handle `Close`"));
    assert!(error.to_string().contains("state `Closed`"));
}

#[test]
fn component_error_lifetime_violation_display_is_actionable() {
    let error = ComponentError::LifetimeViolation {
        component: "Tooltip",
        reason: "service was used after unmount".to_owned(),
    };

    assert!(matches!(
        error,
        ComponentError::LifetimeViolation {
            component: "Tooltip",
            ..
        }
    ));
    assert!(error.to_string().contains("[ars-ui:Tooltip]"));
    assert!(error.to_string().contains("Lifetime violation"));
    assert!(error.to_string().contains("after unmount"));
}
