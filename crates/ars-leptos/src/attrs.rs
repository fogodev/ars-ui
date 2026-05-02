//! Strategy-aware conversion from [`ars_core::AttrMap`] to Leptos attributes.
//!
//! This module bridges the framework-agnostic connect output used across `ars-ui`
//! to the native Leptos attribute representation spread onto elements.

use ars_core::{AttrMap, AttrValue, CssProperty, StyleStrategy, styles_to_nonce_css};
#[cfg(not(feature = "ssr"))]
use leptos::{
    prelude::{Get, GetValue, SetValue, UpdateValue},
    wasm_bindgen::JsCast,
    web_sys,
};

use crate::provider::{current_ars_context, warn_missing_provider};

/// Type-erased Leptos attribute ready to spread with `{..attrs}` in `view!`.
pub type LeptosAttribute = leptos::tachys::html::attribute::any_attribute::AnyAttribute;

/// Result of converting an [`AttrMap`] into Leptos-ready attributes.
#[derive(Clone, Debug)]
pub struct LeptosAttrResult {
    /// Native Leptos attributes ready to spread onto a Leptos element.
    pub attrs: Vec<LeptosAttribute>,

    /// CSS properties to apply via CSSOM when [`StyleStrategy::Cssom`] is active.
    pub cssom_styles: Vec<(CssProperty, String)>,

    /// Stable key for [`Self::nonce_css`] when [`StyleStrategy::Nonce`] is active.
    pub nonce_css_key: Option<String>,

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
        .filter_map(|(key, value)| attr_value_to_leptos_attr(key.to_string(), value))
        .collect::<Vec<_>>();

    let mut cssom_styles = Vec::new();

    let mut nonce_css_key = None;

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

                attrs.push(string_attr(String::from("style"), style));
            }
        }

        StyleStrategy::Cssom => {
            cssom_styles = parts.styles;
        }

        StyleStrategy::Nonce(_) => {
            if !parts.styles.is_empty() {
                let id = element_id.expect("element_id is required for Nonce style strategy");

                attrs.push(string_attr(
                    String::from("data-ars-style-id"),
                    String::from(id),
                ));

                nonce_css_key = Some(String::from(id));

                nonce_css = styles_to_nonce_css(id, &parts.styles);
            }
        }
    }

    LeptosAttrResult {
        attrs,
        cssom_styles,
        nonce_css_key,
        nonce_css,
    }
}

/// Convenience wrapper around [`attr_map_to_leptos`] for callers that always render
/// with [`StyleStrategy::Inline`] and only need the native Leptos attributes.
///
/// Prefer [`attr_map_to_leptos`] when [`StyleStrategy::Cssom`] or
/// [`StyleStrategy::Nonce`] is in play and the caller needs to apply `cssom_styles`
/// to the DOM or inject `nonce_css` into a `<style>` block.
#[must_use]
pub fn attr_map_to_leptos_inline_attrs(map: AttrMap) -> Vec<LeptosAttribute> {
    attr_map_to_leptos(map, &StyleStrategy::Inline, None).attrs
}

fn attr_value_to_leptos_attr(name: String, value: AttrValue) -> Option<LeptosAttribute> {
    use leptos::tachys::html::attribute::any_attribute::IntoAnyAttribute as _;

    match value {
        AttrValue::String(text) => {
            Some(leptos::attr::custom::custom_attribute(name, text).into_any_attr())
        }

        AttrValue::Bool(true) => Some(string_attr(name, String::new())),

        AttrValue::Reactive(f) => {
            // tachys's `AttributeValue for F where F: ReactiveFunction`
            // wraps the closure in a `RenderEffect`, so the rendered
            // attribute updates whenever the signals read inside the
            // closure change.
            let closure = move || f();

            Some(leptos::attr::custom::custom_attribute(name, closure).into_any_attr())
        }

        AttrValue::ReactiveBool(f) => {
            // Reactive booleans follow HTML presence/absence semantics. ARIA
            // boolean states use the literal `"true"` token instead of an empty
            // string because assistive technology consumes the attribute value.
            let is_aria = name.starts_with("aria-");
            let closure = move || {
                f().then(|| {
                    if is_aria {
                        String::from("true")
                    } else {
                        String::new()
                    }
                })
            };

            Some(leptos::attr::custom::custom_attribute(name, closure).into_any_attr())
        }

        AttrValue::Bool(false) | AttrValue::None => None,
    }
}

