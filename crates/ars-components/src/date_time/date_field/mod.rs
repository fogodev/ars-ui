//! DateField component state machine and connect API.
//!
//! This module implements the framework-agnostic segmented date input defined
//! in `spec/components/date-time/date-field.md`.

mod segment;

#[cfg(test)]
mod tests;

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    cmp::Ordering,
    fmt::{self, Debug},
    num::NonZeroU8,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    Direction, Env, HtmlAttr, IntlBackend, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan,
};
use ars_i18n::{
    CalendarDate, CalendarDateFields, CalendarSystem, DateField as CalendarDateField, Era,
    calendar::CycleOptions,
};
use ars_interactions::KeyboardEventData;
pub use segment::{DateSegment, DateSegmentKind};

/// States for the `DateField` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// No segment has focus; the field as a whole is unfocused.
    Idle,

    /// A specific segment has keyboard focus.
    Focused(DateSegmentKind),
}

/// Events for the `DateField` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// User focused a segment by click, tab, or adapter focus.
    FocusSegment(DateSegmentKind),

    /// Focus left the entire field group.
    BlurAll,

    /// Increment a focused segment by one unit, wrapping within its range.
    IncrementSegment(DateSegmentKind),

    /// Decrement a focused segment by one unit, wrapping within its range.
    DecrementSegment(DateSegmentKind),

    /// Type a printable character into the focused segment.
    TypeIntoSegment(DateSegmentKind, char),

    /// Commit the current type-ahead buffer for a segment.
    TypeBufferCommit(DateSegmentKind),

    /// Begin IME composition for segment text input.
    CompositionStart,

    /// End IME composition and process the composed text for a segment.
    CompositionEnd(DateSegmentKind, String),

    /// Clear the value of one segment.
    ClearSegment(DateSegmentKind),

    /// Clear all segment values and reset the field.
    ClearAll,

    /// Programmatically set the date value.
    SetValue(Option<CalendarDate>),

    /// Synchronize context from a new props snapshot.
    SyncProps(Box<Props>),

    /// Move focus to the next editable segment.
    FocusNextSegment,

    /// Move focus to the previous editable segment.
    FocusPrevSegment,
}

/// `DateField` granularity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateGranularity {
    /// Edit year, month, and day segments.
    #[default]
    Day,
}

