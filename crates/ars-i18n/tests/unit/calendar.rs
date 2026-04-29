use alloc::vec;

#[cfg(feature = "std")]
use temporal_rs::sys::Temporal;

use super::{
    CalendarDate, CalendarDateFields, CalendarDateTime, CalendarError, CalendarSystem,
    CycleOptions, CycleTimeOptions, DateDuration, DateError, DateField, DateTimeDuration,
    DateTimeField, Era, MonthCode, Time, TimeDuration, TimeField, TimeFields, parse, queries,
};
#[cfg(feature = "std")]
use super::{
    Disambiguation, TimeZoneId, ZonedDateTime, to_calendar_date_time, to_zoned, to_zoned_date_time,
};
use crate::{Locale, StubIntlBackend};

fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("Gregorian fixture should validate")
}

fn japanese_era(code: &str, display_name: &str) -> Era {
    Era {
        code: code.into(),
        display_name: display_name.into(),
    }
}

#[cfg(feature = "std")]
fn local_time_zone_override_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("local time-zone override test lock should not be poisoned")
}

#[test]
fn calendar_system_bcp47_roundtrips_and_lists_supported_calendars() {
    for (identifier, calendar) in [
        ("iso8601", CalendarSystem::Iso8601),
        ("gregory", CalendarSystem::Gregorian),
        ("japanese", CalendarSystem::Japanese),
        ("hebrew", CalendarSystem::Hebrew),
        ("roc", CalendarSystem::Roc),
    ] {
        assert_eq!(CalendarSystem::from_bcp47(identifier), Some(calendar));
        assert_eq!(calendar.to_bcp47_value(), identifier);
    }

    assert!(
        CalendarSystem::supported_calendars()
            .iter()
            .any(|metadata| metadata.calendar == CalendarSystem::Iso8601)
    );
    assert!(
        CalendarSystem::supported_calendars()
            .iter()
            .any(|metadata| {
                metadata.calendar == CalendarSystem::Gregorian && metadata.has_custom_eras
            })
    );
}

#[test]
fn calendar_system_public_eras_cover_gregorian_and_japanese() {
    assert_eq!(
        CalendarSystem::Gregorian.eras(),
        vec![
            Era {
                code: "bc".into(),
                display_name: "BC".into(),
            },
            Era {
                code: "ad".into(),
                display_name: "AD".into(),
            },
        ]
    );
    assert_eq!(
        CalendarSystem::Gregorian.default_era(),
        Some(Era {
            code: "ad".into(),
            display_name: "AD".into(),
        })
    );

    let japanese = CalendarSystem::Japanese.eras();

    assert_eq!(
        japanese.first().map(|value| value.code.as_str()),
        Some("meiji")
    );
    assert_eq!(
        japanese.last().map(|value| value.code.as_str()),
        Some("reiwa")
    );
    assert_eq!(
        CalendarSystem::Japanese.default_era(),
        Some(Era {
            code: "reiwa".into(),
            display_name: "Reiwa".into(),
        })
    );
}

#[test]
fn calendar_date_supports_iso8601_and_gregorian_eras() {
    let iso = CalendarDate::new_iso8601(2024, 3, 15).expect("ISO date should validate");

    assert_eq!(iso.calendar(), CalendarSystem::Iso8601);
    assert_eq!(iso.to_iso8601(), "2024-03-15");
    assert_eq!(iso.era(), None);

    let ad = CalendarDate::new(
        CalendarSystem::Gregorian,
        &CalendarDateFields {
            era: Some(Era {
                code: "ad".into(),
                display_name: "AD".into(),
            }),
            year: Some(2024),
            month: Some(3),
            day: Some(15),
            ..CalendarDateFields::default()
        },
    )
    .expect("Gregorian AD date should validate");

    assert_eq!(ad.era().map(|value| value.code.as_str()), Some("ad"));
    assert_eq!(ad.to_iso8601(), "2024-03-15");

    let bc = CalendarDate::new(
        CalendarSystem::Gregorian,
        &CalendarDateFields {
            era: Some(Era {
                code: "bc".into(),
                display_name: "BC".into(),
            }),
            year: Some(1),
            month: Some(1),
            day: Some(1),
            ..CalendarDateFields::default()
        },
    )
    .expect("Gregorian BC date should validate");

    assert_eq!(bc.era().map(|value| value.code.as_str()), Some("bc"));
    assert_eq!(bc.to_iso8601(), "0000-01-01");
}

