---
component: Table
category: data-display
tier: complex
foundation_deps: [architecture, accessibility, collections, interactions]
shared_deps: []
related: []
references:
    react-aria: Table
---

# Table

A data table component with column sorting, single/multiple row selection, and expandable row
content. The Table renders semantic HTML table markup and optionally becomes a keyboard-
navigable `grid` for interactive use.

## 1. State Machine

### 1.1 States

| State  | Description                           |
| ------ | ------------------------------------- |
| `Idle` | No active sort; the table is at rest. |

#### 1.1.1 Sort State Single Source of Truth

Table sort state is managed exclusively through `Context.sort_descriptor: Bindable<Option<SortDescriptor>>`. There is NO `State::Sorting` variant in the state enum — the Table state machine uses only `Idle`, `Scrolling`, and other interaction states.

**Rationale**: Having both a `State::Sorting` variant and a `Context.sort_descriptor` created a sync race where the state and context could disagree about the current sort. By using `Context.sort_descriptor` as the single source of truth:

- External prop changes to `sort_descriptor` are immediately reflected
- No state/context synchronization logic needed
- The Bindable pattern handles controlled vs uncontrolled sort seamlessly

**Visual Feedback**: For components that need to show a sorting indicator (e.g., spinner while async sort completes), use `Context.is_sorting: bool`:

```rust
/// Context for the Table
#[derive(Clone, Debug)]
pub struct TableContext {
    /// The current sort configuration. Single source of truth for sort state.
    pub sort_descriptor: Bindable<Option<SortDescriptor>>,
    /// True while an async sort operation is in progress. Used for visual feedback only.
    pub is_sorting: bool,
    // ... other fields
}
```

When a sort column header is clicked:

1. Update `sort_descriptor` via Bindable (handles controlled/uncontrolled)
2. Set `is_sorting = true`
3. Emit `on_sort_change` callback
4. When sort completes (sync or async): set `is_sorting = false`

### 1.2 Events

| Event         | Payload                | Description                                   |
| ------------- | ---------------------- | --------------------------------------------- |
| `SortColumn`  | `column: String`       | Apply or toggle sort on the given column.     |
| `SelectRow`   | `Key` (row id)         | Mark a row as selected.                       |
| `DeselectRow` | `Key` (row id)         | Remove a row from the selection.              |
| `SelectAll`   | —                      | Select all rows (uses `selection::Set::All`). |
| `DeselectAll` | —                      | Clear all selected rows.                      |
| `ToggleRow`   | `Key` (row id)         | Flip selection state of one row.              |
| `ExpandRow`   | `Key` (row id)         | Show the expanded content for a row.          |
| `CollapseRow` | `Key` (row id)         | Hide the expanded content for a row.          |
| `Focus`       | `cell: (usize, usize)` | Move the logical grid focus to a cell.        |
| `Blur`        | —                      | Remove focus from the grid.                   |
| `RowAction`   | `Key` (row id)         | Primary action triggered on a row.            |

### 1.3 Context

```rust
/// Context for the Table component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Currently selected row IDs (uses canonical `selection::Set` from `ars_collections`).
    pub selected_rows: Bindable<selection::Set>,
    /// Full selection state machine (mode, behavior, anchor, focus, disabled keys).
    pub selection_state: selection::State,
    /// Currently expanded row IDs (expansion ≠ selection; remains `BTreeSet`).
    pub expanded_rows: Bindable<BTreeSet<Key>>,
    /// Unified sort state: which column is sorted and in which direction.
    /// `None` means no active sort. Uses `SortDescriptor` from `ars_collections`.
    pub sort_descriptor: Bindable<Option<SortDescriptor<String>>>,
    /// Grid focus position: (col_index, row_index). None when unfocused.
    pub focused_cell: Option<(usize, usize)>,
    /// Focused row ID for row-level keyboard navigation.
    pub focused_row: Option<Key>,
    /// Focused column index for cell-level keyboard navigation.
    pub focused_col: Option<usize>,
    /// Keys of individually disabled rows.
    pub disabled_keys: BTreeSet<Key>,
    /// Row selection mode.
    pub selection_mode: selection::Mode,
    /// When true, renders role="grid" with full keyboard navigation.
    pub interactive: bool,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for selection and sort UI.
    pub messages: Messages,
    /// Unique IDs wired to ARIA attributes.
    pub caption_id: String,
    /// Component instance ID.
    pub id: String,
}

// `SortDirection` — defined in `06-collections.md`

```

### 1.4 Guards

```rust
/// Checks if selection is enabled.
fn is_selection_enabled(ctx: &Context, _: &Props) -> bool {
    ctx.selection_mode != selection::Mode::None
}

/// Checks if multiple selection is enabled.
fn is_multiple_selection(ctx: &Context, _: &Props) -> bool {
    ctx.selection_mode == selection::Mode::Multiple
}

/// Checks if a row is selected.
fn is_row_selected(ctx: &Context, row_id: &Key) -> bool {
    ctx.selected_rows.get().contains(row_id)
}

/// Checks if a row is expanded.
fn is_row_expanded(ctx: &Context, row_id: &Key) -> bool {
    ctx.expanded_rows.get().contains(row_id)
}
```

### 1.5 Props

```rust
/// Controls behavior when Escape is pressed while rows are selected.
#[derive(Clone, Debug, PartialEq)]
pub enum EscapeKeyBehavior {
    /// Escape clears the current selection.
    ClearSelection,
    /// Escape is not handled by the Table.
    None,
}

/// Props for the Table component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    // ── Selection ──────────────────────────────────────────────────────────
    /// Controlled selected rows (uses canonical `selection::Set` from `ars_collections`).
    pub selected_rows: Option<selection::Set>,
    /// Uncontrolled initial selection.
    pub default_selected_rows: selection::Set,

    // ── Expansion ──────────────────────────────────────────────────────────
    /// Controlled expanded rows.
    pub expanded_rows: Option<BTreeSet<Key>>,
    /// Uncontrolled initial expansion.
    pub default_expanded_rows: BTreeSet<Key>,

    // ── Sort ───────────────────────────────────────────────────────────────
    /// Controlled/uncontrolled sort state. Uses `SortDescriptor` from `ars_collections`
    /// (see `06-collections.md §7.2`). `None` means no active sort.
    pub sort_descriptor: Bindable<Option<SortDescriptor<String>>>,

    // ── Behavior ───────────────────────────────────────────────────────────
    pub selection_mode: selection::Mode,
    /// Controls whether clicking a selected row deselects it (`Toggle`) or whether
    /// the user must hold Ctrl/Cmd to toggle (`Replace`). Default: `Toggle`.
    /// See `selection::Behavior` in `11-selection-components.md` §0.1.
    pub selection_behavior: selection::Behavior,
    /// Keys of individually disabled rows. Disabled rows cannot be selected or
    /// expanded, and are announced as disabled to assistive technology.
    pub disabled_keys: BTreeSet<Key>,
    /// Controls behavior when Escape is pressed while rows are selected.
    /// `ClearSelection` (default) deselects all; `None` disables Escape handling.
    pub escape_key_behavior: EscapeKeyBehavior,
    /// Enables role="grid" with keyboard cell navigation.
    pub interactive: bool,
    /// Fixes the thead while the tbody scrolls.
    pub sticky_header: bool,
    /// Optional visible caption text.
    pub caption: Option<String>,
    /// Localized labels for selection and sort UI.
    /// Prevents deselecting the last remaining selected row. When `true` and the user
    /// attempts to deselect the only selected row, the action is a no-op, ensuring at
    /// least one row is always selected.
    pub disallow_empty_selection: bool,
    // Change callbacks provided by the adapter layer
}
```

