---
component: PinInput
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
  ark-ui: PinInput
  radix-ui: OneTimePasswordField
---

# PinInput

A row of single-character input cells for PIN, OTP, or verification code entry.

## 1. State Machine

### 1.1 States

```rust
/// The state of the PinInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused {
        /// The index of the focused cell.
        index: usize,
    },
    /// The component is in a completed state.
    Completed,
}
```

### 1.2 Events

```rust
/// The events for the PinInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus {
        /// The index of the focused cell.
        index: usize,
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// The component lost focus.
    Blur,
    /// A character was input into a cell.
    InputChar {
        /// The index of the cell that received input.
        index: usize,
        /// The character that was input.
        char: char,
    },
    /// A character was deleted from a cell.
    DeleteChar {
        /// The index of the cell that was deleted.
        index: usize,
    },
    /// Text was pasted.
    Paste(String),
    /// All cells were cleared.
    Clear,
    /// Pin entry is complete.
    Complete(String),
    /// Focus the next cell.
    FocusNext,
    /// Focus the previous cell.
    FocusPrev,
    /// IME composition started.
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
}
```

### 1.3 Context

```rust
/// The input mode of the PinInput component.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Only digits accepted.
    Numeric,
    /// Letters and digits accepted.
    Alphanumeric,
    /// Any character accepted (masked display).
    Password,
}

/// The context of the PinInput component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The value (one string per cell) — controlled or uncontrolled.
    pub value: Bindable<Vec<String>>,
    /// Number of cells.
    pub length: usize,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether the component is an OTP input.
    pub otp: bool,
    /// Whether the characters are masked.
    pub mask: bool,
    /// Placeholder character for empty cells.
    pub placeholder: Option<String>,
    /// The index of the focused cell.
    pub focused_index: Option<usize>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether all cells are filled.
    pub complete: bool,
    /// The input validation mode.
    pub mode: Mode,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// When true, select cell content on focus.
    pub select_on_focus: bool,
    /// When true, blur the component after all cells are filled.
    pub blur_on_complete: bool,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved messages for the pin input.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the PinInput component.
#[derive(Clone, HasId)]
// Manual Debug/PartialEq impls needed (Callback fields)
pub struct Props {
    pub id: String,
    /// Controlled value. When Some, component is controlled.
    pub value: Option<Vec<String>>,
    /// Default value for uncontrolled mode.
    pub default_value: Vec<String>,
    /// Number of cells.
    pub length: usize,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Whether this is an OTP input.
    pub otp: bool,
    /// Whether characters are masked.
    pub mask: bool,
    /// Placeholder character for empty cells.
    pub placeholder: Option<String>,
    /// The input validation mode.
    pub mode: Mode,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether the pin input is required.
    pub required: bool,
    /// Whether the pin input is read-only.
    pub readonly: bool,
    /// When `true`, select the content of each cell when it receives focus.
    /// Default: `false`.
    pub select_on_focus: bool,
    /// When `true`, the component loses focus after all cells are filled.
    /// Default: `false`.
    pub blur_on_complete: bool,
    /// When true, automatically fires `on_value_complete` when all digits are entered.
    pub auto_submit: bool,
    /// Callback fired when all slots are filled.
    pub on_value_complete: Option<Callback<dyn Fn(&str)>>,
    /// Translatable messages.
    pub messages: Option<Messages>,
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: Vec::new(),
            length: 6,
            disabled: false,
            invalid: false,
            otp: false,
            mask: false,
            placeholder: None,
            mode: Mode::Numeric,
            name: None,
            form: None,
            required: false,
            readonly: false,
            select_on_focus: false,
            blur_on_complete: false,
            auto_submit: false,
            on_value_complete: None,
            messages: None,
            locale: None,
        }
    }
}

/// Messages for the PinInput component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Ordinal label for a cell at position `pos` of `total`.
    pub ordinal_label: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            ordinal_label: MessageFn::new(|pos, total, _locale| format!("Digit {pos} of {total}")),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.5 Full Machine Implementation

```rust
/// Machine for the PinInput component.
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
        let initial = props.value.clone()
            .unwrap_or_else(|| {
                let mut v = props.default_value.clone();
                v.resize(props.length, String::new());
                v
            });
        let complete = initial.iter().all(|v| !v.is_empty());
        let state = if complete { State::Completed } else { State::Idle };
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(initial),
            },
            length: props.length,
            disabled: props.disabled,
            invalid: props.invalid,
            otp: props.otp,
            mask: props.mask,
            placeholder: props.placeholder.clone(),
            focused_index: None,
            focus_visible: false,
            complete,
            mode: props.mode.clone(),
            name: props.name.clone(),
            select_on_focus: props.select_on_focus,
            blur_on_complete: props.blur_on_complete,
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
        // Disabled guard
        if ctx.disabled {
            match event {
                Event::InputChar { .. } | Event::DeleteChar { .. }
                | Event::Paste(_) | Event::Clear | Event::Complete(_) => return None,
                _ => {}
            }
        }

        match (state, event) {
            // ── Focus / Blur ──────────────────────────────────────
            // When `ctx.select_on_focus` is true, the adapter calls `.select()` on the
            // input element after this transition completes. The machine only sets state;
            // the adapter reads `ctx.select_on_focus` in its focus event handler.
            (_, Event::Focus { index, is_keyboard }) => {
                let idx = *index;
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused { index: idx }).apply(move |ctx| {
                    ctx.focused_index = Some(idx);
                    ctx.focus_visible = is_kb;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::to(if ctx.complete { State::Completed } else { State::Idle })
                    .apply(|ctx| {
                        ctx.focused_index = None;
                        ctx.focus_visible = false;
                    }))
            }

            // ── InputChar ─────────────────────────────────────────
            (State::Focused { .. }, Event::InputChar { index, char: c }) => {
                let c = *c;
                let i = *index;
                let length = ctx.length;
                let mut vals = ctx.value.get().clone();
                vals[i] = c.to_string();

                if vals.iter().all(|v| !v.is_empty()) {
                    let combined: String = vals.iter().map(|v| v.as_str()).collect();
                    let blur = ctx.blur_on_complete;
                    let mut plan = TransitionPlan::to(State::Completed).apply(move |ctx| {
                        ctx.value.set(vals);
                        ctx.complete = true;
                    }).with_effect(PendingEffect::new("complete", move |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_value_complete {
                            let cb = cb.clone();
                            Box::new(move || { cb(&combined); })
                        } else {
                            no_cleanup()
                        }
                    }));
                    if blur {
                        plan = plan.with_effect(PendingEffect::new("blur-on-complete", |_ctx, _props, _send| {
                            let platform = use_platform_effects();
                            platform.focus_body();
                            no_cleanup()
                        }));
                    }
                    Some(plan)
                } else {
                    let next = (i + 1..length).find(|j| vals[*j].is_empty()).unwrap_or(i);
                    Some(TransitionPlan::to(State::Focused { index: next }).apply(move |ctx| {
                        ctx.value.set(vals);
                        ctx.focused_index = Some(next);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let input_id = ctx.ids.item("input", ctx.focused_index.unwrap_or(0));
                        platform.focus_element_by_id(&input_id);
                        no_cleanup()
                    })))
                }
            }

            // ── DeleteChar ────────────────────────────────────────
            (_, Event::DeleteChar { index }) => {
                let index = *index;
                let vals = ctx.value.get().clone();
                if vals[index].is_empty() && index > 0 {
                    let prev = index - 1;
                    Some(TransitionPlan::to(State::Focused { index: prev }).apply(move |ctx| {
                        let mut vals = ctx.value.get().clone();
                        vals[prev] = String::new();
                        ctx.value.set(vals);
                        ctx.focused_index = Some(prev);
                        ctx.complete = false;
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let input_id = ctx.ids.item("input", ctx.focused_index.unwrap_or(0));
                        platform.focus_element_by_id(&input_id);
                        no_cleanup()
                    })))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        let mut vals = ctx.value.get().clone();
                        vals[index] = String::new();
                        ctx.value.set(vals);
                        ctx.complete = false;
                    }))
                }
            }

            // ── Paste ─────────────────────────────────────────────
            (_, Event::Paste(text)) => {
                let mode = ctx.mode.clone();
                let chars: Vec<char> = text.chars()
                    .filter(|c| match mode {
                        Mode::Numeric => c.is_ascii_digit(),
                        Mode::Alphanumeric => c.is_alphanumeric(),
                        Mode::Password => true,
                    })
                    .take(ctx.length)
                    .collect();
                let mut vals = ctx.value.get().clone();
                let start = ctx.focused_index.unwrap_or(0);
                let length = ctx.length;
                for (i, ch) in chars.iter().enumerate() {
                    if start + i < length { vals[start + i] = ch.to_string(); }
                }

                if vals.iter().all(|v| !v.is_empty()) {
                    let combined: String = vals.iter().map(|v| v.as_str()).collect();
                    let blur = ctx.blur_on_complete;
                    let mut plan = TransitionPlan::to(State::Completed).apply(move |ctx| {
                        ctx.value.set(vals);
                        ctx.complete = true;
                    }).with_effect(PendingEffect::new("complete", move |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_value_complete {
                            let cb = cb.clone();
                            Box::new(move || { cb(&combined); })
                        } else { no_cleanup() }
                    }));
                    if blur {
                        plan = plan.with_effect(PendingEffect::new("blur-on-complete", |_ctx, _props, _send| {
                            let platform = use_platform_effects();
                            platform.focus_body();
                            no_cleanup()
                        }));
                    }
                    Some(plan)
                } else {
                    Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(vals); }))
                }
            }

            // ── Clear ─────────────────────────────────────────────
            (_, Event::Clear) => {
                let length = ctx.length;
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.value.set(vec![String::new(); length]);
                    ctx.complete = false;
                    ctx.focused_index = None;
                }))
            }

            // ── Navigation ────────────────────────────────────────
            (_, Event::FocusPrev) => {
                let current = ctx.focused_index.unwrap_or(0);
                if current == 0 { return None; }
                let prev = current - 1;
                Some(TransitionPlan::to(State::Focused { index: prev }).apply(move |ctx| {
                    ctx.focused_index = Some(prev);
                }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let input_id = ctx.ids.item("input", ctx.focused_index.unwrap_or(0));
                    platform.focus_element_by_id(&input_id);
                    no_cleanup()
                })))
            }

            (_, Event::FocusNext) => {
                let current = ctx.focused_index.unwrap_or(0);
                if current + 1 >= ctx.length { return None; }
                let next = current + 1;
                Some(TransitionPlan::to(State::Focused { index: next }).apply(move |ctx| {
                    ctx.focused_index = Some(next);
                }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let input_id = ctx.ids.item("input", ctx.focused_index.unwrap_or(0));
                    platform.focus_element_by_id(&input_id);
                    no_cleanup()
                })))
            }

            // ── IME Composition ───────────────────────────────────
            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = true; }))
            }
            (_, Event::CompositionEnd) => {
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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "pin-input"]
pub enum Part {
    Root,
    Label,
    Input { cell_index: usize },       // cell index
    HiddenInput,
    Description,
    ErrorMessage,
}

