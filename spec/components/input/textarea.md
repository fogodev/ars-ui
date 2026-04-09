---
component: Textarea
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [text-field]
references:
  ark-ui: Field
  react-aria: TextField
---

# Textarea

A multi-line text input with optional auto-resize behavior, character count, and standard form field parts (label, description, error message).

## 1. State Machine

### 1.1 States

```rust
/// The state of the Textarea component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is idle (not focused).
    Idle,
    /// The component is focused.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the Textarea component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus { is_keyboard: bool },
    /// The component lost focus.
    Blur,
    /// The text value changed.
    Change(String),
    /// The text was cleared.
    Clear,
    /// Validation state changed.
    SetInvalid(bool),
    /// IME composition started.
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
}
```

### 1.3 Context

```rust
/// The resize mode of the textarea.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeMode {
    /// No resizing allowed.
    None,
    /// Both horizontal and vertical resizing.
    Both,
    /// Horizontal resizing only.
    Horizontal,
    /// Vertical resizing only.
    Vertical,
}

/// The context for the Textarea component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The text value — controlled or uncontrolled.
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
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The placeholder text.
    pub placeholder: Option<String>,
    /// The maximum character length.
    pub max_length: Option<u32>,
    /// The minimum character length.
    pub min_length: Option<u32>,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The autocomplete hint.
    pub autocomplete: Option<String>,
    /// Number of visible text rows.
    pub rows: u32,
    /// Number of visible text columns.
    pub cols: Option<u32>,
    /// The resize mode.
    pub resize: ResizeMode,
    /// Whether the textarea auto-resizes to fit content.
    pub auto_resize: bool,
    /// Maximum height constraint for auto-resize (CSS value).
    pub max_height: Option<String>,
    /// True while an IME composition session is active.
    pub is_composing: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Text direction for RTL support.
    pub dir: Direction,
    /// Mobile on-screen keyboard layout hint.
    pub input_mode: Option<InputMode>,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the Textarea component.
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
    /// The placeholder text.
    pub placeholder: Option<String>,
    /// The maximum character length.
    pub max_length: Option<u32>,
    /// The minimum character length.
    pub min_length: Option<u32>,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// The autocomplete hint.
    pub autocomplete: Option<String>,
    /// Number of visible text rows. Default: 3.
    pub rows: u32,
    /// Number of visible text columns.
    pub cols: Option<u32>,
    /// The resize mode. Default: Vertical.
    pub resize: ResizeMode,
    /// Whether the textarea auto-resizes to fit content.
    pub auto_resize: bool,
    /// Maximum height constraint for auto-resize (CSS value).
    pub max_height: Option<String>,
    /// Maximum number of rows for auto-resize height capping.
    pub max_rows: Option<u32>,
    /// The direction of the component.
    pub dir: Direction,
    /// Hint for the virtual keyboard type on mobile devices.
    pub input_mode: Option<InputMode>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: String::new(),
            disabled: false, readonly: false, invalid: false, required: false,
            placeholder: None, max_length: None, min_length: None,
            name: None, form: None, autocomplete: None,
            rows: 3, cols: None,
            resize: ResizeMode::Vertical,
            auto_resize: false, max_height: None, max_rows: None,
            dir: Direction::Ltr, input_mode: None,
        }
    }
}
```

### 1.5 Auto-Resize Behavior

When `auto_resize=true`, the machine emits an auto-resize effect after every `Change` event. The adapter handles auto-resize via `platform.resize_to_content()` (see `PlatformEffects` in `01-architecture.md` section 2.2.7). The effect closure in `Event::Change` already calls this method -- no standalone resize function is needed in core.

**Growth rules:**

1. **Height Calculation**: On each input event, set `height: auto` momentarily, read `scrollHeight`, then set `height` to `scrollHeight + border-box adjustment`. This ensures the textarea grows to fit content without scrollbars.
2. **Growth Timing**: Height recalculation runs synchronously after the input event, before the next paint. Use a layout effect (not a regular effect) to avoid visual flicker.
3. **Maximum Height**: When `max_rows` is specified, compute `max_height = max_rows * line_height + padding`. Once `scrollHeight` exceeds `max_height`, set `height: max_height` and `overflow-y: auto` to enable scrolling within the capped area.
4. **Shrinking**: The textarea shrinks when content is deleted. The `height: auto` reset before measurement ensures correct shrink behavior.
5. **External Value Changes**: When the `value` prop is updated externally (controlled mode), trigger the same height recalculation in a layout effect.

### 1.6 Full Machine Implementation

