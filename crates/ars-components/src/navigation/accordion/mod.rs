//! Accordion navigation component.
//!
//! Accordion owns the framework-agnostic disclosure-group state: open item
//! keys, item registration, disabled guards, roving focus intent, orientation
//! aware keyboard navigation, and ARIA/data attributes for the accordion
//! anatomy. The agnostic core never moves DOM focus itself; it emits a typed
//! [`Effect::FocusFocusedItem`] intent and adapters focus their own native
//! element handles.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    Direction, Env, HtmlAttr, KeyboardKey, Orientation, PendingEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

use super::key_token::dom_safe_key_token;

/// The only accordion machine state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// Runtime state is held in [`Context`].
    #[default]
    Idle,
}

/// Events accepted by the accordion state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Expand an item by key.
    ExpandItem(Key),

    /// Collapse an item by key.
    CollapseItem(Key),

    /// Toggle an item by key.
    ToggleItem(Key),

    /// Expand every registered enabled item. Ignored in single mode.
    ExpandAll,

    /// Collapse every open enabled item.
    CollapseAll,

    /// A trigger received focus.
    Focus(Key),

    /// Focus left the accordion trigger set.
    Blur,

    /// Move focus to the next enabled trigger.
    FocusNext,

    /// Move focus to the previous enabled trigger.
    FocusPrev,

    /// Move focus to the first enabled trigger.
    FocusFirst,

    /// Move focus to the last enabled trigger.
    FocusLast,

    /// Replace the registered item list in DOM order.
    SetItems(Vec<ItemRegistration>),

    /// Synchronize prop-backed context fields.
    SyncProps,

    /// Synchronize the controlled open-item set.
    SyncControlledValue(BTreeSet<Key>),
}

/// Typed effect intents emitted by the accordion machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter must move DOM focus to [`Context::focused_item`].
    FocusFocusedItem,
}

/// Adapter-supplied registration data for one rendered accordion item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemRegistration {
    /// Stable item key in DOM order.
    pub key: Key,

    /// Whether this item is disabled.
    pub disabled: bool,
}

impl ItemRegistration {
    /// Builds an enabled item registration.
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            disabled: false,
        }
    }

    /// Builds a disabled item registration.
    #[must_use]
    pub const fn disabled(key: Key) -> Self {
        Self {
            key,
            disabled: true,
        }
    }
}

/// Localized messages for [`Accordion`](self).
///
/// Accordion emits no built-in strings, so this is intentionally empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Immutable configuration for an accordion instance.
#[derive(Clone, Debug, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled set of open item keys.
    pub value: Option<BTreeSet<Key>>,

    /// Initial uncontrolled set of open item keys.
    pub default_value: BTreeSet<Key>,

    /// Whether multiple items may be open at once.
    pub multiple: bool,

    /// Whether the only open item in single mode may be closed.
    pub collapsible: bool,

    /// Whether every item trigger is disabled.
    pub disabled: bool,

    /// Visual stacking axis.
    pub orientation: Orientation,

    /// Text direction used for horizontal arrow-key resolution.
    pub dir: Direction,

    /// Adapter hint: defer first content mount until an item opens.
    pub lazy_mount: bool,

    /// Adapter hint: unmount content after close.
    pub unmount_on_exit: bool,

    /// Heading level adapters use around item triggers.
    pub heading_level: u8,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            multiple: false,
            collapsible: false,
            disabled: false,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            lazy_mount: false,
            unmount_on_exit: false,
            heading_level: 3,
        }
    }
}

impl Props {
    /// Returns default accordion props.
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

    /// Sets [`value`](Self::value), making the accordion controlled at mount.
    #[must_use]
    pub fn value(mut self, value: BTreeSet<Key>) -> Self {
        self.value = Some(value);
        self
    }

    /// Clears [`value`](Self::value), making the accordion uncontrolled at mount.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, value: BTreeSet<Key>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`multiple`](Self::multiple).
    #[must_use]
    pub const fn multiple(mut self, value: bool) -> Self {
        self.multiple = value;
        self
    }

    /// Sets [`collapsible`](Self::collapsible).
    #[must_use]
    pub const fn collapsible(mut self, value: bool) -> Self {
        self.collapsible = value;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Sets [`heading_level`](Self::heading_level).
    #[must_use]
    pub const fn heading_level(mut self, value: u8) -> Self {
        self.heading_level = value;
        self
    }
}

/// Runtime context for an accordion instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// Open item keys, controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,

    /// Item key whose trigger currently has focus.
    pub focused_item: Option<Key>,

    /// Whether multiple items may be open at once.
    pub multiple: bool,

    /// Whether single-mode open items may collapse.
    pub collapsible: bool,

    /// Whether every trigger is disabled.
    pub disabled: bool,

    /// Visual stacking axis.
    pub orientation: Orientation,

    /// Text direction used by horizontal arrow-key navigation.
    pub dir: Direction,

    /// Heading level adapters should use for item trigger wrappers.
    pub heading_level: u8,

    /// Registered item keys in DOM order.
    pub items: Vec<Key>,

    /// Disabled flags keyed by item key.
    pub disabled_items: BTreeMap<Key, bool>,

    /// Hydration-stable generated ids for accordion parts.
    pub ids: ComponentIds,
}

