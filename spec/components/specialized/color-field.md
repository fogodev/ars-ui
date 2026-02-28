---
component: ColorField
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker]
references:
  ark-ui: ColorPicker
  react-aria: ColorField
---

# ColorField

A text input for typing and editing color values. Operates in two modes:

1. **Whole-color mode** (default, `channel: None`): Accepts full color strings
   (`#ff0000`, `rgb(255,0,0)`, `hsl(0,100%,50%)`). Parsed on commit (blur/Enter)
   via `parse_color_string` from §1.1.
2. **Channel mode** (`channel: Some(ch)`): Numeric spinbutton for a single channel.
   Arrow keys increment/decrement by step using `channel_value`, `with_channel`,
   and `channel_range` from §1.1.

`ColorField` reuses `ColorValue`, `ColorChannel`, `ColorFormat`, and the shared helpers
(`channel_value`, `with_channel`, `channel_range`, `channel_step_default`,
`parse_color_string`, `format_color_string`) from the color-picker shared types (§1.1 of `color-picker.md`).

## 1. State Machine

### 1.1 States

```rust
/// The states for the ColorField component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Input is not focused.
    Idle,
    /// Input is focused, user may be editing text.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the ColorField component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Input received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Input lost focus — triggers commit.
    Blur,
    /// Raw text changed (keystroke or paste). No parsing until commit.
    Change(String),
    /// Enter key — parse and commit without leaving Focused.
    Commit,
    /// Programmatic value update from parent.
    SetValue(ColorValue),
    /// Programmatic invalid state.
    SetInvalid(bool),
    /// Channel mode: increment by step (ArrowUp).
    Increment,
    /// Channel mode: decrement by step (ArrowDown).
    Decrement,
    /// Channel mode: increment by large_step (PageUp).
    IncrementLarge,
    /// Channel mode: decrement by large_step (PageDown).
    DecrementLarge,
    /// Channel mode: snap to max (End).
    IncrementToMax,
    /// Channel mode: snap to min (Home).
    DecrementToMin,
    /// IME composition started.
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
}
```

### 1.3 Context

```rust
/// The context for the ColorField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled). `None` when empty.
    pub value: Bindable<Option<ColorValue>>,
    /// Raw text in the input. Diverges from `value` while editing.
    pub input_text: String,
    /// If `Some`, the field edits a single channel (numeric spinbutton).
    /// If `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,
    /// Display format for formatting value → text. Default: Hex.
    pub color_format: ColorFormat,
    /// Step size for channel-mode keyboard adjustment.
    pub step: f64,
    /// Large step size for channel-mode PageUp/PageDown.
    pub large_step: f64,
    /// Whether the input is focused.
    pub focused: bool,
    /// Whether focus was via keyboard (for focus-visible ring).
    pub focus_visible: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the current value is invalid.
    pub invalid: bool,
    /// Whether a value is required.
    pub required: bool,
    /// Whether IME composition is in progress.
    pub is_composing: bool,
    /// Whether a description part is rendered.
    pub has_description: bool,
    /// Form submission name.
    pub name: Option<String>,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component instance IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the ColorField component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,
    /// Default value for uncontrolled mode.
    pub default_value: Option<ColorValue>,
    /// If `Some`, the field edits a single channel (numeric spinbutton).
    /// If `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,
    /// Display format for whole-color mode. Default: Hex.
    pub color_format: ColorFormat,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the value is invalid (external validation).
    pub invalid: bool,
    /// Whether a value is required.
    pub required: bool,
    /// Form submission name.
    pub name: Option<String>,
    /// Step size for channel-mode keyboard adjustment.
    /// Default: `channel_step_default(ch)` when channel is set.
    pub step: Option<f64>,
    /// Large step size for channel-mode PageUp/PageDown.
    /// Default: `step * 10`.
    pub large_step: Option<f64>,
    /// Optional locale override. When `None`, resolved from the nearest
    /// `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            channel: None,
            color_format: ColorFormat::Hex,
            disabled: false,
            readonly: false,
            invalid: false,
            required: false,
            name: None,
            step: None,
            large_step: None,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// Format a color value for display in the input.
