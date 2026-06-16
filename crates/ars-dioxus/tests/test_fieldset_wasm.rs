//! Browser coverage tests for the Dioxus Fieldset adapter.

#![cfg(target_arch = "wasm32")]

use ars_dioxus::utility::{field, fieldset};
use ars_forms::validation::Error;
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
            fieldset::Root { id: "wasm-billing", disabled: true,
                fieldset::Legend { "Billing" }
                fieldset::Description { "Billing details." }
                fieldset::Content {
                    input { name: "postal-code" }
                }
                fieldset::ErrorMessage { "Billing is incomplete." }
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
            fieldset::Root {
                id: "wasm-disabled-group",
                disabled: true,
                invalid: true,
                readonly: true,
                fieldset::Legend { "Account" }
                fieldset::Content {
                    field::Root { id: "wasm-grouped-email",
                        field::Label { "Email" }
                        field::Input { name: "email" }
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

#[wasm_bindgen_test(async)]
async fn fieldset_errors_reach_descendant_field_invalid_attrs() {
    fn app() -> Element {
        rsx! {
            fieldset::Root {
                id: "wasm-error-group",
                errors: vec![Error::server("Account details are incomplete.")],
                fieldset::Legend { "Account" }
                fieldset::Content {
                    field::Root { id: "wasm-error-grouped-email",
                        field::Label { "Email" }
                        field::Input { name: "email" }
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
        .query_selector("#wasm-error-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx! reports the event handler closure as an unused qualification on wasm."
)]
async fn fieldset_state_updates_reach_descendant_fields_without_remount() {
    fn app() -> Element {
        let mut disabled = use_signal(|| false);
        let mut invalid = use_signal(|| false);
        let mut readonly = use_signal(|| false);

        rsx! {
            fieldset::Root {
                id: "wasm-reactive-group",
                disabled: disabled(),
                invalid: invalid(),
                readonly: readonly(),
                fieldset::Legend { "Account" }
                fieldset::Content {
                    field::Root { id: "wasm-reactive-grouped-email",
                        field::Label { "Email" }
                        field::Input { name: "email" }
                    }
                }
                button {
                    r#type: "button",
                    onclick: move |_| {
                        disabled.set(true);
                        invalid.set(true);
                        readonly.set(true);
                    },
                    "Apply state"
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

    let fieldset = parent
        .query_selector("#wasm-reactive-group")
        .expect("query should succeed")
        .expect("fieldset should exist");

    let input = parent
        .query_selector("#wasm-reactive-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_eq!(fieldset.get_attribute("disabled"), None);
    assert_eq!(input.get_attribute("disabled"), None);
    assert_eq!(input.get_attribute("aria-invalid"), None);
    assert_eq!(input.get_attribute("readonly"), None);

    let button = parent
        .query_selector("button[type='button']")
        .expect("query should succeed")
        .expect("state button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("state button should be an HtmlElement");

    button.click();

    flush().await;

    assert_bool_attr(&fieldset, "disabled");
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
