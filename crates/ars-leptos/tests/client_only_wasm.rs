//! Browser coverage tests for the Leptos `ClientOnly` adapter.

#![cfg(target_arch = "wasm32")]

use ars_leptos::utility::client_only::ClientOnly;
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist")
}

fn container() -> web_sys::HtmlElement {
    let element = document()
        .create_element("div")
        .expect("container should be created");

    document()
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
        .dyn_into::<web_sys::HtmlElement>()
        .expect("container should be an HtmlElement")
}

#[wasm_bindgen_test(async)]
async fn client_only_swaps_fallback_for_children_after_mount() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <ClientOnly fallback=|| view! { <span id="client-only-fallback">"Loading"</span> }>
                    <span id="client-only-child">"Client"</span>
                </ClientOnly>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    assert!(
        parent
            .query_selector("#client-only-child")
            .expect("query should succeed")
            .is_some(),
        "client child should render after mount"
    );
    assert!(
        parent
            .query_selector("#client-only-fallback")
            .expect("query should succeed")
            .is_none(),
        "fallback should be removed after mount"
    );

    parent.remove();
}
