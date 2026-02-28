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
    /// Locale for i18n message resolution.
    pub locale: Option<Locale>,
    /// Translatable messages.
    pub messages: Option<Messages>,
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
            locale: None,
            messages: None,
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

```rust
// Already present in Context:
pub editing_tag: Option<usize>,     // Index of the tag currently being edited

// Additional editing context (added to Context):
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
`disabled: true`, or `readonly: true`.

### 1.6 Full Machine Implementation

```rust
/// The machine for the TagsInput component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props) -> (Self::State, Self::Context) {
        let state = State::Idle;
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            input_value: String::new(),
            focused: false,
            focus_visible: false,
            focused_tag: None,
            editing_tag: None,
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
            locale,
            ids: ComponentIds::from_id(&props.id),
            messages,
        };

        (state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
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

        match event {
            Event::AddTag(tag) => {
                let tag = tag.trim().to_string();
                if tag.is_empty() { return None; }

                // Check max
                if let Some(max) = ctx.max {
                    if ctx.value.get().len() >= max { return None; }
                }

                // Check duplicates
                if !ctx.allow_duplicates && ctx.value.get().contains(&tag) {
                    return None;
                }

                let allow_dupes = ctx.allow_duplicates;
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut tags = ctx.value.get().clone();
                    if allow_dupes || !tags.contains(&tag) {
                        tags.push(tag);
                    }
                    ctx.value.set(tags);
                    ctx.input_value.clear();
                }))
            }

            Event::RemoveTag(val) => {
                let val = val.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut tags = ctx.value.get().clone();
                    let removed_idx = tags.iter().position(|t| t == &val);
                    tags.retain(|t| t != &val);
                    ctx.value.set(tags.clone());
                    if tags.is_empty() {
                        ctx.focused_tag = None;
                    } else if let Some(idx) = removed_idx {
                        ctx.focused_tag = Some(idx.min(tags.len() - 1));
                    }
                }).with_effect(PendingEffect::new("focus_tag", |ctx, _props, _send| {
                    if let Some(idx) = ctx.focused_tag {
                        let platform = use_platform_effects();
                        let tag_id = ctx.ids.item("tag", &idx);
                        platform.focus_element_by_id(&tag_id);
                    }
                    no_cleanup()
                })))
            }

            Event::RemoveTagAtIndex(idx) => {
                let idx = *idx;
                if idx >= ctx.value.get().len() { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut tags = ctx.value.get().clone();
                    if idx < tags.len() {
                        tags.remove(idx);
                        ctx.value.set(tags.clone());
                        if tags.is_empty() {
                            ctx.focused_tag = None;
                        } else {
                            ctx.focused_tag = Some(idx.min(tags.len() - 1));
                        }
                    }
                }).with_effect(PendingEffect::new("focus_tag", |ctx, _props, _send| {
                    if let Some(idx) = ctx.focused_tag {
                        let platform = use_platform_effects();
                        let tag_id = ctx.ids.item("tag", &idx);
                        platform.focus_element_by_id(&tag_id);
                    }
                    no_cleanup()
                })))
            }

            Event::EditTag { index } => {
                if *index >= ctx.value.get().len() { return None; }
                let idx = *index;
                Some(TransitionPlan::to(State::EditingTag { index: idx }).apply(move |ctx| {
                    ctx.editing_tag = Some(idx);
                }).with_effect(PendingEffect::new("focus_edit_input", |ctx, _props, _send| {
                    if let Some(idx) = ctx.editing_tag {
                        let platform = use_platform_effects();
                        let edit_id = ctx.ids.item("tag-edit-input", &idx);
                        platform.focus_element_by_id(&edit_id);
                    }
                    no_cleanup()
                })))
            }

            Event::CommitEdit { index, value } => {
                let idx = *index;
                let val = value.trim().to_string();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    if val.is_empty() {
                        let mut tags = ctx.value.get().clone();
                        if idx < tags.len() {
                            tags.remove(idx);
                            ctx.value.set(tags);
                        }
                    } else {
                        let mut tags = ctx.value.get().clone();
                        if idx < tags.len() {
                            tags[idx] = val;
                            ctx.value.set(tags);
                        }
                    }
                    ctx.editing_tag = None;
                }).with_effect(PendingEffect::new("focus_input", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let input_id = ctx.ids.part("input");
                    platform.focus_element_by_id(&input_id);
                    no_cleanup()
                })))
            }

            Event::CancelEdit => {
                Some(TransitionPlan::to(State::Focused).apply(|ctx| {
                    ctx.editing_tag = None;
                }).with_effect(PendingEffect::new("focus_input", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let input_id = ctx.ids.part("input");
                    platform.focus_element_by_id(&input_id);
                    no_cleanup()
                })))
            }

            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                    ctx.focused_tag = None;
                }))
            }

            Event::Blur if matches!(state, State::EditingTag { .. }) => {
                let idx = match state {
                    State::EditingTag { index } => *index,
                    _ => 0,
                };
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    // Commit the in-progress edit before transitioning
                    ctx.editing_tag = None;
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.focused_tag = None;
                }))
            }

            Event::Blur => {
                // Respect blur_behavior: Add creates a tag, Clear discards input.
                let input_trimmed = ctx.input_value.trim().to_string();
                let should_attempt_add = ctx.blur_behavior == BlurBehavior::Add;
                let can_add = should_attempt_add
                    && !input_trimmed.is_empty()
                    && ctx.max.map_or(true, |m| ctx.value.get().len() < m)
                    && (ctx.allow_duplicates || !ctx.value.get().contains(&input_trimmed));
                let tag_to_add = if can_add { Some(input_trimmed) } else { None };

                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    if let Some(tag) = tag_to_add {
                        let mut tags = ctx.value.get().clone();
                        tags.push(tag);
                        ctx.value.set(tags);
                    }
                    ctx.input_value.clear();
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.focused_tag = None;
                }))
            }

            Event::InputChange(val) => {
                let val = val.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.input_value = val;
                    ctx.focused_tag = None;
                }))
            }

            Event::Paste(text) => {
                if !ctx.add_on_paste {
                    let text = text.clone();
                    return Some(TransitionPlan::context_only(move |ctx| {
                        ctx.input_value = text;
                    }));
                }
                let delimiter = ctx.delimiter.clone();
                let max = ctx.max;
                let allow_duplicates = ctx.allow_duplicates;
                let parts: Vec<String> = text.split(&delimiter)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let current_tags = ctx.value.get().clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut tags = current_tags;
                    for part in parts {
                        if max.map_or(true, |m| tags.len() < m)
                            && (allow_duplicates || !tags.contains(&part))
                        {
                            tags.push(part);
                        }
                    }
                    ctx.value.set(tags);
                    ctx.input_value.clear();
                }))
            }

            Event::ClearAll => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.value.set(Vec::new());
                    ctx.input_value.clear();
                }))
            }

            Event::FocusPrevTag => {
                let len = ctx.value.get().len();
                if len == 0 { return None; }
                let current_tag = ctx.focused_tag;
                let new_idx = match current_tag {
                    Some(idx) if idx > 0 => idx - 1,
                    Some(0) => 0,
                    None => len - 1,
                    _ => 0,
                };
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_tag = Some(new_idx);
                }).with_effect(PendingEffect::new("focus_tag", |ctx, _props, _send| {

                    if let Some(idx) = ctx.focused_tag {
                        let platform = use_platform_effects();
                        let tag_id = ctx.ids.item("tag", &idx);
                        platform.focus_element_by_id(&tag_id);
                    }
                    no_cleanup()
                })))
            }

            Event::FocusNextTag => {
                let len = ctx.value.get().len();
                let current_tag = ctx.focused_tag;
                match current_tag {
                    Some(idx) if idx + 1 < len => {
                        let next = idx + 1;
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.focused_tag = Some(next);
                        }).with_effect(PendingEffect::new("focus_tag", |ctx, _props, _send| {
                            if let Some(idx) = ctx.focused_tag {
                                let platform = use_platform_effects();
                                let tag_id = ctx.ids.item("tag", &idx);
                                platform.focus_element_by_id(&tag_id);
                            }
                            no_cleanup()
                        })))
                    }
                    Some(_) => {
                        // Past last tag → focus input
                        Some(TransitionPlan::context_only(|ctx| {
                            ctx.focused_tag = None;
                        }).with_effect(PendingEffect::new("focus_input", |ctx, _props, _send| {
                            let platform = use_platform_effects();
                            let input_id = ctx.ids.part("input");
                            platform.focus_element_by_id(&input_id);
                            no_cleanup()
                        })))
                    }
                    None => None,
                }
            }

            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = true; }))
            }
            Event::CompositionEnd => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = false; }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tags-input"]
