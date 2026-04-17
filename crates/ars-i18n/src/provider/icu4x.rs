//! [`Icu4xProvider`] — production provider backed by ICU4X CLDR data.
//!
//! Implements the [`IcuProvider`](crate::IcuProvider) contract using ICU4X 2.x
//! compiled data. See spec §9.5.2.

use alloc::string::{String, ToString};
use core::num::NonZero;

use fixed_decimal::Decimal;
use icu::{
    calendar::{
        Date,
        week::{WeekInformation, WeekPreferences},
    },
    datetime::{
        DateTimeFormatter, DateTimeFormatterPreferences, NoCalendarFormatter,
        fieldsets::{E, M, T},
    },
    decimal::{
        DecimalFormatter, DecimalFormatterPreferences,
        options::{DecimalFormatterOptions, GroupingStrategy},
    },
    locale::preferences::extensions::unicode::keywords::HourCycle as IcuHourCycle,
    time::Time,
};

use crate::{
    CalendarDate, CalendarSystem, Era, HourCycle, IcuProvider, Locale, Weekday,
    calendar::{
        bounded_days_in_month, bounded_months_in_year, default_era_for, minimum_day_in_month,
        minimum_month_in_year, years_in_era,
    },
};

/// Production ICU4X-backed provider with full CLDR data.
///
/// Uses compiled CLDR data (via the `icu/compiled_data` feature) to resolve
/// weekday and month names, day-period labels, locale-aware digit
/// formatting, hour cycle, first-day-of-week, and cross-calendar date
/// conversion.
///
/// The struct is zero-sized: ICU4X's compiled-data path does not require
/// runtime data to be carried in the formatter instance.
#[derive(Clone, Copy, Debug, Default)]
pub struct Icu4xProvider;

impl Icu4xProvider {
    /// Maps [`Weekday`] to a January 2024 day-of-month for format-and-extract.
    ///
    /// January 1, 2024 is a Monday and January 7, 2024 is a Sunday, so a
    /// reference date in that range uniquely identifies a weekday without
    /// requiring a separate weekday lookup table.
    const fn weekday_to_jan2024_day(weekday: Weekday) -> u8 {
        match weekday {
            Weekday::Monday => 1,
            Weekday::Tuesday => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday => 4,
            Weekday::Friday => 5,
            Weekday::Saturday => 6,
            Weekday::Sunday => 7,
        }
    }
}

impl IcuProvider for Icu4xProvider {
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String {
        let formatter = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(locale.as_icu()),
            E::short(),
        )
        .expect("compiled_data guarantees weekday formatter availability");

        let date = Date::try_new_iso(2024, 1, Self::weekday_to_jan2024_day(weekday))
            .expect("2024-01-01..07 are valid ISO dates");

