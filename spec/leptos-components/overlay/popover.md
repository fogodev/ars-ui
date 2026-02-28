---
adapter: leptos
component: popover
category: overlay
source: components/overlay/popover.md
source_foundation: foundation/08-adapter-leptos.md
---

# Popover — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Popover`](../../components/overlay/popover.md) machine to Leptos 0.8.x. The adapter owns compound component context publication, portal rendering, positioning engine integration via `ars-dom`, z-index allocation via `ZIndexAllocator`, click-outside race prevention (rAF-deferred listener attachment), dismiss-button rendering, focus management via `FocusScope::popover()` composition, CSS custom property application on the positioner element, `Presence` composition for mount/unmount animation lifecycle, and cleanup of all positioning subscriptions, click-outside listeners, and allocated z-index layers.

## 2. Public Adapter API

```rust
#[component] pub fn Popover(...) -> impl IntoView
#[component] pub fn Trigger(children: Children) -> impl IntoView
#[component] pub fn Anchor(children: Children) -> impl IntoView
#[component] pub fn Positioner(children: Children) -> impl IntoView
#[component] pub fn Content(children: Children) -> impl IntoView
#[component] pub fn Arrow() -> impl IntoView
#[component] pub fn Title(children: Children) -> impl IntoView
#[component] pub fn Description(children: Children) -> impl IntoView
#[component] pub fn CloseTrigger(children: Children) -> impl IntoView
```

