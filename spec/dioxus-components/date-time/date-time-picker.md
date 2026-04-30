---
adapter: dioxus
component: date-time-picker
category: date-time
source: components/date-time/date-time-picker.md
source_foundation: foundation/09-adapter-dioxus.md
---

# DateTimePicker — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`DateTimePicker`](../../components/date-time/date-time-picker.md) contract onto a Dioxus 0.7.x component. The adapter adds a unified segmented control contract, calendar-overlay composition, web-only hidden-input policy, cross-boundary traversal, and host fallback rules.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct DateTimePickerProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<DateTime>>>,
    #[props(optional)]
    pub default_value: Option<DateTime>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub format: Option<String>,
    #[props(default = 1)]
    pub visible_months: usize,
    #[props(optional)]
    pub min_value: Option<DateTime>,
    #[props(optional)]
    pub max_value: Option<DateTime>,
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
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<DateTime>>>,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
}

#[component]
pub fn DateTimePicker(props: DateTimePickerProps) -> Element
```

The adapter keeps the unified machine from the agnostic spec and mounts a child `Calendar` overlay for date selection while time segments remain inline.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core date-time-picker contract.
- Part parity: full parity with `Root`, `Label`, `Control`, `DateSegmentGroup`, `TimeSegmentGroup`, repeated `Segment` and `Literal`, `Separator`, `Trigger`, `ClearTrigger`, `Positioner`, `Content`, `Description`, `ErrorMessage`, and `HiddenInput`.
- Known adapter deviations: hidden-input form participation is web-only; popup and DOM semantics may use host fallbacks outside web.

## 4. Part Mapping

| Core part / structure            | Required?              | Adapter rendering target                | Ownership     | Attr source                                | Notes                                     |
| -------------------------------- | ---------------------- | --------------------------------------- | ------------- | ------------------------------------------ | ----------------------------------------- |
| root / label / control           | required               | wrapper, label, unified control group   | adapter-owned | date-time-picker API attrs                 | Control is the shared accessible group.   |
| date and time segment groups     | required               | two grouped bundles inside the control  | adapter-owned | group attrs plus repeated segment attrs    | Traversal crosses the group boundary.     |
| trigger / clear                  | required / conditional | button nodes                            | adapter-owned | picker attrs plus utility button semantics | Trigger opens date overlay only.          |
| overlay shell + child `Calendar` | conditional            | positioner, content, and child calendar | adapter-owned | picker attrs plus overlay helpers          | Only date selection lives in the overlay. |
| `HiddenInput`                    | conditional            | web-only hidden input                   | adapter-owned | API attrs                                  | Omitted on desktop/mobile.                |

## 5. Attr Merge and Ownership Rules

| Target node                | Core attrs                                                | Adapter-owned attrs                         | Consumer attrs      | Merge order                        | Ownership notes                            |
| -------------------------- | --------------------------------------------------------- | ------------------------------------------- | ------------------- | ---------------------------------- | ------------------------------------------ |
| control and segment groups | group roles, labels, invalid, required                    | host classes                                | decoration only     | core semantic attrs win on web     | Non-web hosts may map semantics logically. |
| editable `Segment`         | spinbutton attrs, tabindex, readonly and disabled markers | focus and composition handlers              | decoration only     | core semantic attrs win            | Segments remain adapter-owned.             |
| overlay shell              | dialog attrs and open state                               | dismissable, focus-scope, measurement hooks | visual classes only | adapter preserves dialog semantics | Content exists only while open.            |
| `HiddenInput`              | name and ISO datetime value                               | none                                        | none                | adapter controls fully             | Web only.                                  |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data plus overlay utility helpers.
- Optional consumed contexts: utility `field` / `form`.
- Missing required context behavior: overlay helper absence `fail fast` in debug; `ArsProvider` locale/ICU context follows the foundation policy in `09-adapter-dioxus.md` §16.
- Composition rule: the unified machine owns both date and time segment bundles; the child `Calendar` receives derived date props only and bridges selection back into that machine.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync updates the unified machine after mount.
- Segment input, IME composition, increment/decrement, and focus traversal normalize directly onto unified-machine events.
- Trigger, Escape, outside dismiss, and child-calendar selection normalize onto open-state transitions.
- Child-calendar selection updates only the date portion of the committed datetime value.
- Web hidden-input bridging mirrors only the committed ISO datetime value.

## 8. Registration and Cleanup Contract

| Registered entity       | Registration trigger   | Identity key     | Cleanup trigger             | Cleanup action                | Notes                                |
| ----------------------- | ---------------------- | ---------------- | --------------------------- | ----------------------------- | ------------------------------------ |
| segment node refs       | segment mount          | composite        | segment rerender or cleanup | drop node handle              | Needed for cross-boundary traversal. |
| IME / type-ahead timers | segment entry          | instance-derived | commit, blur, or cleanup    | cancel timer and clear handle | Covers both date and time segments.  |
| overlay helper bundle   | overlay open           | instance-derived | overlay close or cleanup    | dispose helper work           | Instance-scoped only.                |
| hidden-input bridge     | web render with `name` | instance-derived | name removal or cleanup     | remove hidden input node      | Web only.                            |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability    | Composition rule                    | Notes                               |
| ------------------ | ------------- | ------------- | -------------------- | ----------------------------------- | ----------------------------------- |
| `Control`          | yes           | adapter-owned | required after mount | may compose with utility field ref  | Used for group-level focus checks.  |
| editable `Segment` | yes           | adapter-owned | required after mount | keyed by logical segment kind       | Traversal crosses group boundaries. |
| `Trigger`          | yes           | adapter-owned | required after mount | may compose with utility button ref | Used for close-time focus return.   |
| `Content`          | yes           | adapter-owned | client-only          | may compose with overlay helpers    | Exists only while open.             |

## 10. State Machine Boundary Rules

- Machine-owned state: committed datetime value, date and time segment bundles, open state, focused segment, and pending controlled value.
- Adapter-local derived bookkeeping: refs, timers, overlay helper handles.
- Forbidden local mirrors: date-only value, time-only value, open state, or focused segment outside the machine.
- Allowed snapshot-read contexts: render, handlers, overlay lifecycle, and web serialization.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape      | Timing                          | Cancelable? | Notes                                                   |
| ----------------------------------------- | -------------------------- | ------------------ | ------------------------------- | ----------- | ------------------------------------------------------- |
| `on_value_change`                         | machine-derived snapshot   | `Option<DateTime>` | after committed datetime update | no          | Calendar selection combines with current time snapshot. |
| `on_open_change`                          | normalized adapter payload | `bool`             | after open state commit         | no          | Trigger, dismiss, and Escape share this path.           |
| dismissal analytics callback when exposed | raw framework event        | framework event    | before close dispatch           | yes         | Optional wrapper-only surface.                          |

## 12. Failure and Degradation Rules

- Missing overlay helpers: `fail fast` in debug and `degrade gracefully` only when inline fallback is explicitly documented.
- Missing segment ref during traversal: `no-op`.
- Impossible min/max datetime bounds: `warn and ignore`.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Hosts without browser form APIs: `not applicable` for hidden-input behavior.

## 13. Identity and Key Policy

- Segment nodes use `data-derived` identity from segment kind and literal index.
- Overlay shell uses `instance-derived` identity.
- Child calendar bundles use `composite` identity from picker id plus calendar id.
- Hidden input uses `instance-derived` identity on web and `not applicable` elsewhere.

## 14. SSR and Client Boundary Rules

- Web SSR emits stable control, segment groups, buttons, description, error, and hidden-input structure.
- Overlay helpers, child-calendar refs, IME timers, and focus traversal are client-only.
- Desktop/mobile hosts do not participate in SSR.
- Hydration must preserve segment order and the separator between date and time groups on web.

## 15. Performance Constraints

- Keep segment refs stable across ordinary updates.
- Reuse timers and cancel them promptly on blur and cleanup.
- Mount overlay helpers only while open.
- Do not rebuild the whole control when only overlay state changes.

## 16. Implementation Dependencies

| Dependency                                  | Required? | Dependency type      | Why it matters                                                                                                 |
| ------------------------------------------- | --------- | -------------------- | -------------------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context            | required  | context contract     | Segment ordering, localized parsing and formatting, child calendar text, and inherited direction depend on it. |
| shared segment formatting / parsing helpers | required  | shared helper        | Date and time bundles must stay aligned with the unified machine.                                              |
| utility overlay helpers                     | required  | composition contract | Dismissal, focus return, and layer policy are adapter-owned.                                                   |
| hidden-input helper                         | required  | shared helper        | Web datetime ISO serialization should align with other controls.                                               |
| child calendar composition helper           | required  | composition contract | Date overlay selection must bridge into the unified machine.                                                   |

## 17. Recommended Implementation Sequence

1. Render root, label, unified control, segment groups, and buttons.
2. Add controlled value sync plus segment refs and traversal.
3. Add IME and type-ahead cleanup across date and time segments.
4. Mount child `Calendar` overlay and bridge date selection.
5. Add web-only hidden-input bridge and finalize overlay cleanup.

## 18. Anti-Patterns

- Do not split date and time ownership into separate parent machines.
- Do not serialize partially edited datetime text into the hidden input.
- Do not keep overlay helpers mounted while closed.

## 19. Consumer Expectations and Guarantees

- Consumers may assume cross-boundary traversal is continuous across the separator.
- Consumers may assume the web hidden input contains a committed ISO datetime only.
- Consumers must not assume the child calendar owns time selection semantics.

## 20. Platform Support Matrix

| Capability / behavior             | Web          | Desktop        | Mobile         | SSR            | Notes                                     |
| --------------------------------- | ------------ | -------------- | -------------- | -------------- | ----------------------------------------- |
| unified segmented control         | full support | full support   | full support   | full support   | Structure and callbacks are host-neutral. |
| hidden-input form bridge          | full support | not applicable | not applicable | full support   | Browser forms exist only on web.          |
| overlay positioning and dismissal | full support | full support   | fallback path  | SSR-safe empty | Mobile may rely on simpler host overlays. |
| calendar date-selection bridge    | full support | full support   | fallback path  | SSR-safe empty | Overlay content itself is client-mounted. |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| unsupported overlay helper absence       | fail fast            | degrade gracefully  | Only with documented inline fallback.        |
| inconsistent min/max datetime bounds     | debug warning        | warn and ignore     | Keep machine-clamped behavior.               |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept            | Required? | Responsibility                                  | Reused by                  | Notes                |
| ------------------------- | --------- | ----------------------------------------------- | -------------------------- | -------------------- |
| segment formatting helper | yes       | Produce localized date and time segment bundles | segmented adapters         | Shared concept only. |
| overlay helper bundle     | yes       | Dismissal, focus-scope, and layer work          | picker adapters            | Shared concept only. |
| hidden-input helper       | yes       | Serialize committed ISO datetime on web         | form-participating pickers | Avoid drift.         |

## 23. Framework-Specific Behavior

- Dioxus effects should dispatch unified-machine updates for controlled values.
- Stable hook positions matter for repeated derivations; extract child components when needed.
- Desktop/mobile hosts may replace DOM popup and aria surfaces with logical equivalents while preserving behavior.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<date_time_picker::Machine>(props);

rsx! {
    div { ..root_attrs,
        DateTimeControl { machine }
        if is_open { DateTimeOverlay { machine } }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
struct DateTimeOverlayProps {
    pub machine: UseMachineReturn<date_time_picker::Machine>,
}

#[component]
fn DateTimeOverlay(props: DateTimeOverlayProps) -> Element {
    let calendar_props = props.machine.with_api_snapshot(|api| api.calendar_props());
    rsx! { Calendar { ..calendar_props } }
}
```

