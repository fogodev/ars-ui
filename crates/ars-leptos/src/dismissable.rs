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
//! surface (`Props`, `Messages`, `Part`, `DismissReason`, `DismissAttempt`,
//! `dismiss_button_attrs`) so consumers reach every
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

use std::{
    fmt::{self, Debug},
    sync::Arc,
};

pub use ars_components::utility::dismissable::{
    Api, DismissAttempt, DismissReason, Messages, Part, Props, dismiss_button_attrs,
};
use ars_core::{I18nRegistries, resolve_messages};
use ars_i18n::Locale;
use leptos::{callback::Callback as LeptosCallback, html, prelude::*};
#[cfg(not(feature = "ssr"))]
use {
    ars_dom::{
        OutsideInteractionConfig,
        overlay_stack::{OverlayEntry, push_overlay, remove_overlay},
    },
    ars_interactions::InteractOutsideEvent,
    leptos::web_sys,
    std::{cell::RefCell, rc::Rc},
};

use crate::{
    attrs::attr_map_to_leptos_inline_attrs,
    id::use_id,
    provider::{current_ars_context, use_locale},
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

    let Some(root_element) = root_ref.get() else {
        return;
    };

    let root_element: web_sys::HtmlElement = (*root_element).clone();
    let root_element: web_sys::Element = root_element.into();

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
        disable_outside_pointer_events,
        exclude_ids,
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
            // Leptos component bodies run once, so the props captured
            // here are stable for the component lifetime; wrap the
            // boolean in a closure to match the agnostic helper's
            // `Rc<dyn Fn() -> bool>` shape.
            disable_outside_pointer_events: Rc::new(move || disable_outside_pointer_events),
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

                // Whether or not the consumer vetoed the dismiss decision,
                // the topmost overlay received the Escape and gets to
                // consume it. Returning `true` here makes the helper call
                // `Event::stop_propagation` so ancestor overlays and
                // global `keydown` handlers don't also react to the same
                // keystroke (per `spec/foundation/05-interactions.md`
                // §12.6).
                if attempt.is_prevented() {
                    return true;
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

    /// Explicit accessible-label override for both visually-hidden
    /// dismiss buttons.
    ///
    /// When omitted, [`Region`] resolves [`Messages`] from the nearest
    /// `ArsProvider` message registry and locale, falling back to
    /// `"Dismiss"`. Overlay components may pass context-specific
    /// wording such as `"Close dialog"` or `"Dismiss popover"`.
    #[prop(optional, into)]
    dismiss_label: Option<Signal<String>>,

    /// Explicit locale override used when resolving provider/default
    /// [`Messages`].
    #[prop(optional, into)]
    locale: Option<Signal<Locale>>,

    /// Explicit message-bundle override used when `dismiss_label` is
    /// omitted.
    #[prop(optional)]
    messages: Option<Messages>,

    /// Children rendered between the start and end dismiss buttons.
    children: Children,
) -> impl IntoView {
    let root_ref = NodeRef::<html::Div>::new();

    let boundaries = inside_boundaries.unwrap_or_else(|| Signal::stored(Vec::new()));

    let provider_locale = use_locale();

    let registries = current_ars_context().map_or_else(
        || Arc::new(I18nRegistries::new()),
        |ctx| Arc::clone(&ctx.i18n_registries),
    );

    let dismiss_label = dismiss_label.unwrap_or_else(|| {
        Signal::derive(move || {
            let resolved_locale = locale
                .as_ref()
                .map_or_else(|| provider_locale.get(), |locale| locale.get());

            let resolved_messages =
                resolve_messages(messages.as_ref(), registries.as_ref(), &resolved_locale);

            (resolved_messages.dismiss_label)(&resolved_locale)
        })
    });

    let api = Api::new(props.clone(), move || dismiss_label.get());

    let root_attrs = attr_map_to_leptos_inline_attrs(api.root_attrs());
    let inline_attrs = attr_map_to_leptos_inline_attrs(api.dismiss_button_attrs());
    let start_attrs = inline_attrs.clone();
    let end_attrs = inline_attrs;

    let handle = use_dismissable(root_ref, props, boundaries);

    view! {
        <div {..root_attrs} node_ref=root_ref>
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

    use ars_core::{I18nRegistries, MessageFn, MessagesRegistry};
    use ars_i18n::Locale;
    use leptos::{mount::mount_to, prelude::*, reactive::owner::Owner};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, Element, EventInit, HtmlElement, KeyboardEventInit, PointerEventInit};

    // Explicit imports to shadow `leptos::prelude::Props` (the
    // component-props trait) with the dismissable struct.
    use super::{DismissAttempt, DismissReason, Messages, Props, Region};

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        web_sys::window()
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

    fn spanish_messages() -> Arc<I18nRegistries> {
        let mut registries = I18nRegistries::new();

        registries.register(MessagesRegistry::new(Messages::default()).register(
            "es",
            Messages {
                dismiss_label: MessageFn::static_str("Cerrar"),
            },
        ));

        Arc::new(registries)
    }

    fn first_dismiss_button_label(container: &Element) -> String {
        container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("at least one dismiss button must exist")
            .get_attribute("aria-label")
            .expect("dismiss button must carry an aria-label")
    }

    /// Tick the Leptos reactive cycle once.
    async fn tick() {
        leptos::task::tick().await;
    }

    /// Builds a `pointerdown` `PointerEvent` whose `clientX` / `clientY`
    /// fall inside `target`'s bounding rect.
    ///
    /// `ars_dom::install_outside_interaction_listeners` resolves the
    /// pointer-event target via `Document::element_from_point(clientX,
    /// clientY)` first (with `event.target()` as fallback) so it
    /// classifies pointer-capture interactions correctly. Synthetic
    /// pointerdowns with default-zero coords would resolve to whatever
    /// sits at viewport `(0, 0)` rather than the dispatched element,
    /// so each test that wants the "real" target classification
    /// centers the event on the target's bounding rect.
    fn pointerdown_centered_on(target: &Element) -> web_sys::PointerEvent {
        let rect = target.get_bounding_client_rect();
        // `as i32` truncation is harmless: PointerEventInit takes `i32`
        // pixel coordinates, and bbox values fall well within `i32`.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "PointerEventInit takes i32 pixel coords; bbox values fit i32."
        )]
        let center_x = (rect.left() + rect.width() / 2.0) as i32;

        #[expect(
            clippy::cast_possible_truncation,
            reason = "PointerEventInit takes i32 pixel coords; bbox values fit i32."
        )]
        let center_y = (rect.top() + rect.height() / 2.0) as i32;

        let init = PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");
        init.set_client_x(center_x);
        init.set_client_y(center_y);

        web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown event should construct")
    }

    /// Forces `target` to render with non-zero pixel dimensions so
    /// `Document::element_from_point` can hit-test it. Default
    /// `<div>` / `<button>` flow elements often render zero-height in a
    /// fresh test container, which would make
    /// `pointerdown_centered_on` resolve to `body`/`html` instead of
    /// the intended target.
    fn ensure_hit_testable(target: &Element) {
        let html: HtmlElement = target.clone().unchecked_into();

        html.style()
            .set_property("display", "block")
            .expect("style display");

        html.style()
            .set_property("min-width", "10px")
            .expect("style min-width");

        html.style()
            .set_property("min-height", "10px")
            .expect("style min-height");
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
    async fn region_default_label_uses_provider_messages_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();
        let registries = spanish_messages();

        let _handle = mount_to(parent, move || {
            view! {
                <crate::ArsProvider
                    locale=Locale::parse("es-MX").expect("locale should parse")
                    i18n_registries=Arc::clone(&registries)
                >
                    <Region props=Props::new()>
                        <span>"content"</span>
                    </Region>
                </crate::ArsProvider>
            }
        });

        tick().await;

        assert_eq!(
            first_dismiss_button_label(&container),
            "Cerrar",
            "Region default dismiss label must resolve through provider messages",
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
    async fn prevent_dismiss_in_on_escape_key_down_still_consumes_event_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let escape_calls = Arc::new(AtomicUsize::new(0));
        let dismiss_log = Arc::new(Mutex::new(Vec::<DismissReason>::new()));
        let outer_keydown_calls = Arc::new(AtomicUsize::new(0));

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        // Outer keydown listener on the container — this should NOT
        // fire when the inner overlay's `on_escape` consumes the event.
        let outer_for_listener = Arc::clone(&outer_keydown_calls);

        let outer_listener =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::KeyboardEvent| {
                outer_for_listener.fetch_add(1, Ordering::SeqCst);
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        let outer_target: web_sys::EventTarget = container.clone().unchecked_into();

        outer_target
            .add_event_listener_with_callback("keydown", outer_listener.as_ref().unchecked_ref())
            .expect("outer listener must attach");

        let escape_for_props = Arc::clone(&escape_calls);
        let dismiss_for_props = Arc::clone(&dismiss_log);

        let _handle = mount_to(parent, move || {
            let escape_for_callback = Arc::clone(&escape_for_props);
            let dismiss_for_callback = Arc::clone(&dismiss_for_props);
            let props = Props::new()
                .on_escape_key_down(move |attempt: DismissAttempt<()>| {
                    escape_for_callback.fetch_add(1, Ordering::SeqCst);
                    attempt.prevent_dismiss();
                })
                .on_dismiss(move |reason: DismissReason| {
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

        assert_eq!(
            escape_calls.load(Ordering::SeqCst),
            1,
            "on_escape_key_down must fire once for the Escape keydown",
        );

        let dismiss_log_inner = dismiss_log
            .lock()
            .expect("dismiss_log mutex must not be poisoned");

        assert!(
            dismiss_log_inner.is_empty(),
            "calling DismissAttempt::prevent_dismiss must veto the on_dismiss call",
        );

        drop(dismiss_log_inner);

        // The critical assertion: even though the consumer vetoed
        // dismissal, the topmost overlay still consumed the Escape so
        // ancestor / global keydown handlers never see it. Without the
        // fix this counter is 1 (event bubbled to the outer listener).
        assert_eq!(
            outer_keydown_calls.load(Ordering::SeqCst),
            0,
            "Escape on a topmost overlay must always stop_propagation, \
             even when dismissal is vetoed — the outer keydown listener \
             must not receive the event",
        );

        drop(outer_target.remove_event_listener_with_callback(
            "keydown",
            outer_listener.as_ref().unchecked_ref(),
        ));

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

        ensure_hit_testable(&outside);

        let event = pointerdown_centered_on(&outside);

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

        ensure_hit_testable(&outside);

        let event = pointerdown_centered_on(&outside);

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

        ensure_hit_testable(&trigger);

        let event = pointerdown_centered_on(&trigger);

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
    async fn region_dismiss_label_prop_overrides_provider_messages_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();
        let registries = spanish_messages();

        let _handle = mount_to(parent, move || {
            view! {
                <crate::ArsProvider
                    locale=Locale::parse("es-MX").expect("locale should parse")
                    i18n_registries=Arc::clone(&registries)
                >
                    <Region
                        props=Props::new()
                        dismiss_label="Close dialog"
                    >
                        <span>"content"</span>
                    </Region>
                </crate::ArsProvider>
            }
        });

        tick().await;

        assert_eq!(
            first_dismiss_button_label(&container),
            "Close dialog",
            "Region dismiss_label prop must override provider messages",
        );

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn region_dismiss_label_signal_updates_button_label_on_wasm() {
        let owner = Owner::new();
        owner.set();

        let container = with_container();
        let parent: HtmlElement = container.clone().unchecked_into();

        let label_signal = RwSignal::new(String::from("Dismiss"));
        let dismiss_label: Signal<String> = Signal::from(label_signal);

        let _handle = mount_to(parent, move || {
            view! {
                <Region
                    props=Props::new()
                    dismiss_label=dismiss_label
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

        let initial = button
            .get_attribute("aria-label")
            .expect("dismiss button must carry an aria-label");

        assert_eq!(
            initial, "Dismiss",
            "Region aria-label must reflect the initial dismiss_label signal value",
        );

        label_signal.set(String::from("Close dialog"));

        tick().await;

        let updated = button
            .get_attribute("aria-label")
            .expect("dismiss button must still carry an aria-label after label change");

        assert_eq!(
            updated, "Close dialog",
            "Region aria-label must update when the dismiss_label signal changes",
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

        ensure_hit_testable(&outside);

        let first_event = pointerdown_centered_on(&outside);

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

        let second_event = pointerdown_centered_on(&outside);

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
