---
adapter: dioxus
component: color-wheel
category: specialized
source: components/specialized/color-wheel.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ColorWheel — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorWheel`](../../components/specialized/color-wheel.md) contract onto Dioxus `0.7.x`. The adapter preserves circular hue selection, keyboard stepping, hidden-input submission, and composition with other color primitives.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct ColorWheelProps {
    #[props(optional)]
    pub value: Option<Signal<ColorValue>>,
    #[props(optional)]
    pub default_value: Option<ColorValue>,
    #[props(optional)]
    pub disabled: Option<bool>,
    #[props(optional)]
    pub readonly: Option<bool>,
}

#[component]
pub fn ColorWheel(props: ColorWheelProps) -> Element
```

The adapter owns the root, track, thumb, and optional hidden input for form participation.

## 3. Mapping to Core Component Contract

- Props parity: full parity with bindable color, disabled/read-only state, and form participation.
- Part parity: full parity with `Root`, `Track`, `Thumb`, and `HiddenInput`.
- Adapter additions: explicit angle math and drag cleanup.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                | Notes                               |
| --------------------- | --------- | ------------------------ | ------------- | -------------------------- | ----------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`         | group surface and state markers     |
| `Track`               | required  | `<div>`                  | adapter-owned | `api.track_attrs()`        | conic-gradient hue ring             |
| `Thumb`               | required  | `<div>`                  | adapter-owned | `api.thumb_attrs()`        | `role="slider"` hue handle          |
| `HiddenInput`         | optional  | `<input type="hidden">`  | adapter-owned | `api.hidden_input_attrs()` | form bridge using the current color |

## 5. Attr Merge and Ownership Rules

The thumb keeps slider semantics and hue value attrs. Consumer styling may restyle the ring but must not remove the focusable thumb or hidden input.

## 6. Composition / Context Contract

`ColorWheel` is standalone and composes through shared color binding with `ColorPicker`. No parent context is required.

## 7. Prop Sync and Event Mapping

Controlled value sync flows through the writable signal. Pointer interaction on the track maps angle around the ring into hue. Keyboard interaction on the thumb dispatches hue step, large-step, home, and end actions.

## 8. Registration and Cleanup Contract

Register one active drag session on pointer-down. Cleanup must release move/up listeners and pointer-capture bookkeeping on drag end, cancellation, or unmount.

## 9. Ref and Node Contract

`Track` requires a live node for center-point geometry reads. `Thumb` remains adapter-owned and focusable.

## 10. State Machine Boundary Rules

- machine-owned state: color value, dragging state, focus state, and hue math.
- adapter-local derived bookkeeping: live track rect and drag-session handles only.
- forbidden local mirrors: do not keep a second hue value outside the machine.

## 11. Callback Payload Contract

No dedicated callback is required; value observation happens through the controlled signal or hidden input.

## 12. Failure and Degradation Rules

If geometry cannot be measured, degrade gracefully by keeping keyboard operation active and suppressing pointer drag until the track becomes measurable.

## 13. Identity and Key Policy

The control is instance-scoped and has no repeated keyed descendants.

## 14. SSR and Client Boundary Rules

SSR renders the ring structure and hidden input. Pointer measurement and drag listeners are client-only.

## 15. Performance Constraints

Keep at most one drag session active. Recompute the conic gradient only when the color value changes.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type | Why it must exist first                | Notes                            |
| ------------------- | ----------- | --------------- | -------------------------------------- | -------------------------------- |
| circular hue helper | required    | geometry helper | maps pointer angle into hue            | shared with circular controls    |
| hidden-input helper | recommended | form helper     | keeps color form submission consistent | shared with other color controls |

## 17. Recommended Implementation Sequence

1. Render root, track, thumb, and hidden input.
2. Add thumb keyboard handling.
3. Add track measurement and circular drag cleanup.
4. Verify standalone and composed shared-signal use.

## 18. Anti-Patterns

- Do not move slider semantics from the thumb onto the whole ring.
- Do not keep stale ring geometry across layout changes.

## 19. Consumer Expectations and Guarantees

- Consumers may assume standalone use without `ColorPicker`.
- Consumers may assume shared `Signal<ColorValue>` composition with other color controls.
- Consumers must not assume pointer drag works before mount.

## 20. Platform Support Matrix

| Capability / behavior        | Web          | Desktop       | Mobile        | SSR          | Notes                                                 |
| ---------------------------- | ------------ | ------------- | ------------- | ------------ | ----------------------------------------------------- |
| keyboard hue slider behavior | full support | full support  | full support  | full support | structural and semantic parity on SSR                 |
| circular pointer drag        | full support | full support  | full support  | client-only  | requires live track geometry                          |
| hidden input                 | full support | fallback path | fallback path | full support | host form semantics outside web are adapter-dependent |

## 21. Debug Diagnostics and Production Policy

Missing track geometry after mount is a debug warning and graceful pointer fallback. Multiple drag sessions are fail-fast.

## 22. Shared Adapter Helper Notes

Use the shared circular-geometry helper for hue angle conversion and drag cleanup ordering.

## 23. Framework-Specific Behavior

Dioxus should measure the mounted ring node through a stable mounted handle and keep drag cleanup in instance-local effects or event-owned guards.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ColorWheel() -> Element {
    let machine = use_machine::<color_wheel::Machine>(color_wheel::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Build the machine from bindable props.
- Render the hue ring, thumb, and hidden input.
- Convert pointer angle into hue from live geometry only.
- Clean up drag listeners eagerly.

## 26. Adapter Invariants

- `Thumb` remains the only slider semantic node.
- At most one drag session exists at a time.
- Hue math is always driven by the core machine.

## 27. Accessibility and SSR Notes

The thumb’s hue-valuetext remains authoritative for assistive technology. The ring itself is a geometry surface only.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- hue keyboard stepping
- circular drag and cleanup
- shared-signal composition with other color controls
- hidden input reflects the current color

## 30. Test Oracle Notes

| Behavior         | Preferred oracle type | Notes                                                  |
| ---------------- | --------------------- | ------------------------------------------------------ |
| slider semantics | DOM attrs             | assert hue value attrs on `Thumb`                      |
| drag cleanup     | cleanup side effects  | assert no stale listeners survive                      |
| circular math    | rendered behavior     | assert representative coordinates map to expected hues |

## 31. Implementation Checklist

- [ ] Circular geometry is measured from a live node only.
- [ ] Drag cleanup is explicit and singular.
- [ ] Hidden input stays synchronized with the current color.
