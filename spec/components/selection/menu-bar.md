---
component: MenuBar
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    radix-ui: Menubar
---

# MenuBar

A horizontal bar of top-level menu triggers (File, Edit, View...) where hovering between
open menus switches which popup is shown.

Top-level menus are stored as a `StaticCollection<menu_bar::Menu>` (from `06-collections.md`).
Navigation uses `Collection` trait methods.

## 1. State Machine

```rust
/// Payload for top-level menu bar entries.
#[derive(Clone, Debug)]
pub struct Menu {
    /// The label of the menu bar menu.
    pub label: String,
}
```

### 1.1 States

```rust
/// The state of the MenuBar component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No menu is active; arrow keys focus triggers.
    Inactive,
    /// A menu is open; arrow keys navigate within it.
    Active {
        /// The key of the active menu.
        menu: Key,
    },
}
```

### 1.2 Events

```rust
/// The events of the MenuBar component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Focus a top-level menu trigger.
    FocusItem(Key),
    /// Activate (open) a menu popup.
    ActivateMenu(Key),
    /// Deactivate — close current menu.
    DeactivateMenu,
    /// Move focus to next top-level trigger (wraps).
    MoveToNextMenu,
    /// Move focus to previous top-level trigger (wraps).
    MoveToPrevMenu,
    /// Close everything.
    Close,
    /// Focus the menu bar.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// Blur the menu bar.
    Blur,
    /// Update the top-level menu collection dynamically.
    UpdateMenus(StaticCollection<Menu>),
    /// Synchronize context values derived from updated props.
    SyncProps,
}
```

### 1.3 Context

```rust
/// The context of the `MenuBar` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// The menus of the menu bar.
    pub menus: StaticCollection<Menu>,
    /// The active menu of the menu bar.
    pub active_menu: Option<Key>,
    /// The focused item of the menu bar.
    pub focused_item: Option<Key>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the MenuBar state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the menu bar.
    pub id: String,
    /// Whether the menu bar is disabled.
    pub disabled: bool,
    /// Orientation of the menu bar. Default: `Horizontal`.
    pub orientation: Orientation,
    /// Text direction for RTL support. Default: `Ltr`.
    pub dir: Direction,
    /// Whether focus wraps from the last trigger back to the first (and vice versa).
    /// Default: `true`.
    pub loop_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
        }
    }
}
```

### 1.5 Full Machine Implementation

The agnostic implementation owns top-level menu collection state, active-menu intent, focused
trigger intent, focus-visible state, ids, orientation, direction, and loop-focus behavior. It uses
`NoEffect`: adapters consume state transitions to move live focus, position menu content, and
attach dismissal resources.

Transition requirements:

- `ActivateMenu(key)` validates that `key` exists, enters `State::Active { menu: key }`, and
  stores both `active_menu` and `focused_item`.
- `DeactivateMenu` and `Close` clear `active_menu` while preserving the focused trigger.
- `FocusItem(key)` validates that `key` exists. In inactive state it only moves trigger focus;
  in active state it also switches the active menu to that trigger.
- `Focus { is_keyboard }` updates focus-visible state and initializes focus to the first menu
  if no trigger is focused.
- `Blur` returns to inactive state, clears active/focus-visible state, and preserves or
  restores a focused trigger so one enabled menu trigger remains tabbable.
- `MoveToNextMenu` and `MoveToPrevMenu` use collection order and `loop_focus`; in active
  state they switch the active menu, and in inactive state they only move focused trigger.
- `UpdateMenus` replaces the top-level collection and invalidates stale active/focused keys;
  if the active menu key is removed, the machine transitions to `Inactive`; when enabled,
  it restores focus to the first menu if no focused trigger remains.
- `SyncProps` updates ids; when the menubar becomes disabled, it transitions to `Inactive`
  and clears active/focused/focus-visible state.
