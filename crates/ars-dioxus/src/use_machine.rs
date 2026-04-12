//! Reactive hook bridging [`ars_core::Machine`] to Dioxus signals.
//!
//! The [`use_machine`] hook creates a [`Service`] instance, wraps it in Dioxus
//! reactive primitives, and returns a [`UseMachineReturn`] handle for reading
//! state, sending events, and deriving fine-grained reactive values.

use std::fmt;

use ars_core::{Env, HasId, Machine, Service};
use dioxus::prelude::*;

use crate::{
    ephemeral::EphemeralRef,
    provider::{resolve_locale, use_icu_provider, use_messages},
    use_id,
};

/// Return type from [`use_machine`].
///
/// Provides reactive access to a running [`Machine`] instance. All fields are
/// `Copy` (arena-allocated Dioxus handles), so this struct can be freely passed
/// into closures without cloning.
///
/// # Reactive contract
///
/// - Reading [`state`](Self::state) in a component body subscribes to state changes.
/// - Calling [`send`](Self::send) dispatches an event and may update state/context.
/// - [`derive()`](Self::derive) creates fine-grained memos from the connect API.
/// - [`with_api_snapshot()`](Self::with_api_snapshot) provides one-shot imperative access.
pub struct UseMachineReturn<M: Machine + 'static>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    /// Read-only projection of the machine state. Reading it in a component
    /// creates a re-render dependency.
    pub state: ReadSignal<M::State>,

    /// Send an event to the machine.
    /// Safe to call from any handler — does not require reactive scope.
    pub send: Callback<M::Event>,

    /// Access the underlying service for context/props reads and [`derive()`](Self::derive).
    /// Use sparingly — prefer [`derive()`](Self::derive) for reactive data and
    /// [`with_api_snapshot()`](Self::with_api_snapshot) for imperative access.
    pub service: Signal<Service<M>>,

    /// Monotonically increasing counter that increments whenever context changes.
    /// Used by [`derive()`](Self::derive) to track context mutations even when
    /// state remains the same.
    pub context_version: ReadSignal<u64>,
}

// Manual Clone/Copy impls to avoid requiring M: Clone/Copy — all fields are
// arena-allocated Dioxus handles that are always Copy regardless of M.
impl<M: Machine + 'static> Clone for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Machine + 'static> Copy for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
}

impl<M: Machine + 'static> fmt::Debug for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UseMachineReturn")
            .field("state", &self.state)
            .field("context_version", &self.context_version)
            .finish_non_exhaustive()
    }
}

impl<M: Machine + 'static> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    /// Gets a one-shot snapshot of the connect API.
    ///
    /// **Prefer [`derive()`](Self::derive) for reactive data** — this method does
    /// not track dependencies. Use `with_api_snapshot` only for imperative
    /// operations (e.g., reading a value once inside an event handler).
    ///
    /// The connect closure uses a panic callback because `peek()` holds an
    /// immutable borrow on the `Signal`, preventing `send` (which calls
    /// `write()`) from re-entering. Sending events from API snapshots would
    /// cause a re-entrant borrow panic.
    pub fn with_api_snapshot<T>(&self, f: impl Fn(&M::Api<'_>) -> T) -> T {
        let svc = self.service.peek();
        let api = svc.connect(&|_e| {
            #[cfg(debug_assertions)]
            panic!("Cannot send events inside with_api_snapshot — use event handlers instead");
        });
        f(&api)
    }

    /// Creates a fine-grained memo that derives a value from the connect API.
    ///
    /// Only re-computes when the underlying state or context changes, and only
    /// notifies dependents when the derived value actually changes.
    ///
    /// **Important:** `Api<'a>` has a non-`'static` lifetime and cannot be stored
    /// in Dioxus signals. The `&M::Api<'_>` reference passed to the closure is
    /// valid only for that closure call. Extract the values you need (strings,
    /// booleans, [`AttrMap`](ars_core::AttrMap)) and return them.
    ///
    /// The closure must not call `send()` — it is a read-only projection.
    pub fn derive<T: Clone + PartialEq + 'static>(
        &self,
        f: impl Fn(&M::Api<'_>) -> T + 'static,
    ) -> Memo<T> {
        let state = self.state;
        let context_version = self.context_version;
        let service = self.service;
        use_memo(move || {
            // Subscribe to both state and context_version so the memo
            // re-computes when either changes.
            let _ = &*state.read();
            let _ = &*context_version.read();
            let svc = service.peek();
            let api = svc.connect(&|_e| {
                #[cfg(debug_assertions)]
                panic!("Cannot send events inside derive() — use event handlers instead");
            });
            f(&api)
        })
    }

    /// Provides imperative, non-reactive API access wrapped in an [`EphemeralRef`].
    ///
    /// Use this inside event handlers when you need to read the current API state
    /// without creating a reactive subscription. The `EphemeralRef` wrapper prevents
    /// the borrowed API from being stored in signals.
    ///
    /// For reactive derived values, use [`derive()`](Self::derive) instead.
    pub fn with_api_ephemeral<R>(&self, f: impl Fn(EphemeralRef<'_, M::Api<'_>>) -> R) -> R {
        let send = self.send;
        let svc = self.service.peek();
        let send_fn = move |e| send.call(e);
        let api = svc.connect(&send_fn);
        f(EphemeralRef::new(api))
    }
}

