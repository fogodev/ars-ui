---
adapter: leptos
component: calendar
category: date-time
source: components/date-time/calendar.md
source_foundation: foundation/08-adapter-leptos.md
---

# Calendar — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Calendar`](../../components/date-time/calendar.md) contract onto a Leptos 0.8.x component. The adapter adds the Leptos-facing signature, multi-month rendering structure, node ownership, semantic repair, live-announcement wiring, and SSR-safe rules that the agnostic spec leaves to the adapter.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Calendar(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Signal<Option<CalendarDate>>,
    #[prop(optional)] default_value: Option<CalendarDate>,
    #[prop(optional)] min: Option<CalendarDate>,
    #[prop(optional)] max: Option<CalendarDate>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] readonly: bool,
    #[prop(optional)] first_day_of_week: Option<Weekday>,
    #[prop(optional)] visible_months: usize,
    #[prop(optional)] page_behavior: calendar::PageBehavior,
    #[prop(optional)] show_week_numbers: bool,
    #[prop(optional)] is_rtl: bool,
    #[prop(optional)] on_value_change: Option<Callback<Option<CalendarDate>>>,
) -> impl IntoView
```

The adapter exposes the full core prop surface, uses `Signal<Option<CalendarDate>>` for controlled post-mount value sync, and keeps all grid subparts adapter-owned.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core calendar contract, including multi-month pagination, disabled and unavailable dates, and RTL behavior.
- Part parity: full parity with explicit rendering of `GridGroup`, per-grid headings, row and cell triggers.
- Known adapter deviations: none beyond Leptos-specific signal watching and mount-gated node access.

## 4. Part Mapping

| Core part / structure                | Required?   | Adapter rendering target                     | Ownership     | Attr source                                              | Notes                                             |
| ------------------------------------ | ----------- | -------------------------------------------- | ------------- | -------------------------------------------------------- | ------------------------------------------------- |
| `Root`                               | required    | outer `<div>`                                | adapter-owned | `api.root_attrs()`                                       | Carries state and direction attrs.                |
| `Header` + triggers + heading        | required    | header wrapper with buttons and heading text | adapter-owned | `api.header_attrs()`, trigger attrs, heading attrs       | Navigation buttons stay separate from grid nodes. |
| `GridGroup`                          | conditional | wrapper around repeated month tables         | adapter-owned | `api.grid_group_attrs()`                                 | Render only when `visible_months > 1`.            |
| `Grid`, `HeadRow`, `HeadCell`, `Row` | required    | `<table>`, `<tr>`, `<th>`                    | adapter-owned | offset-specific grid helpers                             | One grid bundle per visible month.                |
| `Cell` + `CellTrigger`               | required    | `<td>` + inner `<button>`                    | adapter-owned | `api.cell_attrs_for(...)`, `api.cell_trigger_attrs(...)` | Trigger is the focus target and selection target. |

## 5. Attr Merge and Ownership Rules

| Target node          | Core attrs                                   | Adapter-owned attrs                         | Consumer attrs               | Merge order                                           | Ownership notes                                                         |
| -------------------- | -------------------------------------------- | ------------------------------------------- | ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------- |
| `Root`               | state, direction, ids                        | structural class hooks, utility-layer attrs | host wrapper decoration only | core semantic attrs win; class/style merge additively | Consumers do not replace the root node.                                 |
| navigation triggers  | button labels, disabled state                | trigger handlers, utility button attrs      | visual decoration only       | core disabled and aria attrs win                      | Trigger semantics may not be stripped.                                  |
| `Grid` / `GridGroup` | role, labelled-by, multiselect               | repeated ids, grouping repair               | none                         | adapter applies required grid attrs first             | Multi-month grouping is adapter-owned.                                  |
| `CellTrigger`        | selected, disabled, unavailable, today flags | focus handler, hover handler, data attrs    | decorative class/style only  | machine-required aria/data attrs win                  | Consumer attrs must not remove `tabindex`, `disabled`, or `aria-label`. |

## 6. Composition / Context Contract

- Provided contexts: none.
- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data from the Leptos adapter foundation.
- Optional consumed contexts: environment helpers layered on top of `ArsProvider` when the application uses shared utilities.
- Missing required context behavior: fail fast in debug; degrade gracefully only through the documented `ArsProvider` fallback policy in `08-adapter-leptos.md` §13.
- Composition rule: `Calendar` is standalone, but picker adapters may mount it inside `Dismissable` and `FocusScope` without replacing its internal grid structure.

## 7. Prop Sync and Event Mapping

- Controlled value sync uses a post-mount watcher that dispatches the core `SetValue` path without recreating grid structure.
- `disabled`, `readonly`, and unavailable-date predicates must affect trigger attrs and selection gating immediately after prop change.
- `visible_months`, `page_behavior`, and `first_day_of_week` are treated as configuration inputs; wrappers that make them reactive must re-run grid derivation without replacing stable keyed rows unnecessarily.
- Pointer and keyboard interactions normalize onto the machine’s `SelectDate`, `FocusDate`, `NextMonth`, `PrevMonth`, `NextYear`, and `PrevYear` events.
- No form bridge exists for standalone calendar; form participation belongs to picker or field wrappers.

## 8. Registration and Cleanup Contract

| Registered entity        | Registration trigger       | Identity key     | Cleanup trigger                   | Cleanup action                   | Notes                                          |
| ------------------------ | -------------------------- | ---------------- | --------------------------------- | -------------------------------- | ---------------------------------------------- |
| focused cell node ref    | cell button mount          | composite        | cell rerender or unmount          | drop stored node handle          | Used for focus restoration after pagination.   |
| live announcement effect | heading / selection change | instance-derived | effect rerun or component cleanup | cancel pending announcement work | Cleanup must not leak old month announcements. |

## 9. Ref and Node Contract

| Target part / node         | Ref required? | Ref owner     | Node availability                  | Composition rule      | Notes                                         |
| -------------------------- | ------------- | ------------- | ---------------------------------- | --------------------- | --------------------------------------------- |
| `Root`                     | no            | adapter-owned | always structural, handle optional | no composition        | Structural only.                              |
| navigation trigger buttons | yes           | adapter-owned | required after mount               | no composition        | Needed for focus return within picker shells. |
| focused `CellTrigger`      | yes           | adapter-owned | required after mount               | keyed by visible date | Never address focused cells by id alone.      |
| `GridGroup`                | no            | adapter-owned | always structural, handle optional | no composition        | Rendered only for multi-month mode.           |

## 10. State Machine Boundary Rules

- Machine-owned state: selected date, focused date, visible month and year, disabled and unavailable semantics, heading text, and selection attrs.
- Adapter-local derived bookkeeping: mounted node refs and live-announcement scheduling.
- Forbidden local mirrors: selected value, focused date, page index, and cell disabled state.
- Allowed snapshot-read contexts: render derivation, key handlers, pointer hover, and focus restoration.

## 11. Callback Payload Contract

| Callback                                            | Payload source             | Payload shape                                            | Timing                             | Cancelable? | Notes                                                |
| --------------------------------------------------- | -------------------------- | -------------------------------------------------------- | ---------------------------------- | ----------- | ---------------------------------------------------- |
| `on_value_change`                                   | machine-derived snapshot   | `Option<CalendarDate>`                                   | after `SelectDate` commits         | no          | Only fires for successful selections.                |
| `on_page_change` when exposed by wrappers           | normalized adapter payload | `{ visible_start: CalendarDate, visible_months: usize }` | after prev/next navigation settles | no          | Useful for analytics, not required for base adapter. |
| focus diagnostics callback when exposed by wrappers | normalized adapter payload | `{ focused_date: CalendarDate }`                         | after focus transition             | no          | Never fires for disabled cells.                      |

## 12. Failure and Degradation Rules

- `visible_months == 0`: `fail fast` in debug and `degrade gracefully` to `1` in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the adapter foundation defines a documented fallback, in which case `degrade gracefully`.
- Impossible prop combination such as `min > max`: `warn and ignore` the impossible bound and leave the machine clamped to the valid side.
- Missing focused cell node after pagination: `no-op` for focus restoration and keep machine focus state intact.
- SSR-only absence of browser APIs for announcements: `SSR-safe empty`.

## 13. Identity and Key Policy

- Visible month bundles use `composite` identity from base id plus month offset.
- Rows use `data-derived` identity from first visible date in the week.
- Cells use `data-derived` identity from `CalendarDate`.
- Live-announcement effect uses `instance-derived` identity.
- Hidden input identity is `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render outputs stable root, header, headings, and grid structure.
- Ref access, focus movement, and live announcements wait until hydration and mount.
- Per-cell listeners are attached only on the client.
- Multi-month structure must remain hydration-stable for the same `visible_months` configuration.

