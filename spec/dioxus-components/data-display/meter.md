---
adapter: dioxus
component: meter
category: data-display
source: components/data-display/meter.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Meter — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Meter`](../../components/data-display/meter.md) contract onto a Dioxus 0.7.x component. The adapter must prefer native `<meter>` semantics when available, document the fallback host strategy, and make zone-change announcements, locale value text, and RTL visual handling explicit across Dioxus platforms.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct MeterProps {
    #[props(optional)]
    pub id: Option<String>,
    pub value: f64,
    #[props(optional)]
    pub min: Option<f64>,
    #[props(optional)]
    pub max: Option<f64>,
    #[props(optional)]
    pub low: Option<f64>,
    #[props(optional)]
    pub high: Option<f64>,
    #[props(optional)]
    pub optimum: Option<f64>,
    #[props(optional)]
    pub label: Option<String>,
    #[props(optional)]
    pub value_text: Option<String>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<meter::Messages>,
}

#[component]
pub fn Meter(props: MeterProps) -> Element
```

The adapter exposes the core meter inputs directly. `value_text` is an adapter override; otherwise the adapter derives localized `aria-valuetext` from messages and locale.

## 3. Mapping to Core Component Contract

- Props parity: full parity with explicit adapter `value_text` override support.
- Part parity: full parity for `Root`, `Label`, `Track`, `Range`, and `ValueText`.
- Traceability note: this spec promotes host element fallback, `aria-valuetext`, zone-crossing announcements, and RTL rendering guidance from the agnostic spec.

## 4. Part Mapping

| Core part   | Required?                    | Adapter rendering target              | Ownership     | Attr source              | Notes                                                              |
| ----------- | ---------------------------- | ------------------------------------- | ------------- | ------------------------ | ------------------------------------------------------------------ |
| `Root`      | required                     | `<meter>` preferred, `<div>` fallback | adapter-owned | `api.root_attrs()`       | Fallback keeps ARIA semantics explicit.                            |
| `Label`     | optional                     | `<label>` or `<span>`                 | adapter-owned | `api.label_attrs()`      | Associate with `Root` through native or ARIA wiring.               |
| `Track`     | required for custom fallback | `<div>`                               | adapter-owned | `api.track_attrs()`      | Native `<meter>` may visually replace this in web implementations. |
| `Range`     | required for custom fallback | `<div>`                               | adapter-owned | `api.range_attrs()`      | Width reflects computed percent.                                   |
| `ValueText` | optional                     | `<span>`                              | adapter-owned | `api.value_text_attrs()` | Visible mirror of `aria-valuetext`; hidden from AT.                |

## 5. Attr Merge and Ownership Rules

- Core attrs come from the API, including `role`, numeric bounds, native meter attrs, and `data-ars-segment`.
- The adapter owns fallback semantics when `<meter>` is not used, including `role="meter"` or documented fallback `role="progressbar"` when required for broader support.
- Consumer classes and styles may decorate the host but must not drop `aria-valuenow`, `aria-valuetext`, `value`, `min`, `max`, `low`, `high`, or `optimum`.
- For custom fallback rendering, adapter-owned track and range wrappers must remain explicit.

## 6. Composition / Context Contract

`Meter` is standalone. It may resolve locale and messages from the nearest `ArsProvider`, and may optionally use a shared live-announcement helper for zone changes, but it does not publish adapter context.

## 7. Prop Sync and Event Mapping

- All props are render-derived; there is no state machine.
- When `value`, `low`, `high`, or `optimum` change, the adapter recomputes percent, segment, and `aria-valuetext`.
- Zone-change announcements are adapter-derived from previous and next segment values; there are no user-facing machine events.

## 8. Registration and Cleanup Contract

- If a live announcer helper is used for zone crossings, the adapter registers at mount and reuses it rather than allocating per update.
- No observers are required. Cleanup consists only of releasing any adapter-owned announcement handle.

## 9. Ref and Node Contract

The root ref is optional. It is only needed when the implementation chooses runtime capability detection between native and fallback host behavior.

## 10. State Machine Boundary Rules

Meter has no state machine. Segment and percent are pure derived values and must not be mirrored into mutable local state except for previous-segment announcement bookkeeping.

## 11. Callback Payload Contract

No public adapter callback is required. Internal announcement helpers observe segment transitions only.

## 12. Failure and Degradation Rules

| Condition                                           | Policy             | Notes                                                                          |
| --------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------ |
| native `<meter>` unavailable on the active renderer | fallback path      | Render custom `Track` and `Range` wrappers with explicit ARIA attrs.           |
| `role="meter"` unsupported in target AT stack       | degrade gracefully | Fall back to `role="progressbar"` while preserving descriptive value text.     |
| invalid numeric bounds                              | fail fast          | Normalize or reject impossible `min >= max` configurations during development. |

## 13. Identity and Key Policy

The adapter owns one root identity and, in fallback mode, stable `Track` and `Range` child identities. Switching between native and fallback hosts must not occur after mount unless the component is recreated.

## 14. SSR and Client Boundary Rules

- SSR may emit either the preferred host or a documented fallback host, but the choice must be stable through hydration.
- Zone-change announcements are client-only and never run during SSR.
- Locale-derived `aria-valuetext` must match server and client formatting inputs.

## 15. Performance Constraints

