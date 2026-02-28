---
adapter: leptos
component: date-field
category: date-time
source: components/date-time/date-field.md
source_foundation: foundation/08-adapter-leptos.md
---

# DateField — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateField`](../../components/date-time/date-field.md) contract onto a Leptos 0.8.x component. The adapter adds the Leptos-facing segmented API, segment-node ownership, hidden-input form participation, IME cleanup, and SSR-safe focus behavior.

## 2. Public Adapter API

```rust
#[component]
pub fn DateField(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<CalendarDate>>,
    #[prop(optional)] default_value: Option<CalendarDate>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] calendar: CalendarSystem,
    #[prop(optional)] granularity: DateGranularity,
    #[prop(optional)] min_value: Option<CalendarDate>,
    #[prop(optional)] max_value: Option<CalendarDate>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] invalid: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] force_leading_zeros: bool,
    #[prop(optional)] on_value_change: Option<Callback<Option<CalendarDate>>>,
) -> impl IntoView
```

The adapter keeps segments machine-owned, uses `Signal<Option<CalendarDate>>` for controlled sync, and renders label, description, error, and hidden-input parts directly or via utility wrappers.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `DateField` props including locale, calendar, segment ordering, messages, and form participation.
- Part parity: full parity with `Root`, `Label`, `FieldGroup`, repeated `Segment` and `Literal`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none beyond Leptos-specific watcher, node-ref, and IME event integration.

## 4. Part Mapping

| Core part / structure                        | Required?   | Adapter rendering target            | Ownership     | Attr source                             | Notes                                                            |
| -------------------------------------------- | ----------- | ----------------------------------- | ------------- | --------------------------------------- | ---------------------------------------------------------------- |
| `Root` and `Label`                           | required    | outer wrapper and visible label     | adapter-owned | `api.root_attrs()`, `api.label_attrs()` | Utility `field` wrappers may decorate but not replace semantics. |
| `FieldGroup`                                 | required    | `<div role="group">`                | adapter-owned | `api.field_group_attrs()`               | Carries accessible name and described-by wiring.                 |
| editable `Segment`                           | required    | focusable `<div role="spinbutton">` | adapter-owned | `api.segment_attrs(...)`                | One per editable logical segment.                                |
| `Literal`                                    | conditional | `<span aria-hidden="true">`         | adapter-owned | `api.literal_attrs(...)`                | One per separator in the resolved segment order.                 |
| `Description`, `ErrorMessage`, `HiddenInput` | conditional | prose blocks + hidden form bridge   | adapter-owned | corresponding API attrs                 | Hidden input exists when form participation is enabled.          |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                                                         | Adapter-owned attrs                        | Consumer attrs          | Merge order                | Ownership notes                            |
| ------------------ | ------------------------------------------------------------------ | ------------------------------------------ | ----------------------- | -------------------------- | ------------------------------------------ |
| `FieldGroup`       | role, ids, aria-labelled/described-by, invalid, readonly, required | utility-field attrs, root classes          | wrapper decoration only | core semantic attrs win    | Consumers do not override `role="group"`.  |
| editable `Segment` | spinbutton aria values, tabindex, disabled and readonly markers    | inputmode, event handlers, data-part hooks | class/style only        | core aria and tabindex win | Segment semantics must survive decoration. |
| `HiddenInput`      | name and ISO value                                                 | none                                       | none                    | adapter controls fully     | Form bridge is adapter-owned.              |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus optional utility `field` / `form` contexts.
- Optional consumed contexts: shared invalid, disabled, readonly, and described-by context from utility wrappers.
- Missing required context behavior: `field` and `form` contexts are optional; `ArsProvider` locale/ICU context follows the adapter-foundation failure policy in `08-adapter-leptos.md` §13.
- Composition rule: higher-level wrappers may compose around `DateField`, but they must not replace or reorder individual segments.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync dispatches the machine update path after mount and must respect the core pending-controlled-value rules during active editing.
- `disabled`, `readonly`, `invalid`, and form-derived state sync immediately to segment attrs.
- `locale`, `calendar`, `granularity`, `segment_order`, and `force_leading_zeros` require segment rebuilding when wrappers make them reactive.
- Keydown, focus, click, composition, and focusout handlers normalize onto the machine’s focus, increment, decrement, clear, type-ahead, and blur events.
- Hidden-input form bridging mirrors the committed value only; partially typed buffers must not leak into form submission.

## 8. Registration and Cleanup Contract

