//! Menu selection component machine.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString as _},
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

use crate::overlay::positioning::PositioningOptions;

/// Open-state change callback.
pub type OpenChangeCallback = dyn Fn(bool) + Send + Sync;

/// Menu item action callback.
pub type ActionCallback = dyn Fn(Key) + Send + Sync;

/// User-facing payload for menu items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Human-readable item label.
    pub label: String,

    /// Menu item subtype.
    pub item_type: ItemType,

    /// Visual keyboard shortcut hint displayed beside the label.
    pub shortcut: Option<String>,

    /// Semantic keyboard shortcut exposed through `aria-keyshortcuts`.
    pub aria_keyshortcuts: Option<String>,

    /// Per-item override for menu-level close-on-action behavior.
    pub close_on_action: Option<bool>,
}

/// Menu item subtype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemType {
    /// Action menu item.
    Normal,

    /// Checkbox menu item.
    Checkbox,

    /// Radio menu item.
    Radio {
        /// Radio group identifier.
        group: String,
    },

    /// Submenu trigger item.
    Submenu,
}

/// Menu machine states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The menu is closed.
    #[default]
    Closed,

    /// The menu is open.
    Open,
}

/// Events accepted by the menu machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the menu.
    Open,

    /// Close the menu.
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

    /// Mark focus state.
    Focus {
        /// Whether focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// Mark the menu blurred.
    Blur,

    /// Close due to an outside click.
    ClickOutside,

    /// Search by type-ahead character and timestamp.
    TypeaheadSearch(char, u64),

    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),
}

/// Menu machine context.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Resolved locale for locale-aware type-ahead matching.
    pub locale: Locale,

    /// Item collection.
    pub items: StaticCollection<Item>,

    /// Whether the menu is open.
    pub open: bool,

    /// Highlighted item key.
    pub highlighted_key: Option<Key>,

    /// Whether menu focus is currently inside the component.
    pub focused: bool,

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
}

/// Props for the Menu component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Whether the menu is disabled.
    pub disabled: bool,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Adapter positioning configuration.
    pub positioning: PositioningOptions,

    /// Whether item activation closes the menu by default.
    pub close_on_action: bool,

    /// Disabled-item behavior for keyboard navigation.
    pub disabled_behavior: DisabledBehavior,

    /// Whether the final checked checkbox item cannot be unchecked.
    pub disallow_empty_selection: bool,

    /// Callback invoked when the menu open state changes.
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

impl Props {
    /// Returns default menu props.
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

    /// Sets [`Self::loop_focus`].
    #[must_use]
    pub const fn loop_focus(mut self, value: bool) -> Self {
        self.loop_focus = value;
        self
    }

    /// Sets [`Self::positioning`].
    #[must_use]
    pub fn positioning(mut self, value: PositioningOptions) -> Self {
        self.positioning = value;
        self
    }

    /// Sets [`Self::close_on_action`].
    #[must_use]
    pub const fn close_on_action(mut self, value: bool) -> Self {
        self.close_on_action = value;
        self
    }

    /// Sets [`Self::disabled_behavior`].
    #[must_use]
    pub const fn disabled_behavior(mut self, value: DisabledBehavior) -> Self {
        self.disabled_behavior = value;
        self
    }

