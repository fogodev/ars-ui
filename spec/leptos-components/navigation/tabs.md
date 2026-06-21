---
adapter: leptos
component: tabs
category: navigation
source: components/navigation/tabs.md
source_foundation: foundation/08-adapter-leptos.md
---

# Tabs — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Tabs`](../../components/navigation/tabs.md) contract onto Leptos 0.8.x. The adapter preserves compound tablist composition, roving focus, selection sync, indicator measurement, lazy panel presence, closable-tab support, and reorder announcements.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Root<K: TabKey>(
    #[prop(optional, into)] value: Option<Signal<Option<K>>>,
    #[prop(into)]
    default_value: K,
    #[prop(into)] tabs: tabs::TabsSource<K>,
    #[prop(optional, into)] orientation: Option<Signal<Orientation>>,
    #[prop(optional, into)] activation_mode: Option<Signal<tabs::ActivationMode>>,
    #[prop(optional, into)] dir: Option<Signal<Direction>>,
    #[prop(optional, into)] loop_focus: Option<Signal<bool>>,
    #[prop(optional, into)] disallow_empty_selection: Option<Signal<bool>>,
    #[prop(optional, into)] lazy_mount: Option<Signal<bool>>,
    #[prop(optional, into)] unmount_on_exit: Option<Signal<bool>>,
    #[prop(optional, into)] disabled_keys: Option<Signal<BTreeSet<K>>>,
    #[prop(optional, into)] reorderable: Option<Signal<bool>>,
    #[prop(optional)] on_value_change: Option<Callback<Option<K>>>,
    #[prop(optional)] on_close_tab: Option<Callback<K>>,
    #[prop(optional)] on_reorder: Option<Callback<tabs::ReorderEvent<K>, bool>>,
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView
where
    K: TabKey,

#[component]
pub fn List<K: TabKey>(
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
    #[prop(optional, into)] tab_row: Option<TabRenderer<K>>,
) -> impl IntoView

#[component]
pub fn Panels<K: TabKey>(
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
    #[prop(optional, into)] panel: Option<TabPanelRenderer<K>>,
) -> impl IntoView

#[derive(Clone, Debug)]
pub struct TabRenderItem<K: TabKey> {
    pub tab: Tab<K>,
}

#[derive(Clone)]
pub struct TabRenderer<K: TabKey>(/* private */);

#[derive(Clone)]
pub struct TabPanelRenderer<K: TabKey>(/* private */);

