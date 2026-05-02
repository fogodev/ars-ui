//! Per-toast state machine.
//!
//! Owns a single toast's lifecycle (`Visible → Paused → Dismissing → Dismissed`),
//! the auto-dismiss countdown intent, the swipe gesture state, and the ARIA /
//! data attribute output for the toast anatomy parts (`Root`, `Title`,
//! `Description`, `ActionTrigger`, `CloseTrigger`, `ProgressBar`).
//!
//! The `aria-live` region shells live one layer up — see
//! [`super::manager::Api::region_attrs`] — because they belong to the
//! manager's lifetime, not any individual toast.
//!
//! The agnostic core never reads `performance.now()`, never installs document
//! listeners, and never measures viewport geometry. Adapters own the clock and
//! hand pause snapshots back atomically through
//! [`Event::Pause`]; auto-dismiss, exit-animation, announcement, and
//! `on_open_change` intents are surfaced as the typed [`Effect`] enum so
//! framework adapters can translate them into `set_timeout`, `announce`, and
//! consumer callbacks (see `spec/components/overlay/toast.md` §1.6).

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, AttrValue, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the per-toast machine
/// emits.
///
/// Adapters dispatch on `effect.name` exhaustively (`match effect.name {
/// toast::single::Effect::DurationTimer => …, … }`) so name typos and
/// unhandled variants surface at compile time — the same convention used by
/// [`dialog::Effect`](crate::overlay::dialog::Effect),
/// [`popover::Effect`](crate::overlay::popover::Effect), and
/// [`tooltip::Effect`](crate::overlay::tooltip::Effect). The variant names
/// themselves are the contract; there is no parallel kebab-case wire form to
/// keep in sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts (or restarts) the auto-dismiss countdown using
    /// `Context::remaining.unwrap_or(Context::duration)` and dispatches
    /// [`Event::DurationExpired`] when the timer fires. Emitted on initial
    /// mount when `duration.is_some()`, on every `Paused → Visible`
    /// transition, and cancelled on `Visible → Paused` and `Visible →
    /// Dismissing` transitions.
    DurationTimer,

    /// Adapter waits for the toast's exit animation to complete (or for the
    /// configured `Provider::remove_delay` when animations are skipped) and
    /// dispatches [`Event::AnimationComplete`]. Emitted on every transition
    /// into [`State::Dismissing`].
    ExitAnimation,

    /// Adapter inserts the toast's title/description into the polite
    /// `aria-live` region. Emitted on initial mount for `Kind::Info`,
    /// `Kind::Success`, and `Kind::Loading` toasts.
    AnnouncePolite,

    /// Adapter inserts the toast's title/description into the assertive
    /// `aria-live` region. Emitted on initial mount for `Kind::Warning` and
    /// `Kind::Error` toasts (per `aria-live="assertive"` urgency).
    AnnounceAssertive,

    /// Adapter invokes consumer-supplied open-change callbacks (e.g.
    /// `manager::Config::on_pause_change`-equivalent open hooks) with the
    /// post-transition open state. Emitted on every transition into
    /// [`State::Dismissing`].
    OpenChange,
}

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Toast`](self) component.
///
/// Toast uses a four-state lifecycle so adapters can distinguish
/// `Visible` (auto-dismiss timer running), `Paused` (timer cancelled but
/// the toast remains on-screen), `Dismissing` (exit animation running,
/// `open` already false for `Presence`), and `Dismissed` (terminal —
/// the surrounding [`Provider`](super::manager) removes the toast).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The toast is on-screen with its auto-dismiss timer running (when a
    /// finite [`Props::duration`] is set) or persistent (when
    /// [`Props::duration`] is `None`, e.g. `Kind::Loading`).
    #[default]
    Visible,

    /// The toast is on-screen with its auto-dismiss timer cancelled because
    /// the pointer is hovering, focus moved into the content, or the
    /// surrounding region requested a global pause.
    Paused,

    /// The toast was dismissed (auto-dismiss, manual dismiss, or swipe) and
    /// the adapter is running its exit animation. `open` is already `false`
    /// so adapters composing with [`Presence`](super::super::presence) can
    /// drive the unmount transition.
    Dismissing,

    /// Terminal state. The surrounding manager observes this and removes the
    /// toast from its visible list.
    Dismissed,
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Toast`](self) state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Programmatic dismissal request (close button click, `DismissAll` from
    /// the surrounding manager, etc.). Always transitions to
    /// [`State::Dismissing`] from either `Visible` or `Paused`.
    Dismiss,

    /// Pause the auto-dismiss countdown. Cancels [`Effect::DurationTimer`]
    /// and atomically records the remaining time so callers cannot observe
    /// a "paused but `remaining == None`" intermediate state.
    Pause {
        /// Remaining auto-dismiss time read from the adapter's clock at the
        /// moment of pause, clamped to zero. The agnostic core writes it
        /// straight into [`Context::remaining`].
        remaining: Duration,
    },

    /// Resume the auto-dismiss countdown. Re-emits [`Effect::DurationTimer`]
    /// so the adapter can restart its `set_timeout` using
    /// `remaining.unwrap_or(duration)`.
    Resume,

    /// The user started swiping the toast. Carries the initial pointer
    /// offset along the swipe axis (px); adapters resolve the axis from the
    /// surrounding manager's [`Placement`](super::manager::Placement).
    SwipeStart(f64),

    /// The pointer moved during a swipe. Carries the latest offset (px).
    SwipeMove(f64),

    /// The user released the pointer. The machine compares
    /// `velocity.abs() > 0.5` and `offset.abs() > Props::swipe_threshold`
    /// to decide whether to dismiss or snap the toast back into place.
    SwipeEnd {
        /// Pointer velocity at release, signed along the swipe axis (px/ms).
        velocity: f64,

        /// Final pointer offset, signed along the swipe axis (px).
        offset: f64,
    },

    /// The auto-dismiss timer fired. Always transitions to
    /// [`State::Dismissing`] from [`State::Visible`].
    DurationExpired,

    /// The exit animation completed (or the configured
    /// `Provider::remove_delay` elapsed when animations are skipped).
    /// Transitions to [`State::Dismissed`].
    AnimationComplete,

    /// Reapply context-relevant fields from the latest [`Props`].
    ///
    /// Auto-emitted by [`Machine::on_props_changed`] whenever the consumer
    /// passes a new `Props` value to `Service::set_props` — the typical
    /// flow when a manager-level `Update(id, ...)` translates into a
    /// `set_props` on the per-toast `Service` (e.g. a loading toast
    /// converting to success/error). Without this, `Context::duration`,
    /// `kind`, `title`, and `description` would stay frozen at their
    /// init-time values; in particular a loading→success transition
    /// would leave `ctx.duration: None`, so `Resume` (driven by manager
    /// `PauseAll`/`ResumeAll`) would never re-emit
    /// [`Effect::DurationTimer`] and the toast would never auto-dismiss.
    ///
    /// Resets [`Context::remaining`] to `None` whenever `duration`
    /// changes — the recorded snapshot reflects the OLD duration's
    /// elapsed time and would scramble the timer math under the new
    /// value.
    SyncProps,
}

