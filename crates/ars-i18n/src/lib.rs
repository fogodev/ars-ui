//! Internationalization types for locale, text direction, and layout orientation.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a [`Direction`] enum for LTR/RTL text flow, an
//! [`Orientation`] enum for horizontal/vertical layout axes, and the [`IcuProvider`]
//! trait for calendar/locale data abstraction.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use core::fmt;

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

/// A BCP 47 locale identifier (e.g. `"en-US"`, `"ar-EG"`).
///
/// Wraps a locale string and is used by the environment provider to propagate
/// locale context to all components for i18n-aware formatting and RTL handling.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Locale(String);

impl Locale {
    /// Creates a new locale from a BCP 47 string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the locale as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse a BCP 47 locale identifier with structural validation.
    ///
    /// Validates that the string follows the basic BCP 47 structure
    /// (RFC 5646 §2.1): a 2–8 letter language subtag, followed by optional
    /// hyphen-separated subtags of 1–8 alphanumeric characters each.
    ///
    /// This performs **structural** validation only — it does not check whether
    /// the language, script, or region subtags are registered in the IANA
    /// subtag registry. Full semantic validation will come with ICU4X.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleParseError`] if the string is empty, contains invalid
    /// characters, or violates basic BCP 47 structure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ars_i18n::Locale;
    /// let en = Locale::parse("en-US").expect("valid");
    /// let ar = Locale::parse("ar").expect("valid");
    /// let ja = Locale::parse("ja-JP-u-ca-japanese").expect("valid");
    ///
    /// assert!(Locale::parse("").is_err());
    /// assert!(Locale::parse("123").is_err());
    /// assert!(Locale::parse("-en").is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, LocaleParseError> {
        let mut subtags = s.split('-');

        // Language subtag: 2–8 ASCII alphabetic characters (required).
        let lang = subtags.next().ok_or(LocaleParseError { _private: () })?;
        if lang.len() < 2 || lang.len() > 8 || !lang.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(LocaleParseError { _private: () });
        }

        // Subsequent subtags: 1–8 ASCII alphanumeric characters each.
        for subtag in subtags {
            if subtag.is_empty()
                || subtag.len() > 8
                || !subtag.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                return Err(LocaleParseError { _private: () });
            }
        }

        Ok(Self(String::from(s)))
    }
}

/// Error returned when a locale string fails BCP 47 structural validation.
///
/// See [`Locale::parse`] for the validation rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleParseError {
    _private: (),
}

impl fmt::Display for LocaleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid BCP 47 locale identifier")
    }
}

impl core::error::Error for LocaleParseError {}

// ────────────────────────────────────────────────────────────────────
// ICU data provider abstraction
// ────────────────────────────────────────────────────────────────────

/// Trait abstracting ICU4X data provider for calendar/locale operations.
///
/// Production uses `Icu4xProvider` with CLDR data; tests and non-date-time
/// components use [`StubIcuProvider`]. The trait is object-safe so it can be
/// stored as `ArsRc<dyn IcuProvider>` in `Env`.
///
/// On native targets, requires `Send + Sync` for multi-threaded runtimes.
/// On WASM (single-threaded), these bounds are omitted.
#[cfg(not(target_arch = "wasm32"))]
pub trait IcuProvider: Send + Sync + 'static {}

/// Trait abstracting ICU4X data provider for calendar/locale operations.
///
/// Production uses `Icu4xProvider` with CLDR data; tests and non-date-time
/// components use [`StubIcuProvider`]. The trait is object-safe so it can be
/// stored as `ArsRc<dyn IcuProvider>` in `Env`.
///
/// On native targets, requires `Send + Sync` for multi-threaded runtimes.
/// On WASM (single-threaded), these bounds are omitted.
#[cfg(target_arch = "wasm32")]
pub trait IcuProvider: 'static {}

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
    use alloc::string::ToString;

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

    // ── Locale::parse tests ────────────────────────────────────────

    #[test]
    fn parse_valid_locales() {
        assert!(Locale::parse("en").is_ok());
        assert!(Locale::parse("en-US").is_ok());
        assert!(Locale::parse("zh-Hans-CN").is_ok());
        assert!(Locale::parse("ja-JP-u-ca-japanese").is_ok());
        assert!(Locale::parse("ar").is_ok());
        assert!(Locale::parse("sr-Latn-RS").is_ok());
    }

    #[test]
    fn parse_invalid_locales() {
        assert!(Locale::parse("").is_err());
        assert!(Locale::parse("e").is_err()); // too short
        assert!(Locale::parse("123").is_err()); // digits in language
        assert!(Locale::parse("-en").is_err()); // leading hyphen
        assert!(Locale::parse("en-").is_err()); // trailing hyphen
        assert!(Locale::parse("en--US").is_err()); // double hyphen
        assert!(Locale::parse("abcdefghi").is_err()); // language too long (>8)
    }

    #[test]
    fn parse_rejects_invalid_subsequent_subtags() {
        // Subtag too long in second position
        assert!(Locale::parse("en-abcdefghi").is_err());
        // Non-alphanumeric character in subtag
        assert!(Locale::parse("en-US!").is_err());
        // Non-ASCII in subtag
        assert!(Locale::parse("en-Ü").is_err());
    }

    #[test]
    fn parse_roundtrips_through_as_str() {
        let locale = Locale::parse("en-US").expect("valid");
        assert_eq!(locale.as_str(), "en-US");
    }

    #[test]
    fn locale_new_creates_unvalidated_locale() {
        let locale = Locale::new("en-US");
        assert_eq!(locale.as_str(), "en-US");

        // new() accepts anything — no validation
        let garbage = Locale::new("not-a-real-locale-!!!");
        assert_eq!(garbage.as_str(), "not-a-real-locale-!!!");
    }

    #[test]
    fn locale_default_is_empty() {
        let locale = Locale::default();
        assert_eq!(locale.as_str(), "");
    }

    #[test]
    fn locale_parse_error_display() {
        let err = Locale::parse("").unwrap_err();
        assert_eq!(err.to_string(), "invalid BCP 47 locale identifier");
    }
}
