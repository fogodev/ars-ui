//! PinInput component state machine and connect API.
//!
//! This module implements the framework-agnostic `PinInput` machine defined in
//! `spec/components/input/pin-input.md`. PinInput renders a row of single-
//! character `<input>` cells for PIN, OTP, or verification-code entry, plus a
//! hidden `<input type="hidden">` that carries the concatenated value into
//! form submission. The agnostic core owns per-cell values, paste
//! distribution, focus-index intent, and ARIA/data attrs; moving DOM focus
//! between cells is an adapter concern surfaced via [`Effect::FocusCell`].

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The states for the `PinInput` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No cell is focused and the pin is not yet complete.
    Idle,

    /// A cell is focused. `index` identifies which cell holds focus.
    Focused {
        /// The index of the focused cell.
        index: usize,
    },

    /// All cells are filled.
    Completed,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused { .. } => "focused",
            Self::Completed => "completed",
        })
    }
}

/// The events for the `PinInput` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A cell received focus.
    Focus {
        /// The index of the focused cell.
        index: usize,

        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// All cells lost focus.
    Blur,

    /// A character was input into a specific cell.
    InputChar {
        /// The index of the cell that received input.
        index: usize,

        /// The character that was input.
        char: char,
    },

    /// A character was deleted from a specific cell.
    DeleteChar {
        /// The index of the cell whose character should be deleted.
        index: usize,
    },

    /// Text was pasted into the group.
    Paste(String),

    /// All cells were cleared.
    Clear,

    /// Focus the next cell (`ArrowRight`).
    FocusNext,

    /// Focus the previous cell (`ArrowLeft`).
    FocusPrev,

    /// IME composition started.
    CompositionStart,

    /// IME composition ended.
    CompositionEnd,
}

/// The input validation mode for the `PinInput` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Only digits accepted.
    Numeric,

    /// Letters and digits accepted.
    Alphanumeric,

    /// Any character accepted; cells render masked.
    Password,
}

