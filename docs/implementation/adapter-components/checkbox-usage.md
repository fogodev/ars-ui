# Checkbox Usage Notes

Checkbox is a form control built from the framework-agnostic Checkbox machine.
`ars-leptos` and `ars-dioxus` expose unstyled primitive parts. Ready-made
styled source templates live in `ars-leptos-components` and
`ars-dioxus-components`. Both layers can be used standalone, inside `Form`, and
inside `Fieldset`.

The long-term distribution model is source ownership, not a closed styled
component package. The styled crates are checked-in reference implementations
for widgets, tests, and future registry generation. The future `ars-ui` binary
will copy component source into the user's project, where the copied files
become application-owned and editable.

## Standalone

Use `ars_leptos_components::input::checkbox::css::Checkbox`,
`ars_leptos_components::input::checkbox::tailwind::Checkbox`,
`ars_dioxus_components::input::checkbox::css::Checkbox`, or
`ars_dioxus_components::input::checkbox::tailwind::Checkbox` when the
application or widget wants the checked-in reference styled template. Use
`ars_*::input::checkbox::{Root, Label, Control, Indicator, HiddenInput,
Description, ErrorMessage}` when the application needs full anatomy control.
`checked` makes the control controlled; omitting it uses the machine's
uncontrolled state. `State::Indeterminate` is a controlled visual state and
must not toggle optimistically until the parent updates the value.

The adapter renders the core anatomy: `Root`, `Label`, `Control`, `Indicator`,
`HiddenInput`, optional `Description`, and optional `ErrorMessage`.

## Primitive Parts

Use the compound API when the application needs to style individual anatomy
parts, especially in Tailwind examples where styling should stay in class
tokens instead of raw CSS strings.

The primitive parts share the same machine contract as the styled `Checkbox`
component:

- `checkbox::Root` owns state, form props, Fieldset/Form context merging, and
  callbacks.
- `checkbox::Label` renders the accessible label attrs from core.
- `checkbox::Control` renders the focusable ARIA checkbox attrs and dispatches
  pointer/keyboard events.
- `checkbox::Indicator` renders the visual indicator attrs.
- `checkbox::HiddenInput` renders native form participation.
- `checkbox::Description` and `checkbox::ErrorMessage` register their presence
  so `aria-describedby` and `aria-errormessage` remain accurate in the browser.
- `checkbox::Root` also exposes `has_description` and `has_error_message` for
  SSR-stable first render when those parts are known to be present.

Dioxus parts accept normal global attributes through `GlobalAttributes`.
Leptos parts expose reactive `class` and `style` props with `TextProp`.
Semantic attrs, ids, ARIA, and `data-ars-*` state still come from the
framework-agnostic Checkbox API; examples should not recreate that logic.

Tailwind demos should import the Tailwind styled Checkbox from
`ars-*-components` unless the demo is specifically demonstrating primitive
composition. Do not add root-level `control_class` / `indicator_style` style
prop families for new parts; expose a public part component or a styled wrapper
instead.

## Copied Source Distribution

When the `ars-ui` binary exists, its Checkbox install command should copy normal
source files into the consuming application instead of hiding customization in a
crate boundary. A CSS install should produce a Rust component plus a stylesheet,
for example:

```text
src/components/ars/checkbox.rs
src/components/ars/checkbox.css
```

A Tailwind install should produce a Rust component whose class tokens are
statically discoverable by Tailwind. The generated code should compose
`ars-leptos` or `ars-dioxus` primitives, keep framework-agnostic behavior in
`ars-components`, and let application authors edit layout, spacing, color, and
part markup directly in their own repository.

Do not add adapter or styled-crate APIs solely to solve customization that is
better handled by editing copied component source. Add framework primitives
when the missing capability is semantic anatomy; add agnostic core helpers when
the missing capability is behavior, ARIA, validation, form state, keyboard
handling, or state derivation.

## Inside Form

Use `name` and `value` when a Checkbox should participate in form submission.
Only `State::Checked` submits the hidden checkbox value. Unchecked and
indeterminate states do not submit the value.

When rendered inside `Form`, Checkbox merges matching validation errors by
`name` into its invalid state. The visible error message remains
consumer-provided through `error_message`; the validation error source controls
state, not English prose owned by the adapter. Unmatched form errors must not
make unrelated checkboxes invalid.

Form reset must restore the Checkbox to its default state and clear demo/status
state through the public `Form` reset path. Public demos should use ars-ui
`Button` submit/reset controls when demonstrating ars-ui integration, and
should avoid browser-native validation bubbles unless the reference matrix
records that as intentional.

## Inside Fieldset

When rendered inside `Fieldset`, Checkbox inherits `disabled`, `readonly`, and
`invalid` state through the shared adapter field-support helper. Explicit
Checkbox props and inherited context merge with OR semantics for these boolean
state flags: an inherited disabled/readonly/invalid state cannot be cleared by
the child.

Disabled and readonly both block mutation. Readonly is an ars-ui extension for
Checkbox: it stays focusable but does not toggle.

## Required Documentation And Tests

Every Checkbox adapter or styled-template change must update usage docs when it
changes standalone, Form, or Fieldset behavior. Tests must cover standalone
anatomy, controlled and uncontrolled state, indeterminate, required,
invalid/error relationships, Form submit/reset, Fieldset inheritance, matching
validation errors by `name`, unmatched validation errors ignored, and the CSS
and Tailwind styled templates.
