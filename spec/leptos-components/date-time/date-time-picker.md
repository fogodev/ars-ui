---
adapter: leptos
component: date-time-picker
category: date-time
source: components/date-time/date-time-picker.md
source_foundation: foundation/08-adapter-leptos.md
---

# DateTimePicker — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateTimePicker`](../../components/date-time/date-time-picker.md) contract onto a Leptos 0.8.x component. The adapter adds the unified segmented control contract, calendar-overlay composition, hidden-input form bridge, cross-boundary focus traversal, and Leptos-specific cleanup rules.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn DateTimePicker(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<DateTime>>,
    #[prop(optional)] default_value: Option<DateTime>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] format: Option<String>,
    #[prop(optional)] visible_months: usize,
    #[prop(optional)] min_value: Option<DateTime>,
    #[prop(optional)] max_value: Option<DateTime>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] invalid: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] on_value_change: Option<Callback<Option<DateTime>>>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
) -> impl IntoView
```

The adapter keeps the unified machine from the agnostic spec, renders both date and time segment groups in one control row, and mounts a `Calendar` overlay for date selection only.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core date-time-picker contract.
- Part parity: full parity with `Root`, `Label`, `Control`, `DateSegmentGroup`, `TimeSegmentGroup`, repeated `Segment` and `Literal`, `Separator`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none beyond explicit overlay and ref ownership.

## 4. Part Mapping

| Core part / structure                   | Required?              | Adapter rendering target                | Ownership                 | Attr source                                | Notes                                         |
| --------------------------------------- | ---------------------- | --------------------------------------- | ------------------------- | ------------------------------------------ | --------------------------------------------- |
| root / label / control                  | required               | wrapper, label, unified control group   | adapter-owned             | date-time-picker API attrs                 | Control is the shared accessible group.       |
| `DateSegmentGroup` / `TimeSegmentGroup` | required               | two grouped bundles inside control      | adapter-owned             | group attrs + repeated segment attrs       | Segment traversal crosses the group boundary. |
| `Trigger` / `ClearTrigger`              | required / conditional | buttons                                 | adapter-owned             | picker attrs plus utility button semantics | Trigger opens date overlay only.              |
| overlay shell + `Calendar`              | conditional            | positioner, content, and child calendar | adapter-owned composition | derived calendar props                     | Only date selection lives in the overlay.     |
| `HiddenInput`                           | conditional            | `<input type="hidden">`                 | adapter-owned             | API attrs                                  | Mirrors committed ISO datetime only.          |

## 5. Attr Merge and Ownership Rules

| Target node                  | Core attrs                                    | Adapter-owned attrs                         | Consumer attrs      | Merge order                        | Ownership notes                           |
| ---------------------------- | --------------------------------------------- | ------------------------------------------- | ------------------- | ---------------------------------- | ----------------------------------------- |
| `Control` and segment groups | group roles, labels, invalid, required        | utility-field attrs                         | decoration only     | core semantic attrs win            | Structure stays adapter-owned.            |
| editable `Segment`           | spinbutton attrs, tabindex, readonly/disabled | focus handlers, composition handlers        | class/style only    | core aria and tabindex win         | Consumer attrs must not replace segments. |
| overlay shell                | dialog attrs and open state                   | dismissable, focus-scope, measurement hooks | visual classes only | adapter preserves dialog semantics | Overlay exists only while open.           |
| `HiddenInput`                | name and ISO datetime value                   | none                                        | none                | adapter controls fully             | Form bridge is adapter-owned.             |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus overlay utility helpers.
- Optional consumed contexts: utility `field` / `form`.
- Missing required context behavior: overlay helper absence `fail fast` in debug; `ArsProvider` locale/ICU context follows the foundation policy in `08-adapter-leptos.md` §13.
- Composition rule: the unified machine owns both date and time segments; the child `Calendar` receives only derived date props and emits selection callbacks back into that unified machine.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync updates the unified machine after mount.
- Segment input, IME composition, increment/decrement, and focus movement normalize directly onto the unified machine events.
- Trigger, Escape, outside dismiss, and calendar selection normalize onto open-state transitions.
- Calendar selection updates the date portion only; time segments remain machine-owned inline state.
- Hidden-input form bridging mirrors only the committed ISO datetime value.

## 8. Registration and Cleanup Contract

| Registered entity       | Registration trigger       | Identity key     | Cleanup trigger             | Cleanup action                                         | Notes                                |
| ----------------------- | -------------------------- | ---------------- | --------------------------- | ------------------------------------------------------ | ------------------------------------ |
| segment node refs       | segment mount              | composite        | segment rerender or cleanup | drop node handle                                       | Needed for cross-boundary traversal. |
| IME / type-ahead timers | segment text entry         | instance-derived | commit, blur, or cleanup    | cancel timer and clear handle                          | Covers date and time segments.       |
| overlay helper bundle   | overlay open               | instance-derived | overlay close or cleanup    | dispose dismissable, focus-scope, and measurement work | Instance-scoped only.                |
| hidden-input bridge     | render with `name` present | instance-derived | name removal or cleanup     | remove hidden input node                               | Keeps form participation scoped.     |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                    | Notes                                       |
| ------------------ | ------------- | ------------- | -------------------- | ----------------------------------- | ------------------------------------------- |
| `Control`          | yes           | adapter-owned | required after mount | may compose with utility field ref  | Used for group focus checks.                |
| editable `Segment` | yes           | adapter-owned | required after mount | keyed by segment kind               | Traversal crosses date/time group boundary. |
| `Trigger`          | yes           | adapter-owned | required after mount | may compose with utility button ref | Used for close-time focus return.           |
| `Content`          | yes           | adapter-owned | client-only          | may compose with overlay helpers    | Exists only while open.                     |

## 10. State Machine Boundary Rules

- Machine-owned state: committed datetime value, date and time segment bundles, open state, focused segment, and pending controlled value.
- Adapter-local derived bookkeeping: node refs, timers, overlay helper handles.
- Forbidden local mirrors: date-only value, time-only value, open state, or focused segment outside the machine.
- Allowed snapshot-read contexts: render, key handlers, overlay lifecycle, and hidden-input serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape      | Timing                          | Cancelable? | Notes                                                          |
| ----------------------------------------- | -------------------------- | ------------------ | ------------------------------- | ----------- | -------------------------------------------------------------- |
| `on_value_change`                         | machine-derived snapshot   | `Option<DateTime>` | after committed datetime update | no          | Date-only calendar changes combine with current time snapshot. |
| `on_open_change`                          | normalized adapter payload | `bool`             | after open state commit         | no          | Trigger, dismiss, and Escape share this path.                  |
| dismissal analytics callback when exposed | raw framework event        | framework event    | before close dispatch           | yes         | Optional wrapper-only surface.                                 |

## 12. Failure and Degradation Rules

- Missing overlay helpers: `fail fast` in debug and `degrade gracefully` only when an inline fallback is explicitly documented.
- Missing segment ref during traversal: `no-op`.
- Impossible min/max datetime bounds: `warn and ignore` invalid bound input and keep machine clamping.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- SSR-only absence of browser composition and overlay APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from logical segment kind and literal index.
- Overlay shell uses `instance-derived` identity.
- Child calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden input uses `instance-derived` identity.

## 14. SSR and Client Boundary Rules

- Server render emits stable control, segment groups, buttons, description, error, and hidden-input structure.
- Overlay helpers, calendar content refs, IME timers, and focus traversal are client-only.
- Hydration must preserve segment order and the separator between date and time groups.
- Hidden-input value may render on the server because it derives from committed machine state.

## 15. Performance Constraints

- Keep segment refs stable across ordinary segment updates.
- Reuse timers and cancel them promptly on blur and cleanup.
- Mount overlay helpers only while content is open.
- Do not rebuild the whole control row when only the date overlay opens or closes.

## 16. Implementation Dependencies

| Dependency                                  | Required? | Dependency type      | Why it matters                                                                                                 |
| ------------------------------------------- | --------- | -------------------- | -------------------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context            | required  | context contract     | Segment ordering, localized parsing and formatting, child calendar text, and inherited direction depend on it. |
| shared segment formatting / parsing helpers | required  | shared helper        | Date and time groups must stay aligned with the unified machine.                                               |
| utility overlay helpers                     | required  | composition contract | Dismissal, focus return, and layer policy are adapter-owned.                                                   |
| hidden-input helper                         | required  | shared helper        | Datetime ISO serialization should align with other form-participating adapters.                                |
| child calendar composition helper           | required  | composition contract | Date overlay selection must bridge into the unified machine.                                                   |

## 17. Recommended Implementation Sequence

1. Render root, label, unified control, segment groups, and buttons.
2. Add controlled value sync plus segment refs and traversal.
3. Add IME and type-ahead cleanup across date and time segments.
4. Mount child `Calendar` overlay and bridge date selection.
5. Add hidden-input bridging and finalize overlay cleanup.

## 18. Anti-Patterns

- Do not split date and time ownership into separate parent-level machines.
- Do not serialize partially edited datetime text into the hidden input.
- Do not keep overlay helpers mounted while closed.

## 19. Consumer Expectations and Guarantees

- Consumers may assume cross-boundary segment traversal is continuous across the date/time separator.
- Consumers may assume the hidden input contains a committed ISO datetime only.
- Consumers must not assume the calendar overlay owns time selection semantics.

## 20. Platform Support Matrix

| Capability / behavior             | Browser client | SSR            | Notes                                             |
| --------------------------------- | -------------- | -------------- | ------------------------------------------------- |
| unified segmented control         | full support   | full support   | Server render may include committed ISO datetime. |
| overlay positioning and dismissal | full support   | SSR-safe empty | Overlay helpers are client-only.                  |
| IME / type-ahead handling         | full support   | SSR-safe empty | Browser composition events are client-only.       |
| calendar date selection bridge    | full support   | SSR-safe empty | Overlay content itself is client-mounted.         |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                            |
| ---------------------------------------- | -------------------- | ------------------- | ------------------------------------------------ |
| unsupported overlay helper absence       | fail fast            | degrade gracefully  | Only with documented inline fallback.            |
| inconsistent min/max datetime bounds     | debug warning        | warn and ignore     | Keep machine-clamped behavior.                   |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only through the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                                  | Reused by                                      | Notes                |
| ------------------------- | --------- | ----------------------------------------------- | ---------------------------------------------- | -------------------- |
| segment formatting helper | yes       | Produce localized date and time segment bundles | `date-field`, `time-field`, `date-time-picker` | Shared concept only. |
| overlay helper bundle     | yes       | Dismissal, focus-scope, and layer work          | picker adapters                                | Shared concept only. |
| hidden-input helper       | yes       | Serialize committed ISO datetime                | form-participating pickers                     | Avoid drift.         |

## 23. Framework-Specific Behavior

- Leptos watchers should dispatch unified-machine updates for controlled value changes.
- Segment refs should be keyed by logical kind so traversal can cross the date/time boundary reliably.
- Overlay content refs and focus return run only after content mounts.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_time_picker::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <DateTimeControl machine=machine />
        <Show when=move || machine.derive(|api| api.is_open()).get()>
            <DateTimeOverlay machine=machine />
        </Show>
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust
fn overlay(machine: UseMachineReturn<date_time_picker::Machine>) -> impl IntoView {
    let calendar_props = machine.with_api_snapshot(|api| api.calendar_props());
    view! {
        <Positioner>
            <FocusScope>
                <Dismissable on_dismiss=move || machine.dispatch(date_time_picker::Event::Close)>
                    <Calendar ..calendar_props />
                </Dismissable>
            </FocusScope>
        </Positioner>
    }
}
```

