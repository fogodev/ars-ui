use alloc::{format, rc::Rc, string::String};
use core::{fmt, num::NonZeroU8, str::FromStr};

use fixed_decimal::{Decimal, SignDisplay as FixedSignDisplay};
use icu::decimal::DecimalFormatter;
pub use icu_experimental::measure::measureunit::MeasureUnit;
#[cfg(feature = "std")]
use {alloc::collections::BTreeMap, std::cell::RefCell};
#[cfg(feature = "icu4x")]
use {
    icu::decimal::{
        DecimalFormatterPreferences,
        options::{DecimalFormatterOptions, GroupingStrategy},
    },
    icu_experimental::{
        dimension::{
            currency::{
                CurrencyCode as IcuCurrencyCode,
                formatter::{
                    CurrencyFormatter as IcuCurrencyFormatter, CurrencyFormatterPreferences,
                },
                options::CurrencyFormatterOptions,
            },
            percent::{
                formatter::{PercentFormatter, PercentFormatterPreferences},
                options::PercentFormatterOptions,
            },
            units::{
                formatter::{UnitsFormatter, UnitsFormatterPreferences},
                options::{UnitsFormatterOptions, Width},
            },
        },
        measure::parser::ids::CLDR_IDS_TRIE,
    },
    tinystr::TinyAsciiStr,
};

use crate::Locale;

/// Options controlling locale-aware number formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberFormatOptions {
    /// The high-level number style to format.
    pub style: NumberStyle,
    /// The display width to use when formatting [`NumberStyle::Unit`] values.
    pub unit_display: UnitDisplay,
    /// The minimum number of digits to display before the decimal separator.
    pub min_integer_digits: NonZeroU8,
    /// The minimum number of digits to display after the decimal separator.
    pub min_fraction_digits: u8,
    /// The maximum number of digits to display after the decimal separator.
    pub max_fraction_digits: u8,
    /// Whether locale-appropriate grouping separators should be emitted.
    pub use_grouping: bool,
    /// How positive, negative, and zero values should display a sign.
    pub sign_display: SignDisplay,
    /// The rounding rule to apply before the number is formatted.
    pub rounding_mode: RoundingMode,
}

/// The overall presentation style for formatted numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberStyle {
    /// A plain decimal number.
    Decimal,
    /// A percentage value.
    Percent,
    /// A monetary value in the given ISO 4217 currency.
    Currency(CurrencyCode),
    /// A measurement value in the given CLDR unit.
    Unit(MeasureUnit),
}

/// The width used when formatting measurement units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnitDisplay {
    /// Locale-appropriate long unit names.
    Long,
    /// Locale-appropriate short unit names.
    #[default]
    Short,
    /// Locale-appropriate narrow unit names.
    Narrow,
}

/// Sign display policy for formatted numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignDisplay {
    /// Show a sign only for negative values.
    Auto,
    /// Always show a sign for non-negative and negative values.
    Always,
    /// Never show a sign.
    Never,
    /// Show a sign for non-zero values only.
    ExceptZero,
    /// Show only the negative sign.
    Negative,
}

/// Rounding mode used before formatting a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RoundingMode {
    /// Round to nearest, with ties resolved to the nearest even digit.
    #[default]
    HalfEven,
    /// Round to nearest, with ties resolved away from zero.
    HalfUp,
    /// Round to nearest, with ties resolved toward zero.
    HalfDown,
    /// Round toward positive infinity.
    Ceiling,
    /// Round toward negative infinity.
    Floor,
    /// Round toward zero.
    Truncate,
}

/// An ISO 4217 currency code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CurrencyCode(
    /// Uppercase ASCII bytes encoding the currency identifier.
    pub [u8; 3],
);

impl CurrencyCode {
    /// United States Dollar.
    pub const USD: Self = Self(*b"USD");
    /// Euro.
    pub const EUR: Self = Self(*b"EUR");
    /// British Pound Sterling.
    pub const GBP: Self = Self(*b"GBP");
    /// Japanese Yen.
    pub const JPY: Self = Self(*b"JPY");
    /// Chinese Yuan Renminbi.
    pub const CNY: Self = Self(*b"CNY");

    /// Parse an ISO 4217 currency code from uppercase ASCII text.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "API matches the specification"
    )]
    pub fn from_str(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return None;
        }

        Some(Self([bytes[0], bytes[1], bytes[2]]))
    }

    /// Return this currency code as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("CurrencyCode contains valid ASCII")
    }

    #[cfg(feature = "icu4x")]
    fn as_icu(self) -> IcuCurrencyCode {
        let code = TinyAsciiStr::<3>::try_from_utf8(self.as_str().as_bytes())
            .expect("CurrencyCode contains a valid TinyAsciiStr");
        IcuCurrencyCode(code)
    }
}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            style: NumberStyle::Decimal,
            unit_display: UnitDisplay::Short,
            min_integer_digits: NonZeroU8::new(1).expect("hardcoded nonzero"),
            min_fraction_digits: 0,
            max_fraction_digits: 3,
            use_grouping: true,
            sign_display: SignDisplay::Auto,
            rounding_mode: RoundingMode::HalfEven,
        }
    }
}

