//! Outside-interaction containment helpers and listener installation.
//!
//! Implements the spec-mandated shared adapter helpers from
//! `spec/{leptos,dioxus}-components/utility/dismissable.md` §22:
//!
//! - **node-boundary registration helper** — [`target_is_inside_boundary`]
//!   walks DOM ancestors comparing each node's `id`,
//!   [`crate::portal::PORTAL_OWNER_ATTR`], and the global overlay stack
//!   against the consumer-supplied `inside_boundaries` and `exclude_ids`
//!   sets. Promoting this logic to a shared module keeps overlays
//!   (`Dialog`, `Popover`, `Menu`, `Combobox`, `Select`, `Tooltip`) and
//!   `focus-scope` from re-implementing the same DOM walk.
//!
//! - **platform capability helper** — [`install_outside_interaction_listeners`]
//!   normalises the document `pointerdown`/`focusin` and root-scoped
//!   `keydown` (Escape) listener triplet across `web` and non-`web` targets.
//!   Web builds attach real listeners that gate on
//!   [`crate::overlay_stack::is_topmost`] and call
//!   [`target_is_inside_boundary`] before invoking the supplied callbacks.
//!   Non-web builds (Dioxus Desktop SSR, server renders) return a no-op
//!   teardown so adapters can call the helper unconditionally and still
//!   match the documented "degrade gracefully" contract.
//!
//! Both helpers are deliberately decoupled from any framework or adapter
//! crate so the same calling convention works from `ars-leptos`,
//! `ars-dioxus`, and any future adapter.

use std::{
    fmt::{self, Debug},
    rc::Rc,
};

use ars_core::PointerType;
#[cfg(feature = "web")]
use web_sys::Element;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    crate::{
        overlay_stack::{is_above, is_topmost},
        portal::PORTAL_OWNER_ATTR,
    },
    std::cell::RefCell,
    wasm_bindgen::{JsCast, closure::Closure},
    web_sys::{Event, EventTarget, FocusEvent, KeyboardEvent, PointerEvent},
};

// ────────────────────────────────────────────────────────────────────
// Pure id-set predicate
// ────────────────────────────────────────────────────────────────────

/// Returns whether `id` should be treated as inside the dismissable surface
/// based on the consumer-registered `exclude_ids` and `inside_boundaries`
/// sets.
///
/// Empty ids never match — the DOM walk skips ancestors without an `id`
/// attribute even when the registries are non-empty.
#[must_use]
pub fn id_matches_inside_set(
    id: &str,
    exclude_ids: &[String],
    inside_boundaries: &[String],
) -> bool {
    if id.is_empty() {
        return false;
    }

    exclude_ids.iter().any(|other| other == id) || inside_boundaries.iter().any(|other| other == id)
}

// ────────────────────────────────────────────────────────────────────
// DOM containment walk
// ────────────────────────────────────────────────────────────────────

/// Returns whether `target` should be treated as inside the dismissable
/// surface owned by `overlay_id`.
///
/// The walk applies, in order:
///
/// 1. **Root containment** — `root.contains(target)` short-circuits the walk.
/// 2. **Ancestor id match** — `target` and each ancestor's `id` are checked
///    against [`id_matches_inside_set`].
/// 3. **Portal-owner match** — the `data-ars-portal-owner` attribute on each
///    ancestor is compared against `overlay_id` and `inside_boundaries`. If
///    the portal-owner is a stacked-above overlay
///    (`overlay_stack::is_above(owner, overlay_id)`), the click is treated as
///    inside the owning overlay and must not dismiss the parent (per
///    `spec/foundation/05-interactions.md` §12.8 rule 2).
///
/// `None` for `target` returns `false` — adapters resolve their own
/// `event.target()` and pass `Some(elem)` only when extraction succeeds.
/// Non-wasm web fallback returning `false` — DOM walks have no meaning
/// outside the browser.
#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
#[must_use]
pub fn target_is_inside_boundary(
    _target: Option<&Element>,
    _root: &Element,
    _overlay_id: &str,
    _inside_boundaries: &[String],
    _exclude_ids: &[String],
) -> bool {
    false
}

