//! RadioGroup component state machine and connect API.
//!
//! This module implements the framework-agnostic `RadioGroup` machine defined
//! in `spec/components/input/radio-group.md`. The machine owns single-value
//! selection, stable-key roving focus, per-item disabled state, and ARIA/form
//! metadata while leaving DOM focus resolution to framework adapters.

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Orientation, PendingEffect, TransitionPlan,
    no_cleanup,
};
use ars_interactions::KeyboardEventData;

/// The state of the `RadioGroup` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No item is currently focused inside the group.
    #[default]
    Idle,

    /// A radio item has focus.
    Focused {
        /// The stable value key of the focused item.
        item: Key,
    },
}

/// Events accepted by the `RadioGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Select a radio item by value.
    SelectValue(Key),

    /// Focus moved to a specific item.
    FocusItem {
        /// The stable value key of the focused item.
        item: Key,

        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus left the group.
    Blur,

    /// Move focus to the next enabled item.
    FocusNext,

    /// Move focus to the previous enabled item.
    FocusPrev,

    /// Move focus to the first enabled item.
    FocusFirst,

    /// Move focus to the last enabled item.
    FocusLast,

    /// Register a rendered item in logical DOM order.
    RegisterItem(Radio),

    /// Unregister a rendered item by value.
    UnregisterItem(Key),

    /// Restore the selected value to [`Props::default_value`].
    Reset,

    /// Synchronize the externally controlled value prop.
    SetValue(Option<Key>),

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether a description part is rendered.
    SetHasDescription(bool),

    /// Track whether an error message part is rendered.
    SetHasErrorMessage(bool),
}

/// Context for the `RadioGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Selected radio value, controlled or uncontrolled.
    pub value: Bindable<Option<Key>>,

    /// The stable value key of the focused item.
    pub focused_item: Option<Key>,

    /// True when focus was initiated by keyboard navigation.
    pub focus_visible: bool,

    /// Whether the whole group is disabled.
    pub disabled: bool,

    /// Whether the selected value is read-only.
    pub readonly: bool,

    /// Whether selecting a value is required.
    pub required: bool,

    /// Whether the field is in an invalid state.
    pub invalid: bool,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Shared native form field name for all radio inputs.
    pub name: Option<String>,

    /// Whether arrow-key focus wraps at the ends.
    pub loop_focus: bool,

    /// Whether a description part is rendered and should be referenced.
    pub has_description: bool,

    /// Whether an error message part is rendered and should be referenced.
    pub has_error_message: bool,

    /// Ordered radio item registry used for roving focus.
    pub items: Vec<Radio>,

    /// Stable IDs for radio-group anatomy parts.
    pub ids: ComponentIds,
}

/// A registered radio item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Radio {
    /// Stable submitted value and identity key for this radio item.
    pub value: Key,

    /// Whether this item is disabled independent of the group state.
    pub disabled: bool,
}

impl Radio {
    /// Creates an enabled radio item with the given stable value key.
    #[must_use]
    pub fn new(value: impl Into<Key>) -> Self {
        Self {
            value: value.into(),
            disabled: false,
        }
    }

    /// Marks this radio item as disabled or enabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Props for the `RadioGroup` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled selected value. When `Some`, the component is controlled.
    pub value: Option<Key>,

    /// Default selected value for uncontrolled mode.
    pub default_value: Option<Key>,

    /// Whether the whole group is disabled.
    pub disabled: bool,

    /// Whether the selected value is read-only.
    pub readonly: bool,

    /// Whether selecting a value is required.
    pub required: bool,

    /// Whether the field is in an invalid state.
    pub invalid: bool,

    /// Layout axis used for arrow-key navigation and ARIA.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal arrow-key navigation.
    pub dir: Direction,

    /// Shared native form field name for all radio inputs.
    pub name: Option<String>,

    /// ID of the associated native form element.
    pub form: Option<String>,

    /// Whether arrow-key focus wraps at the ends.
    pub loop_focus: bool,

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
            required: false,
            invalid: false,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            name: None,
            form: None,
            loop_focus: true,
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

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
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

    /// Sets [`name`](Self::name), the shared form-submission field name.
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

