//! CheckboxGroup component state machine and connect API.
//!
//! This module implements the framework-agnostic `CheckboxGroup` machine
//! defined in `spec/components/input/checkbox-group.md`. The machine owns a
//! `BTreeSet<Key>` checked-value set, group-level accessibility state, select-all
//! helpers, child checkbox context, and native form-submission metadata.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, Orientation, PendingEffect, TransitionPlan, no_cleanup,
};

use super::checkbox;

/// The state of the `CheckboxGroup` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The group has no active state beyond its context.
    #[default]
    Idle,
}

/// Events accepted by the `CheckboxGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Toggle a checkbox value.
    Toggle(Key),

    /// Set a checkbox value to checked.
    Check(Key),

    /// Set a checkbox value to unchecked.
    Uncheck(Key),

    /// Replace the complete checked-value set.
    SetValue(BTreeSet<Key>),

    /// Check all values from [`Props::all_values`], truncated by `max_checked`.
    CheckAll,

    /// Uncheck all values.
    UncheckAll,

    /// Restore the checked-value set to [`Props::default_value`].
    Reset,

    /// Focus entered the group.
    Focus {
        /// True when focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// Focus left the group.
    Blur,

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether a description part is rendered.
    SetHasDescription(bool),

    /// Track whether an error-message part is rendered.
    SetHasErrorMessage(bool),
}

/// Context for the `CheckboxGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Checked values, controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,

    /// Shared form field name for all child checkboxes.
    pub name: Option<String>,

    /// Whether the group is disabled.
    pub disabled: bool,

    /// Whether at least one checkbox must be checked by validation.
    pub required: bool,

    /// Whether the checked-value set is read-only.
    pub readonly: bool,

    /// Whether the group is in an invalid state.
    pub invalid: bool,

    /// Text direction for root layout and bidirectional content.
    pub dir: Direction,

    /// Layout axis communicated through `aria-orientation`.
    pub orientation: Orientation,

    /// Maximum number of checked values allowed at once.
    pub max_checked: Option<usize>,

    /// Whether focus is currently inside the group.
    pub focused: bool,

    /// True when focus was initiated by keyboard navigation.
    pub focus_visible: bool,

    /// Whether a description part is rendered and should be referenced.
    pub has_description: bool,

    /// Whether an error message part is rendered and should be referenced.
    pub has_error_message: bool,

    /// Stable IDs for checkbox-group anatomy parts.
    pub ids: ComponentIds,
}

impl Context {
    /// Returns the checked state for a parent checkbox representing all values.
    #[must_use]
    pub fn parent_checked_state(&self, all_values: &BTreeSet<Key>) -> checkbox::State {
        if all_values.is_empty() {
            return checkbox::State::Unchecked;
        }

        let checked_count = all_values
            .iter()
            .filter(|value| self.value.get().contains(*value))
            .count();

        if checked_count == 0 {
            checkbox::State::Unchecked
        } else if checked_count == all_values.len() {
            checkbox::State::Checked
        } else {
            checkbox::State::Indeterminate
        }
    }

    /// Returns whether a specific value is currently checked.
    #[must_use]
    pub fn is_checked(&self, value: &Key) -> bool {
        self.value.get().contains(value)
    }

    /// Returns true when the maximum number of checked values has been reached.
    #[must_use]
    pub fn is_at_max(&self) -> bool {
        self.max_checked
            .is_some_and(|max| self.value.get().len() >= max)
    }
}

