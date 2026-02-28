# ICU4X 2.x — Text Processing Reference

## 1. `icu_plurals` 2.1.1

### 1.1 `PluralRules`

**Constructors:**

```rust
// General (specify cardinal vs ordinal via options)
PluralRules::try_new(
    prefs: PluralRulesPreferences,  // locale!("en").into()
    options: PluralRulesOptions,
) -> Result<Self, DataError>

// Convenience
PluralRules::try_new_cardinal(prefs: PluralRulesPreferences) -> Result<Self, DataError>
PluralRules::try_new_ordinal(prefs: PluralRulesPreferences) -> Result<Self, DataError>
```

**Methods:**

```rust
rules.category_for(input: impl Into<PluralOperands>) -> PluralCategory
rules.categories() -> impl Iterator<Item = PluralCategory>
```

`category_for` accepts: all integer types (i8-i128, u8-u128, usize, isize), `Decimal`, and `PluralOperands`.

### 1.2 `PluralCategory`

```rust
pub enum PluralCategory {
    Zero = 0,
    One = 1,
    Two = 2,
    Few = 3,
    Many = 4,
    Other = 5,
}
```

### 1.3 `PluralOperands`

- **Not directly constructible** by fields (private fields)
- `From<T>` for all integer types
- `FromStr` for decimal strings: `"123.45".parse::<PluralOperands>()`
- `From<Decimal>` for fixed-decimal values
- **No float support** — floats can't carry trailing zeros needed for plural rules

---

## 2. `icu_decimal` 2.1.1

### 2.1 `DecimalFormatter` (NOT `FixedDecimalFormatter`)

**Constructor:**

```rust
DecimalFormatter::try_new(
    prefs: DecimalFormatterPreferences,  // locale!("en").into()
    options: DecimalFormatterOptions,
) -> Result<Self, DataError>
```

**Format methods:**

```rust
formatter.format(value: &Decimal) -> FormattedDecimal
formatter.format_to_string(value: &Decimal) -> String  // requires `alloc`
```

### 2.2 `DecimalFormatterOptions`

```rust
pub struct DecimalFormatterOptions {
    pub grouping_strategy: Option<GroupingStrategy>,  // thousand separators
}
```

### 2.3 Decimal type (from `fixed_decimal` 0.7.1)

`Decimal` is a type alias: `pub type Decimal = Signed<UnsignedDecimal>`

**Constructors:**

```rust
Decimal::from(42i64)                                         // from integer
Decimal::try_from_str("123.45") -> Result<Self, ParseError>  // from string
Decimal::try_from_f64(1.5, FloatPrecision::Floating) -> Result<Self, LimitError>  // requires `ryu`
```

**Key methods:**

```rust
decimal.multiply_pow10(delta: i16)              // shift decimal point
decimal.multiplied_pow10(delta: i16) -> Self    // consuming version
decimal.round(position: i16)                     // in-place rounding
decimal.rounded(position: i16) -> Self           // consuming version
decimal.ceil(position: i16)
decimal.floor(position: i16)
decimal.trunc(position: i16)
decimal.is_zero() -> bool
decimal.digit_at(magnitude: i16) -> u8
```

---

## 3. `icu_collator` 2.1.1

### 3.1 `Collator`

**Constructor — returns `CollatorBorrowed<'static>`, NOT `Collator`:**

```rust
Collator::try_new(
    prefs: CollatorPreferences,  // locale!("en").into()
    options: CollatorOptions,
) -> Result<CollatorBorrowed<'static>, DataError>
```

The owned `Collator` type exists but you get it via `CollatorBorrowed::static_to_owned()`.

### 3.2 `CollatorBorrowed` (the working type)

**Comparison methods:**

```rust
collator.compare(left: &str, right: &str) -> Ordering
collator.compare_utf8(left: &[u8], right: &[u8]) -> Ordering
collator.compare_utf16(left: &[u16], right: &[u16]) -> Ordering
```