`Popover` accepts the full core prop set: `id`, `open` (controlled `Option<Signal<bool>>`), `default_open`, `modal`, `close_on_escape`, `close_on_interact_outside`, `positioning`, `offset`, `cross_offset`, `same_width`, `portal`, `lazy_mount`, `unmount_on_exit`, `on_open_change`, `messages`, and `locale`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props` struct including convenience offset aliases.
- Event parity: `Open`, `Close`, `Toggle`, `CloseOnEscape`, `CloseOnInteractOutside`, and `PositioningUpdate` all map through the adapter.
- Structure parity: all nine core parts are rendered as separate compound components. Dismiss buttons are adapter-owned inside content.

## 4. Part Mapping

| Core part / structure | Required?               | Adapter rendering target                           | Ownership                         | Attr source                                      | Notes                                     |
| --------------------- | ----------------------- | -------------------------------------------------- | --------------------------------- | ------------------------------------------------ | ----------------------------------------- |
| `Root`                | required                | wrapper `<div>`                                    | adapter-owned                     | `api.root_attrs()`                               | Context provider, no portal.              |
| `Trigger`             | required                | native `<button>` wrapping consumer children       | adapter-owned                     | `api.trigger_attrs()`                            | Click toggles open state.                 |
| `Anchor`              | optional                | consumer-chosen element                            | consumer-owned with adapter attrs | `api.anchor_attrs()`                             | Alternative positioning reference.        |
| `Positioner`          | required                | `<div>` inside portal root                         | adapter-owned                     | `api.positioner_attrs()` + CSS custom properties | Positioned absolutely via `ars-dom`.      |
| `Content`             | required                | `<div>` inside positioner                          | adapter-owned                     | `api.content_attrs()`                            | `role="group"` or `role="dialog"`.        |
| `Arrow`               | optional                | `<div>` inside positioner                          | adapter-owned                     | `api.arrow_attrs()`                              | Arrow position via CSS custom properties. |
| `Title`               | optional                | consumer-chosen element                            | consumer-owned with adapter attrs | `api.title_attrs()`                              | Registers title ID in context.            |
| `Description`         | optional                | consumer-chosen element                            | consumer-owned with adapter attrs | `api.description_attrs()`                        | Registers description ID in context.      |
| `CloseTrigger`        | optional                | native `<button>` wrapping consumer children       | adapter-owned                     | `api.close_trigger_attrs()`                      | `aria-label` from resolved messages.      |
| DismissButton (start) | required when non-modal | visually hidden `<button>` before content children | adapter-owned                     | `dismiss_button_attrs()`                         | Screen reader close mechanism.            |
| DismissButton (end)   | required when non-modal | visually hidden `<button>` after content children  | adapter-owned                     | `dismiss_button_attrs()`                         | Second screen reader close mechanism.     |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                  | Adapter-owned attrs                                                                                                                                                                                                 | Consumer attrs               | Merge order                                                                                         | Ownership notes                        |
| -------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `Root`         | `api.root_attrs()`          | `data-ars-state`                                                                                                                                                                                                    | consumer root attrs          | core scope/part/state attrs win; `class`/`style` merge additively                                   | adapter-owned wrapper                  |
| `Trigger`      | `api.trigger_attrs()`       | `type="button"`                                                                                                                                                                                                     | consumer trigger attrs       | core `aria-expanded`/`aria-controls` win                                                            | adapter-owned native button            |
| `Anchor`       | `api.anchor_attrs()`        | none                                                                                                                                                                                                                | consumer anchor attrs        | core scope/part attrs win; consumer owns element choice                                             | consumer-owned element                 |
| `Positioner`   | `api.positioner_attrs()`    | CSS custom properties (`--ars-x`, `--ars-y`, `--ars-z-index`, `--ars-reference-width`, `--ars-reference-height`, `--ars-available-width`, `--ars-available-height`, `--ars-transform-origin`), `position: absolute` | none                         | core placement attrs win; CSS vars are adapter-owned                                                | adapter-owned portal child             |
| `Content`      | `api.content_attrs()`       | `tabindex="-1"`, focus-scope composition attrs                                                                                                                                                                      | consumer content attrs       | core `role`/`aria-modal`/`aria-labelledby`/`aria-describedby` win; `class`/`style` merge additively | adapter-owned                          |
| `Arrow`        | `api.arrow_attrs()`         | arrow CSS custom properties (`--ars-arrow-x`, `--ars-arrow-y`)                                                                                                                                                      | consumer arrow attrs         | core arrow position attrs win                                                                       | adapter-owned                          |
| `Title`        | `api.title_attrs()`         | none                                                                                                                                                                                                                | consumer title attrs         | core `id` wins                                                                                      | consumer-owned element with adapter ID |
| `Description`  | `api.description_attrs()`   | none                                                                                                                                                                                                                | consumer description attrs   | core `id` wins                                                                                      | consumer-owned element with adapter ID |
| `CloseTrigger` | `api.close_trigger_attrs()` | `type="button"`                                                                                                                                                                                                     | consumer close-trigger attrs | core `aria-label` wins                                                                              | adapter-owned native button            |
| DismissButton  | `dismiss_button_attrs()`    | visually hidden styles                                                                                                                                                                                              | none                         | core attrs win                                                                                      | adapter-owned, not consumer-exposed    |

- Consumers must not override `role`, `aria-modal`, `aria-expanded`, `aria-controls`, `aria-labelledby`, or `aria-describedby` on their respective parts.
- CSS custom properties on the positioner are the adapter's positioning output channel; consumers must not override them but may read them.

## 6. Composition / Context Contract

`Popover` publishes a `Context` context consumed by all child parts. The context carries: machine send handle, open state signal, trigger ref, content ref, anchor ref (optional), title ID, description ID, service handle, and context version.

Composition dependencies:

- **Dismissable**: `Content` composes `Dismissable` for outside-interaction detection. The trigger element and any anchor element are registered as inside boundaries so clicks on them do not dismiss the popover.
- **FocusScope**: `Content` composes `FocusScope::popover()` preset (`contain=false`, `restore_focus=true`). Focus moves to the first tabbable element on open, restores to trigger on close. Focus is NOT trapped.
- **ArsProvider**: Portal rendering reads the portal root from `ArsContext`.
- **ZIndexAllocator**: The positioner allocates a z-index layer on open and releases it on close.
- **Presence**: When `lazy_mount` or `unmount_on_exit` is enabled, `Presence` manages the content mount/unmount lifecycle for animation support.
- **ArsProvider**: Messages and locale resolution use the nearest `ArsProvider` context when `locale` is `None`.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled open state is not supported after mount. `default_open` is init-only. When `open` is `Some(signal)`, the adapter watches it via a deferred effect and dispatches `Open`/`Close` events on change.

| Adapter prop                | Mode                      | Sync trigger              | Machine event / update path                 | Visible effect                         | Notes                                |
| --------------------------- | ------------------------- | ------------------------- | ------------------------------------------- | -------------------------------------- | ------------------------------------ |
| `open`                      | controlled                | signal change after mount | `Open` / `Close`                            | opens or closes popover                | deferred effect, not body-level sync |
| `default_open`              | uncontrolled init         | initial render only       | initial machine props                       | seeds internal open state              | read once at initialization          |
| `modal`                     | non-reactive adapter prop | render time only          | initial machine props                       | determines `role` and focus behavior   | dynamic change not supported         |
| `close_on_escape`           | non-reactive adapter prop | render time only          | guards `CloseOnEscape` transition           | controls Escape dismissal              | static configuration                 |
| `close_on_interact_outside` | non-reactive adapter prop | render time only          | guards `CloseOnInteractOutside` transition  | controls outside-click dismissal       | static configuration                 |
| `positioning`               | non-reactive adapter prop | render time only          | positioning engine configuration            | determines placement, flip, boundary   | reconfiguration requires remount     |
| `offset` / `cross_offset`   | non-reactive adapter prop | render time only          | merged into `positioning`                   | populates positioning offset fields    | convenience aliases                  |
| `same_width`                | non-reactive adapter prop | render time only          | sets `min-width` on positioner              | popover matches trigger width          | uses trigger `offsetWidth`           |
| `portal`                    | non-reactive adapter prop | render time only          | portal rendering decision                   | content renders inside or outside tree | default `true`                       |
| `lazy_mount`                | non-reactive adapter prop | render time only          | Presence configuration                      | defers first mount until open          | works with Presence                  |
| `unmount_on_exit`           | non-reactive adapter prop | render time only          | Presence configuration                      | removes content DOM after close        | works with Presence                  |
| `on_open_change`            | callback                  | after state transition    | invoked by adapter after machine transition | notifies consumer of open state        | fires with new boolean               |
| `messages`                  | non-reactive adapter prop | render time only          | resolved via `resolve_messages`             | dismiss button labels                  | merged with defaults                 |
| `locale`                    | non-reactive adapter prop | render time only          | resolved via `resolve_locale`               | locale for message formatting          | falls back to context                |

| UI event                    | Preconditions                                                   | Machine event / callback path | Ordering notes                               | Notes                         |
| --------------------------- | --------------------------------------------------------------- | ----------------------------- | -------------------------------------------- | ----------------------------- |
| trigger click               | trigger interactive                                             | `Toggle`                      | fires before `on_open_change`                | native button click           |
| `Escape` on content         | open and `close_on_escape` enabled                              | `CloseOnEscape`               | Escape check runs before outside-interaction | keydown on content element    |
| outside pointer interaction | open and `close_on_interact_outside` enabled, rAF guard elapsed | `CloseOnInteractOutside`      | rAF-deferred attachment prevents race        | Dismissable-composed          |
| close-trigger click         | close trigger interactive                                       | `Close`                       | fires before `on_open_change`                | native button click           |
| dismiss-button activation   | dismiss button rendered and interactive                         | `Close`                       | dismiss normalized before `on_open_change`   | visually hidden button        |
| positioning update          | content mounted and positioning engine active                   | `PositioningUpdate(result)`   | context-only update, no state change         | CSS custom properties updated |

## 8. Registration and Cleanup Contract

| Registered entity           | Registration trigger             | Identity key                    | Cleanup trigger                                       | Cleanup action                             | Notes                 |
| --------------------------- | -------------------------------- | ------------------------------- | ----------------------------------------------------- | ------------------------------------------ | --------------------- |
| positioning subscription    | content mount on client          | popover instance                | close or component cleanup                            | unsubscribe positioning engine             | client-only           |
| click-outside listener      | open transition, deferred by rAF | popover instance                | close, `CloseOnInteractOutside`, or component cleanup | cancel pending rAF and remove listener     | rAF guard required    |
| z-index allocation          | open transition                  | popover instance                | close or component cleanup                            | release allocated z-index layer            | via ZIndexAllocator   |
| focus-scope registration    | content mount on client          | popover instance via FocusScope | close or component cleanup                            | unregister scope and restore focus         | FocusScope::popover() |
| Dismissable registration    | content mount on client          | popover instance                | close or component cleanup                            | remove dismiss listeners and boundary refs | Dismissable-composed  |
| portal mount                | content mount with `portal=true` | popover instance                | close or component cleanup                            | remove portal content from portal root     | via ArsProvider       |
| title ID registration       | Title mount                      | title instance                  | Title unmount                                         | clear title_id in context                  | optional part         |
| description ID registration | Description mount                | description instance            | Description unmount                                   | clear description_id in context            | optional part         |

## 9. Ref and Node Contract

| Target part / node     | Ref required?     | Ref owner                               | Node availability                  | Composition rule                              | Notes                                                   |
| ---------------------- | ----------------- | --------------------------------------- | ---------------------------------- | --------------------------------------------- | ------------------------------------------------------- |
| Trigger button         | yes               | adapter-owned                           | required after mount               | compose only if consumer needs the ref        | Positioning anchor reference when no Anchor part.       |
| Anchor element         | yes when rendered | consumer-owned, registered with adapter | client-only                        | registered as alternate positioning reference | Overrides trigger as anchor.                            |
| Positioner `<div>`     | yes               | adapter-owned                           | client-only                        | no consumer composition                       | Receives CSS custom properties from positioning engine. |
| Content `<div>`        | yes               | adapter-owned                           | client-only                        | compose only if consumer needs the ref        | Focus target, Dismissable root, click-outside boundary. |
| Arrow `<div>`          | yes when rendered | adapter-owned                           | client-only                        | no consumer composition                       | Arrow position computed by positioning engine.          |
| DismissButton elements | no                | adapter-owned                           | always structural, handle optional | no composition                                | Semantics do not depend on stored refs.                 |

## 10. State Machine Boundary Rules

- machine-owned state: open/closed state, modal flag, positioning result cache, title/description ID presence, and all transition guards.
- adapter-local derived bookkeeping: trigger ref, content ref, anchor ref, positioner ref, arrow ref, click-outside guard handles (pending rAF, active listener), z-index allocation handle, positioning subscription handle, portal mount handle, and Presence animation state.
- forbidden local mirrors: do not keep an unsynchronized open flag separate from the machine state or controlled signal. Do not cache positioning results outside the machine context.
- allowed snapshot-read contexts: render derivation, click handlers, keydown handlers, positioning callbacks, focus-scope activation, Dismissable callbacks, and cleanup.

## 11. Callback Payload Contract

| Callback                          | Payload source             | Payload shape             | Timing                             | Cancelable?           | Notes                           |
| --------------------------------- | -------------------------- | ------------------------- | ---------------------------------- | --------------------- | ------------------------------- |
| `on_open_change`                  | machine-derived snapshot   | `bool` (new open state)   | after machine transition completes | no                    | Fires for all open/close paths. |
| Dismissable `on_interact_outside` | normalized adapter payload | outside interaction event | before dismiss decision            | yes (via Dismissable) | Consumers can veto dismissal.   |
| Dismissable `on_escape_key_down`  | raw framework event        | keyboard event            | before dismiss decision            | yes (via Dismissable) | Only fires on the client.       |

## 12. Failure and Degradation Rules

| Condition                                    | Policy             | Notes                                                                          |
| -------------------------------------------- | ------------------ | ------------------------------------------------------------------------------ |
| trigger ref missing after mount              | fail fast          | Positioning anchor requires a concrete node.                                   |
| content ref missing after mount when open    | fail fast          | Dismissable containment and focus scope require a concrete node.               |
| positioner ref missing after mount when open | fail fast          | CSS custom property application requires a concrete node.                      |
| portal root unavailable                      | degrade gracefully | Render content inline and emit a debug warning.                                |
| positioning engine unavailable or error      | degrade gracefully | Render positioner without computed position; content visible but unpositioned. |
| ZIndexAllocator context missing              | degrade gracefully | Use fallback z-index and emit a debug warning.                                 |
| browser APIs absent during SSR               | no-op              | Render trigger structure; defer content, positioning, and listeners to client. |
| anchor ref missing when Anchor part rendered | warn and ignore    | Fall back to trigger as positioning reference.                                 |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source               | Duplicates allowed?     | DOM order must match registration order? | SSR/hydration stability                                   | Notes                                 |
| -------------------------------- | ----------------------------- | ----------------------- | ---------------------------------------- | --------------------------------------------------------- | ------------------------------------- |
| popover instance                 | data-derived (from `id` prop) | no                      | not applicable                           | ID must remain stable across hydration                    | Machine uses `ComponentIds::from_id`. |
| title ID registration            | instance-derived              | no                      | not applicable                           | title structure must remain stable across hydration       | Optional part, registers on mount.    |
| description ID registration      | instance-derived              | no                      | not applicable                           | description structure must remain stable across hydration | Optional part, registers on mount.    |
| z-index allocation               | instance-derived              | yes (multiple overlays) | not applicable                           | client-only                                               | Released on close/cleanup.            |
| dismiss buttons                  | not applicable                | yes (paired)            | yes (start before end)                   | both must render consistently                             | Structural contract.                  |

## 14. SSR and Client Boundary Rules

- SSR renders the `Root` wrapper and `Trigger` button with correct `aria-expanded="false"` (or `"true"` if `default_open` is true).
- SSR does NOT render portal content, positioner, or content parts when closed. When `default_open` is true, SSR renders the content structure without positioning or listeners.
- Positioning engine, click-outside listeners, z-index allocation, focus-scope activation, and Dismissable listeners are all client-only.
- Trigger and content refs are server-safe absent and required after mount.
- Portal rendering is client-only; the portal root node does not exist during SSR.
- No dismiss callback, positioning callback, or `on_open_change` may fire during SSR.
- Hydration must match: if `default_open` is true, the server-rendered content structure must match what the client expects to hydrate.

## 15. Performance Constraints

- Positioning subscriptions must only register while the popover is open; they must not remain active when closed.
- Click-outside listeners must detach on close, not persist idle on the document.
- Z-index allocation and release must be paired; leaked allocations degrade the stacking order for other overlays.
- CSS custom property application on the positioner should batch with the positioning callback, not trigger additional re-renders.
- Title and description ID registration should update only on mount/unmount of those parts, not on every render.
- `derive()` calls should be scoped to the minimum necessary data to avoid unnecessary re-renders of child parts.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                           | Notes                                                 |
| ------------------- | ----------- | ----------------------- | ----------------------------------------------------------------- | ----------------------------------------------------- |
| `presence`          | required    | composition contract    | Animation lifecycle for lazy mount and unmount on exit.           | Must be implemented before popover content rendering. |
| `dismissable`       | required    | composition contract    | Outside-interaction detection for click-outside and Escape.       | Popover content composes Dismissable.                 |
| `focus-scope`       | required    | behavioral prerequisite | Non-trapping focus management on open/close.                      | Uses `FocusScope::popover()` preset.                  |
| `ars-provider`      | required    | context contract        | Portal root resolution and scoped DOM access.                     | Popover content renders into portal root.             |
| `z-index-allocator` | required    | context contract        | Dynamic z-index layer allocation for stacking order.              | Allocated on open, released on close.                 |
| `button`            | recommended | behavioral prerequisite | Trigger and close-trigger share button-like activation semantics. | Reuse normalized interaction patterns.                |

## 17. Recommended Implementation Sequence

1. Initialize the popover machine with `use_machine::<popover::Machine>(props)` and publish `Context` context.
2. Render `Popover` wrapper and `Trigger` with click handler dispatching `Toggle`.
3. Wire controlled open-state sync via deferred effect when `open` is `Some(signal)`.
4. Implement portal rendering for `Positioner` using `ArsProvider` portal root.
5. Allocate z-index from `ZIndexAllocator` on open transition; release on close.
6. Integrate positioning engine (`ars-dom`) between trigger/anchor ref and positioner ref; apply CSS custom properties on each positioning update.
7. Compose `Dismissable` inside `Content` with trigger (and anchor, if present) registered as inside boundaries. Implement rAF-deferred click-outside guard.
8. Compose `FocusScope::popover()` inside `Content` for non-trapping focus with restore-on-close.
9. Render paired dismiss buttons (start and end) inside content when non-modal.
10. Implement `Arrow` with arrow position CSS custom properties.
11. Implement `Title` and `Description` with context-based ID registration.
12. Compose `Presence` for `lazy_mount` and `unmount_on_exit` behavior.
13. Wire `on_open_change` callback after machine transitions.
14. Verify cleanup ordering: positioning unsubscribe, click-outside listener removal, z-index release, focus restore, Dismissable teardown, portal unmount.

## 18. Anti-Patterns

- Do not attach click-outside listeners synchronously on open; the triggering click will bubble and immediately close the popover. Use the rAF-deferred guard.
- Do not keep positioning subscriptions active when the popover is closed.
- Do not leak z-index allocations by forgetting to release on close or cleanup.
- Do not hardcode z-index values; always use `ZIndexAllocator`.
- Do not render popover content inline when `portal=true`; it must render into the portal root.
- Do not trap focus in non-modal popovers; use `FocusScope::popover()` with `contain=false`.
- Do not omit dismiss buttons in non-modal mode; screen reader users depend on them to close the popover.
- Do not fork open state into a local signal that diverges from the machine state or controlled prop.
- Do not fire `on_open_change` during SSR.
- Do not render portal content during SSR when the portal root does not exist.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the popover content renders in a portal root outside the component tree when `portal=true`.
- Consumers may assume focus moves to the first tabbable element inside the popover on open, and restores to the trigger on close.
- Consumers may assume Tab is not trapped; it flows naturally through the popover content and then continues into the page.
- Consumers may assume the positioner element carries CSS custom properties reflecting the current computed position.
- Consumers may assume `on_open_change` fires after every open/close transition regardless of the trigger source.
- Consumers may assume paired dismiss buttons are present in non-modal mode for screen reader accessibility.
- Consumers must not assume the popover content exists in the DOM when closed (it may be unmounted depending on `unmount_on_exit` and `lazy_mount`).
- Consumers must not assume positioning runs during SSR.
- Consumers must not assume the z-index value is stable across different open/close cycles.

## 20. Platform Support Matrix

| Capability / behavior            | Browser client | SSR            | Notes                                                    |
| -------------------------------- | -------------- | -------------- | -------------------------------------------------------- |
| trigger rendering and aria attrs | full support   | full support   | Trigger renders on server with correct aria-expanded.    |
| portal content rendering         | full support   | SSR-safe empty | Content deferred to client when portal root unavailable. |
| positioning engine integration   | full support   | client-only    | ars-dom requires live DOM measurements.                  |
| click-outside detection          | full support   | client-only    | Dismissable listeners are client-only.                   |
| z-index allocation               | full support   | client-only    | ZIndexAllocator is client-only.                          |
| focus management                 | full support   | client-only    | FocusScope activation is client-only.                    |
| dismiss buttons                  | full support   | full support   | Rendered in server HTML for non-modal.                   |
| Presence animation               | full support   | SSR-safe empty | Animation lifecycle is client-only.                      |
| `on_open_change` callback        | full support   | client-only    | Never fires during SSR.                                  |
| `same_width` trigger measurement | full support   | client-only    | Requires live `offsetWidth` measurement.                 |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior | Production behavior | Notes                                             |
| ---------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------- |
| portal root unavailable                        | debug warning        | degrade gracefully  | Render content inline as fallback.                |
| ZIndexAllocator context missing                | debug warning        | degrade gracefully  | Use fallback z-index value.                       |
| trigger ref missing after mount                | fail fast            | fail fast           | Positioning cannot function without anchor.       |
| content ref missing after mount when open      | fail fast            | fail fast           | Dismissable and focus scope require content node. |
| positioning engine error                       | debug warning        | degrade gracefully  | Content visible but unpositioned.                 |
| anchor ref missing when Anchor rendered        | debug warning        | warn and ignore     | Fall back to trigger ref.                         |
| click-outside rAF cancelled due to rapid close | no-op                | no-op               | Expected behavior during rapid interactions.      |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                                      | Reused by                                                     | Notes                                        |
| -------------------------- | ----------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------- |
| positioning helper         | required    | Subscribe to positioning engine, apply CSS custom properties to positioner, compute arrow position. | `popover`, `tooltip`, `hover-card`, `tour`                    | Shared across all positioned overlays.       |
| portal helper              | required    | Render content into the portal root from ArsProvider context.                                       | `popover`, `dialog`, `tooltip`, `hover-card`, `toast`, `tour` | Shared across all portal-rendering overlays. |
| z-index helper             | required    | Allocate and release z-index layers via ZIndexAllocator.                                            | `popover`, `dialog`, `tooltip`, `hover-card`, `toast`, `tour` | Paired allocation/release lifecycle.         |
| click-outside guard helper | required    | Implement rAF-deferred click-outside listener attachment and cleanup.                               | `popover`, `hover-card`                                       | Prevents open-click race condition.          |
| dismiss-button helper      | required    | Render paired visually hidden dismiss buttons with correct attrs.                                   | `popover`, `hover-card`, `tooltip`                            | Non-modal overlays only.                     |
| merge helper               | recommended | Merge core attrs, adapter attrs, and consumer attrs with documented precedence.                     | all overlay components                                        | Prevents accidental attr override.           |
| measurement helper         | recommended | Measure trigger `offsetWidth` for `same_width` support.                                             | `popover`, `combobox`                                         | Client-only DOM measurement.                 |

## 23. Framework-Specific Behavior

Leptos uses `on_cleanup` for teardown of positioning subscriptions, click-outside listeners, z-index allocations, and Dismissable registrations. Controlled open state is watched via a deferred `Effect::new` (not body-level sync) because the effect dispatches `Open`/`Close` events. Portal rendering uses Leptos `Portal` or manual DOM insertion into the `ArsProvider` portal root node. `NodeRef<html::Div>` and `NodeRef<html::Button>` are used for positioner, content, trigger, and arrow refs. Context is published via `provide_context(Context { ... })` and consumed via `use_context::<Context>().expect("...")`.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Popover(
    #[prop(into)] id: String,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional)] default_open: bool,
    #[prop(optional)] modal: bool,
    #[prop(optional, default = true)] close_on_escape: bool,
    #[prop(optional, default = true)] close_on_interact_outside: bool,
    #[prop(optional)] positioning: popover::PositioningOptions,
    #[prop(optional)] offset: f64,
    #[prop(optional)] cross_offset: f64,
    #[prop(optional)] same_width: bool,
    #[prop(optional, default = true)] portal: bool,
    #[prop(optional)] lazy_mount: bool,
    #[prop(optional)] unmount_on_exit: bool,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] messages: Option<popover::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView {
    let props = popover::Props {
        id,
        open: open.map(|s| s.get_untracked()),
        default_open,
        modal,
        close_on_escape,
        close_on_interact_outside,
        positioning,
        offset,
        cross_offset,
        same_width,
        portal,
        lazy_mount,
        unmount_on_exit,
        on_open_change: on_open_change.clone(),
        messages,
        locale,
    };

    let machine = use_machine::<popover::Machine>(props);
    let is_open = machine.derive(|api| api.is_open());
    let root_attrs = machine.derive(|api| api.root_attrs());

    let trigger_ref = NodeRef::<html::Button>::new();
    let content_ref = NodeRef::<html::Div>::new();
    let anchor_ref: RwSignal<Option<NodeRef<html::Div>>> = RwSignal::new(None);

    // Watch controlled open signal
    if let Some(open_sig) = open {
        let send = machine.send;
        let mut prev_open: RwSignal<Option<bool>> = RwSignal::new(None);
        Effect::new(move |_| {
            let new_open = open_sig.get();
            let prev = prev_open.get();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send.run(popover::Event::Open);
                    } else {
                        send.run(popover::Event::Close);
                    }
                }
                prev_open.set(Some(new_open));
            }
        });
    }

    // Fire on_open_change after transitions
    if let Some(cb) = on_open_change {
        let open_derived = is_open;
        Effect::new(move |_| {
            cb.run(open_derived.get());
        });
    }

    provide_context(Context {
        send: machine.send,
        is_open,
        trigger_ref,
        content_ref,
        anchor_ref,
        service: machine.service,
        context_version: machine.context_version,
        modal,
        portal,
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
        .expect("popover::Trigger must be used inside Popover");
    let trigger_attrs = /* machine.derive(|api| api.trigger_attrs()) */;

    view! {
        <button
            node_ref=ctx.trigger_ref
            {..trigger_attrs.get()}
            on:click=move |_| ctx.send.run(popover::Event::Toggle)
        >
            {children()}
        </button>
    }
}

#[component]
pub fn Content(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("popover::Content must be used inside Popover");
    let content_attrs = /* machine.derive(|api| api.content_attrs()) */;
    let send = ctx.send;
    let dismiss_label = /* machine.derive(|api| api.dismiss_label()) */;

    // Compose: Dismissable for outside-interaction
    // Compose: FocusScope::popover() for non-trapping focus
    // Render: paired dismiss buttons for non-modal

    view! {
        <div
            node_ref=ctx.content_ref
            {..content_attrs.get()}
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    send.run(popover::Event::CloseOnEscape);
                }
            }
        >
            <Show when=move || !ctx.modal>
                <button
                    style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)"
                    on:click=move |_| send.run(popover::Event::Close)
                >
                    {dismiss_label.get()}
                </button>
            </Show>
            {children()}
            <Show when=move || !ctx.modal>
                <button
                    style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)"
                    on:click=move |_| send.run(popover::Event::Close)
                >
                    {dismiss_label.get()}
                </button>
            </Show>
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
// Popover
let machine = use_machine::<popover::Machine>(props);
let trigger_ref = create_trigger_ref();
let content_ref = create_content_ref();
let positioner_ref = create_positioner_ref();
let anchor_ref = create_optional_anchor_ref();

publish_popover_context(machine, trigger_ref, content_ref, anchor_ref);
watch_controlled_open_signal(open_signal, machine.send);
fire_on_open_change_after_transitions(machine, on_open_change);

render_root_wrapper(machine.derive(|api| api.root_attrs()));

// Trigger
let ctx = consume_popover_context();
render_native_button(ctx.trigger_ref, trigger_attrs, toggle_handler);

// Positioner (client-only rendering inside portal)
let ctx = consume_popover_context();
let z_index = allocate_z_index_on_open(ctx.is_open);
let positioning_sub = subscribe_positioning_engine(
    ctx.trigger_ref_or_anchor_ref(),
    positioner_ref,
    positioning_options,
);
apply_css_custom_properties(positioner_ref, positioning_sub);
apply_same_width_measurement(trigger_ref, positioner_ref, same_width);

render_in_portal_or_inline(ctx.portal, || {
    render_positioner(positioner_ref, positioner_attrs, z_index);
});

// Content
let ctx = consume_popover_context();
let dismiss_handle = compose_dismissable(
    ctx.content_ref,
    vec![ctx.trigger_ref, ctx.anchor_ref],
    ctx.send,
);
let focus_scope_handle = compose_focus_scope_popover(ctx.content_ref);

render_content_with_dismiss_buttons(ctx.content_ref, content_attrs, ctx.modal);
wire_escape_keydown(ctx.content_ref, ctx.send);

// Arrow
let arrow_ref = create_arrow_ref();
apply_arrow_css_custom_properties(arrow_ref, positioning_sub);
render_arrow(arrow_ref, arrow_attrs);

// Title / Description
register_title_id_in_context(ctx);
register_description_id_in_context(ctx);

// Presence composition (wraps Positioner or Content)
compose_presence(ctx.is_open, lazy_mount, unmount_on_exit);

on_cleanup(|| {
    positioning_sub.unsubscribe();
    dismiss_handle.teardown();
    focus_scope_handle.teardown();
    z_index.release();
    portal_handle.unmount();
    cancel_pending_raf_click_outside_guard();
});
```

