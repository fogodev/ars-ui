//! AttrMap-to-Dioxus attribute conversion with attribute name interning.
//!
//! Bridges the framework-agnostic [`AttrMap`] from `ars-core` to Dioxus's
//! [`Attribute`] type, supporting all three [`StyleStrategy`] variants
//! (Inline, CSSOM, Nonce) for CSP-aware style rendering.

use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use ars_core::{AttrMap, AttrMapParts, AttrValue, CssProperty, HtmlAttr, StyleStrategy};
use dioxus::prelude::*;
use dioxus_core::AttributeValue;

use crate::provider::{ArsContext, warn_missing_provider};

// ── Attribute name interning ────────────────────────────────────────

/// Intern pool for attribute name strings.
///
/// Dioxus [`Attribute::new`] requires `&'static str`. Known HTML/ARIA attribute
/// names are compile-time constants via [`HtmlAttr::static_name()`]. Dynamic
/// `data-*` attribute names are interned on first use; the set is bounded by the
/// number of component parts (~500 across all 111 components), so total leaked
/// memory is negligible (~10 KB).
static ATTR_NAMES: LazyLock<Mutex<HashSet<&'static str>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Convert an [`HtmlAttr`] to a `&'static str` suitable for [`Attribute::new`].
///
/// Static variants (Class, Id, Role, etc.) return compile-time string slices.
/// [`HtmlAttr::Data`] variants intern `"data-{name}"` via a global pool.
pub fn intern_attr_name(attr: &HtmlAttr) -> &'static str {
    // Fast path: if HtmlAttr has a known static name, return it directly.
    if let Some(name) = attr.static_name() {
        return name;
    }

    // Slow path: Data(...) attributes — intern the formatted string.
    let name = attr.to_string(); // e.g., "data-ars-state"

    let mut pool = ATTR_NAMES.lock().expect("attr name pool");

    if let Some(&existing) = pool.get(name.as_str()) {
        return existing;
    }

    let leaked: &'static str = Box::leak(name.into_boxed_str());

    pool.insert(leaked);

    leaked
}

// ── Conversion result ───────────────────────────────────────────────

/// Result of converting an [`AttrMap`] with style strategy awareness.
#[derive(Debug)]
pub struct DioxusAttrResult {
    /// Dioxus dynamic attributes ready for spreading via `..attrs`.
    pub attrs: Vec<Attribute>,

    /// Styles to apply via CSSOM (`element.style().set_property()`).
    /// Non-empty only when strategy is [`StyleStrategy::Cssom`].
    pub cssom_styles: Vec<(CssProperty, String)>,

    /// CSS rule text to inject into a `<style nonce="...">` block.
    /// Non-empty only when strategy is [`StyleStrategy::Nonce`].
    pub nonce_css: String,
}

// ── Core conversion ─────────────────────────────────────────────────

/// Convert an [`AttrMap`] into Dioxus attributes using the given [`StyleStrategy`].
///
/// - `map.styles` are rendered according to the active strategy.
/// - `element_id` is required for [`StyleStrategy::Nonce`] (used as CSS selector).
/// - `class` and other space-separated attributes are already merged in the
///   [`AttrMap`] by `set()` and flow through the main attrs loop naturally.
pub fn attr_map_to_dioxus(
    map: AttrMap,
    strategy: &StyleStrategy,
    element_id: Option<&str>,
) -> DioxusAttrResult {
    let AttrMapParts { attrs, styles } = map.into_parts();

    let mut result = attrs
        .into_iter()
        .filter_map(|(key, val)| match val {
            AttrValue::String(s) => Some(Attribute::new(
                intern_attr_name(&key),
                AttributeValue::Text(s),
                None,
                false,
            )),

            AttrValue::Bool(true) => Some(Attribute::new(
                intern_attr_name(&key),
                AttributeValue::Text(String::new()),
                None,
                false,
            )),

            AttrValue::Bool(false) | AttrValue::None => None,
        })
        .collect::<Vec<_>>();

    let mut cssom_styles = Vec::new();
    let mut nonce_css = String::new();

    match strategy {
        StyleStrategy::Inline => {
            if !styles.is_empty() {
                let style_str = styles
                    .into_iter()
                    .map(|(prop, val)| format!("{prop}: {val};"))
                    .collect::<Vec<_>>()
                    .join(" ");

                result.push(Attribute::new(
                    "style",
                    AttributeValue::Text(style_str),
                    None,
                    false,
                ));
            }
        }

        StyleStrategy::Cssom => {
            cssom_styles = styles;
        }

        StyleStrategy::Nonce(_) => {
            if !styles.is_empty() {
                let id = element_id.expect("element_id is required for Nonce style strategy");

                result.push(Attribute::new(
                    "data-ars-style-id",
                    AttributeValue::Text(id.to_string()),
                    None,
                    false,
                ));

                nonce_css = styles_to_nonce_css(id, &styles);
            }
        }
    }

    DioxusAttrResult {
        attrs: result,
        cssom_styles,
        nonce_css,
    }
}

// ── CSSOM helper ────────────────────────────────────────────────────

/// Apply styles to a DOM element via the CSSOM API.
///
/// Used when [`StyleStrategy::Cssom`] is active. Iterates the style entries
/// and calls `element.style().setProperty()` for each one.
#[cfg(feature = "web")]
pub fn apply_styles_cssom(el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    let style = el.style();

    for (prop, val) in styles {
        if let Err(error) = style.set_property(&prop.to_string(), val) {
            #[cfg(debug_assertions)]
            web_sys::console::warn_1(&error);

            #[cfg(not(debug_assertions))]
            drop(error);
        }
    }
}

// ── Nonce CSS helpers ───────────────────────────────────────────────

/// Convert styles to a CSS rule string for nonce-based injection.
fn styles_to_nonce_css(id: &str, styles: &[(CssProperty, String)]) -> String {
    let decls = styles
        .iter()
        .map(|(prop, val)| format!("  {prop}: {val};"))
        .collect::<Vec<_>>();

    format!("[data-ars-style-id=\"{id}\"] {{\n{}\n}}", decls.join("\n"))
}

/// Context for collecting nonce CSS rules during rendering.
#[derive(Clone, Debug)]
pub struct ArsNonceCssCtx {
    /// Reactive signal holding accumulated CSS rule strings.
    pub rules: Signal<Vec<String>>,
}

/// Collects nonce CSS from descendant components and renders a
/// `<style nonce="...">` block.
///
/// Place this component near the document `<head>`:
/// ```rust,ignore
/// rsx! {
///     ArsProvider { style_strategy: StyleStrategy::Nonce(nonce.clone()),
///         ArsNonceStyle { nonce: nonce.clone() }
///         App {}
///     }
/// }
/// ```
#[component]
pub fn ArsNonceStyle(nonce: String) -> Element {
    let rules = use_signal(Vec::<String>::new);

    use_context_provider(|| ArsNonceCssCtx { rules });

    let css_text = use_memo(move || rules.read().join("\n"));

    rsx! {
        style { nonce, {css_text()} }
    }
}

