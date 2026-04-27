//! Leptos adapter for the framework-agnostic `Dismissable` behavior.
//!
//! Exposes [`use_dismissable`] and [`Region`] (a paired-button wrapper),
//! composing [`Props`] with the shared
//! [`ars_dom::outside_interaction`] helpers so overlay components
//! (`Dialog`, `Popover`, `Tooltip`, `Select`, `Combobox`, `Menu`) get
//! document-level outside-interaction listeners, portal-aware boundary
//! checks, paired dismiss buttons, and topmost-overlay gating without
//! re-implementing each piece per overlay.
//!
//! This module re-exports the agnostic `ars_components::utility::dismissable`
//! surface (`Props`, `Messages`, `Part`, `DismissReason`,
//! `DismissAttempt`, `dismiss_button_attrs`) so consumers reach every
//! dismissable type through a single namespace —
//! `dismissable::Props`, `dismissable::Region`, `dismissable::Handle`,
//! `dismissable::use_dismissable`, etc.
//!
//! Listeners are client-only (do not attach during SSR), portal-aware
//! (consult `data-ars-portal-owner`), reactive (snapshot
//! `inside_boundaries` per event so wrapper updates take effect without
//! re-mounting), and topmost-aware (consult `ars_dom::overlay_stack`).
//!
//! See `spec/leptos-components/utility/dismissable.md` for the full
//! adapter contract.

use std::fmt::{self, Debug};

pub use ars_components::utility::dismissable::{
    DismissAttempt, DismissReason, Messages, Part, Props, dismiss_button_attrs,
};
use leptos::{callback::Callback as LeptosCallback, html, prelude::*};
#[cfg(not(feature = "ssr"))]
use {
    ars_dom::{
        OutsideInteractionConfig,
        overlay_stack::{OverlayEntry, push_overlay, remove_overlay},
    },
    ars_interactions::InteractOutsideEvent,
    std::{cell::RefCell, rc::Rc},
};

use crate::{
    attrs::attr_map_to_leptos_inline_attrs,
    id::use_id,
    provider::{resolve_locale, use_messages},
};

// ────────────────────────────────────────────────────────────────────
// Handle
// ────────────────────────────────────────────────────────────────────

/// Handle returned by [`use_dismissable`].
///
/// Consumers compose around the dismissable surface by reading
/// [`overlay_id`](Self::overlay_id) (the stack registration key) and
/// invoking [`dismiss`](Self::dismiss) for the programmatic dismiss-button
/// activation path defined in `spec/components/utility/dismissable.md`
/// §11 — calling `props.on_dismiss(DismissReason::DismissButton)`
/// directly without firing veto-capable callbacks first.
///
/// Both fields are arena-backed Leptos primitives, so [`Handle`] is `Copy`
/// — consumers can move it into multiple closures or pass it through the
/// view tree without an explicit clone. The handle stays valid until the
/// owning [`Owner`](leptos::reactive::owner::Owner) is dropped.
#[derive(Clone, Copy)]
pub struct Handle {
    /// Programmatic dismiss-button activation. Invoking
    /// `Callback::run(())` fires `props.on_dismiss(DismissReason::DismissButton)`
    /// if a callback is registered. Backed by Leptos's arena-allocated
    /// callback storage so the handle stays `Copy`.
    pub dismiss: LeptosCallback<()>,

    /// Stable id used for overlay-stack registration and portal-owner
    /// matching. Stored in the Leptos arena so [`Handle`] remains `Copy`;
    /// read the underlying [`String`] with
    /// `overlay_id.read_value()` (clone) or `overlay_id.with_value(|id| …)`
    /// (borrow).
    pub overlay_id: StoredValue<String>,
}

impl Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Handle");

        debug.field("dismiss", &"<Callback<()>>");

        self.overlay_id
            .try_with_value(|id| {
                debug.field("overlay_id", id);
            })
            .unwrap_or_else(|| {
                debug.field("overlay_id", &"<disposed>");
            });

        debug.finish()
    }
}

// ────────────────────────────────────────────────────────────────────
// Public hook
// ────────────────────────────────────────────────────────────────────

