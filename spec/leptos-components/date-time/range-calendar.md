---
adapter: leptos
component: range-calendar
category: date-time
source: components/date-time/range-calendar.md
source_foundation: foundation/08-adapter-leptos.md
---

# RangeCalendar — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`RangeCalendar`](../../components/date-time/range-calendar.md) contract onto a Leptos 0.8.x component. The adapter adds explicit range-preview rendering, hover cleanup, multi-month group structure, live announcements, and Leptos-specific ref ownership.

## 2. Public Adapter API

```rust
#[component]
pub fn RangeCalendar(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<DateRange>>,
    #[prop(optional)] default_value: Option<DateRange>,
    #[prop(optional)] min: Option<CalendarDate>,
    #[prop(optional)] max: Option<CalendarDate>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] visible_months: usize,
    #[prop(optional)] page_behavior: calendar::PageBehavior,
    #[prop(optional)] show_week_numbers: bool,
    #[prop(optional)] on_value_change: Option<Callback<Option<DateRange>>>,
) -> impl IntoView
```

The adapter keeps the full range-selection machine and surfaces only machine-compatible callbacks; hover preview remains adapter-owned behavior.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core range calendar contract, including anchor, preview, and normalized range semantics.
- Part parity: full parity with the same part set as `Calendar`, plus range-specific data attrs on cell triggers.
- Known adapter deviations: none beyond Leptos event and ref mechanics.

## 4. Part Mapping

| Core part / structure                   | Required?   | Adapter rendering target              | Ownership     | Attr source                 | Notes                                                             |
| --------------------------------------- | ----------- | ------------------------------------- | ------------- | --------------------------- | ----------------------------------------------------------------- |
| root and header bundle                  | required    | same host structure as `Calendar`     | adapter-owned | range-calendar API attrs    | Includes pending-range state attrs.                               |
| `GridGroup` and repeated `Grid` bundles | conditional | multi-month wrapper + repeated tables | adapter-owned | offset-aware grid helpers   | Default `visible_months` remains two.                             |
| `Cell` + `CellTrigger`                  | required    | `<td>` + inner `<button>`             | adapter-owned | range-specific cell helpers | Carries start, end, in-range, hover-range, and anchor data attrs. |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                           | Adapter-owned attrs                  | Consumer attrs          | Merge order                                   | Ownership notes                             |
| ------------- | ------------------------------------ | ------------------------------------ | ----------------------- | --------------------------------------------- | ------------------------------------------- |
| `Root`        | ids, state, readonly, disabled       | pending-range marker                 | wrapper decoration only | core semantic attrs win                       | Root remains adapter-owned.                 |
| `CellTrigger` | selection, range markers, aria-label | pointer hover handler, focus handler | decoration only         | core data and aria attrs win                  | Consumers must not remove range data attrs. |
| live heading  | heading attrs and labels             | polite live-region behavior          | none                    | adapter preserves single visible live heading | Avoid duplicate announcements.              |

## 6. Composition / Context Contract

- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: overlay utilities when embedded in pickers.
- Missing required context behavior: fail fast in debug; degrade gracefully only through the documented `ArsProvider` fallback policy in `08-adapter-leptos.md` §13.
- Composition rule: picker adapters may wrap `RangeCalendar`, but they must not split the calendar’s anchor and hover bookkeeping across wrapper state.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync updates the machine’s range value after mount.
- Pointer enter and leave normalize onto `HoverDate` and `HoverEnd`; click and keyboard activation normalize onto `SelectDate`.
- `disabled`, `readonly`, and bounds changes update selection and preview gating immediately.
- No separate form bridge exists for standalone range calendar.

## 8. Registration and Cleanup Contract

| Registered entity        | Registration trigger              | Identity key     | Cleanup trigger                                 | Cleanup action                 | Notes                                     |
| ------------------------ | --------------------------------- | ---------------- | ----------------------------------------------- | ------------------------------ | ----------------------------------------- |
| hovered cell bookkeeping | pointer enter on cell trigger     | data-derived     | pointer leave, selection completion, or cleanup | clear hover range state        | Never retain stale preview after unmount. |
| focused cell node ref    | cell button mount                 | composite        | cell rerender or unmount                        | drop node handle               | Used for focus restoration.               |
| live announcement effect | anchor or completed range changes | instance-derived | effect rerun or cleanup                         | cancel stale announcement work | Needed for start and completion messages. |

## 9. Ref and Node Contract

