---
component: Menu
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Menu
    radix-ui: DropdownMenu
    react-aria: Menu
---

# Menu

A trigger-activated popup menu containing actions, checkbox items, radio items, and
submenus.

Items are stored as a `StaticCollection<menu::Item>` (from `06-collections.md`). Separators
are structural `NodeType::Separator` nodes. Groups are `NodeType::Section` + `NodeType::Header`
nodes. Navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`.
Typeahead uses `typeahead::State`.

## 1. State Machine

```rust
/// User-facing payload for Menu items. The machine uses Node-level metadata
/// (key, text_value, node_type) for navigation and ARIA; the `Item` payload
/// carries the item subtype and display metadata.
#[derive(Clone, Debug)]
pub struct Item {
    /// The label of the item.
    pub label: String,
    /// The type of the item.
    pub item_type: ItemType,
    /// The keyboard shortcut text displayed to the right of the item label.
    /// Keyboard shortcut text displayed to the right of the item label
    /// (e.g., "Ctrl+C", "⌘V"). Purely visual — global shortcut registration
    /// is the consumer's responsibility.
    pub shortcut: Option<String>,
    /// Semantic keyboard shortcut for `aria-keyshortcuts` (e.g., "Control+S").
    pub aria_keyshortcuts: Option<String>,
    /// Per-item override for menu-level `close_on_action` behavior.
    /// Applies to both selectable items (checkbox/radio) and action items (normal).
    /// When `Some(false)`, activating this item does NOT close the menu.
    /// When `Some(true)`, activating this item closes the menu.
    /// When `None`, the menu-level default applies. Default: `None`.
    pub close_on_action: Option<bool>,
}

/// The type of the menu item.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemType {
    /// A normal menu item.
    Normal,
    /// A checkbox menu item.
    Checkbox,
    /// A radio menu item.
    Radio {
        /// The group of the radio item.
        group: String,
    },
    /// A submenu menu item.
    Submenu,
}
```

Separators are represented as `NodeType::Separator` nodes in the collection (not as a
`ItemType` variant), built via `CollectionBuilder::separator()`.

### 1.1 States

```rust
/// The states of the Menu state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The menu is closed.
    Closed,
    /// The menu is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the Menu state machine.
