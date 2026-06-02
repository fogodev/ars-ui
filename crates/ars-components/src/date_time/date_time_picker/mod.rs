//! DateTimePicker component state machine and connect API.
//!
//! This module implements the framework-agnostic composite defined in
//! `spec/components/date-time/date-time-picker.md`. It unifies a segmented date
//! input (like [`date_field`](super::date_field)), a segmented time input (like
//! [`time_field`](super::time_field)), and a calendar popover (an embedded
//! [`calendar`](super::calendar)) into a single state machine that owns both
//! date and time segments and produces a combined [`CalendarDateTime`] value.
//!
//! ## Implementation note: effects
//!
//! Like the sibling `date_picker`/`time_field` machines, this machine never
//! performs DOM focus moves itself. It emits typed [`Effect`] intents via
//! [`PendingEffect::named`]; the adapter dispatches the real focus operation
//! (move focus into the calendar, restore focus to the trigger, focus the first
//! time segment, start the type-ahead commit timer) on each named effect using
//! live element handles. Stable string IDs are used only for semantic ARIA
//! relationships and `data-ars-*` hooks, not as a substitute for live handles.

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
    CalendarDate, CalendarDateFields, CalendarDateTime, CalendarSystem, DateOrder, Era, HourCycle,
    Time, date_field_separator, date_order,
};
use ars_interactions::KeyboardEventData;

use super::{
    calendar,
    date_field::segment::{DateSegment, DateSegmentKind},
    hour_cycle::{
        digits_needed, display_hour, display_hour_range, display_hour_to_24, has_day_period, is_pm,
    },
    time_field::TimeGranularity,
};

/// States for the `DateTimePicker` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Idle; no segment has focus and the popover is closed.
    Idle,

    /// A segment (date or time) has focus; the popover is closed.
    Focused,

    /// The calendar popover is open.
    Open,
}

/// Events for the `DateTimePicker` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the calendar popover.
    Open,

    /// Close the calendar popover.
    Close,

    /// Toggle the calendar popover.
    Toggle,

    /// A date was selected from the calendar popover.
    CalendarSelectDate(CalendarDate),

    /// Focus moved to a specific segment.
    FocusSegment(DateSegmentKind),

    /// Focus moved to the next editable segment.
    FocusNextSegment,

    /// Focus moved to the previous editable segment.
    FocusPrevSegment,

    /// Increment the focused segment value by one unit, wrapping.
    IncrementSegment {
        /// The segment to increment.
        segment: DateSegmentKind,
    },

    /// Decrement the focused segment value by one unit, wrapping.
    DecrementSegment {
        /// The segment to decrement.
        segment: DateSegmentKind,
    },

    /// A printable character was typed into a segment.
    TypeIntoSegment {
        /// The segment receiving typed input.
        segment: DateSegmentKind,

        /// The typed character.
        ch: char,
    },

    /// The type-ahead buffer timer fired; commit buffered digits.
    TypeBufferCommit {
        /// The segment whose buffer should commit.
        segment: DateSegmentKind,
    },

    /// A segment value changed programmatically.
    SegmentChange {
        /// The segment that changed.
        segment: DateSegmentKind,

        /// The new raw value.
        value: i32,
    },

    /// Commit (or clear) the full combined date-time value.
    ValueCommit(Option<CalendarDateTime>),

    /// Clear the value of one segment.
    ClearSegment {
        /// The segment to clear.
        segment: DateSegmentKind,
    },

    /// Clear the entire date-time value.
    ClearAll,

    /// Focus entered the component.
    FocusIn,

    /// Focus left the component entirely.
    FocusOut,

    /// Keyboard shortcut on a segment, trigger, or popover.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },

    /// Synchronize context from a new props snapshot.
    SyncProps(Box<Props>),
}

/// Props for the `DateTimePicker` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The stable DOM id of the date-time picker.
    pub id: String,

    /// Controlled date-time value. `Some(v)` = controlled, `None` = uncontrolled.
    pub value: Option<Option<CalendarDateTime>>,

    /// Default date-time for uncontrolled mode.
    pub default_value: Option<CalendarDateTime>,

    /// Minimum allowed date-time.
    pub min_value: Option<CalendarDateTime>,

    /// Maximum allowed date-time.
    pub max_value: Option<CalendarDateTime>,

    /// Finest time segment to display.
    pub granularity: TimeGranularity,

    /// Whether the component is non-interactive.
    pub disabled: bool,

    /// Whether the component allows viewing but not editing.
    pub readonly: bool,

    /// Form field name for hidden input submission.
    pub name: Option<String>,

    /// Calendar system used by the embedded calendar.
    pub calendar: CalendarSystem,

    /// Hour cycle override. `None` uses the locale default.
    pub hour_cycle: Option<HourCycle>,

    /// Whether the field is required.
    pub required: bool,

    /// Label text.
    pub label: String,

    /// Description/help text.
    pub description: Option<String>,

    /// Error message.
    pub error_message: Option<String>,

    /// Whether the field is in an invalid state.
    pub invalid: bool,

    /// Right-to-left layout direction.
    pub is_rtl: bool,

    /// Number of months to display in the calendar popover.
    pub visible_months: usize,

    /// The "today" date, injected by the adapter for testability and SSR. It is
    /// forwarded to the embedded calendar so the popover opens on the current
    /// month and marks the correct day.
    pub today: CalendarDate,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min_value: None,
            max_value: None,
            granularity: TimeGranularity::Minute,
            disabled: false,
            readonly: false,
            name: None,
            calendar: CalendarSystem::Gregorian,
            hour_cycle: None,
            required: false,
            label: String::new(),
            description: None,
            error_message: None,
            invalid: false,
            is_rtl: false,
            visible_months: 1,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] value with the `DateTimePicker` defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the stable DOM id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), the externally controlled value.
    #[must_use]
    pub fn value(mut self, value: Option<CalendarDateTime>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`default_value`](Self::default_value), the uncontrolled initial value.
    #[must_use]
    pub fn default_value(mut self, value: Option<CalendarDateTime>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`min_value`](Self::min_value), the minimum allowed date-time.
    #[must_use]
    pub fn min_value(mut self, value: Option<CalendarDateTime>) -> Self {
        self.min_value = value;
        self
    }

    /// Sets [`max_value`](Self::max_value), the maximum allowed date-time.
    #[must_use]
    pub fn max_value(mut self, value: Option<CalendarDateTime>) -> Self {
        self.max_value = value;
        self
    }

    /// Sets [`granularity`](Self::granularity), the finest time segment shown.
    #[must_use]
    pub const fn granularity(mut self, granularity: TimeGranularity) -> Self {
        self.granularity = granularity;
        self
    }

    /// Sets [`disabled`](Self::disabled), whether user interaction is blocked.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`readonly`](Self::readonly), whether editing is blocked.
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`name`](Self::name), the hidden input's form field name.
    #[must_use]
    pub fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Sets [`calendar`](Self::calendar), the embedded calendar system.
    #[must_use]
    pub const fn calendar(mut self, calendar: CalendarSystem) -> Self {
        self.calendar = calendar;
        self
    }

    /// Sets [`hour_cycle`](Self::hour_cycle), overriding the locale default.
    #[must_use]
    pub const fn hour_cycle(mut self, hour_cycle: Option<HourCycle>) -> Self {
        self.hour_cycle = hour_cycle;
        self
    }

    /// Sets [`required`](Self::required), whether the field is required.
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`label`](Self::label), the visible label text.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
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

    /// Sets [`is_rtl`](Self::is_rtl), the layout direction flag.
    #[must_use]
    pub const fn is_rtl(mut self, value: bool) -> Self {
        self.is_rtl = value;
        self
    }

    /// Sets [`visible_months`](Self::visible_months), the calendar month count.
    #[must_use]
    pub const fn visible_months(mut self, value: usize) -> Self {
        self.visible_months = value;
        self
    }

    /// Sets [`today`](Self::today), the adapter-injected current date.
    #[must_use]
    pub fn today(mut self, today: CalendarDate) -> Self {
        self.today = today;
        self
    }
}

