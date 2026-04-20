# Date & Time Shared Types

## 1. Overview

This document specifies the shared calendar and time types used by ars-ui's date and time components.

The public contract lives in `ars-i18n`. `ars-core`, `ars-leptos`, and `ars-dioxus` consume these
types but do not define alternative calendar models.

The current design is intentionally close to React Aria and ECMAScript Temporal:

- calendar identifiers are explicit
- dates are opaque validated values, not raw public structs
- pure calendar arithmetic does not depend on `IcuProvider`
- locale-aware week and formatting queries still use `IcuProvider`
- zoned and local-time behavior is available on `std` builds

### 1.1 Shared Types (`ars-i18n`)

The shared public surface is:

```rust
use core::cmp::Ordering;

use ars_i18n::{CalendarError, CalendarConversionError, DateError, IcuProvider, Locale, Weekday};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CalendarSystem {
    Iso8601,
    #[default]
    Gregorian,
    Buddhist,
    Japanese,
    Hebrew,
    IslamicCivil,
    IslamicUmmAlQura,
    Persian,
    Indian,
    Chinese,
    Coptic,
    Dangi,
    Ethiopic,
    EthiopicAmeteAlem,
    Roc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarMetadata {
    pub calendar: CalendarSystem,
    pub bcp47: &'static str,
    pub has_custom_eras: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JapaneseEra {
    pub code: &'static str,
    pub english_name: &'static str,
    pub japanese_name: &'static str,
    pub start_year: i32,
    pub start_month: u8,
    pub start_day: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Era {
    pub code: String,
    pub display_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MonthCode(/* private */);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeZoneId(/* private */);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CalendarDateFields {
    pub era: Option<Era>,
    pub year: Option<i32>,
    pub month: Option<u8>,
    pub month_code: Option<MonthCode>,
    pub day: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeFields {
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub millisecond: Option<u16>,
    pub microsecond: Option<u16>,
    pub nanosecond: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateDuration {
    pub years: i32,
    pub months: i32,
    pub weeks: i32,
    pub days: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeDuration {
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub milliseconds: i64,
    pub microseconds: i64,
    pub nanoseconds: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateTimeDuration {
    pub date: DateDuration,
    pub time: TimeDuration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateField {
    Year,
    Month,
    Day,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeField {
    Hour,
    Minute,
    Second,
    Millisecond,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateTimeField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleOptions {
    pub wrap: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleTimeOptions {
    pub wrap: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disambiguation {
    #[default]
    Compatible,
    Earlier,
    Later,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDate { /* private */ }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time { /* private */ }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDateTime { /* private */ }

#[cfg(feature = "std")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZonedDateTime { /* private */ }
```

The shared calendar methods are:

```rust
impl CalendarSystem {
    pub fn from_bcp47(identifier: &str) -> Option<Self>;
    pub const fn to_bcp47_value(self) -> &'static str;
    pub fn from_locale(locale: &Locale) -> Self;
    pub const fn has_custom_eras(self) -> bool;
    pub const fn japanese_eras() -> &'static [JapaneseEra];
    pub const fn supported_calendars() -> &'static [CalendarMetadata];

    pub fn eras(self) -> Vec<Era>;
    pub fn default_era(self) -> Option<Era>;
    pub fn months_in_year(self, date: &CalendarDate) -> u8;
    pub fn days_in_month(self, date: &CalendarDate) -> u8;
    pub fn years_in_era(self, date: &CalendarDate) -> Option<i32>;
    pub fn minimum_month_in_year(self, date: &CalendarDate) -> u8;
    pub fn minimum_day_in_month(self, date: &CalendarDate) -> u8;
}

impl MonthCode {
    pub fn new(code: &str) -> Result<Self, DateError>;
    pub fn new_normal(month: u8) -> Result<Self, DateError>;
    pub fn as_str(&self) -> &str;
    pub fn is_leap(self) -> bool;
}

impl TimeZoneId {
    pub fn new(identifier: impl AsRef<str>) -> Result<Self, CalendarError>;
    pub fn as_str(&self) -> &str;
}
```

The date, time, and zoned value methods are:

