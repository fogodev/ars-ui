# Roadmap

## Objective

Build `ars-ui` from a spec-only workspace into an implementation workspace with:

- architecture-aligned crates
- stable core contracts
- reusable subsystem primitives
- framework-agnostic test harnesses
- a first shippable utility slice in both Leptos and Dioxus

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

Exit criteria:

- test-harness API is stable enough for the first component slice
- CI failures identify the failing tier

### Phase 4: First shippable utility slice

Scope:

- `ArsProvider`
- `AsChild`
- `VisuallyHidden`
- `FocusRing`
- `Button`
- `Toggle`
- `ToggleButton`
- `FocusScope`
- `Field`
- `Fieldset`
- `Form`

Exit criteria:

- all slice components exist in both adapters
- parity tests cover shared behavior and `data-ars-*`/ARIA output
- spec and implementation remain aligned for any discovered framework-specific constraints

## Spec synchronization rules

- Each implementation task must declare `Spec impact`.
- If the implementation proves the spec wrong or incomplete, update the spec in the same task.
- Shared abstraction changes go into `spec/foundation/` or `spec/shared/`.
- Adapter-specific realization belongs in `spec/foundation/08-adapter-leptos.md`, `spec/foundation/09-adapter-dioxus.md`, and the per-component adapter specs.
- Adapter code must not become the only authoritative explanation for future framework ports.
