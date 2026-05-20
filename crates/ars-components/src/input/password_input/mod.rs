//! PasswordInput component state machine and connect API.
//!
//! This module implements the framework-agnostic `PasswordInput` machine defined
//! in `spec/components/input/password-input.md`. It extends a single native
//! `<input>` with a visibility toggle that flips between `type="password"` and
//! `type="text"`. The native input participates in form submission directly; no
//! hidden input is emitted.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, Locale, MessageFn, NoEffect,
};

/// The states for the `PasswordInput` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Password is hidden — the input renders as `type="password"`.
    Masked,

    /// Password is visible — the input renders as `type="text"`.
    Visible,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Masked => "masked",
            Self::Visible => "visible",
        })
    }
}

/// The events for the `PasswordInput` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Flip between [`State::Masked`] and [`State::Visible`].
    ToggleVisibility,

    /// Explicitly set visibility to the given value.
    SetVisibility(bool),

    /// The component received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// The component lost focus.
    Blur,

    /// Synchronize the externally controlled value prop.
    ///
    /// `Some` switches the component to controlled mode and pushes the new
    /// value; `None` returns the component to uncontrolled mode.
    SetValue(Option<String>),

    /// Synchronize output-affecting props (disabled / readonly / invalid /
    /// required / placeholder / name / form / autocomplete) stored in
    /// [`Context`] when [`Service::set_props`] reports a change.
    SetProps,

    /// Track whether a [`Part::Description`] part is rendered (gates
    /// `aria-describedby`).
    SetHasDescription(bool),
}

/// The context for the `PasswordInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the password is currently visible.
    pub visible: bool,

    /// The controlled/uncontrolled value of the component.
    pub value: Bindable<String>,

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

    /// Whether the focus is visible (keyboard-initiated).
    pub focus_visible: bool,

    /// Whether a Description part is rendered (gates `aria-describedby`).
    pub has_description: bool,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The props for the `PasswordInput` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the password input root.
    pub id: String,

    /// Controlled value. When `Some`, component is controlled.
    pub value: Option<String>,

    /// Default value for uncontrolled mode.
    pub default_value: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the password is visible on initial render.
    pub default_visible: bool,

    /// The placeholder of the component.
    pub placeholder: Option<String>,

    /// The `name` attribute used for form submission.
    pub name: Option<String>,

    /// The ID of the form element the input is associated with.
    pub form: Option<String>,

    /// Autocomplete hint for password managers.
    ///
    /// Defaults to `"current-password"` when unset.
    pub autocomplete: Option<String>,
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the adapter-provided base ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), switching to controlled mode.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Clears [`value`](Self::value), switching to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = value.into();
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

    /// Sets [`default_visible`](Self::default_visible).
    #[must_use]
    pub const fn default_visible(mut self, value: bool) -> Self {
        self.default_visible = value;
        self
    }

    /// Sets [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Clears [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn no_placeholder(mut self) -> Self {
        self.placeholder = None;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Clears [`name`](Self::name).
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, value: impl Into<String>) -> Self {
        self.form = Some(value.into());
        self
    }

    /// Clears [`form`](Self::form).
    #[must_use]
    pub fn no_form(mut self) -> Self {
        self.form = None;
        self
    }

    /// Sets [`autocomplete`](Self::autocomplete).
    #[must_use]
    pub fn autocomplete(mut self, value: impl Into<String>) -> Self {
        self.autocomplete = Some(value.into());
        self
    }

    /// Clears [`autocomplete`](Self::autocomplete) so the input falls back to
    /// the default `"current-password"` hint.
    #[must_use]
    pub fn no_autocomplete(mut self) -> Self {
        self.autocomplete = None;
        self
    }
}

/// Locale-specific labels for the `PasswordInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label used on the toggle button when the password is masked.
    pub show_password_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label used on the toggle button when the password is visible.
    pub hide_password_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            show_password_label: MessageFn::static_str("Show password"),
            hide_password_label: MessageFn::static_str("Hide password"),
        }
    }
}

impl ComponentMessages for Messages {}

