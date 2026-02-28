---
adapter: dioxus
category: data-display
source_foundation: foundation/09-adapter-dioxus.md
---

# Data-Display Components — Dioxus Adapter

These documents map the framework-agnostic data-display contracts in `spec/components/data-display/*` onto Dioxus 0.7.x APIs.

## Scope

- Core behavior, state machines, anatomy, accessibility, and collection semantics remain defined by the framework-agnostic data-display specs.
- These Dioxus adapter specs define the Dioxus-facing `#[component]` APIs, `Element` and named slot shape, prop ownership, signal usage, context contracts, multi-platform behavior, and SSR or hydration constraints for data-display rendering.
- Stateless display primitives stay implementation-light but must still make host element choice, semantic repair, `ArsProvider`-driven locale formatting, and reduced-motion behavior explicit.
- Stateful data-display components must make controlled prop synchronization, callback timing, measurement ownership, live-region announcements, repeated structural nodes, and cleanup ordering explicit.
- Collection-heavy data-display components must document empty-state rendering, descendant registration, keyboard navigation, selection bridging, scroll-triggered loading, and RTL behavior rather than leaving them implied by the agnostic specs.
- Adapter sections are normative and implementation-facing. Canonical examples remain illustrative only.
- Adapter-owned behavior must not remain only in the agnostic specs. If a core data-display spec says the adapter handles semantic repair, live announcements, load-more wiring, measurement, fallback element choice, cleanup, or platform variance, the Dioxus spec must restate that behavior explicitly.
- Recommendation-level adapter guidance must also be restated explicitly rather than left in the agnostic prose.
- High-risk adapter-only behavior should land in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`.
- `Framework-Specific Behavior` is reserved for true Dioxus divergence.
- Per-component specs remain the primary source of truth when category guidance and component guidance overlap.

## Conventions

- All component APIs use explicit `#[derive(Props, Clone, PartialEq)]` structs per foundation spec Section 4. Zero-prop parts may use bare function signatures.
- Use plain Dioxus props by default. Introduce `Signal<T>` only when the adapter must observe a value after mount, such as controlled progress values, selected keys, sort descriptors, loading flags, or image source replacement.
- Use `Element` or explicit slot props consistently. Display components with structural subparts should keep adapter-owned wrappers explicit instead of flattening them into arbitrary consumer markup.
- Compound or collection-backed widgets use `use_context_provider` and `try_use_context`. Required consumers call `try_use_context::<T>().expect("Clear context requirement message")`.
- Machine-backed components use `use_machine::<...>()`, `derive(...)`, and controlled-sync patterns from the Dioxus adapter foundation spec.
- Components that announce value changes, sort updates, removals, or empty-state transitions must document whether they rely on semantic host markup, adapter-owned `aria-live` nodes, or shared live-region helpers.
- Components that measure layout, scroll range, media load status, or duplicated content width keep node refs, observers, timers, and cleanup instance-scoped. Cleanup is adapter-owned and must be explicit.
- Empty states for `GridList`, `TagGroup`, and `Table` are adapter-rendered structural nodes, not core-machine-owned DOM. Their insertion, announcement, and identity rules must be explicit in the adapter specs.
- SSR must preserve hydration-stable structure. Browser-only measurement, hover state, media load listeners, IntersectionObserver wiring, and scroll threshold checks start only after mount.
- Category-level helper taxonomy should stay consistent across the tree: attr-merge helpers, formatting helpers, live-announcement helpers, measurement helpers, collection-registration helpers, load-more helpers, semantic-repair helpers, and diagnostics helpers.
- Suggested build order is: stateless semantics (`badge`, `meter`, `skeleton`, `stat`), then single-instance stateful widgets (`avatar`, `progress`, `rating-group`, `marquee`), then collection widgets (`grid-list`, `tag-group`), then `table`.

### Final Section Structure

Every data-display adapter component spec in this tree uses this final section order:

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
- `Reference Implementation Skeleton` is required for every stateful or behavior-heavy data-display component and may explicitly say no expanded skeleton is required for purely semantic wrappers.

### Shared Terminology

- `semantic repair`: adapter-owned attrs, roles, labels, or wrapper selection needed so the rendered host still satisfies the core accessibility contract.
- `structural node`: a rendered node whose presence, multiplicity, or order is part of the adapter contract rather than incidental implementation detail.
- `announcement node`: an adapter-owned live-region target used when semantic host markup alone cannot communicate changes reliably.
- `measurement loop`: adapter-owned reads of DOM size, scroll range, media load state, or duplicated-content width used to keep rendering aligned with the core machine state.
- `registration`: adapter-owned mount or unmount bookkeeping for repeated items, row metadata, load-more sentinels, or live handles.
- `fallback path`: the documented degraded behavior used when a preferred host element, platform capability, or runtime primitive is unavailable.
- `traceability note`: a short parity note naming the agnostic adapter-owned concerns that were promoted explicitly into the adapter spec.
- `verification recipe`: a short `Test Oracle Notes` recipe describing the cheapest reliable way to validate a failure-prone adapter behavior.

### Cross-Category Consumption Notes

- `GridList`, `TagGroup`, and `Table` should consume shared collection helpers from the Dioxus adapter foundation rather than inventing ad hoc descendant registries or selection observation patterns.
- Any data-display component that needs live announcements should consume the utility `LiveRegion` semantics or a shared announcement helper instead of open-coding multiple hidden announcers.
- Components with interactive roots or polymorphic triggers should consume `as-child` instead of inventing custom root reassignment.
- Components with removable chips, rating items, or table row actions should reuse utility `button` or `toggle-button` interaction semantics when they expose a pressable subpart.
- Components that need `ArsProvider` environment context, layering, or portal behavior should defer to utility and overlay primitives rather than re-defining those contracts locally.

### Authoring Lint Checklist

- [ ] No adapter-owned data-display behavior remains only in the framework-agnostic spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful platform variance has a platform-matrix row instead of prose alone.
- [ ] High-risk widgets include a concise traceability note and a cheap verification recipe.
- [ ] Diagnostics say whether they are debug-only, warning-only, fail-fast, graceful-degradation, or no-op paths.

## Data-Display Index

- [Avatar](avatar.md)
- [Badge](badge.md)
- [GridList](grid-list.md)
- [Marquee](marquee.md)
- [Meter](meter.md)
- [Progress](progress.md)
- [RatingGroup](rating-group.md)
- [Skeleton](skeleton.md)
- [Stat](stat.md)
- [Table](table.md)
- [TagGroup](tag-group.md)
