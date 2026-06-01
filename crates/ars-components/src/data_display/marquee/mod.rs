//! Marquee data-display component machine.
//!
//! Owns play/pause state, loop counting, interaction pause flags, and
//! semantic attributes. Adapters perform live content measurement, animation
//! wiring, and reduced-motion media query detection.

use alloc::{format, string::String};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, CssProperty,
    Env, HasId, HtmlAttr, Locale, MessageFn, NoEffect, TransitionPlan,
};

/// States for the `Marquee` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The marquee is currently scrolling.
    Playing,

    /// The marquee is paused.
    Paused,
}

/// Events accepted by the `Marquee` machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Start or resume scrolling.
    Play,

    /// Pause scrolling.
    Pause,

    /// Pointer entered the root element.
    HoverIn,

    /// Pointer left the root element.
    HoverOut,

    /// Focus moved into the root element.
    FocusIn,

    /// Focus moved out of the root element.
    FocusOut,

    /// One full loop of the content completed.
    LoopComplete,

    /// Synchronize context-backed values after props change.
    SyncProps,
}

/// Scroll direction for the `Marquee` content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Direction {
    /// The marquee is scrolling from left to right.
    #[default]
    Left,

    /// The marquee is scrolling from right to left.
    Right,

    /// The marquee is scrolling from top to bottom.
    Up,

    /// The marquee is scrolling from bottom to top.
    Down,
}

impl Direction {
    /// Returns the data and CSS token for this direction.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

/// Props for the `Marquee` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Scroll speed in pixels per second.
    pub speed: f64,

    /// Scroll direction.
    pub direction: Direction,

    /// Gap in pixels between the original and duplicated content.
    pub gap: f64,

    /// Whether to pause on pointer hover.
    pub pause_on_hover: bool,

    /// Whether to pause when the component receives focus.
    pub pause_on_focus: bool,

    /// Maximum number of loops. `None` means infinite.
    pub loop_count: Option<usize>,

    /// Whether adapters should duplicate content to fill the viewport.
    pub auto_fill: bool,

    /// Delay in seconds before the animation starts.
    pub delay: f64,

    /// Whether scrolling starts automatically.
    pub auto_play: bool,

    /// Whether the component is disabled.
    pub disabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            speed: 50.0,
            direction: Direction::Left,
            gap: 40.0,
            pause_on_hover: true,
            pause_on_focus: true,
            loop_count: None,
            auto_fill: false,
            delay: 0.0,
            auto_play: true,
            disabled: false,
        }
    }
}

impl Props {
    /// Returns fresh `Marquee` props with documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets scroll speed in pixels per second.
    #[must_use]
    pub const fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    /// Sets scroll direction.
    #[must_use]
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the gap in pixels between the original and duplicated content.
    #[must_use]
    pub const fn gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    /// Enables or disables pointer-hover pausing.
    #[must_use]
    pub const fn pause_on_hover(mut self, pause_on_hover: bool) -> Self {
        self.pause_on_hover = pause_on_hover;
        self
    }

    /// Enables or disables focus-within pausing.
    #[must_use]
    pub const fn pause_on_focus(mut self, pause_on_focus: bool) -> Self {
        self.pause_on_focus = pause_on_focus;
        self
    }

    /// Sets a finite loop count.
    #[must_use]
    pub const fn loop_count(mut self, loop_count: usize) -> Self {
        self.loop_count = Some(loop_count);
        self
    }

    /// Sets the optional loop count directly.
    #[must_use]
    pub const fn loop_count_option(mut self, loop_count: Option<usize>) -> Self {
        self.loop_count = loop_count;
        self
    }

    /// Enables or disables adapter auto-fill behavior.
    #[must_use]
    pub const fn auto_fill(mut self, auto_fill: bool) -> Self {
        self.auto_fill = auto_fill;
        self
    }

    /// Sets the animation delay in seconds.
    #[must_use]
    pub const fn delay(mut self, delay: f64) -> Self {
        self.delay = delay;
        self
    }

