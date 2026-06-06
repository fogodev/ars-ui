//! Browser coverage tests for the Dioxus Form adapter.

#![cfg(target_arch = "wasm32")]

use std::{cell::RefCell, collections::BTreeMap};

use ars_dioxus::utility::{
    field::{ErrorMessage, Field, Input, Label},
    form::Form,
};
use ars_forms::validation::Error;
use dioxus::prelude::*;
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
        Some("Please correct the highlighted fields.")
    );

    parent.remove();
}
