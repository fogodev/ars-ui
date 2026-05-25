//! TimeField component state machine and connect API.
//!
//! This module implements the framework-agnostic segmented time input defined
//! in `spec/components/date-time/time-field.md`.

#[cfg(test)]
mod tests;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    num::NonZeroU8,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    Direction, Env, HtmlAttr, IntlBackend, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan,
};
use ars_i18n::{HourCycle, Time};
use ars_interactions::KeyboardEventData;

use super::date_field::segment::{DateSegment, DateSegmentKind};

/// States for the `TimeField` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The field has no focused segment.
    Idle,

    /// A specific editable time segment has focus.
    Focused(DateSegmentKind),
}

/// Events for the `TimeField` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus moved to a segment.
    FocusSegment {
        /// The segment that received focus.
        kind: DateSegmentKind,
    },

    /// Focus left the entire field group.
    BlurAll,

    /// Increment a focused segment by one unit, wrapping within its range.
    IncrementSegment {
        /// The segment to increment.
        kind: DateSegmentKind,
    },

    /// Decrement a focused segment by one unit, wrapping within its range.
    DecrementSegment {
        /// The segment to decrement.
        kind: DateSegmentKind,
    },

    /// Type a printable character into the focused segment.
    TypeIntoSegment {
        /// The segment receiving typed input.
        kind: DateSegmentKind,

        /// The typed character.
        ch: char,
    },

    /// Commit the current type-ahead buffer for a segment.
    TypeBufferCommit {
        /// The segment whose buffer should commit.
        kind: DateSegmentKind,
    },

    /// Clear the value of one segment.
    ClearSegment {
        /// The segment to clear.
        kind: DateSegmentKind,
    },

    /// Clear all segment values and reset the field.
    ClearAll,

    /// Programmatically set the time value.
    SetValue(Option<Time>),

    /// Synchronize context from a new props snapshot.
    SyncProps(Box<Props>),

    /// Move focus to the next editable segment.
    FocusNextSegment,

    /// Move focus to the previous editable segment.
    FocusPrevSegment,
}

/// Editable precision for a `TimeField`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeGranularity {
    /// Edit only the hour segment.
    Hour,

    /// Edit hour and minute segments.
    #[default]
    Minute,

    /// Edit hour, minute, and second segments.
    Second,
}

/// Props for the `TimeField` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The stable DOM id for the field.
    pub id: String,

    /// The externally controlled value of the field.
    pub value: Option<Time>,

    /// The initial value used when the field is uncontrolled.
    pub default_value: Option<Time>,

    /// The editable time precision.
    pub granularity: TimeGranularity,

    /// Optional hour-cycle override.
    pub hour_cycle: Option<HourCycle>,

    /// Whether time-zone segments should be hidden if ever present.
    pub hide_time_zone: bool,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// Whether the field is required for form submission.
    pub required: bool,

    /// The minimum allowed time value.
    pub min_value: Option<Time>,

    /// The maximum allowed time value.
    pub max_value: Option<Time>,

    /// The visible label text of the field.
    pub label: String,

    /// The accessible label when no visible label is used.
    pub aria_label: Option<String>,

    /// Extra ID references used to describe the field group.
    pub aria_describedby: Option<String>,

    /// Optional descriptive help text for the field.
    pub description: Option<String>,

    /// Optional validation error text for the field.
    pub error_message: Option<String>,

    /// Whether the field is currently invalid.
    pub invalid: bool,

    /// Optional form field name for the hidden input.
    pub name: Option<String>,

    /// Whether numeric segments should force fixed-width leading zeroes.
    pub force_leading_zeros: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            granularity: TimeGranularity::Minute,
            hour_cycle: None,
            hide_time_zone: false,
            disabled: false,
            readonly: false,
            required: false,
            min_value: None,
            max_value: None,
            label: String::new(),
            aria_label: None,
            aria_describedby: None,
            description: None,
            error_message: None,
            invalid: false,
            name: None,
            force_leading_zeros: false,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] value with the `TimeField` defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the stable DOM id for the field.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), the externally controlled field value.
    #[must_use]
    pub const fn value(mut self, value: Option<Time>) -> Self {
        self.value = value;
        self
    }

    /// Sets [`default_value`](Self::default_value), the uncontrolled initial value.
    #[must_use]
    pub const fn default_value(mut self, value: Option<Time>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`granularity`](Self::granularity), the editable time precision.
    #[must_use]
    pub const fn granularity(mut self, granularity: TimeGranularity) -> Self {
        self.granularity = granularity;
        self
    }

    /// Sets [`hour_cycle`](Self::hour_cycle), overriding locale hour-cycle preferences.
    #[must_use]
    pub const fn hour_cycle(mut self, hour_cycle: Option<HourCycle>) -> Self {
        self.hour_cycle = hour_cycle;
        self
    }

    /// Sets [`hide_time_zone`](Self::hide_time_zone), the time-zone segment visibility flag.
    #[must_use]
    pub const fn hide_time_zone(mut self, value: bool) -> Self {
        self.hide_time_zone = value;
        self
    }

    /// Sets [`disabled`](Self::disabled), whether user interaction is blocked.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`readonly`](Self::readonly), whether editing is blocked while focus remains allowed.
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`required`](Self::required), whether the field is required.
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`min_value`](Self::min_value), the minimum allowed time.
    #[must_use]
    pub const fn min_value(mut self, value: Option<Time>) -> Self {
        self.min_value = value;
        self
    }

    /// Sets [`max_value`](Self::max_value), the maximum allowed time.
    #[must_use]
    pub const fn max_value(mut self, value: Option<Time>) -> Self {
        self.max_value = value;
        self
    }

    /// Sets [`label`](Self::label), the visible field label text.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets [`aria_label`](Self::aria_label), the field group's accessible label.
    #[must_use]
    pub fn aria_label(mut self, label: Option<String>) -> Self {
        self.aria_label = label;
        self
    }

    /// Sets [`aria_describedby`](Self::aria_describedby), extra description references.
    #[must_use]
    pub fn aria_describedby(mut self, id: Option<String>) -> Self {
        self.aria_describedby = id;
        self
    }

    /// Sets [`description`](Self::description), the optional help text.
    #[must_use]
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    /// Sets [`error_message`](Self::error_message), the optional validation message.
    #[must_use]
    pub fn error_message(mut self, message: Option<String>) -> Self {
        self.error_message = message;
        self
    }

    /// Sets [`invalid`](Self::invalid), whether the field is invalid.
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`name`](Self::name), the hidden input's form field name.
    #[must_use]
    pub fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Sets [`force_leading_zeros`](Self::force_leading_zeros), whether numeric segments use fixed width.
    #[must_use]
    pub const fn force_leading_zeros(mut self, value: bool) -> Self {
        self.force_leading_zeros = value;
        self
    }
}