- Keyboard dispatch helpers resolve `Direction::Auto` through `Context::locale` before
  applying Left/Right semantics.

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;
}
```

### 1.6 Connect / API

The connect API derives DOM-facing attrs and event dispatch helpers for the top-level menu
strip. Nested menu content uses the separate `Menu` component contract.

Required public pieces:

- `Part` uses scope `menu-bar` and declares `Root`, `Menu`, `MenuTrigger`,
  `MenuPositioner`, and `MenuContent`.
- `root_attrs()` emits scope/part data attrs, `role="menubar"`, and
  `aria-orientation` from `Props::orientation`.
- `menu_attrs(key)` emits the top-level menu wrapper id.
- `menu_trigger_attrs(key)` emits stable id, `role="menuitem"`, `aria-haspopup="menu"`,
  `aria-expanded`, `aria-controls`, roving `tabindex`, active/focused data attrs, and
  disabled attrs.
- `menu_positioner_attrs(key)` and `menu_content_attrs(key)` emit stable ids; content emits
  `role="menu"`, `tabindex="-1"`, and `aria-labelledby` pointing to the trigger.
- `on_trigger_click(key)` toggles the active menu.
- `on_trigger_keydown(key, data)` opens on Enter/Space, opens with the orientation-specific
  submenu arrow, and moves with orientation-specific top-level traversal arrows
  (Left/Right for horizontal, Up/Down for vertical, with horizontal RTL inversion).
- `on_trigger_pointer_enter(key)` switches menus only while a menu is already active.
- `on_content_keydown(data)` switches top-level menus on orientation-specific traversal
  arrows and closes on Escape.
- `on_root_focus(is_keyboard)` and `on_root_blur()` dispatch focus/blur state events.

## 2. Anatomy

| Part             | Selector                                                       | Element    |
| ---------------- | -------------------------------------------------------------- | ---------- |
| `Root`           | `[data-ars-scope="menu-bar"][data-ars-part="root"]`            | `<div>`    |
| `Menu`           | `[data-ars-scope="menu-bar"][data-ars-part="menu"]`            | `<div>`    |
| `MenuTrigger`    | `[data-ars-scope="menu-bar"][data-ars-part="menu-trigger"]`    | `<button>` |
| `MenuPositioner` | `[data-ars-scope="menu-bar"][data-ars-part="menu-positioner"]` | `<div>`    |
| `MenuContent`    | `[data-ars-scope="menu-bar"][data-ars-part="menu-content"]`    | `<div>`    |

Nested item-type parts (`Item`, `CheckboxItem`, `RadioItem`, `Separator`, `SubTrigger`,
`SubContent`, and related parts) are delegated to the nested `Menu` component rendered inside
`MenuContent`.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element       | Value                                        |
| ------------------ | ------------- | -------------------------------------------- |
| `role`             | `Root`        | `menubar`                                    |
| `aria-orientation` | `Root`        | `"horizontal"` or `"vertical"` from props    |
| `role`             | `MenuTrigger` | `menuitem`                                   |
| `aria-haspopup`    | `MenuTrigger` | `menu`                                       |
| `aria-expanded`    | `MenuTrigger` | `true` when menu is open                     |
| `tabindex`         | `MenuTrigger` | Roving: focused trigger gets `0`, others `-1`; when enabled and menus exist, exactly one trigger is tabbable |

### 3.2 Keyboard Interaction

| Key                                    | Inactive Mode               | Active Mode                       |
| -------------------------------------- | --------------------------- | --------------------------------- |
| Horizontal ArrowLeft/Right             | Move focus between triggers | Close current, open adjacent menu |
| Vertical ArrowUp/Down                  | Move focus between triggers | Close current, open adjacent menu |
| Horizontal ArrowDown                   | Open current menu           | Navigate within menu              |
| Vertical ArrowRight (LTR) / Left (RTL) | Open current menu           | Navigate within menu              |
| Enter / Space                          | Open current menu           | Activate menu item                |
| Escape                                 | ---                         | Close menu -> Inactive            |
| Tab                                    | Leave menubar               | Close menu, leave menubar         |

## 4. Internationalization

### 4.1 Messages

```rust
/// Translatable messages for MenuBar.
#[derive(Clone, Debug)]
pub struct Messages {
    // No component-generated text — all labels are consumer-provided.
    // Struct exists for pattern conformance with other component machines.
}

