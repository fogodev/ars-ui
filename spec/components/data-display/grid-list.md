---
component: GridList
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility, collections, interactions]
shared_deps: []
related: []
references:
    react-aria: GridList
---

# GridList

A keyboard-navigable grid of items with two-dimensional arrow key navigation and selection.
GridList renders items in a grid layout using `role="grid"` and supports single, multiple,
and no-selection modes. Unlike Table, GridList does not require columnar data — each cell is
an opaque item. Maps to React Aria's `GridList`.

## 1. State Machine

### 1.1 States

| State     | Description                                            |
| --------- | ------------------------------------------------------ |
| `Idle`    | No item is focused.                                    |
| `Focused` | An item within the grid has keyboard or pointer focus. |

### 1.2 Events

| Event             | Payload                               | Description                                           |
| ----------------- | ------------------------------------- | ----------------------------------------------------- |
| `Focus`           | `key: Option<Key>, is_keyboard: bool` | Focus entered the grid or moved to a specific item.   |
| `Blur`            | —                                     | Focus left the grid entirely.                         |
| `Select`          | `Key`                                 | Select a single item (replace current selection).     |
| `ToggleSelect`    | `Key`                                 | Toggle the selected state of a single item.           |
| `SelectRange`     | `from: Key, to: Key`                  | Select all items between `from` and `to` (inclusive). |
| `FocusUp`         | —                                     | Move focus one row up.                                |
| `FocusDown`       | —                                     | Move focus one row down.                              |
| `FocusLeft`       | —                                     | Move focus one cell left.                             |
| `FocusRight`      | —                                     | Move focus one cell right.                            |
| `FocusFirst`      | —                                     | Move focus to the first item (Home).                  |
| `FocusLast`       | —                                     | Move focus to the last item (End).                    |
| `SelectAll`       | —                                     | Select all non-disabled items (Ctrl+A).               |
| `ClearSelection`  | —                                     | Deselect all items.                                   |
| `ItemAction`      | `Key`                                 | Primary action triggered on an item (Enter key).      |
| `TypeaheadSearch` | `ch: char, now: Duration`             | Focus the next enabled item matching typed text.      |
| `SyncProps`       | —                                     | Refresh context from changed props.                   |

### 1.3 Context

```rust
/// Context for the GridList component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Flat list of grid items, laid out left-to-right, top-to-bottom.
    pub items: StaticCollection<ItemDef>,
    /// Number of columns in the grid layout.
    pub columns: NonZeroUsize,
    /// Key of the currently focused item, if any.
    pub focused_key: Option<Key>,
    /// True when focus was keyboard-initiated (drives visible focus ring).
    pub focus_visible: bool,
    /// Selection mode for grid items.
    pub selection_mode: selection::Mode,
    /// Selection behavior for selectable grid items.
    pub selection_behavior: selection::Behavior,
    /// Currently selected item keys.
    pub selected_keys: Bindable<BTreeSet<Key>>,
    /// Latest user-requested selected keys for adapter change notification.
    pub requested_selected_keys: Option<BTreeSet<Key>>,
    /// Latest user-requested action key for adapter change notification.
    pub requested_action_key: Option<Key>,
    /// When true, all items are non-interactive.
    pub disabled: bool,
    /// Keys of individually disabled items.
    pub disabled_keys: BTreeSet<Key>,
    /// Whether deselecting the final selected item is blocked.
    pub disallow_empty_selection: bool,
    /// Escape key behavior.
    pub escape_key_behavior: EscapeKeyBehavior,
    /// Whether composite grid focus semantics are active.
    pub composite: bool,
    /// Whether a load-more request is currently in flight. Copied from props;
    /// drives `data-ars-loading` on the `LoadingSentinel` part.
    pub loading: bool,
    /// Whether drag-and-drop attrs are enabled.
    pub dnd_enabled: bool,
    /// Component IDs used for hydration-stable relationships.
    pub ids: ComponentIds,
    /// Typeahead search state.
    pub typeahead: typeahead::State,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for position announcements.
    pub messages: Messages,
}

/// Definition of a grid item.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemDef {
    /// Unique identifier for this grid item.
    pub key: Key,
    /// Localized text label used for typeahead and adapter-rendered content.
    pub label: String,
    /// Whether this individual item is disabled.
    pub disabled: bool,
    /// Optional navigation URL. When `Some`, the cell renders as an `<a>` element
    /// instead of the default element, enabling native link behavior (middle-click
    /// to open in new tab, etc.). The `role="gridcell"` / `role="row"` is preserved
    /// on the `<a>`. Stored as `SafeUrl` so URL-valued output attributes are
    /// validated before they reach `cell_attrs`.
    pub href: Option<SafeUrl>,
}
```

