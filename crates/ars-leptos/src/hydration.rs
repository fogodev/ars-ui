//! SSR hydration helpers for the Leptos adapter.
//!
//! The adapter reuses [`ars_core::HydrationSnapshot`] as the single wire-format
//! contract for stateful machine hydration and provides browser-only helpers
//! for `FocusScope` cleanup once client hydration has completed.

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(any(feature = "ssr", all(feature = "hydrate", target_arch = "wasm32")))]
pub use ars_core::HydrationSnapshot;
#[cfg(feature = "ssr")]
use ars_core::{HasId, Machine, Service};
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use leptos::{
    prelude::*,
    reactive::owner::LocalStorage,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
    web_sys,
};

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
const HYDRATION_FOCUS_TARGET_SELECTOR: &str = concat!(
    "[autofocus]:not([disabled]):not([aria-hidden='true']),",
    "button:not([disabled]):not([aria-hidden='true']),",
    "input:not([disabled]):not([aria-hidden='true']),",
    "select:not([disabled]):not([aria-hidden='true']),",
    "textarea:not([disabled]):not([aria-hidden='true']),",
    "a[href]:not([aria-hidden='true']),",
    "area[href]:not([aria-hidden='true']),",
    "[tabindex]:not([disabled]):not([aria-hidden='true']),",
    "[contenteditable]:not([contenteditable='false']):not([aria-hidden='true'])",
);

/// Serializes a machine service snapshot for embedding in SSR HTML.
///
/// The returned JSON contains only the machine state and component ID. Context
/// is intentionally recomputed on the client by [`Service::new_hydrated`].
///
/// # Panics
///
/// Panics if `M::State` cannot be serialized.
#[cfg(feature = "ssr")]
#[must_use]
pub fn serialize_snapshot<M: Machine>(svc: &Service<M>) -> String
where
    M::State: Clone + serde::Serialize,
{
    serde_json::to_string(&HydrationSnapshot::<M> {
        state: svc.state().clone(),
        id: svc.props().id().to_string(),
    })
    .expect("HydrationSnapshot must be serializable for SSR — ensure State implements Serialize")
}

/// Marks the document body as fully hydrated for `FocusScope` activation gates.
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub fn mark_body_hydrated() {
    let Some(document) = browser_document() else {
        return;
    };

    let Some(body) = document.body() else {
        return;
    };

    drop(body.set_attribute("data-ars-hydrated", ""));
}

/// Emits a debug warning when a mounted server-rendered ID differs from the client ID.
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub fn warn_if_mounted_id_mismatch(element: &web_sys::Element, client_id: &str) {
    let server_id = element.id();

    if server_id.is_empty() {
        return;
    }

    if server_id == client_id {
        return;
    }

    #[cfg(debug_assertions)]
    web_sys::console::warn_1(&JsValue::from_str(&hydration_id_mismatch_message(
        &server_id, client_id,
    )));
}

