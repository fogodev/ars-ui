---
component: TextField
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [textarea, password-input, search-input]
references:
    ark-ui: Field
    react-aria: TextField
---

# TextField

A single-line text input with label, description, error, and decorator slots.

**Placeholder as Label Fallback**: When a TextField has no explicit `label` and no `aria-label`/`aria-labelledby`, the `placeholder` text is used as a last-resort accessible name via `aria-label`. This is a fallback only — explicit labels are strongly preferred per WCAG 1.3.1. If neither label nor placeholder is provided, a development-mode warning should be emitted.

Accessible name resolution order (per the WAI-ARIA Name Computation):

1. `aria-labelledby` — explicit association via ID reference(s)
2. `aria-label` — string label directly on the element
3. `<label>` element — implicit (wrapping) or explicit (`for` attribute) association
4. `title` attribute — last resort tooltip-based name

> `placeholder` is NOT a valid accessible name fallback. It disappears when the user begins typing and is not reliably announced by all screen readers. The `placeholder`-to-`aria-label` promotion described above is an emergency fallback that SHOULD trigger a development-mode warning, not a recommended pattern.

## 1. State Machine

### 1.1 States

```rust
/// The states for the TextField component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the TextField component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received a focus event.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// The component received a blur event.
    Blur,
    /// The component received a change event.
    Change(String),
    /// The component received a clear event.
    Clear,
    /// The component received a set invalid event.
    SetInvalid(bool),
    /// IME composition started (CJK, etc.).
    CompositionStart,
    /// IME composition ended with the final committed value.
    CompositionEnd(String),
    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),
    /// Synchronize output-affecting props stored in context.
    SetProps,
    /// Track whether a Description part is rendered.
    SetHasDescription(bool),
}
```

### 1.3 Context