impl Default for Messages {
    fn default() -> Self {
        Self {}
    }
}

impl ComponentMessages for Messages {}
```

- **RTL**: ArrowLeft/Right swap direction for horizontal menubar.
- **Trigger labels**: User-provided, localized.
- **Keyboard shortcut text**: Follows OS conventions (Ctrl vs Cmd).

## 5. Library Parity

> Compared against: Radix UI (`Menubar`).

### 5.1 Props

| Feature                | ars-ui        | Radix UI                                   | Notes                                                |
| ---------------------- | ------------- | ------------------------------------------ | ---------------------------------------------------- |
| Disabled               | `disabled`    | --                                         | ars-ui exclusive                                     |
| Orientation            | `orientation` | --                                         | Radix is horizontal-only                             |
| Direction (RTL)        | `dir`         | `dir`                                      | --                                                   |
| Loop focus             | `loop_focus`  | `loop`                                     | --                                                   |
| Controlled active menu | --            | `value` / `defaultValue` / `onValueChange` | Radix exposes which menu is open as controlled state |

**Gaps:** None. Radix's controlled `value`/`onValueChange` for tracking which menu is open is a convenience; ars-ui manages this internally via `State::Active { menu }`.

### 5.2 Anatomy

| Part                      | ars-ui              | Radix UI                        | Notes             |
| ------------------------- | ------------------- | ------------------------------- | ----------------- |
| Root                      | `Root`              | `Root`                          | Menubar container |
| MenuTrigger               | `MenuTrigger`       | `Trigger`                       | --                |
| MenuPositioner            | `MenuPositioner`    | `Portal`                        | --                |
| MenuContent               | `MenuContent`       | `Content`                       | --                |
| Item (within menu)        | delegated to `Menu` | `Item`                          | --                |
| Group                     | delegated to `Menu` | `Group`                         | --                |
| Label                     | delegated to `Menu` | `Label`                         | --                |
| CheckboxItem              | delegated to `Menu` | `CheckboxItem`                  | --                |
| RadioGroup                | delegated to `Menu` | `RadioGroup`                    | --                |
| RadioItem                 | delegated to `Menu` | `RadioItem`                     | --                |
| ItemIndicator             | delegated to `Menu` | `ItemIndicator`                 | --                |
| Separator                 | delegated to `Menu` | `Separator`                     | --                |
| Arrow                     | delegated to `Menu` | `Arrow`                         | --                |
| Sub/SubTrigger/SubContent | delegated to `Menu` | `Sub`/`SubTrigger`/`SubContent` | --                |

**Gaps:** None. ars-ui delegates individual menu content to the `Menu` component rather than re-declaring all menu parts.

### 5.3 Events

| Callback           | ars-ui                        | Radix UI            | Notes |
| ------------------ | ----------------------------- | ------------------- | ----- |
| Active menu change | via `State::Active { menu }`  | `onValueChange`     | --    |
| Item action        | delegated to `Menu.on_action` | per-item `onSelect` | --    |

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                        | Radix UI |
| --------------------------- | ----------------------------- | -------- |
| Horizontal menubar          | Yes                           | Yes      |
| Vertical menubar            | Yes (`orientation: Vertical`) | No       |
| Arrow key menu switching    | Yes                           | Yes      |
| Hover-to-switch when active | Yes                           | Yes      |
| Submenus (within menus)     | Yes (via `Menu`)              | Yes      |
| Checkbox/radio items        | Yes (via `Menu`)              | Yes      |
| Typeahead                   | Yes (via `Menu`)              | Yes      |
| RTL support                 | Yes                           | Yes      |
| Focus wrapping              | Yes                           | Yes      |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui supports vertical orientation; Radix is horizontal-only; (2) ars-ui delegates individual menu content to the `Menu` component rather than re-declaring all menu item types in the MenuBar spec; (3) Radix exposes the active menu as a controlled `value` string; ars-ui uses internal state tracking.
- **Recommended additions:** None.
