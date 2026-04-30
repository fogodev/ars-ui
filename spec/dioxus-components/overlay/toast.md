---
adapter: dioxus
component: toast
category: overlay
source: components/overlay/toast.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Toast -- Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Toast`](../../components/overlay/toast.md) notification system to Dioxus 0.7.x. The adapter owns: the `Provider` context that holds the imperative queue, the `Region` that renders dual `aria-live` containers (polite and assertive), per-toast `Service` lifecycle management, swipe-to-dismiss pointer handling (web only), auto-dismiss timer wiring (including pause-on-hover and pause-on-page-idle), progress bar animation, promise toast future spawning, and announcement coordination timing. On Desktop and Mobile targets, swipe-to-dismiss and `visibilitychange` degrade gracefully.

## 2. Public Adapter API

```rust,no_check
/// Provider props.
#[derive(Props, Clone, PartialEq)]
pub struct ProviderProps {
    #[props(optional)]
    pub placement: Option<toast::Placement>,
    #[props(optional)]
    pub max_visible: Option<usize>,
    #[props(optional)]
    pub pause_on_hover: Option<bool>,
    #[props(optional)]
    pub pause_on_page_idle: Option<bool>,
    #[props(optional)]
    pub offsets: Option<toast::EdgeOffsets>,
    #[props(optional)]
    pub overlap: Option<bool>,
    #[props(optional)]
    pub hotkey: Option<String>,
    #[props(optional)]
    pub remove_delay: Option<u32>,
    #[props(optional)]
    pub default_durations: Option<toast::DefaultDurations>,
    #[props(optional)]
    pub deduplicate_all: Option<bool>,
    #[props(optional)]
    pub messages: Option<toast::Messages>,
    #[props(optional)]
    pub locale: Option<Locale>,
    pub children: Element,
}

/// Provider component. Place at the application root so that the dual
/// aria-live regions are present in server-rendered HTML (web targets).
#[component]
pub fn Provider(props: ProviderProps) -> Element

/// Imperative hook. Returns a Toaster handle for creating, updating,
/// dismissing, and promise-tracking toasts from anywhere in the tree.
pub fn use_toast() -> Toaster

/// Region component rendered internally by Provider. Contains the
/// dual aria-live containers and all visible toast instances.
#[component]
fn Region() -> Element

/// Per-toast root props.
#[derive(Props, Clone, PartialEq)]
pub struct RootProps {
    #[props(into)]
    pub toast_id: String,
}

/// Per-toast root component. Rendered by the region for each active toast.
#[component]
fn Root(props: RootProps) -> Element

/// Per-toast title props.
#[derive(Props, Clone, PartialEq)]
pub struct TitleProps {
    pub children: Element,
}

/// Per-toast title slot.
#[component]
pub fn Title(props: TitleProps) -> Element

/// Per-toast description props.
#[derive(Props, Clone, PartialEq)]
pub struct DescriptionProps {
    pub children: Element,
}

/// Per-toast description slot.
#[component]
pub fn Description(props: DescriptionProps) -> Element

/// Per-toast action button props.
#[derive(Props, Clone, PartialEq)]
pub struct ActionTriggerProps {
    #[props(into)]
    pub alt_text: String,
    pub children: Element,
}

/// Per-toast action button with mandatory alt_text for screen readers.
#[component]
pub fn ActionTrigger(props: ActionTriggerProps) -> Element

/// Per-toast close button props.
#[derive(Props, Clone, PartialEq)]
pub struct CloseTriggerProps {
    pub children: Element,
}

/// Per-toast close button.
#[component]
pub fn CloseTrigger(props: CloseTriggerProps) -> Element

/// Progress bar for time-remaining visualization.
#[component]
pub fn ProgressBar() -> Element
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with `toast::Props`, `toast::provider::Props`, and `toast::provider::Config`.
- Event parity: `Init`, `Pause`, `Resume`, `Dismiss`, `SwipeStart`, `SwipeMove`, `SwipeEnd`, `DurationExpired`, and `AnimationComplete` are all adapter-driven per-toast machine events.
- Manager parity: `provider::Event::Add`, `Update`, `Remove`, `PauseAll`, `ResumeAll`, and `DismissAll` map to the `Manager` and are exposed through the `Toaster` imperative handle.
- Structure parity: dual `aria-live` regions, per-toast `Root`/`Title`/`Description`/`ActionTrigger`/`CloseTrigger`/`ProgressBar` parts.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                            | Ownership                                       | Attr source                                    | Notes                                                             |
| --------------------- | ----------- | --------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------- |
| Region (polite)       | required    | `<div>` with `aria-live="polite"` `role="status"`   | adapter-owned, rendered by `Region`             | `toast::region_attrs(messages, locale, false)` | Must exist in SSR HTML on web.                                    |
| Region (assertive)    | required    | `<div>` with `aria-live="assertive"` `role="alert"` | adapter-owned, rendered by `Region`             | `toast::region_attrs(messages, locale, true)`  | Must exist in SSR HTML on web.                                    |
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

