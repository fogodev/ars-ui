//! Event listener lifecycle helpers for Leptos adapter components.

use std::{
    cell::Cell,
    fmt::{self, Debug},
    rc::{Rc, Weak},
};

use leptos::{
    prelude::*,
    reactive::owner::LocalStorage,
    wasm_bindgen::{JsCast, closure::Closure},
    web_sys,
};

type ListenerClosure = Closure<dyn FnMut(web_sys::Event)>;
struct RegisteredListener {
    target: web_sys::EventTarget,
    event_name: &'static str,
    capture: bool,
    active: Rc<Cell<bool>>,
    closure: ListenerClosure,
}

type ListenerClosureStore = StoredValue<Vec<RegisteredListener>, LocalStorage>;
type ListenerHandler = Rc<dyn Fn(web_sys::Event)>;

/// DOM event listener definition for batch lifecycle registration.
#[derive(Clone)]
pub struct SafeEventListener {
    event_name: &'static str,
    options: SafeEventListenerOptions,
    handler: ListenerHandler,
}

impl Debug for SafeEventListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SafeEventListener")
            .field("event_name", &self.event_name)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl SafeEventListener {
    /// Creates an event listener definition for [`use_safe_event_listeners`].
    pub fn new(event_name: &'static str, handler: impl Fn(web_sys::Event) + 'static) -> Self {
        Self::new_with_options(event_name, SafeEventListenerOptions::default(), handler)
    }

    /// Creates an event listener definition with explicit listener options.
    pub fn new_with_options(
        event_name: &'static str,
        options: SafeEventListenerOptions,
        handler: impl Fn(web_sys::Event) + 'static,
    ) -> Self {
        Self {
            event_name,
            options,
            handler: Rc::new(handler),
        }
    }
}

/// Options passed to DOM event listener registration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SafeEventListenerOptions {
    /// Whether the listener should run during the capture phase.
    pub capture: bool,

    /// Whether the listener promises not to call `preventDefault()`.
    pub passive: bool,

    /// Whether the browser should remove the listener after the first dispatch.
    pub once: bool,
}

impl SafeEventListenerOptions {
    /// Returns listener options with the capture flag set.
    #[must_use]
    pub const fn capture(mut self, capture: bool) -> Self {
        self.capture = capture;
        self
    }

    /// Returns listener options with the passive flag set.
    #[must_use]
    pub const fn passive(mut self, passive: bool) -> Self {
        self.passive = passive;
        self
    }

    /// Returns listener options with the once flag set.
    #[must_use]
    pub const fn once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }
}

/// Attaches a DOM event listener with Leptos-owned lifecycle cleanup.
pub fn use_safe_event_listener<E>(
    target: NodeRef<E>,
    event_name: &'static str,
    handler: impl Fn(web_sys::Event) + 'static,
) where
    E: leptos::tachys::html::element::ElementType,
    E::Output: JsCast + Clone + 'static,
{
    use_safe_event_listeners(target, vec![SafeEventListener::new(event_name, handler)]);
}

/// Attaches DOM event listeners with batched cleanup before registration.
pub fn use_safe_event_listeners<E>(target: NodeRef<E>, listeners: Vec<SafeEventListener>)
where
    E: leptos::tachys::html::element::ElementType,
    E::Output: JsCast + Clone + 'static,
{
    let weak_listeners = listeners
        .iter()
        .map(|listener| {
            (
                listener.event_name,
                listener.options,
                Rc::downgrade(&listener.handler),
            )
        })
        .collect::<Vec<_>>();

    let handler_store = StoredValue::new_local(Some(
        listeners
            .into_iter()
            .map(|listener| listener.handler)
            .collect::<Vec<_>>(),
    ));

    let closure_handle = StoredValue::new_local(Vec::new());
    let cleaned_up = StoredValue::new_local(false);
    let active = StoredValue::new_local(true);

    Effect::new(move |_| {
        remove_previous_listeners(closure_handle);

        let Some(element) = target.get() else {
            return;
        };

        let element: web_sys::EventTarget = element.unchecked_into();

        let mut registrations = Vec::with_capacity(weak_listeners.len());

        for (event_name, options, weak_handler) in &weak_listeners {
            let weak = Weak::clone(weak_handler);
            let registration_active = Rc::new(Cell::new(true));

            let closure = guarded_listener_closure(active, Rc::clone(&registration_active), weak);

            let listener_options = listener_options(*options);

            element
                .add_event_listener_with_callback_and_add_event_listener_options(
                    event_name,
                    closure.as_ref().unchecked_ref(),
                    &listener_options,
                )
                .expect("addEventListener");

            registrations.push(RegisteredListener {
                target: element.clone(),
                event_name,
                capture: options.capture,
                active: registration_active,
                closure,
            });
        }

        closure_handle.set_value(registrations);

        cleaned_up.set_value(false);
    });

    on_cleanup(move || {
        if *cleaned_up.read_value() {
            return;
        }

        cleaned_up.set_value(true);
        active.set_value(false);

        remove_previous_listeners(closure_handle);

        handler_store.set_value(None);
    });
}

