# Initial Backlog

This backlog is the seed set for the implementation program. It intentionally stops at the first shippable utility slice instead of attempting the whole library at once.

> **Backlog reset note (2026-04-03):** The seed backlog reached the crate-shell milestone, but the remaining open planning card `#24` is premature. Before any utility-slice decomposition resumes, use [foundation-gap-audit.md](./foundation-gap-audit.md) as the canonical reference for the missing middle-layer contracts and the replacement foundation-first task sequence.

## Epics

### Epic: Workspace bootstrap

- Point target: `8`
- Layer: `Core`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/01-architecture.md#1-crate-structure`, `spec/testing/14-ci.md`

### Epic: Core runtime

- Point target: `8`
- Layer: `Core`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/01-architecture.md#2-state-machine-core-ars-core`

### Epic: A11y

- Point target: `25` (revised from `5` after full spec audit — 2026-04-10)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/03-accessibility.md`

### Epic: Interactions

- Point target: `30` (revised from `8` after full spec audit — 2026-04-10)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/05-interactions.md`

### Epic: Forms

- Point target: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/07-forms.md`

### Epic: DOM utilities

- Point target: `5`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/01-architecture.md#25-scrolllockmanager-ars-dom`, `spec/foundation/11-dom-utilities.md`

### Epic: Test infrastructure

- Point target: `8`
- Layer: `Harness`
- Framework: `Both`
- Test tier: `Mixed`
- Spec refs: `spec/testing/00-overview.md`, `spec/testing/15-test-harness.md`

### Epic: Leptos adapter

- Point target: `8`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/foundation/08-adapter-leptos.md`

### Epic: Dioxus adapter

- Point target: `8`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/foundation/09-adapter-dioxus.md`

### Epic: First utility slice

- Point target: `8`
- Layer: `Component`
- Framework: `Both`
- Test tier: `Mixed`
- Spec refs: `spec/components/utility/_category.md`

### Epic: Spec synchronization

- Point target: `3`
- Layer: `Spec`
- Framework: `Both`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/01-architecture.md`, `spec/foundation/08-adapter-leptos.md`, `spec/foundation/09-adapter-dioxus.md`

## Agent-ready tasks

### #12: Replace workspace exclusion with architecture-defined crate membership

- Points: `2`
- Layer: `Core`
- Framework: `None`
- Test tier: `Mixed`
- Depends on: none
- Spec refs: `spec/foundation/01-architecture.md#11-workspace-layout`
- Tests first: `cargo check --workspace`, `cargo test --workspace`
- Acceptance: workspace lists all baseline crates and builds cleanly
- Spec impact: `No spec change required`

### #13: Add initial `ars-core` contract shell with service-level tests

- Points: `3`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#12`
- Spec refs: `spec/foundation/01-architecture.md#21-the-machine-trait`, `spec/testing/01-unit-tests.md`, `spec/testing/02-integration-tests.md`
- Tests first: initial unit tests for state transitions and `Bindable`
- Acceptance: `ars-core` exposes the baseline contract and unit tests pass
- Spec impact: `No spec change required`

### #14: Add initial `ars-derive` shell for `HasId` and `ComponentPart`

- Points: `2`
- Layer: `Core`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#12`
- Spec refs: `spec/foundation/01-architecture.md#4-anatomy-system`
- Tests first: compile coverage for derive entry points
- Acceptance: proc-macro crate exists and downstream crates can depend on it
- Spec impact: `No spec change required`

### #15: Add baseline `ars-a11y` types for roles, attributes, and component IDs

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#13`
- Spec refs: `spec/foundation/03-accessibility.md#2-aria-attribute-system`
- Tests first: unit tests for canonical type construction and ID helpers
- Acceptance: adapter/component crates can depend on shared a11y types
- Spec impact: `No spec change required`

