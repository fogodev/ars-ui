//! SegmentGroup component state machine and connect API.

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Orientation, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The state of the `SegmentGroup` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No segment is focused.
    #[default]
    Idle,

    /// A segment has keyboard or pointer focus.
    Focused {
        /// The value of the focused segment.
        item: Key,
    },
}

/// The events accepted by the `SegmentGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Select a segment by value.
    SelectValue(Key),

    /// Focus moved to a specific segment.
    FocusItem {
        /// The value of the focused segment.
        item: Key,

        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus left the group.
    Blur,

    /// Move focus to the next enabled segment.
    FocusNext,

    /// Move focus to the previous enabled segment.
    FocusPrev,

    /// Focus the first enabled segment.
    FocusFirst,

    /// Focus the last enabled segment.
    FocusLast,

    /// Register a mounted segment value in logical DOM order.
    RegisterItem(Key),

    /// Unregister a mounted segment value.
    UnregisterItem(Key),

    /// Synchronize controlled value props.
    SetValue(Option<Key>),

    /// Synchronize context-backed props.
    SetProps,

    /// Restore the selected value to [`Props::default_value`].
    Reset,
}

/// Definition of a single segment within the group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// The value this segment represents.
    pub value: Key,

    /// Whether this individual segment is disabled.
    pub disabled: bool,
}

impl Segment {
    /// Creates an enabled segment for the provided value.
    #[must_use]
    pub fn new(value: impl Into<Key>) -> Self {
        Self {
            value: value.into(),
            disabled: false,
        }
    }

    /// Marks this segment as disabled or enabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Props for the `SegmentGroup` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled selected value. When `Some`, the component is controlled.
    pub value: Option<Key>,

    /// Default selected value for uncontrolled mode.
    pub default_value: Option<Key>,

    /// Whether the entire group is disabled.
    pub disabled: bool,

    /// Whether the group is read-only.
    pub readonly: bool,

    /// Whether the segment group is in an invalid state.
    pub invalid: bool,

    /// Native form field name for hidden input submission.
    pub name: Option<String>,

    /// ID of the associated native form element.
    pub form: Option<String>,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Whether arrow-key focus wraps at the ends.
    pub loop_focus: bool,

    /// Ordered segment definitions used for attributes and fallback navigation.
    pub items: Vec<Segment>,

