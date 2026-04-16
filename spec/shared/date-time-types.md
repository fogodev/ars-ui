# Date & Time Shared Types

## 1. Overview

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

### 1.1 Shared Types (`ars-i18n`)

All date/time components operate on the following shared types defined in `ars-i18n`:

```rust
// ars-i18n/src/calendar/types.rs
use core::num::NonZero;

/// Shorthand for `NonZero<u8>` from a compile-time-known literal.
/// Panics at const-eval if `n == 0`.
const fn nzu8(n: u8) -> NonZero<u8> {
    match NonZero::new(n) {
        Some(v) => v,
        None => panic!("value must be nonzero"),
    }
}

/// A date in an arbitrary calendar system.
/// **Note:** `CalendarDate` intentionally does NOT derive `Ord` or `PartialOrd`.
/// Raw field-order comparison is unsound across calendar systems and eras.
/// Use `compare_within_calendar()` for same-calendar ordering, or convert to
/// epoch-day via `IcuProvider` for cross-calendar comparison.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDate {
    pub calendar: CalendarSystem,
    pub era: Option<Era>,
    pub year: i32,
    pub month: NonZero<u8>,  // 1-based (1–12)
    pub day: NonZero<u8>,    // 1-based (1–31)
}

impl CalendarDate {
    /// Compare two dates within the same calendar system.
    /// Returns `None` if calendars differ or if both have different eras
    /// (where raw field comparison would be chronologically incorrect).
    pub fn compare_within_calendar(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.calendar != other.calendar {
            return None;
        }
        // If both have eras and they differ, raw comparison is unreliable
        if let (Some(ref a), Some(ref b)) = (&self.era, &other.era) {
            if a.code != b.code {
                return None; // caller must use IcuProvider epoch-day conversion
            }
        }
        // Manual field-by-field comparison (CalendarDate does not derive Ord)
        Some(
            self.year.cmp(&other.year)
                .then(self.month.cmp(&other.month))
                .then(self.day.cmp(&other.day))
        )
    }

    pub fn new_gregorian(year: i32, month: NonZero<u8>, day: NonZero<u8>) -> Self {
        let month_value = month.get();
        let day_value = day.get();
        assert!(year >= 1, "Gregorian dates require a year of at least 1");
        assert!((1..=12).contains(&month_value), "Gregorian dates require a month in the range 1..=12");
        assert!(
            day_value <= gregorian_days_in_month(year, month_value),
            "Gregorian dates require a valid day for the given month"
        );
        Self {
            calendar: CalendarSystem::Gregorian,
            era: None,
            year,
            month,
            day,
        }
    }

    /// Validated constructor. Returns `None` if the date is outside the valid
    /// range for the given calendar system and era.
    ///
    /// Non-Gregorian calendars have different month counts and day-per-month
    /// rules. For example:
    /// - Hebrew: 12 or 13 months (Adar I/II in leap years)
    /// - Ethiopic: 13 months (Pagume has 5-6 days)
    /// - Chinese/Dangi: 12 or 13 months (intercalary month in some years)
    /// - Islamic: Months alternate 29/30 days
    /// - Persian: First 6 months have 31 days, next 5 have 30, last has 29-30
    pub fn new(
        provider: &dyn IcuProvider,
        calendar: CalendarSystem,
        era: Option<Era>,
        year: i32,
        month: u8,
        day: u8,
    ) -> Option<Self> {
        // Multi-era calendars follow the React Aria / Temporal-style model:
        // when the caller omits the era, the provider resolves the current era.
        let era = era.or_else(|| provider.default_era(&calendar));
        let era_code = era.as_ref().map(|e| e.code.as_str());
        if year < 1 || !era_code_is_valid(calendar, era_code) {
            return None;
        }
        let max_month = max_months_in_year(provider, calendar, year, era_code);
        if !(1..=max_month).contains(&month) {
            return None;
        }
        let validated_month = NonZero::new(month)?;
        let provisional = Self {
            calendar,
            era: era.clone(),
            year,
            month: validated_month,
            day: NonZero::new(1).expect("one is non-zero"),
        };
        if let Some(max_year) = provider.years_in_era(&provisional) {
            if provisional.year > max_year {
                return None;
            }
        }
        if month < provider.minimum_month_in_year(&provisional) {
            return None;
        }
        let max_day = days_in_month_for_calendar(provider, calendar, year, month, era_code);
        if !(1..=max_day).contains(&day) {
            return None;
        }
        let candidate = Self {
            calendar,
            era,
            year,
            month: validated_month,
            day: NonZero::new(day).expect("validated 1-based"),
        };
        if day < provider.minimum_day_in_month(&candidate) {
            return None;
        }
        Some(candidate)
    }

    /// String representation in `YYYY-MM-DD` format using raw field values.
    ///
    /// **Note:** For non-Gregorian calendars, this outputs the raw calendar fields
    /// (e.g., Hebrew year 5784, month 7, day 15 → "5784-07-15"), NOT the Gregorian
    /// equivalent. For true ISO 8601 output (Gregorian), use
    /// `IcuProvider::convert_date(date, CalendarSystem::Gregorian)` first.
    pub fn to_iso8601(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month.get(), self.day.get())
    }

    /// Returns `true` if `self` falls within `[start, end]` (inclusive).
    /// Returns `false` if calendars differ or eras are incomparable.
    pub fn is_between(&self, start: &CalendarDate, end: &CalendarDate) -> bool {
        match (self.compare_within_calendar(start), self.compare_within_calendar(end)) {
            (Some(lo), Some(hi)) => {
                lo != core::cmp::Ordering::Less && hi != core::cmp::Ordering::Greater
            }
            _ => false, // cross-calendar or cross-era: use IcuProvider epoch-day instead
        }
    }

    pub fn days_in_month(&self, provider: &dyn IcuProvider) -> u8 {
        let era_code = self.era.as_ref().map(|e| e.code.as_str());
        days_in_month_for_calendar(provider, self.calendar, self.year, self.month.get(), era_code)
    }

    /// Add months to a date within the same calendar and era.
    ///
    /// React Aria-inspired behavior:
    /// - omitted eras are resolved through `IcuProvider::default_era()`
    /// - same-era year rollover is allowed (e.g. Heisei 1-12 -> Heisei 2-01)
    /// - day clamping within a valid start-of-era month clamps forward to the
    ///   minimum valid day
    /// - cross-era rollover returns `None`; callers should use
    ///   `IcuProvider::convert_date()` to round-trip through ISO and re-resolve era
    pub fn add_months(&self, provider: &dyn IcuProvider, n: i32) -> Option<CalendarDate> {
        let era = self.era.clone().or_else(|| provider.default_era(&self.calendar));
        let era_code = era.as_ref().map(|e| e.code.as_str());
        let (year, month_u8) = if let Some(months_per_year) = fixed_months_per_year(self.calendar) {
            let total_month_index = i64::from(self.year - 1)
                .checked_mul(i64::from(months_per_year))?
                .checked_add(i64::from(self.month.get() - 1))?
                .checked_add(i64::from(n))?;
            let normalized_year = total_month_index.div_euclid(i64::from(months_per_year)).checked_add(1)?;
            let normalized_month = total_month_index.rem_euclid(i64::from(months_per_year));
            if normalized_year < 1 {
                return None;
            }
            (
                i32::try_from(normalized_year).ok()?,
                u8::try_from(normalized_month.checked_add(1)?).ok()?,
            )
        } else {
            let mut year = i64::from(self.year);
            let mut month = i64::from(self.month.get()).checked_add(i64::from(n))?;
            loop {
                let current_year = i32::try_from(year).ok()?;
                let max = i64::from(provider.max_months_in_year(&self.calendar, current_year, era_code));
                if month <= max { break; }
                month -= max;
                year = year.checked_add(1)?;
            }
            while month < 1 {
                year = year.checked_sub(1)?;
                let current_year = i32::try_from(year).ok()?;
                let max = i64::from(provider.max_months_in_year(&self.calendar, current_year, era_code));
                month += max;
            }
            (
                i32::try_from(year).ok()?,
                u8::try_from(month).expect("validated month count fits in u8"),
            )
        };
        let mut candidate = CalendarDate {
            year,
            month: NonZero::new(month_u8).expect("month result is 1-based"),
            day: self.day,
            calendar: self.calendar,
            era: era.clone(),
        };
        let min_month = provider.minimum_month_in_year(&candidate);
        if month_u8 < min_month {
            return None;
        }
        let max_day = days_in_month_for_calendar(provider, self.calendar, year, month_u8, era_code);
        let mut clamped_day = self.day.get().min(max_day);
        candidate.day = NonZero::new(clamped_day).expect("day clamped to valid range");
        let min_day = provider.minimum_day_in_month(&candidate);
        if clamped_day < min_day {
            clamped_day = min_day;
        }
        CalendarDate::new(provider, self.calendar, era, year, month_u8, clamped_day)
    }

    /// Add days (Gregorian-only). Returns None for non-Gregorian calendars.
    /// For non-Gregorian day arithmetic, use `add_days_with_provider()`.
    pub fn add_days(&self, n: i32) -> Option<CalendarDate> {
        if self.calendar != CalendarSystem::Gregorian {
            return None;
        }
        let jdn = self.to_jdn() + n as i64;
        let shifted = Self::from_jdn(jdn, self.calendar);
        (shifted.year >= 1).then_some(shifted)
    }

    /// Add days for any calendar system, delegating to the ICU provider for
    /// non-Gregorian calendars. This parallels `add_months` which also uses
    /// `IcuProvider` for universal calendar support.
    pub fn add_days_with_provider(&self, provider: &dyn IcuProvider, n: i32) -> CalendarDate {
        if self.calendar == CalendarSystem::Gregorian {
            self
                .add_days(n)
                .expect("Gregorian day arithmetic must remain within the supported year range")
        } else {
            // Convert to Gregorian, add days, convert back
            let greg = provider.convert_date(self, CalendarSystem::Gregorian);
            let greg_result = greg
                .add_days(n)
                .expect("provider-backed day arithmetic must remain within the supported year range");
            provider.convert_date(&greg_result, self.calendar)
        }
    }

    /// Convert a Gregorian date to Julian Day Number.
    /// NOTE: This algorithm is Gregorian-only. For non-Gregorian calendars, convert through IcuProvider first.
    fn to_jdn(&self) -> i64 {
        debug_assert!(self.calendar == CalendarSystem::Gregorian, "to_jdn is Gregorian-only");
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
    /// NOTE: This algorithm is Gregorian-only. For non-Gregorian calendars, convert through IcuProvider first.
    fn from_jdn(jdn: i64, calendar: CalendarSystem) -> CalendarDate {
        debug_assert!(
            calendar == CalendarSystem::Gregorian,
            "from_jdn is Gregorian-only; non-Gregorian calendars must convert through IcuProvider"
        );
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
    /// e.g., Gregorian -> Japanese, Islamic -> Gregorian.
    ///
    /// Same-calendar conversions return a clone. Cross-calendar conversions
    /// require `provider` to perform calendar-aware conversion via ICU4X.
    pub fn to_calendar(&self, provider: &dyn IcuProvider, target: CalendarSystem) -> CalendarDate {
        if self.calendar == target {
            return self.clone();
        }
        // Non-Gregorian conversion: delegate to IcuProvider
        provider.convert_date(self, target)
    }

    /// NOTE: Sakamoto's algorithm is Gregorian-only. For non-Gregorian dates, convert to Gregorian first.
    pub fn weekday(&self) -> Weekday {
        debug_assert!(self.calendar == CalendarSystem::Gregorian, "weekday() is Gregorian-only");
        // Tomohiko Sakamoto's algorithm for Gregorian.
        let year = if self.month.get() < 3 { self.year - 1 } else { self.year };
        let month_index = usize::from(self.month.get() - 1);
        let day = i64::from(self.day.get());
        static OFFSETS: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let year = i64::from(year);
        let weekday = year + year.div_euclid(4) - year.div_euclid(100) + year.div_euclid(400) + OFFSETS[month_index] + day;
        Weekday::from_sunday_zero(weekday.rem_euclid(7) as u8)
    }
}

/// Validated month value (1..=13). Library code uses this to avoid panics on invalid input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month(u8);

impl Month {
    pub fn new(m: u8) -> Option<Self> {
        // 13 months needed for Hebrew calendar leap years
        (1..=13).contains(&m).then(|| Self(m))
    }
    pub fn get(&self) -> u8 { self.0 }
}

impl TryFrom<u8> for Month {
    type Error = ();
    fn try_from(m: u8) -> Result<Self, ()> {
        Self::new(m).ok_or(())
    }
}

fn gregorian_days_in_month(year: i32, month: u8) -> u8 {
    debug_assert!((1..=12).contains(&month), "invalid month {month}");
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            if leap { 29 } else { 28 }
        }
        // Safety: debug_assert above catches invalid months in debug builds.
        // In release builds, return 30 as a safe fallback.
        _ => 30,
    }
}

/// Maximum number of months in a year for the given calendar system.
///
/// Delegates to `IcuProvider::max_months_in_year()` for calendar-aware computation.
/// Production (`Icu4xProvider`): uses ICU4X `Date::months_in_year()` for
/// precise results across all calendar systems (Hebrew 13-month leap years,
/// Chinese/Dangi intercalary months, Ethiopic/Coptic 13th month).
/// Tests (`StubIcuProvider`): returns hardcoded values matching the original
/// English-only logic (see `04-internationalization.md` §9.5).
fn max_months_in_year(provider: &dyn IcuProvider, calendar: CalendarSystem, year: i32, era: Option<&str>) -> u8 {
    provider.max_months_in_year(&calendar, year, era)
}

fn era_code_is_valid(calendar: CalendarSystem, era: Option<&str>) -> bool {
    match calendar {
        CalendarSystem::Japanese => era.is_some_and(|era_code| {
            CalendarSystem::japanese_eras()
                .iter()
                .any(|candidate| candidate.name.eq_ignore_ascii_case(era_code))
        }),
        _ => era.is_none(),
    }
}

/// Days in a given month for the given calendar system.
///
/// Delegates to `IcuProvider::days_in_month()` for calendar-aware computation.
/// Production (`Icu4xProvider`): uses ICU4X `Date::days_in_month()`
/// for precise results including observation-based Islamic calendars, Hebrew
/// deficient/regular/complete year types, and Chinese/Dangi lunation lengths.
/// Tests (`StubIcuProvider`): falls back to Gregorian logic for all calendars
/// (see `04-internationalization.md` §9.5).
fn days_in_month_for_calendar(provider: &dyn IcuProvider, calendar: CalendarSystem, year: i32, month: u8, era: Option<&str>) -> u8 {
    provider.days_in_month(&calendar, year, month, era)
}

/// Inclusive date range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    pub start: CalendarDate,
    pub end: CalendarDate,
}

impl DateRange {
    /// Returns `None` if `start > end` or if calendars/eras are incomparable.
    /// Prefer `normalized()` for auto-ordering.
    pub fn new(start: CalendarDate, end: CalendarDate) -> Option<Self> {
        match start.compare_within_calendar(&end) {
            Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal) => {
                Some(Self { start, end })
            }
            Some(core::cmp::Ordering::Greater) => None,
            None => None, // cross-calendar or cross-era: use IcuProvider epoch-day instead
        }
    }

    pub fn contains(&self, date: &CalendarDate) -> bool {
        date.is_between(&self.start, &self.end)
    }

    /// ISO 8601 date range string (e.g., "2024-03-15/2024-03-20").
    pub fn to_iso8601(&self) -> String {
        format!("{}/{}", self.start.to_iso8601(), self.end.to_iso8601())
    }

    /// Primary constructor — always produces a valid range by ordering inputs.
    /// Returns `None` if calendars/eras are incomparable.
    pub fn normalized(a: CalendarDate, b: CalendarDate) -> Option<Self> {
        match a.compare_within_calendar(&b) {
            Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal) => {
                Some(Self { start: a, end: b })
            }
            Some(core::cmp::Ordering::Greater) => {
                Some(Self { start: b, end: a })
            }
            None => None, // cross-calendar or cross-era: use IcuProvider epoch-day instead
        }
    }
}

// Canonical definition. Extended with methods in 04-internationalization.md §5.2.
///
/// Supported calendar systems. Runtime dispatch — no feature gates.
/// All variants are always available; the IcuProvider handles calendar-specific logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CalendarSystem {
    #[default]
    Gregorian,
    Buddhist,
    Japanese,
    Hebrew,
    Islamic,          // Maps to ICU4X 2.x `HijriSimulatedMecca`
    IslamicCivil,     // Maps to ICU4X 2.x `HijriTabularTypeIIFriday`
    IslamicUmmAlQura, // Maps to ICU4X 2.x `HijriUmmAlQura`
    Persian,
    Indian,
    Chinese,
    Coptic,
    Dangi,
    Ethiopic,
    EthiopicAmeteAlem,
    Roc,
}
```