/// Adapter-owned dismissable hook for Leptos.
///
/// Allocates an overlay id, registers it on the global overlay stack
/// while mounted, and installs the document `pointerdown`/`focusin` and
/// root-scoped `keydown` listener triplet via
/// [`ars_dom::install_outside_interaction_listeners`]. Listeners are
/// client-only — under SSR the hook is a no-op aside from id allocation
/// and handle construction.
#[must_use]
pub fn use_dismissable(
    root_ref: NodeRef<html::Div>,
    props: Props,
    inside_boundaries: Signal<Vec<String>>,
) -> Handle {
    let overlay_id = StoredValue::new(use_id("ars-dismissable"));

    let dismiss = build_dismiss_button_callback(&props);

    #[cfg(not(feature = "ssr"))]
    install_dismissable_listeners(root_ref, props, inside_boundaries, &overlay_id.read_value());

    // Reference the inputs under SSR so the public signature still
    // exercises every parameter and rust-analyzer / clippy are happy.
    #[cfg(feature = "ssr")]
    {
        drop((root_ref, props, inside_boundaries));
    }

    Handle {
        dismiss,
        overlay_id,
    }
}

/// Returns the callback wired onto each visually-hidden dismiss button
/// and into [`Handle::dismiss`]. Invoking it fires
/// `props.on_dismiss(DismissReason::DismissButton)` directly per spec
/// §11 — no veto-capable callbacks run first.
fn build_dismiss_button_callback(props: &Props) -> LeptosCallback<()> {
    let on_dismiss = props.on_dismiss.clone();

    LeptosCallback::new(move |()| {
        if let Some(cb) = on_dismiss.as_ref() {
            cb(DismissReason::DismissButton);
        }
    })
}

// ────────────────────────────────────────────────────────────────────
// Client-only listener wiring
// ────────────────────────────────────────────────────────────────────

#[cfg(not(feature = "ssr"))]
fn install_dismissable_listeners(
    root_ref: NodeRef<html::Div>,
    props: Props,
    inside_boundaries: Signal<Vec<String>>,
    overlay_id: &str,
) {
    let state = Rc::new(DismissableState {
        overlay_id: overlay_id.into(),
        props,
        teardown: RefCell::new(None),
        overlay_pushed: RefCell::new(false),
        cleaned_up: RefCell::new(false),
    });

    let stored_state = StoredValue::new_local(state);
    let stored_boundaries = StoredValue::new_local(inside_boundaries);

    Effect::new(move |_| {
        stored_state.with_value(|state| {
            stored_boundaries.with_value(|boundaries| {
                attach(state, root_ref, *boundaries);
            });
        });
    });

    on_cleanup(move || {
        stored_state.with_value(|state| {
            teardown(state);
        });
    });
}

#[cfg(not(feature = "ssr"))]
struct DismissableState {
    overlay_id: String,
    props: Props,
    teardown: RefCell<Option<Box<dyn FnOnce()>>>,
    overlay_pushed: RefCell<bool>,
    cleaned_up: RefCell<bool>,
}

#[cfg(not(feature = "ssr"))]
fn attach(
    state: &Rc<DismissableState>,
    root_ref: NodeRef<html::Div>,
    inside_boundaries: Signal<Vec<String>>,
) {
    if *state.cleaned_up.borrow() || state.teardown.borrow().is_some() {
        return;
    }

    let Some(root_element) = root_ref.get_untracked() else {
        return;
    };

    let root_element: leptos::web_sys::HtmlElement = (*root_element).clone();
    let root_element: leptos::web_sys::Element = root_element.into();

    push_overlay(OverlayEntry {
        id: state.overlay_id.clone(),
        modal: false,
        z_index: None,
    });

    *state.overlay_pushed.borrow_mut() = true;

    let Props {
        on_interact_outside,
        on_escape_key_down,
        on_dismiss,
        exclude_ids,
        ..
    } = state.props.clone();

    let on_dismiss_for_pointer = on_dismiss.clone();
    let on_dismiss_for_focus = on_dismiss.clone();
    let on_dismiss_for_escape = on_dismiss.clone();

    let on_interact_outside_for_pointer = on_interact_outside.clone();
    let on_interact_outside_for_focus = on_interact_outside;

    let teardown_fn = ars_dom::install_outside_interaction_listeners(
        &root_element,
        OutsideInteractionConfig {
            overlay_id: state.overlay_id.clone(),
            inside_boundaries: Rc::new(move || inside_boundaries.get_untracked()),
            exclude_ids: Rc::new(move || exclude_ids.clone()),
            on_pointer_outside: Box::new(move |client_x, client_y, pointer_type| {
                let attempt = DismissAttempt::new(InteractOutsideEvent::PointerOutside {
                    client_x,
                    client_y,
                    pointer_type,
                });

                if let Some(cb) = on_interact_outside_for_pointer.as_ref() {
                    cb(attempt.clone());
                }

                if attempt.is_prevented() {
                    return;
                }

                if let Some(cb) = on_dismiss_for_pointer.as_ref() {
                    cb(DismissReason::OutsidePointer);
                }
            }),
            on_focus_outside: Box::new(move || {
                let attempt = DismissAttempt::new(InteractOutsideEvent::FocusOutside);

                if let Some(cb) = on_interact_outside_for_focus.as_ref() {
                    cb(attempt.clone());
                }

                if attempt.is_prevented() {
                    return;
                }

                if let Some(cb) = on_dismiss_for_focus.as_ref() {
                    cb(DismissReason::OutsideFocus);
                }
            }),
            on_escape: Box::new(move || {
                let attempt = DismissAttempt::new(());

                if let Some(cb) = on_escape_key_down.as_ref() {
                    cb(attempt.clone());
                }

                if attempt.is_prevented() {
                    return false;
                }

                if let Some(cb) = on_dismiss_for_escape.as_ref() {
                    cb(DismissReason::Escape);
                }

                true
            }),
        },
    );

    *state.teardown.borrow_mut() = Some(teardown_fn);
}

