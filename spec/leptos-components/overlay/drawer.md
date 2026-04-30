---
adapter: leptos
component: drawer
category: overlay
source: components/overlay/drawer.md
source_foundation: foundation/08-adapter-leptos.md
---

# Drawer -- Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Drawer`](../../components/overlay/drawer.md) machine to Leptos 0.8.x. The adapter owns compound component rendering (Drawer through DragHandle), portal rendering into the `ArsProvider` portal root, focus-scope activation via `FocusScope`, scroll-lock management, inert-attribute management via `dialog_stack_push`/`dialog_stack_pop`, dismissable outside-interaction detection via `Dismissable`, z-index allocation via `ZIndexAllocator`, backdrop sibling rendering, CSS transform-based slide animation coordination via `Presence`, drag/swipe gesture wiring on the content and drag-handle elements, snap-point keyboard navigation, and `aria-roledescription` semantic repair for screen readers.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn Drawer(...) -> impl IntoView
#[component] pub fn Trigger(children: Children) -> impl IntoView
#[component] pub fn Backdrop() -> impl IntoView
#[component] pub fn Positioner(children: Children) -> impl IntoView
#[component] pub fn Content(children: Children) -> impl IntoView
#[component] pub fn Title(children: Children) -> impl IntoView
#[component] pub fn Description(children: Children) -> impl IntoView
#[component] pub fn Header(children: Children) -> impl IntoView
#[component] pub fn Body(children: Children) -> impl IntoView
#[component] pub fn Footer(children: Children) -> impl IntoView
#[component] pub fn CloseTrigger(children: Children) -> impl IntoView
#[component] pub fn DragHandle() -> impl IntoView
```

`Drawer` accepts the full core prop set: `id`, `open`, `default_open`, `placement`, `modal`, `close_on_backdrop`, `close_on_escape`, `prevent_scroll`, `restore_focus`, `initial_focus`, `final_focus`, `dir`, `title_level`, `messages`, `snap_points`, `default_snap_index`, `on_open_change`, `lazy_mount`, `unmount_on_exit`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `drawer::Props`.
- Event parity: Open, Close, Toggle, DragStart, DragMove, DragEnd, SnapTo, CloseOnBackdropClick, CloseOnEscape all map to adapter-driven UI events.
- Structure parity: all twelve parts (Root, Trigger, Backdrop, Positioner, Content, Title, Description, Header, Body, Footer, CloseTrigger, DragHandle) are rendered as compound child components.
- Placement resolution: logical Start/End placements resolve to physical Left/Right based on `dir` prop, using `resolve_placement()`.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership         | Attr source                 | Notes                                                                         |
| --------------------- | --------- | ------------------------ | ----------------- | --------------------------- | ----------------------------------------------------------------------------- |
| Root                  | required  | `<div>` wrapper          | adapter-owned     | `api.root_attrs()`          | Container for the compound component tree; not portalled.                     |
| Trigger               | required  | native `<button>`        | consumer-composed | `api.trigger_attrs()`       | Placed inline with consumer content, outside the portal.                      |
| Backdrop              | required  | `<div>` in portal root   | adapter-owned     | `api.backdrop_attrs()`      | Sibling of Positioner inside the portal root (backdrop sibling pattern).      |
| Positioner            | required  | `<div>` in portal root   | adapter-owned     | `api.positioner_attrs()`    | Contains Content; owns CSS transform for slide animation.                     |
| Content               | required  | `<div>` in portal root   | adapter-owned     | `api.content_attrs()`       | `role="dialog"`, focus-trap target, drag gesture surface.                     |
| Title                 | optional  | `<h{n}>` element         | consumer-composed | `api.title_attrs()`         | Heading level from `title_level` prop. Labels Content via `aria-labelledby`.  |
| Description           | optional  | `<div>` element          | consumer-composed | `api.description_attrs()`   | Describes Content via `aria-describedby`.                                     |
| Header                | optional  | `<div>` element          | consumer-composed | `api.header_attrs()`        | Structural layout part.                                                       |
| Body                  | optional  | `<div>` element          | consumer-composed | `api.body_attrs()`          | Structural layout part.                                                       |
| Footer                | optional  | `<div>` element          | consumer-composed | `api.footer_attrs()`        | Structural layout part.                                                       |
| CloseTrigger          | optional  | native `<button>`        | consumer-composed | `api.close_trigger_attrs()` | `aria-label` from `Messages.close_label`.                                     |
| DragHandle            | optional  | `<div>` element          | consumer-composed | `api.drag_handle_attrs()`   | `role="slider"` when snap points configured; keyboard snap navigation target. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                                                           | Adapter-owned attrs                                                   | Consumer attrs           | Merge order                                                             | Ownership notes                                         |
| ----------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------- | ------------------------------------------------------- |
| Root        | `api.root_attrs()` (scope, part, state)                                                                              | none                                                                  | consumer root attrs      | core scope and state attrs win; `class`/`style` merge additively        | adapter-owned container                                 |
| Trigger     | `api.trigger_attrs()` (scope, part, aria-haspopup, aria-expanded)                                                    | none                                                                  | consumer trigger attrs   | core ARIA attrs win; handlers compose around Toggle                     | consumer-composed inside adapter context                |
| Backdrop    | `api.backdrop_attrs()` (scope, part, aria-hidden, inert)                                                             | click handler                                                         | consumer decoration only | core dismissal semantics win                                            | adapter-owned; click handler sends CloseOnBackdropClick |
| Positioner  | `api.positioner_attrs()` (scope, part)                                                                               | CSS transform style for slide direction                               | consumer decoration only | adapter CSS transform wins for positioning                              | adapter-owned sliding container                         |
| Content     | `api.content_attrs()` (role, aria-modal, aria-roledescription, aria-labelledby, aria-describedby, data-ars-dragging) | keydown handler, pointer/touch handlers for drag                      | consumer content attrs   | core ARIA attrs win; handlers compose; `class`/`style` merge additively | adapter-owned dialog surface                            |
| DragHandle  | `api.drag_handle_attrs()` (scope, part)                                                                              | slider ARIA attrs when snap points configured, pointer/touch handlers | consumer decoration only | adapter slider attrs win when snap points active                        | adapter-owned drag interaction surface                  |

- Consumers must not override `role`, `aria-modal`, `aria-roledescription`, or `aria-labelledby`/`aria-describedby` on Content.
- Consumers must not override slider ARIA attrs on DragHandle when snap points are configured.
- The `data-ars-dragging` attribute on Content is presence-only and must not be overridden.

## 6. Composition / Context Contract

`Drawer` provides a `Context` context containing machine state, send callback, derived open state, title/description IDs, placement, resolved placement, snap-point state, and service handle. All child part components consume this context via `use_context::<Context>().expect("...")`.

The drawer composes:

- `Presence` for mount/unmount animation lifecycle of Backdrop, Positioner, and Content.
- `FocusScope` for focus trapping within Content when modal.
- `Dismissable` for outside-interaction detection (pointer and focus events outside Content).
- `ZIndexAllocator` for portal layer ordering.
- `ArsProvider` portal root for rendering overlay content outside the component tree.
- `dialog_stack_push`/`dialog_stack_pop` for nested modal inert-attribute management (shared with Dialog).

## 7. Prop Sync and Event Mapping

Controlled `open` uses an immediate-effect watcher that sends `Open`/`Close` events on change. Switching between controlled and uncontrolled mode after mount is not supported. `default_open` is init-only.

| Adapter prop        | Mode                        | Sync trigger            | Machine event / update path | Visible effect                                    | Notes                                                          |
| ------------------- | --------------------------- | ----------------------- | --------------------------- | ------------------------------------------------- | -------------------------------------------------------------- |
| `open`              | controlled                  | prop change after mount | `Open` / `Close`            | opens or closes the drawer                        | deferred `use_effect` (same pattern as Dialog controlled open) |
| `default_open`      | uncontrolled internal state | initial render only     | initial machine props       | seeds initial open state                          | read once at initialization                                    |
| `placement`         | non-reactive adapter prop   | render time only        | initial machine props       | determines slide direction and resolved side      | changes require remount                                        |
| `dir`               | non-reactive adapter prop   | render time only        | initial machine props       | resolves logical Start/End to physical Left/Right | changes require remount                                        |
| `snap_points`       | non-reactive adapter prop   | render time only        | initial machine props       | enables bottom-sheet behavior                     | changes require remount                                        |
| `modal`             | non-reactive adapter prop   | render time only        | initial machine props       | controls focus trap, scroll lock, inert           | changes require remount                                        |
| `close_on_backdrop` | non-reactive adapter prop   | render time only        | initial machine props       | enables/disables backdrop dismiss                 | changes require remount                                        |
| `close_on_escape`   | non-reactive adapter prop   | render time only        | initial machine props       | enables/disables Escape dismiss                   | changes require remount                                        |

| UI event                                    | Preconditions                               | Machine event / callback path | Ordering notes                                              | Notes                                  |
| ------------------------------------------- | ------------------------------------------- | ----------------------------- | ----------------------------------------------------------- | -------------------------------------- |
| Trigger click                               | trigger rendered and interactive            | `Toggle`                      | fires before open-change callback                           | standard button activation             |
| Backdrop click                              | drawer open and `close_on_backdrop` enabled | `CloseOnBackdropClick`        | containment check runs before close transition              | Dismissable-mediated when composed     |
| Escape keydown on Content                   | drawer open and `close_on_escape` enabled   | `CloseOnEscape`               | Escape handling runs before notification callbacks          | topmost-only via dialog stack          |
| CloseTrigger click                          | drawer open                                 | `Close`                       | fires before open-change callback                           | direct close path                      |
| Pointer/touch down on Content or DragHandle | drawer open and drag enabled                | `DragStart(position)`         | sets Dragging state before subsequent moves                 | initiates swipe gesture tracking       |
| Pointer/touch move during drag              | `state == Dragging(_)`                      | `DragMove(position)`          | updates drag position before visual feedback                | CSS transform updated to follow drag   |
| Pointer/touch up during drag                | `state == Dragging(_)`                      | `DragEnd(position)`           | velocity-based snap resolution runs before state transition | may close drawer if threshold exceeded |
| Arrow Up on DragHandle                      | drawer open with snap points                | `SnapTo(index)`               | fires after focus validation                                | moves to next larger snap point        |
| Arrow Down on DragHandle                    | drawer open with snap points                | `SnapTo(index)`               | fires after focus validation                                | moves to next smaller snap point       |
| Home on DragHandle                          | drawer open with snap points                | `SnapTo(last_index)`          | fires after focus validation                                | moves to largest snap point            |
| End on DragHandle                           | drawer open with snap points                | `SnapTo(0)`                   | fires after focus validation                                | moves to smallest snap point           |

## 8. Registration and Cleanup Contract

| Registered entity      | Registration trigger               | Identity key       | Cleanup trigger                               | Cleanup action                                                   | Notes                                      |
| ---------------------- | ---------------------------------- | ------------------ | --------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------ |
| dialog stack entry     | drawer opens (modal)               | drawer instance ID | drawer closes or component cleanup            | `dialog_stack_pop(id)` and re-apply inert for new top            | shared with Dialog                         |
| scroll lock            | drawer opens with `prevent_scroll` | drawer instance ID | drawer closes or component cleanup            | restore body scroll position and overflow                        | nested drawer inherits outermost lock      |
| focus scope            | drawer opens (modal)               | drawer instance ID | drawer closes or component cleanup            | deactivate focus trap, restore focus to trigger or `final_focus` | FocusScope stacking for nested overlays    |
| z-index allocation     | portal content mounts              | drawer instance ID | portal content unmounts or component cleanup  | release allocated z-index                                        | via ZIndexAllocator context                |
| Dismissable listeners  | drawer opens on the client         | drawer instance ID | drawer closes or component cleanup            | remove document-level pointer/focus/Escape listeners             | client-only                                |
| Presence animation     | portal content mounts              | drawer instance ID | exit animation completes or component cleanup | unmount portal content                                           | coordinates lazy_mount and unmount_on_exit |
| drag gesture listeners | drag starts on the client          | drawer instance ID | drag ends or component cleanup                | remove pointer/touch move/up listeners                           | client-only; document-level during drag    |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                 | Composition rule                          | Notes                                                           |
| ------------------ | ------------- | ------------- | --------------------------------- | ----------------------------------------- | --------------------------------------------------------------- |
| Root               | no            | adapter-owned | always structural handle optional | no composition needed                     | Non-portalled container; no live handle required.               |
| Trigger            | yes           | adapter-owned | required after mount              | compose if consumer needs ref access      | Needed for focus restoration on close.                          |
| Backdrop           | no            | adapter-owned | client-only                       | no composition needed                     | Portal-rendered; click handler only.                            |
| Positioner         | yes           | adapter-owned | client-only                       | no composition needed                     | Needed for CSS transform updates during drag animation.         |
| Content            | yes           | adapter-owned | required after mount              | compose if Dismissable needs boundary ref | Focus-scope target, drag surface, outside-interaction boundary. |
| DragHandle         | yes           | adapter-owned | client-only                       | no composition needed                     | Needed for pointer/touch event attachment and slider focus.     |

## 10. State Machine Boundary Rules

- Machine-owned state: open/closed/dragging state, current snap index, drag position, placement resolution, and all context fields.
- Adapter-local derived bookkeeping: pointer position during drag, velocity samples for snap resolution, Presence animation state, CSS transform values, allocated z-index, portal mount state.
- Forbidden local mirrors: do not keep a local open/closed flag that can diverge from the machine state or controlled `open` prop.
- Allowed snapshot-read contexts: render derivation via `machine.derive(...)`, pointer/touch event handlers, cleanup effects, and animation callbacks.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape           | Timing                           | Cancelable? | Notes                                    |
| ---------------- | ------------------------ | ----------------------- | -------------------------------- | ----------- | ---------------------------------------- |
| `on_open_change` | machine-derived snapshot | `bool` (new open state) | after state transition completes | no          | fires on both open and close transitions |

## 12. Failure and Degradation Rules

| Condition                                    | Policy             | Notes                                                                                              |
| -------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------- |
| Content ref missing after mount              | fail fast          | Focus trapping, drag detection, and outside-interaction detection all require a live Content node. |
| Positioner ref missing after mount           | fail fast          | CSS transform animation cannot function without a positioner node handle.                          |
| Portal root unavailable (no ArsProvider)     | fail fast          | Overlay content cannot render without a portal root.                                               |
| ZIndexAllocator context missing              | degrade gracefully | Fall back to a static z-index value and emit a debug warning.                                      |
| Snap points empty array                      | warn and ignore    | Treat as no snap points configured; disable bottom-sheet behavior.                                 |
| Snap point values outside 0.0--1.0 range     | warn and ignore    | Clamp values to valid range; emit a debug warning.                                                 |
| Browser pointer/touch APIs absent during SSR | no-op              | Render structural nodes; defer all drag and dismiss behavior until mount.                          |
| FocusScope context unavailable               | degrade gracefully | Skip focus trapping; log a debug warning. Modal behavior is degraded but content still renders.    |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                              | Notes                                        |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ---------------------------------------------------- | -------------------------------------------- |
| dialog stack entry               | instance-derived | no                  | not applicable                           | instance ID must remain stable across hydration      | Drawer uses the same dialog stack as Dialog. |
| z-index allocation               | instance-derived | no                  | not applicable                           | allocation is client-only                            | Released on unmount.                         |
| Presence mount                   | instance-derived | no                  | not applicable                           | portal structure must remain stable across hydration | Governs lazy_mount and unmount_on_exit.      |

## 14. SSR and Client Boundary Rules

- SSR renders the Root and Trigger parts inline. Portal content (Backdrop, Positioner, Content, and all children) is SSR-safe empty when `default_open` is false; when `default_open` is true, the portal structure is rendered for hydration stability.
- Scroll lock, inert management, focus trapping, dismiss listeners, drag gesture listeners, and z-index allocation are all client-only.
- Title and Description IDs must be generated deterministically (from `ComponentIds`) so that `aria-labelledby` and `aria-describedby` references remain stable across hydration.
- No callback may fire during SSR.
- `lazy_mount` defers content rendering until the first open transition; the adapter must not render content during SSR when `lazy_mount` is true and `default_open` is false.

## 15. Performance Constraints

- Drag gesture handling must throttle `DragMove` events to animation-frame rate to avoid excessive state updates.
- CSS transform updates during drag must bypass the reactive system and write directly to the positioner node's style to avoid re-rendering the component tree on each frame.
- Velocity sampling for snap resolution should keep only the last 3--5 pointer positions, not an unbounded history.
- Presence animation listeners must not churn on every render; they attach once when the content mounts and detach on unmount.
- Dialog stack, scroll lock, and inert operations must not run during drag -- only on open/close transitions.
- Backdrop click handler must use a single event listener, not per-render attachment.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                                                        | Notes                                      |
| ------------------- | ----------- | ----------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `dialog`            | required    | behavioral prerequisite | Drawer shares modal patterns (dialog stack, scroll lock, inert, focus management) with Dialog. | Reuse Dialog's adapter infrastructure.     |
| `presence`          | required    | composition contract    | Animation lifecycle for portal content mount/unmount.                                          | Entry/exit animation coordination.         |
| `focus-scope`       | required    | composition contract    | Focus trapping within Content when modal.                                                      | Shared with Dialog.                        |
| `dismissable`       | required    | composition contract    | Outside-interaction detection for backdrop and Escape dismissal.                               | Shared with Dialog.                        |
| `ars-provider`      | required    | context contract        | Portal root for rendering overlay content.                                                     | Shared across all overlays.                |
| `z-index-allocator` | recommended | context contract        | Layer ordering for portal content.                                                             | Fallback to static z-index if unavailable. |

## 17. Recommended Implementation Sequence

1. Initialize the drawer machine with core props and provide `Context` via `provide_context`.
2. Render Root and Trigger inline; wire Trigger click to `Toggle`.
3. Set up portal rendering for Backdrop, Positioner, and Content via ArsProvider.
4. Compose Presence for mount/unmount animation lifecycle of portal content.
5. Wire Backdrop click to `CloseOnBackdropClick`; compose Dismissable for outside-interaction detection.
6. Wire Content keydown for Escape to `CloseOnEscape`.
7. Activate FocusScope on Content after entry animation starts; wire focus restoration on close.
8. Apply scroll lock and dialog stack push/pop on open/close transitions.
9. Allocate z-index via ZIndexAllocator on portal mount.
10. Wire drag gesture listeners (pointer/touch down, move, up) on Content and DragHandle.
11. Implement velocity-based snap resolution and rubber-band overdrag for bottom-sheet mode.
12. Wire DragHandle keyboard navigation (Arrow Up/Down, Home/End) to `SnapTo` events.
13. Add slider ARIA attrs to DragHandle when snap points are configured.
14. Verify cleanup ordering: drag listeners, dismiss listeners, focus scope, scroll lock, dialog stack pop, z-index release, Presence unmount.

## 18. Anti-Patterns

- Do not render Backdrop as a parent wrapper around Content; use the backdrop sibling pattern inside the portal root.
- Do not activate FocusScope before the entry animation has started (wait for `animationstart` or immediate activation if no animation is configured).
- Do not apply scroll lock or inert changes during drag -- only on open/close transitions.
- Do not keep an unbounded velocity history during drag gestures.
- Do not write CSS transform updates through the reactive system during drag; write directly to the DOM node style.
- Do not hardcode z-index values; use `ZIndexAllocator`.
- Do not attach drag gesture listeners during SSR.
- Do not use `tabindex="0"` on Content during the animation delay period; use `tabindex="-1"` to prevent premature focus entry.
- Do not fire `on_open_change` during SSR.
- Do not keep a local open/closed flag that diverges from the machine state.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the drawer slides from the edge specified by `placement` with logical Start/End resolved based on `dir`.
- Consumers may assume focus is trapped within Content when `modal` is true and the drawer is open.
- Consumers may assume Escape closes the topmost drawer only in a nested overlay stack.
- Consumers may assume `on_open_change` fires after the state transition completes.
- Consumers may assume `data-ars-dragging` is present on Content during drag gestures and can be used for CSS transition suppression.
- Consumers may assume snap-point keyboard navigation works on DragHandle when snap points are configured.
- Consumers must not assume drag gestures fire during SSR.
- Consumers must not assume the drawer is mounted in the DOM before the first open when `lazy_mount` is true.
- Consumers must not assume z-index values are static across instances.

## 20. Platform Support Matrix

| Capability / behavior                          | Browser client | SSR            | Notes                                                            |
| ---------------------------------------------- | -------------- | -------------- | ---------------------------------------------------------------- |
| Structural rendering (Root, Trigger)           | full support   | full support   | Inline parts render on both server and client.                   |
| Portal content (Backdrop, Positioner, Content) | full support   | SSR-safe empty | Portal content renders only when open or `default_open` is true. |
| Focus trapping                                 | full support   | client-only    | FocusScope activates after mount.                                |
| Scroll lock                                    | full support   | client-only    | Body overflow manipulation is DOM-only.                          |
| Inert management                               | full support   | client-only    | Attribute manipulation is DOM-only.                              |
| Dismiss listeners                              | full support   | client-only    | Document-level listeners attach after mount.                     |
| Drag gestures                                  | full support   | client-only    | Pointer/touch events require DOM.                                |
| Snap-point keyboard navigation                 | full support   | client-only    | Keyboard events require DOM.                                     |
| CSS transform animation                        | full support   | client-only    | Animation requires DOM.                                          |
| Z-index allocation                             | full support   | client-only    | Allocation is runtime-only.                                      |

## 21. Debug Diagnostics and Production Policy

| Condition                          | Debug build behavior | Production behavior | Notes                                                            |
| ---------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------- |
| Content ref missing after mount    | fail fast            | fail fast           | Core overlay functionality cannot work without the content node. |
| Positioner ref missing after mount | fail fast            | fail fast           | Slide animation cannot function without the positioner node.     |
| Portal root unavailable            | fail fast            | fail fast           | No fallback exists for portal rendering.                         |
| ZIndexAllocator context missing    | debug warning        | degrade gracefully  | Falls back to static z-index.                                    |
| Snap point values outside 0.0--1.0 | debug warning        | warn and ignore     | Values are clamped silently in production.                       |
| Empty snap_points array            | debug warning        | warn and ignore     | Bottom-sheet mode disabled.                                      |
| FocusScope context unavailable     | debug warning        | degrade gracefully  | Modal behavior degraded.                                         |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                    | Reused by                                                       | Notes                                                          |
| -------------------------- | ----------- | ----------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------- |
| Portal helper              | required    | Render overlay content into the ArsProvider portal root.          | `dialog`, `drawer`, `popover`, `tooltip`, `hover-card`, `toast` | Shared across all overlays.                                    |
| Focus-scope helper         | required    | Activate and deactivate focus trapping within Content.            | `dialog`, `drawer`, `alert-dialog`                              | Modal overlays share focus-scope infrastructure.               |
| Dismiss helper             | required    | Attach and detach outside-interaction listeners.                  | `dialog`, `drawer`, `alert-dialog`, `popover`                   | Shared dismiss infrastructure.                                 |
| Dialog stack helper        | required    | Push/pop drawer onto the global modal stack for inert management. | `dialog`, `drawer`, `alert-dialog`                              | Drawer participates in the same stack as Dialog.               |
| Scroll lock helper         | required    | Apply and restore body scroll lock.                               | `dialog`, `drawer`, `alert-dialog`                              | Nested modal stacking for outermost lock ownership.            |
| Z-index helper             | recommended | Allocate and release z-index from ZIndexAllocator.                | `dialog`, `drawer`, `popover`, `tooltip`, `hover-card`, `toast` | Fallback to static z-index if context missing.                 |
| Merge helper               | recommended | Combine core and consumer attrs with documented merge order.      | all overlay adapters                                            | Standard attr merge utility.                                   |
| Drag gesture helper        | required    | Track pointer/touch down/move/up for swipe-to-dismiss and snap.   | `drawer`                                                        | Drawer-specific; may be reused by future draggable overlays.   |
| Velocity sampling helper   | required    | Compute swipe velocity from recent pointer positions.             | `drawer`                                                        | Used for velocity-based snap targeting.                        |
| Platform capability helper | recommended | Normalize pointer/touch API availability across environments.     | `dismissable`, `drawer`, `drop-zone`                            | Surface capability caveats without duplicating listener logic. |

## 23. Framework-Specific Behavior

Leptos uses `NodeRef<html::Div>` for Positioner, Content, and DragHandle refs. Drag gesture listeners are attached via `on:pointerdown`, `on:pointermove`, `on:pointerup` on the Content and DragHandle elements; document-level move/up listeners are added via `leptos::window().add_event_listener(...)` during active drag and removed on drag end or cleanup via `on_cleanup`. CSS transform updates during drag bypass the signal system by writing directly to `positioner_ref.get().unwrap().style().set_property(...)`. The controlled `open` prop uses the same deferred `use_effect` pattern as Dialog (not body-level sync, because open/close dispatches Open/Close events). Presence animation detection uses `on:animationstart` and `on:animationend` event listeners on the Positioner element.

## 24. Canonical Implementation Sketch

```rust,no_check
use leptos::prelude::*;
use ars_core::drawer;

