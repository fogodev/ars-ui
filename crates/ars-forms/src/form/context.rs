//! Form context, validation mode, and cross-field validation.
//!
//! [`Context`] is the central state for a form — it tracks field
//! registration, validation results, dirty/touched state, and server errors.
//! [`Mode`] controls when field-level validation triggers.
//! [`CrossFieldValidator`] and [`AnyValidator`] provide advanced validator
//! composition. [`Data`] is the collected field data passed to submit
//! handlers.

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use hashbrown::DefaultHashBuilder;
use indexmap::IndexMap;

type OrderedIndexMap<K, V> = IndexMap<K, V, DefaultHashBuilder>;

use crate::{
    field::{State, Value},
    validation::{
        BoxedAsyncValidator, BoxedValidator, Context as ValContext, Error, Errors, Result,
        ResultExt, Validator,
    },
};

/// The central state for a form.
///
/// `Context` is a plain data structure, not a `Machine` implementor. It
/// holds mutable form state (field registry, validation results, dirty/touched
/// tracking). `ars_components::utility::form_submit::Machine` embeds
/// `Context` in its `Context` type and drives the submission lifecycle as a
/// proper state machine.
#[derive(Clone)]
pub struct Context {
    /// State of each registered field.
    ///
    /// Uses [`IndexMap`] (insertion-ordered) so that "focus first invalid field"
    /// iterates fields in DOM registration order, not alphabetical order.
    pub fields: OrderedIndexMap<String, State>,

    /// Whether the form is currently being submitted.
    pub is_submitting: bool,

    /// Whether the form has been submitted at least once.
    pub is_submitted: bool,

    /// Externally injected server-side errors (e.g., from API response).
    pub server_errors: BTreeMap<String, Vec<String>>,

    /// The current locale, injected from `ArsProvider` on construction.
    ///
    /// Set `locale` to enable localized validation messages. Defaults to
    /// English when `None`.
    pub locale: Option<ars_i18n::Locale>,

    /// Validation trigger mode.
    pub validation_mode: Mode,

    /// Registered validators per field.
    // BTreeMap is used intentionally: validators are accessed by field name
    // (keyed lookup), not iterated in order. Registration order is maintained
    // by `fields: IndexMap`.
    #[doc(hidden)]
    validators: BTreeMap<String, BoxedValidator>,

    /// Async validators per field.
    #[doc(hidden)]
    async_validators: BTreeMap<String, BoxedAsyncValidator>,

    /// Registry of cross-field validators indexed by the field they validate.
    cross_field_registry: BTreeMap<String, Vec<CrossFieldValidator>>,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("fields", &self.fields)
            .field("is_submitting", &self.is_submitting)
            .field("is_submitted", &self.is_submitted)
            .field("server_errors", &self.server_errors)
            .field("locale", &self.locale)
            .field("validation_mode", &self.validation_mode)
            .field(
                "validators",
                &format_args!("<{} validators>", self.validators.len()),
            )
            .field(
                "async_validators",
                &format_args!("<{} async_validators>", self.async_validators.len()),
            )
            .field(
                "cross_field_registry",
                &format_args!("<{} cross-field entries>", self.cross_field_registry.len()),
            )
            .finish()
    }
}

/// When field-level validation is triggered.
///
/// **Note:** Submit-time validation is always performed by
/// [`Context::submit()`] regardless of these settings. There is no
/// `on_submit` flag because skipping validation at submit time is never
/// valid — `submit()` unconditionally calls `validate_all()` before
/// dispatching. These flags only control *field-level* validation timing
/// (blur, change, input).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Mode {
    /// Validate on input change.
    pub on_change: bool,

    /// Validate on field blur.
    pub on_blur: bool,

    /// Validate on every keystroke (before debounce).
    pub on_input: bool,

    /// If already invalid, re-validate on change (React Hook Form pattern).
    pub revalidate_on_change: bool,
}

impl Mode {
    /// Validate on blur, re-validate on change if already invalid.
    #[must_use]
    pub fn on_blur_revalidate() -> Self {
        Self {
            on_blur: true,
            revalidate_on_change: true,
            ..Default::default()
        }
    }

    /// Validate on every change.
    #[must_use]
    pub fn on_change() -> Self {
        Self {
            on_change: true,
            ..Default::default()
        }
    }

    /// Only validate on submit (no field-level validation).
    #[must_use]
    pub fn on_submit() -> Self {
        Self::default()
    }
}

/// Passes if ANY inner validator returns `Ok(())`. Collects errors only if all fail.
pub struct AnyValidator {
    /// The validators to try (at least one must pass).
    pub validators: Vec<BoxedValidator>,
}

impl Debug for AnyValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyValidator")
            .field(
                "validators",
                &format!("<{} validators>", self.validators.len()),
            )
            .finish()
    }
}

impl AnyValidator {
    /// Build an OR-combination validator: passes if ANY inner validator returns `Ok(())`.
    #[must_use]
    pub fn new(validators: Vec<BoxedValidator>) -> Self {
        Self { validators }
    }

