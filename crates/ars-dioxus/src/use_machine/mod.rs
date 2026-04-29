//! Reactive hook bridging [`ars_core::Machine`] to Dioxus signals.
//!
//! The [`use_machine`] hook creates a [`Service`] instance, wraps it in Dioxus
//! reactive primitives, and returns a [`UseMachineReturn`] handle for reading
//! state, sending events, and deriving fine-grained reactive values.

use std::{
    collections::HashMap,
    fmt::{self, Debug},
    sync::{Arc, Mutex},
};

use ars_core::{CleanupFn, Env, HasId, Machine, RenderMode, Service};
use dioxus::prelude::*;

use crate::{
    ephemeral::EphemeralRef,
    provider::{resolve_locale, use_intl_backend, use_messages},
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

const fn current_render_mode() -> RenderMode {
    if cfg!(feature = "ssr") {
        RenderMode::Server
    } else {
        RenderMode::Client
    }
}

struct MachineRuntime<M: Machine + 'static>
where
    M::State: Clone + PartialEq + 'static,
{
    service: Signal<Service<M>>,
    state: Signal<M::State>,
    context_version: Signal<u64>,
    effect_cleanups: Signal<HashMap<&'static str, CleanupFn>>,
    pending_events: Arc<Mutex<Vec<M::Event>>>,
}

impl<M: Machine + 'static> Clone for MachineRuntime<M>
where
    M::State: Clone + PartialEq + 'static,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service,
            state: self.state,
            context_version: self.context_version,
            effect_cleanups: self.effect_cleanups,
            pending_events: Arc::clone(&self.pending_events),
        }
    }
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

