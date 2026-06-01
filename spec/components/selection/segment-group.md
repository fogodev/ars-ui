---
component: SegmentGroup
category: selection
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [radio-group, toggle-group]
references:
    ark-ui: SegmentGroup
---

# SegmentGroup

A `SegmentGroup` is a set of mutually exclusive options visually styled as connected segments with an animated selection indicator. Semantically equivalent to a single-select RadioGroup but with a segmented control appearance — commonly used for view switchers (e.g., "Grid" / "List"), display modes, or compact option selectors.

> Matches Ark UI's `SegmentGroup` component.

## 1. State Machine

### 1.1 States

```rust
/// The state of the SegmentGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No segment is focused.
    Idle,
    /// A segment has keyboard or pointer focus.
    Focused {
        /// The value of the focused segment.
        item: Key,
    },
}
```

### 1.2 Events

```rust
/// The events for the SegmentGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Select a segment by value.
    SelectValue(Key),
    /// Focus moved to a specific segment.
    FocusItem {
        /// The value of the focused segment.
        item: Key,
        /// Whether the focus was initiated by keyboard.
        is_keyboard: bool,
    },
    /// Focus left the group.
    Blur,
    /// Move focus to the next enabled segment.
    FocusNext,
    /// Move focus to the previous enabled segment.
    FocusPrev,
    /// Focus the first enabled segment.
    FocusFirst,
    /// Focus the last enabled segment.
    FocusLast,
    /// Register a mounted segment value in logical DOM order.
    RegisterItem(Key),
    /// Unregister a mounted segment value.
    UnregisterItem(Key),
    /// Synchronize controlled value props.
    SetValue(Option<Key>),
    /// Synchronize context-backed props.
    SetProps,
    /// Restore the selected value to `Props::default_value`.
    Reset,
}
```

### 1.3 Context

```rust
/// The context of the SegmentGroup component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The currently selected value.
    pub value: Bindable<Option<Key>>,
    /// The value of the currently focused segment.
    pub focused_item: Option<Key>,
    /// Whether focus was keyboard-initiated (for focus-visible styling).
    pub focus_visible: bool,
    /// Whether the entire group is disabled.
    pub disabled: bool,
    /// Whether the entire group is read-only.
    pub readonly: bool,
    /// Layout orientation (horizontal or vertical).
    pub orientation: Orientation,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// Whether focus wraps around at the ends.
    pub loop_focus: bool,
    /// Ordered list of segment definitions for navigation.
    pub items: Vec<Segment>,
    /// Mounted segment values in logical DOM order.
    pub registered_items: Vec<Key>,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// Definition of a single segment within the group.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// The value this segment represents.
    pub value: Key,
    /// Whether this individual segment is disabled.
    pub disabled: bool,
}
```

### 1.4 Props

```rust
use ars_i18n::{Orientation, Direction};

/// The props for the SegmentGroup component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the SegmentGroup component.
    pub id: String,
    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<Key>,
    /// Initial value for uncontrolled mode.
    pub default_value: Option<Key>,
    /// Whether the entire group is disabled.
    pub disabled: bool,
    /// Whether the group is read-only (value visible but not changeable).
    pub readonly: bool,
    /// Whether the segment group is in an invalid state.
    pub invalid: bool,
    /// The name for form submission.
    pub name: Option<String>,
    /// The ID of the form element the component is associated with.
    pub form: Option<String>,
    /// Layout orientation. Affects keyboard navigation:
    /// - `Horizontal` (default): Left/Right arrows navigate.
    /// - `Vertical`: Up/Down arrows navigate.
    pub orientation: Orientation,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// Whether focus wraps from last to first and vice versa.
    pub loop_focus: bool,
    /// Ordered segment definitions used for attrs and fallback navigation.
    pub items: Vec<Segment>,
    /// Called when user intent requests a new selected value.
    pub on_value_change: Option<Callback<dyn Fn(Option<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            disabled: false,
            readonly: false,
            invalid: false,
            name: None,
            form: None,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            items: Vec::new(),
            on_value_change: None,
        }
    }
}
```

### 1.5 Machine Behaviour

