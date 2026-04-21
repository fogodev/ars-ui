# Date & Time Components

Cross-references: `00-overview.md` for naming conventions and data attributes, `01-architecture.md` for the `Machine` trait, `AttrMap`, `Bindable`, and crate structure, `03-accessibility.md` for ARIA types and screen reader matrix, `05-interactions.md` for keyboard and pointer handling.

## Table of Contents

- [DateField](date-field.md)
- [TimeField](time-field.md)
- [Calendar](calendar.md)
- [DatePicker](date-picker.md)
- [DateRangePicker](date-range-picker.md)
- [DateRangeField](date-range-field.md)
- [DateTimePicker](date-time-picker.md)

---

## Overview

This document specifies six closely related components that together form ars-ui's date and time input system:

| Component         | Purpose                                                                      |
| ----------------- | ---------------------------------------------------------------------------- |
| `DateField`       | Segmented date input; each field (month, day, year) is individually editable |
| `TimeField`       | Segmented time input; segments for hour, minute, second, AM/PM               |
| `Calendar`        | Grid-based calendar for date browsing and selection                          |
| `DatePicker`      | `DateField` + `Calendar` in a popover                                        |
| `DateRangePicker` | Two `DateField`s + range-mode `Calendar` in a popover                        |
| `DateRangeField`  | Inline two-field range input without a popover                               |

All six components live in `ars-core` as state machine definitions (zero framework dependencies) and are exposed via thin adapter wrappers in `ars-leptos` and `ars-dioxus`.

### Shared Types (`ars-i18n`)

All date/time components operate on the following shared types defined in `ars-i18n`:

