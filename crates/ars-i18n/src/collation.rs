//! Locale-aware string sorting and comparison helpers.
//!
//! This module exposes the spec-defined collation API with backend selection
//! hidden behind cargo features.
//!
//! - `icu4x` uses ICU4X collators with CLDR data compiled into the binary.
//! - `web-intl` on `wasm32` uses the browser's `Intl.Collator` implementation.
//! - Without either backend, or on non-wasm `web-intl` builds, comparison
//!   falls back to Rust's byte-order `Ord` implementation so the public API
//!   remains available in feature-matrix checks.

use alloc::{string::String, vec::Vec};
use core::cmp::Ordering;

#[cfg(feature = "icu4x")]
use icu::collator::{
    Collator as OwnedCollator,
    options::{CollatorOptions as IcuCollatorOptions, Strength},
    preferences::CollationNumericOrdering,
};
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use {
    js_sys::{
        Array, Function,
        Intl::{Collator as JsCollator, CollatorOptions as JsCollatorOptions},
    },
    wasm_bindgen::JsValue,
};

use crate::Locale;

/// Comparison strength for locale-aware string collation.
#[cfg(feature = "icu4x")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CollationStrength {
    /// Compare base letters only, ignoring accents and case.
    Primary,
    /// Compare base letters and accents, ignoring case.
    Secondary,
    /// Compare base letters, accents, and case.
    #[default]
    Tertiary,
    /// Compare punctuation and symbol differences in addition to tertiary data.
    Quaternary,
}

/// Comparison strength for locale-aware string collation.
///
/// This fallback definition keeps downstream code compiling when ICU4X is not
/// enabled. In that configuration, [`StringCollator`] degrades to Rust's
/// default byte-order comparison regardless of the selected strength.
#[cfg(not(feature = "icu4x"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CollationStrength {
    /// Compare base letters only, ignoring accents and case.
    Primary,
    /// Compare base letters and accents, ignoring case.
    Secondary,
    /// Compare base letters, accents, and case.
    #[default]
    Tertiary,
    /// Compare punctuation and symbol differences in addition to tertiary data.
    Quaternary,
}

/// Options controlling locale-aware string comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollationOptions {
    /// The comparison sensitivity to use.
    pub strength: CollationStrength,

    /// Whether case differences should be ignored.
    ///
    /// When `true`, the effective sensitivity is forced to secondary-strength
    /// behavior so case is ignored while accent differences remain observable.
    pub case_insensitive: bool,

    /// Whether numeric substrings should be ordered by numeric value.
    ///
    /// When enabled, strings such as `"file9"` and `"file10"` sort in natural
    /// numeric order instead of pure lexicographic order.
    pub numeric: bool,
}

impl Default for CollationOptions {
    fn default() -> Self {
        Self {
            strength: CollationStrength::Tertiary,
            case_insensitive: false,
            numeric: true,
        }
    }
}

/// Common interface for collation backends.
pub trait CollationFormat {
    /// Compare two strings according to locale-aware collation rules.
    fn compare(&self, a: &str, b: &str) -> Ordering;
}

/// A locale-aware string collator.
///
/// The concrete backend varies by feature set:
/// - `icu4x`: ICU4X `Collator`
/// - `web-intl` on `wasm32`: browser `Intl.Collator`
/// - otherwise: Rust byte-order comparison
#[cfg(feature = "icu4x")]
#[derive(Debug)]
pub struct StringCollator {
    collator: OwnedCollator,
}

#[cfg(feature = "icu4x")]
impl StringCollator {
    /// Create a new locale-aware string collator.
    #[must_use]
    pub fn new(locale: &Locale, options: CollationOptions) -> Self {
        let mut icu_options = IcuCollatorOptions::default();
        icu_options.strength = Some(icu_strength(options));

        let mut preferences = icu::collator::CollatorPreferences::from(locale.as_icu());
        if options.numeric && preferences.numeric_ordering.is_none() {
            preferences.numeric_ordering = Some(CollationNumericOrdering::True);
        }

        let collator = OwnedCollator::try_new(preferences, icu_options)
            .expect("compiled_data guarantees collation data is available for all locales")
            .static_to_owned();

        Self { collator }
    }

