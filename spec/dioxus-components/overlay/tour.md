---
adapter: dioxus
component: tour
category: overlay
source: components/overlay/tour.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Tour — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Tour`](../../components/overlay/tour.md) behavior to Dioxus 0.7.x. The adapter owns portal rendering, z-index allocation, overlay and spotlight positioning, scroll-to-target behavior, step content positioning via the `ars-dom` engine, keyboard event normalization, and all 12 compound sub-components. Step definitions are passed as a `Vec<tour::Step>` prop; target elements are resolved by CSS selector or ID on the client. On Desktop and Mobile, target resolution uses the webview DOM; non-webview targets degrade gracefully.

## 2. Public Adapter API

The adapter exposes a compound component set rather than a single hook. The root component establishes the machine and provides context; child parts consume it.

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct TourProps {
    #[props(into)]
    pub id: String,
    pub steps: Vec<tour::Step>,
    #[props(optional)]
    pub open: Option<bool>,
    #[props(optional, default = false)]
    pub default_open: bool,
    #[props(optional, default = false)]
    pub auto_start: bool,
    #[props(optional, default = true)]
    pub close_on_overlay_click: bool,
    #[props(optional, default = true)]
    pub close_on_escape: bool,
    #[props(optional, default = true)]
    pub keyboard_navigation: bool,
    #[props(optional)]
    pub on_open_change: Option<Callback<bool>>,
    #[props(optional)]
    pub on_step_change: Option<Callback<usize>>,
    #[props(optional, default = false)]
    pub lazy_mount: bool,
    #[props(optional, default = false)]
    pub unmount_on_exit: bool,
    #[props(optional)]
    pub messages: Option<tour::Messages>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

pub fn Tour(props: TourProps) -> Element

pub fn Backdrop() -> Element

pub fn Spotlight() -> Element

pub fn Positioner() -> Element

#[derive(Props, Clone, PartialEq)]
pub struct ContentProps {
    pub children: Element,
}

pub fn Content(props: ContentProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct TitleProps {
    pub children: Element,
}

pub fn Title(props: TitleProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct DescriptionProps {
    pub children: Element,
}

pub fn Description(props: DescriptionProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct CloseTriggerProps {
    pub children: Element,
}

pub fn CloseTrigger(props: CloseTriggerProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct NextTriggerProps {
    pub children: Element,
}

pub fn NextTrigger(props: NextTriggerProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct PrevTriggerProps {
    pub children: Element,
}

pub fn PrevTrigger(props: PrevTriggerProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct SkipTriggerProps {
    pub children: Element,
}

pub fn SkipTrigger(props: SkipTriggerProps) -> Element

#[derive(Props, Clone, PartialEq)]
pub struct ProgressProps {
    pub children: Element,
}

pub fn Progress(props: ProgressProps) -> Element
```

The public surface matches the full core `Props`, including `steps`, `open`, `default_open`, `auto_start`, `close_on_overlay_click`, `close_on_escape`, `keyboard_navigation`, `on_open_change`, `on_step_change`, `lazy_mount`, `unmount_on_exit`, `messages`, and `locale`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core props.
- Event parity: Start, NextStep, PrevStep, GoToStep, Skip, Complete, Dismiss, AddStep, RemoveStep, UpdateStep, StepChange, Focus, and Blur all map to the core machine events.
- Structure parity: all 12 core parts (Root, Overlay, Highlight, StepContent, StepTitle, StepDescription, NextTrigger, PrevTrigger, SkipTrigger, CloseTrigger, Progress, StepIndicator) are rendered as distinct elements with their core attrs applied.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                           | Ownership                                        | Attr source                       | Notes                                                                       |
| --------------------- | ----------- | -------------------------------------------------- | ------------------------------------------------ | --------------------------------- | --------------------------------------------------------------------------- |
| Root                  | required    | `div` wrapping the entire tour output              | adapter-owned                                    | `api.root_attrs()`                | Rendered inside a portal.                                                   |
| Overlay               | required    | `div` covering the viewport                        | adapter-owned                                    | `api.overlay_attrs()`             | Click handler sends `Event::Dismiss` when `close_on_overlay_click` is true. |
| Highlight             | conditional | `div` positioned over the target element           | adapter-owned                                    | `api.highlight_attrs()`           | Only rendered for `StepType::Tooltip` steps with a resolved target.         |
| StepContent           | required    | `div` with `role="dialog"` positioned near target  | adapter-owned structure, consumer-owned children | `api.step_content_attrs()`        | Receives keyboard handler for Escape and arrow keys.                        |
| StepTitle             | required    | `h3` inside StepContent                            | adapter-owned structure, consumer-owned children | `api.step_title_attrs()`          | ID cross-referenced by `aria-labelledby`.                                   |
| StepDescription       | required    | `p` inside StepContent                             | adapter-owned structure, consumer-owned children | `api.step_description_attrs()`    | ID cross-referenced by `aria-describedby`.                                  |
| NextTrigger           | required    | `button` inside StepContent                        | adapter-owned                                    | `api.next_trigger_attrs()`        | Label changes to done label on last step.                                   |
| PrevTrigger           | required    | `button` inside StepContent                        | adapter-owned                                    | `api.prev_trigger_attrs()`        | `aria-disabled` when on step 0.                                             |
| SkipTrigger           | optional    | `button` inside StepContent                        | adapter-owned                                    | `api.skip_trigger_attrs()`        | Sends `Event::Skip`.                                                        |
| CloseTrigger          | required    | `button` inside StepContent                        | adapter-owned                                    | `api.close_trigger_attrs()`       | Sends `Event::Dismiss`.                                                     |
| Progress              | required    | `div` with `aria-live="polite"` inside StepContent | adapter-owned structure, consumer-owned children | `api.progress_attrs()`            | Announces step changes to screen readers.                                   |
| StepIndicator         | repeated    | `span` per step inside Progress                    | adapter-owned                                    | `api.step_indicator_attrs(index)` | `data-ars-current` marks active indicator.                                  |

