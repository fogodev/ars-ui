//! Adapter-parity smoke tests for the Leptos Checkbox adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::input::checkbox;
use leptos::{prelude::*, reactive::owner::Owner};

#[test]
fn checkbox_adapter_parity_smoke() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <checkbox::Root id="leptos-checkbox-parity">
                <checkbox::Label>"Accept"</checkbox::Label>
                <checkbox::Control>
                    <checkbox::Indicator />
                </checkbox::Control>
                <checkbox::HiddenInput />
            </checkbox::Root>
        }
        .to_html()
    });

    drop(owner);

    assert!(html.contains(r#"data-ars-scope="checkbox""#));
}