```rust
/// Machine for the SegmentGroup component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;
}

/// Typed effect intents emitted by the SegmentGroup machine.
pub enum Effect {
    /// Adapter invokes `Props::on_value_change` with the requested value.
    ValueChange,
    /// Adapter moves DOM focus to the item keyed by `Context::focused_item`.
    FocusItem,
}
```

Machine behaviour:

- `SelectValue` is ignored when the group is disabled or readonly, when the target segment is disabled, or when the requested value is already selected.
- Uncontrolled selection updates `Context::value` and emits `Effect::ValueChange`; controlled selection emits `Effect::ValueChange` without committing the requested value.
- `FocusItem`, `FocusNext`, `FocusPrev`, `FocusFirst`, and `FocusLast` only target enabled segment values.
- Arrow-key focus events emit `Effect::FocusItem`; the agnostic core does not focus or measure DOM elements by ID.
- `RegisterItem` and `UnregisterItem` maintain mounted logical order for adapter focus movement. When no items are registered, navigation falls back to `Props::items` order.
- `SetValue`, `SetProps`, and `Reset` keep controlled props, props-derived context, form reset behaviour, and item disabled state synchronized.

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "segment-group"]
pub enum Part {
    Root,
    Item { value: Key },
    ItemText { value: Key },
    Indicator,
    HiddenInput,
}

/// API for the SegmentGroup component.
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
        attrs.set(HtmlAttr::Role, "radiogroup");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.props.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        attrs
    }

    /// Handle keydown on the root element for arrow key navigation.
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        match data.key {
            KeyboardKey::ArrowRight => {
                if is_horizontal && is_rtl {
                    (self.send)(Event::FocusPrev)
                } else {
                    (self.send)(Event::FocusNext)
                }
            }
            KeyboardKey::ArrowLeft => {
                if is_horizontal && is_rtl {
                    (self.send)(Event::FocusNext)
                } else {
                    (self.send)(Event::FocusPrev)
                }
            }
            KeyboardKey::ArrowDown => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            _ => {}
        }
    }

    /// Attributes for a single segment item.
    pub fn item_attrs(&self, item_value: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", item_value);
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        let is_focused = self.ctx.focused_item.as_ref() == Some(item_value);
        let is_disabled = self.ctx.disabled || self.ctx.items.iter().any(|i| i.value == *item_value && i.disabled);

        // Roving tabindex: selected item (or first enabled if none selected) gets 0
        let is_tabbable = if self.ctx.value.get().is_some() {
            is_selected
        } else {
            self.ctx.items.iter()
                .find(|i| !i.disabled)
                .map(|i| &i.value) == Some(item_value)
        };

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "checked" } else { "unchecked" });
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "radio");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if is_selected { "true" } else { "false" });
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs.set(HtmlAttr::TabIndex, if is_tabbable { "0" } else { "-1" });
        if is_disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.focus_visible && is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Handle click on a segment item.
    pub fn on_item_click(&self, item_value: &Key) {
        (self.send)(Event::SelectValue(item_value.clone()));
    }

    /// Handle focus on a segment item.
    pub fn on_item_focus(&self, item_value: &Key, is_keyboard: bool) {
        (self.send)(Event::FocusItem {
            item: item_value.clone(),
            is_keyboard,
        });
    }

    /// Handle blur on a segment item.
    pub fn on_item_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handle keydown on a segment item.
    pub fn on_item_keydown(&self, item_value: &Key, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Space || data.key == KeyboardKey::Enter {
            (self.send)(Event::SelectValue(item_value.clone()));
        }
    }

    /// Attributes for the item text content.
    pub fn item_text_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemText { value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "checked" } else { "unchecked" });
        attrs
    }

    /// Attributes for the animated selection indicator.
    ///
    /// The adapter measures the selected item's bounding rect relative to the
    /// group root and sets CSS custom properties as inline styles:
    /// - `--ars-indicator-inset-inline-start` (RTL-safe inline offset)
    /// - `--ars-indicator-top`
    /// - `--ars-indicator-width`
    /// - `--ars-indicator-height`
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if let Some(selected) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Data("ars-active-value"), selected.to_string());
        }
        attrs
    }

    /// Attributes for the hidden input element (form submission).
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref val) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, val.to_string());
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref value } => self.item_attrs(value),
            Part::ItemText { ref value } => self.item_text_attrs(value),
            Part::Indicator => self.indicator_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