/// A locale-aware formatter for decimals, percents, currencies, and units.
#[derive(Clone)]
pub struct NumberFormatter {
    locale: Locale,
    options: NumberFormatOptions,
    decimal_separator: char,
    grouping_separator: Option<char>,
    backend: FormatterBackend,
}

#[derive(Clone)]
enum FormatterBackend {
    Decimal(Rc<DecimalFormatter>),
    #[cfg(feature = "icu4x")]
    Percent(Rc<PercentFormatter<DecimalFormatter>>),
    #[cfg(not(feature = "icu4x"))]
    Percent,
    #[cfg(feature = "icu4x")]
    Currency(Rc<IcuCurrencyFormatter>),
    #[cfg(not(feature = "icu4x"))]
    Currency,
    #[cfg(feature = "icu4x")]
    Unit(Rc<UnitsFormatter>),
    #[cfg(not(feature = "icu4x"))]
    Unit,
}

impl fmt::Debug for NumberFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NumberFormatter")
            .field("locale", &self.locale)
            .field("options", &self.options)
            .field("decimal_separator", &self.decimal_separator)
            .field("grouping_separator", &self.grouping_separator)
            .finish()
    }
}

#[cfg(feature = "std")]
thread_local! {
    static NUMBER_FORMATTER_CACHE: RefCell<BTreeMap<String, NumberFormatter>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Return a cached formatter for the given locale and options.
///
/// On `std` builds ars-i18n keeps a process-local cache keyed by locale and
/// format options so repeated formatter construction can be avoided.
#[cfg(feature = "std")]
#[must_use]
pub fn get_number_formatter(locale: &Locale, options: &NumberFormatOptions) -> NumberFormatter {
    let key = format!("{:?}-{:?}", locale.to_bcp47(), options);
    NUMBER_FORMATTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(existing) = cache.get(&key) {
            return existing.clone();
        }

        let formatter = NumberFormatter::new(locale, options.clone());
        cache.insert(key, formatter.clone());
        formatter
    })
}

impl NumberFormatter {
    /// Create a new locale-aware formatter for the given locale and options.
    #[must_use]
    pub fn new(locale: &Locale, options: NumberFormatOptions) -> Self {
        let options = normalize_style_defaults(options);
        let (decimal_separator, grouping_separator) = decimal_and_group_separators(locale);
        let backend = FormatterBackend::for_locale_and_style(locale, &options);

        Self {
            locale: locale.clone(),
            options,
            decimal_separator,
            grouping_separator: Some(grouping_separator),
            backend,
        }
    }

    /// Format a numeric value according to this formatter's locale and style.
    #[must_use]
    pub fn format(&self, value: f64) -> String {
        let decimal = self.prepare_decimal(value);

        match &self.backend {
            FormatterBackend::Decimal(formatter) => formatter.format(&decimal).to_string(),
            #[cfg(feature = "icu4x")]
            FormatterBackend::Percent(formatter) => formatter.format(&decimal).to_string(),
            #[cfg(not(feature = "icu4x"))]
            FormatterBackend::Percent => fallback_format(&decimal, self),
            #[cfg(feature = "icu4x")]
            FormatterBackend::Currency(formatter) => {
                #[cfg(not(feature = "std"))]
                use alloc::string::ToString as _;

                let NumberStyle::Currency(code) = self.options.style else {
                    unreachable!("currency backend must match currency style");
                };
                formatter
                    .format_fixed_decimal(&decimal, &code.as_icu())
                    .to_string()
            }
            #[cfg(not(feature = "icu4x"))]
            FormatterBackend::Currency => fallback_format(&decimal, self),
            #[cfg(feature = "icu4x")]
            FormatterBackend::Unit(formatter) => {
                formatter.format_fixed_decimal(&decimal).to_string()
            }
            #[cfg(not(feature = "icu4x"))]
            FormatterBackend::Unit => fallback_format(&decimal, self),
        }
    }

    /// Parse a locale-formatted number back into a numeric value.
    pub fn parse(&self, input: &str) -> Option<f64> {
        let parsed = parse_locale_number(input, &self.locale)?;
        if matches!(self.options.style, NumberStyle::Percent) {
            Some(parsed / 100.0)
        } else {
            Some(parsed)
        }
    }

