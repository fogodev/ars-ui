use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::fmt::{self, Display};

use icu::calendar::AnyCalendarKind;
use tinystr::TinyAsciiStr;

use super::{
    CalendarDate,
    date::{projected_date_for_calendar, temporal_calendar_for},
    helpers::{default_era_for, era_definition, gregorian_days_in_month},
};
use crate::{Locale, Weekday};

/// Supported calendar systems used throughout ars-ui.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CalendarSystem {
    /// ISO 8601 calendar semantics.
    Iso8601,

    /// Proleptic Gregorian calendar semantics.
    #[default]
    Gregorian,

    /// Buddhist Era calendar.
    Buddhist,

    /// Japanese imperial calendar.
    Japanese,

    /// Hebrew calendar.
    Hebrew,

    /// Tabular Hijri calendar using the civil Friday leap-year pattern.
    IslamicCivil,

    /// Umm al-Qura Hijri calendar.
    IslamicUmmAlQura,

    /// Persian Solar Hijri calendar.
    Persian,

    /// Indian national calendar.
    Indian,

    /// Chinese traditional calendar.
    Chinese,

    /// Coptic calendar.
    Coptic,

    /// Korean traditional lunar calendar.
    Dangi,

    /// Ethiopic calendar.
    Ethiopic,

    /// Ethiopic Amete Alem calendar.
    EthiopicAmeteAlem,

    /// Republic of China (Minguo) calendar.
    Roc,
}

impl CalendarSystem {
    /// Parses a BCP 47 `u-ca-*` calendar identifier.
    #[must_use]
    pub fn from_bcp47(identifier: &str) -> Option<Self> {
        match identifier {
            "iso8601" => Some(Self::Iso8601),
            "gregory" | "gregorian" => Some(Self::Gregorian),
            "buddhist" => Some(Self::Buddhist),
            "japanese" | "japanext" => Some(Self::Japanese),
            "hebrew" => Some(Self::Hebrew),
            "islamic-umalqura" => Some(Self::IslamicUmmAlQura),
            "islamic-civil" => Some(Self::IslamicCivil),
            "persian" => Some(Self::Persian),
            "indian" => Some(Self::Indian),
            "chinese" => Some(Self::Chinese),
            "coptic" => Some(Self::Coptic),
            "dangi" => Some(Self::Dangi),
            "ethiopic" => Some(Self::Ethiopic),
            "ethioaa" => Some(Self::EthiopicAmeteAlem),
            "roc" => Some(Self::Roc),
            _ => None,
        }
    }

