//! TreeView navigation component.
//!
//! TreeView owns the framework-agnostic state of a hierarchical, keyboard
//! navigable tree implementing the WAI-ARIA tree pattern (`role="tree"`,
//! `role="treeitem"`, `role="group"`): expanded/collapsed branch keys, node
//! selection (single or multiple), the active-descendant focus indicator,
//! typeahead, and the ARIA/`data-ars-*` attributes for every anatomy part.
//!
//! The data model is a [`TreeCollection<TreeItem>`] from `ars-collections`,
//! which owns parent/child relationships and visible-key computation.
//!
//! The agnostic core never touches the DOM. Focus uses the active-descendant
//! pattern (the root keeps `tabindex="0"` and points `aria-activedescendant`
//! at the focused node's stable id); when focus moves the core emits a
//! [`Effect::ScrollFocusedIntoView`] intent and adapters resolve the live
//! element to scroll. The optional drag-and-drop reorder surface
//! ([`Props::dnd_enabled`]) owns all pure data-model logic — valid drop-slot
//! enumeration, cycle-free drop validation, and [`ReorderEvent`] path
//! computation — and surfaces completion through [`Props::on_reorder`] driven
//! by a named [`Effect::Reorder`]. Pointer hit-testing, hover-expand timing,
//! and `use_drag`/`use_drop` wiring remain adapter concerns.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_collections::{
    Collection, Key, Node, TreeCollection, TreeItemConfig,
    dnd::{CollectionDropTarget, DropPosition},
    selection, typeahead,
};
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

use super::key_token::dom_safe_key_token;

/// Value type stored in the [`TreeCollection`] backing a [`TreeView`](self).
///
/// Carries the per-node properties the machine needs that are not part of the
/// collection's structural model: the display label, disabled state, the
/// lazy-load `has_children` affordance, and an optional navigation `href`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeItem {
    /// Display label for the node (used in typeahead and accessibility).
    pub label: String,

    /// Whether this node is disabled. Disabled nodes are not selectable and
    /// cannot be dragged; they remain focusable so their state is announced.
    pub disabled: bool,

    /// When `true`, the node renders an expand affordance even if it has no
    /// children loaded yet. Essential for lazy-loaded trees where the child
    /// count is unknown until the node is first expanded.
    pub has_children: bool,

    /// Optional navigation URL. When `Some`, the clickable area renders as an
    /// `<a>` element (native middle-click / Ctrl+click), while the parent
    /// `role="treeitem"` is preserved.
    pub href: Option<String>,
}

/// The lazy-load status of a node's children (spec §5.3).
///
/// Tracked per node in [`Context::load_state`]. A branch advertising the
/// [`TreeItem::has_children`] affordance but with no children loaded yet starts
/// `NotLoaded`; expanding it transitions to `Loading` and emits
/// [`Effect::LoadChildren`]; [`Event::ChildrenLoaded`] settles it to `Loaded`
/// while [`Event::LoadError`] settles it to `Error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeLoadState {
    /// Children are present in the collection.
    Loaded,

    /// Children are being fetched (loading indicator shown).
    Loading,

    /// Load failed — adapters may show a retry affordance.
    Error,

    /// Children have not been requested yet (initial state for lazy nodes).
    NotLoaded,
}

/// Machine states for [`TreeView`](self).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The tree does not have keyboard focus.
    #[default]
    Idle,

    /// The tree has keyboard focus on some node.
    Focused,
}

/// Events accepted by the [`TreeView`](self) state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Expand a branch node.
    ExpandNode(Key),

    /// Collapse a branch node.
    CollapseNode(Key),

    /// Toggle the expand/collapse state of a branch node.
    ToggleNode(Key),

    /// Add a node to the selection (respecting selection mode/behavior).
    SelectNode(Key),

    /// Remove a node from the selection.
    DeselectNode(Key),

    /// Move the focus indicator to a specific node.
    FocusNode(Key),

    /// Move focus to the next visible node (does not wrap).
    FocusNext,

    /// Move focus to the previous visible node (does not wrap).
    FocusPrev,

    /// Move focus to the first visible node.
    FocusFirst,

    /// Move focus to the last visible node.
    FocusLast,

    /// Move focus to the parent of the focused node.
    FocusParent,

    /// The tree container received focus.
    Focus {
        /// Whether focus arrived via the keyboard.
        is_keyboard: bool,
    },

    /// The tree container lost focus.
    Blur,

    /// Append a character to the typeahead buffer and jump to the next matching
    /// node. Carries the adapter-provided monotonic timestamp so the shared
    /// [`typeahead::State`] can reset the buffer after its timeout.
    TypeaheadSearch(char, Duration),

    /// Reset the typeahead buffer (e.g. on blur or an explicit timeout tick).
    ClearTypeahead,

    /// Expand every expandable node in the tree.
    ExpandAll,

    /// Collapse every node in the tree.
    CollapseAll,

    /// Begin a drag (keyboard pickup or pointer drag start) on a node.
    /// Ignored unless [`Props::dnd_enabled`].
    DragStart(Key),

    /// Set the resolved drop target (adapter pointer hit-testing). Ignored when
    /// no drag is active or the target is invalid.
    DragOver(CollectionDropTarget),

    /// Step the keyboard drop target to the next valid slot.
    DragMoveNext,

    /// Step the keyboard drop target to the previous valid slot.
    DragMovePrev,

    /// Confirm the drop at the current drop target (fires [`Props::on_reorder`]).
    Drop,

    /// Cancel the active drag and discard the drop target.
    CancelDrag,

    /// Re-sync prop-derived context (the data source and controlled
    /// `selected`/`expanded` bindings) after the consumer supplies new props.
    /// Emitted by [`Machine::on_props_changed`](ars_core::Machine::on_props_changed).
    SyncProps,

    // ── Lazy loading (spec §5) ──────────────────────────────────────────
    /// The consumer (via the adapter's lazy-load callback) delivers the
    /// lazily-fetched children for `parent`. The machine inserts them into the
    /// collection under `parent` and marks the parent's
    /// [`NodeLoadState`] `Loaded`.
    ChildrenLoaded {
        /// The parent node key the children belong to.
        parent: Key,

        /// The fetched child configurations, inserted under `parent`.
        children: Vec<TreeItemConfig<TreeItem>>,
    },

    /// The consumer reports that a lazy load failed for `key`; the machine marks
    /// the node's [`NodeLoadState`] `Error` so adapters can show a retry
    /// affordance.
    LoadError(Key),

    // ── Renamable nodes (spec §6) ───────────────────────────────────────
    /// Begin an inline rename of a node (e.g. F2 or slow double-click). Ignored
    /// unless [`Props::renamable`] and the node is enabled.
    RenameStart(Key),

    /// Commit the new name for a node currently being renamed. The machine
    /// clears the rename state; the consumer persists the new name by updating
    /// its data source and supplying new props.
    RenameCommit {
        /// The node being renamed.
        key: Key,

        /// The committed new name (the rename input's current value).
        new_name: String,
    },

    /// Cancel an in-progress rename for a node (Escape or blur without change),
    /// discarding the edit.
    RenameCancel(Key),
}

/// Typed effect intents emitted by the [`TreeView`](self) machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter must scroll the active-descendant node ([`Context::focused_node`])
    /// into view. The agnostic core never moves DOM focus itself; the tree uses
    /// the active-descendant pattern, so only scrolling is adapter-resolved.
    ScrollFocusedIntoView,

    /// Adapter invokes [`Props::on_selection_change`] with the requested
    /// selection set. Emitted on every user-requested selection change so a
    /// controlled parent can echo the new value back through `Props::selected`.
    SelectionChange,

    /// Adapter invokes [`Props::on_expanded_change`] with the requested expanded
    /// set. Emitted on every user-requested expansion change so a controlled
    /// parent can echo the new value back through `Props::expanded`.
    ExpandedChange,

    /// Adapter invokes [`Props::on_reorder`] with the completed [`ReorderEvent`].
    Reorder,

    /// Adapter invokes [`Props::on_load_children`] with the branch key whose
    /// children must be lazily fetched. Emitted when a `NotLoaded` (or
    /// previously failed `Error`) branch is expanded; the consumer eventually
    /// sends [`Event::ChildrenLoaded`] (or [`Event::LoadError`]) back to the
    /// machine.
    LoadChildren,

    /// Adapter invokes [`Props::on_rename`] with the committed
    /// [`RenameEvent`] so the consumer can persist the new label (renamable
    /// variant, spec §6).
    Rename,
}

/// Emitted when a tree node is reordered via drag-and-drop.
///
/// Paths are sequences of keys from the root to the item, so consumers can
/// apply the reorder against their hierarchical data source without resolving
/// flat indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderEvent {
    /// Path of the dragged node (root → node).
    pub source_path: Vec<Key>,

    /// Path of the drop target node (root → target).
    pub target_path: Vec<Key>,

    /// Where, relative to the target, the node was dropped.
    pub position: DropPosition,
}

/// Emitted when an inline rename commits (renamable variant, spec §6).
///
/// Delivered to [`Props::on_rename`] via [`Effect::Rename`]. The consumer is
/// responsible for persisting the new label (e.g. updating the
/// [`TreeCollection`] data source); the machine never mutates
/// [`TreeItem::label`] on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameEvent {
    /// The node whose label was edited.
    pub key: Key,

    /// The new label value the user committed.
    pub new_name: String,
}

