//! Locale-aware text case transformation helpers.
//!
//! Rust's built-in Unicode case conversion is locale-independent, which means
//! it misses language-sensitive mappings such as Turkish dotted and dotless I
//! and Greek final sigma handling. These helpers delegate to ICU4X
//! [`CaseMapper`](icu::casemap::CaseMapper) or the browser's locale-aware
//! string APIs so components can apply the spec-defined behavior consistently
//! across backends.

use alloc::string::String;

#[cfg(feature = "icu4x")]
use icu::casemap::CaseMapper;
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use js_sys::JsString;

use crate::Locale;

/// Locale-aware uppercase transformation.
///
/// Delegates to ICU4X [`CaseMapper`] so locale-specific mappings such as
/// Turkish dotted capital I are preserved. Under the wasm `web-intl` backend,
/// this delegates to browser `String.prototype.toLocaleUpperCase()`. On
/// non-wasm builds with `web-intl`, it falls back to Unicode case conversion so
/// the public API remains available in feature-matrix builds.
#[must_use]
#[cfg(feature = "icu4x")]
pub fn to_uppercase(text: &str, locale: &Locale) -> String {
    CaseMapper::new()
        .uppercase_to_string(text, locale.language_identifier())
        .into_owned()
}

/// Locale-aware uppercase transformation.
///
/// Delegates to browser `String.prototype.toLocaleUpperCase()` on wasm
/// `web-intl` builds.
#[must_use]
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
pub fn to_uppercase(text: &str, locale: &Locale) -> String {
    let locale_tag = locale.to_bcp47();
    String::from(JsString::from(text).to_locale_upper_case(Some(locale_tag.as_str())))
}

/// Locale-aware uppercase transformation.
///
/// Non-wasm `web-intl` builds cannot access the browser `Intl` APIs, so this
/// falls back to Rust's Unicode case mapping while preserving the shared public
/// API surface.
#[must_use]
#[cfg(all(
    feature = "web-intl",
    not(target_arch = "wasm32"),
    not(feature = "icu4x")
))]
pub fn to_uppercase(text: &str, _locale: &Locale) -> String {
    text.to_uppercase()
}

/// Locale-aware lowercase transformation.
///
/// Delegates to ICU4X [`CaseMapper`] so locale-specific mappings such as
/// Turkish dotless i and Greek final sigma are preserved. Under the wasm
/// `web-intl` backend, this delegates to browser
/// `String.prototype.toLocaleLowerCase()`. On non-wasm builds with `web-intl`,
/// it falls back to Unicode case conversion so the public API remains
/// available in feature-matrix builds.
#[must_use]
#[cfg(feature = "icu4x")]
pub fn to_lowercase(text: &str, locale: &Locale) -> String {
    CaseMapper::new()
        .lowercase_to_string(text, locale.language_identifier())
        .into_owned()
}

/// Unicode case folding for case-insensitive comparison.
///
/// Returns the canonical case-fold form per Unicode Technical Report 21 —
/// the right primitive for case-insensitive **matching** (as opposed to
/// case **transformation**, which is what [`to_lowercase`] / [`to_uppercase`]
/// do). The key differences for matching:
///
/// - German eszett: `fold("ß") == fold("SS") == "ss"`, so `"ß"` and `"ss"`
///   match each other (whereas `to_lowercase("ß") == "ß"`).
/// - Greek final sigma collapses into the medial form, so a query against
///   text containing `"Ος"` matches `"ΟΣ"` and vice versa.
/// - Turkic dotted/dotless I: when `locale` is a Turkic language (`tr` /
///   `az`), the implementation switches to ICU4X
///   `CaseMapper::fold_turkic_string` so `İ ↔ i` and `I ↔ ı` per the
///   Turkic case-folding convention. Non-Turkic locales use the default
///   Unicode fold.
///
/// Available only under the `icu4x` feature; the `web-intl` backend has
/// no equivalent (browser `Intl` exposes only locale-aware lowercase /
/// uppercase) so callers requiring case-fold-based matching must use the
/// ICU4X backend.
#[must_use]
#[cfg(feature = "icu4x")]
pub fn case_fold(text: &str, locale: &Locale) -> String {
    let mapper = CaseMapper::new();

    if is_turkic_locale(locale) {
        mapper.fold_turkic_string(text).into_owned()
    } else {
        mapper.fold_string(text).into_owned()
    }
}

/// Returns `true` for locales that use Turkic case-folding rules — Turkish
/// (`tr`) and Azerbaijani (`az`).
///
/// These are the language tags ICU treats as Turkic for case mapping per
/// CLDR's `special-casing` data, which is the same set that
/// `CaseMapper::lowercase_to_string` already switches behaviour on. Other
/// Turkic family languages (Kazakh `kk`, Tatar `tt`, Uzbek `uz`) follow
/// regular Unicode case rules in CLDR.
#[cfg(feature = "icu4x")]
fn is_turkic_locale(locale: &Locale) -> bool {
    matches!(locale.language(), "tr" | "az")
}

/// Locale-aware lowercase transformation.
///
/// Delegates to browser `String.prototype.toLocaleLowerCase()` on wasm
/// `web-intl` builds.
#[must_use]
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
pub fn to_lowercase(text: &str, locale: &Locale) -> String {
    let locale_tag = locale.to_bcp47();
    String::from(JsString::from(text).to_locale_lower_case(Some(locale_tag.as_str())))
}

