---
component: RadioGroup
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
  ark-ui: RadioGroup
  radix-ui: RadioGroup
  react-aria: RadioGroup
---

# RadioGroup

A RadioGroup lets the user select exactly one value from a set of options. It manages a
group-level machine plus per-item rendering via the connect API.

## 1. State Machine

### 1.1 States

```rust
/// The state of the RadioGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused {
        /// The value of the focused item.
        item: Key,
    },
}
```

### 1.2 Events

```rust
/// The events for the RadioGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Select a radio item by value.
    SelectValue(Key),
    /// Focus moved to a specific item.
    FocusItem {
        /// The value of the focused item.
        item: Key,
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus left the group.
    Blur,
    /// Move focus to the next item (wraps).
    FocusNext,
    /// Move focus to the previous item (wraps).
    FocusPrev,
    /// Focus the first item.
    FocusFirst,
    /// Focus the last item.
    FocusLast,
}
```

### 1.3 Context

```rust
/// The context of the RadioGroup component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The selected value — controlled or uncontrolled.
    pub value: Bindable<Option<Key>>,
    /// The value of the focused item.
    pub focused_item: Option<Key>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The orientation of the RadioGroup component.
    pub orientation: Orientation,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// Whether the focus should loop.
    pub loop_focus: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Ordered list of item values for navigation.
    pub items: Vec<Radio>,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The definition of a radio item.
#[derive(Clone, Debug, PartialEq)]
pub struct Radio {
    /// The value of the radio item.
    pub value: Key,
    /// Whether the radio item is disabled.
    pub disabled: bool,
}
```

### 1.4 Props

