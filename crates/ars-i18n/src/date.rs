//! Locale-aware date formatting helpers.
//!
//! The public API stays stable across ICU4X, browser `Intl`, and fallback
//! builds. Backend selection is internal to this module.

#[cfg(any(
    feature = "icu4x",
    all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")),
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
use alloc::format;
use alloc::string::String;

#[cfg(feature = "icu4x")]
use icu::datetime::{
    DateTimeFormatter as IcuDateTimeFormatter, DateTimeFormatterPreferences,
    fieldsets::{T, YMD, YMDE},
};
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use {
    js_sys::{
        Array, Date as JsDate, Function,
        Intl::{DateTimeFormat as JsDateTimeFormat, DateTimeFormatOptions, DateTimeStyle},
    },
    wasm_bindgen::JsValue,
};

#[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
use crate::calendar::internal::CalendarDate as InternalCalendarDate;
use crate::{CalendarDate, CalendarSystem, Locale};

/// Length of the formatted date/time string.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FormatLength {
    /// Full date with weekday.
    Full,
    /// Long month name date.
    Long,
    /// Medium abbreviated date.
    #[default]
    Medium,
    /// Short numeric date.
    Short,
}

impl FormatLength {
    /// Returns the ICU4X field set for date-only formatting.
    #[cfg(feature = "icu4x")]
    fn to_icu_date_field_set(self) -> YMD {
        debug_assert!(
            !self.is_full(),
            "full format must use to_icu_full_date_field_set"
        );

        match self {
            Self::Full | Self::Long => YMD::long(),
            Self::Medium => YMD::medium(),
            Self::Short => YMD::short(),
        }
    }

    /// Returns the ICU4X field set for weekday-inclusive full dates.
    #[cfg(feature = "icu4x")]
    const fn to_icu_full_date_field_set(self) -> YMDE {
        YMDE::long()
    }

    /// Returns `true` when this format length includes the weekday.
    #[must_use]
    pub const fn is_full(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Returns the ICU4X field set for time-only formatting.
    #[cfg(feature = "icu4x")]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "time field-set support is part of the public formatter contract even though date-only formatting is implemented in this task"
        )
    )]
    fn to_icu_time_field_set(self) -> T {
        match self {
            Self::Full | Self::Long => T::hms(),
            Self::Medium | Self::Short => T::hm(),
        }
    }

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    const fn to_js_date_style(self) -> DateTimeStyle {
        match self {
            Self::Full => DateTimeStyle::Full,
            Self::Long => DateTimeStyle::Long,
            Self::Medium => DateTimeStyle::Medium,
            Self::Short => DateTimeStyle::Short,
        }
    }
}

/// Internal ICU4X formatter storage.
#[cfg(feature = "icu4x")]
#[derive(Debug)]
enum DateFormatterInner {
    Ymd(IcuDateTimeFormatter<YMD>),
    Ymde(IcuDateTimeFormatter<YMDE>),
}

/// A locale-aware date formatter.
pub struct DateFormatter {
    locale: Locale,
    length: FormatLength,
    #[cfg(feature = "icu4x")]
    inner: DateFormatterInner,
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    inner: JsDateTimeFormat,
}

impl core::fmt::Debug for DateFormatter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DateFormatter")
            .field("locale", &self.locale)
            .field("length", &self.length)
            .finish()
    }
}

#[cfg(feature = "icu4x")]
impl DateFormatter {
    /// Creates a new locale-aware date formatter.
    #[must_use]
    pub fn new(locale: &Locale, length: FormatLength) -> Self {
        let prefs = DateTimeFormatterPreferences::from(locale.as_icu());

        let inner = if length.is_full() {
            DateFormatterInner::Ymde(
                IcuDateTimeFormatter::try_new(prefs, length.to_icu_full_date_field_set())
                    .expect("compiled_data guarantees date formatter data for all locales"),
            )
        } else {
            DateFormatterInner::Ymd(
                IcuDateTimeFormatter::try_new(prefs, length.to_icu_date_field_set())
                    .expect("compiled_data guarantees date formatter data for all locales"),
            )
        };

        Self {
            locale: locale.clone(),
            length,
            inner,
        }
    }

