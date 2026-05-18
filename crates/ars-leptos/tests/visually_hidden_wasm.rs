//! Browser coverage tests for the Leptos `VisuallyHidden` adapter.

#![cfg(target_arch = "wasm32")]

use ars_leptos::utility::visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild};
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
async fn visually_hidden_browser_attrs_cover_default_focusable_and_as_child() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <VisuallyHidden id="vh-wasm-default">"Screen reader only"</VisuallyHidden>
                <VisuallyHidden id="vh-wasm-focusable" is_focusable=true>
                    "Skip to content"
                </VisuallyHidden>
                <VisuallyHidden>"Unnamed screen reader text"</VisuallyHidden>
                <VisuallyHiddenAsChild id="vh-wasm-as-child" is_focusable=true>
                    <a href="#main">"Skip"</a>
                </VisuallyHiddenAsChild>
                <VisuallyHiddenAsChild id="vh-wasm-classed">
                    <span class="skip-link">"Classed hidden copy"</span>
                </VisuallyHiddenAsChild>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let default = parent
        .query_selector("#vh-wasm-default")
        .expect("query should succeed")
        .expect("default root should exist");

    assert_eq!(default.tag_name(), "SPAN");
    assert_eq!(
        default.get_attribute("data-ars-scope").as_deref(),
        Some("visually-hidden")
    );
    assert_eq!(
        default.get_attribute("data-ars-part").as_deref(),
        Some("root")
    );
    assert_eq!(
        default.get_attribute("class").as_deref(),
        Some("ars-visually-hidden")
    );

    let focusable = parent
        .query_selector("#vh-wasm-focusable")
        .expect("query should succeed")
        .expect("focusable root should exist");

    assert_eq!(
        focusable
            .get_attribute("data-ars-visually-hidden-focusable")
            .as_deref(),
        Some("")
    );
    assert_eq!(focusable.get_attribute("class"), None);

    let unnamed = parent
        .query_selector("span[data-ars-scope='visually-hidden']:not([id])")
        .expect("query should succeed")
        .expect("unnamed root should exist");

    assert_eq!(unnamed.get_attribute("id"), None);

    let as_child = parent
        .query_selector("#vh-wasm-as-child")
        .expect("query should succeed")
        .expect("as-child root should exist");

    assert_eq!(as_child.tag_name(), "A");
    assert_eq!(
        as_child.get_attribute("data-ars-scope").as_deref(),
        Some("visually-hidden")
    );
    assert_eq!(as_child.get_attribute("href").as_deref(), Some("#main"));

    let classed = parent
        .query_selector("#vh-wasm-classed")
        .expect("query should succeed")
        .expect("classed as-child root should exist");

    assert_eq!(classed.tag_name(), "SPAN");
    assert!(
        classed.class_list().contains("skip-link"),
        "consumer class should be preserved"
    );
    assert!(
        classed.class_list().contains("ars-visually-hidden"),
        "component hidden class should be preserved"
    );

    parent.remove();
}