/// Props for the `DateField` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The ID of the field.
    pub id: String,

    /// The externally controlled value of the field.
    ///
    /// `None` means the field is uncontrolled. `Some(None)` means the parent is
    /// controlling the field as empty. `Some(Some(date))` means the parent is
    /// controlling a concrete date.
    pub value: Option<Option<CalendarDate>>,

    /// The initial value used when the field is uncontrolled.
    pub default_value: Option<CalendarDate>,

    /// The calendar system used to validate and edit date values.
    pub calendar: CalendarSystem,

    /// The editable date granularity.
    pub granularity: DateGranularity,

    /// The minimum allowed date value.
    pub min_value: Option<CalendarDate>,

    /// The maximum allowed date value.
    pub max_value: Option<CalendarDate>,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// Whether the field is required for form submission.
    pub required: bool,

    /// Whether the field should receive focus on mount.
    pub auto_focus: bool,

    /// The visible label text of the field.
    pub label: String,

    /// The accessible label when no visible label is used.
    pub aria_label: Option<String>,

    /// The ID reference used to label the field group.
    pub aria_labelledby: Option<String>,

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

    /// Optional override for rendered segment order.
    pub segment_order: Option<Vec<DateSegmentKind>>,

    /// Whether numeric segments should force fixed-width leading zeroes.
    pub force_leading_zeros: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            calendar: CalendarSystem::Gregorian,
            granularity: DateGranularity::Day,
            min_value: None,
            max_value: None,
            disabled: false,
            readonly: false,
            required: false,
            auto_focus: false,
            label: String::new(),
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            description: None,
            error_message: None,
            invalid: false,
            name: None,
            segment_order: None,
            force_leading_zeros: false,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] value with the `DateField` defaults.
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
    pub fn value(mut self, value: Option<CalendarDate>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`default_value`](Self::default_value), the uncontrolled initial value.
    #[must_use]
    pub fn default_value(mut self, value: Option<CalendarDate>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`calendar`](Self::calendar), the calendar system for validation.
    #[must_use]
    pub const fn calendar(mut self, calendar: CalendarSystem) -> Self {
        self.calendar = calendar;
        self
    }

    /// Sets [`granularity`](Self::granularity), the editable date precision.
    #[must_use]
    pub const fn granularity(mut self, granularity: DateGranularity) -> Self {
        self.granularity = granularity;
        self
    }

    /// Sets [`min_value`](Self::min_value), the minimum allowed date.
    #[must_use]
    pub fn min_value(mut self, value: Option<CalendarDate>) -> Self {
        self.min_value = value;
        self
    }

    /// Sets [`max_value`](Self::max_value), the maximum allowed date.
    #[must_use]
    pub fn max_value(mut self, value: Option<CalendarDate>) -> Self {
        self.max_value = value;
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

    /// Sets [`auto_focus`](Self::auto_focus), whether the field should focus on mount.
    #[must_use]
    pub const fn auto_focus(mut self, value: bool) -> Self {
        self.auto_focus = value;
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

    /// Sets [`aria_labelledby`](Self::aria_labelledby), the field group's label reference.
    #[must_use]
    pub fn aria_labelledby(mut self, id: Option<String>) -> Self {
        self.aria_labelledby = id;
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

    /// Sets [`segment_order`](Self::segment_order), the optional rendered segment order override.
    #[must_use]
    pub fn segment_order(mut self, order: Option<Vec<DateSegmentKind>>) -> Self {
        self.segment_order = order;
        self
    }

    /// Sets [`force_leading_zeros`](Self::force_leading_zeros), whether numeric segments use fixed width.
    #[must_use]
    pub const fn force_leading_zeros(mut self, value: bool) -> Self {
        self.force_leading_zeros = value;
        self
    }
}

/// Context for the `DateField` component.
#[derive(Clone)]
pub struct Context {
    /// Controlled or uncontrolled date value.
    pub value: Bindable<Option<CalendarDate>>,

    /// Segments in display order.
    pub segments: Vec<DateSegment>,

    /// The segment currently holding keyboard focus.
    pub focused_segment: Option<DateSegmentKind>,

    /// Accumulated typed characters for the focused segment.
    pub type_buffer: String,

    /// Pending controlled value held until active segment editing completes.
    pub pending_controlled_value: Option<Option<CalendarDate>>,

    /// Whether an IME composition is currently active.
    pub is_composing: bool,

    /// The resolved locale.
    pub locale: Locale,

    /// Backend used for locale-dependent labels and digits.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// Resolved `DateField` messages.
    pub messages: Messages,

    /// The active calendar system.
    pub calendar: CalendarSystem,

    /// The active date granularity.
    pub granularity: DateGranularity,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// The minimum allowed date value.
    pub min_value: Option<CalendarDate>,

    /// The maximum allowed date value.
    pub max_value: Option<CalendarDate>,

    /// Component id for ARIA correlation.
    pub id: String,

    /// Derived component part ids.
    pub ids: ComponentIds,

    /// Whether the field is invalid.
    pub invalid: bool,

    /// The field-level error-message id.
    pub error_message_id: String,

    /// Whether numeric segments should force fixed-width leading zeroes.
    pub force_leading_zeros: bool,

    segment_order: Option<Vec<DateSegmentKind>>,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("segments", &self.segments)
            .field("focused_segment", &self.focused_segment)
            .field("type_buffer", &self.type_buffer)
            .field("pending_controlled_value", &self.pending_controlled_value)
            .field("is_composing", &self.is_composing)
            .field("locale", &self.locale)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("messages", &self.messages)
            .field("calendar", &self.calendar)
            .field("granularity", &self.granularity)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("min_value", &self.min_value)
            .field("max_value", &self.max_value)
            .field("id", &self.id)
            .field("ids", &self.ids)
            .field("invalid", &self.invalid)
            .field("error_message_id", &self.error_message_id)
            .field("force_leading_zeros", &self.force_leading_zeros)
            .field("segment_order", &self.segment_order)
            .finish()
    }
}

impl Context {
    /// Recomputes segments from the current value, locale, calendar, and granularity.
    pub fn rebuild_segments(&mut self) {
        self.segments = build_segments(self);

        self.refresh_segment_ranges();
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

    /// Returns `true` when every required editable date segment has a value.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        required_date_kinds(self.granularity)
            .into_iter()
            .all(|kind| self.get_segment_value(kind).is_some())
    }

    /// Assembles a validated calendar date from current segment values.
    #[must_use]
    pub fn assemble_date(&self) -> Option<CalendarDate> {
        let year = self.get_segment_value(DateSegmentKind::Year)?;
        let month = u8::try_from(self.get_segment_value(DateSegmentKind::Month)?).ok()?;
        let day = u8::try_from(self.get_segment_value(DateSegmentKind::Day)?).ok()?;

        CalendarDate::new(
            self.calendar,
            &CalendarDateFields {
                era: self.current_era_for_assembly(),
                year: Some(year),
                month: Some(month),
                day: Some(day),
                ..CalendarDateFields::default()
            },
        )
        .ok()
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
        let (min, max) = self.segment_range(kind);

        let value = raw.clamp(min, max);

        let text = self.format_segment_value(kind, value);

        if let Some(segment) = self.segment_mut(kind) {
            segment.min = min;
            segment.max = max;
            segment.value = Some(value);
            segment.text = text;
        }

        self.refresh_segment_ranges();
    }

    /// Clears a segment value.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) {
        if let Some(segment) = self.segment_mut(kind) {
            segment.value = None;

            segment.text.clear();
        }

        self.refresh_segment_ranges();
    }

    fn increment_segment(&mut self, kind: DateSegmentKind) {
        if let Some(current) = self.assemble_date() {
            let field = match kind {
                DateSegmentKind::Year => Some(CalendarDateField::Year),
                DateSegmentKind::Month => Some(CalendarDateField::Month),
                DateSegmentKind::Day => Some(CalendarDateField::Day),
                _ => None,
            };

            if let Some(field) = field
                && let Ok(next) = current.cycle(field, 1, CycleOptions { wrap: true })
            {
                self.apply_date_to_segments(&next);

                return;
            }
        }

        self.step_segment_value(kind, 1);
    }

    fn decrement_segment(&mut self, kind: DateSegmentKind) {
        if let Some(current) = self.assemble_date() {
            let field = match kind {
                DateSegmentKind::Year => Some(CalendarDateField::Year),
                DateSegmentKind::Month => Some(CalendarDateField::Month),
                DateSegmentKind::Day => Some(CalendarDateField::Day),
                _ => None,
            };

            if let Some(field) = field
                && let Ok(next) = current.cycle(field, -1, CycleOptions { wrap: true })
            {
                self.apply_date_to_segments(&next);

                return;
            }
        }

        self.step_segment_value(kind, -1);
    }

    fn step_segment_value(&mut self, kind: DateSegmentKind, delta: i32) {
        let (min, max) = self.segment_range(kind);

        let current =
            self.get_segment_value(kind)
                .unwrap_or(if delta >= 0 { min - 1 } else { max + 1 });

        let span = max - min + 1;

        let next = (current - min + delta).rem_euclid(span) + min;

        self.set_segment_value(kind, next);
    }

    fn apply_date_to_segments(&mut self, date: &CalendarDate) {
        if let Some(index) = self.era_index_for_date(date) {
            self.set_segment_value(DateSegmentKind::Era, index);
        }

        self.set_segment_value(DateSegmentKind::Year, date.year());
        self.set_segment_value(DateSegmentKind::Month, i32::from(date.month()));
        self.set_segment_value(DateSegmentKind::Day, i32::from(date.day()));
    }

    fn refresh_segment_ranges(&mut self) {
        let formatted = self
            .segments
            .iter()
            .map(|segment| {
                let (min, max) = self.segment_range(segment.kind);
                let text = segment
                    .value
                    .map(|value| self.format_segment_value(segment.kind, value.clamp(min, max)));
                (segment.kind, min, max, text)
            })
            .collect::<Vec<_>>();

        let locale = self.locale.clone();

        let backend = Arc::clone(&self.intl_backend);

        let calendar = self.calendar;

        let force = self.force_leading_zeros;

        let day_range = self.segment_range(DateSegmentKind::Day);

        let month_range = self.segment_range(DateSegmentKind::Month);

        for segment in &mut self.segments {
            let (mut min, mut max, text) = formatted
                .iter()
                .find(|(kind, ..)| *kind == segment.kind)
                .map(|(_, min, max, text)| (*min, *max, text.clone()))
                .unwrap_or((segment.min, segment.max, None));

            if segment.kind == DateSegmentKind::Month {
                (min, max) = month_range;
            } else if segment.kind == DateSegmentKind::Day {
                (min, max) = day_range;
            }

            segment.min = min;
            segment.max = max;

            if let Some(value) = segment.value {
                let clamped = value.clamp(min, max);

                segment.value = Some(clamped);

                segment.text = text.unwrap_or_else(|| {
                    format_segment_value_for(
                        backend.as_ref(),
                        &locale,
                        calendar,
                        segment.kind,
                        clamped,
                        force,
                    )
                });
            }
        }
    }

    fn segment_range(&self, kind: DateSegmentKind) -> (i32, i32) {
        match kind {
            DateSegmentKind::Year => {
                let max = self
                    .current_era_for_assembly()
                    .and_then(|era| {
                        CalendarDate::new(
                            self.calendar,
                            &CalendarDateFields {
                                era: Some(era),
                                year: Some(1),
                                month: Some(1),
                                day: Some(1),
                                ..CalendarDateFields::default()
                            },
                        )
                        .ok()
                    })
                    .and_then(|date| self.intl_backend.years_in_era(&date))
                    .unwrap_or(9999);

                (1, max)
            }

            DateSegmentKind::Month => {
                let era = self.current_era_for_assembly();

                let era_code = era.as_ref().map(|era| era.code.as_str());

                let max = self
                    .get_segment_value(DateSegmentKind::Year)
                    .map_or(12, |year| {
                        self.intl_backend
                            .max_months_in_year(&self.calendar, year, era_code)
                    });

                (1, i32::from(max))
            }

            DateSegmentKind::Day => {
                let era = self.current_era_for_assembly();

                let era_code = era.as_ref().map(|era| era.code.as_str());

                let max = match (
                    self.get_segment_value(DateSegmentKind::Year),
                    self.get_segment_value(DateSegmentKind::Month),
                ) {
                    (Some(year), Some(month)) => u8::try_from(month).ok().map_or(31, |month| {
                        self.intl_backend
                            .days_in_month(&self.calendar, year, month, era_code)
                    }),

                    _ => 31,
                };

                (1, i32::from(max))
            }

            DateSegmentKind::Hour => (0, 23),

            DateSegmentKind::Minute | DateSegmentKind::Second => (0, 59),

            DateSegmentKind::DayPeriod => (0, 1),

            DateSegmentKind::Era => {
                let max = i32::try_from(self.calendar.eras().len())
                    .unwrap_or(1)
                    .max(1);

                (1, max)
            }

            DateSegmentKind::Weekday | DateSegmentKind::Literal | DateSegmentKind::TimeZoneName => {
                (0, 0)
            }
        }
    }

    fn current_era_for_assembly(&self) -> Option<Era> {
        if let Some(value) = self.get_segment_value(DateSegmentKind::Era)
            && let Some(era) = self.era_for_value(value)
        {
            return Some(era);
        }

        self.value
            .get()
            .as_ref()
            .and_then(|date| date.era().cloned())
            .or_else(|| self.intl_backend.default_era(&self.calendar))
    }

    fn era_for_value(&self, value: i32) -> Option<Era> {
        let index = usize::try_from(value.checked_sub(1)?).ok()?;

        self.calendar.eras().get(index).cloned()
    }

    fn era_index_for_date(&self, date: &CalendarDate) -> Option<i32> {
        let era = date
            .era()
            .cloned()
            .or_else(|| self.intl_backend.default_era(&self.calendar))?;

        self.calendar
            .eras()
            .iter()
            .position(|candidate| candidate.code == era.code)
            .and_then(|index| i32::try_from(index + 1).ok())
    }

    fn format_segment_value(&self, kind: DateSegmentKind, value: i32) -> String {
        if kind == DateSegmentKind::Era
            && let Some(era) = self.era_for_value(value)
        {
            return localized_era_name(self.calendar, &era, &self.locale);
        }

        format_segment_value(
            self.intl_backend.as_ref(),
            &self.locale,
            kind,
            value,
            self.force_leading_zeros,
        )
    }
}

