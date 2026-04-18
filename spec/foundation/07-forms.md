# Forms Specification (`ars-forms`)

## 1. Overview

The `ars-forms` crate provides a complete form management system for ars-ui components. It handles:

- **Validation**: built-in (HTML5 constraints), custom (Rust closures), server-side errors
- **Form context**: field registration, submission lifecycle, cross-field validation
- **Field association**: label ↔ input ↔ description ↔ error message linkage for accessibility
- **Hidden inputs**: allowing complex components (Select, DatePicker) to participate in native HTML form submission
- **Async validation**: debounced remote validation (e.g., username availability)

> **When to use each form approach:**
>
> - **`Context.submit()`** — Simple synchronous forms with client-side validation only.
> - **`form_submit::Machine`** — Standard forms with async validation and server submission.
> - **`form::Machine` (§14)** — Standard forms rendered as a `<form>` component; simplified 2-state lifecycle with server error integration.

<!-- Section map: §1 Overview, §2 Core Types, §3 Validator Trait, §4 Async Validation,
     §5 Form Context, §6 Field Association, §7 Hidden Inputs, §8 Form Submit Machine,
     §9 Focus Management, §10 Framework Integration, §11 Testing, §12 Fieldset Component,
     §13 Field Component, §14 Form Component, §15 Disabled vs Readonly Contract. -->

Design goals:

- **HTML-native first**: components render real `<input>` elements where possible for native form submission
- **Accessible by default**: `aria-required`, `aria-invalid`, `aria-describedby` computed automatically
- **Zero framework coupling**: validation logic is pure Rust, usable without framework dependencies
- **Controlled or uncontrolled**: works with React Hook Form-style controlled forms or native uncontrolled submission

### 1.1 Module Structure

Types use module-based namespacing (project convention: `module::Type` instead of `PrefixType`):

| Module         | Types                                                                                                                                                                         | Crate path                   |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `field`        | `State`, `Value`, `FileRef`, `Context`, `Descriptors`, `InputAria`, `ValueExt`, `SelectionExt`, `CheckboxExt`                                                                 | `ars_forms::field::*`        |
| `validation`   | `Error`, `ErrorCode`, `Errors`, `Result`, `ResultExt`, `Validator`, `BoxedValidator`, `Context`, `OwnedContext`, `AsyncValidator`, `BoxedAsyncValidator`, `boxed_validator()` | `ars_forms::validation::*`   |
| `form`         | `Context`, `Data`, `Mode`, `CrossFieldValidator`, `AnyValidator`                                                                                                              | `ars_forms::form::*`         |
| `hidden_input` | `Config`, `Value`, `attrs()`, `multi_attrs()`                                                                                                                                 | `ars_forms::hidden_input::*` |

Code examples in this spec use the short (unqualified) names since each section defines types within their module context.

---

## 2. Core Types

### 2.1 Error

```rust
/// A single validation failure.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    /// Human-readable error message.
    pub message: String,
    /// Machine-readable code for programmatic handling.
    pub code: ErrorCode,
}

/// Semantic codes for validation errors.
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorCode {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Min(f64),
    Max(f64),
    Step(f64),
    Pattern(String),
    Email,
    Url,
    Custom(String),
    Server(String),   // Error returned from server
    Async(String),    // Error from async validator
}

// NOTE: Factory methods accept a `FormMessages` parameter for locale-appropriate
// error text. This ensures no hardcoded English strings leak into production.
// Callers can still override messages per-validator via the `message` field,
// or globally by providing a localized `FormMessages` struct to the form context.
```

#### 2.1.1 Pluralization and ICU MessageFormat

Form validation error messages that include counts (e.g., minimum length, maximum items) MUST respect CLDR plural rules for the active locale. English has two plural categories (one, other), but other languages have up to six:

| Category | Description                  | Example Languages                |
| -------- | ---------------------------- | -------------------------------- |
| `zero`   | Zero quantity                | Arabic, Latvian, Welsh           |
| `one`    | Singular                     | English, German, French, Spanish |
| `two`    | Dual                         | Arabic, Hebrew, Slovenian        |
| `few`    | Paucal / small quantity      | Polish, Czech, Russian, Arabic   |
| `many`   | Large quantity               | Polish, Russian, Arabic, Welsh   |
| `other`  | General / default (required) | All languages                    |

**Plural Category Resolution**:

```rust
// plural_category is provided by ars-i18n (see 04-internationalization.md §4.3).
// ars-forms re-exports it for convenience:
pub use ars_i18n::{plural_category, PluralCategory};
// fn plural_category(count: usize, locale: &Locale) -> PluralCategory
```

**Error Message Pattern**: `MessageFn` closures for count-based validations MUST use `plural_category()` to select the correct message form:

```rust
// MessageFn wraps Arc<dyn Fn(...) + Send + Sync> on every target.
// The closure must return String and satisfy Send + Sync bounds.
let min_length_message: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync> =
    MessageFn::new(move |count: usize, locale: &Locale| -> String {
        let category = plural_category(count, locale);
        match category {
            PluralCategory::One => format!("Minimum {count} character"),
            _ => format!("Minimum {count} characters"),
        }
    });
```

For languages with complex plural rules (e.g., Polish: 1 znak, 2-4 znaki, 5-21 znaków, 22-24 znaki):

```rust
// Polish example
match category {
    PluralCategory::One => format!("Minimum {count} znak"),
    PluralCategory::Few => format!("Minimum {count} znaki"),
    PluralCategory::Many => format!("Minimum {count} znaków"),
    _ => format!("Minimum {count} znaków"),
}
```

Adapters MUST provide locale-aware `MessageFn` factories that handle all six CLDR categories. The `other` category is always required as a fallback.

```rust
impl Error {
    /// Creates a "required" validation error with locale-appropriate message.
    pub fn required(messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.required_error)(locale),
            code: ErrorCode::Required,
        }
    }

    /// Creates a "min length" validation error.
    pub fn min_length(min: usize, messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.min_length_error)(min, locale),
            code: ErrorCode::MinLength(min),
        }
    }

    /// Creates a "max length" validation error.
    pub fn max_length(max: usize, messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.max_length_error)(max, locale),
            code: ErrorCode::MaxLength(max),
        }
    }

    /// Creates a "pattern" validation error.
    pub fn pattern(
        pattern: impl Into<String>,
        messages: &FormMessages,
        locale: &ars_i18n::Locale,
    ) -> Self {
        Self {
            message: (messages.pattern_error)(locale),
            code: ErrorCode::Pattern(pattern.into()),
        }
    }

    /// Creates a "min" validation error.
    pub fn min(min: f64, messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.min_error)(min, locale),
            code: ErrorCode::Min(min),
        }
    }

    /// Creates a "max" validation error.
    pub fn max(max: f64, messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.max_error)(max, locale),
            code: ErrorCode::Max(max),
        }
    }

    /// Creates an "email" validation error with locale-appropriate message.
    pub fn email(messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.email_error)(locale),
            code: ErrorCode::Email,
        }
    }

    /// Create a step mismatch error.
    pub fn step(step: f64, messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.step_error)(step, locale),
            code: ErrorCode::Step(step),
        }
    }

    /// Create a URL validation error.
    pub fn url(messages: &FormMessages, locale: &ars_i18n::Locale) -> Self {
        Self {
            message: (messages.url_error)(locale),
            code: ErrorCode::Url,
        }
    }

    pub fn custom(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: ErrorCode::Custom(code.into()),
        }
    }

    pub fn server(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: ErrorCode::Server(message.clone()),
            message,
        }
    }

    /// Returns `true` if this error originated from the server or an async validator.
    /// Used by `set_server_errors()` to separate server-sourced errors from client-side ones.
    pub fn is_server(&self) -> bool {
        matches!(&self.code, ErrorCode::Server(_) | ErrorCode::Async(_))
    }
}

/// A collection of validation errors for a single field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Errors(pub Vec<Error>);

impl Errors {
    pub fn new() -> Self { Self(Vec::new()) }

    pub fn push(&mut self, error: Error) {
        self.0.push(error);
    }

    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn len(&self) -> usize { self.0.len() }

    /// Get the first error message.
    pub fn first_message(&self) -> Option<&str> {
        self.0.first().map(|e| e.message.as_str())
    }

    /// Check if a specific code is present.
    pub fn has_code(&self, code: &ErrorCode) -> bool {
        self.0.iter().any(|e| &e.code == code)
    }
}
```

### 2.2 Result

`Result` is a type alias over Rust's standard `Result`, giving every validation
outcome the `?` operator, combinators, and idiomatic `Ok(())`/`Err(errors)`
pattern matching. Domain-specific helpers live on the `ResultExt` extension trait.

```rust
/// The result of validating a field value.
///
/// - `Ok(())` — the value passed validation.
/// - `Err(Errors)` — the value failed with one or more errors.
pub type Result = core::result::Result<(), Errors>;

/// Extension methods for `Result`.
pub trait ResultExt {
    /// Returns the validation errors, if any.
    fn errors(&self) -> Option<&Errors>;

    /// Merges two validation results. If both are invalid, their errors
    /// are combined into one `Err`.
    fn merge(self, other: Result) -> Result;

    /// Returns the first error message, if any.
    fn first_error_message(&self) -> Option<&str>;

    /// Returns a new `Result` with server-sourced and async errors removed.
    /// Preserves client-side validation errors. Returns `Ok(())` if no
    /// errors remain.
    fn without_server_errors(&self) -> Result;
}

impl ResultExt for Result {
    fn errors(&self) -> Option<&Errors> {
        self.as_ref().err()
    }

    fn merge(self, other: Result) -> Result {
        match (self, other) {
            (Ok(()), other) => other,
            (Err(mut e1), Err(e2)) => {
                e1.0.extend(e2.0);
                Err(e1)
            }
            (err, Ok(())) => err,
        }
    }

    fn first_error_message(&self) -> Option<&str> {
        self.errors().and_then(|e| e.first_message())
    }

    fn without_server_errors(&self) -> Result {
        match self {
            Ok(()) => Ok(()),
            Err(errors) => {
                let filtered: Vec<Error> = errors.0.iter()
                    .filter(|e| !matches!(
                        &e.code,
                        ErrorCode::Server(_) | ErrorCode::Async(_)
                    ))
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    Ok(())
                } else {
                    Err(Errors(filtered))
                }
            }
        }
    }
}
```

### 2.3 State

````rust
/// The complete state of a single form field.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Initial field value, stored for reset().
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
    /// cancellation. Incremented on every value change. When an
    /// async validation future completes, the handler compares its captured
    /// generation against the current `validation_generation` — if they differ,
    /// a newer value has been set and the result is stale, so it is discarded.
    ///
    /// **Cancellation pattern:**
    /// ```rust
    /// // On value change:
    /// field.validation_generation += 1;
    /// let gen = field.validation_generation;
    /// spawn(async move {
    ///     let result = validator.validate_async(&value, &ctx).await;
    ///     // In the completion handler:
    ///     if field.validation_generation == gen {
    ///         field.validation = result;
    ///         field.validating = false;
    ///     }
    ///     // else: stale result — discard silently
    /// });
    /// ```
    pub validation_generation: u64,
}

/// A reference to an uploaded file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRef {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

/// The value of a form field.
// Value intentionally has no Default impl — the correct variant depends
// on the field type (Text vs Number vs Date etc.).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    /// Note: `f64` NaN violates PartialEq reflexivity (NaN != NaN). Number fields
    /// SHOULD reject NaN at the validator level. If NaN reaches Value, dirty
    /// tracking via PartialEq will incorrectly report the field as always dirty.
    Number(Option<f64>),
    Bool(bool),
    MultipleText(Vec<String>),
    File(Vec<FileRef>),
    Date(Option<ars_i18n::CalendarDate>),
    Time(Option<ars_i18n::Time>),
    DateRange(Option<ars_i18n::DateRange>),
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self { Value::Text(s) => Some(s), _ => None }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self { Value::Number(n) => *n, _ => None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self { Value::Bool(b) => Some(*b), _ => None }
    }

    pub fn to_string_for_validation(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Number(Some(n)) => n.to_string(),
            Value::Number(None) => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::MultipleText(v) => v.join(","),
            Value::File(v) => v.len().to_string(),
            Value::Date(Some(d)) => d.to_iso8601(),      // "2024-03-15"
            Value::Date(None) => String::new(),
            Value::Time(Some(t)) => t.to_iso8601(),      // "14:30:00"
            Value::Time(None) => String::new(),
            Value::DateRange(Some(r)) => r.to_iso8601(), // "2024-03-15/2024-03-20"
            Value::DateRange(None) => String::new(),
        }
    }
}

impl State {
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
    pub fn show_error(&self) -> bool {
        self.touched && self.validation.is_err()
    }

    /// Whether the field is currently invalid.
    pub fn is_invalid(&self) -> bool {
        self.validation.is_err()
    }

    /// Get the error message to display.
    pub fn error_message(&self) -> Option<&str> {
        if self.show_error() {
            self.validation.first_error_message()
        } else {
            None
        }
    }
}
````

---

## 3. The Validator Trait

### 3.1 Synchronous Validator

````rust
use std::sync::Arc;
use std::fmt;

/// Context available to validators during validation.
///
/// `Clone` is derived because this struct holds only references. However, cloning
/// only copies the borrows — it does not extend their lifetimes. If you need an
/// owned copy that outlives the borrow scope (e.g., for async validation), use
/// `fn snapshot(&self) -> OwnedContext` instead.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// The name of the field being validated.
    pub field_name: &'a str,

    /// All current form values (for cross-field validation).
    pub form_values: &'a std::collections::BTreeMap<String, Value>,

    /// The current locale (for locale-aware messages).
    pub locale: Option<&'a ars_i18n::Locale>,
}

/// An owned version of `Context` that can outlive the borrow scope.
/// Use `Context::snapshot()` to create one — e.g., for async validation
/// where the future must own its context.
#[derive(Clone, Debug)]
pub struct OwnedContext {
    pub field_name: String,
    pub form_values: std::collections::BTreeMap<String, Value>,
    pub locale: Option<ars_i18n::Locale>,
}

impl OwnedContext {
    /// Convert back to a borrowed `Context<'_>` for passing to
    /// `AsyncValidator::validate_async()` and similar APIs.
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
    ///
    /// # Example
    /// ```rust
    /// let ctx = Context::standalone("email");
    /// let result = my_validator.validate(&Value::Text(input), &ctx);
    /// ```
    pub fn standalone(field_name: &'a str) -> Self {
        use std::sync::LazyLock;
        static EMPTY_MAP: LazyLock<std::collections::BTreeMap<String, Value>> =
            LazyLock::new(std::collections::BTreeMap::new);
        Self {
            field_name,
            form_values: &EMPTY_MAP,
            locale: None,
        }
    }
}

/// A synchronous field validator.
///
/// Validators are always `Send + Sync`, so the same trait object shape works
/// on every target without cfg-gated API differences. Custom validators created via
/// `FnValidator` or `ValidatorsBuilder::add()` automatically pick up the same bounds.
pub trait Validator: Send + Sync {
    fn validate(&self, value: &Value, ctx: &Context) -> Result;
}

/// A type-erased synchronous validator.
/// Uses `Arc` instead of `Box` for cheap cloning across reactive signals.
pub type BoxedValidator = Arc<dyn Validator + Send + Sync>;

/// Helper to wrap a Validator into the standard shared pointer type.
pub fn boxed_validator(v: impl Validator + 'static) -> BoxedValidator {
    Arc::new(v)
}

pub type BoxedAsyncValidator = Arc<dyn AsyncValidator + Send + Sync>;
````

