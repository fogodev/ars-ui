---
adapter: dioxus
component: date-field
category: date-time
source: components/date-time/date-field.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DateField — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateField`](../../components/date-time/date-field.md) contract onto a Dioxus 0.7.x component. The adapter adds Dioxus-facing segmented rendering, node ownership, host-specific form-bridge policy, IME cleanup, and SSR-safe focus behavior.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct DateFieldProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<CalendarDate>>>,
    #[props(optional)]
    pub default_value: Option<CalendarDate>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub calendar: CalendarSystem,
    pub granularity: DateGranularity,
    #[props(optional)]
    pub min_value: Option<CalendarDate>,
    #[props(optional)]
    pub max_value: Option<CalendarDate>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(default = false)]
    pub force_leading_zeros: bool,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<CalendarDate>>>,
}

#[component]
pub fn DateField(props: DateFieldProps) -> Element
```

Plain props remain preferred; the controlled value becomes a `Signal` only when post-mount sync is required.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `DateField` contract.
- Part parity: full parity with `Root`, `Label`, `FieldGroup`, repeated `Segment` and `Literal`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input form bridging is web-only; desktop/mobile hosts treat form participation as not applicable.

## 4. Part Mapping

| Core part / structure         | Required?   | Adapter rendering target         | Ownership     | Attr source              | Notes                                               |
| ----------------------------- | ----------- | -------------------------------- | ------------- | ------------------------ | --------------------------------------------------- |
| `Root`, `Label`, `FieldGroup` | required    | wrapper, label, grouped segments | adapter-owned | date-field API attrs     | Group carries field-level accessibility state.      |
| editable `Segment`            | required    | host focusable segment node      | adapter-owned | `api.segment_attrs(...)` | One per editable logical segment.                   |
| `Literal`                     | conditional | host text node wrapper           | adapter-owned | `api.literal_attrs(...)` | One per separator in the resolved order.            |
| `Description`, `ErrorMessage` | conditional | prose nodes                      | adapter-owned | corresponding API attrs  | Host equivalents may replace DOM attrs outside web. |
| `HiddenInput`                 | conditional | web `<input type="hidden">` only | adapter-owned | API attrs                | Omitted on desktop/mobile.                          |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                                                      | Adapter-owned attrs            | Consumer attrs          | Merge order                    | Ownership notes                            |
| ------------------ | --------------------------------------------------------------- | ------------------------------ | ----------------------- | ------------------------------ | ------------------------------------------ |
| `FieldGroup`       | ids, labels, invalid, required, described-by                    | host classes and utility attrs | wrapper decoration only | core semantic attrs win on web | Non-web hosts may map semantics logically. |
| editable `Segment` | spinbutton aria values, tabindex, readonly and disabled markers | inputmode, host handlers       | class/style only        | core semantic attrs win        | Consumer attrs must not replace segments.  |
| `HiddenInput`      | name and ISO value                                              | none                           | none                    | adapter controls fully         | Rendered only on web.                      |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: utility `field` / `form` contexts on web and logical equivalents on other hosts.
- Missing required context behavior: `ArsProvider` locale/ICU context follows the foundation policy in `09-adapter-dioxus.md` §16; utility contexts are optional.
- Composition rule: wrappers may compose around `DateField`, but editable segments remain adapter-owned and ordered by the machine.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync dispatches the machine update path after mount.
- `disabled`, `readonly`, `invalid`, and form-derived state sync immediately to segment attrs.
- `locale`, `calendar`, `granularity`, `segment_order`, and `force_leading_zeros` require segment rebuilding when wrappers make them reactive.
- Keydown, focus, click, composition, and blur normalize onto the machine’s focus, increment, decrement, clear, and type-ahead events.
- Hidden-input form bridging mirrors committed value only and exists on web only.

## 8. Registration and Cleanup Contract