## 15. Performance Constraints

- Keep row and cell keys stable across value changes.
- Avoid recreating node-ref maps for unaffected visible months.
- Recompute offset-specific grid data incrementally rather than rebuilding unrelated months.
- Keep announcement effects instance-scoped and cancel stale work on rerender.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type      | Why it matters                                                                     |
| -------------------------------- | ----------- | -------------------- | ---------------------------------------------------------------------------------- |
| `ArsProvider` locale/ICU context | required    | context contract     | Month, weekday, announcement text, and inherited direction depend on it.           |
| calendar grid helpers            | required    | shared helper        | Offset-aware headings, weeks, and cell attrs must stay consistent across adapters. |
| utility `button` semantics       | recommended | composition contract | Navigation triggers should reuse existing trigger semantics.                       |
| live-region helper               | recommended | shared helper        | Keeps month-change and selection announcements consistent.                         |

## 17. Recommended Implementation Sequence

1. Wire the machine and root/header attrs.
2. Implement single-month grid rendering with stable row and cell keys.
3. Add multi-month `GridGroup` rendering and offset-specific heading/grid helpers.
4. Add node refs and focus restoration.
5. Add live announcements and finalize unavailable/disabled repair.

## 18. Anti-Patterns

- Do not duplicate selected or focused date in a local Leptos signal.
- Do not put `aria-live` on every per-grid heading in multi-month mode.
- Do not key rows or cells by loop index when the date itself is available.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the rendered grid always matches the machine’s focused and selected state.
- Consumers may assume multi-month mode preserves one heading per visible month plus one range heading for the group.
- Consumers must not assume they can replace cell triggers with arbitrary child nodes.

