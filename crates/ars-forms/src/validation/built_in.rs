//! Built-in synchronous validators for common form constraints.
//!
//! These validators implement the spec-defined rules for required values,
//! length constraints, numeric bounds, pattern matching, email addresses,
//! step increments, URLs, and closure-backed custom logic.

use std::fmt::{self, Display};

use regex::Regex;

use super::{
    BoxedValidator, Context, Error, ErrorCode, Errors, Result, Validator, boxed_validator,
    validator::DEFAULT_VALIDATOR_LOCALE,
};
use crate::{field::Value, form_messages::FormMessages};

/// Fails when a field's value is considered empty by the forms contract.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RequiredValidator {
    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl RequiredValidator {
    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for RequiredValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let is_empty = match value {
            Value::Text(s) => s.trim().is_empty(),
            Value::Number(None) | Value::Bool(false) => true,
            Value::Number(Some(_)) | Value::Bool(true) => false,
            Value::MultipleText(values) => values.is_empty(),
            Value::File(files) => files.is_empty(),
            Value::Date(date) => date.is_none(),
            Value::Time(time) => time.is_none(),
            Value::DateRange(range) => range.is_none(),
        };

        if is_empty {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

            let error = self.message.as_ref().map_or_else(
                || Error::required(&FormMessages::default(), locale),
                |message| Error {
                    message: message.clone(),
                    code: ErrorCode::Required,
                },
            );

            Err(Errors(vec![error]))
        } else {
            Ok(())
        }
    }
}

/// Fails when the stringified value is shorter than the configured minimum.
#[derive(Clone, Debug, PartialEq)]
pub struct MinLengthValidator {
    /// Minimum number of Unicode scalar values required.
    pub min: usize,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl MinLengthValidator {
    /// Creates a minimum-length validator with the default localized message.
    #[must_use]
    pub const fn with_length(min: usize) -> Self {
        Self { min, message: None }
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for MinLengthValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let value = value.to_string_for_validation();

        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

        if value.chars().count() < self.min {
            let error = self.message.clone().map_or_else(
                || Error::min_length(self.min, &FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::MinLength(self.min),
                },
            );

            Err(Errors(vec![error]))
        } else {
            Ok(())
        }
    }
}

/// Fails when the stringified value exceeds the configured maximum length.
#[derive(Clone, Debug, PartialEq)]
pub struct MaxLengthValidator {
    /// Maximum number of Unicode scalar values allowed.
    pub max: usize,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl MaxLengthValidator {
    /// Creates a maximum-length validator with the default localized message.
    #[must_use]
    pub const fn with_length(max: usize) -> Self {
        Self { max, message: None }
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for MaxLengthValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let value = value.to_string_for_validation();

        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

        if value.chars().count() > self.max {
            let error = self.message.clone().map_or_else(
                || Error::max_length(self.max, &FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::MaxLength(self.max),
                },
            );

            Err(Errors(vec![error]))
        } else {
            Ok(())
        }
    }
}

/// Fails when a numeric value is smaller than the configured minimum.
#[derive(Clone, Debug, PartialEq)]
pub struct MinValidator {
    /// Inclusive minimum numeric value.
    pub min: f64,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl MinValidator {
    /// Creates a minimum-value validator with the default localized message.
    #[must_use]
    pub const fn with_value(min: f64) -> Self {
        Self { min, message: None }
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for MinValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let Some(number) = value.as_number() else {
            return Ok(());
        };

        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

        if number < self.min {
            let error = self.message.clone().map_or_else(
                || Error::min(self.min, &FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::Min(self.min),
                },
            );

            Err(Errors(vec![error]))
        } else {
            Ok(())
        }
    }
}

/// Fails when a numeric value is larger than the configured maximum.
#[derive(Clone, Debug, PartialEq)]
pub struct MaxValidator {
    /// Inclusive maximum numeric value.
    pub max: f64,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl MaxValidator {
    /// Creates a maximum-value validator with the default localized message.
    #[must_use]
    pub const fn with_value(max: f64) -> Self {
        Self { max, message: None }
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for MaxValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let Some(number) = value.as_number() else {
            return Ok(());
        };

        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

        if number > self.max {
            let error = self.message.clone().map_or_else(
                || Error::max(self.max, &FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::Max(self.max),
                },
            );

            Err(Errors(vec![error]))
        } else {
            Ok(())
        }
    }
}

/// Regex-based validator that matches the entire field value.
#[derive(Clone, Debug)]
pub struct PatternValidator {
    /// Cached compiled regex using an anchored `^(?:...)$` pattern.
    pub compiled: Regex,

