# ICU4X 2.x — Locale & Provider Reference

## 1. `icu_locale` 2.1.1 (replaces icu_locid)

### 1.1 Locale

```rust
pub struct Locale {
    pub id: LanguageIdentifier,
    pub extensions: Extensions,
}
```

**Constants:**

```rust
Locale::UNKNOWN  // the "und" locale
```

**Parsing:**

```rust
Locale::try_from_str("en-US") -> Result<Locale, ParseError>
"en-US".parse::<Locale>()  // via FromStr
```

### 1.2 `LanguageIdentifier`

```rust
pub struct LanguageIdentifier {
    pub language: Language,
    pub script: Option<Script>,
    pub region: Option<Region>,
    pub variants: Variants,
}
```

```rust
LanguageIdentifier::UNKNOWN
LanguageIdentifier::try_from_str("en") -> Result<Self, ParseError>
```

### 1.3 `locale!()` macro

Compile-time validated locale construction:

```rust
use icu::locale::{locale, Locale};

const EN_US: Locale = locale!("en-US");
const DE: Locale = locale!("de");
const JA_JP: Locale = locale!("ja-JP");
```

**Limitations:** single variant only, single Unicode extension keyword only. For complex locales, use runtime parsing.

There is also `langid!()` for `LanguageIdentifier`.

---

## 2. Preferences System (2.x pattern)

In ICU4X 2.x, formatters don't take locales directly. Instead, each formatter has its own `*Preferences` type.

### 2.1 `LocalePreferences` (base type)

```rust
// Construction
LocalePreferences::default()
From<&Locale> for LocalePreferences
From<&LanguageIdentifier> for LocalePreferences

// Methods
prefs.language() -> Language
prefs.region() -> Option<Region>
prefs.extend(&mut self, other: LocalePreferences)  // merge
```

Implements `Copy`.

### 2.2 Per-formatter Preferences

Each formatter defines its own preferences type containing `locale_preferences` plus component-specific fields:

```rust
// These all exist:
PluralRulesPreferences
DecimalFormatterPreferences
DateTimeFormatterPreferences
CollatorPreferences
RelativeTimeFormatterPreferences
// etc.

// All implement From<Locale> and From<&Locale>:
let prefs: DecimalFormatterPreferences = locale!("en").into();
```

### 2.3 Layered composition

```rust
let mut prefs = LocalePreferences::default();
prefs.extend(locale_prefs);   // base locale
prefs.extend(os_prefs);       // OS overrides
prefs.extend(app_prefs);      // app-level overrides
// Non-None values override
```

---

## 3. `DataLocale`

Optimized locale type for data pipeline lookups. Strips most extensions, keeping only `-u-sd` keywords.

```rust
// Construction
DataLocale::from(locale)
DataLocale::from(&locale)
DataLocale::try_from_str("en-US") -> Result<DataLocale, ParseError>

// Conversion
data_locale.into_locale() -> Locale

// Query
data_locale.is_unknown() -> bool
```

Implements `Copy`.

---

## 4. `icu_provider` 2.1.1 — Data Loading

### 4.1 Three strategies

1. **Compiled data (default)** — CLDR data baked into binary via `compiled_data` feature. Zero-cost, no runtime I/O.

2. **BufferProvider** — loads serialized bytes (postcard/JSON) at runtime. Use `icu_provider_blob` crate.

3. **`DataProvider<M>`** — type-safe trait, unstable API.

### 4.2 Constructor pattern

Every formatter supports all three:

```rust
// 1. Compiled data (default, no provider arg)
Formatter::try_new(prefs, options)

// 2. BufferProvider (runtime blob)
Formatter::try_new_with_buffer_provider(&provider, prefs, options)

// 3. Unstable (explicit DataProvider)
Formatter::try_new_unstable(&provider, prefs, options)
```

### 4.3 Key traits

```rust
DataProvider<M: DataMarker>           // primary trait
DynamicDataProvider<M>                // runtime-parameterized
BufferProvider                        // alias for DynamicDataProvider<BufferMarker>
DryDataProvider                       // checks if data can be loaded
IterableDataProvider                  // iterates supported identifiers
```

### 4.4 Key types

```rust
DataRequest       // locale selectors
DataResponse<M>   // response container
DataPayload<M>    // zero-copy data container
DataError         // error type
DataMarkerInfo    // runtime marker representation
```

### 4.5 `icu_provider_blob` 2.1.1

For runtime data loading:

```rust
use icu_provider_blob::BlobDataProvider;

let blob = std::fs::read("icu_data.postcard")?;
let provider = BlobDataProvider::try_new_from_blob(blob.into())?;

// Pass to any formatter
let fmt = DecimalFormatter::try_new_with_buffer_provider(
    &provider,
    locale!("en").into(),
    Default::default(),
)?;
```

### 4.6 Baked data

Baked data is built into each component crate's `compiled_data` feature (backed by `icu_provider`'s `baked` module). There is no separate `icu_provider_baked` crate.

---

## 5. Recommended `Cargo.toml`

```toml
[dependencies]
# Use umbrella crate for convenience:
icu = "2.1"

# Or individual crates for smaller builds:
icu_locale = { version = "2.1", features = ["serde"] }
icu_calendar = "2.1"
icu_datetime = "2.1"
icu_plurals = "2.1"
icu_decimal = "2.1"
icu_collator = "2.1"
icu_segmenter = "2.1"
icu_provider = "2.1"
icu_experimental = "0.4"  # for relativetime
fixed_decimal = "0.7"

# For runtime data loading (optional):
icu_provider_blob = "2.1"
```
