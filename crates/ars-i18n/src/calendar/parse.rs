//! Parsing and current-time helpers for calendar values.

use alloc::{format, string::ToString};
use core::str::FromStr;

#[cfg(feature = "std")]
use temporal_rs::{Instant, Temporal};
use temporal_rs::{PlainDate, PlainDateTime, PlainTime};

use super::{CalendarDate, CalendarDateTime, CalendarError, DateTimeDuration, Time};
#[cfg(feature = "std")]
use super::{CalendarSystem, TimeZoneId};

/// Parses a date string.
///
/// # Errors
///
/// Returns an error if `input` is not a valid Temporal date string.
pub fn parse_date(input: &str) -> Result<CalendarDate, CalendarError> {
    let parsed =
        PlainDate::from_str(input).map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let iso = parsed.with_calendar(temporal_rs::Calendar::ISO);

    CalendarDate::new_iso8601(iso.year(), iso.month(), iso.day())
}

/// Parses a date-time string.
///
/// # Errors
///
/// Returns an error if `input` is not a valid Temporal date-time string.
pub fn parse_date_time(input: &str) -> Result<CalendarDateTime, CalendarError> {
    let parsed = PlainDateTime::from_str(input)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let iso = parsed.with_calendar(temporal_rs::Calendar::ISO);

    Ok(CalendarDateTime::new(
        CalendarDate::new_iso8601(iso.year(), iso.month(), iso.day())?,
        Time::from_temporal(iso.to_plain_time()),
    ))
}

/// Parses a time string.
///
/// # Errors
///
/// Returns an error if `input` is not a valid Temporal time string.
pub fn parse_time(input: &str) -> Result<Time, CalendarError> {
    let parsed =
        PlainTime::from_str(input).map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    Ok(Time::from_temporal(parsed))
}

/// Parses a duration string.
///
/// # Errors
///
/// Returns an error if `input` is not a valid Temporal duration string.
pub fn parse_duration(input: &str) -> Result<DateTimeDuration, CalendarError> {
    let duration = temporal_rs::Duration::from_str(input)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    Ok(DateTimeDuration {
        date: super::DateDuration {
            years: try_duration_component("years", duration.years())?,
            months: try_duration_component("months", duration.months())?,
            weeks: try_duration_component("weeks", duration.weeks())?,
            days: try_duration_component("days", duration.days())?,
        },
        time: super::TimeDuration {
            hours: duration.hours(),
            minutes: duration.minutes(),
            seconds: duration.seconds(),
            milliseconds: duration.milliseconds(),
            microseconds: try_duration_component("microseconds", duration.microseconds())?,
            nanoseconds: try_duration_component("nanoseconds", duration.nanoseconds())?,
        },
    })
}

fn try_duration_component<T, U>(name: &str, value: T) -> Result<U, CalendarError>
where
    T: Copy + ToString,
    U: TryFrom<T>,
{
    U::try_from(value).map_err(|_| {
        CalendarError::Arithmetic(format!(
            "duration {name} component is out of range: {}",
            value.to_string()
        ))
    })
}

/// Parses a zoned date-time string.
///
/// # Errors
///
/// Returns an error if `input` is not a valid zoned date-time string or if
/// the parsed calendar or time-zone data cannot be represented publicly.
#[cfg(feature = "std")]
pub fn parse_zoned_date_time(input: &str) -> Result<super::ZonedDateTime, CalendarError> {
    let parsed = temporal_rs::ZonedDateTime::from_utf8(
        input.as_bytes(),
        temporal_rs::options::Disambiguation::Compatible,
        temporal_rs::options::OffsetDisambiguation::Reject,
    )
    .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let calendar = CalendarSystem::from_bcp47(parsed.calendar().identifier())
        .unwrap_or(CalendarSystem::Iso8601);

    let time_zone = TimeZoneId::new(
        parsed
            .time_zone()
            .identifier()
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
    )?;

    Ok(super::ZonedDateTime {
        inner: parsed,
        calendar,
        time_zone,
    })
}

/// Parses an absolute timestamp and projects it into the requested time zone.
///
/// # Errors
///
/// Returns an error if `input` is not a valid absolute timestamp or if the
/// requested time zone cannot be resolved.
#[cfg(feature = "std")]
pub fn parse_absolute(
    input: &str,
    time_zone: &TimeZoneId,
) -> Result<super::ZonedDateTime, CalendarError> {
    let instant =
        Instant::from_str(input).map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let inner = instant
        .to_zoned_date_time_iso(time_zone.to_temporal()?)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let time_zone = TimeZoneId::new(
        inner
            .time_zone()
            .identifier()
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
    )?;

    Ok(super::ZonedDateTime {
        inner,
        calendar: CalendarSystem::Iso8601,
        time_zone,
    })
}

/// Parses an absolute timestamp into the current local time zone.
///
/// # Errors
///
/// Returns an error if the local time zone cannot be resolved or if the input
/// is not a valid absolute timestamp.
#[cfg(feature = "std")]
pub fn parse_absolute_to_local(input: &str) -> Result<super::ZonedDateTime, CalendarError> {
    parse_absolute(input, &get_local_time_zone()?)
}

