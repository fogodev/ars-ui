---
adapter: dioxus
component: time-field
category: date-time
source: components/date-time/time-field.md
source_foundation: foundation/09-adapter-dioxus.md
---

# TimeField — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TimeField`](../../components/date-time/time-field.md) contract onto a Dioxus 0.7.x component. The adapter adds Dioxus-facing segment rendering, day-period parsing and IME cleanup, web-only form bridge behavior, and host fallback rules.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TimeFieldProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<Time>>>,
    #[props(optional)]
    pub default_value: Option<Time>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub granularity: TimeGranularity,
    pub hour_cycle: HourCycle,
    #[props(default = false)]
    pub hide_time_zone: bool,
    #[props(optional)]
    pub min_value: Option<Time>,
    #[props(optional)]
    pub max_value: Option<Time>,
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
    pub on_value_change: Option<EventHandler<Option<Time>>>,
}

#[component]
pub fn TimeField(props: TimeFieldProps) -> Element
```

Plain props remain preferred; the controlled value becomes a `Signal` only when post-mount sync is required.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `TimeField` contract.
- Part parity: full parity with `Root`, `Label`, `FieldGroup`, repeated `Segment` and `Literal`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input form participation is web-only; desktop/mobile hosts treat it as not applicable.

## 4. Part Mapping

| Core part / structure         | Required?   | Adapter rendering target         | Ownership     | Attr source              | Notes                                                          |
| ----------------------------- | ----------- | -------------------------------- | ------------- | ------------------------ | -------------------------------------------------------------- |
| `Root`, `Label`, `FieldGroup` | required    | wrapper, label, grouped segments | adapter-owned | time-field API attrs     | Field-level accessibility state lives here.                    |
| editable `Segment`            | required    | host focusable segment node      | adapter-owned | `api.segment_attrs(...)` | Hour, minute, second, day period, and optional time-zone text. |
| `Literal`                     | conditional | host text node wrapper           | adapter-owned | `api.literal_attrs(...)` | Colons and spacing remain structural.                          |
| `Description`, `ErrorMessage` | conditional | prose nodes                      | adapter-owned | corresponding API attrs  | Host equivalents may replace DOM attrs.                        |
| `HiddenInput`                 | conditional | web-only hidden input            | adapter-owned | API attrs                | Omitted on desktop/mobile.                                     |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                                     | Adapter-owned attrs      | Consumer attrs  | Merge order             | Ownership notes                            |
| ------------- | ---------------------------------------------- | ------------------------ | --------------- | ----------------------- | ------------------------------------------ |
| `FieldGroup`  | ids, invalid, required, described-by           | host classes             | decoration only | core semantic attrs win | Non-web hosts may map semantics logically. |
| `Segment`     | spinbutton attrs and readonly/disabled markers | inputmode, host handlers | decoration only | core semantic attrs win | Consumer attrs must not replace segments.  |
| `HiddenInput` | name and ISO time value                        | none                     | none            | adapter controls fully  | Web only.                                  |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: utility `field` / `form` on web and logical equivalents elsewhere.
- Missing required context behavior: `ArsProvider` locale/ICU context follows the foundation policy in `09-adapter-dioxus.md` §16.
- Composition rule: wrappers may compose around the field, but segment order and time parsing remain machine-owned.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync dispatches the machine update path after mount.
- `hour_cycle`, `granularity`, `hide_time_zone`, and `force_leading_zeros` rebuild segments when wrappers make them reactive.
- Numeric keys, day-period text input, composition events, focus, blur, increment, decrement, clear, and traversal normalize onto the machine event set.
- Hidden-input bridging mirrors committed ISO time only on web.

## 8. Registration and Cleanup Contract

| Registered entity             | Registration trigger   | Identity key     | Cleanup trigger                   | Cleanup action                | Notes                                 |
| ----------------------------- | ---------------------- | ---------------- | --------------------------------- | ----------------------------- | ------------------------------------- |
| segment node refs             | segment mount          | composite        | segment rerender or unmount       | drop node handle              | Needed for focus traversal.           |
| type-ahead / day-period timer | segment text entry     | instance-derived | commit, blur, or cleanup          | cancel timer and clear handle | Covers CJK day-period disambiguation. |
| IME composition guard         | composition start      | instance-derived | composition end, blur, or cleanup | clear composing flag          | Prevent stale partial commits.        |
| hidden-input bridge           | web render with `name` | instance-derived | name removal or cleanup           | remove hidden input node      | Not applicable on desktop/mobile.     |

## 9. Ref and Node Contract

| Target part / node  | Ref required? | Ref owner     | Node availability                  | Composition rule                   | Notes                                 |
| ------------------- | ------------- | ------------- | ---------------------------------- | ---------------------------------- | ------------------------------------- |
| `FieldGroup`        | yes           | adapter-owned | required after mount               | may compose with utility field ref | Used for focusout checks.             |
| editable `Segment`  | yes           | adapter-owned | required after mount               | keyed by segment kind              | Traversal uses live refs.             |
| hidden input        | no            | adapter-owned | always structural, handle optional | no composition                     | Web only.                             |
| host fallback nodes | no            | adapter-owned | always structural, handle optional | no composition                     | Desktop/mobile logical wrappers only. |

## 10. State Machine Boundary Rules

- Machine-owned state: committed time value, segment bundle, focused segment, buffered input, day-period parsing, and clamping.
- Adapter-local derived bookkeeping: node refs, timer handles, and host fallback handles.
- Forbidden local mirrors: segment text, segment values, or hour-cycle formatting.
- Allowed snapshot-read contexts: render, handlers, and web form-bridge serialization.

## 11. Callback Payload Contract

| Callback                                     | Payload source             | Payload shape               | Timing                         | Cancelable? | Notes                                      |
| -------------------------------------------- | -------------------------- | --------------------------- | ------------------------------ | ----------- | ------------------------------------------ |
| `on_value_change`                            | machine-derived snapshot   | `Option<Time>`              | after committed segment update | no          | Partial day-period buffers do not fire it. |
| `on_segment_focus_change` when exposed       | normalized adapter payload | `{ kind: DateSegmentKind }` | after focus transition         | no          | Optional wrapper-only surface.             |
| form-reset diagnostics callback when exposed | none                       | `()`                        | after reset handling           | no          | Diagnostics only.                          |

## 12. Failure and Degradation Rules

- Invalid hour-cycle input: `fail fast` in debug and `degrade gracefully` to locale default in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Missing segment ref during traversal: `no-op`.
- Unsupported timezone host behavior: `warn and ignore`.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from segment kind and literal index.
- Timer and IME guards use `instance-derived` identity.
- Hidden input uses `instance-derived` identity on web and `not applicable` elsewhere.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable grouped segments, literals, and hidden-input structure.
- Composition events, timers, and ref-based traversal are client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve segment order and literal positions on web.

## 15. Performance Constraints

- Avoid rebuilding all segments when only one committed segment changes.
- Keep timers instance-scoped and cancel stale day-period work promptly.
- Reuse segment refs for stable segment kinds.
- Do not replace the field group on every keystroke.

## 16. Implementation Dependencies

| Dependency                             | Required?   | Dependency type         | Why it matters                                                                              |
| -------------------------------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context       | required    | context contract        | Localized numerals, day-period labels, segment order, and inherited direction depend on it. |
| segment formatting and parsing helpers | required    | shared helper           | Localized numerals and day-period parsing must stay aligned.                                |
| utility `field` / `form` contracts     | required    | composition contract    | Shared label, invalid, description, and reset semantics.                                    |
| hidden-input helper                    | recommended | shared helper           | Keeps web-only serialization uniform.                                                       |
| IME / day-period helper                | recommended | behavioral prerequisite | Needed for CJK disambiguation and composition safety.                                       |

## 17. Recommended Implementation Sequence

1. Wire the machine and render root, label, and field group.
2. Render segments and literals from the machine snapshot.
3. Add refs and traversal.
4. Add day-period parsing, IME guards, and controlled sync.
5. Add web-only hidden-input bridge and host fallback notes.

## 18. Anti-Patterns

- Do not special-case AM/PM input outside the machine’s parsing rules.
- Do not serialize transient buffered text into the hidden input.
- Do not mirror localized display text in adapter-local state.

## 19. Consumer Expectations and Guarantees

- Consumers may assume hidden input uses ISO time-of-day formatting on web only.
- Consumers may assume hour-cycle and day-period behavior come from the machine.
- Consumers must not assume time-zone text implies instant-in-time semantics.

## 20. Platform Support Matrix

| Capability / behavior                   | Web          | Desktop        | Mobile         | SSR            | Notes                                      |
| --------------------------------------- | ------------ | -------------- | -------------- | -------------- | ------------------------------------------ |
| segmented editing and focus traversal   | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on host focus affordances. |
| hidden-input form bridge                | full support | not applicable | not applicable | full support   | Browser forms exist only on web.           |
| day-period composition and IME handling | full support | full support   | fallback path  | SSR-safe empty | Host composition APIs may vary.            |
| time-zone segment hiding / showing      | full support | full support   | full support   | full support   | Pure render behavior when configured.      |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| invalid hour-cycle configuration         | fail fast            | degrade gracefully  | Fall back to locale default.                 |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep machine clamping.                       |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                           | Reused by                          | Notes                        |
| ------------------------- | --------- | ---------------------------------------- | ---------------------------------- | ---------------------------- |
| segment formatting helper | yes       | Produce localized text and placeholders  | segmented adapters                 | Shared concept only.         |
| hidden-input helper       | yes       | Serialize committed values to ISO on web | form-participating controls on web | Avoid drift.                 |
| IME / day-period helper   | yes       | Own timers and composition gating        | `time-field`, `date-time-picker`   | Must remain instance-scoped. |

## 23. Framework-Specific Behavior

- Dioxus effects should dispatch machine updates for controlled values rather than patch segments directly.
- Stable hook positions matter for repeated derivations; extract child components when needed.
- Desktop/mobile hosts may use logical group semantics when browser aria attrs are unavailable.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<time_field::Machine>(props);

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
    pub machine: UseMachineReturn<time_field::Machine>,
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
- IME and day-period timers are canceled on blur and cleanup.

## 27. Accessibility and SSR Notes

- Day-period segments keep display text aligned with equivalent accessible text on supported hosts.
- Field-level description and error wiring remain hydration-stable on web.
- SSR never emits interactive focus behavior, only stable committed structure.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core time-field contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only.
- Traceability note: adapter-owned concerns promoted here include hidden-input policy, IME cleanup, day-period parsing hooks, and host fallback behavior.

## 29. Test Scenarios

1. Hour, minute, second, and day-period segments render with the correct localized order and literals.
2. Keyboard increment, decrement, and traversal update the correct segment.
3. CJK day-period input and composition buffering commit correctly.
4. Controlled updates during editing respect pending-buffer semantics.
5. Web hidden input reflects committed ISO time and resets correctly.

## 30. Test Oracle Notes

- `DOM attrs`: assert spinbutton and hidden-input attrs on web.
- `machine state`: verify committed `Time` value across hosts.
- `callback order`: verify `on_value_change` fires only after committed updates.
- `cleanup side effects`: verify timers clear on blur and unmount.
- Cheap recipe: type an ambiguous CJK day-period prefix, blur, and assert one resolved commit.

## 31. Implementation Checklist

- [ ] Segment refs drive focus traversal.
- [ ] Day-period parsing follows machine rules and respects IME composition.
- [ ] Hidden input is web-only and serializes only committed ISO time.
- [ ] No local mirror of segment text or value exists outside the machine snapshot.
- [ ] Utility `field` / `form` integration remains additive.
