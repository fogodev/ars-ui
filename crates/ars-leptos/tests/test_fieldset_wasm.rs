//! Browser coverage tests for the Leptos Fieldset adapter.

#![cfg(target_arch = "wasm32")]

use ars_leptos::utility::fieldset::{Content, Description, ErrorMessage, Fieldset, Legend};
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn container() -> web_sys::HtmlElement {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist");

    let element = document
        .create_element("div")
        .expect("container should be created");

    document
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
        .dyn_into::<web_sys::HtmlElement>()
        .expect("container should be an HtmlElement")
}

#[wasm_bindgen_test(async)]
async fn fieldset_browser_mounts_group_anatomy() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Fieldset id="wasm-billing" disabled=true>
                    <Legend>"Billing"</Legend>
                    <Description>"Billing details."</Description>
                    <Content>
                        <input name="postal-code" />
                    </Content>
                    <ErrorMessage>"Billing is incomplete."</ErrorMessage>
                </Fieldset>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let fieldset = parent
        .query_selector("#wasm-billing")
        .expect("query should succeed")
        .expect("fieldset should exist");

    assert_eq!(fieldset.get_attribute("disabled").as_deref(), Some(""));
    assert_eq!(
        fieldset.get_attribute("data-ars-scope").as_deref(),
        Some("fieldset")
    );

    let legend = parent
        .query_selector("#wasm-billing-legend")
        .expect("query should succeed")
        .expect("legend should exist");

    let description = parent
        .query_selector("#wasm-billing-description")
        .expect("query should succeed")
        .expect("description should exist");

    let content = parent
        .query_selector("[data-ars-part='content']")
        .expect("query should succeed")
        .expect("content should exist");

    let error = parent
        .query_selector("#wasm-billing-error-message")
        .expect("query should succeed")
        .expect("error message should exist");

    assert_eq!(
        legend.get_attribute("data-ars-part").as_deref(),
        Some("legend")
    );
    assert_eq!(
        description.get_attribute("data-ars-part").as_deref(),
        Some("description")
    );
    assert_eq!(
        content.get_attribute("data-ars-part").as_deref(),
        Some("content")
    );
    assert_eq!(
        error.get_attribute("data-ars-part").as_deref(),
        Some("error-message")
    );
    assert_eq!(error.get_attribute("role").as_deref(), Some("alert"));
    assert_eq!(error.get_attribute("hidden").as_deref(), Some(""));

    parent.remove();
}