#### 1.1.1 Typed Calendar Views

`CalendarDate` remains the dynamic boundary type because locale resolution,
formatting, serialization, and provider conversion all operate on runtime
calendar values. Calendar-specific behavior is exposed through a typed wrapper
that moves those constraints into Rust's type system instead of relying only on
runtime checks.

```rust
use core::marker::PhantomData;

mod sealed {
    pub trait Sealed {}
}

pub trait CalendarKind: sealed::Sealed + Copy + Clone + Default + 'static {
    const SYSTEM: CalendarSystem;
}

pub trait DirectDayArithmetic: CalendarKind {}
pub trait DirectWeekdayComputation: CalendarKind {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Gregorian;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Buddhist;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Japanese;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Hebrew;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Islamic;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IslamicCivil;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IslamicUmmAlQura;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Persian;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Indian;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Chinese;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Coptic;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Dangi;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ethiopic;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EthiopicAmeteAlem;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Roc;

pub struct CalendarTypeError {
    pub expected: CalendarSystem,
    pub found: CalendarSystem,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypedCalendarDate<C: CalendarKind> {
    raw: CalendarDate,
    marker: PhantomData<C>,
}

impl CalendarDate {
    pub fn typed<C: CalendarKind>(&self) -> Result<TypedCalendarDate<C>, CalendarTypeError>;
    pub fn into_typed<C: CalendarKind>(self) -> Result<TypedCalendarDate<C>, CalendarTypeError>;
    pub fn to_calendar_type<C: CalendarKind>(&self, provider: &dyn IcuProvider) -> TypedCalendarDate<C>;
}

impl<C: CalendarKind> TypedCalendarDate<C> {
    pub fn from_raw(raw: CalendarDate) -> Result<Self, CalendarTypeError>;
    pub fn calendar_system() -> CalendarSystem;
    pub fn as_raw(&self) -> &CalendarDate;
    pub fn into_raw(self) -> CalendarDate;
    pub fn era(&self) -> Option<&Era>;
    pub fn year(&self) -> i32;
    pub fn month(&self) -> NonZero<u8>;
    pub fn day(&self) -> NonZero<u8>;
    pub fn days_in_month(&self, provider: &dyn IcuProvider) -> u8;
    pub fn add_months(&self, provider: &dyn IcuProvider, n: i32) -> Option<Self>;
    pub fn add_days_with_provider(&self, provider: &dyn IcuProvider, n: i32) -> Self;
    pub fn to_calendar<T: CalendarKind>(&self, provider: &dyn IcuProvider) -> TypedCalendarDate<T>;
    pub fn compare_within_calendar(&self, other: &Self) -> Option<core::cmp::Ordering>;
}

impl<C: DirectDayArithmetic> TypedCalendarDate<C> {
    pub fn add_days(&self, n: i32) -> Option<Self>;
}

impl<C: DirectWeekdayComputation> TypedCalendarDate<C> {
    pub fn weekday(&self) -> Weekday;
}

impl TypedCalendarDate<Gregorian> {
    pub fn new(year: i32, month: NonZero<u8>, day: NonZero<u8>) -> Self;
}
```

