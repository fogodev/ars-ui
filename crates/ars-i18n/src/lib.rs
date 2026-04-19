//! Internationalization types for locale, number formatting, locale-aware
//! string sorting, text direction, layout orientation, plural rules,
//! locale-aware case mapping, and logical-to-physical layout geometry.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a locale-aware [`NumberFormatter`], a
//! [`Direction`] enum for LTR/RTL text flow, a [`StringCollator`] for
//! locale-aware sorting, an [`Orientation`] enum for horizontal/vertical
//! layout axes, RTL-aware layout geometry types ([`LogicalSide`],
//! [`PhysicalSide`], [`LogicalRect`], [`PhysicalRect`]), plural and ordinal
//! helpers, locale-aware [`to_uppercase`] and [`to_lowercase`] helpers, and
//! the [`IcuProvider`] trait for calendar/locale data abstraction.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "icu4x", feature = "web-intl"))]
compile_error!("features `icu4x` and `web-intl` are mutually exclusive");

extern crate alloc;

use alloc::{format, string::String};
use core::num::NonZero;

mod bidi;
mod calendar;
#[cfg(any(feature = "icu4x", feature = "web-intl"))]
mod case;
mod collation;
mod date;
#[cfg(feature = "std")]
mod detect;
mod layout;
mod locale;
mod locale_stack;
mod number;
mod plural;
mod provider;
mod relative_time;
mod translate;
mod weekday;

pub use bidi::{IsolateDirection, isolate_text_safe};
pub use calendar::{
    Buddhist, CalendarConversionError, CalendarDate, CalendarError, CalendarKind, CalendarMetadata,
    CalendarSystem, CalendarTypeError, Chinese, Coptic, Dangi, DateError, DateRange,
    DirectDayArithmetic, DirectWeekdayComputation, Era, Ethiopic, EthiopicAmeteAlem, Gregorian,
    Hebrew, HourCycle, Indian, Islamic, IslamicCivil, IslamicUmmAlQura, Japanese, JapaneseEra,
    Month, Persian, Roc, Time, TypedCalendarDate, WeekInfo,
};
#[cfg(any(feature = "icu4x", feature = "web-intl"))]
pub use case::{to_lowercase, to_uppercase};
pub use collation::{CollationFormat, CollationOptions, CollationStrength, StringCollator};
pub use date::{DateFormatter, FormatLength};
#[cfg(feature = "std")]
pub use detect::locale_from_accept_language;
pub use layout::{LogicalRect, LogicalSide, PhysicalRect, PhysicalSide};
pub use locale::{Locale, LocaleParseError, locales};
pub use locale_stack::LocaleStack;
pub use number::{
    CurrencyCode, MeasureUnit, NumberFormatOptions, NumberFormatter, NumberStyle, RoundingMode,
    SignDisplay, UnitDisplay, decimal_and_group_separators, normalize_digits, parse_locale_number,
};
#[cfg(feature = "icu4x")]
pub use plural::{DefaultPluralRules, Icu4xPluralRules};
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
pub use plural::{DefaultPluralRules, JsIntlPluralRules};
pub use plural::{
    Plural, PluralCategory, PluralRuleType, PluralRulesFormat, format_plural, plural_category,
    select_plural,
};
#[cfg(feature = "icu4x")]
pub use provider::Icu4xProvider;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub use provider::WebIntlProvider;
pub use provider::{StubIcuProvider, default_provider};
pub use relative_time::{NumericOption, RelativeTimeFormatter};
pub use translate::Translate;
pub use weekday::Weekday;

/// Text and layout direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// Left-to-right text direction (default for most Latin-script locales).
    #[default]
    Ltr,
    /// Right-to-left text direction (used by Arabic, Hebrew, and related scripts).
    Rtl,
    /// Automatic direction detection (resolved by the platform adapter before use).
    Auto,
}

