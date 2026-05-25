---
component: ContextMenu
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Menu
    radix-ui: ContextMenu
---

# ContextMenu

A `Menu` triggered by right-click (`contextmenu` event) rather than a button click. Positioned
at the pointer location. Shares Menu's core item model, navigation, and selection semantics but
differs in trigger mechanism (right-click vs button), positioning (pointer coordinates vs trigger
anchor), and part naming (Target replaces Trigger).

Items are stored as a `StaticCollection<menu::Item>` (from `06-collections.md`). Separators
are structural `NodeType::Separator` nodes. Groups are `NodeType::Section` + `NodeType::Header`
nodes. Navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`.
Typeahead uses `typeahead::State`.
Submenu items use the same submenu intent surface as `Menu`: the core records
which submenu trigger is open and emits submenu parts/ARIA, while adapters own
live submenu positioning and focus movement.

## 1. State Machine

### 1.1 States

```rust
/// The states of the ContextMenu state machine.
/// Same states as Menu — closed or open with highlight tracking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The context menu is closed.
    Closed,
    /// The context menu is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the ContextMenu state machine.
/// Menu's events plus the context-menu-specific open event.
#[derive(Clone, Debug)]
pub enum Event {
    /// Open the context menu at pointer coordinates.
    ContextOpen { x: f64, y: f64 },
    /// Close the menu.
    Close,
    /// Highlight an item by key (None clears highlight).
    HighlightItem(Option<Key>),
    /// Highlight the first enabled item.
    HighlightFirst,
    /// Highlight the last enabled item.
    HighlightLast,
    /// Highlight the next enabled item.
    HighlightNext,
    /// Highlight the previous enabled item.
    HighlightPrev,
    /// Select (activate) a normal menu item.
    SelectItem(Key),
    /// Toggle a checkbox item.
    ToggleCheckboxItem(Key),
    /// Select a radio item within a group.
    SelectRadioItem {
        /// The radio group name.
        group: String,
        /// The key of the radio item.
        value: Key,
    },
    /// Open a submenu for the given trigger item.
    OpenSubmenu(Key),
    /// Close the currently open submenu.
    CloseSubmenu,
    /// Click outside the menu.
    ClickOutside,
    /// Typeahead search with character and timestamp (ms).
    TypeaheadSearch(char, u64),
    /// Update the item collection dynamically.
    UpdateItems(StaticCollection<menu::Item>),
    /// Synchronize context values derived from updated props.
    SyncProps,
}
```

### 1.3 Context

```rust
/// The context of the ContextMenu state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Locale used for typeahead matching.
    pub locale: Locale,
    /// The items of the context menu.
    pub items: StaticCollection<menu::Item>,
    /// Whether the context menu is open.
    pub open: bool,
    /// The highlighted key of the context menu.
    pub highlighted_key: Option<Key>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The checked items of the context menu.
    pub checked_items: BTreeMap<Key, bool>,
    /// The radio groups of the context menu.
    pub radio_groups: BTreeMap<String, Key>,
    /// Key of the currently open submenu trigger.
    pub submenu_open: Option<Key>,
    /// The typeahead state of the context menu.
    pub typeahead: typeahead::State,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    pub loop_focus: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// The pointer position where the context menu was opened.
    /// Set by `ContextOpen` and used by the positioner to anchor the floating content.
    pub position: Option<(f64, f64)>,
}
```

### 1.4 Props

```rust
/// Props for the ContextMenu state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the context menu.
    pub id: String,
    /// Whether the context menu is disabled.
    pub disabled: bool,
    /// Whether the menu closes when an item is activated (selected or triggered).
    /// Applies to both selectable items (checkbox/radio) and action items (normal).
    /// Individual `menu::Item.close_on_action` overrides this per-item.
    /// Default: `true`.
    pub close_on_action: bool,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    /// Default: `true`.
    pub loop_focus: bool,
    /// How disabled items behave in keyboard navigation.
    /// `Skip` = disabled items are skipped during keyboard navigation (not focusable, not selectable).
    /// `FocusOnly` = disabled items are focusable but not selectable.
    /// Default: `DisabledBehavior::Skip`.
    pub disabled_behavior: DisabledBehavior,
    /// Set of keys for menu items that are disabled.
    /// Disabled items are skipped during keyboard navigation and cannot be selected or triggered.
    pub disabled_keys: BTreeSet<Key>,
    /// Callback invoked when the context menu open state changes.
    /// Fires after the transition completes with the new open state value.
    pub on_open_change: Option<Callback<menu::OpenChangeCallback>>,
    /// Callback invoked when a menu item is activated (Enter/click on action items).
    /// Distinct from selection-change callbacks — `on_action` fires for command execution,
    /// not for checkbox/radio state toggling.
    pub on_action: Option<Callback<menu::ActionCallback>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            close_on_action: true,
            loop_focus: true,
            disabled_behavior: DisabledBehavior::Skip,
            disabled_keys: BTreeSet::new(),
            on_open_change: None,
            on_action: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