/// Hydration-safe `FocusScope` setup for modal overlays.
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub fn setup_focus_scope_hydration_safe(
    scope_id: String,
    restore_target: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
) {
    let cleanup_scope_id = scope_id.clone();
    let activation_active = Arc::new(AtomicBool::new(true));

    Effect::new({
        let activation_active = Arc::clone(&activation_active);

        move |_| {
            let Some(document) = browser_document() else {
                return;
            };

            if body_is_hydrated(&document) {
                activate_focus_scope(
                    &document,
                    &scope_id,
                    restore_target,
                    Arc::clone(&activation_active),
                );
            } else {
                request_hydration_activation_after_frame(
                    scope_id.clone(),
                    restore_target,
                    Arc::clone(&activation_active),
                );
            }
        }
    });

    on_cleanup(move || {
        activation_active.store(false, Ordering::Relaxed);

        if let Some(document) = browser_document()
            && let Some(scope) = document.get_element_by_id(&cleanup_scope_id)
        {
            drop(scope.remove_attribute("data-ars-modal-open"));
        }

        restore_target.with_value(|element| {
            if let Some(element) = element {
                drop(element.focus());
            }
        });
    });
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn browser_document() -> Option<web_sys::Document> {
    web_sys::window()?.document()
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn body_is_hydrated(document: &web_sys::Document) -> bool {
    document
        .body()
        .and_then(|body| body.get_attribute("data-ars-hydrated"))
        .is_some()
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn activate_focus_scope(
    document: &web_sys::Document,
    scope_id: &str,
    restore_target: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    activation_active: Arc<AtomicBool>,
) {
    let scope: Option<web_sys::HtmlElement> = document
        .get_element_by_id(scope_id)
        .and_then(|element| element.dyn_into().ok());

    if let Some(scope) = scope.as_ref() {
        drop(scope.set_attribute("data-ars-modal-open", ""));
    }

    remove_orphaned_inert(document);

    if let Some(scope) = scope {
        request_focus_after_frame(scope, restore_target, activation_active);
    }
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn request_hydration_activation_after_frame(
    scope_id: String,
    restore_target: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    activation_active: Arc<AtomicBool>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback = Closure::once_into_js(move || {
        if !activation_active.load(Ordering::Relaxed) {
            return;
        }

        let Some(document) = browser_document() else {
            return;
        };

        if body_is_hydrated(&document) {
            activate_focus_scope(&document, &scope_id, restore_target, activation_active);
        } else {
            request_hydration_activation_after_frame(scope_id, restore_target, activation_active);
        }
    });

    drop(window.request_animation_frame(callback.unchecked_ref()));
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn remove_orphaned_inert(document: &web_sys::Document) {
    let Ok(elements) = document.query_selector_all("[inert][data-ars-modal-inert]") else {
        return;
    };

    for index in 0..elements.length() {
        let Some(node) = elements.item(index) else {
            continue;
        };

        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };

        if !has_open_modal_sibling(&element) {
            drop(element.remove_attribute("inert"));
            drop(element.remove_attribute(ars_dom::portal::MODAL_INERT_ATTR));
        }
    }
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn has_open_modal_sibling(element: &web_sys::Element) -> bool {
    let Some(parent) = element.parent_element() else {
        return false;
    };

    let children = parent.children();

    for index in 0..children.length() {
        let Some(child) = children.item(index) else {
            continue;
        };

        if child.is_same_node(Some(element)) {
            continue;
        }

        if child.has_attribute("data-ars-modal-open") {
            return true;
        }
    }

    false
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn visible_focus_target(scope: &web_sys::HtmlElement) -> Option<web_sys::HtmlElement> {
    if is_focus_target_candidate(scope) && is_visible_focus_target(scope) {
        return Some(scope.clone());
    }

    scope
        .query_selector_all(HYDRATION_FOCUS_TARGET_SELECTOR)
        .ok()
        .and_then(|elements| first_visible_focus_target(&elements))
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn first_visible_focus_target(elements: &web_sys::NodeList) -> Option<web_sys::HtmlElement> {
    for index in 0..elements.length() {
        let Some(node) = elements.item(index) else {
            continue;
        };

        let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() else {
            continue;
        };

        if is_visible_focus_target(&element) {
            return Some(element);
        }
    }

    None
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn is_focus_target_candidate(element: &web_sys::Element) -> bool {
    if element.has_attribute("disabled") {
        return false;
    }

    if element.get_attribute("aria-hidden").as_deref() == Some("true") {
        return false;
    }

    if element.has_attribute("autofocus") || element.has_attribute("tabindex") {
        return true;
    }

    match element.tag_name().as_str() {
        "BUTTON" | "INPUT" | "SELECT" | "TEXTAREA" => true,
        "A" | "AREA" => element.has_attribute("href"),
        _ => element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false"),
    }
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn is_visible_focus_target(element: &web_sys::HtmlElement) -> bool {
    element.offset_parent().is_some() || is_visible_fixed_position_target(element)
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn is_visible_fixed_position_target(element: &web_sys::HtmlElement) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };

    let Ok(Some(style)) = window.get_computed_style(element) else {
        return false;
    };

    style.get_property_value("position").as_deref() == Ok("fixed")
        && style.get_property_value("display").as_deref() != Ok("none")
        && style.get_property_value("visibility").as_deref() != Ok("hidden")
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn request_focus_after_frame(
    scope: web_sys::HtmlElement,
    restore_target: StoredValue<Option<web_sys::HtmlElement>, LocalStorage>,
    activation_active: Arc<AtomicBool>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let Some(document) = window.document() else {
        return;
    };

    let callback = Closure::once_into_js(move || {
        if !activation_active.load(Ordering::Relaxed) {
            return;
        }

        let Some(target) = visible_focus_target(&scope) else {
            return;
        };

        restore_target.set_value(
            document
                .active_element()
                .and_then(|element| element.dyn_into().ok()),
        );

        drop(target.focus());
    });

    drop(window.request_animation_frame(callback.unchecked_ref()));
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32", debug_assertions))]
fn hydration_id_mismatch_message(server_id: &str, client_id: &str) -> String {
    format!(
        "ars-ui hydration ID mismatch: server='{server_id}', client='{client_id}'. Component IDs may be non-deterministic across SSR/client boundaries."
    )
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use ars_core::{AttrMap, ComponentPart, ConnectApi, Env, HasId, Machine, Service};

    #[cfg_attr(feature = "ssr", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestState {
        Off,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestProps {
        id: String,
    }

    impl HasId for TestProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TestPart;

    impl ComponentPart for TestPart {
        const ROOT: Self = Self;

        fn scope() -> &'static str {
            "test"
        }

        fn name(&self) -> &'static str {
            "root"
        }

        fn all() -> Vec<Self> {
            vec![Self]
        }
    }

    struct TestApi;

    impl ConnectApi for TestApi {
        type Part = TestPart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct TestMachine;

    impl Machine for TestMachine {
        type State = TestState;
        type Event = ();
        type Context = ();
        type Props = TestProps;
        type Messages = ();
        type Api<'a> = TestApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (TestState::Off, ())
        }

        fn transition(
            _state: &Self::State,
            _event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            None
        }

        fn connect<'a>(
            _state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            TestApi
        }
    }

    #[test]
    fn serialize_snapshot_round_trips_state_and_id() {
        let service = Service::<TestMachine>::new(
            TestProps {
                id: String::from("toggle-1"),
            },
            &Env::default(),
            &(),
        );

        let json = super::serialize_snapshot(&service);

        let snapshot: super::HydrationSnapshot<TestMachine> =
            serde_json::from_str(&json).expect("snapshot should deserialize");

        assert_eq!(snapshot.state, TestState::Off);
        assert_eq!(snapshot.id, "toggle-1");
        assert!(json.contains("\"id\":\"toggle-1\""));
        assert!(json.contains("\"state\":\"Off\""));
    }
}

#[cfg(all(test, feature = "hydrate", target_arch = "wasm32"))]
mod wasm_tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use leptos::{
        prelude::*,
        reactive::owner::LocalStorage,
        wasm_bindgen::{self, JsCast},
        web_sys,
    };
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .expect("window should exist")
            .document()
            .expect("document should exist")
    }

    fn reset_body() {
        let document = document();
        let body = document.body().expect("body should exist");

        drop(body.remove_attribute("data-ars-hydrated"));

        let containers = document
            .query_selector_all("[data-ars-hydration-test]")
            .expect("query hydration test containers");

        for index in 0..containers.length() {
            let Some(container) = containers.item(index) else {
                continue;
            };

            if let Some(parent) = container.parent_node() {
                drop(parent.remove_child(&container));
            }
        }
    }

    fn append_html(markup: &str) -> web_sys::HtmlElement {
        let document = document();
        let container: web_sys::HtmlElement = document
            .create_element("div")
            .expect("create container")
            .dyn_into()
            .expect("container should be HtmlElement");

        container
            .set_attribute("data-ars-hydration-test", "")
            .expect("mark hydration test container");
        container.set_inner_html(markup);

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append container");

        container
    }

    async fn animation_frame_turn() {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
                drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
            });

            web_sys::window()
                .expect("window should exist")
                .request_animation_frame(callback.unchecked_ref())
                .expect("requestAnimationFrame should schedule");
        });

        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .expect("animation frame promise should resolve");
    }

    fn restore_target() -> StoredValue<Option<web_sys::HtmlElement>, LocalStorage> {
        StoredValue::new_local(None)
    }

    #[wasm_bindgen_test]
    fn mark_body_hydrated_sets_marker() {
        reset_body();

        super::mark_body_hydrated();

        assert_eq!(
            document()
                .body()
                .expect("body should exist")
                .get_attribute("data-ars-hydrated")
                .as_deref(),
            Some("")
        );
    }

    #[wasm_bindgen_test]
    fn orphaned_inert_is_removed_without_open_modal_sibling() {
        reset_body();

        let container = append_html(r#"<main id="page" inert data-ars-modal-inert=""></main>"#);
        let document = document();

        super::remove_orphaned_inert(&document);

        let page = container
            .query_selector("#page")
            .expect("query should succeed")
            .expect("page should exist");

        assert_eq!(page.get_attribute("inert"), None);
        assert_eq!(page.get_attribute(ars_dom::portal::MODAL_INERT_ATTR), None);
    }

    #[wasm_bindgen_test]
    fn unowned_inert_is_not_removed_without_open_modal_sibling() {
        reset_body();

        let container = append_html(r#"<main id="page" inert></main>"#);
        let document = document();

        super::remove_orphaned_inert(&document);

        let page = container
            .query_selector("#page")
            .expect("query should succeed")
            .expect("page should exist");

        assert_eq!(page.get_attribute("inert").as_deref(), Some(""));
    }

    #[wasm_bindgen_test]
    fn inert_is_retained_with_open_modal_sibling() {
        reset_body();

        let container = append_html(
            r#"<main id="page" inert data-ars-modal-inert=""></main><section id="scope" data-ars-modal-open=""></section>"#,
        );
        let document = document();

        super::remove_orphaned_inert(&document);

        let page = container
            .query_selector("#page")
            .expect("query should succeed")
            .expect("page should exist");

        assert_eq!(page.get_attribute("inert").as_deref(), Some(""));
        assert_eq!(
            page.get_attribute(ars_dom::portal::MODAL_INERT_ATTR)
                .as_deref(),
            Some("")
        );
    }

    #[wasm_bindgen_test]
    fn visible_focus_target_skips_hidden_matches_until_visible_target() {
        reset_body();

        let container = append_html(
            r#"<section id="scope"><button id="hidden" style="display: none">hidden</button><input id="target"></section>"#,
        );
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        assert_eq!(
            super::visible_focus_target(&scope)
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("target")
        );
    }

    #[wasm_bindgen_test]
    fn visible_focus_target_accepts_fixed_position_target() {
        reset_body();

        let container = append_html(
            r#"<section id="scope"><button id="target" style="position: fixed">target</button></section>"#,
        );
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        assert_eq!(
            super::visible_focus_target(&scope)
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("target")
        );
    }

    #[wasm_bindgen_test]
    fn visible_focus_target_rejects_hidden_fixed_position_target() {
        reset_body();

        let container = append_html(
            r#"<section id="scope"><button id="target" style="position: fixed; visibility: hidden">target</button></section>"#,
        );
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        assert!(super::visible_focus_target(&scope).is_none());
    }

    #[wasm_bindgen_test]
    fn visible_focus_target_prefers_focusable_scope_root() {
        reset_body();

        let container = append_html(
            r#"<section id="scope" tabindex="-1"><button id="target">target</button></section>"#,
        );
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        assert_eq!(
            super::visible_focus_target(&scope)
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("scope")
        );
    }

    #[wasm_bindgen_test(async)]
    async fn focus_activation_is_deferred_until_animation_frame() {
        reset_body();

        let container = append_html(
            r#"<button id="before">before</button><section id="scope"><button id="target">target</button></section>"#,
        );
        let before: web_sys::HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        before.focus().expect("focus before");

        super::request_focus_after_frame(scope, restore_target(), Arc::new(AtomicBool::new(true)));

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("before")
        );

        animation_frame_turn().await;

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("target")
        );
    }

    #[wasm_bindgen_test(async)]
    async fn focus_activation_skips_after_unmount_guard_is_cleared() {
        reset_body();

        let container = append_html(
            r#"<button id="before">before</button><section id="scope"><button id="target">target</button></section>"#,
        );
        let before: web_sys::HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");
        let scope: web_sys::HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");
        let active = Arc::new(AtomicBool::new(true));

        before.focus().expect("focus before");

        super::request_focus_after_frame(scope, restore_target(), Arc::clone(&active));
        active.store(false, Ordering::Relaxed);
        animation_frame_turn().await;

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("before")
        );
    }

    #[wasm_bindgen_test(async)]
    async fn setup_focus_scope_retries_until_body_is_marked_hydrated() {
        reset_body();

        let container = append_html(
            r#"<button id="before">before</button><section id="scope"><button id="target">target</button></section>"#,
        );
        let before: web_sys::HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");

        before.focus().expect("focus before");

        let owner = Owner::new();
        owner.with(|| {
            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target());
        });

        leptos::task::tick().await;
        animation_frame_turn().await;

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("before")
        );

        super::mark_body_hydrated();
        animation_frame_turn().await;
        animation_frame_turn().await;

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("target")
        );
    }

    #[wasm_bindgen_test]
    fn hydration_mismatch_warning_message_matches_contract() {
        assert_eq!(
            super::hydration_id_mismatch_message("ars-dialog-7", "ars-dialog-9"),
            "ars-ui hydration ID mismatch: server='ars-dialog-7', client='ars-dialog-9'. Component IDs may be non-deterministic across SSR/client boundaries."
        );
    }
}