## 5. Attr Merge and Ownership Rules

| Target node     | Core attrs                 | Adapter-owned attrs                             | Consumer attrs           | Merge order                                                          | Ownership notes         |
| --------------- | -------------------------- | ----------------------------------------------- | ------------------------ | -------------------------------------------------------------------- | ----------------------- |
| Root            | `api.root_attrs()`         | portal container markers, z-index style         | consumer `class`/`style` | core scope and state attrs win; `class`/`style` merge additively     | adapter-owned root      |
| Overlay         | `api.overlay_attrs()`      | inline positioning styles (full viewport)       | consumer `class`/`style` | core `aria-hidden` wins                                              | adapter-owned           |
| Highlight       | `api.highlight_attrs()`    | inline positioning styles from target rect      | consumer `class`/`style` | core `aria-hidden` wins                                              | adapter-owned           |
| StepContent     | `api.step_content_attrs()` | inline positioning styles from `ars-dom` engine | consumer children        | core `role`, `aria-modal`, `aria-labelledby`, `aria-describedby` win | adapter-owned structure |
| Trigger buttons | per-trigger core attrs     | native `button` defaults                        | consumer children text   | core `type`, `aria-label`, `aria-disabled` win                       | adapter-owned           |
| Progress        | `api.progress_attrs()`     | none                                            | consumer children        | core `aria-live` wins                                                | adapter-owned structure |

- Consumers must not override `role="dialog"` or `aria-modal` on StepContent.
- Consumers must not override `aria-hidden="true"` on Overlay or Highlight.
- Trigger handlers compose around normalized machine event dispatch; consumer callbacks observe but must not bypass.

## 6. Composition / Context Contract

`Tour` provides a `Context` via `use_context_provider`. All child parts retrieve it via `try_use_context::<Context>()`. The context contains the machine handle and all derived signals.

The tour composes with:

- **Portal**: Tour root content renders inside a portal to escape stacking contexts. On Web, this targets `document.body` or nearest `ArsProvider` boundary. On Desktop/Mobile, it uses the webview DOM.
- **Z-index allocator**: The root acquires a z-index allocation on mount and releases it on cleanup via `use_drop`.
- **FocusScope**: Step content traps focus within the active step. Focus moves to the new step content on step transitions.
- **Dismissable**: Overlay click and Escape handling follow the dismissable pattern (integrated directly, not composed as a separate component).
- **Presence**: When `lazy_mount` or `unmount_on_exit` is set, the tour content participates in presence-based mount/unmount transitions.

## 7. Prop Sync and Event Mapping

| Adapter prop             | Mode         | Sync trigger                | Machine event / update path               | Visible effect                       | Notes                                                                  |
| ------------------------ | ------------ | --------------------------- | ----------------------------------------- | ------------------------------------ | ---------------------------------------------------------------------- |
| `steps`                  | reactive     | signal change               | machine re-init or dynamic step events    | step definitions update              | changes to length or content update `total_steps` and current step def |
| `open`                   | controlled   | prop change                 | controlled open-state sync                | tour starts or stops                 | two-way binding via `on_open_change`                                   |
| `default_open`           | initial      | render time only            | initial state computation                 | tour starts if true                  | uncontrolled default                                                   |
| `auto_start`             | initial      | render time only            | `State::Active { step_index: 0 }` if true | tour starts on mount                 | fires `on_open_change(true)`                                           |
| `close_on_overlay_click` | non-reactive | render time only            | guards `Event::Dismiss` on overlay click  | overlay click may or may not dismiss | re-read from props at event time                                       |
| `close_on_escape`        | non-reactive | render time only            | guards `Event::Dismiss` on Escape         | Escape may or may not dismiss        | re-read from props at event time                                       |
| `keyboard_navigation`    | non-reactive | render time only            | guards ArrowRight/ArrowLeft handling      | arrow keys may or may not navigate   | re-read from props at event time                                       |
| `on_open_change`         | callback     | after open/close transition | notification only                         | none                                 | fires after state settles                                              |
| `on_step_change`         | callback     | after step transition       | notification only                         | none                                 | fires after step index updates                                         |
| `lazy_mount`             | initial      | render time only            | mount control                             | content not mounted until first open | presence-based                                                         |
| `unmount_on_exit`        | initial      | render time only            | unmount control                           | content removed from DOM after close | presence-based                                                         |
| `messages`               | non-reactive | render time only            | resolved into context                     | aria-labels and progress text        | uses `resolve_messages`                                                |
| `locale`                 | non-reactive | render time only            | resolved into context                     | locale for message formatting        | falls back to nearest `ArsProvider`                                    |

