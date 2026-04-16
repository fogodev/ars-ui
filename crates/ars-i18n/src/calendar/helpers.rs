use alloc::{format, string::String};
use core::cmp::Ordering;

use super::{CalendarDate, CalendarSystem, Era};
use crate::IcuProvider;

/// `iso_to_epoch_days(0001-01-01)` is defined as 0. The conversion helpers use
/// Julian day arithmetic internally, so this offset bridges the civil epoch used
/// by the public API and the Julian day numbering used by the formula.
pub(super) const GREGORIAN_JDN_OFFSET: i64 = 1_721_426;

/// Number of days between `0001-01-01` and the Unix epoch `1970-01-01`.
#[cfg(all(
    any(feature = "icu4x", feature = "web-intl"),
    feature = "std",
    not(target_arch = "wasm32")
))]
const UNIX_EPOCH_DAYS_FROM_CE: i64 = 719_162;

/// Returns the number of days in a proleptic Gregorian month.
///
/// The leap-year branch uses the normal Gregorian 400-year rule:
/// divisible by 4, except centuries that are not divisible by 400.
pub(crate) const fn gregorian_days_in_month(year: i32, month: u8) -> u8 {
    if month == 0 || month > 12 {
        return 30;
    }

    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,

        2 => {
            let is_leap_year = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            if is_leap_year { 29 } else { 28 }
        }

        _ => 30,
    }
}

/// Returns `true` when a Coptic-style year carries a sixth epagomenal day.
///
/// Coptic and Ethiopic calendars insert the leap day into month 13, so the
/// leap-year rule affects only that final month.
pub(crate) const fn coptic_like_is_leap_year(year: i32) -> bool {
    // Coptic-style leap years are the years whose following Gregorian-style
    // ordinal would be divisible by four. Widen before adding so the accepted
    // `i32::MAX` year remains well-defined instead of overflowing here.
    ((year as i64) + 1) % 4 == 0
}

/// Returns the number of days in a Coptic or Ethiopic month.
///
/// Months 1-12 are always 30 days. Month 13 has 5 days in common years and
/// 6 days in leap years.
pub(crate) const fn coptic_like_days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1..=12 => 30,
        13 => 5 + coptic_like_is_leap_year(year) as u8,
        _ => 0,
    }
}

pub(super) fn max_months_in_year(
    provider: &dyn IcuProvider,
    calendar: CalendarSystem,
    year: i32,
    era: Option<&str>,
) -> u8 {
    provider.max_months_in_year(&calendar, year, era)
}

pub(super) fn days_in_month_for_calendar(
    provider: &dyn IcuProvider,
    calendar: CalendarSystem,
    year: i32,
    month: u8,
    era: Option<&str>,
) -> u8 {
    provider.days_in_month(&calendar, year, month, era)
}

pub(super) fn era_code_is_valid(calendar: CalendarSystem, era: Option<&str>) -> bool {
    match calendar {
        CalendarSystem::Japanese => {
            era.is_some_and(|era_code| japanese_era_boundary(era_code).is_some())
        }
        _ => era.is_none(),
    }
}

#[derive(Clone, Copy)]
struct EraBoundary {
    code: &'static str,
    display_name: &'static str,
    start_year: i32,
    start_month: u8,
    start_day: u8,
    end: Option<(i32, u8, u8)>,
}

const JAPANESE_ERA_BOUNDARIES: [EraBoundary; 5] = [
    EraBoundary {
        code: "meiji",
        display_name: "Meiji",
        start_year: 1868,
        start_month: 9,
        start_day: 8,
        end: Some((1912, 7, 29)),
    },
    EraBoundary {
        code: "taisho",
        display_name: "Taisho",
        start_year: 1912,
        start_month: 7,
        start_day: 30,
        end: Some((1926, 12, 24)),
    },
    EraBoundary {
        code: "showa",
        display_name: "Showa",
        start_year: 1926,
        start_month: 12,
        start_day: 25,
        end: Some((1989, 1, 7)),
    },
    EraBoundary {
        code: "heisei",
        display_name: "Heisei",
        start_year: 1989,
        start_month: 1,
        start_day: 8,
        end: Some((2019, 4, 30)),
    },
    EraBoundary {
        code: "reiwa",
        display_name: "Reiwa",
        start_year: 2019,
        start_month: 5,
        start_day: 1,
        end: None,
    },
];

fn japanese_era_boundary(code: &str) -> Option<&'static EraBoundary> {
    JAPANESE_ERA_BOUNDARIES
        .iter()
        .find(|boundary| boundary.code == code)
}