### 3.2 Built-in Validators

```rust
/// Fails if the value is empty.
pub struct RequiredValidator {
    pub message: Option<String>,
}

impl Validator for RequiredValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let is_empty = match value {
            Value::Text(s) => s.trim().is_empty(),
            Value::Number(None) => true,
            Value::Number(Some(_)) => false,
            Value::Bool(false) => true,
            Value::Bool(true) => false,
            Value::MultipleText(v) => v.is_empty(),
            Value::File(v) => v.is_empty(),
            Value::Date(d) => d.is_none(),
            Value::Time(t) => t.is_none(),
            Value::DateRange(r) => r.is_none(),
        };
        if is_empty {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
            Err(Errors(vec![
                self.message.as_ref()
                    .map(|m| Error { message: m.clone(), code: ErrorCode::Required })
                    .unwrap_or_else(|| Error::required(&FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

/// Default locale for built-in validators when no locale is provided via Context.
///
/// **Design decision:** Built-in validators (Required, MinLength, MaxLength, Pattern, etc.)
/// fall back to English ("en") when `Context.locale` is `None`. This produces
/// correct English validation messages but may produce incorrect pluralization for other
/// languages. To enable localized validation messages, set `locale` on `Context::new()`
/// or pass a `Locale` to individual `Context` instances.
static DEFAULT_VALIDATOR_LOCALE: std::sync::LazyLock<ars_i18n::Locale> =
    std::sync::LazyLock::new(|| ars_i18n::Locale::parse("en").expect("valid locale"));

pub struct MinLengthValidator { pub min: usize, pub message: Option<String> }

impl Validator for MinLengthValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let s = value.to_string_for_validation();
        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
        if s.chars().count() < self.min {
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::MinLength(self.min) })
                    .unwrap_or_else(|| Error::min_length(self.min, &FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

pub struct MaxLengthValidator { pub max: usize, pub message: Option<String> }

impl Validator for MaxLengthValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let s = value.to_string_for_validation();
        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
        if s.chars().count() > self.max {
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::MaxLength(self.max) })
                    .unwrap_or_else(|| Error::max_length(self.max, &FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

pub struct MinValidator { pub min: f64, pub message: Option<String> }

impl Validator for MinValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let n = match value.as_number() {
            Some(n) => n,
            None => return Ok(()), // Let RequiredValidator handle None
        };
        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
        if n < self.min {
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::Min(self.min) })
                    .unwrap_or_else(|| Error::min(self.min, &FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

pub struct MaxValidator { pub max: f64, pub message: Option<String> }

impl Validator for MaxValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let n = match value.as_number() {
            Some(n) => n,
            None => return Ok(()),
        };
        let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
        if n > self.max {
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::Max(self.max) })
                    .unwrap_or_else(|| Error::max(self.max, &FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

/// Regex-based pattern validator.
///
/// The `Regex` is compiled once at construction time and cached in `compiled`.
/// This avoids re-compiling on every `validate()` call, which was
/// the previous behavior of the per-call `regex_matches()` helper.
///
/// Construction panics if the pattern is invalid — callers should validate
/// patterns at build time (e.g., in `ValidatorsBuilder::pattern()`).
pub struct PatternValidator {
    /// The cached, compiled regex (anchored with `^(?:...)$`).
    pub compiled: regex::Regex,
    /// Original pattern string, retained for error codes.
    pub pattern: String,
    pub message: Option<String>,
}

impl PatternValidator {
    /// Create a new `PatternValidator`, compiling the regex eagerly.
    ///
    /// # Panics
    /// Panics if `pattern` exceeds 1024 bytes or is not a valid regex.
    pub fn new(pattern: impl Into<String>, message: Option<String>) -> Self {
        let pattern = pattern.into();
        const MAX_PATTERN_LEN: usize = 1024;
        assert!(
            pattern.len() <= MAX_PATTERN_LEN,
            "PatternValidator: pattern exceeds {} bytes", MAX_PATTERN_LEN
        );
        let anchored = format!("^(?:{})$", pattern);
        let compiled = regex::Regex::new(&anchored)
            .expect("PatternValidator: invalid regex pattern");
        Self { compiled, pattern, message }
    }
}

impl Validator for PatternValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let s = value.to_string_for_validation();
        if s.is_empty() { return Ok(()); } // Let Required handle

        if !self.compiled.is_match(&s) {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::Pattern(self.pattern.clone()) })
                    // Do not expose raw regex to users — it is incomprehensible
                    // in any language. Callers should provide a custom `message`.
                    .unwrap_or_else(|| {
                        Error::pattern(self.pattern.clone(), &FormMessages::default(), locale)
                    })
            ]))
        } else {
            Ok(())
        }
    }
}

// `regex_matches` removed — `PatternValidator` caches the compiled `Regex`
// at construction time; no per-call compilation needed.
```

> **Security note:** `PatternValidator` MUST use the `regex` crate (RE2-class finite automaton, no catastrophic backtracking). Do NOT substitute `fancy-regex` or other backtracking engines without adding a compilation timeout. The compiled `Regex` SHOULD be cached in the `PatternValidator` struct (constructed once at creation, not per validation call) to avoid repeated compilation cost. If the pattern comes from user input, apply a `regex_size_limit` (e.g., 1MB compiled size limit via `RegexBuilder::size_limit()`).

````rust

pub struct EmailValidator { pub message: Option<String> }

impl Validator for EmailValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let s = value.to_string_for_validation();
        if s.is_empty() { return Ok(()); }

        // Simple email validation: must contain @ and have something before and after
        let valid = s.contains('@') && {
            let parts: Vec<&str> = s.splitn(2, '@').collect();
            parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.')
        };

        if !valid {
            let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
            Err(Errors(vec![
                self.message.clone()
                    .map(|m| Error { message: m, code: ErrorCode::Email })
                    .unwrap_or_else(|| Error::email(&FormMessages::default(), locale))
            ]))
        } else {
            Ok(())
        }
    }
}

/// Validates that a numeric value is a multiple of the given step,
/// relative to the step base (typically `min` or `0`).
pub struct StepValidator {
    /// The step increment (e.g., `0.01` for currency, `1.0` for integers).
    pub step: f64,
    /// The base value from which steps are counted. Default: `0.0`.
    pub step_base: f64,
    /// Optional custom error message, overriding the default.
    pub message: Option<String>,
}

impl StepValidator {
    pub fn new(step: f64) -> Self {
        Self { step, step_base: 0.0, message: None }
    }

    pub fn with_base(mut self, base: f64) -> Self {
        self.step_base = base;
        self
    }
}

impl Validator for StepValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        if let Some(num) = value.as_number() {
            let remainder = ((num - self.step_base) % self.step).abs();
            if remainder > f64::EPSILON && (self.step - remainder) > f64::EPSILON {
                if let Some(ref msg) = self.message {
                    return Err(Errors(vec![
                        Error::custom("step", msg.clone())
                    ]));
                }
                let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
                return Err(Errors(vec![
                    Error::step(self.step, &FormMessages::default(), locale)
                ]));
            }
        }
        Ok(())
    }
}

/// Validates that a string value is a well-formed URL.
/// Uses the WHATWG URL Standard parsing algorithm.
pub struct UrlValidator {
    /// Optional custom error message, overriding the default.
    pub message: Option<String>,
}

impl UrlValidator {
    pub fn new() -> Self {
        Self { message: None }
    }
}

impl Validator for UrlValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        if let Some(s) = value.as_text() {
            if !s.is_empty() && !is_valid_url(s) {
                if let Some(ref msg) = self.message {
                    return Err(Errors(vec![
                        Error::custom("url", msg.clone())
                    ]));
                }
                let locale = ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE);
                return Err(Errors(vec![
                    Error::url(&FormMessages::default(), locale)
                ]));
            }
        }
        Ok(())
    }
}

/// Check whether `s` is a valid URL per the WHATWG URL Standard.
/// In browser targets, delegates to the `URL` constructor via `web_sys`.
/// In non-browser targets, performs a basic scheme + authority check.
fn is_valid_url(s: &str) -> bool {
    // Minimal validation: must have a scheme followed by "://" and a non-empty authority.
    // Full WHATWG parsing is delegated to the platform URL parser at runtime.
    s.find("://").map_or(false, |pos| s.len() > pos + 3)
}

/// Validator created from a closure.
pub struct FnValidator<F: Fn(&Value, &Context) -> Result + Send + Sync> {
    pub f: F,
}
impl<F: Fn(&Value, &Context) -> Result + Send + Sync> Validator for FnValidator<F> {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        (self.f)(value, ctx)
    }
}

/// ## Heterogeneous Storage Pattern
///
/// To store validators in `Vec<BoxedValidator>`, wrap closures via `FnValidator`:
/// ```rust
/// let validators: Vec<BoxedValidator> = vec![
///     FnValidator::new(|val, _ctx| { /* ... */ }).boxed(),
/// ];
/// ```
/// The `Validator` trait is object-safe. `FnValidator` erases the closure type.
/// Use `.boxed()` to create the standard shared smart pointer type.
impl<F: Fn(&Value, &Context) -> Result + Send + Sync + 'static> FnValidator<F> {
    pub fn new(f: F) -> Self { Self { f } }
    pub fn boxed(self) -> BoxedValidator { boxed_validator(self) }
}
````

### 3.3 Validator Builder (Fluent API)

````rust
/// Build a validator chain fluently.
///
/// # Example
/// ```rust
/// let validator = Validators::new()
///     .required()
///     .min_length(3)
///     .max_length(50)
///     .pattern(r"^[a-zA-Z0-9_]+$")
///     .build();
/// ```
pub struct ValidatorsBuilder {
    validators: Vec<BoxedValidator>,
}

impl ValidatorsBuilder {
    pub fn new() -> Self {
        Self { validators: Vec::new() }
    }

    /// Add a validator. The `Validator` trait itself always carries
    /// `Send + Sync`, so the bound here is simply `Validator + 'static`
    /// and the conversion to `BoxedValidator` compiles on all platforms
    /// without additional where clauses.
    pub fn add(mut self, v: impl Validator + 'static) -> Self {
        let v: Box<dyn Validator + Send + Sync> = Box::new(v);
        self.validators.push(Arc::from(v));
        self
    }

    pub fn required(self) -> Self {
        self.add(RequiredValidator { message: None })
    }

    pub fn required_msg(self, msg: impl Into<String>) -> Self {
        self.add(RequiredValidator { message: Some(msg.into()) })
    }

    pub fn min_length(self, n: usize) -> Self {
        self.add(MinLengthValidator { min: n, message: None })
    }

    pub fn max_length(self, n: usize) -> Self {
        self.add(MaxLengthValidator { max: n, message: None })
    }

    pub fn min(self, n: f64) -> Self {
        self.add(MinValidator { min: n, message: None })
    }

    pub fn max(self, n: f64) -> Self {
        self.add(MaxValidator { max: n, message: None })
    }

    pub fn pattern(self, regex: impl Into<String>) -> Self {
        self.add(PatternValidator::new(regex, None))
    }

    pub fn email(self) -> Self {
        self.add(EmailValidator { message: None })
    }

    /// Add a step validation rule.
    pub fn step(self, step: f64) -> Self {
        self.add(StepValidator::new(step))
    }

    /// Add a step validation rule with a custom base value.
    pub fn step_with_base(self, step: f64, base: f64) -> Self {
        self.add(StepValidator::new(step).with_base(base))
    }

    /// Add URL validation.
    pub fn url(self) -> Self {
        self.add(UrlValidator { message: None })
    }

    // NOTE: `any()` moved to `AnyValidator::new()` — see below.

    pub fn custom<F>(self, f: F) -> Self
    where
        F: Fn(&Value, &Context) -> Result + Send + Sync + 'static,
    {
        self.add(FnValidator { f })
    }

    /// Run all validators and collect ALL errors.
    pub fn build(self) -> ChainValidator {
        ChainValidator { validators: self.validators, stop_on_first: false }
    }

    /// Run validators and stop at the first error.
    pub fn build_first_fail(self) -> ChainValidator {
        ChainValidator { validators: self.validators, stop_on_first: true }
    }
}

pub type Validators = ValidatorsBuilder;

/// Runs multiple validators and combines their results.
pub struct ChainValidator {
    validators: Vec<BoxedValidator>,
    stop_on_first: bool,
}

impl Validator for ChainValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let mut all_errors = Errors::new();

        for validator in &self.validators {
            match validator.validate(value, ctx) {
                Ok(()) => {}
                Err(errors) => {
                    all_errors.0.extend(errors.0);
                    if self.stop_on_first { break; }
                }
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
    /// Wrap this chain in the platform-correct smart pointer (`Arc` on native, `Rc` on WASM).
    pub fn boxed(self) -> BoxedValidator { boxed_validator(self) }
}
````

---

## 4. Async Validation

```rust
use core::pin::Pin;
use std::collections::BTreeMap;
use ars_i18n::Locale;

/// Async validation trait.
///
/// Async validators are always `Send + Sync`, and returned futures are always
/// `Send`, so the same trait object shape works on every target.
pub trait AsyncValidator: Send + Sync {
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>>;
}

/// Wrap a closure as an async validator.
pub struct AsyncFnValidator<F> {
    pub f: F,
}

impl<F, Fut> AsyncValidator for AsyncFnValidator<F>
where
    F: Fn(String, OwnedContext) -> Fut + Send + Sync,
    Fut: Future<Output = Result> + Send + 'static,
{
    fn validate_async<'a>(
        &'a self,
        value: &'a Value,
        ctx: &'a Context<'a>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'a>> {
        let text = value.to_string_for_validation();
        let owned_ctx = ctx.snapshot();
        let fut = (self.f)(text, owned_ctx);
        Box::pin(fut)
    }
}

/// Debounced async validator — waits for `delay` of inactivity before calling.
pub struct DebouncedAsyncValidator {
    pub validator: Arc<dyn AsyncValidator>,
    pub delay_ms: u32,
    /// Adapter-provided callback that spawns an async future to completion.
    /// On native: wraps `tokio::spawn`; on WASM: wraps `wasm_bindgen_futures::spawn_local`.
    /// The callback takes ownership of the `OwnedContext` and the future,
    /// avoiding lifetime issues with borrowed `Context`.
    pub spawn_async_validation: Arc<dyn Fn(Arc<dyn AsyncValidator>, Value, OwnedContext) + Send + Sync>,
    /// Handle to the currently pending debounce timer, if any.
    /// Used to cancel the previous timer when new input arrives.
    pending_timer: Option<TimerHandle>,
}

impl DebouncedAsyncValidator {
    /// Cancel any pending debounce timer and start a new one.
    /// After `delay_ms`, delegates to the inner `AsyncValidator::validate_async`.
    /// The adapter provides `TimerHandle` via its platform timer abstraction
    /// (e.g., `setTimeout` on WASM, `tokio::time::sleep` on native).
    ///
    /// **Design note:** The `spawn_async_validation` callback receives owned data
    /// (validator, value, context) rather than a pre-built future. This avoids the
    /// lifetime problem where `validate_async` returns `Future + 'a` tied to a
    /// borrowed `Context<'a>` — the callback constructs the
    /// `Context` from the owned data inside the spawned task, where the
    /// borrow can live for the task's duration.
    pub fn validate_debounced(
        &mut self,
        value: &Value,
        name: &str,
        form_values: &BTreeMap<String, Value>,
        locale: Option<&Locale>,
        spawn_timer: impl FnOnce(u32, Box<dyn FnOnce()>) -> TimerHandle,
    ) {
        // Cancel previous pending validation
        if let Some(handle) = self.pending_timer.take() {
            handle.cancel();
        }
        let validator = self.validator.clone();
        let value = value.clone();
        let name = name.to_string();
        let owned_ctx = OwnedContext {
            field_name: name.clone(),
            form_values: form_values.clone(),
            locale: locale.cloned(),
        };
        let spawn_async = self.spawn_async_validation.clone();
        self.pending_timer = Some(spawn_timer(self.delay_ms, Box::new(move || {
            // After delay, spawn the async validator. The spawn callback takes
            // ownership of all data and constructs the Context internally,
            // ensuring the future's lifetime is satisfied.
            spawn_async(validator, value, owned_ctx);
        })));
    }
}