### 1.6 Async Data Loading

Tables support asynchronous data loading for large datasets:

```rust
    /// Whether the table is currently loading data.
    /// When `true`, the adapter renders skeleton placeholder rows.
    pub loading: Bindable<bool>,

    /// Callback triggered when the user scrolls near the end of the table,
    /// enabling infinite scroll / load-more patterns.
    pub on_load_more: Option<Callback<()>>,
```

- When `loading` is `true`, the adapter renders a loading indicator in the table footer area and displays skeleton rows in place of data rows.
- `on_load_more` fires when the scroll position reaches within a threshold of the table's scroll end (adapter-defined, typically ~200px). It is debounced to fire at most once per 300ms.
- While `loading` is `true`, `on_load_more` is suppressed to prevent duplicate requests.

```rust
impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            selected_rows: None,
            default_selected_rows: selection::Set::default(),
            expanded_rows: None,
            default_expanded_rows: BTreeSet::new(),
            sort_descriptor: Bindable::uncontrolled(None),
            selection_mode: selection::Mode::None,
            selection_behavior: selection::Behavior::Toggle,
            disabled_keys: BTreeSet::new(),
            escape_key_behavior: EscapeKeyBehavior::ClearSelection,
            interactive: false,
            sticky_header: false,
            caption: None,
            disallow_empty_selection: false,
        }
    }
}
```

### 1.7 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};
use ars_collections::{selection, Key};

// ── States ────────────────────────────────────────────────────────────────────

/// States for the Table.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No active sort; the table is at rest.
    Idle,
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events for the Table.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Apply or toggle sort on the given column.
    SortColumn { column: String },
    /// Mark a row as selected.
    SelectRow(Key),
    /// Remove a row from the selection.
    DeselectRow(Key),
    /// Select all rows (uses `selection::Set::All`).
    SelectAll,
    /// Clear all selected rows.
    DeselectAll,
    /// Flip selection state of one row.
    ToggleRow(Key),
    /// Show the expanded content for a row.
    ExpandRow(Key),
    /// Hide the expanded content for a row.
    CollapseRow(Key),
    /// Move the logical grid focus to a cell.
    Focus { cell: (usize, usize) },
    /// Remove focus from the grid.
    Blur,
    /// Move the logical grid focus to a row.
    FocusRow(Key),
    /// Move the logical grid focus to a cell.
    FocusCell { row: Key, col: usize },
    /// Primary action triggered on a row (Enter key or double-click).
    RowAction(Key),
}

