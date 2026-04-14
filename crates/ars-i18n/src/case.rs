//! Locale-aware text case transformation helpers.
//!
//! Rust's built-in Unicode case conversion is locale-independent, which means
//! it misses language-sensitive mappings such as Turkish dotted and dotless I
//! and Greek final sigma handling. These helpers delegate to ICU4X
//! [`CaseMapper`](icu::casemap::CaseMapper) so components can apply the
//! spec-defined locale-aware behavior consistently.

use alloc::string::String;

use icu::casemap::CaseMapper;

use crate::Locale;

/// Locale-aware uppercase transformation.
///
/// Delegates to ICU4X [`CaseMapper`] so locale-specific mappings such as
/// Turkish dotted capital I are preserved.
#[must_use]
pub fn to_uppercase(text: &str, locale: &Locale) -> String {
    CaseMapper::new()
        .uppercase_to_string(text, locale.language_identifier())
        .into_owned()
}

/// Locale-aware lowercase transformation.
///
/// Delegates to ICU4X [`CaseMapper`] so locale-specific mappings such as
/// Turkish dotless i and Greek final sigma are preserved.
#[must_use]
pub fn to_lowercase(text: &str, locale: &Locale) -> String {
    CaseMapper::new()
        .lowercase_to_string(text, locale.language_identifier())
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::{to_lowercase, to_uppercase};
    use crate::Locale;

    #[test]
    fn turkish_uppercase_uses_dotted_capital_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_uppercase("i", &locale), "İ");
    }

    #[test]
    fn turkish_lowercase_uses_dotless_i() {
        let locale = Locale::parse("tr").expect("locale should parse");

        assert_eq!(to_lowercase("I", &locale), "ı");
    }

    #[test]
    fn german_uppercase_expands_sharp_s() {
        let locale = Locale::parse("de").expect("locale should parse");

        assert_eq!(to_uppercase("ß", &locale), "SS");
    }

    #[test]
    fn lithuanian_uppercase_handles_dotted_i() {
        let locale = Locale::parse("lt").expect("locale should parse");

        assert_eq!(to_uppercase("i\u{307}", &locale), "I");
    }

    #[test]
    fn english_round_trips_ascii_text() {
        let locale = Locale::parse("en-US").expect("locale should parse");

        assert_eq!(to_uppercase("Hello world", &locale), "HELLO WORLD");
        assert_eq!(to_lowercase("HELLO WORLD", &locale), "hello world");
    }

    #[test]
    fn greek_lowercase_applies_final_sigma_rules() {
        let locale = Locale::parse("el").expect("locale should parse");

        assert_eq!(to_lowercase("ΟΣ", &locale), "ος");
    }
}
