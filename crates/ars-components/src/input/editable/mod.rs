//! Editable component state machine and connect API.
//!
//! This module implements the framework-agnostic `Editable` machine defined in
//! `spec/components/input/editable.md`. The core owns preview/editing state,
//! transient edit text, commit/cancel behavior, IME composition state, and
//! semantic attributes. Live input focus and selection remain adapter concerns.

use alloc::string::{String, ToString};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// The state of the `Editable` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default state, displaying the committed value as readable text.
    Preview,

    /// Active state, exposing an input for editing the transient value.
    Editing,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Preview => "preview",
            Self::Editing => "editing",
        })
    }
}

/// Events for the `Editable` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Enter edit mode from the preview state.
    Activate,

    /// Confirm the supplied edit value and return to preview.
    Submit(String),

    /// Discard the edit value and return to preview.
    Cancel,

    /// Update the transient edit value while editing.
    Change(String),

    /// Focus was received; the flag is true for keyboard-initiated focus.
    Focus {
        /// True when focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus was lost.
    Blur,

    /// IME composition started.
    CompositionStart,

    /// IME composition ended with the final committed text.
    CompositionEnd(String),

    /// The key used to confirm an IME candidate was consumed.
    CompositionConfirmKey,

    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),

    /// Synchronize output-affecting props stored in context.
    SetProps,
}

/// Controls how an edit is submitted automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitMode {
    /// Submit when the input loses focus.
    Blur,

    /// Submit when Enter is pressed.
    Enter,

    /// Submit on either blur or Enter.
    Both,

    /// Never auto-submit; only an explicit submit trigger commits.
    None,
}

impl Display for SubmitMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blur => "blur",
            Self::Enter => "enter",
            Self::Both => "both",
            Self::None => "none",
        })
    }
}

/// Controls how preview interaction activates editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivateMode {
    /// A single click activates editing.
    Click,

    /// A double click activates editing.
    DblClick,

    /// Focus activates editing.
    Focus,

    /// Only the explicit edit trigger activates editing.
    None,
}

impl Display for ActivateMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Click => "click",
            Self::DblClick => "dblclick",
            Self::Focus => "focus",
            Self::None => "none",
        })
    }
}

/// Context for the `Editable` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The committed display value, controlled by the parent or internally owned.
    pub value: Bindable<String>,

    /// Transient value while editing; discarded on cancel.
    pub edit_value: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is required.
    pub required: bool,

    /// Determines how the edit is submitted.
    pub submit_mode: SubmitMode,

    /// Determines how edit mode is activated.
    pub activate_mode: ActivateMode,

    /// When true, adapters should select the input text on activation.
    pub auto_select: bool,

    /// The placeholder text for the input.
    pub placeholder: Option<String>,

    /// The maximum UTF-16 code unit count of the input, matching native `maxlength`.
    pub max_length: Option<usize>,

    /// Form field name associated with the editable input.
    pub name: Option<String>,

    /// The form element ID associated with the editable input.
    pub form: Option<String>,

    /// Whether blur is allowed to submit when the submit mode includes blur.
    pub submit_on_blur: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether focus should be visibly indicated.
    pub focus_visible: bool,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// True when the next Enter key may be the IME confirmation key.
    pub suppress_next_enter_after_composition: bool,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// Props for the `Editable` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the editable root.
    pub id: String,

    /// Controlled value. When `Some`, the committed value is parent-owned.
    pub value: Option<String>,

    /// Default value for uncontrolled mode.
    pub default_value: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Determines how the edit is submitted.
    pub submit_mode: SubmitMode,

    /// Determines how edit mode is activated.
    pub activate_mode: ActivateMode,

    /// Select all text when entering edit mode.
    pub auto_select: bool,

    /// The placeholder text for the input.
    pub placeholder: Option<String>,

    /// The maximum UTF-16 code unit count of the input, matching native `maxlength`.
    pub max_length: Option<usize>,

    /// Whether the editable is in an invalid state.
    pub invalid: bool,

    /// Whether the editable is required.
    pub required: bool,

    /// The name for form submission.
    pub name: Option<String>,

    /// The ID of the form element the input is associated with.
    pub form: Option<String>,

    /// Whether blur may submit when the submit mode includes blur.
    pub submit_on_blur: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: String::new(),
            disabled: false,
            readonly: false,
            submit_mode: SubmitMode::Both,
            activate_mode: ActivateMode::DblClick,
            auto_select: true,
            placeholder: None,
            max_length: None,
            invalid: false,
            required: false,
            name: None,
            form: None,
            submit_on_blur: true,
        }
    }
}