    /// Compare two strings according to locale-aware collation rules.
    #[must_use]
    pub fn compare(&self, a: &str, b: &str) -> Ordering {
        self.collator.as_borrowed().compare(a, b)
    }
}

/// A locale-aware string collator.
///
/// The concrete backend varies by feature set:
/// - `icu4x`: ICU4X `Collator`
/// - `web-intl` on `wasm32`: browser `Intl.Collator`
/// - otherwise: Rust byte-order comparison
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
#[derive(Debug)]
pub struct StringCollator {
    collator: JsCollator,
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
impl StringCollator {
    /// Create a new locale-aware string collator using `Intl.Collator`.
    #[must_use]
    pub fn new(locale: &Locale, options: CollationOptions) -> Self {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));
        let js_options = JsCollatorOptions::new();
        js_options.set_sensitivity(js_sensitivity(options));
        js_options.set_ignore_punctuation(js_ignore_punctuation(options));
        js_options.set_numeric(options.numeric);

        let collator = JsCollator::new(&locales, js_options.as_ref());

        Self { collator }
    }

    /// Compare two strings according to locale-aware collation rules.
    #[must_use]
    pub fn compare(&self, a: &str, b: &str) -> Ordering {
        let compare: Function = self.collator.compare();
        let value = compare
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from_str(a),
                &JsValue::from_str(b),
            )
            .expect("Intl.Collator.compare should not throw for string inputs");
        let value = value
            .as_f64()
            .expect("Intl.Collator.compare should return a numeric ordering");

        if value < 0.0 {
            Ordering::Less
        } else if value > 0.0 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// A locale-aware string collator.
///
/// In feature configurations without a runtime collation backend, this type
/// preserves the shared API surface while falling back to byte-order
/// comparison.
#[cfg(any(
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
#[derive(Debug)]
pub struct StringCollator {}

#[cfg(any(
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
impl StringCollator {
    /// Create a fallback collator using Rust's byte-order comparison.
    #[must_use]
    pub const fn new(_locale: &Locale, _options: CollationOptions) -> Self {
        Self {}
    }

    /// Compare two strings using Rust's byte-order comparison.
    #[must_use]
    pub fn compare(&self, a: &str, b: &str) -> Ordering {
        a.cmp(b)
    }
}

// Valid for all backends
impl StringCollator {
    /// Sort a vector of strings in-place according to collation rules.
    #[expect(clippy::ptr_arg, reason = "API matches the specification")]
    pub fn sort(&self, items: &mut Vec<String>) {
        items.sort_by(|a, b| self.compare(a, b));
    }

    /// Sort items by a string key according to collation rules.
    #[expect(clippy::ptr_arg, reason = "API matches the specification")]
    pub fn sort_by_key<T, F: Fn(&T) -> &str>(&self, items: &mut Vec<T>, key: F) {
        items.sort_by(|a, b| self.compare(key(a), key(b)));
    }
}

impl CollationFormat for StringCollator {
    fn compare(&self, a: &str, b: &str) -> Ordering {
        self.compare(a, b)
    }
}

#[cfg(feature = "icu4x")]
const fn icu_strength(options: CollationOptions) -> Strength {
    if options.case_insensitive {
        Strength::Secondary
    } else {
        match options.strength {
            CollationStrength::Primary => Strength::Primary,
            CollationStrength::Secondary => Strength::Secondary,
            CollationStrength::Tertiary => Strength::Tertiary,
            CollationStrength::Quaternary => Strength::Quaternary,
        }
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
const fn js_sensitivity(options: CollationOptions) -> js_sys::Intl::CollatorSensitivity {
    if options.case_insensitive {
        js_sys::Intl::CollatorSensitivity::Accent
    } else {
        match options.strength {
            CollationStrength::Primary => js_sys::Intl::CollatorSensitivity::Base,
            CollationStrength::Secondary => js_sys::Intl::CollatorSensitivity::Accent,
            // `Intl.Collator` has no "accent + case, but not punctuation" mode.
            // `Variant` + `ignorePunctuation = true` is the closest approximation.
            CollationStrength::Tertiary | CollationStrength::Quaternary => {
                js_sys::Intl::CollatorSensitivity::Variant
            }
        }
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
const fn js_ignore_punctuation(options: CollationOptions) -> bool {
    if options.case_insensitive {
        true
    } else {
        !matches!(options.strength, CollationStrength::Quaternary)
    }
}

#[cfg(all(
    test,
    not(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))
))]
mod tests {
    use alloc::{string::ToString, vec};
    use core::cmp::Ordering;

