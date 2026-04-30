//! Switch component state machine and connect API.
//!
//! This module implements the framework-agnostic `Switch` machine defined in
//! `spec/components/input/switch.md`. The machine owns two-state checked
//! state, focus-visible state, form metadata, text direction, and ARIA wiring
//! for every anatomy part.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, PendingEffect, TransitionPlan, no_cleanup,
};

/// The state of the `Switch` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an off state.
    Off,

    /// The component is in an on state.
    On,
}

/// Events for the `Switch` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Toggle between [`State::Off`] and [`State::On`].
    Toggle,

    /// Transition to [`State::On`].
    TurnOn,

    /// Transition to [`State::Off`].
    TurnOff,

    /// Restore checked state to [`Props::default_checked`] for form resets.
    Reset,

    /// Synchronize the externally controlled checked prop.
    SetValue(Option<bool>),

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether a description part is rendered.
    SetHasDescription(bool),

    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus lost.
    Blur,
}

/// Context for the `Switch` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current checked value, controlled or uncontrolled.
    pub checked: Bindable<bool>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// True when focus came from keyboard and should show a visible focus ring.
    pub focus_visible: bool,

    /// The name attribute for form submission.
    pub name: Option<String>,

    /// The ID of the form element the hidden input is associated with.
    pub form: Option<String>,

    /// Value submitted with the form when checked.
    pub value: String,

    /// User-facing label text provided by adapters or consumers.
    pub label: Option<String>,

    /// Whether a description part is rendered and should be referenced by ARIA.
    pub has_description: bool,

    /// Text direction for direction-aware styling.
    pub dir: Direction,

    /// Stable IDs for switch anatomy parts.
    pub ids: ComponentIds,
}

/// Props for the `Switch` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the switch.
    ///
    /// This ID is immutable for the lifetime of a machine instance because
    /// [`Context::ids`] caches the derived part IDs during initialization.
    pub id: String,

    /// Controlled checked value. When `Some`, the component is controlled.
    pub checked: Option<bool>,

    /// Default checked value for uncontrolled mode.
    pub default_checked: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// The name attribute for form submission.
    pub name: Option<String>,

    /// The ID of the form element the hidden input is associated with.
    pub form: Option<String>,

    /// Value attribute for form submission. Defaults to `"on"`.
    pub value: String,

    /// User-facing label text associated with the switch.
    pub label: Option<String>,

    /// Text direction used for direction-aware visual styling.
    pub dir: Direction,

    /// Called after user intent requests a new checked value.
    pub on_checked_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            checked: None,
            default_checked: false,
            disabled: false,
            required: false,
            invalid: false,
            readonly: false,
            name: None,
            form: None,
            value: "on".into(),
            label: None,
            dir: Direction::Ltr,
            on_checked_change: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the adapter-provided base ID for the switch.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`checked`](Self::checked), switching the switch to controlled mode.
    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Clears [`checked`](Self::checked), switching the switch to uncontrolled mode.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.checked = None;
        self
    }

    /// Sets [`default_checked`](Self::default_checked) for uncontrolled mode.
    #[must_use]
    pub const fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`name`](Self::name), the form-submission field name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears [`name`](Self::name).
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets [`form`](Self::form), the associated form element ID.
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Clears [`form`](Self::form).
    #[must_use]
    pub fn no_form(mut self) -> Self {
        self.form = None;
        self
    }

    /// Sets [`value`](Self::value), the submitted value when checked.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    /// Sets [`label`](Self::label), the user-facing switch label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Clears [`label`](Self::label).
    #[must_use]
    pub fn no_label(mut self) -> Self {
        self.label = None;
        self
    }

    /// Sets [`dir`](Self::dir), the text direction for visual styling.
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`on_checked_change`](Self::on_checked_change).
    #[must_use]
    pub fn on_checked_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>,
    ) -> Self {
        self.on_checked_change = Some(callback.into());
        self
    }

    /// Clears [`on_checked_change`](Self::on_checked_change).
    #[must_use]
    pub fn no_checked_change(mut self) -> Self {
        self.on_checked_change = None;
        self
    }
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ars_core::ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the switch machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes `Props::on_checked_change` with the new value.
    CheckedChange,
}