/// Context for the `TimeField` component.
#[derive(Clone)]
pub struct Context {
    /// Controlled or uncontrolled time value.
    pub value: Bindable<Option<Time>>,

    /// Segments in display order.
    pub segments: Vec<DateSegment>,

    /// The segment currently holding keyboard focus.
    pub focused_segment: Option<DateSegmentKind>,

    /// Accumulated typed characters for the focused segment.
    pub type_buffer: String,

    /// The resolved locale.
    pub locale: Locale,

    /// Resolved `TimeField` messages.
    pub messages: Messages,

    /// Backend used for locale-dependent labels and digits.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// The active time granularity.
    pub granularity: TimeGranularity,

    /// The resolved hour cycle.
    pub hour_cycle: HourCycle,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// The minimum allowed time value.
    pub min_value: Option<Time>,

    /// The maximum allowed time value.
    pub max_value: Option<Time>,

    /// Derived component part ids.
    pub ids: ComponentIds,

    /// Whether numeric segments should force fixed-width leading zeroes.
    pub force_leading_zeros: bool,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("segments", &self.segments)
            .field("focused_segment", &self.focused_segment)
            .field("type_buffer", &self.type_buffer)
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("granularity", &self.granularity)
            .field("hour_cycle", &self.hour_cycle)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("min_value", &self.min_value)
            .field("max_value", &self.max_value)
            .field("ids", &self.ids)
            .field("force_leading_zeros", &self.force_leading_zeros)
            .finish()
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.segments == other.segments
            && self.focused_segment == other.focused_segment
            && self.type_buffer == other.type_buffer
            && self.locale == other.locale
            && self.messages == other.messages
            && self.granularity == other.granularity
            && self.hour_cycle == other.hour_cycle
            && self.disabled == other.disabled
            && self.readonly == other.readonly
            && self.min_value == other.min_value
            && self.max_value == other.max_value
            && self.ids == other.ids
            && self.force_leading_zeros == other.force_leading_zeros
    }
}

impl Context {
    /// Recomputes segments from the current value, locale, granularity, and hour cycle.
    pub fn rebuild_segments(&mut self) {
        self.segments = build_segments(self);
    }

    /// Returns the index of a segment kind in display order.
    #[must_use]
    pub fn segment_index(&self, kind: DateSegmentKind) -> Option<usize> {
        self.segments
            .iter()
            .position(|segment| segment.kind == kind)
    }

    /// Returns the first editable segment.
    #[must_use]
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.segments
            .iter()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Returns the next editable segment after `kind`.
    #[must_use]
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let index = self.segment_index(kind)?;

        self.segments[index + 1..]
            .iter()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Returns the previous editable segment before `kind`.
    #[must_use]
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let index = self.segment_index(kind)?;