/// Anatomy parts exposed by the accordion connect API.
#[derive(ComponentPart)]
#[scope = "accordion"]
pub enum Part {
    /// The outer accordion root.
    Root,

    /// An item wrapper.
    Item {
        /// Item key.
        item_key: Key,
    },

    /// Heading wrapper around an item trigger.
    ItemHeader {
        /// Item key.
        item_key: Key,
    },

    /// Button that toggles an item.
    ItemTrigger {
        /// Item key.
        item_key: Key,
    },

    /// Visual indicator inside a trigger.
    ItemIndicator {
        /// Item key.
        item_key: Key,
    },

    /// Collapsible content panel.
    ItemContent {
        /// Item key.
        item_key: Key,
    },
}

/// State machine for the accordion component.
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
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let value = if let Some(value) = &props.value {
            Bindable::controlled(normalize_value_for_mode(value.clone(), props.multiple))
        } else {
            Bindable::uncontrolled(normalize_value_for_mode(
                props.default_value.clone(),
                props.multiple,
            ))
        };

        (
            State::Idle,
            Context {
                value,
                focused_item: None,
                multiple: props.multiple,
                collapsible: props.collapsible,
                disabled: props.disabled,
                orientation: props.orientation,
                dir: props.dir,
                heading_level: props.heading_level,
                items: Vec::new(),
                disabled_items: BTreeMap::new(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::ExpandItem(item) => expand_item_plan(ctx, item),

            Event::CollapseItem(item) => collapse_item_plan(ctx, item),

            Event::ToggleItem(item) => toggle_item_plan(ctx, item),

            Event::ExpandAll => expand_all_plan(ctx),

            Event::CollapseAll => collapse_all_plan(ctx),

            Event::Focus(item) => focus_item_plan(ctx, item),

            Event::Blur => {
                ctx.focused_item.as_ref()?;

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_item = None;
                }))
            }

            Event::FocusNext => {
                let current = focus_anchor(ctx)?;
                let next = step_focus(ctx, &current, FocusStep::Next)?;
                Some(focus_item_transition(next))
            }

            Event::FocusPrev => {
                let current = focus_anchor(ctx)?;
                let prev = step_focus(ctx, &current, FocusStep::Prev)?;
                Some(focus_item_transition(prev))
            }

            Event::FocusFirst => {
                let first = enabled_items(ctx).next()?;
                Some(focus_item_transition(first))
            }

            Event::FocusLast => {
                let last = enabled_items(ctx).next_back()?;
                Some(focus_item_transition(last))
            }

            Event::SetItems(items) => Some(set_items_plan(items)),

            Event::SyncProps => Some(sync_props_plan(ctx, props)),

            Event::SyncControlledValue(value) => Some(sync_controlled_value_plan(ctx, value)),
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();

        if old.multiple != new.multiple
            || old.collapsible != new.collapsible
            || old.disabled != new.disabled
            || old.orientation != new.orientation
            || old.dir != new.dir
            || old.heading_level != new.heading_level
        {
            events.push(Event::SyncProps);
        }

        if let (Some(old_value), Some(new_value)) = (&old.value, &new.value)
            && old_value != new_value
        {
            events.push(Event::SyncControlledValue(new_value.clone()));
        }

        events
    }
}