impl Props {
    /// Creates default editable props with an empty ID.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the stable component ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the externally controlled committed value.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Clears the externally controlled committed value.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets the uncontrolled default committed value.
    #[must_use]
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = value.into();
        self
    }

    /// Sets whether the component is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the component is readonly.
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets how edits are submitted automatically.
    #[must_use]
    pub const fn submit_mode(mut self, submit_mode: SubmitMode) -> Self {
        self.submit_mode = submit_mode;
        self
    }

    /// Sets how preview interaction activates editing.
    #[must_use]
    pub const fn activate_mode(mut self, activate_mode: ActivateMode) -> Self {
        self.activate_mode = activate_mode;
        self
    }

    /// Sets whether adapters should select input text on activation.
    #[must_use]
    pub const fn auto_select(mut self, auto_select: bool) -> Self {
        self.auto_select = auto_select;
        self
    }

    /// Sets the input placeholder.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Clears the input placeholder.
    #[must_use]
    pub fn no_placeholder(mut self) -> Self {
        self.placeholder = None;
        self
    }

    /// Sets the maximum UTF-16 code unit count.
    #[must_use]
    pub const fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Clears the maximum UTF-16 code unit count.
    #[must_use]
    pub const fn no_max_length(mut self) -> Self {
        self.max_length = None;
        self
    }

    /// Sets whether the editable is invalid.
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether the editable is required.
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets the form field name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears the form field name.
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets the associated form element ID.
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Clears the associated form element ID.
    #[must_use]
    pub fn no_form(mut self) -> Self {
        self.form = None;
        self
    }

    /// Sets whether blur may submit when submit mode includes blur.
    #[must_use]
    pub const fn submit_on_blur(mut self, submit_on_blur: bool) -> Self {
        self.submit_on_blur = submit_on_blur;
        self
    }
}

/// Locale-specific labels for the `Editable` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the input element.
    pub field_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the submit trigger.
    pub submit_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the cancel trigger.
    pub cancel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the edit trigger.
    pub edit_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            field_label: MessageFn::static_str("Editable field"),
            submit_label: MessageFn::static_str("Submit edit"),
            cancel_label: MessageFn::static_str("Cancel edit"),
            edit_label: MessageFn::static_str("Edit"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Machine for the `Editable` component.
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
        let initial = props
            .value
            .clone()
            .unwrap_or_else(|| props.default_value.clone());

        (
            State::Preview,
            Context {
                value: if let Some(value) = &props.value {
                    Bindable::controlled(value.clone())
                } else {
                    Bindable::uncontrolled(props.default_value.clone())
                },
                edit_value: initial,
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                required: props.required,
                submit_mode: props.submit_mode,
                activate_mode: props.activate_mode,
                auto_select: props.auto_select,
                placeholder: props.placeholder.clone(),
                max_length: props.max_length,
                name: props.name.clone(),
                form: props.form.clone(),
                submit_on_blur: props.submit_on_blur,
                focused: false,
                focus_visible: false,
                is_composing: false,
                suppress_next_enter_after_composition: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Preview, Event::Activate) => {
                if !can_activate(ctx) {
                    return None;
                }

                let current_value = ctx.value.get().clone();
                Some(
                    TransitionPlan::to(State::Editing).apply(move |ctx: &mut Context| {
                        ctx.edit_value = current_value;
                        ctx.focused = true;
                        ctx.focus_visible = false;
                    }),
                )
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (State::Preview, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    clear_focus(ctx);
                }))
            }

            (State::Editing, Event::Change(value)) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing {
                    return None;
                }

                let value = clamp_to_max_length(value, ctx.max_length);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.edit_value = value;
                    ctx.suppress_next_enter_after_composition = false;
                }))
            }

            (State::Editing, Event::Submit(value)) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing {
                    return None;
                }

                let value = clamp_to_max_length(value, ctx.max_length);
                Some(
                    TransitionPlan::to(State::Preview).apply(move |ctx: &mut Context| {
                        ctx.edit_value = value.clone();

                        if !ctx.value.is_controlled() {
                            ctx.value.set(value);
                        }

                        clear_focus(ctx);

                        ctx.is_composing = false;
                        ctx.suppress_next_enter_after_composition = false;
                    }),
                )
            }

            (State::Editing, Event::Cancel) => {
                let committed = ctx.value.get().clone();
                Some(
                    TransitionPlan::to(State::Preview).apply(move |ctx: &mut Context| {
                        ctx.edit_value = committed;

                        clear_focus(ctx);

                        ctx.is_composing = false;
                        ctx.suppress_next_enter_after_composition = false;
                    }),
                )
            }

            (State::Editing, Event::Blur) => {
                if ctx.is_composing {
                    return None;
                }

                let edit_value = ctx.edit_value.clone();
                let committed = ctx.value.get().clone();

                let should_submit = effective_blur_submits(ctx);

                Some(
                    TransitionPlan::to(State::Preview).apply(move |ctx: &mut Context| {
                        if should_submit && !ctx.disabled && !ctx.readonly {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(edit_value.clone());
                            }

                            ctx.edit_value = edit_value;
                        } else {
                            ctx.edit_value = committed;
                        }

                        clear_focus(ctx);

                        ctx.is_composing = false;
                        ctx.suppress_next_enter_after_composition = false;
                    }),
                )
            }

            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = true;
                    ctx.suppress_next_enter_after_composition = false;
                }))
            }

            (State::Editing, Event::CompositionEnd(value)) => {
                let value = clamp_to_max_length(value, ctx.max_length);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.is_composing = false;
                    ctx.suppress_next_enter_after_composition = true;

                    if !ctx.disabled && !ctx.readonly {
                        ctx.edit_value = value;
                    }
                }))
            }

            (State::Preview, Event::CompositionEnd(_)) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = false;
                    ctx.suppress_next_enter_after_composition = false;
                }))
            }

            (State::Editing, Event::CompositionConfirmKey) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.suppress_next_enter_after_composition = false;
                }))
            }

            (_, Event::SetValue(value)) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(value) = value {
                        let edit_value = clamp_to_max_length(&value, ctx.max_length);

                        ctx.value.set(edit_value.clone());
                        ctx.value.sync_controlled(Some(value));
                        ctx.edit_value = edit_value;
                        ctx.suppress_next_enter_after_composition = false;
                    } else {
                        ctx.value.sync_controlled(None);
                        ctx.edit_value = clamp_to_max_length(ctx.value.get(), ctx.max_length);
                        ctx.suppress_next_enter_after_composition = false;
                    }
                }))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.submit_mode = props.submit_mode;
                    ctx.activate_mode = props.activate_mode;
                    ctx.auto_select = props.auto_select;
                    ctx.placeholder = props.placeholder;
                    ctx.max_length = props.max_length;
                    ctx.name = props.name;
                    ctx.form = props.form;
                    ctx.submit_on_blur = props.submit_on_blur;
                    ctx.edit_value = clamp_to_max_length(&ctx.edit_value, ctx.max_length);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "editable::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
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

