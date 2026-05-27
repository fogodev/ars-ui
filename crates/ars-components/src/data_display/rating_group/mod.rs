//! RatingGroup data-display component machine.
//!
//! Owns committed rating value, hover preview, keyboard focus intent, and
//! form/ARIA attributes. Adapters render the visual star icons and resolve
//! pointer geometry for fractional hit testing.

use alloc::{
    format,
    string::{String, ToString as _},
    sync::Arc,
};
use core::{
    fmt::{self, Debug},
    num::NonZero,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentMessages, ComponentPart, ConnectApi, Env, HasId,
    HtmlAttr, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

type ItemLabelFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

/// Props for the `RatingGroup` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

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
    pub const fn allow_half(mut self, allow_half: bool) -> Self {
        self.allow_half = allow_half;
        self
    }

    /// Sets the rating increment.
    #[must_use]
    pub const fn step(mut self, step: f64) -> Self {
        self.step = step;
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

    /// Pointer entered a rating item.
    HoverItem(usize),

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
            item_label: MessageFn::new(Arc::new(|value: f64, _locale: &Locale| {
                if (value - 1.0).abs() < f64::EPSILON {
                    format!("{value} star")
                } else {
                    format!("{value} stars")
                }
            }) as Arc<ItemLabelFn>),
        }
    }
}

impl ComponentMessages for Messages {}

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
    type Effect = NoEffect;
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
            Event::Rate(value) => {
                let value = round_to_step(
                    sanitize_value(*value, context.count),
                    effective_step(props),
                    context.count,
                );

                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        ctx.value.set(value);
                        ctx.hovered_value = None;
                    }),
                )
            }

            Event::HoverItem(index) => {
                let index = clamp_index(*index, context.count);
                let value = (index + 1) as f64;

                Some(TransitionPlan::to(State::Hovering { index }).apply(
                    move |ctx: &mut Context| {
                        ctx.hovered_value = Some(value);
                    },
                ))
            }

            Event::UnHover => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.hovered_value = None;
            })),

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
                    (*context.value.get() + effective_step(props)).min(max_value(context.count));

                Self::transition(_state, &Event::Rate(value), context, props)
            }

            Event::DecrementRating => {
                let value = (*context.value.get() - effective_step(props)).max(0.0);

                Self::transition(_state, &Event::Rate(value), context, props)
            }

            Event::ClearRating => Self::transition(_state, &Event::Rate(0.0), context, props),
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

        attrs
            .set(HtmlAttr::Id, format!("{}-label", self.props.id))
            .set(HtmlAttr::For, format!("{}-control", self.props.id));

        attrs
    }

    /// Returns attributes for the interactive control.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Control);

        attrs
            .set(HtmlAttr::Id, format!("{}-control", self.props.id))
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                format!("{}-label", self.props.id),
            );

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
                    (self.context.messages.item_label)(
                        *self.context.value.get(),
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

        if has_half_value(self.display_value(), index) {
            attrs.set_bool(HtmlAttr::Data("ars-half"), true);
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
                    if selected { "true" } else { "false" },
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

        if self.props.required {
            attrs.set_bool(HtmlAttr::Required, true);
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

    /// Dispatches pointer leave from the rating control.
    pub fn on_control_mouse_leave(&self) {
        (self.send)(Event::UnHover);
    }

    /// Dispatches a whole-item rating commit.
    pub fn on_item_rate(&self, index: usize) {
        (self.send)(Event::Rate((index + 1) as f64));
    }

    fn is_item_tabbable(&self, index: usize) -> bool {
        let value = *self.context.value.get();

        if value == 0.0 {
            return index == 0;
        }

        ((index + 1) as f64 - value).abs() < f64::EPSILON
    }

    fn uses_slider_pattern(&self) -> bool {
        effective_step(self.props) < 1.0
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

const fn effective_step(props: &Props) -> f64 {
    if props.step.is_finite() && props.step > 0.0 && (props.step - 1.0).abs() > f64::EPSILON {
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
    sanitize_value((value / step).round() * step, count)
}

fn clamp_index(index: usize, count: NonZero<u32>) -> usize {
    index.min(count.get().saturating_sub(1) as usize)
}

const fn has_half_value(value: f64, index: usize) -> bool {
    let lower = index as f64;
    let upper = (index + 1) as f64;

    value > lower && value < upper
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec::Vec};
    use core::{cell::RefCell, num::NonZero};

    use ars_core::{AttrMap, Env, Service};
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
                .default_value(2.0)
                .name("score")
                .required(true),
        );

        let api = service.connect(&|_| {});

        assert_eq!(api.display_value(), 2.0);
        assert!(api.is_item_selected(1));
        assert_eq!(api.control_attrs().get(&HtmlAttr::Role), Some("radiogroup"));

        let selected_item = api.item_attrs(1);

        assert_eq!(selected_item.get(&HtmlAttr::Role), Some("radio"));
        assert_eq!(
            selected_item.get(&HtmlAttr::Aria(AriaAttr::Checked)),
            Some("true")
        );
        assert_eq!(selected_item.get(&HtmlAttr::TabIndex), Some("0"));

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
        assert_eq!(input.get(&HtmlAttr::Required), Some("true"));

        drop(service.send(Event::IncrementRating));

        assert_eq!(*service.context().value.get(), 3.0);

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let api = service.connect(&send);

        api.on_item_keydown(2, &keyboard(KeyboardKey::Home));

        assert_eq!(events.into_inner(), vec![Event::ClearRating]);

        drop(service.send(Event::ClearRating));

        assert_eq!(*service.context().value.get(), 0.0);
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

        api.on_control_keydown(&keyboard(KeyboardKey::ArrowRight));
        api.on_control_keydown(&keyboard(KeyboardKey::ArrowLeft));
        api.on_control_keydown(&keyboard(KeyboardKey::ArrowDown));
        api.on_control_keydown(&keyboard(KeyboardKey::End));
        api.on_item_focus(3, true);
        api.on_item_blur();
        api.on_item_hover(4);
        api.on_control_mouse_leave();
        api.on_item_rate(2);

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
                Event::UnHover,
                Event::Rate(3.0),
            ]
        );
    }

    #[test]
    fn half_mode_slider_attrs_hover_and_rounding() {
        let mut service = service(
            Props::new()
                .id("rating")
                .allow_half(true)
                .default_value(1.0),
        );

        drop(service.send(Event::Rate(2.26)));

        assert_eq!(*service.context().value.get(), 2.5);

        let half_item = service.connect(&|_| {}).item_attrs(2);

        assert_eq!(half_item.get(&HtmlAttr::Data("ars-half")), Some("true"));

        drop(service.send(Event::HoverItem(3)));

        let api = service.connect(&|_| {});

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

        let mut disabled = service(Props::new().id("rating").default_value(2.0).disabled(true));

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
                .control_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
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

        let mut near_default_step = service(
            Props::new()
                .id("rating")
                .default_value(0.0)
                .step(1.0 + f64::EPSILON),
        );

        drop(near_default_step.send(Event::IncrementRating));

        assert_eq!(*near_default_step.context().value.get(), 1.0);

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

        drop(service.send(Event::IncrementRating));

        assert_eq!(*service.context().value.get(), 3.0);
    }

    #[test]
    fn rating_group_snapshots_cover_output_branches() {
        let whole = service(Props::new().id("rating").default_value(2.0).name("score"));

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
