//! Internationalization types for locale, number formatting, locale-aware
//! string sorting, text direction, layout orientation, plural rules,
//! locale-aware case mapping, and logical-to-physical layout geometry.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a locale-aware [`number::Formatter`], a
//! [`Direction`] enum for LTR/RTL text flow, a [`StringCollator`] for
//! locale-aware sorting, an [`Orientation`] enum for horizontal/vertical
//! layout axes, RTL-aware layout geometry types ([`LogicalSide`],
//! [`PhysicalSide`], [`LogicalRect`], [`PhysicalRect`]), plural and ordinal
//! helpers, locale-aware [`to_uppercase`] and [`to_lowercase`] helpers, and
//! the [`IntlBackend`] trait for calendar/locale data abstraction.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "icu4x", feature = "web-intl"))]
compile_error!("features `icu4x` and `web-intl` are mutually exclusive");

extern crate alloc;

use alloc::string::{String, ToString};
use core::num::NonZero;

mod bidi;
pub mod calendar;
#[cfg(any(feature = "icu4x", feature = "web-intl"))]
mod case;
mod collation;
mod date;
#[cfg(feature = "std")]
mod detect;
mod layout;
mod locale;
mod locale_stack;
pub mod number;
mod plural;
mod provider;
mod relative_time;
mod text;
mod translate;
mod weekday;