```rust
/// The input type of the component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputType {
    Text,
    Password,
    Email,
    Url,
    Tel,
    Search,
}

// `InputMode` — defined in `03-accessibility.md`

/// The context for the TextField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled value of the component.
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
    /// Whether the component has focus-visible.
    pub focus_visible: bool,
    /// The placeholder of the component.
    pub placeholder: Option<String>,
    /// The input type of the component.
    pub input_type: InputType,
    /// The maximum length of the component.
    pub max_length: Option<u32>,
    /// The minimum length of the component.
    pub min_length: Option<u32>,
    /// The pattern of the component.
    pub pattern: Option<String>,
    /// The autocomplete of the component.
    pub autocomplete: Option<String>,
    /// The name of the component.
    pub name: Option<String>,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Text direction for RTL support.
    pub dir: Direction,
    /// Mobile on-screen keyboard layout hint.
    pub input_mode: Option<InputMode>,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the TextField component.
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
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The placeholder of the component.
    pub placeholder: Option<String>,
    /// The input type of the component.
    pub input_type: InputType,
    /// The maximum length of the component.
    pub max_length: Option<u32>,
    /// The minimum length of the component.
    pub min_length: Option<u32>,
    /// The pattern of the component.
    pub pattern: Option<String>,
    /// The autocomplete of the component.
    pub autocomplete: Option<String>,
    /// The name of the component.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether the component is clearable.
    pub clearable: bool,
    /// The direction of the component.
    pub dir: Direction,
    /// Hint for the virtual keyboard type on mobile devices.
    pub input_mode: Option<InputMode>,
    /// Convenience callback fired with `true` on Focus and `false` on Blur.
    pub on_focus_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
    /// Callback fired when user interaction requests a value change.
    pub on_value_change: Option<Callback<dyn Fn(String) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: String::new(),
            disabled: false, readonly: false, invalid: false, required: false,
            placeholder: None, input_type: InputType::Text,
            max_length: None, min_length: None, pattern: None,
            autocomplete: None, name: None, form: None, clearable: false,
            dir: Direction::Ltr, input_mode: None,
            on_focus_change: None, on_value_change: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the TextField component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let state = State::Idle;
        let locale = env.locale.clone();
        let messages = messages.clone();
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            required: props.required,
            focused: false,
            focus_visible: false,
            placeholder: props.placeholder.clone(),
            input_type: props.input_type,
            max_length: props.max_length,
            min_length: props.min_length,
            pattern: props.pattern.clone(),
            autocomplete: props.autocomplete.clone(),
            name: props.name.clone(),
            is_composing: false,
            has_description: false,
            dir: props.dir,
            input_mode: props.input_mode.clone(),
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
        match event {
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }
            Event::Change(val) => {
                if ctx.disabled || ctx.readonly || ctx.is_composing { return None; }
                let val = val.clone();
                Some(TransitionPlan::context_only({
                    let val = val.clone();
                    move |ctx| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(val);
                        }
                    }
                }).with_effect(value_change_effect(val)))
            }
            Event::Clear => {
                if ctx.disabled || ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    if !ctx.value.is_controlled() {
                        ctx.value.set(String::new());
                    }
                }).with_effect(value_change_effect(String::new())))
            }
            Event::SetInvalid(invalid) => {
                let inv = *invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.invalid = inv;
                }))
            }
            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = true;
                }))
            }
            Event::CompositionEnd(final_value) => {
                let final_value = final_value.clone();
                let should_change = !ctx.disabled && !ctx.readonly;
                let mut plan = TransitionPlan::context_only({
                    let final_value = final_value.clone();
                    move |ctx| {
                        ctx.is_composing = false;
                        if should_change && !ctx.value.is_controlled() {
                            ctx.value.set(final_value);
                        }
                    }
                });
                if should_change {
                    plan = plan.with_effect(value_change_effect(final_value));
                }
                Some(plan)
            }
            Event::SetValue(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(value) = value {
                        ctx.value.set(value.clone());
                        ctx.value.sync_controlled(Some(value));
                    } else {
                        ctx.value.sync_controlled(None);
                    }
                }))
            }
            Event::SetProps => {
                let props = _props.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.placeholder = props.placeholder;
                    ctx.input_type = props.input_type;
                    ctx.max_length = props.max_length;
                    ctx.min_length = props.min_length;
                    ctx.pattern = props.pattern;
                    ctx.autocomplete = props.autocomplete;
                    ctx.name = props.name;
                    ctx.dir = props.dir;
                    ctx.input_mode = props.input_mode;
                }))
            }
            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.has_description = has_description;
                }))
            }
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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "text-field"]
pub enum Part {
    Root,
    Label,
    Input,
    StartDecorator,
    EndDecorator,
    ClearTrigger,
    Description,
    ErrorMessage,
}

/// The API for the TextField component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
        });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
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

    /// Attributes for the native input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Type, match self.ctx.input_type {
            InputType::Text => "text",
            InputType::Password => "password",
            InputType::Email => "email",
            InputType::Url => "url",
            InputType::Tel => "tel",
            InputType::Search => "search",
        });
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_str());
        if let Some(im) = self.resolved_input_mode() {
            attrs.set(HtmlAttr::InputMode, im.as_str());
        }
        attrs.set(HtmlAttr::Value, self.ctx.value.get());
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Required, true); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if let Some(p) = &self.ctx.placeholder { attrs.set(HtmlAttr::Placeholder, p); }
        if let Some(max) = self.ctx.max_length { attrs.set(HtmlAttr::MaxLength, max.to_string()); }
        if let Some(min) = self.ctx.min_length { attrs.set(HtmlAttr::MinLength, min.to_string()); }
        if let Some(pat) = &self.ctx.pattern { attrs.set(HtmlAttr::Pattern, pat); }
        if let Some(ac) = &self.ctx.autocomplete { attrs.set(HtmlAttr::AutoComplete, ac); }
        if let Some(name) = &self.ctx.name { attrs.set(HtmlAttr::Name, name); }
        if let Some(ref form) = self.props.form { attrs.set(HtmlAttr::Form, form); }
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if self.ctx.has_description {
            let mut describedby_parts = Vec::new();
            describedby_parts.push(self.ctx.ids.part("description"));
            if self.ctx.invalid {
                describedby_parts.push(self.ctx.ids.part("error-message"));
            }
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        } else if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), self.ctx.ids.part("error-message"));
        }
        attrs
    }

    /// Attributes for the start decorator slot.
    pub fn start_decorator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartDecorator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the end decorator slot.
    pub fn end_decorator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EndDecorator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the clear trigger button.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// Attributes for the description/help text element.
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
        attrs
    }

    pub fn on_input_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }
    pub fn on_input_blur(&self) { (self.send)(Event::Blur); }
    pub fn on_input_change(&self, val: String) {
        if !self.ctx.is_composing {
            (self.send)(Event::Change(val));
        }
    }
    pub fn on_input_composition_start(&self) { (self.send)(Event::CompositionStart); }
    pub fn on_input_composition_end(&self, final_value: String) {
        (self.send)(Event::CompositionEnd(final_value));
    }
    pub fn on_input_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.key == KeyboardKey::Escape
            && !data.is_composing
            && self.props.clearable
            && !self.ctx.disabled
            && !self.ctx.readonly
        {
            (self.send)(Event::Clear);
            return true;
        }
        false
    }
    pub fn on_clear_click(&self) { (self.send)(Event::Clear); }
    pub fn should_render_clear_trigger(&self) -> bool {
        self.props.clearable
            && !self.ctx.disabled
            && !self.ctx.readonly
            && !self.ctx.value.get().is_empty()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::StartDecorator => self.start_decorator_attrs(),
            Part::EndDecorator => self.end_decorator_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
TextField
├── Root               <div>     data-ars-scope="text-field" data-ars-part="root"
├── Label              <label>   data-ars-part="label"
├── StartDecorator     <div>     data-ars-part="start-decorator" (optional, aria-hidden)
├── Input         [A]  <input>   data-ars-part="input"
├── ClearTrigger       <button>  data-ars-part="clear-trigger" (optional, when clearable and non-empty)
├── EndDecorator       <div>     data-ars-part="end-decorator" (optional, aria-hidden)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part           | Element    | Key Attributes                                                                |
| -------------- | ---------- | ----------------------------------------------------------------------------- |
| Root           | `<div>`    | `data-ars-scope="text-field"`, `data-ars-state`                               |
| Label          | `<label>`  | `for` points to Input                                                         |
| Input          | `<input>`  | `type`, `aria-labelledby`, `aria-describedby`                                 |
| StartDecorator | `<div>`    | `aria-hidden="true"` — icon/prefix slot (optional)                            |
| EndDecorator   | `<div>`    | `aria-hidden="true"` — icon/suffix slot (optional)                            |
| ClearTrigger   | `<button>` | `aria-label="Clear"`, `type="button"` (optional when clearable and non-empty) |
| Description    | `<div>`    | Help text; linked via `aria-describedby` (optional)                           |
| ErrorMessage   | `<div>`    | Validation error; linked via `aria-describedby` (optional)                    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element | Value                                    |
| ------------------ | ------- | ---------------------------------------- |
| `aria-invalid`     | Input   | Present when `invalid=true`              |
| `aria-required`    | Input   | Implicit via `required` attribute        |
| `aria-labelledby`  | Input   | Points to Label id                       |
| `aria-describedby` | Input   | Points to Description + ErrorMessage ids |

### 3.2 Keyboard Interaction

| Key    | Action                       |
| ------ | ---------------------------- |
| Escape | Clear value (when clearable) |
| Tab    | Move focus to/from input     |

Adapters should render `ClearTrigger` only when `Api::should_render_clear_trigger()` returns
`true`, which requires `clearable=true`, an editable enabled field, and a non-empty value.

### 3.3 IME Composition Handling

During `compositionstart`...`compositionend`, adapters must suppress intermediate value changes and typeahead. The machine's context field `is_composing: bool` tracks composition state. On `compositionend`, adapters send the final element value as `CompositionEnd(final_value)`, so clearing the composition flag and applying/reporting the final value happen in one transition. `KeyboardEvent` handlers must check `event.isComposing` and skip processing when `true`.

See [IME Composition Protocol](./_category.md#ime-composition-protocol) for the shared input method editor behavior.

## 4. Internationalization

- `dir` attribute set from locale direction — ensures text entry direction matches language.
- `placeholder` text is user-provided and must be localized by consumer.
- `inputmode` attribute may differ by locale (e.g., numeric keyboard for Arabic phone numbers
  still uses standard `tel` inputmode).
- When `input_mode` is omitted, the core infers native keyboard hints from `input_type` for
  `Email`, `Url`, `Tel`, and `Search`; explicit `input_mode` always wins.

### 4.1 Messages

```rust
/// Locale-specific labels for the TextField component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the clear button (default: "Clear").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            clear_label: MessageFn::static_str("Clear"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 4.2 Mobile Virtual Keyboard Behavior