| Registered entity     | Registration trigger       | Identity key     | Cleanup trigger                                | Cleanup action                | Notes                                    |
| --------------------- | -------------------------- | ---------------- | ---------------------------------------------- | ----------------------------- | ---------------------------------------- |
| segment node refs     | segment mount              | composite        | segment rerender or unmount                    | drop node handle              | Needed for focus traversal.              |
| type-ahead timer      | digit or month-name entry  | instance-derived | commit, blur, controlled overwrite, or cleanup | cancel timer and clear handle | Prevent stale buffered commits.          |
| IME composition guard | composition start          | instance-derived | composition end, blur, or cleanup              | clear composing flag          | Prevent mid-composition machine commits. |
| hidden input bridge   | render with `name` present | instance-derived | name removal or cleanup                        | remove hidden input node      | Keep form bridge instance-scoped.        |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule                           | Notes                                    |
| ------------------ | ------------- | ------------- | ---------------------------------- | ------------------------------------------ | ---------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition                             | Structural only.                         |
| `FieldGroup`       | yes           | adapter-owned | required after mount               | may compose with utility field wrapper ref | Needed for group-level focusout checks.  |
| editable `Segment` | yes           | adapter-owned | required after mount               | one ref per segment kind                   | Focus traversal uses live refs, not ids. |
| `HiddenInput`      | no            | adapter-owned | always structural, handle optional | no composition                             | Hidden form bridge only.                 |

## 10. State Machine Boundary Rules

- Machine-owned state: committed value, focused segment, rebuilt segments, type buffer, composing rules, and clamping behavior.
- Adapter-local derived bookkeeping: mounted refs and timer handles.
- Forbidden local mirrors: segment text, segment value, focused segment, or pending controlled value.
- Allowed snapshot-read contexts: render, key handlers, blur handling, and form-bridge serialization of committed value.

## 11. Callback Payload Contract

| Callback                                            | Payload source             | Payload shape               | Timing                           | Cancelable? | Notes                                |
| --------------------------------------------------- | -------------------------- | --------------------------- | -------------------------------- | ----------- | ------------------------------------ |
| `on_value_change`                                   | machine-derived snapshot   | `Option<CalendarDate>`      | after a committed segment update | no          | Does not fire for transient buffers. |
| `on_segment_focus_change` when exposed by wrappers  | normalized adapter payload | `{ kind: DateSegmentKind }` | after focus transition           | no          | Optional wrapper callback only.      |
| form-reset bridge callback when exposed by wrappers | none                       | `()`                        | after form reset handling        | no          | Diagnostics only.                    |

## 12. Failure and Degradation Rules

- Invalid custom `segment_order`: `fail fast` in debug and `degrade gracefully` to locale-derived order in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the adapter foundation defines a documented fallback.
- Missing segment ref during focus traversal: `no-op` and leave focus where it is.
- Unsupported prop combinations such as `min_value > max_value`: `warn and ignore` invalid bound input and keep machine clamping.
- SSR-only absence of browser composition and focus APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from `DateSegmentKind` plus occurrence index for literals.
- Type-ahead and IME guards use `instance-derived` identity.
- Hidden input uses `instance-derived` identity.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render emits stable root, field group, segment, literal, description, error, and hidden-input structure.
- Segment refs, timers, IME handling, and focus movement are client-only.
- Hydration must preserve segment order and literal positions exactly.
- Hidden-input value may render on the server because it is derived from committed machine state.

## 15. Performance Constraints

- Avoid recreating segment refs for unaffected segments.
- Keep type-ahead timers instance-scoped.
- Rebuild segments only when value or display-driving props actually change.
- Do not replace the entire field group when one segment value changes.

## 16. Implementation Dependencies

| Dependency                           | Required?   | Dependency type         | Why it matters                                                                         |
| ------------------------------------ | ----------- | ----------------------- | -------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context     | required    | context contract        | Segment order, localized placeholders, numerals, and inherited direction depend on it. |
| segment parsing / formatting helpers | required    | shared helper           | Month names, numerals, and localized placeholders must stay consistent.                |
| utility `field` / `form` contracts   | required    | composition contract    | Label, described-by, invalid, and reset semantics should stay uniform.                 |
| hidden-input helper                  | recommended | shared helper           | Keeps ISO serialization and form resets consistent.                                    |
| IME guard helper                     | recommended | behavioral prerequisite | Prevents premature commit during composition.                                          |

## 17. Recommended Implementation Sequence

1. Wire the machine and render the root, label, and field group.
2. Render editable and literal segments from the machine snapshot.
3. Add segment refs and focus traversal.
4. Add controlled sync, type-ahead timers, and IME guards.
5. Add hidden-input form bridging and finalize utility-context integration.