/// Timer handle returned by the adapter's platform timer abstraction.
/// On WASM, wraps `setTimeout`; on native, wraps `tokio::time::sleep` or similar.
pub struct TimerHandle {
    #[cfg(target_arch = "wasm32")]
    cancel_fn: Box<dyn FnOnce()>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel_fn: Box<dyn FnOnce() + Send + Sync>,
}

impl TimerHandle {
    #[cfg(target_arch = "wasm32")]
    pub fn new(cancel_fn: Box<dyn FnOnce()>) -> Self {
        Self { cancel_fn }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(cancel_fn: Box<dyn FnOnce() + Send + Sync>) -> Self {
        Self { cancel_fn }
    }
    pub fn cancel(self) {
        (self.cancel_fn)()
    }
}

/// `OwnedContext` is defined in §3.1 above. The `as_ref()` method
/// converts it back to a borrowed `Context<'_>` for passing to
/// `AsyncValidator::validate_async()`.
```

---

## 5. Form Context

> **Design note:** `Context` is a plain data structure, not a `Machine` implementor. It holds mutable form state (field registry, validation results, dirty/touched tracking). The `form_submit::Machine` in §8 embeds `Context` in its `Context` type and drives the submission lifecycle as a proper state machine. The `form::Machine` in §14 manages its own simplified context; integration with `Context` is done at the adapter layer via context propagation. Direct mutation of `Context` is valid within Machine `apply` closures.
>
> **Decision matrix — which approach to use:**
>
> | Scenario                                       | Use                         | Why                                                                  |
> | ---------------------------------------------- | --------------------------- | -------------------------------------------------------------------- |
> | Full form with validation + async submit       | `form_submit::Machine` (§8) | Full lifecycle: Idle → Validating → Submitting → Success/Failed      |
> | Simple form without validation states          | `form::Machine` (§14)       | Lightweight: just collects fields and submits                        |
> | Programmatic submission from outside a Machine | `Context.submit()` (§5.1)   | Direct call from adapter hooks when no state machine drives the form |

### 5.1 `Context`

````rust
use std::{collections::BTreeMap, fmt};
use indexmap::IndexMap; // indexmap is a required dependency of ars-forms (preserves field registration order; add `indexmap = "2"` to ars-forms/Cargo.toml)
use ars_i18n::Locale;

/// The central state for a form.
#[derive(Clone)]
pub struct Context {
    /// State of each registered field.
    /// Uses `IndexMap` (insertion-ordered) so that "focus first invalid field"
    /// iterates fields in DOM registration order, not alphabetical order.
    /// A `BTreeMap` would sort alphabetically, causing focus to jump to the
    /// wrong field (e.g., "address" before "name" even if "name" appears first in the DOM).
    pub fields: IndexMap<String, State>,

    /// Whether the form is currently being submitted.
    pub is_submitting: bool,

    /// Whether the form has been submitted at least once.
    pub is_submitted: bool,

    /// Externally injected server-side errors (e.g., from API response).
    pub server_errors: BTreeMap<String, Vec<String>>,

    /// The current locale, injected from ArsProvider on construction.
    /// Set `locale` to enable localized validation messages. Defaults to English when `None`.
    pub locale: Option<Locale>,

    /// Validation trigger mode.
    pub validation_mode: Mode,

    /// Registered validators per field.
    // BTreeMap is used intentionally: validators are accessed by field name (keyed lookup),
    // not iterated in order. Registration order is maintained by `fields: IndexMap`.
    #[doc(hidden)]
    validators: BTreeMap<String, BoxedValidator>,

    /// Async validators per field. Uses `BoxedAsyncValidator` for shared ownership.
    #[doc(hidden)]
    async_validators: BTreeMap<String, BoxedAsyncValidator>,

    /// Registry of cross-field validators indexed by the field they validate.
    /// Each entry maps a field name to its `CrossFieldValidator` whose `depends_on`
    /// may reference other fields. When `on_change(name)` fires, the registry is
    /// scanned for validators whose `depends_on` includes `name`.
    cross_field_registry: BTreeMap<String, Vec<CrossFieldValidator>>,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("fields", &self.fields)
            .field("is_submitting", &self.is_submitting)
            .field("is_submitted", &self.is_submitted)
            .field("server_errors", &self.server_errors)
            .field("locale", &self.locale)
            .field("validation_mode", &self.validation_mode)
            .field("validators", &format_args!("<{} validators>", self.validators.len()))
            .field("async_validators", &format_args!("<{} async_validators>", self.async_validators.len()))
            .field("cross_field_registry", &format_args!("<{} cross-field entries>", self.cross_field_registry.len()))
            .finish()
    }
}

/// When field-level validation is triggered.
///
/// **Note:** Submit-time validation is always performed by `Context::submit()`
/// regardless of these settings. There is no `on_submit` flag because skipping
/// validation at submit time is never valid — `submit()` unconditionally calls
/// `validate_all()` before dispatching. These flags only control *field-level*
/// validation timing (blur, change, input).
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
    pub fn on_blur_revalidate() -> Self {
        Self { on_blur: true, revalidate_on_change: true, ..Default::default() }
    }

    /// Validate on every change.
    pub fn on_change() -> Self {
        Self { on_change: true, ..Default::default() }
    }

    /// Only validate on submit (no field-level validation).
    pub fn on_submit() -> Self {
        Self::default()
    }
}

// ── Extended Validation: Composition, Cross-Field, Timing, Server Errors ────
//
// ### Validator Composition (AND / OR)
//
// The `ChainValidator` (§3.3) runs validators in sequence (AND logic — all must
// pass). For OR logic (at least one must pass), use `AnyValidator`:

/// Passes if ANY inner validator returns `Ok(())`. Collects errors only if all fail.
pub struct AnyValidator {
    pub validators: Vec<BoxedValidator>,
}

impl AnyValidator {
    /// Build an OR-combination validator: passes if ANY inner validator returns `Ok(())`.
    pub fn new(validators: Vec<BoxedValidator>) -> Self {
        Self { validators }
    }

    pub fn boxed(self) -> BoxedValidator { boxed_validator(self) }
}

impl Validator for AnyValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        let mut all_errors = Errors::new();
        for v in &self.validators {
            match v.validate(value, ctx) {
                Ok(()) => return Ok(()),
                Err(errs) => all_errors.0.extend(errs.0),
            }
        }
        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

// The builder supports both:
//
// ```rust
// // AND: all must pass (default chain)
// let v = Validators::new().required().min_length(3).email().build();
//
// // OR: at least one must pass
// let v = AnyValidator::new(vec![
//     boxed_validator(EmailValidator { message: None }),
//     boxed_validator(PatternValidator::new(r"^\+\d+$", None)),
// ]);
// ```

// ### Cross-Field Validation
//
// The `Context` already provides `form_values` for cross-field access.
// For ergonomic cross-field rules, use `CrossFieldValidator`:

/// Validates a field using values from other fields in the form.
///
/// # Example: confirm password
/// ```rust
/// let confirm = CrossFieldValidator {
///     depends_on: vec!["password".into()],
///     validate_fn: Arc::new(|value, ctx| {
///         let password = ctx.form_values.get("password")
///             .and_then(|v| v.as_text());
///         let confirm = value.as_text().unwrap_or_default();
///         // If password field is missing or non-text, fail validation (mismatch).
///         if password.map(|p| p == confirm).unwrap_or(false) {
///             Ok(())
///         } else {
///             Err(Errors(vec![Error::custom("confirm_match", "Passwords do not match")]))
///         }
///     }),
/// };
/// ```
// Type alias avoids clippy::type_complexity on the struct field.
type CrossFieldValidateFn =
    Arc<dyn Fn(&Value, &Context) -> Result + Send + Sync>;

#[derive(Clone)]
pub struct CrossFieldValidator {
    /// Names of fields this validator reads from. When any field in this list
    /// changes (via `on_change`), Context automatically re-validates the
    /// field this validator is registered on. This enables reactive cross-field
    /// validation (e.g., "confirm password" re-validates when "password" changes).
    pub depends_on: Vec<String>,
    pub validate_fn: CrossFieldValidateFn,
}

impl Validator for CrossFieldValidator {
    fn validate(&self, value: &Value, ctx: &Context) -> Result {
        (self.validate_fn)(value, ctx)
    }

}

// Note: When `Context::on_change(field_name)` is called, Context checks
// all registered `CrossFieldValidator` instances on other fields. If any validator's
// `depends_on` list contains `field_name`, the owning field is re-validated.
// This reverse-dependency lookup is O(fields × validators) but is acceptable since
// cross-field validators are rare (typically 1-3 per form).

// ### Server-Side Error Injection
//
// `Context.server_errors` (already defined) stores externally injected errors.
// The full workflow:
//
// 1. Form submits to server.
// 2. Server returns field-level errors as `BTreeMap<String, Vec<String>>`.
// 3. Adapter calls `Context::set_server_errors(errors)`:

// Server errors are displayed identically to client-side errors in the UI.
// When the user modifies a field, `on_change()` clears that field's server
// error. See the canonical `set_server_errors()` implementation in §5.1.

impl Context {
    pub fn new(mode: Mode) -> Self {
        Self {
            fields: IndexMap::new(),
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
    /// Re-registration replaces both validator types. To keep an existing async
    /// validator when re-registering, pass the same async_validator again.
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
    /// When any field listed in `validator.depends_on` changes, `field_name` is re-validated.
    pub fn register_cross_field_validator(&mut self, field_name: &str, validator: CrossFieldValidator) {
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

            let needs_validation = self.validation_mode.on_change
                || (self.validation_mode.revalidate_on_change && field.validation.is_err());
            needs_validation
        } else {
            return;
        };

        // Clear server error when field changes.
        self.server_errors.remove(name);

        // If validation won't re-run (e.g., on_submit mode), clear only server-sourced
        // errors from the field's validation result. Client-side errors from the most
        // recent validate_all() are preserved — they remain visible until the next
        // submission triggers re-validation. Otherwise run_field_validation() will set
        // the correct state.
        if should_validate {
            // run_field_validation() handles both the named field's own validators and
            // cross-field dependency re-validation internally.
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
    /// Only triggers validation when `validation_mode.on_input` is true.
    /// Unlike `on_change`, this does NOT mark the field as `dirty` — dirty tracking
    /// requires `on_change` (which fires on blur or commit). Adapters using `on_input`
    /// validation mode MUST also call `on_change` on blur to properly update dirty state.
    /// Clears server errors for the field (policy: user modification clears server errors).
    pub fn on_input(&mut self, name: &str, value: Value) {
        if let Some(field) = self.fields.get_mut(name) {
            field.validation_generation += 1;
            field.value = value;
        } else {
            return;
        }
        // Clear server errors when user modifies a field (matches on_change behavior).
        self.server_errors.remove(name);
        // Design note: Unlike on_change(), on_input() does NOT trigger cross-field
        // re-validation. This is intentional — on_input fires on every keystroke,
        // and cross-field validators may be expensive (they clone all form values).
        // Cross-field dependencies are re-validated on on_change() (typically blur).
        if self.validation_mode.on_input {
            self.run_field_validation(name);
        } else if let Some(field) = self.fields.get_mut(name) {
            field.validation = field.validation.without_server_errors();
        }
    }

    /// Validate a single field synchronously, building a cross-field context
    /// by cloning all current values.
    ///
    /// **Performance note:** This clones all field values into a `BTreeMap` on every call.
    /// For `revalidate_on_change` mode, this runs on every keystroke. Implementations may
    /// optimize by caching the values map and invalidating on field value changes, or by
    /// checking `validator.needs_form_values()` before building the context.
    pub fn validate_field(&mut self, name: &str) -> Result {
        self.run_field_validation(name);
        self.fields.get(name).map_or(Ok(()), |f| f.validation.clone())
    }

    /// Validate all fields. Returns true if the form is valid.
    pub fn validate_all(&mut self) -> bool {
        let names: Vec<String> = self.fields.keys().cloned().collect();
        let mut valid = true;
        for name in names {
            self.run_field_validation(&name);
            if self.fields.get(&name).is_some_and(|f| f.validation.is_err()) {
                valid = false;
            }
        }
        valid
    }

    /// Internal: runs validation for a field and stores the result in
    /// `field.validation`. Does not return the result — callers that need
    /// the outcome read it from `self.fields` afterward.
    fn run_field_validation(&mut self, name: &str) {
        let value = match self.fields.get(name) {
            Some(f) => f.value.clone(),
            None => return,
        };

        // Design note: Context.form_values uses BTreeMap (not IndexMap) because
        // validators are pure functions that should not depend on field registration order.
        // The per-call allocation cost is acceptable because cross-field validation is rare
        // and the typical form has <20 fields.
        let all_values: BTreeMap<String, Value> = self.fields.iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect();

        let ctx = Context {
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
                && validators.iter().any(|cv| cv.depends_on.contains(&name.to_string()))
            {
                let target_value = match self.fields.get(target_name.as_str()) {
                    Some(f) => f.value.clone(),
                    None => continue,
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

    /// Inject server-side errors returned from an API call.
    ///
    /// # Example
    /// ```
    /// // After a failed form submission:
    /// let errors = [("email", vec!["Email already in use"])];
    /// ctx.set_server_errors(errors.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect());
    /// ```
    pub fn set_server_errors(&mut self, errors: BTreeMap<String, Vec<String>>) {
        // Merge server errors with existing client-side validation errors.
        // Old server errors are removed first, then new server errors are appended.
        // This preserves client-side errors from validate_all() while updating
        // server-sourced errors.
        for (name, messages) in &errors {
            if let Some(field) = self.fields.get_mut(name) {
                field.touched = true; // Show errors immediately
                let server_errs: Vec<Error> = messages.iter()
                    .map(|m| Error::server(m))
                    .collect();
                // Merge: keep existing client-side errors, replace server errors
                let mut client_errs: Vec<Error> = match &field.validation {
                    Err(Errors(errs)) => {
                        errs.iter().filter(|e| !e.is_server()).cloned().collect()
                    }
                    _ => vec![],
                };
                client_errs.extend(server_errs);
                field.validation = Err(Errors(client_errs));
            }
        }
        self.server_errors = errors;
    }

    /// Restores all fields to their initial values and clears metadata
    /// (touched, dirty, validation state). Adapters should observe the
    /// updated `field.value` and sync their reactive signals accordingly.
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

    /// Performs direct synchronous validation and submission without the form_submit::Machine.
    /// When using `form_submit::Machine`, do NOT call this method — the machine
    /// manages the submission lifecycle via state transitions. This method is for
    /// simple forms that don't need a full state machine.
    /// **Note:** `is_submitting` is only meaningful for the duration of the synchronous
    /// `handler` call. For async submission, use `form_submit::Machine` (§8) or
    /// `form::Machine` (§14), which properly manage the Submitting state across async boundaries.
    /// Note: adapter wrappers may require the submit handler closure to be `Send`
    /// so it can be used uniformly across targets and runtimes.
    /// Submit the form with a synchronous handler.
    /// For fallible/async handlers, use `form_submit::Machine` (§8) which provides
    /// `SubmitError` event handling and proper async state management.
    ///
    /// **Note:** `collect_form_data()` includes all fields regardless of disabled state.
    /// If disabled fields must be excluded, the caller (typically the adapter layer)
    /// should filter `Data.fields` before processing. See §15 for the full contract.
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

    fn collect_form_data(&self) -> Data {
        Data {
            fields: self.fields.iter()
                .map(|(k, v)| (k.clone(), v.value.clone()))
                .collect(),
        }
    }

    fn collect_all_errors(&self) -> Errors {
        let mut all = Errors::new();
        for field in self.fields.values() {
            if let Some(errors) = field.validation.errors() {
                all.0.extend(errors.0.clone());
            }
        }
        all
    }

    /// Get field state by name.
    pub fn field(&self, name: &str) -> Option<&State> {
        self.fields.get(name)
    }

    /// Get field state mutably.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut State> {
        self.fields.get_mut(name)
    }

    /// Whether the form is currently valid (no fields have errors).
    pub fn is_valid(&self) -> bool {
        self.fields.values().all(|f| f.validation.is_ok())
    }

    /// Whether any field has been touched.
    pub fn is_dirty(&self) -> bool {
        self.fields.values().any(|f| f.dirty)
    }

    /// Whether any registered fields have async validators pending.
    /// Used by form_submit::Machine to decide between sync and async validation paths.
    pub fn has_async_validators(&self) -> bool {
        !self.async_validators.is_empty()
    }

    /// Collect all async validators with their field names for batch execution.
    /// The adapter spawns these concurrently and sends ValidationPassed/Failed on completion.
    pub fn collect_async_validators(&self) -> Vec<(String, BoxedAsyncValidator)> {
        self.async_validators.iter()
            .map(|(name, v)| (name.clone(), v.clone()))
            .collect()
    }

    /// Register an async validator for a field.
    ///
    /// **Precondition:** The field identified by `name` should already be registered
    /// via `register()`. If the field does not exist, the validator is stored but
    /// never executed (no `State` to validate against).
    pub fn register_async_validator(&mut self, name: impl Into<String>, validator: BoxedAsyncValidator) {
        let name = name.into();
        debug_assert!(
            self.fields.contains_key(&name),
            "register_async_validator: field '{}' not registered — validator will never run",
            name
        );
        self.async_validators.insert(name, validator);
    }
}

/// Collected form data passed to submit handler.
/// Uses `IndexMap` to preserve field registration order (matching Context.fields).
///
/// `Default` produces an empty `Data` with no fields.
/// All `get_*` methods return `None` on a default-constructed instance.
#[derive(Clone, Debug, Default)]
pub struct Data {
    pub fields: IndexMap<String, Value>,
}

impl Data {
    pub fn get_text(&self, name: &str) -> Option<&str> {
        self.fields.get(name).and_then(|v| v.as_text())
    }

    pub fn get_number(&self, name: &str) -> Option<f64> {
        self.fields.get(name).and_then(|v| v.as_number())
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.fields.get(name).and_then(|v| v.as_bool())
    }
}
````

> **Note:** `reset()` restores all fields to their initial values (stored at registration time in `State::initial_value`) and clears metadata (touched, dirty, validation state).
>
> **Serialization:** Data provides structured field data. Serialization to HTTP formats (JSON, form-urlencoded, multipart) is the consumer's responsibility. The adapter may provide helper methods like `to_json()` or `to_form_urlencoded()`.

---

## 6. Field Association (IDs for ARIA)

### 6.1 Descriptors

Each form field consists of multiple related elements that must be linked via ARIA attributes:

- `<label for="inputId">` links label to input
- `<input id="inputId" aria-describedby="descId errorId">` links to description and error
- `<p id="descId">` description text
- `<p id="errorId">` error message

```rust
/// NOTE: Descriptors is a lower-level utility for manual ARIA wiring.
/// When using the Field component (section 13), Descriptors is not needed —
/// the Field component handles all ID generation and ARIA linkage internally
/// via field::Api.
///
/// IDs for all elements of a form field.
#[derive(Clone, Debug)]
pub struct Descriptors {
    pub root_id: String,
    pub label_id: String,
    pub input_id: String,
    pub description_id: String,
    pub error_id: String,
}

impl Descriptors {
    /// Generate IDs from a form ID and field name.
    pub fn new(form_id: &str, field_name: &str) -> Self {
        let base = format!("{}-{}", form_id, field_name);
        Self {
            root_id: base.clone(),
            label_id: format!("{}-label", base),
            input_id: format!("{}-input", base),
            description_id: format!("{}-desc", base),
            error_id: format!("{}-error", base),
        }
    }

    /// Compute aria-describedby for the input element.
    /// Includes description_id always; error_id only when field has an error to show.
    pub fn aria_describedby(&self, field: &State, has_description: bool) -> Option<String> {
        let mut ids = Vec::new();

        if has_description {
            ids.push(self.description_id.clone());
        }

        if field.show_error() {
            ids.push(self.error_id.clone());
        }

        if ids.is_empty() { None } else { Some(ids.join(" ")) }
    }

    /// Compute all ARIA attributes for the input element.
    ///
    /// This is the canonical touch-gated ARIA system: `aria-invalid` and
    /// `aria-errormessage` are only set when `field.show_error()` returns `true`
    /// (i.e., the field is both touched and invalid). `field::Api` defers
    /// to this method for all input ARIA attribute computation.
    ///
    /// When `aria_invalid` is `true`, this sets BOTH `aria-describedby` (general
    /// description + error message ID) AND `aria-errormessage` (error-message ID
    /// only). `aria-errormessage` is the WAI-ARIA 1.2 recommended
    /// attribute for pointing to the error message element. `aria-describedby`
    /// is retained for backwards compatibility with older assistive technologies
    /// that do not support `aria-errormessage`.
    ///
    /// Ordering: `aria-describedby` lists the description ID first, then the
    /// error ID. `aria-errormessage` contains only the error ID.
    pub fn input_aria(&self, field: &State, required: bool, has_description: bool) -> InputAria {
        let aria_errormessage = if field.show_error() {
            Some(self.error_id.clone())
        } else {
            None
        };

        InputAria {
            id: self.input_id.clone(),
            aria_labelledby: self.label_id.clone(),
            aria_describedby: self.aria_describedby(field, has_description),
            aria_invalid: if field.show_error() { Some(true) } else { None },
            aria_required: if required { Some(true) } else { None },
            aria_busy: if field.validating { Some(true) } else { None },
            aria_errormessage,
        }
    }
}

/// ARIA attributes to spread onto an input element.
#[derive(Clone, Debug)]
pub struct InputAria {
    pub id: String,
    pub aria_labelledby: String,
    pub aria_describedby: Option<String>,
    pub aria_invalid: Option<bool>,
    pub aria_required: Option<bool>,
    pub aria_busy: Option<bool>,
    /// WAI-ARIA 1.2 §5.2.7.5: points to the error message element when invalid.
    /// Set only when `aria_invalid` is `true`. Assistive technologies that support
    /// `aria-errormessage` use this instead of `aria-describedby` for error announcements.
    pub aria_errormessage: Option<String>,
}
```

---

## 7. Hidden Inputs for Form Submission

Complex components (Select, DatePicker, etc.) must render a hidden `<input>` to participate in native HTML form submission.

```rust
use ars_core::{AttrMap, HtmlAttr};

/// Configuration for a hidden input that submits with native forms.
#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub value: Value,
    pub form_id: Option<String>,  // For cross-form association
    pub disabled: bool,
}

#[derive(Clone, Debug)]
pub enum Value {
    /// Single value.
    Single(String),
    /// Multiple values (rendered as multiple hidden inputs with the same name).
    Multiple(Vec<String>),
    /// No value (omitted from submission).
    None,
}

/// Builds the common attributes shared by single and multi hidden inputs.
fn base_attrs(config: &Config, value: &str) -> AttrMap {
    let mut map = AttrMap::new();
    map.set(HtmlAttr::Type, "hidden");
    map.set(HtmlAttr::Name, &config.name);
    map.set(HtmlAttr::Value, value);
    if config.disabled {
        map.set(HtmlAttr::Disabled, true);
    }
    if let Some(ref form_id) = config.form_id {
        map.set(HtmlAttr::Form, form_id);
    }
    map
}

/// Generate an `AttrMap` for a single hidden input.
///
/// Returns `None` for `Value::None` (the element should not be rendered).
/// Panics in debug mode if called with `Value::Multiple` — use
/// `multi_attrs()` instead.
pub fn attrs(config: &Config) -> Option<AttrMap> {
    match &config.value {
        Value::Single(v) => Some(base_attrs(config, v)),
        Value::Multiple(_) => {
            debug_assert!(false, "Use multi_attrs for Value::Multiple");
            None
        }
        Value::None => None,
    }
}

/// For multi-select: returns one `AttrMap` per value.
/// Propagates `form_id` and `disabled` from config, matching `attrs`.
pub fn multi_attrs(config: &Config, values: &[String]) -> Vec<AttrMap> {
    values.iter().map(|v| base_attrs(config, v)).collect()
}

// Example usage in Select component:
// The Select machine context tracks:
//   hidden_input: Config
// The connect() API exposes:
//   pub fn hidden_input_props(&self) -> Config
// The Leptos/Dioxus adapter renders:
//   <input {..hidden_input_props()} />
```

### 7.1 Hidden Input Patterns by Component

Each complex component renders hidden `<input>` elements that participate in
native HTML `FormData` submission. The `name` attribute is taken from the
component's `name` prop. When nested inside a `form::Context`, the hidden input
is automatically registered.

| Component           | Hidden input `value` format                     | Notes                                                                                                         |
| ------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| **Select** (single) | The selected key as a string                    | `<input type="hidden" name="country" value="us">`                                                             |
| **Select** (multi)  | One hidden input per selected key               | `<input name="tags" value="a"><input name="tags" value="b">` — `FormData.getAll("tags")` returns `["a", "b"]` |
| **DatePicker**      | ISO 8601 date string                            | `<input name="dob" value="2024-03-15">`                                                                       |
| **DateRangePicker** | Two hidden inputs: `{name}_start`, `{name}_end` | ISO 8601 format for each                                                                                      |
| **TimeField**       | ISO 8601 time string                            | `<input name="time" value="14:30:00">`                                                                        |
| **ColorPicker**     | Hex string                                      | `<input name="color" value="#ff3366">`                                                                        |
| **NumberInput**     | Locale-independent number string                | `<input name="qty" value="42.5">`                                                                             |

**`form::Context` integration**: When a component with a hidden input is registered
in a `form::Context`, the form's `submit()` method collects all hidden input
values via standard `FormData` API. The `name` prop on the component becomes
the form field name.

```rust
// Select multi-value serialization example:
fn hidden_input_config(ctx: &SelectContext) -> Config {
    let name = ctx.name.clone().unwrap_or_default();
    match &ctx.selection_state.selected_keys() {
        keys if keys.is_empty() => Config {
            name,
            value: Value::None,
            form_id: ctx.form_id.clone(),
            disabled: ctx.disabled,
        },
        keys if keys.len() == 1 => Config {
            name,
            value: Value::Single(keys[0].to_string()),
            form_id: ctx.form_id.clone(),
            disabled: ctx.disabled,
        },
        keys => Config {
            name,
            value: Value::Multiple(
                keys.iter().map(|k| k.to_string()).collect()
            ),
            form_id: ctx.form_id.clone(),
            disabled: ctx.disabled,
        },
    }
}
```

#### 7.1.1 Hidden Input Synchronization for Complex Components

Components with complex selection state (Select, DatePicker, Combobox) participate in form submission via hidden `<input>` elements:

1. **Update Timing**: The hidden input's `value` MUST be synced synchronously in the same microtask as the state change — not lazily during form submission. This prevents race conditions where a form submission (triggered programmatically or by a fast user interaction) reads stale hidden input values. Concretely: when a Select's `on_selection_change` fires and updates `Context.selected_keys`, the adapter MUST update the hidden `<input>` element's `value` attribute in the same synchronous execution frame, before yielding to the microtask queue. Deferring the sync (e.g., via `requestAnimationFrame` or `queueMicrotask`) creates a window where `Data` contains the previous value.
2. **Open-but-Uncommitted State**: If a Select dropdown is open with a highlighted (but not yet selected) option, the hidden input retains the last committed value. Only an explicit selection (click or Enter) updates the hidden input.
3. **Form Reset**: On `<form>` `reset` event, complex components restore their hidden input to the `default_value` prop (or empty string if not provided). The component's visual state also resets to match.
4. **Multiple Values**: For multi-select components, multiple hidden inputs with the same `name` are rendered (one per selected value), following standard HTML form conventions.

### 7.2 Common Validation Utility Predicates

Components expose utility accessor methods on their value/state types for use
in validation logic and conditional rendering:

```rust
/// Utility predicates for form field values.
pub trait ValueExt {
    /// Returns `true` if the field has no meaningful value.
    fn is_empty(&self) -> bool;
}

// NOTE: Checkbox-specific extension impls still live with the checkbox types.
// `SelectionExt` remains defined in ars-forms, but ars-forms may implement it
// for shared foundation selection types via an `ars-collections` dependency.

/// Trait for types that expose a set of selected keys (e.g., selection::State).
/// Implemented in ars-forms for shared selection types such as
/// `ars_collections::selection::State`.
pub trait SelectionExt {
    fn is_any_selected(&self) -> bool;
    fn is_all_selected(&self, total_items: usize) -> bool;
}

/// Trait for tri-state toggle types (e.g., CheckboxState).
/// Implemented by the checkbox module in its own crate.
pub trait CheckboxExt {
    fn is_indeterminate(&self) -> bool;
    fn is_checked_or_indeterminate(&self) -> bool;
}

// DateField-specific utilities (on Option<CalendarDate> from ars-i18n):
use ars_i18n::CalendarDate;
impl ValueExt for Option<CalendarDate> {
    fn is_empty(&self) -> bool { self.is_none() }
}

// NumberInput-specific utilities:
impl ValueExt for Option<f64> {
    fn is_empty(&self) -> bool { self.is_none() }
}

// Primary Value type — delegates to variant-specific emptiness.
impl ValueExt for Value {
    fn is_empty(&self) -> bool {
        match self {
            // Note: uses raw is_empty() (no trim), unlike RequiredValidator which trims.
            // This is intentional: is_empty() is a raw structural check (e.g., for "clear"
            // button visibility), while RequiredValidator applies semantic trimming.
            Value::Text(s) => s.is_empty(),
            Value::Number(n) => n.is_none(),
            Value::Bool(b) => !b,
            Value::Date(d) => d.is_none(),
            Value::Time(t) => t.is_none(),
            Value::DateRange(r) => r.is_none(),
            Value::File(f) => f.is_empty(),
            Value::MultipleText(l) => l.is_empty(),
        }
    }
}
```

These predicates are available for use in custom validators and in adapter
template expressions (e.g., conditionally showing a "clear" button when
`!value.is_empty()`).

---

## 8. Form Submit State Machine

```rust
use ars_core::{TransitionPlan, PendingEffect, WeakSend, ConnectApi, AttrMap, HtmlAttr, AriaAttr};
use ars_core::ComponentIds;
use std::sync::Arc;

pub mod form_submit {
    use super::*;
    // Machine trait not imported — see 01-architecture.md §2.1 naming convention.

    /// A state machine for the form submission lifecycle.
    #[derive(Clone, Debug, PartialEq)]
    pub enum State {
        /// Form is ready for user input.
        Idle,
        /// Client-side validation is running.
        Validating,
        /// Validation failed — errors are shown.
        ValidationFailed,
        /// Submission is in progress (async).
        Submitting,
        /// Submission succeeded.
        Succeeded,
        /// Submission failed (server/network error).
        Failed,
    }

    impl fmt::Display for State {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                State::Idle => write!(f, "idle"),
                State::Validating => write!(f, "validating"),
                State::ValidationFailed => write!(f, "validation-failed"),
                State::Submitting => write!(f, "submitting"),
                State::Succeeded => write!(f, "succeeded"),
                State::Failed => write!(f, "failed"),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Event {
        Submit,
        ValidationPassed,
        ValidationFailed,
        SubmitComplete,
        SubmitError(String),
        Reset,
        SetServerErrors(std::collections::BTreeMap<String, Vec<String>>),
        SetMode(Mode),
    }

    #[derive(Clone, Debug)]
    pub struct Context {
        pub form: Context,
        pub ids: ComponentIds,
        pub submit_error: Option<String>,
        /// Whether synchronous validation passed (used by async-validation effect).
        pub sync_valid: bool,
        // Server errors stored in `form.server_errors` (single source of truth).
    }

    pub struct Machine;

    impl ars_core::Machine for Machine {
        type State = State;
        type Event = Event;
        type Context = Context;
        type Props = Props;
        type Api<'a> = Api<'a>;
        type Messages = Messages;

        fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
            (
                State::Idle,
                Context {
                    form: Context::new(props.validation_mode),
                    ids: ComponentIds::from_id(&props.id),
                    submit_error: None,
                    sync_valid: false,
                },
            )
        }

        fn transition(
            state: &Self::State,
            event: &Self::Event,
            ctx: &Self::Context,
            props: &Self::Props,
        ) -> Option<TransitionPlan<Self>> {
            match (state, event) {
                // These methods are provided by Context:
                // fn has_async_validators(&self) -> bool
                // fn collect_async_validators(&self) -> Vec<(String, BoxedAsyncValidator)>

                // Allow re-submission from any terminal state (including Succeeded)
                // without requiring an explicit Reset. This supports "submit again"
                // flows where a user corrects and resubmits.
                (State::Idle | State::ValidationFailed | State::Failed | State::Succeeded, Event::Submit) => { // or-pattern requires Rust edition 2021+
                    Some(TransitionPlan::to(State::Validating).apply(move |ctx| {
                        ctx.submit_error = None; // Clear stale error from prior submission
                        ctx.form.touch_all();
                        ctx.sync_valid = ctx.form.validate_all();
                    // NOTE: PendingEffect receives cloned/owned references per 01-architecture.md §2.2.7 (Context Snapshot Semantics).
                    // `ctx` is a snapshot (cloned Context), `props` is an Arc-wrapped clone,
                    // `send` is a WeakSend that may outlive the Service.
                    }).with_effect(PendingEffect::new("async-validation", |ctx, props, send: WeakSend<Event>| {
                        if ctx.form.has_async_validators() {
                            // Always run async validators when they exist, even if sync
                            // validation failed. This ensures all errors (sync + async)
                            // are shown at once, reducing correction round-trips.
                            // If sync_valid is false, async results are merged with
                            // existing sync errors via the adapter's merge logic in field_mut().
                            let validators = ctx.form.collect_async_validators();
                            // **Adapter contract:** The adapter MUST provide an async spawn
                            // function via its `use_form` hook that:
                            //   1. Runs each (name, validator) pair concurrently
                            //   2. Sends `Event::ValidationPassed` if all async validators pass AND `ctx.sync_valid` is true
                            //   3. Sends `Event::ValidationFailed` if any async validator fails OR `ctx.sync_valid` is false
                            //   4. Returns a `CleanupFn` that cancels in-flight tasks
                            //
                            // Leptos: `spawn_local` + `AbortHandle`
                            // Dioxus: `spawn` + `JoinHandle::abort()`
                            (props.spawn_async_validation)((validators, send))
                        } else {
                            // No async validators — resolve on next microtask to avoid
                            // re-entering send() during effect setup (see 01-architecture.md §2.2)
                            let event = if ctx.sync_valid {
                                Event::ValidationPassed
                            } else {
                                Event::ValidationFailed
                            };
                            // Adapter dispatches the event asynchronously via its platform's
                            // microtask mechanism (WASM: queueMicrotask, native: tokio::spawn).
                            // Safety: form_submit::Event is implicitly Send on native — all variants
                            // contain only String/BTreeMap<String, Vec<String>>. The schedule_microtask
                            // closure satisfies Box<dyn FnOnce() + Send> on native targets.
                            // Cancellation: microtasks execute within the same event loop turn,
                            // so the closure will run before the next user interaction.
                            // A CancellationToken guards against stale dispatch if the
                            // component unmounts between schedule and execution.
                            let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
                            let cancelled_clone = cancelled.clone();
                            (props.schedule_microtask)(Box::new(move || {
                                let is_cancelled = cancelled_clone.load(std::sync::atomic::Ordering::Relaxed);
                                if !is_cancelled {
                                    send.call_if_alive(event);
                                }
                            }));
                            Box::new(move || {
                                cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                            })
                        }
                    })))
                }
                (State::Validating, Event::ValidationPassed) => {
                    Some(TransitionPlan::to(State::Submitting).apply(|ctx| {
                        ctx.form.is_submitting = true;
                    }).with_effect(PendingEffect::new("submit", |_ctx, _props, _send| {
                        // **Adapter contract:** The adapter observes the `Submitting` state
                        // and invokes the user-provided `on_submit` callback. When complete,
                        // it sends `Event::SubmitComplete` (success) or `Event::SubmitError(msg)`.
                        //
                        // This effect is intentionally a no-op in core. It exists so the adapter
                        // can register a cleanup function (the returned `Box<dyn FnOnce()>`)
                        // that cancels in-flight requests when the machine transitions away
                        // from `Submitting` (e.g., component unmount during async submit).
                        no_cleanup()
                    })))
                }
                (State::Validating, Event::ValidationFailed) => {
                    Some(TransitionPlan::to(State::ValidationFailed)
                        .apply(|ctx| {
                            ctx.sync_valid = false;
                        }))
                }
                (State::Submitting, Event::SubmitComplete) => {
                    Some(TransitionPlan::to(State::Succeeded).apply(|ctx| {
                        ctx.form.is_submitting = false;
                        ctx.submit_error = None; // Clear any stale error from prior failed submission
                    }))
                }
                (State::Submitting, Event::SubmitError(msg)) => {
                    let msg = msg.clone();
                    Some(TransitionPlan::to(State::Failed)
                        .apply(move |ctx| {
                            ctx.form.is_submitting = false;
                            ctx.submit_error = Some(msg);
                        }))
                }
                // SetServerErrors can arrive from any state, including Submitting
                // (e.g., server returns validation errors inline with the submit response
                // via streaming or partial response). This is an intentional escape hatch
                // that bypasses SubmitError. When received during Submitting, the expected
                // follow-up is a SubmitComplete event to complete the lifecycle.
                // Without the follow-up, the form remains in ValidationFailed with
                // is_submitting cleared, which is the correct resting state.
                (_, Event::SetServerErrors(errors)) => {
                    let errors = errors.clone();
                    Some(TransitionPlan::to(State::ValidationFailed)
                        .apply(move |ctx| {
                            ctx.form.is_submitting = false;
                            ctx.submit_error = None;
                            ctx.form.set_server_errors(errors);
                        }))
                }
                (_, Event::Reset) => {
                    let mode = props.validation_mode;
                    Some(TransitionPlan::to(State::Idle)
                        .cancel_effect("async-validation")
                        .cancel_effect("submit")
                        .apply(move |ctx| {
                            ctx.form.reset();
                            ctx.form.validation_mode = mode;
                            ctx.submit_error = None;
                        }))
                }
                (_, Event::SetMode(mode)) => {
                    let mode = *mode;
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.form.validation_mode = mode;
                        // No state change, no field reset — only config update
                    }))
                }
                // Note: Submit events during Validating and Submitting states are intentionally
                // dropped (returns None). This provides debounce-by-state-machine — the user
                // cannot re-submit while validation or submission is in progress.
                _ => None,
            }
        }

        fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
            // NOTE: `id` is immutable after init — ComponentIds are computed once
            // in init() and cached in Context. Changing id at runtime is not supported.
            if old.validation_mode != new.validation_mode {
                // Update validation mode without resetting form state.
                vec![Event::SetMode(new.validation_mode)]
            } else {
                vec![]
            }
        }

        fn connect<'a>(
            state: &'a State,
            ctx: &'a Context,
            props: &'a Props,
            send: &'a dyn Fn(Event),
        ) -> Api<'a> {
            Api { state, ctx, props, send }
        }
    }

    /// Note: `id` must be set by the adapter layer (via `use_id`/`use_stable_id`);
    /// the Default empty string is a builder placeholder.
    // Adapters provide default no-op implementations; end-users never construct Props directly.
    type SpawnAsyncValidationInput = (Vec<(String, BoxedAsyncValidator)>, WeakSend<Event>);
    type SpawnAsyncValidationFn = dyn Fn(SpawnAsyncValidationInput) -> Box<dyn FnOnce()>;
    type ScheduleMicrotaskFn = dyn Fn(Box<dyn FnOnce()>);

    #[derive(Clone, HasId)]
    pub struct Props {
        pub id: String,
        pub validation_mode: Mode,
        /// Adapter-provided async spawn for running async validators concurrently.
        /// Signature: (validators, send) -> CleanupFn.
        /// Leptos: wraps `spawn_local`; Dioxus: wraps `spawn`.
        // Tuple parameter: Callback::new() supports single-arg closures; multi-param fns use a tuple.
        pub spawn_async_validation: Callback<SpawnAsyncValidationFn>,
        /// Adapter-provided microtask scheduler for deferred event dispatch.
        /// WASM: wraps `queueMicrotask`; native: wraps `tokio::spawn` or equivalent.
        pub schedule_microtask: Callback<ScheduleMicrotaskFn>,
        // On native targets (tokio), the boxed closure must be Send.
        // Adapters targeting native should use: Box<dyn FnOnce() + Send>
        // WASM targets (single-threaded) do not require Send.
        // on_submit callback is registered in the adapter layer, not in Props.
    }

    impl PartialEq for Props {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.validation_mode == other.validation_mode
            // Callback fields are not compared — identity is determined by id + mode.
        }
    }

    impl core::fmt::Debug for Props {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("form_submit::Props")
                .field("id", &self.id)
                .field("validation_mode", &self.validation_mode)
                .finish_non_exhaustive()
        }
    }

    pub struct Api<'a> {
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    }

    #[derive(ComponentPart)]
    #[scope = "form-submit"]
    pub enum Part {
        Root,
        SubmitButton,
    }

    impl ConnectApi for Api<'_> {
        type Part = Part;

        fn part_attrs(&self, part: Self::Part) -> AttrMap {
            match part {
                Part::Root => self.root_attrs(),
                Part::SubmitButton => self.submit_button_attrs(),
            }
        }
    }

    impl<'a> Api<'a> {
        pub fn is_submitting(&self) -> bool {
            *self.state == State::Submitting
        }

        pub fn is_valid(&self) -> bool {
            self.ctx.form.is_valid()
        }

        pub fn submit_error(&self) -> Option<&str> {
            self.ctx.submit_error.as_deref()
        }

        pub fn root_attrs(&self) -> AttrMap {
            let mut attrs = AttrMap::new();
            attrs.set(HtmlAttr::Id, self.ctx.ids.id());
            let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
            attrs.set(scope_attr, scope_val);
            attrs.set(part_attr, part_val);
            attrs.set(HtmlAttr::Data("ars-state"), self.state.to_string());
            attrs
        }

        /// Typed handler: call from the adapter's `on:submit` handler.
        pub fn on_form_submit(&self) {
            (self.send)(Event::Submit);
        }

        pub fn submit_button_attrs(&self) -> AttrMap {
            let mut attrs = AttrMap::new();
            let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubmitButton.data_attrs();
            attrs.set(scope_attr, scope_val);
            attrs.set(part_attr, part_val);
            if self.is_submitting() {
                attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
                attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
                // Also set HTML disabled to truly prevent double-submit clicks.
                // This is an accepted exception to the general ARIA-disabled pattern
                // (see §15 and 03-accessibility.md §13): during the brief submission
                // window, preventing double-submit outweighs discoverability. The
                // aria-busy="true" attribute above ensures screen readers announce the
                // in-progress state.
                attrs.set_bool(HtmlAttr::Disabled, true);
            }
            attrs
        }
    }
}
```

> **Async validation at submit:** When `submit()` is called, the `form_submit::Machine` transitions to `Validating`. The machine first runs all synchronous validators via `validate_all()`. If any fields have registered async validators, the machine spawns them via `PendingEffect` and remains in `Validating` until all complete. When all async validators settle, the adapter sends a single `ValidationPassed` or `ValidationFailed` event. `ValidationPassed` triggers a transition to `Submitting`; `ValidationFailed` transitions to the `ValidationFailed` state. The `validation_generation` counter prevents stale results from overwriting newer validations.

### 8.1 Server-Side Validation Error Sync Pattern

After form submission, the server may return field-level validation errors (e.g., "Email already registered"). These errors must be synced back to the form state machine so that individual fields display server errors alongside (or instead of) client-side errors.

**Sync pattern — single field**:

```rust
// Server returns a single field error after submission.
// The adapter sends a SetServerErrors event with the field name and error messages:
machine.send(Event::SetServerErrors(
    BTreeMap::from([("email".into(), vec!["Email already registered".into()])])
));
```

**Sync pattern — multiple fields**:

```rust
// Server returns errors for multiple fields (e.g., from a bulk validation endpoint).
let server_errors: BTreeMap<String, Vec<String>> = response.field_errors;
machine.send(Event::SetServerErrors(server_errors));
```

**Lifecycle**: When `SetServerErrors` is received:

1. The form transitions to `State::ValidationFailed` (regardless of current state).
2. `Context::set_server_errors()` injects errors into each named field's `State`, setting `touched = true` so errors display immediately.
3. The adapter moves focus to the first invalid field (the first field with any validation error, including the newly injected server errors) using §9 focus management.
4. When the user edits a field that has a server error, the server error for that field is cleared automatically (the field reverts to client-side validation only).

**Adapter integration** (Leptos example):

```rust
// In the on_submit handler, after receiving a server response:
let on_submit = move |_| {
    spawn_local(async move {
        let result = submit_form(form_data).await;
        match result {
            Ok(_) => machine.send(Event::SubmitComplete),
            Err(ServerError { field_errors }) => {
                machine.send(Event::SetServerErrors(field_errors));
            }
            Err(ServerError { message }) => {
                machine.send(Event::SubmitError(message));
            }
        }
    });
};
```

This pattern ensures server validation errors are first-class citizens in the form state machine, not ad-hoc side effects.

---

## 9. Focus Management on Submit Error

When form submission fails validation, focus should be moved to the first invalid field:

```rust
/// Find the first invalid field and return its input ID.
/// Must be called after all async validators have settled (i.e., after
/// `form_submit::Machine` enters `ValidationFailed` state, not while in `Validating`).
use indexmap::IndexMap;

pub fn first_invalid_field_id(form: &Context, field_descriptors: &IndexMap<String, Descriptors>) -> Option<String> {
    // Iterate fields in DOM order (which is registration order in our IndexMap)
    form.fields.iter()
        .find(|(_, state)| state.validation.is_err())
        .and_then(|(name, _)| field_descriptors.get(name))
        .map(|d| d.input_id.clone())
}

// After submit validation failure:
// 1. Call first_invalid_field_id
// 2. If found: document.getElementById(id).focus()
// 3. Screen reader will announce the error via aria-describedby
```

---

## 10. Framework Integration

### 10.1 Leptos

````rust
// ars-leptos/src/form.rs

use leptos::prelude::*;
use ars_forms::{Context, Mode, Value, Result, BoxedValidator, BoxedAsyncValidator};

/// Leptos hook for form management.
pub fn use_form(mode: Mode) -> UseFormReturn {
    let (form, set_form) = signal(Context::new(mode));

    let register = move |name: &str, initial: Value, validator: Option<BoxedValidator>, async_validator: Option<BoxedAsyncValidator>| {
        set_form.update(|f| f.register(name, initial, validator, async_validator));
    };

    let on_change = move |name: &str, value: Value| {
        set_form.update(|f| f.on_change(name, value));
    };

    let on_blur = move |name: &str| {
        set_form.update(|f| f.on_blur(name));
    };

    #[cfg(not(target_arch = "wasm32"))]
    let submit = move |handler: Box<dyn FnOnce(Data) + Send>| {
        set_form.update(|f| { f.submit(handler); });
    };
    #[cfg(target_arch = "wasm32")]
    let submit = move |handler: Box<dyn FnOnce(Data)>| {
        set_form.update(|f| { f.submit(handler); });
    };

    // Return type: generic type parameters are inferred from the closure types.
    UseFormReturn { form, set_form, register, on_change, on_blur, submit }
}

// Note: use_form returns `UseFormReturn<impl Fn(...), impl Fn(...), impl Fn(...), impl Fn(...)>`
// but Rust does not allow `impl Trait` in return position of non-free functions easily.
// In practice, callers use `let form = use_form(mode);` and let the compiler infer all types.

/// The type parameters `Reg`, `Change`, `Blur`, `Submit` are the concrete closure types
/// inferred from `use_form()`. If a generic context needs to name this type without
/// binding concrete closures (e.g., in a trait or struct that stores the return value),
/// add a `PhantomData<T>` field to carry any unused type parameter:
///
/// ```rust
/// pub struct FormWrapper<T> {
///     inner: UseFormReturn<...>,
///     _marker: PhantomData<T>,
/// }
/// ```
pub struct UseFormReturn<Reg, Change, Blur, Submit> {
    pub form: ReadSignal<Context>,
    pub set_form: WriteSignal<Context>,
    pub register: Reg,    // Fn(&str, Value, Option<BoxedValidator>, Option<BoxedAsyncValidator>)
    pub on_change: Change, // Fn(&str, Value)
    pub on_blur: Blur,     // Fn(&str)
    pub submit: Submit,    // Fn(Box<dyn FnOnce(Data) [+ Send on non-wasm]>)
}

/// A complete Leptos form example using ars-ui components:
///
/// ```rust
/// #[component]
/// fn SignupForm() -> impl IntoView {
///     let UseFormReturn { form, on_change, on_blur, submit, .. } = use_form(Mode::on_blur_revalidate());
///
///     // Register fields with validators
///     // (In real usage, components self-register via FormFieldContext)
///
///     let on_submit = move |e: SubmitEvent| {
///         e.prevent_default();
///         submit(Box::new(|data| {
///             // Call server function here
///         }));
///     };
///
///     view! {
///         <form on:submit=on_submit>
///             <TextField name="email" input_type=InputType::Email required=true />
///             <TextField name="password" input_type=InputType::Password required=true min_length=8 />
///             <Button button_type=button::Type::Submit loading=form.with(|f| f.is_submitting)>
///                 "Create account"
///             </Button>
///         </form>
///     }
/// }
/// ```
````

### 10.2 Dioxus

````rust
// ars-dioxus/src/form.rs

use dioxus::prelude::*;
use ars_forms::{Context, Mode};

pub fn use_form(mode: Mode) -> Signal<Context> {
    use_signal(|| Context::new(mode))
}

/// Dioxus form example:
///
/// ```rust
/// #[component]
/// fn SignupForm() -> Element {
///     let mut form = use_form(Mode::on_blur_revalidate());
///
///     let on_submit = move |e: FormEvent| {
///         e.prevent_default();
///         form.write().submit(|data| {
///             // Handle submission
///         });
///     };
///
///     rsx! {
///         form { onsubmit: on_submit,
///             TextField { name: "email", required: true }
///             Button { r#type: "submit", "Sign up" }
///         }
///     }
/// }
/// ```
````

---

## 11. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_validator() {
        let v = RequiredValidator { message: None };
        let ctx = Context::standalone("test");
        let messages = FormMessages::default();

        let empty_value = Value::Text(String::new());
        let result = v.validate(&empty_value, &ctx);
        assert_eq!(
            result,
            Err(Errors(vec![Error::required(&messages, ctx.locale.unwrap_or(&DEFAULT_VALIDATOR_LOCALE))]))
        );

        let hello_value = Value::Text("hello".into());
        let result = v.validate(&hello_value, &ctx);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_min_length_validator() {
        let v = MinLengthValidator { min: 3, message: None };
        let ctx = Context::standalone("test");

        let short_value = Value::Text("ab".into());
        assert!(v.validate(&short_value, &ctx).is_err());

        let exact_value = Value::Text("abc".into());
        assert!(v.validate(&exact_value, &ctx).is_ok());

        let long_value = Value::Text("abcd".into());
        assert!(v.validate(&long_value, &ctx).is_ok());
    }

    #[test]
    fn test_form_context_on_blur_validation() {
        let mut form = Context::new(Mode::on_blur_revalidate());
        form.register(
            "email",
            Value::Text(String::new()),
            Some(Validators::new().required().email().build_first_fail().boxed()),
            None,
        );

        // Before blur, no error shown
        assert!(!form.field("email").expect("email field must be registered").show_error());

        // After blur with empty value, error shown
        form.on_blur("email");
        assert!(form.field("email").expect("email field must be registered").show_error());
    }

    #[test]
    fn test_server_errors() {
        let mut form = Context::new(Mode::on_submit());
        form.register("email", Value::Text("user@example.com".into()), None, None);

        form.set_server_errors(
            [("email".to_string(), vec!["Email already in use".to_string()])]
                .into_iter().collect()
        );

        assert!(form.field("email").expect("email field must be registered").show_error());
        assert_eq!(
            form.field("email").expect("email field must be registered").error_message(),
            Some("Email already in use")
        );
    }

    #[test]
    fn test_server_error_cleared_on_change() {
        let mut form = Context::new(Mode::on_change());
        form.register("email", Value::Text("used@example.com".into()), None, None);
        form.set_server_errors(
            [("email".to_string(), vec!["Email taken".to_string()])]
                .into_iter().collect()
        );

        assert!(form.field("email").expect("email field must be registered").validation.is_err());

        // User changes the value — server error should be cleared
        form.on_change("email", Value::Text("new@example.com".into()));
        assert!(form.field("email").expect("email field must be registered").validation.is_ok());
    }
}
```

---

## 12. Fieldset Component

> Cross-references: Equivalent to Ark-UI `Fieldset`, HTML `<fieldset>` semantics.

### 12.1 Purpose

`Fieldset` groups related form controls under a shared disabled/invalid context, mapping directly to the HTML `<fieldset>` + `<legend>` pattern. This is the standard mechanism for form sections like "Billing Address" / "Shipping Address" where an entire group of fields can be disabled or marked invalid as a unit.

HTML `<fieldset disabled>` natively disables all contained inputs — `Fieldset` exposes this behavior through the state machine while providing context propagation to child `Field` components.

### 12.2 State Machine

#### 12.2.1 Fieldset Component States

```rust
use ars_i18n::Direction;

/// Fieldset is effectively stateless — disabled/invalid are tracked in Context.
// Single-variant state: required by Machine trait. All transitions are context-only.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
}
```

#### 12.2.2 Fieldset Component Events

```rust
#[derive(Clone, Debug)]
pub enum Event {
    /// Set validation errors at the fieldset level.
    SetErrors(Vec<Error>),
    /// Clear all fieldset-level validation errors.
    ClearErrors,
    /// Sync disabled state from props change.
    SetDisabled(bool),
    /// Sync invalid state from props change.
    SetInvalid(bool),
    /// Sync readonly state from props change.
    SetReadonly(bool),
    /// Sync direction from props change (e.g., LTR↔RTL language switch).
    SetDir(Option<Direction>),
    /// Signal that a Description part has been mounted/unmounted.
    SetHasDescription(bool),
}
```

#### 12.2.3 Fieldset Component Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the entire fieldset and all contained inputs are disabled.
    pub disabled: bool,
    /// Whether the fieldset is in an invalid state.
    pub invalid: bool,
    /// Whether the fieldset is read-only.
    pub readonly: bool,
    /// Layout direction for RTL support.
    pub dir: Option<Direction>,
    /// Fieldset-level validation errors.
    pub errors: Vec<Error>,
    // NOTE: `has_error_message` removed — use `!errors.is_empty()` instead.
    /// Whether a Description part is rendered (set by `SetHasDescription` event).
    pub has_description: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

#### 12.2.4 Fieldset Component Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Whether the entire fieldset and all contained inputs are disabled.
    pub disabled: bool,
    /// Whether the fieldset is in an invalid state.
    pub invalid: bool,
    /// Whether the fieldset is read-only.
    pub readonly: bool,
    /// Layout direction for RTL support.
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            invalid: false,
            readonly: false,
            dir: None,
        }
    }
}
```

#### 12.2.5 Fieldset Component Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ctx = Context {
            disabled: props.disabled,
            invalid: props.invalid,
            readonly: props.readonly,
            dir: props.dir,
            errors: Vec::new(),
            has_description: false,
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // NOTE: `id` is immutable after init — ComponentIds are computed once
        // in init() and cached in Context. Changing id at runtime is not supported.
        let mut events = Vec::new();
        if old.disabled != new.disabled { events.push(Event::SetDisabled(new.disabled)); }
        if old.invalid != new.invalid { events.push(Event::SetInvalid(new.invalid)); }
        if old.readonly != new.readonly { events.push(Event::SetReadonly(new.readonly)); }
        if old.dir != new.dir { events.push(Event::SetDir(new.dir)); }
        events
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetErrors(errors) => {
                let errors = errors.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.errors = errors;
                    ctx.invalid = !ctx.errors.is_empty();
                }))
            }
            Event::ClearErrors => {
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.errors.clear();
                    ctx.invalid = base_invalid;
                }))
            }
            Event::SetDisabled(disabled) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                }))
            }
            Event::SetInvalid(invalid) => {
                let invalid = *invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    // Error-driven invalidity takes precedence: if errors are present,
                    // the field stays invalid regardless of the prop value.
                    ctx.invalid = invalid || !ctx.errors.is_empty();
                }))
            }
            Event::SetReadonly(readonly) => {
                let readonly = *readonly;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.readonly = readonly;
                }))
            }
            Event::SetDir(dir) => {
                let dir = *dir; // Direction is Copy
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.dir = dir;
                }))
            }
            Event::SetHasDescription(has) => {
                let has = *has;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.has_description = has;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

#### 12.2.6 Fieldset Component Connect API

```rust
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

#[derive(ComponentPart)]
#[scope = "fieldset"]
pub enum Part {
    Root,
    Legend,
    Description,
    ErrorMessage,
    Content,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Legend => self.legend_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::Content => self.content_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Attributes for the root `<fieldset>` element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Fieldset uses HTML `disabled` (not just `aria-disabled`) because
        // <fieldset disabled> natively disables all contained form elements.
        // See §15 (Disabled vs Readonly Contract) for the fieldset exception.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if let Some(dir) = self.ctx.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
        }
        // Build aria-describedby: include description (if rendered) and error message (if errors).
        let mut described_by_ids = Vec::new();
        if self.ctx.has_description {
            described_by_ids.push(self.ctx.ids.part("description"));
        }
        if !self.ctx.errors.is_empty() {
            described_by_ids.push(self.ctx.ids.part("error-message"));
        }
        if !described_by_ids.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by_ids.join(" "),
            );
        }
        attrs
    }

    /// Attributes for the `<legend>` element.
    pub fn legend_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("legend"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Legend.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        // Use role="alert" only — do NOT also set aria-live="assertive"
        // to avoid double-announcement on NVDA+Firefox (see §13.3 note).
        attrs.set(HtmlAttr::Role, "alert");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.errors.is_empty() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the content container.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Current fieldset-level validation errors.
    pub fn errors(&self) -> &[Error] {
        &self.ctx.errors
    }

    /// Whether the fieldset is disabled.
    pub fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Whether the fieldset is in an invalid state.
    pub fn is_invalid(&self) -> bool {
        self.ctx.invalid
    }

    /// Whether the fieldset is read-only.
    pub fn is_readonly(&self) -> bool {
        self.ctx.readonly
    }
}
```

### 12.3 Anatomy

| Part             | HTML Element | Description                                                                 |
| ---------------- | ------------ | --------------------------------------------------------------------------- |
| **Root**         | `<fieldset>` | Container element. Receives `disabled` attribute when fieldset is disabled. |
| **Legend**       | `<legend>`   | Accessible label for the fieldset group.                                    |
| **ErrorMessage** | `<span>`     | Group-level error display. Receives `id`, `role="alert"`.                   |
| **Content**      | `<div>`      | Slot for child fields.                                                      |
| **Description**  | `<span>`     | Help/description text for the fieldset group. Receives `id`.                |

### 12.4 ARIA Mapping

| Attribute          | Element | Source                            | Notes                                                                                |
| ------------------ | ------- | --------------------------------- | ------------------------------------------------------------------------------------ |
| `disabled`         | Root    | `ctx.disabled`                    | Native `<fieldset disabled>` propagates to all children                              |
| `aria-describedby` | Root    | Description ID + Error message ID | Includes Description part ID when rendered; ErrorMessage part ID when errors present |
| `dir`              | Root    | `ctx.dir`                         | Emitted when `dir` is `Some`                                                         |

> **Note on `aria-invalid` on `<fieldset>`**: Screen readers (NVDA, JAWS, VoiceOver) do not reliably announce `aria-invalid` on `<fieldset>` elements — they only announce it on focusable form controls. Fieldset-level invalidity is communicated via the `ErrorMessage` anatomy part (which uses `role="alert"`) and optionally by appending "contains errors" to the `Legend` text. The `aria-invalid` attribute is NOT set on the fieldset root.

### 12.5 Integration with Context

When nested inside a `Context`, the `Fieldset` registers itself and can receive form-level disabled state. Children of the `Fieldset` see the **merged** disabled state: a child is disabled if either the fieldset OR the individual field is disabled.

### 12.6 Context — Shared Context for Child Fields

`Context` is the bridge between a parent group component (`Fieldset`, `CheckboxGroup`, `RadioGroup`) and child `Field` components. The parent provides it via framework context; child `Field` components consume it to inherit disabled/invalid state.

```rust
/// Propagated via framework context (Leptos `provide_context`, Dioxus `use_context_provider`)
/// from Fieldset (or CheckboxGroup, RadioGroup) to child Field components.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// Optional field name inherited from the parent group (e.g., checkbox group name).
    pub name: Option<String>,
    /// Whether the parent group is disabled. Child fields merge this with their own `disabled` prop.
    pub disabled: bool,
    /// Whether the parent group is invalid. Child fields merge this with their own `invalid` prop.
    pub invalid: bool,
    /// Whether the parent group is read-only. Child fields merge this with their own `readonly` prop.
    pub readonly: bool,
}
```

**Merge semantics**: When a `Field` detects a parent `Context`, it merges via logical OR:

- `effective_disabled = field_props.disabled || field_ctx.disabled`
- `effective_invalid = field_props.invalid || field_ctx.invalid`
- `effective_readonly = field_props.readonly || field_ctx.readonly`

The merge happens at the **adapter layer** (not inside the core machine), because the core `field::Machine` has no access to framework context. The adapter passes the merged values as props to `field::Machine`, keeping the core machine framework-agnostic. See §13.4 for the merge code pattern.

### 12.7 I18n Considerations

- **RTL**: When `dir` is `Some`, it is emitted on the `<fieldset>` root. `<legend>` text should use Unicode directional isolate characters (`U+2068`/`U+2069`) when content may contain mixed-direction text.
- **Error messages**: Localizable via `Error`. When error messages contain embedded user input, the interpolated text must be wrapped in Unicode isolates.

---

## 13. Field Component (Generic Wrapper)

> Cross-references: Equivalent to Ark-UI `Field`, React Aria `Label` + `Group` + `FieldError` + `Description`.

### 13.1 Purpose

`Field` is a generic form field wrapper that works with **any** input component. It provides:

- Automatic ID generation and ARIA wiring between label, input, description, and error message
- Context propagation (`invalid`, `disabled`, `required`, `readOnly`) to any child input
- Composability with third-party or custom input components

This replaces the need for each component to independently implement label/description/error wiring.

### 13.2 State Machine

#### 13.2.1 Field Component States

```rust
use ars_i18n::Direction;

/// Field is effectively stateless — field metadata is tracked in Context.
// Single-variant state: required by Machine trait. All transitions are context-only.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
}
```

#### 13.2.2 Field Component Events

```rust
#[derive(Clone, Debug)]
pub enum Event {
    /// Set validation errors to display.
    SetErrors(Vec<Error>),
    /// Clear all validation errors.
    ClearErrors,
    /// Notify that a Description part has mounted/unmounted.
    SetHasDescription(bool),
    /// Sync disabled state from props change.
    SetDisabled(bool),
    /// Sync invalid state from props change.
    SetInvalid(bool),
    /// Sync readonly state from props change.
    SetReadonly(bool),
    /// Sync required state from props change.
    SetRequired(bool),
    /// Sync direction from props change (e.g., LTR↔RTL language switch).
    SetDir(Option<Direction>),
    /// Sync async-validation state from Context.
    /// Dispatched by adapter when FormState.validating changes.
    SetValidating(bool),
}
```

#### 13.2.3 Field Component Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is disabled (merged with Fieldset context).
    pub disabled: bool,
    /// Whether the field is read-only.
    pub readonly: bool,
    /// Whether the field is invalid.
    pub invalid: bool,
    /// Whether an async validator is currently running for this field.
    /// Set via `SetValidating` event dispatched by the adapter when
    /// `FormState.validating` changes in the parent Context.
    pub validating: bool,
    /// Layout direction.
    pub dir: Option<Direction>,
    /// Current validation errors.
    pub errors: Vec<Error>,
    /// Whether a Description part is rendered.
    pub has_description: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

#### 13.2.4 Field Component Props

```rust
/// Note: Named `FieldComponentProps` to avoid collision with `State`
/// in §2.3 which tracks per-field form context state.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is read-only.
    pub readonly: bool,
    /// Whether the field is invalid.
    pub invalid: bool,
    /// Layout direction.
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            required: false,
            disabled: false,
            readonly: false,
            invalid: false,
            dir: None,
        }
    }
}
```

#### 13.2.5 Field Component Full Machine Implementation

```rust
// inside field component module
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ctx = Context {
            required: props.required,
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            validating: false,
            dir: props.dir,
            errors: Vec::new(),
            has_description: false,
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // NOTE: `id` is immutable after init — ComponentIds are computed once
        // in init() and cached in Context. Changing id at runtime is not supported.
        let mut events = Vec::new();
        if old.disabled != new.disabled { events.push(Event::SetDisabled(new.disabled)); }
        if old.invalid != new.invalid { events.push(Event::SetInvalid(new.invalid)); }
        if old.readonly != new.readonly { events.push(Event::SetReadonly(new.readonly)); }
        if old.required != new.required { events.push(Event::SetRequired(new.required)); }
        if old.dir != new.dir { events.push(Event::SetDir(new.dir)); }
        events
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetErrors(errors) => {
                let errors = errors.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.errors = errors;
                    ctx.invalid = !ctx.errors.is_empty();
                }))
            }
            Event::ClearErrors => {
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.errors.clear();
                    ctx.invalid = base_invalid;
                }))
            }
            Event::SetHasDescription(has) => {
                let has = *has;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.has_description = has;
                }))
            }
            Event::SetDisabled(disabled) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                }))
            }
            Event::SetInvalid(invalid) => {
                let invalid = *invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    // Error-driven invalidity takes precedence: if errors are present,
                    // the field stays invalid regardless of the prop value.
                    ctx.invalid = invalid || !ctx.errors.is_empty();
                }))
            }
            Event::SetReadonly(readonly) => {
                let readonly = *readonly;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.readonly = readonly;
                }))
            }
            Event::SetRequired(required) => {
                let required = *required;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.required = required;
                }))
            }
            Event::SetDir(dir) => {
                let dir = *dir; // Direction is Copy
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.dir = dir;
                }))
            }
            Event::SetValidating(validating) => {
                let validating = *validating;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.validating = validating;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

#### 13.2.6 Field Component Connect API

```rust
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event), // available for imperative event dispatch by adapter code
}