        self.segments[..index]
            .iter()
            .rev()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Returns `true` when every visible editable time segment has a value.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.segments
            .iter()
            .filter(|segment| segment.is_editable)
            .all(|segment| segment.value.is_some())
    }

    /// Assembles a validated time from current segment values.
    #[must_use]
    pub fn assemble_time(&self) -> Option<Time> {
        let raw_hour = u8::try_from(self.get_segment_value(DateSegmentKind::Hour)?).ok()?;

        let minute = if self.granularity >= TimeGranularity::Minute {
            u8::try_from(self.get_segment_value(DateSegmentKind::Minute)?).ok()?
        } else {
            0
        };

        let second = if self.granularity >= TimeGranularity::Second {
            u8::try_from(self.get_segment_value(DateSegmentKind::Second)?).ok()?
        } else {
            0
        };

        let day_period = self.get_segment_value(DateSegmentKind::DayPeriod);

        let hour = display_hour_to_24(raw_hour, day_period, self.hour_cycle)?;

        Time::new(hour, minute, second, 0).ok()
    }

    /// Returns the current numeric value for a segment.
    #[must_use]
    pub fn get_segment_value(&self, kind: DateSegmentKind) -> Option<i32> {
        self.segments
            .iter()
            .find(|segment| segment.kind == kind)?
            .value
    }

    /// Returns a mutable reference to a segment.
    pub fn segment_mut(&mut self, kind: DateSegmentKind) -> Option<&mut DateSegment> {
        self.segments
            .iter_mut()
            .find(|segment| segment.kind == kind)
    }

    /// Sets and formats a segment value within that segment's valid range.
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        let Some(index) = self
            .segments
            .iter()
            .position(|segment| segment.kind == kind)
        else {
            return;
        };

        if !self.segments[index].is_editable {
            return;
        }

        let min = self.segments[index].min;
        let max = self.segments[index].max;
        let value = raw.clamp(min, max);
        let text = self.format_segment_value(kind, value);

        self.segments[index].value = Some(value);
        self.segments[index].text = text;
    }

    /// Clears a segment value.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) {
        if let Some(segment) = self.segment_mut(kind) {
            segment.value = None;
            segment.text.clear();
        }
    }

    /// Increment the value of a segment, wrapping within its range.
    pub fn increment_segment(&mut self, kind: DateSegmentKind) {
        self.step_segment_value(kind, 1);
    }

    /// Decrement the value of a segment, wrapping within its range.
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) {
        self.step_segment_value(kind, -1);
    }

    fn step_segment_value(&mut self, kind: DateSegmentKind, delta: i32) {
        let Some(segment) = self
            .segments
            .iter()
            .find(|segment| segment.kind == kind && segment.is_editable)
            .cloned()
        else {
            return;
        };

        if kind == DateSegmentKind::DayPeriod && segment.value.is_none() {
            self.set_segment_value(kind, if delta >= 0 { segment.max } else { segment.min });

            return;
        }

        let current = segment.value.unwrap_or(if delta >= 0 {
            segment.min - 1
        } else {
            segment.max + 1
        });

        let span = segment.max - segment.min + 1;
        let next = (current - segment.min + delta).rem_euclid(span) + segment.min;

        self.set_segment_value(kind, next);
    }

    fn format_segment_value(&self, kind: DateSegmentKind, value: i32) -> String {
        format_time_segment(
            self.intl_backend.as_ref(),
            &self.locale,
            kind,
            value,
            self.force_leading_zeros,
        )
    }
}

/// Localized labels for `TimeField` segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the hour segment.
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the minute segment.
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the second segment.
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the day-period segment.
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the `time_field` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts the type-buffer commit timer.
    TypeBufferCommit,
}

