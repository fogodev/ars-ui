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
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Temporarily pause auto-play (hover/focus).
    AutoPlayPause,
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
    /// A slide received focus.
    FocusSlide { index: usize },
    /// Focus left the carousel.
    Blur,
}
```

### 1.3 Context

```rust
use crate::{Bindable, Duration};
use ars_i18n::Orientation;

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

    /// Clamp or wrap an index according to `loop_nav`.
    pub fn clamp_index(&self, i: isize) -> usize {
        let n = self.slide_count.get() as isize;
        if self.loop_nav {
            ((i % n) + n) as usize % self.slide_count.get()
        } else {
            (i.max(0) as usize).min(self.slide_count.get().saturating_sub(1))
        }
    }

    pub fn can_go_prev(&self) -> bool { self.loop_nav || self.current_index() > 0 }
    pub fn can_go_next(&self) -> bool { self.loop_nav || self.current_index() + 1 < self.slide_count.get() }

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
    /// Translatable messages for accessibility labels (see §4.1 Messages). When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
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
            messages: None,
            locale: None,
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
        let initial_state = if props.auto_play.is_some() {
            State::AutoPlaying
        } else {
            State::Idle
        };
        let initial_index = props.default_index.unwrap_or(0);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ctx = Context {
            index: props.index.clone()
                .unwrap_or_else(|| Bindable::uncontrolled(initial_index)),
            slide_count: props.slide_count,
            loop_nav: props.loop_nav,
            auto_play: props.auto_play.clone(),
            auto_play_stopped: false,
            auto_play_paused: false,
            spacing: props.spacing.unwrap_or(0.0),
            slides_per_view: props.slides_per_view.unwrap_or(1.0),
            slides_per_move: props.slides_per_move.unwrap_or(1),
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

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::GoToSlide { index } => {
                if *state == State::Transitioning { return None; }
                let idx = *index;
                Some(TransitionPlan::to(State::Transitioning).apply(move |ctx| {
                    ctx.index.set(idx);
                }))
            }

            Event::GoToNext => {
                if *state == State::Transitioning || !ctx.can_go_next() { return None; }
                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);
                let stop = ctx.auto_play.as_ref().map_or(false, |o| o.stop_on_interaction);
                Some(TransitionPlan::to(State::Transitioning).apply(move |ctx| {
                    ctx.index.set(next);
                    if stop { ctx.auto_play_stopped = true; }
                }))
            }

            Event::GoToPrev => {
                if *state == State::Transitioning || !ctx.can_go_prev() { return None; }
                let step = ctx.slides_per_move as isize;
                let prev = ctx.clamp_index(ctx.current_index() as isize - step);
                let stop = ctx.auto_play.as_ref().map_or(false, |o| o.stop_on_interaction);
                Some(TransitionPlan::to(State::Transitioning).apply(move |ctx| {
                    ctx.index.set(prev);
                    if stop { ctx.auto_play_stopped = true; }
                }))
            }

            Event::TransitionEnd => {
                if ctx.auto_play.is_some() && !ctx.auto_play_stopped && !ctx.auto_play_paused {
                    Some(TransitionPlan::to(State::AutoPlaying))
                } else {
                    Some(TransitionPlan::to(State::Idle))
                }
            }

            Event::AutoPlayStart => {
                if ctx.auto_play_stopped || ctx.auto_play.is_none() { return None; }
                let interval = ctx.auto_play.as_ref()?.interval;
                Some(TransitionPlan::to(State::AutoPlaying).with_named_effect(
                    "auto-play",
                    move |_ctx, _props, send| {
                        let handle = set_interval(move || send(Event::AutoPlayTick), interval);
                        Box::new(move || clear_interval(handle))
                    },
                ))
            }

            Event::AutoPlayStop => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.auto_play_stopped = true;
                }).cancel_effect("auto-play"))
            }

            Event::AutoPlayTick => {
                if *state != State::AutoPlaying { return None; }
                let step = ctx.slides_per_move as isize;
                let next = ctx.clamp_index(ctx.current_index() as isize + step);
                Some(TransitionPlan::to(State::Transitioning).apply(move |ctx| {
                    ctx.index.set(next);
                }))
            }

            Event::AutoPlayPause => {
                let plan = if *state == State::AutoPlaying {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                Some(plan.apply(|ctx| { ctx.auto_play_paused = true; }).cancel_effect("auto-play"))
            }

            Event::AutoPlayResume => {
                if ctx.auto_play_stopped { return None; }
                if let Some(ref opts) = ctx.auto_play {
                    let interval = opts.interval;
                    return Some(TransitionPlan::to(State::AutoPlaying).apply(|ctx| {
                        ctx.auto_play_paused = false;
                    }).with_named_effect("auto-play", move |_ctx, _props, send| {
                        let handle = set_interval(move || send(Event::AutoPlayTick), interval);
                        Box::new(move || clear_interval(handle))
                    }));
                }
                Some(TransitionPlan::context_only(|ctx| { ctx.auto_play_paused = false; }))
            }

            Event::PointerDown { pos, timestamp } => {
                let p = *pos;
                let ts = *timestamp;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.drag_start_pos = Some(p);
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = Some(ts);
                }).cancel_effect("auto-play"))
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
                let delta = ctx.drag_delta;
                let velocity = ctx.swipe_velocity;
                let threshold = ctx.swipe_threshold;
                // A brisk flick (>0.5 px/ms) reduces the distance threshold.
                let effective = if velocity.abs() > 0.5 { threshold * 0.3 } else { threshold };

                let cur = ctx.current_index() as isize;
                let next_idx = if delta < -effective && ctx.can_go_next() {
                    Some(ctx.clamp_index(cur + 1))
                } else if delta > effective && ctx.can_go_prev() {
                    Some(ctx.clamp_index(cur - 1))
                } else {
                    None
                };

                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;
                    if let Some(idx) = next_idx {
                        ctx.index.set(idx);
                    }
                }))
            }

            Event::PointerCancel => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.drag_start_pos = None;
                    ctx.drag_delta = 0.0;
                    ctx.swipe_velocity = 0.0;
                    ctx.drag_last_timestamp = None;
                }))
            }

            Event::FocusSlide { index } => {
                let idx = *index;
                let current = ctx.current_index();
                let should_pause = ctx.auto_play.as_ref().map_or(false, |o| o.pause_on_focus);
                Some(TransitionPlan::context_only(move |ctx| {
                    if idx != current { ctx.index.set(idx); }
                    if should_pause { ctx.auto_play_paused = true; }
                }))
            }

            Event::Blur => {
                if ctx.auto_play_paused && !ctx.auto_play_stopped {
                    if let Some(ref opts) = ctx.auto_play {
                        let interval = opts.interval;
                        return Some(TransitionPlan::to(State::AutoPlaying).apply(|ctx| {
                            ctx.auto_play_paused = false;
                        }).with_named_effect("auto-play", move |_ctx, _props, send| {
                            let handle = set_interval(move || send(Event::AutoPlayTick), interval);
                            Box::new(move || clear_interval(handle))
                        }));
                    }
                }
                None
            }
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
        let live = if self.ctx.auto_play_stopped || self.ctx.auto_play.is_none() {
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
        let is_hidden = !is_current;
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
        attrs
    }

    pub fn indicator_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Indicator { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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
        let is_playing = !self.ctx.auto_play_stopped && !self.ctx.auto_play_paused;
        let label = if is_playing {
            (self.ctx.messages.pause_auto_play_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.start_auto_play_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if is_playing { "true" } else { "false" });
        attrs
    }

    pub fn autoplay_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_playing = !self.ctx.auto_play_stopped && !self.ctx.auto_play_paused;
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
                index: self.ctx.slide_count.get().saturating_sub(1),
            }),
            _ => {}
        }
    }

    pub fn on_prev_trigger_click(&self) { (self.send)(Event::GoToPrev); }
    pub fn on_next_trigger_click(&self) { (self.send)(Event::GoToNext); }
    pub fn on_indicator_click(&self, index: usize) { (self.send)(Event::GoToSlide { index }); }
    pub fn on_auto_play_trigger_click(&self) {
        if self.ctx.auto_play_stopped || self.ctx.auto_play_paused {
            (self.send)(Event::AutoPlayResume);
        } else {
            (self.send)(Event::AutoPlayPause);
        }
    }
    pub fn on_viewport_pointerdown(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerDown { pos, timestamp });
    }
    pub fn on_viewport_pointermove(&self, pos: f64, timestamp: f64) {
        (self.send)(Event::PointerMove { pos, timestamp });
    }
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
            Part::AutoPlayIndicator => self.autoplay_indicator_attrs(),
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
| PrevTrigger       | `<button>`  | `aria-disabled` when at boundary                        |
| NextTrigger       | `<button>`  | `aria-disabled` when at boundary                        |
| IndicatorGroup    | `<div>`     | `role="tablist"`                                        |
| Indicator         | `<button>`  | `role="tab"`, `aria-selected`                           |
| AutoPlayTrigger   | `<button>`  | `aria-pressed`                                          |
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

### 3.3 Screen Reader Announcements

- `aria-live` on `ItemGroup` is `"off"` during auto-play to prevent disruptive announcements, and `"polite"` when paused or stopped.
- Auto-play MUST pause on hover (`mouseenter`) and on focus within the carousel (`focusin`). Resumes on `mouseleave` / `focusout` unless `stop_on_interaction` triggered.
- Non-current slides receive both `aria-hidden="true"` and `inert`, ensuring they are invisible to assistive technology.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    pub carousel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub slide_role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub slide_label: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
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
