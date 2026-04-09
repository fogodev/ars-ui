---
component: Checkbox
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [checkbox-group]
references:
  ark-ui: Checkbox
  radix-ui: Checkbox
  react-aria: Checkbox
---

# Checkbox

A `Checkbox` lets the user select or deselect a boolean value. It supports an `Indeterminate`
third state used when a parent checkbox represents a partially-selected group.

## 1. State Machine

### 1.1 States

```rust
/// The checked state of the component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is unchecked.
    Unchecked,
    /// The component is checked.
    Checked,
    /// The component is indeterminate.
    Indeterminate,
}
```

### 1.2 Events

```rust
/// Events for the Checkbox component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Flip between Unchecked/Checked; exits Indeterminate to Checked.
    Toggle,
    /// Transition to Checked.
    Check,
    /// Transition to Unchecked.
    Uncheck,
    /// Enter or exit Indeterminate.
    SetIndeterminate(bool),
    /// Focus received.
    Focus {
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost.
    Blur,
}
```

### 1.3 Context

```rust
/// Context for the Checkbox component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current checked state — controlled or uncontrolled.
    pub checked: Bindable<State>,
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
    /// True when focus came from keyboard (drives visible focus ring).
    pub focus_visible: bool,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// Value submitted with form. Defaults to "on".
    pub value: String,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// Props for the Checkbox component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Controlled checked state. When Some, component is controlled.
    pub checked: Option<State>,
    /// Default checked state for uncontrolled mode.
    pub default_checked: State,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Value attribute for form submission. Defaults to "on".
    pub value: String,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            checked: None,
            default_checked: State::Unchecked,
            disabled: false,
            required: false,
            invalid: false,
            readonly: false,
            name: None,
            form: None,
            value: "on".into(),
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool {
    ctx.disabled
}
fn is_readonly(ctx: &Context) -> bool {
    ctx.readonly
}
fn is_checked(ctx: &Context) -> bool {
    *ctx.checked.get() == State::Checked
}
fn is_indeterminate(ctx: &Context) -> bool {
    *ctx.checked.get() == State::Indeterminate
}
```

### 1.6 Full Machine Implementation

