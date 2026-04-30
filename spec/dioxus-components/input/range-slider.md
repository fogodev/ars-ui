---
adapter: dioxus
component: range-slider
category: input
source: components/input/range-slider.md
source_foundation: foundation/09-adapter-dioxus.md
---

# RangeSlider — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`RangeSlider`](../../components/input/range-slider.md) contract onto a Dioxus 0.7.x component. The adapter must preserve two-thumb measurement and keyboard behavior, gap constraints, hidden-input submission for both values, and stable thumb identity.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct RangeSliderProps {
    #[props(optional)]
    pub value: Option<[f64; 2]>,
    pub default_value: [f64; 2],
    pub min: f64,
    pub max: f64,
    pub step: f64,
    #[props(optional)]
    pub min_gap: Option<f64>,
    pub orientation: Orientation,
    #[props(default = false)]
    pub start_disabled: bool,
    #[props(default = false)]
    pub end_disabled: bool,
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
pub fn RangeSlider(props: RangeSliderProps) -> Element
```

The adapter also forwards locale, messages, mark definitions, output formatting, thumb alignment, and shared field props from the core contract. The `[start, end]` value pair is the main reactive surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range-slider contract, including per-thumb disabled state, min-gap enforcement, output, markers, and dragging indicator.
- Event parity: per-thumb pointer, keyboard, focus, blur, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<range_slider::Machine>(...)` owns both thumb values, drag state, and crossing or clamping rules.

## 4. Part Mapping

| Core part / structure                           | Required? | Adapter rendering target | Ownership     | Attr source                      | Notes                             |
| ----------------------------------------------- | --------- | ------------------------ | ------------- | -------------------------------- | --------------------------------- |
| `Root`                                          | required  | `<div>`                  | adapter-owned | `api.root_attrs()`               | structural wrapper                |
| `Label`                                         | required  | `<label>`                | adapter-owned | `api.label_attrs()`              | group label                       |
| `Track`                                         | required  | `<div>`                  | adapter-owned | `api.track_attrs()`              | measurement target                |
| `Range`                                         | required  | `<div>`                  | adapter-owned | `api.range_attrs()`              | filled segment between thumbs     |
| `Thumb { Start }` / `Thumb { End }`             | required  | focusable `<div>`        | adapter-owned | `api.thumb_attrs(thumb)`         | each owns `role="slider"`         |
| `Output`                                        | optional  | `<output>`               | adapter-owned | `api.output_attrs()`             | formatted range summary           |
| `MarkerGroup` / `Marker`                        | optional  | `<div>` / `<span>`       | adapter-owned | marker attrs                     | repeated presentational marks     |
| `HiddenInput { Start }` / `HiddenInput { End }` | required  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs(thumb)`  | native submission for both values |
| `DraggingIndicator`                             | optional  | `<div>`                  | adapter-owned | `api.dragging_indicator_attrs()` | decorative drag state             |
| `Description` / `ErrorMessage`                  | optional  | `<div>`                  | adapter-owned | description or error attrs       | field status content              |

## 5. Attr Merge and Ownership Rules

- Per-thumb `role`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-label`, and hidden-input submission attrs always win.
- `class` and `style` merge additively on track, range, thumbs, and optional markers.
- Consumer decoration may style the thumbs, but it must not break start-versus-end thumb identity or min-gap ownership.

## 6. Composition / Context Contract

`RangeSlider` consumes field, fieldset, and optional form context. The component remains a single machine instance even though it exposes repeated thumbs and hidden inputs.

## 7. Prop Sync and Event Mapping

| Adapter prop                                | Mode          | Sync trigger          | Machine event / update path | Visible effect                                             |
| ------------------------------------------- | ------------- | --------------------- | --------------------------- | ---------------------------------------------------------- |
| `value`                                     | controlled    | prop change           | `SetValue`                  | updates both thumbs, range fill, output, and hidden inputs |
| `min` / `max` / `step` / `min_gap`          | controlled    | prop change           | constraint updates          | updates per-thumb bounds and keyboard behavior             |
| `disabled` / per-thumb disabled / `invalid` | controlled    | prop change           | `SetDisabled`-style updates | guards dragging and selection                              |
| pointer or keyboard interaction             | machine-owned | thumb or track events | range-slider events         | updates the active thumb and range values                  |

## 8. Registration and Cleanup Contract

- Track-measurement listeners and scroll or resize helpers are adapter-owned and must be removed on unmount.
- Thumb refs must stay addressable by thumb identity (`Start`, `End`), not only by DOM order.
- No additional descendant registry is required beyond optional marker rendering.

## 9. Ref and Node Contract

- `Track` owns one measurement ref.
- Each thumb owns its own live ref keyed by thumb identity.
- Hidden inputs remain non-focusable and must never become focus targets.

## 10. State Machine Boundary Rules

- The machine owns both thumb values, active-dragging state, and min-gap or crossing rules.
- The adapter may measure pointer position and translate it into an active-thumb value candidate, but it must not commit values outside the machine.
- Hidden inputs always reflect the machine-owned pair of committed values.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed `[start, end]` value pair after snapping and clamping.
- Active-thumb or dragging callbacks, if exposed, must reflect the machine-owned thumb identity.
- Output text must stay aligned with the same localized value text exposed through each thumb.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                             |
| ------------------------------------------------------------- | ------------------ | ----------------------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                                   |
| track measurement is unavailable                              | degrade gracefully | keyboard and direct thumb focus still work                        |
| thumb identity is lost across reorders                        | fail fast          | hidden-input mapping and focus behavior depend on stable identity |