    /// Sets [`Self::disallow_empty_selection`].
    #[must_use]
    pub const fn disallow_empty_selection(mut self, value: bool) -> Self {
        self.disallow_empty_selection = value;
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

/// Localized messages for the menu component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Menu anatomy parts.
#[derive(ComponentPart)]
#[scope = "menu"]
pub enum Part {
    /// Root container.
    Root,

    /// Trigger button.
    Trigger,

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

/// Machine for the menu component.
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
                focused: false,
                focus_visible: false,
                checked_items: BTreeMap::new(),
                radio_groups: BTreeMap::new(),
                submenu_open: None,
                typeahead: typeahead::State::default(),
                loop_focus: props.loop_focus,
                ids: ComponentIds::from_id(&props.id),
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
                Event::Open
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
            (State::Closed, Event::Open) => {
                let highlighted_key = first_key(ctx, props);

                Some(open_plan(highlighted_key, props))
            }

            (State::Open, Event::Close | Event::ClickOutside | Event::Blur) => {
                Some(close_plan(props))
            }

            (_, Event::Focus { is_keyboard }) => {
                let focus_visible = *is_keyboard;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = focus_visible;
                }))
            }

            (_, Event::HighlightItem(key)) => {
                let key = key
                    .clone()
                    .filter(|key| ctx.items.contains_key(key) && is_focusable_key(ctx, props, key));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightFirst) => {
                let key = first_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightLast) => {
                let key = last_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightNext) => {
                let key = next_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let key = prev_key(ctx, props);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
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
                        ctx.highlighted_key = Some(key);
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

            _ => None,
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

/// API for deriving menu attributes and dispatching menu events.
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

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Trigger);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
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
        let mut attrs = part_attrs(&Part::Positioner);

        attrs.set(
            HtmlAttr::Data("ars-placement"),
            self.props.positioning.placement.to_string(),
        );

        attrs
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
                self.ctx.ids.part("trigger"),
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
        let mut attrs = self.item_base_attrs(
            &Part::Item {
                key: Key::default(),
            },
            key,
            "menuitem",
        );

        if let Some(item) = self.item_payload(key)
            && let Some(shortcut) = &item.aria_keyshortcuts
        {
            attrs.set(HtmlAttr::Aria(AriaAttr::KeyShortcuts), shortcut);
        }

        attrs
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
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }

    /// Dispatches a trigger click event.
    pub fn on_trigger_click(&self) {
        if self.ctx.open {
            (self.send)(Event::Close);
        } else {
            (self.send)(Event::Open);
        }
    }

    /// Dispatches trigger keydown events.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        self.on_trigger_keydown_impl(data, ctrl, meta, None);
    }

    /// Dispatches trigger keydown events with an adapter-provided timestamp.
    pub fn on_trigger_keydown_at(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: u64,
    ) {
        self.on_trigger_keydown_impl(data, ctrl, meta, Some(now_ms));
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
        (self.send)(Event::HighlightItem(None));
    }

    fn on_trigger_keydown_impl(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: Option<u64>,
    ) {
        match data.key {
            KeyboardKey::ArrowDown | KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::Open);
            }

            KeyboardKey::Escape if self.ctx.open => (self.send)(Event::Close),

            _ if data.character.is_some() && !ctrl && !meta && !data.is_composing => {
                (self.send)(Event::TypeaheadSearch(
                    data.character.expect("checked"),
                    typeahead_time(now_ms, &self.ctx.typeahead),
                ));
            }

            _ => {}
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

            KeyboardKey::ArrowRight => {
                if let Some(key) = &self.ctx.highlighted_key
                    && item_type_matches(self.ctx, key, |item_type| {
                        matches!(item_type, ItemType::Submenu)
                    })
                {
                    (self.send)(Event::OpenSubmenu(key.clone()));
                }
            }

            KeyboardKey::ArrowLeft if self.ctx.submenu_open.is_some() => {
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

        attrs
    }

    fn item_payload(&self, key: &Key) -> Option<&Item> {
        self.ctx.items.get(key)?.value.as_ref()
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

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn open_plan(highlighted_key: Option<Key>, props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        ctx.open = true;
        ctx.highlighted_key = highlighted_key;

        if let Some(callback) = &on_open_change {
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
        ctx.focused = false;
        ctx.focus_visible = false;

        if was_open && let Some(callback) = &on_open_change {
            callback(false);
        }
    })
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

    if current && props.disallow_empty_selection && checked_count(ctx) <= 1 {
        return None;
    }

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
    ctx.highlighted_key
        .as_ref()
        .and_then(|key| {
            next_enabled_key(
                &ctx.items,
                key,
                &props.disabled_keys,
                props.disabled_behavior,
                ctx.loop_focus,
            )
        })
        .or_else(|| first_key(ctx, props))
}

fn prev_key(ctx: &Context, props: &Props) -> Option<Key> {
    ctx.highlighted_key
        .as_ref()
        .and_then(|key| {
            prev_enabled_key(
                &ctx.items,
                key,
                &props.disabled_keys,
                props.disabled_behavior,
                ctx.loop_focus,
            )
        })
        .or_else(|| last_key(ctx, props))
}