```rust
impl CalendarDate {
    pub fn new(calendar: CalendarSystem, fields: CalendarDateFields) -> Result<Self, CalendarError>;
    pub fn new_iso8601(year: i32, month: u8, day: u8) -> Result<Self, CalendarError>;
    pub fn new_gregorian(year: i32, month: u8, day: u8) -> Result<Self, CalendarError>;

    pub fn calendar(&self) -> CalendarSystem;
    pub fn era(&self) -> Option<&Era>;
    pub fn year(&self) -> i32;
    pub fn month(&self) -> u8;
    pub fn month_code(&self) -> Option<MonthCode>;
    pub fn day(&self) -> u8;

    pub fn add(&self, duration: DateDuration) -> Result<Self, CalendarError>;
    pub fn subtract(&self, duration: DateDuration) -> Result<Self, CalendarError>;
    pub fn set(&self, fields: &CalendarDateFields) -> Result<Self, CalendarError>;
    pub fn cycle(&self, field: DateField, amount: i32, options: CycleOptions) -> Result<Self, CalendarError>;

    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError>;
    pub fn compare(&self, other: &Self) -> Ordering;
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering>;
    pub fn is_between(&self, start: &CalendarDate, end: &CalendarDate) -> bool;
    pub fn to_iso8601(&self) -> String;
    pub fn weekday(&self) -> Weekday;
    pub fn add_days(&self, days: i32) -> Result<Self, CalendarError>;
    pub fn days_until(&self, other: &Self) -> Result<i32, CalendarError>;
    pub fn is_before(&self, other: &Self) -> Result<bool, CalendarError>;
    pub fn today(calendar: CalendarSystem) -> Result<Self, CalendarError>;
    #[cfg(feature = "std")]
    pub fn to_system_time(&self, time_zone: &TimeZoneId) -> Result<std::time::SystemTime, CalendarError>;

    pub fn days_in_month(&self) -> u8;
    pub fn years_in_era(&self) -> Option<i32>;
    pub fn minimum_month_in_year(&self) -> u8;
    pub fn minimum_day_in_month(&self) -> u8;
}

impl Time {
    pub fn new(hour: u8, minute: u8, second: u8, millisecond: u16) -> Result<Self, DateError>;

    pub fn hour(&self) -> u8;
    pub fn minute(&self) -> u8;
    pub fn second(&self) -> u8;
    pub fn millisecond(&self) -> u16;
    pub fn microsecond(&self) -> u16;
    pub fn nanosecond(&self) -> u16;

    pub fn add(&self, duration: TimeDuration) -> Result<Self, DateError>;
    pub fn subtract(&self, duration: TimeDuration) -> Result<Self, DateError>;
    pub fn set(&self, fields: TimeFields) -> Result<Self, DateError>;
    pub fn cycle(&self, field: TimeField, amount: i64, options: CycleTimeOptions) -> Result<Self, DateError>;
}

impl CalendarDateTime {
    pub fn new(date: CalendarDate, time: Time) -> Self;
    pub fn date(&self) -> &CalendarDate;
    pub fn time(&self) -> &Time;
    pub fn add(&self, duration: DateTimeDuration) -> Result<Self, CalendarError>;
    pub fn subtract(&self, duration: DateTimeDuration) -> Result<Self, CalendarError>;
    pub fn set(&self, date_fields: &CalendarDateFields, time_fields: TimeFields) -> Result<Self, CalendarError>;
    pub fn cycle(&self, field: DateTimeField, amount: i64, options: CycleTimeOptions) -> Result<Self, CalendarError>;
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError>;
    #[cfg(feature = "std")]
    pub fn to_system_time(
        &self,
        time_zone: &TimeZoneId,
        disambiguation: Disambiguation,
    ) -> Result<std::time::SystemTime, CalendarError>;
}

#[cfg(feature = "std")]
impl ZonedDateTime {
    pub fn new(
        date_time: &CalendarDateTime,
        time_zone: TimeZoneId,
        disambiguation: Disambiguation,
    ) -> Result<Self, CalendarError>;

    pub fn time_zone(&self) -> &TimeZoneId;
    pub fn offset_minutes(&self) -> i32;
    pub fn add(&self, duration: DateTimeDuration) -> Result<Self, CalendarError>;
    pub fn subtract(&self, duration: DateTimeDuration) -> Result<Self, CalendarError>;
    pub fn with_time_zone(&self, time_zone: TimeZoneId) -> Result<Self, CalendarError>;
    pub fn to_calendar(&self, target: CalendarSystem) -> Result<Self, CalendarConversionError>;
    pub fn to_system_time(&self) -> Result<std::time::SystemTime, CalendarError>;
}
```