## 20. Platform Support Matrix

| Capability / behavior              | Browser client | SSR            | Notes                                            |
| ---------------------------------- | -------------- | -------------- | ------------------------------------------------ |
| grid rendering and selection attrs | full support   | SSR-safe empty | Server output is structural and non-interactive. |
| keyboard focus movement            | full support   | SSR-safe empty | Requires mounted button refs.                    |
| live month announcements           | client-only    | SSR-safe empty | No announcement work on the server.              |
| multi-month layout                 | full support   | full support   | Layout styling remains consumer-owned.           |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                                          |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------------------------- |
| `visible_months == 0`                    | fail fast            | degrade gracefully  | Clamp to one month in production.                              |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep the valid bound.                                          |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only if the adapter foundation offers the documented fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required? | Responsibility                                          | Reused by                                     | Notes                                     |
| -------------------- | --------- | ------------------------------------------------------- | --------------------------------------------- | ----------------------------------------- |
| calendar grid helper | yes       | Produce offset-aware weeks, headings, and cell metadata | `calendar`, `range-calendar`, picker overlays | Shared across both adapters conceptually. |
| live-region helper   | no        | Schedule and dedupe month announcements                 | `calendar`, `range-calendar`, picker overlays | Keep it instance-scoped.                  |
| node-ref helper      | yes       | Track focused trigger nodes safely                      | `calendar`, `range-calendar`                  | Must tolerate rerender churn.             |

