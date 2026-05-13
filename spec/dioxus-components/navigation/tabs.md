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

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps<K: TabKey> {
    #[props(optional)]
    pub value: Option<Option<K>>,
    #[props(into)]
    pub default_value: K,
    #[props(into)]
    pub tabs: tabs::TabsSource<K>,
    pub orientation: Orientation,
    pub activation_mode: tabs::ActivationMode,
    pub dir: Direction,
    #[props(default = true)]
    pub loop_focus: bool,
    #[props(default = false)]
    pub disallow_empty_selection: bool,
    #[props(default = false)]
    pub lazy_mount: bool,
    #[props(default = false)]
    pub unmount_on_exit: bool,
    pub disabled_keys: BTreeSet<K>,
    #[props(default = false)]
    pub reorderable: bool,
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<K>>>,
    #[props(optional)]
    pub on_close_tab: Option<EventHandler<K>>,
    #[props(optional)]
    pub on_reorder: Option<Callback<tabs::ReorderEvent<K>, bool>>,
    pub children: Element,
}

#[component]
pub fn Tabs<K: TabKey>(props: TabsProps<K>) -> Element
```

`K` is a typed public tab identifier that implements `TabKey`
(`Copy + Eq + Ord + Send + Sync + 'static`). The adapter converts
`K` into the agnostic core's internal `Key` at the machine boundary,
then maps committed selection, close, and reorder callbacks back to
`K`. This keeps `Key` an implementation detail and prevents mixing
unrelated application enum families in a single tablist.

`value` is `Option<Option<K>>`. The outer `Option` distinguishes
controlled-vs-uncontrolled mode (fixed at mount per agnostic-core
spec §1.5); the inner `Option<K>` carries the live selection so a
controlled consumer can express "no tab selected" without flipping
out of controlled mode. Inner-value changes flow through
`Event::SyncControlledValue` automatically.

`tabs` accepts inline rows (`Vec<Tab<K>>` or `[Tab<K>; N]`) for the
common static-list case, or a reactive `ReadStore<Vec<Tab<K>>>` from
an `ars_dioxus::dioxus_stores::Store<…>` when the consumer owns a
dynamic list. Inline rows are copied into an adapter-owned store at
mount, so close and reorder interactions still mutate rendered tab
order without requiring consumer store setup. Mutations of either
store source (push, remove, reorder, swap of a tab's `closable` or
`disabled` flag) re-dispatch `Event::SetTabs` / `Event::SyncProps` on
the next render via fingerprint diffing, without remounting the
`Tabs` component.

`Tabs` is a single, monolithic compound component: per-tab content is
supplied via the `tabs` field (each `Tab` carries its own label,
panel content, optional link `href`, and per-row `disabled` /
`closable` flags). Consumers do not compose individual `<Tab>` /
`<TabsPanel>` parts; the adapter renders the full anatomy internally.

`Tab::new(key, panel)` is the preferred constructor for static application
tab lists. It requires `K: TabKey + Translate`, uses the key enum's
translation as the default visible trigger, and uses the same translated
semantic text for close-button labels and reorder announcements. Use
`Tab::new_static(key, label_text, panel)` when the label is static text that
does not need i18n, `Tab::new_with_label(key, label_text, trigger, panel)`
when the visible trigger is a custom element, and `.trigger(element)` to
replace only the visible trigger while preserving the translated semantic
label.

The adapter owns ordered tab registration, selected-tab indicator measurement, panel presence policy, closable-tab trigger semantics, and reorder announcements.

## 3. Mapping to Core Component Contract

- Props parity: full parity with selected tab, activation mode, orientation, direction, disabled keys, and lazy panel behavior.
- State parity: full parity with selected tab, focused tab, and focus-visible state.
- Anatomy parity: the adapter renders the full anatomy (`Root`, `List`,
  repeated `Tab`, `Panel`, `Indicator`, plus closable variant
  `CloseTrigger`) internally. Per-row content is passed through the
  `tabs: TabsSource<K>` prop (`Tab` carries label / panel /
  link / closable / disabled flags); consumers do not nest individual
  `<Tab>` / `<TabsPanel>` components.
