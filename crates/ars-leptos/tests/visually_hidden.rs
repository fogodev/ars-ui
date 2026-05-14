//! SSR tests for the Leptos `VisuallyHidden` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::utility::visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild};
use leptos::prelude::*;

#[test]
fn visually_hidden_renders_default_span_with_hidden_attrs() {
    let html =
        view! { <VisuallyHidden id="vh-default">"Screen reader only"</VisuallyHidden> }.to_html();

    assert!(
        html.trim_start().starts_with("<span"),
        "default root should be a span: {html}"
    );

    for fragment in [
        r#"id="vh-default""#,
        r#"data-ars-scope="visually-hidden""#,
        r#"data-ars-part="root""#,
        r#"class="ars-visually-hidden""#,
        "Screen reader only",
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn visually_hidden_focusable_uses_focusable_data_hook() {
    let html = view! {
        <VisuallyHidden id="vh-focusable" is_focusable=true>
            "Skip to content"
        </VisuallyHidden>
    }
    .to_html();

    assert!(
        html.contains(r#"data-ars-visually-hidden-focusable"#),
        "missing focusable hook: {html}"
    );
    assert!(
        !html.contains(r#"class="ars-visually-hidden""#),
        "focusable variant must not include unconditional hidden class: {html}"
    );
}

#[test]
fn visually_hidden_without_id_does_not_emit_generated_id() {
    let html = view! { <VisuallyHidden>"Screen reader only"</VisuallyHidden> }.to_html();

    assert!(
        !html.contains("id="),
        "passive VisuallyHidden should not emit a generated id: {html}"
    );
}

#[test]
fn visually_hidden_as_child_forwards_attrs_without_wrapper() {
    let html = view! {
        <VisuallyHiddenAsChild id="skip-link" is_focusable=true>
            <a href="#main">"Skip to content"</a>
        </VisuallyHiddenAsChild>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<a"),
        "as-child should render the child root directly: {html}"
    );

    for fragment in [
        r#"id="skip-link""#,
        r##"href="#main""##,
        r#"data-ars-scope="visually-hidden""#,
        r#"data-ars-part="root""#,
        r#"data-ars-visually-hidden-focusable"#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }

    assert!(!html.contains("<span"), "unexpected wrapper span: {html}");
}
