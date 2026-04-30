---
adapter: leptos
component: angle-slider
category: specialized
source: components/specialized/angle-slider.md
source_foundation: foundation/08-adapter-leptos.md
---

# AngleSlider — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`AngleSlider`](../../components/specialized/angle-slider.md) contract onto Leptos `0.8.x`. The adapter preserves circular drag, keyboard stepping, marker rendering, optional hidden-input submission, and pointer-session cleanup.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn AngleSlider(
    #[prop(optional)] value: Option<RwSignal<f64>>,
    #[prop(optional, default = 0.0)] default_value: f64,
    #[prop(optional, default = 1.0)] step: f64,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] show_value_text: bool,
    #[prop(optional)] show_markers: bool,
) -> impl IntoView
```

The adapter renders the entire circular control surface, including optional `ValueText`, marker decorations, and the form-participating hidden input.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable angle value, stepping, disabled/read-only behavior, and display toggles.
- Part parity: full parity with `Root`, `Control`, `Track`, `Range`, `Thumb`, `ValueText`, `MarkerGroup`, `Marker`, and `HiddenInput`.
- Adapter additions: explicit pointer-to-angle conversion and drag cleanup rules.

## 4. Part Mapping

| Core part / structure    | Required?         | Adapter rendering target | Ownership     | Attr source                                        | Notes                                  |
| ------------------------ | ----------------- | ------------------------ | ------------- | -------------------------------------------------- | -------------------------------------- |
| `Root`                   | required          | `<div>`                  | adapter-owned | `api.root_attrs()`                                 | State, disabled, and grouping surface. |
| `Control`                | required          | `<div>`                  | adapter-owned | `api.control_attrs()`                              | Wraps circular track and thumb.        |
| `Track` / `Range`        | required          | `<div>`                  | adapter-owned | `api.track_attrs()` / `api.range_attrs()`          | Range is a visual arc.                 |
| `Thumb`                  | required          | `<div>`                  | adapter-owned | `api.thumb_attrs()`                                | `role="slider"` focus target.          |
| `ValueText`              | optional          | `<output>`               | adapter-owned | `api.value_text_attrs()`                           | Render when `show_value_text=true`.    |
| `MarkerGroup` / `Marker` | optional repeated | `<div>`                  | adapter-owned | `api.marker_group_attrs()` / `api.marker_attrs(i)` | Decorative angle guides only.          |
| `HiddenInput`            | optional          | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()`                         | Form value bridge.                     |

## 5. Attr Merge and Ownership Rules

Core slider semantics, `aria-valuenow/min/max`, and disabled/read-only attrs always win on `Thumb`. Consumer decoration must not replace the circular track or focus target.

## 6. Composition / Context Contract

`AngleSlider` is context-free. When used inside `ColorPicker`, it still owns its own value contract and drag lifecycle.

## 7. Prop Sync and Event Mapping

`value` uses bindable controlled/uncontrolled sync. Pointer-down on `Track` and drag movement dispatch angle-setting events after adapter-side pointer-to-angle conversion. Keyboard events on `Thumb` dispatch increment, decrement, min, max, and large-step actions directly to the core machine.

## 8. Registration and Cleanup Contract

Register one active drag session on thumb or track pointer-down. Cleanup must release any window-level move/up listeners and pointer-capture bookkeeping on drag end, cancellation, or unmount.

## 9. Ref and Node Contract

`Track` needs a live node handle after mount for center-point geometry reads. `Thumb` remains adapter-owned and focusable, but does not need an external consumer ref.

## 10. State Machine Boundary Rules

- machine-owned state: value, dragging state, focus-visible state, and snapping semantics.
- adapter-local derived bookkeeping: active pointer session and last measured track rect only.
- forbidden local mirrors: do not store an unsynchronized angle copy outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; value observation happens through the controlled signal or hidden input.

## 12. Failure and Degradation Rules

