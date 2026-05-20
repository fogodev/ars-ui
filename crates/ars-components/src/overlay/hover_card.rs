//! Hover-triggered interactive overlay machine.
//!
//! The agnostic HoverCard core owns hover/focus timing state, semantic IDs,
//! ARIA/data attributes, placement intent, and z-index intent. Safe-area
//! geometry, live element refs, listener wiring, and DOM measurement are
//! adapter responsibilities.

use alloc::string::{String, ToString as _};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::positioning::{Placement, PositioningOptions, PositioningSnapshot};

/// Typed identifier for every named effect intent emitted by `HoverCard`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts the hover open-delay timer.
    OpenDelay,

    /// Adapter starts the close-delay timer.
    CloseDelay,

    /// Adapter invokes [`Props::on_open_change`] with the latest open state.
    OpenChange,

    /// Adapter allocates a z-index and dispatches [`Event::SetZIndex`].
    AllocateZIndex,

    /// Adapter releases the previously allocated z-index claim.
    ReleaseZIndex,
}

/// The states of the hover card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The hover card is closed.
    #[default]
    Closed,

    /// The hover card is pending open.
    OpenPending,

    /// The hover card is open.
    Open,

    /// The hover card is pending close.
    ClosePending,
}

/// Events accepted by the `HoverCard` state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The pointer entered the trigger.
    TriggerPointerEnter,

    /// The pointer left the trigger.
    TriggerPointerLeave,

    /// The trigger gained keyboard focus.
    TriggerFocus,

    /// The trigger lost keyboard focus.
    TriggerBlur,

    /// A key was pressed on the trigger.
    TriggerKeyDown(KeyboardKey),

    /// The pointer entered the interactive content.
    ContentPointerEnter,

    /// The pointer left the interactive content.
    ContentPointerLeave,

    /// Focus entered the interactive content.
    ContentFocus,

    /// Focus left the interactive content.
    ContentBlur,

    /// The open timer fired.
    OpenTimerFired,

    /// The close timer fired.
    CloseTimerFired,

    /// Escape requested dismissal.
    CloseOnEscape,

    /// The title element mounted.
    TitleMount,

    /// The title element unmounted.
    TitleUnmount,

    /// Adapter reported measured placement state.
    PositioningUpdate(PositioningSnapshot),

    /// Adapter supplied an allocated overlay z-index.
    SetZIndex(u32),

    /// Programmatic open requested immediate visibility.
    Open,

    /// Programmatic close requested immediate dismissal.
    Close,

    /// Controlled props synchronized the visible open state.
    SetControlledOpen(bool),

    /// Props changed without changing controlled visible state.
    SyncProps,
}

/// Localizable strings for `HoverCard`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for content when no title is rendered.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the dismiss button.
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Hover card"),
            dismiss_label: MessageFn::static_str("Dismiss hover card"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Runtime context for `HoverCard`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,

    /// Whether the hover card is visibly open.
    pub open: bool,

    /// Delay before pointer hover opens the card.
    pub open_delay: Duration,

    /// Delay before hover/focus leave closes the card.
    pub close_delay: Duration,

    /// Whether the hover card ignores user interaction.
    pub disabled: bool,

    /// Positioning options forwarded to framework adapters.
    pub positioning: PositioningOptions,

    /// Current placement, initialized from props and updated by adapters.
    pub current_placement: Placement,

    /// Derived component part IDs.
    pub ids: ComponentIds,

    /// Hydration-stable trigger ID used for ARIA relationships.
    pub trigger_id: String,

    /// Hydration-stable content ID used for ARIA relationships.
    pub content_id: String,

    /// Hydration-stable title ID used when the title is rendered.
    pub title_id: String,

    /// Whether a title element has mounted.
    pub has_title: bool,

    /// Whether the pointer is over the trigger or content.
    pub hover_active: bool,

    /// Whether the trigger currently has keyboard focus.
    pub focus_active: bool,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,

    /// Adapter-allocated z-index for the positioner.
    pub z_index: Option<u32>,
}

