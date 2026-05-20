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
    ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan, no_cleanup,
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

    /// Backspace semantics: clear the cell's character and, if the cell
    /// was already empty, move focus to the previous cell and clear that
    /// one. Emitted by Backspace keydown on a cell.
    DeleteChar {
        /// The index of the cell whose character should be deleted.
        index: usize,
    },

    /// Forward-delete semantics: clear the cell's character without
    /// navigating. Emitted by Delete keydown on a cell — Delete should
    /// never delete the *previous* cell's content the way Backspace
    /// does, otherwise pressing Delete on an empty non-first cell would
    /// erase an unrelated digit.
    ClearCell {
        /// The index of the cell whose character should be cleared.
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

    /// Synchronize the externally controlled value prop.
    SetValue(Option<Vec<String>>),

    /// Synchronize output-affecting props (`length` / `disabled` / `invalid`
    /// / `otp` / `mask` / `placeholder` / `mode` / `name` /
    /// `select_on_focus` / `blur_on_complete`) stored in [`Context`] when
    /// [`Service::set_props`] reports a change.
    SetProps,

    /// Track whether a [`Part::Description`] part is rendered (gates
    /// `aria-describedby`).
    SetHasDescription(bool),
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

    /// The text direction for the group. Used by `on_cell_keydown` to
    /// reverse `ArrowLeft`/`ArrowRight` navigation in RTL locales so
    /// arrow keys move in the visually-expected direction.
    pub dir: Direction,

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

    /// Text direction for the group. Drives RTL-aware arrow-key navigation
    /// in `on_cell_keydown`.
    pub dir: Direction,
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
            dir: Direction::Ltr,
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

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
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
                dir: props.dir,
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
                | Event::ClearCell { .. }
                | Event::Paste(_)
                | Event::Clear => return None,
                _ => {}
            }
        }

        if props.readonly {
            match event {
                Event::InputChar { .. }
                | Event::DeleteChar { .. }
                | Event::ClearCell { .. }
                | Event::Paste(_)
                | Event::Clear => {
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

                    // Deleting a filled cell in-place must transition out
                    // of `State::Completed` so that `data-ars-state` (and
                    // adapters that switch on `state()`) reflect that the
                    // pin is no longer complete. We re-enter `Focused`
                    // on the same cell.
                    Some(TransitionPlan::to(State::Focused { index }).apply(
                        move |ctx: &mut Context| {
                            if !ctx.value.is_controlled() {
                                ctx.value.set(values);
                            }

                            ctx.focused_index = Some(index);
                            ctx.complete = false;
                        },
                    ))
                }
            }

            (_, Event::ClearCell { index }) => {
                let index = *index;

                if index >= ctx.length || ctx.value.get()[index].is_empty() {
                    // Nothing to clear — Delete on an empty cell is a
                    // no-op (importantly, it must NOT navigate back like
                    // Backspace does, or pressing Delete on cell 2 with
                    // cell 2 empty would erase cell 1).
                    return None;
                }

                let mut values = ctx.value.get().clone();
                values[index] = String::new();

                // Clearing a filled cell must leave State::Completed if
                // we were in it (the pin is no longer complete) — mirror
                // the in-place DeleteChar transition.
                Some(TransitionPlan::to(State::Focused { index }).apply(
                    move |ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(values);
                        }
                        ctx.focused_index = Some(index);
                        ctx.complete = false;
                    },
                ))
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

            (_, Event::SetValue(value)) => {
                let value = value.clone();
                let length = ctx.length;
                let focused_index = ctx.focused_index;

                // Pre-compute the new completeness so we can pick the
                // correct target state. Without this, dropping from a
                // complete controlled value to an incomplete one leaves
                // the FSM in `State::Completed` even though
                // `ctx.complete = false`.
                let new_complete = match &value {
                    Some(v) => {
                        let mut probe = v.clone();
                        probe.resize(length, String::new());
                        length > 0 && probe.iter().all(|cell| !cell.is_empty())
                    }
                    None => ctx.complete,
                };

                let target = match (focused_index, new_complete) {
                    (Some(index), _) => State::Focused { index },
                    (None, true) => State::Completed,
                    (None, false) => State::Idle,
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    if let Some(mut value) = value {
                        value.resize(length, String::new());
                        ctx.value.set(value.clone());
                        ctx.value.sync_controlled(Some(value));
                        ctx.complete = new_complete;
                    } else {
                        ctx.value.sync_controlled(None);
                    }
                }))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                let new_length = props.length;

                // Pre-compute the post-SetProps target state. SetProps used
                // to be `context_only`, which left the FSM in
                // `State::Focused { index }` even when the new length
                // shrank `focused_index` out of bounds — producing
                // contradictory `state() == Focused` with no focused cell.
                let mut probe_values = ctx.value.get().clone();
                probe_values.resize(new_length, String::new());
                let new_complete =
                    new_length > 0 && probe_values.iter().all(|cell| !cell.is_empty());
                let new_focused_index = ctx.focused_index.filter(|&idx| idx < new_length);
                let target = match (new_focused_index, new_complete) {
                    (Some(index), _) => State::Focused { index },
                    (None, true) => State::Completed,
                    (None, false) => State::Idle,
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    if new_length != ctx.length {
                        // Resize the value vector. In controlled mode we
                        // must also push the resized vector through the
                        // controlled slot — otherwise `get()` keeps
                        // returning the old vector and outputs like
                        // `hidden_input_attrs()` (which joins `get()`)
                        // diverge from the rendered cells.
                        let mut values = ctx.value.get().clone();
                        values.resize(new_length, String::new());
                        let was_controlled = ctx.value.is_controlled();
                        ctx.value.set(values.clone());
                        if was_controlled {
                            ctx.value.sync_controlled(Some(values));
                        }

                        // Clear the focused index if it falls outside the
                        // new bounds. Leaving a stale out-of-range index
                        // strands the roving tabindex (no cell matches the
                        // index, and the `None` fallback to cell 0 does
                        // not fire), making the group unreachable.
                        if let Some(idx) = ctx.focused_index
                            && idx >= new_length
                        {
                            ctx.focused_index = None;
                        }

                        ctx.length = new_length;
                    }
                    ctx.disabled = props.disabled;
                    ctx.invalid = props.invalid;
                    ctx.otp = props.otp;
                    ctx.mask = props.mask;
                    ctx.placeholder = props.placeholder.clone();
                    ctx.mode = props.mode;
                    ctx.name = props.name.clone();
                    ctx.select_on_focus = props.select_on_focus;
                    ctx.blur_on_complete = props.blur_on_complete;
                    ctx.dir = props.dir;
                    ctx.complete =
                        ctx.length > 0 && ctx.value.get().iter().all(|cell| !cell.is_empty());
                }))
            }

            (_, Event::SetHasDescription(has_description)) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }

            (State::Idle, Event::InputChar { .. }) => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "pin_input::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        // Order matters: SetProps MUST run before SetValue when both
        // are emitted in the same cycle. SetValue's resize uses
        // `ctx.length`, so if `length` and `value` change together
        // (4-cell → 6-cell + new 6-cell value), running SetValue first
        // would truncate the new value against the old length, then
        // SetProps would lock in the truncated vector.
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

