---
component: TreeView
category: navigation
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: []
related: []
references:
    ark-ui: TreeView
    react-aria: Tree
---

# TreeView

A hierarchical, keyboard-navigable tree with expand/collapse and optional node selection.
Implements the ARIA tree widget pattern with `role="tree"`, `role="treeitem"`, and
`role="group"` for nested subtrees.

## 1. State Machine

### 1.1 States

| State     | Description                               |
| --------- | ----------------------------------------- |
| `Idle`    | The tree does not have keyboard focus.    |
| `Focused` | The tree has keyboard focus on some node. |

### 1.2 Events

Node identity uses the canonical `Key` enum from `ars_collections` (not `String`),
matching every other component and the `TreeCollection` data model.

| Event                            | Payload         | Description                                                         |
| -------------------------------- | --------------- | ------------------------------------------------------------------- |
| `ExpandNode(Key)`                | node key        | Expand a branch node.                                               |
| `CollapseNode(Key)`              | node key        | Collapse a branch node.                                             |
| `ToggleNode(Key)`                | node key        | Toggle expand/collapse of a branch.                                 |
| `SelectNode(Key)`                | node key        | Add a node to the selection.                                        |
| `DeselectNode(Key)`              | node key        | Remove a node from the selection.                                   |
| `FocusNode(Key)`                 | node key        | Move focus indicator to a node.                                     |
| `FocusNext`                      | —               | Move focus to the next visible node.                                |
| `FocusPrev`                      | —               | Move focus to the previous visible node.                            |
| `FocusFirst`                     | —               | Move focus to the first visible node.                               |
| `FocusLast`                      | —               | Move focus to the last visible node.                                |
| `FocusParent`                    | —               | Move focus to the parent of the focused node.                       |
| `Focus { is_keyboard }`          | `bool`          | The tree container received focus.                                  |
| `Blur`                           | —               | The tree container lost focus.                                      |
| `TypeaheadSearch(char, u64)`     | char + now_ms   | Append a char to the typeahead buffer and jump to the next match.   |
| `ClearTypeahead`                 | —               | Reset the typeahead buffer.                                         |
| `ExpandAll`                      | —               | Expand every expandable node in the tree.                           |
| `CollapseAll`                    | —               | Collapse every node in the tree.                                    |
| `DragStart(Key)`                 | node key        | Begin a drag on a node (drag-and-drop variant, §4).                 |
| `DragOver(CollectionDropTarget)` | resolved target | Set the drop target (adapter pointer hit-testing).                  |
| `DragMoveNext` / `DragMovePrev`  | —               | Step the keyboard drop target through valid slots.                  |
| `Drop`                           | —               | Confirm the drop at the current target (fires `on_reorder`).        |
| `CancelDrag`                     | —               | Cancel the active drag.                                             |
| `SyncProps`                      | —               | Re-sync data source + controlled bindings (via `on_props_changed`). |

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::{
    selection,
    Key, TreeCollection, TreeItemConfig, Collection,
};
use alloc::collections::BTreeSet;

/// Value type for TreeView collection items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeItem {
    /// Display label for the node (used in typeahead and accessibility).
    pub label: String,
    /// Whether this node is disabled. Disabled nodes are not selectable and
    /// cannot be dragged; they remain focusable so their state is announced.
    pub disabled: bool,
    /// When true, the node renders an expand affordance even if no children are
    /// present in the collection yet. Essential for lazy-loaded trees where the
    /// child count is unknown until the node is first expanded.
    pub has_children: bool,
    /// Optional navigation URL. When `Some`, the node renders as an `<a>` element
    /// instead of the default element, enabling native link behavior (middle-click
    /// to open in new tab, etc.). The `role="treeitem"` is preserved on the `<a>`.
    pub href: Option<String>,
}

/// Context for the `TreeView` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The tree collection — holds all nodes, parent-child relationships,
    /// and visible-key computation. See `06-collections.md §2.3`.
    pub items: TreeCollection<TreeItem>,
    /// Currently selected node IDs (uses canonical `selection::Set` from `ars_collections`).
    pub selected: Bindable<selection::Set>,
    /// Full selection state machine (mode, behavior, anchor, focus, disabled keys).
    pub selection_state: selection::State,
    /// Currently expanded branch node IDs (expansion ≠ selection).
    /// Uses `Bindable` for controlled/uncontrolled pattern.
    pub expanded: Bindable<BTreeSet<Key>>,
    /// The node ID that holds the focus indicator.
    pub focused_node: Option<Key>,
    /// True when focus arrived via keyboard.
    pub focus_visible: bool,
    /// Allow multiple nodes to be selected simultaneously.
    pub multiple: bool,
    /// Controls which nodes can be selected (canonical `ars_collections::selection::Mode`).
    pub selection_mode: selection::Mode,
    /// Shared multi-character, locale-aware typeahead state (`ars_collections::typeahead::State`).
    pub typeahead: typeahead::State,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Generated element IDs for all parts.
    pub ids: ComponentIds,
    /// The node currently being dragged, if any (drag-and-drop variant, §4).
    pub dragging: Option<Key>,
    /// The resolved drop target during an active drag, if any.
    pub drop_target: Option<CollectionDropTarget>,
}

// NOTE: The local selection::Mode enum has been removed. TreeView now uses the
// canonical `ars_collections::selection::Mode` (None/Single/Multiple).
//
// NOTE: `init` seeds `selection_state.selected_keys` from the resolved initial
// `selected` binding so the two never diverge on the first select/deselect;
// seeds the initial `expanded` set from the union of `Props::default_expanded`
// and the collection's per-node `TreeItemConfig::default_expanded` branches; and
// seeds `selection_state`'s disabled-key set from the nodes whose `TreeItem`
// is `disabled` so `SelectNode` rejects them (disabled = not selectable).
```

### 1.4 Props

```rust
/// Props for the `TreeView` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// The tree data source. Must be a `TreeCollection<TreeItem>` from `ars_collections`.
    /// Built via `TreeCollection::new(roots)` where `roots: Vec<TreeItemConfig<TreeItem>>`.
    pub items: TreeCollection<TreeItem>,
    /// Controlled selected node IDs (uses canonical `selection::Set` from `ars_collections`).
    pub selected: Option<selection::Set>,
    /// Uncontrolled initial selection.
    pub default_selected: selection::Set,
    /// Controlled expanded node IDs. `Some` = controlled, `None` = uncontrolled.
    pub expanded: Option<BTreeSet<Key>>,
    /// Default expanded node IDs when uncontrolled.
    pub default_expanded: BTreeSet<Key>,
    /// Allow multiple selection.
    pub multiple: bool,
    /// Selection mode (canonical `ars_collections::selection::Mode`).
    pub selection_mode: selection::Mode,
    /// Selection behavior: Toggle (checkbox-like) or Replace (file-explorer-like).
    pub selection_behavior: selection::Behavior,
    /// Enable the drag-and-drop reorder surface (§4).
    pub dnd_enabled: bool,
    /// Called when a drag-and-drop reorder completes (§4). Invoked by the
    /// adapter on the `Effect::Reorder` named effect.
    pub on_reorder: Option<Callback<dyn Fn(ReorderEvent) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: TreeCollection::new(vec![]),
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
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap, no_cleanup};
use ars_collections::{Collection, Node, dnd::{CollectionDropTarget, DropPosition}};

/// States for the `TreeView` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The tree view is in the idle state.
    #[default]
    Idle,
    /// The tree view is in the focused state.
    Focused,
}

/// Events for the `TreeView` component. Node identity is `Key`, not `String`.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Expand a node.
    ExpandNode(Key),
    /// Collapse a node.
    CollapseNode(Key),
    /// Toggle the expand/collapse state of a node.
    ToggleNode(Key),
    /// Select a node.
    SelectNode(Key),
    /// Deselect a node.
    DeselectNode(Key),
    /// Focus a node.
    FocusNode(Key),
    /// Move focus to the next node.
    FocusNext,
    /// Move focus to the previous node.
    FocusPrev,
    /// Move focus to the first node.
    FocusFirst,
    /// Move focus to the last node.
    FocusLast,
    /// Move focus to the parent of the focused node.
    FocusParent,
    /// Focus the tree view.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Blur the tree view.
    Blur,
    /// Append a char to the typeahead buffer and jump to the next match.
    /// `u64` is the adapter-provided current time (ms) for timeout reset.
    TypeaheadSearch(char, u64),
    /// Reset the typeahead buffer.
    ClearTypeahead,
    /// Expand all expandable nodes in the tree.
    ExpandAll,
    /// Collapse all nodes in the tree.
    CollapseAll,
    /// Begin a drag on a node (drag-and-drop variant, §4). Ignored unless
    /// `Props::dnd_enabled`.
    DragStart(Key),
    /// Set the resolved drop target (adapter pointer hit-testing). Ignored when
    /// no drag is active or the target is invalid (self / descendant).
    DragOver(CollectionDropTarget),
    /// Step the keyboard drop target to the next valid slot.
    DragMoveNext,
    /// Step the keyboard drop target to the previous valid slot.
    DragMovePrev,
    /// Confirm the drop at the current drop target (fires `Props::on_reorder`).
    Drop,
    /// Cancel the active drag and discard the drop target.
    CancelDrag,
    /// Re-sync prop-derived context (data source + controlled bindings) after
    /// the consumer supplies new props. Emitted by `on_props_changed`.
    SyncProps,
}

