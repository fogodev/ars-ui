---
adapter: dioxus
component: tabs
category: navigation
source: components/navigation/tabs.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Tabs — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Tabs`](../../components/navigation/tabs.md) contract onto Dioxus 0.7.x. The adapter preserves compound tablist composition, roving focus, selection sync, indicator measurement, lazy panel presence, closable-tab support, and reorder announcements.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    #[props(optional)]
    pub value: Option<Key>,
    pub default_value: Key,
    pub tabs: Vec<tabs::TabDef>,
    pub orientation: Orientation,
    pub activation_mode: tabs::ActivationMode,
    pub dir: Direction,
    #[props(default = false)]
    pub loop_focus: bool,
    #[props(default = false)]
    pub disallow_empty_selection: bool,
    #[props(default = false)]
    pub lazy_mount: bool,
    #[props(default = false)]
    pub unmount_on_exit: bool,
    pub disabled_keys: BTreeSet<Key>,
    #[props(default = false)]
    pub reorderable: bool,
    pub children: Element,
}

#[component]
pub fn Tabs(props: TabsProps) -> Element
```

The adapter owns ordered tab registration, selected-tab indicator measurement, panel presence policy, closable-tab trigger semantics, and reorder announcements.

## 3. Mapping to Core Component Contract

- Props parity: full parity with selected tab, activation mode, orientation, direction, disabled keys, and lazy panel behavior.
- State parity: full parity with selected tab, focused tab, and focus-visible state.
- Part parity: full parity with `Root`, `List`, repeated `Tab`, `Panel`, and `Indicator`, plus closable variant `CloseTrigger`.
- Adapter additions: explicit ordered registration, measurement helpers, and live-region ownership for reorder announcements.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                            | Ownership     | Attr source                                 | Notes                                  |
| --------------------- | ----------- | ------------------------------------------------------------------- | ------------- | ------------------------------------------- | -------------------------------------- |
| `Root`                | required    | `<div>`                                                             | adapter-owned | `api.root_attrs()`                          | owns context and panel presence policy |
| `List`                | required    | `<div>`                                                             | adapter-owned | `api.list_attrs()`                          | `role="tablist"`                       |
| `Tab`                 | repeated    | `<button>` or `<a>` when a tab definition opts into link navigation | adapter-owned | `api.tab_attrs(key, panel_id)`              | roving focus target                    |
| `Indicator`           | optional    | `<span>`                                                            | adapter-owned | `api.tab_indicator_attrs()`                 | measurement-driven visual part         |
| `Panel`               | repeated    | `<div>`                                                             | adapter-owned | `api.panel_attrs(key, tab_id)`              | `role="tabpanel"`                      |
| `CloseTrigger`        | conditional | `<button>`                                                          | adapter-owned | adapter-owned attrs plus core variant rules | closable tabs only                     |
| live region           | conditional | hidden `<div>`                                                      | adapter-owned | adapter-owned attrs                         | reorder announcement surface           |

## 5. Attr Merge and Ownership Rules

| Target node      | Core attrs                                                       | Adapter-owned attrs                                                        | Consumer attrs                         | Merge order                             | Ownership notes                         |
| ---------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------- | --------------------------------------- | --------------------------------------- |
| `List` and `Tab` | tablist roles, selection, controls, tabindex, and disabled attrs | roving keydown handlers, measurement hooks, and route-aware host selection | decoration attrs and trailing handlers | ARIA, tabindex, and selection attrs win | tabs remain adapter-owned focus targets |
| `Panel`          | labelledby, hidden, and current selection attrs                  | lazy-mount or unmount presence policy                                      | decoration attrs                       | linkage and visibility attrs win        | panel ownership stays adapter-side      |
| `Indicator`      | decorative attrs                                                 | measurement-derived CSS custom properties                                  | none                                   | adapter measurement wins                | visual only                             |

## 6. Composition / Context Contract

`Tabs` publishes required root context containing machine access, ordered tab registration, selected tab measurement, and panel presence helpers. Tab and panel surfaces consume that context and fail fast when rendered outside the root boundary.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                           | Mode       | Sync trigger            | Machine event / update path          | Visible effect                                    | Notes                                            |
| ---------------------------------------------------------------------- | ---------- | ----------------------- | ------------------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| `value`                                                                | controlled | prop change after mount | `SelectTab` or controlled sync event | updates selected tab, indicator, and active panel | no controlled/uncontrolled switching after mount |
| `orientation`, `activation_mode`, `dir`, `loop_focus`, `disabled_keys` | controlled | rerender with new props | core prop rebuild                    | updates roving navigation and activation guards   | registry identity remains stable                 |
| `lazy_mount`, `unmount_on_exit`                                        | controlled | rerender with new props | adapter presence policy              | changes panel mount lifecycle                     | not machine state                                |
| `reorderable`                                                          | controlled | rerender with new props | adapter behavior gate                | enables reorder announcements and controls        | no hidden reorder mode                           |

