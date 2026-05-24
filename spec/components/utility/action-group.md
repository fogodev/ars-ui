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
| `RegisterItem`    | `Key`   | Register a rendered item key in logical DOM order for roving-focus navigation.                                                                                           |
| `UnregisterItem`  | `Key`   | Unregister a rendered item key and clear focus/selection state that references the removed key.                                                                          |
| `SetProps`        | ---     | Sync context from updated props.                                                                                                                                         |

### 1.3 Context

```rust
use std::collections::BTreeSet;
use ars_collections::Key;
use ars_core::{Direction, Locale};

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
use std::collections::BTreeSet;
use ars_collections::{Key, selection};
use ars_core::{Callback, Direction, HasId, Orientation};

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
    /// Accessible label for the toolbar. Either `aria_label` or `aria_labelledby` must be set.
    /// `role="toolbar"` requires an accessible name.
    pub aria_label: Option<String>,
    /// ID of the element that labels this toolbar (alternative to `aria_label`).
    pub aria_labelledby: Option<String>,
    /// Callback invoked when an item is activated.
    pub on_action: Option<Callback<dyn Fn(Key) + Send + Sync>>,
    /// Callback invoked when the selected key set changes.
    pub on_selection_change: Option<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverflowMode {
    /// Items wrap to the next line.
    #[default]
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
            aria_label: None,
            aria_labelledby: None,
            on_action: None,
            on_selection_change: None,
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

```rust,no_check
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

```rust,no_check
#[cfg(not(feature = "web"))]
{
    let listener = window().on_resize(move |_| recalculate_overflow());
    use_drop(move || drop(listener));
}
```

### 1.6 Machine Contract

The agnostic core implements `ars_core::Machine` with:

```rust
type State = State;
type Event = Event;
type Context = Context;
type Props = Props;
type Messages = Messages;
type Effect = Effect;
type Api<'a> = Api<'a>;
```

The machine contract is:

- `init` starts in `State::Idle` with empty `selected_items`, `registered_items`, and zero overflow counts.
- A disabled group blocks only `ActivateItem` and `SelectItem`. Focus, blur, roving navigation, registration, unregistration, overflow updates, and prop sync still work.
- `RegisterItem(Key)` appends the key in logical render order when not already present and recomputes `visible_count`.
- `UnregisterItem(Key)` removes the key, removes it from `selected_items`, recomputes `visible_count`, and clears focused state only when the removed key was focused.
- `FocusItem(Key)` only focuses registered keys that are not in `disabled_items`.
- `FocusNext`, `FocusPrev`, `FocusFirst`, and `FocusLast` wrap over `registered_items` and skip `disabled_items`.
- `Blur` returns to `Idle` and clears `focused_item`.
- `ActivateItem(Key)` emits an `Effect::Action` intent and does not mutate selection.
- `SelectItem(Key)` is ignored for disabled groups, disabled item keys, and `selection::Mode::None`.
- `selection::Mode::Single` selects the requested key and clears any previous key; selecting the already selected key clears selection.
- `selection::Mode::Multiple` toggles membership.
- `OverflowChanged(count)` sets `overflow_count = count` and `visible_count = registered_items.len().saturating_sub(count)`.
- `SetProps` synchronizes `disabled`, `dir`, `selection_mode`, and `disabled_items` effects on context: selection is normalized to the new mode, disabled item keys are removed from selection, and focused item is cleared when it becomes item-disabled.
- `on_props_changed` emits `SetProps` when `disabled`, `dir`, `disabled_items`, or `selection_mode` changes.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "action-group"]
pub enum Part {
    Root,
    Item { item_id: Key },
    OverflowTrigger,
}
```

The `Api<'a>` exposes:

- `root_attrs()`
- `item_attrs(&Key)`
- `overflow_trigger_attrs()`
- `is_item_disabled(&Key)`
- `is_overflowed(index: usize)`
- `on_item_keydown(&KeyboardEventData)`
- `on_item_focus(&Key)`
- `on_item_blur()`
- `on_item_click(&Key)`
- `on_item_mount(&Key)`
- `on_item_unmount(&Key)`

