---
component: Presence
category: overlay
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
  ark-ui: Presence
---

# Presence

Presence manages the mount/unmount lifecycle for elements that require exit animations. Without
Presence, removing an element from the DOM immediately cuts off any CSS exit animations. Presence
keeps elements in the DOM during their exit animation, then removes them when the animation
completes.

## 1. State Machine

### 1.1 States

```rust
/// The states of the presence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The element is not in the DOM.
    Unmounted,
    /// The element is being inserted into the DOM but lazy content has not yet settled (only used when `lazy_mount=true`). Entry animations do NOT start in this state.
    Mounting,
    /// The element is in the DOM and the `present` prop is true. Entry animation (`data-ars-state="open"`) triggers on transition to this state.
    Mounted,
    /// The element is in the DOM but an exit animation is running.
    UnmountPending,
}
```

### 1.2 Events

```rust
/// The events of the presence.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The `present` prop changed to true.
    Mount,
    /// The `present` prop changed to false.
    Unmount,
    /// Lazy content has settled and is ready to animate. Dispatched by the adapter after lazy content (e.g., suspended components) has resolved. Only relevant in the `Mounting` state.
    ContentReady,
    /// The CSS animation or transition on the element ended.
    AnimationEnd,
}
```

### 1.3 Context

```rust
/// The context of the presence.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the content should logically be present (the `present` prop value).
    pub present: bool,
    /// Whether the content is currently in the DOM (true during exit animation).
    pub mounted: bool,
    /// True while the exit animation is running.
    pub unmounting: bool,
    /// DOM node ID for the animated element. Set by the adapter after mounting.
    /// The effect uses this ID with `PlatformEffects` to read computed styles
    /// and attach animation/transition listeners.
    pub node_id: Option<String>,
}
```

### 1.4 Props

```rust
/// The props of the presence.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controls whether the content is present. Animate-out when changed to false.
    pub present: bool,
    /// When true, content is lazily mounted and may need to resolve (e.g., suspended
    /// components) before entry animations can begin. Transitions through the `Mounting`
    /// state, waiting for a `ContentReady` event before entering `Mounted`.
    pub lazy_mount: bool,
    /// When true, skip exit animations entirely (e.g., `prefers-reduced-motion`).
    /// Transitions from Mounted → Unmounted immediately without entering UnmountPending.
    pub skip_animation: bool,
    /// When true, skip ALL enter and exit animations (instant show/hide).
    ///
    /// **`prefers-reduced-motion` integration:**
    /// - The adapter SHOULD check `window.matchMedia('(prefers-reduced-motion: reduce)')` on
    ///   mount and set `reduce_motion` automatically based on the system preference.
    /// - When `reduce_motion` is true, both entry and exit animations are skipped entirely:
    ///   the element appears/disappears instantly without any transition states.
    /// - Unlike `skip_animation` (which only skips exit animations), `reduce_motion` also
    ///   suppresses entry animations — `data-ars-state="open"` is still set, but the adapter
    ///   ensures `animation-duration: 0s` and `transition-duration: 0s` are applied.
    /// - Can be overridden per-component for essential motion (e.g., progress indicators,
    ///   loading spinners) where animation conveys meaning rather than decoration.
    /// - Default: follows system preference via adapter. If the adapter detects
    ///   `prefers-reduced-motion: reduce`, this is `true`; otherwise `false`.
    pub reduce_motion: bool,
    // Change callbacks provided by the adapter layer
}
```

### 1.5 Reduced Motion

When the user has `prefers-reduced-motion: reduce` set in their OS/browser preferences:

1. **All animations MUST complete in 0ms** or be omitted entirely. Presence detects this via `window.matchMedia('(prefers-reduced-motion: reduce)')`.
2. When reduced motion is active:
   - `skip_animation` in Props is effectively `true` regardless of its configured value.
   - The `Mounted → Unmount` transition skips `UnmountPending` entirely and goes directly to `Unmounted`.
   - Entry animations (`data-ars-state="open"`) still apply but with `animation-duration: 0s` — the element appears instantly.
3. Presence MUST **not wait** for `animationend` or `transitionend` events when reduced motion is active, as these events may never fire when durations are zero.
4. The `matchMedia` query is evaluated **once** at effect setup time and cached for the lifecycle of that effect. However, Presence MUST also register a dynamic listener to handle mid-animation preference changes as described below.

**Dynamic `prefers-reduced-motion` Change Listener:**

Presence MUST register a `change` event listener on the `MediaQueryList` returned by `window.matchMedia("(prefers-reduced-motion: reduce)")` to handle the case where a user enables reduced motion while an exit animation is in progress.

