---
component: Editable
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
    ark-ui: Editable
---

# Editable

Inline text that switches to an input field for editing. The user double-clicks (or presses
Enter) to activate edit mode, types a new value, then submits with Enter or blur, or cancels
with Escape. This is useful for inline-rename patterns, editable table cells, and settings
that should be readable most of the time but editable on demand.

## 1. State Machine

### 1.1 States

```rust
/// The state of the Editable component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Default state — displays the current value as plain text.
    Preview,
    /// Active state — renders an input field for editing.
    Editing,
}
```

### 1.2 Events

```rust
/// Events for the Editable component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Enter edit mode (double-click, Enter key, or programmatic trigger).
    Activate,
    /// Confirm the current edit value and return to Preview.
    Submit(String),
    /// Discard changes and return to Preview.
    Cancel,
    /// Update the transient edit value while editing.
    Change(String),
    /// Focus received; flag indicates keyboard-initiated focus.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool
    },
    /// Focus lost.
    Blur,
    /// IME composition started.
    CompositionStart,
    /// IME composition ended with the final committed text.
    CompositionEnd(String),
    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),
    /// Synchronize output-affecting props stored in context.
    SetProps,
}
```

### 1.3 Context

```rust
/// Controls how the edit is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitMode {
    /// Submit when the input loses focus.
    Blur,
    /// Submit when Enter is pressed.
    Enter,
    /// Submit on either blur or Enter.
    Both,
    /// Never auto-submit; only the submit trigger button commits.
    None,
}

/// Controls how edit mode is activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivateMode {
    /// Single click activates.
    Click,
    /// Double-click activates.
    DblClick,
    /// Focus activates.
    Focus,
    /// Only the edit trigger button activates.
    None,
}

/// Context for the Editable component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The committed display value — controlled or uncontrolled.
    pub value: Bindable<String>,
    /// Transient value while editing; discarded on cancel.
    pub edit_value: String,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the editable is required.
    pub required: bool,
    /// Determines how the edit is submitted.
    pub submit_mode: SubmitMode,
    /// Determines how edit mode is activated.
    pub activate_mode: ActivateMode,
    /// When true, the input text is selected on activation.
    pub auto_select: bool,
    /// The placeholder text for the input.
    pub placeholder: Option<String>,
    /// The maximum length of the input.
    pub max_length: Option<usize>,
    /// The name for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether blur may submit when the submit mode includes blur.
    pub submit_on_blur: bool,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the editable.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// Props for the Editable component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Controlled value. When Some, component is controlled.
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
    /// The maximum length of the input.
    pub max_length: Option<usize>,
    /// Whether the editable is in an invalid state.
    pub invalid: bool,
    /// Whether the editable is required.
    pub required: bool,
    /// The name for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// When `true` (default), blurring the edit field submits the current value.
    /// When `false`, blurring cancels the edit and reverts to the previous value.
    /// This prop provides a convenient shorthand that interacts with `submit_mode`:
    /// setting `submit_on_blur: false` with `submit_mode: SubmitMode::Both` is
    /// equivalent to `submit_mode: SubmitMode::Enter`.
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

/// Messages for the Editable component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the input element. Default: `"Editable field"`.
    pub field_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the submit trigger. Default: `"Submit edit"`.
    pub submit_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the cancel trigger. Default: `"Cancel edit"`.
    pub cancel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the edit trigger. Default: `"Edit"`.
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
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
fn can_activate(ctx: &Context) -> bool { !ctx.disabled && !ctx.readonly }
fn effective_blur_submits(ctx: &Context) -> bool {
    ctx.submit_on_blur && matches!(ctx.submit_mode, SubmitMode::Blur | SubmitMode::Both)
}
fn clamp_to_max_chars(value: &str, max_length: Option<usize>) -> String {
    match max_length {
        Some(max_length) => value.chars().take(max_length).collect(),
        None => value.to_string(),
    }
}
```

### 1.6 Full Machine Implementation

