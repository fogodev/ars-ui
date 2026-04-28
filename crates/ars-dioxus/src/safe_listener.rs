//! Event listener lifecycle helpers for Dioxus adapter components.

#[cfg(feature = "web")]
mod web {
    use std::{
        cell::Cell,
        fmt::{self, Debug},
        rc::Rc,
    };

    use dioxus::prelude::*;
    use web_sys::wasm_bindgen::{JsCast, closure::Closure};

    type ListenerClosure = Closure<dyn FnMut(web_sys::Event)>;
    pub(super) struct RegisteredListener {
        pub(super) target: web_sys::EventTarget,
        pub(super) event_name: &'static str,
        pub(super) capture: bool,
        pub(super) active: Rc<Cell<bool>>,
        pub(super) closure: ListenerClosure,
    }

    pub(super) type ListenerClosureHandle = CopyValue<Vec<RegisteredListener>>;
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

    /// Attaches a DOM event listener with Dioxus-owned lifecycle cleanup.
    pub fn use_safe_event_listener(
        target: Signal<Option<web_sys::EventTarget>>,
        event_name: &'static str,
        handler: impl Fn(web_sys::Event) + 'static,
    ) {
        use_safe_event_listeners(target, vec![SafeEventListener::new(event_name, handler)]);
    }

    /// Attaches DOM event listeners with batched cleanup before registration.
    pub fn use_safe_event_listeners(
        target: Signal<Option<web_sys::EventTarget>>,
        listeners: Vec<SafeEventListener>,
    ) {
        let mut closure_handle = use_hook(|| CopyValue::new(Vec::<RegisteredListener>::new()));

        let mut cleaned_up = use_hook(|| CopyValue::new(false));

        let mut alive = use_signal(|| true);

        use_effect(move || {
            remove_previous_listeners(closure_handle);

            let Some(element) = target.read().clone() else {
                return;
            };

            let mut registrations = Vec::with_capacity(listeners.len());

            for listener in listeners.iter() {
                let handler = Rc::clone(&listener.handler);
                let registration_active = Rc::new(Cell::new(true));

                let closure =
                    guarded_listener_closure(alive, Rc::clone(&registration_active), handler);

                let options = listener_options(listener.options);

                element
                    .add_event_listener_with_callback_and_add_event_listener_options(
                        listener.event_name,
                        closure.as_ref().unchecked_ref(),
                        &options,
                    )
                    .expect("addEventListener");

                registrations.push(RegisteredListener {
                    target: element.clone(),
                    event_name: listener.event_name,
                    capture: listener.options.capture,
                    active: registration_active,
                    closure,
                });
            }

            closure_handle.set(registrations);

            cleaned_up.set(false);
        });

        use_drop(move || {
            if cleaned_up() {
                return;
            }

            cleaned_up.set(true);

            if let Ok(mut alive_ref) = alive.try_write() {
                *alive_ref = false;
            }

            remove_previous_listeners(closure_handle);
        });
    }