#[derive(ComponentPart)]
#[scope = "field"]
pub enum Part {
    Root,
    Label,
    Input,
    Description,
    ErrorMessage,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(dir) = self.ctx.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
        }
        attrs
    }

    /// Attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes to apply on the child input element.
    /// The child input reads these to get its ARIA wiring.
    ///
    /// Note: Unlike `Descriptors::input_aria()` which gates `aria-invalid` on
    /// `show_error()` (touched AND invalid), this method sets `aria-invalid` immediately
    /// when `self.ctx.invalid` is true. This is intentional: Descriptors is for
    /// Context-managed touch state; field::Api is for prop-driven invalid state
    /// where the parent controls display timing.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        // Build aria-describedby
        let mut describedby = Vec::new();
        if self.ctx.has_description {
            describedby.push(self.ctx.ids.part("description"));
        }
        if !self.ctx.errors.is_empty() {
            describedby.push(self.ctx.ids.part("error-message"));
        }
        if !describedby.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby.join(" "));
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            if !self.ctx.errors.is_empty() {
                attrs.set(HtmlAttr::Aria(AriaAttr::ErrorMessage), self.ctx.ids.part("error-message"));
            }
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            // NOTE: aria-disabled="true" does NOT prevent user interaction on native <input> elements.
            // Adapters MUST intercept events or add CSS pointer-events:none. See §15 (Disabled vs Readonly Contract).
            // Per APG guidance, aria-disabled elements remain focusable but inoperable —
            // removing from tab order would prevent screen reader users from discovering disabled fields.
        }
        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        if self.ctx.validating {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }
        attrs
    }

    /// Attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        // Use role="alert" only — do NOT also set aria-live="assertive"
        // to avoid double-announcement on NVDA+Firefox.
        attrs.set(HtmlAttr::Role, "alert");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.errors.is_empty() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }
}
```

### 13.3 Anatomy

| Part             | HTML Element   | Description                                                                 |
| ---------------- | -------------- | --------------------------------------------------------------------------- |
| **Root**         | `<div>`        | Container with `data-ars-part="root"`.                                      |
| **Label**        | `<label>`      | Linked to input via `for={id}`. Receives `id={id}-label`.                   |
| **Input**        | _(child slot)_ | Any input component. Receives ARIA attributes from `input_attrs()`.         |
| **Description**  | `<span>`       | Help text. Receives `id={id}-description`.                                  |
| **ErrorMessage** | `<span>`       | Validation error display. Receives `id={id}-error-message`, `role="alert"`. |

> **Note on error announcements**: The `ErrorMessage` element uses `role="alert"` which implicitly sets `aria-live="assertive"` and `aria-atomic="true"`. Do NOT additionally set `aria-live="assertive"` — setting both causes double-announcement on NVDA+Firefox (see `03-accessibility.md` §5.1 LiveAnnouncer).

### 13.4 Integration with Fieldset

When nested inside a `Fieldset`, the `Field` merges its own state with the `Fieldset` context (see §12.6 `Context`). The merge is performed at the **adapter layer**, not inside the core machine — the core `field::Machine` has no access to framework context.

**Adapter-layer merge pattern:**

```rust
// Inside the adapter's Field component (Leptos or Dioxus):
// 1. Attempt to read Context from the parent Fieldset/CheckboxGroup/RadioGroup.
let field_ctx: Option<Context> = /* adapter-specific context read */;

