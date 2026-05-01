//! Browser tests for the Leptos Button adapter.

#![cfg(target_arch = "wasm32")]

use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ars_interactions::PressEvent;
use ars_leptos::utility::button::{self, Button};
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

fn push_press(log: &Arc<Mutex<Vec<String>>>, label: &str, event: &PressEvent) {
    log.lock()
        .expect("press log should not be poisoned")
        .push(format!(
            "{label}:{:?}:{:?}:x={:?}:y={:?}:shift={}:alt={}",
            event.pointer_type,
            event.event_type,
            event.client_x,
            event.client_y,
            event.modifiers.shift,
            event.modifiers.alt,
        ));
}

fn dispatch_mouse_pointer(root: &web_sys::HtmlElement, event_type: &str) {
    let init = web_sys::PointerEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_pointer_type("mouse");
    init.set_client_x(12);
    init.set_client_y(34);

    let event = web_sys::PointerEvent::new_with_event_init_dict(event_type, &init)
        .expect("pointer event should construct");

    assert!(
        root.dispatch_event(&event)
            .expect("pointer event should dispatch"),
        "{event_type} should not be canceled"
    );
}

fn dispatch_mouse_click(root: &web_sys::HtmlElement) {
    let event = web_sys::MouseEvent::new("click").expect("click should construct");
    event.init_mouse_event_with_can_bubble_arg_and_cancelable_arg_and_view_arg_and_detail_arg_and_screen_x_arg_and_screen_y_arg_and_client_x_arg_and_client_y_arg_and_ctrl_key_arg_and_alt_key_arg_and_shift_key_arg_and_meta_key_arg(
        "click",
        true,
        true,
        web_sys::window().as_ref(),
        0,
        0,
        0,
        56,
        78,
        false,
        true,
        true,
        false,
    );

    assert!(
        root.dispatch_event(&event).expect("click should dispatch"),
        "click should not be canceled"
    );
}