/// Machine for the Table.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let ids = ComponentIds::from_id(&props.id);
        let selection_state = selection::State::new(
            props.selection_mode,
            props.selection_behavior,
        );
        let ctx = Context {
            selected_rows: match &props.selected_rows {
                Some(v) => Bindable::controlled(v.clone()),
                None    => Bindable::uncontrolled(props.default_selected_rows.clone()),
            },
            selection_state,
            expanded_rows: match &props.expanded_rows {
                Some(v) => Bindable::controlled(v.clone()),
                None    => Bindable::uncontrolled(props.default_expanded_rows.clone()),
            },
            sort_descriptor: props.sort_descriptor.clone(),
            focused_cell: None,
            focused_row: None,
            focused_col: None,
            disabled_keys: props.disabled_keys.clone(),
            selection_mode: props.selection_mode,
            interactive: props.interactive,
            locale,
            messages,
            caption_id: ids.part("caption"),
            id: ids.id().to_string(),
        };
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx:   &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            // ── Sort ─────────────────────────────────────────────────────────
            // Sort state lives entirely in `ctx.sort_descriptor` (see §1.1.1).
            // There is no State::Sorting variant — we stay in `Idle` throughout.
            Event::SortColumn { column } => {
                let new_direction = match ctx.sort_descriptor.get() {
                    Some(desc) if desc.column == *column => {
                        match desc.direction {
                            SortDirection::Ascending  => SortDirection::Descending,
                            SortDirection::Descending => SortDirection::None,
                            SortDirection::None       => SortDirection::Ascending,
                        }
                    }
                    // New column — always start ascending
                    _ => SortDirection::Ascending,
                };
                let col = column.clone();
                let dir = new_direction;
                Some(TransitionPlan::context_only(move |ctx| {
                    let descriptor = if dir == SortDirection::None {
                        None
                    } else {
                        Some(SortDescriptor { column: col, direction: dir })
                    };
                    ctx.sort_descriptor.set(descriptor);
                }))
                // Sort change notification is handled by the adapter layer.
            }

            // ── Row Action ────────────────────────────────────────────────
            Event::RowAction(key) => {
                if ctx.disabled_keys.contains(key) { return None; }
                // RowAction is a notification event — the adapter fires the
                // on_row_action callback. No context mutation needed.
                Some(TransitionPlan::context_only(|_ctx| {}))
            }

            // ── Row Selection (via selection::State from ars_collections) ────────
            Event::SelectRow(key) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                if ctx.disabled_keys.contains(key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.select(key);
                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::DeselectRow(key) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                if ctx.disabled_keys.contains(key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.deselect(key);
                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::ToggleRow(key) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                if ctx.disabled_keys.contains(key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.toggle(key);
                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::SelectAll => {
                if ctx.selection_mode != selection::Mode::Multiple { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let new_state = ctx.selection_state.select_all();
                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                    // LiveAnnouncer: announce "All rows selected"
                }))
            }

            Event::DeselectAll => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let new_state = ctx.selection_state.clear();
                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            // ── Row Expansion ─────────────────────────────────────────────────
            Event::ExpandRow(key) => {
                // Guard: skip when the row is already expanded — avoids
                // creating an unnecessary transition plan.
                if ctx.expanded_rows.get().contains(key) {
                    return None;
                }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut rows = ctx.expanded_rows.get().clone();
                    rows.insert(key);
                    ctx.expanded_rows.set(rows);
                }))
                // Expansion change notification is handled by the adapter layer.
            }

            Event::CollapseRow(key) => {
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut rows = ctx.expanded_rows.get().clone();
                    rows.remove(&key);
                    ctx.expanded_rows.set(rows);
                }))
                // Expansion change notification is handled by the adapter layer.
            }

            // ── Grid Focus ────────────────────────────────────────────────────
            Event::Focus { cell } => {
                if !ctx.interactive { return None; }
                let c = *cell;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_cell = Some(c);
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_cell = None;
                }))
            }

            Event::FocusRow(key) => {
                if !ctx.interactive { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_row = Some(key);
                }))
            }

            Event::FocusCell { row, col } => {
                if !ctx.interactive { return None; }
                let r = row.clone();
                let c = *col;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_cell = Some((c, 0)); // row index resolved by adapter
                    ctx.focused_row = Some(r);
                    ctx.focused_col = Some(c);
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx:   &'a Self::Context,
        props: &'a Self::Props,
        send:  &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.8 Connect / API

```rust
/// API for the Table component.
#[derive(Clone, Debug)]
pub struct Api<'a> {
    /// The current state of the Table.
    state: &'a State,
    /// The current context of the Table.
    ctx:   &'a Context,
    /// The current props of the Table.
    props: &'a Props,
    /// The event sender for the Table.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // ── Helpers ───────────────────────────────────────────────────────────

    /// Checks if a row is selected.
    pub fn is_row_selected(&self, row_id: &Key) -> bool {
        self.ctx.selected_rows.get().contains(row_id)
    }

    /// Checks if a row is expanded.
    pub fn is_row_expanded(&self, row_id: &Key) -> bool {
        self.ctx.expanded_rows.get().contains(row_id)
    }

    /// Returns the current sort descriptor.
    pub fn sort_descriptor(&self) -> Option<&SortDescriptor<String>> {
        self.ctx.sort_descriptor.get().as_ref()
    }

    /// Returns the active sort column.
    pub fn active_sort_column(&self) -> Option<&String> {
        self.sort_descriptor().map(|d| &d.column)
    }

    /// Returns the current sort direction.
    pub fn sort_direction(&self) -> SortDirection {
        self.sort_descriptor().map(|d| d.direction).unwrap_or(SortDirection::None)
    }

    /// Returns `true` when all rows are selected. Handles `selection::Set::All` efficiently
    /// without needing the full row ID list.
    pub fn all_selected(&self, all_row_ids: &[&Key]) -> bool {
        let sel = self.ctx.selected_rows.get();
        if sel.is_all() { return true; }
        if all_row_ids.is_empty() { return false; }
        all_row_ids.iter().all(|id| sel.contains(id))
    }

    /// Returns `true` when some rows are selected.
    pub fn some_selected(&self, all_row_ids: &[&Key]) -> bool {
        let sel = self.ctx.selected_rows.get();
        if sel.is_all() { return !all_row_ids.is_empty(); }
        all_row_ids.iter().any(|id| sel.contains(id))
    }

    // ── Root / Table ──────────────────────────────────────────────────────

    /// Returns the attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.props.sticky_header {
            p.set_bool(HtmlAttr::Data("ars-sticky-header"), true);
        }
        p
    }

    /// Returns the attributes for the table element.
    pub fn table_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        p.set(HtmlAttr::Id, &self.ctx.id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Table.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.ctx.interactive {
            p.set(HtmlAttr::Role, "grid");
        } else {
            p.set(HtmlAttr::Role, "table");
        }
        if self.props.caption.is_some() {
            p.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.caption_id);
        }
        p
    }

    /// Returns the attributes for the caption element.
    pub fn caption_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        p.set(HtmlAttr::Id, &self.ctx.caption_id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Caption.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p
    }

    /// Returns the attributes for the head element.
    pub fn head_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Head.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.props.sticky_header {
            p.set_bool(HtmlAttr::Data("ars-sticky"), true);
        }
        p
    }

    /// Returns the attributes for the body element.
    pub fn body_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Body.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p
    }

    /// Returns the attributes for the foot element.
    pub fn foot_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Foot.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p
    }

    // ── Rows ──────────────────────────────────────────────────────────────

    /// Returns the attributes for the row element.
    pub fn row_attrs(&self, row_id: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Row { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        let selected = self.is_row_selected(row_id);
        let expanded = self.is_row_expanded(row_id);
        if self.ctx.selection_mode != selection::Mode::None {
            p.set(HtmlAttr::Aria(AriaAttr::Selected), if selected { "true" } else { "false" });
            if selected { p.set_bool(HtmlAttr::Data("ars-selected"), true); }
        }
        if expanded {
            p.set(HtmlAttr::Aria(AriaAttr::Expanded), "true");
            p.set_bool(HtmlAttr::Data("ars-expanded"), true);
        }
        if self.ctx.disabled_keys.contains(row_id) {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.interactive {
            p.set(HtmlAttr::Role, "row");
        }
        // Event handlers (e.g. click to toggle row) are typed methods on the Api struct.
        p
    }

    /// Returns row attributes for a row that acts as a link. When `href` is provided,
    /// the adapter SHOULD render the row as a clickable link (or apply `cursor: pointer`
    /// and navigate on click). The row retains all standard attributes from `row_attrs`
    /// with the addition of `data-ars-href`.
    pub fn row_link_attrs(&self, row_id: &Key, href: &str) -> AttrMap {
        let mut p = self.row_attrs(row_id);
        p.set(HtmlAttr::Data("ars-href"), href);
        p
    }

    // ── Column Header ─────────────────────────────────────────────────────

    /// Returns the attributes for the column header element.
    pub fn column_header_attrs(&self, column: &str, sortable: bool) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ColumnHeader { header: String::new() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Scope, "col");
        if sortable {
            let is_sorted = self.active_sort_column().map(|c| c == column).unwrap_or(false);
            let direction = if is_sorted { self.sort_direction() } else { SortDirection::None };
            let aria_sort = match direction {
                SortDirection::Ascending  => "ascending",
                SortDirection::Descending => "descending",
                SortDirection::None       => "none",
            };
            p.set(HtmlAttr::Aria(AriaAttr::Sort), aria_sort);
            p.set(HtmlAttr::Data("ars-sort"), aria_sort);
            if is_sorted { p.set_bool(HtmlAttr::Data("ars-sorted"), true); }
            p.set(HtmlAttr::TabIndex, "0");
            // Event handlers (click/keydown to sort) are typed methods on the Api struct.
        }
        p
    }

    /// Returns the attributes for the row header element.
    pub fn row_header_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RowHeader.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Scope, "row");
        p
    }

    /// Returns the attributes for the cell element.
    pub fn cell_attrs(&self, col: usize, row: usize) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Cell { col: 0, row: 0 }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.ctx.interactive {
            let focused = self.ctx.focused_cell == Some((col, row));
            p.set(HtmlAttr::TabIndex, if focused { "0" } else { "-1" });
            // Event handlers (focus/blur for grid nav) are typed methods on the Api struct.
        }
        p
    }

    // ── Keyboard Navigation ──────────────────────────────────────────────

    /// Handle keydown on a row element (used when `interactive=true`).
    pub fn on_row_keydown(&self, row_id: &Key, data: &KeyboardEventData, all_row_ids: &[&Key]) {
        let current_idx = all_row_ids.iter().position(|id| *id == row_id);
        match data.key {
            KeyboardKey::ArrowDown => {
                if let Some(idx) = current_idx {
                    if let Some(next) = all_row_ids.get(idx + 1) {
                        (self.send)(Event::FocusRow((*next).clone()));
                    }
                }
            }
            KeyboardKey::ArrowUp => {
                if let Some(idx) = current_idx {
                    if idx > 0 {
                        if let Some(prev) = all_row_ids.get(idx - 1) {
                            (self.send)(Event::FocusRow((*prev).clone()));
                        }
                    }
                }
            }
            KeyboardKey::Home => {
                if let Some(first) = all_row_ids.first() {
                    (self.send)(Event::FocusRow((*first).clone()));
                }
            }
            KeyboardKey::End => {
                if let Some(last) = all_row_ids.last() {
                    (self.send)(Event::FocusRow((*last).clone()));
                }
            }
            KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::ToggleRow(row_id.clone()));
            }
            _ => {}
        }
    }

    /// Handle keydown on a cell element (used when `interactive=true` with grid role).
    ///
    /// RTL-aware: ArrowLeft/ArrowRight are resolved based on `ctx.dir` so that
    /// ArrowRight always moves toward the logical "end" column regardless of
    /// text direction.
    pub fn on_cell_keydown(&self, row_id: &Key, col: usize, data: &KeyboardEventData,
                           all_row_ids: &[&Key], col_count: usize) {
        // Resolve horizontal arrow keys for RTL.
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let (next_col_key, prev_col_key) = if is_rtl {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        } else {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        };

        match data.key {
            k if k == next_col_key => {
                if col + 1 < col_count {
                    (self.send)(Event::FocusCell { row: row_id.clone(), col: col + 1 });
                }
            }
            k if k == prev_col_key => {
                if col > 0 {
                    (self.send)(Event::FocusCell { row: row_id.clone(), col: col - 1 });
                }
            }
            KeyboardKey::ArrowDown => {
                let current_idx = all_row_ids.iter().position(|id| *id == row_id);
                if let Some(idx) = current_idx {
                    if let Some(next) = all_row_ids.get(idx + 1) {
                        (self.send)(Event::FocusCell { row: (*next).clone(), col });
                    }
                }
            }
            KeyboardKey::ArrowUp => {
                let current_idx = all_row_ids.iter().position(|id| *id == row_id);
                if let Some(idx) = current_idx {
                    if idx > 0 {
                        if let Some(prev) = all_row_ids.get(idx - 1) {
                            (self.send)(Event::FocusCell { row: (*prev).clone(), col });
                        }
                    }
                }
            }
            KeyboardKey::Home => {
                (self.send)(Event::FocusCell { row: row_id.clone(), col: 0 });
            }
            KeyboardKey::End => {
                (self.send)(Event::FocusCell { row: row_id.clone(), col: col_count.saturating_sub(1) });
            }
            _ => {}
        }
    }

    // ── Selection Controls ────────────────────────────────────────────────

    /// Returns the attributes for the select all checkbox element.
    pub fn select_all_attrs(&self, all_row_ids: &[&Key]) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SelectAllCheckbox.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Type, "checkbox");
        let all = self.all_selected(all_row_ids);
        let some = !all && self.some_selected(all_row_ids);
        // Label sourced from Messages struct — no hardcoded strings (see §2.8 of 03-accessibility.md).
        p.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.select_all)(&self.ctx.locale));
        p.set(HtmlAttr::Aria(AriaAttr::Checked), if all { "true" } else if some { "mixed" } else { "false" });
        if all { p.set_bool(HtmlAttr::Checked, true); }
        if some { p.set_bool(HtmlAttr::Data("ars-indeterminate"), true); }
        // Event handlers (change to select/deselect all) are typed methods on the Api struct.
        p
    }

    /// Returns the attributes for the row checkbox element.
    pub fn row_checkbox_attrs(&self, row_id: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RowCheckbox { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Type, "checkbox");
        let selected = self.is_row_selected(row_id);
        p.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.select_row)(&self.ctx.locale));
        p.set(HtmlAttr::Aria(AriaAttr::Checked), if selected { "true" } else { "false" });
        if selected { p.set_bool(HtmlAttr::Checked, true); }
        // Event handlers (change to toggle row) are typed methods on the Api struct.
        p
    }

    // ── Expand Controls ───────────────────────────────────────────────────

    /// Returns the attributes for the expand trigger element.
    pub fn expand_trigger_attrs(&self, row_id: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ExpandTrigger { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Type, "button");
        let expanded = self.is_row_expanded(row_id);
        p.set(HtmlAttr::Aria(AriaAttr::Expanded), if expanded { "true" } else { "false" });
        // aria-controls points to the detail row ID so AT can navigate to expanded content.
        let detail_id = format!("{}-expanded-{}", self.ctx.id, row_id);
        p.set(HtmlAttr::Aria(AriaAttr::Controls), detail_id);
        if expanded { p.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        // Event handlers (click to expand/collapse) are typed methods on the Api struct.
        p
    }

    /// Returns the attributes for the expanded content element.
    pub fn expanded_content_attrs(&self, row_id: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let detail_id = format!("{}-expanded-{}", self.ctx.id, row_id);
        p.set(HtmlAttr::Id, detail_id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ExpandedContent { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        let expanded = self.is_row_expanded(row_id);
        if expanded { p.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        if !expanded {
            p.set_bool(HtmlAttr::Hidden, true);
        }
        p
    }
}

#[derive(ComponentPart)]
#[scope = "table"]
pub enum Part {
    Root,
    Table,
    Caption,
    Head,
    Body,
    Foot,
    Row { key: Key },
    ColumnHeader { header: String },
    RowHeader,
    Cell { col: usize, row: usize },
    SelectAllCheckbox,
    RowCheckbox { key: Key },
    ExpandTrigger { key: Key },
    ExpandedContent { key: Key },
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root                        => self.root_attrs(),
            Part::Table                       => self.table_attrs(),
            Part::Caption                     => self.caption_attrs(),
            Part::Head                        => self.head_attrs(),
            Part::Body                        => self.body_attrs(),
            Part::Foot                        => self.foot_attrs(),
            Part::Row { key }                 => self.row_attrs(key),
            Part::ColumnHeader { header }     => self.column_header_attrs(header, true),
            Part::RowHeader                   => self.row_header_attrs(),
            Part::Cell { col, row }           => self.cell_attrs(*col, *row),
            Part::SelectAllCheckbox           => self.select_all_attrs(&[]),
            Part::RowCheckbox { key }         => self.row_checkbox_attrs(key),
            Part::ExpandTrigger { key }       => self.expand_trigger_attrs(key),
            Part::ExpandedContent { key }     => self.expanded_content_attrs(key),
        }
    }
}
```

## 2. Anatomy

```text
Table
├── Root              (wrapper div; data-ars-scope="table" data-ars-part="root")
├── Table             (<table> element)
│   ├── Caption       (<caption>; data-ars-part="caption")
│   ├── Head          (<thead>; data-ars-part="head")
│   │   └── Row       (<tr>)
│   │       ├── SelectAllCheckbox  (<input type="checkbox">)
│   │       └── ColumnHeader  (<th scope="col">; data-ars-sort, data-ars-sorted)
│   ├── Body          (<tbody>; data-ars-part="body")
│   │   └── Row       (<tr>; data-ars-selected, data-ars-expanded)
│   │       ├── RowCheckbox      (<input type="checkbox">)
│   │       ├── ExpandTrigger    (<button>; data-ars-expanded)
│   │       ├── RowHeader        (<th scope="row">)
│   │       ├── Cell             (<td>)
│   │       └── ExpandedContent  (<tr> / <td colspan>; hidden when collapsed)
│   └── Foot          (<tfoot>; data-ars-part="foot")
```

### 2.1 Anatomy Part Reference

| Part                | Element                   | Key Attributes                                                                                                   |
| ------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `Root`              | `<div>`                   | `data-ars-scope="table"` `data-ars-part="root"`                                                                  |
| `Table`             | `<table>`                 | `role="grid"` (interactive) or `role="table"`                                                                    |
| `Caption`           | `<caption>`               | `id` wired to table `aria-labelledby`                                                                            |
| `Head`              | `<thead>`                 | `data-ars-sticky` when `sticky_header`                                                                           |
| `Body`              | `<tbody>`                 | —                                                                                                                |
| `Foot`              | `<tfoot>`                 | —                                                                                                                |
| `Row`               | `<tr>`                    | `aria-selected`, `aria-expanded`, `aria-disabled`, `data-ars-selected`, `data-ars-expanded`, `data-ars-disabled` |
| `ColumnHeader`      | `<th scope="col">`        | `aria-sort`, `data-ars-sort`, `data-ars-sorted`                                                                  |
| `RowHeader`         | `<th scope="row">`        | —                                                                                                                |
| `Cell`              | `<td>`                    | `tabindex` managed for grid nav                                                                                  |
| `SelectAllCheckbox` | `<input type="checkbox">` | `aria-label`, `aria-checked` (incl. mixed)                                                                       |
| `RowCheckbox`       | `<input type="checkbox">` | `aria-label`, `aria-checked`                                                                                     |
| `ExpandTrigger`     | `<button>`                | `aria-expanded`, `data-ars-expanded`                                                                             |
| `ExpandedContent`   | `<tr>` + inner `<td>`     | `hidden` when collapsed, `data-ars-expanded`                                                                     |

### 2.2 Expandable Row Anatomy

Tables may support expandable rows that reveal additional detail content:

- **Expand Trigger**: Each expandable row includes a toggle button (typically a chevron icon) as the first cell or as an overlay on the row. The trigger receives `aria-expanded="true|false"` reflecting the current state. The trigger button MUST also set `aria-controls` pointing to the `id` of the detail row so assistive technology can navigate directly to the expanded content.
- **Content Area**: Expanded content is rendered as a full-width `<td colspan="...">` (spanning all columns) inside a `<tr>` immediately below the parent row.
- **Toggle Anatomy**: The expand/collapse control is a `<button>` element with `aria-label` describing the action (e.g., "Expand row" / "Collapse row"), managed via the `Messages` i18n struct.

#### 2.2.1 Nesting Rules

- **Single-level expansion only**: Expandable rows support one level of expansion. A row can expand to show detail content, but the detail content MUST NOT itself contain expandable rows. Nested expansion introduces unpredictable keyboard navigation and screen reader confusion.
- **Hierarchical data**: For tree-structured or multi-level hierarchical data, use the `TreeView` component instead of nested table expansion. TreeView provides proper `aria-level`, `aria-setsize`, and `aria-posinset` attributes for arbitrary nesting depth.
- **Indentation Level**: Since expansion is single-level, `aria-level` attributes are not used on expandable rows. All data rows are at the same level; expanded content is supplementary detail, not a child row.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **role="grid"** when `interactive=true`; **role="table"** otherwise.
- Sortable `ColumnHeader`: `aria-sort="ascending|descending|none"`. Enter/Space toggles sort.
- `Row`: `aria-selected` present only when `selection_mode != None`.
- `Row` with expand: `aria-expanded="true|false"`.
- `SelectAllCheckbox`: `aria-checked="mixed"` (indeterminate) when a subset of rows selected.

**`aria-sort` and `aria-current` Semantics**

- Sortable column headers MUST set `aria-sort` to one of: `"ascending"`, `"descending"`, or `"none"`.
- When a column is not currently sorted, set `aria-sort="none"` (not omitted) so screen readers can distinguish sortable from non-sortable columns.
- When sort state changes (user clicks a column header), announce the new sort state via `LiveAnnouncer` with `Politeness::Polite`: e.g., `"Sorted by Name, ascending"`. Use the `Messages.sort_ascending` / `Messages.sort_descending` message keys.

**Pagination `aria-current`:**

- When Table includes pagination controls, the active page button MUST set `aria-current="page"` to indicate the current page to screen readers.
- Other page buttons MUST NOT have `aria-current` set (omit the attribute entirely, do not set to `"false"`).

### 3.2 Keyboard Interaction

Grid keyboard navigation (when `interactive=true`):

| Key          | Action                                                                                   |
| ------------ | ---------------------------------------------------------------------------------------- |
| `ArrowUp`    | Move focus to the cell above                                                             |
| `ArrowDown`  | Move focus to the cell below                                                             |
| `ArrowLeft`  | Move focus to the previous cell (wraps to previous row)                                  |
| `ArrowRight` | Move focus to the next cell (wraps to next row)                                          |
| `Home`       | Move focus to the first cell in the current row                                          |
| `End`        | Move focus to the last cell in the current row                                           |
| `Ctrl+Home`  | Move focus to the first cell in the first row                                            |
| `Ctrl+End`   | Move focus to the last cell in the last row                                              |
| `Enter`      | Activate the focused cell's primary interactive content                                  |
| `F2`         | Enter edit mode for the focused cell (if editable)                                       |
| `Escape`     | Exit edit mode, return focus to the cell                                                 |
| `Space`      | Select/deselect the focused row (if selection enabled)                                   |
| `Tab`        | Move focus to the next focusable element inside the cell; if none, move to the next cell |

### 3.3 Focus Management

- When `interactive=true`, the Table renders `role="grid"` and manages a roving `tabindex` across cells. The currently focused cell has `tabindex="0"`; all other cells have `tabindex="-1"`.
- On initial focus (Tab into the grid), focus lands on the first cell of the first data row (or the last-focused cell if one was previously focused).
- Arrow keys move focus between cells without wrapping out of the grid. `Home`/`End` wrap within the current row; `Ctrl+Home`/`Ctrl+End` move to the first/last cell of the first/last row.
- When a cell contains interactive content (links, buttons, inputs), `Enter` activates it and `Escape` returns focus to the cell container.

### 3.4 Screen Reader Announcements

Sorting operations MUST be announced to assistive technology so screen reader users
understand how the data is currently ordered.

#### 3.4.1 `Messages` Sort Keys

The `Messages` struct (passed via props or the i18n message bundle) includes the
following sort-related keys:

```rust
/// Sort-related message keys within the Table's message bundle.
/// All messages support ICU MessageFormat placeholders.
pub struct SortMessages {
    /// Announced when a column is sorted ascending.
    /// Placeholder: `{column}` — the column's visible header text.
    /// Default: "{column} sorted ascending"
    pub sort_ascending: String,

    /// Announced when a column is sorted descending.
    /// Default: "{column} sorted descending"
    pub sort_descending: String,

    /// Announced when sort is removed from a column.
    /// Default: "{column} sort removed"
    pub sort_none: String,

    /// Hint text for sortable column headers (used in `aria-roledescription`
    /// or as a tooltip). Default: "activate to sort"
    pub sortable_hint: String,

    /// Announced when multi-column sort changes.
    /// Placeholder: `{columns}` — comma-separated list of active sort columns
    /// with their directions (e.g., "Name ascending, Date descending").
    /// Default: "sorted by {columns}"
    pub multi_sort_changed: String,
}
```

#### 3.4.2 `aria-sort` Attribute

Each sortable `<th>` element MUST carry the `aria-sort` attribute reflecting its current
sort state:

| Sort State | `aria-sort` Value |
| ---------- | ----------------- |
| Ascending  | `"ascending"`     |
| Descending | `"descending"`    |
| Unsorted   | `"none"`          |

Non-sortable columns MUST NOT include `aria-sort`.

#### 3.4.3 Live Region Announcement Timing

When a sort operation completes (i.e., the data has been reordered and the DOM updated),
the adapter MUST inject the appropriate message into an `aria-live="polite"` live region.
The announcement fires immediately after the sort transition — no artificial delay.

For single-column sort, the adapter uses `sort_ascending` or `sort_descending` with the
column name substituted. When sort is removed, `sort_none` is used.

For multi-column sort, the adapter uses `multi_sort_changed` with a formatted list of all
active sort columns and their directions, ordered by sort priority (primary first).

```rust
/// Builds the live-region announcement string after a sort change.
fn build_sort_announcement(
    columns: &[ColumnDef],
    sort_descriptors: &[SortDescriptor],
    messages: &SortMessages,
) -> String {
    if sort_descriptors.is_empty() {
        // All sorts removed — announce the last column that was unsorted.
        return String::new();
    }
    if sort_descriptors.len() == 1 {
        let desc = &sort_descriptors[0];
        let col_name = columns.iter()
            .find(|c| c.key == desc.column_key)
            .map(|c| c.header_text.as_str())
            .unwrap_or("");
        return match desc.direction {
            SortDirection::Ascending => (messages.sort_ascending)(&locale),
            SortDirection::Descending => (messages.sort_descending)(&locale),
        };
    }
    // Multi-column: build comma-separated list.
    let parts = sort_descriptors.iter().map(|desc| {
        let col_name = columns.iter()
            .find(|c| c.key == desc.column_key)
            .map(|c| c.header_text.as_str())
            .unwrap_or("");
        let dir = match desc.direction {
            SortDirection::Ascending => "ascending",
            SortDirection::Descending => "descending",
        };
        format!("{col_name} {dir}")
    }).collect::<Vec<_>>();
    messages.multi_sort_changed.replace("{columns}", &parts.join(", "))
}
```

### 3.5 Virtual Scrolling

When the Table uses virtual scrolling (only rendering visible rows), the `<table>` element must include `aria-colcount` and `aria-rowcount` attributes reflecting the total column and row counts (not just visible ones). Each visible `<tr>` must include `aria-rowindex` indicating its 1-based position in the full dataset.

```rust
// In table_attrs() when virtual scrolling is enabled:
if self.ctx.virtual_scrolling {
    attrs.set(HtmlAttr::Aria(AriaAttr::RowCount), self.ctx.total_rows.to_string());
    attrs.set(HtmlAttr::Aria(AriaAttr::ColCount), self.ctx.total_cols.to_string());
}

// In row_attrs(index):
if self.ctx.virtual_scrolling {
    attrs.set(HtmlAttr::Aria(AriaAttr::RowIndex), (index + 1).to_string());
}
```

> **Virtualizer Integration**: For large datasets, Table integrates with `Virtualizer` from `ars-collections` (§1.6 of `06-collections.md`). When `virtualizer: Option<Virtualizer>` is set in the Table context, only visible rows are rendered. The adapter wires scroll position updates to the Virtualizer, which computes the visible row range. A `ScrollToRow(Key)` event is available to programmatically scroll a specific row into view.

#### 3.5.1 IntersectionObserver for Table Virtual Scroll

When Table uses virtual scrolling for large datasets, the virtualizer MUST integrate with `IntersectionObserver` as specified in `06-collections.md` section "IntersectionObserver for Lazy Rendering & Virtual Scroll".

**Sentinel Placement**: Within the Table's virtual scroll container, place sentinel elements at the top and bottom of the rendered row range. When a sentinel enters the viewport (with the configured `rootMargin` buffer), the virtualizer extends the rendered range in that direction.

**Integration with AsyncCollection**: If Table's data source is an `AsyncCollection`, the bottom sentinel also triggers `load_more()` for infinite scroll / pagination. The sentinel must be placed AFTER the last rendered row but BEFORE the spacer element that maintains scroll height.

**Cross-reference**: See `06-collections.md` for full `IntersectionObserver` configuration (root, rootMargin, threshold), sentinel element pattern, and cleanup requirements.

## 4. Internationalization

### 4.1 BiDi Isolation for Numeric and Currency Values

Currency and numeric values in data display components must use BiDi isolation when rendered in mixed-direction contexts. Formatted values are wrapped in Unicode isolates (U+2066...U+2069) to prevent digit reordering in RTL layouts. Table cells containing numeric data should use `dir='ltr'` attribute to maintain correct digit order regardless of page direction.

### 4.2 Ordinal Formatting

Components displaying ordinal numbers (e.g., pagination "Page 1st", table ranking columns) must use ICU4X `PluralRules` with ordinal type for locale-correct formatting (English: 1st/2nd/3rd, French: 1er/2e, Japanese: 第1). An `format_ordinal(n: u64, locale: &Locale) -> String` utility is provided by `ars-i18n`. Components must not hardcode ordinal suffixes.

- Cell values containing numbers use `NumberFormatter` from `ars-i18n` for locale-aware
  formatting. The Table machine does not format; adapters apply formatting at render time.
- "Select all rows" and "Select row" labels are passed through `Messages`:

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    pub select_all: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub select_row: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub sort_ascending: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub sort_descending: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub sort_none: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            select_all: MessageFn::static_str("Select all rows"),
            select_row: MessageFn::static_str("Select row"),
            sort_ascending: MessageFn::static_str("Sort ascending"),
            sort_descending: MessageFn::static_str("Sort descending"),
            sort_none: MessageFn::static_str("Remove sort"),
        }
    }
}
impl ComponentMessages for Messages {}
```

- **RTL layout**: In right-to-left locales the column order is visually reversed via CSS
  `direction: rtl`. Sort direction indicators flip accordingly. The machine is RTL-agnostic;
  the adapter applies `dir="rtl"` on the `Root` element based on the active locale.

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, Table integrates with `DraggableCollection<T>` and `DroppableCollection<T>` from `06-collections.md` §7. Each row gains an optional `DragHandle` anatomy part, and `DropIndicator` parts appear between rows during drag. Fires `CollectionDndEvent::Reorder` on drop.

### 4.3 Locale-Aware String Collation

When Table sorts columns containing string data, sorting MUST use locale-aware collation rather than byte-order comparison. The `Collator` from `ars-i18n` (backed by ICU4X `Collator`) provides this capability.

**Collation Levels:**

| Level | Name               | Comparison Scope      | Example      |
| ----- | ------------------ | --------------------- | ------------ |
| 1     | Primary            | Base characters only  | a = á = A    |
| 2     | Secondary          | Base + accents        | a = A, a ≠ á |
| 3     | Tertiary (default) | Base + accents + case | a ≠ A, a ≠ á |

**Props:**

- `collation_level: CollationLevel` — optional prop on sortable columns. Defaults to `CollationLevel::Tertiary`.
- The adapter passes the active locale to `Collator::new(locale, level)` when performing sort comparisons.

**Cross-reference:** See `06-collections.md` for the `Collator` API and integration with `SortedCollection`.

## 5. Variant: SelectAll

When `selection_mode == Multiple`, the "Select All" checkbox in the header supports different
strategies for datasets that may be paginated or virtualised.

### 5.1 Additional Props

```rust
/// Controls what "Select All" means for the table.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectAllMode {
    /// No select-all affordance is rendered.
    None,
    /// Select all currently visible (rendered) rows. This is the default.
    AllVisible,
    /// Select all rows in the dataset, including those not yet loaded.
    /// `total_count` is the known cardinality — used for the checkbox label
    /// ("Select all 1,204 items") and for the `aria-checked` derivation.
    AllData { total_count: usize },
}
```

### 5.2 Behavior

**Exclusion-set pattern** — The `selection::Set::All` variant from `ars_collections` natively
supports the "select all with exclusions" pattern. When the user selects all via `AllData`
and then deselects individual rows, `selection::State::deselect()` transitions from `All` to
`Keys(complement)` using the full known key set. For async/paginated collections where the
full key set is unknown, the adapter should track an exclusion set alongside `selection::Set::All`:

```rust
/// Extended selection tracking for server-paginated AllData tables.
/// Wraps the canonical `selection::Set` with an exclusion list for rows
/// deselected after a bulk select-all on an incompletely-loaded collection.
#[derive(Clone, Debug, PartialEq)]
pub struct BulkSelection {
    /// Row IDs explicitly deselected after a `selection::Set::All` selection.
    /// Only meaningful when `selected_rows` is `selection::Set::All`.
    pub excluded: BTreeSet<Key>,
}