// 2. Merge via logical OR — a field is disabled/invalid if either
//    its own prop OR the parent group says so.
let effective_disabled = props.disabled
    || field_ctx.as_ref().map_or(false, |f| f.disabled);
let effective_invalid = props.invalid
    || field_ctx.as_ref().map_or(false, |f| f.invalid);
let effective_readonly = props.readonly
    || field_ctx.as_ref().map_or(false, |f| f.readonly);

// 3. Build merged props for the core machine.
// id, required, dir from props; disabled, invalid, readonly merged from Context.
let merged_props = field::Props {
    disabled: effective_disabled,
    invalid: effective_invalid,
    readonly: effective_readonly,
    ..props
};

// 4. Pass merged props to the core machine — it sees the final merged values.
let machine = use_machine::<field::Machine>(merged_props);
```

This keeps the core machine framework-agnostic while allowing each adapter to use its native context mechanism (`provide_context` in Leptos, `use_context_provider` in Dioxus).

### 13.5 I18n Considerations

- **RTL**: When `dir` is `Some`, it is emitted on the Root element. Label text should use Unicode directional isolate characters when it may contain mixed-direction content.
- **Error messages**: When error messages contain embedded user input (e.g., "'{value}' is not a valid email"), the interpolated text must be wrapped in Unicode directional isolates (`U+2068`/`U+2069`) to prevent BiDi reordering.

---

## 14. Form Component

> **Relationship to form_submit::Machine:** The `form_submit::Machine` (§8) is a lower-level 6-state submission lifecycle machine with explicit validation states, designed for advanced flows (multi-step async validation, complex retry logic). The `form::Machine` (this section) is a standalone high-level component with a simplified 2-state model (Idle, Submitting). They are independent alternatives — **not composed together**. Most consumers should use `form::Machine` via the Form component; use `form_submit::Machine` only when you need fine-grained control over the validation→submission lifecycle. Both machines integrate with `Context` via adapter-provided context propagation.
>
> Cross-references: Equivalent to React Aria `Form`.

### 14.1 Purpose

The `Form` component renders a `<form>` element with pre-wired integration to `Context` and the form submission lifecycle. It handles:

- `onSubmit` / `onReset` event binding
- Validation behavior selection (`Native` vs `Aria`)
- Server-side error injection
- Prevention of default form submission when using client-side validation
- Accessible status announcements for submission results

### 14.2 Validation Behavior

```rust
/// Controls how validation errors are reported to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationBehavior {
    /// Use native HTML constraint validation.
    /// Errors appear via browser-native tooltip UI.
    /// Validation triggers on form submit.
    Native,

    /// Use ARIA-based validation display.
    /// Errors render in custom `ErrorMessage` anatomy parts.
    /// Validation can trigger on blur, change, or submit (configurable).
    Aria,
}