Semantics:

- `CalendarDate` stores canonical ISO date slots internally and projects display fields through the selected calendar.
- `CalendarSystem::Iso8601` is era-less. `CalendarSystem::Gregorian` is distinct and publicly exposes `bc` / `ad`.
- `CalendarDate::to_iso8601()` always serializes the canonical ISO slots, not the display year/month/day of the selected calendar.
- `CalendarDate::compare()` compares canonical ISO days. `compare_within_calendar()` is the raw same-calendar, same-era comparison guard.
- `CalendarDate::constrain()` and `balance()` are defined on already-validated opaque dates, so they are identity-preserving normalization operations in the current implementation.
- `CalendarDate::to_system_time(time_zone)` converts the date at local midnight in the supplied zone.
- `CalendarDateTime::to_system_time(time_zone, disambiguation)` converts the local date-time in the supplied zone and applies the requested gap/overlap policy.
- `ZonedDateTime::to_system_time()` preserves the represented instant exactly.
- `ZonedDateTime` is `std`-only because time-zone lookup, disambiguation, and current-time helpers depend on the runtime engine.

#### 1.1.1 Typed Calendar Views

`ars-i18n` keeps a typed wrapper for statically-known calendars:

```rust
pub trait CalendarKind {
    const SYSTEM: CalendarSystem;
}

pub trait DirectDayArithmetic: CalendarKind {}
pub trait DirectWeekdayComputation: CalendarKind {}

pub struct Gregorian;
pub struct Buddhist;
pub struct Japanese;
pub struct Hebrew;
pub struct IslamicCivil;
pub struct IslamicUmmAlQura;
pub struct Persian;
pub struct Indian;
pub struct Chinese;
pub struct Coptic;
pub struct Dangi;
pub struct Ethiopic;
pub struct EthiopicAmeteAlem;
pub struct Roc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarTypeError {
    pub expected: CalendarSystem,
    pub found: CalendarSystem,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypedCalendarDate<C: CalendarKind> { /* private */ }

impl CalendarDate {
    pub fn typed<C: CalendarKind>(&self) -> Result<TypedCalendarDate<C>, CalendarTypeError>;
    pub fn into_typed<C: CalendarKind>(self) -> Result<TypedCalendarDate<C>, CalendarTypeError>;
    pub fn to_calendar_type<C: CalendarKind>(&self) -> Result<TypedCalendarDate<C>, CalendarConversionError>;
}

impl<C: CalendarKind> TypedCalendarDate<C> {
    pub fn from_raw(raw: CalendarDate) -> Result<Self, CalendarTypeError>;
    pub fn calendar_system() -> CalendarSystem;
    pub fn as_raw(&self) -> &CalendarDate;
    pub fn into_raw(self) -> CalendarDate;
    pub fn era(&self) -> Option<&Era>;
    pub fn year(&self) -> i32;
    pub fn month(&self) -> u8;
    pub fn day(&self) -> u8;
    pub fn days_in_month(&self) -> u8;
    pub fn add_months(&self, month_delta: i32) -> Result<Self, CalendarError>;
    pub fn to_calendar<T: CalendarKind>(&self) -> Result<TypedCalendarDate<T>, CalendarConversionError>;
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering>;
}

impl<C: DirectDayArithmetic> TypedCalendarDate<C> {
    pub fn add_days(&self, day_delta: i32) -> Result<Self, CalendarError>;
}

impl TypedCalendarDate<Gregorian> {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, CalendarError>;
}
```

The typed view is the preferred API when a component or state machine already knows its calendar statically.

#### 1.1.2 Calendar System Validation

The supported calendars and their public era behavior are:

| Calendar | BCP 47 | Public era model | Notes |
| --- | --- | --- | --- |
| `Iso8601` | `iso8601` | none | ISO slots only |
| `Gregorian` | `gregory` | `bc`, `ad` | distinct from `Iso8601` |
| `Buddhist` | `buddhist` | `be` | Gregorian-aligned month/day structure |
| `Japanese` | `japanese` | `meiji`, `taisho`, `showa`, `heisei`, `reiwa` | bounded modern eras |
| `Hebrew` | `hebrew` | `am` | leap-month calendar, month codes are significant |
| `IslamicCivil` | `islamic-civil` | `ah` | tabular civil pattern |
| `IslamicUmmAlQura` | `islamic-umalqura` | `ah` | Umm al-Qura |
| `Persian` | `persian` | `ap` | Solar Hijri |
| `Indian` | `indian` | `shaka` | Indian national calendar |
| `Chinese` | `chinese` | none | leap months use `MonthCode` |
| `Coptic` | `coptic` | `bce`, `ce` | inverse-era rollover supported |
| `Dangi` | `dangi` | none | leap months use `MonthCode` |
| `Ethiopic` | `ethiopic` | `aa`, `am` | ordered-era rollover supported |
| `EthiopicAmeteAlem` | `ethioaa` | `aa` | single-era public view |
| `Roc` | `roc` | `broc`, `roc` | inverse-era rollover supported |

Validation rules:

- `CalendarDate::new(...)` is strict. The requested era, year, month, day, and `month_code` must match the resolved projected date exactly.
- Callers may omit `era`; `CalendarSystem::default_era()` resolves the newest public era for era-bearing calendars.
- `MonthCode` is authoritative for leap-month calendars. A supplied `month_code` must agree with the resolved month ordinal.
- `CalendarSystem::supported_calendars()` is the adapter-facing metadata list used for runtime validation and UI configuration.
- Unsupported BCP 47 calendar identifiers return `None` from `CalendarSystem::from_bcp47(...)`; locale resolution then falls back to Gregorian.

#### 1.1.3 Parsing and Query Helpers

`ars-i18n` also exposes shared parse and query helpers:

```rust
pub trait DateValue: Clone {
    fn date_value(&self) -> CalendarDate;
    fn with_date_value(&self, date: CalendarDate) -> Self;
}

pub fn to_calendar_date_time(date: &CalendarDate, time: Option<Time>) -> CalendarDateTime;
pub fn to_zoned(date: &CalendarDate, time_zone: &TimeZoneId) -> Result<ZonedDateTime, CalendarError>;
pub fn to_zoned_date_time(
    date_time: &CalendarDateTime,
    time_zone: &TimeZoneId,
    disambiguation: Disambiguation,
) -> Result<ZonedDateTime, CalendarError>;

pub mod queries {
    use ars_i18n::{CalendarDate, CalendarError, DateValue, IcuProvider, Locale, TimeZoneId};

    pub fn is_same_day(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_same_month(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_same_year(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_equal_day(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_equal_month(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_equal_year(a: &impl DateValue, b: &impl DateValue) -> bool;
    pub fn is_today(date: &impl DateValue, time_zone: &TimeZoneId) -> bool;

    pub fn start_of_month<T: DateValue>(date: &T) -> T;
    pub fn end_of_month<T: DateValue>(date: &T) -> T;
    pub fn start_of_year<T: DateValue>(date: &T) -> T;
    pub fn end_of_year<T: DateValue>(date: &T) -> T;
    pub fn start_of_week<T: DateValue>(date: &T, locale: &Locale, provider: &dyn IcuProvider) -> T;
    pub fn end_of_week<T: DateValue>(date: &T, locale: &Locale, provider: &dyn IcuProvider) -> T;

    pub fn get_day_of_week(date: &CalendarDate, locale: &Locale, provider: &dyn IcuProvider) -> u8;
    pub fn get_weeks_in_month(date: &CalendarDate, locale: &Locale, provider: &dyn IcuProvider) -> u8;
    pub fn get_hours_in_day(date: &CalendarDate, time_zone: &TimeZoneId) -> Result<u8, CalendarError>;

    pub fn min_date<T: DateValue>(a: &T, b: &T) -> T;
    pub fn max_date<T: DateValue>(a: &T, b: &T) -> T;
    pub fn is_weekend(date: &CalendarDate, locale: &Locale, provider: &dyn IcuProvider) -> bool;
    pub fn is_weekday(date: &CalendarDate, locale: &Locale, provider: &dyn IcuProvider) -> bool;
}

pub mod parse {
    use ars_i18n::{CalendarDate, CalendarDateTime, CalendarError, CalendarSystem, DateTimeDuration, Time, TimeZoneId, ZonedDateTime};

    pub fn parse_date(input: &str) -> Result<CalendarDate, CalendarError>;
    pub fn parse_date_time(input: &str) -> Result<CalendarDateTime, CalendarError>;
    pub fn parse_time(input: &str) -> Result<Time, CalendarError>;
    pub fn parse_duration(input: &str) -> Result<DateTimeDuration, CalendarError>;
    pub fn parse_absolute(input: &str, time_zone: &TimeZoneId) -> Result<ZonedDateTime, CalendarError>;
    pub fn parse_absolute_to_local(input: &str) -> Result<ZonedDateTime, CalendarError>;
    pub fn parse_zoned_date_time(input: &str) -> Result<ZonedDateTime, CalendarError>;
    pub fn now(time_zone: Option<&TimeZoneId>) -> Result<ZonedDateTime, CalendarError>;
    pub fn today(time_zone: Option<&TimeZoneId>) -> Result<CalendarDate, CalendarError>;
    pub fn get_local_time_zone() -> Result<TimeZoneId, CalendarError>;
    pub fn set_local_time_zone_override(time_zone: TimeZoneId);
    pub fn reset_local_time_zone_override();
}
```

