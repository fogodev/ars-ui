//! Textarea component state machine and connect API.
//!
//! This module implements the framework-agnostic `Textarea` machine defined in
//! `spec/components/input/textarea.md`. The native textarea element is the form
//! participant; no hidden input is emitted for this component.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, CssProperty, Direction, EffectMetadata, Env, HtmlAttr, InputMode, Locale,
    PendingEffect, ResizeToContentEffect, TransitionPlan, no_cleanup,
};
use ars_i18n::{
    grapheme_count,
    number::{FormatOptions, Formatter, SignDisplay},
};

/// The state of the `Textarea` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle (not focused).
    Idle,

    /// The component is focused.
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

/// The events for the `Textarea` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The component received focus.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// The component lost focus.
    Blur,

    /// The text value changed.
    Change(String),

    /// The text was cleared.
    Clear,

    /// Validation state changed.
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

/// The resize mode of the textarea.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResizeMode {
    /// No resizing allowed.
    None,

    /// Both horizontal and vertical resizing.
    Both,

    /// Horizontal resizing only.
    Horizontal,

    /// Vertical resizing only.
    #[default]
    Vertical,
}

impl ResizeMode {
    /// Returns the CSS `resize` token for this mode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Both => "both",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

impl Display for ResizeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The context for the `Textarea` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The text value, controlled or uncontrolled.
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

    /// Whether the focus is visible.
    pub focus_visible: bool,

    /// The placeholder text.
    pub placeholder: Option<String>,

    /// The maximum character length.
    pub max_length: Option<u32>,

    /// The minimum character length.
    pub min_length: Option<u32>,

    /// The name attribute for form submission.
    pub name: Option<String>,

    /// The autocomplete hint.
    pub autocomplete: Option<String>,

    /// Number of visible text rows.
    pub rows: u32,

    /// Number of visible text columns.
    pub cols: Option<u32>,

    /// The resize mode.
    pub resize: ResizeMode,

    /// Whether the textarea auto-resizes to fit content.
    pub auto_resize: bool,

    /// Maximum height constraint for auto-resize.
    pub max_height: Option<String>,

    /// Maximum number of rows for auto-resize height capping.
    pub max_rows: Option<u32>,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// Whether a Description part is rendered.
    pub has_description: bool,

    /// Text direction for RTL support.
    pub dir: Direction,

    /// Mobile on-screen keyboard layout hint.
    pub input_mode: Option<InputMode>,

    /// Resolved locale for character-count number formatting.
    pub locale: Locale,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The props for the `Textarea` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the textarea root.
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

    /// The placeholder text.
    pub placeholder: Option<String>,

    /// The maximum character length.
    pub max_length: Option<u32>,

    /// The minimum character length.
    pub min_length: Option<u32>,

    /// The name attribute for form submission.
    pub name: Option<String>,

    /// The ID of the form element the textarea is associated with.
    pub form: Option<String>,

    /// The autocomplete hint.
    pub autocomplete: Option<String>,

    /// Number of visible text rows.
    pub rows: u32,

    /// Number of visible text columns.
    pub cols: Option<u32>,

    /// The resize mode.
    pub resize: ResizeMode,

    /// Whether the textarea auto-resizes to fit content.
    pub auto_resize: bool,

    /// Maximum height constraint for auto-resize.
    pub max_height: Option<String>,

    /// Maximum number of rows for auto-resize height capping.
    pub max_rows: Option<u32>,

    /// The direction of the component.
    pub dir: Direction,

    /// Hint for the virtual keyboard type on mobile devices.
    pub input_mode: Option<InputMode>,

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
            max_length: None,
            min_length: None,
            name: None,
            form: None,
            autocomplete: None,
            rows: 3,
            cols: None,
            resize: ResizeMode::Vertical,
            auto_resize: false,
            max_height: None,
            max_rows: None,
            dir: Direction::Ltr,
            input_mode: None,
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

    /// Sets [`rows`](Self::rows), the visible row count.
    #[must_use]
    pub const fn rows(mut self, value: u32) -> Self {
        self.rows = value;
        self
    }

