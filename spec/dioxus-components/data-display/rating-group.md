---
adapter: dioxus
component: rating-group
category: data-display
source: components/data-display/rating-group.md
source_foundation: foundation/09-adapter-dioxus.md
---

# RatingGroup — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`RatingGroup`](../../components/data-display/rating-group.md) contract onto a Dioxus 0.7.x component. The adapter must preserve the hover/focus/value machine, choose the correct accessibility pattern for whole vs fractional ratings, and keep hidden-input form participation explicit.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct RatingGroupProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<f64>>,
    #[props(optional)]
    pub default_value: Option<f64>,
    pub count: usize,
    #[props(default = false)]
    pub allow_half: bool,
    #[props(optional)]
    pub step: Option<f64>,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<rating_group::Messages>,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<f64>>,
    #[props(optional)]
    pub on_hover_change: Option<EventHandler<Option<f64>>>,
    pub label: Element,
}

#[component]
pub fn RatingGroup(props: RatingGroupProps) -> Element
```

The adapter renders the repeated rating items internally. Fractional ratings switch the control semantics from radiogroup to slider.

## 3. Mapping to Core Component Contract

- Props parity: full parity with controlled/uncontrolled value semantics.
- Part parity: full parity for `Root`, `Label`, `Control`, repeated `Item`, and `HiddenInput`.
- Traceability note: this spec promotes slider-vs-radiogroup semantics, hover callbacks, hidden-input form bridging, and keyboard normalization from the agnostic spec.

## 4. Part Mapping

| Core part     | Required? | Adapter rendering target         | Ownership     | Attr source                | Notes                                           |
| ------------- | --------- | -------------------------------- | ------------- | -------------------------- | ----------------------------------------------- |
| `Root`        | required  | `<div>`                          | adapter-owned | `api.root_attrs()`         | Carries disabled and readonly state attrs.      |
| `Label`       | optional  | `<label>` or `<span>`            | adapter-owned | `api.label_attrs()`        | Associates with `Control`.                      |
| `Control`     | required  | `<div>`                          | adapter-owned | `api.control_attrs()`      | `radiogroup` or `slider` depending on stepping. |
| `Item`        | repeated  | `<span>` or `<button>`-like host | adapter-owned | `api.item_attrs(index)`    | Repeated interactive display items.             |
| `HiddenInput` | optional  | `<input type="hidden">`          | adapter-owned | `api.hidden_input_attrs()` | Enables form submission.                        |

## 5. Attr Merge and Ownership Rules

- Core attrs include selection, highlighted, index, role, `aria-checked`, `aria-valuetext`, and disabled or readonly attrs.
- The adapter owns the accessibility pattern selection: radio items for whole-number ratings, single slider-like control for fractional steps.
- Consumer decoration must not override roving tabindex, checked state, or hidden-input value.

## 6. Composition / Context Contract

`RatingGroup` is standalone. It does not require descendant context because the adapter renders the repeated items from `count`.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode          | Sync trigger              | Machine event / update path                                       | Notes                                              |
| -------------------- | ------------- | ------------------------- | ----------------------------------------------------------------- | -------------------------------------------------- |
| `value`              | controlled    | signal change after mount | `Rate`                                                            | The adapter clamps and rounds through the machine. |
| pointer hover        | adapter event | item enter/leave          | `HoverItem` / `UnHover`                                           | Drives preview state and `on_hover_change`.        |
| focus and blur       | adapter event | item or control focus     | `Focus` / `Blur`                                                  | Focus-visible tracking stays machine-owned.        |
| arrow/home/end keys  | adapter event | keyboard interaction      | `IncrementRating`, `DecrementRating`, or direct focus/rate events | Pattern depends on radio vs slider mode.           |

## 8. Registration and Cleanup Contract

No external descendant registration is required. Cleanup is limited to controlled-value watchers and any transient hover state listeners on the repeated items.

## 9. Ref and Node Contract

The control node may need a root ref when using slider semantics for keyboard focus or focus restoration. Item refs are optional and only needed for richer roving-focus implementations.

## 10. State Machine Boundary Rules

- Machine-owned state: committed value, hovered value, focused index, and focus-visible state.
- Adapter-owned derived values: rendered icon fill state and radio-vs-slider semantics.
- Forbidden mirror: do not store a second selected rating outside the machine.

## 11. Callback Payload Contract

| Callback          | Payload source                      | Payload shape | Timing                          | Cancelable? | Notes                           |
| ----------------- | ----------------------------------- | ------------- | ------------------------------- | ----------- | ------------------------------- |
| `on_value_change` | adapter observation                 | `f64`         | after `Rate` updates context    | no          | Uses clamped and rounded value. |
| `on_hover_change` | adapter event and state observation | `Option<f64>` | after hover enter/leave updates | no          | `None` means preview cleared.   |

## 12. Failure and Degradation Rules

| Condition                                          | Policy             | Notes                                                      |
| -------------------------------------------------- | ------------------ | ---------------------------------------------------------- |
| invalid `step` or `count`                          | fail fast          | The adapter requires a positive count and meaningful step. |
| readonly or disabled interactive events            | no-op              | Focus for AT may remain, but value changes are suppressed. |
| hidden input omitted because no `name` is supplied | degrade gracefully | Form participation becomes opt-in.                         |

## 13. Identity and Key Policy

Each item uses its stable index as identity. The number of items must remain hydration-stable for a given `count`.

## 14. SSR and Client Boundary Rules

- SSR renders the initial value, selected items, and semantic branch.
- Controlled-value watchers and pointer hover behavior start after mount.
- The radio-vs-slider semantic branch must stay stable between server and client.

## 15. Performance Constraints

- Highlight and selection calculations should derive from machine state, not per-item local state.
- Avoid recreating per-item closures when only one value changes if the framework can memoize repeated item rendering.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type      | Why it must exist first                 | Notes                                         |
| ------------------- | --------- | -------------------- | --------------------------------------- | --------------------------------------------- |
| hidden-input helper | optional  | form-bridging helper | Provides consistent form participation. | Shared with other form-participating widgets. |

## 17. Recommended Implementation Sequence

1. Initialize the machine from value props.
2. Decide radio vs slider semantics.
3. Render root, control, items, and optional hidden input.
4. Wire hover, focus, and keyboard events.
5. Observe value and hover changes for callbacks.

## 18. Anti-Patterns

- Do not expose both radio and slider semantics at the same time.
- Do not rely on hover preview as committed state.
- Do not hide the hidden input when `name` is supplied.

## 19. Consumer Expectations and Guarantees

- Consumers may assume whole-number ratings follow a radio-group interaction model.
- Consumers may assume fractional ratings expose a slider-style value model.
- Consumers must not assume hover preview commits a value.

## 20. Platform Support Matrix

| Capability / behavior                 | Web          | Desktop      | Mobile       | SSR            | Notes                                       |
| ------------------------------------- | ------------ | ------------ | ------------ | -------------- | ------------------------------------------- |
| radio-group pattern                   | full support | full support | full support | full support   | Structure and attrs are server-safe.        |
| slider pattern for fractional ratings | full support | full support | full support | full support   | Semantic branch must stay hydration-stable. |
| hover preview callbacks               | client-only  | client-only  | client-only  | SSR-safe empty | Pointer hover is client-only.               |

## 21. Debug Diagnostics and Production Policy

| Condition                                           | Debug build behavior | Production behavior | Notes                                 |
| --------------------------------------------------- | -------------------- | ------------------- | ------------------------------------- |
| invalid `step` or impossible count                  | fail fast            | fail fast           | The component contract is incomplete. |
| consumer overrides roving tabindex or checked attrs | debug warning        | warn and ignore     | Adapter semantics win.                |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                       | Reused by          | Notes                                |
| ------------------- | --------- | ------------------------------------ | ------------------ | ------------------------------------ |
| hidden-input helper | optional  | Serialize committed value for forms. | selection controls | Keep name/value handling consistent. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should memoize machine-derived item state and keep one focusable control node in fractional slider mode rather than many independently tabbable items.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct RatingGroupSketchProps {
    pub count: usize,
}

#[component]
pub fn RatingGroup(props: RatingGroupSketchProps) -> Element {
    let machine = use_machine::<rating_group::Machine>(rating_group::Props {
        count: props.count.into(),
        ..Default::default()
    });
    let strategy = use_style_strategy();

    rsx! {
        div {
            ..attr_map_to_dioxus(machine.derive(|api| api.root_attrs())(), &strategy, None).attrs,
            div {
                ..attr_map_to_dioxus(machine.derive(|api| api.control_attrs())(), &strategy, None).attrs,
                for index in 0..props.count {
                    span { ..attr_map_to_dioxus(machine.derive(move |api| api.item_attrs(index))(), &strategy, None).attrs }
                }
            }
        }
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, one optional controlled-value watcher, and one callback observer for committed value plus hover preview. Radio and slider branches should share the same repeated item rendering where possible.

## 26. Adapter Invariants

- Accessibility pattern is chosen once per semantic mode.
- Hover preview never mutates committed value without a `Rate` event.
- Hidden input value always mirrors the committed machine value when present.

## 27. Accessibility and SSR Notes

- Whole-number mode uses the radio-group pattern.
- Fractional mode uses slider semantics with localized value text.
- Readonly items may remain focusable for AT, but mutation events stay suppressed.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter semantics for whole vs fractional ratings.
- Intentional deviations: none beyond adapter callback timing and internal repeated-item rendering.

## 29. Test Scenarios

1. Whole-number ratings expose radio semantics and update the committed value on click.
2. Fractional ratings expose slider semantics and update on arrow keys.
3. Hover preview highlights items without committing the underlying value.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect control and item roles plus ARIA attrs.
- Preferred oracle for callbacks: record value and hover callback order.
- Verification recipe: switch between readonly and interactive modes and confirm mutation events stop while focus semantics remain valid.

## 31. Implementation Checklist

- [ ] Whole vs fractional semantics are explicit.
- [ ] Controlled sync goes through `Rate`.
- [ ] Hover preview is separate from committed value.
- [ ] Hidden input mirrors the committed value when `name` exists.
- [ ] Tests cover radio mode, slider mode, and hover preview.
