//! Internationalization types for locale, text direction, and layout orientation.
//!
//! This crate provides the core i18n primitives shared across all ars-ui components:
//! a BCP 47 [`Locale`] wrapper, a [`Direction`] enum for LTR/RTL text flow, and an
//! [`Orientation`] enum for horizontal/vertical layout axes.

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
