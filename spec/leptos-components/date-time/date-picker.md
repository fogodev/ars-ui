---
adapter: leptos
component: date-picker
category: date-time
source: components/date-time/date-picker.md
source_foundation: foundation/08-adapter-leptos.md
---

# DatePicker — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DatePicker`](../../components/date-time/date-picker.md) contract onto a Leptos 0.8.x component. The adapter adds the Leptos-facing input and overlay API, utility-layer composition, hidden-input form bridge, focus return, dismissal policy, and SSR-safe overlay behavior.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn DatePicker(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<CalendarDate>>,
    #[prop(optional)] default_value: Option<CalendarDate>,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional)] default_open: bool,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] format: Option<String>,
    #[prop(optional)] min: Option<CalendarDate>,
    #[prop(optional)] max: Option<CalendarDate>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] invalid: bool,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] open_on_click: bool,
    #[prop(optional)] close_on_select: bool,
    #[prop(optional)] visible_months: usize,
    #[prop(optional)] on_value_change: Option<Callback<Option<CalendarDate>>>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
) -> impl IntoView
```

The adapter may render either a text input or a composed `DateField` control, but the overlay, trigger, content, and hidden-input parts remain adapter-owned.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core picker contract, including controlled value, controlled open state, formatting, and bounds.
- Part parity: full parity with `Root`, `Label`, `Control`, `Input`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: none at the contract level; utility overlay composition is made explicit here.

## 4. Part Mapping

| Core part / structure              | Required?              | Adapter rendering target    | Ownership     | Attr source                                    | Notes                                                     |
| ---------------------------------- | ---------------------- | --------------------------- | ------------- | ---------------------------------------------- | --------------------------------------------------------- |
| root / label / control             | required               | wrapper, label, control row | adapter-owned | picker API attrs                               | Control may host a text input or segmented field variant. |
| `Input`, `Trigger`, `ClearTrigger` | required / conditional | input and buttons           | adapter-owned | picker API attrs plus utility button semantics | Trigger and clear remain separate interactive nodes.      |
| `Positioner` + `Content`           | conditional            | overlay shell               | adapter-owned | picker API attrs plus overlay utilities        | Content hosts `Calendar`.                                 |
| `HiddenInput`                      | conditional            | `<input type="hidden">`     | adapter-owned | picker API attrs                               | Mirrors committed ISO date only.                          |

## 5. Attr Merge and Ownership Rules

| Target node                | Core attrs                                  | Adapter-owned attrs                         | Consumer attrs          | Merge order                              | Ownership notes                                |
| -------------------------- | ------------------------------------------- | ------------------------------------------- | ----------------------- | ---------------------------------------- | ---------------------------------------------- |
| `Input`                    | value, popup linkage, described-by, invalid | event handlers, input type, placeholder     | wrapper decoration only | core aria, value, and controls attrs win | Consumers do not replace the input node.       |
| `Trigger` / `ClearTrigger` | labels, disabled state, expanded state      | utility button attrs, dismissal hooks       | class/style only        | core semantic attrs win                  | Trigger remains adapter-owned.                 |
| overlay shell              | ids, state, dialog attrs                    | positioning, focus-scope, dismissable hooks | visual classes only     | adapter preserves dialog semantics       | Positioner and content are not consumer-owned. |
| `HiddenInput`              | name and ISO value                          | none                                        | none                    | adapter controls fully                   | Form bridge remains adapter-owned.             |

## 6. Composition / Context Contract

- Provided contexts: none.
- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus utility overlay contexts for environment and layer allocation when used.
- Optional consumed contexts: utility `field` / `form` contexts.
- Missing required context behavior: overlay utility dependencies `fail fast` in debug when a required helper is unavailable; `ArsProvider` locale/ICU context follows the adapter-foundation policy in `08-adapter-leptos.md` §13.
- Composition rule: `DatePicker` composes one `Calendar` instance inside `Content`; wrapper code must not fork value or open ownership away from the picker machine.

## 7. Prop Sync and Event Mapping

- Controlled `value` and controlled `open` sync after mount via dedicated watchers.
- Text input changes normalize onto parse-and-select machine transitions; segmented variants normalize through the field machine into the picker machine.
- Trigger click, trigger keydown, outside dismiss, Escape, and selection complete normalize onto `Open`, `Close`, and `Toggle`.
- Hidden-input form bridging mirrors committed selected value only.
- Focus return after closing the overlay prefers the input; when close came from Escape on content, returning to the trigger is allowed if documented by the wrapper.

## 8. Registration and Cleanup Contract

| Registered entity           | Registration trigger       | Identity key     | Cleanup trigger                          | Cleanup action                       | Notes                                        |
| --------------------------- | -------------------------- | ---------------- | ---------------------------------------- | ------------------------------------ | -------------------------------------------- |
| dismissable boundary        | overlay open               | instance-derived | overlay close or cleanup                 | unregister outside-interaction hooks | Prevent stale close callbacks.               |
| focus scope                 | overlay open               | instance-derived | overlay close or cleanup                 | unregister trapped focus bookkeeping | Picker content should not leak focus guards. |
| positioner measurement work | overlay open               | instance-derived | overlay close, layout change, or cleanup | dispose measurement / observer work  | Instance-scoped only.                        |
| hidden-input bridge         | render with `name` present | instance-derived | name removal or cleanup                  | remove hidden input node             | Keep form bridge scoped.                     |

## 9. Ref and Node Contract

| Target part / node     | Ref required? | Ref owner                       | Node availability    | Composition rule                                 | Notes                                             |
| ---------------------- | ------------- | ------------------------------- | -------------------- | ------------------------------------------------ | ------------------------------------------------- |
| `Input`                | yes           | adapter-owned                   | required after mount | may compose with utility field ref               | Used for focus return and open-on-click behavior. |
| `Trigger`              | yes           | adapter-owned                   | required after mount | may compose with utility button ref              | Needed for Escape-close fallback.                 |
| `Content`              | yes           | adapter-owned                   | client-only          | may compose with dismissable/focus-scope helpers | Overlay content does not exist while closed.      |
| embedded calendar root | yes           | adapter-owned via child adapter | required after mount | child adapter owns grid refs                     | Needed for open-time focus handoff.               |

## 10. State Machine Boundary Rules

- Machine-owned state: selected value, input text, parsed date, open state, and close-on-select behavior.
- Adapter-local derived bookkeeping: overlay helper handles, mounted refs, and measurement work.
- Forbidden local mirrors: selected date, parsed date, or open state outside the machine / controlled signal contract.
- Allowed snapshot-read contexts: rendering, overlay open / close hooks, and form-bridge serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape          | Timing                  | Cancelable? | Notes                                                  |
| ----------------------------------------- | -------------------------- | ---------------------- | ----------------------- | ----------- | ------------------------------------------------------ |
| `on_value_change`                         | machine-derived snapshot   | `Option<CalendarDate>` | after value commit      | no          | Fires for typed or grid-selected values.               |
| `on_open_change`                          | normalized adapter payload | `bool`                 | after open state commit | no          | Fires for trigger, dismiss, and selection-close paths. |
| dismissal analytics callback when exposed | raw framework event        | framework event        | before close dispatch   | yes         | Optional wrapper-only surface.                         |

## 12. Failure and Degradation Rules

- Invalid `visible_months`: `fail fast` in debug and `degrade gracefully` to `1` in production.
- Missing overlay helper infrastructure: `fail fast` in debug and `degrade gracefully` to inline content only when the wrapper explicitly supports that fallback.
- Missing input or trigger ref during close-time focus return: `no-op`.
- Impossible controlled props such as both unsupported segmented and text-input variants at once: `warn and ignore`.
- SSR-only absence of positioning and dismissable browser APIs: `SSR-safe empty`.

## 13. Identity and Key Policy

- Overlay shell uses `instance-derived` identity.
- Embedded calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden input uses `instance-derived` identity.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render emits root, label, control, input, trigger, description, error, and hidden-input structure.
- Overlay positioning, focus scope, dismissable behavior, and content refs are client-only.
- Hydration must preserve control structure whether the picker is initially open or closed.
- Hidden-input value may render on the server because it is derived from committed state.

## 15. Performance Constraints

- Avoid mounting overlay helpers while closed.
- Reuse the embedded calendar instance shape across open cycles when possible.
- Keep measurement and dismissable work instance-scoped.
- Do not rebuild the entire control row when only open state changes.

## 16. Implementation Dependencies

| Dependency                              | Required?   | Dependency type      | Why it matters                                                                                   |
| --------------------------------------- | ----------- | -------------------- | ------------------------------------------------------------------------------------------------ |
| `ArsProvider` locale/ICU context        | required    | context contract     | Input parsing, display formatting, embedded calendar text, and inherited direction depend on it. |
| utility `button`                        | required    | composition contract | Trigger and clear button semantics should stay aligned.                                          |
| utility `dismissable` and `focus-scope` | required    | composition contract | Overlay dismissal and focus return are adapter-owned concerns.                                   |
| utility `field` / `form`                | required    | composition contract | Label, invalid, description, and reset behavior stay consistent.                                 |
| positioner helper                       | recommended | shared helper        | Keeps overlay placement incremental and disposable.                                              |

## 17. Recommended Implementation Sequence

1. Wire the picker machine and render root, label, control, input, and buttons.
2. Add controlled value and open watchers.
3. Mount the embedded `Calendar` in overlay content.
4. Add dismissable, focus-scope, and focus-return behavior.
5. Add hidden-input bridging and finalize field/form integration.

## 18. Anti-Patterns

- Do not fork selected date ownership between the picker and the embedded calendar.
- Do not keep overlay helpers mounted while the picker is closed.
- Do not serialize typed but unparsed input text into the hidden input.

## 19. Consumer Expectations and Guarantees

- Consumers may assume closing the picker leaves committed value and hidden-input state in sync.
- Consumers may assume the trigger and input remain separate nodes with independent semantics.
- Consumers must not assume overlay content exists in the DOM while the picker is closed.

## 20. Platform Support Matrix

| Capability / behavior                            | Browser client | SSR            | Notes                                          |
| ------------------------------------------------ | -------------- | -------------- | ---------------------------------------------- |
| control row rendering and hidden-input bridge    | full support   | full support   | Server render may include committed ISO value. |
| overlay positioning and dismissal                | full support   | SSR-safe empty | Browser APIs are client-only.                  |
| focus return between input, trigger, and content | full support   | SSR-safe empty | Requires mounted refs.                         |
| embedded calendar multi-month rendering          | full support   | SSR-safe empty | Content itself is overlay-mounted.             |

## 21. Debug Diagnostics and Production Policy

| Condition                          | Debug build behavior | Production behavior | Notes                                              |
| ---------------------------------- | -------------------- | ------------------- | -------------------------------------------------- |
| invalid `visible_months`           | fail fast            | degrade gracefully  | Clamp to one month.                                |
| unsupported overlay helper absence | fail fast            | degrade gracefully  | Only when inline fallback is documented.           |
| conflicting controlled-open props  | debug warning        | warn and ignore     | Keep the explicit controlled signal authoritative. |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required? | Responsibility                                  | Reused by                                              | Notes                        |
| --------------------------- | --------- | ----------------------------------------------- | ------------------------------------------------------ | ---------------------------- |
| overlay helper bundle       | yes       | Dismissal, focus scope, and layer work          | all picker adapters                                    | Shared concept only.         |
| hidden-input helper         | yes       | ISO serialization for committed values          | all form-participating pickers                         | Keep reset behavior aligned. |
| calendar composition helper | yes       | Build child calendar props and callback bridges | `date-picker`, `date-range-picker`, `date-time-picker` | Avoid drift across pickers.  |

## 23. Framework-Specific Behavior

- Leptos watchers should dispatch machine updates for both controlled value and controlled open state.
- Content refs and focus handoff run only after the overlay mounts.
- Input `on:input`, `on:focus`, and `on:keydown` handlers should normalize onto the picker machine instead of duplicating parse logic locally.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_picker::Machine>(props);

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
fn overlay(machine: UseMachineReturn<date_picker::Machine>) -> impl IntoView {
    let calendar_props = machine.with_api_snapshot(|api| api.calendar_props());
    view! {
        <Positioner>
            <FocusScope>
                <Dismissable on_dismiss=move || machine.dispatch(date_picker::Event::Close)>
                    <Calendar ..calendar_props />
                </Dismissable>
            </FocusScope>
        </Positioner>
    }
}
```

