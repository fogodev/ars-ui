# Layout Category Audit

This audit covers every agnostic layout component in epic #226:
`AspectRatio`, `Center`, `Frame`, `Grid`, `Stack`, `Carousel`,
`Collapsible`, `Portal`, `ScrollArea`, `Splitter`, and `Toolbar`.
Adapter implementations remain out of scope for this audit.

The audit compared the component specs in `spec/components/layout/`, the
layout category contract in `spec/components/layout/_category.md`, the shared
layout contract in `spec/shared/layout-shared-types.md`, the agnostic
implementations under `crates/ars-components/src/layout/`, and the layout test
surface under `crates/ars-components/tests/`.

## Component Matrix

| Component   | Implementation status                                                                                                                                 | Spec drift disposition                                                                   | Spec conformance                                          | Proptest status                                                 | Focused verification                                                                         |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| AspectRatio | Stateless agnostic core present with props, API, anatomy attrs, and snapshots.                                                                        | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/aspect_ratio.rs`. | Not applicable; no state machine.                               | Covered by `layout::aspect_ratio` unit/snapshot tests.                                       |
| Center      | Stateless agnostic core present with alignment props, API, anatomy attrs, and snapshots.                                                              | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/center.rs`.       | Not applicable; no state machine.                               | Covered by `layout::center` unit/snapshot tests.                                             |
| Frame       | Stateless agnostic core present with frame semantics, API, anatomy attrs, and snapshots.                                                              | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/frame.rs`.        | Not applicable; no state machine.                               | Covered by `layout::frame` unit/snapshot tests.                                              |
| Grid        | Stateless agnostic core present with grid props, API, anatomy attrs, and snapshots.                                                                   | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/grid.rs`.         | Not applicable; no state machine.                               | Covered by `layout::grid` unit/snapshot tests.                                               |
| Stack       | Stateless agnostic core present with stack props, API, anatomy attrs, and snapshots.                                                                  | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/stack.rs`.        | Not applicable; no state machine.                               | Covered by `layout::stack` unit/snapshot tests.                                              |
| Carousel    | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, keyboard, and RTL behavior.                     | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/carousel.rs`.     | Owned by `tests/proptest_state_machines/layout/carousel.rs`.    | Covered by `layout::carousel` unit/snapshot tests and the carousel proptest invariant.       |
| Collapsible | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, and controlled sync behavior.                   | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/collapsible.rs`.  | Owned by `tests/proptest_state_machines/layout/collapsible.rs`. | Covered by `layout::collapsible` unit/snapshot tests and the collapsible proptest invariant. |
| Portal      | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, and mount policy behavior.                      | Missing layout spec-conformance ownership was found and fixed by adding a Portal module. | Owned by `tests/spec_conformance/layout/portal.rs`.       | Owned by `tests/proptest_state_machines/layout/portal.rs`.      | Covered by `layout::portal` unit/snapshot tests and the portal proptest invariant.           |
| ScrollArea  | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, scrollbar behavior, and RTL behavior.           | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/scroll_area.rs`.  | Owned by `tests/proptest_state_machines/layout/scroll_area.rs`. | Covered by `layout::scroll_area` unit/snapshot tests and the scroll-area proptest invariant. |
| Splitter    | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, resizing, collapse, keyboard, and RTL behavior. | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/splitter.rs`.     | Owned by `tests/proptest_state_machines/layout/splitter.rs`.    | Covered by `layout::splitter` unit/snapshot tests and the splitter proptest invariant.       |
| Toolbar     | Stateful agnostic core present with props, state, events, context, effects, anatomy attrs, snapshots, roving focus, keyboard, and RTL behavior.       | No component-level drift found.                                                          | Owned by `tests/spec_conformance/layout/toolbar.rs`.      | Owned by `tests/proptest_state_machines/layout/toolbar.rs`.     | Covered by `layout::toolbar` unit/snapshot tests and the toolbar proptest invariant.         |

## Findings Landed

| Finding                                                                                                                                                                                                                       | Disposition                                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The layout spec-conformance tests were aggregated in one file, making component ownership unclear and omitting Portal.                                                                                                        | Split into `tests/spec_conformance/layout/mod.rs` plus one module per component, and added Portal anatomy coverage.                                                      |
| The layout proptests were aggregated in one file.                                                                                                                                                                             | Split into `tests/proptest_state_machines/layout/mod.rs` plus modules for the six stateful components: Carousel, Collapsible, Portal, ScrollArea, Splitter, and Toolbar. |
| `spec/components/layout/_category.md` Appendix D pointed per-component feature flags at `crates/ars-core/Cargo.toml`, while the shared layout spec and actual workspace feature layering place those flags in adapter crates. | Updated the category spec to point the feature map at `crates/ars-leptos` / `crates/ars-dioxus`, with a note that `ars-core` keeps only core features.                   |

## Test Layout

The layout spec-conformance tree now has one owner module per epic component:

- `crates/ars-components/tests/spec_conformance/layout/mod.rs`
- `crates/ars-components/tests/spec_conformance/layout/aspect_ratio.rs`
- `crates/ars-components/tests/spec_conformance/layout/carousel.rs`
- `crates/ars-components/tests/spec_conformance/layout/center.rs`
- `crates/ars-components/tests/spec_conformance/layout/collapsible.rs`
- `crates/ars-components/tests/spec_conformance/layout/frame.rs`
- `crates/ars-components/tests/spec_conformance/layout/grid.rs`
- `crates/ars-components/tests/spec_conformance/layout/portal.rs`
- `crates/ars-components/tests/spec_conformance/layout/scroll_area.rs`
- `crates/ars-components/tests/spec_conformance/layout/splitter.rs`
- `crates/ars-components/tests/spec_conformance/layout/stack.rs`
- `crates/ars-components/tests/spec_conformance/layout/toolbar.rs`

The layout state-machine proptest tree now has one owner module per stateful
layout component:

- `crates/ars-components/tests/proptest_state_machines/layout/mod.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/carousel.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/collapsible.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/portal.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/scroll_area.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/splitter.rs`
- `crates/ars-components/tests/proptest_state_machines/layout/toolbar.rs`

The five stateless layout components do not have state-machine proptests because
their meaningful contract is public API, anatomy, attribute, and snapshot
shape, all of which is covered by spec-conformance plus unit/snapshot tests.

## Closure Evidence

The focused audit gates for this epic are:

- `cargo xtask spec validate`
- `cargo test -p ars-components --test spec_conformance layout --features i18n`
- `PROPTEST_CASES=256 cargo test -p ars-components --test proptest_state_machines layout --features i18n -- --ignored`
- `cargo test -p ars-components --lib layout:: --features i18n`
- `cargo llvm-cov test -p ars-components --features i18n --lib --no-fail-fast --text -- layout::`

`cargo insta test --unreferenced=reject -p ars-components --features i18n --lib`
is required only when snapshot fixtures change; this audit does not modify
`.snap` files.

## Coverage Audit Result

`cargo llvm-cov test -p ars-components --features i18n --lib --no-fail-fast --text -- layout::`
completed successfully. The relevant layout module line coverage from the
generated report is:

| Module                | Line coverage |
| --------------------- | ------------: |
| `layout/aspect_ratio` |       100.00% |
| `layout/carousel`     |        97.53% |
| `layout/center`       |        95.76% |
| `layout/collapsible`  |        95.07% |
| `layout/frame`        |       100.00% |
| `layout/grid`         |        96.85% |
| `layout/portal`       |        96.57% |
| `layout/scroll_area`  |        98.30% |
| `layout/splitter`     |        96.87% |
| `layout/stack`        |        95.15% |
| `layout/toolbar`      |        95.44% |

The remaining misses are concentrated in defensive branches and alternate
public helper paths that are already covered by unit/snapshot breadth and the
stateful proptest invariants where applicable. No additional layout test gap was
identified by the post-implementation coverage audit.