    /// Original unanchored pattern retained for error reporting.
    pub pattern: String,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

/// Error returned when a [`PatternValidator`] cannot be constructed.
#[derive(Debug)]
pub enum PatternValidatorError {
    /// The pattern exceeded the maximum accepted UTF-8 byte length.
    PatternTooLong {
        /// Maximum accepted UTF-8 byte length.
        max_bytes: usize,

        /// Actual UTF-8 byte length of the provided pattern.
        actual_bytes: usize,
    },

    /// The pattern was not valid `regex` syntax after anchoring.
    InvalidRegex {
        /// Source error returned by the `regex` crate.
        source: regex::Error,
    },
}

impl Display for PatternValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PatternTooLong {
                max_bytes,
                actual_bytes,
            } => {
                write!(
                    f,
                    "pattern exceeds {max_bytes} bytes (got {actual_bytes} bytes)"
                )
            }
            Self::InvalidRegex { source } => write!(f, "invalid regex pattern: {source}"),
        }
    }
}

impl std::error::Error for PatternValidatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PatternTooLong { .. } => None,
            Self::InvalidRegex { source } => Some(source),
        }
    }
}

impl PatternValidator {
    /// Creates a validator and eagerly compiles the pattern.
    ///
    /// # Errors
    ///
    /// Returns [`PatternValidatorError::PatternTooLong`] when `pattern`
    /// exceeds 1024 UTF-8 bytes, or [`PatternValidatorError::InvalidRegex`]
    /// when the anchored pattern is not valid `regex` syntax.
    pub fn new(pattern: impl Into<String>) -> std::result::Result<Self, PatternValidatorError> {
        let pattern = pattern.into();

        const MAX_PATTERN_LEN: usize = 1024;

        if pattern.len() > MAX_PATTERN_LEN {
            return Err(PatternValidatorError::PatternTooLong {
                max_bytes: MAX_PATTERN_LEN,
                actual_bytes: pattern.len(),
            });
        }

        let anchored = format!("^(?:{pattern})$");

        let compiled = Regex::new(&anchored)
            .map_err(|source| PatternValidatorError::InvalidRegex { source })?;

        Ok(Self {
            compiled,
            pattern,
            message: None,
        })
    }

    /// Creates a validator from a hardcoded pattern and panics on invalid syntax.
    ///
    /// # Panics
    ///
    /// Panics when `pattern` exceeds 1024 bytes or is not valid regex syntax.
    #[must_use]
    pub fn new_from_static(pattern: &'static str) -> Self {
        Self::new(pattern).unwrap_or_else(|err| panic!("PatternValidator::new_from_static: {err}"))
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for PatternValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let value = value.to_string_for_validation();

        if value.is_empty() {
            return Ok(());
        }

        if self.compiled.is_match(&value) {
            Ok(())
        } else {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

            let error = self.message.clone().map_or_else(
                || Error::pattern(self.pattern.clone(), &FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::Pattern(self.pattern.clone()),
                },
            );

            Err(Errors(vec![error]))
        }
    }
}

/// Email validator with RFC-style parsing when the default
/// `email-validation` feature is enabled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmailValidator {
    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl EmailValidator {
    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for EmailValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let value = value.to_string_for_validation();

        if value.is_empty() {
            return Ok(());
        }

        let valid = is_valid_email(&value);

        if valid {
            return Ok(());
        }

        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

        let error = self.message.clone().map_or_else(
            || Error::email(&FormMessages::default(), locale),
            |message| Error {
                message,
                code: ErrorCode::Email,
            },
        );

        Err(Errors(vec![error]))
    }
}

