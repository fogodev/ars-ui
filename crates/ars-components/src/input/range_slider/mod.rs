//! RangeSlider component state machine and connect API.

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

use super::slider::{SliderPointer, ThumbAlignment};

/// Type of the `Props::format_value` callback.
type ValueFormatter = dyn Fn((f64, f64)) -> String + Send + Sync;

/// Type of the `Props::on_value_change[_end]` callback.
type ValueChanger = dyn Fn([f64; 2]) + Send + Sync;

/// Identifies one of the two range-slider thumbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThumbIndex {
    /// The start thumb controls the lower value.
    Start,

    /// The end thumb controls the upper value.
    End,
}

impl ThumbIndex {
    const fn index(self) -> usize {
        match self {
            Self::Start => 0,
            Self::End => 1,
        }
    }

    const fn other(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::End => Self::Start,
        }
    }

    const fn token(self) -> &'static str {
        match self {
            Self::Start => "0",
            Self::End => "1",
        }
    }
}

/// The state of the `RangeSlider` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle.
    Idle,

    /// One thumb is focused.
    Focused {
        /// The focused thumb.
        thumb: ThumbIndex,
    },

    /// One thumb is being dragged.
    Dragging {
        /// The dragged thumb.
        thumb: ThumbIndex,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused { .. } => "focused",
            Self::Dragging { .. } => "dragging",
        })
    }
}

/// Events accepted by the `RangeSlider` state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// A thumb received focus.
    Focus {
        /// The focused thumb.
        thumb: ThumbIndex,

        /// Whether focus was initiated by keyboard navigation.
        is_keyboard: bool,
    },

    /// A thumb lost focus.
    Blur {
        /// The blurred thumb.
        thumb: ThumbIndex,
    },

    /// A pointer gesture started on a thumb with an adapter-supplied value.
    PointerDown {
        /// The active thumb.
        thumb: ThumbIndex,

        /// Raw logical value computed by the adapter or helper.
        value: f64,
    },

    /// A pointer gesture moved to an adapter-supplied logical value.
    PointerMove {
        /// Raw logical value computed by the adapter or helper.
        value: f64,
    },

    /// The active pointer gesture ended.
    PointerUp,

    /// Increment a thumb by [`Context::step`].
    Increment {
        /// Thumb to increment.
        thumb: ThumbIndex,
    },

    /// Decrement a thumb by [`Context::step`].
    Decrement {
        /// Thumb to decrement.
        thumb: ThumbIndex,
    },

    /// Increment a thumb by [`Context::large_step`] or ten steps.
    IncrementLarge {
        /// Thumb to increment.
        thumb: ThumbIndex,
    },

    /// Decrement a thumb by [`Context::large_step`] or ten steps.
    DecrementLarge {
        /// Thumb to decrement.
        thumb: ThumbIndex,
    },

    /// Set a thumb to the minimum representable value.
    SetToMin {
        /// Thumb to set.
        thumb: ThumbIndex,
    },

    /// Set a thumb to the maximum representable value.
    SetToMax {
        /// Thumb to set.
        thumb: ThumbIndex,
    },

    /// Programmatically set both values.
    SetValues([f64; 2]),

    /// Synchronize the externally controlled value prop.
    SyncValue(Option<[f64; 2]>),

    /// Synchronize output-affecting props stored in context.
    SetProps,

    /// Track whether the description part is rendered.
    SetHasDescription(bool),

    /// Track whether the label part is rendered.
    SetHasLabel(bool),
}

/// Internal context for the `RangeSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current `[start, end]` range value, controlled or uncontrolled.
    pub value: Bindable<[f64; 2]>,

    /// Minimum allowed value.
    pub min: f64,

    /// Maximum allowed value.
    pub max: f64,

    /// Step size for normal increments.
    pub step: f64,

    /// Optional large step for PageUp/PageDown and Shift+Arrow.
    pub large_step: Option<f64>,

    /// Minimum number of steps that must separate the thumbs when representable.
    pub min_steps_between: u32,

    /// Whether dragging past the opposite thumb swaps active thumb identity.
    pub allow_thumb_swap: bool,

    /// Whether the entire range slider is disabled.
    pub disabled: bool,

    /// Whether the range slider is read-only.
    pub readonly: bool,

    /// Whether the range slider is invalid.
    pub invalid: bool,

    /// Range-slider layout orientation.
    pub orientation: Orientation,

    /// Text direction used for horizontal keyboard and visual mirroring.
    pub dir: Direction,

    /// The thumb that currently has focus.
    pub focused_thumb: Option<ThumbIndex>,

    /// Whether focus should be visibly indicated.
    pub focus_visible: bool,

    /// The thumb currently being dragged.
    pub dragging_thumb: Option<ThumbIndex>,

    /// Pending drag value used for controlled commit callbacks.
    pub drag_value: Option<[f64; 2]>,

    /// Whether the active drag changed the effective value.
    pub drag_changed: bool,

    /// How thumbs align with track boundaries.
    pub thumb_alignment: ThumbAlignment,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Associated form id for hidden input submission.
    pub form: Option<String>,

    /// Whether the start thumb is individually disabled.
    pub start_disabled: bool,

    /// Whether the end thumb is individually disabled.
    pub end_disabled: bool,

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

/// Props for the `RangeSlider` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base id for the range slider.
    pub id: String,

    /// Controlled range value.
    pub value: Option<[f64; 2]>,

    /// Default range value for uncontrolled mode.
    pub default_value: [f64; 2],

    /// Minimum allowed value.
    pub min: f64,

    /// Maximum allowed value.
    pub max: f64,

    /// Step size for normal increments.
    pub step: f64,

    /// Optional large step for PageUp/PageDown and Shift+Arrow.
    pub large_step: Option<f64>,

    /// Minimum number of steps that must separate thumbs when representable.
    pub min_steps_between: u32,

    /// Whether the entire range slider is disabled.
    pub disabled: bool,

    /// Whether the range slider is read-only.
    pub readonly: bool,

    /// Whether the range slider is invalid.
    pub invalid: bool,

    /// Range-slider layout orientation.
    pub orientation: Orientation,

    /// Text direction used for horizontal keyboard and visual mirroring.
    pub dir: Direction,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Associated form id for hidden input submission.
    pub form: Option<String>,

    /// Whether dragging past the opposite thumb swaps active thumb identity.
    pub allow_thumb_swap: bool,

    /// Whether the start thumb is individually disabled.
    pub start_disabled: bool,

    /// Whether the end thumb is individually disabled.
    pub end_disabled: bool,

    /// Formatter for thumb `aria-valuetext`; receives `(this_value, other_value)`.
    pub format_value: Option<Callback<ValueFormatter>>,

    /// How thumbs align with track boundaries.
    pub thumb_alignment: ThumbAlignment,

    /// Callback fired when value-changing user intent requests a new range.
    pub on_value_change: Option<Callback<ValueChanger>>,

    /// Callback fired when a drag or keyboard adjustment is committed.
    pub on_value_change_end: Option<Callback<ValueChanger>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: [0.0, 100.0],
            min: 0.0,
            max: 100.0,
            step: 1.0,
            large_step: None,
            min_steps_between: 0,
            disabled: false,
            readonly: false,
            invalid: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            name: None,
            form: None,
            allow_thumb_swap: false,
            start_disabled: false,
            end_disabled: false,
            format_value: None,
            thumb_alignment: ThumbAlignment::Contain,
            on_value_change: None,
            on_value_change_end: None,
        }
    }
}

impl Props {
    /// Returns default range-slider props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Localized messages for the `RangeSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible fallback label for the start thumb.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible fallback label for the end thumb.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            start_label: MessageFn::static_str("Range start"),
            end_label: MessageFn::static_str("Range end"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for side effects emitted by the range-slider machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke [`Props::on_value_change`].
    ValueChange,

    /// Invoke [`Props::on_value_change_end`].
    ValueChangeEnd,
}

/// Machine for the `RangeSlider` component.
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

    fn init(props: &Props, env: &Env, messages: &Messages) -> (State, Context) {
        let initial = normalize_values(
            props.value.unwrap_or(props.default_value),
            props.min,
            props.max,
            props.step,
            props.min_steps_between,
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
                min_steps_between: props.min_steps_between,
                allow_thumb_swap: props.allow_thumb_swap,
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                orientation: props.orientation,
                dir: props.dir,
                focused_thumb: None,
                focus_visible: false,
                dragging_thumb: None,
                drag_value: None,
                drag_changed: false,
                thumb_alignment: props.thumb_alignment,
                name: props.name.clone(),
                form: props.form.clone(),
                start_disabled: props.start_disabled,
                end_disabled: props.end_disabled,
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
                | Event::Increment { .. }
                | Event::Decrement { .. }
                | Event::IncrementLarge { .. }
                | Event::DecrementLarge { .. }
                | Event::SetToMin { .. }
                | Event::SetToMax { .. }
                | Event::SetValues(_) => return None,
                _ => {}
            }
        }

