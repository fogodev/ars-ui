# Internationalization Specification (`ars-i18n`)

## 1. Overview

The `ars-i18n` crate provides complete internationalization support for ars-ui components. It wraps [ICU4X](https://github.com/unicode-org/icu4x) — the Rust-native Unicode internationalization library — to provide:

- **Locale identification** (BCP 47 language tags)
- **RTL/BiDi support** (Arabic, Hebrew, Persian, Urdu, etc.)
- **Number formatting/parsing** (decimals, currency, percent, units)
- **Date/time formatting** (multiple calendar systems, timezones)
- **Plural and ordinal rules** (localized "1 item" vs "2 items")
- **Collation** (locale-aware string sorting)
- **Translatable component messages** (default English, user-overridable)

`ars-i18n` is `no_std` compatible (requires `alloc`). The optional `std` feature enables `Mutex`-based caching and server-side utilities (e.g., `locale_from_accept_language`). ICU4X data is compiled in via the `compiled_data` feature to minimize WASM binary size.

### 1.1 ICU4X vs Browser Intl API

`ars-i18n` supports two formatting backends via feature flags:

- **`icu4x`** (default): Rust-native ICU4X implementation. Required for SSR, desktop, and non-browser environments. Adds ~100-500KB to WASM binary.
- **`web-intl`**: Delegates to the browser's `Intl` API via `js-sys`. Zero binary size overhead. WASM client only.

```rust
#[cfg(feature = "icu4x")]
pub type DefaultNumberFormatter = icu4x::Icu4xNumberFormatter;
#[cfg(feature = "web-intl")]
pub type DefaultNumberFormatter = web_intl::JsIntlNumberFormatter;
```

Both backends implement the same formatter traits. Components are backend-agnostic.

#### 1.1.1 Number Formatter Context Propagation

Numeric components (NumberInput, Slider, RangeSlider, Progress, Meter) MUST resolve their formatting locale through the following inheritance chain:

1. **Component prop**: `locale: Option<Locale>` — highest priority
2. **FormContext**: If the component is inside a Form, inherit `FormContext.locale`
3. **ArsProvider**: Inherit from the nearest ancestor `ArsProvider` context via `use_locale()` hook
4. **Fallback**: `Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")` — always available

All numeric components MUST accept a `number_formatter: Option<NumberFormatter>` prop. If not provided, a default `NumberFormatter` is constructed from the resolved locale.

```rust
/// Trait for locale-aware number formatting and parsing.
pub trait NumberFormatter {
    /// Format a numeric value for display (e.g., 1234.5 -> "1,234.5" in en-US, "1.234,5" in de-DE).
    fn format(&self, value: f64) -> String;
    /// Parse a locale-formatted string back to a number. Accepts both canonical ("1.5") and
    /// locale-specific ("1,5" in de-DE) formats. Returns None if unparseable.
    fn parse(&self, input: &str) -> Option<f64>;
    /// Return the decimal separator for the current locale (e.g., '.' for en-US, ',' for de-DE).
    fn decimal_separator(&self) -> char;
    /// Return the grouping separator for the current locale (e.g., ',' for en-US, '.' for de-DE).
    fn grouping_separator(&self) -> Option<char>;
}
```

---

## 2. Locale System

### 2.1 The Locale Type

````rust
use icu::locale::{Locale as IcuLocale, locale};

/// A BCP 47 locale identifier.
///
/// Wraps ICU4X's Locale type with ars-ui-specific helpers.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale(IcuLocale);

impl Locale {
    /// Parse from a BCP 47 string.
    ///
    /// # Examples
    /// ```
    /// let en_us = Locale::parse("en-US").expect("valid locale");
    /// let ar = Locale::parse("ar").expect("valid locale");
    /// let ja = Locale::parse("ja-JP-u-ca-japanese").expect("valid locale"); // Japanese calendar
    /// ```
    pub fn parse(s: &str) -> Result<Self, LocaleParseError> {
        Ok(Self(s.parse::<IcuLocale>().map_err(LocaleParseError)?))
    }

    /// Create from a known language ID.
    /// Note: This cannot be `const fn` because `Default::default()` for ICU4X
    /// `Extensions` is not a const fn. Use `Locale::parse()` for runtime construction
    /// or define common locales as lazy statics.
    pub fn from_langid(langid: icu::locale::LanguageIdentifier) -> Self {
        Self(IcuLocale { id: langid, extensions: Default::default() })
    }

    /// Returns the text direction for this locale.
    pub fn direction(&self) -> Direction {
        if RTL_SCRIPTS.contains(&self.script_or_default()) {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    /// Returns true if this locale uses right-to-left text.
    pub fn is_rtl(&self) -> bool {
        self.direction() == Direction::Rtl
    }

    /// BCP 47 string representation.
    pub fn to_bcp47(&self) -> String {
        self.0.to_string()
    }

    /// Language subtag (e.g., "en", "ar", "zh").
    pub fn language(&self) -> &str {
        self.0.id.language.as_str()
    }

    /// Script subtag (e.g., "Latn", "Arab", "Hans"), if present.
    pub fn script(&self) -> Option<&str> {
        self.0.id.script.as_ref().map(|s| s.as_str())
    }

    /// Region subtag (e.g., "US", "DE", "JP"), if present.
    pub fn region(&self) -> Option<&str> {
        self.0.id.region.as_ref().map(|r| r.as_str())
    }

    /// Get the calendar system extension from the locale (u-ca-*).
    pub fn calendar_extension(&self) -> Option<&str> {
        self.0.extensions.unicode.keywords
            .get(&icu::locale::extensions::unicode::key!("ca"))
            .and_then(|v| v.as_single_subtag().map(|s| s.as_str()))
    }

    /// Get the first day of week from the locale (u-fw-*), if specified.
    pub fn first_day_of_week_extension(&self) -> Option<Weekday> {
        self.0.extensions.unicode.keywords
            .get(&icu::locale::extensions::unicode::key!("fw"))
            .and_then(|v| v.as_single_subtag())
            .and_then(|s| Weekday::from_icu_str(s.as_str()))
    }

    /// Convert to an ICU4X DataLocale for provider API calls.
    pub fn to_data_locale(&self) -> icu_provider::DataLocale {
        (&self.0).into()
    }

    fn script_or_default(&self) -> &str {
        self.script().unwrap_or_else(|| {
            // Infer script from language for common cases
            match self.language() {
                "ar" | "fa" | "ur" | "ps" | "ug" | "sd" => "Arab",
                "he" | "yi" => "Hebr",
                "dv" => "Thaa",
                "ks" => "Arab",    // Kashmiri (Perso-Arabic script)
                "nqo" => "Nkoo",   // N'Ko script
                "pa" if self.region() == Some("PK") => "Arab", // Punjabi in Pakistan (Shahmukhi)
                "ku" if self.region() == Some("IQ") => "Arab",
                _ => "Latn",
            }
        })
    }
}

/// Scripts that use right-to-left text direction.
const RTL_SCRIPTS: &[&str] = &[
    "Arab", // Arabic
    "Hebr", // Hebrew
    "Thaa", // Thaana (Maldivian)
    "Syrc", // Syriac
    "Tfng", // Tifinagh
    "Adlm", // Adlam (Fula)
    "Rohg", // Hanifi Rohingya
    "Mand", // Mandaic
    "Nbat", // Nabataean
    "Palm", // Palmyrene
    "Nkoo", // N'Ko (Manding languages)
    "Samr", // Samaritan
];
````

#### 2.1.1 BiDi Text Isolation

When interpolating user-provided text into messages, labels, or ARIA attributes, the interpolated text MUST be wrapped in Unicode bidirectional isolates to prevent BiDi reordering of the surrounding text.

```rust
/// Wrap text in Unicode bidirectional isolates appropriate for its direction.
///
/// - LTR text in an RTL context: U+2066 (LRI) ... U+2069 (PDI)
/// - RTL text in an LTR context: U+2067 (RLI) ... U+2069 (PDI)
/// - Unknown direction: U+2068 (FSI) ... U+2069 (PDI) — let the Unicode algorithm decide
```

#### 2.1.2 Emoji and Pictographic Content in BiDi Isolation

Emoji and pictographic characters require special handling within BiDi isolation contexts:

- **BiDi class ON (Other Neutral):** Emoji characters are classified as BiDi class ON (Other Neutral). They do not have inherent directionality and will adopt the direction of their surrounding context. This means emoji within isolated text segments inherit the isolation's direction correctly without additional markup.
- **ZWJ sequences as atomic units:** Zero Width Joiner (U+200D) sequences (e.g., family emoji 👨‍👩‍👧‍👦, skin tone + profession combos) MUST be treated as atomic units. Isolation boundaries (LRI/RLI/FSI ... PDI) MUST NOT be inserted within a ZWJ sequence. Implementations MUST use Unicode grapheme cluster segmentation (UAX #29) to identify cluster boundaries before inserting isolation marks.
- **Variation selectors stay attached:** Variation selectors U+FE0E (text presentation) and U+FE0F (emoji presentation) MUST remain attached to their base character. An isolation boundary MUST NOT separate a base character from its trailing variation selector.
- **Screen reader CLDR short names:** Screen readers announce emoji using CLDR short names (e.g., "grinning face" for 😀). BiDi isolation marks (LRI/RLI/PDI) are invisible formatting characters and do NOT interfere with CLDR short name lookup or announcement — no special handling is needed on the accessibility side.
- **Grapheme cluster integrity (CAUTION):** Never split grapheme clusters across isolation boundaries. Before inserting any isolation mark, use Unicode segmentation (UAX #29 extended grapheme clusters) to verify the insertion point is at a cluster boundary. Splitting a cluster can cause rendering artifacts (orphaned combining marks, broken emoji sequences) and accessibility failures (partial CLDR names).

#### 2.1.3 RTL Mixed Content in Form Labels

Labels containing user input MUST isolate the user-provided text with BiDi marks to prevent reordering of the surrounding label text. This is especially critical in RTL form layouts:

- **BiDi isolation in labels:** Wrap user-input segments within form labels using `isolate_text_safe()` (see above). Example: an Arabic label `"اسم: {isolate(user_name)}"` where `user_name` may be LTR English text.
- **RTL form layout:** In RTL locales, the label appears on the **left** side of the input (visually — this is the inline-end side in RTL). CSS `direction: rtl` on the form container automatically handles this via the browser's bidirectional layout algorithm.
- **CSS direction on container:** Set `dir="rtl"` on the form root element (or the nearest `ArsProvider` ancestor) so that all descendant form controls inherit the correct text direction and layout flow. Do NOT set `direction: rtl` on individual label elements — use the container-level attribute.
- **Mixed-direction form fields:** When a form contains both RTL labels and LTR input values (e.g., email addresses, URLs), ensure the `<input>` element has explicit `dir="ltr"` to prevent the input text from rendering in RTL order.

**Mandate**: All `MessageFn` closures that interpolate dynamic values (user input, item names, counts) MUST call `isolate_text_safe()` around the interpolated segment. This applies to:

- Form validation error messages (e.g., `"Field {isolate(field_name)} is required"`)
- Combobox deletion labels (e.g., `"Delete {isolate(item_name)}"`)
- Live region announcements containing user-provided text
- ARIA label interpolation in all components

Failure to isolate can cause entire sentences to reverse direction when a single interpolated value has opposite directionality.

```rust
/// Common pre-defined locales.
///
/// These use `unwrap_or_else` with a fallback to `en-US` so that a corrupted
/// or future-incompatible ICU4X locale string never causes a panic at runtime.
pub mod locales {
    use super::Locale;

    /// Fallback used when a locale string unexpectedly fails to parse.
    fn fallback() -> Locale {
        // "en-US" is guaranteed valid — if even this fails, something is
        // catastrophically wrong with the ICU4X data provider.
        Locale::parse("en-US").expect("en-US must always be a valid locale")
    }

    pub fn en_us() -> Locale { Locale::parse("en-US").unwrap_or_else(|_| fallback()) }
    pub fn en_gb() -> Locale { Locale::parse("en-GB").unwrap_or_else(|_| fallback()) }
    pub fn ar() -> Locale { Locale::parse("ar").unwrap_or_else(|_| fallback()) }
    pub fn ar_sa() -> Locale { Locale::parse("ar-SA").unwrap_or_else(|_| fallback()) }
    pub fn he() -> Locale { Locale::parse("he").unwrap_or_else(|_| fallback()) }
    pub fn fa() -> Locale { Locale::parse("fa").unwrap_or_else(|_| fallback()) }
    pub fn de() -> Locale { Locale::parse("de").unwrap_or_else(|_| fallback()) }
    pub fn fr() -> Locale { Locale::parse("fr-FR").unwrap_or_else(|_| fallback()) }
    pub fn ja() -> Locale { Locale::parse("ja").unwrap_or_else(|_| fallback()) }
    pub fn zh_hans() -> Locale { Locale::parse("zh-Hans").unwrap_or_else(|_| fallback()) }
    pub fn ko() -> Locale { Locale::parse("ko").unwrap_or_else(|_| fallback()) }
}
```

#### 2.1.4 Locale Fallback Chain for Unsupported Locales

When a requested locale is not directly supported, the system applies the following fallback chain:

1. **Full locale** — e.g., `zh-Hant-TW` (exact match)
2. **Drop region** — e.g., `zh-Hant` (strip region subtag per RFC 4647 truncation)
3. **Drop script** — e.g., `zh` (strip script subtag — language only)
4. **English fallback** — `en` (ultimate fallback; always available)

**Per-feature fallback differences:** The fallback chain may resolve differently for different ICU4X features. For example, date formatting may match `pt-BR` directly (because CLDR has `pt-BR` date data), while collation may fall back to `pt` (because collation rules are shared across Portuguese variants). Each ICU4X service (date formatting, collation, number formatting, plural rules) performs its own locale negotiation against its available data. Consumers should not assume that all features resolve to the same locale in the chain.

### 2.2 Locale Stack (Fallback Chain)

```rust
/// A locale with a fallback chain. Used for message lookup.
///
/// Example: ["pt-BR", "pt", "en"] — Brazilian Portuguese → Portuguese → English
#[derive(Clone, Debug)]
pub struct LocaleStack {
    locales: Vec<Locale>,
}

impl LocaleStack {
    pub fn new(primary: Locale) -> Self {
        let mut locales = vec![primary.clone()];

        // Auto-generate fallbacks following BCP 47 truncation:
        // zh-Hant-TW → zh-Hant → zh
        // pt-BR → pt
        // en-US → en
        //
        // The script subtag level is critical for CJK locales where
        // zh-Hant (Traditional Chinese) and zh-Hans (Simplified Chinese)
        // are not interchangeable.
        if primary.region().is_some() && primary.script().is_some() {
            // Full tag has both script and region: add lang+script level
            let lang_script = format!("{}-{}", primary.language(), primary.script().expect("script checked above"));
            if let Ok(ls) = Locale::parse(&lang_script) {
                if ls != primary {
                    locales.push(ls);
                }
            }
        }
        if primary.region().is_some() || primary.script().is_some() {
            let lang_only = Locale::parse(primary.language()).ok();
            if let Some(lo) = lang_only {
                if !locales.contains(&lo) {
                    locales.push(lo);
                }
            }
        }

        Self { locales }
    }

    pub fn with_fallback(mut self, fallback: Locale) -> Self {
        self.locales.push(fallback);
        self
    }

    pub fn primary(&self) -> &Locale {
        &self.locales[0]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Locale> {
        self.locales.iter()
    }

    /// Find the first locale matching a predicate (for message bundle lookup).
    pub fn find<F: Fn(&Locale) -> bool>(&self, f: F) -> Option<&Locale> {
        self.locales.iter().find(|l| f(l))
    }
}
```

### 2.3 Locale Provider Trait

```rust
/// Trait for non-reactive locale access (used in core tests and static contexts).
/// Framework adapters use ArsContext with reactive signals instead.
pub trait LocaleProvider: Clone + 'static {
    /// Get the current locale.
    fn locale(&self) -> &Locale;

    /// Get the current direction (derived from locale unless overridden).
    fn direction(&self) -> Direction {
        self.locale().direction()
    }
}

/// A static locale provider (no reactivity).
#[derive(Clone)]
pub struct StaticLocaleProvider {
    locale: Locale,
    direction_override: Option<Direction>,
}

impl StaticLocaleProvider {
    pub fn new(locale: Locale) -> Self {
        Self { locale, direction_override: None }
    }

    pub fn with_direction(mut self, dir: Direction) -> Self {
        self.direction_override = Some(dir);
        self
    }
}

impl LocaleProvider for StaticLocaleProvider {
    fn locale(&self) -> &Locale { &self.locale }

    fn direction(&self) -> Direction {
        self.direction_override.unwrap_or_else(|| self.locale.direction())
    }
}
```

#### 2.3.1 Canonical Per-Component Locale Inheritance Pattern

Locale, messages, and ICU provider are **environment context** resolved by the adapter
layer — not by core component code. Core `Props` structs MUST NOT contain `locale`,
`messages`, or `icu_provider` fields. The adapter reads these from `ArsProvider`
context and passes them to core code via the `Env` struct and `Messages` parameter
(see `01-architecture.md` §2.1 for the `Env` definition).

**Missing provider warning:** All adapter context hooks (`use_locale()`, `use_icu_provider()`,
`use_style_strategy()`) emit a debug-mode warning when no `ArsProvider` is found.
This helps developers catch missing provider setup during development. The warning
is compiled out in release builds.

```rust
/// Emit a debug-mode warning when ArsProvider context is missing.
/// Compiled out in release builds (zero-cost).
#[cfg(debug_assertions)]
fn warn_missing_provider(hook_name: &str) {
    eprintln!(
        "[ars-ui] {hook_name}: No ArsProvider found in the component tree. \
         Falling back to defaults."
    );
}

#[cfg(not(debug_assertions))]
fn warn_missing_provider(_hook_name: &str) {}
```

The adapter resolves locale through a three-level chain:

1. **Adapter prop override** — The adapter-level component (Leptos/Dioxus) accepts `locale: Option<Locale>`. If provided, the adapter uses this locale directly.
2. **ArsProvider inheritance** — If the adapter prop is `None`, the adapter calls `use_locale()` to read the locale from the nearest `ArsProvider` in the component tree.
3. **Ultimate fallback** — If no `ArsProvider` is found, the fallback is `Locale::parse("en-US").expect("en-US is always valid")`.

```rust
// resolve_locale() — an ADAPTER-ONLY utility (defined in 08-adapter-leptos.md / 09-adapter-dioxus.md).
// NOT available in core crates. Core code receives a fully-resolved Locale via Env.
fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| use_locale())  // adapter hook, reads from ArsProvider
}

// use_locale() implementation (in adapter layer):
fn use_locale() -> Locale {
    use_context::<ArsContext>()
        .map(|ctx| ctx.locale().clone())
        .unwrap_or_else(|| {
            warn_missing_provider("use_locale");
            Locale::parse("en-US").expect("en-US is a valid BCP-47 tag")
        })
}

// Adapter usage — resolve env before passing to core:
let locale = resolve_locale(adapter_props.locale.as_ref());
let icu_provider = use_icu_provider();
let env = Env { locale, icu_provider };
let messages = resolve_messages::<dialog::Messages>(adapter_props.messages.as_ref(), &registries, &env.locale);
let service = Service::new(core_props, env, messages);
```

Core component code receives a fully-resolved `Env` — it never calls `use_locale()`
or `use_context()` directly. This ensures framework-agnostic crates (`ars-core`,
`ars-interactions`) have no dependency on adapter hooks.

#### 2.3.2 Canonical ICU Provider Resolution

Date-time components need locale-aware calendar data (weekday names, month names, hour cycles, etc.) via the `IcuProvider` trait (§9.5). The provider is resolved by the adapter and passed to core code via the `Env` struct — **never called from core code directly**:

1. **ArsProvider inheritance** — The adapter calls `use_icu_provider()` to read the provider from the nearest `ArsProvider` in the component tree.
2. **Ultimate fallback** — If no provider is found, the fallback is `StubIcuProvider` (English-only, zero dependencies).

```rust
// use_icu_provider() implementation (in adapter layer):
fn use_icu_provider() -> ArsRc<dyn IcuProvider> {
    use_context::<ArsContext>()
        .map(|ctx| ctx.icu_provider())
        .unwrap_or_else(|| {
            warn_missing_provider("use_icu_provider");
            ArsRc::new(StubIcuProvider)
        })
}
```

The adapter calls `use_icu_provider()` and passes the result via `Env.icu_provider`.
Core component code accesses the provider from `Env` during `init()`:

```rust
// In Props: NO provider field — resolved by the adapter.
// In Context:
pub provider: ArsRc<dyn IcuProvider>,

// In init():
provider: env.icu_provider.clone(),
```

This mirrors how React Aria resolves calendar data through its `I18nProvider` and how Ark UI uses the browser's `Intl` API — the data source is an application-level concern, not a per-component prop.

---

## 3. RTL and Bidirectionality

### 3.1 Direction Type

```rust
/// Text and layout direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// Left-to-right text direction (default for most Latin-script locales).
    #[default]
    Ltr,
    /// Right-to-left text direction (used by Arabic, Hebrew, and related scripts).
    Rtl,
    /// Automatic direction detection (resolved by the platform adapter before use).
    Auto,
}

impl Direction {
    /// CSS `direction` value.
    #[must_use]
    pub fn as_css(&self) -> &'static str {
        match self {
            Direction::Ltr => "ltr",
            Direction::Rtl => "rtl",
            Direction::Auto => "auto",
        }
    }

    /// HTML `dir` attribute value.
    #[must_use]
    pub fn as_html_attr(&self) -> &'static str {
        self.as_css()
    }

    /// Returns `true` if this direction is right-to-left.
    #[must_use]
    pub fn is_rtl(&self) -> bool {
        *self == Direction::Rtl
    }

    /// Flip a side for RTL.
    ///
    /// In RTL, "start" maps to right, "end" maps to left.
    #[must_use]
    pub fn inline_start_is_right(&self) -> bool {
        self.is_rtl()
    }
}
```

> `Orientation` — defined in `foundation/01-architecture.md`

### 3.2 Logical CSS Properties

```rust
/// A logical side in inline/block flow (independent of writing direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalSide {
    InlineStart,  // Left in LTR, Right in RTL
    InlineEnd,    // Right in LTR, Left in RTL
    BlockStart,   // Top in horizontal writing
    BlockEnd,     // Bottom in horizontal writing
}

/// A physical side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl LogicalSide {
    /// Convert to physical side given a writing direction.
    pub fn to_physical(self, dir: Direction) -> PhysicalSide {
        debug_assert!(dir != Direction::Auto, "Direction::Auto must be resolved to Ltr or Rtl before physical conversion");
        match (self, dir) {
            (LogicalSide::InlineStart, Direction::Rtl) => PhysicalSide::Right,
            (LogicalSide::InlineStart, _) => PhysicalSide::Left,
            (LogicalSide::InlineEnd, Direction::Rtl) => PhysicalSide::Left,
            (LogicalSide::InlineEnd, _) => PhysicalSide::Right,
            (LogicalSide::BlockStart, _) => PhysicalSide::Top,
            (LogicalSide::BlockEnd, _) => PhysicalSide::Bottom,
        }
    }
}

impl PhysicalSide {
    pub fn as_css(&self) -> &'static str {
        match self {
            PhysicalSide::Left => "left",
            PhysicalSide::Right => "right",
            PhysicalSide::Top => "top",
            PhysicalSide::Bottom => "bottom",
        }
    }
}

/// A logical rectangle with inline-start/end and block-start/end edges.
#[derive(Clone, Copy, Debug, Default)]
pub struct LogicalRect {
    pub inline_start: f64,
    pub inline_end: f64,
    pub block_start: f64,
    pub block_end: f64,
}

impl LogicalRect {
    /// Convert to physical rect (x, y, width, height) given direction.
    pub fn to_physical(&self, dir: Direction) -> PhysicalRect {
        if dir.is_rtl() {
            PhysicalRect {
                left: self.inline_end,
                right: self.inline_start,
                top: self.block_start,
                bottom: self.block_end,
            }
        } else {
            PhysicalRect {
                left: self.inline_start,
                right: self.inline_end,
                top: self.block_start,
                bottom: self.block_end,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicalRect {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}
```

#### 3.2.1 Mixed-Script Text Measurement and Truncation

When truncating text that contains mixed scripts (e.g., Arabic text with embedded English brand names, or CJK text with Latin abbreviations):

1. **Measurement Direction**: Use `Intl.Segmenter` (with granularity `"grapheme"`) to count user-perceived characters rather than UTF-16 code units. This avoids splitting surrogate pairs or combining character sequences.
2. **Truncation Point**: Always truncate at a grapheme cluster boundary. Never split a BiDi run mid-word — prefer truncating at the nearest whitespace or script-change boundary.
3. **Ellipsis Placement**: In RTL contexts, the ellipsis (`…`) is placed at the logical end of the string (visually on the left). In mixed-direction text, wrap the truncated text in `<bdi>` or use Unicode isolates to prevent the ellipsis from appearing in the wrong visual position.
4. **Width Measurement**: For fixed-width truncation (e.g., table cells), measure rendered width using `Canvas.measureText()` or equivalent rather than character count, as CJK characters are typically double-width relative to Latin characters.

5. **CSS-Based Truncation in RTL Contexts**: When truncation is handled via CSS (e.g., single-line labels in constrained layouts), adapters MUST emit the following CSS properties when the text direction is RTL and truncation is enabled:

    ```css
    /* Applied to the container element */
    direction: rtl;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    ```

    This ensures the browser's native text truncation places the ellipsis on the correct (visually left) side for RTL text. For mixed-script content (e.g., an Arabic label containing an English brand name), the `direction: rtl` on the container combined with `text-overflow: ellipsis` handles the common case. For fine-grained control over mixed-direction truncation, use the programmatic approach (items 1-4 above) instead of CSS truncation.

#### 3.2.2 Grapheme-Safe BiDi Isolation

Inserting Unicode BiDi isolation marks (LRI `U+2066`, RLI `U+2067`, FSI `U+2068`, PDI `U+2069`) without respecting UAX #29 extended grapheme cluster boundaries can split multi-codepoint sequences. For example, the ZWJ family emoji `👨‍👩‍👧` (U+1F468 U+200D U+1F469 U+200D U+1F467) is a single grapheme cluster; placing an isolation mark between any of its constituent codepoints causes the emoji to decompose into three separate heads. Screen readers then announce each codepoint independently (e.g., "man", "woman", "girl") instead of the unified "family" label, degrading the assistive-technology experience.

**Rule**: `ars-i18n` MUST never insert BiDi isolates within a grapheme cluster. All BiDi isolation in user-visible text MUST go through the `isolate_text_safe()` utility which iterates grapheme clusters via the `unicode-segmentation` crate.

````rust
use unicode_segmentation::UnicodeSegmentation;

/// BiDi isolation direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IsolateDirection {
    /// Left-to-right isolate (LRI, U+2066).
    Ltr,
    /// Right-to-left isolate (RLI, U+2067).
    Rtl,
    /// First-strong isolate (FSI, U+2068) — let the Unicode BiDi
    /// algorithm determine direction from the first strong character.
    FirstStrong,
}

impl IsolateDirection {
    fn opening_mark(self) -> char {
        match self {
            Self::Ltr => '\u{2066}',        // LRI
            Self::Rtl => '\u{2067}',        // RLI
            Self::FirstStrong => '\u{2068}', // FSI
        }
    }
}

const PDI: char = '\u{2069}'; // Pop Directional Isolate

/// Wrap `text` in BiDi isolation marks without splitting grapheme clusters.
///
/// The function iterates over extended grapheme clusters (UAX #29) and
/// inserts the opening isolate mark before the first cluster and the
/// closing PDI after the last cluster. Interior clusters are never
/// broken apart.
///
/// # Examples
/// ```
/// let family = "👨\u{200D}👩\u{200D}👧";
/// let isolated = isolate_text_safe(family, IsolateDirection::FirstStrong);
/// // Result: "\u{2068}👨\u{200D}👩\u{200D}👧\u{2069}"
/// // The ZWJ family emoji remains a single grapheme cluster.
/// assert_eq!(
///     isolated.graphemes(true).count(),
///     3, // FSI + family emoji + PDI
/// );
/// ```
pub fn isolate_text_safe(text: &str, direction: IsolateDirection) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Pre-allocate: opening mark (3 B) + text + closing mark (3 B).
    let mut out = String::with_capacity(text.len() + 6);
    out.push(direction.opening_mark());

    // Copy text grapheme-by-grapheme to guarantee we never insert
    // anything in the middle of a cluster.
    for cluster in text.graphemes(true) {
        out.push_str(cluster);
    }

    out.push(PDI);
    out
}
````

> **RULE**: Raw insertion of `'\u{2066}'`–`'\u{2069}'` via `String::insert` or
> `format!` is **prohibited** in user-facing text. Always use
> `isolate_text_safe()` to ensure grapheme cluster integrity.

### 3.3 RTL-Aware Placement

Overlay components use `Placement` from `11-dom-utilities.md` §2.2, which includes logical variants (`Start`, `End`, `StartTop`, `StartBottom`, `EndTop`, `EndBottom`) resolved via `Placement::resolve_logical(dir)`. This handles RTL mirroring without requiring a separate `LogicalPlacement` type.

---

## 4. Number Formatting and Parsing

### 4.1 NumberFormatter

````rust
use icu::decimal::DecimalFormatter;
use fixed_decimal::Decimal;

/// Options for formatting numbers.
#[derive(Clone, Debug)]
pub struct NumberFormatOptions {
    pub style: NumberStyle,
    pub min_integer_digits: NonZero<u8>,
    pub min_fraction_digits: u8,
    pub max_fraction_digits: u8,
    pub use_grouping: bool,
    pub sign_display: SignDisplay,
    pub rounding_mode: RoundingMode,
}

// #### Currency and Percent Formatting Defaults
//
// **ISO 4217 Fraction Digit Defaults:**
//
// The `max_fraction_digits` for currency formatting MUST default to the
// ISO 4217 standard fraction digits for the given currency code. The
// `NumberFormatter` resolves this via ICU4X `CurrencyInfo`.
//
// | Currency Code | Currency Name           | Fraction Digits |
// |---------------|-------------------------|-----------------|
// | USD           | US Dollar               | 2               |
// | EUR           | Euro                    | 2               |
// | GBP           | British Pound           | 2               |
// | JPY           | Japanese Yen            | 0               |
// | KRW           | South Korean Won        | 0               |
// | BHD           | Bahraini Dinar          | 3               |
// | KWD           | Kuwaiti Dinar           | 3               |
// | OMR           | Omani Rial              | 3               |
// | CLF           | Chilean UF              | 4               |
//
// When `NumberStyle::Currency(code)` is used and the caller has NOT
// explicitly set `max_fraction_digits`, the formatter MUST look up the
// ISO 4217 minor unit for `code` and apply it as the default.
//
// **Percent Precision Default:**
//
// When `NumberStyle::Percent` is used and no explicit fraction digits
// are provided, the default is `max_fraction_digits: 0` (i.e., whole
// percentages like "42%"). Applications requiring decimal precision
// (e.g., "4.5%") MUST explicitly set `max_fraction_digits` to the
// desired value.
//
// **Rounding Tie Semantics:**
//
// The default `RoundingMode` is `HalfEven` (banker's rounding), which
// matches the behavior of ICU4X `DecimalFormatter` and ECMA-402
// `Intl.NumberFormat`. In `HalfEven` mode, values exactly halfway
// between two representable values round to the nearest even digit:
// - 2.5 → 2, 3.5 → 4, 4.5 → 4, 5.5 → 6
//
// This minimizes cumulative rounding bias in financial calculations.

#[derive(Clone, Debug, PartialEq)]
pub enum NumberStyle {
    Decimal,
    Percent,
    Currency(CurrencyCode),
    Unit(MeasureUnit),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignDisplay {
    Auto,       // negative only
    Always,     // always show sign
    Never,      // never show sign
    ExceptZero, // always except for zero
    Negative,   // show minus for negatives only (no plus)
}

/// Rounding mode for numeric formatting.
///
/// Controls how values are rounded when the raw precision exceeds
/// `max_fraction_digits`. The default is `HalfEven` (banker's rounding)
/// to minimize cumulative rounding bias in financial calculations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RoundingMode {
    /// Round towards the nearest neighbour; if equidistant, round towards
    /// the nearest even digit. 2.5 → 2, 3.5 → 4, 4.5 → 4, 5.5 → 6.
    #[default]
    HalfEven,
    /// Round towards the nearest neighbour; if equidistant, round away
    /// from zero. 2.5 → 3, -2.5 → -3. (ECMA-402 "halfExpand")
    HalfUp,
    /// Round towards the nearest neighbour; if equidistant, round towards
    /// zero. 2.5 → 2, -2.5 → -2.
    HalfDown,
    /// Round towards positive infinity. 2.1 → 3, -2.9 → -2.
    Ceiling,
    /// Round towards negative infinity. 2.9 → 2, -2.1 → -3.
    Floor,
    /// Round towards zero (truncation). 2.9 → 2, -2.9 → -2.
    Truncate,
}

/// ISO 4217 minor-unit (fraction digit) table.
///
/// Used by `format_currency` to automatically determine the number of
/// decimal places when the caller has not explicitly set
/// `max_fraction_digits`.
///
/// | Currency Code | Currency Name           | Fraction Digits |
/// |---------------|-------------------------|-----------------|
/// | USD           | US Dollar               | 2               |
/// | EUR           | Euro                    | 2               |
/// | GBP           | British Pound           | 2               |
/// | JPY           | Japanese Yen            | 0               |
/// | KRW           | South Korean Won        | 0               |
/// | BHD           | Bahraini Dinar          | 3               |
/// | KWD           | Kuwaiti Dinar           | 3               |
/// | OMR           | Omani Rial              | 3               |
/// | CLF           | Chilean UF              | 4               |
fn iso4217_minor_units(code: &CurrencyCode) -> u8 {
    match code.as_str() {
        "BHD" | "KWD" | "OMR" | "IQD" | "LYD" | "TND" => 3,
        "CLF" | "UYW" => 4,
        "JPY" | "KRW" | "VND" | "ISK" | "CLP" | "UGX"
            | "GNF" | "XOF" | "XAF" | "XPF" | "RWF"
            | "DJF" | "KMF" | "VUV" | "PYG" => 0,
        _ => 2,
    }
}

// Currency, percent, and rounding helpers are inherent methods on NumberFormatter
// (no separate trait — these are not used polymorphically).

impl NumberFormatter {
    /// Format a monetary amount using the ISO 4217 precision for the
    /// given `currency_code`.
    ///
    /// The decimal places are resolved automatically from the ISO 4217
    /// minor-unit table (e.g., `JPY` → 0, `USD` → 2, `KWD` → 3).
    ///
    /// # Examples
    /// ```
    /// let fmt = NumberFormatter::new(&locale_en_us, opts);
    /// assert_eq!(fmt.format_currency(1234.5, "USD"), "$1,234.50");
    /// assert_eq!(fmt.format_currency(1234.5, "JPY"), "¥1,235");
    /// assert_eq!(fmt.format_currency(1234.5, "KWD"), "KWD 1,234.500");
    /// ```
    pub fn format_currency(&self, amount: f64, currency_code: &str) -> String {
        let code = CurrencyCode::from_str(currency_code)
            .expect("invalid ISO 4217 currency code");
        let precision = iso4217_minor_units(&code);

        let mut opts = self.options.clone();
        opts.style = NumberStyle::Currency(code);
        opts.min_fraction_digits = precision;
        opts.max_fraction_digits = precision;

        NumberFormatter::new(&self.locale, opts).format(amount)
    }

    pub fn format_percent(&self, value: f64, max_fraction_digits: Option<u8>) -> String {
        let frac = max_fraction_digits.unwrap_or(0);

        let mut opts = self.options.clone();
        opts.style = NumberStyle::Percent;
        opts.min_fraction_digits = 0;
        opts.max_fraction_digits = frac;

        // ICU4X Percent style handles the ×100 conversion internally;
        // pass the raw fraction (e.g., 0.75 for 75%).
        NumberFormatter::new(&self.locale, opts).format(value)
    }

    fn rounding_mode(&self) -> RoundingMode {
        self.options.rounding_mode
    }
}

/// ISO 4217 currency code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CurrencyCode(pub [u8; 3]);

impl CurrencyCode {
    pub const USD: Self = Self(*b"USD");
    pub const EUR: Self = Self(*b"EUR");
    pub const GBP: Self = Self(*b"GBP");
    pub const JPY: Self = Self(*b"JPY");
    pub const CNY: Self = Self(*b"CNY");

    pub fn from_str(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 3 {
            return None;
        }
        // Validate all bytes are ASCII uppercase letters (A-Z)
        if !bytes.iter().all(|b| b.is_ascii_uppercase()) {
            return None;
        }
        Some(Self([bytes[0], bytes[1], bytes[2]]))
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0)
            .expect("CurrencyCode must contain valid ASCII")
    }
}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            style: NumberStyle::Decimal,
            min_integer_digits: NonZero::new(1).expect("hardcoded nonzero"),
            min_fraction_digits: 0,
            max_fraction_digits: 3,
            use_grouping: true,
            sign_display: SignDisplay::Auto,
            rounding_mode: RoundingMode::HalfEven,
        }
    }
}

