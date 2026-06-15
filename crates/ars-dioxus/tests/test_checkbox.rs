//! Adapter-parity smoke tests for the Dioxus Checkbox adapter.

use ars_dioxus::input::checkbox;
use dioxus::prelude::*;

#[test]
fn checkbox_adapter_parity_smoke() {
    let mut vdom = VirtualDom::new(|| {
        rsx! {
            checkbox::Root { id: "dioxus-checkbox-parity",
                checkbox::Label { "Accept" }
                checkbox::Control { checkbox::Indicator {} }
                checkbox::HiddenInput {}
            }
        }
    });

    vdom.rebuild_in_place();

    let html = dioxus_ssr::render(&vdom);

    assert!(html.contains(r#"data-ars-scope="checkbox""#));
}