    /// Returns the canonical BCP 47 `u-ca-*` identifier for this calendar.
    #[must_use]
    pub const fn to_bcp47_value(self) -> &'static str {
        match self {
            Self::Iso8601 => "iso8601",
            Self::Gregorian => "gregory",
            Self::Buddhist => "buddhist",
            Self::Japanese => "japanese",
            Self::Hebrew => "hebrew",
            Self::IslamicCivil => "islamic-civil",
            Self::IslamicUmmAlQura => "islamic-umalqura",
            Self::Persian => "persian",
            Self::Indian => "indian",
            Self::Chinese => "chinese",
            Self::Coptic => "coptic",
            Self::Dangi => "dangi",
            Self::Ethiopic => "ethiopic",
            Self::EthiopicAmeteAlem => "ethioaa",
            Self::Roc => "roc",
        }
    }

    /// Resolves the requested calendar from a locale, defaulting to Gregorian.
    #[must_use]
    pub fn from_locale(locale: &Locale) -> Self {
        locale
            .calendar_extension()
            .and_then(Self::from_bcp47)
            .unwrap_or(Self::Gregorian)
    }

    /// Converts this calendar system to ICU4X's runtime calendar discriminant.
    #[must_use]
    pub const fn to_icu_kind(self) -> AnyCalendarKind {
        match self {
            Self::Iso8601 => AnyCalendarKind::Iso,
            Self::Gregorian => AnyCalendarKind::Gregorian,
            Self::Buddhist => AnyCalendarKind::Buddhist,
            Self::Japanese => AnyCalendarKind::Japanese,
            Self::Hebrew => AnyCalendarKind::Hebrew,
            Self::IslamicCivil => AnyCalendarKind::HijriTabularTypeIIFriday,
            Self::IslamicUmmAlQura => AnyCalendarKind::HijriUmmAlQura,
            Self::Persian => AnyCalendarKind::Persian,
            Self::Indian => AnyCalendarKind::Indian,
            Self::Chinese => AnyCalendarKind::Chinese,
            Self::Coptic => AnyCalendarKind::Coptic,
            Self::Dangi => AnyCalendarKind::Dangi,
            Self::Ethiopic => AnyCalendarKind::Ethiopian,
            Self::EthiopicAmeteAlem => AnyCalendarKind::EthiopianAmeteAlem,
            Self::Roc => AnyCalendarKind::Roc,
        }
    }

    /// Returns `true` when this calendar uses named eras in the public API.
    #[must_use]
    pub const fn has_custom_eras(self) -> bool {
        !matches!(self, Self::Iso8601 | Self::Chinese | Self::Dangi)
    }

    /// Returns the known Japanese eras referenced by the specification.
    #[must_use]
    pub const fn japanese_eras() -> &'static [JapaneseEra] {
        &JAPANESE_ERAS
    }

    /// Returns supported calendar metadata used for validation and adapter UI.
    #[must_use]
    pub const fn supported_calendars() -> &'static [CalendarMetadata] {
        &SUPPORTED_CALENDARS
    }

    /// Returns the named eras supported by this calendar in chronological order.
    #[must_use]
    pub fn eras(self) -> Vec<Era> {
        match self {
            Self::Japanese => Self::japanese_eras()
                .iter()
                .map(|era| Era {
                    code: String::from(era.code),
                    display_name: String::from(era.english_name),
                })
                .collect(),

            Self::Gregorian => vec![
                Era {
                    code: String::from("bc"),
                    display_name: String::from("BC"),
                },
                Era {
                    code: String::from("ad"),
                    display_name: String::from("AD"),
                },
            ],

            _ => {
                let mut eras = Vec::new();

                if let Some(default) = default_era_for(self) {
                    let Some(definition) = era_definition(self, default.code.as_str()) else {
                        return vec![default];
                    };

                    let candidates = match self {
                        Self::Roc => &[("broc", "Before ROC"), ("roc", "ROC")][..],

                        Self::Coptic => &[("bce", "BCE"), ("ce", "CE")][..],

                        Self::Ethiopic => &[("aa", "AA"), ("am", "AM")][..],

                        _ => &[(definition.code, definition.display_name)][..],
                    };

                    for (code, display_name) in candidates {
                        eras.push(Era {
                            code: String::from(*code),
                            display_name: String::from(*display_name),
                        });
                    }
                }

                eras
            }
        }
    }

    /// Returns the default era used when callers omit one.
    #[must_use]
    pub fn default_era(self) -> Option<Era> {
        default_era_for(self)
    }

    /// Returns the number of months in the given date's display year for this calendar.
    #[must_use]
    pub fn months_in_year(self, date: &CalendarDate) -> u8 {
        projected_date_for_calendar(date, self).map_or(12, |projected| {
            if self == Self::Japanese {
                let Some(era) = projected.era.as_ref() else {
                    return projected.months_in_year;
                };

                return japanese_era_max_month(projected.year, era.code.as_str())
                    .unwrap_or(projected.months_in_year);
            }

            projected.months_in_year
        })
    }

    /// Returns the number of days in the given date's display month for this calendar.
    #[must_use]
    pub fn days_in_month(self, date: &CalendarDate) -> u8 {
        projected_date_for_calendar(date, self).map_or(31, |projected| {
            if self == Self::Japanese {
                let Some(era) = projected.era.as_ref() else {
                    return projected.days_in_month;
                };

                return japanese_era_max_day(projected.year, projected.month, era.code.as_str())
                    .unwrap_or(projected.days_in_month);
            }

            projected.days_in_month
        })
    }

    /// Returns the maximum year value in the date's current era, if the era is bounded.
    #[must_use]
    pub fn years_in_era(self, date: &CalendarDate) -> Option<i32> {
        let projected = projected_date_for_calendar(date, self)?;

        let era = projected.era.as_ref()?;

        era_definition(self, era.code.as_str())?.max_year
    }

    /// Returns the minimum allowed month ordinal for the date's current year.
    #[must_use]
    pub fn minimum_month_in_year(self, date: &CalendarDate) -> u8 {
        let Some(projected) = projected_date_for_calendar(date, self) else {
            return 1;
        };

        if self != Self::Japanese {
            return 1;
        }

        let Some(era) = projected.era.as_ref() else {
            return 1;
        };

        let Some(boundary) = Self::japanese_eras()
            .iter()
            .find(|candidate| candidate.code == era.code)
        else {
            return 1;
        };

        if projected.year == 1 {
            boundary.start_month
        } else {
            1
        }
    }

    /// Returns the minimum allowed day ordinal for the date's current month.
    #[must_use]
    pub fn minimum_day_in_month(self, date: &CalendarDate) -> u8 {
        let Some(projected) = projected_date_for_calendar(date, self) else {
            return 1;
        };

        if self != Self::Japanese {
            return 1;
        }

        let Some(era) = projected.era.as_ref() else {
            return 1;
        };

        let Some(boundary) = Self::japanese_eras()
            .iter()
            .find(|candidate| candidate.code == era.code)
        else {
            return 1;
        };

        if projected.year == 1 && projected.month == boundary.start_month {
            boundary.start_day
        } else {
            1
        }
    }
}