- Adapter additions: explicit ordered registration, measurement helpers, and live-region ownership for reorder announcements.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                                       | Ownership     | Attr source                                 | Notes                                  |
| --------------------- | ----------- | ------------------------------------------------------------------------------ | ------------- | ------------------------------------------- | -------------------------------------- |
| `Root`                | required    | `<div>`                                                                        | adapter-owned | `api.root_attrs()`                          | owns context and panel presence policy |
| `List`                | required    | `<div>`                                                                        | adapter-owned | `api.list_attrs()`                          | `role="tablist"`                       |
| `Tab`                 | repeated    | `<a>` for link tabs; otherwise a role-backed focus target such as `<div>`      | adapter-owned | `api.tab_attrs(key, panel_id)`              | roving focus target                    |
| `Indicator`           | optional    | `<span>`                                                                       | adapter-owned | `api.tab_indicator_attrs()`                 | measurement-driven visual part         |
| `Panel`               | repeated    | `<div>`                                                                        | adapter-owned | `api.panel_attrs(key, tab_id)`              | `role="tabpanel"`                      |
| `CloseTrigger`        | conditional | non-tabbable nested element; avoid a nested native control inside the tab root | adapter-owned | adapter-owned attrs plus core variant rules | closable tabs only                     |
| live region           | conditional | hidden `<div>`                                                                 | adapter-owned | adapter-owned attrs                         | reorder announcement surface           |

## 5. Attr Merge and Ownership Rules

| Target node      | Core attrs                                                       | Adapter-owned attrs                                                        | Consumer attrs                         | Merge order                             | Ownership notes                         |
| ---------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------- | --------------------------------------- | --------------------------------------- |
| `List` and `Tab` | tablist roles, selection, controls, tabindex, and disabled attrs | roving keydown handlers, measurement hooks, and route-aware host selection | decoration attrs and trailing handlers | ARIA, tabindex, and selection attrs win | tabs remain adapter-owned focus targets |
| `Panel`          | labelledby, hidden, and current selection attrs                  | lazy-mount or unmount presence policy                                      | decoration attrs                       | linkage and visibility attrs win        | panel ownership stays adapter-side      |
| `Indicator`      | decorative attrs                                                 | measurement-derived CSS custom properties                                  | none                                   | adapter measurement wins                | visual only                             |

## 6. Composition / Context Contract

`Tabs` is a single compound component that owns the machine internally
and renders the full anatomy in one pass. There is no consumer-facing
root context to publish: per-tab data flows in through the
`tabs: TabsSource<K>` prop, and the agnostic core's `ConnectApi` is
reached via the adapter's render path. A future
compound API (separate `<TabsRoot>`, `<TabsList>`, `<Tab>`,
`<TabsPanel>` parts) could be layered on top without breaking this
single-component surface.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                           | Mode       | Sync trigger            | Machine event / update path          | Visible effect                                    | Notes                                            |
| ---------------------------------------------------------------------- | ---------- | ----------------------- | ------------------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| `value`                                                                | controlled | prop change after mount | `SelectTab` or controlled sync event | updates selected tab, indicator, and active panel | no controlled/uncontrolled switching after mount |
| `orientation`, `activation_mode`, `dir`, `loop_focus`, `disabled_keys` | controlled | rerender with new props | core prop rebuild                    | updates roving navigation and activation guards   | registry identity remains stable                 |
| `lazy_mount`, `unmount_on_exit`                                        | controlled | rerender with new props | adapter presence policy              | changes panel mount lifecycle                     | not machine state                                |
| `reorderable`                                                          | controlled | rerender with new props | adapter behavior gate                | enables reorder announcements and controls        | no hidden reorder mode                           |

| UI event                 | Preconditions           | Machine event / callback path                                                                | Ordering notes                                                            | Notes                                        |
| ------------------------ | ----------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------- |
| tab click or Enter/Space | target tab not disabled | `SelectTab(key)`                                                                             | selection commits before trailing callbacks                               | manual mode still allows explicit activation |
| roving arrow navigation  | enabled tabs exist      | `FocusNext`, `FocusPrev`, `FocusFirst`, `FocusLast`                                          | automatic mode may also commit selection                                  | RTL swaps only horizontal sibling navigation |
| tab focus                | tab receives focus      | `Focus { tab, is_keyboard }`                                                                 | focus-visible state settles first                                         | selected tab may differ in manual mode       |
| close trigger activation | tab is closable         | `CloseTab(key)`, `on_close_tab(key)`, and owned-store removal when using inline tabs         | callback fires after the core notification                                | trigger is not part of roving order          |
| reorder action           | `reorderable=true`      | `on_reorder(ReorderEvent<K>)`, owned-store reorder when using inline tabs, then `ReorderTab` | veto suppresses owned-store mutation, core notification, and announcement | adapter owns announcer                       |

