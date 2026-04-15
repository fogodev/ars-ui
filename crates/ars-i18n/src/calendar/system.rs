use alloc::{format, string::String};
use core::{
    fmt::{self, Display},
    ops::RangeInclusive,
};

use icu::calendar::AnyCalendarKind;

use crate::{IcuProvider, Locale, Weekday};

/// Supported calendar systems used throughout ars-ui.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CalendarSystem {
    /// Proleptic Gregorian calendar.
    #[default]
    Gregorian,

    /// Buddhist Era calendar.
    Buddhist,

    /// Japanese imperial calendar.
    Japanese,

    /// Hebrew calendar.
    Hebrew,

    /// Astronomical Hijri calendar using the simulated Mecca calculation.
    Islamic,

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
    ///
    /// Deprecated aliases such as `japanext` are normalized onto the current
    /// public calendar variants.
    #[must_use]
    pub fn from_bcp47(identifier: &str) -> Option<Self> {
        match identifier {
            "gregory" | "gregorian" => Some(Self::Gregorian),
            "buddhist" => Some(Self::Buddhist),
            "japanese" | "japanext" => Some(Self::Japanese),
            "hebrew" => Some(Self::Hebrew),
            "islamic" => Some(Self::Islamic),
            "islamic-civil" => Some(Self::IslamicCivil),
            "islamic-umalqura" => Some(Self::IslamicUmmAlQura),
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
    pub const fn to_icu_kind(&self) -> AnyCalendarKind {
        match self {
            Self::Gregorian => AnyCalendarKind::Gregorian,
            Self::Buddhist => AnyCalendarKind::Buddhist,
            Self::Japanese => AnyCalendarKind::Japanese,
            Self::Hebrew => AnyCalendarKind::Hebrew,
            Self::Islamic => AnyCalendarKind::HijriSimulatedMecca,
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

    /// Returns `true` if this calendar typically carries named eras beyond
    /// plain CE/BCE handling.
    #[must_use]
    pub const fn has_custom_eras(&self) -> bool {
        matches!(
            self,
            Self::Japanese
                | Self::Ethiopic
                | Self::EthiopicAmeteAlem
                | Self::Coptic
                | Self::Hebrew
                | Self::Persian
                | Self::Islamic
                | Self::IslamicCivil
                | Self::IslamicUmmAlQura
                | Self::Roc
        )
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
}

/// Metadata describing the validation envelope for a calendar system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarMetadata {
    /// The calendar system the metadata applies to.
    pub system: CalendarSystem,

    /// The inclusive month ordinal range the calendar may use.
    pub month_range: RangeInclusive<u8>,

    /// Whether the calendar may introduce leap months.
    pub has_leap_months: bool,

    /// Whether selecting dates in the calendar requires an explicit named era.
    pub era_required: bool,

    /// Representative year lengths supported by the calendar.
    pub typical_year_lengths: &'static [u16],
}

/// A named Japanese imperial era.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JapaneseEra {
    /// The romanized era name.
    pub name: &'static str,

    /// The Gregorian start year of the era.
    pub start_year: u32,
}

impl JapaneseEra {
    /// Returns the localized era name for the given locale.
    #[must_use]
    pub fn localized_name(&self, locale: &Locale) -> String {
        if locale.language() == "ja" {
            String::from(self.native_name())
        } else {
            String::from(self.romanized_name())
        }
    }

    /// Returns the romanized era name.
    #[must_use]
    pub const fn romanized_name(&self) -> &str {
        self.name
    }

    fn native_name(&self) -> &str {
        match self.name {
            "Reiwa" => "令和",
            "Heisei" => "平成",
            "Showa" => "昭和",
            "Taisho" => "大正",
            "Meiji" => "明治",
            other => other,
        }
    }
}

/// Week numbering information for a locale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekInfo {
    /// The locale's first day of the week.
    pub first_day: Weekday,

    /// The minimum number of days required in the first week of the year.
    pub min_days_in_first_week: u8,
}

impl WeekInfo {
    /// Computes week information from locale region and `u-fw-*` override data.
    #[must_use]
    pub fn for_locale(locale: &Locale) -> Self {
        let region = locale.region().unwrap_or("");

        let min_days = match region {
            "US" | "CA" | "MX" | "AU" | "JP" | "CN" | "TW" | "HK" | "KR" | "SG" | "AF" | "IR"
            | "SA" | "AE" | "EG" | "DZ" | "MA" | "TN" | "LY" => 1,
            _ => 4,
        };

        if let Some(first_day) = locale.first_day_of_week_extension() {
            return Self {
                first_day,
                min_days_in_first_week: min_days,
            };
        }

        let first_day = match region {
            "US" | "CA" | "MX" | "AU" | "JP" | "CN" | "TW" | "HK" | "KR" | "SG" => Weekday::Sunday,
            "AF" | "IR" | "SA" | "AE" | "EG" | "DZ" | "MA" | "TN" | "LY" => Weekday::Saturday,
            _ => Weekday::Monday,
        };

        Self {
            first_day,
            min_days_in_first_week: min_days,
        }
    }

    /// Returns weekdays ordered for calendar header rendering.
    #[must_use]
    pub fn ordered_weekdays(&self) -> [Weekday; 7] {
        let all = [
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
        ];

        let start = all
            .iter()
            .position(|weekday| *weekday == self.first_day)
            .unwrap_or(1);

        let mut ordered = [Weekday::Monday; 7];

        let mut index = 0;

        while index < 7 {
            ordered[index] = all[(start + index) % 7];
            index += 1;
        }

        ordered
    }
}

/// Error returned when a date's components are invalid or out of bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateError {
    /// The provided year, month, and day do not form a valid date.
    InvalidDate,

    /// The provided date falls outside the representable range.
    OutOfRange,

    /// The calendar backend returned a detailed validation error.
    CalendarError(String),
}

