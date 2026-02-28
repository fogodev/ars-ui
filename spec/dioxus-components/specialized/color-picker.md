---
adapter: dioxus
component: color-picker
category: specialized
source: components/specialized/color-picker.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ColorPicker — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorPicker`](../../components/specialized/color-picker.md) contract onto Dioxus `0.7.x`. The adapter preserves popover-backed color editing, standalone color-primitive composition, swatch presets, format switching, optional alpha editing, and EyeDropper fallback policy.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerProps {
    #[props(optional)]
    pub value: Option<Signal<ColorValue>>,
    #[props(optional)]
    pub default_value: Option<ColorValue>,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(optional)]
    pub default_open: Option<bool>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub readonly: Option<bool>,
    #[props(optional)]
    pub show_alpha: Option<bool>,
    #[props(optional)]
    pub formats: Option<Vec<ColorFormat>>,
    #[props(optional)]
    pub swatches: Option<Vec<ColorValue>>,
}

#[component]
pub fn ColorPicker(props: ColorPickerProps) -> Element
```

The adapter renders the integrated picker surface: trigger, popover content, color area, channel sliders, optional alpha slider, format controls, text inputs, swatches, eyedropper trigger, and hidden input.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable value/open state, alpha visibility, formats, swatches, and disabled/read-only behavior.
- Part parity: full parity with `Root`, `Label`, `Control`, `Trigger`, `Content`, `Area`, `AreaThumb`, `ChannelSlider`, `ChannelSliderThumb`, `AlphaSlider`, `SwatchGroup`, `Swatch`, `FormatSelect`, `ChannelInput`, `HexInput`, `EyeDropperTrigger`, and `HiddenInput`.
- Adapter additions: explicit composition of standalone color primitives, popover ownership, and browser EyeDropper gating.

## 4. Part Mapping

| Core part / structure     | Required?                    | Adapter rendering target               | Ownership     | Attr source                                    | Notes                               |
| ------------------------- | ---------------------------- | -------------------------------------- | ------------- | ---------------------------------------------- | ----------------------------------- |
| trigger shell             | required                     | `<button>` plus swatch/control wrapper | adapter-owned | `api.trigger_attrs()` and related attrs        | owns open/close behavior            |
| popover content           | required                     | `<div>`                                | adapter-owned | `api.content_attrs()`                          | dialog-style popup surface          |
| area and sliders          | required with optional alpha | composed child adapters                | adapter-owned | color primitive attrs via shared value binding | primitives remain composition units |
| format and text inputs    | optional                     | `<select>` and `<input>`               | adapter-owned | core attrs                                     | format switching and numeric entry  |
| swatch group and swatches | optional repeated            | `<div>` plus buttons                   | adapter-owned | core attrs                                     | selection presets                   |
| `EyeDropperTrigger`       | optional                     | `<button>`                             | adapter-owned | `api.eye_dropper_trigger_attrs()`              | render only when supported          |
| `HiddenInput`             | optional                     | `<input type="hidden">`                | adapter-owned | `api.hidden_input_attrs()`                     | form bridge                         |

## 5. Attr Merge and Ownership Rules

Trigger popup semantics, content dialog semantics, slider semantics on composed primitives, and hidden-input synchronization always win. Consumer decoration must not weaken the composed accessibility contract.

## 6. Composition / Context Contract

`ColorPicker` composes `ColorArea`, `ColorSlider`, `ColorField`, `ColorSwatchPicker`, `ColorSwatch`, `ColorWheel`, and `AngleSlider` through a shared `Signal<ColorValue>` or equivalent machine-owned binding. Consumers do not receive a separate public context from the base surface.

## 7. Prop Sync and Event Mapping

`value` and `open` each use controlled/uncontrolled binding. Trigger activation toggles open state. Composed primitive changes feed the shared color value. Format selection dispatches format-switch events. EyeDropper activation must start directly from the trigger gesture and dispatch the sampled color back into the same value pipeline.

## 8. Registration and Cleanup Contract

Popover-owned outside-interaction handlers, any active color-drag sessions inside composed primitives, and EyeDropper in-flight state all clean up with the component instance. No independent primitive may leak listeners after the picker closes.

## 9. Ref and Node Contract

Trigger and content need the live handles required by the popup contract. Composed primitives own their own measurement refs and must not share them unsafely across instances.

## 10. State Machine Boundary Rules

- machine-owned state: current color, open state, current format, active color-space view, swatch selection, and read-only/disabled semantics.
- adapter-local derived bookkeeping: EyeDropper support probe and popup/primitive node handles only.
- forbidden local mirrors: do not keep a second color value or open flag outside the machine bindings.

## 11. Callback Payload Contract

No dedicated callback is required beyond the bindable `value` and `open` surfaces.

## 12. Failure and Degradation Rules

