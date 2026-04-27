---
adapter: leptos
component: presence
category: overlay
source: components/overlay/presence.md
source_foundation: foundation/08-adapter-leptos.md
---

# Presence — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Presence`](../../components/overlay/presence.md) behavior to Leptos 0.8.x. The adapter owns the animated root element, DOM event listeners for `animationend`/`transitionend`, the `prefers-reduced-motion` media query listener, the `visibilitychange` handler, fallback timeouts, computed style reads for animation detection, lazy-mount content-readiness dispatch, and conditional child rendering based on `api.is_mounted()`.

## 2. Public Adapter API

```rust
/// Hook that wires the Presence state machine, installs DOM listeners,
/// and returns a handle for querying mount/unmount state.
pub fn use_presence(
    props: presence::Props,
) -> PresenceHandle

pub struct PresenceHandle {
    /// Whether children should be rendered into the DOM.
    pub is_mounted: Signal<bool>,
    /// Whether an exit animation is currently in progress.
    pub is_unmounting: Signal<bool>,
    /// Attributes to spread onto the animated root element.
    pub root_attrs: Signal<AttrMap>,
    /// Node ref the consumer must attach to the animated root element.
    pub root_ref: NodeRef<html::Div>,
}
```

The public surface matches the full core `Props`: `present`, `lazy_mount`, `skip_animation`, and `reduce_motion`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core props.
- State parity: all four states (`Unmounted`, `Mounting`, `Mounted`, `UnmountPending`) are represented.
- Event parity: `Mount`, `Unmount`, `ContentReady`, and `AnimationEnd` map to the core events.
- API parity: `is_mounted()`, `is_present()`, `is_unmounting()`, `root_attrs()`, and `sync_present()` are all surfaced through the handle.
- Structure parity: the adapter conditionally renders children when `is_mounted` is true, and applies `data-ars-state` and `data-ars-presence` attributes to the root element.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                                  | Ownership                                             | Attr source                 | Notes                                                  |
| --------------------- | --------- | --------------------------------------------------------- | ----------------------------------------------------- | --------------------------- | ------------------------------------------------------ |
| `Root`                | required  | consumer-provided element with adapter-spread attrs       | adapter-owned attrs on consumer-owned element         | `api.root_attrs()`          | Consumer attaches `root_ref` and spreads `root_attrs`. |
| conditional children  | required  | consumer children rendered only when `is_mounted` is true | consumer-owned content inside adapter-controlled gate | controlled by machine state | Children are removed from DOM when `Unmounted`.        |

## 5. Attr Merge and Ownership Rules

| Target node               | Core attrs                                                                                   | Adapter-owned attrs                           | Consumer attrs                        | Merge order                                           | Ownership notes                               |
| ------------------------- | -------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------- | ----------------------------------------------------- | --------------------------------------------- |
| `Root` (animated element) | `api.root_attrs()`: `data-ars-scope`, `data-ars-part`, `data-ars-state`, `data-ars-presence` | `will-change` during animation setup/teardown | consumer structural and styling attrs | core data attrs win; `class`/`style` merge additively | adapter-owned attrs on consumer-owned element |

- `data-ars-state` is `"open"` only in `Mounted`; `Mounting`, `UnmountPending`, and `Unmounted` expose `"closed"`. CSS animations target the transition into or out of `Mounted`.
- `data-ars-presence` is `"mounted"` when idle, `"exiting"` during exit animation.
- The adapter sets `will-change: transform, opacity` before animation starts and removes it after animation completes.

## 6. Composition / Context Contract

Presence is composed by overlay components (Dialog, Popover, Tooltip, etc.) to manage their content panel mount/unmount lifecycle. The composing overlay owns all ARIA attributes; Presence adds only `data-ars-*` attributes.

The adapter does not publish or consume framework context. The composing overlay reads `PresenceHandle` fields directly. If the composing component needs `ArsContext` for DOM boundary resolution, it reads that context itself — Presence does not relay it.

## 7. Prop Sync and Event Mapping