```rust
// ars-i18n/src/calendar/types.rs
use core::num::NonZero;

/// Shorthand for `NonZero<u8>` from a compile-time-known literal.
const fn nzu8(n: u8) -> NonZero<u8> {
    match NonZero::new(n) {
        Some(v) => v,
        None => panic!("value must be nonzero"),
    }
}

/// A date in an arbitrary calendar system.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    /// The calendar system of the date.
    pub calendar: CalendarSystem,
    /// The era of the date.
    pub era: Option<Era>,
    /// The year of the date.
    pub year: i32,
    /// The month of the date.
    pub month: NonZero<u8>,  // 1-based
    /// The day of the date.
    pub day: NonZero<u8>,    // 1-based
}

impl CalendarDate {
    /// Creates a new Gregorian date.
    pub fn new_gregorian(year: i32, month: NonZero<u8>, day: NonZero<u8>) -> Self {
        Self {
            calendar: CalendarSystem::Gregorian,
            era: None,
            year,
            month,
            day,
        }
    }

    /// Validated constructor. Returns `None` if month or day is out of range
    /// for the given calendar system.
    ///
    /// Non-Gregorian calendars have different month counts and day-per-month
    /// rules. For example:
    /// - Hebrew: 12 or 13 months (Adar I/II in leap years)
    /// - Ethiopic: 13 months (Pagume has 5-6 days)
    /// - Chinese/Dangi: 12 or 13 months (intercalary month in some years)
    /// - Islamic: Months alternate 29/30 days
    /// - Persian: First 6 months have 31 days, next 5 have 30, last has 29-30
    pub fn new(backend: &dyn IntlBackend, calendar: CalendarSystem, year: i32, month: u8, day: u8) -> Option<Self> {
        let max_month = max_months_in_year(backend, calendar, year);
        if !(1..=max_month).contains(&month) {
            return None;
        }
        let max_day = days_in_month_for_calendar(backend, calendar, year, month);
        if !(1..=max_day).contains(&day) {
            return None;
        }
        Some(Self {
            calendar, era: None, year,
            month: NonZero::new(month).expect("validated 1-based"),
            day: NonZero::new(day).expect("validated 1-based"),
        })
    }

    /// Checks if the date is between two other dates.
    pub fn is_between(&self, start: &CalendarDate, end: &CalendarDate) -> bool {
        self >= start && self <= end
    }

    /// Returns the number of days in the month of the date.
    pub fn days_in_month(&self, backend: &dyn IntlBackend) -> u8 {
        days_in_month_for_calendar(backend, self.calendar, self.year, self.month.get())
    }

    /// Adds a number of months to the date.
    pub fn add_months(&self, backend: &dyn IntlBackend, n: i32) -> CalendarDate {
        let total_months = (self.year * 12 + (self.month.get() as i32 - 1)) + n;
        let new_year  = total_months.div_euclid(12);
        let new_month = (total_months.rem_euclid(12) + 1) as u8;
        // Use calendar-aware days-in-month via IntlBackend.
        let max_day = days_in_month_for_calendar(backend, self.calendar, new_year, new_month);
        let clamped_day = self.day.get().min(max_day);
        CalendarDate {
            year: new_year,
            month: NonZero::new(new_month).expect("month result is 1-based"),
            day: NonZero::new(clamped_day).expect("day clamped to valid range"),
            calendar: self.calendar,
        }
    }

    /// Adds a number of days to the date.
    pub fn add_days(&self, n: i32) -> CalendarDate {
        let jdn = self.to_jdn() + n as i64;
        Self::from_jdn(jdn, self.calendar)
    }

    /// Convert a Gregorian date to Julian Day Number.
    fn to_jdn(&self) -> i64 {
        let y = self.year as i64;
        let m = self.month.get() as i64;
        let d = self.day.get() as i64;
        // Standard Gregorian → JDN algorithm
        let a = (14 - m) / 12;
        let y2 = y + 4800 - a;
        let m2 = m + 12 * a - 3;
        d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045
    }

    /// Convert a Julian Day Number back to a Gregorian CalendarDate.
    fn from_jdn(jdn: i64, calendar: CalendarSystem) -> CalendarDate {
        // Inverse of the Gregorian JDN algorithm
        let a = jdn + 32044;
        let b = (4 * a + 3) / 146097;
        let c = a - (146097 * b) / 4;
        let d = (4 * c + 3) / 1461;
        let e = c - (1461 * d) / 4;
        let m = (5 * e + 2) / 153;
        let day = (e - (153 * m + 2) / 5 + 1) as u8;
        let month = (m + 3 - 12 * (m / 10)) as u8;
        let year = (100 * b + d - 4800 + m / 10) as i32;
        CalendarDate {
            calendar, era: None, year,
            month: NonZero::new(month).expect("JDN yields 1-based month"),
            day: NonZero::new(day).expect("JDN yields 1-based day"),
        }
    }

    /// Converts this date to a different calendar system.
    /// e.g., Gregorian -> Japanese, Islamic -> Gregorian
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<CalendarDate, CalendarConversionError> {
        if self.calendar == target {
            return Ok(self.clone());
        }
        // Convert through Julian Day Number as intermediate representation
        let jdn = self.to_julian_day_number()?;
        CalendarDate::from_julian_day_number(jdn, target)
    }

    /// Returns the weekday of the date.
    pub fn weekday(&self) -> Weekday {
        // Tomohiko Sakamoto's algorithm for Gregorian.
        let y = if self.month.get() < 3 { self.year - 1 } else { self.year };
        let m = self.month.get() as i32;
        let d = self.day.get() as i32;
        static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let w = (y + y/4 - y/100 + y/400 + T[(m-1) as usize] + d) % 7;
        Weekday::from_u8(((w + 7) % 7) as u8)
    }
}

/// Validated month value (1..=12). Library code uses this to avoid panics on invalid input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(u8);

impl Month {
    /// Creates a new month.
    pub fn new(m: u8) -> Option<Self> {
        (1..=12).contains(&m).then(|| Self(m))
    }

    /// Returns the month value.
    pub fn get(&self) -> u8 { self.0 }
}

impl TryFrom<u8> for Month {
    type Error = ();
    fn try_from(m: u8) -> Result<Self, ()> {
        Self::new(m).ok_or(())
    }
}

fn gregorian_days_in_month(year: i32, month: Month) -> u8 {
    match month.get() {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            if leap { 29 } else { 28 }
        }
        _ => unreachable!("Month type ensures valid month value"),
    }
}

/// Maximum number of months in a year for the given calendar system.
///
/// Delegates to `IntlBackend::max_months_in_year()` for calendar-aware computation.
/// Production (`Icu4xBackend`): uses ICU4X `AnyCalendar::months_in_year()` for
/// precise results across all calendar systems (Hebrew 13-month leap years,
/// Chinese/Dangi intercalary months, Ethiopic/Coptic 13th month).
/// Tests (`StubIntlBackend`): returns hardcoded values matching the original
/// English-only logic (see `04-internationalization.md` §9.5).
fn max_months_in_year(backend: &dyn IntlBackend, calendar: CalendarSystem, year: i32) -> u8 {
    backend.max_months_in_year(&calendar, year)
}

/// Days in a given month for the given calendar system.
///
/// Delegates to `IntlBackend::days_in_month()` for calendar-aware computation.
/// Production (`Icu4xBackend`): uses ICU4X `AnyCalendar::days_in_month()`
/// for precise results including observation-based Islamic calendars, Hebrew
/// deficient/regular/complete year types, and Chinese/Dangi lunation lengths.
/// Tests (`StubIntlBackend`): falls back to Gregorian logic for all calendars
/// (see `04-internationalization.md` §9.5).
fn days_in_month_for_calendar(backend: &dyn IntlBackend, calendar: CalendarSystem, year: i32, month: u8) -> u8 {
    backend.days_in_month(&calendar, year, month)
}

/// Inclusive date range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    /// The start date of the range.
    pub start: CalendarDate,
    /// The end date of the range.
    pub end: CalendarDate,
}

impl DateRange {
    /// Returns `None` if `start > end`. Prefer `normalized()` for auto-ordering.
    pub fn new(start: CalendarDate, end: CalendarDate) -> Option<Self> {
        if start <= end { Some(Self { start, end }) } else { None }
    }

    /// Checks if the date is within the range.
    pub fn contains(&self, date: &CalendarDate) -> bool {
        date.is_between(&self.start, &self.end)
    }

    /// Primary constructor — always produces a valid range by ordering inputs.
    pub fn normalized(a: CalendarDate, b: CalendarDate) -> Self {
        if a <= b { Self { start: a, end: b } } else { Self { start: b, end: a } }
    }
}

// `CalendarSystem` — defined in `shared/date-time-types.md`
```

