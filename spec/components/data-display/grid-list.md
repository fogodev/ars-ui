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

| Event            | Payload                               | Description                                           |
| ---------------- | ------------------------------------- | ----------------------------------------------------- |
| `Focus`          | `key: Option<Key>, is_keyboard: bool` | Focus entered the grid or moved to a specific item.   |
| `Blur`           | —                                     | Focus left the grid entirely.                         |
| `Select`         | `Key`                                 | Select a single item (replace current selection).     |
| `ToggleSelect`   | `Key`                                 | Toggle the selected state of a single item.           |
| `SelectRange`    | `from: Key, to: Key`                  | Select all items between `from` and `to` (inclusive). |
| `FocusUp`        | —                                     | Move focus one row up.                                |
| `FocusDown`      | —                                     | Move focus one row down.                              |
| `FocusLeft`      | —                                     | Move focus one cell left.                             |
| `FocusRight`     | —                                     | Move focus one cell right.                            |
| `FocusFirst`     | —                                     | Move focus to the first item (Home).                  |
| `FocusLast`      | —                                     | Move focus to the last item (End).                    |
| `SelectAll`      | —                                     | Select all non-disabled items (Ctrl+A).               |
| `ClearSelection` | —                                     | Deselect all items.                                   |
| `ItemAction`     | `Key`                                 | Primary action triggered on an item (Enter key).      |

### 1.3 Context