    /// Wrap into a [`BoxedValidator`].
    pub fn boxed(self) -> BoxedValidator {
        crate::validation::boxed_validator(self)
    }
}

impl Validator for AnyValidator {
    fn validate(&self, value: &Value, ctx: &ValContext) -> Result {
        let mut all_errors = Errors::new();

        for v in &self.validators {
            if let Err(errs) = v.validate(value, ctx) {
                all_errors.0.extend(errs.0);
            } else {
                return Ok(());
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

/// Validates a field using values from other fields in the form.
///
/// When any field listed in `depends_on` changes (via `on_change`),
/// `Context` automatically re-validates the field this validator is
/// registered on.
/// A type-erased cross-field validation function.
///
/// Uses [`Arc`](std::sync::Arc) on all targets for shared ownership.
type CrossFieldValidateFn = Arc<dyn Fn(&Value, &ValContext) -> Result + Send + Sync>;

/// Validates a field using values from other fields in the form.
///
/// When any field listed in `depends_on` changes (via `on_change`),
/// [`Context`] automatically re-validates the field this validator is
/// registered on.
#[derive(Clone)]
pub struct CrossFieldValidator {
    /// Names of fields this validator reads from. When any field in this
    /// list changes, `Context` re-validates the owning field.
    pub depends_on: Vec<String>,

    /// The validation function.
    pub validate_fn: CrossFieldValidateFn,
}

impl Debug for CrossFieldValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CrossFieldValidator")
            .field("depends_on", &self.depends_on)
            .field("validate_fn", &"<fn>")
            .finish()
    }
}

impl Validator for CrossFieldValidator {
    fn validate(&self, value: &Value, ctx: &ValContext) -> Result {
        (self.validate_fn)(value, ctx)
    }
}

impl Context {
    /// Creates a new form context with the given validation mode.
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            fields: OrderedIndexMap::default(),
            is_submitting: false,
            is_submitted: false,
            server_errors: BTreeMap::new(),
            locale: None,
            validation_mode: mode,
            validators: BTreeMap::new(),
            async_validators: BTreeMap::new(),
            cross_field_registry: BTreeMap::new(),
        }
    }

