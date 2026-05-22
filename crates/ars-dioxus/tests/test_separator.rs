//! SSR tests for the Dioxus Separator adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::{
    prelude::Orientation,
    utility::separator::{Separator, SeparatorAsChild},
};
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn separator_renders_horizontal_semantic_root() {
    fn app() -> Element {
        rsx! {
            Separator { id: "sep-horizontal" }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            Separator { id: "sep-vertical", orientation: Orientation::Vertical }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            Separator {}
        }
    }

    let html = render_app(app);

    assert!(
        !html.contains("id="),
        "passive Separator should not emit a generated id: {html}"
    );
}

#[test]
fn separator_as_child_forwards_attrs_without_wrapper() {
    fn app() -> Element {
        rsx! {
            SeparatorAsChild {
                id: "sep-menu",
                orientation: Orientation::Vertical,
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    div { class: "menu-separator", ..slot.attrs }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<div"),
        "as-child should render the callback root directly: {html}"
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
    fn app() -> Element {
        rsx! {
            Separator { id: "sep-decorative", decorative: true }
        }
    }

    let html = render_app(app);

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