    /// Return this formatter's decimal separator.
    #[must_use]
    pub const fn decimal_separator(&self) -> char {
        self.decimal_separator
    }

    /// Return this formatter's grouping separator, when one is known.
    #[must_use]
    pub const fn grouping_separator(&self) -> Option<char> {
        self.grouping_separator
    }

    /// Format a monetary amount using the given ISO 4217 currency code.
    #[must_use]
    pub fn format_currency(&self, amount: f64, currency_code: &str) -> String {
        let code = CurrencyCode::from_str(currency_code).expect("invalid ISO 4217 currency code");
        let precision = iso4217_minor_units(code);

        let mut options = self.options.clone();
        options.style = NumberStyle::Currency(code);
        options.min_fraction_digits = precision;
        options.max_fraction_digits = precision;

        Self::new(&self.locale, options).format(amount)
    }

    /// Format a fractional value as a percentage.
    #[must_use]
    pub fn format_percent(&self, value: f64, max_fraction_digits: Option<u8>) -> String {
        let mut options = self.options.clone();
        options.style = NumberStyle::Percent;
        options.min_fraction_digits = 0;
        options.max_fraction_digits = max_fraction_digits.unwrap_or(0);

        Self::new(&self.locale, options).format(value)
    }

    /// Format a start/end numeric range using a locale-sensitive separator.
    #[must_use]
    pub fn format_range(&self, start: f64, end: f64, locale: &Locale) -> String {
        let separator = match locale.language() {
            "fr" => " – ",
            "ja" | "zh" | "ko" => "〜",
            _ => "–",
        };

        format!("{}{}{}", self.format(start), separator, self.format(end))
    }

    fn prepare_decimal(&self, value: f64) -> Decimal {
        let scaled = if matches!(self.options.style, NumberStyle::Percent) {
            value * 100.0
        } else {
            value
        };

        if !scaled.is_finite() {
            return Decimal::default();
        }

        let rounded = round_value(
            scaled,
            self.options.max_fraction_digits,
            self.options.rounding_mode,
        );
        let precision = usize::from(self.options.max_fraction_digits);
        let decimal_literal = if precision == 0 {
            format!("{rounded:.0}")
        } else {
            format!("{rounded:.precision$}")
        };

        let mut decimal = Decimal::from_str(&decimal_literal).unwrap_or_default();
        decimal.absolute.trim_end();
        decimal
            .absolute
            .pad_end(-i16::from(self.options.min_fraction_digits));
        decimal
            .absolute
            .pad_start(i16::from(self.options.min_integer_digits.get()));
        decimal.apply_sign_display(self.options.sign_display.into_fixed_decimal());
        decimal
    }
}

impl FormatterBackend {
    fn for_locale_and_style(locale: &Locale, options: &NumberFormatOptions) -> Self {
        match &options.style {
            NumberStyle::Decimal => {
                Self::Decimal(Rc::new(build_decimal_formatter(locale, options)))
            }
            #[cfg(feature = "icu4x")]
            NumberStyle::Percent => {
                let formatter = PercentFormatter::try_new(
                    PercentFormatterPreferences::from(locale.as_icu()),
                    PercentFormatterOptions::default(),
                )
                .expect("compiled_data guarantees percent formatter availability");
                Self::Percent(Rc::new(formatter))
            }
            #[cfg(not(feature = "icu4x"))]
            NumberStyle::Percent => Self::Percent,
            #[cfg(feature = "icu4x")]
            NumberStyle::Currency(_) => {
                let formatter = IcuCurrencyFormatter::try_new(
                    CurrencyFormatterPreferences::from(locale.as_icu()),
                    CurrencyFormatterOptions::default(),
                )
                .expect("compiled_data guarantees currency formatter availability");
                Self::Currency(Rc::new(formatter))
            }
            #[cfg(not(feature = "icu4x"))]
            NumberStyle::Currency(_) => Self::Currency,
            #[cfg(feature = "icu4x")]
            NumberStyle::Unit(unit) => {
                let unit_id = resolve_measure_unit_id(unit)
                    .expect("unit formatter requires a resolvable CLDR unit id");
                let mut formatter_options = UnitsFormatterOptions::default();
                formatter_options.width = options.unit_display.into_icu_width();
                let formatter = UnitsFormatter::try_new(
                    UnitsFormatterPreferences::from(locale.as_icu()),
                    &unit_id,
                    formatter_options,
                )
                .expect("compiled_data guarantees unit formatter availability");
                Self::Unit(Rc::new(formatter))
            }
            #[cfg(not(feature = "icu4x"))]
            NumberStyle::Unit(_) => Self::Unit,
        }
    }
}