/// Typed effect intents emitted by the `TreeView` machine. The agnostic core
/// never touches the DOM; it emits these intents for adapters to resolve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter scrolls the active-descendant node (`Context::focused_node`)
    /// into view. The tree uses the active-descendant pattern (root keeps
    /// `tabindex="0"` + `aria-activedescendant`), so focus movement only
    /// requires scrolling, not a DOM focus change.
    ScrollFocusedIntoView,
    /// Adapter invokes `Props::on_reorder` with the completed `ReorderEvent`.
    Reorder,
}

/// Machine for the `TreeView` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;
    type Effect = Effect;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let selected = match &props.selected {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_selected.clone()),
        };
        let expanded = match &props.expanded {
            Some(v) => Bindable::controlled(v.clone()),
            None => {
                // Honor branches the collection marks expanded via
                // `TreeItemConfig::default_expanded`, unioned with the
                // component-level `default_expanded` prop (pit of success).
                let mut initial = props.items.all_nodes()
                    .filter(|node| node.is_expanded == Some(true))
                    .map(|node| node.key.clone())
                    .collect::<BTreeSet<Key>>();
                initial.extend(props.default_expanded.iter().cloned());
                Bindable::uncontrolled(initial)
            }
        };
        // Disabled nodes are not selectable: seed the disabled-key set so
        // `selection::State::select` rejects them.
        let disabled_keys = props.items.all_nodes()
            .filter(|n| n.value.as_ref().is_some_and(|v| v.disabled))
            .map(|n| n.key.clone())
            .collect::<BTreeSet<Key>>();
        // `multiple = true` upgrades the effective mode to `Multiple`.
        let selection_mode = effective_selection_mode(props);
        // Seed the selection state machine from the resolved initial selection
        // so it stays consistent with the `selected` binding.
        let mut selection_state = selection::State::new(
            selection_mode,
            props.selection_behavior,
        ).with_disabled(disabled_keys);
        selection_state.selected_keys = normalize_selection(selected.get().clone(), selection_mode);
        let locale = env.locale.clone();
        let messages = messages.clone();
        let ids = ComponentIds::from_id(&props.id);
        (State::Idle, Context {
            items: props.items.clone(),
            selected,
            selection_state,
            expanded,
            focused_node: None,
            focus_visible: false,
            multiple: props.multiple,
            selection_mode,
            typeahead: typeahead::State::default(),
            locale,
            messages,
            ids,
            dragging: None,
            drop_target: None,
        })
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {

            // Expand/collapse/toggle act only on a real, enabled, expandable
            // branch — never a leaf, disabled node, or stale/unknown key.
            Event::ExpandNode(key) => {
                if !is_interactive_branch(&ctx.items, key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    exp.insert(key);
                    ctx.expanded.set(exp);
                }))
            }

            Event::CollapseNode(key) => {
                if !is_interactive_branch(&ctx.items, key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    exp.remove(&key);
                    ctx.expanded.set(exp);
                    // Focused node may now be hidden under the collapsed branch.
                    clamp_focus_to_visible(ctx);
                }))
            }

            Event::ToggleNode(key) => {
                if !is_interactive_branch(&ctx.items, key) { return None; }
                let is_expanded = ctx.expanded.get().contains(key);
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    if is_expanded { exp.remove(&key); } else { exp.insert(key); }
                    ctx.expanded.set(exp);
                    clamp_focus_to_visible(ctx);
                }))
            }

            Event::SelectNode(key) => {
                // Ignore in non-selectable mode or for a stale/unknown key.
                if ctx.selection_mode == selection::Mode::None
                    || !ctx.items.get(key).is_some_and(Node::is_focusable)
                {
                    return None;
                }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.select(key);
                    apply_selection(ctx, new_state);
                }))
            }

            Event::DeselectNode(key) => {
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.deselect(&key);
                    apply_selection(ctx, new_state);
                }))
            }

            // Keyboard navigation arms route through `focus_plan(_, true)` (sets
            // the active descendant + focus-visible + emits
            // `Effect::ScrollFocusedIntoView`). `FocusNode` is pointer/
            // programmatic, so it passes `false` (no keyboard focus ring) and is
            // gated to currently-visible keys so the active descendant never
            // dangles.
            Event::FocusNode(key) => {
                if !visible_keys(ctx).contains(key) { return None; }
                Some(focus_plan(key.clone(), false))
            }

            // On container focus with no active node, initialise to the first
            // selected visible node, else the first visible node.
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                let initial = if ctx.focused_node.is_some() { None } else { initial_active_node(ctx) };
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focus_visible = is_keyboard;
                        if let Some(key) = initial { ctx.focused_node = Some(key); }
                    })
                    .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView)))
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle)
                .apply(|ctx| ctx.focus_visible = false)),

            // ── Navigation (logical semantics; not RTL-swapped, §3.3) ──────
            Event::FocusNext => focus_relative(ctx, Direction::Next),
            Event::FocusPrev => focus_relative(ctx, Direction::Prev),
            Event::FocusFirst => visible_keys(ctx).first().cloned().map(|k| focus_plan(k, true)),
            Event::FocusLast => visible_keys(ctx).last().cloned().map(|k| focus_plan(k, true)),
            Event::FocusParent => ctx.focused_node.as_ref()
                .and_then(|focused| ctx.items.get(focused))
                .and_then(|node| node.parent_key.clone())
                .map(|k| focus_plan(k, true)),

            Event::TypeaheadSearch(ch, now_ms) => {
                let (typeahead, found) = process_typeahead(ctx, *ch, *now_ms);
                Some(match found {
                    Some(key) => TransitionPlan::to(State::Focused)
                        .apply(move |ctx| {
                            ctx.typeahead = typeahead;
                            ctx.focused_node = Some(key);
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView)),
                    None => TransitionPlan::context_only(move |ctx| { ctx.typeahead = typeahead; }),
                })
            }

            Event::ClearTypeahead => Some(TransitionPlan::context_only(|ctx| {
                ctx.typeahead = typeahead::State::default();
            })),

            // ── Expand/Collapse All ────────────────────────────────────
            // ExpandAll uses `all_nodes()` (not the visible-only `nodes()`) so
            // expandable branches hidden under collapsed parents are reached,
            // and includes lazy branches (item-level `has_children` flag) so
            // bulk expansion can trigger consumer lazy-loading.
            Event::ExpandAll => {
                let expandable = ctx.items.all_nodes()
                    .filter(|n| n.has_children
                        || n.value.as_ref().is_some_and(|v| v.has_children))
                    .map(|n| n.key.clone())
                    .collect::<Vec<_>>();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    exp.extend(expandable);
                    ctx.expanded.set(exp);
                }))
            }

            Event::CollapseAll => Some(TransitionPlan::context_only(|ctx| {
                ctx.expanded.set(BTreeSet::new());
                clamp_focus_to_visible(ctx);
            })),

            // ── Drag and drop reorder (§4) ──────────────────────────────
            Event::DragStart(key) => {
                // Disabled nodes block all interaction, including drag.
                let node = ctx.items.get(key);
                let draggable = node.is_some_and(Node::is_focusable)
                    && !node.and_then(|n| n.value.as_ref()).is_some_and(|v| v.disabled);
                if !props.dnd_enabled || !draggable {
                    return None;
                }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
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
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.drop_target = Some(target);
                }))
            }

            Event::DragMoveNext => drag_step(ctx, props, Direction::Next),
            Event::DragMovePrev => drag_step(ctx, props, Direction::Prev),

            Event::Drop => {
                if !props.dnd_enabled { return None; }
                let dragging = ctx.dragging.as_ref()?;
                let target = ctx.drop_target.as_ref()?;
                if !is_valid_drop(&ctx.items, dragging, target) { return None; }
                let reorder = ReorderEvent {
                    source_path: path_to(&ctx.items, dragging),
                    target_path: path_to(&ctx.items, &target.key),
                    position: target.position,
                };
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.dragging = None;
                    ctx.drop_target = None;
                })
                .with_effect(reorder_effect(reorder)))
            }

            Event::CancelDrag => {
                if ctx.dragging.is_none() && ctx.drop_target.is_none() { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.dragging = None;
                    ctx.drop_target = None;
                }))
            }

            Event::SyncProps => {
                // Re-derive prop-backed context from the new props: data source,
                // controlled bindings, disabled keys, and selection config.
                let items = props.items.clone();
                let id = props.id.clone();
                let selected = props.selected.clone();
                let expanded = props.expanded.clone();
                let multiple = props.multiple;
                let selection_mode = effective_selection_mode(props);
                let selection_behavior = props.selection_behavior;
                let disabled_keys = items.all_nodes()
                    .filter(|n| n.value.as_ref().is_some_and(|v| v.disabled))
                    .map(|n| n.key.clone())
                    .collect::<BTreeSet<Key>>();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.items = items;
                    ctx.ids = ComponentIds::from_id(&id); // track a changed Props::id
                    ctx.multiple = multiple;
                    ctx.selection_mode = selection_mode;
                    ctx.selected.sync_controlled(selected);
                    ctx.expanded.sync_controlled(expanded);
                    // Normalize the (uncontrolled) selection for the new mode.
                    let normalized = normalize_selection(ctx.selected.get().clone(), selection_mode);
                    ctx.selected.set(normalized);
                    let mut new_state = selection::State::new(selection_mode, selection_behavior)
                        .with_disabled(disabled_keys);
                    // Normalize even a mode-violating controlled binding for the
                    // rendered selection without mutating the parent value.
                    new_state.selected_keys =
                        normalize_selection(ctx.selected.get().clone(), selection_mode);
                    ctx.selection_state = new_state;
                    // The data source changed: discard any in-flight drag so a
                    // later Drop cannot fire a reorder with a stale path.
                    ctx.dragging = None;
                    ctx.drop_target = None;
                    clamp_focus_to_visible(ctx);
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
        Api { state, ctx, props, send }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        // Re-sync when the data source, a controlled binding, or any selection
        // configuration field changes (e.g. after a reorder, or switching a
        // tree between single and multiple selection).
        if old.items != new.items
            || old.id != new.id
            || old.selected != new.selected
            || old.expanded != new.expanded
            || old.multiple != new.multiple
            || old.selection_mode != new.selection_mode
            || old.selection_behavior != new.selection_behavior
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Direction shared by relative focus and keyboard drop-slot stepping.
#[derive(Clone, Copy)]
enum Direction { Next, Prev }

/// The effective selection mode: `multiple = true` upgrades to `Multiple` so
/// the documented shortcut enables real multi-selection.
const fn effective_selection_mode(props: &Props) -> selection::Mode {
    if props.multiple { selection::Mode::Multiple } else { props.selection_mode }
}

/// Commit a new selection state. `selection_state.selected_keys` is the
/// authoritative rendered selection: consistent with the binding (controlled or
/// not) and normalized for the mode, so controlled selection never diverges and
/// a mode-violating binding is clamped for rendering without mutating it.
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
    visible.iter().find(|&key| selected.contains(key)).or_else(|| visible.first()).cloned()
}

