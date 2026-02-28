---
component: ActionGroup
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions, i18n]
shared_deps: []
related: [toggle-group]
references:
  react-aria: ActionGroup
---

# ActionGroup

`ActionGroup` is a toolbar container that manages a group of action buttons with keyboard navigation and optional overflow-to-menu behavior for responsive layouts.

## 1. State Machine

### 1.1 States

| State                   | Description                                             |
| ----------------------- | ------------------------------------------------------- |
| `Idle`                  | Default resting state. No item is focused via keyboard. |
| `Focused { item: Key }` | An item within the action group has keyboard focus.     |

### 1.2 Events

| Event             | Payload | Description                                                                                                                                                              |
| ----------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `FocusItem`       | `Key`   | Focus a specific item by key.                                                                                                                                            |
| `Blur`            | ---     | Focus left the action group.                                                                                                                                             |
| `FocusNext`       | ---     | Move focus to the next item (wraps around).                                                                                                                              |
| `FocusPrev`       | ---     | Move focus to the previous item (wraps around).                                                                                                                          |
| `FocusFirst`      | ---     | Move focus to the first item.                                                                                                                                            |
| `FocusLast`       | ---     | Move focus to the last item.                                                                                                                                             |
| `ActivateItem`    | `Key`   | Activate the item by key — fires the item's action callback. Does not change selection state; this is for non-toggle "fire and forget" actions (e.g., "Delete", "Copy"). |
| `SelectItem`      | `Key`   | Toggle selection state by key — toggles the item's membership in `selected_items`. Only meaningful when `selection_mode != None`.                                        |
| `OverflowChanged` | `usize` | Number of items that overflowed into the menu.                                                                                                                           |
| `SetProps`        | ---     | Sync context from updated props.                                                                                                                                         |

### 1.3 Context

```rust
use std::collections::BTreeSet;
use ars_core::Key;
use ars_i18n::{Direction, Orientation};

/// The context for the `ActionGroup` component.
#[derive(Clone, Debug)]
pub struct Context {
    /// Whether the action group is disabled.
    pub disabled: bool,
    /// The key of the currently focused item.
    pub focused_item: Option<Key>,
    /// The keys of the currently selected items.
    pub selected_items: BTreeSet<Key>,
    /// The number of items that overflowed into the menu.
    pub overflow_count: usize,
    /// The number of items that are visible in the toolbar.
    pub visible_count: usize,
    /// All registered item keys in DOM/insertion order.
    /// Used for keyboard navigation (FocusNext/Prev/First/Last).
    pub registered_items: Vec<Key>,
    /// Text direction — used to swap ArrowLeft/ArrowRight in horizontal groups.
    pub dir: Direction,
    /// The active locale, inherited from ArsProvider context.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
use ars_i18n::{Direction, Orientation};

/// How text labels are displayed alongside icons in action items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonLabelBehavior {
    /// Always show text labels alongside icons.
    #[default]
    Show,
    /// Collapse text labels to icon-only with tooltips when space is limited, before overflowing.
    Collapse,
    /// Hide text labels entirely — icon-only buttons.
    Hide,
}

/// Props for the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The orientation of the action group.
    pub orientation: Orientation,
    /// Text direction for RTL-aware arrow key navigation.
    pub dir: Direction,
    /// The overflow mode for the action group.
    pub overflow_mode: OverflowMode,
    /// Visual variant for styling hooks.
    pub variant: Variant,
    /// Whether the action group is disabled.
    pub disabled: bool,
    /// Keys of individually disabled items. These items render with
    /// `aria-disabled="true"` and skip event handlers.
    pub disabled_items: BTreeSet<Key>,
    /// The selection mode for the action group.
    pub selection_mode: selection::Mode,
    /// When set, only the first N actions are visible; remaining actions are
    /// placed into an overflow menu triggered by a "..." button.
    /// When `None`, all actions are visible (overflow is determined solely by
    /// `overflow_mode` and container width).
    pub max_visible_actions: Option<usize>,
    /// How text labels are displayed alongside icons.
    pub button_label_behavior: ButtonLabelBehavior,
    /// Density hint exposed as `data-ars-density` for CSS styling hooks.
    pub density: Option<String>,
    /// When true, items stretch to fill the available space equally.
    /// Exposed as `data-ars-justified` on the root element.
    pub justified: bool,
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Accessible label for the toolbar. Either `aria_label` or `aria_labelledby` must be set.
    /// `role="toolbar"` requires an accessible name.
    pub aria_label: Option<String>,
    /// ID of the element that labels this toolbar (alternative to `aria_label`).
    pub aria_labelledby: Option<String>,
    /// Localized labels for accessibility. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

/// Visual variant for `ActionGroup`. Exposed as `data-ars-variant` on the root
/// element for CSS styling hooks. Does not affect behavior or accessibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Variant {
    /// Default toolbar appearance.
    #[default]
    Toolbar,
    /// Outlined button group with visible borders.
    Outlined,
    /// Flat/borderless button group.
    Flat,
}

/// The overflow mode for the action group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverflowMode {
    /// Items wrap to the next line.
    Wrap,
    /// Items that don't fit are hidden (CSS `overflow: hidden`).
    Collapse,
    /// Items that don't fit are moved into an overflow "More" menu.
    Menu,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: Orientation::Horizontal,
            dir: Direction::default(),
            overflow_mode: OverflowMode::Wrap,
            variant: Variant::Toolbar,
            disabled: false,
            disabled_items: BTreeSet::new(),
            selection_mode: selection::Mode::None,
            max_visible_actions: None,
            button_label_behavior: ButtonLabelBehavior::default(),
            density: None,
            justified: false,
            locale: None,
            aria_label: None,
            aria_labelledby: None,
            messages: None,
        }
    }
}
```

