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
    provider::{resolve_locale, use_intl_backend, use_messages},
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
///
///      view! {
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

    let intl_backend = use_intl_backend();

    let messages = use_messages::<M::Messages>(None, Some(&locale));

    let env = Env {
        locale,
        intl_backend,
    };

    // Create the service once — runs only on component initialization.
    let service = StoredValue::new(Service::<M>::new(props, &env, &messages));

    // Create a signal tracking the current state.
    let initial_state = service.with_value(|s| s.state().clone());

    let (state_read, state_write) = signal(initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let (context_version_read, context_version_write) = signal(0u64);

    let effect_cleanups = StoredValue::new_local(HashMap::new());

    let send_ref = StoredValue::new(None);

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
mod test_support;

#[cfg(test)]
#[path = "../../tests/unit/use_machine.rs"]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests;