```rust
/// Machine for the Checkbox component.
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
        let state = match props.checked.as_ref().unwrap_or(&props.default_checked) {
            State::Unchecked => State::Unchecked,
            State::Checked => State::Checked,
            State::Indeterminate => State::Indeterminate,
        };
        let initial = props.checked.clone()
            .unwrap_or_else(|| props.default_checked.clone());
        let ctx = Context {
            checked: match &props.checked {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_checked.clone()),
            },
            disabled: props.disabled,
            required: props.required,
            invalid: props.invalid,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
            name: props.name.clone(),
            value: props.value.clone(),
            has_description: false,
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
        // Disabled guard: reject Toggle, Check, Uncheck
        if is_disabled(ctx) || is_readonly(ctx) {
            match event {
                Event::Toggle
                | Event::Check
                | Event::Uncheck
                | Event::SetIndeterminate(_) => return None,
                _ => {}
            }
        }

        match (state, event) {
            // ── Toggle ──────────────────────────────────────────────
            (State::Unchecked, Event::Toggle) => {
                Some(TransitionPlan::to(State::Checked).apply(|ctx| {
                    ctx.checked.set(State::Checked);
                }))
            }
            (State::Checked, Event::Toggle) => {
                Some(TransitionPlan::to(State::Unchecked).apply(|ctx| {
                    ctx.checked.set(State::Unchecked);
                }))
            }
            (State::Indeterminate, Event::Toggle) => {
                Some(TransitionPlan::to(State::Checked).apply(|ctx| {
                    ctx.checked.set(State::Checked);
                }))
            }

            // ── Check / Uncheck ─────────────────────────────────────
            (_, Event::Check) => {
                Some(TransitionPlan::to(State::Checked).apply(|ctx| {
                    ctx.checked.set(State::Checked);
                }))
            }
            (_, Event::Uncheck) => {
                Some(TransitionPlan::to(State::Unchecked).apply(|ctx| {
                    ctx.checked.set(State::Unchecked);
                }))
            }

            // ── SetIndeterminate ────────────────────────────────────
            (_, Event::SetIndeterminate(true)) => {
                Some(TransitionPlan::to(State::Indeterminate).apply(|ctx| {
                    ctx.checked.set(State::Indeterminate);
                }))
            }
            (_, Event::SetIndeterminate(false)) => {
                // Exit indeterminate → go to Checked
                if *ctx.checked.get() == State::Indeterminate {
                    Some(TransitionPlan::to(State::Checked).apply(|ctx| {
                        ctx.checked.set(State::Checked);
                    }))
                } else {
                    // Already not indeterminate — return a successful no-op
                    // (idempotent) rather than None (unhandled). This ensures
                    // callers can distinguish "event was valid but no change
                    // needed" from "event was rejected".
                    Some(TransitionPlan::context_only(|_| {}))
                }
            }

            // ── Focus / Blur ────────────────────────────────────────
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

> **Controlled value sync obligation:** For controlled components (e.g.,
> `checked: Some(Unchecked)`), the machine state transitions to the "desired"
> state optimistically (e.g., `State::Checked` on Toggle). The `ctx.checked.set()`
> call is a no-op for controlled `Bindable` — the actual value only changes when the
> adapter's `on_change` callback propagates the new value back through props. The
> adapter MUST sync machine context on every prop change via `Event::SetValue`.
> Until the adapter confirms, the machine state represents intent, not the committed
> value. Adapters that do not call back with the updated prop will see a
> state/value mismatch (e.g., `State=Checked` but `aria-checked="false"`). This
> is the expected controlled-component contract — see `01-architecture.md` §Bindable.
> **Controlled Bindable Sync — Timing Requirements:** Adapters MUST call
> `Event::SetValue()` synchronously on prop change within the same render cycle.
> The state machine SHOULD ignore user input events for controlled values until
> the latest prop is reflected. This prevents race conditions where a user
> interaction (e.g., Toggle) is processed against stale context. The sequence is:
>
> 1. Parent updates controlled prop (e.g., `checked` signal changes).
> 2. Adapter detects prop change and sends `Event::SetValue(new_value)` synchronously.
> 3. Machine updates context to reflect the new controlled value.
> 4. Only then are subsequent user input events (Toggle, Check, etc.) processed.
>
> If the adapter uses a reactive effect to sync props, the effect MUST have higher
> priority than event handlers to guarantee ordering.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "checkbox"]
pub enum Part {
    Root,
    Label,
    Control,
    Indicator,
    HiddenInput,
    Description,
    ErrorMessage,
}

/// Api for the Checkbox component.
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
        attrs.set(HtmlAttr::Data("ars-state"), match self.ctx.checked.get() {
            State::Unchecked => "unchecked",
            State::Checked => "checked",
            State::Indeterminate => "indeterminate",
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
        attrs.set(HtmlAttr::For, self.ctx.ids.part("control"));
        attrs
    }

    /// Attributes for the interactive control element.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("control"));
        attrs.set(HtmlAttr::Role, "checkbox");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), match self.ctx.checked.get() {
            State::Unchecked => "false",
            State::Checked => "true",
            State::Indeterminate => "mixed",
        });
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        if self.ctx.readonly { attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        // Conditional aria-describedby: only when a Description part is rendered
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
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Attributes for the visual indicator (check/dash icon).
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Value, &self.ctx.value);
        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        if *self.ctx.checked.get() == State::Checked {
            attrs.set_bool(HtmlAttr::Checked, true);
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
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

    /// Attributes for the error message element (shown when invalid).
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs
    }

    /// Click handler for the control element.
    pub fn on_control_click(&self) { (self.send)(Event::Toggle); }

    /// Keydown handler for the control element.
    pub fn on_control_keydown(&self, data: &KeyboardEventData, _shift: bool) {
        if data.key == KeyboardKey::Space { (self.send)(Event::Toggle); }
    }

    /// Focus handler for the control element.
    pub fn on_control_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    /// Blur handler for the control element.
    pub fn on_control_blur(&self) { (self.send)(Event::Blur); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Checkbox
├── Root               <div>    data-ars-scope="checkbox" data-ars-part="root"
├── Label              <label>  data-ars-part="label"
├── Control       [A]  <div>    data-ars-part="control" (role="checkbox")
├── Indicator          <div>    data-ars-part="indicator" (aria-hidden)
├── HiddenInput        <input>  data-ars-part="hidden-input" (type="checkbox", aria-hidden)
├── Description        <div>    data-ars-part="description" (optional)
└── ErrorMessage       <div>    data-ars-part="error-message" (optional, shown when invalid)
```

