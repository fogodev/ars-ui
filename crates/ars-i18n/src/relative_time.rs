//! Locale-aware relative time formatting helpers.

#[cfg(any(
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
use {
    fixed_decimal::Decimal,
    icu_experimental::relativetime::{
        RelativeTimeFormatter as IcuRelativeTimeFormatter, RelativeTimeFormatterOptions,
        RelativeTimeFormatterPreferences, options::Numeric,
    },
};
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use {
    js_sys::{
        Array,
        Intl::{
            RelativeTimeFormat as JsRelativeTimeFormat, RelativeTimeFormatNumeric,
            RelativeTimeFormatOptions, RelativeTimeFormatStyle,
        },
    },
    wasm_bindgen::JsValue,
};

use crate::Locale;

/// Controls whether relative time uses numeric or natural-language phrasing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NumericOption {
    /// Always use numeric output such as `1 day ago`.
    #[default]
    Always,
    /// Use natural-language output where the backend supports it.
    Auto,
}

impl NumericOption {
    #[cfg(feature = "icu4x")]
    fn to_icu(self) -> RelativeTimeFormatterOptions {
        let mut options = RelativeTimeFormatterOptions::default();

        options.numeric = match self {
            Self::Always => Numeric::Always,
            Self::Auto => Numeric::Auto,
        };

        options
    }

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    const fn to_js(self) -> RelativeTimeFormatNumeric {
        match self {
            Self::Always => RelativeTimeFormatNumeric::Always,
            Self::Auto => RelativeTimeFormatNumeric::Auto,
        }
    }
}

/// A locale-aware relative-time formatter.
pub struct RelativeTimeFormatter {
    locale: Locale,

    numeric: NumericOption,

    #[cfg(feature = "icu4x")]
    second_formatter: IcuRelativeTimeFormatter,

    #[cfg(feature = "icu4x")]
    minute_formatter: IcuRelativeTimeFormatter,

    #[cfg(feature = "icu4x")]
    hour_formatter: IcuRelativeTimeFormatter,

    #[cfg(feature = "icu4x")]
    day_formatter: IcuRelativeTimeFormatter,

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    formatter: JsRelativeTimeFormat,
}

impl core::fmt::Debug for RelativeTimeFormatter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RelativeTimeFormatter")
            .field("locale", &self.locale)
            .field("numeric", &self.numeric)
            .finish()
    }
}

#[cfg(feature = "icu4x")]
impl RelativeTimeFormatter {
    /// Creates a formatter using numeric phrasing by default.
    #[must_use]
    pub fn new(locale: &Locale) -> Self {
        Self::with_numeric(locale, NumericOption::default())
    }

    /// Creates a formatter with an explicit numeric strategy.
    #[must_use]
    pub fn with_numeric(locale: &Locale, numeric: NumericOption) -> Self {
        let options = numeric.to_icu();
        let prefs = RelativeTimeFormatterPreferences::from(locale.as_icu());

        Self {
            locale: locale.clone(),
            numeric,
            second_formatter: IcuRelativeTimeFormatter::try_new_long_second(prefs, options)
                .expect("compiled_data guarantees second relative-time data"),
            minute_formatter: IcuRelativeTimeFormatter::try_new_long_minute(prefs, options)
                .expect("compiled_data guarantees minute relative-time data"),
            hour_formatter: IcuRelativeTimeFormatter::try_new_long_hour(prefs, options)
                .expect("compiled_data guarantees hour relative-time data"),
            day_formatter: IcuRelativeTimeFormatter::try_new_long_day(prefs, options)
                .expect("compiled_data guarantees day relative-time data"),
        }
    }

