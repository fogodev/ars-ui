//! Automatic lifecycle hooks for DOM-backed floating positioning.
//!
//! This module wires browser observers and event listeners used by overlays to
//! keep their computed position in sync with viewport, scroll, and DOM changes.

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
use std::cell::RefCell;
#[cfg(all(test, not(all(feature = "web", target_arch = "wasm32"))))]
use std::rc::Rc;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use {
    crate::{debug, scroll::scrollable_ancestors},
    std::rc::Rc,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
type CleanupFn = Box<dyn FnOnce()>;

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Debug, Default, Eq, PartialEq)]
struct VisibilityOverrideState {
    restore_visibility: Option<Option<String>>,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
impl VisibilityOverrideState {
    fn record_override(&mut self, visibility: Option<&str>) {
        if self.restore_visibility.is_none() {
            self.restore_visibility = Some(visibility.map(str::to_owned));
        }
    }

    fn take_restore_visibility(&mut self) -> Option<Option<String>> {
        self.restore_visibility.take()
    }
}

/// Sets up automatic repositioning when the anchor or viewport changes.
///
/// The returned cleanup function disconnects all observers and removes all
/// listeners installed by this call. The core implementation calls `update()`
/// directly from each trigger; adapters remain responsible for requestAnimationFrame
/// batching when they want to coalesce multiple triggers into one recomputation.
#[cfg(feature = "web")]
#[must_use]
pub fn auto_update(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    update: impl Fn() + 'static,
) -> Box<dyn FnOnce()> {
    auto_update_impl(anchor, floating, update)
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[derive(Default)]
struct CleanupStack {
    cleanups: Vec<CleanupFn>,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
impl CleanupStack {
    fn push(&mut self, cleanup: impl FnOnce() + 'static) {
        self.cleanups.push(Box::new(cleanup));
    }

    fn into_runner(self) -> CleanupRunner {
        CleanupRunner {
            cleanups: Some(self.cleanups),
        }
    }
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
struct CleanupRunner {
    cleanups: Option<Vec<CleanupFn>>,
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
impl CleanupRunner {
    fn run(&mut self) {
        if let Some(cleanups) = self.cleanups.take() {
            for cleanup in cleanups.into_iter().rev() {
                cleanup();
            }
        }
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    fn into_cleanup(mut self) -> Box<dyn FnOnce()> {
        Box::new(move || self.run())
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
fn auto_update_impl(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    update: impl Fn() + 'static,
) -> Box<dyn FnOnce()> {
    let Some(window) = web_sys::window() else {
        debug::warn_skipped("auto_update()", "window");

        return Box::new(|| {});
    };

    let update = Rc::new(update);

    let mut cleanups = CleanupStack::default();

    install_resize_observer(anchor, floating, Rc::clone(&update), &mut cleanups);

    install_scroll_and_resize_listeners(anchor, &window, Rc::clone(&update), &mut cleanups);

    install_mutation_observer(anchor, floating, &update, &mut cleanups);

    install_intersection_observer(anchor, floating, Rc::clone(&update), &mut cleanups);

    // The initial recomputation happens after all available observers/listeners
    // have been installed so adapters start from the latest geometry.
    update();

    cleanups.into_runner().into_cleanup()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_resize_observer(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    update: Rc<impl Fn() + 'static>,
    cleanups: &mut CleanupStack,
) {
    let resize_cb = Closure::wrap(Box::new(
        move |_entries: js_sys::Array, _observer: web_sys::ResizeObserver| {
            update();
        },
    )
        as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);

    let Ok(resize_observer) = web_sys::ResizeObserver::new(resize_cb.as_ref().unchecked_ref())
    else {
        debug::warn_message(format_args!(
            "auto_update() could not create ResizeObserver"
        ));

        return;
    };

    resize_observer.observe(anchor);
    resize_observer.observe(floating);

    let resize_observer = resize_observer.clone();
    cleanups.push(move || {
        resize_observer.disconnect();
        drop(resize_cb);
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_scroll_and_resize_listeners(
    anchor: &web_sys::Element,
    window: &web_sys::Window,
    update: Rc<impl Fn() + 'static>,
    cleanups: &mut CleanupStack,
) {
    let event_cb = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        update();
    }) as Box<dyn FnMut(web_sys::Event)>);

    let callback = event_cb
        .as_ref()
        .unchecked_ref::<js_sys::Function>()
        .clone();
    let scroll_ancestors = scrollable_ancestors(anchor);

    let opts = web_sys::AddEventListenerOptions::new();

    opts.set_passive(true);
    opts.set_capture(true);

    for ancestor in &scroll_ancestors {
        debug::warn_dom_error(
            "adding auto_update scroll listener to ancestor",
            ancestor.add_event_listener_with_callback_and_add_event_listener_options(
                "scroll",
                event_cb.as_ref().unchecked_ref(),
                &opts,
            ),
        );
    }

    debug::warn_dom_error(
        "adding auto_update window scroll listener",
        window.add_event_listener_with_callback("scroll", event_cb.as_ref().unchecked_ref()),
    );
    debug::warn_dom_error(
        "adding auto_update window resize listener",
        window.add_event_listener_with_callback("resize", event_cb.as_ref().unchecked_ref()),
    );

    let visual_viewport =
        install_visual_viewport_listeners(window.visual_viewport().as_ref(), &event_cb);

    let window = window.clone();

    cleanups.push(move || {
        for ancestor in &scroll_ancestors {
            debug::warn_dom_error(
                "removing auto_update scroll listener from ancestor",
                ancestor.remove_event_listener_with_callback_and_bool("scroll", &callback, true),
            );
        }

        debug::warn_dom_error(
            "removing auto_update window scroll listener",
            window.remove_event_listener_with_callback("scroll", &callback),
        );
        debug::warn_dom_error(
            "removing auto_update window resize listener",
            window.remove_event_listener_with_callback("resize", &callback),
        );

        remove_visual_viewport_listeners(visual_viewport, &callback);

        drop(event_cb);
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_visual_viewport_listeners(
    visual_viewport: Option<&web_sys::VisualViewport>,
    event_cb: &Closure<dyn FnMut(web_sys::Event)>,
) -> Option<web_sys::VisualViewport> {
    let visual_viewport = visual_viewport?;

    debug::warn_dom_error(
        "adding auto_update visualViewport resize listener",
        visual_viewport
            .add_event_listener_with_callback("resize", event_cb.as_ref().unchecked_ref()),
    );
    debug::warn_dom_error(
        "adding auto_update visualViewport scroll listener",
        visual_viewport
            .add_event_listener_with_callback("scroll", event_cb.as_ref().unchecked_ref()),
    );

    Some(visual_viewport.clone())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn remove_visual_viewport_listeners(
    visual_viewport: Option<web_sys::VisualViewport>,
    callback: &js_sys::Function,
) {
    let Some(visual_viewport) = visual_viewport else {
        return;
    };

    debug::warn_dom_error(
        "removing auto_update visualViewport resize listener",
        visual_viewport.remove_event_listener_with_callback("resize", callback),
    );
    debug::warn_dom_error(
        "removing auto_update visualViewport scroll listener",
        visual_viewport.remove_event_listener_with_callback("scroll", callback),
    );
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_mutation_observer(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    update: &Rc<impl Fn() + 'static>,
    cleanups: &mut CleanupStack,
) {
    let parent_update = Rc::clone(update);
    let mutation_cb = Closure::wrap(Box::new(
        move |_entries: js_sys::Array, _observer: web_sys::MutationObserver| {
            parent_update();
        },
    )
        as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);

    let Ok(parent_mutation_observer) =
        web_sys::MutationObserver::new(mutation_cb.as_ref().unchecked_ref())
    else {
        debug::warn_message(format_args!(
            "auto_update() could not create MutationObserver"
        ));
        return;
    };

    let parent_opts = web_sys::MutationObserverInit::new();

    parent_opts.set_child_list(true);
    parent_opts.set_character_data(true);
    parent_opts.set_subtree(true);

    let geometry_anchor_opts = web_sys::MutationObserverInit::new();
    let geometry_parent_opts = web_sys::MutationObserverInit::new();

    geometry_anchor_opts.set_attributes(true);
    geometry_anchor_opts.set_attribute_filter(&js_sys::Array::of2(
        &JsValue::from_str("class"),
        &JsValue::from_str("style"),
    ));

    geometry_parent_opts.set_attributes(true);
    geometry_parent_opts.set_subtree(true);
    geometry_parent_opts.set_attribute_filter(&js_sys::Array::of2(
        &JsValue::from_str("class"),
        &JsValue::from_str("style"),
    ));

    if let Some(parent) = anchor.parent_element() {
        debug::warn_dom_error(
            "observing auto_update mutations",
            parent_mutation_observer.observe_with_options(&parent, &parent_opts),
        );
    }

    let floating_for_geometry = floating.clone();
    let geometry_update = Rc::clone(update);
    let geometry_mutation_cb = Closure::wrap(Box::new(
        move |entries: js_sys::Array, _observer: web_sys::MutationObserver| {
            if should_update_for_geometry_mutations(&entries, &floating_for_geometry) {
                geometry_update();
            }
        },
    )
        as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);

    let geometry_mutation_observer =
        web_sys::MutationObserver::new(geometry_mutation_cb.as_ref().unchecked_ref()).ok();

    if geometry_mutation_observer.is_none() {
        debug::warn_message(format_args!(
            "auto_update() could not create geometry MutationObserver"
        ));
    }

    if let Some(geometry_mutation_observer) = geometry_mutation_observer.as_ref() {
        debug::warn_dom_error(
            "observing auto_update anchor geometry mutations",
            geometry_mutation_observer.observe_with_options(anchor, &geometry_anchor_opts),
        );

        if let Some(parent) = anchor.parent_element() {
            debug::warn_dom_error(
                "observing auto_update parent geometry mutations",
                geometry_mutation_observer.observe_with_options(&parent, &geometry_parent_opts),
            );
        }
    }

    let parent_mutation_observer = parent_mutation_observer.clone();
    cleanups.push(move || {
        parent_mutation_observer.disconnect();

        if let Some(geometry_mutation_observer) = geometry_mutation_observer {
            geometry_mutation_observer.disconnect();
        }

        drop(geometry_mutation_cb);
        drop(mutation_cb);
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn should_update_for_geometry_mutations(
    entries: &js_sys::Array,
    floating: &web_sys::Element,
) -> bool {
    let floating_node: &web_sys::Node = floating.as_ref();

    for index in 0..entries.length() {
        let Ok(record) = entries.get(index).dyn_into::<web_sys::MutationRecord>() else {
            return true;
        };

        let Some(target) = record.target() else {
            return true;
        };

        if !floating_node.contains(Some(&target)) {
            return true;
        }
    }

    false
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn latest_intersection_entry(
    entries: &js_sys::Array,
) -> Option<web_sys::IntersectionObserverEntry> {
    (0..entries.length()).rev().find_map(|index| {
        entries
            .get(index)
            .dyn_into::<web_sys::IntersectionObserverEntry>()
            .ok()
    })
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn install_intersection_observer(
    anchor: &web_sys::Element,
    floating: &web_sys::Element,
    update: Rc<impl Fn() + 'static>,
    cleanups: &mut CleanupStack,
) {
    let floating = floating.clone();

    let cleanup_floating = floating.clone();

    let visibility_state = Rc::new(RefCell::new(VisibilityOverrideState::default()));

    let cleanup_visibility_state = Rc::clone(&visibility_state);

    let intersection_cb = Closure::wrap(Box::new(
        move |entries: js_sys::Array, _observer: web_sys::IntersectionObserver| {
            let Some(entry) = latest_intersection_entry(&entries) else {
                return;
            };

            if entry.intersection_ratio() == 0.0 {
                hide_floating_visibility(&floating, visibility_state.as_ref());
            } else {
                restore_floating_visibility(&floating, visibility_state.as_ref());

                update();
            }
        },
    )
        as Box<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>);

    let io_opts = web_sys::IntersectionObserverInit::new();

    io_opts.set_threshold(&js_sys::Array::of1(&JsValue::from_f64(0.0)).into());

    let Ok(intersection_observer) = web_sys::IntersectionObserver::new_with_options(
        intersection_cb.as_ref().unchecked_ref(),
        &io_opts,
    ) else {
        debug::warn_message(format_args!(
            "auto_update() could not create IntersectionObserver"
        ));
        return;
    };

    intersection_observer.observe(anchor);

    let intersection_observer = intersection_observer.clone();
    cleanups.push(move || {
        restore_floating_visibility(&cleanup_floating, cleanup_visibility_state.as_ref());

        intersection_observer.disconnect();

        drop(intersection_cb);
    });
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn hide_floating_visibility(
    floating: &web_sys::Element,
    visibility_state: &RefCell<VisibilityOverrideState>,
) {
    let Some(floating) = floating.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };

    let current_visibility = floating
        .style()
        .get_property_value("visibility")
        .ok()
        .filter(|value| !value.is_empty());

    visibility_state
        .borrow_mut()
        .record_override(current_visibility.as_deref());

    debug::warn_dom_error(
        "setting auto_update floating visibility",
        floating.style().set_property("visibility", "hidden"),
    );
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_floating_visibility(
    floating: &web_sys::Element,
    visibility_state: &RefCell<VisibilityOverrideState>,
) {
    let Some(floating) = floating.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };

    let Some(restore_visibility) = visibility_state.borrow_mut().take_restore_visibility() else {
        return;
    };

    if let Some(visibility) = restore_visibility {
        debug::warn_dom_error(
            "restoring auto_update floating visibility",
            floating.style().set_property("visibility", &visibility),
        );

        return;
    }

    debug::warn_dom_error(
        "clearing auto_update floating visibility",
        floating.style().remove_property("visibility").map(|_| ()),
    );
}

#[cfg(all(feature = "web", not(target_arch = "wasm32")))]
#[must_use]
fn auto_update_impl(
    _anchor: &web_sys::Element,
    _floating: &web_sys::Element,
    _update: impl Fn() + 'static,
) -> Box<dyn FnOnce()> {
    Box::new(|| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_runner_executes_steps_only_once() {
        let events = Rc::new(RefCell::new(Vec::new()));

        let mut stack = CleanupStack::default();

        {
            let events = Rc::clone(&events);
            stack.push(move || events.borrow_mut().push("first"));
        }
        {
            let events = Rc::clone(&events);
            stack.push(move || events.borrow_mut().push("second"));
        }

        let mut runner = stack.into_runner();

        runner.run();
        runner.run();

        assert_eq!(&*events.borrow(), &["second", "first"]);
    }

    #[test]
    fn cleanup_runner_removes_before_owned_values_drop() {
        struct DropMarker(&'static str, Rc<RefCell<Vec<&'static str>>>);

        impl Drop for DropMarker {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }

        let events = Rc::new(RefCell::new(Vec::new()));

        let mut stack = CleanupStack::default();

        {
            let events = Rc::clone(&events);
            stack.push(move || {
                let marker = DropMarker("drop-a", Rc::clone(&events));
                events.borrow_mut().push("remove-a");
                drop(marker);
            });
        }
        {
            let events = Rc::clone(&events);
            stack.push(move || {
                let marker = DropMarker("drop-b", Rc::clone(&events));
                events.borrow_mut().push("remove-b");
                drop(marker);
            });
        }

        let mut runner = stack.into_runner();

        runner.run();

        assert_eq!(
            &*events.borrow(),
            &["remove-b", "drop-b", "remove-a", "drop-a"]
        );
    }

    #[test]
    fn visibility_override_state_restores_absent_property() {
        let mut state = VisibilityOverrideState::default();

        state.record_override(None);

        assert_eq!(state.take_restore_visibility(), Some(None));
        assert_eq!(state.take_restore_visibility(), None);
    }

    #[test]
    fn visibility_override_state_preserves_original_property() {
        let mut state = VisibilityOverrideState::default();

        state.record_override(Some("collapse"));
        state.record_override(Some("visible"));

        assert_eq!(
            state.take_restore_visibility(),
            Some(Some(String::from("collapse")))
        );
        assert_eq!(state.take_restore_visibility(), None);
    }
}

#[cfg(all(test, feature = "web", not(target_arch = "wasm32")))]
mod host_web_tests {
    use wasm_bindgen::{JsCast, JsValue};

    use super::*;

    fn dummy_element() -> web_sys::Element {
        JsValue::NULL.unchecked_into()
    }

    #[test]
    fn non_wasm_web_stub_is_safe_to_call() {
        let anchor = dummy_element();

        let floating = dummy_element();

        let cleanup = auto_update(&anchor, &floating, || {});
        cleanup();
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{
        cell::{Cell, RefCell},
        future::Future,
        pin::Pin,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    use js_sys::{Array, Function, Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;
    use crate::scroll::scrollable_ancestors;

    wasm_bindgen_test_configure!(run_in_browser);

    fn assert_reflect_set_succeeds(result: &Result<bool, JsValue>, context: &str) {
        assert!(matches!(result, Ok(true)), "{context}: {result:?}");
    }

    struct JsConstructorGuard {
        window: web_sys::Window,
        constructor_name: &'static str,
        registry_name: &'static str,
        original_window_ctor: JsValue,
        original_global_ctor: JsValue,
        original_registry: JsValue,
    }

    impl Drop for JsConstructorGuard {
        fn drop(&mut self) {
            let global = js_sys::global();

            let restore_global_ctor = Reflect::set(
                &global,
                &JsValue::from_str(self.constructor_name),
                &self.original_global_ctor,
            );

            assert_reflect_set_succeeds(
                &restore_global_ctor,
                "restoring global constructor must succeed",
            );

            let restore_window_ctor = Reflect::set(
                self.window.as_ref(),
                &JsValue::from_str(self.constructor_name),
                &self.original_window_ctor,
            );

            assert_reflect_set_succeeds(
                &restore_window_ctor,
                "restoring window constructor must succeed",
            );

            let restore_registry = Reflect::set(
                &global,
                &JsValue::from_str(self.registry_name),
                &self.original_registry,
            );

            assert_reflect_set_succeeds(
                &restore_registry,
                "restoring observer registry must succeed",
            );
        }
    }

    fn install_constructor_stub(
        constructor_name: &'static str,
        registry_name: &'static str,
        ctor: &Function,
    ) -> JsConstructorGuard {
        let window = web_sys::window().expect("window must exist");

        let global = js_sys::global();

        let original_window_ctor =
            Reflect::get(window.as_ref(), &JsValue::from_str(constructor_name))
                .expect("window constructor must be readable");

        let original_global_ctor = Reflect::get(&global, &JsValue::from_str(constructor_name))
            .expect("global constructor must be readable");
        let original_registry =
            Reflect::get(&global, &JsValue::from_str(registry_name)).unwrap_or(JsValue::UNDEFINED);

        let install_registry = Reflect::set(
            &global,
            &JsValue::from_str(registry_name),
            Array::new().as_ref(),
        );

        assert_reflect_set_succeeds(
            &install_registry,
            "installing observer registry must succeed",
        );

        let install_global =
            Reflect::set(&global, &JsValue::from_str(constructor_name), ctor.as_ref());

        assert_reflect_set_succeeds(
            &install_global,
            "installing global constructor stub must succeed",
        );

        let install_window = Reflect::set(
            window.as_ref(),
            &JsValue::from_str(constructor_name),
            ctor.as_ref(),
        );

        assert_reflect_set_succeeds(
            &install_window,
            "installing window constructor stub must succeed",
        );

        JsConstructorGuard {
            window,
            constructor_name,
            registry_name,
            original_window_ctor,
            original_global_ctor,
            original_registry,
        }
    }

    fn constructor_registry(name: &str) -> Array {
        Reflect::get(&js_sys::global(), &JsValue::from_str(name))
            .expect("observer registry must be readable")
            .dyn_into::<Array>()
            .expect("observer registry must be an Array")
    }

    fn function_prototype(function: &Function) -> Object {
        Reflect::get(function.as_ref(), &JsValue::from_str("prototype"))
            .expect("constructor prototype must be readable")
            .dyn_into::<Object>()
            .expect("constructor prototype must be an object")
    }

    fn first_registry_instance(registry_name: &str) -> JsValue {
        let registry = constructor_registry(registry_name);

        assert_eq!(registry.length(), 1);

        registry.get(0)
    }

    struct StubbedResizeObserver {
        _guard: JsConstructorGuard,
    }

    impl StubbedResizeObserver {
        const CONSTRUCTOR: &'static str = "ResizeObserver";
        const REGISTRY: &'static str = "__arsTestResizeObserverInstances";

        fn install() -> Self {
            let ctor = Function::new_with_args(
                "callback",
                "this.__callback = callback;
                 this.__observed = [];
                 this.__disconnectCount = 0;
                 globalThis.__arsTestResizeObserverInstances.push(this);",
            );

            let prototype = function_prototype(&ctor);

            let observe = Function::new_with_args("target", "this.__observed.push(target);");

            let disconnect = Function::new_no_args("this.__disconnectCount += 1;");

            let set_observe =
                Reflect::set(&prototype, &JsValue::from_str("observe"), observe.as_ref());

            assert_reflect_set_succeeds(
                &set_observe,
                "installing ResizeObserver.observe must succeed",
            );

            let set_disconnect = Reflect::set(
                &prototype,
                &JsValue::from_str("disconnect"),
                disconnect.as_ref(),
            );

            assert_reflect_set_succeeds(
                &set_disconnect,
                "installing ResizeObserver.disconnect must succeed",
            );

            Self {
                _guard: install_constructor_stub(Self::CONSTRUCTOR, Self::REGISTRY, &ctor),
            }
        }

        fn observed_targets(&self) -> Array {
            Reflect::get(
                &first_registry_instance(Self::REGISTRY),
                &JsValue::from_str("__observed"),
            )
            .expect("ResizeObserver observed targets must be readable")
            .dyn_into::<Array>()
            .expect("ResizeObserver observed targets must be an Array")
        }

        fn disconnect_count(&self) -> u32 {
            Reflect::get(
                &first_registry_instance(Self::REGISTRY),
                &JsValue::from_str("__disconnectCount"),
            )
            .expect("ResizeObserver disconnect count must be readable")
            .as_f64()
            .expect("ResizeObserver disconnect count must be numeric") as u32
        }

        fn invoke_callback(&self) {
            let instance = first_registry_instance(Self::REGISTRY);

            let callback = Reflect::get(&instance, &JsValue::from_str("__callback"))
                .expect("ResizeObserver callback must be readable")
                .dyn_into::<Function>()
                .expect("ResizeObserver callback must be callable");

            callback
                .call2(&JsValue::UNDEFINED, Array::new().as_ref(), &instance)
                .expect("invoking ResizeObserver callback must succeed");
        }
    }

    struct ThrowingConstructorGuard {
        _guard: JsConstructorGuard,
    }

    impl ThrowingConstructorGuard {
        fn install(constructor_name: &'static str, registry_name: &'static str) -> Self {
            let ctor = Function::new_no_args("throw new Error('stubbed constructor failure');");

            Self {
                _guard: install_constructor_stub(constructor_name, registry_name, &ctor),
            }
        }
    }

    struct StubbedIntersectionObserverEntry {
        _guard: JsConstructorGuard,
    }

    impl StubbedIntersectionObserverEntry {
        const CONSTRUCTOR: &'static str = "IntersectionObserverEntry";
        const REGISTRY: &'static str = "__arsTestIntersectionObserverEntryRegistry";

        fn install() -> Self {
            let ctor = Function::new_with_args(
                "ratio",
                "this.intersectionRatio = ratio;
                 globalThis.__arsTestIntersectionObserverEntryRegistry.push(this);",
            );

            Self {
                _guard: install_constructor_stub(Self::CONSTRUCTOR, Self::REGISTRY, &ctor),
            }
        }

        fn create(ratio: f64) -> JsValue {
            let constructor =
                Reflect::get(&js_sys::global(), &JsValue::from_str(Self::CONSTRUCTOR))
                    .expect("IntersectionObserverEntry constructor must be readable")
                    .dyn_into::<Function>()
                    .expect("IntersectionObserverEntry constructor must be callable");

            Reflect::construct(&constructor, &Array::of1(&JsValue::from_f64(ratio)))
                .expect("constructing IntersectionObserverEntry must succeed")
        }
    }

    struct StubbedIntersectionObserver {
        _guard: JsConstructorGuard,
    }

    impl StubbedIntersectionObserver {
        const CONSTRUCTOR: &'static str = "IntersectionObserver";
        const REGISTRY: &'static str = "__arsTestIntersectionObserverInstances";

        fn install() -> Self {
            let ctor = Function::new_with_args(
                "callback, options",
                "this.__callback = callback;
                 this.__options = options;
                 this.__observed = [];
                 this.__disconnectCount = 0;
                 globalThis.__arsTestIntersectionObserverInstances.push(this);",
            );

            let prototype = function_prototype(&ctor);

            let observe = Function::new_with_args("target", "this.__observed.push(target);");

            let disconnect = Function::new_no_args("this.__disconnectCount += 1;");

            let set_observe =
                Reflect::set(&prototype, &JsValue::from_str("observe"), observe.as_ref());

            assert_reflect_set_succeeds(
                &set_observe,
                "installing IntersectionObserver.observe must succeed",
            );

            let set_disconnect = Reflect::set(
                &prototype,
                &JsValue::from_str("disconnect"),
                disconnect.as_ref(),
            );

            assert_reflect_set_succeeds(
                &set_disconnect,
                "installing IntersectionObserver.disconnect must succeed",
            );

            Self {
                _guard: install_constructor_stub(Self::CONSTRUCTOR, Self::REGISTRY, &ctor),
            }
        }

        fn observed_targets(&self) -> Array {
            Reflect::get(
                &first_registry_instance(Self::REGISTRY),
                &JsValue::from_str("__observed"),
            )
            .expect("IntersectionObserver observed targets must be readable")
            .dyn_into::<Array>()
            .expect("IntersectionObserver observed targets must be an Array")
        }

        fn disconnect_count(&self) -> u32 {
            Reflect::get(
                &first_registry_instance(Self::REGISTRY),
                &JsValue::from_str("__disconnectCount"),
            )
            .expect("IntersectionObserver disconnect count must be readable")
            .as_f64()
            .expect("IntersectionObserver disconnect count must be numeric") as u32
        }

        fn threshold_is_zero(&self) -> bool {
            let options = Reflect::get(
                &first_registry_instance(Self::REGISTRY),
                &JsValue::from_str("__options"),
            )
            .expect("IntersectionObserver options must be readable");

            let threshold = Reflect::get(&options, &JsValue::from_str("threshold"))
                .expect("IntersectionObserver threshold must be readable");

            if let Some(threshold) = threshold.as_f64() {
                return threshold == 0.0;
            }

            threshold
                .dyn_into::<Array>()
                .ok()
                .and_then(|values| values.get(0).as_f64())
                .is_some_and(|value| value == 0.0)
        }

        fn invoke_callback(&self, entry: &JsValue) {
            self.invoke_entries(&Array::of1(entry));
        }

        fn invoke_entries(&self, entries: &Array) {
            let instance = first_registry_instance(Self::REGISTRY);

            let callback = Reflect::get(&instance, &JsValue::from_str("__callback"))
                .expect("IntersectionObserver callback must be readable")
                .dyn_into::<Function>()
                .expect("IntersectionObserver callback must be callable");

            callback
                .call2(&JsValue::UNDEFINED, entries.as_ref(), &instance)
                .expect("invoking IntersectionObserver callback must succeed");
        }
    }

    fn document() -> web_sys::Document {
        web_sys::window()
            .expect("window must exist")
            .document()
            .expect("document must exist")
    }

    fn body() -> web_sys::HtmlElement {
        document().body().expect("body must exist")
    }

    fn append_div(parent: &web_sys::Element, style: &str) -> web_sys::HtmlElement {
        let element = document()
            .create_element("div")
            .expect("element creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("div must be HtmlElement");

        element
            .set_attribute("style", style)
            .expect("style assignment must succeed");

        parent
            .append_child(&element)
            .expect("append_child must succeed");

        element
    }

    fn cleanup(node: &web_sys::HtmlElement) {
        node.remove();
    }

    fn event(name: &str) -> web_sys::Event {
        web_sys::Event::new(name).expect("event creation must succeed")
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

    async fn next_task() {
        TimeoutFuture::new(0).await;
    }

    #[wasm_bindgen_test]
    fn resize_observer_constructor_failure_is_ignored() {
        let _failure = ThrowingConstructorGuard::install(
            "ResizeObserver",
            "__arsTestResizeObserverFailureRegistry",
        );

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_constructor_failure_is_ignored() {
        let _resize_stub = StubbedResizeObserver::install();

        let _failure = ThrowingConstructorGuard::install(
            "IntersectionObserver",
            "__arsTestIntersectionObserverFailureRegistry",
        );

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn resize_observer_observes_both_elements_and_disconnects_on_cleanup() {
        let resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        let observed = resize_stub.observed_targets();

        assert_eq!(observed.length(), 2);
        assert!(observed.get(0).is_instance_of::<web_sys::Element>());
        assert!(
            observed
                .get(0)
                .dyn_into::<web_sys::Element>()
                .ok()
                .is_some_and(|element| element.is_same_node(Some(anchor.as_ref())))
        );
        assert!(
            observed
                .get(1)
                .dyn_into::<web_sys::Element>()
                .ok()
                .is_some_and(|element| element.is_same_node(Some(floating.as_ref())))
        );

        assert_eq!(updates.get(), 1);

        resize_stub.invoke_callback();

        assert_eq!(updates.get(), 2);

        cleanup_auto();

        assert_eq!(resize_stub.disconnect_count(), 1);

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_observes_anchor_and_applies_visibility_transitions() {
        let _resize_stub = StubbedResizeObserver::install();

        let _entry_stub = StubbedIntersectionObserverEntry::install();

        let intersection_stub = StubbedIntersectionObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        let observed = intersection_stub.observed_targets();

        assert_eq!(observed.length(), 1);
        assert!(
            observed
                .get(0)
                .dyn_into::<web_sys::Element>()
                .ok()
                .is_some_and(|element| element.is_same_node(Some(anchor.as_ref())))
        );
        assert!(intersection_stub.threshold_is_zero());
        assert_eq!(updates.get(), 1);

        let hidden_entry = StubbedIntersectionObserverEntry::create(0.0);

        intersection_stub.invoke_callback(&hidden_entry);

        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            "hidden"
        );
        assert_eq!(updates.get(), 1);

        let visible_entry = StubbedIntersectionObserverEntry::create(1.0);

        intersection_stub.invoke_callback(&visible_entry);

        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            ""
        );
        assert_eq!(updates.get(), 2);

        cleanup_auto();

        assert_eq!(intersection_stub.disconnect_count(), 1);

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_uses_latest_entry_when_multiple_are_queued() {
        let _resize_stub = StubbedResizeObserver::install();

        let _entry_stub = StubbedIntersectionObserverEntry::install();

        let intersection_stub = StubbedIntersectionObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        let hidden_entry = StubbedIntersectionObserverEntry::create(0.0);
        let visible_entry = StubbedIntersectionObserverEntry::create(1.0);
        let entries = Array::new();

        entries.push(&hidden_entry);
        entries.push(&visible_entry);
        intersection_stub.invoke_entries(&entries);

        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            ""
        );
        assert_eq!(updates.get(), 2);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_cleanup_restores_previous_visibility() {
        let _resize_stub = StubbedResizeObserver::install();

        let _entry_stub = StubbedIntersectionObserverEntry::install();

        let intersection_stub = StubbedIntersectionObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(
            root.as_ref(),
            "position:absolute;width:80px;height:20px;visibility:collapse;",
        );

        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), || {});

        let hidden_entry = StubbedIntersectionObserverEntry::create(0.0);

        intersection_stub.invoke_callback(&hidden_entry);

        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            "hidden"
        );

        cleanup_auto();

        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            "collapse"
        );

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_ignores_invalid_entries() {
        let _resize_stub = StubbedResizeObserver::install();

        let _entry_stub = StubbedIntersectionObserverEntry::install();

        let intersection_stub = StubbedIntersectionObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);
        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            ""
        );

        intersection_stub.invoke_entries(&Array::new());

        assert_eq!(updates.get(), 1);
        assert_eq!(
            floating
                .style()
                .get_property_value("visibility")
                .expect("visibility lookup must succeed"),
            ""
        );

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn intersection_observer_skips_visibility_writes_for_non_html_elements() {
        let _resize_stub = StubbedResizeObserver::install();

        let _entry_stub = StubbedIntersectionObserverEntry::install();

        let intersection_stub = StubbedIntersectionObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
            .expect("svg creation must succeed");

        root.append_child(&floating)
            .expect("svg append must succeed");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), &floating, move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        let hidden_entry = StubbedIntersectionObserverEntry::create(0.0);

        intersection_stub.invoke_callback(&hidden_entry);

        assert!(floating.get_attribute("visibility").is_none());
        assert!(floating.get_attribute("style").is_none());
        assert_eq!(updates.get(), 1);

        let visible_entry = StubbedIntersectionObserverEntry::create(1.0);

        intersection_stub.invoke_callback(&visible_entry);

        assert!(floating.get_attribute("visibility").is_none());
        assert_eq!(updates.get(), 2);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn window_and_scroll_ancestor_events_trigger_updates_and_cleanup() {
        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:320px;height:320px;",
        );
        let outer = append_div(
            root.as_ref(),
            "width:220px;height:220px;overflow:auto;border:0;padding:0;margin:0;",
        );
        let inner_scroll = append_div(
            outer.as_ref(),
            "width:180px;height:260px;overflow:auto;border:0;padding:0;margin-top:40px;",
        );
        let content = append_div(inner_scroll.as_ref(), "width:160px;height:500px;");
        let anchor = append_div(
            content.as_ref(),
            "width:100px;height:20px;margin-top:260px;",
        );
        let floating = append_div(root.as_ref(), "position:absolute;width:120px;height:40px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        let window = web_sys::window().expect("window must exist");

        window
            .dispatch_event(&event("resize"))
            .expect("window resize dispatch must succeed");

        assert_eq!(updates.get(), 2);

        let ancestors = scrollable_ancestors(anchor.as_ref());

        for ancestor in &ancestors {
            let before = updates.get();

            ancestor
                .dispatch_event(&event("scroll"))
                .expect("scroll dispatch must succeed");

            assert!(updates.get() > before);
        }

        let after_scroll_dispatches = updates.get();

        cleanup_auto();

        window
            .dispatch_event(&event("resize"))
            .expect("window resize dispatch must succeed");

        assert_eq!(updates.get(), after_scroll_dispatches);

        for ancestor in &ancestors {
            ancestor
                .dispatch_event(&event("scroll"))
                .expect("scroll dispatch must succeed");
            assert_eq!(updates.get(), after_scroll_dispatches);
        }

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn visual_viewport_events_trigger_updates_and_cleanup_when_available() {
        let window = web_sys::window().expect("window must exist");

        let Some(visual_viewport) = window.visual_viewport() else {
            return;
        };

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let anchor = append_div(root.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        visual_viewport
            .dispatch_event(&event("resize"))
            .expect("visualViewport resize dispatch must succeed");

        assert_eq!(updates.get(), 2);

        visual_viewport
            .dispatch_event(&event("scroll"))
            .expect("visualViewport scroll dispatch must succeed");

        assert_eq!(updates.get(), 3);

        cleanup_auto();

        visual_viewport
            .dispatch_event(&event("resize"))
            .expect("visualViewport resize dispatch must succeed");

        visual_viewport
            .dispatch_event(&event("scroll"))
            .expect("visualViewport scroll dispatch must succeed");

        assert_eq!(updates.get(), 3);

        cleanup(&root);
    }

    #[wasm_bindgen_test]
    fn visual_viewport_listener_helpers_ignore_absent_viewport() {
        let callback = Closure::wrap(
            Box::new(move |_event: web_sys::Event| {}) as Box<dyn FnMut(web_sys::Event)>
        );

        let callback_fn = callback.as_ref().unchecked_ref::<Function>().clone();

        assert!(install_visual_viewport_listeners(None, &callback).is_none());

        remove_visual_viewport_listeners(None, &callback_fn);

        drop(callback);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_constructor_failure_is_ignored() {
        let _resize_stub = StubbedResizeObserver::install();

        let _failure = ThrowingConstructorGuard::install(
            "MutationObserver",
            "__arsTestMutationObserverFailureRegistry",
        );

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        let child = append_div(parent.as_ref(), "width:10px;height:10px;");

        next_task().await;

        assert_eq!(updates.get(), 1);

        cleanup_auto();

        child.remove();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_triggers_updates_and_cleanup() {
        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        let child = append_div(parent.as_ref(), "width:10px;height:10px;");

        next_task().await;

        assert!(updates.get() >= 2);

        let after_child_insert = updates.get();

        child.remove();

        next_task().await;

        assert!(updates.get() > after_child_insert);

        anchor
            .set_attribute("class", "moved")
            .expect("anchor class update must succeed");

        next_task().await;

        assert!(updates.get() > after_child_insert + 1);

        cleanup_auto();

        let after_cleanup = updates.get();

        let child_after_cleanup = append_div(parent.as_ref(), "width:12px;height:12px;");

        next_task().await;

        assert_eq!(updates.get(), after_cleanup);

        anchor
            .set_attribute("style", "width:48px;height:48px;")
            .expect("anchor style update must succeed");

        next_task().await;

        assert_eq!(updates.get(), after_cleanup);

        child_after_cleanup.remove();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_tracks_anchor_style_and_class_changes() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(parent.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        anchor
            .set_attribute("class", "anchor-shift")
            .expect("anchor class update must succeed");

        next_task().await;

        assert!(updates.get() >= 2);

        let after_class_update = updates.get();

        anchor
            .style()
            .set_property("transform", "translateX(10px)")
            .expect("anchor style update must succeed");

        next_task().await;

        assert!(updates.get() > after_class_update);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_tracks_parent_style_and_class_changes() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(parent.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        parent
            .set_attribute("class", "parent-shift")
            .expect("parent class update must succeed");

        next_task().await;

        assert!(updates.get() >= 2);

        let after_class_update = updates.get();

        parent
            .style()
            .set_property("transform", "translateX(10px)")
            .expect("parent style update must succeed");

        next_task().await;

        assert!(updates.get() > after_class_update);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_tracks_sibling_style_and_class_changes() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let sibling = append_div(parent.as_ref(), "width:20px;height:20px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(parent.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        sibling
            .set_attribute("class", "sibling-shift")
            .expect("sibling class update must succeed");

        next_task().await;

        assert!(updates.get() >= 2);

        let after_class_update = updates.get();

        sibling
            .style()
            .set_property("height", "32px")
            .expect("sibling style update must succeed");

        next_task().await;

        assert!(updates.get() > after_class_update);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_tracks_parent_subtree_text_mutations() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let sibling = append_div(parent.as_ref(), "width:10px;height:10px;");
        let text = document().create_text_node("before");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(parent.as_ref(), "position:absolute;width:80px;height:20px;");

        sibling
            .append_child(&text)
            .expect("text append must succeed");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        text.set_data("after");

        next_task().await;

        assert!(updates.get() >= 2);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_ignores_floating_style_writes_under_same_parent() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = append_div(parent.as_ref(), "width:40px;height:40px;");
        let floating = append_div(parent.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let floating_for_update = floating.clone();
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            let next_count = update_counter.get() + 1;

            update_counter.set(next_count);
            floating_for_update
                .style()
                .set_property("left", &format!("{next_count}px"))
                .expect("floating style write must succeed");
        });

        assert_eq!(updates.get(), 1);

        next_task().await;

        assert_eq!(updates.get(), 1);

        cleanup_auto();

        cleanup(&root);
    }

    #[wasm_bindgen_test(async)]
    async fn mutation_observer_is_not_installed_for_detached_anchor() {
        let _resize_stub = StubbedResizeObserver::install();

        let root = append_div(
            body().as_ref(),
            "position:fixed;left:-10000px;top:0;width:240px;height:240px;",
        );
        let parent = append_div(root.as_ref(), "width:200px;height:200px;");
        let anchor = document()
            .create_element("div")
            .expect("anchor creation must succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("div must be HtmlElement");
        let floating = append_div(root.as_ref(), "position:absolute;width:80px;height:20px;");

        let updates = Rc::new(Cell::new(0));
        let update_counter = Rc::clone(&updates);
        let cleanup_auto = auto_update(anchor.as_ref(), floating.as_ref(), move || {
            update_counter.set(update_counter.get() + 1);
        });

        assert_eq!(updates.get(), 1);

        parent
            .append_child(&anchor)
            .expect("anchor append must succeed");

        next_task().await;

        assert_eq!(updates.get(), 1);

        let sibling = append_div(parent.as_ref(), "width:10px;height:10px;");

        next_task().await;

        assert_eq!(updates.get(), 1);

        cleanup_auto();

        sibling.remove();

        cleanup(&root);
    }
}
