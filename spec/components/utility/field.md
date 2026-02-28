---
component: Field
category: utility
tier: stateless
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [fieldset]
references:
  ark-ui: Field
  radix-ui: Label
  react-aria: Label
---

# Field

Field provides the structural glue that connects a form control to its label, description, and error message. It manages the ID associations required for accessible form experiences.

> **Canonical specification:** The full state machine, validation lifecycle, and form context integration are defined in `spec/foundation/07-forms.md` §13. This file provides an inline summary for quick reference.

## 1. API

### 1.1 Props

> This is an adapter-level convenience summary. The canonical Props definition is in `07-forms.md` §13.2.4. The core machine Props are: `id`, `required`, `disabled`, `readonly`, `invalid`, `dir`.

```rust
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
```

### 1.2 Connect / API

The `Api` provides attribute maps for each anatomy part, ensuring correct ID linkage:

| Method                       | Purpose                                                                  |
| ---------------------------- | ------------------------------------------------------------------------ |
| `root_attrs()`               | Container element attributes (`data-ars-scope="field"`)                  |
| `label_attrs()`              | Label element — sets `for` attribute pointing to input ID                |
| `input_attrs()`              | Input element — sets `aria-describedby`, `aria-invalid`, `aria-required` |
| `description_attrs()`        | Help text element — sets ID referenced by input's `aria-describedby`     |
| `error_message_attrs()`      | Error message element — sets ID, `role="alert"`                          |
| `required_indicator_attrs()` | Required marker element — sets `aria-hidden="true"` (decorative)         |

## 2. Anatomy

```html
<div data-ars-scope="field" data-ars-part="root">
  <label data-ars-part="label" for="{input-id}">
    Email
    <span data-ars-part="required-indicator" aria-hidden="true">*</span>
  </label>
  <input
    data-ars-part="input"
    id="{input-id}"
    aria-describedby="{description-id} {error-id}"
    aria-invalid="true|false"
    aria-required="true|false"
  />
  <span data-ars-part="description" id="{description-id}">
    Enter your email address
  </span>
  <span data-ars-part="error-message" id="{error-id}" role="alert">
    Email is required
  </span>
</div>
```

### 2.1 RequiredIndicator

The `required-indicator` part renders a visual required marker (typically `*`) when the field's `required` prop is true. It is marked `aria-hidden="true"` because the `aria-required` attribute on the input already communicates the requirement to assistive technology.

Adapters MAY allow custom content for the indicator (e.g., a custom icon or localized text like "(required)").

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element           | Value                                      |
| ------------------ | ----------------- | ------------------------------------------ |
| `for`/`id`         | Label / Input     | Label associated with input via pairing    |
| `aria-describedby` | Input             | Space-separated IDs of description + error |
| `aria-invalid`     | Input             | `"true"` when validation fails             |
| `aria-required`    | Input             | `"true"` when field is required            |
| `role="alert"`     | ErrorMessage      | Dynamic error announcements                |
| `aria-hidden`      | RequiredIndicator | `"true"` (decorative)                      |

- Error messages use `role="alert"` for dynamic announcements (`role="alert"` implicitly sets `aria-live="assertive"` — do NOT additionally set `aria-live` to avoid double-announcement on NVDA+Firefox)
- Disabled fields set `aria-disabled="true"` and the native `disabled` attribute

## 4. Async Validation

During async validation, the Context field `validating: bool` is set to `true`, and `input_attrs()` emits `aria-busy="true"`. See `07-forms.md` §4 for the full async validation lifecycle.

## 5. Adapter Notes

### 5.1 Textarea Auto-Resize

Textarea height auto-adjustment is an adapter-level concern. Adapters MAY implement auto-resize using `ResizeObserver` to track content height or by measuring `scrollHeight` on input events and setting the element's height accordingly. This is not part of the core Field machine.

## 6. Library Parity

> Compared against: Ark UI (`Field`), Radix UI (`Label`), React Aria (`Label`).

### 6.1 Props

| Feature   | ars-ui              | Ark UI     | Radix UI  | React Aria | Notes                                       |
| --------- | ------------------- | ---------- | --------- | ---------- | ------------------------------------------- |
| Disabled  | `disabled`          | `disabled` | --        | --         | Ark has disabled; Radix/RA Label is simpler |
| Invalid   | `invalid`           | `invalid`  | --        | --         | Ark has invalid                             |
| Read-only | `readonly`          | `readOnly` | --        | --         | Ark has readOnly                            |
| Required  | `required`          | `required` | --        | --         | Ark has required                            |
| Dir       | `dir`               | --         | --        | --         | ars-ui addition                             |
| htmlFor   | via `label_attrs()` | --         | `htmlFor` | --         | Radix Label has htmlFor                     |

**Gaps:** None.

### 6.2 Anatomy

| Part              | ars-ui              | Ark UI              | Radix UI | React Aria | Notes                           |
| ----------------- | ------------------- | ------------------- | -------- | ---------- | ------------------------------- |
| Root              | `Root`              | `Root`              | --       | --         | Ark has Root container          |
| Label             | `Label`             | `Label`             | `Root`   | `Label`    | All libraries                   |
| Input             | `Input`             | `Input`             | --       | --         | Ark has Input                   |
| Description       | `Description`       | `HelperText`        | --       | --         | Different naming                |
| ErrorMessage      | `ErrorMessage`      | `ErrorText`         | --       | --         | Different naming                |
| RequiredIndicator | `RequiredIndicator` | `RequiredIndicator` | --       | --         | Ark has RequiredIndicator       |
| Textarea          | --                  | `Textarea`          | --       | --         | Ark has dedicated Textarea part |
| Select            | --                  | `Select`            | --       | --         | Ark has dedicated Select part   |

**Gaps:** None. Ark UI's `Textarea` and `Select` parts are handled by their own components in ars-ui.

### 6.3 Features

| Feature                    | ars-ui | Ark UI | Radix UI | React Aria |
| -------------------------- | ------ | ------ | -------- | ---------- |
| Label-input association    | Yes    | Yes    | Yes      | Yes        |
| aria-describedby wiring    | Yes    | Yes    | --       | --         |
| Error message (role=alert) | Yes    | Yes    | --       | --         |
| Required indicator         | Yes    | Yes    | --       | --         |
| Async validation           | Yes    | --     | --       | --         |

**Gaps:** None.

### 6.4 Summary

- **Overall:** Full parity with Ark UI. Radix/React Aria only provide a Label component without Field wrapping.
- **Divergences:** Ark UI has `Textarea` and `Select` as Field sub-parts; ars-ui delegates to dedicated component specs. Ark's `HelperText`/`ErrorText` map to ars-ui's `Description`/`ErrorMessage`.
- **Recommended additions:** None.