        match event {
            Event::Focus { thumb, is_keyboard } => {
                if ctx.disabled {
                    return None;
                }

                let thumb = *thumb;
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::to(State::Focused { thumb }).apply(
                    move |ctx: &mut Context| {
                        ctx.focused_thumb = Some(thumb);
                        ctx.focus_visible = is_keyboard;
                    },
                ))
            }

            Event::Blur { thumb } => {
                let thumb = *thumb;
                let plan = if ctx.dragging_thumb.is_some() && ctx.dragging_thumb != Some(thumb) {
                    TransitionPlan::new()
                } else {
                    TransitionPlan::to(State::Idle)
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    if ctx.focused_thumb == Some(thumb) {
                        ctx.focused_thumb = None;
                    }

                    ctx.focus_visible = false;

                    if ctx.dragging_thumb == Some(thumb) {
                        ctx.dragging_thumb = None;
                        ctx.drag_value = None;
                        ctx.drag_changed = false;
                    }
                }))
            }

            Event::PointerDown { thumb, value } => {
                if thumb_disabled(ctx, *thumb) || !value.is_finite() {
                    return None;
                }

                Some(
                    set_thumb_value_plan(
                        ctx,
                        *thumb,
                        *value,
                        Some(State::Dragging { thumb: *thumb }),
                        true,
                        false,
                        true,
                    )
                    .apply(|ctx: &mut Context| {
                        ctx.dragging_thumb = Some(match ctx.dragging_thumb {
                            Some(thumb) => thumb,
                            None => ThumbIndex::Start,
                        });
                    }),
                )
            }

            Event::PointerMove { value } => {
                let thumb = ctx.dragging_thumb?;

                if thumb_disabled(ctx, thumb) || !value.is_finite() {
                    return None;
                }

                Some(set_thumb_value_plan(
                    ctx, thumb, *value, None, true, false, true,
                ))
            }

            Event::PointerUp => {
                ctx.dragging_thumb?;

                let target = if let Some(thumb) = ctx.focused_thumb {
                    State::Focused { thumb }
                } else {
                    State::Idle
                };

                let value = ctx.drag_value.unwrap_or_else(|| bounded_values(ctx));

                let mut plan = TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.dragging_thumb = None;
                    ctx.drag_value = None;
                    ctx.drag_changed = false;
                });

                if ctx.drag_changed {
                    plan = plan.with_effect(value_change_end_effect(value));
                }

                Some(plan)
            }

            Event::Increment { thumb } => step_thumb_plan(ctx, *thumb, ctx.step, true),

            Event::Decrement { thumb } => step_thumb_plan(ctx, *thumb, ctx.step, false),

            Event::IncrementLarge { thumb } => step_thumb_plan(ctx, *thumb, large_step(ctx), true),

            Event::DecrementLarge { thumb } => step_thumb_plan(ctx, *thumb, large_step(ctx), false),

            Event::SetToMin { thumb } => {
                if thumb_disabled(ctx, *thumb) {
                    return None;
                }

                let (min, _) = normalized_bounds(ctx.min, ctx.max);

                Some(set_thumb_value_plan(
                    ctx, *thumb, min, None, true, true, false,
                ))
            }

            Event::SetToMax { thumb } => {
                if thumb_disabled(ctx, *thumb) {
                    return None;
                }

                let (_, max) = normalized_bounds(ctx.min, ctx.max);

                Some(set_thumb_value_plan(
                    ctx, *thumb, max, None, true, true, false,
                ))
            }

            Event::SetValues(values) => {
                if values.iter().any(|value| !value.is_finite()) {
                    return None;
                }

                Some(set_values_plan(ctx, *values, None, true, true))
            }

            Event::SyncValue(value) => match value {
                Some(value) if value.iter().all(|value| value.is_finite()) => {
                    let value =
                        normalize_values(*value, ctx.min, ctx.max, ctx.step, ctx.min_steps_between);
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.value.set(value);
                        ctx.value.sync_controlled(Some(value));
                    }))
                }

                Some(_) => {
                    let value = bounded_values(ctx);
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
                    ctx.min_steps_between = props.min_steps_between;
                    ctx.allow_thumb_swap = props.allow_thumb_swap;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.orientation = props.orientation;
                    ctx.dir = props.dir;
                    ctx.thumb_alignment = props.thumb_alignment;
                    ctx.name = props.name.clone();
                    ctx.form = props.form.clone();
                    ctx.start_disabled = props.start_disabled;
                    ctx.end_disabled = props.end_disabled;

                    let value = normalize_values(
                        *ctx.value.get(),
                        ctx.min,
                        ctx.max,
                        ctx.step,
                        ctx.min_steps_between,
                    );

                    let controlled = ctx.value.is_controlled();

                    ctx.value.set(value);

                    if controlled {
                        ctx.value.sync_controlled(Some(value));
                    }

                    if ctx.dragging_thumb.is_some() {
                        let drag_value = normalize_values(
                            ctx.drag_value.unwrap_or(value),
                            ctx.min,
                            ctx.max,
                            ctx.step,
                            ctx.min_steps_between,
                        );

                        ctx.drag_changed |= values_changed(drag_value, value);
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
            "range_slider::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
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

/// Structural parts exposed by the `RangeSlider` connect API.
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

    /// The filled range between thumbs.
    Range,

    /// An interactive thumb.
    Thumb {
        /// Which thumb the part represents.
        thumb: ThumbIndex,
    },

    /// Optional value output.
    Output,

    /// Optional marker group.
    MarkerGroup,

    /// A marker for a specific value.
    Marker {
        /// The marker value.
        value: f64,
    },

    /// Hidden input used for one submitted range value.
    HiddenInput {
        /// Which thumb the hidden input represents.
        thumb: ThumbIndex,
    },

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
            (
                Self::Thumb { thumb: left } | Self::HiddenInput { thumb: left },
                Self::Thumb { thumb: right } | Self::HiddenInput { thumb: right },
            ) => core::mem::discriminant(self) == core::mem::discriminant(other) && left == right,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Part {}

impl Hash for Part {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Thumb { thumb } | Self::HiddenInput { thumb } => thumb.hash(state),
            Self::Marker { value } => value.to_bits().hash(state),
            _ => {}
        }
    }
}

impl ComponentPart for Part {
    const ROOT: Self = Self::Root;

    fn scope() -> &'static str {
        "range-slider"
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Label => "label",
            Self::Track => "track",
            Self::Range => "range",
            Self::Thumb { .. } => "thumb",
            Self::Output => "output",
            Self::MarkerGroup => "marker-group",
            Self::Marker { .. } => "marker",
            Self::HiddenInput { .. } => "hidden-input",
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
            Self::Thumb {
                thumb: ThumbIndex::Start,
            },
            Self::Output,
            Self::MarkerGroup,
            Self::Marker { value: 0.0 },
            Self::HiddenInput {
                thumb: ThumbIndex::Start,
            },
            Self::DraggingIndicator,
            Self::Description,
            Self::ErrorMessage,
        ]
    }
}