fn is_focusable_key(ctx: &Context, props: &Props, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable)
        && (!props.disabled_keys.contains(key)
            || props.disabled_behavior == DisabledBehavior::FocusOnly)
}

fn is_selectable_item(ctx: &Context, props: &Props, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable) && !props.disabled_keys.contains(key)
}

fn item_payload<'a>(ctx: &'a Context, key: &Key) -> Option<&'a Item> {
    ctx.items.get(key)?.value.as_ref()
}

fn item_type_matches(ctx: &Context, key: &Key, predicate: impl FnOnce(&ItemType) -> bool) -> bool {
    item_payload(ctx, key).is_some_and(|item| predicate(&item.item_type))
}

fn checked_count(ctx: &Context) -> usize {
    ctx.checked_items
        .values()
        .filter(|checked| **checked)
        .count()
}

fn invalidate_collection_references(ctx: &mut Context) {
    if ctx
        .highlighted_key
        .as_ref()
        .is_some_and(|key| !ctx.items.contains_key(key))
    {
        ctx.highlighted_key = None;
    }

    if ctx
        .submenu_open
        .as_ref()
        .is_some_and(|key| !ctx.items.contains_key(key))
    {
        ctx.submenu_open = None;
    }

    ctx.checked_items
        .retain(|key, _| ctx.items.contains_key(key));
    ctx.radio_groups
        .retain(|_, key| ctx.items.contains_key(key));
}

