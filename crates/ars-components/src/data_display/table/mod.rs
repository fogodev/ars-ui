//! Table data-display component machine.
//!
//! Owns row/column selection, sort descriptors, expansion, grid focus,
//! optional column-resize state, optional virtual-scrolling metadata, and
//! the registered row list. The agnostic core never moves DOM focus,
//! never measures column widths, never observes scroll, and never reads
//! the rendered cell contents — adapters apply DOM focus by inspecting
//! [`Context::focused_cell`] / [`Context::focused_row`] /
//! [`Context::focused_col`], and observe pointer events to drive
//! [`Event::ColumnResize`] / [`Event::ColumnResizeEnd`].
//!
//! Sort state lives exclusively in [`Context::sort_descriptor`] (spec
//! §1.1.1). [`State`] has a single `Idle` variant; the visual sort
//! spinner is gated on [`Context::is_sorting`].
//!
//! Direction resolution: [`Props::dir`] is mirrored into
//! [`Context::dir`] at [`Machine::init`] time. Adapters must dispatch
//! [`Event::SetDirection`] from [`Machine::on_props_changed`] (and on
//! mount when the prop is [`Direction::Auto`] — the runtime-resolved
//! concrete value is passed in) so RTL keyboard navigation stays live.
//!
//! Row registry: the machine tracks the rendered row list via
//! [`Context::rows`], replaced by [`Event::SetRows`]. The registry
//! powers `disallow_empty_selection` pruning and the
//! `aria-rowcount`/`aria-rowindex` derivations for virtual scrolling.
//! Keyboard helpers ([`Api::on_row_keydown`], [`Api::on_cell_keydown`])
//! still accept an `all_row_ids` slice so adapters can pass the rendered
//! row order without an extra registry round-trip.
//!
//! See `spec/components/data-display/table.md` for the full contract,
//! plus §5 (SelectAll), §6 (Column Resizing), §3.5 (Virtual Scrolling),
//! and §1.6 (Async loading).

