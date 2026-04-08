//! Reactive hook bridging [`ars_core::Machine`] to Leptos signals.
//!
//! The [`use_machine`] hook creates a [`Service`] instance, wraps it in Leptos
//! reactive primitives, and returns a [`UseMachineReturn`] handle for reading
//! state, sending events, and deriving fine-grained reactive values.

use ars_core::{Machine, Service};
use leptos::prelude::*;

use crate::ephemeral::EphemeralRef;

/// Return type from [`use_machine`].
///
/// Provides reactive access to a running [`Machine`] instance. All fields are
/// `Copy` (arena-allocated Leptos handles), so this struct can be freely passed
/// into closures without cloning.
///
/// # Reactive contract
///
/// - Reading [`state`](Self::state) in a reactive scope subscribes to state changes.
/// - Calling [`send`](Self::send) dispatches an event and may update state/context.
/// - [`derive()`](Self::derive) creates fine-grained memos from the connect API.
/// - [`with_api_snapshot()`](Self::with_api_snapshot) provides one-shot imperative access.
pub struct UseMachineReturn<M: Machine + 'static>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
{
    /// Reactive signal for the current machine state.
    /// Reading it inside a reactive scope creates a dependency.
    pub state: ReadSignal<M::State>,

    /// Send an event to the machine.
    /// Safe to call from any closure — does not require reactive scope.
    pub send: Callback<M::Event>,

    /// Access the full service (context + state) via a `StoredValue`.
    /// Use sparingly — prefer [`derive()`](Self::derive) for reactive data and
    /// [`with_api_ephemeral()`](Self::with_api_ephemeral) for imperative access.
    pub service: StoredValue<Service<M>>,

    /// Monotonically increasing counter that increments whenever context changes.
    /// Used by [`derive()`](Self::derive) to track context mutations even when
    /// state remains the same.
    pub context_version: ReadSignal<u64>,
}

// Manual Clone/Copy impls to avoid requiring M: Clone/Copy — all fields are
// arena-allocated Leptos handles that are always Copy regardless of M.
impl<M: Machine + 'static> Clone for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Machine + 'static> Copy for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
{
}

