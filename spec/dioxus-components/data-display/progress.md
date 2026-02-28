---
adapter: dioxus
component: progress
category: data-display
source: components/data-display/progress.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Progress — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Progress`](../../components/data-display/progress.md) contract onto a Dioxus 0.7.x component. The adapter must preserve the value/indeterminate state machine, locale-aware `aria-valuetext`, and both linear and circular part mappings without hiding controlled-sync behavior.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ProgressProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<f64>>>,
    #[props(optional)]
    pub default_value: Option<f64>,
    #[props(optional)]
    pub min: Option<f64>,
    #[props(optional)]
    pub max: Option<f64>,
    #[props(optional)]
    pub orientation: Option<progress::Orientation>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<progress::Messages>,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<f64>>>,
    pub label: Element,
    pub value_text: Element,
}

#[component]
pub fn Progress(props: ProgressProps) -> Element
```

The adapter surfaces the core progress props plus adapter callbacks and slots for visible label and value text content.

## 3. Mapping to Core Component Contract

- Props parity: full parity with controlled/uncontrolled value semantics.
- Part parity: full parity for `Root`, `Label`, `Track`, `Range`, `ValueText`, `CircleTrack`, and `CircleRange`.
- Traceability note: this spec promotes `aria-valuetext`, controlled sync, indeterminate semantics, and circular rendering guidance from the agnostic spec.

## 4. Part Mapping

| Core part     | Required?                   | Adapter rendering target | Ownership     | Attr source                      | Notes                                            |
| ------------- | --------------------------- | ------------------------ | ------------- | -------------------------------- | ------------------------------------------------ |
| `Root`        | required                    | `<div>`                  | adapter-owned | `api.root_attrs()`               | Carries `role="progressbar"` and state attrs.    |
| `Label`       | optional                    | `<label>` or `<span>`    | adapter-owned | `api.label_attrs()`              | Associated to `Root`.                            |
| `Track`       | required for linear variant | `<div>`                  | adapter-owned | `api.track_attrs()`              | Background bar.                                  |
| `Range`       | required for linear variant | `<div>`                  | adapter-owned | `api.range_attrs()`              | Width reflects percent or indeterminate styling. |
| `ValueText`   | optional                    | `<span>`                 | adapter-owned | `api.value_text_attrs()`         | Visible companion to `aria-valuetext`.           |
| `CircleTrack` | optional                    | `<circle>`               | adapter-owned | `api.circle_track_attrs()`       | SVG background ring.                             |
| `CircleRange` | optional                    | `<circle>`               | adapter-owned | `api.circle_range_attrs(radius)` | SVG progress ring.                               |

## 5. Attr Merge and Ownership Rules