    /// Enables or disables initial auto-play.
    #[must_use]
    pub const fn auto_play(mut self, auto_play: bool) -> Self {
        self.auto_play = auto_play;
        self
    }

    /// Enables or disables the component.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Context for the `Marquee` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Scroll speed in pixels per second.
    pub speed: f64,

    /// Scroll direction.
    pub direction: Direction,

    /// Gap in pixels between the original and duplicated content.
    pub gap: f64,

    /// Whether to pause on pointer hover.
    pub pause_on_hover: bool,

    /// Whether to pause when the component receives focus.
    pub pause_on_focus: bool,

    /// Maximum number of loops. `None` means infinite.
    pub loop_count: Option<usize>,

    /// Whether adapters should duplicate content to fill the viewport.
    pub auto_fill: bool,

    /// Delay in seconds before the animation starts.
    pub delay: f64,

    /// Number of completed loops.
    pub current_loop: usize,

    /// Whether the pause was triggered by a hover event.
    pub paused_by_hover: bool,

    /// Whether the pause was triggered by a focus event.
    pub paused_by_focus: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved messages for the marquee.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// Localizable strings for the `Marquee` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the pause control.
    pub pause_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the play control.
    pub play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the scrolling content region.
    pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            pause_label: MessageFn::static_str("Pause scrolling"),
            play_label: MessageFn::static_str("Start scrolling"),
            region_label: MessageFn::static_str("Scrolling content"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Structural parts exposed by the `Marquee` connect API.
#[derive(ComponentPart)]
#[scope = "marquee"]
pub enum Part {
    /// The root viewport and region element.
    Root,

    /// The duplicated scrolling content element.
    Content,

    /// A decorative fade edge on the marquee viewport.
    Edge {
        /// The logical edge where the overlay is placed.
        side: EdgeSide,
    },

    /// The play/pause button.
    AutoPlayTrigger,
}

/// Which edge of the marquee viewport a gradient overlay is placed on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EdgeSide {
    /// The start edge.
    #[default]
    Start,

    /// The end edge.
    End,
}

impl EdgeSide {
    /// Returns the data-attribute token for this edge side.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Machine for the `Marquee` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let state = if props.auto_play && !props.disabled && props.loop_count != Some(0) {
            State::Playing
        } else {
            State::Paused
        };