| Registered entity     | Registration trigger           | Identity key     | Cleanup trigger                     | Cleanup action                | Notes                             |
| --------------------- | ------------------------------ | ---------------- | ----------------------------------- | ----------------------------- | --------------------------------- |
| segment node refs     | segment mount                  | composite        | segment rerender or unmount         | drop node handle              | Needed for focus traversal.       |
| type-ahead timer      | text entry                     | instance-derived | commit, blur, overwrite, or cleanup | cancel timer and clear handle | Prevent stale buffered commits.   |
| IME composition guard | composition start              | instance-derived | composition end, blur, or cleanup   | clear composing flag          | Prevent premature commits.        |
| hidden-input bridge   | web render with `name` present | instance-derived | name removal or cleanup             | remove hidden input node      | Not applicable on desktop/mobile. |

## 9. Ref and Node Contract

| Target part / node  | Ref required? | Ref owner     | Node availability                  | Composition rule                   | Notes                                 |
| ------------------- | ------------- | ------------- | ---------------------------------- | ---------------------------------- | ------------------------------------- |
| `FieldGroup`        | yes           | adapter-owned | required after mount               | may compose with utility field ref | Used for group-level focus checks.    |
| editable `Segment`  | yes           | adapter-owned | required after mount               | one ref per segment kind           | Traversal uses live refs, not ids.    |
| hidden input        | no            | adapter-owned | always structural, handle optional | no composition                     | Web only.                             |
| host fallback nodes | no            | adapter-owned | always structural, handle optional | no composition                     | Desktop/mobile logical wrappers only. |

## 10. State Machine Boundary Rules

- Machine-owned state: committed value, segment bundle, focused segment, buffered input, and clamping.
- Adapter-local derived bookkeeping: node refs, timer handles, and host fallback handles.
- Forbidden local mirrors: segment text, segment value, or focused segment.
- Allowed snapshot-read contexts: render, handlers, and form-bridge serialization on web.

## 11. Callback Payload Contract

| Callback                                     | Payload source             | Payload shape               | Timing                 | Cancelable? | Notes                                |
| -------------------------------------------- | -------------------------- | --------------------------- | ---------------------- | ----------- | ------------------------------------ |
| `on_value_change`                            | machine-derived snapshot   | `Option<CalendarDate>`      | after committed update | no          | Does not fire for transient buffers. |
| `on_segment_focus_change` when exposed       | normalized adapter payload | `{ kind: DateSegmentKind }` | after focus transition | no          | Optional wrapper-only surface.       |
| form-reset diagnostics callback when exposed | none                       | `()`                        | after reset handling   | no          | Diagnostics only.                    |

## 12. Failure and Degradation Rules

- Invalid custom `segment_order`: `fail fast` in debug and `degrade gracefully` to locale order in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Missing segment ref during traversal: `no-op`.
- Impossible bounds: `warn and ignore`.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from segment kind and literal index.
- Type-ahead and IME guards use `instance-derived` identity.
- Hidden input uses `instance-derived` identity on web and `not applicable` elsewhere.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable root, field group, segments, literals, description, error, and hidden-input structure.
- Refs, timers, IME handling, and focus movement are client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve segment order and literal positions on web.

## 15. Performance Constraints

- Avoid recreating segment refs for unaffected segments.
- Keep type-ahead timers instance-scoped.
- Rebuild segments only when display-driving props change.
- Do not replace the entire field group when one segment updates.

## 16. Implementation Dependencies

| Dependency                           | Required?   | Dependency type         | Why it matters                                                                         |
| ------------------------------------ | ----------- | ----------------------- | -------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context     | required    | context contract        | Segment order, localized placeholders, numerals, and inherited direction depend on it. |
| segment parsing / formatting helpers | required    | shared helper           | Month names, numerals, and placeholders must stay aligned.                             |
| utility `field` / `form` contracts   | required    | composition contract    | Shared label, invalid, description, and reset semantics.                               |
| hidden-input helper                  | recommended | shared helper           | Keeps web-only form serialization uniform.                                             |
| IME guard helper                     | recommended | behavioral prerequisite | Prevents premature commits during composition.                                         |

## 17. Recommended Implementation Sequence

1. Wire the machine and render root, label, and field group.
2. Render editable and literal segments from the machine snapshot.
3. Add refs and focus traversal.
4. Add controlled sync, timers, and IME guards.
5. Add web-only hidden-input bridging and host fallback notes.

## 18. Anti-Patterns

