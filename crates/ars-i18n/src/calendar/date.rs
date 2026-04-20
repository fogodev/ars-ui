use alloc::{
    format,
    string::{String, ToString},
};
use core::cmp::Ordering;

use temporal_rs::{
    Calendar, Duration as TemporalDuration, PlainDate, PlainDateTime as TemporalPlainDateTime,
    PlainTime as TemporalPlainTime,
    fields::CalendarFields,
    options::Overflow,
    partial::{PartialDate, PartialTime},
};
#[cfg(feature = "std")]
use {
    super::TimeZoneId,
    std::time::{Duration, SystemTime, UNIX_EPOCH},
    temporal_rs::options::Disambiguation as TemporalDisambiguation,
    temporal_rs::partial::PartialZonedDateTime,
};

use super::{
    CalendarConversionError, CalendarError, CalendarSystem, DateError, Era, MonthCode,
    canonical_era, infer_public_era, month_code_from_temporal,
};
use crate::Weekday;

/// Partial calendar-date fields used by constructors and `set`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CalendarDateFields {
    /// Optional era code.
    pub era: Option<Era>,

    /// Display year or era year.
    pub year: Option<i32>,

    /// 1-based display month ordinal.
    pub month: Option<u8>,

    /// Calendar-specific month code.
    pub month_code: Option<MonthCode>,

    /// 1-based day of month.
    pub day: Option<u8>,
}

/// Partial wall-clock time fields used by constructors and `set`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeFields {
    /// Hour field.
    pub hour: Option<u8>,

    /// Minute field.
    pub minute: Option<u8>,

    /// Second field.
    pub second: Option<u8>,

    /// Millisecond field.
    pub millisecond: Option<u16>,

    /// Microsecond field.
    pub microsecond: Option<u16>,

    /// Nanosecond field.
    pub nanosecond: Option<u16>,
}

/// Date-only duration fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateDuration {
    /// Years component.
    pub years: i32,

    /// Months component.
    pub months: i32,

    /// Weeks component.
    pub weeks: i32,

    /// Days component.
    pub days: i32,
}

/// Time-only duration fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeDuration {
    /// Hours component.
    pub hours: i64,

    /// Minutes component.
    pub minutes: i64,

    /// Seconds component.
    pub seconds: i64,

    /// Milliseconds component.
    pub milliseconds: i64,

    /// Microseconds component.
    pub microseconds: i64,

    /// Nanoseconds component.
    pub nanoseconds: i64,
}

/// Combined date and time duration fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateTimeDuration {
    /// Date portion.
    pub date: DateDuration,

    /// Time portion.
    pub time: TimeDuration,
}

/// Date field identifiers accepted by `CalendarDate::cycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateField {
    /// Cycle the display year.
    Year,

    /// Cycle the display month.
    Month,

    /// Cycle the display day.
    Day,
}

/// Time field identifiers accepted by `Time::cycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeField {
    /// Cycle the hour field.
    Hour,

    /// Cycle the minute field.
    Minute,

    /// Cycle the second field.
    Second,

    /// Cycle the millisecond field.
    Millisecond,
}

/// Date-time field identifiers accepted by `CalendarDateTime::cycle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateTimeField {
    /// Cycle the date year field.
    Year,

    /// Cycle the date month field.
    Month,

    /// Cycle the date day field.
    Day,

    /// Cycle the time hour field.
    Hour,

    /// Cycle the time minute field.
    Minute,

    /// Cycle the time second field.
    Second,

    /// Cycle the time millisecond field.
    Millisecond,
}

/// Options controlling `CalendarDate::cycle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleOptions {
    /// Whether cycling wraps within the current unit range.
    pub wrap: bool,
}

/// Options controlling `Time::cycle` and `CalendarDateTime::cycle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleTimeOptions {
    /// Whether cycling wraps within the current unit range.
    pub wrap: bool,
}

/// Time-zone disambiguation strategies for local-to-zoned conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disambiguation {
    /// Prefer compatible platform behavior.
    #[default]
    Compatible,

    /// Choose the earlier instant in overlaps.
    Earlier,

    /// Choose the later instant in overlaps.
    Later,

    /// Reject ambiguous or skipped local times.
    Reject,
}

impl Disambiguation {
    #[cfg(feature = "std")]
    const fn to_temporal(self) -> TemporalDisambiguation {
        match self {
            Self::Compatible => TemporalDisambiguation::Compatible,
            Self::Earlier => TemporalDisambiguation::Earlier,
            Self::Later => TemporalDisambiguation::Later,
            Self::Reject => TemporalDisambiguation::Reject,
        }
    }
}

/// A validated calendar date with canonical ISO storage and cached display fields.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDate {
    pub(crate) calendar: CalendarSystem,
    pub(crate) era: Option<Era>,
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) month_code: Option<MonthCode>,
    pub(crate) day: u8,
    pub(crate) iso_year: i32,
    pub(crate) iso_month: u8,
    pub(crate) iso_day: u8,
}

impl CalendarDate {
    /// Creates a validated date in the requested calendar system.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied fields do not form a valid date in the
    /// requested calendar or if the calendar engine rejects the conversion.
    pub fn new(
        calendar: CalendarSystem,
        fields: &CalendarDateFields,
    ) -> Result<Self, CalendarError> {
        build_calendar_date(calendar, fields, Overflow::Reject)
    }