fn japanese_era_start(code: &str) -> Option<&'static JapaneseEra> {
    CalendarSystem::japanese_eras()
        .iter()
        .find(|candidate| candidate.code == code)
}

fn japanese_era_end(code: &str) -> Option<(i32, u8, u8)> {
    match code {
        "meiji" => Some((1912, 7, 29)),

        "taisho" => Some((1926, 12, 24)),

        "showa" => Some((1989, 1, 7)),

        "heisei" => Some((2019, 4, 30)),

        _ => None,
    }
}

fn japanese_era_max_month(year: i32, era_code: &str) -> Option<u8> {
    let start = japanese_era_start(era_code)?;

    let (end_year, end_month, _) = japanese_era_end(era_code)?;

    let max_year = end_year - start.start_year + 1;

    if year == max_year {
        Some(end_month)
    } else {
        Some(12)
    }
}

fn japanese_era_max_day(year: i32, month: u8, era_code: &str) -> Option<u8> {
    let start = japanese_era_start(era_code)?;

    let absolute_year = start.start_year.checked_add(year)?.checked_sub(1)?;

    let mut max_day = gregorian_days_in_month(absolute_year, month);

    if let Some((end_year, end_month, end_day)) = japanese_era_end(era_code) {
        let max_year = end_year - start.start_year + 1;

        if year == max_year && month == end_month {
            max_day = max_day.min(end_day);
        }
    }

    Some(max_day)
}

/// Static metadata describing a supported calendar system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarMetadata {
    /// The calendar variant.
    pub calendar: CalendarSystem,

    /// The canonical BCP 47 identifier.
    pub bcp47: &'static str,

    /// Whether the public API exposes named eras for this calendar.
    pub has_custom_eras: bool,
}

/// Public metadata for one of the modern Japanese eras.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JapaneseEra {
    /// Lowercase public era code.
    pub code: &'static str,

    /// English display label.
    pub english_name: &'static str,

    /// Japanese display label.
    pub japanese_name: &'static str,

    /// ISO start year.
    pub start_year: i32,

    /// 1-based start month.
    pub start_month: u8,

    /// 1-based start day.
    pub start_day: u8,
}

impl JapaneseEra {
    /// Returns a localized era label for the given locale.
    #[must_use]
    pub fn localized_name(self, locale: &Locale) -> &'static str {
        if locale.language() == "ja" {
            self.japanese_name
        } else {
            self.english_name
        }
    }
}

/// Named era information exposed through the public calendar API.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Era {
    /// Stable lowercase era code.
    pub code: String,

    /// User-facing display name.
    pub display_name: String,
}

/// A validated calendar month code like `M03` or `M05L`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MonthCode(TinyAsciiStr<4>);

impl MonthCode {
    /// Parses a month code.
    ///
    /// # Errors
    ///
    /// Returns an error if `code` is not a valid Temporal month-code string.
    pub fn new(code: &str) -> Result<Self, DateError> {
        let inner = temporal_rs::MonthCode::try_from_utf8(code.as_bytes())
            .map_err(|error| DateError::CalendarError(error.to_string()))?;

        Ok(Self(inner.as_tinystr()))
    }

