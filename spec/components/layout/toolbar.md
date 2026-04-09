---
component: Toolbar
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
  radix-ui: Toolbar
  react-aria: Toolbar
---

# Toolbar

`Toolbar` groups a set of interactive controls (buttons, toggles, menus) into a single container with managed keyboard navigation following the WAI-ARIA Toolbar pattern. Uses roving tabindex for focus management with wrapping navigation and disabled item skipping.

## 1. State Machine

### 1.1 States

```rust
/// Toolbar is always in a single state; focus tracking is managed via context.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The idle state.
    Idle,
}
```

### 1.2 Events

```rust
/// The events for the Toolbar component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus a specific item by index.
    FocusItem(usize),
    /// Move focus to the next enabled item (wraps).
    FocusNext,
    /// Move focus to the previous enabled item (wraps).
    FocusPrev,
    /// Move focus to the first enabled item.
    FocusFirst,
    /// Move focus to the last enabled item.
    FocusLast,
    /// Focus entered the toolbar.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus left the toolbar.
    Blur,
}
```

### 1.3 Context

```rust
/// The context for the Toolbar component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Index of the currently focused item (roving tabindex target).
    /// `None` when no item has been focused yet (first Tab into toolbar
    /// focuses the first enabled item).
    pub focused_index: Option<usize>,
    /// Toolbar orientation.
    pub orientation: Orientation,
    /// Text direction for RTL-aware arrow key navigation.
    pub dir: Direction,
    /// Whether the toolbar is disabled (disables all child items).
    pub disabled: bool,
    /// Total number of items (set by adapter during render).
    pub item_count: usize,
    /// Set of disabled item indices.
    pub disabled_items: Vec<usize>,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
use ars_i18n::{Orientation, Direction};

/// Props for `Toolbar`.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the toolbar.
    pub id: String,
    /// Toolbar orientation. Determines which arrow keys navigate between items.
    /// `Horizontal` (default): ArrowLeft/ArrowRight. `Vertical`: ArrowUp/ArrowDown.
    pub orientation: Orientation,
    /// Text direction for RTL support.
    pub dir: Direction,
    /// Accessible label for the toolbar.
    pub aria_label: Option<String>,
    /// Whether the toolbar is disabled. When `true`, all child items are disabled
    /// and the toolbar ignores all navigation events.
    pub disabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            id: String::new(),
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            aria_label: None,
            disabled: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
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
        let ctx = Context {
            focused_index: None,
            orientation: props.orientation,
            dir: props.dir,
            disabled: props.disabled,
            item_count: 0,
            disabled_items: Vec::new(),
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            Event::FocusItem(index) => {
                let idx = *index;
                if idx >= ctx.item_count || ctx.disabled_items.contains(&idx) {
                    return None;
                }
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = Some(idx);
                }))
            }
            Event::FocusNext => {
                let next = next_enabled_index(
                    ctx.focused_index.unwrap_or(0),
                    ctx.item_count,
                    &ctx.disabled_items,
                    true,
                );
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = next;
                }))
            }
            Event::FocusPrev => {
                let prev = next_enabled_index(
                    ctx.focused_index.unwrap_or(0),
                    ctx.item_count,
                    &ctx.disabled_items,
                    false,
                );
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = prev;
                }))
            }
            Event::FocusFirst => {
                let first = first_enabled_index(ctx.item_count, &ctx.disabled_items);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = first;
                }))
            }
            Event::FocusLast => {
                let last = last_enabled_index(ctx.item_count, &ctx.disabled_items);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = last;
                }))
            }
            Event::Focus { .. } => {
                if ctx.focused_index.is_some() { return None; }
                let first = first_enabled_index(ctx.item_count, &ctx.disabled_items);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_index = first;
                }))
            }
            Event::Blur => {
                // Preserve focused_index so re-entering toolbar restores position.
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

/// Find the next enabled index, wrapping around.
fn next_enabled_index(
    current: usize, count: usize, disabled: &[usize], forward: bool,
) -> Option<usize> {
    if count == 0 { return None; }
    for i in 1..count {
        let idx = if forward {
            (current + i) % count
        } else {
            (current + count - i) % count
        };
        if !disabled.contains(&idx) { return Some(idx); }
    }
    None
}

fn first_enabled_index(count: usize, disabled: &[usize]) -> Option<usize> {
    (0..count).find(|i| !disabled.contains(i))
}

fn last_enabled_index(count: usize, disabled: &[usize]) -> Option<usize> {
    (0..count).rev().find(|i| !disabled.contains(i))
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "toolbar"]
pub enum Part {
    Root,
    Item { index: usize },
    Separator,
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
        attrs.set(HtmlAttr::Role, "toolbar");
        attrs.set(
            HtmlAttr::Aria(AriaAttr::Orientation),
            match self.ctx.orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            },
        );
        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());
        attrs
    }

    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { index: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_focused = self.ctx.focused_index == Some(index);
        let is_disabled = self.ctx.disabled || self.ctx.disabled_items.contains(&index);
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if is_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        attrs
    }

    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "separator");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation),
            match self.ctx.orientation {
                Orientation::Horizontal => "vertical",
                Orientation::Vertical => "horizontal",
            },
        );
        attrs
    }

    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let is_rtl = self.ctx.dir == Direction::Rtl;
        match data.key {
            KeyboardKey::ArrowRight if is_horizontal => {
                if is_rtl { (self.send)(Event::FocusPrev) }
                else { (self.send)(Event::FocusNext) }
            }
            KeyboardKey::ArrowLeft if is_horizontal => {
                if is_rtl { (self.send)(Event::FocusNext) }
                else { (self.send)(Event::FocusPrev) }
            }
            KeyboardKey::ArrowDown if !is_horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp if !is_horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            _ => {}
        }
    }

    pub fn on_item_focus(&self, index: usize, is_keyboard: bool) {
        (self.send)(Event::FocusItem(index));
    }

    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::Separator => self.separator_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Toolbar
├── Root       <div>     role="toolbar" aria-orientation
│   ├── Item   (×N)      tabindex="0|-1" (roving)
│   ├── Separator         role="separator" aria-orientation (perpendicular)
│   └── Item   (×N)      tabindex="0|-1"
```

