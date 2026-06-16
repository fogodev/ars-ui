//! Browser tests for the Dioxus Checkbox adapter.

#![cfg(target_arch = "wasm32")]
#![expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports test event closures as unnecessary qualifications."
)]

use ars_components::input::checkbox::State;
use ars_dioxus::{
    input::checkbox,
    utility::{
        button::{Button, Type as ButtonType},
        form::Form,
    },
};
use dioxus::{dioxus_core::AttributeValue, prelude::*};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Props, Clone, PartialEq)]
struct TestCheckboxProps {
    id: &'static str,
    #[props(optional)]
    checked: Option<State>,
    #[props(default = State::Unchecked)]
    default_checked: State,
    #[props(default = false)]
    disabled: bool,
    #[props(default = false)]
    readonly: bool,
    #[props(default = false)]
    invalid: bool,
    #[props(optional)]
    name: Option<&'static str>,
    #[props(optional)]
    value: Option<&'static str>,
    #[props(optional)]
    error_message: Option<Element>,
    #[props(optional)]
    on_checked_change: Option<EventHandler<State>>,
    children: Element,
}

#[component]
fn TestCheckbox(props: TestCheckboxProps) -> Element {
    rsx! {
        checkbox::Root {
            id: props.id,
            checked: props.checked,
            default_checked: props.default_checked,
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            name: props.name.map(str::to_string),
            value: props.value.map(str::to_string),
            on_checked_change: props.on_checked_change,
            checkbox::Label { {props.children} }
            checkbox::Control { checkbox::Indicator {} }
            checkbox::HiddenInput {}
            if let Some(error_message) = props.error_message {
                checkbox::ErrorMessage { {error_message} }
            }
        }
    }
}

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

async fn animation_frame_turn() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let callback = wasm_bindgen::closure::Closure::once_into_js({
            let resolve = resolve.clone();
            move || {
                drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
            }
        });

        web_sys::window()
            .expect("window should exist")
            .request_animation_frame(callback.unchecked_ref())
            .expect("requestAnimationFrame should succeed");
    });

    drop(wasm_bindgen_futures::JsFuture::from(promise).await);
}

async fn flush() {
    for _ in 0..3 {
        animation_frame_turn().await;

        drop(
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                &wasm_bindgen::JsValue::UNDEFINED,
            ))
            .await,
        );
    }
}

