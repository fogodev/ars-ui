//! Browser coverage tests for the Leptos `Separator` adapter.

#![cfg(target_arch = "wasm32")]

use ars_i18n::Orientation;
use ars_leptos::utility::separator::{Separator, SeparatorAsChild};
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
async fn separator_browser_attrs_cover_semantic_decorative_and_as_child() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Separator id="sep-wasm-horizontal" />
                <Separator id="sep-wasm-vertical" orientation=Orientation::Vertical />
                <Separator id="sep-wasm-decorative" decorative=true />
                <Separator />
                <SeparatorAsChild id="sep-wasm-as-child" orientation=Orientation::Vertical>
                    <div class="menu-separator"></div>
                </SeparatorAsChild>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let horizontal = parent
        .query_selector("#sep-wasm-horizontal")
        .expect("query should succeed")
        .expect("horizontal root should exist");

    assert_eq!(horizontal.tag_name(), "HR");
    assert_eq!(
        horizontal.get_attribute("data-ars-scope").as_deref(),
        Some("separator")
    );
    assert_eq!(
        horizontal.get_attribute("role").as_deref(),
        Some("separator")
    );
    assert_eq!(
        horizontal.get_attribute("aria-orientation").as_deref(),
        Some("horizontal")
    );

    let vertical = parent
        .query_selector("#sep-wasm-vertical")
        .expect("query should succeed")
        .expect("vertical root should exist");

    assert_eq!(
        vertical.get_attribute("aria-orientation").as_deref(),
        Some("vertical")
    );
    assert_eq!(
        vertical.get_attribute("data-ars-orientation").as_deref(),
        Some("vertical")
    );

    let decorative = parent
        .query_selector("#sep-wasm-decorative")
        .expect("query should succeed")
        .expect("decorative root should exist");

    assert_eq!(decorative.get_attribute("role").as_deref(), Some("none"));
    assert_eq!(decorative.get_attribute("aria-orientation"), None);
    assert_eq!(decorative.get_attribute("data-ars-orientation"), None);

    let unnamed = parent
        .query_selector("hr[data-ars-scope='separator']:not([id])")
        .expect("query should succeed")
        .expect("unnamed root should exist");

    assert_eq!(unnamed.get_attribute("id"), None);

    let as_child = parent
        .query_selector("#sep-wasm-as-child")
        .expect("query should succeed")
        .expect("as-child root should exist");

    assert_eq!(as_child.tag_name(), "DIV");
    assert_eq!(
        as_child.get_attribute("class").as_deref(),
        Some("menu-separator")
    );
    assert_eq!(
        as_child.get_attribute("aria-orientation").as_deref(),
        Some("vertical")
    );

    parent.remove();
}
