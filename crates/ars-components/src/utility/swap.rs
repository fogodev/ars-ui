//! Swap component state machine and connect API.

use alloc::string::String;
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentPart, ConnectApi, Env, HtmlAttr,
    Locale, MessageFn, PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

/// The state of the `Swap` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an off state.
    Off,

    /// The component is in an on state.
    On,
}

/// The events for the `Swap` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Toggle between on and off states.
    Toggle,

    /// Explicitly set to on.
    SetOn,

    /// Explicitly set to off.
    SetOff,

    /// Update disabled state from props.
    SetDisabled(bool),

    /// Synchronize the externally controlled checked prop.
    SetValue(Option<bool>),

    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus lost.
    Blur,
}

/// The animation style for the swap transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Animation {
    /// No animation; instant swap.
    #[default]
    None,

    /// Content rotates 180 degrees during the swap.
    Rotate,

    /// Content flips along the Y-axis.
    Flip,

    /// Cross-fade between slots.
    Fade,
}

/// The context of the `Swap` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current checked on/off state.
    pub checked: Bindable<bool>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether focus was received via keyboard.
    pub focus_visible: bool,

    /// Component instance IDs.
    pub ids: ComponentIds,

    /// Active locale inherited from provider context.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

/// Props for the `Swap` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled checked state. If `Some`, the component is controlled.
    pub checked: Option<bool>,

    /// Default checked state for uncontrolled mode.
    pub default_checked: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Stable accessible label for the root element.
    pub label: Option<String>,

    /// The animation style for the swap transition.
    pub animation: Animation,

    /// Callback invoked when user intent requests a new checked state.
    pub on_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            checked: None,
            default_checked: false,
            disabled: false,
            label: None,
            animation: Animation::None,
            on_change: None,
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

    /// Sets [`checked`](Self::checked), switching the swap to controlled mode.
    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Clears [`checked`](Self::checked), switching the swap to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.checked = None;
        self
    }

    /// Sets [`default_checked`](Self::default_checked).
    #[must_use]
    pub const fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`label`](Self::label).
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets [`animation`](Self::animation).
    #[must_use]
    pub const fn animation(mut self, animation: Animation) -> Self {
        self.animation = animation;
        self
    }

    /// Sets [`on_change`](Self::on_change).
    #[must_use]
    pub fn on_change(mut self, callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>) -> Self {
        self.on_change = Some(callback.into());
        self
    }
}

/// The messages for the `Swap` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// State-specific label for the on state.
    pub on_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// State-specific label for the off state.
    pub off_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            on_label: MessageFn::static_str("On"),
            off_label: MessageFn::static_str("Off"),
        }
    }
}

impl ars_core::ComponentMessages for Messages {}

/// Typed effect intents emitted by the swap machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_change`] with the requested checked value.
    Change,
}

/// The machine for the `Swap` component.
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
        let initial = props.checked.unwrap_or(props.default_checked);

        (
            state_from_checked(initial),
            Context {
                checked: match props.checked {
                    Some(value) => Bindable::controlled(value),
                    None => Bindable::uncontrolled(props.default_checked),
                },
                disabled: props.disabled,
                focus_visible: false,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && matches!(event, Event::Toggle | Event::SetOn | Event::SetOff) {
            return None;
        }

        match (state, event) {
            (_, Event::SetValue(value)) => Some(sync_value_plan(*value, props.checked.is_some())),

            (_, Event::SetDisabled(disabled)) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                }))
            }

            (State::Off, Event::Toggle) | (_, Event::SetOn) => Some(value_change_plan(ctx, true)),

            (State::On, Event::Toggle) | (_, Event::SetOff) => Some(value_change_plan(ctx, false)),

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focus_visible = false;
            })),
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "swap::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.checked != new.checked {
            events.push(Event::SetValue(new.checked));
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

/// DOM parts of the `Swap` component.
#[derive(ComponentPart)]
#[scope = "swap"]
pub enum Part {
    /// The interactive root element.
    Root,

    /// Content shown when on.
    OnContent,

    /// Content shown when off.
    OffContent,
}