        (
            state,
            Context {
                speed: props.speed,
                direction: props.direction,
                gap: props.gap,
                pause_on_hover: props.pause_on_hover,
                pause_on_focus: props.pause_on_focus,
                loop_count: props.loop_count,
                auto_fill: props.auto_fill,
                delay: props.delay,
                current_loop: 0,
                paused_by_hover: false,
                paused_by_focus: false,
                disabled: props.disabled,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && !matches!(event, Event::SyncProps) {
            return None;
        }

        match (state, event) {
            (State::Paused, Event::Play) if !finite_loop_exhausted(ctx) => Some(
                TransitionPlan::to(State::Playing).apply(|ctx: &mut Context| {
                    ctx.paused_by_hover = false;
                    ctx.paused_by_focus = false;
                }),
            ),

            (State::Playing, Event::Pause) => Some(TransitionPlan::to(State::Paused).apply(
                |ctx: &mut Context| {
                    ctx.paused_by_hover = false;
                    ctx.paused_by_focus = false;
                },
            )),

            (State::Paused, Event::Pause) if ctx.paused_by_hover || ctx.paused_by_focus => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.paused_by_hover = false;
                    ctx.paused_by_focus = false;
                }))
            }

            (State::Playing, Event::HoverIn) if ctx.pause_on_hover => Some(
                TransitionPlan::to(State::Paused).apply(|ctx: &mut Context| {
                    ctx.paused_by_hover = true;
                }),
            ),

            (State::Paused, Event::HoverIn)
                if ctx.pause_on_hover && ctx.paused_by_focus && !ctx.paused_by_hover =>
            {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.paused_by_hover = true;
                }))
            }

            (State::Paused, Event::HoverOut) if ctx.paused_by_hover => Some(
                TransitionPlan::to(if ctx.paused_by_focus {
                    State::Paused
                } else {
                    State::Playing
                })
                .apply(|ctx: &mut Context| {
                    ctx.paused_by_hover = false;
                }),
            ),

            (State::Playing, Event::FocusIn) if ctx.pause_on_focus => Some(
                TransitionPlan::to(State::Paused).apply(|ctx: &mut Context| {
                    ctx.paused_by_focus = true;
                }),
            ),

            (State::Paused, Event::FocusIn)
                if ctx.pause_on_focus && ctx.paused_by_hover && !ctx.paused_by_focus =>
            {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.paused_by_focus = true;
                }))
            }

            (State::Paused, Event::FocusOut) if ctx.paused_by_focus => Some(
                TransitionPlan::to(if ctx.paused_by_hover {
                    State::Paused
                } else {
                    State::Playing
                })
                .apply(|ctx: &mut Context| {
                    ctx.paused_by_focus = false;
                }),
            ),

            (State::Playing, Event::LoopComplete) => {
                let next_loop = next_loop_count(ctx);
                let exhausted = ctx.loop_count.is_some_and(|max| next_loop >= max);

                if exhausted {
                    Some(
                        TransitionPlan::to(State::Paused).apply(move |ctx: &mut Context| {
                            ctx.current_loop = next_loop;
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.current_loop = next_loop;
                    }))
                }
            }

            (_, Event::SyncProps) => {
                let next_current_loop = props
                    .loop_count
                    .map_or(ctx.current_loop, |max| ctx.current_loop.min(max));
                let finite_loop_exhausted =
                    props.loop_count.is_some_and(|max| next_current_loop >= max);
                let next_state = if props.disabled || finite_loop_exhausted {
                    State::Paused
                } else {
                    state.clone()
                };
                let speed = props.speed;
                let direction = props.direction;
                let gap = props.gap;
                let pause_on_hover = props.pause_on_hover;
                let pause_on_focus = props.pause_on_focus;
                let loop_count = props.loop_count;
                let auto_fill = props.auto_fill;
                let delay = props.delay;
                let disabled = props.disabled;
                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        ctx.speed = speed;
                        ctx.direction = direction;
                        ctx.gap = gap;
                        ctx.pause_on_hover = pause_on_hover;
                        ctx.pause_on_focus = pause_on_focus;
                        ctx.loop_count = loop_count;
                        ctx.auto_fill = auto_fill;
                        ctx.delay = delay;
                        ctx.current_loop = next_current_loop;
                        ctx.disabled = disabled;
                        if !pause_on_hover || disabled {
                            ctx.paused_by_hover = false;
                        }
                        if !pause_on_focus || disabled {
                            ctx.paused_by_focus = false;
                        }
                    }),
                )
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "marquee::Props.id must remain stable after init"
        );

        if context_relevant_props_changed(old, new) {
            vec![Event::SyncProps]
        } else {
            Vec::new()
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

fn next_loop_count(ctx: &Context) -> usize {
    let next = ctx.current_loop.saturating_add(1);
    ctx.loop_count.map_or(next, |max| next.min(max))
}

fn finite_loop_exhausted(ctx: &Context) -> bool {
    ctx.loop_count.is_some_and(|max| ctx.current_loop >= max)
}

fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.speed != new.speed
        || old.direction != new.direction
        || old.gap != new.gap
        || old.pause_on_hover != new.pause_on_hover
        || old.pause_on_focus != new.pause_on_focus
        || old.loop_count != new.loop_count
        || old.auto_fill != new.auto_fill
        || old.delay != new.delay
        || old.disabled != new.disabled
}