/// Normalize non-Latin decimal digits into ASCII digits.
#[must_use]
pub fn normalize_digits(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                c
            } else if ('\u{0660}'..='\u{0669}').contains(&c) {
                char::from(b'0' + (c as u32 - 0x0660) as u8)
            } else if ('\u{06F0}'..='\u{06F9}').contains(&c) {
                char::from(b'0' + (c as u32 - 0x06F0) as u8)
            } else if ('\u{0966}'..='\u{096F}').contains(&c) {
                char::from(b'0' + (c as u32 - 0x0966) as u8)
            } else if ('\u{09E6}'..='\u{09EF}').contains(&c) {
                char::from(b'0' + (c as u32 - 0x09E6) as u8)
            } else {
                c
            }
        })
        .collect()
}

/// Parse a locale-aware numeric string.
pub fn parse_locale_number(input: &str, locale: &Locale) -> Option<f64> {
    let transliterated = normalize_digits(input);
    let (decimal_sep, group_sep) = decimal_and_group_separators(locale);
    let filtered = filter_numeric_characters(&transliterated, decimal_sep, group_sep);
    let chosen_decimal = choose_decimal_separator(&filtered, decimal_sep, group_sep);
    let normalized = normalize_numeric_syntax(&filtered, chosen_decimal);
    normalized.parse::<f64>().ok()
}

/// Determine the decimal and grouping separators for a locale.
#[must_use]
pub fn decimal_and_group_separators(locale: &Locale) -> (char, char) {
    #[cfg(feature = "icu4x")]
    {
        let formatter = DecimalFormatter::try_new(
            DecimalFormatterPreferences::from(locale.as_icu()),
            DecimalFormatterOptions::default(),
        )
        .expect("compiled_data guarantees decimal formatter availability");
        let formatted = formatter
            .format(&Decimal::from_str("12345.6").expect("static decimal string must parse"))
            .to_string();

        parse_separators(&formatted)
    }

    #[cfg(not(feature = "icu4x"))]
    {
        fallback_separators(locale)
    }
}

fn filter_numeric_characters(input: &str, decimal_sep: char, group_sep: char) -> String {
    input
        .chars()
        .filter(|c| {
            c.is_ascii_digit()
                || matches!(*c, '+' | '-' | '.' | ',')
                || matches!(*c, '\u{066B}' | '\u{066C}' | '\u{00A0}' | '\u{202F}')
                || c.is_ascii_whitespace()
                || *c == decimal_sep
                || *c == group_sep
        })
        .collect()
}

fn normalize_numeric_syntax(input: &str, decimal_separator: Option<char>) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut seen_sign = false;

    for c in input.chars() {
        if c.is_ascii_digit() {
            normalized.push(c);
        } else if matches!(c, '+' | '-') && !seen_sign && normalized.is_empty() {
            normalized.push(c);
            seen_sign = true;
        } else if Some(c) == decimal_separator {
            normalized.push('.');
        }
    }

    normalized
}

fn choose_decimal_separator(input: &str, locale_decimal: char, locale_group: char) -> Option<char> {
    let locale_decimal_occurs = input.contains(locale_decimal);
    if locale_decimal_occurs {
        return Some(locale_decimal);
    }

    if input.contains('\u{066B}') {
        return Some('\u{066B}');
    }

    let last = input
        .char_indices()
        .rev()
        .find(|(_, c)| matches!(*c, '.' | ',' | '\u{066B}'));
    let (idx, separator) = last?;

    let digits_after = input[idx + separator.len_utf8()..]
        .chars()
        .filter(char::is_ascii_digit)
        .count();
    let occurrences = input.chars().filter(|c| *c == separator).count();
    let other_separator_present = input
        .chars()
        .any(|c| matches!(c, '.' | ',' | '\u{066B}') && c != separator);

    if separator == '\u{066C}' {
        return None;
    }

    if separator == locale_group && digits_after == 3 && !other_separator_present {
        return None;
    }

    if occurrences > 1 && digits_after == 3 && !other_separator_present {
        return None;
    }

    Some(separator)
}

#[cfg(feature = "icu4x")]
fn parse_separators(formatted: &str) -> (char, char) {
    let mut decimal_sep = '.';
    let mut group_sep = ',';

    if let Some((idx, ch)) = formatted
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_numeric() && !matches!(*c, '+' | '-'))
    {
        let has_digit_after = formatted[idx + ch.len_utf8()..]
            .chars()
            .any(char::is_numeric);
        if has_digit_after {
            decimal_sep = ch;
        }
    }

    let integer_part = formatted.split(decimal_sep).next().unwrap_or(formatted);
    if let Some(ch) = integer_part
        .chars()
        .find(|c| !c.is_numeric() && !matches!(*c, '+' | '-'))
    {
        group_sep = ch;
    }

    (decimal_sep, group_sep)
}

