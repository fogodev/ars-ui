//! Browser coverage tests for the Leptos Form adapter.

#![cfg(target_arch = "wasm32")]

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use ars_forms::validation::Error;
use ars_leptos::utility::{
    field::{ErrorMessage, Field, Input, Label},
    form::Form,
};
use leptos::{mount::mount_to, prelude::*};
use wasm_bindgen::{JsCast, closure::Closure};
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

fn cancelable_event(event_type: &str) -> web_sys::Event {
    let init = web_sys::EventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    web_sys::Event::new_with_event_init_dict(event_type, &init)
        .expect("form event should construct")
}

#[wasm_bindgen_test(async)]
async fn form_browser_mounts_status_region() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Form id="wasm-account-form" action="/account">
                    <input name="email" />
                </Form>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-account-form")
        .expect("query should succeed")
        .expect("form should exist");

    let status = form
        .query_selector("[data-ars-part='status-region']")
        .expect("query should succeed")
        .expect("status region should exist");

    assert_eq!(form.get_attribute("novalidate").as_deref(), Some(""));
    assert_eq!(status.get_attribute("role").as_deref(), Some("status"));

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert!(
        submit.default_prevented(),
        "default ARIA validation behavior should prevent native navigation even without a callback"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_submit_and_reset_callbacks_fire_and_block_native_submit() {
    let owner = Owner::new();

    let (mount_handle, parent, log) = owner.with(|| {
        let parent = container();

        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let submit_log = Arc::clone(&log);
        let reset_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="wasm-account-form-callback"
                    on_submit=Callback::new(move |()| {
                        submit_log
                            .lock()
                            .expect("form log should not be poisoned")
                            .push(String::from("submit"));
                    })
                    on_reset=Callback::new(move |()| {
                        reset_log
                            .lock()
                            .expect("form log should not be poisoned")
                            .push(String::from("reset"));
                    })
                >
                    <input name="email" />
                </Form>
            }
        });

        (mount_handle, parent, log)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-account-form-callback")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert_eq!(
        form.get_attribute("data-ars-state").as_deref(),
        Some("idle"),
        "intercepted submit should complete the rendered form state after the callback"
    );
    assert_eq!(
        form.get_attribute("aria-busy"),
        None,
        "completed submit should clear busy state"
    );

    let reset = cancelable_event("reset");

    form.dispatch_event(&reset)
        .expect("reset event should dispatch");

    leptos::task::tick().await;

    assert!(
        submit.default_prevented(),
        "adapter form submit should prevent native navigation"
    );
    assert_eq!(
        log.lock()
            .expect("form log should not be poisoned")
            .as_slice(),
        &[String::from("submit"), String::from("reset")]
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_submit_button_click_fires_submit_callback_without_navigation() {
    let owner = Owner::new();

    let (mount_handle, parent, log) = owner.with(|| {
        let parent = container();

        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let submit_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="wasm-account-form-click-submit"
                    action="/account"
                    on_submit=Callback::new(move |()| {
                        submit_log
                            .lock()
                            .expect("form log should not be poisoned")
                            .push(String::from("submit"));
                    })
                >
                    <input name="email" value="admin@email.com" />
                    <button type="submit">"Submit"</button>
                </Form>
            }
        });

        (mount_handle, parent, log)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-account-form-click-submit")
        .expect("query should succeed")
        .expect("form should exist");

    let default_prevented = Arc::new(Mutex::new(false));
    let observed_default_prevented = Arc::clone(&default_prevented);

    let submit_observer =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            *observed_default_prevented
                .lock()
                .expect("submit observation should not be poisoned") = event.default_prevented();
        });

    form.add_event_listener_with_callback("submit", submit_observer.as_ref().unchecked_ref())
        .expect("submit observer should attach");

    let button = parent
        .query_selector("button[type='submit']")
        .expect("query should succeed")
        .expect("submit button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("submit button should be an HtmlElement");

    button.click();

    leptos::task::tick().await;

    assert_eq!(
        log.lock()
            .expect("form log should not be poisoned")
            .as_slice(),
        &[String::from("submit")]
    );
    assert!(
        *default_prevented
            .lock()
            .expect("submit observation should not be poisoned"),
        "submit button click should be default-prevented before browser navigation"
    );

    drop(submit_observer);
    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_validation_behavior_updates_without_remount() {
    let owner = Owner::new();

    let (_mount_handle, parent, set_behavior) = owner.with(|| {
        let parent = container();
        let (behavior, set_behavior) =
            signal(ars_leptos::utility::form::ValidationBehavior::Native);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="wasm-validation-behavior-form"
                    action="/account"
                    validation_behavior=behavior
                >
                    <input name="email" />
                </Form>
            }
        });

        (mount_handle, parent, set_behavior)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-validation-behavior-form")
        .expect("query should succeed")
        .expect("form should exist");

    assert_eq!(
        form.get_attribute("novalidate"),
        None,
        "native validation behavior should not render novalidate"
    );

    set_behavior.set(ars_leptos::utility::form::ValidationBehavior::Aria);

    leptos::task::tick().await;

    assert_eq!(
        form.get_attribute("novalidate").as_deref(),
        Some(""),
        "aria validation behavior should update root novalidate without remount"
    );

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert!(
        submit.default_prevented(),
        "submit prevention should use the latest validation behavior"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_default_aria_blocks_invalid_required_submit_callback() {
    let owner = Owner::new();

    let (mount_handle, parent, log) = owner.with(|| {
        let parent = container();

        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let submit_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="wasm-invalid-required-form"
                    on_submit=Callback::new(move |()| {
                        submit_log
                            .lock()
                            .expect("form log should not be poisoned")
                            .push(String::from("submit"));
                    })
                >
                    <input name="email" required />
                </Form>
            }
        });

        (mount_handle, parent, log)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-invalid-required-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert!(submit.default_prevented());
    assert!(
        log.lock()
            .expect("form log should not be poisoned")
            .is_empty(),
        "invalid ARIA submit should not emit the submit callback"
    );
    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("1 error found. Please correct the highlighted field.")
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_default_aria_counts_only_invalid_named_controls_in_groups() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form id="wasm-invalid-group-form">
                    <fieldset>
                        <input name="email" required />
                    </fieldset>
                </Form>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-invalid-group-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("1 error found. Please correct the highlighted field."),
        "group containers matching :invalid must not inflate the announced field count"
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn formnovalidate_submitter_skips_aria_constraint_validation() {
    let owner = Owner::new();

    let (mount_handle, parent, log) = owner.with(|| {
        let parent = container();

        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let submit_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="wasm-formnovalidate-form"
                    on_submit=Callback::new(move |()| {
                        submit_log
                            .lock()
                            .expect("form log should not be poisoned")
                            .push(String::from("submit"));
                    })
                >
                    <input name="email" required />
                    <button type="submit" formnovalidate=true>
                        "Submit without validation"
                    </button>
                </Form>
            }
        });

        (mount_handle, parent, log)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-formnovalidate-form")
        .expect("query should succeed")
        .expect("form should exist");

    let button = parent
        .query_selector("button[formnovalidate]")
        .expect("query should succeed")
        .expect("formnovalidate button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("formnovalidate button should be an HtmlElement");

    let default_prevented = Arc::new(Mutex::new(false));
    let observed_default_prevented = Arc::clone(&default_prevented);

    let submit_observer =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            *observed_default_prevented
                .lock()
                .expect("submit observation should not be poisoned") = event.default_prevented();
        });

    form.add_event_listener_with_callback("submit", submit_observer.as_ref().unchecked_ref())
        .expect("submit observer should attach");

    button.click();

    leptos::task::tick().await;

    assert_eq!(
        log.lock()
            .expect("form log should not be poisoned")
            .as_slice(),
        &[String::from("submit")],
        "formnovalidate submitter should bypass ARIA constraint blocking"
    );
    assert!(
        *default_prevented
            .lock()
            .expect("submit observation should not be poisoned"),
        "ARIA form should still prevent native navigation"
    );
    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("")
    );

    drop(submit_observer);
    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_invalid_required_submit_updates_named_field_errors() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Form id="wasm-invalid-field-form">
                    <Field id="wasm-invalid-email-field" name="email" required=true>
                        <Label>"Email"</Label>
                        <Input name="email" />
                        <ErrorMessage>"Email is required."</ErrorMessage>
                    </Field>
                </Form>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-invalid-field-form")
        .expect("query should succeed")
        .expect("form should exist");

    let input = parent
        .query_selector("#wasm-invalid-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(input.get_attribute("aria-invalid"), None);

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert!(submit.default_prevented());
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        input.get_attribute("aria-errormessage").as_deref(),
        Some("wasm-invalid-email-field-error-message")
    );
    assert_eq!(
        parent
            .query_selector("#wasm-invalid-email-field-error-message")
            .expect("query should succeed")
            .expect("field error should exist")
            .get_attribute("hidden"),
        None
    );

    input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .expect("field input should be an HtmlInputElement")
        .set_value("admin@email.com");

    let valid_submit = cancelable_event("submit");

    form.dispatch_event(&valid_submit)
        .expect("valid submit event should dispatch");

    leptos::task::tick().await;

    assert!(valid_submit.default_prevented());
    assert_eq!(
        input.get_attribute("aria-invalid"),
        None,
        "valid ARIA submit should clear native validation errors from descendant fields"
    );
    assert_eq!(
        input.get_attribute("aria-errormessage"),
        None,
        "valid ARIA submit should clear stale error relationships"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_valid_submit_preserves_controlled_validation_errors() {
    let owner = Owner::new();

    let (_mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Form
                    id="wasm-controlled-error-form"
                    validation_errors=BTreeMap::from([
                        (
                            "email".to_string(),
                            vec![Error::server("Server still rejects this email.")],
                        ),
                    ])
                >
                    <Field id="wasm-controlled-error-email-field" name="email" required=true>
                        <Label>"Email"</Label>
                        <Input name="email" value="admin@email.com" />
                        <ErrorMessage>"Server still rejects this email."</ErrorMessage>
                    </Field>
                </Form>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let form = parent
        .query_selector("#wasm-controlled-error-form")
        .expect("query should succeed")
        .expect("form should exist");

    let input = parent
        .query_selector("#wasm-controlled-error-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    leptos::task::tick().await;

    assert!(submit.default_prevented());
    assert_eq!(
        input.get_attribute("aria-invalid").as_deref(),
        Some("true"),
        "valid native submit must preserve controlled server errors"
    );
    assert_eq!(
        input.get_attribute("aria-errormessage").as_deref(),
        Some("wasm-controlled-error-email-field-error-message")
    );

    parent.remove();
}