use alloc::{
    borrow::ToOwned as _,
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::{Key, SortDescriptor, SortDirection, selection};
use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    Direction, Env, HtmlAttr, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

#[cfg(test)]
mod tests;

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Table`](self) component.
///
/// The table machine never changes state — every interaction (sort,
/// selection, expansion, grid focus, column resize, virtual scroll)
/// is expressed as a context mutation. Sort state in particular lives
/// exclusively in [`Context::sort_descriptor`] per spec §1.1.1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The table is at rest.
    #[default]
    Idle,
}

// ────────────────────────────────────────────────────────────────────
// Escape-key behavior
// ────────────────────────────────────────────────────────────────────

/// Controls behavior when Escape is pressed while rows are selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EscapeKeyBehavior {
    /// Escape clears the current selection (default).
    #[default]
    ClearSelection,

    /// Escape is not handled by the Table.
    None,
}

// ────────────────────────────────────────────────────────────────────
// SelectAll mode (§5.1)
// ────────────────────────────────────────────────────────────────────

/// Strategy for the `SelectAll` affordance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SelectAllMode {
    /// No select-all affordance is rendered.
    None,

    /// Select all currently visible (rendered) rows. Default.
    #[default]
    AllVisible,

    /// Select every row in the dataset, including rows not yet loaded.
    /// `total_count` is the known cardinality.
    AllData {
        /// Total number of rows in the dataset.
        total_count: usize,
    },
}

// ────────────────────────────────────────────────────────────────────
// Events
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Table`](self) state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Apply or toggle sort on the given column. Cycle is
    /// `None → Ascending → Descending → None`.
    SortColumn {
        /// Column identifier the user activated.
        column: String,
    },

    /// Mark a row as selected. Rejected when selection is disabled or
    /// the row is in [`Context::disabled_keys`].
    SelectRow(Key),

    /// Remove a row from the selection. Honors
    /// [`Context::disallow_empty_selection`].
    DeselectRow(Key),

    /// Select every row (`Mode::Multiple` only). Uses
    /// [`selection::Set::All`].
    SelectAll,

    /// Clear the entire selection. Rejected when
    /// [`Context::disallow_empty_selection`] would leave the table empty.
    DeselectAll,

    /// Flip the selection state of one row. Honors disabled and
    /// `disallow_empty_selection`.
    ToggleRow(Key),

    /// Show the expanded content for a row. No-op when already
    /// expanded or the row is disabled.
    ExpandRow(Key),

    /// Hide the expanded content for a row.
    CollapseRow(Key),

    /// Move the logical grid focus to a cell by `(col, row)` indices.
    Focus {
        /// `(col, row)` index pair.
        cell: (usize, usize),
    },

    /// Remove focus from the grid. Clears
    /// [`Context::focused_cell`] / [`Context::focused_row`] /
    /// [`Context::focused_col`].
    Blur,

    /// Move the logical row focus to the supplied row key.
    FocusRow(Key),

    /// Move the logical grid focus to the supplied row + column pair.
    FocusCell {
        /// Row key receiving focus.
        row: Key,

        /// Column index receiving focus.
        col: usize,

        /// Zero-based index of `row` inside [`Context::rows`] (so the
        /// transition can write `(col, row_index)` into
        /// [`Context::focused_cell`] without re-scanning the row list).
        row_index: usize,
    },

    /// Primary action triggered on a row (Enter, double-click).
    RowAction(Key),

    /// User pressed Escape. Honors
    /// [`Context::escape_key_behavior`].
    EscapeKey,

    /// Replace the registered row list. Stale selection / expansion /
    /// focused row keys are pruned.
    SetRows(Vec<Key>),

    /// Replace [`Context::dir`]. Idempotent.
    SetDirection(Direction),

    /// Replace the virtualized total row / column counts (§3.5).
    SetRowCounts {
        /// Total row count across the entire dataset.
        total_rows: usize,

        /// Total column count across the entire dataset.
        total_cols: usize,
    },

    /// Toggle the async loading indicator (§1.6).
    SetLoading(bool),

    /// Toggle the async-sort indicator (§1.1.1).
    ///
    /// Adapters dispatch `SetIsSorting(true)` before kicking off an async
    /// sort against their data source, and `SetIsSorting(false)` when the
    /// sort completes. The agnostic core never touches this flag itself
    /// because synchronous sorts complete in the same render frame and
    /// the indicator would never be visible.
    SetIsSorting(bool),

    /// Update the cached width for `column`. Clamps to
    /// [`Context::min_column_width`] and upserts the entry on first
    /// resize. Records `column` as the currently resizing column.
    ColumnResize {
        /// Column identifier.
        column: String,

        /// Requested width in pixels (pre-clamp).
        width: f64,
    },

    /// Clear [`Context::resizing_column`] when it matches `column`.
    ColumnResizeEnd {
        /// Column identifier whose drag ended.
        column: String,
    },

    /// Re-apply non-Bindable `Props` fields and re-prune
    /// selection/expansion against new `disabled_keys`.
    SyncProps,

    /// Push a new controlled value into [`Context::selected_rows`].
    SyncControlledSelectedRows(Option<selection::Set>),

    /// Push a new controlled value into [`Context::expanded_rows`].
    SyncControlledExpandedRows(Option<BTreeSet<Key>>),

    /// Push a new controlled value into [`Context::sort_descriptor`].
    ///
    /// The outer `Option` is the controlled-mode toggle, matching
    /// [`Bindable::sync_controlled`]'s `Option<T>` argument:
    ///
    /// * `Some(opt)` — enter / update controlled mode with value `opt`.
    ///   `opt = None` means "controlled, no active sort".
    /// * `None` — leave controlled mode entirely; the Bindable falls
    ///   back to its uncontrolled internal value.
    SyncControlledSortDescriptor(Option<Option<SortDescriptor<String>>>),

    /// Push a new controlled value into [`Context::loading`]. Same
    /// outer-Option semantics as
    /// [`Event::SyncControlledSortDescriptor`].
    SyncControlledLoading(Option<bool>),
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Closure signature for the localized label messages.
///
/// Receives the active [`Locale`] and returns the localized string.
pub type LocaleFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Closure signature for the `SelectAll` label which may take a row count.
///
/// Receives the total row count (or `0` when none is known) and the
/// active [`Locale`], and returns the localized "Select all N rows"
/// label.
pub type SelectAllLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Localizable strings for [`Table`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the `SelectAll` checkbox. Receives the total
    /// row count (`0` when unknown) so [`SelectAllMode::AllData`] can
    /// render "Select all 1,204 rows".
    pub select_all: MessageFn<SelectAllLabelFn>,

    /// Accessible label for a row's selection checkbox.
    pub select_row: MessageFn<LocaleFn>,

    /// Announcement fired when a column is sorted ascending.
    pub sort_ascending: MessageFn<LocaleFn>,

    /// Announcement fired when a column is sorted descending.
    pub sort_descending: MessageFn<LocaleFn>,

    /// Announcement fired when sort is removed.
    pub sort_none: MessageFn<LocaleFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            select_all: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 0 {
                    "Select all rows".to_string()
                } else {
                    format!("Select all {count} rows")
                }
            }),
            select_row: MessageFn::new(|_locale: &Locale| "Select row".to_string()),
            sort_ascending: MessageFn::new(|_locale: &Locale| "Sort ascending".to_string()),
            sort_descending: MessageFn::new(|_locale: &Locale| "Sort descending".to_string()),
            sort_none: MessageFn::new(|_locale: &Locale| "Remove sort".to_string()),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Immutable configuration for a [`Table`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id. Required.
    pub id: String,

    // ── Selection ────────────────────────────────────────────────────
    /// Controlled selected rows.
    pub selected_rows: Option<selection::Set>,

    /// Uncontrolled initial selection. Disabled rows are pruned during
    /// [`Machine::init`].
    pub default_selected_rows: selection::Set,

    /// Row selection mode.
    pub selection_mode: selection::Mode,

    /// Controls whether selection toggles or replaces on click.
    pub selection_behavior: selection::Behavior,

    /// Keys of individually disabled rows.
    pub disabled_keys: BTreeSet<Key>,

    /// Forbid deselecting the last remaining selected row.
    pub disallow_empty_selection: bool,

    /// Behavior when Escape is pressed while rows are selected.
    pub escape_key_behavior: EscapeKeyBehavior,

    /// `SelectAll` affordance strategy (§5).
    pub select_all_mode: SelectAllMode,

    // ── Expansion ────────────────────────────────────────────────────
    /// Controlled expanded rows.
    pub expanded_rows: Option<BTreeSet<Key>>,

    /// Uncontrolled initial expansion.
    pub default_expanded_rows: BTreeSet<Key>,

    // ── Sort ─────────────────────────────────────────────────────────
    /// Controlled/uncontrolled sort state.
    pub sort_descriptor: Bindable<Option<SortDescriptor<String>>>,

    // ── Layout & a11y ────────────────────────────────────────────────
    /// Enables `role="grid"` with full keyboard cell navigation.
    pub interactive: bool,

    /// Fixes the `<thead>` while the `<tbody>` scrolls.
    pub sticky_header: bool,

    /// Optional visible caption text. When `Some`, the table renders
    /// `aria-labelledby` pointing at the caption's id.
    pub caption: Option<String>,

    /// Text direction. `Direction::Auto` resolves from the active
    /// locale at [`Machine::init`] time.
    pub dir: Direction,

    // ── Column resizing (§6) ─────────────────────────────────────────
    /// Minimum allowed column width in pixels.
    pub min_column_width: f64,

    /// Keyboard width-adjustment step in pixels.
    pub column_resize_step: f64,

    // ── Virtual scrolling (§3.5) ─────────────────────────────────────
    /// When `true`, the table emits `aria-rowcount` / `aria-colcount`
    /// and rows emit `aria-rowindex`.
    pub virtual_scrolling: bool,

    /// Total row count in the dataset (not just visible rows).
    pub total_rows: usize,

    /// Total column count in the dataset.
    pub total_cols: usize,

    // ── Async loading (§1.6) ─────────────────────────────────────────
    /// Async loading flag. Adapters render skeleton rows when `true`.
    pub loading: Bindable<bool>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            selected_rows: None,
            default_selected_rows: selection::Set::default(),
            selection_mode: selection::Mode::None,
            selection_behavior: selection::Behavior::Toggle,
            disabled_keys: BTreeSet::new(),
            disallow_empty_selection: false,
            escape_key_behavior: EscapeKeyBehavior::ClearSelection,
            select_all_mode: SelectAllMode::default(),
            expanded_rows: None,
            default_expanded_rows: BTreeSet::new(),
            sort_descriptor: Bindable::uncontrolled(None),
            interactive: false,
            sticky_header: false,
            caption: None,
            dir: Direction::default(),
            min_column_width: 50.0,
            column_resize_step: 10.0,
            virtual_scrolling: false,
            total_rows: 0,
            total_cols: 0,
            loading: Bindable::uncontrolled(false),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for the [`Table`](self) machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Currently selected row IDs.
    pub selected_rows: Bindable<selection::Set>,

    /// Full selection state machine (mode, behavior, anchor, focus,
    /// disabled keys).
    pub selection_state: selection::State,

    /// Currently expanded row IDs (expansion is unrelated to selection).
    pub expanded_rows: Bindable<BTreeSet<Key>>,

    /// Unified sort state. `None` means no active sort.
    pub sort_descriptor: Bindable<Option<SortDescriptor<String>>>,

    /// True while an async sort is in progress (drives sort-indicator
    /// visuals only; the canonical state is `sort_descriptor`).
    pub is_sorting: bool,

    /// Grid focus position: `(col_index, row_index)`. None when
    /// unfocused.
    pub focused_cell: Option<(usize, usize)>,

    /// Focused row key for row-level keyboard navigation.
    pub focused_row: Option<Key>,

    /// Focused column index for cell-level keyboard navigation.
    pub focused_col: Option<usize>,

    /// Keys of individually disabled rows.
    pub disabled_keys: BTreeSet<Key>,

    /// Row selection mode.
    pub selection_mode: selection::Mode,

    /// When `true`, the table renders `role="grid"`.
    pub interactive: bool,

    /// Registered row keys, replaced via [`Event::SetRows`].
    pub rows: Vec<Key>,

    /// Resolved layout direction.
    pub dir: Direction,

    /// Per-column cached widths (§6).
    pub column_widths: BTreeMap<String, f64>,

    /// Column currently being resized (§6).
    pub resizing_column: Option<String>,

    /// True when virtual scrolling drives the ARIA output (§3.5).
    pub virtual_scrolling: bool,

    /// Total row count across the entire dataset.
    pub total_rows: usize,

    /// Total column count across the entire dataset.
    pub total_cols: usize,

    /// Async loading flag (§1.6).
    pub loading: Bindable<bool>,

    /// Escape-key behavior.
    pub escape_key_behavior: EscapeKeyBehavior,

    /// `SelectAll` affordance strategy.
    pub select_all_mode: SelectAllMode,

    /// Minimum allowed column width when resizing (§6).
    pub min_column_width: f64,

    /// Keyboard width-adjustment step when resizing (§6).
    pub column_resize_step: f64,

    /// Whether deselecting the last selected row is forbidden.
    pub disallow_empty_selection: bool,

    /// Whether the `<thead>` is fixed while the `<tbody>` scrolls.
    pub sticky_header: bool,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved messages for selection and sort UI.
    pub messages: Messages,

    /// `<caption>` element id wired into `aria-labelledby`.
    pub caption_id: String,

    /// Component instance ID.
    pub id: String,
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Anatomy parts for the [`Table`](self) component.
#[derive(ars_core::ComponentPart)]
#[scope = "table"]
pub enum Part {
    /// Wrapper `<div>` around the table element.
    Root,

    /// The `<table>` element itself.
    Table,

    /// The `<caption>` element.
    Caption,

    /// The `<thead>` element.
    Head,

    /// The `<tbody>` element.
    Body,

    /// The `<tfoot>` element.
    Foot,

    /// A data `<tr>` row.
    Row {
        /// Row identifier.
        key: Key,
    },

    /// A `<th scope="col">` column header.
    ColumnHeader {
        /// Column identifier.
        header: String,
    },

    /// A `<th scope="row">` row header cell.
    RowHeader,

    /// A `<td>` data cell.
    Cell {
        /// Column index.
        col: usize,

        /// Row index.
        row: usize,
    },

    /// The `SelectAll` checkbox in the header.
    SelectAllCheckbox,

    /// The per-row selection checkbox.
    RowCheckbox {
        /// Row identifier.
        key: Key,
    },

    /// The expand/collapse trigger for an expandable row.
    ExpandTrigger {
        /// Row identifier.
        key: Key,
    },

    /// The expanded detail content for a row.
    ExpandedContent {
        /// Row identifier.
        key: Key,
    },

    /// The column resize handle (§6).
    ColumnResizeHandle {
        /// Column identifier.
        column: String,
    },
}

// ────────────────────────────────────────────────────────────────────
// Helpers (selection / row registry pruning)
// ────────────────────────────────────────────────────────────────────

/// Removes disabled keys from a [`selection::Set`].
fn prune_selection_against(set: &selection::Set, disabled: &BTreeSet<Key>) -> selection::Set {
    // `Empty` and `All` (and any future non-exhaustive variants) cannot
    // independently carry a disabled key, so they're cloned through
    // unchanged via the wildcard arm at the bottom.
    #[expect(
        clippy::match_same_arms,
        reason = "explicit non-exhaustive wildcard distinguishes Empty/All from unknown future variants"
    )]
    match set {
        selection::Set::Single(key) => {
            if disabled.contains(key) {
                selection::Set::Empty
            } else {
                selection::Set::Single(key.clone())
            }
        }

        selection::Set::Multiple(keys) => {
            let pruned = keys
                .iter()
                .filter(|k| !disabled.contains(k))
                .cloned()
                .collect::<BTreeSet<_>>();

            match pruned.len() {
                0 => selection::Set::Empty,
                1 => selection::Set::Single(pruned.into_iter().next().unwrap()),
                _ => selection::Set::Multiple(pruned),
            }
        }

        selection::Set::Empty | selection::Set::All => set.clone(),

        _ => set.clone(),
    }
}