    /// Register a field with optional sync and async validators.
    ///
    /// Re-registration replaces both validator types. To keep an existing
    /// async validator when re-registering, pass the same `async_validator` again.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        initial: Value,
        validator: Option<BoxedValidator>,
        async_validator: Option<BoxedAsyncValidator>,
    ) {
        let name = name.into();

        self.fields.insert(name.clone(), State::new(initial));

        // Remove stale validators before inserting new ones, so re-registration
        // with validator=None properly clears a previously registered validator.
        self.validators.remove(&name);

        self.async_validators.remove(&name);

        self.cross_field_registry.remove(&name);

        if let Some(v) = validator {
            self.validators.insert(name.clone(), v);
        }

        if let Some(v) = async_validator {
            self.async_validators.insert(name, v);
        }
    }

    /// Remove a field and its validators from the form.
    pub fn deregister(&mut self, name: &str) {
        self.fields.shift_remove(name);
        self.validators.remove(name);
        self.async_validators.remove(name);
        self.cross_field_registry.remove(name);
    }

    /// Register a cross-field validator for the given field.
    ///
    /// When any field listed in `validator.depends_on` changes, `field_name`
    /// is re-validated.
    pub fn register_cross_field_validator(
        &mut self,
        field_name: &str,
        validator: CrossFieldValidator,
    ) {
        self.cross_field_registry
            .entry(field_name.to_string())
            .or_default()
            .push(validator);
    }

    /// Called when a field's value changes.
    pub fn on_change(&mut self, name: &str, value: Value) {
        // Extract whether we need to validate before dropping the mutable borrow.
        let should_validate = if let Some(field) = self.fields.get_mut(name) {
            field.validation_generation += 1;
            field.value = value;
            field.dirty = true;

            self.validation_mode.on_change
                || (self.validation_mode.revalidate_on_change && field.validation.is_err())
        } else {
            return;
        };

        // Clear server error when field changes.
        self.server_errors.remove(name);

        if should_validate {
            self.run_field_validation(name);
        } else if let Some(field) = self.fields.get_mut(name) {
            // Remove only server-sourced errors; preserve client-side validation errors.
            field.validation = field.validation.without_server_errors();
        }
    }

    /// Called when a field loses focus.
    pub fn on_blur(&mut self, name: &str) {
        if let Some(field) = self.fields.get_mut(name) {
            field.touched = true;
        }

        if self.validation_mode.on_blur {
            self.run_field_validation(name);
        }
    }

    /// Called on every keystroke/input event (before debounce).
    ///
    /// Only triggers validation when `validation_mode.on_input` is true.
    /// Unlike `on_change`, this does NOT mark the field as `dirty` — dirty
    /// tracking requires `on_change` (which fires on blur or commit).
    /// Clears server errors for the field.
    pub fn on_input(&mut self, name: &str, value: Value) {
        if let Some(field) = self.fields.get_mut(name) {
            field.validation_generation += 1;
            field.value = value;
        } else {
            return;
        }

        // Clear server errors when user modifies a field.
        self.server_errors.remove(name);

        if self.validation_mode.on_input {
            self.run_field_validation(name);
        } else if let Some(field) = self.fields.get_mut(name) {
            field.validation = field.validation.without_server_errors();
        }
    }

    /// Validate a single field synchronously, building a cross-field context
    /// by cloning all current values.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when the field fails validation.
    pub fn validate_field(&mut self, name: &str) -> Result {
        self.run_field_validation(name);

        self.fields
            .get(name)
            .map_or(Ok(()), |f| f.validation.clone())
    }

    /// Validate all fields. Returns `true` if the form is valid.
    pub fn validate_all(&mut self) -> bool {
        let names = self.fields.keys().cloned().collect::<Vec<_>>();

        let mut valid = true;

        for name in names {
            self.run_field_validation(&name);

            if self
                .fields
                .get(&name)
                .is_some_and(|f| f.validation.is_err())
            {
                valid = false;
            }
        }

        valid
    }

    /// Inject server-side errors returned from an API call.
    ///
    /// Old server errors are removed first, then new server errors are appended.
    /// Preserves client-side errors from `validate_all()`.
    pub fn set_server_errors(&mut self, errors: impl Into<BTreeMap<String, Vec<String>>>) {
        let errors = errors.into();

        for (name, messages) in &errors {
            if let Some(field) = self.fields.get_mut(name) {
                field.touched = true; // Show errors immediately

                let server_errs = messages.iter().map(Error::server).collect::<Vec<_>>();

                // Merge: keep existing client-side errors, replace server errors
                let mut client_errs = if let Err(Errors(errs)) = &field.validation {
                    errs.iter().filter(|e| !e.is_server()).cloned().collect()
                } else {
                    vec![]
                };

                client_errs.extend(server_errs);

                field.validation = Err(Errors(client_errs));
            }
        }

        self.server_errors = errors;
    }

    /// Restores all fields to their initial values and clears metadata
    /// (touched, dirty, validation state).
    pub fn reset(&mut self) {
        for field in self.fields.values_mut() {
            field.value = field.initial_value.clone();
            field.touched = false;
            field.dirty = false;
            field.validation = Ok(());
            field.validating = false;
            field.validation_generation += 1;
        }

        self.is_submitting = false;
        self.is_submitted = false;

        self.server_errors.clear();
    }

    /// Mark all fields as touched (for display of all errors after attempted submit).
    pub fn touch_all(&mut self) {
        for field in self.fields.values_mut() {
            field.touched = true;
        }
    }

    /// Internal: runs validation for a field and stores the result in
    /// `field.validation`. Does not return the result — callers that need
    /// the outcome read it from `self.fields` afterward.
    fn run_field_validation(&mut self, name: &str) {
        let Some(value) = self.fields.get(name).map(|f| f.value.clone()) else {
            return;
        };

        let all_values = self
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect();

        let ctx = ValContext {
            field_name: name,
            form_values: &all_values,
            locale: self.locale.as_ref(),
        };

        let result = if let Some(validator) = self.validators.get(name) {
            validator.validate(&value, &ctx)
        } else {
            Ok(())
        };

        // Run cross-field validators that depend on this field.
        for (target_name, validators) in &self.cross_field_registry {
            if target_name != name
                && validators
                    .iter()
                    .any(|cv| cv.depends_on.contains(&name.to_string()))
            {
                let Some(target_value) = self
                    .fields
                    .get(target_name.as_str())
                    .map(|f| f.value.clone())
                else {
                    continue;
                };

                let target_result = validators
                    .iter()
                    .filter(|cv| cv.depends_on.contains(&name.to_string()))
                    .map(|cv| cv.validate(&target_value, &ctx))
                    .reduce(ResultExt::merge)
                    .unwrap_or(Ok(()));

                if let Some(field) = self.fields.get_mut(target_name.as_str()) {
                    field.validation = target_result;
                }
            }
        }

        if let Some(field) = self.fields.get_mut(name) {
            field.validation = result;
        }
    }

    /// Performs direct synchronous validation and submission without the
    /// form-submit machine.
    ///
    /// **Note:** `is_submitting` is only meaningful for the duration of the
    /// synchronous `handler` call. For async submission, use
    /// `ars_components::utility::form_submit::Machine` (§8) or
    /// `ars_components::utility::form::Machine` (§14).
    ///
    /// `collect_form_data()` includes all fields regardless of disabled state.
    /// If disabled fields must be excluded, the caller should filter
    /// `Data.fields` before processing.
    ///
    /// # Errors
    ///
    /// Returns `Err(Errors)` when any field fails validation; the handler
    /// is not called.
    pub fn submit<F>(&mut self, handler: F) -> Result
    where
        F: FnOnce(Data),
    {
        self.is_submitted = true;

        self.touch_all();

        let is_valid = self.validate_all();

        if is_valid {
            self.is_submitting = true;

            let data = self.collect_form_data();

            handler(data);

            self.is_submitting = false;

            Ok(())
        } else {
            // Adapter: call first_invalid_field_id() and focus the result (see §9)
            Err(self.collect_all_errors())
        }
    }

    /// Get field state by name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&State> {
        self.fields.get(name)
    }

    /// Get field state mutably.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut State> {
        self.fields.get_mut(name)
    }

    /// Whether the form is currently valid (no fields have errors).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.fields.values().all(|f| f.validation.is_ok())
    }

    /// Whether any field has been modified.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.fields.values().any(|f| f.dirty)
    }

    /// Whether any registered fields have async validators pending.
    ///
    /// Used by `ars_components::utility::form_submit::Machine` to decide
    /// between sync and async validation paths.
    #[must_use]
    pub fn has_async_validators(&self) -> bool {
        !self.async_validators.is_empty()
    }

    /// Collect all async validators with their field names for batch execution.
    #[must_use]
    pub fn collect_async_validators(&self) -> Vec<(String, BoxedAsyncValidator)> {
        self.async_validators
            .iter()
            .map(|(name, v)| (name.clone(), BoxedAsyncValidator::clone(v)))
            .collect()
    }

    /// Register an async validator for a field.
    ///
    /// **Precondition:** The field identified by `name` should already be
    /// registered via [`register()`](Self::register). If the field does not
    /// exist, the validator is stored but never executed.
    pub fn register_async_validator(
        &mut self,
        name: impl Into<String>,
        validator: BoxedAsyncValidator,
    ) {
        let name = name.into();

        debug_assert!(
            self.fields.contains_key(&name),
            "register_async_validator: field '{name}' not registered — validator will never run"
        );

        self.async_validators.insert(name, validator);
    }

    fn collect_form_data(&self) -> Data {
        Data {
            fields: self
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), v.value.clone()))
                .collect(),
        }
    }

    fn collect_all_errors(&self) -> Errors {
        Errors(
            self.fields
                .values()
                .filter_map(|field| field.validation.errors().map(|e| e.0.clone()))
                .flatten()
                .collect(),
        )
    }
}