#[test]
fn calendar_date_validates_japanese_era_boundaries() {
    let heisei = japanese_era("heisei", "Heisei");

    let reiwa = japanese_era("reiwa", "Reiwa");

    assert!(
        CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(heisei.clone()),
                year: Some(1),
                month: Some(1),
                day: Some(8),
                ..CalendarDateFields::default()
            },
        )
        .is_ok()
    );
    assert!(
        CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(heisei),
                year: Some(1),
                month: Some(1),
                day: Some(7),
                ..CalendarDateFields::default()
            },
        )
        .is_err()
    );
    assert!(
        CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(reiwa.clone()),
                year: Some(1),
                month: Some(5),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .is_ok()
    );
    assert!(
        CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(reiwa),
                year: Some(1),
                month: Some(4),
                day: Some(30),
                ..CalendarDateFields::default()
            },
        )
        .is_err()
    );
}

#[test]
fn calendar_system_queries_cover_hebrew_leap_months_and_japanese_bounds() {
    let hebrew_leap = CalendarDate::new(
        CalendarSystem::Hebrew,
        &CalendarDateFields {
            year: Some(5784),
            month: Some(13),
            day: Some(1),
            ..CalendarDateFields::default()
        },
    )
    .expect("Hebrew leap-year month 13 should validate");

    let hebrew_common = CalendarDate::new(
        CalendarSystem::Hebrew,
        &CalendarDateFields {
            year: Some(5785),
            month: Some(12),
            day: Some(1),
            ..CalendarDateFields::default()
        },
    )
    .expect("Hebrew common-year month 12 should validate");

    assert_eq!(CalendarSystem::Hebrew.months_in_year(&hebrew_leap), 13);
    assert_eq!(CalendarSystem::Hebrew.months_in_year(&hebrew_common), 12);
    assert!(hebrew_leap.month_code().is_some());

    let reiwa = CalendarDate::new(
        CalendarSystem::Japanese,
        &CalendarDateFields {
            era: Some(japanese_era("reiwa", "Reiwa")),
            year: Some(1),
            month: Some(5),
            day: Some(1),
            ..CalendarDateFields::default()
        },
    )
    .expect("Reiwa start should validate");

    let heisei = CalendarDate::new(
        CalendarSystem::Japanese,
        &CalendarDateFields {
            era: Some(japanese_era("heisei", "Heisei")),
            year: Some(31),
            month: Some(4),
            day: Some(30),
            ..CalendarDateFields::default()
        },
    )
    .expect("Heisei end should validate");

    assert_eq!(reiwa.minimum_month_in_year(), 5);
    assert_eq!(reiwa.minimum_day_in_month(), 1);
    assert_eq!(heisei.years_in_era(), Some(31));
    assert_eq!(CalendarSystem::Japanese.months_in_year(&heisei), 4);
    assert_eq!(CalendarSystem::Japanese.days_in_month(&heisei), 30);
}

#[test]
fn calendar_date_arithmetic_and_cycle_follow_temporal_style_operations() {
    let date = gregorian_date(2024, 1, 31);

    let next_month = date
        .add(DateDuration {
            months: 1,
            ..DateDuration::default()
        })
        .expect("month addition should succeed");

    assert_eq!(next_month.to_iso8601(), "2024-02-29");

    let prior = next_month
        .subtract(DateDuration {
            days: 28,
            ..DateDuration::default()
        })
        .expect("day subtraction should succeed");

    assert_eq!(prior.to_iso8601(), "2024-02-01");

    let cycled = prior
        .cycle(DateField::Day, 30, CycleOptions { wrap: true })
        .expect("day cycle should wrap within the month");

    assert_eq!(cycled.day(), 2);
}