`Provider` provides a `QueueContext` via `use_context_provider`. The context contains the reactive toast list signal, the `Toaster` imperative handle, and provider-level configuration (placement, max_visible, default_durations). `Region` reads this context via `try_use_context::<QueueContext>()`. Each `Root` creates a per-toast `Service<toast::Machine>` and provides a per-toast context for child part components (`Title`, `Description`, `ActionTrigger`, `CloseTrigger`, `ProgressBar`) to read attrs from.

Composes with `Presence` for per-toast enter/exit animation. The toast machine's `ctx.open` field drives the Presence `present` prop. The `on_close_complete` callback fires when Presence transitions to Unmounted and triggers toast removal from the visible list.

z-index allocation: each toast requests a z-index from the `ZIndexAllocator` context (if provided). The allocated index is applied as inline style on the `Root`.

## 7. Prop Sync and Event Mapping

| Adapter prop         | Mode                       | Sync trigger     | Machine event / update path                      | Visible effect                                    | Notes                                           |
| -------------------- | -------------------------- | ---------------- | ------------------------------------------------ | ------------------------------------------------- | ----------------------------------------------- |
| `placement`          | non-reactive provider prop | render time only | stored in provider context                       | determines region positioning and swipe direction | post-mount changes not supported                |
| `max_visible`        | non-reactive provider prop | render time only | stored in provider context                       | controls queuing threshold                        | excess toasts queued until visible ones dismiss |
| `pause_on_hover`     | non-reactive provider prop | render time only | controls region pointer listener attachment      | region-level hover pauses all timers              | default: true; web only for region-level        |
| `pause_on_page_idle` | non-reactive provider prop | render time only | controls visibilitychange listener               | page blur pauses all timers                       | default: true; web only                         |
| `hotkey`             | non-reactive provider prop | render time only | controls keydown listener registration           | keyboard shortcut focuses region                  | web and desktop                                 |
| per-toast `duration` | init-only per toast        | toast creation   | `toast::Props.duration` passed to `Service::new` | auto-dismiss timer length                         | `None` = indefinite (no auto-dismiss)           |
| per-toast `kind`     | init-only per toast        | toast creation   | `toast::Props.kind` passed to `Service::new`     | routes to polite or assertive region              | immutable after creation                        |

| UI event                  | Preconditions                              | Machine event / callback path                | Ordering notes                            | Notes                                                        |
| ------------------------- | ------------------------------------------ | -------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------ |
| `pointerenter` on Root    | toast visible, `pause_on_hover` enabled    | `Event::Pause`                               | fires before any child interaction        | web; on desktop, per-toast hover may use platform equivalent |
| `pointerleave` on Root    | toast was paused via hover                 | `Event::Resume`                              | fires after pointer exits root bounds     | web; desktop equivalent                                      |
| `focusin` on Root         | toast visible                              | `Event::Pause`                               | fires when keyboard/screen reader enters  | web only                                                     |
| `focusout` on Root        | focus leaves entire toast                  | `Event::Resume`                              | verify related target is outside root     | web only; prevents false resumes                             |
| `pointerenter` on Region  | `pause_on_hover` enabled at provider level | `provider::Event::PauseAll`                  | pauses all visible toasts                 | web only                                                     |
| `pointerleave` on Region  | region was paused                          | `provider::Event::ResumeAll`                 | resumes all visible toasts                | web only                                                     |
| `pointerdown` on Root     | toast visible or paused, not dismissing    | `Event::SwipeStart(offset)`                  | begins swipe tracking                     | web only                                                     |
| `pointermove` on document | active swipe in progress                   | `Event::SwipeMove(offset)`                   | updates swipe offset                      | web only                                                     |
| `pointerup` on document   | active swipe in progress                   | `Event::SwipeEnd { velocity, offset }`       | ends swipe; dismiss if threshold exceeded | web only                                                     |
| click on CloseTrigger     | toast has close trigger                    | `Event::Dismiss`                             | fires `api.on_close_trigger_click()`      | all platforms                                                |
| click on ActionTrigger    | toast has action trigger                   | consumer callback                            | action callback fires before any dismiss  | all platforms                                                |
| `visibilitychange`        | `pause_on_page_idle` enabled               | `PauseAll` on hidden, `ResumeAll` on visible | document-level listener                   | web only                                                     |
| hotkey keydown            | `hotkey` configured                        | moves focus to region                        | global keydown listener                   | web and desktop                                              |

## 8. Registration and Cleanup Contract

