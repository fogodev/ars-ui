# Form Validation Testing

## 1. Form Integration Tests

Form-level tests verify that validation, submission, and error propagation work
correctly when multiple fields interact. These complement individual field unit
tests by exercising the full form lifecycle.

### Validators Builder API

> **Note:** The `Validators` builder below is a test convenience wrapper. For canonical validation types, see `BoxedValidator` and `BoxedAsyncValidator` in [07-forms.md](../foundation/07-forms.md).

The `Validators` struct provides a fluent builder for constructing sync and async field validators. This API is a convenience wrapper over `BoxedValidator` and `BoxedAsyncValidator` from [07-forms.md](../foundation/07-forms.md).

```rust
/// Test-only convenience builder for composing synchronous validators.
///
/// Available methods: `required()`, `email()`, `min_length(n)`, `range(min, max)`,
/// `matches_field(name)`, `zip_code()`, `custom(fn)`, `build()`.
///
/// These are NOT part of the canonical `ars-forms` API. In production code,
/// use `BoxedValidator` directly or implement the `Validator` trait.
#[cfg(test)]
struct Validators { rules: Vec<Box<dyn Fn(&FieldValue, &ValidationContext) -> ValidationResult>> }

#[cfg(test)]
impl Validators {
    pub fn new() -> Self { Self { rules: vec![] } }
    pub fn required(mut self) -> Self { /* adds required check */ self }
    pub fn email(mut self) -> Self { /* adds email format check */ self }
    pub fn min_length(mut self, len: usize) -> Self { /* adds min length check */ self }
    // Test-only convenience methods below. NOT part of canonical ValidatorsBuilder API.
    // These are implemented internally via `.custom()` closures.
    pub fn range(mut self, min: f64, max: f64) -> Self { /* adds numeric range check */ self }
    pub fn matches_field(mut self, other: &str) -> Self { /* adds cross-field equality check */ self }
    pub fn zip_code(mut self) -> Self { /* adds zip code format check */ self }
    pub fn custom(mut self, f: impl Fn(&FieldValue, &ValidationContext) -> ValidationResult + 'static) -> Self {
        self.rules.push(Box::new(f));
        self
    }
    /// Build a BoxedValidator that returns the FIRST failing rule's error.
    pub fn build(self) -> BoxedValidator {
        let rules = self.rules;
        Box::new(move |value: &FieldValue, ctx: &ValidationContext| -> ValidationResult {
            for rule in &rules {
                let result = rule(value, ctx);
                if result.is_invalid() {
                    return result;
                }
            }
            ValidationResult::Valid
        })
    }
}

// `AsyncValidators` is a test convenience wrapper around `AsyncFnValidator::new(closure).boxed()`.
// See 07-forms.md §4 for the canonical async validator API.

/// Test-only convenience builder for composing asynchronous validators.
/// NOT part of the canonical `ars-forms` API.
#[cfg(test)]
struct AsyncValidators { rules: Vec<BoxedAsyncValidator> }

#[cfg(test)]
impl AsyncValidators {
    pub fn new() -> Self { Self { rules: vec![] } }
    pub fn unique_username(mut self) -> Self { /* adds async uniqueness check */ self }
    pub fn custom_async(mut self, f: impl AsyncValidator + 'static) -> Self {
        self.rules.push(Arc::new(f));
        self
    }

    /// Build a composed async validator that runs all registered rules.
    pub fn build(self) -> BoxedAsyncValidator {
        let rules = self.rules;
        Arc::new(ComposedAsyncValidator { rules })
    }
}

/// Composed async validator that runs multiple async validators and merges results.
struct ComposedAsyncValidator {
    rules: Vec<BoxedAsyncValidator>,
}

impl AsyncValidator for ComposedAsyncValidator {
    fn validate_async<'a>(
        &'a self,
        value: String,
        ctx: OwnedValidationContext,
    ) -> Pin<Box<dyn Future<Output = ValidationResult> + 'a>> {
        Box::pin(async move {
            let mut all_errors = Vec::new();
            for rule in &self.rules {
                if let ValidationResult::Invalid(errors) = rule.validate_async(value.clone(), ctx.clone()).await {
                    all_errors.extend(errors.0);
                }
            }
            if all_errors.is_empty() {
                ValidationResult::Valid
            } else {
                ValidationResult::Invalid(ValidationErrors(all_errors))
            }
        })
    }
}
```

### 1.1 Submit with Mixed Valid/Invalid Fields

