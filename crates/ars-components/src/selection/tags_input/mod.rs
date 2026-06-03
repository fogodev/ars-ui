//! TagsInput component state machine and connect API.
//!
//! This module implements the framework-agnostic `TagsInput` machine defined in
//! `spec/components/selection/tags-input.md`. A `TagsInput` is a text input that
//! converts entries into removable tag chips with add, edit, remove, paste, and
//! keyboard navigation between tags.
//!
//! The agnostic core owns the tag list, the new-tag input value, the inline-edit
//! draft, highlight/removal state, the live-region announcement text, and every
//! ARIA / `data-ars-*` attribute. Live DOM focus is an adapter concern surfaced
//! through the typed [`Effect`] variants (`FocusTag`, `FocusInput`,
//! `FocusEditInput`) and announcements through the `Announce` variant; adapters
//! resolve the target element from [`Context`] (`focused_tag` / `editing_tag` /
//! `live_message`).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// Message function taking only a locale (e.g. the clear-all label).
type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

/// Message function taking a tag value and a locale (e.g. the remove-tag label).
type TagMessage = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Message function taking the current count, maximum, and a locale.
type CountMessage = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Message function taking the maximum tag count and a locale.
type MaxReachedMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// The states of the `TagsInput` state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle (not focused).
    Idle,

    /// The component is focused.
    Focused,

    /// A tag is being edited inline.
    EditingTag {
        /// The index of the tag being edited.
        index: usize,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused => f.write_str("focused"),
            Self::EditingTag { .. } => f.write_str("editing-tag"),
        }
    }
}

/// The events of the `TagsInput` state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Add a new tag from the given text.
    AddTag(String),

    /// Remove a tag by value (removes all occurrences when duplicates exist).
    RemoveTag(String),

    /// Remove a tag by index.
    RemoveTagAtIndex(usize),

    /// Enter inline edit mode for the tag at `index`.
    EditTag {
        /// The index of the tag to edit.
        index: usize,
    },

    /// Commit an inline edit, replacing the tag at `index` with `value`.
    CommitEdit {
        /// The index of the tag being edited.
        index: usize,

        /// The new value of the tag.
        value: String,
    },

    /// Cancel inline edit mode, discarding the draft.
    CancelEdit,

    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus lost.
    Blur,

    /// The input (or inline-edit) text changed.
    InputChange(String),

    /// Text pasted into the input — may contain delimiters.
    Paste(String),

    /// Clear all tags.
    ClearAll,

    /// Navigate focus to the previous tag.
    FocusPrevTag,

    /// Navigate focus to the next tag (or back to the input).
    FocusNextTag,

    /// IME composition started (CJK, etc.).
    CompositionStart,

    /// IME composition ended.
    CompositionEnd,
}

/// What happens to pending input text when `TagsInput` loses focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlurBehavior {
    /// Create a tag from the current input text (if non-empty and valid).
    #[default]
    Add,

    /// Discard the pending input text.
    Clear,
}

/// The context for the `TagsInput` state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The tag list. A `Vec` preserves insertion order; duplicate prevention is
    /// enforced in the transition logic when [`Context::allow_duplicates`] is false.
    pub value: Bindable<Vec<String>>,

    /// The current new-tag input value.
    pub input_value: String,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether the focus is keyboard-initiated (focus-visible).
    pub focus_visible: bool,

    /// The index of the currently focused tag, if any.
    pub focused_tag: Option<usize>,

    /// The index of the tag currently being edited, if any.
    pub editing_tag: Option<usize>,

    /// Draft text for the tag currently being edited. Initialized from the tag's
    /// current value when editing starts and updated on each keystroke during edit.
    pub editing_draft: String,

    /// The most recent screen-reader live-region announcement text. Surfaced by
    /// adapters in the [`Part::LiveRegion`] element via [`Effect::Announce`].
    pub live_message: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// The maximum number of tags, if limited.
    pub max: Option<usize>,

    /// The delimiter used for paste-splitting and delimiter-on-type detection.
    pub delimiter: String,

    /// Whether pasting text splits it into tags on the delimiter.
    pub add_on_paste: bool,

    /// Whether duplicate tag values are allowed.
    pub allow_duplicates: bool,

    /// What happens to pending input text on blur.
    pub blur_behavior: BlurBehavior,

    /// True while an IME composition session is active (between
    /// [`Event::CompositionStart`] and [`Event::CompositionEnd`]).
    pub is_composing: bool,

    /// The form field name, if any.
    pub name: Option<String>,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Component IDs for part identification.
    pub ids: ComponentIds,

    /// Resolved messages for accessibility labels and announcements.
    pub messages: Messages,
}

