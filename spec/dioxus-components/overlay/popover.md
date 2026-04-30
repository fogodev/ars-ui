---
adapter: dioxus
component: popover
category: overlay
source: components/overlay/popover.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Popover — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Popover`](../../components/overlay/popover.md) machine to Dioxus 0.7.x. The adapter owns compound component context publication, portal rendering, positioning engine integration via `ars-dom`, z-index allocation via `ZIndexAllocator`, click-outside race prevention (rAF-deferred listener attachment on web, platform-appropriate equivalent on desktop/mobile), dismiss-button rendering, focus management via `FocusScope::popover()` composition, CSS custom property application on the positioner element, `Presence` composition for mount/unmount animation lifecycle, and cleanup of all positioning subscriptions, click-outside listeners, and allocated z-index layers. On desktop and mobile targets, the adapter must account for platform differences in outside-interaction detection and portal rendering.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct PopoverProps {
    #[props(into)]
    pub id: String,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(optional, default = false)]
    pub default_open: bool,
    #[props(optional, default = false)]
    pub modal: bool,
    #[props(optional, default = true)]
    pub close_on_escape: bool,
    #[props(optional, default = true)]
    pub close_on_interact_outside: bool,
    #[props(optional)]
    pub positioning: Option<PositioningOptions>,
    #[props(optional, default = 0.0)]
    pub offset: f64,
    #[props(optional, default = 0.0)]
    pub cross_offset: f64,
    #[props(optional, default = false)]
    pub same_width: bool,
    #[props(optional, default = true)]
    pub portal: bool,
    #[props(optional, default = false)]
    pub lazy_mount: bool,
    #[props(optional, default = false)]
    pub unmount_on_exit: bool,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(optional)]
    pub messages: Option<popover::Messages>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

#[component]
pub fn Popover(props: PopoverProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct TriggerProps {
    pub children: Element,
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct AnchorProps {
    pub children: Element,
}

#[component]
pub fn Anchor(props: AnchorProps) -> Element

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
pub struct DescriptionProps {
    pub children: Element,
}

#[component]
pub fn Description(props: DescriptionProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct CloseTriggerProps {
    pub children: Element,
}

#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element
```

`PopoverProps` accepts the full core prop set: `id`, `open` (controlled `Option<Signal<bool>>`), `default_open`, `modal`, `close_on_escape`, `close_on_interact_outside`, `positioning`, `offset`, `cross_offset`, `same_width`, `portal`, `lazy_mount`, `unmount_on_exit`, `on_open_change` (`Option<EventHandler<bool>>`), `messages`, `locale`, and `children: Element`.

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
| DismissButton (start) | required when non-modal | visually hidden `<button>` before content children | adapter-owned                     | `dismiss_button_attrs(label)`                    | Screen reader close mechanism.            |
| DismissButton (end)   | required when non-modal | visually hidden `<button>` after content children  | adapter-owned                     | `dismiss_button_attrs(label)`                    | Second screen reader close mechanism.     |

## 5. Attr Merge and Ownership Rules

