//! Context menu selection component machine.
//!
//! A context menu shares the regular menu item model but opens from an
//! adapter-provided pointer coordinate or keyboard context-menu intent rather
//! than from a trigger button.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::{
    Collection, DisabledBehavior, Key, Node, StaticCollection,
    navigation::{first_enabled_key, last_enabled_key, next_enabled_key, prev_enabled_key},
    typeahead,
};
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, KeyboardKey, Locale, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

use super::menu::{self, ActionCallback, ItemType, OpenChangeCallback};

/// Context menu machine states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The context menu is closed.
    #[default]
    Closed,

    /// The context menu is open.
    Open,
}

/// Events accepted by the context menu machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the context menu at the given viewport coordinates.
    ContextOpen {
        /// Pointer x coordinate.
        x: f64,

        /// Pointer y coordinate.
        y: f64,
    },

    /// Close the context menu.
    Close,

    /// Highlight an item by key.
    HighlightItem(Option<Key>),

    /// Highlight the first enabled item.
    HighlightFirst,

    /// Highlight the last enabled item.
    HighlightLast,

    /// Highlight the next enabled item.
    HighlightNext,

    /// Highlight the previous enabled item.
    HighlightPrev,

    /// Activate a normal action item.
    SelectItem(Key),

    /// Toggle a checkbox menu item.
    ToggleCheckboxItem(Key),

    /// Select a radio menu item.
    SelectRadioItem {
        /// Radio group identifier.
        group: String,

        /// Radio item key.
        value: Key,
    },

    /// Open a submenu for the given trigger item.
    OpenSubmenu(Key),

    /// Close the currently open submenu.
    CloseSubmenu,

    /// Close because the adapter detected an outside click.
    ClickOutside,

    /// Search by type-ahead character and timestamp.
    TypeaheadSearch(char, u64),

    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<menu::Item>),

    /// Synchronize context values derived from updated props.
    SyncProps,
}

/// Context for the context menu machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Resolved locale for locale-aware type-ahead matching.
    pub locale: Locale,

    /// Item collection.
    pub items: StaticCollection<menu::Item>,

    /// Whether the context menu is open.
    pub open: bool,

    /// Highlighted item key.
    pub highlighted_key: Option<Key>,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// Checked state for checkbox menu items.
    pub checked_items: BTreeMap<Key, bool>,

    /// Selected value for each radio group.
    pub radio_groups: BTreeMap<String, Key>,

    /// Key of the currently open submenu trigger.
    pub submenu_open: Option<Key>,

    /// Type-ahead buffer state.
    pub typeahead: typeahead::State,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Pointer position where the context menu was opened.
    pub position: Option<(f64, f64)>,
}

/// Props for the `ContextMenu` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Whether the context menu is disabled.
    pub disabled: bool,

    /// Whether item activation closes the menu by default.
    pub close_on_action: bool,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Disabled-item behavior for keyboard navigation.
    pub disabled_behavior: DisabledBehavior,

    /// Callback invoked when the context menu open state changes.
    pub on_open_change: Option<Callback<OpenChangeCallback>>,

    /// Callback invoked when a normal action item is activated.
    pub on_action: Option<Callback<ActionCallback>>,

    /// Item keys that are disabled.
    pub disabled_keys: BTreeSet<Key>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            close_on_action: true,
            loop_focus: true,
            disabled_behavior: DisabledBehavior::Skip,
            on_open_change: None,
            on_action: None,
            disabled_keys: BTreeSet::new(),
        }
    }
}