/// Closure signature backing [`Messages::loading_label`].
pub type LoadingLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Closure signature backing [`Messages::drag_handle_label`]. Receives the
/// node's label and the active locale.
pub type DragHandleLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Closure signature backing [`Messages::rename_label`]. Receives the node's
/// current label and the active locale.
pub type RenameLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Localizable messages for [`TreeView`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Loading indicator text for branches whose children are being fetched
    /// (default: `"Loading…"`). Rendered as visually hidden text and announced
    /// via `aria-live="polite"` by adapters.
    pub loading_label: MessageFn<LoadingLabelFn>,

    /// Accessible label template for a node's drag handle, called with the
    /// node's label (default: `"Drag {label}"`).
    pub drag_handle_label: MessageFn<DragHandleLabelFn>,

    /// Accessible label for the inline rename input (spec §6.7), called with the
    /// node's current label and the resolved locale
    /// (default: `"Rename {node_name}"`).
    pub rename_label: MessageFn<RenameLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading\u{2026}"),
            drag_handle_label: MessageFn::new(|label: &str, _locale: &Locale| {
                alloc::format!("Drag {label}")
            }),
            rename_label: MessageFn::new(|node_name: &str, _locale: &Locale| {
                alloc::format!("Rename {node_name}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for a [`TreeView`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id (base for generated part/node ids).
    pub id: String,

    /// The tree data source.
    pub items: TreeCollection<TreeItem>,

    /// Controlled selected node keys. `Some` = controlled, `None` = uncontrolled.
    pub selected: Option<selection::Set>,

    /// Uncontrolled initial selection.
    pub default_selected: selection::Set,

    /// Controlled expanded branch keys. `Some` = controlled, `None` = uncontrolled.
    pub expanded: Option<BTreeSet<Key>>,

    /// Default expanded branch keys when uncontrolled.
    pub default_expanded: BTreeSet<Key>,

    /// Allow multiple nodes to be selected simultaneously (drives
    /// `aria-multiselectable`).
    pub multiple: bool,

    /// Which nodes can be selected.
    pub selection_mode: selection::Mode,

    /// Selection behavior: `Toggle` (checkbox-like) or `Replace`
    /// (file-explorer-like).
    pub selection_behavior: selection::Behavior,

    /// Enable the drag-and-drop reorder surface.
    pub dnd_enabled: bool,

    /// When `true`, tree nodes can be renamed inline via F2 or slow
    /// double-click (spec §6). Default: `false`.
    pub renamable: bool,

    /// Called with the requested selection set whenever the user changes the
    /// selection. This is the controlled echo point: a controlled tree
    /// (`selected: Some`) updates `selected` from this callback so the rendered
    /// selection tracks the parent-owned value.
    pub on_selection_change: Option<Callback<dyn Fn(selection::Set) + Send + Sync>>,

    /// Called with the requested expanded-key set whenever the user expands or
    /// collapses a branch. This is the controlled echo point: a controlled tree
    /// (`expanded: Some`) updates `expanded` from this callback so the rendered
    /// expansion tracks the parent-owned value.
    pub on_expanded_change: Option<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,

    /// Called when a drag-and-drop reorder completes.
    pub on_reorder: Option<Callback<dyn Fn(ReorderEvent) + Send + Sync>>,

    /// Called with a branch's key when it is first expanded while its children
    /// are not yet loaded (spec §5). The app fetches the children — typically
    /// asynchronously — and sends [`Event::ChildrenLoaded`] back, or
    /// [`Event::LoadError`] on failure. Fired by [`Effect::LoadChildren`].
    pub on_load_children: Option<Callback<dyn Fn(Key) + Send + Sync>>,

    /// Called with the committed [`RenameEvent`] when an inline rename ends
    /// via Enter or blur (spec §6). The consumer persists the new label
    /// against its data source; the machine does not mutate
    /// [`TreeItem::label`]. Fired by [`Effect::Rename`].
    pub on_rename: Option<Callback<dyn Fn(RenameEvent) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: TreeCollection::new([]),
            selected: None,
            default_selected: selection::Set::default(),
            expanded: None,
            default_expanded: BTreeSet::new(),
            multiple: false,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            dnd_enabled: false,
            renamable: false,
            on_selection_change: None,
            on_expanded_change: None,
            on_reorder: None,
            on_load_children: None,
            on_rename: None,
        }
    }
}

impl Props {
    /// Creates default props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, value: impl Into<String>) -> Self {
        self.id = value.into();
        self
    }

    /// Sets [`items`](Self::items).
    #[must_use]
    pub fn items(mut self, value: TreeCollection<TreeItem>) -> Self {
        self.items = value;
        self
    }

    /// Sets the controlled [`selected`](Self::selected) set.
    #[must_use]
    pub fn selected(mut self, value: selection::Set) -> Self {
        self.selected = Some(value);
        self
    }

    /// Sets [`default_selected`](Self::default_selected).
    #[must_use]
    pub fn default_selected(mut self, value: selection::Set) -> Self {
        self.default_selected = value;
        self
    }

    /// Sets the controlled [`expanded`](Self::expanded) set.
    #[must_use]
    pub fn expanded(mut self, value: BTreeSet<Key>) -> Self {
        self.expanded = Some(value);
        self
    }

    /// Sets [`default_expanded`](Self::default_expanded).
    #[must_use]
    pub fn default_expanded(mut self, value: BTreeSet<Key>) -> Self {
        self.default_expanded = value;
        self
    }

    /// Sets [`multiple`](Self::multiple).
    #[must_use]
    pub const fn multiple(mut self, value: bool) -> Self {
        self.multiple = value;
        self
    }

    /// Sets [`selection_mode`](Self::selection_mode).
    #[must_use]
    pub const fn selection_mode(mut self, value: selection::Mode) -> Self {
        self.selection_mode = value;
        self
    }

    /// Sets [`selection_behavior`](Self::selection_behavior).
    #[must_use]
    pub const fn selection_behavior(mut self, value: selection::Behavior) -> Self {
        self.selection_behavior = value;
        self
    }

    /// Sets [`dnd_enabled`](Self::dnd_enabled).
    #[must_use]
    pub const fn dnd_enabled(mut self, value: bool) -> Self {
        self.dnd_enabled = value;
        self
    }

    /// Sets [`renamable`](Self::renamable).
    #[must_use]
    pub const fn renamable(mut self, value: bool) -> Self {
        self.renamable = value;
        self
    }

    /// Sets [`on_selection_change`](Self::on_selection_change).
    #[must_use]
    pub fn on_selection_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(selection::Set) + Send + Sync>>,
    ) -> Self {
        self.on_selection_change = Some(callback.into());
        self
    }

    /// Sets [`on_expanded_change`](Self::on_expanded_change).
    #[must_use]
    pub fn on_expanded_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
    ) -> Self {
        self.on_expanded_change = Some(callback.into());
        self
    }

    /// Sets [`on_reorder`](Self::on_reorder).
    #[must_use]
    pub fn on_reorder(
        mut self,
        callback: impl Into<Callback<dyn Fn(ReorderEvent) + Send + Sync>>,
    ) -> Self {
        self.on_reorder = Some(callback.into());
        self
    }

    /// Sets [`on_load_children`](Self::on_load_children).
    #[must_use]
    pub fn on_load_children(
        mut self,
        callback: impl Into<Callback<dyn Fn(Key) + Send + Sync>>,
    ) -> Self {
        self.on_load_children = Some(callback.into());
        self
    }

    /// Sets [`on_rename`](Self::on_rename).
    #[must_use]
    pub fn on_rename(
        mut self,
        callback: impl Into<Callback<dyn Fn(RenameEvent) + Send + Sync>>,
    ) -> Self {
        self.on_rename = Some(callback.into());
        self
    }
}

/// Runtime context for a [`TreeView`](self) instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The tree collection — nodes, parent/child relationships, visibility.
    ///
    /// May diverge from [`Self::items_prop`] when lazy loading splices children
    /// in via [`Event::ChildrenLoaded`]: the inserted subtree lives in
    /// `items` but the consumer's `Props::items` may still be the original
    /// lazy tree until they re-supply it.
    pub items: TreeCollection<TreeItem>,

    /// Last-seen `Props::items` snapshot, used by [`Event::SyncProps`] to
    /// decide whether the consumer actually changed the data source. Comparing
    /// the new `Props::items` against this baseline (instead of against the
    /// possibly-lazy-loaded [`Self::items`]) lets unrelated prop echoes (e.g.
    /// toggling `selected`/`expanded`/`renamable`) preserve children inserted
    /// via [`Event::ChildrenLoaded`].
    pub items_prop: TreeCollection<TreeItem>,

    /// Currently selected node keys (kept in sync with `selection_state`).
    pub selected: Bindable<selection::Set>,

    /// Full selection state machine (mode, behavior, anchor, focus).
    pub selection_state: selection::State,

    /// Currently expanded branch keys (expansion is independent of selection).
    pub expanded: Bindable<BTreeSet<Key>>,

    /// The node holding the active-descendant focus indicator.
    pub focused_node: Option<Key>,

    /// `true` when focus arrived via the keyboard.
    pub focus_visible: bool,

    /// Allow multiple selection (drives `aria-multiselectable`).
    pub multiple: bool,

    /// Which nodes can be selected.
    pub selection_mode: selection::Mode,

    /// Shared multi-character, locale-aware typeahead state (buffer, last-key
    /// time for timeout reset, and wrap-around start key). See
    /// [`typeahead::State`].
    pub typeahead: typeahead::State,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Generated element ids for all parts.
    pub ids: ComponentIds,

    /// The node currently being dragged, if any (drag-and-drop surface).
    pub dragging: Option<Key>,

    /// The resolved drop target during an active drag, if any.
    pub drop_target: Option<CollectionDropTarget>,

    /// Per-node lazy-load status (spec §5.3). Seeded at init (and reseeded on
    /// [`Event::SyncProps`]): a branch advertising the
    /// [`TreeItem::has_children`] affordance but with no loaded children is
    /// `NotLoaded`; every other node is `Loaded`.
    pub load_state: BTreeMap<Key, NodeLoadState>,

    /// The node currently being renamed, if any (spec §6.3). When `Some(key)`,
    /// the node identified by `key` renders a [`Part::NodeRenameInput`] instead
    /// of its text label.
    pub renaming_key: Option<Key>,
}

/// Anatomy parts exposed by the [`TreeView`](self) connect API.
#[derive(ComponentPart)]
#[scope = "tree-view"]
pub enum Part {
    /// The tree root container (`role="tree"`).
    Root,

    /// A branch node — expandable, has children (`role="treeitem"`).
    Branch {
        /// Node key.
        node_id: Key,
    },

    /// The clickable control row inside a branch (`<a>` when the node has href).
    BranchControl {
        /// Node key.
        node_id: Key,
    },

    /// The expand/collapse chevron indicator inside a branch.
    BranchIndicator {
        /// Node key.
        node_id: Key,
    },

    /// The text label inside a branch.
    BranchText,

    /// The children container inside an expanded branch (`role="group"`).
    BranchContent {
        /// Node key.
        node_id: Key,
    },

    /// A leaf node — no children (`role="treeitem"`).
    Leaf {
        /// Node key.
        node_id: Key,
    },

    /// The text label inside a leaf node.
    LeafText,

    /// The inline `<input type="text">` rendered in place of a node's text
    /// label while it is being renamed (renamable variant, spec §6.5).
    NodeRenameInput {
        /// Node key.
        node_id: Key,
    },

    /// An optional drag handle affordance for a draggable node.
    DragHandle {
        /// Node key.
        node_id: Key,
    },

    /// The visual indicator showing where a dragged node will be dropped.
    DropIndicator,
}