    /// Formats a duration in seconds relative to now.
    #[must_use]
    pub fn format_seconds(&self, seconds: i64) -> String {
        if seconds.abs() < 60 {
            return self
                .second_formatter
                .format(Decimal::from(seconds))
                .to_string();
        }

        if seconds.abs() < 3_600 {
            return self
                .minute_formatter
                .format(Decimal::from(seconds / 60))
                .to_string();
        }

        if seconds.abs() < 86_400 {
            return self
                .hour_formatter
                .format(Decimal::from(seconds / 3_600))
                .to_string();
        }

        self.day_formatter
            .format(Decimal::from(seconds / 86_400))
            .to_string()
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
impl RelativeTimeFormatter {
    /// Creates a formatter using numeric phrasing by default.
    #[must_use]
    pub fn new(locale: &Locale) -> Self {
        Self::with_numeric(locale, NumericOption::default())
    }

    /// Creates a formatter with an explicit numeric strategy.
    #[must_use]
    pub fn with_numeric(locale: &Locale, numeric: NumericOption) -> Self {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let options = RelativeTimeFormatOptions::new();
        options.set_numeric(numeric.to_js());
        options.set_style(RelativeTimeFormatStyle::Long);

        Self {
            locale: locale.clone(),
            numeric,
            formatter: JsRelativeTimeFormat::new(&locales, options.as_ref()),
        }
    }

    /// Formats a duration in seconds relative to now using browser `Intl`.
    #[must_use]
    pub fn format_seconds(&self, seconds: i64) -> String {
        let (value, unit) = bucket_relative_time(seconds);

        String::from(self.formatter.format(value as f64, unit))
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
impl RelativeTimeFormatter {
    /// Creates a formatter using numeric phrasing by default.
    #[must_use]
    pub fn new(locale: &Locale) -> Self {
        Self::with_numeric(locale, NumericOption::default())
    }

    /// Creates a formatter with an explicit numeric strategy.
    #[must_use]
    pub fn with_numeric(locale: &Locale, numeric: NumericOption) -> Self {
        Self {
            locale: locale.clone(),
            numeric,
        }
    }

    /// Formats a duration in seconds relative to now using the English fallback.
    #[must_use]
    pub fn format_seconds(&self, seconds: i64) -> String {
        fallback_format_relative_time(self.numeric, seconds)
    }
}

#[cfg(any(
    all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")),
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
const fn bucket_relative_time(seconds: i64) -> (i64, &'static str) {
    if seconds.abs() < 60 {
        (seconds, "second")
    } else if seconds.abs() < 3_600 {
        (seconds / 60, "minute")
    } else if seconds.abs() < 86_400 {
        (seconds / 3_600, "hour")
    } else {
        (seconds / 86_400, "day")
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
fn fallback_format_relative_time(numeric: NumericOption, seconds: i64) -> String {
    let (value, unit) = bucket_relative_time(seconds);

    if numeric == NumericOption::Auto {
        match (unit, value) {
            ("second", 0) => return String::from("now"),
            ("minute", 0) => return String::from("this minute"),
            ("hour", 0) => return String::from("this hour"),
            ("day", -1) => return String::from("yesterday"),
            ("day", 0) => return String::from("today"),
            ("day", 1) => return String::from("tomorrow"),
            _ => {}
        }
    }

    let magnitude = value.unsigned_abs();

    let suffix = if magnitude == 1 { "" } else { "s" };

    if value < 0 {
        format!("{magnitude} {unit}{suffix} ago")
    } else {
        format!("in {magnitude} {unit}{suffix}")
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "icu4x")]
    use super::{NumericOption, RelativeTimeFormatter};
    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use super::{NumericOption, RelativeTimeFormatter};
    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::locales;
    #[cfg(feature = "icu4x")]
    use crate::{Locale, locales};

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_seconds_formats_past_minute_in_english() {
        let formatter = RelativeTimeFormatter::new(&locales::en_us());

        assert_eq!(formatter.format_seconds(-60), "1 minute ago");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_seconds_formats_future_hour_in_english() {
        let formatter = RelativeTimeFormatter::new(&locales::en_us());

        assert_eq!(formatter.format_seconds(3_600), "in 1 hour");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn numeric_auto_uses_yesterday_for_negative_day() {
        let formatter = RelativeTimeFormatter::with_numeric(&locales::en_us(), NumericOption::Auto);

        assert_eq!(formatter.format_seconds(-86_400), "yesterday");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn arabic_relative_time_uses_arabic_output() {
        let formatter = RelativeTimeFormatter::new(&Locale::parse("ar-EG").expect("locale"));

        let formatted = formatter.format_seconds(-60);

        assert!(formatted.contains("قبل"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn relative_time_covers_seconds_minutes_hours_and_days() {
        let formatter = RelativeTimeFormatter::new(&locales::en_us());

        assert_eq!(formatter.format_seconds(59), "in 59 seconds");
        assert_eq!(formatter.format_seconds(60), "in 1 minute");
        assert_eq!(formatter.format_seconds(3_599), "in 59 minutes");
        assert_eq!(formatter.format_seconds(3_600), "in 1 hour");
        assert_eq!(formatter.format_seconds(86_399), "in 23 hours");
        assert_eq!(formatter.format_seconds(86_400), "in 1 day");
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
    fn fallback_relative_time_keeps_api_available() {
        let formatter = RelativeTimeFormatter::with_numeric(&locales::en_us(), NumericOption::Auto);

        assert_eq!(formatter.format_seconds(-86_400), "yesterday");
        assert_eq!(formatter.format_seconds(3_600), "in 1 hour");
    }
}

#[cfg(all(
    test,
    feature = "web-intl",
    target_arch = "wasm32",
    not(feature = "icu4x")
))]
mod web_intl_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{NumericOption, RelativeTimeFormatter};
    use crate::{Locale, locales};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn web_intl_relative_time_formats_future_hour() {
        let formatter = RelativeTimeFormatter::new(&locales::en_us());

        assert_eq!(formatter.format_seconds(3_600), "in 1 hour");
    }

    #[wasm_bindgen_test]
    fn web_intl_relative_time_formats_yesterday_in_auto_mode() {
        let formatter = RelativeTimeFormatter::with_numeric(&locales::en_us(), NumericOption::Auto);

        assert_eq!(formatter.format_seconds(-86_400), "yesterday");
    }

    #[wasm_bindgen_test]
    fn web_intl_relative_time_uses_representative_arabic_output() {
        let formatter = RelativeTimeFormatter::new(&Locale::parse("ar-EG").expect("locale"));

        assert!(formatter.format_seconds(-60).contains("قبل"));
    }
}