/// A locale-aware number formatter.
///
/// `Clone` is derived to support caching in `NUMBER_FORMATTER_CACHE` (§9.3).
/// The inner `DecimalFormatter` is wrapped in `Arc` so cloning is cheap.
#[derive(Clone)]
pub struct NumberFormatter {
    locale: Locale,
    options: NumberFormatOptions,
    formatter: Arc<DecimalFormatter>,
}

impl NumberFormatter {
    /// Create a new locale-aware number formatter.
    ///
    /// With the `compiled_data` feature (our default), `DecimalFormatter::try_new()`
    /// cannot fail — CLDR data is baked into the binary for all locales.
    pub fn new(locale: &Locale, options: NumberFormatOptions) -> Self {
        let mut dec_opts = DecimalFormatterOptions::default();
        dec_opts.grouping_strategy = if options.use_grouping {
            Some(GroupingStrategy::Auto)
        } else {
            Some(GroupingStrategy::Never)
        };
        let formatter = DecimalFormatter::try_new((&locale.0).into(), dec_opts)
            .expect("compiled_data guarantees decimal formatter is available for all locales");

        Self {
            locale: locale.clone(),
            options,
            formatter: Arc::new(formatter),
        }
    }

    /// Format an f64 value.
    pub fn format(&self, value: f64) -> String {
        // Note: Decimal::try_from_f64 returns Ok(Decimal::default()) for NaN/Inf inputs,
        // silently formatting them as "0". This is intentional — callers should validate
        // inputs before formatting if special NaN/Inf display is needed.
        let mut fd = Decimal::try_from_f64(value, fixed_decimal::FloatPrecision::Floating)
            .unwrap_or_default();

        // Apply max fraction digits (truncate beyond the limit)
        fd.trunc(-(self.options.max_fraction_digits as i16));
        // Apply min fraction digits (pad trailing zeros)
        // Decimal = Signed<UnsignedDecimal>; .absolute accesses the unsigned inner
        fd.absolute.pad_end(-(self.options.min_fraction_digits as i16));

        let formatted = self.formatter.format(&fd);
        formatted.to_string()
    }

