//! Slider component state machine and connect API.

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug, Display},
    hash::{Hash, Hasher},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, CssProperty, Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, Orientation,
    PendingEffect, Rect, TransitionPlan, no_cleanup,
};
use ars_i18n::number::{FormatOptions, Formatter};

/// The state of the `Slider` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle.
    Idle,

    /// The slider thumb is focused.
    Focused,

    /// The slider thumb is being dragged.
    Dragging,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused => "focused",
            Self::Dragging => "dragging",
        })
    }
}

/// Events accepted by the `Slider` state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The thumb received focus.
    Focus {
        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// The thumb lost focus.
    Blur,

    /// A pointer gesture started with an adapter-supplied logical value.
    PointerDown {
        /// Raw logical slider value computed by the adapter or helper.
        value: f64,
    },

    /// A pointer gesture moved to an adapter-supplied logical value.
    PointerMove {
        /// Raw logical slider value computed by the adapter or helper.
        value: f64,
    },

    /// The active pointer gesture ended.
    PointerUp,

    /// Increment by [`Context::step`].
    Increment,

    /// Decrement by [`Context::step`].
    Decrement,

    /// Increment by [`Context::large_step`] or ten steps.
    IncrementLarge,

    /// Decrement by [`Context::large_step`] or ten steps.
    DecrementLarge,

    /// Set the current value to [`Context::min`].
    SetToMin,

    /// Set the current value to [`Context::max`].
    SetToMax,

    /// Programmatically set the current value.
    SetValue(f64),

    /// Synchronize the externally controlled value prop.
    SyncValue(Option<f64>),

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether the label part is rendered.
    SetHasLabel(bool),

    /// Track whether the description part is rendered.
    SetHasDescription(bool),
}

/// The origin used to render the filled range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Origin {
    /// Fill starts at the minimum value and ends at the thumb.
    #[default]
    Start,

    /// Fill starts at the middle of the track and extends to the thumb.
    Center,

    /// Fill starts at the thumb and ends at the maximum value.
    End,
}

/// How the thumb aligns with the track boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThumbAlignment {
    /// The thumb center aligns with the track min and max.
    Center,

    /// The thumb remains visually contained within track bounds.
    #[default]
    Contain,
}

/// A configured slider mark.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    /// The numeric value represented by the mark.
    pub value: f64,

    /// Optional localized mark label.
    pub label: Option<String>,
}

/// Adapter-normalized pointer coordinates used by slider geometry helpers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderPointer {
    /// Pointer x-coordinate in viewport coordinates.
    pub x: f64,

    /// Pointer y-coordinate in viewport coordinates.
    pub y: f64,
}

/// Internal context for the `Slider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current slider value, controlled or uncontrolled.
    pub value: Bindable<f64>,

    /// Minimum allowed value.
    pub min: f64,

    /// Maximum allowed value.
    pub max: f64,

    /// Step size for normal increments.
    pub step: f64,

    /// Optional large step for PageUp/PageDown and Shift+Arrow.
    pub large_step: Option<f64>,

    /// Whether the slider is disabled.
    pub disabled: bool,

    /// Whether the slider is read-only.
    pub readonly: bool,

    /// Whether the slider is invalid.
    pub invalid: bool,

    /// Slider layout orientation.
    pub orientation: Orientation,

    /// Text direction used for horizontal keyboard and visual mirroring.
    pub dir: Direction,

    /// Whether the thumb is focused.
    pub focused: bool,

    /// Whether focus should be visibly indicated.
    pub focus_visible: bool,

    /// Whether a pointer drag is active.
    pub dragging: bool,

    /// Pending drag value used for controlled commit callbacks.
    pub drag_value: Option<f64>,

    /// Whether the active drag changed the effective value.
    pub drag_changed: bool,

    /// Fill origin for the range part.
    pub origin: Origin,

    /// How the thumb aligns with track boundaries.
    pub thumb_alignment: ThumbAlignment,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Associated form id for hidden input submission.
    pub form: Option<String>,

    /// Configured marks.
    pub marks: Vec<Mark>,

    /// Whether the slider uses discrete value-label semantics.
    pub discrete: bool,

    /// Optional labels for discrete value positions.
    pub value_labels: Option<Vec<String>>,

    /// Whether a description part is rendered.
    pub has_description: bool,

    /// Whether a label part is rendered.
    pub has_label: bool,

    /// Resolved locale.
    pub locale: Locale,

    /// Resolved localized messages.
    pub messages: Messages,

    /// Stable component ids.
    pub ids: ComponentIds,
}

/// Props for the `Slider` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base id for the slider.
    pub id: String,

    /// Controlled slider value.
    pub value: Option<f64>,

    /// Default value for uncontrolled mode.
    pub default_value: f64,

    /// Minimum allowed value.
    pub min: f64,

    /// Maximum allowed value.
    pub max: f64,

    /// Step size for normal increments.
    pub step: f64,

    /// Optional large step for PageUp/PageDown and Shift+Arrow.
    pub large_step: Option<f64>,

    /// Whether the slider is disabled.
    pub disabled: bool,

    /// Whether the slider is read-only.
    pub readonly: bool,

    /// Whether the slider is invalid.
    pub invalid: bool,

    /// Slider layout orientation.
    pub orientation: Orientation,

    /// Text direction used for horizontal keyboard and visual mirroring.
    pub dir: Direction,

    /// Fill origin for the range part.
    pub origin: Origin,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Associated form id for hidden input submission.
    pub form: Option<String>,

    /// Configured marks.
    pub marks: Vec<Mark>,

    /// Formatter for visible tick labels.
    pub tick_format: Option<Callback<dyn Fn(f64) -> String + Send + Sync>>,

    /// Formatter for visible current value output.
    pub value_format: Option<Callback<dyn Fn(f64) -> String + Send + Sync>>,

    /// Formatter for current value output and `aria-valuetext`.
    pub format_value: Option<Callback<dyn Fn(f64) -> String + Send + Sync>>,

    /// Formatter used specifically for thumb `aria-valuetext`.
    pub format_value_text: Option<Callback<dyn Fn(f64) -> String + Send + Sync>>,

    /// Whether the slider uses discrete value-label semantics.
    pub discrete: bool,

    /// Optional labels for discrete value positions.
    pub value_labels: Option<Vec<String>>,

    /// How the thumb aligns with track boundaries.
    pub thumb_alignment: ThumbAlignment,

    /// Callback fired when value-changing user intent requests a new value.
    pub on_value_change: Option<Callback<dyn Fn(f64) + Send + Sync>>,

    /// Callback fired when a drag or keyboard adjustment is committed.
    pub on_value_change_end: Option<Callback<dyn Fn(f64) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            large_step: None,
            disabled: false,
            readonly: false,
            invalid: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            origin: Origin::Start,
            name: None,
            form: None,
            marks: Vec::new(),
            tick_format: None,
            value_format: None,
            format_value: None,
            format_value_text: None,
            discrete: false,
            value_labels: None,
            thumb_alignment: ThumbAlignment::Contain,
            on_value_change: None,
            on_value_change_end: None,
        }
    }
}