impl Props {
    /// Returns default context menu props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`Self::id`].
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`Self::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`Self::close_on_action`].
    #[must_use]
    pub const fn close_on_action(mut self, value: bool) -> Self {
        self.close_on_action = value;
        self
    }

    /// Sets [`Self::loop_focus`].
    #[must_use]
    pub const fn loop_focus(mut self, value: bool) -> Self {
        self.loop_focus = value;
        self
    }

    /// Sets [`Self::disabled_behavior`].
    #[must_use]
    pub const fn disabled_behavior(mut self, value: DisabledBehavior) -> Self {
        self.disabled_behavior = value;
        self
    }

    /// Sets [`Self::disabled_keys`].
    #[must_use]
    pub fn disabled_keys(mut self, value: BTreeSet<Key>) -> Self {
        self.disabled_keys = value;
        self
    }

    /// Sets [`Self::on_open_change`].
    #[must_use]
    pub fn on_open_change(mut self, value: Callback<OpenChangeCallback>) -> Self {
        self.on_open_change = Some(value);
        self
    }

    /// Sets [`Self::on_action`].
    #[must_use]
    pub fn on_action(mut self, value: Callback<ActionCallback>) -> Self {
        self.on_action = Some(value);
        self
    }
}

/// Localized messages for the context menu component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Context menu anatomy parts.
#[derive(ComponentPart)]
#[scope = "context-menu"]
pub enum Part {
    /// Root container.
    Root,

    /// Target element that receives context-menu intent.
    Target,

    /// Positioning wrapper.
    Positioner,

    /// Floating arrow.
    Arrow,

    /// Menu content panel.
    Content,

    /// Structural item group.
    ItemGroup {
        /// Group key.
        key: Key,
    },

    /// Label for an item group.
    ItemGroupLabel {
        /// Group key.
        key: Key,
    },

    /// Action menu item.
    Item {
        /// Item key.
        key: Key,
    },

    /// Item text label.
    ItemText {
        /// Item key.
        key: Key,
    },

    /// Checkbox or radio item indicator.
    ItemIndicator {
        /// Item key.
        key: Key,
    },

    /// Separator between menu items.
    Separator,

    /// Checkbox menu item.
    CheckboxItem {
        /// Item key.
        key: Key,
    },

    /// Radio group container.
    RadioGroup {
        /// Radio group key.
        group: Key,
    },

    /// Radio menu item.
    RadioItem {
        /// Item key.
        key: Key,

        /// Radio group key.
        group: Key,
    },

    /// Submenu trigger item.
    SubTrigger {
        /// Item key.
        key: Key,
    },

    /// Submenu positioning wrapper.
    SubPositioner {
        /// Item key.
        key: Key,
    },

    /// Submenu content panel.
    SubContent {
        /// Item key.
        key: Key,
    },

    /// Visual keyboard shortcut hint.
    Shortcut {
        /// Item key.
        key: Key,
    },
}

/// Machine for the context menu component.
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

    fn init(props: &Props, env: &Env, _messages: &Messages) -> (State, Context) {
        (
            State::Closed,
            Context {
                locale: env.locale.clone(),
                items: StaticCollection::default(),
                open: false,
                highlighted_key: None,
                focus_visible: false,
                checked_items: BTreeMap::new(),
                radio_groups: BTreeMap::new(),
                submenu_open: None,
                typeahead: typeahead::State::default(),
                loop_focus: props.loop_focus,
                ids: ComponentIds::from_id(&props.id),
                position: None,
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if props.disabled
            && matches!(
                event,
                Event::ContextOpen { .. }
                    | Event::HighlightItem(_)
                    | Event::HighlightFirst
                    | Event::HighlightLast
                    | Event::HighlightNext
                    | Event::HighlightPrev
                    | Event::SelectItem(_)
                    | Event::ToggleCheckboxItem(_)
                    | Event::SelectRadioItem { .. }
                    | Event::OpenSubmenu(_)
                    | Event::TypeaheadSearch(_, _)
            )
        {
            return None;
        }

        match (state, event) {
            (_, Event::ContextOpen { x, y }) => {
                Some(open_plan((*x, *y), first_key(ctx, props), props))
            }

            (State::Open, Event::Close | Event::ClickOutside) => Some(close_plan(props)),

            (_, Event::HighlightItem(key)) => {
                let key = key
                    .clone()
                    .filter(|key| ctx.items.contains_key(key) && is_focusable_key(ctx, props, key));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    set_highlight(ctx, key);
                }))
            }

            (State::Open, Event::HighlightFirst) => {
                let key = first_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    set_highlight(ctx, key);
                }))
            }

            (State::Open, Event::HighlightLast) => {
                let key = last_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    set_highlight(ctx, key);
                }))
            }

            (State::Open, Event::HighlightNext) => {
                let key = next_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    set_highlight(ctx, key);
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let key = prev_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    set_highlight(ctx, key);
                }))
            }

            (State::Open, Event::SelectItem(key)) => select_item_plan(ctx, props, key.clone()),

            (State::Open, Event::ToggleCheckboxItem(key)) => {
                toggle_checkbox_plan(ctx, props, key.clone())
            }

            (State::Open, Event::SelectRadioItem { group, value }) => {
                select_radio_plan(ctx, props, group.clone(), value.clone())
            }

            (State::Open, Event::OpenSubmenu(key)) => open_submenu_plan(ctx, props, key.clone()),

            (State::Open, Event::CloseSubmenu) => {
                let parent = ctx.submenu_open.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.submenu_open = None;
                    ctx.highlighted_key = parent;
                }))
            }

            (State::Open, Event::TypeaheadSearch(ch, now_ms)) => {
                let (typeahead, found) = ctx.typeahead.process_char_with_locale(
                    *ch,
                    *now_ms,
                    ctx.highlighted_key.as_ref(),
                    &ctx.items,
                    &ctx.locale,
                    &props.disabled_keys,
                    props.disabled_behavior,
                );

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.typeahead = typeahead;

                    if let Some(key) = found {
                        set_highlight(ctx, Some(key));
                    }
                }))
            }

            (_, Event::UpdateItems(items)) => {
                let items = items.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.items = items;

                    invalidate_collection_references(ctx);
                }))
            }

            (_, Event::SyncProps) => Some(sync_props_plan(ctx, props)),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps]
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
}

/// API for deriving context menu attributes and dispatching context menu events.
pub struct Api<'a> {
    /// Current state.
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
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns the current machine state.
    #[must_use]
    pub const fn state(&self) -> &State {
        self.state
    }

    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        if self.props.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the target element.
    #[must_use]
    pub fn target_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Target);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("target"))
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.ctx.open { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        if self.props.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the positioner wrapper.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        part_attrs(&Part::Positioner)
    }

    /// Attributes for the floating arrow.
    #[must_use]
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Arrow);

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the menu content panel.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "menu")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("target"),
            );

        attrs
    }

    /// Attributes for an item group container.
    #[must_use]
    pub fn item_group_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroup {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Role, "group").set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.item_part("group", key, "label"),
        );

        attrs
    }

    /// Attributes for an item group label.
    #[must_use]
    pub fn item_group_label_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroupLabel {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("group", key, "label"));

        attrs
    }

    /// Attributes for an action menu item.
    #[must_use]
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        self.item_base_attrs(
            &Part::Item {
                key: Key::default(),
            },
            key,
            "menuitem",
        )
    }

    /// Attributes for an item's text label.
    #[must_use]
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemText {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "text"));

        attrs
    }

    /// Attributes for an item's check or radio indicator.
    #[must_use]
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemIndicator {
            key: Key::default(),
        });

        attrs
            .set(
                HtmlAttr::Id,
                self.ctx.ids.item_part("item", key, "indicator"),
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for a separator.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Separator);

        attrs.set(HtmlAttr::Role, "separator");

        attrs
    }

    /// Attributes for a checkbox menu item.
    #[must_use]
    pub fn checkbox_item_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = self.item_base_attrs(
            &Part::CheckboxItem {
                key: Key::default(),
            },
            key,
            "menuitemcheckbox",
        );

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Checked),
            if self.ctx.checked_items.get(key).copied().unwrap_or(false) {
                "true"
            } else {
                "false"
            },
        );

        attrs
    }

    /// Attributes for a radio group container.
    #[must_use]
    pub fn radio_group_attrs(&self, group: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::RadioGroup {
            group: Key::default(),
        });

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("radio-group", group))
            .set(HtmlAttr::Role, "group");

        attrs
    }

    /// Attributes for a radio menu item.
    #[must_use]
    pub fn radio_item_attrs(&self, key: &Key, group: &Key) -> AttrMap {
        let group_value = group.to_string();

        let mut attrs = self.item_base_attrs(
            &Part::RadioItem {
                key: Key::default(),
                group: Key::default(),
            },
            key,
            "menuitemradio",
        );

        attrs.set(
            HtmlAttr::Aria(AriaAttr::Checked),
            if self.ctx.radio_groups.get(&group_value) == Some(key) {
                "true"
            } else {
                "false"
            },
        );

        attrs
    }

    /// Attributes for a submenu trigger item.
    #[must_use]
    pub fn sub_trigger_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = self.item_base_attrs(
            &Part::SubTrigger {
                key: Key::default(),
            },
            key,
            "menuitem",
        );

        attrs
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.ctx.submenu_open.as_ref() == Some(key) {
                    "true"
                } else {
                    "false"
                },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.item("sub-content", key),
            );

        attrs
    }

    /// Attributes for a submenu positioner wrapper.
    #[must_use]
    pub fn sub_positioner_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::SubPositioner {
            key: Key::default(),
        });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("sub", key, "positioner"),
        );

        attrs
    }

    /// Attributes for a submenu content panel.
    #[must_use]
    pub fn sub_content_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::SubContent {
            key: Key::default(),
        });

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("sub-content", key))
            .set(HtmlAttr::Role, "menu")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.item("item", key),
            );

        attrs
    }

    /// Attributes for the shortcut element.
    #[must_use]
    pub fn item_shortcut_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Shortcut {
            key: Key::default(),
        });

        attrs
            .set(
                HtmlAttr::Id,
                self.ctx.ids.item_part("item", key, "shortcut"),
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Iterates all collection nodes for rendering.
    pub fn items(&self) -> impl Iterator<Item = &Node<menu::Item>> {
        self.ctx.items.nodes()
    }

    /// Dispatches context-menu pointer intent on the target.
    pub fn on_target_contextmenu(&self, x: f64, y: f64) {
        if !self.props.disabled {
            (self.send)(Event::ContextOpen { x, y });
        }
    }

    /// Dispatches keyboard context-menu intent on the target at adapter-provided coordinates.
    pub fn on_target_keydown(&self, data: &KeyboardEventData, x: f64, y: f64) {
        let opens = data.key == KeyboardKey::ContextMenu
            || (data.key == KeyboardKey::F10 && data.shift_key);

        if opens && !self.props.disabled {
            (self.send)(Event::ContextOpen { x, y });
        }
    }

    /// Dispatches content keydown events.
    pub fn on_content_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        self.on_content_keydown_impl(data, ctrl, meta, None);
    }

    /// Dispatches content keydown events with an adapter-provided timestamp.
    pub fn on_content_keydown_at(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: u64,
    ) {
        self.on_content_keydown_impl(data, ctrl, meta, Some(now_ms));
    }

    /// Dispatches an item click event.
    pub fn on_item_click(&self, key: &Key) {
        self.dispatch_activation(key);
    }

    /// Dispatches an item pointer-enter event.
    pub fn on_item_pointer_enter(&self, key: &Key) {
        (self.send)(Event::HighlightItem(Some(key.clone())));
    }

    /// Dispatches a content pointer-leave event.
    pub fn on_content_pointer_leave(&self) {
        if self.ctx.submenu_open.is_none() {
            (self.send)(Event::HighlightItem(None));
        }
    }

    fn on_content_keydown_impl(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: Option<u64>,
    ) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::HighlightNext),

            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),

            KeyboardKey::Home => (self.send)(Event::HighlightFirst),

            KeyboardKey::End => (self.send)(Event::HighlightLast),

            KeyboardKey::Escape => {
                if self.ctx.submenu_open.is_some() {
                    (self.send)(Event::CloseSubmenu);
                } else {
                    (self.send)(Event::Close);
                }
            }

            key if key == submenu_open_key(self.ctx) => {
                if let Some(key) = &self.ctx.highlighted_key
                    && item_type_matches(self.ctx, key, |item_type| {
                        matches!(item_type, ItemType::Submenu)
                    })
                {
                    (self.send)(Event::OpenSubmenu(key.clone()));
                }
            }

            key if key == submenu_close_key(self.ctx) && self.ctx.submenu_open.is_some() => {
                (self.send)(Event::CloseSubmenu);
            }

            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(key) = &self.ctx.highlighted_key {
                    self.dispatch_activation(key);
                }
            }

            _ if data.character.is_some() && !ctrl && !meta && !data.is_composing => {
                (self.send)(Event::TypeaheadSearch(
                    data.character.expect("checked"),
                    typeahead_time(now_ms, &self.ctx.typeahead),
                ));
            }

            _ => {}
        }
    }

    fn dispatch_activation(&self, key: &Key) {
        if let Some(item) = self.item_payload(key) {
            match &item.item_type {
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

    fn item_base_attrs(&self, part: &Part, key: &Key, role: &'static str) -> AttrMap {
        let mut attrs = part_attrs(part);

        let highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let disabled = self.props.disabled || self.props.disabled_keys.contains(key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("item", key))
            .set(HtmlAttr::Role, role)
            .set(HtmlAttr::TabIndex, if highlighted { "0" } else { "-1" });

        if highlighted {
            attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if let Some(item) = self.item_payload(key)
            && let Some(shortcut) = &item.aria_keyshortcuts
        {
            attrs.set(HtmlAttr::Aria(AriaAttr::KeyShortcuts), shortcut);
        }

        attrs
    }

    fn item_payload(&self, key: &Key) -> Option<&menu::Item> {
        self.ctx.items.get(key)?.value.as_ref()
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
            Part::RadioItem { ref key, ref group } => self.radio_item_attrs(key, group),
            Part::SubTrigger { ref key } => self.sub_trigger_attrs(key),
            Part::SubPositioner { ref key } => self.sub_positioner_attrs(key),
            Part::SubContent { ref key } => self.sub_content_attrs(key),
            Part::Shortcut { ref key } => self.item_shortcut_attrs(key),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn open_plan(
    position: (f64, f64),
    highlighted_key: Option<Key>,
    props: &Props,
) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        let was_open = ctx.open;

        ctx.open = true;
        ctx.position = Some(position);
        ctx.highlighted_key = highlighted_key.clone();

        if !was_open && let Some(callback) = &on_open_change {
            callback(true);
        }
    })
}

fn close_plan(props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
        let was_open = ctx.open;

        ctx.open = false;
        ctx.highlighted_key = None;
        ctx.submenu_open = None;
        ctx.typeahead = typeahead::State::default();
        ctx.position = None;
        ctx.focus_visible = false;

        if was_open && let Some(callback) = &on_open_change {
            callback(false);
        }
    })
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let props = props.clone();
    let on_open_change = props.on_open_change.clone();
    let mut plan = if props.disabled && ctx.open {
        TransitionPlan::to(State::Closed)
    } else {
        TransitionPlan::new()
    };

    plan = plan.apply(move |ctx: &mut Context| {
        let was_open = ctx.open;

        ctx.loop_focus = props.loop_focus;
        ctx.ids = ComponentIds::from_id(&props.id);

        if ctx
            .highlighted_key
            .as_ref()
            .is_some_and(|key| !is_focusable_key(ctx, &props, key))
        {
            set_highlight(ctx, None);
        }

        if props.disabled {
            ctx.open = false;
            ctx.highlighted_key = None;
            ctx.submenu_open = None;
            ctx.position = None;

            if was_open && let Some(callback) = &on_open_change {
                callback(false);
            }
        }
    });

    plan
}

fn select_item_plan(ctx: &Context, props: &Props, key: Key) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_item(ctx, props, &key) {
        return None;
    }

    let item = item_payload(ctx, &key)?;

    if item.item_type != ItemType::Normal {
        return None;
    }

    let close = item.close_on_action.unwrap_or(props.close_on_action);
    let on_action = props.on_action.clone();

    Some(action_plan(key, close, props, move |_ctx, key| {
        if let Some(callback) = &on_action {
            callback(key);
        }
    }))
}

fn toggle_checkbox_plan(ctx: &Context, props: &Props, key: Key) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_item(ctx, props, &key) {
        return None;
    }

    let item = item_payload(ctx, &key)?;

    if item.item_type != ItemType::Checkbox {
        return None;
    }

    let current = ctx.checked_items.get(&key).copied().unwrap_or(false);
    let close = item.close_on_action.unwrap_or(props.close_on_action);

    Some(action_plan(key, close, props, move |ctx, key| {
        ctx.checked_items.insert(key, !current);
    }))
}

fn select_radio_plan(
    ctx: &Context,
    props: &Props,
    group: String,
    value: Key,
) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_item(ctx, props, &value) {
        return None;
    }

    let item = item_payload(ctx, &value)?;

    match &item.item_type {
        ItemType::Radio { group: item_group } if item_group == &group => {}
        _ => return None,
    }

    let close = item.close_on_action.unwrap_or(props.close_on_action);

    Some(action_plan(value, close, props, move |ctx, key| {
        ctx.radio_groups.insert(group.clone(), key);
    }))
}

fn open_submenu_plan(ctx: &Context, props: &Props, key: Key) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_item(ctx, props, &key) {
        return None;
    }

    if !item_type_matches(ctx, &key, |item_type| {
        matches!(item_type, ItemType::Submenu)
    }) {
        return None;
    }

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.submenu_open = Some(key.clone());
        ctx.highlighted_key = Some(key.clone());
    }))
}

fn action_plan(
    key: Key,
    close: bool,
    props: &Props,
    action: impl Fn(&mut Context, Key) + Send + Sync + 'static,
) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    if close {
        TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
            action(ctx, key.clone());

            let was_open = ctx.open;

            ctx.open = false;
            ctx.highlighted_key = None;
            ctx.submenu_open = None;
            ctx.typeahead = typeahead::State::default();
            ctx.position = None;
            ctx.focus_visible = false;

            if was_open && let Some(callback) = &on_open_change {
                callback(false);
            }
        })
    } else {
        TransitionPlan::context_only(move |ctx: &mut Context| {
            action(ctx, key.clone());
            ctx.highlighted_key = Some(key.clone());
        })
    }
}

fn first_key(ctx: &Context, props: &Props) -> Option<Key> {
    first_enabled_key(&ctx.items, &props.disabled_keys, props.disabled_behavior)
}

fn last_key(ctx: &Context, props: &Props) -> Option<Key> {
    last_enabled_key(&ctx.items, &props.disabled_keys, props.disabled_behavior)
}

fn next_key(ctx: &Context, props: &Props) -> Option<Key> {
    if let Some(key) = &ctx.highlighted_key {
        next_enabled_key(
            &ctx.items,
            key,
            &props.disabled_keys,
            props.disabled_behavior,
            ctx.loop_focus,
        )
    } else {
        first_key(ctx, props)
    }
}

fn prev_key(ctx: &Context, props: &Props) -> Option<Key> {
    if let Some(key) = &ctx.highlighted_key {
        prev_enabled_key(
            &ctx.items,
            key,
            &props.disabled_keys,
            props.disabled_behavior,
            ctx.loop_focus,
        )
    } else {
        last_key(ctx, props)
    }
}

fn is_focusable_key(ctx: &Context, props: &Props, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable)
        && (!props.disabled_keys.contains(key)
            || props.disabled_behavior == DisabledBehavior::FocusOnly)
}

fn is_selectable_item(ctx: &Context, props: &Props, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable) && !props.disabled_keys.contains(key)
}

fn item_payload<'a>(ctx: &'a Context, key: &Key) -> Option<&'a menu::Item> {
    ctx.items.get(key)?.value.as_ref()
}

fn item_type_matches(ctx: &Context, key: &Key, predicate: impl FnOnce(&ItemType) -> bool) -> bool {
    item_payload(ctx, key).is_some_and(|item| predicate(&item.item_type))
}

fn set_highlight(ctx: &mut Context, key: Option<Key>) {
    if ctx.submenu_open.as_ref() != key.as_ref() {
        ctx.submenu_open = None;
    }

    ctx.highlighted_key = key;
}

fn invalidate_collection_references(ctx: &mut Context) {
    if ctx
        .highlighted_key
        .as_ref()
        .is_some_and(|key| !ctx.items.get(key).is_some_and(Node::is_focusable))
    {
        set_highlight(ctx, None);
    }

    if ctx.submenu_open.as_ref().is_some_and(|key| {
        !item_type_matches(ctx, key, |item_type| matches!(item_type, ItemType::Submenu))
    }) {
        ctx.submenu_open = None;
    }

    let checkbox_keys = ctx
        .items
        .nodes()
        .filter_map(|node| {
            node.value
                .as_ref()
                .filter(|item| item.item_type == ItemType::Checkbox)
                .map(|_| node.key.clone())
        })
        .collect::<BTreeSet<_>>();

    let radio_keys = ctx
        .items
        .nodes()
        .filter_map(|node| {
            node.value.as_ref().and_then(|item| match &item.item_type {
                ItemType::Radio { group } => Some((node.key.clone(), group.clone())),
                _ => None,
            })
        })
        .collect::<BTreeMap<_, _>>();

    ctx.checked_items
        .retain(|key, _| checkbox_keys.contains(key));

    ctx.radio_groups.retain(|group, key| {
        radio_keys
            .get(key)
            .is_some_and(|item_group| item_group == group)
    });
}

fn submenu_open_key(ctx: &Context) -> KeyboardKey {
    if ctx.locale.is_rtl() {
        KeyboardKey::ArrowLeft
    } else {
        KeyboardKey::ArrowRight
    }
}

fn submenu_close_key(ctx: &Context) -> KeyboardKey {
    if ctx.locale.is_rtl() {
        KeyboardKey::ArrowRight
    } else {
        KeyboardKey::ArrowLeft
    }
}

fn typeahead_time(now_ms: Option<u64>, state: &typeahead::State) -> u64 {
    now_ms.unwrap_or_else(|| {
        current_time_ms().unwrap_or_else(|| state.last_key_time_ms.saturating_add(1))
    })
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

#[cfg(test)]
mod tests {
    use alloc::{
        collections::{BTreeMap, BTreeSet},
        format,
        string::String,
        sync::Arc,
        vec,
    };
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_collections::{Collection, CollectionBuilder, Key};
    use ars_core::{AriaAttr, Callback, ConnectApi, Env, HtmlAttr, KeyboardKey, Service};
    use ars_interactions::KeyboardEventData;

    use super::{Event, Machine, Messages, Part, Props, State};
    use crate::selection::menu::{Item, ItemType};

    fn key(value: &'static str) -> Key {
        Key::str(value)
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

    fn item(label: &'static str, item_type: ItemType) -> Item {
        Item {
            label: label.into(),
            item_type,
            shortcut: None,
            aria_keyshortcuts: None,
            close_on_action: None,
        }
    }

    fn collection() -> ars_collections::StaticCollection<Item> {
        CollectionBuilder::new()
            .item(key("alpha"), "Alpha", item("Alpha", ItemType::Normal))
            .item(key("bravo"), "Bravo", item("Bravo", ItemType::Checkbox))
            .item(
                key("charlie"),
                "Charlie",
                item(
                    "Charlie",
                    ItemType::Radio {
                        group: "density".into(),
                    },
                ),
            )
            .item(key("delta"), "Delta", item("Delta", ItemType::Submenu))
            .separator()
            .build()
    }

    fn service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

        drop(service.send(Event::UpdateItems(collection())));

        service
    }

    fn apply(
        state: State,
        event: &Event,
        ctx: &super::Context,
        props: &Props,
    ) -> Option<ars_core::TransitionPlan<Machine>> {
        <Machine as ars_core::Machine>::transition(&state, event, ctx, props)
    }

    fn snapshot_attrs(attrs: &ars_core::AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn captured_events(
        menu: &Service<Machine>,
        dispatch: impl FnOnce(&super::Api<'_>),
    ) -> Vec<Event> {
        let captured = RefCell::new(Vec::new());

        let send = |event| captured.borrow_mut().push(event);

        let api = menu.connect(&send);

        dispatch(&api);

        captured.into_inner()
    }

    fn dispatch_content_pointer_leave(api: &super::Api<'_>) {
        api.on_content_pointer_leave();
    }

    #[test]
    fn context_open_stores_pointer_position_and_highlights_first_item() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 12.0, y: 34.0 }));

        assert_eq!(menu.state(), &State::Open);
        assert!(menu.context().open);
        assert_eq!(menu.context().position, Some((12.0, 34.0)));
        assert_eq!(menu.context().highlighted_key, Some(key("alpha")));
    }

    #[test]
    fn target_keyboard_shortcuts_dispatch_context_open_at_anchor_position() {
        let menu = service(Props::new().id("context-menu"));

        let mut event = keyboard(KeyboardKey::F10, None);

        event.shift_key = true;

        assert_eq!(
            captured_events(&menu, |api| api.on_target_keydown(&event, 42.0, 24.0)),
            vec![Event::ContextOpen { x: 42.0, y: 24.0 }]
        );

        assert_eq!(
            captured_events(&menu, |api| api.on_target_keydown(
                &keyboard(KeyboardKey::ContextMenu, None),
                8.0,
                16.0
            )),
            vec![Event::ContextOpen { x: 8.0, y: 16.0 }]
        );
    }

    #[test]
    fn disabled_target_does_not_dispatch_open() {
        let menu = service(Props::new().id("context-menu").disabled(true));

        assert!(captured_events(&menu, |api| api.on_target_contextmenu(1.0, 2.0)).is_empty());
    }

    #[test]
    fn builder_methods_preserve_public_props_contract() {
        let props = Props::new()
            .id("context-menu")
            .disabled(true)
            .close_on_action(false)
            .loop_focus(false)
            .disabled_behavior(ars_collections::DisabledBehavior::FocusOnly)
            .disabled_keys(BTreeSet::from([key("alpha")]))
            .on_open_change(Callback::new(|_open: bool| {}))
            .on_action(Callback::new(|_key: Key| {}));

        assert_eq!(props.id, "context-menu");
        assert!(props.disabled);
        assert!(!props.close_on_action);
        assert!(!props.loop_focus);
        assert_eq!(
            props.disabled_behavior,
            ars_collections::DisabledBehavior::FocusOnly
        );
        assert_eq!(props.disabled_keys, BTreeSet::from([key("alpha")]));
        assert!(props.on_open_change.is_some());
        assert!(props.on_action.is_some());
    }

    #[test]
    fn content_and_item_attrs_emit_menu_roles() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));

        let api = menu.connect(&|_| {});

        assert_eq!(api.content_attrs().get(&HtmlAttr::Role), Some("menu"));
        assert_eq!(
            api.item_attrs(&key("alpha")).get(&HtmlAttr::Role),
            Some("menuitem")
        );
        assert_eq!(
            api.checkbox_item_attrs(&key("bravo")).get(&HtmlAttr::Role),
            Some("menuitemcheckbox")
        );
        assert_eq!(
            api.radio_item_attrs(&key("charlie"), &key("density"))
                .get(&HtmlAttr::Role),
            Some("menuitemradio")
        );
    }

    #[test]
    fn close_and_click_outside_clear_open_state() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 1.0, y: 2.0 }));
        drop(menu.send(Event::Close));

        assert_eq!(menu.state(), &State::Closed);
        assert!(!menu.context().open);
        assert_eq!(menu.context().position, None);

        drop(menu.send(Event::ContextOpen { x: 3.0, y: 4.0 }));
        drop(menu.send(Event::ClickOutside));

        assert_eq!(menu.state(), &State::Closed);
        assert!(!menu.context().open);
    }

    #[test]
    fn arrow_navigation_moves_highlight_like_menu() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightNext));

        assert_eq!(menu.context().highlighted_key, Some(key("bravo")));

        drop(menu.send(Event::HighlightLast));

        assert_eq!(menu.context().highlighted_key, Some(key("delta")));

        drop(menu.send(Event::HighlightPrev));

        assert_eq!(menu.context().highlighted_key, Some(key("charlie")));

        drop(menu.send(Event::HighlightFirst));

        assert_eq!(menu.context().highlighted_key, Some(key("alpha")));

        let separator = menu
            .context()
            .items
            .nodes()
            .find(|node| node.node_type == ars_collections::NodeType::Separator)
            .expect("fixture has separator")
            .key
            .clone();

        drop(menu.send(Event::HighlightItem(Some(separator))));

        assert_eq!(menu.context().highlighted_key, None);
    }

    #[test]
    fn content_keydown_escape_closes_menu() {
        let menu = service(Props::new().id("context-menu"));

        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::Escape, None),
                false,
                false
            )),
            vec![Event::Close]
        );
    }

    #[test]
    fn target_keyboard_shortcut_requires_shift_f10_and_enabled_target() {
        let enabled = service(Props::new().id("context-menu"));
        let disabled = service(Props::new().id("context-menu").disabled(true));

        let mut plain_f10 = keyboard(KeyboardKey::F10, None);
        let mut shifted_enter = keyboard(KeyboardKey::Enter, None);

        shifted_enter.shift_key = true;
        plain_f10.shift_key = false;

        assert!(
            captured_events(&enabled, |api| api.on_target_keydown(&plain_f10, 1.0, 2.0)).is_empty()
        );
        assert!(
            captured_events(&enabled, |api| api.on_target_keydown(
                &shifted_enter,
                1.0,
                2.0
            ))
            .is_empty()
        );

        plain_f10.shift_key = true;

        assert!(
            captured_events(&disabled, |api| api.on_target_keydown(&plain_f10, 1.0, 2.0))
                .is_empty()
        );
        assert!(
            captured_events(&disabled, |api| api.on_target_keydown(
                &keyboard(KeyboardKey::ContextMenu, None),
                1.0,
                2.0
            ))
            .is_empty()
        );
    }

    #[test]
    fn api_dispatch_helpers_cover_pointer_and_content_keys() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));

        assert_eq!(
            captured_events(&menu, |api| api.on_target_contextmenu(7.0, 9.0)),
            vec![Event::ContextOpen { x: 7.0, y: 9.0 }]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_item_pointer_enter(&key("alpha"))),
            vec![Event::HighlightItem(Some(key("alpha")))]
        );
        assert_eq!(
            captured_events(&menu, dispatch_content_pointer_leave),
            vec![Event::HighlightItem(None)]
        );

        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        assert!(captured_events(&menu, dispatch_content_pointer_leave).is_empty());
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::ArrowDown, None),
                false,
                false
            )),
            vec![Event::HighlightNext]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::ArrowUp, None),
                false,
                false
            )),
            vec![Event::HighlightPrev]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::Home, None),
                false,
                false
            )),
            vec![Event::HighlightFirst]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::End, None),
                false,
                false
            )),
            vec![Event::HighlightLast]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::Enter, None),
                false,
                false
            )),
            vec![Event::OpenSubmenu(key("delta"))]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::Space, None),
                false,
                false
            )),
            vec![Event::OpenSubmenu(key("delta"))]
        );
    }

    #[test]
    fn submenu_keyboard_dispatch_is_directional_and_stateful() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));

        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::ArrowRight, None),
                false,
                false
            )),
            vec![Event::OpenSubmenu(key("delta"))]
        );

        assert!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::ArrowLeft, None),
                false,
                false
            ))
            .is_empty()
        );

        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::ArrowLeft, None),
                false,
                false
            )),
            vec![Event::CloseSubmenu]
        );
        assert_eq!(
            captured_events(&menu, |api| api.on_content_keydown(
                &keyboard(KeyboardKey::Escape, None),
                false,
                false
            )),
            vec![Event::CloseSubmenu]
        );
    }

    #[test]
    fn printable_key_dispatch_respects_modifier_guards_and_timestamp() {
        let menu = service(Props::new().id("context-menu"));

        let printable = keyboard(KeyboardKey::Unidentified, Some('a'));

        assert_eq!(
            captured_events(&menu, |api| api
                .on_content_keydown_at(&printable, false, false, 42)),
            vec![Event::TypeaheadSearch('a', 42)]
        );

        let fallback_events = captured_events(&menu, |api| {
            api.on_content_keydown(&printable, false, false);
        });

        match fallback_events.as_slice() {
            [Event::TypeaheadSearch('a', timestamp)] => assert!(*timestamp > 1),
            other => panic!("unexpected fallback typeahead events: {other:?}"),
        }

        assert!(
            captured_events(&menu, |api| api.on_content_keydown(&printable, true, false))
                .is_empty()
        );
        assert!(
            captured_events(&menu, |api| api.on_content_keydown(&printable, false, true))
                .is_empty()
        );

        let mut composing = printable.clone();

        composing.is_composing = true;

        assert!(
            captured_events(&menu, |api| api
                .on_content_keydown(&composing, false, false))
            .is_empty()
        );
    }

    #[test]
    fn submenu_item_dispatches_open_submenu() {
        let mut menu = service(Props::new().id("context-menu"));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));

        assert_eq!(
            captured_events(&menu, |api| api.on_item_click(&key("delta"))),
            vec![Event::OpenSubmenu(key("delta"))]
        );
    }

    #[test]
    fn selection_transitions_validate_type_and_disabled_state() {
        let actions = Arc::new(Mutex::new(Vec::<Key>::new()));

        let mut menu = service(
            Props::new()
                .id("context-menu")
                .close_on_action(false)
                .disabled_keys(BTreeSet::from([key("alpha")]))
                .on_action(Callback::new({
                    let actions = Arc::clone(&actions);
                    move |key| actions.lock().expect("action capture poisoned").push(key)
                })),
        );

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));

        assert!(
            apply(
                *menu.state(),
                &Event::SelectItem(key("alpha")),
                menu.context(),
                menu.props()
            )
            .is_none()
        );
        assert!(
            apply(
                *menu.state(),
                &Event::SelectItem(key("bravo")),
                menu.context(),
                menu.props()
            )
            .is_none()
        );
        assert!(
            apply(
                *menu.state(),
                &Event::ToggleCheckboxItem(key("alpha")),
                menu.context(),
                menu.props()
            )
            .is_none()
        );
        assert!(
            apply(
                *menu.state(),
                &Event::SelectRadioItem {
                    group: "wrong".into(),
                    value: key("charlie"),
                },
                menu.context(),
                menu.props()
            )
            .is_none()
        );
        assert!(
            apply(
                *menu.state(),
                &Event::OpenSubmenu(key("alpha")),
                menu.context(),
                menu.props()
            )
            .is_none()
        );
        assert!(
            apply(
                *menu.state(),
                &Event::OpenSubmenu(key("bravo")),
                menu.context(),
                menu.props()
            )
            .is_none()
        );

        drop(menu.send(Event::SelectItem(key("delta"))));

        assert!(actions.lock().expect("action capture poisoned").is_empty());

        drop(menu.send(Event::SelectItem(key("alpha"))));

        assert!(actions.lock().expect("action capture poisoned").is_empty());

        let mut enabled = service(
            Props::new()
                .id("context-menu")
                .close_on_action(false)
                .on_action(Callback::new({
                    let actions = Arc::clone(&actions);
                    move |key| actions.lock().expect("action capture poisoned").push(key)
                })),
        );

        drop(enabled.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(enabled.send(Event::SelectItem(key("alpha"))));

        assert_eq!(
            actions.lock().expect("action capture poisoned").as_slice(),
            &[key("alpha")]
        );
        assert_eq!(enabled.context().highlighted_key, Some(key("alpha")));
    }

    #[test]
    fn close_callbacks_fire_only_on_open_state_edges() {
        let open_changes = Arc::new(Mutex::new(Vec::new()));

        let mut menu = service(
            Props::new()
                .id("context-menu")
                .on_open_change(Callback::new({
                    let open_changes = Arc::clone(&open_changes);
                    move |open| {
                        open_changes
                            .lock()
                            .expect("open-change capture poisoned")
                            .push(open);
                    }
                })),
        );

        drop(menu.send(Event::ContextOpen { x: 1.0, y: 2.0 }));

        assert_eq!(
            open_changes
                .lock()
                .expect("open-change capture poisoned")
                .as_slice(),
            &[true]
        );

        drop(menu.send(Event::ContextOpen { x: 3.0, y: 4.0 }));
        drop(menu.send(Event::Close));
        drop(menu.send(Event::Close));

        assert_eq!(
            open_changes
                .lock()
                .expect("open-change capture poisoned")
                .as_slice(),
            &[true, false]
        );
    }

    #[test]
    fn disabling_open_menu_closes_state_and_fires_open_change() {
        let open_changes = Arc::new(Mutex::new(Vec::new()));
        let old_props = Props::new()
            .id("context-menu")
            .on_open_change(Callback::new({
                let open_changes = Arc::clone(&open_changes);
                move |open| {
                    open_changes
                        .lock()
                        .expect("open-change capture poisoned")
                        .push(open);
                }
            }));
        let new_props = old_props.clone().disabled(true);

        let mut menu = service(old_props);

        drop(menu.send(Event::ContextOpen { x: 1.0, y: 2.0 }));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));
        drop(menu.set_props(new_props));

        assert_eq!(menu.state(), &State::Closed);
        assert!(!menu.context().open);
        assert_eq!(menu.context().highlighted_key, None);
        assert_eq!(menu.context().submenu_open, None);
        assert_eq!(menu.context().position, None);
        assert_eq!(
            open_changes
                .lock()
                .expect("open-change capture poisoned")
                .as_slice(),
            &[true, false]
        );
    }

    #[test]
    fn submenu_close_typeahead_sync_and_props_changed_are_observable() {
        let old_props = Props::new().id("old");
        let new_props = Props::new().id("new").loop_focus(false);

        let mut menu = service(old_props.clone());

        assert!(
            <Machine as ars_core::Machine>::on_props_changed(&old_props, &old_props).is_empty()
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old_props, &new_props),
            vec![Event::SyncProps]
        );

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));
        drop(menu.send(Event::CloseSubmenu));

        assert_eq!(menu.context().submenu_open, None);
        assert_eq!(menu.context().highlighted_key, Some(key("delta")));

        drop(menu.send(Event::HighlightItem(Some(key("alpha")))));
        drop(menu.send(Event::TypeaheadSearch('d', 10)));

        assert_eq!(menu.context().highlighted_key, Some(key("delta")));
        assert_eq!(menu.context().typeahead.search, "d");

        drop(menu.set_props(new_props));

        assert!(!menu.context().loop_focus);
        assert_eq!(menu.context().ids, ars_core::ComponentIds::from_id("new"));
        assert_eq!(menu.context().highlighted_key, Some(key("delta")));
    }

    #[test]
    fn update_items_removes_stale_highlight_submenu_and_selection() {
        let mut menu = service(Props::new().id("context-menu").close_on_action(false));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::SelectRadioItem {
            group: "density".into(),
            value: key("charlie"),
        }));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        assert_eq!(menu.context().submenu_open, Some(key("delta")));
        assert_eq!(
            menu.context().checked_items,
            BTreeMap::from([(key("bravo"), true)])
        );
        assert_eq!(
            menu.context().radio_groups,
            BTreeMap::from([("density".into(), key("charlie"))])
        );

        let replacement = CollectionBuilder::new()
            .item(key("alpha"), "Alpha", item("Alpha", ItemType::Normal))
            .item(key("bravo"), "Bravo", item("Bravo", ItemType::Normal))
            .item(
                key("charlie"),
                "Charlie",
                item(
                    "Charlie",
                    ItemType::Radio {
                        group: "other".into(),
                    },
                ),
            )
            .build();

        drop(menu.send(Event::UpdateItems(replacement)));

        assert_eq!(menu.context().highlighted_key, None);
        assert_eq!(menu.context().submenu_open, None);
        assert!(menu.context().checked_items.is_empty());
        assert!(menu.context().radio_groups.is_empty());
    }

    #[test]
    fn sync_props_and_updates_preserve_valid_focus_only_submenus() {
        let mut menu = service(
            Props::new()
                .id("context-menu")
                .disabled_keys(BTreeSet::from([key("bravo")]))
                .disabled_behavior(ars_collections::DisabledBehavior::FocusOnly),
        );

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("bravo")))));

        assert_eq!(menu.context().highlighted_key, Some(key("bravo")));

        drop(
            menu.set_props(
                Props::new()
                    .id("context-menu")
                    .disabled_keys(BTreeSet::from([key("bravo")]))
                    .disabled_behavior(ars_collections::DisabledBehavior::Skip),
            ),
        );

        assert_eq!(menu.context().highlighted_key, None);

        drop(
            menu.set_props(
                Props::new()
                    .id("context-menu")
                    .disabled_keys(BTreeSet::from([key("bravo")]))
                    .disabled_behavior(ars_collections::DisabledBehavior::FocusOnly),
            ),
        );
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));
        drop(menu.send(Event::UpdateItems(collection())));

        assert_eq!(menu.context().submenu_open, Some(key("delta")));

        drop(menu.send(Event::HighlightItem(Some(key("alpha")))));

        assert_eq!(menu.context().submenu_open, None);
    }

    #[test]
    fn api_items_iterates_collection_and_disabled_attrs_cover_component_and_key() {
        let mut menu = service(
            Props::new()
                .id("context-menu")
                .disabled_keys(BTreeSet::from([key("bravo")])),
        );

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));

        let api = menu.connect(&|_| {});
        let keys = api.items().map(|node| node.key.clone()).collect::<Vec<_>>();

        assert!(keys.contains(&key("alpha")));
        assert!(keys.contains(&key("bravo")));
        assert_eq!(
            api.checkbox_item_attrs(&key("bravo"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );

        let disabled = service(Props::new().id("context-menu").disabled(true));

        let disabled_api = disabled.connect(&|_| {});

        assert_eq!(
            disabled_api
                .item_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn connect_attrs_snapshot_all_parts_and_branches() {
        insta::assert_snapshot!(
            "context_menu_root_default",
            snapshot_attrs(
                &service(Props::new().id("context-menu"))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        insta::assert_snapshot!(
            "context_menu_root_disabled",
            snapshot_attrs(
                &service(Props::new().id("context-menu").disabled(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        let mut menu = service(Props::new().id("context-menu").close_on_action(false));

        drop(menu.send(Event::ContextOpen { x: 0.0, y: 0.0 }));
        drop(menu.send(Event::HighlightItem(Some(key("bravo")))));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::SelectRadioItem {
            group: "density".into(),
            value: key("charlie"),
        }));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        let api = menu.connect(&|_| {});

        insta::assert_snapshot!(
            "context_menu_target_open",
            snapshot_attrs(&api.target_attrs())
        );
        insta::assert_snapshot!(
            "context_menu_positioner",
            snapshot_attrs(&api.positioner_attrs())
        );
        insta::assert_snapshot!("context_menu_arrow", snapshot_attrs(&api.arrow_attrs()));
        insta::assert_snapshot!("context_menu_content", snapshot_attrs(&api.content_attrs()));
        insta::assert_snapshot!(
            "context_menu_item_group",
            snapshot_attrs(&api.item_group_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "context_menu_item_group_label",
            snapshot_attrs(&api.item_group_label_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "context_menu_item_default",
            snapshot_attrs(&api.item_attrs(&key("alpha")))
        );
        insta::assert_snapshot!(
            "context_menu_item_text",
            snapshot_attrs(&api.item_text_attrs(&key("alpha")))
        );
        insta::assert_snapshot!(
            "context_menu_item_indicator",
            snapshot_attrs(&api.item_indicator_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "context_menu_separator",
            snapshot_attrs(&api.separator_attrs())
        );
        insta::assert_snapshot!(
            "context_menu_checkbox_checked",
            snapshot_attrs(&api.checkbox_item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "context_menu_radio_group",
            snapshot_attrs(&api.radio_group_attrs(&key("density")))
        );
        insta::assert_snapshot!(
            "context_menu_radio_checked",
            snapshot_attrs(&api.radio_item_attrs(&key("charlie"), &key("density")))
        );
        insta::assert_snapshot!(
            "context_menu_sub_trigger_open",
            snapshot_attrs(&api.sub_trigger_attrs(&key("delta")))
        );
        insta::assert_snapshot!(
            "context_menu_sub_positioner",
            snapshot_attrs(&api.sub_positioner_attrs(&key("delta")))
        );
        insta::assert_snapshot!(
            "context_menu_sub_content",
            snapshot_attrs(&api.sub_content_attrs(&key("delta")))
        );
        insta::assert_snapshot!(
            "context_menu_shortcut",
            snapshot_attrs(&api.item_shortcut_attrs(&key("alpha")))
        );

        assert_eq!(
            api.part_attrs(Part::Target)
                .get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("menu")
        );
    }
}