fn control(root: &web_sys::Element) -> web_sys::HtmlElement {
    root.query_selector("[data-ars-part='control']")
        .expect("query should succeed")
        .expect("control should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("control should be an HtmlElement")
}

fn keydown_space(element: &web_sys::HtmlElement) {
    let init = web_sys::KeyboardEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_key(" ");
    init.set_code("Space");

    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keyboard event should construct");

    element
        .dispatch_event(&event)
        .expect("keyboard event should dispatch");
}

#[wasm_bindgen_test(async)]
async fn checkbox_click_and_space_toggle_state() {
    fn app() -> Element {
        rsx! {
            TestCheckbox { id: "dioxus-checkbox-wasm", "Alerts" }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let root = parent
        .query_selector("#dioxus-checkbox-wasm")
        .expect("query should succeed")
        .expect("root should exist");

    let control = control(&root);

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    control.click();

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    keydown_space(&control);

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_control_composes_consumer_click_handler() {
    fn app() -> Element {
        let mut consumer_clicked = use_signal(|| false);

        rsx! {
            checkbox::Root { id: "dioxus-checkbox-consumer-control",
                checkbox::Label { "Consumer handler" }
                checkbox::Control {
                    attrs: vec![
                        Attribute::new(
                            "onclick",
                            AttributeValue::listener(move |_event: Event<MouseData>| {
                                consumer_clicked.set(true);
                            }),
                            None,
                            false,
                        ),
                    ],
                    checkbox::Indicator {}
                }
                checkbox::HiddenInput {}
            }
            p { id: "dioxus-checkbox-consumer-clicked", "{consumer_clicked}" }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let root = parent
        .query_selector("#dioxus-checkbox-consumer-control")
        .expect("query should succeed")
        .expect("root should exist");

    let control = control(&root);

    control.click();

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true"),
        "built-in checkbox toggle should still run"
    );
    assert_eq!(
        parent
            .query_selector("#dioxus-checkbox-consumer-clicked")
            .expect("query should succeed")
            .expect("status should exist")
            .text_content()
            .as_deref(),
        Some("true"),
        "consumer onclick should run"
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_controlled_indeterminate_waits_for_parent_update() {
    fn app() -> Element {
        let mut checked = use_signal(|| State::Indeterminate);

        rsx! {
            TestCheckbox {
                id: "dioxus-checkbox-controlled",
                checked: checked(),
                on_checked_change: move |_| {},
                "Controlled"
            }
            button {
                id: "dioxus-checkbox-parent-update",
                onclick: move |_| checked.set(State::Checked),
                "Set checked"
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let root = parent
        .query_selector("#dioxus-checkbox-controlled")
        .expect("query should succeed")
        .expect("root should exist");

    let control = control(&root);

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("mixed")
    );

    control.click();

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("mixed")
    );

    parent
        .query_selector("#dioxus-checkbox-parent-update")
        .expect("query should succeed")
        .expect("button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("button should be an HtmlElement")
        .click();

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_focus_visible_and_blocked_states_are_reflected() {
    fn app() -> Element {
        rsx! {
            TestCheckbox { id: "dioxus-checkbox-focus", "Focus" }
            TestCheckbox { id: "dioxus-checkbox-disabled", disabled: true, "Disabled" }
            TestCheckbox { id: "dioxus-checkbox-readonly", readonly: true, "Readonly" }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let focus = parent
        .query_selector("#dioxus-checkbox-focus")
        .expect("query should succeed")
        .expect("focus root should exist");

    let focus_control = control(&focus);

    focus_control.focus().expect("focus should succeed");

    flush().await;

    assert!(
        matches!(
            focus.get_attribute("data-ars-focus-visible").as_deref(),
            Some("") | Some("true")
        ),
        "focus-visible data attr should be present"
    );

    for id in ["dioxus-checkbox-disabled", "dioxus-checkbox-readonly"] {
        let root = parent
            .query_selector(&format!("#{id}"))
            .expect("query should succeed")
            .expect("root should exist");

        let control = control(&root);

        assert_eq!(
            control.get_attribute("aria-checked").as_deref(),
            Some("false")
        );

        control.click();

        flush().await;

        assert_eq!(
            control.get_attribute("aria-checked").as_deref(),
            Some("false")
        );
    }

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_inside_form_remains_interactive_after_submit() {
    fn app() -> Element {
        let mut checked = use_signal(|| State::Checked);
        let mut submitted = use_signal(|| false);

        rsx! {
            Form {
                id: "dioxus-checkbox-form",
                on_submit: move |_| submitted.set(true),
                TestCheckbox {
                    id: "dioxus-checkbox-form-value",
                    name: "terms",
                    checked: checked(),
                    on_checked_change: move |next| checked.set(next),
                    "Terms"
                }
                button { id: "dioxus-checkbox-form-submit", r#type: "submit", "Submit" }

                if submitted() {
                    p { id: "dioxus-checkbox-form-status", "Submitted" }
                }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let root = parent
        .query_selector("#dioxus-checkbox-form-value")
        .expect("query should succeed")
        .expect("checkbox root should exist");

    let control = control(&root);

    let submit = parent
        .query_selector("#dioxus-checkbox-form-submit")
        .expect("query should succeed")
        .expect("submit button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("submit button should be an HtmlElement");

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    submit.click();

    flush().await;

    assert!(
        parent
            .query_selector("#dioxus-checkbox-form-status")
            .expect("query should succeed")
            .is_some(),
        "form submit status should render"
    );

    control.click();

    flush().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_widget_form_pattern_remains_interactive_after_submit() {
    fn app() -> Element {
        let mut newsletter = use_signal(|| State::Unchecked);
        let mut required = use_signal(|| State::Checked);
        let mut submit_attempted = use_signal(|| false);
        let mut status = use_signal(String::new);

        let required_invalid = submit_attempted() && required() != State::Checked;

        rsx! {
            Form {
                id: "dioxus-checkbox-widget-form",
                on_submit: move |_| {
                    submit_attempted.set(true);

                    if required() != State::Checked {
                        status.set("Select the required value before submitting.".to_string());
                    } else if newsletter() == State::Checked {
                        status.set("Submitted: newsletter=weekly; terms=accepted".to_string());
                    } else {
                        status.set("Submitted: newsletter=none; terms=accepted".to_string());
                    }
                },
                on_reset: move |_| {
                    newsletter.set(State::Unchecked);
                    required.set(State::Checked);
                    submit_attempted.set(false);
                    status.set("Form reset.".to_string());
                },
                TestCheckbox {
                    id: "dioxus-checkbox-widget-newsletter",
                    name: "newsletter",
                    value: "weekly",
                    checked: newsletter(),
                    on_checked_change: move |next| newsletter.set(next),
                    "Optional newsletter value"
                }
                TestCheckbox {
                    id: "dioxus-checkbox-widget-required",
                    name: "terms",
                    checked: required(),
                    invalid: required_invalid,
                    error_message: required_invalid.then(|| rsx! { "Select the required value before submitting." }),
                    on_checked_change: move |next| required.set(next),
                    "Required checked value"
                }
                Button {
                    id: "dioxus-checkbox-widget-submit",
                    r#type: ButtonType::Submit,
                    "Submit"
                }
                Button {
                    id: "dioxus-checkbox-widget-reset",
                    r#type: ButtonType::Reset,
                    "Reset"
                }
                p { id: "dioxus-checkbox-widget-status", "{status}" }
            }
        }
    }

    let parent = container();

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    flush().await;

    let newsletter_root = parent
        .query_selector("#dioxus-checkbox-widget-newsletter")
        .expect("query should succeed")
        .expect("newsletter checkbox root should exist");

    let newsletter_control = control(&newsletter_root);

    let submit = parent
        .query_selector("#dioxus-checkbox-widget-submit")
        .expect("query should succeed")
        .expect("submit button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("submit button should be an HtmlElement");

    submit.click();

    flush().await;

    assert_eq!(
        parent
            .query_selector("#dioxus-checkbox-widget-status")
            .expect("query should succeed")
            .expect("status should exist")
            .text_content()
            .as_deref(),
        Some("Submitted: newsletter=none; terms=accepted")
    );

    newsletter_control.click();

    flush().await;

    assert_eq!(
        newsletter_control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    parent
        .query_selector("#dioxus-checkbox-widget-reset")
        .expect("query should succeed")
        .expect("reset button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("reset button should be an HtmlElement")
        .click();

    flush().await;

    assert_eq!(
        newsletter_control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    assert_eq!(
        parent
            .query_selector("#dioxus-checkbox-widget-status")
            .expect("query should succeed")
            .expect("status should exist")
            .text_content()
            .as_deref(),
        Some("Form reset.")
    );

    parent.remove();
}
