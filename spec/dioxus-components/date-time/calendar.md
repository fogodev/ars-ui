---
adapter: dioxus
component: calendar
category: date-time
source: components/date-time/calendar.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Calendar — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Calendar`](../../components/date-time/calendar.md) contract onto a Dioxus 0.7.x component. The adapter adds the Dioxus-facing API, multi-month structure, node ownership, live-announcement wiring, and host-specific fallback rules.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct CalendarProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<Option<CalendarDate>>>,
    #[props(optional)]
    pub default_value: Option<CalendarDate>,
    #[props(optional)]
    pub min: Option<CalendarDate>,
    #[props(optional)]
    pub max: Option<CalendarDate>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(optional)]
    pub first_day_of_week: Option<Weekday>,
    #[props(default = 1)]
    pub visible_months: usize,
    pub page_behavior: calendar::PageBehavior,
    #[props(default = false)]
    pub show_week_numbers: bool,
    #[props(default = false)]
    pub is_rtl: bool,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<CalendarDate>>>,
}

#[component]
pub fn Calendar(props: CalendarProps) -> Element
```

Plain props are preferred; `value` becomes a `Signal` only when the adapter must watch controlled updates after mount.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core calendar contract.
- Part parity: full parity with explicit rendering of `GridGroup`, per-grid headings, and cell triggers.
- Known adapter deviations: on desktop and mobile hosts, HTML-specific semantics fall back to logical equivalents when the host cannot express full DOM behavior.

## 4. Part Mapping

| Core part / structure                   | Required?   | Adapter rendering target                            | Ownership     | Attr source               | Notes                                                |
| --------------------------------------- | ----------- | --------------------------------------------------- | ------------- | ------------------------- | ---------------------------------------------------- |
| `Root`                                  | required    | outer container element                             | adapter-owned | `api.root_attrs()`        | Carries state and direction attrs.                   |
| header bundle                           | required    | wrapper with buttons and heading                    | adapter-owned | header and trigger attrs  | Navigation triggers remain separate nodes.           |
| `GridGroup` and repeated `Grid` bundles | conditional | repeated month tables or host-equivalent containers | adapter-owned | offset-aware grid helpers | Default web target is semantic table/grid structure. |
| `Cell` + `CellTrigger`                  | required    | host cell wrapper plus interactive trigger          | adapter-owned | cell and trigger attrs    | Trigger remains the focus target.                    |

## 5. Attr Merge and Ownership Rules

| Target node          | Core attrs                             | Adapter-owned attrs                | Consumer attrs          | Merge order                               | Ownership notes                                                                |
| -------------------- | -------------------------------------- | ---------------------------------- | ----------------------- | ----------------------------------------- | ------------------------------------------------------------------------------ |
| `Root`               | ids, state, direction                  | host classes and utility attrs     | wrapper decoration only | core semantic attrs win                   | Consumers do not replace the root node.                                        |
| triggers             | labels and disabled state              | host event handlers                | class/style only        | core semantic attrs win                   | Buttons remain adapter-owned.                                                  |
| `Grid` / `GridGroup` | roles, labelled-by, multiselect        | repeated ids and host repair       | none                    | adapter applies required grid attrs first | Host fallback may emulate DOM roles logically.                                 |
| `CellTrigger`        | selected, disabled, today, unavailable | host event handlers and data hooks | decoration only         | core data and aria attrs win on web       | Desktop/mobile hosts may expose equivalent logical state instead of DOM attrs. |

## 6. Composition / Context Contract

- Provided contexts: none.
- Required consumed contexts: `ArsProvider` locale, direction, and ICU provider data.
- Optional consumed contexts: environment helpers layered on top of `ArsProvider`.
- Missing required context behavior: fail fast in debug; degrade gracefully only through the documented `ArsProvider` fallback policy in `09-adapter-dioxus.md` §16.
- Composition rule: picker adapters may mount `Calendar` inside overlay utilities without replacing internal grid ownership.

## 7. Prop Sync and Event Mapping

- Controlled `value` sync uses a Dioxus effect that dispatches the machine update path after mount.
- `disabled`, `readonly`, bounds, and unavailable predicates update selection gating immediately.
- `visible_months`, `page_behavior`, and `first_day_of_week` are configuration inputs; wrappers that make them reactive must re-run grid derivation without invalidating stable keys.
- Pointer and keyboard interactions normalize onto the core selection and navigation events.
- No form bridge exists for standalone calendar.

## 8. Registration and Cleanup Contract

