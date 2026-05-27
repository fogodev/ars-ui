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
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::{
    Collection, Key, Node, TreeCollection,
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

    /// Move focus to the next visible node (wraps to the first).
    FocusNext,

    /// Move focus to the previous visible node (wraps to the last).
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
    /// node. Carries the current time in milliseconds (adapter-provided) so the
    /// shared [`typeahead::State`] can reset the buffer after its timeout.
    TypeaheadSearch(char, u64),

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
}

/// Typed effect intents emitted by the [`TreeView`](self) machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter must scroll the active-descendant node ([`Context::focused_node`])
    /// into view. The agnostic core never moves DOM focus itself; the tree uses
    /// the active-descendant pattern, so only scrolling is adapter-resolved.
    ScrollFocusedIntoView,

    /// Adapter invokes [`Props::on_reorder`] with the completed [`ReorderEvent`].
    Reorder,
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

/// Closure signature backing [`Messages::loading_label`].
pub type LoadingLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Closure signature backing [`Messages::drag_handle_label`]. Receives the
/// node's label and the active locale.
pub type DragHandleLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

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
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading\u{2026}"),
            drag_handle_label: MessageFn::new(|label: &str, _locale: &Locale| {
                alloc::format!("Drag {label}")
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

    /// Called when a drag-and-drop reorder completes.
    pub on_reorder: Option<Callback<dyn Fn(ReorderEvent) + Send + Sync>>,
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
            on_reorder: None,
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

    /// Sets [`on_reorder`](Self::on_reorder).
    #[must_use]
    pub fn on_reorder(
        mut self,
        callback: impl Into<Callback<dyn Fn(ReorderEvent) + Send + Sync>>,
    ) -> Self {
        self.on_reorder = Some(callback.into());
        self
    }
}

/// Runtime context for a [`TreeView`](self) instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The tree collection — nodes, parent/child relationships, visibility.
    pub items: TreeCollection<TreeItem>,

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
        let selected = if let Some(value) = &props.selected {
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

        selection_state.selected_keys = normalize_selection(selected.get().clone(), selection_mode);

        (
            State::Idle,
            Context {
                items: props.items.clone(),
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
            },
        )
    }

    fn transition(
        _state: &State,
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
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut expanded = ctx.expanded.get().clone();

                    expanded.insert(key);

                    ctx.expanded.set(expanded);
                }))
            }

            Event::CollapseNode(key) => {
                if !is_interactive_branch(&ctx.items, key) {
                    return None;
                }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut expanded = ctx.expanded.get().clone();

                    expanded.remove(&key);

                    ctx.expanded.set(expanded);
                    // The focused node may now be hidden under the collapsed
                    // branch; keep `aria-activedescendant` pointing at a
                    // rendered element.
                    clamp_focus_to_visible(ctx);
                }))
            }

            Event::ToggleNode(key) => {
                if !is_interactive_branch(&ctx.items, key) {
                    return None;
                }
                let is_expanded = ctx.expanded.get().contains(key);
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut expanded = ctx.expanded.get().clone();

                    if is_expanded {
                        expanded.remove(&key);
                    } else {
                        expanded.insert(key);
                    }

                    ctx.expanded.set(expanded);
                    clamp_focus_to_visible(ctx);
                }))
            }

            Event::SelectNode(key) => {
                // Ignore the event in non-selectable mode or for a stale/unknown
                // key, so selection never accumulates keys absent from `items`.
                if ctx.selection_mode == selection::Mode::None
                    || !ctx.items.get(key).is_some_and(Node::is_focusable)
                {
                    return None;
                }

                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let new_state = ctx.selection_state.select(key);
                    apply_selection(ctx, new_state);
                }))
            }

            Event::DeselectNode(key) => {
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let new_state = ctx.selection_state.deselect(&key);
                    apply_selection(ctx, new_state);
                }))
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

            Event::TypeaheadSearch(ch, now_ms) => {
                // Delegate to the shared locale-aware, multi-character matcher.
                let (typeahead, found) = process_typeahead(ctx, *ch, *now_ms);
                Some(match found {
                    Some(key) => TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            ctx.typeahead = typeahead;
                            ctx.focused_node = Some(key);
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView)),
                    None => TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.typeahead = typeahead;
                    }),
                })
            }

            Event::ClearTypeahead => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.typeahead = typeahead::State::default();
            })),

            Event::ExpandAll => {
                // Include lazy branches (the item-level `has_children` flag) so
                // bulk expansion can trigger consumer lazy-loading, matching how
                // `branch_attrs`/`leaf_attrs` treat the flag as expandable.
                let expandable = ctx
                    .items
                    .all_nodes()
                    .filter(|node| {
                        node.has_children
                            || node.value.as_ref().is_some_and(|item| item.has_children)
                    })
                    .map(|node| node.key.clone())
                    .collect::<Vec<_>>();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut expanded = ctx.expanded.get().clone();

                    expanded.extend(expandable);

                    ctx.expanded.set(expanded);
                }))
            }

            Event::CollapseAll => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.expanded.set(BTreeSet::new());
                clamp_focus_to_visible(ctx);
            })),

            // ── Drag and drop reorder ───────────────────────────────────
            Event::DragStart(key) => {
                // An in-flight drag must be dropped or cancelled before another
                // can start, so a second pickup cannot silently retarget it.
                if ctx.dragging.is_some() {
                    return None;
                }
                // Disabled nodes block all interaction, including drag.
                let node = ctx.items.get(key);

                let draggable = node.is_some_and(Node::is_focusable)
                    && !node
                        .and_then(|n| n.value.as_ref())
                        .is_some_and(|item| item.disabled);

                if !props.dnd_enabled || !draggable {
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

                if !props.dnd_enabled || !is_valid_drop(&ctx.items, dragging, target) {
                    return None;
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

                if !is_valid_drop(&ctx.items, dragging, target) {
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
                let disabled_keys = items
                    .all_nodes()
                    .filter(|node| node.value.as_ref().is_some_and(|item| item.disabled))
                    .map(|node| node.key.clone())
                    .collect::<BTreeSet<Key>>();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.items = items;
                    // Rebuild generated ids so node ids / aria-activedescendant
                    // track a changed `Props::id`.
                    ctx.ids = ComponentIds::from_id(&id);
                    ctx.multiple = multiple;
                    ctx.selection_mode = selection_mode;
                    ctx.selected.sync_controlled(selected);
                    ctx.expanded.sync_controlled(expanded);

                    // Normalize the (uncontrolled) selection for the new mode so a
                    // tightened mode cannot leave more keys selected than allowed.
                    let normalized =
                        normalize_selection(ctx.selected.get().clone(), selection_mode);
                    ctx.selected.set(normalized);

                    // Rebuild the selection machine; its set is the binding value
                    // normalized for the mode (so even a mode-violating controlled
                    // binding renders a valid selection shape).
                    let mut new_state = selection::State::new(selection_mode, selection_behavior)
                        .with_disabled(disabled_keys);
                    new_state.disabled_behavior = selection::DisabledBehavior::FocusOnly;
                    new_state.selected_keys =
                        normalize_selection(ctx.selected.get().clone(), selection_mode);
                    ctx.selection_state = new_state;

                    // The data source changed: any in-flight drag refers to the
                    // old collection, so discard it rather than letting a later
                    // `Drop` fire a reorder with a stale source/target path.
                    ctx.dragging = None;
                    ctx.drop_target = None;

                    // A removed/relocated focused node must not leave a dangling
                    // active descendant.
                    clamp_focus_to_visible(ctx);
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
        if old.items != new.items
            || old.id != new.id
            || old.selected != new.selected
            || old.expanded != new.expanded
            || old.multiple != new.multiple
            || old.selection_mode != new.selection_mode
            || old.selection_behavior != new.selection_behavior
            || old.dnd_enabled != new.dnd_enabled
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

/// Resolve the relative focus target, wrapping at the ends.
fn focus_relative(ctx: &Context, direction: Direction) -> Option<TransitionPlan<Machine>> {
    let visible = visible_keys(ctx);

    if visible.is_empty() {
        return None;
    }

    let target = if let Some(current) = &ctx.focused_node
        && let Some(pos) = visible.iter().position(|key| key == current)
    {
        match direction {
            Direction::Next => visible.get(pos + 1).or_else(|| visible.first()),
            Direction::Prev => {
                if pos > 0 {
                    visible.get(pos - 1)
                } else {
                    visible.last()
                }
            }
        }
    } else {
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
fn process_typeahead(ctx: &Context, ch: char, now_ms: u64) -> (typeahead::State, Option<Key>) {
    let items = reconciled_items(ctx);
    ctx.typeahead.process_char_with_locale(
        ch,
        now_ms,
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
/// clock pass `Some(now_ms)`; otherwise fall back to the host clock (std) or a
/// monotonic bump of the last keypress so the buffer never spuriously resets.
fn typeahead_time(now_ms: Option<u64>, state: &typeahead::State) -> u64 {
    now_ms.unwrap_or_else(|| current_time_ms().unwrap_or(state.last_key_time_ms.saturating_add(1)))
}

#[cfg(feature = "std")]
fn current_time_ms() -> Option<u64> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(u64::try_from(millis).unwrap_or(u64::MAX))
}

#[cfg(not(feature = "std"))]
const fn current_time_ms() -> Option<u64> {
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
        selection::Mode::Single => match set.first() {
            Some(key) => selection::Set::Single(key.clone()),
            None => selection::Set::Empty,
        },
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
/// For every visible node that is neither the dragged node nor a descendant of
/// it (which would create a cycle), three slots are produced in reading order:
/// `Before`, `On` (reparent), then `After`. Keyboard stepping cycles through
/// this list.
fn valid_drop_slots(ctx: &Context, dragging: &Key) -> Vec<CollectionDropTarget> {
    let mut slots = Vec::new();

    for key in visible_keys(ctx) {
        if &key == dragging || is_descendant(&ctx.items, dragging, &key) {
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

/// Whether `candidate` is `ancestor` itself or sits below it in the tree.
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

/// Whether dropping the dragged node at `target` is valid: the target must be a
/// real node in the collection (adapters can send stale/unknown keys during
/// pointer hit-testing), and never the dragged node itself or any of its
/// descendants (which would create a cycle).
fn is_valid_drop(
    items: &TreeCollection<TreeItem>,
    dragging: &Key,
    target: &CollectionDropTarget,
) -> bool {
    items.get(&target.key).is_some()
        && &target.key != dragging
        && !is_descendant(items, dragging, &target.key)
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

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected))
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

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected))
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

    /// Handle activation of a leaf node: select and focus.
    pub fn on_leaf_click(&self, node_id: &Key) {
        (self.send)(Event::SelectNode(node_id.clone()));
        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Attributes for the text label inside a leaf node.
    #[must_use]
    pub fn leaf_text_attrs(&self) -> AttrMap {
        part_only_attrs(&Part::LeafText)
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
        let disabled = item.is_some_and(|item| item.disabled);
        let label = item.map_or("", |item| item.label.as_str());

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.drag_handle_label)(label, &self.ctx.locale),
            );

        if disabled {
            // A disabled node cannot be dragged (`DragStart` rejects it), so the
            // handle is marked disabled and removed from the tab sequence rather
            // than presenting an operable-looking control.
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
    /// `now_ms` timestamp for the typeahead timeout (adapter-provided clock).
    pub fn on_node_keydown_at(&self, node_id: &Key, data: &KeyboardEventData, now_ms: u64) {
        self.on_node_keydown_impl(node_id, data, Some(now_ms));
    }

    fn on_node_keydown_impl(&self, node_id: &Key, data: &KeyboardEventData, now_ms: Option<u64>) {
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

            _ => {
                if let Some(ch) = data.character {
                    if ch == '*' {
                        self.expand_siblings(node_id);
                    } else if !ch.is_control() {
                        (self.send)(Event::TypeaheadSearch(
                            ch,
                            typeahead_time(now_ms, &self.ctx.typeahead),
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