```rust
/// Context for the GridList component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Flat list of grid items, laid out left-to-right, top-to-bottom.
    pub items: StaticCollection<ItemDef>,
    /// Number of columns in the grid layout.
    pub columns: NonZero<usize>,
    /// Key of the currently focused item, if any.
    pub focused_key: Option<Key>,
    /// True when focus was keyboard-initiated (drives visible focus ring).
    pub focus_visible: bool,
    /// Selection mode for grid items.
    pub selection_mode: selection::Mode,
    /// Currently selected item keys.
    pub selected_keys: Bindable<BTreeSet<Key>>,
    /// When true, all items are non-interactive.
    pub disabled: bool,
    /// Keys of individually disabled items.
    pub disabled_keys: BTreeSet<Key>,
    /// Whether a load-more request is currently in flight. Copied from props;
    /// drives `data-ars-loading` on the `LoadingSentinel` part.
    pub loading: bool,
    /// Unique component instance identifier.
    pub id: ComponentId,
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
    /// Whether this individual item is disabled.
    pub disabled: bool,
    /// Optional navigation URL. When `Some`, the cell renders as an `<a>` element
    /// instead of the default element, enabling native link behavior (middle-click
    /// to open in new tab, etc.). The `role="gridcell"` / `role="row"` is preserved
    /// on the `<a>`.
    pub href: Option<String>,
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
    pub columns: Option<NonZero<usize>>,
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
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable messages for position announcements.
    pub messages: Option<Messages>,
    /// Callback invoked when the loading sentinel enters the viewport,
    /// signalling that the next page of items should be loaded. When `None`,
    /// the `LoadingSentinel` part is not rendered. See `06-collections.md` §5.3.
    pub on_load_more: Option<Callback<()>>,
    /// Whether a load-more request is currently in flight. When `true`, the
    /// `LoadingSentinel` element receives `data-ars-loading="true"` so the
    /// adapter can display a spinner. Default: `false`.
    pub loading: bool,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: Vec::new(),
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
            locale: None,
            messages: None,
            on_load_more: None,
            loading: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// States for the GridList component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No item is focused.
    Idle,
    /// An item within the grid has keyboard or pointer focus.
    Focused,
}

/// Events for the GridList component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus entered the grid or moved to a specific item.
    Focus { key: Option<Key>, is_keyboard: bool },
    /// Focus left the grid entirely.
    Blur,
    /// Select a single item (replace current selection).
    Select(Key),
    /// Toggle the selected state of a single item.
    ToggleSelect(Key),
    /// Select all items between `from` and `to` (inclusive).
    SelectRange { from: Key, to: Key },
    /// Move focus one row up.
    FocusUp,
    /// Move focus one row down.
    FocusDown,
    /// Move focus one cell left.
    FocusLeft,
    /// Move focus one cell right.
    FocusRight,
    /// Move focus to the first item (Home).
    FocusFirst,
    /// Move focus to the last item (End).
    FocusLast,
    /// Select all non-disabled items (Ctrl+A).
    SelectAll,
    /// Deselect all items.
    ClearSelection,
    /// Primary action triggered on an item (Enter key).
    ItemAction(Key),
}

/// Machine for the GridList component.
pub struct Machine;

impl Machine {
    /// Compute the (column, row) position from a flat index.
    fn index_to_pos(index: usize, columns: NonZero<usize>) -> (usize, usize) {
        let cols = columns.get();
        (index % cols, index / cols)
    }

    /// Compute the flat index from (column, row) position.
    fn pos_to_index(col: usize, row: usize, columns: NonZero<usize>) -> usize {
        row * columns.get() + col
    }

    /// Find the index of the item with the given key.
    fn key_index(items: &StaticCollection<ItemDef>, key: &Key) -> Option<usize> {
        items.iter().position(|item| &item.key == key)
    }

    /// Check if an item key is disabled.
    fn is_disabled(ctx: &Context, key: &Key) -> bool {
        ctx.disabled || ctx.disabled_keys.contains(key)
            || ctx.items.iter().find(|i| &i.key == key).map_or(false, |i| i.disabled)
    }
}

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let columns = props.columns.unwrap_or(props.items.len().max(1));
        (State::Idle, Context {
            items: props.items.clone(),
            columns,
            focused_key: None,
            focus_visible: false,
            selection_mode: props.selection_mode.clone(),
            selected_keys: match &props.selected_keys {
                Some(keys) => Bindable::controlled(keys.clone()),
                None       => Bindable::uncontrolled(props.default_selected_keys.clone()),
            },
            disabled: props.disabled,
            disabled_keys: props.disabled_keys.clone(),
            loading: props.loading,
            id: ComponentId::new(),
            locale,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx:   &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return match event {
                Event::Focus { .. } | Event::Blur => {
                    Some(TransitionPlan::to(State::Idle))
                }
                _ => None,
            };
        }

        match event {
            // ── Focus ────────────────────────────────────────────────────
            Event::Focus { key, is_keyboard } => {
                let target_key = key.clone().or_else(|| {
                    ctx.items.iter()
                        .find(|i| !Self::is_disabled(ctx, &i.key))
                        .map(|i| i.key.clone())
                });
                let kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = target_key;
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused_key = None;
                    ctx.focus_visible = false;
                }))
            }

            // ── Selection ────────────────────────────────────────────────
            Event::Select(key) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                if Self::is_disabled(ctx, key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selected_keys.set(BTreeSet::from([key]));
                }))
                // Selection change notification is handled by the adapter layer.
            }

            Event::ToggleSelect(key) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                if Self::is_disabled(ctx, &key) { return None; }
                let is_selected = ctx.selected_keys.get().contains(&key);
                Some(TransitionPlan::context_only(move |ctx| {
                    if is_selected {
                        ctx.selected_keys.get_mut_owned().retain(|k| *k != key);
                    } else {
                        if ctx.selection_mode == selection::Mode::Single {
                            ctx.selected_keys.set(BTreeSet::from([key]));
                        } else {
                            ctx.selected_keys.get_mut_owned().insert(key);
                        }
                    }
                }))
                // Selection change notification is handled by the adapter layer.
            }

            Event::SelectRange { from, to } => {
                if ctx.selection_mode != selection::Mode::Multiple { return None; }
                let from_idx = Self::key_index(&ctx.items, from)?;
                let to_idx   = Self::key_index(&ctx.items, to)?;
                let (lo, hi) = if from_idx <= to_idx { (from_idx, to_idx) } else { (to_idx, from_idx) };
                let range_keys = ctx.items[lo..=hi].iter()
                    .filter(|i| !Self::is_disabled(ctx, &i.key))
                    .map(|i| i.key.clone())
                    .collect::<BTreeSet<_>>();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selected_keys.set(range_keys);
                }))
                // Selection change notification is handled by the adapter layer.
            }

            Event::SelectAll => {
                if ctx.selection_mode != selection::Mode::Multiple { return None; }
                let all_keys = ctx.items.iter()
                    .filter(|i| !Self::is_disabled(ctx, &i.key))
                    .map(|i| i.key.clone())
                    .collect::<BTreeSet<_>>();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selected_keys.set(all_keys);
                }))
                // Selection change notification is handled by the adapter layer.
            }

            Event::ClearSelection => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.selected_keys.set(BTreeSet::new());
                }))
                // Selection change notification is handled by the adapter layer.
            }

            // ── Item Action ─────────────────────────────────────────────
            Event::ItemAction(key) => {
                if Self::is_disabled(ctx, key) { return None; }
                // ItemAction is a notification event — the adapter fires the
                // on_action callback. No context mutation needed.
                Some(TransitionPlan::context_only(|_ctx| {}))
            }

            // ── 2D Navigation ────────────────────────────────────────────
            Event::FocusUp => {
                let current = ctx.focused_key.as_ref()?;
                let idx = Self::key_index(&ctx.items, current)?;
                let (col, row) = Self::index_to_pos(idx, ctx.columns);
                if row == 0 { return None; }
                let target_idx = Self::pos_to_index(col, row - 1, ctx.columns);
                let target_key = ctx.items.get(target_idx)?.key.clone();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusDown => {
                let current = ctx.focused_key.as_ref()?;
                let idx = Self::key_index(&ctx.items, current)?;
                let (col, row) = Self::index_to_pos(idx, ctx.columns);
                let target_idx = Self::pos_to_index(col, row + 1, ctx.columns);
                let target_key = ctx.items.get(target_idx)?.key.clone();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusLeft => {
                let current = ctx.focused_key.as_ref()?;
                let idx = Self::key_index(&ctx.items, current)?;
                if idx == 0 { return None; }
                let target_key = ctx.items.get(idx - 1)?.key.clone();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusRight => {
                let current = ctx.focused_key.as_ref()?;
                let idx = Self::key_index(&ctx.items, current)?;
                let target_key = ctx.items.get(idx + 1)?.key.clone();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusFirst => {
                let first = ctx.items.iter()
                    .find(|i| !Self::is_disabled(ctx, &i.key))
                    .map(|i| i.key.clone())?;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(first);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusLast => {
                let last = ctx.items.iter().rev()
                    .find(|i| !Self::is_disabled(ctx, &i.key))
                    .map(|i| i.key.clone())?;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(last);
                    ctx.focus_visible = true;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx:   &'a Context,
        props: &'a Props,
        send:  &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "grid-list"]
pub enum Part {
    Root,
    Row { key: Key },
    Cell { key: Key },
    LoadingSentinel,
}

/// API for the GridList component.
pub struct Api<'a> {
    /// Current state of the grid list.
    state: &'a State,
    /// Current context of the grid list.
    ctx:   &'a Context,
    /// Current props of the grid list.
    props: &'a Props,
    /// Send event to the grid list.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns the grid dimensions description (e.g. "4 rows, 3 columns").
    pub fn grid_dimensions(&self) -> String {
        let cols = self.ctx.columns.get();
        let rows = (self.ctx.items.len() + cols - 1) / cols;
        (self.ctx.messages.grid_dimensions)(rows, cols, &self.ctx.locale)
    }

    /// Returns the cell position description (e.g. "Row 2, Column 3").
    pub fn cell_position(&self, key: &Key) -> Option<String> {
        let idx = Machine::key_index(&self.ctx.items, key)?;
        let (col, row) = Machine::index_to_pos(idx, self.ctx.columns);
        Some((self.ctx.messages.cell_position)(row + 1, col + 1, &self.ctx.locale))
    }

    /// Root attributes for the grid list.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "grid");
        if let Some(label) = &self.props.label {
            p.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        if self.ctx.selection_mode == selection::Mode::Multiple {
            p.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }
        if self.ctx.disabled {
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if let Some(col_count) = self.props.column_count {
            p.set(HtmlAttr::Aria(AriaAttr::ColCount), col_count.to_string());
        }
        if let Some(row_count) = self.props.row_count {
            p.set(HtmlAttr::Aria(AriaAttr::RowCount), row_count.to_string());
        }
        p.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
        });
        p
    }

    /// Row attributes for the grid list.
    pub fn row_attrs(&self, key: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Row { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-key"), key.to_string());
        p.set(HtmlAttr::Role, "row");

        let is_selected = self.ctx.selected_keys.get().contains(key);
        if is_selected {
            p.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
            p.set_bool(HtmlAttr::Data("ars-selected"), true);
        }
        p
    }

    /// Cell attributes for the grid list.
    pub fn cell_attrs(&self, key: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Cell { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-key"), key.to_string());
        p.set(HtmlAttr::Role, "gridcell");

        let is_focused = self.ctx.focused_key.as_ref() == Some(key);
        let is_disabled = Machine::is_disabled(self.ctx, key);
        let is_selected = self.ctx.selected_keys.get().contains(key);

        // Roving tabindex
        p.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if is_focused && self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        if is_disabled {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.selection_mode != selection::Mode::None {
            p.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        }
        if is_selected {
            p.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        // When the item has an href, set it so the adapter renders an `<a>` element.
        // The `role="gridcell"` is preserved on the `<a>`.
        let item = self.ctx.items.iter().find(|i| &i.key == key);
        if let Some(href) = item.and_then(|i| i.href.as_deref()) {
            p.set(HtmlAttr::Href, href);
        }

        // Event handlers (focus, blur, click, keydown for 2D grid navigation) are typed methods on the Api struct.
        p
    }

    /// Attributes for the loading sentinel element, rendered after the last item
    /// when `on_load_more` is configured. The framework adapter attaches an
    /// `IntersectionObserver` to this element; when it enters the viewport, the
    /// adapter invokes `on_load_more`. See `06-collections.md` §5.3.
    ///
    /// Returns `None` when `on_load_more` is `None` (no sentinel needed).
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap> {
        if self.props.on_load_more.is_none() { return None; }
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingSentinel.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.loading {
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        }
        Some(attrs)
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Row { key } => self.row_attrs(key),
            Part::Cell { key } => self.cell_attrs(key),
            Part::LoadingSentinel => self.loading_sentinel_attrs().unwrap_or_default(),
        }
    }
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
└── LoadingSentinel   (optional; after last Row; rendered only when on_load_more is set)
```

