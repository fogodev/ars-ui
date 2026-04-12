//! Reactive hook bridging [`ars_core::Machine`] to Leptos signals.
//!
//! The [`use_machine`] hook creates a [`Service`] instance, wraps it in Leptos
//! reactive primitives, and returns a [`UseMachineReturn`] handle for reading
//! state, sending events, and deriving fine-grained reactive values.

use std::{
    collections::HashMap,
    fmt::{self, Debug},
};

use ars_core::{CleanupFn, Env, HasId, Machine, Service};
use leptos::{prelude::*, reactive::owner::LocalStorage};
#[cfg(any(all(test, target_arch = "wasm32"), not(feature = "ssr")))]
use {ars_core::StrongSend, std::sync::Arc};

use crate::{
    ephemeral::EphemeralRef,
    provider::{resolve_locale, use_icu_provider, use_messages},
    use_id,
};

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

impl<M: Machine + 'static> Debug for UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
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
    M::Messages: Send + Sync + 'static,
{
    let (result, ..) = use_machine_inner::<M>(props);
    result
}

/// Creates a machine that watches an external props signal for changes.
///
/// When the props signal changes, the existing service is synchronized via
/// `Service::set_props()`, preserving the current machine instance while
/// updating state, derived context, and effect cleanups as needed.
pub fn use_machine_with_reactive_props<M: Machine + 'static>(
    props_signal: Signal<M::Props>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
    M::Messages: Send + Sync + 'static,
{
    let initial_props = props_signal.get();
    let (result, context_version_write, state_write, send_ref, effect_cleanups) =
        use_machine_inner::<M>(initial_props);

    let service = result.service;
    let prev_props = StoredValue::<Option<M::Props>>::new(None);

    let sync_effect = ImmediateEffect::new_isomorphic(move || {
        let new_props = props_signal.get();
        let should_sync = prev_props.with_value(|prev| prev.as_ref() != Some(&new_props));
        if should_sync {
            let is_initial = prev_props.with_value(Option::is_none);
            if !is_initial {
                let mut extracted = None;
                service.update_value(|svc| {
                    let send_result = svc.set_props(new_props.clone());
                    if send_result.state_changed {
                        state_write.set(svc.state().clone());
                    }
                    if send_result.context_changed {
                        context_version_write.update(|version| *version += 1);
                    }
                    let ctx = svc.context().clone();
                    let props = svc.props().clone();
                    extracted = Some((send_result, ctx, props));
                });

                let (send_result, ctx, props) =
                    extracted.expect("service update should extract send result");
                #[cfg(feature = "ssr")]
                handle_effects::<M>(&send_result, &ctx, &props, send_ref, effect_cleanups);
                #[cfg(not(feature = "ssr"))]
                handle_effects::<M>(send_result, &ctx, &props, send_ref, effect_cleanups);
            }
            prev_props.set_value(Some(new_props));
        }
    });
    on_cleanup(move || drop(sync_effect));

    result
}

type EffectCleanupStore = StoredValue<HashMap<&'static str, CleanupFn>, LocalStorage>;
type SendCallbackRef<M> = StoredValue<Option<Callback<<M as Machine>::Event>>>;
type UseMachineInnerParts<M> = (
    UseMachineReturn<M>,
    WriteSignal<u64>,
    WriteSignal<<M as Machine>::State>,
    SendCallbackRef<M>,
    EffectCleanupStore,
);

