//! Tooltip disclosure and attribute machine.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Direction, Env, HtmlAttr, Locale, PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::positioning::PositioningOptions;

const OPEN_DELAY_EFFECT: &str = "tooltip-open-delay";
const CLOSE_DELAY_EFFECT: &str = "tooltip-close-delay";
const OPEN_CHANGE_EFFECT: &str = "tooltip-open-change";
const ALLOCATE_Z_INDEX_EFFECT: &str = "tooltip-allocate-z-index";
const MIN_TOUCH_AUTO_HIDE: Duration = Duration::from_secs(5);

/// The states of the tooltip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The tooltip is closed.
    #[default]
    Closed,

    /// The tooltip is waiting for its hover open delay.
    OpenPending,

    /// The tooltip is open.
    Open,

    /// The tooltip is waiting for its close delay.
    ClosePending,
}

/// The events of the tooltip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The pointer entered the trigger.
    PointerEnter,

    /// The pointer left the trigger.
    PointerLeave,

    /// The trigger gained keyboard focus.
    Focus,

    /// The trigger lost keyboard focus.
    Blur,

    /// The pointer entered the visible tooltip content.
    ContentPointerEnter,

    /// The pointer left the visible tooltip content.
    ContentPointerLeave,

    /// The hover open timer fired.
    OpenTimerFired,

    /// The close timer fired.
    CloseTimerFired,

    /// Escape requested tooltip dismissal.
    CloseOnEscape,

    /// Trigger activation requested tooltip dismissal.
    CloseOnClick,

    /// Page scroll requested tooltip dismissal.
    CloseOnScroll,

    /// Programmatic open requested immediate visibility.
    Open,

    /// Programmatic close requested immediate dismissal.
    Close,

    /// Controlled props synchronized the visible open state.
    SetControlledOpen(bool),

    /// Props changed without changing controlled visible state.
    SyncProps,

    /// Adapter supplied an allocated overlay z-index.
    SetZIndex(u32),
}

/// Runtime context for Tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,

    /// Whether the tooltip is visibly open.
    pub open: bool,

    /// The open delay for hover-triggered tooltips.
    pub open_delay: Duration,

    /// The close delay.
    pub close_delay: Duration,

    /// Whether the tooltip ignores user interaction.
    pub disabled: bool,

    /// Text direction for tooltip content.
    pub dir: Direction,

    /// Whether the pointer is over the trigger or visible content.
    pub hover_active: bool,

    /// Whether the trigger currently has keyboard focus.
    pub focus_active: bool,

    /// Positioning options forwarded to framework adapters.
    pub positioning: PositioningOptions,

    /// The ID of the trigger element.
    pub trigger_id: String,

    /// The ID of the visible content element.
    pub content_id: String,

    /// The ID of the always-rendered hidden description element.
    pub hidden_description_id: String,

    /// Resolved messages for the tooltip.
    pub messages: Messages,

    /// Adapter-allocated z-index for the positioner.
    pub z_index: Option<u32>,

    /// Touch auto-hide timeout clamped to the accessibility minimum.
    pub touch_auto_hide: Duration,
}

/// Immutable configuration for Tooltip.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled open state. When `Some`, the parent owns visible open state.
    pub open: Option<bool>,

    /// Initial uncontrolled open state.
    pub default_open: bool,

    /// Delay before a hover-triggered tooltip opens.
    pub open_delay: Duration,

    /// Delay before an open tooltip closes.
    pub close_delay: Duration,

    /// Whether the tooltip ignores user interaction.
    pub disabled: bool,

    /// Positioning options forwarded to framework adapters.
    pub positioning: PositioningOptions,

    /// Whether Escape dismisses the tooltip.
    pub close_on_escape: bool,

    /// Whether trigger activation dismisses the tooltip.
    pub close_on_click: bool,

    /// Whether page scroll dismisses the tooltip.
    pub close_on_scroll: bool,

    /// Callback invoked when user interaction requests an open-state change.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Whether content is not mounted until first opened.
    pub lazy_mount: bool,

    /// Whether content is removed from the DOM after closing.
    pub unmount_on_exit: bool,

    /// Text direction for tooltip content.
    pub dir: Direction,

    /// Auto-hide timeout for touch-triggered tooltips.
    pub touch_auto_hide: Duration,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            open_delay: Duration::from_millis(300),
            close_delay: Duration::from_millis(300),
            disabled: false,
            positioning: PositioningOptions::default(),
            close_on_escape: true,
            close_on_click: true,
            close_on_scroll: true,
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            dir: Direction::Ltr,
            touch_auto_hide: Duration::from_secs(20),
        }
    }
}

impl Props {
    /// Returns Tooltip props with documented default values.
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

    /// Sets [`open`](Self::open), the controlled open state.
    #[must_use]
    pub fn open(mut self, value: impl Into<Option<bool>>) -> Self {
        self.open = value.into();
        self
    }

    /// Sets [`default_open`](Self::default_open), the initial uncontrolled
    /// open state.
    #[must_use]
    pub const fn default_open(mut self, value: bool) -> Self {
        self.default_open = value;
        self
    }