| Target node    | Core attrs                    | Adapter-owned attrs                                                                                                                                                                                                 | Consumer attrs               | Merge order                                                                                         | Ownership notes                        |
| -------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------------------- | -------------------------------------- |
| `Root`         | `api.root_attrs()`            | `data-ars-state`                                                                                                                                                                                                    | consumer root attrs          | core scope/part/state attrs win; `class`/`style` merge additively                                   | adapter-owned wrapper                  |
| `Trigger`      | `api.trigger_attrs()`         | `type="button"`                                                                                                                                                                                                     | consumer trigger attrs       | core `aria-expanded`/`aria-controls` win                                                            | adapter-owned native button            |
| `Anchor`       | `api.anchor_attrs()`          | none                                                                                                                                                                                                                | consumer anchor attrs        | core scope/part attrs win; consumer owns element choice                                             | consumer-owned element                 |
| `Positioner`   | `api.positioner_attrs()`      | CSS custom properties (`--ars-x`, `--ars-y`, `--ars-z-index`, `--ars-reference-width`, `--ars-reference-height`, `--ars-available-width`, `--ars-available-height`, `--ars-transform-origin`), `position: absolute` | none                         | core placement attrs win; CSS vars are adapter-owned                                                | adapter-owned portal child             |
| `Content`      | `api.content_attrs()`         | `tabindex="-1"`, focus-scope composition attrs                                                                                                                                                                      | consumer content attrs       | core `role`/`aria-modal`/`aria-labelledby`/`aria-describedby` win; `class`/`style` merge additively | adapter-owned                          |
| `Arrow`        | `api.arrow_attrs()`           | arrow CSS custom properties (`--ars-arrow-x`, `--ars-arrow-y`)                                                                                                                                                      | consumer arrow attrs         | core arrow position attrs win                                                                       | adapter-owned                          |
| `Title`        | `api.title_attrs()`           | none                                                                                                                                                                                                                | consumer title attrs         | core `id` wins                                                                                      | consumer-owned element with adapter ID |
| `Description`  | `api.description_attrs()`     | none                                                                                                                                                                                                                | consumer description attrs   | core `id` wins                                                                                      | consumer-owned element with adapter ID |
| `CloseTrigger` | `api.close_trigger_attrs()`   | `type="button"`                                                                                                                                                                                                     | consumer close-trigger attrs | core `aria-label` wins                                                                              | adapter-owned native button            |
| DismissButton  | `dismiss_button_attrs(label)` | visually hidden styles                                                                                                                                                                                              | none                         | core attrs win                                                                                      | adapter-owned, not consumer-exposed    |

- Consumers must not override `role`, `aria-modal`, `aria-expanded`, `aria-controls`, `aria-labelledby`, or `aria-describedby` on their respective parts.
- CSS custom properties on the positioner are the adapter's positioning output channel; consumers must not override them but may read them.

## 6. Composition / Context Contract

`Popover` publishes a `Context` context (via `use_context_provider`) consumed by all child parts (via `try_use_context`). The context carries: machine send handle (`Callback<popover::Event>`), open state signal, trigger ref, content ref, anchor ref (optional), title ID, description ID, service handle, context version, and configuration flags (modal, portal).

Composition dependencies:

- **Dismissable**: `Content` composes `Dismissable` for outside-interaction detection. The trigger element and any anchor element are registered as inside boundaries so clicks on them do not dismiss the popover.
- **FocusScope**: `Content` composes `FocusScope::popover()` preset (`contain=false`, `restore_focus=true`). Focus moves to the first tabbable element on open, restores to trigger on close. Focus is NOT trapped.
- **ArsProvider**: Portal rendering reads the portal root from `ArsContext`.
- **ZIndexAllocator**: The positioner allocates a z-index layer on open and releases it on close.
- **Presence**: When `lazy_mount` or `unmount_on_exit` is enabled, `Presence` manages the content mount/unmount lifecycle for animation support.
- **ArsProvider**: Messages and locale resolution use the nearest `ArsProvider` context when `locale` is `None`.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled open state is not supported after mount. `default_open` is init-only. When `open` is `Some(signal)`, the adapter watches it via a deferred `use_effect` and dispatches `Open`/`Close` events on change.

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

| UI event                    | Preconditions                                                         | Machine event / callback path | Ordering notes                               | Notes                         |
| --------------------------- | --------------------------------------------------------------------- | ----------------------------- | -------------------------------------------- | ----------------------------- |
| trigger click               | trigger interactive                                                   | `Toggle`                      | fires before `on_open_change`                | native button click           |
| `Escape` on content         | open and `close_on_escape` enabled                                    | `CloseOnEscape`               | Escape check runs before outside-interaction | keydown on content element    |
| outside pointer interaction | open and `close_on_interact_outside` enabled, rAF guard elapsed (web) | `CloseOnInteractOutside`      | rAF-deferred attachment prevents race on web | Dismissable-composed          |
| close-trigger click         | close trigger interactive                                             | `Close`                       | fires before `on_open_change`                | native button click           |
| dismiss-button activation   | dismiss button rendered and interactive                               | `Close`                       | dismiss normalized before `on_open_change`   | visually hidden button        |
| positioning update          | content mounted and positioning engine active                         | `PositioningUpdate(result)`   | context-only update, no state change         | CSS custom properties updated |

