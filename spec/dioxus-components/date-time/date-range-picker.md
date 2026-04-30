---
adapter: dioxus
component: date-range-picker
category: date-time
source: components/date-time/date-range-picker.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DateRangePicker — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateRangePicker`](../../components/date-time/date-range-picker.md) contract onto a Dioxus 0.7.x component. The adapter adds explicit twin-field composition, overlay ownership, range-calendar bridging, web-only hidden-input strategy, and host-specific focus-return rules.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct DateRangePickerProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<DateRange>>>,
    #[props(optional)]
    pub default_value: Option<DateRange>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub min: Option<CalendarDate>,
    #[props(optional)]
    pub max: Option<CalendarDate>,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(default = false)]
    pub close_on_select: bool,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub start_name: Option<String>,
    #[props(optional)]
    pub end_name: Option<String>,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<DateRange>>>,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
}

#[component]
pub fn DateRangePicker(props: DateRangePickerProps) -> Element
```

The adapter owns the shared control row, overlay shell, and web-only hidden-input strategy, and composes two child `DateField` adapters plus one child `RangeCalendar`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range-picker contract.
- Part parity: full parity with `Root`, `Label`, `Control`, `StartInput`, `Separator`, `EndInput`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input submission is web-only; popup and dialog semantics may use host fallbacks outside web.

## 4. Part Mapping

| Core part / structure                 | Required?              | Adapter rendering target                | Ownership                 | Attr source                                | Notes                                   |
| ------------------------------------- | ---------------------- | --------------------------------------- | ------------------------- | ------------------------------------------ | --------------------------------------- |
| root / label / control                | required               | wrapper + control group                 | adapter-owned             | range-picker API attrs                     | Control is the shared accessible group. |
| child `DateField` inputs              | required               | two internal child adapters             | adapter-owned composition | derived child props                        | Child fields stay machine-owned.        |
| trigger / clear                       | required / conditional | button nodes                            | adapter-owned             | picker attrs plus utility button semantics | Separate from the child fields.         |
| overlay shell + child `RangeCalendar` | conditional            | positioner, content, and child calendar | adapter-owned             | picker attrs plus overlay helpers          | Content exists only while open.         |
| hidden input(s)                       | conditional            | web-only one or three hidden inputs     | adapter-owned             | API attrs                                  | Omitted on desktop/mobile.              |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                             | Adapter-owned attrs                    | Consumer attrs        | Merge order                    | Ownership notes                            |
| ----------------- | -------------------------------------- | -------------------------------------- | --------------------- | ------------------------------ | ------------------------------------------ |
| control group     | group role, labelled-by, state markers | wrapper classes                        | decoration only       | core semantic attrs win on web | Non-web hosts may map semantics logically. |
| child inputs      | child field attrs                      | derived min/max and aria labels        | child decoration only | child semantics win            | Parent does not replace child groups.      |
| trigger / content | popup linkage, disabled, dialog attrs  | overlay hooks and utility button attrs | decoration only       | core semantics win             | Content exists only while open.            |
| hidden input(s)   | single or split ISO range value(s)     | none                                   | none                  | adapter controls fully         | Web only.                                  |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus overlay utility helpers.
- Optional consumed contexts: utility `field` / `form`.
- Missing required context behavior: overlay helper absence `fail fast` in debug; `ArsProvider` locale/ICU context follows policy in `09-adapter-dioxus.md` §16.
- Composition rule: the parent machine owns range and open state; child `DateField` and `RangeCalendar` adapters only receive derived props and emit bridge callbacks.

## 7. Prop Sync and Event Mapping

- Controlled `value` and `open` sync after mount via Dioxus effects.
- Child field commits normalize onto the parent range machine.
- Child `RangeCalendar` completion normalizes onto the same parent machine and closes content when `close_on_select=true`.
- Trigger, clear, outside dismiss, and Escape normalize onto open-state transitions.
- Web hidden-input bridging mirrors only committed normalized range state.

## 8. Registration and Cleanup Contract

