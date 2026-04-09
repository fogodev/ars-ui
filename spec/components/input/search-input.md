---
component: SearchInput
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [text-field]
references:
    react-aria: SearchField
---

# SearchInput

A search-specific text input with submit trigger, clear trigger, and loading state.

## 1. State Machine

### 1.1 States

```rust
/// The state of the SearchInput component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused,
    /// The component is performing a search.
    Searching,
}
```

### 1.2 Events

```rust
/// The events for the SearchInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus { is_keyboard: bool },
    /// The component lost focus.
    Blur,
    /// The input value changed.
    Change(String),
    /// The input was cleared (Escape key or clear button).
    Clear,
    /// The search was submitted (Enter key or submit button).
    Submit,
    /// Set the loading/searching state.
    SetSearching(bool),
    /// Fired by the debounce timer effect when the debounce period expires.
    DebounceExpired,
    /// Cancels any active debounce timer without firing the callback.
    CancelDebounce,
    /// IME composition started.
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
}
```

### 1.3 Context

```rust
/// The context for the SearchInput component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The input value — controlled or uncontrolled.
    pub value: Bindable<String>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the component is loading.
    pub loading: bool,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The placeholder text.
    pub placeholder: Option<String>,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the search input.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the SearchInput component.
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
    /// Whether the search input is required.
    pub required: bool,
    /// The placeholder text.
    pub placeholder: Option<String>,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Optional debounce duration in milliseconds for search-as-you-type.
    /// When set, `Event::Change` values are debounced — the machine waits
    /// this many ms after the last keystroke before propagating the change
    /// to the `on_change` callback. Set to `None` (default) for immediate.
    pub debounce_ms: Option<u32>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: String::new(),
            disabled: false, readonly: false, invalid: false, required: false,
            placeholder: None, name: None, form: None,
            debounce_ms: None,
        }
    }
}

/// Messages for the SearchInput component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the clear button. Default: `"Clear search"`.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the submit button. Default: `"Submit search"`.
    pub submit_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            clear_label: MessageFn::static_str("Clear search"),
            submit_label: MessageFn::static_str("Submit search"),
        }
    }
}

impl ComponentMessages for Messages {}
```

> **Callback props (`on_submit`, `on_clear`)**: These are provided by the adapter layer, not the core state machine. The adapter MUST wire: `on_submit` fires when `Event::Submit` is processed (Enter key or SubmitTrigger click), receiving the current `ctx.value`; `on_clear` fires when `Event::Clear` is processed (ClearTrigger click or Escape key).

### 1.5 Debounce Timer Integration

The debounce timer is a `PendingEffect` managed by the state machine, not a simple adapter-side `setTimeout`. This ensures the timer lifecycle is tied to the machine's effect system and properly cleaned up on unmount or state changes.

- **On `Event::Change`**: The machine cancels any existing debounce timer effect and starts a new one with the configured `debounce_ms` duration.
- **On timer expiration**: The effect fires `Event::DebounceExpired`, which triggers the `on_change` callback with the current `ctx.value`.
- **Rapid inputs**: Each new `Event::Change` resets the timer, ensuring only the final value after the user stops typing triggers the callback.
- **`Event::Submit`**: Cancels any pending debounce timer and fires the callback immediately (explicit submit should not be delayed).
- **`Event::Clear`**: Cancels any pending debounce timer and fires the callback immediately with an empty string.

**Props change (`debounce_ms` mutation)**: When the `debounce_ms` prop changes while a debounce timer is active, the machine cancels the active timer and starts a fresh one with the new duration. A `debounce_ms` value of `Some(0)` is treated as `Some(1)` to prevent microtask-level race conditions.

### 1.6 Full Machine Implementation