fn japanese_era_max_year(boundary: &EraBoundary) -> Option<i32> {
    let (end_year, ..) = boundary.end?;

    Some(end_year - boundary.start_year + 1)
}

pub(crate) fn default_era_for(calendar: CalendarSystem) -> Option<Era> {
    match calendar {
        CalendarSystem::Japanese => JAPANESE_ERA_BOUNDARIES.last().map(|boundary| Era {
            code: String::from(boundary.code),
            display_name: String::from(boundary.display_name),
        }),

        _ => None,
    }
}

pub(crate) fn years_in_era(date: &CalendarDate) -> Option<i32> {
    match date.calendar {
        CalendarSystem::Japanese => {
            let era = date.era.as_ref()?;

            japanese_era_max_year(japanese_era_boundary(era.code.as_str())?)
        }

        _ => None,
    }
}

pub(crate) fn minimum_month_in_year(date: &CalendarDate) -> u8 {
    match date.calendar {
        CalendarSystem::Japanese => {
            let Some(era) = date.era.as_ref() else {
                return 1;
            };

            let Some(boundary) = japanese_era_boundary(era.code.as_str()) else {
                return 1;
            };

            if date.year == 1 {
                boundary.start_month
            } else {
                1
            }
        }
        _ => 1,
    }
}

pub(crate) fn minimum_day_in_month(date: &CalendarDate) -> u8 {
    match date.calendar {
        CalendarSystem::Japanese => {
            let Some(era) = date.era.as_ref() else {
                return 1;
            };

            let Some(boundary) = japanese_era_boundary(era.code.as_str()) else {
                return 1;
            };

            if date.year == 1 && date.month.get() == boundary.start_month {
                boundary.start_day
            } else {
                1
            }
        }
        _ => 1,
    }
}

pub(crate) fn bounded_months_in_year(
    calendar: CalendarSystem,
    year: i32,
    era: Option<&str>,
) -> Option<u8> {
    match calendar {
        CalendarSystem::Japanese => {
            let boundary = japanese_era_boundary(era?)?;

            match japanese_era_max_year(boundary) {
                Some(max_year) if year == max_year => boundary.end.map(|(_, month, _)| month),
                _ => Some(12),
            }
        }

        _ => None,
    }
}

pub(crate) fn bounded_days_in_month(
    calendar: CalendarSystem,
    year: i32,
    month: u8,
    era: Option<&str>,
) -> Option<u8> {
    match calendar {
        CalendarSystem::Japanese => {
            let boundary = japanese_era_boundary(era?)?;

            // Japanese public dates store `year` as year-in-era, so convert it
            // back to the underlying Gregorian year before using Gregorian
            // month-length rules.
            let absolute_year = boundary.start_year.checked_add(year)?.checked_sub(1)?;

            let mut max_day = gregorian_days_in_month(absolute_year, month);

            if let Some((end_year, end_month, end_day)) = boundary.end {
                // The final month of a historical era may end early, e.g.
                // Heisei ends on 2019-04-30 instead of the full month.
                let max_year = end_year - boundary.start_year + 1;

                if year == max_year && month == end_month {
                    max_day = max_day.min(end_day);
                }
            }

            Some(max_day)
        }
        _ => None,
    }
}

/// Inclusive date range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    /// The inclusive range start.
    pub start: CalendarDate,

    /// The inclusive range end.
    pub end: CalendarDate,
}

impl DateRange {
    /// Creates a range when `start <= end` within the same comparable calendar.
    #[must_use]
    pub fn new(start: CalendarDate, end: CalendarDate) -> Option<Self> {
        match start.compare_within_calendar(&end) {
            Some(Ordering::Less | Ordering::Equal) => Some(Self { start, end }),
            Some(Ordering::Greater) | None => None,
        }
    }

    /// Returns `true` if `date` is within the inclusive range.
    #[must_use]
    pub fn contains(&self, date: &CalendarDate) -> bool {
        date.is_between(&self.start, &self.end)
    }

    /// Returns the range in `start/end` raw ISO field form.
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        format!("{}/{}", self.start.to_iso8601(), self.end.to_iso8601())
    }

    /// Creates a valid range by ordering the two comparable inputs.
    #[must_use]
    pub fn normalized(first_date: CalendarDate, second_date: CalendarDate) -> Option<Self> {
        match first_date.compare_within_calendar(&second_date) {
            Some(Ordering::Less | Ordering::Equal) => Some(Self {
                start: first_date,
                end: second_date,
            }),

            Some(Ordering::Greater) => Some(Self {
                start: second_date,
                end: first_date,
            }),

            None => None,
        }
    }
}