/// Connected API surface for an accordion instance.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
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
    /// Returns `true` when `item_key` is currently open.
    #[must_use]
    pub fn is_item_open(&self, item_key: &Key) -> bool {
        self.ctx.value.get().contains(item_key)
    }

    /// Returns `true` when `item_key` is globally or individually disabled.
    #[must_use]
    pub fn is_item_disabled(&self, item_key: &Key) -> bool {
        self.ctx.disabled || item_disabled(self.ctx, item_key)
    }

    /// Returns the key whose trigger currently has focus.
    #[must_use]
    pub const fn focused_item(&self) -> Option<&Key> {
        self.ctx.focused_item.as_ref()
    }

    /// Returns the clamped heading level for item trigger wrappers.
    #[must_use]
    pub fn heading_level(&self) -> u8 {
        self.ctx.heading_level.clamp(2, 6)
    }

    /// Returns the generated trigger id for `item_key`.
    #[must_use]
    pub fn trigger_id(&self, item_key: &Key) -> String {
        trigger_dom_id(&self.ctx.ids, item_key)
    }

    /// Returns the generated content id for `item_key`.
    #[must_use]
    pub fn content_id(&self, item_key: &Key) -> String {
        content_dom_id(&self.ctx.ids, item_key)
    }

    /// Attributes for the root container element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-orientation"),
                orientation_token(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for an item wrapper element.
    #[must_use]
    pub fn item_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            item_key: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                state_token(self.is_item_open(item_key)),
            );

        if self.is_item_disabled(item_key) {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for an item heading wrapper.
    #[must_use]
    pub fn item_header_attrs(&self, _item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemHeader {
            item_key: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for an item trigger button.
    #[must_use]
    pub fn item_trigger_attrs(&self, item_key: &Key, focus_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemTrigger {
            item_key: Key::default(),
        }
        .data_attrs();

        let is_open = self.is_item_open(item_key);
        let is_disabled = self.is_item_disabled(item_key);
        let is_focused = self.ctx.focused_item.as_ref() == Some(item_key);

        attrs
            .set(HtmlAttr::Id, self.trigger_id(item_key))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), state_token(is_open))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Expanded), bool_token(is_open))
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.content_id(item_key),
            );

        if is_disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        } else if is_open && !self.ctx.multiple && !self.ctx.collapsible {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if is_focused && focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for an item indicator element.
    #[must_use]
    pub fn item_indicator_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator {
            item_key: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                state_token(self.is_item_open(item_key)),
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for an item content region.
    #[must_use]
    pub fn item_content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemContent {
            item_key: Key::default(),
        }
        .data_attrs();

        let is_open = self.is_item_open(item_key);

        attrs
            .set(HtmlAttr::Id, self.content_id(item_key))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "region")
            .set(HtmlAttr::Data("ars-state"), state_token(is_open))
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.trigger_id(item_key),
            );

        if !is_open {
            attrs.set(HtmlAttr::Hidden, "until-found");
        }

        attrs
    }

    /// Adapter handler: an item trigger was clicked.
    pub fn on_item_trigger_click(&self, item_key: &Key) {
        if !self.is_item_disabled(item_key) {
            (self.send)(Event::ToggleItem(item_key.clone()));
        }
    }

    /// Adapter handler: an item trigger received focus.
    pub fn on_item_trigger_focus(&self, item_key: &Key) {
        if !self.is_item_disabled(item_key) {
            (self.send)(Event::Focus(item_key.clone()));
        }
    }

    /// Adapter handler: an item trigger lost focus.
    pub fn on_item_trigger_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Adapter handler: a key was pressed on an item trigger.
    pub fn on_item_trigger_keydown(&self, item_key: &Key, data: &KeyboardEventData) {
        let (prev, next) = arrow_pair(self.ctx.orientation, self.ctx.dir);

        if data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space {
            if !self.is_item_disabled(item_key) {
                (self.send)(Event::ToggleItem(item_key.clone()));
            }
        } else if data.key == next {
            (self.send)(Event::FocusNext);
        } else if data.key == prev {
            (self.send)(Event::FocusPrev);
        } else if data.key == KeyboardKey::Home {
            (self.send)(Event::FocusFirst);
        } else if data.key == KeyboardKey::End {
            (self.send)(Event::FocusLast);
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { item_key } => self.item_attrs(&item_key),
            Part::ItemHeader { item_key } => self.item_header_attrs(&item_key),
            Part::ItemTrigger { item_key } => self.item_trigger_attrs(&item_key, false),
            Part::ItemIndicator { item_key } => self.item_indicator_attrs(&item_key),
            Part::ItemContent { item_key } => self.item_content_attrs(&item_key),
        }
    }
}

#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

fn expand_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || item_disabled(ctx, item) || ctx.value.get().contains(item) {
        return None;
    }

    let item = item.clone();
    let multiple = ctx.multiple;

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut value = ctx.value.get().clone();

        if multiple {
            value.insert(item.clone());
        } else {
            value.clear();

            value.insert(item.clone());
        }

        ctx.value.set(value);
    }))
}

fn collapse_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || item_disabled(ctx, item) || !ctx.value.get().contains(item) {
        return None;
    }

    if !ctx.multiple && !ctx.collapsible && ctx.value.get().len() <= 1 {
        return None;
    }

    let item = item.clone();

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut value = ctx.value.get().clone();

        value.remove(&item);

        ctx.value.set(value);
    }))
}

fn toggle_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.value.get().contains(item) {
        collapse_item_plan(ctx, item)
    } else {
        expand_item_plan(ctx, item)
    }
}

fn expand_all_plan(ctx: &Context) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || !ctx.multiple {
        return None;
    }

    let mut next = ctx.value.get().clone();
    let mut changed = false;

    for item in enabled_items(ctx) {
        changed |= next.insert(item);
    }

    if !changed {
        return None;
    }

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(next);
    }))
}

fn collapse_all_plan(ctx: &Context) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled {
        return None;
    }

    if !ctx.multiple && !ctx.collapsible && !ctx.value.get().is_empty() {
        return None;
    }

    let next = ctx
        .value
        .get()
        .iter()
        .filter(|item| item_disabled(ctx, item))
        .cloned()
        .collect::<BTreeSet<_>>();

    if &next == ctx.value.get() {
        return None;
    }

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(next);
    }))
}

fn focus_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || !registered(ctx, item) || item_disabled(ctx, item) {
        return None;
    }

    if ctx.focused_item.as_ref() == Some(item) {
        return None;
    }

    let item = item.clone();
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_item = Some(item);
    }))
}

fn focus_item_transition(item: Key) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_item = Some(item);
    })
    .with_effect(PendingEffect::named(Effect::FocusFocusedItem))
}

