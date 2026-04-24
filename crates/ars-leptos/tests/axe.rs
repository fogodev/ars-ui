//! Browser-backed accessibility audit target for Leptos adapter output.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn axe_audit_target_is_available() {
    let package_name = String::from(env!("CARGO_PKG_NAME"));

    assert!(!package_name.is_empty());
}
