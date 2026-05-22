//! Browser coverage tests for the Dioxus `Landmark` adapter.

#![cfg(target_arch = "wasm32")]

use ars_core::MessageFn;
use ars_dioxus::utility::landmark::{Landmark, Messages, Role};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn label(text: &'static str) -> Messages {
    Messages {
        label: MessageFn::static_str(text),
    }
}

#[wasm_bindgen_test]
fn landmark_browser_renders_every_native_role_without_panic() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "lm-banner",
                role: Role::Banner,
                messages: label("Banner"),
                "x"
            }
            Landmark {
                id: "lm-nav",
                role: Role::Navigation,
                messages: label("Primary"),
                "x"
            }
            Landmark { id: "lm-main", role: Role::Main, "x" }
            Landmark {
                id: "lm-aside",
                role: Role::Complementary,
                messages: label("Related"),
                "x"
            }
            Landmark { id: "lm-footer", role: Role::ContentInfo, "x" }
            Landmark { id: "lm-form", role: Role::Form, messages: label("Account"), "x" }
            Landmark {
                id: "lm-region",
                role: Role::Region,
                messages: label("Activity"),
                "x"
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn landmark_browser_search_fallback_div_renders_without_panic() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "lm-search",
                role: Role::Search,
                messages: label("Site search"),
                "input"
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn landmark_browser_labelledby_renders_without_panic() {
    fn app() -> Element {
        rsx! {
            Landmark {
                id: "lm-labelled",
                role: Role::Region,
                labelledby_id: "ext-label",
                messages: label("ignored"),
                "x"
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}