```rust
/// Machine for the Editable component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;
    type Effect = NoEffect;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let initial = props.value.clone()
            .unwrap_or_else(|| props.default_value.clone());
        let state = State::Preview;
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
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
            locale,
            messages,
            ids: ComponentIds::from_id(&props.id),
        };
        (state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // --- Preview state ---
            (State::Preview, Event::Activate) => {
                if !can_activate(ctx) { return None; }
                let current_value = ctx.value.get().clone();
                Some(TransitionPlan::to(State::Editing).apply(move |ctx| {
                    ctx.edit_value = current_value;
                    ctx.focused = true;
                    ctx.focus_visible = false;
                }))
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }

            (State::Preview, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            // --- Editing state ---
            (State::Editing, Event::Change(val)) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing { return None; }
                let max_length = ctx.max_length;
                let val = val.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.edit_value = clamp_to_max_chars(&val, max_length);
                }))
            }

            (State::Editing, Event::Submit(val)) => {
                if ctx.disabled || ctx.readonly { return None; }
                let val = val.clone();
                let max_length = ctx.max_length;
                Some(TransitionPlan::to(State::Preview).apply(move |ctx| {
                    let val = clamp_to_max_chars(&val, max_length);
                    ctx.edit_value = val.clone();
                    if !ctx.value.is_controlled() {
                        ctx.value.set(val);
                    }
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.is_composing = false;
                }))
            }

            (State::Editing, Event::Cancel) => {
                let committed = ctx.value.get().clone();
                Some(TransitionPlan::to(State::Preview).apply(move |ctx| {
                    ctx.edit_value = committed;
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.is_composing = false;
                }))
            }

            (State::Editing, Event::Blur) => {
                let edit_value = ctx.edit_value.clone();
                let committed = ctx.value.get().clone();
                let should_submit = effective_blur_submits(ctx);
                Some(TransitionPlan::to(State::Preview).apply(move |ctx| {
                    if should_submit && !ctx.disabled && !ctx.readonly {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(edit_value.clone());
                        }
                        ctx.edit_value = edit_value;
                    } else {
                        ctx.edit_value = committed;
                    }
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.is_composing = false;
                }))
            }

            (_, Event::CompositionStart) => Some(TransitionPlan::context_only(|ctx| {
                ctx.is_composing = true;
            })),

            (State::Editing, Event::CompositionEnd(value)) => {
                let value = clamp_to_max_chars(value, ctx.max_length);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.is_composing = false;
                    if !ctx.disabled && !ctx.readonly {
                        ctx.edit_value = value;
                    }
                }))
            }

            (State::Preview, Event::CompositionEnd(_)) => Some(TransitionPlan::context_only(|ctx| {
                ctx.is_composing = false;
            })),

            (_, Event::SetValue(value)) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(value) = value {
                        let edit_value = clamp_to_max_chars(&value, ctx.max_length);
                        ctx.value.set(edit_value.clone());
                        ctx.value.sync_controlled(Some(value));
                        ctx.edit_value = edit_value;
                    } else {
                        ctx.value.sync_controlled(None);
                        ctx.edit_value = clamp_to_max_chars(ctx.value.get(), ctx.max_length);
                    }
                }))
            }

            (_, Event::SetProps) => {
                let props = _props.clone();
                Some(TransitionPlan::context_only(move |ctx| {
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
                    ctx.edit_value = clamp_to_max_chars(&ctx.edit_value, ctx.max_length);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(old.id, new.id, "editable::Props.id must remain stable after init");
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
#[scope = "editable"]
pub enum Part {
    Root,
    Label,
    Preview,
    Input,
    SubmitTrigger,
    CancelTrigger,
    EditTrigger,
}

/// API for the Editable component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the Editable is currently in editing mode.
    pub fn is_editing(&self) -> bool { matches!(self.state, State::Editing) }

    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Preview => "preview",
            State::Editing => "editing",
        });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
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

    /// Attributes for the preview text element.
    pub fn preview_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Preview.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Attributes for the input element (editing mode).
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.field_label)(&self.ctx.locale));
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        attrs.set(HtmlAttr::Value, &self.ctx.edit_value);
        if let Some(p) = &self.ctx.placeholder { attrs.set(HtmlAttr::Placeholder, p); }
        if let Some(max) = self.ctx.max_length { attrs.set(HtmlAttr::MaxLength, max.to_string()); }
        if let Some(ref name) = self.ctx.name { attrs.set(HtmlAttr::Name, name); }
        if let Some(ref form) = self.ctx.form { attrs.set(HtmlAttr::Form, form.as_str()); }
        attrs
    }

    /// Attributes for the submit trigger button.
    pub fn submit_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubmitTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.submit_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// Attributes for the cancel trigger button.
    pub fn cancel_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CancelTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.cancel_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// Attributes for the edit trigger button (Preview state).
    pub fn edit_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EditTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.edit_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    pub fn on_preview_click(&self) {
        if self.ctx.activate_mode == ActivateMode::Click {
            (self.send)(Event::Activate);
        }
    }
    pub fn on_preview_dblclick(&self) {
        if self.ctx.activate_mode == ActivateMode::DblClick {
            (self.send)(Event::Activate);
        }
    }
    pub fn on_preview_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
        if self.ctx.activate_mode == ActivateMode::Focus {
            (self.send)(Event::Activate);
        }
    }
    pub fn on_preview_blur(&self) { (self.send)(Event::Blur); }
    pub fn on_preview_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Enter && !self.is_keyboard_composing(data) {
            (self.send)(Event::Activate);
        }
    }
    pub fn on_input_change(&self, val: String) {
        if !self.ctx.is_composing {
            (self.send)(Event::Change(val));
        }
    }
    pub fn on_input_blur(&self) { (self.send)(Event::Blur); }
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, false);
    }
    pub fn on_input_keydown_after_composition_check(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, true);
    }
    pub fn on_input_composition_start(&self) { (self.send)(Event::CompositionStart); }
    pub fn on_input_composition_end(&self, final_value: String) {
        (self.send)(Event::CompositionEnd(final_value));
    }
    pub fn on_submit_click(&self) { (self.send)(Event::Submit(self.ctx.edit_value.clone())); }
    pub fn on_cancel_click(&self) { (self.send)(Event::Cancel); }
    pub fn on_edit_trigger_click(&self) { (self.send)(Event::Activate); }

    fn on_input_keydown_impl(&self, data: &KeyboardEventData, after_composition_check: bool) {
        let composing = self.is_keyboard_composing(data);
        match data.key {
            KeyboardKey::Process => (self.send)(Event::CompositionStart),
            KeyboardKey::Escape if !composing => (self.send)(Event::Cancel),
            KeyboardKey::Enter
                if (!composing || after_composition_check)
                    && !self.ctx.is_composing
                    && matches!(self.ctx.submit_mode, SubmitMode::Enter | SubmitMode::Both) =>
            {
                (self.send)(Event::Submit(self.ctx.edit_value.clone()));
            }
            KeyboardKey::Tab if !composing => {
                (self.send)(Event::Submit(self.ctx.edit_value.clone()));
            }
            _ => {}
        }
    }

    fn is_keyboard_composing(&self, data: &KeyboardEventData) -> bool {
        self.ctx.is_composing || data.is_composing || data.key == KeyboardKey::Process
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Preview => self.preview_attrs(),
            Part::Input => self.input_attrs(),
            Part::SubmitTrigger => self.submit_trigger_attrs(),
            Part::CancelTrigger => self.cancel_trigger_attrs(),
            Part::EditTrigger => self.edit_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Editable
├── Root               <div>     data-ars-scope="editable" data-ars-part="root" (role="group")
├── Label              <label>   data-ars-part="label" (optional)
├── Preview            <span>    data-ars-part="preview" (visible in Preview state)
├── Input              <input>   data-ars-part="input" (visible in Editing state)
├── EditTrigger        <button>  data-ars-part="edit-trigger" (optional, Preview state)
├── SubmitTrigger      <button>  data-ars-part="submit-trigger" (optional, Editing state)
└── CancelTrigger      <button>  data-ars-part="cancel-trigger" (optional, Editing state)
```