/// State machine for the [`TreeView`](self) component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let mut selected = if let Some(value) = &props.selected {
            Bindable::controlled(value.clone())
        } else {
            Bindable::uncontrolled(props.default_selected.clone())
        };

        let expanded = if let Some(value) = &props.expanded {
            Bindable::controlled(value.clone())
        } else {
            // Honor branches the collection marks expanded via
            // `TreeItemConfig::default_expanded`, unioned with the
            // component-level `default_expanded` prop, so either source of
            // initial expansion works (pit of success).
            let mut initial = props
                .items
                .all_nodes()
                .filter(|node| node.is_expanded == Some(true))
                .map(|node| node.key.clone())
                .collect::<BTreeSet<_>>();

            initial.extend(props.default_expanded.iter().cloned());

            Bindable::uncontrolled(initial)
        };

        // Disabled nodes are not selectable: seed the selection machine's
        // disabled-key set so `select` rejects them (`selection::State::select`
        // is a no-op for disabled keys).
        let disabled_keys = props
            .items
            .all_nodes()
            .filter(|node| node.value.as_ref().is_some_and(|item| item.disabled))
            .map(|node| node.key.clone())
            .collect::<BTreeSet<Key>>();

        // `multiple = true` upgrades the effective mode to `Multiple`.
        let selection_mode = effective_selection_mode(props);

        // Seed the selection state machine from the resolved initial selection,
        // normalized for the mode so it stays consistent with the `selected`
        // binding and never starts in a mode-invalid shape.
        let mut selection_state = selection::State::new(selection_mode, props.selection_behavior)
            .with_disabled(disabled_keys);

        // Disabled nodes stay focusable and typeahead-reachable (FocusOnly) while
        // `select` still rejects them, so they cannot be selected.
        selection_state.disabled_behavior = selection::DisabledBehavior::FocusOnly;

        // Drop any disabled or unknown key from the resolved initial selection so
        // a disabled node can never initialize as selected. Write the sanitized
        // set back to the binding too (a no-op for a controlled binding, which
        // the parent owns) so the public `ctx.selected` agrees with
        // `is_node_selected` and the rendered ARIA state from the first render.
        let sanitized = sanitize_selection(
            selected.get().clone(),
            selection_mode,
            &props.items,
            &selection_state.disabled_keys,
        );

        selected.set(sanitized.clone());
        selection_state.selected_keys = sanitized;

        (
            State::Idle,
            Context {
                items: props.items.clone(),
                items_prop: props.items.clone(),
                selected,
                selection_state,
                expanded,
                focused_node: None,
                focus_visible: false,
                multiple: props.multiple,
                selection_mode,
                typeahead: typeahead::State::default(),
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
                dragging: None,
                drop_target: None,
                load_state: seed_load_state(&props.items),
                renaming_key: None,
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            // Expand/collapse/toggle only act on a real, enabled, expandable
            // branch — never a leaf, a disabled node (interaction is blocked),
            // or a stale/unknown key (which would otherwise leave invalid keys
            // in the expansion set or mark a leaf `data-ars-expanded`).
            Event::ExpandNode(key) => {
                if !is_interactive_branch(&ctx.items, key) {
                    return None;
                }

                let mut next = ctx.expanded.get().clone();

                next.insert(key.clone());

                // Expanding never hides the focused node, so no focus clamp.
                expanded_change_plan(ctx, next, false, core::slice::from_ref(key))
            }

            Event::CollapseNode(key) => {
                if !is_interactive_branch(&ctx.items, key) {
                    return None;
                }

                let mut next = ctx.expanded.get().clone();

                next.remove(key);

                // A collapse may hide the focused node, so re-clamp focus.
                expanded_change_plan(ctx, next, true, &[])
            }

            Event::ToggleNode(key) => {
                if !is_interactive_branch(&ctx.items, key) {
                    return None;
                }

                let mut next = ctx.expanded.get().clone();

                // A toggle on an `Error` branch is treated as a retry rather
                // than a collapse: the spec §5 retry path says adapter retry
                // affordances re-dispatch `ExpandNode`/`ToggleNode`, and the
                // default branch-control click goes through `ToggleNode`. If
                // we collapsed on click, retrying would require collapse +
                // re-expand instead of one click.
                let is_error = ctx.load_state.get(key) == Some(&NodeLoadState::Error);
                let was_expanded = next.contains(key);
                let collapsing = was_expanded && !is_error;

                if collapsing {
                    next.remove(key);
                } else {
                    next.insert(key.clone());
                }

                // A toggle that expands may trigger a lazy load; a collapse
                // never does.
                let expanding: &[Key] = if collapsing {
                    &[]
                } else {
                    core::slice::from_ref(key)
                };

                expanded_change_plan(ctx, next, collapsing, expanding)
            }

            Event::SelectNode(key) => {
                // Ignore the event in non-selectable mode or for a stale/unknown
                // key, so selection never accumulates keys absent from `items`.
                if ctx.selection_mode == selection::Mode::None
                    || !ctx.items.get(key).is_some_and(Node::is_focusable)
                {
                    return None;
                }

                selection_change_plan(ctx, ctx.selection_state.select(key.clone()))
            }

            Event::DeselectNode(key) => {
                selection_change_plan(ctx, ctx.selection_state.deselect(key))
            }

            // Pointer / programmatic focus: do not force the keyboard focus
            // ring (modality is not keyboard). Only focus a node that is
            // currently visible, so the active descendant never dangles.
            Event::FocusNode(key) => {
                if !visible_keys(ctx).contains(key) {
                    return None;
                }

                Some(focus_plan(key.clone(), false))
            }

            // When the container gains focus with no active node, initialise it
            // to the first selected visible node, else the first visible node —
            // so `aria-activedescendant` is populated immediately.
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;

                let initial = if ctx.focused_node.is_some() {
                    None
                } else {
                    initial_active_node(ctx)
                };

                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            ctx.focus_visible = is_keyboard;

                            if let Some(key) = initial {
                                ctx.focused_node = Some(key);
                            }
                        })
                        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView)),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focus_visible = false;
                // Reset typeahead so a refocus within the timeout starts a
                // fresh search rather than appending to the old buffer.
                ctx.typeahead = typeahead::State::default();
            })),

            Event::FocusNext => focus_relative(ctx, Direction::Next),

            Event::FocusPrev => focus_relative(ctx, Direction::Prev),

            Event::FocusFirst => visible_keys(ctx)
                .first()
                .cloned()
                .map(|key| focus_plan(key, true)),

            Event::FocusLast => visible_keys(ctx)
                .last()
                .cloned()
                .map(|key| focus_plan(key, true)),

            Event::FocusParent => ctx
                .focused_node
                .as_ref()
                .and_then(|focused| ctx.items.get(focused))
                .and_then(|node| node.parent_key.clone())
                .map(|key| focus_plan(key, true)),

            Event::TypeaheadSearch(ch, now) => {
                // Delegate to the shared locale-aware, multi-character matcher.
                let (typeahead, found) = process_typeahead(ctx, *ch, *now);
                Some(if let Some(key) = found {
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            ctx.typeahead = typeahead;
                            ctx.focused_node = Some(key);
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView))
                } else {
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.typeahead = typeahead;
                    })
                })
            }

            Event::ClearTypeahead => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.typeahead = typeahead::State::default();
            })),

            Event::ExpandAll => {
                // Include lazy branches (the item-level `has_children` flag) so
                // bulk expansion can trigger consumer lazy-loading, matching how
                // `branch_attrs`/`leaf_attrs` treat the flag as expandable. Skip
                // disabled branches: disabled nodes block all interaction, so
                // ExpandAll must honor the same guard as `ExpandNode`/`ToggleNode`.
                let expandable = ctx
                    .items
                    .all_nodes()
                    .filter(|node| {
                        let item = node.value.as_ref();
                        (node.has_children || item.is_some_and(|item| item.has_children))
                            && !item.is_some_and(|item| item.disabled)
                    })
                    .map(|node| node.key.clone())
                    .collect::<Vec<_>>();

                let mut next = ctx.expanded.get().clone();

                next.extend(expandable.iter().cloned());

                // Bulk expansion emits a single `LoadChildren` effect (for the
                // first `NotLoaded` branch in `expandable`); the others remain
                // `NotLoaded` until separately expanded.
                expanded_change_plan(ctx, next, false, &expandable)
            }

            Event::CollapseAll => expanded_change_plan(ctx, BTreeSet::new(), true, &[]),

            // ── Drag and drop reorder ───────────────────────────────────
            Event::DragStart(key) => {
                // An in-flight drag must be dropped or cancelled before another
                // can start, so a second pickup cannot silently retarget it.
                if ctx.dragging.is_some() {
                    return None;
                }

                // Disabled nodes block all interaction, including drag, and a
                // node hidden under a collapsed parent cannot be picked up from
                // the rendered tree — so it is not a valid drag source either.
                if !props.dnd_enabled
                    || !is_draggable(&ctx.items, key)
                    || !visible_keys(ctx).contains(key)
                {
                    return None;
                }

                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dragging = Some(key);
                    ctx.drop_target = None;
                }))
            }

            Event::DragOver(target) => {
                let dragging = ctx.dragging.as_ref()?;

                if !props.dnd_enabled {
                    return None;
                }

                if !is_valid_drop(&ctx.items, &visible_keys(ctx), dragging, target) {
                    // Hovering an invalid slot (the dragged node, a descendant,
                    // or a row hidden under a collapsed parent) must drop any
                    // stale target so a later `Drop` cannot fire against a slot
                    // the user is no longer indicating. With no target there is
                    // nothing to clear.
                    ctx.drop_target.as_ref()?;
                    return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.drop_target = None;
                    }));
                }

                let target = target.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.drop_target = Some(target);
                }))
            }

            Event::DragMoveNext => drag_step(ctx, props, Direction::Next),

            Event::DragMovePrev => drag_step(ctx, props, Direction::Prev),

            Event::Drop => {
                if !props.dnd_enabled {
                    return None;
                }

                let dragging = ctx.dragging.as_ref()?;

                let target = ctx.drop_target.as_ref()?;

                if !is_valid_drop(&ctx.items, &visible_keys(ctx), dragging, target) {
                    return None;
                }

                let reorder = ReorderEvent {
                    source_path: path_to(&ctx.items, dragging),
                    target_path: path_to(&ctx.items, &target.key),
                    position: target.position,
                };

                Some(
                    TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.dragging = None;
                        ctx.drop_target = None;
                    })
                    .with_effect(reorder_effect(reorder)),
                )
            }

            Event::CancelDrag => {
                if ctx.dragging.is_none() && ctx.drop_target.is_none() {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.dragging = None;
                    ctx.drop_target = None;
                }))
            }

            Event::SyncProps => {
                // Re-derive prop-backed context from the new props: the data
                // source, the controlled bindings, the disabled-key set, and the
                // selection configuration (mode/behavior/multiple).
                let items = props.items.clone();
                let id = props.id.clone();
                let selected = props.selected.clone();
                let expanded = props.expanded.clone();
                let multiple = props.multiple;
                let selection_mode = effective_selection_mode(props);
                let selection_behavior = props.selection_behavior;
                let dnd_enabled = props.dnd_enabled;
                let renamable = props.renamable;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    // Whether the consumer actually changed `Props::items`
                    // (vs. an echo of an unrelated prop like `expanded` or
                    // `selected`/`renamable`). Compare against the last-seen
                    // `items_prop` baseline rather than `ctx.items`, since
                    // `ctx.items` may carry lazily-spliced children that the
                    // consumer's `Props::items` does not — comparing against
                    // `ctx.items` here would falsely report a change on every
                    // unrelated echo and wipe the lazy-loaded subtree.
                    let items_changed = ctx.items_prop != items;
                    // Rebuild generated ids so node ids / aria-activedescendant
                    // track a changed `Props::id`.
                    ctx.ids = ComponentIds::from_id(&id);

                    // Reseed the lazy-load state from the new collection only when
                    // the data source actually changed (e.g. a consumer echoed
                    // freshly-loaded children, so a `NotLoaded` branch is now
                    // `Loaded`). An unrelated prop echo (`selected`/`expanded`/
                    // `renamable`) must preserve `ctx.items` (and any in-flight
                    // `Loading`/`Error` state).
                    if items_changed {
                        ctx.items_prop = items.clone();
                        ctx.items = items;
                        ctx.load_state = seed_load_state(&ctx.items);
                        // A node being renamed that no longer exists, or is now
                        // disabled, cannot keep a live rename target — the rest
                        // of the machine treats disabled nodes as blocking all
                        // interaction, and a removed key has no anchor to render
                        // `NodeRenameInput` against.
                        if let Some(renaming) = ctx.renaming_key.clone() {
                            let stale = ctx
                                .items
                                .get(&renaming)
                                .and_then(|node| node.value.as_ref())
                                .is_none_or(|item| item.disabled);
                            if stale {
                                ctx.renaming_key = None;
                            }
                        }
                    }

                    // Disabling `renamable` while a rename is active cancels
                    // the in-flight edit so adapters do not keep rendering
                    // `NodeRenameInput` against a tree whose props say
                    // renaming is no longer allowed.
                    if !renamable {
                        ctx.renaming_key = None;
                    }

                    ctx.multiple = multiple;
                    ctx.selection_mode = selection_mode;
                    ctx.selected.sync_controlled(selected);
                    ctx.expanded.sync_controlled(expanded);

                    // Recompute disabled keys from the CURRENT `ctx.items` —
                    // not from `props.items` captured above — so an unrelated
                    // `SyncProps` echo that preserves a lazy-loaded subtree
                    // (`items_changed == false`) still respects any disabled
                    // children `ChildrenLoaded` inserted. Computing from
                    // `props.items` would silently drop those from
                    // `selection_state.disabled_keys` on every echo.
                    let disabled_keys = ctx
                        .items
                        .all_nodes()
                        .filter(|node| node.value.as_ref().is_some_and(|item| item.disabled))
                        .map(|node| node.key.clone())
                        .collect::<BTreeSet<Key>>();

                    // Normalize the (uncontrolled) selection for the new mode and
                    // drop keys removed from the collection or now disabled, so a
                    // tightened mode or a data update cannot leave a stale or
                    // non-selectable key selected.
                    let normalized = sanitize_selection(
                        ctx.selected.get().clone(),
                        selection_mode,
                        &ctx.items,
                        &disabled_keys,
                    );

                    ctx.selected.set(normalized);

                    // Rebuild the selection machine; its set is the binding value
                    // sanitized + normalized for the mode (so even a mode-violating
                    // controlled binding renders a valid selection shape).
                    let mut new_state = selection::State::new(selection_mode, selection_behavior)
                        .with_disabled(disabled_keys);

                    new_state.disabled_behavior = selection::DisabledBehavior::FocusOnly;
                    new_state.selected_keys = sanitize_selection(
                        ctx.selected.get().clone(),
                        selection_mode,
                        &ctx.items,
                        &new_state.disabled_keys,
                    );

                    ctx.selection_state = new_state;

                    // Re-validate the in-flight drag against the new props rather
                    // than always cancelling it: a controlled-expanded parent can
                    // echo a new `expanded` prop during a hover-expand drag, and
                    // that must not break the reorder. Discard only when DnD is
                    // off, the collection changed (paths now stale), or the
                    // dragged/target keys are no longer valid.
                    if !dnd_enabled || items_changed {
                        ctx.dragging = None;
                        ctx.drop_target = None;
                    } else if let Some(dragging) = ctx.dragging.clone() {
                        // Items are unchanged here, so the dragged node is still
                        // enabled (same disabled flag as at pickup), but an
                        // `expanded` echo may have collapsed an ancestor and
                        // hidden it. A hidden source cannot be dropped (matching
                        // DragStart, which rejects hidden sources), so cancel the
                        // whole drag; otherwise drop only a now-invalid target.
                        let visible = visible_keys(ctx);

                        if visible.contains(&dragging) {
                            // Source still rendered: drop only a now-invalid target.
                            let stale = ctx.drop_target.as_ref().is_some_and(|t| {
                                !is_valid_drop(&ctx.items, &visible, &dragging, t)
                            });

                            if stale {
                                ctx.drop_target = None;
                            }
                        } else {
                            // Source hidden by the echo: cancel the whole drag.
                            ctx.dragging = None;
                            ctx.drop_target = None;
                        }
                    }

                    // A removed/relocated focused node must not leave a dangling
                    // active descendant.
                    clamp_focus_to_visible(ctx);
                }))
            }

            // ── Lazy loading (spec §5) ──────────────────────────────────
            Event::ChildrenLoaded { parent, children } => {
                // Ignore a load arriving for a node absent from the collection
                // (e.g. removed between request and arrival), so children are
                // never spliced under a dangling parent.
                ctx.items.get(parent)?;

                // Ignore stale deliveries: only accept `ChildrenLoaded` while
                // the parent is in the in-flight `Loading` state. A duplicate
                // delivery (or a late async arrival after the parent was
                // already populated via props or a prior load) would otherwise
                // append the same configs again, leaving duplicated visible
                // rows since `TreeCollection::new` does not dedupe.
                if ctx.load_state.get(parent) != Some(&NodeLoadState::Loading) {
                    return None;
                }

                // Reject payloads that would introduce duplicate keys — either
                // collisions with keys already present in `ctx.items` or
                // duplicates within the loaded subtree itself.
                // `TreeCollection::new` silently accepts duplicates while its
                // key->index map points at only the last occurrence, so a
                // bad lazy-load response would leave focus, selection, and
                // ARIA ids resolving to a different row than the visible
                // duplicate.
                if !loaded_keys_are_unique(&ctx.items, children) {
                    return None;
                }

                let parent = parent.clone();
                let children = children.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.items = insert_loaded_children(&ctx.items, &parent, &children);
                    // Settle the parent and seed load_state for the
                    // freshly-inserted subtree only. A full reseed via
                    // `seed_load_state(&ctx.items)` would clobber other
                    // pending `Loading`/`Error` entries — a real hazard
                    // after `ExpandAll` fires one `LoadChildren` per lazy
                    // branch and the first delivery settles before the
                    // others; the later deliveries would then hit the
                    // `== Loading` guard against a reset-to-`NotLoaded`
                    // entry and be silently dropped.
                    ctx.load_state.insert(parent.clone(), NodeLoadState::Loaded);
                    seed_load_state_for_loaded(&children, &mut ctx.load_state);

                    // Recompute the selection machine's disabled-key set from
                    // the new collection: newly-loaded children carrying
                    // `disabled: true` must be rejected by `select` (the rest
                    // of the machine treats disabled nodes as blocking all
                    // interaction). Also drop any selection entry whose key
                    // is no longer focusable, mirroring `SyncProps`.
                    let disabled_keys = ctx
                        .items
                        .all_nodes()
                        .filter(|node| node.value.as_ref().is_some_and(|item| item.disabled))
                        .map(|node| node.key.clone())
                        .collect::<BTreeSet<Key>>();
                    let sanitized = sanitize_selection(
                        ctx.selected.get().clone(),
                        ctx.selection_mode,
                        &ctx.items,
                        &disabled_keys,
                    );
                    ctx.selected.set(sanitized.clone());
                    ctx.selection_state.disabled_keys = disabled_keys;
                    ctx.selection_state.selected_keys = sanitized;

                    // Honor each loaded `TreeItemConfig::default_expanded`
                    // marker — `TreeCollection::new` records it internally,
                    // but rendering uses `ctx.expanded` (the runtime binding),
                    // so the descendants would render collapsed without an
                    // explicit merge. Controlled `expanded` is parent-owned
                    // and is not touched here.
                    if !ctx.expanded.is_controlled() {
                        let mut expanded = ctx.expanded.get().clone();
                        for child in &children {
                            collect_default_expanded(child, &mut expanded);
                        }
                        ctx.expanded.set(expanded);
                    }
                }))
            }

            Event::LoadError(key) => {
                // Only a tracked node can transition to `Error`.
                ctx.items.get(key)?;

                // Ignore stale failures: only accept `LoadError` while the
                // node is in the in-flight `Loading` state, mirroring the
                // `ChildrenLoaded` guard. Otherwise a late failure arriving
                // after a successful load (or after a retry already
                // resolved) would flip an already-`Loaded` branch back to
                // `Error` and let the retry path request loading again.
                if ctx.load_state.get(key) != Some(&NodeLoadState::Loading) {
                    return None;
                }

                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.load_state.insert(key.clone(), NodeLoadState::Error);
                }))
            }

            // ── Renamable nodes (spec §6) ───────────────────────────────
            Event::RenameStart(key) => {
                // Renaming must be enabled, and a disabled node blocks all
                // interaction including rename.
                if !props.renamable || is_disabled_node(&ctx.items, key) {
                    return None;
                }

                // Only a real, visible node can be renamed, and only from a
                // resting state (the tree is `Idle` or `Focused`). A node
                // hidden under a collapsed ancestor has no rendered
                // `treeitem` or `NodeRenameInput`, so accepting the event
                // would set `focused_node` (and `aria-activedescendant`) to
                // an id absent from the DOM. Mirrors the `FocusNode` /
                // `DragStart` visibility guards.
                if ctx.items.get(key).is_none()
                    || !matches!(state, State::Idle | State::Focused)
                    || !visible_keys(ctx).contains(key)
                {
                    return None;
                }

                let key = key.clone();
                // Committing the previous in-flight rename (if any) is the
                // adapter's job — it owns the live input value the spec commits
                // with. The agnostic core cannot read that DOM value, so it
                // simply retargets `renaming_key`; the adapter commits the
                // outgoing input on blur as focus moves to the new one.
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.renaming_key = Some(key.clone());
                        ctx.focused_node = Some(key);
                    }),
                )
            }

            Event::RenameCommit { key, new_name } => {
                // Reject only when no rename is active anywhere (stray
                // commit). When a rename IS active — either for `key` or
                // (during the retarget hand-off) for a different node — fire
                // `Effect::Rename` so the outgoing edit reaches the consumer.
                // Clearing `renaming_key` is gated on the key actually being
                // the current target, so a retargeted blur commits the
                // outgoing value without ending the new node's rename.
                ctx.renaming_key.as_ref()?;

                let event = RenameEvent {
                    key: key.clone(),
                    new_name: new_name.clone(),
                };
                let is_active_target = ctx.renaming_key.as_ref() == Some(key);

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        if is_active_target {
                            ctx.renaming_key = None;
                        }
                    })
                    .with_effect(rename_effect(event)),
                )
            }

            Event::RenameCancel(key) => {
                if ctx.renaming_key.as_ref() != Some(key) {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.renaming_key = None;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        // Re-sync when the data source, a controlled binding, or any selection
        // configuration field changes — e.g. after a reorder (new `items`) or
        // when switching a tree between single and multiple selection.
        // `renamable` toggling false must also reach `SyncProps` so an active
        // rename on a now-non-renamable tree is cancelled, not left dangling.
        if old.items != new.items
            || old.id != new.id
            || old.selected != new.selected
            || old.expanded != new.expanded
            || old.multiple != new.multiple
            || old.selection_mode != new.selection_mode
            || old.selection_behavior != new.selection_behavior
            || old.dnd_enabled != new.dnd_enabled
            || old.renamable != new.renamable
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Direction used by relative focus and keyboard drag-slot stepping.
#[derive(Clone, Copy)]
enum Direction {
    Next,
    Prev,
}

/// Build a focus transition that moves the indicator to `key` and emits the
/// scroll-into-view intent. `keyboard` carries the focus modality:
/// keyboard-driven navigation sets `focus_visible` so adapters render the
/// keyboard focus ring, while pointer/programmatic focus (`FocusNode`) leaves
/// it cleared so a mouse/touch activation does not show keyboard styling.
fn focus_plan(key: Key, keyboard: bool) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused)
        .apply(move |ctx: &mut Context| {
            ctx.focused_node = Some(key);
            ctx.focus_visible = keyboard;
        })
        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView))
}

