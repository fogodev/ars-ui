//! Carousel component machine.
//!
//! `Carousel` presents a sequence of slides with previous/next buttons, dot
//! indicators, keyboard arrow keys, touch/pointer swipe with momentum, and
//! optional auto-play. It supports looping navigation, fractional
//! slides-per-view, configurable alignment, and full WAI-ARIA carousel
//! pattern compliance.
//!
//! The agnostic core owns the slide index, loop / transition / autoplay
//! state, swipe-threshold math computed from adapter-supplied pointer deltas,
//! and the ARIA / data attribute output for every anatomy part. It never
//! queries DOM nodes by id, never reads viewport geometry, never moves focus,
//! and never installs timers directly. Framework adapters own the live
//! viewport/track/slide handles, the actual scrolling/transform application,
//! focus movement, hover/focus listeners, geometry reads, and the auto-play
//! interval — the latter surfaced as the typed [`Effect`] enum so adapters
//! can translate it into `set_interval`/`set_timeout` (see
//! `spec/components/layout/carousel.md` §1.5).

use alloc::{
    format,
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    num::NonZero,
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, CssProperty, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan,
};
use ars_i18n::Orientation;
use ars_interactions::KeyboardEventData;

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the carousel machine
/// emits.
///
/// Adapters dispatch on `effect.name` exhaustively (`match effect.name {
/// carousel::Effect::AutoPlayTimer => … }`) so name typos and unhandled
/// variants surface at compile time — the same convention used by
/// [`toast::single::Effect`](crate::overlay::toast::single::Effect) and the
/// overlay machines. The variant name itself is the contract; there is no
/// parallel kebab-case wire form to keep in sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts (or restarts) a recurring auto-play interval of
    /// [`Context::auto_play`]'s `interval` that dispatches
    /// [`Event::AutoPlayTick`] on each fire. Emitted on mount when the
    /// carousel boots into [`State::AutoPlaying`] (see
    /// [`Machine::initial_effects`](ars_core::Machine::initial_effects)), on
    /// [`Event::AutoPlayStart`], on [`Event::AutoPlayResume`], and on
    /// [`Event::Blur`] when resuming a focus/hover pause. Cancelled on
    /// [`Event::AutoPlayStop`], [`Event::AutoPlayPause`], and
    /// [`Event::PointerDown`].
    AutoPlayTimer,

    /// Adapter invokes [`Props::on_index_change`] with the newly requested
    /// slide index. Emitted whenever the machine changes the index (manual
    /// navigation, swipe, auto-play tick, focus-driven scroll). This is the
    /// round-trip path for **controlled** carousels: in controlled mode
    /// [`Bindable::set`](ars_core::Bindable::set) only updates the pending
    /// internal value, so the parent must observe this callback and push the
    /// new value back through `Props::index` for the visible slide to change.
    IndexChange,
}

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Carousel`](self) component.
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

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Carousel`](self) state machine.
///
/// Not `Copy` because [`Event::SyncProps`] carries an owned [`Props`].
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Navigate to a specific slide by index.
    GoToSlide {
        /// Zero-based slide index to navigate to.
        index: usize,
    },

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
    /// hover/focus pauses ([`Event::HoverStart`] / [`Event::FocusSlide`]).
    AutoPlayPause,

    /// Pointer entered the carousel. Pauses auto-play **only** when
    /// [`AutoPlayOptions::pause_on_hover`] is set; otherwise a no-op.
    HoverStart,

    /// Pointer left the carousel. Resumes a hover/focus auto-play pause (the
    /// counterpart of [`Event::HoverStart`], mirroring [`Event::Blur`]).
    HoverEnd,

    /// Resume auto-play after pause.
    AutoPlayResume,

    /// The CSS transition animation completed.
    TransitionEnd,

    /// Pointer down on the viewport (drag start).
    PointerDown {
        /// Pointer position along the carousel axis, in pixels.
        pos: f64,

        /// Event timestamp in milliseconds (from `performance.now()`).
        timestamp: f64,
    },

    /// Pointer moved during drag.
    PointerMove {
        /// Pointer position along the carousel axis, in pixels.
        pos: f64,

        /// Event timestamp in milliseconds (from `performance.now()`).
        timestamp: f64,
    },

    /// Pointer released (drag end).
    PointerUp,

    /// Pointer cancelled (drag abort).
    PointerCancel,

    /// A slide received focus.
    FocusSlide {
        /// Zero-based index of the slide that received focus.
        index: usize,
    },

    /// Focus left the carousel.
    Blur,

    /// The parent re-rendered with new [`Props`] (via `set_props`). Emitted by
    /// [`Machine::on_props_changed`](ars_core::Machine::on_props_changed) so the
    /// machine re-derives its mutable configuration, tracks the controlled
    /// `index` signal (including controlled→uncontrolled), and reconciles the
    /// auto-play timer — all without animating.
    SyncProps {
        /// The new props to reconcile against the current context.
        props: Props,
    },
}

// ────────────────────────────────────────────────────────────────────
// Context configuration types
// ────────────────────────────────────────────────────────────────────

