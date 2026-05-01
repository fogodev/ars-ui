//! Nonce-backed CSS collection for Leptos adapter components.

use leptos::prelude::*;

use crate::attrs::LeptosAttrResult;

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
    pub rules: RwSignal<Vec<NonceCssRule>>,
}

/// Creates and publishes a nonce CSS collector context for the current Leptos owner.
pub fn use_nonce_css_context_provider() -> ArsNonceCssCtx {
    let context = ArsNonceCssCtx {
        rules: RwSignal::new(Vec::<NonceCssRule>::new()),
    };

    provide_context(context);

    context
}

/// Provides a nonce CSS collector context and renders collected rules with `nonce`.
#[component]
#[expect(
    unreachable_pub,
    reason = "ArsNonceCssProvider is re-exported at the adapter crate root."
)]
pub fn ArsNonceCssProvider<T>(nonce: String, children: TypedChildren<T>) -> impl IntoView
where
    View<T>: IntoView,
{
    use_nonce_css_context_provider();

    view! {
        <ArsNonceStyle nonce />
        {children.into_inner()()}
    }
}

/// Renders the current nonce CSS collector context into a nonce style block.
#[component]
#[expect(
    unreachable_pub,
    reason = "ArsNonceStyle is re-exported at the adapter crate root."
)]
pub fn ArsNonceStyle(nonce: String) -> impl IntoView {
    let rules = use_context::<ArsNonceCssCtx>().map(|context| context.rules);

    let css_text = move || {
        rules
            .map(|rules| {
                rules.with(|rules| {
                    rules
                        .iter()
                        .map(|rule| rule.css.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
            .unwrap_or_default()
    };

    view! { <style nonce=nonce>{css_text}</style> }
}

/// Collects a CSS rule using the rule text as its stable key.
pub fn append_nonce_css(css: String) {
    upsert_nonce_css(css.clone(), css);
}

/// Inserts or replaces a CSS rule in the current nonce collector.
pub fn upsert_nonce_css(key: String, css: String) {
    if let Some(context) = use_context::<ArsNonceCssCtx>() {
        upsert_nonce_css_in_rules(context.rules, key, css);
    }
}

/// Removes a CSS rule from the current nonce collector by key.
pub fn remove_nonce_css(key: &str) {
    if let Some(context) = use_context::<ArsNonceCssCtx>() {
        remove_nonce_css_from_rules(context.rules, key);
    }
}

/// Collects nonce CSS from an attribute conversion result when present.
pub fn collect_nonce_css_from_attrs(result: &LeptosAttrResult) {
    if let Some((key, css)) = nonce_css_entry_from_attrs(result) {
        upsert_nonce_css(key, css);
    }
}

/// Schedules nonce CSS collection from an attribute conversion result after render setup.
///
/// Component code should prefer this hook over calling [`collect_nonce_css_from_attrs`]
/// directly from render logic. Use [`use_nonce_css_rule`] when the rule is reactive.
pub fn use_nonce_css_from_attrs(result: &LeptosAttrResult) {
    let entry = nonce_css_entry_from_attrs(result);

    use_nonce_css_rule(move || entry.clone());
}

/// Schedules reactive keyed nonce CSS collection for the current component owner.
///
/// The `rule` closure runs inside a Leptos effect. When it returns `Some`, the
/// keyed rule is inserted or replaced in the current nonce collector. When it
/// returns `None`, or when the component owner is cleaned up, the previously
/// inserted keyed rule is removed.
pub fn use_nonce_css_rule<F>(rule: F)
where
    F: Fn() -> Option<(String, String)> + 'static,
{
    let rules = use_context::<ArsNonceCssCtx>().map(|context| context.rules);

    let applied_key = StoredValue::new_local(None::<String>);

    Effect::new(move |_| {
        let next = rule();

        let Some(rules) = rules else {
            applied_key.set_value(None);

            return;
        };

        if let Some((key, css)) = next {
            if let Some(previous) = applied_key.get_value()
                && previous != key
            {
                remove_nonce_css_from_rules(rules, &previous);
            }

            upsert_nonce_css_in_rules(rules, key.clone(), css);

            applied_key.set_value(Some(key));
        } else {
            if let Some(previous) = applied_key.get_value() {
                remove_nonce_css_from_rules(rules, &previous);
            }

            applied_key.set_value(None);
        }
    });

    on_cleanup(move || {
        if let Some(rules) = rules
            && let Some(previous) = applied_key.get_value()
        {
            remove_nonce_css_from_rules(rules, &previous);
        }

        applied_key.set_value(None);
    });
}

fn upsert_nonce_css_in_rules(rules: RwSignal<Vec<NonceCssRule>>, key: String, css: String) {
    rules.update(|rules| {
        if let Some(rule) = rules.iter_mut().find(|rule| rule.key == key) {
            rule.css = css;
        } else {
            rules.push(NonceCssRule { key, css });
        }
    });
}

fn remove_nonce_css_from_rules(rules: RwSignal<Vec<NonceCssRule>>, key: &str) {
    rules.update(|rules| {
        rules.retain(|rule| rule.key != key);
    });
}

fn nonce_css_entry_from_attrs(result: &LeptosAttrResult) -> Option<(String, String)> {
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
    use leptos::{prelude::*, reactive::owner::Owner};

    use super::{
        ArsNonceCssCtx, ArsNonceCssProvider, ArsNonceStyle, ArsNonceStyleProps, NonceCssRule,
        append_nonce_css, collect_nonce_css_from_attrs, remove_nonce_css, upsert_nonce_css,
        use_nonce_css_context_provider,
    };
    use crate::attr_map_to_leptos;

    #[test]
    fn ars_nonce_css_context_accumulates_rules() {
        let owner = Owner::new();

        owner.with(|| {
            let rules = RwSignal::new(Vec::<NonceCssRule>::new());

            let context = ArsNonceCssCtx { rules };

            context.rules.update(|rules| {
                rules.push(NonceCssRule {
                    key: String::from("one"),
                    css: String::from(".one { color: red; }"),
                });
                rules.push(NonceCssRule {
                    key: String::from("two"),
                    css: String::from(".two { color: blue; }"),
                });
            });

            assert_eq!(
                context
                    .rules
                    .get_untracked()
                    .into_iter()
                    .map(|rule| rule.css)
                    .collect::<Vec<_>>(),
                vec![
                    String::from(".one { color: red; }"),
                    String::from(".two { color: blue; }")
                ]
            );
        });
    }

    #[test]
    fn append_nonce_css_collects_rules_from_context() {
        let owner = Owner::new();

        owner.with(|| {
            let context = use_nonce_css_context_provider();

            let rules = context.rules;

            append_nonce_css(String::from(".one { color: red; }"));
            append_nonce_css(String::from(".two { color: blue; }"));

            assert_eq!(
                rules
                    .get_untracked()
                    .into_iter()
                    .map(|rule| rule.css)
                    .collect::<Vec<_>>(),
                vec![
                    String::from(".one { color: red; }"),
                    String::from(".two { color: blue; }")
                ]
            );
        });
    }

    #[test]
    fn append_nonce_css_is_noop_without_context() {
        let owner = Owner::new();

        owner.with(|| {
            append_nonce_css(String::from(".orphan { color: red; }"));
        });
    }

    #[test]
    fn upsert_nonce_css_replaces_existing_rule() {
        let owner = Owner::new();

        owner.with(|| {
            let rules = RwSignal::new(Vec::<NonceCssRule>::new());

            provide_context(ArsNonceCssCtx { rules });

            upsert_nonce_css(
                String::from("button-root"),
                String::from(".button { color: red; }"),
            );
            upsert_nonce_css(
                String::from("button-root"),
                String::from(".button { color: blue; }"),
            );

            let collected = rules.get_untracked();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "button-root");
            assert_eq!(collected[0].css, ".button { color: blue; }");
        });
    }

    #[test]
    fn remove_nonce_css_removes_rule_by_key() {
        let owner = Owner::new();

        owner.with(|| {
            let rules = RwSignal::new(Vec::<NonceCssRule>::new());

            provide_context(ArsNonceCssCtx { rules });

            upsert_nonce_css(String::from("button-root"), String::from(".button {}"));
            upsert_nonce_css(String::from("other-root"), String::from(".other {}"));
            remove_nonce_css("button-root");

            let collected = rules.get_untracked();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "other-root");
        });
    }

    #[test]
    fn collect_nonce_css_from_attrs_upserts_rule_by_element_id() {
        let owner = Owner::new();

        owner.with(|| {
            let rules = RwSignal::new(Vec::<NonceCssRule>::new());

            provide_context(ArsNonceCssCtx { rules });

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "10px");

            let first = attr_map_to_leptos(
                map,
                &StyleStrategy::Nonce(String::from("nonce")),
                Some("button-root"),
            );

            collect_nonce_css_from_attrs(&first);

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "20px");

            let second = attr_map_to_leptos(
                map,
                &StyleStrategy::Nonce(String::from("nonce")),
                Some("button-root"),
            );

            collect_nonce_css_from_attrs(&second);

            let collected = rules.get_untracked();

            assert_eq!(collected.len(), 1);
            assert_eq!(collected[0].key, "button-root");
            assert!(collected[0].css.contains("width: 20px;"));
        });
    }

    #[test]
    fn ars_nonce_style_renders_provider_owned_context() {
        let owner = Owner::new();

        owner.with(|| {
            let rules = RwSignal::new(Vec::<NonceCssRule>::new());

            provide_context(ArsNonceCssCtx { rules });

            let _view = ArsNonceStyle(ArsNonceStyleProps {
                nonce: String::from("nonce-123"),
            });

            append_nonce_css(String::from(".one { color: red; }"));

            assert_eq!(
                rules
                    .get_untracked()
                    .into_iter()
                    .map(|rule| rule.css)
                    .collect::<Vec<_>>(),
                vec![String::from(".one { color: red; }")]
            );
        });
    }

    #[test]
    fn ars_nonce_css_provider_constructs_owned_context() {
        let owner = Owner::new();

        owner.with(|| {
            let _view = view! {
                <ArsNonceCssProvider nonce=String::from("nonce-123")>
                    <span></span>
                </ArsNonceCssProvider>
            };

            append_nonce_css(String::from(".owned { color: red; }"));

            let context = use_context::<ArsNonceCssCtx>().expect("nonce context should exist");

            assert_eq!(
                context
                    .rules
                    .get_untracked()
                    .into_iter()
                    .map(|rule| rule.css)
                    .collect::<Vec<_>>(),
                vec![String::from(".owned { color: red; }")]
            );
        });
    }
}

#[cfg(all(test, not(feature = "ssr"), target_arch = "wasm32"))]
mod wasm_tests {
    use ars_core::{AttrMap, CssProperty, StyleStrategy};
    use leptos::{prelude::*, reactive::owner::Owner};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{use_nonce_css_context_provider, use_nonce_css_from_attrs, use_nonce_css_rule};
    use crate::attr_map_to_leptos;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn use_nonce_css_from_attrs_removes_rule_when_owner_is_cleaned_up() {
        let parent = Owner::new();

        let (rules, child) = parent.with(|| {
            let rules = RwSignal::new(Vec::<super::NonceCssRule>::new());

            let child = Owner::new();

            child.with(|| {
                provide_context(super::ArsNonceCssCtx { rules });

                let mut map = AttrMap::new();

                map.set_style(CssProperty::Width, "120px");

                let result = attr_map_to_leptos(
                    map,
                    &StyleStrategy::Nonce(String::from("nonce")),
                    Some("nonce-probe"),
                );

                use_nonce_css_from_attrs(&result);
            });

            (rules, child)
        });

        leptos::task::tick().await;

        assert_eq!(rules.get_untracked().len(), 1);

        child.cleanup();

        assert!(
            rules.get_untracked().is_empty(),
            "nonce hook cleanup should remove the rule owned by the cleaned-up owner",
        );

        parent.cleanup();
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_from_attrs_collects_with_provider_context() {
        let owner = Owner::new();

        let rules = owner.with(|| {
            let context = use_nonce_css_context_provider();

            let mut map = AttrMap::new();

            map.set_style(CssProperty::Width, "120px");

            let result = attr_map_to_leptos(
                map,
                &StyleStrategy::Nonce(String::from("nonce")),
                Some("nonce-probe"),
            );

            use_nonce_css_from_attrs(&result);

            context.rules
        });

        leptos::task::tick().await;

        assert_eq!(rules.get_untracked().len(), 1);

        owner.cleanup();
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_noops_without_collector_context() {
        let owner = Owner::new();

        owner.with(|| {
            use_nonce_css_rule(|| {
                Some((
                    String::from("missing-context"),
                    String::from(".missing-context {}"),
                ))
            });
        });

        leptos::task::tick().await;

        owner.cleanup();
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_replaces_and_removes_reactive_rule() {
        let owner = Owner::new();

        let (rules, enabled, width) = owner.with(|| {
            let context = use_nonce_css_context_provider();

            let (enabled, set_enabled) = signal(true);

            let (width, set_width) = signal(String::from("120px"));

            use_nonce_css_rule(move || {
                enabled.get().then(|| {
                    (
                        String::from("reactive-rule"),
                        format!(".reactive {{ width: {}; }}", width.get()),
                    )
                })
            });

            (context.rules, set_enabled, set_width)
        });

        leptos::task::tick().await;

        assert_eq!(rules.get_untracked().len(), 1);
        assert_eq!(rules.get_untracked()[0].css, ".reactive { width: 120px; }");

        width.set(String::from("200px"));

        leptos::task::tick().await;

        let collected = rules.get_untracked();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].css, ".reactive { width: 200px; }");

        enabled.set(false);

        leptos::task::tick().await;

        assert!(rules.get_untracked().is_empty());
    }

    #[wasm_bindgen_test]
    async fn use_nonce_css_rule_removes_previous_key_when_key_changes() {
        let owner = Owner::new();

        let (rules, key) = owner.with(|| {
            let context = use_nonce_css_context_provider();

            let (key, set_key) = signal(String::from("first-rule"));

            use_nonce_css_rule(move || {
                let key = key.get();
                let formatted = format!(".{key} {{ width: 120px; }}");

                Some((key, formatted))
            });

            (context.rules, set_key)
        });

        leptos::task::tick().await;

        assert_eq!(rules.get_untracked().len(), 1);
        assert_eq!(rules.get_untracked()[0].key, "first-rule");

        key.set(String::from("second-rule"));

        leptos::task::tick().await;

        let collected = rules.get_untracked();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].key, "second-rule");
        assert_eq!(collected[0].css, ".second-rule { width: 120px; }");
    }
}