fn format_value(
    color: &ColorValue,
    channel: Option<ColorChannel>,
    color_format: ColorFormat,
) -> String {
    match channel {
        Some(ch) => {
            let val = channel_value(color, ch);
            match ch {
                ColorChannel::Hue => format!("{:.0}", val),
                ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
                    format!("{:.0}", val)
                }
                _ => format!("{:.0}", val * 100.0),
            }
        }
        None => format_color_string(color, color_format),
    }
}

/// Parse `input_text` and update `value`; reset `input_text` to formatted value.
/// Sets `invalid` if parsing fails.
fn commit_input(ctx: &mut Context) {
    match ctx.channel {
        Some(ch) => {
            // Channel mode: parse as f64.
            match ctx.input_text.trim().parse::<f64>() {
                Ok(raw) => {
                    let (min, max) = channel_range(ch);
                    let clamped = raw.clamp(min, max);
                    if let Some(color) = ctx.value.get() {
                        let new_color = with_channel(color, ch, clamped);
                        ctx.value.set(Some(new_color.clone()));
                        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                        ctx.invalid = false;
                    }
                }
                Err(_) => {
                    ctx.invalid = true;
                }
            }
        }
        None => {
            // Whole-color mode: parse via parse_color_string.
            if ctx.input_text.trim().is_empty() {
                ctx.value.set(None);
                ctx.invalid = ctx.required;
                return;
            }
            match parse_color_string(&ctx.input_text) {
                Some(color) => {
                    ctx.value.set(Some(color.clone()));
                    ctx.input_text = format_color_string(&color, ctx.color_format);
                    ctx.invalid = false;
                }
                None => {
                    ctx.invalid = true;
                }
            }
        }
    }
}

/// Adjust the channel value by `delta` (positive or negative), clamped to range.
fn adjust_channel(ctx: &mut Context, delta: f64) {
    if let (Some(ch), Some(color)) = (ctx.channel, ctx.value.get()) {
        let current = channel_value(color, ch);
        let (min, max) = channel_range(ch);
        let new_val = (current + delta).clamp(min, max);
        let new_color = with_channel(color, ch, new_val);
        ctx.value.set(Some(new_color.clone()));
        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
        ctx.invalid = false;
    }
}

/// The machine for the `ColorField` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(Some(v.clone())),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };

        let step = props.step.unwrap_or_else(|| {
            props.channel.map(channel_step_default).unwrap_or(1.0)
        });

        let large_step = props.large_step.unwrap_or(step * 10.0);

        let input_text = match value.get() {
            Some(c) => format_value(c, props.channel, props.color_format),
            None => String::new(),
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

        (State::Idle, Context {
            value,
            input_text,
            channel: props.channel,
            color_format: props.color_format,
            step,
            large_step,
            focused: false,
            focus_visible: false,
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            required: props.required,
            is_composing: false,
            has_description: false,
            name: props.name.clone(),
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // During IME composition, suppress all keyboard shortcuts.
        if ctx.is_composing {
            return match event {
                Event::CompositionEnd => Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = false;
                })),
                Event::Change(text) => {
                    let t = text.clone();
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.input_text = t;
                    }))
                }
                _ => None,
            };
        }

        if ctx.disabled {
            return match event {
                Event::Focus { is_keyboard } => {
                    let kb = *is_keyboard;

                    Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = kb;
                    }))
                }
                Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                })),
                _ => None,
            };
        }

        match event {
            Event::Focus { is_keyboard } => {
                let kb = *is_keyboard;

                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    commit_input(ctx);
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::Change(text) => {
                let t = text.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.input_text = t;
                }))
            }

            Event::Commit => {
                if ctx.readonly { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    commit_input(ctx);
                }))
            }

            Event::SetValue(color) => {
                let c = color.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    if !ctx.focused {
                        ctx.input_text = format_value(&c, ctx.channel, ctx.color_format);
                    }
                    ctx.value.set(Some(c));
                    ctx.invalid = false;
                }))
            }

            Event::SetInvalid(inv) => {
                let inv = *inv;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.invalid = inv;
                }))
            }

            Event::Increment => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = ctx.step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::Decrement => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = -ctx.step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementLarge => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = ctx.large_step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::DecrementLarge => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = -ctx.large_step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementToMax => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    if let (Some(ch), Some(color)) = (ctx.channel, ctx.value.get()) {
                        let (_, max) = channel_range(ch);
                        let new_color = with_channel(color, ch, max);
                        ctx.value.set(Some(new_color.clone()));
                        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                        ctx.invalid = false;
                    }
                }))
            }

            Event::DecrementToMin => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    if let (Some(ch), Some(color)) = (ctx.channel, ctx.value.get()) {
                        let (min, _) = channel_range(ch);
                        let new_color = with_channel(color, ch, min);
                        ctx.value.set(Some(new_color.clone()));
                        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                        ctx.invalid = false;
                    }
                }))
            }

            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = true;
                }))
            }

            Event::CompositionEnd => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = false;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-field"]