/// Props for the `CheckboxGroup` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled checked values. When `Some`, the component is controlled.
    pub value: Option<BTreeSet<Key>>,

    /// Default checked values for uncontrolled mode.
    pub default_value: BTreeSet<Key>,

    /// Shared form field name for all child checkboxes.
    pub name: Option<String>,

    /// ID of the associated native form element.
    pub form: Option<String>,

    /// Whether the group is disabled.
    pub disabled: bool,

    /// Whether at least one checkbox must be checked by validation.
    pub required: bool,

    /// Whether the checked-value set is read-only.
    pub readonly: bool,

    /// Whether the group is in an invalid state.
    pub invalid: bool,

    /// Text direction for root layout and bidirectional content.
    pub dir: Direction,

    /// Layout axis communicated through `aria-orientation`.
    pub orientation: Orientation,

    /// Complete known value set used by select-all helpers.
    pub all_values: BTreeSet<Key>,

    /// Maximum number of checked values allowed at once.
    pub max_checked: Option<usize>,

    /// Called when user intent requests a new checked-value set.
    pub on_change: Option<Callback<dyn Fn(BTreeSet<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            name: None,
            form: None,
            disabled: false,
            required: false,
            readonly: false,
            invalid: false,
            dir: Direction::Ltr,
            orientation: Orientation::Vertical,
            all_values: BTreeSet::new(),
            max_checked: None,
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

    /// Sets [`id`](Self::id), the component instance ID.
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

    /// Sets [`default_value`](Self::default_value) for uncontrolled mode.
    #[must_use]
    pub fn default_value(mut self, value: BTreeSet<Key>) -> Self {
        self.default_value = value;
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

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
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

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = dir;
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets [`all_values`](Self::all_values), used by select-all helpers.
    #[must_use]
    pub fn all_values(mut self, values: BTreeSet<Key>) -> Self {
        self.all_values = values;
        self
    }

    /// Sets [`max_checked`](Self::max_checked).
    #[must_use]
    pub const fn max_checked(mut self, max_checked: usize) -> Self {
        self.max_checked = Some(max_checked);
        self
    }

    /// Clears [`max_checked`](Self::max_checked).
    #[must_use]
    pub const fn no_max_checked(mut self) -> Self {
        self.max_checked = None;
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

    /// Clears [`on_change`](Self::on_change).
    #[must_use]
    pub fn no_change(mut self) -> Self {
        self.on_change = None;
        self
    }
}

/// Borrowed view of group context consumed by child Checkbox components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildContext<'a> {
    /// The checked-value set owned by the group.
    pub value: &'a BTreeSet<Key>,

    /// Shared form field name for all child checkboxes.
    pub name: Option<&'a str>,

    /// ID of the associated native form element.
    pub form: Option<&'a str>,

    /// Whether the whole group is disabled.
    pub disabled: bool,

    /// Whether child checked state is read-only.
    pub readonly: bool,

    /// Whether the group is invalid.
    pub invalid: bool,

    /// True when unchecked children should be disabled by `max_checked`.
    pub at_max: bool,
}

impl ChildContext<'_> {
    /// Returns whether `value` is checked by the group.
    #[must_use]
    pub fn is_checked(&self, value: &Key) -> bool {
        self.value.contains(value)
    }

    /// Returns the checkbox state child components should render for `value`.
    #[must_use]
    pub fn checked_state(&self, value: &Key) -> checkbox::State {
        if self.is_checked(value) {
            checkbox::State::Checked
        } else {
            checkbox::State::Unchecked
        }
    }

    /// Returns whether a child with `value` should be disabled by group context.
    #[must_use]
    pub fn is_child_disabled(&self, value: &Key) -> bool {
        self.disabled || (self.at_max && !self.is_checked(value))
    }
}

/// Native hidden checkbox input metadata for group form submission or validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenInputConfig {
    /// Shared input name.
    pub name: String,

    /// Submitted value for this item.
    pub value: String,

    /// Whether this native checkbox should be checked.
    pub checked: bool,

    /// Associated form element ID.
    pub form_id: Option<String>,

    /// Whether the native hidden checkbox should be disabled.
    pub disabled: bool,

    /// Whether the native hidden checkbox should carry `required`.
    pub required: bool,
}

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the checkbox group emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter invokes [`Props::on_change`] with the requested checked-value set.
    ValueChange,
}