/// Locale-aware lowercase transformation.
///
/// Non-wasm `web-intl` builds cannot access the browser `Intl` APIs, so this
/// falls back to Rust's Unicode case mapping while preserving the shared public
/// API surface.
#[must_use]
#[cfg(all(
    feature = "web-intl",
    not(target_arch = "wasm32"),
    not(feature = "icu4x")
))]
pub fn to_lowercase(text: &str, _locale: &Locale) -> String {
    text.to_lowercase()
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "icu4x",
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use super::{to_lowercase, to_uppercase};
    #[cfg(any(
        feature = "icu4x",
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::Locale;

    #[cfg(feature = "icu4x")]
    #[test]
    fn turkish_uppercase_uses_dotted_capital_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_uppercase("i", &locale), "İ");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn turkish_lowercase_uses_dotless_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_lowercase("I", &locale), "ı");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn german_uppercase_expands_sharp_s() {
        let locale = Locale::parse("de").expect("locale should parse");

        assert_eq!(to_uppercase("ß", &locale), "SS");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn lithuanian_uppercase_handles_dotted_i() {
        let locale = Locale::parse("lt").expect("locale should parse");

        assert_eq!(to_uppercase("i\u{307}", &locale), "I");
    }

    #[cfg(any(
        feature = "icu4x",
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn english_round_trips_ascii_text() {
        let locale = Locale::parse("en-US").expect("locale should parse");

        assert_eq!(to_uppercase("Hello world", &locale), "HELLO WORLD");
        assert_eq!(to_lowercase("HELLO WORLD", &locale), "hello world");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn greek_lowercase_applies_final_sigma_rules() {
        let locale = Locale::parse("el").expect("locale should parse");

        assert_eq!(to_lowercase("ΟΣ", &locale), "ος");
    }

    // ── case_fold (Unicode case folding for case-insensitive match) ──

    #[cfg(feature = "icu4x")]
    #[test]
    fn fold_expands_eszett_to_ss() {
        // Per Unicode TR21, `case_fold("ß") == "ss"` and `case_fold("SS")
        // == "ss"`, so the two are equivalent for case-insensitive
        // matching. (Lowercase preserves `ß`; only the fold form expands.)
        let de = Locale::parse("de").expect("locale should parse");

        assert_eq!(super::case_fold("ß", &de), "ss");
        assert_eq!(super::case_fold("SS", &de), "ss");
        assert_eq!(super::case_fold("Straße", &de), "strasse");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn fold_collapses_greek_final_sigma_into_medial_form() {
        // Final sigma `ς` and medial sigma `σ` both fold to `σ`, and
        // capital `Σ` folds to the same. So queries against text mixing
        // the three forms still match under case_fold.
        let el = Locale::parse("el").expect("locale should parse");

        assert_eq!(super::case_fold("ΟΣ", &el), super::case_fold("Ος", &el));
        assert_eq!(super::case_fold("σ", &el), super::case_fold("ς", &el));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn fold_turkic_locale_uses_turkic_fold() {
        // Under `tr` / `az`, `İ ↔ i` and `I ↔ ı`. Under non-Turkic
        // locales the standard Unicode fold applies (`İ → i̇`).
        let tr = Locale::parse("tr").expect("locale should parse");
        let en = Locale::parse("en-US").expect("locale should parse");

        // Turkic: dotted/dotless I pairings collapse.
        assert_eq!(super::case_fold("İ", &tr), super::case_fold("i", &tr));
        assert_eq!(super::case_fold("I", &tr), super::case_fold("ı", &tr));

        // Non-Turkic: `İ` does NOT fold to plain `i`.
        assert_ne!(super::case_fold("İ", &en), "i");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn fold_is_idempotent() {
        // Folding an already-folded string is a no-op (the fold form is
        // its own canonical representative).
        for input in ["hello", "Straße", "İstanbul", "ΟΣ"] {
            let de = Locale::parse("de").expect("locale should parse");
            let folded = super::case_fold(input, &de);

            assert_eq!(super::case_fold(&folded, &de), folded);
        }
    }

    #[cfg(all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    ))]
    #[test]
    fn non_wasm_web_intl_keeps_case_api_available() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_uppercase("Hello world", &locale), "HELLO WORLD");
        assert_eq!(to_lowercase("HELLO WORLD", &locale), "hello world");
    }
}

#[cfg(all(
    test,
    feature = "web-intl",
    target_arch = "wasm32",
    not(feature = "icu4x")
))]
mod web_intl_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{to_lowercase, to_uppercase};
    use crate::Locale;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn web_intl_turkish_uppercase_uses_dotted_capital_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_uppercase("i", &locale), "İ");
    }

    #[wasm_bindgen_test]
    fn web_intl_turkish_lowercase_uses_dotless_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_lowercase("I", &locale), "ı");
    }

    #[wasm_bindgen_test]
    fn web_intl_german_uppercase_expands_sharp_s() {
        let locale = Locale::parse("de").expect("locale should parse");

        assert_eq!(to_uppercase("ß", &locale), "SS");
    }

    #[wasm_bindgen_test]
    fn web_intl_lithuanian_uppercase_handles_dotted_i() {
        let locale = Locale::parse("lt").expect("locale should parse");

        assert_eq!(to_uppercase("i\u{307}", &locale), "I");
    }

    #[wasm_bindgen_test]
    fn web_intl_english_round_trips_ascii_text() {
        let locale = Locale::parse("en-US").expect("locale should parse");

        assert_eq!(to_uppercase("Hello world", &locale), "HELLO WORLD");
        assert_eq!(to_lowercase("HELLO WORLD", &locale), "hello world");
    }

    #[wasm_bindgen_test]
    fn web_intl_greek_lowercase_applies_final_sigma_rules() {
        let locale = Locale::parse("el").expect("locale should parse");

        assert_eq!(to_lowercase("ΟΣ", &locale), "ος");
    }
}
