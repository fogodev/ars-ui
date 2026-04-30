---
adapter: dioxus
component: tooltip
category: overlay
source: components/overlay/tooltip.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Tooltip -- Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Tooltip`](../../components/overlay/tooltip.md) behavior to Dioxus 0.7.x. The adapter owns the compound component tree (`Tooltip`, `Trigger`, `Positioner`, `Content`, `Arrow`), hover/focus event wiring, Escape keydown handling with `prevent_default`/`stop_propagation`, portal rendering of positioned content, positioning engine integration with CSS custom property updates, timer lifecycle (open delay, close delay, touch auto-hide), `Group` warmup/cooldown coordination context, the always-rendered `HiddenDescription` span, lazy mount / unmount-on-exit gating via `Presence`, scroll listener for `close_on_scroll`, z-index allocation, and multi-platform behavior (Web, Desktop, Mobile, SSR).

## 2. Public Adapter API

```rust,no_check
/// Root component: initializes the machine, provides Context.
#[derive(Props, Clone, PartialEq)]
pub struct TooltipProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(optional, default = false)]
    pub default_open: bool,
    #[props(optional, default = 300)]
    pub open_delay_ms: u32,
    #[props(optional, default = 300)]
    pub close_delay_ms: u32,
    #[props(optional, default = false)]
    pub disabled: bool,
    #[props(optional, default = false)]
    pub interactive: bool,
    #[props(optional)]
    pub positioning: Option<PositioningOptions>,
    #[props(optional, default = true)]
    pub close_on_escape: bool,
    #[props(optional, default = true)]
    pub close_on_click: bool,
    #[props(optional, default = true)]
    pub close_on_scroll: bool,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(optional, default = false)]
    pub lazy_mount: bool,
    #[props(optional, default = false)]
    pub unmount_on_exit: bool,
    #[props(optional)]
    pub dir: Option<Direction>,
    #[props(optional, default = 20000)]
    pub touch_auto_hide_ms: u32,
    #[props(optional)]
    pub messages: Option<tooltip::Messages>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

#[component]
pub fn Tooltip(props: TooltipProps) -> Element

