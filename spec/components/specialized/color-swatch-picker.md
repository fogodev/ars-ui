---
component: ColorSwatchPicker
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-swatch, color-picker]
references:
    ark-ui: ColorPicker
    react-aria: ColorSwatchPicker
---

# ColorSwatchPicker

An interactive group of color swatches implementing the ARIA listbox pattern.
Users navigate via arrow keys and select via Space/Enter. Supports grid (2D) and
stack (1D) layouts. The picker composes `ColorSwatch` for each item.

The layout mode is configured via `SwatchPickerLayout`:

```rust
/// The layout for the ColorSwatchPicker component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwatchPickerLayout {
    /// 2D grid with configurable columns. Default.
    Grid,
    /// 1D horizontal or vertical stack.
    Stack,
}

impl Default for SwatchPickerLayout {
    fn default() -> Self {
        SwatchPickerLayout::Grid
    }
}
```

## 1. State Machine

### 1.1 States

```rust
/// The states for the `ColorSwatchPicker` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// No focus within the picker.
    Idle,
    /// A swatch item is focused.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the `ColorSwatchPicker` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus entered the picker.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus left the picker.
    Blur,
    /// Select a color at the given index.
    Select {
        /// The index of the color to select.
        index: usize,
    },
    /// Navigate to the next swatch.
    FocusNext,
    /// Navigate to the previous swatch.
    FocusPrev,
    /// Grid-mode: navigate to the swatch in the row above.
    FocusUp,
    /// Grid-mode: navigate to the swatch in the row below.
    FocusDown,
    /// Jump to the first swatch.
    FocusFirst,
    /// Jump to the last swatch.
    FocusLast,
}
```

### 1.3 Context

