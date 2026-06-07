//! Browser coverage tests for the Dioxus Fieldset adapter.

#![cfg(target_arch = "wasm32")]

use ars_dioxus::utility::{
    field::{Field, Input, Label},
    fieldset::{Content, Description, ErrorMessage, Fieldset, Legend},
};
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

async fn flush() {
    for _ in 0..2 {
        animation_frame_turn().await;
    }
}

fn assert_bool_attr(element: &web_sys::Element, name: &str) {
    assert!(
        matches!(
            element.get_attribute(name).as_deref(),
            Some("") | Some("true")
        ),
        "expected boolean attribute {name} to be present"
    );
}

#[wasm_bindgen_test(async)]
async fn fieldset_browser_renders_group_anatomy_attrs() {
    fn app() -> Element {
        rsx! {
            Fieldset { id: "wasm-billing", disabled: true,
                Legend { "Billing" }
                Description { "Billing details." }
                Content {
                    input { name: "postal-code" }
                }
                ErrorMessage { "Billing is incomplete." }
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

    let fieldset = parent
        .query_selector("#wasm-billing")
        .expect("query should succeed")
        .expect("fieldset should exist");

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

    assert_bool_attr(&fieldset, "disabled");
    assert_eq!(
        fieldset.get_attribute("data-ars-scope").as_deref(),
        Some("fieldset")
    );
    assert_eq!(
        fieldset.get_attribute("aria-describedby").as_deref(),
        Some("wasm-billing-description"),
        "rendered fieldset descriptions must be associated with the fieldset"
    );
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
    assert_bool_attr(&error, "hidden");

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn fieldset_state_reaches_descendant_field_input_attrs() {
    fn app() -> Element {
        rsx! {
            Fieldset {
                id: "wasm-disabled-group",
                disabled: true,
                invalid: true,
                readonly: true,
                Legend { "Account" }
                Content {
                    Field { id: "wasm-grouped-email",
                        Label { "Email" }
                        Input { name: "email" }
                    }
                }
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

    let input = parent
        .query_selector("#wasm-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_bool_attr(&input, "disabled");
    assert_eq!(
        input.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    assert_bool_attr(&input, "readonly");
    assert_eq!(
        input.get_attribute("aria-readonly").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}
