---
adapter: leptos
component: color-area
category: specialized
source: components/specialized/color-area.md
source_foundation: foundation/08-adapter-leptos.md
---

# ColorArea — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorArea`](../../components/specialized/color-area.md) contract onto Leptos `0.8.x`. The adapter preserves the two-axis color surface, thumb keyboard semantics, hidden-input submission, and composition with `ColorSlider` and `ColorPicker`.

## 2. Public Adapter API

```rust
#[component]
pub fn ColorArea(
    #[prop(optional)] x_channel: ColorChannel,
    #[prop(optional)] y_channel: ColorChannel,
    #[prop(optional)] value: Option<RwSignal<ColorValue>>,
    #[prop(optional)] default_value: ColorValue,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
) -> impl IntoView
```

The adapter renders the standalone color-area primitive. Shared color state is expressed through the writable `RwSignal<ColorValue>` when the control is composed with other color primitives.

## 3. Mapping to Core Component Contract

- Props parity: full parity with channel selection, bindable value, disabled/read-only state, and form participation.
- Part parity: full parity with `Root`, `Background`, `Thumb`, and `HiddenInput`.
- Adapter additions: explicit 2D geometry measurement and drag-session cleanup.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                | Notes                                    |
| --------------------- | --------- | ------------------------ | ------------- | -------------------------- | ---------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`         | group surface and state markers          |
| `Background`          | required  | `<div>`                  | adapter-owned | `api.background_attrs()`   | gradient plane used for pointer geometry |
| `Thumb`               | required  | `<div>`                  | adapter-owned | `api.thumb_attrs()`        | focusable 2D control handle              |
| `HiddenInput`         | optional  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()` | form bridge using hex value              |

## 5. Attr Merge and Ownership Rules

`Thumb` keeps `role="application"`, `aria-roledescription`, `aria-valuetext`, and focus attrs. Consumer classes may restyle gradients, but must not remove semantic repair or the hidden input.

## 6. Composition / Context Contract

`ColorArea` is standalone and also composes inside `ColorPicker`. It does not require parent context; composition happens through shared `ColorValue` binding only.

## 7. Prop Sync and Event Mapping

Controlled value sync flows through the writable signal. Pointer interaction on `Background` converts local x/y coordinates into channel values and dispatches machine events. Keyboard interaction on `Thumb` adjusts the current two-axis position using the core key contract.

## 8. Registration and Cleanup Contract

Register one active drag session on pointer-down. Cleanup must cancel move/up listeners and any pointer-capture state on drag end, cancellation, or unmount.

## 9. Ref and Node Contract

`Background` requires a live node for bounding-rect reads after mount. `Thumb` remains adapter-owned and focusable without consumer ref composition.

## 10. State Machine Boundary Rules

- machine-owned state: current color value, dragging state, focus-visible state, and channel math.
- adapter-local derived bookkeeping: active background rect and drag-session handles only.
- forbidden local mirrors: do not keep a second color value copy outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; value changes are observed through the controlled signal or form bridge.

## 12. Failure and Degradation Rules

If geometry cannot be measured, degrade gracefully by leaving keyboard interaction enabled and suppressing pointer drag until the background node becomes measurable.

## 13. Identity and Key Policy

The hidden input and drag session are instance-scoped. No repeated descendants require key management.

## 14. SSR and Client Boundary Rules

SSR renders the structural surface and hidden input. Pointer measurement and drag listeners are client-only.

## 15. Performance Constraints

Keep at most one drag session active. Recompute gradient styling only when the value or channel selection changes.

## 16. Implementation Dependencies

| Dependency            | Required?   | Dependency type | Why it must exist first                                    | Notes                            |
| --------------------- | ----------- | --------------- | ---------------------------------------------------------- | -------------------------------- |
| 2D measurement helper | required    | geometry helper | maps background coordinates into normalized channel values | shared with drag surfaces        |
| hidden-input helper   | recommended | form helper     | keeps color form submission consistent                     | shared with other color controls |

## 17. Recommended Implementation Sequence

1. Render root, background, thumb, and hidden input.
2. Add thumb keyboard handling.
3. Add background measurement and pointer drag cleanup.
4. Verify standalone and composed shared-signal use.

## 18. Anti-Patterns

- Do not move 2D semantics from the thumb onto the whole root.
- Do not keep stale background geometry across layout changes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume standalone use without `ColorPicker`.
- Consumers may assume shared `RwSignal<ColorValue>` composition with `ColorSlider`.
- Consumers must not assume pointer interaction before mount.

## 20. Platform Support Matrix

| Capability / behavior              | Browser client | SSR          | Notes                                |
| ---------------------------------- | -------------- | ------------ | ------------------------------------ |
| keyboard and hidden-input behavior | full support   | full support | structural parity on SSR             |
| 2D pointer drag                    | full support   | client-only  | requires live background measurement |

## 21. Debug Diagnostics and Production Policy

Missing background geometry after mount is a debug warning and graceful pointer fallback. Multiple drag sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use a shared 2D drag helper for coordinate normalization and cleanup ordering across color surfaces.

## 23. Framework-Specific Behavior

Leptos should derive attrs reactively and read the background rect from the event target or mounted node inside the pointer path only.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ColorArea() -> impl IntoView {
    let machine = use_machine::<color_area::Machine>(color_area::Props::default());
    view! { <div {..machine.derive(|api| api.root_attrs()).get()} /> }
}
```

## 25. Reference Implementation Skeleton

- Build the machine from bindable value props.
- Render the gradient background, thumb, and hidden input.
- Convert pointer coordinates to channel values from live geometry only.
- Clean up drag listeners eagerly.

## 26. Adapter Invariants

- `Thumb` remains the only interactive semantic node.
- At most one drag session exists at a time.
- Shared color state stays machine-driven even when composed with other controls.

## 27. Accessibility and SSR Notes

The thumb’s `aria-valuetext` is the primary spoken surface. The background remains a geometry surface, not an independent control for assistive technology.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- shared-signal composition with `ColorSlider`
- thumb keyboard movement
- background pointer drag and cleanup
- hidden input reflects the current color

## 30. Test Oracle Notes

| Behavior           | Preferred oracle type | Notes                                                                  |
| ------------------ | --------------------- | ---------------------------------------------------------------------- |
| thumb semantics    | DOM attrs             | assert valuetext and focus attrs                                       |
| drag cleanup       | cleanup side effects  | assert no stale global listeners                                       |
| coordinate mapping | rendered behavior     | assert representative x/y positions map to the expected color channels |

## 31. Implementation Checklist

- [ ] Background measurement is taken from a live node only.
- [ ] Drag cleanup is explicit and singular.
- [ ] Shared color composition does not require parent context.
