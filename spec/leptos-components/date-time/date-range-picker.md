---
adapter: leptos
component: date-range-picker
category: date-time
source: components/date-time/date-range-picker.md
source_foundation: foundation/08-adapter-leptos.md
---

# DateRangePicker — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateRangePicker`](../../components/date-time/date-range-picker.md) contract onto a Leptos 0.8.x component. The adapter adds explicit twin-field composition, overlay ownership, range-calendar bridging, split-or-single hidden-input strategy, and focus return behavior.

## 2. Public Adapter API

```rust
#[component]
pub fn DateRangePicker(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<DateRange>>,
    #[prop(optional)] default_value: Option<DateRange>,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] min: Option<CalendarDate>,
    #[prop(optional)] max: Option<CalendarDate>,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional)] close_on_select: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] start_name: Option<String>,
    #[prop(optional)] end_name: Option<String>,
    #[prop(optional)] on_value_change: Option<Callback<Option<DateRange>>>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
) -> impl IntoView
```

The adapter owns the shared control row, overlay shell, and hidden-input strategy, and composes two internal `DateField` children plus one internal `RangeCalendar`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range-picker contract.
- Part parity: full parity with `Root`, `Label`, `Control`, `StartInput`, `Separator`, `EndInput`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none beyond explicit utility-layer overlay composition.

## 4. Part Mapping

| Core part / structure      | Required?              | Adapter rendering target          | Ownership                 | Attr source                                | Notes                                   |
| -------------------------- | ---------------------- | --------------------------------- | ------------------------- | ------------------------------------------ | --------------------------------------- |
| root / label / control     | required               | wrapper + control group           | adapter-owned             | range-picker API attrs                     | Control is the shared accessible group. |
| `StartInput` / `EndInput`  | required               | two internal `DateField` adapters | adapter-owned composition | derived child props                        | Child fields stay machine-owned.        |
| `Trigger` / `ClearTrigger` | required / conditional | buttons                           | adapter-owned             | picker attrs plus utility button semantics | Separate from the child field groups.   |
| `Positioner` / `Content`   | conditional            | overlay shell                     | adapter-owned             | picker attrs plus overlay utilities        | Content hosts `RangeCalendar`.          |
| hidden input(s)            | conditional            | one or three hidden inputs        | adapter-owned             | API attrs                                  | Strategy matches range-field behavior.  |

## 5. Attr Merge and Ownership Rules

| Target node               | Core attrs                             | Adapter-owned attrs                    | Consumer attrs        | Merge order             | Ownership notes                       |
| ------------------------- | -------------------------------------- | -------------------------------------- | --------------------- | ----------------------- | ------------------------------------- |
| control group             | group role, labelled-by, state markers | wrapper classes, utility attrs         | decoration only       | core semantic attrs win | Control remains adapter-owned.        |
| child inputs              | child field attrs                      | derived min/max and aria labels        | child decoration only | child semantics win     | Parent does not replace child groups. |
| trigger / clear / content | popup linkage, disabled, dialog attrs  | overlay hooks and utility button attrs | class/style only      | core semantic attrs win | Overlay shell remains adapter-owned.  |
| hidden inputs             | single or split ISO value(s)           | none                                   | none                  | adapter controls fully  | Mutually exclusive strategies.        |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus overlay helpers for dismissable, focus scope, and environment / layer allocation.
- Optional consumed contexts: utility `field` / `form`.
- Missing required context behavior: overlay helper absence `fail fast` in debug; `ArsProvider` locale/ICU context follows its documented policy in `08-adapter-leptos.md` §13.
- Composition rule: the parent machine owns range state and open state; child `DateField` and `RangeCalendar` adapters only receive derived props and emit bridge callbacks.

## 7. Prop Sync and Event Mapping

- Controlled `value` and `open` sync after mount via parent-machine watchers.
- Child field commits normalize onto the parent range machine.
- Child `RangeCalendar` completion normalizes onto the same parent machine and closes the overlay when `close_on_select=true`.
- Trigger, clear, outside dismiss, and Escape normalize onto open-state transitions.
- Hidden-input bridging mirrors only committed normalized range state.

## 8. Registration and Cleanup Contract

