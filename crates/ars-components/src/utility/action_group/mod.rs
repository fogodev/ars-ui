//! ActionGroup component state machine and connect API.
//!
//! This module implements the framework-agnostic `ActionGroup` core. Framework
//! adapters own DOM measurement, menu rendering, and direct focus movement;
//! this module owns typed props, logical item registration, roving-focus
//! intent, selection state, overflow state, and `AttrMap` output.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_collections::{Key, selection};
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, KeyboardKey, Locale, MessageFn, Orientation, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The states for the `ActionGroup` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// Default resting state. No item is focused via roving focus.
    #[default]
    Idle,

    /// An item within the action group has logical focus.
    Focused {
        /// The key of the item that has focus.
        item: Key,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused { .. } => f.write_str("focused"),
        }
    }
}

/// Events accepted by the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus a specific registered item by key.
    FocusItem(Key),

    /// Focus left the action group.
    Blur,

    /// Move focus to the next registered enabled item, wrapping at the end.
    FocusNext,

    /// Move focus to the previous registered enabled item, wrapping at the start.
    FocusPrev,

    /// Move focus to the first registered enabled item.
    FocusFirst,

    /// Move focus to the last registered enabled item.
    FocusLast,

    /// Activate an item by key without changing selection.
    ActivateItem(Key),

    /// Toggle selection state by key when selection is enabled.
    SelectItem(Key),

    /// Synchronize the number of items moved to overflow.
    OverflowChanged(usize),

    /// Register a rendered item key in logical render order.
    RegisterItem(Key),

    /// Unregister a rendered item key.
    UnregisterItem(Key),

    /// Synchronize context-backed props.
    SetProps,
}

/// How text labels are displayed alongside icons in action items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonLabelBehavior {
    /// Always show text labels alongside icons.
    #[default]
    Show,

    /// Collapse text labels to icon-only affordances when space is limited.
    Collapse,

    /// Hide text labels entirely, leaving icon-only buttons.
    Hide,
}

impl ButtonLabelBehavior {
    /// Returns the data-attribute token for this label behavior.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Collapse => "collapse",
            Self::Hide => "hide",
        }
    }
}

impl Display for ButtonLabelBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Visual variant for `ActionGroup` styling hooks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Default toolbar appearance.
    #[default]
    Toolbar,

    /// Outlined button group with visible borders.
    Outlined,

    /// Flat or borderless button group.
    Flat,
}

impl Variant {
    /// Returns the data-attribute token for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Toolbar => "toolbar",
            Self::Outlined => "outlined",
            Self::Flat => "flat",
        }
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Overflow behavior for action items that do not fit inline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OverflowMode {
    /// Items wrap to the next line.
    #[default]
    Wrap,

    /// Items that do not fit are hidden by the adapter/CSS layer.
    Collapse,

    /// Items that do not fit are moved to an overflow menu.
    Menu,
}

impl OverflowMode {
    /// Returns the data-attribute token for this overflow mode.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wrap => "wrap",
            Self::Collapse => "collapse",
            Self::Menu => "menu",
        }
    }
}

impl Display for OverflowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Props for the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The orientation of the action group.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// The overflow mode for the action group.
    pub overflow_mode: OverflowMode,

    /// Visual variant for styling hooks.
    pub variant: Variant,

    /// Whether the whole action group is disabled.
    pub disabled: bool,

    /// Keys of individually disabled items.
    pub disabled_items: BTreeSet<Key>,

    /// The selection mode for the action group.
    pub selection_mode: selection::Mode,

    /// Optional explicit number of actions to keep visible.
    pub max_visible_actions: Option<usize>,

    /// How text labels are displayed alongside icons.
    pub button_label_behavior: ButtonLabelBehavior,

    /// Density hint exposed as `data-ars-density`.
    pub density: Option<String>,

    /// Whether items stretch to fill available space equally.
    pub justified: bool,

    /// Accessible label for the toolbar root.
    pub aria_label: Option<String>,

    /// ID of the element that labels this toolbar.
    pub aria_labelledby: Option<String>,

    /// Callback invoked when an item is activated.
    pub on_action: Option<Callback<dyn Fn(Key) + Send + Sync>>,

    /// Callback invoked when the selected key set changes.
    pub on_selection_change: Option<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: Orientation::Horizontal,
            dir: Direction::default(),
            overflow_mode: OverflowMode::Wrap,
            variant: Variant::Toolbar,
            disabled: false,
            disabled_items: BTreeSet::new(),
            selection_mode: selection::Mode::None,
            max_visible_actions: None,
            button_label_behavior: ButtonLabelBehavior::Show,
            density: None,
            justified: false,
            aria_label: None,
            aria_labelledby: None,
            on_action: None,
            on_selection_change: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = dir;
        self
    }

    /// Sets [`overflow_mode`](Self::overflow_mode).
    #[must_use]
    pub const fn overflow_mode(mut self, overflow_mode: OverflowMode) -> Self {
        self.overflow_mode = overflow_mode;
        self
    }

    /// Sets [`variant`](Self::variant).
    #[must_use]
    pub const fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`disabled_items`](Self::disabled_items).
    #[must_use]
    pub fn disabled_items(mut self, disabled_items: BTreeSet<Key>) -> Self {
        self.disabled_items = disabled_items;
        self
    }

    /// Sets [`selection_mode`](Self::selection_mode).
    #[must_use]
    pub const fn selection_mode(mut self, selection_mode: selection::Mode) -> Self {
        self.selection_mode = selection_mode;
        self
    }

    /// Sets [`max_visible_actions`](Self::max_visible_actions).
    #[must_use]
    pub const fn max_visible_actions(mut self, max_visible_actions: Option<usize>) -> Self {
        self.max_visible_actions = max_visible_actions;
        self
    }

    /// Sets [`button_label_behavior`](Self::button_label_behavior).
    #[must_use]
    pub const fn button_label_behavior(
        mut self,
        button_label_behavior: ButtonLabelBehavior,
    ) -> Self {
        self.button_label_behavior = button_label_behavior;
        self
    }

    /// Sets [`density`](Self::density).
    #[must_use]
    pub fn density(mut self, density: impl Into<String>) -> Self {
        self.density = Some(density.into());
        self
    }

    /// Sets [`justified`](Self::justified).
    #[must_use]
    pub const fn justified(mut self, justified: bool) -> Self {
        self.justified = justified;
        self
    }

    /// Sets [`aria_label`](Self::aria_label).
    #[must_use]
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets [`aria_labelledby`](Self::aria_labelledby).
    #[must_use]
    pub fn aria_labelledby(mut self, labelledby: impl Into<String>) -> Self {
        self.aria_labelledby = Some(labelledby.into());
        self
    }

    /// Sets [`on_action`](Self::on_action).
    #[must_use]
    pub fn on_action(mut self, callback: impl Into<Callback<dyn Fn(Key) + Send + Sync>>) -> Self {
        self.on_action = Some(callback.into());
        self
    }

    /// Sets [`on_selection_change`](Self::on_selection_change).
    #[must_use]
    pub fn on_selection_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
    ) -> Self {
        self.on_selection_change = Some(callback.into());
        self
    }
}

