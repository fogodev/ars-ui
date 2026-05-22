//! SSR tests for the Dioxus `VisuallyHidden` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn visually_hidden_renders_default_span_with_hidden_attrs() {
    fn app() -> Element {
        rsx! {
            VisuallyHidden { id: "vh-default", "Screen reader only" }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            VisuallyHidden { id: "vh-focusable", is_focusable: true, "Skip to content" }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            VisuallyHidden { "Screen reader only" }
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("id="),
        "passive VisuallyHidden should not emit a generated id: {html}"
    );
}

#[test]
fn visually_hidden_as_child_forwards_attrs_without_wrapper() {
    fn app() -> Element {
        rsx! {
            VisuallyHiddenAsChild {
                id: "skip-link",
                is_focusable: true,
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    a { href: "#main", ..slot.attrs, "Skip to content" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<a"),
        "as-child should render the callback root directly: {html}"
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

#[test]
fn visually_hidden_as_child_merges_child_class_with_hidden_class() {
    fn app() -> Element {
        rsx! {
            VisuallyHiddenAsChild {
                id: "skip-copy",
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    span { class: "skip-link", ..slot.attrs, "Screen reader only" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<span"),
        "as-child should render the callback root directly: {html}"
    );
    assert_eq!(
        html.matches("class=").count(),
        1,
        "class attrs should merge instead of rendering duplicates: {html}"
    );

    for token in ["skip-link", "ars-visually-hidden"] {
        assert!(
            html.contains(token),
            "missing merged class token {token}: {html}"
        );
    }

    let cached_html = render_app(app);

    assert_eq!(
        cached_html.matches("class=").count(),
        1,
        "cached template class attrs should still merge: {cached_html}"
    );
}

#[test]
fn visually_hidden_as_child_merges_dynamic_child_class_with_hidden_class() {
    fn app() -> Element {
        let class_name = "skip-link";

        rsx! {
            VisuallyHiddenAsChild {
                id: "skip-copy",
                render: move |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    span { class: "{class_name}", ..slot.attrs, "Screen reader only" }
                },
            }
        }
    }

    let html = render_app(app);

    assert_eq!(
        html.matches("class=").count(),
        1,
        "dynamic class attrs should merge instead of rendering duplicates: {html}"
    );

    for token in ["skip-link", "ars-visually-hidden"] {
        assert!(
            html.contains(token),
            "missing merged class token {token}: {html}"
        );
    }
}
