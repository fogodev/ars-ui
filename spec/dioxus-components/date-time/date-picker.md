---
adapter: dioxus
component: date-picker
category: date-time
source: components/date-time/date-picker.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DatePicker — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DatePicker`](../../components/date-time/date-picker.md) contract onto a Dioxus 0.7.x component. The adapter adds explicit input and overlay ownership, web-only hidden-input policy, focus return behavior, and desktop/mobile host fallbacks.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<CalendarDate>>>,
    #[props(optional)]
    pub default_value: Option<CalendarDate>,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(default = false)]
    pub default_open: bool,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub format: Option<String>,
    #[props(optional)]
    pub min: Option<CalendarDate>,
    #[props(optional)]
    pub max: Option<CalendarDate>,
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
    pub open_on_click: bool,
    #[props(default = false)]
    pub close_on_select: bool,
    #[props(default = 1)]
    pub visible_months: usize,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<CalendarDate>>>,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
}

#[component]
pub fn DatePicker(props: DatePickerProps) -> Element
```

Plain props are preferred; `Signal` is reserved for controlled value and controlled open sync.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core picker contract.
- Part parity: full parity with `Root`, `Label`, `Control`, `Input`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input form participation is web-only; desktop/mobile hosts use logical equivalents for popup semantics when needed.

## 4. Part Mapping

| Core part / structure              | Required?              | Adapter rendering target                | Ownership     | Attr source                                | Notes                                    |
| ---------------------------------- | ---------------------- | --------------------------------------- | ------------- | ------------------------------------------ | ---------------------------------------- |
| root / label / control             | required               | wrapper, label, and control row         | adapter-owned | picker API attrs                           | Input and buttons remain separate nodes. |
| `Input`, `Trigger`, `ClearTrigger` | required / conditional | host input and button nodes             | adapter-owned | picker attrs plus utility button semantics | Trigger and clear remain independent.    |
| overlay shell                      | conditional            | positioner, content, and child calendar | adapter-owned | picker attrs plus overlay helpers          | Content hosts `Calendar`.                |
| `HiddenInput`                      | conditional            | web-only hidden input                   | adapter-owned | API attrs                                  | Omitted on desktop/mobile.               |

## 5. Attr Merge and Ownership Rules

| Target node                | Core attrs                                  | Adapter-owned attrs                         | Consumer attrs          | Merge order                        | Ownership notes                                      |
| -------------------------- | ------------------------------------------- | ------------------------------------------- | ----------------------- | ---------------------------------- | ---------------------------------------------------- |
| `Input`                    | value, popup linkage, invalid, described-by | host handlers and placeholder               | wrapper decoration only | core semantic attrs win on web     | Non-web hosts may expose logical popup linkage only. |
| `Trigger` / `ClearTrigger` | labels, disabled, expanded state            | utility button attrs, dismissal hooks       | decoration only         | core semantic attrs win            | Buttons remain adapter-owned.                        |
| overlay shell              | ids, state, dialog attrs                    | positioning, focus-scope, dismissable hooks | visual classes only     | adapter preserves dialog semantics | Content exists only while open.                      |
| `HiddenInput`              | name and ISO value                          | none                                        | none                    | adapter controls fully             | Web only.                                            |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus overlay utility helpers.
- Optional consumed contexts: utility `field` / `form`.
- Missing required context behavior: overlay helper absence `fail fast` in debug; `ArsProvider` locale/ICU context follows its policy in `09-adapter-dioxus.md` §16.
- Composition rule: `DatePicker` composes one child `Calendar`; selected value and open state remain parent-owned.

## 7. Prop Sync and Event Mapping

- Controlled `value` and `open` sync after mount via Dioxus effects.
- Text input changes normalize onto parse-and-select machine transitions.
- Trigger click, trigger keydown, outside dismiss, Escape, and selection completion normalize onto open-state transitions.
- Hidden-input bridging mirrors committed selected value only on web.
- Focus return after close prefers the input and may fall back to the trigger on hosts that require it.

## 8. Registration and Cleanup Contract