| Registered entity         | Registration trigger           | Identity key      | Cleanup trigger                                    | Cleanup action                         | Notes                                        |
| ------------------------- | ------------------------------ | ----------------- | -------------------------------------------------- | -------------------------------------- | -------------------------------------------- |
| `QueueContext` context    | `Provider` mount               | provider instance | `Provider` drop                                    | context dropped via `use_drop`         | single provider per app recommended          |
| per-toast `Service`       | toast added to visible list    | toast ID          | toast reaches `Dismissed` state + Presence unmount | `Service` dropped, all effects cleaned | machine effects (timers) auto-cancel on drop |
| region pointer listener   | `Region` mount on web          | region instance   | `Region` drop                                      | remove listener via `use_drop`         | for pause-on-hover; web only                 |
| region focus listener     | `Region` mount on web          | region instance   | `Region` drop                                      | remove listener via `use_drop`         | for pause-on-focus-within; web only          |
| document pointermove/up   | swipe start on a toast         | per-swipe session | swipe end or toast drop                            | remove document listeners              | web only; must not leak on early dismiss     |
| visibilitychange listener | provider mount on web          | provider instance | provider drop                                      | remove listener via `use_drop`         | for pause-on-page-idle; web only             |
| hotkey keydown listener   | provider mount when hotkey set | provider instance | provider drop                                      | remove listener via `use_drop`         | global keyboard shortcut                     |
| z-index allocation        | per-toast mount                | toast ID          | per-toast unmount                                  | release z-index                        | via `ZIndexAllocator` if present             |
| announcement queue timer  | first toast announcement       | provider instance | provider drop                                      | cancel pending timer                   | 500ms inter-announcement gap                 |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule | Notes                                                                   |
| ------------------ | ------------- | ------------- | ---------------------------------- | ---------------- | ----------------------------------------------------------------------- |
| Region (polite)    | yes on web    | adapter-owned | required after mount               | no composition   | needed for region-level pointer/focus listeners and hotkey focus target |
| Region (assertive) | yes on web    | adapter-owned | required after mount               | no composition   | same as polite region                                                   |
| per-toast Root     | yes on web    | adapter-owned | required after mount               | no composition   | needed for swipe pointer tracking and contains() checks for focusout    |
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
| action trigger click                          | raw framework event        | Dioxus `Event<MouseData>`                                     | on click, before any auto-dismiss reset                          | no          | consumer handles the action; adapter does not auto-dismiss on action  |
| promise success/error                         | normalized adapter payload | consumer-provided `Content` from the success/error mapping fn | after spawned future resolves                                    | no          | adapter calls `toaster.update(id, ...)` with the mapped content       |

## 12. Failure and Degradation Rules

| Condition                                                         | Policy             | Notes                                                                                               |
| ----------------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------------------- |
| `use_toast()` called outside `Provider`                           | fail fast          | panics with descriptive message; toast context must be provided                                     |
| per-toast root ref missing after mount (web)                      | fail fast          | swipe and focusout detection require a concrete node handle                                         |
| region ref missing after mount (web)                              | degrade gracefully | region-level pause-on-hover disabled; per-toast pause still works                                   |
| `aria-live` region absent in SSR HTML                             | warn and ignore    | log dev warning; screen readers may miss early toasts                                               |
| browser APIs unavailable during SSR (timers, pointer, visibility) | no-op              | structure renders; all interactive behavior deferred to mount                                       |
| pointer APIs unavailable on Desktop/Mobile                        | degrade gracefully | swipe-to-dismiss and region-level pointer pause disabled; close trigger and auto-dismiss still work |
| `visibilitychange` unavailable on Desktop/Mobile                  | degrade gracefully | pause-on-page-idle disabled; explicit pause still works                                             |
| promise future completes after toast already dismissed            | no-op              | result silently discarded                                                                           |
| announcement queue exceeds 10 pending items                       | warn and ignore    | dev-mode warning; queue is not capped, all toasts eventually announced                              |

## 13. Identity and Key Policy

| Registered or repeated structure     | Identity source                                 | Duplicates allowed?                                      | DOM order must match registration order? | SSR/hydration stability                           | Notes                                |
| ------------------------------------ | ----------------------------------------------- | -------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------- | ------------------------------------ |
| per-toast instance                   | data-derived (toast ID from `Toaster.create()`) | no (IDs are unique; deduplication prevents same content) | yes, within each region                  | toast IDs must be deterministic or skipped in SSR | new toasts are client-only creations |
| region containers (polite/assertive) | instance-derived (singleton per provider)       | not applicable                                           | not applicable                           | must be stable across hydration (web SSR)         | SSR renders both empty regions       |
| provider context                     | instance-derived                                | not applicable                                           | not applicable                           | stable across hydration                           | single provider per application      |

## 14. SSR and Client Boundary Rules

- Both `aria-live` region containers (polite and assertive) MUST render in SSR HTML with correct `aria-live`, `role`, and `aria-label` attributes. Screen readers only track mutations to live regions present when the page loads.
- Region containers render empty on the server (no toasts exist at SSR time).
- Per-toast `Service` creation, timer setup (`Event::Init`), swipe listeners, pause-on-hover, pause-on-page-idle, hotkey, and announcement coordination are all client-only.
- `Event::Init` MUST be sent inside a `use_effect` guard so timers do not start during SSR where `performance_now()` and `set_timeout` are unavailable.
- Hydration (web SSR): the two empty region divs must produce identical server and client HTML. No dynamic toast content may be injected before hydration completes.
- Desktop and Mobile targets: SSR rules do not apply; regions render at component mount. All timer and listener setup proceeds immediately on mount.