## 8. Registration and Cleanup Contract

| Registered entity           | Registration trigger                   | Identity key                    | Cleanup trigger                                       | Cleanup action                             | Notes                                                     |
| --------------------------- | -------------------------------------- | ------------------------------- | ----------------------------------------------------- | ------------------------------------------ | --------------------------------------------------------- |
| positioning subscription    | content mount on client                | popover instance                | close or component cleanup                            | unsubscribe positioning engine             | client-only on all platforms                              |
| click-outside listener      | open transition, deferred by rAF (web) | popover instance                | close, `CloseOnInteractOutside`, or component cleanup | cancel pending rAF and remove listener     | rAF guard required on web; platform equivalent on desktop |
| z-index allocation          | open transition                        | popover instance                | close or component cleanup                            | release allocated z-index layer            | via ZIndexAllocator                                       |
| focus-scope registration    | content mount on client                | popover instance via FocusScope | close or component cleanup                            | unregister scope and restore focus         | FocusScope::popover()                                     |
| Dismissable registration    | content mount on client                | popover instance                | close or component cleanup                            | remove dismiss listeners and boundary refs | Dismissable-composed                                      |
| portal mount                | content mount with `portal=true`       | popover instance                | close or component cleanup                            | remove portal content from portal root     | via ArsProvider                                           |
| title ID registration       | Title mount                            | title instance                  | Title unmount                                         | clear title_id in context                  | optional part                                             |
| description ID registration | Description mount                      | description instance            | Description unmount                                   | clear description_id in context            | optional part                                             |

Cleanup uses `use_drop` to ensure all registered resources are released before the component is removed from the tree.

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
| browser/webview APIs absent during SSR       | no-op              | Render trigger structure; defer content, positioning, and listeners to client. |
| anchor ref missing when Anchor part rendered | warn and ignore    | Fall back to trigger as positioning reference.                                 |
| desktop/mobile platform missing rAF API      | degrade gracefully | Use timestamp-based click-outside guard (strategy 2 from core spec).           |

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
- On desktop and mobile targets, SSR is not applicable; all rendering is client-side.

## 15. Performance Constraints

- Positioning subscriptions must only register while the popover is open; they must not remain active when closed.
- Click-outside listeners must detach on close, not persist idle on the document.
- Z-index allocation and release must be paired; leaked allocations degrade the stacking order for other overlays.
- CSS custom property application on the positioner should batch with the positioning callback, not trigger additional re-renders.
- Title and description ID registration should update only on mount/unmount of those parts, not on every render.
- `derive()` calls should be scoped to the minimum necessary data to avoid unnecessary re-renders of child parts.
- Since `Signal<T>` is `Copy` in Dioxus, context propagation avoids cloning overhead but implementers must still minimize the number of signals read per component to limit re-render scope.

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

1. Initialize the popover machine with `use_machine::<popover::Machine>(props)` and publish `Context` context via `use_context_provider`.
2. Render `Popover` wrapper and `Trigger` with click handler dispatching `Toggle`.
3. Wire controlled open-state sync via deferred `use_effect` when `open` is `Some(signal)`.
4. Implement portal rendering for `Positioner` using `ArsProvider` portal root.
5. Allocate z-index from `ZIndexAllocator` on open transition; release on close.
6. Integrate positioning engine (`ars-dom`) between trigger/anchor ref and positioner ref; apply CSS custom properties on each positioning update.
7. Compose `Dismissable` inside `Content` with trigger (and anchor, if present) registered as inside boundaries. Implement rAF-deferred click-outside guard on web; timestamp-based guard on desktop/mobile.
8. Compose `FocusScope::popover()` inside `Content` for non-trapping focus with restore-on-close.
9. Render paired dismiss buttons (start and end) inside content when non-modal.
10. Implement `Arrow` with arrow position CSS custom properties.
11. Implement `Title` and `Description` with context-based ID registration.
12. Compose `Presence` for `lazy_mount` and `unmount_on_exit` behavior.
13. Wire `on_open_change` callback after machine transitions.
14. Verify cleanup ordering: positioning unsubscribe, click-outside listener removal, z-index release, focus restore, Dismissable teardown, portal unmount.

