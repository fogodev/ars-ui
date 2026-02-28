---
component: PasswordInput
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [text-field]
references:
  ark-ui: PasswordInput
  radix-ui: PasswordToggleField
---

# PasswordInput

A password input field that extends the standard TextField with a visibility toggle button.
By default the input displays masked text (`type="password"`); clicking the toggle reveals
the plain text (`type="text"`). This component wraps a single `<input>` and adds the
toggle affordance without duplicating TextField's form-participation logic.

## 1. State Machine

### 1.1 States

```rust
/// The state of the PasswordInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Password is hidden (input type="password").
    Masked,
    /// Password is visible (input type="text").
    Visible,
}
```

### 1.2 Events

```rust
/// The events for the PasswordInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Flip between Masked and Visible.
    ToggleVisibility,
    /// Explicitly set visibility.
    SetVisibility(bool),
    /// Focus received; flag indicates keyboard-initiated focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost.
    Blur,
}
```

### 1.3 Context

```rust
/// The context for the PasswordInput component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the password is currently visible.
    pub visible: bool,
    /// The input value — controlled or uncontrolled.
    pub value: Bindable<String>,
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
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the password input.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the PasswordInput component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the component.
    pub id: String,
    /// Controlled value. When Some, component is controlled.
    pub value: Option<String>,
    /// Default value for uncontrolled mode.
    pub default_value: String,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the password is visible on initial render.
    pub default_visible: bool,
    /// The placeholder of the component.
    pub placeholder: Option<String>,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Autocomplete hint for password managers. Defaults to "current-password".
    pub autocomplete: Option<String>,
    /// Locale for i18n message resolution.
    pub locale: Option<Locale>,
    /// Translatable messages.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: String::new(),
            disabled: false,
            required: false,
            invalid: false,
            readonly: false,
            default_visible: false,
            placeholder: None,
            name: None,
            form: None,
            autocomplete: None,
            locale: None,
            messages: None,
        }
    }
}

/// The messages for the PasswordInput component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the toggle when password is masked. Default: `"Show password"`.
    pub show_password_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the toggle when password is visible. Default: `"Hide password"`.
    pub hide_password_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            show_password_label: MessageFn::static_str("Show password"),
            hide_password_label: MessageFn::static_str("Hide password"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the PasswordInput component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props) -> (Self::State, Self::Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let state = if props.default_visible { State::Visible } else { State::Masked };
        let ctx = Context {
            visible: props.default_visible,
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            disabled: props.disabled,
            required: props.required,
            invalid: props.invalid,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
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
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::ToggleVisibility => {
                let next_visible = !ctx.visible;
                Some(TransitionPlan::to(if next_visible {
                    State::Visible
                } else {
                    State::Masked
                }).apply(move |ctx| {
                    ctx.visible = next_visible;
                }))
            }

            Event::SetVisibility(visible) => {
                if ctx.visible == *visible { return None; }
                let vis = *visible;
                Some(TransitionPlan::to(if vis {
                    State::Visible
                } else {
                    State::Masked
                }).apply(move |ctx| {
                    ctx.visible = vis;
                }))
            }

            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
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
#[scope = "password-input"]
pub enum Part {
    Root,
    Label,
    Input,
    Toggle,
    Description,
    ErrorMessage,
}

/// The API for the PasswordInput component.
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
            State::Masked => "masked",
            State::Visible => "visible",
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

    /// Attributes for the password input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Type, if self.ctx.visible { "text" } else { "password" });
        attrs.set(HtmlAttr::Value, self.ctx.value.get());
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
        if let Some(ref placeholder) = self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder);
        }
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::AutoComplete, self.props.autocomplete.as_deref().unwrap_or("current-password"));
        attrs
    }

    /// Attributes for the visibility toggle button.
    pub fn toggle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Toggle.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), if self.ctx.visible {
            (self.ctx.messages.hide_password_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.show_password_label)(&self.ctx.locale)
        });
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
    pub fn on_toggle_click(&self) { (self.send)(Event::ToggleVisibility); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::Toggle => self.toggle_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
PasswordInput
├── Root               <div>     data-ars-scope="password-input" data-ars-part="root"
├── Label              <label>   data-ars-part="label"
├── Input         [A]  <input>   data-ars-part="input" (type="password"|"text")
├── Toggle             <button>  data-ars-part="toggle" (aria-label)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part         | Element    | Key Attributes                                             |
| ------------ | ---------- | ---------------------------------------------------------- |
| Root         | `<div>`    | `data-ars-scope="password-input"`, `data-ars-state`        |
| Label        | `<label>`  | `for` points to Input                                      |
| Input        | `<input>`  | `type="password"` or `"text"`, `aria-labelledby`           |
| Toggle       | `<button>` | `aria-label` (dynamic: show/hide), `type="button"`         |
| Description  | `<div>`    | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage | `<div>`    | Validation error; linked via `aria-describedby` (optional) |

- **Input** renders as `type="password"` when masked, `type="text"` when visible.
- **Toggle** is a button that switches visibility. It displays an eye icon (open/closed)
  managed by the consumer or a default icon slot.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element | Value                                                  |
| ------------------ | ------- | ------------------------------------------------------ |
| `type`             | Input   | `"password"` (Masked) or `"text"` (Visible)            |
| `aria-label`       | Toggle  | Dynamic: `show_password_label` / `hide_password_label` |
| `aria-required`    | Input   | Present when `required=true`                           |
| `aria-invalid`     | Input   | Present when `invalid=true`                            |
| `aria-labelledby`  | Input   | Points to Label id                                     |
| `aria-describedby` | Input   | Points to Description + ErrorMessage ids               |

- Dynamic toggle label communicates state; `aria-pressed` would be redundant and contradictory.

### 3.2 Keyboard Interaction

| Key     | Context        | Action            |
| ------- | -------------- | ----------------- |
| `Enter` | Toggle focused | Toggle visibility |
| `Space` | Toggle focused | Toggle visibility |

> The toggle button is in the tab order (`tabindex="0"`), so users can
> Tab from the password input to the toggle button and activate it with Enter or Space.

### 3.3 Screen Reader Behavior

- The toggle button announces its current label (from `messages.show_password_label` or
  `messages.hide_password_label`) so users know whether the password is revealed.
- Toggling visibility does not move focus — the input retains focus and screen readers
  announce the type change.

### 3.4 IME Composition Handling

See [IME Composition Protocol](./_category.md#ime-composition-protocol) for input method editor behavior
during text composition. PasswordInput wraps a TextField and inherits its IME composition
handling: during `compositionstart`...`compositionend`, custom keyboard handlers are suppressed
and the final composed value is applied as a single update on `compositionend`.

## 4. Internationalization

### 4.1 Messages

Toggle button labels are provided through the `Messages` struct:

- `show_password_label` — displayed when password is masked (default: "Show password")
- `hide_password_label` — displayed when password is visible (default: "Hide password")

See §1.4 for the `Messages` struct definition and `Default` implementation.

- `placeholder` is localized by the consumer.
- RTL: Input text direction follows the document locale. The toggle button position is
  handled via CSS logical properties (`inset-inline-end`).

## 5. Form Integration

- **Hidden input**: The `Input` part is the native `<input>` element and participates directly in form submission via its `name` attribute. No separate hidden input is needed.
- **Validation states**: `aria-invalid="true"` on the Input when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Input points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: `aria-required="true"` on Input.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value` and `visible` to `default_visible`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`PasswordInput`), Radix UI (`PasswordToggleField` preview).