/// API for the `Marquee` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("marquee::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns true when the marquee is currently scrolling.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        *self.state == State::Playing
    }

    /// Returns true when the marquee is paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        *self.state == State::Paused
    }

    /// Sends the play intent.
    pub fn play(&self) {
        (self.send)(Event::Play);
    }

    /// Sends the pause intent.
    pub fn pause(&self) {
        (self.send)(Event::Pause);
    }

    /// Returns root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "marquee")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.region_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-state"), state_attr(self.state))
            .set(
                HtmlAttr::Aria(AriaAttr::Live),
                if self.is_playing() { "off" } else { "polite" },
            );

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Returns content element attributes.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set_style(
                CssProperty::Custom("ars-marquee-speed"),
                format!("{}px", self.ctx.speed),
            )
            .set_style(
                CssProperty::Custom("ars-marquee-direction"),
                self.ctx.direction.as_str(),
            )
            .set_style(
                CssProperty::Custom("ars-marquee-gap"),
                format!("{}px", self.ctx.gap),
            )
            .set_style(
                CssProperty::Custom("ars-marquee-play-state"),
                if self.is_paused() {
                    "paused"
                } else {
                    "running"
                },
            );

        if self.ctx.delay > 0.0 {
            attrs.set_style(
                CssProperty::Custom("ars-marquee-delay"),
                format!("{}s", self.ctx.delay),
            );
        }

        attrs
    }

    /// Returns decorative edge overlay attributes.
    #[must_use]
    pub fn edge_attrs(&self, side: &EdgeSide) -> AttrMap {
        let mut attrs = part_attrs(&Part::Edge { side: *side });

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Data("ars-side"), side.as_str());

        attrs
    }

    /// Returns play/pause trigger button attributes.
    #[must_use]
    pub fn auto_play_trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::AutoPlayTrigger);

        let label = if self.is_playing() {
            (self.ctx.messages.pause_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.play_label)(&self.ctx.locale)
        };

        attrs
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            .set(
                HtmlAttr::Aria(AriaAttr::Pressed),
                if self.is_playing() { "true" } else { "false" },
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Content => self.content_attrs(),
            Part::Edge { side } => self.edge_attrs(&side),
            Part::AutoPlayTrigger => self.auto_play_trigger_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_value)
        .set(part_attr, part_value);

    attrs
}

