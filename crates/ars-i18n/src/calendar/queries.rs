//! Query helpers for date-like values.

use super::{CalendarDate, DateValue};
#[cfg(feature = "std")]
use super::{CalendarDateTime, DateDuration, Disambiguation, Time, TimeZoneId};
use crate::{CalendarSystem, IntlBackend, Locale, Weekday};

/// Returns `true` when two values represent the same day in the first value's calendar.
#[must_use]
pub fn is_same_day(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    convert_for_comparison(&left, &b.date_value()).is_some_and(|right| {
        left.era().map(|value| value.code.as_str()) == right.era().map(|value| value.code.as_str())
            && left.year() == right.year()
            && left.month() == right.month()
            && left.day() == right.day()
    })
}

/// Returns `true` when two values share the same month in the first value's calendar.
#[must_use]
pub fn is_same_month(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    convert_for_comparison(&left, &b.date_value()).is_some_and(|right| {
        left.era().map(|value| value.code.as_str()) == right.era().map(|value| value.code.as_str())
            && left.year() == right.year()
            && left.month() == right.month()
    })
}

/// Returns `true` when two values share the same year in the first value's calendar.
#[must_use]
pub fn is_same_year(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    convert_for_comparison(&left, &b.date_value()).is_some_and(|right| {
        left.era().map(|value| value.code.as_str()) == right.era().map(|value| value.code.as_str())
            && left.year() == right.year()
    })
}

/// Returns `true` when two values represent the same day in the same calendar and era.
#[must_use]
pub fn is_equal_day(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    let right = b.date_value();

    has_same_calendar_identity(&left, &right)
        && left.year() == right.year()
        && left.month() == right.month()
        && left.day() == right.day()
}

/// Returns `true` when two values represent the same month in the same calendar and era.
#[must_use]
pub fn is_equal_month(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    let right = b.date_value();

    has_same_calendar_identity(&left, &right)
        && left.year() == right.year()
        && left.month() == right.month()
}

/// Returns `true` when two values represent the same year in the same calendar and era.
#[must_use]
pub fn is_equal_year(a: &impl DateValue, b: &impl DateValue) -> bool {
    let left = a.date_value();

    let right = b.date_value();

    has_same_calendar_identity(&left, &right) && left.year() == right.year()
}

/// Returns `true` when the given date is today in the provided time zone.
#[cfg(feature = "std")]
#[must_use]
pub fn is_today(date: &impl DateValue, time_zone: &TimeZoneId) -> bool {
    super::parse::today(Some(time_zone)).is_ok_and(|today| is_same_day(&date.date_value(), &today))
}

/// Returns the first day of the month.
#[must_use]
pub fn start_of_month<T: DateValue>(date: &T) -> T {
    let current = date.date_value();

    let next = current
        .set(&super::CalendarDateFields {
            day: Some(current.minimum_day_in_month()),
            ..super::CalendarDateFields::default()
        })
        .unwrap_or(current.clone());

    date.with_date_value(next)
}

/// Returns the last day of the month.
#[must_use]
pub fn end_of_month<T: DateValue>(date: &T) -> T {
    let current = date.date_value();

    let next = current
        .set(&super::CalendarDateFields {
            day: Some(current.days_in_month()),
            ..super::CalendarDateFields::default()
        })
        .unwrap_or(current.clone());

    date.with_date_value(next)
}

/// Returns the first day of the year.
#[must_use]
pub fn start_of_year<T: DateValue>(date: &T) -> T {
    let current = date.date_value();

    let start_month = current.minimum_month_in_year();

    let next = current
        .set(&super::CalendarDateFields {
            month: Some(start_month),
            ..super::CalendarDateFields::default()
        })
        .and_then(|value| {
            value.set(&super::CalendarDateFields {
                day: Some(value.minimum_day_in_month()),
                ..super::CalendarDateFields::default()
            })
        })
        .unwrap_or(current.clone());

    date.with_date_value(next)
}

