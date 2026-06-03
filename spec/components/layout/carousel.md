---
component: Carousel
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
    ark-ui: Carousel
---

# Carousel

`Carousel` presents a sequence of slides with previous/next buttons, dot indicators, keyboard arrow keys, touch/pointer swipe with momentum, and optional auto-play. Supports looping navigation, fractional slides-per-view, configurable alignment, and full WAI-ARIA carousel pattern compliance.

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    /// No animation or auto-play in progress.
    #[default]
    Idle,
    /// Auto-play timer is running.
    AutoPlaying,
    /// A slide transition animation is in progress.
    Transitioning,
}
```

### 1.2 Events

```rust
// Not `Copy` because `SyncProps` carries an owned `Props`.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Navigate to a specific slide by index.
    GoToSlide { index: usize },
    /// Navigate to the next slide.
    GoToNext,
    /// Navigate to the previous slide.
    GoToPrev,
    /// Start the auto-play timer.
    AutoPlayStart,
    /// Permanently stop auto-play.
    AutoPlayStop,
    /// Auto-play timer fired; advance one slide.
    AutoPlayTick,
    /// Manually pause auto-play (e.g. the auto-play trigger button). Pauses
    /// regardless of the `pause_on_*` options, which gate the automatic
    /// hover/focus pauses (`HoverStart` / `FocusSlide`).
    AutoPlayPause,
    /// Pointer entered the carousel. Pauses auto-play only when
    /// `AutoPlayOptions::pause_on_hover` is set; otherwise a no-op.
    HoverStart,
    /// Pointer left the carousel. Resumes a hover/focus auto-play pause.
    HoverEnd,
    /// Resume auto-play after pause.
    AutoPlayResume,
    /// The CSS transition animation completed.
    TransitionEnd,
    /// Pointer down on the viewport (drag start).
    PointerDown { pos: f64, timestamp: f64 },
    /// Pointer moved during drag.
    PointerMove { pos: f64, timestamp: f64 },
    /// Pointer released (drag end).
    PointerUp,
    /// Pointer cancelled (drag abort).
    PointerCancel,
    /// Keyboard focus entered the carousel on a control (Prev/Next, indicator,
    /// auto-play trigger) rather than a slide. Pauses auto-play when
    /// `pause_on_focus` is set, without changing the index.
    FocusEnter,
    /// A slide received focus.
    FocusSlide { index: usize },
    /// Focus left the carousel.
    Blur,
    /// The parent re-rendered with new `Props` (via `set_props`). Emitted by
    /// `on_props_changed` so the machine re-derives its mutable configuration,
    /// tracks the controlled `index` signal (including controlled→uncontrolled),
    /// and reconciles the auto-play timer — all without animating.
    SyncProps { props: Props },
}
```

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_i18n::Orientation;
use core::num::NonZero;
use core::time::Duration;

/// Slide alignment within the viewport.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum SlideAlignment {
    #[default]
    Start,
    Center,
    End,
}

/// Auto-play configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoPlayOptions {
    /// Interval between automatic slide advances.
    pub interval: Duration,
    /// Whether manual interaction permanently stops auto-play.
    pub stop_on_interaction: bool,
    /// Whether keyboard focus within the carousel pauses auto-play.
    pub pause_on_focus: bool,
    /// Whether pointer hover over the carousel pauses auto-play.
    pub pause_on_hover: bool,
}

impl Default for AutoPlayOptions {
    fn default() -> Self {
        AutoPlayOptions {
            interval: Duration::from_millis(4000),
            stop_on_interaction: true,
            pause_on_focus: true,
            pause_on_hover: true,
        }
    }
}

/// Runtime context for the Carousel state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current slide index (controlled/uncontrolled).
    pub index: Bindable<usize>,
    /// Total number of slides.
    pub slide_count: NonZero<usize>,
    /// Whether navigation wraps around.
    pub loop_nav: bool,
    /// Auto-play configuration. `None` disables auto-play.
    pub auto_play: Option<AutoPlayOptions>,
    /// Whether auto-play has been permanently stopped by user interaction.
    pub auto_play_stopped: bool,
    /// Whether the auto-play trigger button manually paused rotation. Tracked
    /// separately from hover/focus so a hover/focus exit never resumes a manual
    /// pause. See `is_auto_play_paused`.
    pub auto_play_paused_manual: bool,
    /// Whether pointer hover currently pauses auto-play (`HoverStart`).
    pub auto_play_paused_hover: bool,
    /// Whether keyboard focus within the carousel pauses auto-play (`FocusSlide`).
    pub auto_play_paused_focus: bool,
    /// Gap between slides in pixels.
    pub spacing: f64,
    /// Number of slides visible at once (fractional supported).
    pub slides_per_view: f64,
    /// Number of slides to advance per navigation action. Defaults to `1`.
    /// When `slides_per_view > 1`, setting this to match `slides_per_view`
    /// provides page-by-page navigation.
    pub slides_per_move: usize,
    /// Slide alignment within the viewport.
    pub align: SlideAlignment,
    /// Slide axis.
    pub orientation: Orientation,
    /// CSS transition duration for slide animations.
    pub transition_duration: Duration,
    /// Pointer position at drag start. `None` when not dragging.
    pub drag_start_pos: Option<f64>,
    /// Accumulated drag distance in pixels.
    pub drag_delta: f64,
    /// Distance threshold (pixels) to trigger a swipe navigation.
    pub swipe_threshold: f64,
    /// Time-normalized swipe velocity (px/ms). Independent of display refresh rate.
    pub swipe_velocity: f64,
    /// Timestamp of the last PointerMove event (ms, from `performance.now()`).
    pub drag_last_timestamp: Option<f64>,
    /// Whether the carousel is right-to-left.
    pub is_rtl: bool,
    /// Resolved locale for MessageFn calls.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    pub fn current_index(&self) -> usize { *self.index.get() }

    /// Whether auto-play is temporarily paused by any source — the manual
    /// trigger pause, a pointer hover, or keyboard focus.
    pub const fn is_auto_play_paused(&self) -> bool {
        self.auto_play_paused_manual || self.auto_play_paused_hover || self.auto_play_paused_focus
    }

    /// Number of slide slots occupied at once, rounding a fractional
    /// `slides_per_view` up so a partially visible trailing slide still counts
    /// as on-screen. Always at least `1`.
    pub fn visible_count(&self) -> usize {
        (self.slides_per_view.ceil() as usize).max(1)
    }

    /// Largest valid starting index for non-looping navigation. With
    /// `slides_per_view > 1` the last page is flush to the end
    /// (contain-scroll): `slide_count - visible_count`.
    pub fn last_index(&self) -> usize {
        self.slide_count.get().saturating_sub(self.visible_count())
    }

    /// Largest reachable starting index. Under `loop_nav` any slide can lead
    /// (the window wraps), so `slide_count - 1`; otherwise the contain-scroll
    /// `last_index`. Used to clamp caller-supplied (default/controlled) indices.
    pub fn max_start_index(&self) -> usize {
        if self.loop_nav { self.slide_count.get().saturating_sub(1) } else { self.last_index() }
    }

    /// Clamp or wrap an index according to `loop_nav`.
    pub fn clamp_index(&self, i: isize) -> usize {
        let n = self.slide_count.get() as isize;
        if self.loop_nav {
            ((i % n) + n) as usize % self.slide_count.get()
        } else {
            (i.max(0) as usize).min(self.last_index())
        }
    }

    // Always `false` when `last_index() == 0` (one slide, or `slides_per_view`
    // already shows them all): no distinct target exists, even when looping.
    pub fn can_go_prev(&self) -> bool {
        self.current_index() > 0 || (self.loop_nav && self.last_index() > 0)
    }
    pub fn can_go_next(&self) -> bool {
        self.current_index() < self.last_index() || (self.loop_nav && self.last_index() > 0)
    }

    /// Whether `index` is within the visible window of `visible_count` slides
    /// starting at `current_index` (wrapping when `loop_nav` is set). Slides
    /// outside the window are hidden from assistive technology.
    pub fn is_slide_visible(&self, index: usize) -> bool {
        let current = self.current_index();
        let count = self.slide_count.get();
        (0..self.visible_count()).any(|offset| {
            let slot = if self.loop_nav { (current + offset) % count } else { current + offset };
            slot == index
        })
    }

    /// CSS translate percentage for the slide track.
    pub fn track_offset_percent(&self, viewport_width: f64) -> f64 {
        let idx = self.current_index() as f64;
        let per_slide = 100.0 / self.slides_per_view;
        let drag_correction = if viewport_width > 0.0 {
            (self.drag_delta / viewport_width) * 100.0
        } else {
            0.0
        };
        -(idx * per_slide) + drag_correction
    }
}
```

