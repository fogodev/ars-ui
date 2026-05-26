//! Grid math for the `Calendar` component.
//!
//! Pure functions over a small slice of the parent [`Context`] state: the
//! visible month/year, the configured first-day-of-week, and the visible
//! month count. The machine in [`super`] holds the full context and forwards
//! the relevant fields here so this module stays free of the wider machine
//! plumbing — useful both for unit testing the math in isolation and for
//! keeping the §1.6 spec algorithm legible alongside the rest of the
//! transitions.

use alloc::vec::Vec;

use ars_i18n::{CalendarDate, Weekday};

/// Sunday-zero index for a weekday.
///
/// The spec's §1.6 grid algorithm uses modulo arithmetic that anchors at
/// Sunday=0, but [`Weekday`] is iso-numbered (Monday=1..Sunday=7). The local
/// helper isolates that conversion so every grid call site uses the same
/// convention.
#[must_use]
pub(super) const fn weekday_sunday_zero(weekday: Weekday) -> u8 {
    match weekday {
        Weekday::Sunday => 0,
        Weekday::Monday => 1,
        Weekday::Tuesday => 2,
        Weekday::Wednesday => 3,
        Weekday::Thursday => 4,
        Weekday::Friday => 5,
        Weekday::Saturday => 6,
    }
}

/// Returns `(month, year)` for the month at `offset` from the first visible
/// month, normalising the result to the 1..=12 month range with a year
/// adjustment.
#[must_use]
pub(super) const fn month_year_at_offset(
    visible_month: u8,
    visible_year: i32,
    offset: usize,
) -> (u8, i32) {
    let raw_index = visible_month as i64 - 1 + offset as i64;

    let normalised_month = raw_index.rem_euclid(12) as u8 + 1;
    let year_delta = raw_index.div_euclid(12) as i32;

    (normalised_month, visible_year + year_delta)
}

/// Advances `(month, year)` by `n` months (signed), normalising the month
/// back into the 1..=12 range.
#[must_use]
pub(super) const fn advance_month(visible_month: u8, visible_year: i32, n: i32) -> (u8, i32) {
    let raw_index = visible_month as i64 - 1 + n as i64;

    let normalised_month = raw_index.rem_euclid(12) as u8 + 1;
    let year_delta = raw_index.div_euclid(12) as i32;

    (normalised_month, visible_year + year_delta)
}

/// Builds the 6-week grid for the month at the supplied offset.
///
/// Always returns exactly 6 rows of 7 dates each (42 cells), padded with
/// leading days from the previous month and trailing days from the next.
/// The leading-day count is derived from the difference between the first
/// day of the target month and the configured `first_day_of_week`, modulo 7.
///
/// Date arithmetic at the start/end of the supported calendar range
/// (`new_gregorian` reject) is treated as a no-op: the returned vector is
/// empty in that case, signalling that the adapter should render an empty
/// grid for the boundary month rather than panic.
#[must_use]
pub(super) fn weeks_for(
    visible_month: u8,
    visible_year: i32,
    first_day_of_week: Weekday,
    offset: usize,
) -> Vec<[CalendarDate; 7]> {
    let (target_month, target_year) = month_year_at_offset(visible_month, visible_year, offset);

    let Ok(first_of_month) = CalendarDate::new_gregorian(target_year, target_month, 1) else {
        return Vec::new();
    };

    let first_weekday_index = weekday_sunday_zero(first_of_month.weekday());
    let start_index = weekday_sunday_zero(first_day_of_week);

    let leading = i32::from((first_weekday_index + 7 - start_index) % 7);

    let Ok(grid_start) = first_of_month.add_days(-leading) else {
        return Vec::new();
    };

    let mut weeks: Vec<[CalendarDate; 7]> = Vec::with_capacity(6);

    let mut current = grid_start;

    for _ in 0..6 {
        let Some(row) = build_week(&current) else {
            return weeks;
        };

        weeks.push(row);

        let Ok(next_week_start) = current.add_days(7) else {
            return weeks;
        };

        current = next_week_start;
    }

    weeks
}

fn build_week(start: &CalendarDate) -> Option<[CalendarDate; 7]> {
    let day0 = start.clone();
    let day1 = start.add_days(1).ok()?;
    let day2 = start.add_days(2).ok()?;
    let day3 = start.add_days(3).ok()?;
    let day4 = start.add_days(4).ok()?;
    let day5 = start.add_days(5).ok()?;
    let day6 = start.add_days(6).ok()?;

    Some([day0, day1, day2, day3, day4, day5, day6])
}

/// Whether `date` belongs to a different `(month, year)` than the month at
/// the given visible-month offset.
#[must_use]
pub(super) const fn is_outside_month_at_offset(
    date: &CalendarDate,
    visible_month: u8,
    visible_year: i32,
    offset: usize,
) -> bool {
    let (target_month, target_year) = month_year_at_offset(visible_month, visible_year, offset);

    date.month() != target_month || date.year() != target_year
}

