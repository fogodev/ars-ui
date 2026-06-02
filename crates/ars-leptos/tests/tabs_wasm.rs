//! Browser tests for the Leptos Tabs adapter.
//!
//! These exercise the WASM-only code paths: keyboard navigation,
//! roving-tabindex focus dispatch, controlled selection sync, and the
//! Ctrl+Arrow reorder live announcement.

#![cfg(target_arch = "wasm32")]

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use ars_collections::Key;
use ars_core::{
    Direction, I18nRegistries, MessageFn, MessagesRegistry, Orientation, PlatformEffects, Rect,
    ResolvedDirection, SafeUrl, TimerHandle,
};
use ars_i18n::locales;
use ars_leptos::{
    ArsProvider,
    navigation::tabs::{ActivationMode, Field, ReorderEvent, Tab, TabLabel, Tabs, TabsSource},
    reactive_stores::{self, Store},
};
use leptos::{
    children::ViewFn,
    mount::mount_to,
    prelude::*,
    web_sys::{self, DragEventInit, KeyboardEventInit, MouseEventInit, PointerEventInit},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

type TestTab = Tab<&'static str>;
type TestReorderEvent = ReorderEvent<&'static str>;

#[wasm_bindgen_test]
fn tab_public_builders_and_debug_paths_are_covered() {
    let label = TabLabel::static_text("Standalone");

    assert_eq!(label.resolve(), "Standalone");
    assert!(format!("{label:?}").contains("Standalone"));

    let tab = Tab::new_static("standalone", "Standalone", || view! { "Panel" })
        .trigger(|| view! { <strong>"Standalone trigger"</strong> })
        .disabled(true)
        .closable(true)
        .link(SafeUrl::from_static("/standalone"));

    assert_eq!(tab.key, "standalone");
    assert_eq!(tab.label_text.resolve(), "Standalone");
    assert!(tab.disabled);
    assert!(tab.closable);
    assert_eq!(
        tab.link.as_ref().map(ToString::to_string),
        Some("/standalone".to_owned())
    );

    let tab_debug = format!("{tab:?}");

    assert!(tab_debug.contains("Tab"));
    assert!(tab_debug.contains("standalone"));
    assert!(tab_debug.contains("Standalone"));

    let from_vec = TabsSource::from(vec![Tab::new_static(
        "first",
        "First",
        || view! { "Panel" },
    )]);

    assert!(matches!(from_vec, TabsSource::Owned(_)));
    assert!(format!("{from_vec:?}").contains("TabsSource::Owned"));

    let from_array = TabsSource::from([Tab::new_static("second", "Second", || view! { "Panel" })]);

    assert!(matches!(from_array, TabsSource::Owned(_)));
}

#[derive(Store)]
struct TabsTestState {
    tabs: Vec<TestTab>,
}

struct FocusByIdProbePlatform {
    calls: Arc<Mutex<Vec<String>>>,
}

impl PlatformEffects for FocusByIdProbePlatform {
    fn focus_element_by_id(&self, id: &str) {
        self.calls
            .lock()
            .expect("focus call log should not be poisoned")
            .push(id.to_owned());
    }

    fn focus_first_tabbable(&self, container_id: &str) {
        ars_dom::WebPlatformEffects.focus_first_tabbable(container_id);
    }

    fn focus_last_tabbable(&self, container_id: &str) {
        ars_dom::WebPlatformEffects.focus_last_tabbable(container_id);
    }

    fn tabbable_element_ids(&self, container_id: &str) -> Vec<String> {
        ars_dom::WebPlatformEffects.tabbable_element_ids(container_id)
    }

    fn focus_body(&self) {
        ars_dom::WebPlatformEffects.focus_body();
    }

    fn set_timeout(&self, delay: Duration, callback: Box<dyn FnOnce()>) -> TimerHandle {
        ars_dom::WebPlatformEffects.set_timeout(delay, callback)
    }

    fn clear_timeout(&self, handle: TimerHandle) {
        ars_dom::WebPlatformEffects.clear_timeout(handle);
    }

    fn announce(&self, message: &str) {
        ars_dom::WebPlatformEffects.announce(message);
    }

    fn announce_assertive(&self, message: &str) {
        ars_dom::WebPlatformEffects.announce_assertive(message);
    }

    fn position_element_at(&self, id: &str, x: f64, y: f64) {
        ars_dom::WebPlatformEffects.position_element_at(id, x, y);
    }

    fn resolved_direction(&self, id: &str) -> ResolvedDirection {
        ars_dom::WebPlatformEffects.resolved_direction(id)
    }

    fn set_background_inert(&self, portal_root_id: &str) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.set_background_inert(portal_root_id)
    }

    fn remove_inert_from_siblings(&self, portal_id: &str) {
        ars_dom::WebPlatformEffects.remove_inert_from_siblings(portal_id);
    }

    fn scroll_lock_acquire(&self) {
        ars_dom::WebPlatformEffects.scroll_lock_acquire();
    }

    fn scroll_lock_release(&self) {
        ars_dom::WebPlatformEffects.scroll_lock_release();
    }

    fn document_contains_id(&self, id: &str) -> bool {
        ars_dom::WebPlatformEffects.document_contains_id(id)
    }

    fn track_pointer_drag(
        &self,
        on_move: Box<dyn Fn(f64, f64)>,
        on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.track_pointer_drag(on_move, on_up)
    }

    fn active_element_id(&self) -> Option<String> {
        ars_dom::WebPlatformEffects.active_element_id()
    }

    fn attach_focus_trap(&self, container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.attach_focus_trap(container_id, on_escape)
    }

    fn can_restore_focus(&self, id: &str) -> bool {
        ars_dom::WebPlatformEffects.can_restore_focus(id)
    }

    fn nearest_focusable_ancestor_id(&self, id: &str) -> Option<String> {
        ars_dom::WebPlatformEffects.nearest_focusable_ancestor_id(id)
    }

    fn set_scroll_top(&self, container_id: &str, scroll_top: f64) {
        ars_dom::WebPlatformEffects.set_scroll_top(container_id, scroll_top);
    }

    fn resize_to_content(&self, id: &str, max_height: Option<&str>) {
        ars_dom::WebPlatformEffects.resize_to_content(id, max_height);
    }

    fn on_reduced_motion_change(&self, callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.on_reduced_motion_change(callback)
    }

    fn is_mac_platform(&self) -> bool {
        ars_dom::WebPlatformEffects.is_mac_platform()
    }

    fn now(&self) -> Duration {
        ars_dom::WebPlatformEffects.now()
    }

    fn get_bounding_rect(&self, id: &str) -> Option<Rect> {
        ars_dom::WebPlatformEffects.get_bounding_rect(id)
    }

    fn on_animation_end(&self, id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.on_animation_end(id, callback)
    }
}

struct MeasurementProbePlatform {
    calls: Arc<AtomicU32>,
}

impl PlatformEffects for MeasurementProbePlatform {
    fn focus_element_by_id(&self, id: &str) {
        ars_dom::WebPlatformEffects.focus_element_by_id(id);
    }

    fn focus_first_tabbable(&self, container_id: &str) {
        ars_dom::WebPlatformEffects.focus_first_tabbable(container_id);
    }

    fn focus_last_tabbable(&self, container_id: &str) {
        ars_dom::WebPlatformEffects.focus_last_tabbable(container_id);
    }

    fn tabbable_element_ids(&self, container_id: &str) -> Vec<String> {
        ars_dom::WebPlatformEffects.tabbable_element_ids(container_id)
    }

    fn focus_body(&self) {
        ars_dom::WebPlatformEffects.focus_body();
    }

    fn set_timeout(&self, delay: Duration, callback: Box<dyn FnOnce()>) -> TimerHandle {
        ars_dom::WebPlatformEffects.set_timeout(delay, callback)
    }

    fn clear_timeout(&self, handle: TimerHandle) {
        ars_dom::WebPlatformEffects.clear_timeout(handle);
    }

    fn announce(&self, message: &str) {
        ars_dom::WebPlatformEffects.announce(message);
    }

    fn announce_assertive(&self, message: &str) {
        ars_dom::WebPlatformEffects.announce_assertive(message);
    }

    fn position_element_at(&self, id: &str, x: f64, y: f64) {
        ars_dom::WebPlatformEffects.position_element_at(id, x, y);
    }

    fn resolved_direction(&self, id: &str) -> ResolvedDirection {
        ars_dom::WebPlatformEffects.resolved_direction(id)
    }

