//! AttrMap-to-Dioxus attribute conversion with attribute name interning.
//!
//! Bridges the framework-agnostic [`AttrMap`] from `ars-core` to Dioxus's
//! [`Attribute`] type, supporting all three [`StyleStrategy`] variants
//! (Inline, CSSOM, Nonce) for CSP-aware style rendering.

use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex, MutexGuard},
};

use ars_core::{
    AttrMap, AttrMapParts, AttrValue, CssProperty, HtmlAttr, StyleStrategy, styles_to_nonce_css,
};
use dioxus::prelude::*;
use dioxus_core::AttributeValue;

use crate::provider::{ArsContext, warn_missing_provider};

// ── Attribute name interning ────────────────────────────────────────

/// Intern pool for attribute names not covered by static fast paths.
///
/// Dioxus [`Attribute::new`] requires `&'static str`. Known HTML, ARIA, and
/// ars-generated `data-*` names are compile-time constants. Unknown `data-*`
/// names are interned on first use as a Dioxus compatibility fallback.
static ATTR_NAMES: LazyLock<Mutex<HashSet<&'static str>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn attr_name_pool() -> MutexGuard<'static, HashSet<&'static str>> {
    ATTR_NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn static_data_attr_name(suffix: &str) -> Option<&'static str> {
    match suffix {
        "ars-animated" => Some("data-ars-animated"),
        "ars-disable-outside-pointer-events" => Some("data-ars-disable-outside-pointer-events"),
        "ars-disabled" => Some("data-ars-disabled"),
        "ars-drag-over" => Some("data-ars-drag-over"),
        "ars-dragging" => Some("data-ars-dragging"),
        "ars-drop-operation" => Some("data-ars-drop-operation"),
        "ars-drop-position" => Some("data-ars-drop-position"),
        "ars-focus-visible" => Some("data-ars-focus-visible"),
        "ars-focus-within" => Some("data-ars-focus-within"),
        "ars-focus-within-visible" => Some("data-ars-focus-within-visible"),
        "ars-focused" => Some("data-ars-focused"),
        "ars-hovered" => Some("data-ars-hovered"),
        "ars-index" => Some("data-ars-index"),
        "ars-invalid" => Some("data-ars-invalid"),
        "ars-loading" => Some("data-ars-loading"),
        "ars-long-pressing" => Some("data-ars-long-pressing"),
        "ars-moving" => Some("data-ars-moving"),
        "ars-part" => Some("data-ars-part"),
        "ars-placement" => Some("data-ars-placement"),
        "ars-presence" => Some("data-ars-presence"),
        "ars-pressed" => Some("data-ars-pressed"),
        "ars-prevent-focus-on-press" => Some("data-ars-prevent-focus-on-press"),
        "ars-readonly" => Some("data-ars-readonly"),
        "ars-scope" => Some("data-ars-scope"),
        "ars-segment" => Some("data-ars-segment"),
        "ars-shape" => Some("data-ars-shape"),
        "ars-size" => Some("data-ars-size"),
        "ars-state" => Some("data-ars-state"),
        "ars-variant" => Some("data-ars-variant"),
        "ars-visually-hidden" => Some("data-ars-visually-hidden"),
        "ars-visually-hidden-focusable" => Some("data-ars-visually-hidden-focusable"),
        _ => None,
    }
}