impl Props {
    /// Returns default slider props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Localized messages for the `Slider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible fallback label for the slider thumb.
    pub thumb_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            thumb_label: MessageFn::static_str("Value"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for side effects emitted by the slider machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_value_change`.
    ValueChange,

    /// Invoke `Props::on_value_change_end`.
    ValueChangeEnd,
}

/// Machine for the `Slider` component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let initial = snap_to_step(
            props.value.unwrap_or(props.default_value),
            props.min,
            props.max,
            props.step,
        );

        (
            State::Idle,
            Context {
                value: if props.value.is_some() {
                    Bindable::controlled(initial)
                } else {
                    Bindable::uncontrolled(initial)
                },
                min: props.min,
                max: props.max,
                step: props.step,
                large_step: props.large_step,
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                orientation: props.orientation,
                dir: props.dir,
                focused: false,
                focus_visible: false,
                dragging: false,
                drag_value: None,
                drag_changed: false,
                origin: props.origin,
                thumb_alignment: props.thumb_alignment,
                name: props.name.clone(),
                form: props.form.clone(),
                marks: props.marks.clone(),
                discrete: props.discrete,
                value_labels: props.value_labels.clone(),
                has_description: false,
                has_label: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        _state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            match event {
                Event::PointerDown { .. }
                | Event::PointerMove { .. }
                | Event::Increment
                | Event::Decrement
                | Event::IncrementLarge
                | Event::DecrementLarge
                | Event::SetToMin
                | Event::SetToMax
                | Event::SetValue(_) => return None,
                _ => {}
            }
        }

        match event {
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
                ctx.dragging = false;
                ctx.drag_value = None;
                ctx.drag_changed = false;
            })),

            Event::PointerDown { value } => {
                if !value.is_finite() {
                    return None;
                }

                Some(
                    set_value_plan(ctx, *value, Some(State::Dragging), true, false, true).apply(
                        |ctx: &mut Context| {
                            ctx.dragging = true;
                        },
                    ),
                )
            }

            Event::PointerMove { value } if ctx.dragging => {
                if !value.is_finite() {
                    return None;
                }

                Some(set_value_plan(ctx, *value, None, true, false, true))
            }

            Event::PointerMove { .. } => None,

            Event::PointerUp => {
                if !ctx.dragging {
                    return None;
                }

                let target = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };

                let value = ctx.drag_value.unwrap_or_else(|| bounded_value(ctx));
                let mut plan = TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.dragging = false;
                    ctx.drag_value = None;
                    ctx.drag_changed = false;
                });

                if ctx.drag_changed {
                    plan = plan.with_effect(value_change_end_effect(value));
                }

                Some(plan)
            }

            Event::Increment => Some(step_plan(ctx, ctx.step, true)),

            Event::Decrement => Some(step_plan(ctx, ctx.step, false)),

            Event::IncrementLarge => Some(step_plan(ctx, large_step(ctx), true)),

            Event::DecrementLarge => Some(step_plan(ctx, large_step(ctx), false)),

            Event::SetToMin => {
                let (min, _) = normalized_bounds(ctx.min, ctx.max);
                Some(set_value_plan(ctx, min, None, true, true, false))
            }

            Event::SetToMax => {
                let (_, max) = normalized_bounds(ctx.min, ctx.max);
                Some(set_value_plan(ctx, max, None, true, true, false))
            }

            Event::SetValue(value) => {
                if !value.is_finite() {
                    return None;
                }

                Some(set_value_plan(ctx, *value, None, true, false, false))
            }

            Event::SyncValue(value) => match value {
                Some(value) if value.is_finite() => {
                    let value = snap_to_step(*value, ctx.min, ctx.max, ctx.step);
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.value.set(value);
                        ctx.value.sync_controlled(Some(value));
                    }))
                }

                Some(_) => {
                    let value = bounded_value(ctx);
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.value.set(value);
                        ctx.value.sync_controlled(Some(value));
                    }))
                }

                None => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.value.sync_controlled(None);
                })),
            },

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.min = props.min;
                    ctx.max = props.max;
                    ctx.step = props.step;
                    ctx.large_step = props.large_step;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.orientation = props.orientation;
                    ctx.dir = props.dir;
                    ctx.origin = props.origin;
                    ctx.thumb_alignment = props.thumb_alignment;
                    ctx.name = props.name.clone();
                    ctx.form = props.form.clone();
                    ctx.marks = props.marks.clone();
                    ctx.discrete = props.discrete;
                    ctx.value_labels = props.value_labels.clone();

                    let value = snap_to_step(*ctx.value.get(), ctx.min, ctx.max, ctx.step);

                    ctx.value.set(value);

                    if ctx.dragging {
                        let drag_value = snap_to_step(
                            ctx.drag_value.unwrap_or(value),
                            ctx.min,
                            ctx.max,
                            ctx.step,
                        );

                        ctx.drag_changed |= (drag_value - value).abs() > f64::EPSILON;
                        ctx.drag_value = Some(drag_value);
                    }
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }

            Event::SetHasLabel(has_label) => {
                let has_label = *has_label;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_label = has_label;
                }))
            }
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "slider::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
        }

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
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

/// Structural parts exposed by the `Slider` connect API.
///
/// This enum implements [`ComponentPart`] manually because `Marker { value }`
/// preserves the spec's `f64` marker value, while `ComponentPart` requires
/// `Eq + Hash`. Marker equality and hashing therefore use `f64::to_bits`.
#[derive(Clone, Debug)]
pub enum Part {
    /// The root container.
    Root,

    /// The visible label.
    Label,

    /// The track that receives pointer intent.
    Track,

    /// The filled range within the track.
    Range,

    /// The interactive slider thumb.
    Thumb,

    /// Optional value output.
    Output,

    /// Optional marker group.
    MarkerGroup,

    /// A marker for a specific value.
    Marker {
        /// The marker value.
        value: f64,
    },

    /// Hidden input used for form submission.
    HiddenInput,

    /// Decorative drag-state indicator.
    DraggingIndicator,

    /// Optional descriptive text.
    Description,

    /// Optional validation error text.
    ErrorMessage,
}

impl PartialEq for Part {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Marker { value: left }, Self::Marker { value: right }) => {
                left.to_bits() == right.to_bits()
            }

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Part {}

impl Hash for Part {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        if let Self::Marker { value } = self {
            value.to_bits().hash(state);
        }
    }
}

impl ComponentPart for Part {
    const ROOT: Self = Self::Root;

    fn scope() -> &'static str {
        "slider"
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Label => "label",
            Self::Track => "track",
            Self::Range => "range",
            Self::Thumb => "thumb",
            Self::Output => "output",
            Self::MarkerGroup => "marker-group",
            Self::Marker { .. } => "marker",
            Self::HiddenInput => "hidden-input",
            Self::DraggingIndicator => "dragging-indicator",
            Self::Description => "description",
            Self::ErrorMessage => "error-message",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::Root,
            Self::Label,
            Self::Track,
            Self::Range,
            Self::Thumb,
            Self::Output,
            Self::MarkerGroup,
            Self::Marker { value: 0.0 },
            Self::HiddenInput,
            Self::DraggingIndicator,
            Self::Description,
            Self::ErrorMessage,
        ]
    }
}

/// Connect API for the `Slider` component.
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

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::Output => self.output_attrs(),
            Part::MarkerGroup => self.marker_group_attrs(),
            Part::Marker { value } => self.marker_attrs(value),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::DraggingIndicator => self.dragging_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Sends a focus event for the thumb.
    pub fn on_thumb_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends a blur event for the thumb.
    pub fn on_thumb_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handles thumb keydown intent.
    pub fn on_thumb_keydown(&self, key: KeyboardKey, shift: bool) {
        let is_rtl_horizontal =
            self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl;

        let event = match key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowLeft => {
                let moves_forward = matches!(key, KeyboardKey::ArrowRight) ^ is_rtl_horizontal;

                match (moves_forward, shift) {
                    (true, true) => Event::IncrementLarge,
                    (true, false) => Event::Increment,
                    (false, true) => Event::DecrementLarge,
                    (false, false) => Event::Decrement,
                }
            }

            KeyboardKey::ArrowUp if shift => Event::IncrementLarge,

            KeyboardKey::ArrowUp => Event::Increment,

            KeyboardKey::ArrowDown if shift => Event::DecrementLarge,

            KeyboardKey::ArrowDown => Event::Decrement,

            KeyboardKey::PageUp => Event::IncrementLarge,

            KeyboardKey::PageDown => Event::DecrementLarge,

            KeyboardKey::Home => Event::SetToMin,

            KeyboardKey::End => Event::SetToMax,

            _ => return,
        };

        (self.send)(event);
    }

    /// Returns attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.state.to_string())
            .set(
                HtmlAttr::Data("ars-orientation"),
                orientation_token(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.dragging {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for the label.
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

    /// Returns attributes for the track.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("track"));

        attrs
    }

    /// Sends track pointerdown intent with an adapter-supplied value.
    pub fn on_track_pointerdown(&self, value: f64) {
        (self.send)(Event::PointerDown { value });
    }

    /// Returns attributes for the filled range.
    #[must_use]
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();

        let (start_prop, start, size_prop, size) = range_style(self.ctx);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set_style(start_prop, start)
            .set_style(size_prop, size);

        attrs
    }

    /// Returns attributes for the thumb.
    #[must_use]
    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();

        let (position_prop, position) = thumb_style(self.ctx);

        let (min, max) = normalized_bounds(self.ctx.min, self.ctx.max);
        let value = bounded_value(self.ctx);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("thumb"))
            .set(HtmlAttr::Role, "slider")
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), number_string(min))
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), number_string(max))
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), number_string(value))
            .set(
                HtmlAttr::Aria(AriaAttr::ValueText),
                value_text(self.ctx, self.props),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.ctx.orientation),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.thumb_label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set_style(position_prop, position)
            .set(
                HtmlAttr::Data("ars-thumb-alignment"),
                match self.ctx.thumb_alignment {
                    ThumbAlignment::Center => "center",
                    ThumbAlignment::Contain => "contain",
                },
            )
            .set(HtmlAttr::Class, "ars-touch-none");

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ErrorMessage),
                self.ctx.ids.part("error-message"),
            );
        }

        if self.ctx.has_label {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );
        }

        if self.ctx.has_description {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for the output element.
    #[must_use]
    pub fn output_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Output.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::For, self.ctx.ids.part("thumb"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "off");

        if self.ctx.has_label {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );
        }

        attrs
    }

    /// Returns attributes for the marker group.
    #[must_use]
    pub fn marker_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MarkerGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "presentation");

        attrs
    }

    /// Returns attributes for a marker value.
    #[must_use]
    pub fn marker_attrs(&self, value: f64) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Marker { value }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-value"), number_string(value));

        if marker_in_range(self.ctx, value) {
            attrs.set_bool(HtmlAttr::Data("ars-in-range"), true);
        }

        if let Some(label) = marker_label(self.props, value) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }

        attrs
    }

    /// Returns attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Value, number_string(bounded_value(self.ctx)));

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.as_str());
        }

        if let Some(form) = &self.ctx.form {
            attrs.set(HtmlAttr::Form, form.as_str());
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns attributes for the drag indicator.
    #[must_use]
    pub fn dragging_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DraggingIndicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if self.ctx.dragging {
                    "dragging"
                } else {
                    "idle"
                },
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if !self.ctx.dragging {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for the description.
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

    /// Returns attributes for the error message.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }
}

/// Computes a slider value from adapter-supplied pointer and track geometry.
#[must_use]
pub fn value_from_pointer(pointer: SliderPointer, track: Rect, ctx: &Context) -> Option<f64> {
    let percent = match ctx.orientation {
        Orientation::Horizontal => {
            if !pointer.x.is_finite()
                || !track.x.is_finite()
                || !track.width.is_finite()
                || track.width <= 0.0
            {
                return None;
            }

            let percent = ((pointer.x - track.x) / track.width).clamp(0.0, 1.0);

            if ctx.dir == Direction::Rtl {
                1.0 - percent
            } else {
                percent
            }
        }

        Orientation::Vertical => {
            if !pointer.y.is_finite()
                || !track.y.is_finite()
                || !track.height.is_finite()
                || track.height <= 0.0
            {
                return None;
            }

            ((track.y + track.height - pointer.y) / track.height).clamp(0.0, 1.0)
        }
    };

    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    Some(snap_to_step(
        min + percent * (max - min),
        min,
        max,
        ctx.step,
    ))
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    float_output_changed(old.min, new.min)
        || float_output_changed(old.max, new.max)
        || float_output_changed(old.step, new.step)
        || old.large_step != new.large_step
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.orientation != new.orientation
        || old.dir != new.dir
        || old.origin != new.origin
        || old.name != new.name
        || old.form != new.form
        || old.marks != new.marks
        || old.discrete != new.discrete
        || old.value_labels != new.value_labels
        || old.thumb_alignment != new.thumb_alignment
}

fn float_output_changed(old: f64, new: f64) -> bool {
    if old.is_nan() || new.is_nan() {
        return old.to_bits() != new.to_bits();
    }

    (old - new).abs() > f64::EPSILON
}

fn set_value_plan(
    ctx: &Context,
    value: f64,
    target: Option<State>,
    value_change: bool,
    value_change_end: bool,
    track_drag: bool,
) -> TransitionPlan<Machine> {
    let next = snap_to_step(value, ctx.min, ctx.max, ctx.step);
    let previous = if track_drag {
        ctx.drag_value.unwrap_or_else(|| bounded_value(ctx))
    } else {
        bounded_value(ctx)
    };
    let changed = (next - previous).abs() > f64::EPSILON;

    let mut plan = if let Some(target) = target {
        TransitionPlan::to(target)
    } else {
        TransitionPlan::new()
    }
    .apply(move |ctx: &mut Context| {
        ctx.value.set(next);
        if track_drag {
            ctx.drag_value = Some(next);
            ctx.drag_changed |= changed;
        }
    });

    if value_change && changed {
        plan = plan.with_effect(value_change_effect(next));
    }

    if value_change_end && changed {
        plan = plan.with_effect(value_change_end_effect(next));
    }

    plan
}

fn step_plan(ctx: &Context, step: f64, up: bool) -> TransitionPlan<Machine> {
    let current = *ctx.value.get();

    let raw = if up { current + step } else { current - step };

    set_value_plan(ctx, raw, None, true, true, false)
}

fn value_change_effect(value: f64) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_change {
                callback(value);
            }

            no_cleanup()
        },
    )
}

fn value_change_end_effect(value: f64) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueChangeEnd,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_change_end {
                callback(value);
            }

            no_cleanup()
        },
    )
}

fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let (min, max) = normalized_bounds(min, max);

    if !value.is_finite() {
        return min;
    }

    let clamped = value.clamp(min, max);

    if step <= 0.0 || !step.is_finite() {
        return clamped;
    }

    let steps_from_min = ((clamped - min) / step).round();

    (min + steps_from_min * step).clamp(min, max)
}

fn normalized_bounds(min: f64, max: f64) -> (f64, f64) {
    match (min.is_finite(), max.is_finite()) {
        (true, true) if min <= max => (min, max),
        (true, true) => (max, min),
        (true, false) => (min, min),
        (false, true) => (max, max),
        (false, false) => (0.0, 0.0),
    }
}

fn bounded_value(ctx: &Context) -> f64 {
    snap_to_step(*ctx.value.get(), ctx.min, ctx.max, ctx.step)
}

fn marker_in_range(ctx: &Context, value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }

    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    if value < min || value > max {
        return false;
    }

    let current = bounded_value(ctx);

    match ctx.origin {
        Origin::Start => value <= current,
        Origin::End => value >= current,
        Origin::Center => {
            let center = min + (max - min) / 2.0;
            if current >= center {
                value >= center && value <= current
            } else {
                value >= current && value <= center
            }
        }
    }
}

fn large_step(ctx: &Context) -> f64 {
    ctx.large_step.unwrap_or(ctx.step * 10.0)
}

fn value_percent(ctx: &Context) -> f64 {
    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    if (max - min).abs() <= f64::EPSILON {
        return 0.0;
    }

    ((bounded_value(ctx) - min) / (max - min)).clamp(0.0, 1.0) * 100.0
}

fn range_style(ctx: &Context) -> (CssProperty, String, CssProperty, String) {
    let percent = value_percent(ctx);

    let is_horizontal = ctx.orientation == Orientation::Horizontal;

    let is_rtl = is_horizontal && ctx.dir == Direction::Rtl;
    let visual_percent = if is_rtl { 100.0 - percent } else { percent };

    let (start, size) = match ctx.origin {
        Origin::Start if is_rtl => (100.0 - percent, percent),

        Origin::Start => (0.0, percent),

        Origin::Center => {
            let start = visual_percent.min(50.0);
            (start, (visual_percent - 50.0).abs())
        }

        Origin::End if is_rtl => (0.0, 100.0 - percent),

        Origin::End => (percent, 100.0 - percent),
    };

    if is_horizontal {
        (
            CssProperty::Left,
            percent_string(start),
            CssProperty::Width,
            percent_string(size),
        )
    } else {
        (
            CssProperty::Bottom,
            percent_string(start),
            CssProperty::Height,
            percent_string(size),
        )
    }
}

fn thumb_style(ctx: &Context) -> (CssProperty, String) {
    let percent = value_percent(ctx);

    match ctx.orientation {
        Orientation::Horizontal if ctx.dir == Direction::Rtl => {
            (CssProperty::Right, percent_string(percent))
        }

        Orientation::Horizontal => (CssProperty::Left, percent_string(percent)),

        Orientation::Vertical => (CssProperty::Bottom, percent_string(percent)),
    }
}

fn value_text(ctx: &Context, props: &Props) -> String {
    let value = bounded_value(ctx);

    if let Some(formatter) = &props.format_value_text {
        return formatter(value);
    }

    if ctx.discrete {
        if let Some(labels) = &ctx.value_labels
            && let Some(label) = discrete_label(ctx, labels)
        {
            return label;
        }

        return localized_number_string(value, &ctx.locale);
    }

    if let Some(formatter) = &props.format_value {
        return formatter(value);
    }

    if let Some(formatter) = &props.value_format {
        return formatter(value);
    }

    number_string(value)
}

fn localized_number_string(value: f64, locale: &Locale) -> String {
    Formatter::new(locale, FormatOptions::default()).format(value)
}

fn discrete_label(ctx: &Context, labels: &[String]) -> Option<String> {
    if labels.is_empty() {
        return None;
    }

    let max_index = labels.len() - 1;
    let percent = value_percent(ctx) / 100.0;
    let index = (percent * max_index as f64).round() as usize;
    let index = index.min(max_index);
    let label = &labels[index];

    Some(format!("{} of {} ({})", index + 1, labels.len(), label))
}

fn marker_label(props: &Props, value: f64) -> Option<String> {
    props
        .marks
        .iter()
        .find(|mark| (mark.value - value).abs() <= f64::EPSILON)
        .and_then(|mark| mark.label.clone())
        .or_else(|| props.tick_format.as_ref().map(|format| format(value)))
}

const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn percent_string(value: f64) -> String {
    format!("{}%", number_string(value))
}

fn number_string(value: f64) -> String {
    if value.fract().abs() <= f64::EPSILON {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use core::{
        cell::RefCell,
        hash::{Hash, Hasher},
    };
    use std::{
        collections::hash_map::DefaultHasher,
        sync::{Arc, Mutex},
    };

    use ars_core::{
        AriaAttr, AttrMap, Direction, Env, HtmlAttr, KeyboardKey, Orientation, Rect, Service,
        StrongSend, callback,
    };
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props {
            id: "volume".to_string(),
            min: 0.0,
            max: 100.0,
            step: 5.0,
            default_value: 25.0,
            ..Props::default()
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn initializes_uncontrolled_and_controlled_values() {
        let uncontrolled = service(props());

        assert_eq!(uncontrolled.state(), &State::Idle);
        assert_eq!(*uncontrolled.context().value.get(), 25.0);
        assert!(!uncontrolled.context().value.is_controlled());

        let controlled = service(Props {
            value: Some(45.0),
            default_value: 10.0,
            ..props()
        });

        assert_eq!(*controlled.context().value.get(), 45.0);
        assert!(controlled.context().value.is_controlled());
    }

    #[test]
    fn transitions_through_focus_dragging_and_idle() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.state(), &State::Focused);
        assert!(svc.context().focused);
        assert!(svc.context().focus_visible);

        drop(svc.send(Event::PointerDown { value: 42.0 }));

        assert_eq!(svc.state(), &State::Dragging);
        assert_eq!(*svc.context().value.get(), 40.0);
        assert!(svc.context().dragging);

        drop(svc.send(Event::PointerMove { value: 67.0 }));

        assert_eq!(*svc.context().value.get(), 65.0);

        drop(svc.send(Event::PointerUp));

        assert_eq!(svc.state(), &State::Focused);
        assert!(!svc.context().dragging);

        drop(svc.send(Event::Blur));

        assert_eq!(svc.state(), &State::Idle);
        assert!(!svc.context().focused);
    }

    #[test]
    fn pointer_move_is_ignored_when_not_dragging() {
        let mut svc = service(props());

        drop(svc.send(Event::PointerMove { value: 95.0 }));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(*svc.context().value.get(), 25.0);
    }

    #[test]
    fn non_finite_value_events_do_not_mutate_state() {
        let mut svc = service(props());

        for event in [
            Event::SetValue(f64::NAN),
            Event::SetValue(f64::INFINITY),
            Event::PointerDown {
                value: f64::NEG_INFINITY,
            },
        ] {
            let result = svc.send(event);

            assert!(result.pending_effects.is_empty());
            assert!(result.cancel_effects.is_empty());
            assert_eq!(svc.state(), &State::Idle);
            assert_eq!(*svc.context().value.get(), 25.0);
        }
    }

    #[test]
    fn disabled_and_readonly_suppress_value_changes() {
        for props in [
            Props {
                disabled: true,
                ..props()
            },
            Props {
                readonly: true,
                ..props()
            },
        ] {
            let mut svc = service(props);
            let initial = *svc.context().value.get();

            drop(svc.send(Event::Increment));
            drop(svc.send(Event::PointerDown { value: 80.0 }));
            drop(svc.send(Event::SetValue(90.0)));

            assert_eq!(*svc.context().value.get(), initial);
            assert_ne!(svc.state(), &State::Dragging);
        }
    }

    #[test]
    fn keyboard_steps_and_respects_rtl_only_for_horizontal_sliders() {
        let mut horizontal = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        let events = RefCell::new(Vec::new());

        {
            let send = |event| events.borrow_mut().push(event);

            let api = horizontal.connect(&send);

            api.on_thumb_keydown(KeyboardKey::ArrowRight, false);
        }

        drop(horizontal.send(events.borrow_mut().pop().expect("keyboard event")));

        assert_eq!(*horizontal.context().value.get(), 20.0);

        {
            let send = |event| events.borrow_mut().push(event);

            let api = horizontal.connect(&send);

            api.on_thumb_keydown(KeyboardKey::ArrowLeft, false);
        }

        drop(horizontal.send(events.borrow_mut().pop().expect("keyboard event")));

        assert_eq!(*horizontal.context().value.get(), 25.0);

        let mut vertical = service(Props {
            orientation: Orientation::Vertical,
            dir: Direction::Rtl,
            ..props()
        });

        let events = RefCell::new(Vec::new());

        {
            let send = |event| events.borrow_mut().push(event);

            let api = vertical.connect(&send);

            api.on_thumb_keydown(KeyboardKey::ArrowRight, false);
        }

        drop(vertical.send(events.borrow_mut().pop().expect("keyboard event")));

        assert_eq!(*vertical.context().value.get(), 30.0);
    }

    #[test]
    fn home_and_end_use_normalized_bounds() {
        let mut svc = service(Props {
            min: 100.0,
            max: 0.0,
            default_value: 50.0,
            ..props()
        });

        drop(svc.send(Event::SetToMin));
        assert_eq!(*svc.context().value.get(), 0.0);

        drop(svc.send(Event::SetToMax));
        assert_eq!(*svc.context().value.get(), 100.0);
    }

    #[test]
    fn connect_api_event_helpers_dispatch_expected_events() {
        let svc = service(props());

        let events = RefCell::new(Vec::new());

        {
            let send = |event| events.borrow_mut().push(event);

            let api = svc.connect(&send);

            api.on_thumb_focus(true);
            api.on_thumb_blur();
            api.on_thumb_keydown(KeyboardKey::ArrowRight, false);
            api.on_thumb_keydown(KeyboardKey::ArrowRight, true);
            api.on_thumb_keydown(KeyboardKey::ArrowLeft, false);
            api.on_thumb_keydown(KeyboardKey::ArrowLeft, true);
            api.on_thumb_keydown(KeyboardKey::ArrowUp, false);
            api.on_thumb_keydown(KeyboardKey::ArrowUp, true);
            api.on_thumb_keydown(KeyboardKey::ArrowDown, false);
            api.on_thumb_keydown(KeyboardKey::ArrowDown, true);
            api.on_thumb_keydown(KeyboardKey::PageUp, false);
            api.on_thumb_keydown(KeyboardKey::PageDown, false);
            api.on_thumb_keydown(KeyboardKey::Home, false);
            api.on_thumb_keydown(KeyboardKey::End, false);
            api.on_track_pointerdown(42.0);
        }

        assert_eq!(
            events.into_inner(),
            vec![
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::Increment,
                Event::IncrementLarge,
                Event::Decrement,
                Event::DecrementLarge,
                Event::Increment,
                Event::IncrementLarge,
                Event::Decrement,
                Event::DecrementLarge,
                Event::IncrementLarge,
                Event::DecrementLarge,
                Event::SetToMin,
                Event::SetToMax,
                Event::PointerDown { value: 42.0 },
            ]
        );
    }

    #[test]
    fn connect_api_event_helpers_dispatch_rtl_horizontal_keys() {
        let svc = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        let events = RefCell::new(Vec::new());

        {
            let send = |event| events.borrow_mut().push(event);

            let api = svc.connect(&send);

            api.on_thumb_keydown(KeyboardKey::ArrowRight, false);
            api.on_thumb_keydown(KeyboardKey::ArrowRight, true);
            api.on_thumb_keydown(KeyboardKey::ArrowLeft, false);
            api.on_thumb_keydown(KeyboardKey::ArrowLeft, true);
        }

        assert_eq!(
            events.into_inner(),
            vec![
                Event::Decrement,
                Event::DecrementLarge,
                Event::Increment,
                Event::IncrementLarge,
            ]
        );
    }

    #[test]
    fn pointer_geometry_maps_to_values_without_dom_lookup() {
        let svc = service(props());

        let track = Rect {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        };

        assert_eq!(
            value_from_pointer(SliderPointer { x: 110.0, y: 70.0 }, track, svc.context()),
            Some(50.0)
        );

        let shifted = service(Props {
            min: 20.0,
            max: 60.0,
            step: 1.0,
            default_value: 20.0,
            ..props()
        });

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 70.0 },
                track,
                shifted.context()
            ),
            Some(40.0)
        );

        let vertical = service(Props {
            orientation: Orientation::Vertical,
            ..props()
        });

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 45.0 },
                track,
                vertical.context()
            ),
            Some(75.0)
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 95.0 },
                track,
                vertical.context()
            ),
            Some(25.0)
        );

        let rtl = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        assert_eq!(
            value_from_pointer(SliderPointer { x: 60.0, y: 70.0 }, track, rtl.context()),
            Some(75.0)
        );

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    width: 0.0,
                    ..track
                },
                svc.context()
            ),
            None
        );
    }

    #[test]
    fn pointer_geometry_uses_normalized_bounds_and_rejects_non_finite_geometry() {
        let track = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
        };
        let reversed = service(Props {
            min: 100.0,
            max: 0.0,
            value: Some(50.0),
            ..props()
        });

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 20.0 },
                track,
                reversed.context()
            ),
            Some(0.0)
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 20.0 },
                track,
                reversed.context()
            ),
            Some(100.0)
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 50.0, y: 20.0 },
                Rect {
                    width: f64::NAN,
                    ..track
                },
                reversed.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer {
                    x: f64::INFINITY,
                    y: 20.0
                },
                track,
                reversed.context()
            ),
            None
        );
    }

    #[test]
    fn thumb_attrs_expose_slider_aria_and_discrete_value_text() {
        let svc = service(Props {
            discrete: true,
            value_labels: Some(vec!["Muted".into(), "Low".into(), "Medium".into()]),
            value: Some(50.0),
            ..props()
        });

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("50"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("2 of 3 (Low)")
        );
    }

    #[test]
    fn thumb_attrs_clamp_aria_value_now_to_current_bounds() {
        let mut svc = service(Props {
            value: Some(75.0),
            ..props()
        });

        drop(svc.set_props(Props {
            value: Some(75.0),
            max: 50.0,
            ..props()
        }));

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("50"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("50"));
    }

    #[test]
    fn thumb_attrs_reference_error_message_when_invalid() {
        let svc = service(Props {
            invalid: true,
            ..props()
        });

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("volume-error-message")
        );
    }

    #[test]
    fn thumb_attrs_normalize_reversed_aria_bounds() {
        let svc = service(Props {
            min: 100.0,
            max: 0.0,
            value: Some(75.0),
            ..props()
        });

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("75"));
    }

    #[test]
    fn thumb_attrs_clamp_aria_value_text_to_current_bounds() {
        let mut svc = service(Props {
            value: Some(75.0),
            format_value_text: Some(callback(|value: f64| format!("{value:.0} dB"))),
            ..props()
        });

        drop(svc.set_props(Props {
            value: Some(75.0),
            max: 50.0,
            format_value_text: Some(callback(|value: f64| format!("{value:.0} dB"))),
            ..props()
        }));

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("50"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("50 dB")
        );
    }

    #[test]
    fn discrete_value_text_fallback_uses_locale_decimal_formatting() {
        let svc = Service::<Machine>::new(
            Props {
                discrete: true,
                min: 0.0,
                max: 1.0,
                step: 0.1,
                value: Some(0.5),
                ..props()
            },
            &Env {
                locale: Locale::parse("de-DE").expect("valid locale"),
                ..Env::default()
            },
            &Messages::default(),
        );

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)), Some("0,5"));
    }

    #[test]
    fn thumb_attrs_only_reference_label_when_label_part_is_rendered() {
        let mut svc = service(props());

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Value"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));

        drop(svc.send(Event::SetHasLabel(true)));

        let attrs = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("volume-label")
        );
    }

    #[test]
    fn output_attrs_only_reference_label_when_label_part_is_rendered() {
        let mut svc = service(props());

        assert!(
            !svc.connect(&|_| {})
                .output_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::LabelledBy))
        );

        drop(svc.send(Event::SetHasLabel(true)));

        assert_eq!(
            svc.connect(&|_| {})
                .output_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("volume-label")
        );
    }

    #[test]
    fn label_attrs_do_not_point_for_at_non_labelable_thumb() {
        let svc = service(props());
        let attrs = svc.connect(&|_| {}).label_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("volume-label"));
        assert!(!attrs.contains(&HtmlAttr::For));
    }

    #[test]
    fn hidden_input_uses_form_metadata_and_current_value() {
        let svc = service(Props {
            name: Some("volume".into()),
            form: Some("settings".into()),
            value: Some(35.0),
            ..props()
        });

        let attrs = svc.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("volume"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("settings"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("35"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
    }

    #[test]
    fn hidden_input_is_disabled_and_uses_bounded_value() {
        let mut svc = service(Props {
            disabled: true,
            name: Some("volume".into()),
            value: Some(75.0),
            ..props()
        });

        drop(svc.set_props(Props {
            disabled: true,
            name: Some("volume".into()),
            value: Some(75.0),
            max: 50.0,
            ..props()
        }));

        let attrs = svc.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("50"));
    }

    #[test]
    fn value_change_callbacks_are_emitted_for_value_and_commit_events() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let ends = Arc::new(Mutex::new(Vec::new()));

        let captured_changes = Arc::clone(&changes);
        let captured_ends = Arc::clone(&ends);

        let mut svc = service(Props {
            on_value_change: Some(callback(move |value: f64| {
                captured_changes.lock().unwrap().push(value);
            })),
            on_value_change_end: Some(callback(move |value: f64| {
                captured_ends.lock().unwrap().push(value);
            })),
            ..props()
        });

        let mut increment = svc.send(Event::Increment);

        assert_eq!(increment.pending_effects.len(), 2);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in increment.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert_eq!(changes.lock().unwrap().as_slice(), &[30.0]);
        assert_eq!(ends.lock().unwrap().as_slice(), &[30.0]);

        let mut drag = svc.send(Event::PointerDown { value: 55.0 });

        assert_eq!(drag.pending_effects.len(), 1);

        for effect in drag.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        let mut end = svc.send(Event::PointerUp);

        assert_eq!(end.pending_effects.len(), 1);

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert_eq!(changes.lock().unwrap().as_slice(), &[30.0, 55.0]);
        assert_eq!(ends.lock().unwrap().as_slice(), &[30.0, 55.0]);
    }

    #[test]
    fn pointer_up_emits_pending_drag_value_for_controlled_slider() {
        let ends = Arc::new(Mutex::new(Vec::new()));
        let captured_ends = Arc::clone(&ends);
        let on_value_change_end = callback(move |value: f64| {
            captured_ends.lock().unwrap().push(value);
        });
        let mut svc = service(Props {
            value: Some(10.0),
            on_value_change_end: Some(on_value_change_end.clone()),
            ..props()
        });

        drop(svc.send(Event::PointerDown { value: 70.0 }));
        drop(svc.send(Event::PointerMove { value: 90.0 }));

        let mut end = svc.send(Event::PointerUp);
        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert_eq!(ends.lock().unwrap().as_slice(), &[90.0]);
    }

    #[test]
    fn non_finite_controlled_value_preserves_controlled_mode() {
        let mut svc = service(props());

        drop(svc.set_props(Props {
            value: Some(f64::NAN),
            ..props()
        }));

        assert!(svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), 25.0);
    }

    #[test]
    fn active_drag_commit_value_is_resnapped_after_props_change() {
        let ends = Arc::new(Mutex::new(Vec::new()));
        let captured_ends = Arc::clone(&ends);
        let on_value_change_end = callback(move |value: f64| {
            captured_ends.lock().unwrap().push(value);
        });
        let mut svc = service(Props {
            value: Some(10.0),
            on_value_change_end: Some(on_value_change_end.clone()),
            ..props()
        });

        drop(svc.send(Event::PointerDown { value: 90.0 }));
        drop(svc.set_props(Props {
            value: Some(10.0),
            max: 50.0,
            on_value_change_end: Some(on_value_change_end),
            ..props()
        }));

        let mut end = svc.send(Event::PointerUp);
        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert_eq!(ends.lock().unwrap().as_slice(), &[50.0]);
    }

    #[test]
    fn no_op_value_changes_do_not_emit_value_change_callbacks() {
        let mut svc = service(props());

        let same = svc.send(Event::SetValue(25.0));

        assert!(same.pending_effects.is_empty());

        let mut epsilon = service(Props {
            max: 1.0,
            step: f64::EPSILON,
            default_value: 0.0,
            ..props()
        });

        let epsilon_step = epsilon.send(Event::SetValue(f64::EPSILON));

        assert!(epsilon_step.pending_effects.is_empty());
        assert_eq!(*epsilon.context().value.get(), f64::EPSILON);
    }

    #[test]
    fn no_op_commit_events_do_not_emit_value_change_end_callbacks() {
        let mut svc = service(Props {
            value: Some(100.0),
            on_value_change_end: Some(callback(|_value: f64| {})),
            ..props()
        });

        let increment = svc.send(Event::Increment);

        assert!(increment.pending_effects.is_empty());
    }

    #[test]
    fn snap_to_step_returns_clamped_value_for_invalid_steps() {
        assert_eq!(snap_to_step(12.0, 0.0, 10.0, 0.0), 10.0);
        assert_eq!(snap_to_step(7.0, 0.0, 10.0, -1.0), 7.0);
        assert_eq!(snap_to_step(7.0, 0.0, 10.0, f64::INFINITY), 7.0);
        assert_eq!(snap_to_step(-2.0, 0.0, 10.0, f64::NAN), 0.0);
    }

    #[test]
    fn snap_to_step_normalizes_invalid_bounds_without_panicking() {
        assert_eq!(snap_to_step(75.0, 100.0, 0.0, 5.0), 75.0);
        assert_eq!(snap_to_step(75.0, f64::NAN, 100.0, 5.0), 100.0);
        assert_eq!(snap_to_step(75.0, f64::NAN, f64::NAN, 5.0), 0.0);
    }

    #[test]
    fn large_step_defaults_to_ten_steps() {
        let mut svc = service(props());

        drop(svc.send(Event::IncrementLarge));

        assert_eq!(*svc.context().value.get(), 75.0);
    }

    #[test]
    fn value_percent_handles_equal_and_non_zero_ranges() {
        let equal = service(Props {
            min: 10.0,
            max: 10.0,
            value: Some(10.0),
            ..props()
        });

        assert_eq!(value_percent(equal.context()), 0.0);

        let shifted = service(Props {
            min: 20.0,
            max: 60.0,
            value: Some(40.0),
            ..props()
        });

        assert_eq!(value_percent(shifted.context()), 50.0);

        let signed = service(Props {
            min: -25.0,
            max: 75.0,
            value: Some(25.0),
            ..props()
        });

        assert_eq!(value_percent(signed.context()), 50.0);
    }

    #[test]
    fn value_percent_uses_normalized_bounds_and_bounded_value() {
        let reversed = service(Props {
            min: 100.0,
            max: 0.0,
            value: Some(25.0),
            ..props()
        });

        assert_eq!(value_percent(reversed.context()), 25.0);

        let mut clamped = service(Props {
            value: Some(75.0),
            ..props()
        });

        drop(clamped.set_props(Props {
            value: Some(75.0),
            max: 50.0,
            ..props()
        }));

        assert_eq!(value_percent(clamped.context()), 100.0);
    }

    #[test]
    fn range_style_mirrors_horizontal_rtl_start_and_end_origins() {
        let start = service(Props {
            dir: Direction::Rtl,
            value: Some(25.0),
            ..props()
        });

        assert_eq!(
            range_style(start.context()),
            (
                CssProperty::Left,
                "75%".into(),
                CssProperty::Width,
                "25%".into()
            )
        );

        let end = service(Props {
            dir: Direction::Rtl,
            origin: Origin::End,
            value: Some(25.0),
            ..props()
        });

        assert_eq!(
            range_style(end.context()),
            (
                CssProperty::Left,
                "0%".into(),
                CssProperty::Width,
                "75%".into()
            )
        );
    }

    #[test]
    fn number_string_formats_integers_and_fractional_values() {
        assert_eq!(number_string(25.0), "25");
        assert_eq!(number_string(25.5), "25.5");
    }

    #[test]
    fn controlled_value_syncs_from_props_and_can_return_to_uncontrolled() {
        let mut svc = service(Props {
            value: Some(40.0),
            ..props()
        });

        assert_eq!(*svc.context().value.get(), 40.0);
        assert!(svc.context().value.is_controlled());

        drop(svc.set_props(Props {
            value: Some(65.0),
            ..props()
        }));

        assert_eq!(*svc.context().value.get(), 65.0);
        assert!(svc.context().value.is_controlled());

        drop(svc.set_props(props()));

        assert!(!svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), 65.0);
    }

    #[test]
    fn controlled_bounds_changes_reclamp_pending_uncontrolled_value() {
        let mut svc = service(Props {
            value: Some(75.0),
            ..props()
        });

        drop(svc.set_props(Props {
            value: Some(75.0),
            max: 50.0,
            ..props()
        }));

        assert_eq!(*svc.context().value.get(), 75.0);
        assert!(svc.context().value.is_controlled());

        drop(svc.set_props(Props {
            max: 50.0,
            ..props()
        }));

        assert!(!svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), 50.0);
    }

    #[test]
    fn output_affecting_prop_changes_are_detected_individually() {
        let base = props();
        let mut changed = base.clone();

        assert!(!props_output_changed(&base, &changed));

        changed.min = f64::EPSILON;

        assert!(!props_output_changed(&base, &changed));
        assert!(props_output_changed(
            &Props {
                min: 1.0,
                ..base.clone()
            },
            &Props {
                min: -1.0,
                ..base.clone()
            }
        ));

        changed = base.clone();
        changed.min = 1.0;

        assert!(props_output_changed(&base, &changed));
        assert!(props_output_changed(
            &Props {
                min: f64::NAN,
                ..base.clone()
            },
            &Props {
                min: 0.0,
                ..base.clone()
            }
        ));

        changed = base.clone();
        changed.max = 99.0;

        assert!(props_output_changed(&base, &changed));
        assert!(props_output_changed(
            &Props {
                max: f64::NAN,
                ..base.clone()
            },
            &Props {
                max: 100.0,
                ..base.clone()
            }
        ));
        assert!(!props_output_changed(
            &Props {
                max: 0.0,
                ..base.clone()
            },
            &Props {
                max: f64::EPSILON,
                ..base.clone()
            }
        ));
        assert!(props_output_changed(
            &Props {
                min: -5.0,
                max: 1.0,
                ..base.clone()
            },
            &Props {
                min: -5.0,
                max: -1.0,
                ..base.clone()
            }
        ));

        changed = base.clone();
        changed.step = 10.0;

        assert!(props_output_changed(&base, &changed));
        assert!(props_output_changed(
            &Props {
                step: f64::NAN,
                ..base.clone()
            },
            &Props {
                step: 5.0,
                ..base.clone()
            }
        ));
        assert!(!props_output_changed(
            &Props {
                step: 0.0,
                ..base.clone()
            },
            &Props {
                step: f64::EPSILON,
                ..base.clone()
            }
        ));
        assert!(props_output_changed(
            &Props {
                step: 1.0,
                ..base.clone()
            },
            &Props {
                step: -1.0,
                ..base.clone()
            }
        ));

        changed = base.clone();
        changed.large_step = Some(25.0);

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.disabled = true;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.readonly = true;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.invalid = true;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.orientation = Orientation::Vertical;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.dir = Direction::Rtl;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.origin = Origin::Center;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.name = Some("volume".into());

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.form = Some("settings".into());

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.marks = vec![Mark {
            value: 25.0,
            label: Some("Quarter".into()),
        }];

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.discrete = true;

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.value_labels = Some(vec!["Low".into()]);

        assert!(props_output_changed(&base, &changed));

        changed = base.clone();
        changed.thumb_alignment = ThumbAlignment::Center;

        assert!(props_output_changed(&base, &changed));
    }

    #[test]
    fn part_equality_hash_and_dispatch_cover_marker_values() {
        assert_ne!(Part::Root, Part::Label);
        assert_eq!(Part::Marker { value: 25.0 }, Part::Marker { value: 25.0 });
        assert_ne!(Part::Marker { value: 25.0 }, Part::Marker { value: -25.0 });

        let mut left = DefaultHasher::new();

        Part::Marker { value: 25.0 }.hash(&mut left);

        let mut right = DefaultHasher::new();

        Part::Marker { value: -25.0 }.hash(&mut right);

        assert_ne!(left.finish(), right.finish());

        let svc = service(props());

        let api = svc.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Thumb), api.thumb_attrs());
        assert_eq!(
            api.part_attrs(Part::Marker { value: 25.0 }),
            api.marker_attrs(25.0)
        );
    }

    #[test]
    fn marker_attrs_compare_against_effective_bounded_value() {
        let mut svc = service(Props {
            value: Some(75.0),
            ..props()
        });

        drop(svc.set_props(Props {
            value: Some(75.0),
            max: 50.0,
            ..props()
        }));

        let api = svc.connect(&|_| {});

        assert!(
            api.marker_attrs(50.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            !api.marker_attrs(75.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
    }

    #[test]
    fn marker_attrs_follow_range_origin() {
        let start = service(Props {
            value: Some(40.0),
            ..props()
        });
        let api = start.connect(&|_| {});

        assert!(
            api.marker_attrs(25.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            !api.marker_attrs(75.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );

        let end = service(Props {
            value: Some(40.0),
            origin: Origin::End,
            ..props()
        });
        let api = end.connect(&|_| {});

        assert!(
            !api.marker_attrs(25.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            api.marker_attrs(75.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );

        let center_low = service(Props {
            value: Some(25.0),
            origin: Origin::Center,
            ..props()
        });
        let api = center_low.connect(&|_| {});

        assert!(
            api.marker_attrs(25.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            api.marker_attrs(50.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            !api.marker_attrs(75.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );

        let center_high = service(Props {
            value: Some(75.0),
            origin: Origin::Center,
            ..props()
        });
        let api = center_high.connect(&|_| {});

        assert!(
            !api.marker_attrs(25.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            api.marker_attrs(50.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            api.marker_attrs(75.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
    }

    #[test]
    fn slider_root_idle_snapshot() {
        let svc = service(props());

        assert_snapshot!(snapshot_attrs(&svc.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn slider_root_focused_dragging_disabled_invalid_snapshot() {
        let mut svc = service(Props {
            disabled: true,
            invalid: true,
            readonly: true,
            ..props()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        svc.context_mut().dragging = true;

        assert_snapshot!(snapshot_attrs(&svc.connect(&|_| {}).root_attrs()));
    }

    #[test]
    fn slider_label_track_output_description_error_snapshots() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        assert_snapshot!("slider_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!("slider_track", snapshot_attrs(&api.track_attrs()));
        assert_snapshot!("slider_output", snapshot_attrs(&api.output_attrs()));
        assert_snapshot!(
            "slider_description",
            snapshot_attrs(&api.description_attrs())
        );
        assert_snapshot!(
            "slider_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
    }

    #[test]
    fn slider_range_origin_and_direction_snapshots() {
        for (name, props) in [
            ("slider_range_start_ltr", props()),
            (
                "slider_range_start_rtl",
                Props {
                    dir: Direction::Rtl,
                    ..props()
                },
            ),
            (
                "slider_range_center",
                Props {
                    origin: Origin::Center,
                    ..props()
                },
            ),
            (
                "slider_range_end",
                Props {
                    origin: Origin::End,
                    ..props()
                },
            ),
            (
                "slider_range_vertical",
                Props {
                    orientation: Orientation::Vertical,
                    ..props()
                },
            ),
        ] {
            let svc = service(props);

            assert_snapshot!(name, snapshot_attrs(&svc.connect(&|_| {}).range_attrs()));
        }
    }

    #[test]
    fn slider_thumb_state_and_format_snapshots() {
        let svc = service(props());

        assert_snapshot!(
            "slider_thumb_ltr",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs())
        );

        let rtl = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        assert_snapshot!(
            "slider_thumb_rtl",
            snapshot_attrs(&rtl.connect(&|_| {}).thumb_attrs())
        );

        let vertical = service(Props {
            orientation: Orientation::Vertical,
            thumb_alignment: ThumbAlignment::Center,
            ..props()
        });

        assert_snapshot!(
            "slider_thumb_vertical_center",
            snapshot_attrs(&vertical.connect(&|_| {}).thumb_attrs())
        );

        let described = {
            let mut svc = service(Props {
                disabled: true,
                readonly: true,
                invalid: true,
                ..props()
            });

            drop(svc.send(Event::Focus { is_keyboard: true }));
            drop(svc.send(Event::SetHasDescription(true)));

            svc
        };

        assert_snapshot!(
            "slider_thumb_disabled_readonly_invalid_described",
            snapshot_attrs(&described.connect(&|_| {}).thumb_attrs())
        );

        let formatted = service(Props {
            value: Some(40.0),
            format_value_text: Some(callback(|value: f64| format!("{value:.0} dB"))),
            ..props()
        });

        assert_snapshot!(
            "slider_thumb_format_value_text",
            snapshot_attrs(&formatted.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn slider_marker_hidden_and_dragging_indicator_snapshots() {
        let mut svc = service(Props {
            marks: vec![
                Mark {
                    value: 10.0,
                    label: Some("Low".into()),
                },
                Mark {
                    value: 90.0,
                    label: None,
                },
            ],
            tick_format: Some(callback(|value: f64| format!("{value:.0}%"))),
            name: Some("volume".into()),
            form: Some("settings".into()),
            ..props()
        });

        assert_snapshot!(
            "slider_marker_in_range_labelled",
            snapshot_attrs(&svc.connect(&|_| {}).marker_attrs(10.0))
        );
        assert_snapshot!(
            "slider_marker_out_of_range_formatted",
            snapshot_attrs(&svc.connect(&|_| {}).marker_attrs(90.0))
        );
        assert_snapshot!(
            "slider_marker_group",
            snapshot_attrs(&svc.connect(&|_| {}).marker_group_attrs())
        );
        assert_snapshot!(
            "slider_hidden_input",
            snapshot_attrs(&svc.connect(&|_| {}).hidden_input_attrs())
        );
        assert_snapshot!(
            "slider_dragging_indicator_idle",
            snapshot_attrs(&svc.connect(&|_| {}).dragging_indicator_attrs())
        );

        drop(svc.send(Event::PointerDown { value: 40.0 }));

        assert_snapshot!(
            "slider_dragging_indicator_active",
            snapshot_attrs(&svc.connect(&|_| {}).dragging_indicator_attrs())
        );
    }
}
