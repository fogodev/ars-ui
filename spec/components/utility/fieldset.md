---
component: Fieldset
category: utility
tier: stateless
foundation_deps: [architecture, accessibility, forms]
shared_deps: []
related: [field]
references:
  ark-ui: Fieldset
---

# Fieldset

Groups related form fields with `<fieldset>`/`<legend>` semantics and shared disabled/error state propagation. When `disabled` is set on the fieldset, all child fields inherit the disabled state. Error messages can be associated with the group as a whole.

> **Canonical specification:** The full state machine and form context integration are defined in `spec/foundation/07-forms.md` §12. This file provides an inline summary for quick reference.

**Ark UI equivalent:** Fieldset
**React Aria equivalent:** — (no direct equivalent; uses native `<fieldset>`)

## 1. API

### 1.1 Props

> This is an adapter-level convenience summary. The canonical Props definition is in `07-forms.md` §12.2.4.

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
```

### 1.2 Connect / API

The canonical state machine and full connect API are defined in `spec/foundation/07-forms.md` §12. The summary below covers the key attribute outputs.

```rust
#[derive(ComponentPart)]
#[scope = "fieldset"]
pub enum Part {
    Root,
    Legend,
    Description,
    ErrorMessage,
}

/// Attributes for the root `<fieldset>` element.
pub fn root_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Id, self.ctx.ids.id());
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    // Native <fieldset disabled> propagates to all contained form elements.
    if self.ctx.disabled {
        attrs.set_bool(HtmlAttr::Disabled, true);
    }
    if let Some(dir) = self.ctx.dir {
        attrs.set(HtmlAttr::Dir, dir.as_html_attr());
    }
    let mut describedby_parts: Vec<String> = Vec::new();
    describedby_parts.push(self.ctx.ids.part("description"));
    if !self.ctx.errors.is_empty() {
        describedby_parts.push(self.ctx.ids.part("error-message"));
    }
    if !describedby_parts.is_empty() {
        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            describedby_parts.join(" "),
        );
    }
    attrs
}

/// Attributes for the `<legend>` element.
pub fn legend_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Id, self.ctx.ids.part("legend"));
    let [(_, _), (part_attr, part_val)] = Part::Legend.data_attrs();
    attrs.set(part_attr, part_val);
    attrs
}

/// Attributes for the description/help text element.
pub fn description_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
    let [(_, _), (part_attr, part_val)] = Part::Description.data_attrs();
    attrs.set(part_attr, part_val);
    attrs
}

/// Attributes for the error message element.
pub fn error_message_attrs(&self) -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
    attrs.set(HtmlAttr::Role, "alert");
    let [(_, _), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
    attrs.set(part_attr, part_val);
    if self.ctx.errors.is_empty() {
        attrs.set_bool(HtmlAttr::Hidden, true);
    }
    attrs
}
```

## 2. Anatomy

```text
Fieldset
├── Root          <fieldset>  data-ars-scope="fieldset" data-ars-part="root"
├── Legend         <legend>   data-ars-part="legend"
├── Description   <div>      data-ars-part="description" (optional)
├── {child fields}            Field components inherit disabled state via Context
└── ErrorMessage  <span>     data-ars-part="error-message" role="alert"
```

| Part         | Element      | Key Attributes                                                  |
| ------------ | ------------ | --------------------------------------------------------------- |
| Root         | `<fieldset>` | `data-ars-scope="fieldset"`, `data-ars-part="root"`, `disabled` |
| Legend       | `<legend>`   | `data-ars-part="legend"`                                        |
| Description  | `<div>`      | `data-ars-part="description"`, wired via `aria-describedby`     |
| ErrorMessage | `<span>`     | `data-ars-part="error-message"`, `role="alert"`                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- Renders as `<fieldset>` with `<legend>` for the legend text. The native `<fieldset>` carries an implicit role — no explicit `role` attribute is needed.
- When `disabled`, sets native `disabled` on the fieldset element (not just `aria-disabled`), which propagates to all contained form controls.
- Error message linked via `aria-describedby` on the fieldset element pointing to the error message part ID.
- **Note:** `aria-invalid` is intentionally NOT set on the `<fieldset>` element. Screen readers (particularly NVDA and JAWS) do not reliably announce `aria-invalid` on fieldset elements. Validation errors are communicated through the `ErrorMessage` anatomy part (which uses `role="alert"`) and through individual Field components within the fieldset.

When rendering as a native `<fieldset>` with `<legend>`, no explicit `aria-labelledby` is needed — the browser provides implicit labeling. If an adapter renders a non-native container element (e.g., `<div role="group">`), it MUST set `aria-labelledby` pointing to the legend element's ID to maintain the accessible name association.

## 4. Adapter Context Propagation

Adapters MUST provide `Context` to descendant fields via framework context (`provide_context` in Leptos, `use_context_provider` in Dioxus). Child Field components consume this context to inherit disabled, invalid, and readonly states from their parent Fieldset.

## 5. Library Parity

> Compared against: Ark UI (`Fieldset`).

### 5.1 Props

| Feature   | ars-ui     | Ark UI    | Notes                                              |
| --------- | ---------- | --------- | -------------------------------------------------- |
| Invalid   | `invalid`  | `invalid` | Both libraries                                     |
| Disabled  | `disabled` | --        | ars-ui addition; Ark Fieldset has no disabled prop |
| Read-only | `readonly` | --        | ars-ui addition                                    |
| Dir       | `dir`      | --        | ars-ui addition                                    |

**Gaps:** None.

### 5.2 Anatomy

| Part         | ars-ui         | Ark UI       | Notes            |
| ------------ | -------------- | ------------ | ---------------- |
| Root         | `Root`         | `Root`       | Both libraries   |
| Legend       | `Legend`       | `Legend`     | Both libraries   |
| Description  | `Description`  | `HelperText` | Different naming |
| ErrorMessage | `ErrorMessage` | `ErrorText`  | Different naming |

**Gaps:** None.

### 5.3 Summary

- **Overall:** Full parity.
- **Divergences:** Ark's `HelperText`/`ErrorText` map to ars-ui's `Description`/`ErrorMessage`. ars-ui adds `disabled` and `readonly` propagation.
- **Recommended additions:** None.