#[test]
fn calendar_date_converts_between_calendars() {
    let gregorian = gregorian_date(2024, 3, 15);

    let japanese = gregorian
        .to_calendar(CalendarSystem::Japanese)
        .expect("Gregorian to Japanese conversion should succeed");

    assert_eq!(japanese.calendar(), CalendarSystem::Japanese);
    assert_eq!(
        japanese.era().map(|value| value.code.as_str()),
        Some("reiwa")
    );
    assert_eq!(japanese.year(), 6);
    assert_eq!(japanese.month(), 3);
    assert_eq!(japanese.day(), 15);
    assert_eq!(
        japanese
            .to_calendar(CalendarSystem::Gregorian)
            .expect("round-trip Gregorian conversion should succeed"),
        gregorian
    );
}

#[test]
fn time_and_date_time_support_add_set_and_cycle() {
    let time = Time::new(23, 45, 10, 250).expect("time should validate");

    let rolled = time
        .add(TimeDuration {
            minutes: 30,
            ..TimeDuration::default()
        })
        .expect("time addition should succeed");

    assert_eq!(rolled.hour(), 0);
    assert_eq!(rolled.minute(), 15);

    let cycled = rolled
        .cycle(TimeField::Hour, -1, CycleTimeOptions { wrap: true })
        .expect("hour cycle should wrap");

    assert_eq!(cycled.hour(), 23);

    let date_time = CalendarDateTime::new(gregorian_date(2024, 3, 15), time);

    let next = date_time
        .add(DateTimeDuration {
            date: DateDuration {
                days: 1,
                ..DateDuration::default()
            },
            time: TimeDuration {
                minutes: 30,
                ..TimeDuration::default()
            },
        })
        .expect("date-time arithmetic should succeed");

    assert_eq!(next.date().to_iso8601(), "2024-03-17");
    assert_eq!(next.time().hour(), 0);
    assert_eq!(next.time().minute(), 15);

    let changed = next
        .set(
            &CalendarDateFields {
                day: Some(20),
                ..CalendarDateFields::default()
            },
            TimeFields {
                hour: Some(9),
                minute: Some(30),
                ..TimeFields::default()
            },
        )
        .expect("date-time set should succeed");

    assert_eq!(changed.date().day(), 20);
    assert_eq!(changed.time().hour(), 9);

    let cycled_dt = changed
        .cycle(DateTimeField::Month, 1, CycleTimeOptions { wrap: false })
        .expect("date-time month cycle should succeed");

    assert_eq!(cycled_dt.date().month(), 4);
}

#[test]
fn query_helpers_operate_on_calendar_dates_and_date_times() {
    let backend = StubIntlBackend;

    let locale = Locale::parse("en-US").expect("test locale should parse");

    let date = gregorian_date(2024, 3, 15);

    let time = Time::new(9, 30, 0, 0).expect("time should validate");

    let date_time = CalendarDateTime::new(date.clone(), time);

    assert!(queries::is_same_day(&date, &date_time));
    assert!(queries::is_same_month(&date, &date_time));
    assert!(queries::is_same_year(&date, &date_time));

    let month_start = queries::start_of_month(&date);

    let month_end = queries::end_of_month(&date);

    let year_start = queries::start_of_year(&date);

    let year_end = queries::end_of_year(&date);

    assert_eq!(month_start.to_iso8601(), "2024-03-01");
    assert_eq!(month_end.to_iso8601(), "2024-03-31");
    assert_eq!(year_start.to_iso8601(), "2024-01-01");
    assert_eq!(year_end.to_iso8601(), "2024-12-31");

    let week_start = queries::start_of_week(&date_time, &locale, &backend);

    let week_end = queries::end_of_week(&date_time, &locale, &backend);

    assert_eq!(week_start.date().to_iso8601(), "2024-03-10");
    assert_eq!(week_end.date().to_iso8601(), "2024-03-16");
    assert_eq!(week_start.time().hour(), 9);
    assert_eq!(queries::get_day_of_week(&date, &locale, &backend), 5);
    assert_eq!(queries::get_weeks_in_month(&date, &locale, &backend), 6);
    assert!(queries::is_weekday(&date, &locale, &backend));
    assert!(!queries::is_weekend(&date, &locale, &backend));
    assert_eq!(queries::min_date(&date, &month_end), date);
    assert_eq!(queries::max_date(&date, &month_end), month_end);
}