/// Trigger component: renders the hover/focus target.
#[derive(Props, Clone, PartialEq)]
pub struct TriggerProps {
    #[props(optional)]
    pub as_child: Option<EventHandler<TriggerRenderProps, Element>>,
    pub children: Element,
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element

/// Positioner component: positioned container rendered into the portal.
#[derive(Props, Clone, PartialEq)]
pub struct PositionerProps {
    pub children: Element,
}

#[component]
pub fn Positioner(props: PositionerProps) -> Element

/// Content component: the visible tooltip surface (role="tooltip").
#[derive(Props, Clone, PartialEq)]
pub struct ContentProps {
    pub children: Element,
}

#[component]
pub fn Content(props: ContentProps) -> Element

/// Arrow component: optional directional arrow inside the positioner.
#[component]
pub fn Arrow() -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with core `tooltip::Props`. `open` uses `Option<Signal<bool>>` for controlled mode; Dioxus `Signal<T>` is `Copy`.
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
- Content carries `aria-hidden="true"` because the accessible description lives in HiddenDescription.
- Positioner CSS custom properties are set by the positioning engine after each computation cycle and must not be overridden by consumers.

## 6. Composition / Context Contract

`Tooltip` provides a `Context` via `use_context_provider`. All child parts consume it via `try_use_context::<Context>().expect("tooltip::Trigger/Content/etc. must be used within a Tooltip")`.

```rust
#[derive(Clone, Copy)]
struct Context {
    open: ReadSignal<bool>,
    send: Callback<tooltip::Event>,
    service: Signal<Service<tooltip::Machine>>,
    context_version: ReadSignal<u64>,
    interactive: bool,
    lazy_mount: bool,
    unmount_on_exit: bool,
}
```

`Group` coordination: `Tooltip` reads `try_use_context::<GroupContext>()` when present. If warm, it skips `OpenPending` and transitions directly to `Open`. On open, it records itself as active and closes any previously active tooltip. On close, it records `last_close_at` for warmup tracking.

Portal rendering: Positioner, Content, and Arrow render into the portal root obtained from `try_use_context::<ArsContext>()`. Z-index is allocated from `try_use_context::<z_index_allocator::Context>()`.

## 7. Prop Sync and Event Mapping

| Adapter prop                                             | Mode                          | Sync trigger              | Machine event / update path                        | Visible effect                  | Notes                                              |
| -------------------------------------------------------- | ----------------------------- | ------------------------- | -------------------------------------------------- | ------------------------------- | -------------------------------------------------- |
| `open`                                                   | controlled via `Signal<bool>` | signal change after mount | `Event::Open` or `Event::Close`                    | opens or closes tooltip         | deferred `use_effect` to avoid body-level dispatch |
| `disabled`                                               | non-reactive                  | render time               | `Context.disabled`                                 | blocks all transitions          | rebuild machine on change                          |
| `positioning`                                            | non-reactive                  | render time               | `Context.positioning`                              | repositions content             | repositioning runs on open                         |
| `open_delay_ms` / `close_delay_ms`                       | non-reactive                  | render time               | `Context.open_delay_ms` / `Context.close_delay_ms` | adjusts timer durations         | interactive minimum 200ms enforced by machine      |
| `close_on_escape` / `close_on_click` / `close_on_scroll` | non-reactive                  | render time               | guards in `transition()`                           | enables or disables close paths | checked per-event in the machine                   |
| `on_open_change`                                         | callback                      | open state change         | notification after transition                      | consumer notified of open/close | fires after machine settles                        |
| `touch_auto_hide_ms`                                     | non-reactive                  | render time               | adapter-local timer config                         | auto-hides on touch devices     | minimum 5000ms clamped                             |

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
| scroll (document)           | tooltip open, `close_on_scroll`            | `Event::CloseOnScroll`                                | client-only listener (web)                                | prevents stale positioning            |

## 8. Registration and Cleanup Contract

| Registered entity               | Registration trigger                      | Identity key     | Cleanup trigger                                      | Cleanup action                       | Notes                                        |
| ------------------------------- | ----------------------------------------- | ---------------- | ---------------------------------------------------- | ------------------------------------ | -------------------------------------------- |
| open delay timer                | `PointerEnter` or `Focus` in Closed state | tooltip instance | `OpenTimerFired`, state change to Closed, or unmount | `clear_timeout` via platform effects | client-only                                  |
| close delay timer               | `PointerLeave` or `Blur` in Open state    | tooltip instance | `CloseTimerFired`, re-entry, or unmount              | `clear_timeout` via platform effects | client-only                                  |
| touch auto-hide timer           | tooltip opens with touch pointer type     | tooltip instance | tooltip closes or unmount                            | `clear_timeout`                      | client-only; minimum 5000ms                  |
| scroll listener                 | tooltip opens with `close_on_scroll`      | tooltip instance | tooltip closes or unmount                            | remove event listener                | web-only, client-only                        |
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
- adapter-local derived bookkeeping: trigger element ID, positioner element ID, content element ID, positioning engine subscription handle, scroll listener handle, touch auto-hide timer handle, `Group` registration, portal mount state, CSS custom property values from positioning engine output.
- forbidden local mirrors: do not keep a local `is_open` signal that can diverge from `api.is_open()`. Do not keep a local timer ID that is not owned by the machine's `PendingEffect` system.
- allowed snapshot-read contexts: positioning callbacks, scroll listener, touch auto-hide timer, `on_open_change` notification, `Group` warmup check.

## 11. Callback Payload Contract

| Callback                      | Payload source           | Payload shape           | Timing                           | Cancelable?                  | Notes                                                                        |
| ----------------------------- | ------------------------ | ----------------------- | -------------------------------- | ---------------------------- | ---------------------------------------------------------------------------- |
| `on_open_change`              | machine-derived snapshot | `bool` (new open state) | after machine transition settles | no                           | fires for all open/close transitions including programmatic                  |
| `on_trigger_keydown` (Escape) | raw framework event      | Dioxus `KeyboardEvent`  | before `CloseOnEscape` dispatch  | yes (via `stop_propagation`) | adapter MUST stop propagation when `api.on_trigger_keydown()` returns `true` |

## 12. Failure and Degradation Rules

| Condition                                            | Policy             | Notes                                                            |
| ---------------------------------------------------- | ------------------ | ---------------------------------------------------------------- |
| trigger ref/ID missing after mount                   | fail fast          | positioning engine cannot anchor without the trigger node        |
| positioner ref/ID missing after mount                | fail fast          | CSS custom properties cannot be set without the positioner node  |
| portal root unavailable (`ArsProvider` not provided) | degrade gracefully | render positioner inline instead of in portal; log debug warning |
| `ZIndexAllocator` context missing                    | degrade gracefully | use fallback z-index value; log debug warning                    |
| `Group` context missing                              | no-op              | each tooltip operates independently without warmup/cooldown      |
| browser timer APIs absent during SSR                 | no-op              | timers are client-only; structure renders without delay behavior |
| positioning engine fails to compute                  | degrade gracefully | positioner renders at default position; log debug warning        |
| scroll listener API unavailable (Desktop/Mobile)     | warn and ignore    | `close_on_scroll` becomes ineffective on non-web targets         |
| Desktop/Mobile pointer dispatch differs from web     | degrade gracefully | validate hover behavior against actual Dioxus target runtime     |

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
- Controlled `open` signal sync via `use_effect` is client-only.
- `Group` warmup check is client-only.
- The trigger's `aria-describedby` attribute and HiddenDescription `id` must remain stable across hydration.
- On Dioxus Desktop and Mobile, there is no SSR phase, but all timer and positioning behavior is still deferred until after mount.

## 15. Performance Constraints

- Pointer event handlers on the trigger must not allocate or clone on every move; they should dispatch a lightweight machine event.
- The positioning engine must debounce or batch updates when the tooltip repositions due to scroll or resize.
- Scroll listeners must use passive event listeners where supported (web) to avoid blocking scroll.
- `Group` warmup check must be a constant-time timestamp comparison, not a linear scan of all tooltips.
- Timer lifecycle (start, cancel, restart) must not churn effects on every render; timers are managed by the machine's `PendingEffect` system.
- CSS custom properties on the positioner should be set via direct style mutation, not by triggering a full reactive re-render.
- Portal mount/unmount should not cause layout thrash in sibling components.
- On Desktop and Mobile, positioning engine frequency should match the host's display refresh rate, not over-poll.

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
2. Implement `Trigger`: trigger element, event wiring (pointer, focus, keydown, click), `as_child` support, `aria-describedby`.
3. Implement `HiddenDescription`: always-rendered visually-hidden span with stable ID.
4. Implement `Positioner`: portal rendering, positioning engine integration, CSS custom property updates, z-index allocation.
5. Implement `Content`: `role="tooltip"`, `aria-hidden="true"`, interactive pointer events, `data-ars-state`.
6. Implement `Arrow`: arrow positioning within the positioner.
7. Wire Presence composition for lazy mount / unmount on exit.
8. Wire scroll listener for `close_on_scroll`.
9. Wire touch auto-hide timer.
10. Implement `Group` provider and warmup/cooldown integration.
11. Verify Escape `stop_propagation` behavior inside nested overlays.
12. Verify cleanup ordering: timers, listeners, positioning, portal, z-index, group registration.
13. Validate hover and focus behavior on Desktop and Mobile targets.

## 18. Anti-Patterns

- Do not omit the HiddenDescription span or conditionally render it based on open state; it must always be in the DOM.
- Do not point `aria-describedby` at the Content element; it must point at the HiddenDescription span.
- Do not start timers during SSR.
- Do not attach scroll listeners during SSR.
- Do not render the Positioner inline with the trigger; it must render into the portal root.
- Do not hardcode z-index values; use the `ZIndexAllocator`.
- Do not keep a local `is_open` signal separate from the machine state.
- Do not skip `event.stop_propagation()` when `api.on_trigger_keydown()` returns `true` for Escape.
- Do not use raw `setTimeout` directly; use the platform effects system for timer management.
- Do not ignore the minimum 200ms close delay enforcement for interactive tooltips.
- Do not allow multiple tooltips to be open simultaneously within the same `Group`.
- Do not omit `aria-hidden="true"` on the Content element.
- Do not assume browser-identical pointer dispatch on Desktop or Mobile targets; validate hover semantics per platform.

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
- Consumers must not assume Desktop and Mobile pointer behavior is identical to web browser behavior.

## 20. Platform Support Matrix

| Capability / behavior                     | Web          | Desktop        | Mobile        | SSR            | Notes                                               |
| ----------------------------------------- | ------------ | -------------- | ------------- | -------------- | --------------------------------------------------- |
| trigger rendering with `aria-describedby` | full support | full support   | full support  | full support   | trigger and HiddenDescription render on all targets |
| HiddenDescription span                    | full support | full support   | full support  | full support   | always rendered for screen reader access            |
| positioned overlay content                | full support | full support   | full support  | client-only    | depends on portal and positioning engine            |
| open/close delay timers                   | full support | full support   | full support  | client-only    | platform effects system                             |
| touch auto-hide timer                     | full support | not applicable | full support  | client-only    | touch detection is web/mobile only                  |
| scroll listener (`close_on_scroll`)       | full support | fallback path  | fallback path | client-only    | Desktop/Mobile scroll event dispatch may differ     |
| `Group` warmup/cooldown                   | full support | full support   | full support  | client-only    | timestamp-based; works on all runtime targets       |
| z-index allocation                        | full support | full support   | full support  | SSR-safe empty | allocator context may provide fallback              |
| positioning engine CSS custom properties  | full support | full support   | full support  | client-only    | computed after mount on all targets                 |
| Escape dismiss with `stop_propagation`    | full support | full support   | fallback path | client-only    | Mobile may lack physical Escape key                 |
| Presence animation lifecycle              | full support | full support   | full support  | SSR-safe empty | animations are client-only                          |
| hover pointer behavior                    | full support | fallback path  | fallback path | not applicable | Desktop/Mobile pointer retargeting may differ       |

## 21. Debug Diagnostics and Production Policy

| Condition                                              | Debug build behavior | Production behavior | Notes                                      |
| ------------------------------------------------------ | -------------------- | ------------------- | ------------------------------------------ |
| trigger element missing after mount                    | fail fast            | fail fast           | positioning cannot function without anchor |
| positioner element missing after mount                 | fail fast            | fail fast           | CSS custom properties cannot be set        |
| `ArsProvider` context missing                          | debug warning        | degrade gracefully  | falls back to inline rendering             |
| `ZIndexAllocator` context missing                      | debug warning        | degrade gracefully  | uses fallback z-index                      |
| `Group` context missing                                | no-op                | no-op               | independent tooltip operation is valid     |
| positioning engine returns error                       | debug warning        | degrade gracefully  | render at default position                 |
| `close_on_scroll` but scroll API unavailable on target | debug warning        | warn and ignore     | scroll dismissal becomes ineffective       |
| touch auto-hide timer clamped below 5000ms             | debug warning        | degrade gracefully  | clamp to 5000ms silently in production     |
| interactive tooltip close delay below 200ms            | debug warning        | degrade gracefully  | machine enforces 200ms minimum             |
| Desktop/Mobile hover dispatch differs from web         | debug warning        | degrade gracefully  | validate against actual target runtime     |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                  | Reused by                                 | Notes                                    |
| -------------------------- | ----------- | ------------------------------------------------------------------------------- | ----------------------------------------- | ---------------------------------------- |
| positioning helper         | required    | compute placement, set CSS custom properties on positioner, handle arrow offset | Tooltip, Popover, HoverCard, Tour         | `ars-dom` positioning engine integration |
| portal helper              | required    | render positioner/content/arrow into portal root                                | Tooltip, Popover, HoverCard, Dialog, Tour | via `ArsProvider`                        |
| z-index helper             | required    | allocate and release z-index for the overlay layer                              | Tooltip, Popover, HoverCard, Dialog, Tour | via `ZIndexAllocator`                    |
| timer helper               | required    | manage open delay, close delay, touch auto-hide timers via platform effects     | Tooltip, HoverCard, Toast                 | cleanup must cancel all pending timers   |
| presence helper            | required    | gate mount/unmount of positioner subtree for animation                          | Tooltip, Popover, HoverCard, Dialog       | `lazy_mount` and `unmount_on_exit`       |
| merge helper               | recommended | merge core attrs with adapter and consumer attrs                                | all overlay components                    | `class`/`style` merge additively         |
| context publication helper | recommended | standardize `use_context_provider` pattern for compound components              | Tooltip, Popover, Dialog                  | consistent context shape                 |
| platform capability helper | recommended | normalize hover, scroll, and timer assumptions for Web, Desktop, and Mobile     | Tooltip, HoverCard, Dismissable           | surface target-specific caveats          |

## 23. Framework-Specific Behavior

Dioxus uses element IDs or `use_hook`-based element references for trigger, positioner, and content. Event handlers are attached via `onpointerenter`, `onpointerleave`, `onfocus`, `onblur`, `onkeydown`, `onclick` in `rsx!`. `use_drop` handles timer cancellation, listener removal, positioning unsubscription, z-index release, and group deregistration. Controlled open state is synced via `use_effect` watching the `Signal<bool>`, dispatching `Event::Open` or `Event::Close` with a previous-value guard stored in a `Signal<Option<bool>>` to avoid re-entrance.

On Dioxus Desktop, hover behavior relies on the webview's pointer dispatch, which may differ from a standalone browser in edge cases around window boundaries and focus transitions. The adapter must validate hover enter/leave semantics on the actual Desktop target rather than assuming browser-identical dispatch.

On Dioxus Mobile, touch interactions are the primary trigger. Hover events may not fire at all on purely touch-based devices; the adapter must ensure the tooltip opens via focus or tap-triggered pointer events on these targets.

Scroll listeners on Desktop and Mobile may need platform-specific attachment points if the standard `document.addEventListener("scroll", ...)` path is not available in the webview.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[component]
pub fn Tooltip(props: TooltipProps) -> Element {
    let core_props = tooltip::Props {
        open: props.open.map(|s| *s.read()),
        default_open: props.default_open,
        open_delay_ms: props.open_delay_ms,
        close_delay_ms: props.close_delay_ms,
        disabled: props.disabled,
        interactive: props.interactive,
        on_open_change: props.on_open_change.as_ref().map(|h| {
            let h = h.clone();
            Callback::new(move |open| h.call(open))
        }),
        ..Default::default()
    };

    let machine = use_machine::<tooltip::Machine>(core_props);
    let is_open = machine.derive(|api| api.is_open());

    // Controlled open sync (deferred effect)
    if let Some(open_sig) = props.open {
        let send = machine.send;
        let mut prev_open: Signal<Option<bool>> = use_signal(|| None);
        use_effect(move || {
            let new_open = *open_sig.read();
            let prev = prev_open.read().clone();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.call(tooltip::Event::Open);
                    } else {
                        send.call(tooltip::Event::Close);
                    }
                }
                *prev_open.write() = Some(new_open);
            }
        });
    }

    // Group integration
    if let Some(group) = try_use_context::<GroupContext>() {
        // Register open/close with group for warmup tracking
    }

    use_context_provider(|| Context {
        open: is_open.into(),
        send: machine.send,
        service: machine.service,
        context_version: machine.context_version,
        interactive: props.interactive,
    });

    let root_attrs = machine.derive(|api| api.root_attrs());

    rsx! {
        div { ..root_attrs,
            {props.children}
        }
    }
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tooltip::Trigger must be used within a Tooltip");

    let trigger_attrs = ctx.service.with(|s| s.api().trigger_attrs());

    rsx! {
        div {
            ..trigger_attrs,
            onpointerenter: move |_| ctx.send.call(tooltip::Event::PointerEnter),
            onpointerleave: move |_| ctx.send.call(tooltip::Event::PointerLeave),
            onfocus: move |_| ctx.send.call(tooltip::Event::Focus),
            onblur: move |_| ctx.send.call(tooltip::Event::Blur),
            onkeydown: move |ev: KeyboardEvent| {
                let handled = ctx.service.with(|s| {
                    s.api().on_trigger_keydown(&KeyboardEventData::from(&ev))
                });
                if handled {
                    ev.stop_propagation();
                }
            },
            onclick: move |_| ctx.send.call(tooltip::Event::CloseOnClick),
            {props.children}
        }
    }
}

