//! Strategy-aware conversion from [`ars_core::AttrMap`] to Leptos attributes.
//!
//! This module bridges the framework-agnostic connect output used across `ars-ui`
//! to the stringly DOM attribute representation Leptos spreads onto elements.

use ars_core::{AttrMap, AttrValue, CssProperty, StyleStrategy};

use crate::provider::{current_ars_context, warn_missing_provider};

/// Result of converting an [`AttrMap`] into Leptos-ready attributes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LeptosAttrResult {
    /// HTML attribute tuples ready to spread onto a Leptos element.
    pub attrs: Vec<(String, String)>,

    /// CSS properties to apply via CSSOM when [`StyleStrategy::Cssom`] is active.
    pub cssom_styles: Vec<(CssProperty, String)>,

    /// Nonce-safe CSS rule text collected when [`StyleStrategy::Nonce`] is active.
    pub nonce_css: String,
}

/// Converts an [`AttrMap`] into Leptos attributes using the provided style strategy.
///
/// Style entries are handled according to `strategy`:
/// - [`StyleStrategy::Inline`] emits a `style` attribute
/// - [`StyleStrategy::Cssom`] returns style pairs in [`LeptosAttrResult::cssom_styles`]
/// - [`StyleStrategy::Nonce`] emits `data-ars-style-id` and scoped CSS rule text
///
/// `element_id` is required when `strategy` is [`StyleStrategy::Nonce`].
#[must_use]
pub fn attr_map_to_leptos(
    map: AttrMap,
    strategy: &StyleStrategy,
    element_id: Option<&str>,
) -> LeptosAttrResult {
    let parts = map.into_parts();

    let mut attrs = parts
        .attrs
        .into_iter()
        .filter_map(|(key, value)| match value {
            AttrValue::String(text) => Some((key.to_string(), text)),
            AttrValue::Bool(true) => Some((key.to_string(), String::new())),
            AttrValue::Bool(false) | AttrValue::None => None,
        })
        .collect::<Vec<_>>();

    let mut cssom_styles = Vec::new();

    let mut nonce_css = String::new();

    match strategy {
        StyleStrategy::Inline => {
            if !parts.styles.is_empty() {
                let style = parts
                    .styles
                    .into_iter()
                    .map(|(property, value)| format!("{property}: {value};"))
                    .collect::<Vec<_>>()
                    .join(" ");

                attrs.push((String::from("style"), style));
            }
        }

        StyleStrategy::Cssom => {
            cssom_styles = parts.styles;
        }

        StyleStrategy::Nonce(_) => {
            if !parts.styles.is_empty() {
                let id = element_id.expect("element_id is required for Nonce style strategy");

                attrs.push((String::from("data-ars-style-id"), String::from(id)));

                nonce_css = styles_to_nonce_css(id, &parts.styles);
            }
        }
    }

    LeptosAttrResult {
        attrs,
        cssom_styles,
        nonce_css,
    }
}

/// Applies CSS properties directly to an element via CSSOM.
#[cfg(not(feature = "ssr"))]
pub fn apply_styles_cssom(el: &leptos::web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    let style = el.style();

    for (property, value) in styles {
        drop(style.set_property(&property.to_string(), value));
    }
}

/// Returns the current provider-configured style strategy.
///
/// Falls back to [`StyleStrategy::Inline`] when no provider context is present.
#[must_use]
pub fn use_style_strategy() -> StyleStrategy {
    current_ars_context().map_or_else(
        || {
            warn_missing_provider("use_style_strategy");
            StyleStrategy::default()
        },
        |context| context.style_strategy().clone(),
    )
}

