---
adapter: leptos
component: time-field
category: date-time
source: components/date-time/time-field.md
source_foundation: foundation/08-adapter-leptos.md
---

# TimeField — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TimeField`](../../components/date-time/time-field.md) contract onto a Leptos 0.8.x component. The adapter adds Leptos-facing segment rendering, day-period parsing and IME cleanup, hidden-input form behavior, and mount-gated focus semantics.

## 2. Public Adapter API

```rust
#[component]
pub fn TimeField(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<Time>>,
    #[prop(optional)] default_value: Option<Time>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] granularity: TimeGranularity,
    #[prop(optional)] hour_cycle: HourCycle,
    #[prop(optional)] hide_time_zone: bool,
    #[prop(optional)] min_value: Option<Time>,
    #[prop(optional)] max_value: Option<Time>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] invalid: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] force_leading_zeros: bool,
    #[prop(optional)] on_value_change: Option<Callback<Option<Time>>>,
) -> impl IntoView
```

The adapter mirrors the full core prop surface, keeps segments machine-owned, and uses `Signal<Option<Time>>` only for controlled value sync.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `TimeField` contract, including hour-cycle and timezone-display options.
- Part parity: full parity with `Root`, `Label`, `FieldGroup`, repeated `Segment` and `Literal`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none beyond Leptos-specific signal, ref, and composition-event wiring.

## 4. Part Mapping

| Core part / structure                        | Required?   | Adapter rendering target           | Ownership     | Attr source              | Notes                                                         |
| -------------------------------------------- | ----------- | ---------------------------------- | ------------- | ------------------------ | ------------------------------------------------------------- |
| `Root`, `Label`, `FieldGroup`                | required    | wrapper + label + grouped segments | adapter-owned | time-field API attrs     | Carries field-level invalid and required state.               |
| editable `Segment`                           | required    | `<div role="spinbutton">`          | adapter-owned | `api.segment_attrs(...)` | Hour, minute, second, day period, and optional timezone name. |
| `Literal`                                    | conditional | `<span aria-hidden="true">`        | adapter-owned | `api.literal_attrs(...)` | Colons and spacing remain structural.                         |
| `Description`, `ErrorMessage`, `HiddenInput` | conditional | prose blocks + hidden input        | adapter-owned | corresponding API attrs  | Hidden input serializes ISO time only.                        |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                                           | Adapter-owned attrs                             | Consumer attrs          | Merge order             | Ownership notes                       |
| ------------- | ---------------------------------------------------- | ----------------------------------------------- | ----------------------- | ----------------------- | ------------------------------------- |
| `FieldGroup`  | role, ids, described-by, invalid, required           | utility-field attrs                             | wrapper decoration only | core semantic attrs win | Group semantics remain adapter-owned. |
| `Segment`     | spinbutton aria values and readonly/disabled markers | inputmode, composition handlers, focus handlers | class/style only        | core aria attrs win     | Consumers may style, not replace.     |
| `HiddenInput` | ISO time value and name                              | none                                            | none                    | adapter controls fully  | Form bridge is adapter-owned.         |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: utility `field` and `form` contexts for shared label, invalid, and reset behavior.
- Missing required context behavior: `ArsProvider` locale/ICU context follows the adapter-foundation policy in `08-adapter-leptos.md` §13; utility contexts are optional.
- Composition rule: wrappers may compose around the field, but editable segments remain adapter-owned and ordered by the machine.

## 7. Prop Sync and Event Mapping

- Controlled value sync uses a post-mount watcher and the core pending-controlled-value behavior.
- `hour_cycle`, `granularity`, `hide_time_zone`, and `force_leading_zeros` require segment rebuilding when reactive wrappers expose them post-mount.
- Numeric keys, day-period text input, IME composition, focus, blur, increment, decrement, clear, and next/previous focus normalize onto the machine event set.
- Hidden-input form bridging mirrors only committed time-of-day values in ISO `HH:MM[:SS]` form.

## 8. Registration and Cleanup Contract