    #[cfg(feature = "icu4x")]
    use super::CollationStrength;
    use super::{CollationFormat, CollationOptions, StringCollator};
    #[cfg(feature = "icu4x")]
    use crate::Locale;
    use crate::locales;

    fn compare_with_trait(collator: &dyn CollationFormat, a: &str, b: &str) -> Ordering {
        collator.compare(a, b)
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn primary_strength_ignores_case_and_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Primary,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Cafe", "café"), Ordering::Equal);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn tertiary_strength_distinguishes_case_and_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Tertiary,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_ne!(collator.compare("Cafe", "café"), Ordering::Equal);
        assert_ne!(collator.compare("Cafe", "cafe"), Ordering::Equal);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn quaternary_strength_keeps_case_and_accent_differences() {
        let options = CollationOptions {
            strength: CollationStrength::Quaternary,
            numeric: false,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_ne!(collator.compare("Cafe", "café"), Ordering::Equal);
        assert_ne!(collator.compare("Cafe", "cafe"), Ordering::Equal);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn case_insensitive_overrides_strength_to_secondary_behavior() {
        let options = CollationOptions {
            strength: CollationStrength::Tertiary,
            case_insensitive: true,
            numeric: false,
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Resume", "resume"), Ordering::Equal);
        assert_ne!(collator.compare("resume", "résumé"), Ordering::Equal);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn numeric_sort_orders_embedded_numbers_naturally() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec![
            "file10".to_string(),
            "file9".to_string(),
            "file2".to_string(),
        ];

        collator.sort(&mut items);

        assert_eq!(items, vec!["file2", "file9", "file10"]);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn german_locale_sorts_umlauts_adjacent_to_base_letter() {
        let locale = Locale::parse("de").expect("de should parse");
        let collator = StringCollator::new(&locale, CollationOptions::default());
        let mut items = vec!["z".to_string(), "ä".to_string(), "a".to_string()];

        collator.sort(&mut items);

        assert_eq!(items[2], "z");
        assert!(items[..2].contains(&"a".to_string()));
        assert!(items[..2].contains(&"ä".to_string()));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn sort_by_key_uses_collation_rules() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec![("file10", 10_u8), ("file2", 2_u8), ("file9", 9_u8)];

        collator.sort_by_key(&mut items, |item| item.0);

        assert_eq!(items, vec![("file2", 2), ("file9", 9), ("file10", 10)]);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn secondary_strength_ignores_case_but_respects_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Secondary,
            numeric: false,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Resume", "resume"), Ordering::Equal);
        assert_ne!(collator.compare("resume", "résumé"), Ordering::Equal);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn locale_numeric_extension_is_not_overridden() {
        let locale = Locale::parse("en-u-kn-false").expect("locale should parse");
        let options = CollationOptions {
            numeric: true,
            ..Default::default()
        };
        let collator = StringCollator::new(&locale, options);

        assert_eq!(collator.compare("file10", "file9"), Ordering::Less);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn collator_implements_shared_trait() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());

        assert_eq!(
            compare_with_trait(&collator, "file9", "file10"),
            Ordering::Less
        );
    }

    #[cfg(not(any(
        feature = "icu4x",
        all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x"))
    )))]
    #[test]
    fn fallback_compare_uses_byte_ordering() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());

        assert_eq!(collator.compare("file10", "file9"), Ordering::Less);
    }

    #[cfg(not(any(
        feature = "icu4x",
        all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x"))
    )))]
    #[test]
    fn fallback_sort_keeps_shared_api_available() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec!["file9".to_string(), "file10".to_string()];

        collator.sort(&mut items);

        assert_eq!(items, vec!["file10", "file9"]);
    }

    #[cfg(not(any(
        feature = "icu4x",
        all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x"))
    )))]
    #[test]
    fn fallback_sort_by_key_keeps_shared_api_available() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec![("file9", 9_u8), ("file10", 10_u8)];

        collator.sort_by_key(&mut items, |item| item.0);

        assert_eq!(items, vec![("file10", 10), ("file9", 9)]);
    }

    #[cfg(not(any(
        feature = "icu4x",
        all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x"))
    )))]
    #[test]
    fn fallback_collator_implements_shared_trait() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());

        assert_eq!(
            compare_with_trait(&collator, "file10", "file9"),
            Ordering::Less
        );
    }
}