### 1.4 Props

```rust
/// Controls behavior when Escape is pressed while items are selected.
#[derive(Clone, Debug, PartialEq)]
pub enum EscapeKeyBehavior {
    /// Escape clears the current selection.
    ClearSelection,
    /// Escape is not handled by the GridList.
    None,
}

/// Props for the GridList component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Grid items to display.
    pub items: StaticCollection<ItemDef>,
    /// Number of columns. Defaults to the number of items (single row).
    pub columns: Option<NonZeroUsize>,
    /// Controlled selected keys.
    pub selected_keys: Option<BTreeSet<Key>>,
    /// Default selected keys for uncontrolled mode.
    pub default_selected_keys: BTreeSet<Key>,
    /// Selection mode.
    pub selection_mode: selection::Mode,
    /// Controls whether clicking a selected item deselects it (`Toggle`) or whether
    /// the user must hold Ctrl/Cmd to toggle (`Replace`). Default: `Toggle`.
    /// See `selection::Behavior` in `11-selection-components.md` §0.1.
    pub selection_behavior: selection::Behavior,
    /// Disable the entire grid.
    pub disabled: bool,
    /// Keys of individually disabled items.
    pub disabled_keys: BTreeSet<Key>,
    /// Accessible label for the grid.
    pub label: Option<String>,
    /// Prevents deselecting the last remaining selected item. When `true` and the user
    /// attempts to deselect the only selected item, the action is a no-op.
    pub disallow_empty_selection: bool,
    /// Controls behavior when Escape is pressed while items are selected.
    /// `ClearSelection` (default) deselects all; `None` disables Escape handling.
    pub escape_key_behavior: EscapeKeyBehavior,
    /// When `true` (default), the grid uses a single tab stop with internal
    /// arrow-key navigation (`role="grid"`). When `false`, each item is an
    /// individual tab stop (`role="list"` with `role="listitem"` children).
    ///
    /// - `composite: true` — standard grid keyboard pattern: Tab moves focus
    ///   into/out of the grid; arrow keys navigate between cells.
    /// - `composite: false` — each item participates in the tab order
    ///   independently; useful when items contain complex interactive content
    ///   that needs its own focus management.
    pub composite: bool,
    /// Localizable messages for position announcements.
    /// Callback invoked when the loading sentinel enters the viewport,
    /// signalling that the next page of items should be loaded. When `None`,
    /// the `LoadingSentinel` part is not rendered. See `06-collections.md` §5.3.
    pub on_load_more: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Callback invoked when an enabled item receives a primary action.
    pub on_action: Option<Callback<dyn Fn(Key) + Send + Sync>>,
    /// Whether a load-more request is currently in flight. When `true`, the
    /// `LoadingSentinel` element receives `data-ars-loading="true"` so the
    /// adapter can display a spinner. Default: `false`.
    pub loading: bool,
    /// Enable the agnostic drag-and-drop affordance attrs. Default: `false`.
    pub dnd_enabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: StaticCollection::default(),
            columns: None,
            selected_keys: None,
            default_selected_keys: BTreeSet::new(),
            selection_mode: selection::Mode::None,
            selection_behavior: selection::Behavior::Toggle,
            disabled: false,
            disabled_keys: BTreeSet::new(),
            disallow_empty_selection: false,
            escape_key_behavior: EscapeKeyBehavior::ClearSelection,
            label: None,
            composite: true,
            on_load_more: None,
            on_action: None,
            loading: false,
            dnd_enabled: false,
        }
    }
}
```

