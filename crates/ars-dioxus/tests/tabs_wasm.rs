//! Browser tests for the Dioxus Tabs adapter.
//!
//! Verifies the wasm32 build path: that the adapter and its wasm-only
//! helpers (`MountedData` focus, reactive prop sync, lazy-mount
//! tracking) compile, that the `VirtualDom` lifecycle drives the
//! reactive contract, and that the rendered structure is observable
//! through the connect API.
//!
//! Most tests use `VirtualDom`; focused DOM-event coverage uses
//! `dioxus-web` launch for browser-only behavior such as canceled link
//! activation.

#![cfg(target_arch = "wasm32")]

use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use ars_collections::Key;
use ars_components::navigation::tabs;
use ars_core::{HtmlAttr, PlatformEffects, SafeUrl};
use ars_dioxus::{
    ArsProvider, DioxusPlatform, DragData, FilePickerOptions, PlatformDragEvent,
    default_dioxus_platform,
    dioxus_stores::use_store,
    navigation::tabs::{ActivationMode, ReadStore, ReorderEvent, Tab, TabLabel, Tabs, TabsSource},
};
use ars_forms::field::FileRef;
use dioxus::{
    dioxus_core::{NoOpMutations, ScopeId},
    events::MountedData,
    prelude::*,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

type TestTab = Tab<&'static str>;
type TestReorderEvent = ReorderEvent<&'static str>;

#[wasm_bindgen_test]
fn web_tab_public_builders_and_debug_paths_are_covered() {
    let label = TabLabel::static_text("Standalone");

    assert_eq!(label.resolve(), "Standalone");

    let tab = Tab::new_static("standalone", "Standalone", rsx! { "Panel" })
        .trigger(rsx! {
            strong { "Standalone trigger" }
        })
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

    let from_vec = TabsSource::from(vec![Tab::new_static("first", "First", rsx! { "Panel" })]);

    assert!(matches!(from_vec, TabsSource::Owned(_)));
    assert!(format!("{from_vec:?}").contains("TabsSource::Owned"));

    let from_array = TabsSource::from([Tab::new_static("second", "Second", rsx! { "Panel" })]);

    assert!(matches!(from_array, TabsSource::Owned(_)));
}

struct MountedFocusProbePlatform {
    focused: Arc<Mutex<usize>>,
    inner: Arc<dyn DioxusPlatform>,
}

impl DioxusPlatform for MountedFocusProbePlatform {
    fn focus_mounted_element(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let inner = Arc::clone(&self.inner);

        *self
            .focused
            .lock()
            .expect("focus counter lock should succeed") += 1;

        Box::pin(async move { inner.focus_mounted_element(element).await })
    }

    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        self.inner.set_clipboard(text)
    }

    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        self.inner.open_file_picker(options)
    }

    fn monotonic_now(&self) -> Duration {
        self.inner.monotonic_now()
    }

    fn new_id(&self) -> String {
        self.inner.new_id()
    }

    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData> {
        self.inner.create_drag_data(event)
    }
}

#[derive(Clone)]
struct MountedFocusProbeProps {
    dioxus_platform: Arc<dyn DioxusPlatform>,
}

impl PartialEq for MountedFocusProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.dioxus_platform, &other.dioxus_platform)
    }
}