#[derive(Clone, Copy)]
struct Context {
    open: Memo<bool>,
    send: Callback<drawer::Event>,
    title_id: Memo<String>,
    description_id: Memo<String>,
    placement: Memo<drawer::ResolvedPlacement>,
    snap_index: Memo<usize>,
    is_dragging: Memo<bool>,
}

#[component]
pub fn Drawer(
    #[prop(into)] id: String,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional, default = false)] default_open: bool,
    #[prop(optional, default = drawer::Placement::Right)] placement: drawer::Placement,
    #[prop(optional, default = true)] modal: bool,
    #[prop(optional, default = true)] close_on_backdrop: bool,
    #[prop(optional, default = true)] close_on_escape: bool,
    #[prop(optional, default = true)] prevent_scroll: bool,
    #[prop(optional)] snap_points: Option<Vec<f64>>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView {
    let props = drawer::Props {
        id,
        open: open.map(|s| s.get_untracked()),
        default_open,
        placement,
        modal,
        close_on_backdrop,
        close_on_escape,
        prevent_scroll,
        snap_points,
        on_open_change,
        ..Default::default()
    };

    let machine = use_machine::<drawer::Machine>(props);
    let is_open = machine.derive(|api| api.is_open());
    let title_id = machine.derive(|api| api.title_id().to_string());
    let description_id = machine.derive(|api| api.description_id().to_string());
    let resolved_placement = machine.derive(|api| api.resolved_side());
    let snap_index = machine.derive(|api| api.current_snap());
    let is_dragging = machine.derive(|api| matches!(api.state(), drawer::State::Dragging(_)));

    // Controlled open sync (deferred effect, same as Dialog)
    if let Some(open_sig) = open {
        let send = machine.send;
        let prev_open = StoredValue::new(None::<bool>);
        Effect::new(move |_| {
            let new_open = open_sig.get();
            let prev = prev_open.read_value();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.run(drawer::Event::Open);
                    } else {
                        send.run(drawer::Event::Close);
                    }
                }
                prev_open.write_value(Some(new_open));
            }
        });
    }

    provide_context(Context {
        open: is_open,
        send: machine.send,
        title_id,
        description_id,
        placement: resolved_placement,
        snap_index,
        is_dragging,
    });

    view! { <div>{children()}</div> }
}