| Part          | Element    | Key Attributes                                         |
| ------------- | ---------- | ------------------------------------------------------ |
| Root          | `<div>`    | `role="group"`, `data-ars-state` ("preview"/"editing") |
| Label         | `<label>`  | `for` points to Input id (optional)                    |
| Preview       | `<span>`   | `tabindex="0"` — displays committed value              |
| Input         | `<input>`  | `aria-label`, value, maxlength                         |
| EditTrigger   | `<button>` | `aria-label` from `messages.edit_label` (optional)     |
| SubmitTrigger | `<button>` | `aria-label` from `messages.submit_label` (optional)   |
| CancelTrigger | `<button>` | `aria-label` from `messages.cancel_label` (optional)   |

- **Preview** is visible when `state == Preview`. It renders the committed value as plain text.
- **Input** is visible when `state == Editing`. It contains the transient `edit_value`.
- **EditTrigger** is optional — shown in Preview state to provide an explicit activation target.
- **SubmitTrigger** and **CancelTrigger** are optional — shown in Editing state for explicit
  confirm/cancel actions.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property            | Element | Value                                                   |
| ------------------- | ------- | ------------------------------------------------------- |
| `role`              | Root    | `group`                                                 |
| `aria-label`        | Input   | From `messages.field_label` (default: "Editable field") |
| `aria-required`     | Input   | Present when `required` is true                         |
| `aria-invalid`      | Input   | Present when `invalid` is true                          |
| `data-ars-disabled` | Root    | Present when `disabled` is true                         |
| `data-ars-readonly` | Root    | Present when `readonly` is true                         |
| `tabindex`          | Preview | `0` (focusable in Preview state)                        |

