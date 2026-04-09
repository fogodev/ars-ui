---
component: ContextMenu
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Menu
    radix-ui: ContextMenu
---

# ContextMenu

A `Menu` triggered by right-click (`contextmenu` event) rather than a button click. Positioned
at the pointer location. Shares Menu's core item model, navigation, and selection semantics but
differs in trigger mechanism (right-click vs button), positioning (pointer coordinates vs trigger
anchor), and part naming (Target replaces Trigger).

Items are stored as a `StaticCollection<menu::Item>` (from `06-collections.md`). Separators
are structural `NodeType::Separator` nodes. Groups are `NodeType::Section` + `NodeType::Header`
nodes. Navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`.
Typeahead uses `typeahead::State`.

## 1. State Machine

### 1.1 States

```rust
/// The states of the ContextMenu state machine.
/// Same states as Menu — closed or open with highlight tracking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The context menu is closed.
    Closed,
    /// The context menu is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the ContextMenu state machine.
/// Menu's events plus the context-menu-specific open event.
#[derive(Clone, Debug)]
pub enum Event {
    /// Open the context menu at pointer coordinates.
    ContextOpen { x: f64, y: f64 },
    /// Close the menu.
    Close,
    /// Highlight an item by key (None clears highlight).
    HighlightItem(Option<Key>),
    /// Highlight the first enabled item.
    HighlightFirst,
    /// Highlight the last enabled item.
    HighlightLast,
    /// Highlight the next enabled item.
    HighlightNext,
    /// Highlight the previous enabled item.
    HighlightPrev,
    /// Select (activate) a normal menu item.
    SelectItem(Key),
    /// Toggle a checkbox item.
    ToggleCheckboxItem(Key),
    /// Select a radio item within a group.
    SelectRadioItem {
        /// The radio group name.
        group: String,
        /// The key of the radio item.
        value: Key,
    },
    /// Click outside the menu.
    ClickOutside,
    /// Typeahead search with character and timestamp (ms).
    TypeaheadSearch(char, u64),
    /// Update the item collection dynamically.
    UpdateItems(StaticCollection<menu::Item>),
}
```

### 1.3 Context

```rust
/// The context of the ContextMenu state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The items of the context menu.
    pub items: StaticCollection<menu::Item>,
    /// Whether the context menu is open.
    pub open: bool,
    /// The highlighted key of the context menu.
    pub highlighted_key: Option<Key>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The checked items of the context menu.
    pub checked_items: BTreeMap<Key, bool>,
    /// The radio groups of the context menu.
    pub radio_groups: BTreeMap<String, Key>,
    /// The typeahead state of the context menu.
    pub typeahead: typeahead::State,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    pub loop_focus: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// The pointer position where the context menu was opened.
    /// Set by `ContextOpen` and used by the positioner to anchor the floating content.
    pub position: Option<(f64, f64)>,
}
```

### 1.4 Props

```rust
/// Props for the ContextMenu state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the context menu.
    pub id: String,
    /// Whether the context menu is disabled.
    pub disabled: bool,
    /// Whether the menu closes when an item is activated (selected or triggered).
    /// Applies to both selectable items (checkbox/radio) and action items (normal).
    /// Individual `menu::Item.close_on_action` overrides this per-item.
    /// Default: `true`.
    pub close_on_action: bool,
    /// Whether the focus loops around from the last item back to the first (and vice versa).
    /// Default: `true`.
    pub loop_focus: bool,
    /// How disabled items behave in keyboard navigation.
    /// `Skip` = disabled items are skipped during keyboard navigation (not focusable, not selectable).
    /// `FocusOnly` = disabled items are focusable but not selectable.
    /// Default: `DisabledBehavior::Skip`.
    pub disabled_behavior: DisabledBehavior,
    /// Set of keys for menu items that are disabled.
    /// Disabled items are skipped during keyboard navigation and cannot be selected or triggered.
    pub disabled_keys: BTreeSet<Key>,
    /// Callback invoked when the context menu open state changes.
    /// Fires after the transition completes with the new open state value.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when a menu item is activated (Enter/click on action items).
    /// Distinct from selection-change callbacks — `on_action` fires for command execution,
    /// not for checkbox/radio state toggling.
    pub on_action: Option<Callback<Key>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            close_on_action: true,
            loop_focus: true,
            disabled_behavior: DisabledBehavior::Skip,
            disabled_keys: BTreeSet::new(),
            on_open_change: None,
            on_action: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