TextField must handle mobile virtual keyboard interactions:

| `input_type` | `inputmode` Attribute | Virtual Keyboard           |
| ------------ | --------------------- | -------------------------- |
| `Text`       | `"text"`              | Standard                   |
| `Email`      | `"email"`             | Email layout (@ key)       |
| `Tel`        | `"tel"`               | Telephone keypad           |
| `Url`        | `"url"`               | URL layout (/, .com)       |
| `Search`     | `"search"`            | Search layout (action key) |
| `Numeric`    | `"numeric"`           | Number pad                 |
| `Decimal`    | `"decimal"`           | Number pad with decimal    |

**Viewport Handling**:

- Avoid CSS `100vh` for layouts containing text fields — use `100dvh` (dynamic viewport height) or JavaScript-based measurement via `window.visualViewport.height`.
- Listen for `visualViewport` `resize` events to reposition any overlays (e.g., autocomplete dropdowns) when the keyboard appears or disappears.
- **iOS**: The keyboard triggers a viewport resize after a brief animation delay (~300ms). Overlays should defer repositioning until the resize event fires.
- **Android**: The keyboard triggers an immediate viewport resize. `resize` events may fire multiple times during keyboard animation.

## 5. Form Integration

- **Hidden input**: The `Input` part is the native `<input>` element and participates directly in form submission via its `name` attribute. No separate hidden input is needed.
- **Validation states**: `aria-invalid="true"` on the Input when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby`. Native constraint validation attributes (`required`, `minlength`, `maxlength`, `pattern`) are set directly on the input.
- **Error message association**: `aria-describedby` on Input points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: Native `required` attribute on Input.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: React Aria (`TextField`).
>
> Note: Ark UI `Field` is a generic wrapper, not a direct TextField equivalent. Radix UI has no TextField component.

### 6.1 Props

| Feature          | ars-ui                          | React Aria     | Notes              |
| ---------------- | ------------------------------- | -------------- | ------------------ |
| Controlled value | `value: Option<String>`         | `value`        | Full parity        |
| Default value    | `default_value: String`         | `defaultValue` | Full parity        |
| Disabled         | `disabled: bool`                | `isDisabled`   | Full parity        |
| Read-only        | `readonly: bool`                | `isReadOnly`   | Full parity        |
| Required         | `required: bool`                | `isRequired`   | Full parity        |
| Invalid          | `invalid: bool`                 | `isInvalid`    | Full parity        |
| Placeholder      | `placeholder: Option<String>`   | `placeholder`  | Full parity        |
| Input type       | `input_type: InputType`         | `type`         | Full parity        |
| Max length       | `max_length: Option<u32>`       | `maxLength`    | Full parity        |
| Min length       | `min_length: Option<u32>`       | `minLength`    | Full parity        |
| Pattern          | `pattern: Option<String>`       | `pattern`      | Full parity        |
| Autocomplete     | `autocomplete: Option<String>`  | `autoComplete` | Full parity        |
| Form name        | `name: Option<String>`          | `name`         | Full parity        |
| Form ID          | `form: Option<String>`          | `form`         | Full parity        |
| Input mode       | `input_mode: Option<InputMode>` | `inputMode`    | Full parity        |
| Clearable        | `clearable: bool`               | --             | ars-ui enhancement |

**Gaps:** None. React Aria's `validate`/`validationBehavior` are handled at the adapter layer.

### 6.2 Anatomy

| Part           | ars-ui           | React Aria          | Notes              |
| -------------- | ---------------- | ------------------- | ------------------ |
| Root           | `Root`           | `TextField`         | Full parity        |
| Label          | `Label`          | `Label`             | Full parity        |
| Input          | `Input`          | `Input`             | Full parity        |
| StartDecorator | `StartDecorator` | --                  | ars-ui enhancement |
| EndDecorator   | `EndDecorator`   | --                  | ars-ui enhancement |
| ClearTrigger   | `ClearTrigger`   | --                  | ars-ui enhancement |
| Description    | `Description`    | `Text[description]` | Full parity        |
| ErrorMessage   | `ErrorMessage`   | `FieldError`        | Full parity        |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui                                           | React Aria                              | Notes              |
| ------------- | ------------------------------------------------ | --------------------------------------- | ------------------ |
| Value changed | `Change(String)`                                 | `onChange`                              | Full parity        |
| Focus         | `Focus`/`Blur`                                   | `onFocus`/`onBlur`/`onFocusChange`      | Full parity        |
| Clear         | `Clear`                                          | --                                      | ars-ui enhancement |
| Composition   | `CompositionStart`/`CompositionEnd(final_value)` | `onCompositionStart`/`onCompositionEnd` | Full parity        |

**Gaps:** None.

### 6.4 Features

| Feature          | ars-ui             | React Aria        |
| ---------------- | ------------------ | ----------------- |
| Form integration | Yes (native input) | Built-in          |
| IME composition  | Yes                | Yes               |
| Clear button     | Yes                | --                |
| Decorator slots  | Yes                | --                |
| Validation       | Yes                | Yes (native+aria) |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with React Aria.
- **Divergences:** ars-ui adds decorator slots (StartDecorator, EndDecorator) and a ClearTrigger part not present in React Aria. ars-ui uses native `<input>` directly for form submission rather than a hidden input.
- **Recommended additions:** None.