- Percent and segment calculations must be memoized from numeric props.
- The adapter must not trigger duplicate live announcements when value changes stay within the same segment.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type      | Why it must exist first                                  | Notes                                                |
| ------------------------ | ----------- | -------------------- | -------------------------------------------------------- | ---------------------------------------------------- |
| formatting helper        | recommended | i18n contract        | Builds `aria-valuetext` and visible value text.          | Shared with `progress` and `stat`.                   |
| live announcement helper | optional    | accessibility helper | Announces zone changes without re-creating hidden nodes. | Used only when segment transitions are communicated. |

## 17. Recommended Implementation Sequence

1. Resolve locale/messages and derive percent plus segment.
2. Choose native or fallback host strategy.
3. Render `Root` with numeric and accessibility attrs.
4. Render fallback `Track` and `Range` if needed.
5. Add zone-change announcement bookkeeping.

## 18. Anti-Patterns

- Do not use color alone to communicate segment meaning.
- Do not change host strategy after mount.
- Do not expose visible `ValueText` to AT when `aria-valuetext` on `Root` already carries the meaning.

## 19. Consumer Expectations and Guarantees

- Consumers may assume segment semantics follow the core HTML-meter-style algorithm.
- Consumers may assume fallback rendering preserves accessible value text.
- Consumers must not assume the native `<meter>` element is always used on every platform.

## 20. Platform Support Matrix

| Capability / behavior                        | Web           | Desktop       | Mobile        | SSR            | Notes                                                 |
| -------------------------------------------- | ------------- | ------------- | ------------- | -------------- | ----------------------------------------------------- |
| native `<meter>` host with locale value text | full support  | fallback path | fallback path | full support   | Non-web renderers may need custom host markup.        |
| custom fallback track/range rendering        | fallback path | full support  | full support  | SSR-safe empty | Used when native behavior or styling is insufficient. |
| zone-change live announcement                | client-only   | client-only   | client-only   | SSR-safe empty | Announcement helpers run only after mount.            |

## 21. Debug Diagnostics and Production Policy

| Condition                            | Debug build behavior | Production behavior | Notes                                                           |
| ------------------------------------ | -------------------- | ------------------- | --------------------------------------------------------------- |
| invalid bounds or threshold ordering | fail fast            | warn and ignore     | Normalize only when the resulting semantics remain unambiguous. |
| missing localized value text inputs  | debug warning        | degrade gracefully  | Fall back to numeric formatting only.                           |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                   | Reused by            | Notes                                            |
| ------------------------ | --------- | ------------------------------------------------ | -------------------- | ------------------------------------------------ |
| formatting helper        | required  | Produce `aria-valuetext` and visible value text. | `progress`, `stat`   | Should centralize locale-sensitive formatting.   |
| live announcement helper | optional  | Announce segment crossings.                      | `table`, `tag-group` | Reuse a single hidden live region when possible. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should keep the host strategy branch stable and memoize percent/segment derivation so unrelated re-renders do not re-announce zone changes.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct MeterSketchProps {
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

#[component]
pub fn Meter(props: MeterSketchProps) -> Element {
    let api = use_memo(move || meter::Api::new(meter::Props { value: props.value, min: props.min, max: props.max, ..Default::default() }));
    let strategy = use_style_strategy();

    rsx! {
        meter {
            ..attr_map_to_dioxus(api().root_attrs(), &strategy, None).attrs,
            div {
                ..attr_map_to_dioxus(api().track_attrs(), &strategy, None).attrs,
                div { ..attr_map_to_dioxus(api().range_attrs(), &strategy, None).attrs }
            }
        }
    }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton is required beyond the canonical sketch for the stateless path, but fallback implementations must preserve the explicit `Root`/`Track`/`Range` split documented above.

## 26. Adapter Invariants

- The host choice is stable across hydration.
- `Root` always exposes numeric bounds and a descriptive `aria-valuetext`.
- Zone-change announcements only fire when the semantic segment actually changes.

## 27. Accessibility and SSR Notes

- Prefer native `<meter>` semantics on web, but fallback markup must stay equivalently descriptive.
- Mirror direction visually in RTL without changing the underlying segment algorithm.
- Keep `ValueText` hidden from AT when its meaning duplicates `aria-valuetext`.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit fallback semantics.
- Intentional deviations: the adapter may use `role="progressbar"` as a documented accessibility fallback when platform support for `role="meter"` is insufficient.

## 29. Test Scenarios

1. Native-host rendering exposes the correct numeric and descriptive attrs.
2. Fallback host rendering keeps `Track` and `Range` semantics plus percent width.
3. Zone transitions announce exactly once when the segment changes.

## 30. Test Oracle Notes

- Preferred oracle for host semantics: inspect `Root` attrs and rendered tag name.
- Preferred oracle for fallback rendering: DOM snapshot of `Track` and `Range`.
- Verification recipe: rerender through several threshold crossings and assert one live announcement per actual segment change.

## 31. Implementation Checklist

- [ ] Native-vs-fallback host choice is documented and hydration-stable.
- [ ] `aria-valuetext` is locale-derived and never omitted.
- [ ] Track/range fallback markup is explicit.
- [ ] Segment changes do not rely on color alone.
- [ ] Tests cover native host, fallback host, and zone announcements.