/// Immutable configuration for a `HoverCard` instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled open state. When `Some`, the consumer owns open state.
    pub open: Option<bool>,

    /// Initial uncontrolled open state.
    pub default_open: bool,

    /// Delay before pointer hover opens the card.
    pub open_delay: Duration,

    /// Delay before hover/focus leave closes the card.
    pub close_delay: Duration,

    /// Whether the hover card ignores user interaction.
    pub disabled: bool,

    /// Positioning options forwarded to framework adapters.
    pub positioning: PositioningOptions,

    /// Callback invoked after open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Whether content is not mounted until first opened.
    pub lazy_mount: bool,

    /// Whether content is removed from the DOM after closing.
    pub unmount_on_exit: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            open_delay: Duration::from_millis(700),
            close_delay: Duration::from_millis(300),
            disabled: false,
            positioning: PositioningOptions::default(),
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
        }
    }
}

impl Props {
    /// Returns `HoverCard` props with documented defaults.
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

    /// Sets the controlled open state.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Sets the uncontrolled initial open state.
    #[must_use]
    pub const fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }
}

/// Anatomy parts exposed by the `HoverCard` connect API.
#[derive(ComponentPart)]
#[scope = "hover-card"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger that opens the hover card.
    Trigger,

    /// The adapter-owned floating positioner element.
    Positioner,

    /// The interactive hover card content surface.
    Content,

    /// The optional arrow element.
    Arrow,

    /// The optional title element used for `aria-labelledby`.
    Title,

    /// The optional dismiss button rendered inside content.
    DismissButton,
}

const fn state_token(open: bool) -> &'static str {
    if open { "open" } else { "closed" }
}

fn open_plan(cancel: Option<Effect>) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open)
        .apply(|ctx: &mut Context| {
            ctx.open = true;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::AllocateZIndex));

    if let Some(effect) = cancel {
        plan = plan.cancel_effect(effect);
    }

    plan
}

fn controlled_open_request_plan(cancel: Option<Effect>) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.hover_active = false;
            ctx.focus_active = false;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange));

    if let Some(effect) = cancel {
        plan = plan.cancel_effect(effect);
    }

    plan
}

fn close_plan(cancel: Option<Effect>) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.hover_active = false;
            ctx.focus_active = false;
            ctx.z_index = None;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::ReleaseZIndex));

    if let Some(effect) = cancel {
        plan = plan.cancel_effect(effect);
    }

    plan
}

fn controlled_close_request_plan(cancel: Option<Effect>) -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open)
        .apply(|ctx: &mut Context| {
            ctx.hover_active = false;
            ctx.focus_active = false;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange));

    if let Some(effect) = cancel {
        plan = plan.cancel_effect(effect);
    }

    plan
}

fn cancel_pending_open_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.hover_active = false;
            ctx.focus_active = false;
        })
        .cancel_effect(Effect::OpenDelay)
}

fn start_close_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::ClosePending).with_effect(PendingEffect::named(Effect::CloseDelay))
}

fn props_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.open_delay != new.open_delay
        || old.close_delay != new.close_delay
        || old.positioning != new.positioning
}