### 1.5 Transition Semantics

GridList uses the standard state-machine boundary from `foundation/00-architecture.md`:
`Props` configure the instance, `Context` owns durable machine state, `Event` drives
transitions, and `Api` exposes adapter-safe attributes and keyboard dispatch helpers.

The machine has two states:

- `Idle`: no cell is focused inside the grid list.
- `Focused`: a present, enabled cell is focused or represented by `aria-activedescendant`.

The machine accepts the public events listed in §1.2. Implementations must preserve these
semantics:

- Columns initialize from `Props::columns` when present; otherwise they use the item count,
  clamped to at least one column.
- `selected_keys` is a `Bindable<BTreeSet<Key>>`. Controlled values are filtered to present,
  enabled item keys, while `requested_selected_keys` preserves the latest user-requested
  selection so adapters can notify parents even when controlled props lag.
- `requested_action_key` preserves the latest enabled item action key so adapters can notify
  consumers even when no selection changed.
- Disabled grids reject focus, selection, action, typeahead, and navigation mutations. Disabled
  or stale item keys are rejected for focus, selection, and action events.
- Arrow, Home, and End navigation operate over the flat collection projected into rows and
  columns. Horizontal navigation never wraps between rows. Navigation skips disabled candidates
  when another enabled candidate exists in the requested direction.
- `Enter` maps to `ItemAction`, `Space` maps to selection toggling, Ctrl/Cmd+A selects all
  enabled items in multiple-selection mode, and `Escape` follows `escape_key_behavior`.
- Selection obeys `selection::Mode`, `selection::Behavior`, and
  `disallow_empty_selection`. Range and select-all selection include only enabled, present item
  keys.
- Printable-character typeahead uses the shared collection typeahead behavior, ignores
  composition and control-modified input, and focuses matching enabled items.
- `SyncProps` refreshes items, disabled keys, columns, selected-key control state,
  `loading`, `composite`, `dnd_enabled`, and component IDs, dropping focus when the grid becomes
  disabled or when the focused key is no longer present or enabled.
- `ItemAction` records `requested_action_key` and emits the `Action` effect so adapters can invoke
  `on_action`. Link activation, DOM focus, scroll-into-view, live announcements, pointer hit
  testing, native drag events, and load-more observation stay in adapters.

### 1.6 Connect / API

```rust,no_check
#[derive(ComponentPart)]
#[scope = "grid-list"]
pub enum Part {
    Root,
    Row { key: Key },
    Cell { key: Key },
    LoadingSentinel,
    DragHandle { key: Key },
    DropIndicator,
}

/// Adapter-facing API for the GridList component.
pub struct Api<'a> { /* private fields */ }

impl Api<'_> {
    /// Returns the current localized grid-dimensions description.
    pub fn grid_dimensions(&self) -> String;
    /// Returns the localized row/column description for `key`.
    pub fn cell_position(&self, key: &Key) -> Option<String>;
    /// Returns the hydration-stable cell ID for `key`.
    pub fn cell_id(&self, key: &Key) -> Option<String>;
    /// Returns the latest user-requested selected keys.
    pub const fn requested_selected_keys(&self) -> Option<&BTreeSet<Key>>;
    /// Returns the latest user-requested action key.
    pub const fn requested_action_key(&self) -> Option<&Key>;
    /// Returns the current item collection.
    pub const fn items(&self) -> &StaticCollection<ItemDef>;
    /// Returns root attributes.
    pub fn root_attrs(&self) -> AttrMap;
    /// Returns row attributes for `key`.
    pub fn row_attrs(&self, key: &Key) -> AttrMap;
    /// Returns cell attributes for `key`.
    pub fn cell_attrs(&self, key: &Key) -> AttrMap;
    /// Returns loading sentinel attributes when `on_load_more` is configured.
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap>;
    /// Returns drag handle attributes for `key`.
    pub fn drag_handle_attrs(&self, key: &Key) -> AttrMap;
    /// Returns drop indicator attributes for `target`.
    pub fn drop_indicator_attrs(&self, target: &CollectionDropTarget) -> AttrMap;
    /// Dispatches a keydown event using a timeout-advancing fallback timestamp.
    pub fn on_cell_keydown(&self, key: &Key, data: &KeyboardEventData);
    /// Dispatches a keydown event with an adapter-provided monotonic timestamp.
    pub fn on_cell_keydown_at(&self, key: &Key, data: &KeyboardEventData, now: Duration);
}

impl ConnectApi for Api<'_> {
    type Part = Part;
}
```