## 18. Anti-Patterns

- Do not mirror segment text or focused segment in a local Leptos signal.
- Do not serialize partially typed segment buffers into the hidden input.
- Do not reorder segments independently of the machine’s resolved segment bundle.

## 19. Consumer Expectations and Guarantees

- Consumers may assume every editable segment is machine-derived and keyboard reachable.
- Consumers may assume the hidden input contains only the committed ISO date.
- Consumers must not assume they can inject arbitrary children into the field group.

## 20. Platform Support Matrix

| Capability / behavior                 | Browser client | SSR            | Notes                                       |
| ------------------------------------- | -------------- | -------------- | ------------------------------------------- |
| segmented editing and focus traversal | full support   | SSR-safe empty | Interaction begins after hydration.         |
| hidden-input form bridge              | full support   | full support   | ISO value can render server-side.           |
| IME composition handling              | full support   | SSR-safe empty | Browser composition events are client-only. |
| locale-driven segment order rendering | full support   | full support   | Deterministic for a resolved locale.        |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                            |
| ---------------------------------------- | -------------------- | ------------------- | ------------------------------------------------ |
| invalid custom `segment_order`           | fail fast            | degrade gracefully  | Fall back to locale order.                       |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep machine clamping on the valid side.         |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only through the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                            | Reused by                                      | Notes                                     |
| ------------------------- | --------- | ----------------------------------------- | ---------------------------------------------- | ----------------------------------------- |
| segment formatting helper | yes       | Produce localized text and placeholders   | `date-field`, `time-field`, `date-time-picker` | Shared concept across segmented adapters. |
| hidden-input helper       | yes       | Serialize committed values to ISO strings | all form-participating date-time controls      | Keep resets consistent.                   |
| IME / type-ahead helper   | yes       | Own timers and composition gating         | `date-field`, `time-field`, `date-time-picker` | Must remain instance-scoped.              |

## 23. Framework-Specific Behavior

- Leptos watchers should dispatch machine updates for controlled value changes instead of patching segments directly.
- Segment refs should be stored in mount-safe containers keyed by segment kind.
- Focusout handling should use related-target checks at the field-group boundary rather than per-segment blur heuristics.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<date_field::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <label {..machine.derive(|api| api.label_attrs()).get()}>{label}</label>
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
fn segment(machine: UseMachineReturn<date_field::Machine>, segment: DateSegment) -> impl IntoView {
    if segment.is_editable {
        view! { <div {..machine.derive(move |api| api.segment_attrs(&segment.kind)).get()} /> }
    } else {
        view! { <span {..machine.derive(move |api| api.literal_attrs(segment.index)).get()}>{segment.text}</span> }
    }
}
```

## 26. Adapter Invariants

- Only one editable segment is tabbable at a time.
- The hidden input reflects only committed machine state.
- Type-ahead timers and IME composition guards are always canceled on blur and cleanup.

## 27. Accessibility and SSR Notes

- Segment `aria-valuenow` and `aria-valuetext` must stay aligned with localized display text.
- Error and description wiring occurs at the field-group level and must remain hydration-stable.
- SSR may render placeholders and committed hidden-input values, but not focus behavior.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core segmented date-field contract.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include hidden-input bridging, IME cleanup, segment refs, and controlled-sync timing.

## 29. Test Scenarios

1. Locale-derived segment order renders the expected editable and literal sequence.
2. Keyboard increment, decrement, and next/previous focus paths update the correct segment.
3. Controlled value changes during active editing respect pending-buffer semantics.
4. IME composition suppresses premature commits.
5. Hidden input reflects committed ISO value and resets correctly.

## 30. Test Oracle Notes

- `DOM attrs`: assert `role="group"`, spinbutton attrs, `tabindex`, and hidden-input value.
- `machine state`: verify focused segment and committed value after keyboard input.
- `callback order`: verify `on_value_change` fires only after machine commit.
- `cleanup side effects`: verify timers are canceled on blur and unmount.
- Cheap recipe: start a buffered edit, blur the field, and assert one commit plus one hidden-input update.

## 31. Implementation Checklist

- [ ] Segment refs drive focus traversal.
- [ ] No local mirror of segment data exists outside the machine snapshot.
- [ ] IME and type-ahead cleanup runs on blur and unmount.
- [ ] Hidden input serializes only committed ISO dates.
- [ ] Utility `field` / `form` integration stays additive and does not replace field-group semantics.