    /// Format a range (for sliders with min-max display).
    ///
    /// The `format_range` method MUST use locale-appropriate range separators
    /// rather than a hardcoded en-dash. Examples:
    /// - English: "5–10" (en-dash, no spaces)
    /// - French: "5 – 10" (en-dash with spaces)
    /// - CJK: "5〜10" (wave dash)
    /// - Arabic: "٥–١٠" (with Arabic-Indic numerals)
    ///
    /// **Implementation backends:**
    /// - `icu4x`: ICU4X 2.x does not yet provide a range formatter. Uses a
    ///   language-match heuristic for locale-specific range separators.
    /// - `web-intl`: Use `Intl.NumberFormat.prototype.formatRange(start, end)` via
    ///   `js_sys` / `wasm_bindgen`.
    ///
    /// **Implementation**: Uses a language-match heuristic for locale-correct
    /// range separators. ICU4X 2.x does not provide a `NumberRangeFormatter`;
    /// the heuristic below is the specified approach.
    pub fn format_range(&self, start: f64, end: f64, locale: &Locale) -> String {
        // Language-match heuristic for the most common CLDR range patterns.
        // Does not handle all CLDR range patterns (e.g., Arabic spacing,
        // Tibetan separators).
        let separator = match locale.language() {
            "fr" => " – ",     // en-dash with spaces
            "ja" | "zh" | "ko" => "〜", // wave dash, no spaces
            _ => "–",          // en-dash, no spaces (default)
        };
        format!("{}{}{}", self.format(start), separator, self.format(end))
    }

    /// Parse a locale-formatted number string back to f64.
    ///
    /// Before parsing, all Unicode Nd (decimal digit) characters are normalized
    /// to ASCII 0-9 via `normalize_digits()`, enabling seamless input from
    /// non-Latin numeral systems.
    pub fn parse(&self, input: &str) -> Option<f64> {
        let normalized = normalize_digits(input);
        parse_locale_number(&normalized, &self.locale)
    }
}
````

> **RULE: All numeric values displayed to users MUST use the locale-aware
> `NumberFormatter`.** Raw `format!("{}", number)` or `to_string()` is prohibited
> in user-facing output. This includes: slider value labels, progress percentages,
> pagination counts, rating values, and any other numeric display.

````rust
/// Normalize any Unicode Nd (decimal digit) character to the corresponding ASCII 0-9.
///
/// This maps characters from any Unicode decimal digit category to their
/// numeric value, then emits the ASCII equivalent. Non-digit characters pass
/// through unchanged.
///
/// **Supported numeral systems** (non-exhaustive — all Unicode Nd characters
/// are handled, but these are the most commonly encountered):
/// - Latin (U+0030–U+0039): `0 1 2 3 4 5 6 7 8 9`
/// - Arabic-Indic (U+0660–U+0669): `٠ ١ ٢ ٣ ٤ ٥ ٦ ٧ ٨ ٩`
/// - Extended Arabic-Indic (U+06F0–U+06F9): `۰ ۱ ۲ ۳ ۴ ۵ ۶ ۷ ۸ ۹`
/// - Devanagari (U+0966–U+096F): `० १ २ ३ ४ ५ ६ ७ ८ ९`
pub fn normalize_digits(input: &str) -> String {
    input.chars().map(|c| {
        // If the character has Unicode category Nd, compute its numeric value
        // via its offset within its decimal-digit block and map to ASCII 0-9.
        if c.is_ascii_digit() {
            c // Fast path: already ASCII 0-9
        } else if c >= '\u{0660}' && c <= '\u{0669}' {
            char::from(b'0' + (c as u32 - 0x0660) as u8) // Arabic-Indic
        } else if c >= '\u{06F0}' && c <= '\u{06F9}' {
            char::from(b'0' + (c as u32 - 0x06F0) as u8) // Extended Arabic-Indic
        } else if c >= '\u{0966}' && c <= '\u{096F}' {
            char::from(b'0' + (c as u32 - 0x0966) as u8) // Devanagari
        } else if c >= '\u{09E6}' && c <= '\u{09EF}' {
            char::from(b'0' + (c as u32 - 0x09E6) as u8) // Bengali
        } else {
            c
        }
    }).collect()
}

// transliterate_digits was removed — use normalize_digits() which covers the same
// character ranges with the same logic.

/// Parse a locale-aware number string to f64.
///
/// Handles locale-specific decimal separators (`.` vs `,`) and grouping,
/// as well as non-Latin numeral systems (Arabic-Indic, Devanagari, Bengali, etc.).
///
/// # Examples
/// - `"1,234.56"` (en-US) → `Some(1234.56)`
/// - `"1.234,56"` (de-DE) → `Some(1234.56)`
/// - `"1 234,56"` (fr-FR) → `Some(1234.56)`
/// - `"١٬٢٣٤٫٥٦"` (ar) → `Some(1234.56)` (Arabic-Indic numerals)
pub fn parse_locale_number(input: &str, locale: &Locale) -> Option<f64> {
    // Step 1: Transliterate non-Latin digits to Western Arabic (0-9).
    let transliterated = normalize_digits(input);

    let (decimal_sep, group_sep) = decimal_and_group_separators(locale);

    // Step 2: Strip grouping separators, replace decimal separator with '.'
    let normalized: String = transliterated
        .chars()
        .filter(|&c| c != group_sep)
        .map(|c| if c == decimal_sep { '.' } else { c })
        .collect();

    normalized.parse::<f64>().ok()
}

/// Get the decimal and grouping separator characters for a locale.
///
/// **Production implementation** uses ICU4X 2.x `DecimalFormatter` for correctness:
/// ```rust
/// pub fn decimal_and_group_separators(locale: &Locale) -> (char, char) {
///     use icu::decimal::DecimalFormatter;
///     use fixed_decimal::Decimal;
///     // Format known numbers to extract separator characters from locale data.
///
///     let fmt = DecimalFormatter::try_new(
///         DecimalFormatterPreferences::from(&locale.0),
///         Default::default(),
///     )
///         .expect("decimal formatter for locale");
///     // Format 1234.5 to extract both grouping and decimal separators.
///     let num = Decimal::try_from_str("1234.5").expect("static decimal string is valid");
///     let formatted = fmt.format(&num).to_string();
///     // Extract decimal separator: last non-digit char with digits after it.
///     // Extract group separator: first non-digit char in the integer part.
///     // See full implementation below for the parsing logic.
///     parse_separators(&formatted)
/// }
/// ```
///
/// Extracts decimal and grouping separators from locale data by formatting
/// a known number through ICU4X `DecimalFormatter` and parsing the output.
pub fn decimal_and_group_separators(locale: &Locale) -> (char, char) {
    use icu::decimal::{DecimalFormatter, DecimalFormatterPreferences};
    use fixed_decimal::Decimal;

    let fmt = DecimalFormatter::try_new(
        DecimalFormatterPreferences::from(&locale.0),
        Default::default(),
    ).expect("decimal formatter must be available for any valid locale");

    // Format 12345.6 — guaranteed to produce both a group separator (between
    // digit groups) and a decimal separator (before the fractional part).
    let num = Decimal::try_from_str("12345.6").expect("valid decimal literal");
    let formatted = fmt.format(&num).to_string();

    // Walk the formatted string: the decimal separator is the last non-digit
    // character before the final digit(s), and the group separator is the
    // first non-digit character in the integer part.
    let mut decimal_sep = '.';
    let mut group_sep = ',';

    // Find decimal separator: last non-digit, non-minus char that has digits after it.
    // Use char::is_numeric() (Unicode Nd category) instead of is_ascii_digit() to
    // correctly handle non-Latin numeral systems (Arabic-Indic, Devanagari, etc.).
    if let Some(pos) = formatted.rfind(|c: char| !c.is_numeric() && c != '-') {
        let ch = formatted[pos..].chars().next().expect("rfind returned a valid char boundary");
        // Verify there are digits after this character (it's the decimal separator)
        if formatted[pos + ch.len_utf8()..].chars().any(|c| c.is_numeric()) {
            decimal_sep = ch;
        }
    }

    // Find group separator: first non-digit char in the integer part (before decimal)
    let integer_part = formatted.split(decimal_sep).next().unwrap_or(&formatted);
    if let Some(ch) = integer_part.chars().find(|c| !c.is_numeric() && *c != '-') {
        group_sep = ch;
    }

    (decimal_sep, group_sep)
}
````

> **Locale-Specific Grouping Sizes:** `NumberFormatter` respects locale-specific grouping sizes, which are NOT always groups of 3. Indian English (`en-IN`) groups the first 3 digits from the right, then groups of 2 thereafter (e.g., `1234567` → `"12,34,567"`). Chinese (`zh`) uses grouping of 4 digits (万/億). The ICU4X `DecimalFormatter` handles this automatically via locale data; the `web-intl` backend delegates to `Intl.NumberFormat` which also respects these rules. Test guidance: verify `en-IN` formats `1234567` as `"12,34,567"` and `ja-JP` formats `100000000` with appropriate grouping.

### 4.2 Percent and Range Formatting

```rust
impl NumberFormatter {
    /// Format as a percentage (0.0-1.0 or 0.0-100.0 based on options).
    ///
    /// Production: Use ICU4X `DecimalFormatter` with `NumberStyle::Percent`.
    /// ICU4X handles locale-specific percent symbols (Arabic: ٪ U+066A),
    /// placement (before vs after number), and spacing.
    ///
    /// The `format_percent` function MUST use locale-aware spacing between
    /// the number and percent symbol. Per CLDR, many locales require a
    /// non-breaking space (e.g., French: "75 %", German: "75 %"). The
    /// function should use ICU4X's percent formatter for full locale support.
    /// At minimum, check the locale's percent pattern for spacing requirements.
}
```

### 4.3 Plural Category Resolution

```rust
use icu::plurals::{
    PluralCategory as IcuPluralCategory,
    PluralRules,
    PluralRulesPreferences,
};

/// Determine the CLDR plural category for a count in the given locale.
/// This function is defined in `ars-i18n` and re-exported by `ars-forms`.
/// Returns the spec-defined `PluralCategory` wrapper (§6.1), not the raw ICU4X type.
pub fn plural_category(count: usize, locale: &Locale) -> PluralCategory {
    let rules = PluralRules::try_new_cardinal(PluralRulesPreferences::from(&locale.0))
        .expect("compiled_data guarantees plural rules are available for all locales");
    PluralCategory::from_icu(rules.category_for(count))
}
```

---

## 5. Date and Time Formatting

### 5.1 CalendarDate Abstraction

```rust
// ICU4X 2.x: `Date<AnyCalendar>` replaces the 1.x `AnyCalendarDate` type.
use icu::calendar::{
    AnyCalendar,
    Date,
    Gregorian,
    types::{
        DayOfMonth,
        MonthInfo,
        Weekday,
        YearInfo,
    },
};

/// A date that can be in any calendar system.
///
/// **Disambiguation:** This is the `pub(crate)` ICU4X-wrapper type used internally
/// by `ars-i18n`. It is NOT the public API type. The public API type is
/// `shared::CalendarDate` (defined in `date-time-types.md`) which uses raw fields.
/// `IcuProvider` trait methods accept and return `shared::CalendarDate`.
/// Methods like `add_days`, `days_until`, `is_before`, `today` on this internal type
/// delegate to ICU4X; the shared type has its own implementations.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CalendarDate {
    inner: Date<AnyCalendar>,
}

impl CalendarDate {
    /// Create from ISO components (year, month, day) in Gregorian calendar.
    pub fn from_iso(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        let date = Date::try_new_gregorian(year, month, day)
            .map_err(|_| DateError::InvalidDate)?;
        Ok(Self {
            inner: date.to_any(),
        })
    }

    /// Create from components in a specific calendar system.
    ///
    /// Uses ICU4X `Date::try_new_from_codes()` with an `AnyCalendar` instance.
    /// The `month` parameter is the 1-based month ordinal (not a month code).
    /// Era is inferred from the year sign for era-based calendars.
    pub(crate) fn from_calendar(
        year: i32,
        month: u8,
        day: u8,
        calendar: CalendarSystem,
    ) -> Result<Self, CalendarConversionError> {
        use icu::calendar::{AnyCalendar, types::MonthCode};
        use tinystr::tinystr;

        let any_cal = AnyCalendar::new(calendar.to_icu_kind());

        // ICU4X 2.x: MonthCode::new_normal(ordinal) constructs "M01".."M13".
        // For leap months (Chinese/Hebrew), use MonthCode::new_leap(ordinal) instead.
        let month_code = MonthCode::new_normal(month)
            .ok_or(CalendarConversionError::InvalidDate)?;

        // For era-based calendars, infer era from the calendar kind.
        // Gregorian: positive year → "ce", negative → "bce".
        // Japanese: era must be resolved from the year; default to current era.
        // Other calendars: pass None and let ICU4X infer.
        let era: Option<&str> = match calendar {
            CalendarSystem::Gregorian => {
                if year >= 1 { Some("ce") } else { Some("bce") }
            }
            _ => None,
        };

        let date = Date::try_new_from_codes(era, year, month_code, day, any_cal)
            .map_err(|e| CalendarConversionError::Icu(e.to_string()))?;

        Ok(Self { inner: date })
    }

    /// Convert to a different calendar system.
    pub fn to_calendar(&self, calendar: CalendarSystem) -> CalendarDate {
        let any_cal = AnyCalendar::new(calendar.to_icu_kind());
        CalendarDate {
            inner: self.inner.to_calendar(any_cal),
        }
    }

    pub fn year(&self) -> i32 {
        // ICU4X 2.x: YearInfo is an enum; era_year_or_related_iso() returns
        // the displayable year number (era year for era-based, related ISO for cyclic).
        self.inner.year().era_year_or_related_iso()
    }

    pub fn month(&self) -> u8 {
        // Safe: month ordinals never exceed 13 (max for lunisolar calendars).
        // ICU4X 2.x: MonthInfo provides month_number() for the 1-based ordinal.
        // Note: Both `MonthInfo.ordinal` (public field) and `.month_number()` method
        // are valid accessors. This spec uses `.month_number()` consistently. For leap
        // months (e.g., Hebrew Adar II), `.month_number()` equals `.ordinal`.
        let ordinal = self.inner.month().month_number();
        debug_assert!(ordinal <= 13, "month ordinal {ordinal} exceeds expected max of 13");
        ordinal
    }

    pub fn day(&self) -> u8 {
        // ICU4X 2.x: DayOfMonth is a newtype wrapping u8; access via `.0`.
        self.inner.day_of_month().0
    }

    /// Day of week.
    /// ICU4X `day_of_week()` returns `Weekday` (ISO 8601: Monday=1, Sunday=7).
    pub fn weekday(&self) -> Weekday {
        Weekday::from_icu_weekday(self.inner.day_of_week())
    }

    /// Era name (relevant for Japanese calendar: Reiwa, Heisei, etc.)
    /// ICU4X 2.x: YearInfo::era() returns Option<EraYear>; EraYear.era is TinyAsciiStr<16>.
    pub fn era(&self) -> Option<String> {
        self.inner.year().era().map(|e| e.era.to_string())
    }

    /// Add days to this date, returning a new `CalendarDate`.
    ///
    /// Strategy: convert to ISO, perform arithmetic on the ISO date (which is
    /// a simple proleptic Gregorian with fixed month lengths), then convert back
    /// to the original calendar system.
    pub fn add_days(&self, days: i32) -> Result<CalendarDate, CalendarError> {
        use icu::calendar::{Date, Iso};

        // Convert to ISO for arithmetic (ICU4X Date<Iso> supports day offset).
        let iso: Date<Iso> = self.inner.to_iso();
        // ICU4X Date<Iso> doesn't expose direct day-add, so reconstruct via
        // epoch-day arithmetic. ISO epoch day: days since 0001-01-01.
        let iso_year = iso.year().era_year_or_related_iso();
        let iso_month = iso.month().month_number();
        let iso_day = iso.day_of_month().0;

        // Convert to a simple day count, add offset, convert back.
        let epoch_days = iso_to_epoch_days(iso_year, iso_month, iso_day);
        let new_epoch = epoch_days + days as i64;
        let (new_y, new_m, new_d) = epoch_days_to_iso(new_epoch);

        let new_iso = Date::try_new_iso(new_y, new_m, new_d)
            .map_err(|e| CalendarError::Arithmetic(e.to_string()))?;

        // Convert back to the original calendar.
        let any_cal = self.inner.calendar().clone();
        Ok(CalendarDate {
            inner: new_iso.to_any().to_calendar(any_cal),
        })
    }

    /// Days between this date and `other` (positive if `other` is later).
    ///
    /// Both dates are converted to ISO for epoch-day subtraction, so this
    /// works correctly across different calendar systems.
    pub fn days_until(&self, other: &CalendarDate) -> Result<i32, CalendarError> {
        let self_iso = self.inner.to_iso();
        let other_iso = other.inner.to_iso();

        let self_epoch = iso_to_epoch_days(
            self_iso.year().era_year_or_related_iso(),
            self_iso.month().month_number(),
            self_iso.day_of_month().0,
        );
        let other_epoch = iso_to_epoch_days(
            other_iso.year().era_year_or_related_iso(),
            other_iso.month().month_number(),
            other_iso.day_of_month().0,
        );

        let diff = other_epoch - self_epoch;
        i32::try_from(diff).map_err(|_| CalendarError::Arithmetic(
            "date difference exceeds i32 range".into()
        ))
    }

    /// Whether this date is chronologically before `other`.
    ///
    /// Comparison is performed via ISO epoch days, so it works correctly
    /// across different calendar systems.
    pub fn is_before(&self, other: &CalendarDate) -> Result<bool, CalendarError> {
        self.days_until(other).map(|diff| diff > 0)
    }

    /// Today's date in the given calendar system.
    ///
    /// On WASM targets, uses `js_sys::Date::now()` to get the current UTC
    /// timestamp. On native targets, uses `std::time::SystemTime`.
    pub fn today(calendar: CalendarSystem) -> Result<CalendarDate, CalendarError> {
        let (year, month, day) = platform_today_iso()
            .map_err(|e| CalendarError::Arithmetic(e))?;

        let iso = icu::calendar::Date::try_new_iso(year, month, day)
            .map_err(|e| CalendarError::Arithmetic(e.to_string()))?;

        let any_cal = AnyCalendar::new(calendar.to_icu_kind());
        Ok(CalendarDate {
            inner: iso.to_any().to_calendar(any_cal),
        })
    }
}

/// Convert an ISO date (year, month, day) to an epoch day count.
/// Epoch: day 0 = 0000-03-01 (shifted March epoch avoids leap-day edge cases).
fn iso_to_epoch_days(year: i32, month: u8, day: u8) -> i64 {
    let y = if month <= 2 { year as i64 - 1 } else { year as i64 };
    let m = if month <= 2 { month as i64 + 9 } else { month as i64 - 3 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468 // shift to Unix-like epoch (0001-01-01)
}

/// Convert an epoch day count back to an ISO (year, month, day) triple.
fn epoch_days_to_iso(epoch: i64) -> (i32, u8, u8) {
    let z = epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m as u8, d as u8)
}

/// Get today's ISO date components from the platform clock.
#[cfg(target_arch = "wasm32")]
fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    let now = js_sys::Date::new_0();
    let year = now.get_full_year() as i32;
    let month = (now.get_month() + 1) as u8; // JS months are 0-indexed; get_month() returns u32
    let day = now.get_date() as u8;
    Ok((year, month, day))
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_today_iso() -> Result<(i32, u8, u8), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let days = (secs / 86400) as i64;
    let (y, m, d) = epoch_days_to_iso(days);
    Ok((y, m, d))
}

// `Weekday` — defined in `shared/date-time-types.md`
// The following extension methods are provided by `ars-i18n` on the canonical type.

impl Weekday {
    // `from_sunday_zero()` and `from_iso_8601()` are defined on the canonical
    // Weekday type in shared/date-time-types.md (ars-core crate). The ars-i18n
    // crate re-exports Weekday from ars-core, so these methods are available
    // without redefinition. See shared/date-time-types.md §3.

    pub fn from_icu_str(s: &str) -> Option<Self> {
        match s {
            "mon" => Some(Weekday::Monday),
            "tue" => Some(Weekday::Tuesday),
            "wed" => Some(Weekday::Wednesday),
            "thu" => Some(Weekday::Thursday),
            "fri" => Some(Weekday::Friday),
            "sat" => Some(Weekday::Saturday),
            "sun" => Some(Weekday::Sunday),
            _ => None,
        }
    }

    /// Parse from BCP 47 `-u-fw-` extension value (e.g., "mon", "sun").
    pub fn from_bcp47_fw(s: &str) -> Option<Self> {
        Self::from_icu_str(s)
    }

    /// Convert from ICU4X `icu::calendar::types::Weekday` to our `Weekday`.
    pub fn from_icu_weekday(iwd: icu::calendar::types::Weekday) -> Self {
        match iwd {
            icu::calendar::types::Weekday::Monday    => Self::Monday,
            icu::calendar::types::Weekday::Tuesday   => Self::Tuesday,
            icu::calendar::types::Weekday::Wednesday => Self::Wednesday,
            icu::calendar::types::Weekday::Thursday  => Self::Thursday,
            icu::calendar::types::Weekday::Friday    => Self::Friday,
            icu::calendar::types::Weekday::Saturday  => Self::Saturday,
            icu::calendar::types::Weekday::Sunday    => Self::Sunday,
        }
    }

    /// Short (abbreviated) day-of-week label for display.
    /// Canonical implementation is in `shared/date-time-types.md`. These convenience methods delegate to IcuProvider.
    // ICU4X: Use DateSymbols API to access weekday names. The exact API depends on ICU4X version.
    pub fn short_label(&self, provider: &dyn IcuProvider, locale: &Locale) -> String {
        provider.weekday_short_label(*self, locale)
    }

    /// Full day-of-week label for display.
    /// Canonical implementation is in `shared/date-time-types.md`. These convenience methods delegate to IcuProvider.
    // ICU4X: Use DateSymbols API to access weekday names. The exact API depends on ICU4X version.
    pub fn long_label(&self, provider: &dyn IcuProvider, locale: &Locale) -> String {
        provider.weekday_long_label(*self, locale)
    }

    /// Convert to ICU4X `Weekday` for use with formatters.
    fn to_icu_weekday(&self) -> icu::calendar::types::Weekday {
        match self {
            Weekday::Monday    => icu::calendar::types::Weekday::Monday,
            Weekday::Tuesday   => icu::calendar::types::Weekday::Tuesday,
            Weekday::Wednesday => icu::calendar::types::Weekday::Wednesday,
            Weekday::Thursday  => icu::calendar::types::Weekday::Thursday,
            Weekday::Friday    => icu::calendar::types::Weekday::Friday,
            Weekday::Saturday  => icu::calendar::types::Weekday::Saturday,
            Weekday::Sunday    => icu::calendar::types::Weekday::Sunday,
        }
    }

    /// English abbreviated day name (private fallback helper).
    fn english_short_label(&self) -> &'static str {
        match self {
            Weekday::Monday    => "Mon",
            Weekday::Tuesday   => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday  => "Thu",
            Weekday::Friday    => "Fri",
            Weekday::Saturday  => "Sat",
            Weekday::Sunday    => "Sun",
        }
    }

    /// English full day name (private fallback helper).
    fn english_long_label(&self) -> &'static str {
        match self {
            Weekday::Monday    => "Monday",
            Weekday::Tuesday   => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday  => "Thursday",
            Weekday::Friday    => "Friday",
            Weekday::Saturday  => "Saturday",
            Weekday::Sunday    => "Sunday",
        }
    }
}
```

