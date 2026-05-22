//! Browser coverage tests for the Leptos `Landmark` adapter.

#![cfg(target_arch = "wasm32")]

use ars_core::MessageFn;
use ars_leptos::utility::landmark::{Landmark, Messages, Role};
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

fn label(text: &'static str) -> Messages {
    Messages {
        label: MessageFn::static_str(text),
    }
}

#[wasm_bindgen_test(async)]
async fn landmark_browser_renders_each_native_role_with_attrs() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Landmark id="lm-banner" role=Role::Banner messages=label("Site banner")>
                    "banner"
                </Landmark>
                <Landmark id="lm-nav" role=Role::Navigation messages=label("Primary")>
                    "nav"
                </Landmark>
                <Landmark id="lm-main" role=Role::Main>
                    "main"
                </Landmark>
                <Landmark id="lm-aside" role=Role::Complementary messages=label("Related")>
                    "aside"
                </Landmark>
                <Landmark id="lm-footer" role=Role::ContentInfo>
                    "footer"
                </Landmark>
                <Landmark id="lm-form" role=Role::Form messages=label("Account")>
                    "form"
                </Landmark>
                <Landmark id="lm-region" role=Role::Region messages=label("Activity")>
                    "region"
                </Landmark>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let cases = [
        ("lm-banner", "HEADER"),
        ("lm-nav", "NAV"),
        ("lm-main", "MAIN"),
        ("lm-aside", "ASIDE"),
        ("lm-footer", "FOOTER"),
        ("lm-form", "FORM"),
        ("lm-region", "SECTION"),
    ];

    for (id, expected_tag) in cases {
        let node = parent
            .query_selector(&format!("#{id}"))
            .expect("query should succeed")
            .unwrap_or_else(|| panic!("{id} root should exist"));

        assert_eq!(
            node.tag_name(),
            expected_tag,
            "expected {expected_tag} for {id}"
        );
        assert_eq!(
            node.get_attribute("data-ars-scope").as_deref(),
            Some("landmark")
        );
        // Native landmark elements must NOT emit an explicit role attribute.
        assert!(
            node.get_attribute("role").is_none(),
            "{id} should omit role attr"
        );
    }

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn landmark_browser_search_falls_back_to_div_with_explicit_role() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Landmark id="lm-search" role=Role::Search messages=label("Site search")>
                    "search"
                </Landmark>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let node = parent
        .query_selector("#lm-search")
        .expect("query should succeed")
        .expect("search Landmark root should exist");

    assert_eq!(node.tag_name(), "DIV");
    assert_eq!(node.get_attribute("role").as_deref(), Some("search"));
    assert_eq!(
        node.get_attribute("aria-label").as_deref(),
        Some("Site search")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn landmark_browser_labelledby_takes_precedence_over_aria_label() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <h2 id="ext-label">"External label"</h2>
                <Landmark
                    id="lm-labelled"
                    role=Role::Region
                    labelledby_id="ext-label"
                    messages=label("ignored")
                >
                    "content"
                </Landmark>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let node = parent
        .query_selector("#lm-labelled")
        .expect("query should succeed")
        .expect("labelled landmark root should exist");

    assert_eq!(
        node.get_attribute("aria-labelledby").as_deref(),
        Some("ext-label")
    );
    assert!(node.get_attribute("aria-label").is_none());

    parent.remove();
}
