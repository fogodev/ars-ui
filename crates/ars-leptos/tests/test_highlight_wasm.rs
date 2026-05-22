//! Browser coverage tests for the Leptos `Highlight` adapter.

#![cfg(target_arch = "wasm32")]

use ars_leptos::utility::highlight::{Highlight, MatchStrategy};
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist")
}

fn container() -> web_sys::HtmlElement {
    let element = document()
        .create_element("div")
        .expect("container should be created");

    document()
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
        .dyn_into::<web_sys::HtmlElement>()
        .expect("container should be an HtmlElement")
}

#[wasm_bindgen_test(async)]
async fn highlight_browser_wraps_matched_chunks_in_mark_elements() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! { <Highlight query=vec!["world".to_string()] text="Hello world!".to_string() /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let root = parent
        .query_selector("span[data-ars-scope='highlight']")
        .expect("query should succeed")
        .expect("highlight root should exist");

    assert_eq!(root.get_attribute("data-ars-part").as_deref(), Some("root"));
    assert_eq!(root.get_attribute("dir").as_deref(), Some("auto"));

    let mark = parent
        .query_selector("mark[data-ars-highlighted='true']")
        .expect("query should succeed")
        .expect("highlighted chunk should be a <mark>");

    assert_eq!(mark.text_content().as_deref(), Some("world"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn highlight_browser_unmatched_chunks_use_spans() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! { <Highlight query=vec!["world".to_string()] text="Hello world!".to_string() /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let unmatched = parent
        .query_selector("span[data-ars-highlighted='false']")
        .expect("query should succeed")
        .expect("unmatched chunk should exist as <span>");

    assert!(
        matches!(
            unmatched.text_content().as_deref(),
            Some("Hello ") | Some("!")
        ),
        "unexpected unmatched chunk text: {:?}",
        unmatched.text_content()
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn highlight_browser_starts_with_strategy_does_not_match_mid_string() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Highlight
                    query=vec!["world".to_string()]
                    text="Hello world!".to_string()
                    match_strategy=MatchStrategy::StartsWith
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let mark = parent.query_selector("mark").expect("query should succeed");

    assert!(
        mark.is_none(),
        "StartsWith match strategy must not produce a <mark> when query is mid-string"
    );

    parent.remove();
}