    /// Sets [`open_delay`](Self::open_delay), the hover-triggered open
    /// delay.
    #[must_use]
    pub const fn open_delay(mut self, value: Duration) -> Self {
        self.open_delay = value;
        self
    }

    /// Sets [`close_delay`](Self::close_delay), the delay before closing
    /// after hover or focus leaves.
    #[must_use]
    pub const fn close_delay(mut self, value: Duration) -> Self {
        self.close_delay = value;
        self
    }

    /// Sets [`disabled`](Self::disabled), whether user interaction is
    /// ignored.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`positioning`](Self::positioning), the adapter-owned floating
    /// placement configuration.
    #[must_use]
    pub fn positioning(mut self, value: PositioningOptions) -> Self {
        self.positioning = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape), whether Escape
    /// dismisses the tooltip.
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`close_on_click`](Self::close_on_click), whether trigger
    /// activation dismisses the tooltip.
    #[must_use]
    pub const fn close_on_click(mut self, value: bool) -> Self {
        self.close_on_click = value;
        self
    }

    /// Sets [`close_on_scroll`](Self::close_on_scroll), whether page scroll
    /// dismisses the tooltip.
    #[must_use]
    pub const fn close_on_scroll(mut self, value: bool) -> Self {
        self.close_on_scroll = value;
        self
    }

    /// Registers [`on_open_change`](Self::on_open_change), the callback
    /// fired when user interaction requests an open-state change.
    #[must_use]
    pub fn on_open_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.on_open_change = Some(Callback::new(f));
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount), whether content is mounted
    /// only after the tooltip first opens.
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit), whether content is
    /// removed from the DOM after closing.
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Sets [`dir`](Self::dir), the text direction for tooltip content.
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`touch_auto_hide`](Self::touch_auto_hide), the adapter-owned
    /// auto-hide timeout for touch-triggered tooltips.
    #[must_use]
    pub const fn touch_auto_hide(mut self, value: Duration) -> Self {
        self.touch_auto_hide = value;
        self
    }
}