The agnostic implementation mirrors `Menu` navigation and selection semantics while replacing
button-triggered open with `ContextOpen { x, y }`. The core machine uses `NoEffect`: it stores
pointer position, open/highlight/submenu intent, checked/radio state, typeahead state, and stable
ids. Framework adapters consume that state to place the floating content, move live focus, return
focus to the target, attach dismissal listeners, and manage timers.

Transition requirements:

- `ContextOpen { x, y }` is ignored when disabled. Otherwise it opens the menu, stores
  `position = Some((x, y))`, and highlights the first enabled item. Re-opening while already
  open updates the stored position and recomputes the first enabled highlight.
- `Close` and `ClickOutside` close the menu and clear `highlighted_key`, `submenu_open`,
  `typeahead`, and `position`.
- `Highlight*` events only target focusable collection items, honoring `disabled_keys` and
  `disabled_behavior`.
- `SelectItem`, `ToggleCheckboxItem`, and `SelectRadioItem` validate the collection node,
  item type, and disabled state before mutating state or firing callbacks.
- `OpenSubmenu` only succeeds for selectable `menu::ItemType::Submenu` items and records
  `submenu_open`; `CloseSubmenu` clears it and restores highlight to the trigger.
- `TypeaheadSearch` delegates to `typeahead::State::process_char_with_locale`.
- `UpdateItems` replaces the collection and invalidates stale highlight, submenu, checked,
  and radio references.
- `SyncProps` updates `loop_focus` and `ids`, and clears interactive state if the component
  becomes disabled.

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

The connect API derives DOM-facing attrs and event dispatch helpers from machine state.

Required public pieces:

- `Part` uses scope `context-menu` and declares `Root`, `Target`, `Positioner`, `Arrow`,
  `Content`, `ItemGroup`, `ItemGroupLabel`, `Item`, `ItemText`, `ItemIndicator`, `Separator`,
  `CheckboxItem`, `RadioGroup`, `RadioItem`, `SubTrigger`, `SubPositioner`, `SubContent`,
  and `Shortcut`.
- `root_attrs()` emits scope/part data attrs and disabled data state.
- `target_attrs()` emits stable id, `aria-haspopup="menu"`, `aria-expanded`,
  `aria-controls`, and disabled attrs.
- `content_attrs()` emits stable id, `role="menu"`, `tabindex="-1"`, and
  `aria-labelledby` pointing to the target.
- Item attrs emit role, roving tabindex, highlighted/disabled data attrs, and
  `aria-keyshortcuts` when present on `menu::Item`.
- Checkbox/radio attrs emit `aria-checked`; submenu attrs emit `aria-haspopup`,
  `aria-expanded`, `aria-controls`, and submenu content labeling.
- `on_target_contextmenu(x, y)` dispatches `ContextOpen { x, y }` when enabled.
- `on_target_keydown(data)` dispatches keyboard context-menu open for `Shift+F10` when
  enabled.
- `on_content_keydown(_at)` handles vertical navigation, Home/End, Escape, submenu
  open/close keys, Enter/Space activation, and printable typeahead with adapter-provided
  timestamps when available.
- `on_item_click`, `on_item_pointer_enter`, and `on_content_pointer_leave` dispatch typed
  highlight/activation events.

## 2. Anatomy

```text
ContextMenu
├── Root
├── Target                (the element receiving the contextmenu event)
├── Positioner            (floating, anchored to {x, y} pointer coordinates)
│   ├── Arrow             (optional)
│   └── Content           (role="menu")
│       ├── ItemGroup     (×N, optional)
│       │   ├── ItemGroupLabel
│       │   └── Item      (×N)  role="menuitem"
│       │       ├── ItemText
│       │       └── ItemIndicator (optional)
│       ├── CheckboxItem  (×N)  role="menuitemcheckbox"
│       │   ├── ItemText
│       │   └── ItemIndicator
│       ├── RadioGroup    (×N)
│       │   └── RadioItem (×N)  role="menuitemradio"
│       │       ├── ItemText
│       │       └── ItemIndicator
│       ├── SubTrigger    (×N)  role="menuitem" aria-haspopup="menu"
│       │   ├── ItemText
│       │   └── Shortcut   (optional)
│       ├── SubPositioner (×N)
│       │   └── SubContent (×N) role="menu"
│       └── Separator
```