/// Reduce a selection set to one valid for `mode`: `None` clears it, `Single`
/// keeps at most one key, `Multiple` leaves it unchanged.
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

/// Move the focus indicator to `key` and emit the scroll-into-view intent.
/// `keyboard` carries the modality: keyboard navigation sets `focus_visible`
/// (keyboard focus ring); pointer/programmatic focus clears it.
fn focus_plan(key: Key, keyboard: bool) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused)
        .apply(move |ctx| { ctx.focused_node = Some(key); ctx.focus_visible = keyboard; })
        .with_effect(PendingEffect::named(Effect::ScrollFocusedIntoView))
}

/// Keep `focused_node` on a visible node after an expansion/data change: move
/// to the nearest visible ancestor, or clear when the node was removed.
fn clamp_focus_to_visible(ctx: &mut Context) {
    let Some(focused) = ctx.focused_node.clone() else { return; };
    let visible = visible_keys(ctx);
    if visible.contains(&focused) { return; }
    let mut current = ctx.items.get(&focused).and_then(|n| n.parent_key.clone());
    while let Some(ancestor) = current {
        if visible.contains(&ancestor) { ctx.focused_node = Some(ancestor); return; }
        current = ctx.items.get(&ancestor).and_then(|n| n.parent_key.clone());
    }
    ctx.focused_node = None;
}

/// Visible keys (DFS order) under the current expansion set.
fn visible_keys(ctx: &Context) -> Vec<Key> {
    ctx.items.visible_keys_with_expanded(ctx.expanded.get())
}

/// Relative focus target, wrapping at the ends.
fn focus_relative(ctx: &Context, direction: Direction) -> Option<TransitionPlan<Machine>> {
    let visible = visible_keys(ctx);
    if visible.is_empty() { return None; }
    let target = match ctx.focused_node.as_ref()
        .and_then(|current| visible.iter().position(|k| k == current))
    {
        Some(pos) => match direction {
            Direction::Next => visible.get(pos + 1).or_else(|| visible.first()),
            Direction::Prev => if pos > 0 { visible.get(pos - 1) } else { visible.last() },
        },
        None => match direction { Direction::Next => visible.first(), Direction::Prev => visible.last() },
    };
    target.cloned().map(|k| focus_plan(k, true))
}

/// Advance the shared typeahead matcher by one character. Delegates to the
/// canonical `ars_collections::typeahead::State` (multi-character buffer with
/// timeout reset and locale-aware collation/case-folding when the `i18n`
/// feature is enabled), matching the collection's `text_value` and skipping
/// disabled nodes.
fn process_typeahead(ctx: &Context, ch: char, now_ms: u64) -> (typeahead::State, Option<Key>) {
    ctx.typeahead.process_char_with_locale(
        ch,
        now_ms,
        ctx.focused_node.as_ref(),
        &ctx.items,
        &ctx.locale,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    )
}

/// Resolve the timestamp for a typeahead keypress: the adapter-provided value
/// when present, else the host clock (std), else a monotonic bump of the last
/// keypress so the buffer never spuriously resets.
fn typeahead_time(now_ms: Option<u64>, state: &typeahead::State) -> u64 {
    now_ms.unwrap_or_else(|| current_time_ms().unwrap_or(state.last_key_time_ms.saturating_add(1)))
}

/// Ordered valid keyboard drop slots for the dragged node: for every visible
/// node that is neither the dragged node nor a descendant of it, three slots
/// `Before`, `On`, `After`.
fn valid_drop_slots(ctx: &Context, dragging: &Key) -> Vec<CollectionDropTarget> {
    let mut slots = Vec::new();
    for key in visible_keys(ctx) {
        if &key == dragging || is_descendant(&ctx.items, dragging, &key) { continue; }
        for position in [DropPosition::Before, DropPosition::On, DropPosition::After] {
            slots.push(CollectionDropTarget { key: key.clone(), position });
        }
    }
    slots
}

/// Step the keyboard drop target to the next/previous valid slot.
fn drag_step(ctx: &Context, props: &Props, direction: Direction) -> Option<TransitionPlan<Machine>> {
    if !props.dnd_enabled { return None; }
    let dragging = ctx.dragging.as_ref()?;
    let slots = valid_drop_slots(ctx, dragging);
    if slots.is_empty() { return None; }
    let next = match ctx.drop_target.as_ref().and_then(|c| slots.iter().position(|s| s == c)) {
        Some(pos) => match direction {
            Direction::Next => (pos + 1) % slots.len(),
            Direction::Prev => (pos + slots.len() - 1) % slots.len(),
        },
        None => match direction { Direction::Next => 0, Direction::Prev => slots.len() - 1 },
    };
    let target = slots[next].clone();
    Some(TransitionPlan::context_only(move |ctx| ctx.drop_target = Some(target)))
}

/// Whether expand/collapse may act on `key`: it exists, is an expandable branch
/// (real children or the lazy `has_children` flag), and is not disabled.
fn is_interactive_branch(items: &TreeCollection<TreeItem>, key: &Key) -> bool {
    items.get(key).is_some_and(|node| {
        let item = node.value.as_ref();
        let expandable = node.has_children || item.is_some_and(|i| i.has_children);
        expandable && !item.is_some_and(|i| i.disabled)
    })
}

/// Whether `candidate` sits below `ancestor` in the tree.
fn is_descendant(items: &TreeCollection<TreeItem>, ancestor: &Key, candidate: &Key) -> bool {
    let mut current = items.get(candidate).and_then(|n| n.parent_key.clone());
    while let Some(parent) = current {
        if &parent == ancestor { return true; }
        current = items.get(&parent).and_then(|n| n.parent_key.clone());
    }
    false
}

/// A drop is valid when the target is a real node (adapters can send stale or
/// unknown keys during pointer hit-testing) and is not the dragged node or one
/// of its descendants (which would create a cycle).
fn is_valid_drop(items: &TreeCollection<TreeItem>, dragging: &Key, target: &CollectionDropTarget) -> bool {
    items.get(&target.key).is_some()
        && &target.key != dragging
        && !is_descendant(items, dragging, &target.key)
}

/// Path of keys from the root down to `key` (inclusive).
fn path_to(items: &TreeCollection<TreeItem>, key: &Key) -> Vec<Key> {
    let mut path = Vec::new();
    let mut current = Some(key.clone());
    while let Some(node_key) = current {
        current = items.get(&node_key).and_then(|n| n.parent_key.clone());
        path.push(node_key);
    }
    path.reverse();
    path
}

/// Named reorder effect that invokes `Props::on_reorder`.
fn reorder_effect(event: ReorderEvent) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::Reorder, move |_ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_reorder { callback(event.clone()); }
        no_cleanup()
    })
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tree-view"]
pub enum Part {
    Root,
    Branch { node_id: Key },
    BranchControl { node_id: Key },
    BranchIndicator { node_id: Key },
    BranchText,
    BranchContent { node_id: Key },
    Leaf { node_id: Key },
    LeafText,
    /// Optional drag handle affordance for a draggable node (§4).
    DragHandle { node_id: Key },
    /// Visual indicator showing where a dragged node will be dropped (§4).
    DropIndicator,
}

/// API for the `TreeView` component. Node identity is `Key`. Branch/leaf
/// elements carry a stable `id` (via [`node_dom_id`]) so the root's
/// `aria-activedescendant` can reference the focused node.
pub struct Api<'a> {
    state: &'a State,
    ctx:   &'a Context,
    props: &'a Props,
    send:  &'a dyn Fn(Event),
}