fn set_items_plan(items: &[ItemRegistration]) -> TransitionPlan<Machine> {
    let items = items.to_vec();

    TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::with_capacity(items.len());
        let mut disabled = BTreeMap::new();

        for item in &items {
            if seen.insert(item.key.clone()) {
                ordered.push(item.key.clone());
                disabled.insert(item.key.clone(), item.disabled);
            }
        }

        ctx.items = ordered;
        ctx.disabled_items = disabled;

        if ctx
            .focused_item
            .as_ref()
            .is_some_and(|item| !registered(ctx, item) || item_disabled(ctx, item))
        {
            ctx.focused_item = None;
        }
    })
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let multiple = props.multiple;
    let collapsible = props.collapsible;
    let disabled = props.disabled;
    let orientation = props.orientation;
    let dir = props.dir;
    let heading_level = props.heading_level;
    let needs_value_normalize = ctx.multiple && !multiple;

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.multiple = multiple;
        ctx.collapsible = collapsible;
        ctx.disabled = disabled;
        ctx.orientation = orientation;
        ctx.dir = dir;
        ctx.heading_level = heading_level;

        if needs_value_normalize {
            let normalized = normalize_value_for_mode(ctx.value.get().clone(), false);

            ctx.value.set(normalized);
        }

        if ctx
            .focused_item
            .as_ref()
            .is_some_and(|item| ctx.disabled || item_disabled(ctx, item))
        {
            ctx.focused_item = None;
        }
    })
}

fn sync_controlled_value_plan(ctx: &Context, value: &BTreeSet<Key>) -> TransitionPlan<Machine> {
    let value = normalize_value_for_mode(value.clone(), ctx.multiple);

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.sync_controlled(Some(value));
    })
}

fn normalize_value_for_mode(mut value: BTreeSet<Key>, multiple: bool) -> BTreeSet<Key> {
    if multiple || value.len() <= 1 {
        return value;
    }

    let first = value.pop_first();
    let mut normalized = BTreeSet::new();

    if let Some(first) = first {
        normalized.insert(first);
    }

    normalized
}

fn enabled_items(ctx: &Context) -> impl DoubleEndedIterator<Item = Key> + '_ {
    ctx.items
        .iter()
        .filter(|item| !ctx.disabled && !item_disabled(ctx, item))
        .cloned()
}

fn focus_anchor(ctx: &Context) -> Option<Key> {
    ctx.focused_item
        .as_ref()
        .filter(|item| registered(ctx, item) && !item_disabled(ctx, item))
        .cloned()
        .or_else(|| enabled_items(ctx).next())
}

fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let enabled = enabled_items(ctx).collect::<Vec<_>>();
    let len = enabled.len();

    if len == 0 {
        return None;
    }

    let index = enabled.iter().position(|item| item == current).unwrap_or(0);

    let next_index = match step {
        FocusStep::Next => (index + 1) % len,

        FocusStep::Prev => {
            if index == 0 {
                len - 1
            } else {
                index - 1
            }
        }
    };

    let next = enabled.get(next_index)?.clone();

    if &next == current {
        return None;
    }

    Some(next)
}

fn registered(ctx: &Context, item: &Key) -> bool {
    ctx.items.iter().any(|registered| registered == item)
}

fn item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled_items.get(item).copied().unwrap_or(false)
}

const fn arrow_pair(orientation: Orientation, dir: Direction) -> (KeyboardKey, KeyboardKey) {
    match (orientation, dir) {
        (Orientation::Horizontal, Direction::Rtl) => {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        }

        (Orientation::Horizontal, Direction::Ltr | Direction::Auto) => {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        }

        (Orientation::Vertical, _) => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
    }
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn state_token(open: bool) -> &'static str {
    if open { "open" } else { "closed" }
}

fn trigger_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("trigger", &dom_safe_key_token(key))
}

fn content_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("content", &dom_safe_key_token(key))
}