#[test]
fn query_helpers_follow_react_aria_calendar_and_locale_semantics() {
    let backend = StubIntlBackend;

    let en_us = Locale::parse("en-US").expect("test locale should parse");

    let fr_fr = Locale::parse("fr-FR").expect("test locale should parse");

    let he_il = Locale::parse("he-IL").expect("test locale should parse");

    let en_us_monday = Locale::parse("en-US-u-fw-mon").expect("test locale should parse");

    let gregorian = gregorian_date(2024, 3, 10);

    let islamic = gregorian
        .to_calendar(CalendarSystem::IslamicUmmAlQura)
        .expect("Gregorian to Umm al-Qura conversion should succeed");

    assert!(queries::is_same_month(&gregorian, &islamic));
    assert!(!queries::is_equal_month(&gregorian, &islamic));

    assert_eq!(queries::get_day_of_week(&gregorian, &en_us, &backend), 0);
    assert_eq!(queries::get_day_of_week(&gregorian, &fr_fr, &backend), 6);

    assert!(queries::is_weekend(&gregorian, &en_us, &backend));
    assert!(!queries::is_weekend(&gregorian, &he_il, &backend));
    assert!(queries::is_weekday(&gregorian, &he_il, &backend));
    assert_eq!(
        queries::start_of_week(&gregorian, &en_us_monday, &backend).to_iso8601(),
        "2024-03-04"
    );
    assert_eq!(
        queries::get_day_of_week(&gregorian, &en_us_monday, &backend),
        6
    );

    let march = gregorian_date(2024, 3, 15);

    assert_eq!(queries::get_weeks_in_month(&march, &en_us, &backend), 6);
    assert_eq!(queries::get_weeks_in_month(&march, &fr_fr, &backend), 5);
    assert_eq!(
        queries::get_weeks_in_month(&march, &en_us_monday, &backend),
        5
    );
}

#[test]
fn parsing_helpers_cover_date_time_duration_and_month_code() {
    let date = parse::parse_date("2024-03-15").expect("date parse should succeed");

    assert_eq!(date.to_iso8601(), "2024-03-15");

    let time = parse::parse_time("23:45:10.250").expect("time parse should succeed");

    assert_eq!(time.hour(), 23);
    assert_eq!(time.minute(), 45);
    assert_eq!(time.second(), 10);
    assert_eq!(time.millisecond(), 250);

    let date_time =
        parse::parse_date_time("2024-03-15T23:45:10.250").expect("date-time parse should succeed");

    assert_eq!(date_time.date().to_iso8601(), "2024-03-15");
    assert_eq!(date_time.time().hour(), 23);

    let duration =
        parse::parse_duration("P1Y2M3DT4H5M6.007008009S").expect("duration parse should succeed");

    assert_eq!(duration.date.years, 1);
    assert_eq!(duration.date.months, 2);
    assert_eq!(duration.date.days, 3);
    assert_eq!(duration.time.hours, 4);
    assert_eq!(duration.time.minutes, 5);
    assert_eq!(duration.time.seconds, 6);

    let month_code = MonthCode::new("M05L").expect("month code should validate");

    assert_eq!(month_code.as_str(), "M05L");
    assert!(month_code.is_leap_month());
}

#[cfg(feature = "std")]
#[test]
fn conversion_helpers_cover_common_date_and_zone_conversions() {
    let date = gregorian_date(2024, 3, 15);

    let time = Time::new(9, 30, 0, 0).expect("time should validate");

    let midnight = to_calendar_date_time(&date, None);

    assert_eq!(midnight.date(), &date);
    assert_eq!(midnight.time().hour(), 0);
    assert_eq!(midnight.time().minute(), 0);

    let preserved = to_calendar_date_time(&date, Some(time));

    assert_eq!(preserved.date(), &date);
    assert_eq!(preserved.time(), &time);

    let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

    let zoned = to_zoned(&date, &time_zone).expect("date to zoned conversion should succeed");

    let zoned_date_time = zoned.inner.to_plain_date_time();

    assert_eq!(zoned.time_zone(), &time_zone);
    assert_eq!(zoned_date_time.year(), 2024);
    assert_eq!(zoned_date_time.month(), 3);
    assert_eq!(zoned_date_time.day(), 15);
    assert_eq!(zoned_date_time.hour(), 0);
    assert_eq!(zoned_date_time.minute(), 0);

    let skipped = CalendarDateTime::new(
        gregorian_date(2024, 3, 10),
        Time::new(2, 30, 0, 0).expect("time should validate"),
    );

    assert!(
        to_zoned_date_time(&skipped, &time_zone, Disambiguation::Reject).is_err(),
        "reject should fail for skipped local times"
    );
    assert!(
        to_zoned_date_time(&skipped, &time_zone, Disambiguation::Compatible).is_ok(),
        "compatible should resolve skipped local times"
    );
}

