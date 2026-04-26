---
adapter: dioxus
component: presence
category: overlay
source: components/overlay/presence.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Presence — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Presence`](../../components/overlay/presence.md) behavior to Dioxus 0.7.x. The adapter owns the animated root element, DOM event listeners for `animationend`/`transitionend`, the `prefers-reduced-motion` media query listener, the `visibilitychange` handler, fallback timeouts, computed style reads for animation detection, lazy-mount content-readiness dispatch, and conditional child rendering based on `api.is_mounted()`. On Dioxus Desktop and Mobile targets where CSS animations may not be available, the adapter provides documented fallback paths.

## 2. Public Adapter API

```rust
/// Hook that wires the Presence state machine, installs DOM listeners,
/// and returns a handle for querying mount/unmount state.
pub fn use_presence(
    props: presence::Props,
) -> PresenceHandle

#[derive(Clone, Copy)]
pub struct PresenceHandle {
    /// Whether children should be rendered into the DOM.
    pub is_mounted: ReadOnlySignal<bool>,
    /// Whether an exit animation is currently in progress.
    pub is_unmounting: ReadOnlySignal<bool>,
    /// Attributes to spread onto the animated root element.
    pub root_attrs: ReadOnlySignal<AttrMap>,
    /// DOM element ID the consumer must set on the animated root element.
    pub root_id: ReadOnlySignal<String>,
}
```

The public surface matches the full core `Props`: `present`, `lazy_mount`, `skip_animation`, and `reduce_motion`. `PresenceHandle` is `Copy` because Dioxus `Signal<T>` is `Copy`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core props.
- State parity: all four states (`Unmounted`, `Mounting`, `Mounted`, `UnmountPending`) are represented.
- Event parity: `Mount`, `Unmount`, `ContentReady`, and `AnimationEnd` map to the core events.
- API parity: `is_mounted()`, `is_present()`, `is_unmounting()`, `root_attrs()`, and `sync_present()` are all surfaced through the handle.
- Structure parity: the adapter conditionally renders children when `is_mounted` is true, and applies `data-ars-state` and `data-ars-presence` attributes to the root element.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                                  | Ownership                                             | Attr source                 | Notes                                                        |
| --------------------- | --------- | --------------------------------------------------------- | ----------------------------------------------------- | --------------------------- | ------------------------------------------------------------ |
| `Root`                | required  | consumer-provided element with adapter-spread attrs       | adapter-owned attrs on consumer-owned element         | `api.root_attrs()`          | Consumer sets `id` from `root_id` and spreads `root_attrs`.  |
| conditional children  | required  | consumer children rendered only when `is_mounted` is true | consumer-owned content inside adapter-controlled gate | controlled by machine state | Children are removed from DOM/virtual tree when `Unmounted`. |

## 5. Attr Merge and Ownership Rules

| Target node               | Core attrs                                                                                   | Adapter-owned attrs                                      | Consumer attrs                        | Merge order                                           | Ownership notes                               |
| ------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------- | --------------------------------------------- |
| `Root` (animated element) | `api.root_attrs()`: `data-ars-scope`, `data-ars-part`, `data-ars-state`, `data-ars-presence` | `will-change` during animation setup/teardown (web only) | consumer structural and styling attrs | core data attrs win; `class`/`style` merge additively | adapter-owned attrs on consumer-owned element |

- `data-ars-state` is `"open"` only in `Mounted`; `Mounting`, `UnmountPending`, and `Unmounted` expose `"closed"`. CSS animations target the transition into or out of `Mounted`.
- `data-ars-presence` is `"mounted"` when idle, `"exiting"` during exit animation.
- The adapter sets `will-change: transform, opacity` before animation starts and removes it after animation completes (web targets only).

## 6. Composition / Context Contract

Presence is composed by overlay components (Dialog, Popover, Tooltip, etc.) to manage their content panel mount/unmount lifecycle. The composing overlay owns all ARIA attributes; Presence adds only `data-ars-*` attributes.

The adapter does not publish or consume framework context via `use_context_provider`/`try_use_context`. The composing overlay reads `PresenceHandle` fields directly. If the composing component needs `ArsContext` for DOM boundary resolution, it reads that context itself — Presence does not relay it.

## 7. Prop Sync and Event Mapping

| Adapter prop     | Mode         | Sync trigger                                                                             | Machine event / update path                                                   | Visible effect                              | Notes                                   |
| ---------------- | ------------ | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------- | --------------------------------------- |
| `present`        | reactive     | signal change                                                                            | `api.sync_present(new_value)` dispatches `Event::Mount` or `Event::Unmount`   | triggers mount/unmount/animation lifecycle  | primary driver of the state machine     |
| `lazy_mount`     | non-reactive | render time only                                                                         | guards `Unmounted → Mounting` vs `Unmounted → Mounted` path                   | defers entry animation until `ContentReady` | changing after mount has no effect      |
| `skip_animation` | non-reactive | render time only                                                                         | guards `Mounted → Unmounted` (skip) vs `Mounted → UnmountPending` path        | bypasses exit animation                     | changing after mount has no effect      |
| `reduce_motion`  | reactive     | adapter-driven via `matchMedia` change listener (web) or platform query (Desktop/Mobile) | when enabled during `UnmountPending`, fires `Event::AnimationEnd` immediately | instant show/hide without any transitions   | adapter auto-detects from OS preference |

| UI event                        | Preconditions                                          | Machine event / callback path                                                       | Ordering notes                                                                                                                     | Notes                                                         |
| ------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `animationend` on root element  | state is `UnmountPending` and CSS animation is active  | `Event::AnimationEnd`                                                               | fires after CSS animation completes; filtered by target to ignore bubbled child events                                             | web-only listener with `stopPropagation`                      |
| `transitionend` on root element | state is `UnmountPending` and CSS transition is active | `Event::AnimationEnd` (after both animation and transition complete if both active) | longest-duration approach; when computed property is `all`, accept only target-owned events at or after the longest-track deadline | web-only listener                                             |
| `prefers-reduced-motion` change | `matchMedia` change listener active (web)              | fires `Event::AnimationEnd` if machine is in `UnmountPending`                       | immediate, no animation wait                                                                                                       | web: `matchMedia`; Desktop/Mobile: platform accessibility API |
| `visibilitychange`              | fallback timeout active during `UnmountPending`        | pauses/resumes fallback timeout; triggers fresh `getComputedStyle()` on resume      | `performance.now()` for monotonic timing                                                                                           | web-only listener                                             |

## 8. Registration and Cleanup Contract

| Registered entity             | Registration trigger                                               | Identity key             | Cleanup trigger                                          | Cleanup action                                 | Notes                                                                  |
| ----------------------------- | ------------------------------------------------------------------ | ------------------------ | -------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| `animationend` listener       | `UnmountPending` entry, inside rAF after `data-ars-state="closed"` | presence instance + node | `AnimationEnd` event, state change, or component cleanup | remove listener from element                   | web-only; one-shot semantics, with manual target filtering when needed |
| `transitionend` listener      | `UnmountPending` entry, inside rAF                                 | presence instance + node | `AnimationEnd` event, state change, or component cleanup | remove listener from element                   | web-only; filtered by longest-duration property                        |
| transition safety-net timeout | alongside `transitionend` listener                                 | presence instance        | `transitionend` fires first, or component cleanup        | `clear_timeout`                                | `max(duration + delay) + 10ms`                                         |
| 5000ms fallback timeout       | `UnmountPending` entry                                             | presence instance        | animation/transition completes, or component cleanup     | `clear_timeout`                                | ultimate safety net against stuck animations                           |
| `matchMedia` change listener  | effect setup alongside animation listeners                         | presence instance        | component cleanup or effect re-run                       | remove `change` listener from `MediaQueryList` | web-only; fires `AnimationEnd` if reduced motion enabled mid-animation |
| `visibilitychange` listener   | `UnmountPending` entry                                             | presence instance        | `AnimationEnd` event or component cleanup                | `removeEventListener` on `document`            | web-only; pauses/resumes fallback timeout                              |
| mounting timeout (5000ms)     | `Mounting` state entry (lazy mount)                                | presence instance        | `ContentReady` received or component cleanup             | `clear_timeout`                                | forces `ContentReady` if lazy content never settles                    |
| `will-change` style           | animation setup phase                                              | presence instance + node | animation completes or component cleanup                 | remove `will-change` from element style        | web-only; frees GPU memory after animation                             |

- All cleanup is synchronous during the framework's dispose lifecycle (`use_drop`).
- The `completed` guard (`SharedFlag`) prevents any pending listener from firing after cleanup runs.

## 9. Ref and Node Contract

| Target part / node    | Ref required?                      | Ref owner                           | Node availability                     | Composition rule                                                      | Notes                                                                                                                                                       |
| --------------------- | ---------------------------------- | ----------------------------------- | ------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| animated root element | yes (via `root_id` for DOM lookup) | adapter-owned ID, consumer attaches | required after mount                  | composing overlay may also need the element; coordinate via `root_id` | `getComputedStyle()` reads and animation listener attachment require a concrete DOM node (web). On Desktop/Mobile, the ID identifies the virtual tree node. |
| lazy content children | no                                 | consumer-owned                      | client-only when `lazy_mount` is true | adapter gates rendering; consumer provides the children               | adapter dispatches `ContentReady` after lazy content settles.                                                                                               |

## 10. State Machine Boundary Rules

- machine-owned state: `State` (Unmounted/Mounting/Mounted/UnmountPending), `Context` (present, mounted, unmounting, node_id).
- adapter-local derived bookkeeping: animation listener handles, timeout handles, `completed` guard flag, animation/transition done flags, `performance.now()` timestamps for visibility pausing, cached `getComputedStyle()` results.
- forbidden local mirrors: do not keep a local `is_open` or `is_animating` signal that can diverge from the machine's `Context.mounted` and `Context.unmounting`.
- allowed snapshot-read contexts: inside `requestAnimationFrame` callbacks (web), animation event listeners, timeout handlers, and `use_drop` — all reading current machine state via the `send` closure or context signals.

## 11. Callback Payload Contract

| Callback                     | Payload source             | Payload shape                      | Timing                                                                                                      | Cancelable? | Notes                                                                    |
| ---------------------------- | -------------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------ |
| `sync_present` (prop change) | normalized adapter payload | `bool` (new present value)         | on reactive signal change                                                                                   | no          | adapter calls `api.sync_present()` which dispatches `Mount` or `Unmount` |
| animation completion         | machine-derived snapshot   | `Event::AnimationEnd` (no payload) | after CSS animation/transition completes, or immediately if reduced motion / zero duration / non-web target | no          | observational; machine transitions `UnmountPending → Unmounted`          |
| `ContentReady` (lazy mount)  | normalized adapter payload | `Event::ContentReady` (no payload) | after lazy content settles, or after 5000ms timeout                                                         | no          | adapter dispatches after detecting content readiness                     |

## 12. Failure and Degradation Rules

| Condition                                                  | Policy             | Notes                                                                                                                                                    |
| ---------------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| root element not found by ID after mount                   | fail fast          | `getComputedStyle()` and listener attachment require a concrete DOM node. Log error and fire `AnimationEnd` immediately to avoid stuck `UnmountPending`. |
| `getComputedStyle()` returns empty/default values          | degrade gracefully | Treat as zero-duration animation; fire `AnimationEnd` immediately.                                                                                       |
| `animationend`/`transitionend` never fires within 5000ms   | degrade gracefully | Fallback timeout fires `AnimationEnd` to prevent permanent hang.                                                                                         |
| element disconnected from DOM during background tab        | degrade gracefully | Skip `getComputedStyle()` check; let cleanup path handle unmounting.                                                                                     |
| animation APIs absent during SSR                           | no-op              | Render nothing (children not mounted); all listeners are client-only.                                                                                    |
| CSS animation not available (Desktop/Mobile)               | degrade gracefully | Fire `AnimationEnd` immediately on `UnmountPending` entry — no animation wait. Equivalent to `skip_animation = true`.                                    |
| `matchMedia` API absent (Desktop/Mobile or older browsers) | degrade gracefully | Assume `reduce_motion = false` unless OS accessibility API provides reduced-motion state; skip media query listener.                                     |
| `performance.now()` unavailable                            | degrade gracefully | Fall back to `Date.now()` for timeout calculations (web only).                                                                                           |
| Desktop `webview` does not fire `animationend`             | degrade gracefully | Fallback timeout handles completion. Document the webview version requirements for CSS animation support.                                                |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?                 | DOM order must match registration order? | SSR/hydration stability                                              | Notes                                                            |
| -------------------------------- | ---------------- | ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| presence root element            | instance-derived | not applicable                      | not applicable                           | root identity stable across hydration when SSR renders a placeholder | listener and timeout ownership is tied to the presence instance. |
| animation listener registrations | instance-derived | no (one per event type per element) | not applicable                           | client-only                                                          | duplicate listener guard prevents accumulation.                  |

## 14. SSR and Client Boundary Rules

- SSR renders nothing when `present` is false (children not in DOM). When `present` is true on the server, children are rendered with `data-ars-state="open"` and `data-ars-presence="mounted"` — no animation listeners attach.
- All animation-related DOM listeners (`animationend`, `transitionend`, `matchMedia`, `visibilitychange`) are client-only (web target) or not applicable (Desktop/Mobile without CSS animation support).
- The root element ID is generated at hook creation time and remains stable across SSR/hydration.
- `getComputedStyle()` calls happen exclusively on web client inside `requestAnimationFrame` callbacks.
- `Context.node_id` is set by the adapter after the element mounts on the client. SSR does not set it.
- No animation-related events (`AnimationEnd`, timeout-driven `ContentReady`) may fire during SSR.
- Hydration: if the server rendered children (present=true), the client hydrates and attaches listeners. If present changes to false during hydration, the normal `Unmount` path runs.

## 15. Performance Constraints

- **Reflow batching (web):** When multiple Presence instances unmount in the same frame, batch all `getComputedStyle()` reads into a single `requestAnimationFrame` callback. Maintain a per-frame cache map; clear at the start of the next frame.
- **Listener churn:** Animation listeners must not re-register on every render. They register only on `UnmountPending` entry and are removed on completion or cleanup.
- **Single-pass cleanup:** All listeners, timeouts, and guards must be cleaned up in one synchronous pass during `use_drop`.
- **GPU promotion (web):** Set `will-change: transform, opacity` only during the animation phase; remove immediately after to free GPU memory.
- **Timeout precision:** Use `performance.now()` (web) for all duration calculations to avoid system clock drift.
- **Event filtering (web):** Use `event.target` identity check and `stopPropagation()` to avoid processing bubbled child animation events.
- **Desktop/Mobile fast path:** When CSS animations are not available, skip all animation detection and listener setup — fire `AnimationEnd` immediately for zero overhead.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type  | Why it must exist first                                                                                | Notes                                                                                                                                         |
| ------------------------ | ----------- | ---------------- | ------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| animation runtime helper | required    | shared helper    | style reads, listener setup, timeout fallback, reduced-motion observation, and `AnimationEnd` dispatch | Core spec keeps the machine pure and delegates DOM work to the adapter/runtime helper. Desktop/Mobile implementations provide fallback paths. |
| `ars-provider`           | recommended | context contract | Provides DOM environment scoping for composing overlays.                                               | Presence itself does not read it, but composing overlays benefit.                                                                             |

## 17. Recommended Implementation Sequence

1. Create the machine service with `use_machine::<presence::Machine>(props)` and extract the API handle.
2. Generate a stable root element ID via `use_hook`.
3. Set up a reactive effect that watches the `present` prop signal and calls `api.sync_present()`.
4. Set `Context.node_id` after the element mounts on the client (inside a client-only effect that looks up the DOM element by ID).
5. Implement an adapter-owned animation runtime helper: after the machine enters `UnmountPending`,
   set `data-ars-state="closed"`, schedule any required `requestAnimationFrame` work (web), attach
   `animationend` / `transitionend` listeners, install a bounded timeout fallback, and dispatch
   `Event::AnimationEnd`. On Desktop/Mobile, fire `AnimationEnd` immediately if CSS animation
   support is absent.
6. For `lazy_mount`, dispatch `ContentReady` after lazy content settles; if the adapter wants a
   timeout safety net, it belongs in the same runtime helper layer rather than inside the core machine.
7. Set `will-change: transform, opacity` on animation start; remove on animation end (web only).
8. Gate child rendering on `api.is_mounted()` — render children only when mounted.
9. Wire `use_drop` to synchronously cancel all listeners, timeouts, and guards.

## 18. Anti-Patterns

- Do not attach `animationend` or `transitionend` listeners during SSR.
- Do not keep a local `is_animating` signal that can diverge from the machine's `UnmountPending` state.
- Do not read `getComputedStyle()` synchronously outside a `requestAnimationFrame` callback on web — Safari may return stale values.
- Do not rely on a single `requestAnimationFrame` before reading computed styles after DOM insertion; use double-rAF to ensure style calculation completes.
- Do not accumulate multiple `animationend` listeners on repeated visibility toggles; use the duplicate listener guard.
- Do not wait for `animationend`/`transitionend` when `reduce_motion` is active or both durations are zero — fire `AnimationEnd` immediately.
- Do not use `Date.now()` for timeout calculations on web — use `performance.now()` for monotonic precision.
- Do not defer listener cleanup to a microtask or `requestAnimationFrame` — cleanup must be synchronous in `use_drop`.
- Do not leave `will-change` set permanently on web — remove after animation completes to free GPU memory.
- Do not assume CSS animations are available on Desktop or Mobile targets — always provide the immediate-completion fallback path.

## 19. Consumer Expectations and Guarantees

- Consumers may assume children are not in the DOM/virtual tree when `is_mounted` is false.
- Consumers may assume `data-ars-state` transitions from `"open"` to `"closed"` before the exit animation starts, and back to `"open"` on re-mount.
- Consumers may assume exit animations complete within 5000ms or are force-completed by the fallback timeout.
- Consumers may assume `prefers-reduced-motion` is respected automatically unless `reduce_motion` is explicitly overridden.
- Consumers may assume the root element is stable across hydration when present on the server.
- Consumers may assume Presence works on Desktop and Mobile targets even without CSS animations — the element appears/disappears instantly.
- Consumers must not assume `animationend` fires on every unmount — reduced motion, zero-duration animations, and non-web targets skip it.
- Consumers must not assume children remain in the DOM after `present` becomes false — they are removed after the exit animation (or immediately if skipped).
- Consumers must not assume the entry animation has started during the `Mounting` state when `lazy_mount` is true.
- Consumers must not assume CSS animation support exists on Desktop or Mobile targets.

## 20. Platform Support Matrix

| Capability / behavior                        | Web          | Desktop        | Mobile         | SSR            | Notes                                                                                                                    |
| -------------------------------------------- | ------------ | -------------- | -------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------ |
| conditional child rendering                  | full support | full support   | full support   | full support   | SSR renders children when `present` is true; omits when false.                                                           |
| `data-ars-state` / `data-ars-presence` attrs | full support | full support   | full support   | full support   | Attributes render on all targets.                                                                                        |
| `animationend` / `transitionend` listeners   | full support | fallback path  | fallback path  | client-only    | Desktop/Mobile: fire `AnimationEnd` immediately if CSS animation APIs are unavailable in the webview.                    |
| `getComputedStyle()` animation detection     | full support | fallback path  | fallback path  | client-only    | Desktop/Mobile: skip detection and treat as zero-duration if `getComputedStyle` is unavailable.                          |
| `prefers-reduced-motion` media query         | full support | fallback path  | fallback path  | client-only    | Desktop: query OS accessibility settings. Mobile: query platform accessibility API. Fall back to `false` if unavailable. |
| `visibilitychange` handler                   | full support | not applicable | not applicable | not applicable | Desktop/Mobile windows do not have tab visibility semantics.                                                             |
| 5000ms fallback timeout                      | full support | full support   | full support   | not applicable | Active on all client targets as safety net.                                                                              |
| `will-change` GPU promotion                  | full support | not applicable | not applicable | not applicable | Only meaningful for web CSS rendering.                                                                                   |
| lazy mount (`Mounting` state)                | full support | full support   | full support   | SSR-safe empty | Server renders nothing for unmounted lazy content.                                                                       |
| reflow batching (per-frame style cache)      | full support | not applicable | not applicable | not applicable | Web-only optimization.                                                                                                   |
| exit animation playback                      | full support | fallback path  | fallback path  | not applicable | Desktop/Mobile: instant show/hide unless the webview supports CSS animations.                                            |

## 21. Debug Diagnostics and Production Policy

| Condition                                            | Debug build behavior | Production behavior | Notes                                                                                                                          |
| ---------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| root element not found by ID after mount             | fail fast            | fail fast           | Log error: `"Presence: root element not found by ID after mount — cannot detect animations"`. Fire `AnimationEnd` immediately. |
| 5000ms fallback timeout fires                        | debug warning        | degrade gracefully  | Log: `"Presence: fallback timeout fired — animation may be stuck"`.                                                            |
| `animationend` fires on detached element             | debug warning        | no-op               | Log: `"Presence: animationend on detached node — ignoring"`. `completed` guard prevents action.                                |
| `getComputedStyle()` returns zero for both durations | debug warning        | no-op               | Log: `"Presence: no animation or transition detected — unmounting immediately"`.                                               |
| rAF stall detected (>100ms gap)                      | debug warning        | no-op               | Log: `"Presence: animation frame stall detected (>100ms)"`. Web only.                                                          |
| CSS animation APIs unavailable on Desktop/Mobile     | debug warning        | degrade gracefully  | Log: `"Presence: CSS animation APIs unavailable on this target — using immediate completion"`.                                 |
| `matchMedia` API absent                              | debug warning        | degrade gracefully  | Log: `"Presence: matchMedia unavailable — reduced motion detection disabled"`.                                                 |
| duplicate `animationend` listener guard triggered    | debug warning        | no-op               | Log: `"Presence: duplicate animationend listener prevented"`.                                                                  |

## 22. Shared Adapter Helper Notes

| Helper concept             | Required?   | Responsibility                                                                                                                               | Reused by                                           | Notes                                                                                                                                       |
| -------------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| animation runtime helper   | required    | Encapsulates animation/transition detection, listener setup, reduced-motion guard, fallback timeout, and cleanup.                            | `presence` (all overlay components that compose it) | Web implementation follows core spec §11. Desktop/Mobile implementation fires callback immediately when CSS animation APIs are unavailable. |
| reduced-motion observer    | recommended | Registers `matchMedia` change listener (web) or platform accessibility listener (Desktop/Mobile) when live preference changes are supported. | `presence`                                          | Returns cleanup function.                                                                                                                   |
| reflow batching cache      | recommended | Per-frame `getComputedStyle()` cache to avoid layout thrashing when multiple Presence instances unmount simultaneously.                      | `presence` instances in the same frame              | Web only. Cleared at the start of each rAF frame.                                                                                           |
| timeout helper             | required    | Monotonic timeout management.                                                                                                                | `presence` transition safety-net                    | Uses `performance.now()` on web; platform timer on Desktop/Mobile. Handles visibility-change pausing on web.                                |
| platform capability helper | recommended | Detects CSS animation support on the current target.                                                                                         | `presence`, overlay components                      | Returns whether `animationend`/`transitionend` and `getComputedStyle()` are available.                                                      |

## 23. Framework-Specific Behavior

Dioxus uses element IDs (not `NodeRef`) for DOM element lookup. The adapter generates a stable ID via `use_hook` and the consumer sets `id: "{root_id}"` on the animated root element. On the web target, the adapter uses `web_sys::window().unwrap().document().unwrap().get_element_by_id(&root_id)` to obtain the DOM element for `getComputedStyle()` reads and listener attachment.

`use_drop` provides synchronous disposal for all listeners, timeouts, and guards. The adapter registers a single `use_drop` callback that runs the `completed` guard, cancels all timeouts, and removes all event listeners in one pass.

Reactive effects use `use_effect` to watch the `present` prop signal. The effect calls
`api.sync_present(new_value)` to drive the state machine. Animation runtime work is adapter-owned
and reacts to machine state changes; it is not encoded as core-machine effects.

Child rendering is gated by a reactive `is_mounted` signal derived from the machine context. When `is_mounted` transitions from true to false, Dioxus removes the children from the virtual tree. When it transitions from false to true, Dioxus inserts them.

On Dioxus Desktop, the webview (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux) may or may not support CSS animations depending on the webview version. The adapter must probe for CSS animation support on first use and cache the result. If CSS animations are not supported, all animation-related behavior degrades to immediate show/hide.

On Dioxus Mobile, similar webview-based rendering applies. The adapter uses the same probe-and-cache strategy.

## 24. Canonical Implementation Sketch

```rust
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PresenceProps {
    pub present: ReadOnlySignal<bool>,
    #[props(default = false)]
    pub lazy_mount: bool,
    #[props(default = false)]
    pub skip_animation: bool,
    pub children: Element,
}

