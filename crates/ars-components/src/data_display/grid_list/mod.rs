//! GridList data-display component machine.
//!
//! Owns item keys, 2D logical focus, selection state, typeahead state,
//! virtualized active-descendant IDs, and ARIA/data attributes. Adapters
//! perform live DOM focus, scroll-into-view, measurement, drag/drop event
//! plumbing, and loading-sentinel observation with native handles.

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString as _},
    sync::Arc,
};
use core::{
    fmt::{self, Debug},
    num::NonZeroUsize,
    time::Duration,
};

use ars_collections::{
    Collection, CollectionDropTarget, CollectionItem, DisabledBehavior, Key, StaticCollection,
    selection, typeahead,
};
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HasId, HtmlAttr, Locale, MessageFn, SafeUrl, TransitionPlan,
    sanitize_url,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

type CellPositionFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;
type DragHandleLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;
type GridDimensionsFn = dyn Fn(usize, usize, &Locale) -> String + Send + Sync;

/// Controls behavior when Escape is pressed while items are selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EscapeKeyBehavior {
    /// Escape clears the current selection.
    #[default]
    ClearSelection,

    /// Escape is not handled by the `GridList`.
    None,
}

/// Definition of a grid-list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemDef {
    /// Unique identifier for this grid item.
    pub key: Key,

    /// Localized text label used for typeahead and adapter-rendered content.
    pub label: String,

    /// Whether this individual item is disabled.
    pub disabled: bool,

    /// Optional validated navigation URL for link-capable items.
    pub href: Option<SafeUrl>,
}

impl CollectionItem for ItemDef {
    fn key(&self) -> &Key {
        &self.key
    }

    fn text_value(&self) -> &str {
        &self.label
    }
}

/// Props for the `GridList` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Grid items to display.
    pub items: StaticCollection<ItemDef>,

    /// Number of columns in the logical grid layout.
    pub columns: Option<NonZeroUsize>,

    /// Controlled selected keys.
    pub selected_keys: Option<BTreeSet<Key>>,

    /// Default selected keys for uncontrolled mode.
    pub default_selected_keys: BTreeSet<Key>,

    /// Selection mode for grid items.
    pub selection_mode: selection::Mode,

    /// Selection behavior for item activation in selectable modes.
    pub selection_behavior: selection::Behavior,

    /// Disable the entire grid.
    pub disabled: bool,

    /// Keys of individually disabled items.
    pub disabled_keys: BTreeSet<Key>,

    /// Accessible label for the grid.
    pub label: Option<String>,

    /// Prevents deselecting the last remaining selected item.
    pub disallow_empty_selection: bool,

    /// Controls behavior when Escape is pressed while items are selected.
    pub escape_key_behavior: EscapeKeyBehavior,

    /// Whether the grid uses a single tab stop with internal arrow navigation.
    pub composite: bool,

    /// Callback invoked by adapters when the loading sentinel is reached.
    pub on_load_more: Option<Callback<dyn Fn() + Send + Sync>>,

    /// Callback invoked when an enabled item receives a primary action.
    pub on_action: Option<Callback<dyn Fn(Key) + Send + Sync>>,

    /// Whether a load-more request is currently in flight.
    pub loading: bool,

    /// Enable the agnostic drag-and-drop affordance attrs.
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
            label: None,
            disallow_empty_selection: false,
            escape_key_behavior: EscapeKeyBehavior::ClearSelection,
            composite: true,
            on_load_more: None,
            on_action: None,
            loading: false,
            dnd_enabled: false,
        }
    }
}

impl Props {
    /// Returns fresh `GridList` props with documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the displayed item collection.
    #[must_use]
    pub fn items(mut self, items: StaticCollection<ItemDef>) -> Self {
        self.items = items;
        self
    }

    /// Sets the logical column count.
    #[must_use]
    pub const fn columns(mut self, columns: NonZeroUsize) -> Self {
        self.columns = Some(columns);
        self
    }

    /// Sets the controlled selected keys.
    #[must_use]
    pub fn selected_keys(mut self, selected_keys: BTreeSet<Key>) -> Self {
        self.selected_keys = Some(selected_keys);
        self
    }

    /// Sets the uncontrolled initial selected keys.
    #[must_use]
    pub fn default_selected_keys(mut self, selected_keys: BTreeSet<Key>) -> Self {
        self.default_selected_keys = selected_keys;
        self
    }

    /// Sets the selection mode.
    #[must_use]
    pub const fn selection_mode(mut self, selection_mode: selection::Mode) -> Self {
        self.selection_mode = selection_mode;
        self
    }

    /// Sets the selection behavior.
    #[must_use]
    pub const fn selection_behavior(mut self, selection_behavior: selection::Behavior) -> Self {
        self.selection_behavior = selection_behavior;
        self
    }

    /// Sets group disabled state.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets individually disabled item keys.
    #[must_use]
    pub fn disabled_keys(mut self, disabled_keys: BTreeSet<Key>) -> Self {
        self.disabled_keys = disabled_keys;
        self
    }

    /// Sets the accessible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets whether the last selected item can be deselected.
    #[must_use]
    pub const fn disallow_empty_selection(mut self, disallow_empty_selection: bool) -> Self {
        self.disallow_empty_selection = disallow_empty_selection;
        self
    }

    /// Sets Escape key behavior.
    #[must_use]
    pub const fn escape_key_behavior(mut self, behavior: EscapeKeyBehavior) -> Self {
        self.escape_key_behavior = behavior;
        self
    }

    /// Sets whether `GridList` uses composite grid focus semantics.
    #[must_use]
    pub const fn composite(mut self, composite: bool) -> Self {
        self.composite = composite;
        self
    }

    /// Sets the load-more callback.
    #[must_use]
    pub fn on_load_more(mut self, callback: Callback<dyn Fn() + Send + Sync>) -> Self {
        self.on_load_more = Some(callback);
        self
    }

    /// Sets the primary item action callback.
    #[must_use]
    pub fn on_action(mut self, callback: Callback<dyn Fn(Key) + Send + Sync>) -> Self {
        self.on_action = Some(callback);
        self
    }

    /// Sets the loading state.
    #[must_use]
    pub const fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets whether drag-and-drop attrs are enabled.
    #[must_use]
    pub const fn dnd_enabled(mut self, dnd_enabled: bool) -> Self {
        self.dnd_enabled = dnd_enabled;
        self
    }
}

/// States for the `GridList` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No item is focused.
    #[default]
    Idle,

    /// An item within the grid has keyboard or pointer focus.
    Focused,
}

/// Events accepted by the `GridList` machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus entered the grid or moved to a specific item.
    Focus {
        /// Item key to focus, or first enabled item when `None`.
        key: Option<Key>,

        /// Whether focus was keyboard initiated.
        is_keyboard: bool,
    },

    /// Focus left the grid entirely.
    Blur,

    /// Select a single item, replacing current selection.
    Select(Key),

    /// Toggle the selected state of a single item.
    ToggleSelect(Key),

    /// Select all items between `from` and `to`, inclusive.
    SelectRange {
        /// First range endpoint.
        from: Key,

        /// Second range endpoint.
        to: Key,
    },

    /// Move focus one row up.
    FocusUp,

    /// Move focus one row down.
    FocusDown,

    /// Move focus one cell left.
    FocusLeft,

    /// Move focus one cell right.
    FocusRight,

    /// Move focus to the first item.
    FocusFirst,

    /// Move focus to the last item.
    FocusLast,

    /// Select all non-disabled items.
    SelectAll,

    /// Deselect all items.
    ClearSelection,

    /// Primary action triggered on an item.
    ItemAction(Key),

    /// Typeahead search intent from a printable keyboard event.
    TypeaheadSearch {
        /// Character entered by the user.
        ch: char,

        /// Monotonic timestamp supplied by the adapter.
        now: Duration,
    },

    /// Synchronize non-controlled props mirrored into context.
    SyncProps,
}

/// Context for the `GridList` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Flat list of grid items, laid out left-to-right, top-to-bottom.
    pub items: StaticCollection<ItemDef>,

    /// Number of columns in the grid layout.
    pub columns: NonZeroUsize,

    /// Key of the currently focused item, if any.
    pub focused_key: Option<Key>,

    /// True when focus was keyboard-initiated.
    pub focus_visible: bool,

    /// Anchor key used to extend repeated Shift+Arrow range selection.
    pub range_anchor_key: Option<Key>,

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

    /// Whether a load-more request is currently in flight.
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

