//! Validation error types.
//!
//! Defines [`Error`], [`ErrorCode`], and [`Errors`] —
//! the building blocks for reporting field-level validation failures.

/// A single validation failure.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable code for programmatic handling.
    pub code: ErrorCode,
}

impl Error {
    /// Creates a custom validation error with a named code and message.
    pub fn custom(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ErrorCode::Custom(code.into()),
        }
    }

    /// Creates a server-originated validation error.
    ///
    /// The message is cloned into the [`ErrorCode::Server`] variant
    /// so that `is_server()` can identify it.
    pub fn server(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: ErrorCode::Server(message.clone()),
            message,
        }
    }

    /// Returns `true` if this error originated from the server or an async validator.
    ///
    /// Used by [`FormContext::set_server_errors()`](crate::FormContext::set_server_errors)
    /// to separate server-sourced errors from client-side ones.
    #[must_use]
    pub fn is_server(&self) -> bool {
        matches!(&self.code, ErrorCode::Server(_) | ErrorCode::Async(_))
    }
}

/// Semantic codes for validation errors.
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorCode {
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
pub struct Errors(pub Vec<Error>);

impl Errors {
    /// Creates an empty error collection.
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends an error to the collection.
    pub fn push(&mut self, error: Error) {
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
    pub fn has_code(&self, code: &ErrorCode) -> bool {
        self.0.iter().any(|e| &e.code == code)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn valid_result_is_server() {
        let server = Error {
            code: ErrorCode::Server("duplicate".to_string()),
            message: "Already exists".to_string(),
        };
        assert!(server.is_server());

        let client = Error {
            code: ErrorCode::Required,
            message: "Required".to_string(),
        };
        assert!(!client.is_server());
    }

    #[test]
    fn validation_errors_first_message() {
        let errors = Errors(vec![Error {
            code: ErrorCode::Required,
            message: "Required".to_string(),
        }]);
        assert_eq!(errors.first_message(), Some("Required"));
    }

    #[test]
    fn validation_errors_has_code() {
        let errors = Errors(vec![Error {
            code: ErrorCode::MinLength(3),
            message: "Too short".to_string(),
        }]);
        assert!(errors.has_code(&ErrorCode::MinLength(3)));
        assert!(!errors.has_code(&ErrorCode::Required));
    }

    #[test]
    fn validation_error_custom_factory() {
        let err = Error::custom("my_code", "My message");
        assert_eq!(err.message, "My message");
        assert_eq!(err.code, ErrorCode::Custom("my_code".to_string()));
        assert!(!err.is_server());
    }

    #[test]
    fn validation_error_server_factory() {
        let err = Error::server("Already exists");
        assert_eq!(err.message, "Already exists");
        assert_eq!(err.code, ErrorCode::Server("Already exists".to_string()));
        assert!(err.is_server());
    }

    #[test]
    fn async_error_is_server() {
        let err = Error {
            code: ErrorCode::Async("check_unique".to_string()),
            message: "Not unique".to_string(),
        };
        assert!(err.is_server());
    }

    #[test]
    fn errors_new_and_push() {
        let mut errors = Errors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);

        errors.push(Error::custom("a", "first"));
        errors.push(Error::custom("b", "second"));
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn errors_first_message_on_empty() {
        let errors = Errors::new();
        assert_eq!(errors.first_message(), None);
    }
}