## 15. Performance Constraints

- Per-toast root attrs must be derived from the machine via `use_memo` or equivalent, not rebuilt from ad hoc logic each render.
- Region-level pointer/focus listeners must not churn on toast additions/removals; attach once on mount, remove on drop.
- Swipe `pointermove` handler must not cause reactive updates on every pixel; batch offset updates or use `requestAnimationFrame` throttling (web) or equivalent.
- Progress bar `--ars-toast-progress` updates should use `requestAnimationFrame` (web) rather than reactive signal updates to avoid layout thrashing.
- Announcement queue timer should use a single repeating timer, not one timer per pending announcement.
- Toast removal after `Dismissed` state should be deferred by `remove_delay` (default 200ms) to allow exit animation CSS transitions to complete before DOM removal.
- Dioxus `Signal<T>` is `Copy`; prefer `Signal` over `Rc<RefCell<T>>` for toast list and queue state.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                                                                       | Notes                                           |
| ------------------- | ----------- | ----------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `Presence`          | required    | composition contract    | per-toast enter/exit animation lifecycle; `ctx.open` drives present prop; `on_exit_complete` triggers removal | core spec mandates animation coordination       |
| `z-index-allocator` | recommended | context contract        | per-toast z-index stacking within the region                                                                  | degrades to unmanaged z-index if absent         |
| `ars-provider`      | recommended | context contract        | scoped DOM queries for document-level swipe listeners and focusout containment                                | useful on web; not applicable on desktop/mobile |
| `dismissable`       | conceptual  | behavioral prerequisite | swipe-to-dismiss and close-trigger patterns share dismiss normalization concepts                              | not composed directly but informs dismiss flow  |

## 17. Recommended Implementation Sequence

1. Implement `Provider` with context provision via `use_context_provider`, reactive toast list signal, and `Toaster` imperative handle.
2. Implement `Region` rendering both `aria-live` containers in SSR-safe HTML (web) or immediate mount HTML (desktop/mobile).
3. Implement per-toast `Service<toast::Machine>` lifecycle: create on add, send `Event::Init` on mount, drop on removal.
4. Wire pause-on-hover and pause-on-focus-within at both region level (all toasts) and per-toast level (web).
5. Implement per-toast part components (`Title`, `Description`, `ActionTrigger`, `CloseTrigger`).
6. Implement swipe-to-dismiss pointer tracking with document-level move/up listeners (web only).
7. Implement `ProgressBar` with `requestAnimationFrame`-driven CSS custom property updates (web) or timer-driven updates (desktop/mobile).
8. Implement announcement coordination with 500ms inter-announcement gap (web).
9. Add `pause_on_page_idle` via `visibilitychange` listener (web only).
10. Add hotkey support via global `keydown` listener (web and desktop).
11. Implement promise toast via `spawn` with loading-to-success/error transitions.
12. Add queuing logic for `max_visible` overflow.
13. Add deduplication check on `Add`.
14. Verify cleanup via `use_drop`: all timers, listeners, z-index allocations, and per-toast Services release on provider drop.

## 18. Anti-Patterns

- Do not create `aria-live` regions dynamically on the client (web SSR); they must exist in server HTML for screen readers to track.
- Do not start auto-dismiss timers during SSR; guard `Event::Init` with `use_effect`.
- Do not maintain a local "remaining time" signal outside the per-toast machine context; the machine owns timer state.
- Do not use `setInterval` for progress bar animation; use `requestAnimationFrame` (web) for smooth visual updates without layout thrashing.
- Do not attach per-toast document-level swipe listeners at render time; attach on `pointerdown` and remove on `pointerup` or cleanup.
- Do not batch multiple toast announcements into a single summary; each toast must be announced individually.
- Do not dismiss a toast synchronously on action trigger click; the consumer callback decides whether to dismiss.
- Do not remove a toast from the DOM immediately on `Dismissed` state; wait `remove_delay` for exit animation.
- Do not hold `.read()` or `.write()` guards on signals across `.await` boundaries; clone values out first.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `use_toast()` returns a stable `Toaster` handle for the lifetime of the provider.
- Consumers may assume toast IDs returned by `Toaster.create()` are unique and usable for `dismiss()` and `update()`.
- Consumers may assume both `aria-live` regions are present in the initial server HTML (web SSR targets).
- Consumers may assume pause-on-hover and pause-on-focus-within work on web targets when `pause_on_hover=true` (default).
- Consumers may assume toasts exceeding `max_visible` are queued and shown in FIFO order as visible toasts dismiss.
- Consumers must not assume toasts render during SSR; toast creation is a client-only imperative action.
- Consumers must not assume the action trigger auto-dismisses the toast; explicit `Toaster.dismiss()` is required if desired.
- Consumers must not assume promise toast future results are delivered if the toast was dismissed before resolution.
- Consumers must not assume swipe-to-dismiss or `visibilitychange` pause work on Desktop or Mobile targets.

