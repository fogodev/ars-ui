//! ToggleButton component state machine and connect API.

use alloc::string::String;
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, PendingEffect, TransitionPlan, no_cleanup,
};

/// The states for the `ToggleButton` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default resting state. Not focused and not being actively pressed.
    Idle,

    /// The button has received focus.
    Focused,

    /// The button is actively being pressed.
    Pressed,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused => f.write_str("focused"),
            Self::Pressed => f.write_str("pressed"),
        }
    }
}

/// Events for the `ToggleButton` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus received; flag indicates keyboard versus pointer source.
    Focus {
        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus lost; resets focus-visible state.
    Blur,

    /// Pointer or keyboard press begins.
    Press,

    /// Pointer or keyboard press ends and toggles the persistent pressed state.
    Release,

    /// Programmatically toggle the persistent pressed state.
    Toggle,

    /// Programmatically set the persistent pressed state.
    SetPressed(bool),

    /// Programmatically set the disabled state.
    SetDisabled(bool),

    /// Restore pressed state to [`Props::default_pressed`] for native form reset.
    Reset,
}

/// The context for the `ToggleButton` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled or uncontrolled persistent pressed value.
    pub pressed: Bindable<bool>,

    /// Whether the button is disabled.
    pub disabled: bool,

    /// Whether the button is currently focused.
    pub focused: bool,

    /// Whether focus should render as keyboard-visible focus.
    pub focus_visible: bool,
}

