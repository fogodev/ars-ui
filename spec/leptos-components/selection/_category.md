---
adapter: leptos
category: selection
source_foundation: foundation/08-adapter-leptos.md
---

# Selection Components — Leptos Adapter

These documents map the framework-agnostic selection contracts in `spec/components/selection/*` onto Leptos 0.8.x APIs.

## Scope

- Core behavior, state machines, anatomy, accessibility, internationalization, and parity notes remain defined by `spec/components/selection/*`.
- These adapter specs define framework-facing APIs, compound composition, descendant registration, form bridging, popup positioning, event normalization, focus strategy, cleanup ordering, and SSR or hydration behavior.
- Selection adapter specs must explicitly restate adapter-owned behavior such as valid `aria-activedescendant` handling, roving-tabindex fallback, modifier-key normalization, typeahead timeout cleanup, hidden-input ownership, and pointer or keyboard popup wiring rather than leaving those obligations only in the agnostic specs.
- Canonical implementation sketches are illustrative only. Sections 1-31, `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist` are normative.

## Conventions

- Use `Signal<T>` or `Option<Signal<T>>` only for props that must react after mount; default-only values are read once during machine initialization.
- Compound selection components use `provide_context` and `use_context::<T>()`; required child parts must `expect(...)` a parent that names the missing root.
- Popup-based selection components should compose existing positioning and dismissal helpers rather than hiding those requirements in prose.
- List-like controls must document whether they use `aria-activedescendant`, roving tabindex, or a documented platform fallback, and they must explain how virtualization affects focus repair.
- Text-entry selection controls must document IME composition suppression, callback timing, and any live-region behavior explicitly.
- Form-participating selection controls must make hidden-input or native form ownership explicit, including reset semantics and described-by wiring.
- `Framework-Specific Behavior` is reserved for true Leptos divergence. Shared adapter obligations belong in the main contract sections.
- High-risk adapter-only behavior should be restated in `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist`.
- Per-component specs remain the primary source of truth when they overlap with this category guide.

### Final Section Structure

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

- Platform support matrices use these exact support labels: `full support`, `fallback path`, `client-only`, `SSR-safe empty`, and `not applicable`.
- Debug diagnostics tables use these exact behavior labels: `debug warning`, `fail fast`, `warn and ignore`, `degrade gracefully`, and `no-op`.
- `Consumer Expectations and Guarantees` must distinguish `Consumers may assume ...` from `Consumers must not assume ...`.
- `Reference Implementation Skeleton` is required for these selection components because every component in this category is stateful or behavior-heavy.
- | Capability / behavior | Browser client | SSR | Notes |

### Shared Terminology

- `interactive node`: the element that owns keyboard focus and the primary ARIA role.
- `structural node`: a rendered node whose presence, order, or identity is part of the adapter contract.
- `live handle`: a runtime node reference that becomes meaningful only after mount.
- `hidden-input bridge`: an adapter-owned native form control kept in sync with machine state for submission and reset semantics.
- `selection registry`: the keyed registration layer that maps rendered items or parts back to the core collection contract.
- `focus strategy fallback`: the documented switch between active-descendant focus and direct DOM focus when a platform cannot reliably announce the default strategy.

### Helper Taxonomy

- registration helpers: keyed item, group, and descendant bookkeeping for selection, typeahead, and focus repair.
- positioning helpers: popup placement, point anchoring, sizing vars, and dismissal boundaries for popup-based controls.
- form helpers: hidden inputs, reset listeners, described-by composition, and disabled or invalid propagation.
- typeahead helpers: timeout ownership, buffer cleanup, and IME-aware suppression rules.
- measurement helpers: virtualization-aware scroll repair, moving-indicator geometry, and fallback focus positioning.
- announcement helpers: optional live-region behavior for result counts, removals, and selection-related status updates.

### Cross-Category Consumption Notes

- Popup-based controls such as `Select`, `Combobox`, `Menu`, `ContextMenu`, and `MenuBar` should consume shared popup positioning and dismissal contracts rather than restating them ad hoc.
- Form-bound controls such as `Select`, `SegmentGroup`, and `TagsInput` should consume field, fieldset, and form utilities rather than redefining error, description, and reset behavior.
- Selection controls that expose text entry must align with shared IME rules from the text-entry adapter specs.
- Selection controls that expose load-more or virtualization behavior should reuse the same keyed registration and focus-repair language used by other collection-based adapters.

### Authoring Lint Checklist

- [ ] No adapter-owned selection behavior remains only in the agnostic selection spec.
- [ ] Valid `aria-activedescendant` behavior or its documented fallback is explicit for every relevant component.
- [ ] Modifier-key selection behavior and typeahead timeout cleanup are documented explicitly where relevant.
- [ ] Hidden-input or native form ownership is explicit for every form-participating selection control.
- [ ] Every popup-based selection control documents positioning, dismissal, and focus-return ownership.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] High-risk behaviors appear in `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist`.

## Selection Index

- [Autocomplete](autocomplete.md)
- [Combobox](combobox.md)
- [TagsInput](tags-input.md)
- [Select](select.md)
- [Listbox](listbox.md)
- [SegmentGroup](segment-group.md)
- [Menu](menu.md)
- [ContextMenu](context-menu.md)
- [MenuBar](menu-bar.md)