        formatter.format(&date).to_string()
    }

    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String {
        let formatter = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(locale.as_icu()),
            E::long(),
        )
        .expect("compiled_data guarantees weekday formatter availability");

        let date = Date::try_new_iso(2024, 1, Self::weekday_to_jan2024_day(weekday))
            .expect("2024-01-01..07 are valid ISO dates");

        formatter.format(&date).to_string()
    }

    fn month_long_name(&self, month: u8, locale: &Locale) -> String {
        if !(1..=12).contains(&month) {
            return String::from("Unknown");
        }

        let formatter = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(locale.as_icu()),
            M::long(),
        )
        .expect("compiled_data guarantees month formatter availability");

        let date = Date::try_new_iso(2024, month, 1)
            .expect("month 1-12, day 1 of 2024 is always a valid ISO date");

        formatter.format(&date).to_string()
    }

    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
        // Force a 12-hour cycle so the formatter always emits a
        // day-period token, even for locales whose CLDR default is
        // 24-hour (e.g., `de-DE`, `fr-FR`, `ja-JP`). Without this
        // override 24-hour locales collapse to numbers only.
        let mut prefs = DateTimeFormatterPreferences::from(locale.as_icu());
        prefs.hour_cycle = Some(IcuHourCycle::H12);

        let formatter = NoCalendarFormatter::try_new(prefs, T::hm())
            .expect("compiled_data guarantees time formatter availability");

        let am_time = Time::try_new(1, 0, 0, 0).expect("01:00 is a valid time");
        let pm_time = Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time");
        let am_formatted = formatter.format(&am_time).to_string();
        let pm_formatted = formatter.format(&pm_time).to_string();

        // Compute the AM- and PM-unique spans by peeling off the
        // longest common prefix and suffix between the two formatted
        // outputs. Whatever remains is the day-period marker by
        // definition — decoration characters that appear in both
        // strings (hour digits, colons, locale hour literals like
        // `bg-BG`'s `ч.`, the Japanese `:` separator) are common and
        // get stripped automatically. The approach was suggested by
        // the Codex round-6 review and handles locales where the
        // previous digit/separator filter left hour-literal fragments
        // in the label (e.g., bg-BG surfacing `ч` as the first char
        // of both AM and PM labels).
        let (am_unique, pm_unique) = unique_span_diff(&am_formatted, &pm_formatted);
        let unique = if is_pm { pm_unique } else { am_unique };
        unique.trim().to_string()
    }

    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
        let am_label = self.day_period_label(false, locale);

        let pm_label = self.day_period_label(true, locale);

        let am_char = am_label.chars().next()?;

        let pm_char = pm_label.chars().next()?;

        let ch_lower = ch
            .to_lowercase()
            .next()
            .expect("to_lowercase always yields at least one char");

        let am_lower = am_char
            .to_lowercase()
            .next()
            .expect("to_lowercase always yields at least one char");

        let pm_lower = pm_char
            .to_lowercase()
            .next()
            .expect("to_lowercase always yields at least one char");

        if ch_lower == am_lower {
            Some(false)
        } else if ch_lower == pm_lower {
            Some(true)
        } else {
            None
        }
    }

    fn format_segment_digits(
        &self,
        value: u32,
        min_digits: NonZero<u8>,
        locale: &Locale,
    ) -> String {
        // Disable locale grouping so segment values never pick up
        // thousand separators. A year like 2024 must format as
        // `"2024"` (or its native-digit equivalent), not `"2,024"` —
        // otherwise downstream parsers that expect a contiguous digit
        // run break, and behavior diverges from `WebIntlProvider`,
        // which already sets `useGrouping: false`.
        let mut options = DecimalFormatterOptions::default();
        options.grouping_strategy = Some(GroupingStrategy::Never);

        let formatter =
            DecimalFormatter::try_new(DecimalFormatterPreferences::from(locale.as_icu()), options)
                .expect("compiled_data guarantees decimal formatter availability");

        let mut decimal = Decimal::from(i64::from(value));

        // `pad_start(n)` grows the integer part so it contains at least
        // `n` digits, filling the leading positions with zeros. Passing
        // the requested minimum digit count directly gives the expected
        // zero-padded output (e.g., `5` with `min_digits = 2` → `"05"`
        // / `"٠٥"`).
        decimal.absolute.pad_start(i16::from(min_digits.get()));

        formatter.format(&decimal).to_string()
    }

    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8 {
        if let Some(months) = bounded_months_in_year(*calendar, year, era) {
            return months;
        }

        // Route through the workspace's ICU4X field-based constructor
        // (`crate::calendar::internal`) rather than the spec sketch that
        // still names the deprecated `Date::try_new_from_codes`/
        // `MonthCode::new_normal` pair. The internal helper is already
        // CLDR-backed and handles Hebrew leap years and Japanese
        // end-of-era clamping that we would otherwise have to replicate.
        crate::calendar::internal::months_in_year(year, *calendar, era).unwrap_or(12)
    }

    fn days_in_month(
        &self,
        calendar: &CalendarSystem,
        year: i32,
        month: u8,
        era: Option<&str>,
    ) -> u8 {
        if let Some(days) = bounded_days_in_month(*calendar, year, month, era) {
            return days;
        }

        crate::calendar::internal::days_in_month(year, month, *calendar, era).unwrap_or(30)
    }

    fn default_era(&self, calendar: &CalendarSystem) -> Option<Era> {
        default_era_for(*calendar)
    }

    fn years_in_era(&self, date: &CalendarDate) -> Option<i32> {
        years_in_era(date)
    }

    fn minimum_month_in_year(&self, date: &CalendarDate) -> u8 {
        minimum_month_in_year(date)
    }

    fn minimum_day_in_month(&self, date: &CalendarDate) -> u8 {
        minimum_day_in_month(date)
    }

    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        let formatter = NoCalendarFormatter::try_new(
            DateTimeFormatterPreferences::from(locale.as_icu()),
            T::hm(),
        )
        .expect("compiled_data guarantees time formatter availability");

        let am_time = Time::try_new(1, 0, 0, 0).expect("01:00 is a valid time");
        let pm_time = Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time");

        let am_formatted = formatter.format(&am_time).to_string();
        let pm_formatted = formatter.format(&pm_time).to_string();

        // Extract the first run of Unicode numerals from each
        // formatted output and compare. A 12-hour locale renders both
        // `01:00` and `13:00` with the same hour digit (`1`), so the
        // runs match; a 24-hour locale renders them with different
        // hour digits (`01` vs `13`, or the locale's native-digit
        // equivalents) and the runs differ. This sidesteps the
        // decoration-character trap in locales like `bg-BG`
        // (`"13:00 ч."`) or `mr-IN-u-hc-h23` (Devanagari `"१३-००"`)
        // where stripping decoration cannot reliably distinguish
        // day-period markers from hour-literal suffixes.
        if first_numeric_run(&am_formatted) == first_numeric_run(&pm_formatted) {
            HourCycle::H12
        } else {
            HourCycle::H23
        }
    }

    fn first_day_of_week(&self, locale: &Locale) -> Weekday {
        if let Some(weekday) = locale.first_day_of_week_extension() {
            return weekday;
        }

        let week_info = WeekInformation::try_new(WeekPreferences::from(locale.as_icu()))
            .expect("compiled_data guarantees week information data for any locale");

        Weekday::from_icu_weekday(week_info.first_weekday)
    }

    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        if date.calendar == target {
            return date.clone();
        }

        let Ok(internal) = crate::calendar::internal::CalendarDate::try_from(date) else {
            return date.clone();
        };

        let converted = internal.to_calendar(target);

        CalendarDate {
            calendar: target,
            era: converted
                .era()
                .filter(|_| target.has_custom_eras())
                .map(|code| Era {
                    code: code.clone(),
                    display_name: code,
                }),
            year: converted.year(),
            month: NonZero::new(converted.month())
                .expect("internal calendar conversion yields a 1-based month"),
            day: NonZero::new(converted.day())
                .expect("internal calendar conversion yields a 1-based day"),
        }
    }
}