pub use bidi::{IsolateDirection, isolate_text_safe};
pub use calendar::{
    Buddhist, CalendarConversionError, CalendarDate, CalendarDateFields, CalendarDateTime,
    CalendarError, CalendarKind, CalendarMetadata, CalendarSystem, CalendarTypeError, Chinese,
    Coptic, Dangi, DateDuration, DateError, DateField, DateRange, DateTimeDuration, DateTimeField,
    DateValue, DirectDayArithmetic, DirectWeekdayComputation, Disambiguation, Era, Ethiopic,
    EthiopicAmeteAlem, Gregorian, Hebrew, HourCycle, Indian, IslamicCivil, IslamicUmmAlQura,
    Japanese, JapaneseEra, Month, MonthCode, Persian, Roc, Time, TimeDuration, TimeField,
    TimeFields, TimeZoneId, TypedCalendarDate, WeekInfo, to_calendar_date_time,
};
#[cfg(feature = "std")]
pub use calendar::{to_zoned, to_zoned_date_time};
#[cfg(any(feature = "icu4x", feature = "web-intl"))]
pub use case::{to_lowercase, to_uppercase};
pub use collation::{CollationFormat, CollationOptions, CollationStrength, StringCollator};
pub use date::{
    DateFormatter, DateFormatterOptions, DateFormatterPart, DateFormatterPartKind,
    DateRangeFormatterPart, DateRangePartSource, FormatLength, MonthFormat, NumericWidth,
    ResolvedDateFormatterOptions, TextWidth, TimeZoneNameFormat,
};
#[cfg(feature = "std")]
pub use detect::locale_from_accept_language;
pub use layout::{LogicalRect, LogicalSide, PhysicalRect, PhysicalSide};
pub use locale::{Locale, LocaleParseError, locales};
pub use locale_stack::LocaleStack;
pub use number::{
    CurrencyCode, MeasureUnit, RoundingMode, SignDisplay, UnitDisplay,
    decimal_and_group_separators, normalize_digits, parse_locale_number,
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
pub use provider::Icu4xBackend;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub use provider::WebIntlBackend;
pub use provider::{StubIntlBackend, default_backend};
pub use relative_time::{NumericOption, RelativeTimeFormatter};
pub use text::{grapheme_count, take_graphemes};
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
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
            Self::Auto => "auto",
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
        *self == Self::Rtl
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
            Self::Ltr => ResolvedDirection::Ltr,
            Self::Rtl => ResolvedDirection::Rtl,
            Self::Auto => fallback,
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
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
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
        matches!(self, Self::Rtl)
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
            ResolvedDirection::Ltr => Self::Ltr,
            ResolvedDirection::Rtl => Self::Rtl,
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
// Locale/calendar backend abstraction
// ────────────────────────────────────────────────────────────────────

/// Trait abstracting locale/calendar data providers for formatting and metadata.
///
/// Production uses `Icu4xBackend` with CLDR data; tests and non-date-time
/// components use [`StubIntlBackend`]. The trait is object-safe so it can be
/// stored as `Arc<dyn IntlBackend>` in `Env`.
///
/// Locale-facing formatting and metadata methods are required provider
/// responsibilities. Calendar compatibility helpers remain canonical defaults
/// over the public [`CalendarDate`] / [`CalendarSystem`] model so providers can
/// reuse the shared implementation when they do not need backend-specific
/// overrides.
///
/// Requires `Send + Sync` on every target so adapters and shared ownership
/// abstractions can treat locale/calendar backends uniformly across native and
/// wasm builds.
///
/// ```compile_fail
/// use ars_i18n::IntlBackend;
///
/// struct IncompleteProvider;
///
/// impl IntlBackend for IncompleteProvider {}
/// ```
pub trait IntlBackend: Send + Sync + 'static {
    /// Short weekday label (abbreviated format): `Mo`, `Tu`, `We`, ...
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String;

    /// Long weekday label (wide format): `Monday`, `Tuesday`, ...
    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String;

    /// Full month name in the locale.
    fn month_long_name(&self, month: u8, locale: &Locale) -> String;

    /// Localized day-period label.
    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String;

    /// Reverse-maps a typed character to AM/PM.
    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool>;

    /// Formats a numeric segment with locale-appropriate zero-padding.
    fn format_segment_digits(&self, value: u32, min_digits: NonZero<u8>, locale: &Locale)
    -> String;

    /// Maximum number of months in a year for the given calendar and year.
    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8 {
        if let Some(bounded) = calendar::bounded_months_in_year(*calendar, year, era) {
            return bounded;
        }

        let date = CalendarDate::new(
            *calendar,
            &CalendarDateFields {
                era: era.map(|code| Era {
                    code: code.to_string(),
                    display_name: code.to_string(),
                }),
                year: Some(year),
                month: Some(1),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .or_else(|_| CalendarDate::new_iso8601(1970, 1, 1))
        .expect("hard-coded fallback date is valid");

        calendar.months_in_year(&date)
    }

    /// Days in a specific month for the given calendar, year, and month.
    fn days_in_month(
        &self,
        calendar: &CalendarSystem,
        year: i32,
        month: u8,
        era: Option<&str>,
    ) -> u8 {
        if let Some(bounded) = calendar::bounded_days_in_month(*calendar, year, month, era) {
            return bounded;
        }

        let date = CalendarDate::new(
            *calendar,
            &CalendarDateFields {
                era: era.map(|code| Era {
                    code: code.to_string(),
                    display_name: code.to_string(),
                }),
                year: Some(year),
                month: Some(month),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .or_else(|_| CalendarDate::new_iso8601(1970, 1, 1))
        .expect("hard-coded fallback date is valid");

        calendar.days_in_month(&date)
    }

    /// Returns the calendar's default era when callers omit one.
    fn default_era(&self, calendar: &CalendarSystem) -> Option<Era> {
        calendar.default_era()
    }

    /// Returns the maximum year value in the date's current era, if bounded.
    fn years_in_era(&self, date: &CalendarDate) -> Option<i32> {
        date.years_in_era()
    }

    /// Returns the minimum allowed month ordinal for the date's current year.
    fn minimum_month_in_year(&self, date: &CalendarDate) -> u8 {
        date.minimum_month_in_year()
    }

    /// Returns the minimum allowed day ordinal for the date's current month.
    fn minimum_day_in_month(&self, date: &CalendarDate) -> u8 {
        date.minimum_day_in_month()
    }

    /// Preferred hour cycle for the locale.
    fn hour_cycle(&self, locale: &Locale) -> HourCycle;

    /// Locale week information including first-day and weekend metadata.
    fn week_info(&self, locale: &Locale) -> WeekInfo;

    /// First day of the week for the locale.
    fn first_day_of_week(&self, locale: &Locale) -> Weekday {
        self.week_info(locale).first_day
    }

    /// Converts a public calendar date into the target calendar system.
    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        date.to_calendar(target).unwrap_or_else(|_| date.clone())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec, vec::Vec};
    use core::num::NonZero;

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

        let backend = StubIntlBackend;

        assert_eq!(locale.first_day_of_week(&backend), Weekday::Sunday);
        assert_eq!(locale.hour_cycle(&backend), HourCycle::H12);
    }

    #[test]
    fn orientation_default_is_horizontal() {
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }

    #[test]
    fn stub_intl_backend_default_helpers_cover_fallback_paths() {
        let backend = StubIntlBackend;

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
            assert_eq!(backend.weekday_short_label(weekday, &locale), short);
            assert_eq!(backend.weekday_long_label(weekday, &locale), long);
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
            assert_eq!(backend.month_long_name(month, &locale), expected);
        }

        assert_eq!(backend.day_period_label(false, &locale), "AM");
        assert_eq!(backend.day_period_label(true, &locale), "PM");
        assert_eq!(backend.day_period_from_char('a', &locale), Some(false));
        assert_eq!(backend.day_period_from_char('A', &locale), Some(false));
        assert_eq!(backend.day_period_from_char('p', &locale), Some(true));
        assert_eq!(backend.day_period_from_char('P', &locale), Some(true));
        assert_eq!(backend.day_period_from_char('x', &locale), None);

        assert_eq!(
            backend.format_segment_digits(7, NonZero::new(2).expect("nonzero"), &locale),
            "07"
        );

        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Hebrew, 5784, None),
            13
        );
        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Hebrew, 5785, None),
            12
        );
        #[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Dangi, 2024, None),
            calendar::internal::months_in_year(2024, CalendarSystem::Dangi, None)
                .expect("ICU4X should resolve Dangi month counts")
        );
        #[cfg(not(any(feature = "icu4x", feature = "web-intl")))]
        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Dangi, 2024, None),
            12
        );
        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Gregorian, 2024, None),
            12
        );
        assert_eq!(
            backend.max_months_in_year(&CalendarSystem::Japanese, 31, Some("heisei")),
            4
        );

        assert_eq!(
            backend.days_in_month(&CalendarSystem::Gregorian, 2024, 2, None),
            29
        );
        assert_eq!(
            backend.days_in_month(&CalendarSystem::Gregorian, 2023, 2, None),
            28
        );
        assert_eq!(
            backend.days_in_month(&CalendarSystem::Japanese, 1, 5, Some("reiwa")),
            31
        );
        assert_eq!(
            backend.days_in_month(&CalendarSystem::Japanese, 31, 4, Some("heisei")),
            30
        );
        assert_eq!(
            backend.default_era(&CalendarSystem::Japanese),
            Some(Era {
                code: "reiwa".to_string(),
                display_name: "Reiwa".to_string(),
            })
        );
        assert_eq!(
            backend.default_era(&CalendarSystem::Gregorian),
            Some(Era {
                code: "ad".to_string(),
                display_name: "AD".to_string(),
            })
        );

        let japanese = CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: "heisei".to_string(),
                    display_name: "Heisei".to_string(),
                }),
                year: Some(1),
                month: Some(1),
                day: Some(8),
                ..CalendarDateFields::default()
            },
        )
        .expect("Japanese date should validate");

        assert_eq!(backend.years_in_era(&japanese), Some(31));
        assert_eq!(backend.minimum_month_in_year(&japanese), 1);
        assert_eq!(backend.minimum_day_in_month(&japanese), 8);

        let gregorian =
            CalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian date should validate");

        assert_eq!(backend.minimum_month_in_year(&gregorian), 1);
        assert_eq!(backend.minimum_day_in_month(&gregorian), 1);

        assert_eq!(backend.hour_cycle(&locale), HourCycle::H12);

        let german = Locale::parse("de-DE").expect("de-DE is valid");

        assert_eq!(backend.hour_cycle(&german), HourCycle::H23);
        assert_eq!(backend.first_day_of_week(&locale), Weekday::Sunday);
        assert_eq!(backend.first_day_of_week(&german), Weekday::Monday);

        assert_eq!(
            backend.convert_date(&gregorian, CalendarSystem::Gregorian),
            gregorian
        );
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_intl_backend_convert_date_supports_non_identity_conversions_when_internal_calendar_is_available()
     {
        let backend = StubIntlBackend;

        let gregorian =
            CalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian date should validate");

        let japanese = backend.convert_date(&gregorian, CalendarSystem::Japanese);

        let round_trip = backend.convert_date(&japanese, CalendarSystem::Gregorian);

        assert_eq!(
            japanese,
            CalendarDate::new(
                CalendarSystem::Japanese,
                &CalendarDateFields {
                    era: Some(Era {
                        code: String::from("reiwa"),
                        display_name: String::from("Reiwa"),
                    }),
                    year: Some(6),
                    month: Some(3),
                    day: Some(15),
                    ..CalendarDateFields::default()
                },
            )
            .expect("converted Japanese date should validate")
        );

        assert_eq!(round_trip, gregorian);
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_intl_backend_convert_date_preserves_bce_gregorian_years() {
        let backend = StubIntlBackend;

        let buddhist = CalendarDate::new(
            CalendarSystem::Buddhist,
            &CalendarDateFields {
                year: Some(1),
                month: Some(1),
                day: Some(1),
                ..CalendarDateFields::default()
            },
        )
        .expect("Buddhist date should validate");

        let gregorian = backend.convert_date(&buddhist, CalendarSystem::Gregorian);

        assert_eq!(
            gregorian,
            CalendarDate::new(
                CalendarSystem::Gregorian,
                &CalendarDateFields {
                    era: Some(Era {
                        code: "bc".to_string(),
                        display_name: "BC".to_string(),
                    }),
                    year: Some(543),
                    month: Some(1),
                    day: Some(1),
                    ..CalendarDateFields::default()
                },
            )
            .expect("Gregorian BCE date should validate")
        );
        assert_eq!(
            buddhist
                .to_calendar(CalendarSystem::Gregorian)
                .expect("direct Gregorian conversion should succeed"),
            gregorian
        );
        assert_eq!(
            backend.convert_date(&gregorian, CalendarSystem::Buddhist),
            buddhist
        );
    }

    #[cfg(all(feature = "std", any(feature = "icu4x", feature = "web-intl")))]
    #[test]
    fn stub_intl_backend_convert_date_falls_back_for_inputs_outside_internal_bridge_range() {
        let backend = StubIntlBackend;

        let gregorian =
            CalendarDate::new_gregorian(10_000, 1, 1).expect("Gregorian date should validate");

        let japanese = backend.convert_date(&gregorian, CalendarSystem::Japanese);

        assert_eq!(japanese.calendar(), CalendarSystem::Japanese);
        assert_eq!(
            backend.convert_date(&japanese, CalendarSystem::Gregorian),
            gregorian
        );
    }

    #[cfg(all(feature = "std", not(any(feature = "icu4x", feature = "web-intl"))))]
    #[test]
    fn stub_intl_backend_convert_date_uses_public_calendar_conversion_without_calendar_backend() {
        let backend = StubIntlBackend;

        let gregorian =
            CalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian date should validate");

        let japanese = backend.convert_date(&gregorian, CalendarSystem::Japanese);

        assert_eq!(japanese.calendar(), CalendarSystem::Japanese);
        assert_eq!(
            backend.convert_date(&japanese, CalendarSystem::Gregorian),
            gregorian
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
