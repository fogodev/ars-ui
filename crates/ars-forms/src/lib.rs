//! Form validation and field state management.
//!
//! This crate provides the validation primitives and field tracking types used by
//! form-related components (text fields, checkboxes, selects, etc.). It defines a
//! [`Validator`] trait for synchronous validation and the [`FieldState`] struct for
//! tracking user interaction (dirty, touched).

extern crate alloc;

use alloc::{string::String, vec::Vec};

/// A single validation error with a machine-readable code and human-readable message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    /// A machine-readable error code (e.g. `"required"`, `"min_length"`).
    pub code: String,
    /// A human-readable error message suitable for display to the user.
    pub message: String,
}

/// The result of validating a field value, containing zero or more errors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationResult {
    /// The list of validation errors. An empty list means the value is valid.
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Returns `true` if the validation produced no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Tracks whether a form field has been modified or interacted with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldState {
    /// `true` if the field value has been changed from its initial value.
    pub dirty: bool,
    /// `true` if the field has received and lost focus at least once.
    pub touched: bool,
}

/// Synchronous validator for a field value of type `T`.
pub trait Validator<T> {
    /// Validates the given value and returns a result with any errors found.
    fn validate(&self, value: &T) -> ValidationResult;
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use super::*;

    #[test]
    fn empty_validation_result_is_valid() {
        let result = ValidationResult::default();
        assert!(result.is_valid());
    }

    #[test]
    fn validation_result_with_errors_is_not_valid() {
        let result = ValidationResult {
            errors: vec![ValidationError {
                code: "required".to_string(),
                message: "This field is required".to_string(),
            }],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn field_state_default_not_dirty_not_touched() {
        let state = FieldState::default();
        assert!(!state.dirty);
        assert!(!state.touched);
    }

    struct RequiredValidator;

    impl Validator<String> for RequiredValidator {
        fn validate(&self, value: &String) -> ValidationResult {
            if value.is_empty() {
                ValidationResult {
                    errors: vec![ValidationError {
                        code: "required".to_string(),
                        message: "Value is required".to_string(),
                    }],
                }
            } else {
                ValidationResult::default()
            }
        }
    }

    #[test]
    fn validator_trait_can_be_implemented() {
        let validator = RequiredValidator;
        assert!(!validator.validate(&String::new()).is_valid());
        assert!(validator.validate(&"hello".to_string()).is_valid());
    }
}
