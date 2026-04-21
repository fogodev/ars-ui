//! `StubIntlBackend` tests (spec §9.5.1).

use alloc::string::ToString;
use core::num::NonZero;

use crate::{
    CalendarDate, CalendarDateFields, CalendarSystem, Era, HourCycle, IntlBackend, Locale,
    StubIntlBackend, WeekInfo, Weekday,
};

fn locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("test locale should parse")
}

fn calendar_date(
    calendar: CalendarSystem,
    era: Option<Era>,
    year: i32,
    month: u8,
    day: u8,
) -> CalendarDate {
    CalendarDate::new(
        calendar,
        &CalendarDateFields {
            era,
            year: Some(year),
            month: Some(month),
            day: Some(day),
            ..CalendarDateFields::default()
        },
    )
    .expect("calendar test date should validate")
}

#[test]
fn stub_provider_explicitly_returns_english_labels() {
    let backend = StubIntlBackend;

    let locale = locale("en-US");

    assert_eq!(backend.weekday_short_label(Weekday::Monday, &locale), "Mo");
    assert_eq!(
        backend.weekday_long_label(Weekday::Monday, &locale),
        "Monday"
    );
    assert_eq!(backend.month_long_name(1, &locale), "January");
    assert_eq!(backend.month_long_name(13, &locale), "Unknown");
    assert_eq!(backend.day_period_label(false, &locale), "AM");
    assert_eq!(backend.day_period_label(true, &locale), "PM");
}

#[test]
fn stub_provider_explicitly_parses_day_period_and_formats_digits() {
    let backend = StubIntlBackend;

    let locale = locale("en-US");

    assert_eq!(backend.day_period_from_char('a', &locale), Some(false));
    assert_eq!(backend.day_period_from_char('P', &locale), Some(true));
    assert_eq!(backend.day_period_from_char('x', &locale), None);
    assert_eq!(
        backend.format_segment_digits(7, NonZero::new(2).expect("2 is non-zero"), &locale),
        "07"
    );
}

#[test]
fn stub_provider_explicitly_reports_locale_hour_cycle_and_week_info() {
    let backend = StubIntlBackend;

    let english = locale("en-US");

    let german = locale("de-DE");

    assert_eq!(backend.hour_cycle(&english), HourCycle::H12);
    assert_eq!(backend.hour_cycle(&german), HourCycle::H23);
    assert_eq!(backend.week_info(&english), WeekInfo::for_locale(&english));
    assert_eq!(backend.week_info(&german), WeekInfo::for_locale(&german));
    assert_eq!(backend.first_day_of_week(&english), Weekday::Sunday);
    assert_eq!(backend.first_day_of_week(&german), Weekday::Monday);
}

#[test]
fn stub_provider_keeps_calendar_helper_defaults() {
    let backend = StubIntlBackend;

    assert_eq!(
        backend.default_era(&CalendarSystem::Japanese),
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        })
    );
    assert_eq!(
        backend.max_months_in_year(&CalendarSystem::Japanese, 31, Some("heisei")),
        4
    );
    assert_eq!(
        backend.days_in_month(&CalendarSystem::Japanese, 31, 4, Some("heisei")),
        30
    );

    let heisei = calendar_date(
        CalendarSystem::Japanese,
        Some(Era {
            code: "heisei".to_string(),
            display_name: "Heisei".to_string(),
        }),
        1,
        1,
        8,
    );

    let reiwa = calendar_date(
        CalendarSystem::Japanese,
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        }),
        1,
        5,
        1,
    );

    assert_eq!(backend.years_in_era(&heisei), Some(31));
    assert_eq!(backend.minimum_month_in_year(&reiwa), 5);
    assert_eq!(backend.minimum_day_in_month(&heisei), 8);
}
