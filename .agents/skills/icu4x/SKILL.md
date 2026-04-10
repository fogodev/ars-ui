---
name: icu4x
description: "ICU4X 2.x internationalization crate family reference (covers icu_calendar, icu_datetime, icu_plurals, icu_decimal, icu_collator, icu_segmenter, icu_locale, icu_provider, icu_experimental, fixed_decimal). Use when writing or reviewing any Rust code that uses ICU4X crates for i18n, l10n, date/time formatting, number formatting, plural rules, collation, text segmentation, or locale handling. Also use when the user mentions ICU4X, works on any .rs file that imports from icu::* or icu_*::*, or reviews spec files referencing ICU4X APIs. Even if the question seems simple, consult this skill — ICU4X had a major breaking redesign from 1.x to 2.x and training data is almost certainly wrong."
---

# ICU4X 2.x — Crate Family Reference

**Version:** 2.1.1 (latest stable as of 2026-03-18)
**Docs:** [docs.rs/icu](https://docs.rs/icu/2.1.1/icu/) | [github.com/unicode-org/icu4x](https://github.com/unicode-org/icu4x)

> This skill covers `icu` (umbrella), `icu_calendar`, `icu_datetime`, `icu_plurals`, `icu_decimal`, `icu_collator`, `icu_segmenter`, `icu_locale`, `icu_provider`, `icu_experimental`, and `fixed_decimal`.

## Crate Versions

```toml
[dependencies]
icu = "2.1"                    # umbrella — re-exports everything below
icu_calendar = "2.1"
icu_datetime = "2.1"
icu_plurals = "2.1"
icu_decimal = "2.1"
icu_collator = "2.1"
icu_segmenter = "2.1"
icu_locale = "2.1"             # REPLACES deprecated icu_locid
icu_provider = "2.1"
icu_experimental = "0.4"       # contains relativetime (replaces stale icu_relativetime 0.1.4)
fixed_decimal = "0.7"          # main type is Decimal, NOT FixedDecimal
```

**Deprecated crates (do NOT use):**

- `icu_locid` — replaced by `icu_locale`
- `icu_relativetime` — stuck at 0.1.4 (1.x era); use `icu_experimental::relativetime`

## Standard Import

There is **no** `icu::prelude`. Import from sub-modules:

```rust
use icu::locale::{locale, Locale};
use icu::calendar::{Date, Gregorian, Iso};
use icu::datetime::{DateTimeFormatter, fieldsets::YMD};
use icu::decimal::DecimalFormatter;
use icu::plurals::PluralRules;
use icu::collator::Collator;
```

## The 2.x Constructor Pattern

All formatters/services take **Preferences + Options**, not raw locales:

```rust
// Every formatter follows this pattern:
let formatter = DecimalFormatter::try_new(
    locale!("en").into(),      // Locale -> *Preferences via .into()
    Default::default(),        // *Options (usually defaultable)
)?;
```

Each component has its own `*Preferences` type (e.g., `PluralRulesPreferences`, `DecimalFormatterPreferences`, `CollatorPreferences`). Convert from `Locale` via `.into()`.

## Quick Patterns

### Calendar dates

```rust
use icu::calendar::{Date, Gregorian, Iso};

let date = Date::try_new_gregorian(2026, 3, 18)?;
let iso = Date::try_new_iso(2026, 3, 18)?;

// Query
let year: i32 = date.year().era_year_or_related_iso();  // NOT .number, NOT .era_year_or_extended()
let month: u8 = date.month().ordinal;                    // public field, OR .month_number()
let day: u8 = date.day_of_month().0;                     // DayOfMonth(pub u8) — access via .0
let weekday = date.day_of_week();                        // Weekday enum (Mon=1..Sun=7)

// Calendar conversion
let hebrew = date.to_calendar(icu::calendar::cal::Hebrew);
```

### YearInfo (enum, NOT a struct)

```rust
// YearInfo is an enum with Era and Cyclic variants:
pub enum YearInfo {
    Era(EraYear),
    Cyclic(CyclicYear),
}

// Key methods:
year_info.era_year_or_related_iso() -> i32   // displayable year number
year_info.extended_year() -> i32             // extended year
year_info.era() -> Option<EraYear>           // era info if present
year_info.cyclic() -> Option<CyclicYear>     // cyclic info if present
```

**There is NO `.number` field and NO `.era_year_or_extended()` method.**

### EraYear struct

```rust
pub struct EraYear {
    pub year: i32,
    pub extended_year: i32,
    pub era: TinyAsciiStr<16>,   // NOT a separate Era type
    pub era_index: Option<u8>,
    pub ambiguity: YearAmbiguity,
}
```

### Date/time formatting

```rust
use icu::datetime::{DateTimeFormatter, fieldsets::{YMD, YMDT, T}};

let fmt = DateTimeFormatter::try_new(locale!("en").into(), YMD::medium())?;
let date = "2026-03-18".parse::<Date<Iso>>()?;
let formatted = fmt.format(&date);  // "Mar 18, 2026"

// Date + time
let fmt = DateTimeFormatter::try_new(locale!("en").into(), YMD::medium().with_time_hm())?;
```

### Number formatting

```rust
use icu::decimal::DecimalFormatter;
use fixed_decimal::Decimal;

let fmt = DecimalFormatter::try_new(locale!("en").into(), Default::default())?;
let num = Decimal::from(1234567i64);
let formatted = fmt.format(&num);  // "1,234,567"
```

### Plural rules

```rust
use icu::plurals::{PluralRules, PluralCategory};

let rules = PluralRules::try_new_cardinal(locale!("en").into())?;
let category = rules.category_for(1u64);  // PluralCategory::One
let category = rules.category_for(42u64); // PluralCategory::Other
```

### Collation

```rust
use icu::collator::{Collator, CollatorOptions, Strength};

let mut opts = CollatorOptions::default();
opts.strength = Some(Strength::Secondary);
// try_new returns CollatorBorrowed<'static>, NOT Collator
let collator = Collator::try_new(locale!("en").into(), opts)?;
let ord = collator.compare("cafe", "CAFE");  // compare() is on CollatorBorrowed
```

### Text segmentation

```rust
use icu::segmenter::{GraphemeClusterSegmenter, WordSegmenter};

// Infallible, no locale needed
let segmenter = GraphemeClusterSegmenter::new();
let breaks = segmenter.segment_str("hello").collect::<Vec<_>>();

// Word segmenter (auto-selects algorithm)
let segmenter = WordSegmenter::new_auto(Default::default());
let breaks = segmenter.segment_str("Hello world!").collect::<Vec<_>>();
```

## Reference Files

Read the appropriate reference file for detailed APIs:

| Topic                    | File                            | When to read                                                                       |
| ------------------------ | ------------------------------- | ---------------------------------------------------------------------------------- |
| **Calendar**             | `references/calendar.md`        | Date types, YearInfo, MonthInfo, DayOfMonth, Weekday, calendar systems, conversion |
| **Date/Time Formatting** | `references/datetime.md`        | DateTimeFormatter, field sets, options, FixedCalendarDateTimeFormatter             |
| **Text Processing**      | `references/text-processing.md` | PluralRules, DecimalFormatter, Collator, Segmenter — full APIs                     |
| **Locale & Provider**    | `references/locale-provider.md` | Locale, locale!(), DataLocale, Preferences, data loading strategies                |

## Critical 1.x to 2.x Breaking Changes

| Aspect                  | 1.x (WRONG)                                  | 2.x (CORRECT)                                                      |
| ----------------------- | -------------------------------------------- | ------------------------------------------------------------------ |
| **Year access**         | `year().number`                              | `year().era_year_or_related_iso()`                                 |
| **YearInfo**            | Flat struct                                  | Enum: `Era(EraYear)` / `Cyclic(CyclicYear)`                        |
| **Era type**            | `Era` wrapper struct                         | `TinyAsciiStr<16>` field on `EraYear`                              |
| **Locale crate**        | `icu_locid`                                  | `icu_locale` (locid is deprecated)                                 |
| **Constructor args**    | `&Locale` or `&DataLocale`                   | `*Preferences` (via `.into()` from Locale)                         |
| **Decimal type**        | `FixedDecimal`                               | `Decimal` (alias for `Signed<UnsignedDecimal>`)                    |
| **Decimal formatter**   | `FixedDecimalFormatter`                      | `DecimalFormatter`                                                 |
| **Datetime formatters** | 7 separate types                             | 3 generic types with field sets                                    |
| **Collator::try_new**   | Returns `Collator`                           | Returns `CollatorBorrowed<'static>`                                |
| **Segmenter::new**      | Returns owned type                           | Returns borrowed type (e.g., `WordSegmenterBorrowed<'static>`)     |
| **Relative time**       | `icu_relativetime` crate                     | `icu_experimental::relativetime` module                            |
| **Islamic calendars**   | `IslamicCivil`, `IslamicObservational`, etc. | `Hijri<S>` with `new_umm_al_qura()`, `new_simulated_mecca()`, etc. |

## Common Pitfalls

1. **`year().number` does not exist.** Use `year().era_year_or_related_iso()`. There is also NO `era_year_or_extended()`.
2. **`YearInfo` is an enum**, not a struct. Don't access fields directly — use methods.
3. **`DayOfMonth` is a newtype** wrapping `u8`. You can't cast it directly: use `.0` to access the inner value.
4. **`MonthInfo.ordinal` is a public `u8` field**, but `.month_number()` also exists as a method.
5. **Constructors take Preferences, not locales.** Always convert: `locale!("en").into()`.
6. **`Collator::try_new()` returns `CollatorBorrowed`**, not `Collator`. The `compare()` method lives on `CollatorBorrowed`.
7. **Don't use `icu_locid`** — it's deprecated. Use `icu_locale`.
8. **Don't use `icu_relativetime`** — it's stuck at 1.x. Use `icu_experimental::relativetime`.
9. **`Decimal`, not `FixedDecimal`** — the type was renamed in `fixed_decimal` 0.7.
10. **No `icu::prelude`** — import from specific sub-modules.