    fn set_background_inert(&self, portal_root_id: &str) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.set_background_inert(portal_root_id)
    }

    fn remove_inert_from_siblings(&self, portal_id: &str) {
        ars_dom::WebPlatformEffects.remove_inert_from_siblings(portal_id);
    }

    fn scroll_lock_acquire(&self) {
        ars_dom::WebPlatformEffects.scroll_lock_acquire();
    }

    fn scroll_lock_release(&self) {
        ars_dom::WebPlatformEffects.scroll_lock_release();
    }

    fn document_contains_id(&self, id: &str) -> bool {
        ars_dom::WebPlatformEffects.document_contains_id(id)
    }

    fn track_pointer_drag(
        &self,
        on_move: Box<dyn Fn(f64, f64)>,
        on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.track_pointer_drag(on_move, on_up)
    }

    fn active_element_id(&self) -> Option<String> {
        ars_dom::WebPlatformEffects.active_element_id()
    }

    fn attach_focus_trap(&self, container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.attach_focus_trap(container_id, on_escape)
    }

    fn can_restore_focus(&self, id: &str) -> bool {
        ars_dom::WebPlatformEffects.can_restore_focus(id)
    }

    fn nearest_focusable_ancestor_id(&self, id: &str) -> Option<String> {
        ars_dom::WebPlatformEffects.nearest_focusable_ancestor_id(id)
    }

    fn set_scroll_top(&self, container_id: &str, scroll_top: f64) {
        ars_dom::WebPlatformEffects.set_scroll_top(container_id, scroll_top);
    }

    fn resize_to_content(&self, id: &str, max_height: Option<&str>) {
        ars_dom::WebPlatformEffects.resize_to_content(id, max_height);
    }

    fn on_reduced_motion_change(&self, callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.on_reduced_motion_change(callback)
    }

    fn is_mac_platform(&self) -> bool {
        ars_dom::WebPlatformEffects.is_mac_platform()
    }

    fn now(&self) -> Duration {
        ars_dom::WebPlatformEffects.now()
    }

    fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
        let call = f64::from(self.calls.fetch_add(1, Ordering::SeqCst));

        Some(Rect {
            x: call,
            y: 0.0,
            width: 100.0 + call,
            height: 18.0,
        })
    }

    fn on_animation_end(&self, id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
        ars_dom::WebPlatformEffects.on_animation_end(id, callback)
    }
}

fn store_field(tabs: Vec<TestTab>) -> Field<Vec<TestTab>> {
    Store::new(TabsTestState { tabs }).tabs().into()
}