/// The visible keys (DFS order) under the current expansion set.
fn visible_keys(ctx: &Context) -> Vec<Key> {
    ctx.items.visible_keys_with_expanded(ctx.expanded.get())
}

/// Resolve the relative focus target. Per the WAI-ARIA tree pattern, Down/Up
/// move to the next/previous visible node and **do not wrap** at the ends
/// (Home/End jump to the boundaries); a step past the last/first node is a
/// no-op.
fn focus_relative(ctx: &Context, direction: Direction) -> Option<TransitionPlan<Machine>> {
    let visible = visible_keys(ctx);

    if visible.is_empty() {
        return None;
    }

    let target = if let Some(current) = &ctx.focused_node
        && let Some(pos) = visible.iter().position(|key| key == current)
    {
        match direction {
            Direction::Next => visible.get(pos + 1),
            Direction::Prev => pos.checked_sub(1).and_then(|prev| visible.get(prev)),
        }
    } else {
        // No prior focus: enter at the near end for the travel direction.
        match direction {
            Direction::Next => visible.first(),
            Direction::Prev => visible.last(),
        }
    };

    target.cloned().map(|key| focus_plan(key, true))
}

/// Advance the shared typeahead matcher by one character. Delegates to the
/// canonical [`typeahead::State`] (multi-character buffer with timeout reset and
/// locale-aware collation/case-folding when the `i18n` feature is enabled),
/// matching against the collection's `text_value`.
///
/// The matcher scans the collection's own visible set, so the collection is
/// first reconciled to the component's live `ctx.expanded` — otherwise typeahead
/// would search the construction-time expansion and skip newly-visible children
/// (or jump to nodes the user has since collapsed).
fn process_typeahead(ctx: &Context, ch: char, now: Duration) -> (typeahead::State, Option<Key>) {
    let items = reconciled_items(ctx);
    ctx.typeahead.process_char_with_locale(
        ch,
        now,
        ctx.focused_node.as_ref(),
        &items,
        &ctx.locale,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    )
}

