//! Plural and ordinal category selection helpers.
//!
//! This module exposes the spec-defined `ars-i18n` plural API with backend
//! selection hidden behind cargo features.
//!
//! - `icu4x` uses ICU4X plural rules with CLDR data compiled into the binary.
//! - `web-intl` uses the browser's `Intl.PluralRules` implementation on
//!   `wasm32` targets.
//! - With neither backend enabled, or with `web-intl` enabled on a non-wasm
//!   target, the public API falls back to English-only selection rules so
//!   `ars-i18n` still builds in feature-matrix checks and backend-free tests.

use alloc::{format, string::String};

use icu::plurals::PluralCategory as IcuPluralCategory;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
use {
    core::fmt::{self, Debug},
    js_sys::{Array, Intl::PluralRules as JsPluralRules, Object, Reflect},
    wasm_bindgen::JsValue,
};
#[cfg(feature = "icu4x")]
use {
    fixed_decimal::{Decimal, FloatPrecision},
    icu::plurals::{PluralRules, PluralRulesPreferences},
};

use crate::Locale;

/// CLDR plural categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    /// Zero quantity.
    Zero,
    /// Singular quantity.
    One,
    /// Dual quantity.
    Two,
    /// Paucal or small quantity.
    Few,
    /// Large quantity.
    Many,
    /// Required fallback category.
    Other,
}

impl PluralCategory {
    /// Converts an ICU4X plural category into the ars-ui wrapper type.
    #[must_use]
    pub const fn from_icu(category: IcuPluralCategory) -> Self {
        match category {
            IcuPluralCategory::Zero => Self::Zero,
            IcuPluralCategory::One => Self::One,
            IcuPluralCategory::Two => Self::Two,
            IcuPluralCategory::Few => Self::Few,
            IcuPluralCategory::Many => Self::Many,
            IcuPluralCategory::Other => Self::Other,
        }
    }
}

/// A map from plural categories to localized values.
#[derive(Clone, Debug)]
pub struct Plural<T: Clone> {
    /// Value for the `zero` category.
    pub zero: Option<T>,
    /// Value for the `one` category.
    pub one: Option<T>,
    /// Value for the `two` category.
    pub two: Option<T>,
    /// Value for the `few` category.
    pub few: Option<T>,
    /// Value for the `many` category.
    pub many: Option<T>,
    /// Required fallback value used for `other` and any unset category.
    pub other: T,
}

impl<T: Clone> Plural<T> {
    /// Creates a plural map with only the required `other` value populated.
    #[must_use]
    pub const fn from_other(other: T) -> Self {
        Self {
            zero: None,
            one: None,
            two: None,
            few: None,
            many: None,
            other,
        }
    }

    /// Returns the value for the given category, falling back to `other`.
    #[must_use]
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
    /// Cardinal numbers such as `1 item` or `2 items`.
    Cardinal,
    /// Ordinal numbers such as `1st`, `2nd`, or `3rd`.
    Ordinal,
}

/// Common interface for plural-rule backends.
///
/// Under `icu4x` and wasm `web-intl`, this trait delegates to the active
/// locale-aware backend. Without either backend feature, or on non-wasm builds
/// with `web-intl`, the free functions and wrapper types use the crate's
/// English-only fallback behavior.
pub trait PluralRulesFormat {
    /// Selects the CLDR plural category for `number`.
    fn select(&self, number: f64) -> PluralCategory;
}

/// ICU4X-backed plural rules wrapper used by the default backend.
#[cfg(feature = "icu4x")]
#[derive(Clone, Debug)]
pub struct Icu4xPluralRules {
    locale: Locale,
    rule_type: PluralRuleType,
}

#[cfg(feature = "icu4x")]
impl Icu4xPluralRules {
    /// Creates a new ICU4X-backed plural rules handle.
    #[must_use]
    pub fn new(locale: &Locale, rule_type: PluralRuleType) -> Self {
        Self {
            locale: locale.clone(),
            rule_type,
        }
    }
}

#[cfg(feature = "icu4x")]
impl PluralRulesFormat for Icu4xPluralRules {
    fn select(&self, number: f64) -> PluralCategory {
        select_plural(&self.locale, number, self.rule_type)
    }
}

/// Default plural-rules backend for the active feature set.
#[cfg(feature = "icu4x")]
pub type DefaultPluralRules = Icu4xPluralRules;