`Gregorian` implements both `DirectDayArithmetic` and `DirectWeekdayComputation`.
No other marker type implements those traits in this issue. This means
`TypedCalendarDate<Gregorian>` exposes `add_days()` and `weekday()`, while
other typed calendars must use `add_days_with_provider()` or convert through
`to_calendar::<Gregorian>()`.

#### 1.1.2 Calendar System Validation

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
- **Chinese / Dangi**: Intercalary (leap) months can occur after any month. The public `CalendarDate` type keeps `month: NonZero<u8>` and represents these calendars using the ordinal month position within the year (`1..=13`), not a separate public leap-month boolean. Validation therefore requires consulting calendar data or calendar arithmetic for the specific year to determine whether ordinal month `13` exists and how ordinal months map onto named display months.

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
// - Months: 1–12 in common years, 1–13 in leap years (month 13 = Adar II).
// - Leap years follow a 19-year Metonic cycle: years 3, 6, 8, 11, 14, 17, 19.
// - `CalendarDate::validated()` MUST reject month 13 when the year is not a
// leap year. Error: `CalendarMessages::invalid_month_for_year`.
// - Month lengths vary: Cheshvan (29 or 30), Kislev (29 or 30) depend on year type.
//
// ### Japanese Calendar (`japanese` feature)
// - Eras: Meiji (1868), Taisho (1912), Showa (1926), Heisei (1989), Reiwa (2019).
// - Era transitions: year 1 of new era starts on transition date (not Jan 1).
// Example: Reiwa 1 starts May 1, 2019 — dates before that in 2019 are Heisei 31.
// - `CalendarDate::validated()` MUST reject dates that fall before the era's
// start date. Error: `CalendarMessages::date_before_era_start`.
// - Future eras: use `#[non_exhaustive]` on `Era`; the ICU4X provider supplies
// new eras via data updates without code changes.
//
// ### Persian (Solar Hijri) Calendar (`persian` feature)
// - Months 1–6 have 31 days, months 7–11 have 30 days, month 12 has 29 or 30.
// - Leap years: month 12 has 30 days. Leap determination follows the 2820-year
// cycle algorithm (or delegated to ICU4X `icu::calendar::Persian`).
// - `CalendarDate::validated()` MUST reject day 30 of month 12 in non-leap years.
// Error: `CalendarMessages::invalid_day_for_month`.
//
// ### Buddhist Calendar (`buddhist` feature)
// - Structurally identical to Gregorian but year = Gregorian year + 543.
// - Thailand uses Buddhist Era (BE); the segment order for `th` locale is D/M/Y
// with the year displayed in BE. `segments_for_locale` handles this via ICU4X.
// - No special month/day validation beyond Gregorian rules.
//
// ### Islamic Calendars (`islamic` feature)
// - All variants have 12 months. Odd months have 30 days, even months have 29 days,
// except month 12 which has 30 days in leap years.
// - Leap year rules differ by variant (tabular: 2,5,7,10,13,16,18,21,24,26,29
// in a 30-year cycle; observational: determined by moon sighting).
// - `CalendarDate::validated()` MUST use the variant-specific leap rule.
//
// ### Min/Max Validation Across Calendars
// - `min_value` and `max_value` props on DateField/Calendar/DatePicker are compared
// after converting both the constraint and the candidate to a common epoch-day
// representation (Julian Day Number). This ensures correct cross-calendar
// comparison (e.g., min in Gregorian, value entered in Hebrew).
// - Validation error: `CalendarMessages::date_out_of_range { min, max }` with
// both dates formatted in the active calendar system for the error message.
```

#### 1.1.3 Comprehensive Calendar Validation Reference

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

| Calendar  | Epoch / Era                                                            | Notes                                                                                                                                                                                                 |
| --------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gregorian | CE / BCE                                                               | CalendarDate.year uses astronomical year numbering internally: year 0 exists and corresponds to 1 BCE. Display formatting should convert to proleptic Gregorian (no year 0) when presenting to users. |
| Japanese  | Meiji (1868), Taishō (1912), Shōwa (1926), Heisei (1989), Reiwa (2019) | `#[non_exhaustive]` for future eras                                                                                                                                                                   |
| Buddhist  | BE (Buddhist Era)                                                      | Year = Gregorian + 543                                                                                                                                                                                |
| ROC       | Minguo                                                                 | Year = Gregorian − 1911; year 0 invalid                                                                                                                                                               |
| Coptic    | Era of Martyrs (AM)                                                    | Epoch: 284 CE                                                                                                                                                                                         |
| Ethiopic  | Ethiopian Era                                                          | Epoch: ~8 CE                                                                                                                                                                                          |

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