impl Api<'_> {
    /// Whether a node is selected. Reads the mode-normalized selection state so
    /// rendering never exposes a selection shape the current mode forbids.
    pub fn is_node_selected(&self, node_id: &Key) -> bool {
        self.ctx.selection_state.selected_keys.contains(node_id)
    }

    /// Whether a node is expanded.
    pub fn is_node_expanded(&self, node_id: &Key) -> bool {
        self.ctx.expanded.get().contains(node_id)
    }

    /// Whether a node holds the focus indicator.
    pub fn is_node_focused(&self, node_id: &Key) -> bool {
        self.ctx.focused_node.as_ref() == Some(node_id)
    }

    /// Whether a node is currently being dragged.
    pub fn is_dragging(&self, node_id: &Key) -> bool {
        self.ctx.dragging.as_ref() == Some(node_id)
    }

    /// The current resolved drop target during an active drag, if any.
    pub const fn drop_target(&self) -> Option<&CollectionDropTarget> {
        self.ctx.drop_target.as_ref()
    }

    /// Look up a node from the collection by key.
    pub fn get_node(&self, node_id: &Key) -> Option<&Node<TreeItem>> {
        self.ctx.items.get(node_id)
    }

    /// Compute `(setsize, posinset)` — sibling count at this level and 1-based
    /// position among siblings.
    pub fn sibling_info(&self, node_id: &Key) -> (u32, u32) {
        let Some(node) = self.ctx.items.get(node_id) else { return (1, 1); };
        let siblings = match &node.parent_key {
            Some(parent) => self.ctx.items.children_of(parent).collect::<Vec<_>>(),
            None => self.ctx.items.all_nodes().filter(|n| n.parent_key.is_none()).collect::<Vec<_>>(),
        };
        let setsize = siblings.len() as u32;
        let posinset = siblings.iter().position(|n| n.key == *node_id).map_or(1, |p| p as u32 + 1);
        (setsize, posinset)
    }

    /// Loading indicator text for a branch whose children are being fetched.
    pub fn loading_label(&self) -> String {
        (self.ctx.messages.loading_label)(&self.ctx.locale)
    }

    /// Attrs for the tree root container (`role="tree"`). Uses the
    /// active-descendant pattern: `tabindex="0"` plus `aria-activedescendant`
    /// pointing at the focused node's stable id.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::Root);
        attrs.set(HtmlAttr::Role, "tree");
        // Advertise multi-selectability whenever multiple selection is possible
        // (the `multiple` shortcut or `selection_mode` Multiple).
        if self.ctx.multiple || self.ctx.selection_mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }
        if let Some(focused) = &self.ctx.focused_node {
            attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), node_dom_id(&self.ctx.ids, focused));
        }
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Handle focus on the tree root.
    pub fn on_root_focus(&self) { (self.send)(Event::Focus { is_keyboard: false }); }

    /// Handle blur on the tree root.
    pub fn on_root_blur(&self) { (self.send)(Event::Blur); }

    /// Attrs for a branch node. `aria-expanded` is emitted when the node has
    /// real children OR its `has_children` flag is set (lazy branches).
    pub fn branch_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::Branch { node_id: Key::default() });
        let node = self.ctx.items.get(node_id);
        let is_expanded = self.is_node_expanded(node_id);
        let is_selected = self.is_node_selected(node_id);
        let is_focused  = self.is_node_focused(node_id);
        let level = node.map_or(1, |n| n.level as u32 + 1);
        let (setsize, posinset) = self.sibling_info(node_id);
        let item = node.and_then(|n| n.value.as_ref());
        let disabled = item.is_some_and(|v| v.disabled);
        let has_children_flag = item.is_some_and(|v| v.has_children);
        let has_actual_children = node.is_some_and(|n| n.has_children);
        attrs.set(HtmlAttr::Id, node_dom_id(&self.ctx.ids, node_id));
        attrs.set(HtmlAttr::Role, "treeitem");
        if has_actual_children || has_children_flag {
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), bool_token(is_expanded));
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected));
        attrs.set(HtmlAttr::Aria(AriaAttr::Level), level.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::SetSize), setsize.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::PosInSet), posinset.to_string());
        self.apply_node_state(&mut attrs, disabled, is_selected, is_expanded, is_focused, node_id);
        attrs
    }

    /// Attrs for the clickable control row inside a branch (`<a>` when href).
    pub fn branch_control_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::BranchControl { node_id: Key::default() });
        if let Some(href) = self.node_href(node_id) { attrs.set(HtmlAttr::Href, href); }
        attrs
    }

    /// Handle activation of a branch control: toggle expansion and focus.
    pub fn on_branch_control_click(&self, node_id: &Key) {
        (self.send)(Event::ToggleNode(node_id.clone()));
        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Attrs for the expand/collapse chevron indicator inside a branch.
    pub fn branch_indicator_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::BranchIndicator { node_id: Key::default() });
        if self.is_node_expanded(node_id) { attrs.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the text label inside a branch.
    pub fn branch_text_attrs(&self) -> AttrMap { part_only_attrs(&Part::BranchText) }

    /// Attrs for the children container inside an expanded branch (`role="group"`).
    pub fn branch_content_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::BranchContent { node_id: Key::default() });
        attrs.set(HtmlAttr::Role, "group");
        if !self.is_node_expanded(node_id) { attrs.set_bool(HtmlAttr::Hidden, true); }
        attrs
    }

    /// Attrs for a leaf node. When `has_children` is set, `aria-expanded` is
    /// emitted so screen readers announce the affordance; `href` renders `<a>`.
    pub fn leaf_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::Leaf { node_id: Key::default() });
        let node = self.ctx.items.get(node_id);
        let is_selected = self.is_node_selected(node_id);
        let is_focused  = self.is_node_focused(node_id);
        let is_expanded = self.is_node_expanded(node_id);
        let level = node.map_or(1, |n| n.level as u32 + 1);
        let (setsize, posinset) = self.sibling_info(node_id);
        let item = node.and_then(|n| n.value.as_ref());
        let disabled = item.is_some_and(|v| v.disabled);
        let has_children_flag = item.is_some_and(|v| v.has_children);
        attrs.set(HtmlAttr::Id, node_dom_id(&self.ctx.ids, node_id));
        attrs.set(HtmlAttr::Role, "treeitem");
        if has_children_flag {
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), bool_token(is_expanded));
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected));
        attrs.set(HtmlAttr::Aria(AriaAttr::Level), level.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::SetSize), setsize.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::PosInSet), posinset.to_string());
        self.apply_node_state(&mut attrs, disabled, is_selected, is_expanded, is_focused, node_id);
        if let Some(href) = self.node_href(node_id) { attrs.set(HtmlAttr::Href, href); }
        attrs
    }

    /// Handle activation of a leaf node: select and focus.
    pub fn on_leaf_click(&self, node_id: &Key) {
        (self.send)(Event::SelectNode(node_id.clone()));
        (self.send)(Event::FocusNode(node_id.clone()));
    }

    /// Attrs for the text label inside a leaf node.
    pub fn leaf_text_attrs(&self) -> AttrMap { part_only_attrs(&Part::LeafText) }

    /// Attrs for a node's drag handle (`role="button"`, localized `aria-label`).
    pub fn drag_handle_attrs(&self, node_id: &Key) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::DragHandle { node_id: Key::default() });
        // `tabindex="0"` so keyboard users can tab to the handle to start a drag.
        attrs.set(HtmlAttr::Role, "button").set(HtmlAttr::TabIndex, "0");
        let label = self.ctx.items.get(node_id).and_then(|n| n.value.as_ref())
            .map_or("", |v| v.label.as_str());
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.drag_handle_label)(label, &self.ctx.locale));
        if self.is_dragging(node_id) { attrs.set(HtmlAttr::Aria(AriaAttr::Grabbed), "true"); }
        attrs
    }

    /// Attrs for the drop indicator at the current drop target.
    pub fn drop_indicator_attrs(&self, target: &CollectionDropTarget) -> AttrMap {
        let mut attrs = part_only_attrs(&Part::DropIndicator);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-drop-position"), target.position.to_string());
        attrs.set(HtmlAttr::Data("ars-drop-target"), node_dom_id(&self.ctx.ids, &target.key));
        attrs
    }

    /// Map a keydown on the tree root (active descendant `node_id`) to events
    /// per the WAI-ARIA tree pattern (§3.2). Logical, not RTL-swapped (§3.3).
    /// Uses a host/fallback clock for typeahead timing; `on_node_keydown_at`
    /// takes an explicit adapter clock.
    pub fn on_node_keydown(&self, node_id: &Key, data: &KeyboardEventData) {
        self.on_node_keydown_impl(node_id, data, None);
    }

    /// `on_node_keydown` with an explicit `now_ms` for typeahead timing.
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
                // Branch only. Collapsed -> expand. Expanded with loaded children
                // -> enter first child. A leaf, or an expanded lazy branch with no
                // rendered children, is inert (WAI-ARIA tree pattern).
                if self.is_branch(node_id) {
                    if !self.is_node_expanded(node_id) {
                        (self.send)(Event::ExpandNode(node_id.clone()));
                    } else if self.has_loaded_children(node_id) {
                        (self.send)(Event::FocusNext);
                    }
                }
            }
            KeyboardKey::ArrowLeft => {
                if self.is_branch(node_id) && self.is_node_expanded(node_id) {
                    (self.send)(Event::CollapseNode(node_id.clone()));
                } else {
                    (self.send)(Event::FocusParent);
                }
            }
            // Enter selects; Space toggles (deselects when already selected).
            KeyboardKey::Enter => (self.send)(Event::SelectNode(node_id.clone())),
            KeyboardKey::Space => if self.is_node_selected(node_id) {
                (self.send)(Event::DeselectNode(node_id.clone()));
            } else {
                (self.send)(Event::SelectNode(node_id.clone()));
            },
            _ => if let Some(ch) = data.character {
                if ch == '*' { self.expand_siblings(node_id); }
                else if !ch.is_control() {
                    (self.send)(Event::TypeaheadSearch(ch, typeahead_time(now_ms, &self.ctx.typeahead)));
                }
            },
        }
    }

    /// Keyboard drag-and-drop protocol on a drag handle: Enter/Space pickup or
    /// confirm, Arrow Up/Down step the target, Escape cancels.
    pub fn on_drag_handle_keydown(&self, node_id: &Key, data: &KeyboardEventData) {
        if !self.props.dnd_enabled { return; }
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => {
                if self.is_dragging(node_id) { (self.send)(Event::Drop); }
                else { (self.send)(Event::DragStart(node_id.clone())); }
            }
            KeyboardKey::ArrowDown if self.is_dragging(node_id) => (self.send)(Event::DragMoveNext),
            KeyboardKey::ArrowUp if self.is_dragging(node_id) => (self.send)(Event::DragMovePrev),
            KeyboardKey::Escape if self.is_dragging(node_id) => (self.send)(Event::CancelDrag),
            _ => {}
        }
    }

    /// Move the focus indicator to a node.
    pub fn focus_node(&self, node_id: &Key) { (self.send)(Event::FocusNode(node_id.clone())); }

    /// Expand all expandable nodes.
    pub fn expand_all(&self) { (self.send)(Event::ExpandAll); }

    /// Collapse all nodes.
    pub fn collapse_all(&self) { (self.send)(Event::CollapseAll); }

    /// Whether the node renders as an expandable branch (real or lazy children).
    fn is_branch(&self, node_id: &Key) -> bool {
        self.ctx.items.get(node_id)
            .is_some_and(|n| n.has_children || n.value.as_ref().is_some_and(|v| v.has_children))
    }

    /// Whether the node has children actually loaded (vs only the lazy flag).
    fn has_loaded_children(&self, node_id: &Key) -> bool {
        self.ctx.items.get(node_id).is_some_and(|n| n.has_children)
    }

    /// Expand every expandable sibling of `node_id` (the `*` shortcut).
    fn expand_siblings(&self, node_id: &Key) {
        let parent = self.ctx.items.get(node_id).and_then(|n| n.parent_key.clone());
        let siblings = match &parent {
            Some(parent) => self.ctx.items.children_of(parent).collect::<Vec<_>>(),
            None => self.ctx.items.all_nodes().filter(|n| n.parent_key.is_none()).collect::<Vec<_>>(),
        };
        for sibling in siblings {
            // Include lazy branches (item-level `has_children` flag).
            let lazy = sibling.value.as_ref().is_some_and(|v| v.has_children);
            if sibling.has_children || lazy { (self.send)(Event::ExpandNode(sibling.key.clone())); }
        }
    }

    /// The node's navigation href, if any. Suppressed for disabled nodes so a
    /// disabled item is not actionable via native anchor navigation.
    fn node_href(&self, node_id: &Key) -> Option<&str> {
        let item = self.ctx.items.get(node_id).and_then(|n| n.value.as_ref())?;
        if item.disabled { return None; }
        item.href.as_deref()
    }

    /// Apply the shared `data-ars-*` / dnd state markers for branch and leaf.
    fn apply_node_state(&self, attrs: &mut AttrMap, disabled: bool, is_selected: bool,
        is_expanded: bool, is_focused: bool, node_id: &Key) {
        if disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if is_selected { attrs.set_bool(HtmlAttr::Data("ars-selected"), true); }
        if is_expanded { attrs.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        if is_focused && self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        // Disabled nodes are not draggable (`DragStart` rejects them).
        if self.props.dnd_enabled && !disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), "draggable");
            if self.is_dragging(node_id) { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        }
    }
}