/// Returns the current zoned date-time.
///
/// # Errors
///
/// Returns an error if the current instant or requested time zone cannot be
/// resolved.
#[cfg(feature = "std")]
pub fn now(time_zone: Option<&TimeZoneId>) -> Result<super::ZonedDateTime, CalendarError> {
    let time_zone = time_zone.cloned().unwrap_or(current_local_time_zone()?);

    let zone = time_zone.to_temporal()?;

    let instant = Temporal::utc_now()
        .instant()
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let inner = temporal_rs::ZonedDateTime::try_new_iso_from_instant(instant, zone)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    let time_zone = TimeZoneId::new(
        inner
            .time_zone()
            .identifier()
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
    )?;

    Ok(super::ZonedDateTime {
        inner,
        calendar: CalendarSystem::Iso8601,
        time_zone,
    })
}

/// Returns today's date in the requested or local time zone.
///
/// # Errors
///
/// Returns an error if the current instant or requested time zone cannot be
/// resolved.
#[cfg(feature = "std")]
pub fn today(time_zone: Option<&TimeZoneId>) -> Result<CalendarDate, CalendarError> {
    let now = now(time_zone)?;

    let date = now
        .inner
        .to_plain_date()
        .with_calendar(temporal_rs::Calendar::ISO);

    CalendarDate::new_iso8601(date.year(), date.month(), date.day())
}

#[cfg(feature = "std")]
pub(crate) fn today_for_calendar(calendar: CalendarSystem) -> Result<CalendarDate, CalendarError> {
    today(None)?
        .to_calendar(calendar)
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))
}

/// Returns the local time-zone identifier.
///
/// # Errors
///
/// Returns an error if the local time zone cannot be resolved or the override
/// lock is poisoned.
#[cfg(feature = "std")]
pub fn get_local_time_zone() -> Result<TimeZoneId, CalendarError> {
    if let Some(time_zone) = local_time_zone_override()
        .lock()
        .map_err(|_| CalendarError::Arithmetic(String::from("local time-zone override poisoned")))?
        .clone()
    {
        return Ok(time_zone);
    }

    let zone = Temporal::local_now()
        .time_zone()
        .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

    TimeZoneId::new(
        zone.identifier()
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
    )
}

/// Sets a process-local time-zone override.
#[cfg(feature = "std")]
pub fn set_local_time_zone_override(time_zone: TimeZoneId) {
    if let Ok(mut guard) = local_time_zone_override().lock() {
        *guard = Some(time_zone);
    }
}

/// Clears a process-local time-zone override.
#[cfg(feature = "std")]
pub fn reset_local_time_zone_override() {
    if let Ok(mut guard) = local_time_zone_override().lock() {
        *guard = None;
    }
}

#[cfg(feature = "std")]
fn current_local_time_zone() -> Result<TimeZoneId, CalendarError> {
    get_local_time_zone()
}

#[cfg(feature = "std")]
fn local_time_zone_override() -> &'static std::sync::Mutex<Option<TimeZoneId>> {
    static LOCAL_TIME_ZONE_OVERRIDE: std::sync::OnceLock<std::sync::Mutex<Option<TimeZoneId>>> =
        std::sync::OnceLock::new();

    LOCAL_TIME_ZONE_OVERRIDE.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_helpers_reject_invalid_inputs() {
        assert!(parse_date("2024-02-30").is_err());
        assert!(parse_date_time("2024-03-15T25:00").is_err());
        assert!(parse_time("25:61").is_err());
        assert!(parse_duration("P-not-a-duration").is_err());
        assert!(parse_duration("P2147483648Y").is_err());
    }

    #[test]
    fn parsing_helpers_accept_fractional_time_and_duration_components() {
        let time = parse_time("12:34:56.789123456").expect("time should parse");

        let duration = parse_duration("P1Y2M3W4DT5H6M7.008009010S").expect("duration should parse");

        assert_eq!((time.hour(), time.minute(), time.second()), (12, 34, 56));
        assert_eq!(time.millisecond(), 789);
        assert_eq!(time.microsecond(), 123);
        assert_eq!(time.nanosecond(), 456);
        assert_eq!(duration.date.weeks, 3);
        assert_eq!(duration.time.hours, 5);
        assert_eq!(duration.time.microseconds, 9);
        assert_eq!(duration.time.nanoseconds, 10);
    }

    #[cfg(feature = "std")]
    #[test]
    fn zoned_and_absolute_parsing_reject_invalid_inputs() {
        let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

        assert!(parse_zoned_date_time("2024-03-10T01:30").is_err());
        assert!(parse_absolute("2024-03-10T07:45:00", &time_zone).is_err());
        assert!(parse_absolute_to_local("not-a-timestamp").is_err());
    }

    #[cfg(feature = "std")]
    #[test]
    fn current_time_helpers_accept_explicit_time_zones_and_calendar_projection() {
        let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

        let now = now(Some(&time_zone)).expect("now should resolve in explicit time zone");

        let today = today(Some(&time_zone)).expect("today should resolve in explicit time zone");

        let japanese_today = today_for_calendar(CalendarSystem::Japanese)
            .expect("calendar-specific today should project into the target calendar");

        assert_eq!(now.time_zone().as_str(), "America/New_York");
        assert_eq!(today.calendar(), CalendarSystem::Iso8601);
        assert_eq!(japanese_today.calendar(), CalendarSystem::Japanese);
    }
}