/// Machine for the `CheckboxGroup` component.
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
                    Bindable::controlled(clamp_to_max(value.clone(), props.max_checked))
                } else {
                    Bindable::uncontrolled(clamp_to_max(
                        props.default_value.clone(),
                        props.max_checked,
                    ))
                },
                name: props.name.clone(),
                disabled: props.disabled,
                required: props.required,
                readonly: props.readonly,
                invalid: props.invalid,
                dir: props.dir,
                orientation: props.orientation,
                max_checked: props.max_checked,
                focused: false,
                focus_visible: false,
                has_description: false,
                has_error_message: false,
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
        if (ctx.disabled || ctx.readonly)
            && matches!(
                event,
                Event::Toggle(_)
                    | Event::Check(_)
                    | Event::Uncheck(_)
                    | Event::CheckAll
                    | Event::UncheckAll
            )
        {
            return None;
        }

        match event {
            Event::Toggle(value) => {
                let mut next = ctx.value.get().clone();

                if !next.remove(value) {
                    if ctx.is_at_max() {
                        return None;
                    }

                    next.insert(value.clone());
                }

                Some(value_change_plan(ctx, next))
            }

            Event::Check(value) => {
                if ctx.value.get().contains(value) {
                    return Some(TransitionPlan::new());
                }

                if ctx.is_at_max() {
                    return None;
                }

                let mut next = ctx.value.get().clone();

                next.insert(value.clone());

                Some(value_change_plan(ctx, next))
            }

            Event::Uncheck(value) => {
                if !ctx.value.get().contains(value) {
                    return Some(TransitionPlan::new());
                }

                let mut next = ctx.value.get().clone();

                next.remove(value);

                Some(value_change_plan(ctx, next))
            }

            Event::SetValue(value) => {
                let controlled = props
                    .value
                    .clone()
                    .map(|value| clamp_to_max(value, ctx.max_checked));
                let next = controlled
                    .clone()
                    .unwrap_or_else(|| clamp_to_max(value.clone(), ctx.max_checked));

                Some(sync_value_plan(controlled, next))
            }

            Event::CheckAll => {
                let next = clamp_to_max(props.all_values.clone(), ctx.max_checked);
                Some(value_change_plan(ctx, next))
            }

            Event::UncheckAll => Some(value_change_plan(ctx, BTreeSet::new())),

            Event::Reset => {
                let next = clamp_to_max(props.default_value.clone(), ctx.max_checked);
                Some(value_change_plan(ctx, next))
            }

            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            Event::Blur => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            Event::SetProps => {
                let controlled = props.value.clone();
                let name = props.name.clone();
                let disabled = props.disabled;
                let required = props.required;
                let readonly = props.readonly;
                let invalid = props.invalid;
                let dir = props.dir;
                let orientation = props.orientation;
                let max_checked = props.max_checked;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let controlled = controlled.map(|value| clamp_to_max(value, max_checked));

                    if let Some(value) = &controlled {
                        ctx.value.set(value.clone());
                    } else {
                        ctx.value
                            .set(clamp_to_max(ctx.value.get().clone(), max_checked));
                    }

                    ctx.value.sync_controlled(controlled);
                    ctx.name = name;
                    ctx.disabled = disabled;
                    ctx.required = required;
                    ctx.readonly = readonly;
                    ctx.invalid = invalid;
                    ctx.dir = dir;
                    ctx.orientation = orientation;
                    ctx.max_checked = max_checked;
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
            "checkbox_group::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            if let Some(value) = &new.value {
                events.push(Event::SetValue(value.clone()));
            } else {
                events.push(Event::SetProps);
            }
        }

        if old.name != new.name
            || old.disabled != new.disabled
            || old.required != new.required
            || old.readonly != new.readonly
            || old.invalid != new.invalid
            || old.dir != new.dir
            || old.orientation != new.orientation
            || old.max_checked != new.max_checked
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

/// Structural parts exposed by the checkbox-group connect API.
#[derive(ComponentPart)]
#[scope = "checkbox-group"]
pub enum Part {
    /// The root group container.
    Root,

    /// The visible group label.
    Label,

    /// Optional descriptive text for the group.
    Description,

    /// Optional validation error text for the group.
    ErrorMessage,
}

/// API for the `CheckboxGroup` component.
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
        }
    }
}

