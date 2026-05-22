//! Browser coverage tests for the Dioxus `Highlight` adapter.

#![cfg(target_arch = "wasm32")]

use ars_dioxus::utility::highlight::{Highlight, MatchStrategy};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn highlight_browser_renders_match_without_panic() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec!["world".to_string()], text: "Hello world!" }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn highlight_browser_empty_query_renders_without_panic() {
    fn app() -> Element {
        rsx! {
            Highlight { query: vec![], text: "Hello" }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn highlight_browser_starts_with_strategy_renders_without_panic() {
    fn app() -> Element {
        rsx! {
            Highlight {
                query: vec!["world".to_string()],
                text: "Hello world!",
                match_strategy: MatchStrategy::StartsWith,
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}
