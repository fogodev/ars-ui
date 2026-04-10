# Roadmap

## Objective

Build `ars-ui` from a spec-only workspace into an implementation workspace with:

- architecture-aligned crates
- stable core contracts
- reusable subsystem primitives
- framework-agnostic test harnesses
- the full agnostic utility component layer (26 components)

## Phase Order

### Phase 0: Workspace bootstrap

Outcome:

- the workspace includes the architecture-defined crates
- each crate compiles with a minimal public surface
- release/profile defaults match the architecture baseline

Exit criteria:

- `cargo check --workspace` passes
- `cargo test --workspace` passes for bootstrap tests

### Phase 1: Core contract lock

Outcome:

- `ars-core` defines the minimum contract for `Machine`, `Service`, `TransitionPlan`, `PendingEffect`, `Bindable`, `ConnectApi`, and `AttrMap`
- `ars-derive` exposes the initial derive surface required to unblock anatomy and ID-related work

Exit criteria:

- unit tests cover the initial service runtime and controlled/uncontrolled behavior
- downstream crates compile against the shared contract without redefining local variants

### Phase 2: Cross-cutting subsystem base

Outcome:

- `ars-a11y`, `ars-interactions`, `ars-forms`, and `ars-dom` provide the shared primitives needed by the first utility slice

Exit criteria:

- each subsystem has a bounded set of unit and integration tests
- adapter crates consume shared primitives instead of copy-pasting framework-local logic

### Phase 3: Testing platform

Outcome:

- `ars-test-harness`, `ars-test-harness-leptos`, and `ars-test-harness-dioxus` expose a unified adapter testing entry point
- CI runs unit, integration, and adapter suites separately
- ARIA assertion helpers are available for all component tests
- `insta` snapshot infrastructure is wired into CI
- Adapter parity types (`ParityTestCase`, `InteractionTestCase`) enable cross-adapter testing

Exit criteria:

- test-harness API is stable enough for the first component slice
- CI failures identify the failing tier
- `ars-core/src/test_helpers.rs` exports 35+ ARIA assertion functions
- `insta` snapshot tests compile and CI rejects unapproved changes
- Both adapter backends can mount, query, and interact with components

Status (2026-04-10): Phase 3 crate shells and CI tier split are done (#19, #20). The full harness API, ARIA helpers, snapshot setup, adapter backends, parity types, CI enforcement, mock infrastructure, and nightly pipeline remain as 11 open tasks (#178–#188, 34 pts). See [Epic #7](https://github.com/fogodev/ars-ui/issues/7).

#### Adapter foundation audit (2026-04-10)

An audit of Epic #8 (Leptos adapter) revealed that the original 3 tasks (#22, #55, #105) covered ~40% of the foundational spec sections in `08-adapter-leptos.md`. Two new tasks were added to close the gaps before component work begins:

- [#190](https://github.com/fogodev/ars-ui/issues/190) — ArsProvider context, reactive props, controlled value helper (5 pts)
- [#191](https://github.com/fogodev/ars-ui/issues/191) — emit/emit_map, event mapping, nonce CSS collector, safe event listeners (3 pts)

A symmetric audit of Epic #9 (Dioxus adapter) confirmed the same gaps plus Dioxus-unique sections. Five new tasks were added (16 pts):

- [#193](https://github.com/fogodev/ars-ui/issues/193) — ArsProvider context, reactive props, controlled value helper (5 pts, symmetric with #190)
- [#194](https://github.com/fogodev/ars-ui/issues/194) — emit/emit_map, event mapping, nonce CSS collector, safe event listeners (3 pts, symmetric with #191)
- [#195](https://github.com/fogodev/ars-ui/issues/195) — DioxusPlatform trait, WebPlatform, DesktopPlatform, NullPlatform, use_platform() (3 pts, Dioxus-unique)
- [#196](https://github.com/fogodev/ars-ui/issues/196) — SSR Hydration: HydrationSnapshot, FocusScope hydration safety (3 pts)
- [#197](https://github.com/fogodev/ars-ui/issues/197) — ArsErrorBoundary component (2 pts)

See `foundation-completion-roadmap.md` for full task details and `foundation-gap-audit.md` for the gap matrix.

### Phase 4: Agnostic utility components

Scope:

All 26 utility components defined in `spec/components/utility/`, implemented as framework-agnostic core (state machines, ConnectApi, Props/Api/Part types):

**Stateless (17):** AsChild, ArsProvider, ClientOnly, Dismissable, DownloadTrigger, Field, Fieldset, FocusRing, Form, Group, Heading, Highlight, Keyboard, Landmark, Separator, VisuallyHidden, ZIndexAllocator

**Stateful (9):** ActionGroup, Button, DropZone, FocusScope, LiveRegion, Swap, Toggle, ToggleButton, ToggleGroup

Decomposed into 20 tasks (64 story points) organized in 5 dependency waves. See [Epic #10](https://github.com/fogodev/ars-ui/issues/10) for the full task breakdown with sub-issues.

Exit criteria:

- all 26 utility components have agnostic core implementations
- state machines (stateful components) match their spec §1 exactly
- ConnectApi implementations produce correct ARIA attributes per spec §2
- all public types documented per workspace `missing_docs` lint
- all tests pass with zero warnings
- spec and implementation remain aligned; any mismatch resolved in the same task

Status (2026-04-10): Epic repurposed from "First utility slice" (11 components) to cover all 26 agnostic utility components. Issue #24 (decomposition card) closed as superseded. Twenty new task issues (#199–#218) created as sub-issues of Epic #10.

## Spec synchronization rules

- Each implementation task must declare `Spec impact`.
- If the implementation proves the spec wrong or incomplete, update the spec in the same task.
- Shared abstraction changes go into `spec/foundation/` or `spec/shared/`.
- Adapter-specific realization belongs in `spec/foundation/08-adapter-leptos.md`, `spec/foundation/09-adapter-dioxus.md`, and the per-component adapter specs.
- Adapter code must not become the only authoritative explanation for future framework ports.
