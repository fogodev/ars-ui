//! TextField component state machine and connect API.
//!
//! This module implements the framework-agnostic `TextField` machine defined in
//! `spec/components/input/text-field.md`. The native input element is the form
//! participant; no hidden input is emitted for this component.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, InputMode, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan, no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The states for the `TextField` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an idle state.
    Idle,

    /// The component is in a focused state.
    Focused,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused => "focused",
        })
    }
}

/// The events for the `TextField` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The component received a focus event.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// The component received a blur event.
    Blur,

    /// The component received a change event.
    Change(String),

    /// The component received a clear event.
    Clear,

    /// The component received a set invalid event.
    SetInvalid(bool),

    /// IME composition started.
    CompositionStart,

    /// IME composition ended with the final committed value.
    CompositionEnd(String),

    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether a description part is rendered.
    SetHasDescription(bool),
}

/// The input type of the component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputType {
    /// Plain text input.
    #[default]
    Text,

    /// Password input.
    Password,

    /// Email-address input.
    Email,

    /// URL input.
    Url,

    /// Telephone-number input.
    Tel,

    /// Search input.
    Search,
}

impl InputType {
    /// Returns the native HTML input `type` token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Email => "email",
            Self::Url => "url",
            Self::Tel => "tel",
            Self::Search => "search",
        }
    }
}

impl Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The context for the `TextField` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled value of the component.
    pub value: Bindable<String>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether the component has focus-visible.
    pub focus_visible: bool,

    /// The placeholder of the component.
    pub placeholder: Option<String>,

    /// The input type of the component.
    pub input_type: InputType,

    /// The maximum length of the component.
    pub max_length: Option<u32>,

    /// The minimum length of the component.
    pub min_length: Option<u32>,

    /// The pattern of the component.
    pub pattern: Option<String>,

    /// The autocomplete of the component.
    pub autocomplete: Option<String>,

    /// The name of the component.
    pub name: Option<String>,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// Whether a Description part is rendered.
    pub has_description: bool,

    /// Text direction for RTL support.
    pub dir: Direction,

    /// Mobile on-screen keyboard layout hint.
    pub input_mode: Option<InputMode>,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The props for the `TextField` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the text field root.
    pub id: String,

    /// Controlled value. When `Some`, component is controlled.
    pub value: Option<String>,

    /// Default value for uncontrolled mode.
    pub default_value: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is required.
    pub required: bool,

    /// The placeholder of the component.
    pub placeholder: Option<String>,

    /// The input type of the component.
    pub input_type: InputType,

    /// The maximum length of the component.
    pub max_length: Option<u32>,

    /// The minimum length of the component.
    pub min_length: Option<u32>,

    /// The pattern of the component.
    pub pattern: Option<String>,

    /// The autocomplete of the component.
    pub autocomplete: Option<String>,

    /// The name of the component.
    pub name: Option<String>,

    /// The ID of the form element the input is associated with.
    pub form: Option<String>,

    /// Whether the component is clearable.
    pub clearable: bool,

    /// The direction of the component.
    pub dir: Direction,

    /// Hint for the virtual keyboard type on mobile devices.
    pub input_mode: Option<InputMode>,

    /// Convenience callback fired with `true` on Focus and `false` on Blur.
    pub on_focus_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback fired when user interaction requests a value change.
    pub on_value_change: Option<Callback<dyn Fn(String) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: String::new(),
            disabled: false,
            readonly: false,
            invalid: false,
            required: false,
            placeholder: None,
            input_type: InputType::Text,
            max_length: None,
            min_length: None,
            pattern: None,
            autocomplete: None,
            name: None,
            form: None,
            clearable: false,
            dir: Direction::Ltr,
            input_mode: None,
            on_focus_change: None,
            on_value_change: None,
        }
    }
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

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
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

    /// Sets [`input_type`](Self::input_type).
    #[must_use]
    pub const fn input_type(mut self, value: InputType) -> Self {
        self.input_type = value;
        self
    }

    /// Sets [`max_length`](Self::max_length).
    #[must_use]
    pub const fn max_length(mut self, value: u32) -> Self {
        self.max_length = Some(value);
        self
    }

    /// Clears [`max_length`](Self::max_length).
    #[must_use]
    pub const fn no_max_length(mut self) -> Self {
        self.max_length = None;
        self
    }

    /// Sets [`min_length`](Self::min_length).
    #[must_use]
    pub const fn min_length(mut self, value: u32) -> Self {
        self.min_length = Some(value);
        self
    }

    /// Clears [`min_length`](Self::min_length).
    #[must_use]
    pub const fn no_min_length(mut self) -> Self {
        self.min_length = None;
        self
    }

    /// Sets [`pattern`](Self::pattern).
    #[must_use]
    pub fn pattern(mut self, value: impl Into<String>) -> Self {
        self.pattern = Some(value.into());
        self
    }

    /// Clears [`pattern`](Self::pattern).
    #[must_use]
    pub fn no_pattern(mut self) -> Self {
        self.pattern = None;
        self
    }

    /// Sets [`autocomplete`](Self::autocomplete).
    #[must_use]
    pub fn autocomplete(mut self, value: impl Into<String>) -> Self {
        self.autocomplete = Some(value.into());
        self
    }

    /// Clears [`autocomplete`](Self::autocomplete).
    #[must_use]
    pub fn no_autocomplete(mut self) -> Self {
        self.autocomplete = None;
        self
    }

    /// Sets [`name`](Self::name), the form-submission field name.
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

    /// Sets [`form`](Self::form), the associated form element ID.
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

    /// Sets [`clearable`](Self::clearable).
    #[must_use]
    pub const fn clearable(mut self, value: bool) -> Self {
        self.clearable = value;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`input_mode`](Self::input_mode).
    #[must_use]
    pub const fn input_mode(mut self, value: InputMode) -> Self {
        self.input_mode = Some(value);
        self
    }

    /// Clears [`input_mode`](Self::input_mode).
    #[must_use]
    pub const fn no_input_mode(mut self) -> Self {
        self.input_mode = None;
        self
    }

    /// Sets [`on_focus_change`](Self::on_focus_change).
    #[must_use]
    pub fn on_focus_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(bool) + Send + Sync>>,
    ) -> Self {
        self.on_focus_change = Some(callback.into());
        self
    }

    /// Clears [`on_focus_change`](Self::on_focus_change).
    #[must_use]
    pub fn no_focus_change(mut self) -> Self {
        self.on_focus_change = None;
        self
    }

    /// Sets [`on_value_change`](Self::on_value_change).
    #[must_use]
    pub fn on_value_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(String) + Send + Sync>>,
    ) -> Self {
        self.on_value_change = Some(callback.into());
        self
    }

    /// Clears [`on_value_change`](Self::on_value_change).
    #[must_use]
    pub fn no_value_change(mut self) -> Self {
        self.on_value_change = None;
        self
    }
}