impl BulkSelection {
    /// Checks if a row is selected.
    pub fn is_selected(&self, row_id: &Key, selection: &selection::Set) -> bool {
        match selection {
            selection::Set::All => !self.excluded.contains(row_id),
            other => other.contains(row_id),
        }
    }

    /// Returns the effective count of selected rows.
    pub fn effective_count(&self, total: usize, selection: &selection::Set) -> usize {
        match selection {
            selection::Set::All => total.saturating_sub(self.excluded.len()),
            selection::Set::Multiple(s) => s.len(),
            selection::Set::Empty => 0,
        }
    }
}
```

When `selection::Set::All` is active and the user deselects a row via `DeselectRow`, the row ID is
added to `excluded`. The `SelectAllCheckbox` shows `aria-checked="mixed"` when `excluded` is
non-empty and selection is `All`.

### 5.3 Accessibility

The adapter communicates the bulk selection state to the server for operations like "delete
selected" — sending `{ select_all: true, excluded: [...] }` is far more efficient than
enumerating potentially millions of selected row IDs.

## 6. Variant: Column Resizing

Table supports user-driven column resizing via drag handles between column headers.

### 6.1 Additional Events

```rust
/// Column resize event — added to the Table Event enum.
ColumnResize { column: String, width: f64 },
```

### 6.2 Additional Context

```rust
/// Added to the Table Context struct.
/// Map from column ID to current width in pixels. `None` means auto-width.
pub column_widths: BTreeMap<String, f64>,
/// Column currently being resized (for drag state).
pub resizing_column: Option<String>,
```

### 6.3 Anatomy Additions

The `ColumnResizeHandle` anatomy part is a child of each `ColumnHeader`:

```text
ColumnHeader  (<th scope="col">)
├── ... (existing children)
└── ColumnResizeHandle  (<div>; data-ars-part="column-resize-handle")
```

| Part                 | Element | Key Attributes                                                                                                                                                |
| -------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ColumnResizeHandle` | `<div>` | `data-ars-part="column-resize-handle"`, `data-ars-resizing` (present during drag), `role="separator"`, `aria-orientation="vertical"`, `aria-valuenow={width}` |

