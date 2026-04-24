//! Leptos-backed adapter test harness entrypoints for ars-ui components.
//!
//! This crate owns the Leptos-specific [`HarnessBackend`] implementation plus
//! adapter-facing `render(...)` helpers used by Leptos component tests.

use std::{
    any::{Any, type_name},
    cell::RefCell,
    fmt::{self, Debug},
    pin::Pin,
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use ars_core::{AttrMap, ComponentPart, ConnectApi, Env, HasId, Machine, Service, TransitionPlan};
use ars_i18n::Locale;
use ars_test_harness::{AnyService, Component, HarnessBackend, TestHarness};

/// Mounted Leptos test component state returned by [`LeptosHarnessComponent`].
pub struct MountedLeptosHarness<M: Machine> {
    service: Arc<Mutex<Service<M>>>,
    rerender: Rc<dyn Fn()>,
    mount_state: Box<dyn Any>,
}

impl<M: Machine> MountedLeptosHarness<M> {
    /// Creates a mounted Leptos harness payload from shared service state and a rerender hook.
    #[must_use]
    pub fn new(
        service: Arc<Mutex<Service<M>>>,
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

impl<M: Machine> Debug for MountedLeptosHarness<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MountedLeptosHarness")
            .field(
                "service",
                &*self
                    .service
                    .lock()
                    .expect("service lock should not be poisoned"),
            )
            .field("rerender", &"<Fn>")
            .field("mount_state", &"<Any>")
            .finish()
    }
}

/// Adapter-local bridge trait implemented by Leptos test fixture components.
pub trait LeptosHarnessComponent: 'static {
    /// The machine wrapped by this fixture component.
    type Machine: Machine;

    /// Mounts the fixture into the supplied isolated test container.
    fn mount(self, container: web_sys::HtmlElement) -> MountedLeptosHarness<Self::Machine>;

    /// Mounts the fixture into the supplied container with an explicit locale wrapper.
    fn mount_with_locale(
        self,
        container: web_sys::HtmlElement,
        locale: Locale,
    ) -> MountedLeptosHarness<Self::Machine>;
}

/// Test harness backend that drives Leptos rendering during adapter tests.
pub struct LeptosHarnessBackend {
    mount_state: Rc<RefCell<Option<Box<dyn Any>>>>,
}

impl LeptosHarnessBackend {
    /// Creates a fresh Leptos backend instance with empty mount state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mount_state: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for LeptosHarnessBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for LeptosHarnessBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LeptosHarnessBackend")
            .field("mounted", &self.mount_state.borrow().is_some())
            .finish()
    }
}

impl HarnessBackend for LeptosHarnessBackend {
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

            Box::pin(async { wasm_only("LeptosHarnessBackend::mount") })
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

            Box::pin(async { wasm_only("LeptosHarnessBackend::mount_with_locale") })
        }
    }

    fn flush(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        #[cfg(target_arch = "wasm32")]
        {
            Box::pin(async {
                leptos::task::tick().await;
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

/// Renders a Leptos fixture component into an isolated test container.
pub async fn render<C>(component: C) -> TestHarness
where
    C: LeptosHarnessComponent,
    C::Machine: Machine,
    <<C as LeptosHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
{
    ars_test_harness::render_with_backend(
        ErasedLeptosComponent(Box::new(component)),
        LeptosHarnessBackend::new(),
    )
    .await
}

/// Renders a Leptos fixture component with an explicit locale provider.
pub async fn mount_with_locale<C>(component: C, locale: Locale) -> TestHarness
where
    C: LeptosHarnessComponent,
    C::Machine: Machine,
    <<C as LeptosHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
{
    ars_test_harness::render_with_locale_and_backend(
        ErasedLeptosComponent(Box::new(component)),
        locale,
        LeptosHarnessBackend::new(),
    )
    .await
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "constructed only by the wasm backend bridge")
)]
struct MountedHarnessDyn {
    service: Box<dyn AnyService>,
    mount_state: Box<dyn Any>,
}

impl<M> MountedLeptosHarness<M>
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
    service: Arc<Mutex<Service<M>>>,
    rerender: Rc<dyn Fn()>,
}

impl<M> AnyService for ReactiveHarnessService<M>
where
    M: Machine,
    M::Event: FromStr + 'static,
{
    fn state_debug(&self) -> String {
        let service = self
            .service
            .lock()
            .expect("service lock should not be poisoned");

        format!("{:?}", service.state())
    }

    fn root_attrs(&self) -> AttrMap {
        let service = self
            .service
            .lock()
            .expect("service lock should not be poisoned");

        let api = M::connect(
            service.state(),
            service.context(),
            service.props(),
            &noop_send::<M::Event>,
        );

        api.part_attrs(<M::Api<'_> as ConnectApi>::Part::ROOT)
    }

    fn part_attrs(&self, part: &str) -> AttrMap {
        let service = self
            .service
            .lock()
            .expect("service lock should not be poisoned");

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
        let result = self
            .service
            .lock()
            .expect("service lock should not be poisoned")
            .send(event);

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
fn downcast_component(component: Box<dyn Any>) -> ErasedLeptosComponent {
    component
        .downcast::<ErasedLeptosComponent>()
        .map_or_else(|_| {
            panic!(
                "LeptosHarnessBackend expected a component created by ars_test_harness_leptos::render()"
            )
        }, |component| *component)
}

#[cfg(not(target_arch = "wasm32"))]
fn wasm_only(name: &str) -> ! {
    panic!("{name} requires wasm32-unknown-unknown and a browser test runtime");
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "its inner trait object is exercised only on wasm")
)]
struct ErasedLeptosComponent(Box<dyn ErasedLeptosHarnessComponent>);

impl Component for ErasedLeptosComponent {
    type Machine = HarnessBridgeMachine;
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        dead_code,
        reason = "its methods are called only by the wasm backend bridge"
    )
)]
trait ErasedLeptosHarnessComponent: 'static {
    fn mount(self: Box<Self>, container: web_sys::HtmlElement) -> MountedHarnessDyn;
    fn mount_with_locale(
        self: Box<Self>,
        container: web_sys::HtmlElement,
        locale: Locale,
    ) -> MountedHarnessDyn;
}