#[derive(Clone, Debug)]
pub enum Event {
    /// Open the menu.
    Open,
    /// Close the menu.
    Close,
    /// Highlight an item.
    HighlightItem(Option<Key>),
    /// Highlight the first item.
    HighlightFirst,
    /// Highlight the last item.
    HighlightLast,
    /// Highlight the next item.
    HighlightNext,
    /// Highlight the previous item.
    HighlightPrev,
    /// Select an item.
    SelectItem(Key),
    /// Toggle a checkbox item.
    ToggleCheckboxItem(Key),
    /// Select a radio item.
    SelectRadioItem {
        /// The group of the radio item.
        group: String,
        /// The value of the radio item.
        value: Key,
    },
    /// Open a submenu.
    OpenSubmenu(Key),
    /// Close a submenu.
    CloseSubmenu,
    // Reserved for future use: submenus and menu-bar focus tracking.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// Blur the menu.
    Blur,
    /// Click outside the menu.
    ClickOutside,
    /// Typeahead search.
    TypeaheadSearch(char, u64),
    /// Update the item collection.
    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),
}
```

### 1.3 Context

```rust
/// The context of the Menu state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The items of the menu.
    pub items: StaticCollection<Item>,
    /// Whether the menu is open.
    pub open: bool,
    /// The highlighted key of the menu.
    pub highlighted_key: Option<Key>,
    // Reserved for future use: submenus and menu-bar focus tracking.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The checked items of the menu.
    pub checked_items: BTreeMap<Key, bool>,
    /// The radio groups of the menu.
    pub radio_groups: BTreeMap<String, Key>,
    /// The submenu open key of the menu.
    pub submenu_open: Option<Key>,
    /// The typeahead state of the menu.
    pub typeahead: typeahead::State,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    pub loop_focus: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// Props for the Menu state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the menu.
    pub id: String,
    /// Whether the menu is disabled.
    pub disabled: bool,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    pub loop_focus: bool,
    /// The positioning options of the menu.
    pub positioning: PositioningOptions,
    /// Whether the menu closes when an item is activated (selected or triggered).
    /// Applies to both selectable items (checkbox/radio) and action items (normal).
    /// Individual `Item.close_on_action` overrides this per-item.
    /// Default: `true`.
    pub close_on_action: bool,
    /// How disabled items behave in keyboard navigation.
    /// `Skip` = disabled items are skipped during keyboard navigation (not focusable, not selectable).
    /// `FocusOnly` = disabled items are focusable but not selectable (default for ARIA Listbox pattern).
    /// Default: `DisabledBehavior::Skip`.
    pub disabled_behavior: DisabledBehavior, // default: DisabledBehavior::Skip
    /// Prevents deselecting the last remaining selected item. When `true` and the user
    /// attempts to deselect the only selected value, the action is a no-op, ensuring at
    /// least one item is always selected. Only meaningful for menus with selectable
    /// (checkbox/radio) items.
    pub disallow_empty_selection: bool,
    /// Callback invoked when the dropdown open state changes.
    /// Fires after the transition completes with the new open state value.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when a menu item is activated (Enter/click on action items).
    /// Distinct from selection-change callbacks — `on_action` fires for command execution,
    /// not for checkbox/radio state toggling.
    pub on_action: Option<Callback<Key>>,
    /// Set of keys for menu items that are disabled.
    /// Disabled items are skipped during keyboard navigation and cannot be selected or triggered.
    /// Follows the same pattern as `disabled_keys` on Listbox.
    pub disabled_keys: BTreeSet<Key>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            loop_focus: true,
            positioning: PositioningOptions::default(),
            close_on_action: true,
            disabled_behavior: DisabledBehavior::Skip,
            disallow_empty_selection: false,
            on_open_change: None,
            on_action: None,
            disabled_keys: BTreeSet::new(),
        }
    }
}
```

### 1.5 Submenu Hover Bridge

When the user hovers a `SubTrigger`, the submenu `SubContent` opens to the side. Moving
the pointer from `SubTrigger` to `SubContent` requires traversing empty space between the two
elements. Without a **safe area** (hover bridge), the pointer will leave the `SubTrigger`'s
bounding box, causing the highlight to shift to adjacent items or the submenu to close.

The same **safe triangle** algorithm used by [HoverCard](../overlay/hover-card.md#safe-area-hover-bridge) (see `12-overlay-components.md` §Safe
Area) applies here:

1. When a `SubTrigger` is highlighted and `submenu_open` is `Some(id)`, compute a triangle
   polygon between the `SubTrigger` bounding rect and the `SubContent` bounding rect.
2. While the pointer remains inside this polygon, the submenu stays open and the `SubTrigger`
   remains highlighted (pointer leave events on the `SubTrigger` are suppressed).
3. The safe area listener is cleaned up when the submenu closes, unmounts, or the pointer
   enters `SubContent`.

This is the standard behavior implemented by all major libraries (Radix `pointerGraceArea`,
Ark-UI `safePolygon`, Floating UI `safePolygon()` middleware).

#### 1.5.1 Submenu Positioning

Submenus are positioned using the positioning engine with direction-aware placement:

| Direction | Placement                                                           |
| --------- | ------------------------------------------------------------------- |
| LTR       | `Placement::RightStart` (submenu opens to the right of the trigger) |
| RTL       | `Placement::LeftStart` (submenu opens to the left of the trigger)   |

The adapter MUST resolve the placement based on `ctx.dir` before calling
`compute_position()`.

### 1.6 Full Machine Implementation

```rust
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ctx = Context {
            items: StaticCollection::empty(),
            open: false,
            highlighted_key: None,
            focused: false,
            focus_visible: false,
            checked_items: BTreeMap::new(),
            radio_groups: BTreeMap::new(),
            submenu_open: None,
            typeahead: typeahead::State::new(),
            loop_focus: props.loop_focus,
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Closed, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Open: highlight first item, focus content
            (State::Closed, Event::Open) => {
                let first = first_enabled_key(&ctx.items, &BTreeSet::new(), props.disabled_behavior);

                Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                    ctx.open = true;
                    ctx.highlighted_key = first;
                }).with_effect(PendingEffect::new("focus_content", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let content_id = ctx.ids.part("content");
                    platform.focus_element_by_id(&content_id);
                    no_cleanup()
                })))
            }

            // Close / ClickOutside / Blur: close menu
            (State::Open, Event::Close) | (State::Open, Event::ClickOutside) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                    ctx.submenu_open = None;
                }).with_effect(PendingEffect::new("focus_trigger", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let trigger_id = ctx.ids.part("trigger");
                    platform.focus_element_by_id(&trigger_id);
                    no_cleanup()
                })))
            }

            // HighlightNext / HighlightPrev: navigate enabled items (only when open)
            (State::Open, Event::HighlightNext) => {
                let next = match &ctx.highlighted_key {
                    Some(k) => next_enabled_key(&ctx.items, k, &BTreeSet::new(),
                        props.disabled_behavior, ctx.loop_focus),
                    None => first_enabled_key(&ctx.items, &BTreeSet::new(), props.disabled_behavior),
                };

                next.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (State::Open, Event::HighlightPrev) => {
                let prev = match &ctx.highlighted_key {
                    Some(k) => prev_enabled_key(&ctx.items, k, &BTreeSet::new(),
                        props.disabled_behavior, ctx.loop_focus),
                    None => last_enabled_key(&ctx.items, &BTreeSet::new(), props.disabled_behavior),
                };

                prev.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightFirst) => {
                let first = first_enabled_key(&ctx.items, &BTreeSet::new(), props.disabled_behavior);

                first.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightLast) => {
                let last = last_enabled_key(&ctx.items, &BTreeSet::new(), props.disabled_behavior);

                last.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightItem(key)) => {
                // Guard: ignore highlight requests for keys not present in the collection.
                if let Some(ref k) = key {
                    if !ctx.items.contains_key(k) { return None; }
                }
                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = key; }))
            }

            // SelectItem: fire action, close menu
            (State::Open, Event::SelectItem(key)) => {
                if ctx.items.get(&key).map_or(true, |n|
                    n.node_type != NodeType::Item) { return None; }

                // Disabled items cannot be selected (disabled_keys not used for Menu; rely on Node lookup)
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                    ctx.submenu_open = None;
                }).with_effect(PendingEffect::new("focus_trigger", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let trigger_id = ctx.ids.part("trigger");
                    platform.focus_element_by_id(&trigger_id);
                    no_cleanup()
                })))
            }

            // ToggleCheckboxItem: toggle checked state, don't close
            (State::Open, Event::ToggleCheckboxItem(key)) => {
                let key = key.clone();

                let current = ctx.checked_items.get(&key).copied().unwrap_or(false);

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.checked_items.insert(key, !current);
                }))
            }

            // SelectRadioItem: set radio group value, don't close
            (State::Open, Event::SelectRadioItem { group, value }) => {
                let group = group.clone();

                let value = value.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.radio_groups.insert(group, value);
                }))
            }

            // OpenSubmenu: open nested menu
            (State::Open, Event::OpenSubmenu(key)) => {
                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.submenu_open = Some(key);
                }).with_effect(PendingEffect::new("focus_submenu_content", |ctx, _props, _send| {
                    if let Some(ref sub_key) = ctx.submenu_open {
                        let platform = use_platform_effects();
                        let sub_content_id = ctx.ids.item("sub-content", &sub_key);
                        platform.focus_element_by_id(&sub_content_id);
                    }
                    no_cleanup()
                })))
            }

            // CloseSubmenu: close nested, return focus to parent item
            (State::Open, Event::CloseSubmenu) => {
                let parent = ctx.submenu_open.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.submenu_open = None;
                    ctx.highlighted_key = parent;
                }).with_effect(PendingEffect::new("focus_parent_item", |ctx, _props, _send| {
                    if let Some(ref item_key) = ctx.highlighted_key {
                        let platform = use_platform_effects();
                        let item_id = ctx.ids.item("item", &item_key);
                        platform.focus_element_by_id(&item_id);
                    }
                    no_cleanup()
                })))
            }

            // TypeaheadSearch: use typeahead::State for accumulated search
            (State::Open, Event::TypeaheadSearch(ch, now_ms)) => {
                let (new_ta, found) = ctx.typeahead.process_char(*ch, *now_ms,
                    ctx.highlighted_key.as_ref(), &ctx.items);

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.typeahead = new_ta;
                    if let Some(k) = found { ctx.highlighted_key = Some(k); }
                }).cancel_effect("typeahead_timeout")
                  .with_effect(PendingEffect::new("typeahead_timeout", |ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send = send.clone();
                    let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
                        send(Event::TypeaheadClear);
                    }));
                    let pc = platform.clone();
                    Box::new(move || { pc.clear_timeout(handle); })
                })))
            }

            // UpdateItems: dynamically replace the item collection
            (_, Event::UpdateItems(new_items)) => {
                let new_items = new_items.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.items = new_items;
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

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "menu"]
pub enum Part {
    Root,
    Trigger,
    Positioner,
    Arrow,
    Content,
    ItemGroup { key: Key },
    ItemGroupLabel { key: Key },
    Item { key: Key },
    ItemText { key: Key },
    ItemIndicator { key: Key },
    Separator,
    CheckboxItem { key: Key },
    RadioGroup { group: Key },
    RadioItem { key: Key, group: Key },
    SubTrigger { key: Key },
    SubPositioner { key: Key },
    SubContent { key: Key },
    Shortcut { key: Key },
}

