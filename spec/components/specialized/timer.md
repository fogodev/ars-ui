---
component: Timer
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Timer
---

# Timer

A Timer component implements countdown or stopwatch functionality with start, pause,
resume, and reset controls. It ticks at a configurable interval and transitions to
a `Completed` state when a countdown reaches zero.

```rust
/// Timer mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Counts down from a target duration to zero.
    #[default]
    Countdown,
    /// Counts up from zero indefinitely.
    Stopwatch,
}
```

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Timer is not running, at initial value.
    Idle,
    /// Timer is actively ticking.
    Running,
    /// Timer is paused, preserving current value.
    Paused,
    /// Countdown reached zero (only for Countdown mode).
    Completed,
}
```

### 1.2 Events

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Start or resume the timer.
    Start,
    /// Pause the timer.
    Pause,
    /// Resume from paused state.
    Resume,
    /// Reset to initial value.
    Reset,
    /// Reset to initial value and immediately start.
    Restart,
    /// A single tick interval elapsed.
    Tick,
    /// Set the remaining/elapsed time directly.
    SetTime(Duration),
    /// Synchronize the context-backed props (`target`, `interval`, `mode`)
    /// after a controlled prop update. Emitted by `Machine::on_props_changed`.
    SyncProps,
}
```

### 1.3 Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current time value.
    /// For countdown: remaining time. For stopwatch: elapsed time.
    pub current: Duration,
    /// The target duration (for countdown).
    pub target: Duration,
    /// Tick interval.
    pub interval: Duration,
    /// Timer mode.
    pub mode: Mode,
    /// Whether auto-start is enabled.
    pub auto_start: bool,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component instance IDs.
    pub ids: ComponentIds,
    /// Backend used for locale-aware digit formatting of the displayed time.
    pub intl_backend: Arc<dyn IntlBackend>,
}
```

`Context` therefore implements `Clone` only via derive; `Debug` and `PartialEq`
are provided manually and exclude `intl_backend` (an injected service, not
observable state), mirroring `TimeField`.

### 1.4 Props

```rust
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
```

### 1.5 Effects

The recurring tick and the completion announcement are side effects the agnostic core cannot
perform itself (it has no DOM and no recurring-interval primitive — `PlatformEffects` exposes only
`set_timeout`). They are therefore emitted as typed **marker effects** that framework adapters
translate into real platform calls, exactly like `clipboard::Effect::FeedbackTimer` and
`toast::single::Effect::DurationTimer`.

```rust
/// Typed effect intents emitted by the timer machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts a recurring interval of `Context::interval`
    /// milliseconds that dispatches `Event::Tick` on each elapse. Emitted on
    /// initial mount when the timer auto-starts (via `Machine::initial_effects`)
    /// and on every transition into `State::Running`. Cancelled whenever the
    /// timer leaves `State::Running` (pause, completion, reset, or restart);
    /// the adapter no-ops the cancellation when no interval is active.
    TimerInterval,

    /// Adapter announces the `Messages::completed_announcement` message into a
    /// polite `aria-live` region. Emitted on the countdown transition into
    /// `State::Completed`.
    AnnounceCompleted,
}
```

### 1.6 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let current = match props.mode {
            Mode::Countdown => props.target,
            Mode::Stopwatch => Duration::ZERO,
        };

        // A zero-duration countdown is already complete; it must not enter
        // `Running` (auto_start is ignored in that degenerate case).
        let state = if is_instantly_complete(props.mode, props.target) {
            State::Completed
        } else if props.auto_start {
            State::Running
        } else {
            State::Idle
        };

        (state, Context {
            current,
            target: props.target,
            // A zero interval would make every tick a no-op, so clamp to 1ms.
            interval: effective_interval(props.interval),
            mode: props.mode,
            auto_start: props.auto_start,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
            intl_backend: Arc::clone(&env.intl_backend),
        })
    }

    // Sync the context-backed props after a controlled update so transitions
    // (Tick/Reset/Restart/SyncProps) read the latest target/interval/mode.
    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(old.id, new.id, "timer::Props.id must remain stable after init");
        if old.target != new.target || old.interval != new.interval || old.mode != new.mode {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }

    // Auto-started timers boot directly into `Running` without a transition,
    // so the interval intent is emitted here on first mount.
    fn initial_effects(
        state: &Self::State,
        _ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        let mut effects = Vec::new();
        if matches!(state, State::Running) {
            effects.push(PendingEffect::named(Effect::TimerInterval));
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
            // Idle start, paused resume, and paused start all enter `Running`
            // and (re)emit the interval intent.
            (State::Idle, Event::Start)
            | (State::Paused, Event::Resume | Event::Start) => {
                Some(TransitionPlan::to(State::Running)
                    .with_effect(PendingEffect::named(Effect::TimerInterval)))
            }

            (State::Running, Event::Tick) => match ctx.mode {
                Mode::Countdown => {
                    let new = ctx.current.saturating_sub(ctx.interval);
                    if new.is_zero() {
                        Some(TransitionPlan::to(State::Completed)
                            .apply(|ctx| { ctx.current = Duration::ZERO; })
                            .cancel_effect(Effect::TimerInterval)
                            .with_effect(PendingEffect::named(Effect::AnnounceCompleted)))
                    } else {
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.current = new;
                        }))
                    }
                }
                Mode::Stopwatch => {
                    let interval = ctx.interval;
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.current = ctx.current.saturating_add(interval);
                    }))
                }
            },

            (State::Running, Event::Pause) => {
                Some(TransitionPlan::to(State::Paused).cancel_effect(Effect::TimerInterval))
            }

            (_, Event::Reset) => {
                let initial = initial_duration(ctx.mode, ctx.target);
                // A zero-duration countdown resets to Completed, not Idle, so it
                // cannot be (re)started into a running 00:00.
                let target_state = if is_instantly_complete(ctx.mode, ctx.target) {
                    State::Completed
                } else {
                    State::Idle
                };
                Some(TransitionPlan::to(target_state)
                    .apply(move |ctx| { ctx.current = initial; })
                    .cancel_effect(Effect::TimerInterval))
            }

            (_, Event::Restart) => {
                let initial = initial_duration(ctx.mode, ctx.target);
                // Restarting a zero-duration countdown completes immediately.
                if is_instantly_complete(ctx.mode, ctx.target) {
                    Some(TransitionPlan::to(State::Completed)
                        .apply(move |ctx| { ctx.current = initial; })
                        .cancel_effect(Effect::TimerInterval))
                } else {
                    Some(TransitionPlan::to(State::Running)
                        .apply(move |ctx| { ctx.current = initial; })
                        .cancel_effect(Effect::TimerInterval)
                        .with_effect(PendingEffect::named(Effect::TimerInterval)))
                }
            }

            (_, Event::SetTime(duration)) => {
                let duration = *duration;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.current = duration;
                }))
            }

            (_, Event::SyncProps) => {
                let target = props.target;
                let interval = effective_interval(props.interval);
                let mode = props.mode;
                // Syncing into a zero-duration countdown completes on arrival.
                if is_instantly_complete(mode, target) {
                    Some(TransitionPlan::to(State::Completed)
                        .apply(move |ctx| {
                            ctx.target = target;
                            ctx.interval = interval;
                            ctx.mode = mode;
                            ctx.current = Duration::ZERO;
                        })
                        .cancel_effect(Effect::TimerInterval))
                } else {
                    // Idle mirrors the new initial; running/paused keep `current`.
                    let reset_current = matches!(state, State::Idle);
                    // A live cadence change re-arms the adapter interval.
                    let rearm = matches!(state, State::Running) && interval != ctx.interval;
                    let mut plan = TransitionPlan::context_only(move |ctx| {
                        ctx.target = target;
                        ctx.interval = interval;
                        ctx.mode = mode;
                        if reset_current {
                            ctx.current = initial_duration(mode, target);
                        }
                    });
                    if rearm {
                        plan = plan
                            .cancel_effect(Effect::TimerInterval)
                            .with_effect(PendingEffect::named(Effect::TimerInterval));
                    }
                    Some(plan)
                }
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
        Api { state, ctx, props, send }
    }
}

/// The initial `current` value for a given mode and target.
const fn initial_duration(mode: Mode, target: Duration) -> Duration {
    match mode {
        Mode::Countdown => target,
        Mode::Stopwatch => Duration::ZERO,
    }
}

/// Replaces a zero interval with a 1ms floor so ticks always make progress.
const fn effective_interval(interval: Duration) -> Duration {
    if interval.is_zero() { Duration::from_millis(1) } else { interval }
}

/// Whether a `(mode, target)` describes a countdown that is already complete.
const fn is_instantly_complete(mode: Mode, target: Duration) -> bool {
    matches!(mode, Mode::Countdown) && target.is_zero()
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "timer"]
pub enum Part {
    Root,
    Label,
    Display,
    Progress,
    StartTrigger,
    PauseTrigger,
    ResetTrigger,
    Separator,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_running(&self) -> bool { *self.state == State::Running }
    pub fn is_paused(&self) -> bool { *self.state == State::Paused }
    pub fn is_completed(&self) -> bool { *self.state == State::Completed }
    pub fn is_idle(&self) -> bool { *self.state == State::Idle }
    pub fn current(&self) -> Duration { self.ctx.current }

    /// Current time broken into hours, minutes, seconds, milliseconds.
    pub fn display_time(&self) -> (u64, u64, u64, u64) {
        let ms = self.ctx.current.as_millis() as u64;
        let hours = ms / 3_600_000;
        let minutes = (ms % 3_600_000) / 60_000;
        let seconds = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;
        (hours, minutes, seconds, millis)
    }

    /// Progress as a fraction [0.0, 1.0].
    ///
    /// Clamped so an over-target stopwatch or out-of-range `SetTime` never
    /// produces a value outside [0.0, 1.0] (which would break the progressbar
    /// `aria-valuenow`/`valuemin`/`valuemax` semantics).
    pub fn progress(&self) -> f64 {
        if self.ctx.target.is_zero() { return 0.0; }
        let fraction = self.ctx.current.as_secs_f64() / self.ctx.target.as_secs_f64();
        let progress = match self.ctx.mode {
            Mode::Countdown => 1.0 - fraction,
            Mode::Stopwatch => fraction,
        };
        progress.clamp(0.0, 1.0)
    }

    /// Formatted time string (HH:MM:SS or MM:SS).
    ///
    /// Digits are rendered through `intl_backend.format_segment_digits` so
    /// non-ASCII numbering systems are honored; the colon separator is fixed.
    pub fn formatted_time(&self) -> String {
        let (hours, minutes, seconds, _) = self.display_time();
        let width = NonZeroU8::new(2).expect("segment width is non-zero");
        let segment = |value: u64| {
            u32::try_from(value).map_or_else(
                |_| value.to_string(),
                |value| self.ctx.intl_backend.format_segment_digits(value, width, &self.ctx.locale),
            )
        };
        if hours > 0 {
            format!("{}:{}:{}", segment(hours), segment(minutes), segment(seconds))
        } else {
            format!("{}:{}", segment(minutes), segment(seconds))
        }
    }

    fn state_str(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Running => "running",
            State::Paused => "paused",
            State::Completed => "completed",
        }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "timer");
        attrs.set(HtmlAttr::Data("ars-state"), self.state_str());
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.formatted_time());
        attrs
    }

    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    pub fn display_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Display.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("display"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn progress_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Progress.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "progressbar");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.0}", self.progress() * 100.0));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), "100");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.progress_label)(&self.ctx.locale));
        attrs.set_style(CssProperty::Custom("ars-timer-progress"), format!("{:.2}", self.progress()));
        attrs
    }

    pub fn start_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), if self.is_paused() {
            (self.ctx.messages.resume_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.start_label)(&self.ctx.locale)
        });
        if self.is_running() || self.is_completed() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn pause_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PauseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.pause_label)(&self.ctx.locale));
        if !self.is_running() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn reset_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ResetTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.reset_label)(&self.ctx.locale));
        if self.is_idle() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn on_start_trigger_click(&self) {
        if self.is_paused() {
            (self.send)(Event::Resume);
        } else {
            (self.send)(Event::Start);
        }
    }
    pub fn on_pause_trigger_click(&self) { (self.send)(Event::Pause); }
    pub fn on_reset_trigger_click(&self) { (self.send)(Event::Reset); }
    pub fn on_restart(&self) { (self.send)(Event::Restart); }
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
```

