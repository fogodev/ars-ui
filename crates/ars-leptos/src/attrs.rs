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

    // SSR / static-string path: materialize reactive variants to a
    // one-shot value so the rendered output snapshots the current closure
    // result. Reactive subscribers don't survive past serialization on
    // this path; the inline-attrs entrypoint below preserves reactivity
    // when the attrs are spread into a live `view!`.
    let mut attrs = parts
        .attrs
        .into_iter()
        .filter_map(|(key, value)| match value {
            AttrValue::String(text) => Some((key.to_string(), text)),
            AttrValue::Bool(true) => Some((key.to_string(), String::new())),
            AttrValue::Reactive(f) => Some((key.to_string(), f())),
            // Reactive booleans materialize to HTML presence semantics
            // matching the static [`AttrValue::Bool`] path: `true` →
            // empty value (attribute present), `false` → skip the
            // attribute entirely.
            AttrValue::ReactiveBool(f) => f().then_some((key.to_string(), String::new())),
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

/// Convenience wrapper around [`attr_map_to_leptos`] for callers that
/// always render with [`StyleStrategy::Inline`] and only need a list of
/// [`leptos::tachys::html::attribute::any_attribute::AnyAttribute`]
/// values ready for spreading via `{..attrs}` in `view!`.
///
/// Each `(name, value)` pair returned by [`attr_map_to_leptos`] is wrapped
/// through [`leptos::attr::custom::custom_attribute`] +
/// [`leptos::tachys::html::attribute::any_attribute::IntoAnyAttribute::into_any_attr`]
/// so the result is the same shape consumers spread with `{..}` in
/// `view!`.
///
/// Use the full [`attr_map_to_leptos`] when [`StyleStrategy::Cssom`] or
/// [`StyleStrategy::Nonce`] is in play and the caller needs to apply
/// `cssom_styles` to the DOM or inject `nonce_css` into a `<style>` block.
#[must_use]
pub fn attr_map_to_leptos_inline_attrs(
    map: AttrMap,
) -> Vec<leptos::tachys::html::attribute::any_attribute::AnyAttribute> {
    use leptos::tachys::html::attribute::any_attribute::IntoAnyAttribute;

    let parts = map.into_parts();

    let mut out: Vec<leptos::tachys::html::attribute::any_attribute::AnyAttribute> = parts
        .attrs
        .into_iter()
        .filter_map(|(key, value)| {
            let name = key.to_string();
            match value {
                AttrValue::String(text) => {
                    Some(leptos::attr::custom::custom_attribute(name, text).into_any_attr())
                }
                AttrValue::Bool(true) => Some(
                    leptos::attr::custom::custom_attribute(name, String::new()).into_any_attr(),
                ),
                AttrValue::Reactive(f) => {
                    // tachys's `AttributeValue for F where F: ReactiveFunction`
                    // wraps the closure in a `RenderEffect`, so the rendered
                    // attribute updates whenever the signals read inside the
                    // closure change.
                    let closure = move || f();
                    Some(leptos::attr::custom::custom_attribute(name, closure).into_any_attr())
                }
                AttrValue::ReactiveBool(f) => {
                    // Reactive booleans follow the HTML presence/absence
                    // semantics symmetric with the static [`AttrValue::Bool`]
                    // path: `true` renders the attribute with an empty
                    // value, `false` removes it from the rendered DOM.
                    // tachys's `AttributeValue for Option<V>` skips the
                    // attribute when the closure returns `None`, so the
                    // reactive output toggles presence as the underlying
                    // signal changes. Consumers that need ARIA-style
                    // `"true"` / `"false"` literal values (`aria-busy`,
                    // `aria-disabled`, `aria-expanded`, etc.) should use
                    // [`AttrValue::reactive`] with a closure that returns
                    // the literal string.
                    let closure = move || f().then(String::new);
                    Some(leptos::attr::custom::custom_attribute(name, closure).into_any_attr())
                }
                AttrValue::Bool(false) | AttrValue::None => None,
            }
        })
        .collect();

    // Keep parity with the SSR/static path: when there are styles, fold them
    // into a single `style=""` inline attribute so the inline strategy
    // matches what `attr_map_to_leptos` would have produced.
    if !parts.styles.is_empty() {
        let style = parts
            .styles
            .into_iter()
            .map(|(property, value)| format!("{property}: {value};"))
            .collect::<Vec<_>>()
            .join(" ");

        out.push(
            leptos::attr::custom::custom_attribute(String::from("style"), style).into_any_attr(),
        );
    }

    out
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

        map.set(HtmlAttr::Id, "button-id")
            .set_style(CssProperty::Display, "inline-flex")
            .set_style(CssProperty::Width, "10px");

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

        map.set(HtmlAttr::Title, "tooltip")
            .set_style(CssProperty::Display, "grid")
            .set_style(CssProperty::Width, "20px");

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

        map.set(HtmlAttr::Class, "root")
            .set_style(CssProperty::Display, "flex");

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

        map.set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Required, false)
            .set(HtmlAttr::Title, AttrValue::None);

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
                Arc::new(ars_core::DefaultModalityContext::new()),
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
    use std::sync::Arc;

    use ars_core::{
        AttrMap, AttrValue, CssProperty, HtmlAttr, I18nRegistries, NullPlatformEffects,
    };
    use leptos::prelude::Owner;
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
    fn attr_map_to_leptos_covers_inline_cssom_and_nonce_strategies_on_wasm() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "btn")
            .set_style(CssProperty::Width, "100px");

        let inline = attr_map_to_leptos(map.clone(), &StyleStrategy::Inline, None);

        let cssom = attr_map_to_leptos(map.clone(), &StyleStrategy::Cssom, None);

        let nonce = attr_map_to_leptos(
            map,
            &StyleStrategy::Nonce(String::from("nonce-123")),
            Some("el-1"),
        );

        assert_eq!(
            inline.attrs,
            vec![
                (String::from("class"), String::from("btn")),
                (String::from("style"), String::from("width: 100px;"))
            ]
        );
        assert_eq!(
            cssom.cssom_styles,
            vec![(CssProperty::Width, String::from("100px"))]
        );
        assert_eq!(
            nonce.attrs,
            vec![
                (String::from("class"), String::from("btn")),
                (String::from("data-ars-style-id"), String::from("el-1"))
            ]
        );
        assert_eq!(
            nonce.nonce_css,
            "[data-ars-style-id=\"el-1\"] {\n  width: 100px;\n}"
        );
    }

    #[wasm_bindgen_test]
    fn attr_map_to_leptos_filters_false_and_none_values_on_wasm() {
        let mut map = AttrMap::new();

        map.set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Required, false)
            .set(HtmlAttr::Title, AttrValue::None);

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(
            result.attrs,
            vec![(String::from("disabled"), String::new())]
        );
    }

    #[wasm_bindgen_test]
    fn attr_map_to_leptos_inline_attrs_returns_one_any_attr_per_underlying_pair_on_wasm() {
        // Two attrs (`class`, `style`) folded by the Inline strategy.
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "btn")
            .set_style(CssProperty::Width, "100px");

        let attrs = attr_map_to_leptos_inline_attrs(map.clone());

        // Same length the underlying helper produces — guarantees the
        // wrapper folds styles (Inline) and doesn't accidentally drop or
        // duplicate entries while mapping into `AnyAttribute`.
        let baseline = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(
            attrs.len(),
            baseline.attrs.len(),
            "wrapper must yield one AnyAttribute per (name, value) pair the Inline strategy produces",
        );

        assert_eq!(attrs.len(), 2);
    }

    #[wasm_bindgen_test]
    fn attr_map_to_leptos_inline_attrs_drops_false_and_none_entries_on_wasm() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "kept")
            .set_bool(HtmlAttr::Disabled, false)
            .set(HtmlAttr::Title, AttrValue::None);

        let attrs = attr_map_to_leptos_inline_attrs(map);

        assert_eq!(
            attrs.len(),
            1,
            "Bool(false) and AttrValue::None must not flow through the wrapper",
        );
    }

    #[wasm_bindgen_test]
    fn use_style_strategy_reads_configured_context_on_wasm() {
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
                Arc::new(ars_core::DefaultModalityContext::new()),
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