```rust
// Canonical CalendarMessages definition.
// Provides localizable error messages for calendar validation.
// Adapters supply locale-appropriate closures; defaults are English fallbacks.
use ars_i18n::{MessageFn, Locale};

// Trait objects include `+ Send + Sync` on all targets (see design note in
// 04-internationalization.md §7.1).

pub struct CalendarMessages {
    /// Month number exceeds the calendar's month count.
    /// Receives (month_number, max_months, locale).
    pub invalid_month_for_calendar: MessageFn<dyn Fn(u8, u8, &Locale) -> String + Send + Sync>,
    /// Day exceeds the maximum for the given month.
    /// Receives (day, max_day, month_name, locale).
    pub invalid_day_for_month: MessageFn<dyn Fn(u8, u8, &str, &Locale) -> String + Send + Sync>,
    /// Leap month used in a non-leap year (e.g., Hebrew month 13).
    /// Receives (month_number, year, locale).
    pub invalid_month_for_year: MessageFn<dyn Fn(u8, i32, &Locale) -> String + Send + Sync>,
    /// Date falls before the start of the specified era (e.g., Japanese era).
    /// Receives (era_name, era_start_display, locale).
    pub date_before_era_start: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
    /// Date falls outside the min/max range constraint.
    /// Receives (min_display, max_display, locale). Either may be empty if unbounded.
    pub date_out_of_range: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
    /// Year is invalid for the era (e.g., year 0 in ROC).
    /// Receives (year, era_name, locale).
    pub invalid_year_for_era: MessageFn<dyn Fn(i32, &str, &Locale) -> String + Send + Sync>,
}

impl Default for CalendarMessages {
    fn default() -> Self {
        Self {
            invalid_month_for_calendar: MessageFn::new(|month: u8, max: u8, _: &Locale| {
                format!("Month {month} is invalid (maximum {max})")
            }),
            invalid_day_for_month: MessageFn::new(|day: u8, max: u8, month: &str, _: &Locale| {
                format!("{month} has at most {max} days (got {day})")
            }),
            invalid_month_for_year: MessageFn::new(|month: u8, year: i32, _: &Locale| {
                format!("Month {month} does not exist in year {year}")
            }),
            date_before_era_start: MessageFn::new(|era: &str, start: &str, _: &Locale| {
                format!("Date is before the start of {era} ({start})")
            }),
            date_out_of_range: MessageFn::new(|min: &str, max: &str, _: &Locale| {
                match (min.is_empty(), max.is_empty()) {
                    (false, false) => format!("Date must be between {min} and {max}"),
                    (false, true) => format!("Date must be after {min}"),
                    (true, false) => format!("Date must be before {max}"),
                    (true, true) => "Date is out of range".into(),
                }
            }),
            invalid_year_for_era: MessageFn::new(|year: i32, era: &str, _: &Locale| {
                format!("Year {year} is invalid for {era}")
            }),
        }
    }
}

impl core::fmt::Debug for CalendarMessages {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CalendarMessages").finish_non_exhaustive()
    }
}
```