pub enum Part {
    Root,
    Label,
    Control,
    Tag { index: usize },
    TagText { index: usize },
    TagDeleteTrigger { index: usize },
    TagEdit { index: usize },
    Input,
    ClearTrigger,
    HiddenInput,
    Description,
    ErrorMessage,
}

/// API for the TagsInput component.
///
/// The `Api` struct is created by `Machine::connect()` (defined in §1.6) and provides
/// per-part attribute methods and event handlers for adapter rendering.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {

    /// The on input focus handler.
    pub fn on_input_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    /// The on input blur handler.
    pub fn on_input_blur(&self) { (self.send)(Event::Blur); }

    /// The on input change handler.
    pub fn on_input_change(&self, val: String) { (self.send)(Event::InputChange(val)); }

    /// The on tag delete handler.
    pub fn on_tag_delete(&self, val: String) { (self.send)(Event::RemoveTag(val)); }

    /// The on clear click handler.
    pub fn on_clear_click(&self) { (self.send)(Event::ClearAll); }

    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        if self.ctx.focused { attrs.set_bool(HtmlAttr::Data("ars-focused"), true); }
        attrs
    }

    /// Attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// Attributes for the control wrapper.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "grid");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if let Some(max) = self.ctx.max {
            let current = self.ctx.value.get().len();
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.count_label)(current, max, &self.ctx.locale));
        }
        attrs
    }

    /// Returns the count text for the current tag count vs maximum.
    /// Returns `None` when no `max` is configured.
    pub fn count_text(&self) -> Option<String> {
        self.ctx.max.map(|max| {
            let current = self.ctx.value.get().len();
            (self.ctx.messages.count_label)(current, max, &self.ctx.locale)
        })
    }

    /// Attributes for a tag element.
    pub fn tag_attrs(&self, index: usize) -> AttrMap {
        let tag_id = self.ctx.ids.item("tag", index);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Tag { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set(HtmlAttr::Id, tag_id);
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
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.delete_hint)(&self.ctx.locale));
        }
        attrs
    }

    /// Attributes for the tag text element.
    pub fn tag_text_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TagText { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "gridcell");
        attrs
    }

    /// Attributes for the tag delete trigger.
    pub fn tag_delete_trigger_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TagDeleteTrigger { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "gridcell");
        if let Some(value) = self.ctx.value.get().get(index) {
            let label = (self.ctx.messages.remove_tag_label)(value, &self.ctx.locale);
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    /// Attributes for the inline tag edit input.
    pub fn tag_edit_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TagEdit { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let edit_id = self.ctx.ids.item("tag-edit-input", index);
        attrs.set(HtmlAttr::Id, edit_id);
        attrs.set(HtmlAttr::Type, "text");
        let is_editing = self.ctx.editing_tag == Some(index);
        if !is_editing {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }
        attrs
    }

    /// Attributes for the new-tag input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        if let Some(ref placeholder) = self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder);
        }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if self.props.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if let Some(max_len) = self.props.max_length {
            attrs.set(HtmlAttr::MaxLength, max_len.to_string());
        }
        // Wire aria-describedby to error-message (first) and description parts.
        let mut describedby_parts: Vec<String> = Vec::new();
        if self.ctx.invalid {
            describedby_parts.push(self.ctx.ids.part("error-message"));
        }
        describedby_parts.push(self.ctx.ids.part("description"));
        if !describedby_parts.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        }
        // Disable input when max tags reached
        if self.ctx.max.map_or(false, |m| self.ctx.value.get().len() >= m) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the clear-all trigger.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.clear_all_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    /// Attributes for the hidden form input.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        attrs.set(HtmlAttr::Name, self.ctx.name.as_deref().unwrap_or(""));
        let value_str = self.ctx.value.get().join(&self.ctx.delimiter);
        attrs.set(HtmlAttr::Value, value_str);
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.props.required { attrs.set_bool(HtmlAttr::Required, true); }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        if !self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }
        attrs
    }

    // — Event handlers —

    /// Handle focus on the input element.
    pub fn on_input_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    /// Handle blur on the input element.
    pub fn on_input_blur(&self) { (self.send)(Event::Blur); }

    /// Handle input value change.
    pub fn on_input_change(&self, val: String) { (self.send)(Event::InputChange(val)); }

    /// Handle paste into the input.
    pub fn on_input_paste(&self, text: String) { (self.send)(Event::Paste(text)); }

    /// Handle keydown on the input element.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if self.ctx.is_composing { return; }
        match data.key {
            KeyboardKey::Enter => {
                if !self.ctx.input_value.trim().is_empty() {
                    (self.send)(Event::AddTag(self.ctx.input_value.clone()));
                }
            }
            KeyboardKey::Backspace => {
                if self.ctx.input_value.is_empty() {
                    (self.send)(Event::FocusPrevTag);
                }
            }
            KeyboardKey::ArrowLeft => {
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

    /// Handle keydown on a focused tag.
    pub fn on_tag_keydown(&self, index: usize, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::RemoveTagAtIndex(index));
            }
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevTag),
            KeyboardKey::ArrowRight => (self.send)(Event::FocusNextTag),
            KeyboardKey::Escape => {
                (self.send)(Event::FocusNextTag); // deselect → focus input
            }
            KeyboardKey::Enter => {
                if self.props.editable {
                    (self.send)(Event::EditTag { index });
                }
            }
            _ => {}
        }
    }

    /// Handle click on a tag's delete trigger.
    pub fn on_tag_delete(&self, val: String) { (self.send)(Event::RemoveTag(val)); }

    /// Handle double-click on a tag (enter edit mode).
    pub fn on_tag_dblclick(&self, index: usize) {
        if self.props.editable {
            (self.send)(Event::EditTag { index });
        }
    }

    /// Handle click on the clear-all trigger.
    pub fn on_clear_click(&self) { (self.send)(Event::ClearAll); }

    /// Handle IME composition start.
    pub fn on_composition_start(&self) { (self.send)(Event::CompositionStart); }

    /// Handle IME composition end.
    pub fn on_composition_end(&self) { (self.send)(Event::CompositionEnd); }
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
        }
    }
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

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element            | Value                                                                            |
| ------------------ | ------------------ | -------------------------------------------------------------------------------- |
| `role`             | `Control`          | `grid` (enables proper AT navigation between tags and their delete actions)      |
| `aria-labelledby`  | `Control`          | Label id                                                                         |
| `role`             | `Tag`              | `row`                                                                            |
| `role`             | `TagText`          | `gridcell`                                                                       |
| `role`             | `TagDeleteTrigger` | `gridcell` (contains the remove button)                                          |
| `aria-describedby` | `Input`            | Description + ErrorMessage ids                                                   |
| `aria-label`       | `Tag`              | `"{value}"`                                                                      |
| `aria-disabled`    | `Tag`              | When tag or group is disabled                                                    |
| `aria-label`       | `TagDeleteTrigger` | `"Remove {value}"`                                                               |
| `aria-label`       | `ClearTrigger`     | `"Remove all tags"`                                                              |
| `aria-describedby` | `Tag`              | ID of shared visually hidden text: "Press Delete to remove" (localized via i18n) |

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