/// The context for the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the action group is disabled.
    pub disabled: bool,

    /// The key of the currently focused item.
    pub focused_item: Option<Key>,

    /// The keys of the currently selected items.
    pub selected_items: BTreeSet<Key>,

    /// The number of items that overflowed into the menu.
    pub overflow_count: usize,

    /// The number of items visible directly in the toolbar.
    pub visible_count: usize,

    /// All registered item keys in logical render order.
    pub registered_items: Vec<Key>,

    /// Text direction used for horizontal arrow-key navigation.
    pub dir: Direction,

    /// Active locale inherited from provider environment.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

/// Messages for the `ActionGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the overflow menu trigger.
    pub overflow_trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Fallback accessible label for the toolbar root.
    pub toolbar_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            overflow_trigger_label: MessageFn::static_str("More actions"),
            toolbar_label: MessageFn::static_str("Actions"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the action group machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter or callback layer invokes an item action.
    Action,

    /// Adapter or callback layer observes a selection set change.
    SelectionChange,

    /// Adapter moves live DOM focus to the logical focused item.
    FocusItem,
}

/// The machine for the `ActionGroup` component.
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
        (
            State::Idle,
            Context {
                disabled: props.disabled,
                focused_item: None,
                selected_items: BTreeSet::new(),
                overflow_count: 0,
                visible_count: 0,
                registered_items: Vec::new(),
                dir: props.dir,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && matches!(event, Event::ActivateItem(_) | Event::SelectItem(_)) {
            return None;
        }

        match (state, event) {
            (_, Event::FocusItem(item)) => {
                if !can_focus_item(ctx, props, item) {
                    return None;
                }

                let item = item.clone();

                if matches!(state, State::Focused { item: current } if current == &item)
                    && ctx.focused_item.as_ref() == Some(&item)
                {
                    return None;
                }

                Some(focus_plan(item))
            }

            (_, Event::Blur) => {
                if matches!(state, State::Idle) && ctx.focused_item.is_none() {
                    return None;
                }

                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_item = None;
                }))
            }

            (State::Idle, Event::FocusNext) | (_, Event::FocusFirst) => {
                let target = first_enabled(ctx, props)?;
                Some(focus_plan(target))
            }

            (State::Idle, Event::FocusPrev) | (_, Event::FocusLast) => {
                let target = last_enabled(ctx, props)?;
                Some(focus_plan(target))
            }

            (State::Focused { item }, Event::FocusNext) => {
                let target = step_focus(ctx, props, item, FocusStep::Next)?;
                Some(focus_plan(target))
            }

            (State::Focused { item }, Event::FocusPrev) => {
                let target = step_focus(ctx, props, item, FocusStep::Prev)?;
                Some(focus_plan(target))
            }

            (_, Event::ActivateItem(item)) => {
                if is_item_disabled(ctx, props, item) {
                    return None;
                }

                let item = item.clone();
                Some(TransitionPlan::new().with_effect(PendingEffect::new(
                    Effect::Action,
                    move |_ctx: &Context, props: &Props, _send| {
                        if let Some(callback) = &props.on_action {
                            callback(item);
                        }

                        no_cleanup()
                    },
                )))
            }

            (_, Event::SelectItem(item)) => {
                if is_item_disabled(ctx, props, item) {
                    return None;
                }

                let mut next = ctx.selected_items.clone();

                match props.selection_mode {
                    selection::Mode::None => return None,

                    selection::Mode::Single => {
                        if next.contains(item) {
                            next.clear();
                        } else {
                            next.clear();

                            next.insert(item.clone());
                        }
                    }

                    selection::Mode::Multiple => {
                        if !next.remove(item) {
                            next.insert(item.clone());
                        }
                    }
                }

                Some(selection_change_plan(next))
            }

            (_, Event::OverflowChanged(count)) => {
                let count = *count;
                let visible_count = ctx.registered_items.len().saturating_sub(count);

                if ctx.overflow_count == count && ctx.visible_count == visible_count {
                    return Some(TransitionPlan::new());
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.overflow_count = count;
                    ctx.visible_count = ctx.registered_items.len().saturating_sub(count);
                }))
            }

            (_, Event::RegisterItem(item)) => {
                if ctx
                    .registered_items
                    .iter()
                    .any(|registered| registered == item)
                {
                    return None;
                }

                let item = item.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.registered_items.push(item);
                    ctx.visible_count = ctx
                        .registered_items
                        .len()
                        .saturating_sub(ctx.overflow_count);
                }))
            }

            (_, Event::UnregisterItem(item)) => {
                if !ctx
                    .registered_items
                    .iter()
                    .any(|registered| registered == item)
                {
                    return None;
                }

                let item = item.clone();
                let focused_removed = ctx.focused_item.as_ref() == Some(&item);

                let plan = if focused_removed {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    ctx.registered_items
                        .retain(|registered| registered != &item);
                    ctx.selected_items.remove(&item);
                    ctx.visible_count = ctx
                        .registered_items
                        .len()
                        .saturating_sub(ctx.overflow_count);

                    if focused_removed {
                        ctx.focused_item = None;
                    }
                }))
            }

            (_, Event::SetProps) => Some(sync_props_plan(ctx, props)),
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "action_group::Props.id must remain stable after init",
        );

        if old.disabled != new.disabled
            || old.dir != new.dir
            || old.disabled_items != new.disabled_items
            || old.selection_mode != new.selection_mode
        {
            vec![Event::SetProps]
        } else {
            Vec::new()
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
}