/// Internal implementation shared between public hooks.
///
/// Returns the public `UseMachineReturn` plus internal write handles needed by
/// `use_machine_with_reactive_props` for syncing external prop changes.
fn use_machine_inner<M: Machine + 'static>(props: M::Props) -> UseMachineInnerParts<M>
where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
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
    let messages = use_messages::<M::Messages>(None, Some(&locale));
    let env = Env {
        locale,
        icu_provider,
    };

    // Create the service once — runs only on component initialization.
    let service = StoredValue::new(Service::<M>::new(props, &env, &messages));

    // Create a signal tracking the current state.
    let initial_state = service.with_value(|s| s.state().clone());
    let (state_read, state_write) = signal(initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let (context_version_read, context_version_write) = signal(0u64);

    let effect_cleanups: EffectCleanupStore = StoredValue::new_local(HashMap::new());
    let send_ref: SendCallbackRef<M> = StoredValue::new(None);

    // Build the send callback. When an event is sent:
    // 1. Snapshot the old state for comparison
    // 2. Forward the event to Service::send()
    // 3. Update signals if state/context changed
    let send = Callback::new(move |event: M::Event| {
        let mut extracted = None;
        service.update_value(|s| {
            let send_result = s.send(event);
            if send_result.state_changed {
                state_write.set(s.state().clone());
            }
            if send_result.context_changed {
                context_version_write.update(|version| *version += 1);
            }
            let ctx = s.context().clone();
            let props = s.props().clone();
            extracted = Some((send_result, ctx, props));
        });

        let (send_result, ctx, props) = extracted.expect("service update should extract result");
        #[cfg(feature = "ssr")]
        handle_effects::<M>(&send_result, &ctx, &props, send_ref, effect_cleanups);
        #[cfg(not(feature = "ssr"))]
        handle_effects::<M>(send_result, &ctx, &props, send_ref, effect_cleanups);
    });

    send_ref.set_value(Some(send));

    // Clean up effects when the component unmounts.
    on_cleanup(move || {
        let mut cleanups = Vec::new();
        effect_cleanups.update_value(|active| {
            cleanups.extend(active.drain().map(|(_, cleanup)| cleanup));
        });
        service.update_value(|svc| {
            svc.unmount(cleanups);
        });
    });

    let result = UseMachineReturn {
        state: state_read,
        send,
        service,
        context_version: context_version_read,
    };

    (
        result,
        context_version_write,
        state_write,
        send_ref,
        effect_cleanups,
    )
}

#[cfg(any(all(test, target_arch = "wasm32"), not(feature = "ssr")))]
fn callback_to_strong_send<E: Send + Sync + 'static>(callback: Callback<E>) -> StrongSend<E> {
    Arc::new(move |event| callback.run(event))
}

#[cfg(feature = "ssr")]
const fn handle_effects<M: Machine + 'static>(
    send_result: &ars_core::SendResult<M>,
    ctx: &M::Context,
    props: &M::Props,
    send_ref: SendCallbackRef<M>,
    effect_cleanups: EffectCleanupStore,
) where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
{
    let _ = (send_result, ctx, props, send_ref, effect_cleanups);
}

