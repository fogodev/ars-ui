//! Browser tests for the Dioxus Button adapter.

#![cfg(target_arch = "wasm32")]

use ars_components::utility::button;
use ars_dioxus::utility::button::{Button, ButtonAsChild};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn rebuild(app: fn() -> Element) {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn button_wasm_renders_default_button_path() {
    fn app() -> Element {
        rsx! {
            Button { id: "primary-action", variant: button::Variant::Primary, "Save" }
        }
    }

    rebuild(app);
}

#[wasm_bindgen_test]
fn button_wasm_renders_loading_path() {
    fn app() -> Element {
        rsx! {
            Button { id: "loading-action", loading: true, "Save" }
        }
    }

    rebuild(app);
}

#[wasm_bindgen_test]
fn button_wasm_renders_disabled_submit_path() {
    fn app() -> Element {
        rsx! {
            Button {
                id: "submit-action",
                disabled: true,
                r#type: button::Type::Submit,
                form: "account-form",
                name: "intent",
                value: "save",
                form_action: "/submit",
                form_method: button::FormMethod::Post,
                form_enc_type: button::FormEncType::MultipartFormData,
                form_target: button::FormTarget::Self_,
                form_no_validate: true,
                "Submit"
            }
        }
    }

    rebuild(app);
}

#[wasm_bindgen_test]
fn button_wasm_renders_custom_root_attrs_path() {
    fn app() -> Element {
        rsx! {
            Button {
                id: "customized-action",
                class: "app-button",
                style: "min-width: 12rem;",
                aria_label: "Save account",
                aria_labelledby: "save-label",
                "Save"
            }
        }
    }

    rebuild(app);
}

#[wasm_bindgen_test]
fn button_wasm_renders_as_child_path() {
    fn app() -> Element {
        rsx! {
            ButtonAsChild {
                id: "docs-link",
                variant: button::Variant::Link,
                render: |slot: ars_dioxus::as_child::AsChildRenderProps| rsx! {
                    a { href: "/docs", ..slot.attrs, "Docs" }
                },
            }
        }
    }

    rebuild(app);
}
