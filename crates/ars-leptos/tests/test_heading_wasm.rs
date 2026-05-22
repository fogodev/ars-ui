//! Browser coverage tests for the Leptos `Heading` adapter.

#![cfg(target_arch = "wasm32")]

use ars_leptos::utility::heading::{Heading, HeadingLevelProvider, Level, Section};
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
async fn heading_browser_resolves_explicit_levels_and_attrs() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Heading id="h-default">"Default"</Heading>
                <Heading id="h-three" level=Level::Three>
                    "Three"
                </Heading>
                <Heading id="h-six" level=Level::Six>
                    "Six"
                </Heading>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let default = parent
        .query_selector("#h-default")
        .expect("query should succeed")
        .expect("default Heading root should exist");

    assert_eq!(default.tag_name(), "H1");
    assert_eq!(
        default.get_attribute("data-ars-scope").as_deref(),
        Some("heading")
    );
    assert_eq!(
        default.get_attribute("data-ars-part").as_deref(),
        Some("root")
    );
    assert_eq!(default.get_attribute("role"), None);
    assert_eq!(default.get_attribute("aria-level"), None);

    let three = parent
        .query_selector("#h-three")
        .expect("query should succeed")
        .expect("Heading level=3 root should exist");

    assert_eq!(three.tag_name(), "H3");

    let six = parent
        .query_selector("#h-six")
        .expect("query should succeed")
        .expect("Heading level=6 root should exist");

    assert_eq!(six.tag_name(), "H6");

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn heading_browser_inherits_provider_and_section_context() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <HeadingLevelProvider level=Level::Two>
                    <Heading id="provided">"Two"</Heading>
                    <Section>
                        <Heading id="section-child">"Three"</Heading>
                    </Section>
                </HeadingLevelProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let provided = parent
        .query_selector("#provided")
        .expect("query should succeed")
        .expect("provided Heading root should exist");

    assert_eq!(provided.tag_name(), "H2");

    let section_child = parent
        .query_selector("#section-child")
        .expect("query should succeed")
        .expect("section-child Heading root should exist");

    assert_eq!(section_child.tag_name(), "H3");

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn heading_browser_default_sibling_of_provider_is_not_affected() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <section>
                    <Heading id="sibling-default">"Default"</Heading>
                    <Heading id="sibling-three" level=Level::Three>
                        "Three"
                    </Heading>
                    <HeadingLevelProvider level=Level::Two>
                        <Heading id="sibling-provided">"Provided"</Heading>
                        <Section>
                            <Heading id="sibling-section-child">"Section"</Heading>
                        </Section>
                    </HeadingLevelProvider>
                </section>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let default = parent
        .query_selector("#sibling-default")
        .expect("query should succeed")
        .expect("sibling-default Heading root should exist");

    let provided = parent
        .query_selector("#sibling-provided")
        .expect("query should succeed")
        .expect("sibling-provided Heading root should exist");

    let section_child = parent
        .query_selector("#sibling-section-child")
        .expect("query should succeed")
        .expect("sibling-section-child Heading root should exist");

    // The bug we hit in the E2E fixture: in Leptos CSR, the `default` Heading
    // (a sibling that appears BEFORE the provider/section in the source view)
    // unexpectedly inherits Section's published level. Document the actual
    // behavior so the fixture and harness work around it.
    assert_eq!(
        default.tag_name(),
        "H1",
        "sibling default Heading must render h1 regardless of later providers in CSR"
    );
    assert_eq!(provided.tag_name(), "H2");
    assert_eq!(section_child.tag_name(), "H3");

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn heading_browser_passive_root_has_no_id_attr() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! { <Heading>"No id"</Heading> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let heading = parent
        .query_selector("h1[data-ars-scope='heading']")
        .expect("query should succeed")
        .expect("passive Heading root should exist");

    assert_eq!(heading.get_attribute("id"), None);

    parent.remove();
}