| Registered entity           | Registration trigger   | Identity key     | Cleanup trigger          | Cleanup action                       | Notes                                        |
| --------------------------- | ---------------------- | ---------------- | ------------------------ | ------------------------------------ | -------------------------------------------- |
| dismissable boundary        | overlay open           | instance-derived | overlay close or cleanup | unregister outside-interaction hooks | Prevent stale close callbacks.               |
| focus scope                 | overlay open           | instance-derived | overlay close or cleanup | unregister trapped-focus bookkeeping | Instance-scoped only.                        |
| positioner measurement work | overlay open           | instance-derived | overlay close or cleanup | dispose measurement work             | Web and desktop hosts may differ internally. |
| hidden-input bridge         | web render with `name` | instance-derived | name removal or cleanup  | remove hidden input node             | Not applicable elsewhere.                    |

## 9. Ref and Node Contract

| Target part / node  | Ref required? | Ref owner           | Node availability    | Composition rule                    | Notes                                    |
| ------------------- | ------------- | ------------------- | -------------------- | ----------------------------------- | ---------------------------------------- |
| `Input`             | yes           | adapter-owned       | required after mount | may compose with utility field ref  | Used for focus return and open-on-click. |
| `Trigger`           | yes           | adapter-owned       | required after mount | may compose with utility button ref | Needed for Escape-close fallback.        |
| `Content`           | yes           | adapter-owned       | client-only          | may compose with overlay helpers    | Content exists only while open.          |
| child calendar root | yes           | child adapter-owned | required after mount | child adapter owns grid refs        | Parent should not steal child refs.      |

## 10. State Machine Boundary Rules

- Machine-owned state: selected value, input text, parsed date, and open state.
- Adapter-local derived bookkeeping: overlay helper handles, mounted refs, and host measurement work.
- Forbidden local mirrors: selected date, parsed date, or open state outside the machine / controlled signal contract.
- Allowed snapshot-read contexts: render, overlay lifecycle, and web form-bridge serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape          | Timing                  | Cancelable? | Notes                                                  |
| ----------------------------------------- | -------------------------- | ---------------------- | ----------------------- | ----------- | ------------------------------------------------------ |
| `on_value_change`                         | machine-derived snapshot   | `Option<CalendarDate>` | after value commit      | no          | Fires for typed or grid-selected values.               |
| `on_open_change`                          | normalized adapter payload | `bool`                 | after open state commit | no          | Fires for trigger, dismiss, and selection-close paths. |
| dismissal analytics callback when exposed | raw framework event        | framework event        | before close dispatch   | yes         | Optional wrapper-only surface.                         |

## 12. Failure and Degradation Rules

- Invalid `visible_months`: `fail fast` in debug and `degrade gracefully` to `1` in production.
- Missing overlay helpers: `fail fast` in debug and `degrade gracefully` only when an inline fallback is explicitly supported.
- Missing input or trigger ref during close-time focus return: `no-op`.
- Impossible controlled prop combinations: `warn and ignore`.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Overlay shell uses `instance-derived` identity.
- Child calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden input uses `instance-derived` identity on web and `not applicable` elsewhere.
- Server error keys are `not applicable`.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable root, control, input, buttons, description, error, and hidden-input structure.
- Overlay helpers, content refs, and focus return are client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve control structure whether the picker is initially open or closed.

## 15. Performance Constraints

- Avoid mounting overlay helpers while closed.
- Reuse the child calendar structure across open cycles when possible.
- Keep measurement and dismissable work instance-scoped.
- Do not rebuild the whole control row when only open state changes.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type      | Why it matters                                                                                   |
| -------------------------------- | ----------- | -------------------- | ------------------------------------------------------------------------------------------------ |
| `ArsProvider` locale/ICU context | required    | context contract     | Input parsing, display formatting, embedded calendar text, and inherited direction depend on it. |
| utility `button`                 | required    | composition contract | Trigger and clear button semantics should stay aligned.                                          |
| utility overlay helpers          | required    | composition contract | Dismissal, focus return, and layer policy are adapter-owned concerns.                            |
| utility `field` / `form`         | required    | composition contract | Label, invalid, description, and reset semantics stay consistent.                                |
| positioner helper                | recommended | shared helper        | Keeps overlay placement incremental and disposable.                                              |

## 17. Recommended Implementation Sequence

1. Wire the picker machine and render root, control, input, and buttons.
2. Add controlled value and open effects.
3. Mount child `Calendar` in overlay content.
4. Add overlay helpers and focus return.
5. Add web-only hidden-input bridging and host fallback notes.

## 18. Anti-Patterns

- Do not fork selected date ownership between the picker and the child calendar.
- Do not keep overlay helpers mounted while closed.
- Do not serialize typed but unparsed text into the hidden input.

