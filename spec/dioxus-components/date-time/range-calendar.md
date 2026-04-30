---
adapter: dioxus
component: range-calendar
category: date-time
source: components/date-time/range-calendar.md
source_foundation: foundation/09-adapter-dioxus.md
---

# RangeCalendar — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`RangeCalendar`](../../components/date-time/range-calendar.md) contract onto a Dioxus 0.7.x component. The adapter adds explicit hover-preview rendering, announcement cleanup, multi-month structure, and Dioxus host fallback rules.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct RangeCalendarProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<DateRange>>>,
    #[props(optional)]
    pub default_value: Option<DateRange>,
    #[props(optional)]
    pub min: Option<CalendarDate>,
    #[props(optional)]
    pub max: Option<CalendarDate>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = 2)]
    pub visible_months: usize,
    pub page_behavior: calendar::PageBehavior,
    #[props(default = false)]
    pub show_week_numbers: bool,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<DateRange>>>,
}

#[component]
pub fn RangeCalendar(props: RangeCalendarProps) -> Element
```

Plain props remain preferred; the controlled range uses `Signal` only when post-mount sync is required.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range calendar contract.
- Part parity: full parity with the same part set as `Calendar`, plus range-specific data attrs and preview semantics on cell triggers.
- Known adapter deviations: desktop/mobile hosts may expose logical equivalents for DOM-specific attrs and announcements.

## 4. Part Mapping

| Core part / structure                   | Required?   | Adapter rendering target                   | Ownership     | Attr source                 | Notes                                                      |
| --------------------------------------- | ----------- | ------------------------------------------ | ------------- | --------------------------- | ---------------------------------------------------------- |
| root and header bundle                  | required    | same host structure as `Calendar`          | adapter-owned | range-calendar API attrs    | Includes pending-range state attrs.                        |
| `GridGroup` and repeated `Grid` bundles | conditional | multi-month wrapper + repeated grids       | adapter-owned | grid helpers                | Default `visible_months` remains two.                      |
| `Cell` + `CellTrigger`                  | required    | host cell wrapper plus interactive trigger | adapter-owned | range-specific cell helpers | Carries endpoint, in-range, hover-range, and anchor state. |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                                      | Adapter-owned attrs | Consumer attrs  | Merge order                                           | Ownership notes                                              |
| -------------- | ----------------------------------------------- | ------------------- | --------------- | ----------------------------------------------------- | ------------------------------------------------------------ |
| `Root`         | ids, readonly, disabled, pending-range markers  | host classes        | decoration only | core semantic attrs win                               | Root stays adapter-owned.                                    |
| `CellTrigger`  | endpoint, preview, unavailable, focused markers | host event handlers | decoration only | core semantic state wins                              | Non-web hosts may expose logical state instead of DOM attrs. |
| shared heading | heading attrs and labels                        | announcement wiring | none            | adapter keeps one shared visible announcement surface | Prevent duplicate announcements.                             |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: overlay helpers when embedded in pickers.
- Missing required context behavior: fail fast in debug; degrade gracefully only through the documented `ArsProvider` fallback policy in `09-adapter-dioxus.md` §16.
- Composition rule: picker adapters may wrap `RangeCalendar`, but hover preview, anchor ownership, and range normalization remain inside this adapter and its machine.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync updates the machine range after mount.
- Pointer enter and leave normalize onto hover preview events; click and keyboard activation normalize onto `SelectDate`.
- `disabled`, `readonly`, and bounds changes update preview and selection gating immediately.
- No form bridge exists for standalone range calendar.

## 8. Registration and Cleanup Contract

| Registered entity         | Registration trigger             | Identity key     | Cleanup trigger                                 | Cleanup action      | Notes                                               |
| ------------------------- | -------------------------------- | ---------------- | ----------------------------------------------- | ------------------- | --------------------------------------------------- |
| hover-preview bookkeeping | pointer enter                    | data-derived     | pointer leave, selection completion, or cleanup | clear preview state | Prevent stale preview after unmount.                |
| focused cell node ref     | cell trigger mount               | composite        | cell rerender or unmount                        | drop node handle    | Used for focus restoration.                         |
| announcement effect       | anchor or completed range change | instance-derived | effect rerun or cleanup                         | cancel stale work   | Needed for range-start and range-complete messages. |

## 9. Ref and Node Contract

| Target part / node   | Ref required? | Ref owner     | Node availability                  | Composition rule | Notes                            |
| -------------------- | ------------- | ------------- | ---------------------------------- | ---------------- | -------------------------------- |
| root                 | no            | adapter-owned | always structural, handle optional | no composition   | Structural only.                 |
| navigation triggers  | yes           | adapter-owned | required after mount               | no composition   | Needed in embedded picker flows. |
| focused cell trigger | yes           | adapter-owned | required after mount               | keyed by date    | Use mounted refs instead of ids. |
| hover preview nodes  | no            | adapter-owned | always structural, handle optional | no composition   | Derived from cell attrs only.    |

## 10. State Machine Boundary Rules

- Machine-owned state: selected range, anchor date, hover date, focused date, visible month, and range membership.
- Adapter-local derived bookkeeping: mounted refs and deduped announcement effects.
- Forbidden local mirrors: range endpoints, hover span, or focus state.
- Allowed snapshot-read contexts: render derivation, handlers, and focus restoration.

## 11. Callback Payload Contract

| Callback                                           | Payload source             | Payload shape                                                         | Timing                      | Cancelable? | Notes                                                              |
| -------------------------------------------------- | -------------------------- | --------------------------------------------------------------------- | --------------------------- | ----------- | ------------------------------------------------------------------ |
| `on_value_change`                                  | machine-derived snapshot   | `Option<DateRange>`                                                   | after completed selection   | no          | First click that only sets anchor does not emit a completed range. |
| `on_range_preview_change` when exposed by wrappers | normalized adapter payload | `{ anchor: Option<CalendarDate>, preview_end: Option<CalendarDate> }` | after hover updates         | no          | Optional wrapper-only surface.                                     |
| announcement hook when exposed                     | none                       | `()`                                                                  | after announcement dispatch | no          | Diagnostics only.                                                  |

## 12. Failure and Degradation Rules

- `visible_months == 0`: `fail fast` in debug and `degrade gracefully` to `2` in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Hover preview on hosts without pointer hover support: `degrade gracefully`.
- Impossible bounds or out-of-range controlled values: `warn and ignore`.
- Hosts without browser live-region APIs: `degrade gracefully`.

## 13. Identity and Key Policy

- Grid bundles use `composite` identity from base id plus offset.
- Cells and preview membership use `data-derived` identity from `CalendarDate`.
- Announcement effects use `instance-derived` identity.
- Hidden inputs are `not applicable`.

## 14. SSR and Client Boundary Rules

- Web SSR renders stable multi-month structure and range attrs.
- Hover preview, focus movement, and announcements are client-only.
- Desktop and mobile hosts do not participate in SSR.
- Hydration must preserve heading ids and bundle order.

## 15. Performance Constraints

- Keep hover preview updates limited to visible cells.
- Avoid rebuilding all month bundles on pointer movement.
- Deduplicate announcement work for repeated hover over the same date.
- Keep node-ref maps instance-scoped.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type         | Why it matters                                                                                     |
| -------------------------------- | ----------- | ----------------------- | -------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required    | context contract        | Month headings, weekday labels, announcements, preview text, and inherited direction depend on it. |
| shared calendar grid helpers     | required    | shared helper           | Range calendar shares month and row computation with `Calendar`.                                   |
| live-region helper               | required    | shared helper           | Range-start and range-complete announcements need dedupe and cleanup.                              |
| pointer normalization            | recommended | behavioral prerequisite | Hover preview should not depend on raw host quirks.                                                |
| utility button semantics         | recommended | composition contract    | Navigation triggers should stay aligned with utility adapters.                                     |

## 17. Recommended Implementation Sequence

1. Reuse the base calendar grid and navigation structure.
2. Add range endpoint and preview attrs.
3. Add hover preview and cleanup.
4. Add start and completion announcements.
5. Finalize host fallback behavior and picker embedding notes.

## 18. Anti-Patterns

- Do not compute in-range and hover-range styling outside the machine snapshot.
- Do not keep stale hover preview after selection completion.
- Do not treat every in-range cell as an endpoint for accessibility.

## 19. Consumer Expectations and Guarantees

- Consumers may assume endpoint, in-range, and preview state stay coherent.
- Consumers may assume first click sets an anchor without completing the range.
- Consumers must not assume every host exposes browser-native hover semantics.

## 20. Platform Support Matrix

| Capability / behavior          | Web          | Desktop       | Mobile        | SSR            | Notes                                                             |
| ------------------------------ | ------------ | ------------- | ------------- | -------------- | ----------------------------------------------------------------- |
| range selection and navigation | full support | full support  | fallback path | SSR-safe empty | Mobile may rely on host focus and tap affordances.                |
| hover preview                  | full support | fallback path | fallback path | SSR-safe empty | Host without hover may show preview only during drag/focus flows. |
| live range announcements       | full support | fallback path | fallback path | SSR-safe empty | Use host announcement APIs where available.                       |
| multi-month rendering          | full support | full support  | fallback path | full support   | Mobile may stack month bundles vertically.                        |

## 21. Debug Diagnostics and Production Policy

| Condition                                  | Debug build behavior | Production behavior | Notes                                        |
| ------------------------------------------ | -------------------- | ------------------- | -------------------------------------------- |
| invalid `visible_months`                   | fail fast            | degrade gracefully  | Clamp to two months.                         |
| reversed or out-of-bounds controlled range | debug warning        | warn and ignore     | Machine keeps normalized/clamped range.      |
| missing `ArsProvider` locale/ICU context   | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required? | Responsibility                         | Reused by                             | Notes                        |
| -------------------- | --------- | -------------------------------------- | ------------------------------------- | ---------------------------- |
| calendar grid helper | yes       | Shared weeks, headings, and pagination | `calendar`, `range-calendar`          | Avoid drift across adapters. |
| range-preview helper | yes       | Compute endpoint and preview state     | `range-calendar`, `date-range-picker` | Shared concept only.         |
| live-region helper   | yes       | Announce anchor and completed range    | `range-calendar`, `date-range-picker` | Host-specific backend.       |

## 23. Framework-Specific Behavior

- Stable hook positions matter for repeated derivations; extract child components instead of deriving inside loops.
- Dioxus effects should dispatch machine updates for controlled ranges rather than mutate local state.
- Desktop/mobile hosts may map hover preview to focus or press-preview where pointer hover does not exist.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<range_calendar::Machine>(props);

rsx! {
    div { ..root_attrs,
        RangeCalendarHeader { machine }
        RangeCalendarGrids { machine }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
struct CalendarCellProps {
    pub machine: UseMachineReturn<range_calendar::Machine>,
    pub date: CalendarDate,
}

#[component]
fn CalendarCell(props: CalendarCellProps) -> Element {
    rsx! {
        button { ..props.machine.derive(move |api| api.cell_trigger_attrs(&props.date)).read().clone() }
    }
}
```