| Registered entity            | Registration trigger  | Identity key     | Cleanup trigger           | Cleanup action                | Notes                             |
| ---------------------------- | --------------------- | ---------------- | ------------------------- | ----------------------------- | --------------------------------- |
| child field bridge callbacks | child mount           | composite        | child rerender or cleanup | drop stale bridge             | Avoid stale start/end routing.    |
| child range-calendar bridge  | overlay open          | instance-derived | overlay close or cleanup  | unregister selection callback | Instance-scoped only.             |
| overlay helper bundle        | overlay open          | instance-derived | overlay close or cleanup  | dispose helper work           | Prevent lingering listeners.      |
| hidden-input bridge(s)       | web render with names | composite        | name change or cleanup    | remove stale hidden inputs    | Respect single vs split strategy. |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner           | Node availability    | Composition rule                    | Notes                            |
| ------------------ | ------------- | ------------------- | -------------------- | ----------------------------------- | -------------------------------- |
| control group      | yes           | adapter-owned       | required after mount | may compose with utility field ref  | Needed for outside-focus checks. |
| trigger            | yes           | adapter-owned       | required after mount | may compose with utility button ref | Used for focus return.           |
| overlay content    | yes           | adapter-owned       | client-only          | may compose with overlay helpers    | Exists only while open.          |
| child field groups | yes           | child adapter-owned | required after mount | child adapters own their refs       | Parent must not steal them.      |

## 10. State Machine Boundary Rules

- Machine-owned state: normalized range, active field, open state, and close-on-select behavior.
- Adapter-local derived bookkeeping: overlay helper handles, mounted refs, and child bridge handles.
- Forbidden local mirrors: range endpoints, active field, or open state outside the machine / controlled signal contract.
- Allowed snapshot-read contexts: render, child-prop derivation, overlay lifecycle, and web serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape       | Timing                       | Cancelable? | Notes                                                  |
| ----------------------------------------- | -------------------------- | ------------------- | ---------------------------- | ----------- | ------------------------------------------------------ |
| `on_value_change`                         | machine-derived snapshot   | `Option<DateRange>` | after committed range update | no          | Fires for child-field or calendar completion.          |
| `on_open_change`                          | normalized adapter payload | `bool`              | after open state commit      | no          | Fires for trigger, dismiss, and selection-close paths. |
| dismissal analytics callback when exposed | raw framework event        | framework event     | before close dispatch        | yes         | Optional wrapper-only surface.                         |

## 12. Failure and Degradation Rules

- Conflicting hidden-input naming strategy: `warn and ignore` split names when single `name` is present.
- Missing overlay helpers: `fail fast` in debug and `degrade gracefully` only when inline fallback is explicitly supported.
- Missing child bridge ref during close-time focus return: `no-op`.
- Impossible range bounds: `warn and ignore`.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Child field instances use `composite` identity from base id plus `start` / `end`.
- Overlay shell uses `instance-derived` identity.
- Child range-calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden-input bridge uses `composite` identity from submission strategy on web and `not applicable` elsewhere.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable control, child field, separator, description, error, and hidden-input structure.
- Overlay helpers, content refs, and focus return are client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve start-before-end ordering and trigger placement on web.

## 15. Performance Constraints

- Keep overlay helpers mounted only while open.
- Reuse child field and child range-calendar structure across open cycles when possible.
- Avoid rebuilding hidden-input strategy when names are unchanged.
- Keep bridge callbacks instance-scoped.

## 16. Implementation Dependencies

| Dependency                       | Required? | Dependency type      | Why it matters                                                                                     |
| -------------------------------- | --------- | -------------------- | -------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required  | context contract     | Child field formatting, range labels, overlay calendar text, and inherited direction depend on it. |
| child `DateField` adapter        | required  | composition contract | Start and end fields should reuse the segmented contract.                                          |
| child `RangeCalendar` adapter    | required  | composition contract | Overlay range selection should reuse preview and grid contracts.                                   |
| utility overlay helpers          | required  | composition contract | Dismissal, focus return, and layer policy are adapter-owned concerns.                              |
| hidden-input helper              | required  | shared helper        | Web submission strategy must stay aligned with `DateRangeField`.                                   |

## 17. Recommended Implementation Sequence

1. Render root, label, control, child fields, separator, and buttons.
2. Add controlled value and open effects.
3. Mount child `RangeCalendar` in overlay content and bridge its events.
4. Add overlay helpers and focus return.
5. Add web-only single and split hidden-input strategies.

## 18. Anti-Patterns

- Do not maintain separate range state inside child fields or the child calendar.
- Do not render both single and split hidden-input strategies simultaneously.
- Do not keep overlay helpers mounted after close.