/// `"true"`/`"false"` ARIA boolean token.
const fn bool_token(value: bool) -> &'static str { if value { "true" } else { "false" } }

/// An `AttrMap` carrying only a part's scope/part data attributes.
fn part_only_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();
    attrs.set(scope_attr, scope_val).set(part_attr, part_val);
    attrs
}

/// The stable DOM id for a tree node, used for `aria-activedescendant` and the
/// node element's `id`.
fn node_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("node", &dom_safe_key_token(key))
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
            Part::DropIndicator => self.ctx.drop_target.as_ref()
                .map_or_else(|| part_only_attrs(&Part::DropIndicator), |t| self.drop_indicator_attrs(t)),
        }
    }
}
```

## 2. Anatomy

```text
TreeView
└── Root                           role="tree" tabindex="0" aria-activedescendant
    ├── Branch                     role="treeitem" id aria-expanded
    │   ├── DragHandle           role="button" (drag-and-drop variant, §4)
    │   ├── BranchControl        (clickable row; <a> when href present)
    │   │   ├── BranchIndicator  aria-hidden="true"
    │   │   └── BranchText
    │   └── BranchContent        role="group"
    │       └── (nested Branch or Leaf nodes)
    ├── Leaf                       role="treeitem" id (aria-expanded when has_children)
    │   └── LeafText             (<a> when href present)
    └── DropIndicator            aria-hidden="true" (drag-and-drop variant, §4)
```

Branch and Leaf carry a stable `id` (so the root's `aria-activedescendant` can
reference the focused node — the tree uses the active-descendant pattern). The
`DragHandle` and `DropIndicator` parts are present only in the drag-and-drop
reorder variant (§4).

| Part              | Element                               | Key Attributes                                                                                                                                                                                                                                                                                       |
| ----------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`            | `<ul>` or `<div>`                     | `data-ars-scope="tree-view"`, `data-ars-part="root"`, `role="tree"`, `tabindex="0"`, `aria-multiselectable` (when `multiple`), `aria-activedescendant` (when focused)                                                                                                                                |
| `Branch`          | `<li>` or `<div>`                     | `id`, `role="treeitem"`, `aria-expanded` (also set when `has_children` is true with no loaded children), `aria-selected`, `aria-level`, `aria-setsize`, `aria-posinset`, `data-ars-expanded`, `data-ars-selected`, `data-ars-focus-visible`, `aria-roledescription="draggable"` (when `dnd_enabled`) |
| `BranchControl`   | `<div>` or `<a>` (when href)          | `data-ars-scope="tree-view"`, `data-ars-part="branch-control"`, `href` (when present)                                                                                                                                                                                                                |
| `BranchIndicator` | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="branch-indicator"`, `aria-hidden="true"`, `data-ars-expanded`                                                                                                                                                                                          |
| `BranchText`      | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="branch-text"`                                                                                                                                                                                                                                          |
| `BranchContent`   | `<ul>` or `<div>`                     | `role="group"`, `data-ars-scope="tree-view"`, `data-ars-part="branch-content"`, `hidden` (when collapsed)                                                                                                                                                                                            |
| `Leaf`            | `<li>`, `<div>`, or `<a>` (when href) | `id`, `role="treeitem"`, `aria-expanded="false"` (when `has_children` is true), `aria-selected`, `aria-level`, `aria-setsize`, `aria-posinset`, `data-ars-selected`, `href` (when present)                                                                                                           |
| `LeafText`        | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="leaf-text"`                                                                                                                                                                                                                                            |
| `DragHandle`      | `<button>`                            | `data-ars-part="drag-handle"`, `role="button"`, `tabindex="0"`, `aria-label` (from `Messages::drag_handle_label`), `aria-grabbed` (while dragging). Drag-and-drop variant (§4).                                                                                                                      |
| `DropIndicator`   | `<div>`                               | `data-ars-part="drop-indicator"`, `aria-hidden="true"`, `data-ars-drop-position`, `data-ars-drop-target`. Drag-and-drop variant (§4).                                                                                                                                                                |

## 3. Accessibility

- While loading, the expandable node shows `aria-busy="true"`.
- A visually hidden "Loading" text (from Messages) is announced via `aria-live="polite"`.
- On error, the node shows an error state and retry affordance with appropriate `aria-label`.

### 3.1 ARIA Roles, States, and Properties

| Part            | Role       | Properties                                                                                                                                                                |
| --------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`          | `tree`     | `aria-multiselectable="true"` when `multiple=true`                                                                                                                        |
| `Branch`        | `treeitem` | `aria-expanded="true\|false"` (emitted when node has children or `has_children` is true), `aria-selected`, `aria-disabled`, `aria-level`, `aria-setsize`, `aria-posinset` |
| `BranchContent` | `group`    | —                                                                                                                                                                         |
| `Leaf`          | `treeitem` | `aria-selected`, `aria-disabled`, `aria-level`, `aria-setsize`, `aria-posinset`. When `has_children` is true: also `aria-expanded="false"`                                |

When a tree item has an `href`, the adapter renders the clickable area (`BranchControl` or
`Leaf`) as an `<a>` element. The `role="treeitem"` on the parent `Branch`/`Leaf` container is
preserved — the `<a>` is a descendant of the `role="treeitem"` element, not a replacement for
it. This allows native link semantics (middle-click, Ctrl+click) while maintaining the ARIA tree
pattern.

`aria-level` reflects DOM nesting depth (1-based). `aria-setsize` and `aria-posinset` enable
screen readers to announce "item 3 of 5" when navigating.