- **Live region announcement**: On tag deletion, emit an `aria-live="polite"` announcement:
  `"Removed {tag_label}"` (localized via `Messages`).
- **Tag `aria-label`**: Each Tag part MUST have `aria-label="{tag_label}, press Delete to remove"`
  (localized via `Messages.delete_hint`), combining the tag value with the
  removal instruction.
- **Max count reached**: When `ctx.value.get().len() >= ctx.max.unwrap_or(usize::MAX)`,
  announce `"Maximum of {max} tags reached"` via the live region. The input SHOULD also
  set `aria-disabled="true"` to indicate no more tags can be added.
- **Focus management after removal**: Focus follows the existing `focus_tag` effect —
  moves to the adjacent tag (same index, clamped to `len - 1`), or to the input if no
  tags remain. This behavior is already implemented in the `RemoveTag` and
  `RemoveTagAtIndex` transitions above.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Template for tag delete trigger label (default: "Remove {value}")
    pub remove_tag_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Clear all trigger label (default: "Remove all tags")
    pub clear_all_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Visually hidden instruction (default: "Press Delete to remove")
    pub delete_hint: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Count label when `max_tags` is set (default: "{current} of {max} tags").
    pub count_label: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            remove_tag_label: MessageFn::new(|value, _locale| format!("Remove {}", value)),
            clear_all_label: MessageFn::static_str("Remove all tags"),
            delete_hint: MessageFn::static_str("Press Delete to remove"),
            count_label: MessageFn::new(|current, max, _locale| format!("{} of {} tags", current, max)),
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