| Part             | Selector                                                            | Element  | Notes                                                 |
| ---------------- | ------------------------------------------------------------------- | -------- | ----------------------------------------------------- |
| `Root`           | `[data-ars-scope="context-menu"][data-ars-part="root"]`             | `<div>`  | Wrapper container                                     |
| `Target`         | `[data-ars-scope="context-menu"][data-ars-part="target"]`           | `<div>`  | Element receiving `contextmenu` event and `Shift+F10` |
| `Positioner`     | `[data-ars-scope="context-menu"][data-ars-part="positioner"]`       | `<div>`  | Anchored to pointer `(x, y)`                          |
| `Arrow`          | `[data-ars-scope="context-menu"][data-ars-part="arrow"]`            | `<div>`  | Optional floating arrow                               |
| `Content`        | `[data-ars-scope="context-menu"][data-ars-part="content"]`          | `<div>`  | Menu panel (`role="menu"`)                            |
| `ItemGroup`      | `[data-ars-scope="context-menu"][data-ars-part="item-group"]`       | `<div>`  | `role="group"`                                        |
| `ItemGroupLabel` | `[data-ars-scope="context-menu"][data-ars-part="item-group-label"]` | `<div>`  | Labels a group                                        |
| `Item`           | `[data-ars-scope="context-menu"][data-ars-part="item"]`             | `<div>`  | Action item (`role="menuitem"`)                       |
| `ItemText`       | `[data-ars-scope="context-menu"][data-ars-part="item-text"]`        | `<span>` | Item label text                                       |
| `ItemIndicator`  | `[data-ars-scope="context-menu"][data-ars-part="item-indicator"]`   | `<div>`  | Check/radio indicator                                 |
| `CheckboxItem`   | `[data-ars-scope="context-menu"][data-ars-part="checkbox-item"]`    | `<div>`  | `role="menuitemcheckbox"`                             |
| `RadioGroup`     | `[data-ars-scope="context-menu"][data-ars-part="radio-group"]`      | `<div>`  | `role="group"`                                        |
| `RadioItem`      | `[data-ars-scope="context-menu"][data-ars-part="radio-item"]`       | `<div>`  | `role="menuitemradio"`                                |
| `SubTrigger`     | `[data-ars-scope="context-menu"][data-ars-part="sub-trigger"]`      | `<div>`  | Submenu trigger (`role="menuitem"`)                   |
| `SubPositioner`  | `[data-ars-scope="context-menu"][data-ars-part="sub-positioner"]`   | `<div>`  | Adapter-positioned submenu wrapper                    |
| `SubContent`     | `[data-ars-scope="context-menu"][data-ars-part="sub-content"]`      | `<div>`  | Submenu panel (`role="menu"`)                         |
| `Shortcut`       | `[data-ars-scope="context-menu"][data-ars-part="shortcut"]`         | `<span>` | Visual keyboard shortcut hint                         |
| `Separator`      | `[data-ars-scope="context-menu"][data-ars-part="separator"]`        | `<hr>`   | `role="separator"`                                    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

ContextMenu uses **roving tabindex** for item navigation (consistent with Menu). When the
context menu opens, focus moves to the Content element, then to the first enabled item. Each
highlighted item receives `tabindex="0"` and `element.focus()`; all other items have
`tabindex="-1"`. This ensures compatibility with all screen readers including VoiceOver iOS,
which does not support `aria-activedescendant`.

