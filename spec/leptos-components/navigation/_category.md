---
adapter: leptos
category: navigation
source_foundation: foundation/08-adapter-leptos.md
---

# Navigation Components — Leptos Adapter

These documents map the framework-agnostic navigation contracts in `spec/components/navigation/*` onto Leptos 0.8.x APIs.

## Scope

- Core behavior, state machines, parts, and accessibility rules remain defined by the framework-agnostic navigation specs.
- These Leptos adapter specs define Leptos-facing `#[component]` signatures, `Children` and slot shape, `Signal<T>` usage, context contracts, descendant registration, lifecycle wiring, and SSR or hydration behavior.
- Navigation adapters cover both simple semantic output (`Breadcrumbs`, `Link`) and compound machine-backed structures (`Accordion`, `Tabs`, `NavigationMenu`, `TreeView`).
- Roving focus, focus-visible state, anchor-vs-button repair, router integration, lazy panel mounting, live announcements, and collection-backed descendant registration must be specified explicitly in adapter-facing sections.
- Canonical examples are illustrative only. The implementation sections, invariants, tests, and checklist remain normative.

## Conventions

- Use `Signal<T>` or `Option<Signal<T>>` only for props that are expected to react after mount, such as controlled open, selected, disabled, page, or step values.
- Use `Children` or named slot props consistently. Compound navigation roots own context publication and descendants consume required context with `use_context::<T>().expect(...)`.
- Use `use_machine::<...>()`, `derive(...)`, and focused prop watchers from `foundation/08-adapter-leptos.md` when the core component is machine-backed.
- Treat `Link` and link-like triggers as anchor-first surfaces. Only repair `role="link"` and keyboard activation when the consumer intentionally renders a non-anchor root.
- Treat button-like triggers as native-button-first surfaces. Only synthesize Space-key activation when the rendered node is not a native `<button>`.
- Roving focus contracts must document registration order, live node ownership, and focus restoration explicitly. Registration order always follows rendered DOM order.
- Presence rules for `Tabs` panels and `NavigationMenu` content must state whether the adapter keeps closed content mounted, delays unmount, or uses a viewport handoff.
- `Pagination` page-change announcements and `Tabs` reorder announcements must document ownership of the hidden live-region surface and the clear-then-insert timing policy.
- `TreeView` and other repeated navigation structures must document descendant registration, key identity, collection-backed ordering, and cleanup behavior rather than leaving it implied.
- Recommendation-level adapter guidance must also be restated explicitly. High-risk adapter-only behavior belongs in `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist`.
- `Framework-Specific Behavior` is reserved for true Leptos divergence, such as `Signal<T>`, `Children`, `provide_context`, `use_context`, and `NodeRef` composition details.
- Per-component specs remain the primary source of truth whenever this category doc overlaps with a specific component spec.

### Final Section Structure

Every navigation adapter component spec in this tree uses this final section order:

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
- `Reference Implementation Skeleton` is required for every stateful or complex navigation component and must be tighter than the canonical sketch by showing helper boundaries, ownership, sequencing, and SSR or client gating.

### Shared Terminology

- `registration`: adapter-owned mount or unmount bookkeeping for descendants, timers, measurement targets, or repeated items.
- `roving target`: the live focusable node that currently owns `tabindex="0"` inside a composite navigation widget.
- `semantic repair`: adapter-owned attrs, roles, IDs, or handlers added so the rendered host still satisfies the core accessibility and interaction contract.
- `viewport handoff`: adapter-owned movement of active `NavigationMenu` content into the optional viewport container while preserving ownership and ARIA linkage.
- `fallback path`: the documented degraded behavior used when a platform capability, router, measurement API, or browser primitive is unavailable.
- `traceability note`: a short parity-section note naming the agnostic adapter-owned concerns that were promoted explicitly into the adapter spec.
- `verification recipe`: a short `Test Oracle Notes` recipe describing the cheapest reliable way to validate a failure-prone adapter behavior.

### Shared Helper Taxonomy

- attr merge helpers
- roving-focus helpers
- button-or-anchor semantic helpers
- router integration helpers
- presence helpers
- live-region helpers
- collection and descendant registration helpers
- measurement and indicator helpers
- timer helpers
- warning and diagnostics helpers

### Cross-Category Consumption Notes

- `Accordion` may reuse disclosure and presence guidance from the layout `collapsible` adapter docs, but must keep accordion-specific heading, registration, and keyboard rules explicit.
- `Tabs`, `NavigationMenu`, and `Pagination` should consume utility-layer `button`, `as-child`, `live-region`, and `keyboard` patterns instead of restating generic trigger semantics.
- `NavigationMenu` should align with overlay-layer measurement, portal, presence, and z-index guidance where the core navigation contract delegates those concerns to the adapter.
- `Link` should remain the source of truth for router-aware anchor semantics rather than duplicating route interception rules in every higher-level navigation surface.
- `TreeView` should align with collection and ordered-registry patterns already used by complex repeated widgets elsewhere in the adapter tree.

### Authoring Lint Checklist

- [ ] No adapter-owned behavior remains only in the framework-agnostic navigation spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful platform variance has a platform-matrix row instead of prose alone.
- [ ] High-risk navigation behavior lands in `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist`.
- [ ] Router interception, live announcements, measurement, timers, and descendant cleanup are documented where the component uses them.
- [ ] Every new `Test Scenario` names at least one preferred oracle in `Test Oracle Notes`.

## Navigation Index

- [Accordion](accordion.md)
- [Breadcrumbs](breadcrumbs.md)
- [Link](link.md)
- [NavigationMenu](navigation-menu.md)
- [Pagination](pagination.md)
- [Steps](steps.md)
- [Tabs](tabs.md)
- [TreeView](tree-view.md)
