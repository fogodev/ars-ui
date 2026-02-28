---
adapter: leptos
component: tooltip
category: overlay
source: components/overlay/tooltip.md
source_foundation: foundation/08-adapter-leptos.md
---

# Tooltip -- Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Tooltip`](../../components/overlay/tooltip.md) behavior to Leptos 0.8.x. The adapter owns the compound component tree (`Tooltip`, `Trigger`, `Positioner`, `Content`, `Arrow`), hover/focus event wiring, Escape keydown handling with `stopPropagation`, portal rendering of positioned content, positioning engine integration with CSS custom property updates, timer lifecycle (open delay, close delay, touch auto-hide), `Group` warmup/cooldown coordination context, the always-rendered `HiddenDescription` span, lazy mount / unmount-on-exit gating via `Presence`, scroll listener for `close_on_scroll`, and z-index allocation.

## 2. Public Adapter API

```rust
/// Root component: initializes the machine, provides Context.
#[component]
pub fn Tooltip(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional, default = false)] default_open: bool,
    #[prop(optional, default = 300)] open_delay_ms: u32,
    #[prop(optional, default = 300)] close_delay_ms: u32,
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional, default = false)] interactive: bool,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional, default = true)] close_on_escape: bool,
    #[prop(optional, default = true)] close_on_click: bool,
    #[prop(optional, default = true)] close_on_scroll: bool,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional, default = false)] lazy_mount: bool,
    #[prop(optional, default = false)] unmount_on_exit: bool,
    #[prop(optional)] dir: Option<Direction>,
    #[prop(optional, default = 20000)] touch_auto_hide_ms: u32,
    #[prop(optional)] messages: Option<tooltip::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView

/// Trigger component: renders the hover/focus target.
#[component]
pub fn Trigger(
    #[prop(optional)] as_child: Option<Callback<TriggerRenderProps, AnyView>>,
    children: Children,
) -> impl IntoView

/// Positioner component: positioned container rendered into the portal.
#[component]
pub fn Positioner(
    children: Children,
) -> impl IntoView

/// Content component: the visible tooltip surface (role="tooltip").
#[component]
pub fn Content(
    children: Children,
) -> impl IntoView

/// Arrow component: optional directional arrow inside the positioner.
#[component]
pub fn Arrow() -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with core `tooltip::Props`. `open` uses `Option<Signal<bool>>` for controlled mode.
- Event parity: `PointerEnter`, `PointerLeave`, `Focus`, `Blur`, `ContentPointerEnter`, `ContentPointerLeave`, `CloseOnEscape`, `CloseOnClick`, `CloseOnScroll` are all wired. Timer events (`OpenTimerFired`, `CloseTimerFired`) fire from platform effects. Programmatic `Open`/`Close` are dispatched from controlled prop sync.
- Structure parity: all six core parts (Root, Trigger, HiddenDescription, Positioner, Content, Arrow) are rendered. The `HiddenDescription` span is always rendered regardless of open state.
- Behavior parity: 4-state machine (Closed, OpenPending, Open, ClosePending), interactive tooltip WCAG 1.4.13 compliance with minimum 200ms close delay, touch auto-hide, `Group` warmup/cooldown.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                              | Ownership                                        | Attr source                                         | Notes                                                        |
| --------------------- | --------- | ----------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------ |
| Root                  | required  | `<div>` wrapper around all parts                      | adapter-owned                                    | `api.root_attrs()`                                  | Structural container, no visual output.                      |
| Trigger               | required  | consumer child or `as_child` render prop              | consumer-owned content in adapter-owned wrapper  | `api.trigger_attrs()` plus adapter event handlers   | Always has `aria-describedby` pointing to HiddenDescription. |
| HiddenDescription     | required  | visually-hidden `<span>` inside Root, always rendered | adapter-owned                                    | `api.hidden_description_attrs()`                    | Contains tooltip text; never conditionally mounted.          |
| Positioner            | required  | `<div>` inside portal, positioned via `ars-dom`       | adapter-owned                                    | `api.positioner_attrs()` plus CSS custom properties | Only visible when open; gated by Presence.                   |
| Content               | required  | `<div>` inside Positioner with `role="tooltip"`       | adapter-owned structure, consumer-owned children | `api.content_attrs()`                               | Visual tooltip surface; `aria-hidden="true"`.                |
| Arrow                 | optional  | `<div>` inside Positioner                             | adapter-owned                                    | `api.arrow_attrs()`                                 | Positioned by the positioning engine.                        |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                       | Adapter-owned attrs                                                                                                                                                                                                    | Consumer attrs                       | Merge order                                                                                  | Ownership notes                                    |
| ----------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| Root              | `api.root_attrs()`               | `data-ars-scope`, `data-ars-state`                                                                                                                                                                                     | consumer `class`/`style` only        | core state and scope attrs win; `class`/`style` merge additively                             | adapter-owned structural wrapper                   |
| Trigger           | `api.trigger_attrs()`            | `onpointerenter`, `onpointerleave`, `onfocus`, `onblur`, `onkeydown`, `onclick` handlers                                                                                                                               | consumer element attrs               | core `aria-describedby` wins; adapter event handlers compose; consumer `class`/`style` merge | adapter-owned event wiring around consumer content |
| HiddenDescription | `api.hidden_description_attrs()` | visually-hidden inline styles                                                                                                                                                                                          | none                                 | core ID wins                                                                                 | adapter-owned; consumers do not touch this node    |
| Positioner        | `api.positioner_attrs()`         | CSS custom properties (`--ars-x`, `--ars-y`, `--ars-z-index`, `--ars-reference-width`, `--ars-reference-height`, `--ars-available-width`, `--ars-available-height`, `--ars-transform-origin`), `style` for positioning | none                                 | adapter positioning attrs win                                                                | adapter-owned; rendered into portal                |
| Content           | `api.content_attrs()`            | `aria-hidden="true"`, `onpointerenter`, `onpointerleave` (when interactive)                                                                                                                                            | consumer `class`/`style` on children | core `role`, `id`, `dir`, `data-ars-state` win; `class`/`style` merge                        | adapter-owned structure with consumer children     |
| Arrow             | `api.arrow_attrs()`              | arrow positioning CSS custom properties                                                                                                                                                                                | consumer `class`/`style`             | core part attrs win                                                                          | adapter-owned                                      |

- The `aria-describedby` on Trigger always points to the HiddenDescription span ID, not to the Content ID.
- Content carries `aria-hidden="true"` because it is visual-only; the accessible description lives in HiddenDescription.
- Positioner CSS custom properties are set by the positioning engine after each computation cycle and must not be overridden by consumers.

## 6. Composition / Context Contract

`Tooltip` provides a `Context` via `provide_context`. All child parts consume it via `use_context::<Context>().expect("tooltip::Trigger/Content/etc. must be used within a Tooltip")`.

```rust
#[derive(Clone, Copy)]
struct Context {
    open: Signal<bool>,
    send: StoredValue<Box<dyn Fn(tooltip::Event)>>,
    trigger_ref: NodeRef<html::Div>,
    positioner_ref: NodeRef<html::Div>,
    content_ref: NodeRef<html::Div>,
    service: StoredValue<Service<tooltip::Machine>>,
    context_version: ReadSignal<u64>,
    interactive: bool,
    lazy_mount: bool,
    unmount_on_exit: bool,
}
```

`Group` coordination: `Tooltip` reads `use_context::<GroupContext>()` when present. If warm, it skips `OpenPending` and transitions directly to `Open`. On open, it records itself as active and closes any previously active tooltip. On close, it records `last_close_at` for warmup tracking.

Portal rendering: Positioner, Content, and Arrow render into the portal root obtained from `use_context::<ArsContext>()`. Z-index is allocated from `use_context::<ZIndexAllocator>()`.

## 7. Prop Sync and Event Mapping

| Adapter prop                                             | Mode                          | Sync trigger              | Machine event / update path                        | Visible effect                  | Notes                                                 |
| -------------------------------------------------------- | ----------------------------- | ------------------------- | -------------------------------------------------- | ------------------------------- | ----------------------------------------------------- |
| `open`                                                   | controlled via `Signal<bool>` | signal change after mount | `Event::Open` or `Event::Close`                    | opens or closes tooltip         | deferred `create_effect` to avoid body-level dispatch |
| `disabled`                                               | non-reactive                  | render time               | `Context.disabled`                                 | blocks all transitions          | rebuild machine on change                             |
| `positioning`                                            | non-reactive                  | render time               | `Context.positioning`                              | repositions content             | repositioning runs on open                            |
| `open_delay_ms` / `close_delay_ms`                       | non-reactive                  | render time               | `Context.open_delay_ms` / `Context.close_delay_ms` | adjusts timer durations         | interactive minimum 200ms enforced by machine         |
| `close_on_escape` / `close_on_click` / `close_on_scroll` | non-reactive                  | render time               | guards in `transition()`                           | enables or disables close paths | checked per-event in the machine                      |
| `on_open_change`                                         | callback                      | open state change         | notification after transition                      | consumer notified of open/close | fires after machine settles                           |
| `touch_auto_hide_ms`                                     | non-reactive                  | render time               | adapter-local timer config                         | auto-hides on touch devices     | minimum 5000ms clamped                                |

| UI event                    | Preconditions                              | Machine event / callback path                         | Ordering notes                                            | Notes                                 |
| --------------------------- | ------------------------------------------ | ----------------------------------------------------- | --------------------------------------------------------- | ------------------------------------- |
| `pointerenter` on trigger   | not disabled, pointer type not touch       | `Event::PointerEnter`                                 | checks `Group` warmup before dispatch                     | starts open delay or immediate open   |
| `pointerleave` on trigger   | tooltip open or pending                    | `Event::PointerLeave`                                 | none                                                      | starts close delay or immediate close |
| `focus` on trigger          | not disabled                               | `Event::Focus`                                        | none                                                      | starts open delay or immediate open   |
| `blur` on trigger           | tooltip open or pending                    | `Event::Blur`                                         | none                                                      | starts close delay or immediate close |
| `keydown` Escape on trigger | tooltip open or pending, `close_on_escape` | `Event::CloseOnEscape` via `api.on_trigger_keydown()` | adapter MUST call `event.stop_propagation()` when handled | prevents parent Dialog from closing   |
| `click` on trigger          | tooltip open or pending, `close_on_click`  | `Event::CloseOnClick`                                 | fires after pointer events                                | dismisses tooltip on click            |
| `pointerenter` on content   | interactive, tooltip in ClosePending       | `Event::ContentPointerEnter`                          | cancels close timer                                       | WCAG 1.4.13 hoverable content         |
| `pointerleave` on content   | interactive, tooltip open                  | `Event::ContentPointerLeave`                          | starts close delay                                        | restarts close timer                  |
| scroll (document)           | tooltip open, `close_on_scroll`            | `Event::CloseOnScroll`                                | client-only listener                                      | prevents stale positioning            |

## 8. Registration and Cleanup Contract

| Registered entity               | Registration trigger                      | Identity key     | Cleanup trigger                                      | Cleanup action                       | Notes                                        |
| ------------------------------- | ----------------------------------------- | ---------------- | ---------------------------------------------------- | ------------------------------------ | -------------------------------------------- |
| open delay timer                | `PointerEnter` or `Focus` in Closed state | tooltip instance | `OpenTimerFired`, state change to Closed, or unmount | `clear_timeout` via platform effects | client-only                                  |
| close delay timer               | `PointerLeave` or `Blur` in Open state    | tooltip instance | `CloseTimerFired`, re-entry, or unmount              | `clear_timeout` via platform effects | client-only                                  |
| touch auto-hide timer           | tooltip opens with touch pointer type     | tooltip instance | tooltip closes or unmount                            | `clear_timeout`                      | client-only; minimum 5000ms                  |
| scroll listener                 | tooltip opens with `close_on_scroll`      | tooltip instance | tooltip closes or unmount                            | `removeEventListener`                | document-level, client-only                  |
| positioning engine subscription | positioner mounts in portal               | tooltip instance | tooltip closes or unmount                            | unsubscribe from positioning updates | client-only                                  |
| z-index allocation              | positioner mounts                         | tooltip instance | tooltip closes or unmount                            | release z-index back to allocator    | via `ZIndexAllocator` context                |
| `Group` registration            | tooltip opens                             | tooltip ID       | tooltip closes or unmount                            | `record_close()` on the group        | optional; only when group context is present |
| portal mount                    | positioner renders                        | tooltip instance | unmount                                              | remove portal children               | via `ArsProvider`                            |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule                               | Notes                                                        |
| ------------------ | ------------- | ------------- | ---------------------------------- | ---------------------------------------------- | ------------------------------------------------------------ |
| Root               | no            | adapter-owned | always structural, handle optional | no composition needed                          | structural wrapper only                                      |
| Trigger            | yes           | adapter-owned | required after mount               | compose with consumer `as_child` when provided | needed as anchor for positioning engine                      |
| HiddenDescription  | no            | adapter-owned | always structural, handle optional | no composition needed                          | always rendered for accessibility                            |
| Positioner         | yes           | adapter-owned | client-only                        | no composition needed                          | positioning engine writes CSS custom properties to this node |
| Content            | yes           | adapter-owned | client-only                        | no composition needed                          | content pointer events for interactive mode                  |
| Arrow              | no            | adapter-owned | client-only                        | no composition needed                          | positioning engine computes arrow offset                     |

## 10. State Machine Boundary Rules

- machine-owned state: `State` (Closed, OpenPending, Open, ClosePending), `Context.open`, `Context.hover_active`, `Context.focus_active`, and all timer lifecycle decisions (when to start, cancel, or fire timers).
- adapter-local derived bookkeeping: trigger `NodeRef`, positioner `NodeRef`, content `NodeRef`, positioning engine subscription handle, scroll listener handle, touch auto-hide timer handle, `Group` registration, portal mount state, CSS custom property values from positioning engine output.
- forbidden local mirrors: do not keep a local `is_open` boolean that can diverge from `api.is_open()`. Do not keep a local timer ID that is not owned by the machine's `PendingEffect` system.
- allowed snapshot-read contexts: positioning callbacks, scroll listener, touch auto-hide timer, `on_open_change` notification, `Group` warmup check.

## 11. Callback Payload Contract

| Callback                      | Payload source           | Payload shape           | Timing                           | Cancelable?                  | Notes                                                                        |
| ----------------------------- | ------------------------ | ----------------------- | -------------------------------- | ---------------------------- | ---------------------------------------------------------------------------- |
| `on_open_change`              | machine-derived snapshot | `bool` (new open state) | after machine transition settles | no                           | fires for all open/close transitions including programmatic                  |
| `on_trigger_keydown` (Escape) | raw framework event      | `KeyboardEvent`         | before `CloseOnEscape` dispatch  | yes (via `stop_propagation`) | adapter MUST stop propagation when `api.on_trigger_keydown()` returns `true` |

## 12. Failure and Degradation Rules

| Condition                                            | Policy             | Notes                                                            |
| ---------------------------------------------------- | ------------------ | ---------------------------------------------------------------- |
| trigger ref missing after mount                      | fail fast          | positioning engine cannot anchor without the trigger node        |
| positioner ref missing after mount                   | fail fast          | CSS custom properties cannot be set without the positioner node  |
| portal root unavailable (`ArsProvider` not provided) | degrade gracefully | render positioner inline instead of in portal; log debug warning |
| `ZIndexAllocator` context missing                    | degrade gracefully | use fallback z-index value; log debug warning                    |
| `Group` context missing                              | no-op              | each tooltip operates independently without warmup/cooldown      |
| browser timer APIs absent during SSR                 | no-op              | timers are client-only; structure renders without delay behavior |
| positioning engine fails to compute                  | degrade gracefully | positioner renders at default position; log debug warning        |
| scroll listener API unavailable                      | warn and ignore    | `close_on_scroll` becomes ineffective                            |

## 13. Identity and Key Policy

| Registered or repeated structure  | Identity source                  | Duplicates allowed?          | DOM order must match registration order? | SSR/hydration stability                          | Notes                                            |
| --------------------------------- | -------------------------------- | ---------------------------- | ---------------------------------------- | ------------------------------------------------ | ------------------------------------------------ |
| tooltip instance in `Group`       | data-derived (from `props.id`)   | no (single-open enforcement) | not applicable                           | ID must remain stable across hydration           | group uses ID to track active tooltip            |
| HiddenDescription span            | data-derived (from `content_id`) | not applicable               | not applicable                           | ID must remain stable across hydration           | `aria-describedby` on trigger references this ID |
| trigger `aria-describedby` target | data-derived (from `content_id`) | not applicable               | not applicable                           | must match HiddenDescription ID across hydration | stable cross-reference                           |

## 14. SSR and Client Boundary Rules

- SSR renders: Root, Trigger (with `aria-describedby`), HiddenDescription span (with tooltip text content). These three must be present in the server HTML for screen reader accessibility.
- SSR does NOT render: Positioner, Content, Arrow. These are client-only because they depend on portal rendering and positioning engine computation. If `default_open: true`, they render after hydration completes.
- No timer may start during SSR. Open delay, close delay, touch auto-hide, and scroll listeners are all client-only.
- `on_open_change` must not fire during SSR.
- Controlled `open` signal sync via `create_effect` is client-only.
- `Group` warmup check is client-only.
- The trigger's `aria-describedby` attribute and HiddenDescription `id` must remain stable across hydration.

## 15. Performance Constraints

- Pointer event handlers on the trigger must not allocate or clone on every move; they should dispatch a lightweight machine event.
- The positioning engine must debounce or batch updates when the tooltip repositions due to scroll or resize.
- Scroll listeners must use passive event listeners where supported to avoid blocking scroll.
- `Group` warmup check must be a constant-time `performance.now()` comparison, not a linear scan of all tooltips.
- Timer lifecycle (start, cancel, restart) must not churn effects on every render; timers are managed by the machine's `PendingEffect` system.
- CSS custom properties on the positioner should be set via direct style mutation, not by triggering a full reactive re-render.
- Portal mount/unmount should not cause layout thrash in sibling components.

## 16. Implementation Dependencies

| Dependency                     | Required?   | Dependency type         | Why it must exist first                                                  | Notes                                                 |
| ------------------------------ | ----------- | ----------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------- |
| `ars-provider`                 | required    | context contract        | portal root for positioner rendering and document-level listener scoping | overlays must render into the portal root             |
| `z-index-allocator`            | required    | context contract        | z-index allocation for the positioner layer                              | prevents hardcoded z-index values                     |
| `presence`                     | required    | composition contract    | mount/unmount animation lifecycle for positioner/content/arrow           | `lazy_mount` and `unmount_on_exit` depend on Presence |
| `dismissable`                  | recommended | behavioral prerequisite | touch device outside-tap detection when tooltip is open                  | touch behavior composes InteractOutside               |
| `positioning engine (ars-dom)` | required    | shared helper           | computes positioner placement and CSS custom properties                  | shared across Popover, HoverCard, Tooltip, Tour       |

## 17. Recommended Implementation Sequence

1. Implement `Tooltip`: machine initialization, context provision, controlled prop sync.
2. Implement `Trigger`: trigger ref, event wiring (pointer, focus, keydown, click), `as_child` support, `aria-describedby`.
3. Implement `HiddenDescription`: always-rendered visually-hidden span with stable ID.
4. Implement `Positioner`: portal rendering, positioning engine integration, CSS custom property updates, z-index allocation.
5. Implement `Content`: `role="tooltip"`, `aria-hidden="true"`, interactive pointer events, `data-ars-state`.
6. Implement `Arrow`: arrow positioning within the positioner.
7. Wire Presence composition for lazy mount / unmount on exit.
8. Wire scroll listener for `close_on_scroll`.
9. Wire touch auto-hide timer.
10. Implement `Group` provider and warmup/cooldown integration.
11. Verify Escape `stopPropagation` behavior inside nested overlays.
12. Verify cleanup ordering: timers, listeners, positioning, portal, z-index, group registration.

## 18. Anti-Patterns

- Do not omit the HiddenDescription span or conditionally render it based on open state; it must always be in the DOM.
- Do not point `aria-describedby` at the Content element; it must point at the HiddenDescription span.
- Do not start timers during SSR.
- Do not attach scroll listeners during SSR.
- Do not render the Positioner inline with the trigger; it must render into the portal root.
- Do not hardcode z-index values; use the `ZIndexAllocator`.
- Do not keep a local `is_open` boolean separate from the machine state.
- Do not skip `event.stop_propagation()` when `api.on_trigger_keydown()` returns `true` for Escape.
- Do not use `setTimeout` directly; use the platform effects system for timer management.
- Do not ignore the minimum 200ms close delay enforcement for interactive tooltips.
- Do not allow multiple tooltips to be open simultaneously within the same `Group`.
- Do not omit `aria-hidden="true"` on the Content element.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the HiddenDescription span is always in the DOM and always referenced by `aria-describedby` on the trigger, regardless of open state.
- Consumers may assume only one tooltip is open at a time within a `Group` scope.
- Consumers may assume that `on_open_change` fires for all open/close transitions, including timer-driven, Escape-driven, scroll-driven, click-driven, and programmatic.
- Consumers may assume that pressing Escape while the tooltip is open does not propagate to parent overlays.
- Consumers may assume that interactive tooltip content remains hoverable per WCAG 1.4.13.
- Consumers may assume that the positioner receives CSS custom properties for positioning after each computation cycle.
- Consumers must not assume the Content element carries the accessible description; the HiddenDescription span does.
- Consumers must not assume the tooltip is rendered inline with the trigger; it renders in the portal.
- Consumers must not assume tooltip content is interactive by default; `interactive: true` must be set explicitly.
- Consumers must not assume the tooltip opens immediately; the open delay applies unless the `Group` is warm.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR            | Notes                                          |
| ----------------------------------------- | -------------- | -------------- | ---------------------------------------------- |
| trigger rendering with `aria-describedby` | full support   | full support   | trigger and HiddenDescription render on server |
| HiddenDescription span                    | full support   | full support   | always rendered for screen reader access       |
| positioned overlay content                | full support   | client-only    | depends on portal and positioning engine       |
| open/close delay timers                   | full support   | client-only    | platform effects system                        |
| touch auto-hide timer                     | full support   | client-only    | touch detection is client-only                 |
| scroll listener                           | full support   | client-only    | document-level listener                        |
| `Group` warmup/cooldown                   | full support   | client-only    | `performance.now()` is client-only             |
| z-index allocation                        | full support   | SSR-safe empty | allocator context may provide fallback         |
| positioning engine CSS custom properties  | full support   | client-only    | computed after mount                           |
| Escape dismiss with `stopPropagation`     | full support   | client-only    | keyboard events are client-only                |
| Presence animation lifecycle              | full support   | SSR-safe empty | animations are client-only                     |

## 21. Debug Diagnostics and Production Policy

| Condition                                    | Debug build behavior | Production behavior | Notes                                      |
| -------------------------------------------- | -------------------- | ------------------- | ------------------------------------------ |
| trigger ref missing after mount              | fail fast            | fail fast           | positioning cannot function without anchor |
| positioner ref missing after mount           | fail fast            | fail fast           | CSS custom properties cannot be set        |
| `ArsProvider` context missing                | debug warning        | degrade gracefully  | falls back to inline rendering             |
| `ZIndexAllocator` context missing            | debug warning        | degrade gracefully  | uses fallback z-index                      |
| `Group` context missing                      | no-op                | no-op               | independent tooltip operation is valid     |
| positioning engine returns error             | debug warning        | degrade gracefully  | render at default position                 |
| `close_on_scroll` but scroll API unavailable | debug warning        | warn and ignore     | scroll dismissal becomes ineffective       |
| touch auto-hide timer clamped below 5000ms   | debug warning        | degrade gracefully  | clamp to 5000ms silently in production     |
| interactive tooltip close delay below 200ms  | debug warning        | degrade gracefully  | machine enforces 200ms minimum             |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                  | Reused by                                 | Notes                                    |
| -------------------------- | ----------- | ------------------------------------------------------------------------------- | ----------------------------------------- | ---------------------------------------- |
| positioning helper         | required    | compute placement, set CSS custom properties on positioner, handle arrow offset | Tooltip, Popover, HoverCard, Tour         | `ars-dom` positioning engine integration |
| portal helper              | required    | render positioner/content/arrow into portal root                                | Tooltip, Popover, HoverCard, Dialog, Tour | via `ArsProvider`                        |
| z-index helper             | required    | allocate and release z-index for the overlay layer                              | Tooltip, Popover, HoverCard, Dialog, Tour | via `ZIndexAllocator`                    |
| timer helper               | required    | manage open delay, close delay, touch auto-hide timers via platform effects     | Tooltip, HoverCard, Toast                 | cleanup must cancel all pending timers   |
| presence helper            | required    | gate mount/unmount of positioner subtree for animation                          | Tooltip, Popover, HoverCard, Dialog       | `lazy_mount` and `unmount_on_exit`       |
| merge helper               | recommended | merge core attrs with adapter and consumer attrs                                | all overlay components                    | `class`/`style` merge additively         |
| context publication helper | recommended | standardize `provide_context` pattern for compound components                   | Tooltip, Popover, Dialog                  | consistent context shape                 |

## 23. Framework-Specific Behavior

Leptos uses `NodeRef<html::Div>` for trigger, positioner, and content refs. Event handlers are attached via `on:pointerenter`, `on:pointerleave`, `on:focus`, `on:blur`, `on:keydown`, `on:click` in the `view!` macro. `on_cleanup` handles timer cancellation, listener removal, positioning unsubscription, z-index release, and group deregistration. Controlled open state is synced via `create_effect` watching the `Signal<bool>`, dispatching `Event::Open` or `Event::Close` with a previous-value guard to avoid re-entrance. The portal is rendered using `leptos::portal::Portal` or the `ArsProvider` portal root. Scroll listeners are attached via `web_sys::EventTarget::add_event_listener_with_callback` on `document` with passive option.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Tooltip(
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional, default = false)] default_open: bool,
    #[prop(optional, default = 300)] open_delay_ms: u32,
    #[prop(optional, default = 300)] close_delay_ms: u32,
    #[prop(optional, default = false)] interactive: bool,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView {
    let core_props = tooltip::Props {
        open: open.map(|s| s.get_untracked()),
        default_open,
        open_delay_ms,
        close_delay_ms,
        interactive,
        on_open_change: on_open_change.clone(),
        ..Default::default()
    };

    let machine = use_machine::<tooltip::Machine>(core_props);
    let is_open = machine.derive(|api| api.is_open());

    // Controlled open sync
    if let Some(open_sig) = open {
        let send = machine.send.clone();
        let mut prev = None::<bool>;
        create_effect(move |_| {
            let new_open = open_sig.get();
            if prev.is_some() && prev != Some(new_open) {
                if new_open {
                    send(tooltip::Event::Open);
                } else {
                    send(tooltip::Event::Close);
                }
            }
            prev = Some(new_open);
        });
    }

    // Group integration
    if let Some(group) = use_context::<GroupContext>() {
        // Register open/close with group for warmup tracking
    }

    provide_context(Context {
        open: is_open,
        send: machine.send.clone(),
        service: machine.service,
        context_version: machine.context_version,
        interactive,
    });

    view! {
        <div {..machine.derive(|api| api.root_attrs())}>
            {children()}
        </div>
    }
}

#[component]
pub fn Trigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("tooltip::Trigger must be used within a Tooltip");
    let trigger_ref = NodeRef::<html::Div>::new();

    view! {
        <div
            node_ref=trigger_ref
            {..ctx.service.with_value(|s| s.api().trigger_attrs())}
            on:pointerenter=move |_| (ctx.send)(tooltip::Event::PointerEnter)
            on:pointerleave=move |_| (ctx.send)(tooltip::Event::PointerLeave)
            on:focus=move |_| (ctx.send)(tooltip::Event::Focus)
            on:blur=move |_| (ctx.send)(tooltip::Event::Blur)
            on:keydown=move |ev| {
                let handled = ctx.service.with_value(|s| {
                    s.api().on_trigger_keydown(&KeyboardEventData::from(&ev))
                });
                if handled {
                    ev.stop_propagation();
                }
            }
            on:click=move |_| (ctx.send)(tooltip::Event::CloseOnClick)
        >
            {children()}
        </div>
    }
}

#[component]
pub fn Positioner(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("tooltip::Positioner must be used within a Tooltip");
    let positioner_ref = NodeRef::<html::Div>::new();

    // Portal rendering + positioning engine integration (client-only)
    // Sets --ars-x, --ars-y, --ars-z-index, etc. on positioner_ref

    view! {
        <Show when=move || ctx.open.get()>
            <Portal>
                <div
                    node_ref=positioner_ref
                    {..ctx.service.with_value(|s| s.api().positioner_attrs())}
                >
                    {children()}
                </div>
            </Portal>
        </Show>
    }
}

#[component]
pub fn Content(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("tooltip::Content must be used within a Tooltip");

    view! {
        <div
            {..ctx.service.with_value(|s| s.api().content_attrs())}
            aria-hidden="true"
            on:pointerenter=move |_| {
                if ctx.interactive {
                    (ctx.send)(tooltip::Event::ContentPointerEnter);
                }
            }
            on:pointerleave=move |_| {
                if ctx.interactive {
                    (ctx.send)(tooltip::Event::ContentPointerLeave);
                }
            }
        >
            {children()}
        </div>
    }
}
```