/// Walks DOM ancestors comparing `target` against the dismissable surface
/// owned by `overlay_id` (root containment, ancestor id matches, and
/// `data-ars-portal-owner` cross-overlay rules).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn target_is_inside_boundary(
    target: Option<&Element>,
    root: &Element,
    overlay_id: &str,
    inside_boundaries: &[String],
    exclude_ids: &[String],
) -> bool {
    let Some(target) = target else {
        return false;
    };

    if root.contains(Some(target)) {
        return true;
    }

    let mut current = Some(target.clone());

    while let Some(node) = current {
        let id = node.id();

        if id_matches_inside_set(&id, exclude_ids, inside_boundaries) {
            return true;
        }

        if let Some(owner) = node.get_attribute(PORTAL_OWNER_ATTR) {
            if owner == overlay_id {
                return true;
            }

            if inside_boundaries.iter().any(|boundary| boundary == &owner) {
                return true;
            }

            if is_above(&owner, overlay_id) {
                return true;
            }
        }

        current = node.parent_element();
    }

    false
}

// ────────────────────────────────────────────────────────────────────
// Listener installation
// ────────────────────────────────────────────────────────────────────

/// Adapter-supplied configuration for an outside-interaction listener
/// triplet.
///
/// Closures returning `Vec<String>` are evaluated on every event so reactive
/// changes to the consumer's boundary registries are observed without
/// re-installing listeners.
///
/// `Send + Sync` is intentionally **not** required on the readers and
/// callbacks — the helper only attaches listeners on wasm (single-
/// threaded), and the non-wasm web fallback never invokes them. Adapters
/// that need to share the config across threads should wrap their state
/// in `Arc<Mutex<...>>` themselves.
pub struct OutsideInteractionConfig {
    /// Stable overlay id used to gate listeners on
    /// [`crate::overlay_stack::is_topmost`] and to compare against
    /// `data-ars-portal-owner` attributes during the containment walk.
    pub overlay_id: String,

    /// Snapshot reader for the inside-boundary id list. Called once per
    /// event so adapters can wire reactive sources without rebuilding the
    /// listener triplet.
    pub inside_boundaries: Rc<dyn Fn() -> Vec<String>>,

    /// Snapshot reader for the exclude id list. Called once per event.
    pub exclude_ids: Rc<dyn Fn() -> Vec<String>>,

    /// Invoked after the boundary check passes for an outside pointer event.
    pub on_pointer_outside: Box<dyn Fn(f64, f64, PointerType)>,

    /// Invoked after the boundary check passes for an outside focus event.
    pub on_focus_outside: Box<dyn Fn()>,

    /// Invoked when Escape is pressed while this overlay is topmost. Should
    /// return `true` if the consumer wants the helper to call
    /// `Event::stop_propagation` so a parent overlay is not dismissed by the
    /// same keystroke (per `spec/foundation/05-interactions.md` §12.6).
    pub on_escape: Box<dyn Fn() -> bool>,
}

impl Debug for OutsideInteractionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutsideInteractionConfig")
            .field("overlay_id", &self.overlay_id)
            .field("inside_boundaries", &"<closure>")
            .field("exclude_ids", &"<closure>")
            .field("on_pointer_outside", &"<closure>")
            .field("on_focus_outside", &"<closure>")
            .field("on_escape", &"<closure>")
            .finish()
    }
}

/// Non-wasm web fallback returning a no-op teardown so adapters can call
/// the helper unconditionally on non-browser builds without a separate
/// cfg branch.
#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
#[must_use]
pub fn install_outside_interaction_listeners(
    _root: &Element,
    _config: OutsideInteractionConfig,
) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

