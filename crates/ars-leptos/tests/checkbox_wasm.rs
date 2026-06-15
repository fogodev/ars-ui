//! Browser tests for the Leptos Checkbox adapter.

#![cfg(target_arch = "wasm32")]

use ars_components::input::checkbox::State;
use ars_leptos::{
    input::checkbox,
    utility::{
        button::{Button, Type as ButtonType},
        fieldset::Fieldset,
        form::Form,
    },
};
use leptos::{
    children::{TypedChildren, ViewFn},
    mount::mount_to,
    prelude::*,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[component]
fn TestCheckbox<T>(
    #[prop(into)] id: &'static str,
    #[prop(optional, into)] checked: Option<Signal<State>>,
    #[prop(optional, default = State::Unchecked)] default_checked: State,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional, into)] name: Option<&'static str>,
    #[prop(optional, into)] value: Option<&'static str>,
    #[prop(optional, into)] error_message: Option<ViewFn>,
    #[prop(optional, into, default = Callback::new(|_| ()))] on_checked_change: Callback<State>,
    children: TypedChildren<T>,
) -> impl IntoView
where
    T: IntoView + 'static,
{
    let has_error_message = error_message.is_some();
    let label = children.into_inner();

    match (checked, name) {
        (Some(checked), Some(name)) => view! {
            <checkbox::Root
                id=id
                checked=checked
                default_checked=default_checked
                disabled=disabled
                readonly=readonly
                invalid=invalid
                name=name
                value=value.unwrap_or("on")
                has_error_message=has_error_message
                on_checked_change=on_checked_change
            >
                <checkbox::Label>{label()}</checkbox::Label>
                <checkbox::Control>
                    <checkbox::Indicator />
                </checkbox::Control>
                <checkbox::HiddenInput />
                {error_message
                    .map(|error_message| {
                        view! {
                            <checkbox::ErrorMessage>{error_message.run()}</checkbox::ErrorMessage>
                        }
                    })}
            </checkbox::Root>
        },
        (Some(checked), None) => view! {
            <checkbox::Root
                id=id
                checked=checked
                default_checked=default_checked
                disabled=disabled
                readonly=readonly
                invalid=invalid
                has_error_message=has_error_message
                on_checked_change=on_checked_change
            >
                <checkbox::Label>{label()}</checkbox::Label>
                <checkbox::Control>
                    <checkbox::Indicator />
                </checkbox::Control>
                <checkbox::HiddenInput />
                {error_message
                    .map(|error_message| {
                        view! {
                            <checkbox::ErrorMessage>{error_message.run()}</checkbox::ErrorMessage>
                        }
                    })}
            </checkbox::Root>
        },
        (None, Some(name)) => view! {
            <checkbox::Root
                id=id
                default_checked=default_checked
                disabled=disabled
                readonly=readonly
                invalid=invalid
                name=name
                value=value.unwrap_or("on")
                has_error_message=has_error_message
                on_checked_change=on_checked_change
            >
                <checkbox::Label>{label()}</checkbox::Label>
                <checkbox::Control>
                    <checkbox::Indicator />
                </checkbox::Control>
                <checkbox::HiddenInput />
                {error_message
                    .map(|error_message| {
                        view! {
                            <checkbox::ErrorMessage>{error_message.run()}</checkbox::ErrorMessage>
                        }
                    })}
            </checkbox::Root>
        },
        (None, None) => view! {
            <checkbox::Root
                id=id
                default_checked=default_checked
                disabled=disabled
                readonly=readonly
                invalid=invalid
                has_error_message=has_error_message
                on_checked_change=on_checked_change
            >
                <checkbox::Label>{label()}</checkbox::Label>
                <checkbox::Control>
                    <checkbox::Indicator />
                </checkbox::Control>
                <checkbox::HiddenInput />
                {error_message
                    .map(|error_message| {
                        view! {
                            <checkbox::ErrorMessage>{error_message.run()}</checkbox::ErrorMessage>
                        }
                    })}
            </checkbox::Root>
        },
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
    let owner = Owner::new();

    let (mount_handle, root) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! { <TestCheckbox id="leptos-checkbox-wasm">"Alerts"</TestCheckbox> }
        });

        let root = parent
            .query_selector("#leptos-checkbox-wasm")
            .expect("query should succeed")
            .expect("root should exist");

        (mount_handle, root)
    });

    leptos::task::tick().await;

    let control = control(&root);

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    control.click();

    leptos::task::tick().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    keydown_space(&control);

    leptos::task::tick().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    drop(mount_handle);

    root.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_controlled_indeterminate_waits_for_parent_update() {
    let owner = Owner::new();

    let (mount_handle, root, set_checked) = owner.with(|| {
        let parent = container();

        let (checked, set_checked) = signal(State::Indeterminate);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <TestCheckbox id="leptos-checkbox-controlled" checked>
                    "Controlled"
                </TestCheckbox>
            }
        });

        let root = parent
            .query_selector("#leptos-checkbox-controlled")
            .expect("query should succeed")
            .expect("root should exist");

        (mount_handle, root, set_checked)
    });

    leptos::task::tick().await;

    let control = control(&root);

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("mixed")
    );

    control.click();

    leptos::task::tick().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("mixed")
    );

    set_checked.set(State::Checked);

    leptos::task::tick().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    drop(mount_handle);

    root.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_focus_visible_and_blocked_states_are_reflected() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <TestCheckbox id="leptos-checkbox-focus">"Focus"</TestCheckbox>
                <TestCheckbox id="leptos-checkbox-disabled" disabled=true>
                    "Disabled"
                </TestCheckbox>
                <TestCheckbox id="leptos-checkbox-readonly" readonly=true>
                    "Readonly"
                </TestCheckbox>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let focus = parent
        .query_selector("#leptos-checkbox-focus")
        .expect("query should succeed")
        .expect("focus root should exist");

    let focus_control = control(&focus);

    focus_control.focus().expect("focus should succeed");

    leptos::task::tick().await;

    assert_eq!(
        focus.get_attribute("data-ars-focus-visible").as_deref(),
        Some("")
    );

    for id in ["leptos-checkbox-disabled", "leptos-checkbox-readonly"] {
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

        leptos::task::tick().await;

        assert_eq!(
            control.get_attribute("aria-checked").as_deref(),
            Some("false")
        );
    }

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_in_form_submits_resets_and_remains_interactive() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let (newsletter, set_newsletter) = signal(State::Unchecked);
        let (required, set_required) = signal(State::Checked);
        let (status, set_status) = signal(String::new());

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Form
                    id="leptos-checkbox-widget-form"
                    on_submit=move |()| {
                        if newsletter.get_untracked() == State::Checked {
                            set_status
                                .set("Submitted: newsletter=weekly; terms=accepted".to_string());
                        } else {
                            set_status
                                .set("Submitted: newsletter=none; terms=accepted".to_string());
                        }
                    }
                    on_reset=move |()| {
                        set_newsletter.set(State::Unchecked);
                        set_required.set(State::Checked);
                        set_status.set("Form reset.".to_string());
                    }
                >
                    <TestCheckbox
                        id="leptos-checkbox-widget-newsletter"
                        name="newsletter"
                        value="weekly"
                        checked=newsletter
                        on_checked_change=move |next| set_newsletter.set(next)
                    >
                        "Optional newsletter value"
                    </TestCheckbox>
                    <TestCheckbox
                        id="leptos-checkbox-widget-required"
                        name="terms"
                        checked=required
                        on_checked_change=move |next| set_required.set(next)
                    >
                        "Required checked value"
                    </TestCheckbox>
                    <Button id="leptos-checkbox-widget-submit" r#type=ButtonType::Submit>
                        "Submit"
                    </Button>
                    <Button id="leptos-checkbox-widget-reset" r#type=ButtonType::Reset>
                        "Reset"
                    </Button>
                    <p id="leptos-checkbox-widget-status">{move || status.get()}</p>
                </Form>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let newsletter_root = parent
        .query_selector("#leptos-checkbox-widget-newsletter")
        .expect("query should succeed")
        .expect("newsletter checkbox root should exist");

    let newsletter_control = control(&newsletter_root);

    let submit = parent
        .query_selector("#leptos-checkbox-widget-submit")
        .expect("query should succeed")
        .expect("submit button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("submit button should be an HtmlElement");

    submit.click();

    leptos::task::tick().await;

    assert_eq!(
        parent
            .query_selector("#leptos-checkbox-widget-status")
            .expect("query should succeed")
            .expect("status should exist")
            .text_content()
            .as_deref(),
        Some("Submitted: newsletter=none; terms=accepted")
    );

    newsletter_control.click();

    leptos::task::tick().await;

    assert_eq!(
        newsletter_control.get_attribute("aria-checked").as_deref(),
        Some("true")
    );

    parent
        .query_selector("#leptos-checkbox-widget-reset")
        .expect("query should succeed")
        .expect("reset button should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("reset button should be an HtmlElement")
        .click();

    leptos::task::tick().await;

    assert_eq!(
        newsletter_control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    assert_eq!(
        parent
            .query_selector("#leptos-checkbox-widget-status")
            .expect("query should succeed")
            .expect("status should exist")
            .text_content()
            .as_deref(),
        Some("Form reset.")
    );

    drop(mount_handle);

    parent.remove();
}

#[wasm_bindgen_test(async)]
async fn checkbox_in_fieldset_inherits_blocking_state_in_browser() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), || {
            view! {
                <Fieldset id="leptos-checkbox-fieldset" disabled=true readonly=true invalid=true>
                    <TestCheckbox id="leptos-checkbox-fieldset-child">"Legal terms"</TestCheckbox>
                </Fieldset>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let root = parent
        .query_selector("#leptos-checkbox-fieldset-child")
        .expect("query should succeed")
        .expect("checkbox root should exist");

    let control = control(&root);

    for (name, expected) in [
        ("aria-disabled", "true"),
        ("aria-readonly", "true"),
        ("aria-invalid", "true"),
        ("aria-checked", "false"),
    ] {
        assert_eq!(control.get_attribute(name).as_deref(), Some(expected));
    }

    control.click();

    leptos::task::tick().await;

    assert_eq!(
        control.get_attribute("aria-checked").as_deref(),
        Some("false")
    );

    drop(mount_handle);

    parent.remove();
}