fn store_handle(tabs: Vec<TestTab>) -> Store<TabsTestState> {
    Store::new(TabsTestState { tabs })
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

fn add_indicator_measurement_styles(parent: &web_sys::HtmlElement) {
    let style = document()
        .create_element("style")
        .expect("style element should be created");

    style.set_text_content(Some(
        r#"
            [data-ars-part="list"] {
                display: inline-flex !important;
                align-items: center !important;
            }

            [data-ars-part="list"][aria-orientation="vertical"] {
                flex-direction: column !important;
                align-items: flex-start !important;
            }

            [data-ars-part="tab"] {
                display: inline-flex !important;
                width: auto !important;
            }
        "#,
    ));

    parent
        .append_child(&style)
        .expect("indicator measurement style should append");
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

async fn deferred_focus_turn() {
    animation_frame_turn().await;
    animation_frame_turn().await;
}

fn three_tabs() -> Vec<TestTab> {
    vec![
        Tab::new_with_label(
            "first",
            "First",
            ViewFn::from(|| view! { "First" }),
            ViewFn::from(|| view! { <p>"Panel one"</p> }),
        ),
        Tab::new_with_label(
            "second",
            "Second",
            ViewFn::from(|| view! { "Second" }),
            ViewFn::from(|| view! { <p>"Panel two"</p> }),
        ),
        Tab::new_with_label(
            "third",
            "Third",
            ViewFn::from(|| view! { "Third" }),
            ViewFn::from(|| view! { <p>"Panel three"</p> }),
        ),
    ]
}

fn dispatch_keydown(
    target: &web_sys::HtmlElement,
    key: &str,
    ctrl: bool,
) -> web_sys::KeyboardEvent {
    dispatch_keydown_with_options(target, key, ctrl, false, false)
}

fn dispatch_keydown_with_options(
    target: &web_sys::HtmlElement,
    key: &str,
    ctrl: bool,
    repeat: bool,
    is_composing: bool,
) -> web_sys::KeyboardEvent {
    let init = KeyboardEventInit::new();

    init.set_key(key);

    init.set_bubbles(true);

    init.set_cancelable(true);

    init.set_ctrl_key(ctrl);
    init.set_repeat(repeat);
    init.set_is_composing(is_composing);

    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keyboard event should construct");

    target
        .dispatch_event(&event)
        .expect("keydown should dispatch");

    event
}

fn dispatch_pointerdown(
    target: &web_sys::HtmlElement,
    pointer_type: &str,
) -> web_sys::PointerEvent {
    let init = PointerEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_pointer_type(pointer_type);

    let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
        .expect("pointerdown should construct");

    target
        .dispatch_event(&event)
        .expect("pointerdown should dispatch");

    event
}

fn click(target: &web_sys::HtmlElement) {
    let event = web_sys::MouseEvent::new("click").expect("click should construct");

    target
        .dispatch_event(&event)
        .expect("click should dispatch");
}

fn cancelable_click(target: &web_sys::HtmlElement) -> web_sys::MouseEvent {
    let init = MouseEventInit::new();

    init.set_bubbles(true);

    init.set_cancelable(true);

    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init)
        .expect("cancelable click should construct");

    target
        .dispatch_event(&event)
        .expect("click should dispatch");

    event
}

fn dispatch_drag_event(target: &web_sys::HtmlElement, kind: &str) -> web_sys::DragEvent {
    let init = DragEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    let event = web_sys::DragEvent::new_with_event_init_dict(kind, &init)
        .expect("drag event should construct");

    target
        .dispatch_event(&event)
        .expect("drag event should dispatch");

    event
}

fn first_with_data_part(parent: &web_sys::HtmlElement, part: &str) -> web_sys::HtmlElement {
    parent
        .query_selector(&format!("[data-ars-part='{part}']"))
        .expect("query should succeed")
        .unwrap_or_else(|| panic!("missing [data-ars-part='{part}'] in subtree"))
        .dyn_into::<web_sys::HtmlElement>()
        .expect("element should be HtmlElement")
}

fn selected_tab_text(parent: &web_sys::HtmlElement) -> String {
    parent
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement")
        .text_content()
        .unwrap_or_default()
}

fn selected_panel_text(parent: &web_sys::HtmlElement) -> String {
    parent
        .query_selector(r#"[role="tabpanel"]:not([hidden])"#)
        .expect("query should succeed")
        .expect("a tab panel should be selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("panel is HtmlElement")
        .text_content()
        .unwrap_or_default()
}

fn tab_at(parent: &web_sys::HtmlElement, index: u32) -> web_sys::HtmlElement {
    first_with_data_part(parent, "list")
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(index)
        .unwrap_or_else(|| panic!("tab at index {index} should exist"))
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement")
}

fn active_element_text() -> String {
    document()
        .active_element()
        .expect("document should have an active element")
        .text_content()
        .unwrap_or_default()
}

fn has_focus_visible(tab: &web_sys::HtmlElement) -> bool {
    tab.has_attribute("data-ars-focus-visible")
}

#[wasm_bindgen_test(async)]
async fn link_click_prevents_default_and_emits_value_change() {
    let owner = Owner::new();

    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let link_tabs = vec![
            Tab::new_with_label(
                "home",
                "Home",
                ViewFn::from(|| view! { "Home" }),
                ViewFn::from(|| view! { <p>"Home panel"</p> }),
            )
            .link(SafeUrl::from_static("/home")),
            Tab::new_with_label(
                "settings",
                "Settings",
                ViewFn::from(|| view! { "Settings" }),
                ViewFn::from(|| view! { <p>"Settings panel"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        default_value="settings"
                        tabs=store_field(link_tabs)
                        on_value_change=Callback::new({
                            let selected = Arc::clone(&selected);
                            move |key| {
                                selected
                                    .lock()
                                    .expect("selected callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let anchor = parent
        .query_selector(r#"a[role="tab"][href="/home"]"#)
        .expect("query should succeed")
        .expect("link tab should render as anchor")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("anchor is HtmlElement");

    let event = cancelable_click(&anchor);

    leptos::task::tick().await;

    assert!(
        event.default_prevented(),
        "link tab click should be canceled so browser navigation does not run"
    );

    assert_eq!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .as_slice(),
        &[Some("home")],
        "selecting the link tab should emit the committed selected key"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn linked_close_trigger_click_prevents_default_navigation() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let link_tabs = vec![
            Tab::new_with_label(
                "home",
                "Home",
                ViewFn::from(|| view! { "Home" }),
                ViewFn::from(|| view! { <p>"Home panel"</p> }),
            )
            .link(SafeUrl::from_static("/home"))
            .closable(true),
            Tab::new_with_label(
                "settings",
                "Settings",
                ViewFn::from(|| view! { "Settings" }),
                ViewFn::from(|| view! { <p>"Settings panel"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            move || {
                view! {
                    <Tabs
                        default_value="settings"
                        tabs=store_field(link_tabs)
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    assert!(
        parent
            .query_selector(r#"a[role="tab"][href="/home"] [data-ars-part="tab-close-trigger"]"#)
            .expect("nested close query should succeed")
            .is_none(),
        "linked close trigger must not be nested inside the anchor tab"
    );

    let close = parent
        .query_selector(r#"a[role="tab"][href="/home"] + [data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("linked close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    let event = cancelable_click(&close);

    leptos::task::tick().await;

    assert!(
        event.default_prevented(),
        "linked close trigger click should cancel browser navigation"
    );
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["home"]
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn manual_link_tabs_activate_from_keyboard_without_browser_navigation() {
    let owner = Owner::new();

    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let link_tabs = vec![
            Tab::new_with_label(
                "home",
                "Home",
                ViewFn::from(|| view! { "Home" }),
                ViewFn::from(|| view! { <p>"Home panel"</p> }),
            )
            .link(SafeUrl::from_static("/home")),
            Tab::new_with_label(
                "settings",
                "Settings",
                ViewFn::from(|| view! { "Settings" }),
                ViewFn::from(|| view! { <p>"Settings panel"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        default_value="settings"
                        tabs=store_field(link_tabs)
                        activation_mode=ActivationMode::Manual
                        on_value_change=Callback::new({
                            let selected = Arc::clone(&selected);
                            move |key| {
                                selected
                                    .lock()
                                    .expect("selected callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let anchor = parent
        .query_selector(r#"a[role="tab"][href="/home"]"#)
        .expect("query should succeed")
        .expect("link tab should render as anchor")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("anchor is HtmlElement");

    anchor.focus().expect("focus should succeed");

    let enter = dispatch_keydown(&anchor, "Enter", false);

    leptos::task::tick().await;

    assert!(
        enter.default_prevented(),
        "manual link tab Enter should select without native navigation"
    );
    assert_eq!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .as_slice(),
        &[Some("home")]
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn controlled_click_emits_value_change_without_mutating_controlled_selection() {
    let owner = Owner::new();

    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let (value, _set_value) = signal::<Option<&'static str>>(Some("first"));
        let value_signal: Signal<Option<&'static str>> = value.into();

        let mount_handle = mount_to(parent.clone(), {
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        value=value_signal
                        default_value="first"
                        tabs=store_field(three_tabs())
                        on_value_change=Callback::new({
                            let selected = Arc::clone(&selected);
                            move |key| {
                                selected
                                    .lock()
                                    .expect("selected callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let second = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(1)
        .expect("second tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    click(&second);

    leptos::task::tick().await;

    assert_eq!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .as_slice(),
        &[Some("second")],
        "controlled Tabs should emit the requested selection even when the prop is unchanged"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "controlled selection should still be owned by the value prop"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn controlled_arrow_emits_value_change_without_mutating_controlled_selection() {
    let owner = Owner::new();

    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let (value, _set_value) = signal::<Option<&'static str>>(Some("first"));
        let value_signal: Signal<Option<&'static str>> = value.into();

        let mount_handle = mount_to(parent.clone(), {
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        value=value_signal
                        default_value="first"
                        tabs=store_field(three_tabs())
                        on_value_change=Callback::new({
                            let selected = Arc::clone(&selected);
                            move |key| {
                                selected
                                    .lock()
                                    .expect("selected callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = first_with_data_part(&parent, "list")
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    assert_eq!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .as_slice(),
        &[Some("second")],
        "controlled Tabs should emit automatic keyboard selection intent"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "controlled selection should still be owned by the value prop"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn close_callback_can_remove_tab_from_store() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let store = store_handle(
            three_tabs()
                .into_iter()
                .map(|tab| {
                    if tab.key == "first" {
                        tab.closable(true)
                    } else {
                        tab
                    }
                })
                .collect(),
        );
        let tabs_for_close = store.tabs();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=store.tabs()
                    on_close_tab=Callback::new(move |key: &'static str| {
                        tabs_for_close.write().retain(|tab| tab.key != key);
                    })
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        2,
        "close callback should be able to mutate the tab store"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn keyboard_close_of_focused_unselected_tab_preserves_manual_selection() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let store = store_handle(
            three_tabs()
                .into_iter()
                .map(|tab| {
                    if tab.key == "second" {
                        tab.closable(true)
                    } else {
                        tab
                    }
                })
                .collect(),
        );

        let tabs_for_close = store.tabs();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=store.tabs()
                    activation_mode=ActivationMode::Manual
                    on_close_tab=Callback::new(move |key: &'static str| {
                        tabs_for_close.write().retain(|tab| tab.key != key);
                    })
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let second_tab = tab_at(&parent, 1);

    second_tab.focus().expect("focus should succeed");

    dispatch_keydown(&second_tab, "Delete", false);

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    assert_eq!(
        tablist
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        2,
        "close callback should remove the focused unselected tab"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "closing a focused unselected tab in manual mode must preserve selection"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn close_request_respects_disallow_empty_selection_for_external_store() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let store = store_handle(vec![
            Tab::new_with_label(
                "only",
                "Only",
                ViewFn::from(|| view! { "Only" }),
                ViewFn::from(|| view! { <p>"Only panel"</p> }),
            )
            .closable(true),
        ]);
        let tabs_for_close = store.tabs();
        let closed = Arc::clone(&closed);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="only"
                    tabs=store.tabs()
                    disallow_empty_selection=true
                    on_close_tab=Callback::new({
                        let closed = Arc::clone(&closed);
                        move |key: &'static str| {
                            closed
                                .lock()
                                .expect("close callback log should not be poisoned")
                                .push(key);
                            tabs_for_close.write().retain(|tab| tab.key != key);
                        }
                    })
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        1,
        "external close callback must not remove the final tab"
    );
    assert!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .is_empty(),
        "blocked close requests must not call on_close_tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn close_request_reads_disallow_empty_selection_at_close_time_for_owned_tabs() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent, set_disallow_empty_selection) = owner.with(|| {
        let parent = container();
        let (disallow_empty_selection, set_disallow_empty_selection) = signal(true);
        let disallow_empty_selection: Signal<bool> = disallow_empty_selection.into();
        let closed = Arc::clone(&closed);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="only"
                    tabs=[
                        Tab::new_with_label(
                                "only",
                                "Only",
                                ViewFn::from(|| view! { "Only" }),
                                ViewFn::from(|| view! { <p>"Only panel"</p> }),
                            )
                            .closable(true),
                    ]
                    disallow_empty_selection=disallow_empty_selection
                    on_close_tab=Callback::new({
                        let closed = Arc::clone(&closed);
                        move |key: &'static str| {
                            closed
                                .lock()
                                .expect("close callback log should not be poisoned")
                                .push(key);
                        }
                    })
                />
            }
        });

        (mount_handle, parent, set_disallow_empty_selection)
    });

    leptos::task::tick().await;

    set_disallow_empty_selection.set(false);

    leptos::task::tick().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        0,
        "the owned-store close guard should use the current disallow_empty_selection value"
    );
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        ["only"],
        "allowed owned closes should still emit the close callback"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn inline_array_close_trigger_removes_owned_tab() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="second"
                    tabs=[
                        Tab::new_with_label(
                            "first",
                            "First",
                            ViewFn::from(|| view! { "First" }),
                            ViewFn::from(|| view! { <p>"Panel one"</p> }),
                        ),
                        Tab::new_with_label(
                                "second",
                                "Second",
                                ViewFn::from(|| view! { "Second" }),
                                ViewFn::from(|| view! { <p>"Panel two"</p> }),
                            )
                            .closable(true),
                        Tab::new_with_label(
                            "third",
                            "Third",
                            ViewFn::from(|| view! { "Third" }),
                            ViewFn::from(|| view! { <p>"Panel three"</p> }),
                        ),
                    ]
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        2,
        "inline array tabs should remove through the adapter-owned store"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "Third",
        "closing the selected owned tab should select the successor"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn keyboard_close_removing_selected_tab_focuses_successor() {
    let owner = Owner::new();
    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let store = store_handle(
            three_tabs()
                .into_iter()
                .map(|tab| {
                    if tab.key == "first" {
                        tab.closable(true)
                    } else {
                        tab
                    }
                })
                .collect(),
        );
        let tabs_for_close = store.tabs();

        let mount_handle = mount_to(parent.clone(), {
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store.tabs()
                        on_close_tab=Callback::new(move |key: &'static str| {
                            tabs_for_close.write().retain(|tab| tab.key != key);
                        })
                        on_value_change=Callback::new(move |key| {
                            selected
                                .lock()
                                .expect("selected callback log should not be poisoned")
                                .push(key);
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "Delete", false);

    leptos::task::tick().await;

    deferred_focus_turn().await;

    assert_eq!(
        selected_tab_text(&parent),
        "Second",
        "removing the selected tab should snap selection to the next enabled tab"
    );
    assert_eq!(
        active_element_text(),
        "Second",
        "keyboard close should keep DOM focus on the successor tab"
    );
    assert_eq!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .as_slice(),
        &[Some("second")],
        "closing the selected tab must notify controlled parents about successor selection"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn close_and_reorder_callbacks_fire_from_user_events() {
    let owner = Owner::new();

    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            )
            .closable(true),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(tabs)
                        reorderable=true
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                true
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let first = tablist
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    leptos::task::tick().await;

    let event = dispatch_keydown(&first, "ArrowRight", true);

    leptos::task::tick().await;

    dispatch_keydown(&first, "Delete", false);

    leptos::task::tick().await;

    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first"]
    );
    assert_eq!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .as_slice(),
        &[TestReorderEvent {
            key: "first",
            old_index: 0,
            new_index: 1,
        }]
    );
    assert!(
        event.default_prevented(),
        "recognized reorder shortcuts should be canceled even when vetoed"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn drag_and_drop_reorders_tabs_through_callback() {
    let owner = Owner::new();

    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let store = store_handle(three_tabs());

        let mount_handle = mount_to(parent.clone(), {
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store.tabs()
                        reorderable=true
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event: TestReorderEvent| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event.clone());
                                let tabs_field = store.tabs();
                                let mut tabs = tabs_field.write();
                                let tab = tabs.remove(event.old_index);
                                tabs.insert(event.new_index, tab);
                                true
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let tabs = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed");

    let first = tabs
        .item(0)
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("first tab is HtmlElement");

    let third = tabs
        .item(2)
        .expect("third tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("third tab is HtmlElement");

    first.focus().expect("focus should succeed");

    dispatch_drag_event(&first, "dragstart");

    let dragover = dispatch_drag_event(&third, "dragover");
    let drop_event = dispatch_drag_event(&third, "drop");

    leptos::task::tick().await;

    deferred_focus_turn().await;

    assert!(dragover.default_prevented(), "dragover should allow drop");
    assert!(
        drop_event.default_prevented(),
        "drop should cancel browser default"
    );
    assert_eq!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .as_slice(),
        &[TestReorderEvent {
            key: "first",
            old_index: 0,
            new_index: 2,
        }]
    );

    let labels = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed");

    assert_eq!(
        labels
            .item(2)
            .expect("third rendered tab should exist")
            .text_content()
            .unwrap_or_default(),
        "First",
        "dropping the first tab on the third tab should move it to index 2"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "drag reorder should preserve selected tab identity"
    );
    assert_eq!(
        active_element_text(),
        "First",
        "drag reorder should preserve DOM focus on the dragged tab"
    );

    let announcement = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("reorder live region should exist")
        .text_content()
        .unwrap_or_default();

    assert!(
        announcement.contains("position 3 of 3"),
        "drag reorder should announce the committed position, got {announcement:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn inline_array_drag_and_drop_reorders_owned_tabs() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=[
                        Tab::new_with_label(
                            "first",
                            "First",
                            ViewFn::from(|| view! { "First" }),
                            ViewFn::from(|| view! { <p>"Panel one"</p> }),
                        ),
                        Tab::new_with_label(
                            "second",
                            "Second",
                            ViewFn::from(|| view! { "Second" }),
                            ViewFn::from(|| view! { <p>"Panel two"</p> }),
                        ),
                        Tab::new_with_label(
                            "third",
                            "Third",
                            ViewFn::from(|| view! { "Third" }),
                            ViewFn::from(|| view! { <p>"Panel three"</p> }),
                        ),
                    ]
                    reorderable=true
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let tabs = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed");

    let first = tabs
        .item(0)
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("first tab is HtmlElement");

    let third = tabs
        .item(2)
        .expect("third tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("third tab is HtmlElement");

    dispatch_drag_event(&first, "dragstart");

    let drop_event = dispatch_drag_event(&third, "drop");

    leptos::task::tick().await;

    assert!(
        drop_event.default_prevented(),
        "owned drag reorder should cancel browser default"
    );

    let labels = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed");

    assert_eq!(
        labels
            .item(2)
            .expect("third rendered tab should exist")
            .text_content()
            .unwrap_or_default(),
        "First",
        "inline array tabs should reorder through the adapter-owned store"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn drag_and_drop_ignores_missing_same_or_disabled_targets() {
    let owner = Owner::new();

    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            ),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            )
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                ViewFn::from(|| view! { "Third" }),
                ViewFn::from(|| view! { <p>"Panel three"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(tabs)
                        reorderable=true
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                true
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);
    let disabled_second = tab_at(&parent, 1);
    let third = tab_at(&parent, 2);

    let missing_source_drop = dispatch_drag_event(&third, "drop");
    assert!(
        !missing_source_drop.default_prevented(),
        "drop without a drag source should fall through"
    );

    dispatch_drag_event(&first, "dragstart");

    let same_target_dragover = dispatch_drag_event(&first, "dragover");
    let same_target_drop = dispatch_drag_event(&first, "drop");

    assert!(
        !same_target_dragover.default_prevented(),
        "dragover on the source tab should not accept a drop"
    );
    assert!(
        !same_target_drop.default_prevented(),
        "drop on the source tab should fall through"
    );

    dispatch_drag_event(&first, "dragstart");

    let disabled_dragover = dispatch_drag_event(&disabled_second, "dragover");
    let disabled_drop = dispatch_drag_event(&disabled_second, "drop");

    assert!(
        !disabled_dragover.default_prevented(),
        "dragover on a disabled target should not accept a drop"
    );
    assert!(
        !disabled_drop.default_prevented(),
        "drop on a disabled target should fall through"
    );

    dispatch_drag_event(&first, "dragstart");
    dispatch_drag_event(&first, "dragend");

    let canceled_dragover = dispatch_drag_event(&third, "dragover");
    let canceled_drop = dispatch_drag_event(&third, "drop");

    assert!(
        !canceled_dragover.default_prevented(),
        "dragover should fall through after dragend clears the source tab"
    );
    assert!(
        !canceled_drop.default_prevented(),
        "drop should fall through after dragend clears the source tab"
    );

    assert!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .is_empty(),
        "invalid drag/drop attempts must not call on_reorder"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn drag_and_drop_reorder_veto_suppresses_drop_commit() {
    let owner = Owner::new();

    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), {
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(three_tabs())
                        reorderable=true
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                false
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);
    let third = tab_at(&parent, 2);

    dispatch_drag_event(&first, "dragstart");

    let drop_event = dispatch_drag_event(&third, "drop");

    leptos::task::tick().await;

    assert!(
        drop_event.default_prevented(),
        "recognized drop should cancel browser default even when vetoed"
    );
    assert_eq!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .as_slice(),
        &[TestReorderEvent {
            key: "first",
            old_index: 0,
            new_index: 2,
        }]
    );
    assert_eq!(
        tab_at(&parent, 0).text_content().unwrap_or_default(),
        "First",
        "vetoed drag reorder must not commit a DOM reorder"
    );
    assert_eq!(
        tab_at(&parent, 2).text_content().unwrap_or_default(),
        "Third",
        "vetoed drag reorder must leave the target tab in place"
    );

    let live_region = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("live region should render when reorderable");

    assert_eq!(
        live_region.text_content().unwrap_or_default(),
        "",
        "vetoed drag reorder must not announce a committed move"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn reorder_veto_suppresses_core_event_and_live_announcement() {
    let owner = Owner::new();

    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), {
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(three_tabs())
                        reorderable=true
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                false
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = first_with_data_part(&parent, "list")
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&first, "ArrowRight", true);

    leptos::task::tick().await;

    assert_eq!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .as_slice(),
        &[TestReorderEvent {
            key: "first",
            old_index: 0,
            new_index: 1,
        }]
    );

    let live_region = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("live region should render when reorderable");

    assert_eq!(
        live_region.text_content().unwrap_or_default(),
        "",
        "vetoed reorder must not announce a committed move"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn external_store_reorder_without_callback_does_not_announce_commit() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) reorderable=true /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");
    let first = tab_at(&parent, 0);
    let live_region = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("live region should render when reorderable");

    dispatch_keydown(&first, "ArrowRight", true);

    leptos::task::tick().await;

    assert_eq!(
        tablist
            .query_selector(r#"[role="tab"]"#)
            .expect("query should succeed")
            .expect("first tab should still render")
            .text_content()
            .unwrap_or_default(),
        "First",
        "external stores without on_reorder must not report a committed DOM reorder"
    );
    assert_eq!(
        live_region.text_content().unwrap_or_default(),
        "",
        "uncommitted external reorders must not announce a move"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_tracks_selected_tab_measurement() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        parent
            .set_attribute(
                "style",
                "position: relative; display: block; width: 400px; height: 200px;",
            )
            .expect("style should set");

        add_indicator_measurement_styles(&parent);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider platform=Arc::new(ars_dom::WebPlatformEffects)>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    let style = indicator
        .get_attribute("style")
        .expect("indicator should receive measurement style");

    assert!(
        style.contains("--ars-indicator-width:"),
        "indicator style should contain measured width: {style}"
    );
    assert!(
        style.contains("--ars-indicator-height:"),
        "indicator style should contain measured height: {style}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_refreshes_after_owned_reorder_of_selected_tab() {
    let owner = Owner::new();
    let measurement_calls = Arc::new(AtomicU32::new(0));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let platform: Arc<dyn PlatformEffects> = Arc::new(MeasurementProbePlatform {
            calls: Arc::clone(&measurement_calls),
        });

        parent
            .set_attribute(
                "style",
                "position: relative; display: block; width: 400px; height: 200px;",
            )
            .expect("style should set");

        add_indicator_measurement_styles(&parent);

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider platform>
                    <Tabs default_value="first" tabs=three_tabs() reorderable=true />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;
    animation_frame_turn().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");
    let before = indicator.get_attribute("style").unwrap_or_default();

    dispatch_keydown(&tab_at(&parent, 0), "ArrowRight", true);

    leptos::task::tick().await;
    animation_frame_turn().await;

    let after = indicator.get_attribute("style").unwrap_or_default();

    assert!(
        before != after && after.contains("--ars-indicator-left:"),
        "committed selected-tab reorder should refresh indicator measurement: before={before:?}, after={after:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_refreshes_after_selected_tab_label_changes() {
    let owner = Owner::new();
    let measurement_calls = Arc::new(AtomicU32::new(0));

    let (mount_handle, parent, tabs_for_update) = owner.with(|| {
        let parent = container();
        let platform: Arc<dyn PlatformEffects> = Arc::new(MeasurementProbePlatform {
            calls: Arc::clone(&measurement_calls),
        });

        parent
            .set_attribute(
                "style",
                "position: relative; display: block; width: 500px; height: 200px;",
            )
            .expect("style should set");
        add_indicator_measurement_styles(&parent);

        let store = store_handle(three_tabs());

        let tabs_for_update = store.tabs();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider platform>
                    <Tabs default_value="first" tabs=store.tabs() />
                </ArsProvider>
            }
        });

        (mount_handle, parent, tabs_for_update)
    });

    leptos::task::tick().await;

    animation_frame_turn().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    let before = indicator.get_attribute("style").unwrap_or_default();

    tabs_for_update.write()[0] = Tab::new_with_label(
        "first",
        "First selected tab with longer label",
        ViewFn::from(|| view! { "First selected tab with longer label" }),
        ViewFn::from(|| view! { <p>"Panel one"</p> }),
    );

    leptos::task::tick().await;

    animation_frame_turn().await;

    let after = indicator.get_attribute("style").unwrap_or_default();

    assert!(
        before != after && after.contains("--ars-indicator-width:"),
        "selected-tab label changes should refresh indicator measurement: before={before:?}, after={after:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_refreshes_after_signal_backed_orientation_changes() {
    let owner = Owner::new();

    let (mount_handle, parent, set_orientation) = owner.with(|| {
        let parent = container();

        parent
            .set_attribute(
                "style",
                "position: relative; display: block; width: 500px; height: 300px;",
            )
            .expect("style should set");

        add_indicator_measurement_styles(&parent);

        let (orientation, set_orientation) = signal(Orientation::Horizontal);

        let orientation: Signal<Orientation> = orientation.into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider platform=Arc::new(ars_dom::WebPlatformEffects)>
                    <Tabs default_value="second" tabs=store_field(three_tabs()) orientation />
                </ArsProvider>
            }
        });

        (mount_handle, parent, set_orientation)
    });

    leptos::task::tick().await;

    animation_frame_turn().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    let before = indicator.get_attribute("style").unwrap_or_default();

    set_orientation.set(Orientation::Vertical);

    leptos::task::tick().await;

    animation_frame_turn().await;

    let after = indicator.get_attribute("style").unwrap_or_default();

    assert!(
        before != after && after.contains("--ars-indicator-top:"),
        "signal-backed orientation changes should remeasure indicator layout: before={before:?}, after={after:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_refreshes_after_selected_trigger_visual_content_resizes() {
    let owner = Owner::new();

    let (mount_handle, parent, set_expanded) = owner.with(|| {
        let parent = container();

        parent
            .set_attribute(
                "style",
                "position: relative; display: block; width: 600px; height: 240px;",
            )
            .expect("style should set");

        add_indicator_measurement_styles(&parent);

        let (expanded, set_expanded) = signal(false);

        let mount_handle = mount_to(parent.clone(), move || {
            let trigger = ViewFn::from(move || {
                view! {
                    <span>
                        {move || {
                            if expanded.get() {
                                "First trigger with expanded visual count 100"
                            } else {
                                "First"
                            }
                        }}
                    </span>
                }
            });

            let tabs = vec![
                Tab::new_with_label(
                    "first",
                    "First",
                    trigger,
                    ViewFn::from(|| view! { <p>"Panel one"</p> }),
                ),
                Tab::new_with_label(
                    "second",
                    "Second",
                    ViewFn::from(|| view! { "Second" }),
                    ViewFn::from(|| view! { <p>"Panel two"</p> }),
                ),
            ];

            view! {
                <ArsProvider platform=Arc::new(ars_dom::WebPlatformEffects)>
                    <Tabs default_value="first" tabs />
                </ArsProvider>
            }
        });

        (mount_handle, parent, set_expanded)
    });

    leptos::task::tick().await;

    animation_frame_turn().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    let before = indicator.get_attribute("style").unwrap_or_default();

    set_expanded.set(true);

    leptos::task::tick().await;

    animation_frame_turn().await;
    animation_frame_turn().await;

    let after = indicator.get_attribute("style").unwrap_or_default();

    assert!(
        before != after && after.contains("--ars-indicator-width:"),
        "selected trigger visual-only size changes should remeasure indicator layout: before={before:?}, after={after:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn indicator_style_degrades_to_empty_without_geometry() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    assert!(
        indicator
            .get_attribute("style")
            .is_none_or(|style| style.is_empty()),
        "indicator should not keep stale measurement styles when geometry is unavailable"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn arrow_keys_move_selection_in_automatic_mode() {
    let owner = Owner::new();

    let (mount_handle, parent, _store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let initial_selected = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected by default")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        initial_selected.text_content().unwrap_or_default(),
        "First",
        "default selection should be first tab"
    );

    // Simulate the user tabbing into the tablist before keyboard
    // navigation: the agnostic Tabs core's `(Idle, FocusNext)` arm
    // bootstraps focus to the currently selected tab without advancing.
    // A real user pressing ArrowRight already had focus on a tab, so
    // we replicate that by calling .focus() first.
    initial_selected.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&initial_selected, "ArrowRight", false);

    leptos::task::tick().await;

    let after_right = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected after ArrowRight")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        after_right.text_content().unwrap_or_default(),
        "Second",
        "ArrowRight should advance selection in automatic mode"
    );
    assert_eq!(
        active_element_text(),
        "Second",
        "ArrowRight should move DOM focus to the newly selected tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn arrow_left_wraps_to_last_when_loop_focus_is_default() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let first = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("first tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&first, "ArrowLeft", false);

    leptos::task::tick().await;

    let wrapped = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected after wrap")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        wrapped.text_content().unwrap_or_default(),
        "Third",
        "ArrowLeft should wrap to the last tab when loop_focus is on (default)"
    );
    assert_eq!(
        active_element_text(),
        "Third",
        "ArrowLeft should move DOM focus to the wrapped tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn home_and_end_jump_to_first_and_last_tabs() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="second" tabs=store_field(three_tabs()) /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let initial = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("default selected tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    initial.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&initial, "End", false);

    leptos::task::tick().await;

    let last = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected after End")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(last.text_content().unwrap_or_default(), "Third");
    assert_eq!(
        active_element_text(),
        "Third",
        "End should move DOM focus to the last tab"
    );

    dispatch_keydown(&last, "Home", false);

    leptos::task::tick().await;

    let first = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should be selected after Home")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(first.text_content().unwrap_or_default(), "First");
    assert_eq!(
        active_element_text(),
        "First",
        "Home should move DOM focus to the first tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn automatic_hotkeys_skip_disabled_tabs_and_support_vertical_axis() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            ),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            )
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                ViewFn::from(|| view! { "Third" }),
                ViewFn::from(|| view! { <p>"Panel three"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(tabs) orientation=Orientation::Vertical /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    leptos::task::tick().await;

    let ignored_horizontal = dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    assert!(
        !ignored_horizontal.default_prevented(),
        "horizontal arrows should be ignored in vertical orientation"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "ignored horizontal arrow must not change selection"
    );

    let down = dispatch_keydown(&first, "ArrowDown", false);

    leptos::task::tick().await;

    assert!(
        down.default_prevented(),
        "vertical ArrowDown should be handled"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "Third",
        "ArrowDown should skip disabled tabs"
    );
    assert_eq!(
        active_element_text(),
        "Third",
        "ArrowDown should move DOM focus to the enabled target"
    );

    let third = tab_at(&parent, 2);

    let up = dispatch_keydown(&third, "ArrowUp", false);

    leptos::task::tick().await;

    assert!(up.default_prevented(), "vertical ArrowUp should be handled");
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "ArrowUp should skip disabled tabs while moving backward"
    );
    assert_eq!(
        active_element_text(),
        "First",
        "ArrowUp should move DOM focus back to the enabled target"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn manual_activation_accepts_enter_and_space_hotkeys() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=store_field(three_tabs())
                    activation_mode=ActivationMode::Manual
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let arrow = dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    assert!(arrow.default_prevented(), "ArrowRight should move focus");
    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "manual mode should move focus without selecting"
    );
    assert_eq!(
        active_element_text(),
        "Second",
        "manual arrow navigation should move DOM focus without selecting"
    );

    let second = tab_at(&parent, 1);

    let space = dispatch_keydown(&second, " ", false);

    leptos::task::tick().await;

    assert!(space.default_prevented(), "Space should select focused tab");
    assert_eq!(selected_tab_text(&parent), "Second");

    let third = tab_at(&parent, 2);

    third.focus().expect("focus should succeed");

    let enter = dispatch_keydown(&third, "Enter", false);

    leptos::task::tick().await;

    assert!(enter.default_prevented(), "Enter should select focused tab");
    assert_eq!(selected_tab_text(&parent), "Third");

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn tab_key_entry_target_tracks_selected_roving_tabindex() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).get_attribute("tabindex").as_deref(),
        Some("0")
    );
    assert_eq!(
        tab_at(&parent, 1).get_attribute("tabindex").as_deref(),
        Some("-1")
    );
    assert_eq!(
        tab_at(&parent, 2).get_attribute("tabindex").as_deref(),
        Some("-1")
    );

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).get_attribute("tabindex").as_deref(),
        Some("-1"),
        "previously selected tab should leave the Tab-key order"
    );
    assert_eq!(
        tab_at(&parent, 1).get_attribute("tabindex").as_deref(),
        Some("0"),
        "newly selected tab should be the only Tab-key entry target"
    );
    assert_eq!(
        tab_at(&parent, 2).get_attribute("tabindex").as_deref(),
        Some("-1")
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn browser_tabbable_order_tracks_only_the_selected_roving_tab() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        parent.set_id("leptos-tabs-tabbable-root");

        let before = document()
            .create_element("button")
            .expect("before sentinel should be created")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("before sentinel should be HtmlElement");

        before.set_id("leptos-tabs-before");

        before.set_text_content(Some("before"));

        parent
            .append_child(&before)
            .expect("before sentinel should attach");

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) /> }
        });

        let after = document()
            .create_element("button")
            .expect("after sentinel should be created")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("after sentinel should be HtmlElement");

        after.set_id("leptos-tabs-after");

        after.set_text_content(Some("after"));

        parent
            .append_child(&after)
            .expect("after sentinel should attach");

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let platform = ars_dom::WebPlatformEffects;

    let first = tab_at(&parent, 0);
    let second = tab_at(&parent, 1);
    let third = tab_at(&parent, 2);

    let first_panel = parent
        .query_selector(r#"[role="tabpanel"]:not([hidden])"#)
        .expect("query should succeed")
        .expect("selected panel should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("panel should be HtmlElement");

    let order = platform.tabbable_element_ids("leptos-tabs-tabbable-root");

    assert_eq!(
        order,
        vec![
            "leptos-tabs-before".to_owned(),
            first.id(),
            first_panel.id(),
            "leptos-tabs-after".to_owned(),
        ],
        "browser tabbable order should include the selected roving tab and selected panel"
    );
    assert!(
        !order.contains(&second.id()) && !order.contains(&third.id()),
        "inactive tabs must not be browser Tab-key targets"
    );

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    animation_frame_turn().await;

    let order = platform.tabbable_element_ids("leptos-tabs-tabbable-root");

    let second_panel = parent
        .query_selector(r#"[role="tabpanel"]:not([hidden])"#)
        .expect("query should succeed")
        .expect("selected panel should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("panel should be HtmlElement");

    assert_eq!(
        order,
        vec![
            "leptos-tabs-before".to_owned(),
            second.id(),
            second_panel.id(),
            "leptos-tabs-after".to_owned(),
        ],
        "after selection changes, native Tab entry should move to the selected tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn focus_visible_tracks_keyboard_and_pointer_modality() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    let second = tab_at(&parent, 1);

    assert!(
        has_focus_visible(&second),
        "keyboard focus should render data-ars-focus-visible on the focused tab"
    );

    let third = tab_at(&parent, 2);

    dispatch_pointerdown(&third, "mouse");

    click(&third);

    third.focus().expect("focus should succeed");

    leptos::task::tick().await;

    assert_eq!(selected_tab_text(&parent), "Third");
    assert!(
        !has_focus_visible(&third),
        "pointer focus should not render keyboard focus-visible state"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn pointerdown_on_focused_tab_clears_focus_visible_without_selection_change() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    let second = tab_at(&parent, 1);

    assert!(
        has_focus_visible(&second),
        "keyboard navigation should make the focused tab focus-visible"
    );

    dispatch_pointerdown(&second, "mouse");

    leptos::task::tick().await;

    assert!(
        !has_focus_visible(&second),
        "pointerdown on the already focused tab should clear stale focus-visible state"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn keyboard_focus_dispatch_prefers_node_ref_over_id_platform_focus() {
    let owner = Owner::new();

    let focus_by_id_calls = Arc::new(Mutex::new(Vec::<String>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();
        let platform = Arc::new(FocusByIdProbePlatform {
            calls: Arc::clone(&focus_by_id_calls),
        });

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider platform=platform>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    animation_frame_turn().await;

    let second = tab_at(&parent, 1);

    let active = document()
        .active_element()
        .expect("document should have an active element");

    assert_eq!(
        active,
        second.clone().unchecked_into::<web_sys::Element>(),
        "keyboard focus dispatch should focus the next tab element"
    );
    assert!(
        focus_by_id_calls
            .lock()
            .expect("focus call log should not be poisoned")
            .is_empty(),
        "Leptos tabs should prefer the live NodeRef path before platform ID focus"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn pointerdown_modality_supports_mouse_touch_and_pen() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    let second = tab_at(&parent, 1);

    assert!(has_focus_visible(&second));

    let third = tab_at(&parent, 2);

    for pointer_type in ["mouse", "touch", "pen"] {
        dispatch_pointerdown(&third, pointer_type);
        click(&third);
        third.focus().expect("focus should succeed");

        leptos::task::tick().await;

        assert!(
            !has_focus_visible(&third),
            "{pointer_type} pointerdown should clear keyboard focus-visible state"
        );

        dispatch_keydown(&third, "ArrowLeft", false);

        leptos::task::tick().await;

        assert!(
            has_focus_visible(&second),
            "keyboard navigation should restore focus-visible before the next pointer variant"
        );
    }

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn virtual_click_preserves_keyboard_focus_visible_modality() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <ArsProvider>
                    <Tabs default_value="first" tabs=store_field(three_tabs()) />
                </ArsProvider>
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    leptos::task::tick().await;

    let third = tab_at(&parent, 2);

    click(&third);

    third.focus().expect("focus should succeed");

    leptos::task::tick().await;

    assert_eq!(selected_tab_text(&parent), "Third");
    assert!(
        has_focus_visible(&third),
        "click activation without a preceding pointerdown should keep keyboard/virtual modality"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn click_selects_tab_and_updates_panel_visibility() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let second_tab = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(1)
        .expect("second tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    click(&second_tab);

    leptos::task::tick().await;

    let selected = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab is selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(selected.text_content().unwrap_or_default(), "Second");

    let panels = parent
        .query_selector_all(r#"[role="tabpanel"]"#)
        .expect("query should succeed");

    let mut visible_count = 0_u32;

    for index in 0..panels.length() {
        let panel = panels
            .item(index)
            .expect("panel exists")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("panel is HtmlElement");

        if panel.get_attribute("hidden").is_none() {
            visible_count += 1;
        }
    }

    assert_eq!(
        visible_count, 1,
        "exactly one panel should be visible after click"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn manual_activation_mode_separates_focus_from_selection() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=store_field(three_tabs())
                    activation_mode=ActivationMode::Manual
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let first_tab = tablist
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first_tab.focus().expect("focus should succeed");

    leptos::task::tick().await;

    dispatch_keydown(&first_tab, "ArrowRight", false);

    leptos::task::tick().await;

    let still_first_selected = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab is selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        still_first_selected.text_content().unwrap_or_default(),
        "First",
        "manual mode should NOT advance selection on arrow keys"
    );

    // Now press Enter on the focused tab — selection should commit.
    let second_tab = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(1)
        .expect("second tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    dispatch_keydown(&second_tab, "Enter", false);

    leptos::task::tick().await;

    let after_enter = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab is selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        after_enter.text_content().unwrap_or_default(),
        "Second",
        "Enter in manual mode should commit selection"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn ctrl_arrow_announces_reorder_in_polite_live_region() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=three_tabs() reorderable=true /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let live_region = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("reorder live region should exist when reorderable=true")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("live region is HtmlElement");

    assert_eq!(
        live_region.text_content().unwrap_or_default(),
        "",
        "live region should be silent on initial render"
    );

    let first_tab = tablist
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    dispatch_keydown(&first_tab, "ArrowRight", true);

    leptos::task::tick().await;

    let announcement = live_region.text_content().unwrap_or_default();

    assert!(
        announcement.contains("First"),
        "live announcement should reference the moved tab label, got {announcement:?}"
    );

    assert!(
        announcement.contains("position 2 of 3"),
        "live announcement should describe the new 1-based position, got {announcement:?}"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn closable_tab_dispatches_close_on_delete_key() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let closable_tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            )
            .closable(true),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            )
            .closable(true),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(closable_tabs.clone())
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    // The closable tab renders an embedded close button; verify it's not in
    // the roving order.
    let close_trigger = tablist
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("closable tab should render a close trigger")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    assert_eq!(close_trigger.tag_name(), "BUTTON");
    assert_eq!(
        close_trigger.get_attribute("tabindex").as_deref(),
        Some("-1"),
        "close trigger must NOT participate in roving tabindex"
    );

    // Press Delete on the focused closable tab.
    let first_tab = tablist
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    dispatch_keydown(&first_tab, "Delete", false);

    leptos::task::tick().await;

    let second_tab = tab_at(&parent, 1);

    second_tab.focus().expect("focus should succeed");

    dispatch_keydown(&second_tab, "Backspace", false);

    leptos::task::tick().await;

    // Markup remains unchanged because CloseTab is pure notification:
    // consumers commit removal via SetTabs themselves. We just verify the
    // initial state survived (no adapter-driven mutation).
    let visible = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .length();

    assert_eq!(
        visible, 2,
        "CloseTab is pure notification; adapter must not auto-remove tabs"
    );
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first", "second"],
        "Delete and Backspace should both emit close requests for closable tabs"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn repeated_and_composing_close_hotkeys_do_not_emit_close_requests() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let closable_tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            )
            .closable(true),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(closable_tabs.clone())
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let repeat = dispatch_keydown_with_options(&first, "Delete", false, true, false);
    let composing = dispatch_keydown_with_options(&first, "Delete", false, false, true);

    leptos::task::tick().await;

    assert!(
        !repeat.default_prevented(),
        "repeated close hotkeys should fall through"
    );
    assert!(
        !composing.default_prevented(),
        "composing close hotkeys should fall through"
    );
    assert!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .is_empty(),
        "repeat/composition close hotkeys must not emit close requests"
    );

    let normal = dispatch_keydown(&first, "Delete", false);

    leptos::task::tick().await;

    assert!(normal.default_prevented());
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first"]
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn repeated_and_composing_manual_activation_hotkeys_do_not_select() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="second"
                    tabs=store_field(three_tabs())
                    activation_mode=ActivationMode::Manual
                />
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let repeat = dispatch_keydown_with_options(&first, "Enter", false, true, false);
    let composing = dispatch_keydown_with_options(&first, "Enter", false, false, true);

    leptos::task::tick().await;

    assert!(
        !repeat.default_prevented(),
        "repeated manual activation should fall through"
    );
    assert!(
        !composing.default_prevented(),
        "composing manual activation should fall through"
    );
    assert_eq!(
        selected_tab_text(&parent),
        "Second",
        "repeat/composition activation hotkeys must not select"
    );

    let normal = dispatch_keydown(&first, "Enter", false);

    leptos::task::tick().await;

    assert!(normal.default_prevented());
    assert_eq!(selected_tab_text(&parent), "First");

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn close_trigger_click_does_not_select_its_tab() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            )
            .closable(true),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            move || {
                view! {
                    <Tabs
                        default_value="second"
                        tabs=store_field(tabs)
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    click(&first_with_data_part(&parent, "tab-close-trigger"));

    leptos::task::tick().await;

    assert_eq!(
        selected_tab_text(&parent),
        "Second",
        "clicking the embedded close trigger must not bubble into tab selection"
    );
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first"]
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn disabled_tabs_ignore_direct_click_close_key_and_reorder_shortcut() {
    let owner = Owner::new();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let tabs = vec![
            Tab::new_with_label(
                "first",
                "First",
                ViewFn::from(|| view! { "First" }),
                ViewFn::from(|| view! { <p>"Panel one"</p> }),
            ),
            Tab::new_with_label(
                "second",
                "Second",
                ViewFn::from(|| view! { "Second" }),
                ViewFn::from(|| view! { <p>"Panel two"</p> }),
            )
            .closable(true)
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                ViewFn::from(|| view! { "Third" }),
                ViewFn::from(|| view! { <p>"Panel three"</p> }),
            ),
        ];

        let mount_handle = mount_to(parent.clone(), {
            let closed = Arc::clone(&closed);
            let reordered = Arc::clone(&reordered);
            let selected = Arc::clone(&selected);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(tabs)
                        reorderable=true
                        on_close_tab=Callback::new({
                            let closed = Arc::clone(&closed);
                            move |key| {
                                closed
                                    .lock()
                                    .expect("close callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                true
                            }
                        })
                        on_value_change=Callback::new({
                            let selected = Arc::clone(&selected);
                            move |key| {
                                selected
                                    .lock()
                                    .expect("selected callback log should not be poisoned")
                                    .push(key);
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;

    let disabled_second = tab_at(&parent, 1);

    assert_eq!(
        disabled_second.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    assert!(
        disabled_second
            .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
            .expect("query should succeed")
            .is_none(),
        "disabled closable tabs must not render an active close trigger"
    );

    click(&disabled_second);

    leptos::task::tick().await;

    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "clicking a disabled tab must not select it"
    );

    disabled_second.focus().expect("focus should succeed");

    let delete = dispatch_keydown(&disabled_second, "Delete", false);
    let reorder = dispatch_keydown(&disabled_second, "ArrowRight", true);

    leptos::task::tick().await;

    assert!(
        !delete.default_prevented(),
        "disabled closable tabs should not consume close hotkeys"
    );
    assert!(
        !reorder.default_prevented(),
        "disabled tabs should not consume keyboard reorder shortcuts"
    );
    assert!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .is_empty(),
        "disabled tabs must not emit close requests"
    );
    assert!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .is_empty(),
        "disabled tabs must not emit reorder requests"
    );
    assert!(
        selected
            .lock()
            .expect("selected callback log should not be poisoned")
            .is_empty(),
        "disabled tabs must not emit value-change requests"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn controlled_value_signal_is_authoritative_for_selection() {
    let owner = Owner::new();

    let (mount_handle, parent, set_value) = owner.with(|| {
        let parent = container();
        let (value, set_value) = signal::<Option<&'static str>>(Some("first"));
        let value_signal: Signal<Option<&'static str>> = value.into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) value=value_signal /> }
        });

        (mount_handle, parent, set_value)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let initial = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("first tab selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(initial.text_content().unwrap_or_default(), "First");

    set_value.set(Some("third"));

    leptos::task::tick().await;

    let updated = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab should now be selected via controlled signal")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        updated.text_content().unwrap_or_default(),
        "Third",
        "controlled value signal should propagate to selection"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn store_pop_removes_tab_at_runtime_via_set_tabs_redispatch() {
    let owner = Owner::new();

    let (mount_handle, parent, store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    let initial_count = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .length();

    assert_eq!(initial_count, 3, "three tabs initially");

    // Mutate the store: pop the last tab. The fingerprint memo updates,
    // Effect::watch fires, the adapter re-dispatches SetTabs, the
    // machine re-derives `Context::tabs`, and the keyed `<For>` retires
    // the third row's DOM node — without remounting the component.
    owner.with(|| {
        store.tabs().write().pop();
    });

    leptos::task::tick().await;

    let after_pop_count = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .length();

    assert_eq!(
        after_pop_count, 2,
        "store.pop() must reduce the rendered tab list to 2"
    );
    assert_eq!(
        tablist
            .get_attribute("aria-owns")
            .unwrap_or_default()
            .split_whitespace()
            .count(),
        2,
        "aria-owns must track the live tab order after store.pop()"
    );

    // The first tab is still selected because it was never popped.
    let selected = tablist
        .query_selector(r#"[role="tab"][aria-selected="true"]"#)
        .expect("query should succeed")
        .expect("a tab is selected")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(selected.text_content().unwrap_or_default(), "First");

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn store_push_adds_tab_at_runtime_via_set_tabs_redispatch() {
    let owner = Owner::new();

    let (mount_handle, parent, store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    // Append a fourth tab.
    owner.with(|| {
        store.tabs().write().push(Tab::new_with_label(
            "fourth",
            "Fourth",
            ViewFn::from(|| view! { "Fourth" }),
            ViewFn::from(|| view! { <p>"Panel four"</p> }),
        ));
    });

    leptos::task::tick().await;

    let count = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .length();

    assert_eq!(count, 4, "store.push() must add a tab without remounting");
    assert_eq!(
        tablist
            .get_attribute("aria-owns")
            .unwrap_or_default()
            .split_whitespace()
            .count(),
        4,
        "aria-owns must track added tabs after store.push()"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test]
fn reorder_announcement_uses_messages_template() {
    use ars_components::navigation::tabs::{Machine, Messages, Props, TabRegistration};
    use ars_core::{Env, MessageFn, Service};

    // Direct test against the agnostic Api so we verify the template
    // path without needing a custom-Messages provider in the adapter.
    let messages = Messages {
        reorder_announce_label: MessageFn::new(
            |label: &str, position: usize, total: usize, _locale: &ars_core::Locale| {
                format!("CUSTOM {label} @ {position}/{total}")
            },
        ),
        ..Messages::default()
    };

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("reorder-i18n")
            .default_value(Some(Key::str("a"))),
        &Env::default(),
        &messages,
    );

    drop(
        service.send(ars_components::navigation::tabs::Event::SetTabs(vec![
            TabRegistration::new(Key::str("a")),
            TabRegistration::new(Key::str("b")),
            TabRegistration::new(Key::str("c")),
        ])),
    );

    let api = service.connect(&|_| {});
    let announcement = api.reorder_announcement("Inbox", 2, 3);

    assert_eq!(
        announcement, "CUSTOM Inbox @ 2/3",
        "Api::reorder_announcement must route through Messages template"
    );
}

#[wasm_bindgen_test(async)]
async fn close_label_uses_live_provider_messages_after_locale_changes() {
    let parent = container();

    let locale = RwSignal::new(locales::en_us());

    let mut registries = I18nRegistries::new();

    registries.register(
        MessagesRegistry::new(ars_components::navigation::tabs::Messages::default()).register(
            "pt-BR",
            ars_components::navigation::tabs::Messages {
                close_tab_label: MessageFn::new(|label: &str, _locale: &ars_core::Locale| {
                    format!("Fechar {label}")
                }),
                ..ars_components::navigation::tabs::Messages::default()
            },
        ),
    );

    let mount_handle = mount_to(parent.clone(), move || {
        view! {
            <ArsProvider
                locale=locale
                direction=Direction::Ltr
                i18n_registries=Arc::new(registries)
            >
                <Tabs
                    default_value="overview"
                    tabs=[
                        Tab::new_static(
                                "overview",
                                "Overview",
                                || view! { <p>"Overview panel"</p> },
                            )
                            .closable(true),
                        Tab::new_static("details", "Details", || view! { <p>"Details panel"</p> }),
                    ]
                />
            </ArsProvider>
        }
    });

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "tab-close-trigger")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Close Overview")
    );

    locale.set(locales::br());

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "tab-close-trigger")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Fechar Overview")
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn store_closable_toggle_redispatches_set_tabs() {
    let owner = Owner::new();

    let (mount_handle, parent, store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    // No close trigger yet — none of the initial tabs are closable.
    assert!(
        parent
            .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
            .expect("query should succeed")
            .is_none(),
        "no tab is closable initially"
    );

    // Flip the second tab to closable via store mutation. The keyed
    // <For> preserves the row's DOM node, but `render_tab_button`
    // subscribes reactively to `tabs.read()` to read the row's
    // `closable` flag — so the close button surfaces without
    // remounting the parent row.
    owner.with(|| {
        let field = store.tabs();
        let mut tabs = field.write();
        let second = Tab::new_with_label(
            "second",
            "Second",
            ViewFn::from(|| view! { "Second" }),
            ViewFn::from(|| view! { <p>"Panel two"</p> }),
        )
        .closable(true);

        tabs[1] = second;
    });

    leptos::task::tick().await;

    let close_btn = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed");

    assert!(
        close_btn.is_some(),
        "closable=true must surface a close trigger after store mutation"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn store_disabled_toggle_updates_tab_accessibility_attrs() {
    let owner = Owner::new();

    let (mount_handle, parent, store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    let second = tab_at(&parent, 1);

    assert_eq!(second.get_attribute("aria-disabled"), None);
    assert_eq!(second.get_attribute("data-ars-disabled"), None);

    owner.with(|| {
        let field = store.tabs();
        let mut tabs = field.write();

        tabs[1] = Tab::new_with_label(
            "second",
            "Second",
            ViewFn::from(|| view! { "Second" }),
            ViewFn::from(|| view! { <p>"Panel two"</p> }),
        )
        .disabled(true);
    });

    leptos::task::tick().await;

    let second = tab_at(&parent, 1);

    assert_eq!(
        second.get_attribute("aria-disabled").as_deref(),
        Some("true"),
        "aria-disabled must update when the backing store disables a tab"
    );
    assert_eq!(
        second.get_attribute("data-ars-disabled").as_deref(),
        Some(""),
        "data-ars-disabled must update when the backing store disables a tab"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn store_same_key_row_update_refreshes_trigger_and_panel_content() {
    let owner = Owner::new();

    let (mount_handle, parent, store) = owner.with(|| {
        let parent = container();
        let store = store_handle(three_tabs());
        let field: Field<Vec<TestTab>> = store.tabs().into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=field /> }
        });

        (mount_handle, parent, store)
    });

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).text_content().unwrap_or_default(),
        "First"
    );
    assert_eq!(selected_panel_text(&parent), "Panel one");

    owner.with(|| {
        let field = store.tabs();
        let mut tabs = field.write();

        tabs[0] = Tab::new_with_label(
            "first",
            "First",
            ViewFn::from(|| view! { "First updated trigger" }),
            ViewFn::from(|| view! { <p>"Panel one updated"</p> }),
        );
    });

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).text_content().unwrap_or_default(),
        "First updated trigger",
        "same-key trigger content should refresh from the latest tab row"
    );
    assert_eq!(
        selected_panel_text(&parent),
        "Panel one updated",
        "same-key panel content should refresh from the latest tab row"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn auto_direction_updates_rendered_root_dir() {
    let owner = Owner::new();

    let (mount_handle, parent) = owner.with(|| {
        let parent = container();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) dir=Direction::Auto /> }
        });

        (mount_handle, parent)
    });

    leptos::task::tick().await;
    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "root")
            .get_attribute("dir")
            .as_deref(),
        Some("ltr"),
        "dir=auto should render the resolved concrete direction after setup"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn signal_backed_orientation_updates_keyboard_navigation() {
    let owner = Owner::new();

    let (mount_handle, parent, set_orientation) = owner.with(|| {
        let parent = container();

        let (orientation, set_orientation) = signal(Orientation::Horizontal);

        let orientation_signal: Signal<Orientation> = orientation.into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! {
                <Tabs
                    default_value="first"
                    tabs=store_field(three_tabs())
                    orientation=orientation_signal
                />
            }
        });

        (mount_handle, parent, set_orientation)
    });

    leptos::task::tick().await;

    set_orientation.set(Orientation::Vertical);

    leptos::task::tick().await;

    assert_eq!(
        first_with_data_part(&parent, "root")
            .get_attribute("data-ars-orientation")
            .as_deref(),
        Some("vertical"),
        "root orientation attrs should track signal-backed orientation"
    );
    assert_eq!(
        first_with_data_part(&parent, "list")
            .get_attribute("aria-orientation")
            .as_deref(),
        Some("vertical"),
        "list orientation attrs should track signal-backed orientation"
    );

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");
    dispatch_keydown(&first, "ArrowDown", false);

    leptos::task::tick().await;

    assert_eq!(
        selected_tab_text(&parent),
        "Second",
        "a signal-backed orientation prop should update arrow-key handling"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn signal_backed_reorderable_updates_draggable_tabs() {
    let owner = Owner::new();

    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));

    let (mount_handle, parent, set_reorderable) = owner.with(|| {
        let parent = container();
        let (reorderable, set_reorderable) = signal(false);
        let reorderable: Signal<bool> = reorderable.into();

        let mount_handle = mount_to(parent.clone(), {
            let reordered = Arc::clone(&reordered);
            move || {
                view! {
                    <Tabs
                        default_value="first"
                        tabs=store_field(three_tabs())
                        reorderable
                        on_reorder=Callback::new({
                            let reordered = Arc::clone(&reordered);
                            move |event| {
                                reordered
                                    .lock()
                                    .expect("reorder callback log should not be poisoned")
                                    .push(event);
                                true
                            }
                        })
                    />
                }
            }
        });

        (mount_handle, parent, set_reorderable)
    });

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).get_attribute("draggable").as_deref(),
        Some("false"),
        "tabs should not advertise drag affordance before reorderable is enabled"
    );
    assert_eq!(
        tab_at(&parent, 0).get_attribute("aria-roledescription"),
        None,
        "tabs should not expose a drag roledescription before reorderable is enabled"
    );

    set_reorderable.set(true);

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).get_attribute("draggable").as_deref(),
        Some("true"),
        "draggable attrs should track signal-backed reorderable"
    );
    assert_eq!(
        tab_at(&parent, 0)
            .get_attribute("aria-roledescription")
            .as_deref(),
        Some("draggable tab"),
        "drag roledescription should track signal-backed reorderable"
    );

    let first = tab_at(&parent, 0);
    let third = tab_at(&parent, 2);

    dispatch_drag_event(&first, "dragstart");

    set_reorderable.set(false);

    leptos::task::tick().await;

    assert_eq!(
        tab_at(&parent, 0).get_attribute("draggable").as_deref(),
        Some("false"),
        "draggable attr should turn off when reorderable turns off"
    );
    assert_eq!(
        tab_at(&parent, 0).get_attribute("aria-roledescription"),
        None,
        "drag roledescription should be removed when reorderable turns off"
    );

    let stale_dragover = dispatch_drag_event(&third, "dragover");
    let stale_drop = dispatch_drag_event(&third, "drop");

    assert!(
        !stale_dragover.default_prevented(),
        "dragover should not accept a stale source after reorderable turns off"
    );
    assert!(
        !stale_drop.default_prevented(),
        "drop should not reorder after reorderable turns off"
    );
    assert!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .is_empty(),
        "turning reorderable off must block stale drag sources"
    );

    drop(mount_handle);
}

#[wasm_bindgen_test(async)]
async fn controlled_value_none_clears_selection_without_leaving_controlled_mode() {
    let owner = Owner::new();

    let (mount_handle, parent, set_value) = owner.with(|| {
        let parent = container();
        let (value, set_value) = signal::<Option<&'static str>>(Some("first"));
        let value_signal: Signal<Option<&'static str>> = value.into();

        let mount_handle = mount_to(parent.clone(), move || {
            view! { <Tabs default_value="first" tabs=store_field(three_tabs()) value=value_signal /> }
        });

        (mount_handle, parent, set_value)
    });

    leptos::task::tick().await;

    let tablist = first_with_data_part(&parent, "list");

    // Initial: first tab selected.
    assert_eq!(
        tablist
            .query_selector_all(r#"[aria-selected="true"]"#)
            .expect("query should succeed")
            .length(),
        1,
        "exactly one tab selected initially"
    );

    // Clear selection via controlled signal. The agnostic core's
    // `snap_value_to_valid_key` will then snap to the first non-disabled
    // tab — which is the first tab. The point is we don't crash and
    // remain in controlled mode.
    set_value.set(None);

    leptos::task::tick().await;

    let after = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .length();

    assert_eq!(after, 3, "tablist still rendered after controlled None");

    drop(mount_handle);
}