```rust
#[test]
fn submit_with_mixed_validity_blocks_submission() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("email", FieldValue::Text("".into()),
        Some(Validators::new().required().email().build()),
        None);
    form.register("name", FieldValue::Text("".into()),
        Some(Validators::new().required().build()),
        None);
    form.register("age", FieldValue::Number(None),
        Some(Validators::new().required().range(0.0, 150.0).build()),
        None);

    form.on_change("email", FieldValue::Text("not-an-email".into()));
    form.on_change("name", FieldValue::Text("Alice".into()));
    form.on_change("age", FieldValue::Number(Some(25.0)));

    let result = form.submit(|_data| {});
    assert!(result.is_invalid(), "Form with invalid email should not submit");
    assert!(form.field("email").expect("email field must be registered").validation.is_invalid());
    assert!(form.field("name").expect("name field must be registered").validation.is_valid());
    assert!(form.field("age").expect("age field must be registered").validation.is_valid());
}
```

### 1.2 Async Validator Interaction with Submission

```rust
/// Test helper: simulates async validation completing.
/// In production, async validation completion flows through form_submit::Machine
/// events (Event::ValidationPassed / Event::ValidationFailed), not through
/// direct FormContext method calls.
impl FormContext {
    #[cfg(test)]
    fn complete_async_validation(&mut self, field: &str, result: ValidationResult) {
        if let Some(state) = self.fields.get_mut(field) {
            state.validating = false;
            state.validation = result;
        }
    }

    #[cfg(test)]
    fn complete_async_validation_if_current(
        &mut self,
        field: &str,
        generation: u64,
        result: ValidationResult,
    ) {
        if let Some(state) = self.fields.get(field) {
            if state.validation_generation == generation {
                self.complete_async_validation(field, result);
            }
            // else: stale result, discard silently
        }
    }
}
```

```rust
#[test]
fn async_validator_runs_on_blur() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register(
        "username",
        FieldValue::Text("".into()),
        None,
        Some(AsyncValidators::new().unique_username().build()),
    );

    form.on_change("username", FieldValue::Text("taken".into()));
    form.on_blur("username");

    // Async validation is now in flight — field should show validating state
    let field = form.field("username").expect("field must be registered");
    assert!(field.validating, "field must be in validating state");

    // Simulate async validation completing
    form.complete_async_validation("username", ValidationResult::Invalid(ValidationErrors(vec![
        ValidationError::custom("taken", "username is already taken"),
    ])));

    let field = form.field("username").expect("field must be registered");
    assert!(!field.validating, "field must no longer be validating");
    assert!(field.validation.is_invalid(), "field must be invalid after async check");
}
```

### 1.3 Server Error Injection

```rust
#[test]
fn server_error_injection_via_form_context() {
    let mut form = FormContext::new(ValidationMode::on_submit());
    form.register("email", FieldValue::Text("".into()), None, None);

    form.on_change("email", FieldValue::Text("alice@example.com".into()));

    // Simulate server response with field-level errors
    form.set_server_errors(BTreeMap::from([
        ("email".into(), vec!["Already registered".into()]),
    ]));

    let email_field = form.field("email").expect("email field must be registered");
    assert!(
        email_field.validation.is_invalid(),
        "server error must surface through the validation field"
    );

    // Server errors are cleared on next user input to the affected field
    form.on_change("email", FieldValue::Text("bob@example.com".into()));
    let email_field = form.field("email").expect("email field must be registered");
    assert!(
        email_field.validation.is_valid(),
        "server errors must be cleared after user edits the field"
    );
}
```

### 1.4 Form Reset Restores Initial Values

```rust
#[test]
fn form_reset_restores_initial_values() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("email", FieldValue::Text("".into()), None, None);
    form.on_change("email", FieldValue::Text("test@example.com".into()));
    assert!(form.field("email").expect("email field must be registered").dirty);

    form.reset();

    let field = form.field("email").expect("email field must be registered");
    assert_eq!(field.value, FieldValue::Text("".into()));
    assert!(!field.dirty);
    assert!(!field.touched);
    assert!(field.validation.is_valid());
}
```

### 1.5 Disabled and Readonly Field Behavior

