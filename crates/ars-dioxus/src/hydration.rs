//! SSR hydration helpers for the Dioxus adapter.
//!
//! The adapter reuses [`ars_core::HydrationSnapshot`] as the single wire-format
//! contract for stateful machine hydration and provides browser-only helpers
//! for `FocusScope` cleanup once client hydration has completed.

#[cfg(feature = "ssr")]
pub use ars_core::HydrationSnapshot;
#[cfg(feature = "ssr")]
use ars_core::{HasId, Machine, Service};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use dioxus::prelude::{ReadableExt, WritableExt};

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
#[cfg(all(feature = "web", target_arch = "wasm32"))]
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
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn warn_if_mounted_id_mismatch(element: &web_sys::Element, client_id: &str) {
    let server_id = element.id();

    if server_id.is_empty() {
        return;
    }

    if server_id == client_id {
        return;
    }

    #[cfg(debug_assertions)]
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
        &hydration_id_mismatch_message(&server_id, client_id),
    ));
}

/// Hydration-safe `FocusScope` setup for modal overlays.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn setup_focus_scope_hydration_safe(
    scope_id: String,
    restore_target: dioxus::prelude::Signal<Option<web_sys::HtmlElement>>,
) {
    use dioxus::prelude::{use_drop, use_effect};

    let cleanup_scope_id = scope_id.clone();

    use_effect({
        move || {
            let Some(document) = browser_document() else {
                return;
            };

            if body_is_hydrated(&document) {
                activate_focus_scope(&document, &scope_id, restore_target);
            } else {
                request_hydration_activation_after_frame(scope_id.clone(), restore_target);
            }
        }
    });

    use_drop(move || {
        if let Some(document) = browser_document()
            && let Some(scope) = document.get_element_by_id(&cleanup_scope_id)
        {
            drop(scope.remove_attribute("data-ars-modal-open"));
        }

        if let Some(element) = restore_target.peek().as_ref() {
            drop(element.focus());
        }
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn browser_document() -> Option<web_sys::Document> {
    web_sys::window()?.document()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn body_is_hydrated(document: &web_sys::Document) -> bool {
    document
        .body()
        .and_then(|body| body.get_attribute("data-ars-hydrated"))
        .is_some()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn activate_focus_scope(
    document: &web_sys::Document,
    scope_id: &str,
    restore_target: dioxus::prelude::Signal<Option<web_sys::HtmlElement>>,
) {
    let scope: Option<web_sys::HtmlElement> = document
        .get_element_by_id(scope_id)
        .and_then(|element| wasm_bindgen::JsCast::dyn_into(element).ok());

    if let Some(scope) = scope.as_ref() {
        drop(scope.set_attribute("data-ars-modal-open", ""));
    }

    remove_orphaned_inert(document);

    if let Some(scope) = scope {
        request_focus_after_frame(scope, restore_target);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn request_hydration_activation_after_frame(
    scope_id: String,
    restore_target: dioxus::prelude::Signal<Option<web_sys::HtmlElement>>,
) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };

    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        let Some(document) = browser_document() else {
            return;
        };

        if body_is_hydrated(&document) {
            activate_focus_scope(&document, &scope_id, restore_target);
        }
    });

    drop(window.request_animation_frame(callback.unchecked_ref()));
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn remove_orphaned_inert(document: &web_sys::Document) {
    let Ok(elements) = document.query_selector_all("[inert]") else {
        return;
    };

    for index in 0..elements.length() {
        let Some(node) = elements.item(index) else {
            continue;
        };

        let Ok(element) = wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(node) else {
            continue;
        };

        if !has_open_modal_sibling(&element) {
            drop(element.remove_attribute("inert"));
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
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

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn visible_focus_target(scope: &web_sys::HtmlElement) -> Option<web_sys::HtmlElement> {
    scope
        .query_selector("[autofocus], [tabindex]")
        .ok()
        .flatten()
        .and_then(|element| wasm_bindgen::JsCast::dyn_into(element).ok())
        .filter(|element: &web_sys::HtmlElement| element.offset_parent().is_some())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn request_focus_after_frame(
    scope: web_sys::HtmlElement,
    mut restore_target: dioxus::prelude::Signal<Option<web_sys::HtmlElement>>,
) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };

    let Some(document) = window.document() else {
        return;
    };

    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        let Some(target) = visible_focus_target(&scope) else {
            return;
        };

        restore_target.set(
            document
                .active_element()
                .and_then(|element| element.dyn_into().ok()),
        );

        drop(target.focus());
    });

    drop(window.request_animation_frame(callback.unchecked_ref()));
}

#[cfg(all(feature = "web", target_arch = "wasm32", debug_assertions))]
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

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{cell::Cell, rc::Rc};

    use dioxus::{
        dioxus_core::{NoOpMutations, ScopeId},
        prelude::*,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::HtmlElement;

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

    fn append_html(markup: &str) -> HtmlElement {
        let document = document();

        let container: HtmlElement = document
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
                .expect("requestAnimationFrame should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
    }

    #[wasm_bindgen_test]
    fn setup_focus_scope_skips_without_hydrated_body() {
        reset_body();

        append_html(
            r#"<main inert></main><section id="scope"><button tabindex="0">focus</button></section>"#,
        );

        let ran = Rc::new(Cell::new(false));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(ran: Rc<Cell<bool>>) -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target);

            ran.set(true);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&ran));

        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);

        assert!(ran.get());
        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some(),
            "inert should remain until body is marked hydrated"
        );
    }

    #[wasm_bindgen_test(async)]
    async fn setup_focus_scope_activates_when_body_is_marked_hydrated_next_frame() {
        reset_body();

        append_html(r#"<main inert></main><section id="scope"></section>"#);

        fn app() -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);

        let scope = document()
            .get_element_by_id("scope")
            .expect("scope should exist");

        assert!(!scope.has_attribute("data-ars-modal-open"));
        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some(),
            "inert should remain before the body is marked hydrated"
        );

        document()
            .body()
            .expect("body should exist")
            .set_attribute("data-ars-hydrated", "")
            .expect("mark hydrated");

        animation_frame_turn().await;

        assert!(scope.has_attribute("data-ars-modal-open"));
        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some(),
            "inert remains while the hydrated modal scope is active"
        );
    }

    #[wasm_bindgen_test(async)]
    async fn setup_focus_scope_stays_inactive_when_body_remains_unhydrated_after_frame() {
        reset_body();

        let container = append_html(
            r#"<button id="before">before</button><main inert></main><section id="scope"><button id="target" tabindex="0">target</button></section>"#,
        );

        let before: HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");

        before.focus().expect("focus before");

        fn app() -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);

        animation_frame_turn().await;

        let scope = document()
            .get_element_by_id("scope")
            .expect("scope should exist");

        assert!(!scope.has_attribute("data-ars-modal-open"));
        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some(),
            "inert should remain when hydration never completes"
        );
        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("before")
        );
    }

    #[wasm_bindgen_test]
    fn setup_focus_scope_cleans_orphaned_inert_when_hydrated_scope_is_missing() {
        reset_body();

        document()
            .body()
            .expect("body should exist")
            .set_attribute("data-ars-hydrated", "")
            .expect("mark hydrated");

        append_html(r#"<main inert></main>"#);

        fn app() -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            super::setup_focus_scope_hydration_safe(String::from("missing-scope"), restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);

        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_none(),
            "orphaned inert should be removed even when the scope is absent"
        );
        assert!(
            document()
                .query_selector("[data-ars-modal-open]")
                .expect("query should succeed")
                .is_none()
        );
    }

    #[wasm_bindgen_test]
    fn orphaned_inert_is_removed_without_open_modal_sibling() {
        reset_body();

        document()
            .body()
            .expect("body should exist")
            .set_attribute("data-ars-hydrated", "")
            .expect("mark hydrated");

        append_html(r#"<main inert></main><section id="scope"></section>"#);

        super::remove_orphaned_inert(&document());

        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_none()
        );
    }

    #[wasm_bindgen_test]
    fn inert_is_retained_with_open_modal_sibling() {
        reset_body();

        append_html(r#"<main inert></main><section data-ars-modal-open></section>"#);

        super::remove_orphaned_inert(&document());

        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some()
        );
    }

    #[wasm_bindgen_test]
    fn setup_focus_scope_marks_and_cleans_up_modal_scope() {
        reset_body();

        document()
            .body()
            .expect("body should exist")
            .set_attribute("data-ars-hydrated", "")
            .expect("mark hydrated");

        append_html(r#"<main inert></main><section id="scope"></section>"#);

        fn app() -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);

        let scope = document()
            .get_element_by_id("scope")
            .expect("scope should exist");

        assert!(scope.has_attribute("data-ars-modal-open"));
        assert!(
            document()
                .query_selector("[inert]")
                .expect("query should succeed")
                .is_some(),
            "inert remains while the hydrated modal scope is active"
        );

        drop(dom);

        let scope = document()
            .get_element_by_id("scope")
            .expect("scope should still exist");

        assert!(!scope.has_attribute("data-ars-modal-open"));
    }

    #[wasm_bindgen_test]
    fn visible_focus_target_skips_display_none_elements() {
        reset_body();

        let container = append_html(
            r#"<section id="scope"><button tabindex="0" style="display: none">hidden</button></section>"#,
        );

        let scope: HtmlElement = container
            .query_selector("#scope")
            .expect("query should succeed")
            .expect("scope should exist")
            .dyn_into()
            .expect("scope should be HtmlElement");

        assert!(super::visible_focus_target(&scope).is_none());
    }

    #[wasm_bindgen_test(async)]
    async fn focus_activation_is_deferred_until_animation_frame() {
        reset_body();

        let container = append_html(
            r#"<button id="before">before</button><section id="scope"><button id="target" tabindex="0">target</button></section>"#,
        );

        let before: HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");

        before.focus().expect("focus before");

        fn app() -> Element {
            let restore_target = use_signal(|| None::<HtmlElement>);

            let scope: HtmlElement = document()
                .get_element_by_id("scope")
                .expect("scope should exist")
                .dyn_into()
                .expect("scope should be HtmlElement");

            super::request_focus_after_frame(scope, restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();

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

    #[wasm_bindgen_test]
    fn use_drop_cleanup_restores_previous_focus() {
        reset_body();

        document()
            .body()
            .expect("body should exist")
            .set_attribute("data-ars-hydrated", "")
            .expect("mark hydrated");

        let container = append_html(
            r#"<button id="before">before</button><section id="scope"><button id="target" tabindex="0">target</button></section>"#,
        );

        let before: HtmlElement = container
            .query_selector("#before")
            .expect("query should succeed")
            .expect("before should exist")
            .dyn_into()
            .expect("before should be HtmlElement");

        before.focus().expect("focus before");

        fn app() -> Element {
            let mut restore_target = use_signal(|| None::<HtmlElement>);

            restore_target.set(
                document()
                    .get_element_by_id("before")
                    .and_then(|element| element.dyn_into().ok()),
            );

            super::setup_focus_scope_hydration_safe(String::from("scope"), restore_target);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();

        let mut mutations = NoOpMutations;

        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut mutations);

        drop(dom);

        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.get_attribute("id"))
                .as_deref(),
            Some("before")
        );
    }

    #[wasm_bindgen_test]
    fn hydration_mismatch_warning_message_matches_contract() {
        let element = document()
            .create_element("div")
            .expect("create element for mismatch check");

        element.set_id("ars-dialog-7");

        super::warn_if_mounted_id_mismatch(&element, "ars-dialog-9");

        assert_eq!(
            super::hydration_id_mismatch_message("ars-dialog-7", "ars-dialog-9"),
            "ars-ui hydration ID mismatch: server='ars-dialog-7', client='ars-dialog-9'. Component IDs may be non-deterministic across SSR/client boundaries."
        );
    }

    #[wasm_bindgen_test]
    fn hydration_mismatch_warning_helper_accepts_empty_and_matching_server_ids() {
        let empty = document()
            .create_element("div")
            .expect("create empty-id element for mismatch check");

        super::warn_if_mounted_id_mismatch(&empty, "ars-dialog-9");

        let matching = document()
            .create_element("div")
            .expect("create matching-id element for mismatch check");

        matching.set_id("ars-dialog-9");

        super::warn_if_mounted_id_mismatch(&matching, "ars-dialog-9");
    }
}