### 5.2 Calendar Systems

> `CalendarSystem` — defined in `shared/date-time-types.md`

The following extension methods are provided by `ars-i18n` on the canonical `CalendarSystem` type:

```rust
impl CalendarSystem {
    /// Get from BCP 47 calendar extension value.
    ///
    /// All 16 variants are mapped from their CLDR/BCP-47 `ca` extension keys.
    /// Reference: Unicode LDML §3.6 "Calendar Algorithm" and IANA BCP-47
    /// `u-ca` subtag registry.
    pub fn from_bcp47(s: &str) -> Option<Self> {
        match s {
            "gregory" | "gregorian" => Some(Self::Gregorian),
            "buddhist" => Some(Self::Buddhist),
            "japanese" => Some(Self::Japanese),
            "japanext" => Some(Self::JapaneseExtended),
            "hebrew" => Some(Self::Hebrew),
            "islamic" => Some(Self::Islamic),
            "islamic-civil" => Some(Self::IslamicCivil),
            "islamic-umalqura" => Some(Self::IslamicUmmAlQura),
            "persian" => Some(Self::Persian),
            "indian" => Some(Self::Indian),
            "chinese" => Some(Self::Chinese),
            "coptic" => Some(Self::Coptic),
            "dangi" => Some(Self::Dangi),
            "ethiopic" => Some(Self::Ethiopic),
            "ethioaa" => Some(Self::EthiopicAmeteAlem),
            "roc" => Some(Self::Roc),
            _ => None,
        }
    }

    /// Get the calendar from a locale's extension.
    pub fn from_locale(locale: &Locale) -> Self {
        locale.calendar_extension()
            .and_then(Self::from_bcp47)
            .unwrap_or(Self::Gregorian)
    }

    /// ICU4X AnyCalendarKind representation.
    ///
    /// Maps each `CalendarSystem` variant to the corresponding ICU4X
    /// `AnyCalendarKind` discriminant. This is used when constructing
    /// an `icu::calendar::AnyCalendar` for date arithmetic and formatting.
    pub fn to_icu_kind(&self) -> icu::calendar::AnyCalendarKind {
        use icu::calendar::AnyCalendarKind::*;
        match self {
            Self::Gregorian => Gregorian,
            Self::Buddhist => Buddhist,
            Self::Japanese => Japanese,
            Self::JapaneseExtended => JapaneseExtended,
            Self::Hebrew => Hebrew,
            Self::Islamic => HijriSimulatedMecca,
            Self::IslamicCivil => HijriTabularTypeIIFriday,
            Self::IslamicUmmAlQura => HijriUmmAlQura,
            Self::Persian => Persian,
            Self::Indian => Indian,
            Self::Chinese => Chinese,
            Self::Coptic => Coptic,
            Self::Dangi => Dangi,
            Self::Ethiopic => Ethiopian,
            Self::EthiopicAmeteAlem => EthiopianAmeteAlem,
            Self::Roc => Roc,
        }
    }

    /// Whether this calendar uses an era system other than AD/BC.
    pub fn has_custom_eras(&self) -> bool {
        matches!(self,
            Self::Japanese | Self::JapaneseExtended
            | Self::Ethiopic | Self::EthiopicAmeteAlem
            | Self::Coptic
            | Self::Hebrew | Self::Persian
            | Self::Islamic | Self::IslamicCivil | Self::IslamicUmmAlQura
            | Self::Roc  // ROC uses the Minguo era
            // Note: Chinese/Dangi are intentionally excluded — they use sexagenary
            // cycles rather than named eras in the traditional sense.
            // Note: Buddhist and Indian use simple epoch offsets (BE/Saka), not
            // named eras like Japanese. They are excluded to avoid generating
            // spurious Era structs in CalendarDate when the year already encodes
            // the era offset.
        )
    }

    /// Japanese era names.
    pub fn japanese_eras() -> &'static [JapaneseEra] {
        &[
            JapaneseEra { name: "Meiji",  start_year: 1868 },
            JapaneseEra { name: "Taisho", start_year: 1912 },
            JapaneseEra { name: "Showa",  start_year: 1926 },
            JapaneseEra { name: "Heisei", start_year: 1989 },
            JapaneseEra { name: "Reiwa",  start_year: 2019 },
        ]
    }
}
```

#### 5.2.1 Calendar System Constraints

The `CalendarSystem` enum MUST provide a `supported_calendars()` method returning metadata for each supported calendar:

```rust
impl CalendarSystem {
    /// Returns the list of supported calendar systems with their constraints.
    pub fn supported_calendars() -> &'static [CalendarMetadata] { /* ... */ }
}

pub struct CalendarMetadata {
    pub system: CalendarSystem,
    pub month_range: RangeInclusive<u8>,    // e.g., 1..=12 for Gregorian, 1..=13 for Hebrew (leap)
    pub has_leap_months: bool,
    pub era_required: bool,
    pub typical_year_lengths: &'static [u16], // e.g., [365, 366] for Gregorian
}
```

| Calendar  | Months | Leap Rule                                          | Era Required | Notes                                                  |
| --------- | ------ | -------------------------------------------------- | ------------ | ------------------------------------------------------ |
| Gregorian | 1–12   | Feb 29 if divisible by 4 (not 100, except 400)     | No           | Default calendar                                       |
| Islamic   | 1–12   | 30-year cycle: years 2,5,7,10,13,16,18,21,24,26,29 | No           | Month length 29–30 days; 354/355-day year              |
| Hebrew    | 1–13   | 19-year Metonic cycle: years 3,6,8,11,14,17,19     | No           | Month 13 (Adar II) only in leap years                  |
| Japanese  | 1–12   | Same as Gregorian                                  | **Yes**      | Era changes on emperor accession (Reiwa, Heisei, etc.) |
| Buddhist  | 1–12   | Same as Gregorian                                  | No           | Year = Gregorian + 543 (BE)                            |
| ROC       | 1–12   | Same as Gregorian                                  | No           | Year = Gregorian − 1911 (Minguo)                       |

Components receiving a `CalendarSystem` MUST validate month/day values against the constraints above. For example, setting month=13 on a non-leap Hebrew year MUST return a validation error.

```rust
pub struct JapaneseEra {
    pub name: &'static str,
    pub start_year: u32,
}

impl JapaneseEra {
    /// Returns the era name in the appropriate script for the locale.
    /// Japanese locale returns native script (e.g., "令和"),
    /// others return romanized form (e.g., "Reiwa").
    pub fn localized_name(&self, locale: &Locale) -> String {
        if locale.language() == "ja" {
            self.native_name().to_string()
        } else {
            self.romanized_name().to_string()
        }
    }

    /// Romanized era name (used for non-Japanese locales).
    pub fn romanized_name(&self) -> &str {
        self.name
    }

    /// Native Japanese script era name.
    fn native_name(&self) -> &str {
        match self.name {
            "Reiwa"  => "令和",
            "Heisei" => "平成",
            "Showa"  => "昭和",
            "Taisho" => "大正",
            "Meiji"  => "明治",
            other    => other, // Fallback to romanized for unknown eras
        }
    }
}
```

### 5.3 Week Information

```rust
/// Week numbering information for a locale.
#[derive(Clone, Debug)]
pub struct WeekInfo {
    /// First day of the week (Monday in ISO, Sunday in US).
    pub first_day: Weekday,
    /// Minimum days in the first week of the year (ISO: 4, US: 1).
    pub min_days_in_first_week: u8,
}

impl WeekInfo {
    /// Get week info for a locale.
    pub fn for_locale(locale: &Locale) -> Self {
        // Look up min_days from the locale's region regardless of fw extension.
        // The fw extension only overrides first_day, not min_days_in_first_week.
        let region = locale.region().unwrap_or("");
        let min_days = match region {
            "US" | "CA" | "MX" | "AU" | "JP" | "CN" | "TW" | "HK" | "KR" | "SG"
            | "AF" | "IR" | "SA" | "AE" | "EG" | "DZ" | "MA" | "TN" | "LY" => 1,
            _ => 4, // ISO 8601 default
        };

        // Check explicit fw extension — overrides ONLY first_day.
        if let Some(fw) = locale.first_day_of_week_extension() {
            return Self { first_day: fw, min_days_in_first_week: min_days };
        }

        // Then infer from region
        match region {
            "US" | "CA" | "MX" | "AU" | "JP" | "CN" | "TW" | "HK" | "KR" | "SG" => {
                Self { first_day: Weekday::Sunday, min_days_in_first_week: 1 }
            }
            "AF" | "IR" | "SA" | "AE" | "EG" | "DZ" | "MA" | "TN" | "LY" => {
                Self { first_day: Weekday::Saturday, min_days_in_first_week: 1 }
            }
            _ => Self { first_day: Weekday::Monday, min_days_in_first_week: 4 }, // ISO 8601
        }
    }

    /// Ordered weekdays starting from first_day.
    pub fn ordered_weekdays(&self) -> [Weekday; 7] {
        let all = [
            Weekday::Sunday, Weekday::Monday, Weekday::Tuesday,
            Weekday::Wednesday, Weekday::Thursday, Weekday::Friday, Weekday::Saturday,
        ];
        // Monday (index 1) is the ISO 8601 default. The lookup can only fail if
        // self.first_day is somehow not in the Weekday enum, which is impossible —
        // but we use a safe fallback rather than panicking.
        let start = all.iter().position(|&w| w == self.first_day).unwrap_or(1);
        let mut result = [Weekday::Monday; 7];
        for i in 0..7 {
            result[i] = all[(start + i) % 7];
        }
        result
    }
}
```

> **First Day of Week in Calendar Layout:** Calendar grid rendering MUST start on the locale's first day of week as determined by `WeekInfo::for_locale()`. US calendars (`en-US`) start on Sunday; most European locales start on Monday; Arabic locales (`ar-SA`) start on Saturday. The `ordered_weekdays()` method provides the correctly-ordered weekday array for rendering the calendar header row. Cross-reference: see `components/date-time/calendar.md` for grid rendering details. Test: `ar-SA` → first column is Saturday; `en-US` → first column is Sunday; `de-DE` → first column is Monday.

### 5.4 DateFormatter

```rust
use icu::datetime::{
    DateTimeFormatter,
    DateTimeFormatterPreferences,
    fieldsets::{YMD, YMDE, T},
};

/// Length of the formatted date/time string.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FormatLength {
    Full,    // "Wednesday, March 15, 2024" — uses YMDE::long() (weekday + date)
    Long,    // "March 15, 2024"
    #[default]
    Medium,  // "Mar 15, 2024"
    Short,   // "3/15/24"
}

impl FormatLength {
    /// Returns the ICU4X 2.x field set for date-only formatting.
    /// `Full` uses `YMDE` (includes weekday); others use `YMD`.
    fn to_icu_date_field_set(&self) -> YMD {
        debug_assert!(!self.is_full(), "Full must use to_icu_full_date_field_set() instead");
        match self {
            // Full is handled separately via to_icu_full_date_field_set()
            // because YMDE is a different type than YMD.
            // Full is handled at the caller level; if reached here, fall back to Long.
            Self::Full => YMD::long(),
            Self::Long => YMD::long(),
            Self::Medium => YMD::medium(),
            Self::Short => YMD::short(),
        }
    }

    /// Returns the YMDE field set for Full length (includes weekday).
    fn to_icu_full_date_field_set(&self) -> YMDE {
        YMDE::long()
    }

    fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Returns the ICU4X 2.x field set for time-only formatting.
    /// ICU4X 2.x `T` fieldset uses precision-based constructors rather than
    /// length-based ones: `T::hms()` (hour+minute+second) and `T::hm()` (hour+minute).
    /// There are no `T::short()`/`T::medium()`/`T::long()`/`T::full()` methods.
    fn to_icu_time_field_set(&self) -> T {
        match self {
            Self::Full | Self::Long => T::hms(),
            Self::Medium | Self::Short => T::hm(),
        }
    }
}

/// Internal wrapper: Full uses YMDE (includes weekday), others use YMD.
enum DateFormatterInner {
    Ymd(DateTimeFormatter<YMD>),
    Ymde(DateTimeFormatter<YMDE>),
}

pub struct DateFormatter {
    locale: Locale,
    length: FormatLength,
    inner: DateFormatterInner,
}

impl DateFormatter {
    /// Create a new locale-aware date formatter.
    ///
    /// With the `compiled_data` feature (our default), `DateTimeFormatter::try_new()`
    /// cannot fail — CLDR data is baked into the binary for all locales.
    pub fn new(locale: &Locale, length: FormatLength) -> Self {
        let prefs = DateTimeFormatterPreferences::from(&locale.0);
        let inner = if length.is_full() {
            // Full: include weekday via YMDE field set
            DateFormatterInner::Ymde(
                DateTimeFormatter::try_new(prefs, length.to_icu_full_date_field_set())
                    .expect("compiled_data guarantees date formatter is available for all locales")
            )
        } else {
            DateFormatterInner::Ymd(
                DateTimeFormatter::try_new(prefs, length.to_icu_date_field_set())
                    .expect("compiled_data guarantees date formatter is available for all locales")
            )
        };

        Self { locale: locale.clone(), length, inner }
    }

    pub fn format(&self, date: &shared::CalendarDate) -> String {
        let internal: CalendarDate = date.into();
        match &self.inner {
            DateFormatterInner::Ymd(fmt) => fmt.format(&internal.inner).to_string(),
            DateFormatterInner::Ymde(fmt) => fmt.format(&internal.inner).to_string(),
        }
    }
}

impl From<&shared::CalendarDate> for CalendarDate {
    fn from(shared: &shared::CalendarDate) -> Self {
        match shared.calendar {
            CalendarSystem::Gregorian => {
                CalendarDate::from_iso(shared.year, shared.month.get(), shared.day.get())
                    .expect("shared::CalendarDate should contain valid date components")
            }
            other => {
                CalendarDate::from_calendar(shared.year, shared.month.get(), shared.day.get(), other)
                    .expect("shared::CalendarDate should contain valid date components")
            }
        }
    }
}
```