The HiddenDescription span is rendered inside the Root, always present:

```rust
// Inside Tooltip, after the trigger children
view! {
    <span
        {..machine.derive(|api| api.hidden_description_attrs())}
        style="position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0;"
    >
        // Consumer provides tooltip text as children or via a slot prop
    </span>
}
```

## 25. Reference Implementation Skeleton

```rust
// Tooltip
let core_props = build_core_props(adapter_props);
let machine = use_machine::<tooltip::Machine>(core_props);
let is_open = machine.derive(|api| api.is_open());
let trigger_ref = NodeRef::<html::Div>::new();
let positioner_ref = NodeRef::<html::Div>::new();
let content_ref = NodeRef::<html::Div>::new();

// Controlled open sync (client-only, deferred effect)
sync_controlled_open(open_signal, machine.send);

// Group warmup integration (client-only)
let group = use_context::<GroupContext>();
integrate_tooltip_group(group, machine.send, &core_props.id);

// Provide compound context
provide_context(Context { ... });

// HiddenDescription: always rendered inside Root
render_hidden_description(machine);

// Trigger
let trigger_attrs = machine.derive(|api| api.trigger_attrs());
attach_trigger_events(trigger_ref, machine.send);  // pointer, focus, keydown, click

// Positioner (client-only, portal, positioning engine)
create_effect(move |_| {
    if is_open.get() {
        let z = allocate_z_index();  // ZIndexAllocator
        let positioning_handle = start_positioning(trigger_ref, positioner_ref, positioning_opts);
        set_css_custom_properties(positioner_ref, positioning_handle.output());

        if close_on_scroll {
            attach_scroll_listener(machine.send);
        }

        if is_touch_device() {
            start_touch_auto_hide_timer(touch_auto_hide_ms, machine.send);
        }
    }
});

// Content: role="tooltip", aria-hidden="true", interactive pointer events
let content_attrs = machine.derive(|api| api.content_attrs());
if interactive {
    attach_content_pointer_events(content_ref, machine.send);
}

// Cleanup
on_cleanup(|| {
    cancel_all_timers();
    remove_scroll_listener();
    unsubscribe_positioning();
    release_z_index();
    deregister_from_tooltip_group();
    unmount_portal_children();
});
```

