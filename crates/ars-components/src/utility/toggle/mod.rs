//! Toggle component state machine and connect API.

use alloc::string::String;
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentPart, ConnectApi, Env, HtmlAttr, PendingEffect,
    TransitionPlan, no_cleanup,
};

/// The state of the `Toggle` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The toggle is not pressed.
    Off,

    /// The toggle is pressed.
    On,
}

/// The events for the `Toggle` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Toggle between on and off.
    Toggle,

    /// Explicitly set to on.
    TurnOn,

    /// Explicitly set to off.
    TurnOff,

    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus lost.
    Blur,

    /// Sync disabled state from props.
    SetDisabled(bool),

    /// Synchronize the externally controlled pressed prop.
    SetValue(Option<bool>),
}

/// The context of the `Toggle` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled/uncontrolled pressed value.
    pub pressed: Bindable<bool>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether focus was received via keyboard.
    pub focus_visible: bool,
}

/// Props for the `Toggle` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled pressed value. If `Some`, the component is controlled.
    pub pressed: Option<bool>,

    /// Default uncontrolled pressed value.
    pub default_pressed: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Callback invoked when user intent requests a new pressed state.
    pub on_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
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

    /// Sets [`pressed`](Self::pressed), switching the toggle to controlled mode.
    #[must_use]
    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Clears [`pressed`](Self::pressed), switching the toggle to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.pressed = None;
        self
    }

    /// Sets [`default_pressed`](Self::default_pressed).
    #[must_use]
    pub const fn default_pressed(mut self, pressed: bool) -> Self {
        self.default_pressed = pressed;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`on_change`](Self::on_change).
    #[must_use]
    pub fn on_change(mut self, callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>) -> Self {
        self.on_change = Some(callback.into());
        self
    }
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ars_core::ComponentMessages for Messages {}

/// Typed effect intents emitted by the toggle machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_change`] with the requested pressed value.
    PressedChange,
}

/// The machine for the `Toggle` component.
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
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let initial = props.pressed.unwrap_or(props.default_pressed);

        (
            state_from_pressed(initial),
            Context {
                pressed: if let Some(value) = props.pressed {
                    Bindable::controlled(value)
                } else {
                    Bindable::uncontrolled(props.default_pressed)
                },
                disabled: props.disabled,
                focused: false,
                focus_visible: false,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && matches!(event, Event::Toggle | Event::TurnOn | Event::TurnOff) {
            return None;
        }

        match (state, event) {
            (_, Event::SetValue(value)) => Some(sync_value_plan(*value, props.pressed.is_some())),

            (_, Event::SetDisabled(disabled)) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                }))
            }

            (State::Off, Event::Toggle) | (_, Event::TurnOn) => Some(value_change_plan(ctx, true)),

            (State::On, Event::Toggle) | (_, Event::TurnOff) => Some(value_change_plan(ctx, false)),

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "toggle::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.pressed != new.pressed {
            events.push(Event::SetValue(new.pressed));
        }

        if old.disabled != new.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }

        events
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

/// DOM parts of the `Toggle` component.
#[derive(ComponentPart)]
#[scope = "toggle"]
pub enum Part {
    /// The root button element.
    Root,

    /// Optional indicator content.
    Indicator,
}

/// The API for the `Toggle` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("toggle::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the toggle is pressed.
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        matches!(self.state, State::On)
    }

    /// The attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.props.id.clone())
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Pressed), self.aria_pressed())
            .set(HtmlAttr::Data("ars-state"), self.data_state())
            .set(HtmlAttr::TabIndex, "0");

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.is_pressed() {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        attrs
    }

    /// The attributes for the indicator element.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.data_state());

        attrs
    }

    /// Dispatches a root click.
    pub fn on_root_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Dispatches root focus.
    pub fn on_root_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches root blur.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }

    const fn data_state(&self) -> &'static str {
        if self.is_pressed() { "on" } else { "off" }
    }

    const fn aria_pressed(&self) -> &'static str {
        if self.is_pressed() { "true" } else { "false" }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Indicator => self.indicator_attrs(),
        }
    }
}

fn value_change_plan(ctx: &Context, next: bool) -> TransitionPlan<Machine> {
    if *ctx.pressed.get() == next {
        return TransitionPlan::new();
    }

    if ctx.pressed.is_controlled() {
        return TransitionPlan::new()
            .apply(|_: &mut Context| {})
            .with_effect(pressed_change_effect(next));
    }

    TransitionPlan::to(state_from_pressed(next))
        .apply(move |ctx: &mut Context| {
            ctx.pressed.set(next);
        })
        .with_effect(pressed_change_effect(next))
}

fn sync_value_plan(value: Option<bool>, is_controlled: bool) -> TransitionPlan<Machine> {
    if let Some(value) = value {
        TransitionPlan::to(state_from_pressed(value)).apply(move |ctx: &mut Context| {
            ctx.pressed.set(value);

            if is_controlled {
                ctx.pressed.sync_controlled(Some(value));
            } else {
                ctx.pressed.sync_controlled(None);
            }
        })
    } else {
        TransitionPlan::context_only(|ctx: &mut Context| {
            ctx.pressed.sync_controlled(None);
        })
    }
}