### 6.1 Props

| Feature                  | ars-ui                         | Ark UI                   | Radix UI                  | Notes                     |
| ------------------------ | ------------------------------ | ------------------------ | ------------------------- | ------------------------- |
| Controlled value         | `value: Option<String>`        | --                       | --                        | ars-ui adds value control |
| Default value            | `default_value: String`        | --                       | --                        | ars-ui adds value control |
| Disabled                 | `disabled: bool`               | `disabled`               | --                        | Ark parity                |
| Required                 | `required: bool`               | `required`               | --                        | Ark parity                |
| Invalid                  | `invalid: bool`                | `invalid`                | --                        | Ark parity                |
| Read-only                | `readonly: bool`               | `readOnly`               | --                        | Ark parity                |
| Default visible          | `default_visible: bool`        | `defaultVisible`         | `defaultVisible`          | Full parity               |
| Controlled visible       | --                             | `visible`                | `visible`                 | See note below            |
| Autocomplete             | `autocomplete: Option<String>` | `autoComplete`           | `autoComplete` (on Input) | Full parity               |
| Form name                | `name: Option<String>`         | `name`                   | `name`                    | Full parity               |
| Form ID                  | `form: Option<String>`         | --                       | --                        | ars-ui enhancement        |
| Placeholder              | `placeholder: Option<String>`  | --                       | --                        | ars-ui enhancement        |
| Ignore password managers | --                             | `ignorePasswordManagers` | --                        | Ark-only feature          |

