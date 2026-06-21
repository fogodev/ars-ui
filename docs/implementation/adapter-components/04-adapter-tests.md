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
- consumer class/global-attribute forwarding on the root and on every public
  stylable part;
- fallback structural part behavior, including the guarantee that an explicit
  public part suppresses the adapter fallback when only one semantic node may
  exist;
- typed collection renderer overrides, when present, proving render callbacks
  receive typed item context from the root collection and consumers do not
  manually repeat or stringify item keys;
- private behavior-critical helper boundaries, when a planned or visible
  anatomy node intentionally remains private, proving stale public symbols do
  not remain exported or documented;
- empty state and loading sentinel output where supported.

For each public stylable part, include at least one focused SSR/unit assertion
that consumer styling reaches the rendered node while core attrs remain intact:

- Leptos parts should prove `class` and `style` merge with the agnostic part
  attrs;
- Dioxus parts should prove `GlobalAttributes` forwarding with at least
  `class:` and `style:`;
- required fallback parts should prove both the fallback path and the explicit
  part path render exactly one required semantic node.

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

For draggable or clickable compound components, browser parity evidence should
include computed cursor checks when the cursor communicates behavior. Verify
the idle cursor on the public shell and visible interactive children, then
verify the active drag state after the component exposes its dragging marker.
Screenshots alone are not enough for this class of regression because browser
cursor changes are not captured reliably in static images.

For form controls, adapter tests must include composition scenarios with
`Form` and `Fieldset`. SSR/unit tests should assert merged disabled, readonly,
invalid, validation-error, description, and error IDREF output. Wasm tests
should assert submit/reset behavior, DOM checked/value properties, and state
updates after validation changes.

For collection-backed components that render child components per item, add
browser-backed regressions for semantic identity whenever reorder, insert, or
remove is supported. Dioxus tests should prove component-local state, mounted
refs, lazy panel state, or focus restoration follows the item key after a
reorder; a key on an inner DOM node is not enough evidence. Leptos tests should
prove keyed `For` output preserves the intended item identity across the same
operation.

Prefer `ars-test-harness-leptos`, `ars-test-harness-dioxus`, and shared
`ars-test-harness` helpers instead of bespoke browser scaffolding.

## Snapshots

Add snapshot tests only when a stable rendered structure is part of the review
surface. Inspect snapshots before marking work ready. If `.snap` files change,
the PR must receive the `snapshot-reviewed` label after opening or updating.