## 26. Adapter Invariants

- The HiddenDescription span MUST always be rendered regardless of tooltip open state.
- The trigger's `aria-describedby` MUST always point to the HiddenDescription span ID, never to the Content element ID.
- Content MUST carry `aria-hidden="true"` because the accessible description lives in HiddenDescription.
- When `api.on_trigger_keydown()` returns `true` for Escape, the adapter MUST call `event.stop_propagation()` to prevent the keydown from reaching parent overlays.
- Timer lifecycle (open delay, close delay) is owned by the machine's `PendingEffect` system; the adapter must not maintain parallel timer state.
- The interactive tooltip minimum close delay of 200ms is enforced by the machine; the adapter must not override or circumvent this.
- Only one tooltip may be visible at a time within a `Group` scope; the adapter must close the previously active tooltip when a new one opens.
- Positioner, Content, and Arrow MUST render into the portal root, not inline.
- Z-index MUST be allocated from `ZIndexAllocator`, not hardcoded.
- CSS custom properties on the positioner MUST be set by the positioning engine output, not hardcoded.
- Scroll listener for `close_on_scroll` MUST be client-only and MUST be removed when the tooltip closes or unmounts.
- Touch auto-hide timer MUST enforce a minimum of 5000ms, clamping lower values.
- No timer, listener, or positioning subscription may outlive the component; all MUST be cleaned up in `on_cleanup`.