fn remove_previous_listeners(closure_handle: ListenerClosureStore) {
    let previous = {
        let mut previous = Vec::new();

        closure_handle.update_value(|value| previous.append(value));

        previous
    };

    for previous in previous {
        previous.active.set(false);

        previous
            .target
            .remove_event_listener_with_callback_and_bool(
                previous.event_name,
                previous.closure.as_ref().unchecked_ref(),
                previous.capture,
            )
            .ok();
    }
}

fn guarded_listener_closure(
    scope_active: StoredValue<bool, LocalStorage>,
    registration_active: Rc<Cell<bool>>,
    weak_handler: Weak<dyn Fn(web_sys::Event)>,
) -> ListenerClosure {
    Closure::wrap(Box::new(move |event: web_sys::Event| {
        if registration_active.get()
            && *scope_active.read_value()
            && let Some(strong) = weak_handler.upgrade()
        {
            strong(event);
        }
    }) as Box<dyn FnMut(web_sys::Event)>)
}

fn listener_options(options: SafeEventListenerOptions) -> web_sys::AddEventListenerOptions {
    let listener_options = web_sys::AddEventListenerOptions::new();

    listener_options.set_capture(options.capture);
    listener_options.set_passive(options.passive);
    listener_options.set_once(options.once);

    listener_options
}

#[cfg(test)]
mod tests {
    use super::{SafeEventListener, SafeEventListenerOptions};

    #[test]
    fn listener_options_builder_sets_capture_passive_and_once() {
        let options = SafeEventListenerOptions::default()
            .capture(true)
            .passive(true)
            .once(true);

        assert!(options.capture);
        assert!(options.passive);
        assert!(options.once);
    }

    #[test]
    fn safe_event_listener_debug_includes_public_diagnostics() {
        let listener = SafeEventListener::new_with_options(
            "ars-debug",
            SafeEventListenerOptions::default()
                .capture(true)
                .passive(true),
            |_| {},
        );

        let debug = format!("{listener:?}");

        assert!(debug.contains("SafeEventListener"));
        assert!(debug.contains("ars-debug"));
        assert!(debug.contains("capture: true"));
        assert!(debug.contains("passive: true"));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::{cell::Cell, rc::Rc};

    use leptos::{
        html,
        mount::mount_to,
        prelude::*,
        reactive::owner::Owner,
        wasm_bindgen::{JsCast, closure::Closure},
        web_sys,
    };
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        RegisteredListener, SafeEventListener, SafeEventListenerOptions, guarded_listener_closure,
        remove_previous_listeners, use_safe_event_listener, use_safe_event_listeners,
    };

    wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> web_sys::Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
    }

    fn dispatch(target: &web_sys::EventTarget, event_name: &str) {
        let event = web_sys::Event::new(event_name).expect("Event must construct");

        target
            .dispatch_event(&event)
            .expect("dispatchEvent must succeed");
    }

    async fn tick() {
        leptos::task::tick().await;
    }

    #[wasm_bindgen_test]
    async fn listener_dispatches_until_mount_cleanup() {
        let owner = Owner::new();

        let (mount_handle, target, calls) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let calls = Rc::new(Cell::new(0usize));
            let calls_for_handler = Rc::clone(&calls);

            let mount_handle = mount_to(parent.clone(), move || {
                let target = NodeRef::<html::Div>::new();

                use_safe_event_listener(target, "ars-test", move |_| {
                    calls_for_handler.set(calls_for_handler.get() + 1);
                });

                view! { <div node_ref=target></div> }
            });

            let target: web_sys::EventTarget = parent
                .first_element_child()
                .expect("mounted element should exist")
                .into();

            (mount_handle, target, calls)
        });

        tick().await;

        dispatch(&target, "ars-test");

        assert_eq!(calls.get(), 1);

        drop(mount_handle);

        dispatch(&target, "ars-test");

        assert_eq!(calls.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn batched_listeners_dispatch_and_cleanup_together() {
        let owner = Owner::new();

        let (mount_handle, target, first_calls, second_calls) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let first_calls = Rc::new(Cell::new(0usize));
            let second_calls = Rc::new(Cell::new(0usize));

            let first_for_handler = Rc::clone(&first_calls);
            let second_for_handler = Rc::clone(&second_calls);

            let mount_handle = mount_to(parent.clone(), move || {
                let target = NodeRef::<html::Div>::new();

                use_safe_event_listeners(
                    target,
                    vec![
                        SafeEventListener::new("ars-first", move |_| {
                            first_for_handler.set(first_for_handler.get() + 1);
                        }),
                        SafeEventListener::new("ars-second", move |_| {
                            second_for_handler.set(second_for_handler.get() + 1);
                        }),
                    ],
                );

                view! { <div node_ref=target></div> }
            });

            let target: web_sys::EventTarget = parent
                .first_element_child()
                .expect("mounted element should exist")
                .into();

            (mount_handle, target, first_calls, second_calls)
        });

        tick().await;

        dispatch(&target, "ars-first");
        dispatch(&target, "ars-second");

        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 1);

        drop(mount_handle);

        dispatch(&target, "ars-first");
        dispatch(&target, "ars-second");

        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn listener_options_once_removes_after_first_dispatch() {
        let owner = Owner::new();

        let (mount_handle, target, calls) = owner.with(|| {
            let parent = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement");

            let calls = Rc::new(Cell::new(0usize));
            let calls_for_handler = Rc::clone(&calls);

            let mount_handle = mount_to(parent.clone(), move || {
                let target = NodeRef::<html::Div>::new();

                use_safe_event_listeners(
                    target,
                    vec![SafeEventListener::new_with_options(
                        "ars-once",
                        SafeEventListenerOptions::default().once(true),
                        move |_| {
                            calls_for_handler.set(calls_for_handler.get() + 1);
                        },
                    )],
                );

                view! { <div node_ref=target></div> }
            });

            let target: web_sys::EventTarget = parent
                .first_element_child()
                .expect("mounted element should exist")
                .into();

            (mount_handle, target, calls)
        });

        tick().await;

        dispatch(&target, "ars-once");
        dispatch(&target, "ars-once");

        assert_eq!(calls.get(), 1);

        drop(mount_handle);
    }

    #[wasm_bindgen_test]
    fn registration_active_token_blocks_stale_closure() {
        let owner = Owner::new();

        owner.with(|| {
            let target: web_sys::EventTarget = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement")
                .into();

            let calls = Rc::new(Cell::new(0usize));
            let calls_for_handler = Rc::clone(&calls);

            let handler: Rc<dyn Fn(web_sys::Event)> = Rc::new(move |_| {
                calls_for_handler.set(calls_for_handler.get() + 1);
            });

            let scope_active = StoredValue::new_local(true);

            let registration_active = Rc::new(Cell::new(true));

            let closure = guarded_listener_closure(
                scope_active,
                Rc::clone(&registration_active),
                Rc::downgrade(&handler),
            );

            target
                .add_event_listener_with_callback("ars-stale", closure.as_ref().unchecked_ref())
                .expect("addEventListener should succeed");

            dispatch(&target, "ars-stale");

            registration_active.set(false);

            dispatch(&target, "ars-stale");

            scope_active.set_value(false);
            registration_active.set(true);

            dispatch(&target, "ars-stale");

            assert_eq!(calls.get(), 1);
        });
    }

    #[wasm_bindgen_test]
    fn registered_listener_removes_from_original_target() {
        let owner = Owner::new();

        owner.with(|| {
            let original: web_sys::EventTarget = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement")
                .into();

            let replacement: web_sys::EventTarget = document()
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement")
                .into();

            let calls = Rc::new(Cell::new(0usize));
            let calls_for_handler = Rc::clone(&calls);

            let active = Rc::new(Cell::new(true));

            let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                calls_for_handler.set(calls_for_handler.get() + 1);
            }) as Box<dyn FnMut(web_sys::Event)>);

            original
                .add_event_listener_with_callback("ars-original", closure.as_ref().unchecked_ref())
                .expect("addEventListener should succeed");

            let store = StoredValue::new_local(vec![RegisteredListener {
                target: original.clone(),
                event_name: "ars-original",
                capture: false,
                active,
                closure,
            }]);

            remove_previous_listeners(store);

            dispatch(&original, "ars-original");
            dispatch(&replacement, "ars-original");

            assert_eq!(calls.get(), 0);
        });
    }
}