impl Api<'_> {
    /// Returns attributes for the root group container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.is_at_max() {
            attrs.set_bool(HtmlAttr::Data("ars-at-max"), true);
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

    /// Returns context values child Checkbox components should inherit.
    #[must_use]
    pub fn child_context(&self) -> ChildContext<'_> {
        ChildContext {
            value: self.ctx.value.get(),
            name: self.ctx.name.as_deref(),
            form: self.props.form.as_deref(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            invalid: self.ctx.invalid,
            at_max: self.ctx.is_at_max(),
        }
    }

    /// Returns hidden checkbox input configs for checked values and required validation.
    #[must_use]
    pub fn hidden_input_configs(&self) -> Vec<HiddenInputConfig> {
        let Some(name) = &self.ctx.name else {
            return Vec::new();
        };

        let mut configs = self
            .ctx
            .value
            .get()
            .iter()
            .map(|value| HiddenInputConfig {
                name: name.clone(),
                value: value.to_string(),
                checked: true,
                form_id: self.props.form.clone(),
                disabled: self.ctx.disabled,
                required: self.ctx.required,
            })
            .collect::<Vec<_>>();

        if configs.is_empty() && self.ctx.required {
            configs.push(HiddenInputConfig {
                name: name.clone(),
                value: String::new(),
                checked: false,
                form_id: self.props.form.clone(),
                disabled: self.ctx.disabled,
                required: true,
            });
        }

        configs
    }

    /// Dispatches a value toggle for a child checkbox.
    pub fn on_child_toggle(&self, value: &Key) {
        let child_ctx = self.child_context();

        if !child_ctx.is_child_disabled(value) {
            (self.send)(Event::Toggle(value.clone()));
        }
    }

    /// Dispatches a value check for a child checkbox.
    pub fn on_child_check(&self, value: &Key) {
        let child_ctx = self.child_context();

        if !child_ctx.is_child_disabled(value) {
            (self.send)(Event::Check(value.clone()));
        }
    }

    /// Dispatches a value uncheck for a child checkbox.
    pub fn on_child_uncheck(&self, value: &Key) {
        if !self.ctx.disabled {
            (self.send)(Event::Uncheck(value.clone()));
        }
    }

    /// Dispatches a select-all event.
    pub fn on_check_all(&self) {
        (self.send)(Event::CheckAll);
    }

    /// Dispatches an unselect-all event.
    pub fn on_uncheck_all(&self) {
        (self.send)(Event::UncheckAll);
    }

    /// Dispatches a focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a native form reset event.
    pub fn on_form_reset(&self) {
        (self.send)(Event::Reset);
    }
}

fn value_change_plan(ctx: &Context, next: BTreeSet<Key>) -> TransitionPlan<Machine> {
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

fn sync_value_plan(
    controlled: Option<BTreeSet<Key>>,
    next: BTreeSet<Key>,
) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(next);
        ctx.value.sync_controlled(controlled);
    })
}

fn value_change_effect(next: BTreeSet<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ValueChange, move |_ctx, props: &Props, _send| {
        if let Some(callback) = &props.on_change {
            callback(next);
        }

        no_cleanup()
    })
}