### 5.5 RelativeTimeFormatter

```rust
use icu::experimental::relativetime::{
    RelativeTimeFormatter as IcuRelativeTimeFormatter,
    RelativeTimeFormatterPreferences,
    options::{RelativeTimeFormatterOptions, Numeric},
};

/// Controls whether relative time uses numeric format or natural language.
/// Maps directly to ICU4X `Numeric` enum.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NumericOption {
    /// Always use numeric format: "1 day ago", "in 2 hours"
    #[default]
    Always,
    /// Use natural language when possible: "yesterday", "tomorrow", "last week"
    Auto,
}

impl NumericOption {
    fn to_icu(self) -> RelativeTimeFormatterOptions {
        RelativeTimeFormatterOptions {
            numeric: match self {
                NumericOption::Always => Numeric::Always,
                NumericOption::Auto => Numeric::Auto,
            },
        }
    }
}

pub struct RelativeTimeFormatter {
    locale: Locale,
    /// Whether to prefer numeric ("1 day ago") or natural ("yesterday") output.
    /// Default: `NumericOption::Always`.
    numeric: NumericOption,
    second_formatter: IcuRelativeTimeFormatter,
    minute_formatter: IcuRelativeTimeFormatter,
    hour_formatter: IcuRelativeTimeFormatter,
    day_formatter: IcuRelativeTimeFormatter,
}

impl RelativeTimeFormatter {
    pub fn new(locale: &Locale) -> Self {
        Self::with_numeric(locale, NumericOption::default())
    }

    /// Create a relative time formatter with explicit numeric preference.
    ///
    /// With the `compiled_data` feature (our default), `IcuRelativeTimeFormatter::try_new_long_*()`
    /// cannot fail — CLDR data is baked into the binary for all locales.
    pub fn with_numeric(locale: &Locale, numeric: NumericOption) -> Self {
        let opts = numeric.to_icu();
        let second_formatter = IcuRelativeTimeFormatter::try_new_long_second(
            RelativeTimeFormatterPreferences::from(&locale.0), opts,
        ).expect("compiled_data guarantees relative time data is available for all locales");
        let minute_formatter = IcuRelativeTimeFormatter::try_new_long_minute(
            RelativeTimeFormatterPreferences::from(&locale.0), opts,
        ).expect("compiled_data guarantees relative time data is available for all locales");
        let hour_formatter = IcuRelativeTimeFormatter::try_new_long_hour(
            RelativeTimeFormatterPreferences::from(&locale.0), opts,
        ).expect("compiled_data guarantees relative time data is available for all locales");
        let day_formatter = IcuRelativeTimeFormatter::try_new_long_day(
            RelativeTimeFormatterPreferences::from(&locale.0), opts,
        ).expect("compiled_data guarantees relative time data is available for all locales");

        Self {
            locale: locale.clone(),
            numeric,
            second_formatter, minute_formatter, hour_formatter, day_formatter,
        }
    }

    /// Format a duration relative to now.
    /// Positive = future ("in 5 minutes"), negative = past ("5 minutes ago").
    /// The sign of `seconds` determines direction: ICU4X handles past/future
    /// formatting based on the sign of the Decimal value.
    pub fn format_seconds(&self, seconds: i64) -> String {
        if seconds.abs() < 60 {
            let fd = fixed_decimal::Decimal::from(seconds);
            self.second_formatter.format(fd).to_string()
        } else if seconds.abs() < 3600 {
            let minutes = seconds / 60;
            let fd = fixed_decimal::Decimal::from(minutes);
            self.minute_formatter.format(fd).to_string()
        } else if seconds.abs() < 86400 {
            let hours = seconds / 3600;
            let fd = fixed_decimal::Decimal::from(hours);
            self.hour_formatter.format(fd).to_string()
        } else {
            let days = seconds / 86400;
            let fd = fixed_decimal::Decimal::from(days);
            self.day_formatter.format(fd).to_string()
        }
    }
}
```

---

## 6. Plural and Ordinal Rules

### 6.1 PluralCategory

````rust
use icu::plurals::{
    PluralCategory as IcuPluralCategory,
    PluralRules,
    PluralRulesPreferences,
};

/// CLDR plural categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    Zero,   // Languages: Arabic (ar), Welsh (cy)
    One,    // Most European languages
    Two,    // Arabic, Welsh, Slovenian (in some contexts)
    Few,    // Slavic languages, Arabic
    Many,   // Arabic, Polish, Russian (for large numbers)
    Other,  // Always present as fallback
}

impl PluralCategory {
    pub fn from_icu(cat: IcuPluralCategory) -> Self {
        match cat {
            IcuPluralCategory::Zero => Self::Zero,
            IcuPluralCategory::One => Self::One,
            IcuPluralCategory::Two => Self::Two,
            IcuPluralCategory::Few => Self::Few,
            IcuPluralCategory::Many => Self::Many,
            IcuPluralCategory::Other => Self::Other,
        }
    }
}

/// A map from plural categories to localized strings.
///
/// # Example
/// ```rust
/// let strings = Plural {
///     one: "1 item selected",
///     other: "{count} items selected",
///     ..Plural::from_other("{count} items selected"),
/// };
/// ```
#[derive(Clone, Debug)]
pub struct Plural<T: Clone> {
    pub zero: Option<T>,
    pub one: Option<T>,
    pub two: Option<T>,
    pub few: Option<T>,
    pub many: Option<T>,
    pub other: T,   // Required fallback
}

impl<T: Clone> Plural<T> {
    pub fn from_other(other: T) -> Self {
        Self { zero: None, one: None, two: None, few: None, many: None, other }
    }

    pub fn get(&self, category: PluralCategory) -> &T {
        match category {
            PluralCategory::Zero => self.zero.as_ref().unwrap_or(&self.other),
            PluralCategory::One => self.one.as_ref().unwrap_or(&self.other),
            PluralCategory::Two => self.two.as_ref().unwrap_or(&self.other),
            PluralCategory::Few => self.few.as_ref().unwrap_or(&self.other),
            PluralCategory::Many => self.many.as_ref().unwrap_or(&self.other),
            PluralCategory::Other => &self.other,
        }
    }
}

/// Selects which plural rule system to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralRuleType {
    /// Cardinal numbers (1 item, 2 items, ...)
    Cardinal,
    /// Ordinal numbers (1st, 2nd, 3rd, ...)
    Ordinal,
}

/// Select the correct plural form for a count.
///
/// With the `compiled_data` feature (our default), `PluralRules::try_new_*`
/// cannot fail — CLDR data is baked into the binary for all locales. The
/// `try_new_*` methods only return `Result` for the custom data provider
/// path. We unwrap here because our `Locale` type guarantees validity by
/// construction and compiled data is always present.
pub fn select_plural(
    locale: &Locale,
    count: f64,
    rule_type: PluralRuleType,
) -> PluralCategory {
    let rules = match rule_type {
        PluralRuleType::Cardinal => PluralRules::try_new_cardinal(PluralRulesPreferences::from(&locale.0)),
        PluralRuleType::Ordinal => PluralRules::try_new_ordinal(PluralRulesPreferences::from(&locale.0)),
    }
    .expect("compiled_data guarantees plural rules are available for all locales");

    // f64 must go through Decimal (not directly to category_for) because CLDR plural
    // rules depend on trailing zeros: "1.0" is "other" in English, not "one". Decimal
    // preserves the original precision from f64. In contrast, plural_category(usize)
    // (§4.3) can pass usize directly since integers have no trailing-zero ambiguity.
    let fd = fixed_decimal::Decimal::try_from_f64(count, fixed_decimal::FloatPrecision::Floating).unwrap_or_default();
    PluralCategory::from_icu(rules.category_for(fd))
}

/// Format a count with the appropriate plural form.
///
/// # Example
/// ```rust
/// let en = Locale::parse("en-US").expect("valid locale");
/// let msg = Plural {
///     one: Some("{count} item selected"),
///     other: "{count} items selected",
///     ..Plural::from_other("{count} items selected")
/// };
/// assert_eq!(
///     format_plural(&en, 1.0, &msg, &[("count", "1")]),
///     "1 item selected"
/// );
/// assert_eq!(
///     format_plural(&en, 3.0, &msg, &[("count", "3")]),
///     "3 items selected"
/// );
/// ```
pub fn format_plural(
    locale: &Locale,
    count: f64,
    plural: &Plural<&str>,
    args: &[(&str, &str)],
) -> String {
    let category = select_plural(locale, count, PluralRuleType::Cardinal);
    let template = plural.get(category);
    interpolate(template, args)
}

fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in args {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
}
````

### 6.2 ICU MessageFormat Plural Syntax

While the `Plural<T>` struct and `format_plural` function provide programmatic plural
handling, all components that display count-dependent text MUST use a consistent template
syntax based on ICU MessageFormat:

```text
{count, plural, one {# item} other {# items}}
```

**Template syntax rules:**

- `{variable, plural, ...}` selects the plural form for the given variable.
- `#` is replaced with the formatted count value (locale-aware number formatting).
- All six CLDR categories are supported: `zero`, `one`, `two`, `few`, `many`, `other`.
- `other` is always required as the fallback.
- Exact matches use `=N` syntax: `{count, plural, =0 {No items} one {# item} other {# items}}`.

**Component integration requirement:** Every component that displays a count MUST use
`format_plural` (or the ICU MessageFormat equivalent) rather than ad-hoc string concatenation.
This applies to:

| Component  | Plural-Dependent Text                                                             |
| ---------- | --------------------------------------------------------------------------------- |
| Select     | `"{count} selected"` / `"{count, plural, one {# item} other {# items}} selected"` |
| TagsInput  | `"{count, plural, one {# tag} other {# tags}}"`                                   |
| Table      | `"{count, plural, one {# row} other {# rows}} selected"`                          |
| Pagination | `"Page {page} of {total, plural, one {# page} other {# pages}}"`                  |
| FileUpload | `"{count, plural, one {# file} other {# files}} selected"`                        |
| Tree       | `"{count, plural, one {# item} other {# items}} expanded"`                        |

**Nesting example** (select inside plural):

```text
{count, plural,
  =0 {No items}
  one {{count, selectordinal, one {#st} two {#nd} few {#rd} other {#th}} item}
  other {# items}
}
```

### 6.3 Ordinal Formatting

Components that display positional labels (e.g., "1st of 6", "Digit 3 of 4") require
ordinal-aware formatting. The `ordinal_label` field on relevant Messages structs
handles this:

```rust
/// Ordinal position label (e.g., "1st of 6", "Digit 3 of 4").
/// Requires CLDR ordinal data for correct localization.
/// Default English: |pos, total| format!("{pos} of {total}")
pub ordinal_label: MessageFn<dyn Fn(usize, usize) -> String + Send + Sync>
```

This field is present on `pin_input::Messages` and any component using positional labels.

---

### 6.4 Locale-Aware Text Case Transformation

Components that transform text case (e.g., uppercase labels, lowercase placeholders) MUST use locale-aware case mapping functions rather than Rust's default `.to_uppercase()` / `.to_lowercase()`, which operate on Unicode scalar values without locale context.

```rust
/// Locale-aware uppercase transformation.
/// Delegates to ICU4X CaseMapper for correct locale-specific mappings.
pub fn to_uppercase(text: &str, locale: &Locale) -> String {
    // CaseMapper::new() returns CaseMapperBorrowed<'static> (compiled data).
    // Methods take &LanguageIdentifier, not &DataLocale.
    // Methods return Cow<'_, str>, so .into_owned() converts to String.
    let case_mapper = icu::casemap::CaseMapper::new();
    case_mapper.uppercase_to_string(text, &locale.0.id).into_owned()
}

/// Locale-aware lowercase transformation.
pub fn to_lowercase(text: &str, locale: &Locale) -> String {
    let case_mapper = icu::casemap::CaseMapper::new();
    case_mapper.lowercase_to_string(text, &locale.0.id).into_owned()
}
```

**Notable locale-specific rules:**

| Locale            | Input             | `.to_uppercase()` (wrong) | `to_uppercase(_, locale)` (correct)          |
| ----------------- | ----------------- | ------------------------- | -------------------------------------------- |
| Turkish (`tr`)    | `"i"`             | `"I"`                     | `"İ"` (dotted capital I)                     |
| Turkish (`tr`)    | `"I"` (lowercase) | —                         | `"ı"` (dotless lowercase i)                  |
| German (`de`)     | `"ß"`             | `"SS"`                    | `"SS"` (same, but `ẞ` in some contexts)      |
| Lithuanian (`lt`) | `"i̇"`             | `"I"`                     | `"I"` (with accent preservation)             |
| Greek (`el`)      | `"σ"`             | `"Σ"`                     | `"Σ"` (final sigma rules apply in lowercase) |

Components that use case transformation include: Button (uppercase variant), Table (header labels), Breadcrumb (uppercase styling), and any component exposing a `text_transform` prop.

---

## 7. Translatable Component `Messages`

### 7.1 The `Messages` Pattern

Each component provides a messages struct with default English strings. Users override these to localize.

Each component's `xxx::Messages` struct provides `fn from_locale(locale: &Locale) -> Self` that returns locale-appropriate default messages for the top-10 supported locales (en, es, fr, de, ja, zh, ko, pt, ar, ru). For other locales, returns English defaults. Framework adapters provide a `MessagesProvider` context that components inherit from when no explicit `messages` prop is passed.

The `MessagesProvider` is exposed by each framework adapter (Leptos, Dioxus) as a context provider component. When a component does not receive an explicit `messages` prop, it reads from the nearest `MessagesProvider` in the component tree, which in turn calls `xxx::Messages::from_locale()` with the active locale. This enables zero-config localization for supported locales while still allowing full customization via explicit props.

**Normative rule — no hardcoded English in connect functions:**
Every `aria-label`, `aria-valuetext`, `aria-roledescription`, and live-region announcement string
that is set inside a connect function (e.g., `attrs.set(HtmlAttr::Aria(AriaAttr::Label), ...)`) **MUST**
read from the component's `Messages` struct (via `self.ctx.messages`). Hardcoding English string
literals directly in connect code is a spec violation because it bypasses the `Messages` override
mechanism and makes the string untranslatable. The only acceptable English strings are the
**defaults inside `impl Default for xxx::Messages`**, which serve as the en-US baseline.

**All `MessageFn` closures receive `&Locale`** so that each field can resolve locale-appropriate
text at call time. This means every message invocation in a connect function passes the active
locale. The three patterns for how locale reaches the call site are:

| Component type                                                       | Locale source                                                                                                                                  | Access pattern                                |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Stateful** (has `Machine` + `Context`)                             | Adapter constructs `Env { locale, icu_provider }`, passes to `Machine::init(props, &env, &messages)`. `init()` stores `env.locale` in Context. | `(self.ctx.messages.label)(&self.ctx.locale)` |
| **Stateless with `Api`** (no state machine, but has an `Api` struct) | Adapter passes `&Env` to `Api::new(props, env, messages)`. Api stores `&env.locale`.                                                           | `(self.messages.label)(self.locale)`          |
| **Standalone function** (no `Api` struct)                            | Adapter passes `locale: &Locale` directly as a function parameter.                                                                             | `(messages.label)(locale)`                    |

For stateful components, the adapter resolves locale from `ArsProvider` context,
places it in the `Env` struct, and passes `Env` to `Machine::init()`. The `init()`
function stores `env.locale.clone()` in the machine's `Context` struct. Connect
functions then access it via `self.ctx.locale`. See `01-architecture.md` §6.4.3
for the environment resolution rule.

**Closure fields** use `MessageFn<dyn Fn>` (`Arc` on native, `Rc` on WASM — not `Box`) so the struct remains `Clone`.
`MessageFn<T>` implements `Debug` by printing `"<closure>"`, so all Messages structs
can `#[derive(Clone, Debug)]` uniformly — no manual `Debug` impls needed.

`MessageFn` lives in `ars-core` (not `ars-i18n`) and wraps [`ArsRc`](01-architecture.md)
internally, eliminating cfg-gated code in its trait impls. `Clone`, `PartialEq`, `Deref`,
and `AsRef` all delegate to `ArsRc`; only `Debug` is custom (prints `"<closure>"`).

**`+ Send + Sync` bounds:** All `MessageFn` trait objects include `+ Send + Sync` on
all targets as a deliberate project-wide convention. On WASM the `Rc` wrapper is
non-atomic, but the trait object bounds remain `Send + Sync` so that the public API
is identical across native and WASM — closures must be thread-safe for native desktop
targets (multi-threaded runtimes in Dioxus Desktop, Tauri). The `Rc` wrapper satisfies
WASM's single-threaded `Send`/`Sync` auto-impl. Do not remove these bounds.

```rust
/// Shared function pointer for Messages closure fields.
/// Wraps `ArsRc<T>` — `Rc` on WASM, `Arc` on native — so no cfg-gated
/// trait impls are needed in this type. Only `Debug` is custom.
pub struct MessageFn<T: ?Sized>(ArsRc<T>);

impl<T: ?Sized> Clone for MessageFn<T> {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}

impl<T: ?Sized> core::fmt::Debug for MessageFn<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("<closure>")
    }
}

// PartialEq by pointer identity — enables derive(PartialEq) on Messages structs.
impl<T: ?Sized> PartialEq for MessageFn<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl<T: ?Sized> core::ops::Deref for MessageFn<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

impl<T: ?Sized> AsRef<T> for MessageFn<T> {
    fn as_ref(&self) -> &T { self.0.as_ref() }
}

impl<T: ?Sized> MessageFn<T> {
    pub fn new(f: impl Into<Self>) -> Self { f.into() }
}

// From impls — each closure signature needs a cfg-gated pair because
// they create ArsRc from raw Rc/Arc for dyn trait object coercion.
// One pair per distinct MessageFn signature used across the component library.

#[cfg(target_arch = "wasm32")]
impl<F: Fn() -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn() -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn() -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn() -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(&Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(&Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(usize, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(usize, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(f64, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(f64, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(&str, SortDirection, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, SortDirection, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(&str, SortDirection, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, SortDirection, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

// From impls for CalendarMessages (shared/date-time-types.md)
#[cfg(target_arch = "wasm32")]
impl<F: Fn(u8, u8, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, u8, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(u8, u8, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, u8, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(u8, u8, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, u8, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(u8, u8, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, u8, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(u8, i32, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, i32, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(u8, i32, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(u8, i32, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(&str, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(&str, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(i32, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(i32, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(i32, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(i32, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

// From impls for DragAnnouncements (05-interactions.md §7.8)
#[cfg(target_arch = "wasm32")]
impl<F: Fn(&str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(&str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(usize, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(usize, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Rc::new(f))) }
}
#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(usize, &str, &Locale) -> String + Send + Sync + 'static> From<F> for MessageFn<dyn Fn(usize, &str, &Locale) -> String + Send + Sync> {
    fn from(f: F) -> Self { Self(ArsRc(Arc::new(f))) }
}

impl MessageFn<dyn Fn(&Locale) -> String + Send + Sync> {
    /// Create a MessageFn from a static string, ignoring the locale parameter.
    /// The locale parameter is ignored — use this for English baselines in Default impls.
    pub fn static_str(s: &'static str) -> Self {
        Self::new(move |_locale: &Locale| s.to_string())
    }
}
```