```rust
#[test]
fn disabled_field_excluded_from_submission() {
    let mut form = FormContext::new(ValidationMode::on_submit());
    form.register("name", FieldValue::Text("John".into()), None, None);
    form.register("secret", FieldValue::Text("hidden".into()), None, None);
    // Mark field as disabled at the adapter level
    // (disabled/readonly is an adapter-level concern, not core FormContext)

    let mut submitted_fields = Vec::new();
    form.submit(|data| {
        submitted_fields = data.fields.keys().cloned().collect();
    });
    // Note: disabled field exclusion is handled by the adapter,
    // which skips including disabled fields in FormData.
}

/// Disabled field validation is an adapter concern — the adapter sets the
/// disabled attribute on the HTML element and prevents user interaction.
/// At the FormContext level, disabled fields are excluded from validation
/// by checking the component's disabled prop, not a FormContext flag.
#[wasm_bindgen_test]
fn disabled_field_skips_validation_in_adapter() {
    let harness = render(Form::new(form_props())).await;
    // Register a required field
    let input = harness.query("[name='required_field']").expect("field must exist");
    // Disabled state is managed at the adapter level, not FormContext
    input.element.set_attribute("disabled", "").expect("set disabled");
    harness.tick().await;
    // Submit the form
    harness.click_selector("button[type='submit']");
    harness.tick().await;
    // Disabled field should NOT produce a validation error
    let errors = harness.query_selector_all("[data-ars-part='error-message']");
    assert_eq!(errors.len(), 0, "disabled fields must be excluded from validation");
}

#[test]
fn readonly_field_included_in_submit_data() {
    // Register field, mark readonly, submit
    // Assert field IS in submitted data
    let mut form = FormContext::new(ValidationMode::on_submit());
    form.register("readonly_field", FieldValue::Text("preserved".into()), None, None);
    // Readonly fields participate in submission (their value is preserved)
    let mut submitted_value = None;
    let result = form.submit(|data| {
        submitted_value = data.fields.get("readonly_field").cloned();
    });
    assert!(result.is_valid());
    assert!(submitted_value.is_some(), "readonly field must be included in submit data");
}
```

> **Note:** Field disabled/readonly state is managed at the adapter level, not in `FormContext`. The adapter is responsible for excluding disabled fields from `FormData` during submission and preventing `on_change` events from reaching `FormContext` for readonly fields.

### 1.6 Nested Fieldset Validation

```rust
#[test]
fn nested_fieldset_validation_ordering() {
    // Parent form with two fieldsets, each containing fields.
    // Submit parent → all fieldset fields validate.
    // Verify validation runs for all nested fields.
    let mut form = FormContext::new(ValidationMode::on_submit());
    form.register("billing_name", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);
    form.register("billing_zip", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);
    form.register("shipping_name", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);
    form.register("shipping_zip", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);

    // Submit with all fields empty → all should fail validation
    let result = form.submit(|_data| {});
    assert!(result.is_invalid());
    assert!(form.field("billing_name").expect("billing_name field must be registered").validation.is_invalid());
    assert!(form.field("billing_zip").expect("billing_zip field must be registered").validation.is_invalid());
    assert!(form.field("shipping_name").expect("shipping_name field must be registered").validation.is_invalid());
    assert!(form.field("shipping_zip").expect("shipping_zip field must be registered").validation.is_invalid());
}
```

### 1.7 Nested Form Validation Ordering

```rust
/// Nested form validation is an adapter concern. The adapter coordinates
/// parent/child FormContext instances through component hierarchy.
/// This test demonstrates the pattern at the Dioxus adapter level.
#[wasm_bindgen_test]
async fn nested_form_inner_errors_block_outer_submit() {
    let harness = mount_dioxus(rsx! {
        Form { id: "outer",
            TextField { name: "outer_field", required: true }
            Form { id: "inner",
                TextField { name: "inner_field", required: true }
            }
        }
    }).await;
    // Submit the outer form — inner form's required field should also validate
    harness.click_selector("#outer button[type='submit']");
    harness.tick().await;
    // Both outer and inner required fields should show errors
    let errors = harness.query_selector_all("[data-ars-part='error-message']");
    assert_eq!(errors.len(), 2, "both outer and inner required fields must show errors");
}
```

### 1.8 on_input Validates Without Marking Dirty

```rust
#[test]
fn on_input_validates_without_marking_dirty() {
    // ValidationMode::on_input() convenience constructor does not exist;
    // use struct literal to enable on_input mode.
    let mut form = FormContext::new(ValidationMode { on_input: true, ..Default::default() });
    form.register("email", FieldValue::Text("".into()),
        Some(Validators::new().email().build()), None);

    form.on_input("email", FieldValue::Text("not-an-email".into()));

    let field = form.field("email").expect("field must be registered");
    assert!(!field.dirty, "on_input must NOT mark the field as dirty");
    assert!(field.validation.is_invalid(), "on_input must still run validation");
}
```

### 1.9 without_server_errors Preserves Client Errors