#[cfg(feature = "email-validation")]
fn is_valid_email(value: &str) -> bool {
    value.parse::<addr_spec::AddrSpec>().is_ok()
}

#[cfg(not(feature = "email-validation"))]
fn is_valid_email(value: &str) -> bool {
    value
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}

/// Validates that a numeric value falls on a configured step increment.
#[derive(Clone, Debug, PartialEq)]
pub struct StepValidator {
    /// Step increment used to validate numeric values.
    pub step: f64,

    /// Base offset from which step increments are measured.
    pub step_base: f64,

    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl StepValidator {
    /// Creates a step validator using `0.0` as the initial base.
    #[must_use]
    pub const fn new(step: f64) -> Self {
        Self {
            step,
            step_base: 0.0,
            message: None,
        }
    }

    /// Returns a copy configured to measure steps from `base`.
    #[must_use]
    pub const fn with_base(mut self, base: f64) -> Self {
        self.step_base = base;
        self
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for StepValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        if let Some(number) = value.as_number() {
            if !self.step.is_finite() || self.step <= 0.0 {
                let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

                let error = self.message.clone().map_or_else(
                    || Error::step(self.step, &FormMessages::default(), locale),
                    |message| Error {
                        message,
                        code: ErrorCode::Step(self.step),
                    },
                );

                return Err(Errors(vec![error]));
            }

            let remainder = ((number - self.step_base) % self.step).abs();

            if remainder > f64::EPSILON && (self.step - remainder) > f64::EPSILON {
                let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

                let error = self.message.clone().map_or_else(
                    || Error::step(self.step, &FormMessages::default(), locale),
                    |message| Error {
                        message,
                        code: ErrorCode::Step(self.step),
                    },
                );

                return Err(Errors(vec![error]));
            }
        }

        Ok(())
    }
}

/// URL validator using WHATWG parsing when the default
/// `url-validation` feature is enabled.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UrlValidator {
    /// Optional custom message overriding the localized default.
    pub message: Option<String>,
}

impl UrlValidator {
    /// Creates a URL validator with the default localized error message.
    #[must_use]
    pub const fn new() -> Self {
        Self { message: None }
    }

    /// Returns a copy configured to use a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl Validator for UrlValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        if let Some(value) = value.as_text()
            && !value.is_empty()
            && !is_valid_url(value)
        {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);

            let error = self.message.clone().map_or_else(
                || Error::url(&FormMessages::default(), locale),
                |message| Error {
                    message,
                    code: ErrorCode::Url,
                },
            );

            return Err(Errors(vec![error]));
        }

        Ok(())
    }
}

/// Closure-backed validator for custom synchronous validation logic.
#[derive(Clone, Debug)]
pub struct FnValidator<F>
where
    F: Fn(&Value, &Context) -> Result + Send + Sync,
{
    /// The closure implementing validation behavior.
    pub f: F,
}

impl<F> Validator for FnValidator<F>
where
    F: Fn(&Value, &Context) -> Result + Send + Sync,
{
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        (self.f)(value, ctx)
    }
}

impl<F> FnValidator<F>
where
    F: Fn(&Value, &Context) -> Result + Send + Sync + 'static,
{
    /// Wraps a closure as a validator value.
    #[must_use]
    pub const fn new(f: F) -> Self {
        Self { f }
    }

    /// Boxes the validator behind the standard shared pointer type.
    #[must_use]
    pub fn boxed(self) -> BoxedValidator {
        boxed_validator(self)
    }
}

#[cfg(feature = "url-validation")]
fn is_valid_url(value: &str) -> bool {
    url::Url::parse(value).is_ok()
}