/// Slide alignment within the viewport.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum SlideAlignment {
    /// Align the active slide to the start edge of the viewport.
    #[default]
    Start,

    /// Center the active slide within the viewport.
    Center,

    /// Align the active slide to the end edge of the viewport.
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

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

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

    /// Whether auto-play is temporarily paused (hover/focus).
    pub auto_play_paused: bool,

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

    /// Time-normalized swipe velocity (px/ms). Independent of display refresh
    /// rate.
    pub swipe_velocity: f64,

    /// Timestamp of the last `PointerMove` event (ms, from
    /// `performance.now()`).
    pub drag_last_timestamp: Option<f64>,

    /// Whether the carousel is right-to-left.
    pub is_rtl: bool,

    /// Resolved locale for `MessageFn` calls.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// The current slide index.
    #[must_use]
    pub fn current_index(&self) -> usize {
        *self.index.get()
    }

    /// Number of slide slots occupied by the viewport at once, rounding a
    /// fractional [`slides_per_view`](Self::slides_per_view) up so a partially
    /// visible trailing slide still counts as on-screen. Always at least `1`.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        (self.slides_per_view.ceil() as usize).max(1)
    }

    /// Largest valid starting index for non-looping navigation.
    ///
    /// With `slides_per_view > 1` the last page is flush to the end
    /// (contain-scroll): the final starting index is
    /// `slide_count - visible_count`, so the trailing slide is fully shown
    /// instead of leaving a partial/blank page past the boundary.
    #[must_use]
    pub fn last_index(&self) -> usize {
        self.slide_count.get().saturating_sub(self.visible_count())
    }

    /// Clamp or wrap an index according to `loop_nav`.
    ///
    /// When `loop_nav` is `true`, out-of-range indices wrap around modulo the
    /// slide count; otherwise they are clamped to the `[0, last_index]` range
    /// (see [`last_index`](Self::last_index), which accounts for
    /// `slides_per_view`).
    #[must_use]
    pub fn clamp_index(&self, i: isize) -> usize {
        let n = self.slide_count.get() as isize;

        if self.loop_nav {
            ((i % n) + n) as usize % self.slide_count.get()
        } else {
            (i.max(0) as usize).min(self.last_index())
        }
    }

    /// Whether the carousel can navigate to a previous slide.
    ///
    /// Always `false` when [`last_index`](Self::last_index) is `0` (a single
    /// slide, or `slides_per_view` already shows them all): there is no
    /// distinct target to move to, even when looping.
    #[must_use]
    pub fn can_go_prev(&self) -> bool {
        self.current_index() > 0 || (self.loop_nav && self.last_index() > 0)
    }

    /// Whether the carousel can navigate to a next slide.
    ///
    /// For `slides_per_view > 1` the boundary is the last full page
    /// ([`last_index`](Self::last_index)), so `Next` disables once the final
    /// slide is already in view rather than allowing a partial page. Always
    /// `false` when `last_index` is `0`, even when looping.
    #[must_use]
    pub fn can_go_next(&self) -> bool {
        self.current_index() < self.last_index() || (self.loop_nav && self.last_index() > 0)
    }

    /// Whether `index` is within the currently visible window of
    /// `visible_count` slides starting at `current_index` (wrapping when
    /// `loop_nav` is set). Slides outside the window are hidden from
    /// assistive technology.
    #[must_use]
    pub fn is_slide_visible(&self, index: usize) -> bool {
        let current = self.current_index();
        let count = self.slide_count.get();
        (0..self.visible_count()).any(|offset| {
            let slot = if self.loop_nav {
                (current + offset) % count
            } else {
                current + offset
            };
            slot == index
        })
    }

    /// CSS translate percentage for the slide track, given the live viewport
    /// extent supplied by the adapter.
    ///
    /// The agnostic core does not read geometry; the adapter passes the
    /// measured viewport extent **along the carousel axis** — width for a
    /// [`Orientation::Horizontal`] carousel, height for an
    /// [`Orientation::Vertical`] one — so the drag-follow correction can be
    /// expressed as a percentage of the track. Returns the percentage the
    /// adapter should apply as a `translate` along the carousel axis.
    #[must_use]
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

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// External configuration for the [`Carousel`](self) component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
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
    /// changes the index. **Required for controlled usage** (when [`index`] is
    /// `Some`): the parent must update its controlled signal from this callback
    /// and push it back through [`index`], otherwise navigation only updates the
    /// hidden pending value and the visible slide never moves.
    ///
    /// [`index`]: Self::index
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

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Closure type for the slide label message, given a slide's one-based
/// position, the total slide count, and the active locale.
pub type SlideLabelFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Translatable messages for the [`Carousel`](self) component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the carousel region (`aria-label` on `Root`).
    pub carousel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Role description for the carousel region
    /// (`aria-roledescription` on `Root`).
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Role description for each slide (`aria-roledescription` on `Item`).
    pub slide_role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for a slide, given its one-based position and the
    /// total slide count (e.g. "Slide 2 of 5").
    pub slide_label: MessageFn<SlideLabelFn>,

    /// Accessible label for the previous-slide trigger.
    pub prev_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the next-slide trigger.
    pub next_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the auto-play trigger while auto-play is running.
    pub pause_auto_play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the auto-play trigger while auto-play is stopped
    /// or paused.
    pub start_auto_play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            carousel_label: MessageFn::static_str("Carousel"),
            role_description: MessageFn::static_str("carousel"),
            slide_role_description: MessageFn::static_str("slide"),
            slide_label: MessageFn::new(|index, total, _locale: &Locale| {
                format!("Slide {index} of {total}")
            }),
            prev_label: MessageFn::static_str("Previous slide"),
            next_label: MessageFn::static_str("Next slide"),
            pause_auto_play_label: MessageFn::static_str("Pause automatic slide show"),
            start_auto_play_label: MessageFn::static_str("Start automatic slide show"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// The [`Carousel`](self) state machine.
#[derive(Debug)]
pub struct Machine;

/// Build the transition for a manual navigation to `idx`: enter
/// [`State::Transitioning`] and set the index, and — when
/// `stop_on_interaction` is configured — permanently stop auto-play and
/// cancel its timer. The cancellation is essential: without it the adapter's
/// recurring interval keeps running after rotation has "stopped", leaking the
/// timer and dispatching ignored [`Event::AutoPlayTick`]s forever.
///
/// Returns `None` when `idx` equals the current index: the track transform
/// would not change, so the adapter has no CSS transition to report and
/// `TransitionEnd` may never arrive — entering `Transitioning` would strand
/// the machine and block all further navigation.
fn navigate_to(ctx: &Context, idx: usize) -> Option<TransitionPlan<Machine>> {
    if idx == ctx.current_index() {
        return None;
    }
    let stop = ctx
        .auto_play
        .as_ref()
        .is_some_and(|o| o.stop_on_interaction);
    let mut plan = TransitionPlan::to(State::Transitioning)
        .apply(move |ctx: &mut Context| {
            ctx.index.set(idx);
            if stop {
                ctx.auto_play_stopped = true;
            }
        })
        .with_effect(index_change_effect(idx));
    if stop {
        plan = plan.cancel_effect(Effect::AutoPlayTimer);
    }
    Some(plan)
}

/// Build the [`Effect::IndexChange`] notification carrying the newly requested
/// slide `index`. The adapter resolves it by invoking [`Props::on_index_change`]
/// — the round-trip path that lets a controlled parent observe navigation and
/// push the new index back through [`Props::index`].
fn index_change_effect(index: usize) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::IndexChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_index_change {
                callback(index);
            }
            ars_core::no_cleanup()
        },
    )
}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let initial_state = if props.auto_play.is_some() {
            State::AutoPlaying
        } else {
            State::Idle
        };

        // Normalize `slides_per_view`: a non-finite or non-positive value
        // would make `track_offset_percent` divide by zero/NaN and corrupt the
        // visible-window math, so fall back to one full slide.
        let slides_per_view = props
            .slides_per_view
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(1.0);

        // `slides_per_move` of zero would make every navigation a no-op (while
        // still able to stop auto-play), so clamp it to at least one.
        let slides_per_move = props.slides_per_move.unwrap_or(1).max(1);

        // Clamp the uncontrolled default to the last valid starting index so
        // `current_index < slide_count` holds from the very first render (a
        // bad `default_index` would otherwise yield "Slide 100 of 3" with no
        // item marked current). Accounts for `slides_per_view`.
        let visible_count = (slides_per_view.ceil() as usize).max(1);
        let max_index = props.slide_count.get().saturating_sub(visible_count);
        let initial_index = props.default_index.unwrap_or(0).min(max_index);

        // Clamp the controlled value too: a caller-supplied controlled `index`
        // past `last_index()` would start the machine out of range before any
        // prop-change sync could run.
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
            auto_play_paused: false,
            spacing: props.spacing.unwrap_or(0.0),
            slides_per_view,
            slides_per_move,
            align: props.align.unwrap_or_default(),
            orientation: props.orientation.unwrap_or_default(),
            is_rtl: props.is_rtl,
            transition_duration: props
                .transition_duration
                .unwrap_or_else(|| Duration::from_millis(300)),
            drag_start_pos: None,
            drag_delta: 0.0,
            swipe_threshold: props.swipe_threshold.unwrap_or(50.0),
            swipe_velocity: 0.0,
            drag_last_timestamp: None,
            locale: env.locale.clone(),
            messages: messages.clone(),
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
        assert_eq!(
            old.id, new.id,
            "carousel::Props.id must remain stable after initialization"
        );

        // Any prop change re-syncs the machine's mutable configuration, the
        // controlled-index signal, and the auto-play timer (see the
        // `SyncProps` arm). Mirrors the `splitter` convention.
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
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::GoToSlide { index } => {
                if *state == State::Transitioning {
                    return None;
                }

                // A direct jump never wraps; clamp out-of-range targets to the
                // last valid starting index so `current_index` can never
                // exceed the boundary (every connect method relies on that
                // invariant). A direct jump is a manual interaction, so it
                // honours `stop_on_interaction` like Next/Prev.
                let idx = (*index).min(ctx.last_index());
                navigate_to(ctx, idx)
            }

            Event::GoToNext => {
                if *state == State::Transitioning || !ctx.can_go_next() {
                    return None;
                }

                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);
                navigate_to(ctx, next)
            }

            Event::GoToPrev => {
                if *state == State::Transitioning || !ctx.can_go_prev() {
                    return None;
                }

                let step = ctx.slides_per_move as isize;
                let prev = ctx.clamp_index(ctx.current_index() as isize - step);
                navigate_to(ctx, prev)
            }

            Event::TransitionEnd => {
                if ctx.auto_play.is_some() && !ctx.auto_play_stopped && !ctx.auto_play_paused {
                    Some(TransitionPlan::to(State::AutoPlaying))
                } else {
                    Some(TransitionPlan::to(State::Idle))
                }
            }

            Event::AutoPlayStart => {
                if ctx.auto_play_stopped || ctx.auto_play.is_none() {
                    return None;
                }

                // Clear any prior pause: starting auto-play means it is now
                // actively rotating, so the paused live-region mode and
                // `aria-pressed="false"` must not linger.
                Some(
                    TransitionPlan::to(State::AutoPlaying)
                        .apply(|ctx: &mut Context| {
                            ctx.auto_play_paused = false;
                        })
                        .with_effect(PendingEffect::named(Effect::AutoPlayTimer)),
                )
            }

            Event::AutoPlayStop => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.auto_play_stopped = true;
                    })
                    .cancel_effect(Effect::AutoPlayTimer),
            ),

            Event::AutoPlayTick => {
                if *state != State::AutoPlaying {
                    return None;
                }

                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);

                // Ignore ticks that would not move the track: the non-looping
                // boundary (`clamp` returns the same index) and the looped
                // no-op case (single slide, or `slides_per_move` a multiple of
                // `slide_count`). Entering `Transitioning` with no transform
                // change risks a missing `transitionend` that strands the
                // machine.
                if next == ctx.current_index() {
                    return None;
                }

                Some(
                    TransitionPlan::to(State::Transitioning)
                        .apply(move |ctx: &mut Context| {
                            ctx.index.set(next);
                        })
                        .with_effect(index_change_effect(next)),
                )
            }

            Event::AutoPlayPause => {
                // Nothing to pause without auto-play configured. Guarding here
                // (not just in the trigger handler) keeps a stray `paused` flag
                // from being set while `auto_play` is `None` — a stale flag
                // would suppress the timer if the parent later enables autoplay.
                ctx.auto_play.as_ref()?;

                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(
                    plan.apply(|ctx: &mut Context| {
                        ctx.auto_play_paused = true;
                    })
                    .cancel_effect(Effect::AutoPlayTimer),
                )
            }

            Event::HoverStart => {
                // Hover-pause is opt-in via `pause_on_hover`; otherwise hovering
                // does nothing. When enabled it pauses like `AutoPlayPause`:
                // leave `AutoPlaying` and cancel the timer.
                if !ctx
                    .auto_play
                    .as_ref()
                    .is_some_and(|options| options.pause_on_hover)
                {
                    return None;
                }

                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(
                    plan.apply(|ctx: &mut Context| {
                        ctx.auto_play_paused = true;
                    })
                    .cancel_effect(Effect::AutoPlayTimer),
                )
            }

            Event::HoverEnd => {
                // Resume a hover/focus pause when the pointer leaves, mirroring
                // `Blur` for focus-out.
                if ctx.auto_play_paused && !ctx.auto_play_stopped && ctx.auto_play.is_some() {
                    return Some(
                        TransitionPlan::to(State::AutoPlaying)
                            .apply(|ctx: &mut Context| {
                                ctx.auto_play_paused = false;
                            })
                            .with_effect(PendingEffect::named(Effect::AutoPlayTimer)),
                    );
                }
                None
            }

            Event::AutoPlayResume => {
                // Nothing to resume without auto-play configured (the paused and
                // stopped flags are only ever set while it is configured).
                ctx.auto_play.as_ref()?;

                // Resume is also the "restart" path the auto-play trigger
                // dispatches when rotation was permanently stopped (the trigger
                // shows the "Start" label and sends `AutoPlayResume`), so it
                // clears BOTH the paused and the stopped flags. Without clearing
                // `auto_play_stopped`, a stopped carousel's start control would
                // be inert with no way to resume rotation.
                Some(
                    TransitionPlan::to(State::AutoPlaying)
                        .apply(|ctx: &mut Context| {
                            ctx.auto_play_paused = false;
                            ctx.auto_play_stopped = false;
                        })
                        .with_effect(PendingEffect::named(Effect::AutoPlayTimer)),
                )
            }

            Event::PointerDown { pos, timestamp } => {
                let pos = *pos;
                let timestamp = *timestamp;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.drag_start_pos = Some(pos);
                        ctx.drag_delta = 0.0;
                        ctx.swipe_velocity = 0.0;
                        ctx.drag_last_timestamp = Some(timestamp);
                    })
                    .cancel_effect(Effect::AutoPlayTimer),
                )
            }

            Event::PointerMove { pos, timestamp } => {
                let start = ctx.drag_start_pos?;
                let pos = *pos;
                let timestamp = *timestamp;
                let prev_delta = ctx.drag_delta;
                let prev_ts = ctx.drag_last_timestamp;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.drag_delta = pos - start;

                    let pixel_delta = ctx.drag_delta - prev_delta;
                    let dt = prev_ts.map_or(0.0, |t| timestamp - t);

                    ctx.swipe_velocity = if dt > 0.0 { pixel_delta / dt } else { 0.0 };
                    ctx.drag_last_timestamp = Some(timestamp);
                }))
            }

            Event::PointerUp => {
                // No-op without an active drag: a stray/adapter-level pointer-up
                // must not run the resume logic (which would re-arm a timer that
                // was never cancelled, creating duplicate intervals). Mirrors
                // the `PointerMove`/`PointerCancel` guards.
                ctx.drag_start_pos?;

                let delta = ctx.drag_delta;
                let velocity = ctx.swipe_velocity;
                let threshold = ctx.swipe_threshold;

                // A brisk flick (>0.5 px/ms) reduces the distance threshold.
                let effective = if velocity.abs() > 0.5 {
                    threshold * 0.3
                } else {
                    threshold
                };

                let cur = ctx.current_index() as isize;
                // A swipe advances by `slides_per_move`, matching button and
                // keyboard navigation (page-by-page when configured).
                let step = ctx.slides_per_move as isize;

                let next_idx = if delta < -effective && ctx.can_go_next() {
                    Some(ctx.clamp_index(cur + step))
                } else if delta > effective && ctx.can_go_prev() {
                    Some(ctx.clamp_index(cur - step))
                } else {
                    None
                };

                // `PointerDown` cancelled the auto-play timer for the duration
                // of the drag. Resolve rotation now the gesture ended:
                //  - a navigating swipe with `stop_on_interaction` permanently
                //    stops it (mark stopped; the timer stays cancelled);
                //  - otherwise, if rotation was still active (not stopped, not
                //    focus/hover-paused), re-arm the timer so it resumes —
                //    without this a swipe silently kills rotation, or leaves
                //    the state reporting "playing" with no timer running.
                let navigated = next_idx.is_some();
                let stop_on_interaction = ctx
                    .auto_play
                    .as_ref()
                    .is_some_and(|o| o.stop_on_interaction);
                let mark_stopped = navigated && stop_on_interaction;
                let resume = ctx.auto_play.is_some()
                    && !mark_stopped
                    && !ctx.auto_play_stopped
                    && !ctx.auto_play_paused;

                let target = if resume {
                    State::AutoPlaying
                } else {
                    State::Idle
                };

                let index_changed = next_idx.is_some_and(|idx| idx != ctx.current_index());

                let mut plan = TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;

                    if let Some(idx) = next_idx {
                        ctx.index.set(idx);
                    }

                    if mark_stopped {
                        ctx.auto_play_stopped = true;
                    }
                });

                if index_changed && let Some(idx) = next_idx {
                    plan = plan.with_effect(index_change_effect(idx));
                }

                if resume {
                    plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer));
                }

                Some(plan)
            }

            Event::PointerCancel => {
                // The drag is aborted (e.g. touch scrolling or pointer-capture
                // interruption). `PointerDown` cancelled the timer, so if the
                // gesture was interrupting an active auto-play carousel, re-arm
                // the timer — otherwise rotation silently dies even though the
                // state still reads `AutoPlaying`.
                let resume = ctx.drag_start_pos.is_some()
                    && ctx.auto_play.is_some()
                    && !ctx.auto_play_stopped
                    && !ctx.auto_play_paused;

                let mut plan = if resume {
                    TransitionPlan::to(State::AutoPlaying)
                } else {
                    TransitionPlan::new()
                }
                .apply(|ctx: &mut Context| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;
                });

                if resume {
                    plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer));
                }

                Some(plan)
            }

            Event::FocusSlide { index } => {
                // Only scroll when the focused slide is not already on-screen.
                // With `slides_per_view > 1`, tabbing into a non-leading but
                // visible slide must NOT shift the track (that would move
                // content out from under the user during normal focus nav).
                let scroll = !ctx.is_slide_visible(*index);
                // Clamp to the last valid starting index so a focused slide near
                // the end maps to the last full page rather than overscrolling.
                let idx = (*index).min(ctx.last_index());
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
                    let mut plan = plan
                        .apply(move |ctx: &mut Context| {
                            if scroll {
                                ctx.index.set(idx);
                            }
                            ctx.auto_play_paused = true;
                        })
                        .cancel_effect(Effect::AutoPlayTimer);
                    if scroll {
                        plan = plan.with_effect(index_change_effect(idx));
                    }
                    return Some(plan);
                }

                if !scroll {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.index.set(idx);
                    })
                    .with_effect(index_change_effect(idx)),
                )
            }

            Event::SyncProps { props } => {
                // Re-derive mutable configuration from the new props, track the
                // controlled-index signal (including controlled→uncontrolled),
                // and reconcile the auto-play timer — all without animating.
                let new_auto = props.auto_play.clone();
                let auto_changed = ctx.auto_play != new_auto;
                // After syncing, should a timer be running? (Honours the
                // preserved stopped/paused flags.)
                let want_timer =
                    new_auto.is_some() && !ctx.auto_play_stopped && !ctx.auto_play_paused;

                // Only the auto-play transition moves the resting state:
                // enabling/resuming → AutoPlaying; disabling while playing → Idle.
                let target = if auto_changed {
                    if want_timer {
                        State::AutoPlaying
                    } else if *state == State::AutoPlaying {
                        State::Idle
                    } else {
                        *state
                    }
                } else {
                    *state
                };

                let props = props.clone();
                let mut plan = TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    let slides_per_view = props
                        .slides_per_view
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
                    ctx.transition_duration = props
                        .transition_duration
                        .unwrap_or_else(|| Duration::from_millis(300));
                    ctx.swipe_threshold = props.swipe_threshold.unwrap_or(50.0);

                    // Track the controlled signal (clamped); `None` returns the
                    // bindable to uncontrolled mode.
                    let controlled = props
                        .index
                        .as_ref()
                        .map(|bindable| (*bindable.get()).min(ctx.last_index()));
                    ctx.index.sync_controlled(controlled);

                    // Keep an uncontrolled index in range if the slide count or
                    // visible window shrank.
                    if !ctx.index.is_controlled() {
                        let clamped = ctx.current_index().min(ctx.last_index());
                        ctx.index.set(clamped);
                    }

                    // Without auto-play configured, the play/pause flags are
                    // meaningless — reset them so the controls read correctly.
                    if ctx.auto_play.is_none() {
                        ctx.auto_play_paused = false;
                        ctx.auto_play_stopped = false;
                    }
                });

                // Restart the interval when auto-play config changed so a new
                // interval/enable takes effect; tear it down when disabled.
                if auto_changed {
                    plan = plan.cancel_effect(Effect::AutoPlayTimer);
                    if want_timer {
                        plan = plan.with_effect(PendingEffect::named(Effect::AutoPlayTimer));
                    }
                }

                Some(plan)
            }

            Event::Blur => {
                if ctx.auto_play_paused && !ctx.auto_play_stopped && ctx.auto_play.is_some() {
                    return Some(
                        TransitionPlan::to(State::AutoPlaying)
                            .apply(|ctx: &mut Context| {
                                ctx.auto_play_paused = false;
                            })
                            .with_effect(PendingEffect::named(Effect::AutoPlayTimer)),
                    );
                }

                None
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// The anatomy parts of the [`Carousel`](self) component.
#[derive(ComponentPart)]
#[scope = "carousel"]
pub enum Part {
    /// The root carousel region (`<section>`).
    Root,

    /// The viewport that clips the slide track.
    Viewport,

    /// The group wrapping all slides (the live region).
    ItemGroup,

    /// A single slide by index.
    Item {
        /// Zero-based slide index.
        index: usize,
    },

    /// The previous-slide trigger.
    PrevTrigger,

    /// The next-slide trigger.
    NextTrigger,

    /// The group wrapping the dot indicators (`role="tablist"`).
    IndicatorGroup,

    /// A single dot indicator by index.
    Indicator {
        /// Zero-based slide index this indicator targets.
        index: usize,
    },

    /// The auto-play play/pause trigger.
    AutoPlayTrigger,

    /// The decorative auto-play status indicator.
    AutoPlayIndicator,

    /// The live progress-text element (e.g. "Slide 2 of 5").
    ProgressText,
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connect API producing attributes and event handlers for the
/// [`Carousel`](self) anatomy parts.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Attributes for the root carousel region.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "region")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.carousel_label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.role_description)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Data("ars-state"),
                match self.state {
                    State::Idle => "idle",
                    State::AutoPlaying => "auto-playing",
                    State::Transitioning => "transitioning",
                },
            )
            .set(
                HtmlAttr::Data("ars-orientation"),
                match self.ctx.orientation {
                    Orientation::Horizontal => "horizontal",
                    Orientation::Vertical => "vertical",
                },
            );

        attrs
    }

    /// Attributes for the viewport that clips the slide track.
    #[must_use]
    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let touch_action = if self.ctx.orientation == Orientation::Horizontal {
            "pan-y"
        } else {
            "pan-x"
        };

        attrs
            .set_style(CssProperty::Overflow, "hidden")
            .set_style(CssProperty::TouchAction, touch_action);

        attrs
    }

    /// Attributes for the group wrapping all slides (the live region).
    #[must_use]
    pub fn item_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroup.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        // "off" only while rotation is actively advancing; once auto-play is
        // absent, permanently stopped, OR temporarily paused (hover/focus),
        // manual slide changes must be announced politely.
        let live = if self.ctx.auto_play.is_none()
            || self.ctx.auto_play_stopped
            || self.ctx.auto_play_paused
        {
            "polite"
        } else {
            "off"
        };

        attrs.set(HtmlAttr::Aria(AriaAttr::Live), live);

        attrs
    }

    /// Attributes for a single slide by index.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = (Part::Item { index }).data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let is_current = index == self.ctx.current_index();
        // With `slides_per_view > 1` several slides are on-screen at once; only
        // slides outside the visible window are hidden from assistive tech and
        // `inert`. Marking a visible slide hidden would make on-screen content
        // unreachable to screen-reader and keyboard users.
        let is_hidden = !self.ctx.is_slide_visible(index);

        attrs
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.slide_role_description)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.slide_label)(
                    index + 1,
                    self.ctx.slide_count.get(),
                    &self.ctx.locale,
                ),
            )
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set_bool(HtmlAttr::Data("ars-active"), is_current);

        if is_hidden {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
                .set_bool(HtmlAttr::Inert, true);
        }

        attrs
    }

    /// Attributes for the previous-slide trigger.
    #[must_use]
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.prev_label)(&self.ctx.locale),
            );

        if !self.ctx.can_go_prev() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the next-slide trigger.
    #[must_use]
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.next_label)(&self.ctx.locale),
            );

        if !self.ctx.can_go_next() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the group wrapping the dot indicators.
    #[must_use]
    pub fn indicator_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::IndicatorGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "tablist");

        attrs
    }

    /// Attributes for a single dot indicator by index.
    #[must_use]
    pub fn indicator_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::Indicator { index }).data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "tab")
            .set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if index == self.ctx.current_index() {
                    "true"
                } else {
                    "false"
                },
            );

        attrs
    }

    /// Attributes for the auto-play play/pause trigger.
    #[must_use]
    pub fn auto_play_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayTrigger.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let is_playing = self.ctx.auto_play.is_some()
            && !self.ctx.auto_play_stopped
            && !self.ctx.auto_play_paused;

        let label = if is_playing {
            (self.ctx.messages.pause_auto_play_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.start_auto_play_label)(&self.ctx.locale)
        };

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label).set(
            HtmlAttr::Aria(AriaAttr::Pressed),
            if is_playing { "true" } else { "false" },
        );

        attrs
    }

    /// Attributes for the decorative auto-play status indicator.
    #[must_use]
    pub fn auto_play_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayIndicator.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let is_playing = self.ctx.auto_play.is_some()
            && !self.ctx.auto_play_stopped
            && !self.ctx.auto_play_paused;

        attrs
            .set(
                HtmlAttr::Data("ars-state"),
                if is_playing { "playing" } else { "paused" },
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the progress text element (e.g. "2 of 5").
    #[must_use]
    pub fn progress_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ProgressText.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Human-readable progress string (e.g. "Slide 2 of 5").
    #[must_use]
    pub fn progress_text(&self) -> String {
        (self.ctx.messages.slide_label)(
            self.ctx.current_index() + 1,
            self.ctx.slide_count.get(),
            &self.ctx.locale,
        )
    }

    /// Handle a keydown on the root region: arrow keys navigate (reversed for
    /// RTL horizontal carousels), `Home`/`End` jump to the first/last slide.
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
            key if key == prev_key => (self.send)(Event::GoToPrev),

            key if key == next_key => (self.send)(Event::GoToNext),

            KeyboardKey::Home => (self.send)(Event::GoToSlide { index: 0 }),

            KeyboardKey::End => (self.send)(Event::GoToSlide {
                index: self.ctx.last_index(),
            }),

            _ => {}
        }
    }

    /// Dispatch a previous-slide navigation.
    pub fn on_prev_trigger_click(&self) {
        (self.send)(Event::GoToPrev);
    }

    /// Dispatch a next-slide navigation.
    pub fn on_next_trigger_click(&self) {
        (self.send)(Event::GoToNext);
    }

    /// Dispatch navigation to the indicator's slide.
    pub fn on_indicator_click(&self, index: usize) {
        (self.send)(Event::GoToSlide { index });
    }

    /// Toggle auto-play: resume when stopped/paused, otherwise pause.
    ///
    /// No-op when auto-play is not configured — there is nothing to toggle, and
    /// dispatching `AutoPlayPause` would set a stale `paused` flag that suppresses
    /// the timer if the parent later enables auto-play.
    pub fn on_auto_play_trigger_click(&self) {
        if self.ctx.auto_play.is_none() {
            return;
        }
        if self.ctx.auto_play_stopped || self.ctx.auto_play_paused {
            (self.send)(Event::AutoPlayResume);
        } else {
            (self.send)(Event::AutoPlayPause);
        }
    }

    /// Dispatch a hover-start (pointer entered the carousel). Auto-play pauses
    /// only when [`AutoPlayOptions::pause_on_hover`] is set.
    pub fn on_root_pointer_enter(&self) {
        (self.send)(Event::HoverStart);
    }

    /// Dispatch a hover-end (pointer left the carousel), resuming a hover pause.
    pub fn on_root_pointer_leave(&self) {
        (self.send)(Event::HoverEnd);
    }

    /// Dispatch a pointer-down (drag start) with adapter-normalized position
    /// and timestamp.
    pub fn on_viewport_pointerdown(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerDown { pos, timestamp });
    }

    /// Dispatch a pointer-move during a drag.
    pub fn on_viewport_pointermove(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerMove { pos, timestamp });
    }

    /// Dispatch a pointer-up (drag end).
    pub fn on_viewport_pointerup(&self) {
        (self.send)(Event::PointerUp);
    }
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