/// `ctx.items` with its internal expansion reconciled to the live `ctx.expanded`
/// set, so the collection's visible iteration matches what the tree renders.
fn reconciled_items(ctx: &Context) -> TreeCollection<TreeItem> {
    let expanded = ctx.expanded.get();
    let mut items = ctx.items.clone();

    let branch_keys = items
        .all_nodes()
        .filter(|node| node.has_children)
        .map(|node| node.key.clone())
        .collect::<Vec<_>>();

    for key in branch_keys {
        let want = expanded.contains(&key);

        if items.is_expanded(&key) != want {
            items = items.set_expanded(&key, want);
        }
    }

    items
}

/// Resolve the timestamp for a typeahead keypress. Adapters that surface a real
/// clock pass `Some(now)`; otherwise fall back to the host clock (std) or a
/// monotonic bump of the last keypress so the buffer never spuriously resets.
fn typeahead_time(now: Option<Duration>, state: &typeahead::State) -> Duration {
    now.unwrap_or_else(|| {
        current_time()
            .unwrap_or_else(|| state.last_key_time.saturating_add(Duration::from_millis(1)))
    })
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

/// The selection mode actually used by the machine. The documented
/// `Props::multiple(true)` shortcut upgrades the mode to `Multiple` so that
/// multi-selection (not just `aria-multiselectable`) actually works without the
/// caller also having to set `selection_mode(Multiple)`.
const fn effective_selection_mode(props: &Props) -> selection::Mode {
    if props.multiple {
        selection::Mode::Multiple
    } else {
        props.selection_mode
    }
}

/// Commit a new selection state. `selection_state.selected_keys` is the
/// authoritative selection the component renders, kept consistent with the
/// binding (`ctx.selected.get()`) and always normalized for the current mode —
/// so controlled selection cannot diverge from the binding, and a binding that
/// violates the mode (e.g. multiple keys under `Single`) is mode-clamped for
/// rendering without mutating the parent-owned value.
fn apply_selection(ctx: &mut Context, mut new_state: selection::State) {
    ctx.selected.set(new_state.selected_keys.clone());

    new_state.selected_keys = normalize_selection(ctx.selected.get().clone(), ctx.selection_mode);

    ctx.selection_state = new_state;
}

/// Build the transition for a requested selection change, following the
/// controlled/uncontrolled bindable contract (mirrors `checkbox_group`): always
/// notify the parent via [`Effect::SelectionChange`], but only optimistically
/// render (commit the binding + `selection_state`) when **uncontrolled** — a
/// controlled parent owns `Props::selected` and echoes the requested value back,
/// so the rendered selection must not change until that echo arrives. Returns
/// `None` (no transition) when the requested selection equals the current one.
fn selection_change_plan(
    ctx: &Context,
    new_state: selection::State,
) -> Option<TransitionPlan<Machine>> {
    let next = normalize_selection(new_state.selected_keys.clone(), ctx.selection_mode);

    if ctx.selected.get() == &next {
        return None;
    }

    let effect = selection_change_effect(next);

    if ctx.selected.is_controlled() {
        return Some(
            TransitionPlan::new()
                .apply(|_: &mut Context| {})
                .with_effect(effect),
        );
    }

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| apply_selection(ctx, new_state))
            .with_effect(effect),
    )
}

/// Build the transition for a requested expansion change. `clamp` re-clamps
/// focus after a collapse (a now-hidden focused node must not dangle). Same
/// controlled/uncontrolled contract as [`selection_change_plan`]: notify via
/// [`Effect::ExpandedChange`], commit the binding only when uncontrolled.
///
/// Lazy loading (spec §5): `expanding` lists the keys this event is expanding
/// (empty for collapses). Every key in `expanding` whose [`NodeLoadState`] is
/// `NotLoaded` (or `Error`, for retry) triggers a lazy load — each such key is
/// marked `Loading` and a dedicated [`Effect::LoadChildren`] carrying its key
/// is attached, which invokes [`Props::on_load_children`]. The bulk path
/// (`ExpandAll`) therefore fans one load per lazy branch instead of stranding
/// the rest as expanded-but-empty. Loads fire regardless of the
/// controlled/uncontrolled split (the load is needed before any echo can
/// produce children) and even when the expansion set is unchanged (re-expanding
/// a still-`NotLoaded`/`Error` branch retries the load).
fn expanded_change_plan(
    ctx: &Context,
    next: BTreeSet<Key>,
    clamp: bool,
    expanding: &[Key],
) -> Option<TransitionPlan<Machine>> {
    let unchanged = ctx.expanded.get() == &next;

    // Every branch this event expands that still awaits a lazy load.
    let lazy: Vec<Key> = expanding
        .iter()
        .filter(|key| needs_lazy_load(ctx, key))
        .cloned()
        .collect();

    // Nothing to do when neither the expansion set nor a load needs to change.
    if unchanged && lazy.is_empty() {
        return None;
    }

    let effect = expanded_change_effect(next.clone());

    let mut plan = if ctx.expanded.is_controlled() {
        TransitionPlan::new().apply(|_: &mut Context| {})
    } else {
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.expanded.set(next);
            if clamp {
                clamp_focus_to_visible(ctx);
            }
        })
    };

    // Only attach the expanded-change notification when the set actually
    // changed; a pure lazy-load retrigger does not move the expansion set.
    if !unchanged {
        plan = plan.with_effect(effect);
    }

    if !lazy.is_empty() {
        let to_mark = lazy.clone();
        plan = plan.apply(move |ctx: &mut Context| {
            for key in &to_mark {
                ctx.load_state.insert(key.clone(), NodeLoadState::Loading);
            }
        });
        for key in lazy {
            plan = plan.with_effect(load_children_effect(key));
        }
    }

    Some(plan)
}

/// Named effect that invokes [`Props::on_selection_change`] with the requested
/// selection set so a controlled parent can echo it back.
fn selection_change_effect(next: selection::Set) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::SelectionChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_selection_change {
                callback(next.clone());
            }

            no_cleanup()
        },
    )
}

/// Named effect that invokes [`Props::on_expanded_change`] with the requested
/// expanded-key set so a controlled parent can echo it back.
fn expanded_change_effect(next: BTreeSet<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ExpandedChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_expanded_change {
                callback(next.clone());
            }

            no_cleanup()
        },
    )
}

/// Named effect that invokes [`Props::on_load_children`] with the branch `key`
/// whose children must be lazily fetched (spec §5). The app resolves the load
/// and sends [`Event::ChildrenLoaded`] (or [`Event::LoadError`]) back.
fn load_children_effect(key: Key) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::LoadChildren,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_load_children {
                callback(key.clone());
            }

            no_cleanup()
        },
    )
}

/// Named effect that invokes [`Props::on_rename`] with the committed
/// [`RenameEvent`] so the consumer can persist the new label
/// (renamable variant, spec §6).
fn rename_effect(event: RenameEvent) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::Rename,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_rename {
                callback(event.clone());
            }

            no_cleanup()
        },
    )
}

/// The node to activate when the tree first receives focus: the first selected
/// visible node, otherwise the first visible node.
fn initial_active_node(ctx: &Context) -> Option<Key> {
    let visible = visible_keys(ctx);

    let selected = &ctx.selection_state.selected_keys;

    visible
        .iter()
        .find(|&key| selected.contains(key))
        .or_else(|| visible.first())
        .cloned()
}

/// Reduce a selection set to one valid for `mode`: `None` clears it, `Single`
/// keeps at most one key, `Multiple` leaves it unchanged. Applied when the
/// effective selection mode tightens at runtime.
fn normalize_selection(set: selection::Set, mode: selection::Mode) -> selection::Set {
    match mode {
        selection::Mode::None => selection::Set::Empty,

        selection::Mode::Single => {
            if let Some(key) = set.first() {
                selection::Set::Single(key.clone())
            } else {
                selection::Set::Empty
            }
        }

        selection::Mode::Multiple => set,
    }
}

/// After an expansion or data-source change, keep `focused_node` pointing at a
/// rendered (visible) node. If the focused node became hidden under a collapsed
/// ancestor, move focus to its nearest visible ancestor; if it was removed
/// entirely (no visible ancestor), clear it. This prevents `root_attrs` from
/// emitting `aria-activedescendant` for an element adapters no longer render.
fn clamp_focus_to_visible(ctx: &mut Context) {
    let Some(focused) = ctx.focused_node.clone() else {
        return;
    };

    let visible = visible_keys(ctx);
    if visible.contains(&focused) {
        return;
    }

    let mut current = ctx
        .items
        .get(&focused)
        .and_then(|node| node.parent_key.clone());

    while let Some(ancestor) = current {
        if visible.contains(&ancestor) {
            ctx.focused_node = Some(ancestor);

            return;
        }

        current = ctx
            .items
            .get(&ancestor)
            .and_then(|node| node.parent_key.clone());
    }

    ctx.focused_node = None;
}

/// The ordered list of valid keyboard drop slots for the dragged node.
///
/// For every visible node that is neither the dragged node, a descendant of it
/// (which would create a cycle), nor disabled (disabled nodes are never drop
/// targets), three slots are produced in reading order: `Before`, `On`
/// (reparent), then `After`. Keyboard stepping cycles through this list. This
/// mirrors the pointer-path validity in [`is_valid_drop`].
fn valid_drop_slots(ctx: &Context, dragging: &Key) -> Vec<CollectionDropTarget> {
    let mut slots = Vec::new();

    for key in visible_keys(ctx) {
        if &key == dragging
            || is_descendant(&ctx.items, dragging, &key)
            || is_disabled_node(&ctx.items, &key)
        {
            continue;
        }

        slots.extend(
            [DropPosition::Before, DropPosition::On, DropPosition::After].map(|position| {
                CollectionDropTarget {
                    key: key.clone(),
                    position,
                }
            }),
        );
    }

    slots
}

/// Step the keyboard drop target to the next/previous valid slot.
fn drag_step(
    ctx: &Context,
    props: &Props,
    direction: Direction,
) -> Option<TransitionPlan<Machine>> {
    if !props.dnd_enabled {
        return None;
    }

    let dragging = ctx.dragging.as_ref()?;

    let slots = valid_drop_slots(ctx, dragging);

    if slots.is_empty() {
        return None;
    }

    let next = if let Some(pos) = ctx
        .drop_target
        .as_ref()
        .and_then(|current| slots.iter().position(|slot| slot == current))
    {
        match direction {
            Direction::Next => (pos + 1) % slots.len(),
            Direction::Prev => (pos + slots.len() - 1) % slots.len(),
        }
    } else {
        match direction {
            Direction::Next => 0,
            Direction::Prev => slots.len() - 1,
        }
    };

    let target = slots[next].clone();

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.drop_target = Some(target);
    }))
}