`root_attrs()` returns `data-ars-scope="action-group"`, `data-ars-part="root"`, `role="toolbar"`, `aria-orientation`, `data-ars-variant`, `data-ars-overflow-mode`, and `data-ars-label-behavior`. It emits `aria-disabled="true"` and `data-ars-disabled` when disabled, `data-ars-density` when `density` is set, `data-ars-justified` when `justified` is true, and uses `aria-labelledby` before `aria-label` before `messages.toolbar_label`.

`item_attrs(&Key)` returns `data-ars-scope="action-group"`, `data-ars-part="item"`, `data-ars-key`, `type="button"`, roving `tabindex`, and `data-ars-state="selected|idle"`. The focused item has `data-ars-focused`. Disabled groups and disabled item keys emit `aria-disabled="true"` and `data-ars-disabled`. `aria-pressed` is emitted only for `selection::Mode::Single` and `selection::Mode::Multiple`.

`overflow_trigger_attrs()` returns `data-ars-scope="action-group"`, `data-ars-part="overflow-trigger"`, `type="button"`, `aria-label` from `messages.overflow_trigger_label`, `aria-haspopup="menu"`, and `aria-expanded="false"`.

## 2. Anatomy

```text
action-group
  root             <div>      data-ars-scope="action-group" data-ars-part="root"
                               role="toolbar" aria-orientation="horizontal|vertical"
  item             <button>   data-ars-scope="action-group" data-ars-part="item"
                               tabindex="0" (focused) | tabindex="-1" (not focused)
  overflow-trigger <button>   data-ars-scope="action-group" data-ars-part="overflow-trigger"
```

The overflow menu, trigger icon, and menu content are adapter composition details, not `ActionGroup` core `Part` variants.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `role="toolbar"` on root with `aria-orientation` and an accessible name (`aria-label`, `aria-labelledby`, or the localized toolbar label fallback).
- Roving tabindex: the focused item has `tabindex="0"`; when idle, the first registered non-item-disabled item has `tabindex="0"`; all others have `tabindex="-1"`.
- When `selection_mode` is `Single` or `Multiple`, items use `aria-pressed="true"` or `aria-pressed="false"` to indicate selection state. Both values are always present (not conditionally omitted) so screen readers consistently announce toggle state.
- When `selection_mode` is `None`, `aria-pressed` is omitted entirely — items are plain action buttons.
- The overflow menu trigger should have `aria-label` from Messages (e.g., "More actions"), `aria-haspopup="menu"`, and `aria-expanded="false"` before the adapter menu opens.
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

The `on_item_keydown()` method on the API handles this mapping automatically from `KeyboardEventData`. It dispatches navigation events for arrow/Home/End keys and dispatches `ActivateItem(focused_item)` for Enter and Space when an item is focused.

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

| Feature        | ars-ui                  | React Aria      | Notes                                     |
| -------------- | ----------------------- | --------------- | ----------------------------------------- |
| Disabled       | `disabled`              | `isDisabled`    | Both libraries                            |
| Orientation    | `orientation`           | `orientation`   | Both libraries                            |
| Selection mode | `selection_mode`        | `selectionMode` | Both libraries                            |
| Disabled items | `disabled_items`        | `disabledKeys`  | Both libraries                            |
| Dir            | `dir`                   | --              | ars-ui addition                           |
| Overflow mode  | `overflow_mode`         | `overflowMode`  | Both libraries                            |
| Visible limit  | `max_visible_actions`   | --              | ars-ui explicit adapter measurement input |
| Density        | `density: Option<_>`    | `density`       | Both expose density; ars-ui leaves tokens |
| Justified      | `justified`             | --              | ars-ui styling hook                       |
| Label behavior | `button_label_behavior` | --              | ars-ui icon/text display hook             |

**Gaps:** None.

### 6.2 Anatomy

| Part            | ars-ui             | React Aria    | Notes                                          |
| --------------- | ------------------ | ------------- | ---------------------------------------------- |
| Root            | `Root`             | `ActionGroup` | Both libraries                                 |
| Item            | `Item { item_id }` | `Item`        | Both libraries                                 |
| OverflowTrigger | `OverflowTrigger`  | --            | ars-ui explicit part for overflow menu trigger |

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
- **Divergences:** ars-ui keeps overflow measurement adapter-owned via `OverflowChanged` and `max_visible_actions`, exposes `OverflowTrigger` as an explicit core part, and uses open density string tokens for styling hooks.
- **Recommended additions:** None.