/// The context for the `PinInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled per-cell value (one slot per cell).
    pub value: Bindable<Vec<String>>,

    /// Number of cells in the group.
    pub length: usize,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is an OTP input (`autocomplete="one-time-code"`).
    pub otp: bool,

    /// Whether each cell renders as `<input type="password">`.
    pub mask: bool,

    /// Placeholder character for empty cells.
    pub placeholder: Option<String>,

    /// The index of the focused cell, if any.
    pub focused_index: Option<usize>,

    /// Whether the focus is visible (keyboard-initiated).
    pub focus_visible: bool,

    /// Whether all cells are non-empty.
    pub complete: bool,

    /// The input validation mode.
    pub mode: Mode,

    /// The `name` attribute used by the hidden input for form submission.
    pub name: Option<String>,

    /// When `true`, adapters call `.select()` on the cell input on focus.
    pub select_on_focus: bool,

    /// When `true`, blur the group after the final cell is filled.
    pub blur_on_complete: bool,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// Whether a Description part is rendered (gates `aria-describedby`).
    pub has_description: bool,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The props for the `PinInput` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the pin-input root.
    pub id: String,

    /// Controlled value. When `Some`, component is controlled.
    pub value: Option<Vec<String>>,

    /// Default value for uncontrolled mode. Resized to `length` on init.
    pub default_value: Vec<String>,

    /// Number of cells in the group.
    pub length: usize,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is an OTP input.
    pub otp: bool,

    /// Whether each cell is rendered as a masked password input.
    pub mask: bool,

    /// Placeholder character for empty cells.
    pub placeholder: Option<String>,

    /// The input validation mode.
    pub mode: Mode,

    /// The `name` attribute for form submission (via the hidden input).
    pub name: Option<String>,

    /// The ID of the form element the hidden input is associated with.
    pub form: Option<String>,

    /// Whether the pin input is required.
    pub required: bool,

    /// Whether the pin input is read-only.
    pub readonly: bool,

    /// Whether each cell's content is selected when it receives focus.
    pub select_on_focus: bool,

    /// Whether the component blurs after all cells are filled.
    pub blur_on_complete: bool,

    /// Whether [`Event::InputChar`] that fills the final cell additionally
    /// fires the [`Self::on_value_complete`] callback.
    pub auto_submit: bool,

    /// Callback fired when all cells are filled. The argument is the
    /// concatenated string formed by joining every cell value left-to-right.
    pub on_value_complete: Option<Callback<dyn Fn(String) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: Vec::new(),
            length: 6,
            disabled: false,
            invalid: false,
            otp: false,
            mask: false,
            placeholder: None,
            mode: Mode::Numeric,
            name: None,
            form: None,
            required: false,
            readonly: false,
            select_on_focus: false,
            blur_on_complete: false,
            auto_submit: false,
            on_value_complete: None,
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
    pub fn value(mut self, value: Vec<String>) -> Self {
        self.value = Some(value);
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
    pub fn default_value(mut self, value: Vec<String>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`length`](Self::length).
    #[must_use]
    pub const fn length(mut self, value: usize) -> Self {
        self.length = value;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`otp`](Self::otp).
    #[must_use]
    pub const fn otp(mut self, value: bool) -> Self {
        self.otp = value;
        self
    }

    /// Sets [`mask`](Self::mask).
    #[must_use]
    pub const fn mask(mut self, value: bool) -> Self {
        self.mask = value;
        self
    }

    /// Sets [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Sets [`mode`](Self::mode).
    #[must_use]
    pub const fn mode(mut self, value: Mode) -> Self {
        self.mode = value;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, value: impl Into<String>) -> Self {
        self.form = Some(value.into());
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`select_on_focus`](Self::select_on_focus).
    #[must_use]
    pub const fn select_on_focus(mut self, value: bool) -> Self {
        self.select_on_focus = value;
        self
    }

    /// Sets [`blur_on_complete`](Self::blur_on_complete).
    #[must_use]
    pub const fn blur_on_complete(mut self, value: bool) -> Self {
        self.blur_on_complete = value;
        self
    }

    /// Sets [`auto_submit`](Self::auto_submit).
    #[must_use]
    pub const fn auto_submit(mut self, value: bool) -> Self {
        self.auto_submit = value;
        self
    }

    /// Sets [`on_value_complete`](Self::on_value_complete).
    #[must_use]
    pub fn on_value_complete(
        mut self,
        callback: impl Into<Callback<dyn Fn(String) + Send + Sync>>,
    ) -> Self {
        self.on_value_complete = Some(callback.into());
        self
    }
}