## 18. Anti-Patterns

- Do not attach click-outside listeners synchronously on open; the triggering click will bubble and immediately close the popover. Use the rAF-deferred guard on web or the timestamp-based guard on desktop/mobile.
- Do not keep positioning subscriptions active when the popover is closed.
- Do not leak z-index allocations by forgetting to release on close or cleanup.
- Do not hardcode z-index values; always use `ZIndexAllocator`.
- Do not render popover content inline when `portal=true`; it must render into the portal root.
- Do not trap focus in non-modal popovers; use `FocusScope::popover()` with `contain=false`.
- Do not omit dismiss buttons in non-modal mode; screen reader users depend on them to close the popover.
- Do not fork open state into a local signal that diverges from the machine state or controlled prop.
- Do not fire `on_open_change` during SSR.
- Do not render portal content during SSR when the portal root does not exist.
- Do not hold `.read()` or `.write()` guards across `.await` boundaries; clone the value out first.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the popover content renders in a portal root outside the component tree when `portal=true`.
- Consumers may assume focus moves to the first tabbable element inside the popover on open, and restores to the trigger on close.
- Consumers may assume Tab is not trapped; it flows naturally through the popover content and then continues into the page.
- Consumers may assume the positioner element carries CSS custom properties reflecting the current computed position.
- Consumers may assume `on_open_change` fires after every open/close transition regardless of the trigger source.
- Consumers may assume paired dismiss buttons are present in non-modal mode for screen reader accessibility.
- Consumers may assume the component works on web, desktop, and mobile targets with platform-appropriate behavior.
- Consumers must not assume the popover content exists in the DOM when closed (it may be unmounted depending on `unmount_on_exit` and `lazy_mount`).
- Consumers must not assume positioning runs during SSR.
- Consumers must not assume the z-index value is stable across different open/close cycles.
- Consumers must not assume the click-outside guard strategy (rAF vs timestamp) is the same across platforms.

## 20. Platform Support Matrix