## 26. Adapter Invariants

- Hidden input reflects only committed selected value.
- Overlay dismissal and focus return are instance-scoped and cleaned up on close.
- The embedded calendar remains the single source of grid selection semantics inside content.

## 27. Accessibility and SSR Notes

- Trigger and input must keep popup linkage attrs coherent with open state.
- Content remains the dialog boundary for assistive tech while open.
- SSR may render control structure and hidden input, but not active overlay helpers.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core picker contract and embedded calendar composition.
- Intentional deviations: wrappers may choose text-input or segmented-input presentation, but the picker contract does not change.
- Traceability note: adapter-owned concerns promoted explicitly here include overlay lifecycle, focus return, hidden-input bridging, and calendar composition.

## 29. Test Scenarios

1. Trigger and input open and close the picker according to `open_on_click`.
2. Grid selection updates committed value and closes content when `close_on_select=true`.
3. Controlled open state stays in sync with trigger attrs and mounted content.
4. Hidden input reflects committed ISO date only.
5. Escape and outside dismiss close the picker and return focus appropriately.

## 30. Test Oracle Notes

- `DOM attrs`: assert trigger and input popup linkage, dialog attrs, and hidden-input value.
- `machine state`: verify selected value and open state after trigger, input, and grid interactions.
- `callback order`: verify `on_open_change` and `on_value_change` ordering on selection-close.
- `cleanup side effects`: verify dismissable and focus-scope helpers are torn down on close.
- Cheap recipe: open, select a date, close, and assert one hidden-input update plus no lingering content node.

## 31. Implementation Checklist

- [ ] Controlled value and open sync dispatch machine updates.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Hidden input mirrors only committed ISO value.
- [ ] Embedded calendar props and callback bridges stay machine-derived.
- [ ] Focus return prefers mounted input or trigger refs without document queries.
