use ars_i18n::{CalendarDate, Time};
use proptest::prelude::*;

pub(super) fn date(year: i32, month: u8, day: u8) -> CalendarDate {
    CalendarDate::new_gregorian(year, month, day).expect("generated date should be valid")
}

pub(super) fn arb_date() -> impl Strategy<Value = CalendarDate> {
    (1900i32..=2100, 1u8..=12, 1u8..=28).prop_map(|(year, month, day)| date(year, month, day))
}

pub(super) fn arb_calendar_date() -> impl Strategy<Value = CalendarDate> {
    arb_date()
}

pub(super) fn arb_time() -> impl Strategy<Value = Time> {
    (0u8..=23, 0u8..=59, 0u8..=59)
        .prop_map(|(hour, minute, second)| Time::new(hour, minute, second, 0).unwrap())
}

pub(super) fn arb_digit() -> impl Strategy<Value = char> {
    (0u8..=9).prop_map(|digit| char::from(b'0' + digit))
}
