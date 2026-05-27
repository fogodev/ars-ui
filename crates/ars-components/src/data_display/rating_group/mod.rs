//! RatingGroup data-display component machine.
//!
//! Owns committed rating value, hover preview, keyboard focus intent, and
//! form/ARIA attributes. Adapters render the visual star icons and resolve
//! pointer geometry for fractional hit testing.

use alloc::{
    format,
    string::{String, ToString as _},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    num::NonZero,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentMessages, ComponentPart, ConnectApi, Env, HasId,
    HtmlAttr, Locale, MessageFn, TransitionPlan, no_cleanup,
};
use ars_i18n::{Plural, format_plural};
use ars_interactions::{KeyboardEventData, KeyboardKey};

type ItemLabelFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

/// Props for the `RatingGroup` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Visible label text rendered by adapters through the label part.
    pub label: Option<String>,

    /// Controlled committed rating value.
    pub value: Option<f64>,

    /// Uncontrolled initial rating value.
    pub default_value: f64,

    /// Number of rating items.
    pub count: NonZero<u32>,

    /// Whether to use half-step rating behavior when `step` is still the
    /// default whole-step value.
    pub allow_half: bool,

    /// Rating increment used by keyboard navigation and value rounding.
    pub step: f64,

    /// Read-only display mode.
    pub readonly: bool,

    /// Disabled display mode.
    pub disabled: bool,

    /// Whether the hidden form value is required.
    pub required: bool,

    /// Hidden input name used for native form submission.
    pub name: Option<String>,

    /// Form owner ID for the hidden input.
    pub form: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: None,
            value: None,
            default_value: 0.0,
            count: NonZero::new(5).expect("rating count is non-zero"),
            allow_half: false,
            step: 1.0,
            readonly: false,
            disabled: false,
            required: false,
            name: None,
            form: None,
        }
    }
}

impl Props {
    /// Returns fresh `RatingGroup` props with documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the visible label text rendered by adapters.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the controlled committed value.
    #[must_use]
    pub const fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the uncontrolled initial value.
    #[must_use]
    pub const fn default_value(mut self, value: f64) -> Self {
        self.default_value = value;
        self
    }

    /// Sets the number of rating items.
    #[must_use]
    pub const fn count(mut self, count: NonZero<u32>) -> Self {
        self.count = count;
        self
    }

    /// Enables or disables half-step behavior.
    #[must_use]
    pub fn allow_half(mut self, allow_half: bool) -> Self {
        self.allow_half = allow_half;
        if allow_half && (self.step - 1.0).abs() <= f64::EPSILON {
            self.step = 0.5;
        }
        self
    }

    /// Sets the rating increment.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        if step.is_finite() && step > 0.0 {
            self.allow_half = false;
        }
        self
    }

    /// Sets read-only mode.
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets disabled mode.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the hidden input is required.
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets the hidden input name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the hidden input form owner.
    #[must_use]
    pub fn form(mut self, form: impl Into<String>) -> Self {
        self.form = Some(form.into());
        self
    }
}

/// States for the `RatingGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No hover or keyboard focus is active.
    Idle,

    /// Keyboard or pointer focus is on the item at `index`.
    Focused {
        /// Zero-based focused item index.
        index: usize,
    },

    /// Pointer is hovering over the item at `index`.
    Hovering {
        /// Zero-based hovered item index.
        index: usize,
    },
}

/// Events accepted by the `RatingGroup` machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Commit a rating value.
    Rate(f64),

    /// Synchronize props mirrored into context.
    SyncProps,

    /// Pointer entered a rating item.
    HoverItem(usize),

    /// Pointer previewed a resolved rating value.
    HoverValue(f64),

    /// Pointer left the rating control.
    UnHover,

    /// Focus moved to a rating item.
    Focus {
        /// Zero-based focused item index.
        index: usize,

        /// Whether focus was keyboard initiated.
        is_keyboard: bool,
    },

    /// Focus left the rating control.
    Blur,

    /// Increase the committed rating.
    IncrementRating,

    /// Decrease the committed rating.
    DecrementRating,

    /// Clear the committed rating back to zero.
    ClearRating,
}

/// Context for the `RatingGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Committed rating value.
    pub value: Bindable<f64>,

    /// Hover preview value.
    pub hovered_value: Option<f64>,

    /// Latest user-requested committed value for adapter change notification.
    pub requested_value: Option<f64>,

    /// Focused item index.
    pub focused_index: Option<usize>,

    /// Whether the focus ring should be visible.
    pub focus_visible: bool,

    /// Number of rating items.
    pub count: NonZero<u32>,

    /// Whether half-step behavior is enabled.
    pub allow_half: bool,

    /// Read-only display mode.
    pub readonly: bool,

    /// Disabled display mode.
    pub disabled: bool,

    /// Active locale for message formatting.
    pub locale: Locale,

    /// Resolved messages for the rating group.
    pub messages: Messages,
}

