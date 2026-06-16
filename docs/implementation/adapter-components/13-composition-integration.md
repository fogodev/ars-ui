# Composition Integration

Adapter components must prove their behavior inside the provider and foundation
contexts they consume. Isolated anatomy tests are not enough when a component
reads `Form`, `Field`, `Fieldset`, collection, group, overlay, or provider
state.

## Mandatory Cases

For any component that consumes a context, add adapter integration tests for the
context boundary and the component's explicit props. The tests must prove the
merged outcome, not only that each component renders alone.

Form controls must cover:

- rendering inside `Form` and submitting the expected `name` / `value`;
- reset returning the control to its default state;
- matching `Form` validation errors by `name`;
- unmatched validation errors not affecting the control;
- rendering inside `Fieldset` and inheriting `disabled`, `readonly`, and
  `invalid` state;
- explicit control props merging with inherited state according to the shared
  adapter helper rules.

If a component also composes with `Field`, add tests for label, description,
error, and IDREF ownership so the control does not create duplicate or dangling
relationships.

## Test Layers

Use SSR/unit tests for static merged attrs and relationship output. Use wasm
browser tests for submit/reset events, DOM property updates, focus behavior, and
state changes after validation updates. Use E2E for the public workflow with
styled visible feedback, form status separation, axe after reached states, and
Leptos/Dioxus parity.

Widgets and fixtures are consumers. They may own sample values and status copy,
but they must not rebuild validation policy, ARIA relationships, keyboard
behavior, or context merge rules that belong in the component contract.