| Adapter prop     | Mode         | Sync trigger                                    | Machine event / update path                                                   | Visible effect                              | Notes                                   |
| ---------------- | ------------ | ----------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------- | --------------------------------------- |
| `present`        | reactive     | signal change                                   | `api.sync_present(new_value)` dispatches `Event::Mount` or `Event::Unmount`   | triggers mount/unmount/animation lifecycle  | primary driver of the state machine     |
| `lazy_mount`     | non-reactive | render time only                                | guards `Unmounted → Mounting` vs `Unmounted → Mounted` path                   | defers entry animation until `ContentReady` | changing after mount has no effect      |
| `skip_animation` | non-reactive | render time only                                | guards `Mounted → Unmounted` (skip) vs `Mounted → UnmountPending` path        | bypasses exit animation                     | changing after mount has no effect      |
| `reduce_motion`  | reactive     | adapter-driven via `matchMedia` change listener | when enabled during `UnmountPending`, fires `Event::AnimationEnd` immediately | instant show/hide without any transitions   | adapter auto-detects from OS preference |

| UI event                        | Preconditions                                          | Machine event / callback path                                                       | Ordering notes                                                                                                                     | Notes                                       |
| ------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `animationend` on root element  | state is `UnmountPending` and CSS animation is active  | `Event::AnimationEnd`                                                               | fires after CSS animation completes; filtered by target to ignore bubbled child events                                             | client-only listener with `stopPropagation` |
| `transitionend` on root element | state is `UnmountPending` and CSS transition is active | `Event::AnimationEnd` (after both animation and transition complete if both active) | longest-duration approach; when computed property is `all`, accept only target-owned events at or after the longest-track deadline | client-only listener                        |
| `prefers-reduced-motion` change | `matchMedia` change listener active                    | fires `Event::AnimationEnd` if machine is in `UnmountPending`                       | immediate, no animation wait                                                                                                       | registered alongside animation listeners    |
| `visibilitychange`              | fallback timeout active during `UnmountPending`        | pauses/resumes fallback timeout; triggers fresh `getComputedStyle()` on resume      | `performance.now()` for monotonic timing                                                                                           | client-only listener                        |

## 8. Registration and Cleanup Contract

| Registered entity             | Registration trigger                                               | Identity key             | Cleanup trigger                                          | Cleanup action                                 | Notes                                                                     |
| ----------------------------- | ------------------------------------------------------------------ | ------------------------ | -------------------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------- |
| `animationend` listener       | `UnmountPending` entry, inside rAF after `data-ars-state="closed"` | presence instance + node | `AnimationEnd` event, state change, or component cleanup | remove listener from element                   | client-only; one-shot semantics, with manual target filtering when needed |
| `transitionend` listener      | `UnmountPending` entry, inside rAF                                 | presence instance + node | `AnimationEnd` event, state change, or component cleanup | remove listener from element                   | client-only; filtered by longest-duration property                        |
| transition safety-net timeout | alongside `transitionend` listener                                 | presence instance        | `transitionend` fires first, or component cleanup        | `clear_timeout`                                | `max(duration + delay) + 10ms`                                            |
| 5000ms fallback timeout       | `UnmountPending` entry                                             | presence instance        | animation/transition completes, or component cleanup     | `clear_timeout`                                | ultimate safety net against stuck animations                              |
| `matchMedia` change listener  | effect setup alongside animation listeners                         | presence instance        | component cleanup or effect re-run                       | remove `change` listener from `MediaQueryList` | fires `AnimationEnd` if reduced motion enabled mid-animation              |
| `visibilitychange` listener   | `UnmountPending` entry                                             | presence instance        | `AnimationEnd` event or component cleanup                | `removeEventListener` on `document`            | pauses/resumes fallback timeout                                           |
| mounting timeout (5000ms)     | `Mounting` state entry (lazy mount)                                | presence instance        | `ContentReady` received or component cleanup             | `clear_timeout`                                | forces `ContentReady` if lazy content never settles                       |
| `will-change` style           | animation setup phase                                              | presence instance + node | animation completes or component cleanup                 | remove `will-change` from element style        | frees GPU memory after animation                                          |