    /// Sets [`cols`](Self::cols), the visible column count.
    #[must_use]
    pub const fn cols(mut self, value: u32) -> Self {
        self.cols = Some(value);
        self
    }

    /// Clears [`cols`](Self::cols).
    #[must_use]
    pub const fn no_cols(mut self) -> Self {
        self.cols = None;
        self
    }

    /// Sets [`resize`](Self::resize).
    #[must_use]
    pub const fn resize(mut self, value: ResizeMode) -> Self {
        self.resize = value;
        self
    }

    /// Sets [`auto_resize`](Self::auto_resize).
    #[must_use]
    pub const fn auto_resize(mut self, value: bool) -> Self {
        self.auto_resize = value;
        self
    }

    /// Sets [`max_height`](Self::max_height).
    #[must_use]
    pub fn max_height(mut self, value: impl Into<String>) -> Self {
        self.max_height = Some(value.into());
        self
    }

    /// Clears [`max_height`](Self::max_height).
    #[must_use]
    pub fn no_max_height(mut self) -> Self {
        self.max_height = None;
        self
    }

    /// Sets [`max_rows`](Self::max_rows).
    #[must_use]
    pub const fn max_rows(mut self, value: u32) -> Self {
        self.max_rows = Some(value);
        self
    }

    /// Clears [`max_rows`](Self::max_rows).
    #[must_use]
    pub const fn no_max_rows(mut self) -> Self {
        self.max_rows = None;
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

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Character-count metadata exposed by the `Textarea` connect API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterCount {
    /// Number of Unicode extended grapheme clusters in the current textarea value.
    pub current: usize,

    /// Maximum value length when a `maxlength` constraint is present.
    pub max: Option<u32>,

    /// Display text for the character-count live region.
    pub text: String,
}

/// Typed identifier for every named effect intent the textarea machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter recomputes textarea height after content changes.
    AutoResize,

    /// Adapter invokes `Props::on_value_change` with the new committed value.
    ValueChange,
}

/// The machine for the `Textarea` component.
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
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                value: match &props.value {
                    Some(value) => Bindable::controlled(value.clone()),
                    None => Bindable::uncontrolled(props.default_value.clone()),
                },
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                required: props.required,
                focused: false,
                focus_visible: false,
                placeholder: props.placeholder.clone(),
                max_length: props.max_length,
                min_length: props.min_length,
                name: props.name.clone(),
                autocomplete: props.autocomplete.clone(),
                rows: props.rows,
                cols: props.cols,
                resize: props.resize,
                auto_resize: props.auto_resize,
                max_height: props.max_height.clone(),
                max_rows: props.max_rows,
                is_composing: false,
                has_description: false,
                dir: props.dir,
                input_mode: props.input_mode,
                locale: env.locale.clone(),
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
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            Event::Change(value) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing {
                    return None;
                }

                let value = value.clone();

                let auto_resize = ctx.auto_resize;
                let mut plan = TransitionPlan::context_only({
                    let value = value.clone();
                    move |ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(value);
                        }
                    }
                })
                .with_effect(value_change_effect(value));

                if auto_resize {
                    plan = plan.with_effect(auto_resize_effect(ctx));
                }

                Some(plan)
            }

            Event::Clear => {
                if ctx.disabled || ctx.readonly {
                    return None;
                }

                let auto_resize = ctx.auto_resize;

                let mut plan = TransitionPlan::context_only(|ctx: &mut Context| {
                    if !ctx.value.is_controlled() {
                        ctx.value.set(String::new());
                    }
                })
                .with_effect(value_change_effect(String::new()));

                if auto_resize {
                    plan = plan.with_effect(auto_resize_effect(ctx));
                }

                Some(plan)
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

                let auto_resize = ctx.auto_resize;

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

                if should_change && auto_resize {
                    plan = plan.with_effect(auto_resize_effect(ctx));
                }

                Some(plan)
            }

            Event::SetValue(value) => {
                let value = value.clone();

                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(value) = value {
                        ctx.value.set(value.clone());

                        ctx.value.sync_controlled(Some(value));
                    } else {
                        ctx.value.sync_controlled(None);
                    }
                });

                if props.auto_resize {
                    plan = plan.with_effect(auto_resize_effect(ctx));
                }

                Some(plan)
            }

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.placeholder = props.placeholder;
                    ctx.max_length = props.max_length;
                    ctx.min_length = props.min_length;
                    ctx.name = props.name;
                    ctx.autocomplete = props.autocomplete;
                    ctx.rows = props.rows;
                    ctx.cols = props.cols;
                    ctx.resize = props.resize;
                    ctx.auto_resize = props.auto_resize;
                    ctx.max_height = props.max_height;
                    ctx.max_rows = props.max_rows;
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
            "textarea::Props.id must remain stable after init"
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