/// Props for the `ToggleButton` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled pressed value. If `Some`, the component is controlled.
    pub pressed: Option<bool>,

    /// Default uncontrolled pressed value.
    pub default_pressed: bool,

    /// Whether the button is disabled.
    pub disabled: bool,

    /// Whether the field is in an invalid state.
    pub invalid: bool,

    /// Whether the field is required.
    pub required: bool,

    /// Identifier when used within a `ToggleGroup`.
    pub value: Option<String>,

    /// Form field name used for standalone hidden input submission.
    pub name: Option<String>,

    /// Associated form ID threaded to hidden input configuration.
    pub form: Option<String>,

    /// Whether adapters should suppress pointer-induced focus.
    pub prevent_focus_on_press: bool,

    /// Callback invoked when user intent requests a new pressed state.
    pub on_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback fired by adapters when hover starts.
    pub on_hover_start: Option<Callback<dyn Fn() + Send + Sync>>,

    /// Callback fired by adapters when hover ends.
    pub on_hover_end: Option<Callback<dyn Fn() + Send + Sync>>,

    /// Callback fired by adapters when hover state changes.
    pub on_hover_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
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

    /// Sets [`pressed`](Self::pressed), switching the component to controlled mode.
    #[must_use]
    pub const fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Clears [`pressed`](Self::pressed), switching the component to uncontrolled mode.
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

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`value`](Self::value).
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Sets [`prevent_focus_on_press`](Self::prevent_focus_on_press).
    #[must_use]
    pub const fn prevent_focus_on_press(mut self, prevent: bool) -> Self {
        self.prevent_focus_on_press = prevent;
        self
    }

    /// Sets [`on_change`](Self::on_change).
    #[must_use]
    pub fn on_change(mut self, callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>) -> Self {
        self.on_change = Some(callback.into());
        self
    }

    /// Sets [`on_hover_start`](Self::on_hover_start).
    #[must_use]
    pub fn on_hover_start(mut self, callback: impl Into<Callback<dyn Fn() + Send + Sync>>) -> Self {
        self.on_hover_start = Some(callback.into());
        self
    }

    /// Sets [`on_hover_end`](Self::on_hover_end).
    #[must_use]
    pub fn on_hover_end(mut self, callback: impl Into<Callback<dyn Fn() + Send + Sync>>) -> Self {
        self.on_hover_end = Some(callback.into());
        self
    }

    /// Sets [`on_hover_change`](Self::on_hover_change).
    #[must_use]
    pub fn on_hover_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>,
    ) -> Self {
        self.on_hover_change = Some(callback.into());
        self
    }
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the toggle button machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_change`] with the requested pressed value.
    PressedChange,
}

/// Hidden input configuration for standalone native form submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenInputConfig {
    /// Form field name.
    pub name: String,

    /// Submitted value.
    pub value: HiddenInputValue,

    /// Optional ID of an associated form element.
    pub form_id: Option<String>,

    /// Whether the hidden input should render disabled.
    pub disabled: bool,
}

/// Hidden input value shape for native form submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiddenInputValue {
    /// Submit one scalar value for this form field.
    Single(String),
}

/// The machine for the `ToggleButton` component.
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
        (
            State::Idle,
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
        if ctx.disabled
            && !matches!(
                event,
                Event::Focus { .. }
                    | Event::Blur
                    | Event::SetPressed(_)
                    | Event::SetDisabled(_)
                    | Event::Reset
            )
        {
            return None;
        }

        match (state, event) {
            (State::Idle, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }),
                )
            }

            (State::Focused | State::Pressed, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (State::Focused | State::Pressed, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(clear_focus))
            }

            (State::Idle | State::Focused, Event::Press) => {
                Some(TransitionPlan::to(State::Pressed))
            }

            (State::Pressed, Event::Release) => {
                let next = !*ctx.pressed.get();
                let target = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(value_change_plan(ctx, target, next))
            }

            (_, Event::Toggle) => {
                let next = !*ctx.pressed.get();
                Some(value_change_plan(ctx, *state, next))
            }

            (_, Event::SetPressed(value)) => Some(set_pressed_plan(
                ctx,
                *state,
                *value,
                props.pressed.is_some(),
            )),

            (_, Event::SetDisabled(disabled)) => {
                let disabled = *disabled;
                if disabled && matches!(state, State::Focused | State::Pressed) {
                    Some(
                        TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                            ctx.disabled = disabled;
                            clear_focus(ctx);
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.disabled = disabled;
                    }))
                }
            }

            (_, Event::Reset) => Some(value_change_plan(ctx, *state, props.default_pressed)),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "toggle_button::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.pressed != new.pressed {
            events
                .push(Event::SetPressed(new.pressed.unwrap_or_else(|| {
                    old.pressed.unwrap_or(new.default_pressed)
                })));
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

/// DOM parts of the `ToggleButton` component.
#[derive(ComponentPart)]
#[scope = "toggle-button"]
pub enum Part {
    /// The root button element.
    Root,
}

/// The API for the `ToggleButton` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("toggle_button::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns whether the toggle button is persistently pressed.
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        *self.ctx.pressed.get()
    }

    /// Returns whether the toggle button is focused.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.ctx.focused
    }

    /// Returns whether focus should render as focus-visible.
    #[must_use]
    pub const fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    /// Returns whether the toggle button is disabled for interaction.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns whether adapters should suppress pointer-induced focus.
    #[must_use]
    pub const fn should_prevent_focus_on_press(&self) -> bool {
        self.props.prevent_focus_on_press
    }

    /// Root button element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Pressed), self.aria_pressed())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string())
            .set(HtmlAttr::TabIndex, "0");

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.is_pressed() {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        if let Some(value) = &self.props.value {
            attrs.set(HtmlAttr::Data("ars-value"), value.clone());
        }

        if self.props.prevent_focus_on_press {
            attrs.set(HtmlAttr::Data("ars-prevent-focus-on-press"), "true");
        }

        attrs
    }

    /// Hidden input configuration for standalone native form submission.
    #[must_use]
    pub fn hidden_input_config(&self) -> Option<HiddenInputConfig> {
        let name = self.props.name.as_ref()?;

        if self.ctx.disabled || !self.is_pressed() {
            return None;
        }

        Some(HiddenInputConfig {
            name: name.clone(),
            value: HiddenInputValue::Single(
                self.props
                    .value
                    .clone()
                    .unwrap_or_else(|| String::from("on")),
            ),
            form_id: self.props.form.clone(),
            disabled: false,
        })
    }

    /// Dispatches a focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a press event.
    pub fn on_press(&self) {
        (self.send)(Event::Press);
    }

    /// Dispatches a release event.
    pub fn on_release(&self) {
        (self.send)(Event::Release);
    }

    /// Dispatches a programmatic toggle event.
    pub fn on_toggle(&self) {
        (self.send)(Event::Toggle);
    }

    /// Dispatches a native form reset event.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
    }

    fn aria_pressed(&self) -> &'static str {
        if *self.ctx.pressed.get() {
            "true"
        } else {
            "false"
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

const fn clear_focus(ctx: &mut Context) {
    ctx.focused = false;
    ctx.focus_visible = false;
}

fn value_change_plan(ctx: &Context, target: State, next: bool) -> TransitionPlan<Machine> {
    if *ctx.pressed.get() == next {
        return TransitionPlan::to(target);
    }

    if ctx.pressed.is_controlled() {
        return TransitionPlan::to(target).with_effect(pressed_change_effect(next));
    }

    TransitionPlan::to(target)
        .apply(move |ctx: &mut Context| {
            ctx.pressed.set(next);
        })
        .with_effect(pressed_change_effect(next))
}

fn set_pressed_plan(
    ctx: &Context,
    target: State,
    next: bool,
    controlled_prop: bool,
) -> TransitionPlan<Machine> {
    if *ctx.pressed.get() == next && ctx.pressed.is_controlled() == controlled_prop {
        return TransitionPlan::new();
    }

    TransitionPlan::to(target).apply(move |ctx: &mut Context| {
        if controlled_prop {
            ctx.pressed.sync_controlled(Some(next));
        } else {
            ctx.pressed.sync_controlled(None);
            ctx.pressed.set(next);
        }
    })
}

fn pressed_change_effect(next: bool) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::PressedChange, move |_ctx, props: &Props, _send| {
        if let Some(cb) = &props.on_change {
            cb(next);
        }

        no_cleanup()
    })
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{AriaAttr, AttrMap, ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props::new().id("favorite")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages)
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn snapshot_config(config: Option<&HiddenInputConfig>) -> String {
        format!("{config:#?}")
    }

    #[test]
    fn toggle_button_initial_state_reflects_default_pressed() {
        let unpressed = service(test_props());

        assert_eq!(unpressed.state(), &State::Idle);
        assert!(!unpressed.context().pressed.get());
        assert!(!unpressed.context().pressed.is_controlled());

        let pressed = service(test_props().default_pressed(true));

        assert_eq!(pressed.state(), &State::Idle);
        assert!(*pressed.context().pressed.get());
        assert!(!pressed.context().pressed.is_controlled());
    }

    #[test]
    fn toggle_button_press_release_toggles_pressed() {
        let mut service = service(test_props());

        let press = service.send(Event::Press);

        assert!(press.state_changed);
        assert_eq!(service.state(), &State::Pressed);
        assert!(!service.context().pressed.get());

        let release = service.send(Event::Release);

        assert!(release.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(*service.context().pressed.get());
        assert_eq!(release.pending_effects.len(), 1);

        drop(service.send(Event::Press));
        drop(service.send(Event::Release));

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed.get());
    }

    #[test]
    fn toggle_button_controlled_press_emits_change_without_committing_state() {
        let mut service = service(test_props().pressed(false));

        drop(service.send(Event::Press));
        let release = service.send(Event::Release);

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed.get());
        assert_eq!(release.pending_effects.len(), 1);
        assert_eq!(release.pending_effects[0].name, Effect::PressedChange);
    }

    #[test]
    fn toggle_button_toggle_and_set_pressed_work_from_all_states() {
        let mut service = service(test_props());

        drop(service.send(Event::Toggle));

        assert!(*service.context().pressed.get());
        assert_eq!(service.state(), &State::Idle);

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::SetPressed(false)));

        assert!(!service.context().pressed.get());
        assert_eq!(service.state(), &State::Focused);

        drop(service.send(Event::Press));
        drop(service.send(Event::Toggle));

        assert!(*service.context().pressed.get());
        assert_eq!(service.state(), &State::Pressed);

        let unchanged = service.send(Event::SetPressed(true));

        assert!(!unchanged.state_changed);
        assert!(unchanged.pending_effects.is_empty());
    }

    #[test]
    fn toggle_button_disabled_ignores_press_events() {
        let mut service = service(test_props().disabled(true));

        assert!(!service.send(Event::Press).state_changed);
        assert!(!service.send(Event::Release).state_changed);
        assert!(!service.send(Event::Toggle).state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed.get());
    }

    #[test]
    fn toggle_button_disabled_allows_set_pressed_for_prop_sync() {
        let mut service = service(test_props().disabled(true));

        let result = service.send(Event::SetPressed(true));

        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(*service.context().pressed.get());
        assert!(service.context().disabled);
    }

    #[test]
    fn toggle_button_disabled_allows_focus_blur_reset_and_set_disabled() {
        let mut service = service(test_props().default_pressed(true).disabled(true));

        drop(service.send(Event::SetPressed(false)));
        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);

        drop(service.send(Event::Reset));

        assert!(*service.context().pressed.get());

        drop(service.send(Event::SetDisabled(false)));

        assert!(!service.context().disabled);
    }

    #[test]
    fn toggle_button_set_disabled_false_preserves_focused_state() {
        let mut service = service(test_props().disabled(true));

        drop(service.send(Event::Focus { is_keyboard: true }));

        let result = service.send(Event::SetDisabled(false));

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Focused);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);
        assert!(!service.context().disabled);
    }

    #[test]
    fn toggle_button_set_props_syncs_pressed_and_disabled_changes() {
        let old = test_props().pressed(false);

        let controlled = test_props().pressed(true);

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &controlled),
            vec![Event::SetPressed(true)],
        );

        let disabled = Props {
            disabled: true,
            ..old.clone()
        };

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &disabled),
            vec![Event::SetDisabled(true)],
        );

        let mut active = service(old);

        let result = active.set_props(controlled);

        assert!(result.context_changed);
        assert!(*active.context().pressed.get());
        assert!(active.context().pressed.is_controlled());

        let result = active.set_props(test_props());

        assert!(result.context_changed);
        assert!(*active.context().pressed.get());
        assert!(!active.context().pressed.is_controlled());

        let mut disabled_service = service(test_props().pressed(false).disabled(true));
        let result = disabled_service.set_props(test_props().pressed(true).disabled(true));

        assert!(result.context_changed);
        assert!(*disabled_service.context().pressed.get());
        assert!(disabled_service.context().disabled);
    }

    #[test]
    fn toggle_button_set_props_ignores_render_only_changes() {
        let old = test_props();
        let new = test_props()
            .invalid(true)
            .required(true)
            .value("bold")
            .name("format")
            .form("article")
            .prevent_focus_on_press(true)
            .on_hover_start(Callback::from(|| {}))
            .on_hover_end(Callback::from(|| {}))
            .on_hover_change(callback(|_: bool| {}));

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn toggle_button_tracks_focus_visible_from_keyboard() {
        let mut service = service(test_props());

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_eq!(service.state(), &State::Focused);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Focus { is_keyboard: false }));

        assert!(service.context().focused);
        assert!(!service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn toggle_button_api_accessors_reflect_context_and_props() {
        let mut active = service(test_props().prevent_focus_on_press(true));

        {
            let api = active.connect(&|_| {});

            assert!(!api.is_pressed());
            assert!(!api.is_focused());
            assert!(!api.is_focus_visible());
            assert!(!api.is_disabled());
            assert!(api.should_prevent_focus_on_press());
        }

        drop(active.send(Event::Focus { is_keyboard: true }));

        {
            let api = active.connect(&|_| {});

            assert!(api.is_focused());
            assert!(api.is_focus_visible());
        }

        drop(active.send(Event::SetDisabled(true)));

        let api = active.connect(&|_| {});

        assert!(!api.is_pressed());
        assert!(!api.is_focused());
        assert!(!api.is_focus_visible());
        assert!(api.is_disabled());
        assert!(api.should_prevent_focus_on_press());

        let service = service(test_props());

        let api = service.connect(&|_| {});

        assert!(!api.should_prevent_focus_on_press());
    }

    #[test]
    fn toggle_button_reset_restores_default_pressed() {
        let mut service = service(test_props().default_pressed(true));

        drop(service.send(Event::SetPressed(false)));

        assert!(!service.context().pressed.get());

        drop(service.send(Event::Reset));

        assert!(*service.context().pressed.get());
    }

    #[test]
    fn toggle_button_on_change_callback_fires_on_release_toggle_and_reset() {
        let changes = Arc::new(Mutex::new(Vec::new()));

        let mut service = service(test_props().default_pressed(true).on_change(callback({
            let changes = Arc::clone(&changes);

            move |pressed: bool| {
                changes.lock().unwrap().push(pressed);
            }
        })));

        drop(service.send(Event::Press));

        let release = service.send(Event::Release);
        let reset = service.send(Event::Reset);
        let toggle = service.send(Event::Toggle);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in release
            .pending_effects
            .into_iter()
            .chain(reset.pending_effects)
            .chain(toggle.pending_effects)
        {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(*changes.lock().unwrap(), vec![false, true, false]);
    }

    #[test]
    fn toggle_button_root_attrs_emit_accessibility_contract() {
        let service = service(test_props());

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("toggle-button"),
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("favorite"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("false"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn toggle_button_root_attrs_emit_invalid_required_and_value_branches() {
        let mut service = service(
            test_props()
                .invalid(true)
                .required(true)
                .value("bold")
                .prevent_focus_on_press(true),
        );

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::Toggle));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-value")), Some("bold"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-focus-visible")),
            Some("true")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-pressed")), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-prevent-focus-on-press")),
            Some("true"),
        );
    }

    #[test]
    fn toggle_button_hidden_input_config_absent_without_name() {
        let mut service = service(test_props());

        drop(service.send(Event::Toggle));

        assert_eq!(service.connect(&|_| {}).hidden_input_config(), None);
    }

    #[test]
    fn toggle_button_hidden_input_config_absent_when_unpressed() {
        let service = service(test_props().name("format"));

        assert_eq!(service.connect(&|_| {}).hidden_input_config(), None);
    }

    #[test]
    fn toggle_button_hidden_input_config_absent_when_disabled() {
        let service = service(
            test_props()
                .default_pressed(true)
                .name("format")
                .disabled(true),
        );

        assert_eq!(service.connect(&|_| {}).hidden_input_config(), None);
    }

    #[test]
    fn toggle_button_hidden_input_config_uses_value_or_on_and_form_id() {
        let explicit = service(
            test_props()
                .default_pressed(true)
                .name("format")
                .value("bold")
                .form("article"),
        );

        assert_eq!(
            explicit.connect(&|_| {}).hidden_input_config(),
            Some(HiddenInputConfig {
                name: "format".into(),
                value: HiddenInputValue::Single("bold".into()),
                form_id: Some("article".into()),
                disabled: false,
            }),
        );

        let fallback = service(test_props().default_pressed(true).name("format"));

        assert_eq!(
            fallback.connect(&|_| {}).hidden_input_config(),
            Some(HiddenInputConfig {
                name: "format".into(),
                value: HiddenInputValue::Single("on".into()),
                form_id: None,
                disabled: false,
            }),
        );
    }

    #[test]
    fn toggle_button_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("favorite")
            .pressed(true)
            .uncontrolled()
            .default_pressed(true)
            .disabled(true)
            .invalid(true)
            .required(true)
            .value("fav")
            .name("favorite")
            .form("profile")
            .prevent_focus_on_press(true)
            .on_change(callback(|_: bool| {}))
            .on_hover_start(Callback::from(|| {}))
            .on_hover_end(Callback::from(|| {}))
            .on_hover_change(callback(|_: bool| {}));

        assert_eq!(props.id, "favorite");
        assert_eq!(props.pressed, None);
        assert!(props.default_pressed);
        assert!(props.disabled);
        assert!(props.invalid);
        assert!(props.required);
        assert_eq!(props.value.as_deref(), Some("fav"));
        assert_eq!(props.name.as_deref(), Some("favorite"));
        assert_eq!(props.form.as_deref(), Some("profile"));
        assert!(props.prevent_focus_on_press);
        assert!(props.on_change.is_some());
        assert!(props.on_hover_start.is_some());
        assert!(props.on_hover_end.is_some());
        assert!(props.on_hover_change.is_some());
    }

    #[test]
    fn toggle_button_part_attrs_dispatches_root() {
        let service = service(test_props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn toggle_button_api_handlers_dispatch_typed_events() {
        let sent = Arc::new(Mutex::new(Vec::new()));

        let send = {
            let sent = Arc::clone(&sent);

            move |event| {
                sent.lock().unwrap().push(event);
            }
        };

        let service = service(test_props());

        let api = service.connect(&send);

        api.on_focus(true);
        api.on_blur();
        api.on_press();
        api.on_release();
        api.on_toggle();
        api.on_form_reset();

        assert_eq!(
            *sent.lock().unwrap(),
            vec![
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Press,
                Event::Release,
                Event::Toggle,
                Event::Reset,
            ],
        );
    }

    #[test]
    fn toggle_button_snapshots_cover_output_branches() {
        let idle = service(test_props());

        assert_snapshot!(
            "toggle_button_root_idle_unpressed",
            snapshot_attrs(&idle.connect(&|_| {}).root_attrs())
        );

        let mut focused = service(test_props());

        drop(focused.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "toggle_button_root_focused_keyboard",
            snapshot_attrs(&focused.connect(&|_| {}).root_attrs())
        );

        let mut transient_pressed = service(test_props());

        drop(transient_pressed.send(Event::Press));

        assert_snapshot!(
            "toggle_button_root_pressed_interaction",
            snapshot_attrs(&transient_pressed.connect(&|_| {}).root_attrs())
        );

        let mut toggled_on = service(test_props());

        drop(toggled_on.send(Event::Toggle));

        assert_snapshot!(
            "toggle_button_root_toggled_on",
            snapshot_attrs(&toggled_on.connect(&|_| {}).root_attrs())
        );

        let disabled = service(test_props().disabled(true));

        assert_snapshot!(
            "toggle_button_root_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).root_attrs())
        );

        let invalid_required = service(test_props().invalid(true).required(true));

        assert_snapshot!(
            "toggle_button_root_invalid_required",
            snapshot_attrs(&invalid_required.connect(&|_| {}).root_attrs())
        );

        let with_value = service(test_props().value("bold"));

        assert_snapshot!(
            "toggle_button_root_with_value",
            snapshot_attrs(&with_value.connect(&|_| {}).root_attrs())
        );

        let hidden_explicit = service(
            test_props()
                .default_pressed(true)
                .name("format")
                .value("bold")
                .form("article"),
        );

        assert_snapshot!(
            "toggle_button_hidden_input_config_explicit",
            snapshot_config(
                hidden_explicit
                    .connect(&|_| {})
                    .hidden_input_config()
                    .as_ref()
            )
        );

        let hidden_default = service(test_props().default_pressed(true).name("format"));

        assert_snapshot!(
            "toggle_button_hidden_input_config_default_on",
            snapshot_config(
                hidden_default
                    .connect(&|_| {})
                    .hidden_input_config()
                    .as_ref()
            )
        );
    }
}
