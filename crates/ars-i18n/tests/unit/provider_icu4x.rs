//! Native `Icu4xProvider` tests (spec §9.5.2).

use alloc::string::{String, ToString};
use core::num::NonZero;

use crate::{
    CalendarDate, CalendarSystem, Era, HourCycle, Icu4xProvider, IcuProvider, Locale, Weekday,
    default_provider,
};

fn locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("test locale should parse")
}

#[test]
fn icu4x_weekday_short_label_localizes_in_arabic() {
    let provider = Icu4xProvider;

    let label = provider.weekday_short_label(Weekday::Monday, &locale("ar"));

    assert!(
        label
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "Arabic weekday label should contain Arabic characters; got {label:?}"
    );
    assert!(
        !label.is_empty(),
        "Arabic weekday label should not be empty"
    );
}

#[test]
fn icu4x_weekday_long_label_localizes_in_japanese() {
    let provider = Icu4xProvider;

    let label = provider.weekday_long_label(Weekday::Monday, &locale("ja"));

    // Japanese long weekday labels always contain the kanji "曜".
    assert!(
        label.contains('曜'),
        "Japanese long weekday label should contain 曜; got {label:?}"
    );
}

#[test]
fn icu4x_weekday_short_label_localizes_in_english() {
    let provider = Icu4xProvider;

    let label = provider.weekday_short_label(Weekday::Monday, &locale("en-US"));

    assert_eq!(label, "Mon");
}

#[test]
fn icu4x_weekday_short_label_covers_every_weekday() {
    let provider = Icu4xProvider;

    let en = locale("en-US");

    // Each weekday maps to a distinct day in January 2024 (Mon=1..Sun=7).
    for (weekday, expected) in [
        (Weekday::Monday, "Mon"),
        (Weekday::Tuesday, "Tue"),
        (Weekday::Wednesday, "Wed"),
        (Weekday::Thursday, "Thu"),
        (Weekday::Friday, "Fri"),
        (Weekday::Saturday, "Sat"),
        (Weekday::Sunday, "Sun"),
    ] {
        assert_eq!(
            provider.weekday_short_label(weekday, &en),
            expected,
            "short label for {weekday:?}"
        );
    }
}

#[test]
fn icu4x_month_long_name_localizes_in_japanese() {
    let provider = Icu4xProvider;

    let name = provider.month_long_name(1, &locale("ja"));

    // CLDR returns "1月" for month 1 in Japanese.
    assert!(
        name.contains('月'),
        "Japanese long month name should contain 月; got {name:?}"
    );
}

#[test]
fn icu4x_month_long_name_returns_unknown_for_invalid_month() {
    let provider = Icu4xProvider;

    assert_eq!(
        provider.month_long_name(0, &locale("en-US")),
        String::from("Unknown")
    );
    assert_eq!(
        provider.month_long_name(13, &locale("en-US")),
        String::from("Unknown")
    );
}

#[test]
fn icu4x_hour_cycle_reflects_locale() {
    let provider = Icu4xProvider;

    assert_eq!(provider.hour_cycle(&locale("en-US")), HourCycle::H12);
    assert_eq!(provider.hour_cycle(&locale("de-DE")), HourCycle::H23);
}

#[test]
fn icu4x_hour_cycle_ignores_native_digits_in_24h_locales() {
    // Regression: before treating non-ASCII digits as numerals, fa-IR
    // was misclassified as H12 because its 24-hour display uses
    // Persian numerals (۱۳:۰۰) whose characters aren't ASCII digits.
    let provider = Icu4xProvider;
    assert_eq!(provider.hour_cycle(&locale("fa-IR")), HourCycle::H23);
}

