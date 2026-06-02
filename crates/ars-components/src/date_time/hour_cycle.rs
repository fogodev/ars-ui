//! Hour-cycle math and numeric-segment helpers shared by the segmented
//! date/time inputs (`time_field`, `date_time_picker`).
//!
//! These functions translate between a [`Time`]'s underlying 24-hour value and
//! the value displayed in an hour segment for a given [`HourCycle`], decide
//! whether a day-period (AM/PM) segment is required, and report the width of a
//! numeric segment's type-ahead buffer. They are deliberately free functions so
//! both the standalone `TimeField` and the composite `DateTimePicker` resolve
//! hour-cycle display identically from a single source of truth.

use ars_i18n::{HourCycle, Time};

/// Returns the inclusive `(min, max)` hour range displayed for `hour_cycle`.
pub(crate) const fn display_hour_range(hour_cycle: HourCycle) -> (i32, i32) {
    match hour_cycle {
        HourCycle::H11 => (0, 11),
        HourCycle::H12 => (1, 12),
        HourCycle::H23 => (0, 23),
        HourCycle::H24 => (1, 24),
    }
}

/// Returns `true` when `hour_cycle` requires a day-period (AM/PM) segment.
pub(crate) const fn has_day_period(hour_cycle: HourCycle) -> bool {
    matches!(hour_cycle, HourCycle::H11 | HourCycle::H12)
}

/// Converts a [`Time`]'s 24-hour value into the value shown in an hour segment.
pub(crate) fn display_hour(time: Time, hour_cycle: HourCycle) -> i32 {
    let hour = time.hour();

    match hour_cycle {
        HourCycle::H11 => i32::from(hour % 12),

        HourCycle::H12 => i32::from(if hour.is_multiple_of(12) {
            12
        } else {
            hour % 12
        }),

        HourCycle::H23 => i32::from(hour),

        HourCycle::H24 => i32::from(if hour == 0 { 24 } else { hour }),
    }
}

/// Converts a displayed hour value (plus optional day-period) back to 24-hour.
///
/// `day_period` is `Some(1)` for PM, `Some(0)` for AM, or `None` when the cycle
/// has no day-period segment. Returns `None` only if the inputs cannot form a
/// valid 24-hour value.
pub(crate) fn display_hour_to_24(
    raw_hour: u8,
    day_period: Option<i32>,
    hour_cycle: HourCycle,
) -> Option<u8> {
    match hour_cycle {
        HourCycle::H11 => {
            let is_pm = day_period.unwrap_or(0) == 1;

            Some(if is_pm { raw_hour + 12 } else { raw_hour })
        }

        HourCycle::H12 => {
            let is_pm = day_period.unwrap_or(0) == 1;

            Some(match (raw_hour, is_pm) {
                (12, false) => 0,
                (12, true) => 12,
                (_, true) => raw_hour + 12,
                (_, false) => raw_hour,
            })
        }

        HourCycle::H23 => Some(raw_hour),

        HourCycle::H24 => Some(if raw_hour == 24 { 0 } else { raw_hour }),
    }
}

/// Returns `true` when the time falls in the afternoon (hour >= 12).
pub(crate) const fn is_pm(time: Time) -> bool {
    time.hour() >= 12
}

/// Returns the number of decimal digits needed to represent `max`.
///
/// Used by numeric segment type-ahead to decide when a buffered entry is "full"
/// and focus should auto-advance to the next segment.
pub(crate) const fn digits_needed(max: u32) -> usize {
    match max {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        _ => 4,
    }
}