/// The API for the `Swap` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("swap::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the swap is in its on state.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        matches!(self.state, State::On)
    }

    /// Whether the swap is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "button")
            .set(HtmlAttr::Aria(AriaAttr::Pressed), self.aria_pressed())
            .set(HtmlAttr::TabIndex, "0")
            .set(HtmlAttr::Data("ars-state"), self.data_state());

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if let Some(label) = &self.props.label {
            if !label.is_empty() {
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.clone());
            }
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.current_label());
        }

        attrs
    }

    /// Attributes for the on content container.
    #[must_use]
    pub fn on_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OnContent.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if !self.is_on() {
            attrs
                .set_bool(HtmlAttr::Hidden, true)
                .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }

        attrs
    }

    /// Attributes for the off content container.
    #[must_use]
    pub fn off_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OffContent.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.is_on() {
            attrs
                .set_bool(HtmlAttr::Hidden, true)
                .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }

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

    /// Handles keyboard activation for the root.
    pub fn on_keydown(&self, data: &KeyboardEventData) {
        if matches!(data.key, KeyboardKey::Enter | KeyboardKey::Space) {
            (self.send)(Event::Toggle);
        }
    }

    const fn data_state(&self) -> &'static str {
        if self.is_on() { "on" } else { "off" }
    }

    const fn aria_pressed(&self) -> &'static str {
        if self.is_on() { "true" } else { "false" }
    }

    fn current_label(&self) -> String {
        if self.is_on() {
            (self.ctx.messages.on_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.off_label)(&self.ctx.locale)
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::OnContent => self.on_content_attrs(),
            Part::OffContent => self.off_content_attrs(),
        }
    }
}

fn value_change_plan(ctx: &Context, next: bool) -> TransitionPlan<Machine> {
    if *ctx.checked.get() == next {
        return TransitionPlan::new();
    }

    if ctx.checked.is_controlled() {
        return TransitionPlan::new()
            .apply(|_: &mut Context| {})
            .with_effect(change_effect(next));
    }

    TransitionPlan::to(state_from_checked(next))
        .apply(move |ctx: &mut Context| {
            ctx.checked.set(next);
        })
        .with_effect(change_effect(next))
}

fn sync_value_plan(value: Option<bool>, is_controlled: bool) -> TransitionPlan<Machine> {
    if let Some(value) = value {
        TransitionPlan::to(state_from_checked(value)).apply(move |ctx: &mut Context| {
            ctx.checked.set(value);

            if is_controlled {
                ctx.checked.sync_controlled(Some(value));
            } else {
                ctx.checked.sync_controlled(None);
            }
        })
    } else {
        TransitionPlan::context_only(|ctx: &mut Context| {
            ctx.checked.sync_controlled(None);
        })
    }
}

fn change_effect(next: bool) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::Change, move |_ctx, props: &Props, _send| {
        if let Some(cb) = &props.on_change {
            cb(next);
        }

        no_cleanup()
    })
}