/// DOM parts of the `ActionGroup` component.
#[derive(ComponentPart)]
#[scope = "action-group"]
pub enum Part {
    /// The root toolbar element.
    Root,

    /// An item button within the group.
    Item {
        /// Stable item key.
        item_id: Key,
    },

    /// The button that opens the overflow menu.
    OverflowTrigger,
}

/// The API for the `ActionGroup` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("action_group::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Root toolbar attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "toolbar")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.props.orientation),
            )
            .set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str())
            .set(
                HtmlAttr::Data("ars-overflow-mode"),
                self.props.overflow_mode.as_str(),
            )
            .set(
                HtmlAttr::Data("ars-label-behavior"),
                self.props.button_label_behavior.as_str(),
            );

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if let Some(density) = &self.props.density {
            attrs.set(HtmlAttr::Data("ars-density"), density.clone());
        }

        if self.props.justified {
            attrs.set_bool(HtmlAttr::Data("ars-justified"), true);
        }

        if let Some(labelledby) = &self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby.clone());
        } else if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.clone());
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.toolbar_label)(&self.ctx.locale),
            );
        }

        attrs
    }

    /// Item button attributes.
    #[must_use]
    pub fn item_attrs(&self, id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            item_id: Key::default(),
        }
        .data_attrs();

        let selected = self.ctx.selected_items.contains(id);
        let focused = self.ctx.focused_item.as_ref() == Some(id);
        let disabled = self.is_item_disabled(id);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-key"), id.to_string())
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, self.item_tabindex(id))
            .set(
                HtmlAttr::Data("ars-state"),
                if selected { "selected" } else { "idle" },
            );

        if focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        match self.props.selection_mode {
            selection::Mode::Single | selection::Mode::Multiple => {
                attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), bool_token(selected));

                if selected {
                    attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
                }
            }

            selection::Mode::None => {}
        }

        attrs
    }

    /// Overflow trigger button attributes.
    #[must_use]
    pub fn overflow_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::OverflowTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.overflow_trigger_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu")
            .set(HtmlAttr::Aria(AriaAttr::Expanded), "false");

        attrs
    }

    /// Returns whether a specific item is disabled by group or item state.
    #[must_use]
    pub fn is_item_disabled(&self, id: &Key) -> bool {
        is_item_disabled(self.ctx, self.props, id)
    }

    /// Returns whether the item at `index` is currently overflowed.
    #[must_use]
    pub const fn is_overflowed(&self, index: usize) -> bool {
        index >= self.ctx.visible_count
    }

    /// Dispatches keyboard navigation for an item.
    pub fn on_item_keydown(&self, data: &KeyboardEventData) {
        let horizontal = self.props.orientation == Orientation::Horizontal;
        let rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight if horizontal && rtl => (self.send)(Event::FocusPrev),
            KeyboardKey::ArrowRight if horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowLeft if horizontal && rtl => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowLeft if horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::ArrowDown if !horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp if !horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(item) = &self.ctx.focused_item {
                    (self.send)(Event::ActivateItem(item.clone()));
                }
            }
            _ => {}
        }
    }

    /// Dispatches focus for an item.
    pub fn on_item_focus(&self, id: &Key) {
        (self.send)(Event::FocusItem(id.clone()));
    }

    /// Dispatches a group blur event.
    pub fn on_item_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches activation for an item.
    pub fn on_item_click(&self, id: &Key) {
        if !self.is_item_disabled(id) {
            (self.send)(Event::ActivateItem(id.clone()));
        }
    }

    /// Dispatches item mount registration.
    pub fn on_item_mount(&self, id: &Key) {
        (self.send)(Event::RegisterItem(id.clone()));
    }

    /// Dispatches item unmount registration.
    pub fn on_item_unmount(&self, id: &Key) {
        (self.send)(Event::UnregisterItem(id.clone()));
    }

    fn item_tabindex(&self, id: &Key) -> &'static str {
        if self.props.disabled_items.contains(id) {
            return "-1";
        }

        if self.is_roving_anchor(id) { "0" } else { "-1" }
    }

    fn is_roving_anchor(&self, id: &Key) -> bool {
        match self.state {
            State::Focused { item } => item == id,
            State::Idle => first_enabled(self.ctx, self.props).as_ref() == Some(id),
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref item_id } => self.item_attrs(item_id),
            Part::OverflowTrigger => self.overflow_trigger_attrs(),
        }
    }
}

#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn is_item_disabled(ctx: &Context, props: &Props, item: &Key) -> bool {
    ctx.disabled || props.disabled_items.contains(item)
}

fn can_focus_item(ctx: &Context, props: &Props, item: &Key) -> bool {
    ctx.registered_items
        .iter()
        .any(|registered| registered == item)
        && !props.disabled_items.contains(item)
}

fn first_enabled(ctx: &Context, props: &Props) -> Option<Key> {
    ctx.registered_items
        .iter()
        .find(|item| !props.disabled_items.contains(*item))
        .cloned()
}

fn last_enabled(ctx: &Context, props: &Props) -> Option<Key> {
    ctx.registered_items
        .iter()
        .rev()
        .find(|item| !props.disabled_items.contains(*item))
        .cloned()
}

fn step_focus(ctx: &Context, props: &Props, current: &Key, step: FocusStep) -> Option<Key> {
    let current_index = ctx
        .registered_items
        .iter()
        .position(|registered| registered == current)?;

    let len = ctx.registered_items.len();

    for offset in 1..=len {
        let next_index = match step {
            FocusStep::Next => (current_index + offset) % len,

            FocusStep::Prev => {
                if current_index >= offset {
                    current_index - offset
                } else {
                    len - (offset - current_index)
                }
            }
        };

        let candidate = ctx.registered_items.get(next_index)?;

        if !props.disabled_items.contains(candidate) {
            return Some(candidate.clone());
        }
    }

    None
}

fn focus_plan(item: Key) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused { item: item.clone() })
        .apply(move |ctx: &mut Context| {
            ctx.focused_item = Some(item);
        })
        .with_effect(PendingEffect::named(Effect::FocusItem))
}

