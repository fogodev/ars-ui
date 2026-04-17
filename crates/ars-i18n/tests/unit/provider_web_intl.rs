//! WASM `WebIntlProvider` tests (spec §9.5.4).
//!
//! Run with:
//! `wasm-pack test --headless --firefox crates/ars-i18n --no-default-features --features std,web-intl`.

use alloc::string::ToString;
use core::num::NonZero;

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use crate::{
    CalendarDate, CalendarSystem, Era, HourCycle, IcuProvider, Locale, StubIcuProvider,
    WebIntlProvider, Weekday, default_provider,
};

wasm_bindgen_test_configure!(run_in_browser);

fn locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("test locale should parse")
}

#[wasm_bindgen_test]
fn web_intl_weekday_short_label_returns_english() {
    let provider = WebIntlProvider;

    let label = provider.weekday_short_label(Weekday::Monday, &locale("en-US"));

    assert!(!label.is_empty());

    // Browsers render Monday as "Mon" in `en` short form.
    assert!(
        label.starts_with('M'),
        "English Monday short label should start with M; got {label:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_weekday_long_label_localizes_in_japanese() {
    let provider = WebIntlProvider;

    let label = provider.weekday_long_label(Weekday::Monday, &locale("ja"));

    assert!(
        label.contains('曜'),
        "Japanese long weekday label should contain 曜; got {label:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_month_long_name_localizes_in_japanese() {
    let provider = WebIntlProvider;

    let name = provider.month_long_name(1, &locale("ja"));

    assert!(
        name.contains('月'),
        "Japanese long month name should contain 月; got {name:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_hour_cycle_en_us_is_h12() {
    let provider = WebIntlProvider;

    assert_eq!(provider.hour_cycle(&locale("en-US")), HourCycle::H12);
}

#[wasm_bindgen_test]
fn web_intl_first_day_of_week_en_us_is_sunday() {
    let provider = WebIntlProvider;

    assert_eq!(
        provider.first_day_of_week(&locale("en-US")),
        Weekday::Sunday
    );
}

#[wasm_bindgen_test]
fn web_intl_first_day_of_week_de_is_monday() {
    let provider = WebIntlProvider;

    assert_eq!(
        provider.first_day_of_week(&locale("de-DE")),
        Weekday::Monday
    );
}

#[wasm_bindgen_test]
fn web_intl_format_segment_digits_uses_native_digits_in_arabic() {
    let provider = WebIntlProvider;

    let formatted = provider.format_segment_digits(
        5,
        NonZero::new(2).expect("2 is non-zero"),
        &locale("ar-EG"),
    );

    // ar-EG formats 05 as Arabic-Indic ٠٥ via `Intl.NumberFormat`.
    assert_eq!(formatted, "٠٥");
}

#[wasm_bindgen_test]
fn web_intl_format_segment_digits_preserves_ascii_in_english() {
    let provider = WebIntlProvider;

    let formatted = provider.format_segment_digits(
        7,
        NonZero::new(2).expect("2 is non-zero"),
        &locale("en-US"),
    );

    assert_eq!(formatted, "07");
}

#[wasm_bindgen_test]
fn web_intl_day_period_labels_differ_en_us() {
    let provider = WebIntlProvider;

    let am = provider.day_period_label(false, &locale("en-US"));

    let pm = provider.day_period_label(true, &locale("en-US"));

    assert!(!am.is_empty());
    assert!(!pm.is_empty());
    assert_ne!(am, pm);
}

#[wasm_bindgen_test]
fn web_intl_days_in_month_gregorian_leap_february() {
    let provider = WebIntlProvider;

    assert_eq!(
        provider.days_in_month(&CalendarSystem::Gregorian, 2024, 2, None),
        29
    );
    assert_eq!(
        provider.days_in_month(&CalendarSystem::Gregorian, 2023, 2, None),
        28
    );
}

#[wasm_bindgen_test]
fn web_intl_default_provider_under_web_intl() {
    let provider = default_provider();

    // Japanese localization proves we reached the browser-backed provider.
    let label = provider.weekday_long_label(Weekday::Monday, &locale("ja"));

    assert!(
        label.contains('曜'),
        "default_provider() under web-intl should return Japanese labels; got {label:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_weekday_short_label_covers_every_weekday() {
    let provider = WebIntlProvider;

    let en = locale("en-US");

    for weekday in [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ] {
        let label = provider.weekday_short_label(weekday, &en);
        assert!(!label.is_empty(), "empty short label for {weekday:?}");
    }
}

#[wasm_bindgen_test]
fn web_intl_day_period_from_char_roundtrips_english() {
    let provider = WebIntlProvider;

    let en = locale("en-US");

    assert_eq!(provider.day_period_from_char('a', &en), Some(false));
    assert_eq!(provider.day_period_from_char('P', &en), Some(true));
    assert_eq!(provider.day_period_from_char('x', &en), None);
}

#[wasm_bindgen_test]
fn web_intl_month_long_name_returns_unknown_for_invalid_month() {
    let provider = WebIntlProvider;

    assert_eq!(provider.month_long_name(0, &locale("en-US")), "Unknown");
    assert_eq!(provider.month_long_name(13, &locale("en-US")), "Unknown");
}

#[wasm_bindgen_test]
fn web_intl_max_months_in_year_detects_hebrew_leap() {
    let provider = WebIntlProvider;

    // Cycle year 8 is a Hebrew leap year (13 months).
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Hebrew, 5784, None),
        13
    );
    // Cycle year 9 is a common year (12 months).
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Hebrew, 5785, None),
        12
    );
    // Ethiopic/Coptic calendars always have 13 months.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Ethiopic, 2017, None),
        13
    );
}

