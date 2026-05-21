//! SSR oracle tests for the Dioxus `ArsProvider` adapter.
//!
//! Covers spec §28 / §29 scenarios from
//! `spec/dioxus-components/utility/ars-provider.md`: `<div dir>` wrapper
//! rendering, direction inference vs. explicit override, provider-owned
//! auto-render of `ArsNonceStyle` when `style_strategy` is
//! `StyleStrategy::Nonce(...)`, and `dioxus_platform` default resolution via
//! feature flags. Browser DOM and reactive-update oracles live in the inline
//! `wasm_tests` module of `src/provider.rs` and in `tests/ars_provider_wasm.rs`.

#![cfg(not(target_arch = "wasm32"))]

use ars_core::{ColorMode, StyleStrategy};
use ars_dioxus::{ArsContext, ArsProvider, ArsProviderProps, default_dioxus_platform};
use ars_i18n::{Direction, Locale};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn ars_provider_renders_ltr_wrapper_for_default_props() {
    fn app() -> Element {
        rsx! {
            ArsProvider {
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"dir="ltr""#),
        "default provider should render dir=\"ltr\": {html}"
    );
    assert!(
        html.contains(r#"id="child""#),
        "children should render inside the provider wrapper: {html}"
    );
    assert!(
        !html.contains("<style nonce"),
        "default StyleStrategy::Inline must not emit a nonce style tag: {html}"
    );
}

#[test]
fn ars_provider_infers_rtl_direction_from_rtl_locale() {
    fn app() -> Element {
        let locale = use_signal(|| Locale::parse("ar-SA").expect("locale should parse"));

        rsx! {
            ArsProvider { locale,
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"dir="rtl""#),
        "RTL locale should produce dir=\"rtl\": {html}"
    );
    assert!(
        html.contains(r#"id="child""#),
        "children should render inside the RTL wrapper: {html}"
    );
}

#[test]
fn ars_provider_explicit_direction_overrides_locale_inferred_direction() {
    fn app() -> Element {
        let locale = use_signal(|| Locale::parse("ar-SA").expect("locale should parse"));
        let direction = use_signal(|| Direction::Ltr);

        rsx! {
            ArsProvider { locale, direction,
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"dir="ltr""#),
        "explicit direction prop must override locale-inferred RTL: {html}"
    );
    assert!(
        !html.contains(r#"dir="rtl""#),
        "no rtl marker should remain after explicit override: {html}"
    );
}

#[test]
fn ars_provider_with_nonce_style_strategy_emits_style_tag_with_nonce_attribute() {
    fn app() -> Element {
        rsx! {
            ArsProvider { style_strategy: StyleStrategy::Nonce(String::from("test-nonce")),
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"<style nonce="test-nonce""#),
        "StyleStrategy::Nonce must auto-render <style nonce=...> tag: {html}"
    );
    assert!(
        html.contains(r#"id="child""#),
        "children must still render alongside the nonce style tag: {html}"
    );
}

#[test]
fn ars_provider_with_inline_style_strategy_emits_no_nonce_style_tag() {
    fn app() -> Element {
        rsx! {
            ArsProvider { style_strategy: StyleStrategy::Inline,
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("<style nonce"),
        "StyleStrategy::Inline must not emit a nonce style tag: {html}"
    );
}

#[test]
fn ars_provider_with_cssom_style_strategy_emits_no_nonce_style_tag() {
    fn app() -> Element {
        rsx! {
            ArsProvider { style_strategy: StyleStrategy::Cssom,
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("<style nonce"),
        "StyleStrategy::Cssom must not emit a nonce style tag: {html}"
    );
}

#[component]
fn PlatformProbe() -> Element {
    let context = try_use_context::<ArsContext>().expect("ArsProvider should publish ArsContext");

    let context_id = context.dioxus_platform.new_id();
    let direct_id = default_dioxus_platform().new_id();
    let context_id_len = context_id.len();
    let direct_id_len = direct_id.len();
    let context_id_hyphens = context_id.chars().filter(|c| *c == '-').count();
    let direct_id_hyphens = direct_id.chars().filter(|c| *c == '-').count();

    rsx! {
        span { "data-testid": "context-id", "{context_id}" }
        span { "data-testid": "direct-id", "{direct_id}" }
        span { "data-testid": "context-id-len", "{context_id_len}" }
        span { "data-testid": "direct-id-len", "{direct_id_len}" }
        span { "data-testid": "context-id-hyphens", "{context_id_hyphens}" }
        span { "data-testid": "direct-id-hyphens", "{direct_id_hyphens}" }
    }
}

#[test]
fn ars_provider_resolves_dioxus_platform_default_via_feature_flag_when_prop_absent() {
    fn app() -> Element {
        rsx! {
            ArsProvider { PlatformProbe {} }
        }
    }

    let html = render_app(app);

    // Spec §29 oracle: "`use_platform()` returns the feature-flag-appropriate
    // impl without explicit prop." Test is feature-flag-agnostic — the context
    // platform must produce IDs in the same family (same length, same hyphen
    // count) as a freshly-constructed `default_dioxus_platform()`. On native
    // SSR without `desktop`, both produce `null-id-N`. With `--all-features`
    // active, `desktop` engages and both produce v4 UUIDs.
    assert!(
        html.contains(r#"data-testid="context-id""#),
        "PlatformProbe should render inside the provider: {html}"
    );

    // Extract the rendered length values from the HTML; both must agree.
    // We just spot-check the equality fingerprint pair is present.
    let context_len = html
        .split(r#"data-testid="context-id-len">"#)
        .nth(1)
        .and_then(|tail| tail.split('<').next())
        .expect("context-id-len span should be present");

    let direct_len = html
        .split(r#"data-testid="direct-id-len">"#)
        .nth(1)
        .and_then(|tail| tail.split('<').next())
        .expect("direct-id-len span should be present");

    assert_eq!(
        context_len, direct_len,
        "context-resolved platform and direct default_dioxus_platform() \
         should produce IDs of the same length (proves same impl family): {html}"
    );

    let context_hyphens = html
        .split(r#"data-testid="context-id-hyphens">"#)
        .nth(1)
        .and_then(|tail| tail.split('<').next())
        .expect("context-id-hyphens span should be present");

    let direct_hyphens = html
        .split(r#"data-testid="direct-id-hyphens">"#)
        .nth(1)
        .and_then(|tail| tail.split('<').next())
        .expect("direct-id-hyphens span should be present");

    assert_eq!(
        context_hyphens, direct_hyphens,
        "context-resolved platform and direct default_dioxus_platform() \
         should produce IDs with the same hyphen count: {html}"
    );
}

#[test]
fn ars_provider_renders_color_mode_disabled_and_read_only_without_extra_wrapper_attrs() {
    fn app() -> Element {
        let color_mode = use_signal(|| ColorMode::Dark);
        let disabled = use_signal(|| true);
        let read_only = use_signal(|| true);

        rsx! {
            ArsProvider { color_mode, disabled, read_only,
                span { id: "child", "hello" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"dir="ltr""#),
        "wrapper still emits dir attribute regardless of color/disabled/read-only props: {html}"
    );
    assert!(
        !html.contains(r#"data-color-mode"#),
        "provider wrapper must not emit color-mode markup: {html}"
    );
    assert!(
        !html.contains(r#"aria-disabled"#),
        "provider wrapper must not emit aria-disabled: {html}"
    );
}

#[test]
fn ars_provider_props_default_is_eq_to_itself() {
    let lhs = ArsProviderProps {
        locale: None,
        direction: None,
        color_mode: None,
        disabled: None,
        read_only: None,
        id_prefix: None,
        portal_container_id: None,
        root_node_id: None,
        platform: None,
        intl_backend: None,
        i18n_registries: None,
        style_strategy: None,
        dioxus_platform: None,
        children: Ok(VNode::placeholder()),
    };

    let rhs = lhs.clone();

    assert_eq!(lhs, rhs, "ArsProviderProps PartialEq must be reflexive");
}
