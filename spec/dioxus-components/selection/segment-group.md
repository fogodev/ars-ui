---
adapter: dioxus
component: segment-group
category: selection
source: components/selection/segment-group.md
source_foundation: foundation/09-adapter-dioxus.md
---

# SegmentGroup — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`SegmentGroup`](../../components/selection/segment-group.md) contract onto Dioxus 0.7.x. The adapter must preserve single-choice segmented control with hidden-input bridge and optional moving indicator while making radio-like keyed selection, hidden-input form participation, indicator measurement, and orientation-aware keyboard handling explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct SegmentGroupProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Key>>,
    #[props(optional)]
    pub default_value: Option<Key>,
    #[props(optional)]
    pub orientation: Option<Orientation>,
    #[props(optional, default = true)]
    pub loop_focus: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub form: Option<String>,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Key>>,
    pub children: Element,
}

#[component]
pub fn SegmentGroup(props: SegmentGroupProps) -> Element

/// Item component: a single segment choice.
#[derive(Props, Clone, PartialEq)]
pub struct ItemProps {
    pub key: Key,
    #[props(optional, default = false)]
    pub disabled: bool,
    pub children: Element,
}

#[component]
pub fn Item(props: ItemProps) -> Element

/// ItemLabel component: the visible label inside a segment item.
#[derive(Props, Clone, PartialEq)]
pub struct ItemLabelProps {
    pub children: Element,
}

#[component]
pub fn ItemLabel(props: ItemLabelProps) -> Element

/// ItemIndicator component: decorative state indicator inside a segment item.
#[component]
pub fn ItemIndicator() -> Element

/// Indicator component: the moving highlight host with adapter-owned measurement.
#[component]
pub fn Indicator() -> Element

