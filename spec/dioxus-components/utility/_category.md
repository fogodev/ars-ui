---
adapter: dioxus
category: utility
source_foundation: foundation/09-adapter-dioxus.md
---

# Utility Components — Dioxus Adapter

These documents map the framework-agnostic utility contracts in `spec/components/utility/*` onto Dioxus 0.7.x APIs.

## Scope

- Core behavior, state machines, parts, and accessibility rules remain defined by the framework-agnostic utility specs.
- These Dioxus adapter specs define the Dioxus-facing component API, `Element` and slot shape, `Signal<T>` usage, context contracts, lifecycle wiring, and platform-specific behavior.
- When a utility relies on DOM APIs, the adapter behavior is constrained by the Dioxus platform split described in `spec/foundation/09-adapter-dioxus.md`.
- Every component spec is contract-first: it explains the adapter API, exact part mapping, attr merge and ownership rules, prop sync and event mapping, registration and cleanup behavior, ref and node ownership, failure policy, implementation dependencies, anti-patterns, test-oracle guidance, and a final implementation checklist for future implementers.

## Conventions

- All component APIs use explicit `#[derive(Props, Clone, PartialEq)]` structs per foundation spec Section 4. Zero-prop parts may use bare function signatures.
- Use plain props by default; use `Signal<T>` only when the adapter must watch a value after mount.
- Use `Element` or explicit slot props consistently; context-only providers render no wrapper node unless the core utility requires one.
- Use `use_context_provider` and `try_use_context` for compound utilities.
- Use `use_machine::<...>()`, `derive(...)`, and prop synchronization from the Dioxus adapter foundation spec when a utility wraps a core machine.
- Treat `as_child` as root reassignment, not part deletion.
- Document repeated structural nodes explicitly when multiplicity matters, such as both `DismissButton` instances in `Dismissable`.
- Canonical implementation sketches are illustrative examples; the contract sections and `Adapter Invariants` remain the source of truth.
- Every utility component spec includes dedicated `Attr Merge and Ownership Rules`, `Prop Sync and Event Mapping`, `Registration and Cleanup Contract`, `Ref and Node Contract`, `State Machine Boundary Rules`, `Callback Payload Contract`, `Failure and Degradation Rules`, `Identity and Key Policy`, `SSR and Client Boundary Rules`, `Performance Constraints`, `Implementation Dependencies`, `Recommended Implementation Sequence`, `Anti-Patterns`, `Consumer Expectations and Guarantees`, `Platform Support Matrix`, `Debug Diagnostics and Production Policy`, `Shared Adapter Helper Notes`, `Test Oracle Notes`, `Reference Implementation Skeleton`, and `Implementation Checklist` sections.
- Every utility component spec includes a dedicated `Adapter Invariants` section covering high-risk implementation rules such as event ordering, controlled sync, SSR or hydration requirements, native semantics, and cleanup ordering.
- These implementation sections are normative and implementation-facing; canonical examples remain illustrative only.
- Adapter-owned behavior must not remain only in the framework-agnostic spec. If a core utility says the adapter must repair semantics, compose handlers, bridge form submission, own cleanup, or apply platform fallbacks, the Dioxus adapter spec must restate that decision explicitly in adapter-facing sections.
- Adapter-facing recommendations must not remain only in the framework-agnostic spec either. If a core utility says the adapter should repair directionality, expose helper context, or prefer a specific semantic fallback, the Dioxus adapter spec must restate that guidance explicitly even when it remains recommendation-level rather than invariant-level.
- Framework API references in examples must stay aligned with supported Dioxus `0.7.x` docs.
- Dioxus examples must stay aligned with supported `0.7.x` APIs.
- Dependency notes describe implementation order guidance for future agents, not runtime dependency graphs.
- Anti-patterns are normative “Do not …” rules and implementation checklists are completion gates, not prose summaries.
- High-risk adapter-only behavior should land in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`, while `Framework-Specific Behavior` should be reserved for true Dioxus-specific divergence.
- Recommendation-level adapter guidance should usually land in `Framework-Specific Behavior`, `Accessibility and SSR Notes`, and `Implementation Checklist`; only promote it into `Adapter Invariants` when correctness or contract parity requires it.
- Platform support matrices use the exact support labels `full support`, `fallback path`, `client-only`, `SSR-safe empty`, and `not applicable`.
- Debug diagnostics tables use the exact behavior labels `debug warning`, `warn and ignore`, `degrade gracefully`, `fail fast`, and `no-op`.
- Per-component specs remain the primary source of truth even when diagnostics, helper concepts, or platform notes repeat across the utility layer.
- Expanded reference implementation skeletons are required for all stateful utilities; simpler or semantic-only utilities must still include the section and may explicitly say no expanded skeleton is needed.
- Shared helper notes describe reusable implementation infrastructure, not public runtime APIs.
- Category-level helper taxonomy should stay consistent across the tree: merge helpers, registry helpers, hidden-input helpers, warning helpers, measurement helpers, platform capability helpers, and focus/context publication helpers.
- Suggested utility-layer build order is: `as-child`, then `button`, then `toggle-button`, then `toggle-group`; `field`/`fieldset` before form-bound controls; `ars-provider`/`z-index-allocator` before overlay consumers; `focus-scope`/`dismissable` before overlay shells.

### Final Section Structure

Every utility adapter component spec in this tree uses this final section order:

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
- `Shared Adapter Helper Notes` describe reusable implementation infrastructure only; they do not define public Dioxus APIs or required concrete symbol names.
- `Reference Implementation Skeleton` is required for every stateful utility and must be tighter than the canonical sketch by showing helper boundaries, ownership, sequencing, and SSR/client gating.

### Shared Terminology

- `registration`: adapter-owned mount or unmount bookkeeping for descendants, temporary resources, listeners, or repeated items.
- `structural node`: a rendered node whose presence, order, or identity is part of the adapter contract rather than incidental implementation detail.
- `live handle`: a runtime node reference that becomes meaningful only after mount and cannot be replaced by a static ID string.
- `fallback path`: the documented degraded behavior used when a platform capability, browser API, or host primitive is unavailable.
- `semantic repair`: adapter-owned attrs, roles, IDs, or handler normalization added so the rendered host still satisfies the core accessibility and interaction contract.
- `traceability note`: a short parity-section note naming the agnostic adapter-owned concerns that were promoted explicitly into the adapter spec.
- `verification recipe`: a short `Test Oracle Notes` recipe describing the cheapest reliable way to validate a failure-prone adapter behavior.

### Cross-Category Consumption Notes

- Overlay and popup categories should consume `dismissable`, `focus-scope`, `ars-provider`, and `z-index-allocator` rather than re-defining containment, focus trapping, or layer allocation contracts.
- Form-bound controls should consume `field`, `fieldset`, and `form` so IDs, described-by wiring, validation state, and registration semantics stay uniform.
- Polymorphic interactive roots should consume `as-child` before inventing ad-hoc root reassignment or vnode merging.
- Selection and pressed-state composites should consume `button`, `toggle-button`, and `toggle-group` before introducing new hidden-input or roving-focus behavior.
- Semantic or typography-focused categories may consume `heading`, `landmark`, `group`, `separator`, `keyboard`, and `highlight` for normalized semantic output instead of duplicating repair logic.

### Authoring Lint Checklist

- [ ] No adapter-owned behavior remains only in the framework-agnostic spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful platform variance has a platform-matrix row instead of prose alone.
- [ ] High-risk utilities include a concise traceability note and a cheap verification recipe.
- [ ] Debug diagnostics say whether they are debug-only, fail-fast, warning-only, or graceful-degradation paths.

## Utility Index

- [ActionGroup](action-group.md)
- [AsChild](as-child.md)
- [Button](button.md)
- [ClientOnly](client-only.md)
- [Dismissable](dismissable.md)
- [DownloadTrigger](download-trigger.md)
- [DropZone](drop-zone.md)
- [ArsProvider](ars-provider.md)
- [Field](field.md)
- [Fieldset](fieldset.md)
- [FocusRing](focus-ring.md)
- [FocusScope](focus-scope.md)
- [Form](form.md)
- [Group](group.md)
- [Heading](heading.md)
- [Highlight](highlight.md)
- [Keyboard](keyboard.md)
- [Landmark](landmark.md)
- [LiveRegion](live-region.md)
- [Separator](separator.md)
- [Swap](swap.md)
- [Toggle](toggle.md)
- [ToggleButton](toggle-button.md)
- [ToggleGroup](toggle-group.md)
- [VisuallyHidden](visually-hidden.md)
- [ZIndexAllocator](z-index-allocator.md)