## 2. Anatomy

```text
GridList
├── Root              (container; data-ars-scope="grid-list" data-ars-part="root"; role="grid")
├── Row               (logical row wrapper; role="row"; data-ars-part="row")
│   ├── Cell          (individual item; role="gridcell"; <a> when href present; data-ars-part="cell")
│   └── ...
├── ...
├── LoadingSentinel   (optional; after last Row; rendered only when on_load_more is set)
├── DragHandle        (optional; drag-and-drop variant)
└── DropIndicator     (optional; drag-and-drop variant)
```

| Part              | Element                                                                      | Key Attributes                                                                                                                                                                                                                                         |
| ----------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Root`            | `<div>`                                                                      | `role="grid"`, `aria-label`, `aria-multiselectable`, `data-ars-state`, `data-ars-disabled`                                                                                                                                                             |
| `Row`             | `<div>`                                                                      | `role="row"`, `aria-selected`, `data-ars-key`, `data-ars-selected`                                                                                                                                                                                     |
| `Cell`            | `<div>` or `<a>` (when href)                                                 | `role="gridcell"`, `tabindex="-1"` in composite mode, independent `tabindex="0"` in non-composite mode, `aria-disabled`, `aria-roledescription` (when draggable), `data-ars-key`, `data-ars-selected`, `data-ars-focus-visible`, `href` (when present) |
| `LoadingSentinel` | `<div>` (optional, after last Row; rendered only when `on_load_more` is set) | `aria-hidden="true"`, `tabindex="-1"`, `data-ars-loading` (when loading)                                                                                                                                                                               |
| `DragHandle`      | `<button>`                                                                   | `role="button"`, `aria-label`, `aria-disabled`, `tabindex`                                                                                                                                                                                             |
| `DropIndicator`   | `<div>`                                                                      | `aria-hidden="true"`, `data-ars-drop-position`, `data-ars-drop-target`                                                                                                                                                                                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **Grid pattern**: GridList uses `role="grid"` at the root level, with `role="row"` grouping
  cells and `role="gridcell"` on each item. This provides two-dimensional keyboard navigation
  semantics to assistive technologies.
- **Composite tab stop**: In `composite: true`, the root is the single tab stop and cells use
  `tabindex="-1"` while `aria-activedescendant` names the active cell. In `composite: false`,
  each enabled cell is an independent tab stop.
- **Selection announcement**: `aria-selected` on rows reflects selection state. When
  `selection_mode` is `Multiple`, the root has `aria-multiselectable="true"`.
- **Disabled items**: `aria-disabled="true"` on individual cells. Disabled items are skipped
  during 2D navigation when possible, but remain discoverable by AT.
- **`aria-activedescendant`**: The root element sets `aria-activedescendant` to the ID of the
  currently focused cell. When `composite: true`, the root element receives focus and
  `aria-activedescendant` points to the visually focused cell.
- **Action vs. selection**: Enter triggers `on_action` (primary action), while Space toggles
  selection. This mirrors the ARIA grid pattern where Enter activates and Space selects.
- **Home/End wrapping**: `Home` moves focus to the first non-disabled item in the grid. `End`
  moves focus to the last non-disabled item. When combined with `Ctrl`, `Ctrl+Home` moves to
  the first item in the first row, and `Ctrl+End` moves to the last item in the last row.
  In 2D navigation, the focus does **not** wrap between rows -- pressing ArrowRight on the last
  cell in a row does not move to the first cell of the next row. This prevents disorientation
  in grid layouts. ArrowUp/ArrowDown navigate within the same column across rows.
- **Composite mode** (`composite` prop):
  - `composite: true` (default) — `role="grid"` on root. Single tab stop; arrow keys navigate
      between cells. This is the standard ARIA grid keyboard pattern.
  - `composite: false` — `role="list"` on root, `role="listitem"` on each item. Each item is
      an independent tab stop. Arrow keys are not captured. Useful when grid items contain complex
      interactive content (e.g., embedded forms, multi-action cards) that needs its own focus
      management.
- **Link items** (`href` on `ItemDef`): When a grid item has an `href`, the adapter renders the
  `Cell` as an `<a>` element. The `role="gridcell"` on the cell and `role="row"` on the parent
  row are preserved -- the `<a>` carries the grid role, not `role="link"`. This allows native
  link semantics (middle-click to open in new tab, Ctrl+click, etc.) while maintaining the
  ARIA grid pattern. Enter activates the link navigation; Space continues to toggle selection.
- **Loading sentinel**: The `LoadingSentinel` element is `aria-hidden="true"` and excluded from
  the tab order (`tabindex="-1"`). It serves purely as an `IntersectionObserver` target for
  infinite scroll. Loading state changes (loading started/completed, new items available) are
  communicated to screen readers via a `LiveAnnouncer` managed by the adapter layer, not via
  ARIA attributes on the sentinel itself. See `06-collections.md` section 5.3.

### 3.2 Keyboard Interaction

| Key           | Action                                                                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Tab`         | Focus enters the grid on the first (or last-focused) cell; Tab again exits.                                                                      |
| `ArrowUp`     | Move focus one row up (same column).                                                                                                             |
| `ArrowDown`   | Move focus one row down (same column).                                                                                                           |
| `ArrowLeft`   | Move focus one cell to the left.                                                                                                                 |
| `ArrowRight`  | Move focus one cell to the right.                                                                                                                |
| `Home`        | Move focus to the first item in the grid.                                                                                                        |
| `End`         | Move focus to the last item in the grid.                                                                                                         |
| `Space`       | Toggle selection of the focused item.                                                                                                            |
| `Enter`       | Activate the focused item (primary action).                                                                                                      |
| `Ctrl+A`      | Select all non-disabled items (multiple selection mode only).                                                                                    |
| `Escape`      | Clear selection.                                                                                                                                 |
| `Shift+Arrow` | Extend selection in the arrow direction (multiple selection mode). In single-selection mode, Shift+Arrow moves focus without changing selection. |

