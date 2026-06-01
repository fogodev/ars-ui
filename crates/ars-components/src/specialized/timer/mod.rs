//! Timer component state machine and connect API.
//!
//! The `Timer` implements countdown or stopwatch functionality with start,
//! pause, resume, reset, and restart controls. It ticks at a configurable
//! interval and transitions to a [`State::Completed`] state when a countdown
//! reaches zero.
//!
//! The agnostic core only emits typed `Effect` intents and exposes the `Api`
//! connect surface; framework adapters translate
//! `Effect::TimerInterval` into a real recurring interval (e.g. `setInterval`)
//! that dispatches `Event::Tick`, and translate `Effect::AnnounceCompleted`
//! into a live-region announcement.

use alloc::{format, string::String, sync::Arc, vec::Vec};
use core::{
    fmt::{self, Debug},
    num::NonZeroU8,
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, CssProperty,
    Env, HasId, HtmlAttr, IntlBackend, Locale, MessageFn, PendingEffect, TransitionPlan,
};

/// Timer mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Counts down from a target duration to zero.
    #[default]
    Countdown,

    /// Counts up from zero indefinitely.
    Stopwatch,
}

/// The state of the `Timer` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Timer is not running, at its initial value.
    Idle,

    /// Timer is actively ticking.
    Running,

    /// Timer is paused, preserving its current value.
    Paused,

    /// Countdown reached zero (only for [`Mode::Countdown`]).
    Completed,
}

/// Events for the `Timer` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Start or resume the timer.
    Start,

    /// Pause the timer.
    Pause,

    /// Resume from the paused state.
    Resume,

    /// Reset to the initial value.
    Reset,

    /// Reset to the initial value and immediately start.
    Restart,

    /// A single tick interval elapsed (dispatched by the adapter's interval).
    Tick,

    /// Set the remaining/elapsed time directly.
    SetTime(Duration),
}

/// Context for the `Timer` component.
///
/// Cloneable but not `Debug`/`PartialEq`-derivable because [`Context::intl_backend`]
/// is a trait object; both impls are provided manually and exclude the backend
/// (it is an injected service, not observable state), mirroring
/// [`time_field`](crate::date_time::time_field).
#[derive(Clone)]
pub struct Context {
    /// Current time value. For countdown this is the remaining time; for
    /// stopwatch this is the elapsed time.
    pub current: Duration,

    /// The target duration (for countdown).
    pub target: Duration,

    /// Tick interval.
    pub interval: Duration,

    /// Timer mode.
    pub mode: Mode,

    /// Whether auto-start is enabled.
    pub auto_start: bool,

    /// Active locale inherited from provider context.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,

    /// Backend used for locale-aware digit formatting of the displayed time.
    pub intl_backend: Arc<dyn IntlBackend>,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("timer::Context")
            .field("current", &self.current)
            .field("target", &self.target)
            .field("interval", &self.interval)
            .field("mode", &self.mode)
            .field("auto_start", &self.auto_start)
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .field("ids", &self.ids)
            .field("intl_backend", &"<dyn IntlBackend>")
            .finish()
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.current == other.current
            && self.target == other.target
            && self.interval == other.interval
            && self.mode == other.mode
            && self.auto_start == other.auto_start
            && self.locale == other.locale
            && self.messages == other.messages
            && self.ids == other.ids
    }
}

/// Props for the `Timer` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Target duration (for countdown mode).
    pub target: Duration,

    /// Tick interval.
    pub interval: Duration,

    /// Timer mode (countdown or stopwatch).
    pub mode: Mode,

    /// Auto-start when mounted.
    pub auto_start: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            target: Duration::from_secs(60),
            interval: Duration::from_secs(1),
            mode: Mode::Countdown,
            auto_start: false,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`target`](Self::target).
    #[must_use]
    pub const fn target(mut self, target: Duration) -> Self {
        self.target = target;
        self
    }

    /// Sets [`interval`](Self::interval).
    #[must_use]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets [`mode`](Self::mode).
    #[must_use]
    pub const fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets [`auto_start`](Self::auto_start).
    #[must_use]
    pub const fn auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }
}