/// State machine for `HoverCard`.
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
        let ids = ComponentIds::from_id(&props.id);

        let initial_open = props.open.unwrap_or(props.default_open);

        let state = if initial_open {
            State::Open
        } else {
            State::Closed
        };

        let current_placement = props.positioning.placement;

        (
            state,
            Context {
                locale: env.locale.clone(),
                open: initial_open,
                open_delay: props.open_delay,
                close_delay: props.close_delay,
                disabled: props.disabled,
                positioning: props.positioning.clone(),
                current_placement,
                trigger_id: ids.part("trigger"),
                content_id: ids.part("content"),
                title_id: ids.part("title"),
                ids,
                has_title: false,
                hover_active: false,
                focus_active: false,
                messages: messages.clone(),
                z_index: None,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (_, Event::SetControlledOpen(open)) => {
                if *open {
                    match state {
                        State::Open => None,
                        State::OpenPending => Some(open_plan(Some(Effect::OpenDelay))),
                        State::ClosePending => Some(open_plan(Some(Effect::CloseDelay))),
                        State::Closed => Some(open_plan(None)),
                    }
                } else {
                    match state {
                        State::Closed => None,
                        State::OpenPending => Some(cancel_pending_open_plan()),
                        State::Open => Some(close_plan(None)),
                        State::ClosePending => Some(close_plan(Some(Effect::CloseDelay))),
                    }
                }
            }

            (_, Event::SyncProps) => Some(TransitionPlan::context_only({
                let disabled = props.disabled;
                let open_delay = props.open_delay;
                let close_delay = props.close_delay;
                let positioning = props.positioning.clone();
                move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                    ctx.open_delay = open_delay;
                    ctx.close_delay = close_delay;
                    if ctx.positioning != positioning {
                        ctx.current_placement = positioning.placement;
                    }
                    ctx.positioning = positioning;
                }
            })),

            (_, Event::SetZIndex(z_index)) => {
                let z_index = *z_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.z_index = Some(z_index);
                }))
            }

            (_, Event::PositioningUpdate(snapshot)) => {
                let placement = snapshot.placement;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.current_placement = placement;
                }))
            }

            (_, Event::TitleMount) if !ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = true;
                }))
            }

            (_, Event::TitleUnmount) if ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = false;
                }))
            }

            (
                _,
                Event::TriggerPointerEnter
                | Event::TriggerFocus
                | Event::TriggerKeyDown(_)
                | Event::ContentPointerEnter
                | Event::ContentFocus
                | Event::OpenTimerFired
                | Event::Open,
            ) if props.disabled => None,

            (State::Closed, Event::TriggerPointerEnter) => Some(
                TransitionPlan::to(State::OpenPending)
                    .apply(|ctx: &mut Context| {
                        ctx.hover_active = true;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenDelay)),
            ),

            (State::Closed, Event::TriggerFocus) => Some(
                TransitionPlan::to(State::OpenPending)
                    .apply(|ctx: &mut Context| {
                        ctx.focus_active = true;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenDelay)),
            ),

            (
                State::Closed | State::OpenPending,
                Event::TriggerKeyDown(KeyboardKey::Enter | KeyboardKey::Space),
            ) => {
                let cancel = matches!(state, State::OpenPending).then_some(Effect::OpenDelay);
                if props.open == Some(false) {
                    return Some(controlled_open_request_plan(cancel));
                }
                Some(open_plan(cancel))
            }

            (State::OpenPending, Event::OpenTimerFired) => {
                if props.open == Some(false) {
                    return Some(controlled_open_request_plan(Some(Effect::OpenDelay)));
                }
                Some(open_plan(Some(Effect::OpenDelay)))
            }

            (State::OpenPending, Event::TriggerFocus) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = true;
                }))
            }

            (State::OpenPending, Event::TriggerPointerLeave) => {
                if ctx.focus_active {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.hover_active = false;
                    }))
                } else {
                    Some(
                        TransitionPlan::to(State::Closed)
                            .apply(|ctx: &mut Context| {
                                ctx.hover_active = false;
                            })
                            .cancel_effect(Effect::OpenDelay),
                    )
                }
            }

            (State::OpenPending, Event::TriggerBlur) => {
                if ctx.hover_active {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    }))
                } else {
                    Some(
                        TransitionPlan::to(State::Closed)
                            .apply(|ctx: &mut Context| {
                                ctx.focus_active = false;
                            })
                            .cancel_effect(Effect::OpenDelay),
                    )
                }
            }

            (State::Open, Event::TriggerPointerEnter | Event::ContentPointerEnter) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hover_active = true;
                }))
            }

            (State::Open, Event::TriggerFocus) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = true;
                }))
            }

            (State::Open, Event::ContentFocus) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focus_active = true;
                }))
            }

            (State::Open, Event::TriggerPointerLeave | Event::ContentPointerLeave) => {
                if ctx.focus_active {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.hover_active = false;
                    }))
                } else {
                    Some(start_close_plan().apply(|ctx: &mut Context| {
                        ctx.hover_active = false;
                    }))
                }
            }

            (State::Open, Event::TriggerBlur) => {
                if ctx.hover_active {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    }))
                } else {
                    Some(start_close_plan().apply(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    }))
                }
            }

            (State::Open, Event::ContentBlur) => {
                if ctx.hover_active {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    }))
                } else {
                    Some(start_close_plan().apply(|ctx: &mut Context| {
                        ctx.focus_active = false;
                    }))
                }
            }

            (State::ClosePending, Event::ContentPointerEnter | Event::TriggerPointerEnter) => Some(
                TransitionPlan::to(State::Open)
                    .apply(|ctx: &mut Context| {
                        ctx.hover_active = true;
                    })
                    .cancel_effect(Effect::CloseDelay),
            ),

            (State::ClosePending, Event::TriggerFocus | Event::ContentFocus) => Some(
                TransitionPlan::to(State::Open)
                    .apply(|ctx: &mut Context| {
                        ctx.focus_active = true;
                    })
                    .cancel_effect(Effect::CloseDelay),
            ),

            (State::ClosePending, Event::CloseTimerFired) => {
                if props.open == Some(true) {
                    return Some(controlled_close_request_plan(Some(Effect::CloseDelay)));
                }
                Some(close_plan(Some(Effect::CloseDelay)))
            }

            (
                State::Closed | State::OpenPending | State::Open | State::ClosePending,
                Event::Open,
            ) => {
                if matches!(state, State::Open) && ctx.open {
                    return None;
                }

                let cancel = match state {
                    State::OpenPending => Some(Effect::OpenDelay),
                    State::ClosePending => Some(Effect::CloseDelay),
                    State::Closed | State::Open => None,
                };

                if props.open == Some(false) {
                    return Some(controlled_open_request_plan(cancel));
                }

                Some(open_plan(cancel))
            }

            (State::OpenPending | State::Open | State::ClosePending, Event::Close) => {
                let cancel = match state {
                    State::OpenPending => Some(Effect::OpenDelay),
                    State::ClosePending => Some(Effect::CloseDelay),
                    State::Open | State::Closed => None,
                };

                if props.open == Some(true) {
                    return Some(controlled_close_request_plan(cancel));
                }

                Some(close_plan(cancel))
            }

            (State::OpenPending, Event::CloseOnEscape) => {
                if props.open == Some(true) {
                    Some(controlled_close_request_plan(Some(Effect::OpenDelay)))
                } else {
                    Some(cancel_pending_open_plan())
                }
            }

            (State::Open | State::ClosePending, Event::CloseOnEscape) => {
                let cancel = match state {
                    State::ClosePending => Some(Effect::CloseDelay),
                    State::Closed | State::OpenPending | State::Open => None,
                };

                if props.open == Some(true) {
                    return Some(controlled_close_request_plan(cancel));
                }

                Some(close_plan(cancel))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx: context,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.id != new.id {
            panic!("HoverCard id cannot change after initialization");
        }

        let mut events = Vec::new();

        if old.open != new.open
            && let Some(open) = new.open
        {
            events.push(Event::SetControlledOpen(open));
        }

        if props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        if matches!(state, State::Open) {
            vec![
                PendingEffect::named(Effect::OpenChange),
                PendingEffect::named(Effect::AllocateZIndex),
            ]
        } else {
            Vec::new()
        }
    }
}