/// API for the Menu component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.props.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        attrs
    }

    /// Attributes for the trigger button.
    pub fn trigger_attrs(&self) -> AttrMap {
        let trigger_id = self.ctx.ids.part("trigger");
        let content_id = self.ctx.ids.part("content");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, trigger_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        if self.props.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Attributes for the positioner wrapper.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the floating arrow.
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the menu content panel.
    pub fn content_attrs(&self) -> AttrMap {
        let content_id = self.ctx.ids.part("content");
        let trigger_id = self.ctx.ids.part("trigger");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_id);
        attrs.set(HtmlAttr::Role, "menu");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), trigger_id);
        attrs
    }

    /// Attributes for an item group container.
    pub fn item_group_attrs(&self, key: &Key) -> AttrMap {
        let label_id = self.ctx.ids.item("item-group-label", key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroup { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), label_id);
        attrs
    }

    /// Attributes for an item group label.
    pub fn item_group_label_attrs(&self, key: &Key) -> AttrMap {
        let label_id = self.ctx.ids.item("item-group-label", key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroupLabel { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, label_id);
        attrs
    }

    /// Attributes for an action menu item.
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.props.disabled_keys.contains(key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "menuitem");
        attrs.set(HtmlAttr::TabIndex, if is_highlighted { "0" } else { "-1" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        // Set aria-keyshortcuts if the item has one
        if let Some(node) = self.ctx.items.get(key) {
            if let Some(ref aks) = node.payload.aria_keyshortcuts {
                attrs.set(HtmlAttr::Aria(AriaAttr::KeyShortcuts), aks);
            }
        }
        attrs
    }

    /// Attributes for an item's text label.
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemText { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "text"));
        attrs
    }

    /// Attributes for an item's check/radio indicator.
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "indicator"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for a separator.
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "separator");
        attrs
    }

    /// Attributes for a checkbox menu item.
    pub fn checkbox_item_attrs(&self, key: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.props.disabled_keys.contains(key);
        let is_checked = self.ctx.checked_items.get(key).copied().unwrap_or(false);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CheckboxItem { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "menuitemcheckbox");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if is_checked { "true" } else { "false" });
        attrs.set(HtmlAttr::TabIndex, if is_highlighted { "0" } else { "-1" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Attributes for a radio group container.
    pub fn radio_group_attrs(&self, group: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RadioGroup { group: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs
    }

    /// Attributes for a radio menu item.
    pub fn radio_item_attrs(&self, key: &Key, group: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.props.disabled_keys.contains(key);
        let is_checked = self.ctx.radio_groups.get(group).map_or(false, |v| v == key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RadioItem { key: Key::default() , group: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "menuitemradio");
        attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if is_checked { "true" } else { "false" });
        attrs.set(HtmlAttr::TabIndex, if is_highlighted { "0" } else { "-1" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Attributes for a submenu trigger item.
    pub fn sub_trigger_attrs(&self, key: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", key);
        let sub_content_id = self.ctx.ids.item("sub-content", key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_submenu_open = self.ctx.submenu_open.as_ref() == Some(key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubTrigger { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "menuitem");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_submenu_open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), sub_content_id);
        attrs.set(HtmlAttr::TabIndex, if is_highlighted { "0" } else { "-1" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        attrs
    }

    /// Attributes for a submenu positioner wrapper.
    pub fn sub_positioner_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubPositioner { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("sub", key, "positioner"));
        attrs
    }

    /// Attributes for a submenu content panel.
    pub fn sub_content_attrs(&self, key: &Key) -> AttrMap {
        let sub_content_id = self.ctx.ids.item("sub-content", key);
        let item_id = self.ctx.ids.item("item", key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubContent { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, sub_content_id);
        attrs.set(HtmlAttr::Role, "menu");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), item_id);
        attrs
    }

    /// Handle click on the trigger button.
    pub fn on_trigger_click(&self) {
        if self.ctx.open {
            (self.send)(Event::Close);
        } else {
            (self.send)(Event::Open);
        }
    }

    /// Handle keydown on the trigger button.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowDown | KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::Open);
            }
            _ => {}
        }
    }

    /// Handle keydown on the menu content.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::HighlightNext),
            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),
            KeyboardKey::Home => (self.send)(Event::HighlightFirst),
            KeyboardKey::End => (self.send)(Event::HighlightLast),
            KeyboardKey::Escape => (self.send)(Event::Close),
            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(ref key) = self.ctx.highlighted_key {
                    if let Some(node) = self.ctx.items.get(key) {
                        match &node.payload.item_type {
                            ItemType::Normal => (self.send)(Event::SelectItem(key.clone())),
                            ItemType::Checkbox => (self.send)(Event::ToggleCheckboxItem(key.clone())),
                            ItemType::Radio { group } => (self.send)(Event::SelectRadioItem {
                                group: group.clone(),
                                value: key.clone(),
                            }),
                            ItemType::Submenu => (self.send)(Event::OpenSubmenu(key.clone())),
                        }
                    }
                }
            }
            KeyboardKey::ArrowRight => {
                if let Some(ref key) = self.ctx.highlighted_key {
                    if let Some(node) = self.ctx.items.get(key) {
                        if node.payload.item_type == ItemType::Submenu {
                            (self.send)(Event::OpenSubmenu(key.clone()));
                        }
                    }
                }
            }
            KeyboardKey::ArrowLeft => {
                if self.ctx.submenu_open.is_some() {
                    (self.send)(Event::CloseSubmenu);
                }
            }
            _ => {
                if let Some(ch) = data.key.as_printable_char() {
                    (self.send)(Event::TypeaheadSearch(ch, data.timestamp_ms));
                }
            }
        }
    }

    /// Handle click on a menu item.
    pub fn on_item_click(&self, key: &Key) {
        if let Some(node) = self.ctx.items.get(key) {
            match &node.payload.item_type {
                ItemType::Normal => (self.send)(Event::SelectItem(key.clone())),
                ItemType::Checkbox => (self.send)(Event::ToggleCheckboxItem(key.clone())),
                ItemType::Radio { group } => (self.send)(Event::SelectRadioItem {
                    group: group.clone(),
                    value: key.clone(),
                }),
                ItemType::Submenu => (self.send)(Event::OpenSubmenu(key.clone())),
            }
        }
    }

    /// Handle pointer enter on a menu item (for highlight tracking).
    pub fn on_item_pointer_enter(&self, key: &Key) {
        (self.send)(Event::HighlightItem(Some(key.clone())));
    }

    /// Handle pointer leave on the menu content (clear highlight).
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }

    /// The attributes for the shortcut element.
    pub fn item_shortcut_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Shortcut { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "shortcut"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Arrow => self.arrow_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::Separator => self.separator_attrs(),
            Part::CheckboxItem { ref key } => self.checkbox_item_attrs(key),
            Part::RadioGroup { ref group } => self.radio_group_attrs(group),
            Part::RadioItem { ref key, ref group } => self.radio_item_attrs(key, group),
            Part::SubTrigger { ref key } => self.sub_trigger_attrs(key),
            Part::SubPositioner { ref key } => self.sub_positioner_attrs(key),
            Part::SubContent { ref key } => self.sub_content_attrs(key),
            Part::Shortcut { ref key } => self.item_shortcut_attrs(key),
        }
    }
}
```

## 2. Anatomy

| Part             | Selector                                                    | Element    | Notes                                                                              |
| ---------------- | ----------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------- |
| `Root`           | `[data-ars-scope="menu"][data-ars-part="root"]`             | `<div>`    |                                                                                    |
| `Trigger`        | `[data-ars-scope="menu"][data-ars-part="trigger"]`          | `<button>` | Opens menu                                                                         |
| `Positioner`     | `[data-ars-scope="menu"][data-ars-part="positioner"]`       | `<div>`    |                                                                                    |
| `Content`        | `[data-ars-scope="menu"][data-ars-part="content"]`          | `<div>`    | Menu panel                                                                         |
| `ItemGroup`      | `[data-ars-scope="menu"][data-ars-part="item-group"]`       | `<div>`    |                                                                                    |
| `ItemGroupLabel` | `[data-ars-scope="menu"][data-ars-part="item-group-label"]` | `<div>`    |                                                                                    |
| `Item`           | `[data-ars-scope="menu"][data-ars-part="item"]`             | `<div>`    | Action item                                                                        |
| `ItemText`       | `[data-ars-scope="menu"][data-ars-part="item-text"]`        | `<span>`   |                                                                                    |
| `ItemIndicator`  | `[data-ars-scope="menu"][data-ars-part="item-indicator"]`   | `<div>`    |                                                                                    |
| `CheckboxItem`   | `[data-ars-scope="menu"][data-ars-part="checkbox-item"]`    | `<div>`    |                                                                                    |
| `RadioGroup`     | `[data-ars-scope="menu"][data-ars-part="radio-group"]`      | `<div>`    |                                                                                    |
| `RadioItem`      | `[data-ars-scope="menu"][data-ars-part="radio-item"]`       | `<div>`    |                                                                                    |
| `Separator`      | `[data-ars-scope="menu"][data-ars-part="separator"]`        | `<hr>`     |                                                                                    |
| `Arrow`          | `[data-ars-scope="menu"][data-ars-part="arrow"]`            | `<div>`    | Floating arrow                                                                     |
| `SubTrigger`     | `[data-ars-scope="menu"][data-ars-part="sub-trigger"]`      | `<div>`    | Opens sub-menu                                                                     |
| `SubContent`     | `[data-ars-scope="menu"][data-ars-part="sub-content"]`      | `<div>`    | Sub-menu panel                                                                     |
| `SubPositioner`  | `[data-ars-scope="menu"][data-ars-part="sub-positioner"]`   | `<div>`    |                                                                                    |
| `Shortcut`       | `[data-ars-scope="menu"][data-ars-part="shortcut"]`         | `<kbd>`    | Rendered when `Item.shortcut` is `Some`. Displayed right-aligned after `ItemText`. |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- The shortcut text is marked `aria-hidden="true"` because it is visual-only. Screen readers announce the menu item label; the shortcut hint is a visual convenience for sighted users.
- The actual keyboard shortcut binding is NOT handled by the Menu component — it is the consumer's responsibility to wire global keyboard handlers.
- For accessibility, consumers should also provide a keyboard shortcuts help dialog or documentation.

Menu items with keyboard shortcuts SHOULD set `aria-keyshortcuts` on the `menuitem` element (e.g., `aria-keyshortcuts="Control+S"`). The visual `<kbd>` shortcut text remains `aria-hidden="true"` to avoid double-announcement, but the shortcut is exposed semantically via `aria-keyshortcuts`. `menu::Item` includes an optional `aria_keyshortcuts: Option<String>` field.

**Focus Management**: Menu uses **roving tabindex** for item navigation (consistent with MenuBar §6). When the menu opens, focus moves to the Content element, then to the first enabled item. Each highlighted item receives `tabindex="0"` and `element.focus()`; all other items have `tabindex="-1"`. This ensures compatibility with all screen readers including VoiceOver iOS, which does not support `aria-activedescendant`.

| Property          | Element        | Value                                                               |
| ----------------- | -------------- | ------------------------------------------------------------------- |
| `aria-haspopup`   | `Trigger`      | `menu`                                                              |
| `aria-expanded`   | `Trigger`      | `true`/`false`                                                      |
| `aria-controls`   | `Trigger`      | Content id                                                          |
| `role`            | `Content`      | `menu`                                                              |
| `tabindex`        | `Content`      | `"-1"` (focusable container, focus immediately moves to first item) |
| `aria-labelledby` | `Content`      | Trigger id                                                          |
| `role`            | `Item`         | `menuitem`                                                          |
| `tabindex`        | `Item`         | `"0"` when highlighted, `"-1"` otherwise                            |
| `role`            | `CheckboxItem` | `menuitemcheckbox`                                                  |
| `aria-checked`    | `CheckboxItem` | `true`/`false`                                                      |
| `role`            | `RadioItem`    | `menuitemradio`                                                     |
| `aria-checked`    | `RadioItem`    | `true`/`false`                                                      |
| `role`            | `Separator`    | `separator`                                                         |
| `role`            | `ItemGroup`    | `group`                                                             |
| `aria-haspopup`   | `SubTrigger`   | `menu`                                                              |
| `aria-expanded`   | `SubTrigger`   | `true` when submenu open, `false` otherwise                         |

**Menu Item Types**:

1. Action items: `role='menuitem'`.
2. `Checkbox` items: `role='menuitemcheckbox'` with `aria-checked`.
3. `Radio` items: `role='menuitemradio'` with `aria-checked`.
4. `Item` type is an explicit prop, not inferred.
5. Mixed types in the same menu are valid and follow WAI-ARIA MenuBar pattern.
6. Screen readers announce the item type automatically from the role.

### 3.2 Keyboard Interaction

| Key                | Action                                |
| ------------------ | ------------------------------------- |
| ArrowDown          | Highlight next item                   |
| ArrowUp            | Highlight previous item               |
| Enter / Space      | Activate highlighted item             |
| ArrowRight         | Open submenu (on submenu trigger)     |
| ArrowLeft / Escape | Close submenu → parent; or close menu |
| Home               | Highlight first item                  |
| End                | Highlight last item                   |
| a-z                | Typeahead highlight                   |

**Typeahead**: Single printable character jumps to the next item starting with
that character. If additional characters are typed within 500ms, the search
accumulates (e.g., "abc" matches "Abc item"). After 500ms timeout, buffer resets.

### 3.3 Focus Management

When a submenu opens, focus MUST move to the **first enabled item** in the submenu
content. This is enforced by the `focus_submenu_content` effect in the `OpenSubmenu`
transition (see above).

- **Focus on open**: The `PendingEffect` focuses the submenu's first item, not the
  container. This ensures screen readers announce the first actionable item.
- **Escape closes current submenu only**: Pressing Escape closes the innermost open
  submenu and returns focus to its parent trigger item. It does NOT close the entire
  menu stack. Each submenu level handles Escape independently via `CloseSubmenu`.
- **`aria-haspopup="menu"`**: Parent items that trigger submenus (SubTrigger parts) MUST
  set `aria-haspopup="menu"` to indicate a nested menu is available.
- **`aria-expanded` announcements**: Changes to `aria-expanded` on SubTrigger items are
  automatically announced by screen readers. The adapter MUST update `aria-expanded`
  synchronously with the submenu open/close state.

### 3.4 Submenu Nesting

Submenu nesting follows these rules for consistent behavior:

- **Open on ArrowRight**: When a `SubTrigger` item is highlighted, pressing ArrowRight
  opens the submenu after a **200ms delay** (to prevent accidental opens during
  navigation). The delay is skipped when the submenu was already opened and closed
  within the last 300ms (hysteresis).
- **Escape closes one level**: Each Escape press closes only the current submenu level
  and returns focus to the parent item. The menu stack unwinds one level at a time.
- **Focus transfer**: On submenu open, focus transfers to the first enabled item in
  the submenu (see Submenu Focus Management above).
- **Depth recommendation**: While there is no strict depth limit enforced by the machine,
  nesting beyond **3 levels** is discouraged for UX reasons. Deeply nested menus are
  difficult to navigate, especially for keyboard and screen reader users. Consider
  alternative patterns (e.g., a panel or dialog) for deeply nested structures.

## 4. Internationalization

- **RTL**: ArrowRight opens submenu in LTR; ArrowLeft opens submenu in RTL.
  Submenu positioning also flips (see §1.5.1 Submenu Positioning table).
- **Typeahead**: Locale-aware via `Collator`.
- **Separator**: Decorative — no localization needed.
- **Shortcut text**: Displayed after item label (e.g., "Ctrl+C") — follows OS conventions.

## 5. Variant: Keyboard Shortcut Display

Menu items may display a keyboard shortcut hint alongside the item label.
The `shortcut` and `aria_keyshortcuts` fields are on `Item`.

### 5.1 Anatomy Additions

The `Shortcut` part is a child of `Item`:

```text
Item
├── ItemIndicator  (for checkable items)
├── ItemText       (label)
└── Shortcut         (<kbd> or <span>; right-aligned shortcut text)
```

| Part       | Element             | Attributes                                       |
| ---------- | ------------------- | ------------------------------------------------ |
| `Shortcut` | `<kbd>` or `<span>` | `data-ars-part="shortcut"`, `aria-hidden="true"` |

### 5.2 Accessibility

The shortcut text is marked `aria-hidden="true"` because it is visual-only; screen readers
should not double-announce the shortcut alongside the item label. The semantic shortcut is
exposed via `aria-keyshortcuts` on the parent `menuitem` element (see `Item.aria_keyshortcuts`
in §1). The `item_shortcut_attrs()` method in §1.7 Connect / API returns the required
attributes for the `Shortcut` part.

## 6. Library Parity

> Compared against: Ark UI (`Menu`), Radix UI (`DropdownMenu`), React Aria (`Menu`).

### 6.1 Props

| Feature                         | ars-ui                                         | Ark UI                              | Radix UI                                | React Aria                            | Notes                                                                   |
| ------------------------------- | ---------------------------------------------- | ----------------------------------- | --------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------- |
| Disabled                        | `disabled`                                     | --                                  | --                                      | --                                    | ars-ui global disable; others per-item only                             |
| Disabled keys                   | `disabled_keys`                                | --                                  | per-item `disabled`                     | `disabledKeys`                        | --                                                                      |
| Loop focus                      | `loop_focus`                                   | `loopFocus`                         | Content `loop`                          | `shouldFocusWrap`                     | --                                                                      |
| Close on action                 | `close_on_action` + per-item `close_on_action` | `closeOnSelect`                     | --                                      | `shouldCloseOnSelect` + per-item      | --                                                                      |
| Positioning                     | `positioning`                                  | `positioning`                       | Content `side`/`align`/etc.             | via `Popover`                         | --                                                                      |
| Open state control              | `on_open_change`                               | `open`/`defaultOpen`/`onOpenChange` | `open`/`defaultOpen`/`onOpenChange`     | `isOpen`/`defaultOpen`/`onOpenChange` | ars-ui uses callback-only                                               |
| On action                       | `on_action`                                    | `onSelect`                          | per-item `onSelect`                     | `onAction`                            | --                                                                      |
| Typeahead                       | Yes (via `typeahead::State`)                   | `typeahead`                         | Yes (built-in)                          | Yes (built-in)                        | --                                                                      |
| Selection mode (checkbox/radio) | `ItemType::Checkbox`/`Radio`                   | `CheckboxItem`/`RadioItem`          | `CheckboxItem`/`RadioGroup`/`RadioItem` | `selectionMode` + `selectedKeys`      | --                                                                      |
| Modal mode                      | --                                             | --                                  | `modal`                                 | --                                    | Radix exclusive; ars-ui uses overlay layer                              |
| Disabled behavior               | `disabled_behavior`                            | --                                  | --                                      | --                                    | ars-ui exclusive (skip vs focus-only)                                   |
| Disallow empty selection        | `disallow_empty_selection`                     | --                                  | --                                      | `disallowEmptySelection`              | --                                                                      |
| Trigger method                  | button click                                   | button click                        | button click                            | `trigger: 'press' \| 'longPress'`     | React Aria supports long-press trigger                                  |
| Anchor point                    | --                                             | `anchorPoint`                       | --                                      | --                                    | Ark UI exclusive; ars-ui uses context-menu for pointer-positioned menus |
| Escape key behavior             | --                                             | --                                  | --                                      | `escapeKeyBehavior`                   | React Aria only; ars-ui always closes                                   |

**Gaps:** None. React Aria's `longPress` trigger is a niche UX pattern better suited to mobile; Ark UI's `anchorPoint` is handled by ars-ui's ContextMenu component.

### 6.2 Anatomy

| Part           | ars-ui           | Ark UI                  | Radix UI        | React Aria          | Notes            |
| -------------- | ---------------- | ----------------------- | --------------- | ------------------- | ---------------- |
| Root           | `Root`           | `Root`                  | `Root`          | `MenuTrigger`       | --               |
| Trigger        | `Trigger`        | `Trigger`               | `Trigger`       | `Button`            | --               |
| Positioner     | `Positioner`     | `Positioner`            | `Portal`        | `Popover`           | --               |
| Arrow          | `Arrow`          | `Arrow` + `ArrowTip`    | `Arrow`         | `OverlayArrow`      | --               |
| Content        | `Content`        | `Content`               | `Content`       | `Menu`              | --               |
| Item           | `Item`           | `Item`                  | `Item`          | `MenuItem`          | --               |
| ItemText       | `ItemText`       | `ItemText`              | --              | --                  | --               |
| ItemIndicator  | `ItemIndicator`  | `ItemIndicator`         | `ItemIndicator` | --                  | --               |
| ItemGroup      | `ItemGroup`      | `ItemGroup`             | `Group`         | `MenuSection`       | --               |
| ItemGroupLabel | `ItemGroupLabel` | `ItemGroupLabel`        | `Label`         | `Header`            | --               |
| CheckboxItem   | `CheckboxItem`   | `CheckboxItem`          | `CheckboxItem`  | via `selectionMode` | --               |
| RadioGroup     | `RadioGroup`     | `RadioItemGroup`        | `RadioGroup`    | via `selectionMode` | --               |
| RadioItem      | `RadioItem`      | `RadioItem`             | `RadioItem`     | via `MenuItem`      | --               |
| Separator      | `Separator`      | `Separator`             | `Separator`     | `Separator`         | --               |
| SubTrigger     | `SubTrigger`     | `TriggerItem`           | `SubTrigger`    | `SubmenuTrigger`    | --               |
| SubContent     | `SubContent`     | -- (nested `Menu.Root`) | `SubContent`    | nested `Menu`       | --               |
| SubPositioner  | `SubPositioner`  | --                      | --              | --                  | ars-ui exclusive |
| Shortcut       | `Shortcut`       | --                      | --              | `Keyboard`          | --               |

**Gaps:** None.

### 6.3 Events

| Callback         | ars-ui                        | Ark UI              | Radix UI                                                        | React Aria          | Notes                      |
| ---------------- | ----------------------------- | ------------------- | --------------------------------------------------------------- | ------------------- | -------------------------- |
| Item action      | `on_action`                   | `onSelect`          | per-item `onSelect`                                             | `onAction`          | --                         |
| Open change      | `on_open_change`              | `onOpenChange`      | `onOpenChange`                                                  | `onOpenChange`      | --                         |
| Highlight change | via `Context.highlighted_key` | `onHighlightChange` | --                                                              | --                  | ars-ui exposes via context |
| Close            | `Event::Close`                | `onExitComplete`    | --                                                              | `onClose`           | --                         |
| Selection change | via checkbox/radio context    | --                  | via `CheckboxItem.onCheckedChange` / `RadioGroup.onValueChange` | `onSelectionChange` | --                         |

**Gaps:** None.

### 6.4 Features

| Feature                    | ars-ui                | Ark UI | Radix UI    | React Aria       |
| -------------------------- | --------------------- | ------ | ----------- | ---------------- |
| Action items               | Yes                   | Yes    | Yes         | Yes              |
| Checkbox items             | Yes                   | Yes    | Yes         | Yes              |
| Radio items                | Yes                   | Yes    | Yes         | Yes              |
| Submenus                   | Yes                   | Yes    | Yes         | Yes              |
| Typeahead                  | Yes                   | Yes    | Yes         | Yes              |
| Keyboard shortcuts display | Yes (`Shortcut` part) | --     | --          | Yes (`Keyboard`) |
| Safe area (hover bridge)   | Yes                   | Yes    | Yes         | Yes              |
| Disabled items             | Yes                   | Yes    | Yes         | Yes              |
| Separator                  | Yes                   | Yes    | Yes         | Yes              |
| RTL                        | Yes                   | Yes    | Yes (`dir`) | Yes              |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui separates `Menu` and `ContextMenu` into distinct components; Ark UI uses a single `Menu` with `ContextTrigger`; (2) ars-ui uses `ItemType` enum to distinguish item types rather than separate component types; (3) React Aria uses `selectionMode` + `selectedKeys` for checkbox/radio items rather than explicit CheckboxItem/RadioItem parts.
- **Recommended additions:** None.