| Part         | Element   | Key Attributes                                                   |
| ------------ | --------- | ---------------------------------------------------------------- |
| Root         | `<div>`   | `data-ars-scope="checkbox"`, `data-ars-state`                    |
| Label        | `<label>` | `for` points to Control                                          |
| Control      | `<div>`   | `role="checkbox"`, `aria-checked`, `tabindex="0"`                |
| Indicator    | `<div>`   | `aria-hidden="true"` — visual check/dash icon                    |
| HiddenInput  | `<input>` | `type="checkbox"`, `aria-hidden="true"` — native form submission |
| Description  | `<div>`   | Help text; linked via `aria-describedby` (optional)              |
| ErrorMessage | `<div>`   | Validation error; linked via `aria-describedby` (optional)       |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Value                                    |
| ------------------ | ---------------------------------------- |
| Role               | `checkbox` on Control                    |
| `aria-checked`     | `"true"` / `"false"` / `"mixed"`         |
| `aria-required`    | Present when `required=true`             |
| `aria-invalid`     | Present when `invalid=true`              |
| `aria-disabled`    | Present when `disabled=true`             |
| `aria-readonly`    | Present when `readonly=true`             |
| `aria-labelledby`  | Points to Label id                       |
| `aria-describedby` | Points to Description + ErrorMessage ids |

### 3.2 Keyboard Interaction

| Key   | Action                      |
| ----- | --------------------------- |
| Space | Toggle checked state        |
| Tab   | Move focus to/from checkbox |

### 3.3 Indeterminate State Preservation in Controlled Mode

When a `Checkbox` is in controlled mode with `indeterminate: Bindable<bool>` set to `true`:

1. **Click toggles checked, not indeterminate**: A user click on an indeterminate checkbox should transition to `checked = true` (or follow the consumer's `on_change` handler). The `indeterminate` state is NOT automatically cleared by the component — it is controlled exclusively by the parent.
2. **`on_change` fires with the new checked value**: The callback receives the next `checked` boolean. The parent decides whether to also update `indeterminate` based on child selection state.
3. **Re-render timing**: If the parent updates both `checked` and `indeterminate` in the same microtask (e.g., in the `on_change` handler), the adapter must apply both updates in a single render pass to avoid a flash of intermediate state.

## 4. Internationalization

- Label text is user-provided — localization is the consumer's responsibility.
- `data-ars-state` values (`checked`, `unchecked`, `indeterminate`) are stable API tokens, not
  localized.
- Error message text should be provided through the i18n message catalog (see
  `04-internationalization.md`).
- RTL: No special handling needed — checkbox is symmetric.

## 5. Form Integration

- **Hidden input**: A hidden `<input type="checkbox">` is rendered via `HiddenInput` part. It carries `name` and `value` from context, and the `checked` attribute when state is `Checked`. The indeterminate state does not set `checked` — only `Checked` does.
- **Validation states**: `aria-invalid="true"` is set on the Control part when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Control points to `Description` (when present) and `ErrorMessage` (when invalid). See `control_attrs()` for the wiring logic.
- **Required**: `aria-required="true"` on Control; `required` attribute on the hidden input.
- **Reset behavior**: On form reset, the adapter sends the appropriate event to restore `checked` to `default_checked`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`Checkbox`), Radix UI (`Checkbox`), React Aria (`Checkbox`).