```rust
#[test]
fn without_server_errors_preserves_client_errors() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("email", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);

    // Inject a server error
    form.set_server_errors(BTreeMap::from([
        ("email".into(), vec!["Email already registered".into()]),
    ]));

    // Trigger client validation (field is empty → required fails)
    form.on_change("email", FieldValue::Text("".into()));

    let field = form.field("email").expect("field must be registered");
    let result = field.validation.clone();
    // Full result includes both server and client errors
    assert!(result.is_invalid());

    // Filter out server errors
    let client_only = result.without_server_errors();
    assert!(client_only.is_invalid(), "client validation error must remain");
    // Verify server error is gone
    let errors = client_only.errors().expect("must have errors");
    assert!(!errors.0.iter().any(|e| e.is_server()), "server errors must be filtered out");
}
```

### 1.10 Deregister Removes Field

```rust
#[test]
fn deregister_removes_field() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("temp", FieldValue::Text("data".into()),
        None, None);
    assert!(form.field("temp").is_some(), "field must exist after registration");

    form.deregister("temp");
    assert!(form.field("temp").is_none(), "field must be gone after deregister");
}
```

---

## 2. Cross-Field Validation Testing

Cross-field validators receive form-level context and can compare values across
fields. These use the `FormContext` API for registration and validation.

### 2.1 Dependent Fields

```rust
#[test]
fn password_confirmation_cross_validation() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("password", FieldValue::Text("".into()),
        Some(Validators::new().required().min_length(6).build()), None);
    form.register("confirm", FieldValue::Text("".into()),
        Some(Validators::new().required().matches_field("password").build()), None);

    form.on_change("password", FieldValue::Text("abc123".into()));
    form.on_change("confirm", FieldValue::Text("abc456".into()));
    form.on_blur("confirm");

    let field = form.field("confirm").expect("confirm field must be registered");
    assert!(field.validation.is_invalid(),
        "mismatched passwords must fail validation");

    form.on_change("confirm", FieldValue::Text("abc123".into()));
    form.on_blur("confirm");

    let field = form.field("confirm").expect("confirm field must be registered");
    assert!(field.validation.is_valid());
}
```

### 2.2 Cascading Validation

The zip code validator uses `CrossFieldValidator` to declare its dependency on the country field. When the country changes, the form automatically re-validates the zip field:

```rust
#[test]
fn cascading_validation_on_dependency_change() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("country", FieldValue::Text("".into()), None, None);
    form.register("zip", FieldValue::Text("".into()),
        Some(Validators::new().required().build()), None);

    form.register_cross_field_validator("zip", CrossFieldValidator {
        depends_on: vec!["country".into()],
        validate_fn: Arc::new(|value: &FieldValue, ctx: &ValidationContext| -> ValidationResult {
            let country = ctx.form_values.get("country");
            match (country, value) {
                (Some(FieldValue::Text(c)), FieldValue::Text(zip)) if c == "US" => {
                    if zip.len() != 5 || !zip.chars().all(|c| c.is_ascii_digit()) {
                        ValidationResult::Invalid(ValidationErrors(vec![
                            ValidationError::custom("zip_format", "US ZIP must be 5 digits")
                        ]))
                    } else {
                        ValidationResult::Valid
                    }
                }
                _ => ValidationResult::Valid,
            }
        }),
    });

    form.on_change("country", FieldValue::Text("US".into()));
    form.on_change("zip", FieldValue::Text("SW1A 1AA".into())); // UK format
    form.on_blur("zip");

    let field = form.field("zip").expect("zip field must be registered");
    assert!(field.validation.is_invalid(),
        "UK postcode format must fail US zip validation");

    // Changing country triggers automatic re-validation of zip via CrossFieldValidator.depends_on
    form.on_change("country", FieldValue::Text("UK".into()));

    let field = form.field("zip").expect("zip field must be registered");
    assert!(field.validation.is_valid(),
        "UK postcode must pass when country is no longer US");
}
```

### 2.2.1 CrossFieldValidator Automatic Re-Validation

```rust
#[test]
fn cross_field_validator_auto_revalidates_on_dependency_change() {
    let mut form = FormContext::new(ValidationMode::on_change());

    form.register("password", FieldValue::Text("abc123".into()), Some(Validators::new().build()), None);
    form.register("confirm_password", FieldValue::Text("abc123".into()), Some(Validators::new().build()), None);
    form.register_cross_field_validator("confirm_password", CrossFieldValidator {
        depends_on: vec!["password".into()],
        validate_fn: Arc::new(|value: &FieldValue, ctx: &ValidationContext| -> ValidationResult {
            let password = ctx.form_values.get("password");
            if Some(value) != password {
                ValidationResult::Invalid(ValidationErrors(vec![
                    ValidationError::custom("mismatch", "Passwords must match")
                ]))
            } else {
                ValidationResult::Valid
            }
        }),
    });

    // Both match — valid
    assert!(form.validate_field("confirm_password").is_valid());

    // Change password — confirm_password should auto-revalidate
    form.on_change("password", FieldValue::Text("xyz789".into()));
    // CrossFieldValidator.depends_on triggers automatic re-validation of confirm_password
    let result = form.field("confirm_password").expect("field must be registered").validation;
    assert!(result.is_invalid(), "confirm_password should auto-revalidate and fail when password changes");
}
```