    /// Creates a non-leap month code from an ordinal month.
    ///
    /// # Errors
    ///
    /// Returns an error if `month` is outside the supported `1..=13` range.
    pub fn new_normal(month: u8) -> Result<Self, DateError> {
        if !(1..=13).contains(&month) {
            return Err(DateError::InvalidDate);
        }

        let code = format!("M{month:02}");

        Self::new(&code)
    }

    /// Returns the month code as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns `true` when the month code denotes a leap month.
    #[must_use]
    pub const fn is_leap_month(&self) -> bool {
        self.0.all_bytes()[3] == b'L'
    }

    pub(crate) fn from_temporal(code: temporal_rs::MonthCode) -> Self {
        Self(code.as_tinystr())
    }

    pub(crate) fn to_temporal(self) -> temporal_rs::MonthCode {
        temporal_rs::MonthCode::try_from_utf8(self.as_str().as_bytes())
            .expect("stored month code is always valid")
    }
}

impl Display for MonthCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A stable time-zone identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeZoneId(String);

impl TimeZoneId {
    /// Parses and canonicalizes a time-zone identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the identifier is empty or the runtime time-zone
    /// engine rejects it.
    pub fn new(identifier: impl Into<String>) -> Result<Self, CalendarError> {
        let identifier = identifier.into();

        #[cfg(feature = "std")]
        let zone = temporal_rs::TimeZone::try_from_str(identifier.as_str())
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        #[cfg(feature = "std")]
        return Ok(Self(
            zone.identifier()
                .map_err(|error| CalendarError::Arithmetic(error.to_string()))?,
        ));

        #[cfg(not(feature = "std"))]
        if identifier.is_empty() {
            return Err(CalendarError::Arithmetic(String::from(
                "time-zone identifier must not be empty",
            )));
        }

        #[cfg(not(feature = "std"))]
        return Ok(Self(identifier));
    }

    /// Returns the canonical identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(feature = "std")]
    pub(crate) fn to_temporal(&self) -> Result<temporal_rs::TimeZone, CalendarError> {
        temporal_rs::TimeZone::try_from_str(self.as_str())
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))
    }
}

impl Display for TimeZoneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Preferred hour cycle for a locale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HourCycle {
    /// 0-11, midnight = 0.
    H11,

    /// 1-12, midnight = 12.
    H12,

    /// 0-23.
    #[default]
    H23,

    /// 1-24.
    H24,
}

/// Locale-derived week information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WeekInfo {
    /// The locale's first day of week.
    pub first_day: Weekday,

    /// The locale's weekend start day.
    pub weekend_start: Weekday,

    /// The locale's weekend end day.
    pub weekend_end: Weekday,

    /// The minimal number of days in the first week of a year.
    pub minimal_days_in_first_week: u8,
}

impl WeekInfo {
    /// Returns default CLDR-like week preferences for the locale when a runtime provider
    /// is unavailable.
    #[must_use]
    pub fn for_locale(locale: &Locale) -> Self {
        let (default_first_day, weekend_start, weekend_end) = match locale.region() {
            Some("US" | "CA" | "AU" | "NZ" | "PH" | "BR" | "JP" | "TH") => {
                (Weekday::Sunday, Weekday::Saturday, Weekday::Sunday)
            }

            Some("IL") => (Weekday::Sunday, Weekday::Friday, Weekday::Saturday),

            Some(
                "AE" | "BH" | "DZ" | "EG" | "IQ" | "JO" | "KW" | "LY" | "OM" | "QA" | "SA" | "SD"
                | "SY" | "YE",
            ) => (Weekday::Saturday, Weekday::Friday, Weekday::Saturday),

            _ => (Weekday::Monday, Weekday::Saturday, Weekday::Sunday),
        };

        // Mirrors CLDR supplemental/weekData minDays territory data:
        // unlisted regions inherit the `001` default of 1, while this explicit set uses 4.
        let minimal_days_in_first_week = match locale.region() {
            Some(
                "AD" | "AN" | "AT" | "AX" | "BE" | "BG" | "CH" | "CZ" | "DE" | "DK" | "EE" | "ES"
                | "FI" | "FJ" | "FO" | "FR" | "GB" | "GF" | "GG" | "GI" | "GP" | "GR" | "HU" | "IE"
                | "IM" | "IS" | "IT" | "JE" | "LI" | "LT" | "LU" | "MC" | "MQ" | "NL" | "NO" | "PL"
                | "PT" | "RE" | "RU" | "SE" | "SJ" | "SK" | "SM" | "VA",
            ) => 4,
            _ => 1,
        };

        let first_day = locale
            .first_day_of_week_extension()
            .unwrap_or(default_first_day);

        Self {
            first_day,
            weekend_start,
            weekend_end,
            minimal_days_in_first_week,
        }
    }
}