| Registered entity        | Registration trigger        | Identity key     | Cleanup trigger          | Cleanup action                   | Notes                            |
| ------------------------ | --------------------------- | ---------------- | ------------------------ | -------------------------------- | -------------------------------- |
| focused cell node ref    | cell trigger mount          | composite        | cell rerender or unmount | drop stored node handle          | Used for focus restoration.      |
| live-announcement effect | heading or selection change | instance-derived | effect rerun or cleanup  | cancel pending announcement work | Avoid stale month announcements. |

## 9. Ref and Node Contract

| Target part / node    | Ref required? | Ref owner     | Node availability                  | Composition rule      | Notes                                     |
| --------------------- | ------------- | ------------- | ---------------------------------- | --------------------- | ----------------------------------------- |
| `Root`                | no            | adapter-owned | always structural, handle optional | no composition        | Structural only.                          |
| navigation triggers   | yes           | adapter-owned | required after mount               | no composition        | Needed for focus return in picker shells. |
| focused `CellTrigger` | yes           | adapter-owned | required after mount               | keyed by visible date | Use mounted refs instead of id lookup.    |
| `GridGroup`           | no            | adapter-owned | always structural, handle optional | no composition        | Render only in multi-month mode.          |

## 10. State Machine Boundary Rules

- Machine-owned state: selected date, focused date, visible month and year, disabled and unavailable semantics, and heading text.
- Adapter-local derived bookkeeping: mounted node refs and live-announcement scheduling.
- Forbidden local mirrors: selected value, focused date, and page state.
- Allowed snapshot-read contexts: render derivation, handlers, and focus restoration.

## 11. Callback Payload Contract

| Callback                                  | Payload source             | Payload shape                                            | Timing                     | Cancelable? | Notes                                |
| ----------------------------------------- | -------------------------- | -------------------------------------------------------- | -------------------------- | ----------- | ------------------------------------ |
| `on_value_change`                         | machine-derived snapshot   | `Option<CalendarDate>`                                   | after successful selection | no          | Only fires for committed selections. |
| `on_page_change` when exposed by wrappers | normalized adapter payload | `{ visible_start: CalendarDate, visible_months: usize }` | after navigation settles   | no          | Optional wrapper-only surface.       |
| focus diagnostics callback when exposed   | normalized adapter payload | `{ focused_date: CalendarDate }`                         | after focus transition     | no          | Optional wrapper-only surface.       |

## 12. Failure and Degradation Rules

- `visible_months == 0`: `fail fast` in debug and `degrade gracefully` to `1` in production.
- Missing `ArsProvider` locale/ICU context: `fail fast` unless the documented foundation fallback applies.
- Missing focused cell node after pagination: `no-op`.
- Impossible bounds (`min > max`): `warn and ignore`.
- Hosts without browser live-region APIs: `degrade gracefully`.

## 13. Identity and Key Policy

- Visible month bundles use `composite` identity from base id plus offset.
- Rows and cells use `data-derived` identity from dates.
- Live-announcement effect uses `instance-derived` identity.
- Hidden inputs are `not applicable`.

## 14. SSR and Client Boundary Rules

- Server render outputs stable root, header, headings, and grid structure for web SSR.
- Refs, focus movement, and live announcements wait until mount.
- Desktop and mobile hosts do not participate in SSR.
- Hydration must preserve per-grid heading ids and month order.

## 15. Performance Constraints

- Keep row and cell keys stable across value changes.
- Avoid recreating node-ref maps for unaffected month bundles.
- Recompute only visible bundles that changed.
- Keep announcement work instance-scoped and cancel stale tasks.

## 16. Implementation Dependencies

| Dependency                       | Required?   | Dependency type      | Why it matters                                                                 |
| -------------------------------- | ----------- | -------------------- | ------------------------------------------------------------------------------ |
| `ArsProvider` locale/ICU context | required    | context contract     | Month and weekday labels, announcements, and inherited direction depend on it. |
| calendar grid helpers            | required    | shared helper        | Offset-aware weeks and headings must stay aligned across adapters.             |
| utility button semantics         | recommended | composition contract | Navigation triggers should reuse button contracts.                             |
| live-region helper               | recommended | shared helper        | Dedupe month-change announcements across hosts.                                |

## 17. Recommended Implementation Sequence

1. Wire the machine and root/header attrs.
2. Implement single-month grid rendering with stable row and cell keys.
3. Add multi-month `GridGroup` rendering and offset helpers.
4. Add node refs and focus restoration.
5. Add live announcements and host fallbacks.

## 18. Anti-Patterns

- Do not duplicate selected or focused date in local Dioxus state.
- Do not put `aria-live` on every per-grid heading in multi-month mode.
- Do not key rows or cells by loop index when the date is available.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the rendered grid follows machine state exactly.
- Consumers may assume multi-month mode preserves one per-grid heading plus one shared range heading.
- Consumers must not assume every host can expose browser DOM attrs directly.

## 20. Platform Support Matrix