/// The props for the `TagsInput` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The adapter-provided base ID for the component.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<Vec<String>>,

    /// Default value for uncontrolled mode.
    pub default_value: Vec<String>,

    /// The maximum number of tags, if limited.
    pub max: Option<usize>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// The delimiter for paste-splitting and delimiter-on-type detection.
    pub delimiter: String,

    /// Whether pasting text splits it into tags on the delimiter.
    pub add_on_paste: bool,

    /// Whether duplicate tag values are allowed.
    pub allow_duplicates: bool,

    /// Whether at least one tag is required for form submission.
    pub required: bool,

    /// Maximum character length per tag (applied to the input element).
    pub max_length: Option<usize>,

    /// The form field name, if any.
    pub name: Option<String>,

    /// The placeholder for the new-tag input.
    pub placeholder: Option<String>,

    /// When `true`, tags can be edited inline by pressing Enter on a focused tag
    /// or double-clicking a tag.
    pub editable: bool,

    /// What happens to pending input text when the component loses focus.
    pub blur_behavior: BlurBehavior,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: Vec::new(),
            max: None,
            disabled: false,
            readonly: false,
            invalid: false,
            delimiter: ",".to_string(),
            add_on_paste: true,
            allow_duplicates: false,
            required: false,
            max_length: None,
            name: None,
            placeholder: None,
            editable: false,
            blur_behavior: BlurBehavior::Add,
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
    pub fn value(mut self, value: impl Into<Vec<String>>) -> Self {
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
    pub fn default_value(mut self, value: impl Into<Vec<String>>) -> Self {
        self.default_value = value.into();
        self
    }

    /// Sets [`max`](Self::max).
    #[must_use]
    pub const fn max(mut self, value: usize) -> Self {
        self.max = Some(value);
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

    /// Sets [`delimiter`](Self::delimiter).
    #[must_use]
    pub fn delimiter(mut self, value: impl Into<String>) -> Self {
        self.delimiter = value.into();
        self
    }

    /// Sets [`add_on_paste`](Self::add_on_paste).
    #[must_use]
    pub const fn add_on_paste(mut self, value: bool) -> Self {
        self.add_on_paste = value;
        self
    }

    /// Sets [`allow_duplicates`](Self::allow_duplicates).
    #[must_use]
    pub const fn allow_duplicates(mut self, value: bool) -> Self {
        self.allow_duplicates = value;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`max_length`](Self::max_length).
    #[must_use]
    pub const fn max_length(mut self, value: usize) -> Self {
        self.max_length = Some(value);
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Sets [`editable`](Self::editable).
    #[must_use]
    pub const fn editable(mut self, value: bool) -> Self {
        self.editable = value;
        self
    }

    /// Sets [`blur_behavior`](Self::blur_behavior).
    #[must_use]
    pub const fn blur_behavior(mut self, value: BlurBehavior) -> Self {
        self.blur_behavior = value;
        self
    }
}

/// The localizable messages for the `TagsInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Template for a tag's delete-trigger label (default: "Remove {value}").
    pub remove_tag_label: MessageFn<TagMessage>,

    /// Clear-all trigger label (default: "Remove all tags").
    pub clear_all_label: MessageFn<LocaleMessage>,

    /// Visually-hidden removal instruction attached to each tag
    /// (default: "Press Delete to remove").
    pub delete_hint: MessageFn<LocaleMessage>,

    /// Count label shown when `max` is set (default: "{current} of {max} tags").
    pub count_label: MessageFn<CountMessage>,

    /// Live-region announcement emitted when a tag is removed
    /// (default: "Removed {value}").
    pub removed_announcement: MessageFn<TagMessage>,

    /// Live-region announcement emitted when an add is blocked because the
    /// maximum is reached (default: "Maximum of {max} tags reached").
    pub max_reached_announcement: MessageFn<MaxReachedMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            remove_tag_label: MessageFn::new(|value: &str, _locale: &Locale| {
                format!("Remove {value}")
            }),
            clear_all_label: MessageFn::static_str("Remove all tags"),
            delete_hint: MessageFn::static_str("Press Delete to remove"),
            count_label: MessageFn::new(|current: usize, max: usize, _locale: &Locale| {
                format!("{current} of {max} tags")
            }),
            removed_announcement: MessageFn::new(|value: &str, _locale: &Locale| {
                format!("Removed {value}")
            }),
            max_reached_announcement: MessageFn::new(|max: usize, _locale: &Locale| {
                format!("Maximum of {max} tags reached")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the `tags_input` machine emits.
///
/// Each variant is resolved by the adapter, which reads the relevant [`Context`]
/// field to perform the live DOM operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter moves DOM focus to the tag at [`Context::focused_tag`].
    FocusTag,

    /// Adapter moves DOM focus to the new-tag input element.
    FocusInput,

    /// Adapter moves DOM focus to the inline-edit input at [`Context::editing_tag`].
    FocusEditInput,

    /// Adapter surfaces [`Context::live_message`] in the live region so assistive
    /// technology announces it.
    Announce,
}

/// The anatomy parts exposed by the `TagsInput` connect API.
#[derive(ComponentPart)]
#[scope = "tags-input"]
pub enum Part {
    /// Root container.
    Root,

    /// Label element.
    Label,

    /// Control wrapper around the tags and the input (the `grid`).
    Control,

    /// A tag chip (a grid `row`).
    Tag {
        /// The index of the tag.
        index: usize,
    },

    /// The text portion of a tag (a `gridcell`).
    TagText {
        /// The index of the tag.
        index: usize,
    },

    /// The delete trigger for a tag (a `gridcell` containing the remove button).
    TagDeleteTrigger {
        /// The index of the tag.
        index: usize,
    },

    /// The inline-edit input for a tag (visible only in edit mode).
    TagEdit {
        /// The index of the tag.
        index: usize,
    },

    /// The new-tag input element.
    Input,

    /// The clear-all trigger.
    ClearTrigger,

    /// The hidden form input carrying the joined tag value.
    HiddenInput,

    /// The description element.
    Description,

    /// The error-message element.
    ErrorMessage,

    /// The visually-hidden live region for screen-reader announcements.
    LiveRegion,
}

/// The machine for the `TagsInput` component.
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
        let context = Context {
            value: if let Some(value) = &props.value {
                Bindable::controlled(value.clone())
            } else {
                Bindable::uncontrolled(props.default_value.clone())
            },
            input_value: String::new(),
            focused: false,
            focus_visible: false,
            focused_tag: None,
            editing_tag: None,
            editing_draft: String::new(),
            live_message: String::new(),
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            max: props.max,
            delimiter: props.delimiter.clone(),
            add_on_paste: props.add_on_paste,
            allow_duplicates: props.allow_duplicates,
            blur_behavior: props.blur_behavior,
            is_composing: false,
            name: props.name.clone(),
            locale: env.locale.clone(),
            ids: ComponentIds::from_id(&props.id),
            messages: messages.clone(),
        };

        (State::Idle, context)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Mutating events are rejected while disabled or read-only.
        if ctx.disabled || ctx.readonly {
            match event {
                Event::AddTag(_)
                | Event::RemoveTag(_)
                | Event::RemoveTagAtIndex(_)
                | Event::EditTag { .. }
                | Event::CommitEdit { .. }
                | Event::ClearAll
                | Event::Paste(_) => return None,
                _ => {}
            }
        }

        if matches!(state, State::EditingTag { .. }) {
            // While editing a tag inline, list-mutating and tag-navigation events
            // are unreachable from the editing UI. Ignoring them keeps the edit
            // atomic (the user must commit, cancel, or blur first) so `editing_tag`
            // and the `EditingTag` state never dangle past a structural change.
            match event {
                Event::AddTag(_)
                | Event::RemoveTag(_)
                | Event::RemoveTagAtIndex(_)
                | Event::EditTag { .. }
                | Event::Paste(_)
                | Event::ClearAll
                | Event::FocusPrevTag
                | Event::FocusNextTag => return None,
                _ => {}
            }
        } else {
            // `CommitEdit` / `CancelEdit` only make sense while editing; a stray
            // one in another state would otherwise mutate the list and dangle focus.
            match event {
                Event::CommitEdit { .. } | Event::CancelEdit => return None,
                _ => {}
            }
        }

        match event {
            Event::AddTag(tag) => add_tag_plan(ctx, tag),

            Event::RemoveTag(value) => {
                let removed_index = ctx.value.get().iter().position(|tag| tag == value);

                remove_plan(ctx, removed_index, value)
            }

            Event::RemoveTagAtIndex(index) => {
                let tags = ctx.value.get();

                if *index >= tags.len() {
                    return None;
                }

                let value = tags[*index].clone();

                remove_plan(ctx, Some(*index), &value)
            }

            Event::EditTag { index } => {
                if !props.editable {
                    return None;
                }

                let index = *index;
                let value = ctx.value.get().get(index).cloned()?;
                Some(
                    TransitionPlan::to(State::EditingTag { index })
                        .apply(move |ctx: &mut Context| {
                            ctx.editing_tag = Some(index);
                            ctx.editing_draft = value;
                            ctx.focused_tag = None;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusEditInput)),
                )
            }

            Event::CommitEdit { index, value } => {
                // Reachable only in `EditingTag`, which `EditTag` enters only when
                // `editable` is set; props are fixed for the machine's lifetime, so no
                // separate `editable` guard is needed here.
                let index = *index;
                let trimmed = value.trim().to_string();
                let allow_duplicates = ctx.allow_duplicates;
                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            let mut tags = ctx.value.get().clone();

                            if index < tags.len() {
                                if trimmed.is_empty() {
                                    tags.remove(index);
                                } else {
                                    let is_duplicate = !allow_duplicates
                                        && tags
                                            .iter()
                                            .enumerate()
                                            .any(|(other, tag)| other != index && tag == &trimmed);

                                    if !is_duplicate {
                                        tags[index] = trimmed;
                                    }
                                }

                                ctx.value.set(tags);
                            }

                            ctx.editing_tag = None;
                            ctx.editing_draft.clear();
                        })
                        .with_effect(PendingEffect::named(Effect::FocusInput)),
                )
            }

            Event::CancelEdit => {
                let restored = ctx.editing_tag;
                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            ctx.editing_tag = None;
                            ctx.editing_draft.clear();
                            ctx.focused_tag = restored;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusTag)),
                )
            }

            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                        ctx.focused_tag = None;
                        ctx.editing_tag = None;
                        ctx.editing_draft.clear();
                    }),
                )
            }

            Event::Blur if matches!(state, State::EditingTag { .. }) => {
                // Blurring out of an inline edit discards the draft and returns to idle.
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.editing_tag = None;
                    ctx.editing_draft.clear();
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.focused_tag = None;
                }))
            }

            Event::Blur => {
                let input_trimmed = ctx.input_value.trim().to_string();

                let can_add = ctx.blur_behavior == BlurBehavior::Add
                    && !input_trimmed.is_empty()
                    && ctx.max.is_none_or(|max| ctx.value.get().len() < max)
                    && (ctx.allow_duplicates || !ctx.value.get().contains(&input_trimmed));

                let tag_to_add = can_add.then_some(input_trimmed);

                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        if let Some(tag) = tag_to_add {
                            let mut tags = ctx.value.get().clone();

                            tags.push(tag);

                            ctx.value.set(tags);
                        }

                        ctx.input_value.clear();
                        ctx.focused = false;
                        ctx.focus_visible = false;
                        ctx.focused_tag = None;
                    }),
                )
            }

            Event::InputChange(value) => input_change_plan(state, ctx, value),

            Event::Paste(text) => paste_plan(ctx, text),

            Event::ClearAll => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.value.set(Vec::new());
                ctx.input_value.clear();
                ctx.focused_tag = None;
            })),

            Event::FocusPrevTag => {
                let len = ctx.value.get().len();

                if len == 0 {
                    return None;
                }

                let new_index = match ctx.focused_tag {
                    Some(index) if index > 0 => index - 1,
                    Some(_) => 0,
                    None => len - 1,
                };

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.focused_tag = Some(new_index);
                    })
                    .with_effect(PendingEffect::named(Effect::FocusTag)),
                )
            }

            Event::FocusNextTag => {
                let len = ctx.value.get().len();

                match ctx.focused_tag {
                    Some(index) if index + 1 < len => {
                        let next = index + 1;
                        Some(
                            TransitionPlan::context_only(move |ctx: &mut Context| {
                                ctx.focused_tag = Some(next);
                            })
                            .with_effect(PendingEffect::named(Effect::FocusTag)),
                        )
                    }

                    Some(_) => Some(
                        TransitionPlan::context_only(|ctx: &mut Context| {
                            ctx.focused_tag = None;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusInput)),
                    ),

                    None => None,
                }
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = false;
            })),
        }
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