/// Converts an ISO date to a day count relative to `0001-01-01`.
#[must_use]
pub(crate) fn iso_to_epoch_days(year: i32, month: u8, day: u8) -> i64 {
    // Shift Jan/Feb to the end of the previous year so the leap-day correction
    // can be handled uniformly with a March-based year.
    let year_adjustment = (14 - i64::from(month)) / 12;
    let march_based_year = i64::from(year) + 4800 - year_adjustment;
    let march_based_month = i64::from(month) + 12 * year_adjustment - 3;

    // The remaining constants are from the standard Gregorian civil-date to
    // Julian day transform:
    // - `153` groups months into a repeating March-based pattern
    // - `32_045` removes the Julian-day epoch bias
    // - `GREGORIAN_JDN_OFFSET` rebases onto `0001-01-01 = 0`
    i64::from(day)
        + (153 * march_based_month + 2) / 5
        + 365 * march_based_year
        + march_based_year / 4
        - march_based_year / 100
        + march_based_year / 400
        - 32_045
        - GREGORIAN_JDN_OFFSET
}

/// Converts an epoch-day count back to an ISO date.
#[must_use]
pub(crate) const fn epoch_days_to_iso(epoch_days: i64) -> (i32, u8, u8) {
    // First move back into the Julian-day numbering domain expected by the
    // inverse civil-date algorithm.
    let julian_day = epoch_days + GREGORIAN_JDN_OFFSET + 32_044;

    // Split the day number across the Gregorian 400-year cycle (`146_097`
    // days), then the 4-year subcycle (`1_461` days).
    let century_cycles = (4 * julian_day + 3) / 146_097;
    let days_of_century = julian_day - (146_097 * century_cycles) / 4;
    let quad_year_cycles = (4 * days_of_century + 3) / 1_461;
    let day_of_year_block = days_of_century - (1_461 * quad_year_cycles) / 4;

    // `153` again encodes the March-based month pattern used by the forward
    // conversion. Converting back from that index yields the ISO month/year.
    let month_index = (5 * day_of_year_block + 2) / 153;
    let day = (day_of_year_block - (153 * month_index + 2) / 5 + 1) as u8;
    let month = (month_index + 3 - 12 * (month_index / 10)) as u8;
    let year = (100 * century_cycles + quad_year_cycles - 4800 + month_index / 10) as i32;

    (year, month, day)
}

#[cfg(all(
    any(feature = "icu4x", feature = "web-intl"),
    feature = "std",
    not(target_arch = "wasm32")
))]
pub(crate) fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?;

    // Convert whole UTC days since 1970-01-01 back into the crate's
    // `0001-01-01`-relative epoch used by `epoch_days_to_iso`.
    let days = i64::try_from(now.as_secs() / 86_400)
        .map_err(|_| String::from("system time exceeds supported range"))?;

    Ok(epoch_days_to_iso(days + UNIX_EPOCH_DAYS_FROM_CE))
}

#[cfg(all(
    any(feature = "icu4x", feature = "web-intl"),
    feature = "std",
    not(target_arch = "wasm32")
))]
// Keep the native `today()` plumbing reachable in library builds until the
// full provider/default-provider tasks wire these helpers through normal
// runtime call paths.
const _: i64 = UNIX_EPOCH_DAYS_FROM_CE;

#[cfg(all(
    any(feature = "icu4x", feature = "web-intl"),
    feature = "std",
    not(target_arch = "wasm32")
))]
// This is the same reachability trick for the function item itself; remove it
// once follow-up provider work gives `platform_today_iso()` a real caller.
const _: fn() -> Result<(i32, u8, u8), String> = platform_today_iso;

#[cfg(all(target_arch = "wasm32", feature = "web-intl"))]
pub(crate) fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    let date = js_sys::Date::new_0();

    let year = i32::try_from(date.get_utc_full_year())
        .map_err(|_| String::from("browser year out of range"))?;

    let month = u8::try_from(date.get_utc_month() + 1)
        .map_err(|_| String::from("browser month out of range"))?;

    let day =
        u8::try_from(date.get_utc_date()).map_err(|_| String::from("browser day out of range"))?;

    Ok((year, month, day))
}

#[cfg(all(feature = "icu4x", target_arch = "wasm32", not(feature = "web-intl")))]
pub(crate) fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    Err(String::from(
        "platform date unavailable on wasm32 without the `web-intl` feature",
    ))
}

#[cfg(all(
    any(feature = "icu4x", feature = "web-intl"),
    not(feature = "std"),
    not(target_arch = "wasm32")
))]
pub(crate) fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    Err(String::from(
        "platform date unavailable without the `std` feature",
    ))
}