/// Machine for the `TimeField` component.
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
        let value = if let Some(value) = props.value {
            Bindable::controlled(Some(value))
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let mut ctx = Context {
            value,
            segments: Vec::new(),
            focused_segment: None,
            type_buffer: String::new(),
            locale: env.locale.clone(),
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            granularity: props.granularity,
            hour_cycle: props
                .hour_cycle
                .unwrap_or_else(|| env.locale.hour_cycle(env.intl_backend.as_ref())),
            disabled: props.disabled,
            readonly: props.readonly,
            min_value: props.min_value,
            max_value: props.max_value,
            ids: ComponentIds::from_id(&props.id),
            force_leading_zeros: props.force_leading_zeros,
        };

        ctx.rebuild_segments();

        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps(Box::new(new.clone()))]
        }
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled && !matches!(event, Event::SetValue(_) | Event::SyncProps(_)) {
            return None;
        }

        match event {
            Event::SyncProps(props) => {
                let props = props.as_ref().clone();
                let mut next_ctx = ctx.clone();

                sync_props(&mut next_ctx, &props);

                let next_state = reconcile_state_after_sync(state, &next_ctx);
                let should_apply_focus = next_state != *state;

                Some(
                    TransitionPlan::to(next_state.clone()).apply(move |ctx: &mut Context| {
                        sync_props(ctx, &props);

                        if should_apply_focus {
                            apply_state_focus(ctx, &next_state);
                        }
                    }),
                )
            }

            Event::FocusSegment { kind } => {
                if !kind.is_editable()
                    || !ctx
                        .segments
                        .iter()
                        .any(|segment| segment.kind == *kind && segment.is_editable)
                {
                    return None;
                }

                let kind = *kind;
                Some(
                    TransitionPlan::to(State::Focused(kind)).apply(move |ctx: &mut Context| {
                        commit_type_buffer(ctx);
                        ctx.focused_segment = Some(kind);
                        ctx.type_buffer.clear();
                    }),
                )
            }

            Event::BlurAll => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        commit_type_buffer(ctx);
                        ctx.focused_segment = None;
                        ctx.type_buffer.clear();
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
            ),

            Event::FocusNextSegment => {
                if let State::Focused(current) = state {
                    let current = *current;
                    let next = ctx.next_editable_after(current);
                    let target = next.map_or(State::Idle, State::Focused);
                    Some(
                        TransitionPlan::to(target)
                            .apply(move |ctx: &mut Context| {
                                commit_type_buffer(ctx);
                                ctx.focused_segment = next;
                                ctx.type_buffer.clear();
                            })
                            .cancel_effect(Effect::TypeBufferCommit),
                    )
                } else {
                    let first = ctx.first_editable()?;
                    Some(TransitionPlan::to(State::Focused(first)).apply(
                        move |ctx: &mut Context| {
                            ctx.focused_segment = Some(first);
                        },
                    ))
                }
            }

            Event::FocusPrevSegment => {
                if let State::Focused(current) = state {
                    let previous = ctx.prev_editable_before(*current)?;
                    Some(
                        TransitionPlan::to(State::Focused(previous))
                            .apply(move |ctx: &mut Context| {
                                commit_type_buffer(ctx);
                                ctx.focused_segment = Some(previous);
                                ctx.type_buffer.clear();
                            })
                            .cancel_effect(Effect::TypeBufferCommit),
                    )
                } else {
                    None
                }
            }

            Event::IncrementSegment { kind } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *kind;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();
                        ctx.increment_segment(kind);
                        maybe_publish(ctx);
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::DecrementSegment { kind } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *kind;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();
                        ctx.decrement_segment(kind);
                        maybe_publish(ctx);
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::TypeIntoSegment { kind, ch } => {
                if ctx.readonly || !matches!(state, State::Focused(_)) {
                    return None;
                }

                type_into_segment(ctx, state, *kind, *ch)
            }

            Event::TypeBufferCommit { kind } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    commit_buffer_for_kind(ctx, kind, true);
                }))
            }

            Event::ClearSegment { kind } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *kind;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();
                        ctx.clear_segment_value(kind);
                        ctx.value.set(None);
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::ClearAll => {
                if ctx.readonly {
                    return None;
                }

                Some(
                    TransitionPlan::to(State::Idle)
                        .apply(|ctx: &mut Context| {
                            let editable = ctx
                                .segments
                                .iter()
                                .filter(|segment| segment.is_editable)
                                .map(|segment| segment.kind)
                                .collect::<Vec<_>>();

                            for kind in editable {
                                ctx.clear_segment_value(kind);
                            }

                            ctx.value.set(None);
                            ctx.focused_segment = None;
                            ctx.type_buffer.clear();
                        })
                        .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::SetValue(value) => {
                let value = *value;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        apply_value(ctx, value);
                        ctx.type_buffer.clear();
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }
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

/// Structural parts exposed by the `TimeField` connect API.
#[derive(ComponentPart)]
#[scope = "time-field"]
pub enum Part {
    /// The root container.
    Root,

    /// The visible label.
    Label,

    /// The group containing all rendered segments.
    FieldGroup,

    /// An editable time segment.
    Segment {
        /// The logical kind of the segment.
        kind: DateSegmentKind,
    },

    /// A non-editable literal separator.
    Literal {
        /// The literal's rendered index.
        index: usize,
    },

    /// The optional description element.
    Description,

    /// The optional error message element.
    ErrorMessage,

    /// The hidden form input.
    HiddenInput,
}

/// API for the `TimeField` component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,

    /// The context of the component.
    ctx: &'a Context,

    /// The props of the component.
    props: &'a Props,

    /// The send function.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl<'a> Api<'a> {
    /// Creates a `TimeField` connect API from machine state.
    #[must_use]
    pub const fn new(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Self {
        Self {
            state,
            ctx,
            props,
            send,
        }
    }

    /// Returns attributes for the outer root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_name());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.props.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        attrs
    }

    /// Returns attributes for the visible label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the segment group.
    #[must_use]
    pub fn field_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::FieldGroup.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("field-group"))
            .set(HtmlAttr::Role, "group")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        let mut described_by = Vec::new();

        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.props.invalid && self.props.error_message.is_some() {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if let Some(extra) = &self.props.aria_describedby {
            described_by.push(extra.clone());
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        attrs
    }

    /// Returns attributes for an editable segment or a literal fallback.
    #[must_use]
    pub fn segment_attrs(&self, kind: &DateSegmentKind) -> AttrMap {
        let Some((index, segment)) = self
            .ctx
            .segments
            .iter()
            .enumerate()
            .find(|(_, segment)| segment.kind == *kind)
        else {
            return AttrMap::new();
        };

        if !segment.is_editable {
            return self.literal_attrs(index);
        }

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::Segment { kind: *kind }).data_attrs();

        let is_focused = self.ctx.focused_segment == Some(segment.kind);

        let is_tab_stop = !self.ctx.disabled
            && (is_focused
                || (self.ctx.focused_segment.is_none()
                    && self.ctx.first_editable() == Some(segment.kind)));

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "spinbutton")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                time_segment_aria_label(segment.kind, &self.ctx.messages, &self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), segment.min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), segment.max.to_string())
            .set(HtmlAttr::Data("ars-segment"), segment.kind.data_name())
            .set(HtmlAttr::TabIndex, if is_tab_stop { "0" } else { "-1" });

        if let Some(value) = segment.value {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string())
                .set(HtmlAttr::Aria(AriaAttr::ValueText), &segment.text);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &segment.placeholder);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.props.invalid && self.props.error_message.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("error-message"),
            );
        }

        attrs
    }

    /// Returns attributes for a literal separator.
    #[must_use]
    pub fn literal_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::Literal { index }).data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::TabIndex, "-1");

        attrs
    }

    /// Returns attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("description"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        let value = self
            .ctx
            .value
            .get()
            .map(|time| time.to_iso8601())
            .unwrap_or_default();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden")
            .set(HtmlAttr::Value, value);

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }

        attrs
    }

    /// Handles focusout on the field group.
    pub fn on_field_group_focusout(&self, focus_leaving_group: bool) {
        if focus_leaving_group {
            (self.send)(Event::BlurAll);
        }
    }

    /// Handles keydown on a segment.
    pub fn on_segment_keydown(
        &self,
        kind: DateSegmentKind,
        data: &KeyboardEventData,
        shift: bool,
        dir: Direction,
    ) {
        if data.is_composing {
            return;
        }

        let is_rtl = dir.is_rtl();

        match data.key {
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementSegment { kind }),

            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment { kind }),

            KeyboardKey::ArrowRight if is_rtl => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::ArrowLeft if is_rtl => (self.send)(Event::FocusNextSegment),

            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::Tab if shift => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::ArrowRight | KeyboardKey::Tab => (self.send)(Event::FocusNextSegment),

            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::ClearSegment { kind });
            }

            KeyboardKey::Escape => (self.send)(Event::ClearAll),

            _ if data.character.is_some() => {
                if let Some(ch) = data.character {
                    (self.send)(Event::TypeIntoSegment { kind, ch });
                }
            }

            _ => {}
        }
    }

    /// Handles focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment { kind });
    }

    /// Handles click on a segment.
    pub fn on_segment_click(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment { kind });
    }

    /// Returns the rendered segments.
    #[must_use]
    pub fn segments(&self) -> &[DateSegment] {
        &self.ctx.segments
    }

    /// Returns the current time value.
    #[must_use]
    pub fn value(&self) -> Option<&Time> {
        self.ctx.value.get().as_ref()
    }

    /// Returns `true` when any segment has focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    const fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Focused(_) => "focused",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::FieldGroup => self.field_group_attrs(),
            Part::Segment { kind } => self.segment_attrs(&kind),
            Part::Literal { index } => self.literal_attrs(index),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

