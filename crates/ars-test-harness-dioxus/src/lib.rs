//! Dioxus-backed adapter test harness entrypoints for ars-ui components.
//!
//! This crate owns the Dioxus-specific [`HarnessBackend`] implementation plus
//! adapter-facing `render(...)` helpers used by Dioxus component tests.
//!
//! For non-web Dioxus targets (Desktop, mobile, SSR) the [`desktop`] module
//! provides a headless [`VirtualDom`](dioxus::prelude::VirtualDom) harness
//! that exercises the `cfg(not(feature = "web"))` graceful-degrade path
//! adapter components follow on those platforms.

#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;

use std::{
    any::{Any, type_name},
    cell::RefCell,
    fmt::{self, Debug},
    pin::Pin,
    rc::Rc,
    str::FromStr,
    time::Duration,
};

use ars_core::{AttrMap, ComponentPart, ConnectApi, Env, HasId, Machine, Service, TransitionPlan};
use ars_i18n::Locale;
use ars_test_harness::{AnyService, Component, HarnessBackend, TestHarness};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Mounted Dioxus test component state returned by [`DioxusHarnessComponent`].
pub struct MountedDioxusHarness<M: Machine> {
    service: Rc<RefCell<Service<M>>>,
    rerender: Rc<dyn Fn()>,
    mount_state: Box<dyn Any>,
}

impl<M: Machine> MountedDioxusHarness<M> {
    /// Creates a mounted Dioxus harness payload from shared service state and a rerender hook.
    #[must_use]
    pub fn new(
        service: Rc<RefCell<Service<M>>>,
        rerender: impl Fn() + 'static,
        mount_state: impl Any,
    ) -> Self {
        Self {
            service,
            rerender: Rc::new(rerender),
            mount_state: Box::new(mount_state),
        }
    }
}

impl<M: Machine> Debug for MountedDioxusHarness<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MountedDioxusHarness")
            .field("service", &self.service.borrow())
            .field("rerender", &"<Fn>")
            .field("mount_state", &"<Any>")
            .finish()
    }
}

/// Adapter-local bridge trait implemented by Dioxus test fixture components.
pub trait DioxusHarnessComponent: 'static {
    /// The machine wrapped by this fixture component.
    type Machine: Machine;

    /// Mounts the fixture into the supplied isolated test container.
    fn mount(self, container: web_sys::HtmlElement) -> MountedDioxusHarness<Self::Machine>;

    /// Mounts the fixture into the supplied container with an explicit locale wrapper.
    fn mount_with_locale(
        self,
        container: web_sys::HtmlElement,
        locale: Locale,
    ) -> MountedDioxusHarness<Self::Machine>;
}

/// Test harness backend that drives Dioxus rendering during adapter tests.
pub struct DioxusHarnessBackend {
    mount_state: Rc<RefCell<Option<Box<dyn Any>>>>,
}

impl DioxusHarnessBackend {
    /// Creates a fresh Dioxus backend instance with empty mount state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mount_state: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for DioxusHarnessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for DioxusHarnessBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DioxusHarnessBackend")
            .field("mounted", &self.mount_state.borrow().is_some())
            .finish()
    }
}