/// Restricts a [`selection::Set`] to keys present in `rows`. `All` is
/// preserved (it logically expands to the registered set).
fn restrict_selection_to_rows(set: &selection::Set, rows: &[Key]) -> selection::Set {
    #[expect(
        clippy::match_same_arms,
        reason = "explicit non-exhaustive wildcard distinguishes Empty/All from unknown future variants"
    )]
    match set {
        selection::Set::Single(key) => {
            if rows.contains(key) {
                selection::Set::Single(key.clone())
            } else {
                selection::Set::Empty
            }
        }

        selection::Set::Multiple(keys) => {
            let pruned = keys
                .iter()
                .filter(|k| rows.contains(k))
                .cloned()
                .collect::<BTreeSet<_>>();

            match pruned.len() {
                0 => selection::Set::Empty,
                1 => selection::Set::Single(pruned.into_iter().next().unwrap()),
                _ => selection::Set::Multiple(pruned),
            }
        }

        selection::Set::Empty | selection::Set::All => set.clone(),

        _ => set.clone(),
    }
}

/// Materializes [`selection::Set::All`] against the registered row set
/// so that `selection::State::deselect` (which is a no-op on `Set::All`)
/// can subsequently drop individual keys. Used by [`Event::ToggleRow`]
/// and [`Event::DeselectRow`] to keep individual-deselect workable after
/// a bulk select-all.
fn materialize_all_against_rows(state: &selection::State, rows: &[Key]) -> selection::State {
    if !matches!(state.selected_keys, selection::Set::All) {
        return state.clone();
    }
    let materialized: BTreeSet<Key> = rows
        .iter()
        .filter(|k| !state.is_disabled(k))
        .cloned()
        .collect();
    let new_keys = match materialized.len() {
        0 => selection::Set::Empty,
        1 => selection::Set::Single(materialized.into_iter().next().unwrap()),
        _ => selection::Set::Multiple(materialized),
    };
    selection::State {
        selected_keys: new_keys,
        ..state.clone()
    }
}

