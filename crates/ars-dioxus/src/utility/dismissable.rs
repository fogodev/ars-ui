//! Dioxus adapter for the framework-agnostic `Dismissable` behavior.
//!
//! Mirrors the Leptos adapter (`ars_leptos::utility::dismissable`) one-for-one,
//! composing [`Props`] with the shared
//! [`ars_dom::outside_interaction`] helpers so overlay components across
//! the Dioxus build inherit document-level outside-interaction listeners,
//! portal-aware boundary checks, paired dismiss buttons, and
//! topmost-overlay gating without re-implementing each piece per overlay.
//!
//! This module re-exports the agnostic `ars_components::utility::dismissable`
//! surface (`Props`, `Messages`, `Part`, `DismissReason`, `DismissAttempt`,
//! `dismiss_button_attrs`) so consumers reach every
//! dismissable type through a single namespace —
//! `dismissable::Props`, `dismissable::Region`, `dismissable::Handle`,
//! `dismissable::use_dismissable`, etc.
//!
//! The hook is **web-only**: on Dioxus Desktop, mobile, or SSR builds the
//! `feature = "web"` cfg gates the listener installation and the helper
//! degrades to id allocation plus the visually-hidden dismiss button
//! markup, matching the documented "degrade gracefully" contract from
//! `spec/dioxus-components/utility/dismissable.md` §22.

use std::{
    fmt::{self, Debug},
    rc::Rc,
};

pub use ars_components::utility::dismissable::{
    Api, DismissAttempt, DismissReason, Messages, Part, Props, dismiss_button_attrs,
};
use ars_i18n::Locale;
use dioxus::prelude::*;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    ars_dom::{
        OutsideInteractionConfig,
        overlay_stack::{OverlayEntry, push_overlay, remove_overlay},
    },
    ars_interactions::InteractOutsideEvent,
    std::cell::RefCell,
    web_sys::Element as WebElement,
};

use crate::{
    attrs::attr_map_to_dioxus_inline_attrs,
    id::use_stable_id,
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
/// Both fields are arena-backed Dioxus primitives, so [`Handle`] is `Copy`
/// — consumers can move it into multiple closures or pass it through the
/// rsx tree without an explicit clone. The handle stays valid until the
/// owning Dioxus scope unmounts.
#[derive(Clone, Copy)]
pub struct Handle {
    /// Programmatic dismiss-button activation. Invoking
    /// `EventHandler::call(())` fires
    /// `props.on_dismiss(DismissReason::DismissButton)` if a callback is
    /// registered. Backed by Dioxus's arena-allocated callback storage so
    /// the handle stays `Copy`.
    pub dismiss: EventHandler<()>,

    /// Stable id used for overlay-stack registration, portal-owner
    /// matching, and DOM root resolution. Stored in the Dioxus arena via
    /// [`CopyValue`] so [`Handle`] remains `Copy`; read the underlying
    /// [`String`] with `overlay_id.read()` (borrow guard) or
    /// `overlay_id.with(|id| …)` (closure).
    pub overlay_id: CopyValue<String>,
}

impl Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Handle");

        debug.field("dismiss", &"<EventHandler<()>>");

        if let Ok(id) = self.overlay_id.try_read() {
            debug.field("overlay_id", &*id);
        } else {
            debug.field("overlay_id", &"<disposed>");
        }

        debug.finish()
    }
}

// ────────────────────────────────────────────────────────────────────
// Public hook
// ────────────────────────────────────────────────────────────────────