#[cfg(test)]
mod tests {
    use core::cell::RefCell;
    use std::sync::{Arc, Mutex};

    use ars_core::{Env, SendResult, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    // ── helpers ──────────────────────────────────────────────────────

    fn nz(n: usize) -> NonZero<usize> {
        NonZero::new(n).expect("non-zero slide count")
    }

    /// Plain carousel with `slide_count` slides and no auto-play.
    fn props(slide_count: usize) -> Props {
        Props {
            id: String::from("carousel"),
            slide_count: nz(slide_count),
            ..Props::default()
        }
    }

    /// Carousel with default auto-play enabled.
    fn autoplay_props(slide_count: usize) -> Props {
        Props {
            auto_play: Some(AutoPlayOptions::default()),
            ..props(slide_count)
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn keydown(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn pending_effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    /// Drive a slide change to completion: navigate then settle the
    /// transition so the machine returns to a resting state.
    fn settle(service: &mut Service<Machine>) {
        drop(service.send(Event::TransitionEnd));
    }

    // ── init / state transitions ─────────────────────────────────────

    #[test]
    fn init_idle_without_autoplay() {
        let service = service(props(3));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn init_autoplaying_with_autoplay() {
        let service = service(autoplay_props(3));

        assert_eq!(service.state(), &State::AutoPlaying);
    }

    #[test]
    fn init_default_index_seeds_current_index() {
        let service = service(Props {
            default_index: Some(2),
            ..props(3)
        });

        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn goto_slide_enters_transitioning_and_sets_index() {
        let mut service = service(props(3));

        drop(service.send(Event::GoToSlide { index: 2 }));

        assert_eq!(service.state(), &State::Transitioning);
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn goto_slide_clamps_out_of_range_to_last() {
        let mut service = service(props(3));

        drop(service.send(Event::GoToSlide { index: 99 }));

        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn focus_slide_clamps_out_of_range_to_last() {
        let mut service = service(props(3));

        drop(service.send(Event::FocusSlide { index: 99 }));

        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn nav_blocked_while_transitioning() {
        let mut service = service(props(3));

        drop(service.send(Event::GoToSlide { index: 1 }));

        assert_eq!(service.state(), &State::Transitioning);

        let result = service.send(Event::GoToNext);

        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn transition_end_returns_to_idle_without_autoplay() {
        let mut service = service(props(3));

        drop(service.send(Event::GoToSlide { index: 1 }));

        settle(&mut service);

        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn transition_end_returns_to_autoplaying_when_active() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());

        // An auto-play tick advances without stopping rotation, so settling
        // the transition returns to AutoPlaying.
        drop(service.send(Event::AutoPlayTick));
        assert_eq!(service.state(), &State::Transitioning);

        settle(&mut service);
        assert_eq!(service.state(), &State::AutoPlaying);
    }

    // ── slide navigation ─────────────────────────────────────────────

    #[test]
    fn goto_next_and_prev_advance_by_one() {
        let mut service = service(props(4));

        drop(service.send(Event::GoToNext));

        assert_eq!(service.context().current_index(), 1);

        settle(&mut service);

        drop(service.send(Event::GoToNext));

        assert_eq!(service.context().current_index(), 2);

        settle(&mut service);

        drop(service.send(Event::GoToPrev));

        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn slides_per_move_advances_in_pages() {
        let mut service = service(Props {
            slides_per_move: Some(2),
            ..props(6)
        });

        drop(service.send(Event::GoToNext));

        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn goto_prev_at_start_blocked_without_loop() {
        let mut service = service(props(3));

        let result = service.send(Event::GoToPrev);

        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn goto_next_at_end_blocked_without_loop() {
        let mut service = service(Props {
            default_index: Some(2),
            ..props(3)
        });

        let result = service.send(Event::GoToNext);

        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn can_go_prev_next_at_boundaries() {
        let ctx = service(props(3)).context().clone();

        assert!(!ctx.can_go_prev());
        assert!(ctx.can_go_next());
    }

    // ── loop mode ────────────────────────────────────────────────────

    #[test]
    fn loop_wraps_next_from_last() {
        let mut service = service(Props {
            loop_nav: true,
            default_index: Some(2),
            ..props(3)
        });

        drop(service.send(Event::GoToNext));

        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn loop_wraps_prev_from_first() {
        let mut service = service(Props {
            loop_nav: true,
            ..props(3)
        });

        drop(service.send(Event::GoToPrev));

        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn clamp_index_wraps_and_clamps() {
        let looped = service(Props {
            loop_nav: true,
            ..props(3)
        });

        assert_eq!(looped.context().clamp_index(-1), 2);
        assert_eq!(looped.context().clamp_index(3), 0);

        let clamped = service(props(3));

        assert_eq!(clamped.context().clamp_index(-1), 0);
        assert_eq!(clamped.context().clamp_index(5), 2);
    }

    // ── auto-play ─────────────────────────────────────────────────────

    #[test]
    fn initial_effects_emit_timer_when_autoplaying() {
        let mut service = service(autoplay_props(3));

        let effects = service
            .take_initial_effects()
            .iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert_eq!(effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn initial_effects_empty_when_idle() {
        let mut service = service(props(3));

        assert!(service.take_initial_effects().is_empty());
    }

    #[test]
    fn autoplay_tick_advances_and_transitions() {
        let mut service = service(autoplay_props(3));

        drop(service.take_initial_effects());
        drop(service.send(Event::AutoPlayTick));

        assert_eq!(service.state(), &State::Transitioning);
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn autoplay_tick_ignored_when_not_autoplaying() {
        let mut service = service(props(3));

        let result = service.send(Event::AutoPlayTick);

        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn autoplay_tick_wraps_with_loop() {
        let mut service = service(Props {
            loop_nav: true,
            default_index: Some(2),
            ..autoplay_props(3)
        });

        drop(service.take_initial_effects());
        drop(service.send(Event::AutoPlayTick));

        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn autoplay_start_emits_timer() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions::default()),
            ..props(3)
        });

        // Move to Idle first (stop), then start fresh.
        drop(service.send(Event::AutoPlayPause));
        drop(service.send(Event::AutoPlayResume));

        let result = service.send(Event::AutoPlayStart);

        assert_eq!(service.state(), &State::AutoPlaying);
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn autoplay_start_noop_when_stopped() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayStop));

        assert!(service.context().auto_play_stopped);

        let result = service.send(Event::AutoPlayStart);

        assert!(!result.state_changed);
    }

    #[test]
    fn autoplay_stop_sets_stopped_and_cancels_timer() {
        let mut service = service(autoplay_props(3));

        let result = service.send(Event::AutoPlayStop);

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().auto_play_stopped);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn item_group_live_is_polite_once_autoplay_stopped() {
        let mut service = service(autoplay_props(3));

        // Active auto-play → "off".
        let live_off = service.connect(&|_| {}).item_group_attrs();

        assert_eq!(live_off.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("off"));

        // After stopping, the live region must become "polite" again.
        drop(service.send(Event::AutoPlayStop));

        let live_polite = service.connect(&|_| {}).item_group_attrs();

        assert_eq!(
            live_polite.get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("polite")
        );
    }

    #[test]
    fn stop_on_interaction_marks_stopped_on_manual_nav() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::GoToNext));

        assert!(service.context().auto_play_stopped);
    }

    #[test]
    fn stop_on_interaction_disabled_keeps_playing() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions {
                stop_on_interaction: false,
                ..AutoPlayOptions::default()
            }),
            ..props(3)
        });

        drop(service.send(Event::GoToNext));

        assert!(!service.context().auto_play_stopped);
    }

    // ── pause on hover / focus ───────────────────────────────────────

    #[test]
    fn autoplay_pause_sets_paused_and_cancels_timer() {
        let mut service = service(autoplay_props(3));

        let result = service.send(Event::AutoPlayPause);

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().auto_play_paused);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn autoplay_resume_restarts_timer() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayPause));

        let result = service.send(Event::AutoPlayResume);

        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_paused);
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn autoplay_resume_restarts_stopped_carousel() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());

        drop(service.send(Event::AutoPlayStop));
        assert!(service.context().auto_play_stopped);

        // Resume is the restart path: it clears the stopped flag, returns to
        // AutoPlaying, and re-arms the timer.
        let result = service.send(Event::AutoPlayResume);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_stopped);
        assert!(!service.context().auto_play_paused);
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn focus_slide_pauses_and_syncs_index() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::FocusSlide { index: 2 }));