#[cfg(test)]
fn snapshot_attrs(attrs: &AttrMap) -> String {
    use core::fmt::Write as _;

    let mut attrs = attrs.iter().collect::<Vec<_>>();

    attrs.sort_by_key(|(attr, _)| attr.to_string());

    let mut out = String::new();

    for (attr, value) in attrs {
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
mod tests {
    use alloc::{collections::BTreeSet, vec, vec::Vec};
    use core::cell::RefCell;

    use ars_collections::Key;
    use ars_core::{
        AriaAttr, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Machine as MachineTrait,
        Orientation, Service,
    };
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::{
        Effect, Event, ItemRegistration, Machine, Messages, Part, Props, content_dom_id,
        snapshot_attrs, trigger_dom_id,
    };

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn props() -> Props {
        Props::new().id("accordion")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages)
    }

    fn keys(values: &[&str]) -> BTreeSet<Key> {
        values.iter().map(|value| key(value)).collect()
    }

    fn keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: key.as_w3c_str().to_owned(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    #[test]
    fn props_builder_sets_every_field() {
        let props = Props::new()
            .id("faq")
            .value(keys(&["a"]))
            .default_value(keys(&["b"]))
            .multiple(true)
            .collapsible(true)
            .disabled(true)
            .orientation(Orientation::Horizontal)
            .dir(Direction::Rtl)
            .lazy_mount(true)
            .unmount_on_exit(true)
            .heading_level(7);

        assert_eq!(props.id, "faq");
        assert_eq!(props.value, Some(keys(&["a"])));
        assert_eq!(props.default_value, keys(&["b"]));
        assert!(props.multiple);
        assert!(props.collapsible);
        assert!(props.disabled);
        assert_eq!(props.orientation, Orientation::Horizontal);
        assert_eq!(props.dir, Direction::Rtl);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert_eq!(props.heading_level, 7);
    }

    #[test]
    fn uncontrolled_builder_only_clears_controlled_value() {
        let props = Props::new()
            .id("faq")
            .value(keys(&["a"]))
            .default_value(keys(&["b"]))
            .multiple(true)
            .collapsible(true)
            .disabled(true)
            .orientation(Orientation::Horizontal)
            .dir(Direction::Rtl)
            .lazy_mount(true)
            .unmount_on_exit(true)
            .heading_level(5)
            .uncontrolled();

        assert_eq!(props.id, "faq");
        assert_eq!(props.value, None);
        assert_eq!(props.default_value, keys(&["b"]));
        assert!(props.multiple);
        assert!(props.collapsible);
        assert!(props.disabled);
        assert_eq!(props.orientation, Orientation::Horizontal);
        assert_eq!(props.dir, Direction::Rtl);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert_eq!(props.heading_level, 5);
    }

    #[test]
    fn init_uses_controlled_value_when_present() {
        let service = service(
            props()
                .value(keys(&["controlled"]))
                .default_value(keys(&["default"])),
        );

        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), &keys(&["controlled"]));
    }

    #[test]
    fn single_mode_replaces_open_item() {
        let mut service = service(props().default_value(keys(&["a"])));

        drop(service.send(Event::ExpandItem(key("b"))));

        assert_eq!(service.context().value.get(), &keys(&["b"]));
    }

    #[test]
    fn multiple_mode_keeps_items_independent() {
        let mut service = service(props().multiple(true).default_value(keys(&["a"])));

        drop(service.send(Event::ExpandItem(key("b"))));

        assert_eq!(service.context().value.get(), &keys(&["a", "b"]));
    }

    #[test]
    fn collapsible_open_item_can_close() {
        let mut service = service(props().collapsible(true).default_value(keys(&["a"])));

        drop(service.send(Event::ToggleItem(key("a"))));

        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn non_collapsible_single_item_stays_open() {
        let mut service = service(props().default_value(keys(&["a"])));

        let result = service.send(Event::ToggleItem(key("a")));

        assert!(!result.context_changed);
        assert_eq!(service.context().value.get(), &keys(&["a"]));
    }

    #[test]
    fn disabled_root_blocks_mutation() {
        let mut service = service(props().disabled(true));

        let result = service.send(Event::ExpandItem(key("a")));

        assert!(!result.context_changed);
        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn disabled_root_blocks_expand_all_and_collapse_item() {
        let mut service = service(
            props()
                .multiple(true)
                .disabled(true)
                .default_value(keys(&["a"])),
        );

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::new(key("b")),
        ])));

        let expand_all = service.send(Event::ExpandAll);
        let collapse_item = service.send(Event::CollapseItem(key("a")));

        assert!(!expand_all.context_changed);
        assert!(!collapse_item.context_changed);
        assert_eq!(service.context().value.get(), &keys(&["a"]));
    }

    #[test]
    fn disabled_item_blocks_mutation() {
        let mut service = service(props().multiple(true));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
        ])));

        let result = service.send(Event::ExpandItem(key("b")));

        assert!(!result.context_changed);
        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn disabled_item_blocks_direct_collapse() {
        let mut service = service(props().multiple(true).default_value(keys(&["a", "b"])));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
        ])));

        let result = service.send(Event::CollapseItem(key("b")));

        assert!(!result.context_changed);
        assert_eq!(service.context().value.get(), &keys(&["a", "b"]));
    }

    #[test]
    fn set_items_registers_order_and_disabled_flags() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
            ItemRegistration::new(key("a")),
        ])));

        assert_eq!(service.context().items, vec![key("a"), key("b")]);
        assert_eq!(
            service.context().disabled_items.get(&key("a")),
            Some(&false)
        );
        assert_eq!(service.context().disabled_items.get(&key("b")), Some(&true));
    }

    #[test]
    fn expand_all_skips_disabled_items() {
        let mut service = service(props().multiple(true));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
            ItemRegistration::new(key("c")),
        ])));

        drop(service.send(Event::ExpandAll));

        assert_eq!(service.context().value.get(), &keys(&["a", "c"]));
    }

    #[test]
    fn expand_all_is_multiple_mode_only_and_noops_when_complete() {
        let mut single = service(props());

        drop(single.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::new(key("b")),
        ])));

        let single_result = single.send(Event::ExpandAll);

        assert!(!single_result.context_changed);
        assert!(single.context().value.get().is_empty());

        let mut complete = service(props().multiple(true).default_value(keys(&["a", "b"])));

        drop(complete.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::new(key("b")),
        ])));

        let complete_result = complete.send(Event::ExpandAll);

        assert!(!complete_result.context_changed);
        assert_eq!(complete.context().value.get(), &keys(&["a", "b"]));
    }

    #[test]
    fn collapse_all_keeps_open_disabled_items() {
        let mut service = service(props().multiple(true).default_value(keys(&["a", "b", "c"])));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
            ItemRegistration::new(key("c")),
        ])));

        drop(service.send(Event::CollapseAll));

        assert_eq!(service.context().value.get(), &keys(&["b"]));
    }

    #[test]
    fn collapse_all_respects_single_non_collapsible_guard_and_disabled_filter() {
        let mut single = service(props().default_value(keys(&["a"])));
        let single_result = single.send(Event::CollapseAll);

        assert!(!single_result.context_changed);
        assert_eq!(single.context().value.get(), &keys(&["a"]));

        let mut multiple = service(props().multiple(true).default_value(keys(&["a", "b"])));

        drop(multiple.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
        ])));

        drop(multiple.send(Event::CollapseAll));

        assert_eq!(multiple.context().value.get(), &keys(&["b"]));
    }

    #[test]
    fn keyboard_enter_and_space_toggle_item() {
        let events = RefCell::new(Vec::new());
        let service = service(props().collapsible(true));
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Enter));
        api.on_item_trigger_keydown(&key("b"), &keyboard(KeyboardKey::Space));

        assert_eq!(
            events.into_inner(),
            vec![Event::ToggleItem(key("a")), Event::ToggleItem(key("b"))]
        );
    }

    #[test]
    fn trigger_handlers_emit_events_and_respect_disabled_state() {
        let events = RefCell::new(Vec::new());
        let enabled_service = service(props());
        let send = |event| events.borrow_mut().push(event);

        let api = enabled_service.connect(&send);

        api.on_item_trigger_click(&key("a"));
        api.on_item_trigger_focus(&key("a"));
        api.on_item_trigger_blur();

        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::ToggleItem(key("a")),
                Event::Focus(key("a")),
                Event::Blur
            ]
        );

        events.borrow_mut().clear();

        let disabled = service(props().disabled(true));
        let disabled_api = disabled.connect(&send);

        disabled_api.on_item_trigger_click(&key("a"));
        disabled_api.on_item_trigger_focus(&key("a"));
        disabled_api.on_item_trigger_blur();

        assert_eq!(events.into_inner(), vec![Event::Blur]);
    }

    #[test]
    fn keydown_handler_maps_navigation_and_ignores_unhandled_keys() {
        let events = RefCell::new(Vec::new());
        let service = service(props().orientation(Orientation::Vertical));
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowDown));
        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowUp));
        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Home));
        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::End));
        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Escape));

        assert_eq!(
            events.into_inner(),
            vec![
                Event::FocusNext,
                Event::FocusPrev,
                Event::FocusFirst,
                Event::FocusLast,
            ]
        );
    }

    #[test]
    fn keydown_handler_only_end_dispatches_focus_last() {
        let events = RefCell::new(Vec::new());
        let service = service(props());
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Escape));

        assert!(events.borrow().is_empty());

        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::End));

        assert_eq!(events.into_inner(), vec![Event::FocusLast]);
    }

    #[test]
    fn vertical_arrow_navigation_wraps_and_skips_disabled() {
        let mut service = service(props().orientation(Orientation::Vertical));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
            ItemRegistration::new(key("c")),
        ])));
        drop(service.send(Event::Focus(key("a"))));

        let result = service.send(Event::FocusNext);

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("c")));
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::FocusFocusedItem)
        );
    }

    #[test]
    fn focus_navigation_wraps_backward_and_bootstraps_without_focus() {
        let mut service = service(props().orientation(Orientation::Vertical));

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
            ItemRegistration::new(key("c")),
        ])));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("c")));

        drop(service.send(Event::Blur));
        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("c")));
    }

    #[test]
    fn focus_prev_moves_to_immediate_previous_enabled_item() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::new(key("b")),
            ItemRegistration::new(key("c")),
            ItemRegistration::new(key("d")),
        ])));
        drop(service.send(Event::Focus(key("c"))));
        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("b")));
    }

    #[test]
    fn focus_navigation_ignores_stale_anchor_when_only_one_enabled_item_remains() {
        let mut unregistered = service(props());

        drop(unregistered.send(Event::SetItems(vec![ItemRegistration::new(key("a"))])));

        unregistered.context_mut().focused_item = Some(key("missing"));

        let unregistered_result = unregistered.send(Event::FocusNext);

        assert!(!unregistered_result.context_changed);
        assert_eq!(
            unregistered.context().focused_item.as_ref(),
            Some(&key("missing"))
        );

        let mut disabled = service(props());

        drop(disabled.send(Event::SetItems(vec![
            ItemRegistration::disabled(key("a")),
            ItemRegistration::new(key("b")),
        ])));

        disabled.context_mut().focused_item = Some(key("a"));

        let disabled_result = disabled.send(Event::FocusNext);

        assert!(!disabled_result.context_changed);
        assert_eq!(disabled.context().focused_item.as_ref(), Some(&key("a")));
    }

    #[test]
    fn horizontal_rtl_swaps_left_right_navigation() {
        let events = RefCell::new(Vec::new());
        let service = service(
            props()
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl),
        );
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowLeft));
        api.on_item_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowRight));

        assert_eq!(
            events.into_inner(),
            vec![Event::FocusNext, Event::FocusPrev]
        );
    }

    #[test]
    fn home_end_focus_first_last_enabled_item() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::disabled(key("a")),
            ItemRegistration::new(key("b")),
            ItemRegistration::new(key("c")),
        ])));

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("b")));

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("c")));
    }

    #[test]
    fn focus_navigation_emits_focus_effect() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::new(key("b")),
        ])));
        drop(service.send(Event::Focus(key("a"))));

        let result = service.send(Event::FocusNext);

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("b")));
        assert_eq!(
            result
                .pending_effects
                .iter()
                .map(|effect| effect.name)
                .collect::<Vec<_>>(),
            vec![Effect::FocusFocusedItem],
        );
    }

    #[test]
    fn focus_item_requires_registered_enabled_item() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
        ])));

        let unregistered = service.send(Event::Focus(key("missing")));
        let disabled = service.send(Event::Focus(key("b")));

        drop(service.send(Event::Focus(key("a"))));

        let repeated = service.send(Event::Focus(key("a")));

        assert!(!unregistered.context_changed);
        assert!(!disabled.context_changed);
        assert!(!repeated.context_changed);
        assert_eq!(service.context().focused_item.as_ref(), Some(&key("a")));
        assert_eq!(service.connect(&|_| {}).focused_item(), Some(&key("a")));
    }

    #[test]
    fn disabled_root_clears_focus_and_blocks_focus_navigation() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![ItemRegistration::new(key("a"))])));
        drop(service.send(Event::Focus(key("a"))));

        let result = service.set_props(props().disabled(true));

        assert!(result.context_changed);
        assert_eq!(service.context().focused_item, None);
        assert!(!service.send(Event::FocusFirst).context_changed);
    }

    #[test]
    fn set_items_clears_focus_when_item_is_removed_or_disabled() {
        let mut removed = service(props());

        drop(removed.send(Event::SetItems(vec![ItemRegistration::new(key("a"))])));
        drop(removed.send(Event::Focus(key("a"))));
        drop(removed.send(Event::SetItems(vec![ItemRegistration::new(key("b"))])));

        assert_eq!(removed.context().focused_item, None);

        let mut disabled = service(props());

        drop(disabled.send(Event::SetItems(vec![ItemRegistration::new(key("a"))])));
        drop(disabled.send(Event::Focus(key("a"))));
        drop(disabled.send(Event::SetItems(vec![ItemRegistration::disabled(key("a"))])));

        assert_eq!(disabled.context().focused_item, None);
    }

    #[test]
    fn sync_props_updates_context_and_normalizes_single_mode() {
        let mut service = service(props().multiple(true).default_value(keys(&["a", "b"])));

        drop(service.send(Event::SetItems(vec![ItemRegistration::new(key("a"))])));
        drop(service.send(Event::Focus(key("a"))));

        let result = service.set_props(
            props()
                .collapsible(true)
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl)
                .heading_level(9),
        );

        assert!(result.context_changed);
        assert!(!service.context().multiple);
        assert!(service.context().collapsible);
        assert_eq!(service.context().orientation, Orientation::Horizontal);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().heading_level, 9);
        assert_eq!(service.context().value.get().len(), 1);
        assert_eq!(service.connect(&|_| {}).heading_level(), 6);
    }

    #[test]
    fn sync_props_keeps_multiple_values_when_multiple_remains_enabled() {
        let mut service = service(props().multiple(true).default_value(keys(&["a", "b"])));

        let result = service.set_props(props().multiple(true).orientation(Orientation::Horizontal));

        assert!(result.context_changed);
        assert!(service.context().multiple);
        assert_eq!(service.context().orientation, Orientation::Horizontal);
        assert_eq!(service.context().value.get(), &keys(&["a", "b"]));
    }

    #[test]
    fn sync_controlled_value_updates_value_and_normalizes_single_mode() {
        let mut service = service(props().value(keys(&["a"])));

        let result = service.set_props(props().value(keys(&["b", "c"])));

        assert!(result.context_changed);
        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get().len(), 1);
        assert!(service.context().value.get().contains(&key("b")));
    }

    #[test]
    fn heading_level_clamps_to_two_through_six() {
        assert_eq!(
            service(props().heading_level(1))
                .connect(&|_| {})
                .heading_level(),
            2
        );
        assert_eq!(
            service(props().heading_level(4))
                .connect(&|_| {})
                .heading_level(),
            4
        );
        assert_eq!(
            service(props().heading_level(9))
                .connect(&|_| {})
                .heading_level(),
            6
        );
    }

    #[test]
    fn part_attrs_matches_direct_attr_methods() {
        let open_service = service(props().default_value(keys(&["a"])));

        let api = open_service.connect(&|_| {});

        let item = key("a");

        let parts = [
            (Part::Root, api.root_attrs()),
            (
                Part::Item {
                    item_key: item.clone(),
                },
                api.item_attrs(&item),
            ),
            (
                Part::ItemHeader {
                    item_key: item.clone(),
                },
                api.item_header_attrs(&item),
            ),
            (
                Part::ItemTrigger {
                    item_key: item.clone(),
                },
                api.item_trigger_attrs(&item, false),
            ),
            (
                Part::ItemIndicator {
                    item_key: item.clone(),
                },
                api.item_indicator_attrs(&item),
            ),
            (
                Part::ItemContent {
                    item_key: item.clone(),
                },
                api.item_content_attrs(&item),
            ),
        ];

        for (part, expected) in parts {
            assert_eq!(api.part_attrs(part), expected);
        }
    }

    #[test]
    fn root_attrs_do_not_emit_selection_container_aria() {
        let service = service(props().multiple(true));
        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::MultiSelectable)), None);
    }

    #[test]
    fn non_collapsible_open_trigger_is_aria_disabled_but_focusable() {
        let service = service(props().default_value(keys(&["a"])));
        let attrs = service
            .connect(&|_| {})
            .item_trigger_attrs(&key("a"), false);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Disabled), None);
    }

    #[test]
    fn dom_safe_ids_are_stable_for_string_and_int_keys() {
        let service = service(props().id("faq"));

        let api = service.connect(&|_| {});

        assert_eq!(api.trigger_id(&Key::Int(42)), "faq-trigger-i-42");
        assert_eq!(api.content_id(&Key::Int(42)), "faq-content-i-42");
        assert_eq!(api.trigger_id(&key("a b")), "faq-trigger-s-612062");
        assert_eq!(api.content_id(&key("a b")), "faq-content-s-612062");
        assert_eq!(
            trigger_dom_id(&service.context().ids, &key("a b")),
            api.trigger_id(&key("a b"))
        );
        assert_eq!(
            content_dom_id(&service.context().ids, &key("a b")),
            api.content_id(&key("a b"))
        );
    }

    #[test]
    fn root_snapshots() {
        assert_snapshot!(
            "accordion_root_default",
            snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "accordion_root_multiple",
            snapshot_attrs(
                &service(props().multiple(true).dir(Direction::Rtl))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
        assert_snapshot!(
            "accordion_root_disabled",
            snapshot_attrs(
                &service(props().disabled(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn item_snapshots() {
        let open_service = service(props().default_value(keys(&["a"])));

        let api = open_service.connect(&|_| {});

        assert_snapshot!(
            "accordion_item_open",
            snapshot_attrs(&api.item_attrs(&key("a")))
        );
        assert_snapshot!(
            "accordion_item_closed",
            snapshot_attrs(&api.item_attrs(&key("b")))
        );

        let mut disabled = service(props());

        drop(disabled.send(Event::SetItems(vec![ItemRegistration::disabled(key("b"))])));

        assert_snapshot!(
            "accordion_item_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).item_attrs(&key("b")))
        );
    }

    #[test]
    fn item_header_snapshot() {
        assert_snapshot!(
            "accordion_item_header",
            snapshot_attrs(
                &service(props())
                    .connect(&|_| {})
                    .item_header_attrs(&key("a"))
            )
        );
    }

    #[test]
    fn trigger_snapshots() {
        let mut focused = service(props().default_value(keys(&["a"])));

        drop(focused.send(Event::SetItems(vec![
            ItemRegistration::new(key("a")),
            ItemRegistration::disabled(key("b")),
        ])));
        drop(focused.send(Event::Focus(key("a"))));

        let api = focused.connect(&|_| {});

        assert_snapshot!(
            "accordion_trigger_open",
            snapshot_attrs(&api.item_trigger_attrs(&key("a"), false))
        );
        assert_snapshot!(
            "accordion_trigger_disabled",
            snapshot_attrs(&api.item_trigger_attrs(&key("b"), false))
        );
        assert_snapshot!(
            "accordion_trigger_focus_visible",
            snapshot_attrs(&api.item_trigger_attrs(&key("a"), true))
        );

        let closed_service = service(props());
        let closed = closed_service.connect(&|_| {});

        assert_snapshot!(
            "accordion_trigger_closed",
            snapshot_attrs(&closed.item_trigger_attrs(&key("c"), false))
        );
    }

    #[test]
    fn indicator_snapshots() {
        let service = service(props().default_value(keys(&["a"])));

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "accordion_indicator_open",
            snapshot_attrs(&api.item_indicator_attrs(&key("a")))
        );
        assert_snapshot!(
            "accordion_indicator_closed",
            snapshot_attrs(&api.item_indicator_attrs(&key("b")))
        );
    }

    #[test]
    fn content_snapshots() {
        let service = service(props().default_value(keys(&["a"])));

        let api = service.connect(&|_| {});

        assert_snapshot!(
            "accordion_content_open",
            snapshot_attrs(&api.item_content_attrs(&key("a")))
        );
        assert_snapshot!(
            "accordion_content_closed",
            snapshot_attrs(&api.item_content_attrs(&key("b")))
        );
    }

    #[test]
    fn on_props_changed_emits_expected_sync_events() {
        let old = props().value(keys(&["a"]));
        let new = props()
            .value(keys(&["b"]))
            .multiple(true)
            .collapsible(true)
            .disabled(true)
            .orientation(Orientation::Horizontal)
            .dir(Direction::Rtl)
            .heading_level(4);

        assert_eq!(
            <Machine as MachineTrait>::on_props_changed(&old, &new),
            vec![Event::SyncProps, Event::SyncControlledValue(keys(&["b"]))]
        );
    }

    #[test]
    fn on_props_changed_detects_each_context_field_independently() {
        let old = props();

        let cases = [
            props().multiple(true),
            props().collapsible(true),
            props().disabled(true),
            props().orientation(Orientation::Horizontal),
            props().dir(Direction::Rtl),
            props().heading_level(4),
        ];

        assert!(<Machine as MachineTrait>::on_props_changed(&old, &old).is_empty());

        for new in cases {
            assert_eq!(
                <Machine as MachineTrait>::on_props_changed(&old, &new),
                vec![Event::SyncProps]
            );
        }
    }

    #[test]
    fn on_props_changed_detects_controlled_value_independently() {
        let old = props().value(keys(&["a"]));
        let new = props().value(keys(&["b"]));

        assert_eq!(
            <Machine as MachineTrait>::on_props_changed(&old, &new),
            vec![Event::SyncControlledValue(keys(&["b"]))]
        );

        assert!(<Machine as MachineTrait>::on_props_changed(&old, &old).is_empty());
    }
}