The key difference from Menu: `ContextOpen { x, y }` sets position and transitions to Open.
There is no Trigger-based open — only the `contextmenu` event (right-click) or `Shift+F10`
opens the menu. On close, focus returns to the Target element rather than a trigger button.

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

    fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
        let ctx = Context {
            items: StaticCollection::empty(),
            open: false,
            highlighted_key: None,
            focus_visible: false,
            checked_items: BTreeMap::new(),
            radio_groups: BTreeMap::new(),
            typeahead: typeahead::State::new(),
            loop_focus: props.loop_focus,
            ids: ComponentIds::from_id(&props.id),
            position: None,
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
            // ContextOpen: open at pointer position, highlight first item
            (State::Closed, Event::ContextOpen { x, y }) => {
                let pos = (*x, *y);
                let first = first_enabled_key(
                    &ctx.items, &props.disabled_keys, props.disabled_behavior,
                );

                Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                    ctx.position = Some(pos);
                    ctx.open = true;
                    ctx.highlighted_key = first;
                }).with_effect(PendingEffect::new("position_at_point", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    if let Some((x, y)) = ctx.position {
                        let content_id = ctx.ids.part("content");
                        platform.position_element_at(&content_id, x, y);
                        platform.focus_element_by_id(&content_id);
                    }
                    no_cleanup()
                })))
            }

            // Re-open at new position if already open (e.g., right-click while open)
            (State::Open, Event::ContextOpen { x, y }) => {
                let pos = (*x, *y);
                let first = first_enabled_key(
                    &ctx.items, &props.disabled_keys, props.disabled_behavior,
                );

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.position = Some(pos);
                    ctx.highlighted_key = first;
                }).with_effect(PendingEffect::new("reposition_at_point", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    if let Some((x, y)) = ctx.position {
                        let content_id = ctx.ids.part("content");
                        platform.position_element_at(&content_id, x, y);
                        platform.focus_element_by_id(&content_id);
                    }
                    no_cleanup()
                })))
            }

            // Close / ClickOutside: close menu, return focus to target
            (State::Open, Event::Close) | (State::Open, Event::ClickOutside) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                    ctx.position = None;
                }).with_effect(PendingEffect::new("focus_target", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let target_id = ctx.ids.part("target");
                    platform.focus_element_by_id(&target_id);
                    no_cleanup()
                })))
            }

            // HighlightNext / HighlightPrev: navigate enabled items (only when open)
            (State::Open, Event::HighlightNext) => {
                let next = match &ctx.highlighted_key {
                    Some(k) => next_enabled_key(
                        &ctx.items, k, &props.disabled_keys,
                        props.disabled_behavior, ctx.loop_focus,
                    ),
                    None => first_enabled_key(
                        &ctx.items, &props.disabled_keys, props.disabled_behavior,
                    ),
                };

                next.map(|k| TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = Some(k);
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let prev = match &ctx.highlighted_key {
                    Some(k) => prev_enabled_key(
                        &ctx.items, k, &props.disabled_keys,
                        props.disabled_behavior, ctx.loop_focus,
                    ),
                    None => last_enabled_key(
                        &ctx.items, &props.disabled_keys, props.disabled_behavior,
                    ),
                };

                prev.map(|k| TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = Some(k);
                }))
            }

            (_, Event::HighlightFirst) => {
                let first = first_enabled_key(
                    &ctx.items, &props.disabled_keys, props.disabled_behavior,
                );

                first.map(|k| TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = Some(k);
                }))
            }

            (_, Event::HighlightLast) => {
                let last = last_enabled_key(
                    &ctx.items, &props.disabled_keys, props.disabled_behavior,
                );

                last.map(|k| TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = Some(k);
                }))
            }

            (_, Event::HighlightItem(key)) => {
                // Guard: ignore highlight requests for keys not present in the collection.
                if let Some(ref k) = key {
                    if !ctx.items.contains_key(k) { return None; }
                }
                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = key;
                }))
            }

            // SelectItem: fire action, optionally close menu
            (State::Open, Event::SelectItem(key)) => {
                if ctx.items.get(key).map_or(true, |n|
                    n.node_type != NodeType::Item) { return None; }

                let should_close = ctx.items.get(key)
                    .and_then(|n| n.payload.close_on_action)
                    .unwrap_or(props.close_on_action);

                if should_close {
                    Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.position = None;
                    }).with_effect(PendingEffect::new("focus_target", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let target_id = ctx.ids.part("target");
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
                } else {
                    Some(TransitionPlan::context_only(|_ctx| {}))
                }
            }

            // ToggleCheckboxItem: toggle checked state
            (State::Open, Event::ToggleCheckboxItem(key)) => {
                let key = key.clone();
                let current = ctx.checked_items.get(&key).copied().unwrap_or(false);

                let should_close = ctx.items.get(&key)
                    .and_then(|n| n.payload.close_on_action)
                    .unwrap_or(props.close_on_action);

                if should_close {
                    Some(TransitionPlan::to(State::Closed).apply(move |ctx| {
                        ctx.checked_items.insert(key, !current);
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.position = None;
                    }).with_effect(PendingEffect::new("focus_target", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let target_id = ctx.ids.part("target");
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.checked_items.insert(key, !current);
                    }))
                }
            }

            // SelectRadioItem: set radio group value
            (State::Open, Event::SelectRadioItem { group, value }) => {
                let group = group.clone();
                let value = value.clone();

                let should_close = ctx.items.get(&value)
                    .and_then(|n| n.payload.close_on_action)
                    .unwrap_or(props.close_on_action);

                if should_close {
                    Some(TransitionPlan::to(State::Closed).apply(move |ctx| {
                        ctx.radio_groups.insert(group, value);
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.position = None;
                    }).with_effect(PendingEffect::new("focus_target", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let target_id = ctx.ids.part("target");
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.radio_groups.insert(group, value);
                    }))
                }
            }

            // TypeaheadSearch: use typeahead::State for accumulated search
            (State::Open, Event::TypeaheadSearch(ch, now_ms)) => {
                let (new_ta, found) = ctx.typeahead.process_char(
                    *ch, *now_ms, ctx.highlighted_key.as_ref(), &ctx.items,
                );

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.typeahead = new_ta;
                    if let Some(k) = found { ctx.highlighted_key = Some(k); }
                }).with_effect(PendingEffect::new("typeahead_timeout", |ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send = send.clone();
                    let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
                        // Typeahead buffer reset handled by typeahead::State internally
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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "context-menu"]
pub enum Part {
    Root,
    Target,
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
    RadioItem { key: Key },
}

/// API for the ContextMenu component.
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

    /// Attributes for the target element (receives the contextmenu event).
    pub fn target_attrs(&self) -> AttrMap {
        let target_id = self.ctx.ids.part("target");
        let content_id = self.ctx.ids.part("content");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Target.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, target_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        if self.props.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Attributes for the positioner wrapper (anchored to pointer coordinates).
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
        let target_id = self.ctx.ids.part("target");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_id);
        attrs.set(HtmlAttr::Role, "menu");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), target_id);
        attrs
    }

    /// Attributes for an item group container.
    pub fn item_group_attrs(&self, key: &Key) -> AttrMap {
        let label_id = self.ctx.ids.item("item-group-label", &key);
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
        let label_id = self.ctx.ids.item("item-group-label", &key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroupLabel { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, label_id);
        attrs
    }

    /// Attributes for an action menu item.
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let item_id = self.ctx.ids.item("item", &key);
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", &key, "text"));
        attrs
    }

    /// Attributes for an item's check/radio indicator.
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", &key, "indicator"));
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
        let item_id = self.ctx.ids.item("item", &key);
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
        let item_id = self.ctx.ids.item("item", &key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.props.disabled_keys.contains(key);
        let is_checked = self.ctx.radio_groups.get(group).map_or(false, |v| v == key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RadioItem { key: Key::default() }.data_attrs();
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

    /// Handle contextmenu event on the target element.
    pub fn on_target_contextmenu(&self, x: f64, y: f64) {
        if !self.props.disabled {
            (self.send)(Event::ContextOpen { x, y });
        }
    }

    /// Handle keydown on the target element (Shift+F10 to open).
    pub fn on_target_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::F10 && data.shift {
            if !self.props.disabled {
                // Use target element center as position when opened via keyboard
                // Adapter resolves actual coordinates from the target's bounding rect.
                (self.send)(Event::ContextOpen { x: 0.0, y: 0.0 });
            }
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
                            menu::ItemType::Normal => (self.send)(Event::SelectItem(key.clone())),
                            menu::ItemType::Checkbox => (self.send)(Event::ToggleCheckboxItem(key.clone())),
                            menu::ItemType::Radio { group } => (self.send)(Event::SelectRadioItem {
                                group: group.clone(),
                                value: key.clone(),
                            }),
                            menu::ItemType::Submenu => {} // ContextMenu does not support submenus
                        }
                    }
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
    pub fn on_item_click(&self, key: &str) {
        let key = Key::from(key);
        if let Some(node) = self.ctx.items.get(&key) {
            match &node.payload.item_type {
                menu::ItemType::Normal => (self.send)(Event::SelectItem(key)),
                menu::ItemType::Checkbox => (self.send)(Event::ToggleCheckboxItem(key)),
                menu::ItemType::Radio { group } => (self.send)(Event::SelectRadioItem {
                    group: group.clone(),
                    value: key,
                }),
                menu::ItemType::Submenu => {} // ContextMenu does not support submenus
            }
        }
    }

    /// Handle pointer enter on a menu item (for highlight tracking).
    pub fn on_item_pointer_enter(&self, key: &str) {
        (self.send)(Event::HighlightItem(Some(Key::from(key))));
    }

    /// Handle pointer leave on the menu content (clear highlight).
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Target => self.target_attrs(),
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
            Part::RadioItem { ref key } => self.radio_item_attrs(key, &Key::default()),
        }
    }
}
```

## 2. Anatomy

```text
ContextMenu
├── Root
├── Target                (the element receiving the contextmenu event)
├── Positioner            (floating, anchored to {x, y} pointer coordinates)
│   ├── Arrow             (optional)
│   └── Content           (role="menu")
│       ├── ItemGroup     (×N, optional)
│       │   ├── ItemGroupLabel
│       │   └── Item      (×N)  role="menuitem"
│       │       ├── ItemText
│       │       └── ItemIndicator (optional)
│       ├── CheckboxItem  (×N)  role="menuitemcheckbox"
│       │   ├── ItemText
│       │   └── ItemIndicator
│       ├── RadioGroup    (×N)
│       │   └── RadioItem (×N)  role="menuitemradio"
│       │       ├── ItemText
│       │       └── ItemIndicator
│       └── Separator
```

| Part             | Selector                                                            | Element  | Notes                                                 |
| ---------------- | ------------------------------------------------------------------- | -------- | ----------------------------------------------------- |
| `Root`           | `[data-ars-scope="context-menu"][data-ars-part="root"]`             | `<div>`  | Wrapper container                                     |
| `Target`         | `[data-ars-scope="context-menu"][data-ars-part="target"]`           | `<div>`  | Element receiving `contextmenu` event and `Shift+F10` |
| `Positioner`     | `[data-ars-scope="context-menu"][data-ars-part="positioner"]`       | `<div>`  | Anchored to pointer `(x, y)`                          |
| `Arrow`          | `[data-ars-scope="context-menu"][data-ars-part="arrow"]`            | `<div>`  | Optional floating arrow                               |
| `Content`        | `[data-ars-scope="context-menu"][data-ars-part="content"]`          | `<div>`  | Menu panel (`role="menu"`)                            |
| `ItemGroup`      | `[data-ars-scope="context-menu"][data-ars-part="item-group"]`       | `<div>`  | `role="group"`                                        |
| `ItemGroupLabel` | `[data-ars-scope="context-menu"][data-ars-part="item-group-label"]` | `<div>`  | Labels a group                                        |
| `Item`           | `[data-ars-scope="context-menu"][data-ars-part="item"]`             | `<div>`  | Action item (`role="menuitem"`)                       |
| `ItemText`       | `[data-ars-scope="context-menu"][data-ars-part="item-text"]`        | `<span>` | Item label text                                       |
| `ItemIndicator`  | `[data-ars-scope="context-menu"][data-ars-part="item-indicator"]`   | `<div>`  | Check/radio indicator                                 |
| `CheckboxItem`   | `[data-ars-scope="context-menu"][data-ars-part="checkbox-item"]`    | `<div>`  | `role="menuitemcheckbox"`                             |
| `RadioGroup`     | `[data-ars-scope="context-menu"][data-ars-part="radio-group"]`      | `<div>`  | `role="group"`                                        |
| `RadioItem`      | `[data-ars-scope="context-menu"][data-ars-part="radio-item"]`       | `<div>`  | `role="menuitemradio"`                                |
| `Separator`      | `[data-ars-scope="context-menu"][data-ars-part="separator"]`        | `<hr>`   | `role="separator"`                                    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

ContextMenu uses **roving tabindex** for item navigation (consistent with Menu). When the
context menu opens, focus moves to the Content element, then to the first enabled item. Each
highlighted item receives `tabindex="0"` and `element.focus()`; all other items have
`tabindex="-1"`. This ensures compatibility with all screen readers including VoiceOver iOS,
which does not support `aria-activedescendant`.

| Property          | Element         | Value                                                               |
| ----------------- | --------------- | ------------------------------------------------------------------- |
| `aria-haspopup`   | `Target`        | `"menu"` — indicates a context menu is available                    |
| `aria-expanded`   | `Target`        | `"true"` when open, `"false"` when closed                           |
| `aria-controls`   | `Target`        | Content element id                                                  |
| `role`            | `Content`       | `"menu"`                                                            |
| `tabindex`        | `Content`       | `"-1"` (focusable container, focus immediately moves to first item) |
| `aria-labelledby` | `Content`       | Target element id                                                   |
| `role`            | `Item`          | `"menuitem"`                                                        |
| `tabindex`        | `Item`          | `"0"` when highlighted, `"-1"` otherwise                            |
| `role`            | `CheckboxItem`  | `"menuitemcheckbox"`                                                |
| `aria-checked`    | `CheckboxItem`  | `"true"` / `"false"`                                                |
| `role`            | `RadioItem`     | `"menuitemradio"`                                                   |
| `aria-checked`    | `RadioItem`     | `"true"` / `"false"`                                                |
| `role`            | `Separator`     | `"separator"`                                                       |
| `role`            | `ItemGroup`     | `"group"`                                                           |
| `aria-labelledby` | `ItemGroup`     | ItemGroupLabel element id                                           |
| `aria-hidden`     | `Arrow`         | `"true"`                                                            |
| `aria-hidden`     | `ItemIndicator` | `"true"`                                                            |

**Menu Item Types** (same as Menu):

1. Action items: `role="menuitem"`.
2. Checkbox items: `role="menuitemcheckbox"` with `aria-checked`.
3. Radio items: `role="menuitemradio"` with `aria-checked`.
4. Item type is an explicit prop on `menu::Item`, not inferred.
5. Mixed types in the same context menu are valid and follow WAI-ARIA Menu pattern.

### 3.2 Keyboard Interaction

| Key              | Action                                                                |
| ---------------- | --------------------------------------------------------------------- |
| Shift+F10        | Open context menu at target element position (when target is focused) |
| Context Menu key | Open context menu at target element position (platform-native)        |
| ArrowDown        | Highlight next enabled item                                           |
| ArrowUp          | Highlight previous enabled item                                       |
| Enter / Space    | Activate highlighted item (select, toggle checkbox, or select radio)  |
| Escape           | Close context menu, return focus to target                            |
| Home             | Highlight first enabled item                                          |
| End              | Highlight last enabled item                                           |
| a-z              | Typeahead highlight — jumps to next item starting with that character |

**Typeahead**: Single printable character jumps to the next item starting with
that character. If additional characters are typed within 500ms, the search
accumulates (e.g., "abc" matches "Abc item"). After 500ms timeout, buffer resets.

**Right-click (contextmenu event)**: The browser's native `contextmenu` event is prevented
and the custom context menu is shown at the pointer coordinates. If the context menu is
already open, it repositions to the new coordinates.

### 3.3 Focus Management

- **On open**: Focus moves to the Content element, then immediately to the first enabled
  item via the `position_at_point` effect. The positioner anchors at the `(x, y)` coordinates
  from the pointer event.
- **On close**: Focus returns to the Target element via the `focus_target` effect. This
  applies to all close triggers: Escape, clicking outside, and selecting an action item
  (when `close_on_action` is true).
- **Shift+F10 open**: When opened via keyboard (`Shift+F10`), the adapter resolves the
  target element's bounding rectangle center as the `(x, y)` position. The menu appears
  near the focused target rather than at arbitrary coordinates.
- **Roving tabindex**: The highlighted item has `tabindex="0"`, all others `tabindex="-1"`.
  ArrowDown/ArrowUp cycle through enabled items. When `loop_focus` is true, navigation
  wraps from last to first and vice versa.

## 4. Internationalization

- **RTL**: No arrow key direction reversal needed — ContextMenu does not have submenu
  navigation. ArrowDown/ArrowUp always mean next/previous regardless of text direction.
- **Typeahead**: Locale-aware via `Collator` for character matching.
- **Separator**: Decorative — no localization needed.

ContextMenu generates no user-visible text, so no Messages struct is required. All labels
are consumer-provided via `menu::Item.label`.

## 5. Library Parity

> Compared against: Ark UI (`Menu` with `ContextTrigger`), Radix UI (`ContextMenu`).

### 5.1 Props

| Feature           | ars-ui              | Ark UI          | Radix UI            | Notes                                       |
| ----------------- | ------------------- | --------------- | ------------------- | ------------------------------------------- |
| Disabled          | `disabled`          | --              | Trigger `disabled`  | --                                          |
| Close on action   | `close_on_action`   | `closeOnSelect` | --                  | --                                          |
| Loop focus        | `loop_focus`        | `loopFocus`     | Content `loop`      | --                                          |
| Disabled keys     | `disabled_keys`     | --              | per-item `disabled` | --                                          |
| Disabled behavior | `disabled_behavior` | --              | --                  | ars-ui exclusive                            |
| On open change    | `on_open_change`    | `onOpenChange`  | `onOpenChange`      | --                                          |
| On action         | `on_action`         | `onSelect`      | per-item `onSelect` | --                                          |
| Modal mode        | --                  | --              | `modal`             | Radix exclusive                             |
| Direction         | --                  | --              | `dir`               | Radix explicit; ars-ui resolves from locale |

**Gaps:** None.

### 5.2 Anatomy

| Part           | ars-ui           | Ark UI                   | Radix UI                            | Notes                                                        |
| -------------- | ---------------- | ------------------------ | ----------------------------------- | ------------------------------------------------------------ |
| Root           | `Root`           | `Root`                   | `Root`                              | --                                                           |
| Target         | `Target`         | `ContextTrigger`         | `Trigger`                           | ars-ui names it `Target` to distinguish from button triggers |
| Positioner     | `Positioner`     | `Positioner`             | `Portal`                            | --                                                           |
| Arrow          | `Arrow`          | `Arrow` + `ArrowTip`     | `Arrow`                             | --                                                           |
| Content        | `Content`        | `Content`                | `Content`                           | --                                                           |
| Item           | `Item`           | `Item`                   | `Item`                              | --                                                           |
| ItemText       | `ItemText`       | `ItemText`               | --                                  | --                                                           |
| ItemIndicator  | `ItemIndicator`  | `ItemIndicator`          | `ItemIndicator`                     | --                                                           |
| ItemGroup      | `ItemGroup`      | `ItemGroup`              | `Group`                             | --                                                           |
| ItemGroupLabel | `ItemGroupLabel` | `ItemGroupLabel`         | `Label`                             | --                                                           |
| CheckboxItem   | `CheckboxItem`   | `CheckboxItem`           | `CheckboxItem`                      | --                                                           |
| RadioGroup     | `RadioGroup`     | `RadioItemGroup`         | `RadioGroup`                        | --                                                           |
| RadioItem      | `RadioItem`      | `RadioItem`              | `RadioItem`                         | --                                                           |
| Separator      | `Separator`      | `Separator`              | `Separator`                         | --                                                           |
| Sub (submenu)  | --               | Yes (nested `Menu.Root`) | `Sub` + `SubTrigger` + `SubContent` | ars-ui ContextMenu does not support submenus                 |

**Gaps:** None. Submenus are intentionally omitted from ContextMenu -- context menus should be shallow. Use the `Menu` component for submenu-capable menus.

### 5.3 Events

| Callback      | ars-ui                | Ark UI              | Radix UI                                     | Notes                                |
| ------------- | --------------------- | ------------------- | -------------------------------------------- | ------------------------------------ |
| Open change   | `on_open_change`      | `onOpenChange`      | `onOpenChange`                               | --                                   |
| Item action   | `on_action`           | `onSelect`          | per-item `onSelect`                          | --                                   |
| Escape key    | `Event::Close`        | `onEscapeKeyDown`   | `onEscapeKeyDown`                            | Radix exposes as interceptable event |
| Click outside | `Event::ClickOutside` | `onInteractOutside` | `onPointerDownOutside` / `onInteractOutside` | --                                   |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui | Ark UI                  | Radix UI                 |
| ----------------------- | ------ | ----------------------- | ------------------------ |
| Right-click trigger     | Yes    | Yes (`ContextTrigger`)  | Yes (`Trigger`)          |
| Shift+F10 keyboard open | Yes    | Yes                     | Yes                      |
| Action items            | Yes    | Yes                     | Yes                      |
| Checkbox items          | Yes    | Yes                     | Yes                      |
| Radio items             | Yes    | Yes                     | Yes                      |
| Typeahead               | Yes    | Yes                     | Yes                      |
| Pointer-positioned      | Yes    | Yes                     | Yes                      |
| Disabled items          | Yes    | Yes                     | Yes                      |
| Separator               | Yes    | Yes                     | Yes                      |
| Submenus                | No     | Yes (via nested `Menu`) | Yes (`Sub`/`SubContent`) |

**Gaps:** None. Submenu omission is an intentional design decision -- context menus should be flat.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses a separate `ContextMenu` component; Ark UI uses the same `Menu` component with a `ContextTrigger` part; (2) ars-ui names the trigger area `Target` (not `Trigger`) to distinguish from button-click triggers; (3) Submenus are intentionally not supported in ContextMenu -- use `Menu` for submenu-capable menus.
- **Recommended additions:** None.
