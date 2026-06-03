---
component: TagsInput
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: TagsInput
    react-aria: TagGroup
---

# TagsInput

A text input that converts entries into removable tags/chips. Supports add, edit, remove,
paste, and navigation between tags.

## 1. State Machine

### 1.1 States

```rust
/// The states of the TagsInput state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused,
    /// The component is in an editing tag state.
    EditingTag {
        /// The index of the tag being edited.
        index: usize,
    },
}
```

### 1.2 Events

```rust
/// The events of the TagsInput state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Add a new tag.
    AddTag(String),
    /// Remove a tag by value.
    RemoveTag(String),
    /// Remove a tag by index.
    RemoveTagAtIndex(usize),
    /// Enter edit mode for a tag.
    EditTag {
        /// The index of the tag being edited.
        index: usize,
    },
    /// Commit an edit.
    CommitEdit {
        /// The index of the tag being edited.
        index: usize,
        /// The new value of the tag.
        value: String,
    },
    /// Cancel edit mode.
    CancelEdit,
    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost.
    Blur,
    /// Input text changed.
    InputChange(String),
    /// Text pasted — may contain delimiters.
    Paste(String),
    /// Clear all tags.
    ClearAll,
    /// Navigate to previous tag.
    FocusPrevTag,
    /// Navigate to next tag (or back to input).
    FocusNextTag,
    /// IME composition started (CJK, etc.).
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
}
```