- All cleanup is synchronous during the framework's dispose lifecycle (`on_cleanup`).
- The `completed` guard (`SharedFlag`) prevents any pending listener from firing after cleanup runs.

## 9. Ref and Node Contract

| Target part / node    | Ref required? | Ref owner                        | Node availability                     | Composition rule                                                                             | Notes                                                                                     |
| --------------------- | ------------- | -------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| animated root element | yes           | adapter-owned, consumer attaches | required after mount                  | composing overlay may also need the ref; use a shared `NodeRef` or coordinate via the handle | `getComputedStyle()` reads and animation listener attachment require a concrete DOM node. |
| lazy content children | no            | consumer-owned                   | client-only when `lazy_mount` is true | adapter gates rendering; consumer provides the children                                      | adapter dispatches `ContentReady` after lazy content settles.                             |

## 10. State Machine Boundary Rules

- machine-owned state: `State` (Unmounted/Mounting/Mounted/UnmountPending), `Context` (present, mounted, unmounting, node_id).
- adapter-local derived bookkeeping: animation listener handles, timeout handles, `completed` guard flag, `SharedFlag` animation/transition done flags, `performance.now()` timestamps for visibility pausing, cached `getComputedStyle()` results.
- forbidden local mirrors: do not keep a local `is_open` or `is_animating` boolean that can diverge from the machine's `Context.mounted` and `Context.unmounting`.
- allowed snapshot-read contexts: inside `requestAnimationFrame` callbacks, animation event listeners, timeout handlers, and `on_cleanup` — all reading current machine state via the `send` closure or context signals.

## 11. Callback Payload Contract

| Callback                     | Payload source             | Payload shape                      | Timing                                                                                     | Cancelable? | Notes                                                                    |
| ---------------------------- | -------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------ | ----------- | ------------------------------------------------------------------------ |
| `sync_present` (prop change) | normalized adapter payload | `bool` (new present value)         | on reactive signal change                                                                  | no          | adapter calls `api.sync_present()` which dispatches `Mount` or `Unmount` |
| animation completion         | machine-derived snapshot   | `Event::AnimationEnd` (no payload) | after CSS animation/transition completes, or immediately if reduced motion / zero duration | no          | observational; machine transitions `UnmountPending → Unmounted`          |
| `ContentReady` (lazy mount)  | normalized adapter payload | `Event::ContentReady` (no payload) | after lazy content settles, or after 5000ms timeout                                        | no          | adapter dispatches after detecting content readiness                     |

## 12. Failure and Degradation Rules

| Condition                                                | Policy             | Notes                                                                                                                                                    |
| -------------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| root node ref missing after mount                        | fail fast          | `getComputedStyle()` and listener attachment require a concrete DOM node. Log error and fire `AnimationEnd` immediately to avoid stuck `UnmountPending`. |
| `getComputedStyle()` returns empty/default values        | degrade gracefully | Treat as zero-duration animation; fire `AnimationEnd` immediately.                                                                                       |
| `animationend`/`transitionend` never fires within 5000ms | degrade gracefully | Fallback timeout fires `AnimationEnd` to prevent permanent hang.                                                                                         |
| element disconnected from DOM during background tab      | degrade gracefully | Skip `getComputedStyle()` check; let cleanup path handle unmounting.                                                                                     |
| browser animation APIs absent during SSR                 | no-op              | Render nothing (children not mounted); all listeners are client-only.                                                                                    |
| `matchMedia` API absent (older browsers)                 | degrade gracefully | Assume `reduce_motion = false`; skip media query listener.                                                                                               |
| `performance.now()` unavailable                          | degrade gracefully | Fall back to `Date.now()` for timeout calculations.                                                                                                      |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?                 | DOM order must match registration order? | SSR/hydration stability                                              | Notes                                                            |
| -------------------------------- | ---------------- | ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| presence root element            | instance-derived | not applicable                      | not applicable                           | root identity stable across hydration when SSR renders a placeholder | listener and timeout ownership is tied to the presence instance. |
| animation listener registrations | instance-derived | no (one per event type per element) | not applicable                           | client-only                                                          | duplicate listener guard prevents accumulation.                  |