### 3.2 Keyboard Interaction

| Key      | Context                                 | Action                                              |
| -------- | --------------------------------------- | --------------------------------------------------- |
| `Enter`  | Preview focused                         | Activate editing                                    |
| `Enter`  | Editing (submit_mode `Enter` or `Both`) | Submit current value                                |
| `Escape` | Editing                                 | Cancel editing, revert to committed value           |
| `Tab`    | Editing                                 | Submit current value and move focus to next element |

### 3.3 Screen Reader Behavior

- When entering edit mode, the input receives focus and screen readers announce the editable
  field with its current value.
- Submit and cancel triggers are labeled with `aria-label` so screen readers announce their
  purpose.
- The root `group` role groups the preview, input, and triggers into a single logical widget.

### 3.4 IME Composition Handling

See [IME Composition Protocol](./_category.md#ime-composition-protocol) for the shared input method editor
behavior during text composition.

The `Editable` component must handle IME (Input Method Editor) composition correctly:

- Add `is_composing: bool` field to the Editable context/state.
- On `compositionstart`: set `is_composing = true`. Suppress value commit and `on_change` emission for the duration.
- On `compositionupdate`: update the visual display but do NOT commit the value or fire `on_change`.
- On `compositionend`: set `is_composing = false` and apply the final composed value through `Event::CompositionEnd`; adapter-level value-change callbacks are outside the agnostic core surface.
- **Enter key during composition**: If `is_composing` is true, Enter key confirms the IME candidate rather than committing the edit. The edit commit only happens on a subsequent Enter press (or blur) after composition ends.

## 4. Internationalization

### 4.1 Messages

Trigger labels are provided through the `Messages` struct:

- `field_label` — accessible label for the input (default: "Editable field")
- `submit_label` — submit trigger label (default: "Submit edit")
- `cancel_label` — cancel trigger label (default: "Cancel edit")
- `edit_label` — edit trigger label (default: "Edit")

See §1.4 for the `Messages` struct definition and `Default` implementation.

- `placeholder` is localized by the consumer.
- RTL: Input text direction follows the document locale. Trigger button positions may swap
  in RTL layouts; CSS logical properties handle this.

## 5. Form Integration

- **Form submission**: The Input part participates in form submission via its `name` attribute. The `form` prop associates the input with a form element by ID.
- **Validation states**: `aria-invalid="true"` on the Input when `invalid=true`. `aria-required="true"` on the Input when `required=true`.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value` and transitions to Preview state.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`Editable`).
>
> Note: Radix UI and React Aria have no Editable component.

### 6.1 Props

| Feature            | ars-ui                        | Ark UI                 | Notes          |
| ------------------ | ----------------------------- | ---------------------- | -------------- |
| Controlled value   | `value: Option<String>`       | `value`                | Full parity    |
| Default value      | `default_value: String`       | `defaultValue`         | Full parity    |
| Disabled           | `disabled: bool`              | `disabled`             | Full parity    |
| Read-only          | `readonly: bool`              | `readOnly`             | Full parity    |
| Invalid            | `invalid: bool`               | `invalid`              | Full parity    |
| Required           | `required: bool`              | `required`             | Full parity    |
| Activation mode    | `activate_mode: ActivateMode` | `activationMode`       | Full parity    |
| Submit mode        | `submit_mode: SubmitMode`     | `submitMode`           | Full parity    |
| Auto-select        | `auto_select: bool`           | `selectOnFocus`        | Full parity    |
| Placeholder        | `placeholder: Option<String>` | `placeholder`          | Full parity    |
| Max length         | `max_length: Option<usize>`   | `maxLength`            | Full parity    |
| Form name          | `name: Option<String>`        | `name`                 | Full parity    |
| Form ID            | `form: Option<String>`        | `form`                 | Full parity    |
| Auto-resize        | --                            | `autoResize`           | See note below |
| Start in edit mode | --                            | `defaultEdit` / `edit` | See note below |

**Gaps:**

- **`auto_resize`**: Ark UI supports auto-resizing the input to fit content. Low value for a single-line editable; consumer can apply CSS resize behavior.
- **Controlled edit state**: Ark UI supports `edit`/`defaultEdit` for controlling whether the component starts in edit mode. ars-ui uses `activate_mode: ActivateMode::None` + programmatic `Activate` event for the same effect.

### 6.2 Anatomy

| Part          | ars-ui          | Ark UI          | Notes                                                 |
| ------------- | --------------- | --------------- | ----------------------------------------------------- |
| Root          | `Root`          | `Root`          | Full parity                                           |
| Label         | `Label`         | `Label`         | Full parity                                           |
| Preview       | `Preview`       | `Preview`       | Full parity                                           |
| Input         | `Input`         | `Input`         | Full parity                                           |
| EditTrigger   | `EditTrigger`   | `EditTrigger`   | Full parity                                           |
| SubmitTrigger | `SubmitTrigger` | `SubmitTrigger` | Full parity                                           |
| CancelTrigger | `CancelTrigger` | `CancelTrigger` | Full parity                                           |
| Area          | --              | `Area`          | Ark wraps Input+Preview; ars-ui Root serves this role |
| Control       | --              | `Control`       | Ark wraps trigger buttons; ars-ui lays them flat      |

**Gaps:** None material. Ark's `Area` and `Control` are layout wrappers.

### 6.3 Events

| Callback          | ars-ui                    | Ark UI          | Notes       |
| ----------------- | ------------------------- | --------------- | ----------- |
| Value changed     | `Change(String)`          | `onValueChange` | Full parity |
| Value committed   | `Submit(String)`          | `onValueCommit` | Full parity |
| Value reverted    | `Cancel`                  | `onValueRevert` | Full parity |
| Edit mode changed | `Activate` / state change | `onEditChange`  | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature                     | ars-ui                             | Ark UI                             |
| --------------------------- | ---------------------------------- | ---------------------------------- |
| Activation modes            | Yes (Click, DblClick, Focus, None) | Yes (focus, dblclick, click, none) |
| Submit modes                | Yes (Blur, Enter, Both, None)      | Yes (enter, blur, none, both)      |
| Select on focus             | Yes                                | Yes                                |
| Form integration            | Yes                                | Yes                                |
| Edit/Cancel/Submit triggers | Yes                                | Yes                                |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with Ark UI Editable.
- **Divergences:** ars-ui uses `ActivateMode`/`SubmitMode` enums for type safety vs. Ark's string unions. Layout wrapper parts (`Area`, `Control`) are omitted in favor of flat composition.
- **Recommended additions:** None.
