---
adapter: leptos
component: hover-card
category: overlay
source: components/overlay/hover-card.md
source_foundation: foundation/08-adapter-leptos.md
---

# HoverCard — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`HoverCard`](../../components/overlay/hover-card.md) behavior to Leptos 0.8.x. The adapter owns the compound component tree (`HoverCard`, `Trigger`, `Positioner`, `Content`, `Arrow`, `Title`, `DismissButton`), pointer and focus event wiring, timer-based open/close delay lifecycle, safe-area (hover bridge) management, portal rendering of positioned content, positioning engine integration with CSS custom properties, z-index allocation, and the `Presence`-based mount/unmount animation lifecycle.

HoverCard is like Popover but triggered by hover with a longer delay (700ms default). Unlike Tooltip, the content is **interactive** — users can Tab into it and interact with links and buttons inside the card. The adapter must keep the card open while the pointer traverses the safe-area triangle between trigger and content.

## 2. Public Adapter API

```rust
use leptos::prelude::*;

#[component]
pub fn HoverCard(
    #[prop(into)] id: String,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional, default = false)] default_open: bool,
    #[prop(optional, default = 700)] open_delay_ms: u32,
    #[prop(optional, default = 300)] close_delay_ms: u32,
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional, default = false)] lazy_mount: bool,
    #[prop(optional, default = false)] unmount_on_exit: bool,
    #[prop(optional)] messages: Option<hover_card::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView

#[component]
pub fn Trigger(
    children: Children,
) -> impl IntoView

#[component]
pub fn Positioner(
    children: Children,
) -> impl IntoView

#[component]
pub fn Content(
    children: Children,
) -> impl IntoView

#[component]
pub fn Arrow() -> impl IntoView

#[component]
pub fn Title(
    children: Children,
) -> impl IntoView

#[component]
pub fn DismissButton(
    children: Children,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with `hover_card::Props`. `open` is `Option<Signal<bool>>` for controlled mode; all other props map directly.
- Event parity: `TriggerPointerEnter`, `TriggerPointerLeave`, `TriggerFocus`, `TriggerBlur`, `TriggerKeyDown`, `ContentPointerEnter`, `ContentPointerLeave`, `OpenTimerFired`, `CloseTimerFired`, `CloseOnEscape`, and `TitleMount` are all wired through adapter event handlers and timer effects.
- Structure parity: all seven core parts (Root, Trigger, Positioner, Content, Arrow, Title, DismissButton) have corresponding adapter components.
- Safe area: the hover bridge triangle algorithm from the core spec is implemented as a client-only `pointermove` document listener managed by the adapter.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                              | Ownership      | Attr source                  | Notes                                                          |
| --------------------- | --------- | ----------------------------------------------------- | -------------- | ---------------------------- | -------------------------------------------------------------- |
| Root                  | required  | `<div>` wrapper around all children                   | adapter-owned  | `api.root_attrs()`           | Provides context for all descendant parts.                     |
| Trigger               | required  | consumer-provided element or `<a>`/`<button>` wrapper | consumer-owned | `api.trigger_attrs()`        | Receives pointer/focus/keyboard event handlers.                |
| Positioner            | required  | `<div>` inside portal root with CSS custom properties | adapter-owned  | `api.positioner_attrs()`     | Positioned by the `ars-dom` positioning engine.                |
| Content               | required  | `<div>` with `role="dialog"` inside positioner        | adapter-owned  | `api.content_attrs()`        | Interactive content; receives pointer enter/leave handlers.    |
| Arrow                 | optional  | `<div>` inside positioner, positioned by engine       | adapter-owned  | `api.arrow_attrs()`          | Decorative arrow pointing toward trigger.                      |
| Title                 | optional  | consumer-provided heading element inside content      | consumer-owned | `api.title_attrs()`          | Fires `TitleMount` on mount; wires `aria-labelledby`.          |
| DismissButton         | optional  | visually hidden `<button>` inside content             | adapter-owned  | `api.dismiss_button_attrs()` | Screen reader close mechanism; sends `CloseOnEscape` on click. |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                   | Adapter-owned attrs                                                              | Consumer attrs              | Merge order                                                                   | Ownership notes                                               |
| ------------- | ---------------------------- | -------------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Root          | `api.root_attrs()`           | structural `data-ars-scope`, `data-ars-state`                                    | consumer `class`/`style`    | core scope and state attrs win; `class`/`style` merge additively              | adapter-owned wrapper                                         |
| Trigger       | `api.trigger_attrs()`        | `on:pointerenter`, `on:pointerleave`, `on:focus`, `on:blur`, `on:keydown`        | consumer `class`/`style`    | core ARIA attrs (`aria-expanded`, `aria-controls`) win; handlers compose      | consumer-owned element with adapter-composed handlers         |
| Positioner    | `api.positioner_attrs()`     | CSS custom properties (`--ars-x`, `--ars-y`, etc.), positioning `style`          | consumer `class` if exposed | core part attrs win; adapter positioning style is authoritative               | adapter-owned; rendered in portal root                        |
| Content       | `api.content_attrs()`        | `on:pointerenter`, `on:pointerleave`, `on:keydown`, `role="dialog"`, ARIA wiring | consumer `class`/`style`    | core `role`, ARIA, and `id` attrs win; pointer handlers are adapter-exclusive | adapter-owned structural node with consumer children          |
| Arrow         | `api.arrow_attrs()`          | positioning style from engine                                                    | consumer `class`/`style`    | core part attrs win; positioning style is authoritative                       | adapter-owned decorative node                                 |
| Title         | `api.title_attrs()`          | `id` for `aria-labelledby` wiring                                                | consumer heading content    | core `id` wins                                                                | consumer-owned content; adapter wires ID and fires TitleMount |
| DismissButton | `api.dismiss_button_attrs()` | visually-hidden styles, `on:click` for close, `aria-label` from messages         | none                        | adapter-exclusive                                                             | adapter-owned; not consumer-customizable                      |

- Consumers must not override `aria-expanded`, `aria-controls`, or `role` on the trigger or content.
- Positioning CSS custom properties on the positioner are authoritative and must not be overridden by consumer styles.
- The `id` attribute on Title is machine-generated and must not be changed by the consumer.

## 6. Composition / Context Contract

`HoverCard` provides a `Context` via `provide_context`. All descendant parts consume it via `use_context::<Context>().expect("... must be used inside HoverCard")`.

```rust
#[derive(Clone, Copy)]
struct Context {
    send: Callback<hover_card::Event>,
    is_open: Memo<bool>,
    trigger_ref: NodeRef<html::Div>,
    content_ref: NodeRef<html::Div>,
    trigger_id: Memo<String>,
    content_id: Memo<String>,
    title_id: Memo<String>,
    has_title: Memo<bool>,
    root_attrs: Memo<AttrMap>,
    trigger_attrs: Memo<AttrMap>,
    positioner_attrs: Memo<AttrMap>,
    content_attrs: Memo<AttrMap>,
    arrow_attrs: Memo<AttrMap>,
    title_attrs: Memo<AttrMap>,
    dismiss_button_attrs: Memo<AttrMap>,
    positioning: PositioningOptions,
    lazy_mount: bool,
    unmount_on_exit: bool,
}
```

Composing overlays:

- HoverCard consumes `ArsProvider` context for portal root resolution.
- HoverCard consumes `ZIndexAllocator` context for z-index allocation on the positioner.
- HoverCard content may optionally compose with `Presence` for enter/exit animations.
- HoverCard does NOT compose with `FocusScope` in contain mode — content is non-modal. However, `FocusScope::popover()` preset (`contain=false`, `restore_focus=true`) is used so focus returns to trigger on close.
- HoverCard does NOT compose with `Dismissable` for outside-click — it closes on pointer leave, not outside interaction.

## 7. Prop Sync and Event Mapping

| Adapter prop     | Mode       | Sync trigger             | Machine event / update path        | Visible effect                           | Notes                                               |
| ---------------- | ---------- | ------------------------ | ---------------------------------- | ---------------------------------------- | --------------------------------------------------- |
| `open`           | controlled | `Signal` change          | `PropSync` open/close event        | opens or closes the hover card           | deferred `Effect` watches `Signal<bool>` changes    |
| `disabled`       | static     | render time only         | guards all transitions in machine  | disables all hover/focus/keyboard open   | machine returns `None` for all events when disabled |
| `open_delay_ms`  | static     | init and context rebuild | stored in `Context.open_delay_ms`  | adjusts timer duration for open pending  | not reactive after mount                            |
| `close_delay_ms` | static     | init and context rebuild | stored in `Context.close_delay_ms` | adjusts timer duration for close pending | not reactive after mount                            |
| `positioning`    | static     | init and positioning run | passed to positioning engine       | affects positioner CSS custom properties | re-runs positioning when content opens              |

| UI event                    | Preconditions         | Machine event                 | Ordering notes                                       | Notes                                                    |
| --------------------------- | --------------------- | ----------------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| pointer enter trigger       | not disabled          | `TriggerPointerEnter`         | starts open delay timer                              | safe area listener not yet active                        |
| pointer leave trigger       | open pending or open  | `TriggerPointerLeave`         | starts safe area check, then close delay if outside  | safe area suspends close delay while pointer in triangle |
| pointer enter content       | close pending or open | `ContentPointerEnter`         | cancels close delay timer                            | keeps card open while pointer is in content              |
| pointer leave content       | open                  | `ContentPointerLeave`         | starts close delay timer                             | allows pointer to return to trigger within delay         |
| trigger focus               | not disabled, closed  | `TriggerFocus`                | starts open delay timer                              | keyboard accessibility path                              |
| trigger blur                | open pending or open  | `TriggerBlur`                 | starts close delay; suppressed if hover still active | focus and hover are independent open channels            |
| trigger keydown Enter/Space | not disabled          | `TriggerKeyDown(Enter/Space)` | opens immediately, bypasses delay                    | keyboard accessibility enhancement over references       |
| trigger keydown Escape      | open or close pending | `CloseOnEscape`               | closes immediately, no delay                         | standard overlay dismiss pattern                         |
| content keydown Escape      | open                  | `CloseOnEscape`               | closes immediately from within content               | Escape works from inside interactive content             |
| title mount                 | any state             | `TitleMount`                  | sets `has_title = true` in context                   | enables `aria-labelledby` on content                     |

## 8. Registration and Cleanup Contract

| Registered entity         | Registration trigger                    | Identity key      | Cleanup trigger                         | Cleanup action                                         | Notes                                  |
| ------------------------- | --------------------------------------- | ----------------- | --------------------------------------- | ------------------------------------------------------ | -------------------------------------- |
| open delay timer          | `TriggerPointerEnter` or `TriggerFocus` | machine effect    | timer fires, cancel, or close           | `platform.clear_timeout(handle)`                       | client-only; managed by machine effect |
| close delay timer         | pointer/focus leave events              | machine effect    | timer fires, cancel, or re-enter        | `platform.clear_timeout(handle)`                       | client-only; managed by machine effect |
| safe area `pointermove`   | `TriggerPointerLeave` while open        | adapter-local     | pointer enters content or exits polygon | `remove_event_listener` on document                    | client-only; adapter-owned cleanup     |
| positioning engine        | content opens (first open or re-open)   | adapter-local     | content closes or component unmount     | detach positioning observer / cleanup                  | client-only; re-runs on open           |
| z-index allocation        | content opens                           | allocator context | content closes or component unmount     | release z-index back to allocator                      | via `ZIndexAllocator` context          |
| portal mount              | content opens (respecting lazy_mount)   | portal context    | unmount_on_exit close or unmount        | remove portal content                                  | via `ArsProvider` portal root          |
| `on_cleanup` registration | component mount                         | component scope   | component unmount                       | cancel all timers, remove safe-area listener, teardown | ensures no leaked timers or listeners  |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner      | Node availability                  | Composition rule                                                     | Notes                                                   |
| ------------------ | ------------- | -------------- | ---------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------- |
| Trigger element    | yes           | adapter-owned  | required after mount               | trigger ref is the positioning anchor; passed to positioning engine  | `NodeRef<html::Div>` or generic element ref             |
| Content element    | yes           | adapter-owned  | client-only                        | content ref for safe-area polygon computation and positioning target | needed for pointer-in-content detection and positioning |
| Positioner element | yes           | adapter-owned  | client-only                        | receives CSS custom properties from positioning engine               | structural handle for positioning updates               |
| Arrow element      | optional      | adapter-owned  | client-only                        | optional ref for arrow positioning within the positioning engine     | only needed if arrow is rendered                        |
| Root element       | no            | adapter-owned  | always structural, handle optional | no composition requirement                                           | structural wrapper; ref not required for behavior       |
| Title element      | no            | consumer-owned | always structural, handle optional | fires `TitleMount` via `Effect` on mount, no ref needed              | ID-based wiring via `aria-labelledby`                   |
| DismissButton      | no            | adapter-owned  | always structural, handle optional | no composition requirement                                           | semantics do not depend on stored refs                  |

## 10. State Machine Boundary Rules

- machine-owned state: `State` (Closed/OpenPending/Open/ClosePending), `Context.open`, `Context.hover_active`, `Context.focus_active`, `Context.has_title`, and all timer-based transitions.
- adapter-local derived bookkeeping: safe-area `pointermove` listener handle, positioning engine handle, z-index allocation handle, portal mount state, `lazy_mount`/`unmount_on_exit` rendering decisions.
- forbidden local mirrors: do not keep a local `is_open` boolean that can diverge from `machine.derive(|api| api.is_open())`. Do not keep a local timer that duplicates the machine's `PendingEffect`-managed timers.
- allowed snapshot-read contexts: event handler closures, mount effects, cleanup closures, and positioning callbacks may snapshot-read from the machine API.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape | Timing                                 | Cancelable? | Notes                                                             |
| ---------------- | ------------------------ | ------------- | -------------------------------------- | ----------- | ----------------------------------------------------------------- |
| `on_open_change` | machine-derived snapshot | `bool`        | after state transitions to Open/Closed | no          | fires when open state changes, not on intermediate pending states |

## 12. Failure and Degradation Rules

| Condition                                      | Policy             | Notes                                                                          |
| ---------------------------------------------- | ------------------ | ------------------------------------------------------------------------------ |
| trigger ref missing after mount                | fail fast          | positioning engine cannot anchor without the trigger element                   |
| content ref missing when open                  | fail fast          | safe-area computation and positioning require the content node                 |
| portal root unavailable (ArsProvider missing)  | degrade gracefully | render content inline as a fallback; log debug warning                         |
| z-index allocator unavailable                  | degrade gracefully | use a reasonable default z-index; log debug warning                            |
| positioning engine fails to compute            | degrade gracefully | content renders at default position; visible but potentially misplaced         |
| timer APIs absent during SSR                   | no-op              | trigger renders with ARIA attrs; content waits for client mount                |
| safe-area pointermove listener fails to attach | warn and ignore    | close delay runs normally without safe-area bridge; pointer may not reach card |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?  | DOM order must match registration order? | SSR/hydration stability                   | Notes                                              |
| -------------------------------- | ---------------- | -------------------- | ---------------------------------------- | ----------------------------------------- | -------------------------------------------------- |
| HoverCard instance               | instance-derived | yes (multiple cards) | not applicable                           | trigger and root structure must be stable | each instance has unique machine-generated IDs     |
| trigger-to-content ARIA wiring   | data-derived     | not applicable       | not applicable                           | IDs must match across hydration           | `aria-controls` on trigger references `content_id` |
| title-to-content ARIA wiring     | data-derived     | not applicable       | not applicable                           | IDs must match across hydration           | `aria-labelledby` on content references `title_id` |

## 14. SSR and Client Boundary Rules

- SSR renders the Root wrapper and Trigger with all ARIA attributes (`aria-expanded="false"`, scope/part data attrs).
- SSR does NOT render Positioner, Content, Arrow, or DismissButton unless `default_open` is true. When `default_open` is true, the structure renders but without positioning CSS custom properties (those are computed client-side).
- All timer-based behavior (open delay, close delay) is client-only. No `PendingEffect` timers run during SSR.
- Safe-area `pointermove` listeners are client-only.
- Positioning engine runs client-only after mount.
- Z-index allocation is client-only.
- Portal rendering is client-only; SSR renders content inline within the component tree if `default_open` is true.
- `on_open_change` callback must not fire during SSR.
- The trigger's `aria-expanded` and `aria-controls` attributes are set during SSR based on `default_open` to ensure hydration stability.
- Title's `id` attribute is set during SSR if the title is rendered, ensuring `aria-labelledby` is valid on hydration.

## 15. Performance Constraints

- Timer effects must not churn on every render; they are managed by the machine's `PendingEffect` system which creates and cleans up timers only on state transitions.
- Safe-area `pointermove` listener attaches only on `TriggerPointerLeave` when the card is open, and detaches when the pointer enters content, exits the polygon, or the card closes. It must not remain attached when the card is closed.
- Positioning engine should run once on open and observe anchor/content changes, not on every render tick.
- `derive(...)` calls cache their results via `Memo`; attr maps should not be recomputed unless the underlying machine state or context changes.
- Portal mount/unmount respects `lazy_mount` (first open) and `unmount_on_exit` (every close) to avoid unnecessary DOM operations.
- Multiple HoverCard instances must not share timer or safe-area state.

## 16. Implementation Dependencies

| Dependency           | Required?   | Dependency type         | Why it must exist first                                                              | Notes                                                    |
| -------------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| `ars-provider`       | required    | context contract        | provides portal root for rendering content outside component tree                    | content renders into `ars-portal-root`                   |
| `z-index-allocator`  | required    | context contract        | provides dynamic z-index for positioner to avoid hardcoded values                    | allocated on open, released on close                     |
| `presence`           | recommended | composition contract    | manages enter/exit animation lifecycle for content mount/unmount                     | optional but expected for polished UX                    |
| `focus-scope`        | recommended | behavioral prerequisite | `FocusScope::popover()` preset restores focus to trigger on close                    | non-modal: `contain=false`, `restore_focus=true`         |
| `dismissable`        | not needed  | conceptual              | HoverCard closes on pointer leave, not outside interaction; Dismissable not composed | Escape handling is wired directly through machine events |
| `positioning engine` | required    | shared helper           | computes placement, CSS custom properties, and arrow positioning                     | `ars-dom` floating positioning system                    |

## 17. Recommended Implementation Sequence

1. Wire `HoverCard` with `use_machine::<hover_card::Machine>()`, context provision, and controlled `open` signal synchronization.
2. Wire `Trigger` with pointer enter/leave, focus/blur, and keydown event handlers dispatching machine events.
3. Wire `Positioner` with portal rendering via `ArsProvider`, positioning engine integration, CSS custom property injection, and z-index allocation.
4. Wire `Content` with pointer enter/leave handlers, `role="dialog"`, and ARIA wiring (`aria-labelledby`/`aria-label`).
5. Wire `Arrow` with arrow positioning from the positioning engine.
6. Wire `Title` with `TitleMount` dispatch on mount and `id` for `aria-labelledby`.
7. Wire `DismissButton` with visually-hidden close button dispatching `CloseOnEscape`.
8. Implement safe-area (hover bridge) triangle computation and `pointermove` document listener.
9. Integrate `Presence` for content enter/exit animations.
10. Verify `lazy_mount` and `unmount_on_exit` behavior.
11. Verify cleanup: all timers, safe-area listeners, positioning observers, and z-index allocations are released on unmount.

## 18. Anti-Patterns

- Do not attach safe-area `pointermove` listeners during SSR.
- Do not start open/close delay timers during SSR.
- Do not keep a local `is_open` flag that can diverge from the machine state.
- Do not hardcode z-index values; use `ZIndexAllocator` context.
- Do not render content inline instead of into the portal root (unless portal is unavailable as a fallback).
- Do not use `FocusScope` in contain mode — HoverCard content is non-modal.
- Do not compose `Dismissable` for outside-click behavior — HoverCard uses pointer leave, not outside interaction.
- Do not leave safe-area `pointermove` listeners attached when the card is closed.
- Do not fire `on_open_change` for intermediate `OpenPending`/`ClosePending` states — only for final `Open`/`Closed` transitions.
- Do not duplicate the machine's timer management with adapter-local timers.
- Do not omit the `role="dialog"` attribute on content.
- Do not set `aria-labelledby` when no Title element is rendered — fall back to `aria-label` from Messages.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the hover card opens after 700ms of hovering (or the configured `open_delay_ms`) and closes 300ms after the pointer leaves (or the configured `close_delay_ms`).
- Consumers may assume the card stays open while the pointer moves through the safe-area triangle from trigger to content.
- Consumers may assume content is interactive — Tab enters the card, links and buttons work.
- Consumers may assume `on_open_change` fires only for final open/close transitions, not intermediate pending states.
- Consumers may assume ARIA attributes are correctly wired: `aria-expanded` on trigger, `aria-controls` when open, `role="dialog"` on content, `aria-labelledby` when Title is present.
- Consumers may assume positioning CSS custom properties are set on the positioner after each positioning update.
- Consumers must not assume content renders inline — it renders in a portal root.
- Consumers must not assume the card opens on click — it opens on hover/focus with a delay, or immediately on Enter/Space.
- Consumers must not assume the card is modal — background content remains interactive.
- Consumers must not assume `aria-labelledby` is set when no `Title` is rendered.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR            | Notes                                                                 |
| ----------------------------------------- | -------------- | -------------- | --------------------------------------------------------------------- |
| trigger rendering with ARIA attrs         | full support   | full support   | `aria-expanded`, `aria-controls`, scope/part data attrs render on SSR |
| content rendering (portal)                | full support   | SSR-safe empty | content waits for client mount; inline fallback if `default_open`     |
| open/close delay timers                   | full support   | client-only    | `PendingEffect` timers are client-only                                |
| safe-area hover bridge                    | full support   | client-only    | document `pointermove` listener is client-only                        |
| positioning engine                        | full support   | client-only    | CSS custom properties computed after mount                            |
| z-index allocation                        | full support   | client-only    | allocated on open, released on close                                  |
| `on_open_change` callback                 | full support   | client-only    | must not fire during SSR                                              |
| keyboard interaction (Enter/Space/Escape) | full support   | client-only    | keydown handlers are client-only                                      |

## 21. Debug Diagnostics and Production Policy

| Condition                            | Debug build behavior | Production behavior | Notes                                               |
| ------------------------------------ | -------------------- | ------------------- | --------------------------------------------------- |
| trigger ref missing after mount      | fail fast            | fail fast           | positioning cannot work without an anchor element   |
| content ref missing when card opens  | fail fast            | fail fast           | safe-area and positioning require the content node  |
| `ArsProvider` context missing        | debug warning        | degrade gracefully  | fall back to inline rendering; log warning in debug |
| `ZIndexAllocator` context missing    | debug warning        | degrade gracefully  | use default z-index; log warning in debug           |
| positioning engine returns no result | debug warning        | degrade gracefully  | content renders at default position                 |
| safe-area listener fails to attach   | debug warning        | warn and ignore     | close delay runs without safe-area bridge           |
| `Trigger` used outside `HoverCard`   | fail fast            | fail fast           | context `.expect()` panics with clear message       |
| `Content` used outside `HoverCard`   | fail fast            | fail fast           | context `.expect()` panics with clear message       |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required?   | Responsibility                                                                     | Reused by                          | Notes                                                              |
| ------------------ | ----------- | ---------------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------ |
| portal helper      | required    | renders content into `ars-portal-root` via `ArsProvider`                           | popover, tooltip, hover-card, tour | shared portal rendering infrastructure                             |
| positioning helper | required    | runs `ars-dom` positioning engine, sets CSS custom properties on positioner        | popover, tooltip, hover-card, tour | includes arrow positioning when arrow ref is provided              |
| z-index helper     | required    | allocates and releases z-index from `ZIndexAllocator` context                      | all overlays                       | allocate on open, release on close/unmount                         |
| timer helper       | recommended | client-only timer management with cleanup integration                              | tooltip, hover-card, toast         | machine `PendingEffect` manages timers; adapter ensures cleanup    |
| safe-area helper   | required    | computes safe triangle between trigger and content, manages `pointermove` listener | hover-card, tooltip (interactive)  | reusable for any hover-triggered overlay with a gap                |
| merge helper       | recommended | merges core `AttrMap` with adapter-owned attrs and consumer attrs                  | all components                     | ensures correct precedence: core wins, then adapter, then consumer |

## 23. Framework-Specific Behavior

Leptos uses `on_cleanup` for timer and listener teardown. Safe-area `pointermove` listeners attach via `web_sys::Document::add_event_listener_with_callback` and must be removed in the cleanup closure. Controlled `open` synchronization uses a deferred `Effect` watching `Signal<bool>` changes — this is an intentional exception to body-level sync because open/close dispatches machine events. Optional `ArsProvider` context is read via `use_context::<ArsContext>()`. The adapter is web-only; there is no Desktop or Mobile target consideration.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn HoverCard(
    #[prop(into)] id: String,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional, default = false)] default_open: bool,
    #[prop(optional, default = 700)] open_delay_ms: u32,
    #[prop(optional, default = 300)] close_delay_ms: u32,
    #[prop(optional, default = false)] disabled: bool,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional, default = false)] lazy_mount: bool,
    #[prop(optional, default = false)] unmount_on_exit: bool,
    #[prop(optional)] messages: Option<hover_card::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView {
    let props = hover_card::Props {
        id,
        open: open.map(|s| s.get_untracked()),
        default_open,
        open_delay_ms,
        close_delay_ms,
        disabled,
        positioning: positioning.unwrap_or_default(),
        on_open_change,
        lazy_mount,
        unmount_on_exit,
        messages,
        locale,
    };

    let machine = use_machine::<hover_card::Machine>(props);

    // Controlled open synchronization
    if let Some(open_sig) = open {
        let send = machine.send;
        let mut prev_open: RwSignal<Option<bool>> = RwSignal::new(None);
        Effect::new(move |_| {
            let new_open = open_sig.get();
            let prev = prev_open.get();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.run(hover_card::Event::TriggerPointerEnter);
                    } else {
                        send.run(hover_card::Event::CloseOnEscape);
                    }
                }
                prev_open.set(Some(new_open));
            }
        });
    }

    let is_open = machine.derive(|api| api.is_open());
    let root_attrs = machine.derive(|api| api.root_attrs());
    let trigger_attrs = machine.derive(|api| api.trigger_attrs());
    let positioner_attrs = machine.derive(|api| api.positioner_attrs());
    let content_attrs = machine.derive(|api| api.content_attrs());
    let arrow_attrs = machine.derive(|api| api.arrow_attrs());
    let title_attrs = machine.derive(|api| api.title_attrs());
    let dismiss_button_attrs = machine.derive(|api| api.dismiss_button_attrs());

    let trigger_ref = NodeRef::<html::Div>::new();
    let content_ref = NodeRef::<html::Div>::new();
    let positioner_ref = NodeRef::<html::Div>::new();

    provide_context(Context {
        send: machine.send,
        is_open,
        trigger_ref,
        content_ref,
        trigger_attrs,
        positioner_attrs,
        content_attrs,
        arrow_attrs,
        title_attrs,
        dismiss_button_attrs,
        positioner_ref,
        positioning: positioning.unwrap_or_default(),
        lazy_mount,
        unmount_on_exit,
    });

    view! {
        <div {..root_attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn Trigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("hover_card::Trigger must be used inside HoverCard");

    view! {
        <div
            node_ref=ctx.trigger_ref
            {..ctx.trigger_attrs.get()}
            on:pointerenter=move |_| ctx.send.run(hover_card::Event::TriggerPointerEnter)
            on:pointerleave=move |_| ctx.send.run(hover_card::Event::TriggerPointerLeave)
            on:focus=move |_| ctx.send.run(hover_card::Event::TriggerFocus)
            on:blur=move |_| ctx.send.run(hover_card::Event::TriggerBlur)
            on:keydown=move |ev| {
                let key = KeyboardKey::from_key_str(&ev.key());
                match key {
                    KeyboardKey::Enter | KeyboardKey::Space => {
                        ctx.send.run(hover_card::Event::TriggerKeyDown(key));
                    }
                    KeyboardKey::Escape => {
                        ctx.send.run(hover_card::Event::CloseOnEscape);
                    }
                    _ => {}
                }
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn Positioner(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("hover_card::Positioner must be used inside HoverCard");

    // Positioning engine runs client-only after mount when open.
    // CSS custom properties (--ars-x, --ars-y, --ars-z-index, etc.)
    // are set on the positioner element by the positioning helper.

    view! {
        <Show when=move || ctx.is_open.get()>
            <Portal>
                <div
                    node_ref=ctx.positioner_ref
                    {..ctx.positioner_attrs.get()}
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
        .expect("hover_card::Content must be used inside HoverCard");

    view! {
        <div
            node_ref=ctx.content_ref
            {..ctx.content_attrs.get()}
            on:pointerenter=move |_| ctx.send.run(hover_card::Event::ContentPointerEnter)
            on:pointerleave=move |_| ctx.send.run(hover_card::Event::ContentPointerLeave)
            on:keydown=move |ev| {
                if KeyboardKey::from_key_str(&ev.key()) == KeyboardKey::Escape {
                    ctx.send.run(hover_card::Event::CloseOnEscape);
                }
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn Arrow() -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("hover_card::Arrow must be used inside HoverCard");

    view! {
        <div {..ctx.arrow_attrs.get()} />
    }
}

#[component]
pub fn Title(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("hover_card::Title must be used inside HoverCard");

    // Fire TitleMount on mount
    Effect::new(move |_| {
        ctx.send.run(hover_card::Event::TitleMount);
    });

    view! {
        <div {..ctx.title_attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn DismissButton(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("hover_card::DismissButton must be used inside HoverCard");

    view! {
        <button
            {..ctx.dismiss_button_attrs.get()}
            on:click=move |_| ctx.send.run(hover_card::Event::CloseOnEscape)
        >
            {children()}
        </button>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
// 1. Machine setup
let machine = use_machine::<hover_card::Machine>(props);
let send = machine.send;

// 2. Controlled open sync (deferred Effect, not body-level)
if let Some(open_sig) = controlled_open {
    sync_controlled_open(open_sig, send);
}

// 3. Derive reactive snapshots
let is_open = machine.derive(|api| api.is_open());
let root_attrs = machine.derive(|api| api.root_attrs());
let trigger_attrs = machine.derive(|api| api.trigger_attrs());
let positioner_attrs = machine.derive(|api| api.positioner_attrs());
let content_attrs = machine.derive(|api| api.content_attrs());
// ... remaining attr derivations

// 4. Refs
let trigger_ref = NodeRef::<html::Div>::new();
let content_ref = NodeRef::<html::Div>::new();
let positioner_ref = NodeRef::<html::Div>::new();

// 5. Context provision
provide_context(Context { send, is_open, trigger_ref, content_ref, /* ... */ });

// 6. Safe-area management (client-only)
let safe_area_handle = create_safe_area_helper(trigger_ref, content_ref, send);

// 7. Positioning engine (client-only, runs when open)
let positioning_handle = create_positioning_helper(
    trigger_ref, positioner_ref, content_ref, positioning_options
);

// 8. Z-index allocation (client-only, on open)
let z_index_handle = create_z_index_helper(is_open);

// 9. Portal rendering (client-only)
// Positioner renders into portal root via ArsProvider

// 10. Presence integration (optional, for animations)
// Content wrapped in Presence for enter/exit transitions

// 11. Cleanup
on_cleanup(move || {
    safe_area_handle.teardown();
    positioning_handle.teardown();
    z_index_handle.release();
    // Machine PendingEffect timers auto-cleanup via machine lifecycle
});
```