```rust
/// The machine for the Textarea component.
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
        let state = State::Idle;
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
            max_length: props.max_length,
            min_length: props.min_length,
            name: props.name.clone(),
            autocomplete: props.autocomplete.clone(),
            rows: props.rows,
            cols: props.cols,
            resize: props.resize,
            auto_resize: props.auto_resize,
            max_height: props.max_height.clone(),
            is_composing: false,
            has_description: false,
            dir: props.dir,
            input_mode: props.input_mode.clone(),
            ids: ComponentIds::from_id(&props.id),
        };
        (state, ctx)
    }

    fn transition(
        _state: &Self::State,
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
                if ctx.disabled || ctx.readonly { return None; }
                let val = val.clone();
                let auto_resize = ctx.auto_resize;
                let max_height = ctx.max_height.clone();
                let mut plan = TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(val);
                });
                if auto_resize {
                    plan = plan.with_effect(PendingEffect::new("auto_resize", move |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let input_id = ctx.ids.part("input");
                        platform.resize_to_content(&input_id, max_height.as_deref());
                        no_cleanup()
                    }));
                }
                Some(plan)
            }
            Event::Clear => {
                if ctx.disabled || ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.value.set(String::new());
                }))
            }
            Event::SetInvalid(invalid) => {
                let inv = *invalid;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.invalid = inv;
                }))
            }
            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = true; }))
            }
            Event::CompositionEnd => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = false; }))
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
#[scope = "textarea"]
pub enum Part {
    Root,
    Label,
    Textarea,
    CharacterCount,
    Description,
    ErrorMessage,
}

/// The API for the Textarea component.
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
        attrs.set(HtmlAttr::For, self.ctx.ids.part("textarea"));
        attrs
    }

    /// Attributes for the textarea element.
    pub fn textarea_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Textarea.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("textarea"));
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_str());
        if let Some(im) = &self.ctx.input_mode {
            attrs.set(HtmlAttr::InputMode, im.as_str());
        }
        attrs.set(HtmlAttr::Rows, self.ctx.rows.to_string());
        if let Some(cols) = self.ctx.cols { attrs.set(HtmlAttr::Cols, cols.to_string()); }
        attrs.set(HtmlAttr::Value, self.ctx.value.get());
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Required, true); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if let Some(p) = &self.ctx.placeholder { attrs.set(HtmlAttr::Placeholder, p); }
        if let Some(max) = self.ctx.max_length { attrs.set(HtmlAttr::MaxLength, max.to_string()); }
        if let Some(min) = self.ctx.min_length { attrs.set(HtmlAttr::MinLength, min.to_string()); }
        if let Some(ac) = &self.ctx.autocomplete { attrs.set(HtmlAttr::Autocomplete, ac); }
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
        // CSS resize property
        attrs.set_style(CssProperty::Resize, match self.ctx.resize {
            ResizeMode::None => "none",
            ResizeMode::Both => "both",
            ResizeMode::Horizontal => "horizontal",
            ResizeMode::Vertical => "vertical",
        });
        attrs
    }

    /// Attributes for the character count element.
    pub fn character_count_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CharacterCount.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
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

    pub fn on_textarea_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }
    pub fn on_textarea_blur(&self) { (self.send)(Event::Blur); }
    pub fn on_textarea_change(&self, val: String) { (self.send)(Event::Change(val)); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Textarea => self.textarea_attrs(),
            Part::CharacterCount => self.character_count_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Textarea
├── Root               <div>       data-ars-scope="textarea" data-ars-part="root"
├── Label              <label>     data-ars-part="label"
├── Textarea      [A]  <textarea>  data-ars-part="textarea"
├── CharacterCount     <div>       data-ars-part="character-count" (optional, aria-live)
├── Description        <div>       data-ars-part="description" (optional)
└── ErrorMessage       <div>       data-ars-part="error-message" (optional)
```

| Part           | Element      | Key Attributes                                             |
| -------------- | ------------ | ---------------------------------------------------------- |
| Root           | `<div>`      | `data-ars-scope="textarea"`, `data-ars-state`              |
| Label          | `<label>`    | `for` points to Textarea                                   |
| Textarea       | `<textarea>` | `rows`, `cols`, `aria-labelledby`, `aria-describedby`      |
| CharacterCount | `<div>`      | `aria-live="polite"`, `aria-atomic="true"` (optional)      |
| Description    | `<div>`      | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage   | `<div>`      | Validation error; linked via `aria-describedby` (optional) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element  | Value                                    |
| ------------------ | -------- | ---------------------------------------- |
| `aria-invalid`     | Textarea | Present when `invalid=true`              |
| `aria-required`    | Textarea | Implicit via `required` attribute        |
| `aria-labelledby`  | Textarea | Points to Label id                       |
| `aria-describedby` | Textarea | Points to Description + ErrorMessage ids |