/// Machine for the `Switch` component.
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
        let initial = props.checked.unwrap_or(props.default_checked);

        (
            state_from_checked(initial),
            Context {
                checked: match props.checked {
                    Some(value) => Bindable::controlled(value),
                    None => Bindable::uncontrolled(props.default_checked),
                },
                disabled: props.disabled,
                required: props.required,
                invalid: props.invalid,
                readonly: props.readonly,
                focused: false,
                focus_visible: false,
                name: props.name.clone(),
                form: props.form.clone(),
                value: props.value.clone(),
                label: props.label.clone(),
                has_description: false,
                dir: props.dir,
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
        if (ctx.disabled || ctx.readonly)
            && matches!(event, Event::Toggle | Event::TurnOn | Event::TurnOff)
        {
            return None;
        }

        match (state, event) {
            (_, Event::Reset) => Some(reset_plan(ctx, props.default_checked)),

            (_, Event::SetValue(value)) => {
                if let Some(value) = value {
                    let value = *value;
                    let is_controlled = props.checked.is_some();
                    Some(TransitionPlan::to(state_from_checked(value)).apply(
                        move |ctx: &mut Context| {
                            ctx.checked.set(value);

                            if is_controlled {
                                ctx.checked.sync_controlled(Some(value));
                            } else {
                                ctx.checked.sync_controlled(None);
                            }
                        },
                    ))
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.checked.sync_controlled(None);
                    }))
                }
            }

            (_, Event::SetProps) => {
                let disabled = props.disabled;
                let required = props.required;
                let invalid = props.invalid;
                let readonly = props.readonly;
                let name = props.name.clone();
                let form = props.form.clone();
                let value = props.value.clone();
                let label = props.label.clone();
                let dir = props.dir;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                    ctx.required = required;
                    ctx.invalid = invalid;
                    ctx.readonly = readonly;
                    ctx.name = name;
                    ctx.form = form;
                    ctx.value = value;
                    ctx.label = label;
                    ctx.dir = dir;
                }))
            }

            (_, Event::SetHasDescription(has_description)) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
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
            "switch::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.checked != new.checked {
            events.push(Event::SetValue(new.checked));
        }

        if old.disabled != new.disabled
            || old.required != new.required
            || old.invalid != new.invalid
            || old.readonly != new.readonly
            || old.name != new.name
            || old.form != new.form
            || old.value != new.value
            || old.label != new.label
            || old.dir != new.dir
        {
            events.push(Event::SetProps);
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

/// Structural parts exposed by the switch connect API.
#[derive(ComponentPart)]
#[scope = "switch"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The interactive switch control element.
    Control,

    /// The visual thumb indicator.
    Thumb,

    /// The hidden native input used for form submission.
    HiddenInput,

    /// The optional descriptive text element.
    Description,

    /// The validation error message element.
    ErrorMessage,
}

