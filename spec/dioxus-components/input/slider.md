---
adapter: dioxus
component: slider
category: input
source: components/input/slider.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Slider — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Slider`](../../components/input/slider.md) contract onto a Dioxus 0.7.x component. The adapter must preserve measurement-driven pointer interaction, hidden-input submission, value formatting, and optional output, markers, and dragging-indicator parts.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct SliderProps {
    #[props(optional)]
    pub value: Option<f64>,
    #[props(optional)]
    pub default_value: Option<f64>,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub orientation: Orientation,
    pub thumb_alignment: slider::ThumbAlignment,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub form: Option<String>,
    pub children: Element,
}

#[component]
pub fn Slider(props: SliderProps) -> Element
```

The adapter also forwards locale, messages, mark definitions, output formatting, and shared field props from the core contract. Value, bounds, and disabled state are the main reactive surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core slider contract, including markers, dragging indicator, orientation, and localized value text.
- Event parity: pointer, keyboard, focus, blur, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<slider::Machine>(...)` owns value, active-dragging state, and all thumb or range positioning rules.

## 4. Part Mapping

| Core part / structure          | Required? | Adapter rendering target | Ownership     | Attr source                      | Notes                         |
| ------------------------------ | --------- | ------------------------ | ------------- | -------------------------------- | ----------------------------- |
| `Root`                         | required  | `<div>`                  | adapter-owned | `api.root_attrs()`               | structural wrapper            |
| `Label`                        | required  | `<label>`                | adapter-owned | `api.label_attrs()`              | linked to `Thumb` semantics   |
| `Track`                        | required  | `<div>`                  | adapter-owned | `api.track_attrs()`              | pointer measurement target    |
| `Range`                        | required  | `<div>`                  | adapter-owned | `api.range_attrs()`              | filled segment                |
| `Thumb`                        | required  | focusable `<div>`        | adapter-owned | `api.thumb_attrs()`              | owns `role="slider"`          |
| `Output`                       | optional  | `<output>`               | adapter-owned | `api.output_attrs()`             | formatted value display       |
| `MarkerGroup` / `Marker`       | optional  | `<div>` / `<span>`       | adapter-owned | marker attrs                     | repeated presentational marks |
| `HiddenInput`                  | required  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()`       | native submission bridge      |
| `DraggingIndicator`            | optional  | `<div>`                  | adapter-owned | `api.dragging_indicator_attrs()` | decorative drag state         |
| `Description` / `ErrorMessage` | optional  | `<div>`                  | adapter-owned | description or error attrs       | field status content          |

## 5. Attr Merge and Ownership Rules

- Slider `role`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`, and hidden-input submission attrs always win.
- `class` and `style` merge additively on `Track`, `Range`, `Thumb`, and optional markers.
- Consumers may decorate marker labels or output text, but they must not take ownership of thumb measurement or drag state.

## 6. Composition / Context Contract

`Slider` consumes field, fieldset, and optional form context. Marker and output rendering are adapter-owned optional parts; they do not imply a separate collection or overlay contract.

## 7. Prop Sync and Event Mapping

| Adapter prop                    | Mode          | Sync trigger          | Machine event / update path  | Visible effect                                          |
| ------------------------------- | ------------- | --------------------- | ---------------------------- | ------------------------------------------------------- |
| `value`                         | controlled    | prop change           | `SetValue`                   | updates thumb position, range, output, and hidden input |
| `min` / `max` / `step`          | controlled    | prop change           | constraint updates           | updates keyboard, pointer, and formatting behavior      |
| `disabled` / `invalid`          | controlled    | prop change           | `SetDisabled` / `SetInvalid` | updates guards and validation attrs                     |
| pointer or keyboard interaction | machine-owned | track or thumb events | slider events                | updates value and dragging state                        |

Pointer math uses adapter-owned track measurement but must dispatch only machine events or machine-derived helpers. Hidden-input value always mirrors the committed machine value.

## 8. Registration and Cleanup Contract

- Track-measurement listeners and any scroll or resize observers are adapter-owned and must be removed on unmount.
- No descendant registry beyond optional marker rendering is required.
- Dragging-state helpers are local to the instance and must not leak listeners globally.

## 9. Ref and Node Contract

- `Track` and `Thumb` own live refs needed for measurement and focus.
- `Output` and markers do not own interactive refs.
- Hidden input remains non-focusable and must never replace `Thumb` as the focus target.

## 10. State Machine Boundary Rules

- The machine owns the committed slider value, dragging state, and keyboard semantics.
- The adapter may measure pointer position and translate it into a normalized value, but it must not commit the value outside the machine.
- Localized `aria-valuetext` and visible output derive from the machine-owned value.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed numeric value after snapping and clamping.
- Dragging-state callbacks, if exposed, emit machine-owned dragging transitions only.
- Output text, if exposed to callbacks, must reflect the same formatted value used for `aria-valuetext`.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                             |
| ------------------------------------------------------------- | ------------------ | ------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                   |
| track measurement is unavailable                              | degrade gracefully | keyboard interaction still works                  |
| client-only scroll or resize helpers cannot attach            | degrade gracefully | value still updates from direct thumb interaction |

## 13. Identity and Key Policy

`Root`, `Track`, `Range`, `Thumb`, hidden input, and optional markers belong to one slider instance. Marker identity should be keyed by mark value.

## 14. SSR and Client Boundary Rules

- SSR renders the same structural parts and initial value-derived attrs as the client.
- Track measurement, dragging, and scroll or resize observation are client-only.
- Hidden-input count and order must match between server and client.

## 15. Performance Constraints

- Measure track geometry only when necessary, not on every render.
- Keep marker rendering stable and keyed by mark value.
- Avoid duplicating value state outside the machine.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type     | Why it must exist first                              |
| ------------------- | --------- | ------------------- | ---------------------------------------------------- |
| measurement helper  | required  | adapter helper      | pointer position must map reliably onto slider value |
| hidden-input helper | required  | submission contract | slider value must submit natively                    |

## 17. Recommended Implementation Sequence

1. Initialize the machine and derive attrs for all structural parts.
2. Establish track and thumb refs plus client-only measurement hooks.
3. Render `Root`, `Label`, `Track`, `Range`, `Thumb`, hidden input, and optional parts in stable order.
4. Wire controlled prop synchronization and pointer, keyboard, focus, or blur events.
5. Add output formatting, markers, and diagnostics.

## 18. Anti-Patterns

- Do not let the hidden input or markers become the interactive target.
- Do not commit pointer-derived values directly without the machine.
- Do not leave scroll or resize listeners attached after unmount.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Thumb` remains the only interactive slider node.
- Consumers may assume hidden-input submission follows the committed machine value.
- Consumers must not assume undocumented marker ownership or alternate drag geometry rules.

