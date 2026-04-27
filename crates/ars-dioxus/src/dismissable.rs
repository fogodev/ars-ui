//! Dioxus adapter for the framework-agnostic `Dismissable` behavior.
//!
//! Mirrors the Leptos adapter (`ars_leptos::dismissable`) one-for-one,
//! composing [`Props`] with the shared
//! [`ars_dom::outside_interaction`] helpers so overlay components across
//! the Dioxus build inherit document-level outside-interaction listeners,
//! portal-aware boundary checks, paired dismiss buttons, and
//! topmost-overlay gating without re-implementing each piece per overlay.
//!
//! This module re-exports the agnostic `ars_components::utility::dismissable`
//! surface (`Props`, `Messages`, `Part`, `DismissReason`,
//! `DismissAttempt`, `dismiss_button_attrs`) so consumers reach every
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
    DismissAttempt, DismissReason, Messages, Part, Props, dismiss_button_attrs,
};
use dioxus::prelude::*;
#[cfg(feature = "web")]
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
/// Both fields are arena-backed Dioxus primitives, so [`Handle`] is `Copy`
/// — consumers can move it into multiple closures or pass it through the
/// rsx tree without an explicit clone. The handle stays valid until the
/// owning Dioxus scope unmounts.
#[derive(Clone, Copy)]
pub struct Handle {
    /// Programmatic dismiss-button activation. Invoking
    /// `Callback::call(())` fires
    /// `props.on_dismiss(DismissReason::DismissButton)` if a callback is
    /// registered. Backed by Dioxus's arena-allocated callback storage so
    /// the handle stays `Copy`.
    pub dismiss: Callback<()>,

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

        debug.field("dismiss", &"<Callback<()>>");

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
/// while mounted, and installs the document `pointerdown`/`focusin` and
/// root-scoped `keydown` listener triplet via
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
    let overlay_id = use_hook(|| CopyValue::new(use_id("ars-dismissable")));

    let dismiss = build_dismiss_button_callback(&props);

    #[cfg(feature = "web")]
    install_dismissable_listeners(root_ref, props, inside_boundaries, overlay_id.cloned());