    /// Creates an ISO 8601 calendar date.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied year, month, and day are not a valid
    /// ISO 8601 date.
    pub fn new_iso8601(year: i32, month: u8, day: u8) -> Result<Self, CalendarError> {
        build_from_iso_parts(CalendarSystem::Iso8601, year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }

    /// Creates a Gregorian calendar date.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied year, month, and day are not a valid
    /// Gregorian date.
    pub fn new_gregorian(year: i32, month: u8, day: u8) -> Result<Self, CalendarError> {
        build_from_iso_parts(CalendarSystem::Gregorian, year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }

    /// Returns the calendar system.
    #[must_use]
    pub const fn calendar(&self) -> CalendarSystem {
        self.calendar
    }

    /// Returns the stored era.
    #[must_use]
    pub const fn era(&self) -> Option<&Era> {
        self.era.as_ref()
    }

    /// Returns the display year in the stored calendar.
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// Returns the 1-based display month ordinal.
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// Returns the calendar-specific month code when available.
    #[must_use]
    pub const fn month_code(&self) -> Option<MonthCode> {
        self.month_code
    }

    /// Returns the 1-based day of month.
    #[must_use]
    pub const fn day(&self) -> u8 {
        self.day
    }

    /// Adds a date duration to this value.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented in this
    /// calendar.
    pub fn add(&self, duration: DateDuration) -> Result<Self, CalendarError> {
        let current = temporal_date_for(self)?;

        let next = current
            .add(&duration.to_temporal()?, Some(Overflow::Constrain))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        build_from_iso_parts(
            self.calendar,
            next.with_calendar(Calendar::ISO).year(),
            next.with_calendar(Calendar::ISO).month(),
            next.with_calendar(Calendar::ISO).day(),
        )
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }

    /// Subtracts a date duration from this value.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented in this
    /// calendar.
    pub fn subtract(&self, duration: DateDuration) -> Result<Self, CalendarError> {
        self.add(DateDuration {
            years: -duration.years,
            months: -duration.months,
            weeks: -duration.weeks,
            days: -duration.days,
        })
    }

    /// Returns a copy with selected fields replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if the replacement fields do not describe a valid date
    /// in this calendar.
    pub fn set(&self, fields: &CalendarDateFields) -> Result<Self, CalendarError> {
        let current = temporal_date_for(self)?;

        let replacement = temporal_calendar_fields(fields)?;

        let next = current
            .with(replacement, Some(Overflow::Reject))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        build_from_iso_parts(
            self.calendar,
            next.with_calendar(Calendar::ISO).year(),
            next.with_calendar(Calendar::ISO).month(),
            next.with_calendar(Calendar::ISO).day(),
        )
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }

    /// Cycles a date field by the requested amount.
    ///
    /// # Errors
    ///
    /// Returns an error if applying the requested cycle would produce an
    /// unrepresentable date.
    pub fn cycle(
        &self,
        field: DateField,
        amount: i32,
        options: CycleOptions,
    ) -> Result<Self, CalendarError> {
        if amount == 0 {
            return Ok(self.clone());
        }

        match field {
            DateField::Year => self.add(DateDuration {
                years: amount,
                ..DateDuration::default()
            }),

            DateField::Month => {
                if options.wrap {
                    let months_in_year = i32::from(self.calendar.months_in_year(self));

                    let next_month =
                        (i32::from(self.month) - 1 + amount).rem_euclid(months_in_year) + 1;

                    self.set(&CalendarDateFields {
                        month: Some(u8::try_from(next_month).map_err(|_| {
                            CalendarError::Arithmetic(String::from("month cycle overflow"))
                        })?),
                        ..CalendarDateFields::default()
                    })
                } else {
                    self.add(DateDuration {
                        months: amount,
                        ..DateDuration::default()
                    })
                }
            }

            DateField::Day => {
                if options.wrap {
                    let days_in_month = i32::from(self.calendar.days_in_month(self));

                    let next_day = (i32::from(self.day) - 1 + amount).rem_euclid(days_in_month) + 1;

                    self.set(&CalendarDateFields {
                        day: Some(u8::try_from(next_day).map_err(|_| {
                            CalendarError::Arithmetic(String::from("day cycle overflow"))
                        })?),
                        ..CalendarDateFields::default()
                    })
                } else {
                    self.add(DateDuration {
                        days: amount,
                        ..DateDuration::default()
                    })
                }
            }
        }
    }

    /// Converts this date into a different calendar system.
    ///
    /// # Errors
    ///
    /// Returns an error if the target calendar is unsupported or the
    /// conversion fails in the underlying calendar engine.
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError> {
        build_from_iso_parts(target, self.iso_year, self.iso_month, self.iso_day)
    }

    /// Compares two dates chronologically using their canonical ISO slots.
    #[must_use]
    pub fn compare(&self, other: &Self) -> Ordering {
        (self.iso_year, self.iso_month, self.iso_day).cmp(&(
            other.iso_year,
            other.iso_month,
            other.iso_day,
        ))
    }

    /// Returns the canonical ISO 8601 representation of this date.
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        format!(
            "{}-{:02}-{:02}",
            format_iso_year(self.iso_year),
            self.iso_month,
            self.iso_day
        )
    }

    /// Returns the ISO weekday for this date.
    #[must_use]
    pub fn weekday(&self) -> Weekday {
        match temporal_iso_date(self.iso_year, self.iso_month, self.iso_day)
            .expect("stored ISO date is always valid")
            .day_of_week()
        {
            1 => Weekday::Monday,
            2 => Weekday::Tuesday,
            3 => Weekday::Wednesday,
            4 => Weekday::Thursday,
            5 => Weekday::Friday,
            6 => Weekday::Saturday,
            _ => Weekday::Sunday,
        }
    }

    /// Adds whole ISO days.
    ///
    /// # Errors
    ///
    /// Returns an error if the resulting date cannot be represented.
    pub fn add_days(&self, days: i32) -> Result<Self, CalendarError> {
        self.add(DateDuration {
            days,
            ..DateDuration::default()
        })
    }

    /// Returns the day distance to another date.
    ///
    /// # Errors
    ///
    /// Returns an error if either date cannot be converted into the internal
    /// calendar engine.
    pub fn days_until(&self, other: &Self) -> Result<i32, CalendarError> {
        let start = temporal_iso_date(self.iso_year, self.iso_month, self.iso_day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        let end = temporal_iso_date(other.iso_year, other.iso_month, other.iso_day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        let diff = start
            .until(&end, Default::default())
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        i32::try_from(diff.days()).map_err(|_| {
            CalendarError::Arithmetic(String::from("date difference exceeds i32 range"))
        })
    }

    /// Returns `true` when this date precedes another date.
    ///
    /// # Errors
    ///
    /// Returns an error if either date cannot be converted into the internal
    /// calendar engine.
    pub fn is_before(&self, other: &Self) -> Result<bool, CalendarError> {
        Ok(self.compare(other) == Ordering::Less)
    }

    /// Returns today's date in the requested calendar.
    ///
    /// # Errors
    ///
    /// Returns an error if the local clock or requested calendar projection
    /// cannot be resolved.
    #[cfg(feature = "std")]
    pub fn today(calendar: CalendarSystem) -> Result<Self, CalendarError> {
        crate::calendar::parse::today_for_calendar(calendar)
    }

    /// Converts this date into `SystemTime` at local midnight in the supplied time zone.
    ///
    /// # Errors
    ///
    /// Returns an error if the time zone is invalid or the local midnight
    /// instant cannot be resolved.
    #[cfg(feature = "std")]
    pub fn to_system_time(&self, time_zone: &TimeZoneId) -> Result<SystemTime, CalendarError> {
        to_zoned(self, time_zone)?.to_system_time()
    }

    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self) -> u8 {
        self.calendar.days_in_month(self)
    }

    /// Returns the maximum year in the current era when it is bounded.
    #[must_use]
    pub fn years_in_era(&self) -> Option<i32> {
        self.calendar.years_in_era(self)
    }

    /// Returns the minimum valid month for the current year in the current era.
    #[must_use]
    pub fn minimum_month_in_year(&self) -> u8 {
        self.calendar.minimum_month_in_year(self)
    }

    /// Returns the minimum valid day for the current month in the current era.
    #[must_use]
    pub fn minimum_day_in_month(&self) -> u8 {
        self.calendar.minimum_day_in_month(self)
    }

    /// Compares two dates only when their calendars and eras match.
    #[must_use]
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering> {
        if self.calendar != other.calendar {
            return None;
        }

        if self.era.as_ref().map(|value| value.code.as_str())
            != other.era.as_ref().map(|value| value.code.as_str())
        {
            return None;
        }

        Some((self.year, self.month, self.day).cmp(&(other.year, other.month, other.day)))
    }

    /// Returns `true` when the date is between two bounds in the same calendar/era.
    #[must_use]
    pub fn is_between(&self, start: &CalendarDate, end: &CalendarDate) -> bool {
        matches!(
            self.compare_within_calendar(start),
            Some(Ordering::Equal | Ordering::Greater)
        ) && matches!(
            self.compare_within_calendar(end),
            Some(Ordering::Equal | Ordering::Less)
        )
    }
}

/// A validated wall-clock time with nanosecond precision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
    pub(crate) millisecond: u16,
    pub(crate) microsecond: u16,
    pub(crate) nanosecond: u16,
}

impl Time {
    /// Creates a time from hour, minute, second, and millisecond fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied fields are outside the valid wall-clock
    /// time range.
    pub fn new(hour: u8, minute: u8, second: u8, millisecond: u16) -> Result<Self, DateError> {
        temporal_time_from_fields(TimeFields {
            hour: Some(hour),
            minute: Some(minute),
            second: Some(second),
            millisecond: Some(millisecond),
            microsecond: Some(0),
            nanosecond: Some(0),
        })?;

        Ok(Self {
            hour,
            minute,
            second,
            millisecond,
            microsecond: 0,
            nanosecond: 0,
        })
    }

    /// Returns the hour field.
    #[must_use]
    pub const fn hour(&self) -> u8 {
        self.hour
    }

    /// Returns the minute field.
    #[must_use]
    pub const fn minute(&self) -> u8 {
        self.minute
    }

    /// Returns the second field.
    #[must_use]
    pub const fn second(&self) -> u8 {
        self.second
    }

    /// Returns the millisecond field.
    #[must_use]
    pub const fn millisecond(&self) -> u16 {
        self.millisecond
    }

    /// Returns the microsecond field.
    #[must_use]
    pub const fn microsecond(&self) -> u16 {
        self.microsecond
    }

    /// Returns the nanosecond field.
    #[must_use]
    pub const fn nanosecond(&self) -> u16 {
        self.nanosecond
    }

    /// Returns the canonical ISO 8601 representation of this time.
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        let whole_seconds = format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second);
        let fractional = u32::from(self.millisecond) * 1_000_000
            + u32::from(self.microsecond) * 1_000
            + u32::from(self.nanosecond);

        if fractional == 0 {
            return whole_seconds;
        }

        let mut fraction = format!("{fractional:09}");

        while fraction.ends_with('0') {
            fraction.pop();
        }

