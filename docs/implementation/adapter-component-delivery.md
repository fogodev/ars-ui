# Adapter Component Delivery Workflow

This document defines the end-to-end workflow for implementing adapter-level
components. It exists so agents can deliver a component without rediscovering
the surrounding obligations every time.

Use this workflow for any issue that adds or materially changes a component in
`crates/ars-leptos` or `crates/ars-dioxus`.

## Source Of Truth

Start from the assigned GitHub issue, not from an epic. The issue acceptance
criteria define the delivery scope.

Before editing code, read:

- the framework-agnostic component spec under `spec/components/<category>/`;
- the matching adapter spec under `spec/leptos-components/<category>/` or
  `spec/dioxus-components/<category>/`;
- the relevant adapter foundation spec:
  `spec/foundation/08-adapter-leptos.md` or
  `spec/foundation/09-adapter-dioxus.md`;
- `docs/implementation/adapter-contract.md`;
- `examples/widgets-ownership.md`.

When touching Leptos or Dioxus code or specs, load the repo skill for that
framework before relying on framework APIs.

## Required Deliverables

An adapter component task is not complete when only the adapter component
compiles. It must update every surface that consumers, tests, and reviewers use.

For each implemented component, include the applicable items below in the same
PR.

### Adapter Crate

- Add or update the component implementation in
  `crates/ars-leptos/src/<category>/<component>.rs` or
  `crates/ars-dioxus/src/<category>/<component>.rs`.
- Add the module to the category `mod.rs`.
- Re-export the component through `crates/ars-leptos/src/prelude.rs` or
  `crates/ars-dioxus/src/prelude.rs` when the component is user-facing.
- Keep both adapter preludes symmetric when both adapters expose the component.
- Update adapter `Cargo.toml` feature wiring if the component needs an
  `ars-components` feature to be reachable.
- Preserve the adapter layering: adapter crates may render framework views and
  browser integration, but component behavior belongs in `ars-components`.

### Adapter Tests

- Add or update framework-specific adapter tests under:
  - `crates/ars-leptos/tests/`
  - `crates/ars-dioxus/tests/`
- Follow the file naming and parity expectations enforced by
  `cargo xtask lint adapter-parity`.
- Cover the component's required props, attributes, ARIA output, state
  synchronization, callbacks, disabled/read-only states, and as-child behavior
  where applicable.
- Add browser-backed wasm tests for behavior that requires a real DOM: focus,
  keyboard navigation, pointer events, layout, clipboard, file upload,
  drag/drop, media queries, observers, portal behavior, or cleanup of browser
  resources.
- Add snapshot tests only when a stable rendered structure is part of the review
  surface. Inspect snapshots before marking the work ready; PRs that touch
  `.snap` files must receive the `snapshot-reviewed` label.

### E2E Fixtures And Harnesses

Add the component to the browser E2E fixture when it has browser-observable
behavior or when reviewers need a cross-adapter browser proof.

Update both fixtures when both adapters exist:

- `crates/ars-e2e/fixtures/leptos/src/main.rs`
- `crates/ars-e2e/fixtures/dioxus/src/main.rs`

Then add or update the matching E2E harness module:

- `crates/ars-e2e/src/<category>/<component>.rs`
- `crates/ars-e2e/src/<category>/mod.rs`
- `crates/ars-e2e/src/lib.rs` only if this is the first component in a new
  category.
- `xtask/src/e2e.rs`, `xtask/src/main.rs`, and `crates/ars-e2e/src/main.rs`
  when this is the first E2E-covered component in a category that does not yet
  have an E2E subcommand.

E2E harnesses should verify what unit or SSR tests cannot prove reliably:

- axe accessibility checks on the mounted fixture;
- keyboard and pointer interaction in a real browser;
- focus movement and visible focus indicators;
- ARIA roles, names, and live text visible to the browser;
- portal, overlay, positioning, scroll lock, or z-index behavior;
- browser-native form, disabled, file, clipboard, or drag/drop semantics;
- adapter parity for the same visible scenario across Leptos and Dioxus.

If a component is purely static and already has strong adapter tests, state why
no E2E fixture change is needed in the PR.

### Widgets Examples

Add a visual/demo entry to every applicable widgets example crate:

- `examples/widgets-leptos`
- `examples/widgets-dioxus`
- `examples/widgets-leptos-css`
- `examples/widgets-dioxus-css`
- `examples/widgets-leptos-tailwind`
- `examples/widgets-dioxus-tailwind`