impl<C> ErasedLeptosHarnessComponent for C
where
    C: LeptosHarnessComponent,
    C::Machine: Machine,
    <<C as LeptosHarnessComponent>::Machine as Machine>::Event: FromStr + 'static,
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
        use std::{
            sync::{Arc, Mutex},
            time::Duration,
        };

        use ars_core::{
            AttrMap, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, Machine, Service,
            TransitionPlan,
        };
        use ars_i18n::{IntlBackend, Locale, StubIntlBackend, locales};
        use ars_test_harness::HarnessBackend;
        use leptos::prelude::*;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

        use crate::{LeptosHarnessComponent, MountedLeptosHarness, mount_with_locale, render};

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

        struct MockMachine;

        impl Machine for MockMachine {
            type State = MockState;
            type Event = MockEvent;
            type Context = MockContext;
            type Props = MockProps;
            type Messages = ();
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

        #[component]
        fn FixtureView(
            service: Arc<Mutex<Service<MockMachine>>>,
            version: RwSignal<u64>,
        ) -> impl IntoView {
            let locale = ars_leptos::use_locale();

            let state_attr_service = Arc::clone(&service);
            let state_text_service = Arc::clone(&service);

            let locale_attr = locale;
            let locale_text = locale;

            view! {
                <div
                    data-ars-scope="fixture"
                    data-testid="root"
                    data-locale=move || locale_attr.get().to_bcp47()
                    data-state=move || {
                        let _ = version.get();
                        format!(
                            "{:?}",
                            state_attr_service
                                .lock()
                                .expect("service lock should not be poisoned")
                                .state()
                        )
                    }
                >
                    <span data-testid="state">
                        {move || {
                            let _ = version.get();
                            format!(
                                "{:?}",
                                state_text_service
                                    .lock()
                                    .expect("service lock should not be poisoned")
                                    .state()
                            )
                        }}
                    </span>
                    <span data-testid="locale">{move || locale_text.get().to_bcp47()}</span>
                </div>
            }
        }

        struct FixtureComponent;

        impl FixtureComponent {
            fn mount_fixture(
                container: web_sys::HtmlElement,
                locale: Option<Locale>,
            ) -> MountedLeptosHarness<MockMachine> {
                let env = Env {
                    locale: locale.clone().unwrap_or_else(locales::en_us),
                    intl_backend: Arc::new(StubIntlBackend) as Arc<dyn IntlBackend>,
                };

                let service = Arc::new(Mutex::new(Service::new(MockProps::default(), &env, &())));

                let version = RwSignal::new(0);

                match locale {
                    Some(locale) => {
                        let mounted_service = Arc::clone(&service);
                        let view_service = Arc::clone(&service);
                        let locale = Signal::stored(locale);
                        let mount_handle = mount_to(container, move || {
                            view! {
                                <ars_leptos::ArsProvider locale>
                                    <FixtureView service=Arc::clone(&view_service) version />
                                </ars_leptos::ArsProvider>
                            }
                        });

                        MountedLeptosHarness::new(
                            mounted_service,
                            move || version.update(|value| *value += 1),
                            mount_handle,
                        )
                    }

                    None => {
                        let mounted_service = Arc::clone(&service);
                        let view_service = Arc::clone(&service);
                        let mount_handle = mount_to(container, move || {
                            view! { <FixtureView service=Arc::clone(&view_service) version /> }
                        });

                        MountedLeptosHarness::new(
                            mounted_service,
                            move || version.update(|value| *value += 1),
                            mount_handle,
                        )
                    }
                }
            }
        }

        struct TimerFixtureComponent;

        impl TimerFixtureComponent {
            fn schedule_open_timeout(
                service: Arc<Mutex<Service<MockMachine>>>,
                version: RwSignal<u64>,
            ) -> wasm_bindgen::closure::Closure<dyn FnMut()> {
                let timeout =
                    wasm_bindgen::closure::Closure::<dyn FnMut()>::wrap(Box::new(move || {
                        let result = service
                            .lock()
                            .expect("service lock should not be poisoned")
                            .send(MockEvent::Open);

                        if result.state_changed || result.context_changed {
                            version.update(|value| *value += 1);
                        }
                    }));

                web_sys::window()
                    .expect("window should exist")
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        timeout.as_ref().unchecked_ref(),
                        25,
                    )
                    .expect("timeout should schedule");

                timeout
            }

            fn mount_fixture(
                container: web_sys::HtmlElement,
                locale: Option<Locale>,
            ) -> MountedLeptosHarness<MockMachine> {
                let env = Env {
                    locale: locale.clone().unwrap_or_else(locales::en_us),
                    intl_backend: Arc::new(StubIntlBackend) as Arc<dyn IntlBackend>,
                };

                let service = Arc::new(Mutex::new(Service::new(MockProps::default(), &env, &())));

                let version = RwSignal::new(0);

                let mounted_service = Arc::clone(&service);
                let view_service = Arc::clone(&service);

                if let Some(locale) = locale {
                    let locale = Signal::stored(locale);

                    let mount_handle = mount_to(container, move || {
                        view! {
                            <ars_leptos::ArsProvider locale>
                                <FixtureView service=Arc::clone(&view_service) version />
                            </ars_leptos::ArsProvider>
                        }
                    });

                    let timeout = Self::schedule_open_timeout(Arc::clone(&service), version);

                    MountedLeptosHarness::new(
                        mounted_service,
                        move || version.update(|value| *value += 1),
                        (mount_handle, timeout),
                    )
                } else {
                    let mount_handle = mount_to(container, move || {
                        view! { <FixtureView service=Arc::clone(&view_service) version /> }
                    });

                    let timeout = Self::schedule_open_timeout(Arc::clone(&service), version);

                    MountedLeptosHarness::new(
                        mounted_service,
                        move || version.update(|value| *value += 1),
                        (mount_handle, timeout),
                    )
                }
            }
        }

        impl LeptosHarnessComponent for FixtureComponent {
            type Machine = MockMachine;

            fn mount(self, container: web_sys::HtmlElement) -> MountedLeptosHarness<Self::Machine> {
                let _ = self;
                Self::mount_fixture(container, None)
            }

            fn mount_with_locale(
                self,
                container: web_sys::HtmlElement,
                locale: Locale,
            ) -> MountedLeptosHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, Some(locale))
            }
        }

        impl LeptosHarnessComponent for TimerFixtureComponent {
            type Machine = MockMachine;

            fn mount(self, container: web_sys::HtmlElement) -> MountedLeptosHarness<Self::Machine> {
                let _ = self;

                Self::mount_fixture(container, None)
            }

            fn mount_with_locale(
                self,
                container: web_sys::HtmlElement,
                locale: Locale,
            ) -> MountedLeptosHarness<Self::Machine> {
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
            let backend = super::super::LeptosHarnessBackend::default();

            let container = append_container();

            assert!(format!("{backend:?}").contains("mounted: false"));

            let service = backend
                .mount(
                    &container,
                    Box::new(super::super::ErasedLeptosComponent(Box::new(
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

            assert!(format!("{mounted:?}").contains("MountedLeptosHarness"));

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

            assert_eq!(dyn_harness.service.state_debug(), "Open");

            dyn_harness.service.send_boxed(Box::new(MockEvent::Close));

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