#[cfg(not(feature = "ssr"))]
fn teardown(state: &Rc<DismissableState>) {
    if *state.cleaned_up.borrow() {
        return;
    }

    *state.cleaned_up.borrow_mut() = true;

    if let Some(teardown_fn) = state.teardown.borrow_mut().take() {
        teardown_fn();
    }

    if *state.overlay_pushed.borrow() {
        remove_overlay(&state.overlay_id);

        *state.overlay_pushed.borrow_mut() = false;
    }
}

// ────────────────────────────────────────────────────────────────────
// Region component
// ────────────────────────────────────────────────────────────────────

/// Renders an adapter-owned dismissable region with paired native
/// `<button>` dismiss controls flanking the consumer children.
///
/// Both buttons fire [`Handle::dismiss`] identically — the duplication is
/// required, not redundant. Tab-cycle exits in either direction,
/// screen-reader reading-order proximity (start announces "Dismiss" up
/// front, end is the next stop after the body), and rotor element-list
/// discovery all rely on having a dismiss target at both boundaries of
/// the region. See [`Part::DismissButton`] and
/// `spec/components/utility/dismissable.md` §3 for the full rationale.
///
/// Wraps [`use_dismissable`] for the common case where a wrapper just
/// wants the documented "DismissButton (start) — content —
/// DismissButton (end)" anatomy with the right attrs and listener
/// wiring.
#[component]
pub fn Region(
    /// Behavioural props forwarded to [`use_dismissable`].
    props: Props,

    /// Optional reactive list of additional inside-boundary ids.
    /// Defaults to an empty signal so the trigger element alone is
    /// matched against `props.exclude_ids`.
    #[prop(optional, into)]
    inside_boundaries: Option<Signal<Vec<String>>>,

    /// Optional locale override — falls through to the surrounding
    /// `ArsProvider` locale when [`None`].
    #[prop(optional)]
    locale: Option<ars_i18n::Locale>,

    /// Optional message bundle override — falls through to the adapter's
    /// [`use_messages`] resolution chain (props → `I18nRegistries` →
    /// `Messages::default`) when [`None`].
    #[prop(optional)]
    messages: Option<Messages>,

    /// Children rendered between the start and end dismiss buttons.
    children: Children,
) -> impl IntoView {
    // Re-bind the prop-supplied overrides so the parameter values are
    // owned locals — Leptos requires component props to be received by
    // value, but `clippy::needless_pass_by_value` would otherwise
    // complain that the parameters are only read via `.as_ref()`.
    let messages_override = messages;
    let locale_override = locale;

    let root_ref = NodeRef::<html::Div>::new();

    let boundaries = inside_boundaries.unwrap_or_else(|| Signal::stored(Vec::new()));

    let resolved_messages =
        use_messages::<Messages>(messages_override.as_ref(), locale_override.as_ref());

    let dismiss_label =
        (resolved_messages.dismiss_label)(&resolve_locale(locale_override.as_ref()));

    let dismiss_attrs = dismiss_button_attrs(&dismiss_label);

    let inline_attrs = attr_map_to_leptos_inline_attrs(dismiss_attrs);
    let start_attrs = inline_attrs.clone();
    let end_attrs = inline_attrs;

    let handle = use_dismissable(root_ref, props, boundaries);

    view! {
        <div node_ref=root_ref>
            <button
                {..start_attrs}
                on:click=move |_| { handle.dismiss.run(()); }
            />
            {children()}
            <button
                {..end_attrs}
                on:click=move |_| { handle.dismiss.run(()); }
            />
        </div>
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use leptos::reactive::owner::Owner;

    use super::*;

    /// Sets up a fresh Leptos reactive [`Owner`] for the duration of a
    /// test. Both [`LeptosCallback`] and [`StoredValue`] live in the
    /// active owner's arena, so any test constructing a [`Handle`] needs
    /// one of these guards in scope.
    #[must_use = "the returned Owner guard must outlive the test body"]
    fn test_owner() -> Owner {
        let owner = Owner::new();

        owner.set();

        owner
    }

    #[test]
    fn build_dismiss_button_callback_invokes_on_dismiss_with_dismiss_button_reason() {
        let _owner = test_owner();

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_props = Arc::clone(&calls);

        let props = Props::new().on_dismiss(move |reason| {
            assert_eq!(reason, DismissReason::DismissButton);
            calls_for_props.fetch_add(1, Ordering::SeqCst);
        });

        let cb = build_dismiss_button_callback(&props);

        cb.run(());
        cb.run(());

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn build_dismiss_button_callback_is_no_op_when_callback_missing() {
        let _owner = test_owner();

        let cb = build_dismiss_button_callback(&Props::new());

        cb.run(());
    }

    #[test]
    fn dismissable_handle_copy_shares_overlay_id_and_dismiss_target() {
        let _owner = test_owner();

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_props = Arc::clone(&calls);

        let props = Props::new().on_dismiss(move |_reason| {
            calls_for_props.fetch_add(1, Ordering::SeqCst);
        });

        let handle = Handle {
            dismiss: build_dismiss_button_callback(&props),
            overlay_id: StoredValue::new("overlay-1".into()),
        };

        let copied = handle;

        let original_id = handle.overlay_id.with_value(String::clone);
        let copied_id = copied.overlay_id.with_value(String::clone);

        assert_eq!(
            original_id, copied_id,
            "Copy must share the same arena slot, not duplicate it",
        );

        handle.dismiss.run(());
        copied.dismiss.run(());

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn dismissable_handle_debug_includes_overlay_id() {
        let _owner = test_owner();

        let handle = Handle {
            dismiss: build_dismiss_button_callback(&Props::new()),
            overlay_id: StoredValue::new("ars-dismissable-42".into()),
        };

        let debug = format!("{handle:?}");

        assert!(debug.contains("ars-dismissable-42"));
    }
}

// ────────────────────────────────────────────────────────────────────
// Wasm tests
// ────────────────────────────────────────────────────────────────────

#[cfg(all(test, target_arch = "wasm32", not(feature = "ssr")))]
mod wasm_tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use ars_core::MessageFn;
    use ars_i18n::{Locale, locales};
    use ars_interactions::InteractOutsideEvent;
    use leptos::{mount::mount_to, prelude::*, reactive::owner::Owner};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, Element, EventInit, HtmlElement, KeyboardEventInit, PointerEventInit};

    // Explicit imports to shadow `leptos::prelude::Props` (the
    // component-props trait) with the dismissable struct.
    use super::{DismissReason, Messages, Props, Region};

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        leptos::web_sys::window()
            .and_then(|w| w.document())
            .expect("browser document should exist")
    }

    /// Creates a fresh isolated container and yields it to the test body.
    /// The container is removed from the body when the returned cleanup
    /// closure runs.
    fn with_container() -> Element {
        let container = document()
            .create_element("div")
            .expect("create_element should succeed");

        document()
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        container
    }

    /// Tick the Leptos reactive cycle once.
    async fn tick() {
        leptos::task::tick().await;
    }

    #[wasm_bindgen_test]
    async fn region_renders_paired_dismiss_buttons_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let _handle = mount_to(parent, move || {
            view! {
                <Region props=Props::new()>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let buttons = container
            .query_selector_all("button[data-ars-part='dismiss-button']")
            .expect("query_selector_all should succeed");

        assert_eq!(
            buttons.length(),
            2,
            "Region must render exactly two visually-hidden dismiss buttons (start + end)",
        );

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn dismiss_button_click_fires_on_dismiss_with_dismiss_button_reason_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new().on_dismiss(move |reason| {
                calls_for_callback
                    .lock()
                    .expect("calls mutex must not be poisoned")
                    .push(reason);
            });

            view! {
                <Region props=props>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let button = container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("at least one dismiss button must exist");

        let html_button: HtmlElement = button.unchecked_into();

        html_button.click();

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert_eq!(
            log.as_slice(),
            &[DismissReason::DismissButton],
            "clicking the visually-hidden dismiss button must fire on_dismiss with DismissButton",
        );

        drop(log);

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn escape_keydown_fires_on_dismiss_with_escape_reason_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new().on_dismiss(move |reason| {
                calls_for_callback
                    .lock()
                    .expect("calls mutex must not be poisoned")
                    .push(reason);
            });

            view! {
                <Region props=props>
                    <span id="region-content">"content"</span>
                </Region>
            }
        });

        tick().await;

        // Find the rendered root <div> — it's the parent of the dismiss button
        // we just located. Dispatch keydown(Escape) on it directly so the
        // root-scoped keydown listener picks it up.
        let dismiss_button = container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("dismiss button must exist");

        let root: Element = dismiss_button
            .parent_element()
            .expect("dismiss button must have a parent");

        let init = KeyboardEventInit::new();

        init.set_key("Escape");
        init.set_bubbles(true);
        init.set_cancelable(true);

        let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("keydown event should construct");

        root.dispatch_event(&event)
            .expect("dispatch_event should succeed");

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert_eq!(
            log.as_slice(),
            &[DismissReason::Escape],
            "Escape on the root must fire on_dismiss with Escape",
        );

        drop(log);

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn outside_pointerdown_fires_on_dismiss_with_outside_pointer_reason_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new().on_dismiss(move |reason| {
                calls_for_callback
                    .lock()
                    .expect("calls mutex must not be poisoned")
                    .push(reason);
            });

            view! {
                <Region props=props>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        // Create an outside element that is NOT a descendant of the
        // dismissable region root, then dispatch a pointerdown on it.
        let outside = document()
            .create_element("div")
            .expect("create_element should succeed");

        outside.set_id("outside");

        document()
            .body()
            .expect("body should exist")
            .append_child(&outside)
            .expect("append_child should succeed");

        let init = PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");

        let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct");

        outside
            .dispatch_event(&event)
            .expect("dispatch_event should succeed");

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert_eq!(
            log.as_slice(),
            &[DismissReason::OutsidePointer],
            "pointerdown outside the region must fire on_dismiss with OutsidePointer",
        );

        drop(log);

        outside.remove();
        container.remove();
    }

    #[wasm_bindgen_test]
    async fn outside_focusin_fires_on_dismiss_with_outside_focus_reason_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new().on_dismiss(move |reason| {
                calls_for_callback
                    .lock()
                    .expect("calls mutex must not be poisoned")
                    .push(reason);
            });

            view! {
                <Region props=props>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let outside = document()
            .create_element("div")
            .expect("create_element should succeed");

        outside.set_id("focus-outside");

        document()
            .body()
            .expect("body should exist")
            .append_child(&outside)
            .expect("append_child should succeed");

        let init = EventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);

        let event = web_sys::Event::new_with_event_init_dict("focusin", &init)
            .expect("focusin event should construct");

        outside
            .dispatch_event(&event)
            .expect("dispatch_event should succeed");

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert_eq!(
            log.as_slice(),
            &[DismissReason::OutsideFocus],
            "focusin outside the region must fire on_dismiss with OutsideFocus",
        );

        drop(log);

        outside.remove();
        container.remove();
    }

    #[wasm_bindgen_test]
    async fn prevent_dismiss_in_on_interact_outside_skips_on_dismiss_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let interact_calls = Arc::new(AtomicUsize::new(0));
        let dismiss_log = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let interact_for_props = Arc::clone(&interact_calls);
        let dismiss_for_props = Arc::clone(&dismiss_log);

        let _handle = mount_to(parent, move || {
            let interact_for_callback = Arc::clone(&interact_for_props);
            let dismiss_for_callback = Arc::clone(&dismiss_for_props);
            let props = Props::new()
                .on_interact_outside(move |attempt| {
                    interact_for_callback.fetch_add(1, Ordering::SeqCst);
                    attempt.prevent_dismiss();
                })
                .on_dismiss(move |reason| {
                    dismiss_for_callback
                        .lock()
                        .expect("dismiss_log mutex must not be poisoned")
                        .push(reason);
                });

            view! {
                <Region props=props>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let outside = document()
            .create_element("div")
            .expect("create_element should succeed");

        outside.set_id("veto-outside");

        document()
            .body()
            .expect("body should exist")
            .append_child(&outside)
            .expect("append_child should succeed");

        let init = PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");

        let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct");

        outside
            .dispatch_event(&event)
            .expect("dispatch_event should succeed");

        tick().await;

        assert_eq!(
            interact_calls.load(Ordering::SeqCst),
            1,
            "on_interact_outside must fire once for the outside pointerdown",
        );

        let log = dismiss_log
            .lock()
            .expect("dismiss_log mutex must not be poisoned");

        assert!(
            log.is_empty(),
            "calling DismissAttempt::prevent_dismiss must veto the on_dismiss call",
        );

        drop(log);

        outside.remove();
        container.remove();
    }

    #[wasm_bindgen_test]
    async fn outside_pointerdown_inside_excluded_id_does_not_fire_on_dismiss_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new()
                .exclude_ids(["trigger"])
                .on_dismiss(move |reason| {
                    calls_for_callback
                        .lock()
                        .expect("calls mutex must not be poisoned")
                        .push(reason);
                });

            view! {
                <Region props=props>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let trigger = document()
            .create_element("button")
            .expect("create_element should succeed");

        trigger.set_id("trigger");

        document()
            .body()
            .expect("body should exist")
            .append_child(&trigger)
            .expect("append_child should succeed");

        let init = PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");

        let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct");

        trigger
            .dispatch_event(&event)
            .expect("dispatch_event should succeed");

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert!(
            log.is_empty(),
            "pointerdown on an excluded-id element must not fire on_dismiss",
        );

        drop(log);

        trigger.remove();
        container.remove();
    }

    #[wasm_bindgen_test]
    async fn region_locale_override_resolves_label_through_use_messages_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let _handle = mount_to(parent, move || {
            let messages = Messages {
                dismiss_label: MessageFn::new(|locale: &Locale| {
                    if locale.language() == "de" {
                        String::from("Schließen")
                    } else {
                        String::from("Dismiss")
                    }
                }),
            };

            view! {
                <Region
                    props=Props::new()
                    locale=locales::de_de()
                    messages=messages
                >
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let button = container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("at least one dismiss button must exist");

        let label = button
            .get_attribute("aria-label")
            .expect("dismiss button must carry an aria-label");

        assert_eq!(
            label, "Schließen",
            "Region locale override must flow through use_messages so dismiss_label sees the German locale",
        );

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn inside_boundaries_signal_change_takes_effect_without_remount_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let calls = Arc::new(Mutex::new(Vec::<DismissReason>::new()));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let calls_for_props = Arc::clone(&calls);

        let (boundaries, set_boundaries) = signal(Vec::<String>::new());

        let _handle = mount_to(parent, move || {
            let calls_for_callback = Arc::clone(&calls_for_props);
            let props = Props::new().on_dismiss(move |reason| {
                calls_for_callback
                    .lock()
                    .expect("calls mutex must not be poisoned")
                    .push(reason);
            });

            view! {
                <Region props=props inside_boundaries=Signal::from(boundaries)>
                    <span>"content"</span>
                </Region>
            }
        });

        tick().await;

        let outside = document()
            .create_element("div")
            .expect("create_element should succeed");

        outside.set_id("late-boundary");

        document()
            .body()
            .expect("body should exist")
            .append_child(&outside)
            .expect("append_child should succeed");

        let init = PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");

        let first_event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct");

        outside
            .dispatch_event(&first_event)
            .expect("dispatch_event should succeed");

        tick().await;

        assert_eq!(
            calls
                .lock()
                .expect("calls mutex must not be poisoned")
                .as_slice(),
            &[DismissReason::OutsidePointer],
            "first pointerdown must fire on_dismiss while boundaries set is empty",
        );

        // Add the outside element's id to the inside-boundaries signal.
        // The closure plumbed into OutsideInteractionConfig reads
        // `inside_boundaries.get_untracked()` per event, so this update
        // must take effect immediately on the next dispatch without a
        // remount.
        set_boundaries.set(vec!["late-boundary".into()]);

        tick().await;

        let second_event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct");

        outside
            .dispatch_event(&second_event)
            .expect("dispatch_event should succeed");

        tick().await;

        let log = calls.lock().expect("calls mutex must not be poisoned");

        assert_eq!(
            log.len(),
            1,
            "second pointerdown must NOT fire on_dismiss after the id is registered as an inside-boundary",
        );

        drop(log);

        outside.remove();
        container.remove();
    }
}