| UI event              | Preconditions                              | Machine event / callback path                      | Ordering notes                             | Notes                                |
| --------------------- | ------------------------------------------ | -------------------------------------------------- | ------------------------------------------ | ------------------------------------ |
| Overlay click         | tour active, `close_on_overlay_click` true | `Event::Dismiss` -> `on_open_change(false)`        | dismiss fires before callback              | client-only                          |
| Escape key            | tour active, `close_on_escape` true        | `Event::Dismiss` -> `on_open_change(false)`        | keydown handler on StepContent             | client-only                          |
| ArrowRight            | tour active, `keyboard_navigation` true    | `Event::NextStep` -> `on_step_change`              | keydown handler on StepContent             | does not wrap past last step         |
| ArrowLeft             | tour active, `keyboard_navigation` true    | `Event::PrevStep` -> `on_step_change`              | keydown handler on StepContent             | does not wrap before first step      |
| Next button click     | tour active                                | `Event::NextStep` or `Event::Complete` (last step) | complete transitions to `State::Completed` | button label changes on last step    |
| Prev button click     | tour active, not first step                | `Event::PrevStep`                                  | no-op when `aria-disabled` on step 0       | button remains in DOM but disabled   |
| Skip button click     | tour active                                | `Event::Skip` -> `on_open_change(false)`           | transitions to `State::Inactive`           | distinct from complete               |
| Close button click    | tour active                                | `Event::Dismiss` -> `on_open_change(false)`        | transitions to `State::Inactive`           | same path as Escape                  |
| Focus on StepContent  | tour active                                | `Event::Focus { is_keyboard }`                     | context-only update                        | sets `focused` and `focus_visible`   |
| Blur from StepContent | tour active                                | `Event::Blur`                                      | context-only update                        | clears `focused` and `focus_visible` |

## 8. Registration and Cleanup Contract

| Registered entity        | Registration trigger     | Identity key                 | Cleanup trigger           | Cleanup action                 | Notes                                         |
| ------------------------ | ------------------------ | ---------------------------- | ------------------------- | ------------------------------ | --------------------------------------------- |
| Z-index allocation       | tour root mount          | component instance           | `use_drop`                | release z-index slot           | prevents stacking leaks                       |
| Portal mount             | tour root mount          | component instance           | `use_drop`                | remove portal container        | DOM node removed from body                    |
| Positioning subscription | step activation (client) | step index + target selector | step change or tour close | cancel positioning computation | prevents stale position updates               |
| Scroll-to-target effect  | step activation (client) | step index                   | step change or tour close | cancel pending scroll          | `scrollIntoView` on target element            |
| Focus-step effect        | step transition          | step index                   | next transition           | no explicit cleanup            | focus moves to new step content               |
| Keyboard listener        | StepContent mount        | component instance           | StepContent unmount       | remove listener                | attached to StepContent element, not document |
| Target element observer  | step activation (client) | step index + target selector | step change or tour close | disconnect observer            | watches for target resize/move                |

## 9. Ref and Node Contract

| Target part / node      | Ref required?              | Ref owner                   | Node availability                 | Composition rule                                         | Notes                             |
| ----------------------- | -------------------------- | --------------------------- | --------------------------------- | -------------------------------------------------------- | --------------------------------- |
| Root (portal container) | yes                        | adapter-owned               | required after mount              | portal container ref for z-index and positioning         | server-safe absent                |
| StepContent             | yes                        | adapter-owned               | required after mount              | focus management and positioning anchor                  | client-only                       |
| Target element          | yes (resolved by selector) | consumer-owned page element | client-only                       | resolved via `document.querySelector` at step activation | always structural handle optional |
| Overlay                 | no                         | adapter-owned               | always structural handle optional | no ref needed; click handler is inline                   |                                   |
| Highlight               | no                         | adapter-owned               | always structural handle optional | positioned via CSS custom properties from target rect    |                                   |
| Trigger buttons         | no                         | adapter-owned               | always structural handle optional | no ref needed; click handlers are inline                 |                                   |

## 10. State Machine Boundary Rules

- Machine-owned state: `State` (Inactive/Active/Completed), `Context` (steps, current_step, total_steps, target_element_id, focused, focus_visible, open, ids, messages).
- Adapter-local derived bookkeeping: positioning coordinates from `ars-dom`, target element bounding rect, scroll-to-target pending state, portal mount handle, z-index allocation handle, observer handles.
- Forbidden local mirrors: do not keep a local `is_open` or `current_step` signal that can diverge from `ctx.open` or `ctx.current_step`. Use `machine.derive(|api| ...)` exclusively.
- Allowed snapshot-read contexts: keyboard event handlers (via `machine.with_api_snapshot`), positioning computation callbacks, scroll-to-target effects.

## 11. Callback Payload Contract

| Callback                | Payload source           | Payload shape                            | Timing                         | Cancelable?          | Notes                                       |
| ----------------------- | ------------------------ | ---------------------------------------- | ------------------------------ | -------------------- | ------------------------------------------- |
| `on_open_change`        | machine-derived snapshot | `bool` (new open state)                  | after state transition settles | no                   | fires for Start, Skip, Dismiss, Complete    |
| `on_step_change`        | machine-derived snapshot | `usize` (new step index)                 | after step transition settles  | no                   | fires for NextStep, PrevStep, GoToStep      |
| `on_keydown` (internal) | raw framework event      | Dioxus `KeyboardEvent` via `event.key()` | before machine event dispatch  | no (machine decides) | normalized to `KeyboardKey` before dispatch |

## 12. Failure and Degradation Rules