## 26. Adapter Invariants

- Click-outside listener attachment MUST be deferred by one `requestAnimationFrame` after the open transition. Synchronous attachment causes the triggering click to immediately close the popover.
- If the state transitions to `Closed` before the deferred rAF callback fires, the pending listener attachment MUST be cancelled.
- Existing click-outside listeners MUST be removed BEFORE attaching new ones during state transitions to prevent duplicate listeners.
- Positioning subscriptions MUST only be active while the popover is open. They MUST be unsubscribed on close and on cleanup.
- Z-index allocations MUST be released on close and on component cleanup. Leaked allocations corrupt the stacking order.
- Portal content MUST render into the `ArsProvider` portal root, not inline with the trigger.
- Non-modal popovers MUST render paired dismiss buttons (start and end) inside the content for screen reader accessibility.
- Focus MUST move to the first tabbable element inside the content on open. Focus MUST restore to the trigger on close.
- Focus MUST NOT be trapped in non-modal popovers. Tab flows naturally through content and into the page.
- Modal popovers MUST use `role="dialog"` with `aria-modal="true"` and trap focus.
- The adapter MUST NOT fire `on_open_change` or any dismiss callback during SSR.
- Controlled open-state sync MUST use a deferred effect, not body-level sync, because it dispatches events.
- CSS custom properties on the positioner (`--ars-x`, `--ars-y`, `--ars-z-index`, etc.) MUST be updated on each positioning engine callback and MUST NOT be set during SSR.

