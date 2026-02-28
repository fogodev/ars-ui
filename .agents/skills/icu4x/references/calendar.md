# icu_calendar 2.1.1 — Calendar Types Reference

## 1. `Date<A: AsCalendar>`

The core date type, generic over calendar system.

### 1.1 Constructors

```rust
// Specific calendars
Date::try_new_gregorian(year: i32, month: u8, day: u8) -> Result<Date<Gregorian>, RangeError>
Date::try_new_iso(year: i32, month: u8, day: u8) -> Result<Date<Iso>, RangeError>

// Any calendar via codes
Date::try_new_from_codes(
    era: Option<&str>,     // None for calendars without eras
    year: i32,
    month_code: MonthCode, // e.g., MonthCode::new_normal(3) for March
    day: u8,
    calendar: A,
) -> Result<Self, DateError>

// From ISO date
Date::new_from_iso(iso: Date<Iso>, calendar: A) -> Self

// From rata die
Date::from_rata_die(rd: RataDie, calendar: A) -> Self
```

### 1.2 Query Methods

```rust
date.year() -> YearInfo              // enum! See below
date.month() -> MonthInfo            // struct with public fields
date.day_of_month() -> DayOfMonth    // newtype: DayOfMonth(pub u8)
date.day_of_week() -> Weekday        // enum: Monday=1..Sunday=7
date.day_of_year() -> DayOfYear      // newtype
date.extended_year() -> i32          // extended year (always available)
date.era_year() -> EraYear           // era-based year info
date.is_in_leap_year() -> bool
date.months_in_year() -> u8
date.days_in_year() -> u16
date.days_in_month() -> u8
```

### 1.3 Conversion

```rust
date.to_calendar(calendar) -> Date<A2>   // convert between calendar systems
date.to_iso() -> Date<Iso>               // convert to ISO
date.to_any() -> Date<AnyCalendar>       // convert to runtime-selected calendar
date.to_rata_die() -> RataDie            // convert to rata die
```

### 1.4 Arithmetic

```rust
date.try_add_with_options(&mut self, duration: DateDuration, options: DateAddOptions) -> Result<(), DateError>
date.try_added_with_options(self, duration: DateDuration, options: DateAddOptions) -> Result<Self, DateError>
date.try_until_with_options(other: &Date<B>, options: DateDifferenceOptions) -> Result<DateDuration, DifferenceError>
```

---

## 2. `YearInfo` — ENUM (not a struct!)

```rust
#[non_exhaustive]
pub enum YearInfo {
    Era(EraYear),      // most calendar systems
    Cyclic(CyclicYear), // Chinese, Dangi
}
```

### 2.1 Methods

| Method                      | Returns              | Description                                                                     |
| --------------------------- | -------------------- | ------------------------------------------------------------------------------- |
| `era_year_or_related_iso()` | `i32`                | Displayable year number. For era-based: era year. For cyclic: related ISO year. |
| `extended_year()`           | `i32`                | Extended year value (always available)                                          |
| `era()`                     | `Option<EraYear>`    | Extracts era info if Era variant                                                |
| `cyclic()`                  | `Option<CyclicYear>` | Extracts cyclic info if Cyclic variant                                          |

**DOES NOT EXIST:** `.number`, `.era_year_or_extended()`

---

## 3. `EraYear`

```rust
#[non_exhaustive]
pub struct EraYear {
    pub year: i32,                    // numeric year within the era
    pub extended_year: i32,           // extended year
    pub era: TinyAsciiStr<16>,        // CLDR era code (NOT a separate Era type)
    pub era_index: Option<u8>,        // era index for small-set calendars
    pub ambiguity: YearAmbiguity,     // whether era/year is ambiguous
}
```

The `era` field is `TinyAsciiStr<16>`, not a dedicated `Era` struct. Access the string with `.as_str()`.

---

## 4. `MonthInfo`

```rust
#[non_exhaustive]
pub struct MonthInfo {
    pub ordinal: u8,                    // 1-based month number (public field)
    pub standard_code: MonthCode,       // round-trips through constructors
    pub formatting_code: MonthCode,     // for formatting (may differ, e.g., Hebrew)
}
```

### 4.1 Methods

| Method           | Returns | Description                   |
| ---------------- | ------- | ----------------------------- |
| `month_number()` | `u8`    | Same as `ordinal`             |
| `is_leap()`      | `bool`  | Whether month is a leap month |

Both `ordinal` (field) and `month_number()` (method) are valid. No `.month_code()` method — use `.standard_code` or `.formatting_code` fields directly.

---

## 5. `MonthCode`

```rust
pub struct MonthCode(pub TinyAsciiStr<4>);  // e.g., "M01", "M03L" (leap)
```

### 5.1 Constructors

```rust
MonthCode::new_normal(number: u8) -> Option<Self>  // "Mxx" format
MonthCode::new_leap(number: u8) -> Option<Self>     // "MxxL" format
```

---

## 6. `DayOfMonth`

```rust
pub struct DayOfMonth(pub u8);  // newtype wrapping u8
```

Access inner value with `.0`:

```rust
let day_num: u8 = date.day_of_month().0;
```

Implements `Copy`, `Clone`, `Debug`, `PartialEq`.

---

## 7. `Weekday`

```rust
#[repr(i8)]
pub enum Weekday {
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}
```

ISO-8601 numbering. There is **no separate `IsoWeekday`** type in 2.x.

### 7.1 Methods

```rust
Weekday::from_days_since_sunday(input: isize) -> Self
```

---

## 8. Calendar Systems

### 8.1 Concrete types (in `icu::calendar::cal`)

`Gregorian`, `Iso`, `Japanese`, `JapaneseExtended`, `Buddhist`, `Hebrew`, `Indian`, `Coptic`, `Ethiopian`, `Persian`, `Roc`, `Julian`

### 8.2 Hijri (replaces Islamic\*)

```rust
Hijri::new_umm_al_qura()        // official Saudi calculation
Hijri::new_simulated_mecca()     // astronomical simulation
Hijri::new_tabular(leap_years, epoch) // mathematical calculation
```

Old names (`IslamicCivil`, `IslamicObservational`, `IslamicTabular`, `IslamicUmmAlQura`) no longer exist.

### 8.3 Runtime calendar selection

```rust
pub enum AnyCalendarKind {
    Buddhist, Chinese, Coptic, Dangi, Ethiopian, EthiopianAmeteAlem,
    Gregorian, Hebrew, Indian, HijriTabularTypeIIFriday,
    HijriSimulatedMecca, HijriTabularTypeIIThursday, HijriUmmAlQura,
    Iso, Japanese, JapaneseExtended, Persian, Roc,
}
```

`AnyCalendar` is the runtime-selected calendar enum. There is **no `CalendarSystem` type** — each calendar is its own struct, unified by the `Calendar` trait.

### 8.4 Deprecated aliases

`Chinese` → use `ChineseTraditional`
`Dangi` → use `KoreanTraditional`