/// Locale-specific labels for the `TextField` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the clear button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            clear_label: MessageFn::static_str("Clear"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the `text_field` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes `Props::on_focus_change` with the new focus state.
    FocusChange,

    /// Adapter invokes `Props::on_value_change` with the new committed value.
    ValueChange,
}

/// The machine for the `TextField` component.
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
        (
            State::Idle,
            Context {
                value: if let Some(value) = &props.value {
                    Bindable::controlled(value.clone())
                } else {
                    Bindable::uncontrolled(props.default_value.clone())
                },
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                required: props.required,
                focused: false,
                focus_visible: false,
                placeholder: props.placeholder.clone(),
                input_type: props.input_type,
                max_length: props.max_length,
                min_length: props.min_length,
                pattern: props.pattern.clone(),
                autocomplete: props.autocomplete.clone(),
                name: props.name.clone(),
                is_composing: false,
                has_description: false,
                dir: props.dir,
                input_mode: props.input_mode,
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
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            ctx.focused = true;

                            ctx.focus_visible = is_keyboard;
                        })
                        .with_effect(focus_change_effect(true)),
                )
            }

            Event::Blur => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.focused = false;

                        ctx.focus_visible = false;
                    })
                    .with_effect(focus_change_effect(false)),
            ),

            Event::Change(value) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing {
                    return None;
                }

                let value = value.clone();
                Some(
                    TransitionPlan::context_only({
                        let value = value.clone();
                        move |ctx: &mut Context| {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(value);
                            }
                        }
                    })
                    .with_effect(value_change_effect(value)),
                )
            }

            Event::Clear => {
                if ctx.disabled || ctx.readonly {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(|ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(String::new());
                        }
                    })
                    .with_effect(value_change_effect(String::new())),
                )
            }

            Event::SetInvalid(invalid) => {
                let invalid = *invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.invalid = invalid;
                }))
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd(value) => {
                let value = value.clone();

                let should_change = !ctx.disabled && !ctx.readonly;

                let mut plan = TransitionPlan::context_only({
                    let value = value.clone();
                    move |ctx: &mut Context| {
                        ctx.is_composing = false;

                        if should_change && !ctx.value.is_controlled() {
                            ctx.value.set(value);
                        }
                    }
                });

                if should_change {
                    plan = plan.with_effect(value_change_effect(value));
                }

                Some(plan)
            }

            Event::SetValue(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(value) = value {
                        ctx.value.set(value.clone());

                        ctx.value.sync_controlled(Some(value));
                    } else {
                        ctx.value.sync_controlled(None);
                    }
                }))
            }

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.placeholder = props.placeholder;
                    ctx.input_type = props.input_type;
                    ctx.max_length = props.max_length;
                    ctx.min_length = props.min_length;
                    ctx.pattern = props.pattern;
                    ctx.autocomplete = props.autocomplete;
                    ctx.name = props.name;
                    ctx.dir = props.dir;
                    ctx.input_mode = props.input_mode;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "text_field::Props.id must remain stable after init"
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