fn type_into_segment(
    ctx: &Context,
    state: &State,
    kind: DateSegmentKind,
    ch: char,
) -> Option<TransitionPlan<Machine>> {
    if kind == DateSegmentKind::DayPeriod {
        let mut buffer = ctx.type_buffer.clone();

        buffer.push(ch);

        let has_cjk_table = cjk_day_period_table(&ctx.locale).is_some();

        let cjk_value = has_cjk_table
            .then(|| day_period_from_cjk_buffer(&buffer, &ctx.locale, ctx.hour_cycle, None, None))
            .flatten();

        let backend_value = ctx
            .intl_backend
            .day_period_from_char(ch, &ctx.locale)
            .map(|is_pm| if is_pm { 1 } else { 0 });

        if has_cjk_table
            && cjk_value.is_none()
            && backend_value.is_none()
            && is_cjk_day_period_prefix(&buffer, &ctx.locale)
        {
            return Some(
                TransitionPlan::to(state.clone())
                    .apply(move |ctx: &mut Context| {
                        ctx.type_buffer = buffer;
                    })
                    .cancel_effect(Effect::TypeBufferCommit)
                    .with_effect(PendingEffect::named(Effect::TypeBufferCommit)),
            );
        }

        let value = cjk_value.or(backend_value)?;

        let next = ctx.next_editable_after(kind);

        let target = next.map_or_else(|| state.clone(), State::Focused);

        return Some(
            TransitionPlan::to(target)
                .apply(move |ctx: &mut Context| {
                    ctx.set_segment_value(kind, value);

                    ctx.type_buffer.clear();

                    if let Some(next) = next {
                        ctx.focused_segment = Some(next);
                    }

                    maybe_publish(ctx);
                })
                .cancel_effect(Effect::TypeBufferCommit),
        );
    }

    if !matches!(
        kind,
        DateSegmentKind::Hour | DateSegmentKind::Minute | DateSegmentKind::Second
    ) {
        return None;
    }

    if !ch.is_ascii_digit() {
        return None;
    }

    let mut new_buffer = ctx.type_buffer.clone();

    new_buffer.push(ch);

    let buffered = new_buffer.parse::<i32>().ok()?;

    let segment = ctx.segments.iter().find(|segment| segment.kind == kind)?;

    let max_digits = digits_needed(u32::try_from(segment.max).ok()?);

    let should_advance = new_buffer.len() >= max_digits || buffered * 10 > segment.max;

    let next = should_advance
        .then(|| ctx.next_editable_after(kind))
        .flatten();

    let target = if should_advance {
        next.map_or_else(|| state.clone(), State::Focused)
    } else {
        state.clone()
    };

    let mut plan = TransitionPlan::to(target).apply(move |ctx: &mut Context| {
        ctx.type_buffer.push(ch);
        let Some(segment) = ctx.segments.iter().find(|segment| segment.kind == kind) else {
            return;
        };

        if let Ok(value) = ctx.type_buffer.parse::<i32>()
            && value <= segment.max
        {
            ctx.set_segment_value(kind, value);

            maybe_publish(ctx);
        }

        if should_advance {
            ctx.type_buffer.clear();

            if let Some(next) = next {
                ctx.focused_segment = Some(next);
            }
        }
    });

    if should_advance {
        plan = plan.cancel_effect(Effect::TypeBufferCommit);
    } else {
        plan = plan
            .cancel_effect(Effect::TypeBufferCommit)
            .with_effect(PendingEffect::named(Effect::TypeBufferCommit));
    }

    Some(plan)
}