## 19. Consumer Expectations and Guarantees

- Consumers may assume child fields and the child calendar stay synchronized through one parent source of truth.
- Consumers may assume web hidden-input submission uses normalized committed range values only.
- Consumers must not assume overlay content remains mounted while closed.

## 20. Platform Support Matrix

| Capability / behavior                       | Web          | Desktop        | Mobile         | SSR            | Notes                                     |
| ------------------------------------------- | ------------ | -------------- | -------------- | -------------- | ----------------------------------------- |
| control row and child synchronization       | full support | full support   | full support   | full support   | Structure and callbacks are host-neutral. |
| hidden-input form bridge                    | full support | not applicable | not applicable | full support   | Browser forms exist only on web.          |
| overlay positioning and dismissal           | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on simpler host overlays. |
| child range-calendar preview and completion | full support | full support   | fallback path  | SSR-safe empty | Overlay content itself is client-mounted. |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                 |
| ---------------------------------------- | -------------------- | ------------------- | ------------------------------------- |
| conflicting hidden-input naming strategy | debug warning        | warn and ignore     | Prefer single `name`.                 |
| unsupported overlay helper absence       | fail fast            | degrade gracefully  | Only with documented inline fallback. |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep machine clamping.                |

## 22. Shared Adapter Helper Notes

| Helper concept        | Required? | Responsibility                                        | Reused by                             | Notes                    |
| --------------------- | --------- | ----------------------------------------------------- | ------------------------------------- | ------------------------ |
| child field bridge    | yes       | Route start/end child commits into the parent machine | range field and range picker adapters | Internal only.           |
| overlay helper bundle | yes       | Dismissal, focus scope, and layer work                | picker adapters                       | Shared concept only.     |
| hidden-input helper   | yes       | Serialize one interval or two endpoint inputs on web  | range field and range picker adapters | Keep strategies aligned. |

## 23. Framework-Specific Behavior

- Dioxus child bridge callbacks should dispatch parent machine events rather than patch parent state directly.
- Overlay content refs and focus return run only after content mounts.
- Desktop/mobile hosts may replace DOM popup attrs with logical equivalents while keeping callback ordering intact.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_range_picker::Machine>(props);

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
    pub machine: UseMachineReturn<date_range_picker::Machine>,
}

#[component]
fn Overlay(props: OverlayProps) -> Element {
    let calendar_props = props.machine.with_api_snapshot(|api| api.calendar_props());
    rsx! { RangeCalendar { ..calendar_props } }
}
```

## 26. Adapter Invariants

- Parent machine state remains the only source of truth for range and open state.
- Web single and split hidden-input strategies are mutually exclusive.
- Overlay cleanup always clears stale child bridges and helper work.

## 27. Accessibility and SSR Notes

- Control remains the shared accessible group for both child fields.
- Separator stays structural and non-interactive.
- Web SSR may render committed hidden-input values, but not active overlay helpers or mounted content.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range-picker contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only; popup semantics may use host fallbacks outside web.
- Traceability note: adapter-owned concerns promoted here include overlay lifecycle, child-bridge ownership, hidden-input policy, and focus return.

## 29. Test Scenarios

1. Start and end child fields receive correct labels, bounds, and callbacks.
2. Child range-calendar completion updates committed range and closes content when configured.
3. Controlled open state stays in sync with popup linkage semantics and mounted content.
4. Web single and split hidden-input strategies serialize normalized range values correctly.
5. Escape and outside dismiss close content and restore focus appropriately.

## 30. Test Oracle Notes

- `DOM attrs`: assert control-group, popup linkage, dialog, and hidden-input attrs on web.
- `machine state`: verify committed range and open state across hosts.
- `callback order`: verify `on_value_change` and `on_open_change` ordering on completion-close.
- `cleanup side effects`: verify overlay helper and child bridge teardown on close.
- Cheap recipe: open, complete a range, close, and assert normalized hidden-input values plus no lingering content node on web.

## 31. Implementation Checklist

- [ ] Parent machine remains the only source of truth for range and open state.
- [ ] Child `DateField` and `RangeCalendar` props derive from the parent snapshot.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Web hidden-input strategies remain mutually exclusive.
- [ ] Focus return uses mounted refs without id lookup.