**Gaps:**

- **Controlled visibility**: Ark UI supports `visible: boolean` for controlled visibility state. ars-ui currently uses `default_visible` + `ToggleVisibility`/`SetVisibility` events, which achieves the same effect through the adapter's `on_visibility_change` callback. Not a functional gap since the adapter can wire controlled visibility through event callbacks.

### 6.2 Anatomy

| Part         | ars-ui         | Ark UI              | Radix UI      | Notes                  |
| ------------ | -------------- | ------------------- | ------------- | ---------------------- |
| Root         | `Root`         | `Root`              | `Root`        | Full parity            |
| Label        | `Label`        | `Label`             | --            | Ark parity             |
| Input        | `Input`        | `Input`             | `Input`       | Full parity            |
| Toggle       | `Toggle`       | `VisibilityTrigger` | `Toggle`      | Full parity            |
| Indicator    | --             | `Indicator`         | `Slot`/`Icon` | See note below         |
| Control      | --             | `Control`           | --            | Ark layout wrapper     |
| Description  | `Description`  | --                  | --            | ars-ui form-field part |
| ErrorMessage | `ErrorMessage` | --                  | --            | ars-ui form-field part |

**Gaps:** None material. Ark's `Indicator` shows different content for visible/hidden states. In ars-ui, the Toggle button's children render conditionally based on `data-ars-state` (the same pattern). Radix's `Slot`/`Icon` serve the same purpose.

### 6.3 Events

| Callback           | ars-ui             | Ark UI               | Radix UI            | Notes       |
| ------------------ | ------------------ | -------------------- | ------------------- | ----------- |
| Visibility changed | `ToggleVisibility` | `onVisibilityChange` | `onVisiblityChange` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature                  | ars-ui | Ark UI | Radix UI |
| ------------------------ | ------ | ------ | -------- |
| Show/hide toggle         | Yes    | Yes    | Yes      |
| Autocomplete hint        | Yes    | Yes    | Yes      |
| Form integration         | Yes    | Yes    | --       |
| Ignore password managers | --     | Yes    | --       |

**Gaps:** None material. Ark's `ignorePasswordManagers` is a niche feature that blocks browser password manager autofill; ars-ui consumers can set `autocomplete: "off"` for a similar effect.

### 6.5 Summary

- **Overall:** Full parity with both reference libraries.
- **Divergences:** ars-ui adds value control (`value`/`default_value`) and form-field parts (Description, ErrorMessage) not present in either reference. Ark's `Indicator` pattern is achieved via conditional rendering on the Toggle button in ars-ui.
- **Recommended additions:** None.