/// Errors returned by `Time` constructors and wall-clock operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateError {
    /// The requested date or time fields do not form a valid value.
    InvalidDate,

    /// The requested date or time is outside the supported range.
    OutOfRange,

    /// The calendar engine rejected the request.
    CalendarError(String),
}

impl Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate => f.write_str("invalid date"),
            Self::OutOfRange => f.write_str("date out of range"),
            Self::CalendarError(message) => write!(f, "calendar error: {message}"),
        }
    }
}

impl core::error::Error for DateError {}

/// Errors returned by date arithmetic and time-zone aware operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarError {
    /// Calendar arithmetic failed.
    Arithmetic(String),
}

impl Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arithmetic(message) => write!(f, "calendar arithmetic failed: {message}"),
        }
    }
}

impl core::error::Error for CalendarError {}

/// Errors returned when converting between calendar systems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarConversionError {
    /// The input date was not valid in its source calendar.
    InvalidDate,

    /// The requested conversion failed in the underlying engine.
    Icu(String),
}

impl Display for CalendarConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate => f.write_str("invalid calendar date"),
            Self::Icu(message) => write!(f, "calendar conversion failed: {message}"),
        }
    }
}

impl core::error::Error for CalendarConversionError {}

pub(crate) const SUPPORTED_CALENDARS: [CalendarMetadata; 15] = [
    CalendarMetadata {
        calendar: CalendarSystem::Iso8601,
        bcp47: "iso8601",
        has_custom_eras: false,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Gregorian,
        bcp47: "gregory",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Buddhist,
        bcp47: "buddhist",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Japanese,
        bcp47: "japanese",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Hebrew,
        bcp47: "hebrew",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::IslamicCivil,
        bcp47: "islamic-civil",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::IslamicUmmAlQura,
        bcp47: "islamic-umalqura",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Persian,
        bcp47: "persian",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Indian,
        bcp47: "indian",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Chinese,
        bcp47: "chinese",
        has_custom_eras: false,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Coptic,
        bcp47: "coptic",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Dangi,
        bcp47: "dangi",
        has_custom_eras: false,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Ethiopic,
        bcp47: "ethiopic",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::EthiopicAmeteAlem,
        bcp47: "ethioaa",
        has_custom_eras: true,
    },
    CalendarMetadata {
        calendar: CalendarSystem::Roc,
        bcp47: "roc",
        has_custom_eras: true,
    },
];

pub(crate) const JAPANESE_ERAS: [JapaneseEra; 5] = [
    JapaneseEra {
        code: "meiji",
        english_name: "Meiji",
        japanese_name: "明治",
        start_year: 1868,
        start_month: 9,
        start_day: 8,
    },
    JapaneseEra {
        code: "taisho",
        english_name: "Taisho",
        japanese_name: "大正",
        start_year: 1912,
        start_month: 7,
        start_day: 30,
    },
    JapaneseEra {
        code: "showa",
        english_name: "Showa",
        japanese_name: "昭和",
        start_year: 1926,
        start_month: 12,
        start_day: 25,
    },
    JapaneseEra {
        code: "heisei",
        english_name: "Heisei",
        japanese_name: "平成",
        start_year: 1989,
        start_month: 1,
        start_day: 8,
    },
    JapaneseEra {
        code: "reiwa",
        english_name: "Reiwa",
        japanese_name: "令和",
        start_year: 2019,
        start_month: 5,
        start_day: 1,
    },
];

