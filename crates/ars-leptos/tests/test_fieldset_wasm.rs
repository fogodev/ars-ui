//! Browser coverage tests for the Leptos Fieldset adapter.

#![cfg(target_arch = "wasm32")]

use ars_forms::validation::Error;
use ars_leptos::utility::{field, fieldset};
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn container() -> web_sys::HtmlElement {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist");

    let element = document
        .create_element("div")
        .expect("container should be created");

    document
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
        .dyn_into::<web_sys::HtmlElement>()
        .expect("container should be an HtmlElement")
}

#[wasm_bindgen_test(async)]
async fn fieldset_browser_mounts_group_anatomy() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <fieldset::Root id="wasm-billing" disabled=true>
                    <fieldset::Legend>"Billing"</fieldset::Legend>
                    <fieldset::Description>"Billing details."</fieldset::Description>
                    <fieldset::Content>
                        <input name="postal-code" />
                    </fieldset::Content>
                    <fieldset::ErrorMessage>"Billing is incomplete."</fieldset::ErrorMessage>
                </fieldset::Root>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let fieldset = parent
        .query_selector("#wasm-billing")
        .expect("query should succeed")
        .expect("fieldset should exist");

    assert_eq!(fieldset.get_attribute("disabled").as_deref(), Some(""));
    assert_eq!(
        fieldset.get_attribute("data-ars-scope").as_deref(),
        Some("fieldset")
    );
    assert_eq!(
        fieldset.get_attribute("aria-describedby").as_deref(),
        Some("wasm-billing-description"),
        "rendered fieldset descriptions must be associated with the fieldset"
    );

    let legend = parent
        .query_selector("#wasm-billing-legend")
        .expect("query should succeed")
        .expect("legend should exist");

    let description = parent
        .query_selector("#wasm-billing-description")
        .expect("query should succeed")
        .expect("description should exist");

    let content = parent
        .query_selector("[data-ars-part='content']")
        .expect("query should succeed")
        .expect("content should exist");

    let error = parent
        .query_selector("#wasm-billing-error-message")
        .expect("query should succeed")
        .expect("error message should exist");

    assert_eq!(
        legend.get_attribute("data-ars-part").as_deref(),
        Some("legend")
    );
    assert_eq!(
        description.get_attribute("data-ars-part").as_deref(),
        Some("description")
    );
    assert_eq!(
        content.get_attribute("data-ars-part").as_deref(),
        Some("content")
    );
    assert_eq!(
        error.get_attribute("data-ars-part").as_deref(),
        Some("error-message")
    );
    assert_eq!(error.get_attribute("role").as_deref(), Some("alert"));
    assert_eq!(error.get_attribute("hidden").as_deref(), Some(""));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn fieldset_state_reaches_descendant_field_input_attrs() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <fieldset::Root id="wasm-disabled-group" disabled=true invalid=true readonly=true>
                    <fieldset::Legend>"Account"</fieldset::Legend>
                    <fieldset::Content>
                        <field::Root id="wasm-grouped-email">
                            <field::Label>"Email"</field::Label>
                            <field::Input name="email" />
                        </field::Root>
                    </fieldset::Content>
                </fieldset::Root>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let input = parent
        .query_selector("#wasm-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_eq!(input.get_attribute("disabled").as_deref(), Some(""));
    assert_eq!(
        input.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("readonly").as_deref(), Some(""));
    assert_eq!(
        input.get_attribute("aria-readonly").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn fieldset_errors_reach_descendant_field_invalid_attrs() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <fieldset::Root
                    id="wasm-error-group"
                    errors=vec![Error::server("Account details are incomplete.")]
                >
                    <fieldset::Legend>"Account"</fieldset::Legend>
                    <fieldset::Content>
                        <field::Root id="wasm-error-grouped-email">
                            <field::Label>"Email"</field::Label>
                            <field::Input name="email" />
                        </field::Root>
                    </fieldset::Content>
                </fieldset::Root>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let input = parent
        .query_selector("#wasm-error-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn fieldset_state_updates_reach_descendant_fields_without_remount() {
    let owner = Owner::new();

    let (_mount_handle, parent, set_disabled, set_invalid, set_readonly) = owner.with(|| {
        let parent = container();
        let (disabled, set_disabled) = signal(false);
        let (invalid, set_invalid) = signal(false);
        let (readonly, set_readonly) = signal(false);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <fieldset::Root
                    id="wasm-reactive-group"
                    disabled=disabled
                    invalid=invalid
                    readonly=readonly
                >
                    <fieldset::Legend>"Account"</fieldset::Legend>
                    <fieldset::Content>
                        <field::Root id="wasm-reactive-grouped-email">
                            <field::Label>"Email"</field::Label>
                            <field::Input name="email" />
                        </field::Root>
                    </fieldset::Content>
                </fieldset::Root>
            }
        });

        (
            mount_handle,
            parent,
            set_disabled,
            set_invalid,
            set_readonly,
        )
    });

    leptos::task::tick().await;

    let fieldset = parent
        .query_selector("#wasm-reactive-group")
        .expect("query should succeed")
        .expect("fieldset should exist");

    let input = parent
        .query_selector("#wasm-reactive-grouped-email-input")
        .expect("query should succeed")
        .expect("grouped field input should exist");

    assert_eq!(fieldset.get_attribute("disabled"), None);
    assert_eq!(input.get_attribute("disabled"), None);
    assert_eq!(input.get_attribute("aria-invalid"), None);
    assert_eq!(input.get_attribute("readonly"), None);

    set_disabled.set(true);
    set_invalid.set(true);
    set_readonly.set(true);

    leptos::task::tick().await;

    assert_eq!(fieldset.get_attribute("disabled").as_deref(), Some(""));
    assert_eq!(input.get_attribute("disabled").as_deref(), Some(""));
    assert_eq!(
        input.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("readonly").as_deref(), Some(""));
    assert_eq!(
        input.get_attribute("aria-readonly").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}