#[cfg(all(
    test,
    feature = "web-intl",
    target_arch = "wasm32",
    not(feature = "icu4x")
))]
mod web_intl_tests {
    use alloc::{string::ToString, vec};
    use core::cmp::Ordering;

    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{CollationFormat, CollationOptions, CollationStrength, StringCollator};
    use crate::{Locale, locales};

    wasm_bindgen_test_configure!(run_in_browser);

    fn compare_with_trait(collator: &dyn CollationFormat, a: &str, b: &str) -> Ordering {
        collator.compare(a, b)
    }

    #[wasm_bindgen_test]
    fn web_intl_constructor_creates_collator() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());

        assert_eq!(collator.compare("a", "a"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_primary_strength_ignores_case_and_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Primary,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Cafe", "café"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_tertiary_strength_respects_case_and_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Tertiary,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_ne!(collator.compare("cafe", "café"), Ordering::Equal);
        assert_ne!(collator.compare("Cafe", "cafe"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_quaternary_strength_keeps_case_accent_and_punctuation_differences() {
        let options = CollationOptions {
            strength: CollationStrength::Quaternary,
            numeric: false,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_ne!(collator.compare("Cafe", "café"), Ordering::Equal);
        assert_ne!(collator.compare("Cafe", "cafe"), Ordering::Equal);
        assert_ne!(collator.compare("ab", "a-b"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_tertiary_strength_ignores_punctuation_differences() {
        let options = CollationOptions {
            strength: CollationStrength::Tertiary,
            numeric: false,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("ab", "a-b"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_numeric_sort_orders_embedded_numbers_naturally() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec![
            "file10".to_string(),
            "file9".to_string(),
            "file2".to_string(),
        ];

        collator.sort(&mut items);

        assert_eq!(items, vec!["file2", "file9", "file10"]);
    }

    #[wasm_bindgen_test]
    fn web_intl_case_insensitive_uses_accent_sensitivity() {
        let options = CollationOptions {
            strength: CollationStrength::Tertiary,
            case_insensitive: true,
            numeric: false,
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Resume", "resume"), Ordering::Equal);
        assert_ne!(collator.compare("resume", "résumé"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_secondary_strength_ignores_case_but_respects_accents() {
        let options = CollationOptions {
            strength: CollationStrength::Secondary,
            numeric: false,
            ..Default::default()
        };

        let collator = StringCollator::new(&locales::en(), options);

        assert_eq!(collator.compare("Resume", "resume"), Ordering::Equal);
        assert_ne!(collator.compare("resume", "résumé"), Ordering::Equal);
    }

    #[wasm_bindgen_test]
    fn web_intl_sort_by_key_uses_collation_rules() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());
        let mut items = vec![("file10", 10_u8), ("file2", 2_u8), ("file9", 9_u8)];

        collator.sort_by_key(&mut items, |item| item.0);

        assert_eq!(items, vec![("file2", 2), ("file9", 9), ("file10", 10)]);
    }

    #[wasm_bindgen_test]
    fn web_intl_collator_implements_shared_trait() {
        let collator = StringCollator::new(&locales::en(), CollationOptions::default());

        assert_eq!(
            compare_with_trait(&collator, "file9", "file10"),
            Ordering::Less
        );
    }

    #[wasm_bindgen_test]
    fn web_intl_german_locale_sorts_umlauts_adjacent_to_base_letter() {
        let locale = Locale::parse("de").expect("de should parse");
        let collator = StringCollator::new(&locale, CollationOptions::default());
        let mut items = vec!["z".to_string(), "ä".to_string(), "a".to_string()];

        collator.sort(&mut items);

        assert_eq!(items[2], "z");
        assert!(items[..2].contains(&"a".to_string()));
        assert!(items[..2].contains(&"ä".to_string()));
    }
}