/// Connect API for the `RangeSlider` component.
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
            Part::Thumb { thumb } => self.thumb_attrs(thumb),
            Part::Output => self.output_attrs(),
            Part::MarkerGroup => self.marker_group_attrs(),
            Part::Marker { value } => self.marker_attrs(value),
            Part::HiddenInput { thumb } => self.hidden_input_attrs(thumb),
            Part::DraggingIndicator => self.dragging_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Sends a focus event for a thumb.
    pub fn on_thumb_focus(&self, thumb: ThumbIndex, is_keyboard: bool) {
        (self.send)(Event::Focus { thumb, is_keyboard });
    }

    /// Sends a blur event for a thumb.
    pub fn on_thumb_blur(&self, thumb: ThumbIndex) {
        (self.send)(Event::Blur { thumb });
    }

    /// Handles thumb keydown intent.
    pub fn on_thumb_keydown(&self, thumb: ThumbIndex, key: KeyboardKey, shift: bool) {
        let is_rtl_horizontal =
            self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl;

        let event = match key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowLeft => {
                let moves_forward = matches!(key, KeyboardKey::ArrowRight) ^ is_rtl_horizontal;

                match (moves_forward, shift) {
                    (true, true) => Event::IncrementLarge { thumb },
                    (true, false) => Event::Increment { thumb },
                    (false, true) => Event::DecrementLarge { thumb },
                    (false, false) => Event::Decrement { thumb },
                }
            }

            KeyboardKey::ArrowUp if shift => Event::IncrementLarge { thumb },

            KeyboardKey::ArrowUp => Event::Increment { thumb },

            KeyboardKey::ArrowDown if shift => Event::DecrementLarge { thumb },

            KeyboardKey::ArrowDown => Event::Decrement { thumb },

            KeyboardKey::PageUp => Event::IncrementLarge { thumb },

            KeyboardKey::PageDown => Event::DecrementLarge { thumb },

            KeyboardKey::Home => Event::SetToMin { thumb },

            KeyboardKey::End => Event::SetToMax { thumb },

            _ => return,
        };

        (self.send)(event);
    }

    /// Sends track pointerdown intent with an adapter-supplied value.
    pub fn on_track_pointerdown(&self, thumb: ThumbIndex, value: f64) {
        (self.send)(Event::PointerDown { thumb, value });
    }

    /// Sends track pointermove intent with an adapter-supplied value.
    pub fn on_track_pointermove(&self, value: f64) {
        (self.send)(Event::PointerMove { value });
    }

    /// Sends pointerup intent for the active pointer gesture.
    pub fn on_pointerup(&self) {
        (self.send)(Event::PointerUp);
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

        if let State::Focused { thumb } | State::Dragging { thumb } = self.state {
            attrs.set(HtmlAttr::Data("ars-active-thumb"), thumb.token());
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.ctx.dragging_thumb.is_some() {
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

    /// Returns attributes for the filled range between thumbs.
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

    /// Returns attributes for a thumb.
    #[must_use]
    pub fn thumb_attrs(&self, thumb: ThumbIndex) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb { thumb }.data_attrs();

        let values = bounded_values(self.ctx);

        let value = values[thumb.index()];
        let other = values[thumb.other().index()];

        let (position_prop, position) = thumb_style(self.ctx, thumb);

        let (min, max) = thumb_aria_bounds(self.ctx, thumb);

        let is_thumb_disabled = thumb_disabled(self.ctx, thumb);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part(thumb_part_id(thumb)))
            .set(HtmlAttr::Role, "slider")
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), number_string(min))
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), number_string(max))
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), number_string(value))
            .set(
                HtmlAttr::Aria(AriaAttr::ValueText),
                value_text(self.props, value, other),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.ctx.orientation),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                match thumb {
                    ThumbIndex::Start => (self.ctx.messages.start_label)(&self.ctx.locale),
                    ThumbIndex::End => (self.ctx.messages.end_label)(&self.ctx.locale),
                },
            )
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(HtmlAttr::Data("ars-index"), thumb.token())
            .set_style(position_prop, position)
            .set(
                HtmlAttr::Data("ars-thumb-alignment"),
                match self.ctx.thumb_alignment {
                    ThumbAlignment::Center => "center",
                    ThumbAlignment::Contain => "contain",
                },
            )
            .set(HtmlAttr::Class, "ars-touch-none");

        if is_thumb_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true").set(
                HtmlAttr::Aria(AriaAttr::ErrorMessage),
                self.ctx.ids.part("error-message"),
            );
        }

        if self.ctx.has_description {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }

        if self.ctx.focus_visible && self.ctx.focused_thumb == Some(thumb) {
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
            .set(
                HtmlAttr::For,
                format!(
                    "{} {}",
                    self.ctx.ids.part(thumb_part_id(ThumbIndex::Start)),
                    self.ctx.ids.part(thumb_part_id(ThumbIndex::End))
                ),
            )
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

        attrs
    }

    /// Returns attributes for one hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self, thumb: ThumbIndex) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::HiddenInput { thumb }.data_attrs();

        let values = bounded_values(self.ctx);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Value, number_string(values[thumb.index()]));

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, format!("{name}[{}]", thumb.index()));
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
                if self.ctx.dragging_thumb.is_some() {
                    "dragging"
                } else {
                    "idle"
                },
            )
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if self.ctx.dragging_thumb.is_none() {
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

/// Computes a range-slider value from adapter-supplied pointer and track geometry.
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
        || old.min_steps_between != new.min_steps_between
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.orientation != new.orientation
        || old.dir != new.dir
        || old.name != new.name
        || old.form != new.form
        || old.allow_thumb_swap != new.allow_thumb_swap
        || old.start_disabled != new.start_disabled
        || old.end_disabled != new.end_disabled
        || old.thumb_alignment != new.thumb_alignment
}

fn float_output_changed(old: f64, new: f64) -> bool {
    if old.is_nan() || new.is_nan() {
        return old.to_bits() != new.to_bits();
    }

    (old - new).abs() > f64::EPSILON
}

fn step_thumb_plan(
    ctx: &Context,
    thumb: ThumbIndex,
    step: f64,
    up: bool,
) -> Option<TransitionPlan<Machine>> {
    if thumb_disabled(ctx, thumb) {
        return None;
    }

    let current = thumb_value(ctx, thumb);

    let raw = if up { current + step } else { current - step };

    Some(set_thumb_value_plan(
        ctx, thumb, raw, None, true, true, false,
    ))
}

