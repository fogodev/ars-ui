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

- Point target: `34` (revised from `5` after full spec audit — 2026-04-10)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/07-forms.md`

### Epic: DOM utilities

- Point target: `42` (revised from `5` after full spec audit — 2026-04-10)
- Layer: `Subsystem`
- Framework: `None`
- Test tier: `Mixed`
- Spec refs: `spec/foundation/01-architecture.md#25-scrolllockmanager-ars-dom`, `spec/foundation/11-dom-utilities.md`

Note: the original 4-task decomposition (#18, #39, #40, #88) covered only the baseline shell and three immediate utilities. A full spec audit (2026-04-10) found `11-dom-utilities.md` is 2804 lines across 10 sections (§2 Positioning, §3 Focus, §4 Scroll, §5 Scroll Lock, §6 Z-Index, §7 Portal/Inert, §8 Modality, §9 Media Queries, §10 URL Sanitization), not the ~400 lines originally estimated. Twelve additional tasks now cover positioning engine (#66, #67, #112–#115), z-index/overlay (#68, #88), portal (#69), modality (#72), scroll (#74), media queries (#85), and URL sanitization (#176). See [Epic #6](https://github.com/fogodev/ars-ui/issues/6) for the full decomposition.

### Epic: Test infrastructure

- Point target: `42` (revised from `8` after full spec audit — 2026-04-10)
- Layer: `Harness`
- Framework: `Both`
- Test tier: `Mixed`
- Spec refs: `spec/testing/00-overview.md`, `spec/testing/15-test-harness.md`, `spec/testing/04-aria-helpers.md`, `spec/testing/03-snapshot-tests.md`, `spec/testing/05-adapter-harness.md`, `spec/testing/14-ci.md`

Note: the original 2-task decomposition (#19 harness shell, #20 CI tier split) established crate skeletons and CI tier separation but covered only ~5% of the spec-defined test harness API. A full spec audit (2026-04-10) found `15-test-harness.md` defines 30+ methods across 8 sections, `04-aria-helpers.md` defines 35+ assertion functions, and `14-ci.md` defines 3 enforcement scripts and a nightly pipeline — none of which were implemented. Eleven additional tasks (#178–#188, 34 pts) now cover the full TestHarness API, ARIA helpers, `insta` snapshot setup, Leptos/Dioxus backends, parity types, CI enforcement scripts, mock infrastructure, and nightly pipeline. See [Epic #7](https://github.com/fogodev/ars-ui/issues/7) for the full decomposition.

### Epic: Leptos adapter

- Point target: `283` (revised from `8` after full component adapter decomposition — 2026-04-10)
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/foundation/08-adapter-leptos.md`, `spec/leptos-components/`

Note: the original 3-task decomposition (#22, #55, #105) covered only the adapter shell and foundational hooks. Two additional foundation tasks (#190, #191, 8 pts) close the remaining foundational gaps. Nine category epics (#303–#311, 264 pts) now cover all 112 Leptos adapter components across 94 tasks organized by category. See [Epic #8](https://github.com/fogodev/ars-ui/issues/8) for the full sub-issue breakdown.

### Epic: Dioxus adapter

- Point target: `292` (revised from `24` after full component adapter decomposition — 2026-04-11)
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/foundation/09-adapter-dioxus.md`, `spec/dioxus-components/`

Note: the original 3-task decomposition (#23, #56, #106) covered only ~40% of the foundational spec sections. Five additional foundation tasks (#193–#197, 16 pts) close the remaining foundational gaps. Nine category epics (#407–#415, 268 pts) now cover all 112 Dioxus adapter components across 94 tasks organized by category. See [Epic #9](https://github.com/fogodev/ars-ui/issues/9) for the full sub-issue breakdown.

### Epic: Agnostic utility components

- Point target: `64` (revised from `8` after full decomposition — 2026-04-10)
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/utility/_category.md`, `spec/components/utility/*.md`

Note: the original "First utility slice" epic scoped only 11 components and had a single decomposition card (#24). A full decomposition (2026-04-10) expanded the epic to cover all 26 utility components as framework-agnostic core implementations. Twenty tasks (#199–#218, 64 pts) organized in 5 dependency waves now cover every component. Issue #24 was closed as superseded. See [Epic #10](https://github.com/fogodev/ars-ui/issues/10) for the full sub-issue breakdown.

### Epic: Agnostic input components

- Point target: `48`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/input/_category.md`, `spec/components/input/*.md`, `spec/foundation/07-forms.md`

All 14 input components (1 stateless, 11 stateful, 2 complex). Twelve tasks (#228–#251, 48 pts) organized in 2 dependency waves. See [Epic #220](https://github.com/fogodev/ars-ui/issues/220) for the full sub-issue breakdown.

### Epic: Agnostic selection components

- Point target: `55`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/selection/_category.md`, `spec/components/selection/*.md`, `spec/shared/selection-patterns.md`

All 9 selection components (2 stateful, 7 complex). Nine tasks (#232–#255, 55 pts) organized in 4 dependency waves. External dep: ars-collections (Epic #53). See [Epic #221](https://github.com/fogodev/ars-ui/issues/221) for the full sub-issue breakdown.

### Epic: Agnostic overlay components

- Point target: `50`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/overlay/_category.md`, `spec/components/overlay/*.md`, `spec/shared/z-index-stacking.md`

All 10 overlay components (4 stateful, 6 complex). Ten tasks (#238–#265, 50 pts) organized in 4 dependency waves. See [Epic #222](https://github.com/fogodev/ars-ui/issues/222) for the full sub-issue breakdown.

### Epic: Agnostic navigation components

- Point target: `33`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/navigation/_category.md`, `spec/components/navigation/*.md`

All 8 navigation components (1 stateless, 4 stateful, 3 complex). Seven tasks (#247–#267, 33 pts) organized in 3 dependency waves. External dep: TreeView depends on ars-collections TreeCollection (#83). See [Epic #223](https://github.com/fogodev/ars-ui/issues/223) for the full sub-issue breakdown.

### Epic: Agnostic date-time components

- Point target: `47`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/date-time/_category.md`, `spec/components/date-time/*.md`, `spec/shared/date-time-types.md`

All 8 date-time components (5 stateful, 3 complex). Eight tasks (#262–#292, 47 pts) organized in 4 dependency waves. External dep: all depend on ars-i18n (Epic #54). See [Epic #224](https://github.com/fogodev/ars-ui/issues/224) for the full sub-issue breakdown.

### Epic: Agnostic data-display components

- Point target: `40`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/data-display/_category.md`, `spec/components/data-display/*.md`

All 11 data-display components (4 stateless, 6 stateful, 1 complex). Nine tasks (#266–#286, 40 pts) organized in 3 dependency waves. External dep: GridList, TagGroup, Table depend on ars-collections (Epic #53). See [Epic #225](https://github.com/fogodev/ars-ui/issues/225) for the full sub-issue breakdown.

### Epic: Agnostic layout components

- Point target: `31`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/layout/_category.md`, `spec/components/layout/*.md`, `spec/shared/layout-shared-types.md`

All 11 layout components (5 stateless, 6 stateful). Eight tasks (#270–#281, 31 pts) in a single wave — no intra-epic dependencies. See [Epic #226](https://github.com/fogodev/ars-ui/issues/226) for the full sub-issue breakdown.

### Epic: Agnostic specialized components

- Point target: `55`
- Layer: `Component`
- Framework: `None` (agnostic core only; adapter work tracked under Epics #8/#9)
- Test tier: `Unit`
- Spec refs: `spec/components/specialized/_category.md`, `spec/components/specialized/*.md`

All 15 specialized components (3 stateless, 10 stateful, 2 complex). Eleven tasks (#288–#301, 55 pts) organized in 4 dependency waves. External dep: FileUpload depends on DnD interactions (#159–#161). See [Epic #227](https://github.com/fogodev/ars-ui/issues/227) for the full sub-issue breakdown.

### Epic: Leptos utility adapter components

- Point target: `40`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/utility/_category.md`, `spec/leptos-components/utility/*.md`, `spec/foundation/08-adapter-leptos.md`

All 26 utility components (17 stateless, 9 stateful). Twenty tasks organized in 5 dependency waves. See [Epic #303](https://github.com/fogodev/ars-ui/issues/303) for the full sub-issue breakdown.

### Epic: Leptos input adapter components

- Point target: `34`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/input/_category.md`, `spec/leptos-components/input/*.md`, `spec/foundation/08-adapter-leptos.md`

All 14 input components (1 stateless, 11 stateful, 2 complex). Twelve tasks organized in 2 dependency waves. See [Epic #304](https://github.com/fogodev/ars-ui/issues/304) for the full sub-issue breakdown.

### Epic: Leptos selection adapter components

- Point target: `34`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/selection/_category.md`, `spec/leptos-components/selection/*.md`, `spec/foundation/08-adapter-leptos.md`

All 9 selection components (2 stateful, 7 complex). Nine tasks organized in 4 dependency waves. See [Epic #305](https://github.com/fogodev/ars-ui/issues/305) for the full sub-issue breakdown.

### Epic: Leptos overlay adapter components

- Point target: `29`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/overlay/_category.md`, `spec/leptos-components/overlay/*.md`, `spec/foundation/08-adapter-leptos.md`

All 10 overlay components (4 stateful, 6 complex). Ten tasks organized in 4 dependency waves. See [Epic #306](https://github.com/fogodev/ars-ui/issues/306) for the full sub-issue breakdown.

### Epic: Leptos navigation adapter components

- Point target: `21`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/navigation/_category.md`, `spec/leptos-components/navigation/*.md`, `spec/foundation/08-adapter-leptos.md`

All 8 navigation components (1 stateless, 4 stateful, 3 complex). Seven tasks organized in 3 dependency waves. See [Epic #307](https://github.com/fogodev/ars-ui/issues/307) for the full sub-issue breakdown.

### Epic: Leptos date-time adapter components

- Point target: `29`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/date-time/_category.md`, `spec/leptos-components/date-time/*.md`, `spec/foundation/08-adapter-leptos.md`

All 8 date-time components (5 stateful, 3 complex). Eight tasks organized in 4 dependency waves. External dep: all depend on ars-i18n (Epic #54). See [Epic #308](https://github.com/fogodev/ars-ui/issues/308) for the full sub-issue breakdown.

### Epic: Leptos data-display adapter components

- Point target: `24`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/data-display/_category.md`, `spec/leptos-components/data-display/*.md`, `spec/foundation/08-adapter-leptos.md`

All 11 data-display components (4 stateless, 6 stateful, 1 complex). Nine tasks organized in 3 dependency waves. See [Epic #309](https://github.com/fogodev/ars-ui/issues/309) for the full sub-issue breakdown.

### Epic: Leptos layout adapter components

- Point target: `21`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/layout/_category.md`, `spec/leptos-components/layout/*.md`, `spec/foundation/08-adapter-leptos.md`

All 11 layout components (5 stateless, 6 stateful). Eight tasks in a single wave — no intra-epic dependencies. See [Epic #310](https://github.com/fogodev/ars-ui/issues/310) for the full sub-issue breakdown.

### Epic: Leptos specialized adapter components

- Point target: `32`
- Layer: `Adapter`
- Framework: `Leptos`
- Test tier: `Adapter`
- Spec refs: `spec/leptos-components/specialized/_category.md`, `spec/leptos-components/specialized/*.md`, `spec/foundation/08-adapter-leptos.md`

All 15 specialized components (3 stateless, 10 stateful, 2 complex). Eleven tasks organized in 4 dependency waves. External dep: FileUpload depends on DnD interactions (#159–#161). See [Epic #311](https://github.com/fogodev/ars-ui/issues/311) for the full sub-issue breakdown.

### Epic: Dioxus utility adapter components

- Point target: `44`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/utility/_category.md`, `spec/dioxus-components/utility/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 26 utility components (17 stateless, 9 stateful). Twenty tasks organized in 5 dependency waves. See [Epic #407](https://github.com/fogodev/ars-ui/issues/407) for the full sub-issue breakdown.

### Epic: Dioxus input adapter components

- Point target: `34`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/input/_category.md`, `spec/dioxus-components/input/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 14 input components (1 stateless, 11 stateful, 2 complex). Twelve tasks organized in 2 dependency waves. See [Epic #408](https://github.com/fogodev/ars-ui/issues/408) for the full sub-issue breakdown.

### Epic: Dioxus selection adapter components

- Point target: `34`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/selection/_category.md`, `spec/dioxus-components/selection/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 9 selection components (2 stateful, 7 complex). Nine tasks organized in 4 dependency waves. See [Epic #409](https://github.com/fogodev/ars-ui/issues/409) for the full sub-issue breakdown.

### Epic: Dioxus overlay adapter components

- Point target: `29`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/overlay/_category.md`, `spec/dioxus-components/overlay/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 10 overlay components (4 stateful, 6 complex). Ten tasks organized in 4 dependency waves. See [Epic #410](https://github.com/fogodev/ars-ui/issues/410) for the full sub-issue breakdown.

### Epic: Dioxus navigation adapter components

- Point target: `21`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/navigation/_category.md`, `spec/dioxus-components/navigation/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 8 navigation components (1 stateless, 4 stateful, 3 complex). Seven tasks organized in 3 dependency waves. See [Epic #411](https://github.com/fogodev/ars-ui/issues/411) for the full sub-issue breakdown.

### Epic: Dioxus date-time adapter components

- Point target: `29`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/date-time/_category.md`, `spec/dioxus-components/date-time/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 8 date-time components (5 stateful, 3 complex). Eight tasks organized in 4 dependency waves. External dep: all depend on ars-i18n (Epic #54). See [Epic #412](https://github.com/fogodev/ars-ui/issues/412) for the full sub-issue breakdown.

### Epic: Dioxus data-display adapter components

- Point target: `24`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/data-display/_category.md`, `spec/dioxus-components/data-display/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 11 data-display components (4 stateless, 6 stateful, 1 complex). Nine tasks organized in 3 dependency waves. See [Epic #413](https://github.com/fogodev/ars-ui/issues/413) for the full sub-issue breakdown.

### Epic: Dioxus layout adapter components

- Point target: `21`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/layout/_category.md`, `spec/dioxus-components/layout/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 11 layout components (5 stateless, 6 stateful). Eight tasks in a single wave — no intra-epic dependencies. See [Epic #414](https://github.com/fogodev/ars-ui/issues/414) for the full sub-issue breakdown.

### Epic: Dioxus specialized adapter components

- Point target: `32`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Adapter`
- Spec refs: `spec/dioxus-components/specialized/_category.md`, `spec/dioxus-components/specialized/*.md`, `spec/foundation/09-adapter-dioxus.md`

All 15 specialized components (3 stateless, 10 stateful, 2 complex). Eleven tasks organized in 4 dependency waves. External dep: FileUpload depends on DnD interactions (#159–#161). See [Epic #415](https://github.com/fogodev/ars-ui/issues/415) for the full sub-issue breakdown.

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

Note: this seed task only covered baseline validation types and field state. Follow-on tasks delivered FormContext (#37), form_submit machine (#38), and hidden-input helpers. A full spec audit (2026-04-10) found the original 3-task decomposition covered ~50% of the 4300-line spec. Eight additional tasks (#164–#171, 26 pts) now cover built-in validators, ValidatorsBuilder, Messages i18n, debounced async validation, and Fieldset/Field/Form component machines. See [Epic #5](https://github.com/fogodev/ars-ui/issues/5) for the full decomposition.

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

Note: this seed task only covered the baseline crate shell. Follow-on tasks delivered focus utilities (#39), scroll locking (#40), positioning engine (#66, #67, #115), z-index allocator (#68), scroll-into-view (#74), and the modality manager listener layer (implemented on `main` as part of PR #91, tracked by #72). A full spec audit (2026-04-10) found `11-dom-utilities.md` is 2804 lines across 10 sections — far larger than the ~400 lines originally estimated. Sixteen sub-issues now cover the full spec. See [Epic #6](https://github.com/fogodev/ars-ui/issues/6) for the full decomposition.

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

Note: this seed task established the crate shells only (~14 lines per backend). The full harness API (30+ methods, ElementHandle, KeyboardKey, AnyService, HarnessBackend trait) is covered by #178. Leptos and Dioxus backend implementations are covered by #181 and #182.

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

Note: this seed task delivered the tier split via xtask CI runner. Additional CI enforcement (adapter parity #184, snapshot linting #185, error variant coverage #186, nightly pipeline #188) are covered by Wave 2 tasks.

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

### #193: Implement ArsProvider context, reactive props, and controlled value helper in ars-dioxus

- Points: `5`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: none
- Spec refs: `spec/foundation/09-adapter-dioxus.md` §16, §16.1, §16.2, §2.3, §19
- Tests first: use_locale fallback, resolve_locale override, use_controlled_prop_sync skip-initial, use_sync_props state_changed/context_changed
- Acceptance: ArsContext, use_locale, use_icu_provider, resolve_locale, resolve_messages, use_sync_props (replacing todo!() stub), use_controlled_prop_sync, use_controlled_prop_sync_optional, use_machine_inner wired to ArsProvider
- Spec impact: `No spec change required`

### #194: Implement Dioxus adapter utilities — emit, event mapping, nonce collector, safe listeners

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: `#193`
- Spec refs: `spec/foundation/09-adapter-dioxus.md` §19.1, §13.1, §3.5.1, §10
- Tests first: emit with Some/None, emit_map transform, dioxus_key_to_keyboard_key mapping, ArsNonceCssCtx accumulation, use_safe_event_listener cleanup idempotency
- Acceptance: emit, emit_map, dioxus_key_to_keyboard_key, ArsNonceCssCtx/ArsNonceStyle wired to provider, use_safe_event_listener with stale-check guard
- Spec impact: `No spec change required`

### #195: Implement DioxusPlatform trait, platform implementations, and use_platform() hook

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: `#193`
- Spec refs: `spec/foundation/09-adapter-dioxus.md` §6, §6.1, §6.2, §6.3
- Tests first: NullPlatform no-ops, use_platform fallback chain, compile-gate feature tests
- Acceptance: DioxusPlatform trait, WebPlatform (web-sys), DesktopPlatform (native), NullPlatform (testing/SSR), use_platform() hook
- Spec impact: `No spec change required`

### #196: Implement SSR Hydration support in ars-dioxus

- Points: `3`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: `#193`
- Spec refs: `spec/foundation/09-adapter-dioxus.md` §20, §20.1, §20.2, §19.2
- Tests first: HydrationSnapshot serde, orphan inert cleanup, focus target validation, use_stable_id format
- Acceptance: `HydrationSnapshot<M>`, setup_focus_scope_hydration_safe with 5 spec rules, use_stable_id, debug hydration mismatch detection
- Spec impact: `No spec change required`

### #197: Implement ArsErrorBoundary component in ars-dioxus

- Points: `2`
- Layer: `Adapter`
- Framework: `Dioxus`
- Test tier: `Unit`
- Depends on: none
- Spec refs: `spec/foundation/09-adapter-dioxus.md` §21
- Tests first: children render when no error, fallback with data-ars-error and role=alert on error, ErrorContext::error() display
- Acceptance: ArsErrorBoundary wrapping Dioxus ErrorBoundary, accessible fallback UI, prelude export
- Spec impact: `No spec change required`

### ~~#24: Break the first utility slice into per-primitive delivery cards~~ (CLOSED)

Closed (2026-04-10): Superseded by the full decomposition of all 26 agnostic utility components under Epic #10. Twenty task issues (#199–#218) now cover every utility component. See [Epic #10](https://github.com/fogodev/ars-ui/issues/10) sub-issues.

## Backlog Reset

The original seed tasks correctly established workspace, core, subsystem, harness, and adapter shells. They did **not** fully surface the shared contracts required by the first utility slice. The missing follow-on work is now tracked in [foundation-gap-audit.md](./foundation-gap-audit.md), which:

- explains why `#24` is blocked,
- identifies the missing shared contracts in `ars-core`, `ars-derive`, `ars-dom`, `ars-forms`, provider/platform wiring, and interaction composition,
- defines the corrected issue-ready replacement task sequence, and
- names the next unblocked high-leverage task: typed connect primitives in `ars-core`.