**Parent-child ARIA relationships:** The DOM hierarchy is sufficient to establish parent-child
relationships — `aria-owns` is NOT used because children are rendered as direct DOM descendants
of their parent treeitem. Branch children are wrapped in a `BranchContent` element with
`role="group"` (see anatomy table above), which groups the child treeitems under the parent
branch. This `role="group"` container is what allows screen readers to determine nesting
without `aria-owns`. If an adapter renders children outside the parent's DOM subtree (e.g., via
portals), it MUST add `aria-owns` on the parent `treeitem` referencing the `role="group"`
container's ID.

### 3.2 Keyboard Interaction

| Key                 | Behavior                                                                                                                                  |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `ArrowDown`         | Move focus to the next visible node (skips hidden children of collapsed branches).                                                        |
| `ArrowUp`           | Move focus to the previous visible node.                                                                                                  |
| `ArrowRight`        | If focused node is a collapsed branch: expand it. If an expanded branch: move focus to first child. On a leaf: inert (no child to enter). |
| `ArrowLeft`         | If focused node is an expanded branch: collapse it. If collapsed (or a leaf): move focus to parent.                                       |
| `Home`              | Move focus to the first node in the tree.                                                                                                 |
| `End`               | Move focus to the last visible node in the tree.                                                                                          |
| `Enter`             | Select the focused node (if `selection_mode != None`).                                                                                    |
| `Space`             | Toggle selection of the focused node — deselects it when already selected (multiple mode).                                                |
| `*` (asterisk)      | Expand all siblings of the focused node.                                                                                                  |
| `F2`                | Start inline rename on the focused node (when `renamable` is true). See [section 6](#6-variant-renamable-nodes).                          |
| Printable character | Typeahead: jump to the next node whose visible label starts with the typed character.                                                     |

### 3.3 RTL Navigation Semantics

`TreeView` keyboard navigation uses **logical** (not visual) semantics that remain consistent regardless of text direction. In both LTR and RTL layouts:

- **ArrowRight / ArrowDown** = expand a collapsed branch, or navigate to the first child of an expanded branch.
- **ArrowLeft / ArrowUp** = collapse an expanded branch, or navigate to the parent node.

This matches the WAI-ARIA `TreeView` pattern, where ArrowRight/ArrowLeft map to expand/collapse (parent-child hierarchy), not to visual direction. The adapter does **not** swap ArrowLeft/ArrowRight in RTL mode for `TreeView`, unlike flat list components (`Accordion`, `Tabs`) where arrows map to next/previous sibling. The rationale is that tree hierarchy is a logical relationship independent of layout direction.

### 3.4 Selection Announcements

- Selected items have `aria-selected="true"` (already present)
- Selection changes are announced via focus movement to the selected node
- For programmatic selection changes (no focus change), use LiveAnnouncer:
  `announce("{item label} selected", "polite")`

### 3.5 Scroll Into View

The tree uses the active-descendant pattern: DOM focus stays on the `Root`
(`tabindex="0"`), and `aria-activedescendant` points at the focused node's id.
When focus moves, the agnostic core emits the `Effect::ScrollFocusedIntoView`
intent (on every focus-moving transition — `FocusNode`, `FocusNext`/`Prev`/
`First`/`Last`/`Parent`, and `TypeaheadSearch`); it never moves DOM focus or
scrolls itself.

The adapter resolves that intent: it scrolls the active-descendant element into view using `element.scrollIntoView({ block: 'nearest', inline: 'nearest' })`. For nested scrollable containers, the adapter must walk up the DOM and call `scrollIntoView` on the closest scrollable ancestor only. On iOS Safari, where programmatic focus doesn't trigger scroll, the adapter must explicitly call `scrollIntoView`. Smooth scrolling is avoided for keyboard navigation to prevent timing conflicts.

### 3.6 Expand/Collapse Keyboard Navigation Race Conditions

Rapid keyboard interactions on `TreeView` nodes can cause race conditions between expand/collapse animations and focus movement:

1. **Debounce Expansion**: Expand/collapse transitions are not debounced — each ArrowRight (expand) or ArrowLeft (collapse) takes effect immediately. However, the focus movement that follows expansion (ArrowRight on expanded node moves focus to first child) waits until the children are actually rendered.
2. **Pending Expansion Guard**: If ArrowRight is pressed on a collapsed node, the node enters an "expanding" state. A second ArrowRight during this state is queued and executed after children render (moving focus to the first child), rather than being ignored.
3. **Collapse During Focus-In-Child**: If ArrowLeft collapses a parent while focus is on a child, focus moves to the parent node before the collapse animation starts. The adapter must move focus synchronously, then animate the collapse.
4. **Async Children**: For lazily-loaded children (`on_expand` triggers async fetch), the node shows a loading indicator and expansion completes when children arrive. ArrowRight during loading is a no-op.

## 4. Internationalization

- **RTL**: Arrow key meanings are not reversed for trees (tree nesting is not direction-dependent).
  The visual chevron indicator flips via CSS using `[dir="rtl"]`.
- **Typeahead**: Character matching is case-insensitive and uses the locale's collation when
  available (delegated to `ars-i18n`).
- **Label text**: All node labels are consumer-provided. The machine emits a `loading_label` for lazy-loaded branches (see §4.1 Messages).

### 4.1 Messages

```rust
/// Locale-specific labels for the TreeView component.
///
/// Derives `PartialEq` because `Context` derives `PartialEq` and holds a
/// `Messages` (the framework requires `Context: PartialEq`).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Loading indicator text for branches whose children are being fetched
    /// (default: "Loading…"). Rendered as visually hidden text and announced
    /// via `aria-live="polite"`.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label template for a node's drag handle, called with the
    /// node's label (default: `"Drag {label}"`). Used by `drag_handle_attrs`.
    pub drag_handle_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading\u{2026}"),
            drag_handle_label: MessageFn::new(|label: &str, _locale: &Locale| {
                format!("Drag {label}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, TreeView surfaces the agnostic drag-and-drop reorder logic — the `DragHandle` and `DropIndicator` anatomy parts (canonical bare names, per `06-collections.md` §10.8 and the listbox/gridlist/table specs), the drag-state events, valid drop-slot enumeration, cycle-free drop validation, and `ReorderEvent` path computation. It reports completion through `Props::on_reorder` driven by the named `Effect::Reorder`. Dropping **between** siblings uses `DropPosition::Before`/`After`; dropping **onto** a node to reparent uses `DropPosition::On`. The collection-level `DraggableCollection<T>` / `DroppableCollection<T>` trait impls (which bundle the live `selection::State`), pointer hit-testing, the 500 ms hover-expand timer, and `use_drag`/`use_drop` DOM wiring are **adapter** concerns (tracked by the TreeView Leptos/Dioxus adapter tasks), not part of this agnostic core.

### 4.2 Drag-Reorder Behavior

When drag-and-drop reorder is enabled (`dnd_enabled: true`), the following behaviors apply. The
**agnostic core** owns the data-model logic (slot enumeration, validity, paths, attrs, keyboard
protocol); pointer hit-testing and the hover-expand timer are **adapter**-resolved.

- **Drag handle**: Each tree item renders an optional drag handle affordance (the `DragHandle`
  anatomy part). The handle is the grab target; the entire row is not draggable by default to
  avoid conflicts with text selection. `drag_handle_attrs` emits `role="button"` and a localized
  `aria-label` from `Messages::drag_handle_label`, plus `aria-grabbed="true"` while dragging.
- **Hover-expand** (adapter): When the pointer dwells over a collapsed node during a drag, the
  adapter expands it after a 500 ms hover delay by sending `ExpandNode`. The agnostic core has no
  timer.
- **Drop target indication**: The `DropIndicator` part marks the current drop slot with
  `data-ars-drop-position` (`before` / `on` / `after`) and `data-ars-drop-target` (the target
  node's id). Three drop positions are supported:
  - `DropPosition::Before` — insert before the target node (horizontal line above)
  - `DropPosition::After` — insert after the target node (horizontal line below)
  - `DropPosition::On` — reparent as a child of the target node (target background highlight)
- **Drop position detection** (adapter): The adapter maps the pointer's vertical position within
  the target row — top 25% = `Before`, bottom 25% = `After`, middle 50% = `On` — to a
  `CollectionDropTarget` and sends `DragOver`.
- **Keyboard drag** (agnostic core, via `on_drag_handle_keydown`):
  - `Enter` / `Space` on a focused drag handle picks up the item (`DragStart`).
  - Arrow Up/Down step the item through the valid drop slots (`DragMovePrev` / `DragMoveNext`).
  - `Enter` confirms the drop (`Drop`).
  - `Escape` cancels and discards the drop target (`CancelDrag`).
- **Drop validity**: The core rejects dropping a node onto itself or any of its descendants
  (cycle prevention) in both `DragOver` and `Drop`.
- **Reorder callback**: `Props::on_reorder: Option<Callback<ReorderEvent>>` fires when a drop is
  completed. The core never mutates `ctx.items`; the consumer applies the reorder to its data
  source and re-supplies props (pure-notification, like Tabs).

```rust
/// Event emitted when a tree node is reordered via drag-and-drop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderEvent {
    /// Path of the dragged node (sequence of keys from root to node).
    pub source_path: Vec<Key>,
    /// Path of the drop target.
    pub target_path: Vec<Key>,
    /// Where relative to the target the node was dropped.
    pub position: DropPosition,
}