impl Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate => write!(f, "invalid date"),
            Self::OutOfRange => write!(f, "date out of range"),
            Self::CalendarError(message) => write!(f, "calendar error: {message}"),
        }
    }
}

impl core::error::Error for DateError {}

/// Error returned by internal calendar arithmetic operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarError {
    /// An arithmetic or platform date conversion failure.
    Arithmetic(String),
}

impl Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arithmetic(message) => write!(f, "calendar arithmetic error: {message}"),
        }
    }
}

impl core::error::Error for CalendarError {}

/// Error returned when constructing or converting an internal ICU-backed date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarConversionError {
    /// The provided fields are invalid for the requested target calendar.
    InvalidDate,

    /// ICU4X reported a conversion failure.
    Icu(String),
}

impl Display for CalendarConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate => write!(f, "invalid date for target calendar"),
            Self::Icu(message) => write!(f, "ICU4X calendar conversion error: {message}"),
        }
    }
}

impl core::error::Error for CalendarConversionError {}

/// A named era carried by a public `CalendarDate`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Era {
    /// The stable era code such as `reiwa` or `showa`.
    pub code: String,

    /// The locale-facing display name for the era.
    pub display_name: String,
}

/// A wall-clock time with no associated date or time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    /// The hour in 24-hour form.
    pub hour: u8,

    /// The minute component.
    pub minute: u8,

    /// The second component.
    pub second: u8,

    /// Optional millisecond precision.
    pub millisecond: u16,
}

impl Time {
    /// Creates a time with second precision.
    #[must_use]
    pub const fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
            millisecond: 0,
        }
    }

    /// Returns the hour in `1..=12` display form.
    #[must_use]
    pub const fn hour_12(&self) -> u8 {
        match self.hour % 12 {
            0 => 12,
            value => value,
        }
    }

    /// Returns `true` when the time is in the PM half of the day.
    #[must_use]
    pub const fn is_pm(&self) -> bool {
        self.hour >= 12
    }

    /// Returns the time as an ISO 8601 time string.
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        if self.millisecond > 0 {
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                self.hour, self.minute, self.second, self.millisecond
            )
        } else {
            format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
        }
    }
}

/// Hour cycle preference for time display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HourCycle {
    /// `0..=11` with a day-period indicator.
    H11,

    /// `1..=12` with a day-period indicator.
    H12,

    /// `0..=23` with no day-period indicator.
    H23,

    /// `1..=24` with no day-period indicator.
    H24,

    /// Resolve from locale-specific defaults.
    Auto,
}

impl HourCycle {
    /// Returns `true` when the cycle includes a day-period segment.
    #[must_use]
    pub const fn has_day_period(&self) -> bool {
        matches!(self, Self::H11 | Self::H12)
    }

    /// Resolves `Auto` through the locale's provider-backed default.
    #[must_use]
    pub fn resolve(&self, provider: &dyn IcuProvider, locale: &Locale) -> HourCycle {
        match self {
            Self::Auto => locale.hour_cycle(provider),
            other => *other,
        }
    }

    /// Returns the inclusive display-hour range for the cycle.
    #[must_use]
    pub const fn display_hour_range(&self) -> (u8, u8) {
        match self {
            Self::H11 => (0, 11),
            Self::H12 => (1, 12),
            Self::H23 | Self::Auto => (0, 23),
            Self::H24 => (1, 24),
        }
    }
}

const JAPANESE_ERAS: [JapaneseEra; 5] = [
    JapaneseEra {
        name: "Meiji",
        start_year: 1868,
    },
    JapaneseEra {
        name: "Taisho",
        start_year: 1912,
    },
    JapaneseEra {
        name: "Showa",
        start_year: 1926,
    },
    JapaneseEra {
        name: "Heisei",
        start_year: 1989,
    },
    JapaneseEra {
        name: "Reiwa",
        start_year: 2019,
    },
];

const SUPPORTED_CALENDARS: [CalendarMetadata; 15] = [
    CalendarMetadata {
        system: CalendarSystem::Gregorian,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Buddhist,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Japanese,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: true,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Hebrew,
        month_range: 1..=13,
        has_leap_months: true,
        era_required: false,
        typical_year_lengths: &[353, 354, 355, 383, 384, 385],
    },
    CalendarMetadata {
        system: CalendarSystem::Islamic,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[354, 355],
    },
    CalendarMetadata {
        system: CalendarSystem::IslamicCivil,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[354, 355],
    },
    CalendarMetadata {
        system: CalendarSystem::IslamicUmmAlQura,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[354, 355],
    },
    CalendarMetadata {
        system: CalendarSystem::Persian,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Indian,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Chinese,
        month_range: 1..=13,
        has_leap_months: true,
        era_required: false,
        typical_year_lengths: &[353, 354, 355, 383, 384, 385],
    },
    CalendarMetadata {
        system: CalendarSystem::Coptic,
        month_range: 1..=13,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Dangi,
        month_range: 1..=13,
        has_leap_months: true,
        era_required: false,
        typical_year_lengths: &[353, 354, 355, 383, 384, 385],
    },
    CalendarMetadata {
        system: CalendarSystem::Ethiopic,
        month_range: 1..=13,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::EthiopicAmeteAlem,
        month_range: 1..=13,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
    CalendarMetadata {
        system: CalendarSystem::Roc,
        month_range: 1..=12,
        has_leap_months: false,
        era_required: false,
        typical_year_lengths: &[365, 366],
    },
];