| Registered entity            | Registration trigger      | Identity key     | Cleanup trigger           | Cleanup action                                         | Notes                             |
| ---------------------------- | ------------------------- | ---------------- | ------------------------- | ------------------------------------------------------ | --------------------------------- |
| child field bridge callbacks | child mount               | composite        | child rerender or cleanup | drop stale callback bridge                             | Avoid stale start/end routing.    |
| range-calendar bridge        | overlay open              | instance-derived | overlay close or cleanup  | unregister selection callback                          | Instance-scoped only.             |
| overlay helper bundle        | overlay open              | instance-derived | overlay close or cleanup  | dispose dismissable, focus-scope, and measurement work | Prevent lingering listeners.      |
| hidden-input bridge(s)       | render with names present | composite        | name change or cleanup    | remove stale hidden inputs                             | Respect single vs split strategy. |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner           | Node availability    | Composition rule                         | Notes                            |
| ------------------ | ------------- | ------------------- | -------------------- | ---------------------------------------- | -------------------------------- |
| control group      | yes           | adapter-owned       | required after mount | may compose with utility field ref       | Needed for outside-focus checks. |
| trigger            | yes           | adapter-owned       | required after mount | may compose with utility button ref      | Used for focus return.           |
| overlay content    | yes           | adapter-owned       | client-only          | may compose with dismissable helper refs | Content exists only while open.  |
| child field groups | yes           | child adapter-owned | required after mount | child adapters compose their own refs    | Parent must not steal them.      |

## 10. State Machine Boundary Rules

- Machine-owned state: normalized range, active field, open state, and close-on-select behavior.
- Adapter-local derived bookkeeping: overlay helper handles, mounted refs, and child bridge handles.
- Forbidden local mirrors: range endpoints, active field, or open state outside the machine / controlled signal contract.
- Allowed snapshot-read contexts: render, child-prop derivation, overlay lifecycle, and hidden-input serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape       | Timing                       | Cancelable? | Notes                                                  |
| ----------------------------------------- | -------------------------- | ------------------- | ---------------------------- | ----------- | ------------------------------------------------------ |
| `on_value_change`                         | machine-derived snapshot   | `Option<DateRange>` | after committed range update | no          | Fires for child-field or calendar completion.          |
| `on_open_change`                          | normalized adapter payload | `bool`              | after open state commit      | no          | Fires for trigger, dismiss, and selection-close paths. |
| dismissal analytics callback when exposed | raw framework event        | framework event     | before close dispatch        | yes         | Optional wrapper-only surface.                         |

## 12. Failure and Degradation Rules

- Conflicting hidden-input naming strategy: `warn and ignore` split names when single `name` is present.
- Missing overlay helpers: `fail fast` in debug and `degrade gracefully` only when an inline fallback is explicitly supported.
- Missing child bridge ref during close-time focus return: `no-op`.
- Impossible range bounds: `warn and ignore` invalid bound input.
- SSR-only absence of browser overlay APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Child field instances use `composite` identity from base id plus `start` / `end`.
- Overlay shell uses `instance-derived` identity.
- Embedded range-calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden input bridge uses `composite` identity from the chosen submission strategy.

## 14. SSR and Client Boundary Rules

- Server render emits stable control, child field, separator, description, error, and hidden-input structure.
- Overlay helpers, content refs, and focus return are client-only.
- Hydration must preserve start-before-end field order and trigger placement.
- Hidden-input values may render on the server because they derive from committed range state.

## 15. Performance Constraints

- Keep overlay helpers mounted only while content is open.
- Reuse child field instances and range-calendar structure across open cycles when possible.
- Avoid rebuilding hidden-input strategy when names are unchanged.
- Keep bridge callbacks instance-scoped.

## 16. Implementation Dependencies

| Dependency                       | Required? | Dependency type      | Why it matters                                                                                     |
| -------------------------------- | --------- | -------------------- | -------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required  | context contract     | Child field formatting, range labels, overlay calendar text, and inherited direction depend on it. |
| `DateField` adapter              | required  | composition contract | Start and end fields should reuse the segmented contract.                                          |
| `RangeCalendar` adapter          | required  | composition contract | Overlay range selection should reuse the grid and preview contract.                                |
| utility overlay helpers          | required  | composition contract | Dismissal, focus return, and layer policy are adapter-owned concerns.                              |
| hidden-input helper              | required  | shared helper        | Submission strategy must stay aligned with `DateRangeField`.                                       |

## 17. Recommended Implementation Sequence

1. Render root, label, control, child fields, separator, and buttons.
2. Add controlled value and open watchers.
3. Mount `RangeCalendar` inside overlay content and bridge its events.
4. Add overlay helpers and focus return.
5. Add single and split hidden-input strategies.

## 18. Anti-Patterns

- Do not maintain separate range state inside child fields or the child calendar.
- Do not render both single and split hidden-input strategies simultaneously.
- Do not keep overlay helpers mounted after close.