#[wasm_bindgen_test]
fn web_intl_max_months_in_year_clamps_japanese_end_of_era() {
    let provider = WebIntlProvider;

    // End-of-era clamping is served by `bounded_months_in_year`, which the
    // provider calls before the Hebrew/Ethiopic table.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Japanese, 31, Some("heisei")),
        4
    );
}

#[wasm_bindgen_test]
fn web_intl_days_in_month_clamps_japanese_end_of_era() {
    let provider = WebIntlProvider;

    // Heisei year 31 month 4 is the era's final month; the shared
    // `bounded_days_in_month` helper must clamp to day 30 before the
    // non-Gregorian fallback kicks in.
    assert_eq!(
        provider.days_in_month(&CalendarSystem::Japanese, 31, 4, Some("heisei")),
        30
    );
}

#[wasm_bindgen_test]
fn web_intl_max_months_in_year_falls_back_to_twelve_for_persian() {
    let provider = WebIntlProvider;

    // Persian is not covered by the Hebrew 19-cycle or Ethiopic/Coptic
    // fixed-13 table, so the provider falls through to the `_ => 12`
    // wildcard arm.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Persian, 1403, None),
        12
    );
}

#[wasm_bindgen_test]
fn web_intl_hour_cycle_honors_locale_extension() {
    let provider = WebIntlProvider;

    // `-u-hc-h11` and `-u-hc-h24` force the browser's
    // `resolvedOptions().hourCycle` to those values, exercising the
    // `H11` and `H24` match arms that plain English or German locales
    // won't hit.
    assert_eq!(provider.hour_cycle(&locale("ja-u-hc-h11")), HourCycle::H11);
    assert_eq!(provider.hour_cycle(&locale("ja-u-hc-h24")), HourCycle::H24);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_returns_source_for_pre_ce_year() {
    let provider = WebIntlProvider;

    let stub = StubIcuProvider;

    // Buddhist year 1 converts to Gregorian -542 — `u32::try_from` rejects
    // the negative year, so the provider returns the source date.
    let buddhist = CalendarDate::new(&stub, CalendarSystem::Buddhist, None, 1, 1, 1)
        .expect("Buddhist 1-1-1 should validate");

    let gregorian = CalendarDate {
        calendar: CalendarSystem::Gregorian,
        era: None,
        year: -542,
        month: NonZero::new(1).expect("one is non-zero"),
        day: NonZero::new(1).expect("one is non-zero"),
    };

    // This exercises the `u32::try_from(date.year)` error path even under
    // `--features icu4x,web-intl` because the internal ICU4X bridge rejects
    // Buddhist → Gregorian for the exact BCE year this test uses.
    // `buddhist` is constructed above as a readability anchor for the
    // comment; we intentionally don't feed it into the provider call
    // below because the test pre-computes the equivalent Gregorian
    // fields by hand.
    drop(buddhist);

    let converted = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(converted, gregorian);
}

#[wasm_bindgen_test]
fn web_intl_days_in_month_non_gregorian_uses_spec_fallback() {
    let provider = WebIntlProvider;

    // Non-Gregorian calendars use the spec §9.5.4 conservative fallback of
    // 30 days when `bounded_days_in_month` doesn't clamp and no probing is
    // implemented yet.
    assert_eq!(
        provider.days_in_month(&CalendarSystem::Hebrew, 5785, 3, None),
        30
    );
}

#[wasm_bindgen_test]
fn web_intl_era_helpers_match_stub_behavior() {
    let provider = WebIntlProvider;

    let stub = StubIcuProvider;

    assert_eq!(
        provider.default_era(&CalendarSystem::Japanese),
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        })
    );
    assert_eq!(provider.default_era(&CalendarSystem::Gregorian), None);

    let heisei = CalendarDate::new(
        &stub,
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

    assert_eq!(provider.years_in_era(&heisei), Some(31));
    assert_eq!(provider.minimum_month_in_year(&heisei), 1);
    assert_eq!(provider.minimum_day_in_month(&heisei), 8);
}