/// HiddenInput component: bridges native form submission.
#[component]
pub fn HiddenInput() -> Element
```

Compound helpers typically include `Item`, `ItemLabel`, `ItemIndicator`, `Indicator`, and `HiddenInput` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with keyed single selection, orientation, readonly or invalid state, and form participation.
- Part parity: full parity for segments, per-item indicators, optional moving indicator, and hidden-input bridge.
- Traceability note: this spec promotes hidden-input behavior, orientation-aware key handling, indicator measurement, forced-colors fallback, and SSR measurement policy from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required?                       | Adapter rendering target   | Ownership     | Attr source                   | Notes                                        |
| --------------------- | ------------------------------- | -------------------------- | ------------- | ----------------------------- | -------------------------------------------- |
| Root                  | required                        | group host                 | adapter-owned | api.root_attrs()              | Owns the segmented control scope.            |
| Item                  | repeated                        | segment host               | adapter-owned | api.item_attrs(key)           | One per segment choice.                      |
| ItemIndicator         | optional repeated               | decorative state indicator | adapter-owned | api.item_indicator_attrs(key) | Can reflect selected state inside each item. |
| Indicator             | optional                        | moving highlight host      | adapter-owned | api.indicator_attrs()         | Uses adapter-owned measurement and CSS vars. |
| HiddenInput           | required when `name` is present | native hidden input        | adapter-owned | api.hidden_input_attrs()      | Bridges native form submission.              |

## 5. Attr Merge and Ownership Rules

- Core attrs win for radio-like checked state, disabled state, and orientation semantics on the root and items.
- The adapter owns hidden-input serialization and CSS custom properties for the moving indicator.
- Consumers may decorate segment content, but they must not replace the keyed segment hosts, hidden-input bridge, or moving-indicator ownership boundary.

## 6. Composition / Context Contract

The root publishes required segmented-control context to item, indicator, and hidden-input parts. The adapter consumes environment, field, and optional measurement helpers. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event  | Mode           | Sync trigger                       | Machine event / update path               | Notes                                                      |
| --------------------- | -------------- | ---------------------------------- | ----------------------------------------- | ---------------------------------------------------------- |
| `value`               | controlled     | signal change after mount          | value sync event                          | Updates checked item state and hidden-input serialization. |
| item activation       | adapter event  | click, Enter, Space, or arrow keys | `SelectItem` or equivalent                | Arrow-key behavior follows orientation and direction.      |
| indicator measurement | adapter effect | selection change or layout change  | update CSS custom properties              | Only for the moving indicator part.                        |
| form reset            | adapter bridge | native form reset                  | restore default value and indicator state | Applies only when the group participates in a form.        |

## 8. Registration and Cleanup Contract

- The adapter owns keyed item registration, hidden-input synchronization, and indicator measurement subscriptions.
- Measurement handles and resize observers must detach on unmount.
- Form-reset listeners must be cleanup-safe and instance-scoped.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                     | Notes                                                               |
| ------------------ | ------------- | ------------- | -------------------- | ------------------------------------ | ------------------------------------------------------------------- |
| Root               | yes           | adapter-owned | required after mount | compose only if explicitly forwarded | Needed for keyboard focus entry and fallback indicator measurement. |
| Item hosts         | recommended   | adapter-owned | required after mount | no composition by default            | Needed for moving-indicator measurement.                            |
| Indicator          | no            | adapter-owned | client-only          | no composition                       | Decorative node; measurement output is adapter-owned.               |

## 10. State Machine Boundary Rules

- Machine-owned state: selected key, disabled or readonly state, and keyboard focus progression.
- Adapter-local derived bookkeeping: hidden-input serialization, measured indicator geometry, and forced-colors fallback behavior.
- Forbidden local mirrors: do not keep a second selected key or indicator target separate from the committed machine state.
- Allowed snapshot reads: item activation handlers, measurement effects, reset listeners, and callback observation.

## 11. Callback Payload Contract

| Callback          | Payload source           | Payload shape | Timing                            | Cancelable? | Notes                                    |
| ----------------- | ------------------------ | ------------- | --------------------------------- | ----------- | ---------------------------------------- |
| `on_value_change` | machine-derived snapshot | `Key`         | after committed selection changes | no          | Fires after the checked item is updated. |

## 12. Failure and Degradation Rules

| Condition                                          | Policy             | Notes                                                                             |
| -------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------- |
| selected item missing from the registered item set | fail fast          | Indicator measurement and hidden-input serialization depend on a live keyed item. |
| indicator measurement unavailable                  | degrade gracefully | Keep checked-state styling without the moving-indicator animation.                |
| hidden-input bridge unavailable while form-bound   | degrade gracefully | Interactive selection still works, but native form submission falls back.         |

## 13. Identity and Key Policy

- Items are data-derived by `Key` and rendered order must match registration order.
- Hidden input and moving indicator are instance-derived structural nodes.
- Measurement handles are instance-derived and must be released on cleanup.

## 14. SSR and Client Boundary Rules

- SSR renders the root, keyed items, and hidden-input bridge when present.
- The moving indicator may render as `display:none` or another documented inert shell until measurement is available after mount.
- Hydration must preserve selected item order and hidden-input value.

## 15. Performance Constraints

- Do not measure every item on every render when only the selected key changes.
- Hidden-input serialization should update only when the committed selection changes.
- Resize or layout observers should stay instance-local and detach promptly.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type  | Why it must exist first                                              | Notes                                                                |
| ------------------- | ----------- | ---------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| hidden-input helper | required    | shared helper    | Form submission and reset semantics are adapter-owned.               | Shared with `select` and `tags-input`.                               |
| measurement helper  | recommended | shared helper    | Moving-indicator geometry should reuse an existing measurement path. | Fallback behavior is required when measurement is unavailable.       |
| direction helper    | required    | context contract | Arrow-key behavior depends on orientation and direction.             | Consume environment direction when no explicit prop override exists. |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render keyed item parts.
2. Wire item activation, keyboard handling, and value-change observation.
3. Attach hidden-input synchronization and optional moving-indicator measurement.
4. Add form reset behavior, forced-colors fallback, and cleanup checks.

## 18. Anti-Patterns

- Do not treat the moving indicator as the source of truth for selection state.
- Do not serialize an empty or stale value into the hidden input when no segment is selected.
- Do not require web-only measurement APIs on targets where only fallback styling is supported.

## 19. Consumer Expectations and Guarantees

- Consumers may assume keyed selection remains machine-owned and radio-like.
- Consumers may assume the hidden-input bridge handles native form participation when `name` is provided.
- Consumers must not assume the moving indicator is available on every target; documented fallback styling may apply.

## 20. Platform Support Matrix

| Capability / behavior                                           | Web           | Desktop        | Mobile         | SSR            | Notes                                                                           |
| --------------------------------------------------------------- | ------------- | -------------- | -------------- | -------------- | ------------------------------------------------------------------------------- |
| segment selection, hidden-input bridging, and keyboard handling | full support  | fallback path  | fallback path  | full support   | Non-web targets may need documented host-form fallbacks.                        |
| moving-indicator measurement                                    | full support  | fallback path  | fallback path  | SSR-safe empty | Desktop and mobile may omit the moving indicator and keep checked styling only. |
| forced-colors fallback styling                                  | fallback path | not applicable | not applicable | full support   | Use target-appropriate checked-state styling when system forced colors differ.  |

## 21. Debug Diagnostics and Production Policy

| Condition                                             | Debug build behavior | Production behavior | Notes                                                              |
| ----------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------------ |
| selected key not registered                           | fail fast            | fail fast           | The adapter cannot satisfy checked-state semantics or measurement. |
| indicator measurement unavailable on a non-web target | debug warning        | degrade gracefully  | Fallback styling remains valid.                                    |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required?   | Responsibility                                            | Reused by                 | Notes                                             |
| ------------------- | ----------- | --------------------------------------------------------- | ------------------------- | ------------------------------------------------- |
| hidden-input helper | required    | Serialize the committed selected key for form submission. | `select`, `tags-input`    | One shared bridge keeps form behavior consistent. |
| measurement helper  | recommended | Compute moving-indicator geometry and update CSS vars.    | `scroll-area`, `splitter` | Fallback behavior is mandatory.                   |
| direction helper    | required    | Resolve RTL-aware horizontal key semantics.               | `menu-bar`, `listbox`     | Consume environment direction when available.     |

## 23. Framework-Specific Behavior

Dioxus should make non-web moving-indicator fallback behavior explicit, keep host-form fallback semantics documented when native hidden inputs are not meaningful, and isolate measurement work to mounted nodes only.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct SegmentGroupProps { /* ... */ }

#[component]
pub fn SegmentGroup(props: SegmentGroupProps) -> Element {
    let machine = use_machine::<segment_group::Machine>(segment_group::Props { /* ... */ });

    rsx! {
        div {
            ..machine.derive(|api| api.root_attrs())(),
            {props.children}
            input { ..machine.derive(|api| api.hidden_input_attrs())() }
        }
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one hidden-input helper, and one optional measurement helper for the moving indicator. Selection callbacks and indicator updates always observe the committed selected key, never incidental DOM focus.

## 26. Adapter Invariants

- Committed selected key remains machine-owned and is the only source of truth for checked state.
- Hidden-input serialization reflects the committed selection only.
- The moving indicator is optional decoration with documented fallback behavior when measurement is unavailable.

## 27. Accessibility and SSR Notes

- The group must expose radio-like or segmented-control semantics that remain valid with or without the moving indicator.
- Forced-colors and high-contrast fallback styling must not rely on moving-indicator geometry.
- Description and error-message linkage should attach to the documented root or hidden-input participant only when those nodes are rendered.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of hidden-input bridging and indicator measurement.
- Intentional deviations: non-web targets may omit the moving indicator in favor of documented checked-state styling.

## 29. Test Scenarios

1. Arrow keys move focus and selection according to orientation and direction.
2. Changing the selected segment updates hidden-input serialization and callback timing.
3. The moving indicator updates after selection changes when measurement is available.
4. SSR and non-web fallback paths keep checked-state styling valid without live measurement.

## 30. Test Oracle Notes

- Preferred oracle for keyed selection: `machine state` plus item `DOM attrs` for checked and focus-visible state.
- Preferred oracle for form participation: hidden-input `DOM attrs` and reset behavior.
- Preferred oracle for the moving indicator: `rendered structure` and CSS custom-property assertions when measurement is available.

## 31. Implementation Checklist

- [ ] Hidden-input bridging, keyboard handling, and measurement fallback paths are explicit adapter contracts.
- [ ] The moving indicator is documented as decoration rather than state ownership.
- [ ] Non-web and SSR fallback behavior is covered by platform, invariant, and test sections.