## 14. SSR and Client Boundary Rules

- SSR renders nothing when `present` is false (children not in DOM). When `present` is true on the server, children are rendered with `data-ars-state="open"` and `data-ars-presence="mounted"` — no animation listeners attach.
- All animation-related DOM listeners (`animationend`, `transitionend`, `matchMedia`, `visibilitychange`) are client-only.
- The root `NodeRef` is server-safe absent and becomes available after client mount.
- `getComputedStyle()` calls happen exclusively on the client inside `requestAnimationFrame` callbacks.
- `Context.node_id` is set by the adapter after the element mounts on the client. SSR does not set it.
- No animation-related events (`AnimationEnd`, timeout-driven `ContentReady`) may fire during SSR.
- Hydration: if the server rendered children (present=true), the client hydrates and attaches listeners. If present changes to false during hydration, the normal `Unmount` path runs.

## 15. Performance Constraints

- **Reflow batching:** When multiple Presence instances unmount in the same frame, batch all `getComputedStyle()` reads into a single `requestAnimationFrame` callback. Maintain a per-frame `WeakRef<Element> → CachedStyleDetection` map; clear at the start of the next frame.
- **Listener churn:** Animation listeners must not re-register on every render. They register only on `UnmountPending` entry and are removed on completion or cleanup.
- **Single-pass cleanup:** All listeners, timeouts, and guards must be cleaned up in one synchronous pass during `on_cleanup`.
- **GPU promotion:** Set `will-change: transform, opacity` only during the animation phase; remove immediately after to free GPU memory.
- **Timeout precision:** Use `performance.now()` for all duration calculations to avoid system clock drift and get sub-millisecond precision.
- **Event filtering:** Use `event.target` identity check and `stopPropagation()` to avoid processing bubbled child animation events.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type  | Why it must exist first                                                                                | Notes                                                                                  |
| ------------------------ | ----------- | ---------------- | ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| animation runtime helper | required    | shared helper    | style reads, listener setup, timeout fallback, reduced-motion observation, and `AnimationEnd` dispatch | Core spec keeps the machine pure and delegates DOM work to the adapter/runtime helper. |
| `ars-provider`           | recommended | context contract | Provides DOM environment scoping for composing overlays.                                               | Presence itself does not read it, but composing overlays benefit.                      |

## 17. Recommended Implementation Sequence

1. Create the machine service with `use_machine::<presence::Machine>(props)` and extract the API handle.
2. Create `NodeRef<html::Div>` for the animated root element.
3. Set up a reactive effect that watches the `present` prop signal and calls `api.sync_present()`.
4. Set `Context.node_id` after the element mounts on the client (inside a client-only effect that reads the `NodeRef`).
5. Implement an adapter-owned animation runtime helper: after the machine enters `UnmountPending`,
   set `data-ars-state="closed"`, schedule any required `requestAnimationFrame` reads, attach
   `animationend` / `transitionend` listeners, install a bounded timeout fallback, and dispatch
   `Event::AnimationEnd` when the exit finishes.
6. For `lazy_mount`, dispatch `ContentReady` after lazy content settles; if the adapter wants a
   timeout safety net, it belongs in the same runtime helper layer rather than inside the core machine.
7. Set `will-change: transform, opacity` on animation start; remove on animation end.
8. Gate child rendering on `api.is_mounted()` — render children only when mounted.
9. Wire `on_cleanup` to synchronously cancel all listeners, timeouts, and guards.

## 18. Anti-Patterns

- Do not attach `animationend` or `transitionend` listeners during SSR.
- Do not keep a local `is_animating` flag that can diverge from the machine's `UnmountPending` state.
- Do not read `getComputedStyle()` synchronously outside a `requestAnimationFrame` callback — Safari may return stale values.
- Do not rely on a single `requestAnimationFrame` before reading computed styles after DOM insertion; use double-rAF to ensure style calculation completes.
- Do not accumulate multiple `animationend` listeners on repeated visibility toggles; use the duplicate listener guard.
- Do not wait for `animationend`/`transitionend` when `reduce_motion` is active or both durations are zero — fire `AnimationEnd` immediately.
- Do not use `Date.now()` for timeout calculations — use `performance.now()` for monotonic precision.
- Do not defer listener cleanup to a microtask or `requestAnimationFrame` — cleanup must be synchronous in `on_cleanup`.
- Do not leave `will-change` set permanently — remove after animation completes to free GPU memory.

