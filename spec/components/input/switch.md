---
component: Switch
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
  ark-ui: Switch
  radix-ui: Switch
  react-aria: Switch
---

# Switch

A Switch is a toggle control that represents an on/off state, visually similar to a physical
light switch. Unlike Checkbox, it does not support an indeterminate state.

## 1. State Machine

### 1.1 States

```rust
/// The state of the Switch component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is in an off state.
    Off,
    /// The component is in an on state.
    On,
}
```

### 1.2 Events

```rust
/// The events for the Switch component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The component received a toggle event.
    Toggle,
    /// The component received a turn on event.
    TurnOn,
    /// The component received a turn off event.
    TurnOff,
    /// The component received a focus event.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// The component received a blur event.
    Blur,
}
```

### 1.3 Context

```rust
/// The context for the Switch component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled checked value.
    pub checked: Bindable<bool>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether the component has focus-visible.
    pub focus_visible: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// The value of the component.
    pub value: String,
    /// The label of the component.
    pub label: Option<String>,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Text direction for RTL support.
    pub dir: Direction,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
use ars_i18n::Direction;

/// Props for the Switch component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// The controlled/uncontrolled checked value.
    pub checked: Option<bool>,
    /// The default checked value for uncontrolled mode.
    pub default_checked: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// The value of the component.
    pub value: String,
    /// The label of the component.
    pub label: Option<String>,
    /// The direction of the component.
    pub dir: Direction,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            checked: None, default_checked: false,
            disabled: false, required: false, invalid: false, readonly: false,
            name: None, form: None, value: "on".into(), label: None, dir: Direction::Ltr,
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
```

### 1.6 Full Machine Implementation