Use the category module that matches the component:

- `src/categories/input.rs`
- `src/categories/selection.rs`
- `src/categories/overlay.rs`
- `src/categories/navigation.rs`
- `src/categories/date_time.rs`
- `src/categories/data_display.rs`
- `src/categories/layout.rs`
- `src/categories/specialized.rs`
- `src/categories/utility.rs`

Do not put category-specific example text in root `WidgetsText`. Add it to the
category-local text enum such as `UtilityText`, `NavigationText`, or
`InputText`.

Only edit `main.rs`, `text.rs`, or `categories/mod.rs` when the top-level spec
category list itself changes.

### Specification Updates

Update the relevant spec in the same PR whenever implementation reveals drift
or incomplete spec coverage. Follow
[README.md § Spec synchronization](../README.md#spec-synchronization).

- Shared behavior belongs in `spec/foundation/` or `spec/shared/`.
- Dependency-machine changes belong in that machine's component spec (for
  example new `popover::Api` accessors used by a composition layer).
- Adapter-specific behavior belongs in
  `spec/foundation/08-adapter-leptos.md`,
  `spec/foundation/09-adapter-dioxus.md`, or the per-component adapter spec.
- Do not leave spec drift as a follow-up — port back missing helpers, incomplete
  code sketches, anatomy/ARIA table gaps, and default-source clarity in the
  same PR.
- Run `cargo xtask spec validate` after spec edits.

## Implementation Order

Use this order unless the issue explicitly says otherwise:

1. Move the issue to **In Progress** on the GitHub Project board.
2. Read the issue acceptance criteria and the specs listed above.
3. Add or update focused adapter tests first.
4. Implement the adapter component code.
5. Wire modules, prelude exports, and feature flags.
6. Add or update E2E fixtures and E2E harness modules.
7. Add or update widgets examples in the matching category modules.
8. Update specs when the intended contract changed or when implementation
   surfaced incomplete spec coverage (see
   [README.md § Spec synchronization](../README.md#spec-synchronization)).
9. Run focused tests and checks for the edited component.
10. Invoke `.agents/skills/post-implementation-audit/SKILL.md`.
11. Fix every audit finding in the same PR.
12. Present the result for user review before committing.

## Validation Checklist

Run the exact commands named by the issue. In addition, use the applicable
checks below.

For Leptos adapter code:

```bash
cargo check -p ars-leptos
cargo test -p ars-leptos --test <component>
```

For Dioxus adapter code:

```bash
cargo check -p ars-dioxus
cargo test -p ars-dioxus --test <component>
```

For browser-backed adapter tests, use the repo browser-test environment
documented in `AGENTS.md` when running wasm tests locally.

For E2E changes:

```bash
cargo check -p ars-e2e
cargo xtask e2e <category> --adapter leptos
cargo xtask e2e <category> --adapter dioxus
```

Today the supported E2E categories are `navigation` and `utility`. If a task
adds the first E2E-covered component for another category, add the matching
category subcommand before documenting the validation command as available.

For widgets examples:

```bash
cd examples
cargo check -p widgets-leptos -p widgets-dioxus \
  -p widgets-leptos-css -p widgets-dioxus-css \
  -p widgets-leptos-tailwind -p widgets-dioxus-tailwind
```

For workspace gates before publishing:

```bash
cargo fmt --all --check
cargo xtask lint adapter-parity
cargo xci-fast
```

Run full `cargo xci` instead of `cargo xci-fast` when the change is broad,
touches feature-flag interactions, or modifies shared adapter infrastructure.

## PR Closeout

Before asking for review:

- confirm all required deliverables above are either implemented or explicitly
  marked N/A with a reason;
- include the issue auto-close keyword in the PR body;
- list spec refs and validation commands in the PR body;
- attach `snapshot-reviewed` if `.snap` files changed;

After the first push (and after every subsequent push to the PR branch), **read
and follow** `.agents/skills/waiting-for-codex-review/SKILL.md` through to Codex
👍. Posting `@codex review` alone does not satisfy this step — the full poll
loop, thread triage, fix/push/reply/resolve cycle, and re-trigger are all
required. Do not treat the PR as merge-ready until the skill completes.

- keep the issue, PR, and Project board state aligned with the actual work.

Do not close the issue until the PR is merged, CI is green, and Codex review has
left a thumbs-up.
