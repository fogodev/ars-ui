# GitHub Project Operating Model

## Project shape

Use one GitHub Project with issue-backed items only.

Top-level epics:

- Workspace bootstrap
- Core runtime
- A11y
- Interactions
- Forms
- DOM utilities
- Test infrastructure
- Leptos adapter
- Dioxus adapter
- Agnostic utility components
- Spec synchronization

## Required Project fields

### `Status`

GitHub Project V2 provides a built-in status field. The current board keeps the built-in values:

- `Todo`
- `In Progress`
- `Done`

Use this field as the coarse lifecycle state that GitHub exposes natively across views and automations.

### `Iteration`

The board uses a dedicated iteration field for timeboxed planning. The initial setup seeds six two-week iterations starting on `2026-04-06`.

Use this field to group active roadmap work into planning windows without overloading issue status.

### `Layer`

Single-select values:

- `Core`
- `Subsystem`
- `Harness`
- `Adapter`
- `Component`
- `Spec`

### `Framework`

Single-select values:

- `None`
- `Leptos`
- `Dioxus`
- `Both`

### `Test Tier`

Single-select values:

- `Unit`
- `Integration`
- `Adapter`
- `Mixed`

### `Points`

Single-select values:

- `1`
- `2`
- `3`
- `5`
- `8`
- `13`

### `Spec Status`

Single-select values:

- `None`
- `Needs Update`
- `Updated`

### Text fields

- `Spec Refs`
- `Depends On`
- `Agent`

`Depends On` is a readable mirror for board/table views. Native GitHub issue dependencies are the source of truth for blockers and execution order.

## Workflow gates

- An issue stays in `Status = Todo` until it has spec refs, tests-first notes, a point estimate, and no unresolved blockers.
- An issue moves to `Status = In Progress` only while someone is actively executing it.
- An issue reaches `Status = Done` only after the named tests exist, pass in CI, and the issue is closed or otherwise complete.
- An issue with `Spec Status = Needs Update` cannot be closed.
- A blocked issue uses native GitHub issue dependencies as the blocker graph and records any ambiguity in the issue body/comments.
- If work stops because of a blocker, move the issue back to `Status = Todo`.
- Component and adapter work cannot start before upstream dependencies are `Done`.
- `13`-point work is not agent-ready work; split it before pickup.
- `8`-point tasks require a written explanation of why additional decomposition would create harmful coupling.

## Recommended views

Create these saved views in the GitHub UI:

- `Backlog`: table view filtered to `Status = Todo`
- `In Flight`: table or board view filtered to `Status = In Progress`
- `By Epic`: table view showing `Parent issue` and `Sub-issues progress`
- `Roadmap`: roadmap view grouped by `Iteration`
- `Blocked`: table view filtered to issues with unresolved dependencies or a non-empty `Depends On` field

## UI-only setup

These parts are still best configured in the GitHub UI:

- Enable built-in project workflows that keep `Status` aligned when issues or pull requests close.
- Surface `Parent issue`, `Sub-issues progress`, and `Iteration` in the default table view.
- Add project insights for `Status`, `Layer`, `Points`, and `Iteration`.

## Fibonacci sizing policy

Points represent delivery effort plus uncertainty.

- `1`: trivial, localized, low-risk change
- `2`: straightforward single-scope task
- `3`: normal bounded task
- `5`: preferred upper bound for most agent-ready work
- `8`: rare tightly-coupled work item
- `13`: oversized slice that must be decomposed

Split tasks by:

1. layer
2. test tier
3. framework after shared contracts stabilize
4. behavior, not arbitrary file boundaries