| Condition                                     | Policy             | Notes                                                                        |
| --------------------------------------------- | ------------------ | ---------------------------------------------------------------------------- |
| Target element not found for current step     | degrade gracefully | Position step content centered (Dialog-style fallback); log debug warning.   |
| Portal container unavailable                  | fail fast          | Tour cannot render without a portal mount point.                             |
| Z-index allocation exhausted                  | degrade gracefully | Use a fallback high z-index value; log debug warning.                        |
| Positioning engine returns no valid placement | degrade gracefully | Fall back to centered positioning.                                           |
| Steps array empty on Start                    | no-op              | Machine guard prevents transition to Active with zero steps.                 |
| `open` prop set to true with zero steps       | no-op              | No transition occurs; `on_open_change(false)` fires to re-sync.              |
| Step index out of bounds after RemoveStep     | degrade gracefully | Machine clamps to valid range or transitions to Inactive if no steps remain. |
| Browser scroll API absent during SSR          | no-op              | Scroll-to-target deferred until mount.                                       |
| Desktop/Mobile without webview DOM access     | warn and ignore    | Target resolution and positioning degrade; step content renders centered.    |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?            | DOM order must match registration order? | SSR/hydration stability               | Notes                              |
| -------------------------------- | ---------------- | ------------------------------ | ---------------------------------------- | ------------------------------------- | ---------------------------------- |
| Tour root                        | instance-derived | no                             | not applicable                           | root ID stable across hydration       | `ComponentIds::from_id(&props.id)` |
| StepIndicator (repeated)         | data-derived     | no (one per step)              | yes                                      | indicator count must match step count | keyed by step index                |
| Step definitions                 | data-derived     | yes (duplicate titles allowed) | yes (order matches `steps` vec)          | step order stable                     | identity is positional index       |
| Z-index allocation               | instance-derived | not applicable                 | not applicable                           | client-only                           | released on cleanup                |

## 14. SSR and Client Boundary Rules

- SSR renders nothing for the tour when `open` is false or `auto_start` is false, because the tour content lives inside a portal that is client-only.
- When `auto_start` is true, the SSR pass may render the initial structure inside the portal placeholder, but positioning, scroll-to-target, and focus effects are deferred to mount.
- Portal container creation, z-index allocation, target element resolution, positioning computation, scroll-to-target, and keyboard listeners are all client-only.
- `on_open_change` and `on_step_change` callbacks must not fire during SSR.
- StepContent, Overlay, and Highlight structure must remain stable across hydration when the tour starts in an open state.
- `lazy_mount` means the DOM content is not emitted at all during SSR if the tour has not been started.
- On Desktop and Mobile targets, SSR is not applicable; the first render is always a client render.

## 15. Performance Constraints

- Positioning computations must not run on every render; they subscribe to target element resize/scroll and recalculate only when the target rect changes or the step changes.
- Step indicator rendering should use keyed iteration (`for indicator in 0..total { span { key: "{indicator}", ... } }`) to avoid re-creating all indicators on step change.
- Derived signals for `is_open`, `current_step`, `is_last_step`, etc. should be memoized via `use_memo` so that parts only re-render when their specific data changes.
- Scroll-to-target should use a single `scrollIntoView` call per step transition, not a continuous polling loop.
- Cleanup must release z-index allocation, disconnect target observers, cancel pending positioning, and remove the portal container in one pass.
- Overlay and Highlight should use CSS transforms or `will-change` for GPU-accelerated positioning rather than triggering layout on every frame.
- Dioxus `Signal<T>` is `Copy`; avoid unnecessary `.clone()` calls on signal handles.

## 16. Implementation Dependencies

| Dependency                     | Required?   | Dependency type         | Why it must exist first                                           | Notes                                                                 |
| ------------------------------ | ----------- | ----------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------- |
| `ars-provider`                 | recommended | context contract        | Portal mount point and DOM boundary resolution.                   | Falls back to `document.body` on Web; webview root on Desktop/Mobile. |
| `focus-scope`                  | required    | behavioral prerequisite | Focus trap within StepContent per step.                           | Focus must move to new step on transition.                            |
| `dismissable`                  | conceptual  | behavioral prerequisite | Overlay click and Escape handling follow the dismissable pattern. | Integrated directly; not composed as a separate component.            |
| `portal`                       | required    | composition contract    | Tour content must escape stacking contexts.                       | Renders into portal container.                                        |
| `z-index-allocator`            | required    | shared helper           | Overlay and content must sit above page content.                  | Allocation acquired on mount, released on cleanup.                    |
| `presence`                     | recommended | composition contract    | Supports `lazy_mount` and `unmount_on_exit` transitions.          | Animation-aware mount/unmount.                                        |
| Positioning engine (`ars-dom`) | required    | shared helper           | StepContent positioned relative to target element.                | Flip, slide, offset logic from `05-interactions.md`.                  |

## 17. Recommended Implementation Sequence

1. Implement `Tour` with machine initialization, context provision via `use_context_provider`, and portal rendering.
2. Implement `Backdrop` (Overlay) with viewport-covering div and click-to-dismiss.
3. Implement `Spotlight` (Highlight) with target element rect tracking and cutout positioning.
4. Implement `Positioner` with `ars-dom` positioning engine integration and CSS custom property output.
5. Implement `Content` with `role="dialog"`, focus trap via FocusScope, and keyboard handler.
6. Implement `Title` and `Description` with ID-based cross-references.
7. Implement `NextTrigger`, `PrevTrigger`, `SkipTrigger`, `CloseTrigger` with event dispatch.
8. Implement `Progress` with `aria-live="polite"` and `StepIndicator` iteration.
9. Wire up z-index allocation, scroll-to-target, and step-change focus effects.
10. Implement `lazy_mount` and `unmount_on_exit` via Presence integration.
11. Verify cleanup order via `use_drop`: observers, positioning, portal, z-index, focus scope.
12. Test on Desktop and Mobile targets; verify graceful degradation when target elements are unavailable.