        format!("{whole_seconds}.{fraction}")
    }

    /// Adds a time duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented as a
    /// wall-clock time.
    pub fn add(&self, duration: TimeDuration) -> Result<Self, DateError> {
        let next = self
            .to_temporal()
            .add(&duration.to_temporal()?)
            .map_err(|error| DateError::CalendarError(error.to_string()))?;

        Ok(Self::from_temporal(next))
    }

    /// Subtracts a time duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented as a
    /// wall-clock time.
    pub fn subtract(&self, duration: TimeDuration) -> Result<Self, DateError> {
        self.add(TimeDuration {
            hours: -duration.hours,
            minutes: -duration.minutes,
            seconds: -duration.seconds,
            milliseconds: -duration.milliseconds,
            microseconds: -duration.microseconds,
            nanoseconds: -duration.nanoseconds,
        })
    }

    /// Returns a copy with selected fields replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if the replacement fields do not form a valid time.
    pub fn set(&self, fields: TimeFields) -> Result<Self, DateError> {
        let next = self
            .to_temporal()
            .with(temporal_partial_time(fields), Some(Overflow::Reject))
            .map_err(|error| DateError::CalendarError(error.to_string()))?;

        Ok(Self::from_temporal(next))
    }

    /// Cycles a time field by the requested amount.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested cycle would produce an invalid time.
    pub fn cycle(
        &self,
        field: TimeField,
        amount: i64,
        options: CycleTimeOptions,
    ) -> Result<Self, DateError> {
        if amount == 0 {
            return Ok(*self);
        }

        match field {
            TimeField::Hour => {
                if options.wrap {
                    let next = (i64::from(self.hour) + amount).rem_euclid(24);

                    self.set(TimeFields {
                        hour: Some(next as u8),
                        ..TimeFields::default()
                    })
                } else {
                    self.add(TimeDuration {
                        hours: amount,
                        ..TimeDuration::default()
                    })
                }
            }

            TimeField::Minute => {
                if options.wrap {
                    let next = (i64::from(self.minute) + amount).rem_euclid(60);

                    self.set(TimeFields {
                        minute: Some(next as u8),
                        ..TimeFields::default()
                    })
                } else {
                    self.add(TimeDuration {
                        minutes: amount,
                        ..TimeDuration::default()
                    })
                }
            }

            TimeField::Second => {
                if options.wrap {
                    let next = (i64::from(self.second) + amount).rem_euclid(60);

                    self.set(TimeFields {
                        second: Some(next as u8),
                        ..TimeFields::default()
                    })
                } else {
                    self.add(TimeDuration {
                        seconds: amount,
                        ..TimeDuration::default()
                    })
                }
            }

            TimeField::Millisecond => {
                if options.wrap {
                    let next = (i64::from(self.millisecond) + amount).rem_euclid(1000);

                    self.set(TimeFields {
                        millisecond: Some(next as u16),
                        ..TimeFields::default()
                    })
                } else {
                    self.add(TimeDuration {
                        milliseconds: amount,
                        ..TimeDuration::default()
                    })
                }
            }
        }
    }

    pub(crate) fn to_temporal(self) -> TemporalPlainTime {
        TemporalPlainTime::new(
            self.hour,
            self.minute,
            self.second,
            self.millisecond,
            self.microsecond,
            self.nanosecond,
        )
        .expect("stored time is always valid")
    }

    pub(crate) const fn from_temporal(time: TemporalPlainTime) -> Self {
        Self {
            hour: time.hour(),
            minute: time.minute(),
            second: time.second(),
            millisecond: time.millisecond(),
            microsecond: time.microsecond(),
            nanosecond: time.nanosecond(),
        }
    }
}

/// A calendar date paired with a wall-clock time.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDateTime {
    date: CalendarDate,
    time: Time,
}

impl CalendarDateTime {
    /// Creates a date-time from its date and time parts.
    #[must_use]
    pub const fn new(date: CalendarDate, time: Time) -> Self {
        Self { date, time }
    }

    /// Returns the date portion.
    #[must_use]
    pub const fn date(&self) -> &CalendarDate {
        &self.date
    }

    /// Returns the time portion.
    #[must_use]
    pub const fn time(&self) -> &Time {
        &self.time
    }

    /// Adds a duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented.
    pub fn add(&self, duration: DateTimeDuration) -> Result<Self, CalendarError> {
        let current = temporal_date_time_for(self)?;

        let next = current
            .add(&duration.to_temporal()?, Some(Overflow::Constrain))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        from_temporal_datetime(self.date.calendar, &next)
    }

    /// Subtracts a duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented.
    pub fn subtract(&self, duration: DateTimeDuration) -> Result<Self, CalendarError> {
        self.add(DateTimeDuration {
            date: DateDuration {
                years: -duration.date.years,
                months: -duration.date.months,
                weeks: -duration.date.weeks,
                days: -duration.date.days,
            },
            time: TimeDuration {
                hours: -duration.time.hours,
                minutes: -duration.time.minutes,
                seconds: -duration.time.seconds,
                milliseconds: -duration.time.milliseconds,
                microseconds: -duration.time.microseconds,
                nanoseconds: -duration.time.nanoseconds,
            },
        })
    }

    /// Returns a copy with selected fields replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if either the replacement date fields or replacement
    /// time fields are invalid.
    pub fn set(
        &self,
        date_fields: &CalendarDateFields,
        time_fields: TimeFields,
    ) -> Result<Self, CalendarError> {
        let next_date = if calendar_fields_are_empty(date_fields) {
            self.date.clone()
        } else {
            self.date.set(date_fields)?
        };

        Ok(Self {
            date: next_date,
            time: self
                .time
                .set(time_fields)
                .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
        })
    }

    /// Cycles one field.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested cycle would produce an invalid
    /// date-time value.
    pub fn cycle(
        &self,
        field: DateTimeField,
        amount: i64,
        options: CycleTimeOptions,
    ) -> Result<Self, CalendarError> {
        let narrowed_date_amount = || {
            i32::try_from(amount).map_err(|_| {
                CalendarError::Arithmetic(String::from("date-time cycle amount exceeds i32 range"))
            })
        };

        match field {
            DateTimeField::Year => Ok(Self {
                date: self.date.cycle(
                    DateField::Year,
                    narrowed_date_amount()?,
                    CycleOptions { wrap: options.wrap },
                )?,
                time: self.time,
            }),

            DateTimeField::Month => Ok(Self {
                date: self.date.cycle(
                    DateField::Month,
                    narrowed_date_amount()?,
                    CycleOptions { wrap: options.wrap },
                )?,
                time: self.time,
            }),

            DateTimeField::Day => Ok(Self {
                date: self.date.cycle(
                    DateField::Day,
                    narrowed_date_amount()?,
                    CycleOptions { wrap: options.wrap },
                )?,
                time: self.time,
            }),

            DateTimeField::Hour => Ok(Self {
                date: self.date.clone(),
                time: self
                    .time
                    .cycle(TimeField::Hour, amount, options)
                    .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
            }),

            DateTimeField::Minute => Ok(Self {
                date: self.date.clone(),
                time: self
                    .time
                    .cycle(TimeField::Minute, amount, options)
                    .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
            }),

            DateTimeField::Second => Ok(Self {
                date: self.date.clone(),
                time: self
                    .time
                    .cycle(TimeField::Second, amount, options)
                    .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
            }),

            DateTimeField::Millisecond => Ok(Self {
                date: self.date.clone(),
                time: self
                    .time
                    .cycle(TimeField::Millisecond, amount, options)
                    .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
            }),
        }
    }

    /// Converts this date-time into a different calendar.
    ///
    /// # Errors
    ///
    /// Returns an error if the target calendar is unsupported or the
    /// conversion fails in the underlying calendar engine.
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError> {
        Ok(Self {
            date: self.date.to_calendar(target)?,
            time: self.time,
        })
    }

    /// Converts this local date-time into `SystemTime` in the supplied time zone.
    ///
    /// # Errors
    ///
    /// Returns an error if the time zone is invalid or the local date-time
    /// cannot be resolved to an instant.
    #[cfg(feature = "std")]
    pub fn to_system_time(
        &self,
        time_zone: &TimeZoneId,
        disambiguation: Disambiguation,
    ) -> Result<SystemTime, CalendarError> {
        to_zoned_date_time(self, time_zone, disambiguation)?.to_system_time()
    }
}