#### Calendar System Validation

When validating dates against a specific calendar system, `CalendarDate` MUST enforce per-calendar month and day bounds:

**Per-Calendar Validation Rules**:

| Calendar  | Month Range  | Day Validation              | Special Rules                                            |
| --------- | ------------ | --------------------------- | -------------------------------------------------------- |
| Gregorian | 1–12         | Standard (28-31 per month)  | Feb: 28 (29 in leap year)                                |
| Islamic   | 1–12         | Months alternate 30/29 days | Month 12: 29 (30 in leap year, 11 of 30-year cycle)      |
| Hebrew    | 1–12 or 1–13 | Varies by month (29-30)     | Month 13 (Adar II) ONLY in leap years (19-year cycle)    |
| Japanese  | 1–12         | Same as Gregorian           | Era MUST be valid (Reiwa ≥ 2019, Heisei 1989-2019, etc.) |
| Buddhist  | 1–12         | Same as Gregorian           | Year = Gregorian + 543; validation same as Gregorian     |
| ROC       | 1–12         | Same as Gregorian           | Year = Gregorian - 1911; year 0 is invalid               |

**Leap Month Handling**:

- **Hebrew**: In a leap year (years 3, 6, 8, 11, 14, 17, 19 of the 19-year Metonic cycle), a 13th month (Adar II / אדר ב׳) is inserted. Setting `month = 13` in a non-leap Hebrew year MUST return `Err(CalendarValidationError::InvalidLeapMonth)`.
- **Chinese (if supported)**: Intercalary (leap) months can occur after any month. The leap month is identified by a boolean flag, not a month number. Validation requires consulting the Chinese calendar data for the specific year.