#[component]
pub fn Presence(props: PresenceProps) -> Element {
    let handle = use_presence(presence::Props {
        id: generate_id("presence"),
        present: *props.present.read(),
        lazy_mount: props.lazy_mount,
        skip_animation: props.skip_animation,
        reduce_motion: false, // auto-detected by adapter
    });

    // Sync present prop reactively.
    let present = props.present;
    use_effect(move || {
        let new_present = *present.read();
        handle.sync_present(new_present);
    });

    if handle.is_mounted.read().clone() {
        rsx! {
            div {
                id: "{handle.root_id}",
                ..handle.root_attrs.read().clone(),
                {props.children}
            }
        }
    } else {
        rsx! {}
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub fn use_presence(props: presence::Props) -> PresenceHandle {
    let service = use_machine::<presence::Machine>(props);
    let api = service.api();
    let root_id = use_hook(|| generate_id("presence-node"));

    // 1. Detect prefers-reduced-motion on mount (client-only, web).
    let reduce_motion = use_hook(|| {
        let platform = use_platform_effects();
        platform.matches_media("(prefers-reduced-motion: reduce)")
    });

    // 2. Set node_id in context after element mounts.
    use_effect(move || {
        service.update_context(|ctx| ctx.node_id = Some(root_id.clone()));
    });

    // 3. Adapter-owned animation runtime helper:
    //      - platform capability probe (CSS animation support)
    //      - double-rAF for style read timing (web)
    //      - getComputedStyle() with per-frame cache (web)
    //      - animationend + transitionend listeners (dual-completion, web)
    //      - reduced-motion guard (immediate AnimationEnd)
    //      - 5000ms fallback timeout (all targets)
    //      - visibilitychange pause/resume (web)
    //      - will-change setup/teardown (web)
    //      - completed guard (SharedFlag)
    //      - immediate completion on Desktop/Mobile if CSS animations unavailable

    // 4. Cleanup: synchronous in use_drop.
    use_drop(move || {
        // Service drop handles all effect cleanup.
        // Listeners, timeouts, and guards are removed synchronously.
    });

    // 5. Derive reactive signals from machine context.
    let is_mounted = use_memo(move || api.is_mounted());
    let is_unmounting = use_memo(move || api.is_unmounting());
    let root_attrs = use_memo(move || api.root_attrs());

    PresenceHandle {
        is_mounted: is_mounted.into(),
        is_unmounting: is_unmounting.into(),
        root_attrs: root_attrs.into(),
        root_id: Signal::new(root_id).into(),
    }
}
```

## 26. Adapter Invariants

- Children must be absent from the DOM/virtual tree when the machine is in `Unmounted` state.
- `data-ars-state` must be `"open"` only in `Mounted`; `Mounting` stays `"closed"` until `ContentReady`. The adapter must set `"closed"` before the rAF that reads `getComputedStyle()` (web).
- `animationend`/`transitionend` listeners must never attach during SSR.
- Only one `animationend` listener and one `transitionend` listener may be active on the root element at any time (web).
- The `completed` guard must be set to `true` synchronously during cleanup, before any listener removal.
- The 5000ms fallback timeout must be present in every state that waits for a DOM event (`UnmountPending`, `Mounting` with lazy mount).
- `getComputedStyle()` must be called inside a double-rAF after setting `data-ars-state="closed"`, never synchronously (web).
- When both CSS animation and transition are active, the adapter must wait for both to complete before sending `AnimationEnd` (web).
- Reduced-motion detection must fire `AnimationEnd` immediately when active, without waiting for any DOM event.
- All DOM cleanup (listener removal, timeout cancellation, guard setting) must be synchronous in a single `use_drop` call.
- `will-change` must be removed after animation completes to avoid permanent GPU memory consumption (web).
- On Desktop and Mobile targets without CSS animation support, `AnimationEnd` must fire immediately on `UnmountPending` entry — the adapter must never hang waiting for a DOM event that will never arrive.

## 27. Accessibility and SSR Notes

- Presence has no ARIA role, `aria-*` attributes, or `tabindex`. The composing overlay owns all accessibility semantics.
- `data-ars-state` and `data-ars-presence` are stable API tokens, not localized strings.
- When `prefers-reduced-motion: reduce` is active (web) or OS-level reduced motion is enabled (Desktop/Mobile), animations are skipped entirely — the element appears/disappears instantly. This is automatic unless the consumer explicitly overrides `reduce_motion` for essential motion.
- SSR renders children with data attributes when present; omits children when not present. No animation behavior occurs on the server.
- Hydration preserves the server-rendered structure. If `present` changes during hydration, the normal state machine path runs on the client.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, state, event, and API parity. All four states (Unmounted, Mounting, Mounted, UnmountPending), all four events (Mount, Unmount, ContentReady, AnimationEnd), conditional rendering, animation detection, reduced-motion support, fallback timeouts, visibility-change pausing, reflow batching, and lazy-mount content-readiness dispatch are fully mapped.

Intentional deviations:

- **Desktop/Mobile animation fallback:** On Dioxus Desktop and Mobile targets where CSS animations may be unavailable in the webview, the adapter fires `AnimationEnd` immediately on `UnmountPending` entry, effectively making all transitions instant. This is documented behavior, not a bug — consumers targeting Desktop/Mobile should not rely on exit animations unless the webview is known to support them.
- **Element ID vs NodeRef:** Dioxus uses a generated element ID and `document.get_element_by_id()` for DOM access rather than a framework-level `NodeRef`, because Dioxus 0.7 does not provide a `NodeRef` equivalent with typed element access for arbitrary HTML elements.

Traceability note: This adapter spec makes explicit the core adapter-owned concerns for DOM animation listener lifecycle, `getComputedStyle()` timing within double-rAF (web), dual animation/transition completion tracking, reduced-motion media query integration, visibility-change timeout pausing (web), fallback timeout management, GPU promotion via `will-change` (web), lazy-mount `ContentReady` dispatch, synchronous cleanup ordering, and multi-platform fallback paths for Desktop and Mobile targets.

## 29. Test Scenarios

- children rendered when `present` is true; removed when `present` is false and animation completes
- `data-ars-state` transitions: `"open"` on mount, `"closed"` on unmount, back to `"open"` on re-mount
- `data-ars-presence` transitions: `"mounted"` when idle, `"exiting"` during exit animation
- exit animation with CSS `animation-name`: `animationend` fires, children removed (web)
- exit animation with CSS `transition-property`: `transitionend` fires, children removed (web)
- dual animation + transition: both must complete before children removed (web)
- no animation detected (zero durations): children removed immediately
- `skip_animation` prop: children removed immediately without entering `UnmountPending`
- `prefers-reduced-motion: reduce` active: children removed immediately
- `prefers-reduced-motion` changes mid-animation: immediate completion
- 5000ms fallback timeout: children removed even if animation events never fire
- `lazy_mount`: children not rendered until `ContentReady`; entry animation deferred
- `lazy_mount` timeout: `ContentReady` forced after 5000ms if content never settles
- visibility-change pausing: fallback timeout paused when tab hidden, resumed when visible (web)
- element disconnected while tab hidden: no `getComputedStyle()` on detached node (web)
- re-mount during exit animation (`UnmountPending → Mounted`): cancel exit, resume showing
- SSR: children rendered with data attrs when present; no listeners attached
- hydration: server-rendered structure preserved; client attaches listeners
- root element not found by ID after mount: error logged, immediate unmount
- Desktop target: immediate show/hide when CSS animations unavailable
- Mobile target: immediate show/hide when CSS animations unavailable
- Desktop target with webview CSS animation support: full animation lifecycle

## 30. Test Oracle Notes

| Behavior                              | Preferred oracle type | Notes                                                                                |
| ------------------------------------- | --------------------- | ------------------------------------------------------------------------------------ |
| conditional child rendering           | rendered structure    | Assert children present/absent in DOM/virtual tree based on `is_mounted`.            |
| `data-ars-state` attribute values     | DOM attrs             | Assert `"open"` or `"closed"` on the root element.                                   |
| `data-ars-presence` attribute values  | DOM attrs             | Assert `"mounted"` or `"exiting"` on the root element.                               |
| animation listener setup (web)        | DOM attrs             | Verify listener attached after `UnmountPending` entry inside rAF.                    |
| animation completion → unmount        | machine state         | Assert machine transitions to `Unmounted` after `animationend`.                      |
| fallback timeout → unmount            | machine state         | Assert machine transitions to `Unmounted` after 5000ms.                              |
| reduced-motion immediate completion   | machine state         | Assert `UnmountPending → Unmounted` without waiting for DOM event.                   |
| listener cleanup on component unmount | cleanup side effects  | Assert `animationend`/`transitionend` listeners removed, timeouts cancelled.         |
| `will-change` lifecycle (web)         | DOM attrs             | Assert `will-change` set during animation, removed after.                            |
| lazy mount `ContentReady` dispatch    | machine state         | Assert `Mounting → Mounted` after content settles.                                   |
| SSR rendered structure                | hydration structure   | Assert children and data attrs present in server HTML.                               |
| Desktop/Mobile immediate completion   | machine state         | Assert `UnmountPending → Unmounted` immediately when CSS animation APIs unavailable. |

Cheap verification recipe:

1. Render Presence with `present=true` and assert children are in the DOM with `data-ars-state="open"`.
2. Set `present=false` and assert `data-ars-state="closed"` and `data-ars-presence="exiting"`.
3. Fire a synthetic `animationend` on the root element (web) and assert children are removed from the DOM.
4. Set `present=true` again and assert children re-appear with `data-ars-state="open"`.
5. Repeat step 2 but with `prefers-reduced-motion: reduce` active; assert children are removed immediately without waiting for `animationend`.
6. On Desktop target, verify `present=false` removes children immediately without animation wait.
7. Unmount the component and assert all listeners and timeouts are cleaned up.

## 31. Implementation Checklist

- [ ] `use_presence` hook creates machine service, stable root ID, and reactive signals.
- [ ] `present` prop changes drive `api.sync_present()` via reactive effect.
- [ ] `Context.node_id` is set after client mount from the generated root element ID.
- [ ] Children are conditionally rendered based on `api.is_mounted()`.
- [ ] `data-ars-state` is `"open"` only in `Mounted`; `Mounting`, `UnmountPending`, and `Unmounted` are `"closed"`.
- [ ] `data-ars-presence` is `"mounted"` when idle, `"exiting"` during exit animation.
- [ ] `animationend` and `transitionend` listeners attach only on `UnmountPending` entry, inside a double-rAF (web).
- [ ] `getComputedStyle()` reads are batched into a per-frame cache (web).
- [ ] Dual animation/transition completion: both must finish before `AnimationEnd` fires (web).
- [ ] Reduced-motion guard fires `AnimationEnd` immediately when active.
- [ ] `matchMedia` change listener fires `AnimationEnd` if reduced motion enabled mid-animation (web).
- [ ] 5000ms fallback timeout installed for `UnmountPending` and `Mounting` states.
- [ ] `visibilitychange` handler pauses/resumes the fallback timeout (web).
- [ ] `will-change: transform, opacity` set during animation, removed after (web).
- [ ] Lazy mount: `ContentReady` dispatched after content settles; 5000ms timeout forces it.
- [ ] `completed` guard prevents pending listeners from firing after cleanup.
- [ ] All cleanup is synchronous in a single `use_drop` call.
- [ ] SSR renders children with data attrs when present; omits when not present; no listeners.
- [ ] Hydration preserves server-rendered structure.
- [ ] No animation-related events fire during SSR.
- [ ] Desktop/Mobile: `AnimationEnd` fires immediately when CSS animation APIs unavailable.
- [ ] Desktop/Mobile: platform accessibility API queried for reduced-motion preference when `matchMedia` is unavailable.
- [ ] Platform capability probe result is cached per session, not re-queried per instance.
