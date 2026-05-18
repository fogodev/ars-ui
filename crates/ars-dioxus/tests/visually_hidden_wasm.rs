//! Browser coverage tests for the Dioxus `VisuallyHidden` adapter.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

use ars_dioxus::{
    as_child::AsChildRenderProps,
    utility::visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn visually_hidden_browser_path_constructs_all_render_variants() {
    thread_local! {
        static RECEIVED_ATTRS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    RECEIVED_ATTRS.with(|attrs| attrs.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            VisuallyHidden { id: "vh-wasm-default", "Screen reader only" }
            VisuallyHidden { id: "vh-wasm-focusable", is_focusable: true, "Skip to content" }
            VisuallyHidden { "Unnamed screen reader text" }
            VisuallyHiddenAsChild {
                id: "vh-wasm-as-child",
                is_focusable: true,
                render: Callback::new(|slot: AsChildRenderProps| {
                    RECEIVED_ATTRS
                        .with(|attrs| {
                            attrs
                                .borrow_mut()
                                .extend(slot.attrs.iter().map(|attr| attr.name.to_string()));
                        });
                    rsx! {
                        a { href: "#main", ..slot.attrs, "Skip" }
                    }
                }),
            }
            VisuallyHiddenAsChild {
                id: "vh-wasm-classed",
                render: Callback::new(|slot: AsChildRenderProps| {
                    rsx! {
                        span { class: "skip-link", ..slot.attrs, "Classed hidden copy" }
                    }
                }),
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    let attrs = RECEIVED_ATTRS.with(|attrs| attrs.borrow().clone());

    assert!(attrs.iter().any(|name| name == "data-ars-scope"));
    assert!(attrs.iter().any(|name| name == "data-ars-part"));
    assert!(
        attrs
            .iter()
            .any(|name| name == "data-ars-visually-hidden-focusable")
    );
}