### 1.4 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Total number of slides.
    pub slide_count: NonZero<usize>,
    /// Controlled slide index.
    pub index: Option<Bindable<usize>>,
    /// Default slide index for uncontrolled usage.
    pub default_index: Option<usize>,
    /// Whether navigation wraps around.
    pub loop_nav: bool,
    /// Auto-play configuration.
    pub auto_play: Option<AutoPlayOptions>,
    /// Gap between slides in pixels.
    pub spacing: Option<f64>,
    /// Number of slides visible at once.
    pub slides_per_view: Option<f64>,
    /// Number of slides to advance per navigation action.
    pub slides_per_move: Option<usize>,
    /// Slide alignment.
    pub align: Option<SlideAlignment>,
    /// Slide axis.
    pub orientation: Option<Orientation>,
    /// CSS transition duration.
    pub transition_duration: Option<Duration>,
    /// Swipe distance threshold in pixels.
    pub swipe_threshold: Option<f64>,
    /// Whether the carousel is right-to-left.
    pub is_rtl: bool,
    /// Callback fired with the newly requested slide index whenever the machine
    /// changes the index. **Required for controlled usage** (`index` is
    /// `Some`): in controlled mode `Bindable::set` only updates the pending
    /// internal value, so the parent must update its controlled signal from
    /// this callback and push it back through `index`, otherwise the visible
    /// slide never moves.
    pub on_index_change: Option<Callback<dyn Fn(usize) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            slide_count: NonZero::new(1).expect("non-zero"),
            index: None,
            default_index: None,
            loop_nav: false,
            auto_play: None,
            spacing: None,
            slides_per_view: None,
            slides_per_move: None,
            align: None,
            orientation: None,
            transition_duration: None,
            swipe_threshold: None,
            is_rtl: false,
            on_index_change: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

Auto-play timing is an adapter concern, not an agnostic one: the core never calls
`set_interval`/`set_timeout` directly. Instead it emits a typed [`Effect`] marker that adapters
dispatch on (`match effect.name { Effect::AutoPlayTimer => … }`) — the same convention used by
`toast::single::Effect`, `dialog::Effect`, `popover::Effect`, and `tooltip::Effect`. The adapter
resolving `Effect::AutoPlayTimer` runs a recurring interval of `ctx.auto_play.interval` that
dispatches `Event::AutoPlayTick`, and tears it down when the effect is cancelled.

