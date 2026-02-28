---
component: Form
category: utility
tier: stateless
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [field, fieldset]
references:
  radix-ui: Form
  react-aria: Form
---

# Form

The Form component renders a `<form>` element with pre-wired integration to `FormContext` and the form submission lifecycle. It handles `onSubmit`/`onReset` event binding, validation behavior selection, server-side error injection, and accessible status announcements.

> **Canonical specification:** The full state machine, validation behavior, i18n messages, and adapter integration are defined in `spec/foundation/07-forms.md` §14. This file provides an inline summary for quick reference.

**Ark UI equivalent:** — (no direct equivalent)
**React Aria equivalent:** Form

## 1. API

### 1.1 Props

See `07-forms.md` §14 for the full definition.

```rust
/// Controls how validation errors are reported to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationBehavior {
    /// Use native HTML constraint validation (browser tooltip UI).
    Native,
    /// Use ARIA-based validation display (custom ErrorMessage parts).
    Aria,
}

#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// How validation errors are reported. Default: `Aria`.
    pub validation_behavior: ValidationBehavior,
    /// Server-side validation errors keyed by field name. When set, the machine
    /// sends `Event::SetServerErrors` via `on_props_changed` to inject these
    /// errors into the form's field error display. This is the declarative
    /// alternative to the imperative `Event::SetServerErrors`.
    pub validation_errors: BTreeMap<String, Vec<String>>,
    /// The URL to submit the form to. Sets the `action` attribute on `<form>`.
    /// When `None`, the form submits to the current page URL (browser default).
    pub action: Option<String>,
    /// Optional ARIA role override for the form element. Set to `"search"` to
    /// create a search landmark (`role="search"`). When `None`, the `<form>`
    /// element uses its implicit role.
    pub role: Option<String>,
}
```

### 1.2 Connect / API

| Method                  | Purpose                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------ |
| `root_attrs()`          | `<form>` element — sets `novalidate` (Aria mode), `aria-busy` (submitting), `action` |
| `status_region_attrs()` | Hidden live region — `role="status"`, `aria-atomic="true"`                           |
| `is_submitting()`       | Whether the form is currently submitting                                             |
| `status_message()`      | Current status announcement text (success/error)                                     |

## 2. Anatomy

```text
Form
├── Root          <form>   data-ars-scope="form" data-ars-part="root"
└── StatusRegion  <div>    data-ars-part="status-region" role="status"
```

| Part         | Element  | Key Attributes                                                  |
| ------------ | -------- | --------------------------------------------------------------- |
| Root         | `<form>` | `data-ars-scope="form"`, `data-ars-part="root"`, `novalidate`   |
| StatusRegion | `<div>`  | `data-ars-part="status-region"`, `role="status"`, `aria-atomic` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute     | Element      | Source                        | Notes                                                       |
| ------------- | ------------ | ----------------------------- | ----------------------------------------------------------- |
| `novalidate`  | Root         | `validation_behavior == Aria` | Suppresses browser validation when using ARIA-based display |
| `aria-busy`   | Root         | `ctx.is_submitting`           | Set `"true"` while form is submitting                       |
| `role`        | StatusRegion | `"status"`                    | Implicit `aria-live="polite"`                               |
| `aria-atomic` | StatusRegion | `"true"`                      | Entire message announced as a unit                          |

Key adapter responsibilities (see `07-forms.md` §14.6 for the complete list):

- Prevent default on `submit` when `validation_behavior == Aria`
- Run validation on all registered fields before calling the submit callback
- Inject server errors into Field components via `Event::SetServerErrors`
- Announce results via the StatusRegion (success message or error count)

## 4. Internationalization

The `FormMessages` struct provides localizable messages for form announcements (submit success, error counts, validation messages). It uses `MessageFn` closures with `Send + Sync` bounds per the ComponentMessages pattern.

See `07-forms.md` §14.7 for the full `FormMessages` definition, default values, provider context pattern, and locale fallback strategy.

## 5. Library Parity

> Compared against: Radix UI (`Form`), React Aria (`Form`).

### 5.1 Props

| Feature             | ars-ui                | Radix UI                    | React Aria           | Notes                                                          |
| ------------------- | --------------------- | --------------------------- | -------------------- | -------------------------------------------------------------- |
| Validation behavior | `validation_behavior` | --                          | `validationBehavior` | Radix uses match-based validation; RA and ars-ui use mode enum |
| Server errors       | `validation_errors`   | `serverInvalid` (per-field) | `validationErrors`   | Radix uses per-field prop; ars-ui/RA use form-level map        |
| Action              | `action`              | --                          | `action`             | React Aria has action                                          |
| Role                | `role`                | --                          | `role`               | Both ars-ui and RA support search role                         |
| Clear server errors | --                    | `onClearServerErrors`       | --                   | Radix-specific callback                                        |

**Gaps:** None. Radix's `onClearServerErrors` is handled by ars-ui's form submission lifecycle (server errors are cleared on re-submit).

### 5.2 Anatomy

| Part          | ars-ui         | Radix UI        | React Aria | Notes                                                             |
| ------------- | -------------- | --------------- | ---------- | ----------------------------------------------------------------- |
| Root          | `Root`         | `Root`          | `Form`     | All libraries                                                     |
| StatusRegion  | `StatusRegion` | --              | --         | ars-ui addition for accessible announcements                      |
| Field         | --             | `Field`         | --         | Radix embeds Field in Form; ars-ui has separate Field component   |
| Label         | --             | `Label`         | --         | Radix embeds Label; ars-ui has separate Field.Label               |
| Control       | --             | `Control`       | --         | Radix-specific                                                    |
| Message       | --             | `Message`       | --         | Radix has match-based messages; ars-ui uses ErrorMessage in Field |
| ValidityState | --             | `ValidityState` | --         | Radix render-prop pattern                                         |
| Submit        | --             | `Submit`        | --         | Radix has Submit part; ars-ui uses Button with type="submit"      |

**Gaps:** None. Radix's richer Form anatomy (Field/Label/Control/Message/ValidityState/Submit) maps to ars-ui's separate `Field`, `Fieldset`, and `Button` components.

### 5.3 Features

| Feature               | ars-ui | Radix UI | React Aria |
| --------------------- | ------ | -------- | ---------- |
| Native validation     | Yes    | Yes      | Yes        |
| ARIA-based validation | Yes    | --       | Yes        |
| Server-side errors    | Yes    | Yes      | Yes        |
| Form status region    | Yes    | --       | --         |
| Submit prevention     | Yes    | --       | --         |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity.
- **Divergences:** Radix UI embeds Field/Label/Control/Message/Submit into Form anatomy; ars-ui keeps these as separate components. Radix uses `ValidityState` render prop and match-based `Message`; ars-ui uses `Field` ErrorMessage with `role="alert"`.
- **Recommended additions:** None.
