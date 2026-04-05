//! Internationalization types for locale, text direction, and layout orientation.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a [`Direction`] enum for LTR/RTL text flow, and an
//! [`Orientation`] enum for horizontal/vertical layout axes.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;

/// The text flow direction of a locale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// Left-to-right text direction (default for most Latin-script locales).
    #[default]
    Ltr,
    /// Right-to-left text direction (used by Arabic, Hebrew, and related scripts).
    Rtl,
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
}

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
    pub fn to_iso8601(&self) -> String {
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
    pub fn to_iso8601(&self) -> String {
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
    pub fn to_iso8601(&self) -> String {
        String::new()
    }
}