## 18. Anti-Patterns

- Do not resolve target elements during SSR; `document.querySelector` is client-only.
- Do not store a local `current_step` mirror; derive it from the machine context.
- Do not attach keyboard listeners to the document; attach them to the StepContent element.
- Do not re-run positioning on every render; subscribe to target rect changes only.
- Do not skip the portal; tour content must escape stacking contexts to overlay correctly.
- Do not omit z-index allocation; manual z-index values cause stacking conflicts with other overlays.
- Do not use `aria-modal="true"` on StepContent; the tour is non-modal and background content remains accessible to assistive technology.
- Do not reverse arrow key direction for RTL; step progression is conceptual, not spatial.
- Do not fire `on_open_change` or `on_step_change` during SSR.
- Do not keep the portal container alive after the component unmounts.
- Do not hold `.read()` or `.write()` guards across `.await` boundaries; clone the value out first.
- Do not assume `document.querySelector` is available on all Dioxus targets; guard with platform checks.

## 19. Consumer Expectations and Guarantees

- Consumers may assume that step content is positioned correctly relative to the target element after mount.
- Consumers may assume that focus moves to the new StepContent on each step transition.
- Consumers may assume that `aria-live="polite"` on Progress announces step changes to screen readers.
- Consumers may assume that `on_open_change` fires after every open/close transition, including Skip, Dismiss, and Complete.
- Consumers may assume that StepIndicator elements are keyed by step index and remain stable across step transitions.
- Consumers may assume the adapter works on Web, Desktop, and Mobile Dioxus targets.
- Consumers must not assume the tour content is in the component's DOM subtree; it renders in a portal.
- Consumers must not assume target elements exist at SSR time.
- Consumers must not assume `on_step_change` fires for the initial step (step 0) on Start; it fires only on transitions.
- Consumers must not assume target element resolution succeeds on non-webview Desktop targets.

## 20. Platform Support Matrix

| Capability / behavior               | Web          | Desktop                | Mobile                 | SSR            | Notes                                            |
| ----------------------------------- | ------------ | ---------------------- | ---------------------- | -------------- | ------------------------------------------------ |
| Tour rendering (portal)             | full support | full support           | full support           | client-only    | Portal container created on mount.               |
| Target element resolution           | full support | full support (webview) | full support (webview) | not available  | `document.querySelector` via webview.            |
| Positioning engine                  | full support | full support (webview) | full support (webview) | not available  | Requires target and viewport measurements.       |
| Scroll-to-target                    | full support | full support (webview) | partial                | not available  | Mobile webviews may have restricted scroll APIs. |
| Focus trap (FocusScope)             | full support | full support           | full support           | not available  | Focus management requires live DOM.              |
| Keyboard navigation                 | full support | full support           | not applicable         | not available  | Mobile targets use touch, not keyboard.          |
| Z-index allocation                  | full support | full support           | full support           | not available  | Allocation is a client-side operation.           |
| `aria-live` announcements           | full support | full support           | partial                | structure only | Mobile webview AT support varies.                |
| `on_open_change` / `on_step_change` | full support | full support           | full support           | no-op          | Callbacks never fire during SSR.                 |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                                           |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------------------------------------- |
| Target element not found for step               | debug warning        | degrade gracefully  | Log selector and step index; fall back to centered positioning. |
| Portal container creation failed                | fail fast            | fail fast           | Tour cannot render.                                             |
| Z-index allocation exhausted                    | debug warning        | degrade gracefully  | Use fallback value; log warning.                                |
| Step index out of bounds                        | debug warning        | degrade gracefully  | Clamp to valid range.                                           |
| `steps` prop is empty when `auto_start` is true | debug warning        | no-op               | Log that auto-start was requested with no steps.                |
| Positioning engine returns no placement         | debug warning        | degrade gracefully  | Fall back to centered.                                          |
| `Content` rendered without `Tour` parent        | fail fast            | fail fast           | Context is required.                                            |
| Platform lacks webview DOM access               | debug warning        | warn and ignore     | Log platform name; step content renders centered.               |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                    | Reused by                                       | Notes                                                     |
| -------------------------- | ----------- | --------------------------------------------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------- |
| Portal helper              | required    | Mount tour content into a detached DOM container.                                 | `dialog`, `popover`, `tooltip`, `toast`, `tour` | Shared portal mounting and cleanup logic.                 |
| Z-index allocator          | required    | Acquire and release z-index slots for overlay stacking.                           | `dialog`, `popover`, `tooltip`, `toast`, `tour` | Must release on cleanup via `use_drop`.                   |
| Positioning engine bridge  | required    | Connect `ars-dom` positioning to CSS custom properties on the positioner element. | `popover`, `tooltip`, `hover-card`, `tour`      | Step-specific placement and target rect.                  |
| Focus scope helper         | required    | Trap focus within StepContent; move focus on step transitions.                    | `dialog`, `popover`, `tour`                     | Per-step focus trap.                                      |
| Scroll-into-view helper    | recommended | Scroll target element into view before positioning.                               | `tour`                                          | Single `scrollIntoView` call per step.                    |
| Target rect observer       | recommended | Watch for target element resize/reposition and trigger re-positioning.            | `tour`                                          | ResizeObserver + scroll listener on ancestors.            |
| Platform capability helper | recommended | Normalize DOM API assumptions across Web, Desktop, and Mobile.                    | `dismissable`, `tour`, `drop-zone`              | Guards `querySelector` and `scrollIntoView` availability. |

