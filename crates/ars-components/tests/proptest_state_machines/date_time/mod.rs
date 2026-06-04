//! Ignored nightly property-based tests for `crates/ars-components/src/date_time/*`.
//!
//! Each stateful or complex date-time component owns a sibling module holding
//! its arbitraries and `proptest!` block.

mod helpers;

mod calendar;
mod date_field;
mod date_picker;
mod date_range_field;
mod date_range_picker;
mod date_time_picker;
mod range_calendar;
mod time_field;