#[test]
fn icu4x_day_period_from_char_disambiguates_arabic_labels() {
    // Regression: before stripping Unicode numerals from the formatted
    // reference time, ar-EG AM/PM labels both started with `١` and
    // `day_period_from_char` could not distinguish AM from PM.
    let provider = Icu4xProvider;
    let ar = locale("ar-EG");
    let am_label = provider.day_period_label(false, &ar);
    let pm_label = provider.day_period_label(true, &ar);
    assert_ne!(
        am_label.chars().next(),
        pm_label.chars().next(),
        "AM and PM labels must not share a first character"
    );
    let am_char = am_label.chars().next().expect("AM label is non-empty");
    let pm_char = pm_label.chars().next().expect("PM label is non-empty");
    assert_eq!(provider.day_period_from_char(am_char, &ar), Some(false));
    assert_eq!(provider.day_period_from_char(pm_char, &ar), Some(true));
}

#[test]
fn icu4x_first_day_of_week_from_cldr() {
    let provider = Icu4xProvider;

    assert_eq!(
        provider.first_day_of_week(&locale("en-US")),
        Weekday::Sunday
    );
    assert_eq!(
        provider.first_day_of_week(&locale("de-DE")),
        Weekday::Monday
    );

    // `-u-fw-` extension overrides the CLDR default.
    assert_eq!(
        provider.first_day_of_week(&locale("en-US-u-fw-mon")),
        Weekday::Monday
    );
}

#[test]
fn icu4x_format_segment_digits_uses_native_digits_in_arabic() {
    let provider = Icu4xProvider;

    let formatted = provider.format_segment_digits(
        5,
        NonZero::new(2).expect("2 is non-zero"),
        &locale("ar-EG"),
    );

    // ar-EG uses Arabic-Indic native digits (٠١٢٣٤٥٦٧٨٩) by default via CLDR.
    assert_eq!(formatted, "٠٥");
}

#[test]
fn icu4x_format_segment_digits_preserves_ascii_in_english() {
    let provider = Icu4xProvider;

    let formatted = provider.format_segment_digits(
        7,
        NonZero::new(2).expect("2 is non-zero"),
        &locale("en-US"),
    );

    assert_eq!(formatted, "07");
}

#[test]
fn icu4x_day_period_label_returns_nonempty() {
    let provider = Icu4xProvider;

    let am = provider.day_period_label(false, &locale("en-US"));

    let pm = provider.day_period_label(true, &locale("en-US"));

    assert!(!am.is_empty(), "AM label should not be empty");
    assert!(!pm.is_empty(), "PM label should not be empty");
    assert_ne!(am, pm, "AM and PM labels must differ");
}

#[test]
fn icu4x_day_period_from_char_roundtrips_through_labels() {
    let provider = Icu4xProvider;

    assert_eq!(
        provider.day_period_from_char('a', &locale("en-US")),
        Some(false)
    );
    assert_eq!(
        provider.day_period_from_char('P', &locale("en-US")),
        Some(true)
    );
    assert_eq!(provider.day_period_from_char('x', &locale("en-US")), None);
}

#[test]
fn icu4x_max_months_in_year_detects_hebrew_leap() {
    let provider = Icu4xProvider;

    // 5784 is year 8 of the 19-year Metonic cycle (year % 19 = 8) — a leap
    // year with 13 months.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Hebrew, 5784, None),
        13
    );

    // 5785 is year 9 of the cycle — a common year with 12 months.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Hebrew, 5785, None),
        12
    );
}

#[test]
fn icu4x_max_months_in_year_clamps_japanese_end_of_era() {
    let provider = Icu4xProvider;

    // Heisei 31 (2019) ended on 30 April; the final year is capped at 4 months.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Japanese, 31, Some("heisei")),
        4
    );
}

#[test]
fn icu4x_days_in_month_respects_japanese_end_of_era() {
    let provider = Icu4xProvider;

    // Heisei 31-04 ends on day 30 (final day of the era).
    assert_eq!(
        provider.days_in_month(&CalendarSystem::Japanese, 31, 4, Some("heisei")),
        30
    );

    // Gregorian February 2024 is 29 days (leap).
    assert_eq!(
        provider.days_in_month(&CalendarSystem::Gregorian, 2024, 2, None),
        29
    );
}