| Target part / node   | Ref required? | Ref owner     | Node availability                  | Composition rule | Notes                                              |
| -------------------- | ------------- | ------------- | ---------------------------------- | ---------------- | -------------------------------------------------- |
| root                 | no            | adapter-owned | always structural, handle optional | no composition   | Structural only.                                   |
| navigation triggers  | yes           | adapter-owned | required after mount               | no composition   | Used in embedded picker focus return.              |
| focused cell trigger | yes           | adapter-owned | required after mount               | keyed by date    | Never infer focusable node from range state alone. |
| hover preview nodes  | no            | adapter-owned | always structural, handle optional | no composition   | Derived from cell attrs only.                      |

## 10. State Machine Boundary Rules

- Machine-owned state: selected range, anchor date, hovering date, focused date, visible month, and normalized range membership.
- Adapter-local derived bookkeeping: mounted node refs and deduped live-announcement effects.
- Forbidden local mirrors: range endpoints, hover preview span, and focus state.
- Allowed snapshot-read contexts: render derivation, pointer handlers, and focus restoration.

## 11. Callback Payload Contract

| Callback                                           | Payload source             | Payload shape                                                         | Timing                          | Cancelable? | Notes                                                              |
| -------------------------------------------------- | -------------------------- | --------------------------------------------------------------------- | ------------------------------- | ----------- | ------------------------------------------------------------------ |
| `on_value_change`                                  | machine-derived snapshot   | `Option<DateRange>`                                                   | after completed range selection | no          | First click that only sets anchor does not emit a completed range. |
| `on_range_preview_change` when exposed by wrappers | normalized adapter payload | `{ anchor: Option<CalendarDate>, preview_end: Option<CalendarDate> }` | after hover updates             | no          | Optional wrapper callback only.                                    |
| announcement hook when exposed by wrappers         | none                       | `()`                                                                  | after announcement dispatch     | no          | Diagnostics only.                                                  |

## 12. Failure and Degradation Rules

- `visible_months == 0`: `fail fast` in debug and `degrade gracefully` to `2` in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the adapter foundation provides the documented fallback.
- Hover preview without a mounted pointer-capable host: `no-op`.
- Impossible bounds or range outside bounds: `warn and ignore` invalid input and keep the machine-clamped range.
- SSR-only absence of live-region host behavior: `SSR-safe empty`.

## 13. Identity and Key Policy

- Grid bundles use `composite` identity from base id plus month offset.
- Cells and hover-preview membership use `data-derived` identity from `CalendarDate`.
- Announcement effects use `instance-derived` identity.
- Hidden inputs are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render outputs stable multi-month structure and range attrs for any provided controlled value.
- Hover preview, focus movement, and live announcements are client-only.
- Ref-based focus restoration waits until mount.
- Hydration must preserve identical per-grid heading ids and table order.

## 15. Performance Constraints

- Keep hover preview updates limited to visible cells.
- Avoid rebuilding all month tables when only hover state changes.
- Deduplicate live announcements for repeated hover over the same date.
- Keep node-ref maps instance-scoped.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type         | Why it matters                                                                                     |
| -------------------------------- | ----------- | ----------------------- | -------------------------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required    | context contract        | Month headings, weekday labels, announcements, preview text, and inherited direction depend on it. |
| shared calendar grid helpers     | required    | shared helper           | Range calendar shares month and row computation with `Calendar`.                                   |
| live-region helper               | required    | shared helper           | Range-start and range-complete announcements need dedupe and cleanup.                              |
| pointer normalization            | recommended | behavioral prerequisite | Hover preview should not depend on raw browser quirks.                                             |
| utility button semantics         | recommended | composition contract    | Navigation triggers should remain aligned with utility adapters.                                   |

## 17. Recommended Implementation Sequence

1. Reuse the base calendar grid rendering and navigation structure.
2. Add range-specific cell attrs and selection semantics.
3. Add hover preview and cleanup.
4. Add start and complete announcements.
5. Finalize focus restoration and picker embedding notes.

## 18. Anti-Patterns

- Do not compute in-range and hover-range styling outside the machine-derived snapshot.
- Do not keep stale hover preview after selection completion.
- Do not treat every selected cell as `aria-selected`; only range endpoints expose selection semantics.

## 19. Consumer Expectations and Guarantees

- Consumers may assume range start, range end, in-range, and hover-range attrs are mutually coherent.
- Consumers may assume the first click sets an anchor without immediately completing the range.
- Consumers must not assume hover preview is available during SSR.

## 20. Platform Support Matrix

