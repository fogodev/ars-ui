---
adapter: leptos
component: toast
category: overlay
source: components/overlay/toast.md
source_foundation: foundation/08-adapter-leptos.md
---

# Toast -- Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Toast`](../../components/overlay/toast.md) notification system to Leptos 0.8.x. The adapter owns: the `Provider` context that holds the imperative queue, the `Region` that renders dual `aria-live` containers (polite and assertive), per-toast `Service` lifecycle management, swipe-to-dismiss pointer handling, auto-dismiss timer wiring (including pause-on-hover and pause-on-page-idle), progress bar animation, promise toast future spawning, and announcement coordination timing.

## 2. Public Adapter API

```rust,no_check
/// Provider component. Place at the application root so that the dual
/// aria-live regions are present in server-rendered HTML.
#[component]
pub fn Provider(
    #[prop(optional)] placement: Option<toast::Placement>,
    #[prop(optional)] max_visible: Option<usize>,
    #[prop(optional)] pause_on_hover: Option<bool>,
    #[prop(optional)] pause_on_page_idle: Option<bool>,
    #[prop(optional)] offsets: Option<toast::EdgeOffsets>,
    #[prop(optional)] overlap: Option<bool>,
    #[prop(optional)] hotkey: Option<String>,
    #[prop(optional)] remove_delay: Option<u32>,
    #[prop(optional)] default_durations: Option<toast::DefaultDurations>,
    #[prop(optional)] deduplicate_all: Option<bool>,
    #[prop(optional)] messages: Option<toast::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView

/// Imperative hook. Returns a Toaster handle for creating, updating,
/// dismissing, and promise-tracking toasts from anywhere in the tree.
pub fn use_toast() -> Toaster

/// Region component rendered internally by Provider. Contains the
/// dual aria-live containers and all visible toast instances.
#[component]
fn Region() -> impl IntoView

/// Per-toast root component. Rendered by the region for each active toast.
#[component]
fn Root(toast_id: String) -> impl IntoView

/// Per-toast title slot.
#[component]
pub fn Title(children: Children) -> impl IntoView

/// Per-toast description slot.
#[component]
pub fn Description(children: Children) -> impl IntoView

/// Per-toast action button with mandatory alt_text for screen readers.
#[component]
pub fn ActionTrigger(
    alt_text: String,
    children: Children,
) -> impl IntoView

/// Per-toast close button.
#[component]
pub fn CloseTrigger(children: Children) -> impl IntoView

/// Progress bar for time-remaining visualization.
#[component]
pub fn ProgressBar() -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with `toast::Props`, `toast::provider::Props`, and `toast::provider::Config`.
- Event parity: `Init`, `Pause`, `Resume`, `Dismiss`, `SwipeStart`, `SwipeMove`, `SwipeEnd`, `DurationExpired`, and `AnimationComplete` are all adapter-driven per-toast machine events.
- Manager parity: `provider::Event::Add`, `Update`, `Remove`, `PauseAll`, `ResumeAll`, and `DismissAll` map to the `Manager` and are exposed through the `Toaster` imperative handle.
- Structure parity: dual `aria-live` regions, per-toast `Root`/`Title`/`Description`/`ActionTrigger`/`CloseTrigger`/`ProgressBar` parts.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                            | Ownership                                       | Attr source                                    | Notes                                                             |
| --------------------- | ----------- | --------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------- |
| Region (polite)       | required    | `<div>` with `aria-live="polite"` `role="status"`   | adapter-owned, rendered by `Region`             | `toast::region_attrs(messages, locale, false)` | Must exist in SSR HTML. Routes info/success/loading toasts.       |
| Region (assertive)    | required    | `<div>` with `aria-live="assertive"` `role="alert"` | adapter-owned, rendered by `Region`             | `toast::region_attrs(messages, locale, true)`  | Must exist in SSR HTML. Routes error/warning toasts.              |
| Root                  | required    | `<div>` per toast inside the appropriate region     | adapter-owned via `Root`                        | `api.root_attrs()`                             | Carries `data-ars-state`, `data-ars-kind`, swipe offset.          |
| Title                 | optional    | `<div>` inside Root                                 | consumer-owned content in adapter-owned wrapper | `api.title_attrs()`                            | Rendered via `Title`.                                             |
| Description           | optional    | `<div>` inside Root                                 | consumer-owned content in adapter-owned wrapper | `api.description_attrs()`                      | Rendered via `Description`.                                       |
| ActionTrigger         | optional    | `<button>` inside Root                              | consumer-owned content in adapter-owned wrapper | `api.action_trigger_attrs(alt_text)`           | `aria-label` from `alt_text`.                                     |
| CloseTrigger          | optional    | `<button>` inside Root                              | adapter-owned                                   | `api.close_trigger_attrs()`                    | `aria-label` from `messages.dismiss_label`.                       |
| ProgressBar           | conditional | `<div>` inside Root when `show_progress=true`       | adapter-owned                                   | `api.progress_bar_attrs()`                     | `role="progressbar"`, `--ars-toast-progress` CSS custom property. |

## 5. Attr Merge and Ownership Rules

| Target node               | Core attrs                                      | Adapter-owned attrs                                                   | Consumer attrs                                 | Merge order                                                      | Ownership notes                                                  |
| ------------------------- | ----------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- |
| Region (polite/assertive) | `region_attrs()`                                | placement positioning, stacking CSS, `tabindex="-1"` for hotkey focus | consumer class/style decoration                | core ARIA attrs win; `class`/`style` merge additively            | adapter-owned; consumers must not override `aria-live` or `role` |
| Root                      | `api.root_attrs()`                              | swipe transform style, pointer listeners, z-index allocation          | consumer class/style per toast render callback | core state/kind/ARIA attrs win; `class`/`style` merge additively | adapter-owned per-toast wrapper                                  |
| Title / Description       | `api.title_attrs()` / `api.description_attrs()` | structural `data-ars-part`                                            | consumer content                               | core attrs apply as-is                                           | adapter-owned wrappers around consumer content                   |
| ActionTrigger             | `api.action_trigger_attrs(alt_text)`            | native button defaults                                                | consumer button content                        | core `aria-label` wins                                           | adapter-owned button around consumer content                     |
| CloseTrigger              | `api.close_trigger_attrs()`                     | native button defaults                                                | consumer icon/label                            | core `aria-label` wins                                           | adapter-owned                                                    |
| ProgressBar               | `api.progress_bar_attrs()`                      | `aria-valuenow`, `--ars-toast-progress` style property                | consumer decoration only                       | core progressbar ARIA wins                                       | adapter-owned                                                    |

- The dual `aria-live` regions must not be removed, relocated, or hidden by consumers.
- Swipe offset and progress value are set via inline style; consumers may add classes but must not override `transform` or `--ars-toast-progress`.

## 6. Composition / Context Contract

`Provider` provides a `QueueContext` via `provide_context`. The context contains the reactive toast list, the `Toaster` imperative handle, and provider-level configuration (placement, max_visible, default_durations). `Region` reads this context to render toasts. Each `Root` creates a per-toast `Service<toast::Machine>` and provides a per-toast context for child part components (`Title`, `Description`, `ActionTrigger`, `CloseTrigger`, `ProgressBar`) to read attrs from.

Composes with `Presence` for per-toast enter/exit animation. The toast machine's `ctx.open` field drives the Presence `present` prop. The `on_close_complete` callback fires when Presence transitions to Unmounted and triggers toast removal from the visible list.

z-index allocation: each toast requests a z-index from the `ZIndexAllocator` context (if provided). The allocated index is applied as inline style on the `Root`.

## 7. Prop Sync and Event Mapping

| Adapter prop         | Mode                       | Sync trigger     | Machine event / update path                      | Visible effect                                    | Notes                                           |
| -------------------- | -------------------------- | ---------------- | ------------------------------------------------ | ------------------------------------------------- | ----------------------------------------------- |
| `placement`          | non-reactive provider prop | render time only | stored in provider context                       | determines region positioning and swipe direction | post-mount changes not supported                |
| `max_visible`        | non-reactive provider prop | render time only | stored in provider context                       | controls queuing threshold                        | excess toasts queued until visible ones dismiss |
| `pause_on_hover`     | non-reactive provider prop | render time only | controls region pointer listener attachment      | region-level hover pauses all timers              | default: true                                   |
| `pause_on_page_idle` | non-reactive provider prop | render time only | controls visibilitychange listener               | page blur pauses all timers                       | default: true                                   |
| `hotkey`             | non-reactive provider prop | render time only | controls keydown listener registration           | keyboard shortcut focuses region                  | client-only listener                            |
| per-toast `duration` | init-only per toast        | toast creation   | `toast::Props.duration` passed to `Service::new` | auto-dismiss timer length                         | `None` = indefinite (no auto-dismiss)           |
| per-toast `kind`     | init-only per toast        | toast creation   | `toast::Props.kind` passed to `Service::new`     | routes to polite or assertive region              | immutable after creation                        |

| UI event                  | Preconditions                                        | Machine event / callback path                | Ordering notes                            | Notes                                          |
| ------------------------- | ---------------------------------------------------- | -------------------------------------------- | ----------------------------------------- | ---------------------------------------------- |
| `pointerenter` on Root    | toast visible, `pause_on_hover` enabled or per-toast | `Event::Pause`                               | fires before any child interaction        | pauses auto-dismiss timer                      |
| `pointerleave` on Root    | toast was paused via hover                           | `Event::Resume`                              | fires after pointer exits root bounds     | resumes with remaining duration                |
| `focusin` on Root         | toast visible                                        | `Event::Pause`                               | fires when keyboard/screen reader enters  | pauses timer for accessibility                 |
| `focusout` on Root        | focus leaves entire toast                            | `Event::Resume`                              | verify `relatedTarget` is outside root    | prevents false resumes on internal focus moves |
| `pointerenter` on Region  | `pause_on_hover` enabled at provider level           | `provider::Event::PauseAll`                  | pauses all visible toasts                 | region-level hover                             |
| `pointerleave` on Region  | region was paused                                    | `provider::Event::ResumeAll`                 | resumes all visible toasts                | region-level leave                             |
| `pointerdown` on Root     | toast visible or paused, not dismissing              | `Event::SwipeStart(offset)`                  | begins swipe tracking                     | records initial pointer position               |
| `pointermove` on document | active swipe in progress                             | `Event::SwipeMove(offset)`                   | updates swipe offset                      | applies CSS transform for visual feedback      |
| `pointerup` on document   | active swipe in progress                             | `Event::SwipeEnd { velocity, offset }`       | ends swipe; dismiss if threshold exceeded | removes document listeners                     |
| click on CloseTrigger     | toast has close trigger                              | `Event::Dismiss`                             | fires `api.on_close_trigger_click()`      | triggers exit animation                        |
| click on ActionTrigger    | toast has action trigger                             | consumer callback                            | action callback fires before any dismiss  | consumer-provided action handler               |
| `visibilitychange`        | `pause_on_page_idle` enabled                         | `PauseAll` on hidden, `ResumeAll` on visible | document-level listener                   | client-only                                    |
| hotkey keydown            | `hotkey` configured                                  | moves focus to region                        | global keydown listener                   | client-only                                    |

## 8. Registration and Cleanup Contract

| Registered entity         | Registration trigger                  | Identity key      | Cleanup trigger                                    | Cleanup action                         | Notes                                        |
| ------------------------- | ------------------------------------- | ----------------- | -------------------------------------------------- | -------------------------------------- | -------------------------------------------- |
| `QueueContext` context    | `Provider` mount                      | provider instance | `Provider` unmount                                 | context dropped                        | single provider per app recommended          |
| per-toast `Service`       | toast added to visible list           | toast ID          | toast reaches `Dismissed` state + Presence unmount | `Service` dropped, all effects cleaned | machine effects (timers) auto-cancel on drop |
| region pointer listener   | `Region` client mount                 | region instance   | `Region` cleanup                                   | remove listener                        | for pause-on-hover                           |
| region focus listener     | `Region` client mount                 | region instance   | `Region` cleanup                                   | remove listener                        | for pause-on-focus-within                    |
| document pointermove/up   | swipe start on a toast                | per-swipe session | swipe end or toast cleanup                         | remove document listeners              | must not leak on early dismiss               |
| visibilitychange listener | provider client mount                 | provider instance | provider cleanup                                   | remove listener                        | for pause-on-page-idle                       |
| hotkey keydown listener   | provider client mount when hotkey set | provider instance | provider cleanup                                   | remove listener                        | global keyboard shortcut                     |
| z-index allocation        | per-toast mount                       | toast ID          | per-toast unmount                                  | release z-index                        | via `ZIndexAllocator` if present             |
| announcement queue timer  | first toast announcement              | provider instance | provider cleanup                                   | cancel pending timer                   | 500ms inter-announcement gap                 |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule | Notes                                                                   |
| ------------------ | ------------- | ------------- | ---------------------------------- | ---------------- | ----------------------------------------------------------------------- |
| Region (polite)    | yes           | adapter-owned | required after mount               | no composition   | needed for region-level pointer/focus listeners and hotkey focus target |
| Region (assertive) | yes           | adapter-owned | required after mount               | no composition   | same as polite region                                                   |
| per-toast Root     | yes           | adapter-owned | required after mount               | no composition   | needed for swipe pointer tracking and contains() checks for focusout    |
| CloseTrigger       | no            | adapter-owned | always structural, handle optional | no composition   | click handler only                                                      |
| ActionTrigger      | no            | adapter-owned | always structural, handle optional | no composition   | click handler only                                                      |
| ProgressBar        | no            | adapter-owned | always structural, handle optional | no composition   | CSS custom property set via style attr                                  |

## 10. State Machine Boundary Rules

- machine-owned state (per toast): `State` (Visible/Paused/Dismissing/Dismissed), `Context` fields (remaining_ms, timer_started_at, paused, swiping, swipe_offset, open), all timer effects.
- adapter-local derived bookkeeping: visible toast list ordering, queued toast list, region-level pause-all state, announcement queue and timing, swipe velocity calculation, progress bar animation frame state, Presence composition wiring.
- forbidden local mirrors: do not maintain a separate per-toast "is paused" or "is dismissing" signal that can diverge from the per-toast machine state. Do not track timer remaining outside the machine context.
- allowed snapshot-read contexts: render derivation for toast attrs, pointer event handlers for swipe calculations, announcement scheduling, progress bar value computation.

## 11. Callback Payload Contract

| Callback                                      | Payload source             | Payload shape                                                 | Timing                                                           | Cancelable? | Notes                                                                 |
| --------------------------------------------- | -------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------- | ----------- | --------------------------------------------------------------------- |
| `on_close_complete` (per toast, via Presence) | machine-derived snapshot   | `String` (toast ID)                                           | after Presence transitions to Unmounted following exit animation | no          | adapter removes toast from visible list and queues next if applicable |
| `on_pause_change` (per toast config)          | machine-derived snapshot   | `bool` (paused state)                                         | after `Event::Pause` or `Event::Resume` transition completes     | no          | observational; consumer cannot prevent pause/resume                   |
| action trigger click                          | raw framework event        | Leptos `ev::MouseEvent`                                       | on click, before any auto-dismiss reset                          | no          | consumer handles the action; adapter does not auto-dismiss on action  |
| promise success/error                         | normalized adapter payload | consumer-provided `Content` from the success/error mapping fn | after spawned future resolves                                    | no          | adapter calls `toaster.update(id, ...)` with the mapped content       |

## 12. Failure and Degradation Rules

| Condition                                                         | Policy             | Notes                                                                  |
| ----------------------------------------------------------------- | ------------------ | ---------------------------------------------------------------------- |
| `use_toast()` called outside `Provider`                           | fail fast          | panics with descriptive message; toast context must be provided        |
| per-toast root ref missing after mount                            | fail fast          | swipe and focusout detection require a concrete node handle            |
| region ref missing after mount                                    | degrade gracefully | region-level pause-on-hover disabled; per-toast pause still works      |
| `aria-live` region absent in SSR HTML                             | warn and ignore    | log dev warning; screen readers may miss early toasts                  |
| browser APIs unavailable during SSR (timers, pointer, visibility) | no-op              | structure renders; all interactive behavior deferred to mount          |
| promise future completes after toast already dismissed            | no-op              | result silently discarded                                              |
| announcement queue exceeds 10 pending items                       | warn and ignore    | dev-mode warning; queue is not capped, all toasts eventually announced |

## 13. Identity and Key Policy

| Registered or repeated structure     | Identity source                                 | Duplicates allowed?                                      | DOM order must match registration order? | SSR/hydration stability                           | Notes                                |
| ------------------------------------ | ----------------------------------------------- | -------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------- | ------------------------------------ |
| per-toast instance                   | data-derived (toast ID from `Toaster.create()`) | no (IDs are unique; deduplication prevents same content) | yes, within each region                  | toast IDs must be deterministic or skipped in SSR | new toasts are client-only creations |
| region containers (polite/assertive) | instance-derived (singleton per provider)       | not applicable                                           | not applicable                           | must be stable across hydration                   | SSR renders both empty regions       |
| provider context                     | instance-derived                                | not applicable                                           | not applicable                           | stable across hydration                           | single provider per application      |

## 14. SSR and Client Boundary Rules

- Both `aria-live` region containers (polite and assertive) MUST render in SSR HTML with correct `aria-live`, `role`, and `aria-label` attributes. Screen readers only track mutations to live regions present when the page loads.
- Region containers render empty on the server (no toasts exist at SSR time).
- Per-toast `Service` creation, timer setup (`Event::Init`), swipe listeners, pause-on-hover, pause-on-page-idle, hotkey, and announcement coordination are all client-only.
- `Event::Init` MUST be sent inside an `on_mount` / `Effect::new` guard so timers do not start during SSR where `performance_now()` and `set_timeout` are unavailable.
- Hydration: the two empty region divs must produce identical server and client HTML. No dynamic toast content may be injected before hydration completes.

## 15. Performance Constraints

- Per-toast root attrs must be memoized or derived from the machine, not rebuilt from ad hoc logic each render cycle.
- Region-level pointer/focus listeners must not churn on toast additions/removals; attach once on mount, remove on cleanup.
- Swipe `pointermove` handler must not cause reactive updates on every pixel; batch offset updates or use `requestAnimationFrame` throttling.
- Progress bar `--ars-toast-progress` updates should use `requestAnimationFrame` rather than reactive signal updates to avoid layout thrashing.
- Announcement queue timer should use a single repeating timer, not one timer per pending announcement.
- Toast removal after `Dismissed` state should be deferred by `remove_delay` (default 200ms) to allow exit animation CSS transitions to complete before DOM removal.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                                                                       | Notes                                          |
| ------------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `Presence`          | required    | composition contract    | per-toast enter/exit animation lifecycle; `ctx.open` drives present prop; `on_exit_complete` triggers removal | core spec mandates animation coordination      |
| `z-index-allocator` | recommended | context contract        | per-toast z-index stacking within the region                                                                  | degrades to unmanaged z-index if absent        |
| `ars-provider`      | recommended | context contract        | scoped DOM queries for document-level swipe listeners and focusout containment                                | useful but not blocking                        |
| `dismissable`       | conceptual  | behavioral prerequisite | swipe-to-dismiss and close-trigger patterns share dismiss normalization concepts                              | not composed directly but informs dismiss flow |

## 17. Recommended Implementation Sequence

1. Implement `Provider` with context provision, reactive toast list signal, and `Toaster` imperative handle.
2. Implement `Region` rendering both `aria-live` containers in SSR-safe HTML.
3. Implement per-toast `Service<toast::Machine>` lifecycle: create on add, send `Event::Init` on mount, drop on removal.
4. Wire pause-on-hover and pause-on-focus-within at both region level (all toasts) and per-toast level.
5. Implement per-toast part components (`Title`, `Description`, `ActionTrigger`, `CloseTrigger`).
6. Implement swipe-to-dismiss pointer tracking with document-level move/up listeners.
7. Implement `ProgressBar` with `requestAnimationFrame`-driven CSS custom property updates.
8. Implement announcement coordination with 500ms inter-announcement gap.
9. Add `pause_on_page_idle` via `visibilitychange` listener.
10. Add hotkey support via global `keydown` listener.
11. Implement promise toast via `spawn_local` with loading-to-success/error transitions.
12. Add queuing logic for `max_visible` overflow.
13. Add deduplication check on `Add`.
14. Verify cleanup: all timers, listeners, z-index allocations, and per-toast Services release on provider unmount.

## 18. Anti-Patterns

- Do not create `aria-live` regions dynamically on the client; they must exist in server HTML for screen readers to track.
- Do not start auto-dismiss timers during SSR; guard `Event::Init` with a mount effect.
- Do not maintain a local "remaining time" signal outside the per-toast machine context; the machine owns timer state.
- Do not use `setInterval` for progress bar animation; use `requestAnimationFrame` for smooth visual updates without layout thrashing.
- Do not attach per-toast document-level swipe listeners at render time; attach on `pointerdown` and remove on `pointerup` or cleanup.
- Do not batch multiple toast announcements into a single summary; each toast must be announced individually.
- Do not dismiss a toast synchronously on action trigger click; the consumer callback decides whether to dismiss.
- Do not remove a toast from the DOM immediately on `Dismissed` state; wait `remove_delay` for exit animation.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `use_toast()` returns a stable `Toaster` handle for the lifetime of the provider.
- Consumers may assume toast IDs returned by `Toaster.create()` are unique and usable for `dismiss()` and `update()`.
- Consumers may assume both `aria-live` regions are present in the initial server HTML.
- Consumers may assume pause-on-hover and pause-on-focus-within work out of the box when `pause_on_hover=true` (default).
- Consumers may assume toasts exceeding `max_visible` are queued and shown in FIFO order as visible toasts dismiss.
- Consumers must not assume toasts render during SSR; toast creation is a client-only imperative action.
- Consumers must not assume the action trigger auto-dismisses the toast; explicit `Toaster.dismiss()` is required if desired.
- Consumers must not assume promise toast future results are delivered if the toast was dismissed before resolution.

## 20. Platform Support Matrix

| Capability / behavior           | Browser client | SSR           | Notes                                                                      |
| ------------------------------- | -------------- | ------------- | -------------------------------------------------------------------------- |
| dual aria-live region rendering | full support   | full support  | regions render empty on server; must be present for screen reader tracking |
| toast creation / dismissal      | full support   | not available | imperative API is client-only                                              |
| auto-dismiss timer              | full support   | not available | `Event::Init` guarded by mount effect                                      |
| pause-on-hover / pause-on-focus | full support   | not available | pointer and focus listeners are client-only                                |
| swipe-to-dismiss                | full support   | not available | pointer event tracking is client-only                                      |
| progress bar animation          | full support   | not available | `requestAnimationFrame` is client-only                                     |
| pause-on-page-idle              | full support   | not available | `visibilitychange` listener is client-only                                 |
| hotkey focus                    | full support   | not available | global keydown listener is client-only                                     |
| promise toast                   | full support   | not available | `spawn_local` is client-only                                               |
| announcement coordination       | full support   | not available | timer-based queue draining is client-only                                  |

## 21. Debug Diagnostics and Production Policy

| Condition                                  | Debug build behavior | Production behavior | Notes                                                              |
| ------------------------------------------ | -------------------- | ------------------- | ------------------------------------------------------------------ |
| `use_toast()` called outside provider      | fail fast            | fail fast           | panic with descriptive message in both builds                      |
| `aria-live` region missing from SSR output | debug warning        | no-op               | dev-time check only; production trusts build correctness           |
| announcement queue exceeds 10 items        | debug warning        | no-op               | suggests reducing toast frequency                                  |
| toast duration below 5000ms (WCAG minimum) | debug warning        | warn and ignore     | accessibility timing concern per WCAG 2.2.1                        |
| promise future panics                      | fail fast            | degrade gracefully  | dev panics; production logs error and leaves loading toast visible |
| per-toast root ref unavailable after mount | debug warning        | degrade gracefully  | swipe disabled for that toast; timer still works                   |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                          | Reused by                       | Notes                                                             |
| -------------------------- | ----------- | ------------------------------------------------------- | ------------------------------- | ----------------------------------------------------------------- |
| Presence composition       | required    | per-toast enter/exit animation lifecycle                | toast, dialog, popover, tooltip | drives `ctx.open` to `present` prop mapping                       |
| z-index allocation         | recommended | per-toast stacking order within region                  | toast, dialog, popover          | via `ZIndexAllocator` context                                     |
| announcement timing helper | required    | 500ms inter-announcement gap with queue draining        | toast only                      | manages `VecDeque` and single repeating timer                     |
| swipe gesture helper       | recommended | pointer tracking, velocity calculation, threshold check | toast, carousel, drawer         | normalize `pointerdown`/`pointermove`/`pointerup` to swipe events |
| platform effect guard      | required    | ensures timer/listener setup only runs client-side      | all overlay components          | wraps `Event::Init` and listener attachment in mount effects      |

## 23. Framework-Specific Behavior

Leptos 0.8.x specifics:

- `provide_context(QueueContext { ... })` for the imperative handle; `use_context::<QueueContext>()` to retrieve.
- `signal()` for the reactive toast list: `let (toasts, set_toasts) = signal(Vec::<Entry>::new());`
- `Effect::new` to guard `Event::Init` dispatch (runs only on client after mount).
- `on_cleanup` for region/provider listener teardown and per-toast Service cleanup.
- `NodeRef::<html::Div>::new()` for region and per-toast root refs.
- `spawn_local` for promise toast future execution.
- `StoredValue` for non-reactive provider config (placement, max_visible, default_durations).
- Progress bar uses `request_animation_frame` from `leptos::web_sys` for smooth updates.
- Swipe velocity computed from timestamp deltas between `pointermove` events.

## 24. Canonical Implementation Sketch

```rust,no_check
use leptos::prelude::*;