```rust
/// Typed identifier for every named effect intent the carousel machine emits.
///
/// Adapters dispatch on `effect.name` exhaustively so unhandled variants and
/// name typos surface at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts (or restarts) a recurring auto-play interval of
    /// `Context::auto_play.interval` that dispatches `Event::AutoPlayTick`.
    /// Emitted on mount when the carousel boots into `State::AutoPlaying`
    /// (see `initial_effects`), on `AutoPlayStart`, on `AutoPlayResume`, and
    /// on `Blur` when resuming a focus/hover pause. Cancelled on
    /// `AutoPlayStop`, `AutoPlayPause`, and `PointerDown`.
    AutoPlayTimer,
    /// Adapter invokes `Props::on_index_change` with the newly requested slide
    /// index. Emitted whenever the machine changes the index (manual nav,
    /// swipe, auto-play tick, focus scroll). This is the round-trip path for
    /// **controlled** carousels: in controlled mode `Bindable::set` only updates
    /// the pending internal value, so the parent must observe this callback and
    /// push the new value back through `Props::index`.
    IndexChange,
}

pub struct Machine;

/// Build the transition for a manual navigation to `idx`: enter
/// `Transitioning` and set the index, and — when `stop_on_interaction` is
/// configured — permanently stop auto-play and cancel its timer. The
/// cancellation is essential: without it the adapter's recurring interval
/// keeps running after rotation has "stopped", leaking the timer and
/// dispatching ignored `AutoPlayTick`s.
///
/// Returns `None` when `idx` equals the current index: the transform would not
/// change, so the adapter has no CSS transition to report and `TransitionEnd`
/// may never arrive — entering `Transitioning` would strand the machine.
fn navigate_to(ctx: &Context, idx: usize) -> Option<TransitionPlan<Machine>> {
    if idx == ctx.current_index() { return None; }
    let stop = ctx.auto_play.as_ref().is_some_and(|o| o.stop_on_interaction);
    let mut plan = TransitionPlan::to(State::Transitioning)
        .apply(move |ctx| {
            ctx.index.set(idx);
            if stop { ctx.auto_play_stopped = true; }
        })
        .with_effect(index_change_effect(idx));
    if stop { plan = plan.cancel_effect(Effect::AutoPlayTimer); }
    Some(settle_if_instant(ctx, plan))
}

/// A zero-length transition fires no `transitionend`, so the adapter never
/// settles `Transitioning`. Self-dispatch `TransitionEnd` so the machine
/// settles synchronously rather than stranding in `Transitioning`.
fn settle_if_instant(ctx: &Context, plan: TransitionPlan<Machine>) -> TransitionPlan<Machine> {
    if ctx.transition_duration.is_zero() { plan.then(Event::TransitionEnd) } else { plan }
}

/// Build the `Effect::IndexChange` notification carrying the newly requested
/// slide `index`; the adapter resolves it by invoking `Props::on_index_change`.
fn index_change_effect(index: usize) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::IndexChange, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_index_change {
            callback(index);
        }
        no_cleanup()
    })
}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let initial_state = if props.auto_play.is_some() {
            State::AutoPlaying
        } else {
            State::Idle
        };
        // Normalize `slides_per_view`: a non-finite or non-positive value would
        // make `track_offset_percent` divide by zero/NaN, so fall back to one.
        let slides_per_view = props.slides_per_view
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.0);
        // `slides_per_move` of zero makes every navigation a no-op, so clamp to one.
        let slides_per_move = props.slides_per_move.unwrap_or(1).max(1);
        // Clamp the default/controlled index to the largest reachable start so
        // `current_index < slide_count` holds from the first render — loop-aware
        // (`slide_count - 1` under loop, else the contain-scroll last page).
        let visible_count = (slides_per_view.ceil() as usize).max(1);
        let max_index = if props.loop_nav {
            props.slide_count.get().saturating_sub(1)
        } else {
            props.slide_count.get().saturating_sub(visible_count)
        };
        let initial_index = props.default_index.unwrap_or(0).min(max_index);
        let locale = env.locale.clone();
        let messages = messages.clone();
        // Clamp the controlled value too: a caller-supplied controlled `index`
        // past `max_index` would start the machine out of range.
        let index = match &props.index {
            Some(controlled) => Bindable::controlled((*controlled.get()).min(max_index)),
            None => Bindable::uncontrolled(initial_index),
        };
        let ctx = Context {
            index,
            slide_count: props.slide_count,
            loop_nav: props.loop_nav,
            auto_play: props.auto_play.clone(),
            auto_play_stopped: false,
            auto_play_paused_manual: false,
            auto_play_paused_hover: false,
            auto_play_paused_focus: false,
            spacing: props.spacing.unwrap_or(0.0),
            slides_per_view,
            slides_per_move,
            align: props.align.unwrap_or_default(),
            orientation: props.orientation.unwrap_or_default(),
            is_rtl: props.is_rtl,
            transition_duration: props.transition_duration
                .unwrap_or_else(|| Duration::from_millis(300)),
            drag_start_pos: None,
            drag_delta: 0.0,
            swipe_threshold: props.swipe_threshold.unwrap_or(50.0),
            swipe_velocity: 0.0,
            drag_last_timestamp: None,
            locale,
            messages,
            ids: ComponentIds::from_id(&props.id),
        };
        (initial_state, ctx)
    }

    fn initial_effects(
        state: &Self::State,
        _ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // `init` boots into `AutoPlaying` when `auto_play.is_some()`, but no
        // `AutoPlayStart` event fires on mount — so the recurring auto-play
        // interval must be armed here. Adapters drain this via
        // `Service::take_initial_effects()` on first mount.
        if *state == State::AutoPlaying {
            vec![PendingEffect::named(Effect::AutoPlayTimer)]
        } else {
            Vec::new()
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(old.id, new.id, "carousel::Props.id must remain stable after initialization");
        // Any prop change re-syncs mutable configuration, the controlled-index
        // signal, and the auto-play timer (see the `SyncProps` arm). Mirrors
        // the `splitter` convention.
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps { props: new.clone() }]
        }
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::GoToSlide { index } => {
                if *state == State::Transitioning { return None; }
                // Clamp via `clamp_index`: wraps under `loop_nav` (any slide
                // selectable, including past `last_index`), saturates at
                // `last_index` otherwise. A direct jump is a manual interaction,
                // so it honours `stop_on_interaction` like Next/Prev.
                let idx = ctx.clamp_index(*index as isize);
                navigate_to(ctx, idx)
            }

            Event::GoToNext => {
                if *state == State::Transitioning || !ctx.can_go_next() { return None; }
                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);
                navigate_to(ctx, next)
            }

            Event::GoToPrev => {
                if *state == State::Transitioning || !ctx.can_go_prev() { return None; }
                let step = ctx.slides_per_move as isize;
                let prev = ctx.clamp_index(ctx.current_index() as isize - step);
                navigate_to(ctx, prev)
            }

            Event::TransitionEnd => {
                if ctx.auto_play.is_some() && !ctx.auto_play_stopped && !ctx.is_auto_play_paused() {
                    // Always re-arm the timer on entering AutoPlaying: a swipe
                    // cancelled it on PointerDown and kept it cancelled through
                    // the snap, so settling here must restart it (otherwise the
                    // controls report "playing" with no interval running).
                    Some(TransitionPlan::to(State::AutoPlaying)
                        .with_effect(PendingEffect::named(Effect::AutoPlayTimer)))
                } else {
                    Some(TransitionPlan::to(State::Idle))
                }
            }

            Event::AutoPlayStart => {
                if ctx.auto_play_stopped || ctx.auto_play.is_none() { return None; }
                // Clear any prior pause: starting auto-play means it is now
                // actively rotating, so the paused live-region mode and
                // `aria-pressed="false"` must not linger.
                Some(TransitionPlan::to(State::AutoPlaying)
                    .apply(|ctx| {
                        ctx.auto_play_paused_manual = false;
                        ctx.auto_play_paused_hover = false;
                        ctx.auto_play_paused_focus = false;
                    })
                    .with_effect(PendingEffect::named(Effect::AutoPlayTimer)))
            }

            Event::AutoPlayStop => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.auto_play_stopped = true;
                }).cancel_effect(Effect::AutoPlayTimer))
            }

            Event::AutoPlayTick => {
                // Drop ticks during a drag: `PointerDown` cancels the timer, but
                // a callback queued just before could still fire and advance the
                // slide under the pointer.
                if *state != State::AutoPlaying || ctx.drag_start_pos.is_some() { return None; }
                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);
                // Ignore ticks that would not move the track: the non-looping
                // boundary and the looped no-op case (single slide, or
                // `slides_per_move` a multiple of `slide_count`). Entering
                // `Transitioning` with no transform change risks a missing
                // `transitionend` that strands the machine.
                if next == ctx.current_index() { return None; }
                Some(settle_if_instant(ctx, TransitionPlan::to(State::Transitioning)
                    .apply(move |ctx| { ctx.index.set(next); })
                    .with_effect(index_change_effect(next))))
            }

            Event::AutoPlayPause => {
                // Nothing to pause without auto-play configured. Guarding here
                // (not just in the trigger handler) keeps a stray `paused` flag
                // from being set while `auto_play` is `None`, which would
                // suppress the timer if the parent later enables autoplay.
                ctx.auto_play.as_ref()?;
                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                Some(plan.apply(|ctx| { ctx.auto_play_paused_manual = true; }).cancel_effect(Effect::AutoPlayTimer))
            }

            Event::HoverStart => {
                // Hover-pause is opt-in via `pause_on_hover`; otherwise hovering
                // does nothing. When enabled it pauses like `AutoPlayPause`.
                if !ctx.auto_play.as_ref().is_some_and(|o| o.pause_on_hover) { return None; }
                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                Some(plan.apply(|ctx| { ctx.auto_play_paused_hover = true; }).cancel_effect(Effect::AutoPlayTimer))
            }

            Event::HoverEnd => {
                // Clear only the hover pause; resume only if no other source
                // (manual trigger or focus) still holds it.
                if !ctx.auto_play_paused_hover { return None; }
                let resume = !ctx.auto_play_paused_manual
                    && !ctx.auto_play_paused_focus
                    && !ctx.auto_play_stopped
                    && ctx.auto_play.is_some();
                let mut plan = if resume {
                    TransitionPlan::to(State::AutoPlaying)
                } else {
                    TransitionPlan::new()
                }.apply(|ctx| { ctx.auto_play_paused_hover = false; });
                if resume { plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer)); }
                Some(plan)
            }

            Event::AutoPlayResume => {
                // Nothing to resume without auto-play configured (the pause and
                // stopped flags are only ever set while it is configured).
                ctx.auto_play.as_ref()?;
                // Resume is also the "restart" path the auto-play trigger
                // dispatches when rotation was stopped; it clears every pause
                // source AND the stopped flag — an explicit play request
                // overrides hover/focus/manual pauses.
                Some(TransitionPlan::to(State::AutoPlaying).apply(|ctx| {
                    ctx.auto_play_paused_manual = false;
                    ctx.auto_play_paused_hover = false;
                    ctx.auto_play_paused_focus = false;
                    ctx.auto_play_stopped = false;
                }).with_effect(PendingEffect::named(Effect::AutoPlayTimer)))
            }

            Event::PointerDown { pos, timestamp } => {
                // Don't start a drag mid-snap (a slide animation is running);
                // accepting one would let rapid pointer input skip slides, the
                // same way button/keyboard nav is blocked during `Transitioning`.
                if *state == State::Transitioning { return None; }
                let p = *pos;
                let ts = *timestamp;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.drag_start_pos = Some(p);
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = Some(ts);
                }).cancel_effect(Effect::AutoPlayTimer))
            }

            Event::PointerMove { pos, timestamp } => {
                if ctx.drag_start_pos.is_none() { return None; }
                let p = *pos;
                let ts = *timestamp;
                let start = ctx.drag_start_pos?;
                let prev_delta = ctx.drag_delta;
                let prev_ts = ctx.drag_last_timestamp;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.drag_delta = p - start;
                    let pixel_delta = ctx.drag_delta - prev_delta;
                    let dt = prev_ts.map_or(0.0, |t| ts - t);
                    ctx.swipe_velocity = if dt > 0.0 { pixel_delta / dt } else { 0.0 };
                    ctx.drag_last_timestamp = Some(ts);
                }))
            }

            Event::PointerUp => {
                // No-op without an active drag: a stray pointer-up must not run
                // the resume logic (which would re-arm a timer that was never
                // cancelled, creating duplicate intervals).
                ctx.drag_start_pos?;

                // Never navigate mid-snap: a release while a slide animation is
                // running just clears the drag (button/keyboard nav is blocked
                // the same way); the in-flight transition settles on its own.
                if *state == State::Transitioning {
                    return Some(TransitionPlan::context_only(|ctx| {
                        ctx.drag_start_pos = None;
                        ctx.drag_delta = 0.0;
                        ctx.swipe_velocity = 0.0;
                        ctx.drag_last_timestamp = None;
                    }));
                }

                let delta = ctx.drag_delta;
                let velocity = ctx.swipe_velocity;
                let threshold = ctx.swipe_threshold;
                // A brisk flick (>0.5 px/ms) reduces the distance threshold.
                let effective = if velocity.abs() > 0.5 { threshold * 0.3 } else { threshold };

                let cur = ctx.current_index() as isize;
                // A swipe advances by `slides_per_move`, matching button/keyboard nav.
                let step = ctx.slides_per_move as isize;
                let next_idx = if delta < -effective && ctx.can_go_next() {
                    Some(ctx.clamp_index(cur + step))
                } else if delta > effective && ctx.can_go_prev() {
                    Some(ctx.clamp_index(cur - step))
                } else {
                    None
                };

                // `PointerDown` cancelled the timer for the drag.
                let target_idx = next_idx.filter(|&idx| idx != ctx.current_index());
                if let Some(idx) = target_idx {
                    // A navigating swipe animates through `Transitioning`, like
                    // button/auto-play nav, so ticks and further navigation are
                    // blocked until `TransitionEnd` (which then resumes auto-play
                    // or stays `Idle` if `stop_on_interaction` stopped it).
                    let stop = ctx.auto_play.as_ref().is_some_and(|o| o.stop_on_interaction);
                    let plan = TransitionPlan::to(State::Transitioning)
                        .apply(move |ctx| {
                            ctx.drag_start_pos = None;
                            ctx.drag_delta = 0.0;
                            ctx.swipe_velocity = 0.0;
                            ctx.drag_last_timestamp = None;
                            ctx.index.set(idx);
                            if stop { ctx.auto_play_stopped = true; }
                        })
                        .with_effect(index_change_effect(idx));
                    return Some(settle_if_instant(ctx, plan));
                }

                // No navigation: reset the drag and resume auto-play if it was
                // active (the timer was cancelled on `PointerDown`).
                let resume = ctx.auto_play.is_some()
                    && !ctx.auto_play_stopped
                    && !ctx.is_auto_play_paused();
                let mut plan = if resume {
                    TransitionPlan::to(State::AutoPlaying)
                } else {
                    TransitionPlan::to(State::Idle)
                }.apply(|ctx| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;
                });
                if resume { plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer)); }
                Some(plan)
            }

            Event::PointerCancel => {
                // The drag is aborted (touch scroll / pointer-capture loss).
                // `PointerDown` cancelled the timer, so re-arm it if the gesture
                // interrupted an active auto-play carousel — otherwise rotation
                // silently dies while the state still reads `AutoPlaying`.
                let resume = ctx.drag_start_pos.is_some()
                    && ctx.auto_play.is_some()
                    && !ctx.auto_play_stopped
                    && !ctx.is_auto_play_paused();
                let mut plan = if resume {
                    TransitionPlan::to(State::AutoPlaying)
                } else {
                    TransitionPlan::new()
                }.apply(|ctx| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;
                });
                if resume { plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer)); }
                Some(plan)
            }

            Event::FocusEnter => {
                // Focus entered a control (not a slide). Pause when
                // `pause_on_focus` is set, without moving the index; `Blur`
                // resumes on focus-out.
                if !ctx.auto_play.as_ref().is_some_and(|o| o.pause_on_focus) { return None; }
                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                Some(plan.apply(|ctx| { ctx.auto_play_paused_focus = true; }).cancel_effect(Effect::AutoPlayTimer))
            }

            Event::FocusSlide { index } => {
                // Only scroll when the focused slide is not already on-screen.
                // With `slides_per_view > 1`, tabbing into a visible non-leading
                // slide must NOT shift the track under the user.
                let scroll = !ctx.is_slide_visible(*index);
                // Clamp via `clamp_index` (wraps under `loop_nav`, saturates at
                // `last_index` otherwise), matching `GoToSlide`.
                let idx = ctx.clamp_index(*index as isize);
                let should_pause = ctx.auto_play.as_ref().is_some_and(|o| o.pause_on_focus);
                if should_pause {
                    // Pausing on focus mirrors `AutoPlayPause`: leave
                    // `AutoPlaying` and cancel the timer so slides do not keep
                    // advancing under keyboard focus.
                    let plan = if *state == State::AutoPlaying {
                        TransitionPlan::to(State::Idle)
                    } else {
                        TransitionPlan::new()
                    };
                    let mut plan = plan.apply(move |ctx| {
                        if scroll { ctx.index.set(idx); }
                        ctx.auto_play_paused_focus = true;
                    }).cancel_effect(Effect::AutoPlayTimer);
                    if scroll { plan = plan.with_effect(index_change_effect(idx)); }
                    return Some(plan);
                }
                if !scroll { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.index.set(idx);
                }).with_effect(index_change_effect(idx)))
            }

            Event::Blur => {
                // Clear only the focus pause; resume only if no other source
                // (manual trigger or hover) still holds it.
                if !ctx.auto_play_paused_focus { return None; }
                let resume = !ctx.auto_play_paused_manual
                    && !ctx.auto_play_paused_hover
                    && !ctx.auto_play_stopped
                    && ctx.auto_play.is_some();
                let mut plan = if resume {
                    TransitionPlan::to(State::AutoPlaying)
                } else {
                    TransitionPlan::new()
                }.apply(|ctx| { ctx.auto_play_paused_focus = false; });
                if resume { plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer)); }
                Some(plan)
            }

            Event::SyncProps { props } => {
                // Re-derive mutable configuration from the new props, track the
                // controlled-index signal (including controlled→uncontrolled),
                // and reconcile the auto-play timer — all without animating.
                let new_auto = props.auto_play.clone();
                let auto_changed = ctx.auto_play != new_auto;
                let want_timer = new_auto.is_some() && !ctx.auto_play_stopped && !ctx.is_auto_play_paused();
                // Only the auto-play transition moves the resting state.
                let target = if auto_changed {
                    if want_timer { State::AutoPlaying }
                    else if *state == State::AutoPlaying { State::Idle }
                    else { *state }
                } else {
                    *state
                };
                let props = props.clone();
                let mut plan = TransitionPlan::to(target).apply(move |ctx| {
                    let slides_per_view = props.slides_per_view
                        .filter(|value| value.is_finite() && *value > 0.0)
                        .unwrap_or(1.0);
                    ctx.slide_count = props.slide_count;
                    ctx.loop_nav = props.loop_nav;
                    ctx.auto_play = props.auto_play.clone();
                    ctx.spacing = props.spacing.unwrap_or(0.0);
                    ctx.slides_per_view = slides_per_view;
                    ctx.slides_per_move = props.slides_per_move.unwrap_or(1).max(1);
                    ctx.align = props.align.unwrap_or_default();
                    ctx.orientation = props.orientation.unwrap_or_default();
                    ctx.is_rtl = props.is_rtl;
                    ctx.transition_duration = props.transition_duration
                        .unwrap_or_else(|| Duration::from_millis(300));
                    ctx.swipe_threshold = props.swipe_threshold.unwrap_or(50.0);
                    // Track the controlled signal (clamped to the largest
                    // reachable start, loop-aware); `None` returns to uncontrolled.
                    let controlled = props.index.as_ref()
                        .map(|bindable| (*bindable.get()).min(ctx.max_start_index()));
                    ctx.index.sync_controlled(controlled);
                    if !ctx.index.is_controlled() {
                        let clamped = ctx.current_index().min(ctx.max_start_index());
                        ctx.index.set(clamped);
                    }
                    if ctx.auto_play.is_none() {
                        ctx.auto_play_paused_manual = false;
                        ctx.auto_play_paused_hover = false;
                        ctx.auto_play_paused_focus = false;
                        ctx.auto_play_stopped = false;
                    }
                });
                if auto_changed {
                    plan = plan.cancel_effect(Effect::AutoPlayTimer);
                    if want_timer { plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer)); }
                }
                Some(plan)
            }
        }
    }

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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "carousel"]
pub enum Part {
    Root,
    Viewport,
    ItemGroup,
    Item { index: usize },
    PrevTrigger,
    NextTrigger,
    IndicatorGroup,
    Indicator { index: usize },
    AutoPlayTrigger,
    AutoPlayIndicator,
    ProgressText,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "region");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.carousel_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.role_description)(&self.ctx.locale));
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::AutoPlaying => "auto-playing",
            State::Transitioning => "transitioning",
        });
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        attrs
    }

    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let touch_action = if self.ctx.orientation == Orientation::Horizontal {
            "pan-y"
        } else {
            "pan-x"
        };
        attrs.set_style(CssProperty::Overflow, "hidden");
        attrs.set_style(CssProperty::TouchAction, touch_action);
        attrs
    }

    pub fn item_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // "off" only while rotation is actively advancing; absent, stopped, or
        // paused (hover/focus) carousels announce manual changes politely.
        let live = if self.ctx.auto_play.is_none()
            || self.ctx.auto_play_stopped
            || self.ctx.is_auto_play_paused()
        {
            "polite"
        } else {
            "off"
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), live);
        attrs
    }

    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_current = index == self.ctx.current_index();
        // With `slides_per_view > 1` several slides are on-screen; only slides
        // outside the visible window are hidden/inert. Marking a visible slide
        // hidden would make on-screen content unreachable to AT and keyboard.
        let is_hidden = !self.ctx.is_slide_visible(index);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.slide_role_description)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.slide_label)(index + 1, self.ctx.slide_count.get(), &self.ctx.locale));
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set_bool(HtmlAttr::Data("ars-active"), is_current);
        if is_hidden {
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            attrs.set_bool(HtmlAttr::Inert, true);
        }
        attrs
    }

    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.prev_label)(&self.ctx.locale));
        if !self.ctx.can_go_prev() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.next_label)(&self.ctx.locale));
        if !self.ctx.can_go_next() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    pub fn indicator_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::IndicatorGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "tablist");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.indicators_label)(&self.ctx.locale));
        attrs
    }

    pub fn indicator_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Indicator { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Role, "tab");
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected),
            if index == self.ctx.current_index() { "true" } else { "false" },
        );
        attrs
    }

    pub fn auto_play_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        let is_playing = self.ctx.auto_play.is_some()
            && !self.ctx.auto_play_stopped
            && !self.ctx.is_auto_play_paused();
        let label = if is_playing {
            (self.ctx.messages.pause_auto_play_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.start_auto_play_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if is_playing { "true" } else { "false" });
        attrs
    }

    pub fn auto_play_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_playing = self.ctx.auto_play.is_some()
            && !self.ctx.auto_play_stopped
            && !self.ctx.is_auto_play_paused();
        attrs.set(HtmlAttr::Data("ars-state"), if is_playing { "playing" } else { "paused" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the progress text element (e.g., "2 of 5").
    pub fn progress_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ProgressText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    /// Human-readable progress string (e.g., "Slide 2 of 5").
    pub fn progress_text(&self) -> String {
        (self.ctx.messages.slide_label)(
            self.ctx.current_index() + 1,
            self.ctx.slide_count.get(),
            &self.ctx.locale,
        )
    }

    /// **Adapter contract:** the agnostic core cannot inspect the DOM event
    /// target, so the adapter MUST only forward keydowns the carousel owns —
    /// those targeting the carousel root/controls, not events bubbled from
    /// interactive slide content (text inputs, sliders, nested widgets).
    /// Otherwise `ArrowLeft`/`ArrowRight` typed into a slide's `<input>` would
    /// be hijacked into slide navigation. Gate on the event target first.
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let (prev_key, next_key) = if is_horizontal {
            if self.ctx.is_rtl {
                (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
            } else {
                (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
            }
        } else {
            (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown)
        };
        match data.key {
            k if k == prev_key => (self.send)(Event::GoToPrev),
            k if k == next_key => (self.send)(Event::GoToNext),
            KeyboardKey::Home => (self.send)(Event::GoToSlide { index: 0 }),
            KeyboardKey::End => (self.send)(Event::GoToSlide {
                index: self.ctx.max_start_index(),
            }),
            _ => {}
        }
    }

    pub fn on_prev_trigger_click(&self) { (self.send)(Event::GoToPrev); }
    pub fn on_next_trigger_click(&self) { (self.send)(Event::GoToNext); }
    pub fn on_indicator_click(&self, index: usize) { (self.send)(Event::GoToSlide { index }); }
    pub fn on_auto_play_trigger_click(&self) {
        // No-op without auto-play configured: there is nothing to toggle, and
        // dispatching `AutoPlayPause` would set a stale `paused` flag.
        if self.ctx.auto_play.is_none() { return; }
        if self.ctx.auto_play_stopped || self.ctx.is_auto_play_paused() {
            (self.send)(Event::AutoPlayResume);
        } else {
            (self.send)(Event::AutoPlayPause);
        }
    }
    /// Pointer entered the carousel; auto-play pauses only when `pause_on_hover`.
    pub fn on_root_pointer_enter(&self) { (self.send)(Event::HoverStart); }
    /// Pointer left the carousel; resumes a hover pause.
    pub fn on_root_pointer_leave(&self) { (self.send)(Event::HoverEnd); }
    /// Focus entered the carousel on a control; pauses when `pause_on_focus`.
    pub fn on_root_focus_in(&self) { (self.send)(Event::FocusEnter); }
    /// Focus left the carousel; resumes a focus pause.
    pub fn on_root_focus_out(&self) { (self.send)(Event::Blur); }
    pub fn on_viewport_pointerdown(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerDown { pos, timestamp });
    }
    pub fn on_viewport_pointermove(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerMove { pos, timestamp });
    }
    /// Gesture aborted by the browser (pointer-capture loss / touch-scroll).
    pub fn on_viewport_pointercancel(&self) { (self.send)(Event::PointerCancel); }
    pub fn on_viewport_pointerup(&self) { (self.send)(Event::PointerUp); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Viewport => self.viewport_attrs(),
            Part::ItemGroup => self.item_group_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::IndicatorGroup => self.indicator_group_attrs(),
            Part::Indicator { index } => self.indicator_attrs(index),
            Part::AutoPlayTrigger => self.auto_play_trigger_attrs(),
            Part::AutoPlayIndicator => self.auto_play_indicator_attrs(),
            Part::ProgressText => self.progress_text_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Carousel
├── Root              <section>  role="region" aria-roledescription="carousel"
│   ├── PrevTrigger   <button>   "Previous slide"
│   ├── Viewport      <div>      overflow:hidden, touch-action
│   │   └── ItemGroup <div>      aria-live="off|polite"
│   │       └── Item  (×N) <div> role="group" aria-roledescription="slide"
│   ├── NextTrigger   <button>   "Next slide"
│   ├── IndicatorGroup <div>     role="tablist"
│   │   └── Indicator (×N) <button> role="tab" aria-selected
│   ├── AutoPlayTrigger <button> aria-pressed
│   ├── AutoPlayIndicator <div> aria-hidden="true" data-ars-state
│   └── ProgressText <div>  aria-live="polite" aria-atomic="true"
```