/// Context for the `DateTimePicker` component.
#[derive(Clone)]
pub struct Context {
    /// The combined date-time value (controlled/uncontrolled).
    pub value: Bindable<Option<CalendarDateTime>>,

    /// The date portion (synced from value or calendar selection).
    pub date_value: Option<CalendarDate>,

    /// The time portion (synced from value or time segment edits).
    pub time_value: Option<Time>,

    /// The currently focused segment (date or time).
    pub focused_segment: Option<DateSegmentKind>,

    /// Whether the calendar popover is open.
    pub open: bool,

    /// Date segments (Year, Month, Day) in locale order, with literals.
    pub date_segments: Vec<DateSegment>,

    /// Time segments (Hour, Minute, optional Second/DayPeriod), with literals.
    pub time_segments: Vec<DateSegment>,

    /// The type-ahead buffer for numeric segment editing.
    pub type_buffer: String,

    /// The resolved locale.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Backend used for locale-dependent hour-cycle resolution.
    pub intl_backend: Arc<dyn IntlBackend>,

    /// The active time granularity.
    pub granularity: TimeGranularity,

    /// The resolved hour cycle.
    pub hour_cycle: HourCycle,

    /// The calendar system used by the embedded calendar.
    pub calendar: CalendarSystem,

    /// Minimum allowed date-time.
    pub min_value: Option<CalendarDateTime>,

    /// Maximum allowed date-time.
    pub max_value: Option<CalendarDateTime>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the field has been interacted with.
    pub is_touched: bool,

    /// Whether the field is required.
    pub required: bool,

    /// Whether the field is invalid.
    pub invalid: bool,

    /// Form field name.
    pub name: Option<String>,

    /// Right-to-left layout.
    pub is_rtl: bool,

    /// The adapter-injected current date forwarded to the calendar.
    pub today: CalendarDate,

    /// Number of months shown in the calendar popover.
    pub visible_months: usize,

    /// Component IDs.
    pub ids: ComponentIds,
}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("value", &self.value)
            .field("date_value", &self.date_value)
            .field("time_value", &self.time_value)
            .field("focused_segment", &self.focused_segment)
            .field("open", &self.open)
            .field("date_segments", &self.date_segments)
            .field("time_segments", &self.time_segments)
            .field("type_buffer", &self.type_buffer)
            .field("locale", &self.locale)
            .field("messages", &self.messages)
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("granularity", &self.granularity)
            .field("hour_cycle", &self.hour_cycle)
            .field("calendar", &self.calendar)
            .field("min_value", &self.min_value)
            .field("max_value", &self.max_value)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("is_touched", &self.is_touched)
            .field("required", &self.required)
            .field("invalid", &self.invalid)
            .field("name", &self.name)
            .field("is_rtl", &self.is_rtl)
            .field("today", &self.today)
            .field("visible_months", &self.visible_months)
            .field("ids", &self.ids)
            .finish()
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        // Compare every field except the `intl_backend` trait object (which is
        // not `PartialEq`). Fields are grouped into reference tuples so the
        // per-field short-circuit comparisons live in `core`'s tuple `PartialEq`
        // rather than as a 25-arm `&&` chain in this function — same semantics,
        // far fewer locally-instrumented branches.
        let lhs = (
            &self.value,
            &self.date_value,
            &self.time_value,
            &self.focused_segment,
            self.open,
            &self.date_segments,
            &self.time_segments,
            &self.type_buffer,
            &self.locale,
            &self.messages,
            self.granularity,
            self.hour_cycle,
        );
        let rhs = (
            &other.value,
            &other.date_value,
            &other.time_value,
            &other.focused_segment,
            other.open,
            &other.date_segments,
            &other.time_segments,
            &other.type_buffer,
            &other.locale,
            &other.messages,
            other.granularity,
            other.hour_cycle,
        );

        let lhs_rest = (
            self.calendar,
            &self.min_value,
            &self.max_value,
            self.disabled,
            self.readonly,
            self.is_touched,
            self.required,
            self.invalid,
            &self.name,
            self.is_rtl,
            &self.today,
            self.visible_months,
        );
        let rhs_rest = (
            other.calendar,
            &other.min_value,
            &other.max_value,
            other.disabled,
            other.readonly,
            other.is_touched,
            other.required,
            other.invalid,
            &other.name,
            other.is_rtl,
            &other.today,
            other.visible_months,
        );

        lhs == rhs && lhs_rest == rhs_rest && self.ids == other.ids
    }
}

impl Context {
    /// All segments in display order: date segments, then time segments.
    pub fn all_segments(&self) -> impl Iterator<Item = &DateSegment> {
        self.date_segments.iter().chain(self.time_segments.iter())
    }

    /// Returns the segment with the given kind, if present in either group.
    #[must_use]
    pub fn segment(&self, kind: DateSegmentKind) -> Option<&DateSegment> {
        self.all_segments().find(|segment| segment.kind == kind)
    }

    /// Returns the current numeric value for a segment.
    #[must_use]
    pub fn segment_value(&self, kind: DateSegmentKind) -> Option<i32> {
        self.segment(kind)?.value
    }

    /// Finds the first editable segment across date and time.
    #[must_use]
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.all_segments()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Finds the next editable segment after `kind`, crossing the date/time boundary.
    #[must_use]
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all = self.all_segments().collect::<Vec<_>>();

        let index = all.iter().position(|segment| segment.kind == kind)?;

        all[index + 1..]
            .iter()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Finds the previous editable segment before `kind`, crossing the boundary.
    #[must_use]
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all = self.all_segments().collect::<Vec<_>>();

        let index = all.iter().position(|segment| segment.kind == kind)?;

