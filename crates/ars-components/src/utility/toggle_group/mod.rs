//! ToggleGroup component state machine and connect API.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug, Display};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentMessages, ComponentPart, ConnectApi, Direction,
    Env, HtmlAttr, KeyboardKey, Locale, MessageFn, Orientation, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The state of the `ToggleGroup` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No item is focused within the group.
    #[default]
    Idle,

    /// An item within the group has focus.
    Focused {
        /// The value of the item that has focus.
        item: Key,
    },
}

/// Events accepted by the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Select an item by value.
    SelectItem(Key),

    /// Deselect an item by value.
    DeselectItem(Key),

    /// Toggle an item's selected state by value.
    ToggleItem(Key),

    /// Focus received on an item by value.
    Focus {
        /// The value of the item that has focus.
        item: Key,

        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus left the group.
    Blur,

    /// Move focus to the next registered enabled item.
    FocusNext,

    /// Move focus to the previous registered enabled item.
    FocusPrev,

    /// Move focus to the first registered enabled item.
    FocusFirst,

    /// Move focus to the last registered enabled item.
    FocusLast,

    /// Register a rendered item value in logical DOM order.
    RegisterItem(Key),

    /// Unregister a rendered item value.
    UnregisterItem(Key),

    /// Restore the selected value to [`Props::default_value`].
    Reset,

    /// Synchronize controlled value props.
    SetValue(Option<BTreeSet<Key>>),

    /// Synchronize context-backed props.
    SetProps,
}

/// The selection mode for the `ToggleGroup` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// No items can be selected; the group is toolbar-only.
    None,

    /// Zero or one item may be selected at a time.
    #[default]
    Single,

    /// Any number of items may be selected.
    Multiple,
}