fn commit_type_buffer(ctx: &mut Context) {
    if let Some(kind) = ctx.focused_segment {
        commit_buffer_for_kind(ctx, kind, kind == DateSegmentKind::DayPeriod);
    }
}

fn commit_buffer_for_kind(ctx: &mut Context, kind: DateSegmentKind, timeout_fallback: bool) {
    if kind == DateSegmentKind::DayPeriod {
        let current_hour = timeout_fallback
            .then(|| {
                ctx.get_segment_value(DateSegmentKind::Hour)
                    .and_then(|value| u8::try_from(value).ok())
            })
            .flatten();
        let current_day_period = timeout_fallback
            .then(|| ctx.get_segment_value(DateSegmentKind::DayPeriod))
            .flatten();

        if let Some(value) = day_period_from_cjk_buffer(
            &ctx.type_buffer,
            &ctx.locale,
            ctx.hour_cycle,
            current_hour,
            current_day_period,
        ) {
            ctx.set_segment_value(kind, value);

            maybe_publish(ctx);
        }

        ctx.type_buffer.clear();

        return;
    }

    if let Ok(value) = ctx.type_buffer.parse::<i32>() {
        ctx.set_segment_value(kind, value);

        maybe_publish(ctx);
    }

    ctx.type_buffer.clear();
}

fn maybe_publish(ctx: &mut Context) {
    if !ctx.is_complete() {
        return;
    }

    let Some(time) = ctx.assemble_time() else {
        return;
    };

    let clamped = clamp_time(time, ctx.min_value.as_ref(), ctx.max_value.as_ref());

    apply_segments_from_time(ctx, clamped);

    ctx.value.set(Some(clamped));
}

fn apply_value(ctx: &mut Context, value: Option<Time>) {
    let value = value.map(|time| clamp_time(time, ctx.min_value.as_ref(), ctx.max_value.as_ref()));

    if ctx.value.is_controlled() {
        ctx.value.set(value);
        ctx.value.sync_controlled(Some(value));
    } else {
        ctx.value.set(value);
    }

    ctx.rebuild_segments();
}

fn apply_controlled_value(ctx: &mut Context, value: Option<Time>) {
    let value = value.map(|time| clamp_time(time, ctx.min_value.as_ref(), ctx.max_value.as_ref()));

    ctx.value.set(value);
    ctx.value.sync_controlled(Some(value));
    ctx.rebuild_segments();
}