/// Messages for the `GridList` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Describes the focused cell position, e.g. "Row 2, Column 3".
    pub cell_position: MessageFn<CellPositionFn>,

    /// Describes total grid dimensions, e.g. "4 rows, 3 columns".
    pub grid_dimensions: MessageFn<GridDimensionsFn>,

    /// Accessible label template for an item's drag handle.
    pub drag_handle_label: MessageFn<DragHandleLabelFn>,

    /// Role description for draggable items.
    pub draggable: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            cell_position: MessageFn::new(Arc::new(|row, col, _locale: &Locale| {
                format!("Row {row}, Column {col}")
            }) as Arc<CellPositionFn>),
            grid_dimensions: MessageFn::new(Arc::new(|rows, cols, _locale: &Locale| {
                format!("{rows} rows, {cols} columns")
            }) as Arc<GridDimensionsFn>),
            drag_handle_label: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("Drag {label}")
            }) as Arc<DragHandleLabelFn>),
            draggable: MessageFn::static_str("draggable"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed side-effect intents emitted by the `GridList` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Notify adapters that the user requested a selection change.
    SelectionChange,

    /// Notify adapters that the user requested a primary item action.
    Action,
}

/// Structural parts exposed by the `GridList` connect API.
#[derive(ComponentPart)]
#[scope = "grid-list"]
pub enum Part {
    /// The root grid-list element.
    Root,

    /// Logical row wrapper for one item.
    Row {
        /// Item key.
        key: Key,
    },

    /// Cell element for one item.
    Cell {
        /// Item key.
        key: Key,
    },

    /// Optional infinite-scroll sentinel element.
    LoadingSentinel,

    /// Optional drag handle affordance for one item.
    DragHandle {
        /// Item key.
        key: Key,
    },

    /// The visual indicator showing where a dragged item will be dropped.
    DropIndicator,
}

/// Machine for the `GridList` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let columns = resolved_columns(props.columns, item_count(&props.items));

        let disabled_keys = effective_disabled_keys(&props.items, &props.disabled_keys);

        (
            State::Idle,
            Context {
                items: props.items.clone(),
                columns,
                focused_key: None,
                focus_visible: false,
                range_anchor_key: None,
                selection_mode: props.selection_mode,
                selection_behavior: props.selection_behavior,
                selected_keys: match &props.selected_keys {
                    Some(keys) => Bindable::controlled(filter_selection(
                        keys,
                        &props.items,
                        &disabled_keys,
                        props.selection_mode,
                    )),
                    None => Bindable::uncontrolled(filter_selection(
                        &props.default_selected_keys,
                        &props.items,
                        &disabled_keys,
                        props.selection_mode,
                    )),
                },
                requested_selected_keys: None,
                requested_action_key: None,
                disabled: props.disabled,
                disabled_keys,
                disallow_empty_selection: props.disallow_empty_selection,
                escape_key_behavior: props.escape_key_behavior,
                composite: props.composite,
                loading: props.loading,
                dnd_enabled: props.dnd_enabled,
                ids: ComponentIds::from_id(props.id()),
                typeahead: typeahead::State::default(),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if let Event::SyncProps = event {
            return Some(sync_props_plan(context, props));
        }

        if context.disabled {
            return match event {
                Event::Blur => Some(blur_plan()),
                _ => None,
            };
        }

        match event {
            Event::Focus { key, is_keyboard } => {
                let key = if let Some(key) = key {
                    Some(enabled_item(context, key)?.key.clone())
                } else {
                    first_enabled_key(&context.items, &context.disabled_keys)
                };

                let is_keyboard = *is_keyboard;

                let target = if key.is_some() {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.focused_key = key;
                    ctx.focus_visible = is_keyboard && ctx.focused_key.is_some();
                    ctx.range_anchor_key = None;
                }))
            }

            Event::Blur => Some(blur_plan()),

            Event::Select(key) => {
                if context.selection_mode == selection::Mode::None
                    || enabled_item(context, key).is_none()
                {
                    return None;
                }

                selection_plan(BTreeSet::from([key.clone()]))
            }

            Event::ToggleSelect(key) => {
                if context.selection_mode == selection::Mode::None
                    || enabled_item(context, key).is_none()
                {
                    return None;
                }

                let selected_source = interaction_selected_keys(context);

                if selected_source.contains(key) {
                    if context.disallow_empty_selection && selected_source.len() <= 1 {
                        return None;
                    }

                    let mut selected = selected_source.clone();

                    selected.remove(key);

                    selection_plan(selected)
                } else {
                    let mut selected = selected_source.clone();

                    if context.selection_mode == selection::Mode::Single {
                        selected.clear();
                    }

                    selected.insert(key.clone());

                    selection_plan(selected)
                }
            }

            Event::SelectRange { from, to } => {
                if context.selection_mode != selection::Mode::Multiple {
                    return None;
                }

                enabled_item(context, from)?;
                enabled_item(context, to)?;

                let selected = range_keys(context, from, to)?;

                if selected.is_empty() && context.disallow_empty_selection {
                    return None;
                }

                selection_plan_with_anchor(selected, Some(from.clone()))
            }

            Event::SelectAll => {
                if context.selection_mode != selection::Mode::Multiple {
                    return None;
                }

                let selected = enabled_keys(&context.items, &context.disabled_keys);

                if selected.is_empty() {
                    return None;
                }

                selection_plan(selected)
            }

            Event::ClearSelection => {
                if context.selection_mode == selection::Mode::None
                    || interaction_selected_keys(context).is_empty()
                    || context.disallow_empty_selection
                {
                    return None;
                }

                selection_plan(BTreeSet::new())
            }

            Event::ItemAction(key) => {
                enabled_item(context, key)?;

                let requested_key = key.clone();
                let callback_key = key.clone();

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.requested_action_key = Some(requested_key);
                    })
                    .with_named_effect(
                        Effect::Action,
                        move |_ctx: &Context, props: &Props, _send| {
                            if let Some(callback) = &props.on_action {
                                callback(callback_key);
                            }

                            ars_core::no_cleanup()
                        },
                    ),
                )
            }

            Event::FocusUp => move_focus_plan(context, Direction2d::Up),

            Event::FocusDown => move_focus_plan(context, Direction2d::Down),

            Event::FocusLeft => move_focus_plan(context, Direction2d::Left),

            Event::FocusRight => move_focus_plan(context, Direction2d::Right),

            Event::FocusFirst => {
                first_enabled_key(&context.items, &context.disabled_keys).map(focus_key_plan)
            }

            Event::FocusLast => {
                last_enabled_key(&context.items, &context.disabled_keys).map(focus_key_plan)
            }

            Event::TypeaheadSearch { ch, now } => {
                let (typeahead, found) = context.typeahead.process_char_with_locale(
                    *ch,
                    *now,
                    context.focused_key.as_ref(),
                    &context.items,
                    &context.locale,
                    &context.disabled_keys,
                    DisabledBehavior::Skip,
                );

                let target = found.filter(|key| enabled_item(context, key).is_some());

                let next_state = if target.is_some() {
                    State::Focused
                } else {
                    *state
                };

                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        ctx.typeahead = typeahead;

                        if let Some(key) = target {
                            ctx.focused_key = Some(key);
                            ctx.focus_visible = true;
                            ctx.range_anchor_key = None;
                        }
                    }),
                )
            }

            Event::SyncProps => unreachable!("SyncProps handled before interactivity guards"),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            context,
            props,
            send,
        }
    }

    fn on_props_changed(_old: &Self::Props, _new: &Self::Props) -> Vec<Self::Event> {
        vec![Event::SyncProps]
    }
}