### 6.1 Props

| Feature          | ars-ui                   | Ark UI                   | Radix UI                 | React Aria        | Notes                      |
| ---------------- | ------------------------ | ------------------------ | ------------------------ | ----------------- | -------------------------- |
| Controlled value | `checked: Option<State>` | `checked: CheckedState`  | `checked`                | `isSelected`      | Full parity                |
| Default value    | `default_checked: State` | `defaultChecked`         | `defaultChecked`         | `defaultSelected` | Full parity                |
| Disabled         | `disabled: bool`         | `disabled`               | `disabled`               | `isDisabled`      | Full parity                |
| Read-only        | `readonly: bool`         | `readOnly`               | --                       | `isReadOnly`      | Ark+RA parity; Radix lacks |
| Required         | `required: bool`         | `required`               | `required`               | `isRequired`      | Full parity                |
| Invalid          | `invalid: bool`          | `invalid`                | --                       | `isInvalid`       | Ark+RA parity; Radix lacks |
| Form name        | `name: Option<String>`   | `name`                   | `name`                   | `name`            | Full parity                |
| Form value       | `value: String`          | `value` (default `"on"`) | `value` (default `"on"`) | `value`           | Full parity                |
| Form ID          | `form: Option<String>`   | `form`                   | --                       | `form`            | Ark+RA parity              |
| Indeterminate    | `State::Indeterminate`   | `CheckedState`           | `'indeterminate'`        | `isIndeterminate` | Full parity                |

**Gaps:** None.

### 6.2 Anatomy

| Part         | ars-ui         | Ark UI        | Radix UI    | React Aria          | Notes                               |
| ------------ | -------------- | ------------- | ----------- | ------------------- | ----------------------------------- |
| Root         | `Root`         | `Root`        | `Root`      | (component)         | Full parity                         |
| Control      | `Control`      | `Control`     | --          | --                  | ars-ui specific interactive element |
| Indicator    | `Indicator`    | `Indicator`   | `Indicator` | --                  | Full parity with Ark/Radix          |
| Label        | `Label`        | `Label`       | --          | (children)          | Full parity with Ark                |
| HiddenInput  | `HiddenInput`  | `HiddenInput` | (built-in)  | (built-in)          | Full parity                         |
| Description  | `Description`  | --            | --          | `Text[description]` | ars-ui form-field part              |
| ErrorMessage | `ErrorMessage` | --            | --          | `FieldError`        | ars-ui form-field part              |

**Gaps:** None. ars-ui includes built-in form-field parts (Description, ErrorMessage) that Ark/Radix handle externally via Field wrapper components.

### 6.3 Events

| Callback      | ars-ui   | Ark UI            | Radix UI          | React Aria         | Notes       |
| ------------- | -------- | ----------------- | ----------------- | ------------------ | ----------- |
| Value changed | `Toggle` | `onCheckedChange` | `onCheckedChange` | `onChange`         | Full parity |
| Focus         | `Focus`  | --                | --                | `onFocus`/`onBlur` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature                 | ars-ui | Ark UI | Radix UI | React Aria        |
| ----------------------- | ------ | ------ | -------- | ----------------- |
| Indeterminate state     | Yes    | Yes    | Yes      | Yes               |
| Form integration        | Yes    | Yes    | Built-in | Built-in          |
| Controlled/uncontrolled | Yes    | Yes    | Yes      | Yes               |
| Disabled                | Yes    | Yes    | Yes      | Yes               |
| Read-only               | Yes    | Yes    | --       | Yes               |
| Validation              | Yes    | Yes    | --       | Yes (native+aria) |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity across all three reference libraries.
- **Divergences:** ars-ui uses a tri-state `State` enum instead of separate `checked`+`indeterminate` booleans. Description and ErrorMessage parts are built into the component rather than requiring external Field wrappers. ars-ui adds `readonly` which Radix lacks.
- **Recommended additions:** None.