// ────────────────────────────────────────────────────────────────────
// Kind
// ────────────────────────────────────────────────────────────────────

/// Toast urgency / appearance category.
///
/// Drives both the `data-ars-kind` attribute on the root and the live-region
/// urgency: `Warning` and `Error` route through the assertive
/// `aria-live="assertive"` region, while `Info`, `Success`, and `Loading`
/// route through the polite region. See `spec/components/overlay/toast.md`
/// §4.1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Generic informational notification (default). Polite urgency.
    #[default]
    Info,

    /// Success notification — operation completed. Polite urgency.
    Success,

    /// Warning notification — non-fatal but worth attention. Assertive urgency.
    Warning,

    /// Error notification — operation failed or invalid state. Assertive urgency.
    Error,

    /// Promise / async pending notification. Polite urgency. Pairs with
    /// `Props::duration = None` so the toast persists until updated.
    Loading,
}

impl Kind {
    /// Returns the wire token used for `data-ars-kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Loading => "loading",
        }
    }

    /// Returns `true` when this kind routes to the assertive live region.
    #[must_use]
    pub const fn is_assertive(self) -> bool {
        matches!(self, Self::Warning | Self::Error)
    }

    /// Returns the announcement priority that matches this kind.
    ///
    /// Mirrors [`is_assertive`](Self::is_assertive) but expresses the
    /// result as the typed [`super::manager::AnnouncePriority`] so adapters
    /// can route directly to the matching live region.
    #[must_use]
    pub const fn announce_priority(self) -> super::manager::AnnouncePriority {
        if self.is_assertive() {
            super::manager::AnnouncePriority::Assertive
        } else {
            super::manager::AnnouncePriority::Polite
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for [`Toast`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,

    /// Hydration-stable component IDs derived from [`Props::id`] at init.
    /// Adapters read sub-part IDs through [`ComponentIds::part`] (e.g.
    /// `ids.part("title")`) so ARIA wiring stays in sync with the rendered
    /// element IDs.
    pub ids: ComponentIds,

    /// Optional title shown in the toast.
    pub title: Option<String>,

    /// Optional description shown beneath the title.
    pub description: Option<String>,

    /// Toast urgency / appearance category.
    pub kind: Kind,

    /// Auto-dismiss duration. `None` means the toast is persistent (no
    /// [`Effect::DurationTimer`] is emitted on mount).
    pub duration: Option<Duration>,

    /// Remaining auto-dismiss time.
    ///
    /// The agnostic core never reads `performance.now()` itself: when the
    /// toast pauses, the adapter computes `duration - elapsed` from its own
    /// clock and the snapshot is recorded atomically through
    /// [`Event::Pause`]. On resume, the adapter restarts its timer using
    /// `remaining.unwrap_or(duration)`.
    pub remaining: Option<Duration>,

    /// Whether the toast is currently paused (mirrors [`State::Paused`]).
    pub paused: bool,

    /// Whether a swipe gesture is currently in progress.
    pub swiping: bool,

    /// Current swipe offset along the placement-derived swipe axis (px).
    pub swipe_offset: f64,

    /// Whether the toast is open (for [`Presence`](super::super::presence)
    /// composition). Set to `false` on transition into [`State::Dismissing`].
    pub open: bool,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Default swipe-to-dismiss threshold in pixels (matches the spec §7.3
/// recommendation).
pub const DEFAULT_SWIPE_THRESHOLD: f64 = 50.0;

/// Immutable configuration for [`Toast`](self).
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id (hydration-stable; used for ARIA wiring and
    /// the `data-*` hooks rendered by [`Api`]).
    pub id: String,

    /// Toast title.
    pub title: Option<String>,

    /// Toast description.
    pub description: Option<String>,

    /// Toast urgency / appearance category.
    pub kind: Kind,

    /// Auto-dismiss duration. `None` makes the toast persistent (typical
    /// for `Kind::Loading`).
    pub duration: Option<Duration>,

    /// Whether to show a progress bar inside the toast (rendered through
    /// [`Api::progress_bar_attrs`]).
    pub show_progress: bool,

    /// Distance threshold (px) past which `SwipeEnd` dismisses the toast.
    /// Velocity above `0.5` also dismisses regardless of distance.
    /// Defaults to [`DEFAULT_SWIPE_THRESHOLD`].
    pub swipe_threshold: f64,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: None,
            description: None,
            kind: Kind::Info,
            duration: Some(Duration::from_secs(5)),
            show_progress: false,
            swipe_threshold: DEFAULT_SWIPE_THRESHOLD,
        }
    }
}

impl Props {
    /// Returns Toast props with documented default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id) to the supplied component instance id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`title`](Self::title).
    #[must_use]
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }

    /// Sets [`description`](Self::description).
    #[must_use]
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets [`kind`](Self::kind).
    #[must_use]
    pub const fn kind(mut self, value: Kind) -> Self {
        self.kind = value;
        self
    }

    /// Sets [`duration`](Self::duration), the auto-dismiss timeout. Pass
    /// `None` to make the toast persistent.
    #[must_use]
    pub const fn duration(mut self, value: Option<Duration>) -> Self {
        self.duration = value;
        self
    }

    /// Sets [`show_progress`](Self::show_progress).
    #[must_use]
    pub const fn show_progress(mut self, value: bool) -> Self {
        self.show_progress = value;
        self
    }

    /// Sets [`swipe_threshold`](Self::swipe_threshold) in pixels.
    #[must_use]
    pub const fn swipe_threshold(mut self, value: f64) -> Self {
        self.swipe_threshold = value;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings for [`Toast`](self).
///
/// Per-toast Messages carry only the dismiss-button label. The
/// `aria-live` region landmark label lives on
/// [`super::manager::Messages`] because it belongs to the surrounding
/// region shell, not any individual toast.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the dismiss button. Defaults to
    /// `"Dismiss notification"`.
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dismiss_label: MessageFn::static_str("Dismiss notification"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Structural parts exposed by the Toast connect API.
///
/// `Region` is **not** a per-toast part — the `aria-live` shells live
/// at the surrounding [`Provider`](super::manager) level. Adapters use
/// [`super::manager::Api::region_attrs`] for those shells.
#[derive(ComponentPart)]
#[scope = "toast"]
pub enum Part {
    /// The root container for a single toast.
    Root,

    /// The optional title element.
    Title,

    /// The optional description element.
    Description,

    /// Optional CTA action button. The `alt_text` payload describes the
    /// full effect of the action (e.g. "Undo message deletion") and is
    /// rendered as `aria-label`.
    ActionTrigger {
        /// Screen-reader description of the action's effect.
        alt_text: String,
    },

    /// Dismiss button rendered inside the toast.
    CloseTrigger,

    /// Optional progress bar reflecting the remaining auto-dismiss time.
    ProgressBar,
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the [`Toast`](self) component.
#[derive(Debug)]
pub struct Machine;

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
        (
            State::Visible,
            Context {
                locale: env.locale.clone(),
                ids: ComponentIds::from_id(&props.id),
                title: props.title.clone(),
                description: props.description.clone(),
                kind: props.kind,
                duration: props.duration,
                remaining: None,
                paused: false,
                swiping: false,
                swipe_offset: 0.0,
                open: true,
                messages: messages.clone(),
            },
        )
    }

    fn initial_effects(
        _state: &Self::State,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        let mut effects = Vec::new();

        effects.push(PendingEffect::named(announce_intent(ctx.kind)));

        if ctx.duration.is_some() {
            effects.push(PendingEffect::named(Effect::DurationTimer));
        }

        effects
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Pause — atomically record the remaining-time snapshot and
            // cancel the running timer.
            (State::Visible, Event::Pause { remaining }) => {
                let remaining = *remaining;
                Some(
                    TransitionPlan::to(State::Paused)
                        .apply(move |ctx: &mut Context| {
                            ctx.paused = true;
                            ctx.remaining = Some(remaining);
                        })
                        .cancel_effect(Effect::DurationTimer),
                )
            }

            // Resume — restart the timer using the recorded `remaining`.
            //
            // Persistent toasts (`duration: None`, typical for
            // `Kind::Loading`) MUST NOT emit `Effect::DurationTimer` here.
            // Adapters following the documented contract
            // (`remaining.unwrap_or(duration)`) would otherwise either
            // schedule a `set_timeout(None)` or auto-dismiss a toast that
            // is supposed to stay until explicitly updated. This fires
            // when manager-level `PauseAll`/`ResumeAll` cycles a
            // persistent toast through `Paused → Visible`.
            (State::Paused, Event::Resume) => {
                let mut plan = TransitionPlan::to(State::Visible).apply(|ctx: &mut Context| {
                    ctx.paused = false;
                });

                if ctx.duration.is_some() {
                    plan = plan.with_effect(PendingEffect::named(Effect::DurationTimer));
                }

                Some(plan)
            }

            // Auto-dismiss or manual dismiss → animate out.
            (State::Visible, Event::DurationExpired | Event::Dismiss)
            | (State::Paused, Event::Dismiss) => Some(dismiss_plan()),

            // Animation complete → final state.
            (State::Dismissing, Event::AnimationComplete) => {
                Some(TransitionPlan::to(State::Dismissed))
            }

            // Swipe gestures — pure context updates while the gesture is
            // in flight; SwipeEnd decides whether to dismiss.
            (State::Visible | State::Paused, Event::SwipeStart(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.swiping = true;
                    ctx.swipe_offset = offset;
                }))
            }

            (State::Visible | State::Paused, Event::SwipeMove(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.swipe_offset = offset;
                }))
            }

            (State::Visible | State::Paused, Event::SwipeEnd { velocity, offset }) => {
                let velocity = *velocity;
                let offset = *offset;
                let threshold = props.swipe_threshold;

                if velocity.abs() > 0.5 || offset.abs() > threshold {
                    // `dismiss_plan` already resets `swiping` /
                    // `swipe_offset` (and `paused`, `open`); see its
                    // doc comment.
                    Some(dismiss_plan())
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.swiping = false;
                        ctx.swipe_offset = 0.0;
                    }))
                }
            }

            // Reapply context-backed prop fields. Valid in any state —
            // adapters can update content while the toast is animating
            // out (it's harmless: Dismissing/Dismissed are about
            // lifecycle, not content). When `duration` actually changes,
            // `remaining` is reset to `None` so the next `Resume` (or
            // `initial_effects`-equivalent restart) starts the timer
            // from the new full duration rather than scrambling the
            // math with a snapshot that referred to the old duration.
            //
            // The promise-toast flow is the motivating case: a `Loading`
            // toast starts with `duration: None` (persistent, no
            // running timer); when the promise resolves the adapter
            // calls `set_props` with `Kind::Success` and a finite
            // duration. Without re-emitting `DurationTimer` from this
            // arm the resolved toast would never auto-dismiss because
            // no `Resume` follows — the toast was never paused. The
            // dual case (finite → `None`) cancels any running timer so
            // a toast switched to persistent doesn't keep its old
            // countdown alive.
            (_, Event::SyncProps) => {
                let title = props.title.clone();
                let description = props.description.clone();
                let kind = props.kind;
                let duration = props.duration;

                let duration_changed = ctx.duration != duration;

                let restart_timer =
                    duration_changed && duration.is_some() && matches!(state, State::Visible);

                let cancel_running_timer =
                    duration_changed && duration.is_none() && matches!(state, State::Visible);

                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.title = title;
                    ctx.description = description;
                    ctx.kind = kind;
                    ctx.duration = duration;

                    if duration_changed {
                        ctx.remaining = None;
                    }
                });

                // When restarting, cancel first so the adapter's
                // effect dispatcher doesn't end up with two live
                // timers if the previous one had not fired yet.
                if restart_timer || cancel_running_timer {
                    plan = plan.cancel_effect(Effect::DurationTimer);
                }

                if restart_timer {
                    plan = plan.with_effect(PendingEffect::named(Effect::DurationTimer));
                }

                Some(plan)
            }

            _ => None,
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // The toast id is baked into `Context::ids` at init and feeds
        // every ARIA / `aria-labelledby` / `aria-describedby`
        // relationship rendered by `Api`. Allowing it to change at
        // runtime would silently break those relationships, so this
        // panic catches the misuse early — the same convention used by
        // Tooltip, Popover, and Dialog.
        assert_eq!(
            old.id, new.id,
            "Toast id cannot change after initialization"
        );

        if context_relevant_props_changed(old, new) {
            alloc::vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Returns `true` when any context-backed prop differs between `old` and
/// `new`. Used by [`Machine::on_props_changed`] to decide whether to
/// emit [`Event::SyncProps`].
///
/// `swipe_threshold` and `show_progress` are read directly from `props`
/// at call time (in `transition` and `Api::progress_bar_attrs`
/// respectively) so they are not mirrored on `Context` and do not need a
/// sync event.
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.title != new.title
        || old.description != new.description
        || old.kind != new.kind
        || old.duration != new.duration
}

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