| Part          | Selector                                                         | Element                 |
| ------------- | ---------------------------------------------------------------- | ----------------------- |
| `Root`        | `[data-ars-scope="segment-group"][data-ars-part="root"]`         | `<div>`                 |
| `Item`        | `[data-ars-scope="segment-group"][data-ars-part="item"]`         | `<button>`              |
| `ItemText`    | `[data-ars-scope="segment-group"][data-ars-part="item-text"]`    | `<span>`                |
| `Indicator`   | `[data-ars-scope="segment-group"][data-ars-part="indicator"]`    | `<div>`                 |
| `HiddenInput` | `[data-ars-scope="segment-group"][data-ars-part="hidden-input"]` | `<input type="hidden">` |

```diagram
┌─ Root (div, role="radiogroup") ─────────────────────┐
│ ┌─ Item (button, role="radio") ─┐  ┌─ Item ──┐      │
│ │ ┌─ ItemText ─┐                │  │  ...    │      │
│ │ │  "Grid"    │                │  │         │      │
│ │ └────────────┘                │  │         │      │
│ └───────────────────────────────┘  └─────────┘      │
│ ┌─ Indicator (div, aria-hidden) ────────────────┐   │
│ │  (animated sliding highlight behind selected) │   │
│ └───────────────────────────────────────────────┘   │
│ ┌─ HiddenInput (input type="hidden") ───────────┐   │
│ └───────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part | Role         | Key Attributes                                       |
| ---- | ------------ | ---------------------------------------------------- |
| Root | `radiogroup` | `aria-orientation`                                   |
| Item | `radio`      | `aria-checked`, `aria-disabled`, `tabindex` (roving) |

- The `Indicator` part is `aria-hidden="true"` — it is purely decorative.
- Each `Item` receives `aria-checked="true"` when selected, `"false"` otherwise.
- `aria-orientation` is set on the Root to `"horizontal"` or `"vertical"` matching the `orientation` prop.

### 3.2 Keyboard Interaction

| Key                    | Action                                                 |
| ---------------------- | ------------------------------------------------------ |
| ArrowRight / ArrowLeft | Focus next/previous enabled segment in horizontal mode |
| ArrowDown / ArrowUp    | Focus next/previous enabled segment in vertical mode   |
| Home                   | Focus first enabled segment                            |
| End                    | Focus last enabled segment                             |
| Space / Enter          | Select focused segment                                 |
| Tab                    | Move focus into/out of group                           |

**Roving tabindex**: The selected segment (or the first enabled segment if none is selected) receives `tabindex="0"`. All other segments receive `tabindex="-1"`. This ensures Tab enters the group on the selected item and the user navigates within the group using arrow keys.

**RTL**: In horizontal orientation with `dir: Rtl`, ArrowLeft and ArrowRight swap semantic meaning (ArrowLeft → next, ArrowRight → previous).

**Focus follows selection**: Arrow key navigation moves focus but does not automatically select. The user presses Space or Enter (or clicks) to select. This matches the WAI-ARIA radio group pattern where focus and selection are separate concerns.

## 4. Internationalization

- In RTL mode (`dir: Rtl`), ArrowLeft/ArrowRight reverse for horizontal orientation.
- Segment labels are consumer-provided and localized by the consumer.

### 4.1 Messages

```rust
/// Translatable messages for SegmentGroup.
#[derive(Clone, Debug)]
pub struct Messages {
    // No component-generated text — all labels are consumer-provided.
    // Struct exists for pattern conformance and future extension.
}

impl Default for Messages {
    fn default() -> Self {
        Self {}
    }
}