/// API for the `GridList` component.
pub struct Api<'a> {
    /// Current state of the grid list.
    state: &'a State,

    /// Current context of the grid list.
    context: &'a Context,

    /// Current props of the grid list.
    props: &'a Props,

    /// Send event to the grid list.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("context", self.context)
            .field("props", self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl Api<'_> {
    /// Returns the current grid dimensions description.
    #[must_use]
    pub fn grid_dimensions(&self) -> String {
        let cols = self.context.columns.get();
        let rows = item_count(&self.context.items).div_ceil(cols);

        (self.context.messages.grid_dimensions)(rows, cols, &self.context.locale)
    }

    /// Returns the cell position description for `key`.
    #[must_use]
    pub fn cell_position(&self, key: &Key) -> Option<String> {
        let index = key_item_index(&self.context.items, key)?;
        let (col, row) = index_to_pos(index, self.context.columns);

        Some((self.context.messages.cell_position)(
            row + 1,
            col + 1,
            &self.context.locale,
        ))
    }

    /// Returns the stable cell ID for `key`.
    #[must_use]
    pub fn cell_id(&self, key: &Key) -> Option<String> {
        item_key_present(&self.context.items, key).then(|| self.context.ids.item("cell", key))
    }

    /// Returns the latest user-requested selected keys.
    #[must_use]
    pub const fn requested_selected_keys(&self) -> Option<&BTreeSet<Key>> {
        self.context.requested_selected_keys.as_ref()
    }

    /// Returns the latest user-requested action key.
    #[must_use]
    pub const fn requested_action_key(&self) -> Option<&Key> {
        self.context.requested_action_key.as_ref()
    }

    /// Returns the current item collection.
    #[must_use]
    pub const fn items(&self) -> &StaticCollection<ItemDef> {
        &self.context.items
    }

    /// Returns root attributes for the grid list.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, self.context.ids.id())
            .set(
                HtmlAttr::Role,
                if self.context.composite {
                    "grid"
                } else {
                    "list"
                },
            )
            .set(
                HtmlAttr::Data("ars-state"),
                match self.state {
                    State::Idle => "idle",
                    State::Focused => "focused",
                },
            );

        if let Some(label) = &self.props.label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }

        if self.context.selection_mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        if self.context.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.context.composite {
            attrs.set(
                HtmlAttr::TabIndex,
                if self.context.disabled { "-1" } else { "0" },
            );

            if let Some(key) = &self.context.focused_key
                && let Some(id) = self.cell_id(key)
            {
                attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), id);
            }
        }

        attrs
    }

    /// Returns row attributes for `key`.
    #[must_use]
    pub fn row_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Row {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Data("ars-key"), key.to_string()).set(
            HtmlAttr::Role,
            if self.context.composite {
                "row"
            } else {
                "listitem"
            },
        );

        if self.context.composite && self.context.selection_mode != selection::Mode::None {
            let selected = rendered_selected_keys(self.context).contains(key);

            attrs.set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if selected { "true" } else { "false" },
            );

            if selected {
                attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
            }
        }

        if !self.context.composite && rendered_selected_keys(self.context).contains(key) {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        attrs
    }

    /// Returns cell attributes for `key`.
    #[must_use]
    pub fn cell_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Cell {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Data("ars-key"), key.to_string());

        if let Some(id) = self.cell_id(key) {
            attrs.set(HtmlAttr::Id, id);
        }

        if self.context.composite {
            attrs.set(HtmlAttr::Role, "gridcell");
        }

        let focused = self.context.focused_key.as_ref() == Some(key);
        let selected = rendered_selected_keys(self.context).contains(key);
        let disabled =
            is_disabled_key(self.context, key) || !item_key_present(&self.context.items, key);

        let tabindex = if !disabled && !self.context.composite {
            "0"
        } else {
            "-1"
        };

        attrs.set(HtmlAttr::TabIndex, tabindex);

        if focused && self.context.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.context.dnd_enabled && !disabled {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.context.messages.draggable)(&self.context.locale),
            );
        }

        if self.context.composite && self.context.selection_mode != selection::Mode::None {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if selected { "true" } else { "false" },
            );
        }

        if selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        if !disabled
            && let Some(href) = self
                .context
                .items
                .get(key)
                .and_then(|node| node.value.as_ref())
                .and_then(|item| item.href.as_ref())
        {
            attrs.set(HtmlAttr::Href, sanitize_url(href.as_str()));
        }

        attrs
    }

    /// Returns attributes for the optional loading sentinel.
    #[must_use]
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap> {
        self.props.on_load_more.as_ref()?;

        let mut attrs = part_attrs(&Part::LoadingSentinel);

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::TabIndex, "-1");

        if self.context.loading {
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        }

        Some(attrs)
    }

    /// Returns drag handle attributes for `key`.
    #[must_use]
    pub fn drag_handle_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::DragHandle {
            key: Key::default(),
        });

        let item = self
            .context
            .items
            .get(key)
            .and_then(|node| node.value.as_ref());

        let label = item.map_or("", |item| item.label.as_str());

        let inert =
            !self.context.dnd_enabled || item.is_none_or(|_| is_disabled_key(self.context, key));

        attrs.set(HtmlAttr::Role, "button").set(
            HtmlAttr::Aria(AriaAttr::Label),
            (self.context.messages.drag_handle_label)(label, &self.context.locale),
        );

        if inert {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set(HtmlAttr::TabIndex, "-1");
        } else {
            attrs.set(HtmlAttr::TabIndex, "0");
        }

        attrs
    }

    /// Returns drop indicator attributes for `target`.
    #[must_use]
    pub fn drop_indicator_attrs(&self, target: &CollectionDropTarget) -> AttrMap {
        let mut attrs = part_attrs(&Part::DropIndicator);

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(
                HtmlAttr::Data("ars-drop-position"),
                target.position.to_string(),
            )
            .set(
                HtmlAttr::Data("ars-drop-target"),
                self.cell_id(&target.key)
                    .unwrap_or_else(|| target.key.to_string()),
            );

        attrs
    }

    /// Dispatches a keydown event using the platform clock when available.
    pub fn on_cell_keydown(&self, key: &Key, data: &KeyboardEventData) {
        self.on_cell_keydown_impl(key, data, None);
    }

    /// Dispatches a keydown event for a cell with an adapter-provided timestamp.
    pub fn on_cell_keydown_at(&self, key: &Key, data: &KeyboardEventData, now: Duration) {
        self.on_cell_keydown_impl(key, data, Some(now));
    }

    fn on_cell_keydown_impl(&self, key: &Key, data: &KeyboardEventData, now: Option<Duration>) {
        if !self.context.composite {
            match data.key {
                KeyboardKey::Enter => (self.send)(Event::ItemAction(key.clone())),
                KeyboardKey::Space => (self.send)(self.select_key_event(key)),
                KeyboardKey::Escape
                    if self.context.escape_key_behavior == EscapeKeyBehavior::ClearSelection =>
                {
                    (self.send)(Event::ClearSelection);
                }
                _ if data
                    .character
                    .is_some_and(|ch| ch.eq_ignore_ascii_case(&'a'))
                    && (data.ctrl_key || data.meta_key) =>
                {
                    (self.send)(Event::SelectAll);
                }
                _ => {}
            }

            return;
        }

        match data.key {
            KeyboardKey::ArrowUp => self.dispatch_arrow(key, Direction2d::Up, Event::FocusUp, data),

            KeyboardKey::ArrowDown => {
                self.dispatch_arrow(key, Direction2d::Down, Event::FocusDown, data);
            }

            KeyboardKey::ArrowLeft if self.is_rtl() => {
                self.dispatch_arrow(key, Direction2d::Right, Event::FocusRight, data);
            }

            KeyboardKey::ArrowLeft => {
                self.dispatch_arrow(key, Direction2d::Left, Event::FocusLeft, data);
            }

            KeyboardKey::ArrowRight if self.is_rtl() => {
                self.dispatch_arrow(key, Direction2d::Left, Event::FocusLeft, data);
            }

            KeyboardKey::ArrowRight => {
                self.dispatch_arrow(key, Direction2d::Right, Event::FocusRight, data);
            }

            KeyboardKey::Home => (self.send)(Event::FocusFirst),

            KeyboardKey::End => (self.send)(Event::FocusLast),

            KeyboardKey::Enter => (self.send)(Event::ItemAction(key.clone())),

            KeyboardKey::Space => (self.send)(self.select_key_event(key)),

            KeyboardKey::Escape
                if self.context.escape_key_behavior == EscapeKeyBehavior::ClearSelection =>
            {
                (self.send)(Event::ClearSelection);
            }

            _ if data
                .character
                .is_some_and(|ch| ch.eq_ignore_ascii_case(&'a'))
                && (data.ctrl_key || data.meta_key) =>
            {
                (self.send)(Event::SelectAll);
            }

            _ if data.character.is_some()
                && !data.ctrl_key
                && !data.meta_key
                && !data.alt_key
                && !data.is_composing =>
            {
                (self.send)(Event::TypeaheadSearch {
                    ch: data.character.expect("checked"),
                    now: typeahead_timestamp(now, self.context),
                });
            }

            _ => {}
        }
    }

    fn is_rtl(&self) -> bool {
        Direction::from(self.context.locale.direction()) == Direction::Rtl
    }

    fn select_key_event(&self, key: &Key) -> Event {
        match self.context.selection_behavior {
            selection::Behavior::Toggle => Event::ToggleSelect(key.clone()),
            selection::Behavior::Replace => Event::Select(key.clone()),
        }
    }

    fn dispatch_arrow(
        &self,
        key: &Key,
        direction: Direction2d,
        focus_event: Event,
        data: &KeyboardEventData,
    ) {
        if data.shift_key
            && self.context.selection_mode == selection::Mode::Multiple
            && let Some(target) = navigation_target_from_key(self.context, key, direction)
        {
            let anchor = self
                .context
                .range_anchor_key
                .as_ref()
                .filter(|anchor| enabled_item(self.context, anchor).is_some())
                .cloned()
                .unwrap_or_else(|| key.clone());

            (self.send)(focus_event);
            (self.send)(Event::SelectRange {
                from: anchor,
                to: target,
            });

            return;
        }

        (self.send)(focus_event);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Row { key } => self.row_attrs(&key),
            Part::Cell { key } => self.cell_attrs(&key),
            Part::LoadingSentinel => self.loading_sentinel_attrs().unwrap_or_default(),
            Part::DragHandle { key } => self.drag_handle_attrs(&key),
            Part::DropIndicator => part_attrs(&Part::DropIndicator),
        }
    }
}

