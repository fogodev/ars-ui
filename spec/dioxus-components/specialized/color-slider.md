---
adapter: dioxus
component: color-slider
category: specialized
source: components/specialized/color-slider.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ColorSlider — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorSlider`](../../components/specialized/color-slider.md) contract onto Dioxus `0.7.x`. The adapter preserves one-dimensional color-channel manipulation, orientation-aware semantics, gradient track rendering, and shared-color composition with `ColorArea` and `ColorPicker`.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ColorSliderProps {
    pub channel: ColorChannel,
    #[props(optional)]
    pub orientation: Option<Orientation>,
    #[props(optional)]
    pub value: Option<Signal<ColorValue>>,
    #[props(optional)]
    pub default_value: Option<ColorValue>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub readonly: Option<bool>,
    #[props(optional)]
    pub show_output: Option<bool>,
}

#[component]
pub fn ColorSlider(props: ColorSliderProps) -> Element
```

The adapter owns the full track, thumb, optional output, and hidden-input surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with channel selection, orientation, bindable color, and disabled/read-only state.
- Part parity: full parity with `Root`, `Label`, `Track`, `Thumb`, `Output`, and `HiddenInput`.
- Adapter additions: explicit track geometry reads and orientation-aware semantic repair.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                | Notes                                              |
| --------------------- | --------- | ------------------------ | ------------- | -------------------------- | -------------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`         | group surface with channel and orientation markers |
| `Label`               | required  | `<label>`                | adapter-owned | `api.label_attrs()`        | labels the thumb                                   |
| `Track`               | required  | `<div>`                  | adapter-owned | `api.track_attrs()`        | gradient geometry surface                          |
| `Thumb`               | required  | `<div>`                  | adapter-owned | `api.thumb_attrs()`        | `role="slider"` focus target                       |
| `Output`              | optional  | `<output>`               | adapter-owned | `api.output_attrs()`       | render when `show_output=true`                     |
| `HiddenInput`         | optional  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()` | form bridge using color string                     |

## 5. Attr Merge and Ownership Rules

The thumb keeps `aria-valuenow`, `aria-valuetext`, `aria-orientation`, and disabled/read-only attrs. Consumer decoration may restyle the gradient but must not remove the semantic thumb or output linkage.

## 6. Composition / Context Contract

`ColorSlider` is standalone and also composes through a shared `ColorValue` signal with `ColorArea`, `ColorField`, and `ColorPicker`. No parent context is required.

## 7. Prop Sync and Event Mapping

Controlled value sync flows through the writable signal. Pointer interaction on `Track` maps one-dimensional position into the selected channel. Keyboard interaction on `Thumb` dispatches the core step, large-step, home, and end actions and must honor `orientation`.

## 8. Registration and Cleanup Contract

Register one active drag session on pointer-down. Cleanup must release move/up listeners and pointer-capture bookkeeping on drag end, cancellation, or unmount.

## 9. Ref and Node Contract

`Track` requires a live node for orientation-aware geometry reads. `Thumb` is the focus owner and remains adapter-owned.

## 10. State Machine Boundary Rules

- machine-owned state: color value, dragging state, focus state, and channel math.
- adapter-local derived bookkeeping: live track rect and drag-session handles only.
- forbidden local mirrors: do not keep a second channel value outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; value observation happens through the controlled signal or hidden input.

## 12. Failure and Degradation Rules

If geometry cannot be measured, degrade gracefully by keeping keyboard operation active and suppressing pointer drag until the track becomes measurable.

## 13. Identity and Key Policy

The control is instance-scoped and has no repeated registration beyond the optional hidden input.

## 14. SSR and Client Boundary Rules

SSR renders the root, label, track, thumb, optional output, and hidden input. Pointer measurement and drag listeners are client-only.

## 15. Performance Constraints

Keep at most one drag session active. Recompute the gradient only when the channel, orientation, or color value changes.

## 16. Implementation Dependencies

| Dependency            | Required?   | Dependency type | Why it must exist first                             | Notes                            |
| --------------------- | ----------- | --------------- | --------------------------------------------------- | -------------------------------- |
| 1D measurement helper | required    | geometry helper | maps pointer position to normalized slider progress | shared with sliders              |
| hidden-input helper   | recommended | form helper     | keeps color form submission consistent              | shared with other color controls |

## 17. Recommended Implementation Sequence

1. Render root, label, track, thumb, output, and hidden input.
2. Add thumb keyboard handling with orientation support.
3. Add track measurement and pointer drag cleanup.
4. Verify standalone and composed shared-signal use.

## 18. Anti-Patterns

- Do not move slider semantics from the thumb onto the track.
- Do not omit `aria-orientation` on the thumb.

## 19. Consumer Expectations and Guarantees

- Consumers may assume standalone use without `ColorPicker`.
- Consumers may assume shared `Signal<ColorValue>` composition with other color controls.
- Consumers must not assume pointer drag works before mount.

## 20. Platform Support Matrix

| Capability / behavior    | Web          | Desktop       | Mobile        | SSR          | Notes                                                 |
| ------------------------ | ------------ | ------------- | ------------- | ------------ | ----------------------------------------------------- |
| keyboard slider behavior | full support | full support  | full support  | full support | structural and semantic parity on SSR                 |
| pointer drag             | full support | full support  | full support  | client-only  | requires live track geometry                          |
| hidden input             | full support | fallback path | fallback path | full support | host form semantics outside web are adapter-dependent |

## 21. Debug Diagnostics and Production Policy

Missing track geometry after mount is a debug warning and graceful pointer fallback. Multiple drag sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use a shared 1D drag helper for position normalization and drag cleanup ordering.

## 23. Framework-Specific Behavior

Dioxus should measure the mounted track node through a stable mounted handle and keep drag cleanup in instance-local effects or event-owned guards.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ColorSliderSketchProps {
    pub channel: ColorChannel,
}

#[component]
pub fn ColorSlider(props: ColorSliderSketchProps) -> Element {
    let machine = use_machine::<color_slider::Machine>(color_slider::Props { channel: props.channel, ..Default::default() });
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Build the machine from bindable props.
- Render the orientation-aware slider surface.
- Convert pointer position into channel progress from live geometry only.
- Clean up drag listeners eagerly.

## 26. Adapter Invariants

- `Thumb` remains the only slider semantic node.
- At most one drag session exists at a time.
- Orientation is reflected in both geometry math and ARIA output.

## 27. Accessibility and SSR Notes

`Output` must remain `aria-live="off"` or equivalent non-announcing output so `Thumb` `aria-valuetext` stays authoritative.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- horizontal and vertical keyboard navigation
- track click and thumb drag
- shared-signal composition with `ColorArea`
- hidden input reflects the current color

## 30. Test Oracle Notes

| Behavior          | Preferred oracle type | Notes                                       |
| ----------------- | --------------------- | ------------------------------------------- |
| slider semantics  | DOM attrs             | assert `aria-orientation` and value attrs   |
| drag cleanup      | cleanup side effects  | assert no stale listeners survive           |
| gradient behavior | rendered structure    | assert channel-specific CSS vars or classes |

## 31. Implementation Checklist

- [ ] `aria-orientation` is always explicit.
- [ ] Drag cleanup is explicit and singular.
- [ ] Shared color composition does not require parent context.
