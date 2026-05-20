//! SSR tests for the Dioxus `ClientOnly` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::utility::client_only::ClientOnly;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[test]
fn client_only_renders_fallback_during_ssr() {
    fn app() -> Element {
        rsx! {
            ClientOnly {
                fallback: rsx! {
                    span { id: "client-only-fallback", "Loading" }
                },
                span { id: "client-only-child", "Client" }
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"id="client-only-fallback""#),
        "fallback should render during SSR: {html}"
    );
    assert!(
        !html.contains(r#"id="client-only-child""#),
        "client children must not render during SSR: {html}"
    );
}

#[test]
fn client_only_without_fallback_renders_no_wrapper_during_ssr() {
    fn app() -> Element {
        rsx! {
            section { id: "before" }
            ClientOnly {
                span { id: "client-only-child", "Client" }
            }
            section { id: "after" }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"id="before""#) && html.contains(r#"id="after""#),
        "siblings should still render: {html}"
    );
    assert!(
        !html.contains(r#"id="client-only-child""#),
        "client children must not render during SSR: {html}"
    );
    assert!(
        !html.contains("client-only"),
        "adapter should not invent a wrapper or marker: {html}"
    );
}