/// Combines a date with an optional time, defaulting to local midnight.
#[must_use]
pub fn to_calendar_date_time(date: &CalendarDate, time: Option<Time>) -> CalendarDateTime {
    CalendarDateTime::new(date.clone(), time.unwrap_or_default())
}

/// A zoned date-time storing an exact instant, time zone, and calendar projection.
#[cfg(feature = "std")]
#[derive(Clone, Debug)]
pub struct ZonedDateTime {
    pub(crate) inner: temporal_rs::ZonedDateTime,
    pub(crate) calendar: CalendarSystem,
    pub(crate) time_zone: TimeZoneId,
}

#[cfg(feature = "std")]
impl ZonedDateTime {
    /// Creates a zoned date-time from a local date-time and time zone.
    ///
    /// # Errors
    ///
    /// Returns an error if the time zone is invalid or the local date-time
    /// cannot be resolved using the requested disambiguation strategy.
    pub fn new(
        date_time: &CalendarDateTime,
        time_zone: TimeZoneId,
        disambiguation: Disambiguation,
    ) -> Result<Self, CalendarError> {
        let temporal_time_zone = time_zone.to_temporal()?;

        let temporal_date_time = temporal_date_time_for(date_time)?;
        let calendar_fields = temporal_calendar_fields(&CalendarDateFields {
            era: date_time.date.era.clone(),
            year: Some(date_time.date.year()),
            month: Some(date_time.date.month()),
            month_code: date_time.date.month_code(),
            day: Some(date_time.date.day()),
        })?;

        let mut parsed = PartialZonedDateTime::new()
            .with_timezone(Some(temporal_time_zone))
            .with_calendar_fields(calendar_fields)
            .with_time(temporal_partial_time(TimeFields {
                hour: Some(date_time.time.hour),
                minute: Some(date_time.time.minute),
                second: Some(date_time.time.second),
                millisecond: Some(date_time.time.millisecond),
                microsecond: Some(date_time.time.microsecond),
                nanosecond: Some(date_time.time.nanosecond),
            }));

        parsed.calendar = temporal_date_time.calendar().clone();

        let inner = temporal_rs::ZonedDateTime::from_partial(
            parsed,
            Some(Overflow::Reject),
            Some(disambiguation.to_temporal()),
            None,
        )
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner,
            calendar: date_time.date.calendar,
            time_zone,
        })
    }

    /// Returns the time-zone identifier.
    #[must_use]
    pub const fn time_zone(&self) -> &TimeZoneId {
        &self.time_zone
    }

    /// Returns the current UTC offset in whole minutes.
    #[must_use]
    pub fn offset_minutes(&self) -> i32 {
        i32::try_from(self.inner.offset_nanoseconds() / 60_000_000_000).unwrap_or(0)
    }

    /// Adds a duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented.
    pub fn add(&self, duration: DateTimeDuration) -> Result<Self, CalendarError> {
        let inner = self
            .inner
            .add(&duration.to_temporal()?, Some(Overflow::Constrain))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner,
            calendar: self.calendar,
            time_zone: self.time_zone.clone(),
        })
    }

    /// Subtracts a duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the arithmetic result cannot be represented.
    pub fn subtract(&self, duration: DateTimeDuration) -> Result<Self, CalendarError> {
        let inner = self
            .inner
            .subtract(&duration.to_temporal()?, Some(Overflow::Constrain))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner,
            calendar: self.calendar,
            time_zone: self.time_zone.clone(),
        })
    }

    /// Changes the attached time zone while preserving the instant.
    ///
    /// # Errors
    ///
    /// Returns an error if the target time zone is invalid.
    pub fn with_time_zone(&self, time_zone: TimeZoneId) -> Result<Self, CalendarError> {
        let inner = self
            .inner
            .with_timezone(time_zone.to_temporal()?)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner,
            calendar: self.calendar,
            time_zone,
        })
    }

    /// Changes the display calendar while preserving the instant.
    ///
    /// # Errors
    ///
    /// Returns an error if the target calendar is unsupported for zoned
    /// conversion.
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError> {
        let temporal = temporal_calendar_for(target).ok_or_else(|| {
            CalendarConversionError::Icu(String::from(
                "simulated mecca zoned conversion is not yet supported",
            ))
        })?;

        Ok(Self {
            inner: self.inner.with_calendar(temporal),
            calendar: target,
            time_zone: self.time_zone.clone(),
        })
    }

    /// Converts this exact instant into `SystemTime`.
    ///
    /// # Errors
    ///
    /// Returns an error if the represented instant is outside the supported
    /// `SystemTime` range.
    pub fn to_system_time(&self) -> Result<SystemTime, CalendarError> {
        system_time_from_epoch_nanoseconds(self.inner.epoch_nanoseconds().as_i128())
    }
}

#[cfg(feature = "std")]
impl PartialEq for ZonedDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.calendar == other.calendar
            && self.time_zone == other.time_zone
            && self.inner.compare_instant(&other.inner).is_eq()
    }
}

#[cfg(feature = "std")]
impl Eq for ZonedDateTime {}

/// Converts a date into a zoned date-time at midnight in the target zone.
///
/// # Errors
///
/// Returns an error if the time zone is invalid or the local midnight instant
/// cannot be resolved.
#[cfg(feature = "std")]
pub fn to_zoned(
    date: &CalendarDate,
    time_zone: &TimeZoneId,
) -> Result<ZonedDateTime, CalendarError> {
    to_zoned_date_time(
        &to_calendar_date_time(date, None),
        time_zone,
        Disambiguation::Compatible,
    )
}

/// Converts a local date-time into a zoned date-time using the requested disambiguation.
#[cfg(feature = "std")]
///
/// # Errors
///
/// Returns an error if the time zone is invalid or the local date-time cannot
/// be resolved using the requested disambiguation strategy.
pub fn to_zoned_date_time(
    date_time: &CalendarDateTime,
    time_zone: &TimeZoneId,
    disambiguation: Disambiguation,
) -> Result<ZonedDateTime, CalendarError> {
    ZonedDateTime::new(date_time, time_zone.clone(), disambiguation)
}

/// Shared trait used by query helpers over date-like values.
pub trait DateValue: Clone {
    /// Returns the calendar-date view of this value.
    fn date_value(&self) -> CalendarDate;

    /// Returns a value of the same type with the date portion replaced.
    #[must_use]
    fn with_date_value(&self, date: CalendarDate) -> Self;
}

impl DateValue for CalendarDate {
    fn date_value(&self) -> CalendarDate {
        self.clone()
    }

    fn with_date_value(&self, date: CalendarDate) -> Self {
        date
    }
}

impl DateValue for CalendarDateTime {
    fn date_value(&self) -> CalendarDate {
        self.date.clone()
    }