/// Structural parts exposed by the `Textarea` connect API.
#[derive(ComponentPart)]
#[scope = "textarea"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The native textarea element.
    Textarea,

    /// Optional character count live region.
    CharacterCount,

    /// Optional descriptive text element.
    Description,

    /// Optional validation error message element.
    ErrorMessage,
}

/// The API for the `Textarea` component.
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
            Part::Textarea => self.textarea_attrs(),
            Part::CharacterCount => self.character_count_attrs(),
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
            .set(HtmlAttr::For, self.ctx.ids.part("textarea"));

        attrs
    }

    /// Attributes for the textarea element.
    #[must_use]
    pub fn textarea_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Textarea.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("textarea"))
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr())
            .set(HtmlAttr::Rows, self.ctx.rows.to_string())
            .set(HtmlAttr::Value, self.ctx.value.get().clone())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set_style(CssProperty::Resize, self.ctx.resize.as_str());

        if let Some(input_mode) = self.ctx.input_mode {
            attrs.set(HtmlAttr::InputMode, input_mode.as_str());
        }

        if let Some(cols) = self.ctx.cols {
            attrs.set(HtmlAttr::Cols, cols.to_string());
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

    /// Attributes for the character count element.
    #[must_use]
    pub fn character_count_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CharacterCount.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Returns character-count metadata for adapter rendering.
    #[must_use]
    pub fn character_count(&self) -> CharacterCount {
        let current = grapheme_count(self.ctx.value.get());

        let max = self.ctx.max_length;

        let formatter = count_formatter(&self.ctx.locale);

        let current_text = formatter.format(current as f64);

        let text = if let Some(max) = max {
            format!("{current_text} / {}", formatter.format(f64::from(max)))
        } else {
            current_text
        };

        CharacterCount { current, max, text }
    }

    /// Attributes for the description/help text.
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

    /// Sends [`Event::Focus`] for textarea focus.
    pub fn on_textarea_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends [`Event::Blur`] for textarea blur.
    pub fn on_textarea_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::Change`] for textarea changes when no composition is active.
    pub fn on_textarea_change(&self, value: String) {
        if !self.ctx.is_composing {
            (self.send)(Event::Change(value));
        }
    }

    /// Sends [`Event::CompositionStart`] for IME composition start.
    pub fn on_textarea_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Sends composition end followed by the final committed value.
    pub fn on_textarea_composition_end(&self, final_value: String) {
        (self.send)(Event::CompositionEnd(final_value));
    }

    /// Sends [`Event::Clear`] for an adapter-level clear command.
    pub fn on_clear(&self) {
        (self.send)(Event::Clear);
    }
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.required != new.required
        || old.placeholder != new.placeholder
        || old.max_length != new.max_length
        || old.min_length != new.min_length
        || old.name != new.name
        || old.form != new.form
        || old.autocomplete != new.autocomplete
        || old.rows != new.rows
        || old.cols != new.cols
        || old.resize != new.resize
        || old.auto_resize != new.auto_resize
        || old.max_height != new.max_height
        || old.max_rows != new.max_rows
        || old.dir != new.dir
        || old.input_mode != new.input_mode
}

fn auto_resize_effect(ctx: &Context) -> PendingEffect<Machine> {
    PendingEffect::named_with_metadata(
        Effect::AutoResize,
        EffectMetadata::ResizeToContent(ResizeToContentEffect {
            element_id: ctx.ids.part("textarea"),
            max_height: ctx.max_height.clone(),
            max_rows: ctx.max_rows,
        }),
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

fn count_formatter(locale: &Locale) -> Formatter {
    Formatter::new(
        locale,
        FormatOptions {
            max_fraction_digits: 0,
            sign_display: SignDisplay::Never,
            ..FormatOptions::default()
        },
    )
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

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("bio-field")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages)
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn textarea_initial_state_is_idle() {
        let service = service(props().default_value("Hello"));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), "Hello");
        assert_eq!(service.context().rows, 3);
        assert_eq!(service.context().resize, ResizeMode::Vertical);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
        assert!(!service.context().is_composing);
        assert_eq!(service.context().ids.part("textarea"), "bio-field-textarea");
    }

    #[test]
    fn textarea_focus_and_blur_track_focus_visible() {
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
    fn textarea_change_updates_uncontrolled_value() {
        let mut service = service(props());

        drop(service.send(Event::Change("Bio".to_string())));

        assert_eq!(service.context().value.get(), "Bio");
    }

    #[test]
    fn textarea_change_noops_when_disabled_readonly_or_composing() {
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
    fn textarea_controlled_value_syncs_from_props() {
        let mut service = service(props().value("parent"));

        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), "parent");

        drop(service.set_props(props().value("updated")));

        assert_eq!(service.context().value.get(), "updated");

        drop(service.set_props(props().uncontrolled()));

        assert!(!service.context().value.is_controlled());
    }

    #[test]
    fn textarea_set_props_syncs_output_affecting_context() {
        let mut service = service(
            props()
                .placeholder("Bio")
                .max_length(400)
                .min_length(2)
                .name("bio")
                .form("profile")
                .autocomplete("off")
                .required(true)
                .readonly(true)
                .rows(6)
                .cols(40)
                .auto_resize(true)
                .max_height("240px")
                .max_rows(8)
                .input_mode(InputMode::Text),
        );

        drop(
            service.set_props(
                props()
                    .disabled(true)
                    .invalid(true)
                    .placeholder("Summary")
                    .no_max_length()
                    .no_min_length()
                    .no_name()
                    .no_form()
                    .no_autocomplete()
                    .rows(4)
                    .no_cols()
                    .resize(ResizeMode::Horizontal)
                    .auto_resize(false)
                    .no_max_height()
                    .no_max_rows()
                    .dir(Direction::Rtl)
                    .no_input_mode(),
            ),
        );

        assert!(service.context().disabled);
        assert!(!service.context().readonly);
        assert!(service.context().invalid);
        assert!(!service.context().required);
        assert_eq!(service.context().placeholder.as_deref(), Some("Summary"));
        assert_eq!(service.context().max_length, None);
        assert_eq!(service.context().min_length, None);
        assert_eq!(service.context().name, None);
        assert_eq!(service.context().autocomplete, None);
        assert_eq!(service.context().rows, 4);
        assert_eq!(service.context().cols, None);
        assert_eq!(service.context().resize, ResizeMode::Horizontal);
        assert!(!service.context().auto_resize);
        assert_eq!(service.context().max_height, None);
        assert_eq!(service.context().max_rows, None);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().input_mode, None);

        let attrs = service.connect(&|_| {}).textarea_attrs();

        assert!(attrs.contains(&HtmlAttr::Disabled));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Placeholder), Some("Summary"));
        assert_eq!(attrs.get(&HtmlAttr::Rows), Some("4"));
        assert_eq!(attrs.get(&HtmlAttr::Cols), None);
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Form), None);
    }

    #[test]
    fn textarea_props_output_changed_covers_each_field() {
        let base = props();

        assert!(!props_output_changed(&base, &base.clone()));

        for next in [
            base.clone().disabled(true),
            base.clone().readonly(true),
            base.clone().invalid(true),
            base.clone().required(true),
            base.clone().placeholder("Bio"),
            base.clone().max_length(400),
            base.clone().min_length(2),
            base.clone().name("bio"),
            base.clone().form("profile"),
            base.clone().autocomplete("off"),
            base.clone().rows(6),
            base.clone().cols(40),
            base.clone().resize(ResizeMode::Horizontal),
            base.clone().auto_resize(true),
            base.clone().max_height("240px"),
            base.clone().max_rows(8),
            base.clone().dir(Direction::Rtl),
            base.clone().input_mode(InputMode::Text),
        ] {
            assert!(props_output_changed(&base, &next));
        }
    }

    #[test]
    fn textarea_builder_clearers_are_covered() {
        let props = props()
            .rows(8)
            .placeholder("Bio")
            .no_placeholder()
            .on_value_change(callback(|_: String| {}))
            .no_value_change();

        assert_eq!(props.id, "bio-field");
        assert_eq!(props.rows, 8);
        assert_eq!(props.placeholder, None);
        assert_eq!(props.on_value_change, None);
    }

    #[test]
    fn textarea_clear_respects_disabled_and_readonly() {
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
    fn textarea_invalid_and_description_drive_describedby() {
        let mut service = service(props());

        drop(service.send(Event::SetHasDescription(true)));

        let attrs = service.connect(&|_| {}).textarea_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("bio-field-description")
        );

        drop(service.send(Event::SetInvalid(true)));

        let attrs = service.connect(&|_| {}).textarea_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("bio-field-description bio-field-error-message")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
    }

    #[test]
    fn textarea_api_helpers_suppress_and_commit_ime_changes() {
        let sent = core::cell::RefCell::new(Vec::new());
        let send = |event| sent.borrow_mut().push(event);

        let idle_service = service(props());

        let api = idle_service.connect(&send);

        api.on_textarea_focus(true);
        api.on_textarea_blur();
        api.on_textarea_change("typed".to_string());
        api.on_clear();
        api.on_textarea_composition_start();

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

        api.on_textarea_change("ignored".to_string());

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

        api.on_textarea_composition_end("final".to_string());

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
    fn textarea_composition_end_allows_final_value() {
        let mut service = service(props());

        drop(service.send(Event::CompositionStart));
        drop(service.send(Event::CompositionEnd("こんにちは".to_string())));

        assert!(!service.context().is_composing);
        assert_eq!(service.context().value.get(), "こんにちは");
    }

    #[test]
    fn textarea_composition_end_clears_composing_without_change_when_blocked() {
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
    fn textarea_blocked_composition_end_does_not_auto_resize() {
        let mut service = service(
            props()
                .disabled(true)
                .auto_resize(true)
                .max_height("240px")
                .default_value("before"),
        );

        drop(service.send(Event::CompositionStart));

        let result = service.send(Event::CompositionEnd("after".to_string()));

        assert!(result.context_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context().value.get(), "before");
        assert!(!service.context().is_composing);
    }

    #[test]
    fn textarea_clear_and_composition_end_emit_controlled_value_effects() {
        let mut service = service(props().value("parent").auto_resize(true));

        let send: StrongSend<Event> = Arc::new(|_| {});

        let clear = service.send(Event::Clear);

        assert_eq!(service.context().value.get(), "parent");
        assert_eq!(clear.pending_effects.len(), 2);

        for effect in clear.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        drop(service.send(Event::CompositionStart));

        let composition_end = service.send(Event::CompositionEnd("final".to_string()));

        assert_eq!(service.context().value.get(), "parent");
        assert_eq!(composition_end.pending_effects.len(), 2);

        for effect in composition_end.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    #[test]
    fn textarea_value_change_effect_fires_for_controlled_and_uncontrolled_changes() {
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
    fn textarea_auto_resize_effect_is_emitted_only_when_enabled() {
        let mut plain_service = service(props());

        let result = plain_service.send(Event::Change("plain".to_string()));

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);

        let mut resize_service = service(
            props()
                .auto_resize(true)
                .max_height("240px")
                .max_rows(8)
                .default_value("old"),
        );

        let result = resize_service.send(Event::Change("resized".to_string()));

        assert_eq!(result.pending_effects.len(), 2);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);
        assert_eq!(result.pending_effects[1].name, Effect::AutoResize);
        assert_eq!(
            result.pending_effects[1].metadata,
            Some(EffectMetadata::ResizeToContent(ResizeToContentEffect {
                element_id: "bio-field-textarea".to_string(),
                max_height: Some("240px".to_string()),
                max_rows: Some(8),
            }))
        );
        assert_eq!(
            resize_service.context().ids.part("textarea"),
            "bio-field-textarea"
        );
        assert_eq!(
            resize_service.context().max_height.as_deref(),
            Some("240px")
        );
        assert_eq!(resize_service.context().max_rows, Some(8));

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(
                resize_service.context(),
                resize_service.props(),
                Arc::clone(&send),
            ));
        }

        let result = resize_service.set_props(
            props()
                .auto_resize(true)
                .max_height("240px")
                .max_rows(8)
                .value("parent"),
        );

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AutoResize);
        assert_eq!(
            result.pending_effects[0].metadata,
            Some(EffectMetadata::ResizeToContent(ResizeToContentEffect {
                element_id: "bio-field-textarea".to_string(),
                max_height: Some("240px".to_string()),
                max_rows: Some(8),
            }))
        );

        let result = resize_service.set_props(
            props()
                .auto_resize(true)
                .max_height("320px")
                .max_rows(12)
                .value("parent-updated"),
        );

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AutoResize);
        assert_eq!(
            result.pending_effects[0].metadata,
            Some(EffectMetadata::ResizeToContent(ResizeToContentEffect {
                element_id: "bio-field-textarea".to_string(),
                max_height: Some("320px".to_string()),
                max_rows: Some(12),
            }))
        );

        let result = resize_service.send(Event::Clear);

        assert_eq!(result.pending_effects.len(), 2);
        assert_eq!(result.pending_effects[1].name, Effect::AutoResize);

        drop(resize_service.send(Event::CompositionStart));

        let result = resize_service.send(Event::CompositionEnd("final".to_string()));

        assert_eq!(result.pending_effects.len(), 2);
        assert_eq!(result.pending_effects[1].name, Effect::AutoResize);
    }

    #[test]
    fn textarea_attrs_cover_output_props_and_resize_modes() {
        for (resize, token) in [
            (ResizeMode::None, "none"),
            (ResizeMode::Both, "both"),
            (ResizeMode::Horizontal, "horizontal"),
            (ResizeMode::Vertical, "vertical"),
        ] {
            let attrs = service(props().resize(resize))
                .connect(&|_| {})
                .textarea_attrs();

            assert_eq!(
                attrs
                    .iter_styles()
                    .find(|(prop, _)| *prop == CssProperty::Resize)
                    .map(|(_, value)| value.as_str()),
                Some(token)
            );
        }

        let attrs = service(
            props()
                .value("value")
                .placeholder("Bio")
                .max_length(400)
                .min_length(2)
                .autocomplete("off")
                .name("bio")
                .form("profile")
                .required(true)
                .readonly(true)
                .input_mode(InputMode::Text)
                .dir(Direction::Rtl)
                .rows(5)
                .cols(30),
        )
        .connect(&|_| {})
        .textarea_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("bio"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("profile"));
        assert_eq!(attrs.get(&HtmlAttr::MaxLength), Some("400"));
        assert_eq!(attrs.get(&HtmlAttr::MinLength), Some("2"));
        assert_eq!(attrs.get(&HtmlAttr::AutoComplete), Some("off"));
        assert_eq!(attrs.get(&HtmlAttr::InputMode), Some("text"));
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Rows), Some("5"));
        assert_eq!(attrs.get(&HtmlAttr::Cols), Some("30"));
        assert!(attrs.get(&HtmlAttr::Required).is_some());
        assert!(attrs.get(&HtmlAttr::ReadOnly).is_some());
    }

    #[test]
    fn textarea_character_count_attrs_are_live_and_atomic() {
        let textarea_service = service(
            props()
                .default_value("e\u{301}👨\u{200d}👩\u{200d}👧")
                .max_length(10),
        );

        let api = textarea_service.connect(&|_| {});

        let attrs = api.character_count_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(
            api.character_count(),
            CharacterCount {
                current: 2,
                max: Some(10),
                text: "2 / 10".to_string(),
            }
        );

        let no_max_service = service(props().default_value("abc"));

        assert_eq!(
            no_max_service.connect(&|_| {}).character_count(),
            CharacterCount {
                current: 3,
                max: None,
                text: "3".to_string(),
            }
        );
    }

    #[test]
    fn textarea_count_formatter_suppresses_sign_and_fraction_digits() {
        let formatter = count_formatter(&Locale::parse("en-US").expect("locale should parse"));

        assert_eq!(formatter.format(-1.25), "1");
    }

    #[test]
    fn textarea_resize_display_is_covered() {
        assert_eq!(ResizeMode::Horizontal.to_string(), "horizontal");
    }

    #[test]
    fn textarea_part_attrs_delegate_for_all_parts() {
        let textarea_service = service(props());

        let api = textarea_service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Textarea), api.textarea_attrs());
        assert_eq!(
            api.part_attrs(Part::CharacterCount),
            api.character_count_attrs()
        );
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
    }

    #[test]
    fn textarea_api_debug_redacts_sender() {
        let textarea_service = service(props());

        let api = textarea_service.connect(&|_| {});

        assert!(format!("{api:?}").contains("send: \"<callback>\""));
    }

    #[test]
    fn textarea_snapshots() {
        let mut focused = service(props());

        drop(focused.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "textarea_root_idle",
            snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "textarea_root_focused_keyboard",
            snapshot_attrs(&focused.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "textarea_root_disabled_readonly_invalid",
            snapshot_attrs(
                &service(props().disabled(true).readonly(true).invalid(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
        assert_snapshot!(
            "textarea_label",
            snapshot_attrs(&service(props()).connect(&|_| {}).label_attrs())
        );
        assert_snapshot!(
            "textarea_default",
            snapshot_attrs(&service(props()).connect(&|_| {}).textarea_attrs())
        );
        assert_snapshot!(
            "textarea_form_constraints",
            snapshot_attrs(
                &service(
                    props()
                        .value("Hello")
                        .placeholder("Biography")
                        .max_length(500)
                        .min_length(10)
                        .autocomplete("off")
                        .name("bio")
                        .form("profile")
                        .required(true)
                        .rows(6)
                        .cols(40)
                )
                .connect(&|_| {})
                .textarea_attrs()
            )
        );

        let mut described = service(props().invalid(true));

        drop(described.send(Event::SetHasDescription(true)));

        assert_snapshot!(
            "textarea_described_invalid",
            snapshot_attrs(&described.connect(&|_| {}).textarea_attrs())
        );

        for resize in [
            ResizeMode::None,
            ResizeMode::Both,
            ResizeMode::Horizontal,
            ResizeMode::Vertical,
        ] {
            assert_snapshot!(
                format!("textarea_resize_{}", resize.as_str()),
                snapshot_attrs(
                    &service(props().resize(resize))
                        .connect(&|_| {})
                        .textarea_attrs()
                )
            );
        }

        assert_snapshot!(
            "textarea_character_count",
            snapshot_attrs(&service(props()).connect(&|_| {}).character_count_attrs())
        );
        assert_snapshot!(
            "textarea_description",
            snapshot_attrs(&service(props()).connect(&|_| {}).description_attrs())
        );
        assert_snapshot!(
            "textarea_error_message",
            snapshot_attrs(&service(props()).connect(&|_| {}).error_message_attrs())
        );
    }
}
