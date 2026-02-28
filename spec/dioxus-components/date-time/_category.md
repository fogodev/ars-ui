---
adapter: dioxus
category: date-time
source_foundation: foundation/09-adapter-dioxus.md
---

# Date-Time Components — Dioxus Adapter

These documents map the framework-agnostic date-time contracts in `spec/components/date-time/*` onto Dioxus 0.7.x APIs.

## Scope

- Core behavior, state machines, accessibility rules, and ICU-driven formatting remain defined by the framework-agnostic date-time specs.
- These Dioxus adapter specs define the Dioxus-facing component API, `Element` usage, signal boundaries, host-platform fallbacks, hidden-input policy on web, and cleanup rules for mounted nodes and effects.
- The category is organized around three implementation families: segmented fields (`date-field`, `time-field`), calendar grids (`calendar`, `range-calendar`), and composite pickers (`date-picker`, `date-range-field`, `date-range-picker`, `date-time-picker`).
- Canonical examples are illustrative only; the implementation-facing sections in each component spec are normative.

## Conventions

- All component APIs use explicit `#[derive(Props, Clone, PartialEq)]` structs per foundation spec Section 4. Zero-prop parts may use bare function signatures.
- Use plain Dioxus props by default. Use `Signal<T>` only when post-mount watching is required by the adapter contract.
- Keep machine-owned state inside the core machine; adapter-local Dioxus state is limited to refs, timers, overlay handles, and host-specific fallback bookkeeping.
- Segmented fields remain structural wrappers around machine-owned segment bundles. They must not mirror segment values or focus state locally.
- Calendar grids own host semantics, ids, and range-preview attrs even on non-web hosts where some DOM semantics fall back to logical equivalents.
- Composite pickers should consume utility `button`, `dismissable`, `focus-scope`, `ars-provider`, and `z-index-allocator` contracts instead of redefining overlay behavior locally.
- High-risk adapter-only rules belong in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`; `Framework-Specific Behavior` is reserved for true Dioxus divergence.
- Per-component specs remain the primary source of truth when category guidance and a component contract overlap.

### Final Section Structure

Every date-time adapter component spec in this tree uses this final section order:

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
- `Consumer Expectations and Guarantees` distinguishes:
  `Consumers may assume ...` from `Consumers must not assume ...`.
- `Shared Adapter Helper Notes` describe reusable implementation infrastructure only; they do not define required public Dioxus symbols.

### Shared Terminology

- `segment bundle`: the ordered set of editable and literal date/time segments rendered from machine state.
- `grid bundle`: one visible month with its heading and rows.
- `form bridge`: the adapter-owned hidden-input synchronization used only on web hosts.
- `overlay shell`: the positioner, dismissable boundary, focus scope, and layer allocation wrapped around picker content.
- `host fallback`: the documented desktop/mobile behavior used when browser-native DOM semantics do not exist.

### Helper Taxonomy

- segment formatting and parsing helpers
- hidden-input helpers
- calendar grid helpers
- range-formatting helpers
- live-region helpers
- overlay and focus-dismiss helpers

### Cross-Category Consumption Notes

- `date-field`, `time-field`, and `date-range-field` should consume `field` and `form` on web and use documented host fallbacks elsewhere.
- `date-picker`, `date-range-picker`, and `date-time-picker` should consume `button`, `dismissable`, `focus-scope`, `ars-provider`, and `z-index-allocator` for overlay behavior.
- `calendar` and `range-calendar` should reuse shared calendar-grid helpers rather than duplicating heading, row, and cell logic.

### Authoring Lint Checklist

- [ ] No adapter-owned behavior remains only in the framework-agnostic date-time specs.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] Every fallback path is paired with a production behavior policy.
- [ ] Every meaningful platform variance has a platform-matrix row instead of prose alone.
- [ ] High-risk date-time components include concise invariants and a cheap verification recipe.
- [ ] Framework API examples stay aligned with supported Dioxus `0.7.x` patterns already used elsewhere in the adapter tree.

## Date-Time Index

- [Calendar](calendar.md)
- [DateField](date-field.md)
- [DatePicker](date-picker.md)
- [DateRangeField](date-range-field.md)
- [DateRangePicker](date-range-picker.md)
- [DateTimePicker](date-time-picker.md)
- [RangeCalendar](range-calendar.md)
- [TimeField](time-field.md)