```rust
// Canonical Weekday. ISO 8601: Monday=1..Sunday=7.
// Implemented in `ars_i18n` and re-exported by `ars_core` for convenience.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Create a Weekday from a Sunday-zero-indexed number (0=Sunday, 6=Saturday).
    /// This matches the JavaScript `Date.getDay()` convention.
    pub fn from_sunday_zero(n: u8) -> Self {
        // Canonical definition — 04-internationalization.md re-uses this.
        const WEEKDAYS: [Weekday; 7] = [
            Weekday::Sunday, Weekday::Monday, Weekday::Tuesday,
            Weekday::Wednesday, Weekday::Thursday, Weekday::Friday,
            Weekday::Saturday,
        ];
        WEEKDAYS[(n % 7) as usize]
    }

    /// Create a Weekday from ISO 8601 numbering (1=Monday, 7=Sunday).
    /// Returns `None` for values outside the valid range 1..=7.
    pub fn from_iso_8601(n: u8) -> Option<Self> {
        match n {
            1 => Some(Weekday::Monday),
            2 => Some(Weekday::Tuesday),
            3 => Some(Weekday::Wednesday),
            4 => Some(Weekday::Thursday),
            5 => Some(Weekday::Friday),
            6 => Some(Weekday::Saturday),
            7 => Some(Weekday::Sunday),
            _ => None,
        }
    }

    /// Short weekday label (abbreviated format) for the given locale.
    ///
    /// Delegates to `IcuProvider::weekday_short_label()`.
    /// Production: ICU4X `DateSymbols::weekday_names(FieldLength::Abbreviated)`.
    /// Examples: en→"Mo", de→"Mo", ja→"月", ar→"ن", ko→"월".
    pub fn short_label(&self, provider: &dyn IcuProvider, locale: &Locale) -> String {
        provider.weekday_short_label(*self, locale)
    }

    /// Long weekday label (wide format) for the given locale.
    ///
    /// Delegates to `IcuProvider::weekday_long_label()`.
    /// Production: ICU4X `DateSymbols::weekday_names(FieldLength::Wide)`.
    /// Examples: en→"Monday", de→"Montag", ja→"月曜日", ar→"الاثنين".
    pub fn long_label(&self, provider: &dyn IcuProvider, locale: &Locale) -> String {
        provider.weekday_long_label(*self, locale)
    }
}

/// Era for calendar systems that use them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Era {
    pub code: String,           // "reiwa", "showa", etc.
    pub display_name: String,   // Locale-specific, e.g. "令和"
}

/// **Production code MUST use `ars_i18n::Locale` directly.** This placeholder is
/// `#[cfg(test)]` only — it exists so that unit tests in `date-time-types` can compile
/// without pulling in the full `ars-i18n` crate. The two types convert via `From`:
///   `impl From<&ars_i18n::Locale> for Locale { fn from(l) -> Self { Locale(l.to_bcp47()) } }`
///   `impl From<&Locale> for ars_i18n::Locale { fn from(l) -> Self { ars_i18n::Locale::parse(l.as_str()).expect("test locale is valid BCP-47") } }`
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(pub String);

