---
adapter: dioxus
component: radio-group
category: input
source: components/input/radio-group.md
source_foundation: foundation/09-adapter-dioxus.md
---

# RadioGroup — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`RadioGroup`](../../components/input/radio-group.md) contract onto a Dioxus 0.7.x component. The adapter must preserve roving tabindex, group-level validation, repeated item anatomy, and per-item hidden-input form submission.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct RadioGroupProps {
    #[props(optional)]
    pub value: Option<String>,
    #[props(optional)]
    pub default_value: Option<String>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(optional)]
    pub orientation: Option<Orientation>,
    #[props(default = false)]
    pub loop_focus: bool,
    #[props(optional)]
    pub name: Option<String>,
    pub children: Element,
}

#[component]
pub fn RadioGroup(props: RadioGroupProps) -> Element
```

The adapter also forwards shared group props from the core contract, including locale or messages and directionality. Item content is consumer-owned, but item-control semantics remain adapter-owned.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core radio-group contract, including orientation, loop-focus, and form naming.
- Event parity: `SelectValue`, focus-navigation events, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<radio_group::Machine>(...)` owns the selected value, roving focus order, and per-item checked state.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership                                         | Attr source                          | Notes                                   |
| --------------------- | --------- | ------------------------ | ------------------------------------------------- | ------------------------------------ | --------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned                                     | `api.root_attrs()`                   | owns `role="radiogroup"` semantics      |
| `Label`               | required  | `<label>`                | adapter-owned                                     | `api.label_attrs()`                  | group label                             |
| `Item`                | repeated  | `<div>`                  | adapter-owned wrapper or documented helper output | item attrs from API                  | each item is keyed by value             |
| `ItemControl`         | repeated  | focusable `<div>`        | adapter-owned                                     | `api.item_control_attrs(value)`      | owns `role="radio"` and roving tabindex |
| `ItemIndicator`       | repeated  | `<div>`                  | adapter-owned                                     | `api.item_indicator_attrs(value)`    | decorative only                         |
| `ItemLabel`           | repeated  | `<label>`                | adapter-owned or shared                           | `api.item_label_attrs(value)`        | linked to `ItemControl`                 |
| `ItemHiddenInput`     | repeated  | `<input type="radio">`   | adapter-owned                                     | `api.item_hidden_input_attrs(value)` | native submission bridge                |
| `Description`         | optional  | `<div>`                  | adapter-owned                                     | `api.description_attrs()`            | described-by content                    |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned                                     | `api.error_message_attrs()`          | invalid-only content                    |

## 5. Attr Merge and Ownership Rules

- Group-level ARIA, required, invalid, and orientation attrs on `Root` always win.
- Per-item `role`, `aria-checked`, `tabindex`, `name`, and `checked` semantics on `ItemControl` or `ItemHiddenInput` always win.
- Consumer decoration may add classes or content, but it must not break item identity by value.

## 6. Composition / Context Contract

`RadioGroup` is a compound input. The group publishes context for repeated items, and each item must receive a stable item value so roving focus, selection, and hidden-input submission remain aligned.

## 7. Prop Sync and Event Mapping

| Adapter prop                  | Mode          | Sync trigger                        | Machine event / update path   | Visible effect                                |
| ----------------------------- | ------------- | ----------------------------------- | ----------------------------- | --------------------------------------------- |
| `value`                       | controlled    | prop change                         | `SetValue`                    | updates selected item and roving focus target |
| `disabled` / `readonly`       | controlled    | prop change                         | `SetDisabled` / `SetReadonly` | guards group and item selection               |
| `required` / `invalid`        | controlled    | prop change                         | `SetRequired` / `SetInvalid`  | updates root-level validation attrs           |
| item activation or navigation | machine-owned | click, `Space`, `Enter`, arrow keys | `SelectValue` / focus events  | updates selected value and roving tabindex    |

## 8. Registration and Cleanup Contract

- Repeated items must register in DOM order or a stable adapter-maintained order consistent with the rendered value list.
- Removing an item must cleanly remove it from roving focus and hidden-input submission.
- No global listeners are required.

## 9. Ref and Node Contract

- `Root` may own a group-level ref for focus-entry logic.
- Each `ItemControl` owns its own live ref for roving focus.
- Hidden inputs remain non-focusable and must never become the roving target.

## 10. State Machine Boundary Rules

- The machine owns selected value, roving focus, and focus wrapping rules.
- The adapter may translate arrow keys based on directionality, but it must not compute checked state outside the machine.
- Hidden inputs reflect the machine-selected item only.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed selected item value.
- Focus-navigation callbacks, if exposed by wrappers, reflect the machine-chosen target item.
- Validation callbacks observe root-level invalid state after described-by wiring settles.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                   |
| ------------------------------------------------------------- | ------------------ | ------------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                         |
| repeated items are missing stable values                      | fail fast          | roving focus and submission depend on stable identity   |
| platform focus repair is unavailable                          | degrade gracefully | selection still updates even if focus repair is reduced |