/// Returns the last day of the year.
#[must_use]
pub fn end_of_year<T: DateValue>(date: &T) -> T {
    let current = date.date_value();

    let next = current
        .set(&super::CalendarDateFields {
            month: Some(current.calendar().months_in_year(&current)),
            ..super::CalendarDateFields::default()
        })
        .and_then(|value| {
            value.set(&super::CalendarDateFields {
                day: Some(value.days_in_month()),
                ..super::CalendarDateFields::default()
            })
        })
        .unwrap_or(current.clone());

    date.with_date_value(next)
}

/// Returns the first day of the locale week containing the date.
#[must_use]
pub fn start_of_week<T: DateValue>(date: &T, locale: &Locale, backend: &dyn IntlBackend) -> T {
    let current = date.date_value();

    let first_day = backend.first_day_of_week(locale);

    let delta = weekday_index(current.weekday()) - weekday_index(first_day);

    let next = current
        .add_days(-delta.rem_euclid(7))
        .unwrap_or(current.clone());

    date.with_date_value(next)
}

/// Returns the last day of the locale week containing the date.
#[must_use]
pub fn end_of_week<T: DateValue>(date: &T, locale: &Locale, backend: &dyn IntlBackend) -> T {
    let start = start_of_week(date, locale, backend).date_value();

    let next = start.add_days(6).unwrap_or(start.clone());

    date.with_date_value(next)
}

/// Returns the weekday index relative to the locale's first day of week.
#[must_use]
pub fn get_day_of_week(date: &CalendarDate, locale: &Locale, backend: &dyn IntlBackend) -> u8 {
    let first_day = backend.week_info(locale).first_day;

    let offset = (weekday_index(date.weekday()) - weekday_index(first_day)).rem_euclid(7);

    u8::try_from(offset).expect("weekday index is always in the 0..=6 range")
}

/// Returns the number of calendar weeks that intersect the month.
#[must_use]
pub fn get_weeks_in_month(date: &CalendarDate, locale: &Locale, backend: &dyn IntlBackend) -> u8 {
    let month_start = start_of_month(date);

    let month_end = end_of_month(date);

    let first_week_start = start_of_week(&month_start, locale, backend).date_value();

    let last_week_end = end_of_week(&month_end, locale, backend).date_value();

    first_week_start
        .days_until(&last_week_end)
        .ok()
        .and_then(|days| u8::try_from((days + 1) / 7).ok())
        .unwrap_or(5)
}

/// Returns the number of hours in the given day and time zone.
///
/// # Errors
///
/// Returns an error when the calendar engine cannot resolve the day length for
/// the supplied date and time zone.
#[cfg(feature = "std")]
pub fn get_hours_in_day(
    date: &CalendarDate,
    time_zone: &TimeZoneId,
) -> Result<u8, super::CalendarError> {
    let midnight = Time::new(0, 0, 0, 0)
        .map_err(|error| super::CalendarError::Arithmetic(error.to_string()))?;

    let start_local = CalendarDateTime::new(date.clone(), midnight);
    let start =
        super::ZonedDateTime::new(&start_local, time_zone.clone(), Disambiguation::Compatible)?;

    let next_day = date.add(DateDuration {
        days: 1,
        ..DateDuration::default()
    })?;

    let end_local = CalendarDateTime::new(next_day, midnight);
    let end = super::ZonedDateTime::new(&end_local, time_zone.clone(), Disambiguation::Compatible)?;

    let diff = end.inner.epoch_nanoseconds().as_i128() - start.inner.epoch_nanoseconds().as_i128();

    let hours = diff / 3_600_000_000_000_i128;

    u8::try_from(hours).map_err(|_| {
        super::CalendarError::Arithmetic(String::from("day length exceeded supported hour range"))
    })
}

/// Returns the earlier of two dates.
#[must_use]
pub fn min_date<T: DateValue>(a: &T, b: &T) -> T {
    if a.date_value().compare(&b.date_value()).is_le() {
        a.clone()
    } else {
        b.clone()
    }
}

/// Returns the later of two dates.
#[must_use]
pub fn max_date<T: DateValue>(a: &T, b: &T) -> T {
    if a.date_value().compare(&b.date_value()).is_ge() {
        a.clone()
    } else {
        b.clone()
    }
}

