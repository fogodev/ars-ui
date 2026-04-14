use std::cmp::Ordering;

use crate::{Locale, locales};

/// Detect locale from an HTTP `Accept-Language` header.
///
/// Returns the best matching locale from `supported`, preferring exact matches,
/// then language-only matches, then the first supported locale. When
/// `supported` is empty, this falls back to `en-US`.
#[must_use]
pub fn locale_from_accept_language(accept_language: &str, supported: &[Locale]) -> Locale {
    let mut preferences = accept_language
        .split(',')
        .filter_map(|part| {
            let mut iter = part.trim().splitn(2, ";q=");

            let tag = iter.next()?.trim().to_string();

            let quality = iter.next().and_then(|q| q.parse().ok()).unwrap_or(1.0);

            Some((tag, quality))
        })
        .collect::<Vec<_>>();

    preferences.sort_by(|(_, left), (_, right)| right.partial_cmp(left).unwrap_or(Ordering::Equal));

    for (tag, _) in &preferences {
        if let Ok(locale) = Locale::parse(tag) {
            if supported.contains(&locale) {
                return locale;
            }

            if let Ok(language_locale) = Locale::parse(locale.language()) {
                if let Some(matched) = supported.iter().find(|supported_locale| {
                    supported_locale.language() == language_locale.language()
                }) {
                    return matched.clone();
                }
            }
        }
    }

    supported.first().cloned().unwrap_or_else(locales::en_us)
}

#[cfg(test)]
mod tests {
    use super::locale_from_accept_language;
    use crate::{Locale, locales};

    #[test]
    fn accepts_exact_supported_match() {
        let supported = [locales::en_us(), locales::de()];

        let locale = locale_from_accept_language("en-US,en;q=0.9,de;q=0.8", &supported);

        assert_eq!(locale, locales::en_us());
    }

    #[test]
    fn falls_back_to_language_only_match() {
        let supported = [
            Locale::parse("pt").expect("pt should parse"),
            locales::en_us(),
        ];

        let locale = locale_from_accept_language("pt-BR", &supported);

        assert_eq!(locale, Locale::parse("pt").expect("pt should parse"));
    }

    #[test]
    fn prefers_higher_quality_matches() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("en;q=0.4,de;q=0.8", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn malformed_quality_values_default_to_one() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("de;q=abc,en-US;q=", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn preserves_order_for_nan_quality_values() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("en;q=NaN,de;q=NaN", &supported);

        assert_eq!(locale, locales::en_us());
    }

    #[test]
    fn skips_invalid_locale_tags_and_uses_next_valid_preference() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("@@@,de;q=0.8", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn ignores_empty_header_entries() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language(" , ,de;q=0.8", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn falls_back_to_first_supported_locale_when_no_match_exists() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("fr-CA,fr;q=0.9", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn falls_back_to_en_us_when_supported_locales_are_empty() {
        let locale = locale_from_accept_language("fr-CA,fr;q=0.9", &[]);

        assert_eq!(locale, locales::en_us());
    }
}
