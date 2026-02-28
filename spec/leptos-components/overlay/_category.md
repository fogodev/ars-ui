---
adapter: leptos
category: overlay
source_foundation: foundation/08-adapter-leptos.md
---

# Overlay Components — Leptos Adapter

These documents map the framework-agnostic overlay contracts in `spec/components/overlay/*` onto Leptos 0.8.x APIs.

## Scope

- Core behavior, state machines, parts, and accessibility rules remain defined by the framework-agnostic overlay specs.
- These Leptos adapter specs define the Leptos-facing component API, `Children` and slot shape, `Signal<T>` usage, context contracts, lifecycle wiring, and SSR or hydration behavior.
- Overlay components render into a portal root managed by `ArsProvider` and coordinate z-index allocation through `ZIndexAllocator`.
- Positioned overlays (Popover, Tooltip, HoverCard, Tour) integrate with the `ars-dom` positioning engine and set CSS custom properties on positioner elements.
- Modal overlays (Dialog, AlertDialog, Drawer) consume `FocusScope` for focus trapping and `Dismissable` for outside-interaction detection.
- Components with enter/exit animations compose `Presence` for mount/unmount lifecycle management.
- When a utility relies on DOM APIs, the adapter behavior is constrained by the web-only scope described in `spec/foundation/08-adapter-leptos.md`.
- Every component spec is contract-first: it explains the adapter API, exact part mapping, attr merge and ownership rules, prop sync and event mapping, registration and cleanup behavior, ref and node ownership, failure policy, implementation dependencies, anti-patterns, test-oracle guidance, and a final implementation checklist for future implementers.

## Conventions

