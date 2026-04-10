use alloc::string::String;
use core::{cmp::Ordering, fmt};

use icu::locale::{LanguageIdentifier, Locale as IcuLocale};
use icu_provider::DataLocale;

use crate::{Direction, Weekday};

/// A BCP 47 locale identifier.
///
/// Wraps ICU4X's locale type with ars-ui-specific helpers for directionality,
/// Unicode extension access, and provider interop.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(IcuLocale);

impl Locale {
    /// Parse from a BCP 47 string.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleParseError`] when `s` is not a valid BCP 47 locale tag.
    pub fn parse(s: &str) -> Result<Self, LocaleParseError> {
        Ok(Self(s.parse::<IcuLocale>().map_err(LocaleParseError)?))
    }

    /// Create a locale from a known language identifier.
    #[must_use]
    pub fn from_langid(langid: LanguageIdentifier) -> Self {
        Self(IcuLocale {
            id: langid,
            extensions: Default::default(),
        })
    }

    /// Returns the text direction for this locale.
    #[must_use]
    pub fn direction(&self) -> Direction {
        if RTL_SCRIPTS.contains(&self.script_or_default()) {
            Direction::Rtl
        } else {
            Direction::Ltr
        }
    }

    /// Returns `true` if this locale uses right-to-left text.
    #[must_use]
    pub fn is_rtl(&self) -> bool {
        self.direction() == Direction::Rtl
    }

    /// Returns the locale's BCP 47 string representation.
    #[must_use]
    pub fn to_bcp47(&self) -> String {
        self.0.to_string()
    }

    /// Returns the language subtag.
    #[must_use]
    pub const fn language(&self) -> &str {
        self.0.id.language.as_str()
    }

    /// Returns the optional script subtag.
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.0
            .id
            .script
            .as_ref()
            .map(icu::locale::subtags::Script::as_str)
    }

    /// Returns the optional region subtag.
    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.0
            .id
            .region
            .as_ref()
            .map(icu::locale::subtags::Region::as_str)
    }

    /// Returns the calendar system requested by the `u-ca-*` Unicode extension.
    #[must_use]
    pub fn calendar_extension(&self) -> Option<&str> {
        self.0
            .extensions
            .unicode
            .keywords
            .get(&icu::locale::extensions::unicode::key!("ca"))
            .and_then(|value| {
                value
                    .as_single_subtag()
                    .map(icu::locale::subtags::Subtag::as_str)
            })
    }

    /// Returns the first day of week requested by the `u-fw-*` Unicode extension.
    #[must_use]
    pub fn first_day_of_week_extension(&self) -> Option<Weekday> {
        self.0
            .extensions
            .unicode
            .keywords
            .get(&icu::locale::extensions::unicode::key!("fw"))
            .and_then(|value| value.as_single_subtag())
            .and_then(|subtag| Weekday::from_bcp47_fw(subtag.as_str()))
    }

    /// Converts this locale to the ICU4X provider locale type.
    #[must_use]
    pub fn to_data_locale(&self) -> DataLocale {
        (&self.0).into()
    }

    #[cfg(feature = "icu4x")]
    pub(crate) const fn as_icu(&self) -> &IcuLocale {
        &self.0
    }

    fn script_or_default(&self) -> &str {
        self.script().unwrap_or_else(|| match self.language() {
            "ar" | "fa" | "ur" | "ps" | "ug" | "sd" | "ks" => "Arab",
            "he" | "yi" => "Hebr",
            "dv" => "Thaa",
            "nqo" => "Nkoo",
            "pa" if self.region() == Some("PK") => "Arab",
            "ku" if self.region() == Some("IQ") => "Arab",
            _ => "Latn",
        })
    }
}

/// Error returned when parsing a locale string fails.
#[derive(Debug)]
pub struct LocaleParseError(pub icu::locale::ParseError);

impl fmt::Display for LocaleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ars-ui locale parse error: {}", self.0)
    }
}

impl core::error::Error for LocaleParseError {}

impl PartialOrd for Locale {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Locale {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_bcp47().cmp(&other.to_bcp47())
    }
}

/// Scripts that use right-to-left text direction.
const RTL_SCRIPTS: &[&str] = &[
    "Arab", "Hebr", "Thaa", "Syrc", "Tfng", "Adlm", "Rohg", "Mand", "Nbat", "Palm", "Nkoo", "Samr",
];

/// Common pre-defined locales.
pub mod locales {
    use super::Locale;

    /// Returns a fallback locale used when a hard-coded locale tag fails to parse.
    fn fallback() -> Locale {
        Locale::parse("en-US").expect("en-US must always be a valid locale")
    }

    /// Returns the canonical Brazilian (A.K.A. Brazilian Portuguese - "pt-BR") locale.
    #[must_use]
    pub fn br() -> Locale {
        Locale::parse("pt-BR").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical English language locale.
    #[must_use]
    pub fn en() -> Locale {
        Locale::parse("en").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical American English locale.
    #[must_use]
    pub fn en_us() -> Locale {
        Locale::parse("en-US").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical British English locale.
    #[must_use]
    pub fn en_gb() -> Locale {
        Locale::parse("en-GB").unwrap_or_else(|_| fallback())
    }

    /// Returns the Arabic language locale.
    #[must_use]
    pub fn ar() -> Locale {
        Locale::parse("ar").unwrap_or_else(|_| fallback())
    }

    /// Returns the Saudi Arabic locale.
    #[must_use]
    pub fn ar_sa() -> Locale {
        Locale::parse("ar-SA").unwrap_or_else(|_| fallback())
    }

    /// Returns the Egyptian Arabic locale.
    #[must_use]
    pub fn ar_eg() -> Locale {
        Locale::parse("ar-EG").unwrap_or_else(|_| fallback())
    }

    /// Returns the Hebrew language locale.
    #[must_use]
    pub fn he() -> Locale {
        Locale::parse("he").unwrap_or_else(|_| fallback())
    }

    /// Returns the Persian language locale.
    #[must_use]
    pub fn fa() -> Locale {
        Locale::parse("fa").unwrap_or_else(|_| fallback())
    }

    /// Returns the German language locale.
    #[must_use]
    pub fn de() -> Locale {
        Locale::parse("de").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical German locale.
    #[must_use]
    pub fn de_de() -> Locale {
        Locale::parse("de-DE").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical French locale.
    #[must_use]
    pub fn fr() -> Locale {
        Locale::parse("fr-FR").unwrap_or_else(|_| fallback())
    }

    /// Returns the Japanese language locale.
    #[must_use]
    pub fn ja() -> Locale {
        Locale::parse("ja").unwrap_or_else(|_| fallback())
    }

    /// Returns the canonical Japanese locale.
    #[must_use]
    pub fn ja_jp() -> Locale {
        Locale::parse("ja-JP").unwrap_or_else(|_| fallback())
    }

    /// Returns the Simplified Chinese locale.
    #[must_use]
    pub fn zh_hans() -> Locale {
        Locale::parse("zh-Hans").unwrap_or_else(|_| fallback())
    }

    /// Returns the Korean language locale.
    #[must_use]
    pub fn ko() -> Locale {
        Locale::parse("ko").unwrap_or_else(|_| fallback())
    }
}