```rust
/// The machine for the SearchInput component.
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
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            focused: false,
            focus_visible: false,
            loading: false,
            name: props.name.clone(),
            placeholder: props.placeholder.clone(),
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
                let is_loading = ctx.loading;
                Some(TransitionPlan::to(if is_loading { State::Searching } else { State::Idle })
                    .apply(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }))
            }
            Event::Change(val) => {
                if ctx.disabled || ctx.readonly { return None; }
                let val = val.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(val);
                }))
            }
            Event::Clear => {
                if ctx.disabled || ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.value.set(String::new());
                }))
            }
            Event::Submit => {
                if ctx.disabled { return None; }
                Some(TransitionPlan::to(State::Searching).apply(|ctx| {
                    ctx.loading = true;
                }))
            }
            Event::SetSearching(loading) => {
                let loading = *loading;
                let is_focused = ctx.focused;
                if !loading {
                    Some(TransitionPlan::to(if is_focused { State::Focused } else { State::Idle })
                        .apply(move |ctx| { ctx.loading = loading; }))
                } else {
                    Some(TransitionPlan::to(State::Searching)
                        .apply(move |ctx| { ctx.loading = loading; }))
                }
            }
            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = true; }))
            }
            Event::CompositionEnd => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = false; }))
            }
            Event::DebounceExpired | Event::CancelDebounce => {
                // Handled by the effect system; no state/context change
                None
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

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "search-input"]
pub enum Part {
    Root,
    Label,
    Input,
    ClearTrigger,
    SubmitTrigger,
    LoadingIndicator,
    Description,
    ErrorMessage,
}

/// The API for the SearchInput component.
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
        attrs.set(HtmlAttr::Role, "search");
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
            State::Searching => "searching",
        });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        if self.ctx.loading { attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true"); }
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

    /// Attributes for the search input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Type, "search");
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
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.props.required { attrs.set_bool(HtmlAttr::Required, true); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if let Some(ref p) = self.ctx.placeholder { attrs.set(HtmlAttr::Placeholder, p); }
        if let Some(ref name) = self.ctx.name { attrs.set(HtmlAttr::Name, name); }
        if let Some(ref form) = self.props.form { attrs.set(HtmlAttr::Form, form); }
        attrs
    }

    /// Attributes for the clear trigger button.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        let is_empty = self.ctx.value.get().is_empty();
        if is_empty { attrs.set_style(CssProperty::Display, "none"); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
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
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// Attributes for the loading indicator.
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the description/help text.
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
    pub fn on_input_change(&self, val: String) { (self.send)(Event::Change(val)); }
    pub fn on_clear_click(&self) { (self.send)(Event::Clear); }
    pub fn on_submit_click(&self) { (self.send)(Event::Submit); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::SubmitTrigger => self.submit_trigger_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
SearchInput
├── Root               <div>     data-ars-scope="search-input" data-ars-part="root" (role="search")
├── Label              <label>   data-ars-part="label"
├── Input         [A]  <input>   data-ars-part="input" (type="search")
├── ClearTrigger       <button>  data-ars-part="clear-trigger" (hidden when empty)
├── SubmitTrigger      <button>  data-ars-part="submit-trigger" (optional)
├── LoadingIndicator   <div>     data-ars-part="loading-indicator" (shown when searching)
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part             | Element    | Key Attributes                                              |
| ---------------- | ---------- | ----------------------------------------------------------- |
| Root             | `<div>`    | `role="search"`, `data-ars-state`, `aria-busy` when loading |
| Label            | `<label>`  | `for` points to Input                                       |
| Input            | `<input>`  | `type="search"`, `aria-labelledby`, `aria-describedby`      |
| ClearTrigger     | `<button>` | `aria-label`, hidden via `display:none` when value is empty |
| SubmitTrigger    | `<button>` | `aria-label` (optional)                                     |
| LoadingIndicator | `<div>`    | `aria-hidden="true"` — shown during Searching state         |
| Description      | `<div>`    | Help text; linked via `aria-describedby` (optional)         |
| ErrorMessage     | `<div>`    | Validation error; linked via `aria-describedby` (optional)  |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element       | Value                                      |
| ------------------ | ------------- | ------------------------------------------ |
| `role`             | Root          | `search` (landmark)                        |
| `type`             | Input         | `"search"`                                 |
| `aria-busy`        | Root          | Present when `loading=true`                |
| `aria-invalid`     | Input         | Present when `invalid=true`                |
| `required`         | Input         | Present when `required=true` (native attr) |
| `aria-labelledby`  | Input         | Points to Label id                         |
| `aria-describedby` | Input         | Points to Description + ErrorMessage       |
| `aria-label`       | ClearTrigger  | From `messages.clear_label`                |
| `aria-label`       | SubmitTrigger | From `messages.submit_label`               |

### 3.2 Keyboard Interaction

| Key    | Action                              |
| ------ | ----------------------------------- |
| Enter  | Submit search query                 |
| Escape | Clear input                         |
| Tab    | Navigate between input and triggers |

### 3.3 Keyboard Event Propagation in Nested Components

When `SearchInput` is nested inside a parent component that also handles keyboard events (e.g., `Combobox`), event propagation must be carefully managed:

**Rule**: `SearchInput` calls `stopPropagation()` **only** for keys it actually handles (Enter, Escape). It MUST NOT call `stopPropagation()` for keys it does not handle (ArrowUp, ArrowDown, Home, End, etc.), allowing the parent component to process them.

**IME composition during nesting**: SearchInput emits `on_change` during active IME composition with the `is_composing: true` flag set in context. Parent components (e.g., Combobox) MUST check `search_input_ctx.is_composing` before filtering — when `true`, skip filtering to avoid showing results for incomplete CJK input.

### 3.4 Browser SearchBox Integration

When used with dropdown-style results (e.g., paired with a Listbox for suggestions), the following ARIA attributes are mandatory on the Input part:

- `aria-controls`: Points to the results container element ID.
- `aria-expanded`: `"true"` when the results dropdown is visible, `"false"` otherwise.
- `autocomplete="off"`: Prevents browser autofill from interfering with custom results.
- `role="combobox"`: When paired with a results dropdown, overrides the implicit role.

When SearchInput is used standalone (no dropdown), these attributes are not required.

### 3.5 Autocomplete Results Announcement

When paired with a results dropdown, the adapter MUST announce result counts to screen reader users via a live region, debounced by at least 300ms:

- `"{N} results available"` when results arrive.
- `"No results available"` when the query returns no matches.
- The live region uses `aria-live="polite"` and `aria-atomic="true"`.

## 4. Internationalization

- `placeholder` localized by consumer ("Search...", "Rechercher...", etc.).
- `aria-label` for `ClearTrigger` and `SubmitTrigger` localized via `Messages`.
- RTL: Input text direction follows locale; icon positions may swap (CSS handles this).

### 4.1 IME Composition Handling

`SearchInput` tracks `is_composing: bool` in Context. During composition:

- `Event::Change` still updates `ctx.value` for display, but the `on_change` callback and debounce timer are NOT triggered.
- After `compositionend`, the adapter fires a final `Event::Change` with the committed text, which triggers normal filter/debounce processing.

See [IME Composition Protocol](./_category.md#ime-composition-protocol).

### 4.2 Clear Event and Debounce Interaction

When the user triggers the clear action:

1. Any pending debounce timer is immediately cancelled.
2. `on_change("")` is emitted synchronously — bypasses debounce delay entirely.
3. A `debounce_ms` value of `Some(0)` is treated as `Some(1)` to prevent microtask-level race conditions.

## 5. Form Integration

- **Hidden input**: The `Input` part is the native `<input type="search">` and participates directly in form submission via its `name` attribute.
- **Validation states**: `aria-invalid="true"` on the Input when `invalid=true`. Native `required` attribute on Input when `required=true`. The `ErrorMessage` part is linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Input points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value` and transitions to Idle.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: React Aria (`SearchField`).
>
> Note: Ark UI and Radix UI have no dedicated search input component.

