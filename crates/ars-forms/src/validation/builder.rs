//! Fluent validator builder and chain executor.
//!
//! [`ValidatorsBuilder`] provides chainable methods that accumulate
//! [`BoxedValidator`] entries. Calling [`.build()`](ValidatorsBuilder::build)
//! produces a [`ChainValidator`] that runs all validators and collects every
//! error; [`.build_first_fail()`](ValidatorsBuilder::build_first_fail) stops
//! at the first error instead.
//!
//! The [`Validators`] type alias is a convenience re-export of
//! [`ValidatorsBuilder`] for ergonomic use in end-user code.

use std::fmt::{self, Debug};

use super::{
    BoxedValidator, Context, Errors, Result, Validator, boxed_validator,
    built_in::{
        EmailValidator, FnValidator, MaxLengthValidator, MaxValidator, MinLengthValidator,
        MinValidator, PatternValidator, PatternValidatorError, RequiredValidator, StepValidator,
        UrlValidator,
    },
};
use crate::field::Value;

/// Runs multiple validators and combines their results.
///
/// When `stop_on_first` is `false`, all validators run and every error is
/// collected. When `true`, iteration stops after the first validator that
/// returns an error.
pub struct ChainValidator {
    /// The validators to run in order.
    validators: Vec<BoxedValidator>,

    /// Whether to stop after the first validator that returns an error.
    stop_on_first: bool,
}

impl Debug for ChainValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainValidator")
            .field("validator_count", &self.validators.len())
            .field("stop_on_first", &self.stop_on_first)
            .finish()
    }
}

impl Validator for ChainValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let mut all_errors = Errors::new();

        for validator in &self.validators {
            if let Err(mut errors) = validator.validate(value, ctx) {
                if self.stop_on_first {
                    // Return only the very first error, even when a single
                    // validator produces multiple.
                    errors.0.truncate(1);
                    return Err(errors);
                }

                all_errors.0.extend(errors.0);
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

impl ChainValidator {
    /// Wraps this chain in the standard shared pointer type for storage
    /// in form field state.
    #[must_use]
    pub fn boxed(self) -> BoxedValidator {
        boxed_validator(self)
    }
}

/// Build a validator chain fluently.
///
/// # Example
///
/// ```rust
/// # use ars_forms::validation::{Validators, BoxedValidator};
/// let validator: BoxedValidator = Validators::new()
///     .required()
///     .min_length(3)
///     .max_length(50)
///     .pattern_static(r"^[a-zA-Z0-9_]+$")
///     .build()
///     .boxed();
/// ```
pub struct ValidatorsBuilder {
    /// Accumulated validators to run when built.
    validators: Vec<BoxedValidator>,
}

impl Debug for ValidatorsBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidatorsBuilder")
            .field("validator_count", &self.validators.len())
            .finish()
    }
}

impl ValidatorsBuilder {
    /// Creates an empty builder with no validators.
    #[must_use]
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    /// Adds an arbitrary validator to the chain.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "spec §3.3 names this method `add`; it is not related to `std::ops::Add`"
    )]
    pub fn add(mut self, v: impl Validator + 'static) -> Self {
        self.validators.push(boxed_validator(v));
        self
    }

    /// Adds a [`RequiredValidator`] with the default localized message.
    #[must_use]
    pub fn required(self) -> Self {
        self.add(RequiredValidator::default())
    }

    /// Adds a [`RequiredValidator`] with a custom error message.
    #[must_use]
    pub fn required_msg(self, msg: impl Into<String>) -> Self {
        self.add(RequiredValidator::default().with_message(msg))
    }

    /// Adds a [`MinLengthValidator`] for the given minimum character count.
    #[must_use]
    pub fn min_length(self, n: usize) -> Self {
        self.add(MinLengthValidator::with_length(n))
    }

    /// Adds a [`MaxLengthValidator`] for the given maximum character count.
    #[must_use]
    pub fn max_length(self, n: usize) -> Self {
        self.add(MaxLengthValidator::with_length(n))
    }

    /// Adds a [`MinValidator`] for the given inclusive minimum numeric value.
    #[must_use]
    pub fn min(self, n: f64) -> Self {
        self.add(MinValidator::with_value(n))
    }

    /// Adds a [`MaxValidator`] for the given inclusive maximum numeric value.
    #[must_use]
    pub fn max(self, n: f64) -> Self {
        self.add(MaxValidator::with_value(n))
    }

    /// Adds a [`PatternValidator`] from a compile-time pattern string.
    ///
    /// # Panics
    ///
    /// Panics if `regex` exceeds 1024 bytes or is not valid regex syntax.
    #[must_use]
    pub fn pattern_static(self, regex: &'static str) -> Self {
        self.add(PatternValidator::new_from_static(regex))
    }

    /// Adds a [`PatternValidator`], returning an error if the pattern is
    /// invalid or too long.
    ///
    /// # Errors
    ///
    /// Returns [`PatternValidatorError`] when the pattern cannot be compiled.
    pub fn try_pattern(
        self,
        regex: impl Into<String>,
    ) -> std::result::Result<Self, PatternValidatorError> {
        Ok(self.add(PatternValidator::new(regex)?))
    }

    /// Adds an [`EmailValidator`] with the default localized message.
    #[must_use]
    pub fn email(self) -> Self {
        self.add(EmailValidator::default())
    }

    /// Adds a [`StepValidator`] with base `0.0`.
    #[must_use]
    pub fn step(self, step: f64) -> Self {
        self.add(StepValidator::new(step))
    }

    /// Adds a [`StepValidator`] measured from a custom base value.
    #[must_use]
    pub fn step_with_base(self, step: f64, base: f64) -> Self {
        self.add(StepValidator::new(step).with_base(base))
    }

    /// Adds a [`UrlValidator`] with the default localized message.
    #[must_use]
    pub fn url(self) -> Self {
        self.add(UrlValidator::new())
    }

    /// Adds a closure-backed [`FnValidator`] for custom synchronous logic.
    #[must_use]
    pub fn custom<F>(self, f: F) -> Self
    where
        F: Fn(&Value, &Context) -> Result + Send + Sync + 'static,
    {
        self.add(FnValidator::new(f))
    }

    /// Runs all validators and collects every error.
    #[must_use]
    pub fn build(self) -> ChainValidator {
        ChainValidator {
            validators: self.validators,
            stop_on_first: false,
        }
    }

    /// Runs validators and stops at the first error.
    #[must_use]
    pub fn build_first_fail(self) -> ChainValidator {
        ChainValidator {
            validators: self.validators,
            stop_on_first: true,
        }
    }
}