/// Connected `HoverCard` API.
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
    /// Returns whether the hover card is visibly open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.ctx.open
    }

    /// Returns the value of [`Props::lazy_mount`].
    #[must_use]
    pub const fn lazy_mount(&self) -> bool {
        self.props.lazy_mount
    }

    /// Returns the value of [`Props::unmount_on_exit`].
    #[must_use]
    pub const fn unmount_on_exit(&self) -> bool {
        self.props.unmount_on_exit
    }

    /// Returns the current resolved placement.
    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.ctx.current_placement
    }

    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()));

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
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.trigger_id.clone())
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            );

        if self.is_open() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.content_id.clone(),
            );
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Dispatches trigger pointer-enter.
    pub fn on_trigger_pointer_enter(&self) {
        (self.send)(Event::TriggerPointerEnter);
    }

    /// Dispatches trigger pointer-leave.
    pub fn on_trigger_pointer_leave(&self) {
        (self.send)(Event::TriggerPointerLeave);
    }

    /// Dispatches trigger focus.
    pub fn on_trigger_focus(&self) {
        (self.send)(Event::TriggerFocus);
    }

    /// Dispatches trigger blur.
    pub fn on_trigger_blur(&self) {
        (self.send)(Event::TriggerBlur);
    }

    /// Handles trigger keydown and returns whether the key was consumed.
    #[must_use]
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space
                if !self.ctx.disabled
                    && matches!(self.state, State::Closed | State::OpenPending) =>
            {
                (self.send)(Event::TriggerKeyDown(data.key));
                true
            }

            KeyboardKey::Escape
                if matches!(
                    self.state,
                    State::OpenPending | State::Open | State::ClosePending
                ) =>
            {
                (self.send)(Event::CloseOnEscape);
                true
            }

            _ => false,
        }
    }

    /// Dispatches content pointer-enter.
    pub fn on_content_pointer_enter(&self) {
        (self.send)(Event::ContentPointerEnter);
    }

    /// Dispatches content pointer-leave.
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::ContentPointerLeave);
    }

    /// Dispatches content focus.
    pub fn on_content_focus(&self) {
        (self.send)(Event::ContentFocus);
    }

    /// Dispatches content blur.
    pub fn on_content_blur(&self) {
        (self.send)(Event::ContentBlur);
    }

    /// Returns attributes for the adapter-owned positioner.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()))
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.current_placement.as_str(),
            );

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Returns attributes for the interactive content surface.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.content_id.clone())
            .set(HtmlAttr::Role, "dialog")
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()))
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.has_title {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.title_id.clone(),
            );
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            );
        }

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
                self.ctx.current_placement.as_str(),
            );

        attrs
    }

    /// Returns attributes for the optional title element.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.title_id.clone());

        attrs
    }

    /// Dispatches the title-mounted registration event.
    pub fn on_title_mount(&self) {
        (self.send)(Event::TitleMount);
    }

    /// Dispatches the title-unmounted registration event.
    pub fn on_title_unmount(&self) {
        (self.send)(Event::TitleUnmount);
    }

    /// Returns attributes for the optional dismiss button.
    #[must_use]
    pub fn dismiss_button_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.dismiss_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Dispatches dismiss-button activation.
    pub fn on_dismiss_button_click(&self) {
        (self.send)(Event::Close);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
            Part::Title => self.title_attrs(),
            Part::DismissButton => self.dismiss_button_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, string::ToString, vec};
    use core::{cell::RefCell, time::Duration};

    use ars_core::{AriaAttr, AttrMap, CssProperty, Env, HtmlAttr, Service};
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;
    use crate::overlay::positioning::{Placement, PositioningOptions, PositioningSnapshot};

    fn test_props() -> Props {
        Props {
            id: "hover-card".to_string(),
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

    #[test]
    fn default_init_starts_closed() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(service.context().open_delay, Duration::from_millis(700));
        assert_eq!(service.context().close_delay, Duration::from_millis(300));
        assert_eq!(service.context().trigger_id, "hover-card-trigger");
        assert_eq!(service.context().content_id, "hover-card-content");
    }

    #[test]
    fn controlled_open_overrides_default() {
        let service = Service::<Machine>::new(
            Props {
                open: Some(false),
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    #[test]
    fn hover_trigger_opens_after_delay() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let enter = service.send(Event::TriggerPointerEnter);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().hover_active);
        assert_eq!(effect_names(&enter), vec![Effect::OpenDelay]);

        let fired = service.send(Event::OpenTimerFired);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(fired.cancel_effects, vec![Effect::OpenDelay]);
        assert_eq!(
            effect_names(&fired),
            vec![Effect::OpenChange, Effect::AllocateZIndex]
        );
    }

    #[test]
    fn focus_and_keyboard_open_accessibly() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let focus = service.send(Event::TriggerFocus);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().focus_active);
        assert_eq!(effect_names(&focus), vec![Effect::OpenDelay]);

        let open = service.send(Event::TriggerKeyDown(KeyboardKey::Enter));

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(!service.context().hover_active);
        assert_eq!(open.cancel_effects, vec![Effect::OpenDelay]);

        let observed = Rc::new(RefCell::new(Vec::new()));

        let send = {
            let observed = Rc::clone(&observed);
            move |event| observed.borrow_mut().push(event)
        };

        let api = service.connect(&send);

        assert!(!api.on_trigger_keydown(&keyboard_data(KeyboardKey::Space)));
        assert!(observed.borrow().is_empty());
    }

    #[test]
    fn trigger_keydown_consumes_only_handled_keys() {
        let closed = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());
        let observed = Rc::new(RefCell::new(Vec::new()));
        let send = {
            let observed = Rc::clone(&observed);
            move |event| observed.borrow_mut().push(event)
        };
        let closed_api = closed.connect(&send);

        assert!(closed_api.on_trigger_keydown(&keyboard_data(KeyboardKey::Enter)));
        assert!(!closed_api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert_eq!(
            &*observed.borrow(),
            &[Event::TriggerKeyDown(KeyboardKey::Enter)]
        );

        let open = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );
        let observed = Rc::new(RefCell::new(Vec::new()));
        let send = {
            let observed = Rc::clone(&observed);
            move |event| observed.borrow_mut().push(event)
        };
        let open_api = open.connect(&send);

        assert!(!open_api.on_trigger_keydown(&keyboard_data(KeyboardKey::Space)));
        assert!(open_api.on_trigger_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert_eq!(&*observed.borrow(), &[Event::CloseOnEscape]);
    }

    #[test]
    fn content_enter_cancels_close_delay() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let leave = service.send(Event::TriggerPointerLeave);

        assert_eq!(service.state(), &State::ClosePending);
        assert_eq!(effect_names(&leave), vec![Effect::CloseDelay]);

        let enter = service.send(Event::ContentPointerEnter);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().hover_active);
        assert_eq!(enter.cancel_effects, vec![Effect::CloseDelay]);
    }

    #[test]
    fn focus_during_pending_hover_open_survives_pointer_leave() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::TriggerPointerEnter));

        let focus = service.send(Event::TriggerFocus);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(service.context().focus_active);
        assert!(effect_names(&focus).is_empty());

        let leave = service.send(Event::TriggerPointerLeave);

        assert_eq!(service.state(), &State::OpenPending);
        assert!(!service.context().hover_active);
        assert!(service.context().focus_active);
        assert!(effect_names(&leave).is_empty());

        drop(service.send(Event::OpenTimerFired));

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    #[test]
    fn content_focus_cancels_close_delay_after_trigger_blur() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::TriggerFocus));

        let blur = service.send(Event::TriggerBlur);

        assert_eq!(service.state(), &State::ClosePending);
        assert_eq!(effect_names(&blur), vec![Effect::CloseDelay]);

        let content_focus = service.send(Event::ContentFocus);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().focus_active);
        assert_eq!(content_focus.cancel_effects, vec![Effect::CloseDelay]);

        let content_blur = service.send(Event::ContentBlur);

        assert_eq!(service.state(), &State::ClosePending);
        assert!(!service.context().focus_active);
        assert_eq!(effect_names(&content_blur), vec![Effect::CloseDelay]);
    }

    #[test]
    fn close_timer_and_escape_close() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::ContentPointerLeave));

        let close = service.send(Event::CloseTimerFired);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(close.cancel_effects, vec![Effect::CloseDelay]);
        assert_eq!(
            effect_names(&close),
            vec![Effect::OpenChange, Effect::ReleaseZIndex]
        );

        drop(service.send(Event::Open));

        let escape = service.send(Event::CloseOnEscape);

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(
            effect_names(&escape),
            vec![Effect::OpenChange, Effect::ReleaseZIndex]
        );
    }

    #[test]
    fn disabled_suppresses_user_interaction_but_allows_sync() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let enter = service.send(Event::TriggerPointerEnter);

        assert!(!enter.state_changed);
        assert_eq!(service.state(), &State::Closed);

        let controlled = service.send(Event::SetControlledOpen(true));

        assert_eq!(service.state(), &State::Open);
        assert!(controlled.state_changed);
    }

    #[test]
    fn disabled_open_hover_card_still_allows_close_cleanup() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        service.set_props(Props {
            disabled: true,
            ..test_props()
        });

        let leave = service.send(Event::TriggerPointerLeave);

        assert_eq!(service.state(), &State::ClosePending);
        assert_eq!(effect_names(&leave), vec![Effect::CloseDelay]);

        let close = service.send(Event::CloseTimerFired);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(
            effect_names(&close),
            vec![Effect::OpenChange, Effect::ReleaseZIndex]
        );
    }

    #[test]
    fn controlled_open_close_cancel_pending_timers_without_spurious_open_change() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::TriggerPointerEnter));

        let controlled_close = service.send(Event::SetControlledOpen(false));

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(controlled_close.cancel_effects, vec![Effect::OpenDelay]);
        assert!(controlled_close.pending_effects.is_empty());

        drop(service.send(Event::Open));
        drop(service.send(Event::TriggerPointerLeave));

        let controlled_open = service.send(Event::SetControlledOpen(true));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(controlled_open.cancel_effects, vec![Effect::CloseDelay]);
        assert_eq!(
            effect_names(&controlled_open),
            vec![Effect::OpenChange, Effect::AllocateZIndex]
        );
    }

    #[test]
    fn controlled_close_request_preserves_open_state_until_prop_sync() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::TriggerPointerEnter));
        drop(service.send(Event::TriggerFocus));

        let close = service.send(Event::Close);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(!service.context().hover_active);
        assert!(!service.context().focus_active);
        assert_eq!(effect_names(&close), vec![Effect::OpenChange]);
    }

    #[test]
    fn controlled_false_open_timer_requests_without_opening() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::TriggerPointerEnter));

        let timer = service.send(Event::OpenTimerFired);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(timer.cancel_effects, vec![Effect::OpenDelay]);
        assert_eq!(effect_names(&timer), vec![Effect::OpenChange]);

        let key = service.send(Event::TriggerKeyDown(KeyboardKey::Enter));

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(effect_names(&key), vec![Effect::OpenChange]);
    }

    #[test]
    fn controlled_true_close_timer_requests_without_closing() {
        let mut service = Service::<Machine>::new(
            Props {
                open: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::TriggerPointerLeave));

        let timer = service.send(Event::CloseTimerFired);

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert_eq!(timer.cancel_effects, vec![Effect::CloseDelay]);
        assert_eq!(effect_names(&timer), vec![Effect::OpenChange]);
    }

    #[test]
    fn escape_before_open_cancels_delay_without_open_change() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::TriggerPointerEnter));

        let escape = service.send(Event::CloseOnEscape);

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(escape.cancel_effects, vec![Effect::OpenDelay]);
        assert!(escape.pending_effects.is_empty());
    }

    #[test]
    fn props_sync_preserves_measured_placement_for_non_positioning_changes() {
        let mut service = Service::<Machine>::new(
            Props {
                positioning: PositioningOptions {
                    placement: Placement::RightStart,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopEnd,
            arrow: None,
        })));

        service.set_props(Props {
            disabled: true,
            positioning: PositioningOptions {
                placement: Placement::RightStart,
                ..PositioningOptions::default()
            },
            ..test_props()
        });

        assert_eq!(service.context().current_placement, Placement::TopEnd);

        service.set_props(Props {
            positioning: PositioningOptions {
                placement: Placement::LeftStart,
                ..PositioningOptions::default()
            },
            ..test_props()
        });

        assert_eq!(service.context().current_placement, Placement::LeftStart);
    }

    #[test]
    fn positioning_title_and_attrs_reflect_state() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                positioning: PositioningOptions {
                    placement: Placement::RightStart,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetZIndex(42)));
        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopEnd,
            arrow: None,
        })));

        let api = service.connect(&|_| {});

        let trigger = api.trigger_attrs();
        let positioner = api.positioner_attrs();
        let content = api.content_attrs();

        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("dialog")
        );
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("hover-card-content")
        );
        assert_eq!(
            positioner.get(&HtmlAttr::Data("ars-placement")),
            Some("top-end")
        );
        assert!(
            positioner
                .styles()
                .contains(&(CssProperty::Custom("ars-z-index"), "42".to_string()))
        );
        assert_eq!(content.get(&HtmlAttr::Role), Some("dialog"));
        assert_eq!(
            content.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Hover card")
        );

        drop(service.send(Event::TitleMount));

        let labelled = service.connect(&|_| {}).content_attrs();

        assert_eq!(
            labelled.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("hover-card-title")
        );
        assert!(!labelled.contains(&HtmlAttr::Aria(AriaAttr::Label)));

        drop(service.send(Event::TitleUnmount));

        let fallback = service.connect(&|_| {}).content_attrs();

        assert_eq!(
            fallback.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Hover card")
        );
        assert!(!fallback.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
    }

    #[test]
    fn hover_card_root_closed_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn hover_card_root_open_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn hover_card_root_disabled_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn hover_card_trigger_closed_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).trigger_attrs()));
    }

    #[test]
    fn hover_card_trigger_open_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).trigger_attrs()));
    }

    #[test]
    fn hover_card_positioner_default_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).positioner_attrs()));
    }

    #[test]
    fn hover_card_positioner_with_z_index_snapshot() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                positioning: PositioningOptions {
                    placement: Placement::LeftStart,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetZIndex(123)));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).positioner_attrs()));
    }

    #[test]
    fn hover_card_content_fallback_label_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).content_attrs()));
    }

    #[test]
    fn hover_card_content_labelled_by_title_snapshot() {
        let mut service = Service::<Machine>::new(
            Props {
                default_open: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::TitleMount));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).content_attrs()));
    }

    #[test]
    fn hover_card_arrow_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                positioning: PositioningOptions {
                    placement: Placement::Top,
                    ..PositioningOptions::default()
                },
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).arrow_attrs()));
    }

    #[test]
    fn hover_card_title_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).title_attrs()));
    }

    #[test]
    fn hover_card_dismiss_button_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).dismiss_button_attrs()
        ));
    }
}