fn set_thumb_value_plan(
    ctx: &Context,
    thumb: ThumbIndex,
    raw: f64,
    target: Option<State>,
    value_change: bool,
    value_change_end: bool,
    track_drag: bool,
) -> TransitionPlan<Machine> {
    let (next_thumb, next) = set_thumb_value(ctx, thumb, raw, track_drag);

    let previous = if track_drag {
        ctx.drag_value.unwrap_or_else(|| bounded_values(ctx))
    } else {
        bounded_values(ctx)
    };

    let changed = values_changed(next, previous);

    let target = if track_drag {
        Some(State::Dragging { thumb: next_thumb })
    } else {
        target
    };

    let mut plan = if let Some(target) = target {
        TransitionPlan::to(target)
    } else {
        TransitionPlan::new()
    }
    .apply(move |ctx: &mut Context| {
        ctx.value.set(next);

        if track_drag {
            ctx.dragging_thumb = Some(next_thumb);
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

fn set_values_plan(
    ctx: &Context,
    values: [f64; 2],
    target: Option<State>,
    value_change: bool,
    value_change_end: bool,
) -> TransitionPlan<Machine> {
    let next = normalize_values(values, ctx.min, ctx.max, ctx.step, ctx.min_steps_between);
    let previous = bounded_values(ctx);

    let changed = values_changed(next, previous);

    let mut plan = if let Some(target) = target {
        TransitionPlan::to(target)
    } else {
        TransitionPlan::new()
    }
    .apply(move |ctx: &mut Context| {
        ctx.value.set(next);
    });

    if value_change && changed {
        plan = plan.with_effect(value_change_effect(next));
    }

    if value_change_end && changed {
        plan = plan.with_effect(value_change_end_effect(next));
    }

    plan
}

fn value_change_effect(value: [f64; 2]) -> PendingEffect<Machine> {
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

fn value_change_end_effect(value: [f64; 2]) -> PendingEffect<Machine> {
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

fn set_thumb_value(
    ctx: &Context,
    thumb: ThumbIndex,
    raw: f64,
    track_drag: bool,
) -> (ThumbIndex, [f64; 2]) {
    let mut active = thumb;

    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    let gap = effective_gap(ctx);

    let snapped = snap_to_step(raw, min, max, ctx.step);

    let [start, end] = if track_drag {
        ctx.drag_value.unwrap_or_else(|| bounded_values(ctx))
    } else {
        bounded_values(ctx)
    };

    if track_drag && ctx.allow_thumb_swap && !thumb_disabled(ctx, thumb.other()) {
        match thumb {
            ThumbIndex::Start if snapped > end => active = ThumbIndex::End,
            ThumbIndex::End if snapped < start => active = ThumbIndex::Start,
            _ => {}
        }
    }

    let next = match active {
        ThumbIndex::Start => [snapped.clamp(min, end - gap), end],
        ThumbIndex::End => [start, snapped.clamp(start + gap, max)],
    };

    (
        active,
        normalize_values(next, ctx.min, ctx.max, ctx.step, ctx.min_steps_between),
    )
}

fn normalize_values(
    values: [f64; 2],
    min: f64,
    max: f64,
    step: f64,
    min_steps_between: u32,
) -> [f64; 2] {
    let (min, max) = normalized_bounds(min, max);

    let mut start = snap_to_step(values[0], min, max, step);
    let mut end = snap_to_step(values[1], min, max, step);

    if start > end {
        core::mem::swap(&mut start, &mut end);
    }

    let gap = effective_gap_for(min, max, step, min_steps_between);

    if end - start < gap {
        let preferred_end = (start + gap).min(max);

        if preferred_end - start >= gap {
            end = preferred_end;
        } else {
            start = (end - gap).max(min);
        }
    }

    [start.clamp(min, max), end.clamp(min, max)]
}

const fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
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

const fn normalized_bounds(min: f64, max: f64) -> (f64, f64) {
    match (min.is_finite(), max.is_finite()) {
        (true, true) if min <= max => (min, max),
        (true, true) => (max, min),
        (true, false) => (min, min),
        (false, true) => (max, max),
        (false, false) => (0.0, 0.0),
    }
}

fn bounded_values(ctx: &Context) -> [f64; 2] {
    normalize_values(
        *ctx.value.get(),
        ctx.min,
        ctx.max,
        ctx.step,
        ctx.min_steps_between,
    )
}

fn thumb_value(ctx: &Context, thumb: ThumbIndex) -> f64 {
    bounded_values(ctx)[thumb.index()]
}

#[cfg(test)]
fn other_thumb_value(ctx: &Context, thumb: ThumbIndex) -> f64 {
    bounded_values(ctx)[thumb.other().index()]
}

fn effective_gap(ctx: &Context) -> f64 {
    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    effective_gap_for(min, max, ctx.step, ctx.min_steps_between)
}

fn effective_gap_for(min: f64, max: f64, step: f64, min_steps_between: u32) -> f64 {
    if step <= 0.0 || !step.is_finite() {
        return 0.0;
    }

    (step * f64::from(min_steps_between))
        .min(max - min)
        .max(0.0)
}

const fn thumb_disabled(ctx: &Context, thumb: ThumbIndex) -> bool {
    ctx.disabled
        || match thumb {
            ThumbIndex::Start => ctx.start_disabled,
            ThumbIndex::End => ctx.end_disabled,
        }
}

fn large_step(ctx: &Context) -> f64 {
    ctx.large_step.unwrap_or(ctx.step * 10.0)
}

const fn values_changed(left: [f64; 2], right: [f64; 2]) -> bool {
    (left[0] - right[0]).abs() > f64::EPSILON || (left[1] - right[1]).abs() > f64::EPSILON
}

fn thumb_aria_bounds(ctx: &Context, thumb: ThumbIndex) -> (f64, f64) {
    let [start, end] = bounded_values(ctx);
    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    let gap = effective_gap(ctx);

    match thumb {
        ThumbIndex::Start => (min, end - gap),
        ThumbIndex::End => (start + gap, max),
    }
}

const fn value_percent(ctx: &Context, value: f64) -> f64 {
    let (min, max) = normalized_bounds(ctx.min, ctx.max);

    if (max - min).abs() <= f64::EPSILON {
        return 0.0;
    }

    ((value - min) / (max - min)).clamp(0.0, 1.0) * 100.0
}

fn range_style(ctx: &Context) -> (CssProperty, String, CssProperty, String) {
    let [start, end] = bounded_values(ctx);

    let start_percent = value_percent(ctx, start);
    let end_percent = value_percent(ctx, end);

    let is_horizontal = ctx.orientation == Orientation::Horizontal;

    let (start, size) = if is_horizontal && ctx.dir == Direction::Rtl {
        (100.0 - end_percent, end_percent - start_percent)
    } else {
        (start_percent, end_percent - start_percent)
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

fn thumb_style(ctx: &Context, thumb: ThumbIndex) -> (CssProperty, String) {
    let value = thumb_value(ctx, thumb);
    let percent = value_percent(ctx, value);

    match ctx.orientation {
        Orientation::Horizontal if ctx.dir == Direction::Rtl => {
            (CssProperty::Right, percent_string(percent))
        }

        Orientation::Horizontal => (CssProperty::Left, percent_string(percent)),

        Orientation::Vertical => (CssProperty::Bottom, percent_string(percent)),
    }
}

fn marker_in_range(ctx: &Context, value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }

    let [start, end] = bounded_values(ctx);

    value >= start && value <= end
}

fn value_text(props: &Props, value: f64, other: f64) -> String {
    props
        .format_value
        .as_ref()
        .map_or_else(|| number_string(value), |format| format((value, other)))
}

const fn thumb_part_id(thumb: ThumbIndex) -> &'static str {
    match thumb {
        ThumbIndex::Start => "thumb-start",
        ThumbIndex::End => "thumb-end",
    }
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
    use core::hash::{Hash, Hasher};
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
            id: "price".to_string(),
            min: 0.0,
            max: 100.0,
            step: 5.0,
            default_value: [25.0, 75.0],
            ..Props::default()
        }
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn run_effects(effects: &mut Vec<PendingEffect<Machine>>, ctx: &Context, props: &Props) {
        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in effects.drain(..) {
            drop(effect.run(ctx, props, Arc::clone(&send)));
        }
    }

    #[test]
    fn part_marker_and_thumb_identity_are_hashable() {
        assert_eq!(
            Part::Marker { value: f64::NAN },
            Part::Marker { value: f64::NAN }
        );
        assert_ne!(Part::Marker { value: 0.0 }, Part::Marker { value: 1.0 });
        assert_ne!(
            Part::Thumb {
                thumb: ThumbIndex::Start,
            },
            Part::Thumb {
                thumb: ThumbIndex::End,
            }
        );
        assert_ne!(
            Part::HiddenInput {
                thumb: ThumbIndex::Start,
            },
            Part::HiddenInput {
                thumb: ThumbIndex::End,
            }
        );

        let mut left = DefaultHasher::new();
        let mut right = DefaultHasher::new();

        Part::Marker { value: f64::NAN }.hash(&mut left);
        Part::Marker { value: f64::NAN }.hash(&mut right);

        assert_eq!(left.finish(), right.finish());

        let mut thumb_start = DefaultHasher::new();
        let mut thumb_end = DefaultHasher::new();
        let mut input_start = DefaultHasher::new();
        let mut marker = DefaultHasher::new();
        let mut marker_other = DefaultHasher::new();

        Part::Thumb {
            thumb: ThumbIndex::Start,
        }
        .hash(&mut thumb_start);
        Part::Thumb {
            thumb: ThumbIndex::End,
        }
        .hash(&mut thumb_end);
        Part::HiddenInput {
            thumb: ThumbIndex::Start,
        }
        .hash(&mut input_start);
        Part::Marker { value: 0.0 }.hash(&mut marker);
        Part::Marker { value: 1.0 }.hash(&mut marker_other);

        assert_ne!(thumb_start.finish(), thumb_end.finish());
        assert_ne!(thumb_start.finish(), input_start.finish());
        assert_ne!(thumb_start.finish(), marker.finish());
        assert_ne!(marker.finish(), marker_other.finish());
    }

    #[test]
    fn initializes_uncontrolled_and_controlled_values() {
        assert_eq!(Props::new(), Props::default());
        assert_eq!(
            State::Focused {
                thumb: ThumbIndex::Start
            }
            .to_string(),
            "focused"
        );
        assert_eq!(
            State::Dragging {
                thumb: ThumbIndex::End
            }
            .to_string(),
            "dragging"
        );

        let uncontrolled = service(props());

        assert_eq!(uncontrolled.state(), &State::Idle);
        assert_eq!(*uncontrolled.context().value.get(), [25.0, 75.0]);
        assert!(!uncontrolled.context().value.is_controlled());

        let controlled = service(Props {
            value: Some([70.0, 20.0]),
            default_value: [10.0, 90.0],
            ..props()
        });

        assert_eq!(*controlled.context().value.get(), [20.0, 70.0]);
        assert!(controlled.context().value.is_controlled());

        let non_finite = service(Props {
            default_value: [f64::NAN, f64::INFINITY],
            ..props()
        });

        assert_eq!(*non_finite.context().value.get(), [0.0, 0.0]);

        let reversed_bounds = service(Props {
            min: 100.0,
            max: 0.0,
            default_value: [85.0, 15.0],
            ..props()
        });

        assert_eq!(*reversed_bounds.context().value.get(), [15.0, 85.0]);
    }

    #[test]
    fn transitions_through_focus_dragging_and_idle() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            thumb: ThumbIndex::End,
            is_keyboard: true,
        }));

        assert_eq!(
            svc.state(),
            &State::Focused {
                thumb: ThumbIndex::End,
            }
        );
        assert_eq!(svc.context().focused_thumb, Some(ThumbIndex::End));
        assert!(svc.context().focus_visible);

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 84.0,
        }));

        assert_eq!(
            svc.state(),
            &State::Dragging {
                thumb: ThumbIndex::End,
            }
        );
        assert_eq!(*svc.context().value.get(), [25.0, 85.0]);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));

        drop(svc.send(Event::PointerMove { value: 64.0 }));

        assert_eq!(*svc.context().value.get(), [25.0, 65.0]);

        drop(svc.send(Event::PointerUp));

        assert_eq!(
            svc.state(),
            &State::Focused {
                thumb: ThumbIndex::End,
            }
        );
        assert_eq!(svc.context().dragging_thumb, None);

        drop(svc.send(Event::Blur {
            thumb: ThumbIndex::End,
        }));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(svc.context().focused_thumb, None);
    }

    #[test]
    fn pointer_move_is_ignored_when_not_dragging() {
        let mut svc = service(props());

        drop(svc.send(Event::PointerMove { value: 95.0 }));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(*svc.context().value.get(), [25.0, 75.0]);
    }

    #[test]
    fn unrelated_blur_does_not_cancel_active_drag() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            thumb: ThumbIndex::Start,
            is_keyboard: true,
        }));
        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 80.0,
        }));
        drop(svc.send(Event::Blur {
            thumb: ThumbIndex::Start,
        }));

        assert_eq!(svc.context().focused_thumb, None);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));
        assert!(svc.context().drag_changed);

        drop(svc.send(Event::PointerMove { value: 90.0 }));

        assert_eq!(*svc.context().value.get(), [25.0, 90.0]);
        assert_eq!(
            svc.state(),
            &State::Dragging {
                thumb: ThumbIndex::End,
            }
        );
    }

    #[test]
    fn disabled_readonly_and_per_thumb_disabled_suppress_value_changes() {
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

            drop(svc.send(Event::Increment {
                thumb: ThumbIndex::Start,
            }));
            drop(svc.send(Event::PointerDown {
                thumb: ThumbIndex::End,
                value: 90.0,
            }));
            drop(svc.send(Event::SetValues([0.0, 100.0])));

            assert_eq!(*svc.context().value.get(), initial);
            assert_ne!(
                svc.state(),
                &State::Dragging {
                    thumb: ThumbIndex::End,
                }
            );
        }

        let mut svc = service(Props {
            start_disabled: true,
            ..props()
        });

        drop(svc.send(Event::Increment {
            thumb: ThumbIndex::Start,
        }));
        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 50.0,
        }));

        assert_eq!(*svc.context().value.get(), [25.0, 75.0]);

        drop(svc.send(Event::Increment {
            thumb: ThumbIndex::End,
        }));

        assert_eq!(*svc.context().value.get(), [25.0, 80.0]);
    }

    #[test]
    fn non_finite_value_events_do_not_mutate_state() {
        let mut svc = service(props());

        for event in [
            Event::SetValues([f64::NAN, 50.0]),
            Event::PointerDown {
                thumb: ThumbIndex::Start,
                value: f64::INFINITY,
            },
            Event::PointerMove {
                value: f64::NEG_INFINITY,
            },
        ] {
            let result = svc.send(event);

            assert!(result.pending_effects.is_empty());
            assert!(result.cancel_effects.is_empty());
            assert_eq!(*svc.context().value.get(), [25.0, 75.0]);
        }
    }

    #[test]
    fn crossing_prevention_and_min_gap_are_enforced() {
        let mut svc = service(Props {
            min_steps_between: 2,
            ..props()
        });

        drop(svc.send(Event::SetValues([75.0, 25.0])));

        assert_eq!(*svc.context().value.get(), [25.0, 75.0]);

        drop(svc.send(Event::SetValues([75.0, 75.0])));

        assert_eq!(*svc.context().value.get(), [75.0, 85.0]);

        drop(svc.send(Event::SetToMax {
            thumb: ThumbIndex::Start,
        }));

        assert_eq!(*svc.context().value.get(), [75.0, 85.0]);

        drop(svc.send(Event::SetToMin {
            thumb: ThumbIndex::End,
        }));

        assert_eq!(*svc.context().value.get(), [75.0, 85.0]);

        drop(svc.send(Event::Decrement {
            thumb: ThumbIndex::End,
        }));

        assert_eq!(*svc.context().value.get(), [75.0, 85.0]);

        let no_change = svc.send(Event::Decrement {
            thumb: ThumbIndex::End,
        });

        assert!(no_change.pending_effects.is_empty());
    }

    #[test]
    fn clamp_mode_drag_past_stops_at_opposite_thumb() {
        let mut svc = service(Props {
            min_steps_between: 1,
            ..props()
        });

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 90.0,
        }));

        assert_eq!(*svc.context().value.get(), [70.0, 75.0]);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::Start));

        let mut end = service(Props {
            min_steps_between: 1,
            ..props()
        });

        drop(end.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 10.0,
        }));

        assert_eq!(*end.context().value.get(), [25.0, 30.0]);
        assert_eq!(end.context().dragging_thumb, Some(ThumbIndex::End));
    }

    #[test]
    fn swap_mode_drag_past_changes_active_thumb() {
        let mut svc = service(Props {
            allow_thumb_swap: true,
            ..props()
        });

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 80.0,
        }));

        assert_eq!(*svc.context().value.get(), [25.0, 80.0]);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));
        assert_eq!(
            svc.state(),
            &State::Dragging {
                thumb: ThumbIndex::End,
            }
        );

        drop(svc.send(Event::PointerMove { value: 90.0 }));

        assert_eq!(*svc.context().value.get(), [25.0, 90.0]);

        let mut equal = service(Props {
            allow_thumb_swap: true,
            ..props()
        });

        drop(equal.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 75.0,
        }));

        assert_eq!(*equal.context().value.get(), [75.0, 75.0]);
        assert_eq!(equal.context().dragging_thumb, Some(ThumbIndex::Start));

        let mut reverse = service(Props {
            allow_thumb_swap: true,
            ..props()
        });

        drop(reverse.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 20.0,
        }));

        assert_eq!(*reverse.context().value.get(), [20.0, 75.0]);
        assert_eq!(reverse.context().dragging_thumb, Some(ThumbIndex::Start));

        let mut end_equal = service(Props {
            allow_thumb_swap: true,
            ..props()
        });

        drop(end_equal.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 25.0,
        }));

        assert_eq!(*end_equal.context().value.get(), [25.0, 25.0]);
        assert_eq!(end_equal.context().dragging_thumb, Some(ThumbIndex::End));
    }

    #[test]
    fn controlled_swap_drag_continues_from_pending_drag_value() {
        let ends = Arc::new(Mutex::new(Vec::new()));
        let captured_ends = Arc::clone(&ends);
        let on_value_change_end = callback(move |value: [f64; 2]| {
            captured_ends.lock().unwrap().push(value);
        });

        let mut svc = service(Props {
            allow_thumb_swap: true,
            value: Some([10.0, 20.0]),
            on_value_change_end: Some(on_value_change_end),
            ..props()
        });

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 80.0,
        }));

        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));
        assert_eq!(svc.context().drag_value, Some([10.0, 80.0]));

        drop(svc.send(Event::PointerMove { value: 90.0 }));

        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));
        assert_eq!(svc.context().drag_value, Some([10.0, 90.0]));

        let mut end = svc.send(Event::PointerUp);

        run_effects(&mut end.pending_effects, svc.context(), svc.props());

        assert_eq!(ends.lock().unwrap().as_slice(), &[[10.0, 90.0]]);
    }

    #[test]
    fn thumb_swap_does_not_move_into_disabled_thumb() {
        let mut svc = service(Props {
            allow_thumb_swap: true,
            start_disabled: true,
            ..props()
        });

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 20.0,
        }));

        assert_eq!(*svc.context().value.get(), [25.0, 25.0]);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::End));
        assert_eq!(
            svc.state(),
            &State::Dragging {
                thumb: ThumbIndex::End,
            }
        );
    }

    #[test]
    fn pointer_move_stops_after_active_thumb_becomes_disabled() {
        let mut svc = service(props());

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::Start,
            value: 30.0,
        }));

        drop(svc.set_props(Props {
            start_disabled: true,
            ..props()
        }));

        drop(svc.send(Event::PointerMove { value: 40.0 }));

        assert_eq!(*svc.context().value.get(), [30.0, 75.0]);
        assert_eq!(svc.context().dragging_thumb, Some(ThumbIndex::Start));
    }

    #[test]
    fn keyboard_navigation_updates_requested_thumb() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&sent);
        let send = move |event| captured.lock().unwrap().push(event);

        let svc = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        let api = svc.connect(&send);

        api.on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowRight, false);
        api.on_thumb_keydown(ThumbIndex::End, KeyboardKey::ArrowLeft, true);
        api.on_thumb_keydown(ThumbIndex::End, KeyboardKey::Home, false);

        assert_eq!(
            sent.lock().unwrap().as_slice(),
            &[
                Event::Decrement {
                    thumb: ThumbIndex::Start,
                },
                Event::IncrementLarge {
                    thumb: ThumbIndex::End,
                },
                Event::SetToMin {
                    thumb: ThumbIndex::End,
                },
            ]
        );

        sent.lock().unwrap().clear();

        let vertical = service(Props {
            dir: Direction::Rtl,
            orientation: Orientation::Vertical,
            ..props()
        });

        let captured = Arc::clone(&sent);
        let send = move |event| captured.lock().unwrap().push(event);

        vertical
            .connect(&send)
            .on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowRight, false);

        assert_eq!(
            sent.lock().unwrap().as_slice(),
            &[Event::Increment {
                thumb: ThumbIndex::Start,
            }]
        );
    }

    #[test]
    fn api_event_handlers_dispatch_every_public_intent() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&sent);
        let send = move |event| captured.lock().unwrap().push(event);

        let svc = service(props());

        let api = svc.connect(&send);

        api.on_thumb_focus(ThumbIndex::Start, true);
        api.on_thumb_blur(ThumbIndex::End);
        api.on_track_pointerdown(ThumbIndex::End, 42.0);
        api.on_track_pointermove(55.0);
        api.on_pointerup();
        api.on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowUp, false);
        api.on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowUp, true);
        api.on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowDown, false);
        api.on_thumb_keydown(ThumbIndex::Start, KeyboardKey::ArrowDown, true);
        api.on_thumb_keydown(ThumbIndex::End, KeyboardKey::PageUp, false);
        api.on_thumb_keydown(ThumbIndex::End, KeyboardKey::PageDown, false);
        api.on_thumb_keydown(ThumbIndex::End, KeyboardKey::End, false);

        assert_eq!(
            sent.lock().unwrap().as_slice(),
            &[
                Event::Focus {
                    thumb: ThumbIndex::Start,
                    is_keyboard: true,
                },
                Event::Blur {
                    thumb: ThumbIndex::End,
                },
                Event::PointerDown {
                    thumb: ThumbIndex::End,
                    value: 42.0,
                },
                Event::PointerMove { value: 55.0 },
                Event::PointerUp,
                Event::Increment {
                    thumb: ThumbIndex::Start,
                },
                Event::IncrementLarge {
                    thumb: ThumbIndex::Start,
                },
                Event::Decrement {
                    thumb: ThumbIndex::Start,
                },
                Event::DecrementLarge {
                    thumb: ThumbIndex::Start,
                },
                Event::IncrementLarge {
                    thumb: ThumbIndex::End,
                },
                Event::DecrementLarge {
                    thumb: ThumbIndex::End,
                },
                Event::SetToMax {
                    thumb: ThumbIndex::End,
                },
            ]
        );
    }

    #[test]
    fn connect_part_attrs_dispatches_each_part() {
        let svc = service(Props {
            name: Some("price".into()),
            ..props()
        });

        let api = svc.connect(&|_| {});

        for part in [
            Part::Root,
            Part::Label,
            Part::Track,
            Part::Range,
            Part::Thumb {
                thumb: ThumbIndex::Start,
            },
            Part::Thumb {
                thumb: ThumbIndex::End,
            },
            Part::Output,
            Part::MarkerGroup,
            Part::Marker { value: 50.0 },
            Part::HiddenInput {
                thumb: ThumbIndex::Start,
            },
            Part::HiddenInput {
                thumb: ThumbIndex::End,
            },
            Part::DraggingIndicator,
            Part::Description,
            Part::ErrorMessage,
        ] {
            let attrs = api.part_attrs(part.clone());

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some(part.name()));
            assert!(
                !attrs.attrs().is_empty() || !attrs.styles().is_empty(),
                "part_attrs({part:?}) must not be empty"
            );
        }
    }

    #[test]
    fn value_change_callbacks_are_emitted_for_value_and_commit_events() {
        let changes = Arc::new(Mutex::new(Vec::new()));
        let ends = Arc::new(Mutex::new(Vec::new()));

        let captured_changes = Arc::clone(&changes);
        let captured_ends = Arc::clone(&ends);

        let mut svc = service(Props {
            on_value_change: Some(callback(move |value: [f64; 2]| {
                captured_changes.lock().unwrap().push(value);
            })),
            on_value_change_end: Some(callback(move |value: [f64; 2]| {
                captured_ends.lock().unwrap().push(value);
            })),
            ..props()
        });

        let mut increment = svc.send(Event::Increment {
            thumb: ThumbIndex::Start,
        });

        assert_eq!(increment.pending_effects.len(), 2);

        run_effects(&mut increment.pending_effects, svc.context(), svc.props());

        assert_eq!(changes.lock().unwrap().as_slice(), &[[30.0, 75.0]]);
        assert_eq!(ends.lock().unwrap().as_slice(), &[[30.0, 75.0]]);

        let mut drag = svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 55.0,
        });

        assert_eq!(drag.pending_effects.len(), 1);

        run_effects(&mut drag.pending_effects, svc.context(), svc.props());

        let mut end = svc.send(Event::PointerUp);

        assert_eq!(end.pending_effects.len(), 1);

        run_effects(&mut end.pending_effects, svc.context(), svc.props());

        assert_eq!(
            changes.lock().unwrap().as_slice(),
            &[[30.0, 75.0], [30.0, 55.0]]
        );
        assert_eq!(
            ends.lock().unwrap().as_slice(),
            &[[30.0, 75.0], [30.0, 55.0]]
        );
    }

    #[test]
    fn set_values_and_drag_set_props_preserve_pending_commit_state() {
        let mut svc = service(props());

        let changed = svc.send(Event::SetValues([10.0, 90.0]));

        assert_eq!(*svc.context().value.get(), [10.0, 90.0]);
        assert_eq!(changed.pending_effects.len(), 2);

        let mut dragging = service(Props {
            value: Some([10.0, 20.0]),
            ..props()
        });

        drop(dragging.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 70.0,
        }));

        assert!(dragging.context().drag_changed);

        drop(dragging.set_props(Props {
            value: Some([10.0, 20.0]),
            min: 0.0,
            max: 80.0,
            ..props()
        }));

        assert!(dragging.context().drag_changed);
        assert_eq!(dragging.context().drag_value, Some([10.0, 70.0]));

        let mut unchanged_drag = service(props());

        drop(unchanged_drag.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 80.0,
        }));

        assert!(unchanged_drag.context().drag_changed);

        drop(unchanged_drag.set_props(Props {
            name: Some("price".into()),
            ..props()
        }));

        assert!(unchanged_drag.context().drag_changed);
        assert_eq!(unchanged_drag.context().drag_value, Some([25.0, 80.0]));

        let mut unchanged_set_values = service(props());

        drop(unchanged_set_values.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 80.0,
        }));

        assert!(unchanged_set_values.context().drag_changed);

        drop(unchanged_set_values.send(Event::SetValues([25.0, 80.0])));

        assert!(unchanged_set_values.context().drag_changed);
        assert_eq!(
            unchanged_set_values.context().drag_value,
            Some([25.0, 80.0])
        );
    }

    #[test]
    fn pointer_up_emits_pending_drag_value_for_controlled_range_slider() {
        let ends = Arc::new(Mutex::new(Vec::new()));
        let captured_ends = Arc::clone(&ends);
        let on_value_change_end = callback(move |value: [f64; 2]| {
            captured_ends.lock().unwrap().push(value);
        });

        let mut svc = service(Props {
            value: Some([10.0, 20.0]),
            on_value_change_end: Some(on_value_change_end.clone()),
            ..props()
        });

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 70.0,
        }));
        drop(svc.send(Event::PointerMove { value: 90.0 }));

        let mut end = svc.send(Event::PointerUp);

        run_effects(&mut end.pending_effects, svc.context(), svc.props());

        assert_eq!(ends.lock().unwrap().as_slice(), &[[10.0, 90.0]]);
    }

    #[test]
    fn no_op_value_changes_do_not_emit_callbacks() {
        let mut svc = service(props());

        let same = svc.send(Event::SetValues([25.0, 75.0]));

        assert!(same.pending_effects.is_empty());

        let increment_at_gap = svc.send(Event::Decrement {
            thumb: ThumbIndex::End,
        });

        assert_eq!(*svc.context().value.get(), [25.0, 70.0]);
        assert!(!increment_at_gap.pending_effects.is_empty());
    }

    #[test]
    fn set_props_and_sync_value_resnap_controlled_values() {
        let mut svc = service(props());

        drop(svc.set_props(Props {
            value: Some([21.0, 79.0]),
            ..props()
        }));

        assert!(svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), [20.0, 80.0]);

        drop(svc.set_props(Props {
            value: Some([21.0, 79.0]),
            max: 50.0,
            ..props()
        }));

        assert_eq!(*svc.context().value.get(), [20.0, 50.0]);

        drop(svc.set_props(Props {
            value: Some([f64::NAN, 90.0]),
            max: 50.0,
            ..props()
        }));

        assert!(svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), [20.0, 50.0]);
    }

    #[test]
    fn set_props_applies_new_bounds_before_syncing_controlled_value() {
        let mut svc = service(Props {
            value: Some([20.0, 80.0]),
            max: 100.0,
            ..props()
        });

        drop(svc.set_props(Props {
            value: Some([120.0, 180.0]),
            max: 200.0,
            ..props()
        }));

        assert!(svc.context().value.is_controlled());
        assert_eq!(*svc.context().value.get(), [120.0, 180.0]);
    }

    #[test]
    fn props_output_changed_detects_each_output_affecting_prop() {
        let base = props();

        let cases = [
            Props {
                min: -10.0,
                ..base.clone()
            },
            Props {
                max: 90.0,
                ..base.clone()
            },
            Props {
                step: 10.0,
                ..base.clone()
            },
            Props {
                large_step: Some(25.0),
                ..base.clone()
            },
            Props {
                min_steps_between: 2,
                ..base.clone()
            },
            Props {
                disabled: true,
                ..base.clone()
            },
            Props {
                readonly: true,
                ..base.clone()
            },
            Props {
                invalid: true,
                ..base.clone()
            },
            Props {
                orientation: Orientation::Vertical,
                ..base.clone()
            },
            Props {
                dir: Direction::Rtl,
                ..base.clone()
            },
            Props {
                name: Some("range".into()),
                ..base.clone()
            },
            Props {
                form: Some("filters".into()),
                ..base.clone()
            },
            Props {
                allow_thumb_swap: true,
                ..base.clone()
            },
            Props {
                start_disabled: true,
                ..base.clone()
            },
            Props {
                end_disabled: true,
                ..base.clone()
            },
            Props {
                thumb_alignment: ThumbAlignment::Center,
                ..base.clone()
            },
        ];

        for changed in cases {
            assert!(
                props_output_changed(&base, &changed),
                "props_output_changed must detect {changed:?}"
            );
        }

        let callback_only = Props {
            on_value_change: Some(callback(|_: [f64; 2]| {})),
            on_value_change_end: Some(callback(|_: [f64; 2]| {})),
            format_value: Some(callback(|(value, _): (f64, f64)| value.to_string())),
            ..base.clone()
        };

        assert!(!props_output_changed(&base, &callback_only));
        assert!(!float_output_changed(1.0, 1.0));
        assert!(float_output_changed(1.0, 1.0 + f64::EPSILON * 2.0));
        assert!(!float_output_changed(1.0, 1.0 + f64::EPSILON));
        assert!(float_output_changed(f64::NAN, 1.0));
        assert!(!float_output_changed(f64::NAN, f64::NAN));
        assert!(float_output_changed(
            f64::NAN,
            f64::from_bits(f64::NAN.to_bits() + 1)
        ));
    }

    #[test]
    fn thumb_attrs_expose_coordinated_slider_aria() {
        let mut svc = service(Props {
            min_steps_between: 2,
            name: Some("price".into()),
            form: Some("filters".into()),
            ..props()
        });

        drop(svc.send(Event::Focus {
            thumb: ThumbIndex::Start,
            is_keyboard: true,
        }));
        drop(svc.send(Event::SetHasLabel(true)));
        drop(svc.send(Event::SetHasDescription(true)));

        let api = svc.connect(&|_| {});
        let start = api.thumb_attrs(ThumbIndex::Start);
        let end = api.thumb_attrs(ThumbIndex::End);

        assert_eq!(start.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(start.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(start.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("65"));
        assert_eq!(start.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("25"));
        assert_eq!(start.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(start.get(&HtmlAttr::Data("ars-index")), Some("0"));
        assert!(!start.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
        assert!(start.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));
        assert!(start.contains(&HtmlAttr::Data("ars-focus-visible")));
        assert!(!end.contains(&HtmlAttr::Data("ars-focus-visible")));

        assert_eq!(end.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("35"));
        assert_eq!(end.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
        assert_eq!(end.get(&HtmlAttr::TabIndex), Some("0"));
        assert!(!end.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));

        let hidden_start = api.hidden_input_attrs(ThumbIndex::Start);
        let hidden_end = api.hidden_input_attrs(ThumbIndex::End);

        assert_eq!(hidden_start.get(&HtmlAttr::Name), Some("price[0]"));
        assert_eq!(hidden_start.get(&HtmlAttr::Value), Some("25"));
        assert_eq!(hidden_start.get(&HtmlAttr::Form), Some("filters"));
        assert!(!hidden_start.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
        assert_eq!(hidden_end.get(&HtmlAttr::Name), Some("price[1]"));
        assert_eq!(hidden_end.get(&HtmlAttr::Value), Some("75"));
        assert!(!hidden_end.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));

        let whole_disabled = service(Props {
            disabled: true,
            ..props()
        });
        let disabled_attrs = whole_disabled
            .connect(&|_| {})
            .thumb_attrs(ThumbIndex::Start);

        assert_eq!(disabled_attrs.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn both_enabled_thumbs_remain_in_tab_sequence() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus {
            thumb: ThumbIndex::Start,
            is_keyboard: true,
        }));

        let api = svc.connect(&|_| {});

        assert_eq!(
            api.thumb_attrs(ThumbIndex::Start).get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            api.thumb_attrs(ThumbIndex::End).get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn thumb_accessible_names_remain_distinct_when_label_is_present() {
        let mut svc = service(props());

        drop(svc.send(Event::SetHasLabel(true)));

        let api = svc.connect(&|_| {});
        let start = api.thumb_attrs(ThumbIndex::Start);
        let end = api.thumb_attrs(ThumbIndex::End);

        assert_eq!(
            start.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Range start")
        );
        assert_eq!(end.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Range end"));
        assert!(!start.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
        assert!(!end.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
    }

    #[test]
    fn per_thumb_disabled_remains_focusable_when_roving() {
        let mut svc = service(Props {
            start_disabled: true,
            ..props()
        });

        drop(svc.send(Event::Focus {
            thumb: ThumbIndex::Start,
            is_keyboard: true,
        }));

        let attrs = svc.connect(&|_| {}).thumb_attrs(ThumbIndex::Start);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn marker_and_dragging_indicator_attrs_reflect_range_state() {
        let mut svc = service(props());
        let api = svc.connect(&|_| {});

        assert!(
            api.marker_attrs(50.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(
            !api.marker_attrs(90.0)
                .contains(&HtmlAttr::Data("ars-in-range"))
        );
        assert!(api.dragging_indicator_attrs().contains(&HtmlAttr::Hidden));

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 80.0,
        }));

        assert!(
            !svc.connect(&|_| {})
                .dragging_indicator_attrs()
                .contains(&HtmlAttr::Hidden)
        );
    }

    #[test]
    fn value_from_pointer_uses_adapter_supplied_geometry() {
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
        assert_eq!(
            value_from_pointer(SliderPointer { x: 210.0, y: 70.0 }, track, svc.context()),
            Some(100.0)
        );

        let offset_bounds = service(Props {
            min: 10.0,
            max: 110.0,
            ..props()
        });

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 70.0 },
                track,
                offset_bounds.context()
            ),
            Some(60.0)
        );

        let rtl = service(Props {
            dir: Direction::Rtl,
            ..props()
        });

        assert_eq!(
            value_from_pointer(SliderPointer { x: 60.0, y: 70.0 }, track, rtl.context()),
            Some(75.0)
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
                SliderPointer {
                    x: f64::NAN,
                    y: 45.0
                },
                track,
                svc.context()
            ),
            None
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

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    x: f64::NAN,
                    ..track
                },
                svc.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    width: f64::NAN,
                    ..track
                },
                svc.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer {
                    x: 110.0,
                    y: f64::NAN,
                },
                track,
                vertical.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    y: f64::NAN,
                    ..track
                },
                vertical.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    height: f64::NAN,
                    ..track
                },
                vertical.context()
            ),
            None
        );
        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 10.0, y: 10.0 },
                Rect {
                    height: 0.0,
                    ..track
                },
                vertical.context()
            ),
            None
        );

        let reversed = service(Props {
            min: 100.0,
            max: 0.0,
            ..props()
        });

        assert_eq!(
            value_from_pointer(
                SliderPointer { x: 110.0, y: 70.0 },
                track,
                reversed.context()
            ),
            Some(50.0)
        );
    }

    #[test]
    fn helper_accessors_return_current_thumb_values() {
        let svc = service(props());

        assert_eq!(thumb_value(svc.context(), ThumbIndex::Start), 25.0);
        assert_eq!(other_thumb_value(svc.context(), ThumbIndex::Start), 75.0);
    }

    #[test]
    fn helper_math_covers_gap_step_percent_and_style_edges() {
        assert_eq!(snap_to_step(12.0, 0.0, 100.0, 5.0), 10.0);
        assert_eq!(snap_to_step(92.0, 10.0, 110.0, 5.0), 90.0);
        assert_eq!(snap_to_step(12.0, 0.0, 100.0, 0.0), 12.0);
        assert_eq!(snap_to_step(12.0, 0.0, 100.0, f64::NAN), 12.0);
        assert_eq!(snap_to_step(f64::NAN, 0.0, 100.0, 5.0), 0.0);
        assert_eq!(normalized_bounds(100.0, 0.0), (0.0, 100.0));
        assert_eq!(normalized_bounds(f64::NAN, 50.0), (50.0, 50.0));
        assert_eq!(effective_gap_for(0.0, 10.0, 5.0, 4), 10.0);
        assert_eq!(effective_gap_for(10.0, 30.0, 5.0, 2), 10.0);
        assert_eq!(effective_gap_for(10.0, 20.0, 8.0, 2), 10.0);
        assert_eq!(effective_gap_for(0.0, 10.0, 0.0, 4), 0.0);
        assert_eq!(effective_gap_for(0.0, 10.0, f64::NAN, 4), 0.0);
        assert_eq!(large_step(service(props()).context()), 50.0);
        assert_eq!(
            large_step(
                service(Props {
                    large_step: Some(15.0),
                    ..props()
                })
                .context()
            ),
            15.0
        );
        assert!(!values_changed([1.0, 2.0], [1.0, 2.0]));
        assert!(!values_changed([1.0, 2.0], [1.0 + f64::EPSILON, 2.0]));
        assert!(!values_changed([1.0, 2.0], [1.0, 2.0 + f64::EPSILON]));
        assert!(!values_changed([0.0, 0.0], [0.0, f64::EPSILON]));
        assert!(values_changed([1.0, 2.0], [1.0 + f64::EPSILON * 2.0, 2.0]));
        assert!(values_changed([1.0, 2.0], [1.0, 2.0 + f64::EPSILON * 2.0]));

        assert_eq!(
            normalize_values([90.0, 10.0], 0.0, 100.0, 5.0, 0),
            [10.0, 90.0]
        );
        assert_eq!(
            normalize_values([50.0, 50.0], 0.0, 100.0, 5.0, 0),
            [50.0, 50.0]
        );
        assert_eq!(
            normalize_values([50.0, 50.0], 0.0, 100.0, 5.0, 2),
            [50.0, 60.0]
        );
        assert_eq!(
            normalize_values([50.0, 60.0], 0.0, 100.0, 5.0, 2),
            [50.0, 60.0]
        );
        assert_eq!(
            normalize_values([95.0, 95.0], 0.0, 100.0, 5.0, 2),
            [85.0, 95.0]
        );

        let svc = service(props());

        assert_eq!(value_percent(svc.context(), 25.0), 25.0);
        assert_eq!(
            value_percent(
                service(Props {
                    min: 10.0,
                    max: 110.0,
                    ..props()
                })
                .context(),
                60.0
            ),
            50.0
        );
        assert_eq!(
            value_percent(
                service(Props {
                    min: 10.0,
                    max: 10.0,
                    ..props()
                })
                .context(),
                10.0
            ),
            0.0
        );
        assert_eq!(number_string(25.0), "25");
        assert_eq!(number_string(25.5), "25.5");

        let rtl = service(Props {
            dir: Direction::Rtl,
            default_value: [10.0, 40.0],
            ..props()
        });

        let vertical = service(Props {
            orientation: Orientation::Vertical,
            default_value: [10.0, 40.0],
            ..props()
        });

        let vertical_rtl = service(Props {
            dir: Direction::Rtl,
            orientation: Orientation::Vertical,
            default_value: [10.0, 40.0],
            ..props()
        });

        let ltr = service(Props {
            default_value: [10.0, 40.0],
            ..props()
        });

        assert_eq!(
            range_style(rtl.context()),
            (
                CssProperty::Left,
                "60%".to_string(),
                CssProperty::Width,
                "30%".to_string(),
            )
        );
        assert_eq!(
            range_style(vertical.context()),
            (
                CssProperty::Bottom,
                "10%".to_string(),
                CssProperty::Height,
                "30%".to_string(),
            )
        );
        assert_eq!(
            range_style(vertical_rtl.context()),
            (
                CssProperty::Bottom,
                "10%".to_string(),
                CssProperty::Height,
                "30%".to_string(),
            )
        );
        assert_eq!(
            range_style(ltr.context()),
            (
                CssProperty::Left,
                "10%".to_string(),
                CssProperty::Width,
                "30%".to_string(),
            )
        );
        assert_eq!(
            thumb_style(rtl.context(), ThumbIndex::Start),
            (CssProperty::Right, "10%".to_string())
        );
        assert_eq!(
            thumb_style(vertical.context(), ThumbIndex::Start),
            (CssProperty::Bottom, "10%".to_string())
        );
    }

    #[test]
    fn range_slider_root_snapshots() {
        let svc = service(props());

        assert_snapshot!(
            "range_slider_root_idle",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );

        let mut active = service(Props {
            disabled: true,
            invalid: true,
            readonly: true,
            ..props()
        });

        drop(active.send(Event::Focus {
            thumb: ThumbIndex::Start,
            is_keyboard: true,
        }));

        active.context_mut().dragging_thumb = Some(ThumbIndex::Start);

        assert_snapshot!(
            "range_slider_root_focused_dragging_disabled_invalid",
            snapshot_attrs(&active.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn range_slider_label_track_output_description_error_snapshots() {
        let svc = service(props());

        let api = svc.connect(&|_| {});

        assert_snapshot!("range_slider_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!("range_slider_track", snapshot_attrs(&api.track_attrs()));
        assert_snapshot!("range_slider_output", snapshot_attrs(&api.output_attrs()));
        assert_snapshot!(
            "range_slider_description",
            snapshot_attrs(&api.description_attrs())
        );
        assert_snapshot!(
            "range_slider_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
    }

    #[test]
    fn range_slider_range_direction_snapshots() {
        for (name, props) in [
            ("range_slider_range_ltr", props()),
            (
                "range_slider_range_rtl",
                Props {
                    dir: Direction::Rtl,
                    ..props()
                },
            ),
            (
                "range_slider_range_vertical",
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
    fn range_slider_thumb_state_and_format_snapshots() {
        let svc = service(props());

        assert_snapshot!(
            "range_slider_thumb_start",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs(ThumbIndex::Start))
        );
        assert_snapshot!(
            "range_slider_thumb_end",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs(ThumbIndex::End))
        );

        let disabled = {
            let mut svc = service(Props {
                start_disabled: true,
                invalid: true,
                ..props()
            });

            drop(svc.send(Event::Focus {
                thumb: ThumbIndex::Start,
                is_keyboard: true,
            }));
            drop(svc.send(Event::SetHasDescription(true)));

            svc
        };

        assert_snapshot!(
            "range_slider_thumb_start_disabled_described",
            snapshot_attrs(&disabled.connect(&|_| {}).thumb_attrs(ThumbIndex::Start))
        );

        let formatted = service(Props {
            format_value: Some(callback(|(value, other): (f64, f64)| {
                format!("{value:.0} of {other:.0}")
            })),
            ..props()
        });

        assert_snapshot!(
            "range_slider_thumb_format_value_text",
            snapshot_attrs(&formatted.connect(&|_| {}).thumb_attrs(ThumbIndex::End))
        );
    }

    #[test]
    fn range_slider_marker_hidden_and_dragging_indicator_snapshots() {
        let mut svc = service(Props {
            name: Some("price".into()),
            form: Some("filters".into()),
            ..props()
        });

        assert_snapshot!(
            "range_slider_marker_in_range",
            snapshot_attrs(&svc.connect(&|_| {}).marker_attrs(50.0))
        );
        assert_snapshot!(
            "range_slider_marker_out_of_range",
            snapshot_attrs(&svc.connect(&|_| {}).marker_attrs(90.0))
        );
        assert_snapshot!(
            "range_slider_marker_group",
            snapshot_attrs(&svc.connect(&|_| {}).marker_group_attrs())
        );
        assert_snapshot!(
            "range_slider_hidden_input_start",
            snapshot_attrs(&svc.connect(&|_| {}).hidden_input_attrs(ThumbIndex::Start))
        );
        assert_snapshot!(
            "range_slider_hidden_input_end",
            snapshot_attrs(&svc.connect(&|_| {}).hidden_input_attrs(ThumbIndex::End))
        );
        assert_snapshot!(
            "range_slider_dragging_indicator_idle",
            snapshot_attrs(&svc.connect(&|_| {}).dragging_indicator_attrs())
        );

        drop(svc.send(Event::PointerDown {
            thumb: ThumbIndex::End,
            value: 80.0,
        }));

        assert_snapshot!(
            "range_slider_dragging_indicator_active",
            snapshot_attrs(&svc.connect(&|_| {}).dragging_indicator_attrs())
        );
    }
}