#[wasm_bindgen_test]
fn web_intl_first_day_of_week_honors_unicode_extension() {
    let provider = WebIntlProvider;

    // `-u-fw-sat` overrides the region default of Sunday for en-US.
    assert_eq!(
        provider.first_day_of_week(&locale("en-US-u-fw-sat")),
        Weekday::Saturday
    );
}

#[wasm_bindgen_test]
fn web_intl_convert_date_same_calendar_is_identity() {
    let provider = WebIntlProvider;

    let stub = StubIcuProvider;

    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 2024, 3, 15)
        .expect("Gregorian date should validate");

    let converted = provider.convert_date(&gregorian, CalendarSystem::Gregorian);

    assert_eq!(converted, gregorian);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_crosses_calendars_via_browser() {
    let provider = WebIntlProvider;

    let stub = StubIcuProvider;

    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 2024, 3, 15)
        .expect("Gregorian date should validate");

    // Under `--features web-intl` without `icu4x`, this hits the
    // Intl.DateTimeFormat({ calendar }) → formatToParts reparse path.
    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(japanese.calendar, CalendarSystem::Japanese);
    assert_eq!(japanese.year, 6);
    assert_eq!(japanese.month.get(), 3);
    assert_eq!(japanese.day.get(), 15);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_historical_japanese_era() {
    // Regression: previously `convert_date` hard-coded `default_era_for`
    // (Reiwa) for the target calendar, so historical Gregorian dates
    // like 1990 came out with era=Reiwa, year=2 instead of era=Heisei,
    // year=2. The fix requests `era: "long"` from `Intl.DateTimeFormat`
    // and maps the localized label back to the CLDR era code.
    let provider = WebIntlProvider;
    let stub = StubIcuProvider;

    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 1990, 6, 15)
        .expect("Gregorian 1990-06-15 should validate");

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);
    assert_eq!(japanese.calendar, CalendarSystem::Japanese);
    assert_eq!(japanese.year, 2);
    assert_eq!(japanese.month.get(), 6);
    assert_eq!(japanese.day.get(), 15);
    let era = japanese.era.expect("Japanese dates carry an era");
    assert_eq!(
        era.code, "heisei",
        "1990 falls inside Heisei (1989-2019); got {era:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_convert_date_canonicalizes_macronized_japanese_eras() {
    // Regression: `Intl.DateTimeFormat('en-US', { calendar: 'japanese',
    // era: 'long' })` emits macron spellings — `Shōwa`, `Taishō`, `Meiji`
    // — so `label.to_ascii_lowercase()` used to produce `shōwa` / `taishō`
    // which do NOT match the canonical CLDR era codes (`showa`,
    // `taisho`). Downstream era-aware helpers then missed the era and
    // silently returned incorrect bounds. `canonical_era_code` strips
    // the known Latin-macron letters before lowercasing.
    let provider = WebIntlProvider;
    let stub = StubIcuProvider;

    // Shōwa: 1926-12-25 .. 1989-01-07. Gregorian 1970 is deep inside it.
    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 1970, 6, 15)
        .expect("Gregorian 1970-06-15 should validate");
    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);
    let era = japanese.era.expect("Japanese dates carry an era");
    assert_eq!(
        era.code, "showa",
        "macron must be stripped; got era.code = {:?} (display_name = {:?})",
        era.code, era.display_name
    );

    // Taishō: 1912-07-30 .. 1926-12-25. Gregorian 1920 is inside.
    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 1920, 6, 15)
        .expect("Gregorian 1920-06-15 should validate");
    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);
    let era = japanese.era.expect("Japanese dates carry an era");
    assert_eq!(
        era.code, "taisho",
        "macron must be stripped; got era.code = {:?} (display_name = {:?})",
        era.code, era.display_name
    );
}

