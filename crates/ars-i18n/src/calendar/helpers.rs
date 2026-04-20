use alloc::{format, string::String};
use core::cmp::Ordering;

use super::{CalendarDate, CalendarSystem, Era};

/// `iso_to_epoch_days(0001-01-01)` is defined as 0. The conversion helpers use
/// Julian day arithmetic internally, so this offset bridges the civil epoch used
/// by the public API and the Julian day numbering used by the formula.
#[cfg(test)]
pub(super) const GREGORIAN_JDN_OFFSET: i64 = 1_721_426;

/// Number of days between `0001-01-01` and the Unix epoch `1970-01-01`.
#[cfg(all(test, feature = "icu4x", feature = "std", not(target_arch = "wasm32")))]
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

#[derive(Clone, Copy)]
pub(crate) struct EraDefinition {
    pub(crate) code: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) max_year: Option<i32>,
}

#[derive(Clone, Copy)]
struct EraBoundary {
    code: &'static str,
    display_name: &'static str,
    start_year: i32,
    end: Option<(i32, u8, u8)>,
}

const JAPANESE_ERA_BOUNDARIES: [EraBoundary; 5] = [
    EraBoundary {
        code: "meiji",
        display_name: "Meiji",
        start_year: 1868,
        end: Some((1912, 7, 29)),
    },
    EraBoundary {
        code: "taisho",
        display_name: "Taisho",
        start_year: 1912,
        end: Some((1926, 12, 24)),
    },
    EraBoundary {
        code: "showa",
        display_name: "Showa",
        start_year: 1926,
        end: Some((1989, 1, 7)),
    },
    EraBoundary {
        code: "heisei",
        display_name: "Heisei",
        start_year: 1989,
        end: Some((2019, 4, 30)),
    },
    EraBoundary {
        code: "reiwa",
        display_name: "Reiwa",
        start_year: 2019,
        end: None,
    },
];

const BUDDHIST_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "be",
    display_name: "BE",
    max_year: Some(9999),
}];

const GREGORIAN_ERAS: [EraDefinition; 2] = [
    EraDefinition {
        code: "bc",
        display_name: "BC",
        max_year: Some(9999),
    },
    EraDefinition {
        code: "ad",
        display_name: "AD",
        max_year: Some(9999),
    },
];

const HEBREW_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "am",
    display_name: "AM",
    max_year: Some(9999),
}];

const ISLAMIC_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "ah",
    display_name: "AH",
    max_year: Some(9665),
}];

const PERSIAN_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "ap",
    display_name: "AP",
    max_year: Some(9377),
}];

const INDIAN_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "shaka",
    display_name: "Shaka",
    max_year: Some(9919),
}];

const COPTIC_ERAS: [EraDefinition; 2] = [
    EraDefinition {
        code: "bce",
        display_name: "BCE",
        max_year: Some(9999),
    },
    EraDefinition {
        code: "ce",
        display_name: "CE",
        max_year: Some(9715),
    },
];

const ETHIOPIC_ERAS: [EraDefinition; 2] = [
    EraDefinition {
        code: "aa",
        display_name: "AA",
        max_year: Some(9999),
    },
    EraDefinition {
        code: "am",
        display_name: "AM",
        max_year: Some(9991),
    },
];

const ETHIOPIC_AMETE_ALEM_ERAS: [EraDefinition; 1] = [EraDefinition {
    code: "aa",
    display_name: "AA",
    max_year: Some(9999),
}];

const ROC_ERAS: [EraDefinition; 2] = [
    EraDefinition {
        code: "broc",
        display_name: "Before ROC",
        max_year: Some(9999),
    },
    EraDefinition {
        code: "roc",
        display_name: "ROC",
        max_year: Some(8088),
    },
];

pub(crate) const fn era_definitions(calendar: CalendarSystem) -> &'static [EraDefinition] {
    match calendar {
        CalendarSystem::Gregorian => &GREGORIAN_ERAS,

        CalendarSystem::Buddhist => &BUDDHIST_ERAS,

        CalendarSystem::Hebrew => &HEBREW_ERAS,

        CalendarSystem::IslamicCivil | CalendarSystem::IslamicUmmAlQura => &ISLAMIC_ERAS,

        CalendarSystem::Persian => &PERSIAN_ERAS,

        CalendarSystem::Indian => &INDIAN_ERAS,

        CalendarSystem::Coptic => &COPTIC_ERAS,

        CalendarSystem::Ethiopic => &ETHIOPIC_ERAS,

        CalendarSystem::EthiopicAmeteAlem => &ETHIOPIC_AMETE_ALEM_ERAS,

        CalendarSystem::Roc => &ROC_ERAS,

        CalendarSystem::Iso8601
        | CalendarSystem::Japanese
        | CalendarSystem::Chinese
        | CalendarSystem::Dangi => &[],
    }
}

pub(crate) fn era_definition(calendar: CalendarSystem, era_code: &str) -> Option<EraDefinition> {
    if calendar == CalendarSystem::Japanese {
        return japanese_era_boundary(era_code).map(|boundary| EraDefinition {
            code: boundary.code,
            display_name: boundary.display_name,
            max_year: japanese_era_max_year(boundary),
        });
    }

    era_definitions(calendar)
        .iter()
        .copied()
        .find(|definition| definition.code == era_code)
}

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
    if calendar == CalendarSystem::Japanese {
        return JAPANESE_ERA_BOUNDARIES.last().map(|boundary| Era {
            code: String::from(boundary.code),
            display_name: String::from(boundary.display_name),
        });
    }

    era_definitions(calendar).last().map(|definition| Era {
        code: String::from(definition.code),
        display_name: String::from(definition.display_name),
    })
}