#[component]
pub fn Trigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::Trigger must be used inside Drawer");
    let trigger_attrs = /* machine.derive(|api| api.trigger_attrs()) */;
    view! {
        <button
            type="button"
            {..trigger_attrs.get()}
            on:click=move |_| ctx.send.run(drawer::Event::Toggle)
        >
            {children()}
        </button>
    }
}

#[component]
pub fn Content(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::Content must be used inside Drawer");
    let content_ref = NodeRef::<html::Div>::new();

    // Escape key handler
    let send = ctx.send;
    view! {
        <Show when=move || ctx.open.get()>
            <div node_ref=content_ref
                role="dialog"
                aria-modal="true"
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        send.run(drawer::Event::CloseOnEscape);
                    }
                }
            >
                {children()}
            </div>
        </Show>
    }
}

#[component]
pub fn Backdrop() -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::Backdrop must be used inside Drawer");
    view! {
        <Show when=move || ctx.open.get()>
            <div
                aria-hidden="true"
                inert
                on:click=move |_| ctx.send.run(drawer::Event::CloseOnBackdropClick)
            />
        </Show>
    }
}

#[component]
pub fn CloseTrigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::CloseTrigger must be used inside Drawer");
    view! {
        <button type="button"
            on:click=move |_| ctx.send.run(drawer::Event::Close)
        >
            {children()}
        </button>
    }
}