fn typeahead_time(now_ms: Option<u64>, state: &typeahead::State) -> u64 {
    now_ms.unwrap_or_else(|| state.last_key_time_ms.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, format, sync::Arc, vec, vec::Vec};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_collections::{Collection, CollectionBuilder, DisabledBehavior, Key, NodeType};
    use ars_core::{AriaAttr, Callback, ConnectApi, Env, HtmlAttr, KeyboardKey, Service};
    use ars_interactions::KeyboardEventData;

    use super::{Event, Item, ItemType, Machine, Messages, Part, Props, State};
    use crate::overlay::positioning::{Placement, PositioningOptions};

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

    fn alternate_collection() -> ars_collections::StaticCollection<Item> {
        CollectionBuilder::new()
            .item(key("echo"), "Echo", item("Echo", ItemType::Normal))
            .build()
    }

    fn checkbox_pair_collection() -> ars_collections::StaticCollection<Item> {
        CollectionBuilder::new()
            .item(key("alpha"), "Alpha", item("Alpha", ItemType::Checkbox))
            .item(key("bravo"), "Bravo", item("Bravo", ItemType::Checkbox))
            .build()
    }

    fn shortcut_collection() -> ars_collections::StaticCollection<Item> {
        CollectionBuilder::new()
            .item(
                key("save"),
                "Save",
                Item {
                    label: "Save".into(),
                    item_type: ItemType::Normal,
                    shortcut: Some("Ctrl+S".into()),
                    aria_keyshortcuts: Some("Control+S".into()),
                    close_on_action: None,
                },
            )
            .build()
    }

    fn service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

        drop(service.send(Event::UpdateItems(collection())));

        service
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

    #[test]
    fn content_and_item_attrs_emit_menu_roles() {
        let mut menu = service(Props::new().id("menu"));

        drop(menu.send(Event::Open));

        let api = menu.connect(&|_| {});

        assert_eq!(api.content_attrs().get(&HtmlAttr::Role), Some("menu"));
        assert_eq!(
            api.item_attrs(&key("alpha")).get(&HtmlAttr::Role),
            Some("menuitem")
        );
    }

    #[test]
    fn trigger_attrs_link_to_content_and_reflect_open_state() {
        let mut menu = service(Props::new().id("menu"));

        let closed = menu.connect(&|_| {}).trigger_attrs();

        assert_eq!(
            closed.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("menu")
        );
        assert_eq!(
            closed.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
        assert_eq!(
            closed.get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("menu-content")
        );

        drop(menu.send(Event::Open));

        assert_eq!(
            menu.connect(&|_| {})
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
    }

    #[test]
    fn arrow_key_navigation_moves_highlight() {
        let mut menu = service(Props::new().id("menu"));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::HighlightNext));

        assert_eq!(menu.context().highlighted_key, Some(key("bravo")));
    }

    #[test]
    fn direct_highlight_events_cover_first_last_prev_and_invalid_keys() {
        let mut menu = service(Props::new().id("menu").loop_focus(false));

        drop(menu.send(Event::Open));
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
            .find(|node| node.node_type == NodeType::Separator)
            .expect("fixture has a separator")
            .key
            .clone();

        drop(menu.send(Event::HighlightItem(Some(separator))));

        assert_eq!(menu.context().highlighted_key, None);
    }

    #[test]
    fn close_focus_and_open_change_transitions_update_context() {
        let open_changes = Arc::new(Mutex::new(Vec::new()));
        let mut menu = service(Props::new().id("menu").on_open_change(Callback::new({
            let open_changes = Arc::clone(&open_changes);
            move |open| {
                open_changes
                    .lock()
                    .expect("open-change capture poisoned")
                    .push(open);
            }
        })));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::Focus { is_keyboard: true }));

        assert!(menu.context().focused);
        assert!(menu.context().focus_visible);

        drop(menu.send(Event::Close));

        assert_eq!(menu.state(), &State::Closed);
        assert!(!menu.context().open);
        assert_eq!(menu.context().highlighted_key, None);
        assert!(!menu.context().focused);
        assert!(!menu.context().focus_visible);
        assert_eq!(
            open_changes
                .lock()
                .expect("open-change capture poisoned")
                .as_slice(),
            &[true, false]
        );

        drop(menu.send(Event::Close));

        assert_eq!(
            open_changes
                .lock()
                .expect("open-change capture poisoned")
                .as_slice(),
            &[true, false]
        );

        drop(menu.send(Event::Open));
        drop(menu.send(Event::ClickOutside));
        drop(menu.send(Event::Open));
        drop(menu.send(Event::Blur));

        assert_eq!(menu.state(), &State::Closed);
    }

    #[test]
    fn roving_tabindex_tracks_highlighted_item() {
        let mut menu = service(Props::new().id("menu"));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::HighlightItem(Some(key("charlie")))));

        let api = menu.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("alpha")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert_eq!(
            api.radio_item_attrs(&key("charlie"), &key("density"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn arrow_right_opens_submenu_and_arrow_left_closes_submenu() {
        let mut menu = service(Props::new().id("menu"));

        let captured = RefCell::new(Vec::new());

        drop(menu.send(Event::Open));
        drop(menu.send(Event::HighlightItem(Some(key("delta")))));

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_content_keydown(&keyboard(KeyboardKey::ArrowRight, None), false, false);
        }

        assert_eq!(
            captured.borrow().as_slice(),
            &[Event::OpenSubmenu(key("delta"))]
        );

        drop(menu.send(captured.take().remove(0)));

        assert_eq!(menu.context().submenu_open, Some(key("delta")));

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_content_keydown(&keyboard(KeyboardKey::ArrowLeft, None), false, false);
        }

        assert_eq!(captured.into_inner(), vec![Event::CloseSubmenu]);

        drop(menu.send(Event::CloseSubmenu));

        assert_eq!(menu.context().submenu_open, None);
        assert_eq!(menu.context().highlighted_key, Some(key("delta")));
    }

    #[test]
    fn enter_and_space_activate_normal_checkbox_radio_and_submenu_items() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        let captured = RefCell::new(Vec::new());

        drop(menu.send(Event::Open));

        for expected_key in ["alpha", "bravo", "charlie", "delta"] {
            drop(menu.send(Event::HighlightItem(Some(key(expected_key)))));

            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_content_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
        }

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::SelectItem(key("alpha")),
                Event::ToggleCheckboxItem(key("bravo")),
                Event::SelectRadioItem {
                    group: "density".into(),
                    value: key("charlie"),
                },
                Event::OpenSubmenu(key("delta")),
            ]
        );

        let captured = RefCell::new(Vec::new());

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_item_click(&key("alpha"));
            api.on_item_pointer_enter(&key("bravo"));
            api.on_content_pointer_leave();
            api.on_trigger_click();
        }

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::SelectItem(key("alpha")),
                Event::HighlightItem(Some(key("bravo"))),
                Event::HighlightItem(None),
                Event::Close,
            ]
        );
    }

    #[test]
    fn typeahead_matches_next_enabled_item() {
        let mut menu = service(Props::new().id("menu"));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::TypeaheadSearch('b', 100)));

        assert_eq!(menu.context().highlighted_key, Some(key("bravo")));

        drop(menu.send(Event::TypeaheadSearch('d', 700)));

        assert_eq!(menu.context().highlighted_key, Some(key("delta")));
    }

    #[test]
    fn disabled_items_are_skipped_or_focusable_based_on_disabled_behavior() {
        let mut skipped = service(
            Props::new()
                .id("menu")
                .disabled_keys(BTreeSet::from([key("alpha")]))
                .disabled_behavior(DisabledBehavior::Skip),
        );

        drop(skipped.send(Event::Open));

        assert_eq!(skipped.context().highlighted_key, Some(key("bravo")));

        let mut focusable = service(
            Props::new()
                .id("menu")
                .disabled_keys(BTreeSet::from([key("alpha")]))
                .disabled_behavior(DisabledBehavior::FocusOnly),
        );

        drop(focusable.send(Event::Open));
        drop(focusable.send(Event::HighlightItem(Some(key("alpha")))));

        assert_eq!(focusable.context().highlighted_key, Some(key("alpha")));
        assert_eq!(
            focusable
                .connect(&|_| {})
                .item_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );

        drop(focusable.send(Event::SelectItem(key("alpha"))));

        assert_eq!(focusable.state(), &State::Open);
    }

    #[test]
    fn escape_closes_menu() {
        let mut menu = service(Props::new().id("menu"));

        let captured = RefCell::new(Vec::new());

        drop(menu.send(Event::Open));

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_content_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
        }

        assert_eq!(captured.into_inner(), vec![Event::Close]);
    }

    #[test]
    fn submenu_trigger_attrs_reflect_open_state() {
        let mut menu = service(Props::new().id("menu"));

        drop(menu.send(Event::Open));

        let closed = menu.connect(&|_| {}).sub_trigger_attrs(&key("delta"));

        assert_eq!(
            closed.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("menu")
        );
        assert_eq!(
            closed.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );

        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        assert_eq!(
            menu.connect(&|_| {})
                .sub_trigger_attrs(&key("delta"))
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
    }

    #[test]
    fn checkbox_and_radio_attrs_emit_selection_roles_and_checked_state() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::SelectRadioItem {
            group: "density".into(),
            value: key("charlie"),
        }));

        let api = menu.connect(&|_| {});

        assert_eq!(
            api.checkbox_item_attrs(&key("bravo")).get(&HtmlAttr::Role),
            Some("menuitemcheckbox")
        );
        assert_eq!(
            api.checkbox_item_attrs(&key("bravo"))
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
        assert_eq!(
            api.radio_item_attrs(&key("charlie"), &key("density"))
                .get(&HtmlAttr::Role),
            Some("menuitemradio")
        );
        assert_eq!(
            api.radio_item_attrs(&key("charlie"), &key("density"))
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
    }

    #[test]
    fn close_on_action_honors_menu_default_and_item_override() {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let mut menu = service(Props::new().id("menu").close_on_action(false).on_action(
            Callback::new({
                let actions = Arc::clone(&actions);
                move |key: Key| {
                    actions.lock().expect("action capture poisoned").push(key);
                }
            }),
        ));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::SelectItem(key("alpha"))));

        assert_eq!(menu.state(), &State::Open);
        assert_eq!(
            actions.lock().expect("action capture poisoned").as_slice(),
            &[key("alpha")]
        );

        let mut closing = service(Props::new().id("menu"));

        drop(closing.send(Event::Open));
        drop(closing.send(Event::SelectItem(key("alpha"))));

        assert_eq!(closing.state(), &State::Closed);
    }

    #[test]
    fn disallow_empty_selection_blocks_final_checkbox_uncheck() {
        let mut menu = service(
            Props::new()
                .id("menu")
                .close_on_action(false)
                .disallow_empty_selection(true),
        );

        drop(menu.send(Event::Open));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));

        assert_eq!(menu.context().checked_items.get(&key("bravo")), Some(&true));
    }

    #[test]
    fn stale_or_structural_item_events_do_not_mutate_state() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        drop(menu.send(Event::Open));

        let separator = menu
            .context()
            .items
            .nodes()
            .find(|node| node.node_type == NodeType::Separator)
            .expect("fixture has a separator")
            .key
            .clone();

        drop(menu.send(Event::SelectItem(key("missing"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("missing"))));
        drop(menu.send(Event::SelectItem(separator.clone())));
        drop(menu.send(Event::ToggleCheckboxItem(separator.clone())));
        drop(menu.send(Event::OpenSubmenu(separator)));

        assert_eq!(menu.state(), &State::Open);
        assert!(menu.context().checked_items.is_empty());
        assert_eq!(menu.context().submenu_open, None);
    }

    #[test]
    fn radio_selection_requires_matching_declared_group() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::SelectRadioItem {
            group: "spacing".into(),
            value: key("charlie"),
        }));

        assert!(menu.context().radio_groups.is_empty());
    }

    #[test]
    fn disallow_empty_selection_allows_unchecking_when_another_checkbox_remains_checked() {
        let mut menu = service(
            Props::new()
                .id("menu")
                .close_on_action(false)
                .disallow_empty_selection(true),
        );

        drop(menu.send(Event::UpdateItems(checkbox_pair_collection())));
        drop(menu.send(Event::Open));
        drop(menu.send(Event::ToggleCheckboxItem(key("alpha"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("alpha"))));

        assert_eq!(
            menu.context().checked_items.get(&key("alpha")),
            Some(&false)
        );
        assert_eq!(menu.context().checked_items.get(&key("bravo")), Some(&true));
    }

    #[test]
    fn items_and_radio_group_attrs_expose_collection_and_group_metadata() {
        let menu = service(Props::new().id("menu"));

        let api = menu.connect(&|_| {});

        assert_eq!(api.items().count(), menu.context().items.nodes().count());

        let radio_group = api.radio_group_attrs(&key("density"));

        assert_eq!(radio_group.get(&HtmlAttr::Role), Some("group"));
        assert_eq!(
            radio_group.get(&HtmlAttr::Data("ars-part")),
            Some("radio-group")
        );
        assert_eq!(
            radio_group.get(&HtmlAttr::Id),
            Some("menu-radio-group-density")
        );
    }

    #[test]
    fn keyboard_helpers_dispatch_navigation_activation_and_modifier_filtered_typeahead() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        assert_eq!(
            captured_events(&menu, |api| {
                api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowDown, None), false, false);
                api.on_trigger_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
                api.on_trigger_keydown(&keyboard(KeyboardKey::Space, Some(' ')), false, false);
                api.on_trigger_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
                api.on_trigger_keydown(
                    &keyboard(KeyboardKey::Unidentified, Some('x')),
                    true,
                    false,
                );
                api.on_trigger_keydown(
                    &keyboard(KeyboardKey::Unidentified, Some('x')),
                    false,
                    true,
                );
                let mut composing = keyboard(KeyboardKey::Unidentified, Some('x'));
                composing.is_composing = true;
                api.on_trigger_keydown(&composing, false, false);
            }),
            vec![Event::Open, Event::Open, Event::Open]
        );

        drop(menu.send(Event::Open));

        assert_eq!(
            captured_events(&menu, |api| {
                api.on_trigger_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
            }),
            vec![Event::Close]
        );

        assert_eq!(
            captured_events(&menu, |api| {
                api.on_content_keydown(&keyboard(KeyboardKey::ArrowDown, None), false, false);
                api.on_content_keydown(&keyboard(KeyboardKey::ArrowUp, None), false, false);
                api.on_content_keydown(&keyboard(KeyboardKey::Home, None), false, false);
                api.on_content_keydown(&keyboard(KeyboardKey::End, None), false, false);
                api.on_content_keydown(&keyboard(KeyboardKey::ArrowRight, None), false, false);
                api.on_content_keydown(&keyboard(KeyboardKey::ArrowLeft, None), false, false);
                api.on_content_keydown(
                    &keyboard(KeyboardKey::Unidentified, Some('b')),
                    true,
                    false,
                );
                api.on_content_keydown(
                    &keyboard(KeyboardKey::Unidentified, Some('b')),
                    false,
                    true,
                );
                let mut composing = keyboard(KeyboardKey::Unidentified, Some('b'));
                composing.is_composing = true;
                api.on_content_keydown(&composing, false, false);
            }),
            vec![
                Event::HighlightNext,
                Event::HighlightPrev,
                Event::HighlightFirst,
                Event::HighlightLast,
            ]
        );

        drop(menu.send(Event::HighlightItem(Some(key("delta")))));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));

        assert_eq!(
            captured_events(&menu, |api| {
                api.on_content_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
            }),
            vec![Event::CloseSubmenu]
        );
    }

    #[test]
    fn update_items_clears_stale_highlight_and_submenu() {
        let mut menu = service(Props::new().id("menu").close_on_action(false));

        drop(menu.send(Event::Open));
        drop(menu.send(Event::OpenSubmenu(key("delta"))));
        drop(menu.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(menu.send(Event::SelectRadioItem {
            group: "density".into(),
            value: key("charlie"),
        }));
        drop(menu.send(Event::UpdateItems(alternate_collection())));

        assert_eq!(menu.context().highlighted_key, None);
        assert_eq!(menu.context().submenu_open, None);
        assert!(menu.context().checked_items.is_empty());
        assert!(menu.context().radio_groups.is_empty());
    }

    #[test]
    fn keyboard_helpers_emit_typeahead_with_or_without_adapter_timestamps() {
        let menu = service(Props::new().id("menu"));

        let captured = RefCell::new(Vec::new());

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = menu.connect(&send);

            api.on_trigger_keydown(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
            );
            api.on_trigger_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
                100,
            );
            api.on_content_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('d')),
                false,
                false,
                200,
            );
        }

        let captured = captured.into_inner();

        assert!(matches!(
            captured[0],
            Event::TypeaheadSearch('b', timestamp) if timestamp > 0
        ));
        assert_eq!(captured[1], Event::TypeaheadSearch('b', 100));
        assert_eq!(captured[2], Event::TypeaheadSearch('d', 200));
    }

    #[test]
    fn menu_attr_snapshots_cover_anatomy_and_state_branches() {
        let mut menu = service(
            Props::new()
                .id("menu")
                .disabled(true)
                .disabled_keys(BTreeSet::from([key("alpha")]))
                .disabled_behavior(DisabledBehavior::FocusOnly),
        );

        drop(menu.send(Event::UpdateItems(shortcut_collection())));
        drop(menu.send(Event::HighlightItem(Some(key("save")))));

        let api = menu.connect(&|_| {});

        insta::assert_snapshot!(
            "menu_root_default",
            snapshot_attrs(
                &service(Props::new().id("menu"))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
        insta::assert_snapshot!("menu_root_disabled", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!(
            "menu_trigger_closed_disabled",
            snapshot_attrs(&api.trigger_attrs())
        );
        insta::assert_snapshot!("menu_positioner", snapshot_attrs(&api.positioner_attrs()));
        insta::assert_snapshot!("menu_arrow", snapshot_attrs(&api.arrow_attrs()));
        insta::assert_snapshot!("menu_content", snapshot_attrs(&api.content_attrs()));
        insta::assert_snapshot!(
            "menu_item_shortcut",
            snapshot_attrs(&api.item_attrs(&key("save")))
        );
        insta::assert_snapshot!(
            "menu_item_text",
            snapshot_attrs(&api.item_text_attrs(&key("save")))
        );
        insta::assert_snapshot!(
            "menu_item_indicator",
            snapshot_attrs(&api.item_indicator_attrs(&key("save")))
        );
        insta::assert_snapshot!("menu_separator", snapshot_attrs(&api.separator_attrs()));
        insta::assert_snapshot!(
            "menu_item_group",
            snapshot_attrs(&api.item_group_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "menu_item_group_label",
            snapshot_attrs(&api.item_group_label_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "menu_shortcut",
            snapshot_attrs(&api.item_shortcut_attrs(&key("save")))
        );

        let mut open = service(Props::new().id("menu").close_on_action(false));

        drop(open.send(Event::Open));
        drop(open.send(Event::ToggleCheckboxItem(key("bravo"))));
        drop(open.send(Event::SelectRadioItem {
            group: "density".into(),
            value: key("charlie"),
        }));
        drop(open.send(Event::HighlightItem(Some(key("alpha")))));

        let api = open.connect(&|_| {});

        insta::assert_snapshot!("menu_trigger_open", snapshot_attrs(&api.trigger_attrs()));
        insta::assert_snapshot!(
            "menu_item_default",
            snapshot_attrs(&api.item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "menu_item_highlighted",
            snapshot_attrs(&api.item_attrs(&key("alpha")))
        );
        insta::assert_snapshot!(
            "menu_checkbox_unchecked",
            snapshot_attrs(&api.checkbox_item_attrs(&key("alpha")))
        );
        insta::assert_snapshot!(
            "menu_checkbox_checked",
            snapshot_attrs(&api.checkbox_item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "menu_checkbox_disabled",
            snapshot_attrs(
                &service(
                    Props::new()
                        .id("menu")
                        .disabled_keys(BTreeSet::from([key("bravo")]))
                )
                .connect(&|_| {})
                .checkbox_item_attrs(&key("bravo"))
            )
        );
        insta::assert_snapshot!(
            "menu_radio_unchecked",
            snapshot_attrs(&api.radio_item_attrs(&key("alpha"), &key("density")))
        );
        insta::assert_snapshot!(
            "menu_radio_checked",
            snapshot_attrs(&api.radio_item_attrs(&key("charlie"), &key("density")))
        );
        insta::assert_snapshot!(
            "menu_radio_disabled",
            snapshot_attrs(
                &service(
                    Props::new()
                        .id("menu")
                        .disabled_keys(BTreeSet::from([key("charlie")]))
                )
                .connect(&|_| {})
                .radio_item_attrs(&key("charlie"), &key("density"))
            )
        );
        insta::assert_snapshot!(
            "menu_sub_trigger_closed",
            snapshot_attrs(&api.sub_trigger_attrs(&key("delta")))
        );

        drop(open.send(Event::OpenSubmenu(key("delta"))));

        let api = open.connect(&|_| {});

        insta::assert_snapshot!(
            "menu_sub_trigger_open",
            snapshot_attrs(&api.sub_trigger_attrs(&key("delta")))
        );
        insta::assert_snapshot!(
            "menu_sub_positioner",
            snapshot_attrs(&api.sub_positioner_attrs(&key("delta")))
        );
        insta::assert_snapshot!(
            "menu_sub_content",
            snapshot_attrs(&api.sub_content_attrs(&key("delta")))
        );
    }

    #[test]
    fn builder_methods_preserve_public_props_contract() {
        let positioning = PositioningOptions {
            placement: Placement::RightStart,
            ..PositioningOptions::default()
        };

        let props = Props::new()
            .id("menu")
            .disabled(true)
            .loop_focus(false)
            .positioning(positioning.clone())
            .close_on_action(false)
            .disabled_behavior(DisabledBehavior::FocusOnly)
            .disabled_keys(BTreeSet::from([key("alpha")]))
            .disallow_empty_selection(true)
            .on_open_change(Callback::new(|_open: bool| {}))
            .on_action(Callback::new(|_key: Key| {}));

        assert_eq!(props.id, "menu");
        assert!(props.disabled);
        assert!(!props.loop_focus);
        assert_eq!(props.positioning, positioning);
        assert!(!props.close_on_action);
        assert_eq!(props.disabled_behavior, DisabledBehavior::FocusOnly);
        assert_eq!(props.disabled_keys, BTreeSet::from([key("alpha")]));
        assert!(props.disallow_empty_selection);
        assert!(props.on_open_change.is_some());
        assert!(props.on_action.is_some());
    }

    #[test]
    fn connect_api_dispatches_all_declared_parts() {
        let menu = service(Props::new().id("menu"));

        let api = menu.connect(&|_| {});

        assert!(
            api.part_attrs(Part::Root)
                .contains(&HtmlAttr::Data("ars-scope"))
        );
        assert!(matches!(menu.state(), State::Closed));
    }
}