pub(crate) fn canonical_era(calendar: CalendarSystem, code: &str) -> Era {
    if matches!(
        calendar,
        CalendarSystem::IslamicCivil | CalendarSystem::IslamicUmmAlQura
    ) && matches!(code, "ah" | "bh")
    {
        return Era {
            code: String::from("ah"),
            display_name: String::from("AH"),
        };
    }

    if calendar == CalendarSystem::Coptic && matches!(code, "am" | "ce") {
        return Era {
            code: String::from("ce"),
            display_name: String::from("CE"),
        };
    }

    if calendar == CalendarSystem::Gregorian {
        let (public_code, display_name) = match code {
            "bce" | "bc" => ("bc", "BC"),

            "ce" | "ad" => ("ad", "AD"),

            _ => (code, code),
        };
        return Era {
            code: String::from(public_code),
            display_name: String::from(display_name),
        };
    }

    if let Some(definition) = era_definition(calendar, code) {
        return Era {
            code: String::from(definition.code),
            display_name: String::from(definition.display_name),
        };
    }

    Era {
        code: String::from(code),
        display_name: String::from(code),
    }
}

pub(crate) fn infer_public_era(
    calendar: CalendarSystem,
    iso_year: i32,
    iso_month: u8,
    iso_day: u8,
) -> Option<Era> {
    match calendar {
        CalendarSystem::Coptic => Some(if (iso_year, iso_month, iso_day) < (284, 8, 29) {
            Era {
                code: String::from("bce"),
                display_name: String::from("BCE"),
            }
        } else {
            Era {
                code: String::from("ce"),
                display_name: String::from("CE"),
            }
        }),
        _ => None,
    }
}