#[wasm_bindgen_test(async)]
async fn button_reactive_loading_updates_root_and_parts() {
    let owner = Owner::new();

    let (mount_handle, root, set_loading) = owner.with(|| {
        let parent = container();

        let (loading, set_loading) = signal(false);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Button id="reactive-button" loading=loading>
                    "Save"
                </Button>
            }
        });

        let root = parent
            .query_selector("#reactive-button")
            .expect("query should succeed")
            .expect("button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button should be an HtmlElement");

        (mount_handle, root, set_loading)
    });

    leptos::task::tick().await;

    assert_eq!(root.get_attribute("aria-busy"), None);
    assert!(
        root.query_selector("[data-ars-part='loading-indicator']")
            .expect("query should succeed")
            .is_none()
    );

    set_loading.set(true);

    leptos::task::tick().await;

    assert_eq!(root.get_attribute("aria-busy").as_deref(), Some("true"));
    assert_eq!(root.get_attribute("aria-disabled").as_deref(), Some("true"));

    let indicator = root
        .query_selector("[data-ars-part='loading-indicator']")
        .expect("query should succeed")
        .expect("loading indicator should exist");

    assert_eq!(indicator.get_attribute("role").as_deref(), Some("status"));
    assert_eq!(
        indicator.get_attribute("aria-live").as_deref(),
        Some("polite")
    );
    assert_eq!(
        indicator.get_attribute("aria-label").as_deref(),
        Some("Loading")
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn button_press_callbacks_fire_in_native_event_order() {
    let owner = Owner::new();

    let (mount_handle, root, log) = owner.with(|| {
        let parent = container();
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let start_log = Arc::clone(&log);
        let end_log = Arc::clone(&log);
        let press_log = Arc::clone(&log);
        let change_log = Arc::clone(&log);
        let up_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Button
                    id="callback-button"
                    on_press_start=Callback::new(move |event| {
                        push_press(&start_log, "start", &event);
                    })
                    on_press_end=Callback::new(move |event| {
                        push_press(&end_log, "end", &event);
                    })
                    on_press=Callback::new(move |event| {
                        push_press(&press_log, "press", &event);
                    })
                    on_press_change=Callback::new(move |pressed| {
                        change_log
                            .lock()
                            .expect("press log should not be poisoned")
                            .push(format!("change:{pressed}"));
                    })
                    on_press_up=Callback::new(move |event| {
                        push_press(&up_log, "up", &event);
                    })
                >
                    "Save"
                </Button>
            }
        });

        let root = parent
            .query_selector("#callback-button")
            .expect("query should succeed")
            .expect("button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button should be an HtmlElement");

        (mount_handle, root, log)
    });

    leptos::task::tick().await;

    dispatch_mouse_pointer(&root, "pointerdown");
    dispatch_mouse_pointer(&root, "pointerup");
    dispatch_mouse_click(&root);

    leptos::task::tick().await;

    assert_eq!(
        log.lock()
            .expect("press log should not be poisoned")
            .as_slice(),
        &[
            String::from("start:Mouse:PressStart:x=Some(12.0):y=Some(34.0):shift=false:alt=false"),
            String::from("change:true"),
            String::from("end:Mouse:PressEnd:x=Some(12.0):y=Some(34.0):shift=false:alt=false"),
            String::from("up:Mouse:PressUp:x=Some(12.0):y=Some(34.0):shift=false:alt=false"),
            String::from("change:false"),
            String::from("press:Mouse:Press:x=Some(56.0):y=Some(78.0):shift=true:alt=true"),
        ]
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn disabled_button_suppresses_press_callbacks() {
    let owner = Owner::new();

    let (mount_handle, root, log) = owner.with(|| {
        let parent = container();

        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let start_log = Arc::clone(&log);
        let end_log = Arc::clone(&log);
        let press_log = Arc::clone(&log);
        let change_log = Arc::clone(&log);
        let up_log = Arc::clone(&log);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Button
                    id="disabled-callback-button"
                    disabled=true
                    on_press_start=Callback::new(move |event| {
                        push_press(&start_log, "start", &event);
                    })
                    on_press_end=Callback::new(move |event| {
                        push_press(&end_log, "end", &event);
                    })
                    on_press=Callback::new(move |event| {
                        push_press(&press_log, "press", &event);
                    })
                    on_press_change=Callback::new(move |pressed| {
                        change_log
                            .lock()
                            .expect("press log should not be poisoned")
                            .push(format!("change:{pressed}"));
                    })
                    on_press_up=Callback::new(move |event| {
                        push_press(&up_log, "up", &event);
                    })
                >
                    "Save"
                </Button>
            }
        });

        let root = parent
            .query_selector("#disabled-callback-button")
            .expect("query should succeed")
            .expect("button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button should be an HtmlElement");

        (mount_handle, root, log)
    });

    leptos::task::tick().await;

    dispatch_mouse_pointer(&root, "pointerdown");
    dispatch_mouse_pointer(&root, "pointerup");
    dispatch_mouse_click(&root);

    leptos::task::tick().await;

    assert!(
        log.lock()
            .expect("press log should not be poisoned")
            .is_empty(),
        "disabled buttons must not emit press callbacks"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn button_prevent_focus_on_press_cancels_pointerdown_default() {
    let owner = Owner::new();

    let (mount_handle, root) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Button id="prevent-focus-button" prevent_focus_on_press=true>
                    "Open"
                </Button>
            }
        });

        let root = parent
            .query_selector("#prevent-focus-button")
            .expect("query should succeed")
            .expect("button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button should be an HtmlElement");

        (mount_handle, root)
    });

    leptos::task::tick().await;

    let init = web_sys::PointerEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_pointer_type("mouse");

    let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
        .expect("pointerdown should construct");

    assert!(
        !root
            .dispatch_event(&event)
            .expect("pointerdown should dispatch"),
        "prevent_focus_on_press should cancel pointerdown default"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn loading_submit_button_cancels_native_activation() {
    let owner = Owner::new();

    let (mount_handle, root, submits) = owner.with(|| {
        let parent = container();
        let submits = Rc::new(Cell::new(0usize));

        let mount_handle = mount_to(parent.clone(), {
            let submit = Rc::clone(&submits);
            move || {
                view! {
                    <form
                        id="loading-form"
                        on:submit=move |ev| {
                            submit.set(submit.get() + 1);
                            ev.prevent_default();
                        }
                    >
                        <Button id="loading-submit" loading=true r#type=button::Type::Submit>
                            "Save"
                        </Button>
                    </form>
                }
            }
        });

        let root = parent
            .query_selector("#loading-submit")
            .expect("query should succeed")
            .expect("button should exist")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button should be an HtmlElement");

        (mount_handle, root, submits)
    });

    leptos::task::tick().await;

    root.click();

    leptos::task::tick().await;

    assert_eq!(
        submits.get(),
        0,
        "loading submit button should prevent native form submission"
    );

    drop(mount_handle);
}