**Signature conventions** for `MessageFn` fields:

- `MessageFn<dyn Fn(&Locale) -> String + Send + Sync>` — for all label fields; every message receives the active locale so users can provide per-locale translations
- `MessageFn<dyn Fn(param, &Locale) -> String + Send + Sync>` — for parameterized messages where additional arguments carry format data (e.g., `MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>` for plural-aware counts, `MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>` for string interpolation)

> **Design note — All `MessageFn` trait objects include `+ Send + Sync`.**
> This avoids cfg-gated dual struct definitions for every `*Messages` type. On WASM,
> `Rc<dyn Fn(...) + Send + Sync>` is valid — the bound constrains the closure type, not
> the wrapper. Most closures satisfy `Send + Sync` by default. If WASM gains threading,
> the migration to `Arc` is a single-line change in the `MessageFn` typedef.

```rust
/// The pattern all component message structs follow.
pub trait ComponentMessages: Clone + Default {}
```

**Example** (from `spec/components/selection/select.md`):

```rust
/// Messages for the Select component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Placeholder when nothing is selected.
    pub placeholder: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Message when dropdown has no items.
    pub empty: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Plural-aware count announcement. Accepts count AND locale.
    pub selected_count: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// aria-label for the clear trigger button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// aria-label for the trigger button.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            placeholder: MessageFn::static_str("Select an option"),
            empty: MessageFn::static_str("No options available"),
            selected_count: MessageFn::new(|n, _locale| {
                match n {
                    1 => "1 option selected".into(),
                    _ => format!("{} options selected", n),
                }
            }),
            clear_label: MessageFn::static_str("Clear selection"),
            trigger_label: MessageFn::static_str("Open dropdown"),
        }
    }
}

impl ComponentMessages for Messages {}
```

**Component Messages are defined in each component's spec file.** Every component that
produces localizable strings has a `Messages` section in its own spec
(`spec/components/{category}/{component}.md`). The component file is the **single source
of truth** for its Messages struct — field names, types, defaults, and documentation.
This foundation file defines the pattern (`ComponentMessages` trait, `MessageFn` type,
`static_str` helper, signature conventions) but does not duplicate the per-component
definitions.

To find a component's Messages, look for the "Messages" heading in its spec file,
or use `cargo run -p spec-tool -- deps <component>` to locate the file.

### 7.2 Localized Messages Registry

```rust
/// A registry of pre-built message sets for common locales.
pub struct MessagesRegistry<M: ComponentMessages> {
    messages: alloc::collections::BTreeMap<String, M>,
    default: M,
}

impl<M: ComponentMessages> MessagesRegistry<M> {
    pub fn new(default: M) -> Self {
        Self { messages: Default::default(), default }
    }

    pub fn register(mut self, locale_tag: &str, messages: M) -> Self {
        self.messages.insert(locale_tag.to_string(), messages);
        self
    }

    /// Retrieve messages for the given locale, falling back through
    /// the locale's tag hierarchy:
    ///   full BCP 47 tag → language+script → language-only → default.
    ///
    /// The language+script level is critical for CJK locales:
    /// `zh-Hant-TW` → `zh-Hant` → `zh` → default
    /// Without this, Traditional Chinese users would fall through to
    /// Simplified Chinese or English.
    ///
    /// For full fallback chain support (e.g., "pt-BR" → "pt" → "en"),
    /// use `get_with_stack(&LocaleStack)` which follows the canonical
    /// locale resolution order.
    pub fn get(&self, locale: &Locale) -> &M {
        // Level 1: Full BCP 47 tag (e.g., "zh-Hant-TW")
        if let Some(m) = self.messages.get(&locale.to_bcp47()) {
            return m;
        }
        // Level 2: Language + script (e.g., "zh-Hant") — critical for CJK
        if let Some(script) = locale.script() {
            let lang_script = format!("{}-{}", locale.language(), script);
            if let Some(m) = self.messages.get(&lang_script) {
                return m;
            }
        }
        // Level 3: Language only (e.g., "zh")
        if let Some(m) = self.messages.get(locale.language()) {
            return m;
        }
        &self.default
    }
}
```

### 7.3 I18n Registries and Global Message Resolution

The adapter resolves messages through a three-level chain — matching the locale resolution pattern:

1. **Adapter prop override** — `messages: Option<Messages>` on the adapter-level component (highest priority)
2. **ArsProvider context** — Application-level `I18nRegistries` bundled in `ArsContext`
3. **Built-in defaults** — `Messages::default()` (English fallbacks)

````rust
/// Type-erased storage for per-component message registries.
/// Keyed by `TypeId` of each component's `Messages` struct so components
/// can look up their own translations without knowing about other components.
pub struct I18nRegistries {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl I18nRegistries {
    pub fn new() -> Self { Self { map: HashMap::new() } }

    /// Register a `MessagesRegistry` for a specific component's Messages type.
    ///
    /// ```rust
    /// let mut registries = I18nRegistries::new();
    /// registries.register::<dialog::Messages>(dialog_registry);
    /// registries.register::<select::Messages>(select_registry);
    /// ```
    pub fn register<M: ComponentMessages + 'static>(&mut self, registry: MessagesRegistry<M>) {
        self.map.insert(TypeId::of::<M>(), Box::new(registry));
    }

    /// Look up the `MessagesRegistry` for a component's Messages type.
    pub fn get<M: ComponentMessages + 'static>(&self) -> Option<&MessagesRegistry<M>> {
        self.map.get(&TypeId::of::<M>())
            .and_then(|b| b.downcast_ref::<MessagesRegistry<M>>())
    }
}
````

**Canonical resolution function** — a **pure function** called by the adapter before constructing
a `Service` or `Api`. Takes `registries` as an explicit parameter; does NOT call `use_context()`.
Can live in `ars-i18n` or `ars-core`.

```rust
/// Resolve messages for a component following the three-level chain.
///
/// This is a pure function — no framework hooks. The adapter passes
/// `registries` explicitly after reading them from `ArsContext`.
///
/// - `adapter_props_messages`: The optional per-instance override from the adapter's Props.
/// - `registries`: The `I18nRegistries` from `ArsContext`.
/// - `locale`: The resolved locale (from `resolve_locale()`).
fn resolve_messages<M: ComponentMessages + 'static>(
    adapter_props_messages: Option<&M>,
    registries: &I18nRegistries,
    locale: &Locale,
) -> M {
    // Level 1: explicit adapter prop override
    if let Some(m) = adapter_props_messages {
        return m.clone();
    }
    // Level 2: ArsProvider i18n registries
    if let Some(registry) = registries.get::<M>() {
        return registry.get(locale).clone();
    }
    // Level 3: built-in defaults
    M::default()
}
```

**Core component Props pattern** — Props contain only behavioral configuration.
Locale and messages are NOT in Props; they arrive via `Env` and `Messages` parameters:

```rust
// Core Props: NO locale or messages fields.
pub struct Props {
    pub id: String,
    // ... behavioral configuration only ...
}

// init(): receives Env and Messages from the adapter.
fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
    let locale = env.locale.clone();
    let messages = messages.clone();
    // ...
}
```

**Adapter usage** — the adapter resolves environment values and passes them to core:

```rust
// In Leptos/Dioxus adapter component:
let locale = resolve_locale(adapter_props.locale.as_ref());
let icu_provider = use_icu_provider();
let registries = use_i18n_registries();
let messages = resolve_messages::<dialog::Messages>(
    adapter_props.messages.as_ref(), &registries, &locale,
);
let env = Env { locale, icu_provider };
let service = Service::new(core_props, env, messages);
```

**Application setup:**

```rust
// Build i18n registries and pass to ArsProvider
let mut registries = I18nRegistries::new();

// Spanish Dialog messages
registries.register(MessagesRegistry::new(dialog::Messages::default())
    .register("es", dialog::Messages {
        close_label: MessageFn::static_str("Cerrar"),
        ..Default::default()
    }));

// Spanish Select messages
registries.register(MessagesRegistry::new(select::Messages::default())
    .register("es", select::Messages {
        placeholder: MessageFn::static_str("Seleccionar..."),
        ..Default::default()
    }));

// Pass registries to ArsProvider — all descendants inherit these translations
// ArsProvider { i18n_registries: Some(Rc::new(registries)), .. }
```

### 7.4 User-Defined Translatable Text

The `ComponentMessages` system (§7.1–7.3) handles translations for **library component** text.
For **application-level** text — page titles, custom labels, descriptions, user-facing copy — users
implement the `Translate` trait on their own enums and use the `t()` function from the adapter
prelude to render them reactively in views.

#### 7.4.1 The `Translate` Trait

```rust
/// Trait for user-defined translatable text.
///
/// Users define an enum with one variant per translatable string.
/// Data-carrying variants support parameterized text (plurals, interpolation).
/// The `t()` function (from adapter prelude) wraps enum variants for reactive
/// rendering in views — locale and ICU provider are resolved from ArsProvider context.
///
/// Named following Rust verb-form trait conventions (`Clone`, `Display`, `Send`).
pub trait Translate {
    /// Produce the localized text for this variant.
    ///
    /// - `locale`: The active locale (from `ArsProvider` / `use_locale()`)
    /// - `icu`: The ICU data provider (from `ArsProvider` / `use_icu_provider()`)
    fn translate(&self, locale: &Locale, icu: &dyn IcuProvider) -> String;
}
```

#### 7.4.2 Conventions

- **One enum per page/domain/feature** (e.g., `Inventory`, `Checkout`, `AdminDashboard`) — not one giant enum for the whole app.
- **Unit variants** for static text, **data-carrying variants** for parameterized text (plurals, interpolation, formatted values).
- **Match on locale first, then on `self`** — this groups all strings for the same language together, making it easy to review and maintain a single language at a time.
- **Always include a fallback arm** (`_` → English) as the last locale match arm.
- Use `ars_i18n::select_plural()` for plural-aware variants.
- Use `ars_i18n::NumberFormatter` / `DateFormatter` within `translate()` for locale-aware number and date formatting.
- The `icu` parameter provides access to calendar data, plural rules, and other CLDR data needed inside `translate()`.

#### 7.4.3 Worked Example

```rust
use ars_i18n::{Translate, Locale, IcuProvider, PluralCategory, PluralRuleType};

/// All translatable text for the inventory page.
enum Inventory {
    Title,
    Welcome,
    ItemCount { count: usize },
}

impl Translate for Inventory {
    fn translate(&self, locale: &Locale, icu: &dyn IcuProvider) -> String {
        match locale.language().as_str() {
            "es" => match self {
                Self::Title => "Inventario".into(),
                Self::Welcome => "¡Bienvenido!".into(),
                Self::ItemCount { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} elemento"),
                        _ => format!("{count} elementos"),
                    }
                }
            },
            "fr" => match self {
                Self::Title => "Inventaire".into(),
                Self::Welcome => "Bienvenue !".into(),
                Self::ItemCount { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} élément"),
                        _ => format!("{count} éléments"),
                    }
                }
            },
            _ => match self { // English fallback
                Self::Title => "Inventory".into(),
                Self::Welcome => "Welcome!".into(),
                Self::ItemCount { count } => {
                    let cat = ars_i18n::select_plural(
                        locale, *count as f64, PluralRuleType::Cardinal,
                    );
                    match cat {
                        PluralCategory::One => format!("{count} item"),
                        _ => format!("{count} items"),
                    }
                }
            },
        }
    }
}
```

**Using in Leptos views:**

```rust
use ars_leptos::prelude::*; // t() is in the prelude

fn InventoryPage(item_count: usize) -> impl IntoView {
    view! {
        <h1>{t(Inventory::Title)}</h1>
        <p>{t(Inventory::Welcome)}</p>
        <span>{t(Inventory::ItemCount { count: item_count })}</span>
    }
}
```

**Using in Dioxus views:**

```rust
use ars_dioxus::prelude::*; // t() is in the prelude

fn InventoryPage(item_count: usize) -> Element {
    rsx! {
        h1 { {t(Inventory::Title)} }
        p { {t(Inventory::Welcome)} }
        span { {t(Inventory::ItemCount { count: item_count })} }
    }
}
```

#### 7.4.4 Multiple Domains

Users can define multiple enums for different parts of their application. Each enum is a
self-contained translation domain:

```rust
enum Checkout { Total, ShippingLabel, ConfirmButton }
enum AdminDashboard { Title, UserCount { count: usize } }
```

There is no registration step — unlike `ComponentMessages` which uses `I18nRegistries`,
`Translate` enums are resolved directly via `t()` at render time. The `t()` function
reads locale and ICU provider from `ArsProvider` context and calls `translate()` immediately.

#### 7.4.5 Relationship to ComponentMessages

| System                                 | Purpose                                                          | Resolution                                                     |
| -------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------- |
| `ComponentMessages` + `I18nRegistries` | Override **library component** messages at runtime               | Registry lookup via `resolve_messages()` in component `init()` |
| `Translate` + `t()`                    | User-defined **application text** with compile-time translations | Direct `translate()` call via `t()` at render time             |

Both systems share the same locale context from `ArsProvider`. They are complementary
and do not interact — users never register `Translate` enums in `I18nRegistries`.

### 7.5 The `t()` Function

The `t()` function is the adapter-specific bridge between `Translate` enums and the
framework's reactive rendering system. It reads locale and ICU provider from `ArsProvider`
context and produces a reactive text node.

```rust
/// Resolve a `Translate` value into a reactive text node for rendering.
///
/// Reads the current locale and ICU provider from `ArsProvider` context,
/// calls `msg.translate()`, and returns a framework-specific reactive view
/// that updates when the locale changes.
///
/// Included in `ars_leptos::prelude` and `ars_dioxus::prelude`.
///
/// # Fallback
///
/// If no `ArsProvider` is present in the ancestor tree, falls back to
/// `en-US` locale and `StubIcuProvider` — matching the default behavior
/// of `use_locale()` and `use_icu_provider()`.
///
/// # Reactivity
///
/// - **Leptos:** Returns a reactive closure. The closure subscribes to the
///   locale signal; when locale changes, only the text node re-evaluates
///   (fine-grained reactivity).
/// - **Dioxus:** Returns a `String` evaluated during render. The `Signal::read()`
///   call inside `t()` subscribes the calling component to locale changes;
///   when locale changes, the component re-renders and `t()` produces the
///   new string (component-level reactivity).
#[inline]
#[must_use]
pub fn t<T: Translate>(msg: T) -> /* adapter-specific return type */;
```

Each adapter provides its own `t()` implementation — see §10.1 (Leptos) and §10.2 (Dioxus)
in the adapter specs for the concrete signatures.

---

## 8. Collation (Locale-Aware String Sorting)

```rust
use icu::collator::{Collator as OwnedCollator, CollatorOptions as IcuCollatorOptions, Strength};

/// Options for locale-aware string comparison.
#[derive(Clone, Debug)]
pub struct CollationOptions {
    /// Comparison strength.
    pub strength: CollationStrength,
    /// Whether to ignore case. When `true`, overrides `strength` to at least
    /// `Secondary` (ignoring case requires Secondary or weaker). If `strength`
    /// is already `Primary`, it will be raised to `Secondary`.
    pub case_insensitive: bool,
    /// Whether to treat numeric substrings numerically.
    /// "file10" comes after "file9" (not before as lexicographic).
    pub numeric: bool,
}

// Individual components may override this default in their props (e.g., Table uses Secondary, Combobox uses Primary).
// This is the canonical definition. `06-collections.md` §7.3 re-exports it:
//   #[cfg(feature = "i18n")] pub use ars_i18n::CollationStrength;
//   #[cfg(not(feature = "i18n"))] // fallback definition
#[cfg(feature = "icu4x")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CollationStrength {
    Primary,    // Ignore accents and case
    Secondary,  // Ignore case, respect accents
    #[default]
    Tertiary,   // Respect accents and case
    Quaternary, // Also respect punctuation
}

/// Fallback for consumers without the `icu4x` feature. Provides the same
/// enum variants so downstream code compiles unconditionally. Sorting
/// degrades to Rust's default `Ord` (byte-order) comparison.
#[cfg(not(feature = "icu4x"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CollationStrength {
    Primary,
    Secondary,
    #[default]
    Tertiary,
    Quaternary,
}

impl Default for CollationOptions {
    fn default() -> Self {
        Self {
            strength: CollationStrength::Tertiary,
            // Tertiary strength already respects case; set true only to override
            case_insensitive: false,
            numeric: true,
        }
    }
}

/// A locale-aware string collator.
/// ICU4X 2.x: `Collator::try_new()` returns `CollatorBorrowed<'static>`.
/// We store the owned `Collator` (via `.static_to_owned()`) and call
/// `.as_borrowed()` for comparison operations.
pub struct StringCollator {
    locale: Locale,
    collator: OwnedCollator,
}

impl StringCollator {
    /// Create a new locale-aware string collator.
    ///
    /// With the `compiled_data` feature (our default), `Collator::try_new()`
    /// cannot fail — CLDR collation data is baked into the binary for all locales.
    pub fn new(locale: &Locale, options: CollationOptions) -> Self {
        let mut icu_opts = IcuCollatorOptions::default();
        icu_opts.strength = Some(match options.strength {
            CollationStrength::Primary => Strength::Primary,
            CollationStrength::Secondary => Strength::Secondary,
            CollationStrength::Tertiary => Strength::Tertiary,
            CollationStrength::Quaternary => Strength::Quaternary,
        });
        // case_insensitive = true forces strength to Secondary, overriding `strength`.
        // To set a specific strength, leave case_insensitive = false and set strength directly.
        if options.case_insensitive {
            icu_opts.strength = Some(Strength::Secondary);
        }
        // ICU4X 2.x: numeric collation is set via CollatorPreferences.
        let mut prefs = icu::collator::CollatorPreferences::from(&locale.0);
        if options.numeric && prefs.numeric_ordering.is_none() {
            // Only set if the locale doesn't already specify a -u-kn- preference.
            // This avoids silently overriding an explicit locale `-u-kn-false` tag.
            prefs.numeric_ordering = Some(icu::collator::preferences::CollationNumericOrdering::True);
        }

        // ICU4X 2.x: Collator::try_new returns CollatorBorrowed<'static>.
        // Call .static_to_owned() to get an owned Collator for storage in structs.
        let collator = OwnedCollator::try_new(prefs, icu_opts)
            .expect("compiled_data guarantees collation data is available for all locales")
            .static_to_owned();

        Self { locale: locale.clone(), collator }
    }

    /// Compare two strings according to locale rules.
    pub fn compare(&self, a: &str, b: &str) -> core::cmp::Ordering {
        self.collator.as_borrowed().compare(a, b)
    }

    /// Sort a slice of strings in-place according to locale rules.
    pub fn sort(&self, items: &mut Vec<String>) {
        items.sort_by(|a, b| self.compare(a, b));
    }