fn sync_props(ctx: &mut Context, props: &Props) {
    let was_controlled = ctx.value.is_controlled();
    let previous_granularity = ctx.granularity;
    let previous_hour_cycle = ctx.hour_cycle;
    let previous_force_leading_zeros = ctx.force_leading_zeros;

    ctx.granularity = props.granularity;
    ctx.hour_cycle = props
        .hour_cycle
        .unwrap_or_else(|| ctx.locale.hour_cycle(ctx.intl_backend.as_ref()));
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.min_value = props.min_value;
    ctx.max_value = props.max_value;
    ctx.ids = ComponentIds::from_id(&props.id);
    ctx.force_leading_zeros = props.force_leading_zeros;

    if let Some(value) = props.value {
        apply_controlled_value(ctx, Some(value));
    } else if was_controlled {
        ctx.value.sync_controlled(None);
        ctx.value.set(None);
        ctx.rebuild_segments();
    } else {
        ctx.value.sync_controlled(None);

        let clamped = ctx
            .value
            .get()
            .map(|time| clamp_time(time, ctx.min_value.as_ref(), ctx.max_value.as_ref()));
        let value_changed = clamped != *ctx.value.get();
        let must_rebuild = previous_granularity != ctx.granularity
            || previous_hour_cycle != ctx.hour_cycle
            || previous_force_leading_zeros != ctx.force_leading_zeros
            || value_changed;

        ctx.value.set(clamped);

        if must_rebuild {
            ctx.rebuild_segments();
        }
    }
}

fn reconcile_state_after_sync(state: &State, ctx: &Context) -> State {
    let State::Focused(kind) = state else {
        return State::Idle;
    };

    let still_focusable = !ctx.disabled
        && ctx
            .segments
            .iter()
            .any(|segment| segment.kind == *kind && segment.is_editable);

    if still_focusable {
        State::Focused(*kind)
    } else {
        State::Idle
    }
}

fn apply_state_focus(ctx: &mut Context, state: &State) {
    match state {
        State::Idle => {
            ctx.focused_segment = None;
            ctx.type_buffer.clear();
        }

        State::Focused(kind) => {
            ctx.focused_segment = Some(*kind);
            ctx.type_buffer.clear();
        }
    }
}

fn apply_segments_from_time(ctx: &mut Context, time: Time) {
    for segment in &mut ctx.segments {
        let value = match segment.kind {
            DateSegmentKind::Hour => Some(display_hour(time, ctx.hour_cycle)),
            DateSegmentKind::Minute => Some(i32::from(time.minute())),
            DateSegmentKind::Second => Some(i32::from(time.second())),
            DateSegmentKind::DayPeriod => Some(if is_pm(time) { 1 } else { 0 }),
            _ => None,
        };

        if let Some(value) = value {
            segment.value = Some(value);

            segment.text = format_time_segment(
                ctx.intl_backend.as_ref(),
                &ctx.locale,
                segment.kind,
                value,
                ctx.force_leading_zeros,
            );
        }
    }
}

fn build_segments(ctx: &Context) -> Vec<DateSegment> {
    let mut segments = Vec::new();

    let value = *ctx.value.get();

    let (hour_min, hour_max) = display_hour_range(ctx.hour_cycle);

    let mut hour = DateSegment::new_numeric(DateSegmentKind::Hour, hour_min, hour_max, "hh");

    if let Some(time) = value {
        let display = display_hour(time, ctx.hour_cycle);

        hour.value = Some(display);
        hour.text = ctx.format_segment_value(DateSegmentKind::Hour, display);
    }

    segments.push(hour);

    if ctx.granularity >= TimeGranularity::Minute {
        segments.push(DateSegment::new_literal(":"));

        let mut minute = DateSegment::new_numeric(DateSegmentKind::Minute, 0, 59, "mm");

        if let Some(time) = value {
            let value = i32::from(time.minute());

            minute.value = Some(value);
            minute.text = ctx.format_segment_value(DateSegmentKind::Minute, value);
        }

        segments.push(minute);
    }

    if ctx.granularity >= TimeGranularity::Second {
        segments.push(DateSegment::new_literal(":"));

        let mut second = DateSegment::new_numeric(DateSegmentKind::Second, 0, 59, "ss");

        if let Some(time) = value {
            let value = i32::from(time.second());

            second.value = Some(value);
            second.text = ctx.format_segment_value(DateSegmentKind::Second, value);
        }

        segments.push(second);
    }

    if has_day_period(ctx.hour_cycle) {
        segments.push(DateSegment::new_literal("\u{00A0}"));

        let mut period = DateSegment {
            kind: DateSegmentKind::DayPeriod,
            value: None,
            min: 0,
            max: 1,
            text: String::new(),
            placeholder: ctx.intl_backend.day_period_label(false, &ctx.locale),
            literal: None,
            is_editable: true,
        };

        if let Some(time) = value {
            let value = if is_pm(time) { 1 } else { 0 };

            period.value = Some(value);
            period.text = ctx.format_segment_value(DateSegmentKind::DayPeriod, value);
        }

        segments.push(period);
    }

    segments
}

const fn display_hour_range(hour_cycle: HourCycle) -> (i32, i32) {
    match hour_cycle {
        HourCycle::H11 => (0, 11),
        HourCycle::H12 => (1, 12),
        HourCycle::H23 => (0, 23),
        HourCycle::H24 => (1, 24),
    }
}

const fn has_day_period(hour_cycle: HourCycle) -> bool {
    matches!(hour_cycle, HourCycle::H11 | HourCycle::H12)
}