/// The machine for the `PasswordInput` component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let state = if props.default_visible {
            State::Visible
        } else {
            State::Masked
        };

        (
            state,
            Context {
                visible: props.default_visible,
                value: if let Some(value) = &props.value {
                    Bindable::controlled(value.clone())
                } else {
                    Bindable::uncontrolled(props.default_value.clone())
                },
                disabled: props.disabled,
                required: props.required,
                invalid: props.invalid,
                readonly: props.readonly,
                focused: false,
                focus_visible: false,
                has_description: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        _state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<ars_core::TransitionPlan<Self>> {
        match event {
            Event::ToggleVisibility => {
                if ctx.disabled {
                    return None;
                }

                let next_visible = !ctx.visible;

                let target = if next_visible {
                    State::Visible
                } else {
                    State::Masked
                };

                Some(
                    ars_core::TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                        ctx.visible = next_visible;
                    }),
                )
            }

            Event::SetVisibility(visible) => {
                if ctx.disabled || ctx.visible == *visible {
                    return None;
                }

                let visible = *visible;

                let target = if visible {
                    State::Visible
                } else {
                    State::Masked
                };

                Some(
                    ars_core::TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                        ctx.visible = visible;
                    }),
                )
            }

            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(ars_core::TransitionPlan::context_only(
                    move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    },
                ))
            }

            Event::Blur => Some(ars_core::TransitionPlan::context_only(
                |ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                },
            )),

            Event::SetValue(value) => {
                let value = value.clone();
                Some(ars_core::TransitionPlan::context_only(
                    move |ctx: &mut Context| {
                        if let Some(value) = value {
                            ctx.value.set(value.clone());
                            ctx.value.sync_controlled(Some(value));
                        } else {
                            ctx.value.sync_controlled(None);
                        }
                    },
                ))
            }

            Event::SetProps => {
                let props = props.clone();
                Some(ars_core::TransitionPlan::context_only(
                    move |ctx: &mut Context| {
                        ctx.disabled = props.disabled;
                        ctx.required = props.required;
                        ctx.invalid = props.invalid;
                        ctx.readonly = props.readonly;
                    },
                ))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(ars_core::TransitionPlan::context_only(
                    move |ctx: &mut Context| {
                        ctx.has_description = has_description;
                    },
                ))
            }
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "password_input::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// Structural parts exposed by the `PasswordInput` connect API.
#[derive(ComponentPart)]
#[scope = "password-input"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The native password input element.
    Input,

    /// The visibility toggle button.
    Toggle,

    /// Optional descriptive help-text element.
    Description,

    /// Optional validation error message element.
    ErrorMessage,
}

/// The API for the `PasswordInput` component.
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

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::Toggle => self.toggle_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string());

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

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Attributes for the native password input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(
                HtmlAttr::Type,
                if self.ctx.visible { "text" } else { "password" },
            )
            .set(HtmlAttr::Value, self.ctx.value.get().clone())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set(
                HtmlAttr::AutoComplete,
                self.props
                    .autocomplete
                    .clone()
                    .unwrap_or_else(|| "current-password".to_string()),
            );

        set_described_by(&mut attrs, self.ctx);

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ErrorMessage),
                self.ctx.ids.part("error-message"),
            );
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if let Some(placeholder) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.clone());
        }

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        attrs
    }

    /// Attributes for the visibility toggle button.
    #[must_use]
    pub fn toggle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Toggle.data_attrs();

        let label = if self.ctx.visible {
            (self.ctx.messages.hide_password_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.show_password_label)(&self.ctx.locale)
        };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "0")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the description/help text element.
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

    /// Attributes for the validation error message element.
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

    /// Sends [`Event::Focus`] for input focus.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends [`Event::Blur`] for input blur.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::ToggleVisibility`] for toggle activation.
    pub fn on_toggle_click(&self) {
        (self.send)(Event::ToggleVisibility);
    }
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.required != new.required
        || old.invalid != new.invalid
        || old.readonly != new.readonly
        || old.placeholder != new.placeholder
        || old.name != new.name
        || old.form != new.form
        || old.autocomplete != new.autocomplete
}