- Core attrs include `role="progressbar"`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext`, `data-ars-state`, and orientation attrs.
- The adapter owns omission of `aria-valuenow` in indeterminate mode and the choice between linear and circular structural parts.
- Consumer classes merge additively, but required value and orientation semantics must remain intact.

## 6. Composition / Context Contract

`Progress` is standalone. It may resolve locale and messages from the nearest `ArsProvider`, and may render visible label/value slots, but it does not publish adapter context.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode                    | Sync trigger              | Machine event / update path | Notes                                                |
| -------------------- | ----------------------- | ------------------------- | --------------------------- | ---------------------------------------------------- |
| `value`              | controlled              | signal change after mount | `SetValue`                  | `None` means indeterminate.                          |
| `max`                | controlled or init-only | prop change               | `SetMax`                    | Recomputes percent.                                  |
| completion shortcut  | derived                 | `value >= max`            | `Complete`                  | Adapter does not duplicate completion state locally. |
| `on_value_change`    | adapter callback        | bindable observation      | observe machine context     | Fires after state/context update.                    |

## 8. Registration and Cleanup Contract

No descendant registration is required. Cleanup is limited to signal watchers or effects that observe controlled props.

## 9. Ref and Node Contract

No live ref is required for the core linear path. Circular SVG rendering does not require persistent DOM handles either.

## 10. State Machine Boundary Rules

- Machine-owned state: indeterminate/loading/complete state, percent, and current value.
- Adapter-owned derived values: visible slot content and optional circular radius math.
- Forbidden mirror: do not track a separate local percent outside machine derivation.

## 11. Callback Payload Contract

| Callback          | Payload source      | Payload shape | Timing               | Cancelable? | Notes                                |
| ----------------- | ------------------- | ------------- | -------------------- | ----------- | ------------------------------------ |
| `on_value_change` | adapter observation | `Option<f64>` | after machine update | no          | `None` indicates indeterminate mode. |

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                 |
| ------------------------------------------------------------- | ------------------ | ----------------------------------------------------- |
| invalid bounds                                                | fail fast          | Reject `min >= max` in development.                   |
| missing localized messages for indeterminate or complete text | degrade gracefully | Fall back to built-in defaults.                       |
| unsupported circular styling                                  | fallback path      | Keep linear semantics and omit circular presentation. |

## 13. Identity and Key Policy

The root identity remains stable across determinate and indeterminate updates. Switching between linear and circular presentation must be wrapper-driven and hydration-stable.

## 14. SSR and Client Boundary Rules

- SSR renders the initial state semantics and structural branch.
- Controlled value watchers start after mount.
- Indeterminate vs determinate output must match on server and client for the same initial props.

## 15. Performance Constraints

- Percent and `aria-valuetext` should be memoized from machine context.
- Circular stroke calculations should be derived, not recomputed through ad hoc DOM reads.

## 16. Implementation Dependencies

| Dependency        | Required? | Dependency type | Why it must exist first                        | Notes                           |
| ----------------- | --------- | --------------- | ---------------------------------------------- | ------------------------------- |
| formatting helper | required  | i18n contract   | Produces locale-aware percent and status text. | Shared with `meter` and `stat`. |

## 17. Recommended Implementation Sequence

1. Initialize the machine from value props.
2. Wire controlled `value` and `max` sync.
3. Render root semantics plus linear or circular parts.
4. Observe value changes for callback emission.

## 18. Anti-Patterns

- Do not set `aria-valuenow` in indeterminate mode.
- Do not derive completion only from visible range width.
- Do not recompute progress state outside the machine.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `aria-valuetext` always reflects the current machine state.
- Consumers may assume indeterminate mode is represented semantically, not only visually.
- Consumers must not assume both linear and circular parts render simultaneously.

## 20. Platform Support Matrix

| Capability / behavior        | Web          | Desktop      | Mobile       | SSR            | Notes                                           |
| ---------------------------- | ------------ | ------------ | ------------ | -------------- | ----------------------------------------------- |
| linear progress semantics    | full support | full support | full support | full support   | Server-safe attr output.                        |
| circular SVG rendering       | full support | full support | full support | full support   | Structural branch must remain hydration-stable. |
| controlled value observation | client-only  | client-only  | client-only  | SSR-safe empty | Reactive watchers start after mount.            |

## 21. Debug Diagnostics and Production Policy

| Condition                              | Debug build behavior | Production behavior | Notes                                                        |
| -------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------ |
| impossible bounds                      | fail fast            | warn and ignore     | Production may clamp only when semantics remain coherent.    |
| callback wired to stale observed value | debug warning        | no-op               | Callback emission should follow the newest machine snapshot. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required? | Responsibility                    | Reused by       | Notes                                             |
| ----------------- | --------- | --------------------------------- | --------------- | ------------------------------------------------- |
| formatting helper | required  | Build status and percent strings. | `meter`, `stat` | Must support locale-sensitive percent formatting. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should observe controlled props with effects and memoize the derived attr maps so progress updates only re-render the parts that depend on percent or value text.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct ProgressSketchProps {
    #[props(optional)]
    pub value: Option<Signal<Option<f64>>>,
}

#[component]
pub fn Progress(props: ProgressSketchProps) -> Element {
    let machine = use_machine::<progress::Machine>(progress::Props {
        value: props.value.as_ref().map(|s| s.read().clone()),
        ..Default::default()
    });
    let strategy = use_style_strategy();

    use_effect(move || {
        if let Some(value) = props.value.as_ref() {
            machine.send.call(progress::Event::SetValue(value.read().clone()));
        }
    });

    rsx! {
        div {
            ..attr_map_to_dioxus(machine.derive(|api| api.root_attrs())(), &strategy, None).attrs,
            div {
                ..attr_map_to_dioxus(machine.derive(|api| api.track_attrs())(), &strategy, None).attrs,
                div { ..attr_map_to_dioxus(machine.derive(|api| api.range_attrs())(), &strategy, None).attrs }
            }
        }
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, optional `value` and `max` watchers, and one value-change observer for callback emission. Circular rendering should remain a pure presentational branch over the same machine state.

## 26. Adapter Invariants

- `aria-valuetext` is always present and meaningful.
- Indeterminate mode omits `aria-valuenow`.
- Controlled sync never bypasses the machine.

## 27. Accessibility and SSR Notes

- Localize indeterminate and complete labels.
- Keep visible `ValueText` consistent with the root's `aria-valuetext`.
- SSR must not change structural branch or state semantics during hydration.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit controlled-sync and circular-rendering guidance.
- Intentional deviations: none beyond adapter callback wiring and slot support.

## 29. Test Scenarios

1. Determinate value updates recompute percent and fire `on_value_change`.
2. Indeterminate mode omits `aria-valuenow` and uses localized loading text.
3. Circular rendering uses the same machine state as the linear path.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect root ARIA attrs across idle, loading, and complete states.
- Preferred oracle for callback timing: assert observer order after machine updates.
- Verification recipe: toggle between `Some(v)` and `None` values and confirm only semantic attrs and documented parts change.

## 31. Implementation Checklist

- [ ] Controlled `value` and `max` sync through machine events.
- [ ] `aria-valuetext` is locale-aware.
- [ ] Indeterminate mode omits `aria-valuenow`.
- [ ] Linear and circular branches are pure presentations over one machine.
- [ ] Tests cover determinate, indeterminate, and callback behavior.