fn intern_attr_name(attr: &HtmlAttr) -> &'static str {
    // Fast path: if HtmlAttr has a known static name, return it directly.
    if let Some(name) = attr.static_name() {
        return name;
    }

    if let HtmlAttr::Data(suffix) = attr
        && let Some(name) = static_data_attr_name(suffix)
    {
        return name;
    }

    // Fallback: unknown Data(...) attributes need a process-lifetime name for Dioxus.
    let name = attr.to_string(); // e.g., "data-ars-state"

    let mut pool = attr_name_pool();

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

    /// Stable key for [`Self::nonce_css`] when [`StyleStrategy::Nonce`] is active.
    pub nonce_css_key: Option<String>,

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
                AttributeValue::Bool(true),
                None,
                false,
            )),

            // Reactive variants are evaluated at conversion time. Dioxus
            // re-runs component bodies whenever a tracked signal changes,
            // so calling the closure during each `attr_map_to_dioxus`
            // invocation is the framework-idiomatic reactive path —
            // each render produces a fresh Attribute with the current
            // value. The Reactive variants exist on the agnostic
            // `AttrMap` so consumers writing component glue do not need
            // to reach for adapter-specific reactive primitives.
            AttrValue::Reactive(f) => Some(Attribute::new(
                intern_attr_name(&key),
                AttributeValue::Text(f()),
                None,
                false,
            )),

            // Reactive booleans follow the HTML presence/absence
            // semantics symmetric with the static [`AttrValue::Bool`]
            // path: `true` materializes to a Dioxus boolean Attribute,
            // `false` skips the attribute entirely. Dioxus re-runs
            // component bodies on signal changes, so each render's
            // conversion picks up the current closure result.
            AttrValue::ReactiveBool(f) => f().then(|| {
                Attribute::new(
                    intern_attr_name(&key),
                    AttributeValue::Bool(true),
                    None,
                    false,
                )
            }),

            AttrValue::Bool(false) | AttrValue::None => None,
        })
        .collect::<Vec<_>>();

    let mut cssom_styles = Vec::new();

    let mut nonce_css_key = None;

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

                nonce_css_key = Some(id.to_string());

                nonce_css = styles_to_nonce_css(id, &styles);
            }
        }
    }

    DioxusAttrResult {
        attrs: result,
        cssom_styles,
        nonce_css_key,
        nonce_css,
    }
}

/// Convenience wrapper around [`attr_map_to_dioxus`] for callers that
/// always render with [`StyleStrategy::Inline`] and only need the
/// [`Attribute`] vector ready for spreading via `..attrs`.
///
/// Equivalent to:
///
/// ```ignore
/// let DioxusAttrResult { attrs, .. } =
///     attr_map_to_dioxus(map, &StyleStrategy::Inline, None);
/// ```
///
/// Use the full [`attr_map_to_dioxus`] when [`StyleStrategy::Cssom`] or
/// [`StyleStrategy::Nonce`] is in play and the caller needs to apply
/// `cssom_styles` to the DOM or inject `nonce_css` into a `<style>`
/// block.
#[must_use]
pub fn attr_map_to_dioxus_inline_attrs(map: AttrMap) -> Vec<Attribute> {
    attr_map_to_dioxus(map, &StyleStrategy::Inline, None).attrs
}

// ── CSSOM helper ────────────────────────────────────────────────────

/// Tracks CSS properties applied through CSSOM so later syncs can remove stale entries.
#[cfg(feature = "web")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssomStyleHandle {
    /// CSS properties applied during the previous sync.
    applied: Vec<CssProperty>,
}

#[cfg(feature = "web")]
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
                && let Err(error) = style.remove_property(&property.to_string())
            {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&error);

                #[cfg(not(debug_assertions))]
                drop(error);
            }
        }

        for (property, value) in styles {
            if let Err(error) = style.set_property(&property.to_string(), value) {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&error);

                #[cfg(not(debug_assertions))]
                drop(error);
            }

            self.applied.push(property.clone());
        }
    }

    /// Removes every property currently owned by this handle.
    pub fn clear(&mut self, el: &web_sys::HtmlElement) {
        let style = el.style();

        for property in self.applied.drain(..) {
            if let Err(error) = style.remove_property(&property.to_string()) {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&error);

                #[cfg(not(debug_assertions))]
                drop(error);
            }
        }
    }
}

/// Apply styles to a DOM element via the CSSOM API.
///
/// Used when [`StyleStrategy::Cssom`] is active. Iterates the style entries
/// and calls `element.style().setProperty()` for each one.
///
/// Prefer [`CssomStyleHandle`] when styles can change over time, so stale
/// properties are removed on later renders.
#[cfg(feature = "web")]
pub fn apply_styles_cssom(el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    CssomStyleHandle::new().sync(el, styles);
}

