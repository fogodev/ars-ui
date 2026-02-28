# icu_datetime 2.1.1 — Date/Time Formatting Reference

## 1. Formatter Types

Only **3 types** in 2.x (down from 7 in 1.x):

| Type                                      | Purpose                                                                        |
| ----------------------------------------- | ------------------------------------------------------------------------------ |
| `DateTimeFormatter<FSet>`                 | Any calendar, any field set — the primary formatter                            |
| `FixedCalendarDateTimeFormatter<C, FSet>` | Compile-time calendar selection (smaller binary)                               |
| `NoCalendarFormatter<FSet>`               | Time and time zone only (alias for `FixedCalendarDateTimeFormatter<(), FSet>`) |

## 2. `DateTimeFormatter<FSet>`

### 2.1 Constructor

```rust
pub fn try_new(
    prefs: DateTimeFormatterPreferences,  // Locale.into()
    field_set_with_options: FSet,         // e.g., YMD::medium()
) -> Result<Self, DateTimeFormatterLoadError>
```

With explicit provider:

```rust
pub fn try_new_with_buffer_provider<P: BufferProvider>(
    provider: &P,
    prefs: DateTimeFormatterPreferences,
    field_set_with_options: FSet,
) -> Result<Self, DateTimeFormatterLoadError>
```

### 2.2 Format Methods

```rust
// Auto-converts calendar to match formatter:
formatter.format(&datetime) -> FormattedDateTime

// Requires same calendar (returns Result):
formatter.format_same_calendar(&datetime) -> Result<FormattedDateTime, MismatchedCalendarError>
```

### 2.3 Conversion

```rust
formatter.try_into_typed_formatter::<C>() -> Result<FixedCalendarDateTimeFormatter<C, FSet>, _>
formatter.calendar() -> Ref<'_, AnyCalendar>
```

---

## 3. Field Sets (`icu::datetime::fieldsets`)

Zero-sized structs that encode which fields to display:

### 3.1 Date field sets

| Set    | Fields                       | Example output (en, medium) |
| ------ | ---------------------------- | --------------------------- |
| `Y`    | year                         | "2026"                      |
| `M`    | month                        | "March"                     |
| `D`    | day                          | "18"                        |
| `E`    | weekday                      | "Wednesday"                 |
| `YM`   | year + month                 | "Mar 2026"                  |
| `YMD`  | year + month + day           | "Mar 18, 2026"              |
| `YMDE` | year + month + day + weekday | "Wed, Mar 18, 2026"         |
| `MD`   | month + day                  | "Mar 18"                    |
| `MDE`  | month + day + weekday        | "Wed, Mar 18"               |
| `DE`   | day + weekday                | "Wed 18"                    |

### 3.2 Combined date + time field sets

| Set     | Fields                              |
| ------- | ----------------------------------- |
| `DT`    | day + time                          |
| `ET`    | weekday + time                      |
| `MDT`   | month + day + time                  |
| `MDET`  | month + day + weekday + time        |
| `YMDT`  | year + month + day + time           |
| `YMDET` | year + month + day + weekday + time |
| `DET`   | day + weekday + time                |

### 3.3 Time field set

`T` — time only (use with `NoCalendarFormatter`)

### 3.4 Constructing field sets

```rust
// Length presets
YMD::long()
YMD::medium()
YMD::short()

// Builder pattern
YMD::medium()
    .with_alignment(alignment)
    .with_year_style(YearStyle::Full)

// Composing date + time
YMD::medium().with_time_hm()     // -> YMDT (hour:minute)
YMD::medium().with_time_hms()    // -> YMDT (hour:minute:second)

// Time standalone
T::hm()     // hours + minutes
T::hms()    // hours + minutes + seconds

// Adding time zone
YMD::medium().with_zone::<Z>(zone) // -> Combo<YMD, Z>
```

---

## 4. Options

```rust
pub enum Length { Long, Medium, Short }

pub enum TimePrecision {
    Hour, Minute, Second,
    Subsecond(SubsecondDigits),
    MinuteOptional,
}

pub enum YearStyle {
    Auto,     // show era/century only when ambiguous
    Full,     // always show century
    WithEra,  // always show both century and era
}
```

---

## 5. Input Types

```rust
use icu::calendar::Date;
use icu::time::{DateTime, Time};

// Time construction
let time = Time::try_new(15, 47, 50, 0)?;  // hour, minute, second, nanosecond
let midnight = Time::start_of_day();

// DateTime (from icu::time::DateTime)
let dt = DateTime { date, time };

// Parse from ISO string
let dt = "2024-07-17T16:01:17".parse::<DateTime<Iso>>()?;
```

---

## 6. Complete Example

```rust
use icu::locale::locale;
use icu::datetime::{DateTimeFormatter, fieldsets::YMD};
use icu::calendar::{Date, Iso};

// Format a date
let fmt = DateTimeFormatter::try_new(locale!("fr").into(), YMD::long())?;
let date = Date::try_new_iso(2026, 3, 18)?;
println!("{}", fmt.format(&date));  // "18 mars 2026"

// Format date + time
use icu::datetime::fieldsets::YMDT;
let fmt = DateTimeFormatter::try_new(
    locale!("ja").into(),
    YMD::medium().with_time_hm(),
)?;
```