/// Builds the plan for an [`Event::AddTag`], including the max-reached announcement.
fn add_tag_plan(ctx: &Context, tag: &str) -> Option<TransitionPlan<Machine>> {
    let trimmed = tag.trim().to_string();

    if trimmed.is_empty() {
        return None;
    }

    if let Some(max) = ctx.max.filter(|&max| ctx.value.get().len() >= max) {
        let announcement = (ctx.messages.max_reached_announcement)(max, &ctx.locale);

        return Some(
            TransitionPlan::context_only(move |ctx: &mut Context| {
                ctx.live_message = announcement;
            })
            .with_effect(PendingEffect::named(Effect::Announce)),
        );
    }

    if !ctx.allow_duplicates && ctx.value.get().contains(&trimmed) {
        return None;
    }

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut tags = ctx.value.get().clone();

        tags.push(trimmed);

        ctx.value.set(tags);
        ctx.input_value.clear();
        ctx.focused_tag = None;
    }))
}

/// Builds the plan for removing the tag `value` (first found at `removed_index`),
/// updating focus and emitting the removal announcement.
fn remove_plan(
    ctx: &Context,
    removed_index: Option<usize>,
    value: &str,
) -> Option<TransitionPlan<Machine>> {
    let removed_index = removed_index?;

    let new_tags = ctx
        .value
        .get()
        .iter()
        .filter(|tag| tag.as_str() != value)
        .cloned()
        .collect::<Vec<_>>();

    let will_be_empty = new_tags.is_empty();

    let announcement = (ctx.messages.removed_announcement)(value, &ctx.locale);

    let focus_effect = if will_be_empty {
        Effect::FocusInput
    } else {
        Effect::FocusTag
    };

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            let focused_tag = (!new_tags.is_empty()).then(|| removed_index.min(new_tags.len() - 1));

            ctx.value.set(new_tags);
            ctx.focused_tag = focused_tag;
            ctx.live_message = announcement;
        })
        .with_effect(PendingEffect::named(focus_effect))
        .with_effect(PendingEffect::named(Effect::Announce)),
    )
}

/// Builds the plan for an [`Event::InputChange`], handling inline-edit drafts and
/// delimiter-on-type tag splitting.
fn input_change_plan(state: &State, ctx: &Context, value: &str) -> Option<TransitionPlan<Machine>> {
    if matches!(state, State::EditingTag { .. }) {
        let draft = value.to_string();

        return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.editing_draft = draft;
        }));
    }

    let delimiter = ctx.delimiter.clone();

    let should_split =
        !ctx.is_composing && !delimiter.is_empty() && value.contains(delimiter.as_str());

    if !should_split {
        let value = value.to_string();

        return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.input_value = value;
            ctx.focused_tag = None;
        }));
    }

    let value = value.to_string();

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut segments = value.split(delimiter.as_str()).collect::<Vec<_>>();

        // The final segment is the trailing remainder still being typed.
        let remainder = segments.pop().unwrap_or("").to_string();

        let mut tags = ctx.value.get().clone();

        for segment in segments {
            push_if_allowed(ctx, &mut tags, segment);
        }

        ctx.value.set(tags);
        ctx.input_value = remainder;
        ctx.focused_tag = None;
    }))
}

/// Builds the plan for an [`Event::Paste`].
fn paste_plan(ctx: &Context, text: &str) -> Option<TransitionPlan<Machine>> {
    if !ctx.add_on_paste {
        let text = text.to_string();

        return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.input_value = text;
        }));
    }

    let delimiter = ctx.delimiter.clone();
    let text = text.to_string();

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut tags = ctx.value.get().clone();

        if delimiter.is_empty() {
            push_if_allowed(ctx, &mut tags, &text);
        } else {
            for segment in text.split(delimiter.as_str()) {
                push_if_allowed(ctx, &mut tags, segment);
            }
        }

        ctx.value.set(tags);
        ctx.input_value.clear();
    }))
}

/// Pushes the trimmed `segment` onto `tags` when it is non-empty and respects the
/// max-tags and duplicate constraints in `ctx`.
fn push_if_allowed(ctx: &Context, tags: &mut Vec<String>, segment: &str) {
    let candidate = segment.trim();

    if candidate.is_empty() {
        return;
    }

    let under_max = ctx.max.is_none_or(|max| tags.len() < max);

    let unique = ctx.allow_duplicates || !tags.iter().any(|tag| tag == candidate);

    if under_max && unique {
        tags.push(candidate.to_string());
    }
}

/// The connect API for the `TagsInput` component.
///
/// Created by the [`Machine`]'s `connect` method (its `ars_core::Machine` impl);
/// provides per-part attribute methods and event handlers for adapter rendering.
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

