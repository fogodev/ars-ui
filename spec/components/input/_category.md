# Input Components Specification

Cross-references: `00-overview.md` for naming conventions and data attributes,
`01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and `Service`.
`03-accessibility.md` for focus management, ARIA patterns, and keyboard navigation.
`04-internationalization.md` for locale handling, number formatting, and RTL support.
`07-forms.md` for form participation, validation, and hidden input patterns.

---

## Table of Contents

- [Checkbox](checkbox.md)
- [CheckboxGroup](checkbox-group.md)
- [RadioGroup](radio-group.md)
- [Switch](switch.md)
- [TextField](text-field.md)
- [Textarea](textarea.md)
- [NumberInput](number-input.md)
- [PinInput](pin-input.md)
- [Slider](slider.md)
- [RangeSlider](range-slider.md)
- [SearchInput](search-input.md)
- [Editable](editable.md)
- [PasswordInput](password-input.md)
- [FileTrigger](file-trigger.md)

---

## Overview

Input components are the building blocks of data entry in ars-ui. They span boolean selection
(Checkbox, Radio, Switch), text entry (TextField, Textarea, SearchInput), numeric entry
(NumberInput), multi-cell PIN entry (PinInput), and range selection (Slider, RangeSlider).

All input components share these traits:

- **Form participation**: Each renders a hidden `<input>` so its value joins native form
  submission.
- **Controlled/uncontrolled duality**: Every value uses `Bindable<T>` so both modes work
  identically.
- **Label association**: Every component exposes a `Label` part; the connect API wires the
  `for`/`id` relationship automatically.
- **`aria-errormessage` wiring**: When `invalid === true`, components set both
  `aria-describedby` (description + error ID) and `aria-errormessage` (error ID only).
  `aria-errormessage` provides a direct machine-readable link to the error, while
  `aria-describedby` includes both the description and the error for a complete announcement.
  The `aria-errormessage` attribute MUST be omitted when the component is not invalid
  (pointing to a hidden error element is a WCAG violation).
- **Error and description linkage**: `aria-describedby` is built from `Description` and
  `ErrorMessage` anatomy parts. The standard wiring pattern for the primary interactive
  element's attrs method is:

    ```rust,no_check
    // Build aria-describedby from description + error message parts.
    // IMPORTANT: Only reference IDs for parts that are actually rendered,
    // otherwise the aria-describedby will point to a non-existent element
    // (a "dangling reference"), which confuses assistive technology.
    let mut describedby_parts = Vec::new();
    if self.ctx.has_description {
        describedby_parts.push(self.ctx.ids.part("description"));
    }
    if self.ctx.invalid && self.ctx.has_error_message {
        describedby_parts.push(self.ctx.ids.part("error-message"));
    }
    if !describedby_parts.is_empty() {
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
    }
    ```

    **Dangling reference prevention**: The error ID MUST only be appended to `aria-describedby`
    when the component is in an invalid state AND the error message element is actually rendered
    in the DOM (i.e., `invalid && has_error_message`). If the error message element is not
    rendered, including its ID creates a dangling ARIA reference to a non-existent element, which
    confuses assistive technology. The same guard applies to `aria-errormessage`.

    **Multiple IDs order in `aria-describedby`**: When multiple IDs are present in
    `aria-describedby`, the order MUST be: description ID first, then error ID. Example:
    `aria-describedby="field-desc field-error"`. Screen readers announce referenced elements in
    the order the IDs are listed, so placing the description before the error ensures the user
    hears contextual help before the error message.

- **Error announcement strategy**: Error messages are announced to assistive technology
  through a two-part mechanism:
    1. **`aria-describedby`**: The primary interactive element's `aria-describedby` points to
       the error-message part ID (when invalid). Screen readers announce this on focus.
    2. **`aria-live="polite"` region** (optional): For real-time validation (e.g., as-you-type),
       the error-message part may include `aria-live="polite"` so changes are announced without
       requiring focus movement. Components using `role="alert"` (which implies
       `aria-live="assertive"`) should NOT also set `aria-live` explicitly to avoid
       double-announcement.
    3. **Multi-field form batching**: When validating multiple fields simultaneously (e.g., on
       form submit), adapters SHOULD debounce error announcements to avoid overwhelming screen
       reader users. A recommended pattern is to announce a summary (e.g., "3 errors found")
       via a single live region, then let individual field errors be discovered via
       `aria-describedby` on focus.
- **Disabled and readonly propagation**: Both states flow to ARIA attributes and native HTML.
- **Focus visibility**: Keyboard-initiated focus emits `data-ars-focus-visible`; pointer does
  not.

- **`on_focus_change` callback**: All input components accept an optional
  `on_focus_change: Option<Callback<bool>>` prop as a convenience over separate Focus/Blur
  event handling. The callback fires with `true` on Focus and `false` on Blur. This is a
  convenience shorthand — consumers who need richer focus information (e.g., `is_keyboard`)
  should handle the individual `Focus` and `Blur` events directly. Adapters fire
  `on_focus_change` in the same microtask as the underlying focus/blur transition.

### Disabled State Coordination

Components may be disabled by multiple independent sources: a direct `disabled` prop, a
`DisabledContext` provided by a parent (e.g., a `FieldSet` or `Form`), or per-item flags
(e.g., per-thumb disabled in RangeSlider). The effective disabled state is the logical OR
of all sources — if **any** source is `true`, the component is disabled.

| Component     | `disabled` prop | `DisabledContext` | Per-item disabled | Guard logic                                                                |
| ------------- | :-------------: | :---------------: | :---------------: | -------------------------------------------------------------------------- |
| Checkbox      |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| CheckboxGroup |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`; propagates to children via ChildCtx  |
| RadioGroup    |       Yes       |        Yes        |  Per-radio item   | Group: `ctx.disabled \|\| disabled_context`; Item: also `item.disabled`    |
| Switch        |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| TextField     |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| Textarea      |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| PasswordInput |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| NumberInput   |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| PinInput      |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| Slider        |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| RangeSlider   |       Yes       |        Yes        |     Per-thumb     | `ctx.disabled \|\| disabled_context`; Per-thumb: also `start/end_disabled` |
| SearchInput   |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| Editable      |       Yes       |        Yes        |        No         | `ctx.disabled \|\| disabled_context`                                       |
| FileTrigger   |       Yes       |        No         |        No         | `props.disabled` (stateless — no context)                                  |