> **IME composition:** This component tracks `is_composing: bool` in `Context`. During composition (`is_composing == true`), Enter key MUST NOT trigger `AddTag` and delimiter detection MUST be suppressed. See §IME in `03-accessibility.md`.
> See also [IME Composition Handling](10-input-components.md#ime-composition-protocol) for the shared input method editor behavior during text composition.

### 1.3 Context

```rust
/// The context for the TagsInput state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// TagsInput uses Vec for insertion-order preservation. Duplicate prevention is enforced in transition logic.
    pub value: Bindable<Vec<String>>,
    /// The current input value.
    pub input_value: String,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The index of the focused tag.
    pub focused_tag: Option<usize>,
    /// The index of the tag being edited.
    pub editing_tag: Option<usize>,
    /// Draft text for the tag currently being edited. Initialized from the tag's
    /// current value when editing starts and updated on each keystroke during edit.
    pub editing_draft: String,
    /// The most recent screen-reader live-region announcement text, surfaced by
    /// adapters in the `LiveRegion` part via the `Announce` effect.
    pub live_message: String,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The maximum number of tags.
    pub max: Option<usize>,
    /// The delimiter for the tags.
    pub delimiter: String,
    /// Whether to add a tag on paste.
    pub add_on_paste: bool,
    /// Whether to allow duplicates.
    pub allow_duplicates: bool,
    /// What happens to pending input on blur.
    pub blur_behavior: BlurBehavior,
    /// True while an IME composition session is active (between CompositionStart and CompositionEnd).
    pub is_composing: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props for the TagsInput component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// The value of the component.
    pub value: Option<Vec<String>>,
    /// The default value of the component.
    pub default_value: Vec<String>,
    /// The maximum number of tags.
    pub max: Option<usize>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The delimiter for the tags.
    pub delimiter: String,
    /// Whether to add a tag on paste.
    pub add_on_paste: bool,
    /// Whether to allow duplicates.
    pub allow_duplicates: bool,
    /// Whether the tags input is required.
    pub required: bool,
    /// Maximum character length per tag.
    pub max_length: Option<usize>,
    /// The name of the component.
    pub name: Option<String>,
    /// The placeholder for the input.
    pub placeholder: Option<String>,
    /// When `true`, tags can be edited inline by pressing Enter on a focused tag or
    /// double-clicking a tag. The tag enters an inline edit mode where the user can
    /// modify the text. Pressing Enter commits the edit; Escape cancels it.
    /// Default: `false`.
    pub editable: bool,
    /// What happens to pending input text when the component loses focus.
    /// `Add` creates a tag from the current input (if non-empty and valid).
    /// `Clear` discards the pending input. Default: `BlurBehavior::Add`.
    pub blur_behavior: BlurBehavior,
    // Change callbacks provided by the adapter layer
}

/// What happens to pending input when TagsInput loses focus.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum BlurBehavior {
    /// Create a tag from the current input text (if non-empty and valid).
    #[default]
    Add,
    /// Discard the pending input text.
    Clear,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: Vec::new(),
            max: None, disabled: false, readonly: false, invalid: false,
            delimiter: ",".into(), add_on_paste: true, allow_duplicates: false,
            required: false, max_length: None,
            name: None, placeholder: None,
            editable: false,
            blur_behavior: BlurBehavior::Add,
        }
    }
}
```

### 1.5 TagsInput Inline Editing

When `editable: true`, tags support inline editing via the following state/event additions:

**Additional State**: The `EditingTag { index: usize }` state (already defined in the State enum)
becomes reachable. During this state, the tag at `index` renders an inline text input
pre-filled with the tag's current value.

**Editing Context Fields**:

Both fields live in `Context` (§1.3):

```rust,no_check
pub editing_tag: Option<usize>,     // Index of the tag currently being edited
/// Draft text for the tag currently being edited. Initialized from the tag's
/// current value when editing starts; updated on each keystroke during edit.
pub editing_draft: String,
```

**Editing Events**: The following events (already defined in the Event enum) drive the editing flow:

| Event                         | Trigger                                                         | Behavior                                                                                                                                                                                     |
| ----------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `EditTag { index }`           | Enter key on focused tag, or double-click when `editable: true` | Transition to `EditingTag { index }`. Set `editing_tag = Some(index)`, `editing_draft` = current tag value. Focus the inline input.                                                          |
| `CommitEdit { index, value }` | Enter key during edit                                           | If `value` is non-empty and passes duplicate validation (when `allow_duplicates: false`), replace the tag at `index`. Transition back to `Focused`. Clear `editing_tag` and `editing_draft`. |
| `CancelEdit`                  | Escape key during edit                                          | Discard `editing_draft`. Transition back to `Focused`. Return focus to the tag at the editing index.                                                                                         |
| `InputChange(text)`           | Typing in the inline edit input                                 | Update `editing_draft` with the new text. No state transition.                                                                                                                               |

**Keyboard Bindings During Editing**:

| Key    | In `Focused` state (tag focused)                     | In `EditingTag` state        |
| ------ | ---------------------------------------------------- | ---------------------------- |
| Enter  | Start edit (`EditTag { index }`) if `editable: true` | Commit edit (`CommitEdit`)   |
| Escape | —                                                    | Cancel edit (`CancelEdit`)   |
| Tab    | Move focus to next tag or input                      | Commit edit, then move focus |

**Guard**: `EditTag`, `CommitEdit`, and `CancelEdit` events are rejected when `editable: false`,
`disabled: true`, or `readonly: true`. `CommitEdit` and `CancelEdit` are additionally rejected
outside the `EditingTag` state. While in `EditingTag`, the list-mutating and tag-navigation events
(`AddTag`, `RemoveTag`, `RemoveTagAtIndex`, `EditTag`, `Paste`, `ClearAll`, `FocusPrevTag`,
`FocusNextTag`) are ignored so the edit stays atomic — the user must commit, cancel, or blur first.
A `Focus` event exits the edit (clearing `editing_tag` and `editing_draft`).

### 1.6 Full Machine Implementation

```rust
/// The machine for the TagsInput component.
pub struct Machine;

// Adapters resolve live DOM focus and announcements from these typed effects;
// the agnostic core never calls into the adapter platform directly.
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
            value: match &props.value {
                Some(value) => Bindable::controlled(value.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
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
                if !props.editable {
                    return None;
                }
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
    let new_tags: Vec<String> = ctx
        .value
        .get()
        .iter()
        .filter(|tag| tag.as_str() != value)
        .cloned()
        .collect();
    let will_be_empty = new_tags.is_empty();
    let announcement = (ctx.messages.removed_announcement)(value, &ctx.locale);
    let focus_effect = if will_be_empty {
        Effect::FocusInput
    } else {
        Effect::FocusTag
    };

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            let focused_tag = if new_tags.is_empty() {
                None
            } else {
                Some(removed_index.min(new_tags.len() - 1))
            };
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
        let mut segments: Vec<&str> = value.split(delimiter.as_str()).collect();
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
```

### 1.7 Connect / API

```rust
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

/// The connect API for the `TagsInput` component.
///
/// Created by [`Machine::connect`]; provides per-part attribute methods and event
/// handlers for adapter rendering.
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
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// Attributes for the control wrapper (the `grid`).
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::Control);
        attrs.set(HtmlAttr::Role, "grid");
        attrs.set(
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
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("tag", &index));
        attrs.set(HtmlAttr::Role, "row");
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("tag-edit-input", &index));
        attrs.set(HtmlAttr::Type, "text");
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
        let mut described_by: Vec<String> = Vec::new();
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
        attrs.set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.clear_all_label)(&self.ctx.locale),
        );
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    /// Attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::HiddenInput);
        attrs.set(HtmlAttr::Type, "hidden");
        attrs.set(HtmlAttr::Name, self.ctx.name.as_deref().unwrap_or(""));
        attrs.set(
            HtmlAttr::Value,
            self.ctx.value.get().join(self.ctx.delimiter.as_str()),
        );
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if self.props.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        if !self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }
        attrs
    }

    /// Attributes for the visually-hidden live region.
    #[must_use]
    pub fn live_region_attrs(&self) -> AttrMap {
        let mut attrs = base_attrs(&Part::LiveRegion);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("live-region"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
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
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs
}
```

## 2. Anatomy

| Part               | Selector                                                            | Element    | Notes                |
| ------------------ | ------------------------------------------------------------------- | ---------- | -------------------- |
| `Root`             | `[data-ars-scope="tags-input"][data-ars-part="root"]`               | `<div>`    |                      |
| `Label`            | `[data-ars-scope="tags-input"][data-ars-part="label"]`              | `<label>`  |                      |
| `Control`          | `[data-ars-scope="tags-input"][data-ars-part="control"]`            | `<div>`    | Wraps tags + input   |
| `Tag`              | `[data-ars-scope="tags-input"][data-ars-part="tag"]`                | `<span>`   | `data-ars-index`     |
| `TagText`          | `[data-ars-scope="tags-input"][data-ars-part="tag-text"]`           | `<span>`   |                      |
| `TagDeleteTrigger` | `[data-ars-scope="tags-input"][data-ars-part="tag-delete-trigger"]` | `<button>` | ×/close icon         |
| `TagEdit`          | `[data-ars-scope="tags-input"][data-ars-part="tag-edit"]`           | `<input>`  | Visible in edit mode |
| `Input`            | `[data-ars-scope="tags-input"][data-ars-part="input"]`              | `<input>`  | New tag entry        |
| `ClearTrigger`     | `[data-ars-scope="tags-input"][data-ars-part="clear-trigger"]`      | `<button>` | Remove all           |
| `HiddenInput`      | `[data-ars-scope="tags-input"][data-ars-part="hidden-input"]`       | `<input>`  | Form value           |
| `Description`      | `[data-ars-scope="tags-input"][data-ars-part="description"]`        | `<div>`    |                      |
| `ErrorMessage`     | `[data-ars-scope="tags-input"][data-ars-part="error-message"]`      | `<div>`    |                      |
| `LiveRegion`       | `[data-ars-scope="tags-input"][data-ars-part="live-region"]`        | `<div>`    | Polite announcements |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element            | Value                                                                                           |
| ------------------ | ------------------ | ----------------------------------------------------------------------------------------------- |
| `role`             | `Control`          | `grid` (enables proper AT navigation between tags and their delete actions)                     |
| `aria-labelledby`  | `Control`          | Label id                                                                                        |
| `role`             | `Tag`              | `row`                                                                                           |
| `role`             | `TagText`          | `gridcell`                                                                                      |
| `role`             | `TagDeleteTrigger` | `gridcell` (contains the remove button)                                                         |
| `aria-describedby` | `Input`            | ErrorMessage id (when invalid) then Description id                                              |
| `aria-label`       | `Tag`              | `"{value}"`                                                                                     |
| `aria-disabled`    | `Tag`              | When tag or group is disabled                                                                   |
| `aria-label`       | `TagDeleteTrigger` | `"Remove {value}"`                                                                              |
| `aria-label`       | `ClearTrigger`     | `"Remove all tags"`                                                                             |
| `aria-description` | `Tag`              | "Press Delete to remove" (localized via `Messages.delete_hint`); omitted when disabled/readonly |

### 3.2 Keyboard Interaction

| Key          | In Input                           | On Tag                     |
| ------------ | ---------------------------------- | -------------------------- |
| Enter        | Add current input as tag           | —                          |
| Backspace    | If empty: focus last tag           | Remove tag                 |
| Delete       | —                                  | Remove tag                 |
| ArrowLeft    | If cursor at start: focus last tag | Focus previous tag         |
| ArrowRight   | —                                  | Focus next tag or input    |
| Escape       | Clear input                        | Deselect tag → focus input |
| Double-click | —                                  | Enter edit mode on tag     |

### 3.3 Screen Reader Announcements

When a tag is removed, screen reader users must be informed of the removal and the
resulting focus location:

- **Live region announcement**: On tag deletion, the `RemoveTag` / `RemoveTagAtIndex`
  transitions write `"Removed {value}"` (localized via `Messages.removed_announcement`) to
  `ctx.live_message` and emit `Effect::Announce`. The adapter surfaces `ctx.live_message`
  in the `LiveRegion` part (`aria-live="polite"`).
- **Tag semantics**: Each Tag part has `aria-label="{value}"` plus
  `aria-description` set to the removal instruction (localized via `Messages.delete_hint`,
  default "Press Delete to remove").
- **Max count reached**: When an `AddTag` is blocked because `ctx.value.get().len() >= max`,
  the transition writes `"Maximum of {max} tags reached"` (localized via
  `Messages.max_reached_announcement`) to `ctx.live_message` and emits `Effect::Announce`.
  The input also sets `aria-disabled="true"` once the maximum is reached.
- **Focus management after removal**: The `RemoveTag` / `RemoveTagAtIndex` transitions move
  focus to the adjacent tag (same index, clamped to `len - 1`) via `Effect::FocusTag`, or to
  the input via `Effect::FocusInput` when no tags remain.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Template for tag delete trigger label (default: "Remove {value}")
    pub remove_tag_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Clear all trigger label (default: "Remove all tags")
    pub clear_all_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Visually hidden instruction (default: "Press Delete to remove")
    pub delete_hint: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Count label when `max` is set (default: "{current} of {max} tags").
    pub count_label: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
    /// Live-region announcement when a tag is removed (default: "Removed {value}").
    pub removed_announcement: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Live-region announcement when an add is blocked because the maximum is
    /// reached (default: "Maximum of {max} tags reached").
    pub max_reached_announcement: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            remove_tag_label: MessageFn::new(|value, _locale| format!("Remove {value}")),
            clear_all_label: MessageFn::static_str("Remove all tags"),
            delete_hint: MessageFn::static_str("Press Delete to remove"),
            count_label: MessageFn::new(|current, max, _locale| format!("{current} of {max} tags")),
            removed_announcement: MessageFn::new(|value, _locale| format!("Removed {value}")),
            max_reached_announcement: MessageFn::new(|max, _locale| {
                format!("Maximum of {max} tags reached")
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

All hardcoded `aria-label` values in `TagDeleteTrigger` and `ClearTrigger` MUST read from this struct.

- **Delimiter**: Default `,` — may be configured per locale (some locales use `;`).
- **Tag labels**: `"Remove {value}"` — localized via `Messages.remove_tag_label`.
- **RTL**: Tag order renders right-to-left; ArrowLeft/Right reversed visually.
- **Max count message**: `"{current} of {max} tags"` — localized with plural rules via `Messages.count_label`.

## 5. Form Integration

- **Hidden input**: The `HiddenInput` part is a `<input type="hidden">` whose `value` is the joined tag list (`ctx.value.get().join(&ctx.delimiter)`). The `name` attribute is set from `Props.name`.
- **Validation states**: `aria-invalid="true"` on `Root` when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby` on the `Input`.
- **Per-tag validation**: Use the forms foundation `Validator` trait (`07-forms.md`) via the `Field` wrapper. The `FieldValue::Text` variant receives the joined tag string. For per-tag granularity, register a custom `Validator` that splits the value by the delimiter and validates each segment individually:

```rust
use ars_forms::{Validator, FieldValue, ValidationContext, ValidationResult, ValidationError, ValidationErrorCode};

struct PerTagValidator<F: Fn(&str) -> Result<(), String>> {
    delimiter: String,
    validate_tag: F,
}

impl<F: Fn(&str) -> Result<(), String> + Send + Sync> Validator for PerTagValidator<F> {
    fn validate(&self, value: &FieldValue, _ctx: &ValidationContext) -> ValidationResult {
        let text = match value {
            FieldValue::Text(t) => t.as_str(),
            _ => return ValidationResult::Valid,
        };
        for tag in text.split(&self.delimiter).map(str::trim).filter(|t| !t.is_empty()) {
            if let Err(reason) = (self.validate_tag)(tag) {
                return ValidationResult::Invalid(vec![ValidationError {
                    message: reason.clone(),
                    code: ValidationErrorCode::Custom(reason),
                }]);
            }
        }
        ValidationResult::Valid
    }
}
```

- **Required validation**: When `required == true`, at least one tag must be present. The `HiddenInput` has `required` set and the empty string value triggers native constraint validation.
- **Max tags**: Enforced by the state machine via `Props.max`. No form-level validator needed.
- **Duplicate prevention**: Enforced by the state machine when `Props.allow_duplicates == false`. No form-level validator needed.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`TagsInput`), React Aria (`TagGroup`).

Note: Ark UI's `TagsInput` and React Aria's `TagGroup` differ significantly in scope. Ark UI's is an input-focused component (type to add tags); React Aria's is a display-focused component (tag list with remove). ars-ui's `TagsInput` covers both use cases.

### 6.1 Props

| Feature                       | ars-ui                          | Ark UI                             | React Aria                             | Notes                                             |
| ----------------------------- | ------------------------------- | ---------------------------------- | -------------------------------------- | ------------------------------------------------- |
| Controlled/uncontrolled value | `value` / `default_value`       | `value` / `defaultValue`           | `selectedKeys` / `defaultSelectedKeys` | React Aria uses selection model, not input model  |
| Max tags                      | `max`                           | `max`                              | --                                     | --                                                |
| Disabled                      | `disabled`                      | `disabled`                         | --                                     | --                                                |
| Read-only                     | `readonly`                      | `readOnly`                         | --                                     | --                                                |
| Invalid                       | `invalid`                       | `invalid`                          | --                                     | --                                                |
| Required                      | `required`                      | `required`                         | --                                     | --                                                |
| Delimiter                     | `delimiter`                     | `delimiter`                        | --                                     | React Aria has no input (display-only)            |
| Add on paste                  | `add_on_paste`                  | `addOnPaste`                       | --                                     | --                                                |
| Allow duplicates              | `allow_duplicates`              | --                                 | --                                     | ars-ui exclusive                                  |
| Max length per tag            | `max_length`                    | `maxLength`                        | --                                     | --                                                |
| Editable (inline edit)        | `editable`                      | `editable`                         | --                                     | --                                                |
| Blur behavior                 | `blur_behavior` (`Add`/`Clear`) | `blurBehavior` (`clear`/`add`)     | --                                     | --                                                |
| Name (form)                   | `name`                          | `name`                             | --                                     | --                                                |
| Placeholder                   | `placeholder`                   | `placeholder`                      | --                                     | --                                                |
| Input value                   | --                              | `inputValue` / `defaultInputValue` | --                                     | Ark UI has controlled input value                 |
| Validate (custom)             | --                              | `validate`                         | --                                     | Ark UI has per-tag validation callback            |
| Selection mode                | --                              | --                                 | `selectionMode`                        | React Aria exclusive (tag selection for bulk ops) |
| Disabled keys                 | --                              | --                                 | `disabledKeys`                         | React Aria exclusive                              |
| Disallow empty selection      | --                              | --                                 | `disallowEmptySelection`               | React Aria exclusive                              |
| Escape key behavior           | --                              | --                                 | `escapeKeyBehavior`                    | React Aria exclusive                              |
| On remove                     | --                              | --                                 | `onRemove`                             | React Aria fires on tag removal                   |

**Gaps:** None. Ark UI's `inputValue` controlled input is not needed -- ars-ui tracks input value in context. Ark UI's `validate` callback is handled via ars-ui's form integration (`PerTagValidator` in §5). React Aria's `TagGroup` is fundamentally a different component (display-only tag list) and its selection/removal props do not apply to the input-focused `TagsInput`.

### 6.2 Anatomy

| Part         | ars-ui             | Ark UI              | React Aria               | Notes                                     |
| ------------ | ------------------ | ------------------- | ------------------------ | ----------------------------------------- |
| Root         | `Root`             | `Root`              | `TagGroup`               | --                                        |
| Label        | `Label`            | `Label`             | `Label`                  | --                                        |
| Control      | `Control`          | `Control`           | `TagList`                | Wraps tags + input                        |
| Input        | `Input`            | `Input`             | --                       | React Aria has no input                   |
| Tag          | `Tag`              | `Item`              | `Tag`                    | A grid `row`                              |
| Tag preview  | --                 | `ItemPreview`       | --                       | ars-ui folds the chip wrapper into `Tag`  |
| Tag text     | `TagText`          | `ItemText`          | --                       | --                                        |
| Tag delete   | `TagDeleteTrigger` | `ItemDeleteTrigger` | --                       | React Aria handles remove via render prop |
| Tag edit     | `TagEdit`          | `ItemInput`         | --                       | Inline edit input                         |
| ClearTrigger | `ClearTrigger`     | `ClearTrigger`      | --                       | --                                        |
| HiddenInput  | `HiddenInput`      | `HiddenInput`       | --                       | Form submission                           |
| Description  | `Description`      | --                  | `Text[slot=description]` | --                                        |
| ErrorMessage | `ErrorMessage`     | --                  | `FieldError`             | --                                        |
| LiveRegion   | `LiveRegion`       | --                  | --                       | Polite removal/limit announcements        |

**Gaps:** None. ars-ui has no separate `ItemPreview` part — the chip wrapper role is covered by `Tag` itself — and adds a dedicated `LiveRegion` part for screen-reader announcements.

### 6.3 Events

| Callback         | ars-ui                    | Ark UI               | React Aria          | Notes                       |
| ---------------- | ------------------------- | -------------------- | ------------------- | --------------------------- |
| Value change     | via `Bindable`            | `onValueChange`      | --                  | --                          |
| Input change     | `Event::InputChange`      | `onInputValueChange` | --                  | --                          |
| Highlight change | via `Context.focused_tag` | `onHighlightChange`  | --                  | --                          |
| Invalid tag      | --                        | `onValueInvalid`     | --                  | ars-ui uses form validation |
| Selection change | --                        | --                   | `onSelectionChange` | React Aria exclusive        |
| Remove           | `Event::RemoveTag`        | --                   | `onRemove`          | --                          |

**Gaps:** None.

### 6.4 Features

| Feature                    | ars-ui | Ark UI | React Aria        |
| -------------------------- | ------ | ------ | ----------------- |
| Add tags by typing         | Yes    | Yes    | No (display-only) |
| Remove tags                | Yes    | Yes    | Yes               |
| Inline editing             | Yes    | Yes    | No                |
| Paste support              | Yes    | Yes    | No                |
| Keyboard navigation (tags) | Yes    | Yes    | Yes               |
| Max tags limit             | Yes    | Yes    | No                |
| Delimiter support          | Yes    | Yes    | No                |
| Blur behavior              | Yes    | Yes    | No                |
| Form integration           | Yes    | Yes    | No                |
| IME composition            | Yes    | --     | --                |
| RTL support                | Yes    | Yes    | Yes               |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) React Aria's `TagGroup` is a display/selection component, not an input component -- it does not support adding tags by typing; ars-ui's `TagsInput` covers both input and display use cases; (2) ars-ui uses form-level validation (`PerTagValidator`) instead of a component-level `validate` callback; (3) ars-ui tracks IME composition state for CJK input support.
- **Recommended additions:** None.
