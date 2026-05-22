//! SSR tests for the Leptos Separator adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_i18n::Orientation;
use ars_leptos::utility::separator::{Separator, SeparatorAsChild};
use leptos::prelude::*;

#[test]
fn separator_renders_horizontal_semantic_root() {
    let html = view! { <Separator id="sep-horizontal" /> }.to_html();

    assert!(
        html.trim_start().starts_with("<hr"),
        "default separator should render an hr root: {html}"
    );

    for fragment in [
        r#"id="sep-horizontal""#,
        r#"data-ars-scope="separator""#,
        r#"data-ars-part="root""#,
        r#"role="separator""#,
        r#"aria-orientation="horizontal""#,
        r#"data-ars-orientation="horizontal""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn separator_renders_vertical_orientation() {
    let html =
        view! { <Separator id="sep-vertical" orientation=Orientation::Vertical /> }.to_html();

    for fragment in [
        r#"id="sep-vertical""#,
        r#"role="separator""#,
        r#"aria-orientation="vertical""#,
        r#"data-ars-orientation="vertical""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn separator_without_id_does_not_emit_generated_id() {
    let html = view! { <Separator /> }.to_html();

    assert!(
        !html.contains("id="),
        "passive Separator should not emit a generated id: {html}"
    );
}

#[test]
fn separator_as_child_forwards_attrs_without_wrapper() {
    let html = view! {
        <SeparatorAsChild id="sep-menu" orientation=Orientation::Vertical>
            <div class="menu-separator"></div>
        </SeparatorAsChild>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<div"),
        "as-child should render the child root directly: {html}"
    );

    for fragment in [
        r#"id="sep-menu""#,
        r#"class="menu-separator""#,
        r#"data-ars-scope="separator""#,
        r#"data-ars-part="root""#,
        r#"role="separator""#,
        r#"aria-orientation="vertical""#,
        r#"data-ars-orientation="vertical""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(!html.contains("<hr"), "unexpected wrapper hr: {html}");
}

#[test]
fn separator_renders_decorative_role_without_orientation_attrs() {
    let html = view! { <Separator id="sep-decorative" decorative=true /> }.to_html();

    for fragment in [
        r#"id="sep-decorative""#,
        r#"data-ars-scope="separator""#,
        r#"data-ars-part="root""#,
        r#"role="none""#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(
        !html.contains("aria-orientation"),
        "decorative separator should omit aria-orientation: {html}"
    );
    assert!(
        !html.contains("data-ars-orientation"),
        "decorative separator should omit data-ars-orientation: {html}"
    );
}