const fn state_attr(state: &State) -> &'static str {
    match state {
        State::Playing => "playing",
        State::Paused => "paused",
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU64, Ordering};

    use ars_core::{AriaAttr, AttrMap, CssProperty, Env, HtmlAttr, Locale, MessageFn, Service};
    use insta::assert_snapshot;

    use super::*;

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    fn style<'a>(attrs: &'a AttrMap, property: &CssProperty) -> Option<&'a str> {
        attrs
            .styles()
            .iter()
            .find_map(|(candidate, value)| (candidate == property).then_some(value.as_str()))
    }

    fn changed(result: &ars_core::SendResult<Machine>) -> bool {
        result.state_changed || result.context_changed
    }

    #[test]
    fn marquee_initializes_playing_paused_and_disabled_states() {
        let playing = service(Props::new().id("ticker"));

        assert_eq!(playing.state(), &State::Playing);
        assert_eq!(playing.context().speed, 50.0);
        assert_eq!(playing.context().direction, Direction::Left);
        assert_eq!(playing.context().gap, 40.0);
        assert!(playing.context().pause_on_hover);
        assert!(playing.context().pause_on_focus);

        let paused = service(Props::new().id("ticker").auto_play(false));

        assert_eq!(paused.state(), &State::Paused);

        let disabled = service(Props::new().id("ticker").disabled(true));

        assert_eq!(disabled.state(), &State::Paused);
        assert!(disabled.context().disabled);
    }

    #[test]
    fn marquee_play_and_pause_events_toggle_state() {
        let mut marquee = service(Props::new().id("ticker").auto_play(false));

        assert_eq!(marquee.state(), &State::Paused);

        assert!(changed(&marquee.send(Event::Play)));
        assert_eq!(marquee.state(), &State::Playing);

        assert!(changed(&marquee.send(Event::Pause)));
        assert_eq!(marquee.state(), &State::Paused);
    }

    #[test]
    fn marquee_hover_pause_tracks_and_resumes_only_when_enabled() {
        let mut marquee = service(Props::new().id("ticker"));

        assert!(changed(&marquee.send(Event::HoverIn)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(marquee.context().paused_by_hover);

        assert!(changed(&marquee.send(Event::HoverOut)));
        assert_eq!(marquee.state(), &State::Playing);
        assert!(!marquee.context().paused_by_hover);

        let mut disabled_hover = service(Props::new().id("ticker").pause_on_hover(false));

        assert!(!changed(&disabled_hover.send(Event::HoverIn)));
        assert_eq!(disabled_hover.state(), &State::Playing);
    }

    #[test]
    fn marquee_focus_pause_tracks_and_resumes_only_when_enabled() {
        let mut marquee = service(Props::new().id("ticker"));

        assert!(changed(&marquee.send(Event::FocusIn)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(marquee.context().paused_by_focus);

        assert!(changed(&marquee.send(Event::FocusOut)));
        assert_eq!(marquee.state(), &State::Playing);
        assert!(!marquee.context().paused_by_focus);

        let mut disabled_focus = service(Props::new().id("ticker").pause_on_focus(false));

        assert!(!changed(&disabled_focus.send(Event::FocusIn)));
        assert_eq!(disabled_focus.state(), &State::Playing);
    }

    #[test]
    fn marquee_loop_complete_counts_and_pauses_when_exhausted() {
        let mut marquee = service(Props::new().id("ticker").loop_count(2));

        assert!(changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.state(), &State::Playing);
        assert_eq!(marquee.context().current_loop, 1);

        assert!(changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().current_loop, 2);

        assert!(!changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.context().current_loop, 2);
    }

    #[test]
    fn marquee_play_does_not_restart_after_finite_loop_count_is_exhausted() {
        let mut marquee = service(Props::new().id("ticker").loop_count(1));

        assert!(changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().current_loop, 1);

        assert!(!changed(&marquee.send(Event::Play)));
        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().current_loop, 1);

        assert!(!changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.context().current_loop, 1);
    }

    #[test]
    fn marquee_hover_and_focus_pause_causes_must_all_clear_before_resume() {
        let mut marquee = service(Props::new().id("ticker"));

        assert!(changed(&marquee.send(Event::HoverIn)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(marquee.context().paused_by_hover);

        assert!(changed(&marquee.send(Event::FocusIn)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(marquee.context().paused_by_hover);
        assert!(marquee.context().paused_by_focus);

        assert!(changed(&marquee.send(Event::HoverOut)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(!marquee.context().paused_by_hover);
        assert!(marquee.context().paused_by_focus);

        assert!(changed(&marquee.send(Event::FocusOut)));
        assert_eq!(marquee.state(), &State::Playing);
        assert!(!marquee.context().paused_by_hover);
        assert!(!marquee.context().paused_by_focus);
    }

    #[test]
    fn marquee_interaction_end_preserves_manual_pause() {
        let mut manual = service(Props::new().id("ticker").auto_play(false));

        assert!(!changed(&manual.send(Event::HoverIn)));
        assert_eq!(manual.state(), &State::Paused);
        assert!(!manual.context().paused_by_hover);

        assert!(!changed(&manual.send(Event::HoverOut)));
        assert_eq!(manual.state(), &State::Paused);

        assert!(!changed(&manual.send(Event::FocusIn)));
        assert_eq!(manual.state(), &State::Paused);
        assert!(!manual.context().paused_by_focus);

        assert!(!changed(&manual.send(Event::FocusOut)));
        assert_eq!(manual.state(), &State::Paused);
    }

    #[test]
    fn marquee_pause_while_interaction_paused_prevents_interaction_resume() {
        let mut marquee = service(Props::new().id("ticker"));

        assert!(changed(&marquee.send(Event::HoverIn)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(marquee.context().paused_by_hover);

        assert!(changed(&marquee.send(Event::Pause)));
        assert_eq!(marquee.state(), &State::Paused);
        assert!(!marquee.context().paused_by_hover);

        assert!(!changed(&marquee.send(Event::HoverOut)));
        assert_eq!(marquee.state(), &State::Paused);
    }

    #[test]
    fn marquee_set_props_syncs_context_backed_values_and_disabled_state() {
        let mut marquee = service(Props::new().id("ticker"));

        drop(
            marquee.set_props(
                Props::new()
                    .id("ticker")
                    .speed(120.0)
                    .direction(Direction::Down)
                    .gap(12.0)
                    .delay(0.5)
                    .pause_on_hover(false)
                    .pause_on_focus(false)
                    .loop_count(3)
                    .auto_fill(true)
                    .disabled(true),
            ),
        );

        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().speed, 120.0);
        assert_eq!(marquee.context().direction, Direction::Down);
        assert_eq!(marquee.context().gap, 12.0);
        assert_eq!(marquee.context().delay, 0.5);
        assert!(!marquee.context().pause_on_hover);
        assert!(!marquee.context().pause_on_focus);
        assert_eq!(marquee.context().loop_count, Some(3));
        assert!(marquee.context().auto_fill);
        assert!(marquee.context().disabled);

        let api = marquee.connect(&|_| {});
        let root_attrs = api.root_attrs();
        let content_attrs = api.content_attrs();

        assert_eq!(
            root_attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
        assert_eq!(
            style(&content_attrs, &CssProperty::Custom("ars-marquee-speed")),
            Some("120px")
        );
        assert_eq!(
            style(
                &content_attrs,
                &CssProperty::Custom("ars-marquee-direction")
            ),
            Some("down")
        );
    }

    #[test]
    fn marquee_set_props_pauses_when_loop_count_shrinks_to_exhausted() {
        let mut marquee = service(Props::new().id("ticker"));

        for _ in 0..5 {
            assert!(changed(&marquee.send(Event::LoopComplete)));
        }

        assert_eq!(marquee.state(), &State::Playing);
        assert_eq!(marquee.context().current_loop, 5);

        drop(marquee.set_props(Props::new().id("ticker").loop_count(3)));

        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().current_loop, 3);

        assert!(!changed(&marquee.send(Event::Play)));
        assert_eq!(marquee.state(), &State::Paused);
    }

    #[test]
    fn marquee_disabled_ignores_runtime_events() {
        let mut marquee = service(Props::new().id("ticker").disabled(true));

        assert!(!changed(&marquee.send(Event::Play)));
        assert!(!changed(&marquee.send(Event::LoopComplete)));
        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(marquee.context().current_loop, 0);
    }

    #[test]
    fn marquee_content_attrs_reflect_motion_configuration() {
        let marquee = service(
            Props::new()
                .id("ticker")
                .speed(80.0)
                .direction(Direction::Up)
                .gap(12.5)
                .delay(1.25),
        );

        let attrs = marquee.connect(&|_| {}).content_attrs();

        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-speed")),
            Some("80px")
        );
        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-direction")),
            Some("up")
        );
        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-gap")),
            Some("12.5px")
        );
        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-delay")),
            Some("1.25s")
        );
        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-play-state")),
            Some("running")
        );
    }

    #[test]
    fn marquee_reduced_motion_can_pause_through_event() {
        let mut marquee = service(Props::new().id("ticker"));

        assert!(changed(&marquee.send(Event::Pause)));

        let attrs = marquee.connect(&|_| {}).content_attrs();

        assert_eq!(marquee.state(), &State::Paused);
        assert_eq!(
            style(&attrs, &CssProperty::Custom("ars-marquee-play-state")),
            Some("paused")
        );
    }

    #[test]
    fn marquee_root_attrs_reflect_accessible_state_and_messages() {
        let messages = Messages {
            region_label: MessageFn::static_str("Latest headlines"),
            ..Messages::default()
        };

        let marquee = Service::<Machine>::new(
            Props::new().id("ticker").disabled(true),
            &Env::default(),
            &messages,
        );

        let attrs = marquee.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("ticker"));
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("marquee"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Latest headlines")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("paused"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
    }

    #[test]
    fn marquee_auto_play_trigger_attrs_reflect_state_labels_and_disabled() {
        let messages = Messages {
            pause_label: MessageFn::static_str("Stop ticker"),
            play_label: MessageFn::static_str("Start ticker"),
            ..Messages::default()
        };

        let playing =
            Service::<Machine>::new(Props::new().id("ticker"), &Env::default(), &messages);

        let attrs = playing.connect(&|_| {}).auto_play_trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Stop ticker")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("true"));

        let paused = Service::<Machine>::new(
            Props::new().id("ticker").disabled(true),
            &Env::default(),
            &messages,
        );

        let attrs = paused.connect(&|_| {}).auto_play_trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Start ticker")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("false"));
        assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
    }

    #[test]
    fn marquee_edge_attrs_mark_decorative_sides() {
        let marquee = service(Props::new().id("ticker"));

        let api = marquee.connect(&|_| {});

        let start = api.edge_attrs(&EdgeSide::Start);
        let end = api.edge_attrs(&EdgeSide::End);

        assert_eq!(start.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(start.get(&HtmlAttr::Data("ars-side")), Some("start"));
        assert_eq!(end.get(&HtmlAttr::Data("ars-side")), Some("end"));
    }

    #[test]
    fn marquee_api_methods_dispatch_events() {
        let sent = AtomicU64::new(0);

        let marquee = service(Props::new().id("ticker"));

        let send = |event| match event {
            Event::Play => {
                sent.fetch_or(1, Ordering::SeqCst);
            }
            Event::Pause => {
                sent.fetch_or(2, Ordering::SeqCst);
            }
            Event::HoverIn => {
                sent.fetch_or(4, Ordering::SeqCst);
            }
            Event::HoverOut => {
                sent.fetch_or(8, Ordering::SeqCst);
            }
            Event::FocusIn => {
                sent.fetch_or(16, Ordering::SeqCst);
            }
            Event::FocusOut => {
                sent.fetch_or(32, Ordering::SeqCst);
            }
            Event::LoopComplete => {
                sent.fetch_or(64, Ordering::SeqCst);
            }
            Event::SyncProps => {
                sent.fetch_or(128, Ordering::SeqCst);
            }
        };

        let api = marquee.connect(&send);

        assert!(api.is_playing());
        assert!(!api.is_paused());

        api.play();
        api.pause();

        assert_eq!(sent.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn marquee_messages_accept_localized_functions() {
        let messages = Messages {
            pause_label: MessageFn::new(|_: &Locale| "Pausar".to_string()),
            play_label: MessageFn::new(|_: &Locale| "Reproduzir".to_string()),
            region_label: MessageFn::new(|_: &Locale| "Conteudo rolavel".to_string()),
        };

        let marquee = Service::<Machine>::new(
            Props::new().id("ticker").auto_play(false),
            &Env::default(),
            &messages,
        );

        let api = marquee.connect(&|_| {});

        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Conteudo rolavel")
        );
        assert_eq!(
            api.auto_play_trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Reproduzir")
        );
    }

    #[test]
    fn marquee_snapshots_cover_part_attrs() {
        let marquee = service(
            Props::new()
                .id("ticker")
                .speed(72.0)
                .direction(Direction::Right)
                .gap(16.0)
                .delay(0.5),
        );

        let api = marquee.connect(&|_| {});

        assert_snapshot!("marquee_root_playing", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!(
            "marquee_content_playing",
            snapshot_attrs(&api.content_attrs())
        );
        assert_snapshot!(
            "marquee_edge_start",
            snapshot_attrs(&api.edge_attrs(&EdgeSide::Start))
        );
        assert_snapshot!(
            "marquee_edge_end",
            snapshot_attrs(&api.edge_attrs(&EdgeSide::End))
        );
        assert_snapshot!(
            "marquee_auto_play_trigger_playing",
            snapshot_attrs(&api.auto_play_trigger_attrs())
        );

        let paused = service(Props::new().id("ticker").auto_play(false));

        let api = paused.connect(&|_| {});

        assert_snapshot!("marquee_root_paused", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!(
            "marquee_content_paused",
            snapshot_attrs(&api.content_attrs())
        );
        assert_snapshot!(
            "marquee_auto_play_trigger_paused",
            snapshot_attrs(&api.auto_play_trigger_attrs())
        );
    }
}