/// Localized labels for `DateField` segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the year segment.
    pub year_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the month segment.
    pub month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the day segment.
    pub day_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the hour segment.
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the minute segment.
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the second segment.
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the day-period segment.
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the era segment.
    pub era_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the weekday segment.
    pub weekday_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the time-zone-name segment.
    pub timezone_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            year_label: MessageFn::static_str("Year"),
            month_label: MessageFn::static_str("Month"),
            day_label: MessageFn::static_str("Day"),
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
            era_label: MessageFn::static_str("Era"),
            weekday_label: MessageFn::static_str("Day of week"),
            timezone_label: MessageFn::static_str("Time zone"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the `date_field` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts the type-buffer commit timer.
    TypeBufferCommit,
}

/// Machine for the `DateField` component.
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
        let ids = ComponentIds::from_id(&props.id);

        let value = if let Some(value) = &props.value {
            Bindable::controlled(value.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let calendar = resolve_calendar(&env.locale, props.calendar);

        let mut ctx = Context {
            value,
            segments: Vec::new(),
            focused_segment: None,
            type_buffer: String::new(),
            pending_controlled_value: None,
            is_composing: false,
            locale: env.locale.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            messages: messages.clone(),
            calendar,
            granularity: props.granularity,
            disabled: props.disabled,
            readonly: props.readonly,
            min_value: props.min_value.clone(),
            max_value: props.max_value.clone(),
            id: props.id.clone(),
            error_message_id: ids.part("error-message"),
            ids,
            invalid: props.invalid,
            force_leading_zeros: props.force_leading_zeros,
            segment_order: props.segment_order.clone(),
        };

        ctx.rebuild_segments();

        let state = if props.auto_focus && !props.disabled {
            if let Some(first) = ctx.first_editable() {
                ctx.focused_segment = Some(first);
                State::Focused(first)
            } else {
                State::Idle
            }
        } else {
            State::Idle
        };

        (state, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "date_field::Props.id must remain stable after init"
        );

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
        if ctx.disabled
            && !matches!(
                event,
                Event::SetValue(_) | Event::SyncProps(_) | Event::CompositionEnd(_, _)
            )
        {
            return None;
        }

        match event {
            Event::SyncProps(props) => {
                let props = props.as_ref().clone();
                let mut next_ctx = ctx.clone();

                sync_props(&mut next_ctx, &props);

                if let Some(next_state) = reconcile_state_after_sync(state, &next_ctx) {
                    Some(
                        TransitionPlan::to(next_state.clone()).apply(move |ctx: &mut Context| {
                            sync_props(ctx, &props);
                            apply_state_focus(ctx, &next_state);
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        sync_props(ctx, &props);
                    }))
                }
            }

            Event::FocusSegment(kind) => {
                if !kind.is_editable() {
                    return None;
                }

                let kind = *kind;
                Some(
                    TransitionPlan::to(State::Focused(kind)).apply(move |ctx: &mut Context| {
                        commit_type_buffer(ctx);
                        flush_pending_controlled_value(ctx);

                        ctx.focused_segment = Some(kind);
                        ctx.type_buffer.clear();
                    }),
                )
            }

            Event::BlurAll => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        commit_type_buffer(ctx);

                        flush_pending_controlled_value(ctx);

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

                                flush_pending_controlled_value(ctx);

                                ctx.focused_segment = next;
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
                                ctx.type_buffer.clear();

                                flush_pending_controlled_value(ctx);

                                ctx.focused_segment = Some(previous);
                            })
                            .cancel_effect(Effect::TypeBufferCommit),
                    )
                } else {
                    None
                }
            }

            Event::IncrementSegment(kind) => {
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

            Event::DecrementSegment(kind) => {
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

            Event::TypeIntoSegment(kind, ch) => {
                if ctx.readonly || ctx.is_composing || !matches!(state, State::Focused(_)) {
                    return None;
                }

                type_into_segment(ctx, state, *kind, *ch)
            }

            Event::TypeBufferCommit(kind) => {
                if ctx.readonly {
                    return None;
                }

                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    commit_buffer_for_kind(ctx, kind);
                }))
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd(kind, text) => {
                let kind = *kind;
                let text = text.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.is_composing = false;

                    if ctx.readonly || ctx.disabled {
                        return;
                    }

                    ctx.type_buffer.clear();

                    for ch in text.chars() {
                        if kind.is_numeric() && ch.is_ascii_digit() {
                            ctx.type_buffer.push(ch);
                        }
                    }

                    commit_buffer_for_kind(ctx, kind);
                }))
            }

            Event::ClearSegment(kind) => {
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
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_value_update(ctx, value);
                }))
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