| Part              | ars-ui              | Ark UI              | React Aria               | Notes                                     |
| ----------------- | ------------------- | ------------------- | ------------------------ | ----------------------------------------- |
| Root              | `Root`              | `Root`              | `TagGroup`               | --                                        |
| Label             | `Label`             | `Label`             | `Label`                  | --                                        |
| Control           | `Control`           | `Control`           | `TagList`                | Wraps tags + input                        |
| Input             | `Input`             | `Input`             | --                       | React Aria has no input                   |
| Item (tag)        | `Item`              | `Item`              | `Tag`                    | --                                        |
| ItemPreview       | `ItemPreview`       | `ItemPreview`       | --                       | Tag visual container                      |
| ItemText          | `ItemText`          | `ItemText`          | --                       | --                                        |
| ItemDeleteTrigger | `ItemDeleteTrigger` | `ItemDeleteTrigger` | --                       | React Aria handles remove via render prop |
| ItemInput         | `ItemInput`         | `ItemInput`         | --                       | Inline edit input                         |
| ClearTrigger      | `ClearTrigger`      | `ClearTrigger`      | --                       | --                                        |
| HiddenInput       | `HiddenInput`       | `HiddenInput`       | --                       | Form submission                           |
| Description       | `Description`       | --                  | `Text[slot=description]` | --                                        |
| ErrorMessage      | `ErrorMessage`      | --                  | `FieldError`             | --                                        |

**Gaps:** None.

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