#[component]
pub fn TabShell<K: TabKey>(
    item: TabRenderItem<K>,
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView

#[component]
pub fn Trigger<K: TabKey>(
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
) -> impl IntoView

#[component]
pub fn CloseTrigger<K: TabKey>(
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
) -> impl IntoView

#[component]
pub fn Panel<K: TabKey>(
    item: TabRenderItem<K>,
    #[prop(optional, into)] class: Option<TextProp>,
    #[prop(optional, into)] style: Option<TextProp>,
) -> impl IntoView

#[component]
pub fn LiveRegion() -> impl IntoView
```

`K` is a typed public tab identifier that implements `TabKey`
(`Copy + Eq + Ord + Send + Sync + 'static`). The adapter converts
`K` into the agnostic core's internal `Key` at the machine boundary,
then maps committed selection, close, and reorder callbacks back to
`K`. This keeps `Key` an implementation detail and prevents mixing
unrelated application enum families in a single tablist.

`value` is an optional controlled `Signal<Option<K>>`. The outer
`Option` distinguishes controlled-vs-uncontrolled mode (fixed at
mount per agnostic-core spec §1.5); the inner `Option<K>` carries
the live selection so a controlled consumer can express
"no tab selected" without flipping out of controlled mode. Inner-
value changes flow through `Event::SyncControlledValue` automatically.

`tabs` accepts inline rows (`Vec<Tab<K>>` or `[Tab<K>; N]`) for the
common static-list case, or a reactive `Field<Vec<Tab<K>>>` from an
`ars_leptos::reactive_stores::Store<…>` when the consumer owns a
dynamic list. Inline rows are copied into an adapter-owned store at
mount, so close and reorder interactions still mutate rendered tab
order without requiring consumer store setup. Mutations of either
store source (push, remove, reorder, swap of a tab's `closable` or
`disabled` flag) re-dispatch `Event::SetTabs` / `Event::SyncProps`
automatically without remounting the `Tabs` component.

`Root` is the machine and collection owner. It does not render a
closed Tabs anatomy by itself; consumers compose `List`, `Panels`, and
`LiveRegion` as children. `List` and `Panels` iterate the single
`tabs: TabsSource<K>` collection supplied to `Root`, so consumers do
not manually repeat or stringify tab keys.

`Root::children` intentionally uses optional `Children`, not
`TypedChildren<T>`. The root slot is a heterogeneous composition area for
public parts and local decoration after context publication; it does not have a
single typed child-root contract, does not forward attrs through `add_attr`, and
does not consume child values to establish tab identity. Tabs type-safety is
instead provided by `TabsSource<K>`, `TabRenderItem<K>`, and the typed
renderer callbacks on `List<K>` / `Panels<K>`. Keeping `children` optional also permits tests and
partial compositions to render the root/context wrapper without forcing a
placeholder child.

`List` renders one public `TabShell` for each row and an internal
indicator node. `Panels` renders one public `Panel` for each row. The typed
`TabRenderItem<K>` handed to `TabShell` / `Panel` carries the row data
from the root collection order. For custom anatomy, `List<K>::tab_row`
and `Panels<K>::panel` accept typed row renderers that receive
`TabRenderItem<K>` values from the same `TabsSource<K>`. Consumers still do
not duplicate key order; they only arrange public adapter parts for each row.
`TabShell` publishes that row to descendant `Trigger<K>` and
`CloseTrigger<K>` parts through typed context, so those child parts do not
accept an `item` prop. Consumers spell the key type explicitly when Rust
cannot infer it from props.
Default `List` / `Panels` calls must spell the key type, for example
`<tabs::List<SettingsTab> />`, because the renderer props live on generic
collection parts instead of on `Root`.

The public trigger part is named `Trigger` rather than `Tab` because
`tabs::Tab<K>` remains the row-data constructor. `Trigger` and
`CloseTrigger` own their adapter event policy, refs, ARIA, disabled guards,
and localized labels; custom renderers compose them instead of rebuilding
that behavior. `Tab::close_trigger` replaces only the visible glyph/content
inside the adapter-owned close affordance.

`Tab::new(key, panel)` is the preferred constructor for static application
tab lists. It requires `K: TabKey + Translate`, uses the key enum's
translation as the default visible trigger, and uses the same translated
semantic text for close-affordance labels and reorder announcements. Use
`Tab::new_static(key, label_text, panel)` when the label is static text that
does not need i18n, `Tab::new_with_label(key, label_text, trigger, panel)`
when the visible trigger is a custom view, and `.trigger(view)` to replace
only the visible trigger while preserving the translated semantic label.
Use `.close_trigger(view)` to replace the fallback close glyph while
leaving close semantics and accessible labeling adapter-owned.

The adapter owns ordered tab registration, selected-tab indicator measurement, renderer application of the core panel-presence predicate, closable-tab trigger semantics, and reorder announcements.

Tab labels carry semantic text separately from custom trigger views:

```rust,no_check
#[derive(Clone)]
pub enum TabLabel {
    Static(Oco<'static, str>),
    Dynamic(TextProp),
}

impl TabLabel {
    pub fn static_text(text: impl Into<Oco<'static, str>>) -> Self;
    pub fn translated<T>(message: T) -> Self
    where
        T: Translate + Send + Sync + 'static;
    pub fn dynamic(text: impl Into<TextProp>) -> Self;
    pub fn resolve(&self) -> String;
    pub fn debug_label(&self) -> String;
}
```

`Static` is for non-localized literals. `Dynamic` is the required shape for
provider-backed i18n or any other reactive semantic label. The adapter uses the
resolved label for default visible triggers, close-affordance labels, and
reorder announcements; a custom trigger view must not be the only source of
semantic tab text.

End-user prelude imports expose the `tabs` module only. Consumers must spell
primitive parts and row-data helpers through that namespace, for example
`tabs::Root`, `tabs::List`, `tabs::Tab`, and `tabs::TabsSource`; adapter
preludes do not flatten component parts into the application namespace. The
closed ready-made `Tabs` component lives only in
`ars-leptos-components::navigation::tabs::{css,tailwind}`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with selected tab, activation mode, orientation, direction, disabled keys, and lazy panel behavior.
- State parity: full parity with selected tab, focused tab, and focus-visible state.
- Anatomy parity: the adapter exposes unstyled primitive parts
  (`Root`, `List`, `Panels`, `TabShell`, `Trigger`, `CloseTrigger`, `Panel`,
  `LiveRegion`) while keeping focus, selection, reorder, indicator
  measurement, and ARIA policy adapter-owned.
- Adapter additions: explicit ordered registration, measurement helpers, and live-region ownership for reorder announcements.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                                      | Ownership     | Attr source                               | Notes                                                                                                                                                                |
| --------------------- | ----------- | ----------------------------------------------------------------------------- | ------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`                | required    | `<div>`                                                                       | adapter-owned | `api.root_attrs()`                        | owns context and applies panel body presence                                                                                                                         |
| `List`                | required    | `<div>`                                                                       | adapter-owned | `api.list_attrs()`                        | `role="tablist"`                                                                                                                                                     |
| `Panels`              | required    | `<div>`                                                                       | adapter-owned | `api.panels_attrs()`                      | panel collection wrapper                                                                                                                                             |
| `TabShell`            | repeated    | presentational `<div>` wrapping one tab trigger and optional close trigger    | adapter-owned | `api.tab_shell_attrs(key, focus_visible)` | mirrors selected, disabled, closable, and focus-visible state so the whole row can be styled directly; owns browser `draggable` and pointer drag/drop handlers       |
| `Trigger`             | repeated    | `<a>` for link tabs; otherwise a role-backed focus target such as `<div>`     | adapter-owned | `api.tab_attrs(key, focus_visible)`       | public trigger part; roving focus target; owns tab role, selection, focus, and keyboard handlers                                                                     |
| `Indicator`           | optional    | internal `<span>` rendered by `List`                                          | adapter-owned | `api.tab_indicator_attrs()`               | measurement-driven visual node; styled through stable data attrs or higher-level styled templates instead of a standalone public component                           |
| `Panel`               | repeated    | `<div>`                                                                       | adapter-owned | `api.panel_attrs(key, tab_id)`            | `role="tabpanel"`                                                                                                                                                    |
| `CloseTrigger`        | conditional | non-roving pointer affordance sibling after the tab trigger inside `TabShell` | adapter-owned | `api.close_trigger_attrs(label)`          | closable tabs only; renders `Tab::close_trigger` content when supplied, otherwise a small SVG glyph so unstyled primitives remain visible without CSS pseudo-content |
| live region           | conditional | hidden `<div>`                                                                | adapter-owned | adapter-owned attrs                       | reorder announcement surface                                                                                                                                         |

## 4.1 Customization Boundary

The adapter crate exposes unstyled primitives only. Ready-made visual
Tabs live in `ars-leptos-components/src/navigation/tabs/` as
category-first CSS and Tailwind source-template modules. Those styled
templates compose the adapter primitives and may expose the ergonomic
closed-anatomy props, but they are not adapter primitives.

`Trigger` and `CloseTrigger` are public child parts, but they are behavior
parts rather than raw DOM shortcuts. They own roving focus, ARIA
relationships, DOM refs, keyboard dispatch, close semantics, disabled guards,
and localized labels. `TabShell` owns browser `draggable`, drop target, and
drag-image wiring so the whole row, including the close affordance area, is
the drag surface. Consumers customize row layout with `List<K>::tab_row` and
`TabShell`, customize close glyph content with `Tab::close_trigger(view)`, and
style each node directly:

```rust,no_check
view! {
    <tabs::Root
        default_value=SettingsTab::Home
        tabs=[
            tabs::Tab::new(SettingsTab::Home, home_panel),
            tabs::Tab::new(SettingsTab::Settings, settings_panel)
                .closable(true)
                .close_trigger(|| view! { <span aria-hidden="true">"x"</span> }),
        ]
    >
        <tabs::List<SettingsTab>
            class="relative"
            tab_row=|item| view! {
                <tabs::TabShell item=item class="inline-flex items-center gap-1">
                    <tabs::Trigger<SettingsTab> class="px-3 py-2" />
                    <tabs::CloseTrigger<SettingsTab> class="grid size-5 place-items-center rounded-full hover:bg-red-100" />
                </tabs::TabShell>
            }
        />
        <tabs::Panels<SettingsTab> />
        <tabs::LiveRegion />
    </tabs::Root>
}
```

Tailwind users who want a ready-made closed anatomy should use the
`ars-leptos-components::navigation::tabs::tailwind` source template and edit
its inline classes. Low-level adapter users should attach Tailwind classes to
public parts such as `List`, `Root`, `TabShell`, `Trigger`, and
`CloseTrigger`; they should not
rebuild the close trigger, trigger, indicator, selection, or keyboard policy in
application code.

## 5. Attr Merge and Ownership Rules

| Target node          | Core attrs                                                       | Adapter-owned attrs                                                        | Consumer attrs                         | Merge order                             | Ownership notes                         |
| -------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------- | --------------------------------------- | --------------------------------------- |
| `List` and `Trigger` | tablist roles, selection, controls, tabindex, and disabled attrs | roving keydown handlers, measurement hooks, and route-aware host selection | decoration attrs and trailing handlers | ARIA, tabindex, and selection attrs win | tabs remain adapter-owned focus targets |
| `Panels`             | panel collection anatomy attrs                                   | none                                                                       | decoration attrs                       | core attrs win                          | wrapper is a public core part           |
| `Panel`              | labelledby, hidden, and current selection attrs                  | renderer application of the core lazy/unmount predicate                    | decoration attrs                       | linkage and visibility attrs win        | panel ownership stays adapter-side      |
| `Indicator`          | decorative attrs                                                 | measurement-derived CSS custom properties                                  | none                                   | adapter measurement wins                | internal visual node                    |

## 6. Composition / Context Contract

`Root` publishes typed adapter context to child primitives. `List` and
`Panels` are collection-driven readers of the root `TabsSource<K>`;
`TabShell`, `Trigger`, `CloseTrigger`, and `Panel` receive a `TabRenderItem<K>` from those
collection renderers. Consumers do not manually mirror key order in
children. Public parts fail fast when rendered outside `Root`. The
indicator node is private because calling it correctly requires
adapter-owned measurement attrs and style state. CSS variants customize it
through `data-ars-part="tab-indicator"` selectors; Tailwind variants put the
equivalent arbitrary descendant variants on the `List` class string so copied
source remains self-contained.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                           | Mode       | Sync trigger              | Machine event / update path                | Visible effect                                    | Notes                                            |
| ---------------------------------------------------------------------- | ---------- | ------------------------- | ------------------------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| `value`                                                                | controlled | signal change after mount | `SelectTab` or controlled sync event       | updates selected tab, indicator, and active panel | no controlled/uncontrolled switching after mount |
| `orientation`, `activation_mode`, `dir`, `loop_focus`, `disabled_keys` | controlled | rerender with new props   | core prop rebuild                          | updates roving navigation and activation guards   | registry identity remains stable                 |
| `lazy_mount`, `unmount_on_exit`                                        | controlled | rerender with new props   | core presence predicate applied by adapter | changes panel mount lifecycle                     | not machine state                                |
| `reorderable`                                                          | controlled | rerender with new props   | adapter behavior gate                      | enables reorder announcements and controls        | no hidden reorder mode                           |

| UI event                 | Preconditions           | Machine event / callback path                                                                | Ordering notes                                                            | Notes                                        |
| ------------------------ | ----------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------- |
| tab click or Enter/Space | target tab not disabled | `SelectTab(key)`                                                                             | selection commits before trailing callbacks                               | manual mode still allows explicit activation |
| roving arrow navigation  | enabled tabs exist      | `FocusNext`, `FocusPrev`, `FocusFirst`, `FocusLast`                                          | automatic mode may also commit selection                                  | RTL swaps only horizontal sibling navigation |
| tab focus                | tab receives focus      | `Focus { tab, is_keyboard }`                                                                 | focus-visible state settles first                                         | selected tab may differ in manual mode       |
| close trigger activation | tab is closable         | `CloseTab(key)`, `on_close_tab(key)`, and owned-store removal when using inline tabs         | callback fires after the core notification                                | trigger is not part of roving order          |
| reorder action           | `reorderable=true`      | `on_reorder(ReorderEvent<K>)`, owned-store reorder when using inline tabs, then `ReorderTab` | veto suppresses owned-store mutation, core notification, and announcement | adapter owns announcer                       |

## 8. Registration and Cleanup Contract

- The adapter normalizes `TabsSource<K>` to a reactive `Field<Vec<Tab<K>>>`.
  Inline arrays and vectors create an adapter-owned store; consumer
  fields remain consumer-owned. Every change to the active field (push,
  remove, reorder, swap of a tab's `closable` flag) is normalized into
  a `TabMeta<K>` snapshot. The shared `registrations_from_meta` helper
  produces the registration memo committed to the core machine through
  `Event::SetTabs`.
- Per-row `disabled` toggles flow through `Props::disabled_keys` via the
  reactive `props_signal`, which aggregates directly from the live
  `Field<Vec<Tab<K>>>` and fires `Event::SyncProps` through
  `use_machine_with_reactive_props`. Initial machine construction reads
  this signal untracked; the follow-up effect is the tracked subscription.
- Initial registration is dispatched synchronously during component
  setup so SSR markup reflects the registered tab list. Subsequent
  store mutations re-dispatch `SetTabs` through `Effect::watch`
  (client-only).
- Cleanup removes tabs by key when they leave the field; the keyed
  `<For>` rendering retires the corresponding DOM nodes without
  remounting sibling rows or the parent `Tabs` component.
- Reorder announcements: each commit overwrites the live-region text;
  rapid successive reorders rely on `aria-live="polite"` semantics to
  coalesce at the assistive-technology layer (the adapter does not
  debounce or actively cancel queued utterances).

## 9. Ref and Node Contract

| Target part / node      | Ref required? | Ref owner                                        | Node availability                     | Composition rule                                  | Notes                                              |
| ----------------------- | ------------- | ------------------------------------------------ | ------------------------------------- | ------------------------------------------------- | -------------------------------------------------- |
| each `Tab`              | yes           | shared between adapter and optional wrapper refs | required after mount                  | compose adapter ref with any exposed consumer ref | roving focus and measurement depend on live nodes  |
| `List`                  | yes           | adapter-owned                                    | required after mount                  | no composition                                    | indicator measurements are relative to list bounds |
| hidden live-region root | conditional   | adapter-owned                                    | required after mount when reorderable | no composition                                    | announcer only when reorder support exists         |

## 10. State Machine Boundary Rules

- Selected tab, focused tab, disabled tab guards, and activation mode remain core-owned.
- Ordered registration, indicator measurement, renderer application of the core panel-presence predicate, and reorder announcements remain adapter-owned.
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
- The normalized `Field<Vec<Tab<K>>>` is read once during the SSR pass via the
  synchronous initial-`SetTabs` dispatch; reactive subscriptions
  (`Effect::watch`) only attach on the client. SSR markup reflects the
  store's value at render time.
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

1. Resolve `ModalityContext` and `PlatformEffects` from `ArsContext`
   so DOM focus dispatch and `data-ars-focus-visible` go through the
   provider-supplied handles.
2. Build a reactive `Memo<Props>` from the tabs field (so per-row
   `disabled` toggles fire `SyncProps`) and feed it to
   `use_machine_with_reactive_props`.
3. Dispatch the initial `Event::SetTabs` synchronously, then watch
   the `(key, closable)` fingerprint memo for store mutations.
4. Render tablist, tabs, indicator, and panels in one pass via a
   keyed `<For>`; per-tab content comes from `Tab`.
5. Watch the `focused_tab` `Memo` and call
   `PlatformEffects::focus_element_by_id` when it changes.
6. Add closable tabs (close affordance + Delete/Backspace), reorder
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
- Consumers may import the `tabs` module from `ars_leptos::prelude::*`, then
  access the full adapter and agnostic Tabs surface through namespaced paths
  such as `tabs::Root`, `tabs::TabRenderItem`, and `tabs::TabsSource`.

## 20. Platform Support Matrix

| Capability / behavior                                     | Browser client | SSR          | Notes                        |
| --------------------------------------------------------- | -------------- | ------------ | ---------------------------- |
| tablist semantics, roving focus, and active panel linkage | full support   | full support | baseline tabs behavior       |
| indicator measurement                                     | full support   | client-only  | visual enhancement only      |
| reorder announcement                                      | full support   | client-only  | only when `reorderable=true` |

## 21. Debug Diagnostics and Production Policy

| Condition                                     | Debug build behavior | Production behavior | Notes                        |
| --------------------------------------------- | -------------------- | ------------------- | ---------------------------- |
| missing root context for tab or panel surface | fail fast            | fail fast           | compound structure violation |
| selected key missing from registry            | debug warning        | warn and ignore     | fallback selection path      |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required?                 | Responsibility                                                              | Reused by           | Notes                       |
| ----------------------- | ------------------------- | --------------------------------------------------------------------------- | ------------------- | --------------------------- |
| `TabMeta<K>` snapshot   | required                  | captures typed key, internal key, label, closable, disabled row state       | Dioxus Tabs adapter | DOM-free semantic row data  |
| registration helper     | required                  | maps `TabMeta<K>` into `TabRegistration`                                    | Dioxus Tabs adapter | roving source of truth      |
| reorder planning helper | required when reorderable | validates source/target and builds `ReorderEvent<K>` plus announcement data | Dioxus Tabs adapter | skips disabled endpoints    |
| measurement helper      | recommended               | computes indicator CSS custom properties                                    | `navigation-menu`   | relative to list bounds     |
| live-region helper      | required when reorderable | announces committed reorder actions                                         | `pagination`        | keeps initial render silent |

## 23. Framework-Specific Behavior

Leptos should watch the controlled selected key after mount, compose each tab `NodeRef` with any wrapper ref, and derive indicator styles reactively from the selected tab and list measurements.

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

view! {
    <tabs::Root
        default_value=SettingsTab::Home
        tabs=[
            tabs::Tab::new(SettingsTab::Home, home_panel),
            tabs::Tab::new(SettingsTab::Settings, settings_panel)
                .closable(true),
        ]
        reorderable=true
    >
        <tabs::List<SettingsTab> />
        <tabs::Panels<SettingsTab> />
        <tabs::LiveRegion />
    </tabs::Root>
}
```

Inside the adapter, `use_machine_with_reactive_props` consumes a
reactive `Signal<Props>` derived from the normalized field. The
shared `TabMeta<K>` snapshot drives `Event::SetTabs` through
`registrations_from_meta`; direct disabled aggregation from the live
field drives `Event::SyncProps`; drag-and-drop reorder handlers use
`drag_reorder_plan` to build the typed `ReorderEvent<K>` and
announcement data and clone the public `TabShell` as the native drag
image; owned inline tabs mutate their internal store for close and
reorder interactions; controlled `value` drives
`Event::SyncControlledValue`.
`Effect::FocusFocusedTab` is dispatched by watching `ctx.focused_tab`
and calling `PlatformEffects::focus_element_by_id`.

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
- Promotes ordered registration, indicator measurement, panel presence, close-trigger behavior, and reorder announcements into explicit Leptos-facing rules.

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
  the close affordance appears / disappears without remounting siblings.
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
- controlled value None: a controlled `Signal<Option<K>>` set to
  `None` clears the selection without flipping out of controlled mode.

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