impl HarnessBackend for DioxusHarnessBackend {
    fn mount(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
        #[cfg(target_arch = "wasm32")]
        {
            let container = container.clone();

            let mount_state = Rc::clone(&self.mount_state);

            Box::pin(async move {
                let component = downcast_component(component);

                let timer_guard = ars_test_harness::install_fake_timers();

                let mounted = component.0.mount(container);

                *mount_state.borrow_mut() = Some(Box::new((mounted.mount_state, timer_guard)));

                mounted.service
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            drop((container, component));

            Box::pin(async { wasm_only("DioxusHarnessBackend::mount") })
        }
    }

    fn mount_with_locale(
        &self,
        container: &web_sys::HtmlElement,
        component: Box<dyn Any>,
        locale: Locale,
    ) -> Pin<Box<dyn Future<Output = Box<dyn AnyService>>>> {
        #[cfg(target_arch = "wasm32")]
        {
            let container = container.clone();

            let mount_state = Rc::clone(&self.mount_state);

            Box::pin(async move {
                let component = downcast_component(component);

                let timer_guard = ars_test_harness::install_fake_timers();

                let mounted = component.0.mount_with_locale(container, locale);

                *mount_state.borrow_mut() = Some(Box::new((mounted.mount_state, timer_guard)));

                mounted.service
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            drop((container, component, locale));

            Box::pin(async { wasm_only("DioxusHarnessBackend::mount_with_locale") })
        }
    }

    fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        #[cfg(target_arch = "wasm32")]
        {
            Box::pin(async {
                best_effort_flush().await;
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Box::pin(async {})
        }
    }

    fn advance_time(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
            ars_test_harness::advance_fake_time(duration);
        })
    }
}

/// Renders a Dioxus fixture component into an isolated test container.
pub async fn render<C>(component: C) -> TestHarness
where
    C: DioxusHarnessComponent,
    C::Machine: Machine,
    <<C as DioxusHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
{
    ars_test_harness::render_with_backend(
        ErasedDioxusComponent(Box::new(component)),
        DioxusHarnessBackend::new(),
    )
    .await
}

/// Renders a Dioxus fixture component with an explicit locale provider.
pub async fn mount_with_locale<C>(component: C, locale: Locale) -> TestHarness
where
    C: DioxusHarnessComponent,
    C::Machine: Machine,
    <<C as DioxusHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
{
    ars_test_harness::render_with_locale_and_backend(
        ErasedDioxusComponent(Box::new(component)),
        locale,
        DioxusHarnessBackend::new(),
    )
    .await
}

#[cfg(target_arch = "wasm32")]
async fn best_effort_flush() {
    // Public dioxus-web launch APIs consume the VirtualDom and do not expose a retained
    // handle we can await directly. Waiting for a browser task boundary plus a
    // trailing microtask reliably lets the spawned Dioxus work loop settle DOM
    // edits after external `schedule_update()` invalidations.
    animation_frame_turn().await;
    microtask_turn().await;
    animation_frame_turn().await;
}

#[cfg(target_arch = "wasm32")]
async fn microtask_turn() {
    drop(
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
            &wasm_bindgen::JsValue::UNDEFINED,
        ))
        .await,
    );
}

#[cfg(target_arch = "wasm32")]
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

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "constructed only by the wasm backend bridge")
)]
struct MountedHarnessDyn {
    service: Box<dyn AnyService>,
    mount_state: Box<dyn Any>,
}

impl<M> MountedDioxusHarness<M>
where
    M: Machine,
    M::Event: FromStr + 'static,
{
    fn into_dyn(self) -> MountedHarnessDyn {
        MountedHarnessDyn {
            service: Box::new(ReactiveHarnessService {
                service: self.service,
                rerender: self.rerender,
            }),
            mount_state: self.mount_state,
        }
    }
}

struct ReactiveHarnessService<M: Machine> {
    service: Rc<RefCell<Service<M>>>,
    rerender: Rc<dyn Fn()>,
}