/// Returns the first contiguous run of Unicode numerals from `s`, or
/// an empty slice when the string contains none.
///
/// Uses [`char::is_numeric`] so every Unicode decimal digit (`Nd`)
/// counts — ASCII, Arabic-Indic (٠-٩), Persian (۰-۹), Bengali (০-৯),
/// Devanagari (०-९), Myanmar (၀-၉), and so on. Hour-cycle detection
/// compares the runs from the 01:00 and 13:00 probes: when they match
/// the locale uses 12-hour formatting (both hours render as `1`);
/// when they differ the locale uses 24-hour formatting.
pub(crate) fn first_numeric_run(s: &str) -> &str {
    let Some(start) = s
        .char_indices()
        .find_map(|(i, c)| c.is_numeric().then_some(i))
    else {
        return "";
    };
    let end = s[start..]
        .char_indices()
        .find_map(|(i, c)| (!c.is_numeric()).then_some(start + i))
        .unwrap_or(s.len());
    &s[start..end]
}

/// Returns the AM-only and PM-only substrings produced by stripping
/// the longest common prefix and suffix from `am_formatted` /
/// `pm_formatted`. Decoration text that appears in both strings (hour
/// digits, separators, locale hour literals like `bg-BG`'s `ч.`) is
/// shared and collapses into the prefix/suffix; what remains are the
/// two day-period markers by construction.
///
/// The slices are returned in the order `(am_unique, pm_unique)` and
/// are trimmed by the caller before use.
pub(crate) fn unique_span_diff<'a>(
    am_formatted: &'a str,
    pm_formatted: &'a str,
) -> (&'a str, &'a str) {
    let mut prefix_len = 0_usize;
    for (ach, pch) in am_formatted.chars().zip(pm_formatted.chars()) {
        if ach != pch {
            break;
        }
        prefix_len += ach.len_utf8();
    }

    let am_rest = &am_formatted[prefix_len..];
    let pm_rest = &pm_formatted[prefix_len..];

    let mut suffix_len = 0_usize;
    for (ach, pch) in am_rest.chars().rev().zip(pm_rest.chars().rev()) {
        if ach != pch {
            break;
        }
        suffix_len += ach.len_utf8();
    }

    let am_end = am_formatted.len().saturating_sub(suffix_len);
    let pm_end = pm_formatted.len().saturating_sub(suffix_len);
    let am_end = am_end.max(prefix_len);
    let pm_end = pm_end.max(prefix_len);
    (
        &am_formatted[prefix_len..am_end],
        &pm_formatted[prefix_len..pm_end],
    )
}