Every transition guard that checks `ctx.disabled` MUST evaluate the effective disabled
state: `ctx.disabled || self.disabled_context()`. Adapters resolve `DisabledContext` during
`connect` and merge it into the guard check. This prevents a scenario where the prop is
`false` but the context is `true`, yet events are still processed.

> **Important**: `placeholder` is NOT a substitute for a label. All inputs MUST have an
> associated `<label>` element, `aria-label`, or `aria-labelledby`. Placeholder text
> disappears on input and is not reliably announced by screen readers.

### Label Visibility (WCAG 2.5.3 Label in Name)

All input components must have a visible label whose text is included in the accessible name.
Using `aria-label` alone without a visible label fails WCAG 2.5.3. Recommended patterns:

1. **Visible `<label>` element** with `for` attribute (preferred).
2. **`aria-labelledby`** pointing to a visible element.
3. **If a visible label is truly impossible** (e.g., icon-only search field), provide `aria-label`
   AND `title` attribute with matching text.

### IME Composition Protocol

All text-input components (`TextField`, `Textarea`, `NumberInput`, `SearchInput`, `ComboBox`, `DateField`,
`TimeField`) must implement the IME composition protocol:

1. Track `is_composing: bool` in machine context — set `true` on `compositionstart`, `false`
   on `compositionend`.
2. During composition, suppress: typeahead search, value validation, auto-formatting, and
   change callbacks.
3. On `compositionend`, apply the final composed value and resume normal processing.
4. Adapters register `compositionstart`, `compositionupdate`, and `compositionend` event
   listeners on all text inputs.

