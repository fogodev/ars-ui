//! Browser-backed [`ars_core::PlatformEffects`] implementation.

use std::{string::String, vec::Vec};

use ars_core::{PlatformEffects, Rect, TimerHandle};
use ars_i18n::ResolvedDirection;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    js_sys::{Function, Reflect},
    std::{cell::RefCell, mem, rc::Rc},
    wasm_bindgen::{JsCast, closure::Closure},
    web_sys::{
        AddEventListenerOptions, Document, Element, Event, EventTarget, HtmlElement, KeyboardEvent,
        Window,
    },
};

use crate::{announcer, focus, portal, scroll_lock};

/// Production [`PlatformEffects`] implementation for browser builds.
#[derive(Debug, Default, Clone, Copy)]
pub struct WebPlatformEffects;

impl PlatformEffects for WebPlatformEffects {
    fn focus_element_by_id(&self, id: &str) {
        focus::focus_element_by_id(id);
    }

    fn focus_first_tabbable(&self, container_id: &str) {
        focus::focus_first_tabbable(container_id);
    }

    fn focus_last_tabbable(&self, container_id: &str) {
        focus::focus_last_tabbable_for_platform(container_id);
    }

    fn tabbable_element_ids(&self, container_id: &str) -> Vec<String> {
        focus::tabbable_element_ids_for_platform(container_id)
    }

    fn focus_body(&self) {
        focus::focus_body();
    }

    fn set_timeout(&self, delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        set_timeout_impl(delay_ms, callback)
    }

    fn clear_timeout(&self, handle: TimerHandle) {
        clear_timeout_impl(handle);
    }

    fn announce(&self, message: &str) {
        announcer::announce_polite(message);
    }

    fn announce_assertive(&self, message: &str) {
        announcer::announce_assertive(message);
    }

    fn position_element_at(&self, id: &str, x: f64, y: f64) {
        position_element_at_impl(id, x, y);
    }

    fn resolved_direction(&self, id: &str) -> ResolvedDirection {
        resolved_direction_impl(id)
    }

    fn set_background_inert(&self, portal_root_id: &str) -> Box<dyn FnOnce()> {
        portal::set_background_inert(portal_root_id)
    }

    fn remove_inert_from_siblings(&self, portal_id: &str) {
        portal::remove_inert_from_siblings(portal_id);
    }

    fn scroll_lock_acquire(&self) {
        scroll_lock::acquire();
    }

    fn scroll_lock_release(&self) {
        scroll_lock::release();
    }

    fn document_contains_id(&self, id: &str) -> bool {
        focus::document_contains_id_for_platform(id)
    }