// `DropPosition` and `CollectionDropTarget` — defined in `ars_collections::dnd`
// (`06-collections.md` §10). Reconciliation: `CollectionDndEvent::Reorder`/`Move`
// is the low-level *collection* contract (flat `keys` + a single `target`);
// `ReorderEvent` is the TreeView *component* surface (hierarchical root→node
// paths). The adapter maps between them.
```

## 5. Variant: Lazy Loading

`TreeView` supports lazy loading of child nodes for large or server-driven hierarchies. When a node is first expanded and its children are not yet loaded, the machine fires a `LoadChildren` event.

### 5.1 Additional Props

```rust,no_check
/// The load_children callback is registered in the adapter layer, not in Props.
/// When provided, nodes without pre-loaded children will trigger lazy loading on expand.
/// The adapter observes `LoadChildren(Key)` events and calls the user-provided callback.
```

```rust
/// Type alias for the lazy-load callback.
/// The adapter calls this with the parent node's key when children need loading.
/// The callback should eventually send a `ChildrenLoaded` event back to the machine.
/// Uses `Rc` so the adapter can clone it into event handler closures.
pub type LoadChildrenFn = Rc<dyn Fn(Key)>;
```

### 5.2 Additional Events

```rust,no_check
/// Added to TreeView Event enum.
/// Fired when a node is expanded for the first time and has no children loaded.
LoadChildren(Key),
/// Fired by the consumer (via adapter callback) when lazy-loaded children arrive.
ChildrenLoaded {
    /// The parent node key.
    parent: Key,
    /// The loaded children nodes.
    children: Vec<Node<T>>,
},
```

### 5.3 Additional Node State

```rust
/// Extended NodeState to include loading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeLoadState {
    /// Children are present in the collection.
    Loaded,
    /// Children are being fetched (loading indicator shown).
    Loading,
    /// Load failed — adapter may show retry affordance.
    Error,
    /// Children have not been requested yet (initial state for lazy nodes).
    NotLoaded,
}
```

The `tree_view::Context` tracks `load_state: BTreeMap<Key, NodeLoadState>`.

### 5.4 Behavior

1. User expands a node whose `load_state` is `NotLoaded`.
2. Machine transitions `load_state` to `Loading` and fires `LoadChildren(key)`.
3. Adapter calls `load_children(key)` prop. The consumer fetches data asynchronously.
4. Consumer sends `ChildrenLoaded { parent, children }` when data arrives.
5. Machine inserts children into the collection and transitions `load_state` to `Loaded`.
6. If loading fails, consumer sends `LoadError(key)` and machine transitions to `Error`.

## 6. Variant: Renamable Nodes

`TreeView` supports inline rename of tree nodes, similar to file-explorer rename behavior.
When enabled, users can start renaming a node via F2 or slow double-click. The node's text
display is replaced by an inline `<input>` element until the rename is committed or cancelled.

### 6.1 Additional Props

```rust,no_check
/// Additional props for the renamable variant.
impl Props {
    /// When true, tree items can be renamed inline via F2 or slow double-click.
    /// Default: `false`.
    pub renamable: bool,
}

// In Props::default():
// renamable: false,
```

### 6.2 Additional Events

```rust,no_check
/// Added to TreeView Event enum for the renamable variant.

/// User initiates rename on a node (e.g., F2 or slow double-click).
RenameStart(Key),
/// User commits the new name.
RenameCommit { key: Key, new_name: String },
/// User cancels rename (Escape or blur without change).
RenameCancel(Key),
```

### 6.3 Additional Context

```rust,no_check
/// Added to TreeView Context for the renamable variant.
impl Context {
    /// The node currently being renamed, if any. When `Some(key)`, the node
    /// identified by `key` renders a `NodeRenameInput` instead of its text label.
    pub renaming_key: Option<Key>,
}

// In Machine::init():
// renaming_key: None,
```

### 6.4 Behavior

- **`RenameStart(key)`**: Only allowed from `Idle` or `Focused` states. If `renamable` is
  false or the target node is disabled, the event is ignored. If another node is currently
  being renamed (`renaming_key` is `Some(other_key)`), that rename is committed first (fires
  `RenameCommit` for the previous node with its current input value), then `renaming_key` is
  set to `Some(key)`. Focus moves to the rename input.
- **`RenameCommit { key, new_name }`**: Clears `renaming_key` to `None`. The consumer is
  responsible for persisting the new name (e.g., updating the `TreeCollection` data source).
  The machine does not modify the `TreeItem.label` — the consumer updates the collection and
  passes new props.
- **`RenameCancel(key)`**: Clears `renaming_key` to `None`. No value change occurs. Focus
  returns to the tree item that was being renamed.

```rust,no_check
/// Transition logic for rename events.
/// Added to the Machine::transition match block.

Event::RenameStart(key) => {
    if !_props.renamable { return None; }
    let node_disabled = ctx.items.get(key)
        .and_then(|n| n.value.as_ref())
        .map(|v| v.disabled)
        .unwrap_or(false);
    if node_disabled { return None; }
    match state {
        State::Idle | State::Focused => {
            let key = key.clone();
            Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                ctx.renaming_key = Some(key.clone());
                ctx.focused_node = Some(key);
            }))
        }
        _ => None,
    }
}

Event::RenameCommit { key, .. } => {
    if ctx.renaming_key.as_ref() != Some(key) { return None; }
    Some(TransitionPlan::context_only(|ctx| {
        ctx.renaming_key = None;
    }))
}

Event::RenameCancel(key) => {
    if ctx.renaming_key.as_ref() != Some(key) { return None; }
    Some(TransitionPlan::context_only(|ctx| {
        ctx.renaming_key = None;
    }))
}
```

### 6.5 Anatomy Additions

```text
TreeView (renamable variant)
└── Root
    ├── Branch
    │   ├── BranchControl
    │   │   ├── BranchIndicator
    │   │   ├── BranchText           (hidden when renaming_key matches)
    │   │   └── NodeRenameInput(key)   (shown when renaming_key matches)
    │   └── BranchContent
    └── Leaf
        ├── LeafText                 (hidden when renaming_key matches)
        └── NodeRenameInput(key)       (shown when renaming_key matches)
```

The `NodeRenameInput(Key)` part is an `<input type="text">` element rendered inside the
branch control or leaf when `renaming_key == Some(key)`. It replaces the text display during
rename and is pre-filled with the node's current `label` value.

```rust,no_check
/// Added to Part enum (node identity is `Key`, consistent with the core).
NodeRenameInput { node_id: Key },
```

```rust
/// Attrs for the inline rename input.
/// Rendered only when `renaming_key == Some(key)` for this node.
pub fn node_rename_input_attrs(&self, node_id: &Key) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] =
        Part::NodeRenameInput { node_id: Key::default() }.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    let label = self.get_node(node_id)
        .and_then(|n| n.value.as_ref())
        .map(|v| v.label.as_str())
        .unwrap_or("");
    attrs.set(HtmlAttr::Type, "text");
    attrs.set(HtmlAttr::Value, label);
    attrs.set(
        HtmlAttr::Aria(AriaAttr::Label),
        (self.ctx.messages.rename_label)(label, &self.ctx.locale),
    );
    attrs
}

/// Handle keydown events on the rename input.
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

/// Handle blur on the rename input — commits the rename if still active.
pub fn on_rename_input_blur(&self, node_id: &Key, current_value: &str) {
    if self.ctx.renaming_key.as_ref() == Some(node_id) {
        (self.send)(Event::RenameCommit {
            key: node_id.clone(),
            new_name: current_value.to_string(),
        });
    }
}