fn clamp_to_max(values: BTreeSet<Key>, max_checked: Option<usize>) -> BTreeSet<Key> {
    max_checked.map_or(values.clone(), |max| values.into_iter().take(max).collect())
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, rc::Rc, sync::Arc, vec};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_core::{ConnectApi, Env, HtmlAttr, Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::from(value)
    }

    fn set(values: &[&str]) -> BTreeSet<Key> {
        values.iter().map(|value| key(value)).collect()
    }

    fn props() -> Props {
        Props::new()
            .id("features")
            .name("features")
            .form("settings")
            .all_values(set(&["alpha", "beta", "gamma"]))
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages)
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn snapshot_configs(configs: &[HiddenInputConfig]) -> String {
        format!("{configs:#?}")
    }

    #[test]
    fn checkbox_group_initial_state_is_idle_with_default_value() {
        let service = service(props().default_value(set(&["alpha"])));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), &set(&["alpha"]));
        assert_eq!(service.context().ids.id(), "features");
        assert_eq!(service.context().name.as_deref(), Some("features"));
        assert!(service.context().is_checked(&key("alpha")));
        assert!(!service.context().is_checked(&key("beta")));
    }

    #[test]
    fn checkbox_group_props_clearers_preserve_other_builder_fields() {
        let cleared = Props::new()
            .id("features")
            .value(set(&["alpha"]))
            .default_value(set(&["beta"]))
            .disabled(true)
            .form("settings")
            .on_change(callback(|_: BTreeSet<Key>| {}))
            .uncontrolled()
            .no_form()
            .no_change();

        assert_eq!(cleared.id, "features");
        assert!(cleared.disabled);
        assert_eq!(cleared.value, None);
        assert_eq!(cleared.default_value, set(&["beta"]));
        assert_eq!(cleared.form, None);
        assert!(cleared.on_change.is_none());
    }

    #[test]
    fn checkbox_group_toggle_check_uncheck_update_uncontrolled_values() {
        let mut service = service(props());

        drop(service.send(Event::Toggle(key("alpha"))));

        assert_eq!(service.context().value.get(), &set(&["alpha"]));

        drop(service.send(Event::Check(key("beta"))));

        assert_eq!(service.context().value.get(), &set(&["alpha", "beta"]));

        drop(service.send(Event::Toggle(key("alpha"))));

        assert_eq!(service.context().value.get(), &set(&["beta"]));

        drop(service.send(Event::Uncheck(key("beta"))));

        assert_eq!(service.context().value.get(), &BTreeSet::new());
    }

    #[test]
    fn checkbox_group_set_value_check_all_uncheck_all_and_reset() {
        let mut service = service(props().default_value(set(&["beta"])).max_checked(2));

        drop(service.send(Event::SetValue(set(&["alpha", "beta", "gamma"]))));

        assert_eq!(service.context().value.get(), &set(&["alpha", "beta"]));

        drop(service.send(Event::UncheckAll));

        assert_eq!(service.context().value.get(), &BTreeSet::new());

        drop(service.send(Event::CheckAll));

        assert_eq!(service.context().value.get(), &set(&["alpha", "beta"]));

        drop(service.send(Event::Reset));

        assert_eq!(service.context().value.get(), &set(&["beta"]));
    }

    #[test]
    fn checkbox_group_controlled_change_emits_without_committing_value() {
        let mut service = service(props().value(set(&["alpha"])));

        let result = service.send(Event::Check(key("beta")));

        assert!(result.context_changed);
        assert_eq!(service.context().value.get(), &set(&["alpha"]));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);
    }

    #[test]
    fn checkbox_group_controlled_value_is_clamped_to_max_checked() {
        let service = service(props().value(set(&["alpha", "beta"])).max_checked(1));

        assert_eq!(service.context().value.get(), &set(&["alpha"]));
        assert_eq!(
            service.connect(&|_| {}).hidden_input_configs(),
            vec![HiddenInputConfig {
                name: "features".to_string(),
                value: "alpha".to_string(),
                checked: true,
                form_id: Some("settings".to_string()),
                disabled: false,
                required: false,
            }]
        );
    }

    #[test]
    fn checkbox_group_on_change_callback_receives_requested_set() {
        let changes = Arc::new(Mutex::new(Vec::new()));

        let captured = Arc::clone(&changes);
        let mut service = service(props().on_change(callback(move |value: BTreeSet<Key>| {
            captured.lock().unwrap().push(value);
        })));

        let mut result = service.send(Event::Check(key("alpha")));

        let effect = result.pending_effects.pop().expect("value-change effect");

        let send: StrongSend<Event> = Arc::new(|_| {});

        drop(effect.run(service.context(), service.props(), send));

        assert_eq!(changes.lock().unwrap().as_slice(), &[set(&["alpha"])]);
    }

    #[test]
    fn checkbox_group_disabled_and_readonly_prevent_value_changes() {
        let mut disabled = service(props().disabled(true));
        let mut readonly = service(props().readonly(true));

        assert!(disabled.send(Event::Check(key("alpha"))).is_noop());
        assert!(readonly.send(Event::Check(key("alpha"))).is_noop());
        assert_eq!(disabled.context().value.get(), &BTreeSet::new());
        assert_eq!(readonly.context().value.get(), &BTreeSet::new());
    }

    #[test]
    fn checkbox_group_set_value_syncs_even_when_disabled_or_readonly() {
        let mut disabled = service(props().disabled(true));
        let mut readonly = service(props().readonly(true));

        drop(disabled.send(Event::SetValue(set(&["alpha"]))));
        drop(readonly.send(Event::SetValue(set(&["beta"]))));

        assert_eq!(disabled.context().value.get(), &set(&["alpha"]));
        assert_eq!(readonly.context().value.get(), &set(&["beta"]));
    }

    #[test]
    fn checkbox_group_reset_restores_defaults_when_disabled_or_readonly() {
        let mut disabled = service(
            props()
                .default_value(set(&["beta"]))
                .disabled(true)
                .max_checked(1),
        );
        let mut readonly = service(props().default_value(set(&["gamma"])).readonly(true));

        disabled.context_mut().value.set(set(&["alpha"]));
        readonly.context_mut().value.set(set(&["alpha"]));

        drop(disabled.send(Event::Reset));
        drop(readonly.send(Event::Reset));

        assert_eq!(disabled.context().value.get(), &set(&["beta"]));
        assert_eq!(readonly.context().value.get(), &set(&["gamma"]));
    }

    #[test]
    fn checkbox_group_max_checked_blocks_additions_but_allows_unchecking() {
        let mut service = service(props().default_value(set(&["alpha"])).max_checked(1));

        assert!(service.send(Event::Check(key("beta"))).is_noop());
        assert_eq!(service.context().value.get(), &set(&["alpha"]));

        drop(service.send(Event::Uncheck(key("alpha"))));

        assert_eq!(service.context().value.get(), &BTreeSet::new());
    }

    #[test]
    fn checkbox_group_parent_checked_state_covers_select_all_states() {
        let all = set(&["alpha", "beta"]);

        let unchecked = service(props());
        let partial = service(props().default_value(set(&["alpha"])));
        let checked = service(props().default_value(all.clone()));

        assert_eq!(
            unchecked.context().parent_checked_state(&all),
            checkbox::State::Unchecked
        );
        assert_eq!(
            partial.context().parent_checked_state(&all),
            checkbox::State::Indeterminate
        );
        assert_eq!(
            checked.context().parent_checked_state(&all),
            checkbox::State::Checked
        );
    }

    #[test]
    fn checkbox_group_child_context_propagates_group_state() {
        let service = service(
            props()
                .default_value(set(&["alpha"]))
                .disabled(true)
                .readonly(true)
                .invalid(true)
                .max_checked(1),
        );

        let api = service.connect(&|_| {});

        let child = api.child_context();

        assert_eq!(child.name, Some("features"));
        assert_eq!(child.form, Some("settings"));
        assert!(child.disabled);
        assert!(child.readonly);
        assert!(child.invalid);
        assert!(child.at_max);
        assert_eq!(child.checked_state(&key("alpha")), checkbox::State::Checked);
        assert_eq!(
            child.checked_state(&key("beta")),
            checkbox::State::Unchecked
        );
        assert!(child.is_child_disabled(&key("beta")));
    }

    #[test]
    fn checkbox_group_hidden_input_configs_are_sorted_by_checked_value() {
        let service = service(
            props()
                .default_value(set(&["gamma", "alpha"]))
                .required(true),
        );

        assert_eq!(
            service.connect(&|_| {}).hidden_input_configs(),
            vec![
                HiddenInputConfig {
                    name: "features".to_string(),
                    value: "alpha".to_string(),
                    checked: true,
                    form_id: Some("settings".to_string()),
                    disabled: false,
                    required: true,
                },
                HiddenInputConfig {
                    name: "features".to_string(),
                    value: "gamma".to_string(),
                    checked: true,
                    form_id: Some("settings".to_string()),
                    disabled: false,
                    required: true,
                },
            ]
        );
    }

    #[test]
    fn checkbox_group_required_empty_value_emits_unchecked_validation_config() {
        let service = service(props().required(true));

        assert_eq!(
            service.connect(&|_| {}).hidden_input_configs(),
            vec![HiddenInputConfig {
                name: "features".to_string(),
                value: String::new(),
                checked: false,
                form_id: Some("settings".to_string()),
                disabled: false,
                required: true,
            }]
        );
    }

    #[test]
    fn checkbox_group_connect_api_emits_group_attrs() {
        let mut service = service(
            props()
                .default_value(set(&["alpha"]))
                .invalid(true)
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl)
                .max_checked(1),
        );

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetHasErrorMessage(true)));
        drop(service.send(Event::Focus { is_keyboard: true }));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("group"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("features-label")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
        assert!(attrs.contains(&HtmlAttr::Data("ars-at-max")));
    }

    #[test]
    fn checkbox_group_root_describedby_requires_rendered_error_message() {
        let service = service(props().invalid(true));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));
    }

    #[test]
    fn checkbox_group_error_message_uses_alert_without_explicit_live() {
        let attrs = service(props()).connect(&|_| {}).error_message_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Live)));
    }

    #[test]
    fn checkbox_group_on_props_changed_syncs_value_and_context_props() {
        let old = props();
        let new = props()
            .value(set(&["alpha"]))
            .disabled(true)
            .required(true)
            .readonly(true)
            .invalid(true)
            .dir(Direction::Rtl)
            .orientation(Orientation::Horizontal)
            .max_checked(1)
            .no_name();

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            vec![Event::SetValue(set(&["alpha"])), Event::SetProps]
        );
    }

    #[test]
    fn checkbox_group_set_props_clamps_uncontrolled_value_when_max_checked_shrinks() {
        let mut service = service(props().default_value(set(&["alpha", "beta", "gamma"])));

        drop(service.send(Event::SetProps));

        assert_eq!(
            service.context().value.get(),
            &set(&["alpha", "beta", "gamma"])
        );

        drop(service.set_props(props().max_checked(1)));

        assert_eq!(service.context().value.get(), &set(&["alpha"]));
    }

    #[test]
    fn checkbox_group_set_props_clamps_controlled_value_when_max_checked_shrinks() {
        let mut service = service(props().value(set(&["alpha", "beta", "gamma"])));

        assert_eq!(
            service.context().value.get(),
            &set(&["alpha", "beta", "gamma"])
        );

        drop(
            service.set_props(
                props()
                    .value(set(&["alpha", "beta", "gamma"]))
                    .max_checked(1),
            ),
        );

        assert_eq!(service.context().value.get(), &set(&["alpha"]));
        assert_eq!(
            service.connect(&|_| {}).hidden_input_configs(),
            vec![HiddenInputConfig {
                name: "features".to_string(),
                value: "alpha".to_string(),
                checked: true,
                form_id: Some("settings".to_string()),
                disabled: false,
                required: false,
            }]
        );
    }

    #[test]
    fn checkbox_group_on_props_changed_emits_set_props_for_each_context_field() {
        let cases = [
            props().no_name(),
            props().disabled(true),
            props().required(true),
            props().readonly(true),
            props().invalid(true),
            props().dir(Direction::Rtl),
            props().orientation(Orientation::Horizontal),
            props().max_checked(1),
        ];

        for new in cases {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&props(), &new),
                vec![Event::SetProps]
            );
        }
    }

    #[test]
    fn checkbox_group_part_attrs_delegate_for_all_parts() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
    }

    #[test]
    fn checkbox_group_event_helpers_send_expected_events() {
        let service = service(props().default_value(set(&["alpha"])).max_checked(1));

        let events = Rc::new(RefCell::new(Vec::new()));
        let sent = Rc::clone(&events);
        let send = move |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_child_toggle(&key("alpha"));
        api.on_child_check(&key("beta"));
        api.on_child_uncheck(&key("alpha"));
        api.on_check_all();
        api.on_uncheck_all();
        api.on_focus(true);
        api.on_blur();
        api.on_form_reset();

        assert_eq!(
            events.borrow().as_slice(),
            &[
                Event::Toggle(key("alpha")),
                Event::Uncheck(key("alpha")),
                Event::CheckAll,
                Event::UncheckAll,
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Reset,
            ]
        );
    }

    #[test]
    fn checkbox_group_on_child_check_dispatches_when_not_at_max() {
        let service = service(props().default_value(set(&["alpha"])));

        let events = Rc::new(RefCell::new(Vec::new()));
        let sent = Rc::clone(&events);
        let send = move |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_child_check(&key("beta"));

        assert_eq!(events.borrow().as_slice(), &[Event::Check(key("beta"))]);
    }

    #[test]
    fn checkbox_group_api_debug_is_stable() {
        let service = service(props());

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("Api"));
        assert!(debug.contains("features"));
        assert!(debug.contains("<callback>"));
    }

    #[test]
    fn checkbox_group_root_idle_snapshot() {
        let service = service(props());

        assert_snapshot!(
            "checkbox_group_root_idle",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn checkbox_group_root_invalid_description_error_snapshot() {
        let mut service = service(
            props()
                .default_value(set(&["alpha"]))
                .invalid(true)
                .disabled(true)
                .orientation(Orientation::Horizontal)
                .dir(Direction::Rtl)
                .max_checked(1),
        );

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetHasErrorMessage(true)));
        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "checkbox_group_root_invalid_description_error",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn checkbox_group_label_description_error_snapshots() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!("checkbox_group_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!(
            "checkbox_group_description",
            snapshot_attrs(&api.description_attrs())
        );
        assert_snapshot!(
            "checkbox_group_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
    }

    #[test]
    fn checkbox_group_hidden_input_configs_snapshot() {
        let service = service(
            props()
                .default_value(set(&["gamma", "alpha"]))
                .required(true),
        );

        assert_snapshot!(
            "checkbox_group_hidden_input_configs",
            snapshot_configs(&service.connect(&|_| {}).hidden_input_configs())
        );
    }

    #[test]
    fn checkbox_group_hidden_input_configs_absent_without_name_snapshot() {
        let service = service(
            props()
                .default_value(set(&["alpha"]))
                .required(true)
                .no_name(),
        );

        assert_snapshot!(
            "checkbox_group_hidden_input_configs_absent_without_name",
            snapshot_configs(&service.connect(&|_| {}).hidden_input_configs())
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