```rust
use ars_i18n::{Orientation, Direction};

/// The props for the RadioGroup component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the RadioGroup component.
    pub id: String,
    /// Controlled value. When Some, component is controlled.
    pub value: Option<Key>,
    /// Default value for uncontrolled mode.
    pub default_value: Option<Key>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// Layout orientation. Affects keyboard navigation:
    /// - `Horizontal`: Left/Right arrows move between options.
    /// - `Vertical` (default): Up/Down arrows move between options.
    pub orientation: Orientation,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether the focus should loop.
    pub loop_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            disabled: false,
            readonly: false,
            required: false,
            invalid: false,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            name: None,
            form: None,
            loop_focus: true,
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
/// Machine for the RadioGroup component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props) -> (Self::State, Self::Context) {
        let state = State::Idle;
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(Some(v.clone())),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            focused_item: None,
            focus_visible: false,
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            invalid: props.invalid,
            orientation: props.orientation,
            dir: props.dir,
            name: props.name.clone(),
            loop_focus: props.loop_focus,
            has_description: false,
            items: Vec::new(),
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
                Event::SelectValue(_) => return None,
                _ => {}
            }
        }

        match event {
            Event::SelectValue(val) => {
                // Skip if item is disabled
                if ctx.items.iter().any(|i| i.value == *val && i.disabled) {
                    return None;
                }
                // Skip when same value is already selected
                if ctx.value.get().as_ref() == Some(val) {
                    return None;
                }
                let val = val.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(Some(val));
                }))
            }

            Event::FocusItem { item, is_keyboard } => {
                let item_clone = item.clone();
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused {
                    item: item.clone(),
                }).apply(move |ctx| {
                    ctx.focused_item = Some(item_clone);
                    ctx.focus_visible = is_kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused_item = None;
                    ctx.focus_visible = false;
                }))
            }

            Event::FocusNext => {
                let next = navigate_items(&ctx.items, &ctx.focused_item, 1, ctx.loop_focus);
                if let Some(val) = next {
                    let val_clone = val.clone();
                    Some(TransitionPlan::to(State::Focused { item: val }).apply(move |ctx| {
                        ctx.focused_item = Some(val_clone);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(key) = &ctx.focused_item {
                            let platform = use_platform_effects();
                            let item_id = ctx.ids.item("item", key);
                            platform.focus_element_by_id(&item_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            Event::FocusPrev => {
                let prev = navigate_items(&ctx.items, &ctx.focused_item, -1, ctx.loop_focus);
                if let Some(val) = prev {
                    let val_clone = val.clone();
                    Some(TransitionPlan::to(State::Focused { item: val }).apply(move |ctx| {
                        ctx.focused_item = Some(val_clone);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(key) = &ctx.focused_item {
                            let platform = use_platform_effects();
                            let item_id = ctx.ids.item("item", &key);
                            platform.focus_element_by_id(&item_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            Event::FocusFirst => {
                let first = ctx.items.iter().find(|i| !i.disabled).map(|i| i.value.clone());
                if let Some(val) = first {
                    let val_clone = val.clone();
                    Some(TransitionPlan::to(State::Focused { item: val }).apply(move |ctx| {
                        ctx.focused_item = Some(val_clone);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(first) = ctx.items.iter().find(|i| !i.disabled) {
                            let platform = use_platform_effects();
                            let item_id = ctx.ids.item("item", &first.value);
                            platform.focus_element_by_id(&item_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            Event::FocusLast => {
                let last = ctx.items.iter().rev().find(|i| !i.disabled).map(|i| i.value.clone());
                if let Some(val) = last {
                    let val_clone = val.clone();
                    Some(TransitionPlan::to(State::Focused { item: val }).apply(move |ctx| {
                        ctx.focused_item = Some(val_clone);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(last) = ctx.items.iter().rev().find(|i| !i.disabled) {
                            let platform = use_platform_effects();
                            let item_id = ctx.ids.item("item", &last.value);
                            platform.focus_element_by_id(&item_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
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

/// Navigate forward/backward through non-disabled items.
fn navigate_items(
    items: &[Radio],
    current: &Option<Key>,
    direction: i32,
    wrap: bool,
) -> Option<Key> {
    let enabled: Vec<&Radio> = items.iter().filter(|i| !i.disabled).collect();
    if enabled.is_empty() { return None; }

    let current_idx = current.as_ref()
        .and_then(|c| enabled.iter().position(|i| &i.value == c));

    let next_idx = match current_idx {
        Some(idx) => {
            let new = idx as i32 + direction;
            if wrap {
                Some(new.rem_euclid(enabled.len() as i32) as usize)
            } else if new >= 0 && (new as usize) < enabled.len() {
                Some(new as usize)
            } else {
                None
            }
        }
        None => Some(if direction > 0 { 0 } else { enabled.len() - 1 }),
    };

    next_idx.map(|i| enabled[i].value.clone())
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "radio-group"]
pub enum Part {
    Root,
    Label,
    Description,
    ErrorMessage,
    Item { item_value: Key },
    ItemControl { item_value: Key },
    ItemIndicator { item_value: Key },
    ItemLabel { item_value: Key },
    ItemHiddenInput { item_value: Key },
}

/// API for the RadioGroup component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Look up whether an item is disabled (from context items or group disabled).
    fn is_item_disabled(&self, item_value: &Key) -> bool {
        self.ctx.disabled
            || self.ctx.items.iter().any(|i| &i.value == item_value && i.disabled)
    }

    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "radiogroup");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match &self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
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

    /// Attributes for a single radio item container.
    pub fn item_attrs(&self, item_value: &Key) -> AttrMap {
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        let is_focused = self.ctx.focused_item.as_ref() == Some(item_value);
        let is_disabled = self.is_item_disabled(item_value);

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { item_value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "checked" } else { "unchecked" });
        if is_disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.focus_visible && is_focused { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    /// Attributes for the interactive radio control (receives focus).
    pub fn item_control_attrs(&self, item_value: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", item_value);
        let label_id = self.ctx.ids.item_part("item", item_value, "label");
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        let is_disabled = self.is_item_disabled(item_value);

        // Roving tabindex: only the selected item (or first if none selected) gets tabindex=0
        let is_tabbable = if self.ctx.value.get().is_some() {
            is_selected
        } else {
            self.ctx.items.first().map(|i| &i.value) == Some(item_value)
        };

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemControl { item_value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "radio");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if is_selected { "true" } else { "false" });
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), label_id);
        attrs.set(HtmlAttr::TabIndex, if is_tabbable { "0" } else { "-1" });
        attrs
    }

    /// Attributes for the visual radio indicator.
    pub fn item_indicator_attrs(&self, item_value: &Key) -> AttrMap {
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator { item_value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "checked" } else { "unchecked" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the radio item label.
    pub fn item_label_attrs(&self, item_value: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", &item_value);
        let label_id = self.ctx.ids.item_part("item", item_value, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemLabel { item_value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, label_id);
        attrs.set(HtmlAttr::For, item_id);
        attrs
    }

    /// Attributes for the hidden radio input (form submission).
    pub fn item_hidden_input_attrs(&self, item_value: &Key) -> AttrMap {
        let is_selected = self.ctx.value.get().as_ref() == Some(item_value);
        let is_disabled = self.is_item_disabled(item_value);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemHiddenInput { item_value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "radio");
        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::Value, item_value.to_string());
        if is_selected { attrs.set_bool(HtmlAttr::Checked, true); }
        if is_disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Required, true); }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        match data.key {
            KeyboardKey::ArrowRight => {
                if is_horizontal && is_rtl { (self.send)(Event::FocusPrev) }
                else { (self.send)(Event::FocusNext) }
            }
            KeyboardKey::ArrowLeft => {
                if is_horizontal && is_rtl { (self.send)(Event::FocusNext) }
                else { (self.send)(Event::FocusPrev) }
            }
            KeyboardKey::ArrowDown => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            _ => {}
        }
    }

    pub fn on_item_control_click(&self, item_value: &Key) {
        (self.send)(Event::SelectValue(item_value.clone()));
    }

    pub fn on_item_control_focus(&self, item_value: &Key, is_keyboard: bool) {
        (self.send)(Event::FocusItem { item: item_value.clone(), is_keyboard });
    }

    pub fn on_item_control_blur(&self) { (self.send)(Event::Blur); }

    pub fn on_item_control_keydown(&self, item_value: &Key, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Space || data.key == KeyboardKey::Enter {
            (self.send)(Event::SelectValue(item_value.clone()));
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::Item { item_value } => self.item_attrs(&item_value),
            Part::ItemControl { item_value } => self.item_control_attrs(&item_value),
            Part::ItemIndicator { item_value } => self.item_indicator_attrs(&item_value),
            Part::ItemLabel { item_value } => self.item_label_attrs(&item_value),
            Part::ItemHiddenInput { item_value } => self.item_hidden_input_attrs(&item_value),
        }
    }
}
```