fn pressed_change_effect(next: bool) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::PressedChange, move |_ctx, props: &Props, _send| {
        if let Some(cb) = &props.on_change {
            cb(next);
        }

        no_cleanup()
    })
}

const fn state_from_pressed(pressed: bool) -> State {
    if pressed { State::On } else { State::Off }
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "bold".into(),
            ..Props::default()
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn toggle_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("italic")
            .pressed(true)
            .uncontrolled()
            .default_pressed(true)
            .disabled(true)
            .on_change(callback(|_: bool| {}));

        assert_eq!(props.id, "italic");
        assert_eq!(props.pressed, None);
        assert!(props.default_pressed);
        assert!(props.disabled);
        assert!(props.on_change.is_some());
    }

    #[test]
    fn toggle_initial_state_is_off() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().pressed.get());
        assert!(!service.context().pressed.is_controlled());
        assert!(!service.context().disabled);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn toggle_default_pressed_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                default_pressed: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().pressed.get());
    }

    #[test]
    fn toggle_controlled_pressed_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                pressed: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().pressed.get());
        assert!(service.context().pressed.is_controlled());
    }

    #[test]
    fn toggle_event_cycles_off_on_off() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert!(service.send(Event::Toggle).state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().pressed.get());

        assert!(service.send(Event::Toggle).state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().pressed.get());
    }

    #[test]
    fn toggle_turn_on_and_turn_off_are_idempotent() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::TurnOff);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());

        drop(service.send(Event::TurnOn));

        let result = service.send(Event::TurnOn);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::On);
    }

    #[test]
    fn toggle_controlled_user_toggle_emits_change_without_committing_state() {
        let mut service = Service::<Machine>::new(
            Props {
                pressed: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Toggle);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().pressed.get());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::PressedChange);
    }

    #[test]
    fn toggle_set_props_syncs_controlled_pressed() {
        let mut service = Service::<Machine>::new(
            Props {
                pressed: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.set_props(Props {
            pressed: Some(true),
            ..test_props()
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().pressed.get());

        drop(service.set_props(Props {
            pressed: None,
            ..test_props()
        }));

        assert!(!service.context().pressed.is_controlled());
    }

    #[test]
    fn toggle_set_props_syncs_disabled_without_pressed_change() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(Props {
            disabled: true,
            ..test_props()
        });

        assert!(result.context_changed);
        assert!(service.context().disabled);

        let result = service.set_props(Props {
            disabled: true,
            ..test_props()
        });

        assert!(!result.context_changed);
        assert!(service.context().disabled);
    }

    #[test]
    fn toggle_disabled_blocks_value_events() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert!(!service.send(Event::Toggle).state_changed);
        assert!(!service.send(Event::TurnOn).state_changed);
        assert!(!service.send(Event::TurnOff).state_changed);
        assert_eq!(service.state(), &State::Off);
    }

    #[test]
    fn toggle_disabled_still_allows_focus_and_blur() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn toggle_user_toggle_runs_change_callback() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let captured_changes = Arc::clone(&changes);
        let mut service = Service::<Machine>::new(
            Props {
                on_change: Some(callback(move |pressed: bool| {
                    captured_changes.lock().unwrap().push(pressed);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Toggle);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(*changes.lock().unwrap(), vec![true]);
    }

    #[test]
    fn toggle_part_attrs_dispatches_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
    }

    #[test]
    fn toggle_root_handlers_dispatch_typed_events() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured_sent = Arc::clone(&sent);
        let send = move |event| {
            captured_sent.lock().unwrap().push(event);
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&send);

        api.on_root_click();
        api.on_root_focus(true);
        api.on_root_blur();

        assert_eq!(
            *sent.lock().unwrap(),
            vec![
                Event::Toggle,
                Event::Focus { is_keyboard: true },
                Event::Blur,
            ]
        );
    }

    #[test]
    fn toggle_root_attrs_include_required_contract() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("toggle"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("off"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("false"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
    }

    #[test]
    fn toggle_snapshots_cover_output_branches() {
        let off = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "toggle_root_off",
            snapshot_attrs(&off.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "toggle_indicator_off",
            snapshot_attrs(&off.connect(&|_| {}).indicator_attrs())
        );

        let mut on = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(on.send(Event::Toggle));

        assert_snapshot!(
            "toggle_root_on",
            snapshot_attrs(&on.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "toggle_indicator_on",
            snapshot_attrs(&on.connect(&|_| {}).indicator_attrs())
        );

        let mut disabled = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(disabled.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "toggle_root_disabled_focus_visible",
            snapshot_attrs(&disabled.connect(&|_| {}).root_attrs())
        );
    }
}