- Character count element uses `aria-live="polite"` so screen readers announce remaining characters on change.
- Auto-resize does not affect accessibility — no ARIA changes needed.

### 3.2 Keyboard Interaction

| Key | Action                      |
| --- | --------------------------- |
| Tab | Move focus to/from textarea |

> Enter inserts a newline (native `<textarea>` behavior). No custom keyboard handling is needed for text entry.

### 3.3 IME Composition Handling

See [IME Composition Protocol](./_category.md#ime-composition-protocol). During `compositionstart`...`compositionend`, adapters must suppress value change callbacks. The machine's `is_composing: bool` tracks composition state.

## 4. Internationalization

- `dir` attribute set from locale direction — ensures text entry direction matches language.
- `placeholder` text is user-provided and must be localized by consumer.
- `resize: Horizontal` should be `resize: Vertical` in vertical writing modes.
- Character count format: "42 / 200" — numbers formatted per locale.
- RTL: Text direction follows the document locale.

## 5. Form Integration

- **Hidden input**: The `Textarea` part is the native `<textarea>` element and participates directly in form submission via its `name` attribute. No separate hidden input is needed.
- **Validation states**: `aria-invalid="true"` on the Textarea when `invalid=true`. Native constraint validation attributes (`required`, `minlength`, `maxlength`) are set directly.
- **Error message association**: `aria-describedby` on Textarea points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: Native `required` attribute on Textarea.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: React Aria (`TextField` with `TextArea` sub-component).
>
> Note: Ark UI `Field.Textarea` has an `autoresize` prop but no standalone Textarea component. Radix UI has no Textarea component.

### 6.1 Props

| Feature          | ars-ui                         | React Aria              | Notes                                            |
| ---------------- | ------------------------------ | ----------------------- | ------------------------------------------------ |
| Controlled value | `value: Option<String>`        | `value`                 | Full parity                                      |
| Default value    | `default_value: String`        | `defaultValue`          | Full parity                                      |
| Disabled         | `disabled: bool`               | `isDisabled`            | Full parity                                      |
| Read-only        | `readonly: bool`               | `isReadOnly`            | Full parity                                      |
| Required         | `required: bool`               | `isRequired`            | Full parity                                      |
| Invalid          | `invalid: bool`                | `isInvalid`             | Full parity                                      |
| Placeholder      | `placeholder: Option<String>`  | `placeholder`           | Full parity                                      |
| Max/min length   | `max_length`/`min_length`      | `maxLength`/`minLength` | Full parity                                      |
| Autocomplete     | `autocomplete: Option<String>` | `autoComplete`          | Full parity                                      |
| Form name        | `name: Option<String>`         | `name`                  | Full parity                                      |
| Form ID          | `form: Option<String>`         | `form`                  | Full parity                                      |
| Rows/cols        | `rows`/`cols`                  | (native attrs)          | Full parity                                      |
| Auto-resize      | `auto_resize: bool`            | --                      | ars-ui enhancement (Ark `autoresize` equivalent) |
| Resize mode      | `resize: ResizeMode`           | --                      | ars-ui enhancement                               |
| Max height       | `max_height: Option<String>`   | --                      | ars-ui enhancement                               |
| Character count  | via `CharacterCount` part      | --                      | ars-ui enhancement                               |

**Gaps:** None.

### 6.2 Anatomy

| Part           | ars-ui           | React Aria          | Notes              |
| -------------- | ---------------- | ------------------- | ------------------ |
| Root           | `Root`           | `TextField`         | Full parity        |
| Label          | `Label`          | `Label`             | Full parity        |
| Textarea       | `Textarea`       | `TextArea`          | Full parity        |
| CharacterCount | `CharacterCount` | --                  | ars-ui enhancement |
| Description    | `Description`    | `Text[description]` | Full parity        |
| ErrorMessage   | `ErrorMessage`   | `FieldError`        | Full parity        |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui           | React Aria         | Notes       |
| ------------- | ---------------- | ------------------ | ----------- |
| Value changed | `Change(String)` | `onChange`         | Full parity |
| Focus         | `Focus`/`Blur`   | `onFocus`/`onBlur` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature          | ars-ui                | React Aria |
| ---------------- | --------------------- | ---------- |
| Auto-resize      | Yes                   | --         |
| Character count  | Yes                   | --         |
| IME composition  | Yes                   | Yes        |
| Form integration | Yes (native textarea) | Built-in   |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with React Aria TextField's TextArea mode.
- **Divergences:** ars-ui adds auto-resize, character count, and resize mode controls not present in React Aria. Matches Ark UI's `Field.Textarea` `autoresize` prop.
- **Recommended additions:** None.