#[cfg(not(feature = "url-validation"))]
fn is_valid_url(value: &str) -> bool {
    value
        .find("://")
        .is_some_and(|position| value.len() > position + 3)
}

#[cfg(test)]
mod tests {
    use ars_i18n::locales;

    use super::*;
    use crate::field::FileRef;

    fn error_code(result: Result) -> ErrorCode {
        result.expect_err("validation should fail").0[0]
            .code
            .clone()
    }

    fn error_message(result: Result) -> String {
        result.expect_err("validation should fail").0[0]
            .message
            .clone()
    }

    #[test]
    fn required_empty_text_fails() {
        let validator = RequiredValidator::default();

        assert_eq!(
            error_code(validator.validate(&Value::Text("   ".into()), &Context::standalone("x"))),
            ErrorCode::Required
        );
    }

    #[test]
    fn required_nonempty_text_passes() {
        let validator = RequiredValidator::default();

        assert!(
            validator
                .validate(&Value::Text("hello".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn required_none_number_fails() {
        let validator = RequiredValidator::default();

        assert_eq!(
            error_code(validator.validate(&Value::Number(None), &Context::standalone("x"))),
            ErrorCode::Required
        );
    }

    #[test]
    fn required_some_number_passes() {
        let validator = RequiredValidator::default();

        assert!(
            validator
                .validate(&Value::Number(Some(1.0)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn required_false_bool_fails() {
        let validator = RequiredValidator::default();

        assert_eq!(
            error_code(validator.validate(&Value::Bool(false), &Context::standalone("x"))),
            ErrorCode::Required
        );
    }

    #[test]
    fn required_true_bool_passes() {
        let validator = RequiredValidator::default();

        assert!(
            validator
                .validate(&Value::Bool(true), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn required_empty_multiple_text_fails() {
        let validator = RequiredValidator::default();

        assert_eq!(
            error_code(validator.validate(&Value::MultipleText(vec![]), &Context::standalone("x"))),
            ErrorCode::Required
        );
    }

    #[test]
    fn required_empty_file_list_fails() {
        let validator = RequiredValidator::default();

        assert_eq!(
            error_code(validator.validate(&Value::File(vec![]), &Context::standalone("x"))),
            ErrorCode::Required
        );
    }

    #[test]
    fn required_missing_temporal_values_fail() {
        let validator = RequiredValidator::default();

        let ctx = Context::standalone("x");

        assert_eq!(
            error_code(validator.validate(&Value::Date(None), &ctx)),
            ErrorCode::Required
        );
        assert_eq!(
            error_code(validator.validate(&Value::Time(None), &ctx)),
            ErrorCode::Required
        );
        assert_eq!(
            error_code(validator.validate(&Value::DateRange(None), &ctx)),
            ErrorCode::Required
        );
    }

    #[test]
    fn min_length_short_fails() {
        let validator = MinLengthValidator::with_length(3);

        assert_eq!(
            error_code(validator.validate(&Value::Text("hi".into()), &Context::standalone("x"))),
            ErrorCode::MinLength(3)
        );
    }

    #[test]
    fn min_length_exact_passes() {
        let validator = MinLengthValidator::with_length(3);

        assert!(
            validator
                .validate(&Value::Text("hey".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn max_length_long_fails() {
        let validator = MaxLengthValidator::with_length(3);

        assert_eq!(
            error_code(validator.validate(&Value::Text("long".into()), &Context::standalone("x"))),
            ErrorCode::MaxLength(3)
        );
    }

    #[test]
    fn max_length_exact_passes() {
        let validator = MaxLengthValidator::with_length(4);

        assert!(
            validator
                .validate(&Value::Text("long".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn min_below_fails() {
        let validator = MinValidator::with_value(10.0);

        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(9.0)), &Context::standalone("x"))),
            ErrorCode::Min(10.0)
        );
    }

    #[test]
    fn min_at_boundary_passes() {
        let validator = MinValidator::with_value(10.0);

        assert!(
            validator
                .validate(&Value::Number(Some(10.0)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn max_above_fails() {
        let validator = MaxValidator::with_value(10.0);

        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(11.0)), &Context::standalone("x"))),
            ErrorCode::Max(10.0)
        );
    }

    #[test]
    fn max_at_boundary_passes() {
        let validator = MaxValidator::with_value(10.0);

        assert!(
            validator
                .validate(&Value::Number(Some(10.0)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn min_and_max_skip_non_numbers() {
        let ctx = Context::standalone("x");

        assert!(
            MinValidator::with_value(1.0)
                .validate(&Value::Text("not-a-number".into()), &ctx)
                .is_ok()
        );
        assert!(
            MaxValidator::with_value(1.0)
                .validate(&Value::Text("not-a-number".into()), &ctx)
                .is_ok()
        );
    }

    #[test]
    fn pattern_no_match_fails() {
        let validator = PatternValidator::new(r"[a-z]+").expect("valid pattern");

        assert_eq!(
            error_code(validator.validate(&Value::Text("123".into()), &Context::standalone("x"))),
            ErrorCode::Pattern("[a-z]+".into())
        );
    }

    #[test]
    fn pattern_match_passes() {
        let validator = PatternValidator::new(r"[a-z]+").expect("valid pattern");

        assert!(
            validator
                .validate(&Value::Text("abc".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn pattern_empty_skips() {
        let validator = PatternValidator::new(r"[a-z]+").expect("valid pattern");

        assert!(
            validator
                .validate(&Value::Text(String::new()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn invalid_regex_pattern_is_rejected() {
        assert!(matches!(
            PatternValidator::new("("),
            Err(PatternValidatorError::InvalidRegex { .. })
        ));
    }

    #[test]
    fn oversized_regex_pattern_is_rejected() {
        assert!(matches!(
            PatternValidator::new("a".repeat(1025)),
            Err(PatternValidatorError::PatternTooLong {
                max_bytes: 1024,
                actual_bytes: 1025,
            })
        ));
    }

    #[test]
    #[should_panic(expected = "PatternValidator::new_from_static: invalid regex pattern")]
    fn invalid_static_regex_pattern_panics() {
        drop(PatternValidator::new_from_static("("));
    }

    #[test]
    #[should_panic(
        expected = "PatternValidator::new_from_static: pattern exceeds 1024 bytes (got 1025 bytes)"
    )]
    fn oversized_static_regex_pattern_panics() {
        const OVERSIZED_PATTERN: &str = concat!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "a"
        );

        drop(PatternValidator::new_from_static(OVERSIZED_PATTERN));
    }

    #[test]
    fn email_invalid_fails() {
        let validator = EmailValidator::default();

        assert_eq!(
            error_code(
                validator.validate(&Value::Text("bad-email".into()), &Context::standalone("x"))
            ),
            ErrorCode::Email
        );
    }

    #[test]
    fn email_empty_local_part_fails() {
        let validator = EmailValidator::default();

        assert_eq!(
            error_code(validator.validate(
                &Value::Text("@example.com".into()),
                &Context::standalone("x")
            )),
            ErrorCode::Email
        );
    }

    #[test]
    fn email_valid_passes() {
        let validator = EmailValidator::default();

        assert!(
            validator
                .validate(
                    &Value::Text("user@example.com".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[test]
    fn email_empty_skips() {
        let validator = EmailValidator::default();

        assert!(
            validator
                .validate(&Value::Text(String::new()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[cfg(feature = "email-validation")]
    #[test]
    fn email_quoted_local_part_passes() {
        let validator = EmailValidator::default();

        assert!(
            validator
                .validate(
                    &Value::Text(r#""quoted local"@example.com"#.into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[cfg(feature = "email-validation")]
    #[test]
    fn email_domain_literal_passes() {
        let validator = EmailValidator::default();

        assert!(
            validator
                .validate(
                    &Value::Text("user@[127.0.0.1]".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[cfg(feature = "email-validation")]
    #[test]
    fn email_invalid_domain_fails_under_full_validation() {
        let validator = EmailValidator::default();

        assert_eq!(
            error_code(validator.validate(
                &Value::Text("user@.example.com".into()),
                &Context::standalone("x")
            )),
            ErrorCode::Email
        );
    }

    #[test]
    fn step_mismatch_fails() {
        let validator = StepValidator::new(0.5);

        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(0.3)), &Context::standalone("x"))),
            ErrorCode::Step(0.5)
        );
    }

    #[test]
    fn step_match_passes() {
        let validator = StepValidator::new(0.5);

        assert!(
            validator
                .validate(&Value::Number(Some(1.5)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn step_with_base() {
        let validator = StepValidator::new(2.0).with_base(1.0);

        assert!(
            validator
                .validate(&Value::Number(Some(5.0)), &Context::standalone("x"))
                .is_ok()
        );
        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(4.0)), &Context::standalone("x"))),
            ErrorCode::Step(2.0)
        );
    }

    #[test]
    fn step_rounding_boundary_passes() {
        let validator = StepValidator::new(0.1);

        assert!(
            validator
                .validate(&Value::Number(Some(-55.2)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn step_zero_fails() {
        let validator = StepValidator::new(0.0);

        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(1.0)), &Context::standalone("x"))),
            ErrorCode::Step(0.0)
        );
    }

    #[test]
    fn step_infinite_fails() {
        let validator = StepValidator::new(f64::INFINITY);

        assert_eq!(
            error_code(validator.validate(&Value::Number(Some(1.0)), &Context::standalone("x"))),
            ErrorCode::Step(f64::INFINITY)
        );
    }

    #[test]
    fn step_none_number_skips() {
        let validator = StepValidator::new(0.5);

        assert!(
            validator
                .validate(&Value::Number(None), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn step_non_number_skips() {
        let validator = StepValidator::new(0.5);

        assert!(
            validator
                .validate(
                    &Value::Text("not-a-number".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[test]
    fn url_invalid_fails() {
        let validator = UrlValidator::new();

        assert_eq!(
            error_code(
                validator.validate(&Value::Text("notaurl".into()), &Context::standalone("x"))
            ),
            ErrorCode::Url
        );
    }

    #[test]
    fn url_non_text_skips() {
        let validator = UrlValidator::new();

        assert!(
            validator
                .validate(&Value::Bool(true), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[cfg(feature = "url-validation")]
    #[test]
    fn url_mailto_scheme_passes() {
        let validator = UrlValidator::new();

        assert!(
            validator
                .validate(
                    &Value::Text("mailto:user@example.com".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[cfg(feature = "url-validation")]
    #[test]
    fn url_invalid_host_fails_under_full_validation() {
        let validator = UrlValidator::new();

        assert_eq!(
            error_code(validator.validate(
                &Value::Text("https://exa mple.com".into()),
                &Context::standalone("x")
            )),
            ErrorCode::Url
        );
    }

    #[test]
    fn url_valid_passes() {
        let validator = UrlValidator::new();

        assert!(
            validator
                .validate(
                    &Value::Text("https://example.com".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );
    }

    #[test]
    fn url_empty_skips() {
        let validator = UrlValidator::new();

        assert!(
            validator
                .validate(&Value::Text(String::new()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn fn_validator_custom_logic() {
        let validator = FnValidator::new(|value, _ctx| {
            if value.as_text() == Some("ok") {
                Ok(())
            } else {
                Err(Errors(vec![Error::custom("custom", "custom failure")]))
            }
        });

        assert!(
            validator
                .validate(&Value::Text("ok".into()), &Context::standalone("x"))
                .is_ok()
        );
        assert_eq!(
            error_code(validator.validate(&Value::Text("nope".into()), &Context::standalone("x"))),
            ErrorCode::Custom("custom".into())
        );
    }

    #[test]
    fn fn_validator_boxed() {
        let validator = FnValidator::new(|_value, _ctx| Ok(())).boxed();

        assert!(
            validator
                .validate(&Value::Text("anything".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn custom_message_overrides_default() {
        let ctx = Context::standalone("x");

        let required_result = RequiredValidator::default()
            .with_message("required override")
            .validate(&Value::Text(String::new()), &ctx);

        let min_length_result = MinLengthValidator::with_length(3)
            .with_message("min override")
            .validate(&Value::Text("hi".into()), &ctx);

        let max_length_result = MaxLengthValidator::with_length(1)
            .with_message("max override")
            .validate(&Value::Text("hi".into()), &ctx);

        let min_result = MinValidator::with_value(10.0)
            .with_message("min number override")
            .validate(&Value::Number(Some(9.0)), &ctx);

        let max_result = MaxValidator::with_value(1.0)
            .with_message("max number override")
            .validate(&Value::Number(Some(2.0)), &ctx);

        let pattern_result = PatternValidator::new(r"[a-z]+")
            .expect("valid pattern")
            .with_message("pattern override")
            .validate(&Value::Text("123".into()), &ctx);

        let email_result = EmailValidator::default()
            .with_message("email override")
            .validate(&Value::Text("invalid".into()), &ctx);

        let step_result = StepValidator::new(2.0)
            .with_message("step override")
            .validate(&Value::Number(Some(3.0)), &ctx);

        let url_result = UrlValidator::new()
            .with_message("url override")
            .validate(&Value::Text("invalid".into()), &ctx);

        let required_message = error_message(required_result.clone());

        let min_length_message = error_message(min_length_result.clone());

        let max_length_message = error_message(max_length_result.clone());

        let min_message = error_message(min_result.clone());

        let max_message = error_message(max_result.clone());

        let pattern_message = error_message(pattern_result.clone());

        let email_message = error_message(email_result.clone());

        let step_message = error_message(step_result.clone());

        let url_message = error_message(url_result.clone());

        assert_eq!(error_code(required_result), ErrorCode::Required);
        assert_eq!(error_code(min_length_result), ErrorCode::MinLength(3));
        assert_eq!(error_code(max_length_result), ErrorCode::MaxLength(1));
        assert_eq!(error_code(min_result), ErrorCode::Min(10.0));
        assert_eq!(error_code(max_result), ErrorCode::Max(1.0));
        assert_eq!(
            error_code(pattern_result),
            ErrorCode::Pattern("[a-z]+".into())
        );
        assert_eq!(error_code(email_result), ErrorCode::Email);
        assert_eq!(error_code(step_result), ErrorCode::Step(2.0));
        assert_eq!(error_code(url_result), ErrorCode::Url);

        assert_eq!(required_message, "required override");
        assert_eq!(min_length_message, "min override");
        assert_eq!(max_length_message, "max override");
        assert_eq!(min_message, "min number override");
        assert_eq!(max_message, "max number override");
        assert_eq!(pattern_message, "pattern override");
        assert_eq!(email_message, "email override");
        assert_eq!(step_message, "step override");
        assert_eq!(url_message, "url override");
    }

    #[test]
    fn defaults_use_english_locale_fallback() {
        let message = error_message(
            RequiredValidator::default()
                .validate(&Value::Text(String::new()), &Context::standalone("x")),
        );

        assert_eq!(message, "This field is required");
    }

    #[test]
    fn defaults_use_context_locale_when_present() {
        let locale = locales::en();

        let ctx = Context {
            field_name: "x",
            form_values: &std::collections::BTreeMap::new(),
            locale: Some(&locale),
        };

        let message =
            error_message(EmailValidator::default().validate(&Value::Text("bad".into()), &ctx));

        assert_eq!(message, "Must be a valid email address");
    }

    #[test]
    fn required_nonempty_file_passes() {
        let validator = RequiredValidator::default();

        let files = vec![FileRef {
            name: "file.txt".into(),
            size: 1,
            mime_type: "text/plain".into(),
        }];

        assert!(
            validator
                .validate(&Value::File(files), &Context::standalone("x"))
                .is_ok()
        );
    }
}