| UI event                 | Preconditions           | Machine event / callback path                       | Ordering notes                                    | Notes                                        |
| ------------------------ | ----------------------- | --------------------------------------------------- | ------------------------------------------------- | -------------------------------------------- |
| tab click or Enter/Space | target tab not disabled | `SelectTab(key)`                                    | selection commits before trailing callbacks       | manual mode still allows explicit activation |
| roving arrow navigation  | enabled tabs exist      | `FocusNext`, `FocusPrev`, `FocusFirst`, `FocusLast` | automatic mode may also commit selection          | RTL swaps only horizontal sibling navigation |
| tab focus                | tab receives focus      | `Focus { tab, is_keyboard }`                        | focus-visible state settles first                 | selected tab may differ in manual mode       |
| close trigger activation | tab is closable         | `CloseTab(key)`                                     | close event fires before consumer removes the tab | trigger is not part of roving order          |
| reorder action           | `reorderable=true`      | reorder callback plus live announcement             | announcement follows committed new order          | adapter owns announcer                       |

## 8. Registration and Cleanup Contract

- Each tab registers in DOM order on mount.
- Cleanup removes tabs and their measurement targets from the registry.
- Reorder announcements must cancel stale queued messages on unmount or rapid successive reorders.

## 9. Ref and Node Contract

| Target part / node      | Ref required? | Ref owner                                        | Node availability                     | Composition rule                                          | Notes                                              |
| ----------------------- | ------------- | ------------------------------------------------ | ------------------------------------- | --------------------------------------------------------- | -------------------------------------------------- |
| each `Tab`              | yes           | shared between adapter and optional wrapper refs | required after mount                  | compose adapter node handle with any exposed consumer ref | roving focus and measurement depend on live nodes  |
| `List`                  | yes           | adapter-owned                                    | required after mount                  | no composition                                            | indicator measurements are relative to list bounds |
| hidden live-region root | conditional   | adapter-owned                                    | required after mount when reorderable | no composition                                            | announcer only when reorder support exists         |

## 10. State Machine Boundary Rules

- Selected tab, focused tab, disabled tab guards, and activation mode remain core-owned.
- Ordered registration, indicator measurement, panel presence policy, and reorder announcements remain adapter-owned.
- The adapter must not keep an unsynchronized selected-tab mirror.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape                                      | Timing                    | Cancelable? | Notes                              |
| --------------------- | ------------------------ | -------------------------------------------------- | ------------------------- | ----------- | ---------------------------------- |
| value-change callback | machine-derived snapshot | `{ key: Key }`                                     | after committed selection | no          | wrapper-owned                      |
| reorder callback      | adapter-derived          | `{ key: Key, old_index: usize, new_index: usize }` | after committed reorder   | yes         | announcement follows this callback |

## 12. Failure and Degradation Rules

| Condition                                           | Policy             | Notes                                                       |
| --------------------------------------------------- | ------------------ | ----------------------------------------------------------- |
| selected key no longer exists after consumer update | warn and ignore    | move to nearest valid fallback per core rules               |
| selected tab cannot be measured for indicator       | degrade gracefully | render indicator hidden or stale-free rather than incorrect |
| reorder announcement helper unavailable             | degrade gracefully | reorder still commits without announcement                  |

## 13. Identity and Key Policy

Tab identity is the tab key. Registration order, panel linkage, and reorder operations all key off that stable identity. Server and client must agree on the initial tab order and selection.

## 14. SSR and Client Boundary Rules

- SSR renders the selected tab, tablist, and active panel branch from initial props.
- Indicator measurement and reorder announcements are client-only.
- Server and client must preserve tab host choice, key order, and the initial panel presence branch.

## 15. Performance Constraints

- Keep tab registration incremental.
- Measure only the selected tab for indicator updates rather than all tabs eagerly.
- Avoid remounting inactive panels when only selection attrs change and presence policy does not require unmount.

## 16. Implementation Dependencies

| Dependency                  | Required?                 | Dependency type     | Why it must exist first                                       | Notes                                      |
| --------------------------- | ------------------------- | ------------------- | ------------------------------------------------------------- | ------------------------------------------ |
| ordered registration helper | required                  | registration helper | roving focus and reorder semantics depend on stable tab order | shared with `accordion`                    |
| live-region helper          | required when reorderable | behavioral helper   | reorder announcements are adapter-owned                       | reuse pagination-style announcer semantics |
| measurement helper          | recommended               | measurement helper  | indicator positioning depends on selected tab and list bounds | shared with `navigation-menu`              |