/// Whether `date` falls within any of the `visible_months` months starting
/// at `(visible_month, visible_year)`.
#[must_use]
pub(super) fn is_in_visible_range(
    date: &CalendarDate,
    visible_month: u8,
    visible_year: i32,
    visible_months: usize,
) -> bool {
    (0..visible_months).any(|offset| {
        let (month, year) = month_year_at_offset(visible_month, visible_year, offset);

        date.month() == month && date.year() == year
    })
}

/// Ordered weekday sequence starting at `first_day_of_week`, returned as
/// `(weekday, sunday_zero_index)` pairs so callers can fetch labels from the
/// `IntlBackend` without re-deriving the index.
#[must_use]
pub(super) fn ordered_weekdays(first_day_of_week: Weekday) -> [Weekday; 7] {
    let start = weekday_sunday_zero(first_day_of_week);

    let mut out = [Weekday::Sunday; 7];

    for (i, slot) in out.iter_mut().enumerate() {
        *slot = sunday_zero_to_weekday((start + i as u8) % 7);
    }

    out
}

const fn sunday_zero_to_weekday(index: u8) -> Weekday {
    match index % 7 {
        0 => Weekday::Sunday,
        1 => Weekday::Monday,
        2 => Weekday::Tuesday,
        3 => Weekday::Wednesday,
        4 => Weekday::Thursday,
        5 => Weekday::Friday,
        _ => Weekday::Saturday,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_year_offset_within_same_year() {
        assert_eq!(month_year_at_offset(1, 2024, 0), (1, 2024));
        assert_eq!(month_year_at_offset(1, 2024, 2), (3, 2024));
        assert_eq!(month_year_at_offset(10, 2024, 1), (11, 2024));
    }

    #[test]
    fn month_year_offset_wraps_into_next_year() {
        assert_eq!(month_year_at_offset(11, 2024, 2), (1, 2025));
        assert_eq!(month_year_at_offset(12, 2024, 1), (1, 2025));
    }

    #[test]
    fn advance_month_handles_negative_into_prev_year() {
        assert_eq!(advance_month(1, 2024, -1), (12, 2023));
        assert_eq!(advance_month(3, 2024, -12), (3, 2023));
        assert_eq!(advance_month(6, 2024, 12), (6, 2025));
    }

    #[test]
    fn weeks_for_january_2024_sunday_start_has_leading_days_from_december() {
        let weeks = weeks_for(1, 2024, Weekday::Sunday, 0);

        assert_eq!(weeks.len(), 6);

        // January 1, 2024 is a Monday, so with Sunday-start the first row
        // starts on December 31, 2023.
        let first = &weeks[0][0];

        assert_eq!(first.year(), 2023);
        assert_eq!(first.month(), 12);
        assert_eq!(first.day(), 31);

        // Second cell is January 1, 2024.
        let second = &weeks[0][1];

        assert_eq!(second.year(), 2024);
        assert_eq!(second.month(), 1);
        assert_eq!(second.day(), 1);
    }

    #[test]
    fn weeks_for_january_2024_monday_start_first_cell_is_january_first() {
        let weeks = weeks_for(1, 2024, Weekday::Monday, 0);

        assert_eq!(weeks.len(), 6);

        let first = &weeks[0][0];

        assert_eq!(first.year(), 2024);
        assert_eq!(first.month(), 1);
        assert_eq!(first.day(), 1);
    }

    #[test]
    fn ordered_weekdays_starts_at_configured_day() {
        assert_eq!(
            ordered_weekdays(Weekday::Sunday),
            [
                Weekday::Sunday,
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
                Weekday::Saturday,
            ],
        );
        assert_eq!(
            ordered_weekdays(Weekday::Monday),
            [
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
                Weekday::Saturday,
                Weekday::Sunday,
            ],
        );
    }

    #[test]
    fn is_outside_month_detects_leading_and_trailing_days() {
        let weeks = weeks_for(1, 2024, Weekday::Sunday, 0);

        // First date is December 31, 2023, which is outside January 2024.
        assert!(is_outside_month_at_offset(&weeks[0][0], 1, 2024, 0));
        // January 1 is inside.
        assert!(!is_outside_month_at_offset(&weeks[0][1], 1, 2024, 0));
        // Last row generally contains February dates.
        assert!(is_outside_month_at_offset(&weeks[5][6], 1, 2024, 0));
    }

    #[test]
    fn is_in_visible_range_with_multiple_months() {
        let jan = CalendarDate::new_gregorian(2024, 1, 15).unwrap();
        let feb = CalendarDate::new_gregorian(2024, 2, 15).unwrap();
        let mar = CalendarDate::new_gregorian(2024, 3, 15).unwrap();

        assert!(is_in_visible_range(&jan, 1, 2024, 2));
        assert!(is_in_visible_range(&feb, 1, 2024, 2));
        assert!(!is_in_visible_range(&mar, 1, 2024, 2));
        assert!(is_in_visible_range(&mar, 1, 2024, 3));
    }
}
