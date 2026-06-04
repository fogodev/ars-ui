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
- consumer class/global-attribute forwarding;
- empty state and loading sentinel output where supported.

## Wasm Browser Coverage

Add browser-backed wasm tests for behavior that requires a real DOM:

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

Prefer `ars-test-harness-leptos`, `ars-test-harness-dioxus`, and shared
`ars-test-harness` helpers instead of bespoke browser scaffolding.

## Snapshots

Add snapshot tests only when a stable rendered structure is part of the review
surface. Inspect snapshots before marking work ready. If `.snap` files change,
the PR must receive the `snapshot-reviewed` label after opening or updating.