/// Toggle helper that avoids the `Collection` generic on
/// [`selection::State::toggle`]. The Table machine intentionally does
/// not own a `Collection`, so it computes toggle directly. Callers
/// pre-materialize [`selection::Set::All`] via
/// [`materialize_all_against_rows`] so `deselect` can drop individual
/// keys.
fn toggle_state_without_collection(state: &selection::State, key: Key) -> selection::State {
    if state.is_disabled(&key) {
        return state.clone();
    }

    if state.is_selected(&key) {
        state.deselect(&key)
    } else {
        state.select(key)
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the [`Table`](self) component.
///
/// # Examples
///
/// ```
/// use ars_components::data_display::table;
/// use ars_collections::{Key, selection};
/// use ars_core::{Env, Service};
///
/// let props = table::Props {
///     id: "demo".to_string(),
///     selection_mode: selection::Mode::Multiple,
///     ..table::Props::default()
/// };
///
/// let mut service = Service::<table::Machine>::new(
///     props,
///     &Env::default(),
///     &table::Messages::default(),
/// );
///
/// // Register the rendered row list — required for selection /
/// // expansion / focus pruning, and to drive `aria-rowcount`.
/// drop(service.send(table::Event::SetRows(vec![Key::str("r1"), Key::str("r2")])));
/// drop(service.send(table::Event::ToggleRow(Key::str("r1"))));
///
/// let api = service.connect(&|_| {});
///
/// // `role="table"` by default; flip `interactive` for `role="grid"`.
/// assert!(api.is_row_selected(&Key::str("r1")));
/// ```
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();

        let messages = messages.clone();

        let ids = ComponentIds::from_id(&props.id);

        let pruned_default_selection =
            prune_selection_against(&props.default_selected_rows, &props.disabled_keys);

        let pruned_default_expansion = props
            .default_expanded_rows
            .iter()
            .filter(|key| !props.disabled_keys.contains(key))
            .cloned()
            .collect::<BTreeSet<_>>();

        let initial_selection = if let Some(controlled) = &props.selected_rows {
            controlled.clone()
        } else {
            pruned_default_selection.clone()
        };

        let selection_state = selection::State::new(props.selection_mode, props.selection_behavior)
            .with_disabled(props.disabled_keys.clone());

        let selection_state = selection::State {
            selected_keys: initial_selection,
            ..selection_state
        };

        let dir = if props.dir == Direction::Auto {
            // Resolve `Auto → concrete` from the active locale. Adapters
            // can override via `Event::SetDirection` if the platform
            // provides a more authoritative resolution.
            Direction::from(locale.direction())
        } else {
            props.dir
        };

        let ctx = Context {
            selected_rows: if let Some(v) = &props.selected_rows {
                Bindable::controlled(v.clone())
            } else {
                Bindable::uncontrolled(pruned_default_selection)
            },
            selection_state,
            expanded_rows: if let Some(v) = &props.expanded_rows {
                Bindable::controlled(v.clone())
            } else {
                Bindable::uncontrolled(pruned_default_expansion)
            },
            sort_descriptor: props.sort_descriptor.clone(),
            is_sorting: false,
            focused_cell: None,
            focused_row: None,
            focused_col: None,
            disabled_keys: props.disabled_keys.clone(),
            selection_mode: props.selection_mode,
            interactive: props.interactive,
            rows: Vec::new(),
            dir,
            column_widths: BTreeMap::new(),
            resizing_column: None,
            virtual_scrolling: props.virtual_scrolling,
            total_rows: props.total_rows,
            total_cols: props.total_cols,
            loading: props.loading.clone(),
            escape_key_behavior: props.escape_key_behavior,
            select_all_mode: props.select_all_mode.clone(),
            min_column_width: props.min_column_width,
            column_resize_step: props.column_resize_step,
            disallow_empty_selection: props.disallow_empty_selection,
            sticky_header: props.sticky_header,
            locale,
            messages,
            caption_id: ids.part("caption"),
            id: ids.id().to_string(),
        };

        (State::Idle, ctx)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            // ── Sort ─────────────────────────────────────────────────
            // Sort state lives entirely in `ctx.sort_descriptor`. The
            // single-column cycle is None → Ascending → Descending → None.
            Event::SortColumn { column } => {
                let current = ctx.sort_descriptor.get().clone();

                let new_descriptor = if let Some(desc) = current
                    && desc.column == *column
                {
                    match desc.direction {
                        SortDirection::Ascending => Some(SortDescriptor {
                            column: column.clone(),
                            direction: SortDirection::Descending,
                        }),
                        SortDirection::Descending => None,
                    }
                } else {
                    Some(SortDescriptor {
                        column: column.clone(),
                        direction: SortDirection::Ascending,
                    })
                };

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.sort_descriptor.set(new_descriptor);
                    // `ctx.is_sorting` is adapter-controlled — see
                    // `Event::SetIsSorting`. Synchronous sorts complete
                    // in the same render frame as `SortColumn`, so the
                    // agnostic core does not flip the flag itself.
                }))
            }

            // ── Row Action ───────────────────────────────────────────
            Event::RowAction(key) => {
                if ctx.disabled_keys.contains(key) {
                    return None;
                }

                // Notification only — adapter listens for the event.
                Some(TransitionPlan::context_only(|_ctx: &mut Context| {}))
            }

            // ── Selection ────────────────────────────────────────────
            Event::SelectRow(key) => {
                if ctx.selection_mode == selection::Mode::None {
                    return None;
                }

                if ctx.disabled_keys.contains(key) {
                    return None;
                }

                if ctx.selection_state.is_selected(key) {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let new_state = ctx.selection_state.select(key);

                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::DeselectRow(key) => {
                if ctx.selection_mode == selection::Mode::None {
                    return None;
                }

                if ctx.disabled_keys.contains(key) {
                    return None;
                }

                if !ctx.selection_state.is_selected(key) {
                    return None;
                }

                if ctx.disallow_empty_selection
                    && would_empty_after_deselect(&ctx.selection_state.selected_keys, key)
                {
                    return None;
                }

                // Materializing `Set::All` only makes sense when the
                // mode's `Set::All` actually represents "every visible
                // row" — i.e. `SelectAllMode::AllVisible`. In `AllData`
                // the `Set::All` carries the "every row in the dataset
                // (incl. unloaded)" semantic that adapters track with
                // §5.2 `BulkSelection`; the agnostic core must NOT
                // downgrade it to `Multiple(ctx.rows)` because that
                // would drop every unloaded row from selection on the
                // first individual deselect.
                if matches!(ctx.select_all_mode, SelectAllMode::AllData { .. })
                    && matches!(ctx.selected_rows.get(), selection::Set::All)
                {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    // `Set::All` is opaque to `selection::State::deselect`,
                    // so materialize it against the registered row list
                    // first — otherwise individual checkbox uncheck would
                    // silently keep every row selected after a bulk
                    // select-all. The `AllData` short-circuit above
                    // protects against incorrect materialization in
                    // paginated mode.
                    let materialized =
                        materialize_all_against_rows(&ctx.selection_state, &ctx.rows);
                    let new_state = materialized.deselect(&key);

                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state.selected_keys = ctx.selected_rows.get().clone();
                    // Preserve anchor/focus/mode/behavior from the
                    // materialized state.
                    ctx.selection_state.anchor_key = new_state.anchor_key;
                    ctx.selection_state.focused_key = new_state.focused_key;
                }))
            }

            Event::ToggleRow(key) => {
                if ctx.selection_mode == selection::Mode::None {
                    return None;
                }

                if ctx.disabled_keys.contains(key) {
                    return None;
                }

                let is_currently_selected = ctx.selection_state.is_selected(key);

                if is_currently_selected
                    && ctx.disallow_empty_selection
                    && would_empty_after_deselect(&ctx.selection_state.selected_keys, key)
                {
                    return None;
                }

                // Same `AllData` preservation rule as `DeselectRow` —
                // toggling off an individual row in paginated mode would
                // otherwise downgrade `Set::All` → `Multiple(ctx.rows)`.
                let is_toggling_off_set_all =
                    is_currently_selected && matches!(ctx.selected_rows.get(), selection::Set::All);
                if is_toggling_off_set_all
                    && matches!(ctx.select_all_mode, SelectAllMode::AllData { .. })
                {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    // Materialize `Set::All` so the toggle-off path can
                    // actually drop the individual key. See
                    // `materialize_all_against_rows` for the rationale.
                    // (Safe in `AllVisible`; gated out above for
                    // `AllData`.)
                    let materialized =
                        materialize_all_against_rows(&ctx.selection_state, &ctx.rows);
                    let new_state = toggle_state_without_collection(&materialized, key);

                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::SelectAll => {
                if ctx.selection_mode != selection::Mode::Multiple {
                    return None;
                }

                // Materialization differs by SelectAll strategy:
                //
                // - `AllData { total_count }` → write `Set::All`, the
                //   global "every row in the dataset, including unloaded
                //   ones" semantics. The Bindable stays at `Set::All`
                //   until adapter exclusion logic kicks in (§5.2).
                // - `AllVisible` (default) → write `Multiple(ctx.rows)`,
                //   only the rows currently registered. `Set::All` here
                //   would falsely report unloaded rows as selected.
                // - `None` → no SelectAll affordance; reject the event.
                let materialize_as_all = match &ctx.select_all_mode {
                    SelectAllMode::AllData { .. } => true,
                    SelectAllMode::AllVisible => false,
                    SelectAllMode::None => return None,
                };

                let already_all = matches!(ctx.selected_rows.get(), selection::Set::All);
                if materialize_as_all && already_all {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let new_keys = if materialize_as_all {
                        selection::Set::All
                    } else {
                        let visible: BTreeSet<Key> = ctx
                            .rows
                            .iter()
                            .filter(|k| !ctx.disabled_keys.contains(k))
                            .cloned()
                            .collect();
                        match visible.len() {
                            0 => selection::Set::Empty,
                            1 => selection::Set::Single(visible.into_iter().next().unwrap()),
                            _ => selection::Set::Multiple(visible),
                        }
                    };
                    ctx.selection_state.selected_keys = new_keys.clone();
                    ctx.selected_rows.set(new_keys);
                }))
            }

            Event::DeselectAll => {
                if ctx.selection_mode == selection::Mode::None {
                    return None;
                }

                if matches!(ctx.selected_rows.get(), selection::Set::Empty) {
                    return None;
                }

                if ctx.disallow_empty_selection {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let new_state = ctx.selection_state.clear();

                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            // ── Expansion ────────────────────────────────────────────
            Event::ExpandRow(key) => {
                if ctx.disabled_keys.contains(key) {
                    return None;
                }

                if ctx.expanded_rows.get().contains(key) {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut rows = ctx.expanded_rows.get().clone();

                    rows.insert(key);

                    ctx.expanded_rows.set(rows);
                }))
            }

            Event::CollapseRow(key) => {
                if !ctx.expanded_rows.get().contains(key) {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut rows = ctx.expanded_rows.get().clone();

                    rows.remove(&key);

                    ctx.expanded_rows.set(rows);
                }))
            }

            // ── Grid focus ───────────────────────────────────────────
            Event::Focus { cell } => {
                if !ctx.interactive {
                    return None;
                }

                let c = *cell;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_cell = Some(c);
                }))
            }

            Event::Blur => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused_cell = None;
                ctx.focused_row = None;
                ctx.focused_col = None;
            })),

            Event::FocusRow(key) => {
                if !ctx.interactive {
                    return None;
                }

                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_row = Some(key);
                }))
            }

            Event::FocusCell {
                row,
                col,
                row_index,
            } => {
                if !ctx.interactive {
                    return None;
                }

                let row = row.clone();
                let col = *col;
                let row_index = *row_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_cell = Some((col, row_index));
                    ctx.focused_row = Some(row);
                    ctx.focused_col = Some(col);
                }))
            }

            // ── Escape key ───────────────────────────────────────────
            Event::EscapeKey => {
                if ctx.escape_key_behavior != EscapeKeyBehavior::ClearSelection {
                    return None;
                }

                if ctx.disallow_empty_selection {
                    return None;
                }

                if matches!(ctx.selected_rows.get(), selection::Set::Empty) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let new_state = ctx.selection_state.clear();

                    ctx.selected_rows.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            // ── Row registry ─────────────────────────────────────────
            Event::SetRows(new_rows) => {
                let rows = new_rows.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let restricted_selection =
                        restrict_selection_to_rows(&ctx.selected_rows.get().clone(), &rows);

                    let restricted_expansion = ctx
                        .expanded_rows
                        .get()
                        .iter()
                        .filter(|k| rows.contains(k))
                        .cloned()
                        .collect::<BTreeSet<_>>();

                    // Rebase focus against the new row list:
                    //   * row disappeared → clear focus entirely
                    //   * row remained but moved index → update
                    //     `focused_cell.1` so adapter focus wiring
                    //     targets the right cell after sort / filter /
                    //     reorder
                    if let Some(focused) = ctx.focused_row.clone() {
                        if let Some(new_index) = rows.iter().position(|k| k == &focused) {
                            if let Some((col, _old_index)) = ctx.focused_cell {
                                ctx.focused_cell = Some((col, new_index));
                            }
                        } else {
                            ctx.focused_row = None;
                            ctx.focused_cell = None;
                            ctx.focused_col = None;
                        }
                    }

                    // `set(...)` updates the internal fallback only;
                    // for controlled Bindables, `get()` still returns
                    // the parent value. Re-sync
                    // `selection_state.selected_keys` from `get()` so
                    // transition guards (`is_selected`, etc.) agree
                    // with what the API surfaces — otherwise a split
                    // brain forms after the first row-list change in
                    // controlled mode.
                    ctx.selected_rows.set(restricted_selection);
                    ctx.selection_state.selected_keys = ctx.selected_rows.get().clone();
                    ctx.expanded_rows.set(restricted_expansion);
                    ctx.rows = rows;
                }))
            }

            // ── Direction ────────────────────────────────────────────
            Event::SetDirection(dir) => {
                if ctx.dir == *dir {
                    return None;
                }

                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            // ── Virtual scroll counts ────────────────────────────────
            Event::SetRowCounts {
                total_rows,
                total_cols,
            } => {
                if ctx.total_rows == *total_rows && ctx.total_cols == *total_cols {
                    return None;
                }

                let total_rows = *total_rows;
                let total_cols = *total_cols;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.total_rows = total_rows;
                    ctx.total_cols = total_cols;
                }))
            }

            // ── Async loading ────────────────────────────────────────
            Event::SetLoading(value) => {
                if *ctx.loading.get() == *value {
                    return None;
                }

                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.loading.set(value);
                }))
            }

            Event::SetIsSorting(value) => {
                if ctx.is_sorting == *value {
                    return None;
                }
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.is_sorting = value;
                }))
            }

            // ── Column resize ────────────────────────────────────────
            Event::ColumnResize { column, width } => {
                let min = ctx.min_column_width;

                let clamped = if *width < min { min } else { *width };
                let column = column.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.column_widths.insert(column.clone(), clamped);
                    ctx.resizing_column = Some(column);
                }))
            }

            Event::ColumnResizeEnd { column } => {
                if ctx.resizing_column.as_deref() != Some(column.as_str()) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.resizing_column = None;
                }))
            }

            // ── Prop sync ────────────────────────────────────────────
            // Mirror every non-Bindable Props field into Context, then
            // re-prune selection / expansion against the (possibly new)
            // disabled set. Bindable fields are pushed via the
            // `SyncControlled*` events.
            Event::SyncProps => {
                let new_id = props.id.clone();
                let new_disabled = props.disabled_keys.clone();
                let new_selection_mode = props.selection_mode;
                let new_selection_behavior = props.selection_behavior;
                let new_interactive = props.interactive;
                let new_sticky_header = props.sticky_header;
                let new_escape_key_behavior = props.escape_key_behavior;
                let new_select_all_mode = props.select_all_mode.clone();
                let new_min_column_width = props.min_column_width;
                let new_column_resize_step = props.column_resize_step;
                let new_virtual_scrolling = props.virtual_scrolling;
                let new_total_rows = props.total_rows;
                let new_total_cols = props.total_cols;
                let new_disallow_empty_selection = props.disallow_empty_selection;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    // Rebuild ComponentIds-derived strings when `Props.id`
                    // changes — otherwise `table_attrs`,
                    // `caption_attrs`, and expand-trigger
                    // `aria-controls` ids stay stale.
                    if ctx.id != new_id {
                        let ids = ComponentIds::from_id(&new_id);
                        ctx.id = ids.id().to_string();
                        ctx.caption_id = ids.part("caption");
                    }

                    ctx.disabled_keys = new_disabled.clone();
                    ctx.selection_mode = new_selection_mode;
                    ctx.selection_state.mode = new_selection_mode;
                    ctx.selection_state.behavior = new_selection_behavior;
                    ctx.selection_state.disabled_keys = new_disabled.clone();
                    ctx.interactive = new_interactive;
                    ctx.sticky_header = new_sticky_header;
                    ctx.escape_key_behavior = new_escape_key_behavior;
                    ctx.select_all_mode = new_select_all_mode;
                    ctx.min_column_width = new_min_column_width;
                    ctx.column_resize_step = new_column_resize_step;
                    ctx.virtual_scrolling = new_virtual_scrolling;
                    ctx.total_rows = new_total_rows;
                    ctx.total_cols = new_total_cols;
                    ctx.disallow_empty_selection = new_disallow_empty_selection;

                    // Pruning rules:
                    //   * uncontrolled Bindable — write the pruned value
                    //     directly via `set()`. The agnostic core owns
                    //     the value, so it can clean it.
                    //   * controlled Bindable — the parent owns the value;
                    //     `set()` would update the internal field but
                    //     `get()` would still return the external
                    //     controlled value, silently desyncing
                    //     `selection_state.selected_keys` from the
                    //     user-visible selection. Skip the pruning write
                    //     and let the parent pass a clean value next render.
                    if !ctx.selected_rows.is_controlled() {
                        let pruned_selection = prune_selection_against(
                            &ctx.selected_rows.get().clone(),
                            &new_disabled,
                        );
                        ctx.selected_rows.set(pruned_selection);
                    }
                    if !ctx.expanded_rows.is_controlled() {
                        let pruned_expansion: BTreeSet<Key> = ctx
                            .expanded_rows
                            .get()
                            .iter()
                            .filter(|k| !new_disabled.contains(k))
                            .cloned()
                            .collect();
                        ctx.expanded_rows.set(pruned_expansion);
                    }

                    // Re-sync `selection_state.selected_keys` from the
                    // user-visible selection so it tracks the actual
                    // current value regardless of controlled-mode.
                    ctx.selection_state.selected_keys = ctx.selected_rows.get().clone();
                }))
            }

            Event::SyncControlledSelectedRows(value) => {
                let v = value.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.selected_rows.sync_controlled(v);

                    // Re-sync `selection_state.selected_keys` from
                    // `get()` so it reflects the user-visible selection
                    // regardless of whether we just entered, updated,
                    // or left controlled mode. Leaving controlled
                    // (`None` payload) flips `get()` back to the
                    // internal fallback, which may differ from the
                    // last controlled value.
                    ctx.selection_state.selected_keys = ctx.selected_rows.get().clone();
                }))
            }

            Event::SyncControlledExpandedRows(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.expanded_rows.sync_controlled(value);
                }))
            }

            Event::SyncControlledSortDescriptor(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    // The outer `Option` is the controlled-mode toggle —
                    // pass it straight through. `Some(opt)` enters /
                    // updates controlled mode (with `opt` carrying the
                    // inner sort value); `None` leaves controlled mode.
                    ctx.sort_descriptor.sync_controlled(value);
                }))
            }

            Event::SyncControlledLoading(value) => {
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.loading.sync_controlled(value);
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();

        // `dir` flows through `SetDirection`, not `SyncProps`, so a
        // runtime-resolved direction (e.g. one the adapter installed via
        // `SetDirection` after resolving `Direction::Auto`) is not
        // overwritten by an unrelated prop delta. `SetDirection` is
        // idempotent.
        if old.dir != new.dir {
            events.push(Event::SetDirection(new.dir));
        }

        if non_dir_context_props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        // Each controlled Bindable has its own sync event because the
        // outer `Option` is the controlled-mode toggle and the inner
        // value carries the new content.
        if old.selected_rows != new.selected_rows {
            events.push(Event::SyncControlledSelectedRows(new.selected_rows.clone()));
        }
        if old.expanded_rows != new.expanded_rows {
            events.push(Event::SyncControlledExpandedRows(new.expanded_rows.clone()));
        }

        // Sort + loading Bindables: emit on any controlled-mode
        // transition (entering, exiting, or value change while
        // controlled) so parent renders can both push new controlled
        // values AND drop back to uncontrolled.
        if bindable_controlled_state_changed(&old.sort_descriptor, &new.sort_descriptor) {
            events.push(Event::SyncControlledSortDescriptor(bindable_sync_payload(
                &new.sort_descriptor,
            )));
        }
        if bindable_controlled_state_changed(&old.loading, &new.loading) {
            events.push(Event::SyncControlledLoading(bindable_sync_payload(
                &new.loading,
            )));
        }

        events
    }
}

