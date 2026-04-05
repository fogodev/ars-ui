//! Synchronous validation trait and context.
//!
//! Defines the [`Validator`] trait for synchronous field validation,
//! [`BoxedValidator`] for type-erased storage, and [`Context`]
//! which provides cross-field access during validation.

use std::collections::BTreeMap;

use super::result::Result;
use crate::field::Value;

/// Context available to validators during validation.
///
/// `Clone` is derived because this struct holds only references. However,
/// cloning only copies the borrows — it does not extend their lifetimes.
/// If you need an owned copy that outlives the borrow scope (e.g., for
/// async validation), use [`snapshot()`](Self::snapshot) instead.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// The name of the field being validated.
    pub field_name: &'a str,

    /// All current form values (for cross-field validation).
    pub form_values: &'a BTreeMap<String, Value>,

    /// The current locale (for locale-aware messages).
    pub locale: Option<&'a ars_i18n::Locale>,
}

/// An owned version of [`Context`] that can outlive the borrow scope.
///
/// Use [`Context::snapshot()`] to create one — e.g., for async
/// validation where the future must own its context.
#[derive(Clone, Debug)]
pub struct OwnedContext {
    /// The name of the field being validated.
    pub field_name: String,
    /// All current form values.
    pub form_values: BTreeMap<String, Value>,
    /// The current locale.
    pub locale: Option<ars_i18n::Locale>,
}

impl OwnedContext {
    /// Convert back to a borrowed [`Context<'_>`] for passing to
    /// [`AsyncValidator::validate_async()`](crate::AsyncValidator::validate_async)
    /// and similar APIs.
    pub fn as_ref(&self) -> Context<'_> {
        Context {
            field_name: &self.field_name,
            form_values: &self.form_values,
            locale: self.locale.as_ref(),
        }
    }
}

impl<'a> Context<'a> {
    /// Create an owned snapshot of this context, suitable for sending into
    /// async validation futures that outlive the borrow scope.
    pub fn snapshot(&self) -> OwnedContext {
        OwnedContext {
            field_name: self.field_name.to_owned(),
            form_values: self.form_values.clone(),
            locale: self.locale.cloned(),
        }
    }

    /// Create a standalone validation context for calling validators outside
    /// of a form. Uses an empty `form_values` map and no locale, which is
    /// sufficient for single-field validation without cross-field dependencies.
    pub fn standalone(field_name: &'a str) -> Self {
        use std::sync::LazyLock;
        static EMPTY_MAP: LazyLock<BTreeMap<String, Value>> = LazyLock::new(BTreeMap::new);
        Self {
            field_name,
            form_values: &EMPTY_MAP,
            locale: None,
        }
    }
}

/// A synchronous field validator.
///
/// **Platform-dependent bounds:** On native targets, `Validator` requires
/// `Send + Sync` so that [`BoxedValidator`] (`Arc<dyn Validator + Send + Sync>`)
/// can wrap any implementor. On WASM (single-threaded), no extra bounds are
/// required. All built-in validators satisfy both bound sets.
#[cfg(not(target_arch = "wasm32"))]
pub trait Validator: Send + Sync {
    /// Validates the given value and returns a result with any errors found.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when the value fails validation.
    fn validate(&self, value: &Value, ctx: &Context) -> Result;
}

/// A synchronous field validator (WASM variant without `Send + Sync`).
#[cfg(target_arch = "wasm32")]
pub trait Validator {
    /// Validates the given value and returns a result with any errors found.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when the value fails validation.
    fn validate(&self, value: &Value, ctx: &Context) -> Result;
}

/// A type-erased synchronous validator.
///
/// Uses `Arc` instead of `Box` for cheap cloning across reactive signals.
/// On WASM targets, uses `Rc` to avoid unnecessary atomic overhead.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedValidator = std::sync::Arc<dyn Validator + Send + Sync>;

/// A type-erased synchronous validator (WASM variant using `Rc`).
#[cfg(target_arch = "wasm32")]
pub type BoxedValidator = std::rc::Rc<dyn Validator>;

/// Helper to wrap a [`Validator`] into the correct smart pointer for the platform.
pub fn boxed_validator(v: impl Validator + 'static) -> BoxedValidator {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::sync::Arc::new(v)
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::rc::Rc::new(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{Error, ErrorCode, Errors};

    struct RequiredValidator;

    impl Validator for RequiredValidator {
        fn validate(&self, value: &Value, _ctx: &Context) -> Result {
            if let Some(text) = value.as_text() {
                if text.trim().is_empty() {
                    return Err(Errors(vec![Error {
                        code: ErrorCode::Required,
                        message: "Value is required".to_string(),
                    }]));
                }
            }
            Ok(())
        }
    }

    #[test]
    fn validator_trait_can_be_implemented() {
        let validator = RequiredValidator;
        let ctx = Context::standalone("test");

        assert!(
            validator
                .validate(&Value::Text(String::new()), &ctx)
                .is_err()
        );
        assert!(
            validator
                .validate(&Value::Text("hello".to_string()), &ctx)
                .is_ok()
        );
    }

    #[test]
    fn standalone_context_has_empty_values() {
        let ctx = Context::standalone("email");
        assert!(ctx.form_values.is_empty());
        assert!(ctx.locale.is_none());
        assert_eq!(ctx.field_name, "email");
    }

    #[test]
    fn snapshot_produces_owned_context() {
        let values: BTreeMap<String, Value> =
            [("name".to_string(), Value::Text("Alice".to_string()))]
                .into_iter()
                .collect();
        let locale = ars_i18n::Locale::new("en-US");
        let ctx = Context {
            field_name: "name",
            form_values: &values,
            locale: Some(&locale),
        };

        let owned = ctx.snapshot();
        assert_eq!(owned.field_name, "name");
        assert_eq!(owned.form_values.len(), 1);
        assert_eq!(
            owned.locale.as_ref().map(ars_i18n::Locale::as_str),
            Some("en-US")
        );
    }

    #[test]
    fn owned_context_as_ref_round_trips() {
        let owned = OwnedContext {
            field_name: "email".to_string(),
            form_values: BTreeMap::new(),
            locale: Some(ars_i18n::Locale::new("fr-FR")),
        };
        let borrowed = owned.as_ref();
        assert_eq!(borrowed.field_name, "email");
        assert!(borrowed.form_values.is_empty());
        assert_eq!(borrowed.locale.map(ars_i18n::Locale::as_str), Some("fr-FR"));
    }

    #[test]
    fn boxed_validator_wraps_correctly() {
        let boxed = boxed_validator(RequiredValidator);
        let ctx = Context::standalone("test");
        assert!(boxed.validate(&Value::Text(String::new()), &ctx).is_err());
        assert!(boxed.validate(&Value::Text("ok".to_string()), &ctx).is_ok());
    }
}