    /// Sort items by a key function according to locale rules.
    pub fn sort_by_key<T, F: Fn(&T) -> &str>(&self, items: &mut Vec<T>, key: F) {
        items.sort_by(|a, b| self.compare(key(a), key(b)));
    }
}
```

---

## 9. ICU4X Integration Strategy

### 9.1 Crate Dependencies

```toml
# ars-i18n/Cargo.toml
[dependencies]
# Umbrella crate — re-exports icu::calendar, icu::collator, icu::datetime, etc.
# All code uses icu::* paths; no need for individual icu_* crates in [dependencies].
icu = { version = "2.1", features = ["serde"] }
icu_experimental = "0.4"  # Contains relativetime (not yet in umbrella)
fixed_decimal = "0.7"     # Main type is Decimal (not FixedDecimal)

# Infrastructure crates — not re-exported by the umbrella
icu_provider = "2.1"
icu_datagen = { version = "2.1", optional = true }

[features]
default = ["icu4x", "compiled-data", "gregorian"]
icu4x = ["dep:icu"]
web-intl = ["dep:wasm-bindgen", "dep:js-sys"]
compiled-data = ["icu/compiled_data"]

# Calendar feature flags — include only what you need
gregorian = []     # Always included
buddhist = ["icu/icu_calendar_buddhist"]
japanese = ["icu/icu_calendar_japanese"]
hebrew = ["icu/icu_calendar_hebrew"]
islamic = ["icu/icu_calendar_islamic"]
persian = ["icu/icu_calendar_persian"]
ethiopic = ["icu/icu_calendar_ethiopic"]
indian = ["icu/icu_calendar_indian"]
chinese = ["icu/icu_calendar_chinese"]
dangi = ["icu/icu_calendar_dangi"]
coptic = ["icu/icu_calendar_coptic"]
all-calendars = ["buddhist", "japanese", "hebrew", "islamic", "persian", "ethiopic", "indian", "chinese", "dangi", "coptic"]
```

### 9.2 Compiled Data for WASM

```rust
// For minimal WASM binary size, use compiled/baked data.
// This is generated at build time and linked in statically.

// In a build.rs (optional — for custom locale data sets):
// icu_datagen generates a Rust file with embedded data.

// Usage (automatically used when "compiled-data" feature is on):
// ICU4X formatters use compiled baked data internally (via each crate's `compiled_data` feature).

// Estimate WASM binary size impact:
// - Gregorian calendar only: ~200KB
// - All calendars: ~500KB
// - Full CLDR data: ~2MB
// Feature flags let users trade functionality for size.
```

### 9.3 Lazy-Loaded Formatters

```rust
/// Cache formatters keyed by locale + options to avoid re-creation.
///
/// Note: This cache uses `std::sync::Mutex`, which requires `std`.
/// On `no_std` targets (bare `alloc`), formatters must be constructed
/// per-call or cached by the application using thread-local storage.
#[cfg(feature = "std")]
static NUMBER_FORMATTER_CACHE: std::sync::LazyLock<
    std::sync::Mutex<alloc::collections::BTreeMap<String, NumberFormatter>>
> = std::sync::LazyLock::new(Default::default);

#[cfg(feature = "std")]
pub fn get_number_formatter(locale: &Locale, options: &NumberFormatOptions) -> NumberFormatter {
    let key = format!("{:?}-{:?}", locale.to_bcp47(), options);
    let mut cache = NUMBER_FORMATTER_CACHE.lock().expect("NumberFormatter cache lock poisoned");
    if let Some(existing) = cache.get(&key) {
        return existing.clone();
    }
    let formatter = NumberFormatter::new(locale, options.clone());
    cache.insert(key, formatter.clone());
    formatter
}
```

### 9.4 Browser Intl API Feature Flag (`web-intl`)

For WASM client builds, the browser's `Intl` API provides the same formatting capabilities as ICU4X with zero bundle size overhead. The `web-intl` feature flag enables this backend.

#### 9.4.1 Feature-flagged type aliases

```rust
// ars-i18n/src/lib.rs — zero-cost dispatch via cfg type aliases

// ── Number formatting ──
#[cfg(feature = "icu4x")]
pub type DefaultNumberFormatter = icu4x::Icu4xNumberFormatter;
#[cfg(feature = "web-intl")]
pub type DefaultNumberFormatter = web_intl::JsIntlNumberFormatter;

// ── Date formatting ──
#[cfg(feature = "icu4x")]
pub type DefaultDateFormatter = icu4x::Icu4xDateFormatter;
#[cfg(feature = "web-intl")]
pub type DefaultDateFormatter = web_intl::JsIntlDateFormatter;

// ── Plural rules ──
#[cfg(feature = "icu4x")]
pub type DefaultPluralRules = icu4x::Icu4xPluralRules;
#[cfg(feature = "web-intl")]
pub type DefaultPluralRules = web_intl::JsIntlPluralRules;
```

#### 9.4.2 Shared traits

Both backends implement common traits:

```rust
pub trait NumberFormat {
    fn format(&self, value: f64) -> String;
}

pub trait DateFormat {
    fn format(&self, date: &Date) -> String;
}

pub trait PluralRulesFormat {
    fn select(&self, number: f64) -> PluralCategory;
}
```

#### 9.4.3 `web-intl` backend sketch

```rust
#[cfg(feature = "web-intl")]
pub struct JsIntlNumberFormatter {
    inner: js_sys::Intl::NumberFormat,
}

#[cfg(feature = "web-intl")]
impl JsIntlNumberFormatter {
    pub fn new(locale: &str, options: &NumberFormatOptions) -> Self {
        use js_sys::{Array, Object, Reflect, Intl};
        use wasm_bindgen::JsValue;

        let locales = Array::of1(&JsValue::from_str(locale));
        let js_opts = Object::new();

        // Map our options to Intl.NumberFormat options.
        Reflect::set(&js_opts, &"minimumFractionDigits".into(),
            &JsValue::from_f64(options.min_fraction_digits as f64)).expect("Reflect::set on JS object");
        Reflect::set(&js_opts, &"maximumFractionDigits".into(),
            &JsValue::from_f64(options.max_fraction_digits as f64)).expect("Reflect::set on JS object");
        Reflect::set(&js_opts, &"useGrouping".into(),
            &JsValue::from_bool(options.use_grouping)).expect("Reflect::set on JS object");

        let inner = Intl::NumberFormat::new(&locales, &js_opts);
        Self { inner }
    }
}

#[cfg(feature = "web-intl")]
impl NumberFormat for JsIntlNumberFormatter {
    fn format(&self, value: f64) -> String {
        self.inner.format(value).as_string().unwrap_or_default()
    }
}
```

#### 9.4.4 Feature combination guidance

| Environment           | Feature    | Rationale                                 |
| --------------------- | ---------- | ----------------------------------------- |
| SSR / Desktop         | `icu4x`    | No browser APIs available                 |
| WASM client           | `web-intl` | Smaller bundle, browser provides ICU data |
| WASM client (offline) | `icu4x`    | Needs built-in ICU data                   |

> **Note**: `parse()` (string -> number/date) is not available via the browser `Intl` API. Applications that need parsing must use the `icu4x` backend even on the client.

### 9.5 Calendar/Locale Provider Trait (`IcuProvider`)

Date-time components need locale-aware calendar data (weekday names, month names, day
period labels, digit formatting, month counts, hour cycles, first-day-of-week). Rather
than coupling directly to ICU4X types, all calendar/locale queries go through the
`IcuProvider` trait. This gives us:

- **Testability** — unit tests use `StubIcuProvider` (English-only, zero dependencies).
- **Flexibility** — production uses `Icu4xProvider` backed by CLDR data; WASM clients
  could use a `WebIntlProvider` that delegates to the browser.
- **Feature-flag gating** — the `icu4x` feature enables `Icu4xProvider`; without it,
  only the stub is available.

```rust
// ars-i18n/src/provider.rs

use crate::{CalendarSystem, HourCycle, Locale, Weekday};
use crate::shared::CalendarDate;

/// Trait abstracting ICU4X data provider for calendar/locale operations.
///
/// Production uses `Icu4xProvider` with CLDR data; tests use `StubIcuProvider`.
/// All date-time components accept `&dyn IcuProvider` so they remain
/// backend-agnostic.
// On native targets, IcuProvider must be Send + Sync for multi-threaded runtimes.
// On WASM (single-threaded), these bounds are omitted to allow Rc-based components.
#[cfg(not(target_arch = "wasm32"))]
pub trait IcuProvider: Send + Sync + 'static {
    /// Short weekday label (abbreviated format): "Mo", "Tu", "We", …
    /// ICU4X: `DateSymbols::weekday_names(FieldLength::Abbreviated)`
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String;

    /// Long weekday label (wide format): "Monday", "Tuesday", …
    /// ICU4X: `DateSymbols::weekday_names(FieldLength::Wide)`
    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String;

    /// Full month name: "January", "Février", "مارس", …
    /// ICU4X: `DateSymbols::month_names(FieldLength::Wide)`
    fn month_long_name(&self, month: u8, locale: &Locale) -> String;

    /// AM/PM label for the locale: "AM"/"PM", "午前"/"午後", "ص"/"م", …
    /// ICU4X 2.x: Extract by formatting known AM/PM times via `NoCalendarFormatter` with `T::hm()`.
    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String;

    /// Map a typed character to a day-period value: 'a'→Some(false), 'p'→Some(true).
    /// Locale-aware: Arabic ص→AM, م→PM; CJK input not character-based so returns None.
    /// ICU4X 2.x: Reverse-lookup against first characters of formatted AM/PM labels.
    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool>;

    /// Format a numeric segment with locale-appropriate digits and zero-padding.
    /// Arabic-Indic ٠١٢٣٤٥٦٧٨٩, Persian ۰۱۲۳۴۵۶۷۸۹, Bengali ০১২৩৪৫৬৭৮৯, etc.
    /// ICU4X: `DecimalFormatter` with the locale's default numbering system.
    fn format_segment_digits(&self, value: u32, min_digits: NonZero<u8>, locale: &Locale) -> String;

    /// Maximum number of months in a year for the given calendar and year.
    /// Most calendars return 12; Hebrew leap years return 13; Chinese/Dangi may return 13.
    /// ICU4X: Constructs a date via `AnyCalendar::date_from_codes()` then calls `months_in_year()`.
    /// `era` is required for multi-era calendars (Japanese, Ethiopic); pass `None` for others.
    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8;

    /// Days in a specific month for the given calendar, year, and month.
    /// ICU4X: Constructs a date via `AnyCalendar::date_from_codes()` then calls `days_in_month()`.
    /// `era` is required for multi-era calendars (Japanese, Ethiopic); pass `None` for others.
    fn days_in_month(&self, calendar: &CalendarSystem, year: i32, month: u8, era: Option<&str>) -> u8;

    /// Preferred hour cycle for the locale (H12, H23, etc.).
    /// ICU4X: `HourCycle` preference from CLDR `timeData` via `icu::datetime`.
    fn hour_cycle(&self, locale: &Locale) -> HourCycle;

    /// First day of the week for the locale (Monday, Saturday, Sunday, etc.).
    /// ICU4X: `WeekInformation::first_weekday` field via CLDR `weekData`.
    fn first_day_of_week(&self, locale: &Locale) -> Weekday;

    /// Convert a `CalendarDate` from its current calendar system to the
    /// `target` calendar system. Used by `CalendarDate::to_calendar()` for
    /// non-Gregorian conversions.
    /// ICU4X: `AnyCalendar::convert()` via `icu::calendar`.
    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate;
}

#[cfg(target_arch = "wasm32")]
pub trait IcuProvider: 'static {
    // Same methods as above — duplicated via cfg. In implementation, use a macro
    // to avoid maintaining two copies:
    //   macro_rules! icu_provider_methods { ... }
    //   #[cfg(not(wasm32))] pub trait IcuProvider: Send + Sync + 'static { icu_provider_methods!(); }
    //   #[cfg(wasm32)]      pub trait IcuProvider: 'static { icu_provider_methods!(); }
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String;
    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String;
    fn month_long_name(&self, month: u8, locale: &Locale) -> String;
    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String;
    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool>;
    fn format_segment_digits(&self, value: u32, min_digits: NonZero<u8>, locale: &Locale) -> String;
    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8;
    fn days_in_month(&self, calendar: &CalendarSystem, year: i32, month: u8, era: Option<&str>) -> u8;
    fn hour_cycle(&self, locale: &Locale) -> HourCycle;
    fn first_day_of_week(&self, locale: &Locale) -> Weekday;
    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate;
}
```

#### 9.5.1 Stub implementation (test/no-ICU4X builds)

The stub preserves the current English-only behavior. It is the default when the `icu4x`
feature is disabled.

```rust
// ars-i18n/src/provider/stub.rs

/// English-only stub provider for tests and no-ICU4X builds.
///
/// All methods return hardcoded English values, matching the behavior of the
/// original non-ICU4X implementations. This is the default provider when the
/// `icu4x` feature flag is not enabled.
pub struct StubIcuProvider;

impl IcuProvider for StubIcuProvider {
    fn weekday_short_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday    => "Su",
            Weekday::Monday    => "Mo",
            Weekday::Tuesday   => "Tu",
            Weekday::Wednesday => "We",
            Weekday::Thursday  => "Th",
            Weekday::Friday    => "Fr",
            Weekday::Saturday  => "Sa",
        }.to_string()
    }

    fn weekday_long_label(&self, weekday: Weekday, _locale: &Locale) -> String {
        match weekday {
            Weekday::Sunday    => "Sunday",
            Weekday::Monday    => "Monday",
            Weekday::Tuesday   => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday  => "Thursday",
            Weekday::Friday    => "Friday",
            Weekday::Saturday  => "Saturday",
        }.to_string()
    }

    fn month_long_name(&self, month: u8, _locale: &Locale) -> String {
        match month {
            1 => "January", 2 => "February", 3 => "March",
            4 => "April", 5 => "May", 6 => "June",
            7 => "July", 8 => "August", 9 => "September",
            10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown",
        }.to_string()
    }

    fn day_period_label(&self, is_pm: bool, _locale: &Locale) -> String {
        if is_pm { "PM".to_string() } else { "AM".to_string() }
    }

    fn day_period_from_char(&self, ch: char, _locale: &Locale) -> Option<bool> {
        match ch.to_ascii_lowercase() {
            'a' => Some(false), // AM
            'p' => Some(true),  // PM
            _ => None,
        }
    }

    fn format_segment_digits(&self, value: u32, min_digits: NonZero<u8>, _locale: &Locale) -> String {
        match min_digits.get() {
            4 => format!("{:04}", value),
            2 => format!("{:02}", value),
            _ => format!("{}", value),
        }
    }

    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, _era: Option<&str>) -> u8 {
        // Simplified: only handles Gregorian-like (12) and Hebrew leap (13).
        // Production Icu4xProvider handles all calendar systems precisely.
        // Over-permissive: Chinese/Dangi have 13 months only in leap years.
        // Stub accepts month 13 in all years for simplicity.
        match calendar {
            // No cfg gates — CalendarSystem enum is always fully available (runtime dispatch).
            CalendarSystem::Gregorian => 12,
            CalendarSystem::Hebrew => {
                let cycle_year = year.rem_euclid(19);
                // Metonic cycle leap years: 3, 6, 8, 11, 14, 17, 19
                // Here 0 == 19 mod 19, i.e., year 19 of the cycle
                if [3, 6, 8, 11, 14, 17, 0 /* = year 19 */].contains(&cycle_year) { 13 } else { 12 }
            }
            CalendarSystem::Ethiopic => 13,
            CalendarSystem::Coptic => 13,
            CalendarSystem::Chinese => 13,
            CalendarSystem::Dangi => 13,
            _ => 12,
        }
    }

    fn days_in_month(&self, calendar: &CalendarSystem, year: i32, month: u8, _era: Option<&str>) -> u8 {
        // Simplified: Gregorian-only logic. Production Icu4xProvider delegates
        // to ICU4X AnyCalendar for all calendar systems.
        // gregorian_days_in_month is defined in ars-core (shared/date-time-types.md §1.1)
        use ars_core::date_time::gregorian_days_in_month;
        match calendar {
            CalendarSystem::Gregorian => gregorian_days_in_month(year, month),
            _ => gregorian_days_in_month(year, month), // fallback
        }
    }

    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        // Return H12 for locales that conventionally use 12-hour time
        // (English, Korean, etc.) to avoid silent test failures.
        match locale.language() {
            "en" | "ko" => HourCycle::H12,
            _ => HourCycle::H23,
        }
    }

    fn first_day_of_week(&self, _locale: &Locale) -> Weekday {
        Weekday::Monday // ISO 8601 default
    }

    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        // Stub: only supports Gregorian identity conversion.
        // Non-Gregorian conversion panics in tests — use Icu4xProvider for production.
        panic!("StubIcuProvider does not support non-Gregorian calendar conversion; use Icu4xProvider")
    }
}
```

#### 9.5.2 Production implementation (`Icu4xProvider`)

Enabled by the `icu4x` feature flag. Uses compiled CLDR data via ICU4X's
`compiled_data` feature or `BlobDataProvider`.

```rust
// ars-i18n/src/provider/icu4x_impl.rs

#[cfg(feature = "icu4x")]
use {
    icu::{
        calendar::{
            AnyCalendar,
            Date,
            types::MonthCode,
            // NOTE: WeekInformation path at icu::calendar::week::WeekInformation
            // verified against icu_calendar 2.1.1 docs.
            week::{
                WeekInformation,
                WeekPreferences,
            },
        },
        datetime::{
            DateTimeFormatter,
            DateTimeFormatterPreferences,
            NoCalendarFormatter,
            fieldsets::{YMD, T},
        },
        decimal::{
            DecimalFormatter,
            DecimalFormatterPreferences,
        },
        time::Time,
    },
    fixed_decimal::Decimal,
    tinystr::TinyAsciiStr,
};

/// Production ICU4X-backed provider with full CLDR data.
///
/// Holds a reference to the ICU4X data provider (typically compiled data via
/// `compiled_data` feature, or `BlobDataProvider` for dynamically loaded data).
/// Formatters are created lazily and cached internally.
#[cfg(feature = "icu4x")]
pub struct Icu4xProvider {
    // The data provider is generic in ICU4X; for compiled data it is a
    // zero-size type. For blob data, it holds the deserialized postcard blob.
    //
    // In practice this will be:
    //   compiled_data feature — zero-cost, no field needed
    //   BlobDataProvider  (dynamic data)          — holds Arc<[u8]>
}

#[cfg(feature = "icu4x")]
impl Icu4xProvider {
    pub fn new() -> Self { Self {} }

    fn english_month_name_fallback(month: u8) -> &'static str {
        match month {
            1 => "January", 2 => "February", 3 => "March",
            4 => "April", 5 => "May", 6 => "June",
            7 => "July", 8 => "August", 9 => "September",
            10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown",
        }
    }

    /// Map Weekday to January 2024 day-of-month for format-and-extract.
    /// January 1, 2024 = Monday, January 7, 2024 = Sunday.
    fn weekday_to_jan2024_day(weekday: Weekday) -> u8 {
        match weekday {
            Weekday::Monday    => 1,
            Weekday::Tuesday   => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday  => 4,
            Weekday::Friday    => 5,
            Weekday::Saturday  => 6,
            Weekday::Sunday    => 7,
        }
    }
}