#[component]
pub fn Positioner(props: PositionerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tooltip::Positioner must be used within a Tooltip");

    let positioner_attrs = ctx.service.with(|s| s.api().positioner_attrs());

    // Portal rendering + positioning engine integration (client-only)
    // Sets --ars-x, --ars-y, --ars-z-index, etc.

    if *ctx.open.read() {
        rsx! {
            // Rendered into portal via ArsProvider
            div { ..positioner_attrs,
                {props.children}
            }
        }
    } else {
        None
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tooltip::Content must be used within a Tooltip");

    let content_attrs = ctx.service.with(|s| s.api().content_attrs());

    rsx! {
        div {
            ..content_attrs,
            "aria-hidden": "true",
            onpointerenter: move |_| {
                if ctx.interactive {
                    ctx.send.call(tooltip::Event::ContentPointerEnter);
                }
            },
            onpointerleave: move |_| {
                if ctx.interactive {
                    ctx.send.call(tooltip::Event::ContentPointerLeave);
                }
            },
            {props.children}
        }
    }
}
```

The HiddenDescription span is rendered inside the Root, always present:

```rust
// Inside Tooltip, before or after children
rsx! {
    span {
        ..machine.derive(|api| api.hidden_description_attrs()),
        style: "position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0;",
        // Consumer provides tooltip text as children or via a slot prop
    }
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Tooltip
let core_props = build_core_props(adapter_props);
let machine = use_machine::<tooltip::Machine>(core_props);
let is_open = machine.derive(|api| api.is_open());

// Controlled open sync (client-only, deferred effect)
sync_controlled_open(open_signal, machine.send);  // use_effect + prev guard

// Group warmup integration (client-only)
let group = try_use_context::<GroupContext>();
integrate_tooltip_group(group, machine.send, &core_props.id);

// Provide compound context
use_context_provider(|| Context { ... });

// HiddenDescription: always rendered inside Root
render_hidden_description(machine);

// Trigger
let trigger_attrs = machine.derive(|api| api.trigger_attrs());
attach_trigger_events(machine.send);  // pointer, focus, keydown, click

// Positioner (client-only, portal, positioning engine)
use_effect(move || {
    if is_open.read() {
        let z = allocate_z_index();  // ZIndexAllocator
        let positioning_handle = start_positioning(trigger_id, positioner_id, positioning_opts);
        set_css_custom_properties(positioner_id, positioning_handle.output());

        if close_on_scroll {
            attach_scroll_listener(machine.send);  // web-only or platform-adapted
        }

        if is_touch_device() {
            start_touch_auto_hide_timer(touch_auto_hide_ms, machine.send);
        }
    }
});

// Content: role="tooltip", aria-hidden="true", interactive pointer events
let content_attrs = machine.derive(|api| api.content_attrs());
if interactive {
    attach_content_pointer_events(machine.send);
}

// Cleanup
use_drop(|| {
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
- No timer, listener, or positioning subscription may outlive the component; all MUST be cleaned up in `use_drop`.
- On Desktop and Mobile, hover and scroll behavior must be validated against the actual Dioxus target runtime rather than assuming browser-identical dispatch.

## 27. Accessibility and SSR Notes

- The HiddenDescription span ensures screen reader users (including touch-only VoiceOver/TalkBack) always hear the tooltip description when focusing the trigger, regardless of whether the visual tooltip is open.
- The Content element uses `aria-hidden="true"` because it is a visual duplicate; the accessible description is in the HiddenDescription span.
- On SSR (Dioxus fullstack), the trigger renders with `aria-describedby` pointing to the HiddenDescription span, and the span renders with its content. This gives screen readers immediate access on page load.
- The Positioner/Content/Arrow do not render during SSR; they appear after hydration when the tooltip is opened.
- On Desktop and Mobile (no SSR phase), all content is rendered client-side, but timer and positioning setup is still deferred until after mount.
- Escape dismissal is available on all platforms with keyboard support. On Mobile without a physical Escape key, the `DismissButton` pattern (when `interactive: true`) provides an alternative dismiss path for screen reader users.
- Touch device auto-hide (minimum 5000ms, default 20000ms) satisfies WCAG 2.2.1 by giving users sufficient reading time.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, structure, and behavior parity. All six parts rendered. 4-state machine fully wired. Timer lifecycle, Group warmup/cooldown, touch auto-hide, interactive WCAG 1.4.13 compliance, scroll dismissal, and Escape `stop_propagation` all covered.

Intentional deviations: none.

Traceability note: This adapter spec explicitly covers the following core adapter-owned concerns: portal rendering of positioned content, positioning engine CSS custom property updates, timer management via platform effects, HiddenDescription always-rendered span, `aria-describedby` wiring from trigger to hidden span (not content), `aria-hidden="true"` on Content, Escape `stop_propagation`, scroll listener lifecycle, touch auto-hide timer with minimum clamping, `Group` warmup/cooldown coordination, z-index allocation, Presence composition for lazy mount / unmount on exit, client-only boundary enforcement for all timer and listener behavior, and multi-platform validation for hover, scroll, and pointer dispatch on Desktop and Mobile targets.

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
- Desktop hover behavior matches documented containment expectations
- Mobile touch-only tooltip opening via tap/focus

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
| Desktop hover semantics           | callback order        | verify pointer enter/leave on Desktop webview target            |
| Mobile tap-to-open                | machine state         | verify tooltip opens via focus from tap on Mobile target        |

Cheap verification recipe:

1. Render a tooltip and assert the HiddenDescription span exists with the correct ID and that the trigger's `aria-describedby` references it, both before and after opening.
2. Open the tooltip via hover, verify Content appears with `role="tooltip"` and `aria-hidden="true"`, then press Escape and verify the tooltip closes and the Escape event does not propagate.
3. In a `Group`, open tooltip A, hover to tooltip B's trigger, and verify tooltip A closes and tooltip B opens immediately (warmup).
4. Unmount the component and verify no timers, scroll listeners, or positioning subscriptions remain active.
5. On Dioxus Desktop, repeat the hover test and verify pointer enter/leave dispatch matches the documented expectations. On Dioxus Mobile, repeat the open test via tap and verify tooltip opens via focus.

## 31. Implementation Checklist

- [ ] `Tooltip` initializes the machine and provides `Context` via `use_context_provider`.
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
- [ ] Controlled `open` signal syncs via `use_effect` with previous-value guard in `Signal<Option<bool>>`.
- [ ] `on_open_change` fires after all open/close transitions.
- [ ] `Group` integration: warmup check, single-open enforcement, close/open recording.
- [ ] Presence composition gates positioner subtree for `lazy_mount` and `unmount_on_exit`.
- [ ] All timers, listeners, positioning subscriptions, z-index allocations, and group registrations cleaned up in `use_drop`.
- [ ] SSR renders trigger and HiddenDescription only; positioner/content/arrow are client-only.
- [ ] Desktop hover behavior validated against actual Dioxus Desktop target.
- [ ] Mobile tap-to-open behavior validated against actual Dioxus Mobile target.
- [ ] Test scenarios from section 29 are covered.
