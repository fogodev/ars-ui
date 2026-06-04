# E2E Fixtures And Harnesses

The fixture and harness layer must cover every browser-observable public
feature of every adapter-level component. Smoke-only coverage is a workflow
violation unless the PR body records a valid exception.

## Required Files

When both adapters exist, update both fixture category aggregators.

Existing categories may still be flat Rust modules:

- `crates/ars-e2e/fixtures/leptos/src/categories/<category>.rs`
- `crates/ars-e2e/fixtures/dioxus/src/categories/<category>.rs`

If a category is already directory-backed, or the PR migrates it to a
directory-backed module and updates `categories/mod.rs` to import that module,
component-specific fixture code belongs in sibling component modules:

- `crates/ars-e2e/fixtures/leptos/src/categories/<category>/<component>.rs`
- `crates/ars-e2e/fixtures/dioxus/src/categories/<category>/<component>.rs`

For large categories, prefer migrating to the directory-backed form in the same
PR instead of appending hundreds of component-specific lines to a flat
`<category>.rs` file. The category `mod.rs` should aggregate component panels
and delegate message registration.

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

When both Leptos and Dioxus adapters exist for a component, their E2E harnesses
must cover the same feature axes. If a feature assertion lands for one adapter,
add the matching assertion for the other adapter in the same PR, or document why
that axis is not applicable to the other adapter.

If the adapter spec lists a feature and no E2E test function drives it, the
coverage is incomplete.
If the counterpart outcome matrix marks an axis as supported and no E2E test
drives it, the parity claim is incomplete.

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

Every implemented adapter component must have explicit E2E axis accounting in
the component harness module or in an existing shared E2E matrix file. Do not
point tasks at a matrix path that does not exist in the current tree. If a PR
introduces a shared matrix module, it must wire that module into the E2E runner
in the same PR before listing it as a required destination.

Account for every axis:

- pointer;
- keyboard;
- focus;
- state;
- forms;
- validation;
- hover and press;
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
- indeterminate, readonly, required, pressed, focus-visible, and form
  validation feedback when supported;
- clean browser console after representative interactions.

Avoid brittle screenshot baselines unless rendering is deterministic enough to
avoid churn.

## Category Runs

Use the category commands that actually exist in the current tree. Check
`cargo xtask e2e --help` and `cargo xtask e2e <category> --help` before adding
commands to an issue plan or PR body.

```bash
cargo xtask e2e <category> --adapter leptos
cargo xtask e2e <category> --adapter dioxus
```

Current category commands accept the adapter, port, WebDriver, server, and
headed/headless options exposed by their `--help` output. Do not document or
require focused flags such as `--component`, `--test-filter`, `--visual-only`,
or `--behavior-only` unless the same PR implements those flags in `xtask` and
the standalone E2E harness.

When a PR adds the first E2E-covered component in a category, it must add the
category command before listing it as validation.

## Exceptions

Smoke-only coverage is acceptable only when:

- the component is purely static and adapter tests already cover every output
  branch; or
- the issue acceptance criteria explicitly limit the E2E surface and the PR
  body lists the unexercised axes with reasons.

The default is exhaustive browser coverage.