/// Append a CSS rule to the nonce collector.
///
/// Called internally by components when [`StyleStrategy::Nonce`] is active.
/// Does nothing if no [`ArsNonceCssCtx`] is present in the context tree.
pub fn append_nonce_css(css: String) {
    if let Some(mut ctx) = try_use_context::<ArsNonceCssCtx>() {
        ctx.rules.write().push(css);
    }
}

// ── Style strategy hook ─────────────────────────────────────────────

/// Read the current style strategy from Dioxus context.
///
/// Returns [`StyleStrategy::Inline`] (the default) if no `ArsProvider` is
/// present in the component tree.
pub fn use_style_strategy() -> StyleStrategy {
    try_use_context::<ArsContext>().map_or_else(
        || {
            warn_missing_provider("use_style_strategy");
            StyleStrategy::default()
        },
        |ctx| ctx.style_strategy().clone(),
    )
}

// ── Macro ───────────────────────────────────────────────────────────

/// Convenience macro for converting an [`AttrMap`] to Dioxus attributes.
///
/// Equivalent to calling [`attr_map_to_dioxus`] directly.
///
/// # Usage
///
/// ```rust,ignore
/// let attrs = api.root_attrs();
///
/// let strategy = use_style_strategy();
///
/// rsx! {
///     div { ..attr_map_to_dioxus(attrs, &strategy, Some("my-el")).attrs, {children} }
/// }
/// ```
#[macro_export]
macro_rules! dioxus_attrs {
    ($map:expr, $strategy:expr, $id:expr) => {
        $crate::attr_map_to_dioxus($map, $strategy, $id)
    };
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, HtmlAttr};

    use super::*;

    // ── intern_attr_name ────────────────────────────────────────────

    #[test]
    fn intern_attr_name_fast_path_returns_static_slices() {
        assert_eq!(intern_attr_name(&HtmlAttr::Class), "class");
        assert_eq!(intern_attr_name(&HtmlAttr::Id), "id");
        assert_eq!(
            intern_attr_name(&HtmlAttr::Aria(AriaAttr::Label)),
            "aria-label"
        );
        assert_eq!(intern_attr_name(&HtmlAttr::TabIndex), "tabindex");
        assert_eq!(intern_attr_name(&HtmlAttr::Role), "role");
        assert_eq!(intern_attr_name(&HtmlAttr::Disabled), "disabled");
    }

    #[test]
    fn intern_attr_name_slow_path_interns_data_attributes() {
        let first = intern_attr_name(&HtmlAttr::Data("ars-state"));
        let second = intern_attr_name(&HtmlAttr::Data("ars-state"));

        assert_eq!(first, "data-ars-state");
        assert_eq!(second, "data-ars-state");
        // Same pointer — proves the pool returns the existing leaked reference.
        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
    }

    #[test]
    fn intern_attr_name_data_distinct_values_differ() {
        let foo = intern_attr_name(&HtmlAttr::Data("foo"));

        let bar = intern_attr_name(&HtmlAttr::Data("bar"));

        assert_eq!(foo, "data-foo");
        assert_eq!(bar, "data-bar");
        assert_ne!(foo, bar);
    }

    // ── attr_map_to_dioxus ──────────────────────────────────────────

    /// Helper: find an attribute by name in the result.
    fn find_attr<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
        attrs.iter().find(|a| a.name == name)
    }

    /// Helper: extract text value from an Attribute.
    fn text_value(attr: &Attribute) -> Option<&str> {
        match &attr.value {
            AttributeValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn build_test_map() -> AttrMap {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "btn")
            .set_style(CssProperty::Width, "100px");

        map
    }

    #[test]
    fn attr_map_to_dioxus_inline_strategy_renders_style_attribute() {
        let result = attr_map_to_dioxus(build_test_map(), &StyleStrategy::Inline, None);

        let class_attr = find_attr(&result.attrs, "class").expect("class attr present");

        assert_eq!(text_value(class_attr), Some("btn"));

        let style_attr = find_attr(&result.attrs, "style").expect("style attr present");

        assert_eq!(text_value(style_attr), Some("width: 100px;"));

        assert!(result.cssom_styles.is_empty());
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn attr_map_to_dioxus_cssom_strategy_returns_styles_separately() {
        let result = attr_map_to_dioxus(build_test_map(), &StyleStrategy::Cssom, None);

        // class attr present, no style attr
        assert!(find_attr(&result.attrs, "class").is_some());
        assert!(find_attr(&result.attrs, "style").is_none());

        assert_eq!(result.cssom_styles.len(), 1);
        assert_eq!(result.cssom_styles[0].0, CssProperty::Width);
        assert_eq!(result.cssom_styles[0].1, "100px");
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn attr_map_to_dioxus_nonce_strategy_generates_css_rule() {
        let result = attr_map_to_dioxus(
            build_test_map(),
            &StyleStrategy::Nonce("n123".into()),
            Some("el-1"),
        );

        // class attr present
        assert!(find_attr(&result.attrs, "class").is_some());

        // data-ars-style-id injected
        let id_attr =
            find_attr(&result.attrs, "data-ars-style-id").expect("data-ars-style-id present");

        assert_eq!(text_value(id_attr), Some("el-1"));

        // nonce CSS rule generated
        assert!(result.nonce_css.contains("[data-ars-style-id=\"el-1\"]"));
        assert!(result.nonce_css.contains("width: 100px;"));
        assert!(result.cssom_styles.is_empty());
    }

    #[test]
    fn attr_map_to_dioxus_value_mapping_filters_false_and_none() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Id, "x")
            .set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Hidden, false);

        // AttrValue::None via set then remove pattern: set Inert then override
        // We can test None by checking that Bool(false) is filtered.

        let result = attr_map_to_dioxus(map, &StyleStrategy::Inline, None);

        let id_attr = find_attr(&result.attrs, "id").expect("id attr present");

        assert_eq!(text_value(id_attr), Some("x"));

        let disabled_attr = find_attr(&result.attrs, "disabled").expect("disabled attr present");

        assert_eq!(text_value(disabled_attr), Some(""));

        // Bool(false) attributes are filtered out
        assert!(find_attr(&result.attrs, "hidden").is_none());

        // Only id and disabled
        assert_eq!(result.attrs.len(), 2);
    }

    #[test]
    fn attr_map_to_dioxus_empty_styles_omit_style_attribute() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "btn");

        // No styles

        let result = attr_map_to_dioxus(map, &StyleStrategy::Inline, None);
        assert!(find_attr(&result.attrs, "style").is_none());
    }

    // ── styles_to_nonce_css ─────────────────────────────────────────

    #[test]
    fn styles_to_nonce_css_formats_selector_and_declarations() {
        let styles = vec![
            (CssProperty::Width, "100px".to_string()),
            (CssProperty::Color, "red".to_string()),
        ];

        let css = styles_to_nonce_css("el-1", &styles);

        let expected = "[data-ars-style-id=\"el-1\"] {\n  width: 100px;\n  color: red;\n}";

        assert_eq!(css, expected);
    }

    // ── dioxus_attrs! macro ─────────────────────────────────────────

    #[test]
    fn dioxus_attrs_macro_delegates_to_function() {
        let map = build_test_map();

        let result = dioxus_attrs!(map, &StyleStrategy::Inline, None);

        // Verify it produces a DioxusAttrResult with expected content
        assert!(find_attr(&result.attrs, "class").is_some());
        assert!(find_attr(&result.attrs, "style").is_some());
    }

    #[test]
    fn attr_map_to_dioxus_nonce_strategy_empty_styles_is_noop() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "btn");

        // No styles — Nonce should not inject data-ars-style-id or generate CSS.

        let result = attr_map_to_dioxus(map, &StyleStrategy::Nonce("n456".into()), Some("el-2"));

        assert!(find_attr(&result.attrs, "class").is_some());
        assert!(find_attr(&result.attrs, "data-ars-style-id").is_none());
        assert!(result.nonce_css.is_empty());
        assert!(result.cssom_styles.is_empty());
    }

    #[test]
    fn attr_map_to_dioxus_attr_value_none_is_filtered() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Id, "x");

        // Setting then removing produces AttrValue::None internally via set(_, AttrValue::None).
        map.set(HtmlAttr::Title, AttrValue::None);

        let result = attr_map_to_dioxus(map, &StyleStrategy::Inline, None);

        assert!(find_attr(&result.attrs, "id").is_some());
        assert!(find_attr(&result.attrs, "title").is_none());
        assert_eq!(result.attrs.len(), 1);
    }

    // ── ArsNonceStyle + append_nonce_css ────────────────────────────

    #[test]
    fn ars_nonce_style_mounts_and_provides_context() {
        fn app() -> Element {
            rsx! {
                ArsNonceStyle { nonce: "test-nonce".to_string() }
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn append_nonce_css_collects_rules_from_context() {
        fn app() -> Element {
            // Provide the context directly (same as ArsNonceStyle does internally).
            let rules = use_signal(Vec::<String>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

            append_nonce_css(".rule-1 { color: red; }".into());
            append_nonce_css(".rule-2 { color: blue; }".into());

            let collected = rules.peek().clone();
            assert_eq!(collected.len(), 2);
            assert_eq!(collected[0], ".rule-1 { color: red; }");
            assert_eq!(collected[1], ".rule-2 { color: blue; }");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn append_nonce_css_is_noop_without_context() {
        fn app() -> Element {
            // No ArsNonceCssCtx provided — should silently do nothing.
            append_nonce_css("orphan rule".into());

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    // ── use_style_strategy ──────────────────────────────────────────

    #[test]
    fn use_style_strategy_defaults_without_provider() {
        fn app() -> Element {
            let strategy = use_style_strategy();

            assert_eq!(strategy, StyleStrategy::Inline);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_style_strategy_reads_context() {
        fn app() -> Element {
            use std::sync::Arc;

            use ars_core::{ColorMode, I18nRegistries, NullPlatformEffects};
            use ars_i18n::{Direction, locales};

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(ars_core::DefaultModalityContext::new()),
                Arc::new(ars_i18n::StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::new(crate::provider::NullPlatform),
                StyleStrategy::Cssom,
            );

            use_context_provider(|| ctx);

            let strategy = use_style_strategy();

            assert_eq!(strategy, StyleStrategy::Cssom);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::sync::Arc;

    use ars_core::{
        AriaAttr, ColorMode, CssProperty, HtmlAttr, I18nRegistries, NullPlatformEffects,
    };
    use ars_i18n::{Direction, locales};
    use dioxus::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
    }

    fn find_attr<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
        attrs.iter().find(|attr| attr.name == name)
    }

    const fn text_value(attr: &Attribute) -> Option<&str> {
        match &attr.value {
            AttributeValue::Text(text) => Some(text.as_str()),
            _ => None,
        }
    }

    #[wasm_bindgen_test]
    fn intern_attr_name_and_attr_map_to_dioxus_cover_core_conversion_paths_on_wasm() {
        assert_eq!(intern_attr_name(&HtmlAttr::Class), "class");
        assert_eq!(
            intern_attr_name(&HtmlAttr::Data("ars-state")),
            "data-ars-state"
        );
        assert_eq!(
            intern_attr_name(&HtmlAttr::Aria(AriaAttr::Label)),
            "aria-label"
        );

        let mut map = AttrMap::new();
        map.set(HtmlAttr::Class, "btn")
            .set(HtmlAttr::Title, AttrValue::None)
            .set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Hidden, false)
            .set_style(CssProperty::Width, "100px");

        let inline = attr_map_to_dioxus(map.clone(), &StyleStrategy::Inline, None);

        let cssom = attr_map_to_dioxus(map.clone(), &StyleStrategy::Cssom, None);

        let nonce = attr_map_to_dioxus(
            map,
            &StyleStrategy::Nonce(String::from("n123")),
            Some("el-1"),
        );

        assert_eq!(
            text_value(find_attr(&inline.attrs, "class").expect("class attr present")),
            Some("btn")
        );
        assert_eq!(
            text_value(find_attr(&inline.attrs, "style").expect("style attr present")),
            Some("width: 100px;")
        );
        assert_eq!(
            text_value(find_attr(&inline.attrs, "disabled").expect("disabled attr present")),
            Some("")
        );
        assert!(find_attr(&inline.attrs, "hidden").is_none());
        assert!(find_attr(&inline.attrs, "title").is_none());

        assert!(find_attr(&cssom.attrs, "style").is_none());
        assert_eq!(
            cssom.cssom_styles,
            vec![(CssProperty::Width, String::from("100px"))]
        );
        assert_eq!(
            text_value(
                find_attr(&nonce.attrs, "data-ars-style-id")
                    .expect("nonce style id attr should exist"),
            ),
            Some("el-1")
        );
        assert_eq!(
            nonce.nonce_css,
            "[data-ars-style-id=\"el-1\"] {\n  width: 100px;\n}"
        );
    }

    #[wasm_bindgen_test]
    fn nonce_css_helpers_and_style_strategy_context_work_on_wasm() {
        fn app() -> Element {
            let rules = use_signal(Vec::<String>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

            append_nonce_css(".rule-1 { color: red; }".into());
            append_nonce_css(".rule-2 { color: blue; }".into());

            let collected = rules.peek().clone();

            assert_eq!(collected.len(), 2);
            assert_eq!(collected[0], ".rule-1 { color: red; }");
            assert_eq!(collected[1], ".rule-2 { color: blue; }");

            let ctx = ArsContext::new(
                locales::en_us(),
                Direction::Ltr,
                ColorMode::System,
                false,
                false,
                None,
                None,
                None,
                Arc::new(NullPlatformEffects),
                Arc::new(ars_core::DefaultModalityContext::new()),
                Arc::new(ars_i18n::StubIntlBackend),
                Arc::new(I18nRegistries::new()),
                Arc::new(crate::provider::NullPlatform),
                StyleStrategy::Cssom,
            );

            use_context_provider(|| ctx);

            assert_eq!(use_style_strategy(), StyleStrategy::Cssom);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test]
    fn apply_styles_cssom_sets_style_properties() {
        let element = document()
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into::<web_sys::HtmlElement>()
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