/// Messages for the `RatingGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Returns the label for a rating value.
    pub item_label: MessageFn<ItemLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            item_label: MessageFn::new(Arc::new(|value: f64, locale: &Locale| {
                let value_text = value.to_string();
                let mut labels = Plural::from_other("{value} stars");

                labels.one = Some("{value} star");

                format_plural(locale, value, &labels, &[("value", &value_text)])
            }) as Arc<ItemLabelFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed side-effect intents emitted by the `RatingGroup` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Notify adapters that the user requested a new committed value.
    ValueChange,
}

/// Structural parts exposed by the `RatingGroup` connect API.
#[derive(ComponentPart)]
#[scope = "rating-group"]
pub enum Part {
    /// The root rating group element.
    Root,

    /// The visible label element.
    Label,

    /// The interactive rating control.
    Control,

    /// One rating item slot.
    Item {
        /// Zero-based item index.
        index: usize,
    },

    /// Hidden input used for native form submission.
    HiddenInput,
}

/// Machine for the `RatingGroup` component.
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
        let value = sanitize_value(props.value.unwrap_or(props.default_value), props.count);
        let value = round_to_step(value, effective_step(props), props.count);

        (
            State::Idle,
            Context {
                value: if props.value.is_some() {
                    Bindable::controlled(value)
                } else {
                    Bindable::uncontrolled(value)
                },
                hovered_value: None,
                requested_value: None,
                focused_index: None,
                focus_visible: false,
                count: props.count,
                allow_half: props.allow_half,
                readonly: props.readonly,
                disabled: props.disabled,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if let Event::SyncProps = event {
            let count = props.count;
            let allow_half = props.allow_half;
            let readonly = props.readonly;
            let disabled = props.disabled;
            let value = sync_props_value(context, props);
            let controlled_value = props.value.map(|_| value);
            let focused_index = context
                .focused_index
                .filter(|_| !disabled)
                .map(|index| clamp_index(index, count));
            let target = focused_index.map_or(State::Idle, |index| State::Focused { index });

            return Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                ctx.value.sync_controlled(controlled_value);

                if controlled_value.is_none() {
                    ctx.value.set(value);
                }

                ctx.hovered_value = None;
                ctx.requested_value = None;
                ctx.focused_index = focused_index;
                ctx.focus_visible = ctx.focused_index.is_some() && ctx.focus_visible;
                ctx.count = count;
                ctx.allow_half = allow_half;
                ctx.readonly = readonly;
                ctx.disabled = disabled;
            }));
        }

        if context.disabled {
            return match event {
                Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_index = None;
                    ctx.focus_visible = false;
                    ctx.hovered_value = None;
                })),

                _ => None,
            };
        }

        if context.readonly {
            return match event {
                Event::Focus { index, is_keyboard } => {
                    let index = clamp_index(*index, context.count);
                    let is_keyboard = *is_keyboard;

                    Some(TransitionPlan::to(State::Focused { index }).apply(
                        move |ctx: &mut Context| {
                            ctx.focused_index = Some(index);
                            ctx.focus_visible = is_keyboard;
                        },
                    ))
                }

                Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_index = None;
                    ctx.focus_visible = false;
                    ctx.hovered_value = None;
                })),

                _ => None,
            };
        }

        match event {
            Event::Rate(value) => rate_plan(
                *value,
                context,
                props,
                keyboard_focus_index(*value, context, props),
            ),

            Event::HoverItem(index) => {
                let index = clamp_index(*index, context.count);
                let value = (index + 1) as f64;

                Some(TransitionPlan::to(State::Hovering { index }).apply(
                    move |ctx: &mut Context| {
                        ctx.hovered_value = Some(value);
                    },
                ))
            }

            Event::HoverValue(value) => {
                let value = round_to_step(
                    sanitize_value(*value, context.count),
                    effective_step(props),
                    context.count,
                );
                let index = value_to_focus_index(value, context.count);

                Some(TransitionPlan::to(State::Hovering { index }).apply(
                    move |ctx: &mut Context| {
                        ctx.hovered_value = Some(value);
                    },
                ))
            }

            Event::UnHover => {
                let target = context
                    .focused_index
                    .map_or(State::Idle, |index| State::Focused { index });

                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.hovered_value = None;
                }))
            }

            Event::Focus { index, is_keyboard } => {
                let index = clamp_index(*index, context.count);
                let is_keyboard = *is_keyboard;

                Some(TransitionPlan::to(State::Focused { index }).apply(
                    move |ctx: &mut Context| {
                        ctx.focused_index = Some(index);
                        ctx.focus_visible = is_keyboard;
                    },
                ))
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused_index = None;
                ctx.focus_visible = false;
                ctx.hovered_value = None;
            })),

            Event::IncrementRating => {
                let value =
                    (pending_value(context) + effective_step(props)).min(max_value(context.count));

                rate_plan(
                    value,
                    context,
                    props,
                    keyboard_focus_index(value, context, props),
                )
            }

            Event::DecrementRating => {
                let value = (pending_value(context) - effective_step(props)).max(0.0);

                rate_plan(
                    value,
                    context,
                    props,
                    keyboard_focus_index(value, context, props),
                )
            }

            Event::ClearRating => rate_plan(
                0.0,
                context,
                props,
                keyboard_focus_index(0.0, context, props),
            ),

            Event::SyncProps => unreachable!("SyncProps handled before interactivity guards"),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            context,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.value != new.value
            || old.id != new.id
            || old.label != new.label
            || old.count != new.count
            || old.allow_half != new.allow_half
            || old.step != new.step
            || old.readonly != new.readonly
            || old.disabled != new.disabled
            || old.required != new.required
            || old.name != new.name
            || old.form != new.form
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// API for the `RatingGroup` component.
pub struct Api<'a> {
    state: &'a State,
    context: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("rating_group::Api")
            .field("state", self.state)
            .field("context", self.context)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns the current visual rating value, preferring hover preview.
    #[must_use]
    pub fn display_value(&self) -> f64 {
        self.context
            .hovered_value
            .unwrap_or_else(|| *self.context.value.get())
    }

    /// Returns the latest user-requested committed value.
    #[must_use]
    pub const fn requested_value(&self) -> Option<f64> {
        self.context.requested_value
    }

    /// Returns whether the item at `index` is visually highlighted.
    #[must_use]
    pub fn is_item_highlighted(&self, index: usize) -> bool {
        self.display_value() >= (index + 1) as f64
    }

    /// Returns whether the item at `index` is selected by the committed value.
    #[must_use]
    pub fn is_item_selected(&self, index: usize) -> bool {
        *self.context.value.get() >= (index + 1) as f64
    }

    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs.set(HtmlAttr::Id, &self.props.id);

        if self.context.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.context.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Returns attributes for the visible label.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Label);

        attrs.set(HtmlAttr::Id, format!("{}-label", self.props.id));

        attrs
    }

    /// Returns attributes for the interactive control.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Control);

        attrs.set(HtmlAttr::Id, format!("{}-control", self.props.id));

        if self.props.label.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                format!("{}-label", self.props.id),
            );
        }

        if self.uses_slider_pattern() {
            attrs
                .set(HtmlAttr::Role, "slider")
                .set(HtmlAttr::Aria(AriaAttr::ValueMin), "0")
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueMax),
                    self.context.count.get().to_string(),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueNow),
                    self.context.value.get().to_string(),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::ValueText),
                    rating_value_text(
                        *self.context.value.get(),
                        self.context.count.get(),
                        &self.context.messages,
                        &self.context.locale,
                    ),
                )
                .set(
                    HtmlAttr::TabIndex,
                    if self.context.disabled { "-1" } else { "0" },
                );
        } else {
            attrs.set(HtmlAttr::Role, "radiogroup");
        }

        if self.context.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.context.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        attrs
    }

    /// Returns attributes for the rating item at `index`.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = part_attrs(&Part::Item { index });

        let item_value = (index + 1) as f64;
        let selected = self.is_item_selected(index);
        let highlighted = self.is_item_highlighted(index);

        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());

        if highlighted {
            attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true);
        }

        if selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        if let Some(fraction) = item_fraction(self.display_value(), index) {
            attrs.set(HtmlAttr::Data("ars-fraction"), fraction.to_string());

            if (fraction - 0.5).abs() <= f64::EPSILON {
                attrs.set_bool(HtmlAttr::Data("ars-half"), true);
            }
        }

        if !self.uses_slider_pattern() {
            attrs
                .set(HtmlAttr::Role, "radio")
                .set(
                    HtmlAttr::Aria(AriaAttr::Label),
                    (self.context.messages.item_label)(item_value, &self.context.locale),
                )
                .set(
                    HtmlAttr::Aria(AriaAttr::Checked),
                    if self.is_item_checked(index) {
                        "true"
                    } else {
                        "false"
                    },
                )
                .set(
                    HtmlAttr::TabIndex,
                    if self.context.disabled {
                        "-1"
                    } else if self.is_item_tabbable(index) {
                        "0"
                    } else {
                        "-1"
                    },
                );
        } else if self.context.disabled {
            attrs.set(HtmlAttr::TabIndex, "-1");
        }

        if self.context.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.context.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Returns attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::HiddenInput);

        attrs.set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }

        attrs.set(HtmlAttr::Value, self.context.value.get().to_string());

        if self.context.disabled {
            attrs.set(HtmlAttr::Disabled, "true");
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }

        attrs
    }

    /// Dispatches the control keydown intent.
    pub fn on_control_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => (self.send)(Event::IncrementRating),

            KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => {
                (self.send)(Event::DecrementRating);
            }

            KeyboardKey::Home => (self.send)(Event::ClearRating),

            KeyboardKey::End => (self.send)(Event::Rate(max_value(self.context.count))),

            _ => {}
        }
    }

    /// Dispatches the item keydown intent.
    pub fn on_item_keydown(&self, _index: usize, data: &KeyboardEventData) {
        self.on_control_keydown(data);
    }

    /// Dispatches focus for a rating item.
    pub fn on_item_focus(&self, index: usize, is_keyboard: bool) {
        (self.send)(Event::Focus { index, is_keyboard });
    }

    /// Dispatches blur from a rating item.
    pub fn on_item_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches pointer hover for a rating item.
    pub fn on_item_hover(&self, index: usize) {
        (self.send)(Event::HoverItem(index));
    }

    /// Dispatches pointer hover for a resolved rating value.
    pub fn on_value_hover(&self, value: f64) {
        (self.send)(Event::HoverValue(value));
    }

    /// Dispatches pointer leave from the rating control.
    pub fn on_control_mouse_leave(&self) {
        (self.send)(Event::UnHover);
    }

    /// Dispatches a whole-item rating commit.
    pub fn on_item_rate(&self, index: usize) {
        (self.send)(Event::Rate((index + 1) as f64));
    }

    /// Dispatches a resolved rating-value commit.
    pub fn on_value_rate(&self, value: f64) {
        (self.send)(Event::Rate(value));
    }

    fn is_item_tabbable(&self, index: usize) -> bool {
        let value = *self.context.value.get();

        if value == 0.0 {
            return index == 0;
        }

        ((index + 1) as f64 - value).abs() < f64::EPSILON
    }

    fn is_item_checked(&self, index: usize) -> bool {
        ((index + 1) as f64 - *self.context.value.get()).abs() < f64::EPSILON
    }

    fn uses_slider_pattern(&self) -> bool {
        effective_step(self.props).fract().abs() > f64::EPSILON
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_value)
        .set(part_attr, part_value);

    attrs
}