## 19. Consumer Expectations and Guarantees

- Consumers may assume closing the picker leaves committed value and hidden-input state in sync on web.
- Consumers may assume trigger and input remain separate nodes with independent semantics.
- Consumers must not assume overlay content exists while the picker is closed.

## 20. Platform Support Matrix

| Capability / behavior                            | Web          | Desktop        | Mobile         | SSR            | Notes                                        |
| ------------------------------------------------ | ------------ | -------------- | -------------- | -------------- | -------------------------------------------- |
| control row rendering                            | full support | full support   | full support   | full support   | Structure and callbacks remain host-neutral. |
| hidden-input form bridge                         | full support | not applicable | not applicable | full support   | Browser forms exist only on web.             |
| overlay positioning and dismissal                | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on simpler host overlays.    |
| focus return between input, trigger, and content | full support | full support   | fallback path  | SSR-safe empty | Requires mounted refs.                       |

## 21. Debug Diagnostics and Production Policy

| Condition                          | Debug build behavior | Production behavior | Notes                                             |
| ---------------------------------- | -------------------- | ------------------- | ------------------------------------------------- |
| invalid `visible_months`           | fail fast            | degrade gracefully  | Clamp to one month.                               |
| unsupported overlay helper absence | fail fast            | degrade gracefully  | Only when inline fallback is documented.          |
| conflicting controlled-open props  | debug warning        | warn and ignore     | Explicit controlled signal remains authoritative. |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required? | Responsibility                                  | Reused by                  | Notes                        |
| --------------------------- | --------- | ----------------------------------------------- | -------------------------- | ---------------------------- |
| overlay helper bundle       | yes       | Dismissal, focus scope, and layer work          | picker adapters            | Shared concept only.         |
| hidden-input helper         | yes       | ISO serialization on web                        | form-participating pickers | Keep reset behavior aligned. |
| calendar composition helper | yes       | Build child calendar props and callback bridges | picker adapters            | Avoid drift across pickers.  |

## 23. Framework-Specific Behavior

- Dioxus effects should dispatch machine updates for controlled value and controlled open state.
- Content refs and focus handoff run only after overlay mount.
- Desktop/mobile hosts may replace DOM popup attrs with logical equivalents while keeping callback ordering intact.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_picker::Machine>(props);

rsx! {
    div { ..root_attrs,
        Control { machine }
        if is_open { Overlay { machine } }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
struct OverlayProps {
    pub machine: UseMachineReturn<date_picker::Machine>,
}

#[component]
fn Overlay(props: OverlayProps) -> Element {
    let calendar_props = props.machine.with_api_snapshot(|api| api.calendar_props());
    rsx! { Calendar { ..calendar_props } }
}
```

## 26. Adapter Invariants

- Web hidden input reflects only committed selected value.
- Overlay dismissal and focus return are instance-scoped and cleaned up on close.
- The child calendar remains the single source of grid selection semantics inside content.

## 27. Accessibility and SSR Notes

- Trigger and input keep popup linkage state coherent on web.
- Content remains the dialog boundary while open on hosts that support dialog semantics.
- Web SSR may render control structure and hidden input, but not active overlay helpers.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core picker contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only; popup semantics may use host fallbacks outside web.
- Traceability note: adapter-owned concerns promoted here include overlay lifecycle, focus return, hidden-input policy, and calendar composition.

## 29. Test Scenarios

1. Trigger and input open and close the picker according to `open_on_click`.
2. Grid selection updates committed value and closes content when configured.
3. Controlled open state stays in sync with popup linkage attrs and mounted content.
4. Web hidden input reflects committed ISO date only.
5. Escape and outside dismiss close the picker and restore focus appropriately.

## 30. Test Oracle Notes

- `DOM attrs`: assert popup linkage, dialog attrs, and hidden-input value on web.
- `machine state`: verify selected value and open state across hosts.
- `callback order`: verify `on_open_change` and `on_value_change` ordering on selection-close.
- `cleanup side effects`: verify overlay helpers tear down on close.
- Cheap recipe: open, select a date, close, and assert one hidden-input update plus no lingering content node on web.

## 31. Implementation Checklist

- [ ] Controlled value and open sync dispatch machine updates.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Hidden input is web-only and mirrors only committed ISO value.
- [ ] Child calendar props and callback bridges stay machine-derived.
- [ ] Focus return uses mounted refs without id lookup.