/// Returns `true` when the given date falls on the locale weekend.
#[must_use]
pub fn is_weekend(date: &CalendarDate, locale: &Locale, backend: &dyn IntlBackend) -> bool {
    let week_info = backend.week_info(locale);

    let day = weekday_index(date.weekday());

    let start = weekday_index(week_info.weekend_start);

    let end = weekday_index(week_info.weekend_end);

    if start <= end {
        (start..=end).contains(&day)
    } else {
        day >= start || day <= end
    }
}

/// Returns `true` when the given date is not on the locale weekend.
#[must_use]
pub fn is_weekday(date: &CalendarDate, locale: &Locale, backend: &dyn IntlBackend) -> bool {
    !is_weekend(date, locale, backend)
}

const fn weekday_index(value: Weekday) -> i32 {
    match value {
        Weekday::Monday => 1,
        Weekday::Tuesday => 2,
        Weekday::Wednesday => 3,
        Weekday::Thursday => 4,
        Weekday::Friday => 5,
        Weekday::Saturday => 6,
        Weekday::Sunday => 7,
    }
}

fn has_same_calendar_identity(left: &CalendarDate, right: &CalendarDate) -> bool {
    left.calendar() == right.calendar()
        && left.era().map(|value| value.code.as_str())
            == right.era().map(|value| value.code.as_str())
}

fn convert_for_comparison(left: &CalendarDate, right: &CalendarDate) -> Option<CalendarDate> {
    if left.calendar() == right.calendar() {
        Some(right.clone())
    } else {
        convert_date(right, left.calendar())
    }
}