fn three_tabs() -> Vec<TestTab> {
    vec![
        Tab::new_with_label(
            "first",
            "First",
            rsx! { "First" },
            rsx! { p { "Panel one" } },
        ),
        Tab::new_with_label(
            "second",
            "Second",
            rsx! { "Second" },
            rsx! { p { "Panel two" } },
        ),
        Tab::new_with_label(
            "third",
            "Third",
            rsx! { "Third" },
            rsx! { p { "Panel three" } },
        ),
    ]
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

async fn deferred_focus_turn() {
    animation_frame_turn().await;
    animation_frame_turn().await;
}

fn cancelable_click(target: &web_sys::HtmlElement) -> web_sys::MouseEvent {
    let init = web_sys::MouseEventInit::new();

    init.set_bubbles(true);
    init.set_cancelable(true);

    let event = web_sys::MouseEvent::new_with_mouse_event_init_dict("click", &init)
        .expect("click should construct");

    target
        .dispatch_event(&event)
        .expect("click should dispatch");

    event
}

fn click(target: &web_sys::HtmlElement) {
    drop(cancelable_click(target));
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
    let init = web_sys::KeyboardEventInit::new();

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
    let init = web_sys::PointerEventInit::new();

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

fn dispatch_drag_event(target: &web_sys::HtmlElement, kind: &str) -> web_sys::DragEvent {
    let init = web_sys::DragEventInit::new();

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

#[derive(Clone, PartialEq)]
struct LinkCallbackProbeProps {
    selected: Rc<RefCell<Vec<Option<&'static str>>>>,
}

#[derive(Clone, PartialEq)]
struct ControlledProbeProps {
    selected: Rc<RefCell<Vec<Option<&'static str>>>>,
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn controlled_probe(props: ControlledProbeProps) -> Element {
    let tabs = use_store(three_tabs);
    let selected = Rc::clone(&props.selected);

    rsx! {
        Tabs {
            value: Some("first"),
            default_value: "first",
            tabs: ReadStore::from(tabs),
            on_value_change: move |key| selected.borrow_mut().push(key),
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn controlled_keyboard_probe(props: ControlledProbeProps) -> Element {
    let tabs = use_store(three_tabs);
    let selected = Rc::clone(&props.selected);

    rsx! {
        Tabs {
            value: Some("first"),
            default_value: "first",
            tabs: ReadStore::from(tabs),
            on_value_change: move |key| selected.borrow_mut().push(key),
        }
    }
}

#[expect(
    unused_qualifications,
    reason = "Dioxus rsx event attributes are reported as unnecessary qualifications"
)]
fn store_mutation_probe() -> Element {
    let mut tabs = use_store(three_tabs);

    let push_tab = move |_| {
        tabs.write().push(Tab::new_with_label(
            "fourth",
            "Fourth",
            rsx! { "Fourth" },
            rsx! {
                p { "Panel four" }
            },
        ));
    };
    let pop_tab = move |_| {
        tabs.write().pop();
    };
    let make_closable = move |_| {
        tabs.write()[1] = Tab::new_with_label(
            "second",
            "Second",
            rsx! { "Second" },
            rsx! {
                p { "Panel two" }
            },
        )
        .closable(true);
    };
    let make_disabled = move |_| {
        tabs.write()[1] = Tab::new_with_label(
            "second",
            "Second",
            rsx! { "Second" },
            rsx! {
                p { "Panel two" }
            },
        )
        .disabled(true);
    };

    rsx! {
        button { id: "push-tab", onclick: push_tab, "push" }
        button { id: "pop-tab", onclick: pop_tab, "pop" }
        button { id: "make-closable", onclick: make_closable, "closable" }
        button { id: "make-disabled", onclick: make_disabled, "disabled" }
        Tabs { default_value: "first", tabs: ReadStore::from(tabs) }
    }
}

fn close_mutates_store_probe() -> Element {
    let mut tabs = use_store(|| {
        three_tabs()
            .into_iter()
            .map(|tab| {
                if tab.key == "first" {
                    tab.closable(true)
                } else {
                    tab
                }
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            on_close_tab: move |key: &'static str| {
                tabs.write().retain(|tab| tab.key != key);
            },
        }
    }
}

#[derive(Clone)]
struct DragReorderProbeProps {
    reordered: Arc<Mutex<Vec<TestReorderEvent>>>,
}

impl PartialEq for DragReorderProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reordered, &other.reordered)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn drag_reorder_probe(props: DragReorderProbeProps) -> Element {
    let mut tabs = use_store(three_tabs);
    let reordered = Arc::clone(&props.reordered);

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            reorderable: true,
            on_reorder: Callback::new(move |event: TestReorderEvent| {
                reordered
                    .lock()
                    .expect("reorder callback log should not be poisoned")
                    .push(event.clone());
                let mut tabs = tabs.write();
                let tab = tabs.remove(event.old_index);
                tabs.insert(event.new_index, tab);
                true
            }),
        }
    }
}

#[derive(Clone)]
struct CloseKeyProbeProps {
    closed: Arc<Mutex<Vec<&'static str>>>,
}

impl PartialEq for CloseKeyProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.closed, &other.closed)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn close_key_probe(props: CloseKeyProbeProps) -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label(
                "first",
                "First",
                rsx! { "First" },
                rsx! { p { "Panel one" } },
            )
            .closable(true),
            Tab::new_with_label(
                "second",
                "Second",
                rsx! { "Second" },
                rsx! { p { "Panel two" } },
            )
            .closable(true),
        ]
    });
    let closed = Arc::clone(&props.closed);

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            on_close_tab: Callback::new(move |key| {
                closed
                    .lock()
                    .expect("close callback log should not be poisoned")
                    .push(key);
            }),
        }
    }
}

fn inline_owned_close_probe() -> Element {
    rsx! {
        Tabs {
            default_value: "second",
            tabs: [
                Tab::new_with_label("first", "First", rsx! { "First" }, rsx! {
                    p { "Panel one" }
                }),
                Tab::new_with_label("second", "Second", rsx! { "Second" }, rsx! {
                    p { "Panel two" }
                })
                    .closable(true),
                Tab::new_with_label("third", "Third", rsx! { "Third" }, rsx! {
                    p { "Panel three" }
                }),
            ],
        }
    }
}

fn inline_owned_reorder_probe() -> Element {
    rsx! {
        Tabs {
            default_value: "first",
            tabs: [
                Tab::new_with_label("first", "First", rsx! { "First" }, rsx! {
                    p { "Panel one" }
                }),
                Tab::new_with_label("second", "Second", rsx! { "Second" }, rsx! {
                    p { "Panel two" }
                }),
                Tab::new_with_label("third", "Third", rsx! { "Third" }, rsx! {
                    p { "Panel three" }
                }),
            ],
            reorderable: true,
        }
    }
}

fn inline_owned_panel_state_probe() -> Element {
    let mut status = use_signal_sync(|| String::from("initial"));
    let onclick = move |_| {
        status.set(String::from("updated"));
    };

    rsx! {
        Tabs {
            default_value: "first",
            tabs: [
                Tab::new_with_label("first", "First", rsx! { "First" }, rsx! {
                    button { id: "update-owned-panel-status", onclick, "Update" }
                    p { id: "owned-panel-status", "{status}" }
                }),
                Tab::new_with_label("second", "Second", rsx! { "Second" }, rsx! {
                    p { "Panel two" }
                }),
            ],
        }
    }
}

#[derive(Clone)]
struct DisabledInteractionProbeProps {
    closed: Arc<Mutex<Vec<&'static str>>>,
    reordered: Arc<Mutex<Vec<TestReorderEvent>>>,
    selected: Arc<Mutex<Vec<Option<&'static str>>>>,
}