## 20. Platform Support Matrix

| Capability / behavior           | Web          | Desktop       | Mobile        | SSR           | Notes                                                                            |
| ------------------------------- | ------------ | ------------- | ------------- | ------------- | -------------------------------------------------------------------------------- |
| dual aria-live region rendering | full support | full support  | full support  | full support  | regions render empty on server; must be present for screen reader tracking (web) |
| toast creation / dismissal      | full support | full support  | full support  | not available | imperative API is client/mount-only                                              |
| auto-dismiss timer              | full support | full support  | full support  | not available | `Event::Init` guarded by `use_effect`                                            |
| pause-on-hover (per toast)      | full support | full support  | limited       | not available | mobile has no persistent pointer; desktop webview supports pointer events        |
| pause-on-hover (region)         | full support | full support  | limited       | not available | same as per-toast                                                                |
| pause-on-focus-within           | full support | limited       | limited       | not available | desktop/mobile focus models differ from web                                      |
| swipe-to-dismiss                | full support | not available | not available | not available | pointer event tracking is web-specific                                           |
| progress bar animation          | full support | full support  | full support  | not available | web uses rAF; desktop/mobile use timer fallback                                  |
| pause-on-page-idle              | full support | not available | not available | not available | `visibilitychange` is web-specific                                               |
| hotkey focus                    | full support | full support  | not available | not available | mobile has no global keyboard shortcut                                           |
| promise toast                   | full support | full support  | full support  | not available | `spawn` is available on all client targets                                       |
| announcement coordination       | full support | limited       | limited       | not available | screen reader support varies by platform                                         |

## 21. Debug Diagnostics and Production Policy

| Condition                                        | Debug build behavior | Production behavior | Notes                                                              |
| ------------------------------------------------ | -------------------- | ------------------- | ------------------------------------------------------------------ |
| `use_toast()` called outside provider            | fail fast            | fail fast           | panic with descriptive message in both builds                      |
| `aria-live` region missing from SSR output (web) | debug warning        | no-op               | dev-time check only; production trusts build correctness           |
| announcement queue exceeds 10 items              | debug warning        | no-op               | suggests reducing toast frequency                                  |
| toast duration below 5000ms (WCAG minimum)       | debug warning        | warn and ignore     | accessibility timing concern per WCAG 2.2.1                        |
| promise future panics                            | fail fast            | degrade gracefully  | dev panics; production logs error and leaves loading toast visible |
| per-toast root ref unavailable after mount (web) | debug warning        | degrade gracefully  | swipe disabled for that toast; timer still works                   |
| platform capability missing (swipe on desktop)   | no-op                | no-op               | expected platform limitation; no diagnostic needed                 |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                           | Reused by                       | Notes                                                       |
| -------------------------- | ----------- | ------------------------------------------------------------------------ | ------------------------------- | ----------------------------------------------------------- |
| Presence composition       | required    | per-toast enter/exit animation lifecycle                                 | toast, dialog, popover, tooltip | drives `ctx.open` to `present` prop mapping                 |
| z-index allocation         | recommended | per-toast stacking order within region                                   | toast, dialog, popover          | via `ZIndexAllocator` context                               |
| announcement timing helper | required    | 500ms inter-announcement gap with queue draining                         | toast only                      | manages `VecDeque` and single repeating timer               |
| swipe gesture helper       | recommended | pointer tracking, velocity calculation, threshold check                  | toast, carousel, drawer         | web only; normalize pointer events to swipe events          |
| platform effect guard      | required    | ensures timer/listener setup only runs client-side                       | all overlay components          | wraps `Event::Init` and listener attachment in `use_effect` |
| platform capability check  | recommended | detect available APIs per target (pointer events, rAF, visibilitychange) | toast, dismissable, drop-zone   | enables graceful degradation on desktop/mobile              |

## 23. Framework-Specific Behavior

Dioxus 0.7.x specifics:

- `use_context_provider(|| QueueContext { ... })` for the imperative handle; `try_use_context::<QueueContext>()` to retrieve.
- `use_signal(Vec::new)` for the reactive toast list: `let mut toasts: Signal<Vec<Entry>> = use_signal(Vec::new);` Signal is `Copy`.
- `use_effect` to guard `Event::Init` dispatch (runs only on client after mount).
- `use_drop` for provider/region listener teardown and per-toast Service cleanup.
- Dioxus does not have `NodeRef` in the same way as Leptos; element IDs or `onmounted` callbacks provide node handles on web targets.
- `spawn` (not `spawn_local`) for promise toast future execution; Dioxus tasks are scoped to the component.
- Toast list mutations use `toasts.write().push(entry)` / `toasts.write().retain(...)`.
- Progress bar uses `request_animation_frame` from web-sys (web) or timer-based fallback (desktop/mobile).
- Swipe velocity computed from timestamp deltas between `pointermove` events (web only).
- Event handlers use Dioxus event types: `Event<PointerData>`, `Event<MouseData>`, `Event<FocusData>`, `Event<KeyboardData>`.
- Dioxus `Callback` uses `.call(value)` rather than `.run(value)`.