impl Direction {
    /// CSS `direction` value.
    #[must_use]
    pub const fn as_css(&self) -> &'static str {
        match self {
            Direction::Ltr => "ltr",
            Direction::Rtl => "rtl",
            Direction::Auto => "auto",
        }
    }

    /// HTML `dir` attribute value.
    #[must_use]
    pub const fn as_html_attr(&self) -> &'static str {
        self.as_css()
    }

    /// Returns `true` if this direction is right-to-left.
    #[must_use]
    pub fn is_rtl(&self) -> bool {
        *self == Direction::Rtl
    }

    /// Flip a side for RTL.
    ///
    /// In RTL, "start" maps to right, "end" maps to left.
    #[must_use]
    pub fn inline_start_is_right(&self) -> bool {
        self.is_rtl()
    }

    /// Resolve `Auto` to a concrete direction using the given fallback.
    ///
    /// `Ltr` and `Rtl` pass through unchanged; `Auto` returns `fallback`.
    #[must_use]
    pub const fn resolve(self, fallback: ResolvedDirection) -> ResolvedDirection {
        match self {
            Direction::Ltr => ResolvedDirection::Ltr,
            Direction::Rtl => ResolvedDirection::Rtl,
            Direction::Auto => fallback,
        }
    }
}

/// A direction that has been resolved to a concrete value — only `Ltr` or `Rtl`.
///
/// Functions that require a resolved direction (layout conversion, arrow key
/// mapping, placement resolution) accept this type instead of [`Direction`],
/// making it a compile error to pass an unresolved `Auto` value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResolvedDirection {
    /// Left-to-right.
    #[default]
    Ltr,
    /// Right-to-left.
    Rtl,
}

impl ResolvedDirection {
    /// CSS `direction` value.
    #[must_use]
    pub const fn as_css(self) -> &'static str {
        match self {
            ResolvedDirection::Ltr => "ltr",
            ResolvedDirection::Rtl => "rtl",
        }
    }

    /// HTML `dir` attribute value.
    #[must_use]
    pub const fn as_html_attr(self) -> &'static str {
        self.as_css()
    }

    /// Returns `true` if this direction is right-to-left.
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, ResolvedDirection::Rtl)
    }

    /// Returns `true` when inline-start maps to the right side.
    #[must_use]
    pub const fn inline_start_is_right(self) -> bool {
        self.is_rtl()
    }
}

impl From<ResolvedDirection> for Direction {
    fn from(resolved: ResolvedDirection) -> Self {
        match resolved {
            ResolvedDirection::Ltr => Direction::Ltr,
            ResolvedDirection::Rtl => Direction::Rtl,
        }
    }
}

/// The layout axis for components that arrange children along a single direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Children are laid out along the horizontal axis (default).
    #[default]
    Horizontal,
    /// Children are laid out along the vertical axis.
    Vertical,
}

// ────────────────────────────────────────────────────────────────────
// ICU data provider abstraction
// ────────────────────────────────────────────────────────────────────