fn normalize_style_defaults(mut options: NumberFormatOptions) -> NumberFormatOptions {
    if options.min_fraction_digits == 0 && options.max_fraction_digits == 3 {
        match options.style {
            NumberStyle::Percent => {
                options.max_fraction_digits = 0;
            }
            NumberStyle::Currency(code) => {
                let precision = iso4217_minor_units(code);
                options.min_fraction_digits = precision;
                options.max_fraction_digits = precision;
            }
            NumberStyle::Decimal | NumberStyle::Unit(_) => {}
        }
    }

    options
}

fn iso4217_minor_units(code: CurrencyCode) -> u8 {
    match code.as_str() {
        "BHD" | "KWD" | "OMR" | "IQD" | "LYD" | "TND" => 3,
        "CLF" | "UYW" => 4,
        "JPY" | "KRW" | "VND" | "ISK" | "CLP" | "UGX" | "GNF" | "XOF" | "XAF" | "XPF" | "RWF"
        | "DJF" | "KMF" | "VUV" | "PYG" => 0,
        _ => 2,
    }
}

#[cfg(feature = "icu4x")]
fn resolve_measure_unit_id(unit: &MeasureUnit) -> Option<String> {
    #[cfg(not(feature = "std"))]
    use alloc::borrow::ToOwned as _;

    if let Some(id) = unit.id {
        return Some(id.to_owned());
    }

    for (candidate, _) in CLDR_IDS_TRIE.iter() {
        let Ok(parsed) = MeasureUnit::try_from_str(&candidate) else {
            continue;
        };
        if parsed == *unit {
            return Some(candidate);
        }
    }

    None
}

#[cfg(feature = "icu4x")]
fn build_decimal_formatter(locale: &Locale, options: &NumberFormatOptions) -> DecimalFormatter {
    let mut formatter_options = DecimalFormatterOptions::default();
    formatter_options.grouping_strategy = Some(if options.use_grouping {
        GroupingStrategy::Auto
    } else {
        GroupingStrategy::Never
    });

    DecimalFormatter::try_new(
        DecimalFormatterPreferences::from(locale.as_icu()),
        formatter_options,
    )
    .expect("compiled_data guarantees decimal formatter availability")
}

#[cfg(not(feature = "icu4x"))]
fn build_decimal_formatter(_locale: &Locale, _options: &NumberFormatOptions) -> DecimalFormatter {
    unreachable!("decimal formatters are only constructed when the icu4x feature is enabled")
}

fn round_value(value: f64, fraction_digits: u8, mode: RoundingMode) -> f64 {
    let factor = core_maths::CoreFloat::powi(10_f64, i32::from(fraction_digits));
    let scaled = value * factor;
    let rounded = match mode {
        RoundingMode::HalfEven => round_half_even(scaled),
        RoundingMode::HalfUp => round_half_away_from_zero(scaled),
        RoundingMode::HalfDown => round_half_toward_zero(scaled),
        RoundingMode::Ceiling => core_maths::CoreFloat::ceil(scaled),
        RoundingMode::Floor => core_maths::CoreFloat::floor(scaled),
        RoundingMode::Truncate => core_maths::CoreFloat::trunc(scaled),
    };

    rounded / factor
}

fn round_half_even(value: f64) -> f64 {
    let abs = value.abs();
    let floor = core_maths::CoreFloat::floor(abs);
    let fraction = abs - floor;
    let rounded = if fraction < 0.5 {
        floor
    } else if fraction > 0.5 {
        floor + 1.0
    } else if core_maths::CoreFloat::rem_euclid(floor, 2.0) == 0.0 {
        floor
    } else {
        floor + 1.0
    };

    rounded.copysign(value)
}

fn round_half_away_from_zero(value: f64) -> f64 {
    let abs = value.abs();
    let floor = core_maths::CoreFloat::floor(abs);
    let fraction = abs - floor;
    let rounded = if fraction >= 0.5 { floor + 1.0 } else { floor };
    rounded.copysign(value)
}

fn round_half_toward_zero(value: f64) -> f64 {
    let abs = value.abs();
    let floor = core_maths::CoreFloat::floor(abs);
    let fraction = abs - floor;
    let rounded = if fraction > 0.5 { floor + 1.0 } else { floor };
    rounded.copysign(value)
}

#[cfg(not(feature = "icu4x"))]
fn fallback_separators(locale: &Locale) -> (char, char) {
    match locale.language() {
        "de" | "pt" => (',', '.'),
        "fr" => (',', ' '),
        "ar" => ('٫', '٬'),
        _ => ('.', ','),
    }
}