## 23. Framework-Specific Behavior

- Leptos `<For>` should key month rows and cells by date-derived identity, not offset-only identity.
- Signal watchers for controlled value changes should dispatch machine events rather than mutate local derived state.
- Focus restoration should happen from mount-safe node refs, not `document.getElementById`.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<calendar::Machine>(props);
let offsets = move || 0..machine.derive(|api| api.visible_month_count()).get();

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <header {..machine.derive(|api| api.header_attrs()).get()}>
            <button {..machine.derive(|api| api.prev_trigger_attrs()).get()} />
            <span {..machine.derive(|api| api.heading_text_attrs()).get()}>
                {move || machine.derive(|api| api.range_heading_text()).get()}
            </span>
            <button {..machine.derive(|api| api.next_trigger_attrs()).get()} />
        </header>
        <div {..machine.derive(|api| api.grid_group_attrs()).get()}>
            <For each=offsets key=|offset| *offset let:offset>
                <CalendarGrid machine=machine offset=offset />
            </For>
        </div>
    </div>
}
```

## 25. Reference Implementation Skeleton

```rust
#[component]
fn CalendarGrid(machine: UseMachineReturn<calendar::Machine>, offset: usize) -> impl IntoView {
    let heading = machine.derive(move |api| api.heading_text_for(offset));
    let weeks = machine.derive(move |api| api.weeks_for(offset));

    view! {
        <span {..machine.derive(move |api| api.heading_text_attrs_for(offset)).get()} class="sr-only">
            {move || heading.get()}
        </span>
        <table {..machine.derive(move |api| api.grid_attrs_for(offset)).get()}>
            // head row and keyed weeks
        </table>
    }
}
```

## 26. Adapter Invariants

- The focused cell trigger is the only tabbable cell trigger at any time.
- Multi-month mode renders one hidden per-grid heading per table plus one visible range heading for the whole bundle.
- Selection and unavailable semantics stay on cell triggers even when consumers decorate cells.

## 27. Accessibility and SSR Notes

- The visible range heading remains the only polite live region for month changes.
- Per-grid hidden headings still label each table for assistive tech.
- SSR output must preserve table semantics and heading ids so hydration can attach focus logic without replacing structure.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core calendar machine, including multi-month behavior.
- Intentional deviations: none at the contract level; Leptos-specific watcher and ref mechanics are implementation details.
- Traceability note: adapter-owned concerns promoted explicitly here include multi-month grouping, live announcements, node refs, and SSR gating.

## 29. Test Scenarios

1. Single-month render exposes correct grid, heading, and selected-cell attrs.
2. Multi-month render exposes `GridGroup`, per-grid headings, and offset-specific outside-month attrs.
3. Navigation updates heading text and keeps focus on the machine-focused date when the node still exists.
4. Disabled and unavailable dates remain focus and selection safe.
5. Controlled value updates change selected attrs without replacing unrelated rows.

## 30. Test Oracle Notes

- `DOM attrs`: assert `role`, `aria-selected`, `aria-disabled`, `aria-labelledby`, and month heading ids.
- `machine state`: verify selected and focused dates after keyboard and pointer navigation.
- `rendered structure`: verify one `GridGroup` and one table per visible month.
- `cleanup side effects`: verify stale announcement effects do not fire after unmount.
- Cheap recipe: render `visible_months=2`, navigate once, and assert only one polite live heading remains.

## 31. Implementation Checklist

- [ ] Controlled value sync dispatches machine updates instead of mutating local mirrors.
- [ ] Multi-month mode renders `GridGroup` plus one hidden heading per grid.
- [ ] Focus restoration uses mounted node refs rather than document queries.
- [ ] Month-change announcements are deduped and client-gated.
- [ ] Cell keys and row keys are data-derived.
