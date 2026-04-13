//! Internationalization types for locale, number formatting, text direction,
//! layout orientation, and logical-to-physical layout geometry.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a locale-aware [`NumberFormatter`], a
//! [`Direction`] enum for LTR/RTL text flow, an [`Orientation`] enum for
//! horizontal/vertical layout axes, RTL-aware layout geometry types
//! ([`LogicalSide`], [`PhysicalSide`], [`LogicalRect`], [`PhysicalRect`]),
//! and the [`IcuProvider`] trait for calendar/locale data abstraction.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "icu4x", feature = "web-intl"))]
compile_error!("features `icu4x` and `web-intl` are mutually exclusive");

extern crate alloc;

use alloc::string::String;

mod bidi;
mod layout;
mod locale;
mod locale_stack;
mod number;
mod translate;
mod weekday;

pub use bidi::{IsolateDirection, isolate_text_safe};
pub use layout::{LogicalRect, LogicalSide, PhysicalRect, PhysicalSide};
pub use locale::{Locale, LocaleParseError, locales};
pub use locale_stack::LocaleStack;
#[cfg(feature = "std")]
pub use number::get_number_formatter;
pub use number::{
    CurrencyCode, MeasureUnit, NumberFormatOptions, NumberFormatter, NumberStyle, RoundingMode,
    SignDisplay, UnitDisplay, decimal_and_group_separators, normalize_digits, parse_locale_number,
};
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
pub trait IcuProvider: Send + Sync + 'static {}

/// English-only stub provider for tests and non-ICU4X builds.
///
/// Returns hardcoded English values for all provider operations. This is the
/// default provider used by [`Env::default()`](ars_core::Env).
#[derive(Debug)]
pub struct StubIcuProvider;

impl IcuProvider for StubIcuProvider {}

// ────────────────────────────────────────────────────────────────────
// Placeholder date/time types
// ────────────────────────────────────────────────────────────────────

/// A calendar date (year, month, day).
///
/// This is a placeholder type that will be backed by ICU4X when the full
/// internationalization layer is implemented. Components use this in
/// [`FieldValue::Date`](../ars_forms/enum.FieldValue.html) for date fields.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarDate {
    _private: (),
}

impl CalendarDate {
    /// Returns the date formatted as an ISO 8601 string (`"YYYY-MM-DD"`).
    ///
    /// Placeholder implementation — returns an empty string until the full
    /// ICU4X-backed date type is available.
    #[must_use]
    pub const fn to_iso8601(&self) -> String {
        String::new()
    }
}

/// A time of day (hours, minutes, seconds).
///
/// This is a placeholder type that will be backed by ICU4X when the full
/// internationalization layer is implemented. Components use this in
/// [`FieldValue::Time`](../ars_forms/enum.FieldValue.html) for time fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Time {
    _private: (),
}

impl Time {
    /// Returns the time formatted as an ISO 8601 string (`"HH:MM:SS"`).
    ///
    /// Placeholder implementation — returns an empty string until the full
    /// ICU4X-backed time type is available.
    #[must_use]
    pub const fn to_iso8601(&self) -> String {
        String::new()
    }
}

/// A range of two calendar dates (start and end).
///
/// This is a placeholder type that will be backed by ICU4X when the full
/// internationalization layer is implemented. Components use this in
/// [`FieldValue::DateRange`](../ars_forms/enum.FieldValue.html) for date range fields.
#[derive(Clone, Debug, PartialEq)]
pub struct DateRange {
    _private: (),
}

impl DateRange {
    /// Returns the date range formatted as an ISO 8601 interval string
    /// (`"YYYY-MM-DD/YYYY-MM-DD"`).
    ///
    /// Placeholder implementation — returns an empty string until the full
    /// ICU4X-backed date range type is available.
    #[must_use]
    pub const fn to_iso8601(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec, vec::Vec};

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
                Direction::Rtl,
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
                Direction::Rtl,
                "{} should be RTL",
                locale.to_bcp47()
            );
            assert!(locale.is_rtl(), "{} should be RTL", locale.to_bcp47());
        }
    }

    #[test]
    fn locale_direction_defaults_to_ltr_when_not_rtl() {
        let locale = Locale::parse("en-US").expect("en-US is valid");
        assert_eq!(locale.direction(), Direction::Ltr);
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

    #[test]
    fn placeholder_iso_helpers_return_empty_strings() {
        let date = CalendarDate { _private: () };
        let time = Time { _private: () };
        let range = DateRange { _private: () };

        assert_eq!(date.to_iso8601(), "");
        assert_eq!(time.to_iso8601(), "");
        assert_eq!(range.to_iso8601(), "");
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