| Part      | Element | Key Attributes                                             |
| --------- | ------- | ---------------------------------------------------------- |
| Root      | `<div>` | `role="toolbar"`, `aria-orientation`, `aria-label`         |
| Item      | varies  | `tabindex="0\|-1"` (roving), `aria-disabled` when disabled |
| Separator | `<div>` | `role="separator"`, `aria-orientation` (perpendicular)     |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element   | Value                                     |
| ------------------ | --------- | ----------------------------------------- |
| `role`             | Root      | `"toolbar"`                               |
| `aria-orientation` | Root      | `"horizontal"` or `"vertical"`            |
| `aria-label`       | Root      | User-provided label                       |
| `aria-disabled`    | Root      | Present when toolbar is disabled          |
| `tabindex`         | Item      | `"0"` for focused item, `"-1"` for others |
| `aria-disabled`    | Item      | Present when item or toolbar is disabled  |
| `role`             | Separator | `"separator"`                             |
| `aria-orientation` | Separator | Perpendicular to toolbar orientation      |

### 3.2 Keyboard Interaction

| Key          | Horizontal (LTR)          | Horizontal (RTL)          | Vertical                  |
| ------------ | ------------------------- | ------------------------- | ------------------------- |
| `ArrowRight` | Focus next item           | Focus previous item       | --                        |
| `ArrowLeft`  | Focus previous item       | Focus next item           | --                        |
| `ArrowDown`  | --                        | --                        | Focus next item           |
| `ArrowUp`    | --                        | --                        | Focus previous item       |
| `Home`       | Focus first item          | Focus first item          | Focus first item          |
| `End`        | Focus last item           | Focus last item           | Focus last item           |
| `Tab`        | Move focus out of toolbar | Move focus out of toolbar | Move focus out of toolbar |

Focus wraps from last to first and vice versa. Disabled items are skipped during keyboard navigation.

RTL: In horizontal orientation with `dir="rtl"`, ArrowRight and ArrowLeft are reversed per `03-accessibility.md` §4.1.

## 4. Internationalization

- The `aria-label` prop is consumer-provided and must be localized by the consumer.
- `data-ars-state` values and part names are stable API tokens, not localized.
- RTL arrow key reversal is handled automatically by the keyboard handler.

## 5. Library Parity

> Compared against: Radix UI (`Toolbar`), React Aria (`Toolbar`).

### 5.1 Props

| Feature          | ars-ui        | Radix UI              | React Aria    | Notes                                        |
| ---------------- | ------------- | --------------------- | ------------- | -------------------------------------------- |
| Orientation      | `orientation` | `orientation`         | `orientation` | Same                                         |
| Direction (RTL)  | `dir`         | `dir`                 | --            | React Aria infers from context               |
| Loop navigation  | Always wraps  | `loop` (default true) | --            | ars-ui always wraps; same effective behavior |
| Disabled         | `disabled`    | --                    | --            | ars-ui addition                              |
| Accessible label | `aria_label`  | --                    | --            | ars-ui addition                              |

**Gaps:** None.

### 5.2 Anatomy

| Part        | ars-ui      | Radix UI                       | React Aria        | Notes                                                         |
| ----------- | ----------- | ------------------------------ | ----------------- | ------------------------------------------------------------- |
| Root        | `Root`      | `Root`                         | Toolbar container | --                                                            |
| Item        | `Item`      | `Button`, `Link`, `ToggleItem` | --                | Radix types items; ars-ui uses generic Item                   |
| Separator   | `Separator` | `Separator`                    | --                | --                                                            |
| ToggleGroup | --          | `ToggleGroup`                  | --                | Radix embeds toggle group; ars-ui uses standalone ToggleGroup |

**Gaps:** None. Radix's `ToggleGroup`/`ToggleItem` within Toolbar are the standalone ToggleGroup component embedded as toolbar items. ars-ui achieves the same by nesting a ToggleGroup inside the Toolbar. Radix's `Button` and `Link` are thin wrappers that just forward `asChild`; ars-ui's generic `Item` serves the same purpose.

### 5.3 Events

| Callback         | ars-ui                      | Radix UI | React Aria | Notes                |
| ---------------- | --------------------------- | -------- | ---------- | -------------------- |
| Focus navigation | `FocusNext/Prev/First/Last` | --       | --         | State machine events |

**Gaps:** None. Both references handle focus internally.

### 5.4 Features

| Feature              | ars-ui | Radix UI | React Aria |
| -------------------- | ------ | -------- | ---------- |
| Roving tabindex      | Yes    | Yes      | Yes        |
| Arrow key navigation | Yes    | Yes      | Yes        |
| Home/End support     | Yes    | --       | --         |
| RTL reversal         | Yes    | Yes      | --         |
| Disabled items skip  | Yes    | --       | --         |
| Disabled toolbar     | Yes    | --       | --         |
| Separator support    | Yes    | Yes      | --         |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Radix provides typed item variants (`Button`, `Link`, `ToggleItem`); ars-ui uses a generic `Item` part that the consumer renders with the correct HTML element and semantics. ars-ui adds `disabled` prop for the entire toolbar and `Home`/`End` key support.
- **Recommended additions:** None.