pub enum Part {
    Root,
    Label,
    Input,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// The connect API for the `ColorField` component.
pub struct Api<'a> {
    /// The current state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the component is currently focused.
    pub fn is_focused(&self) -> bool {
        matches!(self.state, State::Focused)
    }

    /// The current value of the component.
    pub fn value(&self) -> Option<&ColorValue> {
        self.ctx.value.get().as_ref()
    }

    /// The current input text of the component.
    pub fn input_text(&self) -> &str {
        &self.ctx.input_text
    }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// The attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let label_id = self.ctx.ids.part("label");
        attrs.set(HtmlAttr::Id, label_id);
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// The attributes for the input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let input_id = self.ctx.ids.part("input");
        attrs.set(HtmlAttr::Id, input_id);
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::Value, &self.ctx.input_text);

        // Mode-specific ARIA
        match self.ctx.channel {
            Some(ch) => {
                // Channel mode: numeric spinbutton
                attrs.set(HtmlAttr::Role, "spinbutton");
                attrs.set(HtmlAttr::InputMode, "numeric");
                if let Some(color) = self.ctx.value.get() {
                    let val = channel_value(color, ch);
                    let (min, max) = channel_range(ch);
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.2}", val));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{:.2}", min));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{:.2}", max));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueText),
                        (self.ctx.messages.channel_value_text)(ch, val, &self.ctx.locale));
                }
                attrs.set(HtmlAttr::Aria(AriaAttr::Label),
                    (self.ctx.messages.channel_label)(ch, &self.ctx.locale));
            }
            None => {
                // Whole-color mode: standard text input
                attrs.set(HtmlAttr::InputMode, "text");
                attrs.set(HtmlAttr::Aria(AriaAttr::Label),
                    (self.ctx.messages.color_label)(&self.ctx.locale));
            }
        }

        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Aria(AriaAttr::ReadOnly), true);
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set_bool(HtmlAttr::Aria(AriaAttr::Disabled), true);
        }

        // describedby: description + error message
        let mut describedby = Vec::new();
        if self.ctx.has_description {
            describedby.push(self.ctx.ids.part("description"));
        }
        if self.ctx.invalid {
            describedby.push(self.ctx.ids.part("error-message"));
        }
        if !describedby.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby.join(" "));
        }

        attrs
    }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Returns the error message text when color parsing fails.
    /// The adapter renders this inside the `ErrorMessage` part.
    pub fn invalid_message(&self) -> String {
        (self.ctx.messages.invalid_message)(&self.ctx.locale)
    }

    /// The attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        attrs
    }

    /// The attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(color) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, color.to_hex(true));
        }
        attrs
    }

    // --- Event dispatch helpers ---

    /// The on input focus handler.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// The on input blur handler.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// The on input change handler.
    pub fn on_input_change(&self, text: String) {
        (self.send)(Event::Change(text));
    }

    /// Handle keydown on the input element.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if self.ctx.is_composing { return; }
        match data.key {
            KeyboardKey::Enter => (self.send)(Event::Commit),
            KeyboardKey::ArrowUp if self.ctx.channel.is_some() => (self.send)(Event::Increment),
            KeyboardKey::ArrowDown if self.ctx.channel.is_some() => (self.send)(Event::Decrement),
            KeyboardKey::PageUp if self.ctx.channel.is_some() => (self.send)(Event::IncrementLarge),
            KeyboardKey::PageDown if self.ctx.channel.is_some() => (self.send)(Event::DecrementLarge),
            KeyboardKey::Home if self.ctx.channel.is_some() => (self.send)(Event::DecrementToMin),
            KeyboardKey::End if self.ctx.channel.is_some() => (self.send)(Event::IncrementToMax),
            _ => {}
        }
    }

    /// The on composition start handler.
    pub fn on_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// The on composition end handler.
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
            Part::Input => self.input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ColorField