## 26. Adapter Invariants

- Completed range selection clears hover preview immediately.
- Only one shared visible heading remains the announcement surface.
- Range endpoint and in-range state always derive from normalized order.

## 27. Accessibility and SSR Notes

- Range start and completion announcements remain deduped and polite on supported hosts.
- Endpoint cells expose clear labels; preview-only cells do not impersonate selected endpoints.
- Web SSR keeps bundle structure stable so hydration can attach hover and keyboard logic cleanly.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range calendar on web and logical parity on desktop/mobile.
- Intentional deviations: host fallback may replace DOM-native hover and aria surfaces with logical equivalents.
- Traceability note: adapter-owned concerns promoted explicitly here include preview cleanup, announcement policy, node refs, and host fallback behavior.

## 29. Test Scenarios

1. First click sets anchor and pending-range state without firing completion callback.
2. Hover preview marks preview cells and clears on leave or completion.
3. Second click completes the normalized range.
4. Multi-month rendering keeps headings and range markers coherent across month boundaries.
5. Controlled updates clear stale preview state.

## 30. Test Oracle Notes

- `DOM attrs`: assert endpoint and preview attrs on web.
- `machine state`: verify anchor, hover, and normalized range endpoints across hosts.
- `callback order`: verify completion callback fires only after second selection.
- `cleanup side effects`: verify preview and announcement work clear on unmount.
- Cheap recipe: click one date, preview a second date, then unmount and confirm no stale announcement fires.

## 31. Implementation Checklist

- [ ] Hover preview dispatches machine preview events without local mirror state.
- [ ] Completed range selection clears preview and emits normalized range.
- [ ] Multi-month rendering stays keyed by date-derived identities.
- [ ] Announcement behavior is deduped and host-gated.
- [ ] Range endpoint state remains on cell triggers, not just wrappers.