#[test]
fn icu4x_default_era_for_japanese_is_reiwa() {
    let provider = Icu4xProvider;

    assert_eq!(
        provider.default_era(&CalendarSystem::Japanese),
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        })
    );
}

#[test]
fn icu4x_era_boundary_queries_match_spec() {
    let provider = Icu4xProvider;

    let heisei_1_1_8 = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(Era {
            code: "heisei".to_string(),
            display_name: "Heisei".to_string(),
        }),
        1,
        1,
        8,
    )
    .expect("Heisei 1-1-8 should validate");

    assert_eq!(provider.years_in_era(&heisei_1_1_8), Some(31));
    assert_eq!(provider.minimum_month_in_year(&heisei_1_1_8), 1);
    assert_eq!(provider.minimum_day_in_month(&heisei_1_1_8), 8);

    let reiwa_1_5_1 = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        }),
        1,
        5,
        1,
    )
    .expect("Reiwa 1-5-1 should validate");

    assert_eq!(provider.minimum_month_in_year(&reiwa_1_5_1), 5);
}

#[test]
fn icu4x_convert_date_crosses_calendars() {
    let provider = Icu4xProvider;

    let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 3, 15)
        .expect("Gregorian 2024-03-15 should validate");

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(japanese.calendar, CalendarSystem::Japanese);
    assert_eq!(japanese.year, 6);
    assert_eq!(japanese.month.get(), 3);
    assert_eq!(japanese.day.get(), 15);
    assert!(japanese.era.is_some());

    // Cross-calendar conversion is reversible through the provider.
    let round_trip = provider.convert_date(&japanese, CalendarSystem::Gregorian);

    assert_eq!(round_trip, gregorian);
}

#[test]
fn icu4x_convert_date_to_hebrew_yields_valid_date() {
    let provider = Icu4xProvider;

    let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 1992, 9, 2)
        .expect("Gregorian 1992-09-02 should validate");

    let hebrew = provider.convert_date(&gregorian, CalendarSystem::Hebrew);

    assert_eq!(hebrew.calendar, CalendarSystem::Hebrew);
    assert!(hebrew.year >= 5752 && hebrew.year <= 5753);
    assert!((1..=13).contains(&hebrew.month.get()));
    assert!((1..=30).contains(&hebrew.day.get()));
}

#[test]
fn icu4x_convert_date_returns_source_for_out_of_range_year() {
    // `internal::CalendarDate::try_from` rejects dates the ICU4X bridge
    // can't represent; the provider falls back to the source date instead
    // of panicking. Year 10_000 is the documented upper bound for the
    // bridge.
    let provider = Icu4xProvider;

    let gregorian = CalendarDate::new_gregorian(
        10_000,
        NonZero::new(1).expect("one is non-zero"),
        NonZero::new(1).expect("one is non-zero"),
    );

    let converted = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(converted, gregorian);
}

#[test]
fn icu4x_convert_date_same_calendar_is_identity() {
    let provider = Icu4xProvider;

    let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 3, 15)
        .expect("Gregorian date should validate");

    let converted = provider.convert_date(&gregorian, CalendarSystem::Gregorian);

    assert_eq!(converted, gregorian);
}

#[test]
fn icu4x_default_provider_under_icu4x_feature() {
    // With the `icu4x` feature enabled, `default_provider()` selects the
    // Icu4xProvider branch. The Boxed trait object isn't downcastable
    // without extra machinery, so we assert observable behavior that the
    // Stub would get wrong.
    let provider = default_provider();

    let label = provider.weekday_long_label(Weekday::Monday, &locale("ja"));

    assert!(
        label.contains('曜'),
        "default_provider() under icu4x should return Japanese labels; got {label:?}"
    );
}