| Property          | Element         | Value                                                               |
| ----------------- | --------------- | ------------------------------------------------------------------- |
| `aria-haspopup`   | `Target`        | `"menu"` — indicates a context menu is available                    |
| `aria-expanded`   | `Target`        | `"true"` when open, `"false"` when closed                           |
| `aria-controls`   | `Target`        | Content element id                                                  |
| `role`            | `Content`       | `"menu"`                                                            |
| `tabindex`        | `Content`       | `"-1"` (focusable container, focus immediately moves to first item) |
| `aria-labelledby` | `Content`       | Target element id                                                   |
| `role`            | `Item`          | `"menuitem"`                                                        |
| `tabindex`        | `Item`          | `"0"` when highlighted, `"-1"` otherwise                            |
| `role`            | `CheckboxItem`  | `"menuitemcheckbox"`                                                |
| `aria-checked`    | `CheckboxItem`  | `"true"` / `"false"`                                                |
| `role`            | `RadioItem`     | `"menuitemradio"`                                                   |
| `aria-checked`    | `RadioItem`     | `"true"` / `"false"`                                                |
| `role`            | `Separator`     | `"separator"`                                                       |
| `role`            | `ItemGroup`     | `"group"`                                                           |
| `aria-labelledby` | `ItemGroup`     | ItemGroupLabel element id                                           |
| `role`            | `SubTrigger`    | `"menuitem"`                                                        |
| `aria-haspopup`   | `SubTrigger`    | `"menu"`                                                            |
| `aria-expanded`   | `SubTrigger`    | `"true"` when that submenu is open, `"false"` otherwise             |
| `aria-controls`   | `SubTrigger`    | SubContent element id                                               |
| `role`            | `SubContent`    | `"menu"`                                                            |
| `aria-labelledby` | `SubContent`    | SubTrigger item element id                                          |
| `aria-hidden`     | `Arrow`         | `"true"`                                                            |
| `aria-hidden`     | `ItemIndicator` | `"true"`                                                            |
| `aria-hidden`     | `Shortcut`      | `"true"`                                                            |

**Menu Item Types** (same as Menu):

1. Action items: `role="menuitem"`.
2. Checkbox items: `role="menuitemcheckbox"` with `aria-checked`.
3. Radio items: `role="menuitemradio"` with `aria-checked`.
4. Submenu trigger items: `role="menuitem"` with `aria-haspopup="menu"`.
5. Item type is an explicit prop on `menu::Item`, not inferred.
6. Mixed types in the same context menu are valid and follow WAI-ARIA Menu pattern.

### 3.2 Keyboard Interaction

| Key              | Action                                                                |
| ---------------- | --------------------------------------------------------------------- |
| Shift+F10        | Open context menu at target element position (when target is focused) |
| Context Menu key | Open context menu at target element position (platform-native)        |
| ArrowDown        | Highlight next enabled item                                           |
| ArrowUp          | Highlight previous enabled item                                       |
| Enter / Space    | Activate highlighted item (select, toggle checkbox, or select radio)  |
| Escape           | Close context menu, return focus to target                            |
| Home             | Highlight first enabled item                                          |
| End              | Highlight last enabled item                                           |
| a-z              | Typeahead highlight — jumps to next item starting with that character |

**Typeahead**: Single printable character jumps to the next item starting with
that character. If additional characters are typed within 500ms, the search
accumulates (e.g., "abc" matches "Abc item"). After 500ms timeout, buffer resets.

**Right-click (contextmenu event)**: The browser's native `contextmenu` event is prevented
and the custom context menu is shown at the pointer coordinates. If the context menu is
already open, it repositions to the new coordinates.

### 3.3 Focus Management

- **On open**: The machine stores the pointer `(x, y)` coordinates and highlights the
  first enabled item. The adapter uses that position and highlight intent to place the
  menu and move live focus.
- **On close**: The machine clears open state, highlight, submenu state, typeahead state,
  and pointer position. The adapter returns focus to the Target element.
- **Shift+F10 open**: When opened via keyboard (`Shift+F10`), the adapter resolves the
  target element's bounding rectangle center as the `(x, y)` position. The menu appears
  near the focused target rather than at arbitrary coordinates.
- **Roving tabindex**: The highlighted item has `tabindex="0"`, all others `tabindex="-1"`.
  ArrowDown/ArrowUp cycle through enabled items. When `loop_focus` is true, navigation
  wraps from last to first and vice versa.

## 4. Internationalization

- **RTL**: Vertical ArrowDown/ArrowUp navigation is stable. Submenu open/close direction
  follows the same locale-aware ArrowRight/ArrowLeft convention as Menu.
- **Typeahead**: Locale-aware via `Collator` for character matching.
- **Separator**: Decorative — no localization needed.

ContextMenu generates no user-visible text, so no Messages struct is required. All labels
are consumer-provided via `menu::Item.label`.

## 5. Library Parity

> Compared against: Ark UI (`Menu` with `ContextTrigger`), Radix UI (`ContextMenu`).

### 5.1 Props

