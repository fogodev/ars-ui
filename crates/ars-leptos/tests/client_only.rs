//! SSR tests for the Leptos `ClientOnly` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::utility::client_only::ClientOnly;
use leptos::prelude::*;

#[test]
fn client_only_renders_fallback_during_ssr() {
    let html = view! {
        <ClientOnly fallback=|| view! { <span id="client-only-fallback">"Loading"</span> }>
            <span id="client-only-child">"Client"</span>
        </ClientOnly>
    }
    .to_html();

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
    let html = view! {
        <section id="before"></section>
        <ClientOnly>
            <span id="client-only-child">"Client"</span>
        </ClientOnly>
        <section id="after"></section>
    }
    .to_html();

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