fn rating_value_text(value: f64, count: u32, messages: &Messages, locale: &Locale) -> String {
    format!(
        "{} of {}",
        value,
        (messages.item_label)(f64::from(count), locale)
    )
}

fn effective_step(props: &Props) -> f64 {
    if props.step.is_finite() && props.step > 0.0 {
        props.step
    } else if props.allow_half {
        0.5
    } else {
        1.0
    }
}

fn sanitize_value(value: f64, count: NonZero<u32>) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, max_value(count))
    } else {
        0.0
    }
}

fn max_value(count: NonZero<u32>) -> f64 {
    f64::from(count.get())
}

fn round_to_step(value: f64, step: f64, count: NonZero<u32>) -> f64 {
    let value = sanitize_value(value, count);

    if (value - max_value(count)).abs() < f64::EPSILON {
        value
    } else {
        sanitize_value((value / step).round() * step, count)
    }
}

fn rate_plan(
    value: f64,
    context: &Context,
    props: &Props,
    focused_index: Option<usize>,
) -> Option<TransitionPlan<Machine>> {
    let value = round_to_step(
        sanitize_value(value, context.count),
        effective_step(props),
        context.count,
    );
    if (value - *context.value.get()).abs() < f64::EPSILON {
        if context.focused_index != focused_index {
            let target = focused_index.map_or(State::Idle, |index| State::Focused { index });

            return Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                ctx.hovered_value = None;
                ctx.focused_index = focused_index;
            }));
        }

        return None;
    }

    let target = focused_index.map_or(State::Idle, |index| State::Focused { index });

    Some(
        TransitionPlan::to(target)
            .apply(move |ctx: &mut Context| {
                ctx.value.set(value);
                ctx.requested_value = Some(value);
                ctx.hovered_value = None;

                if let Some(index) = focused_index {
                    ctx.focused_index = Some(index);
                }
            })
            .with_named_effect(Effect::ValueChange, |_ctx, _props, _send| no_cleanup()),
    )
}