| Component       | Value Type        | Machine States                      | Key ARIA Pattern                     |
| --------------- | ----------------- | ----------------------------------- | ------------------------------------ |
| `Checkbox`      | `checkbox::State` | Unchecked / Checked / Indeterminate | `role="checkbox"`, `aria-checked`    |
| `CheckboxGroup` | `BTreeSet<Key>`   | Idle                                | `role="group"`, child propagation    |
| `RadioGroup`    | `Option<String>`  | Idle / Focused                      | `role="radiogroup"`, roving tabindex |
| `Switch`        | `bool`            | Off / On                            | `role="switch"`, `aria-checked`      |
| `TextField`     | `String`          | Idle / Focused                      | `role` implicit, `aria-invalid`      |
| `Textarea`      | `String`          | Idle / Focused                      | `role` implicit, `aria-invalid`      |
| `PasswordInput` | `String`          | Masked / Visible                    | `type="password"/"text"`, toggle     |
| `NumberInput`   | `Option<f64>`     | Idle / Focused / Scrubbing          | `role="spinbutton"`                  |
| `PinInput`      | `Vec<String>`     | Idle / Focused / Completed          | per-cell `aria-label`                |
| `Slider`        | `f64`             | Idle / Focused / Dragging           | `role="slider"`                      |
| `RangeSlider`   | `[f64; 2]`        | Idle / Focused / Dragging           | dual `role="slider"`                 |
| `SearchInput`   | `String`          | Idle / Focused / Searching          | `role="search"`                      |
| `Editable`      | `String`          | Preview / Editing                   | `role="group"`, inline edit          |
| `FileTrigger`   | (Platform files)  | (Stateless)                         | hidden `<input type="file">`         |

---

## Appendix A: Shared Data Attributes

All input components emit these data attributes on their Root part:

| Attribute                | Condition                      |
| ------------------------ | ------------------------------ |
| `data-ars-disabled`      | Component is disabled          |
| `data-ars-readonly`      | Component is readonly          |
| `data-ars-invalid`       | Component has validation error |
| `data-ars-required`      | Component is required          |
| `data-ars-focus-visible` | Keyboard focus is active       |
| `data-ars-state`         | Component-specific state token |

## Appendix B: Shared Conventions

### `Bindable<T>` Pattern

Every input value uses `Bindable<T>` (defined in `01-architecture.md`):

```rust,no_check
// Uncontrolled: consumer provides default_value, machine owns state
let checkbox = Props { default_checked: checkbox::State::Checked, ..Default::default() };

// Controlled: consumer provides value, adapter handles change callbacks
let checkbox = Props {
    checked: Some(checkbox::State::Checked),
    ..Default::default()
};
```

### `AttrMap` Spreading

The connect API returns `AttrMap` values that framework adapters spread onto DOM elements:

```rust,no_check
// Leptos example
let api = Machine::connect(&state, &ctx, &props, &send);
view! {
    <div {..api.root_attrs()}>
        <label {..api.label_attrs()}>"Accept terms"</label>
        <div {..api.control_attrs()}>
            <div {..api.indicator_attrs()} />
        </div>
    </div>
}
```

### Focus Visibility

All input components track `focus_visible` independently from `focused`:

- `focused`: Set on any `Focus` event.
- `focus_visible`: Set only when `Focus { is_keyboard: true }`.
- `data-ars-focus-visible` attribute only appears when `focus_visible=true`.
- CSS can use `[data-ars-focus-visible]:focus { outline: 2px solid blue; }` for keyboard-only
  focus rings.

### Form Participation

Every input component renders a hidden `<input>` (or `<input type="hidden">`) for native
form submission.

> **Shadow DOM constraint:** The hidden input MUST be in the same DOM context as the
> `<form>` element. If a component uses Shadow DOM, the hidden input must be rendered
> in the light DOM (e.g., via a `<slot>`) or use an explicit `form` attribute pointing
> to the form's `id` so the input participates in form submission across shadow
> boundaries. Adapters using Shadow DOM MUST implement one of these strategies.

Per-component hidden input patterns:

- `Checkbox`: `<input type="checkbox" name="..." value="..." checked>`
- `CheckboxGroup`: `<input type="checkbox" name="..." value="..." checked>` per checked value
- `RadioGroup`: `<input type="radio" name="..." value="..." checked>` per item
- `Switch`: `<input type="checkbox" name="..." value="..." checked>`
- `TextField`: value is on the visible `<input>` itself
- `Textarea`: value is on the visible `<textarea>` itself
- `PasswordInput`: value is on the visible `<input>` itself
- `NumberInput`: value is on the visible `<input>` itself (role="spinbutton")
- `PinInput`: `<input type="hidden" name="..." value="...">` (combined string)
- `Slider`: `<input type="hidden" name="..." value="...">`
- `RangeSlider`: `<input type="hidden" name="...[0]" value="...">` + `name="...[1]"` (two inputs)
- `SearchInput`: value is on the visible `<input type="search">` itself
- `Editable`: no hidden input by default (consumer wraps if needed)
- `FileTrigger`: `<input type="file">` (visually hidden, native file submission)