### 6.4 Behavior

- **Pointer drag**: `pointerdown` on the handle starts resize. `pointermove` updates
  `column_widths[column]`. `pointerup` commits.
- **Keyboard**: When focused, `ArrowLeft`/`ArrowRight` adjust width by `step` (default 10px).
  `aria-valuenow` reflects the current pixel width.
- **Constraint**: `min_column_width` prop (default 50px) prevents columns from collapsing to zero.
- The adapter applies `style="width: {w}px"` to each `<col>` in a `<colgroup>` element.

## 7. Library Parity

> Compared against: React Aria (`Table`).

### 7.1 Props

| Feature                                   | ars-ui                             | React Aria                   | Notes                                                      |
| ----------------------------------------- | ---------------------------------- | ---------------------------- | ---------------------------------------------------------- |
| `selection_mode`                          | `selection::Mode`                  | `SelectionMode`              | Equivalent                                                 |
| `selected_rows` / `default_selected_rows` | `selection::Set`                   | `Iterable<Key> \| 'all'`     | ars-ui uses canonical `selection::Set` with `All` variant  |
| `sort_descriptor`                         | `Bindable<Option<SortDescriptor>>` | `SortDescriptor`             | ars-ui adds controlled/uncontrolled via Bindable           |
| `disabled_keys`                           | `BTreeSet<Key>`                    | `Iterable<Key>`              | Added from React Aria                                      |
| `disallow_empty_selection`                | `bool`                             | `bool`                       | Equivalent                                                 |
| `escape_key_behavior`                     | `EscapeKeyBehavior`                | `'none' \| 'clearSelection'` | Added from React Aria                                      |
| `selection_behavior`                      | `selection::Behavior`              | `SelectionBehavior`          | Equivalent                                                 |
| `interactive`                             | `bool`                             | --                           | ars-ui original (controls `role="grid"` vs `role="table"`) |
| `sticky_header`                           | `bool`                             | --                           | ars-ui original                                            |
| `caption`                                 | `Option<String>`                   | --                           | ars-ui original                                            |
| `expanded_rows`                           | `Bindable<BTreeSet<Key>>`          | --                           | ars-ui original                                            |
| `drag_and_drop_hooks`                     | Via `06-collections.md`            | `DragAndDropHooks`           | Both support DnD                                           |
| `disabledBehavior`                        | --                                 | `DisabledBehavior`           | Omitted; ars-ui disabled rows are fully non-interactive    |