Query semantics:

- `is_same_*` compares display fields in the first argument's calendar and era. If the second argument uses a different calendar, it is converted into the first argument's calendar before comparing year, month, and day fields.
- `is_equal_*` requires the same calendar and era and compares the exposed display fields directly. Cross-calendar projections of the same ISO day are not equal.
- `start_of_week`, `end_of_week`, `get_weeks_in_month`, `is_weekday`, and `is_weekend` are locale-aware and therefore still require `IcuProvider`.
- `get_day_of_week` returns `0..=6`, where `0` is the locale's first day of week from provider-backed `WeekInfo`. When the locale carries a `u-fw-*` Unicode extension, that explicit first-day override takes precedence over the locale default.
- `get_hours_in_day` uses real zoned day-boundary math and returns `23`, `24`, or `25` when DST changes alter the local day length.
- `is_weekend` and `is_weekday` use locale weekend metadata rather than a hard-coded Saturday/Sunday rule.
- `parse_date` and `parse_date_time` preserve supported IXDTF calendar annotations rather than coercing annotated input to ISO.
- `parse_absolute` parses an ISO 8601 absolute timestamp with `Z` or an explicit offset and projects the instant into the supplied display time zone.
- `parse_absolute_to_local` resolves the local time zone through `get_local_time_zone()` and delegates to `parse_absolute`.
- `parse_zoned_date_time` remains the entry point for strings that already carry an explicit bracketed IANA zone identifier, and it rejects parsed calendar annotations that cannot be represented by `CalendarSystem`.
- `to_calendar_date_time` pairs a date with an explicit time or defaults to midnight when the time is omitted.
- `to_zoned` is the shorthand midnight conversion from a `CalendarDate` into a `ZonedDateTime`.
- `to_zoned_date_time` is the explicit local-to-zoned conversion entry point and applies the supplied time-zone disambiguation strategy.
- `parse_*` and `ZonedDateTime` helpers follow Temporal-style parsing rules, including calendar annotations and time-zone disambiguation where supported by the runtime engine.

#### 1.1.4 Date Ranges and Week Data

The remaining shared helpers are:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    pub start: CalendarDate,
    pub end: CalendarDate,
}