/// Props for the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled selected item set.
    pub value: Option<BTreeSet<Key>>,

    /// Default selected item set for uncontrolled mode.
    pub default_value: BTreeSet<Key>,

    /// Selection behavior for item activation.
    pub selection_mode: SelectionMode,

    /// Whether the whole group is disabled.
    pub disabled: bool,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Whether arrow navigation wraps at the ends.
    pub loop_focus: bool,

    /// Whether items use the roving tabindex pattern.
    pub roving_focus: bool,

    /// Accessible label for the group root.
    pub aria_label: Option<String>,

    /// ID of the element that labels the group root.
    pub aria_labelledby: Option<String>,

    /// Whether the last selected item cannot be deselected.
    pub disallow_empty_selection: bool,

    /// Native form field name for hidden input submission.
    pub name: Option<String>,

    /// Whether the field is currently invalid.
    pub invalid: bool,

    /// Whether a selection is required.
    pub required: bool,

    /// ID of an associated form element.
    pub form: Option<String>,

    /// Whether user selection changes are blocked while values still submit.
    pub read_only: bool,

    /// Set of item values disabled independently from the group.
    pub disabled_items: BTreeSet<Key>,

    /// Callback invoked when user intent requests a new selected set.
    pub on_change: Option<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            selection_mode: SelectionMode::Single,
            disabled: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            roving_focus: true,
            aria_label: None,
            aria_labelledby: None,
            disallow_empty_selection: false,
            name: None,
            invalid: false,
            required: false,
            form: None,
            read_only: false,
            disabled_items: BTreeSet::new(),
            on_change: None,
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

    /// Sets [`value`](Self::value), switching the group to controlled mode.
    #[must_use]
    pub fn value(mut self, value: BTreeSet<Key>) -> Self {
        self.value = Some(value);
        self
    }

    /// Clears [`value`](Self::value), switching the group to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, default_value: BTreeSet<Key>) -> Self {
        self.default_value = default_value;
        self
    }

    /// Sets [`selection_mode`](Self::selection_mode).
    #[must_use]
    pub const fn selection_mode(mut self, selection_mode: SelectionMode) -> Self {
        self.selection_mode = selection_mode;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

    /// Sets [`roving_focus`](Self::roving_focus).
    #[must_use]
    pub const fn roving_focus(mut self, roving_focus: bool) -> Self {
        self.roving_focus = roving_focus;
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

    /// Sets [`disallow_empty_selection`](Self::disallow_empty_selection).
    #[must_use]
    pub const fn disallow_empty_selection(mut self, disallow: bool) -> Self {
        self.disallow_empty_selection = disallow;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }

    /// Sets [`read_only`](Self::read_only).
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Sets [`disabled_items`](Self::disabled_items).
    #[must_use]
    pub fn disabled_items(mut self, disabled_items: BTreeSet<Key>) -> Self {
        self.disabled_items = disabled_items;
        self
    }

    /// Sets [`on_change`](Self::on_change).
    #[must_use]
    pub fn on_change(
        mut self,
        callback: impl Into<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
    ) -> Self {
        self.on_change = Some(callback.into());
        self
    }
}

/// The context for the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled or uncontrolled selected item set.
    pub value: Bindable<BTreeSet<Key>>,

    /// Currently focused item value.
    pub focused_item: Option<Key>,

    /// Whether focus should render as keyboard-visible focus.
    pub focus_visible: bool,

    /// Selection behavior for item activation.
    pub selection_mode: SelectionMode,

    /// Whether the whole group is disabled.
    pub disabled: bool,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Whether arrow navigation wraps at the ends.
    pub loop_focus: bool,

    /// Whether items use the roving tabindex pattern.
    pub roving_focus: bool,

    /// Whether the last selected item cannot be deselected.
    pub disallow_empty_selection: bool,

    /// Registered item values in logical DOM order.
    pub registered_items: Vec<Key>,

    /// Set of item values disabled independently from the group.
    pub disabled_items: BTreeSet<Key>,

    /// Active locale inherited from provider environment.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

/// Per-item context provided by adapters to child toggle buttons.
#[derive(Clone, Debug)]
pub struct ToggleGroupItemContext {
    /// Parent group component ID.
    pub group_id: String,

    /// Parent group selection mode.
    pub selection_mode: SelectionMode,

    /// Parent group orientation.
    pub orientation: Orientation,

    /// Whether the parent group is disabled.
    pub disabled: bool,

    /// Whether the parent group uses roving focus.
    pub roving_focus: bool,

    /// Callback used by item adapters to send group events.
    pub send: Callback<dyn Fn(Event) + Send + Sync>,
}

/// Localizable strings for `ToggleGroup`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Fallback accessible label for the group root.
    pub group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            group_label: MessageFn::new(|_locale: &Locale| String::from("Toggle group")),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the toggle group machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_change`] with the requested selected set.
    ValueChange,

    /// Adapter moves DOM focus to the item keyed by [`Context::focused_item`].
    FocusItem,
}

/// Hidden input configuration for native form submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenInputConfig {
    /// Form field name.
    pub name: String,

    /// Submitted value shape.
    pub value: HiddenInputValue,

    /// Optional ID of an associated form element.
    pub form_id: Option<String>,

    /// Whether generated hidden inputs should render disabled.
    pub disabled: bool,
}

/// Hidden input value shape for native form submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiddenInputValue {
    /// Render no submitted value for this field.
    None,

    /// Submit one scalar value for this form field.
    Single(String),

    /// Submit one scalar value per selected item for this form field.
    Multiple(Vec<String>),
}

/// The machine for the `ToggleGroup` component.
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
                value: if let Some(value) = &props.value {
                    Bindable::controlled(normalize_value(value.clone(), props.selection_mode))
                } else {
                    Bindable::uncontrolled(normalize_value(
                        props.default_value.clone(),
                        props.selection_mode,
                    ))
                },
                focused_item: None,
                focus_visible: false,
                selection_mode: props.selection_mode,
                disabled: props.disabled,
                orientation: props.orientation,
                dir: props.dir,
                loop_focus: props.loop_focus,
                roving_focus: props.roving_focus,
                disallow_empty_selection: props.disallow_empty_selection,
                registered_items: Vec::new(),
                disabled_items: props.disabled_items.clone(),
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
        if ctx.disabled
            && matches!(
                event,
                Event::SelectItem(_) | Event::DeselectItem(_) | Event::ToggleItem(_)
            )
        {
            return None;
        }

        if props.read_only
            && matches!(
                event,
                Event::SelectItem(_) | Event::DeselectItem(_) | Event::ToggleItem(_)
            )
        {
            return None;
        }

        match (state, event) {
            (_, Event::SelectItem(item)) => {
                if is_item_disabled(ctx, item) {
                    return None;
                }

                let mut next = ctx.value.get().clone();

                match ctx.selection_mode {
                    SelectionMode::None => return None,

                    SelectionMode::Single => {
                        next.clear();
                        next.insert(item.clone());
                    }

                    SelectionMode::Multiple => {
                        next.insert(item.clone());
                    }
                }

                value_change_plan(ctx, next)
            }

            (_, Event::DeselectItem(item)) => {
                if is_item_disabled(ctx, item) || !ctx.value.get().contains(item) {
                    return None;
                }

                if ctx.disallow_empty_selection && ctx.value.get().len() <= 1 {
                    return None;
                }

                let mut next = ctx.value.get().clone();

                next.remove(item);

                value_change_plan(ctx, next)
            }

            (_, Event::ToggleItem(item)) => {
                if ctx.value.get().contains(item) {
                    Self::transition(state, &Event::DeselectItem(item.clone()), ctx, props)
                } else {
                    Self::transition(state, &Event::SelectItem(item.clone()), ctx, props)
                }
            }

            (_, Event::Focus { item, is_keyboard }) => {
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
                let target = idle_focus_seed(ctx, FocusStep::Next)?;
                Some(focus_plan(target, true, true))
            }

            (State::Idle, Event::FocusPrev) => {
                let target = idle_focus_seed(ctx, FocusStep::Prev)?;
                Some(focus_plan(target, true, true))
            }

            (State::Focused { item }, Event::FocusNext) => {
                let target = step_focus(ctx, item, FocusStep::Next)?;
                Some(focus_plan(target, true, true))
            }

            (State::Focused { item }, Event::FocusPrev) => {
                let target = step_focus(ctx, item, FocusStep::Prev)?;
                Some(focus_plan(target, true, true))
            }

            (_, Event::FocusFirst) => {
                let target = first_enabled(ctx)?;
                Some(focus_plan(target, true, true))
            }

            (_, Event::FocusLast) => {
                let target = last_enabled(ctx)?;
                Some(focus_plan(target, true, true))
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
                let controlled = ctx.value.is_controlled();

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

                    if !controlled {
                        let mut next = ctx.value.get().clone();

                        next.remove(&item);

                        ctx.value.set(next);
                    }

                    if focused_removed {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }
                }))
            }

            (_, Event::Reset) => {
                let next = normalize_value(props.default_value.clone(), ctx.selection_mode);
                value_change_plan(ctx, next)
            }

            (_, Event::SetValue(value)) => {
                let value = value
                    .clone()
                    .map(|value| normalize_value(value, props.selection_mode));

                let internal = value.clone().unwrap_or_else(|| {
                    normalize_value(ctx.value.get().clone(), props.selection_mode)
                });

                if ctx.value.is_controlled() == value.is_some()
                    && ctx.value.get() == value.as_ref().unwrap_or(ctx.value.get())
                {
                    return Some(TransitionPlan::new());
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(internal);
                    ctx.value.sync_controlled(value);
                }))
            }

            (_, Event::SetProps) => Some(sync_props_plan(state, ctx, props)),
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "toggle_group::Props.id must remain stable after init",
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        if old.disabled != new.disabled
            || old.orientation != new.orientation
            || old.dir != new.dir
            || old.loop_focus != new.loop_focus
            || old.roving_focus != new.roving_focus
            || old.selection_mode != new.selection_mode
            || old.read_only != new.read_only
            || old.disallow_empty_selection != new.disallow_empty_selection
            || old.disabled_items != new.disabled_items
        {
            events.push(Event::SetProps);
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

/// DOM parts of the `ToggleGroup` component.
#[derive(ComponentPart)]
#[scope = "toggle-group"]
pub enum Part {
    /// The root group element.
    Root,

    /// An item button within the group.
    Item {
        /// Stable item value.
        value: Key,
    },

    /// The optional animated selection indicator.
    Indicator,
}

/// The API for the `ToggleGroup` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("toggle_group::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns whether the item with the given value is selected.
    #[must_use]
    pub fn is_selected(&self, item_id: &Key) -> bool {
        self.ctx.value.get().contains(item_id)
    }

    /// Returns whether the whole group is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns whether a specific item is disabled by group or item state.
    #[must_use]
    pub fn is_item_disabled(&self, item_id: &Key) -> bool {
        is_item_disabled(self.ctx, item_id)
    }

    /// Returns the currently focused item value, if any.
    #[must_use]
    pub const fn focused_item(&self) -> Option<&Key> {
        self.ctx.focused_item.as_ref()
    }

    /// Root group element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        let orientation = orientation_token(self.ctx.orientation);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, root_role(self.ctx.selection_mode))
            .set(HtmlAttr::Data("ars-orientation"), orientation);

        if self.ctx.selection_mode == SelectionMode::Single {
            attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), orientation);
        }

        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }

        if let Some(labelledby) = &self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby.clone());
        } else if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.clone());
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.group_label)(&self.ctx.locale),
            );
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

        if self.props.required && self.ctx.selection_mode == SelectionMode::Single {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.props.read_only {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Item button attributes.
    #[must_use]
    pub fn item_attrs(&self, item_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            value: Key::default(),
        }
        .data_attrs();

        let selected = self.is_selected(item_id);
        let focused = self.ctx.focused_item.as_ref() == Some(item_id);
        let disabled = self.is_item_disabled(item_id);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-value"), item_id.to_string())
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, self.item_tabindex(item_id));

        match self.ctx.selection_mode {
            SelectionMode::Single => {
                attrs
                    .set(HtmlAttr::Role, "radio")
                    .set(HtmlAttr::Aria(AriaAttr::Checked), bool_token(selected));
            }

            SelectionMode::Multiple => {
                attrs
                    .set(HtmlAttr::Role, "button")
                    .set(HtmlAttr::Aria(AriaAttr::Pressed), bool_token(selected));
            }

            SelectionMode::None => {
                attrs.set(HtmlAttr::Role, "button");
            }
        }

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

    /// Optional animated selection indicator attributes.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(selected) = self.ctx.value.get().iter().next() {
            attrs.set(HtmlAttr::Data("ars-active-value"), selected.to_string());
        }

        attrs
    }

    /// Hidden input configuration for native form submission.
    #[must_use]
    pub fn hidden_input_config(&self) -> Option<HiddenInputConfig> {
        let name = self.props.name.as_ref()?;

        let value = match self.ctx.selection_mode {
            SelectionMode::None => HiddenInputValue::None,

            SelectionMode::Single => self
                .ctx
                .value
                .get()
                .iter()
                .next()
                .map_or(HiddenInputValue::None, |selected| {
                    HiddenInputValue::Single(selected.to_string())
                }),

            SelectionMode::Multiple => {
                let selected = self
                    .ctx
                    .value
                    .get()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                if selected.is_empty() {
                    HiddenInputValue::None
                } else {
                    HiddenInputValue::Multiple(selected)
                }
            }
        };

        Some(HiddenInputConfig {
            name: name.clone(),
            value,
            form_id: self.props.form.clone(),
            disabled: self.ctx.disabled,
        })
    }

    /// Dispatches a click/press activation for an item.
    pub fn on_item_click(&self, item_id: &Key) {
        if !self.is_item_disabled(item_id) {
            (self.send)(Event::ToggleItem(item_id.clone()));
        }
    }

    /// Dispatches a focus event for an item.
    pub fn on_item_focus(&self, item_id: &Key, is_keyboard: bool) {
        (self.send)(Event::Focus {
            item: item_id.clone(),
            is_keyboard,
        });
    }

    /// Dispatches a group blur event.
    pub fn on_item_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches keyboard navigation or activation for an item.
    pub fn on_item_keydown(&self, item_id: &Key, data: &KeyboardEventData) {
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
            KeyboardKey::Space | KeyboardKey::Enter => {
                (self.send)(Event::ToggleItem(item_id.clone()));
            }
            _ => {}
        }
    }

    /// Dispatches an item mount registration event.
    pub fn on_item_mount(&self, item_id: &Key) {
        (self.send)(Event::RegisterItem(item_id.clone()));
    }

    /// Dispatches an item unmount registration event.
    pub fn on_item_unmount(&self, item_id: &Key) {
        (self.send)(Event::UnregisterItem(item_id.clone()));
    }

    /// Dispatches a native form reset event.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
    }

    fn item_tabindex(&self, item_id: &Key) -> &'static str {
        if is_item_focus_disabled(self.ctx, item_id) {
            return "-1";
        }

        if !self.ctx.roving_focus {
            return "0";
        }

        if self.is_roving_anchor(item_id) {
            "0"
        } else {
            "-1"
        }
    }

    fn is_roving_anchor(&self, item_id: &Key) -> bool {
        match self.state {
            State::Focused { item } => item == item_id,

            State::Idle => {
                let first_registered_selected =
                    self.ctx.registered_items.iter().find(|item| {
                        !is_item_focus_disabled(self.ctx, item) && self.is_selected(item)
                    });

                if let Some(selected) = first_registered_selected {
                    return selected == item_id;
                }

                first_enabled(self.ctx).as_ref() == Some(item_id)
            }
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref value } => self.item_attrs(value),
            Part::Indicator => self.indicator_attrs(),
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

const fn root_role(mode: SelectionMode) -> &'static str {
    match mode {
        SelectionMode::Single => "radiogroup",
        SelectionMode::None | SelectionMode::Multiple => "group",
    }
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn is_item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled || ctx.disabled_items.contains(item)
}

fn is_item_focus_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled_items.contains(item)
}

fn can_focus_item(ctx: &Context, item: &Key) -> bool {
    ctx.registered_items
        .iter()
        .any(|registered| registered == item)
        && !is_item_focus_disabled(ctx, item)
}

fn first_enabled(ctx: &Context) -> Option<Key> {
    ctx.registered_items
        .iter()
        .find(|item| !is_item_focus_disabled(ctx, item))
        .cloned()
}

fn last_enabled(ctx: &Context) -> Option<Key> {
    ctx.registered_items
        .iter()
        .rev()
        .find(|item| !is_item_focus_disabled(ctx, item))
        .cloned()
}

fn idle_focus_seed(ctx: &Context, step: FocusStep) -> Option<Key> {
    if let Some(selected) = ctx
        .registered_items
        .iter()
        .find(|item| ctx.value.get().contains(*item) && !is_item_focus_disabled(ctx, item))
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
    let enabled = ctx
        .registered_items
        .iter()
        .filter(|item| !is_item_focus_disabled(ctx, item))
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        return None;
    }

    let current_index = enabled.iter().position(|item| *item == current)?;

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

fn normalize_value(mut value: BTreeSet<Key>, mode: SelectionMode) -> BTreeSet<Key> {
    match mode {
        SelectionMode::None => BTreeSet::new(),
        SelectionMode::Multiple => value,
        SelectionMode::Single => value.pop_first().into_iter().collect(),
    }
}

fn value_change_plan(ctx: &Context, next: BTreeSet<Key>) -> Option<TransitionPlan<Machine>> {
    let next = normalize_value(next, ctx.selection_mode);

    if ctx.value.get() == &next {
        return Some(TransitionPlan::new());
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

fn value_change_effect(next: BTreeSet<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ValueChange, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_change {
            callback(next);
        }

        no_cleanup()
    })
}

fn sync_props_plan(state: &State, ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let selection_mode = props.selection_mode;
    let disabled = props.disabled;
    let orientation = props.orientation;
    let dir = props.dir;
    let loop_focus = props.loop_focus;
    let roving_focus = props.roving_focus;
    let disallow_empty_selection = props.disallow_empty_selection;
    let disabled_items = props.disabled_items.clone();

    let focused_will_be_disabled = ctx
        .focused_item
        .as_ref()
        .is_some_and(|focused| disabled_items.contains(focused));

    let target_idle = matches!(state, State::Focused { .. }) && focused_will_be_disabled;

    let apply = move |ctx: &mut Context| {
        ctx.selection_mode = selection_mode;
        ctx.disabled = disabled;
        ctx.orientation = orientation;
        ctx.dir = dir;
        ctx.loop_focus = loop_focus;
        ctx.roving_focus = roving_focus;
        ctx.disallow_empty_selection = disallow_empty_selection;
        ctx.disabled_items = disabled_items;

        let normalized = normalize_value(ctx.value.get().clone(), selection_mode);

        if ctx.value.is_controlled() {
            ctx.value.sync_controlled(Some(normalized));
        } else {
            ctx.value.set(normalized);
        }

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
    use alloc::{string::ToString, sync::Arc, vec, vec::Vec};
    use std::sync::Mutex;

    use ars_core::{ConnectApi, Service, StrongSend, callback};
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
        Props::new().id("format").aria_label("Formatting")
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

    fn snapshot_config(config: Option<&HiddenInputConfig>) -> String {
        format!("{config:#?}")
    }

    #[test]
    fn toggle_group_initial_state_is_idle_with_default_value() {
        let empty = service(props());

        assert_eq!(empty.state(), &State::Idle);
        assert!(empty.context().value.get().is_empty());
        assert!(!empty.context().value.is_controlled());

        let selected = service(props().default_value(key_set(&["bold"])));

        assert_eq!(selected.state(), &State::Idle);
        assert_eq!(selected.context().value.get(), &key_set(&["bold"]));
        assert!(!selected.context().value.is_controlled());
    }

    #[test]
    fn toggle_group_state_display_matches_state_names() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Focused { item: key("bold") }.to_string(), "focused");
    }

    #[test]
    fn toggle_group_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("format")
            .value(key_set(&["bold"]))
            .uncontrolled()
            .default_value(key_set(&["italic"]))
            .selection_mode(SelectionMode::Multiple)
            .disabled(true)
            .orientation(Orientation::Vertical)
            .dir(Direction::Rtl)
            .loop_focus(false)
            .roving_focus(false)
            .aria_label("Tools")
            .aria_labelledby("tools-label")
            .disallow_empty_selection(true)
            .name("format")
            .invalid(true)
            .required(true)
            .form("article")
            .read_only(true)
            .disabled_items(key_set(&["strike"]))
            .on_change(callback(|_: BTreeSet<Key>| {}));

        assert_eq!(props.id, "format");
        assert_eq!(props.value, None);
        assert_eq!(props.default_value, key_set(&["italic"]));
        assert_eq!(props.selection_mode, SelectionMode::Multiple);
        assert!(props.disabled);
        assert_eq!(props.orientation, Orientation::Vertical);
        assert_eq!(props.dir, Direction::Rtl);
        assert!(!props.loop_focus);
        assert!(!props.roving_focus);
        assert_eq!(props.aria_label.as_deref(), Some("Tools"));
        assert_eq!(props.aria_labelledby.as_deref(), Some("tools-label"));
        assert!(props.disallow_empty_selection);
        assert_eq!(props.name.as_deref(), Some("format"));
        assert!(props.invalid);
        assert!(props.required);
        assert_eq!(props.form.as_deref(), Some("article"));
        assert!(props.read_only);
        assert_eq!(props.disabled_items, key_set(&["strike"]));
        assert!(props.on_change.is_some());
    }

    #[test]
    fn toggle_group_set_props_syncs_controlled_value_and_context_fields() {
        let old = props().value(key_set(&["bold"]));
        let new = Props {
            value: Some(key_set(&["italic"])),
            disabled: true,
            orientation: Orientation::Vertical,
            dir: Direction::Rtl,
            loop_focus: false,
            roving_focus: false,
            selection_mode: SelectionMode::Multiple,
            read_only: true,
            disallow_empty_selection: true,
            disabled_items: key_set(&["strike"]),
            ..old.clone()
        };

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            vec![Event::SetValue(Some(key_set(&["italic"]))), Event::SetProps],
        );

        let mut service = service(old);
        let result = service.set_props(new);

        assert!(result.context_changed);
        assert_eq!(service.context().value.get(), &key_set(&["italic"]));
        assert!(service.context().value.is_controlled());
        assert!(service.context().disabled);
        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert!(!service.context().loop_focus);
        assert!(!service.context().roving_focus);
        assert_eq!(service.context().selection_mode, SelectionMode::Multiple);
        assert!(service.context().disallow_empty_selection);
        assert_eq!(service.context().disabled_items, key_set(&["strike"]));

        let result = service.set_props(props().selection_mode(SelectionMode::Multiple));

        assert!(result.context_changed);
        assert!(!service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), &key_set(&["italic"]));
    }

    #[test]
    fn toggle_group_set_props_ignores_render_only_callbacks() {
        let old = props();
        let new = props()
            .aria_label("Other")
            .aria_labelledby("other-label")
            .name("format")
            .invalid(true)
            .required(true)
            .form("article")
            .on_change(callback(|_: BTreeSet<Key>| {}));

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn toggle_group_set_props_emits_for_each_behavioral_prop() {
        let old = props();
        let cases = [
            props().orientation(Orientation::Vertical),
            props().dir(Direction::Rtl),
            props().loop_focus(false),
            props().roving_focus(false),
            props().selection_mode(SelectionMode::Multiple),
            props().read_only(true),
            props().disallow_empty_selection(true),
            props().disabled_items(key_set(&["bold"])),
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                vec![Event::SetProps],
            );
        }
    }

    #[test]
    fn toggle_group_set_props_normalizes_controlled_value_when_mode_changes() {
        let mut service = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .value(key_set(&["bold", "italic"])),
        );

        let result = service.set_props(
            props()
                .selection_mode(SelectionMode::Single)
                .value(key_set(&["bold", "italic"])),
        );

        assert!(result.context_changed);
        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), &key_set(&["bold"]));
    }

    #[test]
    fn toggle_group_select_and_deselect_item_update_selection() {
        let mut service = service(props().selection_mode(SelectionMode::Multiple));

        register(&mut service, &["bold"]);

        drop(service.send(Event::SelectItem(key("bold"))));

        assert_eq!(service.context().value.get(), &key_set(&["bold"]));

        drop(service.send(Event::DeselectItem(key("bold"))));

        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn toggle_group_deselect_noops_for_disabled_or_unselected_items() {
        let mut disabled = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["bold"]))
                .disabled_items(key_set(&["bold"])),
        );

        register(&mut disabled, &["bold"]);

        let disabled_result = disabled.send(Event::DeselectItem(key("bold")));

        assert!(!disabled_result.context_changed);
        assert!(disabled_result.pending_effects.is_empty());
        assert_eq!(disabled.context().value.get(), &key_set(&["bold"]));

        let mut unselected = service(props().selection_mode(SelectionMode::Multiple));

        register(&mut unselected, &["bold"]);

        let unselected_result = unselected.send(Event::DeselectItem(key("bold")));

        assert!(!unselected_result.context_changed);
        assert!(unselected_result.pending_effects.is_empty());
        assert!(unselected.context().value.get().is_empty());
    }

    #[test]
    fn toggle_group_single_mode_allows_only_one_selected_item() {
        let mut service = service(props());

        register(&mut service, &["bold", "italic"]);

        drop(service.send(Event::SelectItem(key("bold"))));
        drop(service.send(Event::SelectItem(key("italic"))));

        assert_eq!(service.context().value.get(), &key_set(&["italic"]));
    }

    #[test]
    fn toggle_group_multiple_mode_allows_multiple_selected_items() {
        let mut service = service(props().selection_mode(SelectionMode::Multiple));

        register(&mut service, &["bold", "italic"]);

        drop(service.send(Event::SelectItem(key("bold"))));
        drop(service.send(Event::SelectItem(key("italic"))));

        assert_eq!(service.context().value.get(), &key_set(&["bold", "italic"]));
    }

    #[test]
    fn toggle_group_none_mode_never_selects_items() {
        let mut service = service(props().selection_mode(SelectionMode::None));

        register(&mut service, &["bold"]);

        let result = service.send(Event::ToggleItem(key("bold")));

        assert!(!result.context_changed);
        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn toggle_group_disabled_ignores_value_changes_but_allows_prop_sync() {
        let mut service = service(props().disabled(true));

        register(&mut service, &["bold"]);

        assert!(!service.send(Event::SelectItem(key("bold"))).context_changed);
        assert!(service.context().value.get().is_empty());

        let result = service.send(Event::SetValue(Some(key_set(&["bold"]))));

        assert!(result.context_changed);
        assert_eq!(service.context().value.get(), &key_set(&["bold"]));
        assert!(service.context().disabled);
    }

    #[test]
    fn toggle_group_read_only_ignores_value_changes() {
        let mut service = service(props().read_only(true));

        register(&mut service, &["bold"]);

        assert!(!service.send(Event::SelectItem(key("bold"))).context_changed);
        assert!(service.context().value.get().is_empty());
    }

    #[test]
    fn toggle_group_disallow_empty_selection_keeps_last_item() {
        let mut service = service(
            props()
                .default_value(key_set(&["bold"]))
                .disallow_empty_selection(true),
        );

        register(&mut service, &["bold"]);

        let result = service.send(Event::DeselectItem(key("bold")));

        assert!(!result.context_changed);
        assert_eq!(service.context().value.get(), &key_set(&["bold"]));
    }

    #[test]
    fn toggle_group_controlled_mode_emits_change_without_committing_state() {
        let mut service = service(props().value(key_set(&[])));

        register(&mut service, &["bold"]);

        let result = service.send(Event::SelectItem(key("bold")));

        assert_eq!(service.context().value.get(), &key_set(&[]));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);
    }

    #[test]
    fn toggle_group_on_change_callback_fires_for_selection_changes() {
        let changes = Arc::new(Mutex::new(Vec::new()));

        let mut service = service(props().selection_mode(SelectionMode::Multiple).on_change(
            callback({
                let changes = Arc::clone(&changes);

                move |value: BTreeSet<Key>| {
                    changes.lock().unwrap().push(value);
                }
            }),
        ));

        register(&mut service, &["bold", "italic"]);

        let select = service.send(Event::SelectItem(key("bold")));
        let add = service.send(Event::SelectItem(key("italic")));
        let remove = service.send(Event::DeselectItem(key("bold")));

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
                key_set(&["bold"]),
                key_set(&["bold", "italic"]),
                key_set(&["italic"]),
            ],
        );
    }

    #[test]
    fn toggle_group_reset_restores_default_value() {
        let mut service = service(props().default_value(key_set(&["italic"])));

        register(&mut service, &["bold", "italic"]);

        drop(service.send(Event::SelectItem(key("bold"))));

        assert_eq!(service.context().value.get(), &key_set(&["bold"]));

        drop(service.send(Event::Reset));

        assert_eq!(service.context().value.get(), &key_set(&["italic"]));
    }

    #[test]
    fn toggle_group_reset_restores_default_value_when_disabled() {
        let mut service = service(props().default_value(key_set(&["italic"])).disabled(true));

        service.context_mut().value.set(key_set(&["bold"]));

        drop(service.send(Event::Reset));

        assert_eq!(service.context().value.get(), &key_set(&["italic"]));
    }

    #[test]
    fn toggle_group_register_unregister_maintain_item_list() {
        let mut service = service(props());

        drop(service.send(Event::RegisterItem(key("bold"))));
        drop(service.send(Event::RegisterItem(key("italic"))));

        assert_eq!(
            service.context().registered_items,
            vec![key("bold"), key("italic")]
        );

        drop(service.send(Event::UnregisterItem(key("bold"))));

        assert_eq!(service.context().registered_items, vec![key("italic")]);
    }

    #[test]
    fn toggle_group_register_deduplicates_existing_keys() {
        let mut service = service(props());

        drop(service.send(Event::RegisterItem(key("bold"))));

        let result = service.send(Event::RegisterItem(key("bold")));

        assert!(!result.context_changed);
        assert_eq!(service.context().registered_items, vec![key("bold")]);
    }

    #[test]
    fn toggle_group_unregister_removes_selection_and_focus_for_removed_item() {
        let mut service = service(props().default_value(key_set(&["bold"])));

        register(&mut service, &["bold"]);

        drop(service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let result = service.send(Event::UnregisterItem(key("bold")));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().registered_items.is_empty());
        assert!(service.context().value.get().is_empty());
        assert_eq!(service.context().focused_item, None);
    }

    #[test]
    fn toggle_group_focus_event_transitions_idle_to_focused() {
        let mut service = service(props());

        register(&mut service, &["bold"]);

        let result = service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Focused { item: key("bold") });
        assert_eq!(service.context().focused_item, Some(key("bold")));
        assert!(service.context().focus_visible);
    }

    #[test]
    fn toggle_group_focus_and_blur_noops_are_precise() {
        let mut service = service(props());

        register(&mut service, &["bold", "italic"]);

        drop(service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let repeated = service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        });

        assert!(!repeated.state_changed);
        assert!(!repeated.context_changed);
        assert!(repeated.pending_effects.is_empty());

        let mouse_focus = service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: false,
        });

        assert!(mouse_focus.context_changed);
        assert!(!service.context().focus_visible);

        let other_item = service.send(Event::Focus {
            item: key("italic"),
            is_keyboard: false,
        });

        assert!(other_item.state_changed);
        assert_eq!(service.context().focused_item, Some(key("italic")));

        let blur = service.send(Event::Blur);

        assert!(blur.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);

        let repeated_blur = service.send(Event::Blur);

        assert!(!repeated_blur.state_changed);
        assert!(!repeated_blur.context_changed);
    }

    #[test]
    fn toggle_group_blur_repairs_stale_idle_focus_context() {
        let mut ctx = service(props()).context().clone();

        ctx.focused_item = Some(key("bold"));

        let plan =
            <Machine as ars_core::Machine>::transition(&State::Idle, &Event::Blur, &ctx, &props());

        assert!(plan.is_some());
    }

    #[test]
    fn toggle_group_focus_next_prev_walk_registered_items() {
        let mut service = service(props());

        register(&mut service, &["bold", "italic", "strike"]);

        let first = service.send(Event::FocusNext);

        assert_eq!(service.context().focused_item, Some(key("bold")));
        assert_eq!(first.pending_effects[0].name, Effect::FocusItem);

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item, Some(key("italic")));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item, Some(key("bold")));

        drop(service.send(Event::Focus {
            item: key("strike"),
            is_keyboard: true,
        }));
        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_item, Some(key("italic")));
    }

    #[test]
    fn toggle_group_idle_focus_uses_registered_selected_order() {
        let mut service = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["z-first", "a-second"])),
        );

        register(&mut service, &["z-first", "a-second"]);

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_item, Some(key("z-first")));
    }

    #[test]
    fn toggle_group_home_end_move_to_first_last_enabled_item() {
        let mut service = service(props().disabled_items(key_set(&["bold"])));

        register(&mut service, &["bold", "italic", "strike"]);

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_item, Some(key("italic")));

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_item, Some(key("strike")));
    }

    #[test]
    fn toggle_group_loop_focus_wraps_when_enabled() {
        let mut wrapping = service(props());

        register(&mut wrapping, &["bold", "italic"]);

        drop(wrapping.send(Event::Focus {
            item: key("italic"),
            is_keyboard: true,
        }));
        drop(wrapping.send(Event::FocusNext));

        assert_eq!(wrapping.context().focused_item, Some(key("bold")));

        let mut clamped = service(props().loop_focus(false));

        register(&mut clamped, &["bold", "italic"]);

        drop(clamped.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let prev_result = clamped.send(Event::FocusPrev);

        assert!(!prev_result.state_changed);
        assert_eq!(clamped.context().focused_item, Some(key("bold")));

        drop(clamped.send(Event::Focus {
            item: key("italic"),
            is_keyboard: true,
        }));

        let result = clamped.send(Event::FocusNext);

        assert!(!result.state_changed);
        assert_eq!(clamped.context().focused_item, Some(key("italic")));

        drop(wrapping.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));
        drop(wrapping.send(Event::FocusPrev));

        assert_eq!(wrapping.context().focused_item, Some(key("italic")));
    }

    #[test]
    fn toggle_group_focus_does_not_target_disabled_items() {
        let mut service = service(props().disabled_items(key_set(&["bold"])));

        register(&mut service, &["bold", "italic"]);

        assert!(
            !service
                .send(Event::Focus {
                    item: key("bold"),
                    is_keyboard: true,
                })
                .state_changed
        );

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_item, Some(key("italic")));
    }

    #[test]
    fn toggle_group_disabled_group_preserves_keyboard_focus_navigation() {
        let mut service = service(props().disabled(true));

        register(&mut service, &["bold", "italic"]);

        let result = service.send(Event::FocusFirst);

        assert!(result.state_changed);
        assert_eq!(service.context().focused_item, Some(key("bold")));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("bold"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true"),
        );
        assert_eq!(
            api.item_attrs(&key("bold")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
        assert_eq!(
            api.item_attrs(&key("italic")).get(&HtmlAttr::TabIndex),
            Some("-1"),
        );
    }

    #[test]
    fn toggle_group_rtl_swaps_horizontal_arrow_left_right() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let sent = Arc::clone(&sent);
            move |event| sent.lock().unwrap().push(event)
        };

        let service = service(props().dir(Direction::Rtl));

        let api = service.connect(&send);

        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowRight));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(
            *sent.lock().unwrap(),
            vec![Event::FocusPrev, Event::FocusNext],
        );
    }

    #[test]
    fn toggle_group_vertical_arrow_up_down_ignore_rtl() {
        let sent = Arc::new(Mutex::new(Vec::new()));

        let send = {
            let sent = Arc::clone(&sent);
            move |event| sent.lock().unwrap().push(event)
        };

        let service = service(
            props()
                .orientation(Orientation::Vertical)
                .dir(Direction::Rtl),
        );

        let api = service.connect(&send);

        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowDown));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowUp));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowRight));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(
            *sent.lock().unwrap(),
            vec![Event::FocusNext, Event::FocusPrev],
        );
    }

    #[test]
    fn toggle_group_keydown_dispatch_matrix_covers_orientation_and_activation() {
        let horizontal = Arc::new(Mutex::new(Vec::new()));
        let horizontal_send = {
            let horizontal = Arc::clone(&horizontal);
            move |event| horizontal.lock().unwrap().push(event)
        };

        let service = service(props());

        let api = service.connect(&horizontal_send);

        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowRight));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowLeft));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowDown));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::ArrowUp));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::Home));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::End));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::Space));
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::Enter));

        assert_eq!(
            *horizontal.lock().unwrap(),
            vec![
                Event::FocusNext,
                Event::FocusPrev,
                Event::FocusFirst,
                Event::FocusLast,
                Event::ToggleItem(key("bold")),
                Event::ToggleItem(key("bold")),
            ],
        );
    }

    #[test]
    fn toggle_group_root_attrs_emit_accessibility_contract() {
        let service = service(props());

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("toggle-group")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("format"));
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("radiogroup"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-orientation")),
            Some("horizontal")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Formatting")
        );
    }

    #[test]
    fn toggle_group_root_attrs_emit_labelledby_label_fallback_priority() {
        let labelled = service(props().aria_labelledby("format-label"));

        let attrs = labelled.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("format-label"),
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), None);

        let fallback = service(Props::new().id("format"));

        assert_eq!(
            fallback
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Toggle group"),
        );
    }

    #[test]
    fn toggle_group_root_attrs_emit_disabled_invalid_required_readonly() {
        let service = service(
            props()
                .disabled(true)
                .invalid(true)
                .required(true)
                .read_only(true),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-invalid")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-readonly")), Some("true"));
    }

    #[test]
    fn toggle_group_group_role_root_attrs_omit_unsupported_aria() {
        for mode in [SelectionMode::Multiple, SelectionMode::None] {
            let service = service(props().selection_mode(mode).required(true));

            let attrs = service.connect(&|_| {}).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Role), Some("group"));
            assert_eq!(
                attrs.get(&HtmlAttr::Data("ars-orientation")),
                Some("horizontal")
            );
            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)), None);
            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), None);
        }
    }

    #[test]
    fn toggle_group_item_attrs_emit_single_mode_radio_contract() {
        let mut service = service(props().default_value(key_set(&["bold"])));

        register(&mut service, &["bold", "italic"]);

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("radio"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-selected")), Some("true"));
    }

    #[test]
    fn toggle_group_item_attrs_emit_multiple_mode_button_pressed_contract() {
        let mut service = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["bold"])),
        );

        register(&mut service, &["bold", "italic"]);

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
    }

    #[test]
    fn toggle_group_item_attrs_emit_none_mode_toolbar_button_contract() {
        let mut service = service(props().selection_mode(SelectionMode::None));

        register(&mut service, &["bold"]);

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Checked)), None);
    }

    #[test]
    fn toggle_group_roving_tabindex_only_focused_or_fallback_item_is_zero() {
        let mut group = service(props());

        register(&mut group, &["bold", "italic"]);

        let api = group.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("bold")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
        assert_eq!(
            api.item_attrs(&key("italic")).get(&HtmlAttr::TabIndex),
            Some("-1"),
        );

        drop(group.send(Event::Focus {
            item: key("italic"),
            is_keyboard: true,
        }));

        let api = group.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("bold")).get(&HtmlAttr::TabIndex),
            Some("-1"),
        );
        assert_eq!(
            api.item_attrs(&key("italic")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );

        let mut disabled_selected = service(
            props()
                .default_value(key_set(&["bold"]))
                .disabled_items(key_set(&["bold"])),
        );

        register(&mut disabled_selected, &["bold", "italic"]);

        let api = disabled_selected.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("bold")).get(&HtmlAttr::TabIndex),
            Some("-1"),
        );
        assert_eq!(
            api.item_attrs(&key("italic")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
    }

    #[test]
    fn toggle_group_roving_focus_false_makes_all_items_tab_focusable() {
        let mut service = service(props().roving_focus(false));

        register(&mut service, &["bold", "italic"]);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.item_attrs(&key("bold")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
        assert_eq!(
            api.item_attrs(&key("italic")).get(&HtmlAttr::TabIndex),
            Some("0"),
        );
    }

    #[test]
    fn toggle_group_item_attrs_emit_disabled_and_selected_branches() {
        let mut service = service(
            props()
                .default_value(key_set(&["bold"]))
                .disabled_items(key_set(&["bold"])),
        );

        register(&mut service, &["bold"]);

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-selected")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn toggle_group_item_attrs_separate_focus_and_focus_visible() {
        let mut service = service(props());

        register(&mut service, &["bold"]);

        drop(service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: false,
        }));

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-focused")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-focus-visible")), None);

        drop(service.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let attrs = service.connect(&|_| {}).item_attrs(&key("bold"));

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-focus-visible")),
            Some("true"),
        );
    }

    #[test]
    fn toggle_group_indicator_attrs_emit_active_value_when_selected() {
        let service = service(props().default_value(key_set(&["bold"])));

        let attrs = service.connect(&|_| {}).indicator_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-active-value")), Some("bold"),);
    }

    #[test]
    fn toggle_group_hidden_input_config_absent_without_name() {
        let service = service(props().default_value(key_set(&["bold"])));

        assert_eq!(service.connect(&|_| {}).hidden_input_config(), None);
    }

    #[test]
    fn toggle_group_hidden_input_config_single_multiple_none_and_disabled() {
        let single = service(
            props()
                .default_value(key_set(&["bold"]))
                .name("format")
                .form("article"),
        );

        assert_eq!(
            single.connect(&|_| {}).hidden_input_config(),
            Some(HiddenInputConfig {
                name: "format".into(),
                value: HiddenInputValue::Single("bold".into()),
                form_id: Some("article".into()),
                disabled: false,
            }),
        );

        let multiple = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["bold", "italic"]))
                .name("format"),
        );

        assert_eq!(
            multiple.connect(&|_| {}).hidden_input_config(),
            Some(HiddenInputConfig {
                name: "format".into(),
                value: HiddenInputValue::Multiple(vec!["bold".into(), "italic".into()]),
                form_id: None,
                disabled: false,
            }),
        );

        let none = service(
            props()
                .selection_mode(SelectionMode::None)
                .default_value(key_set(&["bold"]))
                .name("format"),
        );

        assert_eq!(
            none.connect(&|_| {}).hidden_input_config().unwrap().value,
            HiddenInputValue::None,
        );

        let disabled = service(
            props()
                .default_value(key_set(&["bold"]))
                .name("format")
                .disabled(true),
        );

        assert!(
            disabled
                .connect(&|_| {})
                .hidden_input_config()
                .unwrap()
                .disabled
        );
    }

    #[test]
    fn toggle_group_api_handlers_dispatch_typed_events() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let sent = Arc::clone(&sent);
            move |event| sent.lock().unwrap().push(event)
        };

        let service = service(props());

        let api = service.connect(&send);

        api.on_item_mount(&key("bold"));
        api.on_item_focus(&key("bold"), true);
        api.on_item_keydown(&key("bold"), &keyboard(KeyboardKey::Space));
        api.on_item_click(&key("bold"));
        api.on_item_blur();
        api.on_item_unmount(&key("bold"));
        api.on_form_reset();

        assert_eq!(
            *sent.lock().unwrap(),
            vec![
                Event::RegisterItem(key("bold")),
                Event::Focus {
                    item: key("bold"),
                    is_keyboard: true,
                },
                Event::ToggleItem(key("bold")),
                Event::ToggleItem(key("bold")),
                Event::Blur,
                Event::UnregisterItem(key("bold")),
                Event::Reset,
            ],
        );
    }

    #[test]
    fn toggle_group_api_accessors_expose_current_state() {
        let mut enabled = service(props());

        register(&mut enabled, &["bold"]);

        drop(enabled.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let api = enabled.connect(&|_| {});

        assert!(!api.is_disabled());
        assert_eq!(api.focused_item(), Some(&key("bold")));

        let disabled = service(props().disabled(true));
        let api = disabled.connect(&|_| {});

        assert!(api.is_disabled());
        assert_eq!(api.focused_item(), None);
    }

    #[test]
    fn toggle_group_set_props_preserves_focus_when_group_becomes_disabled() {
        let mut group_disabled = service(props());

        register(&mut group_disabled, &["bold"]);

        drop(group_disabled.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let result = group_disabled.set_props(props().disabled(true));

        assert!(!result.state_changed);
        assert_eq!(
            group_disabled.state(),
            &State::Focused { item: key("bold") }
        );
        assert_eq!(group_disabled.context().focused_item, Some(key("bold")));
    }

    #[test]
    fn toggle_group_set_props_clears_focus_when_item_becomes_disabled() {
        let mut item_disabled = service(props());

        register(&mut item_disabled, &["bold"]);

        drop(item_disabled.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        let result = item_disabled.set_props(props().disabled_items(key_set(&["bold"])));

        assert!(result.state_changed);
        assert_eq!(item_disabled.state(), &State::Idle);
        assert_eq!(item_disabled.context().focused_item, None);
    }

    #[test]
    fn toggle_group_sync_props_plan_keeps_idle_state_without_target() {
        let mut ctx = service(props()).context().clone();

        ctx.focused_item = Some(key("bold"));

        let plan = sync_props_plan(&State::Idle, &ctx, &props().disabled(true));

        assert_eq!(plan.target, None);
    }

    #[test]
    fn toggle_group_part_attrs_dispatches_all_parts() {
        let service = service(props().default_value(key_set(&["bold"])));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(
            api.part_attrs(Part::Item { value: key("bold") }),
            api.item_attrs(&key("bold")),
        );
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
    }

    #[test]
    fn toggle_group_snapshots_cover_output_branches() {
        let mut idle = service(props());

        register(&mut idle, &["bold", "italic"]);

        assert_snapshot!(
            "toggle_group_root_idle",
            snapshot_attrs(&idle.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "toggle_group_item_single_unselected",
            snapshot_attrs(&idle.connect(&|_| {}).item_attrs(&key("italic")))
        );

        let mut focused = service(props());

        register(&mut focused, &["bold"]);

        drop(focused.send(Event::Focus {
            item: key("bold"),
            is_keyboard: true,
        }));

        assert_snapshot!(
            "toggle_group_item_focused_keyboard",
            snapshot_attrs(&focused.connect(&|_| {}).item_attrs(&key("bold")))
        );

        let mut selected = service(props().default_value(key_set(&["bold"])));

        register(&mut selected, &["bold"]);

        assert_snapshot!(
            "toggle_group_item_single_selected",
            snapshot_attrs(&selected.connect(&|_| {}).item_attrs(&key("bold")))
        );
        assert_snapshot!(
            "toggle_group_indicator_selected",
            snapshot_attrs(&selected.connect(&|_| {}).indicator_attrs())
        );

        let mut multiple = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["bold"]))
                .disabled_items(key_set(&["bold"])),
        );

        register(&mut multiple, &["bold"]);

        assert_snapshot!(
            "toggle_group_item_multiple_disabled_selected",
            snapshot_attrs(&multiple.connect(&|_| {}).item_attrs(&key("bold")))
        );

        let none = service(
            props()
                .selection_mode(SelectionMode::None)
                .disabled(true)
                .invalid(true)
                .required(true)
                .read_only(true),
        );

        assert_snapshot!(
            "toggle_group_root_none_disabled_invalid_required_readonly",
            snapshot_attrs(&none.connect(&|_| {}).root_attrs())
        );

        let hidden = service(
            props()
                .selection_mode(SelectionMode::Multiple)
                .default_value(key_set(&["bold", "italic"]))
                .name("format")
                .form("article"),
        );

        assert_snapshot!(
            "toggle_group_hidden_input_config_multiple",
            snapshot_config(hidden.connect(&|_| {}).hidden_input_config().as_ref())
        );
    }
}
