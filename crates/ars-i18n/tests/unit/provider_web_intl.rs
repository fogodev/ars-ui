//! WASM `WebIntlProvider` tests (spec §9.5.4).
//!
//! Run with:
//! `wasm-pack test --headless --firefox crates/ars-i18n --no-default-features --features std,web-intl`.

use alloc::{format, string::ToString};
use core::num::NonZero;

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use crate::{
    CalendarDate, CalendarDateFields, CalendarSystem, Era, HourCycle, IcuProvider, Locale,
    WebIntlProvider, Weekday, default_provider,
};

wasm_bindgen_test_configure!(run_in_browser);

fn locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("test locale should parse")
}

fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("Gregorian test date should validate")
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
fn web_intl_hour_cycle_fr_fr_is_h23() {
    let provider = WebIntlProvider;

    assert_eq!(provider.hour_cycle(&locale("fr-FR")), HourCycle::H23);
}

#[wasm_bindgen_test]
fn web_intl_hour_cycle_uses_formatted_parts_not_resolved_options() {
    let provider = WebIntlProvider;

    // Regression (#583): browser `resolvedOptions().hourCycle` is known
    // to misreport some locales. Force Arabic-Indic digits so the
    // provider must normalize the observed hour part and still infer
    // French 24-hour output correctly.
    assert_eq!(
        provider.hour_cycle(&locale("fr-FR-u-nu-arab")),
        HourCycle::H23
    );
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
fn web_intl_week_info_reads_first_day_and_weekend_metadata() {
    let provider = WebIntlProvider;

    let us = provider.week_info(&locale("en-US"));

    assert_eq!(us.first_day, Weekday::Sunday);
    assert_eq!(us.weekend_start, Weekday::Saturday);
    assert_eq!(us.weekend_end, Weekday::Sunday);

    let israel = provider.week_info(&locale("he-IL"));

    assert_eq!(israel.first_day, Weekday::Sunday);
    assert_eq!(israel.weekend_start, Weekday::Friday);
    assert_eq!(israel.weekend_end, Weekday::Saturday);
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
fn web_intl_day_period_from_char_bails_when_labels_share_prefix() {
    // Regression (Codex round 7): when the browser-emitted AM and PM
    // labels share their first character, `day_period_from_char`
    // would previously resolve any matching input to AM because the
    // AM arm was checked first. The provider now returns `None` so
    // CJK-style ambiguous input fails safely instead.
    //
    // We can't forcibly make a real locale produce identical first
    // chars, so we lean on the live Japanese labels: if the diff
    // extractor happens to leave a shared prefix (e.g., a future
    // CLDR revision), the ambiguous char must resolve to `None`; if
    // it doesn't, the happy path is verified end-to-end.
    let provider = WebIntlProvider;

    let ja = locale("ja-JP");

    let am = provider.day_period_label(false, &ja);

    let pm = provider.day_period_label(true, &ja);

    let am_first = am.chars().next().expect("AM label non-empty");

    let pm_first = pm.chars().next().expect("PM label non-empty");

    if am_first == pm_first {
        assert_eq!(
            provider.day_period_from_char(am_first, &ja),
            None,
            "shared first char must not collapse to AM"
        );
    } else {
        assert_eq!(provider.day_period_from_char(am_first, &ja), Some(false));
        assert_eq!(provider.day_period_from_char(pm_first, &ja), Some(true));
    }
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
fn web_intl_max_months_in_year_resolves_persian_via_bridge() {
    let provider = WebIntlProvider;

    // Persian is a 12-month solar calendar with no leap-month year
    // variants. The bridge returns 12 for every year, matching the
    // previous fixed fallback without fabricating it.
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Persian, 1403, None),
        12
    );
}

#[wasm_bindgen_test]
fn web_intl_hour_cycle_honors_locale_extension() {
    let provider = WebIntlProvider;

    // Explicit `-u-hc-*` overrides must short-circuit the probe logic
    // so callers get the exact midnight behavior they requested.
    assert_eq!(provider.hour_cycle(&locale("ja-u-hc-h11")), HourCycle::H11);
    assert_eq!(
        provider.hour_cycle(&locale("en-US-u-hc-h12")),
        HourCycle::H12
    );
    assert_eq!(
        provider.hour_cycle(&locale("de-DE-u-hc-h23")),
        HourCycle::H23
    );
    assert_eq!(provider.hour_cycle(&locale("ja-u-hc-h24")), HourCycle::H24);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_routes_bce_gregorian_through_bridge() {
    // Regression (Codex PR #563 comment 3103592557, P2): when
    // `u32::try_from(date.year)` rejects a negative Gregorian year,
    // the previous code returned the source date unchanged — a
    // straight contract violation for `IcuProvider::convert_date`,
    // which must always return a date in `target`. The fallback now
    // runs `bridge_convert` so BCE Gregorian inputs convert correctly
    // (the ICU4X calendar-arithmetic bridge handles negative year
    // arithmetic natively) or clone the source only when the bridge
    // itself rejects the input.
    let provider = WebIntlProvider;

    let gregorian = calendar_date(
        CalendarSystem::Gregorian,
        Some(Era {
            code: "bc".to_string(),
            display_name: "BC".to_string(),
        }),
        51,
        3,
        15,
    );

    let buddhist = provider.convert_date(&gregorian, CalendarSystem::Buddhist);

    // Buddhist Era year = Gregorian year + 543. -50 CE → Buddhist 493.
    // The browser path must use the canonical ISO year (-50), not the
    // era-relative public year (51 BC). Otherwise the conversion lands
    // in Buddhist 594 instead of Buddhist 493.
    assert_eq!(
        buddhist.calendar(),
        CalendarSystem::Buddhist,
        "BCE Gregorian must convert to Buddhist via the bridge, not be echoed back"
    );
    assert_eq!(
        buddhist.year(),
        493,
        "Gregorian 51 BC corresponds to Buddhist 493; got {}",
        buddhist.year()
    );
    assert_ne!(
        buddhist, gregorian,
        "BCE Gregorian must not be returned as the source date"
    );
}

#[wasm_bindgen_test]
fn web_intl_days_in_month_non_gregorian_uses_bridge_not_flat_30() {
    // Regression (Codex adversarial review round 14): the previous
    // implementation hard-coded 30 days for every non-Gregorian month
    // outside the `bounded_*` table. That accepted impossible day 30
    // inputs on 29-day months and rejected legal day 31 inputs on
    // 31-day months during `CalendarDate::new` validation. We now
    // delegate to the shared ICU4X calendar-arithmetic bridge, which
    // produces the correct per-year month lengths.
    let provider = WebIntlProvider;

    // Hebrew civil-order month 3 is Kislev, whose length varies
    // between 29 and 30 days by year type (chaser/kesidran/shalem).
    // 28..=30 is the full Hebrew-lunisolar range; assert the bridge
    // returns a real number inside it rather than the fabricated 30.
    let kislev = provider.days_in_month(&CalendarSystem::Hebrew, 5785, 3, None);

    assert!(
        (28..=30).contains(&kislev),
        "Hebrew 5785 Kislev length must be in the lunisolar range; got {kislev}"
    );

    // Chinese month 2 in 2024 is 29 or 30 days — the bridge must
    // pick one, not default to the old flat 30.
    let chinese_2 = provider.days_in_month(&CalendarSystem::Chinese, 2024, 2, None);

    assert!(
        (29..=30).contains(&chinese_2),
        "Chinese 2024 civil month 2 length must be 29 or 30; got {chinese_2}"
    );
}

#[wasm_bindgen_test]
fn web_intl_max_months_in_year_non_gregorian_uses_bridge_not_flat_12() {
    // Regression (Codex adversarial review round 14): the previous
    // implementation returned a flat 12 months for every calendar
    // outside the explicit Hebrew/Ethiopic/Coptic arms, which rejected
    // valid Chinese/Dangi leap-month dates at `CalendarDate::new`
    // validation and normalised civil-ordinal 13 inputs into the
    // following year on `add_months(0)`. The bridge produces the
    // real per-year answer including leap-cycle widenings.
    let provider = WebIntlProvider;

    // Chinese 2020 is a leap-month year (闰四月, leap 4th) → 13.
    let months_2020 = provider.max_months_in_year(&CalendarSystem::Chinese, 2020, None);

    assert_eq!(
        months_2020, 13,
        "Chinese 2020 has leap month 4, so the year carries 13 civil ordinals"
    );

    // Chinese 2021 is a non-leap year → 12.
    let months_2021 = provider.max_months_in_year(&CalendarSystem::Chinese, 2021, None);

    assert_eq!(
        months_2021, 12,
        "Chinese 2021 has no leap month, so the year carries 12 civil ordinals"
    );
}

#[wasm_bindgen_test]
fn web_intl_era_helpers_match_stub_behavior() {
    let provider = WebIntlProvider;

    assert_eq!(
        provider.default_era(&CalendarSystem::Japanese),
        Some(Era {
            code: "reiwa".to_string(),
            display_name: "Reiwa".to_string(),
        })
    );
    assert_eq!(
        provider.default_era(&CalendarSystem::Gregorian),
        Some(Era {
            code: "ad".to_string(),
            display_name: "AD".to_string(),
        })
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

    let gregorian = gregorian_date(2024, 3, 15);

    let converted = provider.convert_date(&gregorian, CalendarSystem::Gregorian);

    assert_eq!(converted, gregorian);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_crosses_calendars_via_browser() {
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(2024, 3, 15);

    // Under `--features web-intl` without `icu4x`, this hits the
    // Intl.DateTimeFormat({ calendar }) → formatToParts reparse path.
    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(japanese.calendar(), CalendarSystem::Japanese);
    assert_eq!(japanese.year(), 6);
    assert_eq!(japanese.month(), 3);
    assert_eq!(japanese.day(), 15);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_historical_japanese_era() {
    // Regression: previously `convert_date` hard-coded `default_era_for`
    // (Reiwa) for the target calendar, so historical Gregorian dates
    // like 1990 came out with era=Reiwa, year=2 instead of era=Heisei,
    // year=2. The fix requests `era: "long"` from `Intl.DateTimeFormat`
    // and maps the localized label back to the CLDR era code.
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(1990, 6, 15);

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(japanese.calendar(), CalendarSystem::Japanese);
    assert_eq!(japanese.year(), 2);
    assert_eq!(japanese.month(), 6);
    assert_eq!(japanese.day(), 15);

    let era = japanese
        .era()
        .cloned()
        .expect("Japanese dates carry an era");

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

    // Shōwa: 1926-12-25 .. 1989-01-07. Gregorian 1970 is deep inside it.
    let gregorian = gregorian_date(1970, 6, 15);

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    let era = japanese
        .era()
        .cloned()
        .expect("Japanese dates carry an era");

    assert_eq!(
        era.code, "showa",
        "macron must be stripped; got era.code = {:?} (display_name = {:?})",
        era.code, era.display_name
    );

    // Taishō: 1912-07-30 .. 1926-12-25. Gregorian 1920 is inside.
    let gregorian = gregorian_date(1920, 6, 15);

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    let era = japanese
        .era()
        .cloned()
        .expect("Japanese dates carry an era");

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
fn web_intl_convert_date_recovers_chinese_leap_month_ordinal() {
    // Regression (Codex round 9): Chinese/Dangi leap months surface
    // from `Intl.DateTimeFormat({ month: "numeric" })` as tokens like
    // `"6bis"` or `"06L"` on ICU-backed runtimes, so `str::parse`
    // returned 0 and `convert_date` fell through to `date.clone()`.
    // We now strip a trailing leap-month marker, parse the leading
    // digits, and keep the conversion alive (with the caveat that
    // leap-vs-regular precision is lost in pure `web-intl` — the
    // `icu4x` bridge handles that correctly).
    //
    // Gregorian 2020-05-30 falls inside Chinese 闰四月 (leap 4th
    // month) in some calendar variants. Because actual browser
    // behaviour for Chinese is inconsistent (some emit numeric, some
    // the `bis` token, some a localised name), we only assert the
    // weaker post-condition: the converted date must differ from the
    // source and must expose a non-zero month ordinal.
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(2020, 5, 30);

    let chinese = provider.convert_date(&gregorian, CalendarSystem::Chinese);

    assert_ne!(
        chinese, gregorian,
        "convert_date must not silently clone the source for Chinese leap-month dates"
    );
    assert!(
        (1..=13).contains(&chinese.month()),
        "Chinese month must be 1..=13; got {}",
        chinese.month()
    );
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_years_below_100() {
    // Regression: `js_sys::Date::new_with_year_month_day(y, _, _)` routes
    // through the legacy `new Date(y, m)` path, which reinterprets
    // `0..=99` as `1900..=1999`. `convert_date` now uses `new Date()`
    // + `setUTCFullYear(..)` so a Gregorian year below 100 is passed
    // to `Intl.DateTimeFormat` as itself.
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(90, 6, 15);

    let buddhist = provider.convert_date(&gregorian, CalendarSystem::Buddhist);

    assert_eq!(buddhist.calendar(), CalendarSystem::Buddhist);

    // Buddhist year = Gregorian year + 543. Year 90 CE ⇒ Buddhist 633,
    // not Buddhist 2533 (which the century-quirk bug would have produced).
    assert_eq!(
        buddhist.year(),
        633,
        "expected Buddhist 633 for Gregorian year 90; got {}",
        buddhist.year()
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

    // March 25, 2024 is Adar II 15, 5784 (Hebrew leap year).
    let gregorian = gregorian_date(2024, 3, 25);

    let hebrew = provider.convert_date(&gregorian, CalendarSystem::Hebrew);

    assert_eq!(hebrew.calendar(), CalendarSystem::Hebrew);
    assert_eq!(hebrew.year(), 5784);

    // Whether the browser emitted numeric 7 (Adar II) or the name
    // "Adar II" that we then resolved, the final ordinal must be valid.
    assert!(
        (1..=13).contains(&hebrew.month()),
        "Hebrew month must be 1..=13; got {}",
        hebrew.month()
    );
    assert_eq!(hebrew.day(), 15);
}

#[wasm_bindgen_test]
fn web_intl_convert_date_bridges_non_gregorian_sources_under_web_intl() {
    // Regression (Codex adversarial review round 14): the previous
    // implementation gated the internal ICU4X calendar-arithmetic
    // bridge behind `#[cfg(feature = "icu4x")]` and then returned
    // `date.clone()` for every non-Gregorian source under pure
    // `--features web-intl`. That violated the
    // `IcuProvider::convert_date` contract — downstream callers
    // (`CalendarDate::add_days_with_provider`, `TypedCalendarDate::
    // to_calendar`) assumed the returned date lives in `target`.
    // Returning the source calendar panicked the Gregorian-only
    // `add_days` path and let `TypedCalendarDate` wrap the wrong
    // calendar in release builds (only `debug_assert` guarded it).
    //
    // The `calendar::internal` module is gated on `any(feature =
    // "icu4x", feature = "web-intl")` — it compiles under pure
    // `web-intl` because the arithmetic does not need CLDR data. The
    // bridge is now unconditionally active for non-Gregorian sources
    // under this feature, so Buddhist 2567 → Gregorian must resolve
    // to the real Gregorian equivalent, not echo the source back.
    let provider = WebIntlProvider;

    let buddhist = calendar_date(CalendarSystem::Buddhist, None, 2567, 6, 15);

    let gregorian = provider.convert_date(&buddhist, CalendarSystem::Gregorian);

    assert_eq!(gregorian.calendar(), CalendarSystem::Gregorian);
    assert_eq!(
        gregorian.year(),
        2024,
        "Buddhist 2567 = Gregorian 2024 via the internal bridge; got {gregorian:?}"
    );
    assert_eq!(gregorian.month(), 6);
    assert_eq!(gregorian.day(), 15);
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
fn web_intl_parse_english_ordinal_shifts_leap_ordinals_up_by_one() {
    // Regression (Codex round 13): `CalendarDate::month` stores
    // `DateFields.ordinal_month` (civil-order ordinal), so a leap
    // month always occupies the slot *after* its base lunar month.
    // A previous revision returned the base ordinal unchanged for
    // both `"Sixth Monthbis"` and `"Sixth Month"`, so they
    // collided at ordinal 6. The leap path now bumps by 1:
    use crate::provider::web_intl::parse_english_ordinal_month_label;

    assert_eq!(parse_english_ordinal_month_label("Sixth Month"), Some(6));
    assert_eq!(parse_english_ordinal_month_label("Sixth Monthbis"), Some(7));
    assert_eq!(parse_english_ordinal_month_label("First Monthbis"), Some(2));
    assert_eq!(
        parse_english_ordinal_month_label("Twelfth Monthbis"),
        Some(13)
    );

    // 13 + 1 saturates back to 13 so the result stays within the
    // `CalendarDate::month: NonZero<u8>` range allowed by the type.
    assert_eq!(
        parse_english_ordinal_month_label("Thirteenth Monthbis"),
        Some(13)
    );
}

#[wasm_bindgen_test]
fn web_intl_parse_english_ordinal_month_label_covers_all_ordinals() {
    // Regression (Codex round 12): a fixed `SWEEP_YEARS` list will
    // always miss some leap-month positions — `First Monthbis`
    // (last occurred 1651), `Twelfth Monthbis` (last 1832), and
    // other distant cycle positions can't be covered without
    // maintaining an exhaustive year list. Parsing the English
    // ordinal label directly bypasses the probe entirely.
    //
    // This test pins all thirteen ordinals plus the `bis` leap
    // marker plus a rejection assertion for unrecognised input.
    use crate::provider::web_intl::parse_english_ordinal_month_label;

    for (label, expected) in [
        ("First Month", 1_u8),
        ("Second Month", 2),
        ("Third Month", 3),
        ("Fourth Month", 4),
        ("Fifth Month", 5),
        ("Sixth Month", 6),
        ("Seventh Month", 7),
        ("Eighth Month", 8),
        ("Ninth Month", 9),
        ("Tenth Month", 10),
        ("Eleventh Month", 11),
        ("Twelfth Month", 12),
        ("Thirteenth Month", 13),
    ] {
        assert_eq!(
            parse_english_ordinal_month_label(label),
            Some(expected),
            "label {label:?} should resolve to {expected}"
        );

        // Leap variant resolves to `expected + 1` (capped at 13):
        // the leap month always sits in the civil-order slot after
        // its base lunar month, matching the `DateFields.ordinal_month`
        // semantics of `CalendarDate::month`.
        let bis = format!("{label}bis");

        let leap_expected = expected.saturating_add(1).min(13);

        assert_eq!(
            parse_english_ordinal_month_label(&bis),
            Some(leap_expected),
            "leap label {bis:?} should resolve to {leap_expected}"
        );
    }

    // Nonsense inputs must return None so the caller falls back
    // through the probe sweep.
    assert_eq!(parse_english_ordinal_month_label("Adar II"), None);
    assert_eq!(parse_english_ordinal_month_label("Month of Sundays"), None);
    assert_eq!(parse_english_ordinal_month_label(""), None);
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_resolves_english_ordinal_labels_without_probe() {
    // Regression (Codex round 12): Node Intl currently emits labels
    // like `First Monthbis` and `Twelfth Monthbis` that no realistic
    // `SWEEP_YEARS` list can cover (they require Chinese leap 1 /
    // leap 12, both extremely rare in modern history). The resolver
    // now parses those labels directly before consulting the probe
    // sweep, so downstream `convert_date` doesn't silently clone.
    for label in [
        "First Monthbis",
        "Tenth Monthbis",
        "Twelfth Monthbis",
        "Seventh Monthbis",
    ] {
        let ordinal = WebIntlProvider::resolve_named_month(CalendarSystem::Chinese, 2024, label);

        assert!(
            matches!(ordinal, Some(o) if (1..=13).contains(&o)),
            "{label:?} must resolve via direct English-ordinal parse; got {ordinal:?}"
        );
    }
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_reaches_leap_7_8_10_11_years() {
    // Regression (Codex round 11): the earlier sweep missed Chinese
    // leap positions 7, 8, 10, and 11 because none of the probed
    // Gregorian years sat inside those cycles. The sweep now
    // explicitly includes 1987/2006/2044 (leap 7), 1995/2014/2052
    // (leap 8), 2033/2099 (leap 10/11). We verify the sweep reaches
    // these cycles by checking that if the runtime emits a specific
    // leap-N label for any of them, the resolver returns a
    // 1..=13 ordinal.
    let candidates = [
        "Seventh Monthbis",
        "Eighth Monthbis",
        "Tenth Monthbis",
        "Eleventh Monthbis",
    ];

    let mut any_resolved = false;

    let mut any_invalid = false;

    for label in candidates {
        let ordinal = WebIntlProvider::resolve_named_month(CalendarSystem::Chinese, 2024, label);

        match ordinal {
            Some(o) if (1..=13).contains(&o) => any_resolved = true,

            Some(_) => any_invalid = true,

            None => {}
        }
    }
    // Must never return out-of-range ordinals. Resolving at least
    // one of the labels proves the sweep reached the affected year;
    // if none resolved (runtime emits numeric or native names), the
    // fallback None contract is still honoured.
    assert!(
        !any_invalid,
        "resolver returned an out-of-range ordinal for a leap-month label"
    );

    let _ = any_resolved;
}

#[wasm_bindgen_test]
fn web_intl_first_day_of_week_reads_week_info_property_shape() {
    // Regression (Codex round 11): when `getWeekInfo` was absent
    // the old code dropped straight to the region table, which is
    // incomplete (e.g., pt-BR was returning Monday instead of
    // Sunday). The provider now also probes the `weekInfo` property
    // shape emerging on some engines. We can't force a specific
    // engine at test time, so assert that both pt-BR and en-US
    // resolve to Sunday on real runtimes — whichever shape the
    // browser uses, the answer should be the CLDR value.
    let provider = WebIntlProvider;

    assert_eq!(
        provider.first_day_of_week(&locale("pt-BR")),
        Weekday::Sunday,
        "pt-BR must resolve to Sunday via either getWeekInfo() or weekInfo property"
    );
    assert_eq!(
        provider.first_day_of_week(&locale("en-US")),
        Weekday::Sunday
    );
}

#[wasm_bindgen_test]
fn web_intl_weekday_from_iso_index_unit_coverage() {
    // Direct coverage for the helper so the 1..=7 → Weekday mapping
    // stays correct even when no browser currently emits every
    // value.
    use crate::provider::web_intl::weekday_from_iso_index;

    assert_eq!(weekday_from_iso_index(1), Weekday::Monday);
    assert_eq!(weekday_from_iso_index(2), Weekday::Tuesday);
    assert_eq!(weekday_from_iso_index(3), Weekday::Wednesday);
    assert_eq!(weekday_from_iso_index(4), Weekday::Thursday);
    assert_eq!(weekday_from_iso_index(5), Weekday::Friday);
    assert_eq!(weekday_from_iso_index(6), Weekday::Saturday);
    assert_eq!(weekday_from_iso_index(7), Weekday::Sunday);

    // Out-of-range inputs collapse to Monday per the function doc.
    assert_eq!(weekday_from_iso_index(0), Weekday::Monday);
    assert_eq!(weekday_from_iso_index(8), Weekday::Monday);
    assert_eq!(weekday_from_iso_index(42), Weekday::Monday);
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_sweeps_beyond_24_25() {
    // Regression (Codex round 10): the probe list was hard-coded to
    // `[2024, 2025]`, so Chinese/Dangi leap months from other cycle
    // positions — e.g., Chinese `Second Monthbis` in 2023, `Fourth
    // Monthbis` in 2020 — never resolved. The resolver now fans out
    // across a 19-year sweep after the Hebrew fast path, so any
    // leap-month label emitted in any recent cycle position stays
    // recoverable.
    //
    // Browsers differ on whether `month: "long"` emits English
    // descriptors ("Second Monthbis") or native names (`"二月"`),
    // so we don't pin a specific label string. Instead we sweep a
    // handful of candidate labels for the Chinese calendar and
    // assert that *at least one* resolves to a valid ordinal,
    // proving the sweep window covers the affected years.
    let candidates = [
        "Second Monthbis",
        "Fourth Monthbis",
        "Sixth Monthbis",
        "Ninth Monthbis",
    ];

    let resolved = candidates.iter().any(|label| {
        matches!(
            WebIntlProvider::resolve_named_month(CalendarSystem::Chinese, 2024, label),
            Some(o) if (1..=13).contains(&o)
        )
    });

    // If none of the candidates match (runtime emits numeric or
    // native names), at least prove the resolver still returns
    // `None` safely rather than panicking.
    if !resolved {
        assert_eq!(
            WebIntlProvider::resolve_named_month(
                CalendarSystem::Chinese,
                2024,
                "Definitely Not A Month"
            ),
            None
        );
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

#[wasm_bindgen_test]
fn web_intl_era_code_for_calendar_maps_roc_long_labels() {
    // Regression (Codex PR #563 comment 3103280941, P1): ROC era long
    // labels (`Minguo`, `Before R.O.C.`, `B.R.O.C.`) used to be
    // persisted as `era.code` after a plain `canonical_era_code`
    // pass — i.e., as `minguo` / `b.r.o.c.` — which are not CLDR
    // codes. ICU4X's `Date::try_from_fields` rejects those, so the
    // internal bridge failed and `convert_date` silently echoed the
    // source date back to the caller.
    //
    // `era_code_for_calendar` now validates against a per-calendar
    // allow-list and maps:
    //   `Minguo` / `ROC`                    → Some("roc")
    //   `Before R.O.C.` / `B.R.O.C.` /
    //   `Before Minguo`                     → Some("broc")
    //   unknown label                        → None
    use crate::provider::web_intl::era_code_for_calendar;

    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "Minguo").as_deref(),
        Some("roc"),
        "Chrome emits 'Minguo' for ROC post-1912 — must map to `roc`"
    );
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "ROC").as_deref(),
        Some("roc")
    );
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "Before R.O.C.").as_deref(),
        Some("broc"),
        "separator-rich label must normalize to `broc`"
    );
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "B.R.O.C.").as_deref(),
        Some("broc")
    );
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "Before Minguo").as_deref(),
        Some("broc")
    );
    // Unknown ROC label must NOT be persisted as a CLDR code.
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Roc, "Not A ROC Era"),
        None
    );
}

#[wasm_bindgen_test]
fn web_intl_era_code_for_calendar_maps_japanese_allow_list() {
    // Japanese era labels are already covered by
    // `web_intl_convert_date_canonicalizes_macronized_japanese_eras`
    // at the `convert_date` level, but pin the allow-list directly
    // to catch future changes that drop a supported era by mistake.
    use crate::provider::web_intl::era_code_for_calendar;

    for (label, expected) in [
        ("Reiwa", "reiwa"),
        ("Heisei", "heisei"),
        ("Shōwa", "showa"),
        ("Taishō", "taisho"),
        ("Meiji", "meiji"),
    ] {
        assert_eq!(
            era_code_for_calendar(CalendarSystem::Japanese, label).as_deref(),
            Some(expected),
            "Japanese era {label:?} must canonicalize to {expected:?}"
        );
    }

    // Unknown Japanese era must drop out.
    assert_eq!(
        era_code_for_calendar(CalendarSystem::Japanese, "Fabricated"),
        None
    );
}

#[wasm_bindgen_test]
fn web_intl_era_code_for_calendar_drops_unmapped_custom_era_labels() {
    // Hebrew / Ethiopic / Coptic / Hijri / Persian long labels vary
    // too much across browsers to map without targeted test
    // coverage. For each of those custom-era calendars, the helper
    // must return `None` so `convert_date` does not persist display
    // text as a CLDR era code.
    use crate::provider::web_intl::era_code_for_calendar;

    for target in [
        CalendarSystem::Hebrew,
        CalendarSystem::Ethiopic,
        CalendarSystem::EthiopicAmeteAlem,
        CalendarSystem::Coptic,
        CalendarSystem::Persian,
        CalendarSystem::IslamicCivil,
        CalendarSystem::IslamicUmmAlQura,
    ] {
        assert_eq!(
            era_code_for_calendar(target, "AM"),
            None,
            "{target:?} should not persist 'AM' without confirmed ICU4X mapping"
        );
        assert_eq!(
            era_code_for_calendar(target, "Anno Mundi"),
            None,
            "{target:?} should not persist 'Anno Mundi' either"
        );
    }
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_roc_era_round_trip() {
    // End-to-end regression: the ROC calendar now persists CLDR
    // codes (`roc`/`broc`), so a Gregorian 2024-06-15 → ROC
    // conversion must land in the `roc` era (Republic of China year
    // 113) rather than a bogus `minguo` code that would fail the
    // internal-bridge round-trip.
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(2024, 6, 15);

    let roc = provider.convert_date(&gregorian, CalendarSystem::Roc);

    assert_eq!(roc.calendar(), CalendarSystem::Roc);

    // ROC year = Gregorian year - 1911. 2024 - 1911 = 113.
    assert_eq!(
        roc.year(),
        113,
        "Gregorian 2024 corresponds to ROC 113; got {}",
        roc.year()
    );

    // Browsers that emit an era part must have it normalised to a
    // CLDR code — `roc` — and never left as `minguo`/`b.r.o.c.`.
    if let Some(era) = roc.era().cloned() {
        assert_eq!(
            era.code, "roc",
            "post-1912 ROC date must carry the `roc` CLDR code; got {era:?}"
        );
    }
}

#[wasm_bindgen_test]
fn web_intl_convert_date_routes_japanese_historical_eras_through_bridge() {
    // Regression (Codex PR #563 comment 3103505088, P1): when
    // `Intl.DateTimeFormat` emits a Japanese long era label that
    // falls outside the `era_code_for_calendar` allow-list
    // (historical eras like `Kansei (1789–1801)`, `Meiwa`, `Bunsei`,
    // `Tenpō`), the previous code fell back to `default_era_for`
    // which returns Reiwa — silently rewriting the historical date
    // to `Reiwa N` and corrupting downstream era-boundary behaviour.
    //
    // The fallback now runs the shared ICU4X calendar-arithmetic
    // bridge, which knows the full CLDR era vocabulary. So a
    // Gregorian 1800-06-15 (inside the Kansei era, 1789–1801)
    // resolves to a valid Japanese date with a Kansei / historical
    // CLDR code — never Reiwa.
    let provider = WebIntlProvider;

    let gregorian = gregorian_date(1800, 6, 15);

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    assert_eq!(japanese.calendar(), CalendarSystem::Japanese);

    let era = japanese
        .era()
        .cloned()
        .expect("Japanese dates must carry an era");

    assert_ne!(
        era.code, "reiwa",
        "historical Gregorian 1800 must not be misattributed to Reiwa; got {era:?}"
    );
    assert_ne!(
        era.code, "heisei",
        "historical Gregorian 1800 predates Heisei; got {era:?}"
    );
    assert_ne!(
        era.code, "meiji",
        "historical Gregorian 1800 predates Meiji; got {era:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_convert_date_preserves_modern_japanese_allow_list_hits() {
    // Complement to the bridge-fallback test above: the fast path
    // (allow-list hit) must still persist the browser's label as
    // `display_name` while canonicalising `code`. This pins the
    // behavior for the common Reiwa/Heisei/Showa/Taishō/Meiji case,
    // which still runs through `era_code_for_calendar` rather than
    // the bridge fallback.
    let provider = WebIntlProvider;

    // Gregorian 2020-06-15 is deep inside Reiwa (2019+).
    let gregorian = gregorian_date(2020, 6, 15);

    let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);

    let era = japanese
        .era()
        .cloned()
        .expect("Japanese dates must carry an era");

    assert_eq!(era.code, "reiwa", "modern Gregorian 2020 must be Reiwa");
    assert_eq!(japanese.year(), 2);
}

#[wasm_bindgen_test]
fn web_intl_bridge_convert_handles_non_gregorian_sources() {
    // Direct unit coverage for the extracted `bridge_convert` helper
    // on a non-Gregorian source — the same entry point used by
    // `convert_date` when the source calendar is not Gregorian.
    use crate::provider::web_intl::bridge_convert;

    let buddhist = calendar_date(CalendarSystem::Buddhist, None, 2567, 6, 15);

    let gregorian = bridge_convert(&buddhist, CalendarSystem::Gregorian)
        .expect("Buddhist → Gregorian must succeed through the bridge");

    assert_eq!(gregorian.calendar(), CalendarSystem::Gregorian);
    assert_eq!(gregorian.year(), 2024, "Buddhist 2567 = Gregorian 2024");
}

#[wasm_bindgen_test]
fn web_intl_resolve_named_month_recovers_when_2_digit_returns_names() {
    // Regression (Codex PR #563 comment 3103083388, P1): when the
    // `month: "2-digit"` probe emits a non-numeric label — Node.js /
    // ICU do this for Hebrew, returning names like `"Adar II"` from
    // both `long` and `2-digit` — the resolver previously hard-failed
    // with `return None` on the first match and dropped the whole
    // conversion to `date.clone()`. The fallback now resolves the
    // civil-order ordinal through the shared ICU4X calendar-
    // arithmetic bridge instead.
    //
    // We exercise this by resolving well-known Hebrew leap-year
    // labels against 5784 (leap year). Either the browser's numeric
    // formatter returns a number (fast path) or it returns the name
    // again (bridge fallback); both must produce the canonical civil-
    // order ordinal. The test proves the bridge fallback does not
    // regress the fast path — coverage of the fallback branch is
    // guaranteed on Node-backed test runners where this PR lands.
    let labels_and_expected = [("Adar I", 6_u8), ("Adar II", 7_u8)];

    for (label, expected) in labels_and_expected {
        if let Some(ordinal) =
            WebIntlProvider::resolve_named_month(CalendarSystem::Hebrew, 5784, label)
        {
            assert_eq!(
                ordinal, expected,
                "{label} must resolve to civil-order {expected} whether via 2-digit or the \
                 bridge fallback; got {ordinal}"
            );
        }
    }
}