    pub(super) fn remove_previous_listeners(mut closure_handle: ListenerClosureHandle) {
        let Ok(mut registrations) = closure_handle.try_write() else {
            return;
        };

        let previous = core::mem::take(&mut *registrations);

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

    pub(super) fn guarded_listener_closure(
        alive: Signal<bool>,
        registration_active: Rc<Cell<bool>>,
        handler: ListenerHandler,
    ) -> ListenerClosure {
        Closure::wrap(Box::new(move |event: web_sys::Event| {
            if registration_active.get() && alive.try_read().is_ok_and(|alive_ref| *alive_ref) {
                handler(event);
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
}

#[cfg(feature = "web")]
pub use web::{
    SafeEventListener, SafeEventListenerOptions, use_safe_event_listener, use_safe_event_listeners,
};

#[cfg(all(test, feature = "web"))]
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

#[cfg(all(test, target_arch = "wasm32", feature = "web"))]
mod wasm_tests {
    use std::{cell::Cell, rc::Rc};

    use dioxus::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        SafeEventListener, SafeEventListenerOptions, use_safe_event_listener,
        use_safe_event_listeners,
        web::{
            ListenerClosureHandle, RegisteredListener, guarded_listener_closure,
            remove_previous_listeners,
        },
    };

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct FixtureProps {
        target: Rc<web_sys::EventTarget>,
        first_calls: Rc<Cell<usize>>,
        second_calls: Rc<Cell<usize>>,
    }

    impl PartialEq for FixtureProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.target, &other.target)
                && Rc::ptr_eq(&self.first_calls, &other.first_calls)
                && Rc::ptr_eq(&self.second_calls, &other.second_calls)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn fixture(props: FixtureProps) -> Element {
        let target = (*props.target).clone();

        let first_calls = Rc::clone(&props.first_calls);
        let second_calls = Rc::clone(&props.second_calls);

        let target_signal = use_signal(move || Some(target.clone()));

        use_safe_event_listeners(
            target_signal,
            vec![
                SafeEventListener::new("ars-first", move |_| {
                    first_calls.set(first_calls.get() + 1);
                }),
                SafeEventListener::new("ars-second", move |_| {
                    second_calls.set(second_calls.get() + 1);
                }),
            ],
        );

        rsx! {
            div {}
        }
    }

    #[derive(Clone)]
    struct OnceFixtureProps {
        target: Rc<web_sys::EventTarget>,
        calls: Rc<Cell<usize>>,
    }

    impl PartialEq for OnceFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.target, &other.target) && Rc::ptr_eq(&self.calls, &other.calls)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn once_fixture(props: OnceFixtureProps) -> Element {
        let target = (*props.target).clone();

        let calls = Rc::clone(&props.calls);

        let target_signal = use_signal(move || Some(target.clone()));

        use_safe_event_listeners(
            target_signal,
            vec![SafeEventListener::new_with_options(
                "ars-once",
                SafeEventListenerOptions::default().once(true),
                move |_| {
                    calls.set(calls.get() + 1);
                },
            )],
        );

        rsx! {
            div {}
        }
    }

    #[derive(Clone)]
    struct SingleFixtureProps {
        target: Rc<web_sys::EventTarget>,
        calls: Rc<Cell<usize>>,
    }

    impl PartialEq for SingleFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.target, &other.target) && Rc::ptr_eq(&self.calls, &other.calls)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn single_fixture(props: SingleFixtureProps) -> Element {
        let target = (*props.target).clone();

        let calls = Rc::clone(&props.calls);

        let target_signal = use_signal(move || Some(target.clone()));

        use_safe_event_listener(target_signal, "ars-single", move |_| {
            calls.set(calls.get() + 1);
        });

        rsx! {
            div {}
        }
    }

    fn dispatch(target: &web_sys::EventTarget, event_name: &str) {
        let event = web_sys::Event::new(event_name).expect("Event must construct");

        target
            .dispatch_event(&event)
            .expect("dispatchEvent must succeed");
    }

    async fn flush() {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            web_sys::window()
                .expect("window should exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50)
                .expect("setTimeout should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
    }

    #[wasm_bindgen_test]
    async fn batched_listeners_dispatch_after_mount() {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist");

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let target: web_sys::EventTarget = document
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into()
            .expect("created div should be an EventTarget");

        let target = Rc::new(target);

        let first_calls = Rc::new(Cell::new(0usize));
        let second_calls = Rc::new(Cell::new(0usize));

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            fixture,
            FixtureProps {
                target: Rc::clone(&target),
                first_calls: Rc::clone(&first_calls),
                second_calls: Rc::clone(&second_calls),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container),
        );

        flush().await;

        dispatch(&target, "ars-first");
        dispatch(&target, "ars-second");

        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn listener_options_once_removes_after_first_dispatch() {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist");

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let target: web_sys::EventTarget = document
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into()
            .expect("created div should be an EventTarget");

        let target = Rc::new(target);

        let calls = Rc::new(Cell::new(0usize));

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            once_fixture,
            OnceFixtureProps {
                target: Rc::clone(&target),
                calls: Rc::clone(&calls),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container),
        );

        flush().await;

        dispatch(&target, "ars-once");
        dispatch(&target, "ars-once");

        assert_eq!(calls.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn single_listener_wrapper_dispatches_after_mount() {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist");

        let container = document
            .create_element("div")
            .expect("create_element should succeed");

        let target: web_sys::EventTarget = document
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into()
            .expect("created div should be an EventTarget");

        let target = Rc::new(target);

        let calls = Rc::new(Cell::new(0usize));

        document
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        let dom = VirtualDom::new_with_props(
            single_fixture,
            SingleFixtureProps {
                target: Rc::clone(&target),
                calls: Rc::clone(&calls),
            },
        );

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container),
        );

        flush().await;

        dispatch(&target, "ars-single");

        assert_eq!(calls.get(), 1);
    }

    #[wasm_bindgen_test]
    fn registration_active_token_blocks_stale_closure() {
        let calls = Rc::new(Cell::new(0usize));

        #[derive(Clone)]
        struct StaleClosureProps {
            calls: Rc<Cell<usize>>,
        }

        impl PartialEq for StaleClosureProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.calls, &other.calls)
            }
        }

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(props: StaleClosureProps) -> Element {
            let alive = use_signal(|| true);

            let target: web_sys::EventTarget = web_sys::window()
                .and_then(|window| window.document())
                .expect("browser document should exist")
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement")
                .into();

            let calls_for_handler = Rc::clone(&props.calls);

            let handler: Rc<dyn Fn(web_sys::Event)> = Rc::new(move |_| {
                calls_for_handler.set(calls_for_handler.get() + 1);
            });

            let registration_active = Rc::new(Cell::new(true));

            let closure = guarded_listener_closure(alive, Rc::clone(&registration_active), handler);

            target
                .add_event_listener_with_callback("ars-stale", closure.as_ref().unchecked_ref())
                .expect("addEventListener should succeed");

            dispatch(&target, "ars-stale");

            registration_active.set(false);

            dispatch(&target, "ars-stale");

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(
            app,
            StaleClosureProps {
                calls: Rc::clone(&calls),
            },
        );

        dom.rebuild_in_place();

        assert_eq!(calls.get(), 1);
    }

    #[wasm_bindgen_test]
    fn registered_listener_removes_from_original_target() {
        let original: Rc<web_sys::EventTarget> = Rc::new(
            web_sys::window()
                .and_then(|window| window.document())
                .expect("browser document should exist")
                .create_element("div")
                .expect("create_element should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("created div should be an HtmlElement")
                .into(),
        );

        let replacement: web_sys::EventTarget = web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
            .create_element("div")
            .expect("create_element should succeed")
            .dyn_into::<web_sys::HtmlElement>()
            .expect("created div should be an HtmlElement")
            .into();

        let calls = Rc::new(Cell::new(0usize));

        #[derive(Clone)]
        struct RemoveListenerProps {
            original: Rc<web_sys::EventTarget>,
            calls: Rc<Cell<usize>>,
        }

        impl PartialEq for RemoveListenerProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.original, &other.original) && Rc::ptr_eq(&self.calls, &other.calls)
            }
        }

        fn app(props: RemoveListenerProps) -> Element {
            use_hook(move || {
                let original = (*props.original).clone();

                let calls_for_handler = Rc::clone(&props.calls);

                let active = Rc::new(Cell::new(true));

                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::Event| {
                        calls_for_handler.set(calls_for_handler.get() + 1);
                    })
                        as Box<dyn FnMut(web_sys::Event)>);

                original
                    .add_event_listener_with_callback(
                        "ars-original",
                        closure.as_ref().unchecked_ref(),
                    )
                    .expect("addEventListener should succeed");

                let handle = CopyValue::new(vec![RegisteredListener {
                    target: original,
                    event_name: "ars-original",
                    capture: false,
                    active,
                    closure,
                }]);

                remove_previous_listeners(handle);
            });

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(
            app,
            RemoveListenerProps {
                original: Rc::clone(&original),
                calls: Rc::clone(&calls),
            },
        );

        dom.rebuild_in_place();

        dispatch(&original, "ars-original");
        dispatch(&replacement, "ars-original");

        assert_eq!(calls.get(), 0);
    }
}