#[cfg(feature = "std")]
#[test]
fn native_interop_helpers_convert_to_system_time_without_exposing_temporal_types() {
    use std::time::{Duration, UNIX_EPOCH};

    let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

    let date = gregorian_date(2024, 3, 15);

    let date_system_time = date
        .to_system_time(&time_zone)
        .expect("date system time conversion should succeed");

    assert_eq!(
        date_system_time,
        UNIX_EPOCH + Duration::from_secs(1_710_475_200)
    );

    let skipped = CalendarDateTime::new(
        gregorian_date(2024, 3, 10),
        Time::new(2, 30, 0, 0).expect("time should validate"),
    );

    assert!(
        skipped
            .to_system_time(&time_zone, Disambiguation::Reject)
            .is_err(),
        "reject should fail for skipped local times"
    );
    assert_eq!(
        skipped
            .to_system_time(&time_zone, Disambiguation::Compatible)
            .expect("compatible should resolve skipped local times"),
        UNIX_EPOCH + Duration::from_secs(1_710_055_800)
    );

    let zoned = parse::parse_absolute("2024-03-10T07:45:00Z", &time_zone)
        .expect("absolute parse should succeed");

    assert_eq!(
        zoned
            .to_system_time()
            .expect("zoned system time conversion should succeed"),
        UNIX_EPOCH + Duration::from_secs(1_710_056_700)
    );
}

#[cfg(feature = "std")]
#[test]
fn zoned_date_time_and_local_zone_override_work() {
    let _guard = local_time_zone_override_test_lock();
    let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

    parse::reset_local_time_zone_override();
    let local = CalendarDateTime::new(
        gregorian_date(2024, 3, 10),
        Time::new(1, 30, 0, 0).expect("time should validate"),
    );

    let zoned = ZonedDateTime::new(&local, time_zone.clone(), Disambiguation::Compatible)
        .expect("zoned date-time construction should succeed");

    assert_eq!(zoned.time_zone(), &time_zone);

    let reparsed = parse::parse_zoned_date_time("2024-03-10T01:30-05:00[America/New_York]")
        .expect("zoned parse should succeed");

    assert_eq!(reparsed.time_zone().as_str(), "America/New_York");

    parse::set_local_time_zone_override(time_zone.clone());

    assert_eq!(
        parse::get_local_time_zone()
            .expect("local override should read back")
            .as_str(),
        "America/New_York"
    );
    assert_eq!(
        parse::now(None)
            .expect("now() should honor the override")
            .time_zone()
            .as_str(),
        "America/New_York"
    );

    let absolute = parse::parse_absolute("2024-03-10T07:45:00Z", &time_zone)
        .expect("absolute parse should succeed");

    let absolute_date_time = absolute.inner.to_plain_date_time();

    assert_eq!(absolute.time_zone().as_str(), "America/New_York");
    assert_eq!(absolute_date_time.year(), 2024);
    assert_eq!(absolute_date_time.month(), 3);
    assert_eq!(absolute_date_time.day(), 10);
    assert_eq!(absolute_date_time.hour(), 3);
    assert_eq!(absolute_date_time.minute(), 45);

    let converted = parse::parse_absolute(
        "2024-03-10T07:45:00-05:00",
        &TimeZoneId::new("America/Los_Angeles").expect("zone should validate"),
    )
    .expect("offset absolute parse should succeed");

    let converted_date_time = converted.inner.to_plain_date_time();

    assert_eq!(converted.time_zone().as_str(), "America/Los_Angeles");
    assert_eq!(converted_date_time.year(), 2024);
    assert_eq!(converted_date_time.month(), 3);
    assert_eq!(converted_date_time.day(), 10);
    assert_eq!(converted_date_time.hour(), 5);
    assert_eq!(converted_date_time.minute(), 45);

    let local = parse::parse_absolute_to_local("2024-03-10T07:45:00Z")
        .expect("local absolute parse should use the override");

    assert_eq!(local.time_zone().as_str(), "America/New_York");

    assert!(parse::parse_absolute("not-a-timestamp", &time_zone).is_err());

    parse::reset_local_time_zone_override();
}