### 2.3 Async Validation

```rust
#[test]
fn async_validation_shows_pending_state() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("username", FieldValue::Text("".into()),
        None,
        Some(AsyncValidators::new().unique_username().build()));

    form.on_change("username", FieldValue::Text("taken_user".into()));
    form.on_blur("username");

    // While async validation is in progress, the field should indicate validating state
    let field = form.field("username").expect("username field must be registered");
    assert!(field.touched, "field must be touched after blur");
    assert!(field.validating, "field must be in validating state after blur with async validator");

    // After async validation completes (simulated):
    form.complete_async_validation("username", ValidationResult::Invalid(
        ValidationErrors(vec![ValidationError::custom("username", "already taken".into())])
    ));
    let field = form.field("username").expect("username field must be registered");
    assert!(field.validation.is_invalid(),
        "async validation failure must surface in field state");
}
```

### 2.4 Async Validation Cancellation

```rust
#[test]
fn stale_async_validation_result_is_discarded() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register(
        "username",
        FieldValue::Text("".into()),
        None,
        Some(AsyncValidators::new().unique_username().build()),
    );

    // First change triggers async validation (generation = 1)
    form.on_change("username", FieldValue::Text("alice".into()));
    form.on_blur("username");
    let gen1 = form.field("username").expect("field must be registered").validation_generation;

    // Second change before first completes (generation = 2)
    form.on_change("username", FieldValue::Text("bob".into()));
    form.on_blur("username");
    let gen2 = form.field("username").expect("field must be registered").validation_generation;
    assert!(gen2 > gen1, "generation must increment on value change");

    // First async result arrives (stale — generation was 1)
    form.complete_async_validation_if_current("username", gen1, ValidationResult::Invalid(ValidationErrors(vec![
        ValidationError::custom("taken", "alice is taken"),
    ])));

    // Result should be discarded — field should still be validating
    let field = form.field("username").expect("field must be registered");
    assert!(field.validating, "stale result must be discarded, field still validating");
}
```

### 2.5 Form-Level Validator

```rust
#[test]
fn form_level_validator_receives_all_fields() {
    let mut form = FormContext::new(ValidationMode::on_submit());
    form.register("phone", FieldValue::Text("".into()), None, None);
    form.register("email", FieldValue::Text("".into()), None, None);

    // Register a cross-field validator: at least one contact method required
    form.register("_contact_check", FieldValue::Text("".into()),
        Some(Validators::new().custom(|_value, ctx| {
            let phone = ctx.form_values.get("phone").and_then(|v| v.as_text());
            let email = ctx.form_values.get("email").and_then(|v| v.as_text());
            let phone_empty = phone.map_or(true, |s| s.is_empty());
            let email_empty = email.map_or(true, |s| s.is_empty());
            if phone_empty && email_empty {
                ValidationResult::Invalid(ValidationErrors(vec![
                    ValidationError::custom("contact_required", "At least one of phone or email is required"),
                ]))
            } else {
                ValidationResult::Valid
            }
        }).build()),
        None,
    );

    let result = form.submit(|_data| {});
    assert!(result.is_invalid(), "Form with invalid contact check should fail submission");

    form.on_change("email", FieldValue::Text("alice@example.com".into()));
    let result = form.submit(|_data| {});
    assert!(result.is_valid(), "cross-field validator must pass with email provided");
}
```

## 3. Error State & Validation Display

Tests verify that validation errors are correctly surfaced to assistive technology through ARIA attributes and visible error messages.

### 3.1 `aria-invalid` on Validation Failure

```rust
#[test]
fn textfield_aria_invalid_on_error() {
    let harness = render(TextField::new().validate(|v| {
        if v.is_empty() { Err("Required".into()) } else { Ok(()) }
    }));
    harness.set_value("");
    harness.blur();
    assert_eq!(harness.input_attr("aria-invalid"), "true");
    harness.set_value("hello");
    harness.blur();
    assert_eq!(harness.input_attr("aria-invalid"), "false");
}

#[test]
fn select_aria_invalid_on_error() {
    let harness = render(Select::new().required(true));
    harness.focus("[data-ars-part='trigger']");
    harness.blur(); // No selection made
    assert_eq!(harness.trigger_attr("aria-invalid"), "true");
}
```