    /// Formats a calendar date for the formatter locale.
    #[must_use]
    pub fn format(&self, date: &CalendarDate) -> String {
        let Ok(internal) = InternalCalendarDate::try_from(date) else {
            return fallback_format_date(&self.locale, self.length, date);
        };

        match &self.inner {
            DateFormatterInner::Ymd(formatter) => formatter.format(&internal.inner).to_string(),
            DateFormatterInner::Ymde(formatter) => formatter.format(&internal.inner).to_string(),
        }
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
impl DateFormatter {
    /// Creates a new browser-backed date formatter.
    #[must_use]
    pub fn new(locale: &Locale, length: FormatLength) -> Self {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let options = DateTimeFormatOptions::new();
        options.set_date_style(length.to_js_date_style());
        options.set_time_zone("UTC");

        let formatter = JsDateTimeFormat::new(&locales, options.as_ref());

        Self {
            locale: locale.clone(),
            length,
            inner: formatter,
        }
    }

    /// Formats a calendar date through `Intl.DateTimeFormat`.
    ///
    /// Public `CalendarDate` values are first resolved to their absolute
    /// Gregorian day and then formatted using the locale's active calendar,
    /// including any `u-ca-*` Unicode extension carried by the locale.
    #[must_use]
    pub fn format(&self, date: &CalendarDate) -> String {
        let Some(js_date) = js_date_from_calendar(date) else {
            return fallback_format_date(&self.locale, self.length, date);
        };

        let format: Function = self.inner.format();

        format
            .call1(&JsValue::UNDEFINED, js_date.as_ref())
            .expect("Intl.DateTimeFormat.format should not throw for a valid Date")
            .as_string()
            .unwrap_or_default()
    }
}

#[cfg(any(
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
impl DateFormatter {
    /// Creates a compile-safe fallback date formatter.
    #[must_use]
    pub fn new(locale: &Locale, length: FormatLength) -> Self {
        Self {
            locale: locale.clone(),
            length,
        }
    }

    /// Formats dates using a deterministic English fallback.
    #[must_use]
    pub fn format(&self, date: &CalendarDate) -> String {
        fallback_format_date(&self.locale, self.length, date)
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_from_calendar(date: &CalendarDate) -> Option<JsDate> {
    if date.calendar == CalendarSystem::Gregorian {
        let js_date = js_date_from_ymd(date.year, date.month.get(), date.day.get());

        return js_date_is_valid(&js_date).then_some(js_date);
    }

    let internal = InternalCalendarDate::try_from(date).ok()?;

    let gregorian = internal.to_calendar(CalendarSystem::Gregorian);

    let js_date = js_date_from_ymd(gregorian.year(), gregorian.month(), gregorian.day());

    js_date_is_valid(&js_date).then_some(js_date)
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_from_ymd(year: i32, month: u8, day: u8) -> JsDate {
    let iso = format!("{}-{month:02}-{day:02}T12:00:00.000Z", js_iso_year(year),);

    JsDate::new(&JsValue::from_str(&iso))
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_is_valid(date: &JsDate) -> bool {
    !date.get_time().is_nan()
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_iso_year(year: i32) -> String {
    if (0..=9_999).contains(&year) {
        format!("{year:04}")
    } else if year < 0 {
        format!("-{:06}", year.unsigned_abs())
    } else {
        format!("+{year:06}")
    }
}

fn fallback_format_date(_locale: &Locale, length: FormatLength, date: &CalendarDate) -> String {
    if date.calendar != CalendarSystem::Gregorian {
        return date.to_iso8601();
    }

    let year = date.year;
    let month = date.month.get();
    let day = date.day.get();

    match length {
        FormatLength::Short => format!("{month}/{day}/{:02}", year.rem_euclid(100)),

        FormatLength::Medium => {
            format!("{} {day}, {year}", english_month_short(month))
        }

        FormatLength::Long => {
            format!("{} {day}, {year}", english_month_long(month))
        }

        FormatLength::Full => {
            format!(
                "{}, {} {day}, {year}",
                english_weekday_long(date.weekday()),
                english_month_long(month)
            )
        }
    }
}

const fn english_month_short(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

const fn english_month_long(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

const fn english_weekday_long(weekday: crate::Weekday) -> &'static str {
    match weekday {
        crate::Weekday::Sunday => "Sunday",
        crate::Weekday::Monday => "Monday",
        crate::Weekday::Tuesday => "Tuesday",
        crate::Weekday::Wednesday => "Wednesday",
        crate::Weekday::Thursday => "Thursday",
        crate::Weekday::Friday => "Friday",
        crate::Weekday::Saturday => "Saturday",
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use alloc::format;
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use alloc::string::String;
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use core::num::NonZero;

    #[cfg(feature = "icu4x")]
    use icu::datetime::fieldsets::{T, YMD, YMDE};

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use super::{
        DateFormatter, FormatLength, english_month_long, english_month_short, english_weekday_long,
        fallback_format_date,
    };
    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::locales;
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::{CalendarDate, CalendarSystem, Era};
    #[cfg(feature = "icu4x")]
    use crate::{Locale, locales};

    #[cfg(feature = "icu4x")]
    fn march_2024() -> CalendarDate {
        CalendarDate::new_gregorian(
            2024,
            NonZero::new(3).expect("nonzero"),
            NonZero::new(15).expect("nonzero"),
        )
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    fn japanese_reiwa_date() -> CalendarDate {
        CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(Era {
                code: String::from("reiwa"),
                display_name: String::from("Reiwa"),
            }),
            year: 1,
            month: NonZero::new(5).expect("nonzero"),
            day: NonZero::new(1).expect("nonzero"),
        }
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn short_date_formatter_formats_en_us_date() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Short);

        assert_eq!(formatter.format(&march_2024()), "3/15/24");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_length_icu_field_sets_cover_time_and_full_variants() {
        assert_eq!(
            FormatLength::Full.to_icu_full_date_field_set(),
            YMDE::long()
        );
        assert_eq!(FormatLength::Long.to_icu_date_field_set(), YMD::long());
        assert_eq!(FormatLength::Medium.to_icu_date_field_set(), YMD::medium());
        assert_eq!(FormatLength::Short.to_icu_date_field_set(), YMD::short());
        assert_eq!(FormatLength::Full.to_icu_time_field_set(), T::hms());
        assert_eq!(FormatLength::Long.to_icu_time_field_set(), T::hms());
        assert_eq!(FormatLength::Medium.to_icu_time_field_set(), T::hm());
        assert_eq!(FormatLength::Short.to_icu_time_field_set(), T::hm());
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn full_date_formatter_includes_weekday_name() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Full);

        let formatted = formatter.format(&march_2024());

        assert!(formatted.contains("Friday"));
        assert!(formatted.contains("March"));
        assert!(formatted.contains("2024"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn date_formatter_supports_representative_locales() {
        let english = DateFormatter::new(&locales::en_us(), FormatLength::Medium);

        let german = DateFormatter::new(
            &Locale::parse("de-DE").expect("locale should parse"),
            FormatLength::Medium,
        );

        let arabic = DateFormatter::new(
            &Locale::parse("ar-SA").expect("locale should parse"),
            FormatLength::Medium,
        );

        let japanese = DateFormatter::new(
            &Locale::parse("ja-JP").expect("locale should parse"),
            FormatLength::Medium,
        );

        assert_ne!(english.format(&march_2024()), german.format(&march_2024()));
        assert_ne!(english.format(&march_2024()), arabic.format(&march_2024()));
        assert_ne!(
            english.format(&march_2024()),
            japanese.format(&march_2024())
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn non_gregorian_japanese_dates_format_with_era_year() {
        let formatter = DateFormatter::new(
            &Locale::parse("en-US-u-ca-japanese").expect("locale should parse"),
            FormatLength::Long,
        );

        let date = CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(Era {
                code: String::from("reiwa"),
                display_name: String::from("Reiwa"),
            }),
            year: 1,
            month: NonZero::new(5).expect("nonzero"),
            day: NonZero::new(1).expect("nonzero"),
        };

        let formatted = formatter.format(&date);

        assert!(formatted.contains("Reiwa"));
        assert!(formatted.contains("May"));
        assert_ne!(formatted, date.to_iso8601());
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formatter_gracefully_formats_large_gregorian_years() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);
        let date = CalendarDate::new_gregorian(
            10_000,
            NonZero::new(1).expect("nonzero"),
            NonZero::new(1).expect("nonzero"),
        );

        assert_eq!(formatter.format(&date), "January 1, 10000");
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn date_formatter_debug_includes_locale_and_length() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);
        let debug = format!("{formatter:?}");

        assert!(debug.contains("DateFormatter"));
        assert!(debug.contains("en-US"));
        assert!(debug.contains("Long"));
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn fallback_format_date_covers_all_lengths_and_non_gregorian_passthrough() {
        let locale = locales::en_us();
        let gregorian = CalendarDate::new_gregorian(
            2024,
            NonZero::new(3).expect("nonzero"),
            NonZero::new(15).expect("nonzero"),
        );

        assert_eq!(
            fallback_format_date(&locale, FormatLength::Short, &gregorian),
            "3/15/24"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Medium, &gregorian),
            "Mar 15, 2024"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Long, &gregorian),
            "March 15, 2024"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Full, &gregorian),
            "Friday, March 15, 2024"
        );

        let japanese = japanese_reiwa_date();
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Long, &japanese),
            japanese.to_iso8601()
        );
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn english_fallback_helpers_cover_valid_and_invalid_inputs() {
        assert_eq!(english_month_short(1), "Jan");
        assert_eq!(english_month_short(12), "Dec");
        assert_eq!(english_month_short(0), "???");
        assert_eq!(english_month_short(13), "???");

        assert_eq!(english_month_long(1), "January");
        assert_eq!(english_month_long(12), "December");
        assert_eq!(english_month_long(0), "Unknown");
        assert_eq!(english_month_long(13), "Unknown");

        assert_eq!(english_weekday_long(crate::Weekday::Sunday), "Sunday");
        assert_eq!(english_weekday_long(crate::Weekday::Monday), "Monday");
        assert_eq!(english_weekday_long(crate::Weekday::Tuesday), "Tuesday");
        assert_eq!(english_weekday_long(crate::Weekday::Wednesday), "Wednesday");
        assert_eq!(english_weekday_long(crate::Weekday::Thursday), "Thursday");
        assert_eq!(english_weekday_long(crate::Weekday::Friday), "Friday");
        assert_eq!(english_weekday_long(crate::Weekday::Saturday), "Saturday");
    }

    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn fallback_date_formatter_keeps_api_available() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Full);

        let formatted = formatter.format(&CalendarDate::new_gregorian(
            2024,
            NonZero::new(3).expect("nonzero"),
            NonZero::new(15).expect("nonzero"),
        ));

        assert!(formatted.contains("Friday"));
        assert!(formatted.contains("March"));
    }
}

#[cfg(all(
    test,
    feature = "web-intl",
    target_arch = "wasm32",
    not(feature = "icu4x")
))]
mod web_intl_tests {
    use alloc::string::String;
    use core::num::NonZero;

    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        DateFormatter, FormatLength, js_date_from_calendar, js_date_from_ymd, js_date_is_valid,
        js_iso_year,
    };
    use crate::{CalendarDate, CalendarSystem, Locale, StubIcuProvider, locales};

    wasm_bindgen_test_configure!(run_in_browser);

    fn march_2024() -> CalendarDate {
        CalendarDate::new_gregorian(
            2024,
            NonZero::new(3).expect("nonzero"),
            NonZero::new(15).expect("nonzero"),
        )
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_formats_gregorian_dates() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Short);

        assert_eq!(formatter.format(&march_2024()), "3/15/24");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_tracks_locale_shape() {
        let english = DateFormatter::new(&locales::en_us(), FormatLength::Medium);

        let german = DateFormatter::new(
            &Locale::parse("de-DE").expect("locale should parse"),
            FormatLength::Medium,
        );

        assert_ne!(english.format(&march_2024()), german.format(&march_2024()));
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_formats_non_gregorian_dates_via_browser_calendar() {
        let locale = Locale::parse("en-US-u-ca-japanese").expect("locale should parse");

        let formatter = DateFormatter::new(&locale, FormatLength::Long);

        let date = CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(crate::Era {
                code: String::from("reiwa"),
                display_name: String::from("Reiwa"),
            }),
            year: 1,
            month: NonZero::new(5).expect("nonzero"),
            day: NonZero::new(1).expect("nonzero"),
        };

        let direct = direct_browser_format(&locale, FormatLength::Long, &date);

        let formatted = formatter.format(&date);

        assert_eq!(formatted, direct);
        assert!(formatted.contains("Reiwa"));
        assert_ne!(formatted, date.to_iso8601());
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_preserve_low_gregorian_years() {
        let date = CalendarDate::new_gregorian(
            44,
            NonZero::new(3).expect("nonzero"),
            NonZero::new(15).expect("nonzero"),
        );

        let js_date = js_date_from_ymd(44, 3, 15);
        let converted =
            js_date_from_calendar(&date).expect("Gregorian test fixture should map to a Date");

        assert_eq!(js_date.get_utc_full_year(), 44);
        assert_eq!(js_date.get_utc_month(), 2);
        assert_eq!(js_date.get_utc_date(), 15);
        assert_eq!(converted.get_utc_full_year(), 44);
        assert_eq!(converted.get_utc_month(), 2);
        assert_eq!(converted.get_utc_date(), 15);
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_gracefully_formats_large_gregorian_years() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);
        let date = CalendarDate::new_gregorian(
            10_000,
            NonZero::new(1).expect("nonzero"),
            NonZero::new(1).expect("nonzero"),
        );

        let formatted = formatter.format(&date);

        assert_eq!(formatted, "January 1, 10000");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_support_pre_ce_gregorian_dates() {
        let js_date = js_date_from_ymd(-1, 1, 1);

        assert_eq!(js_date.to_iso_string(), "-000001-01-01T12:00:00.000Z");
        assert_eq!(js_iso_year(-1), "-000001");
        assert_eq!(js_iso_year(0), "0000");
        assert_eq!(js_iso_year(44), "0044");
        assert_eq!(js_iso_year(10_000), "+010000");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_preserve_astronomical_gregorian_years_from_non_gregorian_input() {
        let provider = StubIcuProvider;
        let buddhist = CalendarDate::new(&provider, CalendarSystem::Buddhist, None, 1, 1, 1)
            .expect("Buddhist date should validate");

        let converted = js_date_from_calendar(&buddhist)
            .expect("Buddhist fixture should map to a browser Date");

        assert_eq!(converted.to_iso_string(), "-000542-01-01T12:00:00.000Z");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_falls_back_for_js_out_of_range_gregorian_years() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);
        let date = CalendarDate::new_gregorian(
            1_000_000,
            NonZero::new(1).expect("nonzero"),
            NonZero::new(1).expect("nonzero"),
        );

        assert!(js_date_is_valid(&js_date_from_ymd(10_000, 1, 1)));
        assert!(!js_date_is_valid(&js_date_from_ymd(1_000_000, 1, 1)));
        assert!(js_date_from_calendar(&date).is_none());
        assert_eq!(formatter.format(&date), "January 1, 1000000");
    }

    fn direct_browser_format(locale: &Locale, length: FormatLength, date: &CalendarDate) -> String {
        use js_sys::{
            Array, Function,
            Intl::{DateTimeFormat as JsDateTimeFormat, DateTimeFormatOptions},
        };
        use wasm_bindgen::JsValue;

        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let options = DateTimeFormatOptions::new();

        options.set_date_style(length.to_js_date_style());
        options.set_time_zone("UTC");

        let formatter = JsDateTimeFormat::new(&locales, options.as_ref());

        let format: Function = formatter.format();

        let js_date =
            js_date_from_calendar(date).expect("test fixtures should map to a browser Date");

        format
            .call1(&JsValue::UNDEFINED, js_date.as_ref())
            .expect("Intl.DateTimeFormat.format should not throw for a valid Date")
            .as_string()
            .unwrap_or_default()
    }
}