/// Structural parts exposed by the `Editable` connect API.
#[derive(ComponentPart)]
#[scope = "editable"]
pub enum Part {
    /// Root container that groups the editable parts.
    Root,

    /// Optional label associated with the input.
    Label,

    /// Focusable preview text shown outside edit mode.
    Preview,

    /// Native text input shown in edit mode.
    Input,

    /// Explicit trigger for entering edit mode.
    EditTrigger,

    /// Explicit trigger for submitting the current edit.
    SubmitTrigger,

    /// Explicit trigger for canceling the current edit.
    CancelTrigger,
}

/// API for deriving editable attributes and dispatching editable events.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Preview => self.preview_attrs(),
            Part::Input => self.input_attrs(),
            Part::EditTrigger => self.edit_trigger_attrs(),
            Part::SubmitTrigger => self.submit_trigger_attrs(),
            Part::CancelTrigger => self.cancel_trigger_attrs(),
        }
    }
}

impl Api<'_> {
    /// Returns true when the editable is currently in editing mode.
    #[must_use]
    pub const fn is_editing(&self) -> bool {
        matches!(self.state, State::Editing)
    }

    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "group")
            .set(HtmlAttr::Data("ars-state"), self.state.to_string());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
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
        let mut attrs = part_attrs(&Part::Label);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Attributes for the preview text element.
    #[must_use]
    pub fn preview_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Preview);

        if !self.ctx.disabled {
            attrs.set(HtmlAttr::TabIndex, "0");
        }

        attrs
    }

    /// Attributes for the input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Input);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.field_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Value, self.ctx.edit_value.as_str());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if let Some(placeholder) = &self.ctx.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.as_str());
        }

        if let Some(max_length) = self.ctx.max_length {
            attrs.set(HtmlAttr::MaxLength, max_length.to_string());
        }

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.as_str());
        }

        if let Some(form) = &self.ctx.form {
            attrs.set(HtmlAttr::Form, form.as_str());
        }

        attrs
    }

    /// Attributes for the submit trigger button.
    #[must_use]
    pub fn submit_trigger_attrs(&self) -> AttrMap {
        let mut attrs = trigger_attrs(&Part::SubmitTrigger);

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.submit_label)(&self.ctx.locale),
        );

        self.apply_trigger_disabled(&mut attrs);

        attrs
    }

    /// Attributes for the cancel trigger button.
    #[must_use]
    pub fn cancel_trigger_attrs(&self) -> AttrMap {
        let mut attrs = trigger_attrs(&Part::CancelTrigger);

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.cancel_label)(&self.ctx.locale),
        );

        self.apply_trigger_disabled(&mut attrs);

        attrs
    }

    /// Attributes for the edit trigger button.
    #[must_use]
    pub fn edit_trigger_attrs(&self) -> AttrMap {
        let mut attrs = trigger_attrs(&Part::EditTrigger);

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.edit_label)(&self.ctx.locale),
        );

        self.apply_trigger_disabled(&mut attrs);

        attrs
    }

    /// Dispatches preview click activation when configured.
    pub fn on_preview_click(&self) {
        if self.ctx.activate_mode == ActivateMode::Click {
            (self.send)(Event::Activate);
        }
    }

    /// Dispatches preview double-click activation when configured.
    pub fn on_preview_dblclick(&self) {
        if self.ctx.activate_mode == ActivateMode::DblClick {
            (self.send)(Event::Activate);
        }
    }

    /// Dispatches preview focus and focus activation when configured.
    pub fn on_preview_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });

        if self.ctx.activate_mode == ActivateMode::Focus {
            (self.send)(Event::Activate);
        }
    }

    /// Dispatches preview blur.
    pub fn on_preview_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handles normalized keydown data on the preview element.
    pub fn on_preview_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Enter
            && self.ctx.activate_mode != ActivateMode::None
            && !self.is_keyboard_composing(data)
        {
            (self.send)(Event::Activate);
        }
    }

    /// Dispatches an input change when no composition is active.
    pub fn on_input_change(&self, value: String) {
        if !self.ctx.is_composing {
            (self.send)(Event::Change(value));
        }
    }

    /// Dispatches input blur.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handles normalized keydown data on the input.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, false);
    }

    /// Re-checks Enter handling after an adapter-scheduled composition microtask.
    pub fn on_input_keydown_after_composition_check(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, true);
    }

    /// Dispatches IME composition start.
    pub fn on_input_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Dispatches IME composition end with the final committed text.
    pub fn on_input_composition_end(&self, final_value: String) {
        (self.send)(Event::CompositionEnd(final_value));
    }

    /// Dispatches submit trigger activation.
    pub fn on_submit_click(&self) {
        (self.send)(Event::Submit(self.ctx.edit_value.clone()));
    }

    /// Dispatches cancel trigger activation.
    pub fn on_cancel_click(&self) {
        (self.send)(Event::Cancel);
    }

    /// Dispatches explicit edit trigger activation.
    pub fn on_edit_trigger_click(&self) {
        (self.send)(Event::Activate);
    }

    fn on_input_keydown_impl(&self, data: &KeyboardEventData, after_composition_check: bool) {
        let composing = self.is_keyboard_composing(data);

        match data.key {
            KeyboardKey::Process => (self.send)(Event::CompositionStart),

            KeyboardKey::Escape if !composing => (self.send)(Event::Cancel),

            KeyboardKey::Enter
                if after_composition_check && self.ctx.suppress_next_enter_after_composition =>
            {
                (self.send)(Event::CompositionConfirmKey);
            }

            KeyboardKey::Enter
                if (!composing || after_composition_check)
                    && !self.ctx.is_composing
                    && matches!(self.ctx.submit_mode, SubmitMode::Enter | SubmitMode::Both) =>
            {
                (self.send)(Event::Submit(self.ctx.edit_value.clone()));
            }

            KeyboardKey::Tab if !composing && effective_blur_submits(self.ctx) => {
                (self.send)(Event::Submit(self.ctx.edit_value.clone()));
            }

            _ => {}
        }
    }

    fn is_keyboard_composing(&self, data: &KeyboardEventData) -> bool {
        self.ctx.is_composing || data.is_composing || data.key == KeyboardKey::Process
    }

    fn apply_trigger_disabled(&self, attrs: &mut AttrMap) {
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
    }
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