    fn track_pointer_drag(
        &self,
        on_move: Box<dyn Fn(f64, f64)>,
        on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()> {
        track_pointer_drag_impl(on_move, on_up)
    }

    fn active_element_id(&self) -> Option<String> {
        focus::active_element_id_for_platform()
    }

    fn attach_focus_trap(&self, container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()> {
        attach_focus_trap_impl(container_id, on_escape)
    }

    fn can_restore_focus(&self, id: &str) -> bool {
        focus::can_restore_focus_for_platform(id)
    }

    fn nearest_focusable_ancestor_id(&self, id: &str) -> Option<String> {
        focus::nearest_focusable_ancestor_id_for_platform(id)
    }

    fn set_scroll_top(&self, container_id: &str, scroll_top: f64) {
        set_scroll_top_impl(container_id, scroll_top);
    }

    fn resize_to_content(&self, id: &str, max_height: Option<&str>) {
        resize_to_content_impl(id, max_height);
    }

    fn on_reduced_motion_change(&self, callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        on_reduced_motion_change_impl(callback)
    }

    fn is_mac_platform(&self) -> bool {
        is_mac_platform_impl()
    }

    fn now_ms(&self) -> u64 {
        now_ms_impl()
    }

    fn get_bounding_rect(&self, id: &str) -> Option<Rect> {
        get_bounding_rect_impl(id)
    }

    fn on_animation_end(&self, id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
        on_animation_end_impl(id, callback)
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Debug)]
struct EventListener {
    target: EventTarget,
    event_type: &'static str,
    callback: Closure<dyn FnMut(Event)>,
    capture: bool,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
type SharedOneShot = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl EventListener {
    fn remove(self) {
        if self.capture {
            crate::debug::warn_dom_error(
                &format!("removing {} listener", self.event_type),
                self.target.remove_event_listener_with_callback_and_bool(
                    self.event_type,
                    self.callback.as_ref().unchecked_ref(),
                    true,
                ),
            );
        } else {
            crate::debug::warn_dom_error(
                &format!("removing {} listener", self.event_type),
                self.target.remove_event_listener_with_callback(
                    self.event_type,
                    self.callback.as_ref().unchecked_ref(),
                ),
            );
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_event_listener(
    target: &EventTarget,
    event_type: &'static str,
    callback: Closure<dyn FnMut(Event)>,
    capture: bool,
    once: bool,
) -> Option<EventListener> {
    let result = if once {
        let options = AddEventListenerOptions::new();

        options.set_capture(capture);
        options.set_once(true);

        target.add_event_listener_with_callback_and_add_event_listener_options(
            event_type,
            callback.as_ref().unchecked_ref(),
            &options,
        )
    } else if capture {
        target.add_event_listener_with_callback_and_bool(
            event_type,
            callback.as_ref().unchecked_ref(),
            true,
        )
    } else {
        target.add_event_listener_with_callback(event_type, callback.as_ref().unchecked_ref())
    };

    if result.is_err() {
        crate::debug::warn_dom_error(&format!("adding {event_type} listener"), result);

        return None;
    }

    Some(EventListener {
        target: target.clone(),
        event_type,
        callback,
        capture,
    })
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn window() -> Option<Window> {
    web_sys::window()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn performance_now_ms() -> f64 {
    window()
        .and_then(|window| window.performance())
        .map_or_else(js_sys::Date::now, |performance| performance.now())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document() -> Option<Document> {
    window().and_then(|window| window.document())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn document_visibility_state(document: &Document) -> Option<String> {
    Reflect::get(
        document.as_ref(),
        &wasm_bindgen::JsValue::from_str("visibilityState"),
    )
    .ok()
    .and_then(|value| value.as_string())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn event_target_from<T>(target: &T) -> EventTarget
where
    T: AsRef<wasm_bindgen::JsValue>,
{
    target.as_ref().clone().unchecked_into()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn event_targets_element(event: &Event, element: &Element) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<Element>().ok())
        .is_some_and(|target| target.is_same_node(Some(element)))
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn clear_listener_stack(listeners: &Rc<RefCell<Option<Vec<EventListener>>>>) {
    if let Some(listeners) = listeners.borrow_mut().take() {
        for listener in listeners {
            listener.remove();
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_container(container: &Element) {
    if let Ok(container) = container.clone().dyn_into::<HtmlElement>() {
        focus::focus_element(&container, false);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn set_timeout_impl(delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
    let Some(window) = window() else {
        callback();

        return TimerHandle::new(0);
    };

    let callback = Closure::once_into_js(move || {
        callback();
    });

    let id = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.unchecked_ref::<Function>(),
            delay_ms as i32,
        )
        .unwrap_or_default();

    TimerHandle::new(id as u64)
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn set_timeout_impl(_delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
    callback();

    TimerHandle::new(0)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn clear_timeout_impl(handle: TimerHandle) {
    if let Some(window) = window() {
        window.clear_timeout_with_handle(handle.id() as i32);
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn clear_timeout_impl(_handle: TimerHandle) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn position_element_at_impl(id: &str, x: f64, y: f64) {
    let Some(element) = focus::get_html_element_by_id(id) else {
        return;
    };

    let style = element.style();
    let position = style.get_property_value("position").unwrap_or_default();

    if position.is_empty() {
        crate::debug::warn_dom_error(
            "setting inline position",
            style.set_property("position", "absolute"),
        );
    }

    crate::debug::warn_dom_error(
        "setting inline left",
        style.set_property("left", &format!("{x}px")),
    );

    crate::debug::warn_dom_error(
        "setting inline top",
        style.set_property("top", &format!("{y}px")),
    );
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn position_element_at_impl(_id: &str, _x: f64, _y: f64) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn resolved_direction_impl(id: &str) -> ResolvedDirection {
    let Some(window) = window() else {
        return ResolvedDirection::Ltr;
    };

    let Some(element) = focus::get_html_element_by_id(id) else {
        return ResolvedDirection::Ltr;
    };

    let Ok(Some(style)) = window.get_computed_style(&element) else {
        return ResolvedDirection::Ltr;
    };

    match style
        .get_property_value("direction")
        .unwrap_or_default()
        .as_str()
    {
        "rtl" => ResolvedDirection::Rtl,
        _ => ResolvedDirection::Ltr,
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn resolved_direction_impl(_id: &str) -> ResolvedDirection {
    ResolvedDirection::Ltr
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn track_pointer_drag_impl(
    on_move: Box<dyn Fn(f64, f64)>,
    on_up: Box<dyn FnOnce()>,
) -> Box<dyn FnOnce()> {
    let Some(document) = document() else {
        crate::debug::warn_skipped("tracking pointer drag", "document");
        return Box::new(|| {});
    };

    let listeners = Rc::new(RefCell::new(None::<Vec<EventListener>>));

    let on_up: SharedOneShot = Rc::new(RefCell::new(Some(on_up)));

    let move_listener = {
        let on_move = Rc::new(on_move);
        let callback = Closure::wrap(Box::new(move |event: Event| {
            let Some(pointer_event) = event.dyn_ref::<web_sys::PointerEvent>() else {
                return;
            };

            on_move(
                f64::from(pointer_event.client_x()),
                f64::from(pointer_event.client_y()),
            );
        }) as Box<dyn FnMut(Event)>);

        install_event_listener(
            &event_target_from(&document),
            "pointermove",
            callback,
            false,
            false,
        )
    };

    let terminal_listener = |event_type: &'static str,
                             listeners: Rc<RefCell<Option<Vec<EventListener>>>>,
                             on_up: SharedOneShot| {
        let callback = Closure::wrap(Box::new(move |_event: Event| {
            clear_listener_stack(&listeners);

            if let Some(on_up) = on_up.borrow_mut().take() {
                on_up();
            }
        }) as Box<dyn FnMut(Event)>);

        install_event_listener(
            &event_target_from(&document),
            event_type,
            callback,
            false,
            false,
        )
    };

    let pointer_up = terminal_listener("pointerup", Rc::clone(&listeners), Rc::clone(&on_up));
    let pointer_cancel =
        terminal_listener("pointercancel", Rc::clone(&listeners), Rc::clone(&on_up));

    let mut installed = Vec::new();

    if let Some(listener) = move_listener {
        installed.push(listener);
    }

    if let Some(listener) = pointer_up {
        installed.push(listener);
    }

    if let Some(listener) = pointer_cancel {
        installed.push(listener);
    }

    listeners.replace(Some(installed));

    Box::new(move || {
        clear_listener_stack(&listeners);
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn track_pointer_drag_impl(
    _on_move: Box<dyn Fn(f64, f64)>,
    _on_up: Box<dyn FnOnce()>,
) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn attach_focus_trap_impl(container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()> {
    let Some(container) = focus::get_element_by_id(container_id) else {
        crate::debug::warn_message(format_args!(
            "attaching focus trap skipped because container `{container_id}` is unavailable"
        ));

        return Box::new(|| {});
    };

    let Some(document) = document() else {
        crate::debug::warn_skipped("attaching focus trap", "document");

        return Box::new(|| {});
    };

    let listener_container = container.clone();

    let callback = Closure::wrap(Box::new(move |event: Event| {
        let Some(key_event) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };

        if key_event.default_prevented() {
            return;
        }

        match key_event.key().as_str() {
            "Escape" => {
                key_event.prevent_default();

                event.stop_propagation();

                on_escape();
            }

            "Tab" => {
                let tabbables = focus::get_tabbable_elements(&listener_container);

                if tabbables.is_empty() {
                    key_event.prevent_default();
                    focus_container(&listener_container);
                    return;
                }

                let active = document
                    .active_element()
                    .and_then(|element| element.dyn_into::<HtmlElement>().ok());

                let first = tabbables.first().expect("non-empty tabbable list");
                let last = tabbables.last().expect("non-empty tabbable list");

                let active_inside = active
                    .as_ref()
                    .is_some_and(|element| listener_container.contains(Some(element.as_ref())));

                let should_wrap = if key_event.shift_key() {
                    !active_inside
                        || active
                            .as_ref()
                            .is_some_and(|element| element.is_same_node(Some(first.as_ref())))
                } else {
                    !active_inside
                        || active
                            .as_ref()
                            .is_some_and(|element| element.is_same_node(Some(last.as_ref())))
                };

                if should_wrap {
                    key_event.prevent_default();

                    if key_event.shift_key() {
                        focus::focus_element(last, false);
                    } else {
                        focus::focus_element(first, false);
                    }
                }
            }

            _ => {}
        }
    }) as Box<dyn FnMut(Event)>);

    let listener = install_event_listener(
        &event_target_from(&container),
        "keydown",
        callback,
        false,
        false,
    );

    Box::new(move || {
        if let Some(listener) = listener {
            listener.remove();
        }
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn attach_focus_trap_impl(_container_id: &str, _on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn set_scroll_top_impl(container_id: &str, scroll_top: f64) {
    if let Some(element) = focus::get_html_element_by_id(container_id) {
        element.set_scroll_top(scroll_top.round() as i32);
    }
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn set_scroll_top_impl(_container_id: &str, _scroll_top: f64) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn parse_px_value(value: &str) -> Option<f64> {
    value
        .strip_suffix("px")
        .and_then(|value| value.trim().parse::<f64>().ok())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn resize_to_content_impl(id: &str, max_height: Option<&str>) {
    let Some(element) = focus::get_html_element_by_id(id) else {
        return;
    };

    let style = element.style();

    crate::debug::warn_dom_error(
        "resetting textarea height",
        style.set_property("height", "auto"),
    );

    if let Some(max_height) = max_height {
        crate::debug::warn_dom_error(
            "setting textarea max-height",
            style.set_property("max-height", max_height),
        );
    } else {
        crate::debug::warn_dom_error(
            "removing textarea max-height",
            style.remove_property("max-height").map(|_| ()),
        );
    }

    let border_adjustment = f64::from((element.offset_height() - element.client_height()).max(0));

    let measured_height = f64::from(element.scroll_height()) + border_adjustment;

    let mut applied_height = measured_height;

    if let Some(max_height) = max_height.and_then(parse_px_value) {
        applied_height = applied_height.min(max_height);
    }

    crate::debug::warn_dom_error(
        "setting textarea height",
        style.set_property("height", &format!("{applied_height}px")),
    );

    let overflow_needed = (element.scroll_height() - element.client_height()) > 0;
    let overflow = if overflow_needed { "auto" } else { "hidden" };

    crate::debug::warn_dom_error(
        "setting textarea overflow-y",
        style.set_property("overflow-y", overflow),
    );
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn resize_to_content_impl(_id: &str, _max_height: Option<&str>) {}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn on_reduced_motion_change_impl(callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
    let Some(window) = window() else {
        crate::debug::warn_skipped("subscribing to reduced motion changes", "window");

        return Box::new(|| {});
    };

    let Ok(Some(media_query)) = window.match_media("(prefers-reduced-motion: reduce)") else {
        crate::debug::warn_message(format_args!(
            "subscribing to reduced motion changes skipped because media query support is unavailable"
        ));

        return Box::new(|| {});
    };

    let listener_media_query = media_query.clone();

    let callback = Closure::wrap(Box::new(move |_event: Event| {
        callback(listener_media_query.matches());
    }) as Box<dyn FnMut(Event)>);

    let listener = install_event_listener(
        &event_target_from(&media_query),
        "change",
        callback,
        false,
        false,
    );

    Box::new(move || {
        if let Some(listener) = listener {
            listener.remove();
        }
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn on_reduced_motion_change_impl(_callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_mac_platform_impl() -> bool {
    window().is_some_and(|window| {
        window
            .navigator()
            .platform()
            .unwrap_or_default()
            .contains("Mac")
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn is_mac_platform_impl() -> bool {
    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn now_ms_impl() -> u64 {
    performance_now_ms().round() as u64
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn now_ms_impl() -> u64 {
    0
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn get_bounding_rect_impl(id: &str) -> Option<Rect> {
    let element = focus::get_element_by_id(id)?;

    let rect = element.get_bounding_client_rect();

    Some(Rect {
        x: rect.x(),
        y: rect.y(),
        width: rect.width(),
        height: rect.height(),
    })
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn get_bounding_rect_impl(_id: &str) -> Option<Rect> {
    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn parse_time_ms(value: &str) -> Option<f64> {
    let value = value.trim();

    if let Some(value) = value.strip_suffix("ms") {
        return value.trim().parse::<f64>().ok();
    }
    if let Some(value) = value.strip_suffix('s') {
        return value
            .trim()
            .parse::<f64>()
            .ok()
            .map(|seconds| seconds * 1000.0);
    }

    None
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn parse_time_list_ms(value: &str) -> Vec<f64> {
    value
        .split(',')
        .map(|entry| parse_time_ms(entry).unwrap_or(0.0))
        .collect()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn is_zero_time_list(value: &str) -> bool {
    let times = parse_time_list_ms(value);

    !times.is_empty() && times.iter().all(|duration| *duration <= 0.0)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn parse_name_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|entry| entry.trim().to_owned())
        .filter(|entry| !entry.is_empty())
        .collect()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn repeated_index<T>(values: &[T], index: usize) -> Option<usize> {
    (!values.is_empty()).then_some(index % values.len())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn resolve_longest_transition_property(
    properties: &str,
    durations: &str,
    delays: &str,
) -> Option<(Option<String>, i32)> {
    let properties = parse_name_list(properties);

    let durations = parse_time_list_ms(durations);

    let delays = parse_time_list_ms(delays);

    let count = properties.len().max(durations.len()).max(delays.len());

    if count == 0 {
        return None;
    }

    let mut best_property = None;

    let mut best_total_ms = 0.0;

    for index in 0..count {
        let duration = repeated_index(&durations, index)
            .and_then(|index| durations.get(index).copied())
            .unwrap_or(0.0);

        let delay = repeated_index(&delays, index)
            .and_then(|index| delays.get(index).copied())
            .unwrap_or(0.0);

        let total = duration + delay;

        if total > best_total_ms {
            best_total_ms = total;

            best_property = repeated_index(&properties, index)
                .and_then(|index| properties.get(index).cloned())
                .filter(|property| property != "all");
        }
    }

    Some((best_property, best_total_ms.ceil() as i32))
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct AnimationEndState {
    window: Window,
    callback: Option<Box<dyn FnOnce()>>,
    require_animation: bool,
    require_transition: bool,
    animation_done: bool,
    transition_done: bool,
    completed: bool,
    animation_listener: Option<EventListener>,
    transition_listener: Option<EventListener>,
    visibility_listener: Option<EventListener>,
    style_detection_queued: bool,
    transition_timeout_id: Option<i32>,
    transition_timeout_callback: Option<Closure<dyn FnMut()>>,
    fallback_timeout_id: Option<i32>,
    fallback_timeout_callback: Option<Closure<dyn FnMut()>>,
    fallback_timeout_started_at_ms: Option<f64>,
    fallback_timeout_remaining_ms: i32,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
struct PendingStyleDetection {
    state: Rc<RefCell<AnimationEndState>>,
    element: Element,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Default)]
struct StyleDetectionBatch {
    pending: Vec<PendingStyleDetection>,
    deferred: Vec<PendingStyleDetection>,
    first_raf_id: Option<i32>,
    first_raf_callback: Option<Closure<dyn FnMut(f64)>>,
    second_raf_id: Option<i32>,
    second_raf_callback: Option<Closure<dyn FnMut(f64)>>,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
thread_local! {
    static STYLE_DETECTION_BATCH: RefCell<StyleDetectionBatch> =
        RefCell::new(StyleDetectionBatch::default());
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn complete_animation_if_ready(state: &Rc<RefCell<AnimationEndState>>) {
    let ready = {
        let state = state.borrow();

        if state.completed {
            return;
        }

        (!state.require_animation || state.animation_done)
            && (!state.require_transition || state.transition_done)
    };

    if ready {
        finalize_animation_state(state, true);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn finalize_animation_state(state: &Rc<RefCell<AnimationEndState>>, invoke_callback: bool) {
    remove_style_detection_request(state);

    let (
        window,
        animation_listener,
        transition_listener,
        visibility_listener,
        transition_timeout_id,
        transition_timeout_callback,
        fallback_timeout_id,
        fallback_timeout_callback,
        _fallback_timeout_started_at_ms,
        callback,
    ) = {
        let mut state = state.borrow_mut();

        if state.completed {
            return;
        }

        state.completed = true;

        (
            state.window.clone(),
            state.animation_listener.take(),
            state.transition_listener.take(),
            state.visibility_listener.take(),
            state.transition_timeout_id.take(),
            state.transition_timeout_callback.take(),
            state.fallback_timeout_id.take(),
            state.fallback_timeout_callback.take(),
            state.fallback_timeout_started_at_ms.take(),
            invoke_callback.then(|| state.callback.take()).flatten(),
        )
    };

    if let Some(id) = transition_timeout_id {
        window.clear_timeout_with_handle(id);
    }

    drop(transition_timeout_callback);

    if let Some(id) = fallback_timeout_id {
        window.clear_timeout_with_handle(id);
    }

    drop(fallback_timeout_callback);

    if let Some(listener) = animation_listener {
        listener.remove();
    }

    if let Some(listener) = transition_listener {
        listener.remove();
    }

    if let Some(listener) = visibility_listener {
        listener.remove();
    }

    if let Some(callback) = callback {
        callback();
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_timeout(
    state: &Rc<RefCell<AnimationEndState>>,
    delay_ms: i32,
    mut action: impl FnMut() + 'static,
    store: impl Fn(&mut AnimationEndState, i32, Closure<dyn FnMut()>),
) {
    let window = state.borrow().window.clone();

    let callback = Closure::wrap(Box::new(move || {
        action();
    }) as Box<dyn FnMut()>);

    let Ok(id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        delay_ms,
    ) else {
        return;
    };

    store(&mut state.borrow_mut(), id, callback);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn remaining_timeout_ms(total_ms: i32, started_at_ms: f64, now_ms: f64) -> i32 {
    let elapsed_ms = (now_ms - started_at_ms).max(0.0).ceil() as i32;

    total_ms.saturating_sub(elapsed_ms)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_window_animation_frame(
    window: &Window,
    callback: &Closure<dyn FnMut(f64)>,
) -> Option<i32> {
    let Ok(id) = window.request_animation_frame(callback.as_ref().unchecked_ref()) else {
        return None;
    };

    Some(id)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn cancel_style_detection_frames_if_idle(window: &Window) {
    let (first_raf_id, first_raf_callback, second_raf_id, second_raf_callback) =
        STYLE_DETECTION_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();

            if !batch.pending.is_empty() || !batch.deferred.is_empty() {
                return (None, None, None, None);
            }

            (
                batch.first_raf_id.take(),
                batch.first_raf_callback.take(),
                batch.second_raf_id.take(),
                batch.second_raf_callback.take(),
            )
        });

    if let Some(id) = first_raf_id {
        drop(window.cancel_animation_frame(id));
    }

    drop(first_raf_callback);

    if let Some(id) = second_raf_id {
        drop(window.cancel_animation_frame(id));
    }

    drop(second_raf_callback);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn remove_style_detection_request(state: &Rc<RefCell<AnimationEndState>>) {
    let window = state.borrow().window.clone();

    STYLE_DETECTION_BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();

        batch
            .pending
            .retain(|request| !Rc::ptr_eq(&request.state, state));

        batch
            .deferred
            .retain(|request| !Rc::ptr_eq(&request.state, state));
    });

    state.borrow_mut().style_detection_queued = false;

    cancel_style_detection_frames_if_idle(&window);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_first_style_detection_frame(window: &Window) -> bool {
    let callback_window = window.clone();

    let callback = Closure::wrap(Box::new(move |_timestamp: f64| {
        STYLE_DETECTION_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();

            batch.first_raf_id = None;

            batch.first_raf_callback = None;
        });

        if !schedule_second_style_detection_frame(&callback_window) {
            let requests = STYLE_DETECTION_BATCH.with(|batch| {
                let mut batch = batch.borrow_mut();

                batch.first_raf_id = None;
                batch.first_raf_callback = None;

                batch.second_raf_id = None;
                batch.second_raf_callback = None;

                let mut requests = mem::take(&mut batch.pending);

                requests.append(&mut batch.deferred);

                requests
            });

            for request in requests {
                request.state.borrow_mut().style_detection_queued = false;

                finalize_animation_state(&request.state, true);
            }
        }
    }) as Box<dyn FnMut(f64)>);

    let Some(id) = schedule_window_animation_frame(window, &callback) else {
        return false;
    };

    STYLE_DETECTION_BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();

        batch.first_raf_id = Some(id);
        batch.first_raf_callback = Some(callback);
    });

    true
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_second_style_detection_frame(window: &Window) -> bool {
    let callback_window = window.clone();
    let callback = Closure::wrap(Box::new(move |_timestamp: f64| {
        let requests = STYLE_DETECTION_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();

            batch.second_raf_id = None;
            batch.second_raf_callback = None;

            mem::take(&mut batch.pending)
        });

        for request in requests {
            request.state.borrow_mut().style_detection_queued = false;

            run_style_detection(&request.state, &request.element);
        }

        let should_schedule_next = STYLE_DETECTION_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();

            if batch.pending.is_empty() && !batch.deferred.is_empty() {
                batch.pending = mem::take(&mut batch.deferred);

                true
            } else {
                false
            }
        });

        if should_schedule_next && !schedule_first_style_detection_frame(&callback_window) {
            let requests = STYLE_DETECTION_BATCH.with(|batch| {
                let mut batch = batch.borrow_mut();

                batch.first_raf_id = None;
                batch.first_raf_callback = None;

                batch.second_raf_id = None;
                batch.second_raf_callback = None;

                let mut requests = mem::take(&mut batch.pending);

                requests.append(&mut batch.deferred);

                requests
            });

            for request in requests {
                request.state.borrow_mut().style_detection_queued = false;

                finalize_animation_state(&request.state, true);
            }
        }
    }) as Box<dyn FnMut(f64)>);

    let Some(id) = schedule_window_animation_frame(window, &callback) else {
        return false;
    };

    STYLE_DETECTION_BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();

        batch.second_raf_id = Some(id);
        batch.second_raf_callback = Some(callback);
    });

    true
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_fallback_timeout(state: &Rc<RefCell<AnimationEndState>>) {
    let remaining_ms = {
        let state = state.borrow();

        if state.completed || state.fallback_timeout_id.is_some() {
            return;
        }

        state.fallback_timeout_remaining_ms
    };

    let state_for_fallback = Rc::clone(state);
    schedule_timeout(
        state,
        remaining_ms,
        move || finalize_animation_state(&state_for_fallback, true),
        |state, id, callback| {
            state.fallback_timeout_id = Some(id);
            state.fallback_timeout_callback = Some(callback);
            state.fallback_timeout_started_at_ms = Some(performance_now_ms());
        },
    );
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn pause_fallback_timeout(state: &Rc<RefCell<AnimationEndState>>) {
    let (window, timeout_id, timeout_callback, started_at_ms, total_ms) = {
        let mut state = state.borrow_mut();

        (
            state.window.clone(),
            state.fallback_timeout_id.take(),
            state.fallback_timeout_callback.take(),
            state.fallback_timeout_started_at_ms.take(),
            state.fallback_timeout_remaining_ms,
        )
    };

    let Some(timeout_id) = timeout_id else {
        return;
    };

    window.clear_timeout_with_handle(timeout_id);

    drop(timeout_callback);

    if let Some(started_at_ms) = started_at_ms {
        state.borrow_mut().fallback_timeout_remaining_ms =
            remaining_timeout_ms(total_ms, started_at_ms, performance_now_ms());
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn schedule_style_detection(state: &Rc<RefCell<AnimationEndState>>, element: &Element) {
    if state.borrow().completed || state.borrow().style_detection_queued {
        return;
    }

    state.borrow_mut().style_detection_queued = true;

    let window = state.borrow().window.clone();

    STYLE_DETECTION_BATCH.with(|batch| {
        let mut batch = batch.borrow_mut();

        let request = PendingStyleDetection {
            state: Rc::clone(state),
            element: element.clone(),
        };

        if batch.second_raf_id.is_some() {
            batch.deferred.push(request);
        } else {
            batch.pending.push(request);
        }
    });

    let batch_active = STYLE_DETECTION_BATCH.with(|batch| {
        let batch = batch.borrow();

        batch.first_raf_id.is_some() || batch.second_raf_id.is_some()
    });

    if !batch_active && !schedule_first_style_detection_frame(&window) {
        remove_style_detection_request(state);

        finalize_animation_state(state, true);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn run_style_detection(state: &Rc<RefCell<AnimationEndState>>, element: &Element) {
    if state.borrow().completed {
        return;
    }

    if !element.is_connected() {
        return;
    }

    let Some(browser_window) = window() else {
        finalize_animation_state(state, true);

        return;
    };

    let Ok(Some(style)) = browser_window.get_computed_style(element) else {
        finalize_animation_state(state, true);

        return;
    };

    let animation_name = style
        .get_property_value("animation-name")
        .unwrap_or_default();

    let animation_duration = style
        .get_property_value("animation-duration")
        .unwrap_or_default();

    let transition_duration = style
        .get_property_value("transition-duration")
        .unwrap_or_default();

    let reduced_motion = browser_window
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches());

    let has_animation = !reduced_motion
        && !animation_name.trim().is_empty()
        && animation_name != "none"
        && !is_zero_time_list(&animation_duration);

    let has_transition = !reduced_motion && !is_zero_time_list(&transition_duration);

    if reduced_motion || (!has_animation && !has_transition) {
        finalize_animation_state(state, true);

        return;
    }

    {
        let mut state = state.borrow_mut();

        state.require_animation = has_animation;
        state.require_transition = has_transition;

        state.animation_done = !has_animation;

        state.transition_done = !has_transition;
    }

    if !has_animation {
        if let Some(listener) = state.borrow_mut().animation_listener.take() {
            listener.remove();
        }
    } else if state.borrow().animation_listener.is_none() {
        let animation_state = Rc::clone(state);
        let animation_element = element.clone();
        let callback = Closure::wrap(Box::new(move |event: Event| {
            let is_target = event_targets_element(&event, &animation_element);

            if !is_target {
                return;
            }

            event.stop_propagation();

            animation_state.borrow_mut().animation_done = true;

            complete_animation_if_ready(&animation_state);
        }) as Box<dyn FnMut(Event)>);

        state.borrow_mut().animation_listener = install_event_listener(
            &event_target_from(element),
            "animationend",
            callback,
            false,
            false,
        );
    }

    if has_transition {
        let transition_delay = style
            .get_property_value("transition-delay")
            .unwrap_or_default();

        let transition_property = style
            .get_property_value("transition-property")
            .unwrap_or_default();

        let (property_filter, timeout_ms) = resolve_longest_transition_property(
            &transition_property,
            &transition_duration,
            &transition_delay,
        )
        .unwrap_or((None, 10));

        let listener_deadline_ms = performance_now_ms() + f64::from(timeout_ms.saturating_sub(10));

        let required_elapsed_ms = f64::from(timeout_ms.saturating_sub(10));

        if state.borrow().transition_listener.is_none() {
            let transition_state = Rc::clone(state);
            let transition_element = element.clone();
            let callback = Closure::wrap(Box::new(move |event: Event| {
                let is_target = event_targets_element(&event, &transition_element);

                if !is_target {
                    return;
                }

                let property_name = Reflect::get(
                    event.as_ref(),
                    &wasm_bindgen::JsValue::from_str("propertyName"),
                )
                .ok()
                .and_then(|value| value.as_string());

                if let Some(property_filter) = property_filter.as_deref() {
                    if property_name.as_deref() != Some(property_filter) {
                        return;
                    }
                } else {
                    let elapsed_ms = Reflect::get(
                        event.as_ref(),
                        &wasm_bindgen::JsValue::from_str("elapsedTime"),
                    )
                    .ok()
                    .and_then(|value| value.as_f64())
                    .map(|seconds| seconds * 1000.0);

                    if let Some(elapsed_ms) = elapsed_ms {
                        if elapsed_ms + 1.0 < required_elapsed_ms {
                            return;
                        }
                    } else if performance_now_ms() + 1.0 < listener_deadline_ms {
                        return;
                    }
                }

                event.stop_propagation();

                transition_state.borrow_mut().transition_done = true;

                complete_animation_if_ready(&transition_state);
            }) as Box<dyn FnMut(Event)>);

            state.borrow_mut().transition_listener = install_event_listener(
                &event_target_from(element),
                "transitionend",
                callback,
                false,
                false,
            );
        }

        if state.borrow().transition_timeout_id.is_none() {
            let timeout_state = Rc::clone(state);

            schedule_timeout(
                state,
                timeout_ms.saturating_add(10),
                move || {
                    timeout_state.borrow_mut().transition_done = true;
                    complete_animation_if_ready(&timeout_state);
                },
                |state, id, callback| {
                    state.transition_timeout_id = Some(id);
                    state.transition_timeout_callback = Some(callback);
                },
            );
        }
    } else {
        let (window, timeout_id, timeout_callback, listener) = {
            let mut state = state.borrow_mut();

            (
                state.window.clone(),
                state.transition_timeout_id.take(),
                state.transition_timeout_callback.take(),
                state.transition_listener.take(),
            )
        };

        if let Some(timeout_id) = timeout_id {
            window.clear_timeout_with_handle(timeout_id);
        }

        drop(timeout_callback);

        if let Some(listener) = listener {
            listener.remove();
        }
    }

    complete_animation_if_ready(state);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn on_animation_end_impl(id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
    let Some(browser_window) = window() else {
        crate::debug::warn_skipped("registering animation end listener", "window");

        callback();

        return Box::new(|| {});
    };
    let Some(browser_document) = document() else {
        crate::debug::warn_skipped("registering animation end listener", "document");

        callback();

        return Box::new(|| {});
    };
    let Some(element) = focus::get_element_by_id(id) else {
        crate::debug::warn_message(format_args!(
            "registering animation end listener skipped because element `{id}` is unavailable"
        ));

        callback();

        return Box::new(|| {});
    };

    let state = Rc::new(RefCell::new(AnimationEndState {
        window: browser_window,
        callback: Some(callback),
        require_animation: false,
        require_transition: false,
        animation_done: true,
        transition_done: true,
        completed: false,
        animation_listener: None,
        transition_listener: None,
        visibility_listener: None,
        style_detection_queued: false,
        transition_timeout_id: None,
        transition_timeout_callback: None,
        fallback_timeout_id: None,
        fallback_timeout_callback: None,
        fallback_timeout_started_at_ms: None,
        fallback_timeout_remaining_ms: 5_000,
    }));

    schedule_fallback_timeout(&state);

    let visibility_state = Rc::clone(&state);
    let visibility_element = element.clone();
    let visibility_document = browser_document.clone();
    let visibility_callback = Closure::wrap(Box::new(move |_event: Event| {
        if visibility_state.borrow().completed {
            return;
        }

        match document_visibility_state(&visibility_document).as_deref() {
            Some("hidden") => pause_fallback_timeout(&visibility_state),

            Some("visible") => {
                schedule_fallback_timeout(&visibility_state);

                if visibility_element.is_connected() {
                    schedule_style_detection(&visibility_state, &visibility_element);
                }
            }

            _ => {}
        }
    }) as Box<dyn FnMut(Event)>);

    state.borrow_mut().visibility_listener = install_event_listener(
        &event_target_from(&browser_document),
        "visibilitychange",
        visibility_callback,
        false,
        false,
    );

    if matches!(
        document_visibility_state(&browser_document).as_deref(),
        Some("hidden")
    ) {
        pause_fallback_timeout(&state);
    }

    schedule_style_detection(&state, &element);

    Box::new(move || finalize_animation_state(&state, false))
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
fn on_animation_end_impl(_id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
    callback();

    Box::new(|| {})
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    use std::{cell::Cell, rc::Rc};

    use super::*;

    fn assert_platform_effects<T: PlatformEffects>() {}

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn web_platform_effects_implements_platform_effects() {
        assert_platform_effects::<WebPlatformEffects>();
    }

    #[test]
    fn web_platform_effects_is_send_and_sync() {
        assert_send_sync::<WebPlatformEffects>();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn host_behavior_is_safe() {
        let platform = WebPlatformEffects;

        let fired = Rc::new(Cell::new(false));
        let fired_clone = Rc::clone(&fired);

        let handle = platform.set_timeout(10, Box::new(move || fired_clone.set(true)));

        assert_eq!(handle, TimerHandle::new(0));
        assert!(fired.get());

        platform.clear_timeout(handle);

        let cleanup = platform.track_pointer_drag(Box::new(|_, _| {}), Box::new(|| {}));

        cleanup();

        let cleanup = platform.attach_focus_trap("missing", Box::new(|| {}));

        cleanup();

        let cleanup = platform.on_reduced_motion_change(Box::new(|_| {}));

        cleanup();

        let cleanup = platform.on_animation_end("missing", Box::new(|| {}));

        cleanup();
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn css_time_parser_handles_seconds_and_milliseconds() {
        assert_eq!(parse_time_ms("0.25s"), Some(250.0));
        assert_eq!(parse_time_ms("125ms"), Some(125.0));
        assert_eq!(parse_time_ms("bogus"), None);
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn longest_transition_property_uses_duration_plus_delay() {
        assert_eq!(
            resolve_longest_transition_property("opacity, transform", "100ms, 0.4s", "0ms, 50ms"),
            Some((Some(String::from("transform")), 450))
        );
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn longest_transition_property_falls_back_to_timed_filter_for_all() {
        assert_eq!(
            resolve_longest_transition_property("all", "100ms, 0.4s", "0ms, 50ms"),
            Some((None, 450))
        );
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn longest_transition_property_returns_none_for_empty_inputs() {
        assert_eq!(resolve_longest_transition_property("", "", ""), None);
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn remaining_timeout_clamps_elapsed_time_to_zero() {
        assert_eq!(remaining_timeout_ms(5_000, 200.0, 150.0), 5_000);
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    #[test]
    fn remaining_timeout_saturates_at_zero() {
        assert_eq!(remaining_timeout_ms(250, 100.0, 900.0), 0);
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

    use js_sys::{Function, Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Event, KeyboardEvent, KeyboardEventInit, PointerEvent, PointerEventInit};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &Element, id: &str, style: &str) -> HtmlElement {
        let element = document()
            .create_element("div")
            .expect("div creation must succeed")
            .dyn_into::<HtmlElement>()
            .expect("div must be HtmlElement");

        element.set_id(id);

        element
            .set_attribute("style", style)
            .expect("style assignment must succeed");

        parent
            .append_child(&element)
            .expect("append_child must succeed");

        element
    }

    fn append_button(parent: &Element, id: &str, tabindex: Option<&str>) -> HtmlElement {
        let button = document()
            .create_element("button")
            .expect("button creation must succeed")
            .dyn_into::<HtmlElement>()
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

    fn remove_node(node: &HtmlElement) {
        node.remove();
    }

    fn active_element_id() -> Option<String> {
        document().active_element().map(|element| element.id())
    }

    fn assert_reflect_set_succeeds(result: &Result<bool, JsValue>, context: &str) {
        assert!(matches!(result, Ok(true)), "{context}: {result:?}");
    }

    fn define_property_getter<T>(target: &T, property: &str, getter: &js_sys::Function)
    where
        T: AsRef<JsValue>,
    {
        let descriptor = Object::new();

        let object: &Object = target.as_ref().unchecked_ref();

        let getter_result = Reflect::set(&descriptor, &JsValue::from_str("get"), getter.as_ref());

        assert_reflect_set_succeeds(&getter_result, "defining property getter must succeed");

        let configurable_result = Reflect::set(
            &descriptor,
            &JsValue::from_str("configurable"),
            &JsValue::TRUE,
        );

        assert_reflect_set_succeeds(
            &configurable_result,
            "marking property getter configurable must succeed",
        );

        Object::define_property(object, &JsValue::from_str(property), &descriptor);
    }

    fn dispatch_keyboard_event(target: &HtmlElement, key: &str, shift_key: bool) {
        let init = KeyboardEventInit::new();

        init.set_key(key);

        init.set_shift_key(shift_key);

        let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("keyboard event creation must succeed");

        target
            .dispatch_event(&event)
            .expect("dispatching keyboard event must succeed");
    }

    fn dispatch_transitionend(target: &HtmlElement, property_name: &str) {
        dispatch_transitionend_with_elapsed(target, property_name, 0.0);
    }

    fn dispatch_transitionend_with_elapsed(
        target: &HtmlElement,
        property_name: &str,
        elapsed_seconds: f64,
    ) {
        let event = Event::new("transitionend").expect("transition event creation must succeed");

        let property_set_result = Reflect::set(
            event.as_ref(),
            &JsValue::from_str("propertyName"),
            &JsValue::from_str(property_name),
        );

        assert_reflect_set_succeeds(
            &property_set_result,
            "setting transition propertyName must succeed",
        );

        let elapsed_set_result = Reflect::set(
            event.as_ref(),
            &JsValue::from_str("elapsedTime"),
            &JsValue::from_f64(elapsed_seconds),
        );

        assert_reflect_set_succeeds(
            &elapsed_set_result,
            "setting transition elapsedTime must succeed",
        );

        target
            .dispatch_event(&event)
            .expect("dispatching transitionend must succeed");
    }

    struct StubbedElementMeasurements {
        scroll_height: Rc<Cell<i32>>,
        client_height: Rc<Cell<i32>>,
        offset_height: Rc<Cell<i32>>,
        _scroll_height_getter: Closure<dyn FnMut() -> JsValue>,
        _client_height_getter: Closure<dyn FnMut() -> JsValue>,
        _offset_height_getter: Closure<dyn FnMut() -> JsValue>,
    }

    impl StubbedElementMeasurements {
        fn install(
            element: &HtmlElement,
            scroll_height: i32,
            client_height: i32,
            offset_height: i32,
        ) -> Self {
            let scroll_height_cell = Rc::new(Cell::new(scroll_height));

            let client_height_cell = Rc::new(Cell::new(client_height));

            let offset_height_cell = Rc::new(Cell::new(offset_height));

            let scroll_height_state = Rc::clone(&scroll_height_cell);

            let scroll_height_getter = Closure::wrap(Box::new(move || {
                JsValue::from_f64(f64::from(scroll_height_state.get()))
            }) as Box<dyn FnMut() -> JsValue>);

            define_property_getter(
                element,
                "scrollHeight",
                scroll_height_getter.as_ref().unchecked_ref(),
            );

            let client_height_state = Rc::clone(&client_height_cell);

            let client_height_getter = Closure::wrap(Box::new(move || {
                JsValue::from_f64(f64::from(client_height_state.get()))
            }) as Box<dyn FnMut() -> JsValue>);

            define_property_getter(
                element,
                "clientHeight",
                client_height_getter.as_ref().unchecked_ref(),
            );

            let offset_height_state = Rc::clone(&offset_height_cell);

            let offset_height_getter = Closure::wrap(Box::new(move || {
                JsValue::from_f64(f64::from(offset_height_state.get()))
            }) as Box<dyn FnMut() -> JsValue>);

            define_property_getter(
                element,
                "offsetHeight",
                offset_height_getter.as_ref().unchecked_ref(),
            );

            Self {
                scroll_height: scroll_height_cell,
                client_height: client_height_cell,
                offset_height: offset_height_cell,
                _scroll_height_getter: scroll_height_getter,
                _client_height_getter: client_height_getter,
                _offset_height_getter: offset_height_getter,
            }
        }

        fn set(&self, scroll_height: i32, client_height: i32, offset_height: i32) {
            self.scroll_height.set(scroll_height);
            self.client_height.set(client_height);
            self.offset_height.set(offset_height);
        }
    }

    struct MatchMediaGuard {
        window: Window,
        original: JsValue,
        stub: Closure<dyn FnMut(JsValue) -> JsValue>,
    }

    impl Drop for MatchMediaGuard {
        fn drop(&mut self) {
            let result = Reflect::set(
                self.window.as_ref(),
                &JsValue::from_str("matchMedia"),
                &self.original,
            );

            assert_reflect_set_succeeds(&result, "restoring window.matchMedia must succeed");

            let _ = &self.stub;
        }
    }

    struct NavigatorPlatformGuard {
        navigator: web_sys::Navigator,
        original: JsValue,
        getter: Closure<dyn FnMut() -> JsValue>,
    }

    impl NavigatorPlatformGuard {
        fn install(platform: &'static str) -> Self {
            let navigator = window().expect("browser window must exist").navigator();

            let original = Reflect::get(navigator.as_ref(), &JsValue::from_str("platform"))
                .expect("navigator.platform must be readable");

            let getter = Closure::wrap(
                Box::new(move || JsValue::from_str(platform)) as Box<dyn FnMut() -> JsValue>
            );

            define_property_getter(&navigator, "platform", getter.as_ref().unchecked_ref());

            Self {
                navigator,
                original,
                getter,
            }
        }
    }

    impl Drop for NavigatorPlatformGuard {
        fn drop(&mut self) {
            let original = self.original.clone();

            let restore_getter =
                Closure::wrap(Box::new(move || original.clone()) as Box<dyn FnMut() -> JsValue>);

            define_property_getter(
                &self.navigator,
                "platform",
                restore_getter.as_ref().unchecked_ref(),
            );

            restore_getter.forget();

            let _ = &self.getter;
        }
    }

    struct StubbedMediaQuery {
        _guard: MatchMediaGuard,
        matches: Rc<Cell<bool>>,
        listeners: Rc<RefCell<Vec<Function>>>,
        _add_listener: Closure<dyn FnMut(JsValue, JsValue, JsValue)>,
        _remove_listener: Closure<dyn FnMut(JsValue, JsValue, JsValue)>,
    }

    impl StubbedMediaQuery {
        fn install(query: &'static str, initial: bool) -> Self {
            let window = window().expect("browser window must exist");

            let original = Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
                .expect("window.matchMedia must be readable");

            let matches = Rc::new(Cell::new(initial));

            let matches_state = Rc::clone(&matches);

            let listeners = Rc::new(RefCell::new(Vec::<Function>::new()));

            let media_query_list = Object::new();

            let matches_getter =
                Closure::wrap(Box::new(move || JsValue::from_bool(matches_state.get()))
                    as Box<dyn FnMut() -> JsValue>);

            define_property_getter(
                &media_query_list,
                "matches",
                matches_getter.as_ref().unchecked_ref(),
            );

            let listener_state = Rc::clone(&listeners);
            let add_listener = Closure::wrap(Box::new(
                move |event_type: JsValue, listener: JsValue, _options: JsValue| {
                    if event_type.as_string().as_deref() != Some("change") {
                        return;
                    }

                    if let Ok(listener) = listener.dyn_into::<Function>() {
                        listener_state.borrow_mut().push(listener);
                    }
                },
            )
                as Box<dyn FnMut(JsValue, JsValue, JsValue)>);
            let add_listener_result = Reflect::set(
                media_query_list.as_ref(),
                &JsValue::from_str("addEventListener"),
                add_listener.as_ref().unchecked_ref(),
            );

            assert_reflect_set_succeeds(
                &add_listener_result,
                "installing addEventListener must succeed",
            );

            let remove_listener_state = Rc::clone(&listeners);
            let remove_listener = Closure::wrap(Box::new(
                move |event_type: JsValue, listener: JsValue, _options: JsValue| {
                    if event_type.as_string().as_deref() != Some("change") {
                        return;
                    }

                    remove_listener_state
                        .borrow_mut()
                        .retain(|registered| !JsValue::from(registered.clone()).eq(&listener));
                },
            )
                as Box<dyn FnMut(JsValue, JsValue, JsValue)>);

            let remove_listener_result = Reflect::set(
                media_query_list.as_ref(),
                &JsValue::from_str("removeEventListener"),
                remove_listener.as_ref().unchecked_ref(),
            );

            assert_reflect_set_succeeds(
                &remove_listener_result,
                "installing removeEventListener must succeed",
            );

            matches_getter.forget();

            let target_query = query.to_owned();
            let query_object = media_query_list.clone();
            let stub = Closure::wrap(Box::new(move |requested_query: JsValue| -> JsValue {
                let Some(requested_query) = requested_query.as_string() else {
                    return JsValue::NULL;
                };

                if requested_query == target_query {
                    query_object.clone().into()
                } else {
                    JsValue::NULL
                }
            }) as Box<dyn FnMut(JsValue) -> JsValue>);

            let install_result = Reflect::set(
                window.as_ref(),
                &JsValue::from_str("matchMedia"),
                stub.as_ref().unchecked_ref(),
            );

            assert_reflect_set_succeeds(
                &install_result,
                "installing window.matchMedia must succeed",
            );

            Self {
                _guard: MatchMediaGuard {
                    window,
                    original,
                    stub,
                },
                matches,
                listeners,
                _add_listener: add_listener,
                _remove_listener: remove_listener,
            }
        }

        fn set_matches(&self, value: bool) {
            self.matches.set(value);
        }

        fn dispatch_change(&self) {
            let event = Event::new("change").expect("change event creation must succeed");

            for listener in self.listeners.borrow().iter() {
                listener
                    .call1(&JsValue::NULL, event.as_ref())
                    .expect("change listener invocation must succeed");
            }
        }
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

            window()
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
    fn tabbable_ids_follow_sequential_tab_order() {
        let root = append_div(
            body().as_ref(),
            "platform-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let first = append_button(root.as_ref(), "first-button", None);
        let second = append_button(root.as_ref(), "second-button", Some("2"));

        let platform = WebPlatformEffects;

        assert_eq!(
            platform.tabbable_element_ids("platform-root"),
            vec![second.id(), first.id()]
        );
        assert!(platform.can_restore_focus("first-button"));

        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn now_ms_is_monotonic() {
        let platform = WebPlatformEffects;

        let first = platform.now_ms();
        let second = platform.now_ms();

        assert!(first > 0);
        assert!(second >= first);
    }

    #[wasm_bindgen_test]
    async fn timeout_handles_fire_and_can_be_cleared() {
        let platform = WebPlatformEffects;

        let fired = Rc::new(Cell::new(0));

        let fire_count = Rc::clone(&fired);
        let handle = platform.set_timeout(
            10,
            Box::new(move || {
                fire_count.set(fire_count.get() + 1);
            }),
        );

        TimeoutFuture::new(30).await;
        assert_eq!(fired.get(), 1);

        let cleared_count = Rc::clone(&fired);
        let cleared_handle = platform.set_timeout(
            25,
            Box::new(move || {
                cleared_count.set(cleared_count.get() + 1);
            }),
        );

        platform.clear_timeout(cleared_handle);

        TimeoutFuture::new(45).await;

        assert_eq!(fired.get(), 1);

        platform.clear_timeout(handle);
    }

    #[wasm_bindgen_test]
    fn mac_platform_detection_uses_navigator_platform() {
        let platform = WebPlatformEffects;

        let mac_guard = NavigatorPlatformGuard::install("MacIntel");

        assert!(platform.is_mac_platform());

        drop(mac_guard);

        let windows_guard = NavigatorPlatformGuard::install("Win32");

        assert!(!platform.is_mac_platform());

        drop(windows_guard);
    }

    #[wasm_bindgen_test]
    fn position_and_geometry_methods_round_trip() {
        let root = append_div(
            body().as_ref(),
            "geometry-root",
            "position:absolute;left:10px;top:20px;width:80px;height:30px;",
        );

        let platform = WebPlatformEffects;

        platform.position_element_at("geometry-root", 50.0, 75.0);

        assert_eq!(
            root.style().get_property_value("left").unwrap_or_default(),
            "50px"
        );
        assert_eq!(
            root.style().get_property_value("top").unwrap_or_default(),
            "75px"
        );
        assert!(platform.get_bounding_rect("geometry-root").is_some());

        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn listener_helpers_cover_once_capture_and_error_paths() {
        let body = body();

        let target = event_target_from(&body);

        let capture_callback =
            Closure::wrap(Box::new(move |_event: Event| {}) as Box<dyn FnMut(Event)>);

        let capture_listener =
            install_event_listener(&target, "click", capture_callback, true, false)
                .expect("capture listener installation must succeed");

        capture_listener.remove();

        let once_callback =
            Closure::wrap(Box::new(move |_event: Event| {}) as Box<dyn FnMut(Event)>);

        let once_listener = install_event_listener(&target, "click", once_callback, false, true)
            .expect("once listener installation must succeed");

        once_listener.remove();

        let fake_target: EventTarget = Object::new().unchecked_into();

        let failing_callback =
            Closure::wrap(Box::new(move |_event: Event| {}) as Box<dyn FnMut(Event)>);

        assert!(
            install_event_listener(&fake_target, "click", failing_callback, false, false).is_none()
        );
    }

    #[wasm_bindgen_test]
    fn direction_and_missing_target_wrappers_are_safe() {
        let root = append_div(
            body().as_ref(),
            "wrapper-root",
            "position:relative;direction:rtl;left:0;top:0;width:40px;height:20px;",
        );

        let platform = WebPlatformEffects;

        let immediate = Rc::new(Cell::new(0));
        let immediate_count = Rc::clone(&immediate);

        platform.position_element_at("wrapper-root", 12.0, 18.0);

        assert_eq!(
            root.style()
                .get_property_value("position")
                .unwrap_or_default(),
            "relative"
        );
        assert_eq!(
            platform.resolved_direction("wrapper-root"),
            ResolvedDirection::Rtl
        );
        assert_eq!(
            platform.resolved_direction("missing-direction"),
            ResolvedDirection::Ltr
        );

        platform.set_scroll_top("missing-scroll", 22.0);

        platform.resize_to_content("missing-resize", Some("75%"));

        assert_eq!(platform.get_bounding_rect("missing-rect"), None);

        let cleanup = platform.attach_focus_trap("missing-trap", Box::new(|| {}));

        cleanup();

        let cleanup = platform.on_animation_end(
            "missing-animation-target",
            Box::new(move || immediate_count.set(immediate_count.get() + 1)),
        );

        assert_eq!(immediate.get(), 1);

        cleanup();

        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn focus_wrappers_round_trip_through_platform_effects() {
        let root = append_div(
            body().as_ref(),
            "focus-platform-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        root.set_attribute("tabindex", "-1")
            .expect("tabindex assignment must succeed");

        let first = append_button(root.as_ref(), "focus-platform-first", None);
        let last = append_button(root.as_ref(), "focus-platform-last", None);
        let child = append_div(root.as_ref(), "focus-platform-child", "");

        let platform = WebPlatformEffects;

        assert!(platform.document_contains_id("focus-platform-root"));
        assert!(!platform.document_contains_id("missing-platform-node"));

        platform.focus_element_by_id("focus-platform-last");

        assert_eq!(
            platform.active_element_id(),
            Some(String::from("focus-platform-last"))
        );

        platform.focus_first_tabbable("focus-platform-root");

        assert_eq!(active_element_id(), Some(first.id()));

        platform.focus_last_tabbable("focus-platform-root");

        assert_eq!(active_element_id(), Some(last.id()));

        assert_eq!(
            platform.nearest_focusable_ancestor_id("focus-platform-child"),
            Some(root.id())
        );

        platform.focus_body();

        remove_node(&child);
        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn pointer_drag_tracks_moves_and_cleanup() {
        let platform = WebPlatformEffects;

        let move_count = Rc::new(Cell::new(0));

        let last_x = Rc::new(Cell::new(0.0));
        let last_y = Rc::new(Cell::new(0.0));

        let up_count = Rc::new(Cell::new(0));

        let move_counter = Rc::clone(&move_count);

        let move_x = Rc::clone(&last_x);
        let move_y = Rc::clone(&last_y);

        let up_counter = Rc::clone(&up_count);

        let cleanup = platform.track_pointer_drag(
            Box::new(move |x, y| {
                move_counter.set(move_counter.get() + 1);
                move_x.set(x);
                move_y.set(y);
            }),
            Box::new(move || up_counter.set(up_counter.get() + 1)),
        );

        let init = PointerEventInit::new();

        init.set_client_x(12);
        init.set_client_y(34);

        let move_event = PointerEvent::new_with_event_init_dict("pointermove", init.as_ref())
            .expect("pointer move event creation must succeed");

        document()
            .dispatch_event(&move_event)
            .expect("dispatching pointermove must succeed");

        assert_eq!(move_count.get(), 1);
        assert_eq!(last_x.get(), 12.0);
        assert_eq!(last_y.get(), 34.0);

        let up_event = PointerEvent::new("pointerup").expect("pointerup creation must succeed");

        document()
            .dispatch_event(&up_event)
            .expect("dispatching pointerup must succeed");

        document()
            .dispatch_event(&up_event)
            .expect("dispatching second pointerup must succeed");

        assert_eq!(up_count.get(), 1);

        cleanup();

        let suppressed_up_count = Rc::new(Cell::new(0));
        let suppressed_counter = Rc::clone(&suppressed_up_count);
        let suppressed_cleanup = platform.track_pointer_drag(
            Box::new(|_, _| {}),
            Box::new(move || suppressed_counter.set(suppressed_counter.get() + 1)),
        );

        suppressed_cleanup();

        let cancel_event =
            PointerEvent::new("pointercancel").expect("pointercancel creation must succeed");

        document()
            .dispatch_event(&cancel_event)
            .expect("dispatching pointercancel must succeed");

        assert_eq!(suppressed_up_count.get(), 0);
    }

    #[wasm_bindgen_test]
    fn focus_trap_wraps_tab_navigation_and_handles_empty_scopes() {
        let root = append_div(
            body().as_ref(),
            "trap-wrap-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        root.set_attribute("tabindex", "-1")
            .expect("tabindex assignment must succeed");

        let first = append_button(root.as_ref(), "trap-wrap-first", None);
        let last = append_button(root.as_ref(), "trap-wrap-last", None);

        let platform = WebPlatformEffects;

        let cleanup = platform.attach_focus_trap("trap-wrap-root", Box::new(|| {}));

        focus::focus_element(&last, false);

        dispatch_keyboard_event(&root, "Tab", false);

        assert_eq!(active_element_id(), Some(first.id()));

        focus::focus_element(&first, false);

        dispatch_keyboard_event(&root, "Tab", true);

        assert_eq!(active_element_id(), Some(last.id()));

        cleanup();

        remove_node(&root);

        let empty_root = append_div(
            body().as_ref(),
            "trap-empty-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        empty_root
            .set_attribute("tabindex", "-1")
            .expect("tabindex assignment must succeed");

        let empty_cleanup = platform.attach_focus_trap("trap-empty-root", Box::new(|| {}));

        platform.focus_body();

        dispatch_keyboard_event(&empty_root, "Tab", false);

        assert_eq!(active_element_id(), Some(empty_root.id()));

        empty_cleanup();

        remove_node(&empty_root);
    }

    #[wasm_bindgen_test]
    fn focus_trap_ignores_non_keyboard_events_and_inner_tab_keeps_default() {
        let root = append_div(
            body().as_ref(),
            "trap-middle-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        root.set_attribute("tabindex", "-1")
            .expect("tabindex assignment must succeed");

        let first = append_button(root.as_ref(), "trap-middle-first", None);
        let middle = append_button(root.as_ref(), "trap-middle-middle", None);
        let _last = append_button(root.as_ref(), "trap-middle-last", None);

        let platform = WebPlatformEffects;

        let cleanup = platform.attach_focus_trap("trap-middle-root", Box::new(|| {}));

        let plain_event = Event::new("keydown").expect("plain event creation must succeed");

        root.dispatch_event(&plain_event)
            .expect("dispatching plain keydown must succeed");

        focus::focus_element(&middle, false);

        let init = KeyboardEventInit::new();

        init.set_key("Tab");
        init.set_cancelable(true);

        let tab_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("keyboard event creation must succeed");

        root.dispatch_event(&tab_event)
            .expect("dispatching inner tab must succeed");

        assert_eq!(active_element_id(), Some(middle.id()));
        assert!(!tab_event.default_prevented());

        focus::focus_element(&first, false);

        let shift_init = KeyboardEventInit::new();

        shift_init.set_key("Tab");
        shift_init.set_shift_key(true);
        shift_init.set_cancelable(true);

        let shift_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &shift_init)
            .expect("shift-tab event creation must succeed");

        root.dispatch_event(&shift_event)
            .expect("dispatching shift-tab must succeed");

        assert!(shift_event.default_prevented());

        let enter_init = KeyboardEventInit::new();

        enter_init.set_key("Enter");
        enter_init.set_cancelable(true);

        let enter_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter_init)
            .expect("enter event creation must succeed");

        root.dispatch_event(&enter_event)
            .expect("dispatching enter must succeed");

        assert_eq!(active_element_id(), Some(String::from("trap-middle-last")));
        assert!(!enter_event.default_prevented());

        cleanup();

        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn scroll_and_resize_helpers_update_inline_styles() {
        let root = append_div(
            body().as_ref(),
            "resize-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;overflow:auto;",
        );

        let child = append_div(root.as_ref(), "resize-child", "height:400px;");
        let resized = append_div(
            body().as_ref(),
            "resize-target",
            "position:fixed;left:-10000px;top:0;width:120px;",
        );

        let measurements = StubbedElementMeasurements::install(&resized, 90, 50, 56);

        let platform = WebPlatformEffects;

        platform.set_scroll_top("resize-root", 17.6);

        assert_eq!(root.scroll_top(), 18);

        platform.resize_to_content("resize-target", None);

        assert_eq!(
            resized
                .style()
                .get_property_value("height")
                .unwrap_or_default(),
            "96px"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("overflow-y")
                .unwrap_or_default(),
            "auto"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("max-height")
                .unwrap_or_default(),
            ""
        );

        measurements.set(40, 40, 44);

        platform.resize_to_content("resize-target", Some("80px"));

        assert_eq!(
            resized
                .style()
                .get_property_value("height")
                .unwrap_or_default(),
            "44px"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("overflow-y")
                .unwrap_or_default(),
            "hidden"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("max-height")
                .unwrap_or_default(),
            "80px"
        );

        measurements.set(200, 50, 54);

        platform.resize_to_content("resize-target", Some("80px"));

        assert_eq!(
            resized
                .style()
                .get_property_value("height")
                .unwrap_or_default(),
            "80px"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("overflow-y")
                .unwrap_or_default(),
            "auto"
        );

        platform.resize_to_content("resize-target", Some("75%"));

        assert_eq!(
            resized
                .style()
                .get_property_value("height")
                .unwrap_or_default(),
            "204px"
        );
        assert_eq!(
            resized
                .style()
                .get_property_value("max-height")
                .unwrap_or_default(),
            "75%"
        );

        remove_node(&child);
        remove_node(&root);
        remove_node(&resized);
    }

    #[wasm_bindgen_test]
    fn reduced_motion_change_listener_tracks_media_query_changes() {
        let stub = StubbedMediaQuery::install("(prefers-reduced-motion: reduce)", false);

        let platform = WebPlatformEffects;

        let observed = Rc::new(RefCell::new(Vec::<bool>::new()));
        let observed_values = Rc::clone(&observed);

        let cleanup = platform.on_reduced_motion_change(Box::new(move |value| {
            observed_values.borrow_mut().push(value);
        }));

        stub.set_matches(true);
        stub.dispatch_change();
        stub.set_matches(false);
        stub.dispatch_change();

        assert_eq!(observed.borrow().as_slice(), &[true, false]);

        cleanup();

        stub.set_matches(true);
        stub.dispatch_change();

        assert_eq!(observed.borrow().as_slice(), &[true, false]);
    }

    #[wasm_bindgen_test]
    fn reduced_motion_listener_is_noop_when_match_media_returns_null() {
        let stub = StubbedMediaQuery::install("(some-other-query: yes)", false);

        let platform = WebPlatformEffects;

        let observed = Rc::new(Cell::new(0));
        let observed_count = Rc::clone(&observed);

        let cleanup = platform.on_reduced_motion_change(Box::new(move |_| {
            observed_count.set(observed_count.get() + 1);
        }));

        stub.dispatch_change();

        cleanup();

        assert_eq!(observed.get(), 0);
    }

    #[wasm_bindgen_test]
    fn media_query_stub_rejects_unknown_event_types_and_bad_listeners() {
        let stub = StubbedMediaQuery::install("(prefers-reduced-motion: reduce)", false);

        let listeners_before = stub.listeners.borrow().len();

        let media_query_list = window()
            .expect("browser window must exist")
            .match_media("(prefers-reduced-motion: reduce)")
            .expect("matchMedia should succeed")
            .expect("stubbed query should exist");

        let add_listener = Reflect::get(
            media_query_list.as_ref(),
            &JsValue::from_str("addEventListener"),
        )
        .expect("addEventListener must be readable")
        .dyn_into::<Function>()
        .expect("addEventListener must be callable");

        add_listener
            .call3(
                media_query_list.as_ref(),
                &JsValue::from_str("click"),
                &JsValue::from_str("not-a-function"),
                &JsValue::NULL,
            )
            .expect("calling addEventListener with a bad listener must not throw");

        assert_eq!(stub.listeners.borrow().len(), listeners_before);

        add_listener
            .call3(
                media_query_list.as_ref(),
                &JsValue::from_str("change"),
                &JsValue::from_str("still-not-a-function"),
                &JsValue::NULL,
            )
            .expect("calling addEventListener with a bad change listener must not throw");

        assert_eq!(stub.listeners.borrow().len(), listeners_before);

        let remove_listener = Reflect::get(
            media_query_list.as_ref(),
            &JsValue::from_str("removeEventListener"),
        )
        .expect("removeEventListener must be readable")
        .dyn_into::<Function>()
        .expect("removeEventListener must be callable");

        remove_listener
            .call3(
                media_query_list.as_ref(),
                &JsValue::from_str("click"),
                &JsValue::from_str("not-a-function"),
                &JsValue::NULL,
            )
            .expect("calling removeEventListener with an ignored event type must not throw");

        let match_media = Reflect::get(
            window().expect("window").as_ref(),
            &JsValue::from_str("matchMedia"),
        )
        .expect("window.matchMedia must be readable")
        .dyn_into::<Function>()
        .expect("window.matchMedia must be callable");

        let non_string = match_media
            .call1(window().expect("window").as_ref(), &JsValue::NULL)
            .expect("calling stubbed matchMedia with null must succeed");

        assert!(non_string.is_null());

        let unknown = match_media
            .call1(
                window().expect("window").as_ref(),
                &JsValue::from_str("(unknown-query: true)"),
            )
            .expect("calling stubbed matchMedia with an unknown query must succeed");

        assert!(unknown.is_null());
    }

    #[wasm_bindgen_test(async)]
    async fn timeout_future_handles_ready_state_without_registered_waker() {
        let future = TimeoutFuture::new(0);

        TimeoutFuture::new(10).await;

        future.await;
    }

    #[wasm_bindgen_test(async)]
    async fn concurrent_animation_end_registrations_share_one_style_detection_batch() {
        let root = append_div(
            body().as_ref(),
            "animation-batch-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let first = append_div(
            root.as_ref(),
            "animation-batch-first",
            "width:40px;height:40px;",
        );

        let second = append_div(
            root.as_ref(),
            "animation-batch-second",
            "width:40px;height:40px;",
        );

        let platform = WebPlatformEffects;

        let callback_count = Rc::new(Cell::new(0));
        let first_count = Rc::clone(&callback_count);
        let second_count = Rc::clone(&callback_count);

        let first_cleanup = platform.on_animation_end(
            "animation-batch-first",
            Box::new(move || first_count.set(first_count.get() + 1)),
        );

        let second_cleanup = platform.on_animation_end(
            "animation-batch-second",
            Box::new(move || second_count.set(second_count.get() + 1)),
        );

        STYLE_DETECTION_BATCH.with(|batch| {
            let batch = batch.borrow();

            assert_eq!(batch.pending.len(), 2);
            assert!(batch.deferred.is_empty());
            assert!(batch.first_raf_id.is_some());
            assert!(batch.second_raf_id.is_none());
        });

        TimeoutFuture::new(50).await;

        assert_eq!(callback_count.get(), 2);

        STYLE_DETECTION_BATCH.with(|batch| {
            let batch = batch.borrow();

            assert!(batch.pending.is_empty());
            assert!(batch.deferred.is_empty());
            assert!(batch.first_raf_id.is_none());
            assert!(batch.second_raf_id.is_none());
        });

        first_cleanup();
        second_cleanup();

        remove_node(&second);
        remove_node(&first);
        remove_node(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn animation_end_waits_for_real_completion_events() {
        let root = append_div(
            body().as_ref(),
            "animation-events-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let animation_only = append_div(
            root.as_ref(),
            "animation-only-target",
            "animation-name:fade;animation-duration:1s;animation-delay:0s;",
        );

        let transition_only = append_div(
            root.as_ref(),
            "transition-only-target",
            "transition-property:opacity;transition-duration:1s;transition-delay:0s;",
        );

        let mixed = append_div(
            root.as_ref(),
            "mixed-animation-target",
            "animation-name:fade;animation-duration:1s;animation-delay:0s;transition-property:opacity;transition-duration:1s;transition-delay:0s;",
        );

        let platform = WebPlatformEffects;

        let animation_count = Rc::new(Cell::new(0));
        let transition_count = Rc::new(Cell::new(0));
        let mixed_count = Rc::new(Cell::new(0));

        let animation_cleanup = platform.on_animation_end(
            "animation-only-target",
            Box::new({
                let animation_count = Rc::clone(&animation_count);
                move || animation_count.set(animation_count.get() + 1)
            }),
        );

        let transition_cleanup = platform.on_animation_end(
            "transition-only-target",
            Box::new({
                let transition_count = Rc::clone(&transition_count);
                move || transition_count.set(transition_count.get() + 1)
            }),
        );

        let mixed_cleanup = platform.on_animation_end(
            "mixed-animation-target",
            Box::new({
                let mixed_count = Rc::clone(&mixed_count);
                move || mixed_count.set(mixed_count.get() + 1)
            }),
        );

        TimeoutFuture::new(50).await;

        assert_eq!(animation_count.get(), 0);
        assert_eq!(transition_count.get(), 0);
        assert_eq!(mixed_count.get(), 0);

        let animation_event =
            Event::new("animationend").expect("animation event creation must succeed");

        animation_only
            .dispatch_event(&animation_event)
            .expect("dispatching animationend must succeed");

        assert_eq!(animation_count.get(), 1);

        dispatch_transitionend(&transition_only, "transform");

        assert_eq!(transition_count.get(), 0);

        dispatch_transitionend(&transition_only, "opacity");

        assert_eq!(transition_count.get(), 1);

        mixed
            .dispatch_event(&animation_event)
            .expect("dispatching mixed animationend must succeed");

        assert_eq!(mixed_count.get(), 0);

        dispatch_transitionend(&mixed, "opacity");

        assert_eq!(mixed_count.get(), 1);

        animation_cleanup();
        transition_cleanup();
        mixed_cleanup();

        remove_node(&mixed);
        remove_node(&transition_only);
        remove_node(&animation_only);
        remove_node(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn animation_end_handles_reduced_motion_cleanup_and_transition_all() {
        let reduced_motion = StubbedMediaQuery::install("(prefers-reduced-motion: reduce)", true);

        let root = append_div(
            body().as_ref(),
            "animation-edge-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        let reduced = append_div(
            root.as_ref(),
            "reduced-motion-target",
            "animation-name:fade;animation-duration:1s;transition-property:opacity;transition-duration:1s;",
        );

        let cleanup_target = append_div(
            root.as_ref(),
            "cleanup-animation-target",
            "animation-name:fade;animation-duration:1s;",
        );

        let transition_all = append_div(
            root.as_ref(),
            "transition-all-target",
            "transition-property:all;transition-duration:600ms;transition-delay:0ms;",
        );

        let platform = WebPlatformEffects;

        let reduced_count = Rc::new(Cell::new(0));
        let cleanup_count = Rc::new(Cell::new(0));
        let transition_all_count = Rc::new(Cell::new(0));

        let reduced_cleanup = platform.on_animation_end(
            "reduced-motion-target",
            Box::new({
                let reduced_count = Rc::clone(&reduced_count);
                move || reduced_count.set(reduced_count.get() + 1)
            }),
        );

        TimeoutFuture::new(50).await;

        assert_eq!(reduced_count.get(), 1);

        reduced_cleanup();

        reduced_motion.set_matches(false);

        let cleanup_registration = platform.on_animation_end(
            "cleanup-animation-target",
            Box::new({
                let cleanup_count = Rc::clone(&cleanup_count);
                move || cleanup_count.set(cleanup_count.get() + 1)
            }),
        );

        TimeoutFuture::new(50).await;

        cleanup_registration();

        let cleanup_event =
            Event::new("animationend").expect("cleanup animation event must succeed");

        cleanup_target
            .dispatch_event(&cleanup_event)
            .expect("dispatching cleanup animationend must succeed");

        assert_eq!(cleanup_count.get(), 0);

        let transition_all_cleanup = platform.on_animation_end(
            "transition-all-target",
            Box::new({
                let transition_all_count = Rc::clone(&transition_all_count);
                move || transition_all_count.set(transition_all_count.get() + 1)
            }),
        );

        TimeoutFuture::new(50).await;

        dispatch_transitionend_with_elapsed(&transition_all, "opacity", 0.0);

        assert_eq!(transition_all_count.get(), 0);

        TimeoutFuture::new(560).await;

        dispatch_transitionend_with_elapsed(&transition_all, "opacity", 0.6);

        assert_eq!(transition_all_count.get(), 1);

        transition_all_cleanup();

        remove_node(&transition_all);
        remove_node(&cleanup_target);
        remove_node(&reduced);
        remove_node(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn announcements_write_live_regions() {
        let platform = WebPlatformEffects;

        platform.announce("Polite platform");
        platform.announce_assertive("Assertive platform");

        TimeoutFuture::new(170).await;

        let polite = document()
            .get_element_by_id("ars-live-polite")
            .expect("polite live region must exist");

        let assertive = document()
            .get_element_by_id("ars-live-assertive")
            .expect("assertive live region must exist");

        assert_eq!(polite.text_content().as_deref(), Some("Polite platform"));
        assert_eq!(
            assertive.text_content().as_deref(),
            Some("Assertive platform")
        );
    }

    #[wasm_bindgen_test]
    fn focus_trap_escape_invokes_callback() {
        let root = append_div(
            body().as_ref(),
            "trap-root",
            "position:fixed;left:-10000px;top:0;width:240px;height:120px;",
        );

        root.set_attribute("tabindex", "-1")
            .expect("tabindex assignment must succeed");

        let _first = append_button(root.as_ref(), "trap-first", None);
        let second = append_button(root.as_ref(), "trap-second", None);

        let escaped = Rc::new(Cell::new(false));
        let escaped_clone = Rc::clone(&escaped);

        let platform = WebPlatformEffects;

        let cleanup =
            platform.attach_focus_trap("trap-root", Box::new(move || escaped_clone.set(true)));

        focus::focus_element(&second, false);

        let init = KeyboardEventInit::new();

        init.set_key("Escape");

        let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("keyboard event creation must succeed");

        root.dispatch_event(&event)
            .expect("dispatching escape event must succeed");

        assert!(escaped.get());
        assert_eq!(
            document()
                .active_element()
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
                .map(|element| element.id()),
            Some(second.id())
        );

        cleanup();

        remove_node(&root);
    }

    #[wasm_bindgen_test]
    fn focus_trap_escape_stops_at_innermost_trap() {
        let outer = append_div(
            body().as_ref(),
            "trap-outer-root",
            "position:fixed;left:-10000px;top:0;width:260px;height:140px;",
        );

        outer
            .set_attribute("tabindex", "-1")
            .expect("outer tabindex assignment must succeed");

        let inner = append_div(
            outer.as_ref(),
            "trap-inner-root",
            "position:absolute;left:0;top:0;width:180px;height:80px;",
        );

        inner
            .set_attribute("tabindex", "-1")
            .expect("inner tabindex assignment must succeed");

        let _outer_button = append_button(outer.as_ref(), "trap-outer-button", None);
        let inner_button = append_button(inner.as_ref(), "trap-inner-button", None);

        let outer_escapes = Rc::new(Cell::new(0));
        let inner_escapes = Rc::new(Cell::new(0));

        let platform = WebPlatformEffects;

        let outer_cleanup = platform.attach_focus_trap(
            "trap-outer-root",
            Box::new({
                let outer_escapes = Rc::clone(&outer_escapes);
                move || outer_escapes.set(outer_escapes.get() + 1)
            }),
        );

        let inner_cleanup = platform.attach_focus_trap(
            "trap-inner-root",
            Box::new({
                let inner_escapes = Rc::clone(&inner_escapes);
                move || inner_escapes.set(inner_escapes.get() + 1)
            }),
        );

        focus::focus_element(&inner_button, false);

        let init = KeyboardEventInit::new();

        init.set_key("Escape");
        init.set_bubbles(true);
        init.set_cancelable(true);

        let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("nested escape event creation must succeed");

        inner_button
            .dispatch_event(&event)
            .expect("dispatching nested escape event must succeed");

        assert_eq!(inner_escapes.get(), 1);
        assert_eq!(outer_escapes.get(), 0);
        assert!(event.default_prevented());

        inner_cleanup();
        outer_cleanup();

        remove_node(&outer);
    }
}