## 19. Consumer Expectations and Guarantees

- Consumers may assume children are not in the DOM when `is_mounted` is false.
- Consumers may assume `data-ars-state` transitions from `"open"` to `"closed"` before the exit animation starts, and back to `"open"` on re-mount.
- Consumers may assume exit animations complete within 5000ms or are force-completed by the fallback timeout.
- Consumers may assume `prefers-reduced-motion` is respected automatically unless `reduce_motion` is explicitly overridden.
- Consumers may assume the root element is stable across hydration when present on the server.
- Consumers must not assume `animationend` fires on every unmount — reduced motion and zero-duration animations skip it.
- Consumers must not assume children remain in the DOM after `present` becomes false — they are removed after the exit animation (or immediately if skipped).
- Consumers must not assume the entry animation has started during the `Mounting` state when `lazy_mount` is true.

## 20. Platform Support Matrix

| Capability / behavior                        | Browser client | SSR            | Notes                                                                                      |
| -------------------------------------------- | -------------- | -------------- | ------------------------------------------------------------------------------------------ |
| conditional child rendering                  | full support   | full support   | SSR renders children when `present` is true; omits when false.                             |
| `data-ars-state` / `data-ars-presence` attrs | full support   | full support   | Attributes render on server when present is true.                                          |
| `animationend` / `transitionend` listeners   | full support   | client-only    | Listeners attach only after client mount inside rAF.                                       |
| `getComputedStyle()` animation detection     | full support   | client-only    | Requires live DOM node inside `requestAnimationFrame`.                                     |
| `prefers-reduced-motion` media query         | full support   | client-only    | Adapter auto-detects on client; SSR assumes `reduce_motion = false` unless prop overrides. |
| `visibilitychange` handler                   | full support   | not applicable | Only relevant in browser tabs.                                                             |
| 5000ms fallback timeout                      | full support   | not applicable | Client-only safety net.                                                                    |
| `will-change` GPU promotion                  | full support   | not applicable | Client-only style manipulation.                                                            |
| lazy mount (`Mounting` state)                | full support   | SSR-safe empty | Server renders nothing for unmounted lazy content; client handles `ContentReady`.          |
| reflow batching (per-frame style cache)      | full support   | not applicable | Client-only optimization for concurrent Presence instances.                                |

## 21. Debug Diagnostics and Production Policy

| Condition                                            | Debug build behavior | Production behavior | Notes                                                                                                                  |
| ---------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| root `NodeRef` is `None` after mount                 | fail fast            | fail fast           | Log error: `"Presence: root NodeRef is None after mount — cannot detect animations"`. Fire `AnimationEnd` immediately. |
| 5000ms fallback timeout fires                        | debug warning        | degrade gracefully  | Log: `"Presence: fallback timeout fired — animation may be stuck"`.                                                    |
| `animationend` fires on detached element             | debug warning        | no-op               | Log: `"Presence: animationend on detached node — ignoring"`. `completed` guard prevents action.                        |
| `getComputedStyle()` returns zero for both durations | debug warning        | no-op               | Log: `"Presence: no animation or transition detected — unmounting immediately"`.                                       |
| rAF stall detected (>100ms gap)                      | debug warning        | no-op               | Log: `"Presence: animation frame stall detected (>100ms)"`.                                                            |
| `matchMedia` API absent                              | debug warning        | degrade gracefully  | Log: `"Presence: matchMedia unavailable — reduced motion detection disabled"`.                                         |
| duplicate `animationend` listener guard triggered    | debug warning        | no-op               | Log: `"Presence: duplicate animationend listener prevented"`.                                                          |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                                                                                          | Reused by                                           | Notes                                     |
| ------------------------ | ----------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------- |
| animation runtime helper | required    | Encapsulates animation/transition detection, listener setup, reduced-motion guard, fallback timeout, and cleanup.       | `presence` (all overlay components that compose it) | Web implementation follows core spec §11. |
| reduced-motion observer  | recommended | Registers `matchMedia` change listener for `prefers-reduced-motion` when the adapter supports live preference changes.  | `presence`                                          | Returns cleanup function.                 |
| reflow batching cache    | recommended | Per-frame `getComputedStyle()` cache to avoid layout thrashing when multiple Presence instances unmount simultaneously. | `presence` instances in the same frame              | Cleared at the start of each rAF frame.   |
| timeout helper           | required    | Monotonic timeout management using `performance.now()`.                                                                 | `presence` transition safety-net                    | Handles visibility-change pausing.        |