## 24. Canonical Implementation Sketch

```rust,no_check
use dioxus::prelude::*;

#[component]
pub fn Provider(props: ProviderProps) -> Element {
    let placement = props.placement.unwrap_or_default();
    let max = props.max_visible.unwrap_or(5);
    let messages = props.messages.unwrap_or_default();
    let locale = resolve_locale(None);

    let mut toasts: Signal<Vec<Entry>> = use_signal(Vec::new);
    let mut queue: Signal<Vec<toast::provider::Config>> = use_signal(Vec::new);

    let toaster = Toaster::new(toasts, queue, max);
    use_context_provider(|| QueueContext {
        toasts,
        toaster: toaster.clone(),
        placement,
        max_visible: max,
        messages: messages.clone(),
        locale: locale.clone(),
    });

    rsx! {
        {props.children}
        Region {}
    }
}

#[component]
fn Region() -> Element {
    let ctx = try_use_context::<QueueContext>()
        .expect("toast::Region must be inside a Provider");

    let polite_attrs = toast::region_attrs(&ctx.messages, &ctx.locale, false);
    let assertive_attrs = toast::region_attrs(&ctx.messages, &ctx.locale, true);

    // Client-only: region-level pause-on-hover
    use_effect(move || {
        // attach pointerenter/pointerleave on region elements
        // -> PauseAll / ResumeAll for all visible toasts
    });

    let polite_toasts = ctx.toasts.read().iter()
        .filter(|t| matches!(t.kind, toast::Kind::Info | toast::Kind::Success | toast::Kind::Loading))
        .cloned().collect::<Vec<_>>();
    let assertive_toasts = ctx.toasts.read().iter()
        .filter(|t| matches!(t.kind, toast::Kind::Error | toast::Kind::Warning))
        .cloned().collect::<Vec<_>>();

    rsx! {
        div { ..polite_attrs,
            for entry in polite_toasts {
                Root { key: "{entry.id}", toast_id: entry.id.clone() }
            }
        }
        div { ..assertive_attrs,
            for entry in assertive_toasts {
                Root { key: "{entry.id}", toast_id: entry.id.clone() }
            }
        }
    }
}

#[component]
fn Root(props: RootProps) -> Element {
    let ctx = try_use_context::<QueueContext>()
        .expect("toast::Root requires QueueContext");
    let toast_id = props.toast_id;

    // Create per-toast Service
    let props = /* build toast::Props from entry config */;
    let service = Service::<toast::Machine>::new(props);

    // Client-only: send Init after mount to start auto-dismiss timer
    use_effect(move || {
        service.send(toast::Event::Init);
    });

    let root_attrs = service.derive(|api| api.root_attrs());

    // Provide per-toast context for child parts
    use_context_provider(|| Context {
        service: service.clone(),
        toast_id: toast_id.clone(),
    });

    // Compose with Presence for enter/exit animation
    // present = service.derive(|api| api.is_visible() || matches!(state, Dismissing))
    // on_exit_complete -> remove toast from visible list

    rsx! {
        div {
            ..root_attrs,
            onpointerenter: move |_| service.send(toast::Event::Pause),
            onpointerleave: move |_| service.send(toast::Event::Resume),
            onfocusin: move |_| service.send(toast::Event::Pause),
            onfocusout: move |ev| {
                // Only resume if focus left the toast entirely
                service.send(toast::Event::Resume);
            },
            // Consumer-provided render callback fills in Title, Description, etc.
        }
    }
}

pub fn use_toast() -> Toaster {
    try_use_context::<QueueContext>()
        .expect("use_toast() must be called within a Provider")
        .toaster
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
// Provider
let mut toasts = use_signal(Vec::new);
let mut queue = use_signal(Vec::new);
let toaster = Toaster::new(toasts, queue, max_visible);
use_context_provider(|| QueueContext { toasts, toaster, placement, messages, locale });

// Region
render_dual_aria_live_regions(region_attrs);
attach_region_pause_on_hover_listeners_if_web();
attach_visibility_change_listener_if_web_and_enabled();
attach_hotkey_listener_if_configured();

// Per-toast lifecycle
for_each_toast(|entry| {
    let service = Service::<toast::Machine>::new(entry.to_props());

    use_effect(move || service.send(toast::Event::Init));  // start timer client-only

    derive_and_spread_root_attrs(service);
    attach_per_toast_pointer_listeners_if_web(service);    // hover, swipe
    attach_per_toast_focus_listeners_if_web(service);      // focusin/focusout

    compose_with_presence(service.derive(|api| api.is_visible()), |on_exit| {
        remove_toast_from_list(entry.id);
        dequeue_next_toast_if_queued();
    });

    use_context_provider(|| Context { service, toast_id: entry.id });
    render_toast_root_and_consumer_content();

    use_drop(move || {
        release_z_index(entry.id);
        remove_swipe_document_listeners();
    });
});

// Announcement coordination
let announcement_queue = VecDeque::new();
let announcement_timer = create_repeating_timer(500, || {
    drain_one_announcement(announcement_queue);
});
use_drop(|| cancel_announcement_timer());
```