/// Structural parts exposed by the `DateField` connect API.
#[derive(ComponentPart)]
#[scope = "date-field"]
pub enum Part {
    /// The root container.
    Root,

    /// The visible label.
    Label,

    /// The group containing all rendered segments.
    FieldGroup,

    /// An editable date segment.
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

/// API for the `DateField` component.
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
    /// Creates a `DateField` connect API from machine state.
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

        if self.ctx.invalid {
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
        } else if let Some(labelledby) = &self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby);
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

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        let mut described_by = Vec::new();

        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.invalid && self.props.error_message.is_some() {
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
        let Some(segment) = self
            .ctx
            .segments
            .iter()
            .find(|segment| segment.kind == *kind)
        else {
            return AttrMap::new();
        };

        if !segment.is_editable {
            return self.literal_attrs(0);
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
                segment
                    .kind
                    .aria_label(&self.ctx.messages, &self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), segment.min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), segment.max.to_string())
            .set(HtmlAttr::Data("ars-segment"), segment.kind.data_name())
            .set(HtmlAttr::TabIndex, if is_tab_stop { "0" } else { "-1" });

        if let Some(value) = segment.value {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string());

            if let Some(text) = if segment.kind == DateSegmentKind::Era {
                self.ctx
                    .era_for_value(value)
                    .map(|era| localized_era_name(self.ctx.calendar, &era, &self.ctx.locale))
            } else {
                segment.aria_value_text(self.ctx.intl_backend.as_ref(), &self.ctx.locale)
            } {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), text);
            }
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &segment.placeholder);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.invalid && self.props.error_message.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                &self.ctx.error_message_id,
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
            .as_ref()
            .map(CalendarDate::to_iso8601)
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
            (self.send)(Event::CompositionStart);

            return;
        }

        let is_rtl = dir.is_rtl();

        match data.key {
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementSegment(kind)),

            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment(kind)),

            KeyboardKey::ArrowRight if is_rtl => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::ArrowLeft if is_rtl => (self.send)(Event::FocusNextSegment),

            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::Tab if shift => (self.send)(Event::FocusPrevSegment),

            KeyboardKey::ArrowRight | KeyboardKey::Tab => (self.send)(Event::FocusNextSegment),

            KeyboardKey::Backspace | KeyboardKey::Delete => (self.send)(Event::ClearSegment(kind)),

            KeyboardKey::Escape => (self.send)(Event::ClearAll),

            _ if data.character.is_some() => {
                if let Some(ch) = data.character {
                    (self.send)(Event::TypeIntoSegment(kind, ch));
                }
            }
            _ => {}
        }
    }

    /// Handles focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
    }

    /// Handles click on a segment.
    pub fn on_segment_click(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
    }

    /// Handles composition start on a segment.
    pub fn on_segment_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Handles composition end on a segment.
    pub fn on_segment_composition_end(&self, kind: DateSegmentKind, text: impl Into<String>) {
        (self.send)(Event::CompositionEnd(kind, text.into()));
    }

    /// Returns the rendered segments.
    #[must_use]
    pub fn segments(&self) -> &[DateSegment] {
        &self.ctx.segments
    }

    /// Returns the current date value.
    #[must_use]
    pub fn value(&self) -> Option<&CalendarDate> {
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
        let value = ctx
            .intl_backend
            .day_period_from_char(ch, &ctx.locale)
            .map(|is_pm| if is_pm { 1 } else { 0 })?;

        let next = ctx.next_editable_after(kind);

        let target = next.map_or_else(|| state.clone(), State::Focused);

        return Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
            ctx.set_segment_value(kind, value);

            ctx.type_buffer.clear();

            if let Some(next) = next {
                ctx.focused_segment = Some(next);
            }

            maybe_publish(ctx);
        }));
    }

    if kind == DateSegmentKind::Month && ch.is_alphabetic() {
        return type_month_name(ctx, state, ch);
    }

    if !kind.is_numeric() || !ch.is_ascii_digit() {
        return None;
    }

    let mut new_buffer = ctx.type_buffer.clone();

    new_buffer.push(ch);

    let buffered = new_buffer.parse::<i32>().ok()?;

    let (_, max) = ctx.segment_range(kind);

    let max_digits = digits_needed(u32::try_from(max).ok()?);

    let should_advance = new_buffer.len() >= max_digits || buffered * 10 > max;

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

        let (_, max) = ctx.segment_range(kind);

        if let Ok(value) = ctx.type_buffer.parse::<i32>()
            && value <= max
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