/// Whether `candidate` sits below `ancestor` in the tree (a strict descendant;
/// returns `false` when `candidate == ancestor`).
fn is_descendant(items: &TreeCollection<TreeItem>, ancestor: &Key, candidate: &Key) -> bool {
    let mut current = items
        .get(candidate)
        .and_then(|node| node.parent_key.clone());

    while let Some(parent) = current {
        if &parent == ancestor {
            return true;
        }

        current = items.get(&parent).and_then(|node| node.parent_key.clone());
    }

    false
}

/// Whether `key` is a node that expand/collapse may act on: it exists, is an
/// expandable branch (real children or the lazy `has_children` flag), and is
/// not disabled (disabled nodes block all interaction).
fn is_interactive_branch(items: &TreeCollection<TreeItem>, key: &Key) -> bool {
    items.get(key).is_some_and(|node| {
        let item = node.value.as_ref();

        let expandable = node.has_children || item.is_some_and(|item| item.has_children);
        let disabled = item.is_some_and(|item| item.disabled);

        expandable && !disabled
    })
}

/// Seed the per-node [`NodeLoadState`] map from a collection (spec §5.3). A node
/// advertising the lazy [`TreeItem::has_children`] affordance but with no
/// children actually loaded (`Node::has_children == false`) is `NotLoaded`;
/// every other node is `Loaded`. Used at init and reseeded on every
/// [`Event::SyncProps`] so a node that has since loaded children reports
/// `Loaded` against the new collection.
fn seed_load_state(items: &TreeCollection<TreeItem>) -> BTreeMap<Key, NodeLoadState> {
    items
        .all_nodes()
        .map(|node| {
            let lazy_unloaded =
                !node.has_children && node.value.as_ref().is_some_and(|item| item.has_children);

            let state = if lazy_unloaded {
                NodeLoadState::NotLoaded
            } else {
                NodeLoadState::Loaded
            };

            (node.key.clone(), state)
        })
        .collect()
}

/// Whether expanding `key` should trigger a lazy load: the node's tracked
/// [`NodeLoadState`] is `NotLoaded`. Only the first expansion of an unloaded
/// lazy branch loads; a `Loading`/`Error`/`Loaded` node does not re-trigger.
/// A branch needs a lazy load on (re-)expand when it is awaiting its first
/// fetch (`NotLoaded`) **or** when a previous fetch failed (`Error`); the
/// `Error` arm is what lets adapter-driven retry affordances actually retry by
/// re-dispatching `ExpandNode`/`ToggleNode` for the failed branch.
fn needs_lazy_load(ctx: &Context, key: &Key) -> bool {
    matches!(
        ctx.load_state.get(key),
        Some(NodeLoadState::NotLoaded | NodeLoadState::Error)
    )
}

/// Reconstruct the full `TreeItemConfig` forest from the current collection,
/// splicing `children` in as the direct children of `parent`, then rebuild a
/// fresh [`TreeCollection`]. Uses only the public `T: Clone` collection API
/// (`children_of`/`all_nodes`/`is_expanded`), so it does not require
/// Returns `true` when every key in the loaded `children` forest is unique
/// across the forest itself **and** absent from the current `items`
/// collection. `TreeCollection::new` accepts duplicate nodes silently and
/// resolves `get()` to the last occurrence only, so a lazy-load payload
/// with a duplicated key would leave focus / selection / ARIA ids
/// pointing at a different row than the visible duplicate — making it a
/// hard-to-trace data corruption. Used as a [`Event::ChildrenLoaded`]
/// transition-time guard before splicing.
fn loaded_keys_are_unique(
    items: &TreeCollection<TreeItem>,
    children: &[TreeItemConfig<TreeItem>],
) -> bool {
    fn walk(
        config: &TreeItemConfig<TreeItem>,
        items: &TreeCollection<TreeItem>,
        seen: &mut BTreeSet<Key>,
    ) -> bool {
        if items.get(&config.key).is_some() {
            return false;
        }

        if !seen.insert(config.key.clone()) {
            return false;
        }

        for child in &config.children {
            if !walk(child, items, seen) {
                return false;
            }
        }

        true
    }

    let mut seen = BTreeSet::new();

    for root in children {
        if !walk(root, items, &mut seen) {
            return false;
        }
    }

    true
}

/// Seed `load_state` entries for the keys in a freshly-loaded `configs`
/// forest, leaving every other entry in `load_state` untouched. Used by
/// [`Event::ChildrenLoaded`] instead of a full `seed_load_state` reset: a
/// full reset would clobber other in-flight `Loading`/`Error` entries when
/// multiple branches are lazy-loading concurrently (e.g. `ExpandAll` over
/// several lazy branches), and the later deliveries would then be rejected
/// by the `== Loading` guard against an entry that was silently reset to
/// `NotLoaded`.
///
/// A child with the `TreeItem::has_children` affordance but no actual
/// children in the delivered config is seeded `NotLoaded` (it itself is a
/// lazy branch); every other newly-inserted node is `Loaded`.
fn seed_load_state_for_loaded(
    configs: &[TreeItemConfig<TreeItem>],
    load_state: &mut BTreeMap<Key, NodeLoadState>,
) {
    for config in configs {
        let lazy = config.value.has_children && config.children.is_empty();
        let state = if lazy {
            NodeLoadState::NotLoaded
        } else {
            NodeLoadState::Loaded
        };
        load_state.insert(config.key.clone(), state);
        seed_load_state_for_loaded(&config.children, load_state);
    }
}

/// Walk a [`TreeItemConfig`] subtree and accumulate every key whose config
/// asked to start expanded. Used by [`Event::ChildrenLoaded`] to merge a
/// lazy-loaded subtree's `default_expanded` markers into the runtime
/// expansion binding, so descendants render expanded the way the configs
/// requested (mirroring the init-time seeding from
/// [`TreeItemConfig::default_expanded`]).
fn collect_default_expanded(config: &TreeItemConfig<TreeItem>, expanded: &mut BTreeSet<Key>) {
    if config.default_expanded {
        expanded.insert(config.key.clone());
    }

    for child in &config.children {
        collect_default_expanded(child, expanded);
    }
}

/// `T: CollectionItem` (which [`TreeItem`] does not implement) or any private
/// `Node` constructor. The current expansion state of each branch is preserved
/// via `default_expanded`.
fn insert_loaded_children(
    items: &TreeCollection<TreeItem>,
    parent: &Key,
    children: &[TreeItemConfig<TreeItem>],
) -> TreeCollection<TreeItem> {
    /// Rebuild the config subtree rooted at `node_key`, splicing the loaded
    /// `children` under `parent` when reached.
    fn config_for(
        items: &TreeCollection<TreeItem>,
        node_key: &Key,
        parent: &Key,
        children: &[TreeItemConfig<TreeItem>],
    ) -> Option<TreeItemConfig<TreeItem>> {
        let node = items.get(node_key)?;

        let mut child_configs = items
            .children_of(node_key)
            .filter_map(|child| config_for(items, &child.key, parent, children))
            .collect::<Vec<_>>();

        // Splice the freshly-loaded children in under their parent.
        if node_key == parent {
            child_configs.extend(children.iter().cloned());
        }

        Some(TreeItemConfig {
            key: node.key.clone(),
            text_value: node.text_value.clone(),
            value: node.value.clone().unwrap_or_default(),
            children: child_configs,
            default_expanded: items.is_expanded(node_key),
        })
    }

    let roots = items
        .all_nodes()
        .filter(|node| node.parent_key.is_none())
        .filter_map(|node| config_for(items, &node.key, parent, children))
        .collect::<Vec<_>>();

    TreeCollection::new(roots)
}

/// Whether dropping the dragged node at `target` is valid: the target must be a
/// currently **visible** node (adapters can send stale keys for rows hidden
/// under a collapsed parent during pointer hit-testing or virtualization), and
/// never the dragged node itself or any of its descendants (which would create
/// a cycle). `visible` is the live visible-key set (`visible_keys`).
fn is_valid_drop(
    items: &TreeCollection<TreeItem>,
    visible: &[Key],
    dragging: &Key,
    target: &CollectionDropTarget,
) -> bool {
    visible.contains(&target.key)
        && &target.key != dragging
        && !is_disabled_node(items, &target.key)
        && !is_descendant(items, dragging, &target.key)
}

/// Whether `key` names a disabled node (disabled nodes block all interaction, so
/// they are never valid drag sources or drop targets).
fn is_disabled_node(items: &TreeCollection<TreeItem>, key: &Key) -> bool {
    items
        .get(key)
        .and_then(|node| node.value.as_ref())
        .is_some_and(|item| item.disabled)
}

/// Whether `key` is a node the user may pick up to drag: it exists, is
/// focusable, and is not disabled (disabled nodes block all interaction).
fn is_draggable(items: &TreeCollection<TreeItem>, key: &Key) -> bool {
    let node = items.get(key);

    node.is_some_and(Node::is_focusable)
        && !node
            .and_then(|node| node.value.as_ref())
            .is_some_and(|item| item.disabled)
}

/// Whether `key` names a node that can actually be selected: it exists in the
/// collection and is not disabled. Used to keep the rendered selection free of
/// removed or disabled keys.
fn is_selectable_key(
    items: &TreeCollection<TreeItem>,
    disabled: &BTreeSet<Key>,
    key: &Key,
) -> bool {
    items.get(key).is_some() && !disabled.contains(key)
}

/// Drop selection keys that cannot be selected in `items` (missing or disabled
/// nodes), then normalize the remainder for `mode`. Applied at init and on every
/// `SyncProps` so the rendered selection (and the uncontrolled binding) can
/// never report a removed or disabled node as selected.
fn sanitize_selection(
    set: selection::Set,
    mode: selection::Mode,
    items: &TreeCollection<TreeItem>,
    disabled: &BTreeSet<Key>,
) -> selection::Set {
    let retained = match set {
        selection::Set::Single(key) => {
            if is_selectable_key(items, disabled, &key) {
                selection::Set::Single(key)
            } else {
                selection::Set::Empty
            }
        }

        selection::Set::Multiple(keys) => {
            let kept = keys
                .into_iter()
                .filter(|key| is_selectable_key(items, disabled, key))
                .collect::<BTreeSet<Key>>();

            if kept.is_empty() {
                selection::Set::Empty
            } else {
                selection::Set::Multiple(kept)
            }
        }

        // `Set::All` is symbolic ("every item, including unloaded") and its
        // `contains` returns true for *every* key — including disabled or absent
        // ones. Resolve it to the concrete selectable keys present in the
        // collection so it cannot bypass the disabled/missing filtering.
        selection::Set::All => {
            let kept = items
                .all_keys()
                .filter(|key| !disabled.contains(key))
                .cloned()
                .collect::<BTreeSet<Key>>();

            if kept.is_empty() {
                selection::Set::Empty
            } else {
                selection::Set::Multiple(kept)
            }
        }

        // `Empty` and any future `#[non_exhaustive]` variant pass through.
        other => other,
    };

    normalize_selection(retained, mode)
}

/// The path of keys from the root down to `key` (inclusive).
fn path_to(items: &TreeCollection<TreeItem>, key: &Key) -> Vec<Key> {
    let mut path = Vec::new();
    let mut current = Some(key.clone());

    while let Some(node_key) = current {
        current = items
            .get(&node_key)
            .and_then(|node| node.parent_key.clone());

        path.push(node_key);
    }

    path.reverse();

    path
}

