//! DOM bootstrap for shared live-region announcer nodes.

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    ars_a11y::LiveAnnouncer,
    ars_core::{AttrMap, AttrValue, HtmlAttr},
    std::{cell::RefCell, collections::HashMap, string::String},
    wasm_bindgen::{JsCast, closure::Closure},
};

/// Ensure the shared live-region DOM nodes exist.
#[cfg(feature = "web")]
pub fn ensure_dom() {
    ensure_dom_impl();
}

/// Announce `message` through the polite live region.
#[cfg(feature = "web")]
pub(crate) fn announce_polite(message: &str) {
    announce_impl("ars-live-polite", message);
}

/// Announce `message` through the assertive live region.
#[cfg(feature = "web")]
pub(crate) fn announce_assertive(message: &str) {
    announce_impl("ars-live-assertive", message);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn ensure_dom_impl() {
    let Some(document) = document() else {
        crate::debug::warn_skipped("ensure_dom()", "window.document");

        return;
    };

    let Some(body) = document.body() else {
        crate::debug::warn_skipped("ensure_dom()", "document.body");

        return;
    };

    ensure_region(
        &document,
        body.as_ref(),
        &LiveAnnouncer::polite_region_attrs(),
    );

    ensure_region(
        &document,
        body.as_ref(),
        &LiveAnnouncer::assertive_region_attrs(),
    );
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn ensure_dom_impl() {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
const LIVE_REGION_INSERT_DELAY_MS: i32 = 150;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
const LIVE_REGION_CLEAR_DELAY_MS: i32 = 7_000;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Default)]
struct PendingRegionTimers {
    insert_timeout_id: Option<i32>,
    insert_callback: Option<Closure<dyn FnMut()>>,
    clear_timeout_id: Option<i32>,
    clear_callback: Option<Closure<dyn FnMut()>>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Default)]
struct RegionAnnouncementState {
    last_message: Option<String>,
    voiceover_toggle: bool,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
thread_local! {
    static LIVE_REGION_TIMERS: RefCell<HashMap<String, PendingRegionTimers>> =
        RefCell::new(HashMap::new());

    static LIVE_REGION_STATE: RefCell<HashMap<String, RegionAnnouncementState>> =
        RefCell::new(HashMap::new());
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn announce_impl(region_id: &str, message: &str) {
    ensure_dom_impl();

    let Some(browser_window) = web_sys::window() else {
        crate::debug::warn_skipped("announce()", "window");

        return;
    };

    let Some(browser_document) = document() else {
        crate::debug::warn_skipped("announce()", "window.document");

        return;
    };

    let Some(region) = browser_document.get_element_by_id(region_id) else {
        crate::debug::warn_message(format_args!(
            "announce() skipped because live region #{region_id} was not found"
        ));

        return;
    };

    cancel_region_timers(&browser_window, region_id);
    clear_region_children(&region);

    let region_id = String::from(region_id);

    let message = prepare_live_region_message(&region_id, message);

    let insert_region_id = region_id.clone();

    let insert_callback = Closure::wrap(Box::new(move || {
        let Some(browser_document) = document() else {
            return;
        };

        let Some(region) = browser_document.get_element_by_id(&insert_region_id) else {
            return;
        };

        clear_region_children(&region);

        let span = browser_document
            .create_element("span")
            .expect("live region span creation must succeed");

        span.set_text_content(Some(&message));

        region
            .append_child(&span)
            .expect("live region span append must succeed");

        let clear_region_id = insert_region_id.clone();
        let clear_callback = Closure::wrap(Box::new(move || {
            if let Some(document) = document()
                && let Some(region) = document.get_element_by_id(&clear_region_id)
            {
                clear_region_children(&region);
            }

            LIVE_REGION_TIMERS.with(|timers| {
                let mut timers = timers.borrow_mut();

                let mut should_remove = false;

                if let Some(entry) = timers.get_mut(&clear_region_id) {
                    entry.clear_timeout_id = None;

                    entry.clear_callback = None;

                    should_remove =
                        entry.insert_timeout_id.is_none() && entry.insert_callback.is_none();
                }

                if should_remove {
                    timers.remove(&clear_region_id);
                }
            });
        }) as Box<dyn FnMut()>);

        let clear_timeout_id = web_sys::window()
            .expect("window must exist while scheduling live region clear")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                clear_callback.as_ref().unchecked_ref(),
                LIVE_REGION_CLEAR_DELAY_MS,
            )
            .expect("live region clear timeout must succeed");

        LIVE_REGION_TIMERS.with(|timers| {
            if let Some(entry) = timers.borrow_mut().get_mut(&insert_region_id) {
                entry.insert_timeout_id = None;

                entry.insert_callback = None;

                entry.clear_timeout_id = Some(clear_timeout_id);

                entry.clear_callback = Some(clear_callback);
            }
        });
    }) as Box<dyn FnMut()>);

    let insert_timeout_id = browser_window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            insert_callback.as_ref().unchecked_ref(),
            LIVE_REGION_INSERT_DELAY_MS,
        )
        .expect("live region insert timeout must succeed");