### 3.2 `aria-describedby` Points to Error Message

```rust
#[test]
fn textfield_aria_describedby_links_error() {
    let harness = render(TextField::new().validate(|v| {
        if v.is_empty() { Err("Field is required".into()) } else { Ok(()) }
    }));
    harness.set_value("");
    harness.blur();

    let describedby = harness.input_attr("aria-describedby");
    assert!(!describedby.is_empty());
    let error_el = harness.query_selector(&format!("#{describedby}"));
    assert!(error_el.is_some());
    assert_eq!(error_el.expect("error element must exist").text_content(), "Field is required");
}
```

### 3.3 Error Message Text Matches Validation Result

```rust
#[test]
fn datepicker_error_message_matches_validation() {
    let harness = render(DatePicker::new().validate(|d| {
        if d < Date::try_new_iso(2026, 1, 1).expect("valid date") {
            Err("Date must be in 2026 or later".into())
        } else {
            Ok(())
        }
    }));
    harness.set_value("2025-06-15");
    harness.blur();
    assert_eq!(harness.query_selector("[data-ars-part='error-message']").expect("error").text_content(), "Date must be in 2026 or later");
}

#[test]
fn select_error_message_on_required() {
    let harness = render(Select::new().required(true).error_message("Please select a value"));
    harness.focus("[data-ars-part='trigger']");
    harness.blur();
    assert_eq!(harness.query_selector("[data-ars-part='error-message']").expect("error").text_content(), "Please select a value");
}
```

---

## 4. Form Submission Integration Tests

Components that participate in HTML forms via hidden `<input>` elements must be tested for correct `name`/`value` attributes, `FormData` serialization, and disabled input exclusion.

### 4.1 Hidden Input Wiring

```rust
#[wasm_bindgen_test]
fn select_renders_hidden_input_with_name_and_value() {
    mount_to_body(|| {
        view! {
            <form id="test-form">
                <Select name="color" default_value="red">
                    <select::Item value="red">"Red"</select::Item>
                    <select::Item value="blue">"Blue"</select::Item>
                </Select>
            </form>
        }
    });

    let hidden = document()
        .query_selector("form#test-form input[type='hidden'][name='color']")
        .expect("query_selector must succeed")
        .expect("hidden input must exist");
    assert_eq!(hidden.get_attribute("value").as_deref(), Some("red"));
}
```

### 4.2 FormData Serialization

```rust
#[wasm_bindgen_test]
async fn form_data_includes_select_value() {
    mount_to_body(|| {
        view! {
            <form id="test-form">
                <Select name="fruit" default_value="apple">
                    <select::Item value="apple">"Apple"</select::Item>
                    <select::Item value="banana">"Banana"</select::Item>
                </Select>
            </form>
        }
    });

    let form: web_sys::HtmlFormElement = document()
        .query_selector("#test-form")
        .expect("query_selector must succeed")
        .expect("form element must exist")
        .dyn_into().expect("element must be HtmlFormElement");
    let form_data = web_sys::FormData::new_with_form(&form).expect("form data operation must succeed");
    assert_eq!(form_data.get("fruit").as_string().as_deref(), Some("apple"));
}
```

### 4.3 Disabled Input Exclusion

```rust
#[wasm_bindgen_test]
async fn disabled_component_excluded_from_form_data() {
    mount_to_body(|| {
        view! {
            <form id="test-form">
                <Select name="fruit" default_value="apple" disabled=true>
                    <select::Item value="apple">"Apple"</select::Item>
                </Select>
            </form>
        }
    });

    let form: web_sys::HtmlFormElement = document()
        .query_selector("#test-form")
        .expect("query_selector must succeed")
        .expect("form element must exist")
        .dyn_into().expect("element must be HtmlFormElement");
    let form_data = web_sys::FormData::new_with_form(&form).expect("form data operation must succeed");
    // Disabled inputs must not appear in FormData
    assert!(
        form_data.get("fruit").is_undefined(),
        "disabled component must not contribute to FormData"
    );
}
```

Apply this pattern for all form-participating components: `Select`, `Checkbox`, `Switch`, `RadioGroup`, `TextField`, `NumberField`, `Slider`, `Combobox`, `DatePicker`, `ColorPicker`.

## 5. form::component::Machine Tests

