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

| Event                   | Payload | Description                                                  |
| ----------------------- | ------- | ------------------------------------------------------------ |
| `ExpandNode(String)`    | node ID | Expand a branch node.                                        |
| `CollapseNode(String)`  | node ID | Collapse a branch node.                                      |
| `ToggleNode(String)`    | node ID | Toggle expand/collapse of a branch.                          |
| `SelectNode(String)`    | node ID | Add a node to the selection.                                 |
| `DeselectNode(String)`  | node ID | Remove a node from the selection.                            |
| `FocusNode(String)`     | node ID | Move focus indicator to a node.                              |
| `FocusNext`             | —       | Move focus to the next visible node.                         |
| `FocusPrev`             | —       | Move focus to the previous visible node.                     |
| `FocusFirst`            | —       | Move focus to the first visible node.                        |
| `FocusLast`             | —       | Move focus to the last visible node.                         |
| `FocusParent`           | —       | Move focus to the parent of the focused node.                |
| `Focus { is_keyboard }` | `bool`  | The tree container received focus.                           |
| `Blur`                  | —       | The tree container lost focus.                               |
| `TypeaheadSearch(char)` | `char`  | Jump to the next node whose label starts with the character. |

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::{
    selection,
    Key, TreeCollection, TreeItemConfig, Collection,
};
use alloc::collections::BTreeSet;

/// Value type for TreeView collection items.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeItem {
    /// Display label for the node (used in typeahead and accessibility).
    pub label: String,
    /// Whether this node is disabled (not selectable, skipped during navigation).
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
    /// Typeahead buffer (for multi-character typeahead, cleared after timeout).
    pub typeahead_buffer: String,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Generated element IDs for all parts.
    pub ids: ComponentIds,
}

// NOTE: The local selection::Mode enum has been removed. TreeView now uses the
// canonical `ars_collections::selection::Mode` (None/Single/Multiple).
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
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};

/// States for the `TreeView` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The tree view is in the idle state.
    Idle,
    /// The tree view is in the focused state.
    Focused,
}