If measurement fails after mount, degrade gracefully by keeping keyboard operation active and ignoring pointer drag until geometry becomes available.

## 13. Identity and Key Policy

Marker order must remain stable when `show_markers=true`. Hidden input identity is the component instance.

## 14. SSR and Client Boundary Rules

SSR renders the full structural surface, including optional markers and hidden input. Pointer measurement and drag listeners are client-only.

## 15. Performance Constraints

Keep at most one drag session active. Recompute marker layout and formatted value only when the value or relevant display props change.

## 16. Implementation Dependencies

| Dependency           | Required?   | Dependency type | Why it must exist first                             | Notes                                         |
| -------------------- | ----------- | --------------- | --------------------------------------------------- | --------------------------------------------- |
| pointer-angle helper | required    | geometry helper | converts pointer coordinates into clockwise degrees | shared circular-control math                  |
| hidden-input helper  | recommended | form helper     | keeps form submission behavior consistent           | shared with other form-participating controls |

## 17. Recommended Implementation Sequence

1. Render root, control, track, range, and thumb.
2. Add keyboard handling on the thumb.
3. Add pointer measurement and drag cleanup.
4. Add optional value text, markers, and hidden input.

## 18. Anti-Patterns

- Do not keep drag listeners alive after pointer end.
- Do not move `role="slider"` from `Thumb` onto the entire control.

## 19. Consumer Expectations and Guarantees

- Consumers may assume keyboard and pointer paths produce the same snapped value.
- Consumers may assume markers are decorative only.
- Consumers must not assume pointer dragging works before mount.

## 20. Platform Support Matrix

| Capability / behavior              | Browser client | SSR          | Notes                                   |
| ---------------------------------- | -------------- | ------------ | --------------------------------------- |
| keyboard slider behavior           | full support   | full support | structural plus attr parity on SSR      |
| pointer drag and circular geometry | full support   | client-only  | requires live measurement               |
| hidden input submission            | full support   | full support | static value on SSR, reactive on client |

## 21. Debug Diagnostics and Production Policy

Missing track geometry after mount is a debug warning and graceful pointer fallback. Multiple active drag sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use one circular-geometry helper for pointer-to-angle conversion and one drag-session helper for cleanup ordering.

## 23. Framework-Specific Behavior

Leptos should derive attrs reactively and perform geometry reads from the mounted track node inside pointer handlers, not during SSR render.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn AngleSlider() -> impl IntoView {
    let machine = use_machine::<angle_slider::Machine>(angle_slider::Props::default());
    view! { <div {..machine.derive(|api| api.root_attrs()).get()} /> }
}
```

## 25. Reference Implementation Skeleton

- Build the machine from bindable props.
- Render the circular structure and hidden input.
- Convert pointer coordinates to degrees only from live track geometry.
- Clean up drag listeners eagerly.

## 26. Adapter Invariants

- `Thumb` is always the slider semantics owner.
- At most one pointer drag session exists at a time.
- Pointer cleanup runs before unmount completes.

## 27. Accessibility and SSR Notes

`ValueText` remains non-authoritative for assistive technology; `Thumb` `aria-valuetext` is the primary spoken value.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- keyboard stepping, home/end, and page stepping
- track click to set value
- thumb drag with cleanup on pointer cancel
- hidden input reflects the current value

## 30. Test Oracle Notes

| Behavior         | Preferred oracle type | Notes                                                    |
| ---------------- | --------------------- | -------------------------------------------------------- |
| slider semantics | DOM attrs             | assert value and read-only/disabled attrs on `Thumb`     |
| drag cleanup     | cleanup side effects  | assert global listeners are removed                      |
| circular math    | rendered behavior     | assert representative coordinates map to expected angles |

## 31. Implementation Checklist

- [ ] Circular geometry is measured from the live track.
- [ ] Drag cleanup is explicit and singular.
- [ ] `Thumb` remains the only slider semantic node.