        all[..index]
            .iter()
            .rev()
            .find(|segment| segment.is_editable)
            .map(|segment| segment.kind)
    }

    /// Whether a segment belongs to the date group.
    #[must_use]
    pub fn is_date_segment(&self, kind: DateSegmentKind) -> bool {
        self.date_segments
            .iter()
            .any(|segment| segment.kind == kind)
    }

    /// Whether a segment belongs to the time group.
    #[must_use]
    pub fn is_time_segment(&self, kind: DateSegmentKind) -> bool {
        self.time_segments
            .iter()
            .any(|segment| segment.kind == kind)
    }

    /// Sets and formats a segment value within that segment's valid range.
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        // Resolve the range and localized text up front (immutable borrows of
        // `intl_backend`/`locale`) before taking the mutable segment borrow.
        let Some((min, max)) = self
            .segment(kind)
            .filter(|segment| segment.is_editable)
            .map(|segment| (segment.min, segment.max))
        else {
            return;
        };
        let value = raw.clamp(min, max);
        let text = format_segment_text(self.intl_backend.as_ref(), &self.locale, kind, value);

        if let Some(segment) = self
            .date_segments
            .iter_mut()
            .chain(self.time_segments.iter_mut())
            .find(|segment| segment.kind == kind && segment.is_editable)
        {
            segment.value = Some(value);
            segment.text = text;
        }

        // Month/day ranges are calendar- and year-dependent (leap months in
        // e.g. Hebrew years, varying month lengths), so refresh them whenever
        // the year or month changes, clamping now-invalid values down so the
        // visible fields can never diverge from a committable date.
        if matches!(kind, DateSegmentKind::Year | DateSegmentKind::Month) {
            self.refresh_date_ranges();
        }
    }

    /// Refreshes the month and day segment maxima from the current
    /// year/month/era/calendar (via the intl backend), clamping now-out-of-range
    /// values down. Month is refreshed first so the day refresh sees a valid
    /// month.
    fn refresh_date_ranges(&mut self) {
        let era = self.current_era();
        let era_code = era.as_ref().map(|era| era.code.as_str());
        let year = self.segment_value(DateSegmentKind::Year);

        // Month range (calendars with 13-month leap years expose month 13).
        let month_max = year.map_or(12, |year| {
            i32::from(
                self.intl_backend
                    .max_months_in_year(&self.calendar, year, era_code),
            )
        });
        self.set_segment_max(DateSegmentKind::Month, month_max);

        // Day range depends on year + (clamped) month.
        let month = self
            .segment_value(DateSegmentKind::Month)
            .and_then(|month| u8::try_from(month).ok());
        let day_max = match (year, month) {
            (Some(year), Some(month)) => {
                i32::from(
                    self.intl_backend
                        .days_in_month(&self.calendar, year, month, era_code),
                )
            }
            _ => 31,
        };
        self.set_segment_max(DateSegmentKind::Day, day_max);
    }

    /// Sets a date segment's `max`, clamping a now-out-of-range value down and
    /// reformatting its localized text.
    fn set_segment_max(&mut self, kind: DateSegmentKind, max: i32) {
        // Compute the clamp text first (immutable borrows) before mutating.
        let clamp_text = self
            .segment(kind)
            .and_then(|segment| segment.value)
            .filter(|&value| value > max)
            .map(|_| format_segment_text(self.intl_backend.as_ref(), &self.locale, kind, max));

        if let Some(segment) = self
            .date_segments
            .iter_mut()
            .find(|segment| segment.kind == kind)
        {
            segment.max = max;
            if let Some(text) = clamp_text {
                segment.value = Some(max);
                segment.text = text;
            }
        }
    }

    /// Clears a segment value.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) {
        let segment = self
            .date_segments
            .iter_mut()
            .chain(self.time_segments.iter_mut())
            .find(|segment| segment.kind == kind);

        if let Some(segment) = segment {
            segment.value = None;
            segment.text.clear();
        }
    }

    /// Increment a segment value, wrapping within its range.
    pub fn increment_segment(&mut self, kind: DateSegmentKind) {
        self.step_segment_value(kind, 1);
    }

    /// Decrement a segment value, wrapping within its range.
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) {
        self.step_segment_value(kind, -1);
    }

    fn step_segment_value(&mut self, kind: DateSegmentKind, delta: i32) {
        let Some(segment) = self
            .all_segments()
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

    /// Returns `true` when all editable segments across date and time have values.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.all_segments()
            .filter(|segment| segment.is_editable)
            .all(|segment| segment.value.is_some())
    }

    /// The era to assemble dates under: the current value's era (so editing a
    /// Japanese Reiwa 6 value keeps Reiwa rather than reinterpreting `6` as a
    /// raw year), else the calendar's default era. `DateTimePicker` has no Era
    /// segment, so the era is carried from context rather than user-editable.
    #[must_use]
    fn current_era(&self) -> Option<Era> {
        self.date_value
            .as_ref()
            .and_then(|date| date.era().cloned())
            .or_else(|| {
                self.value
                    .get()
                    .as_ref()
                    .and_then(|dt| dt.date().era().cloned())
            })
            .or_else(|| self.intl_backend.default_era(&self.calendar))
    }

    /// Assembles a [`CalendarDate`] from current date segment values, using the
    /// configured [`calendar`](Self::calendar) system and carrying the current
    /// era so non-Gregorian segment values commit dates in that calendar
    /// (matching `date_field`).
    #[must_use]
    pub fn assemble_date(&self) -> Option<CalendarDate> {
        let year = self.segment_value(DateSegmentKind::Year)?;
        let month = u8::try_from(self.segment_value(DateSegmentKind::Month)?).ok()?;
        let day = u8::try_from(self.segment_value(DateSegmentKind::Day)?).ok()?;

        CalendarDate::new(
            self.calendar,
            &CalendarDateFields {
                era: self.current_era(),
                year: Some(year),
                month: Some(month),
                day: Some(day),
                ..CalendarDateFields::default()
            },
        )
        .ok()
    }

    /// Assembles a [`Time`] from current time segment values.
    #[must_use]
    pub fn assemble_time(&self) -> Option<Time> {
        let raw_hour = u8::try_from(self.segment_value(DateSegmentKind::Hour)?).ok()?;

        let minute = if self.granularity >= TimeGranularity::Minute {
            u8::try_from(self.segment_value(DateSegmentKind::Minute)?).ok()?
        } else {
            0
        };

        let second = if self.granularity >= TimeGranularity::Second {
            u8::try_from(self.segment_value(DateSegmentKind::Second)?).ok()?
        } else {
            0
        };

        let day_period = self.segment_value(DateSegmentKind::DayPeriod);
        let hour = display_hour_to_24(raw_hour, day_period, self.hour_cycle)?;

        Time::new(hour, minute, second, 0).ok()
    }

    /// Assembles a [`CalendarDateTime`] from current date and time segment values.
    #[must_use]
    pub fn assemble_datetime(&self) -> Option<CalendarDateTime> {
        Some(CalendarDateTime::new(
            self.assemble_date()?,
            self.assemble_time()?,
        ))
    }

    /// Recomputes the date segments from the current `date_value` and locale,
    /// then refreshes the day range for the rebuilt year/month.
    fn rebuild_date_segments(&mut self) {
        self.date_segments = build_date_segments(
            self.intl_backend.as_ref(),
            &self.locale,
            self.date_value.as_ref(),
        );
        self.refresh_date_ranges();
    }

    /// Recomputes the time segments from the current `time_value` and hour cycle.
    fn rebuild_time_segments(&mut self) {
        self.time_segments = build_time_segments(
            self.intl_backend.as_ref(),
            &self.locale,
            self.hour_cycle,
            self.granularity,
            self.time_value.as_ref(),
        );
    }
}

/// Message signature for the selected-date-time announcement: formats the
/// human-readable combined value into a localized string.
pub type SelectedDateTimeLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Localized messages for the `DateTimePicker` component.
///
/// The five composite labels (trigger, clear, the two segment groups, and the
/// selected-value announcement) plus the per-segment spinbutton labels needed by
/// the date and time segments. The segment labels live here, rather than being
/// borrowed from `date_field`, so a `DateTimePicker` carries its own complete,
/// independently localizable label set.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Trigger button label (default: "Open date and time picker").
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Clear button label (default: "Clear date and time").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Date segment group label (default: "Date").
    pub date_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Time segment group label (default: "Time").
    pub time_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announces the selected date-time (default: "Selected: {value}").
    pub selected_datetime_label: MessageFn<SelectedDateTimeLabelFn>,

    /// Label for the year segment (default: "Year").
    pub year_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the month segment (default: "Month").
    pub month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the day segment (default: "Day").
    pub day_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the hour segment (default: "Hour").
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the minute segment (default: "Minute").
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the second segment (default: "Second").
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the day-period segment (default: "AM/PM").
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible name for the calendar popover dialog (default: "Choose date and time").
    pub content_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open date and time picker"),
            clear_label: MessageFn::static_str("Clear date and time"),
            date_group_label: MessageFn::static_str("Date"),
            time_group_label: MessageFn::static_str("Time"),
            selected_datetime_label: MessageFn::new(|value: &str, _locale: &Locale| {
                format!("Selected: {value}")
            }),
            year_label: MessageFn::static_str("Year"),
            month_label: MessageFn::static_str("Month"),
            day_label: MessageFn::static_str("Day"),
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
            content_label: MessageFn::static_str("Choose date and time"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for every named effect intent the `date_time_picker` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Move focus into the embedded calendar grid.
    FocusCalendar,

    /// Restore focus to the trigger button.
    RestoreFocusToTrigger,

    /// Move focus to the first editable time segment (after calendar selection).
    FocusFirstTimeSegment,

    /// Adapter starts the type-buffer commit timer.
    TypeBufferCommit,
}