impl Default for ValidatorsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience alias for [`ValidatorsBuilder`].
pub type Validators = ValidatorsBuilder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{Error, ErrorCode, Errors};

    #[test]
    fn chain_collects_all_errors() {
        let chain = ValidatorsBuilder::new().required().min_length(3).build();

        let result = chain.validate(&Value::Text(String::new()), &Context::standalone("name"));

        let errors = result.expect_err("both validators should fail");

        assert!(
            errors.has_code(&ErrorCode::Required),
            "should contain Required error"
        );
        assert!(
            errors.has_code(&ErrorCode::MinLength(3)),
            "should contain MinLength error"
        );
        assert_eq!(errors.len(), 2, "should collect exactly 2 errors");
    }

    #[test]
    fn chain_first_fail_stops_early() {
        let chain = ValidatorsBuilder::new()
            .required()
            .min_length(3)
            .build_first_fail();

        let result = chain.validate(&Value::Text(String::new()), &Context::standalone("name"));

        let errors = result.expect_err("first validator should fail");

        assert_eq!(errors.len(), 1, "should stop after first error");
        assert!(
            errors.has_code(&ErrorCode::Required),
            "first error should be Required"
        );
    }

    #[test]
    fn first_fail_returns_single_error_from_multi_error_validator() {
        // A custom validator that returns two errors at once.
        let chain = Validators::new()
            .custom(|_value, _ctx| {
                Err(Errors(vec![
                    Error::custom("first", "first error"),
                    Error::custom("second", "second error"),
                ]))
            })
            .build_first_fail();

        let result = chain.validate(&Value::Text("x".into()), &Context::standalone("f"));
        let errors = result.expect_err("should fail");

        assert_eq!(
            errors.len(),
            1,
            "first-fail must return exactly one error even when a validator produces multiple"
        );
        assert!(errors.has_code(&ErrorCode::Custom("first".into())));
    }

    #[test]
    fn builder_required_email_chain() {
        let chain = Validators::new().required().email().build();

        // Valid email passes both
        assert!(
            chain
                .validate(
                    &Value::Text("user@example.com".into()),
                    &Context::standalone("email")
                )
                .is_ok()
        );

        // Invalid email fails email validator
        let result = chain.validate(&Value::Text("bad".into()), &Context::standalone("email"));

        let errors = result.expect_err("invalid email should fail");

        assert!(errors.has_code(&ErrorCode::Email));
    }

    #[test]
    fn builder_first_fail_boxed() {
        let boxed: BoxedValidator = Validators::new()
            .required()
            .email()
            .build_first_fail()
            .boxed();

        // Empty value triggers required (first-fail stops there)
        let result = boxed.validate(&Value::Text(String::new()), &Context::standalone("email"));

        let errors = result.expect_err("empty should fail required");

        assert_eq!(errors.len(), 1);
        assert!(errors.has_code(&ErrorCode::Required));
    }

    #[test]
    fn builder_custom_closure() {
        let chain = Validators::new()
            .custom(|value, _ctx| {
                if value.as_text() == Some("magic") {
                    Ok(())
                } else {
                    Err(Errors(vec![Error::custom("magic", "not magic")]))
                }
            })
            .build();

        assert!(
            chain
                .validate(&Value::Text("magic".into()), &Context::standalone("spell"))
                .is_ok()
        );

        let result = chain.validate(
            &Value::Text("mundane".into()),
            &Context::standalone("spell"),
        );

        let errors = result.expect_err("non-magic should fail");

        assert!(errors.has_code(&ErrorCode::Custom("magic".into())));
    }

    #[test]
    fn builder_empty_passes() {
        let chain = Validators::new().build();

        assert!(
            chain
                .validate(
                    &Value::Text("anything".into()),
                    &Context::standalone("field")
                )
                .is_ok()
        );
        assert!(
            chain
                .validate(&Value::Text(String::new()), &Context::standalone("field"))
                .is_ok()
        );
        assert!(
            chain
                .validate(&Value::Number(None), &Context::standalone("field"))
                .is_ok()
        );
    }

    #[test]
    fn validators_type_alias() {
        // Verify the type alias compiles and works identically
        let _builder: Validators = Validators::new();

        let chain = Validators::new().required().build();

        assert!(
            chain
                .validate(&Value::Text("ok".into()), &Context::standalone("field"))
                .is_ok()
        );
    }

    #[test]
    fn builder_required_msg() {
        let chain = Validators::new().required_msg("custom required").build();

        let result = chain.validate(&Value::Text(String::new()), &Context::standalone("x"));
        let errors = result.expect_err("empty should fail");

        assert!(errors.has_code(&ErrorCode::Required));
        assert_eq!(errors.0[0].message, "custom required");
    }

    #[test]
    fn builder_max_length() {
        let chain = Validators::new().max_length(5).build();

        assert!(
            chain
                .validate(&Value::Text("hello".into()), &Context::standalone("x"))
                .is_ok()
        );

        let result = chain.validate(&Value::Text("toolong".into()), &Context::standalone("x"));

        assert!(
            result
                .expect_err("should fail")
                .has_code(&ErrorCode::MaxLength(5))
        );
    }

    #[test]
    fn builder_min_max_numeric() {
        let chain = Validators::new().min(1.0).max(10.0).build();

        assert!(
            chain
                .validate(&Value::Number(Some(5.0)), &Context::standalone("x"))
                .is_ok()
        );

        let below = chain.validate(&Value::Number(Some(0.5)), &Context::standalone("x"));

        assert!(
            below
                .expect_err("should fail")
                .has_code(&ErrorCode::Min(1.0))
        );

        let above = chain.validate(&Value::Number(Some(11.0)), &Context::standalone("x"));

        assert!(
            above
                .expect_err("should fail")
                .has_code(&ErrorCode::Max(10.0))
        );
    }

    #[test]
    fn builder_pattern_static() {
        let chain = Validators::new().pattern_static(r"^[a-z]+$").build();

        assert!(
            chain
                .validate(&Value::Text("abc".into()), &Context::standalone("x"))
                .is_ok()
        );

        let result = chain.validate(&Value::Text("123".into()), &Context::standalone("x"));

        assert!(
            result
                .expect_err("should fail")
                .has_code(&ErrorCode::Pattern("^[a-z]+$".into()))
        );
    }

    #[test]
    fn builder_try_pattern() {
        let chain = Validators::new()
            .try_pattern(r"^[a-z]+$")
            .expect("valid pattern")
            .build();

        assert!(
            chain
                .validate(&Value::Text("abc".into()), &Context::standalone("x"))
                .is_ok()
        );

        assert!(Validators::new().try_pattern("(").is_err());
    }

    #[test]
    fn builder_step_and_step_with_base() {
        let chain = Validators::new().step(0.5).build();

        assert!(
            chain
                .validate(&Value::Number(Some(1.5)), &Context::standalone("x"))
                .is_ok()
        );

        let result = chain.validate(&Value::Number(Some(0.3)), &Context::standalone("x"));

        assert!(
            result
                .expect_err("should fail")
                .has_code(&ErrorCode::Step(0.5))
        );

        let chain_base = Validators::new().step_with_base(2.0, 1.0).build();

        assert!(
            chain_base
                .validate(&Value::Number(Some(5.0)), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn builder_url() {
        let chain = Validators::new().url().build();

        assert!(
            chain
                .validate(
                    &Value::Text("https://example.com".into()),
                    &Context::standalone("x")
                )
                .is_ok()
        );

        let result = chain.validate(&Value::Text("notaurl".into()), &Context::standalone("x"));

        assert!(result.expect_err("should fail").has_code(&ErrorCode::Url));
    }

    #[test]
    fn builder_default() {
        let builder = ValidatorsBuilder::default();
        let chain = builder.build();

        assert!(
            chain
                .validate(&Value::Text("ok".into()), &Context::standalone("x"))
                .is_ok()
        );
    }

    #[test]
    fn chain_validator_debug() {
        let chain = Validators::new().required().email().build();

        let debug = format!("{chain:?}");

        assert!(debug.contains("ChainValidator"));
        assert!(debug.contains("validator_count: 2"));
        assert!(debug.contains("stop_on_first: false"));
    }

    #[test]
    fn validators_builder_debug() {
        let builder = Validators::new().required();

        let debug = format!("{builder:?}");

        assert!(debug.contains("ValidatorsBuilder"));
        assert!(debug.contains("validator_count: 1"));
    }
}