## 2. Anatomy

```text
RadioGroup
├── Root               <div>    data-ars-scope="radio-group" data-ars-part="root" (role="radiogroup")
├── Label              <label>  data-ars-part="label"
├── Item (×N)          <div>    data-ars-part="item"
│   ├── ItemControl    <div>    data-ars-part="item-control" (role="radio")
│   ├── ItemIndicator  <div>    data-ars-part="item-indicator" (aria-hidden)
│   ├── ItemLabel      <label>  data-ars-part="item-label"
│   └── ItemHiddenInput <input> data-ars-part="item-hidden-input" (type="radio", aria-hidden)
├── Description        <div>    data-ars-part="description" (optional)
└── ErrorMessage       <div>    data-ars-part="error-message" (optional)
```

**Group-level parts:**

| Part         | Element   | Key Attributes                                             |
| ------------ | --------- | ---------------------------------------------------------- |
| Root         | `<div>`   | `role="radiogroup"`, `aria-orientation`, `aria-labelledby` |
| Label        | `<label>` | Group label                                                |
| Description  | `<div>`   | Help text; linked via `aria-describedby` (optional)        |
| ErrorMessage | `<div>`   | Validation error; linked via `aria-describedby` (optional) |

**Per-item parts (repeated):**

| Part            | Element   | Key Attributes                                         |
| --------------- | --------- | ------------------------------------------------------ |
| Item            | `<div>`   | `data-ars-state` ("checked"/"unchecked")               |
| ItemControl     | `<div>`   | `role="radio"`, `aria-checked`, roving `tabindex`      |
| ItemIndicator   | `<div>`   | `aria-hidden="true"` — visual radio dot                |
| ItemLabel       | `<label>` | `for` points to ItemControl                            |
| ItemHiddenInput | `<input>` | `type="radio"`, `aria-hidden="true"` — form submission |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element     | Value                                      |
| ------------------ | ----------- | ------------------------------------------ |
| `role`             | Root        | `radiogroup`                               |
| `aria-orientation` | Root        | `"horizontal"` or `"vertical"`             |
| `aria-required`    | Root        | Present when `required=true`               |
| `aria-invalid`     | Root        | Present when `invalid=true`                |
| `aria-labelledby`  | Root        | Points to Label id                         |
| `aria-describedby` | Root        | Points to Description + ErrorMessage ids   |
| `role`             | ItemControl | `radio`                                    |
| `aria-checked`     | ItemControl | `"true"` or `"false"`                      |
| `aria-disabled`    | ItemControl | Present when item or group is disabled     |
| `tabindex`         | ItemControl | Roving: selected item `0`, all others `-1` |