## 13. Identity and Key Policy

Thumb identity is semantic, not positional: `Start` and `End` must remain stable across renders. Hidden inputs use the same stable thumb identity and submit values with consistent ordering.

## 14. SSR and Client Boundary Rules

- SSR renders the same thumbs, hidden inputs, and initial value-derived attrs as the hydrated client.
- Track measurement and drag behavior are client-only.
- Hidden-input count, order, and submitted suffix policy must match between server and client.

## 15. Performance Constraints

- Measure track geometry only when necessary.
- Keep thumb refs keyed by thumb identity rather than rebuilding by DOM order.
- Avoid duplicating the value pair outside the machine.

## 16. Implementation Dependencies

| Dependency          | Required? | Dependency type     | Why it must exist first                                  |
| ------------------- | --------- | ------------------- | -------------------------------------------------------- |
| measurement helper  | required  | adapter helper      | pointer position must map reliably to either thumb value |
| hidden-input helper | required  | submission contract | both thumb values must submit natively                   |

## 17. Recommended Implementation Sequence

1. Initialize the machine and derive attrs for all structural parts.
2. Establish track and per-thumb refs keyed by thumb identity.
3. Render root, track, range, both thumbs, both hidden inputs, and optional parts in stable order.
4. Wire controlled prop synchronization and pointer, keyboard, focus, or blur behavior.
5. Add output formatting, markers, and diagnostics.

## 18. Anti-Patterns

- Do not infer thumb identity from temporary DOM order alone.
- Do not let hidden inputs or markers become the interactive target.
- Do not commit pointer-derived values directly without the machine.

## 19. Consumer Expectations and Guarantees

- Consumers may assume start and end thumb identity stay stable.
- Consumers may assume both hidden inputs submit in stable order and remain adapter-owned.
- Consumers must not assume undocumented thumb crossing behavior beyond the core spec.

## 20. Platform Support Matrix

| Capability / behavior                                                            | Web          | Desktop      | Mobile       | SSR          | Notes                                             |
| -------------------------------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------------------- |
| documented two-thumb semantics, pointer interaction, and hidden-input submission | full support | full support | full support | full support | measurement and dragging activate after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                 |
| ----------------------------------------------- | -------------------- | ------------------- | ------------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode                |
| thumb identity or ordering contract violated    | fail fast            | fail fast           | protects focus and submission mapping |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                         | Notes                            |
| ------------------- | --------- | ------------------------------------------------------ | -------------------------------- |
| measurement helper  | required  | normalize pointer position and maintain track geometry | shared with single-value sliders |
| hidden-input helper | required  | derive both submitted values from machine state        | shared with slider-like controls |

## 23. Framework-Specific Behavior

Dioxus should keep separate refs for start and end thumbs, isolate host-specific geometry quirks behind local helpers, and ensure measurement observers are cleaned up with the instance lifecycle.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<range_slider::Machine>(props);

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        div { ..machine.derive(|api| api.track_attrs()).read().clone(),
            div { ..machine.derive(|api| api.range_attrs()).read().clone() }
            div { ..machine.derive(|api| api.thumb_attrs(ThumbIndex::Start)).read().clone() }
            div { ..machine.derive(|api| api.thumb_attrs(ThumbIndex::End)).read().clone() }
        }
        input { ..machine.derive(|api| api.hidden_input_attrs(ThumbIndex::Start)).read().clone() }
        input { ..machine.derive(|api| api.hidden_input_attrs(ThumbIndex::End)).read().clone() }
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine, then allocate one track ref plus one ref per thumb identity.
- Route pointer and keyboard behavior through machine events only, with measurement helpers choosing the active thumb when needed.
- Keep both hidden-input submission values derived from the same committed machine pair.

## 26. Adapter Invariants

- Start and end thumb identity are always stable.
- Hidden-input submission always reflects the committed `[start, end]` pair in stable order.
- Pointer measurement is adapter-owned, but value commitment is machine-owned.
- Measurement listeners are always cleaned up on unmount.

## 27. Accessibility and SSR Notes

- Each thumb must keep its localized `aria-label` and correct min or max bounds.
- Description-first, error-second ordering is mandatory for group-level `aria-describedby`.
- SSR must preserve thumb order, hidden-input order, and initial range values.

## 28. Parity Summary and Intentional Deviations

- Matches the core range-slider contract without intentional divergence.
- Promotes thumb identity, hidden-input submission, and listener cleanup into Dioxus-facing guidance.

## 29. Test Scenarios

- Dragging either thumb updates the committed value pair, range fill, and both hidden inputs correctly.
- Keyboard interaction respects min-gap constraints and RTL semantics where applicable.
- Controlled pair updates synchronize both thumbs without identity drift.
- Scroll or resize invalidates geometry without leaking listeners.

## 30. Test Oracle Notes

- Inspect per-thumb ARIA attrs and both hidden-input values in the DOM.
- Use synthetic geometry plus pointer events to verify active-thumb selection and min-gap enforcement.
- Assert listener cleanup with lifecycle or teardown checks rather than only visual output.

## 31. Implementation Checklist

- [ ] Keep start and end thumb identity stable across renders.
- [ ] Keep both hidden inputs adapter-owned and ordered deterministically.
- [ ] Route pointer, keyboard, and focus behavior through machine events only.
- [ ] Attach and remove measurement listeners with the track lifecycle.
- [ ] Preserve localized per-thumb labels and bounds.
- [ ] Keep output text and hidden-input submission aligned with the committed pair.