## 8. Registration and Cleanup Contract

- The adapter normalizes `TabsSource<K>` to a reactive
  `ReadStore<Vec<Tab<K>>>`. Inline arrays and vectors create an
  adapter-owned store; consumer read stores remain consumer-owned.
  Every Dioxus render recomputes a `(key, closable)` fingerprint from
  the live store; when the fingerprint differs from the previous
  dispatch the adapter re-emits `Event::SetTabs` to the core machine.
- Per-row `disabled` toggles are aggregated into `Props::disabled_keys`
  on each render. `use_machine`'s built-in `use_sync_props` plumbing
  detects the prop diff and dispatches `Event::SyncProps`.
- Initial registration fires on the first render so SSR markup
  reflects the registered tab list. Subsequent store mutations
  re-dispatch `SetTabs` from inside the same component instance.
- Cleanup removes tabs by key when they leave the store; the
  store-driven iteration retires the corresponding DOM nodes without
  remounting sibling rows or the parent `Tabs` component.
- Reorder announcements: each commit overwrites the live-region text;
  rapid successive reorders rely on `aria-live="polite"` semantics to
  coalesce at the assistive-technology layer (the adapter does not
  debounce or actively cancel queued utterances).

## 9. Ref and Node Contract

| Target part / node      | Ref required? | Ref owner                                             | Node availability                     | Composition rule                                          | Notes                                              |
| ----------------------- | ------------- | ----------------------------------------------------- | ------------------------------------- | --------------------------------------------------------- | -------------------------------------------------- |
| each `Tab`              | yes           | adapter-owned `MountedData` registry keyed by tab key | required after mount                  | compose adapter node handle with any exposed consumer ref | roving focus and measurement depend on live nodes  |
| `List`                  | yes           | adapter-owned                                         | required after mount                  | no composition                                            | indicator measurements are relative to list bounds |
| hidden live-region root | conditional   | adapter-owned                                         | required after mount when reorderable | no composition                                            | announcer only when reorder support exists         |

Keyboard roving focus must prefer
`DioxusPlatform::focus_mounted_element` with the `MountedData` handle
captured from each tab's `onmounted` event. ID-based DOM focus is only
a web fallback for the rare case where keyboard intent arrives before a
mounted handle is available. The `ars-dioxus/web` feature must enable
Dioxus's `mounted` feature so Dioxus Web emits `onmounted` events.

## 10. State Machine Boundary Rules

- Selected tab, focused tab, disabled tab guards, and activation mode remain core-owned.
- Ordered registration, indicator measurement, panel presence policy, and reorder announcements remain adapter-owned.
- The adapter must not keep an unsynchronized selected-tab mirror.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape                                   | Timing                       | Cancelable? | Notes                                                                                             |
| --------------------- | ------------------------ | ----------------------------------------------- | ---------------------------- | ----------- | ------------------------------------------------------------------------------------------------- |
| value-change callback | machine-derived snapshot | `Option<K>`                                     | after committed selection    | no          | only fires when the selected key changes                                                          |
| close callback        | adapter-derived          | `Key`                                           | after `CloseTab` dispatch    | no          | consumer-owned stores remove through callback; inline tabs remove through the adapter-owned store |
| reorder callback      | adapter-derived          | `ReorderEvent<K> { key, old_index, new_index }` | before `ReorderTab` dispatch | yes         | returning `false` vetoes the notification and announcement                                        |

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
- The normalized `ReadStore<Vec<Tab<K>>>` is read on each render; SSR markup
  reflects the store's value at render time. The fingerprint diff
  fires the initial `SetTabs` dispatch synchronously during the first
  render so the SSR HTML pass already sees the registered list.
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
| Dioxus mounted element APIs | required                  | renderer API        | roving focus should use renderer-backed `MountedData` handles | requires `dioxus/mounted` on web           |

## 17. Recommended Implementation Sequence

1. Resolve `ModalityContext`, `PlatformEffects`, and `DioxusPlatform`
   from `ArsContext` so DOM measurement, renderer-backed focus, and
   `data-ars-focus-visible` go through provider-supplied handles.
2. Build per-render `core_props` from the tabs store (so per-row
   `disabled` toggles fire `SyncProps`) and feed it to `use_machine`;
   the built-in `use_sync_props` plumbing diffs across renders.