├── Root             (<div>)        (required)
├── Label            (<label>)      (required)
├── Input            (<input>)      (required — type="text")
├── Description      (<div>)        (optional)
├── ErrorMessage     (<div>)        (optional — role="alert")
└── HiddenInput      (<input>)      (required — type="hidden", submits hex)
```

| Part         | Element   | Key Attributes                                                                        |
| ------------ | --------- | ------------------------------------------------------------------------------------- |
| Root         | `<div>`   | `data-ars-disabled`, `data-ars-readonly`, `data-ars-invalid`, `data-ars-focused`      |
| Label        | `<label>` | `for` pointing to Input                                                               |
| Input        | `<input>` | `type="text"`, `aria-labelledby`, `aria-invalid`, `aria-required`, `aria-describedby` |
| Description  | `<div>`   | Referenced by Input `aria-describedby`                                                |
| ErrorMessage | `<div>`   | `role="alert"`, referenced by Input `aria-describedby`                                |
| HiddenInput  | `<input>` | `type="hidden"`, `name`, `value` (hex)                                                |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute / Behaviour             | Element                  | Value                                          |
| --------------------------------- | ------------------------ | ---------------------------------------------- |
| `role="spinbutton"`               | Input (channel mode)     | ARIA spinbutton pattern                        |
| `inputmode="numeric"`             | Input (channel mode)     | Numeric keyboard on mobile                     |
| `inputmode="text"`                | Input (whole-color mode) | Text keyboard on mobile                        |
| `aria-valuenow`                   | Input (channel mode)     | Current channel value                          |
| `aria-valuemin` / `aria-valuemax` | Input (channel mode)     | From `channel_range(channel)`                  |
| `aria-valuetext`                  | Input (channel mode)     | Localized formatted channel value              |
| `aria-label`                      | Input (channel mode)     | Channel name (from messages)                   |
| `aria-labelledby`                 | Input                    | Label element ID                               |
| `aria-invalid`                    | Input                    | `"true"` when parse failed or external invalid |
| `aria-required`                   | Input                    | `"true"` when required                         |
| `aria-readonly`                   | Input                    | When read-only                                 |
| `aria-disabled` / `disabled`      | Input                    | When disabled                                  |
| `aria-describedby`                | Input                    | Description + ErrorMessage IDs                 |
| `role="alert"`                    | ErrorMessage             | Live error announcement                        |

### 3.2 Keyboard Interaction

| Key       | Context              | Action                  |
| --------- | -------------------- | ----------------------- |
| Enter     | Input                | Commit (parse value)    |
| ArrowUp   | Input (channel mode) | Increment by step       |
| ArrowDown | Input (channel mode) | Decrement by step       |
| PageUp    | Input (channel mode) | Increment by large_step |
| PageDown  | Input (channel mode) | Decrement by large_step |
| Home      | Input (channel mode) | Set to min              |
| End       | Input (channel mode) | Set to max              |

IME: All keyboard shortcuts are suppressed while `is_composing` is true.

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `ColorField` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for a channel input. Default: channel name (e.g., `"Hue"`).
    pub channel_label: MessageFn<dyn Fn(ColorChannel, &Locale) -> String + Send + Sync>,
    /// Formatted channel value for aria-valuetext.
    pub channel_value_text: MessageFn<dyn Fn(ColorChannel, f64, &Locale) -> String + Send + Sync>,
    /// Label for whole-color mode. Default: `"Color value"`.
    pub color_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Message shown when parsing fails. Default: `"Invalid color value"`.
    pub invalid_message: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            channel_label: MessageFn::new(|ch, _locale| format!("{ch:?}")),
            channel_value_text: MessageFn::new(|ch, val, _locale| match ch {
                ColorChannel::Hue => format!("{val:.0}°"),
                ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => format!("{val:.0}"),
                _ => format!("{:.0}%", val * 100.0),
            }),
            color_label: MessageFn::static_str("Color value"),
            invalid_message: MessageFn::static_str("Invalid color value"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                              | Default (en-US)                              | Purpose                         |
| -------------------------------- | -------------------------------------------- | ------------------------------- |
| `color_field.channel_label`      | Channel name (e.g., `"Hue"`)                 | Channel-mode `aria-label`       |
| `color_field.channel_value_text` | Channel-specific formatting (e.g., `"180°"`) | Channel-mode `aria-valuetext`   |
| `color_field.color_label`        | `"Color value"`                              | Whole-color mode label fallback |
| `color_field.invalid_message`    | `"Invalid color value"`                      | Error message text              |

- **Color format strings** (hex/rgb/hsl CSS syntax) are locale-independent.
- **Channel `aria-valuetext`** uses localized number formatting via message closures.
- **RTL**: No special layout handling needed (single text input).

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker.ChannelInput`), React Aria (`ColorField`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI            | React Aria               | Notes                                                 |
| ------------------------ | ------------------------- | ----------------- | ------------------------ | ----------------------------------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | (root-level)      | `value` / `defaultValue` | Equivalent                                            |
| `channel`                | `channel`                 | `channel`         | `channel`                | Equivalent                                            |
| `colorSpace`             | --                        | --                | `colorSpace`             | React Aria has colorSpace; ars-ui uses `color_format` |
| `isDisabled`             | `disabled`                | `disabled` (root) | `isDisabled`             | Equivalent                                            |
| `isReadOnly`             | `readonly`                | `readOnly` (root) | `isReadOnly`             | Equivalent                                            |
| `isRequired`             | `required`                | `required` (root) | `isRequired`             | Equivalent                                            |
| `isInvalid`              | `invalid`                 | `invalid` (root)  | `isInvalid`              | Equivalent                                            |
| `isWheelDisabled`        | --                        | --                | `isWheelDisabled`        | React Aria disables scroll wheel changes              |
| `placeholder`            | --                        | --                | `placeholder`            | React Aria has placeholder text                       |
| `name`                   | `name`                    | --                | `name`                   | Equivalent                                            |
| `validate`               | --                        | --                | `validate`               | React Aria has custom validation function             |
| `validationBehavior`     | --                        | --                | `validationBehavior`     | React Aria native/aria validation                     |

**Gaps:** None worth adopting. `isWheelDisabled` is niche (prevents accidental scroll-wheel changes). Custom `validate` is handled by `invalid` prop + application-level validation. `placeholder` can be set by the adapter.

### 5.2 Anatomy

| Part         | ars-ui         | Ark UI         | React Aria                  | Notes                 |
| ------------ | -------------- | -------------- | --------------------------- | --------------------- |
| Root         | `Root`         | --             | `ColorField`                | Equivalent            |
| Label        | `Label`        | --             | `Label`                     | Equivalent            |
| Input        | `Input`        | `ChannelInput` | `Input`                     | Equivalent            |
| Description  | `Description`  | --             | `Text` (slot="description") | Equivalent            |
| ErrorMessage | `ErrorMessage` | --             | `FieldError`                | Equivalent            |
| HiddenInput  | `HiddenInput`  | --             | --                          | ars-ui has form input |

**Gaps:** None.

### 5.3 Events

| Callback     | ars-ui                         | Ark UI                 | React Aria                             | Notes      |
| ------------ | ------------------------------ | ---------------------- | -------------------------------------- | ---------- |
| Value change | `Bindable` reactivity          | `onValueChange` (root) | `onChange`                             | Equivalent |
| Focus/Blur   | `Event::Focus` / `Event::Blur` | --                     | `onFocus` / `onBlur` / `onFocusChange` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                      | ars-ui                 | Ark UI            | React Aria            |
| ---------------------------- | ---------------------- | ----------------- | --------------------- |
| Whole-color mode             | Yes                    | No (channel only) | Yes                   |
| Channel mode (spinbutton)    | Yes                    | Yes               | Yes                   |
| IME composition handling     | Yes                    | --                | --                    |
| Keyboard increment/decrement | Yes                    | Yes               | Yes                   |
| Validation                   | Yes (external + parse) | --                | Yes (custom + native) |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui supports both whole-color and channel modes in a single component. Ark UI only has channel inputs as part of ColorPicker. React Aria has a dedicated ColorField with both modes.
- **Recommended additions:** None.