## 23. Framework-Specific Behavior

Dioxus uses `use_signal` for local state and `Signal<T>` (which is `Copy`) for derived values. Keyboard events use `onkeydown` with `event.key()` normalized to `KeyboardKey` via a `dioxus_key_to_keyboard_key` helper. Focus management uses `onfocus` and `onblur` on StepContent. Cleanup uses `use_drop` to release z-index, disconnect observers, and remove the portal container. Context is provided via `use_context_provider` and consumed via `try_use_context::<Context>()`. Event handler closures can use `machine.send.call()` directly because `Signal` is `Copy` and does not require explicit cloning.

On Desktop and Mobile targets, DOM APIs (`querySelector`, `scrollIntoView`, `ResizeObserver`) are accessed through the webview bridge. The adapter must guard these calls behind platform checks and degrade gracefully when unavailable.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;
use ars_core::tour;

/// Context shared by all Tour sub-components.
#[derive(Clone, Copy)]
pub struct Context {
    machine: Signal<MachineHandle<tour::Machine>>,
    open: Memo<bool>,
    current_step: Memo<usize>,
    total_steps: Memo<usize>,
    is_last_step: Memo<bool>,
    current_step_def: Memo<Option<tour::Step>>,
}

#[component]
pub fn Tour(props: TourProps) -> Element {
    let core_props = tour::Props {
        id: props.id,
        steps: props.steps,
        open: props.open,
        default_open: props.default_open,
        auto_start: props.auto_start,
        close_on_overlay_click: props.close_on_overlay_click,
        close_on_escape: props.close_on_escape,
        keyboard_navigation: props.keyboard_navigation,
        on_open_change: props.on_open_change,
        on_step_change: props.on_step_change,
        lazy_mount: props.lazy_mount,
        unmount_on_exit: props.unmount_on_exit,
        messages: props.messages,
        locale: props.locale,
    };

    let machine = use_machine::<tour::Machine>(core_props);
    let machine_sig = use_signal(|| machine.clone());

    let tour_ctx = Context {
        machine: machine_sig,
        open: use_memo(move || machine.derive(|api| api.is_open())),
        current_step: use_memo(move || machine.derive(|api| api.current_step())),
        total_steps: use_memo(move || machine.derive(|api| api.total_steps())),
        is_last_step: use_memo(move || machine.derive(|api| api.is_last_step())),
        current_step_def: use_memo(move || machine.derive(|api| api.current_step_def().cloned())),
    };

    use_context_provider(|| tour_ctx);

    // Z-index allocation
    let z_index = use_z_index();

    // Cleanup
    use_drop(move || {
        drop(z_index);
    });

    let root_attrs = machine.derive(|api| api.root_attrs());
    let is_open = tour_ctx.open;

    if !is_open() && props.lazy_mount {
        return rsx! {};
    }

    rsx! {
        // Portal rendering
        document::Link { }  // placeholder — actual portal uses DOM manipulation
        if is_open() {
            div {
                ..root_attrs.read().clone(),
                style: "z-index: {z_index.value()}",
                {props.children}
            }
        }
    }
}

#[component]
pub fn Backdrop() -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Backdrop must be used within a Tour");
    let machine = ctx.machine;
    let overlay_attrs = machine.read().derive(|api| api.overlay_attrs());

    rsx! {
        div {
            ..overlay_attrs.read().clone(),
            onclick: move |_| machine.read().send.call(tour::Event::Dismiss),
        }
    }
}

#[component]
pub fn Spotlight() -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Spotlight must be used within a Tour");
    let machine = ctx.machine;
    let highlight_attrs = machine.read().derive(|api| api.highlight_attrs());

    // Target rect is computed client-side and applied as inline styles
    rsx! {
        div { ..highlight_attrs.read().clone() }
    }
}

#[component]
pub fn Content(props: ContentProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Content must be used within a Tour");
    let machine = ctx.machine;
    let content_attrs = machine.read().derive(|api| api.step_content_attrs());

    rsx! {
        div {
            ..content_attrs.read().clone(),
            onkeydown: move |ev| {
                machine.read().with_api_snapshot(|api| {
                    api.on_keydown(&KeyboardEventData {
                        key: dioxus_key_to_keyboard_key(&ev.key()),
                        ..Default::default()
                    });
                });
            },
            onfocus: move |ev| {
                let is_keyboard = ev.data().as_any().downcast_ref::<FocusData>()
                    .map_or(false, |_| true);
                machine.read().send.call(tour::Event::Focus { is_keyboard });
            },
            onblur: move |_| {
                machine.read().send.call(tour::Event::Blur);
            },
            {props.children}
        }
    }
}

#[component]
pub fn Title(props: TitleProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Title must be used within a Tour");
    let machine = ctx.machine;
    let title_attrs = machine.read().derive(|api| api.step_title_attrs());

    rsx! { h3 { ..title_attrs.read().clone(), {props.children} } }
}

#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Description must be used within a Tour");
    let machine = ctx.machine;
    let desc_attrs = machine.read().derive(|api| api.step_description_attrs());

    rsx! { p { ..desc_attrs.read().clone(), {props.children} } }
}