#[derive(Clone, Copy)]
enum Direction2d {
    Up,
    Down,
    Left,
    Right,
}

fn resolved_columns(columns: Option<NonZeroUsize>, item_count: usize) -> NonZeroUsize {
    columns.unwrap_or_else(|| NonZeroUsize::new(item_count.max(1)).expect("max(1) is non-zero"))
}

const fn index_to_pos(index: usize, columns: NonZeroUsize) -> (usize, usize) {
    let columns = columns.get();
    (index % columns, index / columns)
}

const fn pos_to_index(col: usize, row: usize, columns: NonZeroUsize) -> usize {
    row.saturating_mul(columns.get()).saturating_add(col)
}

fn item_count(items: &StaticCollection<ItemDef>) -> usize {
    items.nodes().filter(|node| node.value.is_some()).count()
}

fn key_item_index(items: &StaticCollection<ItemDef>, key: &Key) -> Option<usize> {
    items
        .nodes()
        .filter(|node| node.value.is_some())
        .position(|node| &node.key == key)
}

fn item_key_at_index(
    items: &StaticCollection<ItemDef>,
    disabled_keys: &BTreeSet<Key>,
    index: usize,
) -> Option<Option<Key>> {
    items
        .nodes()
        .filter(|node| node.value.is_some())
        .nth(index)
        .map(|node| {
            (!disabled_keys.contains(&node.key))
                .then(|| node.value.as_ref().map(|item| item.key.clone()))
                .flatten()
        })
}

fn effective_disabled_keys(
    items: &StaticCollection<ItemDef>,
    disabled_keys: &BTreeSet<Key>,
) -> BTreeSet<Key> {
    items
        .nodes()
        .filter_map(|node| {
            let key = node.key.clone();

            if disabled_keys.contains(&key) || node.value.as_ref().is_some_and(|item| item.disabled)
            {
                Some(key)
            } else {
                None
            }
        })
        .collect()
}

fn is_disabled_key(context: &Context, key: &Key) -> bool {
    context.disabled || context.disabled_keys.contains(key)
}

fn enabled_item<'a>(context: &'a Context, key: &Key) -> Option<&'a ItemDef> {
    if is_disabled_key(context, key) {
        return None;
    }

    context.items.get(key)?.value.as_ref()
}

fn first_enabled_key(
    items: &StaticCollection<ItemDef>,
    disabled_keys: &BTreeSet<Key>,
) -> Option<Key> {
    items
        .nodes()
        .filter(|node| !disabled_keys.contains(&node.key))
        .filter_map(|node| node.value.as_ref().map(|item| item.key.clone()))
        .next()
}

fn last_enabled_key(
    items: &StaticCollection<ItemDef>,
    disabled_keys: &BTreeSet<Key>,
) -> Option<Key> {
    items
        .nodes()
        .filter(|node| !disabled_keys.contains(&node.key))
        .filter_map(|node| node.value.as_ref().map(|item| item.key.clone()))
        .last()
}

fn enabled_keys(items: &StaticCollection<ItemDef>, disabled_keys: &BTreeSet<Key>) -> BTreeSet<Key> {
    items
        .nodes()
        .filter(|node| !disabled_keys.contains(&node.key))
        .filter_map(|node| node.value.as_ref().map(|item| item.key.clone()))
        .collect()
}

fn item_key_present(items: &StaticCollection<ItemDef>, key: &Key) -> bool {
    items
        .get(key)
        .is_some_and(|node| node.is_focusable() && node.value.is_some())
}

fn filter_selection(
    keys: &BTreeSet<Key>,
    items: &StaticCollection<ItemDef>,
    disabled_keys: &BTreeSet<Key>,
    mode: selection::Mode,
) -> BTreeSet<Key> {
    let mut selected = keys
        .iter()
        .filter(|key| item_key_present(items, key) && !disabled_keys.contains(key))
        .cloned()
        .collect();

    normalize_selection(&mut selected, mode);

    selected
}

fn normalize_selection(selected: &mut BTreeSet<Key>, mode: selection::Mode) {
    match mode {
        selection::Mode::None => selected.clear(),

        selection::Mode::Multiple => {}

        selection::Mode::Single => {
            let first = selected.pop_first();

            selected.clear();

            if let Some(key) = first {
                selected.insert(key);
            }
        }
    }
}

fn interaction_selected_keys(context: &Context) -> &BTreeSet<Key> {
    context
        .requested_selected_keys
        .as_ref()
        .unwrap_or_else(|| context.selected_keys.get())
}

fn rendered_selected_keys(context: &Context) -> &BTreeSet<Key> {
    context.selected_keys.get()
}

fn selection_plan(selected: BTreeSet<Key>) -> Option<TransitionPlan<Machine>> {
    selection_plan_with_anchor(selected, None)
}

fn selection_plan_with_anchor(
    mut selected: BTreeSet<Key>,
    range_anchor_key: Option<Key>,
) -> Option<TransitionPlan<Machine>> {
    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            normalize_selection(&mut selected, ctx.selection_mode);

            ctx.requested_selected_keys = Some(selected.clone());
            ctx.selected_keys.set(selected);
            ctx.range_anchor_key = range_anchor_key.clone();
        })
        .with_named_effect(Effect::SelectionChange, |_ctx, _props, _send| {
            ars_core::no_cleanup()
        }),
    )
}

const fn fallback_typeahead_timestamp(context: &Context) -> Duration {
    context
        .typeahead
        .last_key_time
        .saturating_add(Duration::from_millis(1))
}

fn typeahead_timestamp(now: Option<Duration>, context: &Context) -> Duration {
    now.unwrap_or_else(|| current_time().unwrap_or_else(|| fallback_typeahead_timestamp(context)))
}

#[cfg(feature = "std")]
fn current_time() -> Option<Duration> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
}

#[cfg(not(feature = "std"))]
const fn current_time() -> Option<Duration> {
    None
}

fn range_keys(context: &Context, from: &Key, to: &Key) -> Option<BTreeSet<Key>> {
    let from = key_item_index(&context.items, from)?;
    let to = key_item_index(&context.items, to)?;

    let (start, end) = if from <= to { (from, to) } else { (to, from) };

    Some(
        context
            .items
            .nodes()
            .filter(|node| node.value.is_some())
            .enumerate()
            .skip_while(|(index, _node)| *index < start)
            .take_while(|(index, _node)| *index <= end)
            .map(|(_index, node)| node)
            .filter(|node| !context.disabled_keys.contains(&node.key))
            .filter_map(|node| node.value.as_ref().map(|item| item.key.clone()))
            .collect(),
    )
}

fn focus_key_plan(key: Key) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
        ctx.focused_key = Some(key);
        ctx.focus_visible = true;
        ctx.range_anchor_key = None;
    })
}

fn blur_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
        ctx.focused_key = None;
        ctx.focus_visible = false;
        ctx.range_anchor_key = None;
    })
}

fn move_focus_plan(context: &Context, direction: Direction2d) -> Option<TransitionPlan<Machine>> {
    let current = context.focused_key.as_ref()?;
    let target = navigation_target_from_key(context, current, direction)?;

    Some(focus_key_plan(target))
}

fn navigation_target_from_key(context: &Context, key: &Key, direction: Direction2d) -> Option<Key> {
    let current_index = key_item_index(&context.items, key)?;

    match direction {
        Direction2d::Up => scan_vertical(context, current_index, false),
        Direction2d::Down => scan_vertical(context, current_index, true),
        Direction2d::Left => scan_horizontal(context, current_index, false),
        Direction2d::Right => scan_horizontal(context, current_index, true),
    }
}