```rust
// Setup: register alongside the animationend listener in the
// "listen-animation-end" effect.
let platform = use_platform_effects();

let cleanup = platform.on_reduced_motion_change(Box::new(move |reduced| {
    if reduced {
        // User just enabled reduced motion.
        // Only fire if the machine is currently in an animating state.
        if matches!(current_state(), State::UnmountPending) {
            send(Event::AnimationEnd);
        }
    }
}));

// Cleanup: remove the listener when the effect is disposed or
// the element unmounts, to prevent leaked listeners.
on_cleanup(move || {
    cleanup();
});
```

**Behaviour constraints:**

- The listener MUST check the current machine state before firing `Event::AnimationEnd`. If the machine is not in `UnmountPending` (i.e., no animation is running), the `change` event is ignored.
- Only one `change` listener may be active per Presence instance at any time — it is registered and torn down with the same lifecycle as the `animationend` listener.
- When the listener fires `AnimationEnd`, the normal transition path applies: `UnmountPending → Unmounted`, and the element is removed from the DOM immediately.

### 1.6 Page Visibility Pausing

When the page becomes hidden (user switches tabs, minimizes window), animations and timers should be paused to avoid wasted computation and unexpected behavior on return:

**`visibilitychange` Handling:**

- Listen to `document.addEventListener('visibilitychange', ...)` in the `"listen-animation-end"` effect.
- When `document.visibilityState === "hidden"`:
  - Pause the 5000ms fallback timeout by recording elapsed time via `performance.now()`.
  - Browsers may throttle or pause `requestAnimationFrame` callbacks in background tabs — do not rely on rAF for timing while hidden.
- When `document.visibilityState === "visible"`:
  - Resume the fallback timeout with the remaining duration.
  - Trigger a fresh `getComputedStyle()` read to check if the animation completed while hidden.

**Batch Announcements:**

- `LiveAnnouncer` SHOULD flush any queued announcements when the page returns to visibility, as announcements made while hidden may have been dropped by screen readers.

**Timing:**

- All duration calculations in Presence effects MUST use `performance.now()` (monotonic clock) rather than `Date.now()` to avoid issues with system clock changes and to get sub-millisecond precision.

### 1.7 Visibility Change Edge Cases

The following edge cases MUST be handled correctly by the visibility-change integration in the `"listen-animation-end"` effect:

1. **Timeout vs `animationend` race**: When the page returns to visibility, both the resumed fallback timeout and a belated `animationend` event may fire in close succession. The first event to call `send(Event::AnimationEnd)` wins and transitions the machine out of `UnmountPending`. The second call is a **no-op** because the machine has already transitioned to `Unmounted` — the state machine naturally ignores `AnimationEnd` events when not in an animating state. No additional deduplication logic is required.

2. **Element disconnected while tab hidden**: If the element is removed from the DOM while the tab is hidden (e.g., by a parent component unmounting), the visibility-resume handler MUST check `element.isConnected` before calling `getComputedStyle()`. Reading computed style on a disconnected element returns empty/default values and can cause incorrect state transitions. If the element is disconnected, skip the `getComputedStyle()` check and let the normal cleanup path handle unmounting.

3. **Duplicate listener guard**: Only **one** `animationend` listener may be active on the element at any time. The effect MUST track whether a listener is already registered (e.g., via a boolean flag or by storing the listener handle) and skip registration if one exists. This prevents the scenario where repeated visibility toggles (hidden → visible → hidden → visible) accumulate multiple `animationend` listeners, each of which would independently call `send(Event::AnimationEnd)` and potentially interfere with future animation cycles.

### 1.8 Animation Completion Race with Unmount

When a component is being removed from the DOM while an animation is still in progress, a race condition can occur where animation completion callbacks fire on detached DOM nodes or attempt to access dropped state. The following rules prevent this:

1. **Synchronous cleanup on unmount**: Animation event listener cleanup MUST be synchronous during the Unmount transition. When the component begins unmounting (i.e., the framework's teardown/dispose lifecycle fires), immediately cancel all pending animation callbacks — do not defer cleanup to a microtask or `requestAnimationFrame`.

2. **Cancel pending animations immediately**: When the component is being removed from the DOM, do NOT wait for `animationend` or `transitionend` events. Use `animation.cancel()` (Web Animations API) or remove the animation class synchronously to halt running animations.

3. **Prevent callbacks on detached nodes**: If `animationend` or `transitionend` fires after the element has been removed from the DOM (detectable via `!document.contains(node)`), the callback MUST be a no-op. The `completed` guard (`Rc<Cell<bool>>`) in the `"listen-animation-end"` effect serves this purpose — set it to `true` during cleanup.

4. **State access guard**: Animation callbacks that access component state (context, props, signals) MUST verify the component is still alive before reading or writing state. In Rust frameworks, this means checking that `Rc`/`Arc` references have not been dropped (e.g., `Weak::upgrade()` returns `Some`).

5. **Cleanup ordering**: The effect cleanup function MUST:
   - Set the `completed` flag to `true` (prevents any pending listener from firing)
   - Cancel the fallback timeout
   - Remove `animationend` and `transitionend` event listeners
   - Call `animation.cancel()` on any Web Animations API animations
   - All of the above synchronously, in a single synchronous cleanup call

### 1.9 Full Machine Implementation

```rust
/// The machine for the `Presence` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let initial_state = if props.present {
            State::Mounted
        } else {
            State::Unmounted
        };
        let ctx = Context {
            present: props.present,
            mounted: props.present,
            unmounting: false,
            node_id: None,
        };
        (initial_state, ctx)
    }

    fn transition(
        state: &State,
        event: &Event,
        _ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Present becomes true ─────────────────────────────────────────
            // When lazy_mount is true, go through Mounting first so lazy content
            // can resolve before the entry animation starts. The adapter MUST
            // dispatch ContentReady after lazy content has settled.
            (State::Unmounted, Event::Mount) if props.lazy_mount => {
                Some(TransitionPlan::to(State::Mounting).apply(|ctx| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                    // Note: data-ars-state remains unset — entry animation does NOT
                    // trigger until the transition to Mounted.
                }).with_named_effect("mounting-timeout", |_ctx, _props, send| {
                    // Safety-net timeout for Mounting state.
                    // If ContentReady is never dispatched (lazy content fails to load),
                    // forcibly transition to Mounted after 5000ms.
                    let platform = use_platform_effects();
                    let handle = platform.set_timeout(5000, Box::new(move || {
                        send(Event::ContentReady);
                    }));
                    let pc = platform.clone();
                    Box::new(move || { pc.clear_timeout(handle); }) as CleanupFn
                }))
            }
            (State::Unmounted, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                ctx.present = true;
                ctx.mounted = true;
                ctx.unmounting = false;
            })),
            // Lazy content has settled — now safe to start the entry animation.
            // The adapter sets data-ars-state="open" on transition to Mounted.
            (State::Mounting, Event::ContentReady) => Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                // Context already has present=true, mounted=true from Mounting.
                // Entry animation triggers now via data-ars-state="open".
            })),
            // `(Mounting, AnimationEnd)` — intentionally ignored. AnimationEnd
            // during mounting comes from parent animation bubbling (e.g., an
            // ancestor element's entry animation completing) and does not
            // affect mount lifecycle. The Mounting state waits exclusively for
            // `ContentReady` to proceed to `Mounted`.
            (State::Mounting, Event::AnimationEnd) => None,

            // If unmount is requested while still mounting, go straight to Unmounted.
            (State::Mounting, Event::Unmount) => Some(TransitionPlan::to(State::Unmounted).apply(|ctx| {
                ctx.present = false;
                ctx.mounted = false;
                ctx.unmounting = false;
            })),
            // Cancels a pending unmount if present becomes true again during exit.
            (State::UnmountPending, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                ctx.present = true;
                ctx.unmounting = false;
            })),

            // ── Present becomes false ────────────────────────────────────────
            (State::Mounted, Event::Unmount) if props.skip_animation => {
                // skip_animation is true (e.g., prefers-reduced-motion) — skip
                // UnmountPending entirely to avoid waiting for animationend that
                // will never fire.
                Some(TransitionPlan::to(State::Unmounted).apply(|ctx| {
                    ctx.present = false;
                    ctx.mounted = false;
                    ctx.unmounting = false;
                }))
            }
            (State::Mounted, Event::Unmount) => Some(TransitionPlan::to(State::UnmountPending).apply(|ctx| {
                ctx.present = false;
                ctx.unmounting = true;
                // mounted remains true — element stays in DOM during exit animation
            }).with_named_effect("listen-animation-end", |ctx, _props, send| {
                // Read the DOM node ID from context (set by adapter after mounting).
                let node_id = match ctx.node_id.clone() {
                    Some(id) => id,
                    None => {
                        // No DOM node ID — cannot detect animation, unmount immediately.
                        send(Event::AnimationEnd);
                        return no_cleanup() as CleanupFn;
                    }
                };

                // Delegate all animation/transition detection and completion
                // listening to PlatformEffects. The platform handles:
                //   - Reading computed styles (animationDuration, transitionDuration)
                //   - Installing animationend/transitionend listeners
                //   - The reduced-motion guard (fires immediately if active)
                //   - The 5000ms fallback timeout
                //   - Cleanup of listeners and timeouts
                //
                // The adapter MUST run the platform call inside a requestAnimationFrame
                // callback after setting data-ars-state="closed", to ensure the browser
                // has applied the new styles before reading computed properties.
                //
                // **Reflow batching:** All computed style reads MUST be batched
                // into a single rAF callback to avoid layout thrashing. Cache
                // detection results per element so that repeated reads within a
                // frame are free.
                // Check BOTH `animationDuration` AND `transitionDuration` — not
                // just animation — to correctly detect transition-only exit
                // effects.
                let platform = use_platform_effects();
                let send_weak = Rc::downgrade(&send);

                // `on_animation_end` installs listeners for animationend and
                // transitionend on the element identified by `node_id`, reads
                // computed styles to detect which types are active, handles the
                // reduced-motion guard (fires callback immediately if reduced
                // motion is active or both durations are zero), waits for BOTH
                // animation and transition to complete when both are active, and
                // installs a 5000ms fallback timeout. Returns a cleanup function.
                //
                // Race condition safeguards (handled by the platform):
                // 1. Verify element is still connected before DOM operations.
                // 2. Filter events by target to ignore bubbled child events.
                // 3. Stop propagation to prevent parent animation bubbling.
                // 4. Only the FIRST completion event triggers the callback;
                //    all remaining listeners and timers are cancelled immediately.
                let cleanup = platform.on_animation_end(&node_id, Box::new(move || {
                    if let Some(send) = send_weak.upgrade() {
                        send(Event::AnimationEnd);
                    }
                }));

                cleanup
            })),

            // ── Animation completes ──────────────────────────────────────────
            (State::UnmountPending, Event::AnimationEnd) => {
                Some(TransitionPlan::to(State::Unmounted).apply(move |ctx| {
                    ctx.mounted = false;
                    ctx.unmounting = false;
                }))
            }

            _ => None,
        }
    }

    // ── Exit Animation Cleanup Timing ─────────────────────────────────
    //
    // When a Dialog or Popover with `unmount_on_exit` closes, the Presence
    // component animates out. Effect cleanup (including inert attribute
    // removal, scroll lock release, and backdrop dismissal) MUST be
    // deferred until AFTER the exit animation completes. The expected
    // sequence is:
    //
    //   1. Close Event received → State transitions to UnmountPending
    //   2. `ctx.present = false`, `ctx.unmounting = true`
    //   3. Adapter sets `data-ars-state="closed"` → CSS exit animation starts
    //   4. During UnmountPending: element stays in DOM, `inert` attributes
    //      remain on background content, scroll lock stays active
    //   5. `animationend`/`transitionend` fires → Event::AnimationEnd sent
    //   6. State transitions to Unmounted → NOW cleanup runs:
    //      - Remove `inert` from background elements
    //      - Release scroll lock (decrement ref count)
    //      - Restore focus to trigger element
    //      - Remove element from DOM
    //
    // The adapter MUST NOT clean up inert/scroll-lock effects on the
    // Mounted→UnmountPending transition. Instead, these cleanups are
    // tied to the Unmounted state. The `defer_cleanup_until_exit_complete`
    // pattern ensures users cannot accidentally interact with background
    // elements during the exit animation.

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

> **Animation-agnostic detection:** Presence detects CSS animations and transitions automatically
> by reading `getComputedStyle()`. No `animation_type` prop is needed. Both `animation-*` and
> `transition-*` CSS properties are supported simultaneously. The effect inspects
> `animationName` (for CSS animations) and `transitionProperty` (for CSS transitions) to
> determine which event listeners to install.
>
> **Reflow batching for `getComputedStyle()`:** When multiple Presence instances unmount in the same frame (e.g., a Dialog closing its overlay and content simultaneously), each instance's effect reads `getComputedStyle()`. To avoid N forced reflows, the adapter MUST batch all style reads into a single `requestAnimationFrame` callback. Implementation: maintain a per-frame `WeakRef<Element> → CachedStyleDetection` map. The first read for an element computes and caches; subsequent reads within the same frame return the cached result. The cache is cleared at the start of the next frame.
>
> **Both durations checked:** The effect MUST check both `animationDuration` AND `transitionDuration` (not just one) to correctly detect elements that use CSS transitions without CSS animations, and vice versa.
>
> **5000ms timeout fallback:** If neither `animationend` nor `transitionend` fires within 5000ms (e.g., animation cancelled mid-flight, browser bug, element removed from DOM), the fallback timeout fires `AnimationEnd` to prevent the element from being permanently stuck in `UnmountPending`.
>
> **Animation detection is in the effect, not in `transition()`.** The `Mounted → Unmount` transition always enters `UnmountPending` without reading the DOM. The `"listen-animation-end"` effect is responsible for all DOM reads (`getComputedStyle`) and deciding whether to set up listeners or fire `AnimationEnd` immediately. This keeps `transition()` a pure function per the architecture contract (`01-architecture.md` §2.1).
>
> **Reduced-motion guard (handled by effect):** The effect checks `window.matchMedia('(prefers-reduced-motion: reduce)')` and `getComputedStyle(node).animationDuration` / `transitionDuration`. If reduced motion is active or both durations are `'0s'`, the effect fires `AnimationEnd` immediately, causing the machine to proceed to `Unmounted` without waiting.
>
> **Fallback timeout (5000ms):** The effect always installs a 5000ms safety-net timeout alongside any event listeners. If neither `animationend` nor `transitionend` fires within 5 seconds (e.g., animation removed mid-flight, browser bug), the timeout fires `AnimationEnd` to prevent a permanent hang. The timeout is cancelled by the effect cleanup on state change. **Only the FIRST `animationend` or `transitionend` event completes the transition. All listeners and timers MUST be cancelled immediately on the first event to prevent double-firing.** When the fallback timeout fires, it must also cancel all remaining event listeners. Conversely, when an event listener fires first, it must cancel the fallback timer.
>
> **Timeout integration for ALL animation-waiting states:** The 5000ms fallback timeout MUST be applied to every state that waits for an animation or transition event. Specifically:
>
> - **`UnmountPending`**: Already has the timeout via the `"listen-animation-end"` effect (see implementation above).
> - **`Mounting` (when entry animation is pending)**: If `ContentReady` is never dispatched (e.g., lazy content fails to load), a 5000ms timeout MUST fire `ContentReady` to prevent the element from being stuck in `Mounting` forever. This is implemented as a `"mounting-timeout"` effect on the `(Unmounted, Mount) when lazy_mount` transition.
> - **Entry animation (Mounted state)**: If the entry animation (`data-ars-state="open"`) never fires `animationend`, the element is already visible and functional — no timeout is needed (the element is usable despite the missing animation event).
>
> **Dead state prevention rule**: No Presence state may wait indefinitely for a DOM event. Every state that installs an event listener MUST also install a safety-net timeout. After the timeout fires, the machine forcibly completes the pending transition (close → Unmounted, or mount → Mounted).
>
> **Animation detection timing:** After setting `data-ars-state="closed"` to trigger the CSS exit animation, the adapter MUST wait one animation frame (`requestAnimationFrame`) before the effect reads `getComputedStyle(node).animationName` or `transitionProperty`. This ensures the browser has applied the new styles. **Safari requirement:** always use `requestAnimationFrame` before reading computed styles — Safari may return stale values without it. Additionally, `getComputedStyle()` caches MUST be invalidated after DOM mutations (class changes, attribute updates) by forcing a style recalculation via `requestAnimationFrame` or accessing `offsetHeight` before reading animation properties.
>
> **Style detection timing.** After inserting an element into the DOM, `getComputedStyle()`
> may return stale or empty values for animation/transition properties. Use double-rAF
> (two nested `requestAnimationFrame` calls) before reading computed styles to ensure
> the browser has completed style calculation:
>
> ```js
> requestAnimationFrame(() => {
>   requestAnimationFrame(() => {
>     const style = getComputedStyle(element);
>     // Now safe to read transitionDuration, animationName, etc.
>   });
> });
> ```
>
> **Mounting state for lazy content:** The `Mounting` state allows lazy content (e.g., suspended components) to resolve before entry animations begin. When `lazy_mount=true`, the `(Unmounted, Mount)` transition goes to `Mounting` instead of `Mounted`. The element is inserted into the DOM but `data-ars-state="open"` is NOT set yet — no entry animation runs. The adapter MUST dispatch `ContentReady` after lazy content has settled, which transitions to `Mounted` and triggers the entry animation. When `lazy_mount=false`, the `Mounting` state is bypassed entirely (`Unmounted` → `Mounted` directly). If `Unmount` is received while in `Mounting`, the machine transitions directly to `Unmounted` without any exit animation.
>
> **Dual animation/transition support:** The effect listens for BOTH `animationend` AND `transitionend` events. When both an animation and a transition are active, the effect waits for BOTH to complete before sending `AnimationEnd`. Two `Rc<Cell<bool>>` flags (`anim_done`, `trans_done`) track completion; each is pre-set to `true` if that type is not active. Each listener sets its flag and checks the other before firing `AnimationEnd`. If only `transition-property` is detected (no `animation-name`), only `transitionend` is attached, and vice versa. If neither is detected, `AnimationEnd` fires immediately.
>
> **Transition completion detection.** When `transition-property:all` is set, the browser fires
> one `transitionend` event per animated property, making event counting unreliable.
> Instead, use the longest-duration approach:
>
> 1. Read `getComputedStyle(element).transitionDuration` and `transitionDelay`
> 2. Parse all durations, compute `max(duration + delay)` for the longest transition
> 3. Set a `setTimeout(max_total_duration + 10ms)` as the completion signal
> 4. If a `transitionend` fires for the longest-duration property **BEFORE** the timeout, cancel
>    the timeout and complete immediately
> 5. The timeout serves as a safety net for cases where `transitionend` doesn't fire
>    (e.g., display:none mid-transition, element removed from DOM)

### 1.10 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "presence"]
pub enum Part {
    Root,
}

/// The API of the `Presence` component.
pub struct Api<'a> {
    /// The state of the `Presence` component.
    state: &'a State,
    /// The context of the `Presence` component.
    ctx: &'a Context,
    /// The props of the `Presence` component.
    props: &'a Props,
    /// The send callback.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the content should be in the DOM.
    /// Framework adapters use this to conditionally render children.
    pub fn is_mounted(&self) -> bool {
        self.ctx.mounted
    }

    /// Whether the content is logically present (the `present` prop value).
    pub fn is_present(&self) -> bool {
        self.ctx.present
    }

    /// Whether an exit animation is currently running.
    pub fn is_unmounting(&self) -> bool {
        self.ctx.unmounting
    }

    /// The attributes for the root element (the animated content wrapper).
    /// Includes `data-ars-state` for CSS animation targeting and
    /// `data-ars-presence` for animation phase indication.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        // data-ars-state="open" when present, "closed" when unmounting.
        // CSS exit animations should target [data-ars-state="closed"].
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_present() { "open" } else { "closed" });

        // data-ars-presence indicates the current animation phase:
        //   "exiting"   — exit animation is playing (element still in DOM)
        //   "mounted"   — element is mounted and idle (no animation in progress)
        // Entry animations should target [data-ars-state="open"] instead.
        attrs.set(HtmlAttr::Data("ars-presence"), if self.is_unmounting() { "exiting" } else { "mounted" });

        attrs
    }

    /// Call this when the `present` prop changes.
    pub fn sync_present(&self, new_present: bool) {
        if new_present {
            (self.send)(Event::Mount);
        } else {
            (self.send)(Event::Unmount);
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Presence
└── Root  <any>  data-ars-scope="presence" data-ars-part="root"
                 data-ars-state="open|closed"
                 data-ars-presence="mounted|exiting"
```

| Part | Element | Key Attributes                                                                             |
| ---- | ------- | ------------------------------------------------------------------------------------------ |
| Root | any     | `data-ars-scope="presence"`, `data-ars-part="root"`, `data-ars-state`, `data-ars-presence` |

The Root element is the consumer-provided element that Presence wraps. The adapter applies Presence attributes to it for CSS animation targeting.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

Presence is a utility component with no ARIA role or interactive semantics of its own. It does not add `role`, `aria-*`, or `tabindex` attributes. Its only DOM impact is the `data-ars-state` and `data-ars-presence` data attributes used for CSS animation targeting.

The overlay component that composes Presence (Dialog, Tooltip, etc.) is responsible for all ARIA attributes on the content element. Presence defers entirely to the parent component's accessibility contract.

**Reduced motion**: When `prefers-reduced-motion: reduce` is active, Presence skips animations entirely (see §1.5). This ensures screen reader users and users with vestibular disorders are not exposed to motion they have opted out of.

## 4. Internationalization

- No translatable strings. Presence has no user-facing text.
- `data-ars-state` values (`open`, `closed`) and `data-ars-presence` values (`mounted`, `exiting`) are stable API tokens, not localized.

## 5. CSS Integration

```css
/* Entry animation — runs when data-ars-state transitions to "open" */
[data-ars-scope="tooltip"][data-ars-state="open"] {
  animation: tooltip-in 150ms ease-out;
}

/* Exit animation — runs when data-ars-state transitions to "closed"
   Presence keeps the element in the DOM until this finishes. */
[data-ars-scope="tooltip"][data-ars-state="closed"] {
  animation: tooltip-out 100ms ease-in;
}

@keyframes tooltip-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes tooltip-out {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.95);
  }
}
```

## 6. Composition Pattern (Mandatory for Overlay Components)

The following overlay components **MUST** wrap their content in a Presence machine instance:

- **Dialog** / **AlertDialog**: Content panel
- **Drawer**: Content panel
- **Popover**: Content panel
- **Tooltip**: Content panel
- **HoverCard**: Content panel
- **Menu** / **ContextMenu**: Content panel
- **Select** / **Combobox**: Listbox content

**Timing contract:**

1. **Entry** (mount → visible):
   - Presence transitions to `Mounted` — the adapter renders the element into the DOM.
   - The overlay component sets `data-ars-state="open"` — CSS entry animation triggers.
   - If no animation, both steps appear instantaneous.

2. **Exit** (visible → unmount):
   - The overlay component sets `data-ars-state="closed"` — CSS exit animation triggers.
   - Presence transitions to `UnmountPending` and the `"listen-animation-end"` effect attaches `animationend`/`transitionend` listeners.
   - On animation/transition end, Presence transitions to `Unmounted` — the adapter removes the element from the DOM.
   - If no exit animation/transition is detected (or `skip_animation` is true), Presence skips `UnmountPending` and unmounts immediately.

**Consequence**: Overlay components that skip Presence will unmount instantly, losing exit animations.

## 7. Internal Usage in ars-ui

| Component           | Presence Config                                                 |
| ------------------- | --------------------------------------------------------------- |
| `Tooltip`           | `present` = tooltip open state                                  |
| `Dialog`            | `present` = dialog open state                                   |
| `Popover`           | `present` = popover open state                                  |
| `Toast`             | `present` = toast visible state (removed on dismiss or timeout) |
| `Select` dropdown   | `present` = listbox open state                                  |
| `Combobox` dropdown | `present` = listbox open state                                  |

## 8. Animation Performance Monitoring

Presence MAY integrate optional animation performance monitoring for debugging and telemetry:

**rAF Stall Detection:**

- During `UnmountPending`, the `"listen-animation-end"` effect schedules a `requestAnimationFrame` callback. If no frame callback fires within **100ms** (indicating the main thread is blocked or the tab is throttled), log a warning: `"Presence: animation frame stall detected (>100ms)"`.
- Use `performance.now()` timestamps to measure the gap between requesting and receiving animation frames.

**PerformanceObserver Integration (Optional Debug Mode):**

- When `debug_animations: true` is set (debug builds only), register a `PerformanceObserver` with `entryTypes: ["measure"]` to capture animation durations.
- Mark animation start (`performance.mark("ars-presence-exit-start")`) on entering `UnmountPending` and end (`performance.mark("ars-presence-exit-end")`) on `AnimationEnd`.
- Measure with `performance.measure("ars-presence-exit", "ars-presence-exit-start", "ars-presence-exit-end")`.

**Logging:**

- In debug mode, log animation duration to the console: `"Presence exit animation completed in {N}ms"`.
- In production, no performance observers or marks are registered.

## 9. GPU Acceleration Hints

To maintain 60fps during Presence enter/exit animations:

1. **`will-change` Property**: The adapter should set `will-change: transform, opacity` on the animating element before the animation starts (during the `enter`/`exit` setup phase). Remove it after animation completes to free GPU memory.
2. **Compositor-Only Properties**: Prefer animating only `transform` and `opacity` — these run on the GPU compositor thread and avoid layout/paint. Avoid animating `width`, `height`, `top`, `left`, `margin`, or `padding` as these trigger layout recalculation.
3. **Subpixel Aliasing**: When using `transform: translate3d()` for GPU promotion, fractional pixel values (e.g., `translate3d(0, 1.5px, 0)`) may cause text aliasing artifacts. Round translate values to whole pixels for text-heavy elements.
4. **Fallback**: If the user has `prefers-reduced-motion: reduce` active, skip GPU promotion entirely — the animation is replaced with instant show/hide and no compositing overhead is needed.

## 10. Hidden Tab Handling

On hidden tabs, browsers throttle `requestAnimationFrame` to ≤1fps. To prevent animations from stalling:

1. Presence sets a timeout fallback equal to animation duration + 500ms.
2. If `animationend`/`transitionend` doesn't fire before the timeout, the state machine force-completes the animation (transitions to the final state).
3. When the tab becomes visible again (`visibilitychange` event), any pending animations are re-evaluated.

## 11. Platform Implementation: `on_animation_end` (Web)

The `PlatformEffects::on_animation_end(node_id, callback)` method encapsulates the full animation/transition detection algorithm. The web implementation (`WebPlatformEffects`) MUST follow this specification:

### 11.1 Detection Phase

Run inside a `requestAnimationFrame` callback after the adapter has set `data-ars-state="closed"` to ensure the browser has applied new styles.

```rust
// Read computed styles for the element
let style = get_computed_style(&node);
let anim_name = style.animation_name();
let anim_dur = style.animation_duration();
let trans_dur = style.transition_duration();

// Parse duration strings: "0s", "0.0s", "0.3s, 0.5s" for multi-property
fn is_zero_duration(value: &str) -> bool {
    value.split(',').all(|v| {
        let trimmed = v.trim().trim_end_matches('s');
        trimmed.parse::<f64>().map_or(true, |d| d <= 0.0)
    })
}

// Reduced-motion guard: fire immediately if both durations are zero
// or prefers-reduced-motion is active.
let reduced = window_match_media("(prefers-reduced-motion: reduce)");
if reduced || (is_zero_duration(&anim_dur) && is_zero_duration(&trans_dur)) {
    callback();
    return no_cleanup();
}

let has_animation = !is_zero_duration(&anim_dur)
    && anim_name != "none" && !anim_name.is_empty();
let has_transition = !is_zero_duration(&trans_dur);
```

### 11.2 Listener Setup

When both CSS animation and transition are active, wait for **BOTH** to complete.

**Animation listener (`animationend`):**

- Use `{ once: true }` to prevent duplicate fires
- Use `event.target().is_same_node(Some(&element))` for FFI-safe comparison (not Rust `==`)
- Call `event.stop_propagation()` to prevent parent animation bubbling
- Only the FIRST completion event counts

**Transition listener (`transitionend`):**

- Use the "longest-duration" approach: compute `max(duration[i] + delay[i])` across all transition properties
- Filter `transitionend` events by `event.propertyName` matching the longest-duration property — other property completions are ignored
- Install a safety-net timeout at `max_total_ms + 10ms`
- Cancel the timeout if the `transitionend` event fires first

### 11.3 Race Condition Safeguards

1. **DOM detachment:** Verify node is still in DOM (`document.contains(node)`) before DOM operations in cleanup. Component unmount during animation may leave a detached element.
2. **Event filtering:** Track which CSS properties animated via `event.propertyName` and only count the FIRST completion for each type.
3. **Bubbling prevention:** Use `event.stopPropagation()` on all animation/transition listeners to prevent parent CSS animations triggering on child elements.
4. **Style caching:** Cache `getComputedStyle()` result immediately to avoid re-reading during DOM modification.
5. **Single completion:** Only the FIRST `animationend` or `transitionend` event completes the transition. All listeners and timers MUST be cancelled immediately on the first completion event.
6. **Weak references:** Use `Weak` references for send callbacks in long-lived animation listeners to avoid preventing service cleanup.

### 11.4 Fallback Timeout

A 5000ms ultimate fallback timeout guards against stuck animations where neither `animationend` nor `transitionend` fires (e.g., element removed from DOM mid-transition, browser bug). If the service has been dropped by the time the timeout fires, silently ignore (`Weak::upgrade` returns `None`).

### 11.5 Cleanup

The cleanup function returned by `on_animation_end` MUST:

1. Remove the `animationend` listener (if installed)
2. Remove the `transitionend` listener (if installed)
3. Cancel the safety-net transition timeout
4. Cancel the 5000ms fallback timeout
5. Guard all DOM cleanup operations with `document.contains(node)` — skip if the element was removed

## 12. Library Parity

> Compared against: Ark UI (`Presence`).

Radix UI and React Aria do not have a standalone Presence component (Radix uses `forceMount` props; React Aria uses `isEntering`/`isExiting` render props).

### 12.1 Props

| Feature                 | ars-ui                   | Ark UI                 | Notes                                                     |
| ----------------------- | ------------------------ | ---------------------- | --------------------------------------------------------- |
| Present                 | `present`                | `present`              | Both                                                      |
| Lazy mount              | `lazy_mount`             | `lazyMount`            | Both                                                      |
| Skip animation          | `skip_animation`         | --                     | ars-ui addition                                           |
| Reduce motion           | `reduce_motion`          | --                     | ars-ui addition for prefers-reduced-motion                |
| Immediate               | --                       | `immediate`            | Ark UI sync mode; ars-ui handles via skip_animation       |
| Skip animation on mount | --                       | `skipAnimationOnMount` | Ark UI only; ars-ui uses Presence state machine           |
| Unmount on exit         | (caller controls)        | `unmountOnExit`        | Ark UI prop; in ars-ui, the composing component owns this |
| Exit complete           | (via AnimationEnd event) | `onExitComplete`       | Ark UI callback; ars-ui uses state machine transition     |

**Gaps:** None.

### 12.2 Anatomy

| Part | ars-ui | Ark UI     | Notes                                        |
| ---- | ------ | ---------- | -------------------------------------------- |
| Root | Root   | (children) | ars-ui wraps children; Ark UI passes through |

**Gaps:** None.

### 12.3 Events

| Callback      | ars-ui              | Ark UI           | Notes                           |
| ------------- | ------------------- | ---------------- | ------------------------------- |
| Exit complete | Event::AnimationEnd | `onExitComplete` | ars-ui uses state machine event |

**Gaps:** None.

### 12.4 Features

| Feature                           | ars-ui | Ark UI    |
| --------------------------------- | ------ | --------- |
| Mount/unmount lifecycle           | Yes    | Yes       |
| CSS animation detection           | Yes    | Yes       |
| CSS transition detection          | Yes    | Yes       |
| Lazy content mounting             | Yes    | Yes       |
| Reduced motion support            | Yes    | (via CSS) |
| Fallback timeout (5000ms)         | Yes    | --        |
| Page visibility pausing           | Yes    | --        |
| Dynamic reduced-motion listener   | Yes    | --        |
| Animation performance monitoring  | Yes    | --        |
| GPU acceleration hints            | Yes    | --        |
| Dual animation+transition support | Yes    | --        |
| Reflow batching                   | Yes    | --        |
| Mounting state for lazy content   | Yes    | --        |

**Gaps:** None.

### 12.5 Summary

- **Overall:** Full parity with Ark UI; significantly exceeds reference with robustness features.
- **Divergences:** (1) ars-ui uses a full state machine (Unmounted/Mounting/Mounted/UnmountPending) instead of a simple boolean toggle, enabling the Mounting state for lazy content. (2) ars-ui adds a 5000ms fallback timeout, page visibility pausing, dynamic reduced-motion detection, and reflow batching -- none of which are specified in Ark UI. (3) `skip_animation` and `reduce_motion` are explicit props rather than relying solely on CSS `prefers-reduced-motion`.
- **Recommended additions:** None.