## 26. Adapter Invariants

- Timer management (open/close delays) is owned by the machine's `PendingEffect` system. The adapter must not duplicate timer logic. Machine effects handle `set_timeout` and `clear_timeout` calls.
- Safe-area (hover bridge) `pointermove` listener must be adapter-owned, client-only, and cleaned up when the card closes, the pointer enters content, or the component unmounts.
- Content must render into a portal root via `ArsProvider`, not inline with the trigger. Inline fallback is permitted only when portal context is unavailable.
- Positioner CSS custom properties (`--ars-x`, `--ars-y`, `--ars-z-index`, `--ars-reference-width`, `--ars-reference-height`, `--ars-available-width`, `--ars-available-height`, `--ars-transform-origin`) must be set by the positioning engine, not hardcoded.
- Z-index must be allocated from `ZIndexAllocator` on open and released on close/unmount.
- `aria-expanded` on trigger must always reflect the current open state. `aria-controls` must only be present when the card is open (referencing the content ID).
- `role="dialog"` must be set on the content element. `aria-labelledby` must reference the title ID when a `Title` is rendered; otherwise `aria-label` must be set from `Messages.label`.
- `on_open_change` must fire only for final `Open`/`Closed` transitions, not for `OpenPending`/`ClosePending`.
- `DismissButton` must be a visually hidden native `<button>` element — it must not be a `<div>` with `role="button"`.
- Controlled `open` synchronization must use a deferred `Effect`, not body-level prop sync, because it dispatches machine events.
- All document-level listeners (safe-area `pointermove`, keyboard) must be removed before unmount completes.
- SSR must render the trigger with correct ARIA attributes; content structure is client-only unless `default_open` is true.