/// Detects any controlled-mode delta between two Bindables that warrants
/// a sync event:
///
/// * Both controlled, values differ.
/// * One side controlled, the other not.
fn bindable_controlled_state_changed<T>(old: &Bindable<T>, new: &Bindable<T>) -> bool
where
    T: ars_core::BindableValue,
{
    if old.is_controlled() != new.is_controlled() {
        return true;
    }
    if new.is_controlled() && old.get() != new.get() {
        return true;
    }
    false
}

/// Translates a Bindable into the `Option<T>` payload expected by
/// [`Bindable::sync_controlled`]: `Some(value)` to enter/update
/// controlled mode, `None` to exit.
fn bindable_sync_payload<T>(b: &Bindable<T>) -> Option<T>
where
    T: ars_core::BindableValue,
{
    if b.is_controlled() {
        Some(b.get().clone())
    } else {
        None
    }
}

/// Returns `true` when any non-Bindable, non-direction Props field
/// changed between renders. Drives the [`Event::SyncProps`] dispatch
/// inside [`Machine::on_props_changed`].
fn non_dir_context_props_changed(old: &Props, new: &Props) -> bool {
    old.id != new.id
        || old.selection_mode != new.selection_mode
        || old.selection_behavior != new.selection_behavior
        || old.disabled_keys != new.disabled_keys
        || old.disallow_empty_selection != new.disallow_empty_selection
        || old.escape_key_behavior != new.escape_key_behavior
        || old.select_all_mode != new.select_all_mode
        || old.interactive != new.interactive
        || old.sticky_header != new.sticky_header
        || old.min_column_width.to_bits() != new.min_column_width.to_bits()
        || old.column_resize_step.to_bits() != new.column_resize_step.to_bits()
        || old.virtual_scrolling != new.virtual_scrolling
        || old.total_rows != new.total_rows
        || old.total_cols != new.total_cols
}

