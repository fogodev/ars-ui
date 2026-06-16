//! Browser coverage tests for the Leptos Field adapter.

#![cfg(target_arch = "wasm32")]

use std::sync::{Arc, Mutex};

use ars_forms::validation::Error;
use ars_leptos::utility::field;
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

fn dispatch_input(target: &web_sys::Element, value: &str) {
    js_sys::Reflect::set(
        target,
        &wasm_bindgen::JsValue::from_str("value"),
        &wasm_bindgen::JsValue::from_str(value),
    )
    .expect("input value property should be set");

    let event = web_sys::Event::new("input").expect("input event should construct");

    target
        .dispatch_event(&event)
        .expect("input event should dispatch");
}

#[wasm_bindgen_test(async)]
async fn field_browser_mounts_input_with_label_relationship() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <field::Root id="wasm-email-field" required=true invalid=true>
                    <field::Label>"Email"</field::Label>
                    <field::Input r#type=field::InputType::Email name="email" />
                    <field::Description>"Use a reachable email."</field::Description>
                    <field::ErrorMessage>"Email is required."</field::ErrorMessage>
                </field::Root>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let input = parent
        .query_selector("#wasm-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(
        input.get_attribute("aria-labelledby").as_deref(),
        Some("wasm-email-field-label")
    );
    assert_eq!(
        input.get_attribute("aria-describedby").as_deref(),
        Some("wasm-email-field-description")
    );
    assert_eq!(
        input.get_attribute("aria-required").as_deref(),
        Some("true")
    );
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn field_input_emits_value_input_callback() {
    let owner = Owner::new();

    let (mount_handle, parent, values) = owner.with(|| {
        let parent = container();

        let values = Arc::new(Mutex::new(Vec::<String>::new()));
        let callback_values = Arc::clone(&values);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <field::Root id="wasm-email-field-callback">
                    <field::Label>"Email"</field::Label>
                    <field::Input
                        r#type=field::InputType::Email
                        name="email"
                        on_value_input=Callback::new(move |value| {
                            callback_values
                                .lock()
                                .expect("value log should not be poisoned")
                                .push(value);
                        })
                    />
                </field::Root>
            }
        });

        (mount_handle, parent, values)
    });

    leptos::task::tick().await;

    let input = parent
        .query_selector("#wasm-email-field-callback-input")
        .expect("query should succeed")
        .expect("field input should exist");

    dispatch_input(&input, "ericson@example.com");

    leptos::task::tick().await;

    assert_eq!(
        values
            .lock()
            .expect("value log should not be poisoned")
            .as_slice(),
        &[String::from("ericson@example.com")]
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn field_reactive_errors_update_invalid_relationship() {
    let owner = Owner::new();

    let (mount_handle, parent, email) = owner.with(|| {
        let parent = container();

        let email = RwSignal::new(String::new());
        let errors = Signal::derive(move || {
            if email.with(String::is_empty) {
                vec![Error::server("Email is required.")]
            } else {
                Vec::new()
            }
        });
        let invalid = Signal::derive(move || !errors.with(Vec::is_empty));

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <field::Root
                    id="wasm-reactive-email-field"
                    required=true
                    invalid=invalid
                    errors=errors
                >
                    <field::Label>"Email"</field::Label>
                    <field::Description>"Use a reachable email."</field::Description>
                    <field::Input
                        r#type=field::InputType::Email
                        name="email"
                        value=email
                        on_value_input=Callback::new(move |value| email.set(value))
                    />
                    <field::ErrorMessage>"Email is required."</field::ErrorMessage>
                </field::Root>
            }
        });

        (mount_handle, parent, email)
    });

    leptos::task::tick().await;

    let root = parent
        .query_selector("#wasm-reactive-email-field")
        .expect("query should succeed")
        .expect("field root should exist");
    let input = parent
        .query_selector("#wasm-reactive-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(root.get_attribute("data-ars-invalid").as_deref(), Some(""));
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        input.get_attribute("aria-errormessage").as_deref(),
        Some("wasm-reactive-email-field-error-message")
    );

    let error_message = parent
        .query_selector("#wasm-reactive-email-field-error-message")
        .expect("query should succeed")
        .expect("field error message should exist");

    assert_eq!(
        error_message.get_attribute("hidden"),
        None,
        "invalid email should reveal the field error message"
    );

    email.set(String::from("admin@email.com"));

    leptos::task::tick().await;

    assert_eq!(
        root.get_attribute("data-ars-invalid"),
        None,
        "valid email should clear stale root invalid state"
    );
    assert_eq!(
        input.get_attribute("aria-invalid"),
        None,
        "valid email should clear stale invalid state"
    );
    assert_eq!(
        input.get_attribute("aria-errormessage"),
        None,
        "valid email should clear stale error relationship"
    );
    assert_eq!(
        input.get_attribute("aria-describedby").as_deref(),
        Some("wasm-reactive-email-field-description"),
        "valid email should keep only the description relationship"
    );
    assert_eq!(
        error_message.get_attribute("hidden").as_deref(),
        Some(""),
        "valid email should hide the field error message"
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn field_controlled_value_updates_live_input_property() {
    let owner = Owner::new();

    let (mount_handle, parent, email) = owner.with(|| {
        let parent = container();

        let email = RwSignal::new(String::from("initial@example.com"));

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <field::Root id="wasm-controlled-email-field">
                    <field::Label>"Email"</field::Label>
                    <field::Input r#type=field::InputType::Email name="email" value=email />
                </field::Root>
            }
        });

        (mount_handle, parent, email)
    });

    leptos::task::tick().await;

    let input = parent
        .query_selector("#wasm-controlled-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("field input should be an HtmlInputElement");

    assert_eq!(input.value(), "initial@example.com");

    input.set_value("typed@example.com");
    email.set(String::from("normalized@example.com"));

    leptos::task::tick().await;

    assert_eq!(
        input.value(),
        "normalized@example.com",
        "controlled value changes must update the live input property"
    );

    drop(mount_handle);

    parent.remove();
}