### 1.5 Overflow Detection

When `overflow_mode` is `Menu`, the adapter uses a `ResizeObserver` on the Root element. On each resize callback:

1. Measure total available width (or height for vertical).
2. Iterate items, summing widths, until the sum exceeds available space (minus overflow trigger width).
3. Mark remaining items as overflowed — fire `Event::OverflowChanged(count)`.
4. Overflowed items are rendered inside the `OverflowMenu` instead of directly in the toolbar.

Items are removed from the **inline-end** of the toolbar (last items in DOM order). In RTL layouts, inline-end is visually on the left; in LTR, on the right. DOM order is preserved regardless of direction.

When `max_visible_actions` is `Some(n)`, only the first `n` actions are rendered directly in the toolbar. All remaining actions are placed into an overflow menu. The overflow menu uses the existing [`Menu`](../selection/menu.md)/[`Popover`](../overlay/popover.md) component. Items in the overflow menu maintain their original order.

When `max_visible_actions` is `Some(0)`, all actions are placed in the overflow menu (only the overflow trigger is visible).

**Responsive integration**: `max_visible_actions` can be dynamically derived from the container width via `ResizeObserver`. The adapter may recalculate `max_visible_actions` on each resize callback and update the prop, causing the overflow to adjust. This integrates with the existing `OverflowMode::Menu` detection.

> **Platform note (Dioxus Desktop):** `ResizeObserver` is a web-only API. When targeting Dioxus Desktop or other non-web platforms, the adapter must provide a platform-agnostic fallback for overflow detection (e.g., polling layout dimensions on window resize events, or relying solely on `max_visible_actions` for explicit overflow control). The core state machine is platform-agnostic — only the adapter's resize measurement strategy varies.

#### Platform Note: Dioxus Overflow Detection

**Dioxus Web:** Use `ResizeObserver` to detect overflow. The observer MUST be cleaned up via `use_drop`:

```rust
use_effect(move || {
    #[cfg(feature = "web")]
    {
        let observer = ResizeObserver::new(callback);
        observer.observe(&container_el);
        use_drop(move || observer.disconnect());
    }
});
```

**Dioxus Desktop/Mobile:** `ResizeObserver` is unavailable. Use a window resize event listener as fallback:

```rust
#[cfg(not(feature = "web"))]
{
    let listener = window().on_resize(move |_| recalculate_overflow());
    use_drop(move || drop(listener));
}
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, AttrMap};
use std::collections::BTreeSet;

/// The states for the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The action group is in an idle state. No item is focused.
    Idle,
    /// The action group is in a focused state. An item is focused.
    Focused {
        /// The key of the item that has focus.
        item: Key,
    },
}

/// The events for the `ActionGroup` component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Focus a specific item by key.
    FocusItem(Key),
    /// Focus lost from the action group.
    Blur,
    /// Move focus to the next item (wraps around).
    FocusNext,
    /// Move focus to the previous item (wraps around).
    FocusPrev,
    /// Move focus to the first item.
    FocusFirst,
    /// Move focus to the last item.
    FocusLast,
    /// Activate the item by key (fires the item's action callback).
    ActivateItem(Key),
    /// Toggle selection state (only when `selection_mode != None`).
    SelectItem(Key),
    /// The number of items that overflowed into the menu changed.
    OverflowChanged(usize),
    /// Sync context from updated props.
    SetProps,
}

/// The machine for the `ActionGroup` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ctx = Context {
            disabled: props.disabled,
            focused_item: None,
            selected_items: BTreeSet::new(),
            overflow_count: 0,
            visible_count: 0,
            registered_items: Vec::new(),
            dir: props.dir,
            locale,
            messages,
        };

        (State::Idle, ctx)
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled guard: block value-changing events but allow focus/blur,
        // navigation, overflow updates, and prop sync through so screen readers
        // can still discover and traverse the toolbar.
        if ctx.disabled && !matches!(event,
            Event::FocusItem(_) | Event::Blur |
            Event::FocusNext | Event::FocusPrev |
            Event::FocusFirst | Event::FocusLast |
            Event::OverflowChanged(_) | Event::SetProps
        ) {
            return None;
        }

        match (state, event) {
            // ── FocusItem ───────────────────────────────────────────────────
            (_, Event::FocusItem(id)) => {
                let id = id.clone();
                Some(TransitionPlan::to(State::Focused { item: id.clone() })
                    .apply(move |ctx| {
                        ctx.focused_item = Some(id);
                    }))
            }

            // ── Blur ────────────────────────────────────────────────────────
            (State::Focused { .. }, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        ctx.focused_item = None;
                    }))
            }

            // ── FocusNext ───────────────────────────────────────────────────
            (State::Focused { item }, Event::FocusNext) => {
                let items = ctx.registered_items.clone();
                let current = item.clone();
                let next = {
                    let idx = items.iter().position(|k| k == &current);
                    match idx {
                        Some(i) if i + 1 < items.len() => Some(items[i + 1].clone()),
                        Some(_) if !items.is_empty() => Some(items[0].clone()), // wrap around
                        _ => None,
                    }
                };
                let next_for_state = next.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: next_for_state.unwrap_or_else(|| current.clone()),
                }).apply(move |ctx| {
                    if let Some(next_item) = next {
                        ctx.focused_item = Some(next_item);
                    }
                }))
            }

            // ── FocusPrev ───────────────────────────────────────────────────
            (State::Focused { item }, Event::FocusPrev) => {
                let items = ctx.registered_items.clone();
                let current = item.clone();
                let prev = {
                    let idx = items.iter().position(|k| k == &current);
                    match idx {
                        Some(0) if !items.is_empty() => Some(items[items.len() - 1].clone()), // wrap around
                        Some(i) if i > 0 => Some(items[i - 1].clone()),
                        _ => None,
                    }
                };
                let prev_for_state = prev.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: prev_for_state.unwrap_or_else(|| current.clone()),
                }).apply(move |ctx| {
                    if let Some(prev_item) = prev {
                        ctx.focused_item = Some(prev_item);
                    }
                }))
            }

            // ── FocusFirst ──────────────────────────────────────────────────
            (_, Event::FocusFirst) => {
                if ctx.registered_items.is_empty() {
                    return None;
                }
                let items = ctx.registered_items.clone();
                let first = items.first().cloned();
                let first_for_state = first.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: first_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = first;
                }))
            }

            // ── FocusLast ───────────────────────────────────────────────────
            (_, Event::FocusLast) => {
                if ctx.registered_items.is_empty() {
                    return None;
                }
                let items = ctx.registered_items.clone();
                let last = items.last().cloned();
                let last_for_state = last.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: last_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = last;
                }))
            }

            // ── ActivateItem ────────────────────────────────────────────────
            // Notification-only: fires the item's action callback via the adapter.
            // No state/context change — the adapter invokes the consumer's on_activate
            // callback after sending this event.
            (_, Event::ActivateItem(_)) => None,

            // ── SelectItem ──────────────────────────────────────────────────
            // Respects selection_mode: None discards, Single replaces, Multiple toggles.
            (_, Event::SelectItem(id)) => {
                match props.selection_mode {
                    selection::Mode::None => return None, // selection disabled
                    selection::Mode::Single => {
                        // Deselect previous, select new (or deselect if same)
                        let id = id.clone();
                        let mut new_set = BTreeSet::new();
                        if !ctx.selected_items.contains(&id) {
                            new_set.insert(id);
                        }
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.selected_items = new_set;
                        }))
                    }
                    selection::Mode::Multiple => {
                        // Toggle membership
                        let id = id.clone();
                        Some(TransitionPlan::context_only(move |ctx| {
                            if ctx.selected_items.contains(&id) {
                                ctx.selected_items.remove(&id);
                            } else {
                                ctx.selected_items.insert(id);
                            }
                        }))
                    }
                }
            }

            // ── OverflowChanged ─────────────────────────────────────────────
            (_, Event::OverflowChanged(count)) => {
                let count = *count;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.overflow_count = count;
                    ctx.visible_count = ctx.registered_items.len().saturating_sub(count);
                }))
            }

            // ── SetProps ────────────────────────────────────────────────────
            (_, Event::SetProps) => {
                let disabled = props.disabled;
                let dir = props.dir;
                let locale = resolve_locale(props.locale.as_ref());
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                    ctx.dir = dir;
                    ctx.locale = locale;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        // Only emit SetProps for fields that are stored in Context and need syncing.
        // Props like selection_mode, disabled_items, orientation, button_label_behavior
        // are read directly from self.props in API methods — no Context sync needed.
        if old.disabled != new.disabled
            || old.dir != new.dir
            || old.locale != new.locale
        {
            events.push(Event::SetProps);
        }
        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "action-group"]
pub enum Part {
    Root,
    Item { item_id: Key },
    OverflowTrigger,
}

/// The API for the `ActionGroup` component.
pub struct Api<'a> {
    /// The current state of the action group.
    state: &'a State,
    /// The context of the action group.
    ctx: &'a Context,
    /// The props of the action group.
    props: &'a Props,
    /// The send function for the action group.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns the attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "toolbar");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation),
            if self.props.orientation == Orientation::Vertical { "vertical" } else { "horizontal" });
        if self.props.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Button label behavior styling hook.
        attrs.set(HtmlAttr::Data("ars-label-behavior"), match self.props.button_label_behavior {
            ButtonLabelBehavior::Show => "show",
            ButtonLabelBehavior::Collapse => "collapse",
            ButtonLabelBehavior::Hide => "hide",
        });
        // Density styling hook.
        if let Some(ref density) = self.props.density {
            attrs.set(HtmlAttr::Data("ars-density"), density.as_str());
        }
        // Justified layout styling hook.
        if self.props.justified {
            attrs.set(HtmlAttr::Data("ars-justified"), "true");
        }
        // Accessible name: aria-labelledby > aria_label > messages fallback.
        // role="toolbar" requires an accessible name.
        if let Some(ref labelledby) = self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby.as_str());
        } else if let Some(ref label) = self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.toolbar_label)(&self.ctx.locale));
            #[cfg(debug_assertions)]
            log::warn!("ActionGroup: No accessible name provided. Set aria_label or aria_labelledby on Props. role=\"toolbar\" requires an accessible name.");
        }
        attrs
    }

    /// Returns the attributes for the item element with the given key.
    pub fn item_attrs(&self, id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { item_id: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_focused = self.ctx.focused_item.as_ref() == Some(id);
        // In Idle state (no focused item), the first registered item gets tabindex="0"
        // so the toolbar is reachable by Tab (WAI-ARIA toolbar roving tabindex).
        let is_initial_focus = self.ctx.focused_item.is_none()
            && self.ctx.registered_items.first() == Some(id);
        attrs.set(HtmlAttr::TabIndex, if is_focused || is_initial_focus { "0" } else { "-1" });
        if is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        // Per-item disabled state: check both group-level and item-level disabled.
        let item_disabled = self.ctx.disabled || self.props.disabled_items.contains(id);
        if item_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            // Skip event handlers — adapter must not attach click/keydown
            // handlers when the item is disabled.
        }
        // aria-pressed: always set to "true" or "false" when selection mode is
        // active, so screen readers announce the toggle state on every item.
        // When selection_mode is None, omit aria-pressed entirely.
        match self.props.selection_mode {
            selection::Mode::Single | selection::Mode::Multiple => {
                let is_selected = self.ctx.selected_items.contains(id);
                attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if is_selected { "true" } else { "false" });
                if is_selected {
                    attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
                }
            }
            selection::Mode::None => {
                // No aria-pressed — items are plain action buttons.
            }
        }
        attrs
    }

    /// Returns true if the given item is disabled (either group-level or per-item).
    pub fn is_item_disabled(&self, id: &Key) -> bool {
        self.ctx.disabled || self.props.disabled_items.contains(id)
    }

    /// Returns true if the item at the given index has overflowed into the menu.
    pub fn is_overflowed(&self, index: usize) -> bool {
        index >= self.ctx.visible_count
    }

    /// Returns the attributes for the overflow trigger element.
    pub fn overflow_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OverflowTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.overflow_trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu");
        attrs.set(HtmlAttr::Type, "button");
        attrs
    }

    /// Handle keydown on an action-group item.
    /// Arrow keys navigate between items with RTL-aware direction swapping.
    /// Uses `KeyboardKey` enum and `KeyboardEventData` from `05-interactions.md`.
    pub fn on_item_keydown(&self, data: &KeyboardEventData) {
        let is_horizontal = self.props.orientation == Orientation::Horizontal;
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
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref item_id } => self.item_attrs(item_id),
            Part::OverflowTrigger => self.overflow_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
action-group
  root             <div>      data-ars-scope="action-group" data-ars-part="root"
                               role="toolbar" aria-orientation="horizontal|vertical"
  item             <button>   data-ars-scope="action-group" data-ars-part="item"
                               tabindex="0" (focused) | tabindex="-1" (not focused)
  overflow-menu    (Menu)     Only when overflow_mode=Menu and items overflow
  overflow-trigger <button>   data-ars-scope="action-group" data-ars-part="overflow-trigger"
  overflow-icon    (slot)     Custom content for the overflow trigger (default: "..." icon)
  overflow-content (Menu content with overflowed items)
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `role="toolbar"` on root with `aria-orientation` and an accessible name (`aria-label` or `aria-labelledby`). A `cfg(debug_assertions)` warning is emitted when no accessible name is provided.
- Roving tabindex: only the focused item has `tabindex="0"`, all others have `tabindex="-1"`.
- When `selection_mode` is `Single` or `Multiple`, items use `aria-pressed="true"` or `aria-pressed="false"` to indicate selection state. Both values are always present (not conditionally omitted) so screen readers consistently announce toggle state.
- When `selection_mode` is `None`, `aria-pressed` is omitted entirely — items are plain action buttons.
- The overflow menu trigger should have `aria-label` from Messages (e.g., "More actions") and `aria-haspopup="menu"`.
- **Screen reader note for overflow trigger:** When items overflow into a menu, the trigger button announces its label (e.g., "More actions") and its popup role (`aria-haspopup="menu"`). Screen reader users navigating the toolbar via arrow keys will encounter the overflow trigger as the last focusable item in the toolbar. Activating it opens the overflow menu, which should manage its own focus (moving focus to the first menu item on open).
- **Touch target:** Interactive action items MUST meet the minimum 44x44 CSS pixel touch target size (see foundation/03-accessibility.md section 7.1.1).
- **Forced colors:** In Windows High Contrast Mode (`@media (forced-colors: active)`), selected items MUST remain distinguishable via border or outline. Recommended: `[data-ars-selected] { outline: 2px solid Highlight; outline-offset: -2px; }`

### 3.2 Keyboard Interaction

| Key                        | Action                                   |
| -------------------------- | ---------------------------------------- |
| `ArrowRight` / `ArrowDown` | Move focus to the next item (wraps).     |
| `ArrowLeft` / `ArrowUp`    | Move focus to the previous item (wraps). |
| `Home`                     | Move focus to the first item.            |
| `End`                      | Move focus to the last item.             |
| `Tab`                      | Move focus out of the toolbar.           |
| `Enter` / `Space`          | Activate the focused item.               |

When `dir == Direction::Rtl` and `orientation == Orientation::Horizontal`:

- ArrowLeft maps to `FocusNext` (visually forward)
- ArrowRight maps to `FocusPrev` (visually backward)

The `on_item_keydown()` method on the API handles this mapping automatically. Adapters call `on_item_keydown()` with the raw key string and send the returned event (if any).

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the `ActionGroup` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the overflow menu trigger (default: "More actions").
    pub overflow_trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Fallback accessible label for the toolbar root. Used as `aria-label`
    /// when no `aria_label` or `aria_labelledby` prop is provided.
    pub toolbar_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            overflow_trigger_label: MessageFn::static_str("More actions"),
            toolbar_label: MessageFn::static_str("Actions"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                             | Default (en-US)  | Purpose                                |
| ------------------------------- | ---------------- | -------------------------------------- |
| `action_group.overflow_trigger` | `"More actions"` | `aria-label` for overflow menu trigger |
| `action_group.toolbar_label`    | `"Actions"`      | Fallback `aria-label` for toolbar root |

The `overflow_trigger_attrs()` method on the API reads `aria-label` from `messages.overflow_trigger_label`.

## 5. Adapter Callback Props

Adapters MUST expose these callback props:

| Prop                  | Type                              | Fires when                                |
| --------------------- | --------------------------------- | ----------------------------------------- |
| `on_action`           | `Option<Callback<Key>>`           | An item is activated (non-selection mode) |
| `on_selection_change` | `Option<Callback<BTreeSet<Key>>>` | Selection set changes                     |

## 6. Library Parity

> Compared against: React Aria (`ActionGroup`).

### 6.1 Props

| Feature           | ars-ui                       | React Aria               | Notes                             |
| ----------------- | ---------------------------- | ------------------------ | --------------------------------- |
| Disabled          | `disabled`                   | `isDisabled`             | Both libraries                    |
| Orientation       | `orientation`                | `orientation`            | Both libraries                    |
| Selection mode    | `selection_mode`             | `selectionMode`          | Both libraries                    |
| Disallow empty    | `disallow_empty_selection`   | `disallowEmptySelection` | Both libraries                    |
| Disabled items    | `disabled_items`             | `disabledKeys`           | Both libraries                    |
| Dir               | `dir`                        | --                       | ars-ui addition                   |
| Overflow strategy | `overflow: OverflowStrategy` | `overflowMode`           | Both libraries                    |
| Compact density   | `compact`                    | `density`                | RA uses enum; ars-ui uses boolean |

**Gaps:** None.

### 6.2 Anatomy

| Part            | ars-ui            | React Aria    | Notes                                          |
| --------------- | ----------------- | ------------- | ---------------------------------------------- |
| Root            | `Root`            | `ActionGroup` | Both libraries                                 |
| Item            | `Item { key }`    | `Item`        | Both libraries                                 |
| OverflowTrigger | `OverflowTrigger` | --            | ars-ui explicit part for overflow menu trigger |
| OverflowMenu    | `OverflowMenu`    | --            | ars-ui explicit part for overflow menu         |

**Gaps:** None.

### 6.3 Events

| Callback            | ars-ui                | React Aria          | Notes          |
| ------------------- | --------------------- | ------------------- | -------------- |
| on_action           | `on_action`           | `onAction`          | Both libraries |
| on_selection_change | `on_selection_change` | `onSelectionChange` | Both libraries |

**Gaps:** None.

### 6.4 Features

| Feature             | ars-ui                     | React Aria |
| ------------------- | -------------------------- | ---------- |
| Keyboard navigation | Yes                        | Yes        |
| Overflow to menu    | Yes                        | Yes        |
| Selection modes     | Yes (None/Single/Multiple) | Yes        |
| RTL support         | Yes                        | Yes        |
| Roving tabindex     | Yes                        | Yes        |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses `compact: bool` vs React Aria's `density: "compact"|"regular"`. ars-ui exposes overflow anatomy parts explicitly.
- **Recommended additions:** None.
