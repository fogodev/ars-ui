//! Form validation and field state management.
//!
//! This crate provides the validation primitives and field tracking types used by
//! form-related components (text fields, checkboxes, selects, etc.). It defines a
//! [`Validator`] trait for synchronous validation and the [`FieldState`] struct for
//! tracking user interaction (dirty, touched).

extern crate alloc;

use alloc::{string::String, vec::Vec};

/// A single validation failure.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationError {
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable code for programmatic handling.
    pub code: ValidationErrorCode,
}

impl ValidationError {
    /// Returns `true` if this error originated from the server or an async validator.
    ///
    /// Used by `set_server_errors()` to separate server-sourced errors from client-side ones.
    #[must_use]
    pub fn is_server(&self) -> bool {
        matches!(
            &self.code,
            ValidationErrorCode::Server(_) | ValidationErrorCode::Async(_)
        )
    }
}

/// Semantic codes for validation errors.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationErrorCode {
    /// The field value is required but empty.
    Required,
    /// The value is shorter than the minimum length.
    MinLength(usize),
    /// The value exceeds the maximum length.
    MaxLength(usize),
    /// The numeric value is below the minimum.
    Min(f64),
    /// The numeric value exceeds the maximum.
    Max(f64),
    /// The numeric value does not match the step increment.
    Step(f64),
    /// The value does not match the expected pattern.
    Pattern(String),
    /// The value is not a valid email address.
    Email,
    /// The value is not a valid URL.
    Url,
    /// A custom validation rule identified by name.
    Custom(String),
    /// An error returned from the server.
    Server(String),
    /// An error from an async validator.
    Async(String),
}

/// A collection of validation errors for a single field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationErrors(pub Vec<ValidationError>);

impl ValidationErrors {
    /// Creates an empty error collection.
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends an error to the collection.
    pub fn push(&mut self, error: ValidationError) {
        self.0.push(error);
    }

    /// Returns `true` if there are no errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the first error message, if any.
    #[must_use]
    pub fn first_message(&self) -> Option<&str> {
        self.0.first().map(|e| e.message.as_str())
    }

    /// Returns `true` if any error matches the given code.
    #[must_use]
    pub fn has_code(&self, code: &ValidationErrorCode) -> bool {
        self.0.iter().any(|e| &e.code == code)
    }
}

/// The result of validating a field value.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationResult {
    /// The value passed validation.
    Valid,
    /// The value failed validation with one or more errors.
    Invalid(ValidationErrors),
}

impl ValidationResult {
    /// Returns `true` if the validation produced no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Returns `true` if the validation produced errors.
    #[must_use]
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid(_))
    }

    /// Returns the validation errors, if any.
    #[must_use]
    pub fn errors(&self) -> Option<&ValidationErrors> {
        match self {
            Self::Invalid(e) => Some(e),
            Self::Valid => None,
        }
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
    fn valid_result_is_valid() {
        let result = ValidationResult::Valid;
        assert!(result.is_valid());
        assert!(!result.is_invalid());
    }

    #[test]
    fn invalid_result_is_not_valid() {
        let result = ValidationResult::Invalid(ValidationErrors(vec![ValidationError {
            code: ValidationErrorCode::Required,
            message: "This field is required".to_string(),
        }]));
        assert!(!result.is_valid());
        assert!(result.is_invalid());
    }

    #[test]
    fn validation_errors_first_message() {
        let errors = ValidationErrors(vec![ValidationError {
            code: ValidationErrorCode::Required,
            message: "Required".to_string(),
        }]);
        assert_eq!(errors.first_message(), Some("Required"));
    }

    #[test]
    fn validation_errors_has_code() {
        let errors = ValidationErrors(vec![ValidationError {
            code: ValidationErrorCode::MinLength(3),
            message: "Too short".to_string(),
        }]);
        assert!(errors.has_code(&ValidationErrorCode::MinLength(3)));
        assert!(!errors.has_code(&ValidationErrorCode::Required));
    }

    #[test]
    fn validation_error_is_server() {
        let server = ValidationError {
            code: ValidationErrorCode::Server("duplicate".to_string()),
            message: "Already exists".to_string(),
        };
        assert!(server.is_server());

        let client = ValidationError {
            code: ValidationErrorCode::Required,
            message: "Required".to_string(),
        };
        assert!(!client.is_server());
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
                ValidationResult::Invalid(ValidationErrors(vec![ValidationError {
                    code: ValidationErrorCode::Required,
                    message: "Value is required".to_string(),
                }]))
            } else {
                ValidationResult::Valid
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