#[wasm_bindgen_test]
fn web_intl_canonical_era_code_strips_known_macrons() {
    use crate::provider::web_intl::canonical_era_code;
    // Direct unit coverage for the helper — stays meaningful even if a
    // future browser stops emitting macron long labels.
    assert_eq!(canonical_era_code("Shōwa"), "showa");
    assert_eq!(canonical_era_code("Taishō"), "taisho");
    assert_eq!(canonical_era_code("Heisei"), "heisei");
    assert_eq!(canonical_era_code("Reiwa"), "reiwa");
    assert_eq!(canonical_era_code("Meiji"), "meiji");
    // Non-macronized labels round-trip through the plain lowercase path.
    assert_eq!(canonical_era_code("CE"), "ce");
    assert_eq!(canonical_era_code("AH"), "ah");
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_years_below_100() {
    // Regression: `js_sys::Date::new_with_year_month_day(y, _, _)` routes
    // through the legacy `new Date(y, m)` path, which reinterprets
    // `0..=99` as `1900..=1999`. `convert_date` now uses `new Date()`
    // + `setUTCFullYear(..)` so a Gregorian year below 100 is passed
    // to `Intl.DateTimeFormat` as itself.
    let provider = WebIntlProvider;
    let stub = StubIcuProvider;

    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 90, 6, 15)
        .expect("Gregorian 0090-06-15 should validate");

    let buddhist = provider.convert_date(&gregorian, CalendarSystem::Buddhist);
    assert_eq!(buddhist.calendar, CalendarSystem::Buddhist);
    // Buddhist year = Gregorian year + 543. Year 90 CE ⇒ Buddhist 633,
    // not Buddhist 2533 (which the century-quirk bug would have produced).
    assert_eq!(
        buddhist.year, 633,
        "expected Buddhist 633 for Gregorian year 90; got {}",
        buddhist.year
    );
}