fn styles_to_nonce_css(id: &str, styles: &[(CssProperty, String)]) -> String {
    let declarations = styles
        .iter()
        .map(|(property, value)| format!("  {property}: {value};"))
        .collect::<Vec<_>>()
        .join("\n");

    format!("[data-ars-style-id=\"{id}\"] {{\n{declarations}\n}}")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ars_core::{AttrMap, CssProperty, HtmlAttr, I18nRegistries, NullPlatformEffects};
    use leptos::prelude::Owner;

    use super::*;

    #[test]
    fn inline_strategy_emits_style_attribute() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Id, "button-id");
        map.set_style(CssProperty::Display, "inline-flex");
        map.set_style(CssProperty::Width, "10px");

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert!(result.cssom_styles.is_empty());
        assert!(result.nonce_css.is_empty());
        assert_eq!(
            result.attrs,
            vec![
                (String::from("id"), String::from("button-id")),
                (
                    String::from("style"),
                    String::from("width: 10px; display: inline-flex;")
                )
            ]
        );
    }

    #[test]
    fn inline_strategy_skips_style_attribute_when_no_styles_exist() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Id, "button-id");

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(
            result.attrs,
            vec![(String::from("id"), String::from("button-id"))]
        );
        assert!(result.cssom_styles.is_empty());
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn cssom_strategy_returns_cssom_styles() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Title, "tooltip");
        map.set_style(CssProperty::Display, "grid");
        map.set_style(CssProperty::Width, "20px");

        let result = attr_map_to_leptos(map, &StyleStrategy::Cssom, None);

        assert_eq!(
            result.attrs,
            vec![(String::from("title"), String::from("tooltip"))]
        );
        assert_eq!(
            result.cssom_styles,
            vec![
                (CssProperty::Width, String::from("20px")),
                (CssProperty::Display, String::from("grid"))
            ]
        );
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn nonce_strategy_emits_selector_and_css_text() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "root");
        map.set_style(CssProperty::Display, "flex");

        let result = attr_map_to_leptos(
            map,
            &StyleStrategy::Nonce(String::from("nonce-123")),
            Some("checkbox-root"),
        );

        assert_eq!(
            result.attrs,
            vec![
                (String::from("class"), String::from("root")),
                (
                    String::from("data-ars-style-id"),
                    String::from("checkbox-root")
                )
            ]
        );
        assert!(result.cssom_styles.is_empty());
        assert_eq!(
            result.nonce_css,
            "[data-ars-style-id=\"checkbox-root\"] {\n  display: flex;\n}"
        );
    }

    #[test]
    fn nonce_strategy_skips_selector_and_css_when_no_styles_exist() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "root");

        let result = attr_map_to_leptos(
            map,
            &StyleStrategy::Nonce(String::from("nonce-123")),
            Some("checkbox-root"),
        );

        assert_eq!(
            result.attrs,
            vec![(String::from("class"), String::from("root"))]
        );
        assert!(result.cssom_styles.is_empty());
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn bool_false_and_none_are_filtered_while_bool_true_is_empty_string() {
        let mut map = AttrMap::new();

        map.set_bool(HtmlAttr::Disabled, true);
        map.set_bool(HtmlAttr::Required, false);
        map.set(HtmlAttr::Title, AttrValue::None);

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(
            result.attrs,
            vec![(String::from("disabled"), String::new())]
        );
    }

    #[test]
    fn use_style_strategy_falls_back_to_inline_without_context() {
        let owner = Owner::new();

        owner.with(|| {
            assert_eq!(use_style_strategy(), StyleStrategy::Inline);
        });
    }

    #[test]
    fn use_style_strategy_reads_configured_context_value() {
        let owner = Owner::new();

        owner.with(|| {
            crate::provide_ars_context(crate::ArsContext::new(
                ars_i18n::locales::en_us(),
                ars_i18n::Direction::Ltr,
                ars_core::ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(ars_i18n::StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                StyleStrategy::Nonce(String::from("nonce-456")),
            ));

            assert_eq!(
                use_style_strategy(),
                StyleStrategy::Nonce(String::from("nonce-456"))
            );
        });
    }

    #[test]
    #[should_panic(expected = "element_id is required for Nonce style strategy")]
    fn nonce_strategy_requires_element_id() {
        let mut map = AttrMap::new();

        map.set_style(CssProperty::Display, "block");

        drop(attr_map_to_leptos(
            map,
            &StyleStrategy::Nonce(String::from("nonce")),
            None,
        ));
    }
}

#[cfg(all(test, not(feature = "ssr"), target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> leptos::web_sys::Document {
        leptos::web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
    }

    #[wasm_bindgen_test]
    fn apply_styles_cssom_sets_style_properties() {
        let element = document()
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into::<leptos::web_sys::HtmlElement>()
            .expect("element should cast to HtmlElement");

        let styles = vec![
            (CssProperty::Width, String::from("120px")),
            (CssProperty::Display, String::from("block")),
        ];

        apply_styles_cssom(&element, &styles);

        let style = element.style();

        assert_eq!(
            style
                .get_property_value("width")
                .expect("width should be readable"),
            "120px"
        );
        assert_eq!(
            style
                .get_property_value("display")
                .expect("display should be readable"),
            "block"
        );
    }
}