fn pending_value(context: &Context) -> f64 {
    context
        .requested_value
        .unwrap_or_else(|| *context.value.get())
}

fn keyboard_focus_index(value: f64, context: &Context, props: &Props) -> Option<usize> {
    if context.focused_index.is_some() && effective_step(props).fract().abs() <= f64::EPSILON {
        Some(value_to_focus_index(value, context.count))
    } else {
        context.focused_index
    }
}

fn value_to_focus_index(value: f64, count: NonZero<u32>) -> usize {
    if value <= 0.0 {
        0
    } else {
        clamp_index(value.ceil() as usize - 1, count)
    }
}

fn sync_props_value(context: &Context, props: &Props) -> f64 {
    let source = props.value.unwrap_or_else(|| {
        context
            .requested_value
            .unwrap_or_else(|| *context.value.get())
    });

    round_to_step(
        sanitize_value(source, props.count),
        effective_step(props),
        props.count,
    )
}

fn clamp_index(index: usize, count: NonZero<u32>) -> usize {
    index.min(count.get().saturating_sub(1) as usize)
}

fn item_fraction(value: f64, index: usize) -> Option<f64> {
    let lower = index as f64;
    let upper = (index + 1) as f64;

    if value > lower && value < upper {
        Some(value - lower)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec::Vec};
    use core::{cell::RefCell, num::NonZero};

    use ars_core::{AttrMap, Env, Machine as _, Service};
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    const fn keyboard(key: KeyboardKey) -> KeyboardEventData {
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

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn whole_mode_attrs_and_keyboard_commit_values() {
        let mut service = service(
            Props::new()
                .id("rating")
                .label("Rating")
                .default_value(2.0)
                .name("score")
                .required(true),
        );

        let api = service.connect(&|_| {});

        assert_eq!(api.display_value(), 2.0);
        assert!(api.is_item_selected(1));
        let control_attrs = api.control_attrs();

        assert_eq!(control_attrs.get(&HtmlAttr::Role), Some("radiogroup"));
        assert_eq!(
            control_attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("rating-label")
        );
        assert_eq!(
            control_attrs.get(&HtmlAttr::Aria(AriaAttr::Required)),
            Some("true")
        );

        let selected_item = api.item_attrs(1);

        assert_eq!(selected_item.get(&HtmlAttr::Role), Some("radio"));
        assert_eq!(
            selected_item.get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
        assert_eq!(selected_item.get(&HtmlAttr::TabIndex), Some("0"));

        assert_eq!(
            api.item_attrs(0).get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("false"),
            "only the exact committed value is checked in a radiogroup"
        );

        let unselected_item = api.item_attrs(3);

        assert_eq!(
            unselected_item.get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("false")
        );
        assert_eq!(unselected_item.get(&HtmlAttr::TabIndex), Some("-1"));

        let input = api.hidden_input_attrs();

        assert_eq!(input.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(input.get(&HtmlAttr::Name), Some("score"));
        assert_eq!(input.get(&HtmlAttr::Value), Some("2"));
        assert_eq!(
            input.get(&HtmlAttr::Required),
            None,
            "hidden inputs do not participate in native required validation"
        );

        drop(service.send(Event::IncrementRating));

        assert_eq!(*service.context().value.get(), 3.0);

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let api = service.connect(&send);

        api.on_item_keydown(2, &keyboard(KeyboardKey::Home));

        assert_eq!(events.into_inner(), vec![Event::ClearRating]);

        drop(service.send(Event::ClearRating));

        assert_eq!(*service.context().value.get(), 0.0);

        let unlabeled = Service::<Machine>::new(
            Props::new().id("rating"),
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(
            unlabeled
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            None,
            "control does not reference an optional label when no label is provided"
        );
    }

    #[test]
    fn builders_part_attrs_and_event_helpers_are_observable() {
        let props = Props::new()
            .id("rating")
            .default_value(2.0)
            .step(0.0)
            .form("survey");

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let service = service(props);

        let api = service.connect(&send);

        assert_eq!(
            api.hidden_input_attrs().get(&HtmlAttr::Form),
            Some("survey")
        );
        assert_eq!(
            api.part_attrs(Part::Control).get(&HtmlAttr::Role),
            Some("radiogroup")
        );
        assert_eq!(api.item_attrs(0).get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(api.item_attrs(1).get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(api.label_attrs().get(&HtmlAttr::For), None);

        api.on_control_keydown(&keyboard(KeyboardKey::ArrowRight));
        api.on_control_keydown(&keyboard(KeyboardKey::ArrowLeft));
        api.on_control_keydown(&keyboard(KeyboardKey::ArrowDown));
        api.on_control_keydown(&keyboard(KeyboardKey::End));
        api.on_item_focus(3, true);
        api.on_item_blur();
        api.on_item_hover(4);
        api.on_value_hover(2.5);
        api.on_control_mouse_leave();
        api.on_item_rate(2);
        api.on_value_rate(2.5);

        assert_eq!(
            events.into_inner(),
            vec![
                Event::IncrementRating,
                Event::DecrementRating,
                Event::DecrementRating,
                Event::Rate(5.0),
                Event::Focus {
                    index: 3,
                    is_keyboard: true,
                },
                Event::Blur,
                Event::HoverItem(4),
                Event::HoverValue(2.5),
                Event::UnHover,
                Event::Rate(3.0),
                Event::Rate(2.5),
            ]
        );
    }

    #[test]
    fn prop_changes_sync_controlled_value_and_context_fields() {
        let mut service = service(Props::new().id("rating").value(2.0).name("score"));

        drop(service.send(Event::Focus {
            index: 4,
            is_keyboard: true,
        }));
        drop(service.send(Event::HoverItem(4)));
        drop(
            service.set_props(
                Props::new()
                    .id("rating")
                    .value(4.0)
                    .name("score")
                    .count(NonZero::new(3).expect("nonzero"))
                    .allow_half(true)
                    .readonly(true)
                    .disabled(true),
            ),
        );

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(*service.context().value.get(), 3.0);
        assert_eq!(service.context().count.get(), 3);
        assert!(service.context().allow_half);
        assert!(service.context().readonly);
        assert!(service.context().disabled);
        assert_eq!(service.context().focused_index, None);
        assert_eq!(service.context().hovered_value, None);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.control_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("3")
        );
        assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some("3"));
    }

    #[test]
    fn controlled_release_preserves_pending_requested_value() {
        let mut service = service(Props::new().id("rating").value(2.0));

        drop(service.send(Event::Rate(4.0)));

        assert_eq!(*service.context().value.get(), 2.0);
        assert_eq!(service.context().requested_value, Some(4.0));

        drop(service.set_props(Props::new().id("rating")));

        assert_eq!(*service.context().value.get(), 4.0);
        assert_eq!(service.context().requested_value, None);
        assert_eq!(
            service
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value),
            Some("4")
        );
    }

    #[test]
    fn on_props_changed_emits_sync_props_for_context_changes() {
        let old = Props::new().id("rating").value(2.0);

        assert!(Machine::on_props_changed(&old, &old).is_empty());
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().value(3.0)),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().disabled(true)),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().label("Rating")),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().required(true)),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().form("survey")),
            vec![Event::SyncProps]
        );
    }

    #[test]
    fn default_item_label_uses_plural_rules() {
        let messages = Messages::default();
        let locale = Env::default().locale;

        assert_eq!((messages.item_label)(1.0, &locale), "1 star");
        assert_eq!((messages.item_label)(1.5, &locale), "1.5 stars");
        assert_eq!((messages.item_label)(-1.0, &locale), "-1 star");
    }

    #[test]
    fn whole_rating_keyboard_changes_move_focus_and_expose_requested_value() {
        let mut service = service(Props::new().id("rating").value(2.0));

        drop(service.send(Event::Focus {
            index: 1,
            is_keyboard: true,
        }));

        let result = service.send(Event::IncrementRating);

        assert_eq!(service.state(), &State::Focused { index: 2 });
        assert_eq!(service.context().focused_index, Some(2));
        assert!(service.context().focus_visible);
        assert_eq!(*service.context().value.get(), 2.0);
        assert_eq!(service.context().requested_value, Some(3.0));
        assert_eq!(service.connect(&|_| {}).requested_value(), Some(3.0));
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::ValueChange);

        drop(service.send(Event::ClearRating));

        assert_eq!(service.state(), &State::Focused { index: 0 });
        assert_eq!(service.context().focused_index, Some(0));
        assert_eq!(service.context().requested_value, Some(0.0));
    }

    #[test]
    fn half_mode_slider_attrs_hover_and_rounding() {
        let mut rating = service(
            Props::new()
                .id("rating")
                .allow_half(true)
                .default_value(1.0),
        );

        drop(rating.send(Event::Rate(2.26)));

        assert_eq!(*rating.context().value.get(), 2.5);

        let half_item = rating.connect(&|_| {}).item_attrs(2);

        assert_eq!(half_item.get(&HtmlAttr::Data("ars-fraction")), Some("0.5"));
        assert_eq!(half_item.get(&HtmlAttr::Data("ars-half")), Some("true"));

        drop(rating.send(Event::HoverItem(3)));

        let api = rating.connect(&|_| {});

        assert_eq!(api.display_value(), 4.0);
        assert!(api.is_item_highlighted(3));

        let control = api.control_attrs();

        assert_eq!(control.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(control.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(control.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("5"));
        assert_eq!(
            control.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("2.5")
        );
        assert_eq!(
            control.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("2.5 of 5 stars")
        );

        drop(rating.send(Event::HoverValue(2.5)));

        assert_eq!(rating.state(), &State::Hovering { index: 2 });
        assert_eq!(rating.connect(&|_| {}).display_value(), 2.5);

        drop(rating.send(Event::Focus {
            index: 1,
            is_keyboard: true,
        }));
        drop(rating.send(Event::HoverItem(3)));
        drop(rating.send(Event::UnHover));

        assert_eq!(rating.state(), &State::Focused { index: 1 });
        assert_eq!(
            rating.context().focused_index,
            Some(1),
            "hover exit preserves live focus state"
        );
        assert_eq!(rating.connect(&|_| {}).display_value(), 2.5);

        let custom_step = service(Props::new().id("rating").step(1.5).default_value(1.5));

        assert_eq!(
            custom_step
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Role),
            Some("slider"),
            "fractional custom steps use the slider pattern even above one"
        );
    }

    #[test]
    fn readonly_and_disabled_guard_interactions() {
        let mut readonly = service(Props::new().id("rating").default_value(2.0).readonly(true));

        drop(readonly.send(Event::Rate(5.0)));

        assert_eq!(*readonly.context().value.get(), 2.0);

        drop(readonly.send(Event::Focus {
            index: 1,
            is_keyboard: true,
        }));

        assert_eq!(readonly.context().focused_index, Some(1));
        assert_eq!(
            readonly
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-readonly")),
            Some("true")
        );
        assert_eq!(
            readonly
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ReadOnly)),
            Some("true")
        );

        let mut disabled = service(
            Props::new()
                .id("rating")
                .default_value(2.0)
                .name("score")
                .disabled(true),
        );

        drop(disabled.send(Event::IncrementRating));
        drop(disabled.send(Event::Focus {
            index: 1,
            is_keyboard: true,
        }));

        assert_eq!(*disabled.context().value.get(), 2.0);
        assert_eq!(disabled.context().focused_index, None);
        assert_eq!(
            disabled
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-disabled")),
            Some("true")
        );
        assert_eq!(
            disabled
                .connect(&|_| {})
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
        assert_eq!(
            disabled
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Disabled),
            Some("true")
        );

        drop(readonly.send(Event::Blur));

        assert_eq!(readonly.context().focused_index, None);

        disabled.context_mut().focused_index = Some(2);
        disabled.context_mut().focus_visible = true;
        disabled.context_mut().hovered_value = Some(3.0);

        drop(disabled.send(Event::Blur));

        assert_eq!(disabled.context().focused_index, None);
        assert!(!disabled.context().focus_visible);
        assert_eq!(disabled.context().hovered_value, None);
    }

    #[test]
    fn decrement_and_step_edge_cases_are_observable() {
        let mut group = service(Props::new().id("rating").default_value(2.0));

        drop(group.send(Event::DecrementRating));

        assert_eq!(*group.context().value.get(), 1.0);

        let mut negative_step = service(Props::new().id("rating").default_value(1.0).step(-0.5));

        drop(negative_step.send(Event::IncrementRating));

        assert_eq!(*negative_step.context().value.get(), 2.0);

        let mut fractional = service(Props::new().id("rating").default_value(0.0).step(0.25));

        drop(fractional.send(Event::Rate(2.26)));

        assert_eq!(*fractional.context().value.get(), 2.25);

        let fractional_api = fractional.connect(&|_| {});

        assert_eq!(
            fractional_api.control_attrs().get(&HtmlAttr::Role),
            Some("slider")
        );
        assert_eq!(fractional_api.item_attrs(1).get(&HtmlAttr::Role), None);
        assert_eq!(
            fractional_api
                .item_attrs(2)
                .get(&HtmlAttr::Data("ars-fraction")),
            Some("0.25")
        );
        assert_eq!(
            fractional_api
                .item_attrs(2)
                .get(&HtmlAttr::Data("ars-half")),
            None
        );

        let mut explicit_whole = service(
            Props::new()
                .id("rating")
                .allow_half(true)
                .step(1.0)
                .default_value(0.0),
        );

        drop(explicit_whole.send(Event::Rate(2.5)));

        assert_eq!(*explicit_whole.context().value.get(), 3.0);
        let explicit_whole_api = explicit_whole.connect(&|_| {});
        assert_eq!(
            explicit_whole_api.control_attrs().get(&HtmlAttr::Role),
            Some("radiogroup")
        );

        let mut near_default_step = service(
            Props::new()
                .id("rating")
                .default_value(0.0)
                .step(1.0 + f64::EPSILON),
        );

        drop(near_default_step.send(Event::IncrementRating));

        assert!(
            (*near_default_step.context().value.get() - (1.0 + f64::EPSILON)).abs() <= f64::EPSILON
        );

        let mut non_divisor_step = service(Props::new().id("rating").default_value(4.5).step(1.5));

        drop(non_divisor_step.send(Event::IncrementRating));

        assert_eq!(
            *non_divisor_step.context().value.get(),
            5.0,
            "max remains reachable even when count is not divisible by step"
        );

        drop(non_divisor_step.send(Event::Rate(5.0)));

        assert_eq!(*non_divisor_step.context().value.get(), 5.0);

        let zero = service(Props::new().id("rating").default_value(0.0));

        assert_eq!(
            zero.connect(&|_| {}).item_attrs(0).get(&HtmlAttr::TabIndex),
            Some("0")
        );

        let mut boundary = service(Props::new().id("rating").default_value(1.0));

        boundary.context_mut().value.set(1.0 + f64::EPSILON);

        assert_eq!(
            boundary
                .connect(&|_| {})
                .item_attrs(0)
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );
    }

    #[test]
    fn count_controls_maximum() {
        let mut service = service(
            Props::new()
                .id("rating")
                .count(NonZero::new(3).expect("non-zero"))
                .default_value(4.0),
        );

        assert_eq!(*service.context().value.get(), 3.0);

        let result = service.send(Event::IncrementRating);

        assert_eq!(*service.context().value.get(), 3.0);
        assert!(
            result.pending_effects.is_empty(),
            "unchanged max-bound increment is a no-op"
        );
    }

    #[test]
    fn controlled_increment_repeats_from_pending_request() {
        let mut service = service(Props::new().id("rating").value(1.0));

        drop(service.send(Event::IncrementRating));

        assert_eq!(service.context().requested_value, Some(2.0));

        drop(service.send(Event::IncrementRating));

        assert_eq!(
            service.context().requested_value,
            Some(3.0),
            "keyboard repeats before parent sync continue from the pending request"
        );
        assert_eq!(
            *service.context().value.get(),
            1.0,
            "controlled value stays on the prop value until accepted"
        );
    }

    #[test]
    fn unchanged_rating_updates_whole_mode_focus() {
        let mut service = service(Props::new().id("rating").default_value(0.0));

        drop(service.send(Event::Focus {
            index: 3,
            is_keyboard: true,
        }));
        drop(service.send(Event::ClearRating));

        assert_eq!(
            service.context().focused_index,
            Some(0),
            "Home keeps the committed zero value but moves the roving tab stop"
        );
        assert_eq!(*service.context().value.get(), 0.0);
    }

    #[test]
    fn rating_group_snapshots_cover_output_branches() {
        let whole = service(
            Props::new()
                .id("rating")
                .label("Rating")
                .default_value(2.0)
                .name("score"),
        );

        let whole_api = whole.connect(&|_| {});

        assert_snapshot!("rating_group_root", snapshot_attrs(&whole_api.root_attrs()));
        assert_snapshot!(
            "rating_group_label",
            snapshot_attrs(&whole_api.label_attrs())
        );
        assert_snapshot!(
            "rating_group_control_whole",
            snapshot_attrs(&whole_api.control_attrs())
        );
        assert_snapshot!(
            "rating_group_item_selected",
            snapshot_attrs(&whole_api.item_attrs(1))
        );
        assert_snapshot!(
            "rating_group_hidden_input",
            snapshot_attrs(&whole_api.hidden_input_attrs())
        );

        let mut half = service(
            Props::new()
                .id("rating")
                .default_value(2.5)
                .allow_half(true),
        );

        drop(half.send(Event::HoverItem(2)));

        let half_api = half.connect(&|_| {});

        assert_snapshot!(
            "rating_group_control_half",
            snapshot_attrs(&half_api.control_attrs())
        );
        assert_snapshot!(
            "rating_group_item_half_highlighted",
            snapshot_attrs(&half_api.item_attrs(2))
        );

        let disabled = service(Props::new().id("rating").disabled(true));

        assert_snapshot!(
            "rating_group_item_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).item_attrs(0))
        );
    }
}