#[cfg(not(feature = "icu4x"))]
fn fallback_format(decimal: &Decimal, formatter: &NumberFormatter) -> String {
    let mut output = decimal.to_string();
    if formatter.decimal_separator != '.' {
        output = output.replace('.', &String::from(formatter.decimal_separator));
    }
    output
}

impl SignDisplay {
    const fn into_fixed_decimal(self) -> FixedSignDisplay {
        match self {
            Self::Auto => FixedSignDisplay::Auto,
            Self::Always => FixedSignDisplay::Always,
            Self::Never => FixedSignDisplay::Never,
            Self::ExceptZero => FixedSignDisplay::ExceptZero,
            Self::Negative => FixedSignDisplay::Negative,
        }
    }
}

impl UnitDisplay {
    #[cfg(feature = "icu4x")]
    const fn into_icu_width(self) -> Width {
        match self {
            Self::Long => Width::Long,
            Self::Short => Width::Short,
            Self::Narrow => Width::Narrow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locales;

    #[test]
    fn currency_code_rejects_invalid_text_and_roundtrips_valid_codes() {
        assert_eq!(CurrencyCode::from_str("usd"), None);
        assert_eq!(CurrencyCode::from_str("US"), None);
        assert_eq!(CurrencyCode::from_str("US1"), None);
        assert_eq!(
            CurrencyCode::from_str("USD").map(|code| String::from(code.as_str())),
            Some(String::from("USD"))
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn number_formatter_debug_includes_locale_and_separator_fields() {
        let formatter = NumberFormatter::new(&locales::de_de(), NumberFormatOptions::default());
        let debug = format!("{formatter:?}");

        assert!(debug.contains("NumberFormatter"));
        assert!(debug.contains("de-DE"));
        assert!(debug.contains("decimal_separator"));
        assert!(debug.contains("grouping_separator"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_en_us_integers_and_decimals() {
        let formatter = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());

        assert_eq!(formatter.format(1234.0), "1,234");
        assert_eq!(formatter.format(1234.56), "1,234.56");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_grouping_and_decimal_separators_for_en_us_and_de_de() {
        let en_us = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());
        let de_de = NumberFormatter::new(&locales::de_de(), NumberFormatOptions::default());

        assert_eq!(en_us.format(1234.56), "1,234.56");
        assert_eq!(de_de.format(1234.56), "1.234,56");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn round_trips_parse_for_en_us_and_de_de() {
        let en_us = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());
        let de_de = NumberFormatter::new(&locales::de_de(), NumberFormatOptions::default());

        assert_eq!(en_us.parse("1,234.56"), Some(1234.56));
        assert_eq!(de_de.parse("1.234,56"), Some(1234.56));
        assert_eq!(de_de.parse("1.5"), Some(1.5));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn exposes_decimal_and_grouping_separator_accessors() {
        let en_us = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());
        let de_de = NumberFormatter::new(&locales::de_de(), NumberFormatOptions::default());

        assert_eq!(en_us.decimal_separator(), '.');
        assert_eq!(en_us.grouping_separator(), Some(','));
        assert_eq!(de_de.decimal_separator(), ',');
        assert_eq!(de_de.grouping_separator(), Some('.'));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_percent_from_fractional_input() {
        let options = NumberFormatOptions {
            style: NumberStyle::Percent,
            ..NumberFormatOptions::default()
        };
        let en_us = NumberFormatter::new(&locales::en_us(), options.clone());
        let de_de = NumberFormatter::new(&locales::de_de(), options);

        assert_eq!(en_us.format(0.47), "47%");
        assert_eq!(de_de.format(0.47), "47 %");
        assert_eq!(en_us.format_percent(0.475, Some(1)), "47.5%");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn parses_percent_back_to_fraction() {
        let options = NumberFormatOptions {
            style: NumberStyle::Percent,
            ..NumberFormatOptions::default()
        };
        let formatter = NumberFormatter::new(&locales::en_us(), options);

        assert_eq!(formatter.parse("47%"), Some(0.47));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_currency_with_locale_correct_symbol_placement() {
        let base = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());
        let de = NumberFormatter::new(&locales::de_de(), NumberFormatOptions::default());
        let ja = NumberFormatter::new(&locales::ja_jp(), NumberFormatOptions::default());

        assert_eq!(base.format_currency(1234.5, "USD"), "$1,234.50");
        assert_eq!(de.format_currency(1234.5, "EUR"), "1.234,50 €");
        assert_eq!(ja.format_currency(1234.5, "JPY"), "￥1,234");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_unit_values_for_multiple_widths() {
        let unit = MeasureUnit::try_from_str("kilogram").expect("kilogram is a valid CLDR unit");

        let long = NumberFormatter::new(
            &locales::en_us(),
            NumberFormatOptions {
                style: NumberStyle::Unit(unit.clone()),
                unit_display: UnitDisplay::Long,
                ..NumberFormatOptions::default()
            },
        )
        .format(5.0);
        let short = NumberFormatter::new(
            &locales::en_us(),
            NumberFormatOptions {
                style: NumberStyle::Unit(unit.clone()),
                unit_display: UnitDisplay::Short,
                ..NumberFormatOptions::default()
            },
        )
        .format(5.0);
        let narrow = NumberFormatter::new(
            &locales::en_us(),
            NumberFormatOptions {
                style: NumberStyle::Unit(unit),
                unit_display: UnitDisplay::Narrow,
                ..NumberFormatOptions::default()
            },
        )
        .format(5.0);

        assert_ne!(long, short);
        assert_ne!(short, narrow);
        assert!(long.contains("kil"));
        assert!(short.contains("kg"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn parses_numeric_core_from_currency_or_unit_affixed_strings() {
        let currency = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());
        let unit = MeasureUnit::try_from_str("celsius").expect("celsius is a valid CLDR unit");
        let unit_formatter = NumberFormatter::new(
            &locales::de_de(),
            NumberFormatOptions {
                style: NumberStyle::Unit(unit),
                unit_display: UnitDisplay::Short,
                min_fraction_digits: 1,
                max_fraction_digits: 1,
                ..NumberFormatOptions::default()
            },
        );

        let currency_text = currency.format_currency(1234.5, "USD");
        let unit_text = unit_formatter.format(37.0);

        assert_eq!(currency.parse(&currency_text), Some(1234.5));
        assert_eq!(unit_formatter.parse(&unit_text), Some(37.0));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn parse_returns_none_when_numeric_core_is_missing() {
        let formatter = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());

        assert_eq!(formatter.parse("not a number"), None);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn normalizes_arabic_indic_digits_during_parse() {
        let locale = locales::ar();
        let formatter = NumberFormatter::new(&locale, NumberFormatOptions::default());

        assert_eq!(formatter.parse("١٬٢٣٤٫٥٦"), Some(1234.56));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn respects_non_uniform_grouping() {
        let locale = Locale::parse("en-IN").expect("en-IN is a valid locale");
        let formatter = NumberFormatter::new(&locale, NumberFormatOptions::default());

        assert_eq!(formatter.format(1234567.0), "12,34,567");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formats_without_grouping_when_disabled() {
        let formatter = NumberFormatter::new(
            &locales::en_us(),
            NumberFormatOptions {
                use_grouping: false,
                ..NumberFormatOptions::default()
            },
        );

        assert_eq!(formatter.format(1234.56), "1234.56");
    }

    #[test]
    fn rounds_values_for_all_rounding_modes() {
        assert_eq!(round_value(1.25, 1, RoundingMode::HalfEven), 1.2);
        assert_eq!(round_value(1.35, 1, RoundingMode::HalfEven), 1.4);
        assert_eq!(round_value(2.5, 0, RoundingMode::HalfEven), 2.0);
        assert_eq!(round_value(3.5, 0, RoundingMode::HalfEven), 4.0);
        assert_eq!(round_value(-1.25, 1, RoundingMode::HalfUp), -1.3);
        assert_eq!(round_value(-1.25, 1, RoundingMode::HalfDown), -1.2);
        assert_eq!(round_value(1.21, 1, RoundingMode::Ceiling), 1.3);
        assert_eq!(round_value(-1.21, 1, RoundingMode::Floor), -1.3);
        assert_eq!(round_value(-1.29, 1, RoundingMode::Truncate), -1.2);
    }

    #[test]
    fn sign_display_variants_map_to_fixed_decimal_policies() {
        assert_eq!(
            SignDisplay::Auto.into_fixed_decimal(),
            FixedSignDisplay::Auto
        );
        assert_eq!(
            SignDisplay::Always.into_fixed_decimal(),
            FixedSignDisplay::Always
        );
        assert_eq!(
            SignDisplay::Never.into_fixed_decimal(),
            FixedSignDisplay::Never
        );
        assert_eq!(
            SignDisplay::ExceptZero.into_fixed_decimal(),
            FixedSignDisplay::ExceptZero
        );
        assert_eq!(
            SignDisplay::Negative.into_fixed_decimal(),
            FixedSignDisplay::Negative
        );
    }

    #[test]
    fn normalizes_currency_style_defaults_from_minor_units() {
        let bhd = normalize_style_defaults(NumberFormatOptions {
            style: NumberStyle::Currency(CurrencyCode::from_str("BHD").expect("valid code")),
            ..NumberFormatOptions::default()
        });
        let clf = normalize_style_defaults(NumberFormatOptions {
            style: NumberStyle::Currency(CurrencyCode::from_str("CLF").expect("valid code")),
            ..NumberFormatOptions::default()
        });
        let jpy = normalize_style_defaults(NumberFormatOptions {
            style: NumberStyle::Currency(CurrencyCode::JPY),
            ..NumberFormatOptions::default()
        });

        assert_eq!((bhd.min_fraction_digits, bhd.max_fraction_digits), (3, 3));
        assert_eq!((clf.min_fraction_digits, clf.max_fraction_digits), (4, 4));
        assert_eq!((jpy.min_fraction_digits, jpy.max_fraction_digits), (0, 0));
        assert_eq!(iso4217_minor_units(CurrencyCode::USD), 2);
    }

    #[test]
    fn parses_signed_and_grouped_numbers_with_locale_heuristics() {
        let en_us = locales::en_us();
        let de_de = locales::de_de();
        let ar = locales::ar();

        assert_eq!(parse_locale_number("-1,234.56", &en_us), Some(-1234.56));
        assert_eq!(parse_locale_number("+1,234,567", &en_us), Some(1234567.0));
        assert_eq!(parse_locale_number("+1.234", &de_de), Some(1234.0));
        assert_eq!(parse_locale_number("١٬٢٣٤", &ar), Some(1234.0));
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn caches_number_formatters_by_locale_and_options() {
        let options = NumberFormatOptions {
            min_fraction_digits: 2,
            max_fraction_digits: 2,
            ..NumberFormatOptions::default()
        };

        let cached = get_number_formatter(&locales::en_us(), &options);
        let direct = NumberFormatter::new(&locales::en_us(), options);

        assert_eq!(cached.format(1234.5), direct.format(1234.5));
        assert_eq!(cached.decimal_separator(), direct.decimal_separator());
        assert_eq!(cached.grouping_separator(), direct.grouping_separator());
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn repeated_cached_formatter_lookups_reuse_existing_entries() {
        let options = NumberFormatOptions {
            use_grouping: false,
            ..NumberFormatOptions::default()
        };

        let first = get_number_formatter(&locales::en_us(), &options);
        let second = get_number_formatter(&locales::en_us(), &options);

        assert_eq!(first.format(1234.56), second.format(1234.56));
        assert_eq!(first.grouping_separator(), second.grouping_separator());
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_range_uses_locale_specific_separators() {
        let formatter = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());

        assert_eq!(formatter.format_range(1.0, 2.5, &locales::en_us()), "1–2.5");
        assert_eq!(formatter.format_range(1.0, 2.5, &locales::fr()), "1 – 2.5");
        assert_eq!(formatter.format_range(1.0, 2.5, &locales::ja()), "1〜2.5");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn prepare_decimal_defaults_non_finite_values() {
        let formatter = NumberFormatter::new(&locales::en_us(), NumberFormatOptions::default());

        assert_eq!(formatter.prepare_decimal(f64::INFINITY), Decimal::default());
        assert_eq!(
            formatter.prepare_decimal(f64::NEG_INFINITY),
            Decimal::default()
        );
        assert_eq!(formatter.prepare_decimal(f64::NAN), Decimal::default());
    }

    #[test]
    fn choose_decimal_separator_treats_grouping_marks_as_non_decimal() {
        assert_eq!(choose_decimal_separator("1,234,567", '.', ','), None);
        assert_eq!(choose_decimal_separator("1.234", ',', '.'), None);
        assert_eq!(choose_decimal_separator("١٬٢٣٤", '٫', '٬'), None);
        assert_eq!(choose_decimal_separator("1.234.567", ',', ' '), None);
        assert_eq!(choose_decimal_separator("1,234.5", '.', ','), Some('.'));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn parse_separators_handles_suffixes_and_plain_digits() {
        assert_eq!(parse_separators("37 kg"), ('.', ' '));
        assert_eq!(parse_separators("1234"), ('.', ','));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn resolve_measure_unit_id_uses_embedded_cldr_id_when_present() {
        let mut unit =
            MeasureUnit::try_from_str("kilogram").expect("kilogram is a valid CLDR unit");
        unit.id = Some("kilogram");

        assert_eq!(
            resolve_measure_unit_id(&unit),
            Some(String::from("kilogram"))
        );
    }

    #[test]
    fn round_half_even_rounds_up_when_fraction_exceeds_half() {
        assert_eq!(round_half_even(1.6), 2.0);
        assert_eq!(round_half_even(-1.6), -2.0);
    }

    #[test]
    fn round_half_up_and_half_down_cover_non_tie_paths() {
        assert_eq!(round_half_away_from_zero(1.4), 1.0);
        assert_eq!(round_half_toward_zero(1.6), 2.0);
    }
}
