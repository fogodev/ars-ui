//! Standard component misuse errors.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Display};

/// Standardized error type for ars-ui component API misuse.
///
/// Components use this error for recoverable developer-facing misuse such as
/// missing IDs, blocked disabled interactions, invalid prop combinations, and
/// invalid state-machine requests.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    /// A required component part ID was not provided.
    MissingId {
        /// Component name that rejected the props.
        component: &'static str,

        /// Component part missing an ID.
        part: &'static str,
    },

    /// A disabled component received an event that should have been blocked.
    DisabledGate {
        /// Component name that rejected the event.
        component: &'static str,

        /// Event name that was sent while disabled.
        event: String,
    },

    /// Two or more props conflict and cannot be used together.
    InvalidPropCombination {
        /// Component name that rejected the prop combination.
        component: &'static str,

        /// Props that conflict with each other.
        props: Vec<&'static str>,

        /// Actionable reason the combination is invalid.
        reason: String,
    },

    /// A state machine received an event that violates its protocol.
    InvalidStateTransition {
        /// Component name whose state machine rejected the event.
        component: &'static str,

        /// Current state at the time of rejection.
        current_state: String,

        /// Event name that is invalid for the current state.
        event: String,
    },

    /// A lifetime or ownership constraint was violated.
    LifetimeViolation {
        /// Component name that detected the lifetime violation.
        component: &'static str,

        /// Actionable reason the lifetime contract was violated.
        reason: String,
    },
}

impl Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingId { component, part } => write!(
                f,
                "[ars-ui:{component}] Missing required `id` on part `{part}`. \
                 Provide an explicit ID or use the default ID generation."
            ),

            Self::DisabledGate { component, event } => write!(
                f,
                "[ars-ui:{component}] Event `{event}` was sent to a disabled component. \
                 Check `disabled` prop before dispatching."
            ),

            Self::InvalidPropCombination {
                component,
                props,
                reason,
            } => write!(
                f,
                "[ars-ui:{component}] Invalid prop combination {props:?}: {reason}"
            ),

            Self::InvalidStateTransition {
                component,
                current_state,
                event,
            } => write!(
                f,
                "[ars-ui:{component}] Cannot handle `{event}` in state `{current_state}`. \
                 This is likely a bug in event dispatch logic."
            ),

            Self::LifetimeViolation { component, reason } => {
                write!(f, "[ars-ui:{component}] Lifetime violation: {reason}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl core::error::Error for ComponentError {}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use super::ComponentError;

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
            event: "press".to_string(),
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
            reason: "controlled and uncontrolled open state cannot both be set".to_string(),
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
            current_state: "Closed".to_string(),
            event: "Close".to_string(),
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
            reason: "service was used after unmount".to_string(),
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
}
