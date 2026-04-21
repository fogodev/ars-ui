//! [`StubIntlBackend`] — English-only fallback provider.
//!
//! `StubIntlBackend` is the default implementation available on every
//! target. It explicitly supplies the spec §9.5.1 English/default locale
//! behavior for labels, day periods, digit formatting, hour-cycle fallback,
//! and locale week metadata while reusing [`IntlBackend`](crate::IntlBackend)'s
//! canonical calendar helper defaults.

use alloc::{format, string::String};
use core::num::NonZero;

use crate::{HourCycle, IntlBackend, Locale, WeekInfo, Weekday};

/// English-only stub provider for tests and builds without a backend feature.
///
/// Locale-facing methods implement the final trait contract explicitly.
/// Calendar helpers keep using the trait's canonical shared defaults.
#[derive(Debug, Default)]
pub struct StubIntlBackend;

impl IntlBackend for StubIntlBackend {
    fn weekday_short_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday => String::from("Su"),
            Weekday::Monday => String::from("Mo"),
            Weekday::Tuesday => String::from("Tu"),
            Weekday::Wednesday => String::from("We"),
            Weekday::Thursday => String::from("Th"),
            Weekday::Friday => String::from("Fr"),
            Weekday::Saturday => String::from("Sa"),
        }
    }

    fn weekday_long_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday => String::from("Sunday"),
            Weekday::Monday => String::from("Monday"),
            Weekday::Tuesday => String::from("Tuesday"),
            Weekday::Wednesday => String::from("Wednesday"),
            Weekday::Thursday => String::from("Thursday"),
            Weekday::Friday => String::from("Friday"),
            Weekday::Saturday => String::from("Saturday"),
        }
    }

    fn month_long_name(&self, month: u8, _locale: &Locale) -> String {
        match month {
            1 => String::from("January"),
            2 => String::from("February"),
            3 => String::from("March"),
            4 => String::from("April"),
            5 => String::from("May"),
            6 => String::from("June"),
            7 => String::from("July"),
            8 => String::from("August"),
            9 => String::from("September"),
            10 => String::from("October"),
            11 => String::from("November"),
            12 => String::from("December"),
            _ => String::from("Unknown"),
        }
    }

    fn day_period_label(&self, is_pm: bool, _locale: &Locale) -> String {
        if is_pm {
            String::from("PM")
        } else {
            String::from("AM")
        }
    }

    fn day_period_from_char(&self, ch: char, _locale: &Locale) -> Option<bool> {
        match ch.to_ascii_lowercase() {
            'a' => Some(false),
            'p' => Some(true),
            _ => None,
        }
    }

    fn format_segment_digits(
        &self,
        value: u32,
        min_digits: NonZero<u8>,
        _locale: &Locale,
    ) -> String {
        format!("{value:0width$}", width = usize::from(min_digits.get()))
    }

    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        match locale.language() {
            "en" | "ko" => HourCycle::H12,
            _ => HourCycle::H23,
        }
    }

    fn week_info(&self, locale: &Locale) -> WeekInfo {
        WeekInfo::for_locale(locale)
    }
}