fn display_hour(time: Time, hour_cycle: HourCycle) -> i32 {
    let hour = time.hour();

    match hour_cycle {
        HourCycle::H11 => i32::from(hour % 12),

        HourCycle::H12 => i32::from(if hour.is_multiple_of(12) {
            12
        } else {
            hour % 12
        }),

        HourCycle::H23 => i32::from(hour),

        HourCycle::H24 => i32::from(if hour == 0 { 24 } else { hour }),
    }
}

fn display_hour_to_24(raw_hour: u8, day_period: Option<i32>, hour_cycle: HourCycle) -> Option<u8> {
    match hour_cycle {
        HourCycle::H11 => {
            let is_pm = day_period.unwrap_or(0) == 1;
            Some(if is_pm { raw_hour + 12 } else { raw_hour })
        }

        HourCycle::H12 => {
            let is_pm = day_period.unwrap_or(0) == 1;
            Some(match (raw_hour, is_pm) {
                (12, false) => 0,
                (12, true) => 12,
                (_, true) => raw_hour + 12,
                (_, false) => raw_hour,
            })
        }

        HourCycle::H23 => Some(raw_hour),

        HourCycle::H24 => Some(if raw_hour == 24 { 0 } else { raw_hour }),
    }
}

const fn is_pm(time: Time) -> bool {
    time.hour() >= 12
}

#[derive(Clone, Copy)]
struct CjkDayPeriodEntry {
    am_label: &'static str,
    pm_label: &'static str,
}

fn cjk_day_period_table(locale: &Locale) -> Option<CjkDayPeriodEntry> {
    match locale.language() {
        "ja" => Some(CjkDayPeriodEntry {
            am_label: "午前",
            pm_label: "午後",
        }),

        "ko" => Some(CjkDayPeriodEntry {
            am_label: "오전",
            pm_label: "오후",
        }),

        _ => None,
    }
}

fn is_cjk_day_period_prefix(buffer: &str, locale: &Locale) -> bool {
    let Some(entry) = cjk_day_period_table(locale) else {
        return false;
    };

    let normalized = strip_combining_marks(buffer);

    entry.am_label.starts_with(&normalized) || entry.pm_label.starts_with(&normalized)
}

fn day_period_from_cjk_buffer(
    buffer: &str,
    locale: &Locale,
    hour_cycle: HourCycle,
    current_hour: Option<u8>,
    current_day_period: Option<i32>,
) -> Option<i32> {
    let entry = cjk_day_period_table(locale)?;

    let normalized = strip_combining_marks(buffer);

    let am_matches = entry.am_label.starts_with(&normalized);
    let pm_matches = entry.pm_label.starts_with(&normalized);

    match (
        am_matches,
        pm_matches,
        hour_cycle,
        current_hour,
        current_day_period,
    ) {
        (true, false, ..) => Some(0),
        (false, true, ..) => Some(1),
        (true, true, _, _, Some(day_period)) => Some(day_period.clamp(0, 1)),
        (true, true, HourCycle::H11, _, None) | (true, true, _, None, _) | (false, false, ..) => {
            None
        }
        (true, true, _, Some(hour), _) => Some(if hour < 12 { 0 } else { 1 }),
    }
}

fn strip_combining_marks(input: &str) -> String {
    input
        .chars()
        .filter(|ch| {
            !matches!(
                u32::from(*ch),
                0x0300..=0x036F
                    | 0x1AB0..=0x1AFF
                    | 0x1DC0..=0x1DFF
                    | 0x20D0..=0x20FF
                    | 0xFE20..=0xFE2F
            )
        })
        .collect()
}

fn format_time_segment(
    backend: &dyn IntlBackend,
    locale: &Locale,
    kind: DateSegmentKind,
    value: i32,
    force_leading_zeros: bool,
) -> String {
    if kind == DateSegmentKind::DayPeriod {
        return backend.day_period_label(value == 1, locale);
    }

    let min_digits = if force_leading_zeros { 2 } else { 1 };

    u32::try_from(value).map_or_else(
        |_| value.to_string(),
        |value| {
            backend.format_segment_digits(
                value,
                NonZeroU8::new(min_digits).expect("segment width is non-zero"),
                locale,
            )
        },
    )
}

fn clamp_time(time: Time, min_value: Option<&Time>, max_value: Option<&Time>) -> Time {
    if let Some(min) = min_value
        && time < *min
    {
        return *min;
    }

    if let Some(max) = max_value
        && time > *max
    {
        return *max;
    }

    time
}

const fn digits_needed(max: u32) -> usize {
    match max {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        _ => 4,
    }
}

fn time_segment_aria_label(kind: DateSegmentKind, messages: &Messages, locale: &Locale) -> String {
    fn call(
        message: &MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
        locale: &Locale,
    ) -> String {
        message(locale)
    }

    match kind {
        DateSegmentKind::Hour => call(&messages.hour_label, locale),
        DateSegmentKind::Minute => call(&messages.minute_label, locale),
        DateSegmentKind::Second => call(&messages.second_label, locale),
        DateSegmentKind::DayPeriod => call(&messages.day_period_label, locale),
        _ => String::new(),
    }
}