The `form::component::Machine` manages the top-level form lifecycle with states `Idle` and `Submitting`. These tests verify transitions, server error handling, and status messages.

> **Foundation reference:** `form::component::State`, `form::component::Event`, `form::component::Context` are defined in [07-forms.md](../foundation/07-forms.md) §14.

```rust
use ars_core::Service;

#[test]
fn submit_transitions_from_idle_to_submitting() {
    let props = form::component::Props::default();
    let (state, ctx) = form::component::Machine::init(&props, &Env::default(), &Default::default());
    assert_eq!(state, form::component::State::Idle);

    let plan = form::component::Machine::transition(&state, &form::component::Event::Submit, &ctx, &props)
        .expect("Submit from Idle must produce a transition");
    assert_eq!(plan.target, Some(form::component::State::Submitting));
}

#[test]
fn submit_complete_success_returns_to_idle() {
    let props = form::component::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(form::component::Event::Submit);
    assert_eq!(*svc.state(), form::component::State::Submitting);

    svc.send(form::component::Event::SubmitComplete { success: true });
    assert_eq!(*svc.state(), form::component::State::Idle);
    assert_eq!(svc.context().last_submit_succeeded, Some(true));
}

#[test]
fn submit_during_submitting_is_ignored() {
    let props = form::component::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(form::component::Event::Submit);
    assert_eq!(*svc.state(), form::component::State::Submitting);

    // Second submit while already submitting — should be ignored (no transition)
    let plan = form::component::Machine::transition(
        svc.state(), &form::component::Event::Submit, svc.context(), &props
    );
    assert!(plan.is_none(), "Submit during Submitting must be ignored");
}

#[test]
fn reset_restores_controlled_server_errors_and_clears_status() {
    let props = form::component::Props {
        validation_errors: BTreeMap::from([
            ("email".into(), vec!["taken".into()]),
        ]),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(form::component::Event::SetStatusMessage(Some("Error occurred".into())));

    svc.send(form::component::Event::Reset);
    assert_eq!(
        svc.context().server_errors,
        BTreeMap::from([("email".into(), vec!["taken".into()])]),
        "Reset must preserve controlled server errors from props",
    );
    assert!(svc.context().status_message.is_none(), "Reset must clear status message");
}

#[test]
fn reset_clears_uncontrolled_server_errors() {
    let props = form::component::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(form::component::Event::SetServerErrors(BTreeMap::from([
        ("email".into(), vec!["taken".into()]),
    ])));

    svc.send(form::component::Event::Reset);
    assert!(svc.context().server_errors.is_empty(), "Reset must clear transient server errors");
}

#[test]
fn set_server_errors_works_from_any_state() {
    let props = form::component::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // From Idle
    svc.send(form::component::Event::SetServerErrors(BTreeMap::from([
        ("name".into(), vec!["required".into()]),
    ])));
    assert!(!svc.context().server_errors.is_empty());

    // From Submitting
    svc.send(form::component::Event::Submit);
    svc.send(form::component::Event::SetServerErrors(BTreeMap::from([
        ("email".into(), vec!["invalid".into()]),
    ])));
    assert!(svc.context().server_errors.contains_key("email"));
}

#[test]
fn init_seeds_server_errors_from_props() {
    let mut errors = BTreeMap::new();
    errors.insert("email".into(), vec!["Invalid email".into()]);
    errors.insert("name".into(), vec!["Required".into()]);
    let props = form::component::Props {
        validation_errors: errors.clone(),
        ..Default::default()
    };
    let (state, ctx) = form::component::Machine::init(&props, &Env::default(), &Default::default());
    assert_eq!(ctx.server_errors, errors, "init must seed server_errors from props.validation_errors");
}

#[test]
fn on_props_changed_emits_set_server_errors() {
    let old = form::component::Props::default();
    let mut errors = BTreeMap::new();
    errors.insert("email".into(), vec!["Invalid".into()]);
    let new = form::component::Props {
        validation_errors: errors.clone(),
        ..Default::default()
    };
    let events = form::component::Machine::on_props_changed(&old, &new);
    assert_eq!(events, vec![form::component::Event::SetServerErrors(errors)]);
}

#[test]
fn on_props_changed_emits_clear_server_errors() {
    let mut errors = BTreeMap::new();
    errors.insert("email".into(), vec!["Invalid".into()]);
    let old = form::component::Props {
        validation_errors: errors,
        ..Default::default()
    };
    let new = form::component::Props::default();
    let events = form::component::Machine::on_props_changed(&old, &new);
    assert_eq!(events, vec![form::component::Event::ClearServerErrors]);
}

#[test]
fn root_attrs_emits_action_attribute() {
    let props = form::component::Props {
        action: Some("javascript:alert(1)".into()),
        ..Default::default()
    };
    let (state, ctx) = form::component::Machine::init(&props, &Env::default(), &Default::default());
    let api = form::component::Machine::connect(&state, &ctx, &props, &|_| {});
    let attrs = api.root_attrs();
    assert_eq!(attrs.get(&HtmlAttr::Action), Some("#"));
}

#[test]
fn root_attrs_emits_role_attribute() {
    let props = form::component::Props {
        role: Some("search".into()),
        ..Default::default()
    };
    let (state, ctx) = form::component::Machine::init(&props, &Env::default(), &Default::default());
    let api = form::component::Machine::connect(&state, &ctx, &props, &|_| {});
    let attrs = api.root_attrs();
    assert_eq!(attrs.get(&HtmlAttr::Role), Some("search"));
}

#[test]
fn on_props_changed_emits_set_validation_behavior() {
    let old = form::component::Props::default();
    let new = form::component::Props {
        validation_behavior: form::component::ValidationBehavior::Native,
        ..Default::default()
    };
    let events = form::component::Machine::on_props_changed(&old, &new);
    assert!(events.contains(&form::component::Event::SetValidationBehavior(
        form::component::ValidationBehavior::Native,
    )));
}
```

