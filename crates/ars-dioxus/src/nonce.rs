//! Nonce-backed CSS collection for Dioxus adapter components.

use dioxus::prelude::*;

use crate::attrs::DioxusAttrResult;

/// Stable nonce CSS rule keyed by the styled element or rule owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonceCssRule {
    /// Stable key used to replace this rule on rerender.
    pub key: String,

    /// CSS rule text rendered into the nonce style block.
    pub css: String,
}

/// Context for collecting nonce CSS rules during rendering.
#[derive(Clone, Copy, Debug)]
pub struct ArsNonceCssCtx {
    /// Reactive signal holding collected CSS rules keyed by owner.
    pub rules: Signal<Vec<NonceCssRule>>,
}

/// Creates and publishes a nonce CSS collector context for the current Dioxus scope.
pub fn use_nonce_css_context_provider() -> ArsNonceCssCtx {
    let rules = use_signal(Vec::<NonceCssRule>::new);

    let context = ArsNonceCssCtx { rules };

    use_context_provider(move || context);

    context
}

/// Provides a nonce CSS collector context and renders collected rules with `nonce`.
#[component]
pub fn ArsNonceCssProvider(nonce: String, children: Element) -> Element {
    use_nonce_css_context_provider();

    rsx! {
        ArsNonceStyle { nonce }
        {children}
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx event attribute labels are parsed before macro expansion."
)]
mod wasm_tests {
    use ars_core::{AttrMap, CssProperty, StyleStrategy};
    use dioxus::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{use_nonce_css_context_provider, use_nonce_css_from_attrs, use_nonce_css_rule};
    use crate::attr_map_to_dioxus;

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