## 17. Recommended Implementation Sequence

1. Initialize the core tabs machine.
2. Publish root context and register tabs in DOM order.
3. Render tablist, tabs, indicator, and panels.
4. Add controlled selection sync plus lazy panel presence.
5. Add closable tabs, reorder behavior, and live announcements.

## 18. Anti-Patterns

- Do not put close triggers into the roving tab order.
- Do not set `aria-multiselectable` on the tablist.
- Do not skip disabled-tab guards during click or keyboard activation.

## 19. Consumer Expectations and Guarantees

- Consumers may assume exactly one tab is selected at a time.
- Consumers may assume panel linkage remains stable across reorder operations.
- Consumers must not assume inactive panels stay mounted unless the documented presence props require it.

## 20. Platform Support Matrix

| Capability / behavior                                     | Web          | Desktop      | Mobile       | SSR          | Notes                        |
| --------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ---------------------------- |
| tablist semantics, roving focus, and active panel linkage | full support | full support | full support | full support | baseline tabs behavior       |
| indicator measurement                                     | full support | full support | full support | client-only  | visual enhancement only      |
| reorder announcement                                      | full support | full support | full support | client-only  | only when `reorderable=true` |

## 21. Debug Diagnostics and Production Policy

| Condition                                     | Debug build behavior | Production behavior | Notes                        |
| --------------------------------------------- | -------------------- | ------------------- | ---------------------------- |
| missing root context for tab or panel surface | fail fast            | fail fast           | compound structure violation |
| selected key missing from registry            | debug warning        | warn and ignore     | fallback selection path      |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required?                 | Responsibility                           | Reused by                      | Notes                       |
| --------------------------- | ------------------------- | ---------------------------------------- | ------------------------------ | --------------------------- |
| ordered registration helper | required                  | tracks tab order and cleanup             | `accordion`, `navigation-menu` | roving source of truth      |
| measurement helper          | recommended               | computes indicator CSS custom properties | `navigation-menu`              | relative to list bounds     |
| live-region helper          | required when reorderable | announces committed reorder actions      | `pagination`                   | keeps initial render silent |

## 23. Framework-Specific Behavior

Dioxus should sync the controlled selected key through props, compose each tab node handle with any wrapper ref, and derive indicator styles reactively from the selected tab and list measurements.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<tabs::Machine>(props);
use_context_provider(|| Context::from_machine(machine));

rsx! {
    div { ..machine.derive(|api| api.root_attrs()),
        {children}
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine and ordered registry from current props.
- Register each tab with a live node handle.
- Render panels using the documented lazy or unmount presence policy.
- Update indicator CSS variables only from the selected tab measurement.
- Announce reorders only after the new order is committed.

## 26. Adapter Invariants

- Exactly one tab is selected at a time.
- Only tab surfaces participate in the roving order.
- Panel linkage remains stable by key even across reorder operations.
- Reorder announcements never fire before the order actually changes.

## 27. Accessibility and SSR Notes

- `List` owns `role="tablist"` and orientation attrs.
- Disabled tabs stay discoverable via `aria-disabled` even when activation is blocked.
- SSR must preserve initial selected tab, tab order, and panel presence.

## 28. Parity Summary and Intentional Deviations

- Matches the core tabs contract without intentional adapter divergence.
- Promotes ordered registration, indicator measurement, panel presence, close-trigger behavior, and reorder announcements into explicit Dioxus-facing rules.

## 29. Test Scenarios

- automatic vs manual activation
- disabled-tab navigation and blocked activation
- lazy-mounted and unmount-on-exit panel behavior
- closable tab with preserved roving order
- reorderable tab set with live announcement
- indicator measurement update after selection change

## 30. Test Oracle Notes

- Inspect DOM attrs for `role="tab"`, `aria-selected`, `aria-controls`, and `aria-labelledby`.
- Verify close triggers do not receive roving tabindex.
- Use browser tests to assert indicator CSS variables update from the selected tab.
- Assert reorder announcements stay silent on initial render and fire after committed reorder only.

## 31. Implementation Checklist

- [ ] Register tabs in DOM order and clean them up by key.
- [ ] Keep exactly one selected tab.
- [ ] Keep close triggers out of the roving order.
- [ ] Apply lazy panel presence only through the documented props.
- [ ] Drive indicator measurement from the selected tab only.
- [ ] Announce reorder actions only after the new order commits.