#[cfg(test)]
impl Locale {
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    pub fn en_us() -> Self {
        Self("en-US".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// First day of week for this locale.
    ///
    /// Delegates to `IcuProvider::first_day_of_week()`.
    /// Production: ICU4X `WeekInformation::first_weekday` from CLDR `weekData`,
    /// covering all regions (Sunday-start: US, CA, JP, etc.; Saturday-start:
    /// AF, IR, SA, AE, etc.; Monday-start: most of Europe, ISO 8601 default).
    /// Respects the BCP 47 `fw` extension if present (e.g., `en-US-u-fw-mon`).
    pub fn first_day_of_week(&self, provider: &dyn IcuProvider) -> Weekday {
        let locale = ars_i18n::Locale::parse(self.as_str()).expect("test locale is valid BCP-47");
        provider.first_day_of_week(&locale)
    }

    /// Preferred hour cycle for this locale.
    ///
    /// Delegates to `IcuProvider::hour_cycle()`.
    /// Production: ICU4X `HourCycle` preference from CLDR `timeData`.
    /// Examples: en-US→H12, de→H23, ja→H23, ko→H12, ar→H12, ar-MA→H23.
    pub fn hour_cycle(&self, provider: &dyn IcuProvider) -> HourCycle {
        let locale = ars_i18n::Locale::parse(self.as_str()).expect("test locale is valid BCP-47");
        provider.hour_cycle(&locale)
    }
}

/// Time value (wall clock, no date or timezone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    pub hour: u8,           // 0-23 internal
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
}

impl Time {
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self { hour, minute, second, millisecond: 0 }
    }