    fn with_date_value(&self, date: CalendarDate) -> Self {
        Self {
            date,
            time: self.time,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectedDate {
    pub(crate) era: Option<Era>,
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) month_code: Option<MonthCode>,
    pub(crate) months_in_year: u8,
    pub(crate) days_in_month: u8,
}

pub(crate) const fn temporal_calendar_for(calendar: CalendarSystem) -> Option<Calendar> {
    Some(match calendar {
        CalendarSystem::Iso8601 => Calendar::ISO,
        CalendarSystem::Gregorian => Calendar::GREGORIAN,
        CalendarSystem::Buddhist => Calendar::BUDDHIST,
        CalendarSystem::Japanese => Calendar::JAPANESE,
        CalendarSystem::Hebrew => Calendar::HEBREW,
        CalendarSystem::IslamicCivil => Calendar::HIJRI_TABULAR_FRIDAY,
        CalendarSystem::IslamicUmmAlQura => Calendar::HIJRI_UMM_AL_QURA,
        CalendarSystem::Persian => Calendar::PERSIAN,
        CalendarSystem::Indian => Calendar::INDIAN,
        CalendarSystem::Chinese => Calendar::CHINESE,
        CalendarSystem::Coptic => Calendar::COPTIC,
        CalendarSystem::Dangi => Calendar::DANGI,
        CalendarSystem::Ethiopic => Calendar::ETHIOPIAN,
        CalendarSystem::EthiopicAmeteAlem => Calendar::ETHIOPIAN_AMETE_ALEM,
        CalendarSystem::Roc => Calendar::ROC,
    })
}

pub(crate) fn projected_date_for_calendar(
    date: &CalendarDate,
    calendar: CalendarSystem,
) -> Option<ProjectedDate> {
    let iso = temporal_iso_date(date.iso_year, date.iso_month, date.iso_day).ok()?;

    let temporal = iso.with_calendar(temporal_calendar_for(calendar)?);

    let era = temporal
        .era()
        .map(|era| canonical_era(calendar, era.as_str()))
        .or_else(|| infer_public_era(calendar, date.iso_year, date.iso_month, date.iso_day));

    let year = temporal.era_year().unwrap_or_else(|| temporal.year());

    Some(ProjectedDate {
        era,
        year,
        month: temporal.month(),
        day: temporal.day(),
        month_code: Some(month_code_from_temporal(temporal.month_code())),
        months_in_year: u8::try_from(temporal.months_in_year()).ok()?,
        days_in_month: u8::try_from(temporal.days_in_month()).ok()?,
    })
}

fn build_calendar_date(
    calendar: CalendarSystem,
    fields: &CalendarDateFields,
    overflow: Overflow,
) -> Result<CalendarDate, CalendarError> {
    let temporal =
        PlainDate::from_partial(temporal_partial_date(calendar, fields)?, Some(overflow))
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let iso = temporal.with_calendar(Calendar::ISO);

    let built = build_from_iso_parts(calendar, iso.year(), iso.month(), iso.day())
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    if let Some(expected) = fields.era.as_ref().map(|value| value.code.as_str())
        && built.era.as_ref().map(|value| value.code.as_str()) != Some(expected)
    {
        return Err(CalendarError::Arithmetic(String::from(
            "requested era is outside the supported range for the constructed date",
        )));
    }

    if let Some(expected) = fields.year
        && built.year != expected
    {
        return Err(CalendarError::Arithmetic(String::from(
            "requested year is outside the supported range for the constructed date",
        )));
    }

    if let Some(expected) = fields.month
        && built.month != expected
    {
        return Err(CalendarError::Arithmetic(String::from(
            "requested month is outside the supported range for the constructed date",
        )));
    }

    if let Some(expected) = fields.day
        && built.day != expected
    {
        return Err(CalendarError::Arithmetic(String::from(
            "requested day is outside the supported range for the constructed date",
        )));
    }

    if let Some(expected) = fields.month_code
        && built.month_code != Some(expected)
    {
        return Err(CalendarError::Arithmetic(String::from(
            "requested month code does not match the resolved month",
        )));
    }

    Ok(built)
}

pub(crate) fn build_from_iso_parts(
    calendar: CalendarSystem,
    iso_year: i32,
    iso_month: u8,
    iso_day: u8,
) -> Result<CalendarDate, CalendarConversionError> {
    let projected = projected_date_for_iso_parts(iso_year, iso_month, iso_day, calendar)?;

    Ok(CalendarDate {
        calendar,
        era: projected.era,
        year: projected.year,
        month: projected.month,
        month_code: projected.month_code,
        day: projected.day,
        iso_year,
        iso_month,
        iso_day,
    })
}

fn projected_date_for_iso_parts(
    iso_year: i32,
    iso_month: u8,
    iso_day: u8,
    calendar: CalendarSystem,
) -> Result<ProjectedDate, CalendarConversionError> {
    let iso = temporal_iso_date(iso_year, iso_month, iso_day)
        .map_err(|error| CalendarConversionError::Icu(error.to_string()))?;

    let temporal = iso.with_calendar(
        temporal_calendar_for(calendar)
            .ok_or_else(|| CalendarConversionError::Icu(String::from("unsupported calendar")))?,
    );

    let era = temporal
        .era()
        .map(|era| canonical_era(calendar, era.as_str()))
        .or_else(|| infer_public_era(calendar, iso_year, iso_month, iso_day));

    let year = temporal.era_year().unwrap_or_else(|| temporal.year());

    Ok(ProjectedDate {
        era,
        year,
        month: temporal.month(),
        day: temporal.day(),
        month_code: Some(month_code_from_temporal(temporal.month_code())),
        months_in_year: u8::try_from(temporal.months_in_year())
            .map_err(|error| CalendarConversionError::Icu(error.to_string()))?,
        days_in_month: u8::try_from(temporal.days_in_month())
            .map_err(|error| CalendarConversionError::Icu(error.to_string()))?,
    })
}

fn temporal_partial_date(
    calendar: CalendarSystem,
    fields: &CalendarDateFields,
) -> Result<PartialDate, CalendarError> {
    let year = fields
        .year
        .ok_or_else(|| CalendarError::Arithmetic(String::from("year is required")))?;

    let month_missing = fields.month.is_none() && fields.month_code.is_none();

    if month_missing || fields.day.is_none() {
        return Err(CalendarError::Arithmetic(String::from(
            "month and day are required",
        )));
    }

    let mut partial = PartialDate::new()
        .with_calendar(
            temporal_calendar_for(calendar)
                .ok_or_else(|| CalendarError::Arithmetic(String::from("unsupported calendar")))?,
        )
        .with_day(fields.day)
        .with_month(fields.month)
        .with_month_code(fields.month_code.map(MonthCode::to_temporal));

    if let Some(era) = &fields.era {
        let era_code = match (calendar, era.code.as_str()) {
            (CalendarSystem::Gregorian, "bc") => "bce",

            (CalendarSystem::Gregorian, "ad") => "ce",

            _ => era.code.as_str(),
        };

        let era_value = tinystr::TinyAsciiStr::<19>::try_from_utf8(era_code.as_bytes())
            .map_err(|_| CalendarError::Arithmetic(String::from("invalid era code")))?;

        partial = partial.with_era(Some(era_value)).with_era_year(Some(year));
    } else {
        partial = partial.with_year(Some(year));
    }

    Ok(partial)
}

const fn calendar_fields_are_empty(fields: &CalendarDateFields) -> bool {
    fields.era.is_none()
        && fields.year.is_none()
        && fields.month.is_none()
        && fields.month_code.is_none()
        && fields.day.is_none()
}

fn temporal_calendar_fields(fields: &CalendarDateFields) -> Result<CalendarFields, CalendarError> {
    if calendar_fields_are_empty(fields) {
        return Err(CalendarError::Arithmetic(String::from(
            "at least one calendar field is required",
        )));
    }

    if let Some(era) = &fields.era {
        let mut temporal = CalendarFields::new()
            .with_optional_month(fields.month)
            .with_optional_month_code(fields.month_code.map(MonthCode::to_temporal))
            .with_optional_day(fields.day);

        let era_code = match era.code.as_str() {
            "bc" => "bce",

            "ad" => "ce",

            _ => era.code.as_str(),
        };

        let era_value = tinystr::TinyAsciiStr::<19>::try_from_utf8(era_code.as_bytes())
            .map_err(|_| CalendarError::Arithmetic(String::from("invalid era code")))?;

        temporal = temporal
            .with_era(Some(era_value))
            .with_era_year(fields.year);

        return Ok(temporal);
    }

    Ok(CalendarFields::new()
        .with_optional_year(fields.year)
        .with_optional_month(fields.month)
        .with_optional_month_code(fields.month_code.map(MonthCode::to_temporal))
        .with_optional_day(fields.day))
}

fn temporal_date_for(date: &CalendarDate) -> Result<PlainDate, CalendarError> {
    let iso = temporal_iso_date(date.iso_year, date.iso_month, date.iso_day)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    Ok(iso.with_calendar(
        temporal_calendar_for(date.calendar)
            .ok_or_else(|| CalendarError::Arithmetic(String::from("unsupported calendar")))?,
    ))
}

fn temporal_date_time_for(
    value: &CalendarDateTime,
) -> Result<TemporalPlainDateTime, CalendarError> {
    let date = temporal_date_for(&value.date)?;

    date.to_plain_date_time(Some(value.time.to_temporal()))
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
}

fn from_temporal_datetime(
    calendar: CalendarSystem,
    value: &TemporalPlainDateTime,
) -> Result<CalendarDateTime, CalendarError> {
    let iso = value.with_calendar(Calendar::ISO);

    Ok(CalendarDateTime {
        date: build_from_iso_parts(calendar, iso.year(), iso.month(), iso.day())
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
        time: Time::from_temporal(iso.to_plain_time()),
    })
}

fn temporal_iso_date(
    year: i32,
    month: u8,
    day: u8,
) -> Result<PlainDate, temporal_rs::TemporalError> {
    PlainDate::try_new_iso(year, month, day)
}

const fn temporal_partial_time(fields: TimeFields) -> PartialTime {
    PartialTime::new()
        .with_hour(fields.hour)
        .with_minute(fields.minute)
        .with_second(fields.second)
        .with_millisecond(fields.millisecond)
        .with_microsecond(fields.microsecond)
        .with_nanosecond(fields.nanosecond)
}

fn temporal_time_from_fields(fields: TimeFields) -> Result<TemporalPlainTime, DateError> {
    TemporalPlainTime::new(
        fields.hour.ok_or(DateError::InvalidDate)?,
        fields.minute.ok_or(DateError::InvalidDate)?,
        fields.second.ok_or(DateError::InvalidDate)?,
        fields.millisecond.ok_or(DateError::InvalidDate)?,
        fields.microsecond.unwrap_or(0),
        fields.nanosecond.unwrap_or(0),
    )
    .map_err(|error| DateError::CalendarError(error.to_string()))
}

#[cfg(feature = "std")]
fn system_time_from_epoch_nanoseconds(
    epoch_nanoseconds: i128,
) -> Result<SystemTime, CalendarError> {
    if epoch_nanoseconds >= 0 {
        let seconds = u64::try_from(epoch_nanoseconds / 1_000_000_000).map_err(|_| {
            CalendarError::Arithmetic(String::from("instant exceeds SystemTime positive range"))
        })?;

        let nanoseconds = u32::try_from(epoch_nanoseconds % 1_000_000_000).map_err(|_| {
            CalendarError::Arithmetic(String::from(
                "instant nanoseconds exceed SystemTime precision",
            ))
        })?;

        return UNIX_EPOCH
            .checked_add(Duration::new(seconds, nanoseconds))
            .ok_or_else(|| {
                CalendarError::Arithmetic(String::from("instant exceeds SystemTime positive range"))
            });
    }

    let magnitude = epoch_nanoseconds.checked_neg().ok_or_else(|| {
        CalendarError::Arithmetic(String::from("instant exceeds SystemTime negative range"))
    })?;

    let seconds = u64::try_from(magnitude / 1_000_000_000).map_err(|_| {
        CalendarError::Arithmetic(String::from("instant exceeds SystemTime negative range"))
    })?;

    let nanoseconds = u32::try_from(magnitude % 1_000_000_000).map_err(|_| {
        CalendarError::Arithmetic(String::from(
            "instant nanoseconds exceed SystemTime precision",
        ))
    })?;

    UNIX_EPOCH
        .checked_sub(Duration::new(seconds, nanoseconds))
        .ok_or_else(|| {
            CalendarError::Arithmetic(String::from("instant exceeds SystemTime negative range"))
        })
}

impl DateDuration {
    fn to_temporal(self) -> Result<TemporalDuration, CalendarError> {
        TemporalDuration::new(
            i64::from(self.years),
            i64::from(self.months),
            i64::from(self.weeks),
            i64::from(self.days),
            0,
            0,
            0,
            0,
            0,
            0,
        )
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }
}

impl TimeDuration {
    fn to_temporal(self) -> Result<TemporalDuration, DateError> {
        TemporalDuration::new(
            0,
            0,
            0,
            0,
            self.hours,
            self.minutes,
            self.seconds,
            self.milliseconds,
            i128::from(self.microseconds),
            i128::from(self.nanoseconds),
        )
        .map_err(|error| DateError::CalendarError(error.to_string()))
    }
}

impl DateTimeDuration {
    fn to_temporal(self) -> Result<TemporalDuration, CalendarError> {
        TemporalDuration::new(
            i64::from(self.date.years),
            i64::from(self.date.months),
            i64::from(self.date.weeks),
            i64::from(self.date.days),
            self.time.hours,
            self.time.minutes,
            self.time.seconds,
            self.time.milliseconds,
            i128::from(self.time.microseconds),
            i128::from(self.time.nanoseconds),
        )
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }
}

fn format_iso_year(year: i32) -> String {
    if (0..=9999).contains(&year) {
        return format!("{year:04}");
    }

    let absolute_year = i64::from(year).abs();

    format!(
        "{}{absolute_year:06}",
        if year.is_negative() { "-" } else { "+" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Locale, StubIcuProvider};

    fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("Gregorian fixture should validate")
    }

    #[test]
    fn calendar_date_comparison_helpers_require_matching_calendar_and_era() {
        let start = gregorian_date(2024, 3, 1);

        let middle = gregorian_date(2024, 3, 15);

        let end = gregorian_date(2024, 3, 31);

        let japanese = middle
            .to_calendar(CalendarSystem::Japanese)
            .expect("Gregorian to Japanese conversion should succeed");

        let bc = CalendarDate::new(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("bc"),
                    display_name: String::from("BC"),
                }),
                year: Some(1),
                month: Some(1),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .expect("Gregorian BC date should validate");

        assert_eq!(middle.compare_within_calendar(&end), Some(Ordering::Less));
        assert_eq!(middle.compare_within_calendar(&japanese), None);
        assert_eq!(middle.compare_within_calendar(&bc), None);
        assert!(middle.is_between(&start, &end));
        assert!(!middle.is_between(&japanese, &end));
    }

    #[test]
    fn calendar_date_helpers_cover_ordering_and_day_arithmetic() {
        let earlier = gregorian_date(2024, 3, 10);

        let later = earlier.add_days(5).expect("day arithmetic should work");

        assert_eq!(later.to_iso8601(), "2024-03-15");
        assert_eq!(earlier.days_until(&later).expect("date difference"), 5);
        assert!(earlier.is_before(&later).expect("chronological comparison"));
    }

    #[test]
    fn calendar_date_to_iso8601_uses_expanded_years_outside_four_digit_range() {
        let bce = gregorian_date(-50, 1, 2);
        let large = gregorian_date(10_000, 12, 31);

        assert_eq!(bce.to_iso8601(), "-000050-01-02");
        assert_eq!(large.to_iso8601(), "+010000-12-31");
    }

    #[test]
    fn calendar_date_and_time_field_validators_reject_incomplete_or_invalid_inputs() {
        let missing_month = temporal_partial_date(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                year: Some(2024),
                day: Some(15),
                ..CalendarDateFields::default()
            },
        )
        .expect_err("month is required");

        assert!(
            missing_month
                .to_string()
                .contains("month and day are required")
        );

        let empty = temporal_calendar_fields(&CalendarDateFields::default())
            .expect_err("at least one field is required");

        assert!(
            empty
                .to_string()
                .contains("at least one calendar field is required")
        );

        let invalid_era = temporal_calendar_fields(&CalendarDateFields {
            era: Some(Era {
                code: String::from("way-too-long-era-code"),
                display_name: String::from("Bad"),
            }),
            year: Some(2024),
            ..CalendarDateFields::default()
        })
        .expect_err("invalid era code should be rejected");

        assert!(invalid_era.to_string().contains("invalid era code"));

        let missing_time = temporal_time_from_fields(TimeFields {
            hour: Some(9),
            minute: Some(30),
            ..TimeFields::default()
        })
        .expect_err("second and millisecond are required");

        assert_eq!(missing_time, DateError::InvalidDate);

        assert!(CalendarDate::new_iso8601(2024, 2, 30).is_err());
        assert!(CalendarDate::new_gregorian(2024, 13, 1).is_err());
    }