/// Messages for the `Timer` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Start button label. Default: `"Start timer"`.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Pause button label. Default: `"Pause timer"`.
    pub pause_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Reset button label. Default: `"Reset timer"`.
    pub reset_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Resume button label. Default: `"Resume timer"`.
    pub resume_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Progress bar label. Default: `"Timer progress"`.
    pub progress_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Completion announcement. Default: `"Timer completed"`.
    pub completed_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            start_label: MessageFn::static_str("Start timer"),
            pause_label: MessageFn::static_str("Pause timer"),
            reset_label: MessageFn::static_str("Reset timer"),
            resume_label: MessageFn::static_str("Resume timer"),
            progress_label: MessageFn::static_str("Timer progress"),
            completed_announcement: MessageFn::static_str("Timer completed"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the timer machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts a recurring interval of [`Context::interval`] that
    /// dispatches [`Event::Tick`] on each elapse. Emitted on initial mount when
    /// the timer auto-starts (via
    /// [`Machine::initial_effects`](ars_core::Machine::initial_effects)) and on
    /// every transition into [`State::Running`]. Cancelled whenever the timer
    /// leaves [`State::Running`] (pause, completion, reset, or restart); the
    /// adapter no-ops the cancellation when no interval is active.
    TimerInterval,

    /// Adapter announces the [`Messages::completed_announcement`] message into a
    /// polite `aria-live` region. Emitted on the countdown transition into
    /// [`State::Completed`].
    AnnounceCompleted,
}

/// The machine for the `Timer` component.
///
/// # Examples
///
/// Drive a two-second countdown to completion. Each [`Event::Tick`] is
/// dispatched by the adapter-owned interval started by
/// [`Effect::TimerInterval`]; here we send them directly:
///
/// ```
/// use core::time::Duration;
///
/// use ars_components::specialized::timer::{Event, Machine, Messages, Props, State};
/// use ars_core::{Env, Service};
///
/// let mut timer = Service::<Machine>::new(
///     Props::new()
///         .id("egg")
///         .target(Duration::from_secs(2))
///         .interval(Duration::from_secs(1)),
///     &Env::default(),
///     &Messages::default(),
/// );
///
/// drop(timer.send(Event::Start));
/// assert_eq!(timer.state(), &State::Running);
///
/// drop(timer.send(Event::Tick)); // 2s -> 1s remaining
/// assert_eq!(timer.context().current, Duration::from_secs(1));
///
/// drop(timer.send(Event::Tick)); // 1s -> 0s: the countdown completes
/// assert_eq!(timer.state(), &State::Completed);
/// assert_eq!(timer.connect(&|_| {}).formatted_time(), "00:00");
/// ```
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let current = initial_duration(props.mode, props.target);

        let state = if props.auto_start {
            State::Running
        } else {
            State::Idle
        };

        (
            state,
            Context {
                current,
                target: props.target,
                interval: props.interval,
                mode: props.mode,
                auto_start: props.auto_start,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
                intl_backend: Arc::clone(&env.intl_backend),
            },
        )
    }

    fn initial_effects(
        state: &Self::State,
        _ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        let mut effects = Vec::new();

        // Auto-started timers boot directly into `Running` without a
        // transition, so the interval intent is emitted here on first mount.
        if matches!(state, State::Running) {
            effects.push(PendingEffect::named(Effect::TimerInterval));
        }

        effects
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Idle start, paused resume, and paused start all enter `Running`
            // and (re)emit the interval intent.
            (State::Idle, Event::Start) | (State::Paused, Event::Resume | Event::Start) => {
                Some(TransitionPlan::to(State::Running).with_effect(interval_effect()))
            }

            (State::Running, Event::Tick) => match ctx.mode {
                Mode::Countdown => {
                    let new = ctx.current.saturating_sub(ctx.interval);

                    if new.is_zero() {
                        Some(
                            TransitionPlan::to(State::Completed)
                                .apply(|ctx: &mut Context| {
                                    ctx.current = Duration::ZERO;
                                })
                                .cancel_effect(Effect::TimerInterval)
                                .with_effect(PendingEffect::named(Effect::AnnounceCompleted)),
                        )
                    } else {
                        Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                            ctx.current = new;
                        }))
                    }
                }
                Mode::Stopwatch => {
                    let interval = ctx.interval;
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.current = ctx.current.saturating_add(interval);
                    }))
                }
            },

            (State::Running, Event::Pause) => {
                Some(TransitionPlan::to(State::Paused).cancel_effect(Effect::TimerInterval))
            }

            (_, Event::Reset) => {
                let initial = initial_duration(ctx.mode, ctx.target);
                Some(
                    TransitionPlan::to(State::Idle)
                        .apply(move |ctx: &mut Context| {
                            ctx.current = initial;
                        })
                        .cancel_effect(Effect::TimerInterval),
                )
            }

            (_, Event::Restart) => {
                let initial = initial_duration(ctx.mode, ctx.target);
                Some(
                    TransitionPlan::to(State::Running)
                        .apply(move |ctx: &mut Context| {
                            ctx.current = initial;
                        })
                        .cancel_effect(Effect::TimerInterval)
                        .with_effect(PendingEffect::named(Effect::TimerInterval)),
                )
            }

            (_, Event::SetTime(duration)) => {
                let duration = *duration;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.current = duration;
                }))
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
}