## 2. Anatomy

```text
Timer
├── Root             (required — role="timer", aria-live region)
├── Label            (optional — describes the timer)
├── Display          (required — formatted time string)
├── Progress         (optional — role="progressbar")
├── StartTrigger     (required — start/resume button)
├── PauseTrigger     (required — pause button)
├── ResetTrigger     (required — reset button)
└── Separator        (optional — decorative colon between time segments)
```

| Part         | Element    | Key Attributes                                                        |
| ------------ | ---------- | --------------------------------------------------------------------- |
| Root         | `<div>`    | `role="timer"`, `aria-live="polite"`, `aria-atomic`                   |
| Label        | `<label>`  | `id` for association                                                  |
| Display      | `<span>`   | `aria-hidden="true"` (Root handles live region)                       |
| Progress     | `<div>`    | `role="progressbar"`, `aria-valuenow/min/max`                         |
| StartTrigger | `<button>` | `type="button"`, `aria-label` (Start/Resume), `disabled` when running |
| PauseTrigger | `<button>` | `type="button"`, `aria-label` (Pause), `disabled` when not running    |
| ResetTrigger | `<button>` | `type="button"`, `aria-label` (Reset), `disabled` when idle           |
| Separator    | `<span>`   | `aria-hidden="true"` (decorative)                                     |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part         | Role          | Properties                                               |
| ------------ | ------------- | -------------------------------------------------------- |
| Root         | `timer`       | `aria-live="polite"`, `aria-atomic="true"`, `aria-label` |
| Progress     | `progressbar` | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`        |
| StartTrigger | `button`      | `aria-label`, `disabled`                                 |
| PauseTrigger | `button`      | `aria-label`, `disabled`                                 |
| ResetTrigger | `button`      | `aria-label`, `disabled`                                 |

### 3.2 Keyboard Interaction

| Key           | Action                  |
| ------------- | ----------------------- |
| Enter / Space | Activate focused button |
| Tab           | Move between controls   |

### 3.3 Screen Reader Announcements

The Root element has `role="timer"` with `aria-live="polite"` and `aria-atomic="true"`. Time changes are announced periodically. When countdown completes, the `completed_announcement` message is announced.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
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
```