    #[test]
    fn coptic_projection_infers_public_eras_around_the_epoch_boundary() {
        let before_epoch = infer_public_era(CalendarSystem::Coptic, 284, 8, 28)
            .expect("pre-epoch Coptic era should be inferred");

        let at_epoch_era = infer_public_era(CalendarSystem::Coptic, 284, 8, 29)
            .expect("epoch Coptic era should be inferred");

        let at_epoch = build_from_iso_parts(CalendarSystem::Coptic, 284, 8, 29)
            .expect("epoch Coptic date should project");

        assert_eq!(before_epoch.code, "bce");
        assert_eq!(at_epoch_era.code, "ce");
        assert_eq!(at_epoch.era().map(|value| value.code.as_str()), Some("ce"));

        let projected = projected_date_for_calendar(&at_epoch, CalendarSystem::Coptic)
            .expect("projected Coptic date should exist");

        assert_eq!(
            projected.era.as_ref().map(|value| value.code.as_str()),
            Some("ce")
        );
    }

    #[test]
    fn calendar_date_constructor_rejects_mismatched_fields_and_zero_cycles_no_op() {
        let mismatch = CalendarDate::new(
            CalendarSystem::Hebrew,
            &CalendarDateFields {
                year: Some(5784),
                month: Some(6),
                month_code: Some(MonthCode::new("M06").expect("month code should validate")),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .expect_err("mismatched month code should be rejected");

        assert!(mismatch.to_string().contains("calendar arithmetic failed"));

        let date = gregorian_date(2024, 3, 15);

        assert_eq!(
            date.cycle(DateField::Month, 0, CycleOptions { wrap: true })
                .expect("zero month cycle should no-op"),
            date
        );
        assert_eq!(
            date.cycle(DateField::Day, 1, CycleOptions { wrap: false })
                .expect("non-wrapping day cycle should delegate to add"),
            gregorian_date(2024, 3, 16)
        );
    }

    #[test]
    fn time_helpers_cover_subtract_set_and_non_wrapping_cycles() {
        let time = Time::new(9, 30, 45, 125).expect("time should validate");
        let precise = time
            .set(TimeFields {
                microsecond: Some(222),
                nanosecond: Some(330),
                ..TimeFields::default()
            })
            .expect("time set should succeed");

        assert_eq!(
            time.subtract(TimeDuration {
                minutes: 45,
                ..TimeDuration::default()
            })
            .expect("time subtraction should succeed")
            .hour(),
            8
        );
        assert_eq!(
            time.set(TimeFields {
                second: Some(15),
                microsecond: Some(222),
                nanosecond: Some(333),
                ..TimeFields::default()
            })
            .expect("time set should succeed")
            .microsecond(),
            222
        );
        assert_eq!(
            time.cycle(TimeField::Minute, 45, CycleTimeOptions { wrap: false })
                .expect("non-wrapping minute cycle should delegate to add")
                .hour(),
            10
        );
        assert_eq!(
            time.cycle(TimeField::Second, 30, CycleTimeOptions { wrap: false })
                .expect("non-wrapping second cycle should delegate to add")
                .minute(),
            31
        );
        assert_eq!(
            time.cycle(
                TimeField::Millisecond,
                900,
                CycleTimeOptions { wrap: false },
            )
            .expect("non-wrapping millisecond cycle should delegate to add")
            .second(),
            46
        );
        assert_eq!(
            time.cycle(TimeField::Hour, 0, CycleTimeOptions { wrap: true })
                .expect("zero hour cycle should no-op"),
            time
        );
        assert_eq!(time.to_iso8601(), "09:30:45.125");
        assert_eq!(precise.to_iso8601(), "09:30:45.12522233");
    }

    #[test]
    fn date_time_cycle_covers_time_fields_and_calendar_conversion() {
        let date_time = CalendarDateTime::new(
            gregorian_date(2024, 3, 15),
            Time::new(9, 30, 45, 125).expect("time should validate"),
        );

        let hour = date_time
            .cycle(DateTimeField::Hour, 2, CycleTimeOptions { wrap: true })
            .expect("hour cycle should succeed");

        let minute = date_time
            .cycle(DateTimeField::Minute, -15, CycleTimeOptions { wrap: true })
            .expect("minute cycle should succeed");

        let second = date_time
            .cycle(DateTimeField::Second, 30, CycleTimeOptions { wrap: true })
            .expect("second cycle should succeed");

        let millisecond = date_time
            .cycle(
                DateTimeField::Millisecond,
                900,
                CycleTimeOptions { wrap: true },
            )
            .expect("millisecond cycle should succeed");

        let japanese = date_time
            .to_calendar(CalendarSystem::Japanese)
            .expect("calendar conversion should succeed");

        assert_eq!(hour.time().hour(), 11);
        assert_eq!(minute.time().minute(), 15);
        assert_eq!(second.time().second(), 15);
        assert_eq!(millisecond.time().millisecond(), 25);
        assert_eq!(japanese.time(), date_time.time());
        assert_eq!(japanese.date().calendar(), CalendarSystem::Japanese);
    }

    #[test]
    fn calendar_date_time_set_allows_time_only_updates() {
        let date_time = CalendarDateTime::new(
            gregorian_date(2024, 3, 15),
            Time::new(9, 30, 45, 125).expect("time should validate"),
        );

        let updated = date_time
            .set(
                &CalendarDateFields::default(),
                TimeFields {
                    minute: Some(5),
                    ..TimeFields::default()
                },
            )
            .expect("time-only set should succeed");

        assert_eq!(updated.date(), date_time.date());
        assert_eq!(updated.time().hour(), 9);
        assert_eq!(updated.time().minute(), 5);
    }

    #[test]
    fn date_time_helpers_cover_subtract_set_errors_and_additional_cycles() {
        let date_time = CalendarDateTime::new(
            gregorian_date(2024, 3, 15),
            Time::new(9, 30, 45, 125).expect("time should validate"),
        );

        let previous = date_time
            .subtract(DateTimeDuration {
                date: DateDuration {
                    days: 1,
                    ..DateDuration::default()
                },
                time: TimeDuration {
                    hours: 1,
                    ..TimeDuration::default()
                },
            })
            .expect("date-time subtraction should succeed");

        assert_eq!(previous.date().to_iso8601(), "2024-03-14");
        assert_eq!(previous.time().hour(), 8);

        let invalid = date_time.set(
            &CalendarDateFields::default(),
            TimeFields {
                hour: Some(25),
                ..TimeFields::default()
            },
        );

        assert!(invalid.is_err());

        assert_eq!(
            date_time
                .cycle(DateTimeField::Year, 1, CycleTimeOptions { wrap: false })
                .expect("year cycle should succeed")
                .date()
                .year(),
            2025
        );
        assert_eq!(
            date_time
                .cycle(DateTimeField::Day, -1, CycleTimeOptions { wrap: false })
                .expect("day cycle should succeed")
                .date()
                .day(),
            14
        );

        let overflow = date_time.cycle(
            DateTimeField::Month,
            i64::from(i32::MAX) + 1,
            CycleTimeOptions { wrap: false },
        );

        assert!(matches!(
            overflow,
            Err(CalendarError::Arithmetic(message))
                if message == "date-time cycle amount exceeds i32 range"
        ));
    }

    #[test]
    fn calendar_date_cycle_and_construction_helpers_cover_remaining_branches() {
        let date = gregorian_date(2024, 3, 15);

        assert_eq!(
            date.cycle(DateField::Month, 10, CycleOptions { wrap: true })
                .expect("wrapping month cycle should stay within the current year")
                .month(),
            1
        );
        assert_eq!(
            date.cycle(DateField::Day, 20, CycleOptions { wrap: true })
                .expect("wrapping day cycle should stay within the current month")
                .day(),
            4
        );
        assert_eq!(gregorian_date(2024, 3, 12).weekday(), Weekday::Tuesday);
        assert_eq!(gregorian_date(2024, 3, 13).weekday(), Weekday::Wednesday);
        assert_eq!(gregorian_date(2024, 3, 14).weekday(), Weekday::Thursday);
        assert!(!date.is_between(&gregorian_date(2024, 3, 16), &gregorian_date(2024, 3, 31)));

        let month_mismatch = build_calendar_date(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                year: Some(2024),
                month: Some(13),
                day: Some(5),
                ..CalendarDateFields::default()
            },
            Overflow::Constrain,
        )
        .expect_err("constrained month mismatch should be rejected");

        assert!(
            month_mismatch
                .to_string()
                .contains("requested month is outside the supported range")
        );

        let day_mismatch = build_calendar_date(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                year: Some(2024),
                month: Some(2),
                day: Some(31),
                ..CalendarDateFields::default()
            },
            Overflow::Constrain,
        )
        .expect_err("constrained day mismatch should be rejected");

        assert!(
            day_mismatch
                .to_string()
                .contains("requested day is outside the supported range")
        );
    }