## 27. Accessibility and SSR Notes

- The HiddenDescription span ensures screen reader users (including touch-only VoiceOver/TalkBack) always hear the tooltip description when focusing the trigger, regardless of whether the visual tooltip is open.
- The Content element uses `aria-hidden="true"` because it is a visual duplicate; the accessible description is in the HiddenDescription span.
- On SSR, the trigger renders with `aria-describedby` pointing to the HiddenDescription span, and the span renders with its content. This gives screen readers immediate access on page load.
- The Positioner/Content/Arrow do not render during SSR; they appear after hydration when the tooltip is opened.
- Escape dismissal is available on all platforms with keyboard support, including iPad with external keyboard.
- When `interactive: true`, a `DismissButton` SHOULD be rendered inside the tooltip content for screen reader users who cannot hover/focus away.
- Touch device auto-hide (minimum 5000ms, default 20000ms) satisfies WCAG 2.2.1 by giving users sufficient reading time.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, structure, and behavior parity. All six parts rendered. 4-state machine fully wired. Timer lifecycle, Group warmup/cooldown, touch auto-hide, interactive WCAG 1.4.13 compliance, scroll dismissal, and Escape `stopPropagation` all covered.

Intentional deviations: none.

Traceability note: This adapter spec explicitly covers the following core adapter-owned concerns: portal rendering of positioned content, positioning engine CSS custom property updates, timer management via platform effects, HiddenDescription always-rendered span, `aria-describedby` wiring from trigger to hidden span (not content), `aria-hidden="true"` on Content, Escape `stopPropagation`, scroll listener lifecycle, touch auto-hide timer with minimum clamping, `Group` warmup/cooldown coordination, z-index allocation, Presence composition for lazy mount / unmount on exit, and client-only boundary enforcement for all timer and listener behavior.

