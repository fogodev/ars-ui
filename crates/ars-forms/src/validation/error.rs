//! Validation error types.
//!
//! Defines [`Error`], [`ErrorCode`], and [`Errors`] —
//! the building blocks for reporting field-level validation failures.

use ars_i18n::Locale;

use crate::form_messages::FormMessages;

/// A single validation failure.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    /// Human-readable error message.
    pub message: String,

    /// Machine-readable code for programmatic handling.
    pub code: ErrorCode,
}

impl Error {
    /// Creates a required-field validation error with a localized message.
    pub fn required(messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.required_error)(locale),
            code: ErrorCode::Required,
        }
    }

    /// Creates a minimum-length validation error with a localized message.
    pub fn min_length(min: usize, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.min_length_error)(min, locale),
            code: ErrorCode::MinLength(min),
        }
    }

    /// Creates a maximum-length validation error with a localized message.
    pub fn max_length(max: usize, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.max_length_error)(max, locale),
            code: ErrorCode::MaxLength(max),
        }
    }

    /// Creates a pattern validation error with a localized message.
    pub fn pattern(pattern: impl Into<String>, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.pattern_error)(locale),
            code: ErrorCode::Pattern(pattern.into()),
        }
    }

    /// Creates a minimum-value validation error with a localized message.
    pub fn min(min: f64, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.min_error)(min, locale),
            code: ErrorCode::Min(min),
        }
    }

    /// Creates a maximum-value validation error with a localized message.
    pub fn max(max: f64, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.max_error)(max, locale),
            code: ErrorCode::Max(max),
        }
    }

    /// Creates an email validation error with a localized message.
    pub fn email(messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.email_error)(locale),
            code: ErrorCode::Email,
        }
    }

    /// Creates a step validation error with a localized message.
    pub fn step(step: f64, messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.step_error)(step, locale),
            code: ErrorCode::Step(step),
        }
    }

    /// Creates a URL validation error with a localized message.
    pub fn url(messages: &FormMessages, locale: &Locale) -> Self {
        Self {
            message: (messages.url_error)(locale),
            code: ErrorCode::Url,
        }
    }

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
    pub const fn is_server(&self) -> bool {
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
    pub const fn new() -> Self {
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
    use ars_i18n::locales;

    use super::*;
    use crate::form_messages::FormMessages;

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

    #[test]
    fn error_required_factory() {
        let err = Error::required(&FormMessages::default(), &locales::en());

        assert_eq!(err.message, "This field is required");
        assert_eq!(err.code, ErrorCode::Required);
    }

    #[test]
    fn error_min_length_factory() {
        let err = Error::min_length(3, &FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Must be at least 3 characters");
        assert_eq!(err.code, ErrorCode::MinLength(3));
    }

    #[test]
    fn error_max_length_factory() {
        let err = Error::max_length(8, &FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Must be at most 8 characters");
        assert_eq!(err.code, ErrorCode::MaxLength(8));
    }

    #[test]
    fn error_pattern_factory() {
        let err = Error::pattern(r"^[a-z]+$", &FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Invalid format");
        assert_eq!(err.code, ErrorCode::Pattern(String::from(r"^[a-z]+$")));
    }

    #[test]
    fn validation_errors_has_code_matches_pattern_payload() {
        let pattern = String::from(r"^[a-z]+$");
        let errors = Errors(vec![Error::pattern(
            pattern.clone(),
            &FormMessages::default(),
            &locales::en(),
        )]);

        assert!(errors.has_code(&ErrorCode::Pattern(pattern)));
    }

    #[test]
    fn error_min_factory() {
        let err = Error::min(2.5, &FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Must be at least 2.5");
        assert_eq!(err.code, ErrorCode::Min(2.5));
    }

    #[test]
    fn error_max_factory() {
        let err = Error::max(9.5, &FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Must be at most 9.5");
        assert_eq!(err.code, ErrorCode::Max(9.5));
    }

    #[test]
    fn error_email_factory() {
        let err = Error::email(&FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Must be a valid email address");
        assert_eq!(err.code, ErrorCode::Email);
    }

    #[test]
    fn error_step_factory() {
        let err = Error::step(0.25, &FormMessages::default(), &locales::en());

        assert_eq!(
            err.message,
            "Please enter a valid value. The nearest allowed value is a multiple of 0.25."
        );
        assert_eq!(err.code, ErrorCode::Step(0.25));
    }

    #[test]
    fn error_url_factory() {
        let err = Error::url(&FormMessages::default(), &locales::en());

        assert_eq!(err.message, "Please enter a valid URL.");
        assert_eq!(err.code, ErrorCode::Url);
    }
}