#[component]
pub fn Title(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::Title must be used inside Drawer");
    view! { <h2 id=move || ctx.title_id.get()>{children()}</h2> }
}

#[component]
pub fn Description(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("drawer::Description must be used inside Drawer");
    view! { <div id=move || ctx.description_id.get()>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Drawer initialization
let machine = use_machine::<drawer::Machine>(core_props);
let ctx = create_drawer_context(machine);
provide_context(ctx);

// Controlled open sync
if let Some(open_sig) = controlled_open {
    create_controlled_open_effect(open_sig, machine.send);
}

// Portal rendering (Backdrop + Positioner + Content)
let portal_root = use_portal_root();  // from ArsProvider
let z_index = allocate_z_index();     // from ZIndexAllocator

// Presence composition for portal content
let presence = use_presence(machine.derive(|api| api.is_open()));

// --- Client-only effects (gated behind mount) ---

// Scroll lock effect
create_scroll_lock_effect(machine, props.prevent_scroll);

// Dialog stack push/pop for inert management
create_dialog_stack_effect(machine, props.id, props.modal);

// FocusScope activation (after animationstart)
let content_ref = NodeRef::<html::Div>::new();
create_focus_scope_effect(content_ref, machine, props.modal, props.initial_focus);

// Dismissable composition
let dismiss_handle = use_dismissable(content_ref, dismiss_props, inside_boundaries);

// Drag gesture wiring
let positioner_ref = NodeRef::<html::Div>::new();
let drag_handle_ref = NodeRef::<html::Div>::new();
create_drag_gesture_handler(
    content_ref, drag_handle_ref, positioner_ref,
    machine.send, props.snap_points, props.placement,
);

// DragHandle keyboard navigation
create_snap_keyboard_handler(drag_handle_ref, machine.send, props.snap_points);

// Focus restoration on close
create_focus_restoration_effect(machine, trigger_ref, props.restore_focus, props.final_focus);

// Cleanup
on_cleanup(move || {
    drag_gesture_teardown();
    dismiss_handle.teardown();
    focus_scope_deactivate();
    scroll_lock_restore();
    dialog_stack_pop();
    release_z_index(z_index);
    // Presence handles its own unmount
});
```

## 26. Adapter Invariants

- Backdrop and Positioner must be siblings inside the portal root (backdrop sibling pattern), never parent-child.
- FocusScope must not activate until the entry animation has started (`animationstart` event fires or no animation is configured).
- During the animation delay period, Content must have `tabindex="-1"` to prevent premature focus entry.
- Escape key must route to the topmost drawer only via the dialog stack, not to all open drawers.
- `dialog_stack_pop` must execute synchronously during the close transition effect so that a second Escape targets the correct overlay.
- Scroll lock is owned by the outermost modal in the dialog stack; inner drawers must not release scroll lock on close.
- CSS transform updates during drag must write directly to the DOM node, not through the reactive system.
- Velocity sampling must use the last 3--5 pointer positions, not an unbounded history.
- Rubber-band overdrag must use the 0.3 factor defined in the core spec.
- The `ars-touch-none` class must be applied to DragHandle and Content when snap points are configured.
- `overscroll-behavior: contain` must be set on Content when snap points are configured.
- Controlled open sync must use a deferred effect with previous-value tracking to avoid spurious Open/Close events on mount.
- `on_open_change` must fire after the state transition completes, not before.
- Portal content must not render during SSR when `lazy_mount` is true and `default_open` is false.
- All document-level listeners (drag, dismiss) must be cleaned up before the component unmounts.

## 27. Accessibility and SSR Notes

- Content must have `role="dialog"`, `aria-modal="true"`, `aria-roledescription` from `Messages.role_description`, `aria-labelledby` pointing to the Title ID, and `aria-describedby` pointing to the Description ID.
- CloseTrigger must have `aria-label` from `Messages.close_label`.
- DragHandle must have `role="slider"`, `aria-orientation="vertical"`, `aria-valuemin="0"`, `aria-valuemax`, `aria-valuenow`, and `aria-valuetext` when snap points are configured.
- Logical placements (Start/End) must resolve to physical directions based on `dir` before rendering.
- Title and Description IDs are generated deterministically from `ComponentIds` for hydration stability.
- SSR renders the inline Root and Trigger; portal content is SSR-safe empty unless `default_open` is true.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity, including all twelve parts, all placement options with logical resolution, bottom-sheet snap-point behavior, velocity-based snap targeting, rubber-band overdrag, keyboard snap navigation, drag-handle slider semantics, modal patterns (focus trap, scroll lock, inert, dialog stack), and Presence animation lifecycle.

Intentional deviations: none.

Traceability note: this adapter spec explicitly restates the following core adapter-owned concerns: portal rendering, backdrop sibling pattern, focus-scope activation timing, scroll-lock stacking, dialog-stack participation, inert-attribute management, dismiss-boundary composition, drag-gesture DOM wiring, velocity sampling, rubber-band factor, CSS transform direct-DOM writes, snap-point keyboard routing, slider ARIA repair on DragHandle, touch-action class application, controlled-open sync pattern, and `on_open_change` timing.

## 29. Test Scenarios

- Drawer opens and closes via Trigger toggle
- Drawer opens from each placement (Top, Bottom, Left, Right, Start, End) with correct slide direction
- Logical Start/End resolves to physical Left/Right based on `dir`
- Backdrop click closes the drawer when `close_on_backdrop` is true
- Backdrop click does not close when `close_on_backdrop` is false
- Escape closes the drawer when `close_on_escape` is true
- Escape does not close when `close_on_escape` is false
- Escape closes only the topmost drawer in a nested stack
- Focus is trapped within Content when `modal` is true
- Focus restores to trigger on close (or to `final_focus` if specified)
- Scroll lock applied on open, released on close
- Nested drawer does not release outer drawer's scroll lock
- CloseTrigger closes the drawer
- `on_open_change` fires with correct boolean value after transition
- Controlled `open` prop drives open/close state
- `lazy_mount` defers content rendering until first open
- `unmount_on_exit` removes content after close
- Drag gesture on Content initiates swipe-to-dismiss
- Drag past threshold closes the drawer
- Drag below threshold snaps back to open position
- Bottom-sheet snap points: drag between snap positions
- Velocity-based snap targeting selects correct snap point
- Rubber-band overdrag beyond extreme snap points
- DragHandle Arrow Up/Down navigates between snap points
- DragHandle Home/End navigates to extreme snap points
- DragHandle has slider ARIA attrs when snap points configured
- `data-ars-dragging` present on Content during drag
- CSS transition suppressed during drag via `data-ars-dragging`
- Portal content rendered inside ArsProvider portal root
- Z-index allocated from ZIndexAllocator
- `aria-roledescription` set from Messages
- `aria-label` on CloseTrigger set from Messages
- Title and Description IDs are hydration-stable

## 30. Test Oracle Notes

| Behavior                      | Preferred oracle type | Notes                                                                                    |
| ----------------------------- | --------------------- | ---------------------------------------------------------------------------------------- |
| Drawer open/close state       | DOM attrs             | Assert `data-ars-state="open"` / `"closed"` on Root and `aria-expanded` on Trigger.      |
| Placement and slide direction | DOM attrs             | Assert `data-ars-scope="drawer"` and CSS transform direction on Positioner.              |
| Focus trapping                | rendered structure    | Assert focus remains within Content when Tab is pressed.                                 |
| Focus restoration             | rendered structure    | Assert `document.activeElement` matches trigger or `final_focus` after close.            |
| Scroll lock                   | DOM attrs             | Assert body `overflow: hidden` when open, restored when closed.                          |
| Dialog stack and inert        | DOM attrs             | Assert sibling elements have `inert` when drawer is open; removed when closed.           |
| Backdrop dismiss              | callback order        | Assert `CloseOnBackdropClick` fires before `on_open_change(false)`.                      |
| Escape dismiss                | callback order        | Assert `CloseOnEscape` fires before `on_open_change(false)`.                             |
| Drag gesture state            | machine state         | Assert `Dragging(position)` during active drag.                                          |
| Snap resolution               | machine state         | Assert correct snap index after drag end with velocity.                                  |
| DragHandle slider ARIA        | DOM attrs             | Assert `role="slider"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax` on DragHandle. |
| `data-ars-dragging`           | DOM attrs             | Assert presence on Content during drag, absence when not dragging.                       |
| Portal rendering              | rendered structure    | Assert Backdrop and Content are siblings inside the portal root.                         |
| Cleanup                       | cleanup side effects  | Verify listeners, scroll lock, dialog stack, and z-index are released on unmount.        |
| Hydration stability           | hydration structure   | Assert Title and Description IDs match between SSR and client.                           |

Cheap verification recipe:

1. Mount a Drawer with a Trigger, Backdrop, Content, Title, and CloseTrigger. Assert the inline structure (Root, Trigger) is present and portal content is absent.
2. Click the Trigger. Assert portal content mounts with correct ARIA attrs, `data-ars-state="open"`, and `aria-expanded="true"` on Trigger. Assert focus moves into Content.
3. Press Escape. Assert drawer closes, focus returns to Trigger, `on_open_change(false)` fired, portal content unmounts (if `unmount_on_exit`).
4. For snap-point testing: mount with `placement=Bottom` and `snap_points=[0.25, 0.5, 1.0]`. Simulate drag and assert snap index changes.
5. Unmount the component and assert all cleanup side effects (listeners, scroll lock, dialog stack, z-index) are released.

## 31. Implementation Checklist

- [ ] Drawer provides Context context with machine state, send, IDs, placement, and snap state.
- [ ] Trigger renders a native `<button>` with `aria-haspopup="dialog"` and `aria-expanded`.
- [ ] Backdrop and Positioner/Content are siblings inside the portal root (backdrop sibling pattern).
- [ ] Content has `role="dialog"`, `aria-modal="true"`, `aria-roledescription`, `aria-labelledby`, `aria-describedby`.
- [ ] CloseTrigger has `aria-label` from `Messages.close_label`.
- [ ] DragHandle has `role="slider"` and slider ARIA attrs when snap points configured.
- [ ] Presence composes mount/unmount animation lifecycle for portal content.
- [ ] FocusScope activates after `animationstart`, not before.
- [ ] Content has `tabindex="-1"` during animation delay period.
- [ ] Scroll lock applied on open, released on close; nested modals do not release outer lock.
- [ ] Dialog stack push on open, pop on close; inert applied to correct siblings.
- [ ] Escape routes to topmost drawer only.
- [ ] Z-index allocated from ZIndexAllocator; fallback to static value if unavailable.
- [ ] Controlled `open` sync uses deferred effect with previous-value tracking.
- [ ] `on_open_change` fires after state transition completes.
- [ ] Drag gesture listeners attached on the client only; cleaned up on drag end and unmount.
- [ ] CSS transform during drag writes directly to DOM, not through reactive system.
- [ ] Velocity sampling uses last 3--5 positions; snap resolution uses velocity threshold.
- [ ] Rubber-band overdrag uses 0.3 factor.
- [ ] `ars-touch-none` class applied to DragHandle and Content when snap points configured.
- [ ] `overscroll-behavior: contain` set on Content when snap points configured.
- [ ] DragHandle keyboard navigation (Arrow Up/Down, Home/End) sends `SnapTo` events.
- [ ] `data-ars-dragging` present on Content during drag.
- [ ] Logical Start/End placement resolves to physical Left/Right based on `dir`.
- [ ] `lazy_mount` defers content rendering; `unmount_on_exit` removes content after close.
- [ ] Portal content is SSR-safe empty when not open.
- [ ] Title and Description IDs are hydration-stable.
- [ ] All document-level listeners cleaned up before unmount completes.
