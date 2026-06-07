//! Browser coverage tests for the Dioxus Form adapter.

#![cfg(target_arch = "wasm32")]

use std::{cell::RefCell, collections::BTreeMap};

use ars_dioxus::utility::{
    field::{ErrorMessage, Field, Input, Label},
    form::Form,
};
use ars_forms::validation::Error;
use dioxus::{dioxus_core::AttributeValue, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static FORM_EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|window| window.document())
        .expect("document should exist")
}

fn container() -> web_sys::Element {
    let element = document()
        .create_element("div")
        .expect("container should be created");

    document()
        .body()
        .expect("body should exist")
        .append_child(&element)
        .expect("container should be attached");

    element
}

async fn animation_frame_turn() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let resolve = resolve.clone();
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
        });

        web_sys::window()
            .expect("window should exist")
            .request_animation_frame(callback.unchecked_ref())
            .expect("requestAnimationFrame should succeed");
    });

    drop(wasm_bindgen_futures::JsFuture::from(promise).await);
}

async fn flush() {
    for _ in 0..2 {
        animation_frame_turn().await;
    }
}

fn cancelable_event(event_type: &str) -> web_sys::Event {
    let init = web_sys::EventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    web_sys::Event::new_with_event_init_dict(event_type, &init)
        .expect("form event should construct")
}

fn assert_bool_attr(element: &web_sys::Element, name: &str) {
    assert!(
        matches!(
            element.get_attribute(name).as_deref(),
            Some("") | Some("true")
        ),
        "expected boolean attribute {name} to be present"
    );
}