/// Structural parts exposed by the `TextField` connect API.
#[derive(ComponentPart)]
#[scope = "text-field"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The native input element.
    Input,

    /// Optional leading decorative slot.
    StartDecorator,

    /// Optional trailing decorative slot.
    EndDecorator,

    /// Optional clear button.
    ClearTrigger,

    /// Optional descriptive text element.
    Description,

    /// Optional validation error message element.
    ErrorMessage,
}

/// The API for the `TextField` component.
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
            Part::StartDecorator => self.start_decorator_attrs(),
            Part::EndDecorator => self.end_decorator_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
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

    /// Attributes for the native input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Type, self.ctx.input_type.as_str())
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr())
            .set(HtmlAttr::Value, self.ctx.value.get().clone())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if let Some(input_mode) = self.resolved_input_mode() {
            attrs.set(HtmlAttr::InputMode, input_mode.as_str());
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if let Some(placeholder) = &self.ctx.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.clone());
        }

        if let Some(max_length) = self.ctx.max_length {
            attrs.set(HtmlAttr::MaxLength, max_length.to_string());
        }

        if let Some(min_length) = self.ctx.min_length {
            attrs.set(HtmlAttr::MinLength, min_length.to_string());
        }

        if let Some(pattern) = &self.ctx.pattern {
            attrs.set(HtmlAttr::Pattern, pattern.clone());
        }

        if let Some(autocomplete) = &self.ctx.autocomplete {
            attrs.set(HtmlAttr::AutoComplete, autocomplete.clone());
        }

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        set_described_by(&mut attrs, self.ctx);

        attrs
    }

    /// Attributes for the start decorator slot.
    #[must_use]
    pub fn start_decorator_attrs(&self) -> AttrMap {
        decorator_attrs(&Part::StartDecorator)
    }

    /// Attributes for the end decorator slot.
    #[must_use]
    pub fn end_decorator_attrs(&self) -> AttrMap {
        decorator_attrs(&Part::EndDecorator)
    }

    /// Attributes for the clear trigger button.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            );

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

    /// Attributes for the error message element.
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

    /// Sends [`Event::Change`] for input changes when no composition is active.
    pub fn on_input_change(&self, value: String) {
        if !self.ctx.is_composing {
            (self.send)(Event::Change(value));
        }
    }

    /// Sends [`Event::CompositionStart`] for IME composition start.
    pub fn on_input_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Sends composition end followed by the final committed value.
    pub fn on_input_composition_end(&self, final_value: String) {
        (self.send)(Event::CompositionEnd(final_value));
    }

    /// Sends [`Event::Clear`] for clear trigger activation.
    pub fn on_clear_click(&self) {
        (self.send)(Event::Clear);
    }

    /// Handles normalized input keydown data.
    ///
    /// Returns `true` when the key was handled by the core machine.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.key == KeyboardKey::Escape
            && !data.is_composing
            && self.props.clearable
            && !self.ctx.disabled
            && !self.ctx.readonly
        {
            (self.send)(Event::Clear);

            return true;
        }

        false
    }

    /// Returns whether adapters should render the optional clear trigger.
    #[must_use]
    pub fn should_render_clear_trigger(&self) -> bool {
        self.props.clearable
            && !self.ctx.disabled
            && !self.ctx.readonly
            && !self.ctx.value.get().is_empty()
    }

    fn resolved_input_mode(&self) -> Option<InputMode> {
        self.ctx.input_mode.or(match self.ctx.input_type {
            InputType::Text | InputType::Password => None,
            InputType::Email => Some(InputMode::Email),
            InputType::Url => Some(InputMode::Url),
            InputType::Tel => Some(InputMode::Tel),
            InputType::Search => Some(InputMode::Search),
        })
    }
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.required != new.required
        || old.placeholder != new.placeholder
        || old.input_type != new.input_type
        || old.max_length != new.max_length
        || old.min_length != new.min_length
        || old.pattern != new.pattern
        || old.autocomplete != new.autocomplete
        || old.name != new.name
        || old.form != new.form
        || old.clearable != new.clearable
        || old.dir != new.dir
        || old.input_mode != new.input_mode
}