fn type_month_name(ctx: &Context, state: &State, ch: char) -> Option<TransitionPlan<Machine>> {
    let mut new_buffer = ctx.type_buffer.clone();

    for ch in ch.to_lowercase() {
        new_buffer.push(ch);
    }

    let matches = month_name_prefix_matches(ctx, &new_buffer);

    let selected = *matches.first()?;

    let should_advance = matches.len() == 1;

    let next = should_advance
        .then(|| ctx.next_editable_after(DateSegmentKind::Month))
        .flatten();

    let target = if should_advance {
        next.map_or_else(|| state.clone(), State::Focused)
    } else {
        state.clone()
    };

    let mut plan = TransitionPlan::to(target).apply(move |ctx: &mut Context| {
        ctx.type_buffer = new_buffer;

        ctx.set_segment_value(DateSegmentKind::Month, i32::from(selected));

        maybe_publish(ctx);

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
        commit_buffer_for_kind(ctx, kind);
    }
}

fn commit_buffer_for_kind(ctx: &mut Context, kind: DateSegmentKind) {
    if kind == DateSegmentKind::Month
        && ctx.type_buffer.chars().any(char::is_alphabetic)
        && let Some(month) = month_name_prefix_matches(ctx, &ctx.type_buffer).first()
    {
        ctx.set_segment_value(kind, i32::from(*month));

        maybe_publish(ctx);

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

    let Some(date) = ctx.assemble_date() else {
        return;
    };

    let clamped = clamp_date(date, ctx.min_value.as_ref(), ctx.max_value.as_ref());

    ctx.apply_date_to_segments(&clamped);

    ctx.value.set(Some(clamped));
}

fn clamp_date(
    date: CalendarDate,
    min_value: Option<&CalendarDate>,
    max_value: Option<&CalendarDate>,
) -> CalendarDate {
    if let Some(min) = min_value
        && date.compare(min) == Ordering::Less
    {
        return min.clone();
    }

    if let Some(max) = max_value
        && date.compare(max) == Ordering::Greater
    {
        return max.clone();
    }

    date
}

fn apply_controlled_value_update(ctx: &mut Context, value: Option<CalendarDate>) -> bool {
    let value = clamp_optional_date(ctx, value);

    if ctx.focused_segment.is_some() && !ctx.type_buffer.is_empty() {
        ctx.pending_controlled_value = Some(value);

        return false;
    }

    ctx.pending_controlled_value = None;

    apply_controlled_value(ctx, value);

    true
}

fn apply_value_update(ctx: &mut Context, value: Option<CalendarDate>) -> bool {
    let value = clamp_optional_date(ctx, value);

    if ctx.focused_segment.is_some() && !ctx.type_buffer.is_empty() {
        ctx.pending_controlled_value = Some(value);

        return false;
    }

    apply_value(ctx, value);

    true
}

fn flush_pending_controlled_value(ctx: &mut Context) {
    if let Some(value) = ctx.pending_controlled_value.take() {
        apply_value(ctx, value);
    }
}

fn apply_value(ctx: &mut Context, value: Option<CalendarDate>) {
    let value = clamp_optional_date(ctx, value);

    if ctx.value.is_controlled() {
        ctx.value.set(value.clone());
        ctx.value.sync_controlled(Some(value));
    } else {
        ctx.value.set(value);
    }

    ctx.rebuild_segments();
}

fn apply_controlled_value(ctx: &mut Context, value: Option<CalendarDate>) {
    ctx.value.set(value.clone());
    ctx.value.sync_controlled(Some(value));

    ctx.rebuild_segments();
}

fn clamp_optional_date(ctx: &Context, value: Option<CalendarDate>) -> Option<CalendarDate> {
    value.map(|date| clamp_date(date, ctx.min_value.as_ref(), ctx.max_value.as_ref()))
}

fn sync_props(ctx: &mut Context, props: &Props) {
    let previous_calendar = ctx.calendar;
    let previous_granularity = ctx.granularity;
    let previous_segment_order = ctx.segment_order.clone();
    let previous_force_leading_zeros = ctx.force_leading_zeros;

    let buffered_segment = ctx
        .focused_segment
        .filter(|_| !ctx.type_buffer.is_empty())
        .map(|kind| (kind, ctx.get_segment_value(kind)));

    ctx.calendar = resolve_calendar(&ctx.locale, props.calendar);
    ctx.granularity = props.granularity;
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.min_value = props.min_value.clone();
    ctx.max_value = props.max_value.clone();
    ctx.invalid = props.invalid;
    ctx.force_leading_zeros = props.force_leading_zeros;
    ctx.segment_order = props.segment_order.clone();

    let value_application_deferred = if let Some(value) = &props.value {
        !apply_controlled_value_update(ctx, value.clone())
    } else {
        let preserve_buffered_handoff =
            ctx.focused_segment.is_some() && !ctx.type_buffer.is_empty();

        ctx.pending_controlled_value = None;
        ctx.value.sync_controlled(None);

        preserve_buffered_handoff
    };

    let must_rebuild = previous_calendar != ctx.calendar
        || previous_granularity != ctx.granularity
        || previous_segment_order != ctx.segment_order
        || previous_force_leading_zeros != ctx.force_leading_zeros;

    if must_rebuild {
        ctx.rebuild_segments();
    } else {
        ctx.refresh_segment_ranges();
    }

    if value_application_deferred {
        restore_buffered_segment(ctx, buffered_segment);

        return;
    }

    if let Some(value) = ctx.value.get().clone() {
        let clamped = clamp_date(value, ctx.min_value.as_ref(), ctx.max_value.as_ref());

        ctx.apply_date_to_segments(&clamped);

        apply_current_value(ctx, Some(clamped));
    }
}

fn apply_current_value(ctx: &mut Context, value: Option<CalendarDate>) {
    if ctx.value.is_controlled() {
        ctx.value.set(value.clone());
        ctx.value.sync_controlled(Some(value));
    } else {
        ctx.value.set(value);
    }
}

fn restore_buffered_segment(
    ctx: &mut Context,
    buffered_segment: Option<(DateSegmentKind, Option<i32>)>,
) {
    let Some((kind, value)) = buffered_segment else {
        return;
    };

    if !ctx
        .segments
        .iter()
        .any(|segment| segment.kind == kind && segment.is_editable)
    {
        return;
    }

    if let Some(value) = value {
        ctx.set_segment_value(kind, value);
    } else {
        ctx.clear_segment_value(kind);
    }
}

fn reconcile_state_after_sync(state: &State, ctx: &Context) -> Option<State> {
    let State::Focused(kind) = state else {
        return None;
    };

    let still_focusable = !ctx.disabled
        && ctx
            .segments
            .iter()
            .any(|segment| segment.kind == *kind && segment.is_editable);

    if still_focusable {
        return None;
    }

    Some(
        (!ctx.disabled)
            .then(|| ctx.first_editable())
            .flatten()
            .map_or(State::Idle, State::Focused),
    )
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

fn build_segments(ctx: &Context) -> Vec<DateSegment> {
    let mut segments = Vec::new();

    let pattern = segment_pattern(ctx);

    let value = ctx.value.get().clone();

    for part in pattern {
        let kind = match part {
            SegmentPatternPart::Segment(kind) => kind,

            SegmentPatternPart::Literal(literal) => {
                segments.push(DateSegment::new_literal(literal));

                continue;
            }
        };

        let (min, max, placeholder) = match kind {
            DateSegmentKind::Year => (1, 9999, "yyyy"),

            DateSegmentKind::Month => (1, 12, "mm"),

            DateSegmentKind::Day => (1, 31, "dd"),

            DateSegmentKind::Era => (
                1,
                i32::try_from(ctx.calendar.eras().len()).unwrap_or(1).max(1),
                "era",
            ),

            _ => (0, 0, ""),
        };

        if !kind.is_editable() {
            segments.push(DateSegment::new_non_editable(
                kind,
                &non_editable_segment_text(ctx, kind, value.as_ref()),
            ));

            continue;
        }

        let mut segment = DateSegment::new_numeric(kind, min, max, placeholder);

        if let Some(date) = &value {
            let segment_value = match kind {
                DateSegmentKind::Year => Some(date.year()),

                DateSegmentKind::Month => Some(i32::from(date.month())),

                DateSegmentKind::Day => Some(i32::from(date.day())),

                DateSegmentKind::Era => ctx.era_index_for_date(date),

                _ => None,
            };

            if let Some(segment_value) = segment_value {
                segment.value = Some(segment_value);
                segment.text = ctx.format_segment_value(kind, segment_value);
            }
        }

        segments.push(segment);
    }

    segments
}

fn non_editable_segment_text(
    ctx: &Context,
    kind: DateSegmentKind,
    value: Option<&CalendarDate>,
) -> String {
    match (kind, value) {
        (DateSegmentKind::Weekday, Some(date)) => ctx
            .intl_backend
            .weekday_short_label(date.weekday(), &ctx.locale),

        (DateSegmentKind::Literal, _) => literal_for_locale(&ctx.locale).to_string(),

        _ => kind.aria_label(&ctx.messages, &ctx.locale),
    }
}

#[derive(Clone, Copy)]
enum SegmentPatternPart {
    Segment(DateSegmentKind),
    Literal(&'static str),
}

fn segment_pattern(ctx: &Context) -> Vec<SegmentPatternPart> {
    if let Some(order) = &ctx.segment_order {
        return intersperse_segments(order.iter().copied(), literal_for_locale(&ctx.locale));
    }

    if ctx.calendar == CalendarSystem::Japanese {
        return vec![
            SegmentPatternPart::Segment(DateSegmentKind::Era),
            SegmentPatternPart::Segment(DateSegmentKind::Year),
            SegmentPatternPart::Literal("年"),
            SegmentPatternPart::Segment(DateSegmentKind::Month),
            SegmentPatternPart::Literal("月"),
            SegmentPatternPart::Segment(DateSegmentKind::Day),
            SegmentPatternPart::Literal("日"),
        ];
    }

    intersperse_segments(
        segment_order_for_locale(&ctx.locale, ctx.calendar),
        literal_for_locale(&ctx.locale),
    )
}

fn intersperse_segments(
    order: impl IntoIterator<Item = DateSegmentKind>,
    literal: &'static str,
) -> Vec<SegmentPatternPart> {
    let mut pattern = Vec::new();

    for (index, kind) in order.into_iter().enumerate() {
        if index > 0 {
            pattern.push(SegmentPatternPart::Literal(literal));
        }

        pattern.push(SegmentPatternPart::Segment(kind));
    }

    pattern
}

fn segment_order_for_locale(locale: &Locale, calendar: CalendarSystem) -> Vec<DateSegmentKind> {
    if calendar == CalendarSystem::Japanese {
        return vec![
            DateSegmentKind::Era,
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ];
    }

    match (locale.language(), locale.region()) {
        ("en", Some("US" | "FM" | "PW")) => vec![
            DateSegmentKind::Month,
            DateSegmentKind::Day,
            DateSegmentKind::Year,
        ],

        ("ja" | "zh" | "ko", _)
        | (_, Some("CA" | "CN" | "HU" | "JP" | "KR" | "LT" | "MN" | "TW")) => {
            vec![
                DateSegmentKind::Year,
                DateSegmentKind::Month,
                DateSegmentKind::Day,
            ]
        }

        _ => vec![
            DateSegmentKind::Day,
            DateSegmentKind::Month,
            DateSegmentKind::Year,
        ],
    }
}

fn literal_for_locale(locale: &Locale) -> &'static str {
    match (locale.language(), locale.region()) {
        ("de", Some("DE")) => ".",
        ("ko", Some("KR")) => ". ",
        _ => "/",
    }
}

fn required_date_kinds(granularity: DateGranularity) -> Vec<DateSegmentKind> {
    match granularity {
        DateGranularity::Day => vec![
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ],
    }
}

fn format_segment_value(
    backend: &dyn IntlBackend,
    locale: &Locale,
    kind: DateSegmentKind,
    value: i32,
    force_leading_zeros: bool,
) -> String {
    if kind == DateSegmentKind::DayPeriod {
        return backend.day_period_label(value == 1, locale);
    }

    let width: u8 = match kind {
        DateSegmentKind::Year => 4,
        _ => 2,
    };

    if force_leading_zeros {
        let width = usize::from(width);

        format!("{value:0>width$}")
    } else {
        backend.format_segment_digits(
            u32::try_from(value).unwrap_or_default(),
            NonZeroU8::new(width).expect("segment width is non-zero"),
            locale,
        )
    }
}

fn format_segment_value_for(
    backend: &dyn IntlBackend,
    locale: &Locale,
    calendar: CalendarSystem,
    kind: DateSegmentKind,
    value: i32,
    force_leading_zeros: bool,
) -> String {
    if kind == DateSegmentKind::Era
        && let Some(era) = calendar
            .eras()
            .get(usize::try_from(value.saturating_sub(1)).unwrap_or(0))
    {
        return localized_era_name(calendar, era, locale);
    }

    format_segment_value(backend, locale, kind, value, force_leading_zeros)
}

fn localized_era_name(calendar: CalendarSystem, era: &Era, locale: &Locale) -> String {
    if calendar == CalendarSystem::Japanese
        && let Some(japanese) = CalendarSystem::japanese_eras()
            .iter()
            .find(|candidate| candidate.code == era.code)
    {
        return String::from(japanese.localized_name(locale));
    }

    era.display_name.clone()
}

fn month_name_prefix_matches(ctx: &Context, buffer: &str) -> Vec<u8> {
    let needle = buffer.to_lowercase();

    let (_, max) = ctx.segment_range(DateSegmentKind::Month);

    let max = u8::try_from(max).unwrap_or(12);

    let mut matches = Vec::new();

    for month in 1..=max {
        let label = ctx
            .intl_backend
            .month_long_name(month, &ctx.locale)
            .to_lowercase();

        if label.starts_with(&needle) {
            matches.push(month);
        }
    }

    matches
}

fn resolve_calendar(locale: &Locale, requested: CalendarSystem) -> CalendarSystem {
    if requested == CalendarSystem::Gregorian {
        CalendarSystem::from_locale(locale)
    } else {
        requested
    }
}

fn digits_needed(n: u32) -> usize {
    if n == 0 {
        1
    } else {
        usize::try_from(n.ilog10()).unwrap_or(0) + 1
    }
}