**Era Validation** (Japanese Calendar):

- Each era has a start date. Year 1 of an era corresponds to the accession year.
- Valid eras: Meiji (1868), Taishō (1912), Shōwa (1926), Heisei (1989), Reiwa (2019).
- Setting `era = Heisei, year = 35` is invalid because Heisei ended at year 31 (2019).

**Clamping Behavior**: When switching calendar systems (e.g., Gregorian → Hebrew), if the current date's month/day exceeds the target calendar's bounds, the date MUST be clamped to the nearest valid date:

- If month exceeds max: clamp to last month of year
- If day exceeds max for month: clamp to last day of month
- Emit a `DateClamped` notification to inform the user of the adjustment

```rust
// ── Calendar System Edge Cases ──────────────────────────────────────────────
//
// Each non-Gregorian calendar system has unique edge cases for validation.
// Adapters and the `CalendarDate::validated()` constructor MUST enforce these
// rules. Localizable error messages come from `CalendarMessages`.
//
// ### Hebrew Calendar (`hebrew` feature)
//   - Months: 1–12 in common years, 1–13 in leap years (month 13 = Adar II).
//   - Leap years follow a 19-year Metonic cycle: years 3, 6, 8, 11, 14, 17, 19.
//   - `CalendarDate::validated()` MUST reject month 13 when the year is not a
//     leap year. Error: `CalendarMessages::invalid_month_for_year`.
//   - Month lengths vary: Cheshvan (29 or 30), Kislev (29 or 30) depend on year type.
//
// ### Japanese Calendar (`japanese` feature)
//   - Eras: Meiji (1868), Taisho (1912), Showa (1926), Heisei (1989), Reiwa (2019).
//   - Era transitions: year 1 of new era starts on transition date (not Jan 1).
//     Example: Reiwa 1 starts May 1, 2019 — dates before that in 2019 are Heisei 31.
//   - `CalendarDate::validated()` MUST reject dates that fall before the era's
//     start date. Error: `CalendarMessages::date_before_era_start`.
//   - Future eras: use `#[non_exhaustive]` on `Era`; the ICU4X provider supplies
//     new eras via data updates without code changes.
//
// ### Persian (Solar Hijri) Calendar (`persian` feature)
//   - Months 1–6 have 31 days, months 7–11 have 30 days, month 12 has 29 or 30.
//   - Leap years: month 12 has 30 days. Leap determination follows the 2820-year
//     cycle algorithm (or delegated to ICU4X `icu::calendar::Persian`).
//   - `CalendarDate::validated()` MUST reject day 30 of month 12 in non-leap years.
//     Error: `CalendarMessages::invalid_day_for_month`.
//
// ### Buddhist Calendar (`buddhist` feature)
//   - Structurally identical to Gregorian but year = Gregorian year + 543.
//   - Thailand uses Buddhist Era (BE); the segment order for `th` locale is D/M/Y
//     with the year displayed in BE. `segments_for_locale` handles this via ICU4X.
//   - No special month/day validation beyond Gregorian rules.
//
// ### Islamic Calendars (`islamic` feature)
//   - All variants have 12 months. Odd months have 30 days, even months have 29 days,
//     except month 12 which has 30 days in leap years.
//   - Leap year rules differ by variant (tabular: 2,5,7,10,13,16,18,21,24,26,29
//     in a 30-year cycle; observational: determined by moon sighting).
//   - `CalendarDate::validated()` MUST use the variant-specific leap rule.
//
// ### Min/Max Validation Across Calendars
//   - `min_value` and `max_value` props on DateField/Calendar/DatePicker are compared
//     after converting both the constraint and the candidate to a common epoch-day
//     representation (Julian Day Number). This ensures correct cross-calendar
//     comparison (e.g., min in Gregorian, value entered in Hebrew).
//   - Validation error: `CalendarMessages::date_out_of_range { min, max }` with
//     both dates formatted in the active calendar system for the error message.
```

#### Comprehensive Calendar Validation Reference

The following reference expands on the validation table above with per-calendar day ranges, leap year rules, era/epoch handling, and validation error message codes.

**Per-Calendar Day Range by Month:**

| Calendar  | Month 1 | Month 2 | Month 3 | Month 4 | Month 5 | Month 6 | Month 7 | Month 8 | Month 9 | Month 10 | Month 11 | Month 12 | Month 13  |
| --------- | ------- | ------- | ------- | ------- | ------- | ------- | ------- | ------- | ------- | -------- | -------- | -------- | --------- |
| Gregorian | 31      | 28/29   | 31      | 30      | 31      | 30      | 31      | 31      | 30      | 31       | 30       | 31       | —         |
| Islamic   | 30      | 29      | 30      | 29      | 30      | 29      | 30      | 29      | 30      | 29       | 30       | 29/30    | —         |
| Hebrew    | 30      | 29/30   | 29/30   | 29      | 30      | 29      | 30      | 29      | 30      | 29       | 30       | 29       | 29 (leap) |
| Persian   | 31      | 31      | 31      | 31      | 31      | 31      | 30      | 30      | 30      | 30       | 30       | 29/30    | —         |
| Coptic    | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30       | 30       | 30       | 5/6       |
| Ethiopic  | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30      | 30       | 30       | 30       | 5/6       |

**Leap Year Rules:**

| Calendar              | Rule                                                 | Cycle     |
| --------------------- | ---------------------------------------------------- | --------- |
| Gregorian             | Divisible by 4, not by 100, unless by 400            | 400-year  |
| Islamic (tabular)     | Years 2,5,7,10,13,16,18,21,24,26,29 in 30-year cycle | 30-year   |
| Hebrew                | Years 3,6,8,11,14,17,19 in 19-year Metonic cycle     | 19-year   |
| Persian               | 2820-year cycle algorithm (delegated to ICU4X)       | 2820-year |
| Buddhist/ROC/Japanese | Same as Gregorian (structurally identical)           | 400-year  |
| Coptic/Ethiopic       | Every 4th year (no century exception)                | 4-year    |

**Era/Epoch Handling:**

| Calendar  | Epoch / Era                                                            | Notes                                                                                     |
| --------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Gregorian | CE / BCE                                                               | Year 0 does not exist in proleptic Gregorian (use astronomical year numbering internally) |
| Japanese  | Meiji (1868), Taishō (1912), Shōwa (1926), Heisei (1989), Reiwa (2019) | `#[non_exhaustive]` for future eras                                                       |
| Buddhist  | BE (Buddhist Era)                                                      | Year = Gregorian + 543                                                                    |
| ROC       | Minguo                                                                 | Year = Gregorian − 1911; year 0 invalid                                                   |
| Coptic    | Era of Martyrs (AM)                                                    | Epoch: 284 CE                                                                             |
| Ethiopic  | Ethiopian Era                                                          | Epoch: ~8 CE                                                                              |