fn focus_change_effect(focused: bool) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::FocusChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_focus_change {
                callback(focused);
            }

            no_cleanup()
        },
    )
}

fn value_change_effect(value: String) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_change {
                callback(value);
            }

            no_cleanup()
        },
    )
}

fn decorator_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_val)
        .set(part_attr, part_val)
        .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

    attrs
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
    use alloc::{string::ToString, sync::Arc};
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("name-field")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn text_field_initial_state_is_idle() {
        let service = service(props().default_value("Alice"));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), "Alice");
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
        assert!(!service.context().is_composing);
        assert_eq!(service.context().ids.part("input"), "name-field-input");
    }

    #[test]
    fn text_field_focus_and_blur_track_focus_visible() {
        let mut service = service(props());

        let focus = service.send(Event::Focus { is_keyboard: true });

        assert!(focus.state_changed);
        assert_eq!(service.state(), &State::Focused);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        let blur = service.send(Event::Blur);

        assert!(blur.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn text_field_change_updates_uncontrolled_value() {
        let mut service = service(props());

        drop(service.send(Event::Change("Ada".to_string())));

        assert_eq!(service.context().value.get(), "Ada");
    }

    #[test]
    fn text_field_change_noops_when_disabled_readonly_or_composing() {
        for (props, event) in [
            (
                props().disabled(true),
                Event::Change("disabled".to_string()),
            ),
            (
                props().readonly(true),
                Event::Change("readonly".to_string()),
            ),
        ] {
            let mut service = service(props.default_value("before"));

            let result = service.send(event);

            assert!(!result.context_changed);
            assert_eq!(service.context().value.get(), "before");
        }

        let mut service = service(props().default_value("before"));

        drop(service.send(Event::CompositionStart));

        let result = service.send(Event::Change("during".to_string()));

        assert!(!result.context_changed);
        assert_eq!(service.context().value.get(), "before");
    }

    #[test]
    fn text_field_controlled_value_syncs_from_props() {
        let mut service = service(props().value("parent"));

        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), "parent");

        drop(service.set_props(props().value("updated")));

        assert_eq!(service.context().value.get(), "updated");

        drop(service.set_props(props().uncontrolled()));

        assert!(!service.context().value.is_controlled());
    }

    #[test]
    fn text_field_set_props_syncs_output_affecting_context() {
        let mut service = service(
            props()
                .placeholder("Name")
                .max_length(40)
                .min_length(2)
                .pattern("[A-Za-z]+")
                .autocomplete("name")
                .name("name")
                .form("profile")
                .required(true)
                .readonly(true)
                .input_mode(InputMode::Text),
        );

        drop(
            service.set_props(
                props()
                    .disabled(true)
                    .invalid(true)
                    .placeholder("Handle")
                    .no_max_length()
                    .no_min_length()
                    .no_pattern()
                    .no_autocomplete()
                    .no_name()
                    .no_form()
                    .input_type(InputType::Search)
                    .dir(Direction::Rtl)
                    .no_input_mode(),
            ),
        );

        assert!(service.context().disabled);
        assert!(!service.context().readonly);
        assert!(service.context().invalid);
        assert!(!service.context().required);
        assert_eq!(service.context().placeholder.as_deref(), Some("Handle"));
        assert_eq!(service.context().input_type, InputType::Search);
        assert_eq!(service.context().max_length, None);
        assert_eq!(service.context().min_length, None);
        assert_eq!(service.context().pattern, None);
        assert_eq!(service.context().autocomplete, None);
        assert_eq!(service.context().name, None);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().input_mode, None);

        let attrs = service.connect(&|_| {}).input_attrs();

        assert!(attrs.contains(&HtmlAttr::Disabled));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Placeholder), Some("Handle"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("search"));
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Form), None);
    }

    #[test]
    fn text_field_props_output_changed_covers_each_field() {
        let base = props();

        assert!(!props_output_changed(&base, &base.clone()));

        for next in [
            base.clone().disabled(true),
            base.clone().readonly(true),
            base.clone().invalid(true),
            base.clone().required(true),
            base.clone().placeholder("Name"),
            base.clone().input_type(InputType::Search),
            base.clone().max_length(12),
            base.clone().min_length(2),
            base.clone().pattern("[a-z]+"),
            base.clone().autocomplete("name"),
            base.clone().name("name"),
            base.clone().form("form"),
            base.clone().clearable(true),
            base.clone().dir(Direction::Rtl),
            base.clone().input_mode(InputMode::Text),
        ] {
            assert!(props_output_changed(&base, &next));
        }
    }

    #[test]
    fn text_field_builder_clearers_are_covered() {
        let props = props()
            .placeholder("Name")
            .no_placeholder()
            .on_focus_change(callback(|_: bool| {}))
            .no_focus_change()
            .on_value_change(callback(|_: String| {}))
            .no_value_change();

        assert_eq!(props.placeholder, None);
        assert_eq!(props.on_focus_change, None);
        assert_eq!(props.on_value_change, None);
    }

    #[test]
    fn text_field_clear_respects_disabled_and_readonly() {
        let mut clearable_service = service(props().default_value("before"));

        drop(clearable_service.send(Event::Clear));

        assert_eq!(clearable_service.context().value.get(), "");

        for props in [props().disabled(true), props().readonly(true)] {
            let mut service = service(props.default_value("before"));

            let result = service.send(Event::Clear);

            assert!(!result.context_changed);
            assert_eq!(service.context().value.get(), "before");
        }
    }

    #[test]
    fn text_field_invalid_and_description_drive_describedby() {
        let mut service = service(props());

        drop(service.send(Event::SetHasDescription(true)));

        let attrs = service.connect(&|_| {}).input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("name-field-description")
        );

        drop(service.send(Event::SetInvalid(true)));

        let attrs = service.connect(&|_| {}).input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("name-field-description name-field-error-message")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
    }

    #[test]
    fn text_field_api_helpers_suppress_and_commit_ime_changes() {
        let sent = core::cell::RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let idle_service = service(props());

        let api = idle_service.connect(&send);

        api.on_input_focus(true);
        api.on_input_blur();
        api.on_input_change("typed".to_string());
        api.on_clear_click();
        api.on_input_composition_start();

        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Change("typed".to_string()),
                Event::Clear,
                Event::CompositionStart,
            ]
        );

        let mut composing = service(props());

        drop(composing.send(Event::CompositionStart));

        let api = composing.connect(&send);

        api.on_input_change("ignored".to_string());

        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Change("typed".to_string()),
                Event::Clear,
                Event::CompositionStart,
            ]
        );

        api.on_input_composition_end("final".to_string());

        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Change("typed".to_string()),
                Event::Clear,
                Event::CompositionStart,
                Event::CompositionEnd("final".to_string())
            ]
        );
    }

    #[test]
    fn text_field_composition_end_allows_final_value() {
        let mut service = service(props());

        drop(service.send(Event::CompositionStart));
        drop(service.send(Event::CompositionEnd("東京".to_string())));

        assert!(!service.context().is_composing);
        assert_eq!(service.context().value.get(), "東京");
    }

    #[test]
    fn text_field_composition_end_clears_composing_without_change_when_blocked() {
        for props in [
            props().disabled(true).default_value("before"),
            props().readonly(true).default_value("before"),
        ] {
            let mut service = service(props);

            drop(service.send(Event::CompositionStart));

            let result = service.send(Event::CompositionEnd("after".to_string()));

            assert!(result.context_changed);
            assert!(result.pending_effects.is_empty());
            assert!(!service.context().is_composing);
            assert_eq!(service.context().value.get(), "before");
        }
    }

    #[test]
    fn text_field_clear_and_composition_end_emit_controlled_value_effects() {
        let mut service = service(props().value("parent"));

        let send: StrongSend<Event> = Arc::new(|_| {});

        let clear = service.send(Event::Clear);

        assert_eq!(service.context().value.get(), "parent");
        assert_eq!(clear.pending_effects.len(), 1);

        for effect in clear.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        drop(service.send(Event::CompositionStart));

        let composition_end = service.send(Event::CompositionEnd("final".to_string()));

        assert_eq!(service.context().value.get(), "parent");
        assert_eq!(composition_end.pending_effects.len(), 1);

        for effect in composition_end.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    #[test]
    fn text_field_value_change_effect_fires_for_controlled_and_uncontrolled_changes() {
        let values = Arc::new(std::sync::Mutex::new(Vec::new()));

        let props = props().value("parent").on_value_change({
            let values = Arc::clone(&values);
            callback(move |value: String| {
                values.lock().expect("value log lock").push(value);
            })
        });

        let mut service = service(props);

        let result = service.send(Event::Change("typed".to_string()));

        let send: StrongSend<Event> = Arc::new(|_| {});

        assert_eq!(service.context().value.get(), "parent");

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(
            values.lock().expect("value log lock").as_slice(),
            &["typed".to_string()]
        );
    }

    #[test]
    fn text_field_escape_clears_when_clearable() {
        let sent = core::cell::RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let clearable_service = service(props().clearable(true).default_value("value"));

        let api = clearable_service.connect(&send);

        assert!(api.should_render_clear_trigger());
        assert!(api.on_input_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert_eq!(sent.borrow().as_slice(), &[Event::Clear]);

        let plain_service = service(props().clearable(false));

        let api = plain_service.connect(&send);

        assert!(!api.on_input_keydown(&keyboard_data(KeyboardKey::Escape)));
        assert!(!api.should_render_clear_trigger());

        let empty_service = service(props().clearable(true));

        assert!(!empty_service.connect(&send).should_render_clear_trigger());

        let readonly_service = service(
            props()
                .clearable(true)
                .readonly(true)
                .default_value("value"),
        );

        assert!(
            !readonly_service
                .connect(&send)
                .should_render_clear_trigger()
        );

        let disabled_service = service(
            props()
                .clearable(true)
                .disabled(true)
                .default_value("value"),
        );

        assert!(
            !disabled_service
                .connect(&send)
                .on_input_keydown(&keyboard_data(KeyboardKey::Escape))
        );

        let mut composing_key = keyboard_data(KeyboardKey::Escape);

        composing_key.is_composing = true;

        assert!(
            !clearable_service
                .connect(&send)
                .on_input_keydown(&composing_key)
        );
        assert!(
            !clearable_service
                .connect(&send)
                .on_input_keydown(&keyboard_data(KeyboardKey::Enter))
        );
    }

    #[test]
    fn text_field_focus_change_effect_fires_callback() {
        let count = Arc::new(AtomicUsize::new(0));

        let props = props().on_focus_change({
            let count = Arc::clone(&count);
            callback(move |focused: bool| {
                count.fetch_add(usize::from(focused), Ordering::SeqCst);
            })
        });

        let mut service = service(props);

        let result = service.send(Event::Focus { is_keyboard: false });

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn text_field_input_attrs_cover_output_props_and_input_types() {
        for (input_type, token) in [
            (InputType::Text, "text"),
            (InputType::Password, "password"),
            (InputType::Email, "email"),
            (InputType::Url, "url"),
            (InputType::Tel, "tel"),
            (InputType::Search, "search"),
        ] {
            let attrs = service(props().input_type(input_type))
                .connect(&|_| {})
                .input_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Type), Some(token));
        }

        for (input_type, token) in [
            (InputType::Email, "email"),
            (InputType::Url, "url"),
            (InputType::Tel, "tel"),
            (InputType::Search, "search"),
        ] {
            let attrs = service(props().input_type(input_type))
                .connect(&|_| {})
                .input_attrs();

            assert_eq!(attrs.get(&HtmlAttr::InputMode), Some(token));
        }

        let attrs = service(
            props()
                .value("value")
                .placeholder("Name")
                .max_length(40)
                .min_length(2)
                .pattern("[A-Za-z]+")
                .autocomplete("name")
                .name("name")
                .form("profile")
                .required(true)
                .readonly(true)
                .input_mode(InputMode::Text)
                .dir(Direction::Rtl),
        )
        .connect(&|_| {})
        .input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("name"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("profile"));
        assert_eq!(attrs.get(&HtmlAttr::MaxLength), Some("40"));
        assert_eq!(attrs.get(&HtmlAttr::MinLength), Some("2"));
        assert_eq!(attrs.get(&HtmlAttr::Pattern), Some("[A-Za-z]+"));
        assert_eq!(attrs.get(&HtmlAttr::AutoComplete), Some("name"));
        assert_eq!(attrs.get(&HtmlAttr::InputMode), Some("text"));
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert!(attrs.contains(&HtmlAttr::Required));
        assert!(attrs.contains(&HtmlAttr::ReadOnly));
    }

    #[test]
    fn text_field_display_and_custom_messages_are_covered() {
        assert_eq!(InputType::Password.to_string(), "password");

        let messages = Messages {
            clear_label: MessageFn::static_str("Reset field"),
        };

        let service = Service::<Machine>::new(props(), &Env::default(), &messages);

        let attrs = service.connect(&|_| {}).clear_trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Reset field")
        );
    }

    #[test]
    fn text_field_part_attrs_delegate_for_all_parts() {
        let field_service = service(props());

        let api = field_service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Input), api.input_attrs());
        assert_eq!(
            api.part_attrs(Part::StartDecorator),
            api.start_decorator_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::EndDecorator),
            api.end_decorator_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::ClearTrigger),
            api.clear_trigger_attrs()
        );
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
    }

    #[test]
    fn text_field_api_debug_redacts_sender() {
        let field_service = service(props());

        let api = field_service.connect(&|_| {});

        assert!(format!("{api:?}").contains("send: \"<callback>\""));
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

    #[test]
    fn text_field_snapshots() {
        let mut focused = service(props());

        drop(focused.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "text_field_root_idle",
            snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "text_field_root_focused_keyboard",
            snapshot_attrs(&focused.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "text_field_root_disabled_readonly_invalid",
            snapshot_attrs(
                &service(props().disabled(true).readonly(true).invalid(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
        assert_snapshot!(
            "text_field_label",
            snapshot_attrs(&service(props()).connect(&|_| {}).label_attrs())
        );
        assert_snapshot!(
            "text_field_input_default",
            snapshot_attrs(&service(props()).connect(&|_| {}).input_attrs())
        );
        assert_snapshot!(
            "text_field_input_form_constraints",
            snapshot_attrs(
                &service(
                    props()
                        .value("Alice")
                        .placeholder("Full name")
                        .max_length(50)
                        .min_length(2)
                        .pattern("[A-Za-z ]+")
                        .autocomplete("name")
                        .name("full_name")
                        .form("profile")
                        .required(true)
                )
                .connect(&|_| {})
                .input_attrs()
            )
        );

        let mut described = service(props().invalid(true));

        drop(described.send(Event::SetHasDescription(true)));

        assert_snapshot!(
            "text_field_input_described_invalid",
            snapshot_attrs(&described.connect(&|_| {}).input_attrs())
        );

        assert_snapshot!(
            "text_field_input_search_rtl",
            snapshot_attrs(
                &service(
                    props()
                        .input_type(InputType::Search)
                        .input_mode(InputMode::Search)
                        .dir(Direction::Rtl)
                )
                .connect(&|_| {})
                .input_attrs()
            )
        );
        assert_snapshot!(
            "text_field_start_decorator",
            snapshot_attrs(&service(props()).connect(&|_| {}).start_decorator_attrs())
        );
        assert_snapshot!(
            "text_field_end_decorator",
            snapshot_attrs(&service(props()).connect(&|_| {}).end_decorator_attrs())
        );
        assert_snapshot!(
            "text_field_clear_trigger_default",
            snapshot_attrs(&service(props()).connect(&|_| {}).clear_trigger_attrs())
        );
        assert_snapshot!(
            "text_field_clear_trigger_disabled",
            snapshot_attrs(
                &service(props().disabled(true))
                    .connect(&|_| {})
                    .clear_trigger_attrs()
            )
        );
        assert_snapshot!(
            "text_field_description",
            snapshot_attrs(&service(props()).connect(&|_| {}).description_attrs())
        );
        assert_snapshot!(
            "text_field_error_message",
            snapshot_attrs(&service(props()).connect(&|_| {}).error_message_attrs())
        );
    }
}