/// Machine for the `DateTimePicker` component.
#[derive(Debug)]
pub struct Machine;

impl Machine {
    /// Plan for opening the popover: focuses the calendar. `None` if already
    /// open or read-only.
    fn open_plan(state: &State, ctx: &Context) -> Option<TransitionPlan<Self>> {
        if *state == State::Open || ctx.readonly {
            return None;
        }

        Some(
            TransitionPlan::to(State::Open)
                .apply(|ctx: &mut Context| {
                    ctx.open = true;
                })
                .with_effect(PendingEffect::named(Effect::FocusCalendar)),
        )
    }

    /// Plan for closing the popover: restores focus to the trigger. `None` if
    /// not currently open.
    fn close_plan(state: &State) -> Option<TransitionPlan<Self>> {
        if *state != State::Open {
            return None;
        }

        Some(
            TransitionPlan::to(State::Focused)
                .apply(|ctx: &mut Context| {
                    ctx.open = false;
                })
                .with_effect(PendingEffect::named(Effect::RestoreFocusToTrigger)),
        )
    }

    /// If all segments are complete, assemble, clamp, and commit the value.
    fn maybe_publish(ctx: &mut Context) {
        if !ctx.is_complete() {
            return;
        }

        let Some(datetime) = ctx.assemble_datetime() else {
            return;
        };

        let clamped = clamp_datetime(datetime, ctx.min_value.as_ref(), ctx.max_value.as_ref());

        // Clamping may substitute a (Gregorian) min/max bound from props, so
        // reproject into the active calendar before caching the display date —
        // otherwise a later edit reassembles Gregorian fields under
        // `ctx.calendar` and commits a different ISO date.
        ctx.date_value = Some(project_date(clamped.date(), ctx.calendar));
        ctx.time_value = Some(*clamped.time());

        ctx.rebuild_date_segments();
        ctx.rebuild_time_segments();

        ctx.value.set(Some(clamped));
    }
}

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
        // Clamp the initial value into `[min, max]` so the hidden input and
        // segments never present an out-of-range datetime at mount (later
        // `ValueCommit`/`SyncProps` paths clamp too).
        let clamp = |value: Option<CalendarDateTime>| {
            value.map(|dt| clamp_datetime(dt, props.min_value.as_ref(), props.max_value.as_ref()))
        };
        let value = if let Some(controlled) = &props.value {
            Bindable::controlled(clamp(controlled.clone()))
        } else {
            Bindable::uncontrolled(clamp(props.default_value.clone()))
        };

        let locale = env.locale.clone();

        // Display the date in the configured calendar system.
        let date_value = value.get().as_ref().map(|dt| {
            dt.date()
                .to_calendar(props.calendar)
                .unwrap_or_else(|_| dt.date().clone())
        });
        let time_value = value.get().as_ref().map(|dt| *dt.time());

        let hour_cycle = props
            .hour_cycle
            .unwrap_or_else(|| locale.hour_cycle(env.intl_backend.as_ref()));

        let date_segments =
            build_date_segments(env.intl_backend.as_ref(), &locale, date_value.as_ref());
        let time_segments = build_time_segments(
            env.intl_backend.as_ref(),
            &locale,
            hour_cycle,
            props.granularity,
            time_value.as_ref(),
        );

        let ctx = Context {
            value,
            date_value,
            time_value,
            focused_segment: None,
            open: false,
            date_segments,
            time_segments,
            type_buffer: String::new(),
            locale,
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            granularity: props.granularity,
            hour_cycle,
            calendar: props.calendar,
            min_value: props.min_value.clone(),
            max_value: props.max_value.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            is_touched: false,
            required: props.required,
            invalid: props.invalid,
            name: props.name.clone(),
            is_rtl: props.is_rtl,
            today: props.today.clone(),
            visible_months: props.visible_months,
            ids: ComponentIds::from_id(&props.id),
        };

        let mut ctx = ctx;
        ctx.refresh_date_ranges();

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
        // `SyncProps` flows through even when disabled so parent-driven changes
        // that take the picker out of the disabled state can land.
        if let Event::SyncProps(new_props) = event {
            let new_props = new_props.as_ref().clone();

            let mut probe = ctx.clone();

            sync_props(&mut probe, &new_props);

            let next_state = reconcile_state_after_sync(state, &probe);

            let clear_focus = next_state != *state && next_state == State::Idle;
            let open = next_state == State::Open;

            return Some(
                TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                    sync_props(ctx, &new_props);

                    // Keep the public `open` flag consistent with the reconciled
                    // state — otherwise a sync that closes the picker (e.g. it
                    // became disabled while open) leaves `open == true` while the
                    // state says closed.
                    ctx.open = open;

                    if clear_focus {
                        ctx.focused_segment = None;
                        ctx.type_buffer.clear();
                    }
                }),
            );
        }

        if ctx.disabled {
            return None;
        }

        match event {
            Event::Open => Self::open_plan(state, ctx),

            Event::Close => Self::close_plan(state),

            Event::Toggle => match state {
                State::Open => Self::close_plan(state),
                _ => Self::open_plan(state, ctx),
            },

            Event::CalendarSelectDate(date) => {
                if ctx.readonly {
                    return None;
                }

                // Defense-in-depth: reject a date the picker disallows. The
                // embedded calendar should never offer an out-of-range date, but
                // a stale/scripted `CalendarSelectDate` must not store and display
                // an impossible date before the time segments complete and
                // `maybe_publish` could clamp. Compare the date against the
                // date portion of the min/max bounds.
                let in_range = ctx
                    .min_value
                    .as_ref()
                    .is_none_or(|min| date.compare(min.date()) != Ordering::Less)
                    && ctx
                        .max_value
                        .as_ref()
                        .is_none_or(|max| date.compare(max.date()) != Ordering::Greater);
                if !in_range {
                    return None;
                }

                let date = date.clone();
                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            // Project a (possibly foreign-calendar) selection into
                            // the active calendar before caching, so the next
                            // publish reassembles the same ISO date.
                            ctx.date_value = Some(project_date(&date, ctx.calendar));
                            ctx.open = false;
                            ctx.is_touched = true;

                            ctx.rebuild_date_segments();

                            Machine::maybe_publish(ctx);
                        })
                        .with_effect(PendingEffect::named(Effect::FocusFirstTimeSegment)),
                )
            }

            Event::FocusSegment(kind) => {
                let kind = *kind;

                if !ctx.segment(kind).is_some_and(|segment| segment.is_editable) {
                    return None;
                }

                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            commit_type_buffer(ctx);

                            ctx.focused_segment = Some(kind);
                            ctx.type_buffer.clear();
                        })
                        .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::FocusNextSegment => {
                if let Some(current) = ctx.focused_segment {
                    let next = ctx.next_editable_after(current);
                    let mut plan = TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            commit_type_buffer(ctx);
                            ctx.type_buffer.clear();
                            // `None` past the last segment clears focus rather
                            // than leaving a stale `focused_segment`/`data-ars-focused`.
                            ctx.focused_segment = next;
                        })
                        .cancel_effect(Effect::TypeBufferCommit);

                    // Advancing past the last segment moves focus to the trigger
                    // (spec §3.2), driven by the adapter via this effect.
                    if next.is_none() {
                        plan =
                            plan.with_effect(PendingEffect::named(Effect::RestoreFocusToTrigger));
                    }

                    Some(plan)
                } else {
                    let first = ctx.first_editable()?;
                    Some(
                        TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                            ctx.focused_segment = Some(first);
                        }),
                    )
                }
            }

            Event::FocusPrevSegment => {
                let current = ctx.focused_segment?;
                let previous = ctx.prev_editable_before(current)?;
                Some(
                    TransitionPlan::to(State::Focused)
                        .apply(move |ctx: &mut Context| {
                            commit_type_buffer(ctx);

                            ctx.type_buffer.clear();
                            ctx.focused_segment = Some(previous);
                        })
                        .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::IncrementSegment { segment } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *segment;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();
                        ctx.increment_segment(kind);
                        ctx.is_touched = true;

                        sync_sub_value(ctx, kind);

                        Machine::maybe_publish(ctx);
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::DecrementSegment { segment } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *segment;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();

                        ctx.decrement_segment(kind);

                        ctx.is_touched = true;

                        sync_sub_value(ctx, kind);

                        Machine::maybe_publish(ctx);
                    })
                    .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::TypeIntoSegment { segment, ch } => {
                if ctx.readonly {
                    return None;
                }

                type_into_segment(ctx, *segment, *ch)
            }

            Event::TypeBufferCommit { segment } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    commit_buffer_for_kind(ctx, kind);
                }))
            }

            Event::SegmentChange { segment, value } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *segment;
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.set_segment_value(kind, value);
                    ctx.is_touched = true;

                    sync_sub_value(ctx, kind);

                    Machine::maybe_publish(ctx);
                }))
            }

            Event::ValueCommit(datetime) => {
                if ctx.readonly {
                    return None;
                }

                let datetime = datetime.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_value(ctx, datetime.clone());

                    ctx.is_touched = true;
                    ctx.type_buffer.clear();
                }))
            }

            Event::ClearSegment { segment } => {
                if ctx.readonly {
                    return None;
                }

                let kind = *segment;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.type_buffer.clear();
                        ctx.clear_segment_value(kind);

                        sync_sub_value(ctx, kind);

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
                                .all_segments()
                                .filter(|segment| segment.is_editable)
                                .map(|segment| segment.kind)
                                .collect::<Vec<_>>();

                            for kind in editable {
                                ctx.clear_segment_value(kind);
                            }

                            ctx.date_value = None;
                            ctx.time_value = None;
                            ctx.value.set(None);
                            ctx.focused_segment = None;
                            ctx.type_buffer.clear();
                        })
                        .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::FocusIn => {
                if *state == State::Idle {
                    Some(TransitionPlan::to(State::Focused))
                } else {
                    None
                }
            }

            Event::FocusOut => {
                let was_open = ctx.open;
                Some(
                    TransitionPlan::to(State::Idle)
                        .apply(move |ctx: &mut Context| {
                            commit_type_buffer(ctx);

                            ctx.focused_segment = None;
                            ctx.type_buffer.clear();

                            if was_open {
                                ctx.open = false;
                            }
                        })
                        .cancel_effect(Effect::TypeBufferCommit),
                )
            }

            Event::KeyDown { key } => match key {
                KeyboardKey::Escape if *state == State::Open => Self::close_plan(state),
                KeyboardKey::ArrowDown if *state != State::Open => Self::open_plan(state, ctx),
                _ => None,
            },

            // Handled above the disabled guard.
            Event::SyncProps(_) => None,
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