/// Builds one literal custom Leptos attribute.
pub(crate) fn string_attr(name: String, value: String) -> LeptosAttribute {
    use leptos::tachys::html::attribute::any_attribute::IntoAnyAttribute as _;

    leptos::attr::custom::custom_attribute(name, value).into_any_attr()
}

/// Tracks CSS properties applied through CSSOM so later syncs can remove stale entries.
#[cfg(not(feature = "ssr"))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssomStyleHandle {
    /// CSS properties applied during the previous sync.
    applied: Vec<CssProperty>,
}

#[cfg(not(feature = "ssr"))]
impl CssomStyleHandle {
    /// Creates an empty CSSOM style synchronization handle.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            applied: Vec::new(),
        }
    }

    /// Applies `styles` and removes properties that were applied by the previous sync.
    pub fn sync(&mut self, el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
        let style = el.style();

        for property in self.applied.drain(..) {
            if !styles
                .iter()
                .any(|(next_property, _)| *next_property == property)
            {
                drop(style.remove_property(&property.to_string()));
            }
        }

        for (property, value) in styles {
            drop(style.set_property(&property.to_string(), value));

            self.applied.push(property.clone());
        }
    }

    /// Removes every property currently owned by this handle.
    pub fn clear(&mut self, el: &web_sys::HtmlElement) {
        let style = el.style();

        for property in self.applied.drain(..) {
            drop(style.remove_property(&property.to_string()));
        }
    }
}

/// Applies CSS properties directly to an element via CSSOM.
///
/// Prefer [`CssomStyleHandle`] when styles can change over time, so stale
/// properties are removed on later renders.
#[cfg(not(feature = "ssr"))]
pub fn apply_styles_cssom(el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    CssomStyleHandle::new().sync(el, styles);
}

/// Synchronizes CSSOM styles from an attribute conversion result to a node ref.
///
/// The hook owns a persistent [`CssomStyleHandle`], so styles removed from later
/// syncs are also removed from the DOM element. Cleanup clears every property
/// owned by the handle. Use [`use_cssom_styles`] when the style list is reactive.
#[cfg(not(feature = "ssr"))]
pub fn use_cssom_styles_from_attrs<E>(
    target: leptos::prelude::NodeRef<E>,
    result: &LeptosAttrResult,
) where
    E: leptos::tachys::html::element::ElementType,
    E::Output: JsCast + Clone + 'static,
{
    let styles = result.cssom_styles.clone();

    use_cssom_styles(target, move || styles.clone());
}