/// Type alias for the [`Messages::ordinal_label`] message closure.
pub type OrdinalLabelFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Locale-specific labels for the `PinInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Ordinal label for a single cell. Receives `(position, total, locale)`
    /// and returns the localized accessible name (e.g. `"Digit 2 of 6"`).
    pub ordinal_label: MessageFn<OrdinalLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            ordinal_label: MessageFn::new(|pos: usize, total: usize, _locale: &Locale| -> String {
                alloc::format!("Digit {pos} of {total}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for the named effect intents the `pin_input` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter focuses the cell at [`Context::focused_index`] (e.g. after
    /// auto-advance, backspace navigation, or `FocusPrev`/`FocusNext`).
    FocusCell,

    /// Adapter fires [`Props::on_value_complete`] with the joined value when
    /// `auto_submit` is `true`.
    ValueComplete,

    /// Adapter moves DOM focus to `document.body` after the last cell is
    /// filled when `blur_on_complete` is `true`.
    BlurOnComplete,
}

/// The machine for the `PinInput` component.
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
        let length = props.length;

        let mut initial = props
            .value
            .clone()
            .unwrap_or_else(|| props.default_value.clone());

        initial.resize(length, String::new());

        let complete = length > 0 && initial.iter().all(|cell| !cell.is_empty());

        let state = if complete {
            State::Completed
        } else {
            State::Idle
        };

        let bindable = if props.value.is_some() {
            Bindable::controlled(initial.clone())
        } else {
            Bindable::uncontrolled(initial)
        };

        (
            state,
            Context {
                value: bindable,
                length,
                disabled: props.disabled,
                invalid: props.invalid,
                otp: props.otp,
                mask: props.mask,
                placeholder: props.placeholder.clone(),
                focused_index: None,
                focus_visible: false,
                complete,
                mode: props.mode,
                name: props.name.clone(),
                select_on_focus: props.select_on_focus,
                blur_on_complete: props.blur_on_complete,
                is_composing: false,
                has_description: false,
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
        if ctx.disabled {
            match event {
                Event::InputChar { .. }
                | Event::DeleteChar { .. }
                | Event::Paste(_)
                | Event::Clear => return None,
                _ => {}
            }
        }

        if props.readonly {
            match event {
                Event::InputChar { .. } | Event::DeleteChar { .. } | Event::Paste(_) => {
                    return None;
                }
                _ => {}
            }
        }

        match (state, event) {
            (_, Event::Focus { index, is_keyboard }) => {
                if *index >= ctx.length {
                    return None;
                }

                let index = *index;
                let is_keyboard = *is_keyboard;

                Some(TransitionPlan::to(State::Focused { index }).apply(
                    move |ctx: &mut Context| {
                        ctx.focused_index = Some(index);
                        ctx.focus_visible = is_keyboard;
                    },
                ))
            }

            (_, Event::Blur) => {
                let was_complete = ctx.complete;

                Some(
                    TransitionPlan::to(if was_complete {
                        State::Completed
                    } else {
                        State::Idle
                    })
                    .apply(|ctx: &mut Context| {
                        ctx.focused_index = None;
                        ctx.focus_visible = false;
                    }),
                )
            }

            (State::Focused { .. }, Event::InputChar { index, char: ch })
            | (State::Completed, Event::InputChar { index, char: ch }) => {
                let index = *index;
                let ch = *ch;

                if index >= ctx.length || !ctx.mode.accepts(ch) {
                    return None;
                }

                let mut values = ctx.value.get().clone();

                values[index] = ch.to_string();

                let now_complete = values.iter().all(|cell| !cell.is_empty());

                if now_complete {
                    let joined = values.join("");

                    let auto_submit = props.auto_submit;
                    let blur_on_complete = ctx.blur_on_complete;

                    let mut plan =
                        TransitionPlan::to(State::Completed).apply(move |ctx: &mut Context| {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(values);
                            }
                            ctx.complete = true;
                        });

                    if auto_submit {
                        plan = plan.with_effect(value_complete_effect(joined));
                    }

                    if blur_on_complete {
                        plan = plan.with_effect(PendingEffect::named(Effect::BlurOnComplete));
                    }

                    Some(plan)
                } else {
                    let next_index = next_empty_index(&values, index).unwrap_or(index);

                    Some(
                        TransitionPlan::to(State::Focused { index: next_index })
                            .apply(move |ctx: &mut Context| {
                                if !ctx.value.is_controlled() {
                                    ctx.value.set(values);
                                }

                                ctx.focused_index = Some(next_index);
                                ctx.complete = false;
                            })
                            .with_effect(PendingEffect::named(Effect::FocusCell)),
                    )
                }
            }

            (_, Event::DeleteChar { index }) => {
                let index = *index;

                if index >= ctx.length {
                    return None;
                }

                let cell_empty = ctx.value.get()[index].is_empty();

                if cell_empty && index > 0 {
                    let prev = index - 1;

                    Some(
                        TransitionPlan::to(State::Focused { index: prev })
                            .apply(move |ctx: &mut Context| {
                                let mut values = ctx.value.get().clone();

                                values[prev] = String::new();

                                if !ctx.value.is_controlled() {
                                    ctx.value.set(values);
                                }

                                ctx.focused_index = Some(prev);
                                ctx.complete = false;
                            })
                            .with_effect(PendingEffect::named(Effect::FocusCell)),
                    )
                } else {
                    let mut values = ctx.value.get().clone();

                    values[index] = String::new();

                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(values);
                        }

                        ctx.complete = false;
                    }))
                }
            }

            (_, Event::Paste(text)) => {
                let mode = ctx.mode;
                let length = ctx.length;
                let start = ctx.focused_index.unwrap_or(0);

                let filtered: Vec<char> = text
                    .chars()
                    .filter(|ch| mode.accepts(*ch))
                    .take(length.saturating_sub(start))
                    .collect();

                if filtered.is_empty() {
                    return None;
                }

                let mut values = ctx.value.get().clone();

                for (offset, ch) in filtered.iter().enumerate() {
                    let pos = start + offset;

                    if pos < length {
                        values[pos] = ch.to_string();
                    }
                }

                let now_complete = values.iter().all(|cell| !cell.is_empty());

                if now_complete {
                    let joined = values.join("");

                    let auto_submit = props.auto_submit;
                    let blur_on_complete = ctx.blur_on_complete;

                    let mut plan =
                        TransitionPlan::to(State::Completed).apply(move |ctx: &mut Context| {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(values);
                            }

                            ctx.complete = true;
                        });

                    if auto_submit {
                        plan = plan.with_effect(value_complete_effect(joined));
                    }

                    if blur_on_complete {
                        plan = plan.with_effect(PendingEffect::named(Effect::BlurOnComplete));
                    }

                    Some(plan)
                } else {
                    let next_index = next_empty_index(&values, start).unwrap_or(start);

                    Some(
                        TransitionPlan::context_only(move |ctx: &mut Context| {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(values);
                            }

                            ctx.focused_index = Some(next_index);
                            ctx.complete = false;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusCell)),
                    )
                }
            }

            (_, Event::Clear) => {
                let length = ctx.length;

                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(alloc::vec![String::new(); length]);
                        }

                        ctx.complete = false;
                        ctx.focused_index = None;
                        ctx.focus_visible = false;
                    }),
                )
            }

            (_, Event::FocusPrev) => {
                let current = ctx.focused_index?;

                if current == 0 {
                    return None;
                }

                let prev = current - 1;

                Some(
                    TransitionPlan::to(State::Focused { index: prev })
                        .apply(move |ctx: &mut Context| {
                            ctx.focused_index = Some(prev);
                        })
                        .with_effect(PendingEffect::named(Effect::FocusCell)),
                )
            }

            (_, Event::FocusNext) => {
                let current = ctx.focused_index?;

                if current + 1 >= ctx.length {
                    return None;
                }

                let next = current + 1;

                Some(
                    TransitionPlan::to(State::Focused { index: next })
                        .apply(move |ctx: &mut Context| {
                            ctx.focused_index = Some(next);
                        })
                        .with_effect(PendingEffect::named(Effect::FocusCell)),
                )
            }

            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = true;
                }))
            }

            (_, Event::CompositionEnd) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = false;
                }))
            }

            (State::Idle, Event::InputChar { .. }) => None,
        }
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

impl Mode {
    /// Returns `true` if the given character is allowed under this mode.
    #[must_use]
    pub const fn accepts(self, ch: char) -> bool {
        match self {
            Self::Numeric => ch.is_ascii_digit(),
            Self::Alphanumeric => ch.is_ascii_alphanumeric(),
            Self::Password => true,
        }
    }
}

fn next_empty_index(values: &[String], current: usize) -> Option<usize> {
    let length = values.len();

    (current + 1..length).find(|j| values[*j].is_empty())
}

fn value_complete_effect(joined: String) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueComplete,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_complete {
                callback(joined);
            }

            no_cleanup()
        },
    )
}

/// Structural parts exposed by the `PinInput` connect API.
#[derive(ComponentPart)]
#[scope = "pin-input"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible group label element.
    Label,

    /// A single cell input element. `cell_index` is the zero-based cell index.
    Input {
        /// The zero-based cell index.
        cell_index: usize,
    },

    /// The hidden input that carries the concatenated value into form submission.
    HiddenInput,

    /// The optional descriptive help-text element.
    Description,

    /// The optional validation error message element.
    ErrorMessage,
}

/// The API for the `PinInput` component.
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
            Part::Input { cell_index } => self.input_attrs(cell_index),
            Part::HiddenInput => self.hidden_input_attrs(),
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
            .set(HtmlAttr::Role, "group")
            .set(HtmlAttr::Data("ars-state"), self.state.to_string())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        set_described_by(&mut attrs, self.ctx);

        attrs
    }

    /// Attributes for the group label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Attributes for a single cell input.
    #[must_use]
    pub fn input_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Input { cell_index: index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.item("input", &index))
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(
                HtmlAttr::Type,
                if self.ctx.mask || self.ctx.mode == Mode::Password {
                    "password"
                } else {
                    "text"
                },
            )
            .set(
                HtmlAttr::InputMode,
                match self.ctx.mode {
                    Mode::Numeric => "numeric",
                    Mode::Alphanumeric | Mode::Password => "text",
                },
            )
            .set(HtmlAttr::MaxLength, "1")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.ordinal_label)(index + 1, self.ctx.length, &self.ctx.locale),
            );

        let is_focused_cell = self.ctx.focused_index == Some(index);

        attrs.set(HtmlAttr::TabIndex, if is_focused_cell { "0" } else { "-1" });

        if self.ctx.otp {
            attrs.set(HtmlAttr::AutoComplete, "one-time-code");
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

        if self.props.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if let Some(value) = self.ctx.value.get().get(index)
            && !value.is_empty()
        {
            attrs.set(HtmlAttr::Value, value.clone());
        }

        if let Some(placeholder) = &self.ctx.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.clone());
        }

        attrs
    }

    /// Attributes for the hidden input that carries the joined value into
    /// form submission.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        let combined = self.ctx.value.get().join("");

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Value, combined)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::TabIndex, "-1");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        if self.props.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

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

    /// Sends [`Event::Focus`] for a cell focus.
    pub fn on_cell_focus(&self, index: usize, is_keyboard: bool) {
        (self.send)(Event::Focus { index, is_keyboard });
    }

    /// Sends [`Event::Blur`] for cell blur.
    pub fn on_cell_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::InputChar`] for a cell input event.
    pub fn on_cell_input(&self, index: usize, ch: char) {
        (self.send)(Event::InputChar { index, char: ch });
    }

    /// Sends [`Event::Paste`] for a paste event.
    pub fn on_paste(&self, text: String) {
        (self.send)(Event::Paste(text));
    }

    /// Handles normalized keydown data on a single cell.
    ///
    /// Returns `true` when the key was handled by the core machine.
    pub fn on_cell_keydown(&self, index: usize, data: &KeyboardEventData) -> bool {
        if data.is_composing {
            return false;
        }

        let event = match data.key {
            KeyboardKey::ArrowLeft => Event::FocusPrev,
            KeyboardKey::ArrowRight => Event::FocusNext,
            KeyboardKey::Backspace | KeyboardKey::Delete => Event::DeleteChar { index },
            _ => return false,
        };

        (self.send)(event);

        true
    }
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
    use alloc::{string::ToString, vec};

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, callback};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("pin").length(4)
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn keyboard_event(key: KeyboardKey, is_composing: bool) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing,
        }
    }

    #[test]
    fn pin_input_initial_state_is_idle_with_empty_cells() {
        let svc = service(props());

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(svc.context().length, 4);
        assert_eq!(svc.context().value.get().len(), 4);
        assert!(svc.context().value.get().iter().all(String::is_empty));
        assert!(!svc.context().complete);
        assert_eq!(svc.context().focused_index, None);
    }

    #[test]
    fn pin_input_initial_state_completed_when_default_value_full() {
        let svc =
            service(
                props()
                    .length(3)
                    .default_value(vec!["1".into(), "2".into(), "3".into()]),
            );

        assert_eq!(svc.state(), &State::Completed);
        assert!(svc.context().complete);
    }

    #[test]
    fn pin_input_focus_cell_transitions_to_focused_index() {
        let mut svc = service(props());

        let result = svc.send(Event::Focus {
            index: 2,
            is_keyboard: true,
        });

        assert!(result.state_changed);
        assert_eq!(svc.state(), &State::Focused { index: 2 });
        assert_eq!(svc.context().focused_index, Some(2));
        assert!(svc.context().focus_visible);
    }

    #[test]
    fn pin_input_focus_with_out_of_range_index_is_ignored() {
        let mut svc = service(props());

        let result = svc.send(Event::Focus {
            index: 99,
            is_keyboard: true,
        });

        assert!(!result.state_changed);
        assert_eq!(svc.state(), &State::Idle);
    }

    #[test]
    fn pin_input_input_char_advances_to_next_cell() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::InputChar {
            index: 0,
            char: '1',
        });

        assert_eq!(svc.context().value.get()[0], "1");
        assert_eq!(svc.state(), &State::Focused { index: 1 });
        assert_eq!(svc.context().focused_index, Some(1));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::FocusCell);
    }

    #[test]
    fn pin_input_input_char_filtered_by_numeric_mode() {
        let mut svc = service(props().mode(Mode::Numeric));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::InputChar {
            index: 0,
            char: 'a',
        });

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get()[0], "");
    }

    #[test]
    fn pin_input_input_char_accepted_by_alphanumeric_mode() {
        let mut svc = service(props().mode(Mode::Alphanumeric));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        drop(svc.send(Event::InputChar {
            index: 0,
            char: 'a',
        }));

        assert_eq!(svc.context().value.get()[0], "a");
    }

    #[test]
    fn pin_input_delete_char_clears_cell_in_place() {
        let mut svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), String::new()]));

        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));

        drop(svc.send(Event::DeleteChar { index: 2 }));

        assert_eq!(svc.context().value.get()[2], "");
    }

    #[test]
    fn pin_input_delete_char_on_empty_cell_moves_focus_to_prev() {
        let mut svc = service(props().default_value(vec![
            "1".into(),
            "2".into(),
            String::new(),
            String::new(),
        ]));

        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));

        let result = svc.send(Event::DeleteChar { index: 2 });

        assert_eq!(svc.context().value.get()[1], "");
        assert_eq!(svc.state(), &State::Focused { index: 1 });
        assert_eq!(svc.context().focused_index, Some(1));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::FocusCell);
    }

    #[test]
    fn pin_input_paste_distributes_chars_across_cells() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        drop(svc.send(Event::Paste("1234".to_string())));

        assert_eq!(svc.context().value.get(), &vec!["1", "2", "3", "4"]);
        assert_eq!(svc.state(), &State::Completed);
        assert!(svc.context().complete);
    }

    #[test]
    fn pin_input_paste_filters_by_numeric_mode() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        drop(svc.send(Event::Paste("1a2b3c4".to_string())));

        assert_eq!(svc.context().value.get(), &vec!["1", "2", "3", "4"]);
    }

    #[test]
    fn pin_input_paste_partial_keeps_focus_and_advances() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::Paste("12".to_string()));

        assert_eq!(svc.context().value.get()[0], "1");
        assert_eq!(svc.context().value.get()[1], "2");
        assert_eq!(svc.context().focused_index, Some(2));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::FocusCell);
    }

    #[test]
    fn pin_input_complete_fires_value_complete_effect_when_auto_submit() {
        let mut svc = service(props().auto_submit(true));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::Paste("1234".to_string()));

        let names: Vec<Effect> = result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect();

        assert!(names.contains(&Effect::ValueComplete));
    }

    #[test]
    fn pin_input_complete_without_auto_submit_skips_value_complete_effect() {
        let mut svc = service(props().auto_submit(false));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::Paste("1234".to_string()));

        let names: Vec<Effect> = result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect();

        assert!(!names.contains(&Effect::ValueComplete));
    }

    #[test]
    fn pin_input_blur_on_complete_emits_blur_effect() {
        let mut svc = service(props().blur_on_complete(true));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::Paste("1234".to_string()));

        let names: Vec<Effect> = result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect();

        assert!(names.contains(&Effect::BlurOnComplete));
    }

    #[test]
    fn pin_input_clear_resets_cells_and_state() {
        let mut svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        drop(svc.send(Event::Clear));

        assert_eq!(svc.state(), &State::Idle);
        assert!(svc.context().value.get().iter().all(String::is_empty));
        assert!(!svc.context().complete);
        assert_eq!(svc.context().focused_index, None);
    }

    #[test]
    fn pin_input_focus_next_and_prev_navigate_between_cells() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 1,
            is_keyboard: false,
        }));

        let next = svc.send(Event::FocusNext);

        assert_eq!(svc.state(), &State::Focused { index: 2 });
        assert_eq!(next.pending_effects.len(), 1);
        assert_eq!(next.pending_effects[0].name, Effect::FocusCell);

        let prev = svc.send(Event::FocusPrev);

        assert_eq!(svc.state(), &State::Focused { index: 1 });
        assert_eq!(prev.pending_effects.len(), 1);
        assert_eq!(prev.pending_effects[0].name, Effect::FocusCell);
    }

    #[test]
    fn pin_input_focus_next_at_last_cell_does_nothing() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 3,
            is_keyboard: false,
        }));

        let result = svc.send(Event::FocusNext);

        assert!(!result.state_changed);
    }

    #[test]
    fn pin_input_focus_prev_at_first_cell_does_nothing() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::FocusPrev);

        assert!(!result.state_changed);
    }

    #[test]
    fn pin_input_disabled_blocks_input_paste_clear() {
        let mut svc = service(props().disabled(true));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        for event in [
            Event::InputChar {
                index: 0,
                char: '1',
            },
            Event::Paste("1234".to_string()),
            Event::Clear,
        ] {
            let result = svc.send(event);

            assert!(!result.context_changed);
        }
    }

    #[test]
    fn pin_input_input_cell_aria_label_uses_ordinal_message() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs(2);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Digit 3 of 4")
        );
    }

    #[test]
    fn pin_input_input_cell_uses_roving_tabindex() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));

        let api = svc.connect(&|_| {});

        for index in 0..4 {
            let attrs = api.input_attrs(index);

            let tab = attrs.get(&HtmlAttr::TabIndex);

            if index == 2 {
                assert_eq!(tab, Some("0"));
            } else {
                assert_eq!(tab, Some("-1"));
            }
        }
    }

    #[test]
    fn pin_input_input_cell_password_mode_uses_input_type_password() {
        let svc = service(props().mode(Mode::Password));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("password"));
    }

    #[test]
    fn pin_input_input_cell_mask_uses_input_type_password() {
        let svc = service(props().mask(true));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("password"));
    }

    #[test]
    fn pin_input_input_cell_otp_sets_autocomplete_one_time_code() {
        let svc = service(props().otp(true));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::AutoComplete), Some("one-time-code"));
    }

    #[test]
    fn pin_input_hidden_input_carries_joined_value() {
        let svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        let api = svc.connect(&|_| {});

        let attrs = api.hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("1234"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn pin_input_hidden_input_carries_name_form_and_required() {
        let svc = service(
            props()
                .name("pin")
                .form("login")
                .required(true)
                .default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]),
        );

        let api = svc.connect(&|_| {});

        let attrs = api.hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("pin"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("login"));
        assert!(attrs.contains(&HtmlAttr::Required));
    }

    #[test]
    fn pin_input_root_carries_role_group_and_state() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("group"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
    }

    #[test]
    fn pin_input_invalid_drives_aria_errormessage_on_cell_input() {
        let svc = service(props().invalid(true));

        let api = svc.connect(&|_| {});

        let attrs = api.input_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("pin-error-message")
        );
    }

    #[test]
    fn pin_input_root_invalid_drives_describedby() {
        let svc = service(props().invalid(true));

        let api = svc.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("pin-error-message")
        );
    }

    #[test]
    fn pin_input_keydown_emits_arrow_and_backspace_events() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(props());

        let api = svc.connect(&send);

        assert!(api.on_cell_keydown(1, &keyboard_event(KeyboardKey::ArrowLeft, false)));
        assert!(api.on_cell_keydown(1, &keyboard_event(KeyboardKey::ArrowRight, false)));
        assert!(api.on_cell_keydown(2, &keyboard_event(KeyboardKey::Backspace, false)));
        assert!(api.on_cell_keydown(2, &keyboard_event(KeyboardKey::Delete, false)));

        let events = received.borrow();

        assert_eq!(events.len(), 4);
        assert_eq!(events[0], Event::FocusPrev);
        assert_eq!(events[1], Event::FocusNext);
        assert_eq!(events[2], Event::DeleteChar { index: 2 });
        assert_eq!(events[3], Event::DeleteChar { index: 2 });
    }

    #[test]
    fn pin_input_keydown_ignores_composing_and_unknown_keys() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(props());

        let api = svc.connect(&send);

        assert!(!api.on_cell_keydown(0, &keyboard_event(KeyboardKey::ArrowLeft, true)));
        assert!(!api.on_cell_keydown(0, &keyboard_event(KeyboardKey::Escape, false)));

        assert!(received.borrow().is_empty());
    }

    #[test]
    fn pin_input_composition_lifecycle_tracks_is_composing() {
        let mut svc = service(props());

        drop(svc.send(Event::CompositionStart));

        assert!(svc.context().is_composing);

        drop(svc.send(Event::CompositionEnd));

        assert!(!svc.context().is_composing);
    }

    #[test]
    fn pin_input_part_attrs_delegates_to_each_part_method() {
        let svc = service(props().default_value(vec![
            "1".into(),
            "2".into(),
            String::new(),
            String::new(),
        ]));

        let api = svc.connect(&|_| {});

        for (part, expected) in [
            (Part::Root, snapshot_attrs(&api.root_attrs())),
            (Part::Label, snapshot_attrs(&api.label_attrs())),
            (
                Part::Input { cell_index: 0 },
                snapshot_attrs(&api.input_attrs(0)),
            ),
            (Part::HiddenInput, snapshot_attrs(&api.hidden_input_attrs())),
            (Part::Description, snapshot_attrs(&api.description_attrs())),
            (
                Part::ErrorMessage,
                snapshot_attrs(&api.error_message_attrs()),
            ),
        ] {
            assert_eq!(snapshot_attrs(&api.part_attrs(part)), expected);
        }
    }

    #[test]
    fn pin_input_event_handlers_fan_out_through_send() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(props());

        let api = svc.connect(&send);

        api.on_cell_focus(0, true);
        api.on_cell_blur();
        api.on_cell_input(0, '7');
        api.on_paste("123".to_string());

        let events = received.borrow();

        assert_eq!(events.len(), 4);
        assert_eq!(
            events[0],
            Event::Focus {
                index: 0,
                is_keyboard: true
            }
        );
        assert_eq!(events[1], Event::Blur);
        assert_eq!(
            events[2],
            Event::InputChar {
                index: 0,
                char: '7'
            }
        );
        assert_eq!(events[3], Event::Paste("123".to_string()));
    }

    #[test]
    fn pin_input_on_value_complete_callback_is_invoked_via_value_complete_effect() {
        use std::sync::Mutex;

        let received = alloc::sync::Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = alloc::sync::Arc::clone(&received);
        let mut svc = service(props().auto_submit(true).on_value_complete(callback(
            move |joined: String| {
                captured.lock().expect("mutex unpoisoned").push(joined);
            },
        )));

        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        let result = svc.send(Event::Paste("1234".to_string()));

        let send: ars_core::StrongSend<Event> = alloc::sync::Arc::new(|_| {});
        for effect in result.pending_effects {
            drop(effect.run(svc.context(), svc.props(), alloc::sync::Arc::clone(&send)));
        }

        assert_eq!(
            received.lock().expect("mutex unpoisoned").as_slice(),
            &["1234".to_string()]
        );
    }

    #[test]
    fn pin_input_root_snapshot() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn pin_input_root_completed_snapshot() {
        let svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn pin_input_root_disabled_snapshot() {
        let svc = service(props().disabled(true));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn pin_input_input_cell_unfocused_snapshot() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs(0)));
    }

    #[test]
    fn pin_input_input_cell_focused_snapshot() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 1,
            is_keyboard: true,
        }));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs(1)));
    }

    #[test]
    fn pin_input_input_cell_password_mode_snapshot() {
        let svc = service(props().mode(Mode::Password));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs(0)));
    }

    #[test]
    fn pin_input_input_cell_with_placeholder_snapshot() {
        let svc = service(props().placeholder("_"));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs(0)));
    }

    #[test]
    fn pin_input_input_cell_otp_snapshot() {
        let svc = service(props().otp(true));

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs(0)));
    }

    #[test]
    fn pin_input_hidden_input_snapshot() {
        let svc = service(
            props()
                .name("pin")
                .form("login")
                .required(true)
                .default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]),
        );

        let api = svc.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.hidden_input_attrs()));
    }
}