**Validation Error Message Codes:**

| Code                 | Message Key                                    | Used When                                   |
| -------------------- | ---------------------------------------------- | ------------------------------------------- |
| `invalid_month`      | `CalendarMessages::invalid_month_for_calendar` | Month number exceeds calendar's month count |
| `invalid_day`        | `CalendarMessages::invalid_day_for_month`      | Day exceeds max days for the given month    |
| `invalid_leap_month` | `CalendarMessages::invalid_month_for_year`     | Month 13 in non-leap Hebrew year            |
| `invalid_era_date`   | `CalendarMessages::date_before_era_start`      | Date falls before Japanese era start        |
| `out_of_range`       | `CalendarMessages::date_out_of_range`          | Date outside min/max constraints            |
| `invalid_year`       | `CalendarMessages::invalid_year_for_era`       | Year 0 in ROC, or exceeds era bounds        |

> **Calendar-Specific Error Codes:** For Hebrew calendar leap year validation, use the error code `CalendarValidationError::HebrewLeapYearRequired` when a user attempts to select Adar II (month 13) in a non-leap Hebrew year. Per-calendar error messages MUST be sourced from locale data (ICU4X `CalendarProvider`) rather than hardcoded English strings. The `CalendarMessages` struct provides `MessageFn` closures for each error code, allowing adapters to supply locale-appropriate translations. Additional calendar-specific error codes: `IslamicObservationalDateUnavailable` (for observation-based Islamic calendars where future dates cannot be validated), `ChineseLeapMonthInvalid` (for intercalary month validation in Chinese calendar).
>
> `Weekday` — defined in `shared/date-time-types.md`