/// Returns the dismiss transition plan emitted by `Visible|Paused →
/// Dismissing`. Cancels the running auto-dismiss timer, marks the toast
/// closed for `Presence` composition, asks the adapter to run the exit
/// animation, and notifies open-change listeners.
///
/// Every dismiss source converges through this helper (`Event::Dismiss`,
/// `Event::DurationExpired`, swipe-end past threshold, manager-driven
/// dismiss-all, …), so it is also the single point that resets context
/// fields the source-specific arms might have set before:
///
/// * `ctx.paused` is forced to `false` because `State::Dismissing` is
///   not a paused state — the `Context::paused` doc comment promises it
///   mirrors `State::Paused`. Without this reset, dismissing from
///   `State::Paused` would leave `ctx.paused == true` while
///   `state == Dismissing`, and adapters/callbacks reading `ctx.paused`
///   would apply stale pause behaviour during exit.
///
/// * `ctx.swiping` and `ctx.swipe_offset` are cleared because non-swipe
///   dismiss paths (`Event::DurationExpired` while a drag is in
///   progress, close-button dismiss, manager dismiss-all) bypass the
///   `Event::SwipeEnd` arm that normally resets them. Leaving them set
///   would keep swipe-specific positioning/styling active during the
///   exit animation. The swipe-end dismiss arm continues to clear them
///   itself before delegating to this plan; doing it again here is
///   idempotent.
fn dismiss_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Dismissing)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.paused = false;
            ctx.swiping = false;
            ctx.swipe_offset = 0.0;
        })
        .cancel_effect(Effect::DurationTimer)
        .with_effect(PendingEffect::named(Effect::ExitAnimation))
        .with_effect(PendingEffect::named(Effect::OpenChange))
}

