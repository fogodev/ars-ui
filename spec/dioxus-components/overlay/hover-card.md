---
adapter: dioxus
component: hover-card
category: overlay
source: components/overlay/hover-card.md
source_foundation: foundation/09-adapter-dioxus.md
---

# HoverCard — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`HoverCard`](../../components/overlay/hover-card.md) behavior to Dioxus 0.7.x. The adapter owns the compound component tree (`HoverCard`, `Trigger`, `Positioner`, `Content`, `Arrow`, `Title`, `DismissButton`), pointer and focus event wiring, timer-based open/close delay lifecycle, safe-area (hover bridge) management, portal rendering of positioned content, positioning engine integration with CSS custom properties, z-index allocation, and the `Presence`-based mount/unmount animation lifecycle.

HoverCard is like Popover but triggered by hover with a longer delay (700ms default). Unlike Tooltip, the content is **interactive** — users can Tab into it and interact with links and buttons inside the card. The adapter must keep the card open while the pointer traverses the safe-area triangle between trigger and content. On Dioxus Desktop and Mobile targets, hover behavior may differ from browser expectations; the adapter must validate pointer event dispatch against the actual host runtime.

## 2. Public Adapter API

```rust
pub mod hover_card {
    use dioxus::prelude::*;

    #[derive(Props, Clone, PartialEq)]
    pub struct HoverCardProps {
        #[props(into)]
        pub id: String,
        #[props(optional)]
        pub open: Option<Signal<bool>>,
        #[props(optional, default = false)]
        pub default_open: bool,
        #[props(optional, default = 700)]
        pub open_delay_ms: u32,
        #[props(optional, default = 300)]
        pub close_delay_ms: u32,
        #[props(optional, default = false)]
        pub disabled: bool,
        #[props(optional)]
        pub positioning: Option<PositioningOptions>,
        #[props(optional)]
        pub on_open_change: Option<EventHandler<bool>>,
        #[props(optional, default = false)]
        pub lazy_mount: bool,
        #[props(optional, default = false)]
        pub unmount_on_exit: bool,
        #[props(optional)]
        pub messages: Option<hover_card::Messages>,
        #[props(optional)]
        pub locale: Option<Locale>,
        pub children: Element,
    }

    #[component]
    pub fn HoverCard(props: HoverCardProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct TriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn Trigger(props: TriggerProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct PositionerProps {
        pub children: Element,
    }

    #[component]
    pub fn Positioner(props: PositionerProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct ContentProps {
        pub children: Element,
    }

    #[component]
    pub fn Content(props: ContentProps) -> Element

    #[component]
    pub fn Arrow() -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct TitleProps {
        pub children: Element,
    }

    #[component]
    pub fn Title(props: TitleProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct DismissButtonProps {
        pub children: Element,
    }

    #[component]
    pub fn DismissButton(props: DismissButtonProps) -> Element
}
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

| Target node   | Core attrs                   | Adapter-owned attrs                                                           | Consumer attrs              | Merge order                                                                   | Ownership notes                                               |
| ------------- | ---------------------------- | ----------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Root          | `api.root_attrs()`           | structural `data-ars-scope`, `data-ars-state`                                 | consumer `class`/`style`    | core scope and state attrs win; `class`/`style` merge additively              | adapter-owned wrapper                                         |
| Trigger       | `api.trigger_attrs()`        | `onpointerenter`, `onpointerleave`, `onfocus`, `onblur`, `onkeydown`          | consumer `class`/`style`    | core ARIA attrs (`aria-expanded`, `aria-controls`) win; handlers compose      | consumer-owned element with adapter-composed handlers         |
| Positioner    | `api.positioner_attrs()`     | CSS custom properties (`--ars-x`, `--ars-y`, etc.), positioning `style`       | consumer `class` if exposed | core part attrs win; adapter positioning style is authoritative               | adapter-owned; rendered in portal root                        |
| Content       | `api.content_attrs()`        | `onpointerenter`, `onpointerleave`, `onkeydown`, `role="dialog"`, ARIA wiring | consumer `class`/`style`    | core `role`, ARIA, and `id` attrs win; pointer handlers are adapter-exclusive | adapter-owned structural node with consumer children          |
| Arrow         | `api.arrow_attrs()`          | positioning style from engine                                                 | consumer `class`/`style`    | core part attrs win; positioning style is authoritative                       | adapter-owned decorative node                                 |
| Title         | `api.title_attrs()`          | `id` for `aria-labelledby` wiring                                             | consumer heading content    | core `id` wins                                                                | consumer-owned content; adapter wires ID and fires TitleMount |
| DismissButton | `api.dismiss_button_attrs()` | visually-hidden styles, `onclick` for close, `aria-label` from messages       | none                        | adapter-exclusive                                                             | adapter-owned; not consumer-customizable                      |

- Consumers must not override `aria-expanded`, `aria-controls`, or `role` on the trigger or content.
- Positioning CSS custom properties on the positioner are authoritative and must not be overridden by consumer styles.
- The `id` attribute on Title is machine-generated and must not be changed by the consumer.

## 6. Composition / Context Contract

`HoverCard` provides a `Ctx` via `use_context_provider`. All descendant parts consume it via `try_use_context::<Ctx>().expect("... must be used inside HoverCard")`.

```rust
pub mod hover_card {
    #[derive(Clone, Copy)]
    struct Ctx {
        send: Callback<hover_card::Event>,
        is_open: Memo<bool>,
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
}
```

Composing overlays:

- HoverCard consumes `ArsProvider` context for portal root resolution.
- HoverCard consumes `ZIndexAllocator` context for z-index allocation on the positioner.
- HoverCard content may optionally compose with `Presence` for enter/exit animations.
- HoverCard does NOT compose with `FocusScope` in contain mode — content is non-modal. However, `FocusScope::popover()` preset (`contain=false`, `restore_focus=true`) is used so focus returns to trigger on close.
- HoverCard does NOT compose with `Dismissable` for outside-click — it closes on pointer leave, not outside interaction.

## 7. Prop Sync and Event Mapping

| Adapter prop     | Mode       | Sync trigger             | Machine event / update path        | Visible effect                           | Notes                                                |
| ---------------- | ---------- | ------------------------ | ---------------------------------- | ---------------------------------------- | ---------------------------------------------------- |
| `open`           | controlled | `Signal` change          | `PropSync` open/close event        | opens or closes the hover card           | deferred `use_effect` watches `Signal<bool>` changes |
| `disabled`       | static     | render time only         | guards all transitions in machine  | disables all hover/focus/keyboard open   | machine returns `None` for all events when disabled  |
| `open_delay_ms`  | static     | init and context rebuild | stored in `Context.open_delay_ms`  | adjusts timer duration for open pending  | not reactive after mount                             |
| `close_delay_ms` | static     | init and context rebuild | stored in `Context.close_delay_ms` | adjusts timer duration for close pending | not reactive after mount                             |
| `positioning`    | static     | init and positioning run | passed to positioning engine       | affects positioner CSS custom properties | re-runs positioning when content opens               |

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

| Registered entity       | Registration trigger                    | Identity key      | Cleanup trigger                         | Cleanup action                                         | Notes                                  |
| ----------------------- | --------------------------------------- | ----------------- | --------------------------------------- | ------------------------------------------------------ | -------------------------------------- |
| open delay timer        | `TriggerPointerEnter` or `TriggerFocus` | machine effect    | timer fires, cancel, or close           | `platform.clear_timeout(handle)`                       | client-only; managed by machine effect |
| close delay timer       | pointer/focus leave events              | machine effect    | timer fires, cancel, or re-enter        | `platform.clear_timeout(handle)`                       | client-only; managed by machine effect |
| safe area `pointermove` | `TriggerPointerLeave` while open        | adapter-local     | pointer enters content or exits polygon | `remove_event_listener` on document                    | client-only; adapter-owned cleanup     |
| positioning engine      | content opens (first open or re-open)   | adapter-local     | content closes or component unmount     | detach positioning observer / cleanup                  | client-only; re-runs on open           |
| z-index allocation      | content opens                           | allocator context | content closes or component unmount     | release z-index back to allocator                      | via `ZIndexAllocator` context          |
| portal mount            | content opens (respecting lazy_mount)   | portal context    | unmount_on_exit close or unmount        | remove portal content                                  | via `ArsProvider` portal root          |
| `use_drop` registration | component mount                         | component scope   | component unmount                       | cancel all timers, remove safe-area listener, teardown | ensures no leaked timers or listeners  |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner      | Node availability                  | Composition rule                                                     | Notes                                                         |
| ------------------ | ------------- | -------------- | ---------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------- |
| Trigger element    | yes           | adapter-owned  | required after mount               | trigger ref is the positioning anchor; passed to positioning engine  | Dioxus `onmounted` callback captures the `MountedData` handle |
| Content element    | yes           | adapter-owned  | client-only                        | content ref for safe-area polygon computation and positioning target | needed for pointer-in-content detection and positioning       |
| Positioner element | yes           | adapter-owned  | client-only                        | receives CSS custom properties from positioning engine               | structural handle for positioning updates                     |
| Arrow element      | optional      | adapter-owned  | client-only                        | optional ref for arrow positioning within the positioning engine     | only needed if arrow is rendered                              |
| Root element       | no            | adapter-owned  | always structural, handle optional | no composition requirement                                           | structural wrapper; ref not required for behavior             |
| Title element      | no            | consumer-owned | always structural, handle optional | fires `TitleMount` via `use_effect` on mount, no ref needed          | ID-based wiring via `aria-labelledby`                         |
| DismissButton      | no            | adapter-owned  | always structural, handle optional | no composition requirement                                           | semantics do not depend on stored refs                        |

## 10. State Machine Boundary Rules

- machine-owned state: `State` (Closed/OpenPending/Open/ClosePending), `Context.open`, `Context.hover_active`, `Context.focus_active`, `Context.has_title`, and all timer-based transitions.
- adapter-local derived bookkeeping: safe-area `pointermove` listener handle, positioning engine handle, z-index allocation handle, portal mount state, `lazy_mount`/`unmount_on_exit` rendering decisions, `MountedData` element handles.
- forbidden local mirrors: do not keep a local `is_open` signal that can diverge from `machine.derive(|api| api.is_open())`. Do not keep a local timer that duplicates the machine's `PendingEffect`-managed timers.
- allowed snapshot-read contexts: event handler closures, mount effects, cleanup closures, and positioning callbacks may snapshot-read from the machine API.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape | Timing                                 | Cancelable? | Notes                                                             |
| ---------------- | ------------------------ | ------------- | -------------------------------------- | ----------- | ----------------------------------------------------------------- |
| `on_open_change` | machine-derived snapshot | `bool`        | after state transitions to Open/Closed | no          | fires when open state changes, not on intermediate pending states |

## 12. Failure and Degradation Rules

| Condition                                                  | Policy             | Notes                                                                                      |
| ---------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------ |
| trigger ref missing after mount                            | fail fast          | positioning engine cannot anchor without the trigger element                               |
| content ref missing when open                              | fail fast          | safe-area computation and positioning require the content node                             |
| portal root unavailable (ArsProvider missing)              | degrade gracefully | render content inline as a fallback; log debug warning                                     |
| z-index allocator unavailable                              | degrade gracefully | use a reasonable default z-index; log debug warning                                        |
| positioning engine fails to compute                        | degrade gracefully | content renders at default position; visible but potentially misplaced                     |
| timer APIs absent during SSR                               | no-op              | trigger renders with ARIA attrs; content waits for client mount                            |
| safe-area pointermove listener fails to attach             | warn and ignore    | close delay runs normally without safe-area bridge; pointer may not reach card             |
| Desktop/Mobile pointer event dispatch differs from browser | degrade gracefully | validate hover behavior on actual host; fall back to focus-based open if hover unavailable |

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
- Never hold `.read()` or `.write()` guards on signals across `.await` boundaries — clone values out first.

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

1. Wire `HoverCard` with `use_machine::<hover_card::Machine>()`, context provision via `use_context_provider`, and controlled `open` signal synchronization.
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
12. Test on Dioxus Desktop and Mobile targets to validate pointer event dispatch differences.

## 18. Anti-Patterns

- Do not attach safe-area `pointermove` listeners during SSR.
- Do not start open/close delay timers during SSR.
- Do not keep a local `is_open` signal that can diverge from the machine state.
- Do not hardcode z-index values; use `ZIndexAllocator` context.
- Do not render content inline instead of into the portal root (unless portal is unavailable as a fallback).
- Do not use `FocusScope` in contain mode — HoverCard content is non-modal.
- Do not compose `Dismissable` for outside-click behavior — HoverCard uses pointer leave, not outside interaction.
- Do not leave safe-area `pointermove` listeners attached when the card is closed.
- Do not fire `on_open_change` for intermediate `OpenPending`/`ClosePending` states — only for final `Open`/`Closed` transitions.
- Do not duplicate the machine's timer management with adapter-local timers.
- Do not omit the `role="dialog"` attribute on content.
- Do not set `aria-labelledby` when no Title element is rendered — fall back to `aria-label` from Messages.
- Do not hold `.read()` or `.write()` guards across `.await` — clone values out first.
- Do not assume browser-identical pointer event dispatch on Desktop or Mobile targets.

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
- Consumers must not assume browser-identical hover behavior on Desktop or Mobile targets.

## 20. Platform Support Matrix

| Capability / behavior                     | Web          | Desktop       | Mobile        | SSR            | Notes                                                                            |
| ----------------------------------------- | ------------ | ------------- | ------------- | -------------- | -------------------------------------------------------------------------------- |
| trigger rendering with ARIA attrs         | full support | full support  | full support  | full support   | `aria-expanded`, `aria-controls`, scope/part data attrs render on all platforms  |
| content rendering (portal)                | full support | full support  | full support  | SSR-safe empty | content waits for client mount; inline fallback if `default_open`                |
| open/close delay timers                   | full support | full support  | full support  | client-only    | `PendingEffect` timers are client-only                                           |
| safe-area hover bridge                    | full support | fallback path | fallback path | client-only    | Desktop/Mobile may not dispatch pointer events identically to browser            |
| positioning engine                        | full support | full support  | full support  | client-only    | CSS custom properties computed after mount                                       |
| z-index allocation                        | full support | full support  | full support  | client-only    | allocated on open, released on close                                             |
| `on_open_change` callback                 | full support | full support  | full support  | client-only    | must not fire during SSR                                                         |
| keyboard interaction (Enter/Space/Escape) | full support | full support  | fallback path | client-only    | Mobile may not have physical keyboard; screen reader gestures may differ         |
| hover trigger behavior                    | full support | fallback path | fallback path | client-only    | Desktop webview may differ; Mobile typically lacks hover; focus path still works |

## 21. Debug Diagnostics and Production Policy

| Condition                                                  | Debug build behavior | Production behavior | Notes                                               |
| ---------------------------------------------------------- | -------------------- | ------------------- | --------------------------------------------------- |
| trigger ref missing after mount                            | fail fast            | fail fast           | positioning cannot work without an anchor element   |
| content ref missing when card opens                        | fail fast            | fail fast           | safe-area and positioning require the content node  |
| `ArsProvider` context missing                              | debug warning        | degrade gracefully  | fall back to inline rendering; log warning in debug |
| `ZIndexAllocator` context missing                          | debug warning        | degrade gracefully  | use default z-index; log warning in debug           |
| positioning engine returns no result                       | debug warning        | degrade gracefully  | content renders at default position                 |
| safe-area listener fails to attach                         | debug warning        | warn and ignore     | close delay runs without safe-area bridge           |
| `Trigger` used outside `HoverCard`                         | fail fast            | fail fast           | context `.expect()` panics with clear message       |
| `Content` used outside `HoverCard`                         | fail fast            | fail fast           | context `.expect()` panics with clear message       |
| Desktop/Mobile pointer event dispatch differs from browser | debug warning        | degrade gracefully  | validate hover behavior on actual host runtime      |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                     | Reused by                          | Notes                                                               |
| -------------------------- | ----------- | ---------------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------- |
| portal helper              | required    | renders content into `ars-portal-root` via `ArsProvider`                           | popover, tooltip, hover-card, tour | shared portal rendering infrastructure                              |
| positioning helper         | required    | runs `ars-dom` positioning engine, sets CSS custom properties on positioner        | popover, tooltip, hover-card, tour | includes arrow positioning when arrow ref is provided               |
| z-index helper             | required    | allocates and releases z-index from `ZIndexAllocator` context                      | all overlays                       | allocate on open, release on close/unmount                          |
| timer helper               | recommended | client-only timer management with cleanup integration                              | tooltip, hover-card, toast         | machine `PendingEffect` manages timers; adapter ensures cleanup     |
| safe-area helper           | required    | computes safe triangle between trigger and content, manages `pointermove` listener | hover-card, tooltip (interactive)  | reusable for any hover-triggered overlay with a gap                 |
| merge helper               | recommended | merges core `AttrMap` with adapter-owned attrs and consumer attrs                  | all components                     | ensures correct precedence: core wins, then adapter, then consumer  |
| platform capability helper | recommended | normalizes pointer/hover assumptions across Web, Desktop, and Mobile               | hover-card, tooltip, dismissable   | surfaces target-specific caveats without duplicating listener logic |

## 23. Framework-Specific Behavior

Dioxus uses `use_drop` for timer and listener teardown on unmount. Safe-area `pointermove` listeners on Web targets attach via `web_sys::Document::add_event_listener_with_callback` and must be removed in the drop closure. On Dioxus Desktop, pointer events are dispatched through the webview bridge and may not match browser `pointerenter`/`pointerleave` semantics exactly; the adapter must validate behavior on the actual host. On Dioxus Mobile, hover events are typically unavailable — the focus-based open path (Enter/Space from keyboard or screen reader gesture) is the primary accessibility fallback.

Controlled `open` synchronization uses a deferred `use_effect` watching `Signal<bool>` changes — this is an intentional exception to body-level sync because it dispatches machine events. Optional `ArsProvider` context is read via `try_use_context::<ArsContext>()`.

Dioxus `Signal<T>` is `Copy`, which simplifies passing signals through context. However, `.read()` and `.write()` guards must never be held across `.await` boundaries.

Event handler closures in Dioxus may be `async` (`onclick: move |_| async move { ... }`), but HoverCard handlers are synchronous — they dispatch machine events immediately without async work.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[component]
pub fn HoverCard(props: HoverCardProps) -> Element {
    let core_props = hover_card::Props {
        id: props.id,
        open: props.open.map(|s| *s.read()),
        default_open: props.default_open,
        open_delay_ms: props.open_delay_ms,
        close_delay_ms: props.close_delay_ms,
        disabled: props.disabled,
        positioning: props.positioning.unwrap_or_default(),
        on_open_change: props.on_open_change.map(|h| Callback::new(move |v| h.call(v))),
        lazy_mount: props.lazy_mount,
        unmount_on_exit: props.unmount_on_exit,
        messages: props.messages,
        locale: props.locale,
    };

    let machine = use_machine::<hover_card::Machine>(core_props);
    let send = machine.send;

    // Controlled open synchronization
    if let Some(open_sig) = props.open {
        let mut prev_open = use_signal(|| None::<bool>);
        use_effect(move || {
            let new_open = *open_sig.read();
            let prev = *prev_open.read();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.call(hover_card::Event::TriggerPointerEnter);
                    } else {
                        send.call(hover_card::Event::CloseOnEscape);
                    }
                }
                *prev_open.write() = Some(new_open);
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

    use_context_provider(|| Ctx {
        send,
        is_open,
        trigger_attrs,
        positioner_attrs,
        content_attrs,
        arrow_attrs,
        title_attrs,
        dismiss_button_attrs,
        positioning: props.positioning.unwrap_or_default(),
        lazy_mount: props.lazy_mount,
        unmount_on_exit: props.unmount_on_exit,
    });

    let attrs = root_attrs.read();
    rsx! {
        div {
            "data-ars-scope": "hover-card",
            "data-ars-part": "root",
            "data-ars-state": if is_open() { "open" } else { "closed" },
            {props.children}
        }
    }
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::Trigger must be used inside HoverCard");
    let send = ctx.send;
    let is_open = ctx.is_open;
    let content_id = ctx.content_attrs.read().get("id").cloned().unwrap_or_default();

    rsx! {
        div {
            "data-ars-scope": "hover-card",
            "data-ars-part": "trigger",
            "aria-expanded": is_open().to_string(),
            "aria-controls": if is_open() { content_id.clone() } else { String::new() },
            onpointerenter: move |_| send.call(hover_card::Event::TriggerPointerEnter),
            onpointerleave: move |_| send.call(hover_card::Event::TriggerPointerLeave),
            onfocus: move |_| send.call(hover_card::Event::TriggerFocus),
            onblur: move |_| send.call(hover_card::Event::TriggerBlur),
            onkeydown: move |e: KeyboardEvent| {
                let key = dioxus_key_to_keyboard_key(&e.key());
                match key {
                    KeyboardKey::Enter | KeyboardKey::Space => {
                        send.call(hover_card::Event::TriggerKeyDown(key));
                    }
                    KeyboardKey::Escape => {
                        send.call(hover_card::Event::CloseOnEscape);
                    }
                    _ => {}
                }
            },
            {props.children}
        }
    }
}

#[component]
pub fn Positioner(props: PositionerProps) -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::Positioner must be used inside HoverCard");

    // Positioning engine runs client-only after mount when open.
    // CSS custom properties (--ars-x, --ars-y, --ars-z-index, etc.)
    // are set on the positioner element by the positioning helper.

    if !ctx.is_open() {
        return rsx! {};
    }

    rsx! {
        // Portal rendering via ArsProvider
        div {
            "data-ars-scope": "hover-card",
            "data-ars-part": "positioner",
            {props.children}
        }
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::Content must be used inside HoverCard");
    let send = ctx.send;

    rsx! {
        div {
            role: "dialog",
            "data-ars-scope": "hover-card",
            "data-ars-part": "content",
            "data-ars-state": if ctx.is_open() { "open" } else { "closed" },
            onpointerenter: move |_| send.call(hover_card::Event::ContentPointerEnter),
            onpointerleave: move |_| send.call(hover_card::Event::ContentPointerLeave),
            onkeydown: move |e: KeyboardEvent| {
                if dioxus_key_to_keyboard_key(&e.key()) == KeyboardKey::Escape {
                    send.call(hover_card::Event::CloseOnEscape);
                }
            },
            {props.children}
        }
    }
}

#[component]
pub fn Arrow() -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::Arrow must be used inside HoverCard");

    rsx! {
        div {
            "data-ars-scope": "hover-card",
            "data-ars-part": "arrow",
        }
    }
}

#[component]
pub fn Title(props: TitleProps) -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::Title must be used inside HoverCard");
    let send = ctx.send;

    // Fire TitleMount on mount
    use_effect(move || {
        send.call(hover_card::Event::TitleMount);
    });

    rsx! {
        div {
            "data-ars-scope": "hover-card",
            "data-ars-part": "title",
            {props.children}
        }
    }
}

#[component]
pub fn DismissButton(props: DismissButtonProps) -> Element {
    let ctx = try_use_context::<Ctx>()
        .expect("hover_card::DismissButton must be used inside HoverCard");
    let send = ctx.send;

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "hover-card",
            "data-ars-part": "dismiss-button",
            onclick: move |_| send.call(hover_card::Event::CloseOnEscape),
            {props.children}
        }
    }
}

// Usage:
// rsx! {
//     HoverCard { id: "my-card",
//         hover_card::Trigger {
//             a { href: "https://example.com", "Hover me" }
//         }
//         hover_card::Positioner {
//             hover_card::Content {
//                 hover_card::Title { "Preview" }
//                 p { "This is the hover card content with links and buttons." }
//                 a { href: "https://example.com", "Visit site" }
//                 hover_card::DismissButton { "Close" }
//             }
//             hover_card::Arrow {}
//         }
//     }
// }
```

## 25. Reference Implementation Skeleton

```rust
// 1. Machine setup
let machine = use_machine::<hover_card::Machine>(props);
let send = machine.send;

// 2. Controlled open sync (deferred use_effect, not body-level)
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

// 4. Context provision
use_context_provider(|| Ctx { send, is_open, /* ... */ });

// 5. Safe-area management (client-only)
let safe_area_handle = create_safe_area_helper(trigger_ref, content_ref, send);

// 6. Positioning engine (client-only, runs when open)
let positioning_handle = create_positioning_helper(
    trigger_ref, positioner_ref, content_ref, positioning_options
);

// 7. Z-index allocation (client-only, on open)
let z_index_handle = create_z_index_helper(is_open);

// 8. Portal rendering (client-only)
// Positioner renders into portal root via ArsProvider

// 9. Presence integration (optional, for animations)
// Content wrapped in Presence for enter/exit transitions

// 10. Platform-aware pointer validation
// On Desktop/Mobile, validate pointer event dispatch against host runtime

// 11. Cleanup
use_drop(move || {
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
- Controlled `open` synchronization must use a deferred `use_effect`, not body-level prop sync, because it dispatches machine events.
- All document-level listeners (safe-area `pointermove`, keyboard) must be removed before unmount completes via `use_drop`.
- SSR must render the trigger with correct ARIA attributes; content structure is client-only unless `default_open` is true.
- On Dioxus Desktop and Mobile targets, hover event dispatch must be validated against the actual host runtime. The focus-based open path provides a fallback when hover is unreliable or unavailable.

## 27. Accessibility and SSR Notes

- Content has `role="dialog"` — it is announced as a dialog to screen readers.
- Content is NOT `aria-modal` — background content remains interactive. Users can Tab out of the card.
- `FocusScope::popover()` preset is recommended: `contain=false` (no focus trapping), `restore_focus=true` (focus returns to trigger on close).
- `DismissButton` provides a screen reader close mechanism. It is visually hidden but focusable and labeled via `Messages`.
- `Title` enables `aria-labelledby` on content. When no title is rendered, `aria-label` from `Messages.label` provides the accessible name (default: "Additional information").
- Keyboard accessibility: Enter/Space opens immediately (bypassing delay), Escape closes immediately, Tab can enter content.
- SSR renders the trigger with `aria-expanded="false"` and scope/part data attributes. Content is absent on SSR (client-only portal mount) unless `default_open` is true.
- The trigger ID and content ID are machine-generated and stable across hydration.
- On Dioxus Mobile, hover events may be unavailable. Screen reader gestures and keyboard equivalents (Enter/Space) provide the accessibility fallback for opening the card.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, and structure parity. All seven parts (Root, Trigger, Positioner, Content, Arrow, Title, DismissButton) are mapped to Dioxus compound components with correct ARIA wiring, timer-based lifecycle, safe-area management, portal rendering, and positioning engine integration.

Intentional deviations: none from core contract. Platform-specific behavior differences on Desktop and Mobile are documented as fallback paths, not deviations from the core contract.

Traceability note: This adapter spec explicitly covers the core adapter-owned concerns for safe-area hover bridge management, timer lifecycle via `PendingEffect`, portal rendering, positioning engine integration with CSS custom properties, z-index allocation, ARIA wiring for `aria-labelledby`/`aria-label` fallback, `DismissButton` as a screen reader close mechanism, keyboard accessibility (Enter/Space/Escape), `lazy_mount`/`unmount_on_exit` rendering decisions, and platform-aware pointer event validation on Desktop and Mobile targets.

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
- Dioxus Desktop hover behavior validated against host runtime
- Dioxus Mobile fallback via focus/keyboard when hover unavailable

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
| Desktop/Mobile pointer dispatch  | callback order        | verify pointer events fire expected machine events on the target platform |

Cheap verification recipe:

1. Render HoverCard with Trigger, Positioner, Content, and Title. Assert trigger has `aria-expanded="false"` and no `aria-controls`.
2. Simulate `pointerenter` on trigger. Assert state is `OpenPending`. Advance fake timer by 700ms. Assert state is `Open`, trigger has `aria-expanded="true"` and `aria-controls` referencing content ID.
3. Simulate `pointerleave` on trigger. Assert safe-area listener attached. Simulate `pointerenter` on content. Assert state remains `Open` and safe-area listener detached.
4. Simulate `pointerleave` on content. Assert state is `ClosePending`. Advance fake timer by 300ms. Assert state is `Closed`.
5. Simulate Enter keydown on trigger. Assert state is `Open` immediately (no delay).
6. Simulate Escape keydown. Assert state is `Closed` immediately.
7. Unmount component. Assert no leaked timers, listeners, or z-index allocations.
8. On Dioxus Desktop, repeat steps 2-3 and verify pointer events dispatch correctly through the webview bridge.

## 31. Implementation Checklist

- [ ] `HoverCard` creates machine, provides context via `use_context_provider`, renders root wrapper with scope/state attrs.
- [ ] `Trigger` wires `onpointerenter`, `onpointerleave`, `onfocus`, `onblur`, `onkeydown` to machine events.
- [ ] `Positioner` renders in portal root with CSS custom properties from positioning engine.
- [ ] `Content` has `role="dialog"`, `aria-labelledby`/`aria-label`, pointer enter/leave handlers.
- [ ] `Arrow` positioned by engine within positioner.
- [ ] `Title` fires `TitleMount` on mount via `use_effect`, provides `id` for `aria-labelledby`.
- [ ] `DismissButton` is a native `<button>`, visually hidden, dispatches close.
- [ ] Controlled `open` prop synchronized via deferred `use_effect`.
- [ ] `on_open_change` fires only on final Open/Closed transitions.
- [ ] Open delay timer (700ms default) managed by machine `PendingEffect`.
- [ ] Close delay timer (300ms default) managed by machine `PendingEffect`.
- [ ] Safe-area hover bridge computes triangle and manages `pointermove` listener.
- [ ] Safe-area listener cleaned up on close, content enter, and unmount.
- [ ] Z-index allocated from `ZIndexAllocator` on open, released on close/unmount.
- [ ] Portal rendering via `ArsProvider`; inline fallback with debug warning if unavailable.
- [ ] `lazy_mount` and `unmount_on_exit` control content DOM lifecycle.
- [ ] SSR renders trigger with correct ARIA attrs; content is client-only.
- [ ] All timers, listeners, and allocations cleaned up via `use_drop`.
- [ ] Multiple instances do not share timer, safe-area, or positioning state.
- [ ] Keyboard accessibility: Enter/Space opens immediately, Escape closes immediately, Tab enters content.
- [ ] Dioxus Desktop hover behavior validated against webview host runtime.
- [ ] Dioxus Mobile fallback via focus/keyboard path tested when hover unavailable.