/// Structural parts exposed by the `DateTimePicker` connect API.
#[derive(ComponentPart)]
#[scope = "date-time-picker"]
pub enum Part {
    /// The outermost container.
    Root,

    /// The associated label.
    Label,

    /// The control group wrapping the segments, trigger, and clear button.
    Control,

    /// The group containing all date segments.
    DateSegmentGroup,

    /// The group containing all time segments.
    TimeSegmentGroup,

    /// An editable date or time segment.
    Segment {
        /// The logical kind of the segment.
        kind: DateSegmentKind,
    },

    /// A non-editable literal separator within a segment group.
    Literal {
        /// The literal's rendered index.
        index: usize,
    },

    /// The visual separator between the date and time groups.
    Separator,

    /// The button that toggles the calendar popover.
    Trigger,

    /// The button that clears the value; hidden when empty.
    ClearTrigger,

    /// The floating positioner for the popover content.
    Positioner,

    /// The popover content (`role="dialog"`) containing the embedded calendar.
    Content,

    /// Optional help text.
    Description,

    /// Validation error text (`role="alert"`).
    ErrorMessage,

    /// The hidden input carrying the ISO 8601 datetime for form submission.
    HiddenInput,
}

/// API for the `DateTimePicker` component.
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
    /// Creates a `DateTimePicker` connect API from machine state.
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

    /// Returns attributes for the root element.
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

        if self.ctx.is_rtl {
            attrs.set(HtmlAttr::Dir, "rtl");
        }

        if let Some(announcement) = format_announcement(self.ctx) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_datetime_label)(&announcement, &self.ctx.locale),
            );
        }

        attrs
    }

    /// Returns attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        // No `for`: the `Control` target is a `role="group"`, not a labelable
        // form control, so `for` would be invalid and clicking the label would
        // not focus anything. The group is named via its own `aria-labelledby`.
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Returns attributes for the control wrapper element.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("control"))
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        let mut described_by = Vec::new();

        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.invalid && self.props.error_message.is_some() {
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

    /// Returns attributes for the date segment group.
    #[must_use]
    pub fn date_segment_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DateSegmentGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.date_group_label)(&self.ctx.locale),
            );

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        attrs
    }

    /// Returns attributes for the time segment group.
    #[must_use]
    pub fn time_segment_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TimeSegmentGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.time_group_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Returns attributes for a segment element (date or time).
    #[must_use]
    pub fn segment_attrs(&self, kind: &DateSegmentKind) -> AttrMap {
        let Some(segment) = self.ctx.segment(*kind) else {
            return AttrMap::new();
        };

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::Segment { kind: *kind }).data_attrs();

        // Roving tab stop (matching `date_field`/`time_field`): only the focused
        // segment — or the first editable segment when none is focused — is a
        // `Tab` stop, so native Tab order doesn't bypass the machine-managed
        // segment traversal (buffer commit, last-segment → trigger).
        let is_tab_stop = !self.ctx.disabled
            && (self.ctx.focused_segment == Some(segment.kind)
                || (self.ctx.focused_segment.is_none()
                    && self.ctx.first_editable() == Some(segment.kind)));

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Id,
                self.ctx.ids.item("segment", &segment.kind.data_name()),
            )
            .set(HtmlAttr::Role, "spinbutton")
            .set(HtmlAttr::TabIndex, if is_tab_stop { "0" } else { "-1" })
            .set(HtmlAttr::Data("ars-segment"), segment.kind.data_name())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                segment_aria_label(segment.kind, &self.ctx.messages, &self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), segment.min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), segment.max.to_string());

        if segment.kind.is_numeric() {
            attrs.set(HtmlAttr::InputMode, "numeric");
        }

        if let Some(value) = segment.value {
            // Announce a human-readable value (localized month name, AM/PM
            // label, else the number) via `aria_value_text`, mirroring
            // `date_field`, rather than the raw padded display text.
            let value_text = segment
                .aria_value_text(self.ctx.intl_backend.as_ref(), &self.ctx.locale)
                .unwrap_or_else(|| segment.text.clone());
            attrs
                .set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string())
                .set(HtmlAttr::Aria(AriaAttr::ValueText), value_text);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &segment.placeholder);
        }

        if self.ctx.focused_segment == Some(segment.kind) {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        attrs
    }

    /// Returns attributes for a literal separator within a segment group.
    #[must_use]
    pub fn literal_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            (Part::Literal { index }).data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-index"), index.to_string())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns attributes for the separator between the date and time groups.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Returns attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            )
            // Explicit `type="button"` so activating the trigger never submits a
            // surrounding form (the HTML default button type is `submit`).
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "0");

        // `readonly` blocks opening the popover (see `Machine::open_plan`), so the
        // trigger is disabled in both states — an operable-looking no-op button
        // would mislead pointer and AT users.
        if self.ctx.disabled || self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns attributes for the clear trigger button.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"))
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            )
            // Explicit `type="button"` so clearing never submits a surrounding form.
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.disabled || self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.value.get().is_none() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for the positioner wrapper.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("positioner"));

        if !self.is_open() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for the popover content.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "dialog")
            // Name the dialog so screen-reader users entering the popover hear
            // useful context (the trigger only labels the button).
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.content_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_name());

        attrs
    }

    /// Returns attributes for the description element.
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

    /// Returns attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

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
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }

        if let Some(datetime) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, format_iso8601(datetime));
        }

        // A disabled field is excluded from form submission, so disable the
        // hidden input too (otherwise its value still reaches form handlers).
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Handles click on the trigger button.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Handles keydown on the trigger button.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),
            KeyboardKey::ArrowDown => (self.send)(Event::Open),
            _ => {}
        }
    }

    /// Handles click on the clear trigger button.
    pub fn on_clear_trigger_click(&self) {
        (self.send)(Event::ClearAll);
    }

    /// Handles keydown on a segment (date or time).
    pub fn on_segment_keydown(
        &self,
        kind: DateSegmentKind,
        data: &KeyboardEventData,
        dir: Direction,
    ) {
        let is_rtl = dir.is_rtl();

        match data.key {
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementSegment { segment: kind }),
            KeyboardKey::ArrowDown if data.alt_key => (self.send)(Event::Open),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment { segment: kind }),
            KeyboardKey::ArrowRight if is_rtl => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowLeft if is_rtl => (self.send)(Event::FocusNextSegment),
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::Tab if data.shift_key => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowRight | KeyboardKey::Tab => (self.send)(Event::FocusNextSegment),
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::ClearSegment { segment: kind });
            }
            KeyboardKey::Escape => (self.send)(Event::KeyDown { key: data.key }),
            _ if data.character.is_some() => {
                if let Some(ch) = data.character {
                    (self.send)(Event::TypeIntoSegment { segment: kind, ch });
                }
            }
            _ => {}
        }
    }

    /// Handles focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
    }

    /// Handles focus entering the component.
    pub fn on_focusin(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Handles focus leaving the entire component.
    pub fn on_focusout(&self, focus_leaving_component: bool) {
        if focus_leaving_component {
            (self.send)(Event::FocusOut);
        }
    }

    /// Handles keydown within the popover content.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::Close);
        }
    }

    /// Builds [`calendar::Props`] from the current state.
    ///
    /// The adapter creates a calendar machine with these props inside the
    /// [`Part::Content`] element and wires its `SelectDate` event back to this
    /// machine as [`Event::CalendarSelectDate`].
    #[must_use]
    pub fn calendar_props(&self) -> calendar::Props {
        // Project every date the calendar sees into the configured calendar
        // system so the embedded popover navigates and emits dates in that
        // calendar (the adapter-injected `today` and the min/max bounds are
        // otherwise Gregorian).
        let into_calendar = |date: &CalendarDate| {
            date.to_calendar(self.ctx.calendar)
                .unwrap_or_else(|_| date.clone())
        };

        calendar::Props {
            id: format!("{}-calendar", self.ctx.ids.id()),
            value: Some(self.ctx.date_value.clone()),
            min: self
                .ctx
                .min_value
                .as_ref()
                .map(|dt| into_calendar(dt.date())),
            max: self
                .ctx
                .max_value
                .as_ref()
                .map(|dt| into_calendar(dt.date())),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_rtl: self.ctx.is_rtl,
            visible_months: self.ctx.visible_months,
            today: into_calendar(&self.ctx.today),
            ..calendar::Props::default()
        }
    }

    /// Returns the date segments in display order.
    #[must_use]
    pub fn date_segments(&self) -> &[DateSegment] {
        &self.ctx.date_segments
    }

    /// Returns the time segments in display order.
    #[must_use]
    pub fn time_segments(&self) -> &[DateSegment] {
        &self.ctx.time_segments
    }

    /// Whether the popover is currently open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        *self.state == State::Open
    }

    /// The currently selected date-time value.
    #[must_use]
    pub fn selected_value(&self) -> Option<&CalendarDateTime> {
        self.ctx.value.get().as_ref()
    }

    /// Opens the popover programmatically.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Closes the popover programmatically.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Toggles the popover programmatically.
    pub fn toggle(&self) {
        (self.send)(Event::Toggle);
    }

    const fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Focused => "focused",
            State::Open => "open",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::DateSegmentGroup => self.date_segment_group_attrs(),
            Part::TimeSegmentGroup => self.time_segment_group_attrs(),
            Part::Segment { kind } => self.segment_attrs(&kind),
            Part::Literal { index } => self.literal_attrs(index),
            Part::Separator => self.separator_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Free helpers
