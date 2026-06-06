//! Browser coverage tests for the Dioxus Field adapter.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

use ars_dioxus::utility::field::{Description, ErrorMessage, Field, Input, InputType, Label};
use ars_forms::validation::Error;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static INPUT_VALUES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
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

fn dispatch_input(target: &web_sys::Element, value: &str) {
    js_sys::Reflect::set(
        target,
        &wasm_bindgen::JsValue::from_str("value"),
        &wasm_bindgen::JsValue::from_str(value),
    )
    .expect("input value property should be set");

    let init = web_sys::EventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    let event = web_sys::Event::new_with_event_init_dict("input", &init)
        .expect("input event should construct");

    target
        .dispatch_event(&event)
        .expect("input event should dispatch");
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
async fn field_browser_renders_relationship_attrs_and_input_callback() {
    INPUT_VALUES.with(|values| values.borrow_mut().clear());

    fn app() -> Element {
        rsx! {
            Field {
                id: "wasm-email-field",
                required: true,
                invalid: true,
                errors: vec![Error::server("Email is required.")],
                Label { "Email" }
                Input {
                    r#type: InputType::Email,
                    name: "email",
                    placeholder: "Enter your email",
                    on_value_input: move |value: String| {
                        INPUT_VALUES.with(|values| values.borrow_mut().push(value));
                    },
                }
                Description { "Use a reachable email." }
                ErrorMessage { "Email is required." }
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
        .query_selector("#wasm-email-field-input")
        .expect("query should succeed")
        .expect("field input should exist");

    assert_eq!(input.get_attribute("type").as_deref(), Some("email"));
    assert_eq!(input.get_attribute("name").as_deref(), Some("email"));
    assert_eq!(
        input.get_attribute("placeholder").as_deref(),
        Some("Enter your email")
    );
    assert_eq!(
        input.get_attribute("aria-labelledby").as_deref(),
        Some("wasm-email-field-label")
    );
    assert_eq!(
        input.get_attribute("aria-describedby").as_deref(),
        Some("wasm-email-field-description wasm-email-field-error-message")
    );
    assert_eq!(
        input.get_attribute("aria-required").as_deref(),
        Some("true")
    );
    assert_bool_attr(&input, "required");
    assert_eq!(input.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        input.get_attribute("aria-errormessage").as_deref(),
        Some("wasm-email-field-error-message")
    );

    dispatch_input(&input, "ericson@example.com");

    flush().await;

    INPUT_VALUES.with(|values| {
        assert_eq!(
            values.borrow().as_slice(),
            &[String::from("ericson@example.com")]
        );
    });

    parent.remove();
}