impl Default for ValidationBehavior {
    fn default() -> Self {
        Self::Aria
    }
}
```

#### 14.2.1 Validation Error Message Localization

Validation error messages must respect the component's locale context:

1. **Language Match**: Error messages use the same locale as the component's labels and placeholders. If a DatePicker displays in French, its "Invalid date" error should be "Date invalide".
2. **BiDi Handling**: Error messages in RTL scripts are wrapped with `dir="auto"` to ensure correct text direction when displayed alongside LTR form labels (or vice versa). The adapter applies Unicode BiDi isolation marks when interpolating error text into mixed-direction contexts.
3. **Component Context**: Built-in validation messages are provided via the component's `Messages` struct (e.g., `TextFieldMessages.validation_required`, `DatePickerMessages.validation_invalid`). Custom validator functions receive the locale context and should return localized strings.

### 14.3 State Machine

#### 14.3.1 Form Component States

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Form is idle, ready for input.
    Idle,
    /// Form submission is in progress.
    Submitting,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Idle => write!(f, "idle"),
            State::Submitting => write!(f, "submitting"),
        }
    }
}
```

#### 14.3.2 Form Component Events

```rust
#[derive(Clone, Debug)]
pub enum Event {
    /// Triggered when the form is submitted.
    Submit,
    /// Submission completed (success or failure).
    SubmitComplete { success: bool },
    /// Triggered when the form is reset.
    Reset,
    /// Inject server-side validation errors by field name.
    SetServerErrors(BTreeMap<String, Vec<String>>),
    /// Clear all server-side validation errors.
    ClearServerErrors,
    /// Sync validation behavior from props change.
    SetValidationBehavior(ValidationBehavior),
    /// Set the status message (adapter-driven, for i18n support).
    /// Adapters send this after observing SubmitComplete to provide
    /// a locale-appropriate message via FormMessages.
    SetStatusMessage(Option<String>),
}
```