| Part              | Element     | Key Attributes                                          |
| ----------------- | ----------- | ------------------------------------------------------- |
| Root              | `<section>` | `role="region"`, `aria-roledescription="carousel"`      |
| Viewport          | `<div>`     | `overflow:hidden`, `touch-action`                       |
| ItemGroup         | `<div>`     | `aria-live="off\|polite"`                               |
| Item              | `<div>`     | `role="group"`, `aria-roledescription="slide"`, `inert` |
| PrevTrigger       | `<button>`  | `type="button"`, `aria-disabled` when at boundary       |
| NextTrigger       | `<button>`  | `type="button"`, `aria-disabled` when at boundary       |
| IndicatorGroup    | `<div>`     | `role="tablist"`                                        |
| Indicator         | `<button>`  | `type="button"`, `role="tab"`, `aria-selected`          |
| AutoPlayTrigger   | `<button>`  | `type="button"`, `aria-pressed`                         |
| AutoPlayIndicator | `<div>`     | `aria-hidden="true"`, `data-ars-state`                  |
| ProgressText      | `<div>`     | `aria-live="polite"`, `aria-atomic="true"`              |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

The carousel follows the [WAI-ARIA Carousel Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/carousel/).

| Element           | Attribute              | Value                                            |
| ----------------- | ---------------------- | ------------------------------------------------ |
| Root (`section`)  | `role`                 | `"region"`                                       |
| Root              | `aria-roledescription` | `"carousel"`                                     |
| Item              | `role`                 | `"group"`                                        |
| Item              | `aria-roledescription` | `"slide"`                                        |
| Item (hidden)     | `aria-hidden`          | `"true"` + `inert`                               |
| IndicatorGroup    | `role`                 | `"tablist"`                                      |
| Indicator         | `role`                 | `"tab"`                                          |
| Indicator         | `aria-selected`        | `"true"` for current slide                       |
| ItemGroup         | `aria-live`            | `"off"` during auto-play, `"polite"` when paused |
| AutoPlayIndicator | `aria-hidden`          | `"true"` (purely decorative visual feedback)     |