/// Localizable Tooltip messages.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// The Tooltip state machine.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);

        let open = props.open.unwrap_or(props.default_open);

        let content_id = ids.part("content");

        let state = if open { State::Open } else { State::Closed };

        (
            state,
            Context {
                locale: env.locale.clone(),
                open,
                open_delay: props.open_delay,
                close_delay: close_delay(props),
                disabled: props.disabled,
                dir: props.dir,
                hover_active: false,
                focus_active: false,
                positioning: props.positioning.clone(),
                trigger_id: ids.part("trigger"),
                hidden_description_id: format_description_id(&content_id),
                content_id,
                messages: messages.clone(),
                z_index: None,
                touch_auto_hide: props.touch_auto_hide.max(MIN_TOUCH_AUTO_HIDE),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && disabled_blocks_event(*event) {
            return None;
        }

        match (state, event) {
            (State::Closed, Event::PointerEnter) => Some(
                TransitionPlan::to(State::OpenPending)
                    .apply(|ctx: &mut Context| {
                        ctx.hover_active = true;
                    })
                    .with_effect(PendingEffect::named(OPEN_DELAY_EFFECT)),
            ),

            (State::Closed, Event::Focus) => Some(open_plan(props, |ctx| {
                ctx.focus_active = true;
            })),

            (State::Closed, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = false;
                }))
            }

            (State::OpenPending, Event::OpenTimerFired) => Some(
                open_plan(props, |ctx| {
                    ctx.hover_active = true;
                })
                .cancel_effect(OPEN_DELAY_EFFECT),
            ),

            (State::OpenPending, Event::PointerEnter) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = true;
                }))
            }

            (State::OpenPending, Event::Focus) => Some(
                open_plan(props, |ctx| {
                    ctx.focus_active = true;
                })
                .cancel_effect(OPEN_DELAY_EFFECT),
            ),

            (State::OpenPending, Event::PointerLeave) if ctx.focus_active => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = false;
                }))
            }

            (State::OpenPending, Event::PointerLeave) => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.hover_active = false;
                    })
                    .cancel_effect(OPEN_DELAY_EFFECT),
            ),

            (State::OpenPending, Event::Blur) if ctx.hover_active => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = false;
                }))
            }

            (State::OpenPending, Event::Blur) => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    })
                    .cancel_effect(OPEN_DELAY_EFFECT),
            ),

            (
                State::OpenPending,
                Event::CloseOnEscape | Event::CloseOnClick | Event::CloseOnScroll,
            ) if should_close_pending(*event, props) => Some(
                TransitionPlan::to(State::Closed)
                    .apply(clear_activity)
                    .cancel_effect(OPEN_DELAY_EFFECT),
            ),

            (State::Open, Event::PointerEnter | Event::ContentPointerEnter) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = true;
                }))
            }

            (State::Open, Event::Focus) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = true;
                }))
            }

            (State::Open, Event::PointerLeave | Event::ContentPointerLeave) if ctx.focus_active => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = false;
                }))
            }

            (State::Open, Event::PointerLeave | Event::ContentPointerLeave) => {
                Some(close_pending_or_closed(props, |ctx| {
                    ctx.hover_active = false;
                }))
            }

            (State::Open, Event::Blur) if ctx.hover_active => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = false;
                }))
            }

            (State::Open, Event::Blur) => Some(close_pending_or_closed(props, |ctx| {
                ctx.focus_active = false;
            })),

            (State::Open, Event::CloseOnEscape | Event::CloseOnClick | Event::CloseOnScroll)
                if should_close_visible(*event, props) =>
            {
                Some(close_now_plan(props))
            }

            (State::ClosePending, Event::CloseTimerFired) => Some(
                close_now_plan(props)
                    .apply(|ctx: &mut Context| {
                        ctx.hover_active = false;
                        ctx.focus_active = false;
                    })
                    .cancel_effect(CLOSE_DELAY_EFFECT),
            ),

            (State::ClosePending, Event::PointerEnter | Event::ContentPointerEnter) => Some(
                open_plan(props, |ctx| {
                    ctx.hover_active = true;
                })
                .cancel_effect(CLOSE_DELAY_EFFECT),
            ),

            (State::ClosePending, Event::Focus) => Some(
                open_plan(props, |ctx| {
                    ctx.focus_active = true;
                })
                .cancel_effect(CLOSE_DELAY_EFFECT),
            ),

            (State::ClosePending, Event::Blur) if ctx.hover_active => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = false;
                }))
            }

            (State::ClosePending, Event::PointerLeave | Event::ContentPointerLeave)
                if ctx.focus_active =>
            {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = false;
                }))
            }

            (
                State::ClosePending,
                Event::CloseOnEscape | Event::CloseOnClick | Event::CloseOnScroll,
            ) if should_close_visible(*event, props) => {
                Some(close_now_plan(props).cancel_effect(CLOSE_DELAY_EFFECT))
            }

            (State::Closed, Event::Open) => Some(open_plan(props, |_| {})),

            (State::OpenPending, Event::Open) => {
                Some(open_plan(props, |_| {}).cancel_effect(OPEN_DELAY_EFFECT))
            }

            (State::Open, Event::Close) => Some(close_now_plan(props)),

            (State::OpenPending, Event::Close) => {
                Some(close_now_plan(props).cancel_effect(OPEN_DELAY_EFFECT))
            }

            (State::ClosePending, Event::Close) => {
                Some(close_now_plan(props).cancel_effect(CLOSE_DELAY_EFFECT))
            }

            (_, Event::SetControlledOpen(open)) => Some(sync_controlled_plan(*open, props)),

            (_, Event::SyncProps) => Some(sync_props_plan(props)),

            (_, Event::SetZIndex(z_index)) => Some(TransitionPlan::context_only({
                let z_index = *z_index;
                move |ctx: &mut Context| {
                    ctx.z_index = Some(z_index);
                }
            })),

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
        let mut events = Vec::new();

        match (old.open, new.open) {
            (old_open, Some(new_open)) if old_open != Some(new_open) => {
                events.push(Event::SetControlledOpen(new_open));
            }

            _ if props_context_changed(old, new) => {
                events.push(Event::SyncProps);
            }

            _ => {}
        }

        events
    }
}

/// Structural parts exposed by the Tooltip connect API.
#[derive(ComponentPart)]
#[scope = "tooltip"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger element that owns the tooltip description.
    Trigger,

    /// The always-rendered visually-hidden description.
    HiddenDescription,

    /// The adapter-owned floating positioner element.
    Positioner,

    /// The visible tooltip content element.
    Content,

    /// The optional arrow element.
    Arrow,
}

/// Connected Tooltip API.
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

