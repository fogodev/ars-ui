//! Portal root and background inert utilities for browser-backed adapters.

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use std::string::String;

/// HTML attribute name carrying the owning overlay id on a portal mount node.
///
/// Outside-interaction detection (`crate::outside_interaction`) walks DOM
/// ancestors comparing this attribute against the registered overlay /
/// inside-boundary ids so portaled subtrees are recognised as "inside" their
/// owning overlay.
pub const PORTAL_OWNER_ATTR: &str = "data-ars-portal-owner";

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    crate::focus::{focus_element, get_element_by_id, get_tabbable_elements},
    js_sys::{Math, Reflect},
    std::vec::Vec,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

/// Returns the shared portal root element, creating it when needed.
///
/// All overlay components render into this container so background inert
/// management can operate on a single portal boundary.
#[cfg(feature = "web")]
#[must_use]
pub fn get_or_create_portal_root() -> web_sys::Element {
    get_or_create_portal_root_impl()
}

/// Returns the per-instance mount root for a given portal owner.
///
/// The returned node is a child of the shared portal root and carries the
/// `data-ars-portal-owner` marker used by outside-interaction detection.
#[cfg(feature = "web")]
#[must_use]
pub fn ensure_portal_mount_root(owner_id: &str) -> web_sys::Element {
    ensure_portal_mount_root_impl(owner_id)
}

/// Returns whether the current browser supports the native `inert` attribute.
#[cfg(feature = "web")]
#[must_use]
pub fn supports_inert() -> bool {
    supports_inert_impl()
}

/// Applies background inert handling to siblings of the portal root.
///
/// On browsers with native inert support, this sets `inert` and
/// `aria-hidden="true"` on body siblings. On older browsers it falls back to
/// an `aria-hidden`/`tabindex` polyfill plus a Tab-key containment listener.
#[cfg(feature = "web")]
#[must_use]
pub fn set_background_inert(portal_root_id: &str) -> Box<dyn FnOnce()> {
    set_background_inert_impl(portal_root_id, supports_inert())
}

/// Removes `inert` and `aria-hidden` from siblings of the given portal node.
///
/// This is a best-effort cleanup helper used by higher-level close flows that
/// need direct sibling clearing without the original cleanup closure.
#[cfg(feature = "web")]
pub fn remove_inert_from_siblings(portal_id: &str) {
    remove_inert_from_siblings_impl(portal_id);
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn portal_mount_id(owner_id: &str) -> String {
    format!("ars-portal-{owner_id}")
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Clone, Debug, PartialEq, Eq)]
enum RestoreAction {
    Remove,
    Set(String),
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
fn restore_action(previous: Option<&str>) -> RestoreAction {
    match previous {
        Some(value) => RestoreAction::Set(value.to_owned()),
        None => RestoreAction::Remove,
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Clone)]
struct SavedSiblingAttrs {
    element: web_sys::Element,
    previous_inert: Option<String>,
    previous_aria_hidden: Option<String>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Clone)]