/// API for the `Switch` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Returns true when the switch is in its on state.
    #[must_use]
    pub fn is_checked(&self) -> bool {
        *self.ctx.checked.get()
    }

    /// Returns attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.data_state())
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for the visible label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        if !self.ctx.readonly {
            attrs.set(HtmlAttr::For, self.ctx.ids.part("hidden-input"));
        }

        attrs
    }

    /// Returns attributes for the interactive switch control element.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("control"))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Role, "switch")
            .set(
                HtmlAttr::Aria(AriaAttr::Checked),
                if self.is_checked() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(HtmlAttr::Data("ars-state"), self.data_state());

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        let mut described_by = Vec::new();

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.invalid {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for the visual thumb indicator.
    #[must_use]
    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.data_state())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns attributes for the hidden native input used for form submission.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("hidden-input"))
            .set(HtmlAttr::Type, "checkbox")
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Value, self.ctx.value.clone());

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.ctx.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        if self.is_checked() {
            attrs.set_bool(HtmlAttr::Checked, true);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        attrs
    }

    /// Returns attributes for the description/help text element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Returns attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }

    /// Sends [`Event::Toggle`] for a control click.
    pub fn on_control_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Sends [`Event::Focus`] for control focus.
    pub fn on_control_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends [`Event::Blur`] for control blur.
    pub fn on_control_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::TurnOn`] or [`Event::TurnOff`] for hidden input changes.
    pub fn on_hidden_input_change(&self, checked: bool) {
        (self.send)(if checked {
            Event::TurnOn
        } else {
            Event::TurnOff
        });
    }

    /// Sends [`Event::Reset`] for a native form reset.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
    }

    fn data_state(&self) -> &'static str {
        if self.is_checked() {
            "checked"
        } else {
            "unchecked"
        }
    }
}

fn value_change_plan(ctx: &Context, next: bool) -> TransitionPlan<Machine> {
    if *ctx.checked.get() == next {
        return TransitionPlan::context_only(|_: &mut Context| {});
    }

    if ctx.checked.is_controlled() {
        return TransitionPlan::new()
            .apply(|_: &mut Context| {})
            .with_effect(checked_change_effect(next));
    }

    TransitionPlan::to(state_from_checked(next))
        .apply(move |ctx: &mut Context| {
            ctx.checked.set(next);
        })
        .with_effect(checked_change_effect(next))
}

fn reset_plan(ctx: &Context, default_checked: bool) -> TransitionPlan<Machine> {
    if *ctx.checked.get() == default_checked {
        return TransitionPlan::new();
    }

    if ctx.checked.is_controlled() {
        return value_change_plan(ctx, default_checked);
    }

    TransitionPlan::to(state_from_checked(default_checked)).apply(move |ctx: &mut Context| {
        ctx.checked.set(default_checked);

        ctx.checked.sync_controlled(None);
    })
}

fn checked_change_effect(next: bool) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::CheckedChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(cb) = &props.on_checked_change {
                cb(next);
            }

            no_cleanup()
        },
    )
}

const fn state_from_checked(checked: bool) -> State {
    if checked { State::On } else { State::Off }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, string::ToString, sync::Arc, vec};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_core::{AriaAttr, ConnectApi, Direction, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "wifi".to_string(),
            ..Props::default()
        }
    }

    fn form_props() -> Props {
        Props {
            name: Some("wifi_enabled".to_string()),
            form: Some("settings".to_string()),
            value: "enabled".to_string(),
            ..test_props()
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn switch_initial_state_is_off() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert!(!service.context().disabled);
        assert!(!service.context().required);
        assert!(!service.context().invalid);
        assert!(!service.context().readonly);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
        assert!(!service.context().has_description);
        assert_eq!(service.context().ids.id(), "wifi");
        assert_eq!(service.context().ids.part("control"), "wifi-control");
        assert_eq!(service.context().value, "on");
        assert_eq!(service.context().dir, Direction::Ltr);
    }

    #[test]
    fn switch_default_checked_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                default_checked: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
    }

    #[test]
    fn switch_controlled_checked_initializes_on() {
        let service = Service::<Machine>::new(
            Props {
                checked: Some(true),
                default_checked: false,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(service.context().checked.is_controlled());
    }

    #[test]
    fn switch_toggle_cycles_off_on_off() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());

        let result = service.send(Event::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
    }

    #[test]
    fn switch_turn_on_and_turn_off_are_idempotent() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::TurnOff);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Off);

        drop(service.send(Event::TurnOn));

        let result = service.send(Event::TurnOn);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::On);
    }

    #[test]
    fn switch_disabled_guard_prevents_value_transitions() {
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
        assert!(!service.context().checked.get());
    }

    #[test]
    fn switch_readonly_guard_prevents_value_transitions() {
        let mut service = Service::<Machine>::new(
            Props {
                readonly: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert!(!service.send(Event::Toggle).state_changed);
        assert!(!service.send(Event::TurnOn).state_changed);
        assert!(!service.send(Event::TurnOff).state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
    }

    #[test]
    fn switch_controlled_user_toggle_emits_change_without_committing_state() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Toggle);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::CheckedChange);
    }

    #[test]
    fn switch_user_toggle_runs_checked_change_callback() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let captured_changes = Arc::clone(&changes);
        let mut service = Service::<Machine>::new(
            Props {
                on_checked_change: Some(callback(move |checked: bool| {
                    captured_changes.lock().unwrap().push(checked);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let mut result = service.send(Event::Toggle);

        let effect = result.pending_effects.pop().expect("checked-change effect");
        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().as_slice(), &[true]);
    }

    #[test]
    fn switch_checked_change_effect_is_noop_without_callback() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let mut result = service.send(Event::Toggle);

        let effect = result.pending_effects.pop().expect("checked-change effect");
        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(service.state(), &State::On);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn switch_set_value_some_syncs_controlled_state_and_context() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::SetValue(Some(true)));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(service.context().checked.is_controlled());
    }

    #[test]
    fn switch_set_value_some_preserves_uncontrolled_mode() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::SetValue(Some(true))));

        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(!service.context().checked.is_controlled());

        let result = service.send(Event::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
    }

    #[test]
    fn switch_set_value_none_switches_to_uncontrolled_without_stale_state() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());

        drop(service.send(Event::SetValue(None)));

        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert!(!service.context().checked.is_controlled());
    }

    #[test]
    fn switch_reset_restores_default_checked_without_change_effect() {
        let mut service = Service::<Machine>::new(
            Props {
                default_checked: true,
                on_checked_change: Some(callback(|_: bool| {
                    panic!("reset must not emit checked-change");
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::TurnOff));

        let result = service.send(Event::Reset);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn switch_controlled_reset_requests_default_checked_without_committing_state() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                default_checked: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Reset);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Off);
        assert!(!service.context().checked.get());
        assert!(service.context().checked.is_controlled());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::CheckedChange);
    }

    #[test]
    fn switch_controlled_reset_callback_receives_default_checked() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let captured_changes = Arc::clone(&changes);
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(false),
                default_checked: true,
                on_checked_change: Some(callback(move |checked: bool| {
                    captured_changes.lock().unwrap().push(checked);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let mut result = service.send(Event::Reset);

        let effect = result.pending_effects.pop().expect("checked-change effect");
        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().as_slice(), &[true]);
    }

    #[test]
    fn switch_uncontrolled_reset_is_noop_when_already_at_default() {
        let mut service = Service::<Machine>::new(
            Props {
                default_checked: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Reset);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
    }

    #[test]
    fn switch_controlled_reset_is_noop_when_already_at_default() {
        let mut service = Service::<Machine>::new(
            Props {
                checked: Some(true),
                default_checked: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Reset);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
        assert!(service.context().checked.is_controlled());
    }

    #[test]
    fn switch_reset_runs_even_when_disabled_or_readonly() {
        let mut service = Service::<Machine>::new(
            Props {
                default_checked: true,
                disabled: true,
                readonly: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetValue(Some(false))));

        let result = service.send(Event::Reset);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::On);
        assert!(*service.context().checked.get());
    }

    #[test]
    fn switch_focus_and_blur_update_focus_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn switch_set_has_description_controls_describedby() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert!(
            !service
                .connect(&|_| {})
                .control_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::DescribedBy))
        );

        drop(service.send(Event::SetHasDescription(true)));

        assert_eq!(
            service
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("wifi-description")
        );
    }

    #[test]
    fn switch_on_props_changed_emits_expected_sync_events() {
        let old = test_props();
        let new = Props {
            checked: Some(true),
            disabled: true,
            required: true,
            invalid: true,
            readonly: true,
            name: Some("wifi".to_string()),
            form: Some("settings".to_string()),
            value: "enabled".to_string(),
            label: Some("Wi-Fi".to_string()),
            dir: Direction::Rtl,
            ..test_props()
        };

        let events = <Machine as ars_core::Machine>::on_props_changed(&old, &new);

        assert_eq!(events, vec![Event::SetValue(Some(true)), Event::SetProps]);
    }

    #[test]
    fn switch_on_props_changed_emits_set_props_for_each_output_prop() {
        let old = test_props();
        let cases = [
            Props {
                disabled: true,
                ..test_props()
            },
            Props {
                required: true,
                ..test_props()
            },
            Props {
                invalid: true,
                ..test_props()
            },
            Props {
                readonly: true,
                ..test_props()
            },
            Props {
                name: Some("wifi".to_string()),
                ..test_props()
            },
            Props {
                form: Some("settings".to_string()),
                ..test_props()
            },
            Props {
                value: "enabled".to_string(),
                ..test_props()
            },
            Props {
                label: Some("Wi-Fi".to_string()),
                ..test_props()
            },
            Props {
                dir: Direction::Rtl,
                ..test_props()
            },
        ];

        for new in cases {
            let events = <Machine as ars_core::Machine>::on_props_changed(&old, &new);

            assert_eq!(events, vec![Event::SetProps], "{new:?}");
        }
    }

    #[test]
    fn switch_on_props_changed_no_changes_emits_no_events() {
        let props = test_props();

        let events = <Machine as ars_core::Machine>::on_props_changed(&props, &props);

        assert!(events.is_empty());
    }

    #[test]
    #[should_panic(expected = "switch::Props.id must remain stable after init")]
    fn switch_set_props_panics_when_id_changes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let next = Props {
            id: "other".to_string(),
            ..test_props()
        };

        drop(service.set_props(next));
    }

    #[test]
    fn switch_set_props_syncs_context_fields() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props {
            disabled: true,
            required: true,
            invalid: true,
            readonly: true,
            name: Some("wifi".to_string()),
            form: Some("settings".to_string()),
            value: "enabled".to_string(),
            label: Some("Wi-Fi".to_string()),
            dir: Direction::Rtl,
            ..test_props()
        }));

        assert!(service.context().disabled);
        assert!(service.context().required);
        assert!(service.context().invalid);
        assert!(service.context().readonly);
        assert_eq!(service.context().name.as_deref(), Some("wifi"));
        assert_eq!(service.context().form.as_deref(), Some("settings"));
        assert_eq!(service.context().value, "enabled");
        assert_eq!(service.context().label.as_deref(), Some("Wi-Fi"));
        assert_eq!(service.context().dir, Direction::Rtl);
    }

    #[test]
    fn switch_control_attrs_emit_role_and_aria_checked() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_eq!(
            service
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Role),
            Some("switch")
        );
        assert_eq!(
            service
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Type),
            Some("button")
        );
        assert_eq!(
            service
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("false")
        );

        drop(service.send(Event::TurnOn));

        assert_eq!(
            service
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
    }

    #[test]
    fn switch_hidden_input_reflects_form_value_and_checked_state() {
        let mut service = Service::<Machine>::new(form_props(), &Env::default(), &Messages);

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("wifi-hidden-input"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("checkbox"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("wifi_enabled"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("settings"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("enabled"));
        assert!(!attrs.contains(&HtmlAttr::Checked));

        drop(service.send(Event::TurnOn));

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Checked), Some("true"));
    }

    #[test]
    fn switch_hidden_input_emits_required_and_disabled() {
        let service = Service::<Machine>::new(
            Props {
                disabled: true,
                required: true,
                ..form_props()
            },
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Required), Some("true"));
    }

    #[test]
    fn switch_hidden_input_stays_enabled_when_readonly() {
        let service = Service::<Machine>::new(
            Props {
                readonly: true,
                ..form_props()
            },
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert!(!attrs.contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn switch_label_targets_hidden_native_input() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let attrs = service.connect(&|_| {}).label_attrs();

        assert_eq!(attrs.get(&HtmlAttr::For), Some("wifi-hidden-input"));
    }

    #[test]
    fn switch_label_omits_for_when_readonly() {
        let service = Service::<Machine>::new(
            Props {
                readonly: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).label_attrs();

        assert!(!attrs.contains(&HtmlAttr::For));
    }

    #[test]
    fn switch_part_attrs_delegate_for_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Control), api.control_attrs());
        assert_eq!(api.part_attrs(Part::Thumb), api.thumb_attrs());
        assert_eq!(api.part_attrs(Part::HiddenInput), api.hidden_input_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
    }

    #[test]
    fn switch_event_helpers_send_expected_events() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);
        let events = Rc::new(RefCell::new(Vec::new()));
        let sent = Rc::clone(&events);
        let send = move |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_control_click();
        api.on_control_focus(true);
        api.on_control_blur();
        api.on_hidden_input_change(true);
        api.on_hidden_input_change(false);
        api.on_form_reset();

        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::Toggle,
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::TurnOn,
                Event::TurnOff,
                Event::Reset,
            ]
        );
    }

    #[test]
    fn switch_api_debug_is_stable() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        let debug = format!("{api:?}");

        assert!(debug.contains("Api"));
        assert!(debug.contains("wifi"));
        assert!(debug.contains("Context"));
        assert!(debug.contains("<callback>"));
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("switch-1")
            .checked(true)
            .default_checked(true)
            .disabled(true)
            .required(true)
            .invalid(true)
            .readonly(true)
            .name("wifi")
            .form("settings")
            .value("enabled")
            .label("Wi-Fi")
            .dir(Direction::Rtl)
            .on_checked_change(|_| {});

        assert_eq!(props.id, "switch-1");
        assert_eq!(props.checked, Some(true));
        assert!(props.default_checked);
        assert!(props.disabled);
        assert!(props.required);
        assert!(props.invalid);
        assert!(props.readonly);
        assert_eq!(props.name.as_deref(), Some("wifi"));
        assert_eq!(props.form.as_deref(), Some("settings"));
        assert_eq!(props.value, "enabled");
        assert_eq!(props.label.as_deref(), Some("Wi-Fi"));
        assert_eq!(props.dir, Direction::Rtl);
        assert!(props.on_checked_change.is_some());
    }

    #[test]
    fn props_builder_can_clear_optional_control_fields() {
        let props = Props::new()
            .checked(true)
            .name("wifi")
            .form("settings")
            .label("Wi-Fi")
            .on_checked_change(|_| {})
            .uncontrolled()
            .no_name()
            .no_form()
            .no_label()
            .no_checked_change();

        assert_eq!(props.checked, None);
        assert_eq!(props.name, None);
        assert_eq!(props.form, None);
        assert_eq!(props.label, None);
        assert!(props.on_checked_change.is_none());
    }

    #[test]
    fn switch_root_off_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_root_off",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn switch_root_on_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::TurnOn));

        assert_snapshot!(
            "switch_root_on",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn switch_root_disabled_invalid_readonly_focus_visible_snapshot() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                invalid: true,
                readonly: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "switch_root_disabled_invalid_readonly_focus_visible",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn switch_root_rtl_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                dir: Direction::Rtl,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "switch_root_rtl",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn switch_label_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_label_default",
            snapshot_attrs(&service.connect(&|_| {}).label_attrs())
        );
    }

    #[test]
    fn switch_control_off_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_control_off",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_on_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::TurnOn));

        assert_snapshot!(
            "switch_control_on",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_required_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                required: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "switch_control_required",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_disabled_readonly_invalid_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                disabled: true,
                readonly: true,
                invalid: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "switch_control_disabled_readonly_invalid",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_description_only_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::SetHasDescription(true)));

        assert_snapshot!(
            "switch_control_description_only",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_error_only_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                invalid: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "switch_control_error_only",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_description_and_error_snapshot() {
        let mut service = Service::<Machine>::new(
            Props {
                invalid: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetHasDescription(true)));

        assert_snapshot!(
            "switch_control_description_and_error",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_control_focus_visible_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "switch_control_focus_visible",
            snapshot_attrs(&service.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn switch_thumb_off_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_thumb_off",
            snapshot_attrs(&service.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn switch_thumb_on_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::TurnOn));

        assert_snapshot!(
            "switch_thumb_on",
            snapshot_attrs(&service.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn switch_hidden_input_off_snapshot() {
        let service = Service::<Machine>::new(form_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_hidden_input_off",
            snapshot_attrs(&service.connect(&|_| {}).hidden_input_attrs())
        );
    }

    #[test]
    fn switch_hidden_input_on_snapshot() {
        let mut service = Service::<Machine>::new(form_props(), &Env::default(), &Messages);

        drop(service.send(Event::TurnOn));

        assert_snapshot!(
            "switch_hidden_input_on",
            snapshot_attrs(&service.connect(&|_| {}).hidden_input_attrs())
        );
    }

    #[test]
    fn switch_hidden_input_disabled_required_snapshot() {
        let service = Service::<Machine>::new(
            Props {
                disabled: true,
                required: true,
                ..form_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "switch_hidden_input_disabled_required",
            snapshot_attrs(&service.connect(&|_| {}).hidden_input_attrs())
        );
    }

    #[test]
    fn switch_description_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_description_default",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn switch_error_message_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "switch_error_message_default",
            snapshot_attrs(&service.connect(&|_| {}).error_message_attrs())
        );
    }
}