impl DateRange {
    pub fn new(start: CalendarDate, end: CalendarDate) -> Option<Self>;
    pub fn contains(&self, date: &CalendarDate) -> bool;
    pub fn to_iso8601(&self) -> String;
    pub fn normalized(first_date: CalendarDate, second_date: CalendarDate) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HourCycle {
    H11,
    H12,
    #[default]
    H23,
    H24,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WeekInfo {
    pub first_day: Weekday,
    pub weekend_start: Weekday,
    pub weekend_end: Weekday,
    pub minimal_days_in_first_week: u8,
}
```

`WeekInfo::for_locale(locale)` remains the fallback locale-week contract when runtime provider data is unavailable.

#### 1.1.5 Date Formatting

`ars-i18n` also exposes a backend-stable date/time formatting surface:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FormatLength {
    Full,
    Long,
    #[default]
    Medium,
    Short,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWidth {
    Narrow,
    Short,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericWidth {
    Numeric,
    TwoDigit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonthFormat {
    Numeric,
    TwoDigit,
    Narrow,
    Short,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeZoneNameFormat {
    Short,
    Long,
    ShortOffset,
    LongOffset,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateFormatterOptions {
    pub date_style: Option<FormatLength>,
    pub time_style: Option<FormatLength>,
    pub weekday: Option<TextWidth>,
    pub era: Option<TextWidth>,
    pub year: Option<NumericWidth>,
    pub month: Option<MonthFormat>,
    pub day: Option<NumericWidth>,
    pub hour: Option<NumericWidth>,
    pub minute: Option<NumericWidth>,
    pub second: Option<NumericWidth>,
    pub hour_cycle: Option<HourCycle>,
    pub time_zone: Option<TimeZoneId>,
    pub time_zone_name: Option<TimeZoneNameFormat>,
    pub calendar: Option<CalendarSystem>,
    pub numbering_system: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFormatterPartKind {
    Era,
    Year,
    Month,
    Day,
    Weekday,
    DayPeriod,
    Hour,
    Minute,
    Second,
    FractionalSecond,
    TimeZoneName,
    Literal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateFormatterPart {
    pub kind: DateFormatterPartKind,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangePartSource {
    Shared,
    Start,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRangeFormatterPart {
    pub kind: DateFormatterPartKind,
    pub value: String,
    pub source: DateRangePartSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDateFormatterOptions {
    pub locale: Locale,
    pub calendar: CalendarSystem,
    pub numbering_system: Option<String>,
    pub time_zone: Option<TimeZoneId>,
    pub hour_cycle: Option<HourCycle>,
    pub date_style: Option<FormatLength>,
    pub time_style: Option<FormatLength>,
}

pub struct DateFormatter { /* private fields */ }

impl DateFormatter {
    pub fn new(locale: &Locale, length: FormatLength) -> Self;
    pub fn new_with_options(locale: &Locale, options: DateFormatterOptions) -> Self;

    pub fn format(&self, date: &CalendarDate) -> String;
    pub fn format_date(&self, value: &CalendarDate) -> String;
    pub fn format_date_time(&self, value: &CalendarDateTime) -> String;
    pub fn format_time(&self, value: &Time) -> String;
    #[cfg(feature = "std")]
    pub fn format_zoned(&self, value: &ZonedDateTime) -> String;
    pub fn format_date_range(&self, start: &CalendarDate, end: &CalendarDate) -> String;
    pub fn format_date_time_range(
        &self,
        start: &CalendarDateTime,
        end: &CalendarDateTime,
    ) -> String;
    #[cfg(feature = "std")]
    pub fn format_zoned_range(&self, start: &ZonedDateTime, end: &ZonedDateTime) -> String;

    pub fn format_date_to_parts(&self, value: &CalendarDate) -> Vec<DateFormatterPart>;
    pub fn format_date_time_to_parts(&self, value: &CalendarDateTime) -> Vec<DateFormatterPart>;
    pub fn format_time_to_parts(&self, value: &Time) -> Vec<DateFormatterPart>;
    #[cfg(feature = "std")]
    pub fn format_zoned_to_parts(&self, value: &ZonedDateTime) -> Vec<DateFormatterPart>;
    pub fn format_date_range_to_parts(
        &self,
        start: &CalendarDate,
        end: &CalendarDate,
    ) -> Vec<DateRangeFormatterPart>;
    pub fn format_date_time_range_to_parts(
        &self,
        start: &CalendarDateTime,
        end: &CalendarDateTime,
    ) -> Vec<DateRangeFormatterPart>;
    #[cfg(feature = "std")]
    pub fn format_zoned_range_to_parts(
        &self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
    ) -> Vec<DateRangeFormatterPart>;

    pub fn resolved_options(&self) -> ResolvedDateFormatterOptions;
}
```

Formatter semantics:

- `new(locale, length)` keeps the legacy simple date-only formatting contract.
- `new_with_options(...)` is the canonical richer formatter constructor.
- `format_*_to_parts(...)` returns a normalized public part model shared across ICU4X and browser `Intl` backends.
- `format_*_range(...)` collapses identical leading and trailing segments into a single shared rendering and inserts a shared separator between differing middle segments.
- `format_*_range_to_parts(...)` exposes the same collapsed rendering with explicit `Shared`, `Start`, and `End` part-source metadata.
- `resolved_options()` reports the effective locale, calendar, numbering system, time zone, hour cycle, and style shortcuts after locale defaults and explicit overrides are applied.
- The `calendar` option projects date-like inputs into the resolved display calendar before field parts are produced.
- `format_zoned(...)` and `format_zoned_to_parts(...)` are available only when `std` is enabled.