## 4. Internationalization

- Grid item labels must come from a localized message catalog; GridList itself does not
  generate textual content beyond ARIA attributes.
- **RTL**: In right-to-left locales, ArrowLeft and ArrowRight are reversed: ArrowLeft moves
  focus to the _next_ cell (visually right-to-left) and ArrowRight moves focus to the
  _previous_ cell. The adapter applies `dir="rtl"` on the root element and the keyboard
  handler swaps Left/Right semantics accordingly.
- Row and cell position announcements should use ordinal formatting from `ars-i18n` when
  available:

```rust
/// Messages for the GridList component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Describes the focused cell position, e.g. "Row 2, Column 3".
    pub cell_position: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
    /// Describes total grid dimensions, e.g. "4 rows, 3 columns".
    pub grid_dimensions: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
    /// Accessible label template for an item's drag handle.
    pub drag_handle_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Role description for draggable items.
    pub draggable: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            cell_position: MessageFn::new(|row, col, _locale| format!("Row {row}, Column {col}")),
            grid_dimensions: MessageFn::new(|rows, cols, _locale| format!("{rows} rows, {cols} columns")),
            drag_handle_label: MessageFn::new(|label, _locale| format!("Drag {label}")),
            draggable: MessageFn::static_str("draggable"),
        }
    }
}
impl ComponentMessages for Messages {}
```

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, GridList surfaces the agnostic drag-and-drop affordance attrs described in `06-collections.md` §10: cells receive draggable role-description attrs, `DragHandle` exposes the keyboard-accessible drag affordance, and `DropIndicator` encodes a resolved `CollectionDropTarget`. Adapters wire `DraggableCollection<T>` / `DroppableCollection<T>`, pointer hit-testing, keyboard drag protocol, live announcements, and `CollectionDndEvent::Reorder` completion callbacks.

