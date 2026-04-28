//! Button component state machine and connect API.
//!
//! This module implements the framework-agnostic `Button` core. Framework
//! adapters own DOM event normalization and native handler deduplication;
//! this module owns typed props, state transitions, and `AttrMap` output.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, Locale,
    MessageFn, SafeUrl, TransitionPlan, UnsafeUrlError, sanitize_url,
};

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// The states for the `Button` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default resting state. The button is not focused, pressed, or loading.
    Idle,

    /// The button has received focus.
    Focused,

    /// The button is actively being pressed.
    Pressed,

    /// The button is busy and remains focusable but not activatable.
    Loading,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused => f.write_str("focused"),
            Self::Pressed => f.write_str("pressed"),
            Self::Loading => f.write_str("loading"),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// The events for the `Button` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus was received.
    Focus {
        /// Whether the focus came from keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus was lost.
    Blur,

    /// A pointer or keyboard press began.
    Press,

    /// A pointer or keyboard press ended.
    Release,

    /// The button was activated.
    Click,

    /// Synchronize the loading prop.
    SetLoading(bool),

    /// Synchronize the disabled prop.
    SetDisabled(bool),
}

// ────────────────────────────────────────────────────────────────────
// Typed prop vocabularies
// ────────────────────────────────────────────────────────────────────

/// Visual style token for the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Default neutral button style.
    #[default]
    Default,

    /// Primary action style.
    Primary,

    /// Secondary action style.
    Secondary,

    /// Destructive or dangerous action style.
    Destructive,

    /// Outlined button style.
    Outline,

    /// Low-chrome ghost button style.
    Ghost,

    /// Link-like button style.
    Link,
}

impl Variant {
    /// Returns the data-attribute token for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Visual size token for the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Size {
    /// Small button size.
    Sm,

    /// Medium button size.
    #[default]
    Md,

    /// Large button size.
    Lg,

    /// Icon-only button size.
    Icon,
}

impl Size {
    /// Returns the data-attribute token for this size.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
            Self::Icon => "icon",
        }
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The HTML button type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Type {
    /// A non-submitting button.
    #[default]
    Button,

    /// A form submit button.
    Submit,

    /// A form reset button.
    Reset,
}

impl Type {
    /// Returns the HTML `type` token for this button type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Form submission method override for a submit button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormMethod {
    /// Submit using HTTP GET.
    Get,

    /// Submit using HTTP POST.
    Post,

    /// Close an ancestor dialog without network submission.
    Dialog,
}

impl FormMethod {
    /// Returns the HTML `formmethod` token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Dialog => "dialog",
        }
    }
}

impl Display for FormMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Form encoding type override for a submit button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormEncType {
    /// Standard URL-encoded form body.
    UrlEncoded,

    /// Multipart form body, typically for file uploads.
    MultipartFormData,

    /// Plain text form body.
    TextPlain,
}

impl FormEncType {
    /// Returns the HTML `formenctype` token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UrlEncoded => "application/x-www-form-urlencoded",
            Self::MultipartFormData => "multipart/form-data",
            Self::TextPlain => "text/plain",
        }
    }
}

impl Display for FormEncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Browsing context override for a form submit response.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FormTarget {
    /// Submit into the current browsing context.
    Self_,

    /// Submit into a new browsing context.
    Blank,

    /// Submit into the parent browsing context.
    Parent,

    /// Submit into the top-level browsing context.
    Top,

    /// Submit into a named browsing context.
    Named(String),
}

impl FormTarget {
    /// Returns the HTML `formtarget` token.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Self_ => "_self",
            Self::Blank => "_blank",
            Self::Parent => "_parent",
            Self::Top => "_top",
            Self::Named(name) => name.as_str(),
        }
    }
}

impl Display for FormTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings for the `Button` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label applied while the button is in loading state.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// The context for the `Button` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the button is disabled.
    pub disabled: bool,

    /// Whether the button is in a loading state.
    pub loading: bool,

    /// Whether the button is currently pressed.
    pub pressed: bool,

    /// Whether the button is currently focused.
    pub focused: bool,

    /// Whether focus should be rendered as keyboard-visible focus.
    pub focus_visible: bool,

    /// The active locale inherited from provider context.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the `Button` component.