/// Returns `true` when deselecting `key` would leave the selection empty.
fn would_empty_after_deselect(set: &selection::Set, key: &Key) -> bool {
    match set {
        selection::Set::Single(k) => k == key,

        selection::Set::Multiple(keys) => keys.len() == 1 && keys.contains(key),

        // `Empty` / `All` / unknown non-exhaustive variants are not at
        // risk of becoming empty from a single deselect.
        _ => false,
    }
}

/// Walks `all_row_ids` starting from `current` (exclusive) in the
/// indicated direction (`forward = true` for next, `false` for
/// previous) and returns the first non-disabled row key. Returns `None`
/// when no enabled row exists in that direction.
fn next_enabled_row(
    all_row_ids: &[&Key],
    current: usize,
    disabled: &BTreeSet<Key>,
    forward: bool,
) -> Option<Key> {
    next_enabled_row_index(all_row_ids, current, disabled, forward).map(|i| all_row_ids[i].clone())
}

/// Index-returning variant of [`next_enabled_row`], used by
/// `on_cell_keydown` which also needs the index to populate
/// `Event::FocusCell::row_index`.
fn next_enabled_row_index(
    all_row_ids: &[&Key],
    current: usize,
    disabled: &BTreeSet<Key>,
    forward: bool,
) -> Option<usize> {
    if forward {
        ((current + 1)..all_row_ids.len()).find(|&i| !disabled.contains(all_row_ids[i]))
    } else {
        (0..current)
            .rev()
            .find(|&i| !disabled.contains(all_row_ids[i]))
    }
}

/// Returns the first or last enabled row key. `from_start = true` walks
/// forward from index 0; `false` walks backward from the end (for
/// `End` / `Ctrl+End`). Returns `None` when every row is disabled.
fn first_enabled_row(
    all_row_ids: &[&Key],
    disabled: &BTreeSet<Key>,
    from_start: bool,
) -> Option<Key> {
    first_enabled_row_index(all_row_ids, disabled, from_start).map(|i| all_row_ids[i].clone())
}