    #[component]
    fn NonceProbe() -> Element {
        let mut map = AttrMap::new();

        map.set_style(CssProperty::Width, "120px");

        let result = attr_map_to_dioxus(
            map,
            &StyleStrategy::Nonce(String::from("nonce")),
            Some("nonce-probe"),
        );

        use_nonce_css_from_attrs(&result);

        rsx! {
            span { id: "nonce-probe" }
        }
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_from_attrs_removes_rule_when_component_unmounts() {
        fn app() -> Element {
            let context = use_nonce_css_context_provider();

            let mut show = use_signal(|| true);

            let count = context.rules.read().len();
            let hide = move |_| show.set(false);

            rsx! {
                button { id: "hide-nonce-probe", onclick: hide, "hide" }
                div { id: "nonce-rule-count", "{count}" }
                if show() {
                    NonceProbe {}
                }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(app),
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        let count = container
            .query_selector("#nonce-rule-count")
            .expect("query should succeed")
            .expect("count should exist");

        assert_eq!(count.text_content().as_deref(), Some("1"));

        container
            .query_selector("#hide-nonce-probe")
            .expect("query should succeed")
            .expect("hide button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("hide button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

        assert_eq!(count.text_content().as_deref(), Some("0"));
    }

    #[component]
    fn ReactiveNonceProbe(width: Signal<String>, enabled: Signal<bool>) -> Element {
        use_nonce_css_rule(move || {
            enabled().then(|| {
                (
                    String::from("reactive-rule"),
                    format!(".reactive {{ width: {}; }}", width()),
                )
            })
        });

        rsx! {
            span { id: "reactive-nonce-probe" }
        }
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_replaces_and_removes_reactive_rule() {
        fn app() -> Element {
            let context = use_nonce_css_context_provider();

            let mut width = use_signal(|| String::from("120px"));
            let mut enabled = use_signal(|| true);

            let update = move |_| width.set(String::from("200px"));
            let remove = move |_| enabled.set(false);

            let css = context
                .rules
                .read()
                .first()
                .map(|rule| rule.css.clone())
                .unwrap_or_default();

            rsx! {
                button { id: "update-nonce-rule", onclick: update, "update" }
                button { id: "remove-nonce-rule", onclick: remove, "remove" }
                div { id: "nonce-rule-css", "{css}" }
                ReactiveNonceProbe { width, enabled }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(app),
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        let css = container
            .query_selector("#nonce-rule-css")
            .expect("query should succeed")
            .expect("css output should exist");

        assert_eq!(
            css.text_content().as_deref(),
            Some(".reactive { width: 120px; }")
        );

        container
            .query_selector("#update-nonce-rule")
            .expect("query should succeed")
            .expect("update button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("update button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

        assert_eq!(
            css.text_content().as_deref(),
            Some(".reactive { width: 200px; }")
        );

        container
            .query_selector("#remove-nonce-rule")
            .expect("query should succeed")
            .expect("remove button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("remove button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

        assert_eq!(css.text_content().as_deref(), Some(""));
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_noops_without_collector_context() {
        fn app() -> Element {
            use_nonce_css_rule(|| {
                Some((
                    String::from("missing-context"),
                    String::from(".missing-context {}"),
                ))
            });

            rsx! {
                div { id: "nonce-no-context", "mounted" }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(app),
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        assert!(
            container
                .query_selector("#nonce-no-context")
                .expect("query should succeed")
                .is_some()
        );
    }

    #[component]
    fn KeyedNonceProbe(rule_key: Signal<String>) -> Element {
        use_nonce_css_rule(move || {
            let key = rule_key();
            Some((key.clone(), format!(".{key} {{ width: 120px; }}")))
        });

        rsx! {
            span { id: "keyed-nonce-probe" }
        }
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_removes_previous_key_when_key_changes() {
        fn app() -> Element {
            let context = use_nonce_css_context_provider();

            let mut key = use_signal(|| String::from("first-rule"));

            let update_key = move |_| key.set(String::from("second-rule"));

            let collected = context.rules.read().clone();

            let count = collected.len();

            let current_key = collected
                .first()
                .map(|rule| rule.key.clone())
                .unwrap_or_default();

            rsx! {
                button { id: "update-nonce-key", onclick: update_key, "update" }
                div { id: "nonce-key-count", "{count}" }
                div { id: "nonce-current-key", "{current_key}" }
                KeyedNonceProbe { rule_key: key }
            }
        }

        let document = document();

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(app),
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush_browser_tasks().await;

        let count = container
            .query_selector("#nonce-key-count")
            .expect("query should succeed")
            .expect("count should exist");

        let current_key = container
            .query_selector("#nonce-current-key")
            .expect("query should succeed")
            .expect("current key should exist");

        assert_eq!(count.text_content().as_deref(), Some("1"));
        assert_eq!(current_key.text_content().as_deref(), Some("first-rule"));

        container
            .query_selector("#update-nonce-key")
            .expect("query should succeed")
            .expect("update button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("update button should be an HtmlElement")
            .click();

        flush_browser_tasks().await;

        assert_eq!(count.text_content().as_deref(), Some("1"));
        assert_eq!(current_key.text_content().as_deref(), Some("second-rule"));
    }
}

/// Renders the current nonce CSS collector context into a nonce style block.
#[component]
pub fn ArsNonceStyle(nonce: String) -> Element {
    let context = try_use_context::<ArsNonceCssCtx>();

    let css_text = use_memo(move || {
        context
            .as_ref()
            .map(|context| {
                context
                    .rules
                    .read()
                    .iter()
                    .map(|rule| rule.css.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    });

    rsx! {
        style { nonce, {css_text()} }
    }
}

/// Collects a CSS rule using the rule text as its stable key.
pub fn append_nonce_css(css: String) {
    upsert_nonce_css(css.clone(), css);
}

/// Inserts or replaces a CSS rule in the current nonce collector.
pub fn upsert_nonce_css(key: String, css: String) {
    if let Some(context) = try_use_context::<ArsNonceCssCtx>() {
        upsert_nonce_css_in_rules(context.rules, key, css);
    }
}

/// Removes a CSS rule from the current nonce collector by key.
pub fn remove_nonce_css(key: &str) {
    if let Some(context) = try_use_context::<ArsNonceCssCtx>() {
        remove_nonce_css_from_rules(context.rules, key);
    }
}

/// Collects nonce CSS from an attribute conversion result when present.
pub fn collect_nonce_css_from_attrs(result: &DioxusAttrResult) {
    if let Some((key, css)) = nonce_css_entry_from_attrs(result) {
        upsert_nonce_css(key, css);
    }
}

/// Schedules nonce CSS collection from an attribute conversion result after render setup.
///
/// Component code should prefer this hook over calling [`collect_nonce_css_from_attrs`]
/// directly from render logic. Use [`use_nonce_css_rule`] when the rule is reactive.
pub fn use_nonce_css_from_attrs(result: &DioxusAttrResult) {
    let entry = nonce_css_entry_from_attrs(result);

    use_nonce_css_rule(move || entry.clone());
}

/// Schedules reactive keyed nonce CSS collection for the current component scope.
///
/// The `rule` closure runs inside a Dioxus effect. When it returns `Some`, the
/// keyed rule is inserted or replaced in the current nonce collector. When it
/// returns `None`, or when the component scope is dropped, the previously
/// inserted keyed rule is removed.
pub fn use_nonce_css_rule<F>(rule: F)
where
    F: Fn() -> Option<(String, String)> + 'static,
{
    let rules = try_use_context::<ArsNonceCssCtx>().map(|context| context.rules);

    let mut applied_key = use_hook(|| CopyValue::new(None::<String>));

    use_effect(move || {
        let next = rule();

        let Some(rules) = rules else {
            applied_key.set(None);
            return;
        };

        if let Some((key, css)) = next {
            if let Some(previous) = applied_key.peek().as_ref()
                && previous != &key
            {
                remove_nonce_css_from_rules(rules, previous);
            }

            upsert_nonce_css_in_rules(rules, key.clone(), css);

            applied_key.set(Some(key));
        } else {
            if let Some(previous) = applied_key.write().take() {
                remove_nonce_css_from_rules(rules, &previous);
            }
        }
    });

    use_drop(move || {
        if let Some(rules) = rules
            && let Ok(mut applied_key) = applied_key.try_write()
            && let Some(previous) = applied_key.take()
        {
            remove_nonce_css_from_rules(rules, &previous);
        }
    });
}

fn upsert_nonce_css_in_rules(mut rules: Signal<Vec<NonceCssRule>>, key: String, css: String) {
    let mut rules = rules.write();

    if let Some(rule) = rules.iter_mut().find(|rule| rule.key == key) {
        rule.css = css;
    } else {
        rules.push(NonceCssRule { key, css });
    }
}

fn remove_nonce_css_from_rules(mut rules: Signal<Vec<NonceCssRule>>, key: &str) {
    rules.write().retain(|rule| rule.key != key);
}

fn nonce_css_entry_from_attrs(result: &DioxusAttrResult) -> Option<(String, String)> {
    if result.nonce_css.is_empty() {
        return None;
    }

    let key = result
        .nonce_css_key
        .clone()
        .unwrap_or_else(|| result.nonce_css.clone());

    Some((key, result.nonce_css.clone()))
}

#[cfg(test)]
mod tests {
    use ars_core::{AttrMap, CssProperty, StyleStrategy};
    use dioxus::prelude::*;

    use super::{
        ArsNonceCssCtx, ArsNonceCssProvider, ArsNonceStyle, NonceCssRule, append_nonce_css,
        collect_nonce_css_from_attrs, remove_nonce_css, upsert_nonce_css,
        use_nonce_css_context_provider,
    };
    use crate::attr_map_to_dioxus;

    #[test]
    fn ars_nonce_style_mounts_and_provides_context() {
        fn app() -> Element {
            let rules = use_signal(Vec::<NonceCssRule>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

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
            let context = use_nonce_css_context_provider();

            let rules = context.rules;

            append_nonce_css(".rule-1 { color: red; }".into());
            append_nonce_css(".rule-2 { color: blue; }".into());

            let collected = rules
                .peek()
                .iter()
                .map(|rule| rule.css.clone())
                .collect::<Vec<_>>();

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
            append_nonce_css("orphan rule".into());

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn upsert_nonce_css_replaces_existing_rule() {
        fn app() -> Element {
            let rules = use_signal(Vec::<NonceCssRule>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

            upsert_nonce_css("button-root".into(), ".button { color: red; }".into());
            upsert_nonce_css("button-root".into(), ".button { color: blue; }".into());

            let collected = rules.peek().clone();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "button-root");
            assert_eq!(collected[0].css, ".button { color: blue; }");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn remove_nonce_css_removes_rule_by_key() {
        fn app() -> Element {
            let rules = use_signal(Vec::<NonceCssRule>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

            upsert_nonce_css("button-root".into(), ".button {}".into());
            upsert_nonce_css("other-root".into(), ".other {}".into());
            remove_nonce_css("button-root");

            let collected = rules.peek().clone();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "other-root");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn collect_nonce_css_from_attrs_upserts_rule_by_element_id() {
        fn app() -> Element {
            let rules = use_signal(Vec::<NonceCssRule>::new);

            use_context_provider(|| ArsNonceCssCtx { rules });

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "10px");

            let first = attr_map_to_dioxus(
                map,
                &StyleStrategy::Nonce(String::from("nonce")),
                Some("button-root"),
            );

            collect_nonce_css_from_attrs(&first);

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "20px");

            let second = attr_map_to_dioxus(
                map,
                &StyleStrategy::Nonce(String::from("nonce")),
                Some("button-root"),
            );

            collect_nonce_css_from_attrs(&second);

            let collected = rules.peek().clone();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "button-root");
            assert!(collected[0].css.contains("width: 20px;"));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn ars_nonce_css_provider_owns_context() {
        fn app() -> Element {
            rsx! {
                ArsNonceCssProvider { nonce: "nonce-123".to_string(), Probe {} }
            }
        }

        #[component]
        fn Probe() -> Element {
            append_nonce_css(".owned { color: red; }".into());

            let context = try_use_context::<ArsNonceCssCtx>().expect("nonce context should exist");
            let collected = context.rules.peek().clone();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].css, ".owned { color: red; }");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }
}