// ────────────────────────────────────────────────────────────────────

/// Re-derives `date_value`/`time_value` from the segment lists after an edit.
fn sync_sub_value(ctx: &mut Context, kind: DateSegmentKind) {
    if ctx.is_date_segment(kind) {
        ctx.date_value = ctx.assemble_date();
    } else if ctx.is_time_segment(kind) {
        ctx.time_value = ctx.assemble_time();
    }
}

/// Handles a printable character typed into a segment.
fn type_into_segment(
    ctx: &Context,
    kind: DateSegmentKind,
    ch: char,
) -> Option<TransitionPlan<Machine>> {
    if kind == DateSegmentKind::DayPeriod {
        // Route through the backend so localized AM/PM labels (whose leading
        // character is not ASCII `a`/`p`) are recognised, mirroring `time_field`;
        // fall back to ASCII `a`/`p` when the backend does not map the character.
        let is_pm = ctx
            .intl_backend
            .day_period_from_char(ch, &ctx.locale)
            .or_else(|| match ch.to_ascii_lowercase() {
                'a' => Some(false),
                'p' => Some(true),
                _ => None,
            })?;
        let value = i32::from(is_pm);

        let next = ctx.next_editable_after(kind);

        return Some(
            TransitionPlan::to(State::Focused)
                .apply(move |ctx: &mut Context| {
                    ctx.set_segment_value(DateSegmentKind::DayPeriod, value);
                    ctx.is_touched = true;
                    ctx.time_value = ctx.assemble_time();

                    Machine::maybe_publish(ctx);

                    ctx.type_buffer.clear();

                    if let Some(next) = next {
                        ctx.focused_segment = Some(next);
                    }
                })
                .cancel_effect(Effect::TypeBufferCommit),
        );
    }

    if !kind.is_numeric() || !ch.is_ascii_digit() {
        return None;
    }

    let mut new_buffer = ctx.type_buffer.clone();

    new_buffer.push(ch);

    let buffered = new_buffer.parse::<i32>().ok()?;

    let segment = ctx.segment(kind)?;

    let (seg_min, seg_max) = (segment.min, segment.max);

    let max_digits = digits_needed(u32::try_from(seg_max).ok()?);

    let should_advance = new_buffer.len() >= max_digits || buffered * 10 > seg_max;

    let next = should_advance
        .then(|| ctx.next_editable_after(kind))
        .flatten();

    let mut plan = TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
        ctx.type_buffer.push(ch);

        if let Ok(value) = ctx.type_buffer.parse::<i32>()
            && value >= seg_min
            && value <= seg_max
        {
            ctx.set_segment_value(kind, value);
            ctx.is_touched = true;

            sync_sub_value(ctx, kind);

            Machine::maybe_publish(ctx);
        }

        if should_advance {
            ctx.type_buffer.clear();

            if let Some(next) = next {
                ctx.focused_segment = Some(next);
            }
        }
    });

    plan = if should_advance {
        plan.cancel_effect(Effect::TypeBufferCommit)
    } else {
        plan.cancel_effect(Effect::TypeBufferCommit)
            .with_effect(PendingEffect::named(Effect::TypeBufferCommit))
    };

    Some(plan)
}