impl<M: Machine + 'static> std::fmt::Debug for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
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
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
{
    /// Gets a one-shot snapshot of the connect API.
    ///
    /// **Prefer [`derive()`](Self::derive) for reactive data** — this method does
    /// not track dependencies. Use `with_api_snapshot` only for imperative
    /// operations (e.g., reading a value once inside an event handler).
    ///
    /// The connect closure uses a panic callback because `with_value` holds an
    /// immutable borrow on the `StoredValue`, preventing `send` (which calls
    /// `update_value`) from re-entering. Sending events from API snapshots would
    /// cause a re-entrant borrow panic.
    pub fn with_api_snapshot<T>(&self, f: impl Fn(&M::Api<'_>) -> T) -> T {
        self.service.with_value(|svc| {
            let api = svc.connect(&|_e| {
                #[cfg(debug_assertions)]
                panic!("Cannot send events inside with_api_snapshot — use event handlers instead");
            });
            f(&api)
        })
    }

    /// Creates a fine-grained memo that derives a value from the connect API.
    ///
    /// Only re-computes when the underlying state or context changes, and only
    /// notifies dependents when the derived value actually changes.
    ///
    /// **Important:** `Api<'a>` has a non-`'static` lifetime and cannot be stored
    /// in Leptos signals. The `&M::Api<'_>` reference passed to the closure is
    /// valid only for that closure call. Extract the values you need (strings,
    /// booleans, [`AttrMap`](ars_core::AttrMap)) and return them.
    ///
    /// The closure must not call `send()` — it is a read-only projection.
    pub fn derive<T: Clone + PartialEq + Send + Sync + 'static>(
        &self,
        f: impl Fn(&M::Api<'_>) -> T + Send + Sync + 'static,
    ) -> Memo<T> {
        let state = self.state;
        let context_version = self.context_version;
        let service = self.service;
        Memo::new(move |_| {
            // Subscribe to both state and context_version so the memo
            // re-computes when either changes.
            state.track();
            context_version.track();
            service.with_value(|svc| {
                let api = svc.connect(&|_e| {
                    #[cfg(debug_assertions)]
                    panic!("Cannot send events inside derive() — use event handlers instead");
                });
                f(&api)
            })
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
        self.service.with_value(|svc| {
            let send_fn = move |e| send.run(e);
            let api = svc.connect(&send_fn);
            f(EphemeralRef::new(api))
        })
    }
}

/// Creates and manages a machine service with Leptos reactivity.
///
/// This is the central primitive for using `ars-core` machines in Leptos components.
/// It creates a [`Service<M>`], wraps it in reactive signals, and returns a
/// [`UseMachineReturn`] handle.
///
/// # Example
///
/// ```rust,ignore
/// #[component]
/// pub fn Toggle() -> impl IntoView {
///     let machine = use_machine::<toggle::Machine>(toggle::Props::default());
///     let is_on = machine.derive(|api| api.is_on());
///     view! {
///         <button on:click=move |_| machine.send.run(toggle::Event::Toggle)>
///             {move || if is_on.get() { "ON" } else { "OFF" }}
///         </button>
///     }
/// }
/// ```
pub fn use_machine<M: Machine + 'static>(props: M::Props) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
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
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
{
    todo!("Requires Service::set_props() — blocked on ars-core SendResult addition")
}

/// Internal implementation shared between public hooks.
///
/// Returns the public `UseMachineReturn` plus internal write handles needed by
/// `use_machine_with_reactive_props` for syncing external prop changes.
fn use_machine_inner<M: Machine + 'static>(
    props: M::Props,
) -> (UseMachineReturn<M>, WriteSignal<u64>, WriteSignal<M::State>)
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
{
    // Create the service once — runs only on component initialization.
    let service = StoredValue::new(Service::<M>::new(props));

    // Create a signal tracking the current state.
    let initial_state = service.with_value(|s| s.state().clone());
    let (state_read, state_write) = signal(initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let (context_version_read, context_version_write) = signal(0u64);

    // Build the send callback. When an event is sent:
    // 1. Snapshot the old state for comparison
    // 2. Forward the event to Service::send()
    // 3. Update signals if state/context changed
    //
    let send: Callback<M::Event> = Callback::new(move |event: M::Event| {
        // StoredValue::update_value returns (), so extract result via side-channel.
        let mut state_changed = false;
        let mut context_changed = false;
        service.update_value(|s| {
            let result = s.send(event);
            state_changed = result.state_changed;
            context_changed = result.context_changed;
            // TODO: Dispatch result.pending_effects and handle result.cancel_effects
            // when component implementations need effect lifecycle management.
        });

        if state_changed {
            let new_state = service.with_value(|s| s.state().clone());
            state_write.set(new_state);
        }

        if state_changed || context_changed {
            context_version_write.update(|v| *v += 1);
        }
    });

    // Clean up effects when the component unmounts.
    on_cleanup(move || {
        // Placeholder: when effect dispatch is implemented, drain all
        // active effect cleanups here in LIFO order.
    });

    let result = UseMachineReturn {
        state: state_read,
        send,
        service,
        context_version: context_version_read,
    };

    (result, context_version_write, state_write)
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};
    use leptos::reactive::traits::Get;

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

    struct ToggleApi<'a> {
        is_on: bool,
        send: &'a dyn Fn(ToggleEvent),
    }

    impl ToggleApi<'_> {
        fn is_on(&self) -> bool {
            self.is_on
        }

        fn trigger_toggle(&self) {
            (self.send)(ToggleEvent::Toggle);
        }
    }

    impl ConnectApi for ToggleApi<'_> {
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
        type Api<'a> = ToggleApi<'a>;

        fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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
            send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi {
                is_on: *state == ToggleState::On,
                send,
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
    fn use_machine_return_type_clone_delegates_to_copy_fields() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });
            let cloned = <UseMachineReturn<ToggleMachine> as Clone>::clone(&machine);

            cloned.send.run(ToggleEvent::Toggle);

            assert_eq!(machine.state.get_untracked(), ToggleState::On);
            assert_eq!(cloned.state.get_untracked(), ToggleState::On);
        });
    }

    #[test]
    fn use_machine_return_debug_names_the_type() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let debug = format!("{machine:?}");
            assert!(debug.contains("UseMachineReturn"));
            assert!(debug.contains("context_version"));
        });
    }

    #[test]
    fn use_machine_creates_service_with_initial_state() {
        // Test use_machine within a Leptos reactive Owner.
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            // Initial state should be Off
            assert_eq!(machine.state.get_untracked(), ToggleState::Off);

            // Context version starts at 0
            assert_eq!(machine.context_version.get_untracked(), 0);
        });
    }

    #[test]
    fn use_machine_send_updates_state() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(machine.state.get_untracked(), ToggleState::Off);

            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.state.get_untracked(), ToggleState::On);

            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.state.get_untracked(), ToggleState::Off);
        });
    }

    #[test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn with_api_snapshot_reads_current_state() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let is_on = machine.with_api_snapshot(|api| api.is_on());
            assert!(!is_on);

            machine.send.run(ToggleEvent::Toggle);

            let is_on = machine.with_api_snapshot(|api| api.is_on());
            assert!(is_on);
        });
    }

    #[test]
    #[should_panic(expected = "Cannot send events inside with_api_snapshot")]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn with_api_snapshot_panics_when_callback_sends_events() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            machine.with_api_snapshot(|api| api.trigger_toggle());
        });
    }

    #[test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn derive_tracks_connect_output() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });
            let is_on = machine.derive(|api| api.is_on());

            assert!(!is_on.get());

            machine.send.run(ToggleEvent::Toggle);

            assert!(is_on.get());
        });
    }

    #[test]
    #[should_panic(expected = "Cannot send events inside derive()")]
    fn derive_panics_when_callback_sends_events() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });
            let derived = machine.derive(|api| {
                api.trigger_toggle();
                api.is_on()
            });

            let _ = derived.get();
        });
    }

    #[test]
    fn with_api_ephemeral_reads_current_state() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            let is_on = machine.with_api_ephemeral(|api| api.get().is_on());
            assert!(!is_on);

            machine.send.run(ToggleEvent::Toggle);

            let attrs = machine.with_api_ephemeral(|api| api.get().part_attrs(TogglePart));
            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed)), Some("true"));
        });
    }

    #[test]
    #[should_panic(expected = "Requires Service::set_props()")]
    fn use_machine_with_reactive_props_is_explicitly_unimplemented() {
        let owner = Owner::new();
        owner.with(|| {
            let props = Signal::stored(ToggleProps {
                id: String::from("toggle"),
            });

            let _ = use_machine_with_reactive_props::<ToggleMachine>(props);
        });
    }

    #[test]
    fn context_version_increments_on_transition() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(machine.context_version.get_untracked(), 0);

            machine.send.run(ToggleEvent::Toggle);
            // Conservative: bumps on any applied transition
            assert_eq!(machine.context_version.get_untracked(), 1);

            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.context_version.get_untracked(), 2);
        });
    }
}
