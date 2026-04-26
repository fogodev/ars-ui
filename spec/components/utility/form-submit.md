---
component: FormSubmit
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [form, field, fieldset]
references:
    react-hook-form: handleSubmit
---

# FormSubmit

FormSubmit is the canonical specification for the framework-agnostic
`ars_components::utility::form_submit` machine. It models the lower-level validation and submit
lifecycle for advanced forms that need explicit validation, submission, success, and failure
states.

Shared validator types and form registry/context primitives live in `spec/foundation/07-forms.md`.
This file owns the FormSubmit machine and connect API.

## 1. State Machine

### 1.1 States

- `Idle`
- `Validating`
- `ValidationFailed`
- `Submitting`
- `Succeeded`
- `Failed`

### 1.2 Events

- `Submit`
- `ValidationPassed`
- `ValidationFailed`
- `SubmitComplete`
- `SubmitError(String)`
- `Reset`
- `SetServerErrors(BTreeMap<String, Vec<String>>)`
- `SetMode(Mode)`

### 1.3 Context

The machine context stores:

- `form: ars_forms::form::Context`
- `ids: ComponentIds`
- `submit_error: Option<String>`
- `sync_valid: bool`

### 1.4 Props

The core machine props are:

- `id: String`
- `validation_mode: Mode`
- `spawn_async_validation: Callback<SpawnAsyncValidationFn>`
- `schedule_microtask: Callback<ScheduleMicrotaskFn>`

`form_submit::Machine` uses `type Messages = ()`. Announcement wording belongs to the adapter and
the domain-level form message bundle rather than to the lifecycle machine itself.

### 1.5 Effects and Ownership Boundary

The machine owns the lifecycle and effect registration names:

- `"async-validation"`
- `"submit"`

Adapters own the actual runtime integration:

- async task spawning and cancellation
- microtask scheduling
- user submit callback invocation
- request cleanup on disposal

## 2. Connect API

`form_submit::Api` exposes:

- `root_attrs()`
- `submit_button_attrs()`
- `is_submitting()`
- `is_valid()`
- `submit_error()`
- `on_form_submit()`

The structural parts are:

- `Root`
- `SubmitButton`

## 3. Anatomy

```text
FormSubmit
├── Root          <form>    data-ars-scope="form-submit" data-ars-part="root"
└── SubmitButton  <button>  data-ars-part="submit-button"
```

`root_attrs()` emits the machine state token via `data-ars-state`. `submit_button_attrs()` applies
busy and disabled semantics while the machine is in `Submitting`.

## 4. Accessibility

### 4.1 Submit Button Behavior

While submitting, `submit_button_attrs()` emits:

- `aria-busy="true"`
- `aria-disabled="true"`
- native `disabled`

This is an intentional exception to the general “prefer ARIA disabled over native disabled” pattern
because duplicate form submission is more harmful than temporary discoverability loss during the
brief submitting window.

## 5. Integration

Use FormSubmit when you need:

- explicit `Validating` and `ValidationFailed` states
- async validators that may run alongside sync validation
- a distinct `Succeeded` or `Failed` resting state
- request cancellation tied to machine cleanup

Use [`Form`](form.md) instead when the simpler `Idle`/`Submitting` surface is sufficient.

Shared validation rules, `Mode`, and `ars_forms::form::Context` behavior are specified in
`07-forms.md`.