| Key                            | Default (en-US)     | Purpose                    |
| ------------------------------ | ------------------- | -------------------------- |
| `timer.start_label`            | `"Start timer"`     | Start button label         |
| `timer.resume_label`           | `"Resume timer"`    | Resume button label        |
| `timer.pause_label`            | `"Pause timer"`     | Pause button label         |
| `timer.reset_label`            | `"Reset timer"`     | Reset button label         |
| `timer.progress_label`         | `"Timer progress"`  | Progress bar label         |
| `timer.completed_announcement` | `"Timer completed"` | Screen reader announcement |

- **Number formatting**: Time display uses locale-aware number formatting for the digits. The colon separator may vary by locale convention.
- **RTL**: Timer display and button layout reverse in RTL. The progress bar fills from inline-end toward inline-start.

## 5. Library Parity

> Compared against: Ark UI (`Timer`).

### 5.1 Props

| Feature            | ars-ui                       | Ark UI                | Notes                               |
| ------------------ | ---------------------------- | --------------------- | ----------------------------------- |
| `autoStart`        | `auto_start`                 | `autoStart`           | Equivalent                          |
| `countdown` / mode | `mode` (Countdown/Stopwatch) | `countdown` (boolean) | Equivalent (ars-ui uses enum)       |
| `interval`         | `interval` (`Duration`)      | `interval`            | Equivalent (ars-ui uses `Duration`) |
| `startMs`          | --                           | `startMs`             | Ark can start from arbitrary offset |
| `targetMs`         | `target` (`Duration`)        | `targetMs`            | Equivalent (ars-ui uses `Duration`) |

