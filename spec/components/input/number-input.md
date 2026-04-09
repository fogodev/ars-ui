---
component: NumberInput
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
    ark-ui: NumberInput
    react-aria: NumberField
---

# NumberInput

A numeric input with increment/decrement buttons and optional mouse scrubbing.

## 1. State Machine

### 1.1 States

```rust
/// The state of the NumberInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused,
    /// The component is in a scrubbing state.
    Scrubbing,
}
```

### 1.2 Events

```rust
/// The events for the NumberInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus { is_keyboard: bool },
    /// The component lost focus.
    Blur,
    /// The component's value changed.
    Change(String),
    /// The component's value was incremented.
    Increment,
    /// The component's value was decremented.
    Decrement,
    /// The component's value was incremented by a large amount.
    IncrementLarge,
    /// The component's value was decremented by a large amount.
    DecrementLarge,
    /// The component's value was incremented to the maximum value.
    IncrementToMax,
    /// The component's value was decremented to the minimum value.
    DecrementToMin,
    SetValue(f64),
    /// The component started scrubbing.
    StartScrub,
    /// The component was scrubbed.
    Scrub(f64),
    /// The component ended scrubbing.
    EndScrub,
    /// Mouse wheel input. Positive delta = increment, negative = decrement.
    Wheel { delta: f64 },
    /// The component started a composition session.
    CompositionStart,
    /// The component ended a composition session.
    CompositionEnd,
}
```

### 1.3 Context

```rust
/// Fine-grained formatting options for NumberInput display value.
pub struct NumberFormatOptions {
    /// Minimum fraction digits to display. Default: 0.
    pub minimum_fraction_digits: Option<u8>,
    /// Maximum fraction digits to display. Default: locale-dependent.
    pub maximum_fraction_digits: Option<u8>,
    /// Whether to use grouping separators (e.g., 1,000). Default: true.
    pub use_grouping: Option<bool>,
    /// Currency code (ISO 4217) if formatting as currency. e.g., "USD".
    pub currency: Option<String>,
    /// Currency display style: "symbol" ($), "code" (USD), "name" (US Dollar).
    pub currency_display: Option<String>,
}

/// The context of the NumberInput component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current value of the component.
    pub value: Bindable<Option<f64>>,
    /// The minimum value of the component.
    pub min: f64,
    /// The maximum value of the component.
    pub max: f64,
    /// The step size of the component.
    pub step: f64,
    /// The large step size of the component.
    pub large_step: f64,
    /// The precision of the component.
    pub precision: Option<u32>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether the component has focus-visible.
    pub focus_visible: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// Whether holding increment/decrement repeats the action with acceleration.
    pub spin_on_press: bool,
    /// Whether the component is scrubbing.
    pub scrubbing: bool,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the number input.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

> **IME composition:** This component tracks `is_composing: bool` in Context. During composition (`is_composing == true`), all custom keyboard handlers are suppressed. See §IME in `03-accessibility.md`.

### 1.4 Props

```rust
/// Props for the NumberInput component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Controlled value. When Some, component is controlled.
    pub value: Option<f64>,
    /// Default value for uncontrolled mode.
    pub default_value: Option<f64>,
    /// The minimum value.
    pub min: f64,
    /// The maximum value.
    pub max: f64,
    /// The step size.
    pub step: f64,
    /// The large step size (PageUp/PageDown).
    pub large_step: f64,
    /// The precision (decimal places).
    pub precision: Option<u32>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether the component allows mouse wheel input.
    pub allow_mouse_wheel: bool,
    /// Whether the component clamps the value on blur.
    pub clamp_value_on_blur: bool,
    /// When `true`, holding the increment/decrement button repeats the action
    /// with accelerating speed (press-and-hold to spin). Default: `true`.
    pub spin_on_press: bool,
    /// Fine-grained number formatting options for parsing.
    pub format_options: Option<NumberFormatOptions>,
    /// Display formatting options applied when the input is not focused.
    pub display_format: Option<NumberFormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: None,
            min: f64::NEG_INFINITY, max: f64::INFINITY,
            step: 1.0, large_step: 10.0, precision: None,
            disabled: false, readonly: false, invalid: false, required: false,
            name: None, form: None, allow_mouse_wheel: false, clamp_value_on_blur: true, spin_on_press: true,
            format_options: None, display_format: None,
        }
    }
}