    /// Called when user intent requests a new selected value.
    pub on_value_change: Option<Callback<dyn Fn(Option<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            disabled: false,
            readonly: false,
            invalid: false,
            name: None,
            form: None,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            items: Vec::new(),
            on_value_change: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), switching the group to controlled mode.
    #[must_use]
    pub fn value(mut self, value: impl Into<Key>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Clears [`value`](Self::value), switching the group to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value) for uncontrolled mode.
    #[must_use]
    pub fn default_value(mut self, value: impl Into<Key>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Clears [`default_value`](Self::default_value).
    #[must_use]
    pub fn no_default_value(mut self) -> Self {
        self.default_value = None;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets [`name`](Self::name), the hidden input form field name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Clears [`name`](Self::name).
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets [`form`](Self::form), the associated form element ID.
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Clears [`form`](Self::form).
    #[must_use]
    pub fn no_form(mut self) -> Self {
        self.form = None;
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

    /// Sets [`loop_focus`](Self::loop_focus).
    #[must_use]
    pub const fn loop_focus(mut self, loop_focus: bool) -> Self {
        self.loop_focus = loop_focus;
        self
    }

    /// Sets [`items`](Self::items), the ordered segment definitions.
    #[must_use]
    pub fn items(mut self, items: impl Into<Vec<Segment>>) -> Self {
        self.items = items.into();
        self
    }

    /// Sets [`on_value_change`](Self::on_value_change).
    #[must_use]
    pub fn on_value_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(Option<Key>) + Send + Sync>>,
    ) -> Self {
        self.on_value_change = Some(callback.into());
        self
    }

    /// Clears [`on_value_change`](Self::on_value_change).
    #[must_use]
    pub fn no_value_change(mut self) -> Self {
        self.on_value_change = None;
        self
    }
}

/// The context for the `SegmentGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled or uncontrolled selected value.
    pub value: Bindable<Option<Key>>,

    /// Currently focused segment value.
    pub focused_item: Option<Key>,

    /// Whether focus should render as keyboard-visible focus.
    pub focus_visible: bool,

    /// Whether the entire group is disabled.
    pub disabled: bool,

    /// Whether the selected value is read-only.
    pub readonly: bool,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Whether arrow-key focus wraps at the ends.
    pub loop_focus: bool,

    /// Ordered segment definitions from props.
    pub items: Vec<Segment>,

    /// Mounted segment values in logical DOM order.
    pub registered_items: Vec<Key>,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the segment group machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_value_change`] with the requested value.
    ValueChange,

    /// Adapter moves DOM focus to the item keyed by [`Context::focused_item`].
    FocusItem,
}

/// Machine for the `SegmentGroup` component.
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
        (
            State::Idle,
            Context {
                value: if let Some(value) = &props.value {
                    Bindable::controlled(Some(value.clone()))
                } else {
                    Bindable::uncontrolled(props.default_value.clone())
                },
                focused_item: None,
                focus_visible: false,
                disabled: props.disabled,
                readonly: props.readonly,
                orientation: props.orientation,
                dir: props.dir,
                loop_focus: props.loop_focus,
                items: props.items.clone(),
                registered_items: Vec::new(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if (ctx.disabled || ctx.readonly) && matches!(event, Event::SelectValue(_)) {
            return None;
        }

        match (state, event) {
            (_, Event::SelectValue(value)) => {
                if !item_is_present(ctx, value)
                    || is_item_disabled(ctx, value)
                    || ctx.value.get().as_ref() == Some(value)
                {
                    return None;
                }

                value_change_plan(ctx, Some(value.clone()))
            }

            (_, Event::FocusItem { item, is_keyboard }) => {
                if !can_focus_item(ctx, item) {
                    return None;
                }

                let item = item.clone();

                let is_keyboard = *is_keyboard;

                if matches!(state, State::Focused { item: current } if current == &item)
                    && ctx.focused_item.as_ref() == Some(&item)
                    && ctx.focus_visible == is_keyboard
                {
                    return None;
                }

                Some(focus_plan(item, is_keyboard, false))
            }

            (_, Event::Blur) => {
                if matches!(state, State::Idle) && ctx.focused_item.is_none() && !ctx.focus_visible
                {
                    return None;
                }

                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_item = None;
                    ctx.focus_visible = false;
                }))
            }

            (State::Idle, Event::FocusNext) => {
                idle_focus_seed(ctx, FocusStep::Next).map(|target| focus_plan(target, true, true))
            }

            (State::Idle, Event::FocusPrev) => {
                idle_focus_seed(ctx, FocusStep::Prev).map(|target| focus_plan(target, true, true))
            }

            (State::Focused { item }, Event::FocusNext) => {
                step_focus(ctx, item, FocusStep::Next).map(|target| focus_plan(target, true, true))
            }

            (State::Focused { item }, Event::FocusPrev) => {
                step_focus(ctx, item, FocusStep::Prev).map(|target| focus_plan(target, true, true))
            }

            (_, Event::FocusFirst) => {
                first_enabled(ctx).map(|target| focus_plan(target, true, true))
            }

            (_, Event::FocusLast) => last_enabled(ctx).map(|target| focus_plan(target, true, true)),

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

                let target = if focused_removed {
                    State::Idle
                } else {
                    state.clone()
                };

                let plan = if focused_removed {
                    TransitionPlan::to(target)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    ctx.registered_items
                        .retain(|registered| registered != &item);

                    if focused_removed {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }
                }))
            }

            (_, Event::SetValue(value)) => {
                let sanitized_value = value
                    .clone()
                    .filter(|value| controlled_value_is_selectable(ctx, value));

                let internal = sanitized_value.clone().or_else(|| ctx.value.get().clone());

                if ctx.value.is_controlled() == value.is_some()
                    && ctx.value.get() == &sanitized_value
                {
                    return Some(TransitionPlan::new());
                }

                let controlled = value.as_ref().map(|_| sanitized_value.clone());

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(internal);
                    ctx.value.sync_controlled(controlled);
                }))
            }

            (_, Event::SetProps) => Some(sync_props_plan(state, ctx, props)),

            (_, Event::Reset) => value_change_plan(ctx, props.default_value.clone()),
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "segment_group::Props.id must remain stable after init",
        );

        let mut events = Vec::new();

        if old.disabled != new.disabled
            || old.readonly != new.readonly
            || old.orientation != new.orientation
            || old.dir != new.dir
            || old.loop_focus != new.loop_focus
            || old.items != new.items
        {
            events.push(Event::SetProps);
        }

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        events
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

/// DOM parts of the `SegmentGroup` component.
#[derive(ComponentPart)]
#[scope = "segment-group"]
pub enum Part {
    /// The root radiogroup container.
    Root,

    /// A segment item.
    Item {
        /// Stable segment value.
        value: Key,
    },

    /// Text content within a segment item.
    ItemText {
        /// Stable segment value.
        value: Key,
    },

    /// Decorative animated selection indicator.
    Indicator,

    /// Hidden input used for native form submission.
    HiddenInput,
}

/// The API for the `SegmentGroup` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("segment_group::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns whether the item with the given value is selected.
    #[must_use]
    pub fn is_selected(&self, item_value: &Key) -> bool {
        self.ctx.value.get().as_ref() == Some(item_value)
    }

    /// Returns whether the whole group is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns whether a specific item is disabled by group or item state.
    #[must_use]
    pub fn is_item_disabled(&self, item_value: &Key) -> bool {
        is_item_disabled(self.ctx, item_value)
    }

    /// Returns the currently focused item value, if any.
    #[must_use]
    pub const fn focused_item(&self) -> Option<&Key> {
        self.ctx.focused_item.as_ref()
    }

    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        let orientation = orientation_token(self.ctx.orientation);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "radiogroup")
            .set(HtmlAttr::Aria(AriaAttr::Orientation), orientation)
            .set(HtmlAttr::Data("ars-orientation"), orientation);

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.props.invalid {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Invalid), "true")
                .set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Attributes for a single segment item.
    #[must_use]
    pub fn item_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            value: Key::default(),
        }
        .data_attrs();

        let selected = self.is_selected(item_value);
        let focused = self.ctx.focused_item.as_ref() == Some(item_value);
        let disabled = self.is_item_disabled(item_value);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-value"), item_value.to_string())
            .set(HtmlAttr::Data("ars-state"), checked_state_token(selected))
            .set(HtmlAttr::Id, self.ctx.ids.item("item", item_value))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Role, "radio")
            .set(HtmlAttr::Aria(AriaAttr::Checked), bool_token(selected))
            .set(HtmlAttr::TabIndex, self.item_tabindex(item_value));

        if selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        if focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        if focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the item text content.
    #[must_use]
    pub fn item_text_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemText {
            value: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                checked_state_token(self.is_selected(item_value)),
            );

        attrs
    }

    /// Attributes for the animated selection indicator.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(selected) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Data("ars-active-value"), selected.to_string());
        }

        attrs
    }

    /// Attributes for the hidden input element used for form submission.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(value) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, value.to_string());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Dispatches keyboard navigation from the root element.
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        self.dispatch_navigation_key(data);
    }

    /// Dispatches a click/press activation for an item.
    pub fn on_item_click(&self, item_value: &Key) {
        if !self.is_item_disabled(item_value) {
            (self.send)(Event::SelectValue(item_value.clone()));
        }
    }

    /// Dispatches a focus event for an item.
    pub fn on_item_focus(&self, item_value: &Key, is_keyboard: bool) {
        (self.send)(Event::FocusItem {
            item: item_value.clone(),
            is_keyboard,
        });
    }

    /// Dispatches a group blur event.
    pub fn on_item_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches keyboard activation for an item.
    pub fn on_item_keydown(&self, item_value: &Key, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Space | KeyboardKey::Enter if !data.repeat => {
                (self.send)(Event::SelectValue(item_value.clone()));
            }

            _ => self.dispatch_navigation_key(data),
        }
    }

    /// Dispatches an item mount registration event.
    pub fn on_item_mount(&self, item_value: &Key) {
        (self.send)(Event::RegisterItem(item_value.clone()));
    }

    /// Dispatches an item unmount registration event.
    pub fn on_item_unmount(&self, item_value: &Key) {
        (self.send)(Event::UnregisterItem(item_value.clone()));
    }

    /// Dispatches a native form reset event.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
    }

    fn item_tabindex(&self, item_value: &Key) -> &'static str {
        if self.is_item_disabled(item_value) {
            return "-1";
        }

        if self.is_roving_anchor(item_value) {
            "0"
        } else {
            "-1"
        }
    }

    fn is_roving_anchor(&self, item_value: &Key) -> bool {
        match self.state {
            State::Focused { item } => item == item_value,

            State::Idle => {
                if let Some(selected) = self.ctx.value.get()
                    && !is_item_disabled(self.ctx, selected)
                {
                    return selected == item_value;
                }

                first_enabled(self.ctx).as_ref() == Some(item_value)
            }
        }
    }

    fn dispatch_navigation_key(&self, data: &KeyboardEventData) {
        let horizontal = self.ctx.orientation == Orientation::Horizontal;
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
            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref value } => self.item_attrs(value),
            Part::ItemText { ref value } => self.item_text_attrs(value),
            Part::Indicator => self.indicator_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Focused { .. } => f.write_str("focused"),
        }
    }
}

const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn checked_state_token(selected: bool) -> &'static str {
    if selected { "checked" } else { "unchecked" }
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn item_definition<'a>(ctx: &'a Context, item: &Key) -> Option<&'a Segment> {
    ctx.items.iter().find(|segment| &segment.value == item)
}

fn is_item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled || item_definition(ctx, item).is_some_and(|segment| segment.disabled)
}

fn item_is_present(ctx: &Context, item: &Key) -> bool {
    ordered_items(ctx)
        .iter()
        .any(|registered| registered == item)
}

fn controlled_value_is_selectable(ctx: &Context, item: &Key) -> bool {
    item_definition(ctx, item).is_some_and(|segment| !segment.disabled)
}

fn can_focus_item(ctx: &Context, item: &Key) -> bool {
    ordered_items(ctx)
        .iter()
        .any(|registered| registered == item && !is_item_disabled(ctx, registered))
}

fn ordered_items(ctx: &Context) -> Vec<Key> {
    if ctx.registered_items.is_empty() {
        ctx.items
            .iter()
            .map(|segment| segment.value.clone())
            .collect()
    } else {
        ctx.registered_items.clone()
    }
}

fn first_enabled(ctx: &Context) -> Option<Key> {
    ordered_items(ctx)
        .into_iter()
        .find(|item| !is_item_disabled(ctx, item))
}

fn last_enabled(ctx: &Context) -> Option<Key> {
    ordered_items(ctx)
        .into_iter()
        .rev()
        .find(|item| !is_item_disabled(ctx, item))
}

fn idle_focus_seed(ctx: &Context, step: FocusStep) -> Option<Key> {
    if let Some(selected) = ctx
        .value
        .get()
        .as_ref()
        .filter(|selected| !is_item_disabled(ctx, selected))
        .cloned()
    {
        return Some(selected);
    }

    match step {
        FocusStep::Next => first_enabled(ctx),
        FocusStep::Prev => last_enabled(ctx),
    }
}

fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let enabled = ordered_items(ctx)
        .into_iter()
        .filter(|item| !is_item_disabled(ctx, item))
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        return None;
    }

    let current_index = enabled.iter().position(|item| item == current)?;

    match step {
        FocusStep::Next if current_index + 1 < enabled.len() => {
            Some(enabled[current_index + 1].clone())
        }

        FocusStep::Next if ctx.loop_focus => Some(enabled[0].clone()),

        FocusStep::Prev if current_index > 0 => Some(enabled[current_index - 1].clone()),

        FocusStep::Prev if ctx.loop_focus => Some(enabled[enabled.len() - 1].clone()),

        _ => None,
    }
}

fn focus_plan(target: Key, focus_visible: bool, emit_effect: bool) -> TransitionPlan<Machine> {
    let plan = TransitionPlan::to(State::Focused {
        item: target.clone(),
    })
    .apply(move |ctx: &mut Context| {
        ctx.focused_item = Some(target);
        ctx.focus_visible = focus_visible;
    });

    if emit_effect {
        plan.with_effect(PendingEffect::named(Effect::FocusItem))
    } else {
        plan
    }
}

fn value_change_plan(ctx: &Context, next: Option<Key>) -> Option<TransitionPlan<Machine>> {
    if ctx.value.get() == &next {
        return None;
    }

    let effect = value_change_effect(next.clone());

    if ctx.value.is_controlled() {
        return Some(TransitionPlan::new().with_effect(effect));
    }

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.value.set(next);
        })
        .with_effect(effect),
    )
}

fn value_change_effect(next: Option<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ValueChange, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_value_change {
            callback(next.clone());
        }

        no_cleanup()
    })
}

fn sync_props_plan(state: &State, ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let disabled = props.disabled;
    let readonly = props.readonly;
    let orientation = props.orientation;
    let dir = props.dir;
    let loop_focus = props.loop_focus;
    let items = props.items.clone();

    let focused_will_be_disabled = ctx.focused_item.as_ref().is_some_and(|focused| {
        disabled
            || !items
                .iter()
                .any(|item| &item.value == focused && !item.disabled)
    });

    let target_idle = matches!(state, State::Focused { .. }) && focused_will_be_disabled;

    let apply = move |ctx: &mut Context| {
        ctx.disabled = disabled;
        ctx.readonly = readonly;
        ctx.orientation = orientation;
        ctx.dir = dir;
        ctx.loop_focus = loop_focus;
        ctx.items = items;

        if focused_will_be_disabled {
            ctx.focused_item = None;
            ctx.focus_visible = false;
        }
    };

    if target_idle {
        TransitionPlan::to(State::Idle).apply(apply)
    } else {
        TransitionPlan::context_only(apply)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, sync::Arc, vec};
    use std::sync::Mutex;

    use ars_collections::Key;
    use ars_core::{
        AriaAttr, ConnectApi, Env, HtmlAttr, KeyboardKey, Service, StrongSend, callback,
    };
    use ars_i18n::{Direction, Orientation};
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn segment(value: &str) -> Segment {
        Segment::new(key(value))
    }

    fn disabled_segment(value: &str) -> Segment {
        Segment::new(key(value)).disabled(true)
    }

    fn props() -> Props {
        Props::new()
            .id("view-mode")
            .items(vec![segment("grid"), segment("list"), segment("table")])
            .default_value(key("grid"))
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

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn segment_group_root_and_item_attrs_emit_radio_contract() {
        let service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        let root = api.root_attrs();

        assert_eq!(root.get(&HtmlAttr::Role), Some("radiogroup"));
        assert_eq!(
            root.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );

        let selected = api.item_attrs(&key("grid"));

        assert_eq!(selected.get(&HtmlAttr::Role), Some("radio"));
        assert_eq!(
            selected.get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
        assert_eq!(selected.get(&HtmlAttr::TabIndex), Some("0"));

        let unselected = api.item_attrs(&key("list"));

        assert_eq!(unselected.get(&HtmlAttr::Role), Some("radio"));
        assert_eq!(
            unselected.get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("false")
        );
        assert_eq!(unselected.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn segment_group_enforces_single_selection_and_skips_disabled_items() {
        let changes = Arc::new(Mutex::new(Vec::<Option<Key>>::new()));
        let observed = Arc::clone(&changes);

        let mut service = Service::<Machine>::new(
            Props::new()
                .id("view-mode")
                .items(vec![
                    segment("grid"),
                    segment("list"),
                    disabled_segment("table"),
                ])
                .default_value(key("grid"))
                .on_value_change(callback(move |value| {
                    observed.lock().unwrap().push(value);
                })),
            &Env::default(),
            &Messages,
        );

        let mut result = service.send(Event::SelectValue(key("list")));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("list")));
        assert_eq!(result.pending_effects.len(), 1);

        let effect = result.pending_effects.pop().expect("value-change effect");
        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        drop(service.send(Event::SelectValue(key("table"))));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("list")));
        assert_eq!(
            changes.lock().unwrap().clone(),
            vec![Some(key("list"))],
            "disabled item selection must not emit a value change"
        );
    }

    #[test]
    fn segment_group_controlled_mode_emits_change_without_mutating_value() {
        let changes = Arc::new(Mutex::new(Vec::<Option<Key>>::new()));
        let observed = Arc::clone(&changes);

        let mut service = Service::<Machine>::new(
            Props::new()
                .id("view-mode")
                .items(vec![segment("grid"), segment("list")])
                .value(key("grid"))
                .on_value_change(callback(move |value| {
                    observed.lock().unwrap().push(value);
                })),
            &Env::default(),
            &Messages,
        );

        let mut result = service.send(Event::SelectValue(key("list")));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("grid")));
        assert_eq!(result.pending_effects.len(), 1);

        let effect = result.pending_effects.pop().expect("value-change effect");
        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().clone(), vec![Some(key("list"))]);
    }

    #[test]
    fn segment_group_disabled_and_readonly_block_selection() {
        for props in [props().disabled(true), props().readonly(true)] {
            let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

            let result = service.send(Event::SelectValue(key("list")));

            assert_eq!(service.context().value.get().as_ref(), Some(&key("grid")));
            assert!(result.pending_effects.is_empty());
        }
    }

    #[test]
    fn segment_group_rejects_absent_selection_values() {
        let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        let result = service.send(Event::SelectValue(key("missing")));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("grid")));
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn segment_group_roving_tabindex_prefers_focused_then_selected_then_first_enabled() {
        let mut service = Service::<Machine>::new(
            Props::new().id("view-mode").items(vec![
                disabled_segment("grid"),
                segment("list"),
                segment("table"),
            ]),
            &Env::default(),
            &Messages,
        );

        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(&key("list"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        drop(service.send(Event::SelectValue(key("table"))));

        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(&key("table"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        drop(service.send(Event::FocusItem {
            item: key("list"),
            is_keyboard: true,
        }));

        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(&key("list"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn segment_group_set_props_clears_focus_when_item_is_removed() {
        let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        drop(service.send(Event::FocusItem {
            item: key("table"),
            is_keyboard: true,
        }));

        drop(service.set_props(props().items(vec![segment("grid"), segment("list")])));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(&key("grid"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn segment_group_registration_controls_navigation_order_and_unmount_clears_focus() {
        let mut service =
            Service::<Machine>::new(props().loop_focus(false), &Env::default(), &Messages);

        drop(service.send(Event::RegisterItem(key("table"))));
        drop(service.send(Event::RegisterItem(key("grid"))));
        drop(service.send(Event::RegisterItem(key("table"))));

        assert_eq!(
            service.context().registered_items,
            vec![key("table"), key("grid")]
        );

        let result = service.send(Event::FocusFirst);

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("table")));
        assert_eq!(result.pending_effects.len(), 1);

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("grid")));

        drop(service.send(Event::UnregisterItem(key("grid"))));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert!(!service.context().focus_visible);

        let unchanged = service.send(Event::UnregisterItem(key("list")));

        assert!(unchanged.pending_effects.is_empty());
    }

    #[test]
    fn segment_group_edge_branches_preserve_expected_noops() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("view-mode")
                .items(vec![segment("grid"), segment("list")])
                .loop_focus(true),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("grid")));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("list")));

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("grid")));

        drop(service.send(Event::RegisterItem(key("grid"))));
        drop(service.send(Event::RegisterItem(key("list"))));
        drop(service.send(Event::UnregisterItem(key("list"))));

        assert_eq!(service.state(), &State::Focused { item: key("grid") });
        assert_eq!(service.context().registered_items, vec![key("grid")]);

        drop(service.send(Event::SelectValue(key("grid"))));

        let unchanged = service.send(Event::SelectValue(key("grid")));

        assert!(unchanged.pending_effects.is_empty());

        let unchanged = service.set_props(
            Props::new()
                .id("view-mode")
                .items(vec![segment("grid"), segment("list")])
                .loop_focus(false),
        );

        assert!(unchanged.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Focused { item: key("grid") });
    }

    #[test]
    fn segment_group_keyboard_navigation_respects_orientation_rtl_and_disabled_items() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("view-mode")
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl)
                .items(vec![
                    segment("grid"),
                    disabled_segment("list"),
                    segment("table"),
                ])
                .default_value(key("grid")),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::FocusItem {
            item: key("grid"),
            is_keyboard: true,
        }));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::ArrowLeft));

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("table")));

        let mut vertical = Service::<Machine>::new(
            Props::new()
                .id("view-mode-vertical")
                .orientation(Orientation::Vertical)
                .loop_focus(false)
                .items(vec![segment("grid"), segment("list")]),
            &Env::default(),
            &Messages,
        );

        drop(vertical.send(Event::FocusItem {
            item: key("grid"),
            is_keyboard: true,
        }));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        vertical
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::ArrowUp));

        for event in events.lock().unwrap().drain(..) {
            drop(vertical.send(event));
        }

        assert_eq!(vertical.context().focused_item.as_ref(), Some(&key("grid")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        vertical
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::ArrowDown));

        for event in events.lock().unwrap().drain(..) {
            drop(vertical.send(event));
        }

        assert_eq!(vertical.context().focused_item.as_ref(), Some(&key("list")));
    }

    #[test]
    fn segment_group_keyboard_navigation_covers_ltr_home_end_and_idle_prev() {
        let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(events.lock().unwrap().as_slice(), &[Event::FocusPrev]);

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("grid")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::End));

        assert_eq!(events.lock().unwrap().as_slice(), &[Event::FocusLast]);

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("table")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::Home));

        assert_eq!(events.lock().unwrap().as_slice(), &[Event::FocusFirst]);

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("grid")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_root_keydown(&keyboard(KeyboardKey::Escape));

        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn segment_group_item_keydown_selects_with_space_or_enter_only() {
        let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_item_keydown(&key("list"), &keyboard(KeyboardKey::Space));

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().value.get().as_ref(), Some(&key("list")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_item_keydown(&key("table"), &keyboard(KeyboardKey::Enter));

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().value.get().as_ref(), Some(&key("table")));

        let events = Arc::new(Mutex::new(Vec::<Event>::new()));
        let observed = Arc::clone(&events);

        service
            .connect(&move |event| observed.lock().unwrap().push(event))
            .on_item_keydown(&key("grid"), &keyboard(KeyboardKey::ArrowRight));

        for event in events.lock().unwrap().drain(..) {
            drop(service.send(event));
        }

        assert_eq!(service.context().value.get().as_ref(), Some(&key("table")));
    }

    #[test]
    fn segment_group_item_event_helpers_dispatch_expected_events() {
        let service = Service::<Machine>::new(props(), &Env::default(), &Messages);
        let events = Arc::new(Mutex::new(Vec::<Event>::new()));

        let observed = Arc::clone(&events);

        let send = move |event| observed.lock().unwrap().push(event);

        let api = service.connect(&send);

        api.on_item_click(&key("list"));
        api.on_item_focus(&key("list"), false);
        api.on_item_blur();
        api.on_item_mount(&key("list"));
        api.on_item_unmount(&key("list"));
        api.on_form_reset();

        assert_eq!(
            events.lock().unwrap().as_slice(),
            &[
                Event::SelectValue(key("list")),
                Event::FocusItem {
                    item: key("list"),
                    is_keyboard: false,
                },
                Event::Blur,
                Event::RegisterItem(key("list")),
                Event::UnregisterItem(key("list")),
                Event::Reset,
            ]
        );
    }

    #[test]
    fn segment_group_focus_and_blur_state_are_idempotent() {
        let mut service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        drop(service.send(Event::FocusItem {
            item: key("list"),
            is_keyboard: true,
        }));

        assert_eq!(service.context().focused_item.as_ref(), Some(&key("list")));
        assert_eq!(service.connect(&|_| {}).focused_item(), Some(&key("list")));

        let repeated = service.send(Event::FocusItem {
            item: key("list"),
            is_keyboard: true,
        });

        assert!(repeated.pending_effects.is_empty());

        let disabled = service.send(Event::FocusItem {
            item: key("missing"),
            is_keyboard: true,
        });

        assert!(disabled.pending_effects.is_empty());

        drop(service.send(Event::Blur));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);

        let repeated_blur = service.send(Event::Blur);

        assert!(repeated_blur.pending_effects.is_empty());
    }

    #[test]
    fn segment_group_set_props_syncs_controlled_value_and_context_fields() {
        let old = props().value(key("grid"));
        let new = props()
            .value(key("table"))
            .orientation(Orientation::Vertical)
            .dir(Direction::Rtl)
            .loop_focus(false)
            .items(vec![
                segment("grid"),
                disabled_segment("list"),
                segment("table"),
            ]);

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            [Event::SetProps, Event::SetValue(Some(key("table")))]
        );

        let mut service = Service::<Machine>::new(old, &Env::default(), &Messages);

        drop(service.send(Event::FocusItem {
            item: key("list"),
            is_keyboard: true,
        }));

        let result = service.set_props(new);

        assert_eq!(service.context().value.get().as_ref(), Some(&key("table")));
        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert!(!service.context().loop_focus);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn segment_group_set_props_selects_item_introduced_by_same_prop_sync() {
        let old = props().items(vec![segment("grid")]).value(key("grid"));
        let new = props()
            .items(vec![segment("grid"), segment("table")])
            .value(key("table"));

        let mut service = Service::<Machine>::new(old, &Env::default(), &Messages);

        drop(service.set_props(new));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("table")));
    }

    #[test]
    fn segment_group_set_value_accepts_enabled_controlled_value_when_group_disabled() {
        let mut service = Service::<Machine>::new(
            props()
                .disabled(true)
                .items(vec![segment("grid"), segment("list")])
                .value(key("grid")),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetValue(Some(key("list")))));

        assert_eq!(service.context().value.get().as_ref(), Some(&key("list")));
        assert!(service.context().value.is_controlled());
    }

    #[test]
    fn segment_group_set_value_rejects_disabled_controlled_value() {
        let mut service = Service::<Machine>::new(
            props()
                .items(vec![disabled_segment("grid"), segment("list")])
                .value(key("list")),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SetValue(Some(key("grid")))));

        assert_eq!(service.context().value.get(), &None);
        assert!(service.context().value.is_controlled());

        drop(service.send(Event::SetValue(None)));

        assert!(!service.context().value.is_controlled());
    }

    #[test]
    fn segment_group_reset_emits_default_value_change() {
        let changes = Arc::new(Mutex::new(Vec::<Option<Key>>::new()));
        let observed = Arc::clone(&changes);

        let mut service = Service::<Machine>::new(
            props()
                .default_value(key("grid"))
                .on_value_change(callback(move |value| {
                    observed.lock().unwrap().push(value);
                })),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::SelectValue(key("table"))));

        let mut result = service.send(Event::Reset);

        assert_eq!(service.context().value.get().as_ref(), Some(&key("grid")));
        assert_eq!(result.pending_effects.len(), 1);

        let effect = result
            .pending_effects
            .pop()
            .expect("reset value-change effect");

        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().clone(), vec![Some(key("grid"))]);
    }

    #[test]
    fn segment_group_builder_clearers_and_debug_accessors_round_trip() {
        let cleared_props = Props::new()
            .value(key("grid"))
            .uncontrolled()
            .default_value(key("list"))
            .no_default_value()
            .name("view")
            .no_name()
            .form("settings")
            .no_form()
            .on_value_change(callback(|_| {}))
            .no_value_change();

        assert_eq!(cleared_props.value, None);
        assert_eq!(cleared_props.default_value, None);
        assert_eq!(cleared_props.name, None);
        assert_eq!(cleared_props.form, None);
        assert!(cleared_props.on_value_change.is_none());

        let disabled = Service::<Machine>::new(props().disabled(true), &Env::default(), &Messages);

        let api = disabled.connect(&|_| {});

        assert!(api.is_disabled());
        assert!(format!("{api:?}").contains("segment_group::Api"));
    }

    #[test]
    fn segment_group_hidden_input_attrs_emit_form_submission_contract() {
        let service = Service::<Machine>::new(
            props().name("view").form("settings-form"),
            &Env::default(),
            &Messages,
        );

        let attrs = service.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("view"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("grid"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("settings-form"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn segment_group_hidden_input_is_disabled_with_group_not_readonly() {
        let disabled = Service::<Machine>::new(
            props().name("view").disabled(true),
            &Env::default(),
            &Messages,
        );
        let readonly = Service::<Machine>::new(
            props().name("view").readonly(true),
            &Env::default(),
            &Messages,
        );

        assert_eq!(
            disabled
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Disabled),
            Some("true")
        );
        assert!(
            !readonly
                .connect(&|_| {})
                .hidden_input_attrs()
                .contains(&HtmlAttr::Disabled)
        );
    }

    #[test]
    fn segment_group_part_attrs_delegate_for_all_parts() {
        let service = Service::<Machine>::new(props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(
            api.part_attrs(Part::Item { value: key("grid") }),
            api.item_attrs(&key("grid"))
        );
        assert_eq!(
            api.part_attrs(Part::ItemText { value: key("grid") }),
            api.item_text_attrs(&key("grid"))
        );
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
        assert_eq!(api.part_attrs(Part::HiddenInput), api.hidden_input_attrs());
    }

    #[test]
    fn segment_group_snapshots_cover_parts_and_output_branches() {
        let root = Service::<Machine>::new(
            props()
                .orientation(Orientation::Vertical)
                .disabled(true)
                .invalid(true)
                .readonly(true),
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "segment_group_root_vertical_disabled_invalid_readonly",
            snapshot_attrs(&root.connect(&|_| {}).root_attrs())
        );

        let selected = Service::<Machine>::new(props(), &Env::default(), &Messages);

        assert_snapshot!(
            "segment_group_item_selected",
            snapshot_attrs(&selected.connect(&|_| {}).item_attrs(&key("grid")))
        );
        assert_snapshot!(
            "segment_group_item_unselected",
            snapshot_attrs(&selected.connect(&|_| {}).item_attrs(&key("list")))
        );
        assert_snapshot!(
            "segment_group_item_text_selected",
            snapshot_attrs(&selected.connect(&|_| {}).item_text_attrs(&key("grid")))
        );
        assert_snapshot!(
            "segment_group_indicator_selected",
            snapshot_attrs(&selected.connect(&|_| {}).indicator_attrs())
        );
        assert_snapshot!(
            "segment_group_hidden_input_selected",
            snapshot_attrs(
                &Service::<Machine>::new(
                    props().name("view").form("settings-form"),
                    &Env::default(),
                    &Messages,
                )
                .connect(&|_| {})
                .hidden_input_attrs(),
            )
        );

        let mut focused = Service::<Machine>::new(
            Props::new()
                .id("view-mode")
                .items(vec![disabled_segment("grid"), segment("list")]),
            &Env::default(),
            &Messages,
        );

        drop(focused.send(Event::FocusItem {
            item: key("list"),
            is_keyboard: true,
        }));

        assert_snapshot!(
            "segment_group_item_disabled",
            snapshot_attrs(&focused.connect(&|_| {}).item_attrs(&key("grid")))
        );
        assert_snapshot!(
            "segment_group_item_focused_keyboard",
            snapshot_attrs(&focused.connect(&|_| {}).item_attrs(&key("list")))
        );

        let empty = Service::<Machine>::new(
            Props::new().id("view-mode").items(vec![segment("grid")]),
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "segment_group_indicator_empty",
            snapshot_attrs(&empty.connect(&|_| {}).indicator_attrs())
        );
        assert_snapshot!(
            "segment_group_hidden_input_empty",
            snapshot_attrs(&empty.connect(&|_| {}).hidden_input_attrs())
        );
    }

    #[test]
    fn segment_group_state_and_part_values_are_debuggable() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Focused { item: key("grid") }.to_string(), "focused");
        assert!(Part::Item { value: key("grid") }.name().contains("item"));
    }
}