| Capability / behavior            | Web          | Desktop       | Mobile        | SSR            | Notes                                                    |
| -------------------------------- | ------------ | ------------- | ------------- | -------------- | -------------------------------------------------------- |
| trigger rendering and aria attrs | full support | full support  | full support  | full support   | Trigger renders on all platforms.                        |
| portal content rendering         | full support | full support  | full support  | SSR-safe empty | Content deferred to client when portal root unavailable. |
| positioning engine integration   | full support | full support  | full support  | client-only    | ars-dom requires live DOM/webview measurements.          |
| click-outside detection (rAF)    | full support | fallback path | fallback path | client-only    | Desktop/mobile use timestamp-based guard.                |
| z-index allocation               | full support | full support  | full support  | client-only    | ZIndexAllocator is client-only.                          |
| focus management                 | full support | full support  | full support  | client-only    | FocusScope activation is client-only.                    |
| dismiss buttons                  | full support | full support  | full support  | full support   | Rendered on all platforms for non-modal.                 |
| Presence animation               | full support | full support  | full support  | SSR-safe empty | Animation lifecycle is client-only.                      |
| `on_open_change` callback        | full support | full support  | full support  | client-only    | Never fires during SSR.                                  |
| `same_width` trigger measurement | full support | full support  | full support  | client-only    | Requires live `offsetWidth` measurement.                 |

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
| rAF API unavailable on desktop/mobile          | debug warning        | degrade gracefully  | Switch to timestamp-based click-outside guard.    |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                                                  | Reused by                                                     | Notes                                               |
| -------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------- |
| positioning helper         | required    | Subscribe to positioning engine, apply CSS custom properties to positioner, compute arrow position.             | `popover`, `tooltip`, `hover-card`, `tour`                    | Shared across all positioned overlays.              |
| portal helper              | required    | Render content into the portal root from ArsProvider context.                                                   | `popover`, `dialog`, `tooltip`, `hover-card`, `toast`, `tour` | Shared across all portal-rendering overlays.        |
| z-index helper             | required    | Allocate and release z-index layers via ZIndexAllocator.                                                        | `popover`, `dialog`, `tooltip`, `hover-card`, `toast`, `tour` | Paired allocation/release lifecycle.                |
| click-outside guard helper | required    | Implement rAF-deferred (web) or timestamp-based (desktop/mobile) click-outside listener attachment and cleanup. | `popover`, `hover-card`                                       | Prevents open-click race condition. Platform-aware. |
| dismiss-button helper      | required    | Render paired visually hidden dismiss buttons with correct attrs.                                               | `popover`, `hover-card`, `tooltip`                            | Non-modal overlays only.                            |
| merge helper               | recommended | Merge core attrs, adapter attrs, and consumer attrs with documented precedence.                                 | all overlay components                                        | Prevents accidental attr override.                  |
| measurement helper         | recommended | Measure trigger `offsetWidth` for `same_width` support.                                                         | `popover`, `combobox`                                         | Client-only DOM measurement.                        |
| platform capability helper | recommended | Detect rAF availability and choose click-outside strategy accordingly.                                          | `popover`, `hover-card`, `dismissable`                        | Normalizes web vs desktop/mobile behavior.          |

## 23. Framework-Specific Behavior

Dioxus uses `use_drop` for teardown of positioning subscriptions, click-outside listeners, z-index allocations, and Dismissable registrations. Controlled open state is watched via a deferred `use_effect` (not body-level sync) because the effect dispatches `Open`/`Close` events through `send.call()`. Portal rendering uses Dioxus document APIs or manual DOM insertion into the `ArsProvider` portal root node. `Signal<T>` is `Copy` in Dioxus, making context propagation lightweight. Context is published via `use_context_provider(|| Context { ... })` and consumed via `try_use_context::<Context>().expect("...")`.

On web targets, `requestAnimationFrame` is available for the rAF-deferred click-outside guard. On desktop and mobile targets (webview-based), rAF may be unavailable; the adapter falls back to the timestamp-based comparison strategy from the core spec. Event handlers in Dioxus use `EventHandler<T>` for callbacks and `Callback<T>` for machine send handles. Dioxus `Callback` uses `.call()` while Leptos `Callback` uses `.run()`.

## 24. Canonical Implementation Sketch

