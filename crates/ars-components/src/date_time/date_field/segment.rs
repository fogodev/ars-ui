//! Segment types shared by the `DateField` machine and connect API.

use alloc::string::{String, ToString};

use ars_core::{IntlBackend, Locale, MessageFn};

use super::Messages;

/// The logical kind of a date or time segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DateSegmentKind {
    /// The year segment.
    #[default]
    Year,

    /// The month segment.
    Month,

    /// The day segment.
    Day,

    /// The hour segment.
    Hour,

    /// The minute segment.
    Minute,

    /// The second segment.
    Second,

    /// The AM/PM day period segment.
    DayPeriod,

    /// The weekday segment.
    Weekday,

    /// The era segment.
    Era,

    /// A non-editable separator segment.
    Literal,

    /// The time-zone-name segment.
    TimeZoneName,
}

impl DateSegmentKind {
    /// Returns `true` when this segment accepts user edits.
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        !matches!(self, Self::Literal | Self::Weekday | Self::TimeZoneName)
    }

    /// Returns `true` when this segment accepts numeric type-ahead.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Year | Self::Month | Self::Day | Self::Hour | Self::Minute | Self::Second
        )
    }

    /// Returns the localized ARIA label for this segment kind.
    #[must_use]
    pub fn aria_label(&self, messages: &Messages, locale: &Locale) -> String {
        fn call(
            message: &MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
            locale: &Locale,
        ) -> String {
            message(locale)
        }

        match self {
            Self::Year => call(&messages.year_label, locale),
            Self::Month => call(&messages.month_label, locale),
            Self::Day => call(&messages.day_label, locale),
            Self::Hour => call(&messages.hour_label, locale),
            Self::Minute => call(&messages.minute_label, locale),
            Self::Second => call(&messages.second_label, locale),
            Self::DayPeriod => call(&messages.day_period_label, locale),
            Self::Weekday => call(&messages.weekday_label, locale),
            Self::Era => call(&messages.era_label, locale),
            Self::Literal => String::new(),
            Self::TimeZoneName => call(&messages.timezone_label, locale),
        }
    }

    pub(crate) const fn data_name(self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Month => "month",
            Self::Day => "day",
            Self::Hour => "hour",
            Self::Minute => "minute",
            Self::Second => "second",
            Self::DayPeriod => "day-period",
            Self::Weekday => "weekday",
            Self::Era => "era",
            Self::Literal => "literal",
            Self::TimeZoneName => "time-zone-name",
        }
    }
}

/// A single segment within a `DateField`.
#[derive(Clone, Debug, PartialEq)]
pub struct DateSegment {
    /// The kind of segment.
    pub kind: DateSegmentKind,

    /// Current numeric value, or `None` when the user has not entered it.
    pub value: Option<i32>,

    /// Minimum valid numeric value for editable segments.
    pub min: i32,

    /// Maximum valid numeric value for editable segments.
    pub max: i32,

    /// Formatted display text for the current value.
    pub text: String,

    /// Placeholder shown when the segment has no value.
    pub placeholder: String,

    /// Literal text for non-editable separator segments.
    pub literal: Option<String>,

    /// Whether this segment accepts keyboard input.
    pub is_editable: bool,
}

impl DateSegment {
    /// Creates a new editable numeric segment.
    #[must_use]
    pub fn new_numeric(kind: DateSegmentKind, min: i32, max: i32, placeholder: &str) -> Self {
        Self {
            kind,
            value: None,
            min,
            max,
            text: String::new(),
            placeholder: placeholder.to_string(),
            literal: None,
            is_editable: true,
        }
    }

    /// Creates a new non-editable literal segment.
    #[must_use]
    pub fn new_literal(ch: &str) -> Self {
        Self {
            kind: DateSegmentKind::Literal,
            value: None,
            min: 0,
            max: 0,
            text: ch.to_string(),
            placeholder: ch.to_string(),
            literal: Some(ch.to_string()),
            is_editable: false,
        }
    }

    /// Creates a new non-editable semantic segment.
    #[must_use]
    pub fn new_non_editable(kind: DateSegmentKind, text: &str) -> Self {
        Self {
            kind,
            value: None,
            min: 0,
            max: 0,
            text: text.to_string(),
            placeholder: text.to_string(),
            literal: Some(text.to_string()),
            is_editable: false,
        }
    }

    /// Returns the text adapters should render for this segment.
    #[must_use]
    pub fn display_text(&self) -> &str {
        if self.value.is_some() && !self.text.is_empty() {
            &self.text
        } else {
            &self.placeholder
        }
    }

    /// Returns a human-readable ARIA value text for the current segment value.
    #[must_use]
    pub fn aria_value_text(&self, backend: &dyn IntlBackend, locale: &Locale) -> Option<String> {
        let value = self.value?;

        match self.kind {
            DateSegmentKind::Month => u8::try_from(value)
                .ok()
                .map(|month| backend.month_long_name(month, locale)),

            DateSegmentKind::DayPeriod => Some(backend.day_period_label(value == 1, locale)),

            _ => Some(value.to_string()),
        }
    }
}