/// Index variant of [`first_enabled_row`].
fn first_enabled_row_index(
    all_row_ids: &[&Key],
    disabled: &BTreeSet<Key>,
    from_start: bool,
) -> Option<usize> {
    if from_start {
        (0..all_row_ids.len()).find(|&i| !disabled.contains(all_row_ids[i]))
    } else {
        (0..all_row_ids.len())
            .rev()
            .find(|&i| !disabled.contains(all_row_ids[i]))
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connect API for the [`Table`](self) component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Debug for Api<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl<'a> Api<'a> {
    /// Returns the underlying machine state.
    #[must_use]
    pub const fn state(&self) -> &State {
        self.state
    }

    /// Returns the underlying context.
    #[must_use]
    pub const fn context(&self) -> &Context {
        self.ctx
    }

    /// Returns the underlying props.
    #[must_use]
    pub const fn props(&self) -> &Props {
        self.props
    }

    // ── Helpers ───────────────────────────────────────────────────────

    /// Returns `true` when `row_id` is currently selected.
    #[must_use]
    pub fn is_row_selected(&self, row_id: &Key) -> bool {
        self.ctx.selected_rows.get().contains(row_id)
    }

    /// Returns `true` when `row_id` is currently expanded.
    #[must_use]
    pub fn is_row_expanded(&self, row_id: &Key) -> bool {
        self.ctx.expanded_rows.get().contains(row_id)
    }

    /// Returns `true` when `row_id` is in the disabled set.
    #[must_use]
    pub fn is_row_disabled(&self, row_id: &Key) -> bool {
        self.ctx.disabled_keys.contains(row_id)
    }

    /// Returns the current sort descriptor.
    #[must_use]
    pub fn sort_descriptor(&self) -> Option<&SortDescriptor<String>> {
        self.ctx.sort_descriptor.get().as_ref()
    }

    /// Returns the active sort column, if any.
    #[must_use]
    pub fn active_sort_column(&self) -> Option<&String> {
        self.sort_descriptor().map(|d| &d.column)
    }

    /// Returns the current sort direction for the active sort column.
    /// Returns `None` when no column is sorted (per
    /// [`SortDirection`]'s definition `Ascending`/`Descending` only).
    #[must_use]
    pub fn sort_direction(&self) -> Option<SortDirection> {
        self.sort_descriptor().map(|d| d.direction)
    }

    /// Returns `true` when every key in `all_row_ids` is currently
    /// selected. Handles [`selection::Set::All`] efficiently.
    #[must_use]
    pub fn all_selected(&self, all_row_ids: &[&Key]) -> bool {
        let sel = self.ctx.selected_rows.get();

        if sel.is_all() {
            return true;
        }

        if all_row_ids.is_empty() {
            return false;
        }

        all_row_ids.iter().all(|id| sel.contains(id))
    }

    /// Returns `true` when at least one key in `all_row_ids` is
    /// currently selected.
    #[must_use]
    pub fn some_selected(&self, all_row_ids: &[&Key]) -> bool {
        let sel = self.ctx.selected_rows.get();

        if sel.is_all() {
            return !all_row_ids.is_empty();
        }

        all_row_ids.iter().any(|id| sel.contains(id))
    }

    // ── Root / Table ──────────────────────────────────────────────────

    /// Returns the attribute map for the wrapper `<div>`.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(&mut attrs, &Part::Root);

        if self.ctx.sticky_header {
            attrs.set_bool(HtmlAttr::Data("ars-sticky-header"), true);
        }

        attrs
    }

    /// Returns the attribute map for the `<table>` element.
    #[must_use]
    pub fn table_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, self.ctx.id.clone());

        set_part(&mut attrs, &Part::Table);

        if self.ctx.interactive {
            attrs.set(HtmlAttr::Role, "grid");
            // Advertise multi-select on the grid root so assistive tech
            // doesn't read the table as single-select when the user can
            // actually pick multiple rows.
            if self.ctx.selection_mode == selection::Mode::Multiple {
                attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
            }
        } else {
            attrs.set(HtmlAttr::Role, "table");
        }

        if self.props.caption.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.caption_id.clone(),
            );
        }

        if self.ctx.virtual_scrolling {
            attrs
                .set(
                    HtmlAttr::Aria(AriaAttr::RowCount),
                    self.ctx.total_rows.to_string(),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::ColCount),
                    self.ctx.total_cols.to_string(),
                );
        }

        attrs
    }

    /// Returns the attribute map for the `<caption>` element.
    #[must_use]
    pub fn caption_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Id, self.ctx.caption_id.clone());

        set_part(&mut attrs, &Part::Caption);

        attrs
    }

    /// Returns the attribute map for the `<thead>` element.
    #[must_use]
    pub fn head_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(&mut attrs, &Part::Head);

        if self.ctx.sticky_header {
            attrs.set_bool(HtmlAttr::Data("ars-sticky"), true);
        }

        attrs
    }

    /// Returns the attribute map for the `<tbody>` element.
    #[must_use]
    pub fn body_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(&mut attrs, &Part::Body);

        attrs
    }

    /// Returns the attribute map for the `<tfoot>` element.
    #[must_use]
    pub fn foot_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        set_part(&mut attrs, &Part::Foot);
        attrs
    }

    // ── Rows ──────────────────────────────────────────────────────────

    /// Returns the attribute map for a `<tr>` row.
    ///
    /// `row_index` is the zero-based position of the row in the rendered
    /// dataset; it becomes `aria-rowindex = row_index + 1` when virtual
    /// scrolling is active. For non-virtualized callers, pass `0` or
    /// use [`Self::row_attrs`].
    #[must_use]
    pub fn row_attrs_indexed(&self, row_id: &Key, row_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::Row {
                key: Key::default(),
            },
        );

        let selected = self.is_row_selected(row_id);
        let expanded = self.is_row_expanded(row_id);
        let disabled = self.is_row_disabled(row_id);

        if self.ctx.selection_mode != selection::Mode::None {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if selected { "true" } else { "false" },
            );

            if selected {
                attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
            }
        }

        if expanded {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Expanded), "true")
                .set_bool(HtmlAttr::Data("ars-expanded"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.interactive {
            attrs.set(HtmlAttr::Role, "row");
        }

        if self.ctx.virtual_scrolling {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::RowIndex),
                (row_index + 1).to_string(),
            );
        }
        attrs
    }

    /// Returns the attribute map for a `<tr>` row using `row_index = 0`.
    /// Convenience for non-virtualized tables.
    #[must_use]
    pub fn row_attrs(&self, row_id: &Key) -> AttrMap {
        self.row_attrs_indexed(row_id, 0)
    }

    /// Returns row attributes for a row that acts as a link.
    /// Convenience for non-virtualized callers; passes `row_index = 0`.
    /// Virtualized tables MUST call [`Self::row_link_attrs_indexed`] so
    /// `aria-rowindex` reflects the real position.
    #[must_use]
    pub fn row_link_attrs(&self, row_id: &Key, href: &str) -> AttrMap {
        self.row_link_attrs_indexed(row_id, href, 0)
    }

    /// Returns row attributes for a linked row at the supplied row
    /// index. Use this variant inside a virtualized list so the
    /// `aria-rowindex` adapter output reflects the row's true position
    /// in the dataset (not always `1`).
    #[must_use]
    pub fn row_link_attrs_indexed(&self, row_id: &Key, href: &str, row_index: usize) -> AttrMap {
        let mut attrs = self.row_attrs_indexed(row_id, row_index);

        attrs.set(HtmlAttr::Data("ars-href"), href.to_owned());

        attrs
    }

    // ── Headers / cells ───────────────────────────────────────────────

    /// Returns the attribute map for a `<th scope="col">` column header.
    ///
    /// Non-sortable columns omit `aria-sort` entirely per spec §3.4.2.
    #[must_use]
    pub fn column_header_attrs(&self, column: &str, sortable: bool) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::ColumnHeader {
                header: String::new(),
            },
        );

        attrs.set(HtmlAttr::Scope, "col");

        if sortable {
            let is_sorted = self.active_sort_column().is_some_and(|c| c == column);

            let aria_sort = if is_sorted {
                match self.sort_direction() {
                    Some(SortDirection::Ascending) => "ascending",
                    Some(SortDirection::Descending) => "descending",
                    None => "none",
                }
            } else {
                "none"
            };

            attrs
                .set(HtmlAttr::Aria(AriaAttr::Sort), aria_sort)
                .set(HtmlAttr::Data("ars-sort"), aria_sort);

            if is_sorted {
                attrs.set_bool(HtmlAttr::Data("ars-sorted"), true);
            }

            attrs.set(HtmlAttr::TabIndex, "0");
        }
        attrs
    }

    /// Returns the attribute map for a `<th scope="row">` row header cell.
    #[must_use]
    pub fn row_header_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(&mut attrs, &Part::RowHeader);

        attrs.set(HtmlAttr::Scope, "row");

        attrs
    }

    /// Returns the attribute map for a `<td>` cell.
    ///
    /// Manages a roving `tabindex` when the table is `interactive`.
    #[must_use]
    pub fn cell_attrs(&self, col: usize, row: usize) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(&mut attrs, &Part::Cell { col, row });

        if self.ctx.interactive {
            let focused = self.ctx.focused_cell == Some((col, row));

            attrs.set(HtmlAttr::TabIndex, if focused { "0" } else { "-1" });
        }

        attrs
    }

    // ── Selection controls ────────────────────────────────────────────

    /// Returns the attribute map for the `SelectAll` checkbox.
    ///
    /// Returns an empty map when
    /// [`SelectAllMode::None`](crate::data_display::table::SelectAllMode::None)
    /// is configured.
    #[must_use]
    pub fn select_all_attrs(&self, all_row_ids: &[&Key]) -> AttrMap {
        let mut attrs = AttrMap::new();

        // `Event::SelectAll` is only honored in `Mode::Multiple`. Rendering
        // the checkbox in other modes would surface a control the user
        // can click but the machine will reject — so short-circuit here
        // and let adapters render only when the mode supports it.
        if self.ctx.selection_mode != selection::Mode::Multiple {
            return attrs;
        }

        if matches!(self.ctx.select_all_mode, SelectAllMode::None) {
            return attrs;
        }

        set_part(&mut attrs, &Part::SelectAllCheckbox);

        attrs.set(HtmlAttr::Type, "checkbox");

        let count = if let SelectAllMode::AllData { total_count } = &self.ctx.select_all_mode {
            *total_count
        } else {
            all_row_ids.len()
        };

        let label = (self.ctx.messages.select_all)(count, &self.ctx.locale);

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);

        let all = self.all_selected(all_row_ids);

        let some = !all && self.some_selected(all_row_ids);

        let aria_checked = if all {
            "true"
        } else if some {
            "mixed"
        } else {
            "false"
        };

        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), aria_checked);

        if all {
            attrs.set_bool(HtmlAttr::Checked, true);
        }

        if some {
            attrs.set_bool(HtmlAttr::Data("ars-indeterminate"), true);
        }

        attrs
    }

    /// Returns the attribute map for the per-row selection checkbox.
    #[must_use]
    pub fn row_checkbox_attrs(&self, row_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::RowCheckbox {
                key: Key::default(),
            },
        );

        attrs.set(HtmlAttr::Type, "checkbox");

        let selected = self.is_row_selected(row_id);

        let label = (self.ctx.messages.select_row)(&self.ctx.locale);

        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label).set(
            HtmlAttr::Aria(AriaAttr::Checked),
            if selected { "true" } else { "false" },
        );

        if selected {
            attrs.set_bool(HtmlAttr::Checked, true);
        }

        if self.is_row_disabled(row_id) {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    // ── Expand controls ───────────────────────────────────────────────

    /// Returns the attribute map for an expand trigger `<button>`.
    #[must_use]
    pub fn expand_trigger_attrs(&self, row_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::ExpandTrigger {
                key: Key::default(),
            },
        );

        let expanded = self.is_row_expanded(row_id);
        let detail_id = format!("{}-expanded-{}", self.ctx.id, row_id);

        attrs
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if expanded { "true" } else { "false" },
            )
            .set(HtmlAttr::Aria(AriaAttr::Controls), detail_id);

        if expanded {
            attrs.set_bool(HtmlAttr::Data("ars-expanded"), true);
        }

        if self.is_row_disabled(row_id) {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns the attribute map for the expanded detail `<tr>`.
    #[must_use]
    pub fn expanded_content_attrs(&self, row_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::ExpandedContent {
                key: Key::default(),
            },
        );

        let detail_id = format!("{}-expanded-{}", self.ctx.id, row_id);

        attrs.set(HtmlAttr::Id, detail_id);

        let expanded = self.is_row_expanded(row_id);

        if expanded {
            attrs.set_bool(HtmlAttr::Data("ars-expanded"), true);
        } else {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    // ── Column resize handle (§6) ─────────────────────────────────────

    /// Returns the attribute map for a column resize handle.
    #[must_use]
    pub fn column_resize_handle_attrs(&self, column: &str, current_width: f64) -> AttrMap {
        let mut attrs = AttrMap::new();

        set_part(
            &mut attrs,
            &Part::ColumnResizeHandle {
                column: String::new(),
            },
        );

        let effective_width = self
            .ctx
            .column_widths
            .get(column)
            .copied()
            .unwrap_or(current_width);

        attrs
            .set(HtmlAttr::Role, "separator")
            .set(HtmlAttr::Aria(AriaAttr::Orientation), "vertical")
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                format_width(effective_width),
            );

        if self.ctx.resizing_column.as_deref() == Some(column) {
            attrs.set_bool(HtmlAttr::Data("ars-resizing"), true);
        }

        attrs.set(HtmlAttr::TabIndex, "0");

        attrs
    }

    // ── Keyboard navigation ───────────────────────────────────────────

    /// Handle a keydown event delivered to a row element. Emits the
    /// appropriate [`Event::FocusRow`] / [`Event::ToggleRow`] /
    /// [`Event::EscapeKey`] event.
    pub fn on_row_keydown(&self, row_id: &Key, data: &KeyboardEventData, all_row_ids: &[&Key]) {
        let current_idx = all_row_ids.iter().position(|id| *id == row_id);

        match data.key {
            KeyboardKey::ArrowDown => {
                if let Some(idx) = current_idx
                    && let Some(next) =
                        next_enabled_row(all_row_ids, idx, &self.ctx.disabled_keys, true)
                {
                    (self.send)(Event::FocusRow(next));
                }
            }

            KeyboardKey::ArrowUp => {
                if let Some(idx) = current_idx
                    && let Some(prev) =
                        next_enabled_row(all_row_ids, idx, &self.ctx.disabled_keys, false)
                {
                    (self.send)(Event::FocusRow(prev));
                }
            }

            KeyboardKey::Home => {
                if let Some(first) = first_enabled_row(all_row_ids, &self.ctx.disabled_keys, true) {
                    (self.send)(Event::FocusRow(first));
                }
            }

            KeyboardKey::End => {
                if let Some(last) = first_enabled_row(all_row_ids, &self.ctx.disabled_keys, false) {
                    (self.send)(Event::FocusRow(last));
                }
            }

            KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::ToggleRow(row_id.clone()));
            }

            KeyboardKey::Escape => {
                (self.send)(Event::EscapeKey);
            }

            _ => {}
        }
    }

    /// Handle a keydown event delivered to a cell element when the table
    /// is `interactive=true`. Resolves Arrow Left/Right against
    /// [`Context::dir`] so Arrow Right always moves toward the logical
    /// "end" column.
    pub fn on_cell_keydown(
        &self,
        row_id: &Key,
        col: usize,
        data: &KeyboardEventData,
        all_row_ids: &[&Key],
        col_count: usize,
    ) {
        let current_row_idx = all_row_ids.iter().position(|id| *id == row_id);

        let is_rtl = self.ctx.dir == Direction::Rtl;

        let (next_col_key, prev_col_key) = if is_rtl {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        } else {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        };

        match data.key {
            key if key == next_col_key => {
                if col + 1 < col_count
                    && let Some(idx) = current_row_idx
                {
                    (self.send)(Event::FocusCell {
                        row: row_id.clone(),
                        col: col + 1,
                        row_index: idx,
                    });
                }
            }

            key if key == prev_col_key => {
                if col > 0
                    && let Some(idx) = current_row_idx
                {
                    (self.send)(Event::FocusCell {
                        row: row_id.clone(),
                        col: col - 1,
                        row_index: idx,
                    });
                }
            }

            KeyboardKey::ArrowDown => {
                if let Some(idx) = current_row_idx
                    && let Some(next_idx) =
                        next_enabled_row_index(all_row_ids, idx, &self.ctx.disabled_keys, true)
                {
                    (self.send)(Event::FocusCell {
                        row: all_row_ids[next_idx].clone(),
                        col,
                        row_index: next_idx,
                    });
                }
            }

            KeyboardKey::ArrowUp => {
                if let Some(idx) = current_row_idx
                    && let Some(prev_idx) =
                        next_enabled_row_index(all_row_ids, idx, &self.ctx.disabled_keys, false)
                {
                    (self.send)(Event::FocusCell {
                        row: all_row_ids[prev_idx].clone(),
                        col,
                        row_index: prev_idx,
                    });
                }
            }

            KeyboardKey::Home if data.ctrl_key => {
                if let Some(first_idx) =
                    first_enabled_row_index(all_row_ids, &self.ctx.disabled_keys, true)
                {
                    (self.send)(Event::FocusCell {
                        row: all_row_ids[first_idx].clone(),
                        col: 0,
                        row_index: first_idx,
                    });
                }
            }

            KeyboardKey::End if data.ctrl_key => {
                if let Some(last_idx) =
                    first_enabled_row_index(all_row_ids, &self.ctx.disabled_keys, false)
                {
                    (self.send)(Event::FocusCell {
                        row: all_row_ids[last_idx].clone(),
                        col: col_count.saturating_sub(1),
                        row_index: last_idx,
                    });
                }
            }

            KeyboardKey::Home => {
                if let Some(idx) = current_row_idx {
                    (self.send)(Event::FocusCell {
                        row: row_id.clone(),
                        col: 0,
                        row_index: idx,
                    });
                }
            }

            KeyboardKey::End => {
                if let Some(idx) = current_row_idx {
                    (self.send)(Event::FocusCell {
                        row: row_id.clone(),
                        col: col_count.saturating_sub(1),
                        row_index: idx,
                    });
                }
            }

            KeyboardKey::Space => {
                (self.send)(Event::ToggleRow(row_id.clone()));
            }

            KeyboardKey::Escape => {
                (self.send)(Event::EscapeKey);
            }

            _ => {}
        }
    }

    /// Handle a keydown event delivered to a column resize handle.
    /// Emits [`Event::ColumnResize`] with the current width adjusted by
    /// [`Context::column_resize_step`], honoring RTL.
    pub fn on_resize_handle_keydown(
        &self,
        column: &str,
        current_width: f64,
        data: &KeyboardEventData,
    ) {
        let step = self.ctx.column_resize_step;

        let is_rtl = self.ctx.dir == Direction::Rtl;

        let (grow_key, shrink_key) = if is_rtl {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        } else {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        };

        let next_width = match data.key {
            key if key == grow_key => Some(current_width + step),
            key if key == shrink_key => Some(current_width - step),
            _ => None,
        };

        if let Some(width) = next_width {
            (self.send)(Event::ColumnResize {
                column: column.to_owned(),
                width,
            });
        }
    }
}

