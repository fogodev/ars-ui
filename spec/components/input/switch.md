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
    /// The component received a native form reset event.
    Reset,
    /// Synchronize the externally controlled checked prop.
    SetValue(Option<bool>),
    /// Synchronize output-affecting props stored in context.
    SetProps,
    /// Track whether a Description part is rendered.
    SetHasDescription(bool),
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
    /// The ID of the form element the hidden input is associated with.
    pub form: Option<String>,
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
use ars_core::{Callback, Direction};

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
    /// Called after user intent requests a new checked value.
    pub on_checked_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            checked: None, default_checked: false,
            disabled: false, required: false, invalid: false, readonly: false,
            name: None, form: None, value: "on".into(), label: None, dir: Direction::Ltr,
            on_checked_change: None,
        }
    }
}
```

#### 1.4.1 Builder Surface

`Props::new()` returns `Props::default()`. Builder methods must cover every public field:

| Field               | Setter                        | Clearer / inverse     |
| ------------------- | ----------------------------- | --------------------- |
| `id`                | `id(impl Into<String>)`       | —                     |
| `checked`           | `checked(bool)`               | `uncontrolled()`      |
| `default_checked`   | `default_checked(bool)`       | —                     |
| `disabled`          | `disabled(bool)`              | `disabled(false)`     |
| `required`          | `required(bool)`              | `required(false)`     |
| `invalid`           | `invalid(bool)`               | `invalid(false)`      |
| `readonly`          | `readonly(bool)`              | `readonly(false)`     |
| `name`              | `name(impl Into<String>)`     | `no_name()`           |
| `form`              | `form(impl Into<String>)`     | `no_form()`           |
| `value`             | `value(impl Into<String>)`    | —                     |
| `label`             | `label(impl Into<String>)`    | `no_label()`          |
| `dir`               | `dir(Direction)`              | —                     |
| `on_checked_change` | `on_checked_change(callback)` | `no_checked_change()` |

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
            form: props.form.clone(),
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
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) || is_readonly(ctx) {
            match event {
                Event::Toggle | Event::TurnOn | Event::TurnOff => return None,
                _ => {}
            }
        }

        match (state, event) {
            (_, Event::Reset) => Some(reset_plan(ctx, props.default_checked)),
            (_, Event::SetValue(value)) => {
                match value {
                    Some(value) => {
                        let value = *value;
                        let is_controlled = props.checked.is_some();
                        Some(TransitionPlan::to(if value { State::On } else { State::Off }).apply(
                            move |ctx| {
                                ctx.checked.set(value);
                                ctx.checked.sync_controlled(if is_controlled { Some(value) } else { None });
                            },
                        ))
                    }
                    None => Some(TransitionPlan::context_only(|ctx| {
                        ctx.checked.sync_controlled(None);
                    })),
                }
            }
            (_, Event::SetProps) => {
                let disabled = props.disabled;
                let required = props.required;
                let invalid = props.invalid;
                let readonly = props.readonly;
                let name = props.name.clone();
                let form = props.form.clone();
                let value = props.value.clone();
                let label = props.label.clone();
                let dir = props.dir.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                    ctx.required = required;
                    ctx.invalid = invalid;
                    ctx.readonly = readonly;
                    ctx.name = name;
                    ctx.form = form;
                    ctx.value = value;
                    ctx.label = label;
                    ctx.dir = dir;
                }))
            }
            (_, Event::SetHasDescription(has_description)) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.has_description = has_description;
                }))
            }
            (State::Off, Event::Toggle) | (_, Event::TurnOn) => {
                Some(value_change_plan(ctx, true))
            }
            (State::On, Event::Toggle) | (_, Event::TurnOff) => {
                Some(value_change_plan(ctx, false))
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(old.id, new.id, "switch::Props.id must remain stable after init");

        let mut events = Vec::new();
        if old.checked != new.checked {
            events.push(Event::SetValue(new.checked));
        }
        if old.disabled != new.disabled
            || old.required != new.required
            || old.invalid != new.invalid
            || old.readonly != new.readonly
            || old.name != new.name
            || old.form != new.form
            || old.value != new.value
            || old.label != new.label
            || old.dir != new.dir
        {
            events.push(Event::SetProps);
        }
        events
    }
}
```

User value-change intent is centralized in `value_change_plan`:

```rust
fn reset_plan(ctx: &Context, default_checked: bool) -> TransitionPlan<Machine> {
    if *ctx.checked.get() == default_checked {
        return TransitionPlan::new();
    }

    if ctx.checked.is_controlled() {
        return value_change_plan(ctx, default_checked);
    }

    TransitionPlan::to(if default_checked { State::On } else { State::Off })
        .apply(move |ctx| {
            ctx.checked.set(default_checked);
            ctx.checked.sync_controlled(None);
        })
}

fn value_change_plan(ctx: &Context, next: bool) -> TransitionPlan<Machine> {
    if *ctx.checked.get() == next {
        return TransitionPlan::context_only(|_| {});
    }

    if ctx.checked.is_controlled() {
        return TransitionPlan::new()
            .apply(|_| {})
            .with_effect(checked_change_effect(next));
    }

    TransitionPlan::to(if next { State::On } else { State::Off })
        .apply(move |ctx| ctx.checked.set(next))
        .with_effect(checked_change_effect(next))
}

fn checked_change_effect(next: bool) -> PendingEffect<Machine> {
    PendingEffect::new("checked-change", move |_ctx, props, _send| {
        if let Some(callback) = &props.on_checked_change {
            callback(next);
        }
        no_cleanup()
    })
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
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
        if !self.ctx.readonly {
            attrs.set(HtmlAttr::For, self.ctx.ids.part("hidden-input"));
        }
        attrs
    }

    /// Attributes for the interactive control (switch track).
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("control"));
        attrs.set(HtmlAttr::Type, "button");
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("hidden-input"));
        attrs.set(HtmlAttr::Type, "checkbox");
        attrs.set(HtmlAttr::Value, &self.ctx.value);
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.ctx.form {
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

    pub fn on_control_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    pub fn on_control_blur(&self) { (self.send)(Event::Blur); }

    pub fn on_hidden_input_change(&self, checked: bool) {
        (self.send)(if checked { Event::TurnOn } else { Event::TurnOff });
    }

    pub fn on_form_reset(&self) { (self.send)(Event::Reset); }
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
├── Root               <div>     data-ars-scope="switch" data-ars-part="root"
├── Label              <label>   data-ars-part="label" for="<id>-hidden-input"
├── Control       [A]  <button>  data-ars-part="control" (role="switch")
│   └── Thumb          <span>    data-ars-part="thumb" (aria-hidden)
├── HiddenInput        <input>   data-ars-part="hidden-input" (type="checkbox", aria-hidden)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part         | Element    | Key Attributes                                             |
| ------------ | ---------- | ---------------------------------------------------------- |
| Root         | `<div>`    | `id`, `data-ars-scope="switch"`, `data-ars-state`, `dir`   |
| Label        | `<label>`  | Text label; `for` targets `HiddenInput` unless readonly    |
| Control      | `<button>` | `type="button"`, `role="switch"`, `aria-checked`, `tabindex="0"` |
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

| Key   | Action                                                                  |
| ----- | ----------------------------------------------------------------------- |
| Space | Native `<button>` activation dispatches click, which toggles the switch |
| Enter | Native `<button>` activation dispatches click, which toggles the switch |
| Tab   | Move focus to/from switch                                               |

`Control` is a native `<button>`, so the agnostic API does not expose a keydown
toggle helper. Adapters must rely on the native synthesized click event for
Space and Enter keyboard activation so a single activation cannot send both a
keydown toggle and a click toggle.

### 3.3 Controlled Value Synchronization

When `Props.checked` is `Some`, user activation requests the next boolean value through
`on_checked_change`; the component does not render that value until the parent confirms it
through `Event::SetValue(Some(value))`.

1. **Click or keyboard requests the next value**: `Toggle`, `TurnOn`, and `TurnOff` emit
   `on_checked_change(next)` when the requested value differs from the rendered value.
2. **The callback receives the next boolean value**: The parent decides whether to accept
   the requested value, keep the current value, or choose another value.
3. **Re-render timing**: The adapter applies the confirmed `checked` prop through
   `Event::SetValue(Some(value))` in the same render cycle.
4. **Uncontrolled prop synchronization**: `Event::SetValue(Some(value))` updates the internal
   value while leaving the bindable uncontrolled when `Props.checked` is `None`.
5. **Form reset**: `Event::Reset` is not blocked by `disabled` or `readonly`. In uncontrolled
   mode it restores `default_checked` without calling `on_checked_change`; in controlled mode it
   requests `default_checked` through `on_checked_change(default_checked)` and waits for parent
   confirmation through `Event::SetValue(Some(value))`.

## 4. Internationalization

- Label and description are user-provided — localized by consumer.
- RTL: Thumb animation direction should reverse (CSS handles this via `direction`).
- On/Off semantics are universally understood; no locale-specific labels needed for the
  visual indicator itself.

## 5. Form Integration

- **Hidden input**: A hidden `<input type="checkbox">` is rendered via `HiddenInput` part. It carries `id`, `name`, `form`, and `value` from context, and the `checked` attribute when state is `On`. The native input is disabled only when the component is disabled; readonly values remain enabled so they can be submitted with native forms.
- **Label activation**: `Label` points `for` at `HiddenInput` so native label activation targets a labelable form control. When readonly, `Label` omits `for` because checkboxes have no native readonly behavior and label activation would otherwise mutate the hidden input. Adapters must wire hidden input changes to `Api::on_hidden_input_change(checked)`.
- **Validation states**: `aria-invalid="true"` on Control when `invalid=true`. `ErrorMessage` part linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Control points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: `aria-required="true"` on Control; `required` attribute on the hidden input.
- **Reset behavior**: On form reset, the adapter sends `Event::Reset`. Reset is not blocked by `disabled` or `readonly`. Uncontrolled switches restore `checked` to `default_checked` without emitting `on_checked_change`; controlled switches request `default_checked` through `on_checked_change(default_checked)` and wait for the parent to confirm with `checked: Some(default_checked)`.
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
| Root         | `Root`         | `Root` (`<label>`) | `Root`     | `Switch`   | Comparable root container  |
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

- **Overall:** Full parity for controlled/uncontrolled value, form submission, disabled,
  readonly, required, invalid, keyboard, and RTL behavior.
- **Divergences:** ars-ui uses a neutral `Root` container plus a dedicated `Label` element
  targeting the hidden input for native label activation. ars-ui supports both Space and Enter
  through native `<button>` keyboard activation, which dispatches the same click path as pointer
  activation and avoids double toggles. ars-ui includes built-in Description/ErrorMessage form-field
  parts. ars-ui adds `dir` prop for RTL thumb animation.
- **Recommended additions:** None.