If EyeDropper is unsupported or activation fails, degrade gracefully by hiding or disabling the trigger and preserving the rest of the picker. If popup positioning is unavailable, render the content adjacent to the trigger while keeping dialog labeling intact.

## 13. Identity and Key Policy

Swatch identity is collection order plus color value. Format options and channel inputs must remain stable enough that focus does not jump unexpectedly across controlled rerenders.

## 14. SSR and Client Boundary Rules

SSR renders the closed or open structure implied by the initial `open` value, including trigger, optional popup content branch, and hidden input. EyeDropper support, popup positioning, and primitive drag sessions are client-only.

## 15. Performance Constraints

Reuse one shared color binding across composed primitives. Do not rebuild all primitives when only popup open state changes. Avoid duplicate gradient or text-format recomputation when the color value is unchanged.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type        | Why it must exist first                                                   | Notes                            |
| ------------------------ | ----------- | ---------------------- | ------------------------------------------------------------------------- | -------------------------------- |
| popover contract         | required    | composition dependency | open state, dismissal, and positioning are popup-backed                   | shared overlay behavior          |
| color primitive adapters | required    | composition dependency | `ColorPicker` is an orchestration surface, not a duplicate implementation | shared value binding is critical |
| EyeDropper helper        | recommended | browser helper         | runtime support probe and activation policy                               | Chromium-only capability         |

## 17. Recommended Implementation Sequence

1. Build shared bindable color and open state.
2. Render trigger and popup shell.
3. Compose area, sliders, fields, and swatches from the shared color binding.
4. Add format switching and optional alpha handling.
5. Add EyeDropper gating and fallback.

## 18. Anti-Patterns

- Do not duplicate color math already owned by the standalone primitives.
- Do not call `EyeDropper.open()` outside a direct user gesture.
- Do not keep the popup open-state mirror separate from the machine binding.

## 19. Consumer Expectations and Guarantees

- Consumers may assume all composed primitives stay synchronized through one shared color value.
- Consumers may assume the picker works without EyeDropper support.
- Consumers must not assume every optional part renders in every configuration.

## 20. Platform Support Matrix

| Capability / behavior                                      | Web           | Desktop        | Mobile         | SSR            | Notes                                 |
| ---------------------------------------------------------- | ------------- | -------------- | -------------- | -------------- | ------------------------------------- |
| picker trigger, popup semantics, and primitive composition | full support  | full support   | full support   | full support   | structural parity on SSR              |
| popup positioning and outside interaction                  | full support  | full support   | full support   | client-only    | runtime overlay behavior              |
| EyeDropper trigger                                         | fallback path | not applicable | not applicable | not applicable | Chromium-like browser capability only |

## 21. Debug Diagnostics and Production Policy

Missing shared primitive composition or duplicate state mirrors are fail-fast. EyeDropper unavailability is a debug warning and graceful fallback.

## 22. Shared Adapter Helper Notes

`ColorPicker` should reuse the standalone color primitive adapters, the popup helper taxonomy, and the shared color parser/formatter helpers rather than reimplementing them.

## 23. Framework-Specific Behavior

Dioxus should keep the shared color binding in one signal, derive popup and primitive attrs from that binding, and gate EyeDropper support through a runtime probe kept outside the render-heavy branches.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ColorPicker() -> Element {
    let color = use_signal(ColorValue::default);
    rsx! { button {} }
}
```

## 25. Reference Implementation Skeleton

- Create one shared color binding and one shared open binding.
- Render the trigger and popover surface.
- Compose primitives from the shared value instead of duplicating logic.
- Gate EyeDropper through a support probe and direct trigger activation.

## 26. Adapter Invariants

- All composed primitives read and write the same color binding.
- Popup open state is owned by one bindable source of truth.
- EyeDropper support never blocks the rest of the picker.

## 27. Accessibility and SSR Notes

Trigger labeling, popup dialog labeling, primitive slider semantics, and swatch option semantics must remain intact across all optional branches. EyeDropper remains optional and must not be the only accessible color-entry path.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: the adapter standardizes on composing the standalone color primitives directly rather than describing separate picker-only implementations of those parts.

## 29. Test Scenarios

- trigger open/close and focus return
- shared value sync across area, sliders, fields, and swatches
- format switching with stable focus
- alpha on/off configuration
- EyeDropper hidden or disabled when unsupported

## 30. Test Oracle Notes

| Behavior               | Preferred oracle type      | Notes                                                |
| ---------------------- | -------------------------- | ---------------------------------------------------- |
| popup semantics        | DOM attrs plus interaction | assert trigger popup attrs and content dialog attrs  |
| shared synchronization | rendered behavior          | drive one primitive and assert updates in the others |
| EyeDropper fallback    | mocked browser API         | remove support and verify trigger fallback           |

## 31. Implementation Checklist

- [ ] One shared color binding powers all composed primitives.
- [ ] Popup open state is singular and bindable.
- [ ] EyeDropper support is runtime-gated and optional.