impl<M> AnyService for ReactiveHarnessService<M>
where
    M: Machine,
    M::Event: FromStr + 'static,
{
    fn state_debug(&self) -> String {
        format!("{:?}", self.service.borrow().state())
    }

    fn root_attrs(&self) -> AttrMap {
        let service = self.service.borrow();

        let api = M::connect(
            service.state(),
            service.context(),
            service.props(),
            &noop_send::<M::Event>,
        );

        api.part_attrs(<M::Api<'_> as ConnectApi>::Part::ROOT)
    }

    fn part_attrs(&self, part: &str) -> AttrMap {
        let service = self.service.borrow();

        let api = M::connect(
            service.state(),
            service.context(),
            service.props(),
            &noop_send::<M::Event>,
        );

        for candidate in <M::Api<'_> as ConnectApi>::Part::all() {
            if candidate.name() == part {
                return api.part_attrs(candidate);
            }
        }

        panic!(
            "part_attrs: no part named '{part}' found for {}",
            type_name::<M>()
        );
    }

    fn send_named(&mut self, event_name: &str) {
        let event = event_name.parse::<M::Event>().unwrap_or_else(|_| {
            panic!("unknown event name for {}: {event_name}", type_name::<M>())
        });

        self.send_event(event);
    }

    fn send_boxed(&mut self, event: Box<dyn Any>) {
        let event = event.downcast::<M::Event>().unwrap_or_else(|_| {
            panic!(
                "boxed event type mismatch for {}; expected {}",
                type_name::<M>(),
                type_name::<M::Event>()
            )
        });

        self.send_event(*event);
    }
}

impl<M> ReactiveHarnessService<M>
where
    M: Machine,
{
    fn send_event(&self, event: M::Event) {
        let result = self.service.borrow_mut().send(event);

        if result.state_changed || result.context_changed {
            (self.rerender)();
        }
    }
}

fn noop_send<E>(_event: E) {}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "used only by the wasm backend bridge")
)]
fn downcast_component(component: Box<dyn Any>) -> ErasedDioxusComponent {
    component.downcast::<ErasedDioxusComponent>().map_or_else(
        |_| {
            panic!(
                "DioxusHarnessBackend expected a component created by ars_test_harness_dioxus::render()"
            )
        },
        |component| *component,
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn wasm_only(name: &str) -> ! {
    panic!("{name} requires wasm32-unknown-unknown and a browser test runtime");
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "its inner trait object is exercised only on wasm")
)]
struct ErasedDioxusComponent(Box<dyn ErasedDioxusHarnessComponent>);

impl Component for ErasedDioxusComponent {
    type Machine = HarnessBridgeMachine;
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        dead_code,
        reason = "its methods are called only by the wasm backend bridge"
    )
)]
trait ErasedDioxusHarnessComponent: 'static {
    fn mount(self: Box<Self>, container: web_sys::HtmlElement) -> MountedHarnessDyn;
    fn mount_with_locale(
        self: Box<Self>,
        container: web_sys::HtmlElement,
        locale: Locale,
    ) -> MountedHarnessDyn;
}

impl<C> ErasedDioxusHarnessComponent for C
where
    C: DioxusHarnessComponent,
    C::Machine: Machine,
    <<C as DioxusHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
{
    fn mount(self: Box<Self>, container: web_sys::HtmlElement) -> MountedHarnessDyn {
        (*self).mount(container).into_dyn()
    }

    fn mount_with_locale(
        self: Box<Self>,
        container: web_sys::HtmlElement,
        locale: Locale,
    ) -> MountedHarnessDyn {
        (*self).mount_with_locale(container, locale).into_dyn()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum HarnessBridgePart {
    Root,
}

impl ComponentPart for HarnessBridgePart {
    const ROOT: Self = Self::Root;

    fn scope() -> &'static str {
        "harness-bridge"
    }

    fn name(&self) -> &'static str {
        "root"
    }

    fn all() -> Vec<Self> {
        vec![Self::Root]
    }
}

struct HarnessBridgeApi;

impl ConnectApi for HarnessBridgeApi {
    type Part = HarnessBridgePart;

