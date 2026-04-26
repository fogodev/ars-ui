---
component: Field
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [fieldset, form]
references:
    ark-ui: Field
    radix-ui: Label
    react-aria: Label
---

# Field

Field is the canonical specification for the framework-agnostic `ars_components::utility::field`
machine. It provides the structural glue between a control, its label, description, and error
message, while leaving actual input rendering to the consuming component or adapter.

Shared validation types, field descriptors, and form context primitives live in
`spec/foundation/07-forms.md`. This file owns the Field machine, connect API, anatomy, and adapter
merge contract.

## 1. State Machine

### 1.1 State

`Field` has a single `Idle` state. All meaningful changes are context-only:
`required`, `disabled`, `readonly`, `invalid`, `validating`, `dir`, `errors`, and
`has_description`.

### 1.2 Props

The core machine props are:

- `id: String`
- `required: bool`
- `disabled: bool`
- `readonly: bool`
- `invalid: bool`
- `dir: Option<Direction>`

The `id` is immutable after initialization because `ComponentIds::from_id(&props.id)` is cached in
context.

### 1.3 Events

The machine accepts context-synchronization events:

- `SetErrors(Vec<Error>)`
- `ClearErrors`
- `SetHasDescription(bool)`
- `SetDisabled(bool)`
- `SetInvalid(bool)`
- `SetReadonly(bool)`
- `SetRequired(bool)`
- `SetDir(Option<Direction>)`
- `SetValidating(bool)`

`invalid` is derived as `props.invalid || !errors.is_empty()`.

### 1.4 Connect API

`field::Api` exposes:

- `root_attrs()`
- `label_attrs()`
- `input_attrs()`
- `description_attrs()`
- `error_message_attrs()`

The structural parts are:

- `Root`
- `Label`
- `Input`
- `Description`
- `ErrorMessage`

## 2. Anatomy

```text
Field
├── Root          <div>    data-ars-scope="field" data-ars-part="root"
├── Label         <label>  data-ars-part="label" for="{input-id}"
├── Input         <any>    data-ars-part="input" id="{input-id}"
├── Description   <span>   data-ars-part="description" id="{description-id}" (optional)
└── ErrorMessage  <span>   data-ars-part="error-message" id="{error-id}" role="alert"
```

The core Field contract does **not** define a `RequiredIndicator` part. Any visual required marker
is adapter- or consumer-owned presentation layered on top of the machine’s `required` state.

## 3. Accessibility

### 3.1 ARIA Wiring

`input_attrs()` is responsible for:

- `id`
- `aria-labelledby` -> label ID
- `aria-describedby` -> description ID and/or error message ID when present
- `aria-required="true"` when required
- `aria-invalid="true"` when invalid
- `aria-errormessage` when invalid and errors are present
- `aria-disabled="true"` when disabled
- `aria-readonly="true"` when readonly
- `aria-busy="true"` when validating

`error_message_attrs()` sets `role="alert"` and hides the node when there are no errors.

### 3.2 Disabled Contract

Field emits `aria-disabled`, not a native `disabled` attribute. This is intentional: Field is a
structural wrapper, not the concrete input element. The consuming input component or adapter decides
whether the underlying control should also receive native `disabled`.

## 4. Integration

### 4.1 Fieldset Merge

When nested inside `Fieldset`, the adapter reads parent field context and merges it before
constructing `field::Props`:

- `effective_disabled = field_props.disabled || fieldset_ctx.disabled`
- `effective_invalid = field_props.invalid || fieldset_ctx.invalid`
- `effective_readonly = field_props.readonly || fieldset_ctx.readonly`

The merge happens at the adapter layer so the core machine stays framework-agnostic.

### 4.2 Shared Form Contracts

Use `07-forms.md` for:

- `validation::Error`
- field descriptors and ID association helpers
- form context registration and validation flow
- hidden-input participation for composite widgets

## 5. Library Parity

Compared against Ark UI `Field`, Radix UI `Label`, and React Aria `Label`:

- ars-ui matches Ark UI’s label/description/error wiring model.
- ars-ui intentionally does **not** model Ark’s presentational `RequiredIndicator` as a shared core
  part.
- Radix UI and React Aria provide the label primitive but not the full structural field wrapper.