#[cfg(feature = "icu4x")]
impl IcuProvider for Icu4xProvider {
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String {
        // Format-and-extract: use DateTimeFormatter with E (weekday-only) field set,
        // format a date that falls on the target weekday, return the formatted string.
        use icu::datetime::{DateTimeFormatter, fieldsets::E};
        use icu::calendar::{Date, Iso};
        let fmt = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(&locale.0),
            E::short(),
        ).expect("weekday formatter should be constructible for any supported locale");
        // January 1, 2024 = Monday; January 7, 2024 = Sunday.
        let iso_day = Self::weekday_to_jan2024_day(weekday);
        let date = Date::try_new_iso(2024, 1, iso_day)
            .expect("2024-01-01..07 are valid ISO dates");
        fmt.format(&date).to_string()
    }

    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String {
        use icu::datetime::{DateTimeFormatter, fieldsets::E};
        use icu::calendar::{Date, Iso};
        let fmt = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(&locale.0),
            E::long(),
        ).expect("weekday formatter should be constructible for any supported locale");
        let iso_day = Self::weekday_to_jan2024_day(weekday);
        let date = Date::try_new_iso(2024, 1, iso_day)
            .expect("2024-01-01..07 are valid ISO dates");
        fmt.format(&date).to_string()
    }

    fn month_long_name(&self, month: u8, locale: &Locale) -> String {
        use icu::datetime::{DateTimeFormatter, fieldsets::M};
        use icu::calendar::{Date, Iso};
        let fmt = DateTimeFormatter::try_new(
            DateTimeFormatterPreferences::from(&locale.0),
            M::long(),
        ).expect("month formatter should be constructible for any supported locale");
        // Day 1 of the target month in 2024 (non-leap year is fine; month name doesn't depend on day).
        let date = Date::try_new_iso(2024, month, 1)
            .expect("month 1-12, day 1 is always valid");
        fmt.format(&date).to_string()
    }

    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
        // Returns locale-appropriate AM/PM strings:
        //   en: "AM"/"PM", ja: "午前"/"午後", ar: "ص"/"م", ko: "오전"/"오후"
        // ICU4X 2.x does not expose `DayPeriodNames` as a standalone API.
        // Extract day period labels by formatting known times and parsing the output.
        let formatter = NoCalendarFormatter::try_new(
            DateTimeFormatterPreferences::from(&locale.0),
            T::hm(),
        ).expect("locale data available");
        let test_time = if is_pm {
            Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time")
        } else {
            Time::try_new(1, 0, 0, 0).expect("01:00 is a valid time")
        };
        // Extract the day period string from the formatted output.
        // This is a heuristic — production code should cache these results.
        // NoCalendarFormatter formats Time values directly — no Date needed.
        let formatted = formatter.format(&test_time).to_string();
        // Strip numeric/separator characters to isolate the day period text.
        // Limitation: This heuristic may fail for locales where AM/PM labels contain
        // digits. Consider using ICU4X DateSymbols API for direct day period
        // name access when available.
        // Note: ICU4X 2.x does not expose a direct day-period names API.
        // This heuristic is the specified approach and is functionally correct.
        formatted.chars()
            .filter(|c| !c.is_ascii_digit() && *c != ':')
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
        // Reverse-lookup against the first character of the locale's AM/PM strings.
        let am_label = self.day_period_label(false, locale);
        let pm_label = self.day_period_label(true, locale);
        let am_char = am_label.chars().next()?;
        let pm_char = pm_label.chars().next()?;
        let ch_lower = ch.to_lowercase().next().expect("to_lowercase always yields at least one char");
        if ch_lower == am_char.to_lowercase().next().expect("to_lowercase always yields at least one char") {
            Some(false) // AM
        } else if ch_lower == pm_char.to_lowercase().next().expect("to_lowercase always yields at least one char") {
            Some(true) // PM
        } else {
            None
        }
    }

    fn format_segment_digits(&self, value: u32, min_digits: NonZero<u8>, locale: &Locale) -> String {
        // Automatically handles native digit substitution via locale numbering system:
        //   ar → ٠١٢٣٤٥٦٧٨٩, fa → ۰۱۲۳۴۵۶۷۸۹, bn → ০১২৩৪৫৬৭৮৯, my → ၀၁၂၃၄၅၆၇၈၉
        let fmt = DecimalFormatter::try_new(
            DecimalFormatterPreferences::from(&locale.0),
            Default::default(),
        ).expect("locale data available");
        let mut fd = Decimal::from(value as i64);
        // Decimal is Signed<UnsignedDecimal>; .absolute access verified for fixed_decimal 0.7.x
        fd.absolute.pad_start((min_digits.get() - 1) as i16);
        fmt.format(&fd).to_string()
    }

    // Note: Verify exact ICU4X API — some versions provide months_in_year() on the calendar object rather than on a date.
    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8 {
        // Hebrew calendar has 13 months in leap years; Chinese/Dangi have
        // intercalary months. ICU4X handles all of these correctly.
        // Note: `year` is in the target calendar's numbering. We construct
        // a date directly in that calendar to avoid ISO year mismatch.
        let cal = AnyCalendar::new(calendar.to_icu_kind());
        let month_code = MonthCode::new_normal(1).expect("valid month number");
        match Date::try_new_from_codes(era, year, month_code, 1, cal) {
            Ok(cal_date) => cal_date.months_in_year(),
            Err(_) => 12, // Fallback for unresolvable year/era
        }
    }

    fn days_in_month(&self, calendar: &CalendarSystem, year: i32, month: u8, era: Option<&str>) -> u8 {
        let cal = AnyCalendar::new(calendar.to_icu_kind());
        // Note: MonthCode::new_normal(month) works for standard months (M01-M12)
        // but will produce wrong codes for leap months (e.g., Hebrew Adar II = M06L,
        // Chinese intercalary months). Production code needs a helper like:
        //   fn month_ordinal_to_code(cal: &CalendarSystem, year: i32, ordinal: u8) -> MonthCode
        // that handles: Hebrew (month 6 in leap year → M06L), Chinese/Dangi
        // (intercalary month N → M0{N}L), and standard calendars (M01-M12).
        let month_code = MonthCode::new_normal(month).expect("valid month number");
        let date = Date::try_new_from_codes(era, year, month_code, 1, cal);
        match date {
            Ok(d) => d.days_in_month(),
            Err(_) => 30, // Fallback for invalid year/month/era
        }
    }

    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        // Query CLDR timeData for the locale's preferred hour cycle.
        // ICU4X exposes hour cycle preferences through the locale's
        // "hc" Unicode extension keyword or CLDR supplemental data.
        let formatter = NoCalendarFormatter::try_new(
            DateTimeFormatterPreferences::from(&locale.0),
            T::hm(),
        ).expect("locale data available");
        // Determine hour cycle from the locale's CLDR preferences.
        // Format a known PM time (13:00) and inspect the output.
        // NoCalendarFormatter formats Time values directly — no Date needed.
        let test_time = Time::try_new(13, 0, 0, 0).expect("13:00 is a valid time");
        let formatted = formatter.format(&test_time).to_string();
        // If formatted output contains non-digit, non-separator characters,
        // it has a day period marker (AM/PM equivalent), indicating 12-hour format.
        // NOTE: This heuristic may produce false positives for CJK locales where
        // time separators or era markers contain non-ASCII characters. A more robust
        // approach would compare the numeric hour portion (e.g., "1" vs "13") to
        // distinguish 12h from 24h format. Acceptable for current use cases.
        let has_day_period = formatted.chars().any(|c| !c.is_ascii_digit() && c != ':' && !c.is_ascii_whitespace());
        if has_day_period { HourCycle::H12 } else { HourCycle::H23 }
    }

    fn first_day_of_week(&self, locale: &Locale) -> Weekday {
        // Check explicit -u-fw- extension in locale tag first.
        if let Some(weekday) = locale.first_day_of_week_extension() {
            return weekday;
        }
        // Fall back to CLDR weekData for the locale's region.
        let week_info = WeekInformation::try_new(WeekPreferences::from(&locale.0))
            .expect("locale data available");
        Weekday::from_icu_weekday(week_info.first_weekday)
    }

    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        // Returns shared::CalendarDate (public fields), not i18n::CalendarDate (AnyCalendarDate wrapper)
        // Use ICU4X AnyCalendar to convert between arbitrary calendar systems.
        let source_cal = AnyCalendar::new(date.calendar.to_icu_kind());
        let target_cal = AnyCalendar::new(target.to_icu_kind());
        // See days_in_month() note: MonthCode::new_normal() is wrong for leap months.
        // Production code needs calendar-specific month-to-MonthCode mapping.
        let month_code = MonthCode::new_normal(date.month.get()).expect("valid month number");
        let era_str = date.era.as_ref().map(|e| e.code.as_str());
        let icu_date = Date::try_new_from_codes(
            era_str, date.year, month_code, date.day.get(), source_cal
        ).expect("valid date codes");
        let converted = icu_date.to_calendar(&target_cal);
        let converted_era = converted.year().era();
        CalendarDate {
            calendar: target,
            // display_name is set to the era code as a fallback;
            // callers should resolve a localized display name via the IcuProvider.
            // `YearInfo::era()` returns `Option<EraYear>` — None for calendars
            // without eras (e.g., ISO/Gregorian) or cyclic calendars (Chinese).
            era: converted_era.map(|e| Era {
                code: e.era.as_str().to_string(),
                display_name: e.era.as_str().to_string(),
            }).filter(|_| target.has_custom_eras()),
            year: converted.year().era_year_or_related_iso(),
            month: NonZero::new(converted.month().month_number())
                .expect("ICU4X month_number() is 1-based"),
            day: NonZero::new(converted.day_of_month().0)
                .expect("ICU4X day_of_month() is 1-based"),
        }
    }
}
```

#### 9.5.3 Default provider resolution

```rust
// ars-i18n/src/provider.rs

/// Returns the default IcuProvider for the current feature-flag configuration.
///
/// - With `icu4x` feature: returns `Icu4xProvider` (full CLDR data).
/// - Without `icu4x`: returns `StubIcuProvider` (English-only).
pub fn default_provider() -> Box<dyn IcuProvider> {
    #[cfg(feature = "icu4x")]
    { Box::new(Icu4xProvider::new()) }
    #[cfg(not(feature = "icu4x"))]
    { Box::new(StubIcuProvider) }
}
```

---

## 10. Framework Integration: Locale via ArsProvider

Locale is provided by `ArsProvider` — the single root provider. The formerly
separate `LocaleProvider` is subsumed. See `01-architecture.md` §6.4 for the full
`ArsContext` shape and `08-adapter-leptos.md` §13 / `09-adapter-dioxus.md` §16
for the reactive adapter context types.

### 10.1 Leptos

```rust
// ars-leptos/src/i18n.rs

use leptos::prelude::*;
use ars_i18n::{Locale, Direction};

// ArsContext is defined in 08-adapter-leptos.md §13.
// It carries Signal<Locale>, Memo<Direction>, and other environment values.

/// Access the current locale in a Leptos component.
/// Falls back to `en-US` if no `ArsProvider` is present.
///
/// **Important:** Do not call inside reactive closures or effects.
pub fn use_locale() -> Signal<Locale> {
    use_context::<ArsContext>()
        .map(|ctx| ctx.locale)
        .unwrap_or_else(|| {
            warn_missing_provider("use_locale");
            Signal::stored(Locale::parse("en-US").expect("en-US is always a valid BCP 47 locale"))
        })
}

/// Access the current direction.
pub fn use_direction() -> Memo<Direction> {
    use_context::<ArsContext>()
        .map(|ctx| ctx.direction)
        .unwrap_or_else(|| {
            warn_missing_provider("use_direction");
            Memo::new(|_| Direction::Ltr)
        })
}
```

### 10.2 Dioxus

```rust
// ars-dioxus/src/i18n.rs

use dioxus::prelude::*;
use ars_i18n::{Locale, Direction};

// ArsContext is defined in 09-adapter-dioxus.md §16.
// It carries Signal<Locale>, Memo<Direction>, and other environment values.

/// Access the current locale in a Dioxus component.
/// Falls back to `en-US` if no `ArsProvider` is present.
pub fn use_locale() -> Signal<Locale> {
    let fallback = use_signal(|| Locale::parse("en-US").expect("en-US is always a valid BCP 47 locale"));
    try_use_context::<ArsContext>()
        .map(|ctx| ctx.locale)
        .unwrap_or_else(|| {
            warn_missing_provider("use_locale");
            fallback
        })
}
```

### 10.3 Server-Side Locale Detection

```rust
#[cfg(feature = "std")]
/// Detect locale from HTTP Accept-Language header.
///
/// Returns the best matching locale from the accept header.
pub fn locale_from_accept_language(accept_language: &str, supported: &[Locale]) -> Locale {
    // Parse "en-US,en;q=0.9,de;q=0.8" format
    let mut preferences: Vec<(String, f32)> = accept_language
        .split(',')
        .filter_map(|part| {
            let mut iter = part.trim().splitn(2, ";q=");
            let tag = iter.next()?.trim().to_string();
            let quality: f32 = iter.next()
                .and_then(|q| q.parse().ok())
                .unwrap_or(1.0);
            Some((tag, quality))
        })
        .collect();

    // Sort by quality descending
    // Design decision: partial_cmp returns None for NaN quality values.
    // Treating NaN as equal preserves original order (stable sort) — correct for
    // malformed q= values. Using .expect() would panic on malformed input.
    preferences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Find first supported locale
    for (tag, _) in &preferences {
        if let Ok(locale) = Locale::parse(tag) {
            if supported.contains(&locale) {
                return locale;
            }
            // Try language-only match
            if let Ok(lang_locale) = Locale::parse(locale.language()) {
                if let Some(matched) = supported.iter().find(|s| s.language() == lang_locale.language()) {
                    return matched.clone();
                }
            }
        }
    }

    // Fall back to first supported locale
    supported.first().cloned().unwrap_or_else(|| Locale::parse("en-US").expect("en-US is valid locale"))
}
```

---

## 11. Component-Specific i18n Behavior

### 11.1 NumberInput

- Decimal separator: `.` (en-US) vs `,` (de-DE) — use `parse_locale_number`
- Grouping separator: `,` (en-US) vs `.` (de-DE) vs ` ` (fr-FR)
- Increment/decrement labels use locale-formatted values in `aria-valuetext`
- RTL: increment/decrement button visual order flips

### 11.2 Slider

- `aria-valuetext` uses `NumberFormatter` with user-provided format options
- Range slider: both values formatted with range separator (`–`)
- RTL: value increases from right to left; arrow key semantics flip

### 11.3 Calendar / DatePicker

- Month/weekday names from `DateFormatter` (locale language)
- First day of week from `WeekInfo::for_locale()`
- Calendar system from locale extension or explicit prop
- Japanese calendar: show era name + year-in-era (e.g., "令和6年")
- Hebrew calendar: different month count in leap years

### 11.4 Select / Combobox

- Type-ahead search: use `StringCollator` for locale-aware matching
- Option sorting (if enabled): use `StringCollator.sort_by_key`
- Selected count label: use `SelectMessages.selected_count` with plural rules

### 11.5 Pagination

- Page numbers: use locale's number system (Arabic-Indic, Devanagari, etc.)
- RTL: visual order of page numbers may reverse

### 11.6 Table

- Column sort: `StringCollator` for string columns
- Date/number cells: formatted with locale formatters
- RTL: column order and sort indicator direction reverse

#### 11.6.1 Locale-Aware Sorting Requirements

Table column sorting MUST always use `StringCollator` (from `ars-i18n`) with locale-aware rules. Raw
`String::cmp()` or byte-level comparison is NEVER acceptable for user-visible sorting.

**String columns**: Use `StringCollator` with the current locale. The `collation_strength`
prop allows consumers to control comparison sensitivity. Uses `CollationStrength` from §8
(`Primary`, `Secondary`, `Tertiary`, `Quaternary`) — defined in §8 above; `06-collections.md` re-exports this type.

**Date columns**: Sort by the underlying calendar value (epoch timestamp or
`CalendarDate` comparison), NOT by the formatted display string. Formatted strings
vary by locale and calendar system, making string comparison unreliable.

**Number columns**: Sort by the numeric value, not the formatted string. Locale
formatting adds grouping separators and decimal variants that break lexicographic sort.

**Mixed-type columns**: When a column contains heterogeneous data, define a sort key
function that extracts a comparable value, then use `StringCollator::sort_by_key`.

### 11.7 TagsInput

- Delimiter detection: some locales use different separator conventions
- Tag display order: RTL users expect right-to-left tag addition

---

## 12. Error Types

```rust
#[derive(Debug)]
pub struct LocaleParseError(pub icu::locale::ParseError);

#[derive(Debug)]
pub enum DateError {
    InvalidDate,
    OutOfRange,
    CalendarError(String),
}

/// Error returned by `CalendarDate` arithmetic and query methods
/// (e.g., `add_days`, `days_until`, `today`, `is_before`) when the
/// operation is not yet implemented for the underlying calendar system.
#[derive(Debug)]
pub enum CalendarError {
    /// An ICU4X calendar arithmetic error.
    Arithmetic(String),
}

impl core::fmt::Display for CalendarError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Arithmetic(msg) => write!(f, "calendar arithmetic error: {msg}"),
        }
    }
}

/// Error returned by `CalendarDate::from_calendar()` when conversion to a
/// specific calendar system fails.
#[derive(Debug)]
pub enum CalendarConversionError {
    /// The date components are invalid for the target calendar.
    InvalidDate,
    /// ICU4X conversion error.
    Icu(String),
}

impl core::fmt::Display for CalendarConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDate => write!(f, "invalid date for target calendar"),
            Self::Icu(msg) => write!(f, "ICU4X calendar conversion error: {msg}"),
        }
    }
}
```

---

## 13. Locale Resolution Algorithm

Locale resolution follows this priority order:

1. Component-level `locale` prop (highest priority).
2. Nearest ancestor `ArsProvider` context (via `use_locale()`).
3. HTML `lang` attribute on document root (`document.documentElement.lang`).
4. Navigator language (`navigator.language`).
5. Fallback: `'en-US'`.

Resolution is cached per component lifecycle and invalidated when `ArsProvider` context changes.

## 14. Number Formatting and Non-Uniform Digit Grouping

Number formatting delegates to ICU4X `NumberFormatter` which handles non-uniform grouping (e.g., Indian: 12,34,567) automatically based on locale. Parsing must also accept locale-specific grouping separators. Components must not hardcode grouping assumptions (e.g., every 3 digits).

## 15. Text Expansion Margins for Localized Components

Adapter styling guidance must account for text expansion ratios across locales (English→German: ~+35%, English→Chinese: ~-30%, English→Arabic: ~+25%). Fixed-width containers (buttons, badges, tabs) should use `min-width` rather than fixed width. Adapters should test layouts with pseudo-localization (e.g., doubling all strings) during development.

## 16. Currency Formatting for Mixed LTR/RTL Content

Currency formatting in mixed-direction content must wrap formatted values in Unicode BiDi isolates (U+2066 LRI ... U+2069 PDI) or HTML `<bdi>` elements. `CurrencyFormatter` output includes directional isolates by default. Components displaying monetary values in RTL contexts must not rely on CSS `direction` alone — explicit BiDi isolation prevents digit reordering.