impl<'a> Api<'a> {
    /// Returns `true` when the tooltip is visibly open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.ctx.open
    }

    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Data("ars-disabled"), "true");
        }

        attrs
    }

    /// Returns attributes for the trigger element.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.ctx.trigger_id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                &self.ctx.hidden_description_id,
            );

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Dispatches a trigger pointer-enter event.
    pub fn on_trigger_pointer_enter(&self) {
        (self.send)(Event::PointerEnter);
    }

    /// Dispatches a trigger pointer-leave event.
    pub fn on_trigger_pointer_leave(&self) {
        (self.send)(Event::PointerLeave);
    }

    /// Dispatches a trigger focus event.
    pub fn on_trigger_focus(&self) {
        (self.send)(Event::Focus);
    }

    /// Dispatches a trigger blur event.
    pub fn on_trigger_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a trigger click dismissal request.
    pub fn on_trigger_click(&self) {
        if self.props.close_on_click {
            (self.send)(Event::CloseOnClick);
        }
    }

    /// Dispatches a scroll dismissal request.
    pub fn on_scroll(&self) {
        if self.props.close_on_scroll {
            (self.send)(Event::CloseOnScroll);
        }
    }

    /// Handles trigger keydown and returns whether the key was consumed.
    #[must_use]
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.key == KeyboardKey::Escape
            && !self.ctx.disabled
            && self.props.close_on_escape
            && matches!(
                self.state,
                State::OpenPending | State::Open | State::ClosePending
            )
        {
            (self.send)(Event::CloseOnEscape);
            true
        } else {
            false
        }
    }

    /// Dispatches a visible-content pointer-enter event.
    pub fn on_content_pointer_enter(&self) {
        (self.send)(Event::ContentPointerEnter);
    }

    /// Dispatches a visible-content pointer-leave event.
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::ContentPointerLeave);
    }

    /// Returns attributes for the always-rendered hidden description.
    #[must_use]
    pub fn hidden_description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenDescription.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.ctx.hidden_description_id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-visually-hidden"), "true");

        attrs
    }

    /// Returns attributes for the adapter-owned floating positioner.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.positioning.placement.as_str(),
            );

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Returns attributes for the visible tooltip content.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.ctx.content_id)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Returns attributes for the optional arrow element.
    #[must_use]
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.positioning.placement.as_str(),
            );

        attrs
    }

    const fn state_token(&self) -> &'static str {
        if self.is_open() { "open" } else { "closed" }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::HiddenDescription => self.hidden_description_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
        }
    }
}

const fn close_delay(props: &Props) -> Duration {
    props.close_delay
}

fn format_description_id(content_id: &str) -> String {
    let mut id = String::from(content_id);

    id.push_str("-description");

    id
}

const fn clear_activity(ctx: &mut Context) {
    ctx.hover_active = false;
    ctx.focus_active = false;
}

const fn should_close_pending(event: Event, props: &Props) -> bool {
    match event {
        Event::CloseOnEscape => props.close_on_escape,
        Event::CloseOnClick => props.close_on_click,
        Event::CloseOnScroll => props.close_on_scroll,
        _ => false,
    }
}

const fn should_close_visible(event: Event, props: &Props) -> bool {
    should_close_pending(event, props)
}

fn open_change_effect(open: bool) -> PendingEffect<Machine> {
    PendingEffect::new(
        OPEN_CHANGE_EFFECT,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(cb) = &props.on_open_change {
                cb(open);
            }

            no_cleanup()
        },
    )
}

fn open_plan(
    props: &Props,
    apply_activity: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    if props.open.is_some() {
        TransitionPlan::context_only(apply_activity).with_effect(open_change_effect(true))
    } else {
        TransitionPlan::to(State::Open)
            .apply(move |ctx: &mut Context| {
                ctx.open = true;
                apply_activity(ctx);
            })
            .with_effect(open_change_effect(true))
            .with_effect(PendingEffect::named(ALLOCATE_Z_INDEX_EFFECT))
    }
}

fn close_pending_or_closed(
    props: &Props,
    apply_activity: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    if close_delay(props).is_zero() && props.open.is_none() {
        close_now_plan(props).apply(apply_activity)
    } else if props.open.is_some() {
        TransitionPlan::context_only(apply_activity).with_effect(open_change_effect(false))
    } else {
        TransitionPlan::to(State::ClosePending)
            .apply(apply_activity)
            .with_effect(PendingEffect::named(CLOSE_DELAY_EFFECT))
    }
}

fn close_now_plan(props: &Props) -> TransitionPlan<Machine> {
    let plan = if props.open.is_some() {
        TransitionPlan::context_only(clear_activity)
    } else {
        TransitionPlan::to(State::Closed).apply(|ctx: &mut Context| {
            ctx.open = false;
            clear_activity(ctx);
        })
    };

    plan.with_effect(open_change_effect(false))
}

fn sync_controlled_plan(open: bool, props: &Props) -> TransitionPlan<Machine> {
    let state = if open { State::Open } else { State::Closed };

    let props = props.clone();

    let mut plan = TransitionPlan::to(state).apply(move |ctx: &mut Context| {
        ctx.open = open;

        sync_props_context(ctx, &props);

        if !open {
            clear_activity(ctx);
        }
    });

    if open {
        plan = plan.with_effect(PendingEffect::named(ALLOCATE_Z_INDEX_EFFECT));
    } else {
        plan = plan
            .cancel_effect(OPEN_DELAY_EFFECT)
            .cancel_effect(CLOSE_DELAY_EFFECT);
    }

    plan
}

fn sync_props_plan(props: &Props) -> TransitionPlan<Machine> {
    TransitionPlan::context_only({
        let props = props.clone();
        move |ctx: &mut Context| {
            sync_props_context(ctx, &props);
        }
    })
}

fn sync_props_context(ctx: &mut Context, props: &Props) {
    let ids = ComponentIds::from_id(&props.id);
    let content_id = ids.part("content");

    ctx.open_delay = props.open_delay;
    ctx.close_delay = close_delay(props);
    ctx.disabled = props.disabled;
    ctx.dir = props.dir;
    ctx.positioning = props.positioning.clone();
    ctx.trigger_id = ids.part("trigger");
    ctx.hidden_description_id = format_description_id(&content_id);
    ctx.content_id = content_id;
    ctx.touch_auto_hide = props.touch_auto_hide.max(MIN_TOUCH_AUTO_HIDE);
}

