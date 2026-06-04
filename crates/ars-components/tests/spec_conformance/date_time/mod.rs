//! Spec-conformance tests for `crates/ars-components/src/date_time/*`.
//!
//! Each component module asserts the impl's `Part` enum matches the spec's
//! declared anatomy.

use ars_i18n::CalendarDate;

use super::helper::assert_anatomy;

fn calendar_date_default_for_part_all() -> CalendarDate {
    // Mirrors the `#[part(default = ...)]` value on calendar cell parts.
    // `Part::all()` constructs parameterized variants with their declared
    // defaults, and the helper compares them by `PartialEq`.
    CalendarDate::new_gregorian(1, 1, 1).expect("0001-01-01 is a valid Gregorian date")
}

mod calendar;
mod date_field;
mod date_picker;
mod date_range_field;
mod date_range_picker;
mod date_time_picker;
mod range_calendar;
mod time_field;