```rust
/// The machine for the Switch component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let checked = match props.checked {
            Some(v) => Bindable::controlled(v),
            None => Bindable::uncontrolled(props.default_checked),
        };
        let state = if *checked.get() { State::On } else { State::Off };
        let ctx = Context {
            checked,
            disabled: props.disabled,
            required: props.required,
            invalid: props.invalid,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
            name: props.name.clone(),
            value: props.value.clone(),
            label: props.label.clone(),
            has_description: false,
            dir: props.dir.clone(),
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
        if is_disabled(ctx) || is_readonly(ctx) {
            match event {
                Event::Toggle | Event::TurnOn | Event::TurnOff => return None,
                _ => {}
            }
        }

        match (state, event) {
            (State::Off, Event::Toggle) | (_, Event::TurnOn) => {
                Some(TransitionPlan::to(State::On).apply(|ctx| {
                    ctx.checked.set(true);
                }))
            }
            (State::On, Event::Toggle) | (_, Event::TurnOff) => {
                Some(TransitionPlan::to(State::Off).apply(|ctx| {
                    ctx.checked.set(false);
                }))
            }
            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
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
#[scope = "switch"]
pub enum Part {
    Root,
    Label,
    Control,
    Thumb,
    HiddenInput,
    Description,
    ErrorMessage,
}

/// The API for the Switch component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_checked(&self) -> bool {
        *self.ctx.checked.get()
    }

    fn data_state(&self) -> &'static str {
        if self.is_checked() { "checked" } else { "unchecked" }
    }

    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.data_state());
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_str());
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
        attrs
    }

    /// Attributes for the interactive control (switch track).
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("control"));
        attrs.set(HtmlAttr::Role, "switch");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if self.is_checked() { "true" } else { "false" });
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
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        if self.ctx.readonly { attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs.set(HtmlAttr::Data("ars-state"), self.data_state());
        attrs
    }

    /// Attributes for the thumb element (visual indicator).
    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.data_state());
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the hidden native input (form submission).
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "checkbox");
        attrs.set(HtmlAttr::Value, &self.ctx.value);
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        if self.is_checked() { attrs.set_bool(HtmlAttr::Checked, true); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Required, true); }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
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

    pub fn on_control_click(&self) { (self.send)(Event::Toggle); }

    pub fn on_control_keydown(&self, data: &KeyboardEventData, _shift: bool) {
        if data.key == KeyboardKey::Space || data.key == KeyboardKey::Enter {
            (self.send)(Event::Toggle);
        }
    }

    pub fn on_control_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    pub fn on_control_blur(&self) { (self.send)(Event::Blur); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Switch
├── Root               <label>   data-ars-scope="switch" data-ars-part="root"
├── Label              <span>    data-ars-part="label"
├── Control       [A]  <button>  data-ars-part="control" (role="switch")
│   └── Thumb          <span>    data-ars-part="thumb" (aria-hidden)
├── HiddenInput        <input>   data-ars-part="hidden-input" (type="checkbox", aria-hidden)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part         | Element    | Key Attributes                                             |
| ------------ | ---------- | ---------------------------------------------------------- |
| Root         | `<label>`  | `data-ars-scope="switch"`, `data-ars-state`, `dir`         |
| Label        | `<span>`   | Text label                                                 |
| Control      | `<button>` | `role="switch"`, `aria-checked`, `tabindex="0"`            |
| Thumb        | `<span>`   | `aria-hidden="true"` — sliding thumb indicator             |
| HiddenInput  | `<input>`  | `type="checkbox"`, `aria-hidden="true"`                    |
| Description  | `<div>`    | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage | `<div>`    | Validation error; linked via `aria-describedby` (optional) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Value                                    |
| ------------------ | ---------------------------------------- |
| Role               | `switch` on Control                      |
| `aria-checked`     | `"true"` / `"false"`                     |
| `aria-required`    | Present when `required=true`             |
| `aria-invalid`     | Present when `invalid=true`              |
| `aria-disabled`    | Present when `disabled=true`             |
| `aria-readonly`    | Present when `readonly=true`             |
| `aria-labelledby`  | Points to Label id                       |
| `aria-describedby` | Points to Description + ErrorMessage ids |

### 3.2 Keyboard Interaction

| Key   | Action                    |
| ----- | ------------------------- |
| Space | Toggle switch             |
| Enter | Toggle switch             |
| Tab   | Move focus to/from switch |

## 4. Internationalization

- Label and description are user-provided — localized by consumer.
- RTL: Thumb animation direction should reverse (CSS handles this via `direction`).
- On/Off semantics are universally understood; no locale-specific labels needed for the
  visual indicator itself.

## 5. Form Integration

- **Hidden input**: A hidden `<input type="checkbox">` is rendered via `HiddenInput` part. It carries `name` and `value` from context, and the `checked` attribute when state is `On`.
- **Validation states**: `aria-invalid="true"` on Control when `invalid=true`. `ErrorMessage` part linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Control points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: `aria-required="true"` on Control; `required` attribute on the hidden input.
- **Reset behavior**: On form reset, the adapter sends the appropriate event to restore `checked` to `default_checked`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`Switch`), Radix UI (`Switch`), React Aria (`Switch`).

### 6.1 Props

| Feature          | ars-ui                  | Ark UI                   | Radix UI                 | React Aria        | Notes                      |
| ---------------- | ----------------------- | ------------------------ | ------------------------ | ----------------- | -------------------------- |
| Controlled value | `checked: Option<bool>` | `checked`                | `checked`                | `isSelected`      | Full parity                |
| Default value    | `default_checked: bool` | `defaultChecked`         | `defaultChecked`         | `defaultSelected` | Full parity                |
| Disabled         | `disabled: bool`        | `disabled`               | `disabled`               | `isDisabled`      | Full parity                |
| Read-only        | `readonly: bool`        | `readOnly`               | --                       | `isReadOnly`      | Ark+RA parity; Radix lacks |
| Required         | `required: bool`        | `required`               | `required`               | --                | Ark+Radix parity           |
| Invalid          | `invalid: bool`         | `invalid`                | --                       | --                | Ark parity                 |
| Form name        | `name: Option<String>`  | `name`                   | `name`                   | `name`            | Full parity                |
| Form value       | `value: String`         | `value` (default `"on"`) | `value` (default `"on"`) | `value`           | Full parity                |
| Form ID          | `form: Option<String>`  | --                       | --                       | `form`            | RA parity                  |
| Label            | `label: Option<String>` | `label`                  | --                       | --                | Ark parity                 |
| Direction        | `dir: Direction`        | --                       | --                       | --                | ars-ui specific for RTL    |

**Gaps:** None.

### 6.2 Anatomy

| Part         | ars-ui         | Ark UI             | Radix UI   | React Aria | Notes                      |
| ------------ | -------------- | ------------------ | ---------- | ---------- | -------------------------- |
| Root         | `Root`         | `Root` (`<label>`) | `Root`     | `Switch`   | Full parity                |
| Control      | `Control`      | `Control`          | --         | --         | ars-ui interactive element |
| Thumb        | `Thumb`        | `Thumb`            | `Thumb`    | --         | Full parity with Ark/Radix |
| Label        | `Label`        | `Label`            | --         | (children) | Full parity with Ark       |
| HiddenInput  | `HiddenInput`  | `HiddenInput`      | (built-in) | (built-in) | Full parity                |
| Description  | `Description`  | --                 | --         | --         | ars-ui form-field part     |
| ErrorMessage | `ErrorMessage` | --                 | --         | --         | ars-ui form-field part     |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui   | Ark UI            | Radix UI          | React Aria | Notes       |
| ------------- | -------- | ----------------- | ----------------- | ---------- | ----------- |
| Value changed | `Toggle` | `onCheckedChange` | `onCheckedChange` | `onChange` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature                 | ars-ui | Ark UI | Radix UI | React Aria |
| ----------------------- | ------ | ------ | -------- | ---------- |
| Form integration        | Yes    | Yes    | Built-in | Built-in   |
| Controlled/uncontrolled | Yes    | Yes    | Yes      | Yes        |
| Keyboard (Space+Enter)  | Yes    | Yes    | Yes      | Yes        |
| RTL support             | Yes    | --     | --       | --         |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity across all three reference libraries.
- **Divergences:** ars-ui supports both Space and Enter for toggling (matching `role="switch"` best practice). ars-ui includes built-in Description/ErrorMessage form-field parts. ars-ui adds `dir` prop for RTL thumb animation.
- **Recommended additions:** None.
