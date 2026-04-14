use std::cmp::Ordering;

use crate::{Locale, locales};

#[derive(Clone)]
enum PreferenceTag {
    Wildcard,
    Locale(Locale),
    Language(String),
}

impl PreferenceTag {
    fn matches_specific_locale(&self, locale: &Locale) -> bool {
        match self {
            Self::Wildcard => false,
            Self::Locale(tag) => tag == locale,
            Self::Language(language) => locale.language() == language,
        }
    }
}

/// Detect locale from an HTTP `Accept-Language` header.
///
/// Returns the best matching locale from `supported`, preferring exact matches,
/// then language-only matches, then the first supported locale. When
/// `supported` is empty, this falls back to `en-US`.
#[must_use]
pub fn locale_from_accept_language(accept_language: &str, supported: &[Locale]) -> Locale {
    let mut preferences = accept_language
        .split(',')
        .filter_map(parse_preference)
        .collect::<Vec<_>>();

    preferences.sort_by(|(_, left), (_, right)| right.partial_cmp(left).unwrap_or(Ordering::Equal));

    let specific_ranges = preferences
        .iter()
        .filter_map(|(tag, _)| match tag {
            PreferenceTag::Wildcard => None,
            specific => Some(specific),
        })
        .collect::<Vec<_>>();

    for (tag, quality) in &preferences {
        if *quality <= 0.0 {
            continue;
        }

        match tag {
            PreferenceTag::Wildcard => {
                if let Some(matched) = supported.iter().find(|supported_locale| {
                    !specific_ranges
                        .iter()
                        .any(|range| range.matches_specific_locale(supported_locale))
                }) {
                    return matched.clone();
                }
            }
            PreferenceTag::Locale(locale) => {
                if supported.contains(locale) {
                    return locale.clone();
                }

                if let Some(matched) = supported
                    .iter()
                    .find(|supported_locale| supported_locale.language() == locale.language())
                {
                    return matched.clone();
                }
            }
            PreferenceTag::Language(language) => {
                if let Some(matched) = supported
                    .iter()
                    .find(|supported_locale| supported_locale.language() == *language)
                {
                    return matched.clone();
                }
            }
        }
    }

    supported.first().cloned().unwrap_or_else(locales::en_us)
}

fn parse_preference(part: &str) -> Option<(PreferenceTag, f32)> {
    let mut segments = part.trim().split(';');
    let tag = segments.next()?.trim();

    if tag.is_empty() {
        return None;
    }

    let quality = segments
        .find_map(|parameter| {
            let (name, value) = parameter.split_once('=')?;

            if name.trim().eq_ignore_ascii_case("q") {
                Some(parse_quality(value.trim()))
            } else {
                None
            }
        })
        .unwrap_or(1.0);

    Some((parse_tag(tag)?, quality))
}

fn parse_tag(tag: &str) -> Option<PreferenceTag> {
    if tag == "*" {
        return Some(PreferenceTag::Wildcard);
    }

    let locale = Locale::parse(tag).ok()?;

    if tag.eq_ignore_ascii_case(locale.language()) {
        Some(PreferenceTag::Language(locale.language().to_string()))
    } else {
        Some(PreferenceTag::Locale(locale))
    }
}

fn parse_quality(value: &str) -> f32 {
    match value.parse::<f32>() {
        Ok(quality) if quality.is_finite() => quality,
        _ => 1.0,
    }
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
    fn accepts_q_parameters_with_optional_whitespace() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("en-US; q=0.4, de; q=0.8", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn accepts_case_insensitive_q_parameter_names() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("en-US;Q=0.4,de;Q=0.8", &supported);

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
    fn skips_zero_quality_ranges() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("de;q=0,en-US;q=0.5", &supported);

        assert_eq!(locale, locales::en_us());
    }

    #[test]
    fn skips_language_only_fallback_for_rejected_locale_range() {
        let supported = [
            Locale::parse("de").expect("de should parse"),
            locales::en_us(),
        ];

        let locale = locale_from_accept_language("de-DE;q=0,en-US;q=0.5", &supported);

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
    fn wildcard_matches_first_supported_locale_without_specific_range() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("en-US;q=0.4,*;q=0.8", &supported);

        assert_eq!(locale, locales::de());
    }

    #[test]
    fn wildcard_skips_supported_locales_with_specific_ranges() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("de;q=0.5,*;q=0.8", &supported);

        assert_eq!(locale, locales::en_us());
    }

    #[test]
    fn wildcard_does_not_override_specific_rejections() {
        let supported = [locales::de(), locales::en_us()];

        let locale = locale_from_accept_language("de;q=0,*;q=0.8", &supported);

        assert_eq!(locale, locales::en_us());
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