| Registered entity             | Registration trigger       | Identity key     | Cleanup trigger                   | Cleanup action                | Notes                                            |
| ----------------------------- | -------------------------- | ---------------- | --------------------------------- | ----------------------------- | ------------------------------------------------ |
| segment node refs             | segment mount              | composite        | segment rerender or unmount       | drop node handle              | Needed for focus traversal.                      |
| type-ahead / day-period timer | segment text entry         | instance-derived | commit, blur, or cleanup          | cancel timer and clear handle | Covers CJK day-period disambiguation.            |
| IME composition guard         | composition start          | instance-derived | composition end, blur, or cleanup | clear composing flag          | Prevent stale partial commits.                   |
| hidden-input bridge           | render with `name` present | instance-derived | name removal or cleanup           | remove hidden input node      | Keeps form participation scoped to the instance. |

## 9. Ref and Node Contract

| Target part / node     | Ref required? | Ref owner     | Node availability                  | Composition rule                           | Notes                               |
| ---------------------- | ------------- | ------------- | ---------------------------------- | ------------------------------------------ | ----------------------------------- |
| `FieldGroup`           | yes           | adapter-owned | required after mount               | may compose with utility field wrapper ref | Used for focusout checks.           |
| editable `Segment`     | yes           | adapter-owned | required after mount               | keyed by segment kind                      | Focus traversal uses refs, not ids. |
| hidden input           | no            | adapter-owned | always structural, handle optional | no composition                             | Structural only.                    |
| timezone literal nodes | no            | adapter-owned | always structural, handle optional | no composition                             | Pure render output.                 |

## 10. State Machine Boundary Rules

- Machine-owned state: committed time value, segment bundle, focus state, buffered input, day-period parsing, and clamping.
- Adapter-local derived bookkeeping: node refs and timer handles.
- Forbidden local mirrors: segment text, segment values, or active hour-cycle formatting.
- Allowed snapshot-read contexts: render, key handlers, blur cleanup, and hidden-input serialization.

## 11. Callback Payload Contract

| Callback                                           | Payload source             | Payload shape               | Timing                           | Cancelable? | Notes                                      |
| -------------------------------------------------- | -------------------------- | --------------------------- | -------------------------------- | ----------- | ------------------------------------------ |
| `on_value_change`                                  | machine-derived snapshot   | `Option<Time>`              | after a committed segment update | no          | Partial day-period buffers do not fire it. |
| `on_segment_focus_change` when exposed by wrappers | normalized adapter payload | `{ kind: DateSegmentKind }` | after focus transition           | no          | Optional wrapper callback only.            |
| form-reset diagnostics callback when exposed       | none                       | `()`                        | after reset handling             | no          | Diagnostics only.                          |

## 12. Failure and Degradation Rules

- Invalid hour-cycle input: `fail fast` in debug and `degrade gracefully` to locale default in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the adapter foundation defines a documented fallback.
- Missing segment ref during focus traversal: `no-op`.
- Unsupported timezone-display host behavior: `warn and ignore` and render time zone text only if the machine already produced it.
- SSR-only absence of composition APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from segment kind and literal index.
- Timer and IME guards use `instance-derived` identity.
- Hidden input uses `instance-derived` identity.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render emits stable grouped segments, literals, and hidden-input structure.
- Composition events, timers, and ref-based focus traversal are client-only.
- Hydration must preserve segment order and literal positions exactly.
- Hidden-input value may render on the server because it is derived from committed state.

## 15. Performance Constraints

- Avoid rebuilding all segments when only one committed segment changes.
- Keep timers instance-scoped and cancel stale day-period disambiguation work.
- Reuse segment refs for stable segment kinds.
- Do not replace the entire field group on every keystroke.

## 16. Implementation Dependencies

| Dependency                             | Required?   | Dependency type         | Why it matters                                                                              |
| -------------------------------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context       | required    | context contract        | Localized numerals, day-period labels, segment order, and inherited direction depend on it. |
| segment formatting and parsing helpers | required    | shared helper           | Localized numerals and day-period parsing must stay aligned with the core machine.          |
| utility `field` / `form` contracts     | required    | composition contract    | Label, invalid, description, and reset semantics should stay consistent.                    |
| hidden-input helper                    | recommended | shared helper           | Keeps ISO time serialization uniform.                                                       |
| IME / day-period helper                | recommended | behavioral prerequisite | Needed for CJK disambiguation and composition safety.                                       |

## 17. Recommended Implementation Sequence

1. Wire the machine and render root, label, and field group.
2. Render segments and literals from the machine snapshot.
3. Add refs and focus traversal.
4. Add day-period parsing, IME guards, and controlled sync.
5. Add hidden-input form bridging and finalize utility integration.

## 18. Anti-Patterns