    #[test]
    fn helper_conversions_cover_projection_and_temporal_roundtrips() {
        let japanese = gregorian_date(2024, 3, 15)
            .to_calendar(CalendarSystem::Japanese)
            .expect("Japanese conversion should succeed");

        let projected = projected_date_for_calendar(&japanese, CalendarSystem::Gregorian)
            .expect("Gregorian projection should exist");

        let temporal_date = temporal_date_for(&japanese).expect("temporal date should exist");

        let temporal_date_time = temporal_date_time_for(&CalendarDateTime::new(
            japanese.clone(),
            Time::new(9, 30, 45, 125).expect("time should validate"),
        ))
        .expect("temporal date-time should exist");

        let rebuilt = from_temporal_datetime(
            CalendarSystem::Japanese,
            &temporal_date_time.with_calendar(temporal_date.calendar().clone()),
        )
        .expect("roundtrip from temporal date-time should succeed");

        assert_eq!(projected.year, 2024);
        assert_eq!(projected.month, 3);
        assert_eq!(projected.day, 15);
        assert_eq!(temporal_date.year(), japanese.iso_year);
        assert_eq!(rebuilt.date().calendar(), CalendarSystem::Japanese);
        assert_eq!(rebuilt.time().second(), 45);
        assert!(temporal_calendar_for(CalendarSystem::Persian).is_some());
    }