```rust,no_check
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PopoverProps {
    #[props(into)]
    pub id: String,
    #[props(optional)]
    pub open: Option<Signal<bool>>,
    #[props(optional, default = false)]
    pub default_open: bool,
    #[props(optional, default = false)]
    pub modal: bool,
    #[props(optional, default = true)]
    pub close_on_escape: bool,
    #[props(optional, default = true)]
    pub close_on_interact_outside: bool,
    #[props(optional)]
    pub positioning: popover::PositioningOptions,
    #[props(optional, default = 0.0)]
    pub offset: f64,
    #[props(optional, default = 0.0)]
    pub cross_offset: f64,
    #[props(optional, default = false)]
    pub same_width: bool,
    #[props(optional, default = true)]
    pub portal: bool,
    #[props(optional, default = false)]
    pub lazy_mount: bool,
    #[props(optional, default = false)]
    pub unmount_on_exit: bool,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
    #[props(optional)]
    pub messages: Option<popover::Messages>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

#[derive(Clone, Copy)]
struct Context {
    send: Callback<popover::Event>,
    is_open: ReadSignal<bool>,
    modal: bool,
    portal: bool,
    service: Signal<Service<popover::Machine>>,
    context_version: ReadSignal<u64>,
}

#[component]
pub fn Popover(props: PopoverProps) -> Element {
    let core_props = popover::Props {
        id: props.id.clone(),
        open: props.open.as_ref().map(|s| *s.read()),
        default_open: props.default_open,
        modal: props.modal,
        close_on_escape: props.close_on_escape,
        close_on_interact_outside: props.close_on_interact_outside,
        positioning: props.positioning.clone(),
        offset: props.offset,
        cross_offset: props.cross_offset,
        same_width: props.same_width,
        portal: props.portal,
        lazy_mount: props.lazy_mount,
        unmount_on_exit: props.unmount_on_exit,
        on_open_change: None, // Handled at adapter level
        messages: props.messages.clone(),
        locale: props.locale.clone(),
    };

    let machine = use_machine::<popover::Machine>(core_props);
    let UseMachineReturn { send, .. } = machine;

    let is_open = machine.derive(|api| api.is_open());
    let root_attrs = machine.derive(|api| api.root_attrs());

    // Watch controlled open signal
    if let Some(open_sig) = props.open {
        let send_clone = send;
        let mut prev_open: Signal<Option<bool>> = use_signal(|| None);
        use_effect(move || {
            let new_open = *open_sig.read();
            let prev = prev_open.read().clone();
            if prev.as_ref() != Some(&new_open) {
                if prev.is_some() {
                    if new_open {
                        send_clone.call(popover::Event::Open);
                    } else {
                        send_clone.call(popover::Event::Close);
                    }
                }
                *prev_open.write() = Some(new_open);
            }
        });
    }

    // Fire on_open_change after transitions
    if let Some(cb) = &props.on_open_change {
        let cb = cb.clone();
        let open_derived = is_open;
        use_effect(move || {
            cb.call(open_derived());
        });
    }

    use_context_provider(|| Context {
        send,
        is_open: is_open.into(),
        modal: props.modal,
        portal: props.portal,
        service: machine.service,
        context_version: machine.context_version,
    });

    rsx! {
        div {
            // Spread root_attrs here
            "data-ars-scope": "popover",
            "data-ars-part": "root",
            "data-ars-state": if is_open() { "open" } else { "closed" },
            {props.children}
        }
    }
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("popover::Trigger must be used inside Popover");

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "popover",
            "data-ars-part": "trigger",
            "aria-expanded": ctx.is_open().to_string(),
            onclick: move |_| ctx.send.call(popover::Event::Toggle),
            {props.children}
        }
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("popover::Content must be used inside Popover");
    let send = ctx.send;

    if !ctx.is_open() {
        return rsx! {};
    }

    // Compose: Dismissable for outside-interaction
    // Compose: FocusScope::popover() for non-trapping focus
    // Render: paired dismiss buttons for non-modal

    rsx! {
        div {
            role: if ctx.modal { "dialog" } else { "group" },
            "aria-modal": if ctx.modal { "true" } else { "" },
            tabindex: "-1",
            "data-ars-scope": "popover",
            "data-ars-part": "content",
            "data-ars-state": "open",
            onkeydown: move |e: KeyboardEvent| {
                if dioxus_key_to_keyboard_key(&e.key()).0 == KeyboardKey::Escape {
                    send.call(popover::Event::CloseOnEscape);
                }
            },
            if !ctx.modal {
                button {
                    style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)",
                    onclick: move |_| send.call(popover::Event::Close),
                    "Dismiss popover"
                }
            }
            {props.children}
            if !ctx.modal {
                button {
                    style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)",
                    onclick: move |_| send.call(popover::Event::Close),
                    "Dismiss popover"
                }
            }
        }
    }
}

#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("popover::CloseTrigger must be used inside Popover");
    let machine = UseMachineReturn {
        state: /* from ctx */,
        send: ctx.send,
        service: ctx.service,
        context_version: ctx.context_version,
    };
    let dismiss_label = machine.derive(|api| {
        (api.ctx.messages.dismiss_label)(&api.ctx.locale)
    });

    rsx! {
        button {
            r#type: "button",
            "data-ars-scope": "popover",
            "data-ars-part": "close-trigger",
            "aria-label": dismiss_label(),
            onclick: move |_| ctx.send.call(popover::Event::Close),
            {props.children}
        }
    }
}

#[component]
pub fn Title(props: TitleProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("popover::Title must be used inside Popover");
    let title_id = machine.derive(|api| api.ctx.title_id.clone());
    rsx! {
        h3 {
            id: title_id().unwrap_or_default(),
            "data-ars-scope": "popover",
            "data-ars-part": "title",
            {props.children}
        }
    }
}

// Usage:
// rsx! {
//     Popover { id: "my-popover",
//         Trigger { "Open Popover" }
//         Positioner {
//             Arrow {}
//             Content {
//                 Title { "Popover Title" }
//                 Description { "Some description text." }
//                 CloseTrigger { "X" }
//             }
//         }
//     }
// }
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Popover
let machine = use_machine::<popover::Machine>(props);
let is_open = machine.derive(|api| api.is_open());

use_context_provider(|| Context { send, is_open, modal, portal, service, context_version });
watch_controlled_open_signal(open_signal, send);  // deferred use_effect
fire_on_open_change_after_transitions(is_open, on_open_change);  // use_effect

render_root_wrapper(root_attrs);

// Trigger
let ctx = try_use_context::<Context>().expect("...");
render_native_button(trigger_attrs, toggle_handler);

// Positioner (client-only rendering inside portal)
let ctx = try_use_context::<Context>().expect("...");
let z_index = allocate_z_index_on_open(ctx.is_open);  // ZIndexAllocator
let positioning_sub = subscribe_positioning_engine(
    trigger_ref_or_anchor_ref(),
    positioner_ref,
    positioning_options,
);
apply_css_custom_properties(positioner_ref, positioning_sub);
apply_same_width_measurement(trigger_ref, positioner_ref, same_width);

render_in_portal_or_inline(ctx.portal, || {
    render_positioner(positioner_ref, positioner_attrs, z_index);
});

// Content
let ctx = try_use_context::<Context>().expect("...");
let dismiss_handle = compose_dismissable(
    content_ref,
    vec![trigger_ref, anchor_ref],
    ctx.send,
);
let focus_scope_handle = compose_focus_scope_popover(content_ref);
let click_outside_guard = create_platform_aware_click_outside_guard();

render_content_with_dismiss_buttons(content_ref, content_attrs, ctx.modal);
wire_escape_keydown(content_ref, ctx.send);

// Arrow
apply_arrow_css_custom_properties(arrow_ref, positioning_sub);
render_arrow(arrow_ref, arrow_attrs);

// Title / Description
register_title_id_in_context(ctx);
register_description_id_in_context(ctx);

// Presence composition (wraps Positioner or Content)
compose_presence(ctx.is_open, lazy_mount, unmount_on_exit);

use_drop(|| {
    positioning_sub.unsubscribe();
    dismiss_handle.teardown();
    focus_scope_handle.teardown();
    z_index.release();
    portal_handle.unmount();
    click_outside_guard.cancel_pending();
});
```