## 27. Accessibility and SSR Notes

- Non-modal popovers use `role="group"` to avoid confusing screen readers that announce "dialog" and set user expectations for focus trapping. Modal popovers use `role="dialog"` with `aria-modal="true"`.
- `aria-expanded` on the trigger reflects the open state. `aria-controls` points to the content ID when open.
- `aria-labelledby` and `aria-describedby` on the content reference the Title and Description part IDs when those parts are rendered.
- `tabindex="-1"` on the content allows programmatic focus when no tabbable children exist.
- Paired dismiss buttons provide a screen-reader-discoverable close mechanism for non-modal popovers. They use `aria-label` from the resolved `Messages.dismiss_label`.
- SSR renders the trigger with correct ARIA attributes. Content structure is deferred to the client unless `default_open` is true, in which case the content skeleton is server-rendered for hydration stability but without positioning or listeners.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, part, and behavior parity including all nine compound component parts, click-outside race prevention, positioning engine integration, z-index allocation, focus management, dismiss buttons, Presence composition, and portal rendering.

Intentional deviations: none.

Traceability note: This adapter spec makes explicit the core adapter-owned concerns for rAF-deferred click-outside guard, positioning engine CSS custom property application, portal rendering via ArsProvider, z-index allocation lifecycle, FocusScope::popover() composition, paired dismiss-button rendering, Presence animation lifecycle, controlled open-state sync via deferred effect, and cleanup ordering for all registered resources.