/// Trait abstracting ICU4X data provider for calendar/locale operations.
///
/// Production uses `Icu4xProvider` with CLDR data; tests and non-date-time
/// components use [`StubIcuProvider`]. The trait is object-safe so it can be
/// stored as `Arc<dyn IcuProvider>` in `Env`.
///
/// Requires `Send + Sync` on every target so adapters and shared ownership
/// abstractions can treat ICU providers uniformly across native and wasm
/// builds.
pub trait IcuProvider: Send + Sync + 'static {
    /// Short weekday label (abbreviated format): `Mo`, `Tu`, `We`, ...
    fn weekday_short_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday => String::from("Su"),
            Weekday::Monday => String::from("Mo"),
            Weekday::Tuesday => String::from("Tu"),
            Weekday::Wednesday => String::from("We"),
            Weekday::Thursday => String::from("Th"),
            Weekday::Friday => String::from("Fr"),
            Weekday::Saturday => String::from("Sa"),
        }
    }

    /// Long weekday label (wide format): `Monday`, `Tuesday`, ...
    fn weekday_long_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday => String::from("Sunday"),
            Weekday::Monday => String::from("Monday"),
            Weekday::Tuesday => String::from("Tuesday"),
            Weekday::Wednesday => String::from("Wednesday"),
            Weekday::Thursday => String::from("Thursday"),
            Weekday::Friday => String::from("Friday"),
            Weekday::Saturday => String::from("Saturday"),
        }
    }

    /// Full month name in the locale.
    fn month_long_name(&self, month: u8, _locale: &Locale) -> String {
        match month {
            1 => String::from("January"),
            2 => String::from("February"),
            3 => String::from("March"),
            4 => String::from("April"),
            5 => String::from("May"),
            6 => String::from("June"),
            7 => String::from("July"),
            8 => String::from("August"),
            9 => String::from("September"),
            10 => String::from("October"),
            11 => String::from("November"),
            12 => String::from("December"),
            _ => String::from("Unknown"),
        }
    }

    /// Localized day-period label.
    fn day_period_label(&self, is_pm: bool, _locale: &Locale) -> String {
        if is_pm {
            String::from("PM")
        } else {
            String::from("AM")
        }
    }

    /// Reverse-maps a typed character to AM/PM.
    fn day_period_from_char(&self, ch: char, _locale: &Locale) -> Option<bool> {
        match ch.to_ascii_lowercase() {
            'a' => Some(false),
            'p' => Some(true),
            _ => None,
        }
    }

    /// Formats a numeric segment with locale-appropriate zero-padding.
    fn format_segment_digits(
        &self,
        value: u32,
        min_digits: NonZero<u8>,
        _locale: &Locale,
    ) -> String {
        format!("{value:0width$}", width = usize::from(min_digits.get()))
    }

    /// Maximum number of months in a year for the given calendar and year.
    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, _era: Option<&str>) -> u8 {
        if let Some(months) = calendar::bounded_months_in_year(*calendar, year, _era) {
            return months;
        }

        #[cfg(any(feature = "icu4x", feature = "web-intl"))]
        if let Some(months) = calendar::internal::months_in_year(year, *calendar, _era) {
            return months;
        }

        match calendar {
            CalendarSystem::Hebrew => {
                let cycle_year = year.rem_euclid(19);
                if [3, 6, 8, 11, 14, 17, 0].contains(&cycle_year) {
                    13
                } else {
                    12
                }
            }
            CalendarSystem::Ethiopic
            | CalendarSystem::EthiopicAmeteAlem
            | CalendarSystem::Coptic => 13,
            _ => 12,
        }
    }

    /// Days in a specific month for the given calendar, year, and month.
    fn days_in_month(
        &self,
        _calendar: &CalendarSystem,
        year: i32,
        month: u8,
        _era: Option<&str>,
    ) -> u8 {
        if let Some(days) = calendar::bounded_days_in_month(*_calendar, year, month, _era) {
            return days;
        }

        #[cfg(any(feature = "icu4x", feature = "web-intl"))]
        if let Some(days) = calendar::internal::days_in_month(year, month, *_calendar, _era) {
            return days;
        }

        if matches!(_calendar, CalendarSystem::Japanese) {
            return 0;
        }

        if matches!(
            _calendar,
            CalendarSystem::Coptic | CalendarSystem::Ethiopic | CalendarSystem::EthiopicAmeteAlem
        ) {
            return calendar::coptic_like_days_in_month(year, month);
        }

        calendar::gregorian_days_in_month(year, month)
    }

    /// Returns the calendar's default era when callers omit one.
    fn default_era(&self, calendar: &CalendarSystem) -> Option<Era> {
        calendar::default_era_for(*calendar)
    }

    /// Returns the maximum year value in the date's current era, if bounded.
    fn years_in_era(&self, date: &CalendarDate) -> Option<i32> {
        calendar::years_in_era(date)
    }

    /// Returns the minimum allowed month ordinal for the date's current year.
    fn minimum_month_in_year(&self, date: &CalendarDate) -> u8 {
        calendar::minimum_month_in_year(date)
    }

    /// Returns the minimum allowed day ordinal for the date's current month.
    fn minimum_day_in_month(&self, date: &CalendarDate) -> u8 {
        calendar::minimum_day_in_month(date)
    }

    /// Preferred hour cycle for the locale.
    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        match locale.language() {
            "en" | "ko" => HourCycle::H12,
            _ => HourCycle::H23,
        }
    }

    /// First day of the week for the locale.
    fn first_day_of_week(&self, locale: &Locale) -> Weekday {
        WeekInfo::for_locale(locale).first_day
    }

    /// Converts a public calendar date into the target calendar system.
    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        if date.calendar == target {
            return date.clone();
        }

        #[cfg(any(feature = "icu4x", feature = "web-intl"))]
        {
            let Ok(internal) = calendar::internal::CalendarDate::try_from(date) else {
                return date.clone();
            };
            let converted = internal.to_calendar(target);

            CalendarDate {
                calendar: target,
                era: converted
                    .era()
                    .filter(|_| target.has_custom_eras())
                    .map(|code| Era {
                        code: code.clone(),
                        display_name: code,
                    }),
                year: converted.year(),
                month: NonZero::new(converted.month())
                    .expect("internal calendar conversion yields a 1-based month"),
                day: NonZero::new(converted.day())
                    .expect("internal calendar conversion yields a 1-based day"),
            }
        }

        #[cfg(not(any(feature = "icu4x", feature = "web-intl")))]
        panic!(
            "StubIcuProvider does not support non-Gregorian calendar conversion; use Icu4xProvider"
        );
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec, vec::Vec};
    use core::num::NonZero;
    #[cfg(all(feature = "std", not(any(feature = "icu4x", feature = "web-intl"))))]
    use std::panic::catch_unwind;

    use super::*;

    #[test]
    fn direction_default_is_ltr() {
        assert_eq!(Direction::default(), Direction::Ltr);
    }

    #[test]
    fn direction_as_css_values() {
        assert_eq!(Direction::Ltr.as_css(), "ltr");
        assert_eq!(Direction::Rtl.as_css(), "rtl");
        assert_eq!(Direction::Auto.as_css(), "auto");
    }

    #[test]
    fn direction_as_html_attr_matches_css() {
        assert_eq!(Direction::Ltr.as_html_attr(), "ltr");
        assert_eq!(Direction::Rtl.as_html_attr(), "rtl");
        assert_eq!(Direction::Auto.as_html_attr(), "auto");
    }

    #[test]
    fn direction_is_rtl() {
        assert!(!Direction::Ltr.is_rtl());
        assert!(Direction::Rtl.is_rtl());
        assert!(!Direction::Auto.is_rtl());
    }

    #[test]
    fn direction_inline_start_is_right() {
        assert!(!Direction::Ltr.inline_start_is_right());
        assert!(Direction::Rtl.inline_start_is_right());
        assert!(!Direction::Auto.inline_start_is_right());
    }

    // ── ResolvedDirection tests ──────────────────────────────────

    #[test]
    fn resolved_direction_default_is_ltr() {
        assert_eq!(ResolvedDirection::default(), ResolvedDirection::Ltr);
    }

    #[test]
    fn resolved_direction_is_rtl() {
        assert!(!ResolvedDirection::Ltr.is_rtl());
        assert!(ResolvedDirection::Rtl.is_rtl());
    }

    #[test]
    fn resolved_direction_as_css() {
        assert_eq!(ResolvedDirection::Ltr.as_css(), "ltr");
        assert_eq!(ResolvedDirection::Rtl.as_css(), "rtl");
    }

    #[test]
    fn resolved_direction_as_html_attr() {
        assert_eq!(ResolvedDirection::Ltr.as_html_attr(), "ltr");
        assert_eq!(ResolvedDirection::Rtl.as_html_attr(), "rtl");
    }

    #[test]
    fn resolved_direction_inline_start_is_right() {
        assert!(!ResolvedDirection::Ltr.inline_start_is_right());
        assert!(ResolvedDirection::Rtl.inline_start_is_right());
    }

    #[test]
    fn direction_resolve_passes_through_ltr_rtl() {
        assert_eq!(
            Direction::Ltr.resolve(ResolvedDirection::Rtl),
            ResolvedDirection::Ltr,
        );
        assert_eq!(
            Direction::Rtl.resolve(ResolvedDirection::Ltr),
            ResolvedDirection::Rtl,
        );
    }

    #[test]
    fn direction_resolve_auto_uses_fallback() {
        assert_eq!(
            Direction::Auto.resolve(ResolvedDirection::Ltr),
            ResolvedDirection::Ltr,
        );
        assert_eq!(
            Direction::Auto.resolve(ResolvedDirection::Rtl),
            ResolvedDirection::Rtl,
        );
    }

    #[test]
    fn direction_from_resolved() {
        assert_eq!(Direction::from(ResolvedDirection::Ltr), Direction::Ltr);
        assert_eq!(Direction::from(ResolvedDirection::Rtl), Direction::Rtl);
    }

    // ── Locale tests ───────────────────────────────────────────────

    #[test]
    fn parse_valid_locales() {
        for locale in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "ja-JP-u-ca-japanese",
            "en-US-u-fw-mon",
            "ar",
            "sr-Latn-RS",
        ] {
            assert!(Locale::parse(locale).is_ok(), "{locale} should parse");
        }
    }

    #[test]
    fn parse_invalid_locales() {
        for locale in [
            "",
            "e",
            "123",
            "-en",
            "en-",
            "en--US",
            "abcdefghi",
            "en-abcdefghi",
            "en-US!",
            "en-\u{00DC}",
        ] {
            assert!(
                Locale::parse(locale).is_err(),
                "{locale} should be rejected"
            );
        }
    }

    #[test]
    fn locale_accessors_roundtrip() {
        let locale = Locale::parse("zh-Hans-CN").expect("zh-Hans-CN is valid");
        assert_eq!(locale.to_bcp47(), "zh-Hans-CN");
        assert_eq!(locale.language(), "zh");
        assert_eq!(locale.script(), Some("Hans"));
        assert_eq!(locale.region(), Some("CN"));
    }

    #[test]
    fn locale_extensions_are_exposed() {
        let locale =
            Locale::parse("ja-JP-u-ca-japanese-fw-mon").expect("locale with unicode extensions");
        assert_eq!(locale.calendar_extension(), Some("japanese"));
        assert_eq!(locale.first_day_of_week_extension(), Some(Weekday::Monday));
    }

    #[test]
    fn locale_accessors_return_none_when_subtags_are_absent() {
        let locale = Locale::parse("en").expect("en is valid");
        assert_eq!(locale.script(), None);
        assert_eq!(locale.region(), None);
        assert_eq!(locale.calendar_extension(), None);
        assert_eq!(locale.first_day_of_week_extension(), None);
    }

    #[test]
    fn locale_from_langid_roundtrips_to_bcp47() {
        let langid = "en-US"
            .parse::<icu::locale::LanguageIdentifier>()
            .expect("en-US langid is valid");
        let locale = Locale::from_langid(langid);
        assert_eq!(locale.to_bcp47(), "en-US");
    }

    #[test]
    fn locale_to_data_locale_roundtrips_to_string() {
        let locale = Locale::parse("en-US").expect("en-US is valid");
        assert_eq!(locale.to_data_locale().to_string(), "en-US");
    }

    #[test]
    fn locale_ordering_is_lexical_by_bcp47() {
        let mut locales = vec![
            Locale::parse("fr-FR").expect("fr-FR is valid"),
            Locale::parse("de-DE").expect("de-DE is valid"),
            Locale::parse("en-US").expect("en-US is valid"),
        ];
        locales.sort();
        let sorted = locales
            .into_iter()
            .map(|locale| locale.to_bcp47())
            .collect::<Vec<_>>();
        assert_eq!(sorted, vec!["de-DE", "en-US", "fr-FR"]);
    }

    #[test]
    fn locale_direction_detects_rtl_scripts() {
        for locale in ["ar", "he", "fa", "ar-EG"] {
            let locale = Locale::parse(locale).expect("locale should parse");
            assert_eq!(
                locale.direction(),
                ResolvedDirection::Rtl,
                "{} should be RTL",
                locale.to_bcp47()
            );
            assert!(locale.is_rtl(), "{} should be RTL", locale.to_bcp47());
        }
    }

    #[test]
    fn locale_direction_infers_scripts_for_rtl_languages() {
        for locale in ["dv", "nqo", "pa-PK", "ku-IQ", "yi", "ks"] {
            let locale = Locale::parse(locale).expect("locale should parse");
            assert_eq!(
                locale.direction(),
                ResolvedDirection::Rtl,
                "{} should be RTL",
                locale.to_bcp47()
            );
            assert!(locale.is_rtl(), "{} should be RTL", locale.to_bcp47());
        }
    }

    #[test]
    fn locale_direction_defaults_to_ltr_when_not_rtl() {
        let locale = Locale::parse("en-US").expect("en-US is valid");
        assert_eq!(locale.direction(), ResolvedDirection::Ltr);
        assert!(!locale.is_rtl());
    }

    #[test]
    fn locale_parse_error_display() {
        let err = Locale::parse("").unwrap_err();
        assert_eq!(
            err.to_string(),
            "ars-ui locale parse error: The given language subtag is invalid"
        );
    }

    #[test]
    fn locale_prelude_constructors_return_expected_tags() {
        let constructors = [
            (locales::en(), "en"),
            (locales::en_us(), "en-US"),
            (locales::en_gb(), "en-GB"),
            (locales::ar(), "ar"),
            (locales::ar_sa(), "ar-SA"),
            (locales::ar_eg(), "ar-EG"),
            (locales::he(), "he"),
            (locales::fa(), "fa"),
            (locales::de(), "de"),
            (locales::de_de(), "de-DE"),
            (locales::fr(), "fr-FR"),
            (locales::ja(), "ja"),
            (locales::ja_jp(), "ja-JP"),
            (locales::zh_hans(), "zh-Hans"),
            (locales::ko(), "ko"),
        ];

        for (locale, expected) in constructors {
            assert_eq!(locale.to_bcp47(), expected);
        }
    }

    #[test]
    fn locale_wrapper_helpers_delegate_to_stub_provider_defaults() {
        let locale = Locale::parse("en-US").expect("en-US is valid");
        let provider = StubIcuProvider;

        assert_eq!(locale.first_day_of_week(&provider), Weekday::Sunday);
        assert_eq!(locale.hour_cycle(&provider), HourCycle::H12);
    }

    #[test]
    fn orientation_default_is_horizontal() {
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }

    #[test]
    fn stub_icu_provider_default_helpers_cover_fallback_paths() {
        let provider = StubIcuProvider;
        let locale = Locale::parse("en-US").expect("en-US is valid");

        for (weekday, short, long) in [
            (Weekday::Sunday, "Su", "Sunday"),
            (Weekday::Monday, "Mo", "Monday"),
            (Weekday::Tuesday, "Tu", "Tuesday"),
            (Weekday::Wednesday, "We", "Wednesday"),
            (Weekday::Thursday, "Th", "Thursday"),
            (Weekday::Friday, "Fr", "Friday"),
            (Weekday::Saturday, "Sa", "Saturday"),
        ] {
            assert_eq!(provider.weekday_short_label(weekday, &locale), short);
            assert_eq!(provider.weekday_long_label(weekday, &locale), long);
        }

        for (month, expected) in [
            (1, "January"),
            (2, "February"),
            (3, "March"),
            (4, "April"),
            (5, "May"),
            (6, "June"),
            (7, "July"),
            (8, "August"),
            (9, "September"),
            (10, "October"),
            (11, "November"),
            (12, "December"),
            (13, "Unknown"),
        ] {
            assert_eq!(provider.month_long_name(month, &locale), expected);
        }

        assert_eq!(provider.day_period_label(false, &locale), "AM");
        assert_eq!(provider.day_period_label(true, &locale), "PM");
        assert_eq!(provider.day_period_from_char('a', &locale), Some(false));
        assert_eq!(provider.day_period_from_char('A', &locale), Some(false));
        assert_eq!(provider.day_period_from_char('p', &locale), Some(true));
        assert_eq!(provider.day_period_from_char('P', &locale), Some(true));
        assert_eq!(provider.day_period_from_char('x', &locale), None);

        assert_eq!(
            provider.format_segment_digits(7, NonZero::new(2).expect("nonzero"), &locale),
            "07"
        );

        assert_eq!(
            provider.max_months_in_year(&CalendarSystem::Hebrew, 5784, None),
            13
        );
        assert_eq!(
            provider.max_months_in_year(&CalendarSystem::Hebrew, 5785, None),
            12
        );
        #[cfg(any(feature = "icu4x", feature = "web-intl"))]
        assert_eq!(
            provider.max_months_in_year(&CalendarSystem::Dangi, 2024, None),
            calendar::internal::months_in_year(2024, CalendarSystem::Dangi, None)
                .expect("ICU4X should resolve Dangi month counts")
        );
        #[cfg(not(any(feature = "icu4x", feature = "web-intl")))]
        assert_eq!(
            provider.max_months_in_year(&CalendarSystem::Dangi, 2024, None),
            12
        );
        assert_eq!(
            provider.max_months_in_year(&CalendarSystem::Gregorian, 2024, None),
            12
        );

        assert_eq!(
            provider.days_in_month(&CalendarSystem::Gregorian, 2024, 2, None),
            29
        );
        assert_eq!(
            provider.days_in_month(&CalendarSystem::Gregorian, 2023, 2, None),
            28
        );
        assert_eq!(
            provider.days_in_month(&CalendarSystem::Japanese, 1, 5, Some("reiwa")),
            31
        );

        assert_eq!(
            provider.default_era(&CalendarSystem::Japanese),
            Some(Era {
                code: "reiwa".to_string(),
                display_name: "Reiwa".to_string(),
            })
        );
        assert_eq!(provider.default_era(&CalendarSystem::Gregorian), None);

        let japanese = CalendarDate::new(
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
        .expect("Japanese date should validate");
        assert_eq!(provider.years_in_era(&japanese), Some(31));
        assert_eq!(provider.minimum_month_in_year(&japanese), 1);
        assert_eq!(provider.minimum_day_in_month(&japanese), 8);

        let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 3, 15)
            .expect("Gregorian date should validate");
        assert_eq!(provider.minimum_month_in_year(&gregorian), 1);
        assert_eq!(provider.minimum_day_in_month(&gregorian), 1);

        assert_eq!(provider.hour_cycle(&locale), HourCycle::H12);
        let german = Locale::parse("de-DE").expect("de-DE is valid");
        assert_eq!(provider.hour_cycle(&german), HourCycle::H23);
        assert_eq!(provider.first_day_of_week(&locale), Weekday::Sunday);
        assert_eq!(provider.first_day_of_week(&german), Weekday::Monday);

        assert_eq!(
            provider.convert_date(&gregorian, CalendarSystem::Gregorian),
            gregorian
        );
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_icu_provider_convert_date_supports_non_identity_conversions_when_internal_calendar_is_available()
     {
        let provider = StubIcuProvider;
        let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 3, 15)
            .expect("Gregorian date should validate");
        let japanese = provider.convert_date(&gregorian, CalendarSystem::Japanese);
        let round_trip = provider.convert_date(&japanese, CalendarSystem::Gregorian);

        assert_eq!(
            japanese,
            CalendarDate::new(
                &provider,
                CalendarSystem::Japanese,
                Some(Era {
                    code: String::from("reiwa"),
                    display_name: String::from("reiwa"),
                }),
                6,
                3,
                15,
            )
            .expect("converted Japanese date should validate")
        );
        assert_eq!(round_trip, gregorian);
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_icu_provider_convert_date_preserves_bce_gregorian_years() {
        let provider = StubIcuProvider;
        let buddhist = CalendarDate::new(&provider, CalendarSystem::Buddhist, None, 1, 1, 1)
            .expect("Buddhist date should validate");

        let gregorian = provider.convert_date(&buddhist, CalendarSystem::Gregorian);

        assert_eq!(
            gregorian,
            CalendarDate {
                calendar: CalendarSystem::Gregorian,
                era: None,
                year: -542,
                month: NonZero::new(1).expect("one is non-zero"),
                day: NonZero::new(1).expect("one is non-zero"),
            }
        );
        assert_eq!(
            buddhist.to_calendar(&provider, CalendarSystem::Gregorian),
            gregorian
        );
        assert_eq!(
            provider.convert_date(&gregorian, CalendarSystem::Buddhist),
            buddhist
        );
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_icu_provider_convert_date_falls_back_for_inputs_outside_internal_bridge_range() {
        let provider = StubIcuProvider;
        let gregorian = CalendarDate::new_gregorian(
            10_000,
            NonZero::new(1).expect("one is non-zero"),
            NonZero::new(1).expect("one is non-zero"),
        );

        assert_eq!(
            provider.convert_date(&gregorian, CalendarSystem::Japanese),
            gregorian
        );
        assert_eq!(
            gregorian.to_calendar(&provider, CalendarSystem::Japanese),
            gregorian
        );
    }

    #[cfg(all(feature = "std", not(any(feature = "icu4x", feature = "web-intl"))))]
    #[test]
    fn stub_icu_provider_convert_date_panics_for_non_identity_conversions_without_calendar_backend()
    {
        let provider = StubIcuProvider;
        let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 3, 15)
            .expect("Gregorian date should validate");

        let panic = catch_unwind(|| provider.convert_date(&gregorian, CalendarSystem::Japanese))
            .expect_err("stub provider should panic for unsupported conversion");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&'static str>().copied())
            .expect("panic payload should be a string");

        assert_eq!(
            message,
            "StubIcuProvider does not support non-Gregorian calendar conversion; use Icu4xProvider"
        );
    }

    // ── Weekday tests ──────────────────────────────────────────────

    #[test]
    fn weekday_from_sunday_zero_wraps() {
        assert_eq!(Weekday::from_sunday_zero(0), Weekday::Sunday);
        assert_eq!(Weekday::from_sunday_zero(1), Weekday::Monday);
        assert_eq!(Weekday::from_sunday_zero(6), Weekday::Saturday);
        assert_eq!(Weekday::from_sunday_zero(7), Weekday::Sunday);
    }

    #[test]
    fn weekday_from_iso_8601_validates_range() {
        let cases = [
            (1, Some(Weekday::Monday)),
            (2, Some(Weekday::Tuesday)),
            (3, Some(Weekday::Wednesday)),
            (4, Some(Weekday::Thursday)),
            (5, Some(Weekday::Friday)),
            (6, Some(Weekday::Saturday)),
            (7, Some(Weekday::Sunday)),
        ];

        for (input, expected) in cases {
            assert_eq!(Weekday::from_iso_8601(input), expected);
        }

        assert_eq!(Weekday::from_iso_8601(0), None);
        assert_eq!(Weekday::from_iso_8601(8), None);
    }

    #[test]
    fn weekday_from_icu_str_matches_bcp47_values() {
        assert_eq!(Weekday::from_icu_str("mon"), Some(Weekday::Monday));
        assert_eq!(Weekday::from_bcp47_fw("sun"), Some(Weekday::Sunday));
        assert_eq!(Weekday::from_icu_str("bad"), None);
    }

    #[test]
    fn weekday_from_icu_str_covers_all_named_variants() {
        let cases = [
            ("mon", Weekday::Monday),
            ("tue", Weekday::Tuesday),
            ("wed", Weekday::Wednesday),
            ("thu", Weekday::Thursday),
            ("fri", Weekday::Friday),
            ("sat", Weekday::Saturday),
            ("sun", Weekday::Sunday),
        ];

        for (input, expected) in cases {
            assert_eq!(Weekday::from_icu_str(input), Some(expected));
            assert_eq!(Weekday::from_bcp47_fw(input), Some(expected));
        }
    }

    #[test]
    fn weekday_from_icu_weekday_covers_all_variants() {
        let cases = [
            (icu::calendar::types::Weekday::Monday, Weekday::Monday),
            (icu::calendar::types::Weekday::Tuesday, Weekday::Tuesday),
            (icu::calendar::types::Weekday::Wednesday, Weekday::Wednesday),
            (icu::calendar::types::Weekday::Thursday, Weekday::Thursday),
            (icu::calendar::types::Weekday::Friday, Weekday::Friday),
            (icu::calendar::types::Weekday::Saturday, Weekday::Saturday),
            (icu::calendar::types::Weekday::Sunday, Weekday::Sunday),
        ];

        for (input, expected) in cases {
            assert_eq!(Weekday::from_icu_weekday(input), expected);
        }
    }

    // ── BiDi isolation tests ───────────────────────────────────────

    #[test]
    fn isolate_text_safe_returns_empty_for_empty_input() {
        assert_eq!(isolate_text_safe("", IsolateDirection::FirstStrong), "");
    }

    #[test]
    fn isolate_text_safe_wraps_text_with_ltr_marks() {
        assert_eq!(
            isolate_text_safe("hello", IsolateDirection::Ltr),
            "\u{2066}hello\u{2069}"
        );
    }

    #[test]
    fn isolate_text_safe_wraps_text_with_rtl_marks() {
        assert_eq!(
            isolate_text_safe("مرحبا", IsolateDirection::Rtl),
            "\u{2067}مرحبا\u{2069}"
        );
    }

    #[test]
    fn isolate_text_safe_wraps_text_with_first_strong_marks() {
        assert_eq!(
            isolate_text_safe("abc", IsolateDirection::FirstStrong),
            "\u{2068}abc\u{2069}"
        );
    }

    #[test]
    fn isolate_text_safe_preserves_zwj_sequences() {
        let family = "👨‍👩‍👧";
        let isolated = isolate_text_safe(family, IsolateDirection::FirstStrong);
        let stripped = isolated
            .strip_prefix('\u{2068}')
            .expect("must start with FSI")
            .strip_suffix('\u{2069}')
            .expect("must end with PDI");
        assert_eq!(stripped, family);
    }
}