#[cfg(feature = "std")]
#[test]
fn get_local_time_zone_uses_local_host_zone_when_no_override_is_set() {
    let _guard = local_time_zone_override_test_lock();

    parse::reset_local_time_zone_override();

    let expected = TimeZoneId::new(
        Temporal::local_now()
            .time_zone()
            .expect("local host zone should resolve")
            .identifier()
            .expect("local host zone should expose an identifier"),
    )
    .expect("local host zone identifier should validate");

    assert_eq!(
        parse::get_local_time_zone()
            .expect("local time zone should resolve without override")
            .as_str(),
        expected.as_str()
    );
}

#[cfg(feature = "std")]
#[test]
fn get_hours_in_day_tracks_dst_boundaries() {
    let time_zone = TimeZoneId::new("America/New_York").expect("zone should validate");

    let spring_forward =
        queries::get_hours_in_day(&gregorian_date(2024, 3, 10), &time_zone).expect("DST day");

    let fall_back =
        queries::get_hours_in_day(&gregorian_date(2024, 11, 3), &time_zone).expect("DST day");

    let normal_day =
        queries::get_hours_in_day(&gregorian_date(2024, 3, 11), &time_zone).expect("normal day");

    assert_eq!(spring_forward, 23);
    assert_eq!(fall_back, 25);
    assert_eq!(normal_day, 24);
}

#[test]
fn cross_calendar_conversion_supports_every_supported_calendar_system() {
    // Every variant exercises a distinct arm of `temporal_calendar_for`.
    let gregorian = gregorian_date(2024, 3, 15);

    for target in [
        CalendarSystem::Iso8601,
        CalendarSystem::Gregorian,
        CalendarSystem::Buddhist,
        CalendarSystem::Japanese,
        CalendarSystem::Hebrew,
        CalendarSystem::IslamicCivil,
        CalendarSystem::IslamicUmmAlQura,
        CalendarSystem::Persian,
        CalendarSystem::Indian,
        CalendarSystem::Chinese,
        CalendarSystem::Coptic,
        CalendarSystem::Dangi,
        CalendarSystem::Ethiopic,
        CalendarSystem::EthiopicAmeteAlem,
        CalendarSystem::Roc,
    ] {
        let converted = gregorian
            .to_calendar(target)
            .expect("cross-calendar conversion should succeed");

        assert_eq!(converted.calendar(), target);
        // ISO projection is preserved regardless of display calendar.
        assert_eq!(converted.to_iso8601(), "2024-03-15");
    }
}

#[test]
fn date_duration_to_temporal_errors_when_temporal_limits_are_exceeded() {
    let date = gregorian_date(2024, 3, 15);

    // Saturating every field with `i32::MAX` exceeds Temporal's total-duration
    // magnitude limit; the resulting error surfaces through
    // `DateDuration::to_temporal`'s `.map_err` closure.
    let saturated = DateDuration {
        years: i32::MAX,
        months: i32::MAX,
        weeks: i32::MAX,
        days: i32::MAX,
    };

    assert!(matches!(
        date.add(saturated),
        Err(CalendarError::Arithmetic(_))
    ));
}

#[test]
fn time_duration_to_temporal_errors_when_temporal_limits_are_exceeded() {
    let time = Time::new(9, 30, 0, 0).expect("time should validate");

    // Saturating `hours` overwhelms Temporal's duration range in
    // `TimeDuration::to_temporal`, mapping to `DateError::CalendarError`.
    let saturated = TimeDuration {
        hours: i64::MAX,
        ..TimeDuration::default()
    };

    assert!(matches!(
        time.add(saturated),
        Err(DateError::CalendarError(_))
    ));
}

#[test]
fn date_time_duration_to_temporal_errors_when_temporal_limits_are_exceeded() {
    let date_time = CalendarDateTime::new(
        gregorian_date(2024, 3, 15),
        Time::new(12, 0, 0, 0).expect("time should validate"),
    );

    // A saturated time-component forces `DateTimeDuration::to_temporal` to
    // reject the combined duration and return `CalendarError::Arithmetic`.
    let saturated = DateTimeDuration {
        date: DateDuration::default(),
        time: TimeDuration {
            hours: i64::MAX,
            ..TimeDuration::default()
        },
    };

    assert!(matches!(
        date_time.add(saturated),
        Err(CalendarError::Arithmetic(_))
    ));
}