## 29. Test Scenarios

- trigger click toggles popover open/closed
- controlled open signal syncs with machine state
- Escape key closes the popover when `close_on_escape` is true
- outside click closes the popover when `close_on_interact_outside` is true
- click-outside does NOT fire on the same click that opened the popover (rAF guard)
- rapid open/close cancels pending rAF listener attachment
- close-trigger click dispatches Close event
- dismiss buttons rendered in non-modal mode, absent in modal mode
- dismiss button click closes the popover
- focus moves to first tabbable element on open
- focus restores to trigger on close
- Tab is not trapped in non-modal mode
- focus is trapped in modal mode
- positioner receives CSS custom properties after positioning update
- arrow receives position CSS custom properties
- content renders in portal root when `portal=true`
- z-index allocated on open, released on close
- `same_width` sets positioner min-width to trigger offsetWidth
- `lazy_mount` defers first content render until open
- `unmount_on_exit` removes content from DOM after close
- `on_open_change` fires after every open/close transition
- Title and Description register their IDs in context
- `aria-expanded`, `aria-controls`, `role`, `aria-modal`, `aria-labelledby`, `aria-describedby` all correct
- SSR renders trigger but defers content to client
- cleanup removes all listeners, subscriptions, and allocations

## 30. Test Oracle Notes