#[component]
pub fn Provider(
    #[prop(optional)] placement: Option<toast::Placement>,
    #[prop(optional)] max_visible: Option<usize>,
    #[prop(optional)] messages: Option<toast::Messages>,
    children: Children,
) -> impl IntoView {
    let placement = placement.unwrap_or_default();
    let max = max_visible.unwrap_or(5);
    let messages = messages.unwrap_or_default();
    let locale = resolve_locale(None);

    let (toasts, set_toasts) = signal(Vec::<Entry>::new());
    let (queue, set_queue) = signal(Vec::<toast::provider::Config>::new());

    let toaster = Toaster::new(set_toasts, set_queue, max);
    provide_context(QueueContext {
        toasts,
        toaster: toaster.clone(),
        placement,
        max_visible: max,
        messages: messages.clone(),
        locale: locale.clone(),
    });

    view! {
        {children()}
        <Region />
    }
}

#[component]
fn Region() -> impl IntoView {
    let ctx = use_context::<QueueContext>()
        .expect("toast::Region must be inside a Provider");
    let polite_ref = NodeRef::<html::Div>::new();
    let assertive_ref = NodeRef::<html::Div>::new();

    let polite_attrs = toast::region_attrs(&ctx.messages, &ctx.locale, false);
    let assertive_attrs = toast::region_attrs(&ctx.messages, &ctx.locale, true);

    // Client-only: region-level pause-on-hover
    Effect::new(move |_| {
        // attach pointerenter/pointerleave on region refs
        // -> PauseAll / ResumeAll for all visible toasts
    });

    view! {
        <div {..polite_attrs} node_ref=polite_ref>
            <For
                each=move || ctx.toasts.get().into_iter()
                    .filter(|t| matches!(t.kind, toast::Kind::Info | toast::Kind::Success | toast::Kind::Loading))
                key=|t| t.id.clone()
                let:entry
            >
                <Root toast_id=entry.id.clone() />
            </For>
        </div>
        <div {..assertive_attrs} node_ref=assertive_ref>
            <For
                each=move || ctx.toasts.get().into_iter()
                    .filter(|t| matches!(t.kind, toast::Kind::Error | toast::Kind::Warning))
                key=|t| t.id.clone()
                let:entry
            >
                <Root toast_id=entry.id.clone() />
            </For>
        </div>
    }
}