    fn part_attrs(&self, _part: Self::Part) -> AttrMap {
        AttrMap::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HarnessBridgeProps {
    id: String,
}

impl Default for HarnessBridgeProps {
    fn default() -> Self {
        Self {
            id: String::from("harness-bridge"),
        }
    }
}

impl HasId for HarnessBridgeProps {
    fn id(&self) -> &str {
        &self.id
    }

    fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

struct HarnessBridgeMachine;

impl Machine for HarnessBridgeMachine {
    type State = ();
    type Event = ();
    type Context = ();
    type Props = HarnessBridgeProps;
    type Messages = ();
    type Effect = ars_core::NoEffect;
    type Api<'a> = HarnessBridgeApi;

    fn init(
        _props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        ((), ())
    }

    fn transition(
        _state: &Self::State,
        _event: &Self::Event,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        None
    }

    fn connect<'a>(
        _state: &'a Self::State,
        _context: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        HarnessBridgeApi
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    mod wasm {
        use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

        use ars_core::{
            AttrMap, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Machine, Service,
            TransitionPlan,
        };
        use ars_i18n::{IntlBackend, Locale, StubIntlBackend, locales};
        use ars_test_harness::HarnessBackend;
        use dioxus::prelude::*;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

        use crate::{DioxusHarnessComponent, MountedDioxusHarness, mount_with_locale, render};

        wasm_bindgen_test_configure!(run_in_browser);

        #[derive(Clone, Debug, PartialEq, Eq)]
        enum MockState {
            Idle,
            Open,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        enum MockEvent {
            Open,
            Close,
        }

        impl core::str::FromStr for MockEvent {
            type Err = ();

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                match input {
                    "Open" => Ok(Self::Open),
                    "Close" => Ok(Self::Close),
                    _ => Err(()),
                }
            }
        }

        #[derive(Clone, Debug)]
        struct MockContext;

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct MockProps {
            id: String,
        }

        impl Default for MockProps {
            fn default() -> Self {
                Self {
                    id: String::from("fixture"),
                }
            }
        }

        impl HasId for MockProps {
            fn id(&self) -> &str {
                &self.id
            }

            fn with_id(mut self, id: String) -> Self {
                self.id = id;
                self
            }

            fn set_id(&mut self, id: String) {
                self.id = id;
            }
        }

        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        enum MockPart {
            Root,
        }

        impl ComponentPart for MockPart {
            const ROOT: Self = Self::Root;

            fn scope() -> &'static str {
                "fixture"
            }

            fn name(&self) -> &'static str {
                "root"
            }

            fn all() -> Vec<Self> {
                vec![Self::Root]
            }
        }

        struct MockApi<'a> {
            state: &'a MockState,
        }

        impl ConnectApi for MockApi<'_> {
            type Part = MockPart;

            fn part_attrs(&self, part: Self::Part) -> AttrMap {
                let mut attrs = AttrMap::new();

                attrs.set(HtmlAttr::Data("ars-scope"), MockPart::scope());
                attrs.set(HtmlAttr::Data("ars-part"), part.name());

                match self.state {
                    MockState::Idle => {
                        attrs.set(HtmlAttr::Data("ars-state"), "idle");
                    }

                    MockState::Open => {
                        attrs.set(HtmlAttr::Data("ars-state"), "open");
                    }
                }

                attrs
            }
        }

        type RerenderHandle = Rc<RefCell<Option<Arc<dyn Fn() + Send + Sync>>>>;

        struct MockMachine;

        impl Machine for MockMachine {
            type State = MockState;
            type Event = MockEvent;
            type Context = MockContext;
            type Props = MockProps;
            type Messages = ();
            type Effect = ars_core::NoEffect;
            type Api<'a> = MockApi<'a>;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (MockState::Idle, MockContext)
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (MockState::Idle, MockEvent::Open) => Some(TransitionPlan::to(MockState::Open)),

                    (MockState::Open, MockEvent::Close) => {
                        Some(TransitionPlan::to(MockState::Idle))
                    }