- Do not special-case AM/PM input outside the machine’s parsing rules.
- Do not serialize transient buffered text into the hidden input.
- Do not mirror localized display text in adapter-local state.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the hidden input always uses ISO time-of-day formatting rather than localized text.
- Consumers may assume hour-cycle and day-period behavior come from the machine, not ad hoc adapter logic.
- Consumers must not assume time-zone text implies instant-in-time semantics.

## 20. Platform Support Matrix

| Capability / behavior                   | Browser client | SSR            | Notes                                         |
| --------------------------------------- | -------------- | -------------- | --------------------------------------------- |
| segmented editing and focus traversal   | full support   | SSR-safe empty | Interaction starts after hydration.           |
| hidden-input form bridge                | full support   | full support   | Server output may include committed ISO time. |
| day-period composition and IME handling | full support   | SSR-safe empty | Browser composition events are client-only.   |
| time-zone segment hiding / showing      | full support   | full support   | Pure render behavior when configured.         |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                            |
| ---------------------------------------- | -------------------- | ------------------- | ------------------------------------------------ |
| invalid hour-cycle configuration         | fail fast            | degrade gracefully  | Fall back to locale default cycle.               |
| inconsistent min/max bounds              | debug warning        | warn and ignore     | Keep machine clamping on the valid side.         |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only through the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                          | Reused by                                      | Notes                        |
| ------------------------- | --------- | --------------------------------------- | ---------------------------------------------- | ---------------------------- |
| segment formatting helper | yes       | Produce localized text and placeholders | `date-field`, `time-field`, `date-time-picker` | Shared concept only.         |
| hidden-input helper       | yes       | Serialize committed values to ISO       | all form-participating date-time controls      | Avoid drift across adapters. |
| IME / day-period helper   | yes       | Own timers and composition gating       | `time-field`, `date-time-picker`               | Must stay instance-scoped.   |

## 23. Framework-Specific Behavior

- Leptos composition events should gate machine text entry while `is_composing` is true.
- Controlled sync must dispatch machine updates rather than mutating segments directly.
- Segment refs should be keyed by logical segment kind, not DOM order alone.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<time_field::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <div {..machine.derive(|api| api.field_group_attrs()).get()}>
            <For each=segments key=segment_key let:segment>
                <Segment machine=machine segment=segment />
            </For>
        </div>
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust
fn time_segment(machine: UseMachineReturn<time_field::Machine>, segment: DateSegment) -> impl IntoView {
    if segment.is_editable {
        view! { <div {..machine.derive(move |api| api.segment_attrs(&segment.kind)).get()} /> }
    } else {
        view! { <span {..machine.derive(move |api| api.literal_attrs(segment.index)).get()}>{segment.text}</span> }
    }
}
```

## 26. Adapter Invariants

- Only one editable segment is tabbable at a time.
- Hidden input reflects only committed machine state.
- IME and day-period timers are canceled on blur and cleanup.

## 27. Accessibility and SSR Notes

- Day-period segments keep `aria-valuetext` aligned with localized display labels.
- Field-level description and error wiring remain hydration-stable.
- SSR never emits interactive focus behavior, only stable segment structure and committed value text.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core segmented time-field contract.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include hidden-input bridging, IME cleanup, day-period parsing hooks, and ref-based focus traversal.

## 29. Test Scenarios

1. Hour, minute, second, and day-period segments render with correct localized order and literals.
2. Keyboard increment, decrement, and focus traversal update the correct segment.
3. CJK day-period input and composition buffering commit correctly.
4. Controlled value changes during editing respect pending-buffer semantics.
5. Hidden input reflects committed ISO time and resets correctly.

## 30. Test Oracle Notes

- `DOM attrs`: assert spinbutton aria attrs and hidden-input value.
- `machine state`: verify committed `Time` value after keyboard and composition input.
- `callback order`: verify `on_value_change` fires only after committed updates.
- `cleanup side effects`: verify timers are canceled on blur and unmount.
- Cheap recipe: type an ambiguous CJK day-period prefix, blur, and assert one resolved commit.

## 31. Implementation Checklist

- [ ] Segment refs drive focus traversal.
- [ ] Day-period parsing follows machine rules and respects IME composition.
- [ ] Hidden input serializes only committed ISO time.
- [ ] No local mirror of segment text or value exists outside the machine snapshot.
- [ ] Utility `field` / `form` integration remains additive.