#[component]
fn Root(toast_id: String) -> impl IntoView {
    let root_ref = NodeRef::<html::Div>::new();
    let ctx = use_context::<QueueContext>()
        .expect("toast::Root requires QueueContext");

    // Create per-toast Service
    let props = /* build toast::Props from entry config */;
    let service = Service::<toast::Machine>::new(props);

    // Client-only: send Init after mount to start auto-dismiss timer
    Effect::new(move |_| {
        service.send(toast::Event::Init);
    });

    // Pause-on-hover per toast
    // on:pointerenter -> service.send(toast::Event::Pause)
    // on:pointerleave -> service.send(toast::Event::Resume)
    // on:focusin -> service.send(toast::Event::Pause)
    // on:focusout (if relatedTarget outside root) -> service.send(toast::Event::Resume)

    let root_attrs = service.derive(|api| api.root_attrs());

    // Provide per-toast context for child parts
    provide_context(Context { service: service.clone(), toast_id: toast_id.clone() });

    // Compose with Presence for enter/exit animation
    // present = service.derive(|api| api.is_visible() || matches!(state, Dismissing))
    // on_exit_complete -> remove toast from visible list

    view! {
        <div
            node_ref=root_ref
            {..root_attrs.get()}
            on:pointerenter=move |_| service.send(toast::Event::Pause)
            on:pointerleave=move |_| service.send(toast::Event::Resume)
            on:focusin=move |_| service.send(toast::Event::Pause)
            on:focusout=move |ev| {
                // Only resume if focus left the toast entirely
                // Check ev.related_target() is outside root_ref
                service.send(toast::Event::Resume);
            }
        >
            // Consumer-provided render callback fills in Title, Description, etc.
        </div>
    }
}