The `aria-orientation` attribute informs assistive technology whether navigation uses horizontal or vertical arrow keys.

### 3.2 Keyboard Interaction

| Key                    | Action                         |
| ---------------------- | ------------------------------ |
| ArrowDown / ArrowRight | Focus and select next item     |
| ArrowUp / ArrowLeft    | Focus and select previous item |
| Home                   | Focus and select first item    |
| End                    | Focus and select last item     |
| Space / Enter          | Select focused item            |
| Tab                    | Move focus into/out of group   |

> RTL: ArrowLeft/ArrowRight swap semantic meaning for horizontal orientation per `03-accessibility.md` §4.1.

### 3.3 Focus Management

- Roving tabindex: only the selected item (or first if none selected) has `tabindex="0"`.
- Arrow keys cycle focus through enabled items; wraps when `loop_focus` is enabled.
- Focus moves programmatically via `platform.focus_element_by_id()` (see `PlatformEffects` trait in `01-architecture.md` section 2.2.7).

## 4. Internationalization

- In RTL mode, ArrowLeft/ArrowRight swap semantic meaning for horizontal orientation.
- Group and item labels are user-provided and localized by the consumer.
- "Required" indicator text uses the i18n message catalog.

> `SegmentGroup` shares identical ARIA semantics (`role="radiogroup"` + `role="radio"`) but with a visually connected, segmented control appearance and animated selection indicator. See `components/selection/segment-group.md` for the full specification.

## 5. Form Integration

- **Hidden inputs**: Each radio item renders a hidden `<input type="radio">` via `ItemHiddenInput`. Only the selected item has `checked`. All share the same `name` attribute.
- **Validation states**: `aria-invalid="true"` on Root when `invalid=true`. The `ErrorMessage` part is linked via `aria-describedby`.
- **Error message association**: `aria-describedby` on Root points to `Description` (when present) and `ErrorMessage` (when invalid).
- **Required**: `aria-required="true"` on Root and each ItemControl. Hidden inputs carry the `required` attribute.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Library Parity

> Compared against: Ark UI (`RadioGroup`), Radix UI (`RadioGroup`), React Aria (`RadioGroup`).

### 6.1 Props