- Use `Signal<T>` or `Option<Signal<T>>` only for props that are expected to react to upstream changes after mount, such as controlled `open` state.
- Use `Children` or named slot props consistently; context-only providers render no wrapper node unless the core overlay requires one.
- Use `provide_context` and `use_context` for compound overlays. Optional consumers read `use_context::<T>()`, while required consumers call `use_context::<T>().expect("Proper message explaining why the context is required")`.
- Use `use_machine::<...>()`, `derive(...)`, and prop watchers described in the Leptos adapter foundation spec when an overlay wraps a core machine.
- Overlay content must render into the portal root via `ArsProvider` context, not inline with the trigger.
- Z-index allocation must use the `ZIndexAllocator` context, not hardcoded values or local counters.
- The backdrop element must be a sibling of the content element inside the portal root, not a parent wrapper (backdrop sibling pattern).
- Positioned overlays must set CSS custom properties (`--ars-x`, `--ars-y`, `--ars-z-index`, `--ars-reference-width`, `--ars-reference-height`, `--ars-available-width`, `--ars-available-height`, `--ars-transform-origin`) on the positioner element after each positioning update.
- Timer-based behavior (Tooltip open/close delays, HoverCard delays, Toast auto-dismiss) must use client-only timers that are cleaned up on unmount.
- Compound overlay patterns (Dialog, Popover, Tooltip, etc.) use context-based root/part composition where the root provides context and parts consume it.
- Toast exposes an imperative queue context API (`toast::Provider`/`use_toast`) distinct from the standard machine-per-component pattern.
- Treat `as_child` as root reassignment, not part deletion.
- Canonical implementation sketches are illustrative examples; the contract sections and `Adapter Invariants` remain the source of truth.
- Every overlay component spec includes dedicated `Attr Merge and Ownership Rules`, `Prop Sync and Event Mapping`, `Registration and Cleanup Contract`, `Ref and Node Contract`, `State Machine Boundary Rules`, `Callback Payload Contract`, `Failure and Degradation Rules`, `Identity and Key Policy`, `SSR and Client Boundary Rules`, `Performance Constraints`, `Implementation Dependencies`, `Recommended Implementation Sequence`, `Anti-Patterns`, `Consumer Expectations and Guarantees`, `Platform Support Matrix`, `Debug Diagnostics and Production Policy`, `Shared Adapter Helper Notes`, `Test Oracle Notes`, `Reference Implementation Skeleton`, and `Implementation Checklist` sections.
- Every overlay component spec includes a dedicated `Adapter Invariants` section covering high-risk implementation rules such as event ordering, controlled sync, SSR or hydration requirements, focus trapping, dismiss behavior, portal rendering, z-index allocation, positioning, and cleanup ordering.
- These implementation sections are normative and implementation-facing; canonical examples remain illustrative only.
- Adapter-owned behavior must not remain only in the framework-agnostic spec. If a core overlay says the adapter must repair semantics, compose handlers, manage focus trapping, bridge form submission, own cleanup, or apply platform fallbacks, the Leptos adapter spec must restate that decision explicitly in adapter-facing sections.
- Adapter-facing recommendations must not remain only in the framework-agnostic spec either. If a core overlay says the adapter should repair directionality, expose helper context, or prefer a specific semantic fallback, the Leptos adapter spec must restate that guidance explicitly even when it remains recommendation-level rather than invariant-level.
- Framework API references in examples must stay aligned with supported Leptos `0.8.x` docs, including `use_context::<T>() -> Option<T>` for optional context access.
- Dependency notes describe implementation order guidance for future agents, not runtime dependency graphs.
- Anti-patterns are normative "Do not …" rules and implementation checklists are completion gates, not prose summaries.
- High-risk adapter-only behavior should land in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`, while `Framework-Specific Behavior` should be reserved for true Leptos-specific divergence.
- Recommendation-level adapter guidance should usually land in `Framework-Specific Behavior`, `Accessibility and SSR Notes`, and `Implementation Checklist`; only promote it into `Adapter Invariants` when correctness or contract parity requires it.
- Platform support matrices use the exact support labels `full support`, `fallback path`, `client-only`, `SSR-safe empty`, and `not applicable`.
- Debug diagnostics tables use the exact behavior labels `debug warning`, `warn and ignore`, `degrade gracefully`, `fail fast`, and `no-op`.
- Per-component specs remain the primary source of truth even when diagnostics, helper concepts, or platform notes repeat across the overlay layer.
- Expanded reference implementation skeletons are required for all stateful and complex overlays; simpler overlays must still include the section and may explicitly say no expanded skeleton is needed.
- Shared helper notes describe reusable implementation infrastructure, not public runtime APIs.
- Category-level helper taxonomy should stay consistent across the tree: merge helpers, portal helpers, positioning helpers, z-index helpers, focus-scope helpers, dismiss helpers, timer helpers, registry helpers, warning helpers, measurement helpers, platform capability helpers, and context publication helpers.
- Suggested overlay-layer build order is: `presence` first (animation lifecycle foundation), then `popover` and `dialog` (foundational positioned and modal overlays), then `alert-dialog`, `drawer`, and `tooltip` (depend on dialog or popover patterns), then `hover-card` and `toast`, then `floating-panel` and `tour` (most self-contained complex overlays).

### Final Section Structure

Every overlay adapter component spec in this tree uses this final section order:

1. `Purpose and Adapter Scope`
2. `Public Adapter API`
3. `Mapping to Core Component Contract`
4. `Part Mapping`
5. `Attr Merge and Ownership Rules`
6. `Composition / Context Contract`
7. `Prop Sync and Event Mapping`
8. `Registration and Cleanup Contract`
9. `Ref and Node Contract`
10. `State Machine Boundary Rules`
11. `Callback Payload Contract`
12. `Failure and Degradation Rules`
13. `Identity and Key Policy`
14. `SSR and Client Boundary Rules`
15. `Performance Constraints`
16. `Implementation Dependencies`
17. `Recommended Implementation Sequence`
18. `Anti-Patterns`
19. `Consumer Expectations and Guarantees`
20. `Platform Support Matrix`
21. `Debug Diagnostics and Production Policy`
22. `Shared Adapter Helper Notes`
23. `Framework-Specific Behavior`
24. `Canonical Implementation Sketch`
25. `Reference Implementation Skeleton`
26. `Adapter Invariants`
27. `Accessibility and SSR Notes`
28. `Parity Summary and Intentional Deviations`
29. `Test Scenarios`
30. `Test Oracle Notes`
31. `Implementation Checklist`

### Legends

- Platform support matrices use the exact support labels:
  `full support`, `fallback path`, `client-only`, `SSR-safe empty`, and `not applicable`.
- Debug diagnostics tables use the exact behavior labels:
  `debug warning`, `warn and ignore`, `degrade gracefully`, `fail fast`, and `no-op`.
- `Consumer Expectations and Guarantees` must distinguish:
  `Consumers may assume ...` from `Consumers must not assume ...`.
- `Shared Adapter Helper Notes` describe reusable implementation infrastructure only; they do not define public Leptos APIs or required concrete symbol names.
- `Reference Implementation Skeleton` is required for every stateful and complex overlay and must be tighter than the canonical sketch by showing helper boundaries, ownership, sequencing, and SSR/client gating.

### Shared Terminology

- `registration`: adapter-owned mount or unmount bookkeeping for descendants, temporary resources, listeners, or repeated items.
- `structural node`: a rendered node whose presence, order, or identity is part of the adapter contract rather than incidental implementation detail.
- `live handle`: a runtime node reference that becomes meaningful only after mount and cannot be replaced by a static ID string.
- `fallback path`: the documented degraded behavior used when a platform capability, browser API, or host primitive is unavailable.
- `semantic repair`: adapter-owned attrs, roles, IDs, or handler normalization added so the rendered host still satisfies the core accessibility and interaction contract.
- `traceability note`: a short parity-section note naming the agnostic adapter-owned concerns that were promoted explicitly into the adapter spec.
- `verification recipe`: a short `Test Oracle Notes` recipe describing the cheapest reliable way to validate a failure-prone adapter behavior.
- `portal root`: the adapter-owned DOM container (`ars-portal-root`) for rendering overlay content outside the component tree.
- `positioning engine`: the `ars-dom` floating positioning system that computes overlay placement relative to anchor elements.
- `anchor element`: the trigger or reference element used by the positioning engine to determine overlay placement.
- `backdrop sibling pattern`: the required rendering pattern where backdrop and content are siblings in the portal root, not parent-child.
- `dismiss boundary`: the set of elements considered "inside" for outside-interaction detection via `Dismissable`.
- `timer-based lifecycle`: hover/focus delays for Tooltip and HoverCard, and auto-dismiss for Toast, managed through client-only timers.

### Cross-Category Consumption Notes

- Overlay components consume `dismissable`, `focus-scope`, `ars-provider`, and `z-index-allocator` from the utility category rather than re-defining containment, focus trapping, portal rendering, or layer allocation contracts.
- Overlay components with animation lifecycle compose `Presence` from within the overlay category itself.
- Dialog, AlertDialog, and Drawer share modal patterns; AlertDialog and Drawer may reference the Dialog adapter spec for shared concerns.
- Popover, HoverCard, Tooltip, and Tour share positioning patterns via the `ars-dom` positioning engine.
- Toast has a unique imperative queue pattern not shared with other overlays.
- Form-bound overlays (if any) should consume `field`, `fieldset`, and `form` from the utility category.

### Authoring Lint Checklist

- [ ] No adapter-owned behavior remains only in the framework-agnostic spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful platform variance has a platform-matrix row instead of prose alone.
- [ ] High-risk overlays include a concise traceability note and a cheap verification recipe.
- [ ] Debug diagnostics say whether they are debug-only, fail-fast, warning-only, or graceful-degradation paths.

## Overlay Index

- [AlertDialog](alert-dialog.md)
- [Dialog](dialog.md)
- [Drawer](drawer.md)
- [FloatingPanel](floating-panel.md)
- [HoverCard](hover-card.md)
- [Popover](popover.md)
- [Presence](presence.md)
- [Toast](toast.md)
- [Tooltip](tooltip.md)
- [Tour](tour.md)