#[derive(Clone, Debug, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Whether the button is disabled.
    pub disabled: bool,

    /// Whether the button is in a loading state.
    pub loading: bool,

    /// Visual style token exposed as `data-ars-variant`.
    pub variant: Variant,

    /// Visual size token exposed as `data-ars-size`.
    pub size: Size,

    /// The HTML button type.
    pub r#type: Type,

    /// The ID of the associated form.
    pub form: Option<String>,

    /// Name submitted with form data.
    pub name: Option<String>,

    /// Value submitted with form data.
    pub value: Option<String>,

    /// Whether attributes are merged onto a consumer child element.
    pub as_child: bool,

    /// Whether the button is removed from sequential Tab navigation.
    pub exclude_from_tab_order: bool,

    /// Form action URL override.
    pub form_action: Option<SafeUrl>,

    /// Form method override.
    pub form_method: Option<FormMethod>,

    /// Form encoding type override.
    pub form_enc_type: Option<FormEncType>,

    /// Form target override.
    pub form_target: Option<FormTarget>,

    /// Whether the submit bypasses native form validation.
    pub form_no_validate: bool,

    /// Whether the button receives focus on mount.
    pub auto_focus: bool,

    /// Whether adapters should suppress focus on pointer press.
    pub prevent_focus_on_press: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            loading: false,
            variant: Variant::Default,
            size: Size::Md,
            r#type: Type::Button,
            form: None,
            name: None,
            value: None,
            as_child: false,
            exclude_from_tab_order: false,
            form_action: None,
            form_method: None,
            form_enc_type: None,
            form_target: None,
            form_no_validate: false,
            auto_focus: false,
            prevent_focus_on_press: false,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    ///
    /// This is the documented entry point for the builder chain. Use chained
    /// setters to populate configuration without struct-literal boilerplate.
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

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`loading`](Self::loading).
    #[must_use]
    pub const fn loading(mut self, value: bool) -> Self {
        self.loading = value;
        self
    }

    /// Sets [`variant`](Self::variant).
    #[must_use]
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets [`size`](Self::size).
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets [`r#type`](Self::type).
    #[must_use]
    pub const fn button_type(mut self, button_type: Type) -> Self {
        self.r#type = button_type;
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets [`value`](Self::value).
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets [`as_child`](Self::as_child).
    #[must_use]
    pub const fn as_child(mut self, value: bool) -> Self {
        self.as_child = value;
        self
    }

    /// Sets [`exclude_from_tab_order`](Self::exclude_from_tab_order).
    #[must_use]
    pub const fn exclude_from_tab_order(mut self, value: bool) -> Self {
        self.exclude_from_tab_order = value;
        self
    }

    /// Sets [`form_action`](Self::form_action) from an already validated URL.
    #[must_use]
    pub fn form_action(mut self, action: SafeUrl) -> Self {
        self.form_action = Some(action);
        self
    }

    /// Validates and sets [`form_action`](Self::form_action).
    ///
    /// # Errors
    ///
    /// Returns [`UnsafeUrlError`] when the supplied URL has a disallowed
    /// scheme.
    pub fn try_form_action(mut self, action: impl Into<String>) -> Result<Self, UnsafeUrlError> {
        self.form_action = Some(SafeUrl::new(action)?);
        Ok(self)
    }

    /// Sets [`form_method`](Self::form_method).
    #[must_use]
    pub const fn form_method(mut self, method: FormMethod) -> Self {
        self.form_method = Some(method);
        self
    }

    /// Sets [`form_enc_type`](Self::form_enc_type).
    #[must_use]
    pub const fn form_enc_type(mut self, enc_type: FormEncType) -> Self {
        self.form_enc_type = Some(enc_type);
        self
    }

    /// Sets [`form_target`](Self::form_target).
    #[must_use]
    pub fn form_target(mut self, target: FormTarget) -> Self {
        self.form_target = Some(target);
        self
    }

    /// Sets [`form_no_validate`](Self::form_no_validate).
    #[must_use]
    pub const fn form_no_validate(mut self, value: bool) -> Self {
        self.form_no_validate = value;
        self
    }

    /// Sets [`auto_focus`](Self::auto_focus).
    #[must_use]
    pub const fn auto_focus(mut self, value: bool) -> Self {
        self.auto_focus = value;
        self
    }

    /// Sets [`prevent_focus_on_press`](Self::prevent_focus_on_press).
    #[must_use]
    pub const fn prevent_focus_on_press(mut self, value: bool) -> Self {
        self.prevent_focus_on_press = value;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// The machine for the `Button` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let state = if props.loading {
            State::Loading
        } else {
            State::Idle
        };

        (
            state,
            Context {
                disabled: props.disabled,
                loading: props.loading,
                pressed: false,
                focused: false,
                focus_visible: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled
            && !matches!(
                event,
                Event::Blur | Event::SetDisabled(_) | Event::SetLoading(_)
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

            (State::Focused, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(clear_focus))
            }

            (State::Idle | State::Focused, Event::Press) => Some(
                TransitionPlan::to(State::Pressed).apply(|ctx: &mut Context| {
                    ctx.pressed = true;
                }),
            ),

            (State::Pressed, Event::Release) => {
                let target = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };
                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.pressed = false;
                }))
            }

            (State::Pressed | State::Loading, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (State::Pressed, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.pressed = false;
                    clear_focus(ctx);
                }))
            }

            (State::Loading, Event::Blur) => Some(TransitionPlan::context_only(clear_focus)),

            (_, Event::SetLoading(true)) => Some(TransitionPlan::to(State::Loading).apply(
                |ctx: &mut Context| {
                    ctx.loading = true;
                    ctx.pressed = false;
                },
            )),

            (State::Loading, Event::SetLoading(false)) => {
                let target = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };
                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.loading = false;
                }))
            }

            (_, Event::SetLoading(false)) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.loading = false;
                }))
            }

            (State::Focused | State::Pressed, Event::SetDisabled(true)) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.disabled = true;
                    ctx.pressed = false;
                    clear_focus(ctx);
                }))
            }

            (State::Loading, Event::SetDisabled(true)) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.disabled = true;
                    ctx.pressed = false;
                    clear_focus(ctx);
                }))
            }

            (_, Event::SetDisabled(disabled)) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();

        if old.loading != new.loading {
            events.push(Event::SetLoading(new.loading));
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

const fn clear_focus(ctx: &mut Context) {
    ctx.focused = false;
    ctx.focus_visible = false;
}

// ────────────────────────────────────────────────────────────────────
// Connect / API
// ────────────────────────────────────────────────────────────────────

/// DOM parts of the `Button` component.
#[derive(ComponentPart)]
#[scope = "button"]
pub enum Part {
    /// The root button element.
    Root,

    /// Loading indicator shown while busy.
    LoadingIndicator,

    /// Content slot containing the visible button label or icons.
    Content,
}

/// The API for the `Button` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("button::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl<'a> Api<'a> {
    /// Returns whether the button is in loading state.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self.state, State::Loading) || self.ctx.loading
    }

    /// Returns whether the button is disabled for interaction.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled || self.ctx.loading
    }

    /// Returns whether focus should render as focus-visible.
    #[must_use]
    pub const fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    /// Returns whether the button is currently pressed.
    #[must_use]
    pub const fn is_pressed(&self) -> bool {
        self.ctx.pressed
    }

    /// Returns whether adapters should suppress pointer-induced focus.
    #[must_use]
    pub const fn should_prevent_focus_on_press(&self) -> bool {
        self.props.prevent_focus_on_press
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

    /// Dispatches a click event.
    pub fn on_click(&self) {
        (self.send)(Event::Click);
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "button")
            .set(HtmlAttr::Type, self.props.r#type.as_str())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string())
            .set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str())
            .set(HtmlAttr::Data("ars-size"), self.props.size.as_str());

        if self.is_loading() {
            attrs
                .set_bool(HtmlAttr::Data("ars-loading"), true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set_bool(HtmlAttr::Data("ars-disabled"), true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.pressed {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        if self.props.exclude_from_tab_order || (self.props.as_child && self.ctx.disabled) {
            attrs.set(HtmlAttr::TabIndex, "-1");
        } else if self.props.as_child && !self.ctx.disabled {
            attrs.set(HtmlAttr::TabIndex, "0");
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }

        if let Some(value) = &self.props.value {
            attrs.set(HtmlAttr::Value, value);
        }

        if let Some(form_action) = &self.props.form_action {
            attrs.set(HtmlAttr::FormAction, sanitize_url(form_action.as_str()));
        }

        if let Some(form_method) = self.props.form_method {
            attrs.set(HtmlAttr::FormMethod, form_method.as_str());
        }

        if let Some(form_enc_type) = self.props.form_enc_type {
            attrs.set(HtmlAttr::FormEncType, form_enc_type.as_str());
        }

        if let Some(form_target) = &self.props.form_target {
            attrs.set(HtmlAttr::FormTarget, form_target.as_str());
        }

        if self.props.form_no_validate {
            attrs.set_bool(HtmlAttr::FormNoValidate, true);
        }

        if self.props.auto_focus {
            attrs.set_bool(HtmlAttr::AutoFocus, true);
        }

        if self.props.prevent_focus_on_press {
            attrs.set(HtmlAttr::Data("ars-prevent-focus-on-press"), "true");
        }

        attrs
    }

    /// Loading indicator element attributes.
    #[must_use]
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingIndicator.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.is_loading() {
            attrs
                .set_bool(HtmlAttr::Data("ars-loading"), true)
                .set(HtmlAttr::Role, "status")
                .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

            let loading_text = (self.ctx.messages.loading_label)(&self.ctx.locale);

            if !loading_text.is_empty() {
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), loading_text);
            }
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        }

        attrs
    }

    /// Content slot element attributes.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-loading"),
                if self.is_loading() { "true" } else { "false" },
            );

        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::Content => self.content_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use ars_core::{AttrValue, Machine as _, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props::new().id("button-1")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages::default())
    }

    fn send(service: &mut Service<Machine>, event: Event) {
        drop(service.send(event));
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    // ── Builder tests ──────────────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let safe = SafeUrl::new("/submit").expect("safe URL should validate");
        let props = Props::new()
            .id("button-1")
            .disabled(true)
            .loading(true)
            .variant(Variant::Destructive)
            .size(Size::Icon)
            .button_type(Type::Submit)
            .form("form-1")
            .name("intent")
            .value("delete")
            .as_child(true)
            .exclude_from_tab_order(true)
            .form_action(safe.clone())
            .form_method(FormMethod::Post)
            .form_enc_type(FormEncType::MultipartFormData)
            .form_target(FormTarget::Blank)
            .form_no_validate(true)
            .auto_focus(true)
            .prevent_focus_on_press(true);

        assert_eq!(props.id, "button-1");
        assert!(props.disabled);
        assert!(props.loading);
        assert_eq!(props.variant, Variant::Destructive);
        assert_eq!(props.size, Size::Icon);
        assert_eq!(props.r#type, Type::Submit);
        assert_eq!(props.form.as_deref(), Some("form-1"));
        assert_eq!(props.name.as_deref(), Some("intent"));
        assert_eq!(props.value.as_deref(), Some("delete"));
        assert!(props.as_child);
        assert!(props.exclude_from_tab_order);
        assert_eq!(props.form_action, Some(safe));
        assert_eq!(props.form_method, Some(FormMethod::Post));
        assert_eq!(props.form_enc_type, Some(FormEncType::MultipartFormData));
        assert_eq!(props.form_target, Some(FormTarget::Blank));
        assert!(props.form_no_validate);
        assert!(props.auto_focus);
        assert!(props.prevent_focus_on_press);
    }

    #[test]
    fn props_builder_try_form_action_accepts_safe_url() {
        let props = Props::new()
            .try_form_action("/submit")
            .expect("safe URL should validate");

        assert_eq!(
            props.form_action.as_ref().map(SafeUrl::as_str),
            Some("/submit")
        );
    }

    #[test]
    fn props_builder_try_form_action_rejects_unsafe_url() {
        let error = Props::new()
            .try_form_action("javascript:alert(1)")
            .expect_err("unsafe URL should be rejected");

        assert_eq!(error.0, "javascript:alert(1)");
    }

    // ── Typed token tests ──────────────────────────────────────────

    #[test]
    fn enum_tokens_match_expected_html_values() {
        assert_eq!(Variant::Default.as_str(), "default");
        assert_eq!(Variant::Primary.as_str(), "primary");
        assert_eq!(Variant::Secondary.as_str(), "secondary");
        assert_eq!(Variant::Destructive.as_str(), "destructive");
        assert_eq!(Variant::Outline.as_str(), "outline");
        assert_eq!(Variant::Ghost.as_str(), "ghost");
        assert_eq!(Variant::Link.as_str(), "link");

        assert_eq!(Size::Sm.as_str(), "sm");
        assert_eq!(Size::Md.as_str(), "md");
        assert_eq!(Size::Lg.as_str(), "lg");
        assert_eq!(Size::Icon.as_str(), "icon");

        assert_eq!(Type::Button.as_str(), "button");
        assert_eq!(Type::Submit.as_str(), "submit");
        assert_eq!(Type::Reset.as_str(), "reset");

        assert_eq!(FormMethod::Get.as_str(), "get");
        assert_eq!(FormMethod::Post.as_str(), "post");
        assert_eq!(FormMethod::Dialog.as_str(), "dialog");

        assert_eq!(
            FormEncType::UrlEncoded.as_str(),
            "application/x-www-form-urlencoded"
        );
        assert_eq!(
            FormEncType::MultipartFormData.as_str(),
            "multipart/form-data"
        );
        assert_eq!(FormEncType::TextPlain.as_str(), "text/plain");

        assert_eq!(FormTarget::Self_.as_str(), "_self");
        assert_eq!(FormTarget::Blank.as_str(), "_blank");
        assert_eq!(FormTarget::Parent.as_str(), "_parent");
        assert_eq!(FormTarget::Top.as_str(), "_top");
        assert_eq!(FormTarget::Named("preview".into()).as_str(), "preview");
    }

    #[test]
    fn display_impls_match_expected_html_and_data_tokens() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Focused.to_string(), "focused");
        assert_eq!(State::Pressed.to_string(), "pressed");
        assert_eq!(State::Loading.to_string(), "loading");

        assert_eq!(Variant::Default.to_string(), "default");
        assert_eq!(Variant::Primary.to_string(), "primary");
        assert_eq!(Variant::Secondary.to_string(), "secondary");
        assert_eq!(Variant::Destructive.to_string(), "destructive");
        assert_eq!(Variant::Outline.to_string(), "outline");
        assert_eq!(Variant::Ghost.to_string(), "ghost");
        assert_eq!(Variant::Link.to_string(), "link");

        assert_eq!(Size::Sm.to_string(), "sm");
        assert_eq!(Size::Md.to_string(), "md");
        assert_eq!(Size::Lg.to_string(), "lg");
        assert_eq!(Size::Icon.to_string(), "icon");

        assert_eq!(Type::Button.to_string(), "button");
        assert_eq!(Type::Submit.to_string(), "submit");
        assert_eq!(Type::Reset.to_string(), "reset");

        assert_eq!(FormMethod::Get.to_string(), "get");
        assert_eq!(FormMethod::Post.to_string(), "post");
        assert_eq!(FormMethod::Dialog.to_string(), "dialog");

        assert_eq!(
            FormEncType::UrlEncoded.to_string(),
            "application/x-www-form-urlencoded"
        );
        assert_eq!(
            FormEncType::MultipartFormData.to_string(),
            "multipart/form-data"
        );
        assert_eq!(FormEncType::TextPlain.to_string(), "text/plain");

        assert_eq!(FormTarget::Self_.to_string(), "_self");
        assert_eq!(FormTarget::Blank.to_string(), "_blank");
        assert_eq!(FormTarget::Parent.to_string(), "_parent");
        assert_eq!(FormTarget::Top.to_string(), "_top");
        assert_eq!(FormTarget::Named("preview".into()).to_string(), "preview");
    }

    // ── Machine tests ──────────────────────────────────────────────

    #[test]
    fn initial_state_is_idle() {
        let service = service(test_props());

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().loading);
    }

    #[test]
    fn initial_state_is_loading_when_props_loading() {
        let service = service(test_props().loading(true));

        assert_eq!(*service.state(), State::Loading);
        assert!(service.context().loading);
    }

    #[test]
    fn focus_visible_tracks_keyboard_focus() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });

        assert_eq!(*service.state(), State::Focused);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);
    }

    #[test]
    fn pointer_focus_is_not_focus_visible() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: false });

        assert_eq!(*service.state(), State::Focused);
        assert!(service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn blur_from_focused_returns_to_idle_and_clears_focus() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Blur);

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn press_from_focused_transitions_to_pressed() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);

        assert_eq!(*service.state(), State::Pressed);
        assert!(service.context().pressed);
    }

    #[test]
    fn release_from_pressed_returns_to_focused_when_still_focused() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);
        send(&mut service, Event::Release);

        assert_eq!(*service.state(), State::Focused);
        assert!(!service.context().pressed);
        assert!(service.context().focused);
    }

    #[test]
    fn release_from_pressed_returns_to_idle_when_not_focused() {
        let mut service = service(test_props());

        send(&mut service, Event::Press);
        send(&mut service, Event::Release);

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().pressed);
    }

    #[test]
    fn disabled_guard_blocks_interactive_transitions() {
        let mut service = service(test_props().disabled(true));

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);
        send(&mut service, Event::Release);
        send(&mut service, Event::Click);

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().focused);
        assert!(!service.context().pressed);
    }

    #[test]
    fn click_is_notification_only() {
        let mut service = service(test_props());

        let result = service.send(Event::Click);

        assert_eq!(*service.state(), State::Idle);
        assert!(!result.state_changed);
        assert!(!result.context_changed);
    }

    #[test]
    fn loading_guard_blocks_press_but_allows_focus_and_blur() {
        let mut service = service(test_props().loading(true));

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);

        assert_eq!(*service.state(), State::Loading);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);
        assert!(!service.context().pressed);

        send(&mut service, Event::Blur);

        assert_eq!(*service.state(), State::Loading);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn set_disabled_true_while_loading_clears_focus_without_exiting_loading() {
        let mut service = service(test_props().loading(true));

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::SetDisabled(true));

        assert_eq!(*service.state(), State::Loading);
        assert!(service.context().disabled);
        assert!(!service.context().pressed);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn set_disabled_false_syncs_context_without_changing_state() {
        let mut service = service(test_props().disabled(true));

        send(&mut service, Event::SetDisabled(false));

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().disabled);
    }

    #[test]
    fn set_loading_enters_and_exits_loading_state() {
        let mut service = service(test_props());

        send(&mut service, Event::Press);
        send(&mut service, Event::SetLoading(true));

        assert_eq!(*service.state(), State::Loading);
        assert!(service.context().loading);
        assert!(!service.context().pressed);

        send(&mut service, Event::SetLoading(false));

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().loading);
    }

    #[test]
    fn set_loading_false_returns_to_focused_when_focus_remains() {
        let mut service = service(test_props().loading(true));

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::SetLoading(false));

        assert_eq!(*service.state(), State::Focused);
        assert!(!service.context().loading);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);
    }

    #[test]
    fn stale_set_loading_false_preserves_pressed_context() {
        let mut service = service(test_props());

        send(&mut service, Event::Press);
        send(&mut service, Event::SetLoading(false));

        assert_eq!(*service.state(), State::Pressed);
        assert!(service.context().pressed);
        assert!(!service.context().loading);
    }

    #[test]
    fn set_disabled_true_clears_focused_and_pressed_context() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);
        send(&mut service, Event::SetDisabled(true));

        assert_eq!(*service.state(), State::Idle);
        assert!(service.context().disabled);
        assert!(!service.context().pressed);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn blur_before_release_from_pressed_ends_idle() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);
        send(&mut service, Event::Blur);
        send(&mut service, Event::Release);

        assert_eq!(*service.state(), State::Idle);
        assert!(!service.context().focused);
        assert!(!service.context().pressed);
    }

    #[test]
    fn on_props_changed_emits_loading_and_disabled_sync_events() {
        let old = test_props();
        let new = test_props().loading(true).disabled(true);

        assert_eq!(
            Machine::on_props_changed(&old, &new),
            vec![Event::SetLoading(true), Event::SetDisabled(true)]
        );
    }

    #[test]
    fn on_props_changed_emits_no_events_when_sync_props_match() {
        assert!(Machine::on_props_changed(&test_props(), &test_props()).is_empty());
    }

    // ── API tests ──────────────────────────────────────────────────

    #[test]
    fn api_state_helpers_reflect_context() {
        let mut service = service(test_props().prevent_focus_on_press(true));

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);

        let api = service.connect(&|_| {});

        assert!(api.is_pressed());
        assert!(api.is_focus_visible());
        assert!(api.should_prevent_focus_on_press());
        assert!(!api.is_loading());
        assert!(!api.is_disabled());
    }

    #[test]
    fn api_loading_helpers_reflect_loading_state() {
        let service = service(test_props().loading(true));

        let api = service.connect(&|_| {});

        assert!(api.is_loading());
        assert!(api.is_disabled());
    }

    #[test]
    fn api_debug_includes_state_context_and_props() {
        let service = service(test_props().loading(true));

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("button::Api"));
        assert!(debug.contains("state"));
        assert!(debug.contains("ctx"));
        assert!(debug.contains("props"));
    }

    #[test]
    fn handler_methods_dispatch_expected_events() {
        let events = Arc::new(Mutex::new(Vec::new()));

        let sink = Arc::clone(&events);
        let send = move |event| sink.lock().expect("events mutex").push(event);

        let service = service(test_props());

        let api = service.connect(&send);

        api.on_focus(true);
        api.on_blur();
        api.on_press();
        api.on_release();
        api.on_click();

        assert_eq!(
            *events.lock().expect("events mutex"),
            vec![
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Press,
                Event::Release,
                Event::Click,
            ]
        );
    }

    #[test]
    fn part_attrs_dispatches_all_parts() {
        let service = service(test_props());

        let api = service.connect(&|_| {});

        assert_eq!(
            api.part_attrs(Part::Root).get(&HtmlAttr::Data("ars-part")),
            Some("root")
        );
        assert_eq!(
            api.part_attrs(Part::LoadingIndicator)
                .get(&HtmlAttr::Data("ars-part")),
            Some("loading-indicator")
        );
        assert_eq!(
            api.part_attrs(Part::Content)
                .get(&HtmlAttr::Data("ars-part")),
            Some("content")
        );
    }

    #[test]
    fn messages_default_loading_label_returns_loading() {
        let messages = Messages::default();

        let locale = Locale::parse("en-US").expect("en-US must parse");

        assert_eq!((messages.loading_label)(&locale), "Loading");
    }

    #[test]
    fn messages_default_pair_compares_by_arc_identity() {
        let lhs = Messages::default();
        let rhs = Messages::default();

        let clone = lhs.clone();

        assert_ne!(lhs, rhs);
        assert_eq!(lhs, clone);
    }

    #[test]
    fn root_attrs_default_values_are_correct() {
        let attrs = service(test_props()).connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-variant")), Some("default"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-size")), Some("md"));
    }

    #[test]
    fn root_attrs_exclude_from_tab_order_takes_precedence_over_as_child_tabindex() {
        let attrs = service(test_props().as_child(true).exclude_from_tab_order(true))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn root_attrs_disabled_as_child_removes_from_tab_order() {
        let attrs = service(test_props().as_child(true).disabled(true))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Disabled),
            Some(&AttrValue::Bool(true))
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn root_attrs_form_overrides_are_typed() {
        let props = test_props()
            .button_type(Type::Submit)
            .form("form-1")
            .name("intent")
            .value("save")
            .try_form_action("/submit")
            .expect("safe URL should validate")
            .form_method(FormMethod::Post)
            .form_enc_type(FormEncType::MultipartFormData)
            .form_target(FormTarget::Named("result-frame".into()))
            .form_no_validate(true);

        let attrs = service(props).connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("submit"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("form-1"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("intent"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("save"));
        assert_eq!(attrs.get(&HtmlAttr::FormAction), Some("/submit"));
        assert_eq!(attrs.get(&HtmlAttr::FormMethod), Some("post"));
        assert_eq!(
            attrs.get(&HtmlAttr::FormEncType),
            Some("multipart/form-data")
        );
        assert_eq!(attrs.get(&HtmlAttr::FormTarget), Some("result-frame"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::FormNoValidate),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn root_attrs_uses_sanitize_url_for_form_action_output() {
        let mut props = test_props();

        // SafeUrl prevents unsafe construction through public builders. This
        // test verifies the output path still uses the shared sanitizer by
        // exercising a safe URL that sanitizer must preserve.
        props.form_action = Some(SafeUrl::new("./submit").expect("safe URL should validate"));

        let attrs = service(props).connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::FormAction), Some("./submit"));
    }

    #[test]
    fn root_attrs_loading_preserves_accessible_name() {
        let attrs = service(test_props().loading(true))
            .connect(&|_| {})
            .root_attrs();

        assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)).is_none());
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn root_attrs_loading_and_disabled_includes_disabled_attrs() {
        let attrs = service(test_props().loading(true).disabled(true))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Disabled),
            Some(&AttrValue::Bool(true))
        );
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn loading_indicator_loading_announces_status_without_replacing_root_name() {
        let attrs = service(test_props().loading(true))
            .connect(&|_| {})
            .loading_indicator_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Loading"));
    }

    #[test]
    fn loading_indicator_omits_empty_loading_label() {
        let service: Service<Machine> = Service::new(
            test_props().loading(true),
            &Env::default(),
            &Messages {
                loading_label: MessageFn::static_str(""),
            },
        );

        let attrs = service.connect(&|_: Event| {}).loading_indicator_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)).is_none());
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn button_root_default_snapshot() {
        assert_snapshot!(
            "button_root_default",
            snapshot_attrs(&service(test_props()).connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn button_root_focused_snapshot() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });

        assert_snapshot!(
            "button_root_focused",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn button_root_pressed_snapshot() {
        let mut service = service(test_props());

        send(&mut service, Event::Focus { is_keyboard: true });
        send(&mut service, Event::Press);

        assert_snapshot!(
            "button_root_pressed",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn button_root_disabled_snapshot() {
        assert_snapshot!(
            "button_root_disabled",
            snapshot_attrs(
                &service(test_props().disabled(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn button_root_loading_snapshot() {
        assert_snapshot!(
            "button_root_loading",
            snapshot_attrs(
                &service(test_props().loading(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn button_root_as_child_snapshot() {
        assert_snapshot!(
            "button_root_as_child",
            snapshot_attrs(
                &service(test_props().as_child(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn button_root_form_override_snapshot() {
        let props = test_props()
            .variant(Variant::Primary)
            .size(Size::Lg)
            .button_type(Type::Submit)
            .form("checkout")
            .name("intent")
            .value("pay")
            .try_form_action("/checkout")
            .expect("safe URL should validate")
            .form_method(FormMethod::Post)
            .form_enc_type(FormEncType::UrlEncoded)
            .form_target(FormTarget::Self_)
            .form_no_validate(true)
            .auto_focus(true)
            .prevent_focus_on_press(true);

        assert_snapshot!(
            "button_root_form_override",
            snapshot_attrs(&service(props).connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn button_loading_indicator_idle_snapshot() {
        assert_snapshot!(
            "button_loading_indicator_idle",
            snapshot_attrs(
                &service(test_props())
                    .connect(&|_| {})
                    .loading_indicator_attrs()
            )
        );
    }

    #[test]
    fn button_loading_indicator_loading_snapshot() {
        assert_snapshot!(
            "button_loading_indicator_loading",
            snapshot_attrs(
                &service(test_props().loading(true))
                    .connect(&|_| {})
                    .loading_indicator_attrs()
            )
        );
    }

    #[test]
    fn button_content_idle_snapshot() {
        assert_snapshot!(
            "button_content_idle",
            snapshot_attrs(&service(test_props()).connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn button_content_loading_snapshot() {
        assert_snapshot!(
            "button_content_loading",
            snapshot_attrs(
                &service(test_props().loading(true))
                    .connect(&|_| {})
                    .content_attrs()
            )
        );
    }

    #[test]
    fn handler_callbacks_can_be_invoked_repeatedly() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_send = Arc::clone(&calls);
        let send = move |_event| {
            calls_for_send.fetch_add(1, Ordering::SeqCst);
        };

        let service = service(test_props());

        let api = service.connect(&send);

        api.on_press();
        api.on_click();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
