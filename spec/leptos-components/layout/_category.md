---
adapter: leptos
category: layout
source_foundation: foundation/08-adapter-leptos.md
---

# Layout Components — Leptos Adapter

These documents map the framework-agnostic layout contracts in `spec/components/layout/*` onto Leptos `0.8.x` APIs.

## Scope

- Core behavior, parts, accessibility, and state machines remain defined by the framework-agnostic layout specs.
- These Leptos adapter specs define the Leptos-facing API surface, `Children` and slot shape, `Signal<T>` usage, context contracts, SSR or hydration behavior, measurement wiring, and cleanup rules.
- Stateless layout primitives remain simple structural wrappers; stateful layout widgets must make observer ownership, ref requirements, portal lifecycles, and pointer or keyboard repair explicit.
- Layout specs are implementation-facing and normative in their adapter sections. Canonical examples remain illustrative only.
- Adapter-owned behavior must not remain implied in the agnostic spec. If a core layout spec says the adapter handles semantic repair, measurement, portal ownership, cleanup, or platform fallback, the Leptos spec must restate it explicitly.
- Recommendation-level adapter guidance must also be restated explicitly rather than hidden in the agnostic prose.
- High-risk adapter-only behavior should land in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`.
- `Framework-Specific Behavior` is reserved for true Leptos divergence.
- Per-component specs remain the primary source of truth when category guidance and component guidance overlap.

## Conventions

- Use plain Leptos props for static layout values and `Signal<T>` only for props expected to react after mount, such as controlled open state, controlled slide index, or externally driven visibility.
- Use `Children` or explicit named slot props consistently. Stateless primitives render exactly one structural root unless the core spec requires a wrapper plus child node, such as `Frame` with ratio boxing.
- Compound widgets use `provide_context` and `use_context` for descendant coordination. Optional consumers read `use_context::<T>()`; required consumers call `use_context::<T>().expect("Clear context requirement message")`.
- Machine-backed layout widgets use `use_machine::<...>()`, `derive(...)`, and effect-based prop synchronization from the Leptos adapter foundation guidance.
- Measurement-heavy widgets keep `ResizeObserver`, pointer capture, scroll listeners, timers, and portal mount ownership instance-scoped. Cleanup is adapter-owned and must be explicit.
- Layout shared types such as spacing tokens, directionality, and orientation helpers may be resolved from the nearest `ArsProvider` environment context, but missing optional context must always degrade gracefully.
- SSR must preserve hydration-stable structure. Browser-only listeners, observers, focus repair, portal reparenting, and geometry reads wait until mount.
- Category-level helper taxonomy should stay consistent across the tree: attr-merge helpers, measurement helpers, observer helpers, registration helpers, focus publication helpers, portal helpers, diagnostics helpers, and fallback or capability helpers.
- Suggested build order is: stateless primitives (`aspect-ratio`, `center`, `grid`, `stack`, `frame`), then `portal`, then disclosure and focus composites (`collapsible`, `toolbar`), then measurement-heavy widgets (`scroll-area`, `splitter`, `carousel`).

### Final Section Structure

Every layout adapter component spec in this tree uses this final section order:

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
- `Reference Implementation Skeleton` is required for every stateful or behavior-heavy layout component and must be tighter than the canonical sketch.

### Shared Terminology

- `measurement frame`: one adapter-owned cycle of geometry reads, machine updates, and DOM writes triggered by resize, scroll, or pointer movement.
- `viewport node`: the scrolling or clipping node whose live handle is required after mount for measurement or imperative platform work.
- `structural root`: the node whose identity must remain hydration-stable even when children, measurements, or teleported content change.
- `portal owner`: the logical component instance that owns a teleported mount node and its cleanup.
- `registration`: adapter-owned bookkeeping for descendant items, handles, observers, timers, or mount points.
- `semantic repair`: adapter-owned roles, aria attrs, labels, IDs, or inertness needed so the rendered host satisfies the agnostic contract.
- `fallback path`: a documented degraded rendering or behavior path used when a browser capability is absent during SSR or before mount.
- `verification recipe`: the cheapest reliable test-oracle path for failure-prone adapter behavior.

### Cross-Category Consumption Notes

- Overlay adapters should consume `portal`, `dismissable`, `focus-scope`, `ars-provider`, and `z-index-allocator` instead of redefining teleport, outside-interaction, focus trap, or layer allocation behavior.
- Navigation adapters should consume `collapsible` and `toolbar` when their semantics match the disclosure or roving-tabindex contracts.
- Layout widgets with polymorphic triggers or controls should consume shared utility helpers such as `button`, `as-child`, and `group` instead of inventing duplicate merge rules.
- Measurement-heavy layout widgets should share observer and geometry helpers where possible rather than duplicating scroll or resize normalization logic.

### Authoring Lint Checklist

- [ ] No adapter-owned layout behavior remains only in the framework-agnostic spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful SSR or measurement variance has a platform-matrix row instead of prose alone.
- [ ] High-risk layout widgets include concise invariants and a cheap verification recipe.
- [ ] Diagnostics say whether they are debug-only, fail-fast, warning-only, or graceful-degradation paths.

## Layout Index

- [AspectRatio](aspect-ratio.md)
- [Carousel](carousel.md)
- [Center](center.md)
- [Collapsible](collapsible.md)
- [Frame](frame.md)
- [Grid](grid.md)
- [Portal](portal.md)
- [ScrollArea](scroll-area.md)
- [Splitter](splitter.md)
- [Stack](stack.md)
- [Toolbar](toolbar.md)