/// Check if a node is currently being renamed.
pub fn is_renaming(&self, node_id: &Key) -> bool {
    self.ctx.renaming_key.as_ref() == Some(node_id)
}
```

The `part_attrs` match is extended:

```rust,no_check
Part::NodeRenameInput { ref node_id } => self.node_rename_input_attrs(node_id),
```

### 6.6 Accessibility

| Key      | Behavior                                                       |
| -------- | -------------------------------------------------------------- |
| `F2`     | Start rename on the focused node (when `renamable` is true).   |
| `Enter`  | Commit the rename (when rename input is focused).              |
| `Escape` | Cancel the rename, discard changes, return focus to tree item. |

- The `NodeRenameInput` receives `aria-label` from `Messages.rename_label`, which receives
  the node's current label as the first argument. Screen readers announce e.g.,
  "Rename Documents" when focus enters the input.
- When rename starts, focus moves to the `NodeRenameInput`. When rename ends (commit or
  cancel), focus returns to the tree item.
- The rename input is not part of the tab sequence (`tabindex="-1"` is not needed because
  it is a native `<input>` that receives focus programmatically).

### 6.7 Messages

```rust,no_check
/// Added to the Messages struct for the renamable variant.
impl Messages {
    /// Accessible label for the rename input. Called with the node's current
    /// label and the resolved locale.
    /// Default: `|node_name, _locale| format!("Rename {}", node_name)`
    pub rename_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

// In Messages::default():
// rename_label: MessageFn::new(|node_name, _locale| format!("Rename {}", node_name)),
```

## 7. Library Parity

> Compared against: Ark UI (`TreeView`), React Aria (`Tree`).

### 7.1 Props

| Feature             | ars-ui                             | Ark UI                                 | React Aria                     | Notes                                       |
| ------------------- | ---------------------------------- | -------------------------------------- | ------------------------------ | ------------------------------------------- |
| Tree data source    | `items: TreeCollection<TreeItem>`  | `collection: TreeCollection<T>`        | `items: Iterable<T>`           | All provide collection-based API            |
| Controlled selected | `selected`                         | --                                     | `selectedKeys`                 | Ark uses context API                        |
| Default selected    | `default_selected`                 | `defaultSelectedValue`                 | `defaultSelectedKeys`          | Full match                                  |
| Controlled expanded | `expanded`                         | `expandedValue`                        | `expandedKeys`                 | Full match                                  |
| Default expanded    | `default_expanded`                 | `defaultExpandedValue`                 | `defaultExpandedKeys`          | Full match                                  |
| Selection mode      | `selection_mode`                   | `selectionMode`                        | `selectionMode`                | Full match                                  |
| Selection behavior  | `selection_behavior`               | --                                     | `selectionBehavior`            | ars-ui matches React Aria                   |
| Multiple selection  | `multiple`                         | --                                     | --                             | Expressed via `selection_mode` in all       |
| Disabled keys       | per-item `disabled` field          | --                                     | `disabledKeys`                 | ars-ui uses per-item field in TreeItem      |
| Disabled behavior   | --                                 | --                                     | `disabledBehavior`             | See below                                   |
| Typeahead           | via `TypeaheadSearch` event        | `typeahead`                            | --                             | ars-ui always has typeahead                 |
| Expand on click     | via `on_branch_control_click`      | `expandOnClick`                        | --                             | ars-ui expands on click by default          |
| Lazy loading        | `LoadChildren` variant (section 5) | `loadChildren`                         | --                             | ars-ui has full lazy-load variant           |
| Renamable           | `renamable` variant (section 6)    | `canRename`                            | --                             | ars-ui has full renamable variant           |
| Checked (checkbox)  | --                                 | `checkedValue` / `defaultCheckedValue` | --                             | See below                                   |
| DnD                 | DnD variant (section 4.2)          | --                                     | `dragAndDropHooks`             | ars-ui has full DnD variant                 |
| Escape key behavior | --                                 | --                                     | `escapeKeyBehavior`            | See below                                   |
| Virtualization      | --                                 | `scrollToIndexFn`                      | --                             | Adapter-level concern                       |
| Focused value       | --                                 | `focusedValue` / `defaultFocusedValue` | --                             | ars-ui tracks via `focused_node` in Context |
| Has children flag   | `TreeItem.has_children`            | --                                     | `hasChildItems` (per TreeItem) | Full match                                  |
| Node href           | `TreeItem.href`                    | --                                     | `href` (per TreeItem)          | ars-ui matches React Aria                   |
| Empty state         | --                                 | --                                     | `renderEmptyState`             | Adapter concern                             |
| On action           | --                                 | --                                     | `onAction`                     | See below                                   |

**Gaps:**

- **`disabledBehavior`**: React Aria supports `'all' | 'selection'` to control whether disabled items block all interaction or only selection. ars-ui disabled items block all interaction. Low priority -- `'selection'` mode is niche.
- **`checkedValue` (tri-state checkboxes)**: Ark UI supports per-node checkboxes with `checked`/`unchecked`/`indeterminate` states, including `NodeCheckbox` and `NodeCheckboxIndicator` anatomy. ars-ui could add this as a variant, but it is a specialized use case. Low priority.
- **`escapeKeyBehavior`**: React Aria allows `'clearSelection' | 'none'` on Escape press. ars-ui does not currently handle Escape in the tree. Low priority.
- **`onAction`**: React Aria fires `onAction(Key)` when a tree item is activated (Enter/click). ars-ui handles this via `SelectNode` event. The consumer can observe selection changes. Low priority.

### 7.2 Anatomy

| Part                | ars-ui            | Ark UI                                   | React Aria                   | Notes                              |
| ------------------- | ----------------- | ---------------------------------------- | ---------------------------- | ---------------------------------- |
| Root                | `Root`            | `Root`                                   | `Tree`                       | Full match                         |
| Label               | --                | `Label`                                  | --                           | Ark has optional tree label        |
| Tree list           | --                | `Tree`                                   | --                           | Ark separates Root from Tree list  |
| Branch node         | `Branch`          | `Branch`                                 | `TreeItem` (with children)   | Full match                         |
| Branch control      | `BranchControl`   | `BranchControl`                          | --                           | ars-ui and Ark have this           |
| Branch indicator    | `BranchIndicator` | `BranchIndicator`                        | --                           | ars-ui and Ark have this           |
| Branch text         | `BranchText`      | `BranchText`                             | --                           | ars-ui and Ark have this           |
| Branch content      | `BranchContent`   | `BranchContent`                          | --                           | ars-ui and Ark have this           |
| Branch indent guide | --                | `BranchIndentGuide`                      | --                           | Purely visual; adapter concern     |
| Leaf node           | `Leaf`            | `Item`                                   | `TreeItem` (no children)     | Full match                         |
| Leaf text           | `LeafText`        | `ItemText`                               | --                           | ars-ui and Ark have this           |
| Node checkbox       | --                | `NodeCheckbox` / `NodeCheckboxIndicator` | --                           | Ark-only tri-state checkbox        |
| Rename input        | `NodeRenameInput` | `NodeRenameInput`                        | --                           | ars-ui and Ark have this           |
| Node provider       | --                | `NodeProvider`                           | --                           | Ark-specific context provider      |
| Item content        | --                | --                                       | `TreeItemContent`            | React Aria render prop wrapper     |
| Section             | --                | --                                       | `TreeSection` / `TreeHeader` | React Aria grouping; not in ars-ui |
| Load more item      | --                | --                                       | `TreeLoadMoreItem`           | React Aria infinite scroll trigger |

**Gaps:**

- **`BranchIndentGuide`**: Ark UI has a dedicated indent guide part for rendering vertical tree lines. This is purely a CSS/visual concern and can be achieved via CSS `::before` pseudo-elements on Branch nodes in ars-ui. Not worth adding as a formal part.
- **`Label`**: Ark UI has a `Label` part for the tree's accessible name. ars-ui achieves this via `aria-label` or `aria-labelledby` on the Root element, which is the standard ARIA approach. No gap.
- **`TreeSection` / `TreeHeader`**: React Aria supports grouping tree items into labeled sections. This is an advanced composition pattern; consumers can achieve this by structuring their TreeCollection data appropriately. Low priority.

### 7.3 Events

| Callback            | ars-ui                | Ark UI                   | React Aria          | Notes                         |
| ------------------- | --------------------- | ------------------------ | ------------------- | ----------------------------- |
| Selection change    | `Bindable` onChange   | `onSelectionChange`      | `onSelectionChange` | ars-ui uses Bindable pattern  |
| Expanded change     | `Bindable` onChange   | `onExpandedChange`       | `onExpandedChange`  | ars-ui uses Bindable pattern  |
| Focus change        | `Focus`/`Blur` events | `onFocusChange`          | --                  | ars-ui and Ark track focus    |
| Checked change      | --                    | `onCheckedChange`        | --                  | Ark-only; see checkbox gap    |
| Rename start        | `RenameStart`         | `onRenameStart`          | --                  | ars-ui and Ark have this      |
| Rename complete     | `RenameCommit`        | `onRenameComplete`       | --                  | ars-ui and Ark have this      |
| Before rename       | --                    | `onBeforeRename`         | --                  | Ark has pre-rename validation |
| Load children done  | `ChildrenLoaded`      | `onLoadChildrenComplete` | --                  | ars-ui and Ark have this      |
| Load children error | `LoadError` (implied) | `onLoadChildrenError`    | --                  | ars-ui and Ark have this      |
| Action              | --                    | --                       | `onAction`          | React Aria item activation    |

**Gaps:** None that need addressing. `onBeforeRename` can be implemented by the consumer handling `RenameStart` and conditionally allowing it. `onAction` is covered by `SelectNode` semantics.

### 7.4 Features

| Feature                      | ars-ui        | Ark UI            | React Aria  |
| ---------------------------- | ------------- | ----------------- | ----------- |
| Expand/collapse              | Yes           | Yes               | Yes         |
| Single/multiple selection    | Yes           | Yes               | Yes         |
| Keyboard navigation (arrows) | Yes           | Yes               | Yes         |
| Typeahead search             | Yes           | Yes               | No          |
| Drag and drop                | Yes (variant) | No                | Yes         |
| Lazy loading                 | Yes (variant) | Yes               | No          |
| Inline rename                | Yes (variant) | Yes               | No          |
| RTL support                  | Yes           | Yes               | Yes         |
| Per-node disabled            | Yes           | Yes               | Yes         |
| Tri-state checkboxes         | No            | Yes               | No          |
| Virtualization               | Adapter-level | `scrollToIndexFn` | Virtualizer |
| Expand all / collapse all    | Yes           | Via context API   | No          |

**Gaps:** Tri-state checkboxes (Ark-only) are a specialized feature not essential for v1.

### 7.5 Summary

- **Overall:** Full parity with React Aria. Near-full parity with Ark UI (missing tri-state checkboxes).
- **Divergences:** ars-ui uses per-item `disabled` field in `TreeItem` rather than a `disabledKeys` set (React Aria style). Ark's `BranchIndentGuide` is a CSS concern. ars-ui separates lazy loading and rename into explicit variants rather than always-available props.
- **Recommended additions:** None for v1. Tri-state node checkboxes could be added as a future variant if demand warrants.