impl Api<'_> {
    /// Returns the current state.
    #[must_use]
    pub const fn state(&self) -> &State {
        self.state
    }

    /// Returns the current tag list.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        self.ctx.value.get()
    }

    /// Returns the current inline-edit draft text.
    #[must_use]
    pub fn editing_draft(&self) -> &str {
        &self.ctx.editing_draft
    }

    /// Returns the current live-region announcement text.
    #[must_use]
    pub fn live_message(&self) -> &str {
        &self.ctx.live_message
    }

    /// Returns the count text for the current tag count versus the maximum, or
    /// `None` when no `max` is configured.
    #[must_use]
    pub fn count_text(&self) -> Option<String> {
        self.ctx.max.map(|max| {
            let current = self.ctx.value.get().len();

            (self.ctx.messages.count_label)(current, max, &self.ctx.locale)
        })
    }

    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Root);

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.invalid {
            attrs
                .set_bool(HtmlAttr::Data("ars-invalid"), true)
                .set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        attrs
    }

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Label);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Attributes for the control wrapper (the `grid`).
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Control);

        attrs.set(HtmlAttr::Role, "grid").set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.part("label"),
        );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if let Some(text) = self.count_text() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Description), text);
        }

        attrs
    }

    /// Attributes for the tag at `index` (a grid `row`).
    #[must_use]
    pub fn tag_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = base_attrs(&Part::Tag { index });

        attrs
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(HtmlAttr::Id, self.ctx.ids.item("tag", &index))
            .set(HtmlAttr::Role, "row");

        if let Some(value) = self.ctx.value.get().get(index) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), value);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        let is_focused = self.ctx.focused_tag == Some(index);

        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });

        if self.ctx.focus_visible && is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.editing_tag == Some(index) {
            attrs.set_bool(HtmlAttr::Data("ars-editing"), true);
        }

        if !self.ctx.disabled && !self.ctx.readonly {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.delete_hint)(&self.ctx.locale),
            );
        }

        attrs
    }

    /// Attributes for the text portion of the tag at `index` (a `gridcell`).
    #[must_use]
    pub fn tag_text_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = base_attrs(&Part::TagText { index });

        attrs.set(HtmlAttr::Role, "gridcell");

        attrs
    }

    /// Attributes for the delete trigger of the tag at `index` (a `gridcell`).
    #[must_use]
    pub fn tag_delete_trigger_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = base_attrs(&Part::TagDeleteTrigger { index });

        attrs.set(HtmlAttr::Role, "gridcell");

        if let Some(value) = self.ctx.value.get().get(index) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.remove_tag_label)(value, &self.ctx.locale),
            );
        }

        attrs.set(HtmlAttr::TabIndex, "-1");

        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the inline-edit input of the tag at `index`.
    #[must_use]
    pub fn tag_edit_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = base_attrs(&Part::TagEdit { index });

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("tag-edit-input", &index))
            .set(HtmlAttr::Type, "text");

        let is_editing = self.ctx.editing_tag == Some(index);

        if is_editing {
            attrs.set(HtmlAttr::Value, self.ctx.editing_draft.clone());
        } else {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }

        attrs
    }

    /// Attributes for the new-tag input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Input);

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));

        if let Some(placeholder) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if let Some(max_length) = self.props.max_length {
            attrs.set(HtmlAttr::MaxLength, max_length.to_string());
        }

        let mut described_by = Vec::new();

        if self.ctx.invalid {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        described_by.push(self.ctx.ids.part("description"));

        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            described_by.join(" "),
        );

        if self
            .ctx
            .max
            .is_some_and(|max| self.ctx.value.get().len() >= max)
        {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the clear-all trigger.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::ClearTrigger);

        attrs
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_all_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::HiddenInput);

        attrs
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Name, self.ctx.name.as_deref().unwrap_or(""))
            .set(
                HtmlAttr::Value,
                self.ctx.value.get().join(self.ctx.delimiter.as_str()),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.props.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        attrs
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Description);

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Attributes for the error-message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::ErrorMessage);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        if !self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }

        attrs
    }

    /// Attributes for the visually-hidden live region.
    #[must_use]
    pub fn live_region_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::LiveRegion);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("live-region"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    // — Event handlers —

    /// Handle focus on the input element.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Handle blur on the input element.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handle a value change on the input element.
    pub fn on_input_change(&self, value: String) {
        (self.send)(Event::InputChange(value));
    }

    /// Handle a paste into the input element.
    pub fn on_input_paste(&self, text: String) {
        (self.send)(Event::Paste(text));
    }

    /// Handle a keydown on the input element.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if self.ctx.is_composing {
            return;
        }

        match data.key {
            KeyboardKey::Enter => {
                if !self.ctx.input_value.trim().is_empty() {
                    (self.send)(Event::AddTag(self.ctx.input_value.clone()));
                }
            }

            KeyboardKey::Backspace | KeyboardKey::ArrowLeft => {
                if self.ctx.input_value.is_empty() {
                    (self.send)(Event::FocusPrevTag);
                }
            }

            KeyboardKey::Escape => {
                (self.send)(Event::InputChange(String::new()));
            }

            _ => {}
        }
    }

    /// Handle a keydown on a focused tag at `index`.
    pub fn on_tag_keydown(&self, index: usize, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::RemoveTagAtIndex(index));
            }

            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevTag),

            // ArrowRight moves to the next tag/input; Escape deselects to the input
            // (both reduce to `FocusNextTag`, which returns to the input past the last tag).
            KeyboardKey::ArrowRight | KeyboardKey::Escape => (self.send)(Event::FocusNextTag),

            KeyboardKey::Enter if self.props.editable => {
                (self.send)(Event::EditTag { index });
            }

            _ => {}
        }
    }

    /// Handle a click on a tag's delete trigger.
    pub fn on_tag_delete(&self, value: String) {
        (self.send)(Event::RemoveTag(value));
    }

    /// Handle a double-click on a tag (enter edit mode when editable).
    pub fn on_tag_dblclick(&self, index: usize) {
        if self.props.editable {
            (self.send)(Event::EditTag { index });
        }
    }

    /// Handle a value change on an inline-edit input.
    pub fn on_tag_edit_change(&self, value: String) {
        (self.send)(Event::InputChange(value));
    }

    /// Handle a keydown on an inline-edit input at `index`.
    pub fn on_tag_edit_keydown(&self, index: usize, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter => (self.send)(Event::CommitEdit {
                index,
                value: self.ctx.editing_draft.clone(),
            }),

            KeyboardKey::Escape => (self.send)(Event::CancelEdit),

            _ => {}
        }
    }

    /// Handle a click on the clear-all trigger.
    pub fn on_clear_click(&self) {
        (self.send)(Event::ClearAll);
    }

    /// Handle IME composition start.
    pub fn on_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Handle IME composition end.
    pub fn on_composition_end(&self) {
        (self.send)(Event::CompositionEnd);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Tag { index } => self.tag_attrs(index),
            Part::TagText { index } => self.tag_text_attrs(index),
            Part::TagDeleteTrigger { index } => self.tag_delete_trigger_attrs(index),
            Part::TagEdit { index } => self.tag_edit_attrs(index),
            Part::Input => self.input_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::LiveRegion => self.live_region_attrs(),
        }
    }
}