/// Messages for the NumberInput component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the increment button. Default: `"Increment"`.
    pub increment_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the decrement button. Default: `"Decrement"`.
    pub decrement_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            increment_label: MessageFn::static_str("Increment"),
            decrement_label: MessageFn::static_str("Decrement"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.5 Rounding Mode Specification

All floating-point rounding in NumberInput and Slider MUST use **round-half-up** (standard arithmetic rounding) as the rounding mode. This is the `RoundingMode::HalfUp` equivalent: if the digit being dropped is exactly 5, round away from zero.

**Rationale**: Banker's rounding (`round-half-even`) is used in financial accounting to minimize cumulative bias, but it produces counterintuitive results for end users (e.g., `2.5` rounds to `2`, not `3`). Interactive UI components prioritize user expectation over statistical properties.

```rust
/// Round `value` to `precision` decimal places using round-half-up.
fn round_to_precision(value: f64, precision: Option<u32>) -> f64 {
    match precision {
        Some(p) => {
            let factor = 10_f64.powi(p as i32);
            (value * factor + 0.5_f64.copysign(value)).trunc() / factor
        }
        None => value,
    }
}
```

| Input   | Precision | Result | Explanation                |
| ------- | --------- | ------ | -------------------------- |
| `2.5`   | 0         | `3`    | Half rounds up             |
| `-2.5`  | 0         | `-3`   | Half rounds away from zero |
| `0.105` | 2         | `0.11` | Half-up at 3rd decimal     |
| `0.115` | 2         | `0.12` | Half-up at 3rd decimal     |
| `1.005` | 2         | `1.01` | Half-up at 3rd decimal     |

**Application Order**: Precision rounding is applied AFTER clamping to min/max but BEFORE storing in Context.

### 1.6 Full Machine Implementation

```rust
/// The machine for the NumberInput component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let state = State::Idle;
        let ctx = Context {
            value: match props.value {
                Some(v) => Bindable::controlled(Some(v)),
                None => Bindable::uncontrolled(props.default_value),
            },
            min: props.min,
            max: props.max,
            step: props.step,
            large_step: props.large_step,
            precision: props.precision,
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            required: props.required,
            focused: false,
            focus_visible: false,
            name: props.name.clone(),
            spin_on_press: props.spin_on_press,
            scrubbing: false,
            is_composing: false,
            has_description: false,
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
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            match event {
                Event::Increment | Event::Decrement
                | Event::IncrementLarge | Event::DecrementLarge
                | Event::IncrementToMax | Event::DecrementToMin
                | Event::Change(_) | Event::SetValue(_)
                | Event::StartScrub | Event::Scrub(_) | Event::EndScrub
                | Event::Wheel { .. } => return None,
                _ => {}
            }
        }

        match event {
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => {
                let clamp_min = ctx.min;
                let clamp_max = ctx.max;
                let current_val = *ctx.value.get();
                let clamp_on_blur = props.clamp_value_on_blur;
                let precision = ctx.precision;
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    if clamp_on_blur {
                        if let Some(val) = current_val {
                            let clamped = val.clamp(clamp_min, clamp_max);
                            if clamped != val {
                                ctx.value.set(Some(round_to_precision(clamped, precision)));
                            }
                        }
                    }
                }))
            }
            Event::Increment => {
                let current = ctx.value.get().unwrap_or(ctx.min);
                let next = round_to_precision((current + ctx.step).min(ctx.max), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(next)); }))
            }
            Event::Decrement => {
                let current = ctx.value.get().unwrap_or(ctx.min);
                let prev = round_to_precision((current - ctx.step).max(ctx.min), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(prev)); }))
            }
            Event::IncrementLarge => {
                let current = ctx.value.get().unwrap_or(ctx.min);
                let next = round_to_precision((current + ctx.large_step).min(ctx.max), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(next)); }))
            }
            Event::DecrementLarge => {
                let current = ctx.value.get().unwrap_or(ctx.min);
                let prev = round_to_precision((current - ctx.large_step).max(ctx.min), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(prev)); }))
            }
            Event::IncrementToMax => {
                let max = ctx.max;
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(max)); }))
            }
            Event::DecrementToMin => {
                let min = ctx.min;
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(min)); }))
            }
            Event::SetValue(val) => {
                let rounded = round_to_precision(val.clamp(ctx.min, ctx.max), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(rounded)); }))
            }
            Event::Change(text) => {
                let text = text.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    if text.is_empty() {
                        ctx.value.set(None);
                    } else if let Ok(val) = text.parse::<f64>() {
                        ctx.value.set(Some(val));
                    }
                }))
            }
            Event::StartScrub => {
                Some(TransitionPlan::to(State::Scrubbing).apply(|ctx| { ctx.scrubbing = true; }))
            }
            Event::Scrub(delta) if matches!(state, State::Scrubbing) => {
                let current = ctx.value.get().unwrap_or(0.0);
                let next = round_to_precision((current + delta * ctx.step).clamp(ctx.min, ctx.max), ctx.precision);
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(Some(next)); }))
            }
            Event::EndScrub => {
                let is_focused = ctx.focused;
                Some(TransitionPlan::to(if is_focused { State::Focused } else { State::Idle })
                    .apply(|ctx| { ctx.scrubbing = false; }))
            }
            Event::Wheel { delta } if matches!(state, State::Focused) && !ctx.readonly && props.allow_mouse_wheel => {
                let event = if *delta > 0.0 { Event::Increment } else { Event::Decrement };
                Self::transition(state, &event, ctx, props)
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
#[scope = "number-input"]
pub enum Part {
    Root,
    Label,
    Input,
    IncrementTrigger,
    DecrementTrigger,
    Description,
    ErrorMessage,
}

/// The API for the NumberInput component.
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
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
            State::Scrubbing => "scrubbing",
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
        attrs.set(HtmlAttr::Role, "spinbutton");
        attrs.set(HtmlAttr::InputMode, "decimal");
        if let Some(val) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), val.to_string());
        }
        if self.ctx.min.is_finite() {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
        }
        if self.ctx.max.is_finite() {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
        }
        // aria-valuetext should be set by the adapter using locale-formatted value
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
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if let Some(ref name) = self.ctx.name { attrs.set(HtmlAttr::Name, name); }
        if let Some(ref form) = self.props.form { attrs.set(HtmlAttr::Form, form); }
        attrs
    }

    /// Attributes for the increment trigger button.
    /// **Spin-on-press behavior:** When `ctx.spin_on_press` is `true`, the adapter
    /// attaches `pointerdown` on increment/decrement triggers to start a repeating
    /// timer that sends `Event::Increment`/`Event::Decrement` with accelerating
    /// speed: initial delay 400ms, then 50ms intervals. The timer stops on
    /// `pointerup`/`pointerleave`. When `false`, each click sends a single event.
    pub fn increment_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::IncrementTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.increment_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        // Disable at max boundary
        if let Some(val) = self.ctx.value.get() {
            if *val >= self.ctx.max { attrs.set_bool(HtmlAttr::Disabled, true); }
        }
        attrs
    }

    /// Attributes for the decrement trigger button.
    pub fn decrement_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DecrementTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.decrement_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        // Disable at min boundary
        if let Some(val) = self.ctx.value.get() {
            if *val <= self.ctx.min { attrs.set_bool(HtmlAttr::Disabled, true); }
        }
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
    pub fn on_input_change(&self, text: String) { (self.send)(Event::Change(text)); }
    pub fn on_increment_click(&self) { (self.send)(Event::Increment); }
    pub fn on_decrement_click(&self) { (self.send)(Event::Decrement); }

    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        let event = match data.key {
            KeyboardKey::ArrowUp => Some(Event::Increment),
            KeyboardKey::ArrowDown => Some(Event::Decrement),
            KeyboardKey::PageUp => Some(Event::IncrementLarge),
            KeyboardKey::PageDown => Some(Event::DecrementLarge),
            KeyboardKey::Home => Some(Event::DecrementToMin),
            KeyboardKey::End => Some(Event::IncrementToMax),
            _ => None,
        };
        if let Some(e) = event { (self.send)(e); }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::IncrementTrigger => self.increment_trigger_attrs(),
            Part::DecrementTrigger => self.decrement_trigger_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
NumberInput
├── Root               <div>     data-ars-scope="number-input" data-ars-part="root"
├── Label              <label>   data-ars-part="label"
├── Input         [A]  <input>   data-ars-part="input" (role="spinbutton")
├── IncrementTrigger   <button>  data-ars-part="increment-trigger" (optional)
├── DecrementTrigger   <button>  data-ars-part="decrement-trigger" (optional)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part             | Element    | Key Attributes                                             |
| ---------------- | ---------- | ---------------------------------------------------------- |
| Root             | `<div>`    | `data-ars-scope="number-input"`, `data-ars-state`          |
| Label            | `<label>`  | `for` points to Input                                      |
| Input            | `<input>`  | `role="spinbutton"`, `aria-valuenow/min/max/text`          |
| IncrementTrigger | `<button>` | `aria-label`, disabled at max boundary (optional)          |
| DecrementTrigger | `<button>` | `aria-label`, disabled at min boundary (optional)          |
| Description      | `<div>`    | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage     | `<div>`    | Validation error; linked via `aria-describedby` (optional) |

**Press-and-hold**: When the user holds a stepper button, the adapter starts a repeat timer
(initial delay 400ms, then repeat every 60ms) sending `Event::Increment` or
`Event::Decrement`. The timer is cancelled on pointerup or pointerleave.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element | Value                                    |
| ------------------ | ------- | ---------------------------------------- |
| `role`             | Input   | `spinbutton`                             |
| `aria-valuenow`    | Input   | Current numeric value                    |
| `aria-valuemin`    | Input   | Minimum value (when finite)              |
| `aria-valuemax`    | Input   | Maximum value (when finite)              |
| `aria-valuetext`   | Input   | Locale-formatted value string            |
| `aria-required`    | Input   | Present when `required=true`             |
| `aria-invalid`     | Input   | Present when `invalid=true`              |
| `aria-labelledby`  | Input   | Points to Label id                       |
| `aria-describedby` | Input   | Points to Description + ErrorMessage ids |

### 3.2 Keyboard Interaction

| Key            | Action                     |
| -------------- | -------------------------- |
| ArrowUp        | Increment by step          |
| ArrowDown      | Decrement by step          |
| PageUp         | Increment by large step    |
| PageDown       | Decrement by large step    |
| Home           | Set to minimum             |
| End            | Set to maximum             |
| Tab            | Move focus                 |
| Ctrl+V / Cmd+V | Paste (locale-aware parse) |

### 3.3 Paste Event Handling

When the user pastes content into the NumberInput, the adapter intercepts the `paste` event:

1. **Extract clipboard text**: Read plain-text content from the clipboard.
2. **Strip whitespace**: Trim leading/trailing whitespace.
3. **Locale-aware parse**: Pass the trimmed text through `NumberFormatter::parse()` using the resolved locale. This handles locale-specific decimal separators, grouping separators, currency symbols, and percentage signs.
4. **Validation**: If `parse()` returns `Ok(value)`: clamp to `[min, max]`, round to `precision`, send `Event::SetValue(parsed_value)`, call `event.preventDefault()`.
5. **Rejection**: If `parse()` returns `Err`: call `event.preventDefault()` to reject the paste entirely. Optionally announce "Invalid paste content" via a live region.

### 3.4 Scrubbing Accessibility

During Scrubbing state, the input's `aria-valuenow` updates in real time.
Adapters may throttle `aria-valuetext` updates to avoid excessive screen reader
announcements.

> **Passive wheel listener requirement:** When `allow_mouse_wheel: true`, the adapter attaches a `wheel` event listener with `{ passive: false }` to ensure `preventDefault()` is not silently ignored. This mirrors the passive listener guidance for touch events in `05-interactions.md` §2.4.

## 4. Internationalization

- **Locale resolution**: The adapter resolves locale from `ArsProvider` and passes it
  via `Env.locale` to `Machine::init()`, which stores it in `Context.locale`. All formatting
  and parsing uses this resolved locale.
- **Decimal separator**: Uses locale-appropriate separator (`,` vs `.`).
- **Thousands separator**: Applied when formatting the displayed value.
- **`aria-valuetext`**: Formatted using `NumberFormatter` from `ars-i18n` with the resolved locale.
- **RTL**: Increment/decrement button positions swap visually (CSS `direction` handles this).
- **Input parsing**: Must accept both locale-specific input (e.g., `1.234,56` in `de-DE`) and
  canonical format (e.g., `1234.56`). On blur, the value is normalized to the canonical `f64`
  and the display is re-formatted per locale.

### 4.1 Locale-Aware Number Formatting

`NumberInput` MUST support locale-aware formatting and parsing of numeric values:

- `en-US`: `1,234.56` (comma grouping, dot decimal)
- `de-DE`: `1.234,56` (dot grouping, comma decimal)
- `fr-FR`: `1 234,56` (narrow no-break space grouping, comma decimal)

**Locale Resolution Chain**:

1. `locale` prop (highest priority)
2. `FormContext.locale` (if inside a Form)
3. `ArsProvider` context
4. Fallback: `en-US`

**Non-Uniform Digit Grouping**: Number formatting and parsing delegate to ICU4X
`NumberFormatter`, which handles locale-specific grouping (including non-uniform patterns
like Indian numbering: `12,34,567`). The component must not hardcode grouping assumptions.

**Negative Number Sign Placement**: Negative number display follows locale conventions via
ICU4X — leading minus, trailing minus, or accounting parentheses. Adapters must not hardcode minus sign position.

## 5. Form Integration

- **Hidden input**: The `Input` part is the native `<input>` element and participates directly in form submission via its `name` attribute. The submitted value is the raw numeric string.
- **Validation states**: `aria-invalid="true"` on the Input when `invalid=true`. Native constraint validation is supplemented by the `min`/`max` bounds and `precision` rounding.
- **Error message association**: `aria-describedby` on Input points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: `aria-required="true"` on Input.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`NumberInput`), React Aria (`NumberField`).

### 6.1 Props

| Feature                    | ars-ui                                        | Ark UI             | React Aria                                | Notes                                  |
| -------------------------- | --------------------------------------------- | ------------------ | ----------------------------------------- | -------------------------------------- |
| Controlled value           | `value: Option<f64>`                          | `value: string`    | `value: number`                           | Full parity (ars-ui uses numeric type) |
| Default value              | `default_value: Option<f64>`                  | `defaultValue`     | `defaultValue`                            | Full parity                            |
| Min/Max                    | `min`/`max`                                   | `min`/`max`        | `minValue`/`maxValue`                     | Full parity                            |
| Step                       | `step: f64`                                   | `step`             | `step`                                    | Full parity                            |
| Large step                 | `large_step: f64`                             | --                 | --                                        | ars-ui enhancement for PageUp/PageDown |
| Disabled                   | `disabled: bool`                              | `disabled`         | `isDisabled`                              | Full parity                            |
| Read-only                  | `readonly: bool`                              | `readOnly`         | `isReadOnly`                              | Full parity                            |
| Required                   | `required: bool`                              | `required`         | `isRequired`                              | Full parity                            |
| Invalid                    | `invalid: bool`                               | `invalid`          | `isInvalid`                               | Full parity                            |
| Form name                  | `name: Option<String>`                        | `name`             | `name`                                    | Full parity                            |
| Form ID                    | `form: Option<String>`                        | `form`             | `form`                                    | Full parity                            |
| Mouse wheel                | `allow_mouse_wheel: bool`                     | `allowMouseWheel`  | `isWheelDisabled` (inverted)              | Full parity (inverted boolean)         |
| Clamp on blur              | `clamp_value_on_blur: bool`                   | `clampValueOnBlur` | --                                        | Ark parity                             |
| Spin on press              | `spin_on_press: bool`                         | `spinOnPress`      | --                                        | Ark parity                             |
| Format options             | `format_options: Option<NumberFormatOptions>` | `formatOptions`    | `formatOptions`                           | Full parity                            |
| Locale                     | via `Env.locale` (adapter-resolved)           | `locale`           | --                                        | Ark parity (adapter prop, not core)    |
| Increment/decrement labels | via `Messages`                                | `translations`     | `incrementAriaLabel`/`decrementAriaLabel` | Full parity                            |

**Gaps:** None.

### 6.2 Anatomy

| Part             | ars-ui                | Ark UI             | React Aria          | Notes                                                            |
| ---------------- | --------------------- | ------------------ | ------------------- | ---------------------------------------------------------------- |
| Root             | `Root`                | `Root`             | `NumberField`       | Full parity                                                      |
| Label            | `Label`               | `Label`            | `Label`             | Full parity                                                      |
| Input            | `Input`               | `Input`            | `Input`             | Full parity                                                      |
| IncrementTrigger | `IncrementTrigger`    | `IncrementTrigger` | `Button[increment]` | Full parity                                                      |
| DecrementTrigger | `DecrementTrigger`    | `DecrementTrigger` | `Button[decrement]` | Full parity                                                      |
| Scrubber         | -- (scrub via events) | `Scrubber`         | --                  | ars-ui uses Scrubbing state + events instead of a dedicated part |
| Control          | --                    | `Control`          | `Group`             | Ark/RA wrapper; ars-ui Root serves this role                     |
| ValueText        | --                    | `ValueText`        | --                  | Ark display part; ars-ui uses input value directly               |
| Description      | `Description`         | --                 | `Text[description]` | ars-ui form-field part                                           |
| ErrorMessage     | `ErrorMessage`        | --                 | `FieldError`        | ars-ui form-field part                                           |

**Gaps:** None material. Ark's `Scrubber` is a dedicated part; ars-ui achieves the same via `StartScrub`/`Scrub`/`EndScrub` events on the `Input` element. Ark's `ValueText` is a read-only display; ars-ui formats the value directly in the input.

### 6.3 Events

| Callback                 | ars-ui                          | Ark UI              | React Aria      | Notes                                  |
| ------------------------ | ------------------------------- | ------------------- | --------------- | -------------------------------------- |
| Value changed            | `Change(String)`                | `onValueChange`     | `onChange`      | Full parity                            |
| Value committed          | via `Blur` + clamp              | `onValueCommit`     | --              | Ark parity                             |
| Value invalid (overflow) | via validation                  | `onValueInvalid`    | --              | Ark parity (handled via invalid state) |
| Focus change             | `Focus`/`Blur`                  | `onFocusChange`     | `onFocusChange` | Full parity                            |
| Scrub                    | `StartScrub`/`Scrub`/`EndScrub` | (via Scrubber part) | --              | Ark parity                             |

**Gaps:** None.

### 6.4 Features

| Feature                 | ars-ui                                  | Ark UI | React Aria   |
| ----------------------- | --------------------------------------- | ------ | ------------ |
| Keyboard inc/dec        | Yes (arrows, PageUp/PageDown, Home/End) | Yes    | Yes          |
| Press-and-hold spin     | Yes                                     | Yes    | Yes          |
| Mouse wheel             | Yes                                     | Yes    | Yes (opt-in) |
| Scrubbing               | Yes                                     | Yes    | --           |
| Locale-aware formatting | Yes                                     | Yes    | Yes          |
| Clamp on blur           | Yes                                     | Yes    | --           |
| Form integration        | Yes                                     | Yes    | Built-in     |
| IME composition         | Yes                                     | --     | --           |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with both reference libraries.
- **Divergences:** ars-ui uses `Option<f64>` for the value type (allowing empty state) instead of Ark's string-based representation. ars-ui adds `large_step` for PageUp/PageDown granularity and IME composition tracking. Scrubbing is handled via events rather than a dedicated `Scrubber` part.
- **Recommended additions:** None.