3. Dispatch the initial `Event::SetTabs` synchronously, then track
   the `(key, closable)` fingerprint in a `use_signal` and re-dispatch
   when it changes.
4. Render tablist, tabs, indicator, and panels in one pass; per-tab
   content comes from `Tab`.
5. Capture each tab trigger's `MountedData` in an adapter-owned
   registry keyed by tab key. Watch the `focused_tab` `derive` memo via
   `use_effect` and call `DioxusPlatform::focus_mounted_element` when it
   changes, falling back to ID focus only when the mounted handle is not
   available yet.
6. Add closable tabs (close button + Delete/Backspace), reorder
   behavior (Ctrl+Arrow + drag/drop), and live announcements via
   `Api::reorder_announcement`.

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

| Helper concept          | Required?                 | Responsibility                                                              | Reused by           | Notes                                                    |
| ----------------------- | ------------------------- | --------------------------------------------------------------------------- | ------------------- | -------------------------------------------------------- |
| `TabMeta<K>` snapshot   | required                  | captures typed key, internal key, label, closable, disabled row state       | Leptos Tabs adapter | DOM-free semantic row data                               |
| registration helper     | required                  | maps `TabMeta<K>` into `TabRegistration`                                    | Leptos Tabs adapter | roving source of truth                                   |
| disabled-key helper     | required                  | maps disabled `TabMeta<K>` rows into core `Key` set                         | Leptos Tabs adapter | merges per-row and prop disabled state before core props |
| reorder planning helper | required when reorderable | validates source/target and builds `ReorderEvent<K>` plus announcement data | Leptos Tabs adapter | skips disabled endpoints                                 |
| measurement helper      | recommended               | computes indicator CSS custom properties                                    | `navigation-menu`   | relative to list bounds                                  |
| live-region helper      | required when reorderable | announces committed reorder actions                                         | `pagination`        | keeps initial render silent                              |

## 23. Framework-Specific Behavior

Dioxus should sync the controlled selected key through props, compose each tab node handle with any wrapper ref, and derive indicator styles reactively from the selected tab and list measurements.

## 24. Canonical Implementation Sketch

```rust,no_check
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en")]
enum SettingsTab {
    #[translate(en = "Home")]
    Home,

    #[translate(en = "Settings")]
    Settings,
}

fn app() -> Element {
    rsx! {
        Tabs {
            default_value: SettingsTab::Home,
            tabs: [
                Tab::new(SettingsTab::Home, rsx! { HomePanel {} }),
                Tab::new(SettingsTab::Settings, rsx! { SettingsPanel {} }).closable(true),
            ],
            reorderable: true,
        }
    }
}
```

Inside the adapter, `use_machine` runs `use_sync_props` per render,
so the per-render-rebuilt `core_props` (with the live aggregated
`disabled_keys`) drives `Event::SyncProps` automatically. The shared
`TabMeta<K>` snapshot drives `Event::SetTabs` through
`registrations_from_meta`; drag-and-drop reorder handlers use
`drag_reorder_plan` to build the typed `ReorderEvent<K>` and
announcement data. Owned inline tabs mutate their internal store for
close and reorder interactions. Controlled `value` flows through
`Event::SyncControlledValue`.
`Effect::FocusFocusedTab` is dispatched by watching `ctx.focused_tab`
and calling `DioxusPlatform::focus_mounted_element` with the tab's
registered `MountedData` handle.

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
- runtime mutation: pop a tab from the store, verify the rendered tab
  count drops and `Event::SetTabs` re-dispatches without remounting.
- runtime mutation: push a new tab into the store, verify it joins the
  registered list and selection invariant survives.
- runtime mutation: flip a tab's `closable` flag in the store, verify
  the close button appears / disappears without remounting siblings.
- runtime mutation: flip a tab's `disabled` flag in the store, verify
  `aria-disabled` updates and the tab is skipped during arrow navigation
  via `Event::SyncProps`.
- link tabs: a tab with `Tab::link(href)` renders as `<a>` with the
  expected `href`, retains `role="tab"`, and `event.preventDefault()`
  inhibits navigation while `Event::SelectTab` still fires.
- modality: focus-visible attribute is emitted only when the most
  recent input was a keyboard interaction.
- reorder i18n: `Messages::reorder_announce_label` overrides the
  default English template (verified via a custom `Messages` instance).
- controlled value None: a controlled `Option<Option<K>>` set to
  `Some(None)` clears the selection without flipping out of controlled
  mode.

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