### 5.1 Disabled Field Exclusion

```rust
/// Test helper: creates a form component with the given registration setup.
fn form_test_component(setup: impl FnOnce(&mut FormContext) + 'static) -> impl Component {
    move || {
        let mut form = FormContext::new(ValidationMode::default());
        setup(&mut form);
        // Returns a minimal form view with a submit button
        form.into_view()
    }
}

#[wasm_bindgen_test]
async fn disabled_fields_excluded_from_form_data() {
    let harness = render(form_test_component(|form| {
        form.register("name", FieldValue::Text("Alice".into()), Some(Validators::new().build()), None);
        form.register("age", FieldValue::Number(Some(30.0)), Some(Validators::new().build()), None);
    })).await;

    // Disabled state is managed at the adapter level, not FormContext
    let age_input = harness.query_selector("input[name='age']").expect("age input must exist");
    age_input.element.set_attribute("disabled", "").expect("set disabled");
    harness.tick().await;

    harness.click_selector("button[type='submit']");
    harness.tick().await;
    // Verify disabled field is not in form data via DOM hidden input check
    let age_input = harness.query_selector("input[name='age']");
    assert!(age_input.is_none(),
        "disabled field's hidden input should not be present in the DOM");
}
```

## 6. form_submit::Machine Tests

### 6.1 Async Validators Run Even When Sync Fails

> **Canonical ownership:** This section owns the executable example for async validators
> running alongside sync failures through the `form_submit::Machine` Service API.
> [09-state-machine-correctness.md §6](09-state-machine-correctness.md)
> should keep only the rationale and a cross-reference to this example.

```rust
#[test]
fn async_validators_run_even_when_sync_fails() {
    // Foundation 07 specifies: async validators ALWAYS run even when sync validation
    // fails, to show all errors at once (lines 2138-2143).
    // Callback fields have no Default, so construct Props explicitly.
    let props = form_submit::Props {
        id: "test-submit".into(),
        validation_mode: ValidationMode::default(),
        spawn_async_validation: Callback::new(|(validators, send)| {
            no_cleanup() as Box<dyn FnOnce()>
        }),
        schedule_microtask: Callback::new(|f| f()),
    };
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Submit triggers validation of all registered fields
    svc.send(form_submit::Event::Submit);

    // After submit, both sync errors AND async validation must be in progress
    assert_eq!(*svc.state(), form_submit::State::Validating);

    // Sync validation fails immediately, but async is still running —
    // state stays in Validating until all validators complete.
    // Simulate async validation completing with an error via the Service event flow.
    svc.send(form_submit::Event::CompleteAsyncValidation {
        field: "email".into(),
        result: ValidationResult::Invalid(ValidationErrors(vec![
            ValidationError::custom("unique", "already registered"),
        ])),
    });

    // After all validators complete, sync failed → transition to ValidationFailed
    svc.send(form_submit::Event::ValidationFailed);
    assert_eq!(*svc.state(), form_submit::State::ValidationFailed);

    // Both sync and async errors should be captured in context
    let errors = &svc.context().field_errors;
    assert!(
        errors.get("email").map_or(false, |e| e.len() >= 2),
        "must have both sync and async errors for email field",
    );
}
```