### 3.2 Keyboard Interaction

| Key                           | Behaviour                     |
| ----------------------------- | ----------------------------- |
| `ArrowRight` (horizontal LTR) | Next slide                    |
| `ArrowLeft` (horizontal LTR)  | Previous slide                |
| `ArrowDown` (vertical)        | Next slide                    |
| `ArrowUp` (vertical)          | Previous slide                |
| `Home`                        | First slide                   |
| `End`                         | Last slide                    |
| `Tab`                         | Move focus into slide content |

RTL: Arrow keys reverse per `03-accessibility.md` §4.1.

Arrow-key navigation applies only to keydowns the carousel owns. Because the agnostic core cannot inspect the DOM event target, the adapter MUST gate `on_root_keydown` on the event target and not forward arrow keys bubbled from interactive slide content (text inputs, sliders, nested widgets), so typing inside a slide operates that control rather than navigating the carousel.

### 3.3 Screen Reader Announcements

- `aria-live` on `ItemGroup` is `"off"` during auto-play to prevent disruptive announcements, and `"polite"` when paused or stopped.
- Auto-play pauses on hover (`mouseenter` → `HoverStart`) when `AutoPlayOptions::pause_on_hover` is set, and on focus within the carousel when `pause_on_focus` is set — `focusin` on a slide → `FocusSlide`, on a control (Prev/Next, indicators, auto-play trigger) → `FocusEnter`. Each gate is enforced in the core, not the adapter. Rotation resumes on `mouseleave` / `focusout` (`HoverEnd` / `Blur`) unless permanently stopped. The auto-play trigger button's manual pause (`AutoPlayPause`) is unconditional and independent of these options.
- Slides **outside the visible window** (`current_index` through `current_index + ceil(slides_per_view)`, wrapping when `loop_nav` is set) receive both `aria-hidden="true"` and `inert`, ensuring off-screen slides are invisible to assistive technology. With `slides_per_view > 1` every on-screen slide stays accessible — only the leading slide carries `data-ars-active`.