    /// Display hour in H12 format (1-12).
    pub fn hour_12(&self) -> u8 {
        match self.hour % 12 {
            0 => 12,
            h => h,
        }
    }

    pub fn is_pm(&self) -> bool {
        self.hour >= 12
    }

    /// ISO 8601 time string (e.g., "14:30:00" or "14:30:00.500" when milliseconds are non-zero).
    pub fn to_iso8601(&self) -> String {
        debug_assert!((0..=999).contains(&self.millisecond), "millisecond must be 0-999, got {}", self.millisecond);
        if self.millisecond > 0 {
            format!("{:02}:{:02}:{:02}.{:03}", self.hour, self.minute, self.second, self.millisecond)
        } else {
            format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
        }
    }
}

/// Hour cycle for time display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HourCycle {
    H11,    // 0-11, with AM/PM
    H12,    // 1-12, with AM/PM
    H23,    // 0-23, no AM/PM
    H24,    // 1-24, no AM/PM
    Auto,   // From locale
}

impl HourCycle {
    pub fn has_day_period(&self) -> bool {
        matches!(self, HourCycle::H11 | HourCycle::H12)
    }

    pub fn resolve(&self, provider: &dyn IcuProvider, locale: &Locale) -> HourCycle {
        match self {
            HourCycle::Auto => locale.hour_cycle(provider),
            other => *other,
        }
    }

    pub fn display_hour_range(&self) -> (u8, u8) {
        match self {
            HourCycle::H11 => (0, 11),
            HourCycle::H12 => (1, 12),
            HourCycle::H23 | HourCycle::Auto => (0, 23),
            HourCycle::H24 => (1, 24),
        }
    }
}
```