    LIVE_REGION_TIMERS.with(|timers| {
        timers.borrow_mut().insert(
            region_id,
            PendingRegionTimers {
                insert_timeout_id: Some(insert_timeout_id),
                insert_callback: Some(insert_callback),
                clear_timeout_id: None,
                clear_callback: None,
            },
        );
    });
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn announce_impl(_region_id: &str, _message: &str) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn prepare_live_region_message(region_id: &str, message: &str) -> String {
    LIVE_REGION_STATE.with(|state_map| {
        let mut state_map = state_map.borrow_mut();

        let state = state_map.entry(String::from(region_id)).or_default();

        let is_repeat = state.last_message.as_deref() == Some(message);

        let content = if is_repeat {
            state.voiceover_toggle = !state.voiceover_toggle;

            if state.voiceover_toggle {
                format!("{message}\u{200D}")
            } else {
                String::from(message)
            }
        } else {
            state.voiceover_toggle = false;

            String::from(message)
        };

        state.last_message = Some(String::from(message));

        content
    })
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn cancel_region_timers(window: &web_sys::Window, region_id: &str) {
    LIVE_REGION_TIMERS.with(|timers| {
        if let Some(mut entry) = timers.borrow_mut().remove(region_id) {
            if let Some(timeout_id) = entry.insert_timeout_id.take() {
                window.clear_timeout_with_handle(timeout_id);
            }

            if let Some(timeout_id) = entry.clear_timeout_id.take() {
                window.clear_timeout_with_handle(timeout_id);
            }

            drop(entry.insert_callback.take());

            drop(entry.clear_callback.take());
        }
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn clear_region_children(region: &web_sys::Element) {
    while let Some(child) = region.first_child() {
        region
            .remove_child(&child)
            .expect("removing live region child must succeed");
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
fn reset_region_announcement_state(region_id: &str) {
    LIVE_REGION_STATE.with(|state_map| {
        state_map.borrow_mut().remove(region_id);
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn ensure_region(document: &web_sys::Document, parent: &web_sys::Element, attrs: &AttrMap) {
    let id = attrs
        .get(&HtmlAttr::Id)
        .expect("live region attrs must include an id");

    let element = if let Some(existing) = document.get_element_by_id(id) {
        existing
    } else {
        let created = document
            .create_element("div")
            .expect("live region creation must succeed");

        parent
            .append_child(&created)
            .expect("live region append must succeed");

        created
    };

    apply_attr_map(&element, attrs);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_attr_map(element: &web_sys::Element, attrs: &AttrMap) {
    for (attr, value) in attrs.attrs() {
        apply_attr_value(element, *attr, value);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_attr_value(element: &web_sys::Element, attr: HtmlAttr, value: &AttrValue) {
    match value {
        AttrValue::String(value) => {
            crate::debug::warn_dom_error(
                &format!("setting live region attribute {attr}"),
                element.set_attribute(&attr.to_string(), value),
            );
        }

        AttrValue::Bool(true) => {
            crate::debug::warn_dom_error(
                &format!("setting live region boolean attribute {attr}"),
                element.set_attribute(&attr.to_string(), ""),
            );
        }

        AttrValue::Bool(false) | AttrValue::None => {
            crate::debug::warn_dom_error(
                &format!("removing live region attribute {attr}"),
                element.remove_attribute(&attr.to_string()),
            );
        }
    }
}

#[cfg(all(test, feature = "web", not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;

    #[test]
    fn ensure_dom_is_host_safe_and_idempotent() {
        ensure_dom();
        ensure_dom();
    }

    #[test]
    fn host_announcements_are_safe_noops() {
        announce_polite("hello");
        announce_assertive("alert");
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{
        cell::{Cell, RefCell},
        pin::Pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        super::document().expect("document must exist")
    }

    fn remove_live_regions() {
        if let Some(window) = web_sys::window() {
            cancel_region_timers(&window, "ars-live-polite");
            cancel_region_timers(&window, "ars-live-assertive");
        }

        reset_region_announcement_state("ars-live-polite");
        reset_region_announcement_state("ars-live-assertive");

        if let Some(region) = document().get_element_by_id("ars-live-polite") {
            region.remove();
        }

        if let Some(region) = document().get_element_by_id("ars-live-assertive") {
            region.remove();
        }
    }

    fn region(id: &str) -> web_sys::Element {
        document()
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("missing region {id}"))
    }

    struct TimeoutFuture {
        ready: Rc<Cell<bool>>,
        waker: Rc<RefCell<Option<Waker>>>,
        _callback: Closure<dyn FnMut()>,
    }

    impl TimeoutFuture {
        fn new(timeout_ms: i32) -> Self {
            let ready = Rc::new(Cell::new(false));

            let waker = Rc::new(RefCell::new(None::<Waker>));
            let callback_ready = Rc::clone(&ready);
            let callback_waker = Rc::clone(&waker);
            let callback = Closure::wrap(Box::new(move || {
                callback_ready.set(true);
                if let Some(waker) = callback_waker.borrow_mut().take() {
                    waker.wake();
                }
            }) as Box<dyn FnMut()>);

            web_sys::window()
                .expect("window must exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    timeout_ms,
                )
                .expect("setTimeout must succeed");

            Self {
                ready,
                waker,
                _callback: callback,
            }
        }
    }

    impl Future for TimeoutFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.ready.get() {
                Poll::Ready(())
            } else {
                self.waker.borrow_mut().replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    #[wasm_bindgen_test]
    fn ensure_dom_creates_both_live_regions_under_body() {
        remove_live_regions();

        ensure_dom();

        let polite = region("ars-live-polite");
        let assertive = region("ars-live-assertive");

        let body = document().body().expect("body must exist");

        assert!(body.contains(Some(polite.as_ref())));
        assert!(body.contains(Some(assertive.as_ref())));

        remove_live_regions();
    }

    #[wasm_bindgen_test]
    fn ensure_dom_creates_regions_with_expected_attributes() {
        remove_live_regions();

        ensure_dom();

        let polite = region("ars-live-polite");
        let assertive = region("ars-live-assertive");

        assert_eq!(polite.get_attribute("aria-live").as_deref(), Some("polite"));
        assert_eq!(polite.get_attribute("aria-atomic").as_deref(), Some("true"));
        assert_eq!(
            polite.get_attribute("data-ars-part").as_deref(),
            Some("live-region")
        );
        assert_eq!(
            polite.get_attribute("class").as_deref(),
            Some("ars-visually-hidden")
        );

        assert_eq!(
            assertive.get_attribute("aria-live").as_deref(),
            Some("assertive")
        );
        assert_eq!(
            assertive.get_attribute("aria-atomic").as_deref(),
            Some("true")
        );
        assert_eq!(
            assertive.get_attribute("aria-relevant").as_deref(),
            Some("additions text")
        );
        assert_eq!(
            assertive.get_attribute("data-ars-part").as_deref(),
            Some("live-region")
        );
        assert_eq!(
            assertive.get_attribute("class").as_deref(),
            Some("ars-visually-hidden")
        );

        remove_live_regions();
    }

    #[wasm_bindgen_test]
    fn ensure_dom_is_idempotent() {
        remove_live_regions();

        ensure_dom();
        ensure_dom();

        let regions = document()
            .query_selector_all("#ars-live-polite, #ars-live-assertive")
            .expect("selector must be valid");

        assert_eq!(regions.length(), 2);

        remove_live_regions();
    }

    #[wasm_bindgen_test(async)]
    async fn announce_polite_writes_the_live_region() {
        remove_live_regions();

        announce_polite("Polite update");

        let polite = region("ars-live-polite");

        assert_eq!(polite.text_content().as_deref(), Some(""));

        TimeoutFuture::new(170).await;

        assert_eq!(polite.text_content().as_deref(), Some("Polite update"));
        assert_eq!(polite.child_nodes().length(), 1);

        remove_live_regions();
    }

    #[wasm_bindgen_test(async)]
    async fn announce_assertive_writes_the_live_region() {
        remove_live_regions();

        announce_assertive("Critical alert");

        let assertive = region("ars-live-assertive");

        assert_eq!(assertive.text_content().as_deref(), Some(""));

        TimeoutFuture::new(170).await;

        assert_eq!(assertive.text_content().as_deref(), Some("Critical alert"));
        assert_eq!(assertive.child_nodes().length(), 1);

        remove_live_regions();
    }

    #[wasm_bindgen_test(async)]
    async fn repeated_announcements_toggle_the_voiceover_suffix() {
        remove_live_regions();

        announce_polite("Repeat");

        TimeoutFuture::new(170).await;

        let polite = region("ars-live-polite");

        assert_eq!(polite.text_content().as_deref(), Some("Repeat"));

        announce_polite("Repeat");

        assert_eq!(polite.text_content().as_deref(), Some(""));

        TimeoutFuture::new(170).await;

        assert_eq!(polite.text_content().as_deref(), Some("Repeat\u{200D}"));
        assert_eq!(polite.child_nodes().length(), 1);

        remove_live_regions();
    }

    #[wasm_bindgen_test(async)]
    async fn rapid_reannounce_cancels_the_previous_insert_timer() {
        remove_live_regions();

        announce_polite("First");

        TimeoutFuture::new(60).await;

        announce_polite("Second");

        let polite = region("ars-live-polite");

        assert_eq!(polite.text_content().as_deref(), Some(""));

        TimeoutFuture::new(105).await;

        assert_eq!(polite.text_content().as_deref(), Some(""));

        TimeoutFuture::new(70).await;

        assert_eq!(polite.text_content().as_deref(), Some("Second"));
        assert_eq!(polite.child_nodes().length(), 1);

        remove_live_regions();
    }
}