#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::CloseTrigger must be used within a Tour");
    let machine = ctx.machine;
    let close_attrs = machine.read().derive(|api| api.close_trigger_attrs());

    rsx! {
        button {
            ..close_attrs.read().clone(),
            onclick: move |_| machine.read().send.call(tour::Event::Dismiss),
            {props.children}
        }
    }
}

#[component]
pub fn NextTrigger(props: NextTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::NextTrigger must be used within a Tour");
    let machine = ctx.machine;
    let next_attrs = machine.read().derive(|api| api.next_trigger_attrs());

    rsx! {
        button {
            ..next_attrs.read().clone(),
            onclick: move |_| machine.read().send.call(tour::Event::NextStep),
            {props.children}
        }
    }
}

#[component]
pub fn PrevTrigger(props: PrevTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::PrevTrigger must be used within a Tour");
    let machine = ctx.machine;
    let prev_attrs = machine.read().derive(|api| api.prev_trigger_attrs());

    rsx! {
        button {
            ..prev_attrs.read().clone(),
            onclick: move |_| machine.read().send.call(tour::Event::PrevStep),
            {props.children}
        }
    }
}

#[component]
pub fn SkipTrigger(props: SkipTriggerProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::SkipTrigger must be used within a Tour");
    let machine = ctx.machine;
    let skip_attrs = machine.read().derive(|api| api.skip_trigger_attrs());

    rsx! {
        button {
            ..skip_attrs.read().clone(),
            onclick: move |_| machine.read().send.call(tour::Event::Skip),
            {props.children}
        }
    }
}