## 5. Library Parity

> Compared against: React Aria (`GridList`).

### 5.1 Props

| Feature                                   | ars-ui                      | React Aria                     | Notes                                                                           |
| ----------------------------------------- | --------------------------- | ------------------------------ | ------------------------------------------------------------------------------- |
| `layout`                                  | Implicit via `columns`      | `'stack' \| 'grid'`            | `columns=1` is equivalent to `stack`                                            |
| `selection_mode`                          | `selection::Mode`           | `SelectionMode`                | Equivalent                                                                      |
| `selected_keys` / `default_selected_keys` | `BTreeSet<Key>`             | `Iterable<Key> \| 'all'`       | Equivalent                                                                      |
| `items`                                   | `StaticCollection<ItemDef>` | `Iterable<T>`                  | Equivalent                                                                      |
| `disabled_keys`                           | `BTreeSet<Key>`             | `Iterable<Key>`                | Equivalent                                                                      |
| `disallow_empty_selection`                | `bool`                      | --                             | Added for consistency with Table                                                |
| `escape_key_behavior`                     | `EscapeKeyBehavior`         | --                             | Added for consistency with Table                                                |
| `drag_and_drop_hooks`                     | Via `06-collections.md` §10 | `DragAndDropHooks`             | Both support DnD                                                                |
| `render_empty_state`                      | --                          | `() => ReactNode`              | Adapter-layer concern; the adapter decides what to render when `items` is empty |
| `selection_behavior`                      | `selection::Behavior`       | `SelectionBehavior` (implicit) | Equivalent                                                                      |

**Gaps:** None.

### 5.2 Anatomy

| Part          | ars-ui            | React Aria             | Notes                                                 |
| ------------- | ----------------- | ---------------------- | ----------------------------------------------------- |
| Root          | `Root`            | `GridList`             | --                                                    |
| Row           | `Row`             | --                     | ars-ui adds explicit row grouping                     |
| Cell          | `Cell`            | `GridListItem`         | --                                                    |
| Section       | --                | `GridListSection`      | ars-ui handles grouping at the adapter level          |
| Header        | --                | `GridListHeader`       | Section headers are adapter-level                     |
| LoadMore      | `LoadingSentinel` | `GridListLoadMoreItem` | Same concept                                          |
| DragHandle    | `DragHandle`      | `DragAndDropHooks`     | ars-ui exposes a named handle part for DnD affordance |
| DropIndicator | `DropIndicator`   | `DragAndDropHooks`     | ars-ui exposes a named visual drop-target part        |
| Checkbox      | --                | `Checkbox`             | ars-ui manages selection via `aria-selected` on cells |

**Gaps:** None. Section grouping is an adapter-level concern in ars-ui.

### 5.3 Events

| Callback              | ars-ui                               | React Aria          | Notes                 |
| --------------------- | ------------------------------------ | ------------------- | --------------------- |
| `on_selection_change` | Adapter layer (Bindable observation) | `onSelectionChange` | Equivalent            |
| `on_action`           | `ItemAction` event                   | `onAction`          | Added from React Aria |

**Gaps:** None.

### 5.4 Features

| Feature                   | ars-ui                  | React Aria                   |
| ------------------------- | ----------------------- | ---------------------------- |
| Stack / grid layout       | Yes (via `columns`)     | Yes (`layout` prop)          |
| Single/multiple selection | Yes                     | Yes                          |
| Disabled items            | Yes                     | Yes                          |
| 2D keyboard navigation    | Yes                     | Yes                          |
| Item links (`href`)       | Yes                     | Yes                          |
| Infinite scroll           | Yes (`LoadingSentinel`) | Yes (`GridListLoadMoreItem`) |
| Drag and drop             | Yes (via collections)   | Yes (`dragAndDropHooks`)     |
| Empty state               | Adapter-level           | `renderEmptyState`           |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria provides `renderEmptyState` as a render prop; ars-ui leaves empty state rendering to the adapter since the core machine is render-agnostic. React Aria's `GridListSection`/`GridListHeader` for grouping are adapter-level concerns in ars-ui.
- **Recommended additions:** None.