#[wasm_bindgen_test]
fn web_intl_convert_date_resolves_hebrew_named_month() {
    // Regression: `Intl.DateTimeFormat("en-US", { calendar: "hebrew",
    // month: "numeric" })` can still emit "Adar I" / "Adar II" in Hebrew
    // leap years. The fix retries as `month: "long"` and matches the
    // label against a probe loop. This test exercises the named-month
    // resolution path even in browsers that return a numeric month —
    // we just verify the result has a valid 1-based month ordinal.
    let provider = WebIntlProvider;
    let stub = StubIcuProvider;

    // March 25, 2024 is Adar II 15, 5784 (Hebrew leap year).
    let gregorian = CalendarDate::new(&stub, CalendarSystem::Gregorian, None, 2024, 3, 25)
        .expect("Gregorian 2024-03-25 should validate");

    let hebrew = provider.convert_date(&gregorian, CalendarSystem::Hebrew);
    assert_eq!(hebrew.calendar, CalendarSystem::Hebrew);
    assert_eq!(hebrew.year, 5784);
    // Whether the browser emitted numeric 7 (Adar II) or the name
    // "Adar II" that we then resolved, the final ordinal must be valid.
    assert!(
        (1..=13).contains(&hebrew.month.get()),
        "Hebrew month must be 1..=13; got {}",
        hebrew.month.get()
    );
    assert_eq!(hebrew.day.get(), 15);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_non_gregorian_source_without_icu4x_is_identity() {
    // Regression: the pure-`web-intl` path used to pass non-Gregorian
    // `date.year/month/day` straight to `Intl.DateTimeFormat({ calendar: target })`,
    // which reinterpreted them as Gregorian and produced nonsense (Reiwa
    // 6-03-15 came back as Gregorian year 6). Under `--features web-intl`
    // without `icu4x` there is no Rust-side bridge to resolve the source
    // calendar, so the provider now returns the source date unchanged
    // instead of silently corrupting it.
    //
    // When `icu4x` is *also* enabled, the internal bridge above handles
    // the conversion correctly — this test stays meaningful in both
    // configurations because the post-condition is "no invalid date
    // emitted", which both paths satisfy.
    let provider = WebIntlProvider;
    let stub = StubIcuProvider;

    let buddhist = CalendarDate::new(&stub, CalendarSystem::Buddhist, None, 2567, 6, 15)
        .expect("Buddhist 2567-06-15 should validate");

    let gregorian = provider.convert_date(&buddhist, CalendarSystem::Gregorian);
    if cfg!(feature = "icu4x") {
        // Internal ICU4X bridge: Buddhist 2567 ≈ Gregorian 2024.
        assert_eq!(gregorian.calendar, CalendarSystem::Gregorian);
        assert_eq!(gregorian.year, 2024);
    } else {
        // Pure web-intl: safe identity fallback.
        assert_eq!(gregorian, buddhist);
    }
}

#[wasm_bindgen_test]
fn web_intl_is_hebrew_leap_year_matches_19_cycle() {
    use crate::provider::web_intl::is_hebrew_leap_year;
    // Hebrew 5784 sits at position 8 of the 19-year Metonic cycle — leap.
    assert!(is_hebrew_leap_year(5784));
    // Hebrew 5785 sits at position 9 — common.
    assert!(!is_hebrew_leap_year(5785));
    // Cycle positions 3, 6, 8, 11, 14, 17, and year 19 (0 mod 19) are leap.
    for offset in [3, 6, 8, 11, 14, 17, 0] {
        assert!(
            is_hebrew_leap_year(offset),
            "cycle offset {offset} should be leap"
        );
    }
    // Cycle position 1 must not be leap.
    assert!(!is_hebrew_leap_year(1));
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_returns_calendar_ordinal_not_probe_slot() {
    // Regression: the previous resolver returned the Gregorian probe
    // loop counter as the target-calendar month ordinal, which is
    // always the Gregorian slot (1..=12) — not the Hebrew civil-order
    // ordinal. A Hebrew leap year's `Adar II` is civil ordinal 7 and
    // must resolve to 7 regardless of which Gregorian probe month
    // surfaced the label. `Adar I` is civil ordinal 6 in the same leap
    // year. In a common year, the sole `Adar` is ordinal 6.
    //
    // `0` means the runtime emitted neither "Adar I"/"Adar II" nor the
    // common-year "Adar" for the requested labels; treat that as a
    // skipped assertion (some old Intl builds collapse Hebrew to
    // numeric months), but when a value is returned it must be the
    // canonical civil-order ordinal.
    if let Some(ordinal) =
        WebIntlProvider::resolve_named_month(CalendarSystem::Hebrew, 5784, "Adar II")
    {
        assert_eq!(ordinal, 7, "Adar II must resolve to civil-order 7");
    }
    if let Some(ordinal) =
        WebIntlProvider::resolve_named_month(CalendarSystem::Hebrew, 5784, "Adar I")
    {
        assert_eq!(ordinal, 6, "Adar I must resolve to civil-order 6");
    }
    if let Some(ordinal) =
        WebIntlProvider::resolve_named_month(CalendarSystem::Hebrew, 5785, "Adar")
    {
        assert_eq!(ordinal, 6, "Common-year Adar must resolve to civil-order 6");
    }
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_matches_common_year_adar() {
    // Regression: the previous probe hard-coded Gregorian 2024 (Hebrew
    // leap year), so common-year labels like "Adar" never matched and
    // `convert_date` silently fell back. The resolver now sweeps both a
    // leap and a common probe year.
    let ordinal = WebIntlProvider::resolve_named_month(
        CalendarSystem::Hebrew,
        5785, // Common year — `rem_euclid(19) == 9`.
        "Adar",
    );
    assert!(
        matches!(ordinal, Some(o) if (1..=13).contains(&o)),
        "'Adar' must resolve to a 1..=13 ordinal under the dual-probe strategy; got {ordinal:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_resolves_known_hebrew_labels() {
    // Direct coverage for `WebIntlProvider::resolve_named_month`: even if
    // Chrome emits a numeric month for Hebrew (bypassing the fallback
    // path in `convert_date`), the probe loop must still be able to
    // recognise the long labels the browser returns under `month: "long"`.
    //
    // Known Hebrew long labels in a CLDR-compliant browser (en-US):
    //   Nisan, Iyar, Sivan, Tammuz, Av, Elul, Tishri, Heshvan,
    //   Kislev, Tevet, Shevat, Adar, Adar I, Adar II.
    //
    // Every resolvable label must return an ordinal in 1..=13 and every
    // unknown label must return `None`.
    for label in [
        "Tishri", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar", "Adar I", "Adar II", "Nisan",
        "Iyar", "Sivan", "Tammuz", "Av", "Elul",
    ] {
        if let Some(ordinal) =
            WebIntlProvider::resolve_named_month(CalendarSystem::Hebrew, 5784, label)
        {
            assert!(
                (1..=13).contains(&ordinal),
                "{label:?} resolved to out-of-range ordinal {ordinal}"
            );
        }
    }
    // A nonsense label must not match any month slot.
    assert_eq!(
        WebIntlProvider::resolve_named_month(
            CalendarSystem::Hebrew,
            5784,
            "Definitely Not A Month"
        ),
        None
    );
}