## 27. Accessibility and SSR Notes

- Content has `role="dialog"` — it is announced as a dialog to screen readers.
- Content is NOT `aria-modal` — background content remains interactive. Users can Tab out of the card.
- `FocusScope::popover()` preset is recommended: `contain=false` (no focus trapping), `restore_focus=true` (focus returns to trigger on close).
- `DismissButton` provides a screen reader close mechanism. It is visually hidden but focusable and labeled via `Messages`.
- `Title` enables `aria-labelledby` on content. When no title is rendered, `aria-label` from `Messages.label` provides the accessible name (default: "Additional information").
- Keyboard accessibility: Enter/Space opens immediately (bypassing delay), Escape closes immediately, Tab can enter content.
- SSR renders the trigger with `aria-expanded="false"` and scope/part data attributes. Content is absent on SSR (client-only portal mount) unless `default_open` is true.
- The trigger ID and content ID are machine-generated and stable across hydration.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, and structure parity. All seven parts (Root, Trigger, Positioner, Content, Arrow, Title, DismissButton) are mapped to Leptos compound components with correct ARIA wiring, timer-based lifecycle, safe-area management, portal rendering, and positioning engine integration.

Intentional deviations: none.

Traceability note: This adapter spec explicitly covers the core adapter-owned concerns for safe-area hover bridge management, timer lifecycle via `PendingEffect`, portal rendering, positioning engine integration with CSS custom properties, z-index allocation, ARIA wiring for `aria-labelledby`/`aria-label` fallback, `DismissButton` as a screen reader close mechanism, keyboard accessibility (Enter/Space/Escape), and `lazy_mount`/`unmount_on_exit` rendering decisions.