        assert!(service.context().auto_play_paused);
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn focus_slide_without_pause_on_focus_keeps_playing() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions {
                pause_on_focus: false,
                ..AutoPlayOptions::default()
            }),
            ..props(3)
        });

        drop(service.send(Event::FocusSlide { index: 1 }));

        assert!(!service.context().auto_play_paused);
    }

    #[test]
    fn blur_resumes_when_paused() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayPause));

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_paused);
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn blur_noop_when_not_paused() {
        let mut service = service(autoplay_props(3));

        let result = service.send(Event::Blur);

        assert!(!result.state_changed);
    }

    // ── swipe gesture (injected pointer deltas) ──────────────────────

    #[test]
    fn pointer_down_records_start_and_cancels_timer() {
        let mut service = service(autoplay_props(3));

        let result = service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        });

        assert_eq!(service.context().drag_start_pos, Some(100.0));
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn pointer_move_accumulates_delta_and_velocity() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 80.0,
            timestamp: 10.0,
        }));

        assert_eq!(service.context().drag_delta, -20.0);
        assert_eq!(service.context().swipe_velocity, -2.0);
    }

    #[test]
    fn pointer_move_ignored_without_down() {
        let mut service = service(props(3));

        let result = service.send(Event::PointerMove {
            pos: 50.0,
            timestamp: 5.0,
        });

        assert!(!result.context_changed);
        assert_eq!(service.context().drag_delta, 0.0);
    }

    #[test]
    fn pointer_up_navigates_next_past_threshold() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));

        // delta -60 over 1000ms → slow drag, threshold stays 50.
        drop(service.send(Event::PointerMove {
            pos: 40.0,
            timestamp: 1000.0,
        }));
        drop(service.send(Event::PointerUp));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current_index(), 1);
        assert_eq!(service.context().drag_delta, 0.0);
    }

    #[test]
    fn pointer_up_navigates_prev_past_threshold() {
        let mut service = service(Props {
            default_index: Some(1),
            ..props(3)
        });

        drop(service.send(Event::PointerDown {
            pos: 40.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 100.0,
            timestamp: 1000.0,
        }));
        drop(service.send(Event::PointerUp));

        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn pointer_up_velocity_flick_lowers_threshold() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        // delta -20 (< 50) but fast (2 px/ms) → effective threshold 15.
        drop(service.send(Event::PointerMove {
            pos: 80.0,
            timestamp: 10.0,
        }));
        drop(service.send(Event::PointerUp));

        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn pointer_up_below_threshold_does_not_navigate() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        // delta -10, slow → no navigation.
        drop(service.send(Event::PointerMove {
            pos: 90.0,
            timestamp: 1000.0,
        }));
        drop(service.send(Event::PointerUp));

        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn pointer_cancel_resets_drag_state() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 70.0,
            timestamp: 10.0,
        }));
        drop(service.send(Event::PointerCancel));

        assert_eq!(service.context().drag_start_pos, None);
        assert_eq!(service.context().drag_delta, 0.0);
        assert_eq!(service.context().current_index(), 0);
    }

    // ── keyboard navigation ──────────────────────────────────────────

    fn captured_keydown(service: &Service<Machine>, key: KeyboardKey) -> Vec<Event> {
        let recorder = RefCell::new(Vec::new());

        {
            let record = |event| recorder.borrow_mut().push(event);

            let api = service.connect(&record);

            api.on_root_keydown(&keydown(key));
        }

        recorder.into_inner()
    }

    #[test]
    fn keydown_horizontal_ltr_arrows() {
        let service = service(props(3));

        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowRight),
            vec![Event::GoToNext]
        );
        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowLeft),
            vec![Event::GoToPrev]
        );
    }

    #[test]
    fn keydown_horizontal_rtl_reverses_arrows() {
        let service = service(Props {
            is_rtl: true,
            ..props(3)
        });

        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowRight),
            vec![Event::GoToPrev]
        );
        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowLeft),
            vec![Event::GoToNext]
        );
    }

    #[test]
    fn keydown_vertical_uses_up_down() {
        let service = service(Props {
            orientation: Some(Orientation::Vertical),
            ..props(3)
        });

        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowDown),
            vec![Event::GoToNext]
        );
        assert_eq!(
            captured_keydown(&service, KeyboardKey::ArrowUp),
            vec![Event::GoToPrev]
        );
    }

    #[test]
    fn keydown_unhandled_key_dispatches_nothing() {
        let service = service(props(3));

        assert!(captured_keydown(&service, KeyboardKey::Enter).is_empty());
    }

    #[test]
    fn keydown_home_end_jump_to_bounds() {
        let service = service(props(5));

        assert_eq!(
            captured_keydown(&service, KeyboardKey::Home),
            vec![Event::GoToSlide { index: 0 }]
        );
        assert_eq!(
            captured_keydown(&service, KeyboardKey::End),
            vec![Event::GoToSlide { index: 4 }]
        );
    }

    // ── click / pointer handler dispatch ─────────────────────────────

    #[test]
    fn click_handlers_dispatch_expected_events() {
        let service = service(autoplay_props(3));

        let recorder = RefCell::new(Vec::new());

        {
            let record = |event| recorder.borrow_mut().push(event);

            let api = service.connect(&record);

            api.on_prev_trigger_click();
            api.on_next_trigger_click();
            api.on_indicator_click(2);
            api.on_auto_play_trigger_click();
            api.on_viewport_pointerdown(10.0, 1.0);
            api.on_viewport_pointermove(20.0, 2.0);
            api.on_viewport_pointerup();
        }

        assert_eq!(
            recorder.into_inner(),
            vec![
                Event::GoToPrev,
                Event::GoToNext,
                Event::GoToSlide { index: 2 },
                Event::AutoPlayPause,
                Event::PointerDown {
                    pos: 10.0,
                    timestamp: 1.0
                },
                Event::PointerMove {
                    pos: 20.0,
                    timestamp: 2.0
                },
                Event::PointerUp,
            ]
        );
    }

    #[test]
    fn auto_play_trigger_resumes_when_paused() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayPause));

        let recorder = RefCell::new(Vec::new());

        {
            let record = |event| recorder.borrow_mut().push(event);

            let api = service.connect(&record);

            api.on_auto_play_trigger_click();
        }

        assert_eq!(recorder.into_inner(), vec![Event::AutoPlayResume]);
    }

    // ── track offset math ────────────────────────────────────────────

    #[test]
    fn track_offset_percent_basic_and_drag() {
        let service = service(Props {
            default_index: Some(1),
            ..props(3)
        });

        let ctx = service.context();

        // index 1, slides_per_view 1 → -100%.
        assert_eq!(ctx.track_offset_percent(200.0), -100.0);
    }

    #[test]
    fn track_offset_percent_applies_drag_correction() {
        let mut service = service(props(3));

        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 150.0,
            timestamp: 10.0,
        }));

        // index 0 → 0%; drag_delta +50 over 200px viewport → +25%.
        assert_eq!(service.context().track_offset_percent(200.0), 25.0);
    }

    #[test]
    fn track_offset_percent_fractional_view() {
        let service = service(Props {
            slides_per_view: Some(2.0),
            default_index: Some(1),
            ..props(4)
        });

        // index 1, per_slide = 50% → -50%.
        assert_eq!(service.context().track_offset_percent(0.0), -50.0);
    }

    // ── snapshots: connect()/Api AttrMap output per part & branch ─────

    #[test]
    fn snapshot_root_idle_horizontal() {
        let service = service(props(3));

        assert_snapshot!(
            "root_idle_horizontal",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_autoplaying() {
        let service = service(autoplay_props(3));

        assert_snapshot!(
            "root_autoplaying",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_transitioning() {
        let mut service = service(props(3));

        drop(service.send(Event::GoToSlide { index: 1 }));

        assert_snapshot!(
            "root_transitioning",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_vertical() {
        let service = service(Props {
            orientation: Some(Orientation::Vertical),
            ..props(3)
        });

        assert_snapshot!(
            "root_vertical",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_viewport_horizontal() {
        let service = service(props(3));

        assert_snapshot!(
            "viewport_horizontal",
            snapshot_attrs(&service.connect(&|_| {}).viewport_attrs())
        );
    }

    #[test]
    fn snapshot_viewport_vertical() {
        let service = service(Props {
            orientation: Some(Orientation::Vertical),
            ..props(3)
        });

        assert_snapshot!(
            "viewport_vertical",
            snapshot_attrs(&service.connect(&|_| {}).viewport_attrs())
        );
    }

    #[test]
    fn snapshot_item_group_polite_no_autoplay() {
        let service = service(props(3));

        assert_snapshot!(
            "item_group_polite",
            snapshot_attrs(&service.connect(&|_| {}).item_group_attrs())
        );
    }

    #[test]
    fn snapshot_item_group_off_during_autoplay() {
        let service = service(autoplay_props(3));

        assert_snapshot!(
            "item_group_off",
            snapshot_attrs(&service.connect(&|_| {}).item_group_attrs())
        );
    }

    #[test]
    fn snapshot_item_current() {
        let service = service(props(3));

        assert_snapshot!(
            "item_current",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(0))
        );
    }

    #[test]
    fn snapshot_item_hidden() {
        let service = service(props(3));

        assert_snapshot!(
            "item_hidden",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(1))
        );
    }

    #[test]
    fn snapshot_item_visible_not_current_multi_view() {
        // slides_per_view = 2, index 0: slide 1 is on-screen but not the
        // leading slide — visible (no aria-hidden/inert) yet ars-active=false.
        let service = service(Props {
            slides_per_view: Some(2.0),
            ..props(3)
        });

        assert_snapshot!(
            "item_visible_not_current_multi_view",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(1))
        );
    }

    #[test]
    fn snapshot_prev_trigger_disabled_at_start() {
        let service = service(props(3));

        assert_snapshot!(
            "prev_trigger_disabled",
            snapshot_attrs(&service.connect(&|_| {}).prev_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_prev_trigger_enabled() {
        let service = service(Props {
            default_index: Some(1),
            ..props(3)
        });

        assert_snapshot!(
            "prev_trigger_enabled",
            snapshot_attrs(&service.connect(&|_| {}).prev_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_next_trigger_disabled_at_end() {
        let service = service(Props {
            default_index: Some(2),
            ..props(3)
        });

        assert_snapshot!(
            "next_trigger_disabled",
            snapshot_attrs(&service.connect(&|_| {}).next_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_next_trigger_enabled() {
        let service = service(props(3));

        assert_snapshot!(
            "next_trigger_enabled",
            snapshot_attrs(&service.connect(&|_| {}).next_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_indicator_group() {
        let service = service(props(3));

        assert_snapshot!(
            "indicator_group",
            snapshot_attrs(&service.connect(&|_| {}).indicator_group_attrs())
        );
    }

    #[test]
    fn snapshot_indicator_selected() {
        let service = service(props(3));

        assert_snapshot!(
            "indicator_selected",
            snapshot_attrs(&service.connect(&|_| {}).indicator_attrs(0))
        );
    }

    #[test]
    fn snapshot_indicator_unselected() {
        let service = service(props(3));

        assert_snapshot!(
            "indicator_unselected",
            snapshot_attrs(&service.connect(&|_| {}).indicator_attrs(1))
        );
    }

    #[test]
    fn snapshot_auto_play_trigger_playing() {
        let service = service(autoplay_props(3));

        assert_snapshot!(
            "auto_play_trigger_playing",
            snapshot_attrs(&service.connect(&|_| {}).auto_play_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_auto_play_trigger_paused() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayPause));

        assert_snapshot!(
            "auto_play_trigger_paused",
            snapshot_attrs(&service.connect(&|_| {}).auto_play_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_autoplay_indicator_playing() {
        let service = service(autoplay_props(3));

        assert_snapshot!(
            "autoplay_indicator_playing",
            snapshot_attrs(&service.connect(&|_| {}).auto_play_indicator_attrs())
        );
    }

    #[test]
    fn snapshot_autoplay_indicator_paused() {
        let mut service = service(autoplay_props(3));

        drop(service.send(Event::AutoPlayPause));

        assert_snapshot!(
            "autoplay_indicator_paused",
            snapshot_attrs(&service.connect(&|_| {}).auto_play_indicator_attrs())
        );
    }

    #[test]
    fn snapshot_progress_text_attrs() {
        let service = service(props(3));

        assert_snapshot!(
            "progress_text_attrs",
            snapshot_attrs(&service.connect(&|_| {}).progress_text_attrs())
        );
    }

    #[test]
    fn progress_text_renders_human_readable_position() {
        let service = service(Props {
            default_index: Some(1),
            ..props(5)
        });

        assert_eq!(service.connect(&|_| {}).progress_text(), "Slide 2 of 5");
    }

    #[test]
    fn api_debug_is_non_exhaustive() {
        let service = service(props(3));

        let rendered = format!("{:?}", service.connect(&|_| {}));

        assert!(rendered.starts_with("Api {"));
        assert!(rendered.contains(".."));
    }

    #[test]
    fn part_attrs_delegates_to_each_anatomy_method() {
        let service = service(autoplay_props(3));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Viewport), api.viewport_attrs());
        assert_eq!(api.part_attrs(Part::ItemGroup), api.item_group_attrs());
        assert_eq!(api.part_attrs(Part::Item { index: 1 }), api.item_attrs(1));
        assert_eq!(api.part_attrs(Part::PrevTrigger), api.prev_trigger_attrs());
        assert_eq!(api.part_attrs(Part::NextTrigger), api.next_trigger_attrs());
        assert_eq!(
            api.part_attrs(Part::IndicatorGroup),
            api.indicator_group_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::Indicator { index: 1 }),
            api.indicator_attrs(1)
        );
        assert_eq!(
            api.part_attrs(Part::AutoPlayTrigger),
            api.auto_play_trigger_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::AutoPlayIndicator),
            api.auto_play_indicator_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::ProgressText),
            api.progress_text_attrs()
        );
    }

    #[test]
    fn goto_slide_blocked_while_transitioning() {
        let mut service = service(props(4));

        drop(service.send(Event::GoToSlide { index: 1 }));

        assert_eq!(service.state(), &State::Transitioning);

        let result = service.send(Event::GoToSlide { index: 3 });

        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn goto_prev_with_stop_on_interaction_marks_stopped() {
        let mut service = service(Props {
            default_index: Some(2),
            ..autoplay_props(3)
        });

        drop(service.send(Event::GoToPrev));

        assert!(service.context().auto_play_stopped);
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn autoplay_pause_when_not_playing_sets_flag_without_state_change() {
        // An auto-play carousel mid-transition (not in `AutoPlaying`): pause
        // keeps the state but records the paused flag and cancels the timer.
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        drop(service.send(Event::AutoPlayTick));
        assert_eq!(service.state(), &State::Transitioning);

        let result = service.send(Event::AutoPlayPause);

        assert_eq!(service.state(), &State::Transitioning);
        assert!(service.context().auto_play_paused);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn autoplay_resume_without_config_is_noop() {
        // The paused/stopped flags are never set while auto-play is absent, so
        // a stray `AutoPlayResume` has nothing to do.
        let mut service = service(props(3));
        let result = service.send(Event::AutoPlayResume);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert!(!service.context().auto_play_paused);
    }

    // ── Codex review #716: autoplay timer lifecycle ──────────────────

    #[test]
    fn manual_next_with_stop_on_interaction_cancels_timer() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        let result = service.send(Event::GoToNext);
        assert!(service.context().auto_play_stopped);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
        // Once stopped, settling the transition rests in Idle (no leaked timer).
        settle(&mut service);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn manual_prev_with_stop_on_interaction_cancels_timer() {
        let mut service = service(Props {
            default_index: Some(2),
            ..autoplay_props(3)
        });
        drop(service.take_initial_effects());
        let result = service.send(Event::GoToPrev);
        assert!(service.context().auto_play_stopped);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn goto_slide_with_stop_on_interaction_stops_and_cancels() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        let result = service.send(Event::GoToSlide { index: 2 });
        assert_eq!(service.context().current_index(), 2);
        assert!(service.context().auto_play_stopped);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn manual_nav_without_stop_on_interaction_keeps_timer() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions {
                stop_on_interaction: false,
                ..AutoPlayOptions::default()
            }),
            ..props(3)
        });
        drop(service.take_initial_effects());
        let result = service.send(Event::GoToNext);
        assert!(!service.context().auto_play_stopped);
        assert!(result.cancel_effects.is_empty());
        // Auto-play resumes once the transition settles.
        settle(&mut service);
        assert_eq!(service.state(), &State::AutoPlaying);
    }

    #[test]
    fn swipe_navigation_with_stop_on_interaction_stops_rotation() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 40.0,
            timestamp: 1000.0,
        }));
        let result = service.send(Event::PointerUp);
        assert_eq!(service.context().current_index(), 1);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().auto_play_stopped);
        // Index changed → IndexChange notification, but rotation is stopped so
        // no timer is re-armed.
        assert!(pending_effect_names(&result).contains(&Effect::IndexChange));
        assert!(!pending_effect_names(&result).contains(&Effect::AutoPlayTimer));
    }

    #[test]
    fn swipe_navigation_resumes_when_stop_on_interaction_disabled() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions {
                stop_on_interaction: false,
                ..AutoPlayOptions::default()
            }),
            ..props(3)
        });
        drop(service.take_initial_effects());
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 40.0,
            timestamp: 1000.0,
        }));
        let result = service.send(Event::PointerUp);
        assert_eq!(service.context().current_index(), 1);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_stopped);
        // Rotation resumes (timer re-armed) and the index change is notified.
        assert!(pending_effect_names(&result).contains(&Effect::AutoPlayTimer));
        assert!(pending_effect_names(&result).contains(&Effect::IndexChange));
    }

    #[test]
    fn non_navigating_drag_resumes_autoplay() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        // Slow, below-threshold drag → no navigation, no stop.
        drop(service.send(Event::PointerMove {
            pos: 90.0,
            timestamp: 1000.0,
        }));
        let result = service.send(Event::PointerUp);
        assert_eq!(service.context().current_index(), 0);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_stopped);
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn focus_pause_leaves_autoplaying_and_cancels_timer() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        let result = service.send(Event::FocusSlide { index: 1 });
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().auto_play_paused);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn item_group_live_is_polite_when_paused() {
        let mut service = service(autoplay_props(3));
        drop(service.send(Event::AutoPlayPause));
        assert_eq!(
            service
                .connect(&|_| {})
                .item_group_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Live)),
            Some("polite")
        );
    }

    // ── Codex review #716: init clamp + multi-slide-view ─────────────

    #[test]
    fn init_clamps_out_of_range_default_index() {
        let service = service(Props {
            default_index: Some(99),
            ..props(3)
        });
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn last_index_accounts_for_slides_per_view() {
        let ctx = service(Props {
            slides_per_view: Some(2.0),
            ..props(3)
        })
        .context()
        .clone();
        // 3 slides, 2 visible → last full page starts at index 1.
        assert_eq!(ctx.last_index(), 1);
        assert_eq!(ctx.visible_count(), 2);
    }

    #[test]
    fn can_go_next_stops_at_last_full_page() {
        let mut service = service(Props {
            slides_per_view: Some(2.0),
            default_index: Some(1),
            ..props(3)
        });
        assert!(!service.context().can_go_next());
        let api_disabled = service
            .connect(&|_| {})
            .next_trigger_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::Disabled))
            == Some("true");
        assert!(api_disabled);
        // Navigation past the last full page is rejected.
        let result = service.send(Event::GoToNext);
        assert!(!result.state_changed);
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn multi_view_keeps_visible_slides_accessible() {
        let service = service(Props {
            slides_per_view: Some(2.0),
            ..props(3)
        });
        let api = service.connect(&|_| {});
        // index 0 with two visible: slides 0 and 1 are on-screen.
        let slide0 = api.item_attrs(0);
        let slide1 = api.item_attrs(1);
        let slide2 = api.item_attrs(2);
        assert_eq!(slide0.get(&HtmlAttr::Aria(AriaAttr::Hidden)), None);
        assert_eq!(slide1.get(&HtmlAttr::Aria(AriaAttr::Hidden)), None);
        assert_eq!(slide1.get(&HtmlAttr::Inert), None);
        // slide 2 is off-screen → hidden + inert.
        assert_eq!(slide2.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn end_key_jumps_to_last_full_page() {
        let service = service(Props {
            slides_per_view: Some(2.0),
            ..props(3)
        });
        assert_eq!(
            captured_keydown(&service, KeyboardKey::End),
            vec![Event::GoToSlide { index: 1 }]
        );
    }

    // ── Codex review #716 (second pass) ──────────────────────────────

    #[test]
    fn controlled_index_syncs_from_props() {
        let mut service = service(Props {
            index: Some(Bindable::controlled(0)),
            ..props(4)
        });
        assert_eq!(service.context().current_index(), 0);

        // Parent pushes a new controlled value via set_props.
        drop(service.set_props(Props {
            index: Some(Bindable::controlled(2)),
            ..props(4)
        }));
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn slides_per_move_zero_is_clamped_to_one() {
        let mut service = service(Props {
            slides_per_move: Some(0),
            ..props(4)
        });
        assert_eq!(service.context().slides_per_move, 1);
        drop(service.send(Event::GoToNext));
        assert_eq!(service.context().current_index(), 1);
    }

    #[test]
    fn slides_per_view_zero_is_normalized() {
        let ctx = service(Props {
            slides_per_view: Some(0.0),
            ..props(3)
        })
        .context()
        .clone();
        assert_eq!(ctx.slides_per_view, 1.0);
        assert!(ctx.track_offset_percent(200.0).is_finite());
    }

    #[test]
    fn slides_per_view_non_finite_is_normalized() {
        let ctx = service(Props {
            slides_per_view: Some(f64::NAN),
            ..props(3)
        })
        .context()
        .clone();
        assert_eq!(ctx.slides_per_view, 1.0);
    }

    #[test]
    fn pointer_cancel_resumes_autoplay() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        let result = service.send(Event::PointerCancel);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(service.context().drag_start_pos.is_none());
        assert_eq!(pending_effect_names(&result), vec![Effect::AutoPlayTimer]);
    }

    #[test]
    fn pointer_cancel_without_autoplay_only_clears_drag() {
        let mut service = service(props(3));
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        let result = service.send(Event::PointerCancel);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().drag_start_pos.is_none());
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn auto_play_trigger_not_playing_without_config() {
        let service = service(props(3));
        let trigger = service.connect(&|_| {}).auto_play_trigger_attrs();
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Pressed)),
            Some("false")
        );
        let indicator = service.connect(&|_| {}).auto_play_indicator_attrs();
        assert_eq!(indicator.get(&HtmlAttr::Data("ars-state")), Some("paused"));
    }

    #[test]
    fn autoplay_tick_ignored_at_last_page_non_looping() {
        let mut service = service(Props {
            default_index: Some(2),
            ..autoplay_props(3)
        });
        drop(service.take_initial_effects());
        // At the last slide of a non-looping carousel, a tick cannot advance,
        // so it must not enter Transitioning (which could stall).
        let result = service.send(Event::AutoPlayTick);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert_eq!(service.context().current_index(), 2);
    }

    // ── Codex review #716 (third pass) ───────────────────────────────

    #[test]
    fn init_clamps_out_of_range_controlled_index() {
        let service = service(Props {
            index: Some(Bindable::controlled(100)),
            ..props(3)
        });
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn controlled_to_uncontrolled_via_props() {
        let mut service = service(Props {
            index: Some(Bindable::controlled(1)),
            ..props(4)
        });
        assert!(service.context().index.is_controlled());

        // Parent drops controlled mode.
        drop(service.set_props(props(4)));
        assert!(!service.context().index.is_controlled());

        // Navigation now actually moves the visible index.
        drop(service.send(Event::GoToNext));
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn autoplay_disabled_via_props_cancels_timer() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        let result = service.set_props(props(3));
        assert!(service.context().auto_play.is_none());
        assert_eq!(service.state(), &State::Idle);
        assert!(result.cancel_effects.contains(&Effect::AutoPlayTimer));
    }

    #[test]
    fn autoplay_enabled_via_props_starts_timer() {
        let mut service = service(props(3));
        assert_eq!(service.state(), &State::Idle);
        let result = service.set_props(autoplay_props(3));
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoPlayTimer)
        );
    }

    #[test]
    fn autoplay_tick_noop_for_looped_single_slide() {
        let mut service = service(Props {
            loop_nav: true,
            ..autoplay_props(1)
        });
        drop(service.take_initial_effects());
        let result = service.send(Event::AutoPlayTick);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::AutoPlaying);
    }

    #[test]
    fn goto_slide_to_current_index_is_noop() {
        let mut service = service(props(3));
        let result = service.send(Event::GoToSlide { index: 0 });
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn goto_slide_to_current_does_not_stop_autoplay() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        // Clicking the already-active indicator is a no-op — it must not stop
        // rotation or strand the machine in Transitioning.
        let result = service.send(Event::GoToSlide { index: 0 });
        assert!(!result.state_changed);
        assert!(!service.context().auto_play_stopped);
        assert_eq!(service.state(), &State::AutoPlaying);
    }

    // ── Codex review #716 (fourth pass) ──────────────────────────────

    #[test]
    fn autoplay_pause_without_config_is_noop() {
        let mut service = service(props(3));
        let result = service.send(Event::AutoPlayPause);
        assert!(!result.state_changed);
        assert!(!service.context().auto_play_paused);
    }

    #[test]
    fn auto_play_trigger_click_without_config_dispatches_nothing() {
        let service = service(props(3));
        let recorder: RefCell<Vec<Event>> = RefCell::new(Vec::new());
        {
            let record = |event| recorder.borrow_mut().push(event);
            let api = service.connect(&record);
            api.on_auto_play_trigger_click();
        }
        assert!(recorder.into_inner().is_empty());
    }

    #[test]
    fn enabling_autoplay_after_trigger_click_starts_timer() {
        // Regression: clicking the trigger while auto-play is off must not set a
        // stale paused flag that suppresses the timer once autoplay is enabled.
        let mut service = service(props(3));
        {
            let api = service.connect(&|_| {});
            api.on_auto_play_trigger_click();
        }
        assert!(!service.context().auto_play_paused);

        let result = service.set_props(autoplay_props(3));
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoPlayTimer)
        );
    }

    #[test]
    fn autoplay_start_after_pause_clears_paused_flag() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        drop(service.send(Event::AutoPlayPause));
        assert!(service.context().auto_play_paused);

        let result = service.send(Event::AutoPlayStart);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_paused);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoPlayTimer)
        );
    }

    // ── Codex review #716 (fifth pass) ───────────────────────────────

    #[test]
    fn pointer_up_without_active_drag_is_noop() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        // No preceding PointerDown → must not run resume logic / re-arm a timer.
        let result = service.send(Event::PointerUp);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn swipe_advances_by_slides_per_move() {
        let mut service = service(Props {
            slides_per_move: Some(2),
            ..props(6)
        });
        drop(service.send(Event::PointerDown {
            pos: 100.0,
            timestamp: 0.0,
        }));
        drop(service.send(Event::PointerMove {
            pos: 40.0,
            timestamp: 1000.0,
        }));
        drop(service.send(Event::PointerUp));
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn focus_visible_slide_does_not_scroll() {
        let mut service = service(Props {
            slides_per_view: Some(2.0),
            ..props(3)
        });
        // index 0, slides 0 & 1 visible. Focusing slide 1 must not move.
        let result = service.send(Event::FocusSlide { index: 1 });
        assert!(!result.context_changed);
        assert_eq!(service.context().current_index(), 0);
    }

    #[test]
    fn focus_offscreen_slide_scrolls_into_view() {
        let mut service = service(Props {
            slides_per_view: Some(2.0),
            ..props(4)
        });
        // index 0, window {0,1}. Focusing slide 3 (off-screen) scrolls to the
        // last full page (index 2).
        drop(service.send(Event::FocusSlide { index: 3 }));
        assert_eq!(service.context().current_index(), 2);
    }

    #[test]
    fn loop_controls_disabled_with_single_position() {
        // Single slide, looping: no distinct target, so both controls disabled.
        let ctx = service(Props {
            loop_nav: true,
            ..props(1)
        })
        .context()
        .clone();
        assert_eq!(ctx.last_index(), 0);
        assert!(!ctx.can_go_prev());
        assert!(!ctx.can_go_next());

        // slides_per_view covering every slide: likewise nowhere to move.
        let ctx_all = service(Props {
            loop_nav: true,
            slides_per_view: Some(3.0),
            ..props(3)
        })
        .context()
        .clone();
        assert_eq!(ctx_all.last_index(), 0);
        assert!(!ctx_all.can_go_next());
    }

    /// Run every pending effect from `result`, then return the captured values.
    fn captured_index_changes(service: &Service<Machine>, event: Event) -> Vec<usize> {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&changes);
        // The on_index_change callback lives in props; build a fresh service so
        // the effect can read it. (Caller passes a service already wired.)
        let mut service = Service::<Machine>::new(
            Props {
                on_index_change: Some(callback(move |index: usize| {
                    captured.lock().expect("lock").push(index);
                })),
                ..service.props().clone()
            },
            &Env::default(),
            &Messages::default(),
        );
        drop(service.take_initial_effects());
        let mut result = service.send(event);
        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in result.pending_effects.drain(..) {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
        changes.lock().expect("lock").clone()
    }

    #[test]
    fn controlled_navigation_round_trips_via_on_index_change() {
        // Controlled carousel: navigation must notify the parent of the
        // requested index so it can push it back through `Props::index`.
        let base = service(Props {
            index: Some(Bindable::controlled(0)),
            ..props(4)
        });
        let changes = captured_index_changes(&base, Event::GoToNext);
        assert_eq!(changes, vec![1]);
    }

    #[test]
    fn manual_navigation_emits_index_change_effect() {
        let mut service = service(props(4));
        let result = service.send(Event::GoToNext);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::IndexChange)
        );
    }

    #[test]
    fn hover_start_pauses_when_pause_on_hover_enabled() {
        let mut service = service(autoplay_props(3));
        drop(service.take_initial_effects());
        let result = service.send(Event::HoverStart);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().auto_play_paused);
        assert_eq!(result.cancel_effects, vec![Effect::AutoPlayTimer]);

        // Pointer leaving resumes rotation.
        let resumed = service.send(Event::HoverEnd);
        assert_eq!(service.state(), &State::AutoPlaying);
        assert!(!service.context().auto_play_paused);
        assert!(
            resumed
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoPlayTimer)
        );
    }

    #[test]
    fn hover_start_is_noop_when_pause_on_hover_disabled() {
        let mut service = service(Props {
            auto_play: Some(AutoPlayOptions {
                pause_on_hover: false,
                ..AutoPlayOptions::default()
            }),
            ..props(3)
        });
        drop(service.take_initial_effects());
        let result = service.send(Event::HoverStart);
        assert!(!result.state_changed);
        assert!(!service.context().auto_play_paused);
        assert_eq!(service.state(), &State::AutoPlaying);
    }

    #[test]
    fn hover_start_without_autoplay_is_noop() {
        let mut service = service(props(3));
        let result = service.send(Event::HoverStart);
        assert!(!result.state_changed);
        assert!(!service.context().auto_play_paused);
    }
}