/// Installs document `pointerdown`+`focusin` and root-scoped `keydown`
/// listeners that fire `config`'s callbacks for outside interactions and
/// Escape, gated on the overlay being topmost and the target being outside
/// every registered boundary.
///
/// Returns a teardown closure that removes every listener and is safe to
/// drop on cleanup.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn install_outside_interaction_listeners(
    root: &Element,
    config: OutsideInteractionConfig,
) -> Box<dyn FnOnce()> {
    let Some(window) = web_sys::window() else {
        return Box::new(|| {});
    };

    let Some(document) = window.document() else {
        return Box::new(|| {});
    };

    let shared = Rc::new(SharedConfig {
        overlay_id: config.overlay_id,
        inside_boundaries: config.inside_boundaries,
        exclude_ids: config.exclude_ids,
        on_pointer_outside: config.on_pointer_outside,
        on_focus_outside: config.on_focus_outside,
        on_escape: config.on_escape,
        root: root.clone(),
    });

    let pointer = build_pointer_listener(Rc::clone(&shared));
    let focus = build_focus_listener(Rc::clone(&shared));
    let keydown = build_keydown_listener(Rc::clone(&shared));

    let pointer_target: EventTarget = document.clone().into();
    let focus_target: EventTarget = document.into();
    let keydown_target: EventTarget = root.clone().into();

    if pointer_target
        .add_event_listener_with_callback_and_bool(
            "pointerdown",
            pointer.as_ref().unchecked_ref(),
            true,
        )
        .is_err()
    {
        return Box::new(|| {});
    }

    if focus_target
        .add_event_listener_with_callback("focusin", focus.as_ref().unchecked_ref())
        .is_err()
    {
        drop(pointer_target.remove_event_listener_with_callback_and_bool(
            "pointerdown",
            pointer.as_ref().unchecked_ref(),
            true,
        ));

        return Box::new(|| {});
    }

    if keydown_target
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .is_err()
    {
        drop(pointer_target.remove_event_listener_with_callback_and_bool(
            "pointerdown",
            pointer.as_ref().unchecked_ref(),
            true,
        ));

        drop(
            focus_target
                .remove_event_listener_with_callback("focusin", focus.as_ref().unchecked_ref()),
        );

        return Box::new(|| {});
    }

    let installed = InstalledListeners {
        pointer_target,
        focus_target,
        keydown_target,
        pointer: RefCell::new(Some(pointer)),
        focus: RefCell::new(Some(focus)),
        keydown: RefCell::new(Some(keydown)),
    };

    Box::new(move || installed.teardown())
}

