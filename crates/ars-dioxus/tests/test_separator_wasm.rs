//! Browser coverage tests for the Dioxus `Separator` adapter.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

use ars_dioxus::{
    as_child::AsChildRenderProps,
    prelude::Orientation,
    utility::separator::{Separator, SeparatorAsChild},
};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn separator_browser_path_constructs_all_render_variants() {
    thread_local! {
        static RECEIVED_ATTRS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    RECEIVED_ATTRS.with(|attrs| attrs.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Separator { id: "sep-wasm-horizontal" }
            Separator { id: "sep-wasm-vertical", orientation: Orientation::Vertical }
            Separator { id: "sep-wasm-decorative", decorative: true }
            Separator {}
            SeparatorAsChild {
                id: "sep-wasm-as-child",
                orientation: Orientation::Vertical,
                render: Callback::new(|slot: AsChildRenderProps| {
                    RECEIVED_ATTRS
                        .with(|attrs| {
                            attrs
                                .borrow_mut()
                                .extend(slot.attrs.iter().map(|attr| attr.name.to_string()));
                        });
                    rsx! {
                        div { class: "menu-separator", ..slot.attrs }
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
    assert!(attrs.iter().any(|name| name == "role"));
    assert!(attrs.iter().any(|name| name == "aria-orientation"));
}