| Feature           | ars-ui              | Ark UI          | Radix UI            | Notes                                       |
| ----------------- | ------------------- | --------------- | ------------------- | ------------------------------------------- |
| Disabled          | `disabled`          | --              | Trigger `disabled`  | --                                          |
| Close on action   | `close_on_action`   | `closeOnSelect` | --                  | --                                          |
| Loop focus        | `loop_focus`        | `loopFocus`     | Content `loop`      | --                                          |
| Disabled keys     | `disabled_keys`     | --              | per-item `disabled` | --                                          |
| Disabled behavior | `disabled_behavior` | --              | --                  | ars-ui exclusive                            |
| On open change    | `on_open_change`    | `onOpenChange`  | `onOpenChange`      | --                                          |
| On action         | `on_action`         | `onSelect`      | per-item `onSelect` | --                                          |
| Modal mode        | --                  | --              | `modal`             | Radix exclusive                             |
| Direction         | --                  | --              | `dir`               | Radix explicit; ars-ui resolves from locale |

**Gaps:** None.

### 5.2 Anatomy

| Part           | ars-ui                      | Ark UI                   | Radix UI                            | Notes                                                        |
| -------------- | --------------------------- | ------------------------ | ----------------------------------- | ------------------------------------------------------------ |
| Root           | `Root`                      | `Root`                   | `Root`                              | --                                                           |
| Target         | `Target`                    | `ContextTrigger`         | `Trigger`                           | ars-ui names it `Target` to distinguish from button triggers |
| Positioner     | `Positioner`                | `Positioner`             | `Portal`                            | --                                                           |
| Arrow          | `Arrow`                     | `Arrow` + `ArrowTip`     | `Arrow`                             | --                                                           |
| Content        | `Content`                   | `Content`                | `Content`                           | --                                                           |
| Item           | `Item`                      | `Item`                   | `Item`                              | --                                                           |
| ItemText       | `ItemText`                  | `ItemText`               | --                                  | --                                                           |
| ItemIndicator  | `ItemIndicator`             | `ItemIndicator`          | `ItemIndicator`                     | --                                                           |
| ItemGroup      | `ItemGroup`                 | `ItemGroup`              | `Group`                             | --                                                           |
| ItemGroupLabel | `ItemGroupLabel`            | `ItemGroupLabel`         | `Label`                             | --                                                           |
| CheckboxItem   | `CheckboxItem`              | `CheckboxItem`           | `CheckboxItem`                      | --                                                           |
| RadioGroup     | `RadioGroup`                | `RadioItemGroup`         | `RadioGroup`                        | --                                                           |
| RadioItem      | `RadioItem`                 | `RadioItem`              | `RadioItem`                         | --                                                           |
| Separator      | `Separator`                 | `Separator`              | `Separator`                         | --                                                           |
| Sub (submenu)  | `SubTrigger` + `SubContent` | Yes (nested `Menu.Root`) | `Sub` + `SubTrigger` + `SubContent` | Adapter owns live submenu positioning                        |

**Gaps:** None.

### 5.3 Events

| Callback      | ars-ui                | Ark UI              | Radix UI                                     | Notes                                |
| ------------- | --------------------- | ------------------- | -------------------------------------------- | ------------------------------------ |
| Open change   | `on_open_change`      | `onOpenChange`      | `onOpenChange`                               | --                                   |
| Item action   | `on_action`           | `onSelect`          | per-item `onSelect`                          | --                                   |
| Escape key    | `Event::Close`        | `onEscapeKeyDown`   | `onEscapeKeyDown`                            | Radix exposes as interceptable event |
| Click outside | `Event::ClickOutside` | `onInteractOutside` | `onPointerDownOutside` / `onInteractOutside` | --                                   |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui | Ark UI                  | Radix UI                 |
| ----------------------- | ------ | ----------------------- | ------------------------ |
| Right-click trigger     | Yes    | Yes (`ContextTrigger`)  | Yes (`Trigger`)          |
| Shift+F10 keyboard open | Yes    | Yes                     | Yes                      |
| Action items            | Yes    | Yes                     | Yes                      |
| Checkbox items          | Yes    | Yes                     | Yes                      |
| Radio items             | Yes    | Yes                     | Yes                      |
| Typeahead               | Yes    | Yes                     | Yes                      |
| Pointer-positioned      | Yes    | Yes                     | Yes                      |
| Disabled items          | Yes    | Yes                     | Yes                      |
| Separator               | Yes    | Yes                     | Yes                      |
| Submenus                | Yes    | Yes (via nested `Menu`) | Yes (`Sub`/`SubContent`) |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses a separate `ContextMenu` component; Ark UI uses the same `Menu` component with a `ContextTrigger` part; (2) ars-ui names the trigger area `Target` (not `Trigger`) to distinguish from button-click triggers.
- **Recommended additions:** None.
