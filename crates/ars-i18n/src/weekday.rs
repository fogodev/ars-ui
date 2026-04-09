use icu::calendar::types::Weekday as IcuWeekday;

/// Canonical ISO 8601 weekday.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// Creates a weekday from a Sunday-zero-indexed number.
    #[must_use]
    pub fn from_sunday_zero(n: u8) -> Self {
        const WEEKDAYS: [Weekday; 7] = [
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
        ];

        WEEKDAYS[usize::from(n % 7)]
    }

    /// Creates a weekday from ISO 8601 numbering.
    #[must_use]
    pub const fn from_iso_8601(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            7 => Some(Self::Sunday),
            _ => None,
        }
    }

    /// Parses a weekday from ICU/CLDR short codes such as `"mon"` or `"sun"`.
    #[must_use]
    pub fn from_icu_str(s: &str) -> Option<Self> {
        match s {
            "mon" => Some(Self::Monday),
            "tue" => Some(Self::Tuesday),
            "wed" => Some(Self::Wednesday),
            "thu" => Some(Self::Thursday),
            "fri" => Some(Self::Friday),
            "sat" => Some(Self::Saturday),
            "sun" => Some(Self::Sunday),
            _ => None,
        }
    }

    /// Parses a weekday from a BCP 47 `-u-fw-` extension value.
    #[must_use]
    pub fn from_bcp47_fw(s: &str) -> Option<Self> {
        Self::from_icu_str(s)
    }

    /// Converts from ICU4X's weekday enum.
    #[must_use]
    pub const fn from_icu_weekday(weekday: IcuWeekday) -> Self {
        match weekday {
            IcuWeekday::Monday => Self::Monday,
            IcuWeekday::Tuesday => Self::Tuesday,
            IcuWeekday::Wednesday => Self::Wednesday,
            IcuWeekday::Thursday => Self::Thursday,
            IcuWeekday::Friday => Self::Friday,
            IcuWeekday::Saturday => Self::Saturday,
            IcuWeekday::Sunday => Self::Sunday,
        }
    }
}