| Behavior                          | Preferred oracle type | Notes                                                                                          |
| --------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------- |
| trigger aria attrs                | DOM attrs             | Assert `aria-expanded` and `aria-controls` values.                                             |
| content role and aria             | DOM attrs             | Assert `role="group"` or `role="dialog"`, `aria-modal`, `aria-labelledby`, `aria-describedby`. |
| open/close state                  | machine state         | Assert machine state transitions via `is_open()`.                                              |
| click-outside race prevention     | callback order        | Assert no `CloseOnInteractOutside` fires on the opening click.                                 |
| positioning CSS custom properties | DOM attrs             | Assert `--ars-x`, `--ars-y`, etc. on positioner element.                                       |
| dismiss button presence           | rendered structure    | Assert both dismiss buttons present in non-modal, absent in modal.                             |
| focus movement on open            | DOM attrs             | Assert `document.activeElement` is first tabbable child or content.                            |
| focus restore on close            | DOM attrs             | Assert `document.activeElement` is trigger after close.                                        |
| z-index lifecycle                 | cleanup side effects  | Assert z-index allocated on open and released on close/cleanup.                                |
| portal rendering                  | rendered structure    | Assert content is a child of the portal root node.                                             |
| controlled signal sync            | machine state         | Assert machine state matches external signal changes.                                          |
| on_open_change timing             | callback order        | Assert callback fires after machine transition, not during.                                    |
| cleanup completeness              | cleanup side effects  | Assert no listeners, subscriptions, or z-index leaks after unmount.                            |
| title/description ID registration | context registration  | Assert context IDs update when Title/Description mount/unmount.                                |