/// Browser `Intl.PluralRules` wrapper used by the `web-intl` backend.
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
#[derive(Clone)]
pub struct JsIntlPluralRules {
    locale: Locale,
    rule_type: PluralRuleType,
    inner: JsPluralRules,
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
impl JsIntlPluralRules {
    /// Creates a new browser-backed plural rules handle.
    #[must_use]
    pub fn new(locale: &Locale, rule_type: PluralRuleType) -> Self {
        Self {
            locale: locale.clone(),
            rule_type,
            inner: build_js_plural_rules(locale, rule_type),
        }
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
impl Debug for JsIntlPluralRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsIntlPluralRules")
            .field("locale", &self.locale)
            .field("rule_type", &self.rule_type)
            .finish()
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
impl PluralRulesFormat for JsIntlPluralRules {
    fn select(&self, number: f64) -> PluralCategory {
        PluralCategory::from_js_category(&self.inner.select(number).as_string().unwrap_or_default())
    }
}

/// Default plural-rules backend for the active feature set.
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub type DefaultPluralRules = JsIntlPluralRules;

/// Returns the CLDR cardinal plural category for an integer count.
///
/// With `icu4x` or wasm `web-intl`, this uses locale-aware CLDR cardinal rules.
/// Without either backend feature, or on non-wasm builds with `web-intl`, it
/// falls back to English cardinal behavior: `1 => One`, all other values =>
/// `Other`.
#[must_use]
#[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
pub fn plural_category(count: usize, locale: &Locale) -> PluralCategory {
    plural_category_with_backend(count, locale)
}

/// Returns the CLDR cardinal plural category for an integer count.
///
/// With `icu4x` or wasm `web-intl`, this uses locale-aware CLDR cardinal rules.
/// Without either backend feature, or on non-wasm builds with `web-intl`, it
/// falls back to English cardinal behavior: `1 => One`, all other values =>
/// `Other`.
#[must_use]
#[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
pub const fn plural_category(count: usize, locale: &Locale) -> PluralCategory {
    plural_category_with_backend(count, locale)
}

/// Returns the CLDR plural category for a number and rule type.
///
/// With `icu4x` or wasm `web-intl`, this uses locale-aware CLDR plural rules.
/// Without either backend feature, or on non-wasm builds with `web-intl`, it
/// falls back to English-only behavior: cardinal rules use `One` for exactly
/// `1.0`, and ordinal rules use the standard English `1st`/`2nd`/`3rd`
/// categories.
#[must_use]
pub fn select_plural(locale: &Locale, count: f64, rule_type: PluralRuleType) -> PluralCategory {
    select_plural_with_backend(locale, count, rule_type)
}

/// Formats a pluralized template by selecting the correct category and
/// replacing `{name}` placeholders from `args`.
#[must_use]
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
    let mut result = String::from(template);

    for (key, value) in args {
        result = result.replace(&format!("{{{key}}}"), value);
    }

    result
}

#[cfg(feature = "icu4x")]
fn plural_category_with_backend(count: usize, locale: &Locale) -> PluralCategory {
    let rules = PluralRules::try_new_cardinal(PluralRulesPreferences::from(locale.as_icu()))
        .expect("compiled_data guarantees plural rules are available for all locales");

    PluralCategory::from_icu(rules.category_for(count))
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
fn plural_category_with_backend(count: usize, locale: &Locale) -> PluralCategory {
    select_plural_with_backend(locale, count as f64, PluralRuleType::Cardinal)
}

#[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
const fn plural_category_with_backend(count: usize, locale: &Locale) -> PluralCategory {
    let _ = locale;

    if count == 1 {
        PluralCategory::One
    } else {
        PluralCategory::Other
    }
}

#[cfg(feature = "icu4x")]
fn select_plural_with_backend(
    locale: &Locale,
    count: f64,
    rule_type: PluralRuleType,
) -> PluralCategory {
    let rules = match rule_type {
        PluralRuleType::Cardinal => {
            PluralRules::try_new_cardinal(PluralRulesPreferences::from(locale.as_icu()))
        }
        PluralRuleType::Ordinal => {
            PluralRules::try_new_ordinal(PluralRulesPreferences::from(locale.as_icu()))
        }
    }
    .expect("compiled_data guarantees plural rules are available for all locales");

    let decimal = Decimal::try_from_f64(count, FloatPrecision::RoundTrip).unwrap_or_default();

    PluralCategory::from_icu(rules.category_for(&decimal))
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
fn select_plural_with_backend(
    locale: &Locale,
    count: f64,
    rule_type: PluralRuleType,
) -> PluralCategory {
    JsIntlPluralRules::new(locale, rule_type).select(count)
}

#[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
fn select_plural_with_backend(
    _locale: &Locale,
    count: f64,
    rule_type: PluralRuleType,
) -> PluralCategory {
    match rule_type {
        PluralRuleType::Cardinal => {
            if count == 1.0 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }

        PluralRuleType::Ordinal => fallback_english_ordinal_category(count),
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
fn build_js_plural_rules(locale: &Locale, rule_type: PluralRuleType) -> JsPluralRules {
    let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

    let options = Object::new();

    Reflect::set(
        &options,
        &JsValue::from_str("type"),
        &JsValue::from_str(rule_type.as_js_str()),
    )
    .expect("Reflect::set on Intl.PluralRules options object");

    JsPluralRules::new(&locales, &options)
}

#[cfg(not(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32"))))]
fn fallback_english_ordinal_category(count: f64) -> PluralCategory {
    if !count.is_finite() || count.fract() != 0.0 {
        return PluralCategory::Other;
    }

    let n = count.abs().trunc() as i64;

    let mod10 = n % 10;

    let mod100 = n % 100;

    if (11..=13).contains(&mod100) {
        return PluralCategory::Other;
    }

    match mod10 {
        1 => PluralCategory::One,
        2 => PluralCategory::Two,
        3 => PluralCategory::Few,
        _ => PluralCategory::Other,
    }
}

impl PluralCategory {
    #[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
    fn from_js_category(category: &str) -> Self {
        match category {
            "zero" => Self::Zero,
            "one" => Self::One,
            "two" => Self::Two,
            "few" => Self::Few,
            "many" => Self::Many,
            _ => Self::Other,
        }
    }
}

impl PluralRuleType {
    #[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
    const fn as_js_str(self) -> &'static str {
        match self {
            Self::Cardinal => "cardinal",
            Self::Ordinal => "ordinal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "icu4x")]
    use crate::locales;

    #[test]
    fn plural_category_from_icu_maps_exhaustively() {
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::Zero),
            PluralCategory::Zero
        );
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::One),
            PluralCategory::One
        );
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::Two),
            PluralCategory::Two
        );
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::Few),
            PluralCategory::Few
        );
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::Many),
            PluralCategory::Many
        );
        assert_eq!(
            PluralCategory::from_icu(IcuPluralCategory::Other),
            PluralCategory::Other
        );
    }

    #[test]
    fn plural_from_other_and_get_fall_back_to_other() {
        let plural = Plural::from_other("items");

        assert_eq!(plural.get(PluralCategory::One), &"items");
        assert_eq!(plural.get(PluralCategory::Few), &"items");
        assert_eq!(plural.get(PluralCategory::Other), &"items");
    }

    #[test]
    fn plural_get_prefers_explicit_category_over_other() {
        let mut plural = Plural::from_other("items");

        plural.one = Some("item");
        plural.few = Some("items-few");

        assert_eq!(plural.get(PluralCategory::One), &"item");
        assert_eq!(plural.get(PluralCategory::Few), &"items-few");
    }

    #[test]
    fn plural_get_covers_zero_two_and_many_slots() {
        let mut plural = Plural::from_other("items");

        plural.zero = Some("none");
        plural.two = Some("pair");
        plural.many = Some("many-items");

        assert_eq!(plural.get(PluralCategory::Zero), &"none");
        assert_eq!(plural.get(PluralCategory::Two), &"pair");
        assert_eq!(plural.get(PluralCategory::Many), &"many-items");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn plural_category_uses_english_cardinal_rules() {
        let locale = locales::en();

        assert_eq!(plural_category(0, &locale), PluralCategory::Other);
        assert_eq!(plural_category(1, &locale), PluralCategory::One);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn select_plural_uses_english_ordinal_rules() {
        let locale = locales::en();

        assert_eq!(
            select_plural(&locale, 1.0, PluralRuleType::Ordinal),
            PluralCategory::One
        );
        assert_eq!(
            select_plural(&locale, 2.0, PluralRuleType::Ordinal),
            PluralCategory::Two
        );
        assert_eq!(
            select_plural(&locale, 3.0, PluralRuleType::Ordinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locale, 4.0, PluralRuleType::Ordinal),
            PluralCategory::Other
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn select_plural_handles_fractional_english_cardinals() {
        let locale = locales::en();

        assert_eq!(
            select_plural(&locale, 1.0, PluralRuleType::Cardinal),
            PluralCategory::One
        );
        assert_eq!(
            select_plural(&locale, 1.5, PluralRuleType::Cardinal),
            PluralCategory::Other
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn select_plural_uses_arabic_cardinal_rules() {
        let locale = locales::ar();

        assert_eq!(
            select_plural(&locale, 3.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locale, 11.0, PluralRuleType::Cardinal),
            PluralCategory::Many
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn select_plural_uses_polish_cardinal_rules() {
        let locale = Locale::parse("pl").expect("pl is a valid locale");

        assert_eq!(
            select_plural(&locale, 1.0, PluralRuleType::Cardinal),
            PluralCategory::One
        );
        assert_eq!(
            select_plural(&locale, 2.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locale, 5.0, PluralRuleType::Cardinal),
            PluralCategory::Many
        );
        assert_eq!(
            select_plural(&locale, 22.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn select_plural_uses_welsh_cardinal_rules() {
        let locale = Locale::parse("cy").expect("cy is a valid locale");

        assert_eq!(
            select_plural(&locale, 0.0, PluralRuleType::Cardinal),
            PluralCategory::Zero
        );
        assert_eq!(
            select_plural(&locale, 1.0, PluralRuleType::Cardinal),
            PluralCategory::One
        );
        assert_eq!(
            select_plural(&locale, 2.0, PluralRuleType::Cardinal),
            PluralCategory::Two
        );
        assert_eq!(
            select_plural(&locale, 3.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locale, 6.0, PluralRuleType::Cardinal),
            PluralCategory::Many
        );
        assert_eq!(
            select_plural(&locale, 4.0, PluralRuleType::Cardinal),
            PluralCategory::Other
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_plural_selects_branch_and_interpolates_arguments() {
        let locale = locales::en();

        let mut plural = Plural::from_other("{count} items selected");

        plural.one = Some("{count} item selected");

        assert_eq!(
            format_plural(&locale, 1.0, &plural, &[("count", "1")]),
            "1 item selected"
        );
        assert_eq!(
            format_plural(&locale, 3.0, &plural, &[("count", "3")]),
            "3 items selected"
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn icu4x_plural_rules_wrapper_implements_shared_trait() {
        let locale = locales::en();

        let rules = Icu4xPluralRules::new(&locale, PluralRuleType::Ordinal);

        assert_eq!(rules.select(1.0), PluralCategory::One);
        assert_eq!(rules.select(4.0), PluralCategory::Other);
    }

    #[cfg(all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    ))]
    #[test]
    fn native_web_intl_feature_uses_english_fallback_rules() {
        let locale = Locale::parse("pl").expect("pl should parse");

        assert_eq!(plural_category(1, &locale), PluralCategory::One);
        assert_eq!(plural_category(2, &locale), PluralCategory::Other);
        assert_eq!(
            select_plural(&locale, 2.0, PluralRuleType::Ordinal),
            PluralCategory::Two
        );
    }
}

#[cfg(all(test, feature = "web-intl", target_arch = "wasm32"))]
mod web_intl_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;
    use crate::locales;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn js_intl_plural_rules_wrapper_implements_shared_trait() {
        let locale = locales::en();

        let rules = JsIntlPluralRules::new(&locale, PluralRuleType::Ordinal);

        assert_eq!(rules.select(1.0), PluralCategory::One);
        assert_eq!(rules.select(4.0), PluralCategory::Other);
    }

    #[wasm_bindgen_test]
    fn js_intl_plural_rules_debug_includes_wrapper_state() {
        let locale = locales::en();

        let rules = JsIntlPluralRules::new(&locale, PluralRuleType::Ordinal);

        let debug = format!("{rules:?}");

        assert!(debug.contains("JsIntlPluralRules"));
        assert!(debug.contains("locale"));
        assert!(debug.contains("rule_type"));
    }

    #[wasm_bindgen_test]
    fn web_intl_plural_category_uses_english_cardinal_rules() {
        let locale = locales::en();

        assert_eq!(plural_category(0, &locale), PluralCategory::Other);
        assert_eq!(plural_category(1, &locale), PluralCategory::One);
    }

    #[wasm_bindgen_test]
    fn web_intl_select_plural_uses_english_ordinal_rules() {
        let locale = locales::en();

        assert_eq!(
            select_plural(&locale, 1.0, PluralRuleType::Ordinal),
            PluralCategory::One
        );
        assert_eq!(
            select_plural(&locale, 2.0, PluralRuleType::Ordinal),
            PluralCategory::Two
        );
        assert_eq!(
            select_plural(&locale, 3.0, PluralRuleType::Ordinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locale, 4.0, PluralRuleType::Ordinal),
            PluralCategory::Other
        );
    }

    #[wasm_bindgen_test]
    fn web_intl_select_plural_uses_representative_cardinal_locales() {
        let polish = Locale::parse("pl").expect("pl is a valid locale");

        let welsh = Locale::parse("cy").expect("cy is a valid locale");

        assert_eq!(
            select_plural(&locales::ar(), 3.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&locales::ar(), 11.0, PluralRuleType::Cardinal),
            PluralCategory::Many
        );
        assert_eq!(
            select_plural(&polish, 22.0, PluralRuleType::Cardinal),
            PluralCategory::Few
        );
        assert_eq!(
            select_plural(&welsh, 6.0, PluralRuleType::Cardinal),
            PluralCategory::Many
        );
    }

    #[wasm_bindgen_test]
    fn web_intl_format_plural_selects_branch_and_interpolates_arguments() {
        let locale = locales::en();

        let mut plural = Plural::from_other("{count} items selected");

        plural.one = Some("{count} item selected");

        assert_eq!(
            format_plural(&locale, 1.0, &plural, &[("count", "1")]),
            "1 item selected"
        );
        assert_eq!(
            format_plural(&locale, 3.0, &plural, &[("count", "3")]),
            "3 items selected"
        );
    }
}