**Sort keys:**

```rust
collator.write_sort_key_to(s: &str, sink: &mut S) -> Result
```

**Other:**

```rust
collator.resolved_options() -> ResolvedCollatorOptions
collator.static_to_owned() -> Collator  // only on CollatorBorrowed<'static>
```

### 3.3 `Collator` (owned) -> `CollatorBorrowed`

```rust
let owned: Collator = collator_borrowed.static_to_owned();
let borrowed: CollatorBorrowed = owned.as_borrowed();
borrowed.compare("a", "b");
```

### 3.4 `CollatorOptions`

```rust
pub struct CollatorOptions {
    pub strength: Option<Strength>,
    pub alternate_handling: Option<AlternateHandling>,
    pub case_level: Option<CaseLevel>,
}
```

### 3.5 `CollatorPreferences`

```rust
pub struct CollatorPreferences {
    pub case_first: Option<CollationCaseFirst>,
    pub numeric_ordering: Option<CollationNumericOrdering>,
}
```

### 3.6 `Strength`

```rust
pub enum Strength {
    Primary,     // base characters
    Secondary,   // + accents/diacritics
    Tertiary,    // + case
    Quaternary,  // + punctuation/symbols
}
```

---

## 4. `icu_segmenter` 2.1.2

All segmenters follow an **owned/borrowed** pattern. Constructors often return the borrowed type directly.

### 4.1 `GraphemeClusterSegmenter`

```rust
// Infallible, no locale needed
let segmenter = GraphemeClusterSegmenter::new();  // returns GraphemeClusterSegmenterBorrowed<'static>

// Segment
let breaks: Vec<usize> = segmenter.segment_str("hello").collect();
```

### 4.2 `WordSegmenter`

```rust
// Invariant (no locale-specific rules)
let seg = WordSegmenter::new_auto(Default::default());       // returns WordSegmenterBorrowed<'static>
let seg = WordSegmenter::new_lstm(Default::default());       // LSTM model
let seg = WordSegmenter::new_dictionary(Default::default()); // dictionary model

// With locale-specific rules (fallible)
let seg = WordSegmenter::try_new_auto(options)?;  // returns owned WordSegmenter
let borrowed = seg.as_borrowed();
let breaks: Vec<usize> = borrowed.segment_str("Hello world!").collect();
```

### 4.3 `SentenceSegmenter`

```rust
// Invariant
let seg = SentenceSegmenter::new(Default::default());  // returns SentenceSegmenterBorrowed<'static>

// With locale
let seg = SentenceSegmenter::try_new(options)?;
```

### 4.4 Iterator Pattern

All `segment_str()` / `segment_utf8()` / `segment_utf16()` methods return iterators yielding `usize` boundary positions (byte offsets for UTF-8, code unit offsets for UTF-16).

```rust
let segmenter = WordSegmenter::new_auto(Default::default());
let text = "Hello, world!";
let words: Vec<&str> = {
    let breaks: Vec<usize> = segmenter.segment_str(text).collect();
    breaks.windows(2).map(|w| &text[w[0]..w[1]]).collect()
};
```

---

## 5. `icu_experimental::relativetime` (NOT `icu_relativetime`)

### 5.1 `RelativeTimeFormatter`

Per-unit, per-length constructors:

```rust
use icu::experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterPreferences};

let fmt = RelativeTimeFormatter::try_new_long_second(
    locale!("en").into(),  // RelativeTimeFormatterPreferences
    Default::default(),    // RelativeTimeFormatterOptions
)?;

// Units: second, minute, hour, day, week, month, quarter, year
// Lengths: long, short, narrow
// Pattern: try_new_{length}_{unit}(prefs, options)
```

**Format method:**

```rust
fmt.format(value: Decimal) -> impl Writeable
```

Takes `Decimal` (from `fixed_decimal`), not `i64` or `f64`.