fn scan_horizontal(context: &Context, current_index: usize, forward: bool) -> Option<Key> {
    let (col, row) = index_to_pos(current_index, context.columns);
    let columns = context.columns.get();

    let mut next_col = if forward {
        col.checked_add(1)?
    } else {
        col.checked_sub(1)?
    };

    while next_col < columns {
        let index = pos_to_index(next_col, row, context.columns);

        if let Some(key) = item_key_at_index(&context.items, &context.disabled_keys, index)? {
            return Some(key);
        }

        next_col = if forward {
            next_col.checked_add(1)?
        } else {
            next_col.checked_sub(1)?
        };
    }

    None
}

fn scan_vertical(context: &Context, current_index: usize, forward: bool) -> Option<Key> {
    let (col, row) = index_to_pos(current_index, context.columns);

    let mut next_row = if forward {
        row.checked_add(1)?
    } else {
        row.checked_sub(1)?
    };

    loop {
        let index = pos_to_index(col, next_row, context.columns);
        if let Some(key) = item_key_at_index(&context.items, &context.disabled_keys, index)? {
            return Some(key);
        }

        next_row = if forward {
            next_row.checked_add(1)?
        } else {
            next_row.checked_sub(1)?
        };
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_value)
        .set(part_attr, part_value);

    attrs
}