## 26. Adapter Invariants

- The unified machine remains the only source of truth for committed datetime value, segment bundles, and open state.
- Hidden input reflects only committed ISO datetime state.
- Overlay cleanup always clears stale helper and focus-return work.

## 27. Accessibility and SSR Notes

- Date and time groups retain separate group labels inside one shared control.
- The separator between groups stays `aria-hidden`.
- SSR may render committed hidden-input values, but not active overlay helpers or focus traversal.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core unified date-time-picker contract.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include unified segment traversal, overlay lifecycle, child calendar bridging, and hidden-input serialization.

## 29. Test Scenarios

1. Date and time segment groups render in one control with continuous traversal across the separator.
2. Controlled datetime updates keep both groups and the hidden input in sync.
3. Calendar selection updates only the date portion and preserves time segments.
4. Escape and outside dismiss close the overlay and restore focus.
5. Hidden input reflects committed ISO datetime only.

## 30. Test Oracle Notes

- `DOM attrs`: assert group labels, spinbutton attrs, popup linkage, and hidden-input value.
- `machine state`: verify committed datetime after segment edits and calendar selection.
- `callback order`: verify `on_open_change` and `on_value_change` ordering on overlay-close paths.
- `cleanup side effects`: verify timers and overlay helpers clear on blur, close, and unmount.
- Cheap recipe: edit a time segment, open the calendar, select a date, close, and assert one ISO datetime hidden-input value.

## 31. Implementation Checklist

- [ ] Unified machine remains the only source of truth.
- [ ] Segment refs support cross-boundary traversal.
- [ ] Calendar selection bridges only the date portion of the datetime value.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Hidden input serializes only committed ISO datetime state.
