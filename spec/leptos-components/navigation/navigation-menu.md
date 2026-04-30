---
adapter: leptos
component: navigation-menu
category: navigation
source: components/navigation/navigation-menu.md
source_foundation: foundation/08-adapter-leptos.md
---

# NavigationMenu — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`NavigationMenu`](../../components/navigation/navigation-menu.md) contract onto Leptos 0.8.x. The adapter preserves trigger and content registration, hover-delay timers, skip-delay logic, keyboard focus restoration, indicator measurement, viewport sizing, and viewport or portal ownership for active content.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn NavigationMenu(
    #[prop(optional, into)] value: Option<Signal<Option<Key>>>,
    #[prop(optional)] default_value: Option<Key>,
    items: Vec<navigation_menu::ItemDef>,
    #[prop(optional)] delay_ms: u32,
    #[prop(optional)] skip_delay_ms: u32,
    #[prop(optional)] orientation: Orientation,
    #[prop(optional)] dir: Direction,
    #[prop(optional)] loop_focus: bool,
    #[prop(optional)] with_indicator: bool,
    #[prop(optional)] with_viewport: bool,
    children: Children,
) -> impl IntoView
```

The adapter owns timer handles, ordered trigger registration, content handoff into the optional viewport, z-index allocation, and active-trigger indicator measurement.

## 3. Mapping to Core Component Contract

- Props parity: full parity with controlled open item, delays, direction, orientation, and loop-focus behavior.
- State parity: full parity with idle vs open states, focused trigger tracking, and open-delay skip window behavior.
- Part parity: full parity with `Root`, `List`, repeated `Item`, `Trigger`, `Content`, repeated `Link`, optional `Indicator`, and optional `Viewport`.
- Adapter additions: explicit timer lifecycle, viewport handoff, measurement, and z-index integration.

## 4. Part Mapping

| Core part / structure | Required?            | Adapter rendering target | Ownership     | Attr source                              | Notes                                         |
| --------------------- | -------------------- | ------------------------ | ------------- | ---------------------------------------- | --------------------------------------------- |
| `Root`                | required             | `<nav>`                  | adapter-owned | `api.root_attrs()`                       | owns menu context and optional viewport shell |
| `List`                | required             | `<ul>`                   | adapter-owned | `api.list_attrs()`                       | `role="menubar"`                              |
| `Item`                | repeated             | `<li>`                   | adapter-owned | `api.item_attrs(key)`                    | wrapper for trigger plus content              |
| `Trigger`             | repeated             | `<button>`               | adapter-owned | `api.trigger_attrs(key, content_id)`     | registered roving target                      |
| `Content`             | conditional repeated | `<div>`                  | adapter-owned | `api.content_attrs(key)`                 | active content may hand off into viewport     |
| `Link`                | repeated             | `<a>`                    | adapter-owned | adapter-owned attrs plus `Link` contract | direct links inside content                   |
| `Indicator`           | optional             | `<span>`                 | adapter-owned | `api.indicator_attrs()`                  | measurement-driven visual part                |
| `Viewport`            | optional             | `<div>`                  | adapter-owned | `api.viewport_attrs()`                   | active content container with measured size   |

## 5. Attr Merge and Ownership Rules

| Target node              | Core attrs                                           | Adapter-owned attrs                                                                | Consumer attrs                         | Merge order                          | Ownership notes                    |
| ------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------- | ------------------------------------ | ---------------------------------- |
| `Root` and `List`        | navigation landmark and menubar attrs                | context publication and z-index or viewport setup                                  | wrapper decoration only                | required landmark and menu attrs win | root owns global surface           |
| `Trigger`                | expanded, popup, controls, tabindex, and state attrs | pointer-enter/leave timers, focus restoration hooks, measurement hooks             | decoration attrs and trailing handlers | required ARIA and roving attrs win   | trigger remains focus owner        |
| `Content` and `Viewport` | open or closed state, ids, and motion attrs          | viewport handoff, portal ownership, z-index assignment, measured width/height vars | decoration attrs                       | linkage and state attrs win          | active content stays adapter-owned |
| `Indicator`              | decorative attrs                                     | measured position vars                                                             | none                                   | adapter measurement wins             | visual only                        |

## 6. Composition / Context Contract

`NavigationMenu` publishes required root context containing machine access, ordered trigger registration, timer handles, viewport helpers, and measurement helpers. Trigger, content, indicator, and viewport surfaces consume that context and fail fast when rendered outside the root boundary.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                    | Mode       | Sync trigger              | Machine event / update path      | Visible effect                                           | Notes                                            |
| --------------------------------------------------------------- | ---------- | ------------------------- | -------------------------------- | -------------------------------------------------------- | ------------------------------------------------ |
| `value`                                                         | controlled | signal change after mount | `Open(key)` or `Close` sync path | updates active trigger, content, viewport, and indicator | no controlled/uncontrolled switching after mount |
| `delay_ms`, `skip_delay_ms`, `orientation`, `dir`, `loop_focus` | controlled | rerender with new props   | core prop rebuild                | updates timers and focus movement                        | timer handles must read latest values            |
| `with_indicator`, `with_viewport`                               | controlled | rerender with new props   | adapter structural policy        | toggles indicator and viewport parts                     | active content contract stays explicit           |

| UI event                             | Preconditions               | Machine event / callback path                  | Ordering notes                                              | Notes                                           |
| ------------------------------------ | --------------------------- | ---------------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------- |
| trigger pointer enter                | trigger has content         | `PointerEnter(key, now_ms)`                    | adapter starts open-delay or skip-delay path                | contentless direct-link items bypass open logic |
| trigger pointer leave                | open or pending item exists | `PointerLeave`                                 | close timer may be deferred while pointer is in content     | timer ownership is adapter-side                 |
| trigger click or keyboard activation | trigger has content         | `Open(key)` or `Close(now_ms)`                 | keyboard open path is immediate                             | direct links follow native navigation           |
| trigger focus and roving keys        | enabled triggers exist      | focus events and `FocusNext`/`FocusPrev` paths | focus restoration after Escape targets the trigger registry | RTL swaps only horizontal siblings              |
| content pointer enter/leave          | active content exists       | content enter or leave events                  | cancels or starts close delay                               | viewport handoff must preserve these boundaries |
| Escape                               | content open                | `EscapeKey(now_ms)`                            | closes then restores focus to owning trigger                | focus restore is adapter-owned                  |

## 8. Registration and Cleanup Contract

- Each trigger registers in DOM order on mount.
- Active timer handles register on pointer-enter or pointer-leave paths and must be canceled on state changes or unmount.
- Measurement targets for the active trigger, indicator, and viewport must be invalidated on cleanup.

## 9. Ref and Node Contract

| Target part / node    | Ref required?    | Ref owner                                        | Node availability    | Composition rule                               | Notes                                                       |
| --------------------- | ---------------- | ------------------------------------------------ | -------------------- | ---------------------------------------------- | ----------------------------------------------------------- |
| each `Trigger`        | yes              | shared between adapter and optional wrapper refs | required after mount | compose adapter `NodeRef` with any exposed ref | roving focus and indicator measurement depend on live nodes |
| `Viewport`            | yes when enabled | adapter-owned                                    | required after mount | no composition                                 | active content size measurement target                      |
| active `Content` root | yes              | adapter-owned                                    | conditional          | no composition                                 | viewport handoff and dismiss boundaries depend on live node |

## 10. State Machine Boundary Rules

- Active item, focused trigger, delay window logic, and open or closed state remain core-owned.
- Timer registration, measurement, viewport handoff, portal ownership, and z-index assignment remain adapter-owned.
- The adapter must not mirror the active item in a second uncontrolled store.

## 11. Callback Payload Contract

| Callback              | Payload source           | Payload shape          | Timing                                   | Cancelable? | Notes              |
| --------------------- | ------------------------ | ---------------------- | ---------------------------------------- | ----------- | ------------------ |
| value-change callback | machine-derived snapshot | `{ key: Option<Key> }` | after committed open or close transition | no          | wrapper-owned only |

## 12. Failure and Degradation Rules

| Condition                                       | Policy             | Notes                                               |
| ----------------------------------------------- | ------------------ | --------------------------------------------------- |
| viewport enabled but viewport node unavailable  | degrade gracefully | render content inline without size animation        |
| active trigger cannot be measured for indicator | degrade gracefully | hide indicator rather than render stale coordinates |
| timer API unavailable                           | degrade gracefully | use immediate open and close behavior               |

## 13. Identity and Key Policy

Menu item identity is the item key. Trigger registration order, viewport handoff, and active content ownership all key off that identity. Server and client must preserve item order and initial open state.

## 14. SSR and Client Boundary Rules

- SSR renders the landmark, list, triggers, and whichever content or viewport branch the initial state implies.
- Timers, measurement, and portal or viewport handoff are client-only.
- The server and client must preserve the same trigger order and initial active-item branch for hydration safety.

## 15. Performance Constraints

- Register triggers incrementally and cancel timers eagerly.
- Measure only the active trigger and active content instead of all items.
- Reuse one announcer-free timer pipeline rather than per-trigger global timers.

## 16. Implementation Dependencies

| Dependency                  | Required?   | Dependency type     | Why it must exist first                                             | Notes                               |
| --------------------------- | ----------- | ------------------- | ------------------------------------------------------------------- | ----------------------------------- |
| ordered registration helper | required    | registration helper | roving focus and focus restoration depend on stable trigger order   | shared with `tabs`                  |
| measurement helper          | required    | measurement helper  | indicator and viewport sizing depend on live rects                  | shared with `tabs`                  |
| timer helper                | required    | timer helper        | open-delay, skip-delay, and close-delay ownership are adapter-owned | must support cleanup                |
| z-index allocator           | recommended | overlay helper      | viewport content layering depends on shared stacking policy         | consume `z-index-stacking` guidance |

## 17. Recommended Implementation Sequence

1. Initialize the core machine.
2. Publish root context and ordered trigger registration.
3. Render root, list, triggers, and active content.
4. Add timer lifecycle and focus restoration.
5. Add viewport handoff, indicator measurement, and z-index allocation.

## 18. Anti-Patterns

- Do not leave timer cleanup implicit.
- Do not measure every trigger on every render when only one is active.
- Do not detach active content from its owning trigger semantics when handing off into the viewport.

## 19. Consumer Expectations and Guarantees

- Consumers may assume Escape closes the open content and restores focus to its trigger.
- Consumers may assume direct-link items do not participate in content timers.
- Consumers must not assume the viewport exists unless `with_viewport=true`.

## 20. Platform Support Matrix

| Capability / behavior                                             | Browser client | SSR          | Notes                                                 |
| ----------------------------------------------------------------- | -------------- | ------------ | ----------------------------------------------------- |
| menubar semantics, trigger registration, and open content linkage | full support   | full support | baseline behavior                                     |
| delay timers, viewport measurement, and indicator positioning     | full support   | client-only  | visual and timing behaviors                           |
| viewport fallback to inline content                               | fallback path  | full support | used when viewport node or measurement is unavailable |

## 21. Debug Diagnostics and Production Policy

| Condition                                          | Debug build behavior | Production behavior | Notes                        |
| -------------------------------------------------- | -------------------- | ------------------- | ---------------------------- |
| repeated part outside root context                 | fail fast            | fail fast           | compound structure violation |
| missing measurement target while indicator enabled | debug warning        | degrade gracefully  | hide indicator               |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required?   | Responsibility                                        | Reused by               | Notes                              |
| --------------------------- | ----------- | ----------------------------------------------------- | ----------------------- | ---------------------------------- |
| ordered registration helper | required    | keeps trigger order and focus restoration stable      | `tabs`, `accordion`     | DOM-order source of truth          |
| measurement helper          | required    | computes indicator and viewport CSS custom properties | `tabs`                  | active-item-only measurement       |
| timer helper                | required    | owns open, close, and skip-delay timer lifecycle      | hover-driven composites | cleanup critical                   |
| z-index helper              | recommended | allocates viewport layering                           | overlay surfaces        | aligns with shared stacking policy |

## 23. Framework-Specific Behavior

Leptos should publish the navigation-menu context with `provide_context`, keep timer handles in instance-local effects, and compose trigger `NodeRef`s with any wrapper refs. Controlled `value` should be a watched `Signal<Option<Key>>` only when it changes after mount.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<navigation_menu::Machine>(props);
provide_context(Context::from_machine(machine));

view! {
    <nav {..machine.derive(|api| api.root_attrs()).get()}>
        {children()}
    </nav>
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine and ordered trigger registry.
- Register timer handles and cancel them on every conflicting transition.
- Render active content inline or into the viewport according to the documented branch.
- Measure only the active trigger and active content to update CSS custom properties.

## 26. Adapter Invariants

- Trigger registry order always matches DOM order.
- Escape always restores focus to the owning trigger.
- Active content ownership remains tied to the active trigger even when handed into the viewport.
- Timer handles never survive unmount or state changes that invalidate them.

## 27. Accessibility and SSR Notes

- `Root` owns the navigation landmark and localized label.
- `List` owns `role="menubar"` and orientation attrs.
- SSR must preserve initial active-item semantics and not rely on timers or measurement.

## 28. Parity Summary and Intentional Deviations

- Matches the core navigation-menu contract without intentional adapter divergence.
- Promotes trigger registration, timer ownership, focus restoration, viewport handoff, measurement, and z-index policy into explicit Leptos-facing rules.

## 29. Test Scenarios

- delayed hover open with skip-delay shortcut
- Escape close with focus restoration
- direct-link item that bypasses content timers
- indicator measurement update on active trigger change
- viewport fallback to inline content when measurement is unavailable

## 30. Test Oracle Notes

- Inspect DOM attrs for `aria-expanded`, `aria-controls`, and active motion attrs.
- Use timer-controlled browser tests to verify delayed open, delayed close, and skip-delay behavior.
- Assert Escape restores focus to the owning trigger even after viewport handoff.
- Verify indicator and viewport CSS custom properties update only for the active item.

## 31. Implementation Checklist

- [ ] Register triggers in DOM order and clean them up by key.
- [ ] Own open, close, and skip-delay timers inside the adapter.
- [ ] Restore focus to the owning trigger on Escape.
- [ ] Keep active content ownership stable during viewport handoff.
- [ ] Measure only the active trigger and active content.
- [ ] Fall back cleanly when viewport or measurement support is unavailable.