fn sync_props_plan(context: &Context, props: &Props) -> TransitionPlan<Machine> {
    let disabled_keys = effective_disabled_keys(&props.items, &props.disabled_keys);
    let columns = resolved_columns(props.columns, item_count(&props.items));

    let selection_mode = props.selection_mode;

    let controlled_keys = props
        .selected_keys
        .as_ref()
        .map(|keys| filter_selection(keys, &props.items, &disabled_keys, selection_mode));

    let focused_key = if props.disabled {
        None
    } else {
        context
            .focused_key
            .as_ref()
            .filter(|key| item_key_present(&props.items, key) && !disabled_keys.contains(key))
            .cloned()
    };

    let next_state = if focused_key.is_some() {
        State::Focused
    } else {
        State::Idle
    };

    let items = props.items.clone();
    let disabled = props.disabled;
    let selection_behavior = props.selection_behavior;
    let disallow_empty_selection = props.disallow_empty_selection;
    let escape_key_behavior = props.escape_key_behavior;
    let composite = props.composite;
    let loading = props.loading;
    let dnd_enabled = props.dnd_enabled;
    let ids = ComponentIds::from_id(props.id());

    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
        ctx.items = items;
        ctx.columns = columns;
        ctx.focused_key = focused_key;
        ctx.focus_visible = ctx.focused_key.is_some() && ctx.focus_visible;
        ctx.range_anchor_key = None;
        ctx.selection_mode = selection_mode;
        ctx.selection_behavior = selection_behavior;
        ctx.disabled = disabled;
        ctx.disabled_keys = disabled_keys;
        ctx.disallow_empty_selection = disallow_empty_selection;
        ctx.escape_key_behavior = escape_key_behavior;
        ctx.composite = composite;
        ctx.loading = loading;
        ctx.dnd_enabled = dnd_enabled;
        ctx.ids = ids;
        ctx.requested_selected_keys = None;
        ctx.requested_action_key = None;
        ctx.selected_keys.sync_controlled(controlled_keys.clone());

        if controlled_keys.is_none() {
            let selected = filter_selection(
                ctx.selected_keys.get(),
                &ctx.items,
                &ctx.disabled_keys,
                ctx.selection_mode,
            );

            ctx.selected_keys.set(selected);
        }
    })
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, string::String, sync::Arc, vec::Vec};
    use core::{cell::RefCell, num::NonZero};
    use std::sync::Mutex;

    use ars_collections::{CollectionBuilder, Key, StaticCollection, selection};
    use ars_core::{
        AriaAttr, AttrMap, Callback, ConnectApi, Env, HtmlAttr, SafeUrl, Service, StrongSend,
    };
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn item(value: &str, label: &str, disabled: bool) -> ItemDef {
        ItemDef {
            key: key(value),
            label: label.into(),
            disabled,
            href: None,
        }
    }

    fn link_item(value: &str, label: &str, href: SafeUrl) -> ItemDef {
        ItemDef {
            key: key(value),
            label: label.into(),
            disabled: false,
            href: Some(href),
        }
    }

    fn items() -> StaticCollection<ItemDef> {
        StaticCollection::new([
            (key("alpha"), "Alpha".into(), item("alpha", "Alpha", false)),
            (key("beta"), "Beta".into(), item("beta", "Beta", true)),
            (key("gamma"), "Gamma".into(), item("gamma", "Gamma", false)),
            (key("delta"), "Delta".into(), item("delta", "Delta", false)),
            (key("echo"), "Echo".into(), item("echo", "Echo", false)),
        ])
    }

    fn sectioned_items() -> StaticCollection<ItemDef> {
        CollectionBuilder::new()
            .section(key("section"), "Section")
            .item(key("alpha"), "Alpha", item("alpha", "Alpha", false))
            .end_section()
            .separator()
            .item(key("gamma"), "Gamma", item("gamma", "Gamma", false))
            .build()
    }

    fn selected(keys: &[&str]) -> BTreeSet<Key> {
        keys.iter().map(|value| key(value)).collect()
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn keyboard(key: KeyboardKey, character: Option<char>) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn root_row_and_cell_attrs_cover_grid_roles_active_descendant_and_safe_link() {
        let props = Props::new()
            .id("products")
            .label("Products")
            .columns(NonZero::new(2).expect("non-zero columns"))
            .items(StaticCollection::new([
                (
                    key("alpha"),
                    "Alpha".into(),
                    link_item("alpha", "Alpha", SafeUrl::from_static("/products/alpha")),
                ),
                (key("beta"), "Beta".into(), item("beta", "Beta", false)),
            ]))
            .selection_mode(selection::Mode::Multiple)
            .default_selected_keys(selected(&["alpha"]));

        let mut service = service(props);

        drop(service.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));

        let api = service.connect(&|_| {});

        assert_snapshot!("grid_list_root_grid", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!(
            "grid_list_row_selected",
            snapshot_attrs(&api.row_attrs(&key("alpha")))
        );
        assert_snapshot!(
            "grid_list_cell_link_selected_focused",
            snapshot_attrs(&api.cell_attrs(&key("alpha")))
        );

        assert_eq!(
            api.root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            api.cell_id(&key("alpha")).as_deref()
        );
        assert_eq!(
            api.cell_attrs(&key("alpha")).get(&HtmlAttr::Href),
            Some("/products/alpha")
        );
    }

    #[test]
    fn non_composite_attrs_use_list_roles_and_independent_tab_stops() {
        let mut props = Props::new()
            .id("cards")
            .items(items())
            .selection_mode(selection::Mode::Single);

        props.composite = false;

        let mut grid = service(props);

        drop(grid.send(Event::Focus {
            key: Some(key("gamma")),
            is_keyboard: true,
        }));

        let api = grid.connect(&|_| {});

        assert_snapshot!(
            "grid_list_root_non_composite",
            snapshot_attrs(&api.root_attrs())
        );
        assert_snapshot!(
            "grid_list_row_non_composite",
            snapshot_attrs(&api.row_attrs(&key("gamma")))
        );
        assert_snapshot!(
            "grid_list_cell_non_composite",
            snapshot_attrs(&api.cell_attrs(&key("gamma")))
        );
        assert!(
            api.root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant))
                .is_none()
        );
        assert!(
            api.cell_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected))
                .is_none()
        );
        assert!(
            api.row_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected))
                .is_none()
        );
        assert!(
            api.row_attrs(&key("gamma"))
                .get(&HtmlAttr::Data("ars-selected"))
                .is_none()
        );

        let selected = service(
            Props::new()
                .id("cards-selected")
                .items(items())
                .selection_mode(selection::Mode::Single)
                .default_selected_keys(selected(&["gamma"]))
                .composite(false),
        );
        let selected_api = selected.connect(&|_| {});

        assert!(
            selected_api
                .row_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected))
                .is_none()
        );
        assert_eq!(
            selected_api
                .row_attrs(&key("gamma"))
                .get(&HtmlAttr::Data("ars-selected")),
            Some("true")
        );
    }

    #[test]
    fn arrow_navigation_is_two_dimensional_and_skips_disabled_items() {
        let mut service = service(
            Props::new()
                .id("grid")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns")),
        );

        drop(service.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(service.send(Event::FocusRight));

        assert_eq!(service.context().focused_key, Some(key("alpha")));

        drop(service.send(Event::FocusDown));

        assert_eq!(service.context().focused_key, Some(key("gamma")));

        drop(service.send(Event::FocusRight));

        assert_eq!(service.context().focused_key, Some(key("delta")));

        drop(service.send(Event::FocusLeft));

        assert_eq!(service.context().focused_key, Some(key("gamma")));

        drop(service.send(Event::FocusUp));

        assert_eq!(service.context().focused_key, Some(key("alpha")));

        drop(service.send(Event::FocusDown));
        drop(service.send(Event::FocusRight));

        assert_eq!(service.context().focused_key, Some(key("delta")));
    }

    #[test]
    fn selection_modes_ctrl_a_escape_and_disallow_empty_are_honored() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"]))
                .disallow_empty_selection(true),
        );

        drop(grid.send(Event::ToggleSelect(key("alpha"))));

        assert_eq!(grid.context().selected_keys.get(), &selected(&["alpha"]));

        drop(grid.send(Event::ToggleSelect(key("gamma"))));

        assert_eq!(
            grid.context().selected_keys.get(),
            &selected(&["alpha", "gamma"])
        );

        drop(grid.send(Event::SelectAll));

        assert_eq!(
            grid.context().selected_keys.get(),
            &selected(&["alpha", "gamma", "delta", "echo"])
        );

        drop(grid.send(Event::ClearSelection));

        assert_eq!(
            grid.context().selected_keys.get(),
            &selected(&["alpha", "gamma", "delta", "echo"])
        );

        let mut single = service(
            Props::new()
                .id("single")
                .items(items())
                .selection_mode(selection::Mode::Single),
        );

        drop(single.send(Event::ToggleSelect(key("alpha"))));
        drop(single.send(Event::ToggleSelect(key("gamma"))));

        assert_eq!(single.context().selected_keys.get(), &selected(&["gamma"]));

        let mut clearable = service(
            Props::new()
                .id("clearable")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha", "gamma"])),
        );

        drop(clearable.send(Event::SelectRange {
            from: key("alpha"),
            to: key("delta"),
        }));

        assert_eq!(
            clearable.context().selected_keys.get(),
            &selected(&["alpha", "gamma", "delta"])
        );

        drop(clearable.send(Event::ClearSelection));

        assert!(clearable.context().selected_keys.get().is_empty());
    }

    #[test]
    fn keyboard_helper_dispatches_selection_action_typeahead_and_ignores_modified_text() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);

        let mut service = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple),
        );

        drop(service.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));

        let api = service.connect(&send);

        api.on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Enter, None),
            Duration::ZERO,
        );
        api.on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Space, Some(' ')),
            Duration::ZERO,
        );

        let mut ctrl_a = keyboard(KeyboardKey::Unidentified, Some('a'));

        ctrl_a.ctrl_key = true;

        api.on_cell_keydown_at(&key("alpha"), &ctrl_a, Duration::ZERO);

        api.on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Unidentified, Some('g')),
            Duration::from_millis(1),
        );

        let mut composing = keyboard(KeyboardKey::Unidentified, Some('d'));

        composing.is_composing = true;

        api.on_cell_keydown_at(&key("alpha"), &composing, Duration::from_millis(2));

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::ItemAction(key("alpha")),
                Event::ToggleSelect(key("alpha")),
                Event::SelectAll,
                Event::TypeaheadSearch {
                    ch: 'g',
                    now: Duration::from_millis(1),
                },
            ]
        );
    }

    #[test]
    fn keyboard_helper_honors_replace_selection_behavior_for_space() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);
        let grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace),
        );

        grid.connect(&send).on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Space, Some(' ')),
            Duration::ZERO,
        );

        assert_eq!(captured.into_inner(), vec![Event::Select(key("alpha"))]);
    }

    #[test]
    fn set_props_syncs_controlled_selection_focus_and_context_fields() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns"))
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha", "gamma"]))
                .dnd_enabled(true),
        );

        drop(grid.send(Event::Focus {
            key: Some(key("gamma")),
            is_keyboard: true,
        }));
        drop(grid.send(Event::ToggleSelect(key("delta"))));

        assert_eq!(
            grid.connect(&|_| {}).requested_selected_keys(),
            Some(&selected(&["alpha", "gamma", "delta"]))
        );

        let next_items = StaticCollection::new([
            (key("alpha"), "Alpha".into(), item("alpha", "Alpha", false)),
            (key("zeta"), "Zeta".into(), item("zeta", "Zeta", false)),
        ]);

        let next = Props::new()
            .id("renamed")
            .items(next_items.clone())
            .columns(NonZero::new(1).expect("non-zero columns"))
            .selected_keys(selected(&["alpha", "missing", "zeta"]))
            .selection_mode(selection::Mode::Multiple)
            .selection_behavior(selection::Behavior::Replace)
            .disabled(true)
            .disabled_keys(selected(&["alpha"]))
            .disallow_empty_selection(true)
            .escape_key_behavior(EscapeKeyBehavior::None)
            .composite(false)
            .loading(true);

        let result = grid.set_props(next);

        assert!(result.context_changed);

        let ctx = grid.context();

        assert_eq!(ctx.items, next_items);
        assert_eq!(ctx.columns.get(), 1);
        assert_eq!(ctx.focused_key, None);
        assert!(!ctx.focus_visible);
        assert_eq!(ctx.selection_behavior, selection::Behavior::Replace);
        assert!(ctx.disabled);
        assert!(ctx.disabled_keys.contains(&key("alpha")));
        assert!(ctx.disallow_empty_selection);
        assert_eq!(ctx.escape_key_behavior, EscapeKeyBehavior::None);
        assert!(!ctx.composite);
        assert!(ctx.loading);
        assert!(!ctx.dnd_enabled);
        assert_eq!(ctx.requested_selected_keys, None);
        assert_eq!(ctx.selected_keys.get(), &selected(&["zeta"]));
    }

    #[test]
    fn controlled_selection_attrs_render_bound_value_not_pending_request() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .selected_keys(selected(&["alpha"])),
        );

        drop(grid.send(Event::ToggleSelect(key("gamma"))));

        let api = grid.connect(&|_| {});

        assert_eq!(
            api.requested_selected_keys(),
            Some(&selected(&["alpha", "gamma"]))
        );
        assert_eq!(
            api.cell_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("true")
        );
        assert_eq!(
            api.cell_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("false")
        );
        assert_eq!(
            api.row_attrs(&key("gamma"))
                .get(&HtmlAttr::Data("ars-selected")),
            None
        );
    }

    #[test]
    fn structural_collection_keys_are_rejected_from_selected_state() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(sectioned_items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&[
                    "section",
                    "section-header",
                    "separator-3",
                    "alpha",
                    "gamma",
                ])),
        );

        assert_eq!(
            grid.context().selected_keys.get(),
            &selected(&["alpha", "gamma"])
        );

        let next = Props::new()
            .id("grid")
            .items(sectioned_items())
            .selection_mode(selection::Mode::Multiple)
            .selected_keys(selected(&["section", "section-header", "alpha"]));

        drop(grid.set_props(next));

        assert_eq!(grid.context().selected_keys.get(), &selected(&["alpha"]));
    }

    #[test]
    fn structural_collection_nodes_do_not_consume_grid_cells() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(sectioned_items())
                .columns(NonZero::new(2).expect("non-zero columns")),
        );

        let api = grid.connect(&|_| {});

        assert_eq!(api.grid_dimensions(), "1 rows, 2 columns");
        assert_eq!(
            api.cell_position(&key("alpha")).as_deref(),
            Some("Row 1, Column 1")
        );
        assert_eq!(
            api.cell_position(&key("gamma")).as_deref(),
            Some("Row 1, Column 2")
        );
        assert_eq!(api.cell_position(&key("section")), None);

        drop(grid.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(grid.send(Event::FocusRight));

        assert_eq!(grid.context().focused_key, Some(key("gamma")));
    }

    #[test]
    fn disabled_grid_rejects_user_mutations_but_blur_still_clears_focus() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"])),
        );

        drop(grid.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(grid.set_props(grid.props().clone().disabled(true)));

        for event in [
            Event::Focus {
                key: Some(key("gamma")),
                is_keyboard: true,
            },
            Event::Select(key("gamma")),
            Event::ToggleSelect(key("gamma")),
            Event::ItemAction(key("gamma")),
            Event::FocusDown,
            Event::TypeaheadSearch {
                ch: 'g',
                now: Duration::from_millis(1),
            },
        ] {
            drop(grid.send(event));
        }

        assert_eq!(grid.context().focused_key, None);
        assert_eq!(grid.context().selected_keys.get(), &selected(&["alpha"]));

        drop(grid.send(Event::Blur));

        assert_eq!(grid.context().focused_key, None);
        assert!(!grid.context().focus_visible);
    }

    #[test]
    fn accessors_and_connect_part_attrs_dispatch_all_parts() {
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns"))
                .selection_mode(selection::Mode::Multiple)
                .on_load_more(Callback::new_void(|| {}))
                .dnd_enabled(true),
        );

        drop(grid.send(Event::ToggleSelect(key("alpha"))));

        let api = grid.connect(&|_| {});

        assert_eq!(api.grid_dimensions(), "3 rows, 2 columns");
        assert_eq!(
            api.cell_position(&key("delta")).as_deref(),
            Some("Row 2, Column 2")
        );
        assert_eq!(api.cell_id(&key("missing")), None);
        assert!(api.items().contains_key(&key("alpha")));
        assert_eq!(api.requested_selected_keys(), Some(&selected(&["alpha"])));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(
            api.part_attrs(Part::Row { key: key("alpha") }),
            api.row_attrs(&key("alpha"))
        );
        assert_eq!(
            api.part_attrs(Part::Cell { key: key("alpha") }),
            api.cell_attrs(&key("alpha"))
        );
        assert_eq!(
            api.part_attrs(Part::LoadingSentinel),
            api.loading_sentinel_attrs().expect("sentinel attrs")
        );
        assert_eq!(
            api.part_attrs(Part::DragHandle { key: key("alpha") }),
            api.drag_handle_attrs(&key("alpha"))
        );
        assert_eq!(
            api.part_attrs(Part::DropIndicator),
            part_attrs(&Part::DropIndicator)
        );
    }

    #[test]
    fn non_composite_keyboard_helper_dispatches_only_activation_and_selection() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);

        let grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Single)
                .selection_behavior(selection::Behavior::Replace)
                .composite(false),
        );

        let api = grid.connect(&send);
        let mut ctrl_a = keyboard(KeyboardKey::Unidentified, Some('a'));

        ctrl_a.ctrl_key = true;

        api.on_cell_keydown(&key("alpha"), &keyboard(KeyboardKey::Enter, None));
        api.on_cell_keydown(&key("alpha"), &keyboard(KeyboardKey::Space, Some(' ')));
        api.on_cell_keydown(&key("alpha"), &keyboard(KeyboardKey::Escape, None));
        api.on_cell_keydown(&key("alpha"), &ctrl_a);
        api.on_cell_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowDown, None));

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::ItemAction(key("alpha")),
                Event::Select(key("alpha")),
                Event::ClearSelection,
                Event::SelectAll,
            ]
        );
    }

    #[test]
    fn item_action_sets_requested_key_and_invokes_callback() {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let callback = Callback::new({
            let actions = Arc::clone(&actions);

            move |key: Key| actions.lock().expect("actions lock").push(key)
        });
        let mut grid = service(Props::new().id("grid").items(items()).on_action(callback));

        let result = grid.send(Event::ItemAction(key("alpha")));

        assert_eq!(
            grid.connect(&|_| {}).requested_action_key(),
            Some(&key("alpha"))
        );
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::Action);

        let send: StrongSend<Event> = Arc::new(|_| {});
        let effect = result
            .pending_effects
            .into_iter()
            .next()
            .expect("action effect");

        drop(effect.run(grid.context(), grid.props(), send));

        assert_eq!(*actions.lock().expect("actions lock"), vec![key("alpha")]);

        drop(grid.set_props(grid.props().clone().loading(true)));

        assert_eq!(grid.connect(&|_| {}).requested_action_key(), None);
    }

    #[test]
    fn transition_edges_cover_empty_disabled_mode_and_boundary_noops() {
        let disabled_items = StaticCollection::new([
            (key("alpha"), "Alpha".into(), item("alpha", "Alpha", true)),
            (key("beta"), "Beta".into(), item("beta", "Beta", true)),
        ]);

        let mut empty_focus = service(Props::new().id("disabled-only").items(disabled_items));

        drop(empty_focus.send(Event::Focus {
            key: None,
            is_keyboard: true,
        }));

        assert_eq!(empty_focus.state(), &State::Idle);
        assert_eq!(empty_focus.context().focused_key, None);
        assert!(!empty_focus.context().focus_visible);

        let mut none_mode = service(Props::new().id("none").items(items()));

        for event in [
            Event::Select(key("alpha")),
            Event::ToggleSelect(key("alpha")),
            Event::SelectRange {
                from: key("alpha"),
                to: key("gamma"),
            },
            Event::SelectAll,
            Event::ClearSelection,
            Event::ItemAction(key("beta")),
        ] {
            drop(none_mode.send(event));
        }

        assert!(none_mode.context().selected_keys.get().is_empty());

        let mut multiple = service(
            Props::new()
                .id("multiple")
                .items(items())
                .selection_mode(selection::Mode::Multiple),
        );

        drop(multiple.send(Event::SelectRange {
            from: key("delta"),
            to: key("alpha"),
        }));

        assert_eq!(
            multiple.context().selected_keys.get(),
            &selected(&["alpha", "gamma", "delta"])
        );

        drop(multiple.send(Event::SelectRange {
            from: key("missing"),
            to: key("alpha"),
        }));

        drop(multiple.send(Event::SelectRange {
            from: key("beta"),
            to: key("delta"),
        }));

        assert_eq!(
            multiple.context().selected_keys.get(),
            &selected(&["alpha", "gamma", "delta"])
        );

        let mut all_disabled = service(
            Props::new()
                .id("all-disabled")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .disabled(true),
        );

        drop(all_disabled.send(Event::SelectAll));

        assert!(all_disabled.context().selected_keys.get().is_empty());

        let mut boundary = service(
            Props::new()
                .id("boundary")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns")),
        );

        drop(boundary.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));

        for event in [Event::FocusUp, Event::FocusLeft] {
            drop(boundary.send(event));

            assert_eq!(boundary.context().focused_key, Some(key("alpha")));
        }

        drop(boundary.send(Event::FocusLast));

        assert_eq!(boundary.context().focused_key, Some(key("echo")));

        for invalid_key in [key("beta"), key("missing")] {
            drop(boundary.send(Event::Focus {
                key: Some(invalid_key),
                is_keyboard: true,
            }));

            assert_eq!(boundary.context().focused_key, Some(key("echo")));
            assert!(boundary.context().focus_visible);
        }

        drop(boundary.send(Event::FocusDown));

        assert_eq!(boundary.context().focused_key, Some(key("echo")));
    }

    #[test]
    fn attr_edges_cover_disabled_root_missing_items_and_default_tabstop() {
        let disabled = service(
            Props::new()
                .id("disabled")
                .items(items())
                .selection_mode(selection::Mode::Single)
                .disabled(true),
        );

        let disabled_api = disabled.connect(&|_| {});

        assert_eq!(
            disabled_api
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
        assert_eq!(
            disabled_api.root_attrs().get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert_eq!(
            disabled_api
                .cell_attrs(&key("alpha"))
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );

        let disabled_link = service(
            Props::new()
                .id("disabled-link")
                .items(StaticCollection::new([(
                    key("alpha"),
                    "Alpha".into(),
                    link_item("alpha", "Alpha", SafeUrl::from_static("/disabled")),
                )]))
                .disabled_keys(selected(&["alpha"])),
        );

        assert!(
            disabled_link
                .connect(&|_| {})
                .cell_attrs(&key("alpha"))
                .get(&HtmlAttr::Href)
                .is_none()
        );

        let mut idle = service(
            Props::new()
                .id("idle")
                .items(items())
                .selection_mode(selection::Mode::Single),
        );

        drop(idle.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: false,
        }));

        let idle_api = idle.connect(&|_| {});

        assert_eq!(
            idle_api.cell_attrs(&key("alpha")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert!(
            idle_api
                .cell_attrs(&key("alpha"))
                .get(&HtmlAttr::Data("ars-focus-visible"))
                .is_none()
        );
        assert_eq!(
            idle_api.cell_attrs(&key("gamma")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert!(
            idle_api
                .cell_attrs(&key("missing"))
                .get(&HtmlAttr::Id)
                .is_none()
        );

        let non_composite = service(
            Props::new()
                .id("non-composite")
                .items(items())
                .composite(false),
        );
        let stale_attrs = non_composite.connect(&|_| {}).cell_attrs(&key("missing"));

        assert_eq!(stale_attrs.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            stale_attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
        assert_eq!(
            stale_attrs.get(&HtmlAttr::Data("ars-disabled")),
            Some("true")
        );

        assert_eq!(
            idle_api
                .drag_handle_attrs(&key("missing"))
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert_eq!(
            idle_api
                .drop_indicator_attrs(&CollectionDropTarget {
                    key: key("missing"),
                    position: ars_collections::DropPosition::After,
                })
                .get(&HtmlAttr::Data("ars-drop-target")),
            Some(key("missing").to_string().as_str())
        );

        let selection_none = service(
            Props::new()
                .id("selection-none")
                .items(items())
                .selection_mode(selection::Mode::None),
        );

        assert!(
            selection_none
                .connect(&|_| {})
                .row_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected))
                .is_none()
        );
    }

    #[test]
    fn composite_keyboard_helper_dispatches_navigation_and_modified_edges() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);

        let grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .escape_key_behavior(EscapeKeyBehavior::None),
        );

        let api = grid.connect(&send);

        for keydown in [
            KeyboardKey::ArrowUp,
            KeyboardKey::ArrowDown,
            KeyboardKey::ArrowLeft,
            KeyboardKey::ArrowRight,
            KeyboardKey::Home,
            KeyboardKey::End,
        ] {
            api.on_cell_keydown_at(
                &key("alpha"),
                &keyboard(keydown, None),
                Duration::from_millis(9),
            );
        }

        api.on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Escape, None),
            Duration::from_millis(9),
        );

        let mut meta_a = keyboard(KeyboardKey::Unidentified, Some('A'));

        meta_a.meta_key = true;

        api.on_cell_keydown_at(&key("alpha"), &meta_a, Duration::from_millis(9));

        let mut alt_text = keyboard(KeyboardKey::Unidentified, Some('x'));

        alt_text.alt_key = true;

        api.on_cell_keydown_at(&key("alpha"), &alt_text, Duration::from_millis(9));

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::FocusUp,
                Event::FocusDown,
                Event::FocusLeft,
                Event::FocusRight,
                Event::FocusFirst,
                Event::FocusLast,
                Event::SelectAll,
            ]
        );

        let clear_events = RefCell::new(Vec::new());
        let clear_send = |event| clear_events.borrow_mut().push(event);
        let clear_grid = service(
            Props::new()
                .id("clear-grid")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"])),
        );

        clear_grid.connect(&clear_send).on_cell_keydown_at(
            &key("alpha"),
            &keyboard(KeyboardKey::Escape, None),
            Duration::from_millis(9),
        );

        assert_eq!(clear_events.into_inner(), vec![Event::ClearSelection]);
    }

    #[test]
    fn keyboard_helper_extends_selection_with_shift_arrow_and_reverses_rtl() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);
        let env = Env {
            locale: ars_i18n::locales::ar(),
            ..Default::default()
        };

        let grid = Service::<Machine>::new(
            Props::new()
                .id("grid")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns"))
                .selection_mode(selection::Mode::Multiple),
            &env,
            &Messages::default(),
        );
        let api = grid.connect(&send);
        let mut shift_left = keyboard(KeyboardKey::ArrowLeft, None);

        shift_left.shift_key = true;
        api.on_cell_keydown_at(&key("gamma"), &shift_left, Duration::from_millis(9));

        let mut shift_right = keyboard(KeyboardKey::ArrowRight, None);

        shift_right.shift_key = true;
        api.on_cell_keydown_at(&key("delta"), &shift_right, Duration::from_millis(10));

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::FocusRight,
                Event::SelectRange {
                    from: key("gamma"),
                    to: key("delta"),
                },
                Event::FocusLeft,
                Event::SelectRange {
                    from: key("delta"),
                    to: key("gamma"),
                },
            ]
        );
    }

    #[test]
    fn keyboard_helper_preserves_shift_range_anchor_across_repeated_arrows() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(items())
                .columns(NonZero::new(2).expect("non-zero columns"))
                .selection_mode(selection::Mode::Multiple),
        );

        drop(grid.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(grid.send(Event::FocusDown));
        drop(grid.send(Event::SelectRange {
            from: key("alpha"),
            to: key("gamma"),
        }));

        let mut shift_right = keyboard(KeyboardKey::ArrowRight, None);

        shift_right.shift_key = true;
        grid.connect(&send).on_cell_keydown_at(
            &key("gamma"),
            &shift_right,
            Duration::from_millis(10),
        );

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::FocusRight,
                Event::SelectRange {
                    from: key("alpha"),
                    to: key("delta"),
                },
            ]
        );
    }

    #[test]
    fn select_range_preserves_non_empty_selection_when_range_has_no_enabled_items() {
        let disabled_items = StaticCollection::new([
            (key("alpha"), "Alpha".into(), item("alpha", "Alpha", false)),
            (key("beta"), "Beta".into(), item("beta", "Beta", true)),
        ]);
        let mut grid = service(
            Props::new()
                .id("grid")
                .items(disabled_items)
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"]))
                .disallow_empty_selection(true),
        );

        drop(grid.send(Event::SelectRange {
            from: key("beta"),
            to: key("beta"),
        }));

        assert_eq!(grid.context().selected_keys.get(), &selected(&["alpha"]));
    }

    #[test]
    fn convenience_keydown_uses_clock_backed_typeahead_timestamp() {
        let captured = RefCell::new(Vec::new());
        let send = |event| captured.borrow_mut().push(event);
        let mut grid = service(Props::new().id("grid").items(items()));

        drop(grid.send(Event::TypeaheadSearch {
            ch: 'g',
            now: Duration::from_millis(100),
        }));

        let api = grid.connect(&send);
        api.on_cell_keydown(
            &key("gamma"),
            &keyboard(KeyboardKey::Unidentified, Some('d')),
        );

        let events = captured.into_inner();

        assert_eq!(events.len(), 1);

        match events.as_slice() {
            [Event::TypeaheadSearch { ch: 'd', now }] => {
                assert!(
                    now.saturating_sub(Duration::from_millis(100)) > typeahead::TYPEAHEAD_TIMEOUT
                );
            }
            other => panic!("expected typeahead event, got {other:?}"),
        }
    }

    #[test]
    fn typeahead_moves_focus_to_matching_enabled_item() {
        let mut service = service(Props::new().id("grid").items(items()));

        drop(service.send(Event::Focus {
            key: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(service.send(Event::TypeaheadSearch {
            ch: 'g',
            now: Duration::from_millis(1),
        }));

        assert_eq!(service.context().focused_key, Some(key("gamma")));
    }

    #[test]
    fn empty_grid_and_loading_sentinel_attrs_are_stable() {
        let empty = service(Props::new().id("empty"));

        let empty_api = empty.connect(&|_| {});

        assert_snapshot!(
            "grid_list_empty_root",
            snapshot_attrs(&empty_api.root_attrs())
        );
        assert!(empty_api.loading_sentinel_attrs().is_none());

        let loaded = service(
            Props::new()
                .id("loading")
                .items(items())
                .on_load_more(Callback::new_void(|| {}))
                .loading(true),
        );

        let loaded_api = loaded.connect(&|_| {});

        assert_snapshot!(
            "grid_list_loading_sentinel",
            snapshot_attrs(&loaded_api.loading_sentinel_attrs().expect("sentinel attrs"))
        );
    }

    #[test]
    fn dnd_enabled_surfaces_draggable_cell_handle_and_drop_indicator_attrs() {
        let service = service(Props::new().id("grid").items(items()).dnd_enabled(true));

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "grid_list_cell_dnd_enabled",
            snapshot_attrs(&api.cell_attrs(&key("alpha")))
        );
        assert!(
            api.cell_attrs(&key("beta"))
                .get(&HtmlAttr::Aria(AriaAttr::RoleDescription))
                .is_none()
        );
        assert_snapshot!(
            "grid_list_drag_handle_enabled",
            snapshot_attrs(&api.drag_handle_attrs(&key("alpha")))
        );
        assert_snapshot!(
            "grid_list_drag_handle_disabled_item",
            snapshot_attrs(&api.drag_handle_attrs(&key("beta")))
        );
        assert_snapshot!(
            "grid_list_drop_indicator_before",
            snapshot_attrs(&api.drop_indicator_attrs(&CollectionDropTarget {
                key: key("gamma"),
                position: ars_collections::DropPosition::Before,
            }))
        );
    }
}