## 13. Identity and Key Policy

The group instance owns one `Root` identity and one selected-value identity. Each item must be keyed stably by its item value so roving focus and hidden-input state remain aligned across reorders.

## 14. SSR and Client Boundary Rules

- SSR renders the same group and repeated item structure implied by the initial value.
- Client-only focus management activates after hydration, but item order and checked markup must remain stable.
- Hidden-input count and order must match between server and client.

## 15. Performance Constraints

- Reuse stable per-item identity so roving focus bookkeeping does not churn on unrelated renders.
- Do not rebuild hidden-input arrays from scratch when only selected value changes unless the framework requires it.
- Keep arrow-key translation and focus repair instance-local.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type     | Why it must exist first                           |
| ------------------- | --------- | ------------------- | ------------------------------------------------- |
| roving-focus helper | required  | adapter helper      | repeated radio items need stable focus navigation |
| hidden-input helper | required  | submission contract | selected item must submit natively                |

## 17. Recommended Implementation Sequence

1. Initialize the group machine and derive root-level attrs.
2. Establish repeated item rendering keyed by item value.
3. Render item controls, indicators, labels, and hidden inputs in documented order.
4. Wire controlled prop synchronization and roving-focus key handling.
5. Finish validation and directionality checks.

## 18. Anti-Patterns

- Do not use DOM index as the long-term item identity when value identity exists.
- Do not let hidden inputs become the keyboard navigation target.
- Do not bypass the machine for arrow-key selection changes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume item identity and checked state are keyed by item value.
- Consumers may assume `Root` owns group-level validation and orientation semantics.
- Consumers must not assume undocumented item wrapper removal or alternative focus targets.

## 20. Platform Support Matrix

| Capability / behavior                                                      | Web          | Desktop      | Mobile       | SSR          | Notes                                  |
| -------------------------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | -------------------------------------- |
| documented radiogroup semantics, roving focus, and hidden-input submission | full support | full support | full support | full support | focus repair activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                |
| ----------------------------------------------- | -------------------- | ------------------- | ------------------------------------ |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode               |
| item rendered without stable value identity     | fail fast            | fail fast           | protects roving focus and submission |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                            | Notes                                |
| ------------------- | --------- | --------------------------------------------------------- | ------------------------------------ |
| roving-focus helper | required  | track focusable item order and next-target resolution     | shared with other composite controls |
| hidden-input helper | required  | keep selected radio submission aligned with machine state | shared with checkbox-like controls   |

## 23. Framework-Specific Behavior

Dioxus should keep item refs in a stable value-keyed structure, translate RTL-aware arrow keys before dispatching machine events, and use local focus repair only when the next item control ref is available.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<radio_group::Machine>(props);

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        label { ..machine.derive(|api| api.label_attrs()).read().clone(), {children} }
        for value in item_values {
            div { ..machine.derive(|api| api.item_control_attrs(&value)).read().clone() }
        }
    }
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine and set up value-keyed repeated item bookkeeping.
- Keep roving focus, selected state, and hidden-input submission driven by the same item-value identity.

## 26. Adapter Invariants

- Selected value always lives in the group machine.
- Roving focus always targets `ItemControl`, never hidden inputs.
- Item identity is always keyed by item value.
- Root-level validation and described-by semantics always stay on `Root`.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for group-level `aria-describedby`.
- Horizontal RTL groups must reverse left or right navigation semantics without changing DOM order.
- SSR must preserve initial checked item and item ordering.

## 28. Parity Summary and Intentional Deviations

- Matches the core radio-group contract without intentional divergence.
- Promotes value-keyed item identity, roving focus ownership, and hidden-input submission rules into Dioxus-facing guidance.

## 29. Test Scenarios

- Arrow-key navigation updates roving focus and selected value correctly, including RTL behavior.
- Controlled value updates synchronize selected state and hidden-input submission without DOM-order drift.
- Required and invalid state update group-level validation attrs and described-by ordering correctly.
- Reordering item values with stable keys preserves focus and selection identity.

## 30. Test Oracle Notes

- Inspect `role="radiogroup"` and `role="radio"` attrs plus hidden-input checked state in the DOM.
- Use focus assertions on `ItemControl` nodes rather than hidden inputs.
- Verify selection changes from machine-driven callback logs and not from raw DOM index assumptions.

## 31. Implementation Checklist

- [ ] Keep selected value and roving focus machine-owned.
- [ ] Key repeated items by stable item value.
- [ ] Keep `ItemControl` as the only focusable radio target.
- [ ] Keep hidden-input submission aligned with selected item value.
- [ ] Translate direction-aware arrow keys before dispatching machine events.
- [ ] Preserve root-level validation and described-by semantics.
