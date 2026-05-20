//! Browser coverage tests for the Dioxus `ClientOnly` adapter.

#![cfg(target_arch = "wasm32")]

use ars_dioxus::utility::client_only::ClientOnly;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist")
}

fn container() -> web_sys::Element {
    let element = document()
        .create_element("div")
        .expect("container should be created");

    document()
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
}

async fn animation_frame_turn() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let resolve = resolve.clone();
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
        });

        web_sys::window()
            .expect("window should exist")
            .request_animation_frame(callback.unchecked_ref())
            .expect("requestAnimationFrame should succeed");
    });

    drop(wasm_bindgen_futures::JsFuture::from(promise).await);
}

async fn microtask_turn() {
    drop(
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
            &wasm_bindgen::JsValue::UNDEFINED,
        ))
        .await,
    );
}

async fn flush() {
    for _ in 0..3 {
        animation_frame_turn().await;
        microtask_turn().await;
    }
}

#[wasm_bindgen_test(async)]
async fn client_only_swaps_fallback_for_children_after_effect_flush() {
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

    let parent = container();
    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    assert!(
        parent
            .query_selector("#client-only-child")
            .expect("query should succeed")
            .is_some(),
        "effect flush should render child"
    );
    assert!(
        parent
            .query_selector("#client-only-fallback")
            .expect("query should succeed")
            .is_none(),
        "effect flush should remove fallback"
    );

    parent.remove();
}
