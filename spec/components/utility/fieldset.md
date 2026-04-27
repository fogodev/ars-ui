---
component: Fieldset
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [field, form]
references:
    ark-ui: Fieldset
---

# Fieldset

Fieldset is the canonical specification for the framework-agnostic
`ars_components::utility::fieldset` machine. It models shared disabled, invalid, and readonly
state for a group of related fields while preserving native `<fieldset>` and `<legend>` semantics.

Shared child-field context contracts live in `spec/foundation/07-forms.md`. This file owns the
Fieldset machine, connect API, anatomy, and root attribute behavior.

## 1. State Machine

### 1.1 State

`Fieldset` has a single `Idle` state. Context holds:

- `disabled`
- `invalid`
- `readonly`
- `dir`
- `errors`
- `has_description`
- `ids`

### 1.2 Props

The core machine props are:

- `id: String`
- `disabled: bool`
- `invalid: bool`
- `readonly: bool`
- `dir: Option<Direction>`

The `id` is immutable after initialization because the derived `ComponentIds` are cached in
context.

Construct via the inherent builder: `Props::new()` returns the default; setters (`id`, `disabled`,
`invalid`, `readonly`, `dir`) accept the natural argument and return `Self` for chaining. The
`dir` setter accepts a `Direction` and wraps it in `Some` automatically.

```rust
let props = fieldset::Props::new()
    .id("billing-address")
    .disabled(true)
    .dir(Direction::Rtl);
```

### 1.3 Events

The machine accepts context-synchronization events:

- `SetErrors(Vec<Error>)`
- `ClearErrors`
- `SetDisabled(bool)`
- `SetInvalid(bool)`
- `SetReadonly(bool)`
- `SetDir(Option<Direction>)`
- `SetHasDescription(bool)`

`invalid` is derived as `props.invalid || !errors.is_empty()`.

### 1.4 Connect API

`fieldset::Api` exposes:

- `root_attrs()`
- `legend_attrs()`
- `description_attrs()`
- `error_message_attrs()`
- `content_attrs()`
- `errors()`
- `is_disabled()`
- `is_invalid()`
- `is_readonly()`

The structural parts are:

- `Root`
- `Legend`
- `Description`
- `ErrorMessage`
- `Content`

## 2. Anatomy

```text
Fieldset
├── Root          <fieldset>  data-ars-scope="fieldset" data-ars-part="root"
├── Legend        <legend>    data-ars-part="legend"
├── Description   <span>      data-ars-part="description" id="{description-id}" (optional)
├── ErrorMessage  <span>      data-ars-part="error-message" id="{error-id}" role="alert"
└── Content       <div>       data-ars-part="content"
```

## 3. Accessibility

### 3.1 Root Attributes

`root_attrs()` emits:

- native `disabled` when the group is disabled
- `dir` when present
- `aria-describedby` including the description ID only when the description part is rendered
- `aria-describedby` including the error-message ID only when errors are present

### 3.2 Invalid Contract

Fieldset intentionally does **not** set `aria-invalid` on the root `<fieldset>`. Screen readers do
not reliably announce `aria-invalid` there. Group-level invalidity is expressed through the
error-message part and through descendant Field/input wiring.

## 4. Integration

### 4.1 Child Field Context

Adapters publish shared field context from Fieldset to descendant Field components. Child Field
instances merge parent `disabled`, `invalid`, and `readonly` values before constructing
`field::Props`.

### 4.2 Shared Form Contracts

Use `07-forms.md` for:

- the shared child-field context shape
- form registration and validation coordination
- validator and error types

## 5. Library Parity

Compared against Ark UI `Fieldset`:

- ars-ui matches the grouped `<fieldset>` / `<legend>` model.
- ars-ui explicitly models a `Content` part for child composition.
- ars-ui adds shared `disabled`, `invalid`, and `readonly` propagation because those states are
  central to the broader form architecture.