## 29. Test Scenarios

- hover card opens after 700ms hover delay on trigger
- hover card closes after 300ms when pointer leaves trigger and content
- hover card stays open while pointer moves through safe-area triangle
- hover card stays open while pointer is inside content
- hover card closes immediately on Escape from trigger or content
- hover card opens immediately on Enter/Space keydown
- hover card opens on trigger focus after delay
- hover card does not open when disabled
- controlled `open` prop synchronizes open/close state
- `on_open_change` fires only for final Open/Closed transitions
- `aria-expanded` reflects open state on trigger
- `aria-controls` present only when open
- `role="dialog"` present on content
- `aria-labelledby` set when Title is rendered
- `aria-label` fallback when no Title is rendered
- `TitleMount` event fires when Title mounts
- DismissButton is visually hidden and triggers close on click
- content renders in portal root, not inline
- positioner has CSS custom properties after positioning
- z-index allocated from ZIndexAllocator on open, released on close
- `lazy_mount` delays content rendering until first open
- `unmount_on_exit` removes content DOM on close
- safe-area listener cleaned up on close and unmount
- all timers cleaned up on unmount
- SSR renders trigger with ARIA attrs, content absent until client mount
- multiple HoverCard instances do not share state

## 30. Test Oracle Notes

| Behavior                         | Preferred oracle type | Notes                                                                     |
| -------------------------------- | --------------------- | ------------------------------------------------------------------------- |
| trigger ARIA attributes          | DOM attrs             | assert `aria-expanded`, `aria-controls`, scope/part data attrs            |
| content ARIA attributes          | DOM attrs             | assert `role="dialog"`, `aria-labelledby` or `aria-label`                 |
| open/close state transitions     | machine state         | assert state is Open/Closed after events, not intermediate pending states |
| `on_open_change` callback timing | callback order        | assert fires after final transition, not during pending                   |
| timer-based delays               | machine state         | use fake timers; assert OpenPending after enter, Open after timer fires   |
| safe-area behavior               | machine state         | simulate pointermove within/outside polygon; assert state accordingly     |
| portal rendering                 | rendered structure    | assert content is inside `ars-portal-root`, not inline                    |
| positioner CSS custom properties | DOM attrs             | assert `--ars-x`, `--ars-y`, `--ars-z-index` are set after positioning    |
| z-index allocation               | context registration  | assert z-index allocated on open, released on close                       |
| DismissButton structure          | rendered structure    | assert native `<button>`, visually hidden, labeled                        |
| cleanup on unmount               | cleanup side effects  | assert no leaked timers, listeners, or z-index allocations after unmount  |
| SSR hydration stability          | hydration structure   | assert trigger structure matches between SSR and hydrated client          |