impl<M: Machine + 'static> Debug for UseMachineReturn<M>
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
///
///     let is_on = machine.derive(|api| api.is_on());
///
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
    M::Event: Send + 'static,
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
    M::Event: Send + 'static,
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
    M::Event: Send + 'static,
    M::Messages: Send + Sync + 'static,
{
    let generated_id = use_hook(|| use_id("component"));

    let props = {
        let mut props = props;

        if props.id().is_empty() {
            props.set_id(generated_id);
        }

        props
    };

    let locale = resolve_locale(None);

    let intl_backend = use_intl_backend();

    let env = Env::new(locale, intl_backend).with_render_mode(current_render_mode());

    let messages = use_messages::<M::Messages>(None, Some(&env.locale));

    let props_for_sync = props.clone();

    // Create the service once — use_signal runs its closure only on first mount.
    let service_signal = use_signal(move || Service::<M>::new(props, &env, &messages));

    // Create a signal tracking the current state.
    // Use .peek() to avoid subscribing the component to service_signal changes.
    let initial_state = service_signal.peek().state().clone();

    let state_signal = use_signal::<M::State>(|| initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let context_version = use_signal(|| 0u64);

    let effect_cleanups = use_signal(HashMap::<&'static str, CleanupFn>::new);

    let pending_events = use_hook(|| Arc::new(Mutex::new(Vec::<M::Event>::new())));

    let runtime = MachineRuntime {
        service: service_signal,
        state: state_signal,
        context_version,
        effect_cleanups,
        pending_events: Arc::clone(&pending_events),
    };

    use_sync_props::<M>(props_for_sync, runtime.clone());

    // Build the send callback. When an event is sent:
    // 1. Snapshot the old state for comparison
    // 2. Forward the event to Service::send()
    // 3. Update signals if state/context changed
    //
    // use_hook runs its closure once on mount and returns the cached value on
    // re-renders. Callback is Copy, so the handle is stable. The captured
    // signal handles are Copy indirections that always access current data.
    let send_runtime = runtime.clone();
    let send = use_hook(|| {
        Callback::new(move |event: M::Event| {
            dispatch_event::<M>(event, send_runtime.clone());
        })
    });

    // Clean up effects when the component unmounts.
    let mut cleanup_runtime = runtime.clone();

    use_drop(move || {
        let cleanups = drain_effect_cleanups(cleanup_runtime.effect_cleanups);
        cleanup_runtime.service.write().unmount(cleanups);
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
fn use_sync_props<M: Machine + 'static>(current_props: M::Props, mut runtime: MachineRuntime<M>)
where
    M::Props: Clone + PartialEq + 'static,
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Event: Send + 'static,
{
    let mut prev_props = use_signal(|| None::<M::Props>);

    let previous = prev_props.peek().clone();

    if previous.as_ref() != Some(&current_props) {
        if previous.is_some() {
            let (send_result, ctx, props) = {
                let mut service = runtime.service.write();

                let send_result = service.set_props(current_props.clone());

                if send_result.state_changed {
                    runtime.state.set(service.state().clone());
                }

                if send_result.context_changed {
                    *runtime.context_version.write() += 1;
                }

                let ctx = service.context().clone();

                let props = service.props().clone();

                (send_result, ctx, props)
            };

            #[cfg(feature = "ssr")]
            handle_effects::<M>(&send_result, &ctx, &props, &runtime);

            #[cfg(not(feature = "ssr"))]
            handle_effects::<M>(send_result, &ctx, &props, runtime.clone());
        }

        prev_props.set(Some(current_props));
    }
}

fn dispatch_event<M: Machine + 'static>(event: M::Event, mut runtime: MachineRuntime<M>)
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Event: Send + 'static,
{
    let (result, ctx, props) = {
        let mut service = runtime.service.write();

        let result = service.send(event);

        if result.state_changed {
            runtime.state.set(service.state().clone());
        }

        if result.context_changed {
            *runtime.context_version.write() += 1;
        }

        let ctx = service.context().clone();

        let props = service.props().clone();

        (result, ctx, props)
    };

    #[cfg(feature = "ssr")]
    handle_effects::<M>(&result, &ctx, &props, &runtime);

    #[cfg(not(feature = "ssr"))]
    handle_effects::<M>(result, &ctx, &props, runtime);
}

fn drain_effect_cleanups(
    mut effect_cleanups: Signal<HashMap<&'static str, CleanupFn>>,
) -> Vec<CleanupFn> {
    let mut pending = Vec::new();

    for (_, cleanup) in effect_cleanups.write().drain() {
        pending.push(cleanup);
    }

    pending
}

#[cfg(feature = "ssr")]
const fn handle_effects<M: Machine + 'static>(
    send_result: &ars_core::SendResult<M>,
    ctx: &M::Context,
    props: &M::Props,
    runtime: &MachineRuntime<M>,
) {
    let _ = send_result;
    let _ = ctx;
    let _ = props;
    let _ = runtime;
}

#[cfg(not(feature = "ssr"))]
fn handle_effects<M: Machine + 'static>(
    send_result: ars_core::SendResult<M>,
    ctx: &M::Context,
    props: &M::Props,
    mut runtime: MachineRuntime<M>,
) where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Event: Send + 'static,
{
    let mut cleanups_to_run = Vec::new();

    if send_result.state_changed {
        cleanups_to_run.extend(drain_effect_cleanups(runtime.effect_cleanups));
    } else if !send_result.cancel_effects.is_empty() || !send_result.pending_effects.is_empty() {
        {
            let mut active_cleanups = runtime.effect_cleanups.write();

            for name in send_result.cancel_effects.iter().copied() {
                if let Some(cleanup) = active_cleanups.remove(name) {
                    cleanups_to_run.push(cleanup);
                }
            }

            for effect in &send_result.pending_effects {
                if let Some(cleanup) = active_cleanups.remove(effect.name) {
                    cleanups_to_run.push(cleanup);
                }
            }
        }
    }

    for cleanup in cleanups_to_run {
        cleanup();
    }

    if send_result.pending_effects.is_empty() {
        return;
    }

    let send_handle: Arc<dyn Fn(M::Event) + Send + Sync> = Arc::new({
        let pending_events = Arc::clone(&runtime.pending_events);

        move |event| {
            pending_events
                .lock()
                .expect("pending event queue mutex should not be poisoned")
                .push(event);
        }
    });

    for effect in send_result.pending_effects {
        let name = effect.name;

        let cleanup = effect.run(ctx, props, Arc::clone(&send_handle));

        runtime.effect_cleanups.write().insert(name, cleanup);
    }

    let queued_events = {
        let mut pending = runtime
            .pending_events
            .lock()
            .expect("pending event queue mutex should not be poisoned");

        pending.drain(..).collect::<Vec<_>>()
    };

    for event in queued_events {
        dispatch_event::<M>(event, runtime.clone());
    }
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "../../tests/unit/use_machine.rs"]
mod tests;

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests;
