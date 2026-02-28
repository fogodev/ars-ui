---
adapter: dioxus
category: input
source_foundation: foundation/09-adapter-dioxus.md
---

# Input Components — Dioxus Adapter

These documents map the framework-agnostic input contracts in `spec/components/input/*` onto Dioxus 0.7.x APIs.

## Scope

- Core behavior, state machines, anatomy, accessibility, internationalization, and form semantics remain defined by the framework-agnostic input specs.
- These Dioxus adapter specs define the Dioxus-facing component API, `Element` and slot shape, controlled-prop synchronization, context publication and consumption, attr spreading, cleanup ownership, and platform-specific behavior.
- Input adapter specs must explicitly restate adapter-owned behavior such as hidden-input rendering, DOM listener ownership, roving focus wiring, controlled prop synchronization, and field integration rather than leaving those obligations only in the agnostic specs.
- Canonical implementation sketches are illustrative only. Contract sections, `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist` are normative.

## Conventions

- All component APIs use explicit `#[derive(Props, Clone, PartialEq)]` structs per foundation spec Section 4. Zero-prop parts may use bare function signatures.
- Use plain props by default; introduce `Signal<T>` only when a value must be observed after mount.
- Prefer `Element` or explicit slot props for visual composition. Context-only helpers must not render wrapper nodes unless the core contract requires one.
- Compound inputs use `use_context_provider` and `try_use_context::<T>()`; required parents must fail fast with a message that names the missing provider.
- Inputs that wrap a core machine use `use_machine::<...>()`, memoized `derive(...)` access, and targeted `use_effect` synchronization rather than ad hoc shadow state.
- Form-bound inputs must consume `field`, `fieldset`, and `form` contracts instead of re-specifying label IDs, described-by merging, or disabled propagation.
- Hidden-input behavior must be documented explicitly. Native `<input>` or `<textarea>` controls that submit directly must say so just as explicitly.
- Text-entry components must document composition-event handling, value synchronization timing, and keyboard event propagation when nested inside larger composites.
- Repeated structures such as radio items, pin cells, thumbs, or slider marks must call out identity, ordering, and cleanup expectations explicitly.
- `Framework-Specific Behavior` is reserved for true Dioxus divergence. Shared adapter obligations belong in the main contract sections.
- High-risk adapter-only behavior should be restated in `Adapter Invariants`, `Implementation Checklist`, and `Test Oracle Notes`.
- Per-component docs remain the primary source of truth when they overlap with this category guide.

### Final Section Structure

Every input adapter component spec in this tree uses this final section order:

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
- `Reference Implementation Skeleton` is required for stateful or behavior-heavy inputs and may explicitly say no expanded skeleton is needed only for simple semantic wrappers.

### Shared Terminology

- `interactive node`: the element that owns keyboard focus and the primary ARIA role.
- `structural node`: a rendered node whose presence, order, or identity is part of the adapter contract.
- `live handle`: a runtime node reference that becomes meaningful only after mount.
- `hidden-input bridge`: an adapter-owned native form control kept in sync with machine state for submission and reset semantics.
- `semantic repair`: adapter-owned attrs, roles, IDs, and handler normalization required to satisfy the core accessibility contract.
- `verification recipe`: the cheapest reliable way to prove a failure-prone behavior using DOM inspection, callback traces, or focus assertions.

### Helper Taxonomy

- merge helpers: additive `class` / `style` composition and required-attr precedence.
- field helpers: label, description, error-message, and disabled-context integration.
- hidden-input helpers: native form controls for checkbox-like, radio-like, slider, and pin-style submission.
- registration helpers: repeated item or repeated part mount/unmount bookkeeping.
- measurement helpers: track rect, pointer percent, auto-resize height, and scrub geometry.
- warning helpers: development-only diagnostics for invalid child count, missing context, or conflicting controlled props.
- composition helpers: IME state tracking and deferred callback emission.

### Cross-Category Consumption Notes

- Input components should consume `field`, `fieldset`, and `form` utilities rather than re-defining validation and described-by contracts.
- Inputs that need trigger polymorphism or button-like semantics should consume utility-layer `button`, `toggle`, or `toggle-button` behavior instead of duplicating press normalization.
- Complex selection or popup-enabled inputs should rely on selection or overlay specs for listbox, popup, and dismissal semantics rather than embedding them here.

### Authoring Lint Checklist

- [ ] No adapter-owned behavior remains only in the framework-agnostic input spec.
- [ ] Recommendation-level adapter guidance is restated explicitly rather than implied.
- [ ] Hidden-input ownership or direct-native-submission ownership is explicit.
- [ ] Composition-event handling is documented for every text-entry input.
- [ ] Every repeated structure defines identity, ordering, and cleanup semantics.
- [ ] Every new test scenario names at least one preferred oracle in `Test Oracle Notes`.
- [ ] High-risk behaviors appear in `Adapter Invariants`, `Test Oracle Notes`, and `Implementation Checklist`.

## Input Index

- [Checkbox](checkbox.md)
- [CheckboxGroup](checkbox-group.md)
- [Editable](editable.md)
- [FileTrigger](file-trigger.md)
- [NumberInput](number-input.md)
- [PasswordInput](password-input.md)
- [PinInput](pin-input.md)
- [RadioGroup](radio-group.md)
- [RangeSlider](range-slider.md)
- [SearchInput](search-input.md)
- [Slider](slider.md)
- [Switch](switch.md)
- [TextField](text-field.md)
- [Textarea](textarea.md)