## 20. Platform Support Matrix

| Capability / behavior                                                         | Web          | Desktop      | Mobile       | SSR          | Notes                                             |
| ----------------------------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------------------- |
| documented slider semantics, pointer interaction, and hidden-input submission | full support | full support | full support | full support | measurement and dragging activate after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                             | Debug build behavior | Production behavior | Notes                         |
| ----------------------------------------------------- | -------------------- | ------------------- | ----------------------------- |
| controlled/uncontrolled mode switch after mount       | debug warning        | warn and ignore     | preserves current mode        |
| track measurement unavailable for pointer interaction | debug warning        | degrade gracefully  | keyboard path stays available |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                                  | Notes                                       |
| ------------------- | --------- | --------------------------------------------------------------- | ------------------------------------------- |
| measurement helper  | required  | normalize pointer position to value and maintain rect freshness | shared with range sliders and color sliders |
| hidden-input helper | required  | derive submitted value from machine state                       | shared with other synthetic-form bridges    |

## 23. Framework-Specific Behavior

Dioxus should use mounted-node measurement on hosts that support it, isolate host-specific geometry quirks behind local helpers, and ensure any scroll or resize observers are cleaned up with the instance lifecycle.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<slider::Machine>(props);

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        div { ..machine.derive(|api| api.track_attrs()).read().clone(),
            div { ..machine.derive(|api| api.range_attrs()).read().clone() }
            div { ..machine.derive(|api| api.thumb_attrs()).read().clone() }
        }
        input { ..machine.derive(|api| api.hidden_input_attrs()).read().clone() }
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine, then allocate refs for `Track` and `Thumb`.
- Attach client-only measurement upkeep to the track lifecycle and route pointer or keyboard interaction through machine events only.
- Keep output text, marker state, and hidden-input submission derived from the same committed value.

## 26. Adapter Invariants

- `Thumb` always remains the only focusable slider node.
- Hidden-input submission always reflects the committed machine value.
- Pointer measurement is adapter-owned, but value commitment is machine-owned.
- Measurement listeners are always cleaned up on unmount.

## 27. Accessibility and SSR Notes

- `aria-valuetext` must stay aligned with visible output text when output is rendered.
- Description-first, error-second ordering is mandatory for `aria-describedby`.
- SSR must preserve initial slider value, hidden-input value, and mark ordering.

## 28. Parity Summary and Intentional Deviations

- Matches the core slider contract without intentional divergence.
- Promotes measurement ownership, hidden-input submission, and listener cleanup into Dioxus-facing guidance.

## 29. Test Scenarios

- Pointer dragging updates value, range fill, output text, and hidden input together.
- Arrow keys and page-step keys update the value correctly, including RTL behavior.
- Scroll or resize invalidates geometry without leaking listeners.
- Controlled value updates synchronize thumb position and formatted output without identity drift.

## 30. Test Oracle Notes

- Inspect `role="slider"`, `aria-valuenow`, `aria-valuetext`, and hidden-input value in the DOM.
- Use synthetic geometry plus pointer events to verify measurement-to-value translation.
- Assert listener cleanup with lifecycle or teardown checks rather than only visual behavior.

## 31. Implementation Checklist

- [ ] Keep `Thumb` as the only interactive slider node.
- [ ] Keep hidden-input submission derived from committed machine value.
- [ ] Route pointer, keyboard, and focus behavior through machine events only.
- [ ] Attach and remove measurement listeners with the track lifecycle.
- [ ] Preserve marker identity by mark value.
- [ ] Keep formatted output and `aria-valuetext` aligned.
