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
    /// Adapter-owned animation runtime helpers may use this ID to read computed
    /// styles and attach animation/transition listeners when translating DOM
    /// completion into `Event::AnimationEnd`.
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

1. The adapter SHOULD resolve that preference into `Props::reduce_motion`.
2. When `reduce_motion` is true:
    - entry and exit animations are treated as instant
    - the `Mounted → Unmount` transition skips `UnmountPending` and goes directly to `Unmounted`
    - adapters MUST NOT wait for `animationend` / `transitionend`
3. `skip_animation` is a component-level override for instant exit even when reduced motion is not
   active.
4. If an adapter supports live reduced-motion preference changes while an exit animation is in
   progress, it may immediately dispatch `Event::AnimationEnd`.

**Dynamic `prefers-reduced-motion` Changes:**

If an adapter/runtime helper observes a reduced-motion preference change while an exit animation is
in progress, it may immediately dispatch `Event::AnimationEnd`. The helper must check that the
machine is still in `UnmountPending` before doing so.

### 1.6 Page Visibility Pausing

When the page becomes hidden (user switches tabs, minimizes window), animations and timers should be paused to avoid wasted computation and unexpected behavior on return:

**`visibilitychange` Handling:**

- Adapter-owned animation runtime helpers may listen to `document.addEventListener('visibilitychange', ...)`.
- When `document.visibilityState === "hidden"`:
  - Pause the 5000ms fallback timeout by recording elapsed time via `performance.now()`.
  - Browsers may throttle or pause `requestAnimationFrame` callbacks in background tabs — do not rely on rAF for timing while hidden.
- When `document.visibilityState === "visible"`:
  - Resume the fallback timeout with the remaining duration.
  - Trigger a fresh `getComputedStyle()` read to check if the animation completed while hidden.

**Batch Announcements:**

- `LiveAnnouncer` SHOULD flush any queued announcements when the page returns to visibility, as announcements made while hidden may have been dropped by screen readers.

**Timing:**

- All duration calculations in adapter-owned Presence runtime helpers MUST use `performance.now()`
  (monotonic clock) rather than `Date.now()`.

### 1.7 Visibility Change Edge Cases

The following edge cases MUST be handled correctly by any visibility-aware Presence runtime helper:

1. **Timeout vs `animationend` race**: When the page returns to visibility, both the resumed fallback timeout and a belated `animationend` event may fire in close succession. The first event to call `send(Event::AnimationEnd)` wins and transitions the machine out of `UnmountPending`. The second call is a **no-op** because the machine has already transitioned to `Unmounted` — the state machine naturally ignores `AnimationEnd` events when not in an animating state. No additional deduplication logic is required.

2. **Element disconnected while tab hidden**: If the element is removed from the DOM while the tab is hidden (e.g., by a parent component unmounting), the visibility-resume handler MUST check `element.isConnected` before calling `getComputedStyle()`. Reading computed style on a disconnected element returns empty/default values and can cause incorrect state transitions. If the element is disconnected, skip the `getComputedStyle()` check and let the normal cleanup path handle unmounting.

3. **Duplicate listener guard**: Only **one** `animationend` listener may be active on the element
   at any time. The runtime helper must track whether a listener is already registered and skip
   duplicate registration.

### 1.8 Animation Completion Race with Unmount

When a component is being removed from the DOM while an animation is still in progress, a race condition can occur where animation completion callbacks fire on detached DOM nodes or attempt to access dropped state. The following rules prevent this:

1. **Synchronous cleanup on unmount**: Animation event listener cleanup MUST be synchronous during the Unmount transition. When the component begins unmounting (i.e., the framework's teardown/dispose lifecycle fires), immediately cancel all pending animation callbacks — do not defer cleanup to a microtask or `requestAnimationFrame`.

2. **Cancel pending animations immediately**: When the component is being removed from the DOM, do NOT wait for `animationend` or `transitionend` events. Use `animation.cancel()` (Web Animations API) or remove the animation class synchronously to halt running animations.

3. **Prevent late callbacks after teardown**: If `animationend` or `transitionend` fires after
   teardown has started, the callback MUST be a no-op. Runtime helpers should guard completion so
   detached-node callbacks cannot re-enter dropped component state.

4. **State access guard**: Animation callbacks that access component state (context, props, signals) MUST verify the component is still alive before reading or writing state. The exact ownership pattern is adapter-specific; the invariant is that teardown must prevent callbacks from observing dropped state.

5. **Cleanup ordering**: The runtime helper cleanup MUST:
    - prevent any pending listener from firing
    - cancel fallback timers
    - remove `animationend` and `transitionend` listeners
    - cancel adapter-owned animation handles where applicable
    - All of the above synchronously, in a single synchronous cleanup call

### 1.9 Full Machine Implementation

```rust
/// The machine for the `Presence` component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
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
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Unmounted, Event::Mount) if props.lazy_mount => Some(
                TransitionPlan::to(State::Mounting).apply(|ctx| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                }),
            ),
            (State::Unmounted, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                ctx.present = true;
                ctx.mounted = true;
                ctx.unmounting = false;
            })),
            (State::Mounting, Event::ContentReady) => Some(
                TransitionPlan::to(State::Mounted).apply(|ctx| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                }),
            ),
            (State::Mounting, Event::AnimationEnd) => None,
            (State::Mounting, Event::Unmount) => Some(TransitionPlan::to(State::Unmounted).apply(|ctx| {
                ctx.present = false;
                ctx.mounted = false;
                ctx.unmounting = false;
            })),
            (State::UnmountPending, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                ctx.present = true;
                ctx.mounted = true;
                ctx.unmounting = false;
            })),
            (State::Mounted, Event::Unmount) if props.skip_animation || props.reduce_motion => {
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
            })),
            (State::UnmountPending, Event::AnimationEnd) => {
                Some(TransitionPlan::to(State::Unmounted).apply(move |ctx| {
                    ctx.present = false;
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
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

> **Pure-core rule:** Presence owns only mount/unmount lifecycle state. It does not install DOM
> listeners or read computed styles inside `transition()`.
>
> **Adapter runtime boundary:** Adapters own the runtime that turns DOM animation completion into
> `Event::AnimationEnd`. That runtime may read `getComputedStyle()`, wait for rAF, listen for
> `animationend` / `transitionend`, and install timeouts. Presence only specifies the state machine
> boundary event.
>
> **Dead-state prevention:** No Presence instance may remain indefinitely in `Mounting` or
> `UnmountPending`. If lazy content never settles, adapters must eventually dispatch
> `ContentReady` or abort the mount. If exit completion never arrives, adapters must eventually
> dispatch `AnimationEnd`.
>
> **Runtime policy:** The preferred adapter implementation is a shared helper that:
>
> - batches style reads
> - checks both animation and transition durations
> - respects reduced motion
> - installs a bounded timeout fallback
> - filters bubbled child events
> - performs idempotent cleanup

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

        // data-ars-state="open" only once lazy content is ready and the machine
        // has entered Mounted. Mounting and exit phases stay "closed".
        // CSS entry animations should target the transition into Mounted.
        attrs.set(
            HtmlAttr::Data("ars-state"),
            if matches!(self.state, State::Mounted) { "open" } else { "closed" },
        );

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
- `data-ars-state` is `"open"` only in `Mounted`; `Mounting`, `UnmountPending`, and `Unmounted` expose `"closed"`.
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
    - Presence transitions to `UnmountPending`; adapter-owned animation runtime installs any
      needed `animationend` / `transitionend` listeners and timeout fallback.
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

- During `UnmountPending`, an adapter-owned animation runtime helper may schedule a
  `requestAnimationFrame` callback. If no frame callback fires within **100ms** (indicating the
  main thread is blocked or the tab is throttled), log a warning:
  `"Presence: animation frame stall detected (>100ms)"`.
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

1. The adapter/runtime helper sets a timeout fallback equal to animation duration + 500ms.
2. If `animationend`/`transitionend` doesn't fire before the timeout, the state machine force-completes the animation (transitions to the final state).
3. When the tab becomes visible again (`visibilitychange` event), any pending animations are re-evaluated.

## 11. Adapter Runtime Helper (Web)

Adapters should centralize Presence exit detection in a shared helper that translates DOM animation
completion into `Event::AnimationEnd`. A web implementation should follow this specification:

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

- Use one-shot completion semantics. Native `{ once: true }` is acceptable only
  when bubbled child events cannot consume the listener before the target check;
  otherwise keep the listener installed and ignore non-target events manually.
- Use `event.target().is_same_node(Some(&element))` for FFI-safe comparison (not Rust `==`)
- Call `event.stop_propagation()` to prevent parent animation bubbling
- Only the FIRST completion event counts

**Transition listener (`transitionend`):**

- Use the "longest-duration" approach: compute `max(duration[i] + delay[i])` across all transition properties
- Filter `transitionend` events by `event.propertyName` matching the longest-duration property — other property completions are ignored
- If the computed transition property resolves to `all`, the implementation may
  treat target-owned `transitionend` events as candidates and only accept one
  once the elapsed monotonic time has reached the computed longest-track
  deadline; the `max_total_ms + 10ms` timeout remains the authoritative backstop
- Install a safety-net timeout at `max_total_ms + 10ms`
- Cancel the timeout if the `transitionend` event fires first

### 11.3 Race Condition Safeguards

1. **Teardown safety:** Cleanup must tolerate the element already being detached. Any adapter-managed DOM writes after teardown begins must guard detached-node operations where needed.
2. **Event filtering:** Track which CSS properties animated via `event.propertyName` and only count the FIRST completion for each type.
3. **Bubbling prevention:** Use `event.stopPropagation()` on all animation/transition listeners to prevent parent CSS animations triggering on child elements.
4. **Style caching:** Cache `getComputedStyle()` result immediately to avoid re-reading during DOM modification.
5. **Single completion:** Only the FIRST `animationend` or `transitionend` event completes the transition. All listeners and timers MUST be cancelled immediately on the first completion event.
6. **Ownership safety:** Long-lived animation listeners must not keep component state alive past teardown. The exact ownership strategy is adapter-specific.

### 11.4 Fallback Timeout

A 5000ms ultimate fallback timeout guards against stuck animations where neither `animationend` nor `transitionend` fires (e.g., element removed from DOM mid-transition, browser bug). If teardown has already completed by the time the timeout fires, it must be a no-op.

### 11.5 Cleanup

The cleanup function returned by `on_animation_end` MUST:

1. Remove the `animationend` listener (if installed)
2. Remove the `transitionend` listener (if installed)
3. Cancel the safety-net transition timeout
4. Cancel the 5000ms fallback timeout
5. Mark the completion guard so late events and timeouts become no-ops

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