pub(crate) fn bounded_months_in_year(
    calendar: CalendarSystem,
    year: i32,
    era: Option<&str>,
) -> Option<u8> {
    match calendar {
        CalendarSystem::Japanese => {
            let boundary = japanese_era_boundary(era?)?;

            if let Some(max_year) = japanese_era_max_year(boundary)
                && year == max_year
            {
                boundary.end.map(|(_, month, _)| month)
            } else {
                Some(12)
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
#[cfg(test)]
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
#[cfg(test)]
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

#[cfg(all(test, feature = "icu4x", feature = "std", not(target_arch = "wasm32")))]
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
    test,
    target_arch = "wasm32",
    feature = "web-intl",
    feature = "std",
    feature = "icu4x"
))]
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

#[cfg(all(
    test,
    feature = "icu4x",
    target_arch = "wasm32",
    feature = "std",
    not(feature = "web-intl")
))]
pub(crate) fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    Err(String::from(
        "platform date unavailable on wasm32 without the `web-intl` feature",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CalendarDateFields;

    fn japanese_date(
        era_code: &str,
        display_name: &str,
        year: i32,
        month: u8,
        day: u8,
    ) -> CalendarDate {
        CalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from(era_code),
                    display_name: String::from(display_name),
                }),
                year: Some(year),
                month: Some(month),
                day: Some(day),
                ..CalendarDateFields::default()
            },
        )
        .expect("Japanese fixture should validate")
    }

    #[test]
    fn month_length_helper_covers_gregorian_rules() {
        assert_eq!(gregorian_days_in_month(2024, 2), 29);
        assert_eq!(gregorian_days_in_month(2023, 2), 28);
        assert_eq!(gregorian_days_in_month(2024, 13), 30);
    }

    #[test]
    fn era_helpers_cover_defaults_and_unknown_codes() {
        assert_eq!(
            default_era_for(CalendarSystem::Japanese)
                .expect("Japanese default era")
                .code,
            "reiwa"
        );
        assert_eq!(
            era_definition(CalendarSystem::Japanese, "heisei")
                .expect("Heisei definition")
                .max_year,
            Some(31)
        );
        assert!(era_definition(CalendarSystem::Japanese, "bogus").is_none());
    }

    #[test]
    fn japanese_boundary_helpers_clamp_first_and_last_supported_months() {
        let heisei_start = japanese_date("heisei", "Heisei", 1, 1, 8);

        let heisei_second_year = japanese_date("heisei", "Heisei", 2, 1, 1);

        let reiwa_start = japanese_date("reiwa", "Reiwa", 1, 5, 1);

        assert_eq!(heisei_start.minimum_month_in_year(), 1);
        assert_eq!(heisei_start.minimum_day_in_month(), 8);
        assert_eq!(heisei_second_year.minimum_month_in_year(), 1);
        assert_eq!(heisei_second_year.minimum_day_in_month(), 1);
        assert_eq!(reiwa_start.minimum_month_in_year(), 5);
        assert_eq!(reiwa_start.minimum_day_in_month(), 1);
        assert_eq!(
            bounded_months_in_year(CalendarSystem::Japanese, 31, Some("heisei")),
            Some(4)
        );
        assert_eq!(
            bounded_days_in_month(CalendarSystem::Japanese, 31, 4, Some("heisei")),
            Some(30)
        );
        assert_eq!(
            bounded_months_in_year(CalendarSystem::Japanese, 1, None),
            None
        );
        assert_eq!(
            bounded_days_in_month(CalendarSystem::Japanese, 1, 5, Some("reiwa")),
            Some(31)
        );
    }

    #[test]
    fn date_range_and_epoch_day_helpers_roundtrip_iso_values() {
        let start = CalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian fixture");

        let end = CalendarDate::new_gregorian(2024, 3, 20).expect("Gregorian fixture");

        let earlier = CalendarDate::new_gregorian(2024, 3, 10).expect("Gregorian fixture");

        let range = DateRange::new(start.clone(), end.clone()).expect("ordered range");

        assert!(range.contains(&start));
        assert!(range.contains(&end));
        assert!(!range.contains(&earlier));
        assert_eq!(range.to_iso8601(), "2024-03-15/2024-03-20");
        assert!(DateRange::new(end.clone(), start.clone()).is_none());

        let normalized = DateRange::normalized(end, start).expect("comparable dates");

        assert_eq!(normalized.start.to_iso8601(), "2024-03-15");
        assert_eq!(normalized.end.to_iso8601(), "2024-03-20");

        assert_eq!(epoch_days_to_iso(0), (1, 1, 1));
        assert_eq!(
            epoch_days_to_iso(iso_to_epoch_days(2024, 3, 15)),
            (2024, 3, 15)
        );
        assert_eq!(epoch_days_to_iso(iso_to_epoch_days(44, 3, 15)), (44, 3, 15));
    }
}