- Do not mirror segment text or focused segment in local Dioxus state.
- Do not serialize partially typed segment buffers into the hidden input.
- Do not reorder segments independently of the machine’s resolved bundle.

## 19. Consumer Expectations and Guarantees

- Consumers may assume every editable segment remains machine-derived and keyboard reachable.
- Consumers may assume the hidden input contains committed ISO date data on web only.
- Consumers must not assume desktop or mobile hosts expose browser-native form semantics.

## 20. Platform Support Matrix

| Capability / behavior                 | Web          | Desktop        | Mobile         | SSR            | Notes                                      |
| ------------------------------------- | ------------ | -------------- | -------------- | -------------- | ------------------------------------------ |
| segmented editing and focus traversal | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on host focus affordances. |
| hidden-input form bridge              | full support | not applicable | not applicable | full support   | Browser forms exist only on web.           |
| IME composition handling              | full support | full support   | fallback path  | SSR-safe empty | Host composition APIs may vary.            |
| locale-driven segment rendering       | full support | full support   | full support   | full support   | Deterministic once locale is resolved.     |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| invalid custom `segment_order`           | fail fast            | degrade gracefully  | Fall back to locale order.                   |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep machine clamping.                       |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                           | Reused by                                      | Notes                        |
| ------------------------- | --------- | ---------------------------------------- | ---------------------------------------------- | ---------------------------- |
| segment formatting helper | yes       | Produce localized text and placeholders  | `date-field`, `time-field`, `date-time-picker` | Shared concept only.         |
| hidden-input helper       | yes       | Serialize committed values to ISO on web | form-participating controls on web             | Avoid drift.                 |
| IME / type-ahead helper   | yes       | Own timers and composition gating        | segmented adapters                             | Must remain instance-scoped. |

## 23. Framework-Specific Behavior

- Dioxus effects should dispatch machine updates for controlled values rather than patch segment state directly.
- Stable hook positions matter for repeated derivations; extract child components when needed.
- Desktop/mobile hosts may use logical group semantics when DOM aria attrs are unavailable.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<date_field::Machine>(props);

rsx! {
    div { ..root_attrs,
        Segments { machine }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
struct SegmentProps {
    pub machine: UseMachineReturn<date_field::Machine>,
    pub segment: DateSegment,
}

#[component]
fn Segment(props: SegmentProps) -> Element {
    rsx! { div { ..props.machine.derive(move |api| api.segment_attrs(&props.segment.kind)).read().clone() } }
}
```

## 26. Adapter Invariants

- Only one editable segment is tabbable at a time.
- Web hidden input reflects only committed machine state.
- Timers and IME guards are canceled on blur and cleanup.

## 27. Accessibility and SSR Notes

- Segment value attrs must stay aligned with localized display text on web and with equivalent host semantics elsewhere.
- Error and description wiring remain stable across hydration on web.
- SSR never emits interactive focus behavior, only stable committed structure.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core date-field contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only.
- Traceability note: adapter-owned concerns promoted here include form-bridge policy, IME cleanup, segment refs, and host fallback behavior.

## 29. Test Scenarios

1. Locale-derived segment order renders the expected segment bundle.
2. Keyboard increment, decrement, and traversal update the correct segment.
3. Controlled value changes during active editing respect pending-buffer semantics.
4. IME composition suppresses premature commits.
5. Web hidden input reflects committed ISO value and resets correctly.

## 30. Test Oracle Notes

- `DOM attrs`: assert group, spinbutton, tabindex, and hidden-input attrs on web.
- `machine state`: verify focused segment and committed value across hosts.
- `callback order`: verify `on_value_change` fires only after commit.
- `cleanup side effects`: verify timers clear on blur and unmount.
- Cheap recipe: start a buffered edit, blur, and assert one commit plus one web hidden-input update.

## 31. Implementation Checklist

- [ ] Segment refs drive focus traversal.
- [ ] No local mirror of segment data exists outside the machine snapshot.
- [ ] IME and type-ahead cleanup runs on blur and unmount.
- [ ] Hidden input is web-only and serializes only committed ISO dates.
- [ ] Utility `field` / `form` integration remains additive.