## 23. Framework-Specific Behavior

Leptos uses `NodeRef<html::Div>` for the animated root element, which resolves to `Option<HtmlDivElement>` on the client. The adapter reads the DOM element inside a client-only `Effect` and sets `Context.node_id` from the element's `id` attribute (or generates a stable ID if none is set).

`on_cleanup` provides synchronous disposal for all listeners, timeouts, and guards. The adapter registers a single `on_cleanup` callback that runs the `completed` guard, cancels all timeouts, and removes all event listeners in one pass.

Reactive effects use `Effect::new` or `RenderEffect::new` to watch the `present` prop signal. The
effect calls `api.sync_present(new_value)` to drive the state machine. Animation runtime work is
adapter-owned and reacts to machine state changes; it is not encoded as core-machine effects.

Child rendering is gated by a reactive `is_mounted` signal derived from the machine context. When `is_mounted` transitions from true to false, Leptos removes the children from the DOM. When it transitions from false to true, Leptos inserts them.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Presence(
    #[prop(into)] present: Signal<bool>,
    #[prop(optional)] lazy_mount: bool,
    #[prop(optional)] skip_animation: bool,
    children: Children,
) -> impl IntoView {
    let handle = use_presence(
        presence::Props::new()
            .id(generate_id("presence"))
            .present(present.get_untracked())
            .lazy_mount(lazy_mount)
            .skip_animation(skip_animation),
        // reduce_motion: auto-detected by adapter, left at default `false`
    );

    // Sync present prop reactively.
    Effect::new(move |_| {
        let new_present = present.get();
        handle.sync_present(new_present);
    });

    view! {
        <Show when=move || handle.is_mounted.get()>
            <div
                node_ref=handle.root_ref
                {..handle.root_attrs.get()}
            >
                {children()}
            </div>
        </Show>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub fn use_presence(props: presence::Props) -> PresenceHandle {
    let service = use_machine::<presence::Machine>(props);
    let api = service.api();
    let root_ref = NodeRef::<html::Div>::new();

    // 1. Detect prefers-reduced-motion on mount (client-only).
    let reduce_motion = create_client_only_signal(move || {
        let platform = use_platform_effects();
        platform.matches_media("(prefers-reduced-motion: reduce)")
    });

    // 2. Set node_id in context after element mounts.
    Effect::new(move |_| {
        if let Some(el) = root_ref.get() {
            let id = el.id();
            let node_id = if id.is_empty() {
                let generated = generate_id("presence-node");
                el.set_id(&generated);
                generated
            } else {
                id
            };
            service.update_context(|ctx| ctx.node_id = Some(node_id));
        }
    });

    // 3. Adapter-owned animation runtime helper:
    //      - double-rAF for style read timing when needed
    //      - getComputedStyle() with per-frame cache
    //      - animationend + transitionend listeners (dual-completion)
    //      - reduced-motion guard (immediate AnimationEnd)
    //      - 5000ms fallback timeout
    //      - visibilitychange pause/resume
    //      - will-change setup/teardown
    //      - completed guard (SharedFlag)

    // 4. Cleanup: synchronous in on_cleanup.
    on_cleanup(move || {
        // Service drop handles all effect cleanup.
        // Listeners, timeouts, and guards are removed synchronously.
    });

    // 5. Derive reactive signals from machine context.
    let is_mounted = Signal::derive(move || api.is_mounted());
    let is_unmounting = Signal::derive(move || api.is_unmounting());
    let root_attrs = Signal::derive(move || api.root_attrs());

    PresenceHandle {
        is_mounted,
        is_unmounting,
        root_attrs,
        root_ref,
    }
}
```

## 26. Adapter Invariants

- Children must be absent from the DOM when the machine is in `Unmounted` state.
- `data-ars-state` must be `"open"` only in `Mounted`; `Mounting` stays `"closed"` until `ContentReady`. The adapter must set `"closed"` before the rAF that reads `getComputedStyle()`.
- `animationend`/`transitionend` listeners must never attach during SSR.
- Only one `animationend` listener and one `transitionend` listener may be active on the root element at any time.
- The `completed` guard must be set to `true` synchronously during cleanup, before any listener removal.
- The 5000ms fallback timeout must be present in every state that waits for a DOM event (`UnmountPending`, `Mounting` with lazy mount).
- `getComputedStyle()` must be called inside a double-rAF after setting `data-ars-state="closed"`, never synchronously.
- When both CSS animation and transition are active, the adapter must wait for both to complete before sending `AnimationEnd`.
- Reduced-motion detection must fire `AnimationEnd` immediately when active, without waiting for any DOM event.
- All DOM cleanup (listener removal, timeout cancellation, guard setting) must be synchronous in a single `on_cleanup` call.
- `will-change` must be removed after animation completes to avoid permanent GPU memory consumption.

## 27. Accessibility and SSR Notes

- Presence has no ARIA role, `aria-*` attributes, or `tabindex`. The composing overlay owns all accessibility semantics.
- `data-ars-state` and `data-ars-presence` are stable API tokens, not localized strings.
- When `prefers-reduced-motion: reduce` is active, animations are skipped entirely — the element appears/disappears instantly. This is automatic unless the consumer explicitly overrides `reduce_motion` for essential motion.
- SSR renders children with data attributes when present; omits children when not present. No animation behavior occurs on the server.
- Hydration preserves the server-rendered structure. If `present` changes during hydration, the normal state machine path runs on the client.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, state, event, and API parity. All four states (Unmounted, Mounting, Mounted, UnmountPending), all four events (Mount, Unmount, ContentReady, AnimationEnd), conditional rendering, animation detection, reduced-motion support, fallback timeouts, visibility-change pausing, reflow batching, and lazy-mount content-readiness dispatch are fully mapped.

Intentional deviations: none.

Traceability note: This adapter spec makes explicit the core adapter-owned concerns for DOM animation listener lifecycle, `getComputedStyle()` timing within double-rAF, dual animation/transition completion tracking, reduced-motion media query integration, visibility-change timeout pausing, fallback timeout management, GPU promotion via `will-change`, lazy-mount `ContentReady` dispatch, and synchronous cleanup ordering.

## 29. Test Scenarios

- children rendered when `present` is true; removed when `present` is false and animation completes
- `data-ars-state` transitions: `"open"` on mount, `"closed"` on unmount, back to `"open"` on re-mount
- `data-ars-presence` transitions: `"mounted"` when idle, `"exiting"` during exit animation
- exit animation with CSS `animation-name`: `animationend` fires, children removed
- exit animation with CSS `transition-property`: `transitionend` fires, children removed
- dual animation + transition: both must complete before children removed
- no animation detected (zero durations): children removed immediately
- `skip_animation` prop: children removed immediately without entering `UnmountPending`
- `prefers-reduced-motion: reduce` active: children removed immediately
- `prefers-reduced-motion` changes mid-animation: immediate completion
- 5000ms fallback timeout: children removed even if animation events never fire
- `lazy_mount`: children not rendered until `ContentReady`; entry animation deferred
- `lazy_mount` timeout: `ContentReady` forced after 5000ms if content never settles
- visibility-change pausing: fallback timeout paused when tab hidden, resumed when visible
- element disconnected while tab hidden: no `getComputedStyle()` on detached node
- re-mount during exit animation (`UnmountPending → Mounted`): cancel exit, resume showing
- SSR: children rendered with data attrs when present; no listeners attached
- hydration: server-rendered structure preserved; client attaches listeners
- root `NodeRef` missing after mount: error logged, immediate unmount

## 30. Test Oracle Notes

| Behavior                              | Preferred oracle type | Notes                                                                        |
| ------------------------------------- | --------------------- | ---------------------------------------------------------------------------- |
| conditional child rendering           | rendered structure    | Assert children present/absent in DOM based on `is_mounted`.                 |
| `data-ars-state` attribute values     | DOM attrs             | Assert `"open"` or `"closed"` on the root element.                           |
| `data-ars-presence` attribute values  | DOM attrs             | Assert `"mounted"` or `"exiting"` on the root element.                       |
| animation listener setup              | DOM attrs             | Verify listener attached after `UnmountPending` entry inside rAF.            |
| animation completion → unmount        | machine state         | Assert machine transitions to `Unmounted` after `animationend`.              |
| fallback timeout → unmount            | machine state         | Assert machine transitions to `Unmounted` after 5000ms.                      |
| reduced-motion immediate completion   | machine state         | Assert `UnmountPending → Unmounted` without waiting for DOM event.           |
| listener cleanup on component unmount | cleanup side effects  | Assert `animationend`/`transitionend` listeners removed, timeouts cancelled. |
| `will-change` lifecycle               | DOM attrs             | Assert `will-change` set during animation, removed after.                    |
| lazy mount `ContentReady` dispatch    | machine state         | Assert `Mounting → Mounted` after content settles.                           |
| SSR rendered structure                | hydration structure   | Assert children and data attrs present in server HTML.                       |

Cheap verification recipe:

1. Render Presence with `present=true` and assert children are in the DOM with `data-ars-state="open"`.
2. Set `present=false` and assert `data-ars-state="closed"` and `data-ars-presence="exiting"`.
3. Fire a synthetic `animationend` on the root element and assert children are removed from the DOM.
4. Set `present=true` again and assert children re-appear with `data-ars-state="open"`.
5. Repeat step 2 but with `prefers-reduced-motion: reduce` active; assert children are removed immediately without waiting for `animationend`.
6. Unmount the component and assert all listeners and timeouts are cleaned up.

## 31. Implementation Checklist

- [ ] `use_presence` hook creates machine service, `NodeRef`, and reactive signals.
- [ ] `present` prop changes drive `api.sync_present()` via reactive effect.
- [ ] `Context.node_id` is set after client mount from the root element's ID.
- [ ] Children are conditionally rendered based on `api.is_mounted()`.
- [ ] `data-ars-state` is `"open"` only in `Mounted`; `Mounting`, `UnmountPending`, and `Unmounted` are `"closed"`.
- [ ] `data-ars-presence` is `"mounted"` when idle, `"exiting"` during exit animation.
- [ ] `animationend` and `transitionend` listeners attach only on `UnmountPending` entry, inside a double-rAF.
- [ ] `getComputedStyle()` reads are batched into a per-frame cache.
- [ ] Dual animation/transition completion: both must finish before `AnimationEnd` fires.
- [ ] Reduced-motion guard fires `AnimationEnd` immediately when active.
- [ ] `matchMedia` change listener fires `AnimationEnd` if reduced motion enabled mid-animation.
- [ ] 5000ms fallback timeout installed for `UnmountPending` and `Mounting` states.
- [ ] `visibilitychange` handler pauses/resumes the fallback timeout.
- [ ] `will-change: transform, opacity` set during animation, removed after.
- [ ] Lazy mount: `ContentReady` dispatched after content settles; 5000ms timeout forces it.
- [ ] `completed` guard prevents pending listeners from firing after cleanup.
- [ ] All cleanup is synchronous in a single `on_cleanup` call.
- [ ] SSR renders children with data attrs when present; omits when not present; no listeners.
- [ ] Hydration preserves server-rendered structure.
- [ ] No animation-related events fire during SSR.