**Gaps:** None. `startMs` is niche; ars-ui can achieve via `Event::SetTime`.

### 5.2 Anatomy

| Part         | ars-ui                   | Ark UI                 | Notes                                               |
| ------------ | ------------------------ | ---------------------- | --------------------------------------------------- |
| Root         | `Root` (role="timer")    | `Root`                 | Equivalent                                          |
| Label        | `Label`                  | --                     | ars-ui has label                                    |
| Display      | `Display`                | `Area` + `Item`        | Ark composes time segments; ars-ui has display part |
| Progress     | `Progress` (progressbar) | --                     | ars-ui has progress bar                             |
| StartTrigger | `StartTrigger`           | `ActionTrigger(start)` | Equivalent                                          |
| PauseTrigger | `PauseTrigger`           | `ActionTrigger(pause)` | Equivalent                                          |
| ResetTrigger | `ResetTrigger`           | `ActionTrigger(reset)` | Equivalent                                          |
| Separator    | `Separator`              | `Separator`            | Equivalent                                          |
| Control      | --                       | `Control`              | Ark has wrapper for action triggers                 |
| Item         | --                       | `Item` (per time unit) | Ark renders each unit separately                    |

**Gaps:** None. Ark's `Item` per-unit rendering and `Control` wrapper are layout patterns the adapter handles.

### 5.3 Events

| Callback | ars-ui                        | Ark UI       | Notes      |
| -------- | ----------------------------- | ------------ | ---------- |
| Complete | `State::Completed` transition | `onComplete` | Equivalent |
| Tick     | `Event::Tick`                 | `onTick`     | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                  | ars-ui            | Ark UI         |
| ------------------------ | ----------------- | -------------- |
| Countdown mode           | Yes               | Yes            |
| Stopwatch mode           | Yes               | Yes            |
| Start/Pause/Resume/Reset | Yes               | Yes            |
| Restart (reset + start)  | Yes               | Yes            |
| Auto-start               | Yes               | Yes            |
| Progress indicator       | Yes (progressbar) | No             |
| Completion announcement  | Yes (aria-live)   | No             |
| Formatted time display   | Yes (HH:MM:SS)    | Yes (per-unit) |

**Gaps:** None. ars-ui exceeds Ark UI with progress bar and SR announcements.

### 5.5 Summary

- **Overall:** Full parity, with additional accessibility features.
- **Divergences:** Ark UI renders time as individual `Item` parts per time unit (hours, minutes, seconds); ars-ui uses a single `Display` part with `formatted_time()`. ars-ui adds a `Progress` progressbar and screen reader completion announcement.
- **Recommended additions:** None.
