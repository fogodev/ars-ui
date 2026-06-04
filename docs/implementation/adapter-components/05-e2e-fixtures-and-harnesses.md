# E2E Fixtures And Harnesses

The fixture and harness layer must cover every browser-observable public
feature of every adapter-level component. Smoke-only coverage is a workflow
violation unless the PR body records a valid exception.

## Required Files

When both adapters exist, update both fixture category aggregators:

- `crates/ars-e2e/fixtures/leptos/src/categories/<category>/mod.rs`
- `crates/ars-e2e/fixtures/dioxus/src/categories/<category>/mod.rs`

Component-specific fixture code belongs in component modules:

- `crates/ars-e2e/fixtures/leptos/src/categories/<category>/<component>.rs`
- `crates/ars-e2e/fixtures/dioxus/src/categories/<category>/<component>.rs`

The category `mod.rs` should aggregate component panels and delegate message
registration. Do not append hundreds of component-specific lines to
`<category>/mod.rs`; when a new component lands, add a sibling component module.

Then add or update:

- `crates/ars-e2e/src/<category>/<component>.rs`
- `crates/ars-e2e/src/<category>/mod.rs`
- `crates/ars-e2e/src/lib.rs` if this is the first component in a new
  category;
- `xtask/src/e2e.rs`, `xtask/src/main.rs`, and
  `crates/ars-e2e/src/main.rs` when this is the first E2E-covered component in
  a category.

## Fixture Design

The fixture page must expose one id'd instance per feature combination the
harness drives. Duplication is intentional. Do not share state between fixture
instances.

Example instance set for a selection component:

- `#component-single-basic`
- `#component-multi-grouped`
- `#component-disabled-root`
- `#component-invalid-with-error`
- `#component-named-form`
- `#component-empty-state`
- `#component-loading`
- `#component-actions`

## Harness Design

Write one async test function per feature axis, named after what it asserts.
Failures should identify the broken feature, not "component harness failed".

Cover every prop, slot, callback, ARIA path, keyboard path, pointer
interaction, controlled/uncontrolled mode, and discrete state branch the
component exposes.

If the adapter spec lists a feature and no E2E test function drives it, the
coverage is incomplete.

## Axe Across Visible States

Run axe after every distinct visible state the component can reach, not only on
initial mount. Many accessibility regressions appear only after state changes.

Use:

- `run_axe(driver)` when the whole fixture page is expected to be complete and
  labelled;
- `run_axe_on(driver, "#scenario-id")` when the page co-hosts bare-minimum and
  fully-labelled scenarios.

Name axe-only tests `axe_clean_<state>`. When a behavioral test already reaches
the state, append the axe call and use a name ending in `_and_axe_clean`.

## Matrix Entries

Every implemented adapter component must be represented in
`crates/ars-e2e/src/matrix.rs`.

Account for every axis:

- pointer;
- keyboard;
- focus;
- state;
- forms;
- visual;
- accessibility;
- lifecycle.

If an axis is not meaningful for the component, record `NotApplicable` with a
reason. Do not leave axes implicit.

## Computed Visual Assertions

Any visible state must have browser assertions for rendered feedback, not only
ARIA or data attributes.

Prefer computed style and layout assertions:

- `background-color`, `font-weight`, `color`, `cursor`, `opacity`, `display`,
  and `visibility`;
- nonzero rendered boxes;
- stable dimensions after state changes;
- popup anchoring;
- selected, hovered, focused, disabled, invalid, loading, and drop-target
  feedback;
- clean browser console after representative interactions.

Avoid brittle screenshot baselines unless rendering is deterministic enough to
avoid churn.

## Focused Runs

Every category E2E command must support focused execution:

```bash
cargo xtask e2e <category> --adapter leptos --component <component>
cargo xtask e2e <category> --adapter dioxus --component <component>
cargo xtask e2e <category> --adapter dioxus --component <component> --test-filter <substring>
cargo xtask e2e <category> --adapter dioxus --component <component> --visual-only
cargo xtask e2e <category> --adapter dioxus --component <component> --behavior-only
```

The full category command must still work.

## Exceptions

Smoke-only coverage is acceptable only when:

- the component is purely static and adapter tests already cover every output
  branch; or
- the issue acceptance criteria explicitly limit the E2E surface and the PR
  body lists the unexercised axes with reasons.

The default is exhaustive browser coverage.