## 26. Adapter Invariants

- Both `aria-live` region containers must be present in server-rendered HTML (web SSR) and must persist for the application lifetime.
- `Event::Init` must be sent inside a client-only `use_effect`, never during SSR.
- Each toast must have exactly one `Service<toast::Machine>` instance; the adapter must not share machines across toasts.
- Timer effects managed by the machine (auto-dismiss, exit animation fallback) are cleaned up automatically when the Service is dropped; the adapter must not cancel them independently.
- Swipe document-level listeners (`pointermove`, `pointerup`) must attach on `pointerdown` and detach on `pointerup` or toast cleanup, whichever comes first (web only).
- The `focusout` handler must verify the related target is outside the toast root before sending `Event::Resume` to prevent false resumes on internal focus moves (web only).
- Toast removal from the DOM must be deferred by `remove_delay` after `Dismissed` state to allow CSS exit transitions.
- Announcement coordination must respect the 500ms inter-announcement gap; toasts are never batched.
- Promise toast futures must be spawned via `spawn`; if the toast is dismissed before the future completes, the result is silently discarded.
- The adapter must route toasts to the correct `aria-live` region based on `Kind`: info/success/loading to polite, error/warning to assertive.
- Deduplication must check visible and queued toasts for matching `kind` + `title` + `description` before creating a new toast.
- Never hold `.read()` or `.write()` guards across `.await` boundaries.
- Desktop and Mobile targets must function correctly without swipe-to-dismiss or `visibilitychange`; auto-dismiss timer and close trigger remain the primary dismiss mechanisms.

## 27. Accessibility and SSR Notes

- Dual `aria-live` regions in SSR HTML (web) ensure screen readers track mutations from page load.
- The polite region uses `role="status"` and `aria-live="polite"`; the assertive region uses `role="alert"` and `aria-live="assertive"`.
- Both regions carry `aria-label` from `messages.region_label` for landmark identification.
- `aria-atomic="false"` on regions ensures individual toast insertions are announced, not the entire region content.
- `ActionTrigger` carries `aria-label` from the consumer-provided `alt_text` to give screen readers full action context beyond the button label.
- `CloseTrigger` carries `aria-label` from `messages.dismiss_label`.
- Auto-dismiss pauses on hover and focus-within (web) to satisfy WCAG 2.2.1 (Timing Adjustable).
- Minimum toast display duration of 5000ms is enforced by default; durations below this threshold trigger a debug warning.
- `ProgressBar` uses `role="progressbar"` with `aria-valuenow` (0-100), `aria-valuemin="0"`, `aria-valuemax="100"` but does not announce progress changes via `aria-live` (visual enhancement only).
- Desktop and Mobile targets: screen reader support and ARIA semantics depend on the platform's accessibility stack; the adapter renders the same structural attributes regardless of target.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, event, and manager behavior parity. All adapter-level features (dual regions, imperative `Toaster` API, promise toasts, swipe-to-dismiss, progress bar, pause-on-hover, pause-on-page-idle, hotkey, queuing, deduplication, announcement coordination) are mapped.

Intentional deviations:

- Swipe-to-dismiss is web-only; Desktop and Mobile targets dismiss via close trigger or auto-dismiss timer only.
- `visibilitychange` (pause-on-page-idle) is web-only; Desktop and Mobile do not have equivalent page visibility APIs.
- Region-level pause-on-hover relies on pointer events available on web and desktop webview but not consistently on mobile.
- `focusin`/`focusout` pause behavior is web-only; desktop/mobile focus models may not support equivalent events.

Traceability note: this adapter spec makes explicit the SSR region obligation, per-toast machine lifecycle, mount-guarded `Event::Init`, swipe listener attachment/detachment lifecycle (web), announcement queue timing, promise future spawning via `spawn`, and platform degradation on desktop/mobile.

## 29. Test Scenarios