    /// Sets [`loop_focus`](Self::loop_focus).
    #[must_use]
    pub const fn loop_focus(mut self, loop_focus: bool) -> Self {
        self.loop_focus = loop_focus;
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

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the radio group emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_value_change`] with the requested value.
    ValueChange,
}

/// Machine for the `RadioGroup` component.
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
                required: props.required,
                invalid: props.invalid,
                orientation: props.orientation,
                dir: props.dir,
                name: props.name.clone(),
                loop_focus: props.loop_focus,
                has_description: false,
                has_error_message: false,
                items: Vec::new(),
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
        match event {
            Event::SelectValue(value) => select_value_plan(ctx, Some(value.clone())),

            Event::FocusItem { item, is_keyboard } => {
                if !can_focus_item(ctx, item) {
                    return None;
                }

                if matches!(state, State::Focused { item: current } if current == item)
                    && ctx.focused_item.as_ref() == Some(item)
                    && ctx.focus_visible == *is_keyboard
                {
                    return None;
                }

                Some(focus_only_plan(item.clone(), *is_keyboard))
            }

            Event::Blur => {
                if matches!(state, State::Idle) && ctx.focused_item.is_none() && !ctx.focus_visible
                {
                    return None;
                }

                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_item = None;
                    ctx.focus_visible = false;
                }))
            }

            Event::FocusNext => {
                let target = match state {
                    State::Focused { item } => step_focus(ctx, item, FocusStep::Next)?,
                    State::Idle => idle_focus_seed(ctx, FocusStep::Next)?,
                };

                Some(focus_and_maybe_select_plan(ctx, target))
            }

            Event::FocusPrev => {
                let target = match state {
                    State::Focused { item } => step_focus(ctx, item, FocusStep::Prev)?,
                    State::Idle => idle_focus_seed(ctx, FocusStep::Prev)?,
                };

                Some(focus_and_maybe_select_plan(ctx, target))
            }

            Event::FocusFirst => {
                let target = first_enabled(ctx)?;
                Some(focus_and_maybe_select_plan(ctx, target))
            }

            Event::FocusLast => {
                let target = last_enabled(ctx)?;
                Some(focus_and_maybe_select_plan(ctx, target))
            }

            Event::RegisterItem(item) => {
                let item = item.clone();
                let clears_focus = item.disabled && ctx.focused_item.as_ref() == Some(&item.value);
                let plan = if clears_focus && matches!(state, State::Focused { .. }) {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    if let Some(existing) = ctx
                        .items
                        .iter_mut()
                        .find(|existing| existing.value == item.value)
                    {
                        *existing = item;
                    } else {
                        ctx.items.push(item);
                    }

                    if clears_focus {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }
                }))
            }

            Event::UnregisterItem(item) => {
                if !ctx.items.iter().any(|registered| &registered.value == item) {
                    return None;
                }

                let item = item.clone();

                let focused_removed = ctx.focused_item.as_ref() == Some(&item);
                let selected_removed = ctx.value.get().as_ref() == Some(&item);
                let controlled = ctx.value.is_controlled();

                let plan = if focused_removed {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    ctx.items.retain(|registered| registered.value != item);

                    if focused_removed {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }

                    if selected_removed && !controlled {
                        ctx.value.set(None);
                    }
                }))
            }

            Event::Reset => Some(reset_value_plan(ctx, props.default_value.clone())),

            Event::SetValue(value) => {
                let value = value.clone();
                let internal = value.clone().or_else(|| ctx.value.get().clone());

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(internal);
                    ctx.value.sync_controlled(value.map(Some));
                }))
            }

            Event::SetProps => {
                let disabled = props.disabled;
                let readonly = props.readonly;
                let required = props.required;
                let invalid = props.invalid;
                let orientation = props.orientation;
                let dir = props.dir;
                let name = props.name.clone();
                let loop_focus = props.loop_focus;

                let clears_focus = ctx
                    .focused_item
                    .as_ref()
                    .is_some_and(|focused| disabled || is_registered_item_disabled(ctx, focused));

                let plan = if clears_focus && matches!(state, State::Focused { .. }) {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                    ctx.readonly = readonly;
                    ctx.required = required;
                    ctx.invalid = invalid;
                    ctx.orientation = orientation;
                    ctx.dir = dir;
                    ctx.name = name;
                    ctx.loop_focus = loop_focus;

                    if ctx
                        .focused_item
                        .as_ref()
                        .is_some_and(|focused| !can_focus_item(ctx, focused))
                    {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }

            Event::SetHasErrorMessage(has_error_message) => {
                let has_error_message = *has_error_message;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_error_message = has_error_message;
                }))
            }
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "radio_group::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        if old.disabled != new.disabled
            || old.readonly != new.readonly
            || old.required != new.required
            || old.invalid != new.invalid
            || old.orientation != new.orientation
            || old.dir != new.dir
            || old.name != new.name
            || old.loop_focus != new.loop_focus
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

/// Structural parts exposed by the radio-group connect API.
#[derive(ComponentPart)]
#[scope = "radio-group"]
pub enum Part {
    /// The root radiogroup container.
    Root,

    /// The visible group label.
    Label,

    /// Container for a single radio item.
    Item {
        /// Stable value key of the radio item.
        item_value: Key,
    },

    /// Focusable radio control for a single item.
    ItemControl {
        /// Stable value key of the radio item.
        item_value: Key,
    },

    /// Visual selected-state indicator for a single item.
    ItemIndicator {
        /// Stable value key of the radio item.
        item_value: Key,
    },

    /// Visible label for a single item.
    ItemLabel {
        /// Stable value key of the radio item.
        item_value: Key,
    },

    /// Hidden native radio input for form submission.
    ItemHiddenInput {
        /// Stable value key of the radio item.
        item_value: Key,
    },

    /// Optional descriptive text for the group.
    Description,

    /// Optional validation error text for the group.
    ErrorMessage,
}

/// API for the `RadioGroup` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::Item { item_value } => self.item_attrs(&item_value),
            Part::ItemControl { item_value } => self.item_control_attrs(&item_value),
            Part::ItemIndicator { item_value } => self.item_indicator_attrs(&item_value),
            Part::ItemLabel { item_value } => self.item_label_attrs(&item_value),
            Part::ItemHiddenInput { item_value } => self.item_hidden_input_attrs(&item_value),
        }
    }
}

impl Api<'_> {
    /// Returns the currently selected radio value.
    #[must_use]
    pub fn selected_value(&self) -> Option<&Key> {
        self.ctx.value.get().as_ref()
    }

    /// Returns the current focused item key for adapter-resolved DOM focus.
    #[must_use]
    pub const fn focused_item(&self) -> Option<&Key> {
        self.ctx.focused_item.as_ref()
    }

    /// Returns `true` when the item or group is disabled.
    #[must_use]
    pub fn is_item_disabled(&self, item_value: &Key) -> bool {
        is_item_disabled(self.ctx, item_value)
    }

    /// Returns attributes for the root radiogroup container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "radiogroup")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        let mut described_by = Vec::new();

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.invalid && self.ctx.has_error_message {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        attrs
    }

    /// Returns attributes for the visible group label.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Returns attributes for the description/help text.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Returns attributes for the validation error message.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert");

        attrs
    }

    /// Returns attributes for a radio item container.
    #[must_use]
    pub fn item_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            item_value: Key::default(),
        }
        .data_attrs();

        let selected = self.selected_value() == Some(item_value);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if selected { "checked" } else { "unchecked" },
            );

        if self.is_item_disabled(item_value) {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.focus_visible && self.focused_item() == Some(item_value) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for the focusable radio control.
    #[must_use]
    pub fn item_control_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemControl {
            item_value: Key::default(),
        }
        .data_attrs();

        let selected = self.selected_value() == Some(item_value);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.item("item", item_value))
            .set(HtmlAttr::Role, "radio")
            .set(
                HtmlAttr::Aria(AriaAttr::Checked),
                if selected { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.item_part("item", item_value, "label"),
            )
            .set(HtmlAttr::TabIndex, self.item_tabindex(item_value));

        if self.is_item_disabled(item_value) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        attrs
    }

    /// Returns attributes for the visual radio indicator.
    #[must_use]
    pub fn item_indicator_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator {
            item_value: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if self.selected_value() == Some(item_value) {
                    "checked"
                } else {
                    "unchecked"
                },
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns attributes for a radio item label.
    #[must_use]
    pub fn item_label_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemLabel {
            item_value: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Id,
                self.ctx.ids.item_part("item", item_value, "label"),
            )
            .set(
                HtmlAttr::For,
                self.ctx.ids.item_part("item", item_value, "input"),
            );

        attrs
    }

    /// Returns attributes for the hidden native radio input.
    #[must_use]
    pub fn item_hidden_input_attrs(&self, item_value: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemHiddenInput {
            item_value: Key::default(),
        }
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "radio")
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Value, item_value.to_string());

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        if self.selected_value() == Some(item_value) {
            attrs.set_bool(HtmlAttr::Checked, true);
        }

        if self.is_item_disabled(item_value) {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        attrs
    }

    /// Dispatches a click/press activation for an item.
    pub fn on_item_control_click(&self, item_value: &Key) {
        if !self.is_item_disabled(item_value) {
            (self.send)(Event::SelectValue(item_value.clone()));
        }
    }

    /// Dispatches keyboard activation or roving-focus navigation.
    pub fn on_item_control_keydown(&self, item_value: &Key, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Space | KeyboardKey::Enter
                if !data.repeat && !self.is_item_disabled(item_value) =>
            {
                (self.send)(Event::SelectValue(item_value.clone()));
            }

            KeyboardKey::ArrowRight if self.ctx.orientation == Orientation::Horizontal => {
                if self.ctx.dir == Direction::Rtl {
                    (self.send)(Event::FocusPrev);
                } else {
                    (self.send)(Event::FocusNext);
                }
            }

            KeyboardKey::ArrowLeft if self.ctx.orientation == Orientation::Horizontal => {
                if self.ctx.dir == Direction::Rtl {
                    (self.send)(Event::FocusNext);
                } else {
                    (self.send)(Event::FocusPrev);
                }
            }

            KeyboardKey::ArrowDown if self.ctx.orientation == Orientation::Vertical => {
                (self.send)(Event::FocusNext);
            }

            KeyboardKey::ArrowUp if self.ctx.orientation == Orientation::Vertical => {
                (self.send)(Event::FocusPrev);
            }

            KeyboardKey::Home => (self.send)(Event::FocusFirst),

            KeyboardKey::End => (self.send)(Event::FocusLast),

            _ => {}
        }
    }

    /// Dispatches a focus event for an item.
    pub fn on_item_control_focus(&self, item_value: &Key, is_keyboard: bool) {
        if !self.is_item_disabled(item_value) {
            (self.send)(Event::FocusItem {
                item: item_value.clone(),
                is_keyboard,
            });
        }
    }

    /// Dispatches a blur event for the group.
    pub fn on_item_control_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches an item mount registration event.
    pub fn on_item_mount(&self, item: Radio) {
        (self.send)(Event::RegisterItem(item));
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
        if !can_focus_item(self.ctx, item_value) {
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
                if let Some(selected) = self.selected_value() {
                    return if can_focus_item(self.ctx, selected) {
                        selected == item_value
                    } else {
                        first_enabled(self.ctx).as_ref() == Some(item_value)
                    };
                }

                first_enabled(self.ctx).as_ref() == Some(item_value)
            }
        }
    }
}

#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

fn reset_value_plan(ctx: &Context, next: Option<Key>) -> TransitionPlan<Machine> {
    value_change_plan(ctx, next)
}

fn select_value_plan(ctx: &Context, next: Option<Key>) -> Option<TransitionPlan<Machine>> {
    if next
        .as_ref()
        .is_some_and(|value| is_item_disabled(ctx, value))
    {
        return None;
    }

    if ctx.disabled || ctx.readonly {
        return None;
    }

    Some(value_change_plan(ctx, next))
}

fn value_change_plan(ctx: &Context, next: Option<Key>) -> TransitionPlan<Machine> {
    if ctx.value.get() == &next {
        return TransitionPlan::new();
    }

    let effect = value_change_effect(next.clone());

    if ctx.value.is_controlled() {
        return TransitionPlan::new()
            .apply(|_: &mut Context| {})
            .with_effect(effect);
    }

    TransitionPlan::new()
        .apply(move |ctx: &mut Context| {
            ctx.value.set(next);
        })
        .with_effect(effect)
}

fn focus_only_plan(item: Key, is_keyboard: bool) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused { item: item.clone() }).apply(move |ctx: &mut Context| {
        ctx.focused_item = Some(item);
        ctx.focus_visible = is_keyboard;
    })
}

fn focus_and_maybe_select_plan(ctx: &Context, item: Key) -> TransitionPlan<Machine> {
    let mut plan = focus_only_plan(item.clone(), true);

    if !ctx.disabled && !ctx.readonly && ctx.value.get().as_ref() != Some(&item) {
        let effect = value_change_effect(Some(item.clone()));

        if ctx.value.is_controlled() {
            plan = plan.with_effect(effect);
        } else {
            plan = plan
                .apply(move |ctx: &mut Context| {
                    ctx.value.set(Some(item));
                })
                .with_effect(effect);
        }
    }

    plan
}

fn value_change_effect(next: Option<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ValueChange, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_value_change {
            callback(next);
        }

        no_cleanup()
    })
}

fn can_focus_item(ctx: &Context, item: &Key) -> bool {
    ctx.items.iter().any(|registered| &registered.value == item) && !is_item_disabled(ctx, item)
}

fn is_item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled || is_registered_item_disabled(ctx, item)
}

fn is_registered_item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.items
        .iter()
        .any(|registered| &registered.value == item && registered.disabled)
}

fn enabled_items(ctx: &Context) -> Vec<&Radio> {
    ctx.items
        .iter()
        .filter(|registered| !is_item_disabled(ctx, &registered.value))
        .collect()
}

fn first_enabled(ctx: &Context) -> Option<Key> {
    enabled_items(ctx)
        .first()
        .map(|registered| registered.value.clone())
}

fn last_enabled(ctx: &Context) -> Option<Key> {
    enabled_items(ctx)
        .last()
        .map(|registered| registered.value.clone())
}

fn idle_focus_seed(ctx: &Context, step: FocusStep) -> Option<Key> {
    if let Some(selected) = ctx
        .value
        .get()
        .as_ref()
        .filter(|selected| can_focus_item(ctx, selected))
    {
        return Some(selected.clone());
    }

    match step {
        FocusStep::Next => first_enabled(ctx),
        FocusStep::Prev => last_enabled(ctx),
    }
}

fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let enabled = enabled_items(ctx);

    if enabled.is_empty() {
        return None;
    }

    let current_index = enabled
        .iter()
        .position(|registered| &registered.value == current)?;

    let next_index = match step {
        FocusStep::Next => {
            if current_index + 1 < enabled.len() {
                Some(current_index + 1)
            } else if ctx.loop_focus {
                Some(0)
            } else {
                None
            }
        }

        FocusStep::Prev => {
            if current_index > 0 {
                Some(current_index - 1)
            } else if ctx.loop_focus {
                Some(enabled.len() - 1)
            } else {
                None
            }
        }
    }?;

    Some(enabled[next_index].value.clone())
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, sync::Arc, vec};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::from(value)
    }

    fn props() -> Props {
        Props::new()
            .id("shipping")
            .name("shipping_method")
            .form("checkout")
    }

    fn service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

        drop(service.send(Event::RegisterItem(Radio::new(key("standard")))));
        drop(service.send(Event::RegisterItem(Radio::new(key("express")))));
        drop(service.send(Event::RegisterItem(Radio::new(key("drone")).disabled(true))));

        service
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

    fn repeated_keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            repeat: true,
            ..keyboard(key)
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn radio_group_initial_state_is_idle_with_default_value() {
        let service = service(props().default_value(key("standard")));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(service.context().ids.id(), "shipping");
        assert_eq!(service.context().items.len(), 3);
    }

    #[test]
    fn radio_group_select_value_updates_uncontrolled_value() {
        let mut service = service(props());

        let result = service.send(Event::SelectValue(key("express")));

        assert!(result.context_changed);
        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("express"))
        );
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);
    }

    #[test]
    fn radio_group_controlled_selection_emits_without_committing_value() {
        let mut service = service(props().value(key("standard")));

        let result = service.send(Event::SelectValue(key("express")));

        assert!(result.context_changed);
        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);
    }

    #[test]
    fn radio_group_value_change_callback_receives_requested_value() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let mut service = service(props().on_value_change(callback({
            let changes = Arc::clone(&changes);
            move |value: Option<Key>| {
                changes.lock().unwrap().push(value);
            }
        })));

        let mut result = service.send(Event::SelectValue(key("express")));

        let effect = result.pending_effects.pop().expect("value-change effect");

        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().as_slice(), &[Some(key("express"))]);
    }

    #[test]
    fn radio_group_props_clearers_preserve_other_builder_fields() {
        let cleared = Props::new()
            .id("shipping")
            .value(key("express"))
            .default_value(key("standard"))
            .disabled(true)
            .form("checkout")
            .on_value_change(callback(|_: Option<Key>| {}))
            .uncontrolled()
            .no_default_value()
            .no_form()
            .no_value_change();

        assert_eq!(cleared.id, "shipping");
        assert!(cleared.disabled);
        assert_eq!(cleared.value, None);
        assert_eq!(cleared.default_value, None);
        assert_eq!(cleared.form, None);
        assert!(cleared.on_value_change.is_none());
    }

    #[test]
    fn radio_group_disabled_and_readonly_prevent_value_changes() {
        let mut disabled = service(props().disabled(true));
        let mut readonly = service(props().readonly(true));

        assert!(
            disabled
                .send(Event::SelectValue(key("standard")))
                .pending_effects
                .is_empty()
        );
        assert_eq!(disabled.context().value.get(), &None);

        assert!(
            readonly
                .send(Event::SelectValue(key("standard")))
                .pending_effects
                .is_empty()
        );
        assert_eq!(readonly.context().value.get(), &None);
    }

    #[test]
    fn radio_group_reset_restores_default_when_disabled_readonly_or_item_disabled() {
        let mut disabled = service(props().default_value(key("standard")).disabled(true));
        let mut readonly = service(props().default_value(key("standard")).readonly(true));
        let mut item_disabled = service(props().default_value(key("drone")));

        disabled.context_mut().value.set(Some(key("express")));
        readonly.context_mut().value.set(Some(key("express")));
        item_disabled.context_mut().value.set(Some(key("express")));

        drop(disabled.send(Event::Reset));
        drop(readonly.send(Event::Reset));
        drop(item_disabled.send(Event::Reset));

        assert_eq!(
            disabled.context().value.get().as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(
            readonly.context().value.get().as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(
            item_disabled.context().value.get().as_ref(),
            Some(&key("drone"))
        );
    }

    #[test]
    fn radio_group_per_item_disabled_prevents_selection_and_focus() {
        let mut service = service(props());

        assert!(service.send(Event::SelectValue(key("drone"))).is_noop());
        assert!(
            service
                .send(Event::FocusItem {
                    item: key("drone"),
                    is_keyboard: true,
                })
                .is_noop()
        );
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), &None);

        let api = service.connect(&|_| {});

        assert!(
            api.item_attrs(&key("drone"))
                .contains(&HtmlAttr::Data("ars-disabled"))
        );
        assert!(
            !api.item_attrs(&key("standard"))
                .contains(&HtmlAttr::Data("ars-disabled"))
        );
    }

    #[test]
    fn radio_group_focus_item_noop_requires_matching_state_context_and_visibility() {
        let mut service = service(props());

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));

        assert!(
            service
                .send(Event::FocusItem {
                    item: key("standard"),
                    is_keyboard: true,
                })
                .is_noop()
        );

        let result = service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: false,
        });

        assert!(result.context_changed);
        assert!(!service.context().focus_visible);

        let mut ctx = service.context().clone();

        ctx.focused_item = Some(key("express"));

        assert!(
            <Machine as ars_core::Machine>::transition(
                &State::Focused {
                    item: key("standard")
                },
                &Event::FocusItem {
                    item: key("express"),
                    is_keyboard: false,
                },
                &ctx,
                service.props(),
            )
            .is_some()
        );
    }

    #[test]
    fn radio_group_blur_noop_requires_idle_without_focus_context() {
        let service = service(props());

        assert!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::Blur,
                service.context(),
                service.props(),
            )
            .is_none()
        );

        let mut focused_ctx = service.context().clone();

        focused_ctx.focused_item = Some(key("standard"));

        assert!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::Blur,
                &focused_ctx,
                service.props(),
            )
            .is_some()
        );

        let mut visible_ctx = service.context().clone();

        visible_ctx.focus_visible = true;

        assert!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::Blur,
                &visible_ctx,
                service.props(),
            )
            .is_some()
        );

        assert!(
            <Machine as ars_core::Machine>::transition(
                &State::Focused {
                    item: key("standard")
                },
                &Event::Blur,
                service.context(),
                service.props(),
            )
            .is_some()
        );
    }

    #[test]
    fn radio_group_roving_navigation_updates_focus_and_selection_by_key() {
        let mut service = service(props());

        drop(service.send(Event::FocusNext));

        assert_eq!(
            service.state(),
            &State::Focused {
                item: key("standard")
            }
        );
        assert_eq!(
            service.context().focused_item.as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("standard"))
        );

        drop(service.send(Event::FocusNext));

        assert_eq!(
            service.state(),
            &State::Focused {
                item: key("express")
            }
        );
        assert_eq!(
            service.context().focused_item.as_ref(),
            Some(&key("express"))
        );
        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("express"))
        );
    }

    #[test]
    fn radio_group_navigation_skips_disabled_items_and_honors_loop_focus() {
        let mut wrapping = service(props());

        drop(wrapping.send(Event::FocusLast));
        drop(wrapping.send(Event::FocusNext));

        assert_eq!(
            wrapping.context().focused_item.as_ref(),
            Some(&key("standard"))
        );

        let mut clamped = service(props().loop_focus(false));

        drop(clamped.send(Event::FocusLast));

        let result = clamped.send(Event::FocusNext);

        assert!(!result.state_changed);
        assert_eq!(
            clamped.context().focused_item.as_ref(),
            Some(&key("express"))
        );
    }

    #[test]
    fn radio_group_prev_navigation_moves_backward_and_wraps_from_first() {
        let mut wrapping = service(props());

        drop(wrapping.send(Event::FocusItem {
            item: key("express"),
            is_keyboard: true,
        }));
        drop(wrapping.send(Event::FocusPrev));

        assert_eq!(
            wrapping.context().focused_item.as_ref(),
            Some(&key("standard"))
        );

        drop(wrapping.send(Event::FocusPrev));

        assert_eq!(
            wrapping.context().focused_item.as_ref(),
            Some(&key("express"))
        );

        let mut clamped = service(props().loop_focus(false));

        drop(clamped.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));

        let result = clamped.send(Event::FocusPrev);

        assert!(result.is_noop());
        assert_eq!(
            clamped.context().focused_item.as_ref(),
            Some(&key("standard"))
        );
    }

    #[test]
    fn radio_group_readonly_navigation_tracks_focus_without_value_change() {
        let mut service = service(props().readonly(true));

        drop(service.send(Event::FocusNext));

        assert_eq!(
            service.context().focused_item.as_ref(),
            Some(&key("standard"))
        );
        assert_eq!(service.context().value.get(), &None);
    }

    #[test]
    fn radio_group_unregister_removes_stale_focus_and_uncontrolled_selection() {
        let mut service = service(props().default_value(key("express")));

        drop(service.send(Event::FocusItem {
            item: key("express"),
            is_keyboard: true,
        }));
        drop(service.send(Event::UnregisterItem(key("express"))));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert_eq!(service.context().value.get(), &None);
        assert!(
            !service
                .context()
                .items
                .iter()
                .any(|item| item.value == key("express"))
        );
        assert!(
            service
                .context()
                .items
                .iter()
                .any(|item| item.value == key("standard"))
        );
    }

    #[test]
    fn radio_group_unregister_unknown_item_is_noop() {
        let mut service = service(props());

        let result = service.send(Event::UnregisterItem(key("unknown")));

        assert!(result.is_noop());
        assert_eq!(service.context().items.len(), 3);
    }

    #[test]
    fn radio_group_register_disabling_focused_item_clears_focused_state() {
        let mut service = service(props());

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));
        drop(service.send(Event::RegisterItem(
            Radio::new(key("standard")).disabled(true),
        )));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_item, None);
        assert!(!service.context().focus_visible);
        assert_eq!(
            service
                .connect(&|_| {})
                .item_control_attrs(&key("express"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn radio_group_unregister_preserves_controlled_selected_value() {
        let mut service = service(props().value(key("express")));

        drop(service.send(Event::UnregisterItem(key("express"))));

        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("express"))
        );
        assert!(
            !service
                .context()
                .items
                .iter()
                .any(|item| item.value == key("express"))
        );
    }

    #[test]
    fn radio_group_unregister_non_selected_item_preserves_uncontrolled_value() {
        let mut service = service(props().default_value(key("express")));

        drop(service.send(Event::UnregisterItem(key("standard"))));

        assert_eq!(
            service.context().value.get().as_ref(),
            Some(&key("express"))
        );
        assert!(
            !service
                .context()
                .items
                .iter()
                .any(|item| item.value == key("standard"))
        );
    }

    #[test]
    fn radio_group_set_props_preserves_focus_when_focused_item_remains_enabled() {
        let mut service = service(props());

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));
        drop(service.send(Event::SetProps));

        assert_eq!(
            service.context().focused_item.as_ref(),
            Some(&key("standard"))
        );
        assert!(service.context().focus_visible);
    }

    #[test]
    fn radio_group_set_props_clears_focused_state_when_focus_becomes_unavailable() {
        let mut service = service(props().disabled(false));

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));

        drop(service.send(Event::SetProps));
        assert_eq!(
            service.context().focused_item.as_ref(),
            Some(&key("standard"))
        );

        let plan = <Machine as ars_core::Machine>::transition(
            service.state(),
            &Event::SetProps,
            service.context(),
            &props().disabled(true),
        )
        .expect("disabled props should clear focus");

        assert_eq!(
            plan.target.as_ref(),
            Some(&State::Idle),
            "SetProps must leave state and context aligned when focus is cleared"
        );
    }

    #[test]
    fn radio_group_on_props_changed_syncs_value_and_context_props() {
        let old = props();
        let new = props()
            .value(key("express"))
            .disabled(true)
            .required(true)
            .invalid(true)
            .readonly(true)
            .orientation(Orientation::Horizontal)
            .dir(Direction::Rtl)
            .loop_focus(false)
            .no_name();

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            vec![Event::SetValue(Some(key("express"))), Event::SetProps]
        );
    }

    #[test]
    fn radio_group_on_props_changed_emits_set_props_for_each_context_field() {
        let cases = [
            props().disabled(true),
            props().readonly(true),
            props().required(true),
            props().invalid(true),
            props().orientation(Orientation::Horizontal),
            props().dir(Direction::Rtl),
            props().no_name(),
            props().loop_focus(false),
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&props(), &new),
                vec![Event::SetProps]
            );
        }
    }

    #[test]
    fn radio_group_connect_api_emits_roles_and_checked_state() {
        let service = service(props().default_value(key("express")).required(true));

        let api = service.connect(&|_| {});

        assert_eq!(api.root_attrs().get(&HtmlAttr::Role), Some("radiogroup"));
        assert_eq!(
            api.item_control_attrs(&key("standard"))
                .get(&HtmlAttr::Role),
            Some("radio")
        );
        assert_eq!(
            api.item_control_attrs(&key("express"))
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
        assert_eq!(
            api.item_control_attrs(&key("standard"))
                .get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("false")
        );
    }

    #[test]
    fn radio_group_root_describedby_requires_rendered_error_message() {
        let mut service = service(props().invalid(true));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));

        drop(service.send(Event::SetHasErrorMessage(true)));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("shipping-error-message")
        );
    }

    #[test]
    fn radio_group_item_control_uses_roving_tabindex() {
        let mut service = service(props().default_value(key("express")));

        assert_eq!(
            service
                .connect(&|_| {})
                .item_control_attrs(&key("express"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            service
                .connect(&|_| {})
                .item_control_attrs(&key("standard"))
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));

        assert_eq!(
            service
                .connect(&|_| {})
                .item_control_attrs(&key("standard"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert!(
            service
                .connect(&|_| {})
                .item_attrs(&key("standard"))
                .contains(&HtmlAttr::Data("ars-focus-visible"))
        );
        assert!(
            !service
                .connect(&|_| {})
                .item_attrs(&key("express"))
                .contains(&HtmlAttr::Data("ars-focus-visible"))
        );
    }

    #[test]
    fn radio_group_idle_roving_tabindex_falls_back_to_first_enabled_item() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(
            api.item_control_attrs(&key("standard"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            api.item_control_attrs(&key("express"))
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );
    }

    #[test]
    fn radio_group_idle_roving_tabindex_falls_back_when_selected_key_is_unfocusable() {
        let mut stale = service(props().value(key("missing")));
        let disabled = service(props().value(key("drone")));

        assert_eq!(
            stale
                .connect(&|_| {})
                .item_control_attrs(&key("standard"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            disabled
                .connect(&|_| {})
                .item_control_attrs(&key("standard"))
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        drop(stale.send(Event::FocusItem {
            item: key("missing"),
            is_keyboard: true,
        }));

        assert_eq!(stale.state(), &State::Idle);
    }

    #[test]
    fn radio_group_hidden_input_attrs_cover_form_state() {
        let service = service(props().default_value(key("express")).required(true));

        let attrs = service
            .connect(&|_| {})
            .item_hidden_input_attrs(&key("express"));

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("radio"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("shipping_method"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("checkout"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("express"));
        assert!(attrs.contains(&HtmlAttr::Checked));
        assert!(attrs.contains(&HtmlAttr::Required));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn radio_group_part_attrs_delegate_for_all_parts() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::Item {
                item_value: key("standard")
            }),
            api.item_attrs(&key("standard"))
        );
        assert_eq!(
            api.part_attrs(Part::ItemControl {
                item_value: key("standard")
            }),
            api.item_control_attrs(&key("standard"))
        );
        assert_eq!(
            api.part_attrs(Part::ItemIndicator {
                item_value: key("standard")
            }),
            api.item_indicator_attrs(&key("standard"))
        );
        assert_eq!(
            api.part_attrs(Part::ItemLabel {
                item_value: key("standard")
            }),
            api.item_label_attrs(&key("standard"))
        );
        assert_eq!(
            api.part_attrs(Part::ItemHiddenInput {
                item_value: key("standard")
            }),
            api.item_hidden_input_attrs(&key("standard"))
        );
    }

    #[test]
    fn radio_group_event_helpers_send_expected_events() {
        let service = service(
            props()
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl),
        );

        let events = Rc::new(RefCell::new(Vec::new()));

        let sent = Rc::clone(&events);
        let send = move |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_item_control_click(&key("standard"));
        api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::Space));
        api.on_item_control_keydown(&key("standard"), &repeated_keyboard(KeyboardKey::Space));
        api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowRight));
        api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowLeft));
        api.on_item_control_focus(&key("standard"), true);
        api.on_item_control_blur();
        api.on_item_mount(Radio::new(key("overnight")));
        api.on_item_unmount(&key("overnight"));
        api.on_form_reset();

        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::SelectValue(key("standard")),
                Event::SelectValue(key("standard")),
                Event::FocusPrev,
                Event::FocusNext,
                Event::FocusItem {
                    item: key("standard"),
                    is_keyboard: true
                },
                Event::Blur,
                Event::RegisterItem(Radio::new(key("overnight"))),
                Event::UnregisterItem(key("overnight")),
                Event::Reset,
            ]
        );
    }

    #[test]
    fn radio_group_keydown_helpers_honor_orientation_repeat_and_home_end() {
        let horizontal = service(props().orientation(Orientation::Horizontal));

        let horizontal_events = Rc::new(RefCell::new(Vec::new()));

        let horizontal_sent = Rc::clone(&horizontal_events);
        let horizontal_send = move |event| horizontal_sent.borrow_mut().push(event);

        let horizontal_api = horizontal.connect(&horizontal_send);

        horizontal_api
            .on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowRight));
        horizontal_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowLeft));
        horizontal_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowDown));
        horizontal_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowUp));
        horizontal_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::Home));
        horizontal_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::End));
        horizontal_api
            .on_item_control_keydown(&key("standard"), &repeated_keyboard(KeyboardKey::Enter));

        assert_eq!(
            horizontal_events.borrow().as_slice(),
            &[
                Event::FocusNext,
                Event::FocusPrev,
                Event::FocusFirst,
                Event::FocusLast,
            ]
        );

        let vertical = service(props().orientation(Orientation::Vertical));

        let vertical_events = Rc::new(RefCell::new(Vec::new()));

        let vertical_sent = Rc::clone(&vertical_events);
        let vertical_send = move |event| vertical_sent.borrow_mut().push(event);

        let vertical_api = vertical.connect(&vertical_send);

        vertical_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowRight));
        vertical_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowLeft));
        vertical_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowDown));
        vertical_api.on_item_control_keydown(&key("standard"), &keyboard(KeyboardKey::ArrowUp));

        assert_eq!(
            vertical_events.borrow().as_slice(),
            &[Event::FocusNext, Event::FocusPrev]
        );
    }

    #[test]
    fn radio_group_api_debug_is_stable() {
        let service = service(props());

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("Api"));
        assert!(debug.contains("shipping"));
        assert!(debug.contains("<callback>"));
    }

    #[test]
    fn radio_group_root_idle_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_root_idle",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn radio_group_root_invalid_description_snapshot() {
        let mut service = service(
            props()
                .invalid(true)
                .required(true)
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl),
        );

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetHasErrorMessage(true)));

        assert_snapshot!(
            "radio_group_root_invalid_description",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn radio_group_item_unselected_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_item_unselected",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(&key("standard")))
        );
    }

    #[test]
    fn radio_group_item_selected_focus_visible_snapshot() {
        let mut service = service(props().default_value(key("standard")));

        drop(service.send(Event::FocusItem {
            item: key("standard"),
            is_keyboard: true,
        }));

        assert_snapshot!(
            "radio_group_item_selected_focus_visible",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(&key("standard")))
        );
    }

    #[test]
    fn radio_group_item_disabled_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_item_disabled",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(&key("drone")))
        );
    }

    #[test]
    fn radio_group_item_control_selected_snapshot() {
        let service = service(props().default_value(key("express")).required(true));

        assert_snapshot!(
            "radio_group_item_control_selected",
            snapshot_attrs(&service.connect(&|_| {}).item_control_attrs(&key("express")))
        );
    }

    #[test]
    fn radio_group_item_control_disabled_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_item_control_disabled",
            snapshot_attrs(&service.connect(&|_| {}).item_control_attrs(&key("drone")))
        );
    }

    #[test]
    fn radio_group_item_indicator_snapshot() {
        let service = service(props().default_value(key("standard")));

        assert_snapshot!(
            "radio_group_item_indicator",
            snapshot_attrs(
                &service
                    .connect(&|_| {})
                    .item_indicator_attrs(&key("standard"))
            )
        );
    }

    #[test]
    fn radio_group_item_label_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_item_label",
            snapshot_attrs(&service.connect(&|_| {}).item_label_attrs(&key("standard")))
        );
    }

    #[test]
    fn radio_group_item_hidden_input_snapshot() {
        let service = service(props().default_value(key("express")).required(true));

        assert_snapshot!(
            "radio_group_item_hidden_input",
            snapshot_attrs(
                &service
                    .connect(&|_| {})
                    .item_hidden_input_attrs(&key("express"))
            )
        );
    }

    #[test]
    fn radio_group_item_hidden_input_disabled_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "radio_group_item_hidden_input_disabled",
            snapshot_attrs(
                &service
                    .connect(&|_| {})
                    .item_hidden_input_attrs(&key("drone"))
            )
        );
    }

    #[test]
    fn radio_group_label_description_error_snapshots() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!("radio_group_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!(
            "radio_group_description",
            snapshot_attrs(&api.description_attrs())
        );
        assert_snapshot!(
            "radio_group_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
    }

    trait SendResultExt {
        fn is_noop(&self) -> bool;
    }

    impl SendResultExt for ars_core::SendResult<Machine> {
        fn is_noop(&self) -> bool {
            !self.state_changed && !self.context_changed && self.pending_effects.is_empty()
        }
    }
}