Note: this seed task only covered baseline a11y types. The follow-up `AriaAttribute` bridge onto typed `HtmlAttr`/`AttrMap` and the shared role/state helper layer landed as [#33](https://github.com/fogodev/ars-ui/issues/33) and [#34](https://github.com/fogodev/ars-ui/issues/34). A full spec audit (2026-04-10) revealed the original 4-task decomposition covered only ~30% of the 4000-line spec. Eight additional tasks (#150–#157, 22 pts) now cover FocusZone, keyboard shortcuts, VisuallyHidden, FieldContext, announcements, touch/mobile, and testing infrastructure. See [Epic #3](https://github.com/fogodev/ars-ui/issues/3) for the full decomposition.

### #16: Add baseline `ars-interactions` merge and state primitives

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#13`
- Spec refs: `spec/foundation/05-interactions.md#81-the-composition-problem`, `spec/foundation/05-interactions.md#82-merge_attrs`
- Tests first: unit tests for attribute merge semantics
- Acceptance: shared merge helper and initial interaction states exist
- Spec impact: `No spec change required`

Note: this seed task only covered attribute composition. Follow-on tasks delivered Press (#58), Hover (#59), Focus (#60), InteractOutside (#65), Dismissable (#104), and shared modality (#57/#90). A full spec audit (2026-04-10) found the original decomposition covered ~60% of the 4000-line spec. #78 (Drag and Drop, 8pts) was decomposed into #159, #160, #161 (8pts total), and a gap task #162 (Keyboard interaction types, 2pts) was created for spec §11 which had no coverage. See [Epic #4](https://github.com/fogodev/ars-ui/issues/4) for the full decomposition.

### #17: Add baseline `ars-forms` validation and field-state types

- Points: `3`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#13`, `#15`
- Spec refs: `spec/foundation/07-forms.md#2-core-types`, `spec/foundation/07-forms.md#3-the-validator-trait`
- Tests first: unit tests for validation result behavior
- Acceptance: form contracts exist independently of adapters
- Spec impact: `No spec change required`

### #18: Add baseline `ars-dom` platform utility shell

- Points: `2`
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Unit`
- Depends on: `#16`, `#17`
- Spec refs: `spec/foundation/01-architecture.md#25-scrolllockmanager-ars-dom`
- Tests first: compile checks for feature layout
- Acceptance: DOM crate exists with web/ssr feature structure
- Spec impact: `No spec change required`

### #19: Add initial harness crates and locale-aware test harness shell

- Points: `3`
- Layer: `Harness`
- Framework: `Both`
- Test tier: `Adapter`
- Depends on: `#13`, `#18`
- Spec refs: `spec/testing/15-test-harness.md#2-core-harness`
- Tests first: unit tests for harness value objects and compile checks for adapter backends
- Acceptance: shared harness and per-framework backend crates compile
- Spec impact: `No spec change required`

### #20: Add CI workflow split by unit, integration, and adapter tiers

- Points: `5`
- Layer: `Harness`
- Framework: `Both`
- Test tier: `Mixed`
- Depends on: `#12`, `#19`
- Spec refs: `spec/testing/14-ci.md`, `spec/testing/00-overview.md#11-testing-tiers`
- Tests first: workflow validation via local dry-run or syntax checks
- Acceptance: CI clearly separates tiers and targets workspace crates
- Spec impact: `No spec change required`

### #21: Define shared adapter contract notes and sync checklist in repo workflow

- Points: `2`
- Layer: `Spec`
- Framework: `Both`
- Test tier: `Mixed`
- Depends on: `#12`
- Spec refs: `spec/foundation/08-adapter-leptos.md`, `spec/foundation/09-adapter-dioxus.md`
- Tests first: not applicable; verify workflow docs/templates enforce spec sync
- Acceptance: issue/PR templates require spec impact and adapter sync discussion
- Spec impact: `No spec change required`

### #22: Add initial Leptos adapter shell crate wired to shared contracts

- Points: `3`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Depends on: `#13`, `#15`, `#16`, `#18`, `#19`
- Spec refs: `spec/foundation/08-adapter-leptos.md#1-overview`, `spec/foundation/08-adapter-leptos.md#3-the-use_machine-hook`
- Tests first: compile checks and adapter crate smoke tests
- Acceptance: Leptos adapter crate exists with initial feature structure
- Spec impact: `No spec change required`

### #23: Add initial Dioxus adapter shell crate wired to shared contracts

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Depends on: `#13`, `#15`, `#16`, `#18`, `#19`
- Spec refs: `spec/foundation/09-adapter-dioxus.md`
- Tests first: compile checks and adapter crate smoke tests
- Acceptance: Dioxus adapter crate exists with initial feature structure
- Spec impact: `No spec change required`

### #24: Break the first utility slice into per-primitive delivery cards

- Points: `2`
- Layer: `Component`
- Framework: `Both`
- Test tier: `Mixed`
- Depends on: `#21`
- Spec refs: `spec/components/utility/_category.md`, `spec/components/utility/button.md`, `spec/components/utility/focus-scope.md`, `spec/components/utility/form.md`
- Tests first: not applicable; verify each card includes tests-first details
- Acceptance: the first slice is decomposed into component-level or behavior-level cards no larger than `5`
- Spec impact: `No spec change required`

## Backlog Reset

The original seed tasks correctly established workspace, core, subsystem, harness, and adapter shells. They did **not** fully surface the shared contracts required by the first utility slice. The missing follow-on work is now tracked in [foundation-gap-audit.md](./foundation-gap-audit.md), which:

- explains why `#24` is blocked,
- identifies the missing shared contracts in `ars-core`, `ars-derive`, `ars-dom`, `ars-forms`, provider/platform wiring, and interaction composition,
- defines the corrected issue-ready replacement task sequence, and
- names the next unblocked high-leverage task: typed connect primitives in `ars-core`.