    #[cfg(feature = "std")]
    #[test]
    fn zoned_date_time_overlap_disambiguation_selects_distinct_offsets() {
        let overlap = CalendarDateTime::new(
            gregorian_date(2024, 11, 3),
            Time::new(1, 30, 0, 0).expect("time should validate"),
        );

        let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

        let earlier = ZonedDateTime::new(&overlap, time_zone.clone(), Disambiguation::Earlier)
            .expect("earlier overlap disambiguation should succeed");
        let later = ZonedDateTime::new(&overlap, time_zone, Disambiguation::Later)
            .expect("later overlap disambiguation should succeed");

        assert_eq!(earlier.offset_minutes(), -4 * 60);
        assert_eq!(later.offset_minutes(), -5 * 60);
        assert_ne!(earlier, later);
    }

    #[cfg(feature = "std")]
    #[test]
    fn zoned_date_time_helpers_cover_add_subtract_zone_switch_and_calendar_switch() {
        let local = CalendarDateTime::new(
            gregorian_date(2024, 3, 15),
            Time::new(9, 30, 0, 0).expect("time should validate"),
        );

        let zoned = ZonedDateTime::new(
            &local,
            TimeZoneId::new("America/New_York").expect("zone should validate"),
            Disambiguation::Compatible,
        )
        .expect("zoned date-time should validate");

        let advanced = zoned
            .add(DateTimeDuration {
                date: DateDuration {
                    days: 1,
                    ..DateDuration::default()
                },
                time: TimeDuration::default(),
            })
            .expect("zoned addition should succeed");

        let rewound = advanced
            .subtract(DateTimeDuration {
                date: DateDuration {
                    days: 1,
                    ..DateDuration::default()
                },
                time: TimeDuration::default(),
            })
            .expect("zoned subtraction should succeed");

        let utc = zoned
            .with_time_zone(TimeZoneId::new("UTC").expect("UTC should validate"))
            .expect("time-zone change should succeed");

        let japanese = zoned
            .to_calendar(CalendarSystem::Japanese)
            .expect("calendar projection should succeed");

        assert_eq!(advanced.time_zone().as_str(), "America/New_York");
        assert_eq!(
            rewound.to_system_time().expect("system time"),
            zoned.to_system_time().expect("system time")
        );
        assert_eq!(utc.time_zone().as_str(), "UTC");
        assert_eq!(japanese.time_zone().as_str(), "America/New_York");
    }

    #[cfg(feature = "std")]
    #[test]
    fn zoned_date_time_new_preserves_era_based_calendar_fields() {
        let heisei = CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("heisei"),
                    display_name: String::from("Heisei"),
                }),
                year: Some(2),
                month: Some(1),
                day: Some(8),
                ..CalendarDateFields::default()
            },
        )
        .expect("Japanese fixture should validate");

        let bce = CalendarDate::new(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("bc"),
                    display_name: String::from("BC"),
                }),
                year: Some(1),
                month: Some(1),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .expect("BCE Gregorian fixture should validate");

        let time = Time::new(9, 30, 0, 0).expect("time should validate");
        let utc = TimeZoneId::new("UTC").expect("UTC should validate");

        let japanese_zoned = ZonedDateTime::new(
            &CalendarDateTime::new(heisei.clone(), time),
            utc.clone(),
            Disambiguation::Compatible,
        )
        .expect("Japanese zoned date-time should validate");

        let gregorian_zoned = ZonedDateTime::new(
            &CalendarDateTime::new(bce.clone(), time),
            utc,
            Disambiguation::Compatible,
        )
        .expect("BCE Gregorian zoned date-time should validate");

        let japanese_roundtrip = from_temporal_datetime(
            CalendarSystem::Japanese,
            &japanese_zoned.inner.to_plain_date_time(),
        )
        .expect("Japanese temporal roundtrip should succeed");

        let gregorian_roundtrip = from_temporal_datetime(
            CalendarSystem::Gregorian,
            &gregorian_zoned.inner.to_plain_date_time(),
        )
        .expect("Gregorian temporal roundtrip should succeed");

        assert_eq!(japanese_roundtrip.date(), &heisei);
        assert_eq!(gregorian_roundtrip.date(), &bce);
    }

    #[cfg(feature = "std")]
    #[test]
    fn time_and_date_time_cycles_cover_remaining_non_wrapping_and_date_wrapper_paths() {
        let time = Time::new(9, 30, 45, 125).expect("time should validate");

        let date_time = CalendarDateTime::new(gregorian_date(2024, 3, 15), time);

        assert_eq!(
            time.cycle(TimeField::Hour, 2, CycleTimeOptions { wrap: false })
                .expect("non-wrapping hour cycle should delegate to add")
                .hour(),
            11
        );
        assert_eq!(
            date_time
                .cycle(DateTimeField::Month, 1, CycleTimeOptions { wrap: true })
                .expect("wrapping month cycle should succeed")
                .date()
                .month(),
            4
        );
        assert_eq!(
            date_time
                .cycle(DateTimeField::Day, 20, CycleTimeOptions { wrap: true })
                .expect("wrapping day cycle should succeed")
                .date()
                .day(),
            4
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn system_time_conversion_helper_handles_negative_and_overflowing_instants() {
        use std::time::{Duration, UNIX_EPOCH};

        let negative = system_time_from_epoch_nanoseconds(-1_500_000_000)
            .expect("negative epoch conversion should succeed");

        assert_eq!(negative, UNIX_EPOCH - Duration::new(1, 500_000_000));

        let positive_overflow = system_time_from_epoch_nanoseconds(i128::MAX)
            .expect_err("maximum i128 instant should overflow");

        assert!(
            positive_overflow
                .to_string()
                .contains("instant exceeds SystemTime positive range")
        );

        let overflow = system_time_from_epoch_nanoseconds(i128::MIN)
            .expect_err("minimum i128 instant should overflow");

        assert!(
            overflow
                .to_string()
                .contains("instant exceeds SystemTime negative range")
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn today_helper_returns_a_date_in_the_requested_calendar() {
        let today = CalendarDate::today(CalendarSystem::Gregorian).expect("today should resolve");

        assert_eq!(today.calendar(), CalendarSystem::Gregorian);
    }

    #[test]
    fn provider_locale_semantics_cover_week_info_sensitive_calendar_helpers() {
        let provider = StubIcuProvider;

        let locale = Locale::parse("en-US-u-fw-mon").expect("test locale should parse");

        let date = gregorian_date(2024, 3, 10);

        let start = crate::calendar::queries::start_of_week(&date, &locale, &provider);

        let end = crate::calendar::queries::end_of_week(&date, &locale, &provider);

        assert_eq!(start.to_iso8601(), "2024-03-04");
        assert_eq!(end.to_iso8601(), "2024-03-10");
    }
}