#### 14.3.3 Form Component Context

> **Authoritative source:** `form::Machine::Context` is the authoritative source for `server_errors` and `is_submitting`. `Context` mirrors these values downstream via the adapter effect (§14.6). On divergence, the machine context wins — `Context` is a read-only projection for child components that don't have access to the machine.
>
> **Sync timing:** The adapter MUST synchronize `Context` from `form::Machine::Context` synchronously within the same `apply` closure (not in a deferred effect). This ensures child components reading `Context` during the same render cycle see consistent values. The pattern is: `apply` updates `machine.context`, then immediately writes `form_context.is_submitting = machine.context.is_submitting` (and similarly for `server_errors`) before the closure returns.

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// How validation errors are displayed.
    pub validation_behavior: ValidationBehavior,
    /// Whether the form is currently submitting (authoritative; mirrored to Context).
    pub is_submitting: bool,
    /// Server-side validation errors keyed by field name (authoritative; mirrored to Context).
    pub server_errors: BTreeMap<String, Vec<String>>,
    /// Status message for the live region (success/error announcements).
    pub status_message: Option<String>,
    /// Result of the last submission attempt. `None` before first submit.
    pub last_submit_succeeded: Option<bool>,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

#### 14.3.4 Form Component Props

```rust
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// How validation errors are reported.
    pub validation_behavior: ValidationBehavior,
    /// Server-side validation errors keyed by field name. When this prop changes,
    /// `on_props_changed` sends `Event::SetServerErrors` to inject the errors into
    /// the machine context (and from there into `Context` for child fields).
    /// This is the declarative alternative to sending `Event::SetServerErrors` directly.
    pub validation_errors: BTreeMap<String, Vec<String>>,
    /// The URL to submit the form to. Sets the HTML `action` attribute on `<form>`.
    /// When `None`, the form submits to the current page URL (browser default).
    pub action: Option<String>,
    /// Optional ARIA role override for the form element. Set to `"search"` to
    /// create a search landmark (`role="search"`). When `None`, the `<form>`
    /// element uses its implicit role.
    pub role: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            validation_behavior: ValidationBehavior::default(),
            validation_errors: BTreeMap::new(),
            action: None,
            role: None,
        }
    }
}
```