## 4. Internationalization

### 4.1 Messages

```rust
/// Closure type for the slide label message (factored into a type alias to
/// satisfy the workspace `clippy::type_complexity` lint).
pub type SlideLabelFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    pub carousel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub slide_role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub slide_label: MessageFn<SlideLabelFn>,
    /// Accessible name for the indicator `tablist` (`aria-label` on `IndicatorGroup`).
    pub indicators_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub prev_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub next_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub pause_auto_play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub start_auto_play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            carousel_label: MessageFn::static_str("Carousel"),
            role_description: MessageFn::static_str("carousel"),
            slide_role_description: MessageFn::static_str("slide"),
            slide_label: MessageFn::new(|index, total, _locale| format!("Slide {index} of {total}")),
            indicators_label: MessageFn::static_str("Choose slide"),
            prev_label: MessageFn::static_str("Previous slide"),
            next_label: MessageFn::static_str("Next slide"),
            pause_auto_play_label: MessageFn::static_str("Pause automatic slide show"),
            start_auto_play_label: MessageFn::static_str("Start automatic slide show"),
        }
    }
}

impl ComponentMessages for Messages {}
```

RTL: `on_root_keydown()` reverses `ArrowRight`/`ArrowLeft` when `ctx.is_rtl` is `true`. `PrevTrigger` and `NextTrigger` icons swap visually but semantic labels remain "Previous" / "Next". The slide track is ordered left-to-right in the DOM; CSS `direction: rtl` handles visual reversal.

