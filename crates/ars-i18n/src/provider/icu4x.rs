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
        // override the stripped output is empty and
        // `day_period_from_char` returns `None` for every input.
        let mut prefs = DateTimeFormatterPreferences::from(locale.as_icu());
        prefs.hour_cycle = Some(IcuHourCycle::H12);

        let formatter = NoCalendarFormatter::try_new(prefs, T::hm())
            .expect("compiled_data guarantees time formatter availability");

        let test_time = if is_pm {
            Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time")
        } else {
            Time::try_new(1, 0, 0, 0).expect("01:00 is a valid time")
        };

        // Strip numerals and separators to isolate the day-period text.
        //
        // Limitation: ICU4X 2.x does not expose a direct day-period names
        // API, so we reconstruct the label from a formatted reference time.
        // We strip Unicode numerics (ASCII, Arabic-Indic ٠-٩, Persian ۰-۹,
        // Bengali ০-৯, …) so AM/PM lookup stays correct for locales that
        // render time in native digits — otherwise ar-EG would surface
        // `١٠٠ ص` and `day_period_from_char` would see `١` as the first
        // character of both AM and PM labels.
        formatter
            .format(&test_time)
            .to_string()
            .chars()
            .filter(|c| !is_numeral_or_time_separator(*c))
            .collect::<String>()
            .trim()
            .to_string()
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

        let test_time = Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time");

        let formatted = formatter.format(&test_time).to_string();

        // A 24-hour locale formats 13:00 as "13:00" — or the locale's
        // native-digit equivalent (`۱۳:۰۰` in fa-IR, `١٣:٠٠` in ar-EG) —
        // with no day-period text. Any character that is not a Unicode
        // numeral or a standard time separator signals a 12-hour format
        // (`1 PM`, `午後1:00`, `١ م`, …). Using `char::is_numeric` keeps
        // non-ASCII numerals from being flagged as day-period markers.
        let has_day_period = formatted
            .chars()
            .any(|c| !is_numeral_or_time_separator(c) && !c.is_whitespace());
        if has_day_period {
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

/// Returns `true` when `c` is a Unicode numeral or a standard time-pattern
/// separator (ASCII colon, U+002E period, U+066B Arabic decimal separator,
/// and the locale-neutral punctuation that CLDR routinely uses inside
/// time patterns).
///
/// The filter covers every Unicode decimal digit (`Nd`) via
/// [`char::is_numeric`], so native-digit locales such as `ar-EG`
/// (`٠-٩`), `fa-IR` (`۰-۹`), `bn-BD` (`০-৯`), and `my-MM` (`၀-၉`) are
/// handled uniformly.
fn is_numeral_or_time_separator(c: char) -> bool {
    c.is_numeric() || matches!(c, ':' | '.' | '\u{066B}' | '\u{066C}')
}