/// Synchronizes reactive CSSOM styles to a node ref.
///
/// The `styles` closure runs inside a Leptos effect. Signal reads inside it
/// resubscribe the hook so changed styles are applied, stale properties are
/// removed, and styles are cleared from the previous target when the node ref
/// points at a different element.
#[cfg(not(feature = "ssr"))]
pub fn use_cssom_styles<E, F>(target: leptos::prelude::NodeRef<E>, styles: F)
where
    E: leptos::tachys::html::element::ElementType,
    E::Output: JsCast + Clone + 'static,
    F: Fn() -> Vec<(CssProperty, String)> + 'static,
{
    let handle = leptos::prelude::StoredValue::new_local(CssomStyleHandle::new());
    let applied_element = leptos::prelude::StoredValue::new_local(None);

    leptos::prelude::Effect::new(move |_| {
        let styles = styles();

        let element = target.get().map(JsCast::unchecked_into);

        if let Some(previous) = applied_element.get_value() {
            handle.update_value(|handle| handle.clear(&previous));
        }

        let Some(element) = element else {
            applied_element.set_value(None);

            return;
        };

        handle.update_value(|handle| handle.sync(&element, &styles));

        applied_element.set_value(Some(element));
    });

    leptos::prelude::on_cleanup(move || {
        if let Some(element) = applied_element.get_value() {
            handle.update_value(|handle| handle.clear(&element));
        }

        applied_element.set_value(None);
    });
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

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use ars_core::{AttrMap, CssProperty, HtmlAttr, I18nRegistries, NullPlatformEffects};
    use leptos::reactive::owner::Owner;

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
        assert_eq!(result.attrs.len(), 2);
    }

    #[test]
    fn inline_strategy_skips_style_attribute_when_no_styles_exist() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Id, "button-id");

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(result.attrs.len(), 1);
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

        assert_eq!(result.attrs.len(), 1);
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

        assert_eq!(result.attrs.len(), 2);
        assert!(result.cssom_styles.is_empty());
        assert_eq!(result.nonce_css_key, Some(String::from("checkbox-root")));
        assert_eq!(
            result.nonce_css,
            "[data-ars-style-id=\"checkbox-root\"] {\n  display: flex;\n}"
        );
    }

    #[test]
    fn nonce_strategy_escapes_selector_attribute_value() {
        let mut map = AttrMap::new();

        map.set_style(CssProperty::Width, "10px");

        let result = attr_map_to_leptos(
            map,
            &StyleStrategy::Nonce(String::from("nonce-123")),
            Some("root\"quoted\\path]"),
        );

        assert_eq!(
            result.nonce_css,
            "[data-ars-style-id=\"root\\\"quoted\\\\path]\"] {\n  width: 10px;\n}"
        );
    }

    #[test]
    fn nonce_strategy_escapes_control_characters_in_selector_attribute_value() {
        for (raw, escaped) in [
            ("root\nline", "root\\A line"),
            ("root\rline", "root\\D line"),
            ("root\tline", "root\\9 line"),
            ("root\0line", "root\\FFFD line"),
        ] {
            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "10px");

            let result = attr_map_to_leptos(
                map,
                &StyleStrategy::Nonce(String::from("nonce-123")),
                Some(raw),
            );

            assert_eq!(
                result.nonce_css,
                format!("[data-ars-style-id=\"{escaped}\"] {{\n  width: 10px;\n}}")
            );
        }
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

        assert_eq!(result.attrs.len(), 1);
        assert!(result.cssom_styles.is_empty());
        assert_eq!(result.nonce_css_key, None);
        assert!(result.nonce_css.is_empty());
    }

    #[test]
    fn inline_strategy_preserves_reactive_string_without_eager_materialization() {
        let mut map = AttrMap::new();

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_attr = Arc::clone(&calls);

        map.set(
            HtmlAttr::Aria(ars_core::AriaAttr::Label),
            AttrValue::reactive(move || {
                calls_for_attr.fetch_add(1, Ordering::Relaxed);
                String::from("Schließen")
            }),
        );

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(result.attrs.len(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn inline_strategy_preserves_reactive_bool_without_eager_materialization() {
        let mut map = AttrMap::new();

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_attr = Arc::clone(&calls);

        map.set(
            HtmlAttr::Disabled,
            AttrValue::reactive_bool(move || {
                calls_for_attr.fetch_add(1, Ordering::Relaxed);
                true
            }),
        );

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(result.attrs.len(), 1);
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn bool_false_and_none_are_filtered_while_bool_true_is_empty_string() {
        let mut map = AttrMap::new();

        map.set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Required, false)
            .set(HtmlAttr::Title, AttrValue::None);

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(result.attrs.len(), 1);
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
    #[cfg(target_arch = "wasm32")]
    use leptos::prelude::NodeRefAttribute;
    use leptos::{
        either::Either,
        prelude::{AddAnyAttr, Get, GlobalAttributes, Owner, Set},
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
    }

    #[wasm_bindgen_test]
    async fn attr_map_to_leptos_covers_inline_cssom_and_nonce_strategies_on_wasm() {
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

        assert_eq!(inline.attrs.len(), 2);
        assert_eq!(
            cssom.cssom_styles,
            vec![(CssProperty::Width, String::from("100px"))]
        );
        assert_eq!(nonce.attrs.len(), 2);
        assert_eq!(
            nonce.nonce_css,
            "[data-ars-style-id=\"el-1\"] {\n  width: 100px;\n}"
        );

        let owner = Owner::new();
        let (mount_handle, inline_element, nonce_element) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let inline_attrs = inline.attrs;
            let nonce_attrs = nonce.attrs;

            let mount_handle = leptos::mount::mount_to(parent.clone(), move || {
                leptos::view! {
                    <div id="inline-target" {..inline_attrs}></div>
                    <div id="nonce-target" {..nonce_attrs}></div>
                }
            });

            let inline_element = parent
                .query_selector("#inline-target")
                .expect("query should succeed")
                .expect("inline target should exist")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("inline target should be an HtmlElement");

            let nonce_element = parent
                .query_selector("#nonce-target")
                .expect("query should succeed")
                .expect("nonce target should exist")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("nonce target should be an HtmlElement");

            (mount_handle, inline_element, nonce_element)
        });

        leptos::task::tick().await;

        assert_eq!(
            inline_element.get_attribute("class").as_deref(),
            Some("btn")
        );
        assert_eq!(
            inline_element
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            "100px"
        );
        assert_eq!(nonce_element.get_attribute("class").as_deref(), Some("btn"));
        assert_eq!(
            nonce_element.get_attribute("data-ars-style-id").as_deref(),
            Some("el-1")
        );

        drop(mount_handle);
    }

    #[wasm_bindgen_test]
    fn attr_map_to_leptos_filters_false_and_none_values_on_wasm() {
        let mut map = AttrMap::new();

        map.set_bool(HtmlAttr::Disabled, true)
            .set_bool(HtmlAttr::Required, false)
            .set(HtmlAttr::Title, AttrValue::None);

        let result = attr_map_to_leptos(map, &StyleStrategy::Inline, None);

        assert_eq!(result.attrs.len(), 1);
    }

    #[wasm_bindgen_test]
    async fn attr_map_to_leptos_preserves_reactive_attrs_on_wasm() {
        let owner = Owner::new();

        let (mount_handle, button, set_label, set_disabled) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let (label, set_label) = leptos::prelude::signal(String::from("first"));
            let (disabled, set_disabled) = leptos::prelude::signal(false);

            let mut map = AttrMap::new();

            map.set(
                HtmlAttr::Aria(ars_core::AriaAttr::Label),
                AttrValue::reactive(move || label.get()),
            )
            .set(
                HtmlAttr::Disabled,
                AttrValue::reactive_bool(move || disabled.get()),
            );

            let attrs = attr_map_to_leptos(map, &StyleStrategy::Inline, None).attrs;

            let mount_handle = leptos::mount::mount_to(parent.clone(), move || {
                leptos::view! { <button id="reactive-attrs" {..attrs}></button> }
            });

            let button = parent
                .query_selector("#reactive-attrs")
                .expect("query should succeed")
                .expect("button should exist")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("button should be an HtmlElement");

            (mount_handle, button, set_label, set_disabled)
        });

        leptos::task::tick().await;

        assert_eq!(button.get_attribute("aria-label").as_deref(), Some("first"));
        assert_eq!(button.get_attribute("disabled"), None);

        set_label.set(String::from("second"));
        set_disabled.set(true);

        leptos::task::tick().await;

        assert_eq!(
            button.get_attribute("aria-label").as_deref(),
            Some("second")
        );
        assert_eq!(button.get_attribute("disabled").as_deref(), Some(""));

        drop(mount_handle);
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

    #[wasm_bindgen_test]
    fn cssom_style_handle_removes_stale_properties() {
        let element = document()
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("element should cast to HtmlElement");

        let mut handle = CssomStyleHandle::new();

        handle.sync(
            &element,
            &[
                (CssProperty::Width, String::from("120px")),
                (CssProperty::Display, String::from("block")),
            ],
        );
        handle.sync(&element, &[(CssProperty::Display, String::from("grid"))]);

        let style = element.style();

        assert_eq!(
            style
                .get_property_value("width")
                .expect("width should be readable"),
            ""
        );
        assert_eq!(
            style
                .get_property_value("display")
                .expect("display should be readable"),
            "grid"
        );

        handle.clear(&element);

        assert_eq!(
            style
                .get_property_value("display")
                .expect("display should be readable"),
            ""
        );
    }

    #[wasm_bindgen_test]
    async fn use_cssom_styles_from_attrs_syncs_and_clears_on_cleanup() {
        let owner = Owner::new();

        let (mount_handle, target) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "120px")
                .set_style(CssProperty::Display, "block");

            let result = attr_map_to_leptos(map, &StyleStrategy::Cssom, None);

            let mount_handle = leptos::mount::mount_to(parent.clone(), move || {
                let target = leptos::prelude::NodeRef::<leptos::html::Div>::new();

                use_cssom_styles_from_attrs(target, &result);

                leptos::view! { <div node_ref=target></div> }
            });

            let target = parent
                .first_element_child()
                .expect("mounted element should exist")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("mounted element should be an HtmlElement");

            (mount_handle, target)
        });

        leptos::task::tick().await;

        let style = target.style();

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

        drop(mount_handle);

        assert_eq!(
            style
                .get_property_value("width")
                .expect("width should be readable"),
            ""
        );
        assert_eq!(
            style
                .get_property_value("display")
                .expect("display should be readable"),
            ""
        );
    }

    #[wasm_bindgen_test]
    async fn use_cssom_styles_clears_previous_target_when_node_ref_changes() {
        let owner = Owner::new();

        let (mount_handle, parent, first, set_second) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let (show_second, set_second) = leptos::prelude::signal(false);

            let mount_handle = leptos::mount::mount_to(parent.clone(), move || {
                let target = leptos::prelude::NodeRef::<leptos::html::Div>::new();

                use_cssom_styles(target, || vec![(CssProperty::Width, String::from("120px"))]);

                let child = move || {
                    if show_second.get() {
                        Either::Left(leptos::view! { <div id="second" node_ref=target></div> })
                    } else {
                        Either::Right(leptos::view! { <div id="first" node_ref=target></div> })
                    }
                };

                leptos::view! { {child} }
            });

            let first = parent
                .query_selector("#first")
                .expect("query should succeed")
                .expect("first element should be mounted")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("first element should be an HtmlElement");

            (mount_handle, parent, first, set_second)
        });

        leptos::task::tick().await;

        assert_eq!(
            first
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            "120px"
        );

        set_second.set(true);

        leptos::task::tick().await;

        let second = parent
            .query_selector("#second")
            .expect("query should succeed")
            .expect("second element should be mounted")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("second element should be an HtmlElement");

        assert_eq!(
            first
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            ""
        );
        assert_eq!(
            second
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            "120px"
        );

        drop(mount_handle);
    }

    #[wasm_bindgen_test]
    async fn use_cssom_styles_noops_when_node_ref_is_empty() {
        let owner = Owner::new();

        let mount_handle = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            leptos::mount::mount_to(parent, move || {
                let target = leptos::prelude::NodeRef::<leptos::html::Div>::new();

                use_cssom_styles(target, || vec![(CssProperty::Width, String::from("120px"))]);

                leptos::view! { <span id="empty-cssom-target"></span> }
            })
        });

        leptos::task::tick().await;

        drop(mount_handle);
    }
}