pub fn use_toast() -> Toaster {
    use_context::<QueueContext>()
        .expect("use_toast() must be called within a Provider")
        .toaster
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Provider
let (toasts, set_toasts) = signal(Vec::new());
let (queue, set_queue) = signal(Vec::new());
let toaster = Toaster::new(set_toasts, set_queue, max_visible);
provide_context(QueueContext { toasts, toaster, placement, messages, locale });

// Region
let polite_ref = create_region_ref();
let assertive_ref = create_region_ref();
render_dual_aria_live_regions(polite_ref, assertive_ref, region_attrs);
attach_region_pause_on_hover_listeners(polite_ref, assertive_ref);
attach_visibility_change_listener_if_enabled();
attach_hotkey_listener_if_configured();

// Per-toast lifecycle
for_each_toast(|entry| {
    let service = Service::<toast::Machine>::new(entry.to_props());
    let root_ref = create_root_ref();

    on_mount(|| service.send(toast::Event::Init));  // start timer client-only

    derive_and_spread_root_attrs(service);
    attach_per_toast_pointer_listeners(root_ref, service);  // hover, swipe
    attach_per_toast_focus_listeners(root_ref, service);    // focusin/focusout

    compose_with_presence(service.derive(|api| api.is_visible()), |on_exit| {
        remove_toast_from_list(entry.id);
        dequeue_next_toast_if_queued();
    });

    provide_per_toast_context(service, entry.id);
    render_toast_root_and_consumer_content(root_ref);

    on_cleanup(|| {
        release_z_index(entry.id);
        remove_swipe_document_listeners();
    });
});

// Announcement coordination
let announcement_queue = VecDeque::new();
let announcement_timer = create_repeating_timer(500, || {
    drain_one_announcement(announcement_queue);
});
on_cleanup(|| cancel_announcement_timer());
```

## 26. Adapter Invariants

- Both `aria-live` region containers must be present in server-rendered HTML and must persist for the application lifetime.
- `Event::Init` must be sent inside a client-only mount effect, never during SSR.
- Each toast must have exactly one `Service<toast::Machine>` instance; the adapter must not share machines across toasts.
- Timer effects managed by the machine (auto-dismiss, exit animation fallback) are cleaned up automatically when the Service is dropped; the adapter must not cancel them independently.
- Swipe document-level listeners (`pointermove`, `pointerup`) must attach on `pointerdown` and detach on `pointerup` or toast cleanup, whichever comes first.
- The `focusout` handler must verify `relatedTarget` is outside the toast root before sending `Event::Resume` to prevent false resumes on internal focus moves.
- Toast removal from the DOM must be deferred by `remove_delay` after `Dismissed` state to allow CSS exit transitions.
- Announcement coordination must respect the 500ms inter-announcement gap; toasts are never batched.
- Promise toast futures must be spawned via `spawn_local`; if the toast is dismissed before the future completes, the result is silently discarded.
- The adapter must route toasts to the correct `aria-live` region based on `Kind`: info/success/loading to polite, error/warning to assertive.
- Deduplication must check visible and queued toasts for matching `kind` + `title` + `description` before creating a new toast.

## 27. Accessibility and SSR Notes

- Dual `aria-live` regions in SSR HTML ensure screen readers track mutations from page load.
- The polite region uses `role="status"` and `aria-live="polite"`; the assertive region uses `role="alert"` and `aria-live="assertive"`.
- Both regions carry `aria-label` from `messages.region_label` for landmark identification.
- `aria-atomic="false"` on regions ensures individual toast insertions are announced, not the entire region content.
- `ActionTrigger` carries `aria-label` from the consumer-provided `alt_text` to give screen readers full action context beyond the button label.
- `CloseTrigger` carries `aria-label` from `messages.dismiss_label`.
- Auto-dismiss pauses on hover and focus-within to satisfy WCAG 2.2.1 (Timing Adjustable).
- Minimum toast display duration of 5000ms is enforced by default; durations below this threshold trigger a debug warning.
- `ProgressBar` uses `role="progressbar"` with `aria-valuenow` (0-100), `aria-valuemin="0"`, `aria-valuemax="100"` but does not announce progress changes via `aria-live` (visual enhancement only).

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, event, and manager behavior parity. All adapter-level features (dual regions, imperative `Toaster` API, promise toasts, swipe-to-dismiss, progress bar, pause-on-hover, pause-on-page-idle, hotkey, queuing, deduplication, announcement coordination) are mapped.

Intentional deviations: none. The Leptos adapter is web-only, so all platform capabilities (pointer events, `requestAnimationFrame`, `visibilitychange`, `performance.now()`) are available in the browser client target.

Traceability note: this adapter spec makes explicit the SSR region obligation, per-toast machine lifecycle, mount-guarded `Event::Init`, swipe listener attachment/detachment lifecycle, announcement queue timing, and promise future spawning via `spawn_local`.

## 29. Test Scenarios

- dual aria-live regions present in SSR HTML with correct attributes
- toast creation via `use_toast()` adds toast to the correct region by kind
- auto-dismiss timer fires after configured duration
- pause-on-hover pauses and resumes timer on pointerenter/pointerleave
- pause-on-focus-within pauses timer on focusin, resumes on focusout leaving root
- region-level hover pauses all visible toasts
- swipe-to-dismiss: swipe beyond threshold dismisses; below threshold snaps back
- close trigger click dispatches `Event::Dismiss`
- action trigger click fires consumer callback without auto-dismissing
- progress bar updates `--ars-toast-progress` from 0 to 1 over duration
- promise toast shows loading, transitions to success or error on resolve
- toast exceeding `max_visible` is queued; shown when a visible toast dismisses
- deduplication resets timer instead of creating duplicate
- announcement coordination: rapid toasts are spaced 500ms apart
- hotkey focuses the toast region
- pause-on-page-idle pauses all timers on `visibilitychange` hidden
- `Toaster.dismiss_all()` dismisses all visible toasts
- exit animation via Presence completes before DOM removal
- cleanup: all listeners, timers, and z-index allocations released on provider unmount

## 30. Test Oracle Notes

| Behavior                            | Preferred oracle type | Notes                                                                      |
| ----------------------------------- | --------------------- | -------------------------------------------------------------------------- |
| dual region presence and attrs      | rendered structure    | assert both `aria-live` regions with correct role/label in SSR output      |
| toast routing by kind               | rendered structure    | assert info/success/loading in polite region, error/warning in assertive   |
| per-toast machine state transitions | machine state         | assert Visible -> Paused -> Visible and Visible -> Dismissing -> Dismissed |
| auto-dismiss timer                  | callback order        | assert `DurationExpired` fires after configured ms                         |
| pause/resume timer                  | machine state         | assert remaining_ms decreases, pauses on hover, resumes on leave           |
| swipe gesture                       | DOM attrs             | assert `data-ars-swiping` and swipe offset transform during swipe          |
| announcement timing                 | callback order        | assert 500ms gap between consecutive announcements                         |
| context registration                | context registration  | assert `QueueContext` available to descendants after provider mount        |
| cleanup side effects                | cleanup side effects  | assert all document listeners, timers, and z-index allocations removed     |
| hydration stability                 | hydration structure   | assert server and client HTML match for the two empty region containers    |

Cheap verification recipe:

1. Mount `Provider` and assert both `aria-live` region divs are present with correct attrs.
2. Call `use_toast().create(...)` with an info toast and assert it appears in the polite region with correct `data-ars-state="visible"`.
3. Simulate `pointerenter` on the toast root, assert state transitions to Paused.
4. Simulate `pointerleave`, assert state returns to Visible.
5. Wait for duration expiry, assert state transitions through Dismissing to Dismissed.
6. Verify the toast is removed from the DOM after `remove_delay`.
7. Create an error toast, assert it appears in the assertive region.
8. Unmount the provider, assert no dangling document listeners or timers.

## 31. Implementation Checklist

- [ ] `Provider` provides `QueueContext` with `Toaster` handle via `provide_context`.
- [ ] Both `aria-live` regions render in SSR HTML with correct `aria-live`, `role`, and `aria-label`.
- [ ] `use_toast()` returns a stable `Toaster` handle; panics outside provider.
- [ ] Per-toast `Service<toast::Machine>` created on add, dropped on removal.
- [ ] `Event::Init` sent inside a client-only mount effect (not during SSR).
- [ ] Per-toast hover/focus listeners wire `Event::Pause` / `Event::Resume`.
- [ ] `focusout` handler checks `relatedTarget` before resuming.
- [ ] Region-level hover pauses/resumes all visible toasts.
- [ ] Swipe-to-dismiss attaches document listeners on `pointerdown`, removes on `pointerup`/cleanup.
- [ ] `CloseTrigger` click dispatches `Event::Dismiss`.
- [ ] `ActionTrigger` fires consumer callback; does not auto-dismiss.
- [ ] `ProgressBar` drives `--ars-toast-progress` via `requestAnimationFrame`.
- [ ] Presence composition drives enter/exit animation per toast.
- [ ] Toast removal deferred by `remove_delay` after Dismissed state.
- [ ] Toasts routed to correct region by `Kind`.
- [ ] Queuing logic for `max_visible` overflow.
- [ ] Deduplication check on toast creation.
- [ ] Announcement queue drains at 500ms intervals.
- [ ] `pause_on_page_idle` via `visibilitychange` listener.
- [ ] Hotkey via global `keydown` listener.
- [ ] Promise toast spawned via `spawn_local`; loading -> success/error on resolve.
- [ ] All listeners, timers, z-index allocations cleaned up on unmount.
