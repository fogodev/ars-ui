# Adapter Tests

Adapter tests prove that the framework surface faithfully renders and reacts to
the agnostic component contract.

## Required Locations

Add or update framework-specific tests under:

- `crates/ars-leptos/tests/<component>.rs`
- `crates/ars-dioxus/tests/<component>.rs`
- `crates/ars-leptos/tests/<component>_wasm.rs`
- `crates/ars-dioxus/tests/<component>_wasm.rs`

Follow file naming and parity expectations enforced by:

```bash
cargo xtask lint adapter-parity
```

## Unit And SSR Coverage

Cover:

- required props and defaults;
- rendered roles and ARIA attributes;
- `data-ars-*` anatomy and state attrs;
- ids and relationship wiring;
- disabled, readonly, invalid, selected, focused, active, hidden, and loading
  states where supported;
- controlled and uncontrolled state synchronization;
- callbacks and action dispatch;
- native links, forms, and hidden inputs where supported;
- composition with every foundation context the component consumes;
- consumer class/global-attribute forwarding;
- empty state and loading sentinel output where supported.

## Wasm Browser Coverage

Wasm tests are focused adapter/browser integration tests. They should prove the
framework wrapper actually works in a browser runtime for the DOM wiring that
SSR cannot prove cheaply. They are not the E2E parity harness, and passing wasm
tests is not enough to claim counterpart outcome parity.

Add browser-backed wasm tests for adapter behavior that requires a real DOM:

- event-handler wiring, such as input, change, submit, reset, press, and
  composition callbacks;
- browser-rendered `aria-*`, `data-*`, native boolean, id, and relationship
  attributes after mount;
- reactive updates that must reach the DOM after signals, stores, props, or
  machine state change;
- generated-id stability and label/description/error/live-region
  relationships;
- focus and focus restoration;
- keyboard navigation;
- pointer events;
- typeahead;
- drag/drop;
- clipboard and file upload;
- observers;
- portal behavior;
- layout-sensitive behavior;
- mount/unmount cleanup.

Keep wasm tests small and targeted. A good wasm test usually covers one or two
adapter-specific browser guarantees that would otherwise be invisible to SSR:
for example, `Input.on_value_input` receives the typed value, `Form.on_submit`
prevents navigation before calling the callback, or an invalid `Field` input
renders `aria-errormessage` after browser mount.

Do not turn wasm tests into a duplicate of the E2E suite. If the assertion is
about full user workflows, styled example parity, computed visual feedback,
cross-adapter UX comparison, axe across reached states, or reference-outcome
coverage, put it in E2E/widget/browser parity evidence instead.

For form controls, adapter tests must include composition scenarios with
`Form` and `Fieldset`. SSR/unit tests should assert merged disabled, readonly,
invalid, validation-error, description, and error IDREF output. Wasm tests
should assert submit/reset behavior, DOM checked/value properties, and state
updates after validation changes.

Prefer `ars-test-harness-leptos`, `ars-test-harness-dioxus`, and shared
`ars-test-harness` helpers instead of bespoke browser scaffolding.

## Snapshots

Add snapshot tests only when a stable rendered structure is part of the review
surface. Inspect snapshots before marking work ready. If `.snap` files change,
the PR must receive the `snapshot-reviewed` label after opening or updating.