// ────────────────────────────────────────────────────────────────────
// Internal listener plumbing (web only)
// ────────────────────────────────────────────────────────────────────

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct SharedConfig {
    overlay_id: String,
    inside_boundaries: Rc<dyn Fn() -> Vec<String>>,
    exclude_ids: Rc<dyn Fn() -> Vec<String>>,
    on_pointer_outside: Box<dyn Fn(f64, f64, PointerType)>,
    on_focus_outside: Box<dyn Fn()>,
    on_escape: Box<dyn Fn() -> bool>,
    root: Element,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
type PointerListenerCell = RefCell<Option<Closure<dyn FnMut(PointerEvent)>>>;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
type FocusListenerCell = RefCell<Option<Closure<dyn FnMut(FocusEvent)>>>;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
type KeydownListenerCell = RefCell<Option<Closure<dyn FnMut(KeyboardEvent)>>>;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct InstalledListeners {
    pointer_target: EventTarget,
    focus_target: EventTarget,
    keydown_target: EventTarget,
    pointer: PointerListenerCell,
    focus: FocusListenerCell,
    keydown: KeydownListenerCell,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl InstalledListeners {
    fn teardown(self) {
        if let Some(pointer) = self.pointer.into_inner() {
            drop(
                self.pointer_target
                    .remove_event_listener_with_callback_and_bool(
                        "pointerdown",
                        pointer.as_ref().unchecked_ref(),
                        true,
                    ),
            );
        }

        if let Some(focus) = self.focus.into_inner() {
            drop(
                self.focus_target
                    .remove_event_listener_with_callback("focusin", focus.as_ref().unchecked_ref()),
            );
        }

        if let Some(keydown) = self.keydown.into_inner() {
            drop(
                self.keydown_target.remove_event_listener_with_callback(
                    "keydown",
                    keydown.as_ref().unchecked_ref(),
                ),
            );
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn build_pointer_listener(shared: Rc<SharedConfig>) -> Closure<dyn FnMut(PointerEvent)> {
    Closure::wrap(Box::new(move |event: PointerEvent| {
        if !is_topmost(&shared.overlay_id) {
            return;
        }

        let target = resolve_pointer_target(&event);

        let inside_boundaries = (shared.inside_boundaries)();

        let exclude_ids = (shared.exclude_ids)();

        if target_is_inside_boundary(
            target.as_ref(),
            &shared.root,
            &shared.overlay_id,
            &inside_boundaries,
            &exclude_ids,
        ) {
            return;
        }

        let pointer_type = classify_pointer_type(&event.pointer_type());

        (shared.on_pointer_outside)(
            f64::from(event.client_x()),
            f64::from(event.client_y()),
            pointer_type,
        );
    }) as Box<dyn FnMut(PointerEvent)>)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn build_focus_listener(shared: Rc<SharedConfig>) -> Closure<dyn FnMut(FocusEvent)> {
    Closure::wrap(Box::new(move |event: FocusEvent| {
        if !is_topmost(&shared.overlay_id) {
            return;
        }

        let target = event.target().and_then(|t| t.dyn_into::<Element>().ok());

        let inside_boundaries = (shared.inside_boundaries)();

        let exclude_ids = (shared.exclude_ids)();

        if target_is_inside_boundary(
            target.as_ref(),
            &shared.root,
            &shared.overlay_id,
            &inside_boundaries,
            &exclude_ids,
        ) {
            return;
        }

        (shared.on_focus_outside)();
    }) as Box<dyn FnMut(FocusEvent)>)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn build_keydown_listener(shared: Rc<SharedConfig>) -> Closure<dyn FnMut(KeyboardEvent)> {
    Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if event.key() != "Escape" {
            return;
        }

        if !is_topmost(&shared.overlay_id) {
            return;
        }

        let stop = (shared.on_escape)();

        if stop {
            Event::stop_propagation(&event);
        }
    }) as Box<dyn FnMut(KeyboardEvent)>)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn resolve_pointer_target(event: &PointerEvent) -> Option<Element> {
    event.target().and_then(|t| t.dyn_into::<Element>().ok())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn classify_pointer_type(raw: &str) -> PointerType {
    match raw {
        "mouse" => PointerType::Mouse,
        "pen" => PointerType::Pen,
        "touch" => PointerType::Touch,
        _ => PointerType::Virtual,
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_matches_inside_set_returns_false_for_empty_id() {
        let exclude = vec!["trigger".into()];
        let inside = vec!["panel".into()];

        assert!(!id_matches_inside_set("", &exclude, &inside));
    }

    #[test]
    fn id_matches_inside_set_finds_excluded_id() {
        let exclude = vec!["trigger".into()];

        assert!(id_matches_inside_set("trigger", &exclude, &[]));
    }

    #[test]
    fn id_matches_inside_set_finds_inside_boundary_id() {
        let inside = vec!["panel".into()];

        assert!(id_matches_inside_set("panel", &[], &inside));
    }

    #[test]
    fn id_matches_inside_set_returns_false_for_unrelated_id() {
        let exclude = vec!["trigger".into()];
        let inside = vec!["panel".into()];

        assert!(!id_matches_inside_set("other", &exclude, &inside));
    }

    #[test]
    fn id_matches_inside_set_returns_false_for_empty_lists() {
        assert!(!id_matches_inside_set("anything", &[], &[]));
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };

    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::HtmlElement;

    use super::*;
    use crate::{
        overlay_stack::{OverlayEntry, push_overlay, remove_overlay, reset_overlay_stack},
        portal::PORTAL_OWNER_ATTR,
    };

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &Element, id: &str) -> HtmlElement {
        let element = document()
            .create_element("div")
            .expect("div creation must succeed")
            .dyn_into::<HtmlElement>()
            .expect("div must be HtmlElement");

        element.set_id(id);

        parent
            .append_child(&element)
            .expect("append_child must succeed");

        element
    }

    fn cleanup(nodes: &[&Element]) {
        for node in nodes {
            node.remove();
        }

        reset_overlay_stack();
    }

    // ── target_is_inside_boundary ────────────────────────────────

    #[wasm_bindgen_test]
    fn target_is_inside_for_node_inside_root() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-1");
        let child = append_div(&root, "ti-child-1");

        assert!(target_is_inside_boundary(
            Some(&child),
            &root,
            "ti-overlay-1",
            &[],
            &[],
        ));

        cleanup(&[&root]);
    }

    #[wasm_bindgen_test]
    fn target_is_outside_for_unrelated_node() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-2");
        let outside = append_div(&body(), "ti-outside-2");

        assert!(!target_is_inside_boundary(
            Some(&outside),
            &root,
            "ti-overlay-2",
            &[],
            &[],
        ));

        cleanup(&[&root, &outside]);
    }

    #[wasm_bindgen_test]
    fn target_is_inside_when_ancestor_id_in_inside_boundary() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-3");
        let parent = append_div(&body(), "ti-trigger-3");
        let child = append_div(&parent, "ti-child-3");

        assert!(target_is_inside_boundary(
            Some(&child),
            &root,
            "ti-overlay-3",
            &["ti-trigger-3".into()],
            &[],
        ));

        cleanup(&[&root, &parent]);
    }

    #[wasm_bindgen_test]
    fn target_is_inside_when_portal_owner_matches_overlay() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-4");
        let portal = append_div(&body(), "ti-portal-4");

        portal
            .set_attribute(PORTAL_OWNER_ATTR, "ti-overlay-4")
            .expect("portal owner must set");

        let child = append_div(&portal, "ti-child-4");

        assert!(target_is_inside_boundary(
            Some(&child),
            &root,
            "ti-overlay-4",
            &[],
            &[],
        ));

        cleanup(&[&root, &portal]);
    }

    #[wasm_bindgen_test]
    fn target_is_inside_when_portal_owner_is_stacked_child() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-5");
        let portal = append_div(&body(), "ti-portal-5");

        portal
            .set_attribute(PORTAL_OWNER_ATTR, "ti-child-overlay-5")
            .expect("portal owner must set");

        let child = append_div(&portal, "ti-child-5");

        push_overlay(OverlayEntry {
            id: "ti-overlay-5".into(),
            modal: false,
            z_index: None,
        });

        push_overlay(OverlayEntry {
            id: "ti-child-overlay-5".into(),
            modal: false,
            z_index: None,
        });

        assert!(target_is_inside_boundary(
            Some(&child),
            &root,
            "ti-overlay-5",
            &[],
            &[],
        ));

        remove_overlay("ti-overlay-5");
        remove_overlay("ti-child-overlay-5");

        cleanup(&[&root, &portal]);
    }

    #[wasm_bindgen_test]
    fn target_outside_when_portal_owner_is_unrelated_overlay() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-6");
        let portal = append_div(&body(), "ti-portal-6");

        portal
            .set_attribute(PORTAL_OWNER_ATTR, "ti-other-overlay-6")
            .expect("portal owner must set");

        let child = append_div(&portal, "ti-child-6");

        push_overlay(OverlayEntry {
            id: "ti-overlay-6".into(),
            modal: false,
            z_index: None,
        });

        push_overlay(OverlayEntry {
            id: "ti-other-overlay-6".into(),
            modal: false,
            z_index: None,
        });

        // Mark the unrelated overlay as a sibling — pop+push to make it not above us.
        remove_overlay("ti-overlay-6");

        push_overlay(OverlayEntry {
            id: "ti-overlay-6".into(),
            modal: false,
            z_index: None,
        });

        assert!(!target_is_inside_boundary(
            Some(&child),
            &root,
            "ti-overlay-6",
            &[],
            &[],
        ));

        remove_overlay("ti-overlay-6");
        remove_overlay("ti-other-overlay-6");

        cleanup(&[&root, &portal]);
    }

    #[wasm_bindgen_test]
    fn target_is_outside_for_none() {
        reset_overlay_stack();

        let root = append_div(&body(), "ti-root-7");

        assert!(!target_is_inside_boundary(
            None,
            &root,
            "ti-overlay-7",
            &[],
            &[],
        ));

        cleanup(&[&root]);
    }

    // ── install_outside_interaction_listeners ──────────────────────

    fn arc_static(values: Vec<String>) -> Rc<dyn Fn() -> Vec<String>> {
        Rc::new(move || values.clone())
    }

    fn dispatch_pointerdown_at(target: &EventTarget) {
        let init = web_sys::PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);

        let event = PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("PointerEvent must construct");

        target
            .dispatch_event(&event)
            .expect("dispatch_event must succeed");
    }

    fn dispatch_keydown_on(target: &EventTarget, key: &str) {
        let init = web_sys::KeyboardEventInit::new();

        init.set_key(key);
        init.set_bubbles(true);
        init.set_cancelable(true);

        let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("KeyboardEvent must construct");

        target
            .dispatch_event(&event)
            .expect("dispatch_event must succeed");
    }

    #[wasm_bindgen_test]
    fn install_returns_cleanup_that_drops_listeners() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-1");

        push_overlay(OverlayEntry {
            id: "li-overlay-1".into(),
            modal: false,
            z_index: None,
        });

        let pointer_calls = Rc::new(Cell::new(0usize));
        let escape_calls = Rc::new(Cell::new(0usize));

        let pointer_calls_for_cb = Rc::clone(&pointer_calls);
        let escape_calls_for_cb = Rc::clone(&escape_calls);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-1".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(move |_, _, _| {
                    pointer_calls_for_cb.set(pointer_calls_for_cb.get() + 1);
                }),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(move || {
                    escape_calls_for_cb.set(escape_calls_for_cb.get() + 1);
                    true
                }),
            },
        );

        // Tear down before any dispatch — listeners must not fire.
        teardown();

        let outside = append_div(&body(), "li-outside-1");

        dispatch_pointerdown_at(outside.as_ref());
        dispatch_keydown_on(root.as_ref(), "Escape");

        assert_eq!(pointer_calls.get(), 0);
        assert_eq!(escape_calls.get(), 0);

        remove_overlay("li-overlay-1");

        cleanup(&[&root, &outside]);
    }

    #[wasm_bindgen_test]
    fn pointer_outside_fires_when_topmost_and_outside() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-2");
        let outside = append_div(&body(), "li-outside-2");

        push_overlay(OverlayEntry {
            id: "li-overlay-2".into(),
            modal: false,
            z_index: None,
        });

        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-2".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(move |_, _, _| {
                    fired_for_cb.store(true, Ordering::SeqCst);
                }),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(|| true),
            },
        );

        dispatch_pointerdown_at(outside.as_ref());

        assert!(fired.load(Ordering::SeqCst));

        teardown();

        remove_overlay("li-overlay-2");

        cleanup(&[&root, &outside]);
    }

    #[wasm_bindgen_test]
    fn pointer_inside_root_does_not_fire() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-3");
        let inside = append_div(&root, "li-inside-3");

        push_overlay(OverlayEntry {
            id: "li-overlay-3".into(),
            modal: false,
            z_index: None,
        });

        let fired = Arc::new(AtomicUsize::new(0));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-3".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(move |_, _, _| {
                    fired_for_cb.fetch_add(1, Ordering::SeqCst);
                }),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(|| true),
            },
        );

        dispatch_pointerdown_at(inside.as_ref());

        assert_eq!(fired.load(Ordering::SeqCst), 0);

        teardown();

        remove_overlay("li-overlay-3");

        cleanup(&[&root]);
    }

    #[wasm_bindgen_test]
    fn pointer_outside_skipped_when_not_topmost() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-4");
        let outside = append_div(&body(), "li-outside-4");

        push_overlay(OverlayEntry {
            id: "li-overlay-4".into(),
            modal: false,
            z_index: None,
        });

        push_overlay(OverlayEntry {
            id: "li-other-overlay-4".into(),
            modal: false,
            z_index: None,
        });

        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-4".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(move |_, _, _| {
                    fired_for_cb.store(true, Ordering::SeqCst);
                }),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(|| true),
            },
        );

        dispatch_pointerdown_at(outside.as_ref());

        assert!(!fired.load(Ordering::SeqCst));

        teardown();

        remove_overlay("li-overlay-4");
        remove_overlay("li-other-overlay-4");

        cleanup(&[&root, &outside]);
    }

    #[wasm_bindgen_test]
    fn escape_on_root_fires_when_topmost() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-5");

        push_overlay(OverlayEntry {
            id: "li-overlay-5".into(),
            modal: false,
            z_index: None,
        });

        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-5".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(|_, _, _| {}),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(move || {
                    fired_for_cb.store(true, Ordering::SeqCst);
                    true
                }),
            },
        );

        dispatch_keydown_on(root.as_ref(), "Escape");

        assert!(fired.load(Ordering::SeqCst));

        teardown();

        remove_overlay("li-overlay-5");

        cleanup(&[&root]);
    }

    #[wasm_bindgen_test]
    fn non_escape_keydown_does_not_fire_callback() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-6");

        push_overlay(OverlayEntry {
            id: "li-overlay-6".into(),
            modal: false,
            z_index: None,
        });

        let fired = Arc::new(AtomicBool::new(false));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-6".into(),
                inside_boundaries: arc_static(Vec::new()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(|_, _, _| {}),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(move || {
                    fired_for_cb.store(true, Ordering::SeqCst);
                    true
                }),
            },
        );

        dispatch_keydown_on(root.as_ref(), "Enter");

        assert!(!fired.load(Ordering::SeqCst));

        teardown();

        remove_overlay("li-overlay-6");

        cleanup(&[&root]);
    }

    #[wasm_bindgen_test]
    fn boundaries_are_read_at_event_time() {
        reset_overlay_stack();

        let root = append_div(&body(), "li-root-7");
        let outside = append_div(&body(), "li-outside-7");

        push_overlay(OverlayEntry {
            id: "li-overlay-7".into(),
            modal: false,
            z_index: None,
        });

        let boundaries = Rc::new(RefCell::new(Vec::<String>::new()));
        let boundaries_for_reader = Rc::clone(&boundaries);

        let fired = Arc::new(AtomicUsize::new(0));
        let fired_for_cb = Arc::clone(&fired);

        let teardown = install_outside_interaction_listeners(
            &root,
            OutsideInteractionConfig {
                overlay_id: "li-overlay-7".into(),
                inside_boundaries: Rc::new(move || boundaries_for_reader.borrow().clone()),
                exclude_ids: arc_static(Vec::new()),
                on_pointer_outside: Box::new(move |_, _, _| {
                    fired_for_cb.fetch_add(1, Ordering::SeqCst);
                }),
                on_focus_outside: Box::new(|| {}),
                on_escape: Box::new(|| true),
            },
        );

        // First dispatch — outside is not in boundaries → callback fires.
        dispatch_pointerdown_at(outside.as_ref());

        assert_eq!(fired.load(Ordering::SeqCst), 1);

        // Mutate the boundary list — second dispatch must skip the callback.
        boundaries.borrow_mut().push("li-outside-7".into());

        dispatch_pointerdown_at(outside.as_ref());

        assert_eq!(fired.load(Ordering::SeqCst), 1);

        teardown();

        remove_overlay("li-overlay-7");

        cleanup(&[&root, &outside]);
    }
}