/// Commits the type-ahead buffer for the focused segment, if any.
fn commit_type_buffer(ctx: &mut Context) {
    if let Some(kind) = ctx.focused_segment {
        commit_buffer_for_kind(ctx, kind);
    }
}

/// Commits the type-ahead buffer for a specific segment kind.
fn commit_buffer_for_kind(ctx: &mut Context, kind: DateSegmentKind) {
    if let Ok(value) = ctx.type_buffer.parse::<i32>()
        && let Some(segment) = ctx.segment(kind)
        && value >= segment.min
        && value <= segment.max
    {
        ctx.set_segment_value(kind, value);
        ctx.is_touched = true;

        sync_sub_value(ctx, kind);

        Machine::maybe_publish(ctx);
    }

    ctx.type_buffer.clear();
}

/// Applies a programmatic value, clamping and rebuilding both segment groups.
fn apply_value(ctx: &mut Context, value: Option<CalendarDateTime>) {
    let value = value.map(|dt| clamp_datetime(dt, ctx.min_value.as_ref(), ctx.max_value.as_ref()));

    // Reproject the (clamped, possibly Gregorian) date into the active calendar
    // so the displayed segments and a subsequent edit agree on the ISO date.
    ctx.date_value = value
        .as_ref()
        .map(|dt| project_date(dt.date(), ctx.calendar));
    ctx.time_value = value.as_ref().map(|dt| *dt.time());

    if ctx.value.is_controlled() {
        ctx.value.set(value.clone());
        ctx.value.sync_controlled(Some(value));
    } else {
        ctx.value.set(value);
    }

    ctx.rebuild_date_segments();
    ctx.rebuild_time_segments();
}

/// Re-applies a new props snapshot onto `ctx` (controlled value + scalar fields).
fn sync_props(ctx: &mut Context, props: &Props) {
    let was_controlled = ctx.value.is_controlled();
    let previous_granularity = ctx.granularity;
    let previous_hour_cycle = ctx.hour_cycle;
    let previous_calendar = ctx.calendar;

    ctx.granularity = props.granularity;
    ctx.hour_cycle = props
        .hour_cycle
        .unwrap_or_else(|| ctx.locale.hour_cycle(ctx.intl_backend.as_ref()));
    ctx.calendar = props.calendar;
    ctx.min_value = props.min_value.clone();
    ctx.max_value = props.max_value.clone();
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.required = props.required;
    ctx.invalid = props.invalid;
    ctx.name = props.name.clone();
    ctx.is_rtl = props.is_rtl;
    ctx.today = props.today.clone();
    ctx.visible_months = props.visible_months;
    ctx.ids = ComponentIds::from_id(&props.id);

    // A calendar change must rebuild/re-derive the date segments — otherwise the
    // visible fields keep their old-calendar values while edits reassemble them
    // in the new calendar, committing the wrong ISO date.
    let structure_changed = previous_granularity != ctx.granularity
        || previous_hour_cycle != ctx.hour_cycle
        || previous_calendar != ctx.calendar;

    let calendar = ctx.calendar;
    // Projects the value's date into the active calendar for display.
    let date_in_calendar = |dt: &CalendarDateTime| {
        dt.date()
            .to_calendar(calendar)
            .unwrap_or_else(|_| dt.date().clone())
    };

    if let Some(value) = &props.value {
        // Controlled: adopt the parent's value and rebuild.
        let clamped = value
            .clone()
            .map(|dt| clamp_datetime(dt, ctx.min_value.as_ref(), ctx.max_value.as_ref()));

        ctx.date_value = clamped.as_ref().map(&date_in_calendar);
        ctx.time_value = clamped.as_ref().map(|dt| *dt.time());
        ctx.value.set(clamped.clone());
        ctx.value.sync_controlled(Some(clamped));

        ctx.rebuild_date_segments();
        ctx.rebuild_time_segments();
    } else if was_controlled {
        // Controlled → uncontrolled: the parent relinquished control, so clear
        // the staged controlled value rather than leaving the field filled with
        // (and submitting) the stale value (mirrors `time_field`).
        ctx.value.sync_controlled(None);
        ctx.value.set(None);
        ctx.date_value = None;
        ctx.time_value = None;
        ctx.rebuild_date_segments();
        ctx.rebuild_time_segments();
    } else {
        // Uncontrolled throughout: re-clamp the current value in place.
        ctx.value.sync_controlled(None);

        let clamped = ctx
            .value
            .get()
            .clone()
            .map(|dt| clamp_datetime(dt, ctx.min_value.as_ref(), ctx.max_value.as_ref()));

        let value_changed = clamped != *ctx.value.get();

        ctx.date_value = clamped.as_ref().map(&date_in_calendar);
        ctx.time_value = clamped.as_ref().map(|dt| *dt.time());
        ctx.value.set(clamped);

        if structure_changed || value_changed {
            ctx.rebuild_date_segments();
            ctx.rebuild_time_segments();
        }
    }
}

/// Derives the post-sync state: an editable focus that survives stays, else idle.
fn reconcile_state_after_sync(state: &State, ctx: &Context) -> State {
    match state {
        State::Open if ctx.disabled => State::Idle,

        State::Open => State::Open,

        State::Focused => {
            let still_focusable = !ctx.disabled
                && ctx
                    .focused_segment
                    .is_some_and(|kind| ctx.segment(kind).is_some_and(|s| s.is_editable));

            if still_focusable {
                State::Focused
            } else {
                State::Idle
            }
        }

        State::Idle => State::Idle,
    }
}

/// Builds the locale-ordered date segments for the given date value.
fn build_date_segments(
    backend: &dyn IntlBackend,
    locale: &Locale,
    date: Option<&CalendarDate>,
) -> Vec<DateSegment> {
    let mut year = DateSegment::new_numeric(DateSegmentKind::Year, 1, 9999, "yyyy");
    let mut month = DateSegment::new_numeric(DateSegmentKind::Month, 1, 12, "mm");
    let mut day = DateSegment::new_numeric(DateSegmentKind::Day, 1, 31, "dd");

    if let Some(date) = date {
        year.value = Some(date.year());
        year.text = format_segment_text(backend, locale, DateSegmentKind::Year, date.year());

        month.value = Some(i32::from(date.month()));
        month.text = format_segment_text(
            backend,
            locale,
            DateSegmentKind::Month,
            i32::from(date.month()),
        );

        day.value = Some(i32::from(date.day()));
        day.text =
            format_segment_text(backend, locale, DateSegmentKind::Day, i32::from(date.day()));
    }

    let ordered = match date_order(locale) {
        DateOrder::MonthDayYear => [month, day, year],
        DateOrder::DayMonthYear => [day, month, year],
        DateOrder::YearMonthDay => [year, month, day],
    };

    let separator = date_field_separator(locale);

    let mut segments = Vec::with_capacity(5);

    for (index, segment) in ordered.into_iter().enumerate() {
        if index > 0 {
            segments.push(DateSegment::new_literal(&separator));
        }

        segments.push(segment);
    }

    segments
}