| Capability / behavior              | Web          | Desktop       | Mobile        | SSR            | Notes                                                              |
| ---------------------------------- | ------------ | ------------- | ------------- | -------------- | ------------------------------------------------------------------ |
| grid rendering and selection attrs | full support | fallback path | fallback path | SSR-safe empty | Non-web hosts expose logical state when DOM attrs are unavailable. |
| keyboard focus movement            | full support | full support  | fallback path | SSR-safe empty | Mobile may rely on host focus affordances.                         |
| live month announcements           | full support | fallback path | fallback path | SSR-safe empty | Use host announcement APIs when available.                         |
| multi-month layout                 | full support | full support  | fallback path | full support   | Mobile may stack bundles vertically.                               |

## 21. Debug Diagnostics and Production Policy

| Condition                                | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| invalid `visible_months`                 | fail fast            | degrade gracefully  | Clamp to one month.                          |
| inconsistent bounds                      | debug warning        | warn and ignore     | Keep the valid bound.                        |
| missing `ArsProvider` locale/ICU context | fail fast            | degrade gracefully  | Only via the documented foundation fallback. |

## 22. Shared Adapter Helper Notes

| Helper concept       | Required? | Responsibility                          | Reused by                                     | Notes                                |
| -------------------- | --------- | --------------------------------------- | --------------------------------------------- | ------------------------------------ |
| calendar grid helper | yes       | Produce offset-aware weeks and headings | `calendar`, `range-calendar`, picker overlays | Shared across adapters conceptually. |
| live-region helper   | no        | Schedule and dedupe announcements       | `calendar`, `range-calendar`, picker overlays | Host-specific backend.               |
| node-ref helper      | yes       | Track focused trigger nodes safely      | `calendar`, `range-calendar`                  | Must tolerate rerender churn.        |

## 23. Framework-Specific Behavior

- Dioxus effects should dispatch machine updates for controlled signals instead of patching local state.
- Stable hook positions matter for repeated month, row, and cell rendering; extract child components when derivations would otherwise happen inside loops.
- Desktop/mobile hosts may map grid semantics onto native focusable containers rather than DOM tables.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<calendar::Machine>(props);

rsx! {
    div { ..root_attrs,
        CalendarHeader { machine }
        CalendarGrids { machine }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
struct CalendarGridProps {
    pub machine: UseMachineReturn<calendar::Machine>,
    pub offset: usize,
}

#[component]
fn CalendarGrid(props: CalendarGridProps) -> Element {
    let heading = props.machine.derive(move |api| api.heading_text_for(props.offset));
    let weeks = props.machine.derive(move |api| api.weeks_for(props.offset));
    rsx! {
        table { ..props.machine.derive(move |api| api.grid_attrs_for(props.offset)).read().clone() }
    }
}
```

## 26. Adapter Invariants

- The focused cell trigger is the only tabbable cell trigger at any time.
- Multi-month mode renders one hidden per-grid heading plus one shared visible heading.
- Selection and unavailable semantics remain on cell triggers even when hosts fall back from direct DOM attrs.

## 27. Accessibility and SSR Notes

- The visible shared heading remains the only polite live region for month changes.
- Per-grid headings still label each visible bundle.
- Web SSR must keep ids and structure stable so hydration can attach focus logic without replacement.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core calendar machine on web and logical parity on desktop/mobile.
- Intentional deviations: desktop and mobile hosts may expose logical rather than DOM-native grid semantics; form participation is not applicable here.
- Traceability note: adapter-owned concerns promoted here include multi-month grouping, live announcements, node refs, and host fallback behavior.

## 29. Test Scenarios

1. Single-month render exposes correct grid, heading, and selected-cell state.
2. Multi-month render exposes `GridGroup`, per-grid headings, and offset-specific outside-month markers.
3. Navigation updates heading text and keeps focus on the machine-focused date.
4. Disabled and unavailable dates remain selection safe across hosts.
5. Controlled value updates change selected state without replacing unrelated rows.

## 30. Test Oracle Notes

- `DOM attrs`: assert role, selected, disabled, labelled-by, and heading ids on web.
- `machine state`: verify selected and focused dates after navigation on all hosts.
- `rendered structure`: verify one bundle per visible month.
- `cleanup side effects`: verify stale announcement work does not fire after unmount.
- Cheap recipe: render `visible_months=2`, navigate once, and assert only one active shared announcement surface remains.

## 31. Implementation Checklist

- [ ] Controlled value sync dispatches machine updates.
- [ ] Multi-month mode renders `GridGroup` plus one hidden heading per bundle.
- [ ] Focus restoration uses mounted refs rather than id lookup.
- [ ] Announcement behavior is deduped and host-gated.
- [ ] Row and cell keys are date-derived.