impl Mode {
    /// Returns `true` if the given character is allowed under this mode.
    #[must_use]
    pub fn accepts(self, ch: char) -> bool {
        match self {
            // `char::is_numeric()` accepts any character in Unicode
            // general categories `Nd` (Decimal_Number), `Nl`
            // (Letter_Number), or `No` (Other_Number). For a PIN field
            // this is the right pragmatic choice: it accepts every
            // locale's native decimal digits — ASCII `0`–`9`,
            // Arabic-Indic `٠`–`٩`, Devanagari `०`–`९`, Persian `۰`–`۹`,
            // etc. — without requiring an external Unicode-properties
            // dependency. The over-acceptance to Roman numerals (`Ⅻ`)
            // and fractions (`½`) is theoretical: no keyboard sends
            // these characters into a PIN input. This matches the
            // spec's "Unicode digit category" wording.
            Self::Numeric => ch.is_numeric(),
            // Alphanumeric: any character whose Unicode general
            // category is `Letter` or one of the `Number` subcategories.
            Self::Alphanumeric => ch.is_alphanumeric(),
            Self::Password => true,
        }
    }
}

fn next_empty_index(values: &[String], current: usize) -> Option<usize> {
    let length = values.len();

    (current + 1..length).find(|j| values[*j].is_empty())
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.length != new.length
        || old.disabled != new.disabled
        || old.invalid != new.invalid
        || old.otp != new.otp
        || old.mask != new.mask
        || old.placeholder != new.placeholder
        || old.mode != new.mode
        || old.name != new.name
        || old.form != new.form
        || old.required != new.required
        || old.readonly != new.readonly
        || old.select_on_focus != new.select_on_focus
        || old.blur_on_complete != new.blur_on_complete
        || old.dir != new.dir
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

        // Roving tabindex: the focused cell is the tab stop. When no cell
        // is focused (the initial / post-blur state), fall back to the first
        // cell so the group remains tabbable from the document — without this
        // fallback, every cell carries `tabindex="-1"` and keyboard users
        // cannot enter the PinInput at all.
        let is_focused_cell = self.ctx.focused_index == Some(index);
        let is_default_tab_target = self.ctx.focused_index.is_none() && index == 0;

        attrs.set(
            HtmlAttr::TabIndex,
            if is_focused_cell || is_default_tab_target {
                "0"
            } else {
                "-1"
            },
        );

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

        if self.props.required {
            // `aria-required` lives on every visible cell because the
            // hidden input cannot participate in native constraint
            // validation (browsers skip `type="hidden"`). Adapters that
            // need to enforce required entry at form-submit time must
            // implement custom validation; the spec §5 covers this.
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
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

        // `aria-hidden` is intentionally NOT set on this element: an
        // `<input type="hidden">` is already excluded from the accessibility
        // tree by virtue of its element type, and adding `aria-hidden`
        // produces invalid markup that strict a11y validators flag.
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Value, combined)
            .set(HtmlAttr::TabIndex, "-1");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        // The native `required` attribute is intentionally NOT set on the
        // hidden input: browser constraint validation skips
        // `type="hidden"` elements, so a `required` here is a no-op that
        // silently misleads adapters. Required-state semantics are exposed
        // via `aria-required` on each visible cell — adapters that need to
        // enforce required PIN entry at submit time must implement custom
        // validation (see spec §5).

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
    /// In `disabled` / `readonly` modes the destructive events
    /// (`Backspace`, `Delete`) are dropped by the machine — so the
    /// method also returns `false` for those keys in those modes,
    /// otherwise adapters would suppress native key behavior on a
    /// false positive.
    pub fn on_cell_keydown(&self, index: usize, data: &KeyboardEventData) -> bool {
        if data.is_composing {
            return false;
        }

        // RTL reverses the visual direction of arrow-key navigation:
        // ArrowRight should move toward earlier cells (visually left),
        // ArrowLeft toward later cells (visually right). Without this,
        // Hebrew/Arabic users get inverted navigation.
        let is_rtl = self.ctx.dir == Direction::Rtl;

        let event = match data.key {
            KeyboardKey::ArrowLeft => {
                if is_rtl {
                    Event::FocusNext
                } else {
                    Event::FocusPrev
                }
            }
            KeyboardKey::ArrowRight => {
                if is_rtl {
                    Event::FocusPrev
                } else {
                    Event::FocusNext
                }
            }
            // Backspace = clear current OR navigate-to-prev on empty cell.
            // Delete = clear current cell only, never navigate (so Delete
            // on an empty non-first cell is a no-op instead of erasing
            // the previous digit). Both go through the disabled/readonly
            // guard in `transition`, so report `false` here when the
            // machine would drop them.
            KeyboardKey::Backspace => {
                if self.ctx.disabled || self.props.readonly {
                    return false;
                }
                Event::DeleteChar { index }
            }
            KeyboardKey::Delete => {
                if self.ctx.disabled || self.props.readonly {
                    return false;
                }
                Event::ClearCell { index }
            }
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
    fn pin_input_numeric_mode_accepts_unicode_decimal_digits() {
        // Per spec: `Mode::Numeric` filters by Unicode decimal-digit
        // category, not just ASCII. Localized OTP codes typed from
        // Arabic-Indic, Devanagari, Persian etc. keyboards must fill
        // the cells.
        let mut svc = service(props().mode(Mode::Numeric));
        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        // Arabic-Indic '٧' (U+0667 = 7), Devanagari '३' (U+0969 = 3),
        // Persian '۵' (U+06F5 = 5).
        for (i, ch) in ['٧', '३', '۵', '9'].into_iter().enumerate() {
            drop(svc.send(Event::InputChar { index: i, char: ch }));
        }

        assert_eq!(svc.context().value.get()[0], "٧");
        assert_eq!(svc.context().value.get()[1], "३");
        assert_eq!(svc.context().value.get()[2], "۵");
        assert_eq!(svc.context().value.get()[3], "9");
    }

    #[test]
    fn pin_input_numeric_mode_rejects_letters_and_punctuation() {
        // `char::is_numeric()` admits Nd + Nl + No (so Roman numerals
        // and fractions slip through — see the impl comment for why
        // that over-acceptance is acceptable). Letters and punctuation
        // are firmly outside any numeric category and must still be
        // rejected here.
        let mut svc = service(props().mode(Mode::Numeric));
        drop(svc.send(Event::Focus {
            index: 0,
            is_keyboard: false,
        }));

        for non_numeric in ['a', 'Z', '!', '@', ' '] {
            let result = svc.send(Event::InputChar {
                index: 0,
                char: non_numeric,
            });
            assert!(
                !result.context_changed,
                "non-numeric {non_numeric:?} must be rejected"
            );
        }
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
    fn pin_input_first_cell_is_default_tab_target_when_no_cell_focused() {
        // Initial state: focused_index = None. Roving tabindex must still leave
        // one cell tabbable so keyboard users can enter the group — otherwise
        // every cell carries `tabindex="-1"` and the group is unreachable.
        let svc = service(props());
        assert_eq!(svc.context().focused_index, None);

        let api = svc.connect(&|_| {});

        assert_eq!(api.input_attrs(0).get(&HtmlAttr::TabIndex), Some("0"));
        for index in 1..4 {
            assert_eq!(
                api.input_attrs(index).get(&HtmlAttr::TabIndex),
                Some("-1"),
                "cell {index} must be -1 when no cell is focused"
            );
        }
    }

    #[test]
    fn pin_input_first_cell_is_default_tab_target_after_blur() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.context().focused_index, None);

        let api = svc.connect(&|_| {});

        assert_eq!(api.input_attrs(0).get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(api.input_attrs(2).get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn pin_input_readonly_blocks_clear_event() {
        let mut svc = service(props().readonly(true).default_value(alloc::vec![
            "1".into(),
            "2".into(),
            "3".into(),
            "4".into(),
        ]));

        let result = svc.send(Event::Clear);

        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get().len(), 4);
        assert_eq!(svc.context().value.get()[0], "1");
    }

    #[test]
    fn pin_input_set_props_syncs_controlled_value() {
        let mut svc =
            service(props().value(alloc::vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        assert_eq!(svc.context().value.get()[0], "1");
        assert!(svc.context().complete);

        drop(svc.set_props(props().value(alloc::vec![
            "9".into(),
            "8".into(),
            "7".into(),
            "6".into()
        ])));

        assert!(svc.context().value.is_controlled());
        assert_eq!(svc.context().value.get()[0], "9");

        drop(svc.set_props(props().uncontrolled()));

        assert!(!svc.context().value.is_controlled());
    }

    #[test]
    fn pin_input_set_props_resizes_value_when_length_changes() {
        let mut svc = service(props().length(4));

        drop(svc.set_props(props().length(6)));

        assert_eq!(svc.context().length, 6);
        assert_eq!(svc.context().value.get().len(), 6);
    }

    #[test]
    fn pin_input_set_props_shrinks_controlled_value_through_controlled_slot() {
        let mut svc =
            service(
                props()
                    .length(4)
                    .value(vec!["1".into(), "2".into(), "3".into(), "4".into()]),
            );

        assert!(svc.context().value.is_controlled());

        // Shrink the length. Without syncing the controlled slot,
        // `get()` would still return the 4-cell vector and the hidden
        // input would join "1234" instead of "12".
        drop(svc.set_props(props().length(2).value(vec![
            "1".into(),
            "2".into(),
            "3".into(),
            "4".into(),
        ])));

        assert_eq!(svc.context().length, 2);
        assert_eq!(svc.context().value.get().len(), 2);

        let api = svc.connect(&|_| {});
        assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some("12"));
    }

    #[test]
    fn pin_input_delete_filled_cell_transitions_out_of_completed_state() {
        let mut svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        assert_eq!(svc.state(), &State::Completed);
        assert!(svc.context().complete);

        drop(svc.send(Event::Focus {
            index: 3,
            is_keyboard: false,
        }));

        // Delete a filled cell IN-PLACE (cell is non-empty, so we don't
        // move to a previous cell). The FSM must leave Completed even
        // though we stay focused on the same index.
        drop(svc.send(Event::DeleteChar { index: 3 }));

        assert_eq!(svc.state(), &State::Focused { index: 3 });
        assert!(!svc.context().complete);
        assert_eq!(svc.context().value.get()[3], "");
    }

    #[test]
    fn pin_input_set_value_transitions_state_when_completeness_drops() {
        // Controlled mode: parent syncs a complete value, then a partial
        // one. The FSM must transition out of Completed when the new
        // value is incomplete, otherwise `data-ars-state` lies.
        let mut svc = service(props().value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));

        assert_eq!(svc.state(), &State::Completed);

        drop(svc.send(Event::SetValue(Some(vec![
            "1".into(),
            "2".into(),
            String::new(),
            String::new(),
        ]))));

        assert_eq!(svc.state(), &State::Idle);
        assert!(!svc.context().complete);
    }

    #[test]
    fn pin_input_set_value_transitions_to_completed_when_value_becomes_full() {
        let mut svc = service(props().value(vec![
            "1".into(),
            String::new(),
            String::new(),
            String::new(),
        ]));

        assert_eq!(svc.state(), &State::Idle);

        drop(svc.send(Event::SetValue(Some(vec![
            "1".into(),
            "2".into(),
            "3".into(),
            "4".into(),
        ]))));

        assert_eq!(svc.state(), &State::Completed);
        assert!(svc.context().complete);
    }

    #[test]
    fn pin_input_set_props_transitions_state_when_focus_invalidated() {
        // Length shrinks below the focused index — state was Focused{3},
        // new state must drop to Idle (no longer focused on any valid
        // cell). Without the transition, `data-ars-state` stays
        // `"focused"` despite `focused_index = None`.
        let mut svc = service(props().length(4));

        drop(svc.send(Event::Focus {
            index: 3,
            is_keyboard: false,
        }));
        assert_eq!(svc.state(), &State::Focused { index: 3 });

        drop(svc.set_props(props().length(2)));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(svc.context().focused_index, None);
    }

    #[test]
    fn pin_input_set_props_clears_focused_index_when_out_of_range() {
        let mut svc = service(props().length(4));

        drop(svc.send(Event::Focus {
            index: 3,
            is_keyboard: false,
        }));

        assert_eq!(svc.context().focused_index, Some(3));

        drop(svc.set_props(props().length(2)));

        // index 3 is now out of bounds; SetProps must clear it so the
        // roving tabindex falls back to cell 0 (otherwise every cell is
        // -1 and the group is unreachable).
        assert_eq!(svc.context().focused_index, None);

        let api = svc.connect(&|_| {});
        assert_eq!(api.input_attrs(0).get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(api.input_attrs(1).get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn pin_input_set_has_description_flips_context_flag_and_describedby() {
        let mut svc = service(props());

        assert!(!svc.context().has_description);

        drop(svc.send(Event::SetHasDescription(true)));

        assert!(svc.context().has_description);
        assert_eq!(
            svc.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("pin-description")
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
        // `aria-hidden` is intentionally omitted — `<input type="hidden">`
        // is already invisible to AT by element type and the attribute is
        // invalid markup on hidden inputs.
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), None);
    }

    #[test]
    fn pin_input_hidden_input_carries_name_and_form_but_not_required() {
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
        // `required` on a `type="hidden"` input is a no-op under native
        // constraint validation, so it must NOT be set here. Required
        // semantics live on the cell `aria-required` instead.
        assert!(!attrs.contains(&HtmlAttr::Required));
    }

    #[test]
    fn pin_input_required_sets_aria_required_on_every_cell() {
        let svc = service(props().required(true));

        let api = svc.connect(&|_| {});

        for index in 0..4 {
            let attrs = api.input_attrs(index);
            assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::Required)),
                Some("true"),
                "cell {index} must carry aria-required=true when required"
            );
        }
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
    fn pin_input_keydown_distinguishes_backspace_from_delete() {
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
        // Backspace = DeleteChar (clears or navigates-to-prev on empty).
        assert_eq!(events[2], Event::DeleteChar { index: 2 });
        // Delete = ClearCell (clears current only, no navigation).
        assert_eq!(events[3], Event::ClearCell { index: 2 });
    }

    #[test]
    fn pin_input_keydown_in_rtl_reverses_arrow_navigation() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());
        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let svc = service(props().dir(Direction::Rtl));

        let api = svc.connect(&send);

        // In RTL: visual-left = next cell, visual-right = previous cell.
        assert!(api.on_cell_keydown(1, &keyboard_event(KeyboardKey::ArrowLeft, false)));
        assert!(api.on_cell_keydown(1, &keyboard_event(KeyboardKey::ArrowRight, false)));

        let events = received.borrow();
        assert_eq!(events[0], Event::FocusNext);
        assert_eq!(events[1], Event::FocusPrev);
    }

    #[test]
    fn pin_input_clear_cell_on_filled_clears_without_navigation() {
        let mut svc =
            service(props().default_value(vec!["1".into(), "2".into(), "3".into(), "4".into()]));
        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));

        drop(svc.send(Event::ClearCell { index: 2 }));

        assert_eq!(svc.context().value.get()[2], "");
        // Index 1 (the previous cell) must be untouched — that's the key
        // distinction from Backspace.
        assert_eq!(svc.context().value.get()[1], "2");
        assert_eq!(svc.state(), &State::Focused { index: 2 });
    }

    #[test]
    fn pin_input_clear_cell_on_empty_cell_is_noop() {
        let mut svc = service(props().default_value(vec![
            "1".into(),
            String::new(),
            String::new(),
            String::new(),
        ]));
        drop(svc.send(Event::Focus {
            index: 2,
            is_keyboard: false,
        }));

        let result = svc.send(Event::ClearCell { index: 2 });

        // Delete on an empty non-first cell must NOT erase cell 1.
        assert!(!result.context_changed);
        assert_eq!(svc.context().value.get()[0], "1");
    }

    #[test]
    fn pin_input_keydown_returns_false_when_readonly_drops_destructive_keys() {
        // Backspace/Delete are dropped by the readonly guard, so the
        // method must report `false` for them — otherwise adapters
        // would `preventDefault()` based on a falsely-reported "handled".
        let svc = service(props().readonly(true));
        let api = svc.connect(&|_| {});

        for key in [KeyboardKey::Backspace, KeyboardKey::Delete] {
            let data = keyboard_event(key, false);
            assert!(
                !api.on_cell_keydown(0, &data),
                "readonly must not claim {key:?} as handled"
            );
        }
    }

    #[test]
    fn pin_input_keydown_returns_false_when_disabled_drops_destructive_keys() {
        let svc = service(props().disabled(true));
        let api = svc.connect(&|_| {});

        for key in [KeyboardKey::Backspace, KeyboardKey::Delete] {
            let data = keyboard_event(key, false);
            assert!(!api.on_cell_keydown(0, &data));
        }
    }

    #[test]
    fn pin_input_set_props_with_simultaneous_length_and_value_preserves_full_value() {
        // Bug guard: when `length` and `value` both change in one
        // set_props cycle, SetProps must run before SetValue so
        // SetValue's resize uses the NEW length and doesn't truncate
        // a freshly-provided larger value.
        let mut svc =
            service(
                props()
                    .length(4)
                    .value(vec!["1".into(), "2".into(), "3".into(), "4".into()]),
            );

        // Bump length to 6 AND push a 6-cell value in the same cycle.
        drop(svc.set_props(props().length(6).value(vec![
            "1".into(),
            "2".into(),
            "3".into(),
            "4".into(),
            "5".into(),
            "6".into(),
        ])));

        assert_eq!(svc.context().length, 6);
        assert_eq!(svc.context().value.get().len(), 6);
        assert_eq!(svc.context().value.get()[4], "5");
        assert_eq!(svc.context().value.get()[5], "6");
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