/// Collected form data passed to submit handler.
///
/// Uses [`IndexMap`] to preserve field registration order (matching
/// `Context.fields`).
///
/// `Default` produces an empty `Data` with no fields.
#[derive(Clone, Debug, Default)]
pub struct Data {
    /// The field values, keyed by field name.
    pub fields: OrderedIndexMap<String, Value>,
}

impl Data {
    /// Get a text value by field name.
    #[must_use]
    pub fn get_text(&self, name: &str) -> Option<&str> {
        self.fields.get(name).and_then(|v| v.as_text())
    }

    /// Get a numeric value by field name.
    #[must_use]
    pub fn get_number(&self, name: &str) -> Option<f64> {
        self.fields.get(name).and_then(Value::as_number)
    }

    /// Get a boolean value by field name.
    #[must_use]
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.fields.get(name).and_then(Value::as_bool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::ErrorCode;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn text(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    fn required_validator() -> BoxedValidator {
        struct RequiredV;

        impl Validator for RequiredV {
            fn validate(&self, value: &Value, _ctx: &ValContext) -> Result {
                if let Some(t) = value.as_text()
                    && t.trim().is_empty()
                {
                    return Err(Errors(vec![Error {
                        code: ErrorCode::Required,
                        message: "required".to_string(),
                    }]));
                }

                Ok(())
            }
        }

        crate::validation::boxed_validator(RequiredV)
    }

    fn password_match_validate(value: &Value, ctx_inner: &ValContext) -> Result {
        let password = ctx_inner
            .form_values
            .get("password")
            .and_then(Value::as_text);

        let confirm = value.as_text().unwrap_or_default();

        if password.is_some_and(|p| p == confirm) {
            Ok(())
        } else {
            Err(Errors(vec![Error::custom(
                "confirm_match",
                "Passwords do not match",
            )]))
        }
    }

    fn password_match_validator() -> CrossFieldValidator {
        CrossFieldValidator {
            depends_on: vec!["password".to_string()],
            validate_fn: Arc::new(password_match_validate),
        }
    }

    async fn always_ok_async(_text: String, _ctx: crate::validation::OwnedContext) -> Result {
        Ok(())
    }

    fn boxed_async_ok_validator() -> BoxedAsyncValidator {
        crate::validation::AsyncFnValidator::new(always_ok_async).boxed()
    }

    // ── Field registration order ────────────────────────────────────────

    #[test]
    fn register_preserves_insertion_order() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);
        ctx.register("email", text(""), None, None);
        ctx.register("age", Value::Number(None), None, None);

        let keys = ctx.fields.keys().collect::<Vec<_>>();

        assert_eq!(keys, vec!["name", "email", "age"]);
    }

