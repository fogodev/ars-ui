//! Wasm parity tests for the calendar module (spec §4.1–§4.6).
//!
//! These mirror the backend-agnostic integration tests in
//! `tests/unit/calendar.rs` as `#[wasm_bindgen_test]` twins so a wasm32
//! regression in `temporal_rs` (panic, allocator mismatch, float printing
//! quirk) would fail CI instead of shipping to browsers silently.
//!
//! Tests that depend on `#[cfg(feature = "std")]`-only helpers
//! (`to_zoned`, `ZonedDateTime`, `Temporal::local_now`, `SystemTime`) are
//! intentionally omitted — they can't run on `wasm32-unknown-unknown` without
//! a system clock.
//!
//! Run with:
//! `wasm-pack test --headless --chrome crates/ars-i18n --no-default-features --features std,web-intl`.

use alloc::vec;

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use super::{
    CalendarDate, CalendarDateFields, CalendarDateTime, CalendarSystem, CycleOptions,
    CycleTimeOptions, DateDuration, DateField, DateTimeDuration, DateTimeField, Era, MonthCode,
    Time, TimeDuration, TimeField, TimeFields, parse, queries,
};
use crate::{Locale, StubIntlBackend};

wasm_bindgen_test_configure!(run_in_browser);

fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("Gregorian fixture should validate")
}

fn japanese_era(code: &str, display_name: &str) -> Era {
    Era {
        code: code.into(),
        display_name: display_name.into(),
    }
}

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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