impl ComponentMessages for Messages {}
```

## 5. Indicator Part

The `Indicator` part provides an animated sliding selection highlight. It is positioned via CSS custom properties set by the adapter based on the selected item's DOM measurements.

**CSS Custom Properties** (set as inline styles on the indicator element):

| Property                             | Description                                                             |
| ------------------------------------ | ----------------------------------------------------------------------- |
| `--ars-indicator-inset-inline-start` | Inline-start offset of the indicator (uses `LogicalRect.inline_start`). |
| `--ars-indicator-top`                | Vertical offset of the indicator from the group root.                   |
| `--ars-indicator-width`              | Width of the indicator (matches the selected item).                     |
| `--ars-indicator-height`             | Height of the indicator (matches the selected item).                    |

> **SSR behaviour:** During SSR, render the indicator element with `display: none` inline style. On hydration, the adapter measures item positions and replaces the inline style with CSS custom properties.
>
> **Dioxus Desktop note:** Indicator positioning relies on `getBoundingClientRect()` which is web-only. Desktop adapters should either omit the indicator or use a CSS-only highlight approach (e.g., background color on `[data-ars-state="checked"]`) instead of absolute positioning.

## 6. Forced Colors / High Contrast

In Windows High Contrast Mode (`@media (forced-colors: active)`), the indicator part may become invisible if it relies solely on `background-color`. Adapters MUST provide a `forced-colors` fallback — typically a 2px `Highlight` border on the selected item — so the active segment remains distinguishable.

## 7. Library Parity

> Compared against: Ark UI (`SegmentGroup`).

### 7.1 Props

| Feature                       | ars-ui                    | Ark UI                   | Notes                                         |
| ----------------------------- | ------------------------- | ------------------------ | --------------------------------------------- |
| Controlled/uncontrolled value | `value` / `default_value` | `value` / `defaultValue` | --                                            |
| Disabled                      | `disabled`                | `disabled`               | --                                            |
| Read-only                     | `readonly`                | `readOnly`               | --                                            |
| Invalid                       | `invalid`                 | `invalid`                | --                                            |
| Orientation                   | `orientation`             | `orientation`            | --                                            |
| Name (form)                   | `name`                    | `name`                   | --                                            |
| Form ID                       | `form`                    | `form`                   | --                                            |
| Direction (RTL)               | `dir`                     | --                       | ars-ui explicit; Ark UI inherits from context |
| Loop focus                    | `loop_focus`              | --                       | ars-ui exclusive                              |
| Required                      | --                        | `required`               | Ark UI has it; ars-ui does not                |
| On value change               | `on_value_change`         | `onValueChange`          | --                                            |

**Gaps:** `required` prop is present in Ark UI but missing from ars-ui SegmentGroup. However, SegmentGroup always has a selected value (it is semantically a RadioGroup where one option is always active), making `required` redundant in practice. No action needed.

### 7.2 Anatomy

| Part        | ars-ui        | Ark UI        | Notes                        |
| ----------- | ------------- | ------------- | ---------------------------- |
| Root        | `Root`        | `Root`        | --                           |
| Indicator   | `Indicator`   | `Indicator`   | Animated selection highlight |
| Item        | `Item`        | `Item`        | --                           |
| ItemText    | `ItemText`    | `ItemText`    | --                           |
| HiddenInput | `HiddenInput` | `HiddenInput` | Form submission              |

**Gaps:** None.

### 7.3 Events

| Callback     | ars-ui            | Ark UI          | Notes |
| ------------ | ----------------- | --------------- | ----- |
| Value change | `on_value_change` | `onValueChange` | --    |

**Gaps:** None.

### 7.4 Features

| Feature                     | ars-ui                      | Ark UI              |
| --------------------------- | --------------------------- | ------------------- |
| Single selection            | Yes                         | Yes                 |
| Animated indicator          | Yes (CSS custom properties) | Yes (CSS variables) |
| Keyboard navigation         | Yes                         | Yes                 |
| Per-item disabled           | Yes                         | Yes                 |
| Orientation (h/v)           | Yes                         | Yes                 |
| RTL support                 | Yes                         | Yes                 |
| Form integration            | Yes                         | Yes                 |
| Forced colors/high contrast | Yes                         | --                  |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses logical CSS property names (`--ars-indicator-inset-inline-start`) for RTL-aware positioning; Ark UI uses physical names (`--left`, `--top`); (2) ars-ui has explicit forced-colors / high contrast guidance.
- **Recommended additions:** None.