/// Synchronizes CSSOM styles from an attribute conversion result to an event target.
///
/// The hook owns a persistent [`CssomStyleHandle`], so styles removed from later
/// syncs are also removed from the DOM element. Cleanup clears every property
/// owned by the handle. Use [`use_cssom_styles`] when the style list is reactive.
#[cfg(feature = "web")]
pub fn use_cssom_styles_from_attrs(
    target: Signal<Option<web_sys::EventTarget>>,
    result: &DioxusAttrResult,
) {
    let styles = result.cssom_styles.clone();

    use_cssom_styles(target, move || styles.clone());
}

/// Synchronizes reactive CSSOM styles to an event target.
///
/// The `styles` closure runs inside a Dioxus effect. Signal reads inside it
/// resubscribe the hook so changed styles are applied, stale properties are
/// removed, and styles are cleared from the previous target when `target`
/// points at a different element.
#[cfg(feature = "web")]
pub fn use_cssom_styles<F>(target: Signal<Option<web_sys::EventTarget>>, styles: F)
where
    F: Fn() -> Vec<(CssProperty, String)> + 'static,
{
    use web_sys::wasm_bindgen::JsCast as _;

    let mut handle = use_hook(|| CopyValue::new(CssomStyleHandle::new()));
    let mut applied_element = use_hook(|| CopyValue::new(None::<web_sys::HtmlElement>));

    use_effect(move || {
        let styles = styles();

        let element = target
            .read()
            .as_ref()
            .and_then(|target| target.clone().dyn_into::<web_sys::HtmlElement>().ok());

        if let Some(previous) = applied_element.write().take() {
            handle.write().clear(&previous);
        }

        let Some(element) = element else {
            return;
        };

        handle.write().sync(&element, &styles);

        applied_element.set(Some(element));
    });

    use_drop(move || {
        if let Ok(mut applied_element) = applied_element.try_write()
            && let Some(element) = applied_element.take()
            && let Ok(mut handle) = handle.try_write()
        {
            handle.clear(&element);
        }
    });
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
    fn intern_attr_name_static_data_fast_path_returns_static_slices() {
        let first = intern_attr_name(&HtmlAttr::Data("ars-state"));
        let second = intern_attr_name(&HtmlAttr::Data("ars-state"));

        assert_eq!(first, "data-ars-state");
        assert_eq!(second, "data-ars-state");
        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
        assert_eq!(static_data_attr_name("ars-state"), Some("data-ars-state"));
    }

    #[test]
    fn intern_attr_name_slow_path_interns_unknown_data_attributes() {
        let first = intern_attr_name(&HtmlAttr::Data("ars-custom-test"));
        let second = intern_attr_name(&HtmlAttr::Data("ars-custom-test"));

        assert_eq!(first, "data-ars-custom-test");
        assert_eq!(second, "data-ars-custom-test");
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

    /// Helper: extract bool value from an Attribute.
    const fn bool_value(attr: &Attribute) -> Option<bool> {
        match attr.value {
            AttributeValue::Bool(value) => Some(value),
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
    fn attr_map_to_dioxus_inline_attrs_matches_full_helper() {
        let attrs = attr_map_to_dioxus_inline_attrs(build_test_map());

        let class_attr = find_attr(&attrs, "class").expect("class attr present");

        assert_eq!(text_value(class_attr), Some("btn"));

        let style_attr = find_attr(&attrs, "style").expect("style attr present");

        assert_eq!(text_value(style_attr), Some("width: 100px;"));

        assert_eq!(
            attrs.len(),
            2,
            "Inline strategy folds styles into a single `style` attr; no additional fields leak through",
        );
    }

    #[test]
    fn attr_map_to_dioxus_inline_attrs_filters_false_and_none_values() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Class, "kept")
            .set_bool(HtmlAttr::Disabled, false)
            .set(HtmlAttr::Aria(AriaAttr::Label), AttrValue::None);

        let attrs = attr_map_to_dioxus_inline_attrs(map);

        assert!(find_attr(&attrs, "class").is_some());
        assert!(
            find_attr(&attrs, "disabled").is_none(),
            "Bool(false) entries must be dropped — wrapper must not bypass the underlying filter",
        );
        assert!(
            find_attr(&attrs, "aria-label").is_none(),
            "AttrValue::None entries must be dropped",
        );
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
        assert_eq!(result.nonce_css_key, Some(String::from("el-1")));
        assert!(result.nonce_css.contains("[data-ars-style-id=\"el-1\"]"));
        assert!(result.nonce_css.contains("width: 100px;"));
        assert!(result.cssom_styles.is_empty());
    }

    #[test]
    fn attr_map_to_dioxus_nonce_strategy_escapes_selector_attribute_value() {
        let result = attr_map_to_dioxus(
            build_test_map(),
            &StyleStrategy::Nonce("n123".into()),
            Some("root\"quoted\\path]"),
        );

        assert!(
            result
                .nonce_css
                .contains("[data-ars-style-id=\"root\\\"quoted\\\\path]\"]")
        );
    }

    #[test]
    fn attr_map_to_dioxus_nonce_strategy_escapes_control_characters() {
        for (raw, escaped) in [
            ("root\nline", "root\\A line"),
            ("root\rline", "root\\D line"),
            ("root\tline", "root\\9 line"),
            ("root\0line", "root\\FFFD line"),
        ] {
            let result = attr_map_to_dioxus(
                build_test_map(),
                &StyleStrategy::Nonce("n123".into()),
                Some(raw),
            );

            assert!(
                result
                    .nonce_css
                    .contains(&format!("[data-ars-style-id=\"{escaped}\"]"))
            );
        }
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

        assert_eq!(bool_value(disabled_attr), Some(true));

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
        assert_eq!(result.nonce_css_key, None);
        assert!(result.nonce_css.is_empty());
        assert!(result.cssom_styles.is_empty());
    }

    /// Reactive variants are evaluated at conversion time — Dioxus
    /// re-runs component bodies whenever a tracked signal changes, so
    /// each render produces a fresh Attribute with the closure's
    /// current value. This test pins the materialisation contract:
    /// the rendered Attribute must carry the closure's current return
    /// value, not a placeholder.
    #[test]
    fn attr_map_to_dioxus_materializes_reactive_string_to_current_value() {
        let mut map = AttrMap::new();

        map.set(
            HtmlAttr::Aria(AriaAttr::Label),
            AttrValue::reactive(|| String::from("Schließen")),
        );

        let result = attr_map_to_dioxus(map, &StyleStrategy::Inline, None);

        let aria = find_attr(&result.attrs, "aria-label").expect("aria-label attr present");

        assert_eq!(text_value(aria), Some("Schließen"));
    }

    /// Reactive booleans materialize with HTML boolean semantics
    /// symmetric with [`AttrValue::Bool`]: `true` renders the
    /// attribute as a Dioxus boolean value, `false` skips it.
    #[test]
    fn attr_map_to_dioxus_reactive_bool_follows_presence_semantics() {
        let mut map = AttrMap::new();

        map.set(HtmlAttr::Disabled, AttrValue::reactive_bool(|| true))
            .set(HtmlAttr::Required, AttrValue::reactive_bool(|| false));

        let result = attr_map_to_dioxus(map, &StyleStrategy::Inline, None);

        let disabled = find_attr(&result.attrs, "disabled").expect("disabled attr present");

        assert_eq!(bool_value(disabled), Some(true));
        assert!(find_attr(&result.attrs, "required").is_none());
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
                Arc::new(crate::platform::NullPlatform),
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
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx event attribute labels are parsed before macro expansion."
)]
mod wasm_tests {
    use std::{rc::Rc, sync::Arc};

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

    async fn flush_browser_tasks() {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            web_sys::window()
                .expect("window should exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50)
                .expect("setTimeout should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
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

    const fn bool_value(attr: &Attribute) -> Option<bool> {
        match attr.value {
            AttributeValue::Bool(value) => Some(value),
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
            bool_value(find_attr(&inline.attrs, "disabled").expect("disabled attr present")),
            Some(true)
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
    fn style_strategy_context_works_on_wasm() {
        fn app() -> Element {
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
                Arc::new(crate::platform::NullPlatform),
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
    async fn use_cssom_styles_from_attrs_syncs_styles() {
        #[derive(Clone)]
        struct CssomHookProps {
            target: Rc<web_sys::HtmlElement>,
        }

        impl PartialEq for CssomHookProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.target, &other.target)
            }
        }

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn fixture(props: CssomHookProps) -> Element {
            let target: web_sys::EventTarget = (*props.target).clone().into();
            let target = use_signal(move || Some(target.clone()));

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "120px")
                .set_style(CssProperty::Display, "block");

            let result = attr_map_to_dioxus(map, &StyleStrategy::Cssom, None);

            use_cssom_styles_from_attrs(target, &result);

            rsx! {
                div {}
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let target = Rc::new(
            document
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("element should cast to HtmlElement"),
        );

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            fixture,
            CssomHookProps {
                target: Rc::clone(&target),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container),
        );

        flush_browser_tasks().await;

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
    }

    #[wasm_bindgen_test]
    async fn use_cssom_styles_clears_previous_target_when_signal_changes() {
        #[derive(Clone)]
        struct CssomHookProps {
            first: Rc<web_sys::HtmlElement>,
            second: Rc<web_sys::HtmlElement>,
        }

        impl PartialEq for CssomHookProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.first, &other.first) && Rc::ptr_eq(&self.second, &other.second)
            }
        }

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn fixture(props: CssomHookProps) -> Element {
            let first: web_sys::EventTarget = (*props.first).clone().into();
            let second: web_sys::EventTarget = (*props.second).clone().into();

            let mut target = use_signal(move || Some(first.clone()));

            let swap_target = move |_| target.set(Some(second.clone()));

            use_cssom_styles(target, || vec![(CssProperty::Width, String::from("120px"))]);

            rsx! {
                button { id: "swap-cssom-target", onclick: swap_target, "swap" }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let first = Rc::new(
            document
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("element should cast to HtmlElement"),
        );

        let second = Rc::new(
            document
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("element should cast to HtmlElement"),
        );

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            fixture,
            CssomHookProps {
                first: Rc::clone(&first),
                second: Rc::clone(&second),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        assert_eq!(
            first
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            "120px"
        );

        container
            .query_selector("#swap-cssom-target")
            .expect("query should succeed")
            .expect("swap button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("swap button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

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
    }

    #[wasm_bindgen_test]
    async fn use_cssom_styles_clears_previous_target_when_signal_becomes_none() {
        #[derive(Clone)]
        struct CssomHookProps {
            target: Rc<web_sys::HtmlElement>,
        }

        impl PartialEq for CssomHookProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.target, &other.target)
            }
        }

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn fixture(props: CssomHookProps) -> Element {
            let initial: web_sys::EventTarget = (*props.target).clone().into();

            let mut target = use_signal(move || Some(initial.clone()));

            let clear_target = move |_| target.set(None);

            use_cssom_styles(target, || vec![(CssProperty::Width, String::from("120px"))]);

            rsx! {
                button { id: "clear-cssom-target", onclick: clear_target, "clear" }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let target = Rc::new(
            document
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("element should cast to HtmlElement"),
        );

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            fixture,
            CssomHookProps {
                target: Rc::clone(&target),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        assert_eq!(
            target
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            "120px"
        );

        container
            .query_selector("#clear-cssom-target")
            .expect("query should succeed")
            .expect("clear button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("clear button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

        assert_eq!(
            target
                .style()
                .get_property_value("width")
                .expect("width should be readable"),
            ""
        );
    }
}