**Gaps:** None.

### 7.2 Anatomy

| Part               | ars-ui                             | React Aria                | Notes                                                      |
| ------------------ | ---------------------------------- | ------------------------- | ---------------------------------------------------------- |
| Root wrapper       | `Root`                             | --                        | ars-ui adds a wrapper div                                  |
| Table              | `Table`                            | `Table`                   | --                                                         |
| Header             | `Head`                             | `TableHeader`             | --                                                         |
| Column             | `ColumnHeader`                     | `Column`                  | --                                                         |
| ColumnResizer      | `ColumnResizeHandle` (§6)          | `ColumnResizer`           | --                                                         |
| Body               | `Body`                             | `TableBody`               | --                                                         |
| Row                | `Row`                              | `Row`                     | --                                                         |
| Cell               | `Cell`                             | `Cell`                    | --                                                         |
| Footer             | `Foot`                             | --                        | ars-ui original                                            |
| Caption            | `Caption`                          | --                        | ars-ui original                                            |
| RowHeader          | `RowHeader`                        | --                        | ars-ui original                                            |
| Selection controls | `SelectAllCheckbox`, `RowCheckbox` | `Checkbox`                | ars-ui splits into explicit parts                          |
| Expand             | `ExpandTrigger`, `ExpandedContent` | --                        | ars-ui original                                            |
| Load more          | via `on_load_more` + §1.6          | `TableLoadMoreItem`       | Both support infinite scroll                               |
| ResizableContainer | --                                 | `ResizableTableContainer` | ars-ui handles resizing via §6 without a wrapper component |