#[component]
pub fn Progress(props: ProgressProps) -> Element {
    let ctx = try_use_context::<Context>()
        .expect("tour::Progress must be used within a Tour");
    let machine = ctx.machine;
    let progress_attrs = machine.read().derive(|api| api.progress_attrs());

    rsx! {
        div { ..progress_attrs.read().clone(), {props.children} }
    }
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
let props = build_tour_props(id, steps, open, ...);
let machine = use_machine::<tour::Machine>(props);
let tour_ctx = build_tour_context(machine);
use_context_provider(|| tour_ctx);

let z_index = use_z_index();
let portal_handle = create_portal_container();

// Client-only effects
use_effect(move || {
    let step_def = tour_ctx.current_step_def();
    if let Some(step) = step_def {
        if let Some(selector) = &step.target {
            let target_el = document().query_selector(selector);
            scroll_into_view_if_needed(target_el);
            start_positioning(content_ref, target_el, step.placement);
            start_target_observer(target_el);
        }
    }
});

// Render via portal
render_portal(portal_handle, z_index, rsx! {
    render_root(machine, rsx! {
        Backdrop {}
        Spotlight {}
        Content {
            CloseTrigger { "Close" }
            Title { /* step title */ }
            Description { /* step description */ }
            Progress {
                // StepIndicator elements
                for i in 0..total_steps {
                    span { key: "{i}", ..step_indicator_attrs(i) }
                }
            }
            PrevTrigger { "Previous" }
            NextTrigger { "Next" }
            SkipTrigger { "Skip tour" }
        }
    })
});

use_drop(|| {
    disconnect_target_observer();
    cancel_positioning();
    remove_portal_container(portal_handle);
    drop(z_index);
});
```

## 26. Adapter Invariants

- Tour content must render inside a portal to escape stacking contexts.
- Z-index allocation must be acquired on mount and released on cleanup via `use_drop`.
- StepContent must have `role="dialog"` and `aria-modal="false"` at all times.
- StepTitle ID must match the `aria-labelledby` value on StepContent.
- StepDescription ID must match the `aria-describedby` value on StepContent.
- Progress must have `aria-live="polite"` to announce step changes.
- PrevTrigger must have `aria-disabled="true"` when on the first step.
- NextTrigger label must switch to the done label on the last step.
- Keyboard events must be handled on StepContent, not on document.
- Target element resolution must not run during SSR.
- Positioning must recalculate on target resize, scroll, and step change.
- Focus must move to StepContent on each step transition.
- All observers, positioning subscriptions, portal containers, and z-index allocations must be cleaned up via `use_drop` before unmount completes.
- Overlay click must not dismiss when `close_on_overlay_click` is false.
- Escape must not dismiss when `close_on_escape` is false.
- Arrow keys must not navigate when `keyboard_navigation` is false.
- Signal `.read()` and `.write()` guards must not be held across `.await` boundaries.

## 27. Accessibility and SSR Notes

- StepContent uses `role="dialog"` with `aria-modal="false"` because the tour is non-modal; background content remains accessible to assistive technology.
- `aria-labelledby` on StepContent points to StepTitle; `aria-describedby` points to StepDescription. Both IDs are deterministic from `ComponentIds`.
- Progress uses `aria-live="polite"` so screen readers announce step changes without interrupting the user.
- PrevTrigger uses `aria-disabled="true"` (not the `disabled` attribute) on step 0 so it remains focusable and discoverable by assistive technology.
- All trigger buttons have localized `aria-label` values from the Messages struct.
- SSR emits no tour content when the tour is not open, because portal containers are client-only. When the tour starts in an open state (via `auto_start`), the SSR pass may emit placeholder structure, but positioning and focus are deferred to hydration.
- RTL does not reverse arrow key direction; step progression is conceptual.
- On Desktop and Mobile, screen reader support depends on the webview's AT integration; the adapter emits correct ARIA attributes regardless of platform.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, event, and part parity. All 12 parts are rendered with their documented attrs. All machine events are wired. All callbacks fire at their documented times.

Intentional deviations:

- Dioxus uses `use_drop` for cleanup instead of Leptos `on_cleanup`.
- Dioxus uses `use_context_provider`/`try_use_context` instead of Leptos `provide_context`/`use_context`.
- Dioxus `Signal<T>` is `Copy`; no explicit cloning of signal handles is needed.
- Desktop and Mobile targets add a platform capability check for DOM APIs; the Leptos adapter is web-only.
- Keyboard event normalization uses `dioxus_key_to_keyboard_key` helper instead of direct `ev.key()` string conversion.

Traceability note: This adapter spec makes explicit the adapter-owned concerns for portal rendering, z-index allocation, target element resolution, positioning engine integration, scroll-to-target, focus-per-step management, keyboard listener placement, compound component context distribution, and multi-platform degradation.

## 29. Test Scenarios

- Tour starts via `auto_start` and renders step 0 content in a portal.
- Next button advances step; prev button goes back; both fire `on_step_change`.
- Skip button transitions to Inactive and fires `on_open_change(false)`.
- Close button and Escape both dismiss via `Event::Dismiss`.
- Overlay click dismisses when `close_on_overlay_click` is true; no-op when false.
- ArrowRight/ArrowLeft navigate steps when `keyboard_navigation` is true; no-op when false.
- Escape does not dismiss when `close_on_escape` is false.
- PrevTrigger is `aria-disabled` on step 0.
- NextTrigger label changes to done label on last step.
- Progress region has `aria-live="polite"` and updates on step change.
- StepIndicator count matches step count; current indicator has `data-ars-current`.
- Focus moves to StepContent on each step transition.
- Target element not found falls back to centered positioning.
- `lazy_mount` prevents DOM emission until first open.
- `unmount_on_exit` removes DOM after tour completes or is dismissed.
- Cleanup releases z-index, removes portal, disconnects observers.
- Controlled `open` prop overrides internal state.
- Desktop target: tour renders and functions when target elements are in the webview DOM.
- Desktop target: tour degrades gracefully when target element selector returns no match.

## 30. Test Oracle Notes

| Behavior                                            | Preferred oracle type | Notes                                                                                         |
| --------------------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------- |
| Part attrs (role, aria-_, data-ars-_)               | DOM attrs             | Assert all 12 parts carry their documented attributes.                                        |
| State transitions (Inactive -> Active -> Completed) | machine state         | Assert state after Start, NextStep through last, Complete.                                    |
| Callback ordering (on_open_change, on_step_change)  | callback order        | Assert `on_step_change` fires after step transition; `on_open_change` fires after open/close. |
| Context provision and consumption                   | context registration  | Assert child parts can retrieve Context.                                                      |
| Portal structure                                    | rendered structure    | Assert tour content is a child of portal container, not the component subtree.                |
| SSR output                                          | hydration structure   | Assert no tour content in SSR output when tour is not open.                                   |
| Cleanup                                             | cleanup side effects  | Assert z-index released, portal removed, observers disconnected after unmount.                |

Cheap verification recipe:

1. Mount `Tour` with 3 steps and `auto_start=true`. Assert step 0 renders in a portal with correct attrs on all parts.
2. Click next twice and assert `on_step_change` fires for each transition. Assert step indicators update.
3. On step 2 (last), assert next trigger label is the done label. Click it and assert `on_open_change(false)` fires and tour content is removed.
4. Restart the tour and press Escape. Assert dismiss path fires `on_open_change(false)`.
5. Unmount and assert z-index allocation is released and portal container is removed from the DOM.
6. On Desktop target, repeat steps 1-5 to verify webview DOM integration.

## 31. Implementation Checklist

- [ ] `Tour` initializes machine, provides `Context` via `use_context_provider`, acquires z-index, renders via portal.
- [ ] `Backdrop` renders overlay with click-to-dismiss gated by `close_on_overlay_click`.
- [ ] `Spotlight` renders highlight positioned over target element rect.
- [ ] `Content` renders `role="dialog"` with `aria-modal="false"`, keyboard handler, focus/blur handlers.
- [ ] `Title` and `Description` render with IDs matching `aria-labelledby` and `aria-describedby`.
- [ ] `NextTrigger` sends `NextStep`; label switches to done label on last step.
- [ ] `PrevTrigger` sends `PrevStep`; `aria-disabled` on step 0.
- [ ] `SkipTrigger` sends `Skip`.
- [ ] `CloseTrigger` sends `Dismiss`.
- [ ] `Progress` renders with `aria-live="polite"`; `StepIndicator` elements keyed by step index.
- [ ] Target element resolved by selector on client; centered fallback when not found.
- [ ] Positioning engine positions StepContent relative to target with flip/slide/offset.
- [ ] Scroll-to-target fires once per step transition.
- [ ] Focus moves to StepContent on each step transition.
- [ ] `on_open_change` fires after every open/close transition.
- [ ] `on_step_change` fires after every step transition.
- [ ] `lazy_mount` and `unmount_on_exit` integrated via Presence.
- [ ] Controlled `open` prop synchronizes with machine state.
- [ ] Z-index allocation released on cleanup via `use_drop`.
- [ ] Portal container removed on cleanup.
- [ ] Target observers disconnected on cleanup.
- [ ] No positioning, scroll, focus, or callbacks during SSR.
- [ ] Desktop and Mobile targets degrade gracefully when DOM APIs are unavailable.
- [ ] Signal `.read()`/`.write()` guards are never held across `.await` boundaries.