/// The API for the PinInput component.
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
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused { .. } => "focused",
            State::Completed => "completed",
        });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    /// Attributes for the group label.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    /// Attributes for a single cell input.
    pub fn input_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input { cell_index: index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let input_id = self.ctx.ids.item("input", index);
        attrs.set(HtmlAttr::Id, input_id);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set(HtmlAttr::Type, if self.ctx.mask { "password" } else { "text" });
        attrs.set(HtmlAttr::InputMode, match self.ctx.mode {
            Mode::Numeric => "numeric",
            Mode::Alphanumeric | Mode::Password => "text",
        });
        attrs.set(HtmlAttr::MaxLength, "1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.ordinal_label)(index + 1, self.ctx.length, &self.ctx.locale));
        if self.ctx.otp { attrs.set(HtmlAttr::Autocomplete, "one-time-code"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.props.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
        let is_focused = self.ctx.focused_index == Some(index);
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if let Some(ref val) = self.ctx.value.get().get(index) {
            if !val.is_empty() {
                attrs.set(HtmlAttr::Value, val);
            }
        }
        if let Some(ref ph) = self.ctx.placeholder {
            attrs.set(HtmlAttr::Placeholder, ph);
        }
        attrs
    }

    /// Attributes for the hidden input (form submission).
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        let combined: String = self.ctx.value.get().iter().map(|v| v.as_str()).collect();
        attrs.set(HtmlAttr::Value, combined);
        if let Some(ref name) = self.ctx.name { attrs.set(HtmlAttr::Name, name); }
        if let Some(ref form) = self.props.form { attrs.set(HtmlAttr::Form, form); }
        if self.props.required { attrs.set_bool(HtmlAttr::Required, true); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
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

    pub fn on_cell_keydown(&self, index: usize, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrev),
            KeyboardKey::ArrowRight => (self.send)(Event::FocusNext),
            KeyboardKey::Backspace | KeyboardKey::Delete => (self.send)(Event::DeleteChar { index }),
            _ => {}
        }
    }

    pub fn on_cell_focus(&self, index: usize, is_keyboard: bool) {
        (self.send)(Event::Focus { index, is_keyboard });
    }

    pub fn on_cell_blur(&self) { (self.send)(Event::Blur); }

    pub fn on_cell_input(&self, index: usize, ch: char) {
        (self.send)(Event::InputChar { index, char: ch });
    }

    pub fn on_paste(&self, text: String) { (self.send)(Event::Paste(text)); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input { cell_index } => self.input_attrs(cell_index),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
PinInput
├── Root               <div>    data-ars-scope="pin-input" data-ars-part="root" (role="group")
├── Label              <label>  data-ars-part="label"
├── Input (×N)         <input>  data-ars-part="input" data-ars-index="{n}"
├── HiddenInput        <input>  data-ars-part="hidden-input" (type="hidden", aria-hidden)
├── Description        <div>    data-ars-part="description" (optional)
└── ErrorMessage       <div>    data-ars-part="error-message" (optional)
```

| Part         | Element   | Key Attributes                                             |
| ------------ | --------- | ---------------------------------------------------------- |
| Root         | `<div>`   | `role="group"`, `data-ars-state`                           |
| Label        | `<label>` | Group label                                                |
| Input        | `<input>` | Per-cell; `aria-label`, `maxlength="1"`, roving tabindex   |
| HiddenInput  | `<input>` | `type="hidden"` — combined value for form submission       |
| Description  | `<div>`   | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage | `<div>`   | Validation error; linked via `aria-describedby` (optional) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property       | Element | Value                                                   |
| -------------- | ------- | ------------------------------------------------------- |
| `role`         | Root    | `group`                                                 |
| `aria-label`   | Input   | "Digit {n} of {total}" (localized via `ordinal_label`)  |
| `autocomplete` | Input   | `"one-time-code"` when `otp=true`                       |
| `inputmode`    | Input   | `"numeric"` (Numeric), `"text"` (Alphanumeric/Password) |
| `aria-invalid` | Input   | Present when group is invalid                           |
| `tabindex`     | Input   | Roving: focused cell `0`, all others `-1`               |

### 3.2 Keyboard Interaction

| Key            | Action                         |
| -------------- | ------------------------------ |
| 0-9 / a-z      | Fill cell, auto-advance        |
| Backspace      | Clear cell or move to previous |
| Delete         | Clear current cell             |
| ArrowLeft      | Move to previous cell          |
| ArrowRight     | Move to next cell              |
| Ctrl+V / Cmd+V | Paste across cells             |

### 3.3 Screen Reader Guidance

1. Each input field has `aria-label` in format `"Digit {n} of {total}"` (localized via `ars-i18n`).
2. On auto-advance, the screen reader announces the new field via focus move.
3. On paste, announce `"Code pasted"` via live region after all digits are filled.
4. Group container has `role="group"` with `aria-label` describing the purpose (e.g., "Verification code").

## 4. Internationalization

- Cell label `"Digit N of M"` uses ordinal formatting and localization via `Messages.ordinal_label`.
- Numeric mode filters based on Unicode digit category, not just ASCII 0-9.
- RTL: Cells render right-to-left; ArrowLeft moves forward visually (to the right in LTR).

> **Timing protection:** For `Mode::Password`, the `Completed` state transition SHOULD be delayed by a configurable `submit_delay_ms` (default: 0, security-sensitive applications may set 200-500ms) to prevent per-character timing side-channel analysis. The `data-ars-state` attribute should not change to `"completed"` until after the delay.

## 5. Form Integration

- **Hidden input**: A single `<input type="hidden">` via `HiddenInput` submits the concatenated pin value. The `name` attribute comes from context.
- **Validation states**: `aria-invalid="true"` on each cell input when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby` on the Root.
- **Error message association**: The group Root element can reference Description and ErrorMessage via `aria-describedby`.
- **Reset behavior**: On form reset, the adapter restores all cells to `default_value` and transitions to Idle.
- **Disabled propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`PinInput`), Radix UI (`OneTimePasswordField` preview).

### 6.1 Props

| Feature          | ars-ui                        | Ark UI                                   | Radix UI                   | Notes          |
| ---------------- | ----------------------------- | ---------------------------------------- | -------------------------- | -------------- |
| Controlled value | `value: Option<Vec<String>>`  | `value: string[]`                        | `value: string`            | Full parity    |
| Default value    | `default_value: Vec<String>`  | `defaultValue`                           | `defaultValue`             | Full parity    |
| Length           | `length: usize`               | `count`                                  | (derived from Input count) | Full parity    |
| Disabled         | `disabled: bool`              | `disabled`                               | `disabled`                 | Full parity    |
| Invalid          | `invalid: bool`               | `invalid`                                | --                         | Ark parity     |
| Read-only        | `readonly: bool`              | `readOnly`                               | `readOnly`                 | Full parity    |
| Required         | `required: bool`              | `required`                               | --                         | Ark parity     |
| OTP mode         | `otp: bool`                   | `otp`                                    | `autoComplete`             | Full parity    |
| Mask             | `mask: bool`                  | `mask`                                   | `type: "password"`         | Full parity    |
| Placeholder      | `placeholder: Option<String>` | `placeholder`                            | `placeholder`              | Full parity    |
| Mode             | `mode: Mode`                  | `type` (numeric/alphanumeric/alphabetic) | `validationType`           | Full parity    |
| Form name        | `name: Option<String>`        | `name`                                   | `name`                     | Full parity    |
| Form ID          | `form: Option<String>`        | `form`                                   | `form`                     | Full parity    |
| Select on focus  | `select_on_focus: bool`       | `selectOnFocus`                          | --                         | Ark parity     |
| Blur on complete | `blur_on_complete: bool`      | `blurOnComplete`                         | --                         | Ark parity     |
| Auto-submit      | `auto_submit: bool`           | --                                       | `autoSubmit`               | Radix parity   |
| Pattern          | --                            | `pattern`                                | `sanitizeValue`            | See note below |

**Gaps:** None material. Ark's `pattern` prop is a regex filter; ars-ui's `Mode` enum provides the same filtering. Radix's `sanitizeValue` callback is more flexible but only applies when `validationType: "none"`.

### 6.2 Anatomy

| Part             | ars-ui                 | Ark UI                      | Radix UI      | Notes                                            |
| ---------------- | ---------------------- | --------------------------- | ------------- | ------------------------------------------------ |
| Root             | `Root`                 | `Root`                      | `Root`        | Full parity                                      |
| Label            | `Label`                | `Label`                     | --            | Ark parity                                       |
| Input (per cell) | `Input { cell_index }` | `Input` (with `index` prop) | `Input`       | Full parity                                      |
| HiddenInput      | `HiddenInput`          | `HiddenInput`               | `HiddenInput` | Full parity                                      |
| Control          | --                     | `Control`                   | --            | Ark layout wrapper; ars-ui Root serves this role |
| Description      | `Description`          | --                          | --            | ars-ui form-field part                           |
| ErrorMessage     | `ErrorMessage`         | --                          | --            | ars-ui form-field part                           |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui                                   | Ark UI               | Radix UI             | Notes                                |
| ------------- | ---------------------------------------- | -------------------- | -------------------- | ------------------------------------ |
| Value changed | `InputChar`/`DeleteChar`                 | `onValueChange`      | `onValueChange`      | Full parity                          |
| Completed     | `Complete(String)` / `on_value_complete` | `onValueComplete`    | `onAutoSubmit`       | Full parity                          |
| Invalid input | via `Mode` filtering                     | `onValueInvalid`     | --                   | Ark parity (ars-ui silently rejects) |
| Paste         | `Paste(String)`                          | (handled internally) | (handled internally) | Full parity                          |

**Gaps:** None.

### 6.4 Features

| Feature               | ars-ui | Ark UI | Radix UI |
| --------------------- | ------ | ------ | -------- |
| Auto-advance on input | Yes    | Yes    | Yes      |
| Backspace navigation  | Yes    | Yes    | Yes      |
| Paste support         | Yes    | Yes    | Yes      |
| OTP autocomplete      | Yes    | Yes    | Yes      |
| Masked display        | Yes    | Yes    | Yes      |
| Form integration      | Yes    | Yes    | Yes      |
| IME composition       | Yes    | --     | --       |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with both reference libraries.
- **Divergences:** ars-ui uses `Vec<String>` per-cell value vs. Radix's single string. ars-ui uses a `Mode` enum instead of Ark's string `type` prop. ars-ui adds IME composition tracking.
- **Recommended additions:** None.