**Gaps:** None.

### 7.3 Events

| Callback                                          | ars-ui                               | React Aria                                   | Notes                 |
| ------------------------------------------------- | ------------------------------------ | -------------------------------------------- | --------------------- |
| `on_selection_change`                             | Adapter layer (Bindable observation) | `onSelectionChange`                          | Equivalent            |
| `on_sort_change`                                  | Adapter layer                        | `onSortChange`                               | Equivalent            |
| `on_row_action`                                   | `RowAction` event                    | `onRowAction`                                | Added from React Aria |
| `on_resize` / `on_resize_start` / `on_resize_end` | Via §6 ColumnResize event            | `onResize` / `onResizeStart` / `onResizeEnd` | Equivalent            |

**Gaps:** None.

### 7.4 Features

| Feature                   | ars-ui                | React Aria                |
| ------------------------- | --------------------- | ------------------------- |
| Single/multiple selection | Yes                   | Yes                       |
| Sorting                   | Yes                   | Yes                       |
| Column resizing           | Yes (§6)              | Yes                       |
| Disabled rows             | Yes                   | Yes                       |
| Row expansion             | Yes                   | No                        |
| Sticky header             | Yes                   | No                        |
| Virtual scrolling         | Yes (§3.5)            | Yes                       |
| Infinite scroll           | Yes (`on_load_more`)  | Yes (`TableLoadMoreItem`) |
| Drag and drop             | Yes (via collections) | Yes (`dragAndDropHooks`)  |
| Grid keyboard nav         | Yes (`interactive`)   | Yes                       |
| Row links (`href`)        | Yes                   | Yes                       |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria's `disabledBehavior` (controlling whether disabled items remain focusable) is omitted; ars-ui treats disabled rows as fully non-interactive. React Aria uses a wrapper `ResizableTableContainer` component for column resizing; ars-ui integrates resizing directly via the §6 variant.
- **Recommended additions:** None.