#[test]
fn calendar_date_set_rejects_invalid_day_in_target_month() {
    let date = gregorian_date(2024, 1, 15);

    // February 2024 has 29 days. Requesting day 31 under `Overflow::Reject`
    // propagates Temporal's arithmetic failure through `CalendarDate::set`'s
    // `.map_err` closure.
    let result = date.set(&CalendarDateFields {
        month: Some(2),
        day: Some(31),
        ..CalendarDateFields::default()
    });

    assert!(matches!(result, Err(CalendarError::Arithmetic(_))));
}

#[test]
fn calendar_date_set_rejects_overly_long_era_codes() {
    let date = gregorian_date(2024, 3, 15);

    // Era codes longer than the 19-byte tinystr limit must fail inside
    // `temporal_partial_date`, exercising its `.map_err` closure.
    let result = date.set(&CalendarDateFields {
        era: Some(japanese_era("extraordinarily-long-era-code", "Unsupported")),
        year: Some(1),
        month: Some(5),
        day: Some(1),
        ..CalendarDateFields::default()
    });

    assert!(matches!(result, Err(CalendarError::Arithmetic(_))));
}

#[test]
fn calendar_date_time_set_propagates_invalid_time_fields() {
    let date_time = CalendarDateTime::new(
        gregorian_date(2024, 3, 15),
        Time::new(9, 30, 45, 125).expect("time should validate"),
    );

    // Minute 75 is out of range; the `Time::set` error is remapped through
    // `CalendarDateTime::set`'s `.map_err` closure to `CalendarError`.
    let result = date_time.set(
        &CalendarDateFields::default(),
        TimeFields {
            minute: Some(75),
            ..TimeFields::default()
        },
    );

    assert!(matches!(result, Err(CalendarError::Arithmetic(_))));
}

#[cfg(feature = "std")]
#[test]
fn zoned_date_time_add_and_subtract_reject_saturated_durations() {
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

    let saturated = DateTimeDuration {
        date: DateDuration::default(),
        time: TimeDuration {
            hours: i64::MAX,
            ..TimeDuration::default()
        },
    };

    assert!(matches!(
        zoned.add(saturated),
        Err(CalendarError::Arithmetic(_))
    ));
    assert!(matches!(
        zoned.subtract(saturated),
        Err(CalendarError::Arithmetic(_))
    ));
}

#[cfg(feature = "std")]
#[test]
fn calendar_date_to_system_time_round_trips_through_zoned_date_time() {
    let date = gregorian_date(2024, 3, 15);

    let time_zone = TimeZoneId::new("UTC").expect("UTC should validate");

    let system = date
        .to_system_time(&time_zone)
        .expect("SystemTime projection should succeed");

    // Re-deriving via `to_zoned` must match, exercising the composed
    // `ZonedDateTime::to_system_time` path end-to-end.
    let zoned = to_zoned(&date, &time_zone).expect("zoned projection should succeed");

    assert_eq!(zoned.to_system_time().expect("system time"), system);
}

#[cfg(feature = "std")]
#[test]
fn to_calendar_date_time_composes_with_optional_time() {
    let date = gregorian_date(2024, 3, 15);

    let default_time = to_calendar_date_time(&date, None);
    assert_eq!(default_time.time(), &Time::default());
    assert_eq!(default_time.date(), &date);

    let explicit_time = Time::new(9, 30, 45, 0).expect("time should validate");
    let with_time = to_calendar_date_time(&date, Some(explicit_time));

    assert_eq!(with_time.time(), &explicit_time);

    // Feed it through `to_zoned_date_time` so the optional-time branch carries
    // through the zoned construction helper as well.
    let time_zone = TimeZoneId::new("UTC").expect("UTC should validate");
    let zoned = to_zoned_date_time(&with_time, &time_zone, Disambiguation::Compatible)
        .expect("zoned construction should succeed");

    assert_eq!(zoned.time_zone().as_str(), "UTC");
}