| Feature          | ars-ui                       | Ark UI         | Radix UI       | React Aria     | Notes                      |
| ---------------- | ---------------------------- | -------------- | -------------- | -------------- | -------------------------- |
| Controlled value | `value: Option<Key>`         | `value`        | `value`        | `value`        | Full parity                |
| Default value    | `default_value: Option<Key>` | `defaultValue` | `defaultValue` | `defaultValue` | Full parity                |
| Disabled         | `disabled: bool`             | `disabled`     | `disabled`     | `isDisabled`   | Full parity                |
| Read-only        | `readonly: bool`             | `readOnly`     | --             | `isReadOnly`   | Ark+RA parity; Radix lacks |
| Required         | `required: bool`             | `required`     | `required`     | `isRequired`   | Full parity                |
| Invalid          | `invalid: bool`              | `invalid`      | --             | `isInvalid`    | Ark+RA parity; Radix lacks |
| Form name        | `name: Option<String>`       | `name`         | `name`         | `name`         | Full parity                |
| Form ID          | `form: Option<String>`       | `form`         | --             | --             | Ark parity                 |
| Orientation      | `orientation: Orientation`   | `orientation`  | `orientation`  | `orientation`  | Full parity                |
| Direction        | `dir: Direction`             | --             | `dir`          | --             | Radix parity               |
| Loop focus       | `loop_focus: bool`           | --             | `loop`         | --             | Radix parity               |

**Gaps:** None.

### 6.2 Anatomy

| Part            | ars-ui            | Ark UI                    | Radix UI    | React Aria          | Notes                      |
| --------------- | ----------------- | ------------------------- | ----------- | ------------------- | -------------------------- |
| Root            | `Root`            | `Root`                    | `Root`      | `RadioGroup`        | Full parity                |
| Label           | `Label`           | `Label`                   | --          | `Label`             | Full parity                |
| Item            | `Item`            | `Item`                    | `Item`      | `Radio`             | Full parity                |
| ItemControl     | `ItemControl`     | `ItemControl`             | --          | --                  | ars-ui interactive element |
| ItemIndicator   | `ItemIndicator`   | `Indicator` (group-level) | `Indicator` | --                  | Full parity                |
| ItemLabel       | `ItemLabel`       | `ItemText`                | --          | --                  | Full parity with Ark       |
| ItemHiddenInput | `ItemHiddenInput` | `ItemHiddenInput`         | (built-in)  | (built-in)          | Full parity                |
| Description     | `Description`     | --                        | --          | `Text[description]` | ars-ui form-field part     |
| ErrorMessage    | `ErrorMessage`    | --                        | --          | `FieldError`        | ars-ui form-field part     |

**Gaps:** None.

### 6.3 Events

| Callback         | ars-ui                                           | Ark UI          | Radix UI        | React Aria | Notes                    |
| ---------------- | ------------------------------------------------ | --------------- | --------------- | ---------- | ------------------------ |
| Value changed    | `SelectValue(Key)`                               | `onValueChange` | `onValueChange` | `onChange` | Full parity              |
| Focus navigation | `FocusNext`/`FocusPrev`/`FocusFirst`/`FocusLast` | --              | --              | --         | ars-ui keyboard handling |

**Gaps:** None.

### 6.4 Features

| Feature             | ars-ui | Ark UI | Radix UI | React Aria |
| ------------------- | ------ | ------ | -------- | ---------- |
| Roving tabindex     | Yes    | Yes    | Yes      | Yes        |
| Keyboard navigation | Yes    | Yes    | Yes      | Yes        |
| Per-item disabled   | Yes    | Yes    | Yes      | Yes        |
| Form integration    | Yes    | Yes    | Built-in | Built-in   |
| RTL support         | Yes    | --     | Yes      | --         |
| Loop focus          | Yes    | --     | Yes      | --         |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity across all three reference libraries.
- **Divergences:** ars-ui uses `Key` enum for item values (type-safe) instead of raw strings. ars-ui includes built-in roving tabindex implementation with explicit focus events. Ark UI has a group-level animated `Indicator` part; ars-ui uses per-item `ItemIndicator`.
- **Recommended additions:** None.