#[cfg(not(feature = "ssr"))]
fn handle_effects<M: Machine + 'static>(
    send_result: ars_core::SendResult<M>,
    ctx: &M::Context,
    props: &M::Props,
    send_ref: SendCallbackRef<M>,
    effect_cleanups: EffectCleanupStore,
) where
    M::State: Clone + PartialEq + Send + Sync + 'static,
    M::Context: Clone + Send + Sync + 'static,
    M::Props: Clone + PartialEq + Send + Sync + 'static,
    M::Event: Send + Sync + 'static,
{
    if send_result.state_changed {
        effect_cleanups.update_value(|cleanups| {
            for (_, cleanup) in cleanups.drain() {
                cleanup();
            }
        });
    } else if !send_result.cancel_effects.is_empty() || !send_result.pending_effects.is_empty() {
        effect_cleanups.update_value(|cleanups| {
            for name in send_result.cancel_effects.iter().copied() {
                if let Some(cleanup) = cleanups.remove(name) {
                    cleanup();
                }
            }
            for effect in &send_result.pending_effects {
                if let Some(cleanup) = cleanups.remove(effect.name) {
                    cleanup();
                }
            }
        });
    }

    if send_result.pending_effects.is_empty() {
        return;
    }

    if let Some(send_callback) = send_ref.with_value(|slot| slot.as_ref().copied()) {
        let send_handle = callback_to_strong_send(send_callback);
        for effect in send_result.pending_effects {
            let name = effect.name;
            let cleanup = effect.run(ctx, props, Arc::clone(&send_handle));
            effect_cleanups.update_value(|cleanups| {
                cleanups.insert(name, cleanup);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    #[cfg(not(feature = "ssr"))]
    use std::sync::Mutex;

    #[cfg(not(feature = "ssr"))]
    use ars_core::PendingEffect;
    use ars_core::{
        AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr, I18nRegistries, IcuProvider,
        NullPlatformEffects, TransitionPlan,
    };
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
        type Messages = ();
        type Api<'a> = ToggleApi<'a>;

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
        ) -> Option<TransitionPlan<Self>> {
            match state {
                ToggleState::Off => Some(TransitionPlan::to(ToggleState::On)),
                ToggleState::On => Some(TransitionPlan::to(ToggleState::Off)),
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
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn with_api_snapshot_rejects_callback_sends_events() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let owner = Owner::new();
            owner.with(|| {
                let machine = use_machine::<ToggleMachine>(ToggleProps {
                    id: String::from("toggle"),
                });

                machine.with_api_snapshot(|api| api.trigger_toggle());
            });
        }));

        #[cfg(debug_assertions)]
        assert!(result.is_err());
        #[cfg(not(debug_assertions))]
        assert!(result.is_ok());
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
    fn derive_rejects_callback_sends_events() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
        }));

        #[cfg(debug_assertions)]
        assert!(result.is_err());
        #[cfg(not(debug_assertions))]
        assert!(result.is_ok());
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

    #[derive(Clone)]
    struct TestIcuProvider;

    impl Debug for TestIcuProvider {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("TestIcuProvider")
        }
    }

    impl IcuProvider for TestIcuProvider {}

    #[derive(Clone)]
    struct EnvContext {
        locale: String,
        icu_provider: Arc<dyn IcuProvider>,
    }

    impl Debug for EnvContext {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("EnvContext")
                .field("locale", &self.locale)
                .field("icu_provider", &"Arc(..)")
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
        type Messages = ();
        type Api<'a> = EnvApi;

        fn init(
            _props: &Self::Props,
            env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (
                (),
                EnvContext {
                    locale: env.locale.to_bcp47(),
                    icu_provider: Arc::clone(&env.icu_provider),
                },
            )
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
            EnvApi
        }
    }

    struct EnvApi;

    impl ConnectApi for EnvApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
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

    #[derive(Clone, Debug)]
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
        ) -> Option<TransitionPlan<Self>> {
            match event {
                PropEvent::SetChecked(checked) => Some(TransitionPlan::to(if *checked {
                    PropState::On
                } else {
                    PropState::Off
                })),
                PropEvent::SyncLabel => {
                    Some(TransitionPlan::new().apply(|ctx: &mut PropContext| {
                        ctx.sync_count += 1;
                    }))
                }
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

    struct PropApi {
        is_on: bool,
        sync_count: u32,
    }

    impl PropApi {
        const fn sync_count(&self) -> u32 {
            self.sync_count
        }
    }

    impl ConnectApi for PropApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            let mut attrs = AttrMap::new();
            attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), self.is_on.to_string());
            attrs
        }
    }

    fn provide_test_context(locale: &str, icu_provider: Arc<dyn IcuProvider>) {
        crate::provide_ars_context(crate::ArsContext::new(
            ars_i18n::Locale::parse(locale).expect("locale should parse"),
            ars_i18n::Direction::Ltr,
            ars_core::ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            icu_provider,
            Arc::new(I18nRegistries::new()),
            ars_core::StyleStrategy::Inline,
        ));
    }

    #[test]
    fn use_machine_inner_resolves_locale_and_icu_provider_from_context() {
        let owner = Owner::new();
        owner.with(|| {
            let expected_provider: Arc<dyn IcuProvider> = Arc::new(TestIcuProvider);
            provide_test_context("es-ES", Arc::clone(&expected_provider));

            let machine = use_machine::<EnvMachine>(EnvProps { id: String::new() });
            machine.service.with_value(|service| {
                assert!(service.props().id().starts_with("component-"));
                assert_eq!(service.context().locale, "es-ES");
                assert!(Arc::ptr_eq(
                    &service.context().icu_provider,
                    &expected_provider
                ));
            });
        });
    }

    #[test]
    fn use_machine_injects_generated_id_when_props_id_is_empty() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

            machine.service.with_value(|service| {
                assert!(service.props().id().starts_with("component-"));
            });
        });
    }

    #[test]
    fn use_machine_with_reactive_props_syncs_state_and_context_changes() {
        let owner = Owner::new();
        owner.with(|| {
            let (props, set_props) = signal(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });

            let machine = use_machine_with_reactive_props::<PropMachine>(props.into());
            assert_eq!(machine.state.get_untracked(), PropState::Off);
            assert_eq!(machine.context_version.get_untracked(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "a",
            });
            assert_eq!(machine.state.get_untracked(), PropState::On);
            assert_eq!(machine.context_version.get_untracked(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "b",
            });
            assert_eq!(machine.state.get_untracked(), PropState::On);
            assert_eq!(machine.context_version.get_untracked(), 1);
            machine.service.with_value(|service| {
                assert_eq!(service.context().sync_count, 1);
            });
        });
    }

    #[test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn derive_recomputes_when_only_context_changes() {
        let owner = Owner::new();
        owner.with(|| {
            let (props, set_props) = signal(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });
            let machine = use_machine_with_reactive_props::<PropMachine>(props.into());
            let sync_count = machine.derive(|api| api.sync_count());

            assert_eq!(sync_count.get(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "b",
            });

            assert_eq!(machine.state.get_untracked(), PropState::Off);
            assert_eq!(machine.context_version.get_untracked(), 1);
            assert_eq!(sync_count.get(), 1);
        });
    }

    #[test]
    fn context_version_only_increments_on_context_changes() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(machine.context_version.get_untracked(), 0);

            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.context_version.get_untracked(), 0);

            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.context_version.get_untracked(), 0);
        });
    }

    #[cfg(not(feature = "ssr"))]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum EffectState {
        Idle,
        Active,
    }

    #[cfg(not(feature = "ssr"))]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum EffectEvent {
        Start,
        Replace,
        Cancel,
        Stop,
        StartNotify,
        Notify,
    }

    #[cfg(not(feature = "ssr"))]
    #[derive(Clone, Debug)]
    struct EffectContext {
        log: Arc<Mutex<Vec<&'static str>>>,
        notify_count: u32,
    }

    #[cfg(not(feature = "ssr"))]
    #[derive(Clone, Debug)]
    struct EffectProps {
        id: String,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    #[cfg(not(feature = "ssr"))]
    impl PartialEq for EffectProps {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    #[cfg(not(feature = "ssr"))]
    impl Eq for EffectProps {}

    #[cfg(not(feature = "ssr"))]
    impl HasId for EffectProps {
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

    #[cfg(not(feature = "ssr"))]
    struct EffectApi;

    #[cfg(not(feature = "ssr"))]
    impl ConnectApi for EffectApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    #[cfg(not(feature = "ssr"))]
    struct EffectMachine;

    #[cfg(not(feature = "ssr"))]
    impl Machine for EffectMachine {
        type State = EffectState;
        type Event = EffectEvent;
        type Context = EffectContext;
        type Props = EffectProps;
        type Messages = ();
        type Api<'a> = EffectApi;

        fn init(
            props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (
                EffectState::Idle,
                EffectContext {
                    log: Arc::clone(&props.log),
                    notify_count: 0,
                },
            )
        }

        fn transition(
            _state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<TransitionPlan<Self>> {
            match event {
                EffectEvent::Start => Some(
                    TransitionPlan::to(EffectState::Active).with_effect(tracked_effect(
                        "timer",
                        "setup:start",
                        "cleanup:start",
                    )),
                ),
                EffectEvent::Replace => Some(TransitionPlan::new().with_effect(tracked_effect(
                    "timer",
                    "setup:replace",
                    "cleanup:replace",
                ))),
                EffectEvent::Cancel => Some(TransitionPlan::new().cancel_effect("timer")),
                EffectEvent::Stop => Some(TransitionPlan::to(EffectState::Idle)),
                EffectEvent::StartNotify => Some(
                    TransitionPlan::to(EffectState::Active).with_effect(PendingEffect::new(
                        "notify",
                        |ctx: &EffectContext, _props: &EffectProps, send| {
                            ctx.log
                                .lock()
                                .expect("log mutex should not be poisoned")
                                .push("setup:notify");
                            send.call_if_alive(EffectEvent::Notify);
                            let log = Arc::clone(&ctx.log);
                            Box::new(move || {
                                log.lock()
                                    .expect("log mutex should not be poisoned")
                                    .push("cleanup:notify");
                            })
                        },
                    )),
                ),
                EffectEvent::Notify => {
                    Some(TransitionPlan::new().apply(|ctx: &mut EffectContext| {
                        ctx.notify_count += 1;
                    }))
                }
            }
        }

        fn connect<'a>(
            _state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            EffectApi
        }
    }

    #[cfg(not(feature = "ssr"))]
    fn tracked_effect(
        name: &'static str,
        setup_label: &'static str,
        cleanup_label: &'static str,
    ) -> PendingEffect<EffectMachine> {
        PendingEffect::new(
            name,
            move |ctx: &EffectContext, _props: &EffectProps, _send| {
                ctx.log
                    .lock()
                    .expect("log mutex should not be poisoned")
                    .push(setup_label);
                let log = Arc::clone(&ctx.log);
                Box::new(move || {
                    log.lock()
                        .expect("log mutex should not be poisoned")
                        .push(cleanup_label);
                })
            },
        )
    }

    #[cfg(not(feature = "ssr"))]
    fn effect_log(log: &Arc<Mutex<Vec<&'static str>>>) -> Vec<&'static str> {
        log.lock()
            .expect("log mutex should not be poisoned")
            .clone()
    }

    #[cfg(not(feature = "ssr"))]
    #[test]
    fn effect_lifecycle_replaces_cancels_and_unmounts_cleanups() {
        let owner = Owner::new();
        owner.with(|| {
            let log = Arc::new(Mutex::new(Vec::new()));
            let machine = use_machine::<EffectMachine>(EffectProps {
                id: String::from("effects"),
                log: Arc::clone(&log),
            });

            machine.send.run(EffectEvent::Start);
            assert_eq!(effect_log(&log), vec!["setup:start"]);

            machine.send.run(EffectEvent::Replace);
            assert_eq!(
                effect_log(&log),
                vec!["setup:start", "cleanup:start", "setup:replace"]
            );

            machine.send.run(EffectEvent::Cancel);
            assert_eq!(
                effect_log(&log),
                vec![
                    "setup:start",
                    "cleanup:start",
                    "setup:replace",
                    "cleanup:replace",
                ]
            );

            machine.send.run(EffectEvent::Start);
            assert_eq!(
                effect_log(&log),
                vec![
                    "setup:start",
                    "cleanup:start",
                    "setup:replace",
                    "cleanup:replace",
                    "setup:start",
                ]
            );

            owner.cleanup();
            assert_eq!(
                effect_log(&log),
                vec![
                    "setup:start",
                    "cleanup:start",
                    "setup:replace",
                    "cleanup:replace",
                    "setup:start",
                    "cleanup:start",
                ]
            );
        });
    }

    #[cfg(not(feature = "ssr"))]
    #[test]
    fn state_changes_drain_existing_effect_cleanups() {
        let owner = Owner::new();
        owner.with(|| {
            let log = Arc::new(Mutex::new(Vec::new()));
            let machine = use_machine::<EffectMachine>(EffectProps {
                id: String::from("effects"),
                log: Arc::clone(&log),
            });

            machine.send.run(EffectEvent::Start);
            machine.send.run(EffectEvent::Stop);

            assert_eq!(effect_log(&log), vec!["setup:start", "cleanup:start"]);
            assert_eq!(machine.state.get_untracked(), EffectState::Idle);
        });
    }

    #[cfg(not(feature = "ssr"))]
    #[test]
    fn effect_send_handle_dispatches_follow_up_events() {
        let owner = Owner::new();
        owner.with(|| {
            let log = Arc::new(Mutex::new(Vec::new()));
            let machine = use_machine::<EffectMachine>(EffectProps {
                id: String::from("effects"),
                log: Arc::clone(&log),
            });

            machine.send.run(EffectEvent::StartNotify);

            assert_eq!(machine.state.get_untracked(), EffectState::Active);
            assert_eq!(machine.context_version.get_untracked(), 1);
            machine.service.with_value(|service| {
                assert_eq!(service.context().notify_count, 1);
            });
            assert_eq!(effect_log(&log), vec!["setup:notify"]);
        });
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::sync::{Arc, Mutex};

    use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr, TransitionPlan};
    use leptos::reactive::traits::GetUntracked;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn callback_to_strong_send_uses_wasm_send_handle() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let callback_calls = Arc::clone(&calls);
        let callback = Callback::new(move |event: i32| {
            callback_calls
                .lock()
                .expect("mutex should not be poisoned")
                .push(event);
        });

        let strong = callback_to_strong_send(callback);
        strong(7);
        strong(9);

        assert_eq!(
            calls
                .lock()
                .expect("mutex should not be poisoned")
                .as_slice(),
            &[7, 9]
        );
    }

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

    struct ToggleApi;

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
        ) -> Option<TransitionPlan<Self>> {
            match state {
                ToggleState::Off => Some(TransitionPlan::to(ToggleState::On)),
                ToggleState::On => Some(TransitionPlan::to(ToggleState::Off)),
            }
        }

        fn connect<'a>(
            _state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi
        }
    }

    #[wasm_bindgen_test]
    fn use_machine_updates_state_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps {
                id: String::from("toggle"),
            });

            assert_eq!(machine.state.get_untracked(), ToggleState::Off);
            machine.send.run(ToggleEvent::Toggle);
            assert_eq!(machine.state.get_untracked(), ToggleState::On);
        });
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

    #[derive(Clone, Debug)]
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
        const fn sync_count(&self) -> u32 {
            self.sync_count
        }
    }

    impl ConnectApi for PropApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            let mut attrs = AttrMap::new();
            attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), self.is_on.to_string());
            attrs
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
        ) -> Option<TransitionPlan<Self>> {
            match event {
                PropEvent::SetChecked(checked) => Some(TransitionPlan::to(if *checked {
                    PropState::On
                } else {
                    PropState::Off
                })),
                PropEvent::SyncLabel => {
                    Some(TransitionPlan::new().apply(|ctx: &mut PropContext| {
                        ctx.sync_count += 1;
                    }))
                }
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

    #[wasm_bindgen_test]
    fn reactive_props_sync_state_and_context_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (props, set_props) = signal(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });

            let machine = use_machine_with_reactive_props::<PropMachine>(props.into());
            assert_eq!(machine.state.get_untracked(), PropState::Off);
            assert_eq!(machine.context_version.get_untracked(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "a",
            });
            assert_eq!(machine.state.get_untracked(), PropState::On);
            assert_eq!(machine.context_version.get_untracked(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: true,
                label: "b",
            });
            assert_eq!(machine.context_version.get_untracked(), 1);
            machine.service.with_value(|service| {
                assert_eq!(service.context().sync_count, 1);
            });
        });
    }

    #[wasm_bindgen_test]
    fn use_machine_injects_generated_id_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let machine = use_machine::<ToggleMachine>(ToggleProps { id: String::new() });

            machine.service.with_value(|service| {
                assert!(service.props().id().starts_with("component-"));
            });
        });
    }

    #[wasm_bindgen_test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "Method pointers are not general enough for the lifetime-parameterized test API."
    )]
    fn derive_recomputes_when_only_context_changes_on_wasm() {
        let owner = Owner::new();
        owner.with(|| {
            let (props, set_props) = signal(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "a",
            });
            let machine = use_machine_with_reactive_props::<PropMachine>(props.into());
            let sync_count = machine.derive(|api| api.sync_count());

            assert_eq!(sync_count.get(), 0);

            set_props.set(PropProps {
                id: String::from("toggle"),
                checked: false,
                label: "b",
            });

            assert_eq!(machine.state.get_untracked(), PropState::Off);
            assert_eq!(machine.context_version.get_untracked(), 1);
            assert_eq!(sync_count.get(), 1);
        });
    }
}