| Part              | Element                                                                      | Key Attributes                                                                                                                                |
| ----------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`            | `<div>`                                                                      | `role="grid"`, `aria-label`, `aria-multiselectable`, `data-ars-state`, `data-ars-disabled`                                                    |
| `Row`             | `<div>`                                                                      | `role="row"`, `aria-selected`, `data-ars-key`, `data-ars-selected`                                                                            |
| `Cell`            | `<div>` or `<a>` (when href)                                                 | `role="gridcell"`, `tabindex` (roving), `aria-disabled`, `data-ars-key`, `data-ars-selected`, `data-ars-focus-visible`, `href` (when present) |
| `LoadingSentinel` | `<div>` (optional, after last Row; rendered only when `on_load_more` is set) | `aria-hidden="true"`, `tabindex="-1"`, `data-ars-loading` (when loading)                                                                      |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **Grid pattern**: GridList uses `role="grid"` at the root level, with `role="row"` grouping
  cells and `role="gridcell"` on each item. This provides two-dimensional keyboard navigation
  semantics to assistive technologies.
- **Roving tabindex**: Only the currently focused cell has `tabindex="0"`; all others have
  `tabindex="-1"`. When no cell is focused, the first non-disabled cell receives `tabindex="0"`.
- **Selection announcement**: `aria-selected` on rows reflects selection state. When
  `selection_mode` is `Multiple`, the root has `aria-multiselectable="true"`.
- **Disabled items**: `aria-disabled="true"` on individual cells. Disabled items are skipped
  during 2D navigation when possible, but remain discoverable by AT.
- **`aria-activedescendant`**: The root element sets `aria-activedescendant` to the ID of the
  currently focused cell. This provides an alternative focus management strategy to roving
  tabindex for virtualized grids where not all cells are in the DOM. When `composite: true`,
  the root element receives focus and `aria-activedescendant` points to the visually focused cell.
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
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            cell_position: MessageFn::new(|row, col, _locale| format!("Row {row}, Column {col}")),
            grid_dimensions: MessageFn::new(|rows, cols, _locale| format!("{rows} rows, {cols} columns")),
        }
    }
}
impl ComponentMessages for Messages {}
```

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, GridList integrates with `DraggableCollection<T>` and `DroppableCollection<T>` from `06-collections.md` §7. This adds optional `DragHandle` and `DropIndicator` anatomy parts, and fires `CollectionDndEvent::Reorder` when items are drag-reordered.

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
| `drag_and_drop_hooks`                     | Via `06-collections.md`     | `DragAndDropHooks`             | Both support DnD                                                                |
| `render_empty_state`                      | --                          | `() => ReactNode`              | Adapter-layer concern; the adapter decides what to render when `items` is empty |
| `selection_behavior`                      | `selection::Behavior`       | `SelectionBehavior` (implicit) | Equivalent                                                                      |

**Gaps:** None.

### 5.2 Anatomy

| Part     | ars-ui            | React Aria             | Notes                                                 |
| -------- | ----------------- | ---------------------- | ----------------------------------------------------- |
| Root     | `Root`            | `GridList`             | --                                                    |
| Row      | `Row`             | --                     | ars-ui adds explicit row grouping                     |
| Cell     | `Cell`            | `GridListItem`         | --                                                    |
| Section  | --                | `GridListSection`      | ars-ui handles grouping at the adapter level          |
| Header   | --                | `GridListHeader`       | Section headers are adapter-level                     |
| LoadMore | `LoadingSentinel` | `GridListLoadMoreItem` | Same concept                                          |
| Checkbox | --                | `Checkbox`             | ars-ui manages selection via `aria-selected` on cells |

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