impl<'a> ConnectApi for Api<'a> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Table => self.table_attrs(),
            Part::Caption => self.caption_attrs(),
            Part::Head => self.head_attrs(),
            Part::Body => self.body_attrs(),
            Part::Foot => self.foot_attrs(),
            Part::Row { key } => self.row_attrs(key),
            Part::ColumnHeader { header } => self.column_header_attrs(header, true),
            Part::RowHeader => self.row_header_attrs(),
            Part::Cell { col, row } => self.cell_attrs(*col, *row),
            Part::SelectAllCheckbox => {
                // Drive checked / mixed / aria-checked from the
                // registered row list. The typed `select_all_attrs`
                // method is what adapters should call directly with
                // their per-render visible-row slice — the dispatcher
                // path is for tooling that walks every Part and needs
                // the live state to reflect the machine's row registry.
                let rows: Vec<&Key> = self.ctx.rows.iter().collect();
                self.select_all_attrs(&rows)
            }
            Part::RowCheckbox { key } => self.row_checkbox_attrs(key),
            Part::ExpandTrigger { key } => self.expand_trigger_attrs(key),
            Part::ExpandedContent { key } => self.expanded_content_attrs(key),
            Part::ColumnResizeHandle { column } => self.column_resize_handle_attrs(column, 0.0),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────

/// Sets the canonical `data-ars-scope` / `data-ars-part` attribute pair.
fn set_part(attrs: &mut AttrMap, part: &Part) {
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
}

/// Renders an `aria-valuenow` width value to a clean string, trimming a
/// trailing `.0` when the width is integer-valued.
fn format_width(width: f64) -> String {
    if width.fract() == 0.0 {
        format!("{width:.0}")
    } else {
        format!("{width}")
    }
}