/// Events for the `TreeView` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Expand a node.
    ExpandNode(String),
    /// Collapse a node.
    CollapseNode(String),
    /// Toggle the expand/collapse state of a node.
    ToggleNode(String),
    /// Select a node.
    SelectNode(String),
    /// Deselect a node.
    DeselectNode(String),
    /// Focus a node.
    FocusNode(String),
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
    /// Typeahead search for a node.
    TypeaheadSearch(char),
    /// Expand all nodes in the tree.
    ExpandAll,
    /// Collapse all nodes in the tree.
    CollapseAll,
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let selected = match &props.selected {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_selected.clone()),
        };
        let expanded = match &props.expanded {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_expanded.clone()),
        };
        let selection_state = selection::State::new(
            props.selection_mode,
            props.selection_behavior,
        );
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
            selection_mode: props.selection_mode,
            typeahead_buffer: String::new(),
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {

            Event::ExpandNode(id) => {
                let id = id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    exp.insert(id);
                    ctx.expanded.set(exp);
                }))
            }

            Event::CollapseNode(id) => {
                let id = id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    exp.remove(&id);
                    ctx.expanded.set(exp);
                }))
            }

            Event::ToggleNode(id) => {
                let is_expanded = ctx.expanded.get().contains(id.as_str());
                let id = id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    if is_expanded {
                        exp.remove(&id);
                    } else {
                        exp.insert(id);
                    }
                    ctx.expanded.set(exp);
                }))
            }

            Event::SelectNode(id) => {
                if ctx.selection_mode == selection::Mode::None { return None; }
                let key = Key::from(id);
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.select(key);
                    ctx.selected.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::DeselectNode(id) => {
                let key = Key::from(id);
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_state = ctx.selection_state.deselect(key);
                    ctx.selected.set(new_state.selected_keys.clone());
                    ctx.selection_state = new_state;
                }))
            }

            Event::FocusNode(id) => {
                let id = id.clone();
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focused_node = Some(id);
                    }))
            }

            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focus_visible = is_keyboard;
                    }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        ctx.focus_visible = false;
                    }))
            }

            // ── Navigation (via TreeCollection API) ────────────────────────
            Event::FocusNext => {
                // Use TreeCollection::key_after() which respects expanded state
                // and skips structural nodes. Wraps to first node.
                let expanded = ctx.expanded.get();
                let visible = ctx.items.visible_keys_with_expanded(expanded);
                let target = match &ctx.focused_node {
                    Some(current) => {
                        let pos = visible.iter().position(|k| k == current);
                        pos.and_then(|p| visible.get(p + 1).or_else(|| visible.first()))
                    }
                    None => visible.first(),
                };
                target.map(|key| {
                    let k = key.clone();
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(k);
                        ctx.focus_visible = true;
                    })
                })
            }

            Event::FocusPrev => {
                let expanded = ctx.expanded.get();
                let visible = ctx.items.visible_keys_with_expanded(expanded);
                let target = match &ctx.focused_node {
                    Some(current) => {
                        let pos = visible.iter().position(|k| k == current);
                        pos.and_then(|p| {
                            if p > 0 { visible.get(p - 1) }
                            else { visible.last() }
                        })
                    }
                    None => visible.last(),
                };
                target.map(|key| {
                    let k = key.clone();
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(k);
                        ctx.focus_visible = true;
                    })
                })
            }

            Event::FocusFirst => {
                let expanded = ctx.expanded.get();
                let visible = ctx.items.visible_keys_with_expanded(expanded);
                visible.first().map(|key| {
                    let k = key.clone();
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(k);
                        ctx.focus_visible = true;
                    })
                })
            }

            Event::FocusLast => {
                let expanded = ctx.expanded.get();
                let visible = ctx.items.visible_keys_with_expanded(expanded);
                visible.last().map(|key| {
                    let k = key.clone();
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(k);
                        ctx.focus_visible = true;
                    })
                })
            }

            Event::FocusParent => {
                // Use Node::parent_key from the TreeCollection to navigate up
                let target = ctx.focused_node.as_ref()
                    .and_then(|focused| ctx.items.get(focused))
                    .and_then(|node| node.parent_key.clone());
                target.map(|key| {
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(key);
                        ctx.focus_visible = true;
                    })
                })
            }

            Event::TypeaheadSearch(ch) => {
                // Find the next visible node whose text_value starts with ch
                // (case-insensitive), starting after the focused node.
                let ch = ch.to_lowercase().next().unwrap_or(*ch);
                let expanded = ctx.expanded.get();
                let visible = ctx.items.visible_keys_with_expanded(expanded);
                let start_pos = ctx.focused_node.as_ref()
                    .and_then(|f| visible.iter().position(|k| k == f))
                    .map(|p| p + 1)
                    .unwrap_or(0);

                // Search from start_pos, wrapping around
                let target = visible.iter()
                    .cycle()
                    .skip(start_pos)
                    .take(visible.len())
                    .find(|key| {
                        ctx.items.get(key)
                            .and_then(|node| node.text_value.chars().next())
                            .map(|first| first.to_lowercase().next().unwrap_or(first) == ch)
                            .unwrap_or(false)
                    });

                target.map(|key| {
                    let k = key.clone();
                    TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused_node = Some(k);
                        ctx.focus_visible = true;
                    })
                })
            }

            // ── Expand/Collapse All ────────────────────────────────────
            // ExpandAll — expand every expandable node
            Event::ExpandAll => {
                let expandable = ctx.items.nodes()
                    .filter(|n| n.has_children)
                    .map(|n| n.key.to_string())
                    .collect::<Vec<_>>();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut exp = ctx.expanded.get().clone();
                    for key in expandable {
                        exp.insert(key);
                    }
                    ctx.expanded.set(exp);
                }))
            }

            // CollapseAll — collapse every node
            Event::CollapseAll => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.expanded.set(BTreeSet::new());
                }))
            }

            _ => None,
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
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tree-view"]
pub enum Part {
    Root,
    Branch { node_id: String },
    BranchControl { node_id: String },
    BranchIndicator { node_id: String },
    BranchText,
    BranchContent { node_id: String },
    Leaf { node_id: String },
    LeafText,
}

