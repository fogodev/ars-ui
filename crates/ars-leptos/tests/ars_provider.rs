//! SSR oracle tests for the Leptos `ArsProvider` adapter.
//!
//! These tests cover spec §28 scenarios from
//! `spec/leptos-components/utility/ars-provider.md`:
//! `<div dir>` wrapper rendering, direction inference vs. explicit override,
//! and provider-owned auto-render of `ArsNonceStyle` when
//! `style_strategy` is `StyleStrategy::Nonce(...)`. The wasm DOM and
//! reactive-update oracles live in `tests/ars_provider_wasm.rs` and the
//! inline `wasm_tests` module in `src/provider.rs`.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_core::{ColorMode, StyleStrategy};
use ars_i18n::{Direction, Locale};
use ars_leptos::ArsProvider;
use leptos::prelude::*;

#[test]
fn ars_provider_renders_ltr_wrapper_for_default_props() {
    let html = view! {
        <ArsProvider>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

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
    let locale = Signal::stored(Locale::parse("ar-SA").expect("locale should parse"));

    let html = view! {
        <ArsProvider locale=locale>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

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
    let locale = Signal::stored(Locale::parse("ar-SA").expect("locale should parse"));
    let direction = Signal::stored(Direction::Ltr);

    let html = view! {
        <ArsProvider locale=locale direction=direction>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

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
    let html = view! {
        <ArsProvider style_strategy=StyleStrategy::Nonce(String::from("test-nonce"))>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

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
    let html = view! {
        <ArsProvider style_strategy=StyleStrategy::Inline>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

    assert!(
        !html.contains("<style nonce"),
        "StyleStrategy::Inline must not emit a nonce style tag: {html}"
    );
}

#[test]
fn ars_provider_with_cssom_style_strategy_emits_no_nonce_style_tag() {
    let html = view! {
        <ArsProvider style_strategy=StyleStrategy::Cssom>
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

    assert!(
        !html.contains("<style nonce"),
        "StyleStrategy::Cssom must not emit a nonce style tag: {html}"
    );
}

#[test]
fn ars_provider_renders_color_mode_disabled_and_read_only_without_extra_attributes_on_wrapper() {
    let html = view! {
        <ArsProvider
            color_mode=Signal::stored(ColorMode::Dark)
            disabled=Signal::stored(true)
            read_only=Signal::stored(true)
        >
            <span id="child">"hello"</span>
        </ArsProvider>
    }
    .to_html();

    assert!(
        html.contains(r#"dir="ltr""#),
        "wrapper still emits dir attribute regardless of color/disabled/read-only props: {html}"
    );
    assert!(
        !html.contains(r#"data-color-mode"#),
        "provider wrapper must not emit color-mode markup (consumer-owned): {html}"
    );
    assert!(
        !html.contains(r#"aria-disabled"#),
        "provider wrapper must not emit aria-disabled (descendants own that semantics): {html}"
    );
}