const fn state_from_checked(checked: bool) -> State {
    if checked { State::On } else { State::Off }
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
            id: "theme".into(),
            ..Props::default()
        }
    }

    fn key_data(key: KeyboardKey) -> KeyboardEventData {
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

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn swap_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("theme")
            .checked(true)
            .uncontrolled()
            .default_checked(true)
            .disabled(true)
            .label("Toggle theme")
            .animation(Animation::Flip)
            .on_change(callback(|_: bool| {}));

        assert_eq!(props.id, "theme");
        assert_eq!(props.checked, None);
        assert!(props.default_checked);
        assert!(props.disabled);
        assert_eq!(props.label.as_deref(), Some("Toggle theme"));
        assert_eq!(props.animation, Animation::Flip);
        assert!(props.on_change.is_some());
    }

    #[test]
    fn swap_initial_state_is_off() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert!(!service.context().checked.is_controlled());
        assert!(!service.context().disabled);
        assert!(!service.context().focus_visible);
        assert_eq!(service.context().ids.id(), "theme");
    }

    #[test]
    fn swap_default_checked_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                default_checked: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
    }

    #[test]
    fn swap_controlled_checked_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                checked: Some(true),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(service.context().checked.is_controlled());
    }

    #[test]
    fn swap_toggle_cycles_off_on_off() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert!(service.send(Event::Toggle).state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());

        assert!(service.send(Event::Toggle).state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
    }

    #[test]
    fn swap_set_on_and_set_off_are_idempotent() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let result = service.send(Event::SetOff);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());

        drop(service.send(Event::SetOn));

        let result = service.send(Event::SetOn);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::On);
    }

    #[test]
    fn swap_focus_and_blur_track_focus_visible() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focus_visible);
    }

    #[test]
    fn swap_controlled_user_toggle_emits_change_without_committing_state() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let result = service.send(Event::Toggle);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::Change);
    }

    #[test]
    fn swap_set_props_syncs_controlled_checked() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let result = service.set_props(Props {
            checked: Some(true),
            ..test_props()
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());

        drop(service.set_props(Props {
            checked: None,
            ..test_props()
        }));

        assert!(!service.context().checked.is_controlled());
    }

    #[test]
    fn swap_disabled_blocks_value_events() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        assert!(!service.send(Event::Toggle).state_changed);
        assert!(!service.send(Event::SetOn).state_changed);
        assert!(!service.send(Event::SetOff).state_changed);
        assert_eq!(service.state(), &State::Off);
    }

    #[test]
    fn swap_user_toggle_runs_change_callback() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let captured_changes = Arc::clone(&changes);
        let mut service = Service::<Machine>::new(
            Props {
                on_change: Some(callback(move |checked: bool| {
                    captured_changes.lock().unwrap().push(checked);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let result = service.send(Event::Toggle);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(*changes.lock().unwrap(), vec![true]);
    }

    #[test]
    fn swap_animation_default_is_none() {
        assert_eq!(Animation::default(), Animation::None);
        assert_eq!(Props::default().animation, Animation::None);
    }

    #[test]
    fn swap_part_attrs_dispatches_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::OnContent), api.on_content_attrs());
        assert_eq!(api.part_attrs(Part::OffContent), api.off_content_attrs());
    }

    #[test]
    fn swap_root_handlers_dispatch_typed_events() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured_sent = Arc::clone(&sent);
        let send = move |event| {
            captured_sent.lock().unwrap().push(event);
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let api = service.connect(&send);

        api.on_root_click();
        api.on_root_focus(false);
        api.on_root_blur();

        assert_eq!(
            *sent.lock().unwrap(),
            vec![
                Event::Toggle,
                Event::Focus { is_keyboard: false },
                Event::Blur,
            ]
        );
    }

    #[test]
    fn swap_keyboard_handler_toggles_only_activation_keys() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured_sent = Arc::clone(&sent);
        let send = move |event| {
            captured_sent.lock().unwrap().push(event);
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let api = service.connect(&send);

        api.on_keydown(&key_data(KeyboardKey::Enter));
        api.on_keydown(&key_data(KeyboardKey::Space));
        api.on_keydown(&key_data(KeyboardKey::Escape));

        assert_eq!(*sent.lock().unwrap(), vec![Event::Toggle, Event::Toggle]);
    }

    #[test]
    fn swap_root_attrs_include_required_contract() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let attrs = service.connect(&|_| {}).root_attrs();

        let api = service.connect(&|_| {});

        assert!(!api.is_disabled());
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("swap"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("off"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("false"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Off"));
    }

    #[test]
    fn swap_snapshots_cover_output_branches() {
        let off = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_snapshot!(
            "swap_root_off",
            snapshot_attrs(&off.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "swap_on_content_hidden",
            snapshot_attrs(&off.connect(&|_| {}).on_content_attrs())
        );
        assert_snapshot!(
            "swap_off_content_visible",
            snapshot_attrs(&off.connect(&|_| {}).off_content_attrs())
        );

        let mut on = Service::<Machine>::new(
            Props {
                label: Some("Toggle theme".into()),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(on.send(Event::Toggle));

        assert_snapshot!(
            "swap_root_on_label",
            snapshot_attrs(&on.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "swap_on_content_visible",
            snapshot_attrs(&on.connect(&|_| {}).on_content_attrs())
        );
        assert_snapshot!(
            "swap_off_content_hidden",
            snapshot_attrs(&on.connect(&|_| {}).off_content_attrs())
        );

        let mut disabled = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(disabled.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "swap_root_disabled_focus_visible",
            snapshot_attrs(&disabled.connect(&|_| {}).root_attrs())
        );
    }
}