## 29. Test Scenarios

- HiddenDescription span is always rendered regardless of open state
- trigger `aria-describedby` points to HiddenDescription span ID
- Content has `aria-hidden="true"`
- hover trigger opens tooltip after open delay
- focus trigger opens tooltip after open delay
- pointer leave starts close delay; pointer re-entry cancels it
- Escape dismisses tooltip and stops propagation
- click dismisses tooltip when `close_on_click` is true
- scroll dismisses tooltip when `close_on_scroll` is true
- interactive tooltip: pointer enter on content cancels close delay
- interactive tooltip: pointer leave on content starts close delay
- interactive tooltip: minimum 200ms close delay enforced
- `Group` warmup: second tooltip opens immediately within cooldown window
- `Group` single-open: opening a new tooltip closes the previous one
- controlled open signal syncs with machine state
- `on_open_change` fires for all open/close transitions
- disabled tooltip does not open on hover or focus
- touch auto-hide fires after configured timeout
- touch auto-hide minimum 5000ms clamped
- lazy mount: content not in DOM until first open
- unmount on exit: content removed from DOM after close
- positioner renders into portal root
- CSS custom properties set on positioner after positioning
- z-index allocated from ZIndexAllocator
- all timers and listeners cleaned up on unmount

## 30. Test Oracle Notes