/// DOM parts of the `Timer` component.
#[derive(ComponentPart)]
#[scope = "timer"]
pub enum Part {
    /// Root wrapper element (`role="timer"`, `aria-live` region).
    Root,

    /// Label describing the timer.
    Label,

    /// Formatted time-string display.
    Display,

    /// Progress indicator (`role="progressbar"`).
    Progress,

    /// Start/resume button.
    StartTrigger,

    /// Pause button.
    PauseTrigger,

    /// Reset button.
    ResetTrigger,

    /// Decorative colon between time segments.
    Separator,
}

/// API for the `Timer` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("timer::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the timer is actively ticking.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self.state, State::Running)
    }

    /// Whether the timer is paused.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        matches!(self.state, State::Paused)
    }

    /// Whether the countdown has completed.
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self.state, State::Completed)
    }

    /// Whether the timer is idle at its initial value.
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        matches!(self.state, State::Idle)
    }

    /// The current time value.
    #[must_use]
    pub const fn current(&self) -> Duration {
        self.ctx.current
    }

    /// Current time broken into hours, minutes, seconds, and milliseconds.
    #[must_use]
    pub const fn display_time(&self) -> (u64, u64, u64, u64) {
        let total_ms = self.ctx.current.as_millis() as u64;

        let hours = total_ms / 3_600_000;
        let minutes = (total_ms % 3_600_000) / 60_000;
        let seconds = (total_ms % 60_000) / 1_000;
        let millis = total_ms % 1_000;

        (hours, minutes, seconds, millis)
    }

    /// Progress as a fraction in `[0.0, 1.0]`.
    ///
    /// For countdown this is the fraction of [`Context::target`] already
    /// elapsed; for stopwatch this is the elapsed time as a fraction of
    /// [`Context::target`]. The result is clamped to `[0.0, 1.0]` so an
    /// over-target stopwatch or an out-of-range [`Event::SetTime`] never yields
    /// a value outside the documented range (which would otherwise break the
    /// `progressbar` `aria-valuenow`/`valuemin`/`valuemax` semantics).
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.ctx.target.is_zero() {
            return 0.0;
        }

        let fraction = self.ctx.current.as_secs_f64() / self.ctx.target.as_secs_f64();

        let progress = match self.ctx.mode {
            Mode::Countdown => 1.0 - fraction,
            Mode::Stopwatch => fraction,
        };

        progress.clamp(0.0, 1.0)
    }

    /// Formatted time string (`HH:MM:SS` when hours are present, else `MM:SS`).
    ///
    /// Digits are rendered through [`Context::intl_backend`] so non-ASCII
    /// numbering systems (e.g. Arabic-Indic) are honored when a localizing
    /// backend is provided; the default
    /// [`StubIntlBackend`](ars_core::StubIntlBackend) yields ASCII digits.
    #[must_use]
    pub fn formatted_time(&self) -> String {
        let (hours, minutes, seconds, _millis) = self.display_time();

        let format_segment = |value: u64| {
            let width = NonZeroU8::new(2).expect("segment width is non-zero");
            u32::try_from(value).map_or_else(
                |_| value.to_string(),
                |value| {
                    self.ctx
                        .intl_backend
                        .format_segment_digits(value, width, &self.ctx.locale)
                },
            )
        };

        if hours > 0 {
            format!(
                "{}:{}:{}",
                format_segment(hours),
                format_segment(minutes),
                format_segment(seconds)
            )
        } else {
            format!("{}:{}", format_segment(minutes), format_segment(seconds))
        }
    }

    const fn state_str(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Running => "running",
            State::Paused => "paused",
            State::Completed => "completed",
        }
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "timer")
            .set(HtmlAttr::Data("ars-state"), self.state_str())
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true")
            .set(HtmlAttr::Aria(AriaAttr::Label), self.formatted_time());

        attrs
    }

    /// Label element attributes.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Display element attributes.
    #[must_use]
    pub fn display_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Display.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("display"))
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Progress indicator attributes.
    #[must_use]
    pub fn progress_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Progress.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "progressbar")
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                format!("{:.0}", self.progress() * 100.0),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), "0")
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), "100")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.progress_label)(&self.ctx.locale),
            )
            .set_style(
                CssProperty::Custom("ars-timer-progress"),
                format!("{:.2}", self.progress()),
            );

        attrs
    }

    /// Start/resume trigger attributes.
    #[must_use]
    pub fn start_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                if self.is_paused() {
                    (self.ctx.messages.resume_label)(&self.ctx.locale)
                } else {
                    (self.ctx.messages.start_label)(&self.ctx.locale)
                },
            );

        if self.is_running() || self.is_completed() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Pause trigger attributes.
    #[must_use]
    pub fn pause_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PauseTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.pause_label)(&self.ctx.locale),
            );

        if !self.is_running() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Reset trigger attributes.
    #[must_use]
    pub fn reset_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ResetTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.reset_label)(&self.ctx.locale),
            );

        if self.is_idle() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Separator element attributes.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Dispatches start/resume intent from the start trigger.
    pub fn on_start_trigger_click(&self) {
        if self.is_paused() {
            (self.send)(Event::Resume);
        } else {
            (self.send)(Event::Start);
        }
    }

    /// Dispatches pause intent from the pause trigger.
    pub fn on_pause_trigger_click(&self) {
        (self.send)(Event::Pause);
    }

    /// Dispatches reset intent from the reset trigger.
    pub fn on_reset_trigger_click(&self) {
        (self.send)(Event::Reset);
    }

    /// Dispatches restart intent (reset and immediately start).
    pub fn on_restart(&self) {
        (self.send)(Event::Restart);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Display => self.display_attrs(),
            Part::Progress => self.progress_attrs(),
            Part::StartTrigger => self.start_trigger_attrs(),
            Part::PauseTrigger => self.pause_trigger_attrs(),
            Part::ResetTrigger => self.reset_trigger_attrs(),
            Part::Separator => self.separator_attrs(),
        }
    }
}