#[wasm_bindgen_test(async)]
async fn form_browser_renders_status_region_and_dispatches_callbacks() {
    FORM_EVENTS.with(|events| events.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Form {
                id: "wasm-account-form",
                action: "/account",
                on_submit: move |()| {
                    FORM_EVENTS.with(|events| events.borrow_mut().push("submit"));
                },
                on_reset: move |()| {
                    FORM_EVENTS.with(|events| events.borrow_mut().push("reset"));
                },
                input { name: "email" }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-account-form")
        .expect("query should succeed")
        .expect("form should exist");

    let status = form
        .query_selector("[data-ars-part='status-region']")
        .expect("query should succeed")
        .expect("status region should exist");

    assert_eq!(form.get_attribute("action").as_deref(), Some("/account"));
    assert_bool_attr(&form, "novalidate");
    assert_eq!(status.get_attribute("role").as_deref(), Some("status"));
    assert_eq!(status.get_attribute("aria-live").as_deref(), Some("polite"));
    assert_eq!(status.get_attribute("aria-atomic").as_deref(), Some("true"));

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

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

    flush().await;

    assert!(submit.default_prevented());

    FORM_EVENTS.with(|events| {
        assert_eq!(events.borrow().as_slice(), &["submit", "reset"]);
    });

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_default_aria_submit_prevents_native_navigation_without_callback() {
    fn app() -> Element {
        rsx! {
            Form { id: "wasm-default-aria-form", action: "/account",
                input { name: "email" }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-default-aria-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

    assert!(
        submit.default_prevented(),
        "default ARIA validation behavior should prevent native navigation even without a callback"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx! reports the event handler closure as an unused qualification on wasm."
)]
async fn form_validation_errors_update_existing_descendant_field() {
    fn app() -> Element {
        let mut errors = use_signal(BTreeMap::<String, Vec<Error>>::new);

        rsx! {
            Form { id: "wasm-validation-form", validation_errors: errors(),
                Field { id: "wasm-validation-email", name: "email",
                    Label { "Email" }
                    Input { name: "email" }
                    ErrorMessage { "Email is required." }
                }
                button {
                    r#type: "button",
                    onclick: move |_| {
                        let mut next = BTreeMap::new();
                        next.insert(String::from("email"), vec![Error::server("Email is required.")]);
                        errors.set(next);
                    },
                    "Invalidate"
                }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let input = parent
        .query_selector("#wasm-validation-email-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(input.get_attribute("aria-invalid"), None);

    let button = parent
        .query_selector("button[type='button']")
        .expect("query should succeed")
        .expect("invalidate button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("invalidate button should be an HtmlElement");

    button.click();

    flush().await;

    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        input.get_attribute("aria-errormessage").as_deref(),
        Some("wasm-validation-email-error-message")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx! reports the event handler closure as an unused qualification on wasm."
)]
async fn form_root_action_and_role_follow_prop_rerenders() {
    fn app() -> Element {
        let mut alternate = use_signal(|| false);
        let action = if alternate() { "/second" } else { "/first" };
        let role = if alternate() { "search" } else { "form" };

        rsx! {
            Form { id: "wasm-rerender-form", action, role,
                input { name: "email" }
                button { r#type: "button", onclick: move |_| alternate.set(true), "Change" }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-rerender-form")
        .expect("query should succeed")
        .expect("form should exist");

    assert_eq!(form.get_attribute("action").as_deref(), Some("/first"));
    assert_eq!(form.get_attribute("role").as_deref(), Some("form"));

    let button = parent
        .query_selector("button[type='button']")
        .expect("query should succeed")
        .expect("change button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("change button should be an HtmlElement");

    button.click();

    flush().await;

    assert_eq!(form.get_attribute("action").as_deref(), Some("/second"));
    assert_eq!(form.get_attribute("role").as_deref(), Some("search"));

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_default_aria_blocks_invalid_required_submit_callback() {
    FORM_EVENTS.with(|events| events.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Form {
                id: "wasm-invalid-required-form",
                on_submit: move |()| {
                    FORM_EVENTS.with(|events| events.borrow_mut().push("submit"));
                },
                input { name: "email", required: true }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-invalid-required-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

    assert!(submit.default_prevented());
    FORM_EVENTS.with(|events| {
        assert!(
            events.borrow().is_empty(),
            "invalid ARIA submit should not emit the submit callback"
        );
    });
    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("1 error found. Please correct the highlighted field.")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_default_aria_counts_only_invalid_named_controls_in_groups() {
    fn app() -> Element {
        rsx! {
            Form { id: "wasm-invalid-group-form",
                fieldset {
                    input { name: "email", required: true }
                }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-invalid-group-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("1 error found. Please correct the highlighted field."),
        "group containers matching :invalid must not inflate the announced field count"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_global_onsubmit_does_not_replace_adapter_handler() {
    FORM_EVENTS.with(|events| events.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Form {
                id: "wasm-global-submit-form",
                attrs: vec![
                    Attribute::new(
                        "onsubmit",
                        AttributeValue::listener(move |event: Event<FormData>| {
                            event.prevent_default();
                            FORM_EVENTS.with(|events| events.borrow_mut().push("raw-submit"));
                        }),
                        None,
                        false,
                    ),
                ],
                on_submit: move |()| {
                    FORM_EVENTS.with(|events| events.borrow_mut().push("adapter-submit"));
                },
                input { name: "email", required: true }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

    let form = parent
        .query_selector("#wasm-global-submit-form")
        .expect("query should succeed")
        .expect("form should exist");

    let submit = cancelable_event("submit");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

    assert!(submit.default_prevented());
    FORM_EVENTS.with(|events| {
        assert!(
            events.borrow().is_empty(),
            "raw global onsubmit must not replace the adapter's validation handler"
        );
    });
    assert_eq!(
        form.query_selector("[data-ars-part='status-region']")
            .expect("query should succeed")
            .expect("status region should exist")
            .text_content()
            .as_deref(),
        Some("1 error found. Please correct the highlighted field.")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn formnovalidate_submitter_skips_aria_constraint_validation() {
    FORM_EVENTS.with(|events| events.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Form {
                id: "wasm-formnovalidate-form",
                on_submit: move |()| {
                    FORM_EVENTS.with(|events| events.borrow_mut().push("submit"));
                },
                input { name: "email", required: true }
                button { r#type: "submit", formnovalidate: true, "Submit without validation" }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

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

    let submit = cancelable_event("submit");
    js_sys::Reflect::set(
        submit.as_ref(),
        &wasm_bindgen::JsValue::from_str("submitter"),
        button.as_ref(),
    )
    .expect("submitter property should be attached");

    form.dispatch_event(&submit)
        .expect("submit event should dispatch");

    flush().await;

    FORM_EVENTS.with(|events| {
        assert_eq!(
            events.borrow().as_slice(),
            &["submit"],
            "formnovalidate submitter should bypass ARIA constraint blocking"
        );
    });
    assert!(
        submit.default_prevented(),
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

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn form_invalid_required_submit_updates_named_field_errors() {
    fn app() -> Element {
        rsx! {
            Form { id: "wasm-invalid-field-form",
                Field {
                    id: "wasm-invalid-email-field",
                    name: "email",
                    required: true,
                    Label { "Email" }
                    Input { name: "email" }
                    ErrorMessage { "Email is required." }
                }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

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

    flush().await;

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

    flush().await;

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
    fn app() -> Element {
        rsx! {
            Form {
                id: "wasm-controlled-error-form",
                validation_errors: BTreeMap::from([
                    ("email".to_string(), vec![Error::server("Server still rejects this email.")]),
                ]),
                Field {
                    id: "wasm-controlled-error-email-field",
                    name: "email",
                    required: true,
                    Label { "Email" }
                    Input { name: "email", value: "admin@email.com" }
                    ErrorMessage { "Server still rejects this email." }
                }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone()),
    );

    flush().await;

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

    flush().await;

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