| Behavior                          | Preferred oracle type | Notes                                                           |
| --------------------------------- | --------------------- | --------------------------------------------------------------- |
| HiddenDescription always rendered | rendered structure    | assert `<span>` with correct ID exists regardless of open state |
| trigger `aria-describedby`        | DOM attrs             | assert value matches HiddenDescription span ID                  |
| Content `aria-hidden`             | DOM attrs             | assert `aria-hidden="true"` on content element                  |
| open/close delay timing           | machine state         | assert state transitions after timer fires                      |
| Escape stopPropagation            | callback order        | assert parent overlay does NOT receive the Escape keydown       |
| interactive content hover         | machine state         | assert ClosePending -> Open on ContentPointerEnter              |
| Group single-open                 | rendered structure    | assert only one tooltip content visible in group                |
| Group warmup                      | machine state         | assert direct Closed -> Open transition (skip OpenPending)      |
| controlled open sync              | machine state         | assert machine state follows signal changes                     |
| scroll listener cleanup           | cleanup side effects  | assert no scroll listener on document after unmount             |
| positioning CSS custom properties | DOM attrs             | assert `--ars-x`, `--ars-y`, `--ars-z-index` set on positioner  |
| portal rendering                  | rendered structure    | assert positioner is child of portal root, not trigger          |
| timer cleanup on unmount          | cleanup side effects  | assert no pending timers after unmount                          |