/// The initial `current` value for a given mode and target.
const fn initial_duration(mode: Mode, target: Duration) -> Duration {
    match mode {
        Mode::Countdown => target,
        Mode::Stopwatch => Duration::ZERO,
    }
}

/// Builds the marker effect that starts the adapter-owned tick interval.
fn interval_effect() -> PendingEffect<Machine> {
    PendingEffect::named(Effect::TimerInterval)
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{Machine as _, Service, StubIntlBackend};
    use ars_i18n::{HourCycle, WeekInfo, Weekday};
    use insta::assert_snapshot;

    use super::*;

    /// Test backend that prefixes segment digits with `loc-` so we can assert
    /// `formatted_time`/`aria-label` actually route through the locale backend
    /// rather than hard-coding ASCII. Mirrors `time_field`'s test double.
    struct LocalizedDigitsBackend;

    impl IntlBackend for LocalizedDigitsBackend {
        fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_short_label(weekday, locale)
        }

        fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_long_label(weekday, locale)
        }

        fn month_long_name(&self, month: u8, locale: &Locale) -> String {
            StubIntlBackend.month_long_name(month, locale)
        }

        fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
            StubIntlBackend.day_period_label(is_pm, locale)
        }

        fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
            StubIntlBackend.day_period_from_char(ch, locale)
        }

        fn format_segment_digits(
            &self,
            value: u32,
            min_digits: NonZeroU8,
            _locale: &Locale,
        ) -> String {
            let width = usize::from(min_digits.get());
            format!("loc-{value:0>width$}")
        }

        fn hour_cycle(&self, locale: &Locale) -> HourCycle {
            StubIntlBackend.hour_cycle(locale)
        }

        fn week_info(&self, locale: &Locale) -> WeekInfo {
            StubIntlBackend.week_info(locale)
        }
    }

    /// Leaks an [`Api`] whose context carries a localizing backend, for digit
    /// routing assertions.
    fn api_with_localized_backend(current: Duration) -> Api<'static> {
        let env = Env::new(
            Locale::parse("ar").expect("`ar` is a valid BCP-47 tag"),
            Arc::new(LocalizedDigitsBackend),
        );
        let props = Box::leak(Box::new(
            Props::new().id("timer").target(Duration::from_secs(60)),
        ));
        let messages = Messages::default();
        let (_, mut ctx) = Machine::init(props, &env, &messages);
        ctx.current = current;

        let ctx = Box::leak(Box::new(ctx));
        let state = Box::leak(Box::new(State::Running));
        let send = Box::leak(Box::new(|_: Event| {}));

        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn test_props() -> Props {
        Props::new().id("timer")
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    /// Leaks a fully-formed [`Api`] in an arbitrary state for attribute and
    /// accessor assertions, mirroring the clipboard test helper.
    fn api_for(state: State, current: Duration, mode: Mode, target: Duration) -> Api<'static> {
        let props = Box::leak(Box::new(Props::new().id("timer").mode(mode).target(target)));

        let messages = Messages::default();

        let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

        ctx.current = current;

        let ctx = Box::leak(Box::new(ctx));
        let state = Box::leak(Box::new(state));
        let send = Box::leak(Box::new(|_: Event| {}));

        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    // ───────────────────────── Props ─────────────────────────

    #[test]
    fn timer_props_default_matches_spec() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert_eq!(props.target, Duration::from_secs(60));
        assert_eq!(props.interval, Duration::from_secs(1));
        assert_eq!(props.mode, Mode::Countdown);
        assert!(!props.auto_start);
    }

    #[test]
    fn timer_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("timer")
            .target(Duration::from_secs(5))
            .interval(Duration::from_millis(250))
            .mode(Mode::Stopwatch)
            .auto_start(true);

        assert_eq!(props.id, "timer");
        assert_eq!(props.target, Duration::from_secs(5));
        assert_eq!(props.interval, Duration::from_millis(250));
        assert_eq!(props.mode, Mode::Stopwatch);
        assert!(props.auto_start);
    }

    #[test]
    fn timer_mode_default_is_countdown() {
        assert_eq!(Mode::default(), Mode::Countdown);
    }

    // ───────────────────────── init ─────────────────────────

    #[test]
    fn timer_countdown_init_starts_idle_at_target() {
        let service = fresh_service(test_props().target(Duration::from_secs(30)));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current, Duration::from_secs(30));
        assert_eq!(service.context().target, Duration::from_secs(30));
        assert_eq!(service.context().mode, Mode::Countdown);
        assert_eq!(service.context().ids.id(), "timer");
    }

    #[test]
    fn timer_stopwatch_init_starts_idle_at_zero() {
        let service = fresh_service(
            test_props()
                .mode(Mode::Stopwatch)
                .target(Duration::from_secs(30)),
        );

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current, Duration::ZERO);
    }

    #[test]
    fn timer_auto_start_boots_running_and_emits_interval_effect() {
        let mut service = fresh_service(test_props().auto_start(true));

        assert_eq!(service.state(), &State::Running);
        assert_eq!(
            service
                .take_initial_effects()
                .iter()
                .map(|effect| effect.name)
                .collect::<Vec<_>>(),
            vec![Effect::TimerInterval]
        );
    }

    #[test]
    fn timer_without_auto_start_emits_no_initial_effects() {
        let mut service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Idle);
        assert!(service.take_initial_effects().is_empty());
    }

    // ───────────────────── transitions ─────────────────────

    #[test]
    fn timer_full_lifecycle_idle_running_paused_running_completed() {
        let mut service = fresh_service(
            test_props()
                .target(Duration::from_secs(2))
                .interval(Duration::from_secs(1)),
        );

        let started = service.send(Event::Start);

        assert!(started.state_changed);
        assert_eq!(service.state(), &State::Running);
        assert_eq!(effect_names(&started), vec![Effect::TimerInterval]);

        let tick = service.send(Event::Tick);

        assert_eq!(service.state(), &State::Running);
        assert_eq!(service.context().current, Duration::from_secs(1));
        assert!(tick.pending_effects.is_empty());

        let paused = service.send(Event::Pause);

        assert_eq!(service.state(), &State::Paused);
        assert_eq!(paused.cancel_effects, vec![Effect::TimerInterval]);

        let resumed = service.send(Event::Resume);

        assert_eq!(service.state(), &State::Running);
        assert_eq!(effect_names(&resumed), vec![Effect::TimerInterval]);

        let completed = service.send(Event::Tick);

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().current, Duration::ZERO);
        assert_eq!(completed.cancel_effects, vec![Effect::TimerInterval]);
        assert_eq!(effect_names(&completed), vec![Effect::AnnounceCompleted]);
    }

    #[test]
    fn timer_countdown_tick_decrements_by_interval() {
        let mut service = fresh_service(
            test_props()
                .target(Duration::from_secs(10))
                .interval(Duration::from_millis(2_500)),
        );

        drop(service.send(Event::Start));

        drop(service.send(Event::Tick));

        assert_eq!(service.context().current, Duration::from_millis(7_500));

        drop(service.send(Event::Tick));

        assert_eq!(service.context().current, Duration::from_secs(5));
    }

    #[test]
    fn timer_countdown_tick_at_or_below_interval_completes() {
        // current (500ms) < interval (1s) saturates to zero and completes.
        let mut service = fresh_service(
            test_props()
                .target(Duration::from_millis(500))
                .interval(Duration::from_secs(1)),
        );

        drop(service.send(Event::Start));

        let result = service.send(Event::Tick);

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().current, Duration::ZERO);
        assert_eq!(effect_names(&result), vec![Effect::AnnounceCompleted]);
    }

    #[test]
    fn timer_stopwatch_tick_increments_from_zero() {
        let mut service = fresh_service(
            test_props()
                .mode(Mode::Stopwatch)
                .interval(Duration::from_secs(1)),
        );

        drop(service.send(Event::Start));

        drop(service.send(Event::Tick));

        assert_eq!(service.state(), &State::Running);
        assert_eq!(service.context().current, Duration::from_secs(1));

        drop(service.send(Event::Tick));

        assert_eq!(service.context().current, Duration::from_secs(2));
    }

    #[test]
    fn timer_stopwatch_never_completes() {
        let mut service = fresh_service(
            test_props()
                .mode(Mode::Stopwatch)
                .target(Duration::from_secs(1)),
        );

        drop(service.send(Event::Start));

        for _ in 0..5 {
            drop(service.send(Event::Tick));
        }

        assert_eq!(service.state(), &State::Running);
        assert_eq!(service.context().current, Duration::from_secs(5));
    }

    #[test]
    fn timer_reset_returns_to_idle_and_cancels_interval() {
        let mut service = fresh_service(test_props().target(Duration::from_secs(10)));

        drop(service.send(Event::Start));
        drop(service.send(Event::Tick));

        let result = service.send(Event::Reset);

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current, Duration::from_secs(10));
        assert_eq!(result.cancel_effects, vec![Effect::TimerInterval]);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn timer_reset_stopwatch_returns_to_zero() {
        let mut service = fresh_service(test_props().mode(Mode::Stopwatch));

        drop(service.send(Event::Start));
        drop(service.send(Event::Tick));

        drop(service.send(Event::Reset));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().current, Duration::ZERO);
    }

    #[test]
    fn timer_restart_resets_value_and_restarts_interval() {
        let mut service = fresh_service(test_props().target(Duration::from_secs(10)));

        drop(service.send(Event::Start));
        drop(service.send(Event::Tick));
        drop(service.send(Event::Pause));

        let result = service.send(Event::Restart);

        assert_eq!(service.state(), &State::Running);
        assert_eq!(service.context().current, Duration::from_secs(10));
        assert_eq!(result.cancel_effects, vec![Effect::TimerInterval]);
        assert_eq!(effect_names(&result), vec![Effect::TimerInterval]);
    }

    #[test]
    fn timer_set_time_updates_current_without_changing_state() {
        let mut service = fresh_service(test_props().target(Duration::from_secs(10)));

        drop(service.send(Event::Start));

        let result = service.send(Event::SetTime(Duration::from_millis(3_333)));

        assert_eq!(service.state(), &State::Running);
        assert_eq!(service.context().current, Duration::from_millis(3_333));
        assert!(result.context_changed);
    }

    #[test]
    fn timer_ignores_unhandled_events() {
        let mut service = fresh_service(test_props());

        // Tick while idle is a no-op.
        let result = service.send(Event::Tick);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);

        // Pause while idle is a no-op.
        let result = service.send(Event::Pause);

        assert!(!result.state_changed);

        // Start from completed is rejected.
        let mut service = fresh_service(
            test_props()
                .target(Duration::from_secs(1))
                .interval(Duration::from_secs(1)),
        );

        drop(service.send(Event::Start));
        drop(service.send(Event::Tick));

        assert_eq!(service.state(), &State::Completed);

        let result = service.send(Event::Start);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Completed);
    }

    // ───────────────────────── Api ─────────────────────────

    #[test]
    fn timer_api_state_predicates_match_state() {
        let one_sec = Duration::from_secs(1);

        assert!(api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec).is_idle());
        assert!(api_for(State::Running, Duration::ZERO, Mode::Countdown, one_sec).is_running());
        assert!(api_for(State::Paused, Duration::ZERO, Mode::Countdown, one_sec).is_paused());
        assert!(api_for(State::Completed, Duration::ZERO, Mode::Countdown, one_sec).is_completed());
    }

    #[test]
    fn timer_api_current_reports_context() {
        assert_eq!(
            api_for(
                State::Running,
                Duration::from_millis(4_200),
                Mode::Countdown,
                Duration::from_secs(10),
            )
            .current(),
            Duration::from_millis(4_200)
        );
    }

    #[test]
    fn timer_api_display_time_splits_segments() {
        let value = Duration::from_millis(3_661_000);

        let api = api_for(State::Running, value, Mode::Countdown, value);

        assert_eq!(api.display_time(), (1, 1, 1, 0));

        let value = Duration::from_millis(90_500);

        let api = api_for(State::Running, value, Mode::Countdown, value);

        assert_eq!(api.display_time(), (0, 1, 30, 500));
    }

    #[test]
    fn timer_api_formatted_time_uses_hours_only_when_present() {
        let with_hours = Duration::from_millis(3_661_000);

        assert_eq!(
            api_for(State::Running, with_hours, Mode::Countdown, with_hours).formatted_time(),
            "01:01:01"
        );

        let without_hours = Duration::from_secs(90);

        assert_eq!(
            api_for(
                State::Running,
                without_hours,
                Mode::Countdown,
                without_hours
            )
            .formatted_time(),
            "01:30"
        );
    }

    #[test]
    fn timer_api_progress_countdown_and_stopwatch() {
        let one_sec = Duration::from_secs(1);
        let half_sec = Duration::from_millis(500);

        // Countdown: progress is the elapsed fraction.
        assert!(
            (api_for(State::Idle, one_sec, Mode::Countdown, one_sec).progress() - 0.0).abs() < 1e-9
        );
        assert!(
            (api_for(State::Running, half_sec, Mode::Countdown, one_sec).progress() - 0.5).abs()
                < 1e-9
        );
        assert!(
            (api_for(State::Completed, Duration::ZERO, Mode::Countdown, one_sec).progress() - 1.0)
                .abs()
                < 1e-9
        );

        // Stopwatch: progress is the elapsed fraction of target.
        assert!(
            (api_for(State::Running, half_sec, Mode::Stopwatch, one_sec).progress() - 0.5).abs()
                < 1e-9
        );

        // Zero target guards against division by zero.
        assert!(
            (api_for(State::Idle, Duration::ZERO, Mode::Countdown, Duration::ZERO).progress()
                - 0.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn timer_root_attrs_carry_timer_role_live_region_and_label() {
        let api = api_for(
            State::Running,
            Duration::from_secs(30),
            Mode::Countdown,
            Duration::from_secs(60),
        );

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("timer"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("00:30"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("running"));
    }

    #[test]
    fn timer_root_data_state_tracks_each_state() {
        let one_sec = Duration::from_secs(1);

        assert_eq!(
            api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec)
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("idle")
        );
        assert_eq!(
            api_for(State::Paused, Duration::ZERO, Mode::Countdown, one_sec)
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("paused")
        );
        assert_eq!(
            api_for(State::Completed, Duration::ZERO, Mode::Countdown, one_sec)
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("completed")
        );
    }

    #[test]
    fn timer_progress_attrs_expose_progressbar_role_and_values() {
        let attrs = api_for(
            State::Completed,
            Duration::ZERO,
            Mode::Countdown,
            Duration::from_secs(1),
        )
        .progress_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("progressbar"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("100"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Timer progress")
        );
    }

    #[test]
    fn timer_trigger_disabled_states_match_spec() {
        let one_sec = Duration::from_secs(1);

        // StartTrigger disabled while running or completed.
        assert!(
            api_for(State::Running, Duration::ZERO, Mode::Countdown, one_sec)
                .start_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            api_for(State::Completed, Duration::ZERO, Mode::Countdown, one_sec)
                .start_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            !api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec)
                .start_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );

        // PauseTrigger disabled unless running.
        assert!(
            !api_for(State::Running, Duration::ZERO, Mode::Countdown, one_sec)
                .pause_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec)
                .pause_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );

        // ResetTrigger disabled while idle.
        assert!(
            api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec)
                .reset_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(
            !api_for(State::Running, Duration::ZERO, Mode::Countdown, one_sec)
                .reset_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
    }

    #[test]
    fn timer_start_trigger_label_switches_to_resume_when_paused() {
        let one_sec = Duration::from_secs(1);

        assert_eq!(
            api_for(State::Idle, Duration::ZERO, Mode::Countdown, one_sec)
                .start_trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Start timer")
        );
        assert_eq!(
            api_for(State::Paused, Duration::ZERO, Mode::Countdown, one_sec)
                .start_trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Resume timer")
        );
    }

    #[test]
    fn timer_click_handlers_dispatch_expected_events() {
        let events = Arc::new(Mutex::new(Vec::new()));

        let start_from_idle = {
            let captured = Arc::clone(&events);
            move |event| captured.lock().unwrap().push(event)
        };

        fresh_service(test_props())
            .connect(&start_from_idle)
            .on_start_trigger_click();

        // Resume path from paused.
        let mut paused = fresh_service(test_props());

        drop(paused.send(Event::Start));
        drop(paused.send(Event::Pause));

        {
            let captured = Arc::clone(&events);

            let sink = move |event| captured.lock().unwrap().push(event);

            paused.connect(&sink).on_start_trigger_click();
        }

        let service = fresh_service(test_props());

        {
            let captured = Arc::clone(&events);

            let sink = move |event| captured.lock().unwrap().push(event);

            let api = service.connect(&sink);

            api.on_pause_trigger_click();
            api.on_reset_trigger_click();
            api.on_restart();
        }

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                Event::Start,
                Event::Resume,
                Event::Pause,
                Event::Reset,
                Event::Restart,
            ]
        );
    }

    #[test]
    fn timer_connect_api_dispatch_matches_inherent_attrs() {
        let api = api_for(
            State::Running,
            Duration::from_secs(30),
            Mode::Countdown,
            Duration::from_secs(60),
        );

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Display), api.display_attrs());
        assert_eq!(api.part_attrs(Part::Progress), api.progress_attrs());
        assert_eq!(
            api.part_attrs(Part::StartTrigger),
            api.start_trigger_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::PauseTrigger),
            api.pause_trigger_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::ResetTrigger),
            api.reset_trigger_attrs()
        );
        assert_eq!(api.part_attrs(Part::Separator), api.separator_attrs());
    }

    #[test]
    fn timer_progress_clamped_to_unit_range() {
        // Stopwatch past its target: raw fraction 2.0 clamps to 1.0.
        let stopwatch = api_for(
            State::Running,
            Duration::from_secs(2),
            Mode::Stopwatch,
            Duration::from_secs(1),
        );
        assert!((stopwatch.progress() - 1.0).abs() < 1e-9);
        assert_eq!(
            stopwatch
                .progress_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("100")
        );

        // Countdown whose current exceeds target (e.g. via an out-of-range
        // SetTime): raw -1.0 clamps to 0.0 instead of rendering "-100".
        let countdown = api_for(
            State::Running,
            Duration::from_secs(2),
            Mode::Countdown,
            Duration::from_secs(1),
        );
        assert!((countdown.progress() - 0.0).abs() < 1e-9);
        assert_eq!(
            countdown
                .progress_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("0")
        );
    }

    #[test]
    fn timer_triggers_are_type_button() {
        let api = api_for(
            State::Running,
            Duration::from_secs(30),
            Mode::Countdown,
            Duration::from_secs(60),
        );

        assert_eq!(
            api.start_trigger_attrs().get(&HtmlAttr::Type),
            Some("button")
        );
        assert_eq!(
            api.pause_trigger_attrs().get(&HtmlAttr::Type),
            Some("button")
        );
        assert_eq!(
            api.reset_trigger_attrs().get(&HtmlAttr::Type),
            Some("button")
        );
    }

    #[test]
    fn timer_formatted_time_routes_digits_through_intl_backend() {
        // 90s -> 01:30; the localizing backend prefixes each segment with `loc-`.
        let api = api_with_localized_backend(Duration::from_secs(90));

        assert_eq!(api.formatted_time(), "loc-01:loc-30");

        // The root `aria-label` is sourced from `formatted_time`, so it inherits
        // the localized digits too.
        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("loc-01:loc-30")
        );
    }

    // ─────────────────────── snapshots ───────────────────────

    fn countdown_api(state: State, current_secs: u64) -> Api<'static> {
        api_for(
            state,
            Duration::from_secs(current_secs),
            Mode::Countdown,
            Duration::from_secs(60),
        )
    }

    #[test]
    fn timer_root_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(&countdown_api(State::Idle, 60).root_attrs()));
    }

    #[test]
    fn timer_root_running_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Running, 30).root_attrs()
        ));
    }

    #[test]
    fn timer_root_paused_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Paused, 30).root_attrs()
        ));
    }

    #[test]
    fn timer_root_completed_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Completed, 0).root_attrs()
        ));
    }

    #[test]
    fn timer_root_with_hours_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for(
                State::Running,
                Duration::from_secs(3_600),
                Mode::Countdown,
                Duration::from_secs(3_600),
            )
            .root_attrs()
        ));
    }

    #[test]
    fn timer_label_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).label_attrs()
        ));
    }

    #[test]
    fn timer_display_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).display_attrs()
        ));
    }

    #[test]
    fn timer_progress_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).progress_attrs()
        ));
    }

    #[test]
    fn timer_progress_completed_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Completed, 0).progress_attrs()
        ));
    }

    #[test]
    fn timer_start_trigger_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).start_trigger_attrs()
        ));
    }

    #[test]
    fn timer_start_trigger_paused_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Paused, 30).start_trigger_attrs()
        ));
    }

    #[test]
    fn timer_start_trigger_running_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Running, 30).start_trigger_attrs()
        ));
    }

    #[test]
    fn timer_pause_trigger_running_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Running, 30).pause_trigger_attrs()
        ));
    }

    #[test]
    fn timer_pause_trigger_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).pause_trigger_attrs()
        ));
    }

    #[test]
    fn timer_reset_trigger_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).reset_trigger_attrs()
        ));
    }

    #[test]
    fn timer_reset_trigger_running_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Running, 30).reset_trigger_attrs()
        ));
    }

    #[test]
    fn timer_separator_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &countdown_api(State::Idle, 60).separator_attrs()
        ));
    }
}