fn props_context_changed(old: &Props, new: &Props) -> bool {
    old.id != new.id
        || old.open_delay != new.open_delay
        || old.close_delay != new.close_delay
        || old.disabled != new.disabled
        || old.dir != new.dir
        || old.positioning != new.positioning
        || old.touch_auto_hide != new.touch_auto_hide
}

const fn disabled_blocks_event(event: Event) -> bool {
    matches!(
        event,
        Event::PointerEnter
            | Event::PointerLeave
            | Event::Focus
            | Event::Blur
            | Event::ContentPointerEnter
            | Event::ContentPointerLeave
            | Event::OpenTimerFired
            | Event::Open
            | Event::CloseOnEscape
            | Event::CloseOnClick
            | Event::CloseOnScroll
    )
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, string::ToString, sync::Arc, vec::Vec};
    use core::{cell::RefCell, time::Duration};
    use std::sync::Mutex;

    use ars_core::{ConnectApi, Service, callback};
    use insta::assert_snapshot;

    use super::*;
    use crate::overlay::positioning::Placement;

    fn test_props() -> Props {
        Props {
            id: "tooltip".to_string(),
            ..Props::default()
        }
    }

    fn keyboard_data(key: KeyboardKey) -> KeyboardEventData {
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

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<&'static str> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn snapshot_api(api: &Api<'_>) -> String {
        format!(
            "root:\n{:#?}\ntrigger:\n{:#?}\nhidden_description:\n{:#?}\npositioner:\n{:#?}\ncontent:\n{:#?}\narrow:\n{:#?}",
            api.root_attrs(),
            api.trigger_attrs(),
            api.hidden_description_attrs(),
            api.positioner_attrs(),
            api.content_attrs(),
            api.arrow_attrs(),
        )
    }

    #[test]
    fn tooltip_props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn tooltip_props_builder_chain_applies_each_setter() {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_for_props = Arc::clone(&observed);
        let positioning = PositioningOptions {
            placement: Placement::RightStart,
            ..PositioningOptions::default()
        };

        let props = Props::new()
            .id("tooltip-builder")
            .open(true)
            .default_open(true)
            .open_delay(Duration::from_millis(25))
            .close_delay(Duration::from_millis(75))
            .disabled(true)
            .positioning(positioning.clone())
            .close_on_escape(false)
            .close_on_click(false)
            .close_on_scroll(false)
            .on_open_change(move |open| {
                observed_for_props
                    .lock()
                    .expect("observed callback state should not be poisoned")
                    .push(open);
            })
            .lazy_mount(true)
            .unmount_on_exit(true)
            .dir(Direction::Rtl)
            .touch_auto_hide(Duration::from_secs(30));

        assert_eq!(props.id, "tooltip-builder");
        assert_eq!(props.open, Some(true));
        assert!(props.default_open);
        assert_eq!(props.open_delay, Duration::from_millis(25));
        assert_eq!(props.close_delay, Duration::from_millis(75));
        assert!(props.disabled);
        assert_eq!(props.positioning, positioning);
        assert!(!props.close_on_escape);
        assert!(!props.close_on_click);
        assert!(!props.close_on_scroll);
        assert!(props.on_open_change.is_some());
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert_eq!(props.dir, Direction::Rtl);
        assert_eq!(props.touch_auto_hide, Duration::from_secs(30));

        props
            .on_open_change
            .as_ref()
            .expect("builder should register callback")(false);

        assert_eq!(
            &*observed
                .lock()
                .expect("observed callback state should not be poisoned"),
            &[false]
        );
    }

    #[test]
    fn tooltip_hover_enters_open_pending_with_open_delay_effect() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::PointerEnter);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().hover_active);
        assert!(!service.context().open);
        assert_eq!(effect_names(&result), vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pending_pointer_enter_keeps_pending() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        service.context_mut().hover_active = false;

        let result = service.send(Event::PointerEnter);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().hover_active);
        assert!(!result.state_changed);
    }

    #[test]
    fn tooltip_open_pending_focus_opens_immediately() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        let result = service.send(Event::Focus);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().focus_active);
        assert_eq!(
            effect_names(&result),
            vec![OPEN_CHANGE_EFFECT, ALLOCATE_Z_INDEX_EFFECT]
        );
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_timer_opens_and_allocates_z_index() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        let result = service.send(Event::OpenTimerFired);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(
            effect_names(&result),
            vec![OPEN_CHANGE_EFFECT, ALLOCATE_Z_INDEX_EFFECT]
        );
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pending_pointer_leave_with_focus_stays_pending() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        service.context_mut().focus_active = true;

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(!service.context().hover_active);
        assert!(service.context().focus_active);
        assert!(result.cancel_effects.is_empty());
    }

    #[test]
    fn tooltip_open_pending_pointer_leave_without_focus_closes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().hover_active);
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pending_blur_with_hover_stays_pending() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        service.context_mut().focus_active = true;

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().hover_active);
        assert!(!service.context().focus_active);
        assert!(result.cancel_effects.is_empty());
    }

    #[test]
    fn tooltip_open_pending_blur_without_hover_closes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        service.context_mut().hover_active = false;
        service.context_mut().focus_active = true;

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().focus_active);
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pending_dismiss_cancels_open_delay() {
        for event in [
            Event::CloseOnEscape,
            Event::CloseOnClick,
            Event::CloseOnScroll,
        ] {
            let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

            drop(service.send(Event::PointerEnter));

            let result = service.send(event);

            assert_eq!(service.state(), &State::Closed);
            assert!(!service.context().hover_active);
            assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
        }
    }

    #[test]
    fn tooltip_open_pending_programmatic_open_cancels_open_delay() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        let result = service.send(Event::Open);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pending_programmatic_close_cancels_open_delay() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::PointerEnter));

        let result = service.send(Event::Close);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(result.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_pointer_leave_enters_close_pending_with_close_delay_effect() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::ClosePending);
        assert!(service.context().open);
        assert_eq!(effect_names(&result), vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_open_pointer_enter_and_content_enter_keep_hover_active() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        service.context_mut().hover_active = false;

        drop(service.send(Event::PointerEnter));

        assert!(service.context().hover_active);

        service.context_mut().hover_active = false;

        drop(service.send(Event::ContentPointerEnter));

        assert!(service.context().hover_active);
    }

    #[test]
    fn tooltip_open_pointer_leave_with_focus_stays_open() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Focus));

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::Open);
        assert!(!service.context().hover_active);
        assert!(service.context().focus_active);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_open_blur_with_hover_stays_open() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerEnter));

        service.context_mut().focus_active = true;

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().hover_active);
        assert!(!service.context().focus_active);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_zero_close_delay_closes_immediately() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                close_delay: Duration::ZERO,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_close_timer_closes() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::CloseTimerFired);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_close_pending_focus_cancels_pending_close() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::Focus);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().focus_active);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_close_pending_blur_with_hover_stays_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        service.context_mut().hover_active = true;
        service.context_mut().focus_active = true;

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::ClosePending);
        assert!(service.context().hover_active);
        assert!(!service.context().focus_active);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_close_pending_pointer_leave_with_focus_stays_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        service.context_mut().hover_active = true;
        service.context_mut().focus_active = true;

        let result = service.send(Event::ContentPointerLeave);

        assert_eq!(service.state(), &State::ClosePending);
        assert!(!service.context().hover_active);
        assert!(service.context().focus_active);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_close_pending_trigger_leave_with_focus_stays_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        service.context_mut().hover_active = true;
        service.context_mut().focus_active = true;

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::ClosePending);
        assert!(!service.context().hover_active);
        assert!(service.context().focus_active);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_close_pending_dismiss_closes_and_cancels_delay() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::CloseOnEscape);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_close_pending_programmatic_close_cancels_delay() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::Close);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_closed_blur_clears_stale_controlled_focus_activity() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Focus));

        assert_eq!(service.state(), &State::Closed);
        assert!(service.context().focus_active);

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(!service.context().focus_active);
        assert!(effect_names(&result).is_empty());
        assert!(result.cancel_effects.is_empty());

        drop(service.send(Event::PointerEnter));

        let leave = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().hover_active);
        assert_eq!(leave.cancel_effects, vec![OPEN_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_focus_opens_immediately_without_open_delay() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::Focus);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(service.context().focus_active);
        assert_eq!(
            effect_names(&result),
            vec![OPEN_CHANGE_EFFECT, ALLOCATE_Z_INDEX_EFFECT]
        );
    }

    #[test]
    fn tooltip_blur_closes_through_close_delay() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Focus));

        let result = service.send(Event::Blur);

        assert_eq!(service.state(), &State::ClosePending);
        assert_eq!(effect_names(&result), vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_content_pointer_enter_cancels_pending_close() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::ContentPointerEnter);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().hover_active);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_escape_dismisses_when_enabled() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::CloseOnEscape);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_escape_ignored_when_disabled_by_prop() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                close_on_escape: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::CloseOnEscape);

        assert_eq!(service.state(), &State::Open);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_click_dismiss_respects_prop() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                close_on_click: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::CloseOnClick));

        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn tooltip_scroll_dismiss_respects_prop() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                close_on_scroll: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::CloseOnScroll));

        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn tooltip_disabled_ignores_user_interaction() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::PointerEnter);

        assert_eq!(service.state(), &State::Closed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_disabled_allows_pending_close_timer_to_finish() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));
        drop(service.set_props(Props {
            disabled: true,
            ..test_props()
        }));

        let result = service.send(Event::CloseTimerFired);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(result.cancel_effects, vec![CLOSE_DELAY_EFFECT]);
    }

    #[test]
    fn tooltip_disabled_allows_programmatic_close() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Close);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_disabled_rejects_programmatic_open_event() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Open);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_close_delay_does_not_clamp_for_hoverable_content() {
        let service = Service::<Machine>::new(
            Props {
                close_delay: Duration::from_millis(1),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.context().close_delay, Duration::from_millis(1));
    }

    #[test]
    fn tooltip_touch_auto_hide_clamps_to_minimum() {
        let service = Service::<Machine>::new(
            Props {
                touch_auto_hide: Duration::from_millis(1),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.context().touch_auto_hide, MIN_TOUCH_AUTO_HIDE);
    }

    #[test]
    fn tooltip_programmatic_open_and_close_skip_delays() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let open = service.send(Event::Open);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(
            effect_names(&open),
            vec![OPEN_CHANGE_EFFECT, ALLOCATE_Z_INDEX_EFFECT]
        );

        let close = service.send(Event::Close);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&close), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_controlled_open_request_waits_for_prop_sync() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Focus);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(service.context().focus_active);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_controlled_close_request_waits_for_prop_sync() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerEnter));
        drop(service.send(Event::Focus));

        let result = service.send(Event::Close);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(!service.context().hover_active);
        assert!(!service.context().focus_active);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_controlled_pointer_leave_requests_close_without_state_change() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::PointerLeave);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(!service.context().hover_active);
        assert_eq!(effect_names(&result), vec![OPEN_CHANGE_EFFECT]);
    }

    #[test]
    fn tooltip_controlled_prop_sync_updates_state_and_context() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.set_props(Props {
            open: Some(true),
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(effect_names(&result), vec![ALLOCATE_Z_INDEX_EFFECT]);
    }

    #[test]
    fn tooltip_controlled_prop_sync_false_closes_and_cancels_timers() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        let result = service.set_props(Props {
            open: Some(false),
            ..test_props()
        });

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(
            result.cancel_effects,
            vec![OPEN_DELAY_EFFECT, CLOSE_DELAY_EFFECT]
        );
    }

    #[test]
    fn tooltip_disabled_allows_prop_sync_and_z_index_feedback() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.set_props(Props {
            disabled: true,
            open: Some(true),
            ..test_props()
        }));
        drop(service.send(Event::SetZIndex(42)));

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(service.context().z_index, Some(42));
    }

    #[test]
    fn tooltip_props_changed_syncs_context_without_open_change() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(Props {
            id: "renamed-tooltip".to_string(),
            open_delay: Duration::from_millis(12),
            close_delay: Duration::ZERO,
            disabled: true,
            dir: Direction::Rtl,
            positioning: PositioningOptions {
                placement: Placement::LeftEnd,
                ..PositioningOptions::default()
            },
            touch_auto_hide: Duration::from_millis(1),
            ..test_props()
        });

        let ids = ComponentIds::from_id("renamed-tooltip");
        let content_id = ids.part("content");

        assert_eq!(service.context().open_delay, Duration::from_millis(12));
        assert_eq!(service.context().close_delay, Duration::ZERO);
        assert!(service.context().disabled);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().positioning.placement, Placement::LeftEnd);
        assert_eq!(service.context().trigger_id, ids.part("trigger"));
        assert_eq!(service.context().content_id, content_id);
        assert_eq!(
            service.context().hidden_description_id,
            format_description_id(&service.context().content_id)
        );
        assert_eq!(service.context().touch_auto_hide, MIN_TOUCH_AUTO_HIDE);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn tooltip_id_prop_change_updates_connected_aria_ids() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props {
            id: "renamed-tooltip".to_string(),
            ..test_props()
        }));

        let api = service.connect(&|_| {});
        let ids = ComponentIds::from_id("renamed-tooltip");
        let content_id = ids.part("content");
        let hidden_id = format_description_id(&content_id);

        assert_eq!(
            api.trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some(hidden_id.as_str())
        );
        assert_eq!(
            api.hidden_description_attrs().get(&HtmlAttr::Id),
            Some(hidden_id.as_str())
        );
        assert_eq!(
            api.content_attrs().get(&HtmlAttr::Id),
            Some(content_id.as_str())
        );
    }

    #[test]
    fn tooltip_on_props_changed_reports_each_context_backed_prop() {
        let old = test_props();
        let cases = [
            Props {
                id: "renamed-tooltip".to_string(),
                ..test_props()
            },
            Props {
                open_delay: Duration::from_millis(12),
                ..test_props()
            },
            Props {
                close_delay: Duration::ZERO,
                ..test_props()
            },
            Props {
                disabled: true,
                ..test_props()
            },
            Props {
                dir: Direction::Rtl,
                ..test_props()
            },
            Props {
                positioning: PositioningOptions {
                    placement: Placement::EndBottom,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            Props {
                touch_auto_hide: Duration::from_millis(5_001),
                ..test_props()
            },
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                vec![Event::SyncProps]
            );
        }

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &old).is_empty());
    }

    #[test]
    fn tooltip_set_z_index_updates_positioner_style() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetZIndex(1200)));

        let attrs = service.connect(&|_| {}).positioner_attrs();

        assert_eq!(
            attrs.styles(),
            &[(CssProperty::Custom("ars-z-index"), "1200".to_string())]
        );
    }

    #[test]
    fn tooltip_trigger_describes_hidden_description() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("tooltip-content-description")
        );
        assert_eq!(
            api.hidden_description_attrs().get(&HtmlAttr::Id),
            Some("tooltip-content-description")
        );
    }

    #[test]
    fn tooltip_content_has_accessibility_and_state_attrs() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                dir: Direction::Rtl,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("tooltip-content"));
        assert_eq!(attrs.get(&HtmlAttr::Role), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("open"));
    }

    #[test]
    fn tooltip_positioner_has_placement_attr() {
        let service = Service::<Machine>::new(
            Props {
                positioning: PositioningOptions {
                    placement: Placement::TopStart,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).positioner_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-placement")),
            Some("top-start")
        );
    }

    #[test]
    fn tooltip_part_attrs_match_direct_methods() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(
            api.part_attrs(Part::HiddenDescription),
            api.hidden_description_attrs()
        );
        assert_eq!(api.part_attrs(Part::Positioner), api.positioner_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::Arrow), api.arrow_attrs());
    }

    #[test]
    fn tooltip_api_debug_includes_state_context_and_props() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("Api"));
        assert!(debug.contains("state"));
        assert!(debug.contains("ctx"));
        assert!(debug.contains("props"));
    }

    #[test]
    fn tooltip_api_event_helpers_dispatch_expected_events() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);
        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);

        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_trigger_pointer_enter();
        api.on_trigger_pointer_leave();
        api.on_trigger_focus();
        api.on_trigger_blur();
        api.on_trigger_click();
        api.on_scroll();
        api.on_content_pointer_enter();
        api.on_content_pointer_leave();

        assert_eq!(
            &*sent.borrow(),
            &[
                Event::PointerEnter,
                Event::PointerLeave,
                Event::Focus,
                Event::Blur,
                Event::CloseOnClick,
                Event::CloseOnScroll,
                Event::ContentPointerEnter,
                Event::ContentPointerLeave,
            ]
        );
    }

    #[test]
    fn tooltip_api_dismiss_helpers_respect_disabled_props() {
        let service = Service::<Machine>::new(
            Props {
                close_on_click: false,
                close_on_scroll: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);
        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_trigger_click();
        api.on_scroll();

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn tooltip_keydown_consumes_only_handled_escape() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);
        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        assert!(api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert!(!api.on_trigger_keydown(&keyboard_data(KeyboardKey::Enter)));
        assert_eq!(&*sent.borrow(), &[Event::CloseOnEscape]);
    }

    #[test]
    fn tooltip_keydown_ignores_escape_when_prop_false() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                close_on_escape: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let api = service.connect(&|_| {});

        assert!(!api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
    }

    #[test]
    fn tooltip_keydown_ignores_escape_when_disabled() {
        let service = Service::<Machine>::new(
            Props {
                open: Some(true),
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let sent = Rc::new(RefCell::new(Vec::new()));
        let sent_clone = Rc::clone(&sent);
        let send = move |event| sent_clone.borrow_mut().push(event);

        let api = service.connect(&send);

        assert!(!api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn tooltip_keydown_ignores_escape_when_closed() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert!(!api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
    }

    #[test]
    fn tooltip_close_guard_rejects_non_close_events() {
        assert!(!should_close_pending(Event::Open, &test_props()));
    }

    #[test]
    fn tooltip_open_change_effect_invokes_callback_when_run() {
        let observed = Arc::new(Mutex::new(Vec::new()));
        let observed_clone = Arc::clone(&observed);
        let mut service = Service::<Machine>::new(
            Props {
                on_open_change: Some(callback(move |open| {
                    observed_clone
                        .lock()
                        .expect("observed callback state should not be poisoned")
                        .push(open);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let mut result = service.send(Event::Focus);

        let effect = result.pending_effects.remove(0);
        let send: ars_core::StrongSend<Event> = Arc::new(|_| {});

        let cleanup = effect.run(service.context(), service.props(), send);

        cleanup();

        assert_eq!(
            &*observed
                .lock()
                .expect("observed callback state should not be poisoned"),
            &[true]
        );
    }

    #[test]
    fn snapshot_tooltip_all_parts_closed() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "tooltip_all_parts_closed",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_tooltip_all_parts_open() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "tooltip_all_parts_open",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_tooltip_open_rtl() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                dir: Direction::Rtl,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!("tooltip_open_rtl", snapshot_api(&service.connect(&|_| {})));
    }

    #[test]
    fn snapshot_tooltip_open_with_z_index() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetZIndex(1300)));

        assert_snapshot!(
            "tooltip_open_with_z_index",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_tooltip_close_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::PointerLeave));

        assert_snapshot!(
            "tooltip_close_pending",
            snapshot_api(&service.connect(&|_| {}))
        );
    }

    #[test]
    fn snapshot_tooltip_disabled() {
        let service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!("tooltip_disabled", snapshot_api(&service.connect(&|_| {})));
    }
}