fn convert_date(value: &CalendarDate, target: CalendarSystem) -> Option<CalendarDate> {
    value.to_calendar(target).ok()
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;
    #[cfg(feature = "std")]
    use crate::TimeZoneId;
    use crate::{CalendarDateFields, CalendarDateTime, Era, StubIntlBackend, Time};

    struct WrapWeekendProvider;

    impl IntlBackend for WrapWeekendProvider {
        fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_short_label(weekday, locale)
        }

        fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String {
            StubIntlBackend.weekday_long_label(weekday, locale)
        }

        fn month_long_name(&self, month: u8, locale: &Locale) -> String {
            StubIntlBackend.month_long_name(month, locale)
        }

        fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
            StubIntlBackend.day_period_label(is_pm, locale)
        }

        fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
            StubIntlBackend.day_period_from_char(ch, locale)
        }

        fn format_segment_digits(
            &self,
            value: u32,
            min_digits: core::num::NonZero<u8>,
            locale: &Locale,
        ) -> String {
            StubIntlBackend.format_segment_digits(value, min_digits, locale)
        }

        fn hour_cycle(&self, locale: &Locale) -> crate::HourCycle {
            StubIntlBackend.hour_cycle(locale)
        }

        fn week_info(&self, _locale: &Locale) -> crate::WeekInfo {
            crate::WeekInfo {
                first_day: Weekday::Monday,
                weekend_start: Weekday::Sunday,
                weekend_end: Weekday::Friday,
                minimal_days_in_first_week: 1,
            }
        }
    }

    fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("Gregorian fixture should validate")
    }

    #[test]
    fn comparison_helpers_cover_false_paths_and_private_conversion_helpers() {
        let gregorian = gregorian_date(2024, 3, 15);

        let japanese = gregorian
            .to_calendar(CalendarSystem::Japanese)
            .expect("Japanese conversion should succeed");

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

        assert!(!is_same_day(&gregorian, &bc));
        assert!(!is_equal_year(&gregorian, &japanese));
        assert!(!has_same_calendar_identity(&gregorian, &japanese));
        assert!(!has_same_calendar_identity(&gregorian, &bc));
        assert_eq!(
            convert_for_comparison(&japanese, &gregorian)
                .expect("comparison conversion should succeed")
                .calendar(),
            CalendarSystem::Japanese
        );
        assert_eq!(
            convert_date(&gregorian, CalendarSystem::IslamicUmmAlQura)
                .expect("calendar conversion helper should succeed")
                .calendar(),
            CalendarSystem::IslamicUmmAlQura
        );
    }

    #[test]
    fn month_and_year_boundary_helpers_cover_non_gregorian_minimums() {
        let japanese = CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("reiwa"),
                    display_name: String::from("Reiwa"),
                }),
                year: Some(1),
                month: Some(7),
                day: Some(20),
                ..CalendarDateFields::default()
            },
        )
        .expect("Japanese date should validate");

        let start = start_of_year(&japanese);
        let end = end_of_year(&japanese);

        let month_start = start_of_month(&japanese);
        let month_end = end_of_month(&japanese);

        assert_eq!(start.month(), 5);
        assert_eq!(start.day(), 1);
        assert_eq!(end.month(), 12);
        assert_eq!(end.day(), 31);
        assert_eq!(month_start.day(), 1);
        assert_eq!(month_end.day(), 31);
    }

    #[test]
    fn week_helpers_preserve_time_components_and_cover_weekend_wrap_logic() {
        let backend = StubIntlBackend;

        let locale = Locale::parse("ar-AE").expect("locale should parse");

        let date_time = CalendarDateTime::new(
            gregorian_date(2024, 3, 15),
            Time::new(9, 30, 0, 0).expect("time should validate"),
        );

        let start = start_of_week(&date_time, &locale, &backend);
        let end = end_of_week(&date_time, &locale, &backend);

        assert_eq!(start.time().hour(), 9);
        assert_eq!(start.time().minute(), 30);
        assert_eq!(end.time().hour(), 9);
        assert!(is_weekend(&gregorian_date(2024, 3, 15), &locale, &backend));
        assert!(is_weekend(&gregorian_date(2024, 3, 16), &locale, &backend));
        assert!(!is_weekday(&gregorian_date(2024, 3, 15), &locale, &backend));
    }

    #[cfg(feature = "std")]
    #[test]
    fn today_and_min_max_helpers_cover_remaining_branches() {
        let time_zone = TimeZoneId::new("UTC").expect("UTC should validate");

        let far_future = gregorian_date(2099, 1, 1);
        let today = crate::calendar::parse::today(Some(&time_zone))
            .expect("Gregorian today should resolve for the requested time zone");

        let earlier = gregorian_date(2024, 3, 10);

        let later = gregorian_date(2024, 3, 15);

        assert!(is_today(&today, &time_zone));
        assert!(!is_today(&far_future, &time_zone));
        assert_eq!(min_date(&later, &earlier), earlier);
        assert_eq!(max_date(&earlier, &later), later);
    }

    #[test]
    fn comparison_and_weekend_helpers_cover_remaining_branch_shapes() {
        let backend = StubIntlBackend;

        let wrap_provider = WrapWeekendProvider;

        let locale = Locale::parse("en-US").expect("locale should parse");

        let same_day = gregorian_date(2024, 3, 15);

        let other_day = gregorian_date(2024, 3, 16);

        let other_month = gregorian_date(2024, 4, 15);

        let other_year = gregorian_date(2025, 3, 15);

        let same_day_time = CalendarDateTime::new(
            same_day.clone(),
            Time::new(9, 30, 0, 0).expect("time should validate"),
        );

        assert!(is_equal_day(&same_day, &same_day));
        assert!(is_equal_month(&same_day, &same_day_time));
        assert!(is_equal_year(&same_day, &same_day_time));
        assert!(!is_equal_day(&same_day, &other_day));
        assert!(!is_equal_month(&same_day, &other_month));
        assert!(!is_equal_year(&same_day, &other_year));

        assert_eq!(min_date(&same_day, &same_day).to_iso8601(), "2024-03-15");
        assert_eq!(max_date(&same_day, &same_day).to_iso8601(), "2024-03-15");

        assert_eq!(get_day_of_week(&same_day, &locale, &backend), 5);
        assert!(is_weekend(
            &gregorian_date(2024, 3, 10),
            &locale,
            &wrap_provider
        ));
        assert!(is_weekend(
            &gregorian_date(2024, 3, 14),
            &locale,
            &wrap_provider
        ));
        assert!(!is_weekend(
            &gregorian_date(2024, 3, 16),
            &locale,
            &wrap_provider
        ));
        assert!(!is_weekday(
            &gregorian_date(2024, 3, 10),
            &locale,
            &wrap_provider
        ));
        assert!(is_weekday(
            &gregorian_date(2024, 3, 16),
            &locale,
            &wrap_provider
        ));
    }
}