```rust
/// Era for calendar systems that use them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Era {
    /// The code of the era.
    pub code: String,           // "reiwa", "showa", etc.
    /// The display name of the era.
    pub display_name: String,   // Locale-specific, e.g. "令和"
}

/// Locale identifier (BCP 47).
/// NOTE: Production code should use `ars_i18n::Locale` instead of this placeholder.
/// This definition exists only for spec illustration; the real crate re-exports:
///   `use ars_i18n::Locale;`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(pub String);

impl Locale {
    /// Creates a new locale.
    pub fn new(tag: impl Into<String>) -> Self { Self(tag.into()) }
    /// Creates a new locale for the en-US locale.
    pub fn en_us() -> Self { Self("en-US".to_string()) }
    /// Returns the string representation of the locale.
    pub fn as_str(&self) -> &str { &self.0 }

    /// First day of week for this locale.
    ///
    /// Delegates to `IntlBackend::first_day_of_week()`.
    /// Production: ICU4X `WeekCalculator::first_weekday()` from CLDR `weekData`,
    /// covering all regions (Sunday-start: US, CA, JP, etc.; Saturday-start:
    /// AF, IR, SA, AE, etc.; Monday-start: most of Europe, ISO 8601 default).
    /// Respects the BCP 47 `fw` extension if present (e.g., `en-US-u-fw-mon`).
    pub fn first_day_of_week(&self, backend: &dyn IntlBackend) -> Weekday {
        backend.first_day_of_week(self)
    }

    /// Preferred hour cycle for this locale.
    ///
    /// Delegates to `IntlBackend::hour_cycle()`.
    /// Production: ICU4X `HourCycle` preference from CLDR `timeData`.
    /// Examples: en-US→H12, de→H23, ja→H23, ko→H12, ar→H12, ar-MA→H23.
    pub fn hour_cycle(&self, backend: &dyn IntlBackend) -> HourCycle {
        backend.hour_cycle(self)
    }
}

/// Time value (wall clock, no date or timezone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    /// The hour of the time.
    pub hour: u8,           // 0-23 internal
    /// The minute of the time.
    pub minute: u8,
    /// The second of the time.
    pub second: u8,
    /// The millisecond of the time.
    pub millisecond: u16,
}

impl Time {
    /// Creates a new time.
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self { hour, minute, second, millisecond: 0 }
    }

    /// Display hour in H12 format (1-12).
    pub fn hour_12(&self) -> u8 {
        match self.hour % 12 { 0 => 12, h => h }
    }

    /// Checks if the time is in the PM period.
    pub fn is_pm(&self) -> bool { self.hour >= 12 }
}

// `HourCycle` — defined in `shared/date-time-types.md`
```