/// Build the named reorder effect that invokes [`Props::on_reorder`].
fn reorder_effect(event: ReorderEvent) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::Reorder,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_reorder {
                callback(event.clone());
            }

            no_cleanup()
        },
    )
}

/// The stable DOM id for a tree node, used for `aria-activedescendant`
/// relationships and the node element's `id`.
fn node_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("node", &dom_safe_key_token(key))
}

/// Connect API for the [`TreeView`](self) component.
pub struct Api<'a> {
    /// Current machine state.
    state: &'a State,

    /// Current context.
    ctx: &'a Context,

    /// Current props.
    props: &'a Props,

    /// Event dispatcher.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Whether a node is selected.
    #[must_use]
    pub fn is_node_selected(&self, node_id: &Key) -> bool {
        // Read the mode-normalized selection state (kept consistent with the
        // binding by `apply_selection`/`SyncProps`) so rendering never exposes a
        // selection shape the current mode forbids.
        self.ctx.selection_state.selected_keys.contains(node_id)
    }

    /// Whether a node with the given `disabled` flag can be selected: selection
    /// must be enabled (`selection_mode != None`) and the node must be enabled.
    /// Non-selectable nodes omit `aria-selected` per the WAI-ARIA tree pattern.
    const fn is_node_selectable(&self, disabled: bool) -> bool {
        !disabled && !matches!(self.ctx.selection_mode, selection::Mode::None)
    }

    /// Whether a node is expanded.
    #[must_use]
    pub fn is_node_expanded(&self, node_id: &Key) -> bool {
        self.ctx.expanded.get().contains(node_id)
    }

    /// Whether a node holds the focus indicator.
    #[must_use]
    pub fn is_node_focused(&self, node_id: &Key) -> bool {
        self.ctx.focused_node.as_ref() == Some(node_id)
    }

    /// Whether a node is currently being dragged.
    #[must_use]
    pub fn is_dragging(&self, node_id: &Key) -> bool {
        self.ctx.dragging.as_ref() == Some(node_id)
    }

    /// The current resolved drop target during an active drag, if any.
    #[must_use]
    pub const fn drop_target(&self) -> Option<&CollectionDropTarget> {
        self.ctx.drop_target.as_ref()
    }

    /// Look up a node from the collection by key.
    #[must_use]
    pub fn get_node(&self, node_id: &Key) -> Option<&Node<TreeItem>> {
        self.ctx.items.get(node_id)
    }

    /// Compute `(setsize, posinset)` for a node — the sibling count at its level
    /// and its 1-based position among those siblings.
    #[must_use]
    pub fn sibling_info(&self, node_id: &Key) -> (u32, u32) {
        let Some(node) = self.ctx.items.get(node_id) else {
            return (1, 1);
        };

        let siblings = if let Some(parent) = &node.parent_key {
            self.ctx.items.children_of(parent).collect::<Vec<_>>()
        } else {
            self.ctx
                .items
                .all_nodes()
                .filter(|candidate| candidate.parent_key.is_none())
                .collect::<Vec<_>>()
        };

        let setsize = siblings.len() as u32;

        let posinset = siblings
            .iter()
            .position(|sibling| sibling.key == *node_id)
            .map_or(1, |pos| pos as u32 + 1);

        (setsize, posinset)
    }

    /// The loading indicator text for a branch whose children are being fetched.
    #[must_use]
    pub fn loading_label(&self) -> String {
        (self.ctx.messages.loading_label)(&self.ctx.locale)
    }

    /// The lazy-load status of a node's children (spec §5). Nodes absent from
    /// the tracked map (e.g. an unknown key) report
    /// [`NodeLoadState::Loaded`] — there is nothing pending to load.
    #[must_use]
    pub fn node_load_state(&self, node_id: &Key) -> NodeLoadState {
        self.ctx
            .load_state
            .get(node_id)
            .copied()
            .unwrap_or(NodeLoadState::Loaded)
    }

    /// Whether a node's children are currently being lazily loaded.
    #[must_use]
    pub fn is_loading(&self, node_id: &Key) -> bool {
        self.node_load_state(node_id) == NodeLoadState::Loading
    }

    /// Attributes for the tree root container (`role="tree"`).
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "tree");

        // Advertise multi-selectability whenever multiple selection is actually
        // possible (either the `multiple` shortcut or `selection_mode` Multiple).
        if self.ctx.multiple || self.ctx.selection_mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        if let Some(focused) = &self.ctx.focused_node {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ActiveDescendant),
                node_dom_id(&self.ctx.ids, focused),
            );
        }

        attrs.set(HtmlAttr::TabIndex, "0");

        attrs
    }

    /// Handle focus on the tree root. `is_keyboard` carries the focus modality
    /// the adapter resolved (e.g. tab vs click) so a keyboard tab-in shows the
    /// focus ring while a pointer focus does not.
    pub fn on_root_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Handle blur on the tree root.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Attributes for a branch node (expandable, has children).
    ///
    /// `aria-expanded` is emitted when the node has real children OR its
    /// `has_children` flag is set (lazy-loaded branches that have not fetched
    /// children yet).
    #[must_use]
    pub fn branch_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Branch {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let node = self.ctx.items.get(node_id);
        let is_expanded = self.is_node_expanded(node_id);
        let is_selected = self.is_node_selected(node_id);
        let is_focused = self.is_node_focused(node_id);
        let level = node.map_or(1, |node| node.level as u32 + 1);
        let (setsize, posinset) = self.sibling_info(node_id);
        let item = node.and_then(|node| node.value.as_ref());
        let disabled = item.is_some_and(|item| item.disabled);
        let has_children_flag = item.is_some_and(|item| item.has_children);
        let has_actual_children = node.is_some_and(|node| node.has_children);

        attrs
            .set(HtmlAttr::Id, node_dom_id(&self.ctx.ids, node_id))
            .set(HtmlAttr::Role, "treeitem");

        if has_actual_children || has_children_flag {
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), bool_token(is_expanded));
        }

        // Per the WAI-ARIA tree pattern, only selectable nodes expose
        // `aria-selected`; a non-selectable node (selection disabled, or this
        // node disabled) must not advertise a selection affordance.
        if self.is_node_selectable(disabled) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected));
        }

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Level), level.to_string())
            .set(HtmlAttr::Aria(AriaAttr::SetSize), setsize.to_string())
            .set(HtmlAttr::Aria(AriaAttr::PosInSet), posinset.to_string());

        self.apply_node_state(
            &mut attrs,
            disabled,
            is_selected,
            is_expanded,
            is_focused,
            node_id,
        );

        attrs
    }

    /// Attributes for the clickable control row inside a branch.
    ///
    /// When the node has an `href`, sets `HtmlAttr::Href` so the adapter renders
    /// an `<a>` element (the parent `role="treeitem"` is preserved).
    #[must_use]
    pub fn branch_control_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchControl {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(href) = self.node_href(node_id) {
            attrs.set(HtmlAttr::Href, href);
        }

        attrs
    }

    /// Handle activation of a branch control: toggle expansion and focus.
    pub fn on_branch_control_click(&self, node_id: &Key) {
        (self.send)(Event::ToggleNode(node_id.clone()));
        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Attributes for the expand/collapse chevron indicator inside a branch.
    #[must_use]
    pub fn branch_indicator_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchIndicator {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.is_node_expanded(node_id) {
            attrs.set_bool(HtmlAttr::Data("ars-expanded"), true);
        }

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the text label inside a branch.
    #[must_use]
    pub fn branch_text_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::BranchText)
    }

    /// Attributes for the children container inside an expanded branch
    /// (`role="group"`).
    #[must_use]
    pub fn branch_content_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchContent {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group");

        if !self.is_node_expanded(node_id) {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for a leaf node (no children).
    ///
    /// When the node's `has_children` flag is set (but no children are loaded),
    /// `aria-expanded` is emitted so screen readers announce the affordance.
    /// When the node has an `href`, sets `HtmlAttr::Href`.
    #[must_use]
    pub fn leaf_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Leaf {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let node = self.ctx.items.get(node_id);
        let is_selected = self.is_node_selected(node_id);
        let is_focused = self.is_node_focused(node_id);
        let is_expanded = self.is_node_expanded(node_id);
        let level = node.map_or(1, |node| node.level as u32 + 1);
        let (setsize, posinset) = self.sibling_info(node_id);
        let item = node.and_then(|node| node.value.as_ref());
        let disabled = item.is_some_and(|item| item.disabled);
        let has_children_flag = item.is_some_and(|item| item.has_children);

        attrs
            .set(HtmlAttr::Id, node_dom_id(&self.ctx.ids, node_id))
            .set(HtmlAttr::Role, "treeitem");

        if has_children_flag {
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), bool_token(is_expanded));
        }

        // Only selectable nodes expose `aria-selected` (see `branch_attrs`).
        if self.is_node_selectable(disabled) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected));
        }

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Level), level.to_string())
            .set(HtmlAttr::Aria(AriaAttr::SetSize), setsize.to_string())
            .set(HtmlAttr::Aria(AriaAttr::PosInSet), posinset.to_string());

        self.apply_node_state(
            &mut attrs,
            disabled,
            is_selected,
            is_expanded,
            is_focused,
            node_id,
        );

        if let Some(href) = self.node_href(node_id) {
            attrs.set(HtmlAttr::Href, href);
        }

        attrs
    }

    /// Handle activation of a leaf node: select (or toggle) and focus.
    ///
    /// Under the default `Toggle` selection behavior, clicking an
    /// already-selected leaf **deselects** it (mirroring the keyboard Space path
    /// and the shared collection toggle contract). `Replace` behavior always
    /// (re)selects, since a click replaces the current selection.
    pub fn on_leaf_click(&self, node_id: &Key) {
        if self.is_toggle_behavior() && self.is_node_selected(node_id) {
            (self.send)(Event::DeselectNode(node_id.clone()));
        } else {
            (self.send)(Event::SelectNode(node_id.clone()));
        }

        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Whether the tree uses `Toggle` selection behavior (clicks toggle), as
    /// opposed to `Replace` (clicks replace the selection).
    const fn is_toggle_behavior(&self) -> bool {
        matches!(self.props.selection_behavior, selection::Behavior::Toggle)
    }

    /// Attributes for the text label inside a leaf node.
    #[must_use]
    pub fn leaf_text_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::LeafText)
    }

    /// Whether a node is currently being renamed (spec §6.5). Adapters render
    /// the [`Part::NodeRenameInput`] in place of the node's text label while
    /// this is `true`.
    #[must_use]
    pub fn is_renaming(&self, node_id: &Key) -> bool {
        self.ctx.renaming_key.as_ref() == Some(node_id)
    }

    /// Attributes for the inline rename `<input type="text">` (spec §6.5).
    ///
    /// Rendered only when [`is_renaming`](Self::is_renaming) is `true` for this
    /// node. Pre-filled with the node's current `label`; its `aria-label` comes
    /// from [`Messages::rename_label`] called with that label.
    #[must_use]
    pub fn node_rename_input_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NodeRenameInput {
            node_id: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let label = self
            .get_node(node_id)
            .and_then(|node| node.value.as_ref())
            .map_or("", |item| item.label.as_str());

        attrs
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::Value, label)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.rename_label)(label, &self.ctx.locale),
            );

        attrs
    }

    /// Handle a keydown on the rename input (spec §6.5): `Enter` commits the
    /// rename with `current_value`, `Escape` cancels it.
    pub fn on_rename_input_keydown(&self, node_id: &Key, key_code: &str, current_value: &str) {
        match key_code {
            "Enter" => {
                (self.send)(Event::RenameCommit {
                    key: node_id.clone(),
                    new_name: current_value.to_string(),
                });
            }

            "Escape" => {
                (self.send)(Event::RenameCancel(node_id.clone()));
            }

            _ => {}
        }
    }

    /// Handle blur on the rename input (spec §6.5): commits the rename with
    /// `current_value` when the node is still the active rename target.
    pub fn on_rename_input_blur(&self, node_id: &Key, current_value: &str) {
        // Fire `RenameCommit` whenever any rename is in flight — including
        // the retarget hand-off where the outgoing input's blur fires for
        // `node_id` after `RenameStart` has already moved `renaming_key` to
        // a different node. Without this the user's edit on the outgoing
        // input would be silently dropped. When no rename is active anywhere,
        // blur is a no-op (stray input event).
        if self.ctx.renaming_key.is_some() {
            (self.send)(Event::RenameCommit {
                key: node_id.clone(),
                new_name: current_value.to_string(),
            });
        }
    }

    /// Attributes for a node's drag handle (drag-and-drop surface).
    #[must_use]
    pub fn drag_handle_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DragHandle {
            node_id: Key::default(),
        }
        .data_attrs();

        let item = self
            .ctx
            .items
            .get(node_id)
            .and_then(|node| node.value.as_ref());

        let label = item.map_or("", |item| item.label.as_str());

        // The handle is inert for a disabled node (`DragStart` rejects it) and
        // whenever DnD is off (`on_drag_handle_keydown` no-ops), so a consumer
        // that always renders handles never exposes an operable-looking control
        // that cannot perform its announced action.
        let inert = !self.props.dnd_enabled || item.is_some_and(|item| item.disabled);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.drag_handle_label)(label, &self.ctx.locale),
            );

        if inert {
            // Marked disabled and removed from the tab sequence rather than
            // presenting an operable-looking control.
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set(HtmlAttr::TabIndex, "-1");
        } else {
            // Keyboard users must be able to tab to the handle to start a drag.
            attrs.set(HtmlAttr::TabIndex, "0");

            if self.is_dragging(node_id) {
                attrs.set(HtmlAttr::Aria(AriaAttr::Grabbed), "true");
            }
        }

        attrs
    }

    /// Attributes for the drop indicator at the current drop target.
    #[must_use]
    pub fn drop_indicator_attrs(&self, target: &CollectionDropTarget) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::DropIndicator);

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(
                HtmlAttr::Data("ars-drop-position"),
                target.position.to_string(),
            )
            .set(
                HtmlAttr::Data("ars-drop-target"),
                node_dom_id(&self.ctx.ids, &target.key),
            );

        attrs
    }

    /// Handle a keydown on the tree root, mapping keys to events per the
    /// WAI-ARIA tree pattern. `node_id` is the focused (active-descendant) node.
    ///
    /// Uses a host/fallback clock for typeahead timing; adapters with a real
    /// clock should call [`on_node_keydown_at`](Self::on_node_keydown_at).
    pub fn on_node_keydown(&self, node_id: &Key, data: &KeyboardEventData) {
        self.on_node_keydown_impl(node_id, data, None);
    }

    /// [`on_node_keydown`](Self::on_node_keydown) with an explicit
    /// `now` timestamp for the typeahead timeout (adapter-provided clock).
    pub fn on_node_keydown_at(&self, node_id: &Key, data: &KeyboardEventData, now: Duration) {
        self.on_node_keydown_impl(node_id, data, Some(now));
    }

    fn on_node_keydown_impl(&self, node_id: &Key, data: &KeyboardEventData, now: Option<Duration>) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::FocusNext),

            KeyboardKey::ArrowUp => (self.send)(Event::FocusPrev),

            KeyboardKey::Home => (self.send)(Event::FocusFirst),

            KeyboardKey::End => (self.send)(Event::FocusLast),

            KeyboardKey::ArrowRight => {
                // Branch only. Collapsed -> expand. Expanded with loaded
                // children -> enter the first child (FocusNext). On a leaf, or
                // an expanded lazy branch with no rendered children yet, there
                // is nothing to enter, so the key is inert (WAI-ARIA pattern).
                if self.is_branch(node_id) {
                    if !self.is_node_expanded(node_id) {
                        (self.send)(Event::ExpandNode(node_id.clone()));
                    } else if self.has_loaded_children(node_id) {
                        (self.send)(Event::FocusNext);
                    }
                }
            }

            KeyboardKey::ArrowLeft => {
                // Collapse an expanded branch; otherwise move to the parent.
                if self.is_branch(node_id) && self.is_node_expanded(node_id) {
                    (self.send)(Event::CollapseNode(node_id.clone()));
                } else {
                    (self.send)(Event::FocusParent);
                }
            }

            // Enter selects the focused node; Space toggles its selection
            // (deselects when already selected) per the WAI-ARIA tree contract.
            KeyboardKey::Enter => (self.send)(Event::SelectNode(node_id.clone())),

            KeyboardKey::Space => {
                if self.is_node_selected(node_id) {
                    (self.send)(Event::DeselectNode(node_id.clone()));
                } else {
                    (self.send)(Event::SelectNode(node_id.clone()));
                }
            }

            // F2 starts an inline rename on the focused node (spec §6.6). The
            // machine ignores the request unless `Props::renamable` and the node
            // is enabled, so it is safe to always dispatch.
            KeyboardKey::F2 => (self.send)(Event::RenameStart(node_id.clone())),

            _ => {
                // Ignore character input mid-IME-composition (CJK/accented text):
                // the adapter sends `is_composing = true` with a transient
                // character that must not drive typeahead or `*`-expansion until
                // composition completes.
                if data.is_composing {
                    return;
                }
                if let Some(ch) = data.character {
                    if ch == '*' {
                        self.expand_siblings(node_id);
                    } else if !ch.is_control() {
                        (self.send)(Event::TypeaheadSearch(
                            ch,
                            typeahead_time(now, &self.ctx.typeahead),
                        ));
                    }
                }
            }
        }
    }

    /// Handle keydown on a drag handle (keyboard drag-and-drop protocol).
    pub fn on_drag_handle_keydown(&self, node_id: &Key, data: &KeyboardEventData) {
        if !self.props.dnd_enabled {
            return;
        }

        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => {
                if self.is_dragging(node_id) {
                    (self.send)(Event::Drop);
                } else {
                    (self.send)(Event::DragStart(node_id.clone()));
                }
            }

            KeyboardKey::ArrowDown if self.is_dragging(node_id) => {
                (self.send)(Event::DragMoveNext);
            }

            KeyboardKey::ArrowUp if self.is_dragging(node_id) => {
                (self.send)(Event::DragMovePrev);
            }

            KeyboardKey::Escape if self.is_dragging(node_id) => {
                (self.send)(Event::CancelDrag);
            }

            _ => {}
        }
    }

    /// Move the focus indicator to a node.
    pub fn focus_node(&self, node_id: &Key) {
        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Expand every expandable node in the tree.
    pub fn expand_all(&self) {
        (self.send)(Event::ExpandAll);
    }

    /// Collapse every node in the tree.
    pub fn collapse_all(&self) {
        (self.send)(Event::CollapseAll);
    }

    /// Whether the node renders as an expandable branch (real or lazy children).
    fn is_branch(&self, node_id: &Key) -> bool {
        self.ctx.items.get(node_id).is_some_and(|node| {
            node.has_children || node.value.as_ref().is_some_and(|item| item.has_children)
        })
    }

    /// Whether the node has children actually loaded in the collection (as
    /// opposed to only the lazy `has_children` affordance).
    fn has_loaded_children(&self, node_id: &Key) -> bool {
        self.ctx
            .items
            .get(node_id)
            .is_some_and(|node| node.has_children)
    }

    /// Expand every expandable sibling of `node_id` (the `*` shortcut).
    fn expand_siblings(&self, node_id: &Key) {
        let parent = self
            .ctx
            .items
            .get(node_id)
            .and_then(|node| node.parent_key.clone());

        let siblings = if let Some(parent) = &parent {
            self.ctx.items.children_of(parent).collect::<Vec<_>>()
        } else {
            self.ctx
                .items
                .all_nodes()
                .filter(|node| node.parent_key.is_none())
                .collect::<Vec<_>>()
        };

        for sibling in siblings {
            // Include lazy branches (item-level `has_children` flag), matching
            // `ExpandAll` and `is_branch`.
            let lazy = sibling.value.as_ref().is_some_and(|item| item.has_children);

            if sibling.has_children || lazy {
                (self.send)(Event::ExpandNode(sibling.key.clone()));
            }
        }
    }

    /// The node's navigation href, if any. Suppressed for disabled nodes so a
    /// disabled item is not still actionable via native anchor navigation.
    fn node_href(&self, node_id: &Key) -> Option<&str> {
        let item = self
            .ctx
            .items
            .get(node_id)
            .and_then(|node| node.value.as_ref())?;

        if item.disabled {
            return None;
        }

        item.href.as_deref()
    }

    /// Apply the shared `data-ars-*` state markers used by branch and leaf nodes.
    fn apply_node_state(
        &self,
        attrs: &mut AttrMap,
        disabled: bool,
        is_selected: bool,
        is_expanded: bool,
        is_focused: bool,
        node_id: &Key,
    ) {
        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        if is_expanded {
            attrs.set_bool(HtmlAttr::Data("ars-expanded"), true);
        }

        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        // Lazy loading (spec §5.4): a node whose children are being fetched
        // advertises `aria-busy` and a `data-ars-loading` styling hook so
        // adapters can render the loading affordance / loading label.
        if self.is_loading(node_id) {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Busy), "true")
                .set_bool(HtmlAttr::Data("ars-loading"), true);
        }

        // Disabled nodes are not draggable (`DragStart` rejects them), so do not
        // announce them as draggable.
        if self.props.dnd_enabled && !disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), "draggable");

            if self.is_dragging(node_id) {
                attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
            }
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Branch { ref node_id } => self.branch_attrs(node_id),
            Part::BranchControl { ref node_id } => self.branch_control_attrs(node_id),
            Part::BranchIndicator { ref node_id } => self.branch_indicator_attrs(node_id),
            Part::BranchText => self.branch_text_attrs(),
            Part::BranchContent { ref node_id } => self.branch_content_attrs(node_id),
            Part::Leaf { ref node_id } => self.leaf_attrs(node_id),
            Part::LeafText => self.leaf_text_attrs(),
            Part::NodeRenameInput { ref node_id } => self.node_rename_input_attrs(node_id),
            Part::DragHandle { ref node_id } => self.drag_handle_attrs(node_id),
            Part::DropIndicator => self.ctx.drop_target.as_ref().map_or_else(
                || part_only_attrs(&Part::DropIndicator),
                |target| self.drop_indicator_attrs(target),
            ),
        }
    }
}

/// Render the canonical `"true"`/`"false"` ARIA boolean token.
const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Build an `AttrMap` carrying only a part's scope/part data attributes.
fn part_only_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

#[cfg(test)]
fn snapshot_attrs(attrs: &AttrMap) -> String {
    use core::fmt::Write as _;

    let mut entries = attrs.iter().collect::<Vec<_>>();

    entries.sort_by_key(|(attr, _)| attr.to_string());

    let mut out = String::new();

    for (attr, value) in entries {
        let _ = writeln!(
            &mut out,
            "{}={}",
            attr,
            value.as_str().unwrap_or("<reactive>")
        );
    }

    out
}

#[cfg(test)]
mod tests;