pub(crate) fn month_code_from_temporal(code: temporal_rs::MonthCode) -> MonthCode {
    MonthCode::from_temporal(code)
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained for formatter/calendar annotation follow-up work"
    )
)]
pub(crate) fn temporal_calendar_identifier(calendar: CalendarSystem) -> &'static str {
    temporal_calendar_for(calendar).map_or(calendar.to_bcp47_value(), |value| value.identifier())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Locale, Weekday};

    fn gregorian_date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("Gregorian fixture should validate")
    }

    #[test]
    fn calendar_system_roundtrips_bcp47_metadata_and_locale_extensions() {
        for metadata in CalendarSystem::supported_calendars() {
            assert_eq!(metadata.calendar.to_bcp47_value(), metadata.bcp47);
            assert_eq!(
                CalendarSystem::from_bcp47(metadata.bcp47),
                Some(metadata.calendar)
            );
            assert_eq!(
                metadata.calendar.has_custom_eras(),
                metadata.has_custom_eras
            );
        }

        let buddhist = Locale::parse("th-TH-u-ca-buddhist").expect("locale should parse");

        let defaulted = Locale::parse("en-US").expect("locale should parse");

        assert_eq!(
            CalendarSystem::from_locale(&buddhist),
            CalendarSystem::Buddhist
        );
        assert_eq!(
            CalendarSystem::from_locale(&defaulted),
            CalendarSystem::Gregorian
        );
    }

    #[test]
    fn calendar_system_helpers_cover_japanese_bounds_and_unbounded_calendars() {
        let japanese = CalendarDate::new(
            CalendarSystem::Japanese,
            &crate::calendar::CalendarDateFields {
                era: Some(Era {
                    code: String::from("reiwa"),
                    display_name: String::from("Reiwa"),
                }),
                year: Some(1),
                month: Some(5),
                day: Some(1),
                ..crate::calendar::CalendarDateFields::default()
            },
        )
        .expect("Japanese boundary date should validate");

        let chinese = gregorian_date(2024, 3, 15)
            .to_calendar(CalendarSystem::Chinese)
            .expect("Chinese conversion should succeed");

        assert_eq!(CalendarSystem::Japanese.minimum_month_in_year(&japanese), 5);
        assert_eq!(CalendarSystem::Japanese.minimum_day_in_month(&japanese), 1);
        assert_eq!(CalendarSystem::Chinese.minimum_month_in_year(&chinese), 1);
        assert_eq!(CalendarSystem::Chinese.minimum_day_in_month(&chinese), 1);
        assert_eq!(CalendarSystem::Chinese.years_in_era(&chinese), None);
    }

    #[test]
    fn japanese_era_localization_and_special_era_canonicalization_are_stable() {
        let ja = Locale::parse("ja-JP").expect("locale should parse");

        let en = Locale::parse("en-US").expect("locale should parse");

        assert_eq!(
            CalendarSystem::japanese_eras()[4].localized_name(&ja),
            "令和"
        );
        assert_eq!(
            CalendarSystem::japanese_eras()[4].localized_name(&en),
            "Reiwa"
        );

        assert_eq!(
            canonical_era(CalendarSystem::IslamicUmmAlQura, "bh"),
            Era {
                code: String::from("ah"),
                display_name: String::from("AH"),
            }
        );
        assert_eq!(
            canonical_era(CalendarSystem::Gregorian, "bce"),
            Era {
                code: String::from("bc"),
                display_name: String::from("BC"),
            }
        );
        assert_eq!(
            canonical_era(CalendarSystem::Coptic, "am"),
            Era {
                code: String::from("ce"),
                display_name: String::from("CE"),
            }
        );
    }

    #[test]
    fn unsupported_islamic_identifier_falls_back_to_default_locale_calendar() {
        assert_eq!(CalendarSystem::from_bcp47("islamic"), None);
        assert_eq!(
            CalendarSystem::from_locale(
                &Locale::parse("en-u-ca-islamic").expect("locale should parse")
            ),
            CalendarSystem::Gregorian
        );
        assert!(
            CalendarSystem::supported_calendars()
                .iter()
                .all(|metadata| metadata.bcp47 != "islamic")
        );
    }

    #[test]
    fn month_code_and_time_zone_validation_cover_success_and_error_paths() {
        let normal = MonthCode::new_normal(5).expect("normal month code should validate");

        assert_eq!(normal.as_str(), "M05");
        assert!(!normal.is_leap_month());
        assert!(MonthCode::new_normal(0).is_err());
        assert!(MonthCode::new_normal(14).is_err());

        let utc = TimeZoneId::new("UTC").expect("UTC should validate");

        assert_eq!(utc.as_str(), "UTC");
        #[cfg(feature = "std")]
        assert!(TimeZoneId::new("Not/A_Zone").is_err());
        #[cfg(not(feature = "std"))]
        assert!(TimeZoneId::new("").is_err());
    }

    #[test]
    fn week_info_uses_region_defaults_and_first_day_overrides() {
        let us = WeekInfo::for_locale(&Locale::parse("en-US").expect("locale should parse"));

        let il = WeekInfo::for_locale(&Locale::parse("he-IL").expect("locale should parse"));

        let ae = WeekInfo::for_locale(&Locale::parse("ar-AE").expect("locale should parse"));

        let china = WeekInfo::for_locale(&Locale::parse("zh-CN").expect("locale should parse"));

        let korea = WeekInfo::for_locale(&Locale::parse("ko-KR").expect("locale should parse"));

        let turkey = WeekInfo::for_locale(&Locale::parse("tr-TR").expect("locale should parse"));

        let indonesia = WeekInfo::for_locale(&Locale::parse("id-ID").expect("locale should parse"));

        let override_locale =
            WeekInfo::for_locale(&Locale::parse("en-US-u-fw-mon").expect("locale should parse"));

        assert_eq!(
            (us.first_day, us.weekend_start, us.weekend_end),
            (Weekday::Sunday, Weekday::Saturday, Weekday::Sunday)
        );
        assert_eq!(
            (il.first_day, il.weekend_start, il.weekend_end),
            (Weekday::Sunday, Weekday::Friday, Weekday::Saturday)
        );
        assert_eq!(
            (ae.first_day, ae.weekend_start, ae.weekend_end),
            (Weekday::Saturday, Weekday::Friday, Weekday::Saturday)
        );
        assert_eq!(override_locale.first_day, Weekday::Monday);
        assert_eq!(us.minimal_days_in_first_week, 1);
        assert_eq!(ae.minimal_days_in_first_week, 1);
        assert_eq!(
            WeekInfo::for_locale(&Locale::parse("de-DE").expect("locale should parse"))
                .minimal_days_in_first_week,
            4
        );
        assert_eq!(china.minimal_days_in_first_week, 1);
        assert_eq!(korea.minimal_days_in_first_week, 1);
        assert_eq!(turkey.minimal_days_in_first_week, 1);
        assert_eq!(indonesia.minimal_days_in_first_week, 1);
        assert_eq!(override_locale.minimal_days_in_first_week, 1);
    }

    #[test]
    fn error_types_and_temporal_identifiers_format_stably() {
        assert_eq!(DateError::InvalidDate.to_string(), "invalid date");
        assert_eq!(DateError::OutOfRange.to_string(), "date out of range");
        assert_eq!(
            CalendarError::Arithmetic(String::from("boom")).to_string(),
            "calendar arithmetic failed: boom"
        );
        assert_eq!(
            CalendarConversionError::InvalidDate.to_string(),
            "invalid calendar date"
        );
        assert_eq!(
            CalendarConversionError::Icu(String::from("boom")).to_string(),
            "calendar conversion failed: boom"
        );
        assert_eq!(
            temporal_calendar_identifier(CalendarSystem::EthiopicAmeteAlem),
            "ethioaa"
        );
        assert_eq!(
            CalendarSystem::Gregorian.to_icu_kind(),
            AnyCalendarKind::Gregorian
        );
    }

    #[test]
    fn calendar_metadata_helpers_cover_unknown_and_special_case_paths() {
        assert_eq!(CalendarSystem::from_bcp47("unknown-calendar"), None);

        for metadata in CalendarSystem::supported_calendars() {
            let calendar = metadata.calendar;

            assert_eq!(calendar.to_bcp47_value(), metadata.bcp47);

            let _ = calendar.to_icu_kind();
        }

        assert_eq!(
            canonical_era(CalendarSystem::Gregorian, "custom"),
            Era {
                code: String::from("custom"),
                display_name: String::from("custom"),
            }
        );
        assert_eq!(
            CalendarSystem::Roc.eras(),
            vec![
                Era {
                    code: String::from("broc"),
                    display_name: String::from("Before ROC"),
                },
                Era {
                    code: String::from("roc"),
                    display_name: String::from("ROC"),
                },
            ]
        );
        assert_eq!(
            CalendarSystem::Coptic.eras(),
            vec![
                Era {
                    code: String::from("bce"),
                    display_name: String::from("BCE"),
                },
                Era {
                    code: String::from("ce"),
                    display_name: String::from("CE"),
                },
            ]
        );
        assert_eq!(
            CalendarSystem::Ethiopic.eras(),
            vec![
                Era {
                    code: String::from("aa"),
                    display_name: String::from("AA"),
                },
                Era {
                    code: String::from("am"),
                    display_name: String::from("AM"),
                },
            ]
        );
        assert_eq!(
            MonthCode::new("M05")
                .expect("month code should validate")
                .to_string(),
            "M05"
        );
        assert_eq!(
            TimeZoneId::new("UTC")
                .expect("UTC should validate")
                .to_string(),
            "UTC"
        );
    }

    /// Wasm smoke tests for `calendar/system.rs`. See the module-level note
    /// in `date.rs`'s `wasm_tests` block for the rationale.
    #[cfg(all(target_arch = "wasm32", feature = "web-intl"))]
    mod wasm_tests {
        use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

        use super::*;

        wasm_bindgen_test_configure!(run_in_browser);

        #[wasm_bindgen_test]
        fn wasm_calendar_system_bcp47_roundtrips_supported_identifiers() {
            for (identifier, calendar) in [
                ("iso8601", CalendarSystem::Iso8601),
                ("gregory", CalendarSystem::Gregorian),
                ("japanese", CalendarSystem::Japanese),
                ("roc", CalendarSystem::Roc),
            ] {
                assert_eq!(CalendarSystem::from_bcp47(identifier), Some(calendar));
                assert_eq!(calendar.to_bcp47_value(), identifier);
            }

            assert_eq!(CalendarSystem::from_bcp47("bogus"), None);
        }

        #[wasm_bindgen_test]
        fn wasm_calendar_system_japanese_minimum_helpers_track_reiwa_era_start() {
            let japanese = CalendarDate::new(
                CalendarSystem::Japanese,
                &crate::calendar::CalendarDateFields {
                    era: Some(Era {
                        code: String::from("reiwa"),
                        display_name: String::from("Reiwa"),
                    }),
                    year: Some(1),
                    month: Some(5),
                    day: Some(1),
                    ..crate::calendar::CalendarDateFields::default()
                },
            )
            .expect("Japanese boundary date should validate");

            assert_eq!(CalendarSystem::Japanese.minimum_month_in_year(&japanese), 5);
            assert_eq!(CalendarSystem::Japanese.minimum_day_in_month(&japanese), 1);
        }
    }
}