- dual aria-live regions present in SSR HTML with correct attributes (web)
- toast creation via `use_toast()` adds toast to the correct region by kind
- auto-dismiss timer fires after configured duration
- pause-on-hover pauses and resumes timer on pointerenter/pointerleave (web)
- pause-on-focus-within pauses timer on focusin, resumes on focusout leaving root (web)
- region-level hover pauses all visible toasts (web)
- swipe-to-dismiss: swipe beyond threshold dismisses; below threshold snaps back (web)
- close trigger click dispatches `Event::Dismiss` (all platforms)
- action trigger click fires consumer callback without auto-dismissing (all platforms)
- progress bar updates `--ars-toast-progress` from 0 to 1 over duration
- promise toast shows loading, transitions to success or error on resolve
- toast exceeding `max_visible` is queued; shown when a visible toast dismisses
- deduplication resets timer instead of creating duplicate
- announcement coordination: rapid toasts are spaced 500ms apart (web)
- hotkey focuses the toast region (web, desktop)
- pause-on-page-idle pauses all timers on `visibilitychange` hidden (web)
- `Toaster.dismiss_all()` dismisses all visible toasts
- exit animation via Presence completes before DOM removal
- cleanup via `use_drop`: all listeners, timers, and z-index allocations released on provider drop
- desktop target: toasts work without swipe or visibilitychange

## 30. Test Oracle Notes

| Behavior                            | Preferred oracle type | Notes                                                                                 |
| ----------------------------------- | --------------------- | ------------------------------------------------------------------------------------- |
| dual region presence and attrs      | rendered structure    | assert both `aria-live` regions with correct role/label in SSR output (web)           |
| toast routing by kind               | rendered structure    | assert info/success/loading in polite region, error/warning in assertive              |
| per-toast machine state transitions | machine state         | assert Visible -> Paused -> Visible and Visible -> Dismissing -> Dismissed            |
| auto-dismiss timer                  | callback order        | assert `DurationExpired` fires after configured ms                                    |
| pause/resume timer                  | machine state         | assert remaining_ms decreases, pauses on hover, resumes on leave                      |
| swipe gesture (web)                 | DOM attrs             | assert `data-ars-swiping` and swipe offset transform during swipe                     |
| announcement timing (web)           | callback order        | assert 500ms gap between consecutive announcements                                    |
| context registration                | context registration  | assert `QueueContext` available to descendants after provider mount                   |
| cleanup side effects                | cleanup side effects  | assert all document listeners, timers, and z-index allocations removed via `use_drop` |
| hydration stability (web SSR)       | hydration structure   | assert server and client HTML match for the two empty region containers               |

Cheap verification recipe:

1. Mount `Provider` and assert both `aria-live` region divs are present with correct attrs.
2. Call `use_toast().create(...)` with an info toast and assert it appears in the polite region with correct `data-ars-state="visible"`.
3. Simulate `pointerenter` on the toast root (web), assert state transitions to Paused.
4. Simulate `pointerleave`, assert state returns to Visible.
5. Wait for duration expiry, assert state transitions through Dismissing to Dismissed.
6. Verify the toast is removed from the DOM after `remove_delay`.
7. Create an error toast, assert it appears in the assertive region.
8. Unmount the provider, assert no dangling listeners or timers.
9. On desktop target, verify toasts create and dismiss without swipe support.

## 31. Implementation Checklist

- [ ] `Provider` provides `QueueContext` with `Toaster` handle via `use_context_provider`.
- [ ] Both `aria-live` regions render in SSR HTML (web) with correct `aria-live`, `role`, and `aria-label`.
- [ ] `use_toast()` returns a stable `Toaster` handle; panics outside provider.
- [ ] Per-toast `Service<toast::Machine>` created on add, dropped on removal.
- [ ] `Event::Init` sent inside `use_effect` (not during SSR).
- [ ] Per-toast hover/focus listeners wire `Event::Pause` / `Event::Resume` (web).
- [ ] `focusout` handler checks related target before resuming (web).
- [ ] Region-level hover pauses/resumes all visible toasts (web).
- [ ] Swipe-to-dismiss attaches document listeners on `pointerdown`, removes on `pointerup`/cleanup (web).
- [ ] `CloseTrigger` click dispatches `Event::Dismiss` (all platforms).
- [ ] `ActionTrigger` fires consumer callback; does not auto-dismiss.
- [ ] `ProgressBar` drives `--ars-toast-progress` via `requestAnimationFrame` (web) or timer fallback (desktop/mobile).
- [ ] Presence composition drives enter/exit animation per toast.
- [ ] Toast removal deferred by `remove_delay` after Dismissed state.
- [ ] Toasts routed to correct region by `Kind`.
- [ ] Queuing logic for `max_visible` overflow.
- [ ] Deduplication check on toast creation.
- [ ] Announcement queue drains at 500ms intervals (web).
- [ ] `pause_on_page_idle` via `visibilitychange` listener (web).
- [ ] Hotkey via global `keydown` listener (web, desktop).
- [ ] Promise toast spawned via `spawn`; loading -> success/error on resolve.
- [ ] All listeners, timers, z-index allocations cleaned up via `use_drop`.
- [ ] Desktop and Mobile targets function correctly without swipe or visibilitychange.