/// Returns the announcement effect intent for the given toast kind.
const fn announce_intent(kind: Kind) -> Effect {
    if kind.is_assertive() {
        Effect::AnnounceAssertive
    } else {
        Effect::AnnouncePolite
    }
}

/// Returns the wire token for `data-ars-state`.
const fn state_token(state: State) -> &'static str {
    match state {
        State::Visible => "visible",
        State::Paused => "paused",
        State::Dismissing => "dismissing",
        State::Dismissed => "dismissed",
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connected API surface for a single [`Toast`](self).
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
            .finish()
    }
}

impl Api<'_> {
    /// Returns `true` when the toast is in [`State::Visible`] or [`State::Paused`].
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        matches!(self.state, State::Visible | State::Paused)
    }

    /// Returns `true` when the toast is in [`State::Paused`].
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        matches!(self.state, State::Paused)
    }

    /// Returns `true` when the toast is in [`State::Dismissing`] or
    /// [`State::Dismissed`].
    #[must_use]
    pub const fn is_dismissed(&self) -> bool {
        matches!(self.state, State::Dismissing | State::Dismissed)
    }

    /// Returns the toast's [`Kind`].
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.ctx.kind
    }

    /// Returns the configured swipe-to-dismiss threshold (px).
    #[must_use]
    pub const fn swipe_threshold(&self) -> f64 {
        self.props.swipe_threshold
    }

    /// Returns attributes for the toast root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id().to_string())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), state_token(*self.state))
            .set(HtmlAttr::Data("ars-kind"), self.ctx.kind.as_str());

        // Wire title/description as labelled-by / described-by so the toast
        // landmark exposes its own subject and body to assistive tech.
        if self.ctx.title.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            );
        }
        if self.ctx.description.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }

        if self.ctx.swiping {
            attrs.set_bool(HtmlAttr::Data("ars-swiping"), true);
        }

        attrs
    }

    /// Returns attributes for the title element.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("title"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("description"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the optional CTA action button.
    ///
    /// The supplied `alt_text` describes the full effect of the action for
    /// screen readers (e.g. "Undo message deletion") and is rendered as
    /// `aria-label`. Mirrors Radix's `Toast.Action` `altText` requirement.
    #[must_use]
    pub fn action_trigger_attrs(&self, alt_text: impl Into<AttrValue>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ActionTrigger {
            alt_text: String::new(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), alt_text);

        attrs
    }

    /// Returns attributes for the dismiss / close button.
    #[must_use]
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();

        let label = (self.ctx.messages.dismiss_label)(&self.ctx.locale);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    /// Returns attributes for the optional progress-bar element.
    ///
    /// The progress bar is presentational — `aria-valuenow` is intentionally
    /// **not** emitted because per-frame ARIA updates would defeat the
    /// goal of "screen readers do NOT announce progress" called out in
    /// `spec/components/overlay/toast.md` §7.4. Adapters drive the visual
    /// progress through the `--ars-toast-progress` CSS custom property.
    #[must_use]
    pub fn progress_bar_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ProgressBar.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "progressbar")
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), "0")
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), "100");

        attrs
    }

    /// Dispatches a programmatic dismiss request (typically wired to the
    /// close button's `click` handler).
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Dismiss);
    }

    /// Dispatches a pause request (wired to `pointerenter`/`focusin`),
    /// carrying the remaining-time snapshot the adapter just read from
    /// its own clock.
    pub fn on_pointer_enter(&self, remaining: Duration) {
        (self.send)(Event::Pause { remaining });
    }

    /// Dispatches a resume request (wired to `pointerleave`/`focusout`).
    pub fn on_pointer_leave(&self) {
        (self.send)(Event::Resume);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::ActionTrigger { alt_text } => self.action_trigger_attrs(&alt_text),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::ProgressBar => self.progress_bar_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, vec::Vec};
    use core::cell::RefCell;

    use ars_core::{ConnectApi, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "toast".to_string(),
            title: Some("Saved".to_string()),
            description: Some("Your changes were saved.".to_string()),
            ..Props::default()
        }
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn snapshot_api(api: &Api<'_>) -> String {
        format!(
            "root:\n{:#?}\ntitle:\n{:#?}\ndescription:\n{:#?}\naction_trigger:\n{:#?}\nclose_trigger:\n{:#?}\nprogress_bar:\n{:#?}",
            api.root_attrs(),
            api.title_attrs(),
            api.description_attrs(),
            api.action_trigger_attrs("Undo deletion"),
            api.close_trigger_attrs(),
            api.progress_bar_attrs(),
        )
    }

    // ── Props builder coverage ─────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("toast-builder")
            .title("Saved")
            .description("Your changes were saved.")
            .kind(Kind::Success)
            .duration(Some(Duration::from_secs(8)))
            .show_progress(true)
            .swipe_threshold(80.0);

        assert_eq!(props.id, "toast-builder");
        assert_eq!(props.title.as_deref(), Some("Saved"));
        assert_eq!(
            props.description.as_deref(),
            Some("Your changes were saved.")
        );
        assert_eq!(props.kind, Kind::Success);
        assert_eq!(props.duration, Some(Duration::from_secs(8)));
        assert!(props.show_progress);
        assert_eq!(props.swipe_threshold, 80.0);
    }

    #[test]
    fn props_default_swipe_threshold_matches_constant() {
        assert_eq!(Props::default().swipe_threshold, DEFAULT_SWIPE_THRESHOLD);
    }

    #[test]
    fn props_duration_none_marks_persistent() {
        let props = Props::new().duration(None);

        assert!(props.duration.is_none());
    }

    #[test]
    fn kind_as_str_matches_data_token() {
        for (kind, expected) in [
            (Kind::Info, "info"),
            (Kind::Success, "success"),
            (Kind::Warning, "warning"),
            (Kind::Error, "error"),
            (Kind::Loading, "loading"),
        ] {
            assert_eq!(kind.as_str(), expected);
        }
    }

    #[test]
    fn kind_assertive_routes_warnings_and_errors() {
        assert!(Kind::Warning.is_assertive());
        assert!(Kind::Error.is_assertive());
        assert!(!Kind::Info.is_assertive());
        assert!(!Kind::Success.is_assertive());
        assert!(!Kind::Loading.is_assertive());
    }

    #[test]
    fn kind_announce_priority_matches_assertive_flag() {
        use super::super::manager::AnnouncePriority;

        for kind in [Kind::Info, Kind::Success, Kind::Loading] {
            assert_eq!(kind.announce_priority(), AnnouncePriority::Polite);
        }

        for kind in [Kind::Warning, Kind::Error] {
            assert_eq!(kind.announce_priority(), AnnouncePriority::Assertive);
        }
    }

    // ── init / initial_effects coverage ───────────────────────────

    #[test]
    fn init_starts_visible_with_open_context() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Visible);

        let ctx = service.context();

        assert!(ctx.open);
        assert!(!ctx.paused);
        assert!(!ctx.swiping);
        assert_eq!(ctx.swipe_offset, 0.0);
        assert_eq!(ctx.duration, Some(Duration::from_secs(5)));
        assert!(ctx.remaining.is_none());
        assert_eq!(ctx.ids.id(), "toast");
    }

    #[test]
    fn initial_effects_emit_polite_announce_and_timer_for_info_with_duration() {
        let mut service = fresh_service(test_props());

        let effects = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert_eq!(effects, vec![Effect::AnnouncePolite, Effect::DurationTimer]);
    }

    #[test]
    fn initial_effects_emit_assertive_announce_for_error_kind() {
        let mut service = fresh_service(Props {
            kind: Kind::Error,
            ..test_props()
        });

        let effects = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert_eq!(
            effects,
            vec![Effect::AnnounceAssertive, Effect::DurationTimer]
        );
    }

    #[test]
    fn initial_effects_skip_timer_when_duration_is_none() {
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        let effects = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert_eq!(effects, vec![Effect::AnnouncePolite]);
    }

    #[test]
    fn initial_effects_drain_exactly_once() {
        let mut service = fresh_service(test_props());

        assert!(!service.take_initial_effects().is_empty());
        assert!(service.take_initial_effects().is_empty());
    }

    // ── Pause / Resume coverage ────────────────────────────────────

    #[test]
    fn pause_records_remaining_and_cancels_duration_timer() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        let result = service.send(Event::Pause {
            remaining: Duration::from_millis(2_750),
        });

        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().paused);
        assert_eq!(
            service.context().remaining,
            Some(Duration::from_millis(2_750))
        );
        assert_eq!(result.cancel_effects, vec![Effect::DurationTimer]);
        assert!(effect_names(&result).is_empty());
    }

    #[test]
    fn resume_restarts_duration_timer_and_clears_paused() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(2_500),
        }));

        let result = service.send(Event::Resume);

        assert_eq!(service.state(), &State::Visible);
        assert!(!service.context().paused);

        // `remaining` is preserved so the adapter can read it from the
        // context inside the DurationTimer effect.
        assert_eq!(
            service.context().remaining,
            Some(Duration::from_millis(2_500))
        );
        assert_eq!(effect_names(&result), vec![Effect::DurationTimer]);
    }

    /// Regression test for the P1.B review finding: `Paused → Visible`
    /// previously emitted `Effect::DurationTimer` unconditionally. A
    /// persistent toast (`duration: None`, typical for `Kind::Loading`)
    /// caught up in a manager-level `PauseAll` / `ResumeAll` cycle would
    /// receive a bogus timer-restart intent. Adapters following the
    /// documented contract (`remaining.unwrap_or(duration)`) would then
    /// either auto-dismiss a toast that's supposed to stay until updated
    /// or schedule a `set_timeout(None)`-equivalent.
    #[test]
    fn resume_persistent_toast_does_not_restart_timer() {
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::ZERO,
        }));

        let result = service.send(Event::Resume);

        assert_eq!(service.state(), &State::Visible);
        assert!(!service.context().paused);
        assert!(
            !effect_names(&result).contains(&Effect::DurationTimer),
            "persistent toasts (duration: None) MUST NOT emit DurationTimer on Resume"
        );

        // Stage flipped, but no timer-related effects.
        assert!(result.pending_effects.is_empty());
    }

    /// Regression test for the P1 review finding: per-toast `Context`
    /// snapshotted `title`, `description`, `kind`, and `duration` from
    /// `Props` only at `init`. If the adapter pushed a new `Props`
    /// value via `Service::set_props` (the typical translation of a
    /// manager-level `Update(id, ...)`), those `Context` fields stayed
    /// frozen — so a loading toast converting to success/error would
    /// keep `ctx.duration: None` and never auto-dismiss after Resume.
    /// The fix: `on_props_changed` synthesizes `Event::SyncProps`,
    /// which copies the four context-backed prop fields into `Context`.
    #[test]
    fn set_props_syncs_title_description_kind_and_duration() {
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            title: Some("Saving".to_string()),
            description: Some("loading body".to_string()),
            ..test_props()
        });

        drop(service.take_initial_effects());

        // Pre-condition: ctx mirrors the loading props.
        assert_eq!(service.context().kind, Kind::Loading);
        assert_eq!(service.context().duration, None);
        assert_eq!(service.context().title.as_deref(), Some("Saving"));

        // Loading → success conversion via `set_props`.
        drop(service.set_props(Props {
            duration: Some(Duration::from_secs(5)),
            kind: Kind::Success,
            title: Some("Saved".to_string()),
            description: Some("Your changes were saved.".to_string()),
            ..test_props()
        }));

        let ctx = service.context();

        assert_eq!(ctx.kind, Kind::Success, "kind must mirror new props");
        assert_eq!(
            ctx.duration,
            Some(Duration::from_secs(5)),
            "duration must mirror new props"
        );
        assert_eq!(ctx.title.as_deref(), Some("Saved"));
        assert_eq!(ctx.description.as_deref(), Some("Your changes were saved."));
    }

    /// Sibling P1 regression: a loading toast that converts to success
    /// (`duration: None` → `Some(_)`) via `set_props` MUST emit
    /// `DurationTimer` on the next `Resume`. Before the fix
    /// `ctx.duration` stayed `None`, so the per-toast machine silently
    /// dropped the timer-restart effect.
    #[test]
    fn visible_loading_to_success_via_set_props_emits_duration_timer_immediately() {
        // Round-10 regression: the canonical promise-toast flow has no
        // intervening Pause/Resume. The user calls
        // `toaster.success(promise_id, …)` while the loading toast is
        // still `Visible`. Without re-emitting `DurationTimer` from
        // SyncProps, the resolved toast would never auto-dismiss
        // because no `Resume` follows.
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        drop(service.take_initial_effects());
        assert_eq!(service.state(), &State::Visible);

        let result = service.set_props(Props {
            duration: Some(Duration::from_secs(5)),
            kind: Kind::Success,
            ..test_props()
        });

        assert_eq!(
            service.context().duration,
            Some(Duration::from_secs(5)),
            "SyncProps mirrored the new finite duration"
        );

        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();
        assert!(
            names.contains(&Effect::DurationTimer),
            "Visible loading→success must emit DurationTimer immediately, got {names:?}"
        );
        // The cancel ensures any straggler from a prior duration is
        // dropped before the new timer arms.
        assert!(
            result.cancel_effects.contains(&Effect::DurationTimer),
            "the prior timer (if any) must be cancelled before re-arming"
        );
    }

    #[test]
    fn visible_finite_to_persistent_via_set_props_cancels_running_timer() {
        // Symmetric case: a Visible toast with a running auto-dismiss
        // timer is converted to persistent (`duration: None`). The
        // running timer must be cancelled so the toast doesn't
        // unexpectedly disappear after the old duration elapses.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        let result = service.set_props(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();
        assert!(
            !names.contains(&Effect::DurationTimer),
            "switching to persistent must NOT re-arm the timer; got {names:?}"
        );
        assert!(
            result.cancel_effects.contains(&Effect::DurationTimer),
            "the running timer must be cancelled when duration becomes None"
        );
        assert_eq!(service.context().duration, None);
    }

    #[test]
    fn visible_finite_to_finite_via_set_props_restarts_timer() {
        // Duration changes between two finite values while Visible —
        // the existing timer is for the *old* duration, so it must be
        // cancelled and a fresh one armed for the new value.
        let mut service = fresh_service(Props {
            duration: Some(Duration::from_secs(2)),
            ..test_props()
        });

        drop(service.take_initial_effects());

        let result = service.set_props(Props {
            duration: Some(Duration::from_secs(10)),
            ..test_props()
        });

        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();
        assert!(names.contains(&Effect::DurationTimer));
        assert!(result.cancel_effects.contains(&Effect::DurationTimer));
        assert_eq!(service.context().duration, Some(Duration::from_secs(10)));
    }

    #[test]
    fn paused_loading_to_success_does_not_emit_duration_timer_eagerly() {
        // While Paused, SyncProps must NOT eagerly emit DurationTimer
        // — the toast is paused and timers should only resume on the
        // explicit `Resume` event. This guards against the fix
        // accidentally firing the timer in the wrong state.
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::ZERO,
        }));
        assert_eq!(service.state(), &State::Paused);

        let result = service.set_props(Props {
            duration: Some(Duration::from_secs(5)),
            kind: Kind::Success,
            ..test_props()
        });

        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();
        assert!(
            !names.contains(&Effect::DurationTimer),
            "Paused state must not eagerly arm the timer; got {names:?}"
        );
        // The earlier `loading_to_success_via_set_props_now_restarts_timer_on_resume`
        // test still locks in that a subsequent Resume does fire it.
    }

    #[test]
    fn dismissing_via_set_props_does_not_arm_timer() {
        // SyncProps in Dismissing state must never arm a timer — the
        // toast is animating out and a new timer would extend its
        // lifetime past the exit animation.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Dismiss));
        assert_eq!(service.state(), &State::Dismissing);

        let result = service.set_props(Props {
            duration: Some(Duration::from_secs(99)),
            kind: Kind::Success,
            ..test_props()
        });

        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();
        assert!(!names.contains(&Effect::DurationTimer));
    }

    #[test]
    fn loading_to_success_via_set_props_now_restarts_timer_on_resume() {
        let mut service = fresh_service(Props {
            duration: None,
            kind: Kind::Loading,
            ..test_props()
        });

        drop(service.take_initial_effects());

        // Pause first (manager-level PauseAll equivalent). Persistent
        // toasts ignore the timer, so cancel_effects is harmless.
        drop(service.send(Event::Pause {
            remaining: Duration::ZERO,
        }));

        // Convert to success via set_props.
        drop(service.set_props(Props {
            duration: Some(Duration::from_secs(5)),
            kind: Kind::Success,
            ..test_props()
        }));

        assert_eq!(
            service.context().duration,
            Some(Duration::from_secs(5)),
            "SyncProps mirrored the new finite duration"
        );

        // Resume now emits DurationTimer because ctx.duration is finite.
        let resume = service.send(Event::Resume);

        assert!(
            effect_names(&resume).contains(&Effect::DurationTimer),
            "after loading→success conversion, Resume must emit DurationTimer"
        );
    }

    /// `SyncProps` resets `remaining` whenever `duration` changes — the
    /// recorded snapshot reflected the old duration's elapsed time and
    /// would scramble timer math under the new value.
    #[test]
    fn set_props_resets_remaining_when_duration_changes() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        // Pause to record a remaining snapshot.
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(2_500),
        }));

        assert_eq!(
            service.context().remaining,
            Some(Duration::from_millis(2_500))
        );

        // set_props with a different duration → remaining MUST reset.
        drop(service.set_props(Props {
            duration: Some(Duration::from_secs(10)),
            ..test_props()
        }));

        assert_eq!(
            service.context().remaining,
            None,
            "remaining must reset when duration changes — the snapshot was for the old duration"
        );
        assert_eq!(service.context().duration, Some(Duration::from_secs(10)));
    }

    /// Symmetric: `SyncProps` preserves `remaining` when `duration` is
    /// unchanged — only content fields (title/description/kind) move.
    /// A pause snapshot mid-flight stays valid through a content edit.
    #[test]
    fn set_props_preserves_remaining_when_duration_unchanged() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(2_500),
        }));

        // set_props with same duration but different title.
        drop(service.set_props(Props {
            title: Some("New title".to_string()),
            ..test_props()
        }));

        assert_eq!(
            service.context().remaining,
            Some(Duration::from_millis(2_500)),
            "remaining stays put when duration is unchanged"
        );
        assert_eq!(service.context().title.as_deref(), Some("New title"));
    }

    /// Identical-Props `set_props` is a no-op: `on_props_changed`
    /// returns an empty event list, so context isn't perturbed.
    #[test]
    fn set_props_with_identical_props_is_a_noop() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        let before_ctx = service.context().clone();
        let result = service.set_props(test_props());

        assert!(!result.context_changed);
        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context(), &before_ctx);
    }

    /// Workspace convention: ids are baked into `Context::ids` at init
    /// and feed every ARIA relationship rendered by `Api`. Mutating
    /// the id at runtime would silently break those relationships,
    /// so `on_props_changed` panics. Mirrors Tooltip / Popover / Dialog.
    #[test]
    #[should_panic(expected = "Toast id cannot change after initialization")]
    fn set_props_panics_when_id_changes() {
        let mut service = fresh_service(test_props());
        drop(service.take_initial_effects());

        drop(service.set_props(Props {
            id: "different-id".to_string(),
            ..test_props()
        }));
    }

    #[test]
    fn pause_in_dismissing_state_is_no_op() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Dismiss));

        let result = service.send(Event::Pause {
            remaining: Duration::from_millis(1_000),
        });

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!result.state_changed);
        assert!(!result.context_changed);
    }

    // ── Dismiss / animation coverage ──────────────────────────────

    #[test]
    fn duration_expired_dismisses_visible_toast() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        let result = service.send(Event::DurationExpired);

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().open);
        assert_eq!(
            effect_names(&result),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
        assert_eq!(result.cancel_effects, vec![Effect::DurationTimer]);
    }

    #[test]
    fn dismiss_from_paused_state_dismisses_toast() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(1_000),
        }));

        let result = service.send(Event::Dismiss);

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().open);
        assert_eq!(
            effect_names(&result),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
    }

    #[test]
    fn animation_complete_reaches_dismissed() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Dismiss));

        let result = service.send(Event::AnimationComplete);

        assert_eq!(service.state(), &State::Dismissed);
        assert!(result.state_changed);
    }

    #[test]
    fn duration_expired_in_paused_state_is_ignored() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(500),
        }));

        let result = service.send(Event::DurationExpired);

        assert_eq!(service.state(), &State::Paused);
        assert!(!result.state_changed);
    }

    // ── Swipe coverage ─────────────────────────────────────────────

    #[test]
    fn swipe_start_marks_swiping_and_records_offset() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        let result = service.send(Event::SwipeStart(12.0));

        assert_eq!(service.state(), &State::Visible);
        assert!(service.context().swiping);
        assert_eq!(service.context().swipe_offset, 12.0);
        assert!(result.context_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn swipe_move_updates_offset_only() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));

        drop(service.send(Event::SwipeMove(40.0)));

        assert!(service.context().swiping);
        assert_eq!(service.context().swipe_offset, 40.0);
    }

    #[test]
    fn swipe_end_under_threshold_resets_offset() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(20.0)));

        let result = service.send(Event::SwipeEnd {
            velocity: 0.1,
            offset: 20.0,
        });

        assert_eq!(service.state(), &State::Visible);
        assert!(!service.context().swiping);
        assert_eq!(service.context().swipe_offset, 0.0);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn swipe_end_over_threshold_dismisses() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(80.0)));

        let result = service.send(Event::SwipeEnd {
            velocity: 0.0,
            offset: 80.0,
        });

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().swiping);
        assert_eq!(service.context().swipe_offset, 0.0);
        assert_eq!(
            effect_names(&result),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
    }

    #[test]
    fn swipe_end_high_velocity_dismisses_below_threshold() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(20.0)));

        let result = service.send(Event::SwipeEnd {
            velocity: 1.5,
            offset: 20.0,
        });

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().open);
        assert_eq!(
            effect_names(&result),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
    }

    #[test]
    fn swipe_end_negative_velocity_dismisses() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));

        let result = service.send(Event::SwipeEnd {
            velocity: -1.5,
            offset: -10.0,
        });

        assert_eq!(service.state(), &State::Dismissing);
        assert!(result.state_changed);
    }

    #[test]
    fn swipe_end_uses_props_threshold() {
        let mut service = fresh_service(Props {
            swipe_threshold: 200.0,
            ..test_props()
        });

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));

        let under = service.send(Event::SwipeEnd {
            velocity: 0.0,
            offset: 100.0,
        });

        assert_eq!(service.state(), &State::Visible);
        assert!(under.pending_effects.is_empty());

        drop(service.send(Event::SwipeStart(0.0)));

        let over = service.send(Event::SwipeEnd {
            velocity: 0.0,
            offset: 250.0,
        });

        assert_eq!(service.state(), &State::Dismissing);
        assert_eq!(
            effect_names(&over),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
    }

    #[test]
    fn swipe_in_paused_state_still_works() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(1_000),
        }));

        let result = service.send(Event::SwipeStart(5.0));

        assert_eq!(service.state(), &State::Paused);
        assert!(service.context().swiping);
        assert_eq!(service.context().swipe_offset, 5.0);
        assert!(!result.state_changed);
    }

    // ── dismiss_plan context resets (round-6 regression) ────────────

    #[test]
    fn dismiss_from_paused_clears_paused_flag() {
        // Regression: `Context::paused` is documented to mirror
        // `State::Paused`, but `dismiss_plan` previously left it set
        // when transitioning Paused → Dismissing, leaking stale pause
        // semantics into adapter callbacks during the exit animation.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(1_000),
        }));
        assert!(service.context().paused);

        drop(service.send(Event::Dismiss));

        assert_eq!(service.state(), &State::Dismissing);
        assert!(
            !service.context().paused,
            "paused flag must mirror State::Paused, not survive into Dismissing"
        );
    }

    #[test]
    fn duration_expired_while_swiping_clears_swipe_state() {
        // Regression: a duration-expired event arriving mid-swipe
        // bypasses `Event::SwipeEnd`. Without `dismiss_plan` resetting
        // swipe state, `ctx.swiping` / `ctx.swipe_offset` would stay
        // armed during the exit animation and adapters would render a
        // half-dragged toast as it animates out.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(12.0)));
        drop(service.send(Event::SwipeMove(40.0)));
        assert!(service.context().swiping);
        assert_eq!(service.context().swipe_offset, 40.0);

        drop(service.send(Event::DurationExpired));

        assert_eq!(service.state(), &State::Dismissing);
        assert!(
            !service.context().swiping,
            "swiping flag must clear when dismiss bypasses SwipeEnd"
        );
        assert_eq!(
            service.context().swipe_offset,
            0.0,
            "swipe_offset must reset to 0 when dismiss bypasses SwipeEnd"
        );
    }

    #[test]
    fn close_trigger_dismiss_while_swiping_clears_swipe_state() {
        // Same regression as `duration_expired_while_swiping_…` but for
        // close-button driven dismiss: also bypasses `SwipeEnd`.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(15.0)));
        assert!(service.context().swiping);

        drop(service.send(Event::Dismiss));

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().swiping);
        assert_eq!(service.context().swipe_offset, 0.0);
    }

    #[test]
    fn dismiss_while_paused_and_swiping_clears_both() {
        // Combined case: paused (hover) AND swiping mid-drag, then a
        // close-button dismiss arrives. The single dismiss_plan helper
        // must converge to clean context for adapters reading it
        // during the exit animation.
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(2_000),
        }));
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(25.0)));
        assert!(service.context().paused);
        assert!(service.context().swiping);

        let result = service.send(Event::Dismiss);

        assert_eq!(service.state(), &State::Dismissing);
        assert!(!service.context().paused);
        assert!(!service.context().swiping);
        assert_eq!(service.context().swipe_offset, 0.0);
        assert!(!service.context().open);
        // Same dismiss-effects regardless of source state.
        assert_eq!(
            effect_names(&result),
            vec![Effect::ExitAnimation, Effect::OpenChange]
        );
    }

    // ── Api event helpers ───────────────────────────────────────────

    #[test]
    fn api_event_helpers_dispatch_expected_events() {
        let service = fresh_service(test_props());

        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);
        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_close_trigger_click();
        api.on_pointer_enter(Duration::from_millis(1_500));
        api.on_pointer_leave();

        assert_eq!(
            &*sent.borrow(),
            &[
                Event::Dismiss,
                Event::Pause {
                    remaining: Duration::from_millis(1_500)
                },
                Event::Resume,
            ]
        );
    }

    #[test]
    fn api_predicates_track_state_transitions() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());

        {
            let api = service.connect(&|_| {});

            assert!(api.is_visible());
            assert!(!api.is_paused());
            assert!(!api.is_dismissed());
            assert_eq!(api.kind(), Kind::Info);
            assert_eq!(api.swipe_threshold(), DEFAULT_SWIPE_THRESHOLD);
        }

        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(4_000),
        }));

        {
            let api = service.connect(&|_| {});

            assert!(api.is_visible());
            assert!(api.is_paused());
        }

        drop(service.send(Event::Dismiss));

        {
            let api = service.connect(&|_| {});

            assert!(!api.is_visible());
            assert!(api.is_dismissed());
        }
    }

    #[test]
    fn connect_api_part_attrs_dispatches_to_each_helper() {
        let service = fresh_service(test_props());
        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Title), api.title_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ActionTrigger {
                alt_text: "Undo deletion".to_string(),
            }),
            api.action_trigger_attrs("Undo deletion"),
        );
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
        assert_eq!(api.part_attrs(Part::ProgressBar), api.progress_bar_attrs());
    }

    #[test]
    fn root_attrs_skip_labelledby_when_title_absent() {
        let service = fresh_service(Props {
            title: None,
            description: None,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        let root = api.root_attrs();

        let rendered = format!("{root:#?}");

        assert!(!rendered.contains("LabelledBy"));
        assert!(!rendered.contains("DescribedBy"));
    }

    // ── Snapshot coverage — anatomy parts × state × kind ─────────

    #[test]
    fn snapshot_visible_info() {
        let service = fresh_service(test_props());

        assert_snapshot!(
            "toast_visible_info",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_visible_success() {
        let service = fresh_service(Props {
            kind: Kind::Success,
            ..test_props()
        });

        assert_snapshot!(
            "toast_visible_success",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_visible_warning() {
        let service = fresh_service(Props {
            kind: Kind::Warning,
            ..test_props()
        });

        assert_snapshot!(
            "toast_visible_warning",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_visible_error() {
        let service = fresh_service(Props {
            kind: Kind::Error,
            ..test_props()
        });

        assert_snapshot!(
            "toast_visible_error",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_visible_loading_persistent() {
        let service = fresh_service(Props {
            kind: Kind::Loading,
            duration: None,
            ..test_props()
        });

        assert_snapshot!(
            "toast_visible_loading_persistent",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_paused() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Pause {
            remaining: Duration::from_millis(2_000),
        }));

        assert_snapshot!("toast_paused", snapshot_api(&service.connect(&|_| {})));
    }

    #[test]
    fn snapshot_swiping() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::SwipeStart(0.0)));
        drop(service.send(Event::SwipeMove(20.0)));

        assert_snapshot!("toast_swiping", snapshot_api(&service.connect(&|_| {})));
    }

    #[test]
    fn snapshot_dismissing() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Dismiss));

        assert_snapshot!("toast_dismissing", snapshot_api(&service.connect(&|_| {})));
    }

    #[test]
    fn snapshot_dismissed() {
        let mut service = fresh_service(test_props());

        drop(service.take_initial_effects());
        drop(service.send(Event::Dismiss));
        drop(service.send(Event::AnimationComplete));

        assert_snapshot!("toast_dismissed", snapshot_api(&service.connect(&|_| {})));
    }

    #[test]
    fn snapshot_root_without_title_or_description() {
        let service = fresh_service(Props {
            title: None,
            description: None,
            ..test_props()
        });

        assert_snapshot!(
            "toast_root_no_labels",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_close_trigger_custom_label() {
        let messages = Messages {
            dismiss_label: MessageFn::static_str("Descartar"),
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

        assert_snapshot!(
            "toast_close_trigger_custom_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }
}
