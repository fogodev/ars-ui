//! SSR tests for the Leptos `as_child` adapter slot.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_components::utility::as_child::AsChildMerge;
use ars_core::{AriaAttr, AttrMap, CssProperty, HtmlAttr, StyleStrategy};
use ars_leptos::{
    as_child::{AsChildAttrs, AsChildSlot},
    attr_map_to_leptos, attr_map_to_leptos_inline_attrs,
};
use leptos::prelude::*;

fn component_attrs() -> AttrMap {
    let mut attrs = AttrMap::new();

    attrs
        .set(HtmlAttr::Id, "trigger")
        .set(HtmlAttr::Role, "button")
        .set(HtmlAttr::Data("ars-scope"), "as-child-test")
        .set(HtmlAttr::Data("ars-part"), "trigger")
        .set(HtmlAttr::Aria(AriaAttr::Expanded), "false");

    attrs
}

fn merged_attrs() -> AttrMap {
    let mut child_attrs = AttrMap::new();

    child_attrs
        .set(HtmlAttr::Class, "child")
        .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "child-hint")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "child-label")
        .set_style(CssProperty::Color, "red");

    let mut component_attrs = component_attrs();

    component_attrs
        .set(HtmlAttr::Class, "component")
        .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "component-hint")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label")
        .set_style(CssProperty::Display, "inline-flex");

    component_attrs.merge_onto(child_attrs)
}

fn native_leptos_attr(name: &'static str, value: &'static str) -> ars_leptos::LeptosAttribute {
    leptos::attr::custom::custom_attribute(name, value).into_any_attr()
}

#[test]
fn as_child_slot_applies_attrs_to_typed_root() {
    let attrs = attr_map_to_leptos_inline_attrs(component_attrs());

    let html = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    }
    .to_html();

    assert!(html.contains("<button"), "expected button root: {html}");
    assert!(html.contains(r#"id="trigger""#), "missing id: {html}");
    assert!(html.contains(r#"role="button""#), "missing role: {html}");
    assert!(
        html.contains(r#"data-ars-scope="as-child-test""#),
        "missing scope: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="trigger""#),
        "missing part: {html}"
    );
    assert!(
        html.contains(r#"aria-expanded="false""#),
        "missing aria-expanded: {html}"
    );
}

#[test]
fn as_child_slot_accepts_native_attrs_without_attr_map_conversion() {
    let attrs = vec![
        native_leptos_attr("data-direct", "yes"),
        native_leptos_attr("aria-label", "Native label"),
    ];

    let html = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    }
    .to_html();

    assert!(
        html.contains(r#"data-direct="yes""#),
        "missing direct data attr: {html}"
    );
    assert!(
        html.contains(r#"aria-label="Native label""#),
        "missing direct aria attr: {html}"
    );
}

#[test]
fn as_child_slot_does_not_render_wrapper() {
    let attrs = attr_map_to_leptos_inline_attrs(component_attrs());

    let html = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<button"),
        "slot should render the child root directly: {html}"
    );
    assert!(!html.contains("<div"), "unexpected wrapper div: {html}");
}

#[test]
fn as_child_slot_preserves_merged_class_style_and_aria_tokens() {
    let attrs = attr_map_to_leptos_inline_attrs(merged_attrs());

    let html = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    }
    .to_html();

    assert!(html.contains(r#"class=""#), "missing class attr: {html}");
    assert!(html.contains("child"), "missing child class token: {html}");
    assert!(
        html.contains("component"),
        "missing component class token: {html}"
    );
    assert!(html.contains("color: red;"), "missing color style: {html}");
    assert!(
        html.contains("display: inline-flex;"),
        "missing display style: {html}"
    );
    assert!(
        html.contains(r#"aria-describedby="child-hint component-hint""#),
        "missing merged aria-describedby: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="child-label component-label""#),
        "missing merged aria-labelledby: {html}"
    );
}

#[test]
fn as_child_attrs_merges_child_attrs_before_conversion() {
    let mut component_attrs = AttrMap::new();

    component_attrs
        .set(HtmlAttr::Class, "component")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label");

    let mut child_attrs = AttrMap::new();

    child_attrs
        .set(HtmlAttr::Class, "child")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "child-label");

    let attrs = AsChildAttrs::from_merged_attr_maps(component_attrs, child_attrs);

    let html = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    }
    .to_html();

    assert!(
        html.contains(r#"class="child component""#),
        "child and component classes should merge before conversion: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="child-label component-label""#),
        "child and component aria-labelledby values should merge before conversion: {html}"
    );
}

#[test]
fn as_child_slot_ssr_markup_is_hydration_stable() {
    let attrs = attr_map_to_leptos_inline_attrs(component_attrs());

    let html = view! {
        <AsChildSlot attrs=attrs>
            <a href="/docs">"Docs"</a>
        </AsChildSlot>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<a"),
        "expected anchor as SSR root: {html}"
    );
    assert!(html.contains(r#"href="/docs""#), "missing href: {html}");
    assert!(
        html.contains(r#"data-ars-part="trigger""#),
        "missing forwarded part: {html}"
    );
    assert!(
        !html.contains("as-child-slot"),
        "slot should not add adapter marker markup: {html}"
    );
}

#[test]
fn attr_map_to_leptos_preserves_as_child_style_strategy_payloads() {
    let mut attrs = component_attrs();

    attrs.set_style(CssProperty::Color, "red");

    let cssom = attr_map_to_leptos(attrs.clone(), &StyleStrategy::Cssom, None);

    assert!(
        cssom
            .attrs
            .iter()
            .all(|attr| !format!("{attr:?}").contains("style")),
        "CSSOM strategy should not inline style attrs: {:?}",
        cssom.attrs
    );
    assert_eq!(
        cssom.cssom_styles,
        vec![(CssProperty::Color, String::from("red"))]
    );

    let nonce = attr_map_to_leptos(
        attrs,
        &StyleStrategy::Nonce(String::from("nonce")),
        Some("trigger"),
    );

    assert_eq!(nonce.nonce_css_key.as_deref(), Some("trigger"));
    assert!(
        nonce.nonce_css.contains("color: red;"),
        "missing nonce CSS declaration: {}",
        nonce.nonce_css
    );
}