    #[test]
    fn deregister_removes_field_and_validators() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), Some(required_validator()), None);

        assert!(ctx.field("email").is_some());

        ctx.deregister("email");

        assert!(ctx.field("email").is_none());
    }

    #[test]
    fn register_replaces_existing() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("old"), None, None);
        ctx.register("name", text("new"), None, None);

        let field = ctx.field("name").expect("field exists");

        assert_eq!(field.value, text("new"));
        assert_eq!(field.initial_value, text("new"));
    }

    #[test]
    fn field_returns_registered_state() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text("test@test.com"), None, None);

        assert!(ctx.field("email").is_some());
    }

    #[test]
    fn field_returns_none_for_unknown() {
        let ctx = Context::new(Mode::on_submit());

        assert!(ctx.field("missing").is_none());
    }

    // ── Server error injection and clearing ─────────────────────────────

    #[test]
    fn set_server_errors_marks_touched() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        assert!(!ctx.field("email").expect("exists").touched);

        ctx.set_server_errors([("email".to_string(), vec!["Already exists".to_string()])]);

        assert!(ctx.field("email").expect("exists").touched);
    }

    #[test]
    fn set_server_errors_shows_error() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        ctx.set_server_errors([("email".to_string(), vec!["Already exists".to_string()])]);

        let field = ctx.field("email").expect("exists");

        assert!(field.show_error());
        assert_eq!(field.error_message(), Some("Already exists"));
    }

    #[test]
    fn server_error_cleared_on_change() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        ctx.set_server_errors([("email".to_string(), vec!["Taken".to_string()])]);

        assert!(ctx.field("email").expect("exists").validation.is_err());

        // Change clears server error
        ctx.on_change("email", text("new@test.com"));

        let field = ctx.field("email").expect("exists");

        assert!(field.validation.is_ok());
    }

    #[test]
    fn set_server_errors_preserves_client_errors() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), Some(required_validator()), None);

        // First trigger client-side validation
        let _result = ctx.validate_field("email");

        assert!(ctx.field("email").expect("exists").validation.is_err());

        // Now inject server errors — client errors should be preserved
        ctx.set_server_errors([("email".to_string(), vec!["Already exists".to_string()])]);

        let field = ctx.field("email").expect("exists");

        let all_errors = field.validation.errors().expect("has errors");

        // Should have both client and server errors
        assert!(all_errors.len() >= 2);
        assert!(all_errors.0.iter().any(|e| !e.is_server()));
        assert!(all_errors.0.iter().any(Error::is_server));
    }

    // ── Validation and submit flow ──────────────────────────────────────

    #[test]
    fn on_blur_validates_when_mode_set() {
        let mut ctx = Context::new(Mode::on_blur_revalidate());

        ctx.register("name", text(""), Some(required_validator()), None);

        ctx.on_blur("name");

        assert!(ctx.field("name").expect("exists").touched);
        assert!(ctx.field("name").expect("exists").validation.is_err());
    }

    #[test]
    fn on_change_validates_when_mode_set() {
        let mut ctx = Context::new(Mode::on_change());

        ctx.register("name", text("hello"), Some(required_validator()), None);

        ctx.on_change("name", text(""));

        assert!(ctx.field("name").expect("exists").validation.is_err());
    }

    #[test]
    fn on_change_revalidates_when_invalid() {
        let mut ctx = Context::new(Mode::on_blur_revalidate());

        ctx.register("name", text(""), Some(required_validator()), None);

        // Make it invalid first via validate_field
        let _result = ctx.validate_field("name");

        assert!(ctx.field("name").expect("exists").validation.is_err());

        // on_change should revalidate because revalidate_on_change is true
        ctx.on_change("name", text("fixed"));

        assert!(ctx.field("name").expect("exists").validation.is_ok());
    }

    #[test]
    fn validate_all_returns_false_when_invalid() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), Some(required_validator()), None);
        ctx.register("email", text("test@test.com"), None, None);

        assert!(!ctx.validate_all());
    }

    #[test]
    fn submit_calls_handler_when_valid() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("Alice"), None, None);

        let mut called = false;

        let result = ctx.submit(|data| {
            called = true;
            assert_eq!(data.get_text("name"), Some("Alice"));
        });

        assert!(called);
        assert!(result.is_ok());
        assert!(ctx.is_submitted);
    }

    #[test]
    fn submit_does_not_call_handler_when_invalid() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), Some(required_validator()), None);

        let mut called = false;

        let result = ctx.submit(|_| called = true);

        assert!(!called);

        let errors = result.expect_err("invalid form should return collected errors");

        assert_eq!(errors.0.len(), 1);
        assert_eq!(errors.0[0].code, ErrorCode::Required);
        assert!(ctx.is_submitted);
    }

    #[test]
    fn submit_touches_all_fields() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("a", text(""), None, None);
        ctx.register("b", text(""), None, None);

        assert!(!ctx.field("a").expect("exists").touched);
        assert!(!ctx.field("b").expect("exists").touched);

        let _result = ctx.submit(|_| {});

        assert!(ctx.field("a").expect("exists").touched);
        assert!(ctx.field("b").expect("exists").touched);
    }

    #[test]
    fn reset_restores_initial_values() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("initial"), Some(required_validator()), None);

        // Modify the field
        ctx.on_change("name", text("modified"));
        ctx.on_blur("name");

        ctx.is_submitted = true;
        ctx.is_submitting = true;

        ctx.reset();

        let field = ctx.field("name").expect("exists");

        assert_eq!(field.value, text("initial"));
        assert!(!field.dirty);
        assert!(!field.touched);
        assert!(field.validation.is_ok());
        assert!(!ctx.is_submitted);
        assert!(!ctx.is_submitting);
    }

    #[test]
    fn reset_increments_validation_generation() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("initial"), Some(required_validator()), None);

        let gen_before = ctx.field("name").expect("exists").validation_generation;

        ctx.reset();

        let gen_after = ctx.field("name").expect("exists").validation_generation;

        assert_eq!(gen_after, gen_before + 1);
    }

    #[test]
    fn is_valid_and_is_dirty() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);

        assert!(ctx.is_valid());
        assert!(!ctx.is_dirty());

        ctx.on_change("name", text("Alice"));

        assert!(ctx.is_dirty());

        ctx.fields
            .get_mut("name")
            .expect("registered field")
            .validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));

        assert!(!ctx.is_valid());
    }

    #[test]
    fn cross_field_validator_triggers_on_dependency_change() {
        let mut ctx = Context::new(Mode::on_change());

        ctx.register("password", text("secret"), None, None);
        ctx.register("confirm", text("secret"), None, None);

        ctx.register_cross_field_validator("confirm", password_match_validator());

        // Changing password triggers re-validation of confirm
        ctx.on_change("password", text("changed"));

        let confirm = ctx.field("confirm").expect("exists");

        assert!(confirm.validation.is_err());
    }

    #[test]
    fn on_change_does_not_revalidate_existing_client_error_without_mode_flag() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), Some(required_validator()), None);

        let _result = ctx.validate_field("name");

        assert!(ctx.field("name").expect("exists").validation.is_err());

        ctx.on_change("name", text("Alice"));

        assert!(ctx.field("name").expect("exists").validation.is_err());
    }

    #[test]
    fn cross_field_validator_returns_ok_when_values_match() {
        let mut ctx = Context::new(Mode::on_change());

        ctx.register("password", text("secret"), None, None);
        ctx.register("confirm", text("secret"), None, None);

        ctx.register_cross_field_validator("confirm", password_match_validator());

        ctx.on_change("password", text("secret"));

        assert!(ctx.field("confirm").expect("exists").validation.is_ok());
    }

    #[test]
    fn run_field_validation_ignores_stale_cross_field_target() {
        let mut ctx = Context::new(Mode::on_change());

        ctx.register("password", text("secret"), None, None);

        ctx.cross_field_registry.insert(
            "confirm".to_string(),
            vec![CrossFieldValidator {
                depends_on: vec!["password".to_string()],
                validate_fn: Arc::new(|_value, _ctx| Ok(())),
            }],
        );

        ctx.on_change("password", text("changed"));

        assert!(ctx.field("password").expect("exists").validation.is_ok());
        assert!(ctx.field("confirm").is_none());
    }

    #[test]
    fn direct_validation_does_not_run_cross_field_validators_for_same_target() {
        use core::sync::atomic::{AtomicUsize, Ordering};

        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("confirm", text("secret"), None, None);

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_validator = Arc::clone(&calls);

        ctx.cross_field_registry.insert(
            "confirm".to_string(),
            vec![CrossFieldValidator {
                depends_on: vec!["confirm".to_string()],
                validate_fn: Arc::new(move |_value, _ctx| {
                    calls_for_validator.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }),
            }],
        );

        assert_eq!(ctx.validate_field("confirm"), Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    // ── AnyValidator ────────────────────────────────────────────────────

    #[test]
    fn any_validator_passes_if_one_passes() {
        struct AlwaysFail;

        impl Validator for AlwaysFail {
            fn validate(&self, _v: &Value, _c: &ValContext) -> Result {
                Err(Errors(vec![Error::custom("fail", "always fails")]))
            }
        }

        struct AlwaysPass;

        impl Validator for AlwaysPass {
            fn validate(&self, _v: &Value, _c: &ValContext) -> Result {
                Ok(())
            }
        }

        let any = AnyValidator::new(vec![
            crate::validation::boxed_validator(AlwaysFail),
            crate::validation::boxed_validator(AlwaysPass),
        ]);

        let ctx = ValContext::standalone("test");

        assert!(any.validate(&text("x"), &ctx).is_ok());
    }

    #[test]
    fn any_validator_fails_if_all_fail() {
        struct Fail1;

        impl Validator for Fail1 {
            fn validate(&self, _v: &Value, _c: &ValContext) -> Result {
                Err(Errors(vec![Error::custom("f1", "fail 1")]))
            }
        }

        struct Fail2;

        impl Validator for Fail2 {
            fn validate(&self, _v: &Value, _c: &ValContext) -> Result {
                Err(Errors(vec![Error::custom("f2", "fail 2")]))
            }
        }

        let any = AnyValidator::new(vec![
            crate::validation::boxed_validator(Fail1),
            crate::validation::boxed_validator(Fail2),
        ]);

        let ctx = ValContext::standalone("test");

        let result = any.validate(&text("x"), &ctx);

        assert!(result.is_err());
        assert_eq!(result.errors().expect("has errors").len(), 2);
    }

    #[test]
    fn any_validator_with_no_validators_is_ok() {
        let any = AnyValidator::new(Vec::new());
        let ctx = ValContext::standalone("test");

        assert!(any.validate(&text("x"), &ctx).is_ok());
    }

    #[test]
    fn any_validator_debug_includes_validator_count() {
        let debug = format!("{:?}", AnyValidator::new(Vec::new()));

        assert!(debug.contains("AnyValidator"));
        assert!(debug.contains("<0 validators>"));
    }

    // ── Data ────────────────────────────────────────────────────────

    #[test]
    fn form_data_get_text() {
        let mut data = Data::default();

        data.fields.insert("name".to_string(), text("Alice"));

        assert_eq!(data.get_text("name"), Some("Alice"));
        assert_eq!(data.get_text("missing"), None);
    }

    #[test]
    fn form_data_get_number() {
        let mut data = Data::default();

        data.fields
            .insert("age".to_string(), Value::Number(Some(30.0)));

        assert_eq!(data.get_number("age"), Some(30.0));
        assert_eq!(data.get_number("missing"), None);
    }

    #[test]
    fn form_data_default_is_empty() {
        let data = Data::default();

        assert!(data.fields.is_empty());
    }

    // ── Mode ──────────────────────────────────────────────────

    #[test]
    fn validation_mode_on_submit_all_false() {
        let mode = Mode::on_submit();

        assert!(!mode.on_change);
        assert!(!mode.on_blur);
        assert!(!mode.on_input);
        assert!(!mode.revalidate_on_change);
    }

    #[test]
    fn validation_mode_on_blur_revalidate() {
        let mode = Mode::on_blur_revalidate();

        assert!(mode.on_blur);
        assert!(mode.revalidate_on_change);
        assert!(!mode.on_change);
        assert!(!mode.on_input);
    }

    #[test]
    fn validation_mode_on_change() {
        let mode = Mode::on_change();

        assert!(mode.on_change);
        assert!(!mode.on_blur);
    }

    // ── on_input ─────────────────────────────────────────────────────────

    #[test]
    fn on_input_updates_value_without_marking_dirty() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);

        ctx.on_input("name", text("typing"));

        let field = ctx.field("name").expect("exists");

        assert_eq!(field.value, text("typing"));
        assert!(!field.dirty); // on_input does NOT set dirty
    }

    #[test]
    fn on_input_clears_server_errors() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        ctx.set_server_errors([("email".to_string(), vec!["Taken".to_string()])]);

        assert!(ctx.field("email").expect("exists").validation.is_err());

        ctx.on_input("email", text("new"));

        assert!(ctx.field("email").expect("exists").validation.is_ok());
    }

    #[test]
    fn on_input_validates_when_mode_set() {
        let mode = Mode {
            on_input: true,
            ..Default::default()
        };

        let mut ctx = Context::new(mode);

        ctx.register("name", text(""), Some(required_validator()), None);

        ctx.on_input("name", text(""));

        assert!(ctx.field("name").expect("exists").validation.is_err());

        ctx.on_input("name", text("valid"));

        assert!(ctx.field("name").expect("exists").validation.is_ok());
    }

    #[test]
    fn on_input_increments_validation_generation() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);

        let gen_before = ctx.field("name").expect("exists").validation_generation;

        ctx.on_input("name", text("a"));

        let gen_after = ctx.field("name").expect("exists").validation_generation;

        assert_eq!(gen_after, gen_before + 1);
    }

    #[test]
    fn on_change_increments_validation_generation() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);

        let gen_before = ctx.field("name").expect("exists").validation_generation;

        ctx.on_change("name", text("a"));

        let gen_after = ctx.field("name").expect("exists").validation_generation;

        assert_eq!(gen_after, gen_before + 1);
    }

    #[test]
    fn on_input_on_unregistered_field_is_noop() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.on_input("ghost", text("x")); // should not panic
    }

    // ── Guard paths (unregistered fields) ───────────────────────────────

    #[test]
    fn on_change_on_unregistered_field_is_noop() {
        let mut ctx = Context::new(Mode::on_change());

        ctx.on_change("ghost", text("x")); // should not panic
    }

    #[test]
    fn on_blur_on_unregistered_field_is_noop() {
        let mut ctx = Context::new(Mode::on_blur_revalidate());

        ctx.on_blur("ghost"); // should not panic
    }

    #[test]
    fn set_server_errors_ignores_unknown_fields() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text(""), None, None);

        ctx.set_server_errors([("nonexistent".to_string(), vec!["err".to_string()])]);

        // The known field is unaffected
        assert!(ctx.field("name").expect("exists").validation.is_ok());
    }

    // ── field_mut ───────────────────────────────────────────────────────

    #[test]
    fn field_mut_allows_mutation() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("old"), None, None);

        ctx.field_mut("name").expect("exists").value = text("new");

        assert_eq!(ctx.field("name").expect("exists").value, text("new"));
    }

    // ── Async validator methods ─────────────────────────────────────────

    #[test]
    fn has_async_validators_false_by_default() {
        let ctx = Context::new(Mode::on_submit());

        assert!(!ctx.has_async_validators());
    }

    #[test]
    fn register_and_collect_async_validators() {
        use core::task::{Context as TaskContext, Poll, Waker};

        use crate::validation::BoxedAsyncValidator;

        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        assert!(!ctx.has_async_validators());

        let boxed: BoxedAsyncValidator = boxed_async_ok_validator();

        ctx.register_async_validator("email", boxed);

        assert!(ctx.has_async_validators());

        let collected = ctx.collect_async_validators();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].0, "email");

        let value = text("ok");
        let validation_ctx = ValContext::standalone("email");
        let mut future = collected[0].1.validate_async(&value, &validation_ctx);
        let mut task_ctx = TaskContext::from_waker(Waker::noop());

        assert!(matches!(
            future.as_mut().poll(&mut task_ctx),
            Poll::Ready(Ok(()))
        ));
    }

    #[test]
    fn register_stores_inline_async_validator() {
        use crate::validation::BoxedAsyncValidator;

        let mut ctx = Context::new(Mode::on_submit());
        let boxed: BoxedAsyncValidator = boxed_async_ok_validator();

        ctx.register("email", text(""), None, Some(Arc::clone(&boxed)));

        assert!(ctx.has_async_validators());
        assert_eq!(ctx.collect_async_validators().len(), 1);
    }

    #[test]
    fn cross_field_validator_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        let validator = CrossFieldValidator {
            depends_on: vec![String::from("password")],
            validate_fn: Arc::new(|_value, _ctx| Ok(())),
        };

        assert_send_sync::<CrossFieldValidator>();

        assert!(
            validator
                .validate(
                    &Value::Text(String::new()),
                    &ValContext::standalone("confirm")
                )
                .is_ok()
        );
    }

    #[test]
    fn cross_field_validator_debug_includes_dependencies() {
        let validator = CrossFieldValidator {
            depends_on: vec![String::from("password")],
            validate_fn: Arc::new(|_value, _ctx| Ok(())),
        };

        let debug = format!("{validator:?}");

        assert!(debug.contains("CrossFieldValidator"));
        assert!(debug.contains("password"));
        assert!(debug.contains("<fn>"));
    }

    // ── validate_all true path ──────────────────────────────────────────

    #[test]
    fn validate_all_returns_true_when_all_valid() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("name", text("Alice"), None, None);
        ctx.register("email", text("a@b.com"), None, None);

        assert!(ctx.validate_all());
    }

    // ── reset clears server errors ──────────────────────────────────────

    #[test]
    fn reset_clears_server_errors() {
        let mut ctx = Context::new(Mode::on_submit());

        ctx.register("email", text(""), None, None);

        ctx.set_server_errors([("email".to_string(), vec!["Taken".to_string()])]);

        assert!(!ctx.server_errors.is_empty());

        ctx.reset();

        assert!(ctx.server_errors.is_empty());
        assert!(ctx.field("email").expect("exists").validation.is_ok());
    }

    // ── Data::get_bool ──────────────────────────────────────────────────

    #[test]
    fn form_data_get_bool() {
        let mut data = Data::default();

        data.fields.insert("agree".to_string(), Value::Bool(true));

        assert_eq!(data.get_bool("agree"), Some(true));
        assert_eq!(data.get_bool("missing"), None);
    }

    // ── AnyValidator::boxed ─────────────────────────────────────────────

    #[test]
    fn any_validator_boxed_wraps_correctly() {
        struct AlwaysPass;

        impl Validator for AlwaysPass {
            fn validate(&self, _v: &Value, _c: &ValContext) -> Result {
                Ok(())
            }
        }

        let any = AnyValidator::new(vec![crate::validation::boxed_validator(AlwaysPass)]);

        let boxed = any.boxed();

        let ctx = ValContext::standalone("test");

        assert!(boxed.validate(&text("x"), &ctx).is_ok());
    }

    // ── Debug impl ──────────────────────────────────────────────────────

    #[test]
    fn debug_impl_does_not_panic() {
        let ctx = Context::new(Mode::on_submit());

        let debug = format!("{ctx:?}");

        assert!(debug.contains("Context"));
        assert!(debug.contains("<0 validators>"));
    }
}