Cheap verification recipe:

1. Render a tooltip and assert the HiddenDescription span exists with the correct ID and that the trigger's `aria-describedby` references it, both before and after opening.
2. Open the tooltip via hover, verify Content appears with `role="tooltip"` and `aria-hidden="true"`, then press Escape and verify the tooltip closes and the Escape event does not propagate.
3. In a `Group`, open tooltip A, hover to tooltip B's trigger, and verify tooltip A closes and tooltip B opens immediately (warmup).
4. Unmount the component and verify no timers, scroll listeners, or positioning subscriptions remain active.

## 31. Implementation Checklist

- [ ] `Tooltip` initializes the machine and provides `Context` via `provide_context`.
- [ ] `Trigger` renders the trigger with `aria-describedby` pointing to HiddenDescription span ID.
- [ ] `Trigger` attaches pointer, focus, keydown, and click event handlers.
- [ ] Escape keydown calls `event.stop_propagation()` when `api.on_trigger_keydown()` returns `true`.
- [ ] HiddenDescription span is always rendered with visually-hidden styles and stable ID.
- [ ] Content element has `role="tooltip"`, `aria-hidden="true"`, and `data-ars-state`.
- [ ] Positioner renders into the portal root via `ArsProvider`.
- [ ] Positioning engine sets CSS custom properties on the positioner after each computation.
- [ ] Z-index is allocated from `ZIndexAllocator` and released on close/unmount.
- [ ] Open delay and close delay timers use the machine's `PendingEffect` system.
- [ ] Interactive tooltip close delay enforces minimum 200ms via machine.
- [ ] Interactive Content attaches `pointerenter`/`pointerleave` handlers.
- [ ] Scroll listener attaches on open (client-only) and removes on close/unmount.
- [ ] Touch auto-hide timer starts on touch open with minimum 5000ms.
- [ ] Controlled `open` signal syncs via `create_effect` with previous-value guard.
- [ ] `on_open_change` fires after all open/close transitions.
- [ ] `Group` integration: warmup check, single-open enforcement, close/open recording.
- [ ] Presence composition gates positioner subtree for `lazy_mount` and `unmount_on_exit`.
- [ ] All timers, listeners, positioning subscriptions, z-index allocations, and group registrations cleaned up in `on_cleanup`.
- [ ] SSR renders trigger and HiddenDescription only; positioner/content/arrow are client-only.
- [ ] Test scenarios from section 29 are covered.