| Capability / behavior          | Browser client | SSR            | Notes                                                           |
| ------------------------------ | -------------- | -------------- | --------------------------------------------------------------- |
| range selection and navigation | full support   | SSR-safe empty | Structure renders server-side, interaction starts on hydration. |
| hover preview                  | full support   | SSR-safe empty | Pointer-driven behavior is client-only.                         |
| live range announcements       | client-only    | SSR-safe empty | Start and completion announcements are mount-gated.             |
| multi-month rendering          | full support   | full support   | Default remains two months.                                     |

## 21. Debug Diagnostics and Production Policy

| Condition                                  | Debug build behavior | Production behavior | Notes                                            |
| ------------------------------------------ | -------------------- | ------------------- | ------------------------------------------------ |
| invalid `visible_months`                   | fail fast            | degrade gracefully  | Clamp to two in production.                      |
| reversed or out-of-bounds controlled range | debug warning        | warn and ignore     | Machine keeps normalized or clamped range.       |
| missing `ArsProvider` locale/ICU context   | fail fast            | degrade gracefully  | Only through the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required? | Responsibility                          | Reused by                             | Notes                           |
| -------------------- | --------- | --------------------------------------- | ------------------------------------- | ------------------------------- |
| calendar grid helper | yes       | Shared weeks, headings, and pagination  | `calendar`, `range-calendar`          | Avoid drift across adapters.    |
| range-preview helper | yes       | Compute endpoint and preview data attrs | `range-calendar`, `date-range-picker` | Shared concept, not public API. |
| live-region helper   | yes       | Announce anchor and completed range     | `range-calendar`, `date-range-picker` | Must cancel stale work.         |

## 23. Framework-Specific Behavior

- Leptos pointer-enter and pointer-leave handlers should dispatch hover events without allocating per-cell watchers.
- `<For>` keys must remain date-derived even when hover preview changes on every move.
- Client-only announcement effects should be wrapped so SSR emits no browser-side work.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<range_calendar::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <RangeCalendarHeader machine=machine />
        <RangeCalendarGrids machine=machine />
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust
fn cell(machine: UseMachineReturn<range_calendar::Machine>, date: CalendarDate, offset: usize) -> impl IntoView {
    view! {
        <td {..machine.derive(move |api| api.cell_attrs(&date)).get()}>
            <button
                {..machine.derive(move |api| api.cell_trigger_attrs(&date)).get()}
                on:mouseenter=move |_| machine.with_api_snapshot(|api| api.on_cell_hover(&date))
                on:mouseleave=move |_| machine.with_api_snapshot(|api| api.on_hover_end())
            />
        </td>
    }
}
```

## 26. Adapter Invariants

- Completed range selection clears hover preview immediately.
- Only one visible heading remains the live region for month or range-change announcements.
- Range endpoint and in-range attrs always derive from normalized range order.

## 27. Accessibility and SSR Notes

- Range start and range completion announcements remain polite and deduped.
- Endpoint cells expose clear labels for assistive tech; preview-only cells do not impersonate selected endpoints.
- SSR must keep table semantics stable so hydration can attach hover and keyboard logic cleanly.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core range calendar machine and its shared calendar algorithms.
- Intentional deviations: none at the contract level.
- Traceability note: adapter-owned concerns promoted explicitly here include hover cleanup, announcement policy, node refs, and multi-month grouping.

## 29. Test Scenarios

1. First click sets anchor and pending-range state without firing completed-range callback.
2. Hover preview marks in-hover-range cells and clears on leave.
3. Second click completes the range and emits normalized start and end values.
4. Multi-month rendering keeps per-grid headings and range attrs coherent across month boundaries.
5. Controlled range updates re-render endpoint attrs without stale hover preview.

## 30. Test Oracle Notes

- `DOM attrs`: assert `data-ars-range-start`, `data-ars-range-end`, `data-ars-in-range`, and `data-ars-in-hover-range`.
- `machine state`: verify anchor, hover, and normalized range endpoints.
- `callback order`: verify completion callback fires only after the second selection.
- `cleanup side effects`: verify hover preview and announcement work are cleared on unmount.
- Cheap recipe: click one date, hover another, then unmount and confirm no stale live announcement fires.

## 31. Implementation Checklist

- [ ] Hover preview dispatches `HoverDate` and `HoverEnd` without local mirror state.
- [ ] Completed range selection clears preview and emits normalized range.
- [ ] Multi-month rendering stays keyed by date-derived identities.
- [ ] Start and completion announcements are deduped and client-gated.
- [ ] Range endpoint attrs remain on cell triggers, not just cells.