Cheap verification recipe:

1. Render HoverCard with Trigger, Positioner, Content, and Title. Assert trigger has `aria-expanded="false"` and no `aria-controls`.
2. Simulate `pointerenter` on trigger. Assert state is `OpenPending`. Advance fake timer by 700ms. Assert state is `Open`, trigger has `aria-expanded="true"` and `aria-controls` referencing content ID.
3. Simulate `pointerleave` on trigger. Assert safe-area listener attached. Simulate `pointerenter` on content. Assert state remains `Open` and safe-area listener detached.
4. Simulate `pointerleave` on content. Assert state is `ClosePending`. Advance fake timer by 300ms. Assert state is `Closed`.
5. Simulate Enter keydown on trigger. Assert state is `Open` immediately (no delay).
6. Simulate Escape keydown. Assert state is `Closed` immediately.
7. Unmount component. Assert no leaked timers, listeners, or z-index allocations.

## 31. Implementation Checklist

- [ ] `HoverCard` creates machine, provides context, renders root wrapper with scope/state attrs.
- [ ] `Trigger` wires `pointerenter`, `pointerleave`, `focus`, `blur`, `keydown` to machine events.
- [ ] `Positioner` renders in portal root with CSS custom properties from positioning engine.
- [ ] `Content` has `role="dialog"`, `aria-labelledby`/`aria-label`, pointer enter/leave handlers.
- [ ] `Arrow` positioned by engine within positioner.
- [ ] `Title` fires `TitleMount` on mount, provides `id` for `aria-labelledby`.
- [ ] `DismissButton` is a native `<button>`, visually hidden, dispatches close.
- [ ] Controlled `open` prop synchronized via deferred `Effect`.
- [ ] `on_open_change` fires only on final Open/Closed transitions.
- [ ] Open delay timer (700ms default) managed by machine `PendingEffect`.
- [ ] Close delay timer (300ms default) managed by machine `PendingEffect`.
- [ ] Safe-area hover bridge computes triangle and manages `pointermove` listener.
- [ ] Safe-area listener cleaned up on close, content enter, and unmount.
- [ ] Z-index allocated from `ZIndexAllocator` on open, released on close/unmount.
- [ ] Portal rendering via `ArsProvider`; inline fallback with debug warning if unavailable.
- [ ] `lazy_mount` and `unmount_on_exit` control content DOM lifecycle.
- [ ] SSR renders trigger with correct ARIA attrs; content is client-only.
- [ ] All timers, listeners, and allocations cleaned up via `on_cleanup`.
- [ ] Multiple instances do not share timer, safe-area, or positioning state.
- [ ] Keyboard accessibility: Enter/Space opens immediately, Escape closes immediately, Tab enters content.
