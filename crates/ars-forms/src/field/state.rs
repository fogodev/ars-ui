//! Form field state tracking.
//!
//! [`State`] holds the complete runtime state of a single form field:
//! its current and initial values, interaction flags (dirty, touched), and
//! validation status.

use super::value::Value;
use crate::validation::{Result, ResultExt};

/// The complete state of a single form field.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Initial field value, stored for [`form::Context::reset()`](crate::form::Context::reset).
    pub initial_value: Value,

    /// Current field value.
    pub value: Value,

    /// Whether the user has focused and then blurred this field.
    pub touched: bool,

    /// Whether the value differs from its initial/default value.
    pub dirty: bool,

    /// Current validation result.
    pub validation: Result,

    /// Whether validation is currently running (async validators).
    pub validating: bool,

    /// Monotonically increasing generation counter for async validation
    /// cancellation. Incremented on every value change. When an async
    /// validation future completes, the handler compares its captured
    /// generation against the current `validation_generation` — if they differ,
    /// a newer value has been set and the result is stale, so it is discarded.
    pub validation_generation: u64,
}

impl State {
    /// Creates a new field state with the given initial value.
    ///
    /// The field starts untouched, clean, and valid.
    #[must_use]
    pub fn new(initial: Value) -> Self {
        Self {
            initial_value: initial.clone(),
            value: initial,
            touched: false,
            dirty: false,
            validation: Ok(()),
            validating: false,
            validation_generation: 0,
        }
    }

    /// Whether to show an error (only after the user has interacted).
    ///
    /// Returns `true` when the field is both touched and invalid — this
    /// prevents flashing validation errors on fields the user hasn't
    /// visited yet.
    #[must_use]
    pub const fn show_error(&self) -> bool {
        self.touched && self.validation.is_err()
    }

    /// Whether the field is currently invalid.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        self.validation.is_err()
    }

    /// Get the error message to display.
    ///
    /// Returns the first error message only when [`show_error()`](Self::show_error)
    /// is `true` — i.e., the field has been touched and has validation errors.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        if self.show_error() {
            self.validation.first_error_message()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{Error, ErrorCode, Errors};

    #[test]
    fn new_initializes_clean() {
        let state = State::new(Value::Text("hello".to_string()));

        assert!(!state.dirty);
        assert!(!state.touched);
        assert!(!state.validating);
        assert_eq!(state.validation_generation, 0);
        assert!(state.validation.is_ok());
        assert_eq!(state.value, state.initial_value);
    }

    #[test]
    fn show_error_requires_touched_and_invalid() {
        let mut state = State::new(Value::Text(String::new()));

        // Not touched, not invalid → false
        assert!(!state.show_error());

        // Not touched, invalid → false
        state.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));

        assert!(!state.show_error());

        // Touched, invalid → true
        state.touched = true;

        assert!(state.show_error());

        // Touched, valid → false
        state.validation = Ok(());

        assert!(!state.show_error());
    }

    #[test]
    fn is_invalid_delegates_to_validation() {
        let mut state = State::new(Value::Bool(false));

        assert!(!state.is_invalid());

        state.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));

        assert!(state.is_invalid());
    }

    #[test]
    fn error_message_when_showing() {
        let mut state = State::new(Value::Text(String::new()));

        state.touched = true;
        state.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "Value is required".to_string(),
        }]));

        assert_eq!(state.error_message(), Some("Value is required"));
    }

    #[test]
    fn error_message_none_when_not_touched() {
        let mut state = State::new(Value::Text(String::new()));

        state.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "Value is required".to_string(),
        }]));

        // Not touched — error_message returns None even though invalid
        assert_eq!(state.error_message(), None);
    }
}