## 26. Adapter Invariants

- Click-outside listener attachment MUST be deferred by one `requestAnimationFrame` on web targets after the open transition. On desktop/mobile targets where rAF is unavailable, the timestamp-based comparison strategy MUST be used instead. Synchronous attachment causes the triggering click to immediately close the popover.
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
- Controlled open-state sync MUST use a deferred `use_effect`, not body-level sync, because it dispatches events.
- CSS custom properties on the positioner (`--ars-x`, `--ars-y`, `--ars-z-index`, etc.) MUST be updated on each positioning engine callback and MUST NOT be set during SSR.
- The adapter MUST NOT hold `.read()` or `.write()` signal guards across `.await` boundaries.

## 27. Accessibility and SSR Notes

- Non-modal popovers use `role="group"` to avoid confusing screen readers that announce "dialog" and set user expectations for focus trapping. Modal popovers use `role="dialog"` with `aria-modal="true"`.
- `aria-expanded` on the trigger reflects the open state. `aria-controls` points to the content ID when open.
- `aria-labelledby` and `aria-describedby` on the content reference the Title and Description part IDs when those parts are rendered.
- `tabindex="-1"` on the content allows programmatic focus when no tabbable children exist.
- Paired dismiss buttons provide a screen-reader-discoverable close mechanism for non-modal popovers. They use `aria-label` from the resolved `Messages.dismiss_label`.
- SSR renders the trigger with correct ARIA attributes. Content structure is deferred to the client unless `default_open` is true, in which case the content skeleton is server-rendered for hydration stability but without positioning or listeners.
- On desktop and mobile targets, accessibility behaviors are equivalent (webview-based rendering).

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, part, and behavior parity including all nine compound component parts, click-outside race prevention, positioning engine integration, z-index allocation, focus management, dismiss buttons, Presence composition, and portal rendering. Platform support extends to web, desktop, and mobile targets.