                    _ => None,
                }
            }

            fn connect<'a>(
                state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                MockApi { state }
            }
        }

        #[derive(Clone, Props)]
        struct FixtureViewProps {
            service: Rc<RefCell<Service<MockMachine>>>,
            rerender_handle: RerenderHandle,
        }

        impl PartialEq for FixtureViewProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.service, &other.service)
                    && Rc::ptr_eq(&self.rerender_handle, &other.rerender_handle)
            }
        }

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus component props are passed by value"
        )]
        #[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
        fn FixtureView(props: FixtureViewProps) -> Element {
            *props.rerender_handle.borrow_mut() = Some(dioxus::core::schedule_update());

            let state = format!("{:?}", props.service.borrow().state());

            let locale = ars_dioxus::use_locale().read().to_bcp47();

            rsx! {
                div {
                    "data-ars-scope": "fixture",
                    "data-testid": "root",
                    "data-locale": locale.clone(),
                    "data-state": state.clone(),
                    span { "data-testid": "state", "{state}" }
                    span { "data-testid": "locale", "{locale}" }
                }
            }
        }

        #[derive(Clone, Props)]
        struct FixtureRootProps {
            service: Rc<RefCell<Service<MockMachine>>>,
            locale: Option<Locale>,
            rerender_handle: RerenderHandle,
        }

        impl PartialEq for FixtureRootProps {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.service, &other.service)
                    && self.locale == other.locale
                    && Rc::ptr_eq(&self.rerender_handle, &other.rerender_handle)
            }
        }

        #[expect(non_snake_case, reason = "Dioxus components use PascalCase names")]
        fn FixtureRoot(props: FixtureRootProps) -> Element {
            *props.rerender_handle.borrow_mut() = Some(dioxus::core::schedule_update());
            let service = Rc::clone(&props.service);

            let locale_signal = use_signal(|| props.locale.clone().unwrap_or_else(locales::en_us));

            if props.locale.is_some() {
                rsx! {
                    ars_dioxus::ArsProvider { locale: locale_signal,
                        FixtureView {
                            service,
                            rerender_handle: props.rerender_handle,
                        }
                    }
                }
            } else {
                rsx! {
                    FixtureView { service, rerender_handle: props.rerender_handle }
                }
            }
        }

        struct FixtureComponent;

        impl FixtureComponent {
            fn mount_fixture(
                container: web_sys::HtmlElement,
                locale: Option<Locale>,
            ) -> MountedDioxusHarness<MockMachine> {
                let env = Env::new(
                    locale.clone().unwrap_or_else(locales::en_us),
                    Arc::new(StubIntlBackend) as Arc<dyn IntlBackend>,
                );

                let service = Rc::new(RefCell::new(Service::new(MockProps::default(), &env, &())));

                let rerender_handle: RerenderHandle = Rc::new(RefCell::new(None));

                let rerender = {
                    let rerender_handle = Rc::clone(&rerender_handle);
                    move || {
                        rerender_handle
                            .borrow()
                            .as_ref()
                            .expect("rerender handle should be installed after mount")(
                        );
                    }
                };

                let props = FixtureRootProps {
                    service: Rc::clone(&service),
                    locale,
                    rerender_handle,
                };

                let dom = VirtualDom::new_with_props(FixtureRoot, props);

                dioxus_web::launch::launch_virtual_dom(
                    dom,
                    dioxus_web::Config::new().rootelement(container.into()),
                );

                MountedDioxusHarness::new(service, rerender, ())
            }
        }

        struct TimerFixtureComponent;

        impl TimerFixtureComponent {
            fn mount_fixture(
                container: web_sys::HtmlElement,
                locale: Option<Locale>,
            ) -> MountedDioxusHarness<MockMachine> {
                let env = Env::new(
                    locale.clone().unwrap_or_else(locales::en_us),
                    Arc::new(StubIntlBackend) as Arc<dyn IntlBackend>,
                );

                let service = Rc::new(RefCell::new(Service::new(MockProps::default(), &env, &())));

                let rerender_handle: RerenderHandle = Rc::new(RefCell::new(None));

                let rerender = {
                    let rerender_handle = Rc::clone(&rerender_handle);
                    move || {
                        rerender_handle
                            .borrow()
                            .as_ref()
                            .expect("rerender handle should be installed after mount")(
                        );
                    }
                };

                let props = FixtureRootProps {
                    service: Rc::clone(&service),
                    locale,
                    rerender_handle: Rc::clone(&rerender_handle),
                };

                let dom = VirtualDom::new_with_props(FixtureRoot, props);

                dioxus_web::launch::launch_virtual_dom(
                    dom,
                    dioxus_web::Config::new().rootelement(container.into()),
                );

                let timeout_service = Rc::clone(&service);
                let timeout_rerender_handle = Rc::clone(&rerender_handle);
                let timeout =
                    wasm_bindgen::closure::Closure::<dyn FnMut()>::wrap(Box::new(move || {
                        let result = timeout_service.borrow_mut().send(MockEvent::Open);

                        if result.state_changed || result.context_changed {
                            timeout_rerender_handle
                                .borrow()
                                .as_ref()
                                .expect("rerender handle should be installed before timers fire")(
                            );
                        }
                    }));

                web_sys::window()
                    .expect("window should exist")
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        timeout.as_ref().unchecked_ref(),
                        25,
                    )
                    .expect("timeout should schedule");

                MountedDioxusHarness::new(service, rerender, ((), timeout))
            }
        }

        impl DioxusHarnessComponent for FixtureComponent {
            type Machine = MockMachine;

            fn mount(self, container: web_sys::HtmlElement) -> MountedDioxusHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, None)
            }

            fn mount_with_locale(
                self,
                container: web_sys::HtmlElement,
                locale: Locale,
            ) -> MountedDioxusHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, Some(locale))
            }
        }

        impl DioxusHarnessComponent for TimerFixtureComponent {
            type Machine = MockMachine;

            fn mount(self, container: web_sys::HtmlElement) -> MountedDioxusHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, None)
            }

            fn mount_with_locale(
                self,
                container: web_sys::HtmlElement,
                locale: Locale,
            ) -> MountedDioxusHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, Some(locale))
            }
        }

        fn document() -> web_sys::Document {
            web_sys::window()
                .expect("window should exist")
                .document()
                .expect("document should exist")
        }

        fn container_count() -> u32 {
            document()
                .query_selector_all("[data-ars-test-container]")
                .expect("container query should succeed")
                .length()
        }

        fn append_container() -> web_sys::HtmlElement {
            let container = document()
                .create_element("div")
                .expect("container creation should succeed")
                .dyn_into::<web_sys::HtmlElement>()
                .expect("container should be an HtmlElement");

            document()
                .body()
                .expect("body should exist")
                .append_child(&container)
                .expect("container append should succeed");

            container
        }

        #[wasm_bindgen_test]
        async fn backend_debug_and_mount_state_are_observable() {
            let backend = super::super::DioxusHarnessBackend::default();

            let container = append_container();

            assert!(format!("{backend:?}").contains("mounted: false"));

            let service = backend
                .mount(
                    &container,
                    Box::new(super::super::ErasedDioxusComponent(Box::new(
                        FixtureComponent,
                    ))),
                )
                .await;

            assert!(format!("{backend:?}").contains("mounted: true"));

            drop(service);
            drop(backend);

            container.remove();
        }

        #[wasm_bindgen_test]
        async fn mounted_harness_debug_and_any_service_bridge_paths_work() {
            let container = append_container();

            let mounted = FixtureComponent::mount_fixture(container.clone(), None);

            assert!(format!("{mounted:?}").contains("MountedDioxusHarness"));

            super::super::best_effort_flush().await;

            let mut dyn_harness = mounted.into_dyn();

            assert_eq!(dyn_harness.service.state_debug(), "Idle");
            assert_eq!(
                dyn_harness
                    .service
                    .root_attrs()
                    .get(&HtmlAttr::Data("ars-state")),
                Some("idle")
            );
            assert_eq!(
                dyn_harness
                    .service
                    .part_attrs("root")
                    .get(&HtmlAttr::Data("ars-part")),
                Some("root")
            );

            dyn_harness.service.send_named("Open");

            super::super::best_effort_flush().await;

            assert_eq!(dyn_harness.service.state_debug(), "Open");

            dyn_harness.service.send_boxed(Box::new(MockEvent::Close));

            super::super::best_effort_flush().await;

            assert_eq!(dyn_harness.service.state_debug(), "Idle");

            drop(dyn_harness);

            container.remove();
        }

        #[wasm_bindgen_test]
        async fn render_mounts_into_an_isolated_container() {
            let before = container_count();

            let harness = render(FixtureComponent).await;

            assert_eq!(container_count(), before + 1);
            assert!(harness.query_selector("[data-testid='root']").is_some());
            assert_eq!(
                harness.query("[data-testid='locale']").text_content(),
                "en-US"
            );
        }

        #[wasm_bindgen_test]
        async fn mount_with_locale_wraps_in_ars_provider() {
            let harness = mount_with_locale(
                FixtureComponent,
                Locale::parse("ar-SA").expect("locale should parse"),
            )
            .await;

            assert_eq!(
                harness.query("[data-testid='locale']").text_content(),
                "ar-SA"
            );
            assert_eq!(
                harness.query("[dir='rtl']").attr("dir"),
                Some(String::from("rtl"))
            );
        }

        #[wasm_bindgen_test]
        async fn harness_send_reacts_after_flush() {
            let harness = render(FixtureComponent).await;

            assert_eq!(harness.state(), "Idle");
            assert_eq!(
                harness.query("[data-testid='state']").text_content(),
                "Idle"
            );

            harness.send(MockEvent::Open).await;

            assert_eq!(harness.state(), "Open");
            assert_eq!(
                harness.query("[data-testid='state']").text_content(),
                "Open"
            );
        }

        #[wasm_bindgen_test]
        async fn harness_advance_time_fires_scheduled_timeouts() {
            let harness = render(TimerFixtureComponent).await;

            assert_eq!(
                harness.query("[data-testid='state']").text_content(),
                "Idle"
            );

            harness.advance_time(Duration::from_millis(24)).await;

            assert_eq!(
                harness.query("[data-testid='state']").text_content(),
                "Idle",
                "timeout should not fire before the requested delay"
            );

            harness.advance_time(Duration::from_millis(1)).await;

            assert_eq!(
                harness.query("[data-testid='state']").text_content(),
                "Open",
                "advance_time should drive intercepted browser timers"
            );
        }

        #[wasm_bindgen_test]
        async fn dropping_the_harness_removes_its_container() {
            let before = container_count();

            {
                let _harness = render(FixtureComponent).await;
                assert_eq!(container_count(), before + 1);
            }

            assert_eq!(container_count(), before);
        }

        #[wasm_bindgen_test]
        async fn multiple_harnesses_remain_isolated_across_transitions() {
            let before = container_count();

            let first = render(FixtureComponent).await;
            let second = mount_with_locale(
                FixtureComponent,
                Locale::parse("ar-SA").expect("locale should parse"),
            )
            .await;

            assert_eq!(container_count(), before + 2);
            assert_eq!(
                first.query("[data-testid='locale']").text_content(),
                "en-US"
            );
            assert_eq!(
                second.query("[data-testid='locale']").text_content(),
                "ar-SA"
            );

            first.send(MockEvent::Open).await;

            assert_eq!(first.query("[data-testid='state']").text_content(), "Open");
            assert_eq!(second.query("[data-testid='state']").text_content(), "Idle");

            first.send(MockEvent::Close).await;

            assert_eq!(first.query("[data-testid='state']").text_content(), "Idle");
            assert_eq!(
                second.query("[data-testid='locale']").text_content(),
                "ar-SA"
            );

            drop(first);

            assert_eq!(container_count(), before + 1);
            assert_eq!(second.query("[data-testid='state']").text_content(), "Idle");

            drop(second);

            assert_eq!(container_count(), before);
        }
    }
}