/// Returns an [`AttrMap`] pre-populated with the scope and part `data-ars-*` attrs.
fn base_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};
    use core::cell::RefCell;

    use ars_core::{Env, HtmlAttr, Service};
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::*;

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn props() -> Props {
        Props::new().id("tags")
    }

    fn with_tags(values: &[&str]) -> Service<Machine> {
        service(props().default_value(values.iter().map(ToString::to_string).collect::<Vec<_>>()))
    }

    fn key(key: KeyboardKey) -> KeyboardEventData {
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

    /// Drives `Api` event handlers, capturing emitted events and re-applying them.
    fn dispatch(service: &mut Service<Machine>, run: impl FnOnce(&Api<'_>)) {
        let captured = RefCell::new(Vec::new());

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = service.connect(&send);

            run(&api);
        }

        for event in captured.into_inner() {
            drop(service.send(event));
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn tags_of(service: &Service<Machine>) -> Vec<String> {
        service.context().value.get().clone()
    }

    // — Initial state —

    #[test]
    fn initial_state_is_idle_with_default_value() {
        let service = service(props().default_value(vec!["a".to_string(), "b".to_string()]));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(tags_of(&service), vec!["a", "b"]);
        assert!(!service.context().focused);
        assert_eq!(service.context().ids.part("input"), "tags-input");
    }

    #[test]
    fn controlled_value_overrides_default() {
        let service = service(
            props()
                .default_value(vec!["x".to_string()])
                .value(vec!["a".to_string()]),
        );

        assert_eq!(tags_of(&service), vec!["a"]);
    }

    // — Add —

    #[test]
    fn add_tag_on_enter() {
        let mut service = service(props());

        drop(service.send(Event::InputChange("apple".to_string())));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Enter));
        });

        assert_eq!(tags_of(&service), vec!["apple"]);
        assert_eq!(service.context().input_value, "");
    }

    #[test]
    fn enter_with_blank_input_does_not_add() {
        let mut service = service(props());

        drop(service.send(Event::InputChange("   ".to_string())));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Enter));
        });

        assert!(tags_of(&service).is_empty());
    }

    #[test]
    fn add_tag_on_comma_delimiter_while_typing() {
        let mut service = service(props());

        drop(service.send(Event::InputChange("apple,".to_string())));

        assert_eq!(tags_of(&service), vec!["apple"]);
        assert_eq!(service.context().input_value, "");
    }

    #[test]
    fn delimiter_split_keeps_trailing_remainder() {
        let mut service = service(props());

        drop(service.send(Event::InputChange("apple,banana,ba".to_string())));

        assert_eq!(tags_of(&service), vec!["apple", "banana"]);
        assert_eq!(service.context().input_value, "ba");
    }

    #[test]
    fn add_tag_trims_whitespace() {
        let mut service = service(props());

        drop(service.send(Event::AddTag("  spaced  ".to_string())));

        assert_eq!(tags_of(&service), vec!["spaced"]);
    }

    // — Remove —

    #[test]
    fn remove_tag_by_value_via_delete_button() {
        let mut service = with_tags(&["a", "b", "c"]);

        dispatch(&mut service, |api| api.on_tag_delete("b".to_string()));

        assert_eq!(tags_of(&service), vec!["a", "c"]);
    }

    #[test]
    fn remove_tag_by_index() {
        let mut service = with_tags(&["a", "b", "c"]);

        drop(service.send(Event::RemoveTagAtIndex(0)));

        assert_eq!(tags_of(&service), vec!["b", "c"]);
        assert_eq!(service.context().focused_tag, Some(0));
    }

    #[test]
    fn remove_out_of_range_index_is_noop() {
        let mut service = with_tags(&["a"]);

        let result = service.send(Event::RemoveTagAtIndex(5));

        assert!(!result.state_changed && !result.context_changed);
        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn remove_clamps_focus_to_last_remaining_tag() {
        let mut service = with_tags(&["a", "b", "c"]);

        drop(service.send(Event::RemoveTagAtIndex(2)));

        assert_eq!(tags_of(&service), vec!["a", "b"]);
        assert_eq!(service.context().focused_tag, Some(1));
    }

    #[test]
    fn removing_last_tag_clears_focus_and_focuses_input() {
        let mut service = with_tags(&["only"]);

        let result = service.send(Event::RemoveTagAtIndex(0));

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().focused_tag, None);

        let effects = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        assert!(effects.contains(&Effect::FocusInput));
        assert!(effects.contains(&Effect::Announce));
    }

    #[test]
    fn remove_sets_removal_announcement() {
        let mut service = with_tags(&["alpha", "beta"]);

        drop(service.send(Event::RemoveTag("alpha".to_string())));

        assert_eq!(service.context().live_message, "Removed alpha");
    }

    #[test]
    fn backspace_on_empty_input_focuses_last_tag() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::Focus { is_keyboard: true }));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Backspace));
        });

        assert_eq!(service.context().focused_tag, Some(1));
    }

    #[test]
    fn backspace_with_text_does_not_focus_tag() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::InputChange("typing".to_string())));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Backspace));
        });

        assert_eq!(service.context().focused_tag, None);
    }

    #[test]
    fn backspace_on_focused_tag_removes_it() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::FocusPrevTag));

        let focused = service.context().focused_tag.expect("a tag is focused");

        dispatch(&mut service, |api| {
            api.on_tag_keydown(focused, &key(KeyboardKey::Delete));
        });

        assert_eq!(tags_of(&service), vec!["a"]);
    }

    // — Navigation —

    #[test]
    fn arrow_navigation_between_tags() {
        let mut service = with_tags(&["a", "b", "c"]);

        drop(service.send(Event::FocusPrevTag));

        assert_eq!(service.context().focused_tag, Some(2));

        drop(service.send(Event::FocusPrevTag));

        assert_eq!(service.context().focused_tag, Some(1));

        drop(service.send(Event::FocusNextTag));

        assert_eq!(service.context().focused_tag, Some(2));
    }

    #[test]
    fn focus_next_past_last_tag_returns_to_input() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::FocusPrevTag));

        assert_eq!(service.context().focused_tag, Some(1));

        let result = service.send(Event::FocusNextTag);

        assert_eq!(service.context().focused_tag, None);

        let effects = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        assert!(effects.contains(&Effect::FocusInput));
    }

    #[test]
    fn focus_prev_on_empty_list_is_noop() {
        let mut service = service(props());

        let result = service.send(Event::FocusPrevTag);

        assert!(!result.state_changed && !result.context_changed);
    }

    // — Duplicates —

    #[test]
    fn duplicate_prevented_by_default() {
        let mut service = with_tags(&["apple"]);

        drop(service.send(Event::AddTag("apple".to_string())));

        assert_eq!(tags_of(&service), vec!["apple"]);
    }

    #[test]
    fn duplicates_allowed_when_opted_in() {
        let mut service = service(
            props()
                .allow_duplicates(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::AddTag("apple".to_string())));

        assert_eq!(tags_of(&service), vec!["apple", "apple"]);
    }

    // — Max —

    #[test]
    fn max_tags_limit_blocks_add_and_announces() {
        let mut service = service(
            props()
                .max(2)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        let result = service.send(Event::AddTag("c".to_string()));

        assert_eq!(tags_of(&service), vec!["a", "b"]);
        assert_eq!(service.context().live_message, "Maximum of 2 tags reached");

        let effects = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        assert!(effects.contains(&Effect::Announce));
    }

    #[test]
    fn delimiter_split_respects_max() {
        let mut service = service(props().max(2));

        drop(service.send(Event::InputChange("a,b,c,".to_string())));

        assert_eq!(tags_of(&service), vec!["a", "b"]);
    }

    // — Validation (invalid prop, per spec form integration) —

    #[test]
    fn invalid_prop_sets_aria_and_describedby() {
        let service = service(props().invalid(true));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("true")
        );
        assert_eq!(
            api.input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("tags-error-message tags-description")
        );
        assert!(
            api.error_message_attrs()
                .get(&HtmlAttr::Data("ars-hidden"))
                .is_none()
        );
    }

    #[test]
    fn valid_hides_error_message() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(
            api.error_message_attrs().get(&HtmlAttr::Data("ars-hidden")),
            Some("true")
        );
    }

    // — Inline editing —

    #[test]
    fn edit_requires_editable_prop() {
        let mut service = with_tags(&["a"]);

        let result = service.send(Event::EditTag { index: 0 });

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn edit_initializes_draft_and_enters_editing_state() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        let result = service.send(Event::EditTag { index: 0 });

        assert_eq!(service.state(), &State::EditingTag { index: 0 });
        assert_eq!(service.context().editing_draft, "apple");

        let effects = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        assert!(effects.contains(&Effect::FocusEditInput));
    }

    #[test]
    fn input_change_updates_draft_while_editing() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));
        drop(service.send(Event::InputChange("apricot".to_string())));

        assert_eq!(service.context().editing_draft, "apricot");
        // No delimiter splitting happens while editing.
        assert_eq!(tags_of(&service), vec!["apple"]);
    }

    #[test]
    fn commit_edit_replaces_tag() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));
        drop(service.send(Event::CommitEdit {
            index: 0,
            value: "apricot".to_string(),
        }));

        assert_eq!(tags_of(&service), vec!["apricot"]);
        assert_eq!(service.state(), &State::Focused);
        assert_eq!(service.context().editing_tag, None);
        assert_eq!(service.context().editing_draft, "");
    }

    #[test]
    fn commit_empty_edit_removes_tag() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));
        drop(service.send(Event::CommitEdit {
            index: 0,
            value: "   ".to_string(),
        }));

        assert_eq!(tags_of(&service), vec!["b"]);
    }

    #[test]
    fn commit_duplicate_edit_is_rejected() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));
        drop(service.send(Event::CommitEdit {
            index: 0,
            value: "b".to_string(),
        }));

        // "a" is left unchanged because "b" already exists.
        assert_eq!(tags_of(&service), vec!["a", "b"]);
    }

    #[test]
    fn cancel_edit_discards_draft_and_refocuses_tag() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));

        let result = service.send(Event::CancelEdit);

        assert_eq!(tags_of(&service), vec!["apple"]);
        assert_eq!(service.state(), &State::Focused);
        assert_eq!(service.context().editing_tag, None);
        assert_eq!(service.context().focused_tag, Some(0));

        let effects = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        assert!(effects.contains(&Effect::FocusTag));
    }

    #[test]
    fn blur_while_editing_discards_draft() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));
        drop(service.send(Event::Blur));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().editing_tag, None);
        assert_eq!(tags_of(&service), vec!["apple"]);
    }

    // — Paste —

    #[test]
    fn paste_splits_on_delimiter() {
        let mut service = service(props());

        dispatch(&mut service, |api| {
            api.on_input_paste("a,b,c".to_string());
        });

        assert_eq!(tags_of(&service), vec!["a", "b", "c"]);
    }

    #[test]
    fn paste_without_add_on_paste_fills_input() {
        let mut service = service(props().add_on_paste(false));

        drop(service.send(Event::Paste("a,b,c".to_string())));

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().input_value, "a,b,c");
    }

    // — Clear all —

    #[test]
    fn clear_all_empties_tags() {
        let mut service = with_tags(&["a", "b"]);

        dispatch(&mut service, |api| {
            api.on_clear_click();
        });

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().focused_tag, None);
    }

    // — IME composition —

    #[test]
    fn enter_is_suppressed_during_composition() {
        let mut service = service(props());

        drop(service.send(Event::InputChange("apple".to_string())));
        drop(service.send(Event::CompositionStart));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Enter));
        });

        assert!(tags_of(&service).is_empty());
        assert!(service.context().is_composing);
    }

    #[test]
    fn delimiter_split_suppressed_during_composition() {
        let mut service = service(props());

        drop(service.send(Event::CompositionStart));
        drop(service.send(Event::InputChange("apple,".to_string())));

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().input_value, "apple,");
    }

    // — Blur behavior —

    #[test]
    fn blur_add_behavior_commits_pending_input() {
        let mut service = service(props().blur_behavior(BlurBehavior::Add));

        drop(service.send(Event::Focus { is_keyboard: false }));
        drop(service.send(Event::InputChange("apple".to_string())));
        drop(service.send(Event::Blur));

        assert_eq!(tags_of(&service), vec!["apple"]);
        assert_eq!(service.context().input_value, "");
    }

    #[test]
    fn blur_clear_behavior_discards_pending_input() {
        let mut service = service(props().blur_behavior(BlurBehavior::Clear));

        drop(service.send(Event::Focus { is_keyboard: false }));
        drop(service.send(Event::InputChange("apple".to_string())));
        drop(service.send(Event::Blur));

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().input_value, "");
    }

    // — Disabled / readonly guards —

    #[test]
    fn disabled_blocks_mutations() {
        let mut service = service(props().disabled(true).default_value(vec!["a".to_string()]));

        drop(service.send(Event::AddTag("b".to_string())));
        drop(service.send(Event::RemoveTagAtIndex(0)));
        drop(service.send(Event::ClearAll));

        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn readonly_blocks_mutations() {
        let mut service = service(props().readonly(true).default_value(vec!["a".to_string()]));

        drop(service.send(Event::AddTag("b".to_string())));

        assert_eq!(tags_of(&service), vec!["a"]);
    }

    // — Form integration —

    #[test]
    fn hidden_input_joins_value_with_delimiter() {
        let service = service(
            props()
                .name("tags-field")
                .required(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        let api = service.connect(&|_| {});

        let attrs = api.hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("tags-field"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("a,b"));
        assert_eq!(attrs.get(&HtmlAttr::Required), Some("true"));
    }

    #[test]
    fn count_text_reports_progress_toward_max() {
        let service = service(
            props()
                .max(5)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        let api = service.connect(&|_| {});

        assert_eq!(api.count_text().as_deref(), Some("2 of 5 tags"));
    }

    #[test]
    fn count_text_is_none_without_max() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert!(api.count_text().is_none());
    }

    #[test]
    fn input_disabled_when_at_max() {
        let service = service(props().max(1).default_value(vec!["a".to_string()]));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    // — Connect / grid pattern attrs —

    #[test]
    fn control_uses_grid_pattern() {
        let service = with_tags(&["a"]);

        let api = service.connect(&|_| {});

        assert_eq!(api.control_attrs().get(&HtmlAttr::Role), Some("grid"));
        assert_eq!(api.tag_attrs(0).get(&HtmlAttr::Role), Some("row"));
        assert_eq!(api.tag_text_attrs(0).get(&HtmlAttr::Role), Some("gridcell"));
        assert_eq!(
            api.tag_delete_trigger_attrs(0).get(&HtmlAttr::Role),
            Some("gridcell")
        );
    }

    #[test]
    fn focused_tag_is_tabbable() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::FocusPrevTag));

        let api = service.connect(&|_| {});

        assert_eq!(api.tag_attrs(1).get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(api.tag_attrs(0).get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn delete_trigger_has_localized_label() {
        let service = with_tags(&["apple"]);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.tag_delete_trigger_attrs(0)
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Remove apple")
        );
    }

    #[test]
    fn tag_edit_carries_draft_value_when_editing() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));

        let api = service.connect(&|_| {});

        assert_eq!(api.tag_edit_attrs(0).get(&HtmlAttr::Value), Some("apple"));
        assert_eq!(api.editing_draft(), "apple");
    }

    // — Snapshots: every part and each output-affecting branch —

    #[test]
    fn snapshot_root_default() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn snapshot_root_disabled_readonly_invalid_focused() {
        let mut service = service(props().disabled(true).readonly(true).invalid(true));

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn snapshot_label() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).label_attrs()));
    }

    #[test]
    fn snapshot_control_default() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).control_attrs()));
    }

    #[test]
    fn snapshot_control_with_max() {
        let service = service(props().max(3).default_value(vec!["a".to_string()]));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).control_attrs()));
    }

    #[test]
    fn snapshot_tag_default() {
        let service = with_tags(&["apple"]);

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_attrs(0)));
    }

    #[test]
    fn snapshot_tag_focused_visible() {
        let mut service = with_tags(&["apple"]);

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::FocusPrevTag));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_attrs(0)));
    }

    #[test]
    fn snapshot_tag_editing() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_attrs(0)));
    }

    #[test]
    fn snapshot_tag_disabled() {
        let service = service(
            props()
                .disabled(true)
                .default_value(vec!["apple".to_string()]),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_attrs(0)));
    }

    #[test]
    fn snapshot_tag_text() {
        let service = with_tags(&["apple"]);

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_text_attrs(0)));
    }

    #[test]
    fn snapshot_tag_delete_trigger() {
        let service = with_tags(&["apple"]);

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).tag_delete_trigger_attrs(0)
        ));
    }

    #[test]
    fn snapshot_tag_delete_trigger_disabled() {
        let service = service(
            props()
                .disabled(true)
                .default_value(vec!["apple".to_string()]),
        );

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).tag_delete_trigger_attrs(0)
        ));
    }

    #[test]
    fn snapshot_tag_edit_hidden() {
        let service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_edit_attrs(0)));
    }

    #[test]
    fn snapshot_tag_edit_editing() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).tag_edit_attrs(0)));
    }

    #[test]
    fn snapshot_input_default() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).input_attrs()));
    }

    #[test]
    fn snapshot_input_full_props() {
        let service = service(
            props()
                .placeholder("Add a tag")
                .required(true)
                .max_length(20)
                .invalid(true),
        );

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).input_attrs()));
    }

    #[test]
    fn snapshot_input_disabled_readonly() {
        let service = service(props().disabled(true).readonly(true));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).input_attrs()));
    }

    #[test]
    fn snapshot_input_at_max() {
        let service = service(props().max(1).default_value(vec!["a".to_string()]));

        assert_snapshot!(snapshot_attrs(&service.connect(&|_| {}).input_attrs()));
    }

    #[test]
    fn snapshot_clear_trigger() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).clear_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_clear_trigger_disabled() {
        let service = service(props().disabled(true));

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).clear_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_hidden_input() {
        let service = service(
            props()
                .name("tags-field")
                .required(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).hidden_input_attrs()
        ));
    }

    #[test]
    fn snapshot_hidden_input_disabled() {
        let service = service(props().disabled(true));

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).hidden_input_attrs()
        ));
    }

    #[test]
    fn snapshot_description() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).description_attrs()
        ));
    }

    #[test]
    fn snapshot_error_message_hidden() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).error_message_attrs()
        ));
    }

    #[test]
    fn snapshot_error_message_visible() {
        let service = service(props().invalid(true));

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).error_message_attrs()
        ));
    }

    #[test]
    fn snapshot_live_region() {
        let service = service(props());

        assert_snapshot!(snapshot_attrs(
            &service.connect(&|_| {}).live_region_attrs()
        ));
    }

    // — Coverage: builders, accessors, remaining event handlers, Display —

    #[test]
    fn props_builders_set_every_field() {
        let built = Props::new()
            .id("t")
            .value(vec!["a".to_string()])
            .uncontrolled()
            .default_value(vec!["d".to_string()])
            .max(4)
            .disabled(true)
            .readonly(true)
            .invalid(true)
            .delimiter(";")
            .add_on_paste(false)
            .allow_duplicates(true)
            .required(true)
            .max_length(12)
            .name("field")
            .placeholder("hint")
            .editable(true)
            .blur_behavior(BlurBehavior::Clear);

        assert_eq!(built.id, "t");
        assert_eq!(built.value, None);
        assert_eq!(built.default_value, vec!["d".to_string()]);
        assert_eq!(built.max, Some(4));
        assert!(built.disabled && built.readonly && built.invalid);
        assert_eq!(built.delimiter, ";");
        assert!(!built.add_on_paste && built.allow_duplicates && built.required);
        assert_eq!(built.max_length, Some(12));
        assert_eq!(built.name.as_deref(), Some("field"));
        assert_eq!(built.placeholder.as_deref(), Some("hint"));
        assert!(built.editable);
        assert_eq!(built.blur_behavior, BlurBehavior::Clear);
    }

    #[test]
    fn state_display_renders_each_variant() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Focused.to_string(), "focused");
        assert_eq!(State::EditingTag { index: 1 }.to_string(), "editing-tag");
    }

    #[test]
    fn api_accessors_expose_state_and_tags() {
        let service = with_tags(&["a", "b"]);

        let api = service.connect(&|_| {});

        assert_eq!(api.state(), &State::Idle);
        assert_eq!(api.tags(), &["a".to_string(), "b".to_string()]);
        assert_eq!(api.live_message(), "");
        assert_eq!(api.editing_draft(), "");
    }

    #[test]
    fn input_handlers_emit_expected_events() {
        let mut service = with_tags(&["a"]);

        dispatch(&mut service, |api| api.on_input_focus(true));

        assert!(service.context().focused && service.context().focus_visible);

        dispatch(&mut service, |api| api.on_input_change("typed".to_string()));

        assert_eq!(service.context().input_value, "typed");

        dispatch(&mut service, |api| api.on_input_paste("x,y".to_string()));

        assert_eq!(tags_of(&service), vec!["a", "x", "y"]);

        dispatch(&mut service, |api| {
            api.on_input_blur();
        });

        assert!(!service.context().focused);
    }

    #[test]
    fn input_keydown_arrowleft_and_escape_branches() {
        let mut service = with_tags(&["a", "b"]);
        // ArrowLeft on empty input focuses the previous tag.
        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::ArrowLeft));
        });

        assert_eq!(service.context().focused_tag, Some(1));

        // Escape in the input clears the pending text.
        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::InputChange("draft".to_string())));

        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::Escape));
        });

        assert_eq!(service.context().input_value, "");

        // An unhandled key is a no-op.
        dispatch(&mut service, |api| {
            api.on_input_keydown(&key(KeyboardKey::ArrowUp));
        });
    }

    #[test]
    fn tag_keydown_navigation_and_edit_branches() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );

        drop(service.send(Event::FocusPrevTag));

        assert_eq!(service.context().focused_tag, Some(1));

        dispatch(&mut service, |api| {
            api.on_tag_keydown(1, &key(KeyboardKey::ArrowLeft));
        });

        assert_eq!(service.context().focused_tag, Some(0));

        dispatch(&mut service, |api| {
            api.on_tag_keydown(0, &key(KeyboardKey::ArrowRight));
        });

        assert_eq!(service.context().focused_tag, Some(1));

        // Enter on an editable tag enters edit mode.
        dispatch(&mut service, |api| {
            api.on_tag_keydown(0, &key(KeyboardKey::Enter));
        });

        assert_eq!(service.state(), &State::EditingTag { index: 0 });

        // Escape on a focused tag returns to the input (no-op key otherwise).
        let mut other = with_tags(&["a"]);

        drop(other.send(Event::FocusPrevTag));

        dispatch(&mut other, |api| {
            api.on_tag_keydown(0, &key(KeyboardKey::Escape));
        });

        assert_eq!(other.context().focused_tag, None);

        dispatch(&mut other, |api| {
            api.on_tag_keydown(0, &key(KeyboardKey::ArrowUp));
        });
    }

    #[test]
    fn tag_dblclick_enters_edit_when_editable() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        dispatch(&mut service, |api| api.on_tag_dblclick(0));

        assert_eq!(service.state(), &State::EditingTag { index: 0 });

        // Non-editable double-click is ignored.
        let mut plain = with_tags(&["a"]);

        dispatch(&mut plain, |api| api.on_tag_dblclick(0));

        assert_eq!(plain.state(), &State::Idle);
    }

    #[test]
    fn tag_edit_handlers_commit_and_cancel() {
        let mut service = service(
            props()
                .editable(true)
                .default_value(vec!["apple".to_string()]),
        );

        drop(service.send(Event::EditTag { index: 0 }));

        dispatch(&mut service, |api| {
            api.on_tag_edit_change("apricot".to_string());
        });

        assert_eq!(service.context().editing_draft, "apricot");

        dispatch(&mut service, |api| {
            api.on_tag_edit_keydown(0, &key(KeyboardKey::Enter));
        });

        assert_eq!(tags_of(&service), vec!["apricot"]);

        // Cancel branch.
        drop(service.send(Event::EditTag { index: 0 }));

        dispatch(&mut service, |api| {
            api.on_tag_edit_keydown(0, &key(KeyboardKey::Escape));
        });

        assert_eq!(service.state(), &State::Focused);

        // Unhandled key during edit is a no-op.
        drop(service.send(Event::EditTag { index: 0 }));

        dispatch(&mut service, |api| {
            api.on_tag_edit_keydown(0, &key(KeyboardKey::ArrowUp));
        });

        assert_eq!(service.state(), &State::EditingTag { index: 0 });
    }

    #[test]
    fn composition_handlers_toggle_flag() {
        let mut service = service(props());

        dispatch(&mut service, |api| {
            api.on_composition_start();
        });

        assert!(service.context().is_composing);

        dispatch(&mut service, |api| {
            api.on_composition_end();
        });

        assert!(!service.context().is_composing);
    }

    #[test]
    fn remove_absent_value_is_noop() {
        let mut service = with_tags(&["a"]);

        let result = service.send(Event::RemoveTag("missing".to_string()));

        assert!(!result.state_changed && !result.context_changed);
        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn edit_out_of_range_index_is_noop() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        let result = service.send(Event::EditTag { index: 9 });

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn commit_and_cancel_ignored_outside_editing() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        let commit = service.send(Event::CommitEdit {
            index: 0,
            value: "z".to_string(),
        });

        let cancel = service.send(Event::CancelEdit);

        assert!(!commit.state_changed && !cancel.state_changed);
        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn list_mutations_ignored_while_editing() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        drop(service.send(Event::EditTag { index: 0 }));

        drop(service.send(Event::AddTag("b".to_string())));
        drop(service.send(Event::RemoveTagAtIndex(0)));
        drop(service.send(Event::ClearAll));
        drop(service.send(Event::Paste("c,d".to_string())));
        drop(service.send(Event::FocusPrevTag));

        assert_eq!(service.state(), &State::EditingTag { index: 0 });
        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn focus_exits_inline_edit() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        drop(service.send(Event::EditTag { index: 0 }));

        drop(service.send(Event::Focus { is_keyboard: false }));

        assert_eq!(service.state(), &State::Focused);
        assert_eq!(service.context().editing_tag, None);
        assert_eq!(service.context().editing_draft, "");
    }

    #[test]
    fn paste_with_empty_delimiter_adds_whole_text() {
        let mut service = service(props().delimiter(""));

        drop(service.send(Event::Paste("solid".to_string())));

        assert_eq!(tags_of(&service), vec!["solid"]);
    }

    #[test]
    fn hidden_input_uses_empty_name_when_unset() {
        let service = with_tags(&["a"]);

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some(""));
    }

    #[test]
    fn part_attrs_dispatch_covers_every_part() {
        let mut service = service(
            props()
                .editable(true)
                .invalid(true)
                .max(3)
                .name("f")
                .default_value(vec!["a".to_string()]),
        );

        drop(service.send(Event::Focus { is_keyboard: true }));

        let api = service.connect(&|_| {});

        for part in [
            Part::Root,
            Part::Label,
            Part::Control,
            Part::Tag { index: 0 },
            Part::TagText { index: 0 },
            Part::TagDeleteTrigger { index: 0 },
            Part::TagEdit { index: 0 },
            Part::Input,
            Part::ClearTrigger,
            Part::HiddenInput,
            Part::Description,
            Part::ErrorMessage,
            Part::LiveRegion,
        ] {
            let attrs = api.part_attrs(part);

            assert!(attrs.get(&HtmlAttr::Data("ars-scope")) == Some("tags-input"));
        }
    }

    #[test]
    fn commit_edit_with_out_of_range_index_only_exits_edit() {
        let mut service = service(props().editable(true).default_value(vec!["a".to_string()]));

        drop(service.send(Event::EditTag { index: 0 }));

        drop(service.send(Event::CommitEdit {
            index: 9,
            value: "z".to_string(),
        }));

        assert_eq!(service.state(), &State::Focused);
        assert_eq!(tags_of(&service), vec!["a"]);
        assert_eq!(service.context().editing_tag, None);
    }

    #[test]
    fn focus_prev_stays_on_first_tag() {
        let mut service = with_tags(&["a", "b"]);

        drop(service.send(Event::FocusPrevTag)); // -> Some(1)
        drop(service.send(Event::FocusPrevTag)); // -> Some(0)

        drop(service.send(Event::FocusPrevTag)); // stays Some(0)

        assert_eq!(service.context().focused_tag, Some(0));
    }

    #[test]
    fn focus_next_without_focused_tag_is_noop() {
        let mut service = with_tags(&["a", "b"]);

        let result = service.send(Event::FocusNextTag);

        assert!(!result.state_changed && !result.context_changed);
        assert_eq!(service.context().focused_tag, None);
    }

    #[test]
    fn add_blank_tag_is_noop() {
        let mut service = service(props());

        let result = service.send(Event::AddTag("   ".to_string()));

        assert!(!result.state_changed && !result.context_changed);
        assert!(tags_of(&service).is_empty());
    }

    #[test]
    fn paste_skips_empty_segments() {
        let mut service = service(props());

        drop(service.send(Event::Paste("a,,b".to_string())));

        assert_eq!(tags_of(&service), vec!["a", "b"]);
    }

    #[test]
    fn control_attrs_reflect_disabled_and_readonly() {
        let service = service(props().disabled(true).readonly(true));

        let attrs = service.connect(&|_| {}).control_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-readonly")), Some("true"));
    }

    #[test]
    fn api_debug_is_formattable() {
        let service = service(props());

        let api = service.connect(&|_| {});

        let rendered = format!("{api:?}");

        assert!(rendered.contains("Api"));
        assert!(rendered.contains("<callback>"));
    }

    #[test]
    fn part_attrs_for_out_of_range_index_omit_value() {
        let service = with_tags(&["a"]);
        let api = service.connect(&|_| {});

        // No value/label is set when the index has no backing tag.
        assert!(
            api.tag_attrs(9)
                .get(&HtmlAttr::Aria(AriaAttr::Label))
                .is_none()
        );
        assert!(
            api.tag_delete_trigger_attrs(9)
                .get(&HtmlAttr::Aria(AriaAttr::Label))
                .is_none()
        );
    }

    #[test]
    fn readonly_only_paths_render_disabled_triggers_and_hide_hint() {
        let service = service(props().readonly(true).default_value(vec!["a".to_string()]));
        let api = service.connect(&|_| {});

        // `if disabled || readonly` taken via the readonly operand.
        assert_eq!(
            api.tag_delete_trigger_attrs(0).get(&HtmlAttr::Disabled),
            Some("true")
        );
        assert_eq!(
            api.clear_trigger_attrs().get(&HtmlAttr::Disabled),
            Some("true")
        );
        // `if !disabled && !readonly` skipped via the readonly operand: no delete hint.
        assert!(
            api.tag_attrs(0)
                .get(&HtmlAttr::Aria(AriaAttr::Description))
                .is_none()
        );
    }

    #[test]
    fn enter_on_tag_without_editable_does_not_edit() {
        let mut service = with_tags(&["a"]);
        drop(service.send(Event::FocusPrevTag));

        dispatch(&mut service, |api| {
            api.on_tag_keydown(0, &key(KeyboardKey::Enter));
        });

        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn commit_duplicate_allowed_replaces_tag() {
        let mut service = service(
            props()
                .editable(true)
                .allow_duplicates(true)
                .default_value(vec!["a".to_string(), "b".to_string()]),
        );
        drop(service.send(Event::EditTag { index: 0 }));

        drop(service.send(Event::CommitEdit {
            index: 0,
            value: "b".to_string(),
        }));

        // With duplicates allowed, the duplicate check short-circuits and the replace lands.
        assert_eq!(tags_of(&service), vec!["b", "b"]);
    }

    #[test]
    fn paste_with_duplicates_allowed_keeps_repeats() {
        let mut service = service(
            props()
                .allow_duplicates(true)
                .default_value(vec!["a".to_string()]),
        );

        drop(service.send(Event::Paste("a,a".to_string())));

        assert_eq!(tags_of(&service), vec!["a", "a", "a"]);
    }

    #[test]
    fn input_change_with_empty_delimiter_never_splits() {
        let mut service = service(props().delimiter(""));

        drop(service.send(Event::InputChange("a,b,c".to_string())));

        assert!(tags_of(&service).is_empty());
        assert_eq!(service.context().input_value, "a,b,c");
    }

    #[test]
    fn blur_add_skips_duplicate_pending_input() {
        let mut service = service(props().default_value(vec!["apple".to_string()]));
        drop(service.send(Event::Focus { is_keyboard: false }));
        drop(service.send(Event::InputChange("apple".to_string())));

        drop(service.send(Event::Blur));

        // Duplicate pending input is dropped on blur, not added.
        assert_eq!(tags_of(&service), vec!["apple"]);
        assert_eq!(service.context().input_value, "");
    }

    #[test]
    fn blur_add_skips_when_at_max() {
        let mut service = service(props().max(1).default_value(vec!["a".to_string()]));
        drop(service.send(Event::Focus { is_keyboard: false }));
        drop(service.send(Event::InputChange("b".to_string())));

        drop(service.send(Event::Blur));

        assert_eq!(tags_of(&service), vec!["a"]);
    }

    #[test]
    fn blur_add_with_duplicates_allowed_adds_repeat() {
        let mut service = service(
            props()
                .allow_duplicates(true)
                .default_value(vec!["apple".to_string()]),
        );
        drop(service.send(Event::Focus { is_keyboard: false }));
        drop(service.send(Event::InputChange("apple".to_string())));

        drop(service.send(Event::Blur));

        // allow_duplicates short-circuits the blur-add duplicate guard.
        assert_eq!(tags_of(&service), vec!["apple", "apple"]);
    }

    #[test]
    fn paste_dedupes_when_duplicates_disallowed() {
        let mut service = service(props());

        drop(service.send(Event::Paste("x,x,y".to_string())));

        assert_eq!(tags_of(&service), vec!["x", "y"]);
    }

    #[test]
    fn paste_stops_adding_at_max() {
        let mut service = service(props().max(2));

        drop(service.send(Event::Paste("a,b,c,d".to_string())));

        assert_eq!(tags_of(&service), vec!["a", "b"]);
    }
}