Intentional deviations: the click-outside guard uses rAF on web and timestamp comparison on desktop/mobile where rAF may be unavailable. This is a platform adaptation, not a behavioral deviation.

Traceability note: This adapter spec makes explicit the core adapter-owned concerns for rAF-deferred (or timestamp-based) click-outside guard, positioning engine CSS custom property application, portal rendering via ArsProvider, z-index allocation lifecycle, FocusScope::popover() composition, paired dismiss-button rendering, Presence animation lifecycle, controlled open-state sync via deferred effect, platform-aware outside-interaction detection, and cleanup ordering for all registered resources.

## 29. Test Scenarios

- trigger click toggles popover open/closed
- controlled open signal syncs with machine state
- Escape key closes the popover when `close_on_escape` is true
- outside click closes the popover when `close_on_interact_outside` is true
- click-outside does NOT fire on the same click that opened the popover (rAF guard on web, timestamp guard on desktop)
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
- desktop/mobile: click-outside uses timestamp guard when rAF unavailable

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
| platform click-outside strategy   | callback order        | On desktop, assert timestamp guard prevents opening-click dismissal.                           |

Cheap verification recipe:

1. Render a popover with a trigger and content containing a focusable button. Click the trigger and assert the popover opens, focus moves to the button, positioner has CSS custom properties, and dismiss buttons are present.
2. Click outside the popover and assert it closes; verify the closing click is NOT the same event that opened it by checking the guard behavior.
3. Unmount the popover and assert all document listeners, positioning subscriptions, and z-index allocations are cleaned up.
4. Render with `portal=true` and assert content is a descendant of the portal root, not the component tree.
5. On desktop target, repeat the click-outside test and verify the timestamp-based guard prevents the race condition.

## 31. Implementation Checklist

- [ ] Machine initialization and context publication via `use_context_provider` are correct.
- [ ] Trigger click dispatches `Toggle` and aria attrs update.
- [ ] Controlled open-state sync uses deferred `use_effect` and dispatches `Open`/`Close` via `.call()`.
- [ ] Portal rendering places content in `ArsProvider` portal root.
- [ ] Z-index allocated from `ZIndexAllocator` on open, released on close and cleanup.
- [ ] Positioning engine subscribed on open, CSS custom properties applied to positioner, unsubscribed on close.
- [ ] Click-outside listener deferred by rAF on web; timestamp guard on desktop/mobile. Pending rAF cancelled on rapid close.
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
- [ ] Cleanup via `use_drop` removes all listeners, subscriptions, allocations, and portal mounts.
- [ ] No dismiss callbacks or positioning fire during SSR.
- [ ] Desktop/mobile platform fallback for click-outside guard is implemented and tested.
- [ ] No `.read()`/`.write()` guards held across `.await` boundaries.