/// Adapter-owned dismissable hook for Dioxus.
///
/// Allocates an overlay id, registers it on the global overlay stack
/// while mounted, and installs the document `pointerdown`/`focusin`/`keydown`
/// listener triplet via
/// [`ars_dom::install_outside_interaction_listeners`]. Listeners are
/// `feature = "web"`-only — non-web Dioxus builds (Desktop, mobile, SSR)
/// fall through to the degrade-gracefully no-op in `ars-dom`.
///
/// `root_ref` is a `ReadSignal<Option<Rc<MountedData>>>` populated by the
/// consumer's `onmounted: move |evt| signal.set(Some(evt.data()))` handler
/// on the root element — mirroring the Leptos adapter's
/// [`NodeRef`](leptos::prelude::NodeRef)-based handle pattern. The hook
/// downcasts the `MountedData` to a [`web_sys::Element`] before installing
/// listeners; non-web `RenderedElementBacking` impls (Desktop, mobile,
/// SSR) fail the downcast and the install short-circuits, matching the
/// documented graceful-degrade contract.
#[must_use]
pub fn use_dismissable(
    root_ref: ReadSignal<Option<Rc<MountedData>>>,
    props: Props,
    inside_boundaries: ReadSignal<Vec<String>>,
) -> Handle {
    let stable_overlay_id = use_stable_id("dismissable");

    let overlay_id = use_hook(move || CopyValue::new(stable_overlay_id));

    let dismiss = build_dismiss_button_callback(&props);

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    install_dismissable_listeners(root_ref, props, inside_boundaries, overlay_id.cloned());

    // Reference unused parameters under non-web builds so the public
    // signature exercises every input even when listeners are stubbed
    // out.
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
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
fn build_dismiss_button_callback(props: &Props) -> EventHandler<()> {
    let on_dismiss = props.on_dismiss.clone();

    EventHandler::new(move |()| {
        if let Some(cb) = on_dismiss.as_ref() {
            cb(DismissReason::DismissButton);
        }
    })
}

// ────────────────────────────────────────────────────────────────────
// Client-only listener wiring
// ────────────────────────────────────────────────────────────────────

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_dismissable_listeners(
    root_ref: ReadSignal<Option<Rc<MountedData>>>,
    props: Props,
    inside_boundaries: ReadSignal<Vec<String>>,
    overlay_id: String,
) {
    // `use_hook` runs the initializer exactly once for the component's
    // lifetime and returns the same `Rc` on every subsequent render.
    // Without this, every render would allocate a *fresh*
    // `DismissableState`: `use_drop` only captures the first render's
    // closure (so its state would never receive `teardown`), while
    // `use_effect` runs against later renders' fresh states (which
    // install the listeners and overlay-stack entry). Unmount would
    // then skip cleanup, leaking document listeners and stack entries.
    //
    // `props` lives inside the state behind a `RefCell` so subsequent
    // renders can publish updated callbacks, `exclude_ids`, and the
    // `disable_outside_pointer_events` flag without re-installing the
    // listener triplet — the listener closures read through
    // `state.props.borrow()` at fire-time, and the helper's
    // `disable_outside_pointer_events` closure does the same. Without
    // this, the listeners would stay bound to whatever `Props` value
    // was active at install time and silently ignore prop updates.
    let state = use_hook(|| {
        Rc::new(DismissableState {
            overlay_id,
            props: RefCell::new(props.clone()),
            teardown: RefCell::new(None),
            overlay_pushed: RefCell::new(false),
            cleaned_up: RefCell::new(false),
        })
    });

    // Refresh the cached props on every render so callback identities,
    // exclude-id sets, and the modal flag are always read from the
    // most recent component invocation.
    *state.props.borrow_mut() = props;

    use_effect({
        let state = Rc::clone(&state);
        move || {
            attach(&state, root_ref, inside_boundaries);
        }
    });

    use_drop({
        let state = Rc::clone(&state);
        move || {
            teardown(&state);
        }
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct DismissableState {
    overlay_id: String,
    props: RefCell<Props>,
    teardown: RefCell<Option<Box<dyn FnOnce()>>>,
    overlay_pushed: RefCell<bool>,
    cleaned_up: RefCell<bool>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn attach(
    state: &Rc<DismissableState>,
    root_ref: ReadSignal<Option<Rc<MountedData>>>,
    inside_boundaries: ReadSignal<Vec<String>>,
) {
    if *state.cleaned_up.borrow() || state.teardown.borrow().is_some() {
        return;
    }

    // `MountedData::downcast::<web_sys::Element>` returns `None` on
    // non-web `RenderedElementBacking` impls (Dioxus Desktop, mobile,
    // SSR), so the install short-circuits without panicking — matching
    // the documented graceful-degrade contract from
    // `spec/dioxus-components/utility/dismissable.md` §12. On web, also
    // fall back to the stable rendered root id: Dioxus can run effects
    // before `onmounted` has produced a usable mounted handle, while the
    // root element itself is already queryable by the time effects run.
    let root_element = root_ref()
        .and_then(|root_data| root_data.downcast::<WebElement>().cloned())
        .or_else(|| {
            web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(&state.overlay_id))
        });

    let Some(root_element) = root_element else {
        return;
    };

    push_overlay(OverlayEntry {
        id: state.overlay_id.clone(),
        modal: false,
        z_index: None,
    });

    *state.overlay_pushed.borrow_mut() = true;

    // Each dynamic field on `Props` is reached through `state.props`
    // at fire-time so re-renders that publish new callbacks,
    // `exclude_ids`, or a flipped `disable_outside_pointer_events`
    // flag take effect without tearing down and re-attaching the
    // listener triplet.
    let teardown_fn = ars_dom::install_outside_interaction_listeners(
        &root_element,
        OutsideInteractionConfig {
            overlay_id: state.overlay_id.clone(),
            inside_boundaries: Rc::new(move || inside_boundaries.peek().clone()),
            exclude_ids: Rc::new({
                let state = Rc::clone(state);
                move || state.props.borrow().exclude_ids.clone()
            }),
            disable_outside_pointer_events: Rc::new({
                let state = Rc::clone(state);
                move || state.props.borrow().disable_outside_pointer_events
            }),
            on_pointer_outside: Box::new({
                let state = Rc::clone(state);
                move |client_x, client_y, pointer_type| {
                    let props = state.props.borrow();

                    let attempt = DismissAttempt::new(InteractOutsideEvent::PointerOutside {
                        client_x,
                        client_y,
                        pointer_type,
                    });

                    if let Some(cb) = props.on_interact_outside.as_ref() {
                        cb(attempt.clone());
                    }

                    if attempt.is_prevented() {
                        return;
                    }

                    if let Some(cb) = props.on_dismiss.as_ref() {
                        cb(DismissReason::OutsidePointer);
                    }
                }
            }),
            on_focus_outside: Box::new({
                let state = Rc::clone(state);
                move || {
                    let props = state.props.borrow();

                    let attempt = DismissAttempt::new(InteractOutsideEvent::FocusOutside);

                    if let Some(cb) = props.on_interact_outside.as_ref() {
                        cb(attempt.clone());
                    }

                    if attempt.is_prevented() {
                        return;
                    }

                    if let Some(cb) = props.on_dismiss.as_ref() {
                        cb(DismissReason::OutsideFocus);
                    }
                }
            }),
            on_escape: Box::new({
                let state = Rc::clone(state);
                move || {
                    let props = state.props.borrow();

                    let attempt = DismissAttempt::new(());

                    if let Some(cb) = props.on_escape_key_down.as_ref() {
                        cb(attempt.clone());
                    }

                    // Whether or not the consumer vetoed the dismiss
                    // decision, the topmost overlay received the Escape
                    // and gets to consume it. Returning `true` here
                    // makes the helper call `Event::stop_propagation`
                    // so ancestor overlays and global `keydown`
                    // handlers don't also react to the same keystroke
                    // (per `spec/foundation/05-interactions.md` §12.6).
                    if attempt.is_prevented() {
                        return true;
                    }

                    if let Some(cb) = props.on_dismiss.as_ref() {
                        cb(DismissReason::Escape);
                    }

                    true
                }
            }),
        },
    );

    *state.teardown.borrow_mut() = Some(teardown_fn);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
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

/// Props for [`Region`].
#[derive(Props, Clone, Debug, PartialEq)]
pub struct RegionProps {
    /// Behavioural props forwarded to [`use_dismissable`].
    pub props: Props,

    /// Optional reactive list of additional inside-boundary ids.
    /// Defaults to an empty signal so the trigger element alone is
    /// matched against `props.exclude_ids`.
    #[props(optional)]
    pub inside_boundaries: Option<ReadSignal<Vec<String>>>,

    /// Explicit accessible-label override for both visually-hidden
    /// dismiss buttons.
    ///
    /// When omitted, [`Region`] resolves [`Messages`] from the nearest
    /// `ArsProvider` message registry and locale, falling back to
    /// `"Dismiss"`. Overlay components may pass context-specific
    /// wording such as `"Close dialog"` or `"Dismiss popover"`.
    #[props(optional)]
    pub dismiss_label: Option<String>,

    /// Explicit locale override used when resolving provider/default
    /// [`Messages`].
    #[props(optional, into)]
    pub locale: Option<Signal<Locale>>,

    /// Explicit message-bundle override used when `dismiss_label` is
    /// omitted.
    #[props(optional)]
    pub messages: Option<Messages>,

    /// Children rendered between the start and end dismiss buttons.
    pub children: Element,
}

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
/// wants the documented `DismissButton` (start) → content →
/// `DismissButton` (end) anatomy with the right attrs and listener
/// wiring.
#[component]
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion confuses the unused-qualifications lint on onclick: bindings."
)]
pub fn Region(props: RegionProps) -> Element {
    let RegionProps {
        props,
        inside_boundaries,
        dismiss_label,
        locale,
        messages,
        children,
    } = props;

    let boundaries_fallback = use_signal(Vec::<String>::new);

    let boundaries = inside_boundaries.unwrap_or_else(|| ReadSignal::from(boundaries_fallback));

    let provider_locale = resolve_locale(None);
    let resolved_locale = locale
        .as_ref()
        .map_or(provider_locale, |locale| locale.read().clone());

    let resolved_messages = use_messages(messages.as_ref(), Some(&resolved_locale));
    let dismiss_label =
        dismiss_label.unwrap_or_else(|| (resolved_messages.dismiss_label)(&resolved_locale));

    let api = Api::new(props.clone(), dismiss_label);

    let root_attrs = attr_map_to_dioxus_inline_attrs(api.root_attrs());

    let inline_attrs = attr_map_to_dioxus_inline_attrs(api.dismiss_button_attrs());
    let start_attrs = inline_attrs.clone();
    let end_attrs = inline_attrs;

    let mut root_ref = use_signal(|| None::<Rc<MountedData>>);

    let handle = use_dismissable(ReadSignal::from(root_ref), props, boundaries);
    let root_id = handle.overlay_id.cloned();

    rsx! {
        div {
            id: root_id,
            onmounted: move |evt| {
                root_ref.set(Some(evt.data()));
            },
            ..root_attrs,
            button {
                onclick: move |_| {
                    handle.dismiss.call(());
                },
                ..start_attrs,
            }
            {children}
            button {
                onclick: move |_| {
                    handle.dismiss.call(());
                },
                ..end_attrs,
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Tests that exercise `build_dismiss_button_callback`, `Handle`, or
    // `Handle`'s `Debug` impl now require a Dioxus runtime (EventHandler +
    // CopyValue both live in the scope-local arena). They are covered
    // end-to-end through `crates/ars-dioxus/tests/desktop_dismissable.rs`,
    // which mounts a real fixture inside `DesktopHarness::launch_with_props`
    // — see `region_mounts_on_desktop_without_panic`,
    // `handle_dismiss_fires_on_dismiss_with_dismiss_button_reason`,
    // `handle_is_copy_and_shares_overlay_id`, and
    // `handle_debug_includes_overlay_id` for the equivalent assertions.

    #[test]
    fn dismissable_region_props_clone_preserves_inner_props() {
        let outer = RegionProps {
            props: Props::new().exclude_ids(["trigger"]),
            inside_boundaries: None,
            dismiss_label: Some(String::from("Dismiss")),
            locale: None,
            messages: None,
            children: Ok(VNode::placeholder()),
        };

        let cloned = outer.clone();

        assert_eq!(cloned.props.exclude_ids, vec!["trigger".to_string()]);
    }
}

// ────────────────────────────────────────────────────────────────────
// Wasm tests
// ────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::sync::{Arc, Mutex};

    use ars_core::{I18nRegistries, MessageFn, MessagesRegistry};
    use ars_i18n::Locale;
    use dioxus::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, Element as WebElementHandle};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        web_sys::window()
            .and_then(|w| w.document())
            .expect("browser document should exist")
    }

    fn with_container() -> WebElementHandle {
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

    fn first_dismiss_button_label(container: &WebElementHandle) -> String {
        container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("at least one dismiss button must exist")
            .get_attribute("aria-label")
            .expect("dismiss button must carry an aria-label")
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

    async fn microtask_turn() {
        drop(
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                &wasm_bindgen::JsValue::UNDEFINED,
            ))
            .await,
        );
    }

    /// Wait the supplied number of milliseconds to give
    /// `launch_virtual_dom`'s `spawn_local` task time to run, mount, fire
    /// `onmounted`, and trigger the listener-install effect. The harness
    /// uses a smaller animation-frame-based flush, but `launch_virtual_dom`
    /// schedules through multiple microtasks before the first commit
    /// reaches the DOM, so we use a real timer for reliability.
    async fn sleep_ms(ms: i32) {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            web_sys::window()
                .expect("window should exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .expect("setTimeout should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
    }

    /// Drives the launched `VirtualDom` long enough for mount, `onmounted`,
    /// and the listener-install effect to settle. Document / root
    /// `pointerdown` / `focusin` / `keydown` listeners attach during the
    /// effect after `onmounted` populates `root_ref`, so the timer-driven
    /// turns matter here.
    async fn flush() {
        for _ in 0..3 {
            animation_frame_turn().await;

            microtask_turn().await;
        }

        sleep_ms(100).await;

        for _ in 0..3 {
            animation_frame_turn().await;

            microtask_turn().await;
        }
    }

    #[derive(Clone)]
    struct FixtureProps {
        dismiss_log: Arc<Mutex<Vec<DismissReason>>>,
    }

    impl PartialEq for FixtureProps {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.dismiss_log, &other.dismiss_log)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn fixture(state: FixtureProps) -> Element {
        let dismiss_log = Arc::clone(&state.dismiss_log);

        let props = Props::new().on_dismiss(move |reason| {
            dismiss_log
                .lock()
                .expect("dismiss_log mutex must not be poisoned")
                .push(reason);
        });

        rsx! {
            Region { props,
                span { "content" }
            }
        }
    }

    fn build_state() -> FixtureProps {
        FixtureProps {
            dismiss_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn launch(container: &WebElementHandle, props: FixtureProps) {
        let dom = VirtualDom::new_with_props(fixture, props);

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );
    }

    #[wasm_bindgen_test]
    async fn region_renders_paired_dismiss_buttons_on_wasm() {
        let container = with_container();

        let state = build_state();

        launch(&container, state);

        flush().await;

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

    fn provider_label_fixture() -> Element {
        let i18n_registries = spanish_messages();
        let locale = use_signal(|| Locale::parse("es-MX").expect("locale should parse"));

        rsx! {
            crate::ArsProvider { locale, i18n_registries,
                Region { props: Props::new(),
                    span { "content" }
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn region_default_label_uses_provider_messages_on_wasm() {
        let container = with_container();

        let dom = VirtualDom::new(provider_label_fixture);

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush().await;

        assert_eq!(
            first_dismiss_button_label(&container),
            "Cerrar",
            "Region default dismiss label must resolve through provider messages",
        );

        container.remove();
    }

    #[wasm_bindgen_test]
    async fn dismiss_button_click_fires_on_dismiss_with_dismiss_button_reason_on_wasm() {
        let container = with_container();

        let state = build_state();

        launch(&container, state.clone());

        flush().await;

        let button = container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("at least one dismiss button must exist");

        let html_button: web_sys::HtmlElement = button.unchecked_into();

        html_button.click();

        flush().await;

        let log = state
            .dismiss_log
            .lock()
            .expect("dismiss_log mutex must not be poisoned");

        assert_eq!(
            log.as_slice(),
            &[DismissReason::DismissButton],
            "clicking the visually-hidden dismiss button must fire on_dismiss with DismissButton",
        );

        drop(log);

        container.remove();
    }

    // Escape and outside-pointerdown wasm tests are intentionally
    // omitted from this initial Dioxus wasm test pass — under
    // `dioxus_web::launch_virtual_dom`, the document `pointerdown` /
    // `focusin` / `keydown` listener attachment doesn't
    // fire reliably from synthetic `dispatch_event(...)` calls inside
    // a `#[wasm_bindgen_test]`, even with extended flush windows. The
    // Leptos adapter's symmetric tests (`escape_keydown_fires…` /
    // `outside_pointerdown_fires…`) cover the agnostic listener install
    // path through `ars_dom::install_outside_interaction_listeners`, so
    // the contract is end-to-end-tested at the workspace level.
    //
    // Tracked under issue #612 — the Desktop click-outside bridge work
    // there will require expanded harness support for synthetic event
    // dispatch on Dioxus targets, which can land the Escape / outside-
    // pointerdown wasm tests at the same time.

    #[derive(Clone)]
    struct LabelFixtureProps {
        dismiss_label: String,
    }

    impl PartialEq for LabelFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            self.dismiss_label == other.dismiss_label
        }
    }

    fn label_fixture(state: LabelFixtureProps) -> Element {
        let i18n_registries = spanish_messages();
        let locale = use_signal(|| Locale::parse("es-MX").expect("locale should parse"));

        rsx! {
            crate::ArsProvider { locale, i18n_registries,
                Region { props: Props::new(), dismiss_label: state.dismiss_label,
                    span { "content" }
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn region_dismiss_label_prop_overrides_provider_messages_on_wasm() {
        let container = with_container();

        let dom = VirtualDom::new_with_props(
            label_fixture,
            LabelFixtureProps {
                dismiss_label: String::from("Close dialog"),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush().await;

        assert_eq!(
            first_dismiss_button_label(&container),
            "Close dialog",
            "Region dismiss_label prop must override provider messages",
        );

        container.remove();
    }

    #[derive(Clone)]
    struct LabelSwapFixtureProps {
        initial_label: String,
        swapped_label: String,
    }

    impl PartialEq for LabelSwapFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            self.initial_label == other.initial_label && self.swapped_label == other.swapped_label
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    #[expect(
        unused_qualifications,
        reason = "rsx! macro expansion adds qualified paths around onclick closure capture."
    )]
    fn label_swap_fixture(state: LabelSwapFixtureProps) -> Element {
        let mut label_signal = use_signal(|| state.initial_label.clone());
        let swapped_for_handler = state.swapped_label.clone();
        let dismiss_label = label_signal();

        rsx! {
            Region { props: Props::new(), dismiss_label: Some(dismiss_label),
                span { "content" }
            }
            // Hidden swap-trigger button — the test clicks this to mutate
            // the label signal from inside the Dioxus runtime.
            button {
                id: "swap-trigger",
                onclick: move |_| {
                    label_signal.set(swapped_for_handler.clone());
                },
            }
        }
    }

    #[wasm_bindgen_test]
    async fn region_dismiss_label_signal_updates_button_label_on_wasm() {
        let container = with_container();

        let dom = VirtualDom::new_with_props(
            label_swap_fixture,
            LabelSwapFixtureProps {
                initial_label: String::from("Dismiss"),
                swapped_label: String::from("Close dialog"),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush().await;

        let dismiss_button = container
            .query_selector("button[data-ars-part='dismiss-button']")
            .expect("query_selector should succeed")
            .expect("dismiss button must exist");

        let initial = dismiss_button
            .get_attribute("aria-label")
            .expect("dismiss button must carry an aria-label");

        assert_eq!(
            initial, "Dismiss",
            "Region aria-label must reflect the initial dismiss_label value",
        );

        // Click the swap trigger to mutate the label signal from inside
        // the runtime. Dioxus then invalidates the component scope and
        // updates the rendered `aria-label`.
        let trigger = container
            .query_selector("#swap-trigger")
            .expect("query_selector should succeed")
            .expect("swap trigger must exist");

        let html_trigger: web_sys::HtmlElement = trigger.unchecked_into();

        html_trigger.click();

        flush().await;

        let updated = dismiss_button
            .get_attribute("aria-label")
            .expect("dismiss button must still carry an aria-label after label change");

        assert_eq!(
            updated, "Close dialog",
            "Region aria-label must update when the dismiss_label signal changes",
        );

        container.remove();
    }
}