/// API for the `TreeView` component.
pub struct Api<'a> {
    /// Current machine state.
    state: &'a State,
    /// Current context.
    ctx:   &'a Context,
    /// Current props.
    props: &'a Props,
    /// Event dispatcher.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Check if a node is selected.
    pub fn is_node_selected(&self, id: &str) -> bool {
        self.ctx.selected.get().contains(&Key::from(id))
    }

    /// Check if a node is expanded.
    pub fn is_node_expanded(&self, id: &str) -> bool {
        self.ctx.expanded.get().contains(id)
    }

    /// Check if a node is focused.
    pub fn is_node_focused(&self, id: &str) -> bool {
        self.ctx.focused_node.as_ref().map(|k| k.as_ref()) == Some(id)
    }

    /// Look up a node from the collection by key.
    pub fn get_node(&self, id: &str) -> Option<&ars_collections::Node<TreeItem>> {
        self.ctx.items.get(&Key::from(id))
    }

    /// Compute (setsize, posinset) for a node from the collection.
    /// Returns (sibling count at this level, 1-based position among siblings).
    pub fn sibling_info(&self, id: &str) -> (u32, u32) {
        let key = Key::from(id);
        let node = match self.ctx.items.get(&key) { Some(n) => n, None => return (1, 1) };
        let siblings: Vec<_> = match &node.parent_key {
            Some(parent) => self.ctx.items.children_of(parent).collect(),
            None => {
                // Root-level nodes: collect all level-0 nodes
                self.ctx.items.nodes()
                    .filter(|n| n.parent_key.is_none())
                    .collect()
            }
        };
        let setsize = siblings.len() as u32;
        let posinset = siblings.iter()
            .position(|n| n.key == key)
            .map(|p| p as u32 + 1)
            .unwrap_or(1);
        (setsize, posinset)
    }

    /// Attrs for the tree root container (`role="tree"`).
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Role, "tree");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }
        if let Some(focused) = &self.ctx.focused_node {
            attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), focused);
        }
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Handle focus event for the tree view root.
    pub fn on_root_focus(&self) {
        (self.send)(Event::Focus { is_keyboard: false });
    }

    /// Handle blur event for the tree view root.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Attrs for a branch node (expandable, has children).
    ///
    /// `node_id` — unique ID. Level, setsize, posinset, and disabled state
    /// are derived from the `TreeCollection`.
    ///
    /// When the node's `has_children` flag is true, `aria-expanded` is always
    /// emitted even if the collection currently contains no children for this
    /// node. This is essential for lazy-loaded trees where the expand affordance
    /// must be present before children are fetched.
    pub fn branch_attrs(&self, node_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_expanded = self.is_node_expanded(node_id);
        let is_selected = self.is_node_selected(node_id);
        let is_focused  = self.is_node_focused(node_id);
        let node = self.get_node(node_id);
        let level = node.map(|n| n.level as u32 + 1).unwrap_or(1); // 1-based
        let (setsize, posinset) = self.sibling_info(node_id);
        let disabled = node.and_then(|n| n.value.as_ref()).map(|v| v.disabled).unwrap_or(false);
        let has_children_flag = node.and_then(|n| n.value.as_ref()).map(|v| v.has_children).unwrap_or(false);
        let has_actual_children = node.map(|n| n.has_children).unwrap_or(false);
        attrs.set(HtmlAttr::Role, "treeitem");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Branch { node_id: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Emit aria-expanded when the node has real children OR has_children flag is set
        if has_actual_children || has_children_flag {
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_expanded { "true" } else { "false" });
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Level),   level.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::SetSize), setsize.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::PosInSet),posinset.to_string());
        if disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if is_selected { attrs.set_bool(HtmlAttr::Data("ars-selected"), true); }
        if is_expanded { attrs.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        if is_focused && self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    /// Attrs for the clickable control area inside a branch (the row the user activates).
    /// When the node has an `href`, sets `HtmlAttr::Href` so the adapter renders an `<a>`
    /// element instead of a `<div>`. The `role="treeitem"` on the parent `Branch` is preserved.
    pub fn branch_control_attrs(&self, node_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchControl { node_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(href) = self.get_node(node_id)
            .and_then(|n| n.value.as_ref())
            .and_then(|v| v.href.as_deref())
        {
            attrs.set(HtmlAttr::Href, href);
        }
        attrs
    }

    /// Handle click event for the branch control.
    pub fn on_branch_control_click(&self, node_id: &str) {
        (self.send)(Event::ToggleNode(node_id.to_string()));
        (self.send)(Event::FocusNode(node_id.to_string()));
    }

    /// Attrs for the expand/collapse chevron indicator inside a branch.
    pub fn branch_indicator_attrs(&self, node_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_expanded = self.is_node_expanded(node_id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchIndicator { node_id: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if is_expanded { attrs.set_bool(HtmlAttr::Data("ars-expanded"), true); }
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the text label inside a branch.
    pub fn branch_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the loading indicator text for a branch node whose children are being fetched.
    /// Used as visually hidden text inside the branch when `load_state` is `Loading`.
    pub fn loading_label(&self) -> String {
        (self.ctx.messages.loading_label)(&self.ctx.locale)
    }

    /// Attrs for the children container inside an expanded branch (`role="group"`).
    pub fn branch_content_attrs(&self, node_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_expanded = self.is_node_expanded(node_id);
        attrs.set(HtmlAttr::Role, "group");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::BranchContent { node_id: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if !is_expanded {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for a leaf node (no children, no expand/collapse).
    /// Level, setsize, posinset, and disabled state are derived from the `TreeCollection`.
    ///
    /// When the node's `has_children` flag is true (but no actual children are
    /// loaded), the leaf is treated as an expandable node: `aria-expanded="false"`
    /// is set so screen readers announce the expand affordance. Once children
    /// are loaded, the node transitions from `Leaf` to `Branch` in the adapter.
    ///
    /// When the node has an `href`, sets `HtmlAttr::Href` so the adapter renders
    /// an `<a>` element instead of the default element. `role="treeitem"` is preserved.
    pub fn leaf_attrs(&self, node_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_selected = self.is_node_selected(node_id);
        let is_focused  = self.is_node_focused(node_id);
        let node = self.get_node(node_id);
        let level = node.map(|n| n.level as u32 + 1).unwrap_or(1);
        let (setsize, posinset) = self.sibling_info(node_id);
        let disabled = node.and_then(|n| n.value.as_ref()).map(|v| v.disabled).unwrap_or(false);
        let has_children_flag = node.and_then(|n| n.value.as_ref()).map(|v| v.has_children).unwrap_or(false);
        attrs.set(HtmlAttr::Role, "treeitem");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Leaf { node_id: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // When has_children is true but no children are loaded yet, show expand affordance
        if has_children_flag {
            let is_expanded = self.is_node_expanded(node_id);
            attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_expanded { "true" } else { "false" });
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Level),    level.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::SetSize),  setsize.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::PosInSet), posinset.to_string());
        if disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if is_selected { attrs.set_bool(HtmlAttr::Data("ars-selected"), true); }
        if is_focused && self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        // When href is present, the adapter renders this as an <a> element
        if let Some(href) = node
            .and_then(|n| n.value.as_ref())
            .and_then(|v| v.href.as_deref())
        {
            attrs.set(HtmlAttr::Href, href);
        }
        attrs
    }

    /// Handle click event for the leaf node.
    pub fn on_leaf_click(&self, node_id: &str) {
        (self.send)(Event::SelectNode(node_id.to_string()));
        (self.send)(Event::FocusNode(node_id.to_string()));
    }

    /// Attrs for the text label inside a leaf node.
    pub fn leaf_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LeafText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Expand all expandable nodes in the tree.
    pub fn expand_all(&self) {
        (self.send)(Event::ExpandAll);
    }

    /// Collapse all nodes in the tree.
    pub fn collapse_all(&self) {
        (self.send)(Event::CollapseAll);
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
        }
    }
}
```

## 2. Anatomy

```text
TreeView
└── Root                           role="tree"
    ├── Branch                     role="treeitem" aria-expanded
    │   ├── BranchControl        (clickable row; <a> when href present)
    │   │   ├── BranchIndicator  aria-hidden="true"
    │   │   └── BranchText
    │   └── BranchContent        role="group"
    │       └── (nested Branch or Leaf nodes)
    └── Leaf                       role="treeitem" (aria-expanded when has_children)
        └── LeafText             (<a> when href present)
```

| Part              | Element                               | Key Attributes                                                                                                                                                                                              |
| ----------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`            | `<ul>` or `<div>`                     | `data-ars-scope="tree-view"`, `data-ars-part="root"`, `role="tree"`, `aria-multiselectable`                                                                                                                 |
| `Branch`          | `<li>` or `<div>`                     | `role="treeitem"`, `aria-expanded` (also set when `has_children` is true with no loaded children), `aria-selected`, `aria-level`, `aria-setsize`, `aria-posinset`, `data-ars-expanded`, `data-ars-selected` |
| `BranchControl`   | `<div>` or `<a>` (when href)          | `data-ars-scope="tree-view"`, `data-ars-part="branch-control"`, `href` (when present)                                                                                                                       |
| `BranchIndicator` | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="branch-indicator"`, `aria-hidden="true"`, `data-ars-expanded`                                                                                                 |
| `BranchText`      | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="branch-text"`                                                                                                                                                 |
| `BranchContent`   | `<ul>` or `<div>`                     | `role="group"`, `data-ars-scope="tree-view"`, `data-ars-part="branch-content"`                                                                                                                              |
| `Leaf`            | `<li>`, `<div>`, or `<a>` (when href) | `role="treeitem"`, `aria-expanded="false"` (when `has_children` is true), `aria-selected`, `aria-level`, `aria-setsize`, `aria-posinset`, `data-ars-selected`, `href` (when present)                        |
| `LeafText`        | `<span>`                              | `data-ars-scope="tree-view"`, `data-ars-part="leaf-text"`                                                                                                                                                   |

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

| Key                 | Behavior                                                                                                         |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `ArrowDown`         | Move focus to the next visible node (skips hidden children of collapsed branches).                               |
| `ArrowUp`           | Move focus to the previous visible node.                                                                         |
| `ArrowRight`        | If focused node is a collapsed branch: expand it. If already expanded (or a leaf): move focus to first child.    |
| `ArrowLeft`         | If focused node is an expanded branch: collapse it. If collapsed (or a leaf): move focus to parent.              |
| `Home`              | Move focus to the first node in the tree.                                                                        |
| `End`               | Move focus to the last visible node in the tree.                                                                 |
| `Enter`             | Select the focused node (if `selection_mode != None`).                                                           |
| `Space`             | Toggle selection of the focused node (in multiple mode).                                                         |
| `*` (asterisk)      | Expand all siblings of the focused node.                                                                         |
| `F2`                | Start inline rename on the focused node (when `renamable` is true). See [section 6](#6-variant-renamable-nodes). |
| Printable character | Typeahead: jump to the next node whose visible label starts with the typed character.                            |

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

When focus moves to an off-screen tree item, the adapter scrolls it into view using `element.scrollIntoView({ block: 'nearest', inline: 'nearest' })`. For nested scrollable containers, the adapter must walk up the DOM and call `scrollIntoView` on the closest scrollable ancestor only. On iOS Safari, where programmatic focus doesn't trigger scroll, the adapter must explicitly call `scrollIntoView` after `element.focus()`. Smooth scrolling is avoided for keyboard navigation to prevent timing conflicts.

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
#[derive(Clone, Debug)]
pub struct Messages {
    /// Loading indicator text for branches whose children are being fetched
    /// (default: "Loading…"). Rendered as visually hidden text and announced
    /// via `aria-live="polite"`.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            loading_label: MessageFn::static_str("Loading\u{2026}"),
        }
    }
}

impl ComponentMessages for Messages {}
```

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, TreeView integrates with `DraggableCollection<T>` and `DroppableCollection<T>` from `06-collections.md` §7. This adds optional `drag::Handle` and `drop::Indicator` anatomy parts. TreeView supports both `CollectionDndEvent::Reorder` (reordering siblings) and `CollectionDndEvent::Move` (reparenting nodes by dropping onto a parent item with `drop::Position::On`).

### 4.2 Drag-Reorder Behavior

When drag-and-drop reorder is enabled (`dnd_enabled: true`), the following behaviors apply:

- **Drag handle**: Each tree item renders an optional drag handle affordance (the `drag::Handle` anatomy part). The handle is the grab target; the entire row is not draggable by default to avoid conflicts with text selection.
- **Hover-expand**: When dragging over a collapsed tree node, the node automatically expands after a 500ms hover delay. This allows users to drop items into nested levels without manually expanding first.
- **Drop target indication**: Valid drop zones are visually highlighted during drag. Three drop positions are supported:
  - `Before` — insert before the target node (horizontal line above)
  - `After` — insert after the target node (horizontal line below)
  - `Child` — insert as a child of the target node (target node background highlight)
- **Drop position detection**: The drop position is determined by the pointer's vertical position within the target row: top 25% = `Before`, bottom 25% = `After`, middle 50% = `Child`.
- **Keyboard drag**: Full keyboard support for reordering:
  - `Enter` or `Space` on a focused drag handle to pick up the item
  - Arrow keys to move the item through valid drop positions
  - `Enter` to confirm the drop
  - `Escape` to cancel and return the item to its original position
- **Reorder callback**: `on_reorder: Callback<ReorderEvent>` fires when a drop is completed.

```rust
/// Event emitted when a tree item is reordered via drag-and-drop.
#[derive(Clone, Debug, PartialEq)]
pub struct ReorderEvent {
    /// Path of the dragged item (sequence of keys from root to item).
    pub source_path: Vec<Key>,
    /// Path of the drop target.
    pub target_path: Vec<Key>,
    /// Where relative to the target the item was dropped.
    pub position: DropPosition,
}

// `DropPosition` — defined in `06-collections.md`
```

## 5. Variant: Lazy Loading

`TreeView` supports lazy loading of child nodes for large or server-driven hierarchies. When a node is first expanded and its children are not yet loaded, the machine fires a `LoadChildren` event.

### 5.1 Additional Props

```rust
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

```rust
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

```rust
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

```rust
/// Added to TreeView Event enum for the renamable variant.

/// User initiates rename on a node (e.g., F2 or slow double-click).
RenameStart(Key),
/// User commits the new name.
RenameCommit { key: Key, new_name: String },
/// User cancels rename (Escape or blur without change).
RenameCancel(Key),
```

### 6.3 Additional Context

```rust
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

```rust
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

```rust
/// Added to Part enum.
NodeRenameInput { node_id: String },
```

```rust
/// Attrs for the inline rename input.
/// Rendered only when `renaming_key == Some(key)` for this node.
pub fn node_rename_input_attrs(&self, node_id: &str) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] =
        Part::NodeRenameInput { node_id: Default::default() }.data_attrs();
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
pub fn on_rename_input_keydown(&self, node_id: &str, key_code: &str, current_value: &str) {
    match key_code {
        "Enter" => {
            (self.send)(Event::RenameCommit {
                key: Key::from(node_id),
                new_name: current_value.to_string(),
            });
        }
        "Escape" => {
            (self.send)(Event::RenameCancel(Key::from(node_id)));
        }
        _ => {}
    }
}

/// Handle blur on the rename input — commits the rename if still active.
pub fn on_rename_input_blur(&self, node_id: &str, current_value: &str) {
    let key = Key::from(node_id);
    if self.ctx.renaming_key.as_ref() == Some(&key) {
        (self.send)(Event::RenameCommit {
            key,
            new_name: current_value.to_string(),
        });
    }
}

/// Check if a node is currently being renamed.
pub fn is_renaming(&self, node_id: &str) -> bool {
    self.ctx.renaming_key.as_ref().map(|k| k.as_ref()) == Some(node_id)
}
```

The `part_attrs` match is extended:

```rust
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

```rust
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
