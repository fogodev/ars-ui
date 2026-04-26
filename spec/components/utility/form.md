---
component: Form
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [field, fieldset, form-submit]
references:
    radix-ui: Form
    react-aria: Form
---

# Form

Form is the canonical specification for the framework-agnostic `ars_components::utility::form`
machine. It models the high-level `<form>` component lifecycle for common cases: submit, reset,
server-error synchronization, validation behavior selection, and status announcements.

Shared validator types, form registry/context, and the domain-level `form::Messages` bundle live in
`spec/foundation/07-forms.md`. This file owns the Form component machine and connect API.

## 1. State Machine

### 1.1 Validation Behavior

`ValidationBehavior` has two variants:

- `Native`
- `Aria`

`Aria` is the default.

### 1.2 States

- `Idle`
- `Submitting`

### 1.3 Events

- `Submit`
- `SubmitComplete { success: bool }`
- `Reset`
- `SetServerErrors(BTreeMap<String, Vec<String>>)`
- `ClearServerErrors`
- `SetValidationBehavior(ValidationBehavior)`
- `SetStatusMessage(Option<String>)`

### 1.4 Context

The machine context stores:

- `validation_behavior`
- `is_submitting`
- `server_errors`
- `status_message`
- `last_submit_succeeded`
- `ids`

### 1.5 Props

The core machine props are:

- `id: String`
- `validation_behavior: ValidationBehavior`
- `validation_errors: BTreeMap<String, Vec<String>>`
- `action: Option<String>`
- `role: Option<String>`

The machine uses `type Messages = ()`. Localized wording is resolved separately through the
domain-level `ars_forms::form::Messages` bundle.

### 1.6 Connect API

`form::Api` exposes:

- `root_attrs()`
- `status_region_attrs()`
- `is_submitting()`
- `status_message()`

The structural parts are:

- `Root`
- `StatusRegion`

## 2. Anatomy

```text
Form
├── Root          <form>  data-ars-scope="form" data-ars-part="root"
└── StatusRegion  <div>   data-ars-part="status-region" role="status"
```

## 3. Accessibility

### 3.1 Root Attributes

`root_attrs()` emits:

- `data-ars-state="idle|submitting"`
- `novalidate` when `validation_behavior == Aria`
- `aria-busy="true"` while submitting
- sanitized `action` when present
- optional `role` override when present

### 3.2 Status Region

`status_region_attrs()` emits:

- `role="status"`
- `aria-live="polite"`
- `aria-atomic="true"`

The region exists so adapters can announce submit success and failure outcomes without coupling that
wording to the machine itself.

## 4. Integration

### 4.1 Adapter Responsibilities

Adapters must:

1. Prevent default submit behavior when `validation_behavior == Aria`.
2. Run validation on registered fields before dispatching `Event::Submit`.
3. Synchronize server errors into child fields.
4. Reset registered field state on `Reset`.
5. Resolve localized status text through `ars_forms::form::Messages` and send it via
   `SetStatusMessage`.

### 4.2 Shared Form Contracts

Use `07-forms.md` for:

- `ars_forms::form::Context`
- validator registration and execution
- domain-level `form::Messages`
- hidden-input participation and submission data behavior

### 4.3 Relationship to `form_submit::Machine`

`form::Machine` is the high-level component machine for typical form rendering. Use
[`form-submit`](form-submit.md) when you need the lower-level validation/submission lifecycle with
explicit `Validating`, `ValidationFailed`, `Succeeded`, and `Failed` states.

## 5. Library Parity

Compared against Radix UI `Form` and React Aria `Form`:

- ars-ui matches the semantic `<form>` surface and validation-mode split.
- ars-ui keeps `Field`, `Fieldset`, and `Button` as separate building blocks rather than embedding
  them inside Form anatomy.
- ars-ui adds a dedicated status region so submit announcements are a first-class contract.