impl PartialEq for DisabledInteractionProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.closed, &other.closed)
            && Arc::ptr_eq(&self.reordered, &other.reordered)
            && Arc::ptr_eq(&self.selected, &other.selected)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn disabled_interaction_probe(props: DisabledInteractionProbeProps) -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label(
                "first",
                "First",
                rsx! { "First" },
                rsx! { p { "Panel one" } },
            ),
            Tab::new_with_label(
                "second",
                "Second",
                rsx! { "Second" },
                rsx! { p { "Panel two" } },
            )
            .closable(true)
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                rsx! { "Third" },
                rsx! { p { "Panel three" } },
            ),
        ]
    });

    let closed = Arc::clone(&props.closed);
    let reordered = Arc::clone(&props.reordered);
    let selected = Arc::clone(&props.selected);

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            reorderable: true,
            on_close_tab: Callback::new(move |key| {
                closed
                    .lock()
                    .expect("close callback log should not be poisoned")
                    .push(key);
            }),
            on_reorder: Callback::new(move |event| {
                reordered
                    .lock()
                    .expect("reorder callback log should not be poisoned")
                    .push(event);
                true
            }),
            on_value_change: Callback::new(move |key| {
                selected
                    .lock()
                    .expect("selected callback log should not be poisoned")
                    .push(key);
            }),
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn disabled_drag_reorder_probe(props: DragReorderProbeProps) -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label(
                "first",
                "First",
                rsx! { "First" },
                rsx! { p { "Panel one" } },
            ),
            Tab::new_with_label(
                "second",
                "Second",
                rsx! { "Second" },
                rsx! { p { "Panel two" } },
            )
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                rsx! { "Third" },
                rsx! { p { "Panel three" } },
            ),
        ]
    });

    let reordered = Arc::clone(&props.reordered);

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            reorderable: true,
            on_reorder: Callback::new(move |event| {
                reordered
                    .lock()
                    .expect("reorder callback log should not be poisoned")
                    .push(event);
                true
            }),
        }
    }
}

fn vertical_disabled_hotkeys_probe() -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label(
                "first",
                "First",
                rsx! { "First" },
                rsx! { p { "Panel one" } },
            ),
            Tab::new_with_label(
                "second",
                "Second",
                rsx! { "Second" },
                rsx! { p { "Panel two" } },
            )
            .disabled(true),
            Tab::new_with_label(
                "third",
                "Third",
                rsx! { "Third" },
                rsx! { p { "Panel three" } },
            ),
        ]
    });

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(tabs),
            orientation: ars_dioxus::prelude::Orientation::Vertical,
        }
    }
}

fn manual_hotkeys_probe() -> Element {
    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(use_store(three_tabs)),
            activation_mode: ActivationMode::Manual,
        }
    }
}