    // Reference unused parameters under non-web builds so the public
    // signature exercises every input even when listeners are stubbed
    // out.
    #[cfg(not(feature = "web"))]
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
fn build_dismiss_button_callback(props: &Props) -> Callback<()> {
    let on_dismiss = props.on_dismiss.clone();

    Callback::new(move |()| {
        if let Some(cb) = on_dismiss.as_ref() {
            cb(DismissReason::DismissButton);
        }
    })
}

// ────────────────────────────────────────────────────────────────────
// Client-only listener wiring
// ────────────────────────────────────────────────────────────────────

#[cfg(feature = "web")]
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
    let state = use_hook(|| {
        Rc::new(DismissableState {
            overlay_id,
            props,
            teardown: RefCell::new(None),
            overlay_pushed: RefCell::new(false),
            cleaned_up: RefCell::new(false),
        })
    });

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

#[cfg(feature = "web")]
struct DismissableState {
    overlay_id: String,
    props: Props,
    teardown: RefCell<Option<Box<dyn FnOnce()>>>,
    overlay_pushed: RefCell<bool>,
    cleaned_up: RefCell<bool>,
}

#[cfg(feature = "web")]
fn attach(
    state: &Rc<DismissableState>,
    root_ref: ReadSignal<Option<Rc<MountedData>>>,
    inside_boundaries: ReadSignal<Vec<String>>,
) {
    if *state.cleaned_up.borrow() || state.teardown.borrow().is_some() {
        return;
    }

    // Subscribe via `read()` — `onmounted` populates the ref *after*
    // the use_effect first runs, so without subscribing the effect
    // would short-circuit on the initial `None` and never re-run.
    // Re-runs are idempotent: the `cleaned_up` / `teardown.is_some()`
    // guards above ensure listeners install at most once.
    let mounted = root_ref.read();

    let Some(root_data) = mounted.as_ref() else {
        return;
    };

    // `MountedData::downcast::<web_sys::Element>` returns `None` on
    // non-web `RenderedElementBacking` impls (Dioxus Desktop, mobile,
    // SSR), so the install short-circuits without panicking — matching
    // the documented graceful-degrade contract from
    // `spec/dioxus-components/utility/dismissable.md` §12.
    let Some(root_element) = root_data.downcast::<WebElement>().cloned() else {
        return;
    };

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
    let on_dismiss_for_escape = on_dismiss;

    let on_interact_outside_for_pointer = on_interact_outside.clone();
    let on_interact_outside_for_focus = on_interact_outside;

    let teardown_fn = ars_dom::install_outside_interaction_listeners(
        &root_element,
        OutsideInteractionConfig {
            overlay_id: state.overlay_id.clone(),
            inside_boundaries: Rc::new(move || inside_boundaries.peek().clone()),
            exclude_ids: Rc::new(move || exclude_ids.clone()),
            disable_outside_pointer_events,
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

#[cfg(feature = "web")]
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

    /// Optional locale override — falls through to the surrounding
    /// [`ArsProvider`](crate::ArsContext) locale when [`None`].
    #[props(optional)]
    pub locale: Option<ars_i18n::Locale>,

    /// Optional message bundle override — falls through to the
    /// adapter's [`use_messages`] resolution chain (props →
    /// [`I18nRegistries`] → [`Messages::default`]) when [`None`].
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
        locale: locale_override,
        messages: messages_override,
        children,
    } = props;

    let boundaries_fallback = use_signal(Vec::<String>::new);

    let boundaries = inside_boundaries.unwrap_or_else(|| ReadSignal::from(boundaries_fallback));

    let messages = use_messages::<Messages>(messages_override.as_ref(), locale_override.as_ref());

    // Both `resolve_locale` and the bundle resolution above subscribe to
    // their reactive sources (`use_locale`, `use_context::<I18nRegistries>`)
    // through the Dioxus runtime; deriving `dismiss_label` at the component-
    // body level lets the rendered `aria-label` re-resolve when the
    // surrounding `ArsProvider`'s locale or the `I18nRegistries` bundle
    // updates at runtime. Memoizing with `use_hook` would freeze the label
    // at first render and break runtime language switching.
    let resolved_locale = resolve_locale(locale_override.as_ref());
    let dismiss_label = (messages.dismiss_label)(&resolved_locale);

    let dismiss_attrs = dismiss_button_attrs(dismiss_label);

    let inline_attrs = attr_map_to_dioxus_inline_attrs(dismiss_attrs);
    let start_attrs = inline_attrs.clone();
    let end_attrs = inline_attrs;

    let mut root_ref = use_signal(|| None::<Rc<MountedData>>);

    let handle = use_dismissable(ReadSignal::from(root_ref), props, boundaries);

    rsx! {
        div {
            onmounted: move |evt| {
                root_ref.set(Some(evt.data()));
            },
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
    // `Handle`'s `Debug` impl now require a Dioxus runtime (Callback +
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
    use ars_i18n::{Locale, locales};
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
    // `focusin` and root-scoped `keydown` listener attachment doesn't
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
    struct LocaleFixtureProps {
        locale: Locale,
        messages: Messages,
    }

    impl PartialEq for LocaleFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            self.locale == other.locale && self.messages == other.messages
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn locale_fixture(state: LocaleFixtureProps) -> Element {
        rsx! {
            Region {
                props: Props::new(),
                locale: state.locale.clone(),
                messages: state.messages.clone(),
                span { "content" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn region_locale_override_resolves_label_through_use_messages_on_wasm() {
        let container = with_container();

        let messages = Messages {
            dismiss_label: MessageFn::new(|locale: &Locale| {
                if locale.language() == "de" {
                    String::from("Schließen")
                } else {
                    String::from("Dismiss")
                }
            }),
        };

        let dom = VirtualDom::new_with_props(
            locale_fixture,
            LocaleFixtureProps {
                locale: locales::de_de(),
                messages,
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );

        flush().await;

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

    #[derive(Clone)]
    struct LocaleSwapFixtureProps {
        initial_locale: Locale,
        swapped_locale: Locale,
    }

    impl PartialEq for LocaleSwapFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            self.initial_locale == other.initial_locale
                && self.swapped_locale == other.swapped_locale
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
    fn locale_swap_fixture(state: LocaleSwapFixtureProps) -> Element {
        let mut locale_signal = use_signal(|| state.initial_locale.clone());
        let swapped_for_handler = state.swapped_locale.clone();

        // Locale-aware Messages bundle so the rendered `aria-label` differs
        // between English and German.
        let messages = Messages {
            dismiss_label: MessageFn::new(|locale: &Locale| {
                if locale.language() == "de" {
                    String::from("Schließen")
                } else {
                    String::from("Dismiss")
                }
            }),
        };

        let mut registries = I18nRegistries::new();

        registries.register::<Messages>(MessagesRegistry::new(messages));

        let registries = Arc::new(registries);

        rsx! {
            crate::ArsProvider { locale: locale_signal, i18n_registries: registries,
                Region { props: Props::new(),
                    span { "content" }
                }
                // Hidden swap-trigger button — the test clicks this to
                // mutate the locale signal from inside the Dioxus runtime
                // (signals can only be set from inside the runtime scope).
                button {
                    id: "swap-trigger",
                    onclick: move |_| {
                        locale_signal.set(swapped_for_handler.clone());
                    },
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn region_aria_label_updates_when_provider_locale_signal_changes_on_wasm() {
        let container = with_container();

        let dom = VirtualDom::new_with_props(
            locale_swap_fixture,
            LocaleSwapFixtureProps {
                initial_locale: locales::en_us(),
                swapped_locale: locales::de_de(),
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
            "Region aria-label must reflect the initial English locale from the provider",
        );

        // Click the swap trigger to mutate the locale signal from inside
        // the runtime. Dioxus then invalidates the Region's component
        // scope; the `dismiss_label` re-derives in the next render and
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
            .expect("dismiss button must still carry an aria-label after locale swap");

        assert_eq!(
            updated, "Schließen",
            "Region aria-label must re-resolve through use_messages when the provider locale signal updates at runtime",
        );

        container.remove();
    }
}