## 5. Library Parity

> Compared against: Ark UI (`Carousel`).

### 5.1 Props

| Feature             | ars-ui                        | Ark UI                           | Notes                                             |
| ------------------- | ----------------------------- | -------------------------------- | ------------------------------------------------- |
| Current slide       | `index` (Bindable)            | `page` / `defaultPage`           | Same concept                                      |
| Slide count         | `slide_count`                 | `slideCount`                     | Same                                              |
| Loop                | `loop_nav`                    | `loop`                           | Same                                              |
| Auto-play           | `auto_play` (AutoPlayOptions) | `autoplay` (boolean or {delay})  | ars-ui richer config                              |
| Slides per view     | `slides_per_view`             | `slidesPerPage`                  | Same concept                                      |
| Slides per move     | `slides_per_move`             | `slidesPerMove`                  | Adopted from Ark UI                               |
| Spacing             | `spacing`                     | `spacing`                        | Same                                              |
| Orientation         | `orientation`                 | `orientation`                    | Same                                              |
| Alignment           | `align` (SlideAlignment)      | Item-level `snapAlign`           | ars-ui at root, Ark per-item                      |
| Swipe threshold     | `swipe_threshold`             | --                               | ars-ui addition                                   |
| Transition duration | `transition_duration`         | --                               | ars-ui addition                                   |
| RTL                 | `is_rtl`                      | --                               | ars-ui addition                                   |
| Mouse drag          | Always enabled                | `allowMouseDrag`                 | ars-ui always supports drag                       |
| In-view threshold   | --                            | `inViewThreshold`                | Partial visibility detection; not adopted (niche) |
| Snap type           | --                            | `snapType` (proximity/mandatory) | CSS scroll-snap concern; not adopted              |
| Padding             | --                            | `padding`                        | CSS concern; consumer can set padding directly    |