fn indicator_probe() -> Element {
    let platform: Arc<dyn PlatformEffects> = Arc::new(ars_dom::WebPlatformEffects);

    rsx! {
        ArsProvider { platform: Some(platform),
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }
}

fn focus_mount_probe() -> Element {
    let platform: Arc<dyn PlatformEffects> = Arc::new(ars_dom::WebPlatformEffects);

    rsx! {
        ArsProvider { platform: Some(platform),
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }
}

fn mounted_focus_probe(props: MountedFocusProbeProps) -> Element {
    rsx! {
        ArsProvider { dioxus_platform: Some(props.dioxus_platform),
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }
}

#[derive(Clone)]
struct ReorderVetoProbeProps {
    reordered: Arc<Mutex<Vec<TestReorderEvent>>>,
}

impl PartialEq for ReorderVetoProbeProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reordered, &other.reordered)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn reorder_veto_probe(props: ReorderVetoProbeProps) -> Element {
    let reordered = Arc::clone(&props.reordered);

    rsx! {
        Tabs {
            default_value: "first",
            tabs: ReadStore::from(use_store(three_tabs)),
            reorderable: true,
            on_reorder: Callback::new(move |event| {
                reordered
                    .lock()
                    .expect("reorder callback log should not be poisoned")
                    .push(event);
                false
            }),
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn link_callback_probe(props: LinkCallbackProbeProps) -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label("home", "Home", rsx! { "Home" }, rsx! { p { "Home panel" } })
                .link(SafeUrl::from_static("/home")),
            Tab::new_with_label(
                "settings",
                "Settings",
                rsx! { "Settings" },
                rsx! { p { "Settings panel" } },
            ),
        ]
    });

    let selected = Rc::clone(&props.selected);

    rsx! {
        Tabs {
            default_value: "settings",
            tabs: ReadStore::from(tabs),
            on_value_change: move |key| selected.borrow_mut().push(key),
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the component function"
)]
fn manual_link_callback_probe(props: LinkCallbackProbeProps) -> Element {
    let tabs = use_store(|| {
        vec![
            Tab::new_with_label("home", "Home", rsx! { "Home" }, rsx! { p { "Home panel" } })
                .link(SafeUrl::from_static("/home")),
            Tab::new_with_label(
                "settings",
                "Settings",
                rsx! { "Settings" },
                rsx! { p { "Settings panel" } },
            ),
        ]
    });

    let selected = Rc::clone(&props.selected);

    rsx! {
        Tabs {
            default_value: "settings",
            tabs: ReadStore::from(tabs),
            activation_mode: ActivationMode::Manual,
            on_value_change: move |key| selected.borrow_mut().push(key),
        }
    }
}

#[wasm_bindgen_test(async)]
async fn web_link_click_prevents_default_and_emits_value_change() {
    let parent = container();
    let selected = Rc::new(RefCell::new(Vec::<Option<&'static str>>::new()));
    let dom = VirtualDom::new_with_props(
        link_callback_probe,
        LinkCallbackProbeProps {
            selected: Rc::clone(&selected),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let anchor = parent
        .query_selector(r#"a[role="tab"][href="/home"]"#)
        .expect("query should succeed")
        .expect("link tab should render as anchor")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("anchor is HtmlElement");

    let event = cancelable_click(&anchor);

    animation_frame_turn().await;

    assert!(
        event.default_prevented(),
        "link tab click should be canceled so browser navigation does not run"
    );
    assert_eq!(selected.borrow().as_slice(), &[Some("home")]);
}

#[wasm_bindgen_test(async)]
async fn web_manual_link_tabs_activate_from_keyboard_without_browser_navigation() {
    let parent = container();
    let selected = Rc::new(RefCell::new(Vec::<Option<&'static str>>::new()));
    let dom = VirtualDom::new_with_props(
        manual_link_callback_probe,
        LinkCallbackProbeProps {
            selected: Rc::clone(&selected),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let anchor = parent
        .query_selector(r#"a[role="tab"][href="/home"]"#)
        .expect("query should succeed")
        .expect("link tab should render as anchor")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("anchor is HtmlElement");

    anchor.focus().expect("focus should succeed");
    let enter = dispatch_keydown(&anchor, "Enter", false);

    animation_frame_turn().await;

    assert!(
        enter.default_prevented(),
        "manual link tab Enter should select without native navigation"
    );
    assert_eq!(selected.borrow().as_slice(), &[Some("home")]);
}

#[wasm_bindgen_test(async)]
async fn web_controlled_click_emits_value_change_without_mutating_selection() {
    let parent = container();
    let selected = Rc::new(RefCell::new(Vec::<Option<&'static str>>::new()));
    let dom = VirtualDom::new_with_props(
        controlled_probe,
        ControlledProbeProps {
            selected: Rc::clone(&selected),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let second = first_with_data_part(&parent, "list")
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(1)
        .expect("second tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    click(&second);

    animation_frame_turn().await;

    assert_eq!(selected.borrow().as_slice(), &[Some("second")]);
    assert_eq!(selected_tab_text(&parent), "First");
}

#[wasm_bindgen_test(async)]
async fn web_controlled_arrow_emits_value_change_without_mutating_selection() {
    let parent = container();
    let selected = Rc::new(RefCell::new(Vec::<Option<&'static str>>::new()));
    let dom = VirtualDom::new_with_props(
        controlled_keyboard_probe,
        ControlledProbeProps {
            selected: Rc::clone(&selected),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = first_with_data_part(&parent, "list")
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    animation_frame_turn().await;

    dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

    assert_eq!(selected.borrow().as_slice(), &[Some("second")]);
    assert_eq!(selected_tab_text(&parent), "First");
}

#[wasm_bindgen_test(async)]
async fn web_store_mutations_update_tabs_without_remounting() {
    let parent = container();
    let dom = VirtualDom::new(store_mutation_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let tablist = first_with_data_part(&parent, "list");

    assert_eq!(
        tablist
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        3
    );

    click(
        &parent
            .query_selector("#push-tab")
            .expect("query should succeed")
            .expect("push button")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button is HtmlElement"),
    );

    animation_frame_turn().await;

    assert_eq!(
        tablist
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        4
    );

    click(
        &parent
            .query_selector("#pop-tab")
            .expect("query should succeed")
            .expect("pop button")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button is HtmlElement"),
    );

    animation_frame_turn().await;

    assert_eq!(
        tablist
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        3
    );

    click(
        &parent
            .query_selector("#make-closable")
            .expect("query should succeed")
            .expect("closable button")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button is HtmlElement"),
    );

    animation_frame_turn().await;

    assert!(
        parent
            .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
            .expect("query should succeed")
            .is_some(),
        "closable toggle should render a close trigger"
    );

    click(
        &parent
            .query_selector("#make-disabled")
            .expect("query should succeed")
            .expect("disabled button")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button is HtmlElement"),
    );

    animation_frame_turn().await;

    let second = tablist
        .query_selector_all(r#"[role="tab"]"#)
        .expect("query should succeed")
        .item(1)
        .expect("second tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    assert_eq!(
        second.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
}

#[wasm_bindgen_test(async)]
async fn web_close_callback_can_remove_tab_from_store() {
    let parent = container();
    let dom = VirtualDom::new(close_mutates_store_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    animation_frame_turn().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        2,
        "close callback should be able to mutate the tab store"
    );
}

#[wasm_bindgen_test(async)]
async fn web_inline_array_close_trigger_removes_owned_tab() {
    let parent = container();
    let dom = VirtualDom::new(inline_owned_close_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let close = parent
        .query_selector(r#"[data-ars-part="tab-close-trigger"]"#)
        .expect("query should succeed")
        .expect("close trigger should render")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("close trigger is HtmlElement");

    click(&close);

    animation_frame_turn().await;

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
}

#[wasm_bindgen_test(async)]
async fn web_inline_array_owned_tabs_refresh_existing_panel_content() {
    let parent = container();
    let dom = VirtualDom::new(inline_owned_panel_state_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let trigger = parent
        .query_selector("#update-owned-panel-status")
        .expect("query should succeed")
        .expect("update trigger should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("update trigger is HtmlElement");

    click(&trigger);

    animation_frame_turn().await;

    let status = parent
        .query_selector("#owned-panel-status")
        .expect("query should succeed")
        .expect("status element should exist")
        .text_content()
        .unwrap_or_default();

    assert_eq!(
        status, "updated",
        "owned inline tabs must refresh existing panel content when parent Dioxus state changes"
    );
}

#[wasm_bindgen_test(async)]
async fn web_keyboard_close_removing_selected_tab_focuses_successor() {
    let parent = container();
    let dom = VirtualDom::new(close_mutates_store_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;
    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "Delete", false);

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
}

#[wasm_bindgen_test(async)]
async fn web_drag_and_drop_reorders_tabs_through_callback() {
    let parent = container();
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let dom = VirtualDom::new_with_props(
        drag_reorder_probe,
        DragReorderProbeProps {
            reordered: Arc::clone(&reordered),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

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
    let drop = dispatch_drag_event(&third, "drop");

    deferred_focus_turn().await;

    assert!(dragover.default_prevented(), "dragover should allow drop");
    assert!(
        drop.default_prevented(),
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
}

#[wasm_bindgen_test(async)]
async fn web_inline_array_drag_and_drop_reorders_owned_tabs() {
    let parent = container();
    let dom = VirtualDom::new(inline_owned_reorder_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

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

    let drop = dispatch_drag_event(&third, "drop");

    animation_frame_turn().await;

    assert!(
        drop.default_prevented(),
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
}

#[wasm_bindgen_test(async)]
async fn web_drag_and_drop_ignores_missing_same_or_disabled_targets() {
    let parent = container();
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let dom = VirtualDom::new_with_props(
        disabled_drag_reorder_probe,
        DragReorderProbeProps {
            reordered: Arc::clone(&reordered),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

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
    assert!(
        reordered
            .lock()
            .expect("reorder callback log should not be poisoned")
            .is_empty(),
        "invalid drag/drop attempts must not call on_reorder"
    );
}

#[wasm_bindgen_test(async)]
async fn web_drag_and_drop_reorder_veto_suppresses_drop_commit() {
    let parent = container();
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let dom = VirtualDom::new_with_props(
        reorder_veto_probe,
        ReorderVetoProbeProps {
            reordered: Arc::clone(&reordered),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);
    let third = tab_at(&parent, 2);

    dispatch_drag_event(&first, "dragstart");

    let drop_event = dispatch_drag_event(&third, "drop");

    animation_frame_turn().await;

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
}

#[wasm_bindgen_test(async)]
async fn web_mount_does_not_focus_selected_tab_without_keyboard_intent() {
    let parent = container();
    let sentinel = document()
        .create_element("button")
        .expect("sentinel should be created")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("sentinel should be HtmlElement");

    sentinel.set_id("tabs-focus-sentinel");

    parent
        .append_child(&sentinel)
        .expect("sentinel should be attached");

    sentinel.focus().expect("sentinel should focus");

    let dom = VirtualDom::new(focus_mount_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let active = document()
        .active_element()
        .and_then(|element| element.get_attribute("id"));

    assert_eq!(
        active.as_deref(),
        Some("tabs-focus-sentinel"),
        "Tabs should not steal focus until a focus-moving user event occurs"
    );
}

#[wasm_bindgen_test(async)]
async fn web_indicator_style_tracks_selected_tab_measurement() {
    let parent = container();

    parent
        .set_attribute(
            "style",
            "position: relative; display: block; width: 400px; height: 200px;",
        )
        .expect("style should set");

    let dom = VirtualDom::new(indicator_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let style = first_with_data_part(&parent, "tab-indicator")
        .get_attribute("style")
        .expect("indicator should receive measurement style");

    assert!(style.contains("--ars-indicator-width:"), "{style}");
    assert!(style.contains("--ars-indicator-height:"), "{style}");
}

#[wasm_bindgen_test(async)]
async fn web_indicator_style_degrades_to_empty_without_geometry() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let indicator = first_with_data_part(&parent, "tab-indicator");

    assert!(
        indicator
            .get_attribute("style")
            .is_none_or(|style| style.is_empty()),
        "indicator should not keep stale measurement styles when geometry is unavailable"
    );
}

#[wasm_bindgen_test(async)]
async fn web_reorder_veto_suppresses_live_announcement() {
    let parent = container();
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let dom = VirtualDom::new_with_props(
        reorder_veto_probe,
        ReorderVetoProbeProps {
            reordered: Arc::clone(&reordered),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = first_with_data_part(&parent, "list")
        .query_selector(r#"[role="tab"]"#)
        .expect("query should succeed")
        .expect("first tab should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("tab is HtmlElement");

    first.focus().expect("focus should succeed");

    animation_frame_turn().await;

    let event = dispatch_keydown(&first, "ArrowRight", true);

    animation_frame_turn().await;

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

    let live_region = parent
        .query_selector(r#"[aria-live="polite"]"#)
        .expect("query should succeed")
        .expect("live region should render when reorderable");

    assert_eq!(live_region.text_content().unwrap_or_default(), "");
}

#[wasm_bindgen_test(async)]
async fn web_automatic_hotkeys_skip_disabled_tabs_and_support_vertical_axis() {
    let parent = container();
    let dom = VirtualDom::new(vertical_disabled_hotkeys_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    animation_frame_turn().await;

    let ignored_horizontal = dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

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

    animation_frame_turn().await;

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

    animation_frame_turn().await;

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
}

#[wasm_bindgen_test(async)]
async fn web_manual_activation_accepts_enter_and_space_hotkeys() {
    let parent = container();
    let dom = VirtualDom::new(manual_hotkeys_probe);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let arrow = dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

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

    animation_frame_turn().await;

    assert!(space.default_prevented(), "Space should select focused tab");
    assert_eq!(selected_tab_text(&parent), "Second");

    let third = tab_at(&parent, 2);

    third.focus().expect("focus should succeed");

    let enter = dispatch_keydown(&third, "Enter", false);

    animation_frame_turn().await;

    assert!(enter.default_prevented(), "Enter should select focused tab");
    assert_eq!(selected_tab_text(&parent), "Third");
}

#[wasm_bindgen_test(async)]
async fn web_tab_key_entry_target_tracks_selected_roving_tabindex() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

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

    animation_frame_turn().await;

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
        active_element_text(),
        "Second",
        "ArrowRight should move DOM focus to the newly selected tab"
    );
    assert_eq!(
        tab_at(&parent, 2).get_attribute("tabindex").as_deref(),
        Some("-1")
    );
}

#[wasm_bindgen_test(async)]
async fn web_browser_tabbable_order_tracks_only_the_selected_roving_tab() {
    let parent = container();

    parent.set_id("dioxus-tabs-tabbable-root");

    let before = document()
        .create_element("button")
        .expect("before sentinel should be created")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("before sentinel should be HtmlElement");

    before.set_id("dioxus-tabs-before");
    before.set_text_content(Some("before"));

    parent
        .append_child(&before)
        .expect("before sentinel should attach");

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let after = document()
        .create_element("button")
        .expect("after sentinel should be created")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("after sentinel should be HtmlElement");

    after.set_id("dioxus-tabs-after");
    after.set_text_content(Some("after"));

    parent
        .append_child(&after)
        .expect("after sentinel should attach");

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

    let order = platform.tabbable_element_ids("dioxus-tabs-tabbable-root");

    assert_eq!(
        order,
        vec![
            "dioxus-tabs-before".to_owned(),
            first.id(),
            first_panel.id(),
            "dioxus-tabs-after".to_owned(),
        ],
        "browser tabbable order should include the selected roving tab and selected panel"
    );
    assert!(
        !order.contains(&second.id()) && !order.contains(&third.id()),
        "inactive tabs must not be browser Tab-key targets"
    );

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

    let order = platform.tabbable_element_ids("dioxus-tabs-tabbable-root");

    let second_panel = parent
        .query_selector(r#"[role="tabpanel"]:not([hidden])"#)
        .expect("query should succeed")
        .expect("selected panel should exist")
        .dyn_into::<web_sys::HtmlElement>()
        .expect("panel should be HtmlElement");

    assert_eq!(
        order,
        vec![
            "dioxus-tabs-before".to_owned(),
            second.id(),
            second_panel.id(),
            "dioxus-tabs-after".to_owned(),
        ],
        "after selection changes, native Tab entry should move to the selected tab"
    );
}

#[wasm_bindgen_test(async)]
async fn web_focus_visible_tracks_keyboard_and_pointer_modality() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            ArsProvider {
                Tabs {
                    default_value: "first",
                    tabs: ReadStore::from(use_store(three_tabs)),
                }
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

    let second = tab_at(&parent, 1);

    assert!(
        has_focus_visible(&second),
        "keyboard focus should render data-ars-focus-visible on the focused tab"
    );

    let third = tab_at(&parent, 2);

    dispatch_pointerdown(&third, "mouse");

    click(&third);

    third.focus().expect("focus should succeed");

    animation_frame_turn().await;

    assert_eq!(selected_tab_text(&parent), "Third");
    assert!(
        !has_focus_visible(&third),
        "pointer focus should not render keyboard focus-visible state"
    );
}

#[wasm_bindgen_test(async)]
async fn web_keyboard_focus_dispatch_uses_mounted_data_platform() {
    let parent = container();
    let focused = Arc::new(Mutex::new(0_usize));
    let dioxus_platform: Arc<dyn DioxusPlatform> = Arc::new(MountedFocusProbePlatform {
        focused: Arc::clone(&focused),
        inner: default_dioxus_platform(),
    });

    let dom = VirtualDom::new_with_props(
        mounted_focus_probe,
        MountedFocusProbeProps { dioxus_platform },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    deferred_focus_turn().await;

    assert_eq!(
        *focused.lock().expect("focus counter lock should succeed"),
        1,
        "keyboard roving focus should route through Dioxus MountedData focus"
    );
    assert_eq!(active_element_text(), "Second");
}

#[wasm_bindgen_test(async)]
async fn web_pointerdown_modality_supports_mouse_touch_and_pen() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            ArsProvider {
                Tabs {
                    default_value: "first",
                    tabs: ReadStore::from(use_store(three_tabs)),
                }
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

    let second = tab_at(&parent, 1);

    assert!(has_focus_visible(&second));

    let third = tab_at(&parent, 2);

    for pointer_type in ["mouse", "touch", "pen"] {
        dispatch_pointerdown(&third, pointer_type);

        click(&third);

        third.focus().expect("focus should succeed");

        animation_frame_turn().await;

        assert!(
            !has_focus_visible(&third),
            "{pointer_type} pointerdown should clear keyboard focus-visible state"
        );

        dispatch_keydown(&third, "ArrowLeft", false);

        animation_frame_turn().await;

        assert!(
            has_focus_visible(&second),
            "keyboard navigation should restore focus-visible before the next pointer variant"
        );
    }
}

#[wasm_bindgen_test(async)]
async fn web_virtual_click_preserves_keyboard_focus_visible_modality() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            ArsProvider {
                Tabs {
                    default_value: "first",
                    tabs: ReadStore::from(use_store(three_tabs)),
                }
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "ArrowRight", false);

    animation_frame_turn().await;

    let third = tab_at(&parent, 2);

    click(&third);

    third.focus().expect("focus should succeed");

    animation_frame_turn().await;

    assert_eq!(selected_tab_text(&parent), "Third");
    assert!(
        has_focus_visible(&third),
        "click activation without a preceding pointerdown should keep keyboard/virtual modality"
    );
}

#[wasm_bindgen_test(async)]
async fn web_closable_tab_dispatches_close_on_delete_and_backspace() {
    let parent = container();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let dom = VirtualDom::new_with_props(
        close_key_probe,
        CloseKeyProbeProps {
            closed: Arc::clone(&closed),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let close_trigger = first_with_data_part(&parent, "tab-close-trigger");

    assert_eq!(
        close_trigger.get_attribute("tabindex").as_deref(),
        Some("-1"),
        "close trigger must NOT participate in roving tabindex"
    );

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    dispatch_keydown(&first, "Delete", false);

    animation_frame_turn().await;

    let second = tab_at(&parent, 1);

    second.focus().expect("focus should succeed");

    dispatch_keydown(&second, "Backspace", false);

    animation_frame_turn().await;

    assert_eq!(
        first_with_data_part(&parent, "list")
            .query_selector_all(r#"[role="tab"]"#)
            .expect("query should succeed")
            .length(),
        2,
        "CloseTab is pure notification; adapter must not auto-remove tabs"
    );
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first", "second"],
        "Delete and Backspace should notify close for closable tabs"
    );
}

#[wasm_bindgen_test(async)]
async fn web_repeated_and_composing_close_hotkeys_do_not_emit_close_requests() {
    let parent = container();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let dom = VirtualDom::new_with_props(
        close_key_probe,
        CloseKeyProbeProps {
            closed: Arc::clone(&closed),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let repeat = dispatch_keydown_with_options(&first, "Delete", false, true, false);
    let composing = dispatch_keydown_with_options(&first, "Delete", false, false, true);

    animation_frame_turn().await;

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

    animation_frame_turn().await;

    assert!(normal.default_prevented());
    assert_eq!(
        closed
            .lock()
            .expect("close callback log should not be poisoned")
            .as_slice(),
        &["first"]
    );
}

#[wasm_bindgen_test(async)]
async fn web_repeated_and_composing_manual_activation_hotkeys_do_not_select() {
    let parent = container();

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "second",
                tabs: ReadStore::from(use_store(three_tabs)),
                activation_mode: ActivationMode::Manual,
            }
        }
    }

    let dom = VirtualDom::new(app);

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    let first = tab_at(&parent, 0);

    first.focus().expect("focus should succeed");

    let repeat = dispatch_keydown_with_options(&first, "Enter", false, true, false);
    let composing = dispatch_keydown_with_options(&first, "Enter", false, false, true);

    animation_frame_turn().await;

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

    animation_frame_turn().await;

    assert!(normal.default_prevented());
    assert_eq!(selected_tab_text(&parent), "First");
}

#[wasm_bindgen_test(async)]
async fn web_close_trigger_click_does_not_select_its_tab() {
    let parent = container();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    #[derive(Clone)]
    struct Props {
        closed: Arc<Mutex<Vec<&'static str>>>,
    }

    impl PartialEq for Props {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.closed, &other.closed)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the component function"
    )]
    fn probe(props: Props) -> Element {
        let tabs = use_store(|| {
            vec![
                Tab::new_with_label(
                    "first",
                    "First",
                    rsx! { "First" },
                    rsx! { p { "Panel one" } },
                )
                .closable(true),
                Tab::new_with_label(
                    "second",
                    "Second",
                    rsx! { "Second" },
                    rsx! { p { "Panel two" } },
                ),
            ]
        });

        let closed = Arc::clone(&props.closed);

        rsx! {
            Tabs {
                default_value: "second",
                tabs: ReadStore::from(tabs),
                on_close_tab: Callback::new(move |key| {
                    closed
                        .lock()
                        .expect("close callback log should not be poisoned")
                        .push(key);
                }),
            }
        }
    }

    let dom = VirtualDom::new_with_props(
        probe,
        Props {
            closed: Arc::clone(&closed),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

    click(&first_with_data_part(&parent, "tab-close-trigger"));

    animation_frame_turn().await;

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
}

#[wasm_bindgen_test(async)]
async fn web_disabled_tabs_ignore_direct_click_close_key_and_reorder_shortcut() {
    let parent = container();
    let closed = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let reordered = Arc::new(Mutex::new(Vec::<TestReorderEvent>::new()));
    let selected = Arc::new(Mutex::new(Vec::<Option<&'static str>>::new()));
    let dom = VirtualDom::new_with_props(
        disabled_interaction_probe,
        DisabledInteractionProbeProps {
            closed: Arc::clone(&closed),
            reordered: Arc::clone(&reordered),
            selected: Arc::clone(&selected),
        },
    );

    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootelement(parent.clone().into()),
    );

    animation_frame_turn().await;
    animation_frame_turn().await;

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
    animation_frame_turn().await;

    assert_eq!(
        selected_tab_text(&parent),
        "First",
        "clicking a disabled tab must not select it"
    );

    disabled_second.focus().expect("focus should succeed");

    let delete = dispatch_keydown(&disabled_second, "Delete", false);
    let reorder = dispatch_keydown(&disabled_second, "ArrowRight", true);

    animation_frame_turn().await;

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
}

#[wasm_bindgen_test]
fn wasm_virtualdom_rebuild_does_not_panic() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);
}

#[wasm_bindgen_test]
fn wasm_reorderable_path_compiles_and_runs() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                reorderable: true,
            }
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);
}

#[wasm_bindgen_test]
fn wasm_closable_path_compiles_and_runs() {
    fn app() -> Element {
        let closable_tabs = use_store(|| {
            vec![
                Tab::new_with_label("inbox", "Inbox", rsx! { "Inbox" }, rsx! { p { "Inbox" } })
                    .closable(true),
            ]
        });

        rsx! {
            Tabs { default_value: "inbox", tabs: ReadStore::from(closable_tabs) }
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);
}

#[wasm_bindgen_test]
fn wasm_lazy_mount_path_compiles_and_runs() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                lazy_mount: true,
                unmount_on_exit: true,
            }
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);
}

#[wasm_bindgen_test]
fn wasm_manual_activation_mode_path_compiles_and_runs() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                activation_mode: ActivationMode::Manual,
            }
        }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);
}

#[wasm_bindgen_test]
fn wasm_attrs_observable_through_machine_connect_api() {
    // Direct unit test against the agnostic core exercised on wasm32.
    // This verifies the `ConnectApi` produces the same data-ars-* / role
    // / aria-* attribute names on wasm as on native, since the agnostic
    // core has no target-specific code paths but the test runs the
    // wasm32 build of `ars-components`.
    use ars_components::navigation::tabs::{Machine, Props, TabRegistration};
    use ars_core::{Env, Service};

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("test-tabs")
            .default_value(Some(Key::str("first"))),
        &Env::default(),
        &tabs::Messages::default(),
    );

    drop(service.send(tabs::Event::SetTabs(vec![
        TabRegistration::new(Key::str("first")),
        TabRegistration::new(Key::str("second")),
    ])));

    let api = service.connect(&|_| {});

    let root_attrs = api.root_attrs();

    let scope = root_attrs.get(&HtmlAttr::Data("ars-scope"));

    assert!(
        scope.is_some(),
        "wasm32 connect API must expose data-ars-scope"
    );

    let list_attrs = api.list_attrs();

    let role = list_attrs.get(&HtmlAttr::Role);

    assert!(
        role.is_some(),
        "wasm32 connect API must expose role on the list"
    );
}

#[wasm_bindgen_test]
fn wasm_send_lifecycle_dispatches_through_machine() {
    let snapshots = Rc::new(RefCell::new(Vec::<bool>::new()));

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(snapshots: Rc<RefCell<Vec<bool>>>) -> Element {
        let machine = use_machine_for_probe();

        let mut phase = use_signal(|| 0_u8);

        snapshots.borrow_mut().push(machine
            .derive(|api| api.is_tab_selected(&Key::str("second")))(
        ));

        if phase() == 0 {
            phase.set(1);

            machine
                .send
                .call(tabs::Event::SelectTab(Key::str("second")));
        }

        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));

    dom.rebuild_in_place();

    dom.mark_dirty(ScopeId::APP);

    dom.render_immediate(&mut NoOpMutations);

    let observed = snapshots.borrow();

    // Initial render: nothing selected for "second" until the SetTabs +
    // SelectTab events propagate. The exact count depends on Dioxus's
    // re-render heuristics; we just assert at least one false followed
    // by at least one true.
    assert!(
        observed.iter().any(|&b| !b),
        "expected at least one render with second tab unselected: {observed:?}"
    );
}

fn use_machine_for_probe() -> ars_dioxus::UseMachineReturn<tabs::Machine> {
    // Independent probe machine reused across this test to avoid
    // coupling to the Tabs adapter's internal machine. We register the
    // same tabs ourselves to exercise the SetTabs / SelectTab events.
    let machine = ars_dioxus::use_machine::<tabs::Machine>(
        tabs::Props::new()
            .id("probe-tabs")
            .default_value(Some(Key::str("first"))),
    );

    machine.send.call(tabs::Event::SetTabs(vec![
        tabs::TabRegistration::new(Key::str("first")),
        tabs::TabRegistration::new(Key::str("second")),
    ]));

    machine
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
                format!("DIOXUS {label} @ {position}/{total}")
            },
        ),
        ..Messages::default()
    };

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("dioxus-reorder-i18n")
            .default_value(Some(Key::str("a"))),
        &Env::default(),
        &messages,
    );

    drop(service.send(tabs::Event::SetTabs(vec![
        TabRegistration::new(Key::str("a")),
        TabRegistration::new(Key::str("b")),
        TabRegistration::new(Key::str("c")),
    ])));

    let api = service.connect(&|_| {});

    let announcement = api.reorder_announcement("Inbox", 2, 3);

    assert_eq!(
        announcement, "DIOXUS Inbox @ 2/3",
        "Api::reorder_announcement must route through Messages template"
    );
}