fn set_described_by(attrs: &mut AttrMap, ctx: &Context) {
    let mut described_by = Vec::new();

    if ctx.has_description {
        described_by.push(ctx.ids.part("description"));
    }

    if ctx.invalid {
        described_by.push(ctx.ids.part("error-message"));
    }

    if !described_by.is_empty() {
        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            described_by.join(" "),
        );
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{ConnectApi, Env, HtmlAttr, Service};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("pwd")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn password_input_initial_state_is_masked() {
        let service = service(props().default_value("hunter2"));

        assert_eq!(service.state(), &State::Masked);
        assert!(!service.context().visible);
        assert_eq!(service.context().value.get(), "hunter2");
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
        assert_eq!(service.context().ids.part("input"), "pwd-input");
    }

    #[test]
    fn password_input_initial_state_respects_default_visible() {
        let service = service(props().default_visible(true).default_value("plaintext"));

        assert_eq!(service.state(), &State::Visible);
        assert!(service.context().visible);
    }

    #[test]
    fn password_input_toggle_visibility_flips_state() {
        let mut service = service(props());

        let first = service.send(Event::ToggleVisibility);

        assert!(first.state_changed);
        assert_eq!(service.state(), &State::Visible);
        assert!(service.context().visible);

        let second = service.send(Event::ToggleVisibility);

        assert!(second.state_changed);
        assert_eq!(service.state(), &State::Masked);
        assert!(!service.context().visible);
    }

    #[test]
    fn password_input_set_visibility_to_false_returns_to_masked() {
        let mut service = service(props().default_visible(true));

        let result = service.send(Event::SetVisibility(false));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Masked);
        assert!(!service.context().visible);
    }

    #[test]
    fn password_input_props_builder_clearers_round_trip() {
        let p = Props::new()
            .id("p")
            .value("controlled")
            .uncontrolled()
            .placeholder("placeholder")
            .no_placeholder()
            .name("name")
            .no_name()
            .form("form")
            .no_form()
            .autocomplete("custom")
            .no_autocomplete();

        assert_eq!(p.id, "p");
        assert_eq!(p.value, None);
        assert_eq!(p.placeholder, None);
        assert_eq!(p.name, None);
        assert_eq!(p.form, None);
        assert_eq!(p.autocomplete, None);
    }

    #[test]
    fn password_input_api_debug_does_not_leak_send_closure() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        let formatted = alloc::format!("{api:?}");

        assert!(formatted.contains("<callback>"));
        assert!(formatted.contains("Masked"));
    }

    #[test]
    fn password_input_focus_visible_emits_data_attr_on_root() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus { is_keyboard: true }));

        let api = svc.connect(&|_| {});

        let attrs = api.root_attrs();

        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    #[test]
    fn password_input_described_by_includes_description_when_set_has_description_true() {
        let mut svc = service(props());

        drop(svc.send(Event::SetHasDescription(true)));

        let api = svc.connect(&|_| {});
        let attrs = api.input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("pwd-description")
        );
    }

    #[test]
    fn password_input_set_visibility_is_idempotent() {
        let mut service = service(props());

        let noop = service.send(Event::SetVisibility(false));

        assert!(!noop.state_changed);
        assert!(!noop.context_changed);
        assert_eq!(service.state(), &State::Masked);

        let change = service.send(Event::SetVisibility(true));

        assert!(change.state_changed);
        assert_eq!(service.state(), &State::Visible);
    }

    #[test]
    fn password_input_disabled_blocks_visibility_changes() {
        let mut service = service(props().disabled(true));

        let toggle = service.send(Event::ToggleVisibility);

        let set = service.send(Event::SetVisibility(true));

        assert!(!toggle.state_changed);
        assert!(!set.state_changed);
        assert_eq!(service.state(), &State::Masked);
        assert!(!service.context().visible);
    }

    #[test]
    fn password_input_focus_tracks_focus_visible() {
        let mut service = service(props());

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);

        drop(service.send(Event::Focus { is_keyboard: false }));

        assert!(service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn password_input_input_attrs_carries_type_password_when_masked() {
        let service = service(props().default_value("secret"));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("password"));
        assert_eq!(attrs.get(&HtmlAttr::AutoComplete), Some("current-password"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("secret"));
    }

    #[test]
    fn password_input_input_attrs_carries_type_text_when_visible() {
        let mut service = service(props().default_value("secret"));

        drop(service.send(Event::ToggleVisibility));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("text"));
    }

    #[test]
    fn password_input_input_attrs_supports_custom_autocomplete() {
        let service = service(props().autocomplete("new-password"));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::AutoComplete), Some("new-password"));
    }

    #[test]
    fn password_input_toggle_attrs_uses_show_label_when_masked() {
        let service = service(props());

        let api = service.connect(&|_| {});

        let attrs = api.toggle_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Show password")
        );
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn password_input_toggle_attrs_uses_hide_label_when_visible() {
        let mut service = service(props());

        drop(service.send(Event::ToggleVisibility));

        let api = service.connect(&|_| {});

        let attrs = api.toggle_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Hide password")
        );
    }

    #[test]
    fn password_input_invalid_drives_describedby_and_error_message() {
        let service = service(props().invalid(true));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("pwd-error-message")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("pwd-error-message")
        );
    }

    #[test]
    fn password_input_required_sets_aria_required() {
        let service = service(props().required(true));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
    }

    #[test]
    fn password_input_input_carries_name_form_and_placeholder() {
        let service = service(
            props()
                .placeholder("Password")
                .name("password")
                .form("login")
                .default_value("init"),
        );

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Placeholder), Some("Password"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("password"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("login"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("pwd-label")
        );
    }

    #[test]
    fn password_input_disabled_root_carries_data_attrs() {
        let service = service(
            props()
                .disabled(true)
                .invalid(true)
                .readonly(true)
                .default_value("x"),
        );

        let api = service.connect(&|_| {});

        let root = api.root_attrs();

        let input = api.input_attrs();

        let toggle = api.toggle_attrs();

        assert!(root.contains(&HtmlAttr::Data("ars-disabled")));
        assert!(root.contains(&HtmlAttr::Data("ars-invalid")));
        assert!(root.contains(&HtmlAttr::Data("ars-readonly")));
        assert!(input.contains(&HtmlAttr::Disabled));
        assert!(input.contains(&HtmlAttr::ReadOnly));
        assert!(toggle.contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn password_input_controlled_value_is_read_from_props() {
        let service = service(props().value("from-parent"));

        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), "from-parent");
    }

    #[test]
    fn password_input_set_props_syncs_controlled_value() {
        let mut service = service(props().value("initial"));

        assert_eq!(service.context().value.get(), "initial");

        drop(service.set_props(props().value("updated")));

        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), "updated");

        drop(service.set_props(props().uncontrolled()));

        assert!(!service.context().value.is_controlled());
    }

    #[test]
    fn password_input_set_props_syncs_disabled_invalid_readonly_required() {
        let mut service = service(props());

        assert!(!service.context().disabled);

        drop(
            service.set_props(
                props()
                    .disabled(true)
                    .invalid(true)
                    .readonly(true)
                    .required(true),
            ),
        );

        assert!(service.context().disabled);
        assert!(service.context().invalid);
        assert!(service.context().readonly);
        assert!(service.context().required);
    }

    #[test]
    fn password_input_set_has_description_flips_context_flag() {
        let mut service = service(props());

        assert!(!service.context().has_description);

        drop(service.send(Event::SetHasDescription(true)));

        assert!(service.context().has_description);

        let api = service.connect(&|_| {});
        let attrs = api.input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("pwd-description")
        );
    }

    #[test]
    fn password_input_props_output_changed_covers_each_field() {
        let base = props();

        assert!(!props_output_changed(&base, &base.clone()));

        for next in [
            base.clone().disabled(true),
            base.clone().required(true),
            base.clone().invalid(true),
            base.clone().readonly(true),
            base.clone().placeholder("p"),
            base.clone().name("n"),
            base.clone().form("f"),
            base.clone().autocomplete("a"),
        ] {
            assert!(props_output_changed(&base, &next));
        }
    }

    #[test]
    fn password_input_part_attrs_delegates_to_each_part_method() {
        let service = service(props().default_value("v"));

        let api = service.connect(&|_| {});

        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::Root)),
            snapshot_attrs(&api.root_attrs()),
        );
        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::Label)),
            snapshot_attrs(&api.label_attrs()),
        );
        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::Input)),
            snapshot_attrs(&api.input_attrs()),
        );
        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::Toggle)),
            snapshot_attrs(&api.toggle_attrs()),
        );
        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::Description)),
            snapshot_attrs(&api.description_attrs()),
        );
        assert_eq!(
            snapshot_attrs(&api.part_attrs(Part::ErrorMessage)),
            snapshot_attrs(&api.error_message_attrs()),
        );
    }

    #[test]
    fn password_input_event_handlers_fan_out_through_send() {
        let count = core::cell::Cell::new(0u8);
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
            count.set(count.get() + 1);
        };

        let service = service(props());

        let api = service.connect(&send);

        api.on_input_focus(true);
        api.on_input_blur();
        api.on_toggle_click();

        assert_eq!(count.get(), 3);

        let events = received.borrow();

        assert_eq!(events[0], Event::Focus { is_keyboard: true });
        assert_eq!(events[1], Event::Blur);
        assert_eq!(events[2], Event::ToggleVisibility);
    }

    #[test]
    fn password_input_root_masked_snapshot() {
        let service = service(props().default_value("x"));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn password_input_root_visible_snapshot() {
        let mut service = service(props().default_value("x"));

        drop(service.send(Event::ToggleVisibility));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn password_input_root_disabled_invalid_readonly_snapshot() {
        let service = service(
            props()
                .disabled(true)
                .invalid(true)
                .readonly(true)
                .default_value("x"),
        );

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn password_input_input_masked_snapshot() {
        let service = service(props().default_value("hunter2"));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn password_input_input_visible_snapshot() {
        let mut service = service(props().default_value("hunter2"));

        drop(service.send(Event::ToggleVisibility));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn password_input_input_with_metadata_snapshot() {
        let service = service(
            props()
                .placeholder("Password")
                .name("password")
                .form("login")
                .autocomplete("new-password")
                .required(true)
                .invalid(true)
                .default_value("seed"),
        );

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn password_input_toggle_masked_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.toggle_attrs()));
    }

    #[test]
    fn password_input_toggle_visible_snapshot() {
        let service = service(props().default_visible(true));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.toggle_attrs()));
    }

    #[test]
    fn password_input_description_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.description_attrs()));
    }

    #[test]
    fn password_input_error_message_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.error_message_attrs()));
    }
}
