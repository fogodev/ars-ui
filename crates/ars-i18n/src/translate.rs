//! User-defined translation trait for application copy.
//!
//! Library component text uses [`ComponentMessages`](ars_core::ComponentMessages) in `ars-core`. [`Translate`]
//! is the separate user-facing contract for application-owned text rendered via
//! adapter helpers such as `t()`.

use alloc::string::String;

use crate::{IcuProvider, Locale};

/// Trait for user-defined translatable text.
///
/// Users define an enum with one variant per translatable string. Unit variants
/// cover static strings while data-carrying variants support parameterized text.
/// Implementations should match on locale first, keep English as the fallback
/// arm, and use the provided ICU handle for locale-sensitive formatting when
/// needed.
pub trait Translate {
    /// Produces the localized text for this value.
    fn translate(&self, locale: &Locale, icu: &dyn IcuProvider) -> String;
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use super::Translate;
    use crate::{Locale, StubIcuProvider};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum InventoryText {
        Title,
        Welcome,
        ItemCount { count: usize },
    }

    impl Translate for InventoryText {
        fn translate(&self, locale: &Locale, _icu: &dyn crate::IcuProvider) -> String {
            match locale.language() {
                "es" => match self {
                    Self::Title => String::from("Inventario"),
                    Self::Welcome => String::from("Bienvenido"),
                    Self::ItemCount { count } => format!("{count} elementos"),
                },
                "ar" => match self {
                    Self::Title => String::from("المخزون"),
                    Self::Welcome => String::from("أهلا بك"),
                    Self::ItemCount { count } => format!("{count} عناصر"),
                },
                _ => match self {
                    Self::Title => String::from("Inventory"),
                    Self::Welcome => String::from("Welcome"),
                    Self::ItemCount { count } => format!("{count} items"),
                },
            }
        }
    }

    #[test]
    fn translate_trait_supports_unit_and_parameterized_variants() {
        let locale = Locale::parse("en-US").expect("locale should parse");
        let icu = StubIcuProvider;

        assert_eq!(InventoryText::Title.translate(&locale, &icu), "Inventory");
        assert_eq!(
            InventoryText::ItemCount { count: 3 }.translate(&locale, &icu),
            "3 items"
        );
    }

    #[test]
    fn translate_trait_supports_spanish_and_arabic_variants() {
        let spanish = Locale::parse("es-ES").expect("locale should parse");
        let arabic = Locale::parse("ar-EG").expect("locale should parse");
        let icu = StubIcuProvider;

        assert_eq!(
            InventoryText::Welcome.translate(&spanish, &icu),
            "Bienvenido"
        );
        assert_eq!(InventoryText::Title.translate(&arabic, &icu), "المخزون");
    }

    #[test]
    fn translate_trait_covers_remaining_locale_branches() {
        let english = Locale::parse("en-US").expect("locale should parse");
        let spanish = Locale::parse("es-MX").expect("locale should parse");
        let arabic = Locale::parse("ar-SA").expect("locale should parse");
        let icu = StubIcuProvider;

        assert_eq!(InventoryText::Welcome.translate(&english, &icu), "Welcome");
        assert_eq!(InventoryText::Title.translate(&spanish, &icu), "Inventario");
        assert_eq!(
            InventoryText::ItemCount { count: 2 }.translate(&spanish, &icu),
            "2 elementos"
        );
        assert_eq!(InventoryText::Welcome.translate(&arabic, &icu), "أهلا بك");
        assert_eq!(
            InventoryText::ItemCount { count: 4 }.translate(&arabic, &icu),
            "4 عناصر"
        );
    }

    #[test]
    fn translate_trait_works_with_stub_icu_provider() {
        let locale = Locale::parse("en-US").expect("locale should parse");
        let icu = StubIcuProvider;

        assert_eq!(
            InventoryText::ItemCount { count: 1 }.translate(&locale, &icu),
            "1 items"
        );
    }
}