/// Builds the time segments for the given hour cycle, granularity, and value.
fn build_time_segments(
    backend: &dyn IntlBackend,
    locale: &Locale,
    hour_cycle: HourCycle,
    granularity: TimeGranularity,
    time: Option<&Time>,
) -> Vec<DateSegment> {
    let mut segments = Vec::new();

    let (hour_min, hour_max) = display_hour_range(hour_cycle);

    let mut hour = DateSegment::new_numeric(DateSegmentKind::Hour, hour_min, hour_max, "hh");

    if let Some(time) = time {
        let display = display_hour(*time, hour_cycle);

        hour.value = Some(display);
        hour.text = format_segment_text(backend, locale, DateSegmentKind::Hour, display);
    }

    segments.push(hour);

    if granularity >= TimeGranularity::Minute {
        segments.push(DateSegment::new_literal(":"));

        let mut minute = DateSegment::new_numeric(DateSegmentKind::Minute, 0, 59, "mm");

        if let Some(time) = time {
            let value = i32::from(time.minute());

            minute.value = Some(value);
            minute.text = format_segment_text(backend, locale, DateSegmentKind::Minute, value);
        }

        segments.push(minute);
    }

    if granularity >= TimeGranularity::Second {
        segments.push(DateSegment::new_literal(":"));

        let mut second = DateSegment::new_numeric(DateSegmentKind::Second, 0, 59, "ss");

        if let Some(time) = time {
            let value = i32::from(time.second());

            second.value = Some(value);
            second.text = format_segment_text(backend, locale, DateSegmentKind::Second, value);
        }

        segments.push(second);
    }

    if has_day_period(hour_cycle) {
        segments.push(DateSegment::new_literal("\u{00A0}"));

        let mut period = DateSegment {
            kind: DateSegmentKind::DayPeriod,
            value: None,
            min: 0,
            max: 1,
            text: String::new(),
            placeholder: backend.day_period_label(false, locale),
            literal: None,
            is_editable: true,
        };

        if let Some(time) = time {
            let value = i32::from(is_pm(*time));

            period.value = Some(value);
            period.text = format_segment_text(backend, locale, DateSegmentKind::DayPeriod, value);
        }

        segments.push(period);
    }

    segments
}

/// Formats a segment value for display: year is 4-wide, day-period is AM/PM,
/// every other numeric segment is zero-padded to two digits.
fn format_segment_text(
    backend: &dyn IntlBackend,
    locale: &Locale,
    kind: DateSegmentKind,
    value: i32,
) -> String {
    // Route through the locale backend (like `time_field`/`date_field`) so
    // native digit systems and localized day-period labels are honoured.
    if kind == DateSegmentKind::DayPeriod {
        return backend.day_period_label(value == 1, locale);
    }

    // Year is zero-padded to 4 digits, every other numeric segment to 2.
    let min_digits = if kind == DateSegmentKind::Year { 4 } else { 2 };

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

/// Resolves the ARIA label for a segment from the component messages.
fn segment_aria_label(kind: DateSegmentKind, messages: &Messages, locale: &Locale) -> String {
    let resolve = |message: &MessageFn<dyn Fn(&Locale) -> String + Send + Sync>| message(locale);

    match kind {
        DateSegmentKind::Year => resolve(&messages.year_label),
        DateSegmentKind::Month => resolve(&messages.month_label),
        DateSegmentKind::Day => resolve(&messages.day_label),
        DateSegmentKind::Hour => resolve(&messages.hour_label),
        DateSegmentKind::Minute => resolve(&messages.minute_label),
        DateSegmentKind::Second => resolve(&messages.second_label),
        DateSegmentKind::DayPeriod => resolve(&messages.day_period_label),
        _ => String::new(),
    }
}

/// Compares two date-times: by date first, then by time.
fn cmp_datetime(a: &CalendarDateTime, b: &CalendarDateTime) -> Ordering {
    match a.date().compare(b.date()) {
        Ordering::Equal => a.time().cmp(b.time()),
        other => other,
    }
}

/// Projects a date into the given calendar system for display, falling back to
/// the original date if the conversion is unsupported.
fn project_date(date: &CalendarDate, calendar: CalendarSystem) -> CalendarDate {
    date.to_calendar(calendar).unwrap_or_else(|_| date.clone())
}

/// Clamps a date-time into the inclusive `[min, max]` range.
fn clamp_datetime(
    value: CalendarDateTime,
    min: Option<&CalendarDateTime>,
    max: Option<&CalendarDateTime>,
) -> CalendarDateTime {
    if let Some(min) = min
        && cmp_datetime(&value, min) == Ordering::Less
    {
        return min.clone();
    }

    if let Some(max) = max
        && cmp_datetime(&value, max) == Ordering::Greater
    {
        return max.clone();
    }

    value
}

/// Formats the combined value as `YYYY-MM-DDTHH:MM:SS` for the hidden input.
///
/// The date is serialized from the canonical ISO calendar slots
/// ([`CalendarDate::to_iso8601`]), not the display fields, so a non-Gregorian
/// selected value still submits the spec-promised ISO 8601 datetime.
fn format_iso8601(datetime: &CalendarDateTime) -> String {
    // Compose canonical ISO date + time; `Time::to_iso8601` preserves any
    // fractional seconds so the hidden input matches `selected_value()` exactly.
    format!(
        "{}T{}",
        datetime.date().to_iso8601(),
        datetime.time().to_iso8601(),
    )
}

/// Builds the human-readable selected-value announcement, or `None` when the
/// combined value is not yet complete.
fn format_announcement(ctx: &Context) -> Option<String> {
    let date = ctx.date_value.as_ref()?;
    let time = ctx.time_value.as_ref()?;

    let separator = date_field_separator(&ctx.locale);

    // Localize numeric fields and the day-period label through the intl backend
    // so the spoken announcement matches the (localized) visible segments,
    // rather than emitting ASCII digits / hard-coded AM/PM.
    let digits = |value: i32, width: u8| match u32::try_from(value) {
        Ok(value) => ctx.intl_backend.format_segment_digits(
            value,
            NonZeroU8::new(width).expect("segment width is non-zero"),
            &ctx.locale,
        ),
        Err(_) => value.to_string(),
    };

    let (year, month, day) = (
        digits(date.year(), 4),
        digits(i32::from(date.month()), 2),
        digits(i32::from(date.day()), 2),
    );

    let date_str = match date_order(&ctx.locale) {
        DateOrder::MonthDayYear => format!("{month}{separator}{day}{separator}{year}"),
        DateOrder::DayMonthYear => format!("{day}{separator}{month}{separator}{year}"),
        DateOrder::YearMonthDay => format!("{year}{separator}{month}{separator}{day}"),
    };

    let mut time_str = format!(
        "{}:{}",
        digits(display_hour(*time, ctx.hour_cycle), 2),
        digits(i32::from(time.minute()), 2),
    );

    if ctx.granularity >= TimeGranularity::Second {
        time_str.push(':');
        time_str.push_str(&digits(i32::from(time.second()), 2));
    }

    if has_day_period(ctx.hour_cycle) {
        time_str.push(' ');
        time_str.push_str(&ctx.intl_backend.day_period_label(is_pm(*time), &ctx.locale));
    }

    Some(format!("{date_str} {time_str}"))
}
