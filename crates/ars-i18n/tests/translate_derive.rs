//! Derive-contract coverage for application translation enums.

use ars_i18n::{IntlBackend, Locale, StubIntlBackend, Translate, locales};

#[derive(Clone, Debug, PartialEq, Eq, Translate)]
#[translate(fallback = "en")]
enum InventoryText {
    #[translate(en = "Inventory", pt_BR = "Inventario")]
    Title,

    #[translate(en = "{count} items", pt_BR = "{count} itens")]
    ItemCount { count: usize },

    #[translate(en = "Count", pt = "Contagem", pt_BR = "Contagem brasileira")]
    ExplicitBaseLanguage,

    #[translate(en = "Hello, {name}")]
    Greeting { name: String },

    #[translate(en = "{name}, meet {name}")]
    RepeatedPlaceholder { name: String },

    #[translate(locale = "en", text = "Color")]
    #[translate(locale = "pt_BR", text = "Cor")]
    ExplicitLocaleText,

    #[translate(
        en = "Script",
        en_US = "American script",
        zh_Hant_TW = "Traditional script"
    )]
    ScriptRegion,

    #[translate(en = "Press {{ to open and }} to close")]
    EscapedBracesOnly,
}

#[test]
fn translate_derive_uses_exact_locale_when_available() {
    let locale = Locale::parse("pt-BR").expect("locale should parse");

    assert_eq!(
        InventoryText::Title.translate(&locale, &StubIntlBackend),
        "Inventario"
    );
}

#[test]
fn translate_derive_uses_exact_locale_without_unicode_extensions() {
    let locale = Locale::parse("pt-BR-u-ca-gregory").expect("locale should parse");

    assert_eq!(
        InventoryText::Title.translate(&locale, &StubIntlBackend),
        "Inventario"
    );
}

#[test]
fn translate_derive_falls_back_to_language_before_fallback_locale() {
    let locale = Locale::parse("pt-PT").expect("locale should parse");

    assert_eq!(
        InventoryText::ExplicitBaseLanguage.translate(&locale, &StubIntlBackend),
        "Contagem"
    );
}

#[test]
fn translate_derive_does_not_fallback_to_regional_locale_for_language_match() {
    let locale = Locale::parse("pt-PT").expect("locale should parse");

    assert_eq!(
        InventoryText::ItemCount { count: 3 }.translate(&locale, &StubIntlBackend),
        "3 items"
    );
}

#[test]
fn translate_derive_falls_back_to_configured_locale() {
    assert_eq!(
        InventoryText::Greeting {
            name: String::from("Ada")
        }
        .translate(&locales::fr(), &StubIntlBackend),
        "Hello, Ada"
    );
}

#[test]
fn translate_derive_allows_repeated_placeholders() {
    assert_eq!(
        InventoryText::RepeatedPlaceholder {
            name: String::from("Ada")
        }
        .translate(&locales::en_us(), &StubIntlBackend),
        "Ada, meet Ada"
    );
}

#[test]
fn translate_derive_accepts_intl_backend_parameter_without_using_it() {
    fn assert_translate<T: Translate>(text: &T, locale: &Locale, intl: &dyn IntlBackend) -> String {
        text.translate(locale, intl)
    }

    assert_eq!(
        assert_translate(&InventoryText::Title, &locales::en_us(), &StubIntlBackend),
        "Inventory"
    );
}

#[test]
fn translate_derive_accepts_explicit_locale_text_attributes() {
    let locale = Locale::parse("pt-BR").expect("locale should parse");

    assert_eq!(
        InventoryText::ExplicitLocaleText.translate(&locale, &StubIntlBackend),
        "Cor"
    );
}

#[test]
fn translate_derive_normalizes_locale_identifiers_with_script_and_region() {
    let exact = Locale::parse("zh-Hant-TW").expect("locale should parse");
    let language = Locale::parse("zh-Hans-CN").expect("locale should parse");

    assert_eq!(
        InventoryText::ScriptRegion.translate(&exact, &StubIntlBackend),
        "Traditional script"
    );
    assert_eq!(
        InventoryText::ScriptRegion.translate(&language, &StubIntlBackend),
        "Script"
    );
}

#[test]
fn translate_derive_formats_escaped_braces_without_placeholders() {
    assert_eq!(
        InventoryText::EscapedBracesOnly.translate(&locales::en_us(), &StubIntlBackend),
        "Press { to open and } to close"
    );
}

#[test]
fn translate_derive_ui_tests() {
    let cases = trybuild::TestCases::new();

    cases.compile_fail("tests/ui/translate_*.rs");
}