const fn can_activate(ctx: &Context) -> bool {
    !ctx.disabled && !ctx.readonly
}

const fn clear_focus(ctx: &mut Context) {
    ctx.focused = false;
    ctx.focus_visible = false;
}

const fn effective_blur_submits(ctx: &Context) -> bool {
    ctx.submit_on_blur && matches!(ctx.submit_mode, SubmitMode::Blur | SubmitMode::Both)
}

fn clamp_to_max_length(value: &str, max_length: Option<usize>) -> String {
    if let Some(max_length) = max_length {
        let mut units = 0;
        let mut end = 0;

        for (index, ch) in value.char_indices() {
            let next_units = units + ch.len_utf16();

            if next_units > max_length {
                break;
            }

            units = next_units;
            end = index + ch.len_utf8();
        }

        value[..end].to_string()
    } else {
        value.to_string()
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn trigger_attrs(part: &Part) -> AttrMap {
    let mut attrs = part_attrs(part);

    attrs.set(HtmlAttr::Type, "button");

    attrs
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.submit_mode != new.submit_mode
        || old.activate_mode != new.activate_mode
        || old.auto_select != new.auto_select
        || old.placeholder != new.placeholder
        || old.max_length != new.max_length
        || old.invalid != new.invalid
        || old.required != new.required
        || old.name != new.name
        || old.form != new.form
        || old.submit_on_blur != new.submit_on_blur
}

#[cfg(test)]
mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use core::cell::RefCell;

    use ars_core::{AttrMap, Env, Service, StrongSend};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("editable").default_value("saved")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages::default())
    }

    fn keyboard(key: KeyboardKey, is_composing: bool) -> KeyboardEventData {
        KeyboardEventData {
            key,
            code: "KeyA".to_string(),
            character: None,
            alt_key: false,
            ctrl_key: false,
            meta_key: false,
            shift_key: false,
            repeat: false,
            is_composing,
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn editable_initial_state_uses_uncontrolled_default_value() {
        let editable = service(props().default_value("hello"));

        assert_eq!(*editable.state(), State::Preview);
        assert_eq!(editable.context().value.get(), "hello");
        assert_eq!(editable.context().edit_value, "hello");
        assert!(!editable.context().is_composing);
    }

    #[test]
    fn editable_initial_state_uses_controlled_value() {
        let editable = service(props().default_value("default").value("parent"));

        assert_eq!(editable.context().value.get(), "parent");
        assert_eq!(editable.context().edit_value, "parent");
    }

    #[test]
    fn editable_activate_enters_editing_from_preview() {
        let mut editable = service(props().default_value("current"));

        drop(editable.send(Event::Activate));

        assert_eq!(*editable.state(), State::Editing);
        assert_eq!(editable.context().edit_value, "current");
        assert!(editable.context().focused);
    }

    #[test]
    fn editable_disabled_and_readonly_ignore_activation() {
        for props in [props().disabled(true), props().readonly(true)] {
            let mut editable = service(props);

            let result = editable.send(Event::Activate);

            assert_eq!(*editable.state(), State::Preview);
            assert!(!result.state_changed);
            assert!(!result.context_changed);
        }
    }

    #[test]
    fn editable_builder_unsetters_preserve_other_props() {
        let props = Props::new()
            .id("editable")
            .value("parent")
            .default_value("default")
            .disabled(true)
            .readonly(true)
            .placeholder("placeholder")
            .max_length(8)
            .name("field")
            .form("form")
            .no_placeholder()
            .no_max_length()
            .no_name()
            .no_form()
            .uncontrolled();

        assert_eq!(props.id, "editable");
        assert_eq!(props.default_value, "default");
        assert!(props.disabled);
        assert!(props.readonly);
        assert_eq!(props.value, None);
        assert_eq!(props.placeholder, None);
        assert_eq!(props.max_length, None);
        assert_eq!(props.name, None);
        assert_eq!(props.form, None);
    }

    #[test]
    fn editable_change_updates_transient_value_and_clamps_by_native_maxlength_units() {
        let mut editable = service(props().max_length(3));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Change("a😀b".to_string())));

        assert_eq!(editable.context().edit_value, "a😀");
        assert_eq!(editable.context().value.get(), "saved");
    }

    #[test]
    fn editable_maxlength_does_not_split_non_bmp_scalars() {
        let mut editable = service(props().max_length(1));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Change("😀a".to_string())));

        assert_eq!(editable.context().edit_value, "");
        assert_eq!(editable.context().value.get(), "saved");
    }

    #[test]
    fn editable_change_ignores_disabled_readonly_and_composing() {
        for readonly_props in [
            props().disabled(true),
            props().readonly(true),
            props().disabled(true).readonly(true),
        ] {
            let mut editable = service(props());

            drop(editable.send(Event::Activate));

            editable.set_props(readonly_props);

            drop(editable.send(Event::Change("typed".to_string())));

            assert_eq!(editable.context().edit_value, "saved");
        }

        let mut composing = service(props());

        drop(composing.send(Event::Activate));
        drop(composing.send(Event::CompositionStart));
        drop(composing.send(Event::Change("typed".to_string())));

        assert_eq!(composing.context().edit_value, "saved");
    }

    #[test]
    fn editable_submit_commits_uncontrolled_value() {
        let mut editable = service(props());

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Change("next".to_string())));
        drop(editable.send(Event::Submit("next".to_string())));

        assert_eq!(*editable.state(), State::Preview);
        assert_eq!(editable.context().value.get(), "next");
        assert_eq!(editable.context().edit_value, "next");
    }

    #[test]
    fn editable_submit_is_blocked_during_composition() {
        let mut editable = service(props());

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));
        drop(editable.send(Event::Submit("stale".to_string())));

        assert_eq!(*editable.state(), State::Editing);
        assert_eq!(editable.context().value.get(), "saved");
        assert_eq!(editable.context().edit_value, "saved");
        assert!(editable.context().is_composing);
    }

    #[test]
    fn editable_submit_keeps_controlled_committed_value_parent_owned() {
        let mut editable = service(props().value("parent"));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Submit("local".to_string())));

        assert_eq!(editable.context().value.get(), "parent");
        assert_eq!(editable.context().edit_value, "local");
    }

    #[test]
    fn editable_cancel_reverts_to_committed_value() {
        let mut editable = service(props());

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Change("draft".to_string())));
        drop(editable.send(Event::Cancel));

        assert_eq!(*editable.state(), State::Preview);
        assert_eq!(editable.context().edit_value, "saved");
        assert_eq!(editable.context().value.get(), "saved");
    }

    #[test]
    fn editable_submit_ignores_disabled_or_readonly_after_activation() {
        for readonly_props in [
            props().disabled(true),
            props().readonly(true),
            props().disabled(true).readonly(true),
        ] {
            let mut editable = service(props());

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));

            editable.set_props(readonly_props);

            let result = editable.send(Event::Submit("draft".to_string()));

            assert_eq!(*editable.state(), State::Editing);
            assert!(!result.state_changed);
            assert!(!result.context_changed);
            assert_eq!(editable.context().value.get(), "saved");
        }
    }

    #[test]
    fn editable_escape_key_cancels_editing() {
        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let editable = service(props());

        let api = editable.connect(&send);

        api.on_input_keydown(&keyboard(KeyboardKey::Escape, false));

        assert_eq!(sent.borrow().as_slice(), &[Event::Cancel]);
    }

    #[test]
    fn editable_enter_key_submits_only_for_enter_modes() {
        for mode in [SubmitMode::Enter, SubmitMode::Both] {
            let sent = RefCell::new(Vec::new());

            let mut editable = service(props().submit_mode(mode));

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));

            editable
                .connect(&|event| sent.borrow_mut().push(event))
                .on_input_keydown(&keyboard(KeyboardKey::Enter, false));

            assert_eq!(
                sent.borrow().as_slice(),
                &[Event::Submit("draft".to_string())]
            );
        }

        for mode in [SubmitMode::Blur, SubmitMode::None] {
            let sent = RefCell::new(Vec::new());

            let mut editable = service(props().submit_mode(mode));

            drop(editable.send(Event::Activate));

            editable
                .connect(&|event| sent.borrow_mut().push(event))
                .on_input_keydown(&keyboard(KeyboardKey::Enter, false));

            assert!(sent.borrow().is_empty());
        }
    }

    #[test]
    fn editable_blur_submits_or_cancels_according_to_effective_blur_mode() {
        for props in [
            props().submit_mode(SubmitMode::Blur),
            props().submit_mode(SubmitMode::Both),
        ] {
            let mut editable = service(props);

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));
            drop(editable.send(Event::Blur));

            assert_eq!(editable.context().value.get(), "draft");
            assert_eq!(*editable.state(), State::Preview);
        }

        for props in [
            props().submit_mode(SubmitMode::Enter),
            props().submit_mode(SubmitMode::None),
            props().submit_mode(SubmitMode::Both).submit_on_blur(false),
        ] {
            let mut editable = service(props);

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));
            drop(editable.send(Event::Blur));

            assert_eq!(editable.context().value.get(), "saved");
            assert_eq!(editable.context().edit_value, "saved");
            assert_eq!(*editable.state(), State::Preview);
        }
    }

    #[test]
    fn editable_blur_is_blocked_during_composition() {
        let mut editable = service(props().submit_mode(SubmitMode::Blur));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));
        drop(editable.send(Event::Blur));

        assert_eq!(*editable.state(), State::Editing);
        assert_eq!(editable.context().value.get(), "saved");
        assert_eq!(editable.context().edit_value, "saved");
        assert!(editable.context().is_composing);
    }

    #[test]
    fn editable_blur_submit_preserves_controlled_committed_value() {
        let mut editable = service(
            props()
                .value("parent")
                .submit_mode(SubmitMode::Blur)
                .activate_mode(ActivateMode::Click),
        );

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::Change("draft".to_string())));
        drop(editable.send(Event::Blur));

        assert_eq!(editable.context().value.get(), "parent");
        assert_eq!(editable.context().edit_value, "draft");
        assert_eq!(*editable.state(), State::Preview);
    }

    #[test]
    fn editable_preview_blur_clears_focus_state() {
        let mut editable = service(props());

        drop(editable.send(Event::Focus { is_keyboard: true }));

        assert!(editable.context().focused);
        assert!(editable.context().focus_visible);

        drop(editable.send(Event::Blur));

        assert!(!editable.context().focused);
        assert!(!editable.context().focus_visible);
    }

    #[test]
    fn editable_activation_handlers_respect_activate_mode() {
        for (mode, action, should_activate) in [
            (ActivateMode::Click, "click", true),
            (ActivateMode::DblClick, "dblclick", true),
            (ActivateMode::None, "click", false),
        ] {
            let sent = RefCell::new(Vec::new());
            let send = |event| sent.borrow_mut().push(event);

            let editable = service(props().activate_mode(mode));

            let api = editable.connect(&send);

            match action {
                "click" => api.on_preview_click(),
                "dblclick" => api.on_preview_dblclick(),
                _ => unreachable!("covered activation action"),
            }

            if should_activate {
                assert_eq!(sent.borrow().as_slice(), &[Event::Activate]);
            } else {
                assert!(sent.borrow().is_empty());
            }
        }

        let sent = RefCell::new(Vec::new());

        let focus = service(props().activate_mode(ActivateMode::Focus));

        focus
            .connect(&|event| sent.borrow_mut().push(event))
            .on_preview_focus(true);

        assert_eq!(
            sent.borrow().as_slice(),
            &[Event::Focus { is_keyboard: true }, Event::Activate]
        );
    }

    #[test]
    fn editable_key_and_trigger_helpers_fan_out() {
        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let editable = service(props().activate_mode(ActivateMode::DblClick));

        let api = editable.connect(&send);

        api.on_preview_keydown(&keyboard(KeyboardKey::Enter, false));
        api.on_preview_blur();
        api.on_input_change("typed".to_string());
        api.on_input_blur();
        api.on_input_composition_start();
        api.on_input_composition_end("final".to_string());
        api.on_submit_click();
        api.on_cancel_click();
        api.on_edit_trigger_click();

        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::Activate,
                Event::Blur,
                Event::Change("typed".to_string()),
                Event::Blur,
                Event::CompositionStart,
                Event::CompositionEnd("final".to_string()),
                Event::Submit("saved".to_string()),
                Event::Cancel,
                Event::Activate,
            ]
        );
    }

    #[test]
    fn editable_props_sync_updates_context_and_controlled_value() {
        let mut editable = service(props().value("parent"));

        editable.set_props(
            props()
                .value("next")
                .disabled(true)
                .readonly(true)
                .invalid(true)
                .required(true)
                .submit_mode(SubmitMode::Enter)
                .activate_mode(ActivateMode::Click)
                .auto_select(false)
                .placeholder("placeholder")
                .max_length(2)
                .name("field")
                .form("form")
                .submit_on_blur(false),
        );

        assert_eq!(editable.context().value.get(), "next");
        assert_eq!(editable.context().edit_value, "ne");
        assert!(editable.context().disabled);
        assert!(editable.context().readonly);
        assert!(editable.context().invalid);
        assert!(editable.context().required);
        assert_eq!(editable.context().submit_mode, SubmitMode::Enter);
        assert_eq!(editable.context().activate_mode, ActivateMode::Click);
        assert!(!editable.context().auto_select);
        assert_eq!(
            editable.context().placeholder.as_deref(),
            Some("placeholder")
        );
        assert_eq!(editable.context().max_length, Some(2));
        assert_eq!(editable.context().name.as_deref(), Some("field"));
        assert_eq!(editable.context().form.as_deref(), Some("form"));
        assert!(!editable.context().submit_on_blur);
    }

    #[test]
    fn editable_props_sync_can_clear_controlled_value_without_output_changes() {
        let mut editable = service(props().value("parent"));

        editable.set_props(props().default_value("fallback"));

        assert_eq!(editable.context().value.get(), "parent");
        assert_eq!(editable.context().edit_value, "parent");
    }

    #[test]
    fn editable_props_changed_reports_value_only_and_output_prop_changes() {
        let base = props();
        let value_only = props().value("parent");

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&base, &value_only),
            vec![Event::SetValue(Some("parent".to_string()))]
        );
        assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());

        for changed in [
            props().disabled(true),
            props().readonly(true),
            props().submit_mode(SubmitMode::Enter),
            props().activate_mode(ActivateMode::Click),
            props().auto_select(false),
            props().placeholder("placeholder"),
            props().max_length(8),
            props().invalid(true),
            props().required(true),
            props().name("field"),
            props().form("form"),
            props().submit_on_blur(false),
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&base, &changed),
                vec![Event::SetProps]
            );
        }
    }

    #[test]
    fn editable_composition_lifecycle_tracks_and_applies_final_value() {
        let mut editable = service(props().max_length(2));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));

        assert!(editable.context().is_composing);

        drop(editable.send(Event::CompositionEnd("日本語".to_string())));

        assert!(!editable.context().is_composing);
        assert_eq!(editable.context().edit_value, "日本");
        assert_eq!(editable.context().value.get(), "saved");
    }

    #[test]
    fn editable_composition_end_clears_preview_and_ignores_readonly_edits() {
        let mut preview = service(props());

        drop(preview.send(Event::CompositionStart));
        drop(preview.send(Event::CompositionEnd("ignored".to_string())));

        assert!(!preview.context().is_composing);
        assert_eq!(preview.context().edit_value, "saved");

        let mut readonly = service(props().readonly(true));

        drop(readonly.send(Event::CompositionStart));
        drop(readonly.send(Event::CompositionEnd("ignored".to_string())));

        assert!(!readonly.context().is_composing);
        assert_eq!(readonly.context().edit_value, "saved");

        let mut disabled_while_editing = service(props());

        drop(disabled_while_editing.send(Event::Activate));

        disabled_while_editing.set_props(props().disabled(true));

        drop(disabled_while_editing.send(Event::CompositionStart));
        drop(disabled_while_editing.send(Event::CompositionEnd("ignored".to_string())));

        assert!(!disabled_while_editing.context().is_composing);
        assert_eq!(disabled_while_editing.context().edit_value, "saved");
    }

    #[test]
    fn editable_api_suppresses_change_while_composing() {
        let sent = RefCell::new(Vec::new());
        let mut editable = service(props());

        drop(editable.send(Event::CompositionStart));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_change("ignored".to_string());

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn editable_keyboard_process_starts_composition_and_suppresses_enter() {
        let sent = RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let editable = service(props());

        let api = editable.connect(&send);

        api.on_input_keydown(&keyboard(KeyboardKey::Process, false));
        api.on_input_keydown(&keyboard(KeyboardKey::Enter, true));

        assert_eq!(sent.borrow().as_slice(), &[Event::CompositionStart]);
    }

    #[test]
    fn editable_escape_and_tab_are_suppressed_during_composition() {
        let sent = RefCell::new(Vec::new());
        let mut editable = service(props());

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown(&keyboard(KeyboardKey::Escape, false));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown(&keyboard(KeyboardKey::Tab, false));

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn editable_after_composition_check_submits_only_after_composition_ends() {
        let sent = RefCell::new(Vec::new());

        let mut editable = service(props().submit_mode(SubmitMode::Enter));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown_after_composition_check(&keyboard(KeyboardKey::Enter, false));

        assert!(sent.borrow().is_empty());

        drop(editable.send(Event::CompositionEnd("final".to_string())));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown_after_composition_check(&keyboard(KeyboardKey::Enter, false));

        assert_eq!(sent.borrow().as_slice(), &[Event::CompositionConfirmKey]);

        drop(editable.send(Event::CompositionConfirmKey));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown(&keyboard(KeyboardKey::Enter, false));

        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::CompositionConfirmKey,
                Event::Submit("final".to_string()),
            ]
        );
    }

    #[test]
    fn editable_normal_enter_after_composition_end_can_submit() {
        let sent = RefCell::new(Vec::new());
        let mut editable = service(props().submit_mode(SubmitMode::Enter));

        drop(editable.send(Event::Activate));
        drop(editable.send(Event::CompositionStart));
        drop(editable.send(Event::CompositionEnd("final".to_string())));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_input_keydown(&keyboard(KeyboardKey::Enter, false));

        assert_eq!(
            sent.borrow().as_slice(),
            &[Event::Submit("final".to_string())]
        );
    }

    #[test]
    fn editable_tab_submits_only_when_blur_submission_is_effective() {
        for props in [
            props().submit_mode(SubmitMode::Blur),
            props().submit_mode(SubmitMode::Both),
        ] {
            let sent = RefCell::new(Vec::new());

            let mut editable = service(props);

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));

            editable
                .connect(&|event| sent.borrow_mut().push(event))
                .on_input_keydown(&keyboard(KeyboardKey::Tab, false));

            assert_eq!(
                sent.borrow().as_slice(),
                &[Event::Submit("draft".to_string())]
            );
        }

        for props in [
            props().submit_mode(SubmitMode::Enter),
            props().submit_mode(SubmitMode::None),
            props().submit_mode(SubmitMode::Both).submit_on_blur(false),
        ] {
            let sent = RefCell::new(Vec::new());

            let mut editable = service(props);

            drop(editable.send(Event::Activate));
            drop(editable.send(Event::Change("draft".to_string())));

            editable
                .connect(&|event| sent.borrow_mut().push(event))
                .on_input_keydown(&keyboard(KeyboardKey::Tab, false));

            assert!(sent.borrow().is_empty());
        }
    }

    #[test]
    fn editable_preview_keys_ignore_ime_enter() {
        let sent = RefCell::new(Vec::new());

        let editable = service(props());

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_preview_keydown(&keyboard(KeyboardKey::Enter, true));

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn editable_preview_enter_respects_activate_mode_none() {
        let sent = RefCell::new(Vec::new());

        let editable = service(props().activate_mode(ActivateMode::None));

        editable
            .connect(&|event| sent.borrow_mut().push(event))
            .on_preview_keydown(&keyboard(KeyboardKey::Enter, false));

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn editable_attrs_cover_disabled_and_readonly_input_branches() {
        assert_snapshot!(
            "editable_input_disabled",
            snapshot_attrs(
                &service(props().disabled(true))
                    .connect(&|_| {})
                    .input_attrs()
            )
        );
        assert_snapshot!(
            "editable_input_readonly",
            snapshot_attrs(
                &service(props().readonly(true))
                    .connect(&|_| {})
                    .input_attrs()
            )
        );
    }

    #[test]
    fn editable_disabled_preview_is_not_tabbable() {
        let attrs = service(props().disabled(true))
            .connect(&|_| {})
            .preview_attrs();

        assert!(!attrs.contains(&HtmlAttr::TabIndex));
    }

    #[test]
    fn editable_part_attrs_delegate_to_each_part_method() {
        let editable = service(props());

        let api = editable.connect(&|_| {});

        for (part, expected) in [
            (Part::Root, snapshot_attrs(&api.root_attrs())),
            (Part::Label, snapshot_attrs(&api.label_attrs())),
            (Part::Preview, snapshot_attrs(&api.preview_attrs())),
            (Part::Input, snapshot_attrs(&api.input_attrs())),
            (Part::EditTrigger, snapshot_attrs(&api.edit_trigger_attrs())),
            (
                Part::SubmitTrigger,
                snapshot_attrs(&api.submit_trigger_attrs()),
            ),
            (
                Part::CancelTrigger,
                snapshot_attrs(&api.cancel_trigger_attrs()),
            ),
        ] {
            assert_eq!(snapshot_attrs(&api.part_attrs(part)), expected);
        }
    }

    #[test]
    fn editable_api_reports_editing_state() {
        let mut editable = service(props());

        assert!(!editable.connect(&|_| {}).is_editing());

        drop(editable.send(Event::Activate));

        assert!(editable.connect(&|_| {}).is_editing());
    }

    #[test]
    fn editable_api_debug_redacts_sender() {
        let editable = service(props());
        let debug = format!("{:?}", editable.connect(&|_| {}));

        assert!(debug.contains("send: \"<callback>\""));
        assert!(!debug.contains("send: 0x"));
    }

    #[test]
    fn editable_display_helpers_are_covered() {
        assert_eq!(State::Preview.to_string(), "preview");
        assert_eq!(State::Editing.to_string(), "editing");
        assert_eq!(SubmitMode::Blur.to_string(), "blur");
        assert_eq!(SubmitMode::Enter.to_string(), "enter");
        assert_eq!(SubmitMode::Both.to_string(), "both");
        assert_eq!(SubmitMode::None.to_string(), "none");
        assert_eq!(ActivateMode::Click.to_string(), "click");
        assert_eq!(ActivateMode::DblClick.to_string(), "dblclick");
        assert_eq!(ActivateMode::Focus.to_string(), "focus");
        assert_eq!(ActivateMode::None.to_string(), "none");
    }

    #[test]
    fn editable_snapshots_cover_parts_and_output_branches() {
        assert_snapshot!(
            "editable_root_preview",
            snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
        );

        let mut editing = service(props());

        drop(editing.send(Event::Activate));

        assert_snapshot!(
            "editable_root_editing",
            snapshot_attrs(&editing.connect(&|_| {}).root_attrs())
        );

        let mut flagged = service(props().disabled(true).readonly(true).invalid(true));

        drop(flagged.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "editable_root_disabled_readonly_invalid_focus_visible",
            snapshot_attrs(&flagged.connect(&|_| {}).root_attrs())
        );

        assert_snapshot!(
            "editable_label",
            snapshot_attrs(&service(props()).connect(&|_| {}).label_attrs())
        );

        assert_snapshot!(
            "editable_preview",
            snapshot_attrs(&service(props()).connect(&|_| {}).preview_attrs())
        );

        assert_snapshot!(
            "editable_input_default",
            snapshot_attrs(&service(props()).connect(&|_| {}).input_attrs())
        );

        assert_snapshot!(
            "editable_input_form_constraints",
            snapshot_attrs(
                &service(
                    props()
                        .default_value("draft")
                        .placeholder("Label")
                        .max_length(32)
                        .name("title")
                        .form("settings")
                        .required(true)
                        .invalid(true)
                )
                .connect(&|_| {})
                .input_attrs()
            )
        );

        assert_snapshot!(
            "editable_submit_trigger",
            snapshot_attrs(&service(props()).connect(&|_| {}).submit_trigger_attrs())
        );

        assert_snapshot!(
            "editable_cancel_trigger",
            snapshot_attrs(&service(props()).connect(&|_| {}).cancel_trigger_attrs())
        );

        assert_snapshot!(
            "editable_edit_trigger",
            snapshot_attrs(&service(props()).connect(&|_| {}).edit_trigger_attrs())
        );

        assert_snapshot!(
            "editable_disabled_trigger",
            snapshot_attrs(
                &service(props().disabled(true))
                    .connect(&|_| {})
                    .edit_trigger_attrs()
            )
        );
    }

    #[test]
    fn editable_controlled_value_effects_are_absent() {
        let mut editable = service(props().value("parent"));

        let send: StrongSend<Event> = Arc::new(|_| {});

        let result = editable.send(Event::Submit("local".to_string()));

        for effect in result.pending_effects {
            drop(effect.run(editable.context(), editable.props(), Arc::clone(&send)));
        }

        assert_eq!(editable.context().value.get(), "parent");
    }
}