Cheap verification recipe:

1. Render a popover with a trigger and content containing a focusable button. Click the trigger and assert the popover opens, focus moves to the button, positioner has CSS custom properties, and dismiss buttons are present.
2. Click outside the popover and assert it closes; verify the closing click is NOT the same event that opened it by checking rAF guard behavior.
3. Unmount the popover and assert all document listeners, positioning subscriptions, and z-index allocations are cleaned up.
4. Render with `portal=true` and assert content is a descendant of the portal root, not the component tree.

## 31. Implementation Checklist

- [ ] Machine initialization and context publication are correct.
- [ ] Trigger click dispatches `Toggle` and aria attrs update.
- [ ] Controlled open-state sync uses deferred effect and dispatches `Open`/`Close`.
- [ ] Portal rendering places content in `ArsProvider` portal root.
- [ ] Z-index allocated from `ZIndexAllocator` on open, released on close and cleanup.
- [ ] Positioning engine subscribed on open, CSS custom properties applied to positioner, unsubscribed on close.
- [ ] Click-outside listener deferred by rAF; pending rAF cancelled on rapid close.
- [ ] Dismissable composed with trigger and anchor as inside boundaries.
- [ ] FocusScope::popover() composed with `contain=false`, `restore_focus=true`.
- [ ] Paired dismiss buttons rendered in non-modal mode with correct visually-hidden styling and aria-label.
- [ ] Arrow position CSS custom properties applied.
- [ ] Title and Description register IDs in context on mount, clear on unmount.
- [ ] `on_open_change` fires after all open/close transitions.
- [ ] Presence composes `lazy_mount` and `unmount_on_exit` behavior.
- [ ] `same_width` measures trigger offsetWidth and sets positioner min-width.
- [ ] Modal mode uses `role="dialog"`, `aria-modal="true"`, and traps focus.
- [ ] Non-modal mode uses `role="group"` and does not trap focus.
- [ ] SSR renders trigger with correct aria; content deferred to client.
- [ ] Cleanup removes all listeners, subscriptions, allocations, and portal mounts.
- [ ] No dismiss callbacks or positioning fire during SSR.