## 19. Consumer Expectations and Guarantees

- Consumers may assume child fields and the child calendar stay synchronized through one parent source of truth.
- Consumers may assume hidden-input submission uses normalized committed range values only.
- Consumers must not assume overlay content remains mounted while the picker is closed.

## 20. Platform Support Matrix

| Capability / behavior                 | Browser client | SSR            | Notes                                                   |
| ------------------------------------- | -------------- | -------------- | ------------------------------------------------------- |
| control row and hidden-input bridge   | full support   | full support   | Server render may include committed ISO range value(s). |
| overlay positioning and dismissal     | full support   | SSR-safe empty | Browser APIs are client-only.                           |
| child field focus traversal           | full support   | SSR-safe empty | Requires mounted child refs.                            |
| range-calendar preview and completion | full support   | SSR-safe empty | Overlay content itself is client-mounted.               |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                    |
| ---------------------------------------- | -------------------- | ------------------- | ---------------------------------------- |
| conflicting hidden-input naming strategy | debug warning        | warn and ignore     | Prefer single `name`.                    |
| unsupported overlay helper absence       | fail fast            | degrade gracefully  | Only when inline fallback is documented. |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep machine clamping.                   |

## 22. Shared Adapter Helper Notes

| Helper concept        | Required? | Responsibility                                              | Reused by                               | Notes                    |
| --------------------- | --------- | ----------------------------------------------------------- | --------------------------------------- | ------------------------ |
| child field bridge    | yes       | Route start/end field commits into the parent range machine | `date-range-field`, `date-range-picker` | Internal only.           |
| overlay helper bundle | yes       | Dismissal, focus scope, and layer work                      | picker adapters                         | Shared concept only.     |
| hidden-input helper   | yes       | Serialize one interval or two endpoint inputs               | range field and range picker adapters   | Keep strategies aligned. |

## 23. Framework-Specific Behavior

- Leptos child bridge callbacks should dispatch parent machine events rather than patch parent signals directly.
- Overlay content refs and focus return run only after the overlay mounts.
- Child field props and calendar props should be derived lazily from the parent snapshot.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<date_range_picker::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <Control machine=machine />
        <Show when=move || machine.derive(|api| api.is_open()).get()>
            <Overlay machine=machine />
        </Show>
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust
fn overlay(machine: UseMachineReturn<date_range_picker::Machine>) -> impl IntoView {
    let calendar_props = machine.with_api_snapshot(|api| api.calendar_props());
    view! {
        <Positioner>
            <FocusScope>
                <Dismissable on_dismiss=move || machine.dispatch(date_range_picker::Event::Close)>
                    <RangeCalendar ..calendar_props />
                </Dismissable>
            </FocusScope>
        </Positioner>
    }
}
```

## 26. Adapter Invariants

- Parent machine state remains the only source of truth for range and open state.
- Single and split hidden-input strategies are mutually exclusive.
- Overlay cleanup always clears stale child calendar and dismissable bridges.

## 27. Accessibility and SSR Notes

- Control remains the shared accessible group for both child fields.
- Separator stays `aria-hidden`.
- SSR may render committed hidden-input values, but not active overlay helpers or mounted content.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range-picker contract and range-calendar composition.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include overlay lifecycle, child-field / child-calendar bridging, hidden-input strategy, and focus return.

## 29. Test Scenarios

1. Start and end child fields receive the correct labels, bounds, and callbacks.
2. Range-calendar completion updates committed range and closes content when configured.
3. Controlled open state stays in sync with popup linkage attrs and mounted content.
4. Single and split hidden-input strategies serialize normalized range values correctly.
5. Escape and outside dismiss close content and restore focus appropriately.

## 30. Test Oracle Notes

- `DOM attrs`: assert control-group attrs, trigger popup linkage, dialog attrs, and hidden-input value(s).
- `machine state`: verify committed range and open state after child-field edits and calendar completion.
- `callback order`: verify `on_value_change` and `on_open_change` ordering on completion-close.
- `cleanup side effects`: verify overlay helper and child bridge teardown on close.
- Cheap recipe: open, complete a range in the calendar, close, and assert normalized hidden-input values plus no lingering content node.

## 31. Implementation Checklist

- [ ] Parent machine remains the only source of truth for range and open state.
- [ ] Child `DateField` and `RangeCalendar` props derive from the parent snapshot.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Hidden-input strategies remain mutually exclusive.
- [ ] Focus return uses mounted refs without document queries.