**Gaps:** None critical. `inViewThreshold`, `snapType`, and `padding` are CSS-level concerns or niche features not adopted.

### 5.2 Anatomy

| Part              | ars-ui              | Ark UI              | Notes                                          |
| ----------------- | ------------------- | ------------------- | ---------------------------------------------- |
| Root              | `Root`              | `Root`              | --                                             |
| Viewport          | `Viewport`          | --                  | ars-ui separates viewport from root            |
| ItemGroup         | `ItemGroup`         | `ItemGroup`         | --                                             |
| Item              | `Item`              | `Item`              | --                                             |
| PrevTrigger       | `PrevTrigger`       | `PrevTrigger`       | --                                             |
| NextTrigger       | `NextTrigger`       | `NextTrigger`       | --                                             |
| IndicatorGroup    | `IndicatorGroup`    | `IndicatorGroup`    | --                                             |
| Indicator         | `Indicator`         | `Indicator`         | --                                             |
| AutoPlayTrigger   | `AutoPlayTrigger`   | `AutoplayTrigger`   | --                                             |
| AutoPlayIndicator | `AutoPlayIndicator` | `AutoplayIndicator` | --                                             |
| ProgressText      | `ProgressText`      | `ProgressText`      | Adopted from Ark UI                            |
| Control           | --                  | `Control`           | Wrapper for prev/next; consumer layout concern |

**Gaps:** None. `Control` is a grouping wrapper the consumer can create.

### 5.3 Events

| Callback        | ars-ui                    | Ark UI                   | Notes                             |
| --------------- | ------------------------- | ------------------------ | --------------------------------- |
| Page change     | `Bindable` change         | `onPageChange`           | Handled via Bindable notification |
| Autoplay status | State machine transitions | `onAutoplayStatusChange` | Observable via state              |
| Drag status     | State machine transitions | `onDragStatusChange`     | Observable via state              |

**Gaps:** None. ars-ui uses state machine transitions instead of explicit callbacks.

### 5.4 Features

| Feature                     | ars-ui | Ark UI |
| --------------------------- | ------ | ------ |
| Auto-play with pause/resume | Yes    | Yes    |
| Looping                     | Yes    | Yes    |
| Pointer/touch drag          | Yes    | Yes    |
| Keyboard navigation         | Yes    | Yes    |
| Dot indicators              | Yes    | Yes    |
| Multi-slide view            | Yes    | Yes    |
| Multi-slide advance         | Yes    | Yes    |
| Progress text               | Yes    | Yes    |
| RTL support                 | Yes    | --     |
| Momentum-based swipe        | Yes    | --     |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses `AutoPlayOptions` struct instead of Ark's boolean-or-object; ars-ui includes momentum-based swipe detection; ars-ui has explicit `Viewport` part that Ark folds into Root.
- **Recommended additions:** None.