### 6.1 Props

| Feature          | ars-ui                        | React Aria     | Notes              |
| ---------------- | ----------------------------- | -------------- | ------------------ |
| Controlled value | `value: Option<String>`       | `value`        | Full parity        |
| Default value    | `default_value: String`       | `defaultValue` | Full parity        |
| Disabled         | `disabled: bool`              | `isDisabled`   | Full parity        |
| Read-only        | `readonly: bool`              | `isReadOnly`   | Full parity        |
| Required         | `required: bool`              | `isRequired`   | Full parity        |
| Invalid          | `invalid: bool`               | `isInvalid`    | Full parity        |
| Placeholder      | `placeholder: Option<String>` | `placeholder`  | Full parity        |
| Form name        | `name: Option<String>`        | `name`         | Full parity        |
| Form ID          | `form: Option<String>`        | `form`         | Full parity        |
| Debounce         | `debounce_ms: Option<u32>`    | --             | ars-ui enhancement |

**Gaps:** None.

### 6.2 Anatomy

| Part             | ars-ui             | React Aria          | Notes              |
| ---------------- | ------------------ | ------------------- | ------------------ |
| Root             | `Root`             | `SearchField`       | Full parity        |
| Label            | `Label`            | `Label`             | Full parity        |
| Input            | `Input`            | `Input`             | Full parity        |
| ClearTrigger     | `ClearTrigger`     | `Button` (clear)    | Full parity        |
| SubmitTrigger    | `SubmitTrigger`    | --                  | ars-ui enhancement |
| LoadingIndicator | `LoadingIndicator` | --                  | ars-ui enhancement |
| Description      | `Description`      | `Text[description]` | Full parity        |
| ErrorMessage     | `ErrorMessage`     | `FieldError`        | Full parity        |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui           | React Aria         | Notes       |
| ------------- | ---------------- | ------------------ | ----------- |
| Value changed | `Change(String)` | `onChange`         | Full parity |
| Submit        | `Submit`         | `onSubmit`         | Full parity |
| Clear         | `Clear`          | `onClear`          | Full parity |
| Focus         | `Focus`/`Blur`   | `onFocus`/`onBlur` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature         | ars-ui | React Aria |
| --------------- | ------ | ---------- |
| Clear on Escape | Yes    | Yes        |
| Submit on Enter | Yes    | Yes        |
| Loading state   | Yes    | --         |
| Debounce        | Yes    | --         |
| IME composition | Yes    | Yes        |
| `type="search"` | Yes    | Yes        |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with React Aria SearchField.
- **Divergences:** ars-ui adds a loading/searching state, debounce support, submit trigger, and loading indicator not present in React Aria.
- **Recommended additions:** None.