#### 14.3.5 Form Component Full Machine Implementation

> **Relationship to `form_submit::Machine` (§8):** `form::Machine` is the high-level
> Form component machine with a simplified 2-state model (Idle, Submitting). It is
> intended for typical form use cases. `form_submit::Machine` (§8) is a lower-level
> 6-state machine with explicit validation states, designed for advanced flows
> (multi-step async validation, complex retry logic). They are NOT composed together —
> choose one based on your use case. Most consumers should use `form::Machine` via the
> Form component; use `form_submit::Machine` only when you need fine-grained control
> over the validation→submission lifecycle.

```rust
// inside form component module

pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ctx = Context {
            validation_behavior: props.validation_behavior,
            is_submitting: false,
            server_errors: props.validation_errors.clone(),
            status_message: None,
            last_submit_succeeded: None,
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // NOTE: `id` is immutable after init — ComponentIds are computed once
        // in init() and cached in Context. Changing id at runtime is not supported.
        let mut events = Vec::new();
        if old.validation_behavior != new.validation_behavior {
            events.push(Event::SetValidationBehavior(new.validation_behavior));
        }
        if old.validation_errors != new.validation_errors {
            if new.validation_errors.is_empty() {
                events.push(Event::ClearServerErrors);
            } else {
                events.push(Event::SetServerErrors(new.validation_errors.clone()));
            }
        }
        events
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Note: Unlike form_submit::Machine which accepts Submit from Idle|ValidationFailed,
            // the Form component only has Idle and Submitting states. Failed submissions return
            // to Idle (see SubmitComplete arm below), enabling re-submission without an explicit ValidationFailed state.
            // After success, the form also returns to Idle, allowing immediate re-submission
            // (e.g., "save" forms that can be submitted multiple times without page reload).
            //
            // **Adapter contract:** The adapter MUST call `Context::validate_all()` before
            // sending `Event::Submit`. If validation fails, the adapter does NOT send Submit —
            // it focuses the first invalid field and announces the error count via StatusRegion.
            // The Submit event indicates that client-side validation has already passed.
            (State::Idle, Event::Submit) => {
                Some(TransitionPlan::to(State::Submitting).apply(|ctx| {
                    ctx.is_submitting = true;
                    ctx.status_message = None;
                }))
            }
            (State::Submitting, Event::SubmitComplete { success }) => {
                let success = *success;
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.is_submitting = false;
                    ctx.last_submit_succeeded = Some(success);
                    // status_message is set to None here; the adapter layer is
                    // responsible for setting a locale-appropriate message via
                    // FormMessages after observing the SubmitComplete transition.
                    ctx.status_message = None;
                }))
            }
            (_, Event::Reset) => {
                let behavior = props.validation_behavior;
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.is_submitting = false;
                    ctx.last_submit_succeeded = None;
                    ctx.server_errors.clear();
                    ctx.status_message = None;
                    ctx.validation_behavior = behavior;
                }))
            }
            (_, Event::SetServerErrors(errors)) => {
                let errors = errors.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.server_errors = errors;
                }))
            }
            (_, Event::ClearServerErrors) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.server_errors.clear();
                }))
            }
            (_, Event::SetValidationBehavior(behavior)) => {
                let behavior = *behavior;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.validation_behavior = behavior;
                }))
            }
            (_, Event::SetStatusMessage(msg)) => {
                let msg = msg.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.status_message = msg;
                }))
            }
            // All other (state, event) combinations are intentionally dropped.
            // The machine only processes events that are valid for the current state;
            // e.g., Submit is ignored during Submitting to prevent double-submission.
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}
```

#### 14.3.6 Form Component Connect API

```rust
#[derive(ComponentPart)]
#[scope = "form"]
pub enum Part {
    Root,
    StatusRegion,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::StatusRegion => self.status_region_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.state.to_string());
        if self.ctx.validation_behavior == ValidationBehavior::Aria {
            attrs.set_bool(HtmlAttr::NoValidate, true);
        }
        if self.ctx.is_submitting {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }
        if let Some(ref action) = self.props.action {
            attrs.set(HtmlAttr::Action, action.as_str());
        }
        if let Some(ref role) = self.props.role {
            attrs.set(HtmlAttr::Role, role.as_str());
        }
        attrs
    }

    pub fn status_region_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Role, "status");
        // Note: role="status" implicitly carries aria-live="polite" per WAI-ARIA.
        // Explicit aria-atomic="true" retained for older AT that may not map role to atomic.
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StatusRegion.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn is_submitting(&self) -> bool {
        self.ctx.is_submitting
    }

    pub fn status_message(&self) -> Option<&str> {
        self.ctx.status_message.as_deref()
    }
}
```

### 14.4 Anatomy

| Part             | HTML Element | Description                                                             |
| ---------------- | ------------ | ----------------------------------------------------------------------- |
| **Root**         | `<form>`     | HTML form element. Handles `submit` and `reset` events.                 |
| **StatusRegion** | `<div>`      | Hidden live region for announcing submission results to screen readers. |

### 14.5 ARIA Mapping

| Attribute     | Element      | Source                          | Notes                                                       |
| ------------- | ------------ | ------------------------------- | ----------------------------------------------------------- |
| `novalidate`  | Root         | `validation_behavior == Aria`   | Suppresses browser validation when using ARIA-based display |
| `aria-busy`   | Root         | `ctx.is_submitting`             | Set `"true"` while form is submitting                       |
| `role`        | StatusRegion | `"status"`                      | Implicit `aria-live="polite"`                               |
| `aria-live`   | StatusRegion | (implicit from `role="status"`) | Announces submission results                                |
| `aria-atomic` | StatusRegion | `"true"`                        | Entire message announced as a unit                          |

### 14.6 Adapter Integration

Framework adapters must:

1. **Prevent default** on the `submit` event when `validation_behavior == Aria`
2. **Run validation** on all registered fields in the `Context` before calling the user's submit callback
3. **Inject server errors** into the appropriate `Field` components via `Event::SetServerErrors`
4. **Reset all fields** on `Event::Reset`, clearing values and errors
5. **Set `novalidate`** attribute on `<form>` when `validation_behavior == Aria`
6. **Set `aria-busy="true"`** on the form root while `ctx.is_submitting == true`; disable the submit button and set `aria-disabled="true"`
7. **Announce results** via the `StatusRegion`: on submit success, post a configurable success message; on submit failure with multiple errors, post "N errors found" before moving focus to the first invalid field
8. **Exclude disabled fields** from submission data (see §15)

> **Adapter integration:** The adapter syncs `form::Machine.Context.server_errors` with `Context.server_errors` via a reactive effect that watches the machine's context and calls `Context::set_server_errors()` whenever the machine's server_errors field changes.

### 14.7 I18n — FormMessages

```rust
/// Localizable messages for Form component announcements.
/// Follows the ComponentMessages pattern from `04-internationalization.md` §7.1.
/// `FormMessages` uses `MessageFn` (`Arc` on all targets) for closure fields
/// so the struct can derive `Clone`. `MessageFn<T>` implements `Debug` by printing
/// `"<closure>"`, so `#[derive(Debug)]` works without a manual impl.
///
/// Trait objects include `+ Send + Sync` in their type signature on all targets.
/// `MessageFn` uses shared `Arc` ownership internally, matching the callback
/// and validator closure patterns elsewhere in the codebase. See
/// `04-internationalization.md` §7.1.

#[derive(Clone, Debug)]
pub struct FormMessages {
    /// Announced via StatusRegion on successful submission.
    /// Default: "Form submitted successfully."
    pub submit_success: MessageFn<dyn Fn(&ars_i18n::Locale) -> String + Send + Sync>,
    /// Announced via StatusRegion on validation failure.
    /// Receives the error count. SHOULD use plural rules via `plural_category` for
    /// locale-correct pluralization (see `04-internationalization.md` §4.3).
    /// The built-in default provides English-only pluralization as a fallback;
    /// production apps MUST supply locale-aware messages.
    /// Default: "{count} errors found. Please correct the highlighted fields."
    pub submit_error_count: MessageFn<dyn Fn(usize, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Validation error message factories (used by `Error` factory methods).
    /// Default: "This field is required"
    pub required_error: MessageFn<dyn Fn(&ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Must be at least {min} characters"
    pub min_length_error: MessageFn<dyn Fn(usize, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Must be at most {max} characters"
    pub max_length_error: MessageFn<dyn Fn(usize, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Invalid format"
    pub pattern_error: MessageFn<dyn Fn(&ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Must be at least {min}"
    pub min_error: MessageFn<dyn Fn(f64, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Must be at most {max}"
    pub max_error: MessageFn<dyn Fn(f64, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Default: "Must be a valid email address"
    pub email_error: MessageFn<dyn Fn(&ars_i18n::Locale) -> String + Send + Sync>,
    /// Message for step mismatch errors. Receives step value.
    /// Default: "Please enter a valid value. The nearest allowed value is a multiple of {step}."
    pub step_error: MessageFn<dyn Fn(f64, &ars_i18n::Locale) -> String + Send + Sync>,
    /// Message for URL validation errors.
    /// Default: "Please enter a valid URL."
    pub url_error: MessageFn<dyn Fn(&ars_i18n::Locale) -> String + Send + Sync>,
}


impl Default for FormMessages {
    fn default() -> Self {
        Self {
            submit_success: MessageFn::new(|_locale| "Form submitted successfully.".into()),
            submit_error_count: MessageFn::new(|count, _locale| {
                // Note: Production apps should use plural_category(count, locale) for
                // locale-correct pluralization (see 04-internationalization.md §4.3).
                // This English-only default is a fallback for apps without i18n setup.
                if count == 1 {
                    "1 error found. Please correct the highlighted field.".into()
                } else {
                    format!("{} errors found. Please correct the highlighted fields.", count)
                }
            }),
            required_error: MessageFn::new(|_locale| "This field is required".into()),
            min_length_error: MessageFn::new(|min, _locale| format!("Must be at least {} characters", min)),
            max_length_error: MessageFn::new(|max, _locale| format!("Must be at most {} characters", max)),
            pattern_error: MessageFn::new(|_locale| "Invalid format".into()),
            min_error: MessageFn::new(|min, _locale| format!("Must be at least {}", min)),
            max_error: MessageFn::new(|max, _locale| format!("Must be at most {}", max)),
            email_error: MessageFn::new(|_locale| "Must be a valid email address".into()),
            step_error: MessageFn::new(|step, _locale| {
                format!("Please enter a valid value. The nearest allowed value is a multiple of {step}.")
            }),
            url_error: MessageFn::new(|_locale| {
                "Please enter a valid URL.".to_string()
            }),
        }
    }
}
```

#### 14.7.1 Canonical Default Pattern for Messages Structs

All `XyzMessages` structs that contain `MessageFn` closure fields follow the same `Default`
impl pattern shown above: each closure field receives a sensible English default via
`MessageFn::new(...)`. This enables zero-config usage while allowing full i18n override.

The `#[derive(Debug)]` works because `MessageFn<T>` implements `Debug` by printing `<closure>`,
satisfying the convention from `01-architecture.md` without requiring a manual `Debug` impl.

#### 14.7.2 FormMessages Provider Context and i18n Integration

Adapters MUST provide a **FormMessages provider context** so that all form components in a subtree share a single locale-appropriate `FormMessages` instance. This avoids requiring every field to manually pass a `FormMessages` reference.

**Provider pattern** (Leptos example; Dioxus follows the same shape):

```rust
// Adapter provides a context provider component:
#[component]
pub fn FormMessagesProvider(
    messages: FormMessages,
    children: Children,
) -> impl IntoView {
    provide_context(messages);
    children()
}

// Form components read the context (with English fallback):
fn use_form_messages() -> FormMessages {
    use_context::<FormMessages>().unwrap_or_default()
}
```

**i18n framework integration**: Applications using `ars-i18n` or external i18n frameworks (e.g., `leptos-i18n`, `fluent`) construct a `FormMessages` struct from their translation catalog and pass it through the provider:

```rust
use std::sync::Arc;
use ars_i18n::{plural_category, PluralCategory};

// Build FormMessages from an i18n catalog for the active locale.
fn form_messages_for_locale(locale: &Locale) -> FormMessages {
    let catalog = Arc::new(load_catalog(locale));
    let c1 = Arc::clone(&catalog);
    let c2 = Arc::clone(&catalog);
    let c3 = Arc::clone(&catalog);
    FormMessages {
        submit_success: MessageFn::new({
            let c = Arc::clone(&catalog);
            move |_locale| c.get("form.submit_success").into()
        }),
        submit_error_count: MessageFn::new(move |count, locale| {
            let category = plural_category(count, locale);
            c1.get_plural("form.submit_error_count", category, &[("count", count)])
        }),
        required_error: MessageFn::new(move |_locale| c2.get("form.required").into()),
        min_length_error: MessageFn::new(move |min, locale| {
            let category = plural_category(min, locale);
            c3.get_plural("form.min_length", category, &[("min", min)])
        }),
        // Remaining fields use English defaults via ..Default::default()
        ..Default::default()
    }
}
```

**Locale fallback strategy**: When no translation is available for the active locale, the provider MUST fall back in the following order:

1. **Exact locale** — e.g., `pt-BR`
2. **Language only** — e.g., `pt`
3. **English (`en`)** — the `Default` impl provides English strings
4. **Error code only** — if even English is unavailable (should not happen), display the `ErrorCode` variant name as a last resort

This fallback chain ensures that form error messages are always human-readable regardless of locale coverage gaps.

---

## 15. Disabled vs Readonly Contract

1. **Disabled**: element remains in tab order but is inoperable (per APG guidance, `aria-disabled` elements stay focusable so screen reader users can discover them), `aria-disabled="true"` set, no HTML `disabled` attribute (allows tooltip on hover). Adapter-layer code is responsible for excluding disabled fields from submission data. Core `Context::collect_form_data()` does not enforce this because disabled status is component-level state (on the component machine's `Context`), not form-level state. Adapters must maintain a `BTreeSet<String>` of disabled field names and filter the resulting `Data` before passing to the submit handler.
    > **Adapter pattern for disabled field exclusion:**
    > The adapter maintains a `BTreeSet<String>` of disabled field names, updated whenever a component's disabled prop changes. During `collect_form_data()`, the adapter filters the resulting `Data` by removing entries whose names appear in this set.
2. **Readonly**: element remains in tab order, `aria-readonly="true"` set, value visible but not editable. Form submission includes readonly values.
3. All form components must emit the correct ARIA attribute in their adapter render function — this is verified by snapshot tests.

**Exception:** `<fieldset>` elements use the native HTML `disabled` attribute because it natively
propagates disabled state to all contained form controls per HTML spec. **Note:** HTML `<fieldset disabled>` removes contained native inputs from the tab order (HTML spec behavior), which conflicts with the general disabled-focusable principle (see `03-accessibility.md` §13). Mitigation: individual form controls within the fieldset still use `aria-disabled="true"` per the general rule, ensuring ARIA-based disabled state is set independently of the fieldset's HTML disabled propagation. Adapters should not rely on fieldset-level HTML disabled alone for interactive state on individual controls.

**Adapter responsibility:** Since HTML `disabled` is not used, adapters MUST prevent interaction
on `aria-disabled` elements through event handler removal or CSS `pointer-events: none` on the
element (noting this prevents tooltip hover — use a wrapper element for tooltips on disabled fields).
