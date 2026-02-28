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
    SetTime(u64),
}
```

### 1.3 Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current time value in milliseconds.
    /// For countdown: remaining time. For stopwatch: elapsed time.
    pub current_ms: u64,
    /// The target duration in milliseconds (for countdown).
    pub target_ms: u64,
    /// Tick interval in milliseconds.
    pub interval_ms: u32,
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
}
```

### 1.4 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Target duration in milliseconds (for countdown mode).
    pub target_ms: u64,
    /// Tick interval in milliseconds.
    pub interval_ms: u32,
    /// Timer mode (countdown or stopwatch).
    pub mode: Mode,
    /// Auto-start when mounted.
    pub auto_start: bool,
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            target_ms: 60_000,
            interval_ms: 1000,
            mode: Mode::Countdown,
            auto_start: false,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let current_ms = match props.mode {
            Mode::Countdown => props.target_ms,
            Mode::Stopwatch => 0,
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

        let state = if props.auto_start {
            State::Running
        } else {
            State::Idle
        };

        (state, Context {
            current_ms,
            target_ms: props.target_ms,
            interval_ms: props.interval_ms,
            mode: props.mode,
            auto_start: props.auto_start,
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Idle, Event::Start) => {
                Some(TransitionPlan::to(State::Running).with_named_effect("timer-interval", |ctx, _props, send| {
                    let interval = ctx.interval_ms;
                    let timer = set_interval(interval, move || {
                        send(Event::Tick);
                    });
                    Box::new(move || cancel_interval(timer))
                }))
            }

            (State::Running, Event::Tick) => {
                match ctx.mode {
                    Mode::Countdown => {
                        let new_ms = ctx.current_ms.saturating_sub(ctx.interval_ms as u64);
                        if new_ms == 0 {
                            Some(TransitionPlan::to(State::Completed).apply(|ctx| {
                                ctx.current_ms = 0;
                            }).with_named_effect("announce", |ctx, _props, _send| {
                                let platform = use_platform_effects();
                                platform.announce(&(ctx.messages.completed_announcement)(&ctx.locale));
                                no_cleanup()
                            }))
                        } else {
                            Some(TransitionPlan::context_only(move |ctx| {
                                ctx.current_ms = new_ms;
                            }))
                        }
                    }
                    Mode::Stopwatch => {
                        let interval = ctx.interval_ms as u64;
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.current_ms += interval;
                        }))
                    }
                }
            }

            (State::Running, Event::Pause) => {
                Some(TransitionPlan::to(State::Paused))
            }

            (State::Paused, Event::Resume)
            | (State::Paused, Event::Start) => {
                Some(TransitionPlan::to(State::Running).with_named_effect("timer-interval", |ctx, _props, send| {
                    let interval = ctx.interval_ms;
                    let timer = set_interval(interval, move || {
                        send(Event::Tick);
                    });
                    Box::new(move || cancel_interval(timer))
                }))
            }

            (_, Event::Reset) => {
                let initial_ms = match ctx.mode {
                    Mode::Countdown => ctx.target_ms,
                    Mode::Stopwatch => 0,
                };
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.current_ms = initial_ms;
                }))
            }

            (_, Event::Restart) => {
                let initial_ms = match ctx.mode {
                    Mode::Countdown => ctx.target_ms,
                    Mode::Stopwatch => 0,
                };
                Some(TransitionPlan::to(State::Running).apply(move |ctx| {
                    ctx.current_ms = initial_ms;
                }).with_named_effect("timer-interval", |ctx, _props, send| {
                    let interval = ctx.interval_ms;
                    let timer = set_interval(interval, move || {
                        send(Event::Tick);
                    });
                    Box::new(move || cancel_interval(timer))
                }))
            }

            (_, Event::SetTime(ms)) => {
                let ms = *ms;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.current_ms = ms;
                }))
            }

            _ => None,
        }
    }

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

### 1.6 Connect / API

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
    pub fn current_ms(&self) -> u64 { self.ctx.current_ms }

    /// Current time broken into hours, minutes, seconds, milliseconds.
    pub fn display_time(&self) -> (u64, u64, u64, u64) {
        let ms = self.ctx.current_ms;
        let hours = ms / 3_600_000;
        let minutes = (ms % 3_600_000) / 60_000;
        let seconds = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;
        (hours, minutes, seconds, millis)
    }

    /// Progress as a fraction [0.0, 1.0] (countdown only).
    pub fn progress(&self) -> f64 {
        if self.ctx.target_ms == 0 { return 0.0; }
        match self.ctx.mode {
            Mode::Countdown => 1.0 - (self.ctx.current_ms as f64 / self.ctx.target_ms as f64),
            Mode::Stopwatch => self.ctx.current_ms as f64 / self.ctx.target_ms as f64,
        }
    }

    /// Formatted time string (HH:MM:SS or MM:SS).
    pub fn formatted_time(&self) -> String {
        let (h, m, s, _) = self.display_time();
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
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

| Part         | Element    | Key Attributes                                       |
| ------------ | ---------- | ---------------------------------------------------- |
| Root         | `<div>`    | `role="timer"`, `aria-live="polite"`, `aria-atomic`  |
| Label        | `<label>`  | `id` for association                                 |
| Display      | `<span>`   | `aria-hidden="true"` (Root handles live region)      |
| Progress     | `<div>`    | `role="progressbar"`, `aria-valuenow/min/max`        |
| StartTrigger | `<button>` | `aria-label` (Start/Resume), `disabled` when running |
| PauseTrigger | `<button>` | `aria-label` (Pause), `disabled` when not running    |
| ResetTrigger | `<button>` | `aria-label` (Reset), `disabled` when idle           |
| Separator    | `<span>`   | `aria-hidden="true"` (decorative)                    |

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
| `interval`         | `interval_ms`                | `interval`            | Equivalent                          |
| `startMs`          | --                           | `startMs`             | Ark can start from arbitrary offset |
| `targetMs`         | `target_ms`                  | `targetMs`            | Equivalent                          |

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