```rust
/// The context for the `ColorSwatchPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The currently selected color.
    pub value: Bindable<ColorValue>,
    /// Index of the currently focused swatch, if any.
    pub focused_index: Option<usize>,
    /// The list of colors in the picker.
    pub colors: Vec<ColorValue>,
    /// Layout mode (grid or stack).
    pub layout: SwatchPickerLayout,
    /// Number of columns for grid layout.
    pub columns: usize,
    /// Whether the picker is disabled.
    pub disabled: bool,
    /// Whether focus is within the picker.
    pub focused: bool,
    /// Whether focus was via keyboard (for focus-visible ring).
    pub focus_visible: bool,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component instance IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the `ColorSwatchPicker` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,
    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,
    /// The list of colors to display as swatches.
    pub colors: Vec<ColorValue>,
    /// Layout mode. Default: Grid.
    pub layout: SwatchPickerLayout,
    /// Number of columns for grid layout. Default: 5.
    pub columns: usize,
    /// Whether the picker is disabled.
    pub disabled: bool,
    /// Name attribute for the hidden form input.
    pub name: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: ColorValue::default(),
            colors: Vec::new(),
            layout: SwatchPickerLayout::Grid,
            columns: 5,
            disabled: false,
            name: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the `ColorSwatchPicker` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
            value,
            focused_index: None,
            colors: props.colors.clone(),
            layout: props.layout,
            columns: props.columns,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return None;
        }

        let len = ctx.colors.len();
        if len == 0 {
            return None;
        }

        match event {
            Event::Focus { is_keyboard } => {
                let kb = *is_keyboard;

                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                    // Focus the selected item, or the first item.
                    if ctx.focused_index.is_none() {
                        let selected = ctx.colors.iter().position(|c| c == &ctx.value.get());
                        ctx.focused_index = Some(selected.unwrap_or(0));
                    }
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::Select { index } => {
                let idx = *index;

                Some(TransitionPlan::context_only(move |ctx| {
                    if idx < ctx.colors.len() {
                        ctx.value.set(ctx.colors[idx].clone());
                        ctx.focused_index = Some(idx);
                    }
                }))
            }

            Event::FocusNext => {
                if !matches!(state, State::Focused) { return None; }

                Some(TransitionPlan::context_only(move |ctx| {
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + 1) % ctx.colors.len());
                }))
            }

            Event::FocusPrev => {
                if !matches!(state, State::Focused) { return None; }

                Some(TransitionPlan::context_only(move |ctx| {
                    let current = ctx.focused_index.unwrap_or(0);
                    let len = ctx.colors.len();
                    ctx.focused_index = Some((current + len - 1) % len);
                }))
            }

            Event::FocusUp => {
                if !matches!(state, State::Focused) { return None; }

                if ctx.layout != SwatchPickerLayout::Grid { return None; }

                let cols = ctx.columns;

                Some(TransitionPlan::context_only(move |ctx| {
                    let current = ctx.focused_index.unwrap_or(0);
                    let len = ctx.colors.len();
                    ctx.focused_index = Some((current + len - cols) % len);
                }))
            }

            Event::FocusDown => {
                if !matches!(state, State::Focused) { return None; }

                if ctx.layout != SwatchPickerLayout::Grid { return None; }

                let cols = ctx.columns;

                Some(TransitionPlan::context_only(move |ctx| {
                    let current = ctx.focused_index.unwrap_or(0);
                    ctx.focused_index = Some((current + cols) % ctx.colors.len());
                }))
            }

            Event::FocusFirst => {
                if !matches!(state, State::Focused) { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_index = Some(0);
                }))
            }

            Event::FocusLast => {
                if !matches!(state, State::Focused) { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_index = Some(ctx.colors.len() - 1);
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
#[scope = "color-swatch-picker"]
pub enum Part {
    Root,
    Item { index: usize },
    HiddenInput,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "listbox");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
        });
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        match self.ctx.layout {
            SwatchPickerLayout::Stack => {
                attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");
            }
            SwatchPickerLayout::Grid => {
                // Grid: no aria-orientation (2D navigation)
            }
        }
        attrs
    }

    /// Attributes for the item wrapper at the given index.
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let item_id = self.ctx.ids.item("item", index);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "option");

        let is_selected = index < self.ctx.colors.len()
            && self.ctx.colors[index] == self.ctx.value.get();
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected),
            if is_selected { "true" } else { "false" });
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        let is_focused = self.ctx.focused_index == Some(index);
        // Roving tabindex
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Returns props for the embedded ColorSwatch within each item.
    pub fn item_swatch_props(&self, index: usize) -> color_swatch::Props {
        let color = if index < self.ctx.colors.len() {
            self.ctx.colors[index].clone()
        } else {
            ColorValue::default()
        };
        color_swatch::Props {
            id: self.ctx.ids.item("swatch", index),
            color,
            color_name: None,
            respect_alpha: true,
            messages: color_swatch::ColorSwatchMessages::default(),
        }
    }

    /// Hidden input for form participation.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(true));
        attrs
    }

    /// Handle keyboard navigation on the root/items.
    pub fn on_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowDown
                if self.ctx.layout == SwatchPickerLayout::Stack =>
            {
                (self.send)(Event::FocusNext);
            }
            KeyboardKey::ArrowLeft | KeyboardKey::ArrowUp
                if self.ctx.layout == SwatchPickerLayout::Stack =>
            {
                (self.send)(Event::FocusPrev);
            }
            KeyboardKey::ArrowRight if self.ctx.layout == SwatchPickerLayout::Grid => {
                (self.send)(Event::FocusNext);
            }
            KeyboardKey::ArrowLeft if self.ctx.layout == SwatchPickerLayout::Grid => {
                (self.send)(Event::FocusPrev);
            }
            KeyboardKey::ArrowDown if self.ctx.layout == SwatchPickerLayout::Grid => {
                (self.send)(Event::FocusDown);
            }
            KeyboardKey::ArrowUp if self.ctx.layout == SwatchPickerLayout::Grid => {
                (self.send)(Event::FocusUp);
            }
            KeyboardKey::Home => {
                (self.send)(Event::FocusFirst);
            }
            KeyboardKey::End => {
                (self.send)(Event::FocusLast);
            }
            KeyboardKey::Space | KeyboardKey::Enter => {
                if let Some(idx) = self.ctx.focused_index {
                    (self.send)(Event::Select { index: idx });
                }
            }
            _ => {}
        }
    }

    /// Handle click on a specific item.
    pub fn on_item_click(&self, index: usize) {
        (self.send)(Event::Select { index });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ColorSwatchPicker
├── Root         (required -- <div>, role="listbox", aria-label)
├── Item x N     (required -- <div>, role="option", aria-selected)
│   └── Swatch   (ColorSwatch component)
└── HiddenInput  (optional -- <input type="hidden">, form submission)
```

| Part        | Element   | Key Attributes                                             |
| ----------- | --------- | ---------------------------------------------------------- |
| Root        | `<div>`   | `role="listbox"`, `aria-label`, `aria-orientation` (stack) |
| Item        | `<div>`   | `role="option"`, `aria-selected`, roving `tabindex`        |
| HiddenInput | `<input>` | `type="hidden"`, `name`, `value` (hex color)               |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute          | Element | Value                                                 |
| ------------------ | ------- | ----------------------------------------------------- |
| `role="listbox"`   | Root    | ARIA listbox pattern                                  |
| `aria-label`       | Root    | From `messages.label` (default: "Color swatches")     |
| `aria-orientation` | Root    | `"horizontal"` for Stack layout; unset for Grid       |
| `aria-disabled`    | Root    | `"true"` when disabled                                |
| `role="option"`    | Item    | Individual swatch option                              |
| `aria-selected`    | Item    | `"true"` when this item's color is the selected value |
| `tabindex`         | Item    | Roving: `"0"` on focused item, `"-1"` on others       |

### 3.2 Keyboard Interaction

| Key               | Action                     |
| ----------------- | -------------------------- |
| ArrowRight        | FocusNext (both layouts)   |
| ArrowLeft         | FocusPrev (both layouts)   |
| ArrowDown (Stack) | FocusNext                  |
| ArrowUp (Stack)   | FocusPrev                  |
| ArrowDown (Grid)  | FocusDown (row navigation) |
| ArrowUp (Grid)    | FocusUp (row navigation)   |
| Home              | FocusFirst                 |
| End               | FocusLast                  |
| Space / Enter     | Select focused item        |

RTL: Arrow key direction does NOT flip for RTL in the listbox pattern -- ArrowRight
always moves to the next item, ArrowLeft to the previous. This follows WAI-ARIA
listbox conventions where arrow keys navigate the list structure, not spatial direction.

Grid navigation: ArrowUp/ArrowDown navigate by row (plus/minus columns), wrapping at boundaries.

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `ColorSwatchPicker` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the picker root.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { label: MessageFn::static_str("Color swatches") }
    }
}

impl ComponentMessages for Messages {}
```

| Key                         | Default (en-US)    | Purpose             |
| --------------------------- | ------------------ | ------------------- |
| `color_swatch_picker.label` | `"Color swatches"` | Picker `aria-label` |

Item labels: Each item's accessible name comes from the embedded `ColorSwatch`'s
`color_name()` (which uses `color_swatch::Messages::format_name` for locale ordering).

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker.SwatchGroup`/`SwatchTrigger`), React Aria (`ColorSwatchPicker`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                     | React Aria               | Notes                                              |
| ------------------------ | ------------------------- | -------------------------- | ------------------------ | -------------------------------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | (root-level)               | `value` / `defaultValue` | Equivalent                                         |
| `layout`                 | `layout` (Grid/Stack)     | --                         | `layout` (grid/stack)    | Equivalent                                         |
| `colors`                 | `colors`                  | (individual SwatchTrigger) | (individual Items)       | ars-ui takes a Vec; Ark/React Aria use composition |
| `disabled`               | `disabled`                | `disabled` (per swatch)    | --                       | ars-ui is group-level; Ark is per-item             |
| `columns`                | `columns`                 | --                         | --                       | ars-ui has grid column count                       |
| `name`                   | `name`                    | --                         | --                       | ars-ui has form input                              |

**Gaps:** None worth adopting. Per-item `isDisabled` from React Aria (`ColorSwatchPickerItem`) is a minor feature -- our grid-level disabled is sufficient.

### 5.2 Anatomy

| Part            | ars-ui           | Ark UI            | React Aria              | Notes                                                        |
| --------------- | ---------------- | ----------------- | ----------------------- | ------------------------------------------------------------ |
| Root            | `Root` (listbox) | `SwatchGroup`     | `ColorSwatchPicker`     | Equivalent                                                   |
| Item            | `Item` (option)  | `SwatchTrigger`   | `ColorSwatchPickerItem` | Equivalent                                                   |
| HiddenInput     | `HiddenInput`    | --                | --                      | ars-ui has form input                                        |
| SwatchIndicator | --               | `SwatchIndicator` | --                      | Ark has selection indicator; ars-ui uses `data-ars-selected` |

**Gaps:** None. Selection indication is handled via `data-ars-selected` data attribute, which the adapter can style.

### 5.3 Events

| Callback     | ars-ui                | Ark UI                 | React Aria | Notes      |
| ------------ | --------------------- | ---------------------- | ---------- | ---------- |
| Value change | `Bindable` reactivity | `onValueChange` (root) | `onChange` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                        | ars-ui     | Ark UI | React Aria |
| ------------------------------ | ---------- | ------ | ---------- |
| Grid layout with 2D navigation | Yes        | --     | Yes        |
| Stack layout                   | Yes        | --     | Yes        |
| Roving tabindex                | Yes        | --     | Yes        |
| Keyboard navigation            | Yes (full) | Yes    | Yes        |
| Embedded ColorSwatch           | Yes        | Yes    | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI defines swatches compositionally (individual SwatchTrigger children); ars-ui accepts a `colors: Vec<ColorValue>` for simplicity. React Aria also uses compositional items.
- **Recommended additions:** None.
