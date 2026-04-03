//! Reactive hook bridging [`ars_core::Machine`] to Dioxus signals.
//!
//! The [`use_machine`] hook creates a [`Service`] instance, wraps it in Dioxus
//! reactive primitives, and returns a [`UseMachineReturn`] handle for reading
//! state, sending events, and deriving fine-grained reactive values.

use ars_core::{Machine, Service};
use dioxus::prelude::*;

use crate::ephemeral::EphemeralRef;

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

impl<M: Machine + 'static> std::fmt::Debug for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
{
    let (result, ..) = use_machine_inner::<M>(props);
    result
}

/// Creates a machine that watches an external props signal for changes.
///
/// When the props signal changes, the machine re-initializes with the new props.
/// Use this for components with externally controlled state (e.g., a controlled
/// checkbox whose `checked` value comes from a parent signal).
///
/// # Panics
///
/// Currently unimplemented — requires `Service::set_props()` which is not yet
/// available in `ars-core`. Will be implemented when the core runtime is extended.
pub fn use_machine_with_reactive_props<M: Machine + 'static>(
    _props_signal: Signal<M::Props>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    todo!("Requires Service::set_props() — blocked on ars-core SendResult addition")
}

/// Internal implementation shared between public hooks.
///
/// Returns the public `UseMachineReturn` plus the internal `context_version`
/// write handle needed by `use_machine_with_reactive_props` for syncing
/// external prop changes.
fn use_machine_inner<M: Machine + 'static>(props: M::Props) -> (UseMachineReturn<M>, Signal<u64>)
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    // Create the service once — use_signal runs its closure only on first mount.
    let mut service_signal = use_signal(|| Service::<M>::new(props));

    // Create a signal tracking the current state.
    // Use .peek() to avoid subscribing the component to service_signal changes.
    let initial_state = service_signal.peek().state().clone();
    let mut state_signal: Signal<M::State> = use_signal(|| initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let mut context_version: Signal<u64> = use_signal(|| 0u64);

    // Build the send callback. When an event is sent:
    // 1. Snapshot the old state for comparison
    // 2. Forward the event to Service::send()
    // 3. Update signals if state/context changed
    //
    // Note: Without SendResult from ars-core, we detect state changes by
    // comparing before/after, and conservatively bump context_version
    // whenever a transition was applied (non-empty effects or state change).
    //
    // use_hook runs its closure once on mount and returns the cached value on
    // re-renders. Callback is Copy, so the handle is stable. The captured
    // signal handles are Copy indirections that always access current data.
    let send = use_hook(|| {
        Callback::new(move |event: M::Event| {
            let old_state = service_signal.peek().state().clone();

            // Write lock is held only for the send() call, then dropped.
            let effects = service_signal.write().send(event);

            let new_state = service_signal.peek().state().clone();
            let state_changed = new_state != old_state;

            if state_changed {
                state_signal.set(new_state);
            }

            // Conservative: bump context_version whenever a transition was applied.
            // A transition was applied if state changed OR effects were produced.
            // This may over-notify but is always correct. Will be precise once
            // ars-core returns SendResult with context_changed flag.
            if state_changed || !effects.is_empty() {
                *context_version.write() += 1;
            }

            // TODO: Effects are collected but not dispatched — PendingEffect::run() does
            // not exist yet in ars-core. Effect lifecycle management will be added
            // when component implementations need it.
            drop(effects);
        })
    });

    // Clean up effects when the component unmounts.
    use_drop(move || {
        // Placeholder: when effect dispatch is implemented, drain all
        // active effect cleanups here in LIFO order.
    });

    let result = UseMachineReturn {
        state: state_signal.into(),
        send,
        service: service_signal,
        context_version: context_version.into(),
    };

    (result, context_version)
}

#[cfg(test)]
mod tests {
    use ars_core::{AttrMap, ComponentPart, ConnectApi, TransitionPlan};

    use super::*;

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
    struct ToggleProps;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TogglePart;

    impl ComponentPart for TogglePart {
        fn root() -> Self {
            Self
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
            attrs.insert("aria-pressed".into(), self.is_on.to_string());
            attrs
        }
    }

    struct ToggleMachine;

    impl Machine for ToggleMachine {
        type State = ToggleState;
        type Event = ToggleEvent;
        type Context = ToggleContext;
        type Props = ToggleProps;
        type Api<'a> = ToggleApi;

        fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
            (ToggleState::Off, ToggleContext)
        }

        fn transition(
            state: &Self::State,
            _event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<TransitionPlan<Self::State, Self::Event, Self::Context>> {
            match state {
                ToggleState::Off => Some(TransitionPlan::new(Some(ToggleState::On))),
                ToggleState::On => Some(TransitionPlan::new(Some(ToggleState::Off))),
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

    #[test]
    fn use_machine_return_type_is_copy() {
        // Verify the struct is Copy by checking that all field types are Copy.
        // This is a compile-time check — if UseMachineReturn<ToggleMachine> is
        // not Copy, this function won't compile.
        fn assert_copy<T: Copy>() {}
        assert_copy::<UseMachineReturn<ToggleMachine>>();
    }

    #[test]
    fn use_machine_creates_service_with_initial_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps);

            // Initial state should be Off
            assert_eq!(*machine.state.peek(), ToggleState::Off);

            // Context version starts at 0
            assert_eq!(*machine.context_version.peek(), 0);

            rsx! { div {} }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn use_machine_send_updates_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps);

            assert_eq!(*machine.state.peek(), ToggleState::Off);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.state.peek(), ToggleState::On);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.state.peek(), ToggleState::Off);

            rsx! { div {} }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn with_api_snapshot_reads_current_state() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps);

            let is_on = machine.with_api_snapshot(|api| api.is_on);
            assert!(!is_on);

            machine.send.call(ToggleEvent::Toggle);

            let is_on = machine.with_api_snapshot(|api| api.is_on);
            assert!(is_on);

            rsx! { div {} }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[test]
    fn context_version_increments_on_transition() {
        fn app() -> Element {
            let machine = use_machine::<ToggleMachine>(ToggleProps);

            assert_eq!(*machine.context_version.peek(), 0);

            machine.send.call(ToggleEvent::Toggle);
            // Conservative: bumps on any applied transition
            assert_eq!(*machine.context_version.peek(), 1);

            machine.send.call(ToggleEvent::Toggle);
            assert_eq!(*machine.context_version.peek(), 2);

            rsx! { div {} }
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }
}