fn selection_change_plan(next: BTreeSet<Key>) -> TransitionPlan<Machine> {
    let effect_value = next.clone();

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.selected_items = next;
    })
    .with_effect(PendingEffect::new(
        Effect::SelectionChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_selection_change {
                callback(effect_value);
            }

            no_cleanup()
        },
    ))
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let disabled = props.disabled;
    let dir = props.dir;
    let disabled_items = props.disabled_items.clone();
    let selection_mode = props.selection_mode;

    let mut selected_items = normalize_selection(ctx.selected_items.clone(), selection_mode);

    selected_items.retain(|item| !disabled_items.contains(item));

    let focused_item = ctx
        .focused_item
        .as_ref()
        .filter(|item| !disabled_items.contains(*item))
        .cloned();

    let target = if let Some(item) = &focused_item {
        State::Focused { item: item.clone() }
    } else {
        State::Idle
    };

    TransitionPlan::to(target).apply(move |ctx: &mut Context| {
        ctx.disabled = disabled;
        ctx.dir = dir;
        ctx.selected_items = selected_items;
        ctx.focused_item = focused_item;
    })
}

fn normalize_selection(mut selected: BTreeSet<Key>, mode: selection::Mode) -> BTreeSet<Key> {
    match mode {
        selection::Mode::None => BTreeSet::new(),
        selection::Mode::Multiple => selected,
        selection::Mode::Single => selected.pop_first().into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, string::String, sync::Arc, vec, vec::Vec};
    use std::sync::Mutex;

    use ars_collections::{Key, selection};
    use ars_core::{
        AriaAttr, AttrMap, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, MessageFn,
        Orientation, Service, StrongSend, callback,
    };
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn key_set(values: &[&str]) -> BTreeSet<Key> {
        values.iter().map(|value| key(value)).collect()
    }

    fn keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn props() -> Props {
        Props::new().id("actions").aria_label("Document actions")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::new(props, &Env::default(), &Messages::default())
    }

    fn register(service: &mut Service<Machine>, values: &[&str]) {
        for value in values {
            drop(service.send(Event::RegisterItem(key(value))));
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn initial_state_is_idle() {
        let service = service(props());

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().disabled);
        assert_eq!(service.context().focused_item, None);
        assert!(service.context().selected_items.is_empty());
        assert_eq!(service.context().overflow_count, 0);
        assert_eq!(service.context().visible_count, 0);
        assert!(service.context().registered_items.is_empty());
    }

    #[test]
    fn focus_item_transitions_to_focused_with_roving_tabindex() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::FocusItem(key("delete"))));

        assert_eq!(
            service.state(),
            &State::Focused {
                item: key("delete"),
            },
        );
        assert_eq!(service.context().focused_item, Some(key("delete")));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("copy")).get(&HtmlAttr::TabIndex),
            Some("-1"),
        );
        assert_eq!(
            api.item_attrs(&key("delete")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
    }

    #[test]
    fn focus_item_ignores_unregistered_item() {
        let mut service = service(props());

        let result = service.send(Event::FocusItem(key("missing")));

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
    }

    #[test]
    fn focus_item_ignores_item_disabled_key() {
        let mut service = service(props().disabled_items(key_set(&["delete"])));

        register(&mut service, &["copy", "delete"]);

        let result = service.send(Event::FocusItem(key("delete")));

        assert!(!result.state_changed);
        assert_eq!(service.context().focused_item, None);
    }

    #[test]
    fn activate_item_emits_action_effect() {
        let actions = Arc::new(Mutex::new(Vec::new()));

        let mut service = service(props().on_action(callback({
            let actions = Arc::clone(&actions);
            move |item: Key| {
                actions.lock().unwrap().push(item);
            }
        })));

        register(&mut service, &["copy"]);

        let result = service.send(Event::ActivateItem(key("copy")));

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(service.context().selected_items.is_empty());
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::Action);

        let send: StrongSend<Event> = Arc::new(|_| {});
        let effect = result.pending_effects.into_iter().next().unwrap();
        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(*actions.lock().unwrap(), vec![key("copy")]);
    }

    #[test]
    fn disabled_group_blocks_activation_and_selection() {
        let mut service = service(
            props()
                .disabled(true)
                .selection_mode(selection::Mode::Multiple),
        );

        register(&mut service, &["copy", "delete"]);

        let activate = service.send(Event::ActivateItem(key("copy")));
        let select = service.send(Event::SelectItem(key("copy")));
        let focus = service.send(Event::FocusItem(key("copy")));
        let overflow = service.send(Event::OverflowChanged(1));
        let sync = service.set_props(props().disabled(false).dir(Direction::Rtl));

        assert!(activate.pending_effects.is_empty());
        assert!(!activate.context_changed);
        assert!(!select.context_changed);
        assert!(service.context().selected_items.is_empty());
        assert!(focus.state_changed);
        assert!(overflow.context_changed);
        assert!(sync.context_changed);
        assert!(!service.context().disabled);
        assert_eq!(service.context().dir, Direction::Rtl);
    }

    #[test]
    fn item_disabled_key_blocks_activation_without_group_disabled() {
        let mut service = service(props().disabled_items(key_set(&["copy"])));

        register(&mut service, &["copy"]);

        let result = service.send(Event::ActivateItem(key("copy")));

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn arrow_right_left_move_focus_between_registered_items() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let svc = service(props());

        let api = svc.connect(&send);

        api.on_item_keydown(&keyboard(KeyboardKey::ArrowRight));
        api.on_item_keydown(&keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(
            *events.lock().unwrap(),
            vec![Event::FocusNext, Event::FocusPrev]
        );

        let mut service = service(props());

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::FocusItem(key("copy"))));
        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item, Some(key("delete")));

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item, Some(key("copy")));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item, Some(key("delete")));

        let mut four_items = Service::new(props(), &Env::default(), &Messages::default());

        register(&mut four_items, &["copy", "delete", "archive", "share"]);

        drop(four_items.send(Event::FocusItem(key("archive"))));
        drop(four_items.send(Event::FocusPrev));

        assert_eq!(four_items.context().focused_item, Some(key("delete")));

        drop(four_items.send(Event::FocusItem(key("copy"))));
        drop(four_items.send(Event::FocusPrev));

        assert_eq!(four_items.context().focused_item, Some(key("share")));
    }

    #[test]
    fn home_end_move_focus_to_first_last_registered_items() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete", "archive"]);

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_item, Some(key("archive")));

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_item, Some(key("copy")));

        let events = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let api = service.connect(&send);

        api.on_item_keydown(&keyboard(KeyboardKey::Home));
        api.on_item_keydown(&keyboard(KeyboardKey::End));

        assert_eq!(
            *events.lock().unwrap(),
            vec![Event::FocusFirst, Event::FocusLast]
        );
    }

    #[test]
    fn rtl_swaps_horizontal_arrow_left_and_right() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let service = service(props().dir(Direction::Rtl));

        let api = service.connect(&send);

        api.on_item_keydown(&keyboard(KeyboardKey::ArrowRight));
        api.on_item_keydown(&keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(
            *events.lock().unwrap(),
            vec![Event::FocusPrev, Event::FocusNext]
        );
    }

    #[test]
    fn vertical_arrow_up_down_move_prev_next() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let service = service(props().orientation(Orientation::Vertical));

        let api = service.connect(&send);

        api.on_item_keydown(&keyboard(KeyboardKey::ArrowDown));
        api.on_item_keydown(&keyboard(KeyboardKey::ArrowUp));

        assert_eq!(
            *events.lock().unwrap(),
            vec![Event::FocusNext, Event::FocusPrev]
        );
    }

    #[test]
    fn overflow_changed_updates_overflow_and_visible_counts() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete", "archive"]);

        drop(service.send(Event::OverflowChanged(2)));

        assert_eq!(service.context().overflow_count, 2);
        assert_eq!(service.context().visible_count, 1);

        let api = service.connect(&|_| {});

        assert!(!api.is_overflowed(0));
        assert!(api.is_overflowed(1));
        assert!(api.is_overflowed(2));
    }

    #[test]
    fn overflow_count_saturates_visible_count() {
        let mut service = service(props());

        register(&mut service, &["copy"]);

        drop(service.send(Event::OverflowChanged(9)));

        assert_eq!(service.context().overflow_count, 9);
        assert_eq!(service.context().visible_count, 0);
        assert!(service.connect(&|_| {}).is_overflowed(0));
    }

    #[test]
    fn single_selection_toggles_selected_item() {
        let mut service = service(props().selection_mode(selection::Mode::Single));

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::SelectItem(key("copy"))));

        assert_eq!(service.context().selected_items, key_set(&["copy"]));

        drop(service.send(Event::SelectItem(key("delete"))));

        assert_eq!(service.context().selected_items, key_set(&["delete"]));

        drop(service.send(Event::SelectItem(key("delete"))));

        assert!(service.context().selected_items.is_empty());
    }

    #[test]
    fn multiple_selection_toggles_membership() {
        let mut service = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::SelectItem(key("copy"))));
        drop(service.send(Event::SelectItem(key("delete"))));

        assert_eq!(
            service.context().selected_items,
            key_set(&["copy", "delete"])
        );

        drop(service.send(Event::SelectItem(key("copy"))));

        assert_eq!(service.context().selected_items, key_set(&["delete"]));
    }

    #[test]
    fn selection_mode_none_omits_aria_pressed() {
        let mut service = service(props().selection_mode(selection::Mode::None));

        register(&mut service, &["copy"]);

        let attrs = service.connect(&|_| {}).item_attrs(&key("copy"));

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), None);
    }

    #[test]
    fn root_attrs_include_toolbar_accessibility() {
        let attrs = service(props()).connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("toolbar"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Document actions"),
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal"),
        );
    }

    #[test]
    fn root_labelledby_precedes_label_and_message_fallback() {
        let labelled = service(
            Props::new()
                .id("actions")
                .aria_label("Ignored")
                .aria_labelledby("actions-heading"),
        );

        let labelled_attrs = labelled.connect(&|_| {}).root_attrs();

        assert_eq!(
            labelled_attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("actions-heading"),
        );
        assert_eq!(labelled_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);

        let fallback_attrs = service(Props::new().id("actions"))
            .connect(&|_| {})
            .root_attrs();

        assert_eq!(
            fallback_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Actions"),
        );
    }

    #[test]
    fn item_attrs_emit_tabindex_state_and_disabled_attrs() {
        let mut service = service(
            props()
                .selection_mode(selection::Mode::Multiple)
                .disabled_items(key_set(&["delete"])),
        );

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::SelectItem(key("copy"))));

        let api = service.connect(&|_| {});
        let copy = api.item_attrs(&key("copy"));
        let delete = api.item_attrs(&key("delete"));

        assert_eq!(copy.get(&HtmlAttr::Data("ars-key")), Some("copy"));
        assert_eq!(copy.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(copy.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(copy.get(&HtmlAttr::Data("ars-state")), Some("selected"));
        assert_eq!(copy.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("true"),);

        assert_eq!(
            delete.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true"),
        );
        assert_eq!(delete.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
        assert_eq!(delete.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn overflow_trigger_attrs_include_menu_semantics() {
        let attrs = service(props()).connect(&|_| {}).overflow_trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")),
            Some("overflow-trigger"),
        );
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("More actions"),
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::HasPopup)), Some("menu"),);
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false"),
        );
    }

    #[test]
    fn messages_default_to_more_actions_and_actions() {
        let messages = Messages::default();

        let locale = Env::default().locale;

        assert_eq!((messages.overflow_trigger_label)(&locale), "More actions");
        assert_eq!((messages.toolbar_label)(&locale), "Actions");
    }

    #[test]
    fn props_defaults_match_spec() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert_eq!(props.orientation, Orientation::Horizontal);
        assert_eq!(props.dir, Direction::default());
        assert_eq!(props.overflow_mode, OverflowMode::Wrap);
        assert_eq!(props.variant, Variant::Toolbar);
        assert!(!props.disabled);
        assert!(props.disabled_items.is_empty());
        assert_eq!(props.selection_mode, selection::Mode::None);
        assert_eq!(props.max_visible_actions, None);
        assert_eq!(props.button_label_behavior, ButtonLabelBehavior::Show);
        assert_eq!(props.density, None);
        assert!(!props.justified);
        assert_eq!(props.aria_label, None);
        assert_eq!(props.aria_labelledby, None);
    }

    #[test]
    fn public_token_display_and_as_str_are_stable() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Focused { item: key("copy") }.to_string(), "focused");

        assert_eq!(ButtonLabelBehavior::Show.as_str(), "show");
        assert_eq!(ButtonLabelBehavior::Collapse.as_str(), "collapse");
        assert_eq!(ButtonLabelBehavior::Hide.as_str(), "hide");
        assert_eq!(ButtonLabelBehavior::Show.to_string(), "show");
        assert_eq!(ButtonLabelBehavior::Collapse.to_string(), "collapse");
        assert_eq!(ButtonLabelBehavior::Hide.to_string(), "hide");

        assert_eq!(Variant::Toolbar.as_str(), "toolbar");
        assert_eq!(Variant::Outlined.as_str(), "outlined");
        assert_eq!(Variant::Flat.as_str(), "flat");
        assert_eq!(Variant::Toolbar.to_string(), "toolbar");
        assert_eq!(Variant::Outlined.to_string(), "outlined");
        assert_eq!(Variant::Flat.to_string(), "flat");

        assert_eq!(OverflowMode::Wrap.as_str(), "wrap");
        assert_eq!(OverflowMode::Collapse.as_str(), "collapse");
        assert_eq!(OverflowMode::Menu.as_str(), "menu");
        assert_eq!(OverflowMode::Wrap.to_string(), "wrap");
        assert_eq!(OverflowMode::Collapse.to_string(), "collapse");
        assert_eq!(OverflowMode::Menu.to_string(), "menu");
    }

    #[test]
    fn props_builder_sets_all_fields_and_selection_callback() {
        let props = Props::new()
            .id("actions")
            .orientation(Orientation::Vertical)
            .dir(Direction::Rtl)
            .overflow_mode(OverflowMode::Menu)
            .variant(Variant::Flat)
            .disabled(true)
            .disabled_items(key_set(&["delete"]))
            .selection_mode(selection::Mode::Multiple)
            .max_visible_actions(Some(2))
            .button_label_behavior(ButtonLabelBehavior::Hide)
            .density("compact")
            .justified(true)
            .aria_label("Actions")
            .aria_labelledby("actions-heading")
            .on_action(callback(|_: Key| {}))
            .on_selection_change(callback(|_: BTreeSet<Key>| {}));

        assert_eq!(props.id, "actions");
        assert_eq!(props.orientation, Orientation::Vertical);
        assert_eq!(props.dir, Direction::Rtl);
        assert_eq!(props.overflow_mode, OverflowMode::Menu);
        assert_eq!(props.variant, Variant::Flat);
        assert!(props.disabled);
        assert_eq!(props.disabled_items, key_set(&["delete"]));
        assert_eq!(props.selection_mode, selection::Mode::Multiple);
        assert_eq!(props.max_visible_actions, Some(2));
        assert_eq!(props.button_label_behavior, ButtonLabelBehavior::Hide);
        assert_eq!(props.density.as_deref(), Some("compact"));
        assert!(props.justified);
        assert_eq!(props.aria_label.as_deref(), Some("Actions"));
        assert_eq!(props.aria_labelledby.as_deref(), Some("actions-heading"));
        assert!(props.on_action.is_some());
        assert!(props.on_selection_change.is_some());
    }

    #[test]
    fn register_item_is_idempotent() {
        let mut service = service(props());

        drop(service.send(Event::RegisterItem(key("copy"))));

        let duplicate = service.send(Event::RegisterItem(key("copy")));

        assert!(!duplicate.context_changed);
        assert_eq!(service.context().registered_items, vec![key("copy")]);
    }

    #[test]
    fn repeated_focus_and_idle_blur_are_noops() {
        let mut service = service(props());

        register(&mut service, &["copy"]);

        let idle_blur = service.send(Event::Blur);

        assert!(!idle_blur.state_changed);
        assert!(!idle_blur.context_changed);

        drop(service.send(Event::FocusItem(key("copy"))));

        let repeated_focus = service.send(Event::FocusItem(key("copy")));

        assert!(!repeated_focus.state_changed);
        assert!(!repeated_focus.context_changed);
        assert!(repeated_focus.pending_effects.is_empty());
        assert_eq!(service.context().focused_item, Some(key("copy")));
    }

    #[test]
    fn blur_clears_focused_item() {
        let mut service = service(props());

        register(&mut service, &["copy"]);

        drop(service.send(Event::FocusItem(key("copy"))));

        let result = service.send(Event::Blur);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
    }

    #[test]
    fn transition_repairs_state_context_focus_drift() {
        let (_, mut ctx) =
            <Machine as ars_core::Machine>::init(&props(), &Env::default(), &Messages::default());
        ctx.registered_items.push(key("copy"));

        let state = State::Focused { item: key("copy") };

        let focus_plan = <Machine as ars_core::Machine>::transition(
            &state,
            &Event::FocusItem(key("copy")),
            &ctx,
            &props(),
        )
        .expect("drifted focus should be repaired");

        assert_eq!(
            focus_plan.target,
            Some(State::Focused { item: key("copy") }),
        );
        assert_eq!(focus_plan.effects.len(), 1);
        assert_eq!(focus_plan.effects[0].name, Effect::FocusItem);

        let (_, mut ctx) =
            <Machine as ars_core::Machine>::init(&props(), &Env::default(), &Messages::default());

        ctx.focused_item = Some(key("copy"));

        let blur_plan =
            <Machine as ars_core::Machine>::transition(&State::Idle, &Event::Blur, &ctx, &props())
                .expect("drifted blur should be repaired");

        assert_eq!(blur_plan.target, Some(State::Idle));
    }

    #[test]
    fn transition_returns_none_when_drifted_focus_has_no_focusable_items() {
        let (_, mut ctx) =
            <Machine as ars_core::Machine>::init(&props(), &Env::default(), &Messages::default());

        ctx.registered_items = vec![key("copy"), key("delete")];
        ctx.focused_item = Some(key("copy"));

        let plan = <Machine as ars_core::Machine>::transition(
            &State::Focused { item: key("copy") },
            &Event::FocusNext,
            &ctx,
            &props().disabled_items(key_set(&["copy", "delete"])),
        );

        assert!(plan.is_none());
    }

    #[test]
    fn select_item_noops_for_disabled_items_and_none_mode() {
        let mut disabled = service(
            props()
                .selection_mode(selection::Mode::Multiple)
                .disabled_items(key_set(&["delete"])),
        );

        register(&mut disabled, &["delete"]);

        let disabled_result = disabled.send(Event::SelectItem(key("delete")));

        assert!(!disabled_result.context_changed);
        assert!(disabled.context().selected_items.is_empty());

        let mut none = service(props().selection_mode(selection::Mode::None));

        register(&mut none, &["copy"]);

        let none_result = none.send(Event::SelectItem(key("copy")));

        assert!(!none_result.context_changed);
        assert!(none.context().selected_items.is_empty());
    }

    #[test]
    fn overflow_changed_repeated_same_count_is_noop() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::OverflowChanged(1)));

        let repeated = service.send(Event::OverflowChanged(1));

        assert!(!repeated.context_changed);
        assert_eq!(service.context().overflow_count, 1);
        assert_eq!(service.context().visible_count, 1);
    }

    #[test]
    fn overflow_changed_recomputes_drifted_visible_count() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete", "archive"]);

        drop(service.send(Event::OverflowChanged(1)));

        service.context_mut().visible_count = 99;

        let result = service.send(Event::OverflowChanged(1));

        assert!(result.context_changed);
        assert_eq!(service.context().overflow_count, 1);
        assert_eq!(service.context().visible_count, 2);
    }

    #[test]
    fn unregister_missing_item_is_noop() {
        let mut service = service(props());

        register(&mut service, &["copy"]);

        let result = service.send(Event::UnregisterItem(key("delete")));

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.context().registered_items, vec![key("copy")]);
    }

    #[test]
    fn unregister_item_clears_removed_focus() {
        let mut service = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::FocusItem(key("delete"))));
        drop(service.send(Event::SelectItem(key("delete"))));

        let result = service.send(Event::UnregisterItem(key("delete")));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert_eq!(service.context().registered_items, vec![key("copy")]);
        assert!(service.context().selected_items.is_empty());
    }

    #[test]
    fn unregister_nonfocused_item_keeps_focused_state() {
        let mut service = service(props());

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::FocusItem(key("copy"))));

        let result = service.send(Event::UnregisterItem(key("delete")));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Focused { item: key("copy") },);
        assert_eq!(service.context().registered_items, vec![key("copy")]);
        assert_eq!(service.context().focused_item, Some(key("copy")));
    }

    #[test]
    fn on_props_changed_emits_for_each_context_backed_prop() {
        let old = props();

        let cases = [
            props().disabled(true),
            props().dir(Direction::Rtl),
            props().disabled_items(key_set(&["delete"])),
            props().selection_mode(selection::Mode::Multiple),
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                vec![Event::SetProps],
            );
        }
    }

    #[test]
    fn on_props_changed_ignores_render_only_props() {
        let old = props();
        let new = props()
            .orientation(Orientation::Vertical)
            .overflow_mode(OverflowMode::Menu)
            .variant(Variant::Outlined)
            .max_visible_actions(Some(1))
            .button_label_behavior(ButtonLabelBehavior::Collapse)
            .density("compact")
            .justified(true)
            .aria_label("Other")
            .aria_labelledby("other-label")
            .on_action(callback(|_: Key| {}))
            .on_selection_change(callback(|_: BTreeSet<Key>| {}));

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn set_props_prunes_disabled_selection_and_clears_disabled_focus() {
        let mut service = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::SelectItem(key("copy"))));
        drop(service.send(Event::SelectItem(key("delete"))));
        drop(service.send(Event::FocusItem(key("delete"))));

        let result = service.set_props(
            props()
                .selection_mode(selection::Mode::Multiple)
                .disabled_items(key_set(&["delete"])),
        );

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert_eq!(service.context().selected_items, key_set(&["copy"]));
    }

    #[test]
    fn set_props_normalizes_selection_mode_and_preserves_group_disabled_focus() {
        let mut service = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut service, &["copy", "delete"]);

        drop(service.send(Event::SelectItem(key("copy"))));
        drop(service.send(Event::SelectItem(key("delete"))));
        drop(service.send(Event::FocusItem(key("copy"))));

        drop(service.set_props(props().selection_mode(selection::Mode::Single)));

        assert_eq!(service.context().selected_items, key_set(&["copy"]));
        assert_eq!(service.context().focused_item, Some(key("copy")));

        drop(service.set_props(props().selection_mode(selection::Mode::None)));

        assert!(service.context().selected_items.is_empty());

        drop(service.set_props(props().disabled(true)));

        assert_eq!(service.state(), &State::Focused { item: key("copy") },);
        assert_eq!(service.context().focused_item, Some(key("copy")));

        let send = |_: Event| {};

        assert_eq!(
            service
                .connect(&send)
                .item_attrs(&key("copy"))
                .get(&HtmlAttr::TabIndex),
            Some("0"),
        );
    }

    #[test]
    fn keydown_dispatch_covers_activation_and_cross_axis_ignores() {
        let events = Arc::new(Mutex::new(Vec::new()));

        let mut svc = service(props());

        register(&mut svc, &["copy"]);

        drop(svc.send(Event::FocusItem(key("copy"))));

        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let api = svc.connect(&send);

        api.on_item_keydown(&keyboard(KeyboardKey::Enter));
        api.on_item_keydown(&keyboard(KeyboardKey::Space));
        api.on_item_keydown(&keyboard(KeyboardKey::ArrowUp));
        api.on_item_keydown(&keyboard(KeyboardKey::ArrowDown));

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                Event::ActivateItem(key("copy")),
                Event::ActivateItem(key("copy")),
            ],
        );

        let vertical_events = Arc::new(Mutex::new(Vec::new()));
        let vertical_send = {
            let vertical_events = Arc::clone(&vertical_events);
            move |event| vertical_events.lock().unwrap().push(event)
        };

        let vertical = service(props().orientation(Orientation::Vertical));

        let vertical_api = vertical.connect(&vertical_send);

        vertical_api.on_item_keydown(&keyboard(KeyboardKey::ArrowLeft));
        vertical_api.on_item_keydown(&keyboard(KeyboardKey::ArrowRight));
        vertical_api.on_item_keydown(&keyboard(KeyboardKey::ArrowDown));

        assert_eq!(*vertical_events.lock().unwrap(), vec![Event::FocusNext]);
    }

    #[test]
    fn api_event_helpers_dispatch_typed_events_and_respect_disabled_click() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let events = Arc::clone(&events);
            move |event| events.lock().unwrap().push(event)
        };

        let service = service(props().disabled_items(key_set(&["delete"])));

        let api = service.connect(&send);

        api.on_item_focus(&key("copy"));
        api.on_item_blur();
        api.on_item_click(&key("copy"));
        api.on_item_click(&key("delete"));
        api.on_item_mount(&key("copy"));
        api.on_item_unmount(&key("copy"));

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                Event::FocusItem(key("copy")),
                Event::Blur,
                Event::ActivateItem(key("copy")),
                Event::RegisterItem(key("copy")),
                Event::UnregisterItem(key("copy")),
            ],
        );
    }

    #[test]
    fn part_attrs_dispatches_every_part() {
        let mut service = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut service, &["copy"]);

        drop(service.send(Event::SelectItem(key("copy"))));

        let api = service.connect(&|_| {});

        assert!(format!("{api:?}").contains("action_group::Api"));
        assert_eq!(
            api.part_attrs(Part::Root).get(&HtmlAttr::Data("ars-part")),
            Some("root"),
        );
        assert_eq!(
            api.part_attrs(Part::Item {
                item_id: key("copy"),
            })
            .get(&HtmlAttr::Data("ars-state")),
            Some("selected"),
        );
        assert_eq!(
            api.part_attrs(Part::OverflowTrigger)
                .get(&HtmlAttr::Data("ars-part")),
            Some("overflow-trigger"),
        );
    }

    #[test]
    fn focus_prev_skips_disabled_items_and_wraps() {
        let mut service = service(props().disabled_items(key_set(&["delete", "archive"])));

        register(&mut service, &["copy", "delete", "archive", "share"]);

        drop(service.send(Event::FocusItem(key("copy"))));
        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item, Some(key("share")));

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item, Some(key("copy")));

        let mut wrapped_skip = Service::new(
            props().disabled_items(key_set(&["copy"])),
            &Env::default(),
            &Messages::default(),
        );

        register(&mut wrapped_skip, &["copy", "delete", "archive", "share"]);

        drop(wrapped_skip.send(Event::FocusItem(key("delete"))));
        drop(wrapped_skip.send(Event::FocusPrev));

        assert_eq!(wrapped_skip.context().focused_item, Some(key("share")));
    }

    #[test]
    fn selection_change_callback_fires_for_user_selection_changes() {
        let changes = Arc::new(Mutex::new(Vec::new()));

        let mut service = service(
            props()
                .selection_mode(selection::Mode::Multiple)
                .on_selection_change(callback({
                    let changes = Arc::clone(&changes);

                    move |selected: BTreeSet<Key>| {
                        changes.lock().unwrap().push(selected);
                    }
                })),
        );

        register(&mut service, &["copy", "delete"]);

        let select = service.send(Event::SelectItem(key("copy")));
        let add = service.send(Event::SelectItem(key("delete")));
        let remove = service.send(Event::SelectItem(key("copy")));

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in select
            .pending_effects
            .into_iter()
            .chain(add.pending_effects)
            .chain(remove.pending_effects)
        {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(
            *changes.lock().unwrap(),
            vec![
                key_set(&["copy"]),
                key_set(&["copy", "delete"]),
                key_set(&["delete"]),
            ],
        );
    }

    #[test]
    fn action_group_snapshots_cover_attr_output_branches() {
        let mut default = service(props());

        register(&mut default, &["copy", "delete"]);

        assert_snapshot!(
            "action_group_root_default",
            snapshot_attrs(&default.connect(&|_| {}).root_attrs()),
        );
        assert_snapshot!(
            "action_group_item_idle",
            snapshot_attrs(&default.connect(&|_| {}).item_attrs(&key("copy"))),
        );

        let vertical = service(props().orientation(Orientation::Vertical));

        assert_snapshot!(
            "action_group_root_vertical",
            snapshot_attrs(&vertical.connect(&|_| {}).root_attrs()),
        );

        let disabled = service(props().disabled(true));

        assert_snapshot!(
            "action_group_root_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).root_attrs()),
        );

        let labelledby = service(props().aria_labelledby("actions-heading"));

        assert_snapshot!(
            "action_group_root_labelledby_precedence",
            snapshot_attrs(&labelledby.connect(&|_| {}).root_attrs()),
        );

        let styled = service(
            props()
                .variant(Variant::Outlined)
                .overflow_mode(OverflowMode::Menu)
                .button_label_behavior(ButtonLabelBehavior::Collapse)
                .density("compact")
                .justified(true),
        );
        assert_snapshot!(
            "action_group_root_styling_hooks",
            snapshot_attrs(&styled.connect(&|_| {}).root_attrs()),
        );

        let mut focused = service(props());

        register(&mut focused, &["copy"]);

        drop(focused.send(Event::FocusItem(key("copy"))));

        assert_snapshot!(
            "action_group_item_focused",
            snapshot_attrs(&focused.connect(&|_| {}).item_attrs(&key("copy"))),
        );

        let mut item_disabled = service(props().disabled_items(key_set(&["delete"])));

        register(&mut item_disabled, &["delete"]);

        assert_snapshot!(
            "action_group_item_disabled",
            snapshot_attrs(&item_disabled.connect(&|_| {}).item_attrs(&key("delete"))),
        );

        let mut single = service(props().selection_mode(selection::Mode::Single));

        register(&mut single, &["copy"]);

        drop(single.send(Event::SelectItem(key("copy"))));

        assert_snapshot!(
            "action_group_item_selected_single",
            snapshot_attrs(&single.connect(&|_| {}).item_attrs(&key("copy"))),
        );

        let mut multiple = service(props().selection_mode(selection::Mode::Multiple));

        register(&mut multiple, &["copy"]);

        drop(multiple.send(Event::SelectItem(key("copy"))));

        assert_snapshot!(
            "action_group_item_selected_multiple",
            snapshot_attrs(&multiple.connect(&|_| {}).item_attrs(&key("copy"))),
        );

        assert_snapshot!(
            "action_group_overflow_trigger_default_label",
            snapshot_attrs(&default.connect(&|_| {}).overflow_trigger_attrs()),
        );

        let custom_messages = Messages {
            overflow_trigger_label: MessageFn::static_str("More document actions"),
            toolbar_label: MessageFn::static_str("Document tools"),
        };

        let custom = Service::<Machine>::new(props(), &Env::default(), &custom_messages);

        assert_snapshot!(
            "action_group_overflow_trigger_custom_label",
            snapshot_attrs(&custom.connect(&|_| {}).overflow_trigger_attrs()),
        );
    }
}