## 26. Adapter Invariants

- The unified machine remains the only source of truth for committed datetime value and segment bundles.
- Web hidden input reflects only committed ISO datetime state.
- Overlay cleanup always clears stale helper and focus-return work.

## 27. Accessibility and SSR Notes

- Date and time groups retain separate labels inside one shared control.
- The separator between groups stays structural and non-interactive.
- Web SSR may render committed hidden-input values, but not active overlay helpers or focus traversal.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core unified date-time-picker contract on web and logical parity on desktop/mobile.
- Intentional deviations: hidden-input form participation is web-only; popup semantics may use host fallbacks outside web.
- Traceability note: adapter-owned concerns promoted here include unified traversal, overlay lifecycle, child-calendar bridging, and hidden-input policy.

## 29. Test Scenarios

1. Date and time segment groups render in one control with continuous traversal across the separator.
2. Controlled datetime updates keep both groups and the web hidden input in sync.
3. Child-calendar selection updates only the date portion and preserves time segments.
4. Escape and outside dismiss close the overlay and restore focus.
5. Web hidden input reflects committed ISO datetime only.

## 30. Test Oracle Notes

- `DOM attrs`: assert group labels, popup linkage, and hidden-input value on web.
- `machine state`: verify committed datetime after segment edits and child-calendar selection across hosts.
- `callback order`: verify `on_open_change` and `on_value_change` ordering on close paths.
- `cleanup side effects`: verify timers and overlay helpers clear on blur, close, and unmount.
- Cheap recipe: edit a time segment, open the calendar, select a date, close, and assert one ISO datetime hidden-input value on web.

## 31. Implementation Checklist

- [ ] Unified machine remains the only source of truth.
- [ ] Segment refs support cross-boundary traversal.
- [ ] Child-calendar selection bridges only the date portion.
- [ ] Overlay helpers mount only while open and clean up on close.
- [ ] Web hidden input serializes only committed ISO datetime state.