/// Creates and manages a machine service with Dioxus reactivity.
///
/// This is the central primitive for using `ars-core` machines in Dioxus components.
/// It creates a [`Service<M>`], wraps it in reactive signals, and returns a
/// [`UseMachineReturn`] handle.
///
/// # Example
///
/// ```rust,ignore
/// #[component]
/// pub fn Toggle() -> Element {
///     let machine = use_machine::<toggle::Machine>(toggle::Props::default());
///     let is_on = machine.derive(|api| api.is_on());
///     rsx! {
///         button {
///             onclick: move |_| machine.send.call(toggle::Event::Toggle),
///             if is_on() { "ON" } else { "OFF" }
///         }
///     }
/// }
/// ```
pub fn use_machine<M: Machine + 'static>(props: M::Props) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    let (result, ..) = use_machine_inner::<M>(props);
    result
}

/// Creates a machine that watches an external props signal for changes.
///
/// When the props signal changes, the hook synchronizes the existing service by
/// calling [`Service::set_props`]. Use this for components with externally
/// controlled state (e.g., a controlled checkbox whose `checked` value comes
/// from a parent signal).
pub fn use_machine_with_reactive_props<M: Machine + 'static>(
    props_signal: Signal<M::Props>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    use_machine::<M>(props_signal())
}

/// Internal implementation shared between public hooks.
///
/// Returns the public `UseMachineReturn` plus the internal `context_version`
/// signal so body-level prop sync can update it during re-renders.
fn use_machine_inner<M: Machine + 'static>(props: M::Props) -> (UseMachineReturn<M>, Signal<u64>)
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    let props = {
        let mut props = props;
        if props.id().is_empty() {
            props.set_id(use_id("component"));
        }
        props
    };

    let locale = resolve_locale(None);
    let icu_provider = use_icu_provider();
    let env = Env {
        locale,
        icu_provider,
    };
    let messages = use_messages::<M::Messages>(None, Some(&env.locale));
    let props_for_sync = props.clone();

    // Create the service once — use_signal runs its closure only on first mount.
    let mut service_signal = use_signal(move || Service::<M>::new(props, &env, &messages));

    // Create a signal tracking the current state.
    // Use .peek() to avoid subscribing the component to service_signal changes.
    let initial_state = service_signal.peek().state().clone();
    let mut state_signal = use_signal::<M::State>(|| initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let mut context_version = use_signal(|| 0u64);

    use_sync_props::<M>(
        service_signal,
        props_for_sync,
        context_version,
        state_signal,
    );

    // Build the send callback. When an event is sent:
    // 1. Snapshot the old state for comparison
    // 2. Forward the event to Service::send()
    // 3. Update signals if state/context changed
    //
    // use_hook runs its closure once on mount and returns the cached value on
    // re-renders. Callback is Copy, so the handle is stable. The captured
    // signal handles are Copy indirections that always access current data.
    let send = use_hook(|| {
        Callback::new(move |event: M::Event| {
            // Write lock is held only for the send() call, then dropped.
            let result = service_signal.write().send(event);

            if result.state_changed {
                let new_state = service_signal.peek().state().clone();
                state_signal.set(new_state);
            }

            if result.context_changed {
                *context_version.write() += 1;
            }

            // TODO: Dispatch result.pending_effects and handle result.cancel_effects
            // when component implementations need effect lifecycle management.
        })
    });

    // Clean up effects when the component unmounts.
    use_drop(move || {
        service_signal.write().unmount(Vec::new());
    });

    let result = UseMachineReturn {
        state: state_signal.into(),
        send,
        service: service_signal,
        context_version: context_version.into(),
    };

    (result, context_version)
}