struct SavedTabIndex {
    element: web_sys::HtmlElement,
    previous_tabindex: Option<String>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct FocusTrapListener {
    document: web_sys::Document,
    listener: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<web_sys::Document> {
    window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn get_or_create_portal_root_impl() -> web_sys::Element {
    let document = document().expect("document exists in browser context");

    if let Some(existing) = document.get_element_by_id("ars-portal-root") {
        if existing.has_attribute("data-ars-managed") {
            return existing;
        }

        crate::debug::warn_message(format_args!(
            "found #ars-portal-root without data-ars-managed; creating alternate root"
        ));

        let suffix = (Math::random() * 1_000_000.0) as u32;

        return create_managed_root(&document, &format!("ars-portal-root-{suffix}"));
    }

    create_managed_root(&document, "ars-portal-root")
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn get_or_create_portal_root_impl() -> web_sys::Element {
    dummy_element()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn create_managed_root(document: &web_sys::Document, id: &str) -> web_sys::Element {
    let root = document
        .create_element("div")
        .expect("portal root creation must succeed");

    root.set_id(id);

    root.set_attribute("data-ars-managed", "")
        .expect("managed marker assignment must succeed");

    document
        .body()
        .expect("document must have body")
        .append_child(&root)
        .expect("portal root append must succeed");

    root
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn ensure_portal_mount_root_impl(owner_id: &str) -> web_sys::Element {
    let portal_root = get_or_create_portal_root_impl();

    let mount_id = portal_mount_id(owner_id);

    if let Some(existing) =
        get_element_by_id(&mount_id).filter(|element| portal_root.contains(Some(element.as_ref())))
    {
        return existing;
    }

    let mount = document()
        .expect("document exists in browser context")
        .create_element("div")
        .expect("portal mount creation must succeed");

    mount.set_id(&mount_id);

    mount
        .set_attribute("data-ars-managed", "")
        .expect("managed marker assignment must succeed");

    mount
        .set_attribute("data-ars-portal-owner", owner_id)
        .expect("portal owner assignment must succeed");

    portal_root
        .append_child(&mount)
        .expect("portal mount append must succeed");

    mount
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn ensure_portal_mount_root_impl(_owner_id: &str) -> web_sys::Element {
    dummy_element()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn supports_inert_impl() -> bool {
    let Ok(html_element_ctor) = Reflect::get(&js_sys::global(), &JsValue::from_str("HTMLElement"))
    else {
        return false;
    };
    let Ok(prototype) = Reflect::get(&html_element_ctor, &JsValue::from_str("prototype")) else {
        return false;
    };

    Reflect::has(&prototype, &JsValue::from_str("inert")).unwrap_or(false)
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
const fn supports_inert_impl() -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn set_background_inert_impl(portal_root_id: &str, inert_supported: bool) -> Box<dyn FnOnce()> {
    let Some(document) = document() else {
        crate::debug::warn_skipped("set_background_inert()", "window.document");

        return Box::new(|| {});
    };

    let Some(body) = document.body() else {
        crate::debug::warn_skipped("set_background_inert()", "document.body");

        return Box::new(|| {});
    };
    let Some(portal_root) = document.get_element_by_id(portal_root_id) else {
        crate::debug::warn_message(format_args!(
            "set_background_inert() skipped because portal root #{portal_root_id} was not found"
        ));

        return Box::new(|| {});
    };

    let siblings = collect_body_siblings(&body, &portal_root);

    if siblings.is_empty() {
        return Box::new(|| {});
    }

    if inert_supported {
        let saved_siblings = apply_aria_hidden(&siblings);

        apply_inert(&siblings);

        return Box::new(move || restore_native_inert(saved_siblings));
    }

    let saved_tabbables = disable_tabbable_descendants(&siblings);

    let saved_siblings = apply_aria_hidden(&siblings);

    let focus_trap = install_focus_trap(&document, &portal_root);

    Box::new(move || {
        restore_polyfill_state(saved_siblings, saved_tabbables, focus_trap);
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn set_background_inert_impl(_portal_root_id: &str, _inert_supported: bool) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn collect_body_siblings(
    body: &web_sys::HtmlElement,
    portal_root: &web_sys::Element,
) -> Vec<web_sys::Element> {
    let children = body.children();

    let mut siblings = Vec::new();

    for index in 0..children.length() {
        let Some(child) = children.item(index) else {
            continue;
        };

        if child.is_same_node(Some(portal_root)) || child.contains(Some(portal_root.as_ref())) {
            continue;
        }

        siblings.push(child);
    }

    siblings
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_aria_hidden(siblings: &[web_sys::Element]) -> Vec<SavedSiblingAttrs> {
    siblings
        .iter()
        .map(|sibling| {
            let previous_aria_hidden = sibling.get_attribute("aria-hidden");

            sibling
                .set_attribute("aria-hidden", "true")
                .expect("aria-hidden assignment must succeed");

            SavedSiblingAttrs {
                element: sibling.clone(),
                previous_inert: sibling.get_attribute("inert"),
                previous_aria_hidden,
            }
        })
        .collect()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_inert(siblings: &[web_sys::Element]) {
    for sibling in siblings {
        sibling
            .set_attribute("inert", "")
            .expect("inert assignment must succeed");
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn disable_tabbable_descendants(siblings: &[web_sys::Element]) -> Vec<SavedTabIndex> {
    let mut saved = Vec::new();

    for sibling in siblings {
        for tabbable in get_tabbable_elements(sibling) {
            let previous_tabindex = tabbable.get_attribute("tabindex");

            tabbable
                .set_attribute("tabindex", "-1")
                .expect("tabindex assignment must succeed");

            saved.push(SavedTabIndex {
                element: tabbable,
                previous_tabindex,
            });
        }
    }

    saved
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_focus_trap(
    document: &web_sys::Document,
    portal_root: &web_sys::Element,
) -> Option<FocusTrapListener> {
    let portal_root = portal_root.clone();

    let active_document = document.clone();

    let listener_document = document.clone();

    let listener = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() != "Tab" || !portal_root.is_connected() {
            return;
        }

        let tabbables = get_tabbable_elements(&portal_root);

        if tabbables.is_empty() {
            return;
        }

        let active_element = active_document
            .active_element()
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());

        let first = &tabbables[0];
        let last = tabbables.last().expect("non-empty tabbable list");

        let active_inside_portal = active_element
            .as_ref()
            .is_some_and(|element| portal_root.contains(Some(element.as_ref())));

        let should_wrap = if event.shift_key() {
            !active_inside_portal
                || active_element
                    .as_ref()
                    .is_some_and(|element| element.is_same_node(Some(first.as_ref())))
        } else {
            !active_inside_portal
                || active_element
                    .as_ref()
                    .is_some_and(|element| element.is_same_node(Some(last.as_ref())))
        };

        if !should_wrap {
            return;
        }

        event.prevent_default();

        if event.shift_key() {
            focus_element(last, false);
        } else {
            focus_element(first, false);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    if listener_document
        .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .is_err()
    {
        crate::debug::warn_message(format_args!("installing focus trap listener failed"));

        return None;
    }

    Some(FocusTrapListener {
        document: listener_document,
        listener,
    })
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_native_inert(saved_siblings: Vec<SavedSiblingAttrs>) {
    for saved in saved_siblings {
        restore_element_attribute(&saved.element, "inert", saved.previous_inert.as_deref());
        restore_element_attribute(
            &saved.element,
            "aria-hidden",
            saved.previous_aria_hidden.as_deref(),
        );
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_polyfill_state(
    saved_siblings: Vec<SavedSiblingAttrs>,
    saved_tabbables: Vec<SavedTabIndex>,
    focus_trap: Option<FocusTrapListener>,
) {
    if let Some(FocusTrapListener { document, listener }) = focus_trap {
        crate::debug::warn_dom_error(
            "removing focus trap listener",
            document
                .remove_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref()),
        );
    }

    for saved in saved_tabbables {
        restore_html_attribute(
            &saved.element,
            "tabindex",
            saved.previous_tabindex.as_deref(),
        );
    }

    for saved in saved_siblings {
        restore_element_attribute(
            &saved.element,
            "aria-hidden",
            saved.previous_aria_hidden.as_deref(),
        );
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_element_attribute(element: &web_sys::Element, name: &str, previous: Option<&str>) {
    match restore_action(previous) {
        RestoreAction::Remove => {
            crate::debug::warn_dom_error(
                &format!("removing attribute {name} during portal cleanup"),
                element.remove_attribute(name),
            );
        }

        RestoreAction::Set(value) => {
            crate::debug::warn_dom_error(
                &format!("restoring attribute {name} during portal cleanup"),
                element.set_attribute(name, &value),
            );
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_html_attribute(element: &web_sys::HtmlElement, name: &str, previous: Option<&str>) {
    match restore_action(previous) {
        RestoreAction::Remove => {
            crate::debug::warn_dom_error(
                &format!("removing attribute {name} during portal cleanup"),
                element.remove_attribute(name),
            );
        }

        RestoreAction::Set(value) => {
            crate::debug::warn_dom_error(
                &format!("restoring attribute {name} during portal cleanup"),
                element.set_attribute(name, &value),
            );
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn remove_inert_from_siblings_impl(portal_id: &str) {
    let Some(document) = document() else {
        crate::debug::warn_skipped("remove_inert_from_siblings()", "window.document");

        return;
    };
    let Some(portal) = document.get_element_by_id(portal_id) else {
        crate::debug::warn_message(format_args!(
            "remove_inert_from_siblings() skipped because portal #{portal_id} was not found"
        ));

        return;
    };
    let Some(parent) = portal.parent_element() else {
        crate::debug::warn_message(format_args!(
            "remove_inert_from_siblings() skipped because portal #{portal_id} has no parent element"
        ));

        return;
    };

    let children = parent.children();
    for index in 0..children.length() {
        let Some(child) = children.item(index) else {
            continue;
        };

        if child.is_same_node(Some(portal.as_ref())) {
            continue;
        }

        crate::debug::warn_dom_error(
            "removing inert from portal sibling",
            child.remove_attribute("inert"),
        );

        crate::debug::warn_dom_error(
            "removing aria-hidden from portal sibling",
            child.remove_attribute("aria-hidden"),
        );
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
const fn remove_inert_from_siblings_impl(_portal_id: &str) {}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn dummy_element() -> web_sys::Element {
    JsValue::NULL.unchecked_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_mount_id_uses_owner_id() {
        assert_eq!(portal_mount_id("dialog-1"), "ars-portal-dialog-1");
    }

    #[test]
    fn restore_action_uses_set_when_previous_value_exists() {
        assert_eq!(
            restore_action(Some("true")),
            RestoreAction::Set(String::from("true"))
        );
    }

    #[test]
    fn restore_action_uses_remove_when_previous_value_is_missing() {
        assert_eq!(restore_action(None), RestoreAction::Remove);
    }
}

#[cfg(all(test, feature = "web", not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;

    #[test]
    fn non_wasm_web_stubs_are_safe_to_call() {
        drop(get_or_create_portal_root());
        drop(ensure_portal_mount_root("dialog-1"));

        assert!(!supports_inert());

        let cleanup = set_background_inert("ars-portal-root");

        cleanup();

        remove_inert_from_siblings("ars-portal-root");
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> web_sys::HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &web_sys::Element, id: &str) -> web_sys::HtmlElement {
        let element = document()
            .create_element("div")
            .expect("div creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("div must be HtmlElement");

        element.set_id(id);

        parent
            .append_child(&element)
            .expect("append_child must succeed");

        element
    }

    fn append_button(
        parent: &web_sys::Element,
        id: &str,
        tabindex: Option<&str>,
    ) -> web_sys::HtmlElement {
        let button = document()
            .create_element("button")
            .expect("button creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("button must be HtmlElement");

        button.set_id(id);

        if let Some(tabindex) = tabindex {
            button
                .set_attribute("tabindex", tabindex)
                .expect("tabindex assignment must succeed");
        }

        parent
            .append_child(&button)
            .expect("append_child must succeed");

        button
    }

    fn cleanup_node(node: &web_sys::Element) {
        node.remove();
    }

    fn clear_managed_portals() {
        if let Some(root) = document().get_element_by_id("ars-portal-root") {
            root.remove();
        }

        let extra_roots = document()
            .query_selector_all("[id^='ars-portal-root-']")
            .expect("selector must be valid");

        for index in 0..extra_roots.length() {
            if let Some(node) = extra_roots.item(index) {
                node.unchecked_into::<web_sys::Element>().remove();
            }
        }
    }

    #[wasm_bindgen_test]
    fn get_or_create_portal_root_is_idempotent() {
        clear_managed_portals();

        let first = get_or_create_portal_root();
        let second = get_or_create_portal_root();

        assert!(first.is_same_node(Some(second.as_ref())));
        assert_eq!(first.id(), "ars-portal-root");
        assert!(first.has_attribute("data-ars-managed"));

        cleanup_node(&first);
    }

    #[wasm_bindgen_test]
    fn existing_managed_root_is_reused() {
        clear_managed_portals();

        let managed = append_div(body().as_ref(), "ars-portal-root");

        managed
            .set_attribute("data-ars-managed", "")
            .expect("marker assignment must succeed");

        let found = get_or_create_portal_root();

        assert!(found.is_same_node(Some(managed.as_ref())));

        cleanup_node(managed.as_ref());
    }

    #[wasm_bindgen_test]
    fn unmanaged_root_creates_alternate_managed_root() {
        clear_managed_portals();

        let unmanaged = append_div(body().as_ref(), "ars-portal-root");

        let found = get_or_create_portal_root();

        assert_ne!(found.id(), "ars-portal-root");
        assert!(found.id().starts_with("ars-portal-root-"));
        assert!(found.has_attribute("data-ars-managed"));

        cleanup_node(unmanaged.as_ref());
        cleanup_node(&found);
    }

    #[wasm_bindgen_test]
    fn ensure_portal_mount_root_creates_owner_marked_mount() {
        clear_managed_portals();

        let mount = ensure_portal_mount_root("dialog-1");

        let root = document()
            .get_element_by_id("ars-portal-root")
            .expect("shared root must exist");

        assert_eq!(mount.id(), "ars-portal-dialog-1");
        assert_eq!(
            mount.get_attribute("data-ars-portal-owner").as_deref(),
            Some("dialog-1")
        );
        assert!(root.contains(Some(mount.as_ref())));

        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn ensure_portal_mount_root_is_idempotent_per_owner() {
        clear_managed_portals();

        let first = ensure_portal_mount_root("dialog-1");
        let second = ensure_portal_mount_root("dialog-1");

        assert!(first.is_same_node(Some(second.as_ref())));

        let root = document()
            .get_element_by_id("ars-portal-root")
            .expect("shared root must exist");

        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn ensure_portal_mount_root_creates_distinct_nodes_for_distinct_owners() {
        clear_managed_portals();

        let first = ensure_portal_mount_root("dialog-1");
        let second = ensure_portal_mount_root("dialog-2");

        assert!(!first.is_same_node(Some(second.as_ref())));
        assert_eq!(
            second.get_attribute("data-ars-portal-owner").as_deref(),
            Some("dialog-2")
        );

        let root = document()
            .get_element_by_id("ars-portal-root")
            .expect("shared root must exist");

        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn native_inert_path_sets_and_restores_attributes() {
        clear_managed_portals();

        let before = append_div(body().as_ref(), "before");

        before
            .set_attribute("aria-hidden", "false")
            .expect("aria-hidden assignment must succeed");

        let after = append_div(body().as_ref(), "after");

        let root = get_or_create_portal_root();

        let cleanup = set_background_inert("ars-portal-root");

        assert_eq!(before.get_attribute("inert").as_deref(), Some(""));
        assert_eq!(before.get_attribute("aria-hidden").as_deref(), Some("true"));
        assert_eq!(after.get_attribute("inert").as_deref(), Some(""));
        assert_eq!(after.get_attribute("aria-hidden").as_deref(), Some("true"));

        cleanup();

        assert_eq!(
            before.get_attribute("aria-hidden").as_deref(),
            Some("false")
        );
        assert_eq!(before.get_attribute("inert"), None);
        assert_eq!(after.get_attribute("aria-hidden"), None);
        assert_eq!(after.get_attribute("inert"), None);

        cleanup_node(before.as_ref());
        cleanup_node(after.as_ref());
        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn remove_inert_from_siblings_clears_attributes() {
        clear_managed_portals();

        let root = get_or_create_portal_root();

        let before = append_div(body().as_ref(), "before");

        before
            .set_attribute("inert", "")
            .expect("inert assignment must succeed");

        before
            .set_attribute("aria-hidden", "true")
            .expect("aria-hidden assignment must succeed");

        let after = append_div(body().as_ref(), "after");

        after
            .set_attribute("inert", "")
            .expect("inert assignment must succeed");

        after
            .set_attribute("aria-hidden", "true")
            .expect("aria-hidden assignment must succeed");

        remove_inert_from_siblings("ars-portal-root");

        assert_eq!(before.get_attribute("inert"), None);
        assert_eq!(before.get_attribute("aria-hidden"), None);
        assert_eq!(after.get_attribute("inert"), None);
        assert_eq!(after.get_attribute("aria-hidden"), None);

        cleanup_node(before.as_ref());
        cleanup_node(after.as_ref());
        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn forced_polyfill_restores_tabindex_values() {
        clear_managed_portals();

        let background = append_div(body().as_ref(), "background");

        let with_tabindex = append_button(background.as_ref(), "with-tabindex", Some("3"));

        let without_tabindex = append_button(background.as_ref(), "without-tabindex", None);

        let root = get_or_create_portal_root();

        let _first = append_button(&root, "first", None);
        let _last = append_button(&root, "last", None);

        let cleanup = set_background_inert_impl("ars-portal-root", false);

        assert_eq!(
            background.get_attribute("aria-hidden").as_deref(),
            Some("true")
        );
        assert_eq!(
            with_tabindex.get_attribute("tabindex").as_deref(),
            Some("-1")
        );
        assert_eq!(
            without_tabindex.get_attribute("tabindex").as_deref(),
            Some("-1")
        );

        cleanup();

        assert_eq!(background.get_attribute("aria-hidden"), None);
        assert_eq!(
            with_tabindex.get_attribute("tabindex").as_deref(),
            Some("3")
        );
        assert_eq!(without_tabindex.get_attribute("tabindex"), None);

        cleanup_node(background.as_ref());
        cleanup_node(&root);
    }

    #[wasm_bindgen_test]
    fn forced_polyfill_wraps_focus_for_tab_and_shift_tab() {
        clear_managed_portals();

        let background = append_div(body().as_ref(), "background");
        let background_button = append_button(background.as_ref(), "background-button", None);

        let root = get_or_create_portal_root();

        let first = append_button(&root, "first", None);
        let last = append_button(&root, "last", None);

        let cleanup = set_background_inert_impl("ars-portal-root", false);

        last.focus().expect("focus must succeed");

        let tab_init = web_sys::KeyboardEventInit::new();

        tab_init.set_key("Tab");

        let tab_event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &tab_init)
                .expect("keyboard event creation must succeed");
        document()
            .dispatch_event(&tab_event)
            .expect("dispatch must succeed");

        assert_eq!(
            document().active_element().expect("active element").id(),
            "first"
        );

        first.focus().expect("focus must succeed");

        let shift_tab_init = web_sys::KeyboardEventInit::new();

        shift_tab_init.set_key("Tab");
        shift_tab_init.set_shift_key(true);

        let shift_tab_event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &shift_tab_init)
                .expect("keyboard event creation must succeed");
        document()
            .dispatch_event(&shift_tab_event)
            .expect("dispatch must succeed");

        assert_eq!(
            document().active_element().expect("active element").id(),
            "last"
        );

        background_button
            .focus()
            .expect("focus outside portal must succeed");

        let outside_tab_init = web_sys::KeyboardEventInit::new();

        outside_tab_init.set_key("Tab");

        let outside_tab_event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &outside_tab_init)
                .expect("keyboard event creation must succeed");

        document()
            .dispatch_event(&outside_tab_event)
            .expect("dispatch must succeed");

        assert_eq!(
            document().active_element().expect("active element").id(),
            "first"
        );

        background_button
            .focus()
            .expect("focus outside portal must succeed");

        let outside_shift_tab_init = web_sys::KeyboardEventInit::new();

        outside_shift_tab_init.set_key("Tab");
        outside_shift_tab_init.set_shift_key(true);

        let outside_shift_tab_event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(
            "keydown",
            &outside_shift_tab_init,
        )
        .expect("keyboard event creation must succeed");

        document()
            .dispatch_event(&outside_shift_tab_event)
            .expect("dispatch must succeed");

        assert_eq!(
            document().active_element().expect("active element").id(),
            "last"
        );

        cleanup();

        cleanup_node(background.as_ref());
        cleanup_node(&root);
    }
}