/// Synchronizes external props into an existing Dioxus machine service.
///
/// Runs during the component body so the service observes new props in the same
/// render pass, avoiding a stale frame after parent prop updates.
fn use_sync_props<M: Machine + 'static>(
    mut service: Signal<Service<M>>,
    current_props: M::Props,
    mut context_version: Signal<u64>,
    mut state_signal: Signal<M::State>,
) where
    M::Props: Clone + PartialEq + 'static,
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    let mut prev_props = use_signal(|| None::<M::Props>);
    let previous = prev_props.peek().clone();

    if previous.as_ref() != Some(&current_props) {
        if previous.is_some() {
            let send_result = service.write().set_props(current_props.clone());
            if send_result.state_changed {
                let new_state = service.peek().state().clone();
                state_signal.set(new_state);
            }
            if send_result.context_changed {
                *context_version.write() += 1;
            }
        }
        prev_props.set(Some(current_props));
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    use ars_core::{
        AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr, I18nRegistries, MessageFn,
        NullPlatformEffects,
    };
    use ars_i18n::{Direction, IcuProvider, Locale};
    use dioxus::dioxus_core::{NoOpMutations, ScopeId};

    use super::*;
    use crate::provider::{ArsContext, NullPlatform};

    // --- Test Machine (mirrors ars-core's ToggleMachine) ---

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleEvent {
        Toggle,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleContext;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleProps {
        id: String,
    }

    impl HasId for ToggleProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TogglePart;

    impl ComponentPart for TogglePart {
        const ROOT: Self = Self;

        fn scope() -> &'static str {
            "toggle"
        }

        fn name(&self) -> &'static str {
            "root"
        }

        fn all() -> Vec<Self> {
            vec![Self]
        }
    }

    struct ToggleApi {
        is_on: bool,
    }

    impl ConnectApi for ToggleApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            let mut attrs = AttrMap::new();
            attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), self.is_on.to_string());
            attrs
        }
    }

    struct ToggleMachine;

    impl Machine for ToggleMachine {
        type State = ToggleState;
        type Event = ToggleEvent;
        type Context = ToggleContext;
        type Props = ToggleProps;
        type Messages = ();
        type Api<'a> = ToggleApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (ToggleState::Off, ToggleContext)
        }

        fn transition(
            state: &Self::State,
            _event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match state {
                ToggleState::Off => Some(ars_core::TransitionPlan::to(ToggleState::On)),
                ToggleState::On => Some(ars_core::TransitionPlan::to(ToggleState::Off)),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi {
                is_on: *state == ToggleState::On,
            }
        }
    }

    // --- Tests ---

    #[derive(Clone)]
    struct TestIcuProvider;

    impl fmt::Debug for TestIcuProvider {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("TestIcuProvider")
        }
    }

    impl IcuProvider for TestIcuProvider {}

    #[derive(Clone, Debug, PartialEq)]
    struct EnvMessages {
        label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for EnvMessages {
        fn default() -> Self {
            Self {
                label: MessageFn::static_str("Default"),
            }
        }
    }

    impl ars_core::ComponentMessages for EnvMessages {}

    #[derive(Clone)]
    struct EnvContext {
        locale: String,
        icu_provider: Arc<dyn IcuProvider>,
        label: String,
    }

    impl fmt::Debug for EnvContext {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("EnvContext")
                .field("locale", &self.locale)
                .field("icu_provider", &"Arc(..)")
                .field("label", &self.label)
                .finish()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EnvProps {
        id: String,
    }

    impl HasId for EnvProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct EnvMachine;

    impl Machine for EnvMachine {
        type State = ();
        type Event = ();
        type Context = EnvContext;
        type Props = EnvProps;
        type Messages = EnvMessages;
        type Api<'a> = ToggleApi;

        fn init(
            _props: &Self::Props,
            env: &Env,
            messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (
                (),
                EnvContext {
                    locale: env.locale.to_bcp47(),
                    icu_provider: Arc::clone(&env.icu_provider),
                    label: (messages.label)(&env.locale),
                },
            )
        }

        fn transition(
            _state: &Self::State,
            _event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            None
        }

        fn connect<'a>(
            _state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi { is_on: false }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum PropState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum PropEvent {
        SetChecked(bool),
        SyncLabel,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct PropContext {
        sync_count: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct PropProps {
        id: String,
        checked: bool,
        label: &'static str,
    }

    impl HasId for PropProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id, ..self }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct PropMachine;

    impl Machine for PropMachine {
        type State = PropState;
        type Event = PropEvent;
        type Context = PropContext;
        type Props = PropProps;
        type Messages = ();
        type Api<'a> = ToggleApi;

        fn init(
            props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (
                if props.checked {
                    PropState::On
                } else {
                    PropState::Off
                },
                PropContext { sync_count: 0 },
            )
        }

        fn transition(
            _state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match event {
                PropEvent::SetChecked(checked) => Some(ars_core::TransitionPlan::to(if *checked {
                    PropState::On
                } else {
                    PropState::Off
                })),
                PropEvent::SyncLabel => Some(ars_core::TransitionPlan::new().apply(
                    |ctx: &mut PropContext| {
                        ctx.sync_count += 1;
                    },
                )),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi {
                is_on: *state == PropState::On,
            }
        }

        fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
            let mut events = Vec::new();
            if old.checked != new.checked {
                events.push(PropEvent::SetChecked(new.checked));
            }
            if old.label != new.label {
                events.push(PropEvent::SyncLabel);
            }
            events
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DerivedState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DerivedEvent {
        Toggle,
        BumpContext,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DerivedContext {
        count: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DerivedProps {
        id: String,
    }

    impl HasId for DerivedProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct DerivedApi {
        is_on: bool,
        count: u32,
    }

    impl ConnectApi for DerivedApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct DerivedMachine;

    impl Machine for DerivedMachine {
        type State = DerivedState;
        type Event = DerivedEvent;
        type Context = DerivedContext;
        type Props = DerivedProps;
        type Messages = ();
        type Api<'a> = DerivedApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (DerivedState::Off, DerivedContext { count: 0 })
        }

        fn transition(
            state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match event {
                DerivedEvent::Toggle => Some(ars_core::TransitionPlan::to(match state {
                    DerivedState::Off => DerivedState::On,
                    DerivedState::On => DerivedState::Off,
                })),
                DerivedEvent::BumpContext => Some(ars_core::TransitionPlan::new().apply(
                    |ctx: &mut DerivedContext| {
                        ctx.count += 1;
                    },
                )),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            DerivedApi {
                is_on: *state == DerivedState::On,
                count: context.count,
            }
        }
    }

    fn provide_test_context(
        locale: &str,
        icu_provider: Arc<dyn IcuProvider>,
        registries: Arc<I18nRegistries>,
    ) -> ArsContext {
        ArsContext::new(
            Locale::parse(locale).expect("locale should parse"),
            Direction::Ltr,
            ars_core::ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            icu_provider,
            registries,
            Arc::new(NullPlatform),
            ars_core::StyleStrategy::Inline,
        )
    }

    #[test]
    fn use_machine_return_type_is_copy() {
        // Verify the struct is Copy by checking that all field types are Copy.
        // This is a compile-time check — if UseMachineReturn<ToggleMachine> is
        // not Copy, this function won't compile.
        fn assert_copy<T: Copy>() {}
        assert_copy::<UseMachineReturn<ToggleMachine>>();
    }

    #[test]
    fn use_machine_return_clone_and_debug_impls_work() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            #[expect(
                clippy::clone_on_copy,
                reason = "This test intentionally exercises the manual Clone impl."
            )]
            let clone = machine.clone();
            assert_eq!(*clone.state.peek(), ToggleState::Off);
            assert!(format!("{machine:?}").contains("UseMachineReturn"));

            rsx! {
                div {}

            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn use_machine_creates_service_with_initial_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            // Initial state should be Off
            assert_eq!(*machine.state.peek(), ToggleState::Off);

            // Context version starts at 0
            assert_eq!(*machine.context_version.peek(), 0);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn use_machine_send_updates_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(*machine.state.peek(), ToggleState::Off);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.state.peek(), ToggleState::On);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.state.peek(), ToggleState::Off);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn with_api_snapshot_reads_current_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let is_on = machine.with_api_snapshot(|api| api.is_on);
            assert!(!is_on);

            machine.send.call(ToggleEvent::Toggle);

            let is_on = machine.with_api_snapshot(|api| api.is_on);
            assert!(is_on);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn with_api_ephemeral_reads_current_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let is_on = machine.with_api_ephemeral(|api| api.get().is_on);
            assert!(!is_on);

            machine.send.call(ToggleEvent::Toggle);

            let is_on = machine.with_api_ephemeral(|api| api.get().is_on);
            assert!(is_on);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn context_version_increments_on_transition() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(*machine.context_version.peek(), 0);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.context_version.peek(), 0);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.context_version.peek(), 0);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn context_version_increments_on_context_only_transition() {
        fn app() -> Element {
            let machine = use_machine::<DerivedMachine>(DerivedProps {
                id: String::from("derived"),
            });

            assert_eq!(*machine.context_version.peek(), 0);
            machine.send.call(DerivedEvent::BumpContext);
            assert_eq!(*machine.context_version.peek(), 1);
            assert_eq!(*machine.state.peek(), DerivedState::Off);

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn derive_recomputes_for_state_and_context_changes() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<(bool, u32)>>>) -> Element {
            let machine = use_machine::<DerivedMachine>(DerivedProps {
                id: String::from("derived"),
            });
            let derived = machine.derive(|api| (api.is_on, api.count));
            let mut phase = use_signal(|| 0u8);

            snapshots.borrow_mut().push(derived());

            if phase() == 0 {
                phase.set(1);
                machine.send.call(DerivedEvent::BumpContext);
            } else if phase() == 1 {
                phase.set(2);
                machine.send.call(DerivedEvent::Toggle);
            }

            rsx! {
                div {}

            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            snapshots.borrow().as_slice(),
            &[(false, 0), (false, 1), (true, 1)]
        );
    }

    #[test]
    fn use_machine_inner_resolves_locale_icu_and_messages_from_context() {
        fn app() -> Element {
            let expected_provider: Arc<dyn IcuProvider> = Arc::new(TestIcuProvider);
            let mut registries = I18nRegistries::new();
            registries.register(
                ars_core::MessagesRegistry::new(EnvMessages::default()).register(
                    "es",
                    EnvMessages {
                        label: MessageFn::static_str("Hola"),
                    },
                ),
            );
            let ctx = provide_test_context(
                "es-ES",
                Arc::clone(&expected_provider),
                Arc::new(registries),
            );
            use_context_provider(|| ctx);

            let machine = use_machine::<EnvMachine>(EnvProps { id: String::new() });
            let service = machine.service.peek();
            assert!(service.props().id().starts_with("component-"));
            assert_eq!(service.context().locale, "es-ES");
            assert!(Arc::ptr_eq(
                &service.context().icu_provider,
                &expected_provider
            ));
            assert_eq!(service.context().label, "Hola");

            rsx! {
                div {}

            }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn use_machine_syncs_external_prop_changes_on_rerender() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<(PropState, u64, u32)>>>) -> Element {
            let mut props = use_signal(|| PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });
            let mut phase = use_signal(|| 0u8);

            let machine = use_machine::<PropMachine>(props());
            let sync_count = machine.service.peek().context().sync_count;
            snapshots.borrow_mut().push((
                *machine.state.peek(),
                *machine.context_version.peek(),
                sync_count,
            ));

            if phase() == 0 {
                phase.set(1);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "a",
                });
            } else if phase() == 1 {
                phase.set(2);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "b",
                });
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            snapshots.borrow().as_slice(),
            &[
                (PropState::Off, 0, 0),
                (PropState::On, 0, 0),
                (PropState::On, 1, 1),
            ]
        );
    }

    #[test]
    fn use_machine_with_reactive_props_syncs_external_prop_changes() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<(PropState, u64, u32)>>>) -> Element {
            let mut props = use_signal(|| PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });
            let mut phase = use_signal(|| 0u8);

            let machine = use_machine_with_reactive_props::<PropMachine>(props);
            let sync_count = machine.service.peek().context().sync_count;
            snapshots.borrow_mut().push((
                *machine.state.peek(),
                *machine.context_version.peek(),
                sync_count,
            ));

            if phase() == 0 {
                phase.set(1);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "a",
                });
            } else if phase() == 1 {
                phase.set(2);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "b",
                });
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            snapshots.borrow().as_slice(),
            &[
                (PropState::Off, 0, 0),
                (PropState::On, 0, 0),
                (PropState::On, 1, 1),
            ]
        );
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use std::{cell::RefCell, rc::Rc};

    use ars_core::{AttrMap, ComponentPart, ConnectApi, HasId};
    use dioxus::dioxus_core::{NoOpMutations, ScopeId};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleEvent {
        Toggle,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleContext;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleProps {
        id: String,
    }

    impl HasId for ToggleProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TogglePart;

    impl ComponentPart for TogglePart {
        const ROOT: Self = Self;

        fn scope() -> &'static str {
            "toggle"
        }

        fn name(&self) -> &'static str {
            "root"
        }

        fn all() -> Vec<Self> {
            vec![Self]
        }
    }

    struct ToggleApi {
        is_on: bool,
    }

    impl ConnectApi for ToggleApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct ToggleMachine;

    impl Machine for ToggleMachine {
        type State = ToggleState;
        type Event = ToggleEvent;
        type Context = ToggleContext;
        type Props = ToggleProps;
        type Messages = ();
        type Api<'a> = ToggleApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (ToggleState::Off, ToggleContext)
        }

        fn transition(
            state: &Self::State,
            _event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match state {
                ToggleState::Off => Some(ars_core::TransitionPlan::to(ToggleState::On)),
                ToggleState::On => Some(ars_core::TransitionPlan::to(ToggleState::Off)),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi {
                is_on: *state == ToggleState::On,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum PropState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum PropEvent {
        SetChecked(bool),
        SyncLabel,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct PropContext {
        sync_count: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct PropProps {
        id: String,
        checked: bool,
        label: &'static str,
    }

    impl HasId for PropProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id, ..self }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct PropApi {
        is_on: bool,
        sync_count: u32,
    }

    impl PropApi {
        const fn snapshot(&self) -> (bool, u32) {
            (self.is_on, self.sync_count)
        }
    }

    impl ConnectApi for PropApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct PropMachine;

    impl Machine for PropMachine {
        type State = PropState;
        type Event = PropEvent;
        type Context = PropContext;
        type Props = PropProps;
        type Messages = ();
        type Api<'a> = PropApi;

        fn init(
            props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (
                if props.checked {
                    PropState::On
                } else {
                    PropState::Off
                },
                PropContext { sync_count: 0 },
            )
        }

        fn transition(
            _state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match event {
                PropEvent::SetChecked(checked) => Some(ars_core::TransitionPlan::to(if *checked {
                    PropState::On
                } else {
                    PropState::Off
                })),
                PropEvent::SyncLabel => Some(ars_core::TransitionPlan::new().apply(
                    |ctx: &mut PropContext| {
                        ctx.sync_count += 1;
                    },
                )),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            PropApi {
                is_on: *state == PropState::On,
                sync_count: context.sync_count,
            }
        }

        fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
            let mut events = Vec::new();
            if old.checked != new.checked {
                events.push(PropEvent::SetChecked(new.checked));
            }
            if old.label != new.label {
                events.push(PropEvent::SyncLabel);
            }
            events
        }
    }

    type PropSnapshot = ((bool, u32), PropState, u64);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DerivedState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DerivedEvent {
        Toggle,
        BumpContext,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DerivedContext {
        count: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DerivedProps {
        id: String,
    }

    impl HasId for DerivedProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct DerivedApi {
        is_on: bool,
        count: u32,
    }

    impl ConnectApi for DerivedApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct DerivedMachine;

    impl Machine for DerivedMachine {
        type State = DerivedState;
        type Event = DerivedEvent;
        type Context = DerivedContext;
        type Props = DerivedProps;
        type Messages = ();
        type Api<'a> = DerivedApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (DerivedState::Off, DerivedContext { count: 0 })
        }

        fn transition(
            state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<ars_core::TransitionPlan<Self>> {
            match event {
                DerivedEvent::Toggle => Some(ars_core::TransitionPlan::to(match state {
                    DerivedState::Off => DerivedState::On,
                    DerivedState::On => DerivedState::Off,
                })),
                DerivedEvent::BumpContext => Some(ars_core::TransitionPlan::new().apply(
                    |ctx: &mut DerivedContext| {
                        ctx.count += 1;
                    },
                )),
            }
        }

        fn connect<'a>(
            state: &'a Self::State,
            context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            DerivedApi {
                is_on: *state == DerivedState::On,
                count: context.count,
            }
        }
    }

    #[wasm_bindgen_test]
    fn use_machine_updates_state_on_wasm() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<bool>>>) -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });
            let mut phase = use_signal(|| 0u8);
            snapshots
                .borrow_mut()
                .push(machine.derive(|api| api.is_on)());

            if phase() == 0 {
                phase.set(1);
                machine.send.call(ToggleEvent::Toggle);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(snapshots.borrow().as_slice(), &[false, true]);
    }

    #[wasm_bindgen_test]
    fn use_machine_injects_generated_id_on_wasm() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<String>>>) -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });
            snapshots
                .borrow_mut()
                .push(machine.service.peek().props().id().to_owned());
            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();

        assert_eq!(snapshots.borrow().len(), 1);
        assert!(snapshots.borrow()[0].starts_with("component-"));
    }

    #[wasm_bindgen_test]
    fn derive_and_reactive_props_sync_on_wasm() {
        let snapshots = Rc::new(RefCell::new(Vec::<PropSnapshot>::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<PropSnapshot>>>) -> Element {
            let mut props = use_signal(|| PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });
            let mut phase = use_signal(|| 0u8);

            let machine = use_machine_with_reactive_props::<PropMachine>(props);
            let derived = machine.derive(PropApi::snapshot);
            snapshots.borrow_mut().push((
                derived(),
                *machine.state.peek(),
                *machine.context_version.peek(),
            ));

            if phase() == 0 {
                phase.set(1);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "a",
                });
            } else if phase() == 1 {
                phase.set(2);
                props.set(PropProps {
                    id: String::from("toggle"),
                    checked: true,
                    label: "b",
                });
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            snapshots.borrow().as_slice(),
            &[
                ((false, 0), PropState::Off, 0),
                ((true, 0), PropState::On, 0),
                ((true, 1), PropState::On, 1),
            ]
        );
    }

    #[wasm_bindgen_test]
    fn derive_recomputes_for_state_and_context_changes_on_wasm() {
        let snapshots = Rc::new(RefCell::new(Vec::new()));

        #[expect(
            clippy::needless_pass_by_value,
            reason = "Dioxus root props are moved into the render function."
        )]
        fn app(snapshots: Rc<RefCell<Vec<(bool, u32)>>>) -> Element {
            let machine = use_machine::<DerivedMachine>(DerivedProps {
                id: String::from("derived"),
            });
            let derived = machine.derive(|api| (api.is_on, api.count));
            let mut phase = use_signal(|| 0u8);

            snapshots.borrow_mut().push(derived());

            if phase() == 0 {
                phase.set(1);
                machine.send.call(DerivedEvent::BumpContext);
            } else if phase() == 1 {
                phase.set(2);
                machine.send.call(DerivedEvent::Toggle);
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new_with_props(app, Rc::clone(&snapshots));
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);

        assert_eq!(
            snapshots.borrow().as_slice(),
            &[(false, 0), (false, 1), (true, 1)]
        );
    }
}
