# Dioxus Adapter Specification (`ars-dioxus`)

## 1. Overview

The `ars-dioxus` crate bridges `ars-core` state machines to Dioxus's signal + virtual DOM system.

> **DOM utilities:** Positioning, scroll lock, focus management, and z-index are specified in `11-dom-utilities.md`. The `ars-dom` crate is an optional dependency, enabled by the `web` feature flag.

### 1.1 Key Differences from Leptos Adapter

| Concern        | Leptos (`ars-leptos`)                                               | Dioxus (`ars-dioxus`)                                                                                                                                                                            |
| -------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Render model   | Fine-grained, no VDOM                                               | Component re-runs on signal change, VDOM diff                                                                                                                                                    |
| Signal type    | `ReadSignal<T>` / `WriteSignal<T>` (concrete types from `signal()`) | `Signal<T>` (combined read/write, `Copy`); `ReadSignal<T>` (read-only projection; `Signal<T>` and `Memo<T>` convert into it via `From`); `Memo<T>` (derived computation, returned by `derive()`) |
| Components run | Once only                                                           | When signals read in the component body change (VDOM diff)                                                                                                                                       |
| Context        | `provide_context` / `use_context`                                   | `use_context_provider` / `use_context`¹                                                                                                                                                          |
| Children       | `Children` (Box fn)                                                 | `Element`                                                                                                                                                                                        |
| Named slots    | `#[slot]` macro                                                     | Separate props or explicit `Element` fields                                                                                                                                                      |
| SSR            | `leptos/ssr` feature                                                | `ssr` ars-dioxus feature (enables `dioxus/server` internally)                                                                                                                                    |
| Platforms      | Web (WASM)                                                          | Web, Desktop, Mobile, SSR                                                                                                                                                                        |
| Cleanup        | `on_cleanup`                                                        | `use_drop` hook                                                                                                                                                                                  |

> ¹ Adapter convention: child parts use `try_use_context::<T>().expect("descriptive msg")` rather than `use_context` for better error messages when context is missing.

### 1.2 Design Principles for Dioxus Adapter

1. **Minimize re-renders** via fine-grained `Signal` splitting
2. **Component re-runs are OK** because VDOM diffing makes them cheap
3. **Copy-able context** via `Signal` (which is `Copy + Clone`)
4. **Multi-platform** via the `DioxusPlatform` abstraction trait

```toml
# ars-dioxus/Cargo.toml
[dependencies]
ars-core = { workspace = true }
ars-a11y = { workspace = true }
ars-i18n = { workspace = true }
ars-interactions = { workspace = true }
ars-collections = { workspace = true }
ars-forms = { workspace = true }
ars-dom = { workspace = true, optional = true }
dioxus = { version = "0.7" }
log = { version = "0.4", default-features = false, optional = true }

[features]
default = []
debug = ["dep:log", "ars-core/debug", "ars-interactions/debug", "ars-dom?/debug"]
web = ["dioxus/web", "dep:ars-dom", "ars-dom/web"]
desktop = ["dioxus/desktop"]
desktop-dom = ["desktop", "dep:ars-dom"]
mobile = ["dioxus/mobile"]  # Currently resolves to NullPlatform; MobilePlatform pending
ssr = ["dioxus/server", "dep:ars-dom", "ars-dom/ssr"]
```

---

## 2. The `use_machine` Hook

````rust
use std::sync::Arc;
use dioxus::prelude::*;
use ars_core::{Machine, Service, Env, RenderMode};
use ars_i18n::{Locale, IntlBackend, ComponentMessages, I18nRegistries};

/// Return type from `use_machine`.
///
/// `Copy` is valid here because all fields — `ReadSignal`, `Callback`, and
/// `Signal` — implement `Copy` in Dioxus (they are lightweight handles backed
/// by arena indices). If future fields are added that do not implement `Copy`,
/// this derive must be changed to `Clone` only.
#[derive(Clone, Copy)]
pub struct UseMachineReturn<M: Machine + 'static>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    /// Read-only projection of the machine state. Obtained from `Signal<T>::into()`,
    /// which preserves reactive tracking. Reading it in a component creates a
    /// re-render dependency.
    pub state: ReadSignal<M::State>,

    /// Send an event. Non-reactive — safe to call from any handler.
    pub send: Callback<M::Event>,

    /// Access the underlying service for context/props reads and `derive()`.
    pub service: Signal<Service<M>>,

    /// Monotonically increasing counter that bumps whenever context changes.
    /// Used by `derive()` to track context invalidation explicitly.
    pub context_version: ReadSignal<u64>,
}

impl<M: Machine + 'static> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    /// Create a fine-grained memo that derives a value from the connect API.
    /// Only re-computes when the underlying state changes, and only triggers
    /// re-renders when the derived value actually changes.
    ///
    /// **Safety**: The closure passed to `derive()` must not call `send()` — it is
    /// a read-only projection of the current state and context.
    ///
    /// # Example
    /// ```rust
    /// let machine = use_machine::<select::Machine>(props);
    /// let is_open = machine.derive(|api| api.is_open());
    /// let aria_label = machine.derive(|api| api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)).map(str::to_owned));
    /// ```
    pub fn derive<T: Clone + PartialEq + 'static>(
        &self,
        f: impl Fn(&M::Api<'_>) -> T + 'static,
    ) -> Memo<T> {
        let state = self.state;
        let service = self.service;
        let context_version = self.context_version;
        use_memo(move || {
            // Subscribe to both state and context_version so the memo
            // re-computes when either the state OR the context changes.
            let _ = &*state.read();
            let _ = &*context_version.read();
            let svc = service.peek();
            // Use a no-op send closure inside derive(). The Api is read-only here —
            // event handlers must not be called inside a memo. The read lock on
            // `service` is still held, so calling the real `send` would deadlock
            // (send → service.write() while service.read() is active).
            let api = svc.connect(&|_e| {
                #[cfg(debug_assertions)]
                panic!("Cannot send events inside derive() — use event handlers from with_api_snapshot() instead");
            });
            f(&api)
        })
    }
}
````

> **Deadlock Warning:** `derive()` holds a read lock on `service`. If `with_api_snapshot()`
> is called concurrently in the same render cycle and its `send` callback attempts
> `service.write()`, a deadlock occurs (read lock held → write lock blocked).
> `with_api_snapshot()` MUST NOT be used while a `derive()` memo is evaluating.

### 2.1 `EphemeralRef` Newtype

The Dioxus adapter uses the same `EphemeralRef<'a, T>` newtype as the Leptos adapter to prevent `Api<'a>` from being stored in signals or hooks. See `08-adapter-leptos.md` section "EphemeralRef Newtype" for the full type definition and rationale.

**Dioxus-specific usage**: The hook-based API wraps `derive()` identically:

**Dioxus signal safety**: Dioxus `Signal<T>` requires `T: 'static`. `EphemeralRef` contains `PhantomData<(Rc<()>, &'a ())>`, which is not `'static`, so the compiler rejects any attempt to store it:

```rust
// COMPILE ERROR in Dioxus: EphemeralRef does not implement 'static
let sig = use_signal(|| ephemeral_ref); // ❌ Won't compile
```

> **Note:** `EphemeralRef` is defined in `ars-core` with a `pub` constructor; all adapters use the same definition.
> The canonical `PhantomData` is `PhantomData<(Rc<()>, &'a ())>` — providing `!Send`,
> `!Sync`, and non-`'static` guarantees. See `08-adapter-leptos.md` section
> "EphemeralRef Newtype" for the full type definition and rationale.

Cross-reference: The full `EphemeralRef` type definition, safety guarantees, and design rationale are in `08-adapter-leptos.md`. Both adapters share the same `EphemeralRef` type from `ars-core`.

> **Note:** Dioxus `with_api_snapshot` is equivalent to Leptos `with_api_snapshot` (both return `T` directly).
> Leptos additionally provides `with_api_ephemeral` which wraps the result in `EphemeralRef` for
> extra compile-time safety against storing `Api<'a>` in signals. Dioxus achieves the same safety
> via read-lock scoping (`svc.read()` borrow ends at the function-scope boundary).

```rust
use std::collections::HashMap;

impl<M: Machine + 'static> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Messages: Send + Sync + 'static,
{
    /// Get a one-shot snapshot of the connect API.
    /// **Prefer `derive()` for reactive data** — this method does not track dependencies.
    ///
    /// **Parity note:** Dioxus does not provide `with_api_ephemeral` (Leptos-only).
    /// Dioxus achieves the same safety guarantee via read-lock scoping: `svc.read()`
    /// borrows end at the closure boundary, preventing `Api<'a>` from escaping.
    ///
    /// **Deadlock hazard**: MUST NOT be called while a `derive()` memo is evaluating.
    /// `derive()` holds a read lock on `service`; if `send()` inside this snapshot
    /// tries `service.write()`, it deadlocks. If write lock acquisition fails,
    /// the event is deferred to the next microtask via `queue_microtask`.
    ///
    /// **Thread-safety note**: The `try_write()` TOCTOU check below is only
    /// meaningful on WASM's single-threaded executor. On multi-threaded desktop
    /// targets, another thread could acquire a write lock between `try_write()`
    /// succeeding and `send.call()` executing. Desktop targets should use
    /// `spawn` or a channel-based approach for cross-thread event dispatch.
    pub fn with_api_snapshot<T>(&self, f: impl Fn(&M::Api<'_>) -> T) -> T {
        let svc = self.service.read();
        let send = self.send;
        let service = self.service;
        let api = svc.connect(&|e| {
            // Use try_write to avoid deadlock if derive() holds a read lock.
            // On contention, defer the event to the next microtask.
            if service.try_write().is_some() {
                send.call(e);
            } else {
                #[cfg(debug_assertions)]
                log::warn!("with_api_snapshot: write lock contended, deferring event to next microtask");
                let send = send.clone();
                #[cfg(target_arch = "wasm32")]
                // queue_microtask is a utility wrapper provided by ars-dom
                // that calls web_sys::Window::queue_microtask internally.
                { queue_microtask(move || send.call(e)); }
                #[cfg(not(target_arch = "wasm32"))]
                { spawn(async move { send.call(e); }); }
            }
        });
        f(&api)
    }
}

// ID counter: use thread_local Cell on WASM (no atomics), AtomicU64 on desktop.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static DIOXUS_ID_COUNTER: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
#[cfg(target_arch = "wasm32")]
fn dioxus_id_counter() -> u64 {
    DIOXUS_ID_COUNTER.with(|c| { let v = c.get(); c.set(v + 1); v })
}

#[cfg(not(target_arch = "wasm32"))]
static DIOXUS_ID_COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
#[cfg(not(target_arch = "wasm32"))]
fn dioxus_id_counter() -> u64 { DIOXUS_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed) }

/// Generate a related element ID by appending a suffix.
/// Example: `related_id("menu-1", "trigger")` -> `"menu-1-trigger"`
pub fn related_id(base: &str, suffix: &str) -> String {
    format!("{base}-{suffix}")
}

/// Internal — creates a single Service shared by both public hooks.
///
/// Resolves environment values (locale, ICU provider) and messages from
/// `ArsProvider` context before constructing the `Service`. Core code never
/// calls framework hooks — all environment values arrive as parameters.
const fn current_render_mode() -> RenderMode {
    if cfg!(feature = "ssr") {
        RenderMode::Server
    } else {
        RenderMode::Client
    }
}

fn use_machine_inner<M: Machine + 'static>(
    props: M::Props,
    hydrated_state: Option<M::State>,
) -> (UseMachineReturn<M>, Signal<u64>)
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    let generated_id = use_stable_id("component");
    let props_for_sync = props.clone();

    // Auto-inject ID if not provided by the user.
    // Convention: all Props structs have an `id: String` field.
    let props = {
        let mut p = props;
        if p.id().is_empty() {
            p.set_id(generated_id);
        }
        p
    };

    // Resolve environment values from ArsProvider context.
    // These are adapter-only hooks — core code receives Env and Messages as parameters.
    let locale = resolve_locale(None);
    let intl_backend = use_intl_backend();
    let env = Env::new(locale, intl_backend).with_render_mode(current_render_mode());

    // Resolve messages from adapter-level i18n hooks.
    let messages = use_messages::<M::Messages>(None, Some(&env.locale));

    // `use_machine_hydrated()` passes `Some(snapshot.state)` so client hydration
    // preserves the server-rendered state while recomputing context from the
    // current adapter environment.
    // **Safety**: The `init()` function must not call `api.send()` or otherwise
    // produce events. It runs during component initialization and event
    // processing is not yet set up.
    // Move normalized props into Service creation.
    // use_signal's closure runs exactly once (first mount), consuming `props`.
    let service_signal = use_signal(move || {
        if let Some(state) = hydrated_state {
            #[cfg(feature = "ssr")]
            {
                return Service::new_hydrated(props, state, &env, &messages);
            }
        }

        Service::new(props, &env, &messages)
    });

    let context_version: Signal<u64> = use_signal(|| 0u64);

    // Create state_signal BEFORE use_sync_props to ensure it exists if sync triggers re-render.
    // Use .peek() to avoid subscribing the component to service_signal changes.
    let initial_state = service_signal.peek().state().clone();
    let state_signal = use_signal(|| initial_state);

    // Effect cleanups keyed by effect name. On new effects: only replace effects
    // with matching names, leaving other effects running. On state change: drain ALL.
    let effect_cleanups: Signal<HashMap<&'static str, Box<dyn FnOnce()>>> = use_signal(HashMap::new);
    let pending_events = use_hook(|| Arc::new(Mutex::new(Vec::<M::Event>::new())));
    let runtime = MachineRuntime {
        service: service_signal,
        state: state_signal,
        context_version,
        effect_cleanups,
        pending_events: Arc::clone(&pending_events),
    };

    // Automatically sync props on re-render (no manual use_sync_props needed)
    use_sync_props(props_for_sync, runtime.clone());

    // INVARIANT: Effect cleanup functions MUST NOT call `send()`. Doing so would
    // re-enter the send callback while the signal is being written, causing a panic.
    //
    // **Memory leak prevention:** Context/props passed to effect setup closures
    // should extract only the needed fields (e.g., IDs, flags) rather than
    // cloning the entire context or props struct. This prevents retaining large
    // data structures in cleanup closures.
    //
    // **Cleanup ordering:** Cleanup functions run in LIFO order (last effect set
    // up is first to be cleaned up). If a component re-renders during cleanup
    // (e.g., a signal write triggers a reactive update), defer the new effect
    // setup to the next microtask to avoid interleaving setup and cleanup.

    let send = use_hook(|| Callback::new(move |event: M::Event| {
        dispatch_event::<M>(event, runtime.clone());
    }));

    use_drop(move || {
        let cleanups = effect_cleanups.write().drain().map(|(_, cleanup)| cleanup).collect();
        service_signal.write().unmount(cleanups);
    });

    let result = UseMachineReturn {
        state: state_signal.into(), // Signal<T> -> ReadSignal<T> via From impl
        send,
        service: service_signal,
        context_version: context_version.into(),
    };
    // Return context_version (not service_signal, which is already in UseMachineReturn.service).
    // This matches Leptos's pattern where the second value provides write-side context_version
    // for use_sync_props and on_value_change effects.
    (result, context_version)
}

/// Create and manage a machine service with Dioxus reactivity.
pub fn use_machine<M: Machine + 'static>(props: M::Props) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Event: Send + 'static,
    M::Messages: Send + Sync + 'static,
{
    let (result, _) = use_machine_inner::<M>(props);
    result
}
```

### 2.2 SSR Effect Behavior

During server-side rendering, effects are not executed. The `use_machine_inner` hook gates effect setup with `#[cfg(not(feature = "ssr"))]`, ensuring:

- Timer effects (debounce, delay) are not started on the server
- DOM effects (focus, scroll lock) are not attempted without `web_sys`
- All ARIA attributes are still computed by `connect()` and included in SSR HTML

#### 2.2.1 Clipboard "Copied" State and SSR Hydration

The Clipboard component follows the same SSR-safe rules as the Leptos adapter (see `08-adapter-leptos.md` section "Clipboard 'Copied' State and SSR Hydration"). Key points:

1. SSR always renders `State::Idle` — the `init()` function returns idle state.
2. Use CSS animation for the "copied" indicator instead of state-based timeout to avoid hydration races.
3. The `feedback-timer` effect is gated behind a `has_interacted` flag set only after the first user-initiated `Event::Copy`.

#### 2.2.2 Effect Cleanup SSR Safety

Effect cleanup functions are never executed during SSR. However, effect **setup** closures that capture `web_sys` types will fail to compile for the SSR target. All effect setup closures that reference DOM APIs must be gated with `#[cfg(not(feature = "ssr"))]` or wrapped in a platform check:

```rust
#[cfg(not(feature = "ssr"))]
use_effect(move || {
    // DOM-accessing code here
});
```

### 2.3 Reactive Props Sync

All prop syncing is standardized on `use_sync_props`. Adapter components should use this
single function for all controlled value and prop synchronization rather than implementing
ad-hoc sync logic.

> **WARNING: Ordering requirement** `use_sync_props` MUST execute
> before any `derive()` calls in the component body. If `derive()` runs first, it
> takes a read lock on the `service` signal. When `use_sync_props` subsequently
> attempts `service.write()`, Dioxus will deadlock (write-blocked-on-read within
> the same synchronous component body). The `use_machine` hook enforces this by
> calling `use_sync_props` internally before returning `UseMachineReturn`, so
> components that use `use_machine` are safe by construction. Components that
> call `use_sync_props` manually MUST place it before any signal reads.

```rust
/// Synchronize prop changes to the machine service.
/// Runs synchronously during component body (not in a deferred effect)
/// to avoid stale-state rendering cycles.
///
/// # SAFETY (re-entrance)
/// This function performs multiple signal writes (`service.write()`, `context_version.write()`,
/// `prev_props.write()`) within a single component body execution. This is safe because
/// Dioxus batches signal writes during the component body — subscribers are not notified
/// until the component function returns. No re-entrance can occur from these writes.
/// If Dioxus ever changes to eager notification, this code must be revisited.
///
/// `Service::set_props()` returns a `SendResult` that may indicate state and context
/// changes, plus pending effects. This function processes the result fully: updating
/// `state_signal`, bumping `context_version`, and running any pending effects.
///
/// Dioxus callbacks are runtime-local and do not satisfy the core effect API's
/// `Send + Sync` send-handle contract. The adapter therefore bridges effect-originated
/// follow-up events through a thread-safe queue (`Arc<Mutex<Vec<M::Event>>>`):
/// effect setup closures push events into the queue, and the adapter drains that
/// queue back through `dispatch_event()` on the component thread.
///
/// # Deadlock prevention
/// Uses `try_write()` as a fallback: if `service.write()` would block (e.g., a
/// read lock is already held due to incorrect ordering), the prop sync is deferred
/// to the next microtask via `spawn()`. This prevents a hard deadlock
/// but emits a debug warning — the component author should fix the ordering.
pub fn use_sync_props<M: Machine + 'static>(
    current_props: M::Props,
    runtime: MachineRuntime<M>,
) where
    M::Props: Clone + PartialEq + 'static,
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Event: Send + 'static,
{
    let mut prev_props: Signal<Option<M::Props>> = use_signal(|| None);
    let service_id = runtime.service.peek().props().id().to_owned();
    let current_props = props_with_service_id::<M>(current_props, &service_id);

    // Run synchronously during component body — NOT in use_effect
    // Use .peek() to avoid subscribing the component to prev_props changes.
    let prev = prev_props.peek().clone();
    if prev.as_ref() != Some(&current_props) {
        if prev.is_some() {
            // Only sync after first render — init already has correct props.
            // Use try_write() to avoid deadlock if a read lock is held.
            match runtime.service.try_write() {
                Some(mut svc) => {
                    let send_result = svc.set_props(current_props.clone());
                    if send_result.state_changed {
                        runtime.state.set(svc.state().clone());
                    }
                    if send_result.context_changed {
                        *runtime.context_version.write() += 1;
                    }
                    #[cfg(not(feature = "ssr"))]
                    {
                        let ctx_clone = svc.context().clone();
                        let props_clone = svc.props().clone();
                        drop(svc); // release write lock before running effects
                        let send_arc: Arc<dyn Fn(M::Event) + Send + Sync> = Arc::new({
                            let pending_events = Arc::clone(&runtime.pending_events);
                            move |event| pending_events.lock().expect("pending events").push(event)
                        });
                        for effect in send_result.pending_effects {
                            let name = effect.name;
                            let cleanup =
                                effect.run(&ctx_clone, &props_clone, Arc::clone(&send_arc));
                            runtime.effect_cleanups.write().insert(name, cleanup);
                        }
                    }
                }
                None => {
                    // Write lock unavailable — defer to next microtask.
                    // This should not happen when use_sync_props runs before
                    // derive(), but acts as a safety net.
                    #[cfg(debug_assertions)]
                    dioxus::prelude::tracing::warn!(
                        "use_sync_props: service.try_write() failed — deferring prop sync. \
                         Ensure use_sync_props runs before any derive() calls."
                    );
                    let props_deferred = current_props.clone();
                    let mut runtime = runtime.clone();
                    spawn(async move {
                        let send_result = runtime.service.write().set_props(props_deferred);
                        if send_result.state_changed {
                            runtime.state.set(runtime.service.read().state().clone());
                        }
                        if send_result.context_changed {
                            *runtime.context_version.write() += 1;
                        }
                        // Run pending effects (matching the happy-path branch).
                        #[cfg(not(feature = "ssr"))]
                        for effect in send_result.pending_effects {
                            let ctx_clone = runtime.service.read().context().clone();
                            let props_clone = runtime.service.read().props().clone();
                            let send_arc: Arc<dyn Fn(_) + Send + Sync> = Arc::new({
                                let pending_events = Arc::clone(&runtime.pending_events);
                                move |event| pending_events.lock().expect("pending events").push(event)
                            });
                            let _cleanup = effect.run(&ctx_clone, &props_clone, send_arc);
                        }
                    });
                }
            }
        }
        *prev_props.write() = Some(current_props);
    }
}

fn props_with_service_id<M: Machine>(mut props: M::Props, service_id: &str) -> M::Props {
    if props.id().is_empty() {
        props.set_id(service_id.to_owned());
    }

    props
}
```

> **WARNING: Signal batching fragility** `use_sync_props` relies on the
> fact that Dioxus batches signal writes during the component body — subscribers are
> not notified until the component function returns, so no re-render occurs during
> `use_sync_props` execution. This is a **hard requirement**. If a future Dioxus
> version changes to eager signal notification, `use_sync_props` MUST be migrated to
> `use_effect` to avoid re-entrant rendering and stale-state bugs. Test assertions
> should verify this invariant:
>
> ```rust
> #[test]
> fn use_sync_props_does_not_trigger_rerender_during_execution() {
>     // Arrange: set up a component with use_sync_props and a render counter
>     let render_count = Rc::new(Cell::new(0u32));
>     // Act: change props that trigger use_sync_props
>     // Assert: render_count incremented exactly once (the current render),
>     // NOT twice (which would indicate eager notification mid-body)
> }
> ```
>
> **Adapter difference:** Leptos provides `use_machine_with_reactive_props` as a separate hook because Leptos effects are fine-grained and can watch individual signals. Dioxus integrates prop sync into `use_machine` via `use_sync_props` because Dioxus uses component-level re-rendering.

Usage example:

```rust
#[component]
fn Checkbox(props: CheckboxProps) -> Element {
    let core_props = build_core_props(&props);
    // use_machine automatically syncs props via use_sync_props internally
    let machine = use_machine::<checkbox::Machine>(core_props);
    // ...
}
```

### 2.4 `derive()` and `with_api_snapshot()` Methods

The `derive()` and `with_api_snapshot()` methods are defined on `UseMachineReturn` (see above).
They wire a real `send` callback into the `connect()` API, so derived values and snapshots
can dispatch events correctly.

- **`derive(|api| ...)`** — creates a `Memo<T>` that re-computes only when machine state changes
  and only triggers re-renders when the derived value itself changes. Use this for all reactive
  attribute/state reads in `rsx!`.
- **`with_api_snapshot(|api| ...)`** — one-shot, non-reactive read. Prefer `derive()` for
  anything rendered in the DOM.

```rust
// Example usage:
let machine = use_machine::<select::Machine>(props);
let is_open = machine.derive(|api| api.is_open());
let root_attrs = machine.derive(|api| attr_map_to_dioxus(api.root_attrs(), &strategy, Some("root")));

// One-shot snapshot (non-reactive):
let current_value = machine.with_api_snapshot(|api| api.selected_value());
```

---

## 3. `AttrMap` → Dioxus Attributes

Dioxus uses attribute builders in `rsx!`, not a HashMap. The conversion strategy:

### 3.1 Dynamic Attributes

````rust
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, MutexGuard};
use dioxus::prelude::*;
use dioxus_core::AttributeValue;
use ars_core::{
    AttrMap, AttrMapParts, HtmlAttr, AttrValue, CssProperty, StyleStrategy, styles_to_nonce_css,
};

/// Intern pool for attribute names not covered by static fast paths.
/// Dioxus `Attribute::new` requires `&'static str`. Known HTML, ARIA, and
/// ars-generated `data-*` names are compile-time constants. Unknown `data-*`
/// names are interned on first use as a Dioxus compatibility fallback.
static ATTR_NAMES: LazyLock<Mutex<HashSet<&'static str>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn attr_name_pool() -> MutexGuard<'static, HashSet<&'static str>> {
    ATTR_NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn static_data_attr_name(suffix: &str) -> Option<&'static str> {
    match suffix {
        "ars-animated" => Some("data-ars-animated"),
        "ars-disable-outside-pointer-events" => Some("data-ars-disable-outside-pointer-events"),
        "ars-disabled" => Some("data-ars-disabled"),
        "ars-drag-over" => Some("data-ars-drag-over"),
        "ars-dragging" => Some("data-ars-dragging"),
        "ars-drop-operation" => Some("data-ars-drop-operation"),
        "ars-drop-position" => Some("data-ars-drop-position"),
        "ars-focus-visible" => Some("data-ars-focus-visible"),
        "ars-focus-within" => Some("data-ars-focus-within"),
        "ars-focus-within-visible" => Some("data-ars-focus-within-visible"),
        "ars-focused" => Some("data-ars-focused"),
        "ars-hovered" => Some("data-ars-hovered"),
        "ars-index" => Some("data-ars-index"),
        "ars-invalid" => Some("data-ars-invalid"),
        "ars-loading" => Some("data-ars-loading"),
        "ars-long-pressing" => Some("data-ars-long-pressing"),
        "ars-moving" => Some("data-ars-moving"),
        "ars-part" => Some("data-ars-part"),
        "ars-placement" => Some("data-ars-placement"),
        "ars-presence" => Some("data-ars-presence"),
        "ars-pressed" => Some("data-ars-pressed"),
        "ars-prevent-focus-on-press" => Some("data-ars-prevent-focus-on-press"),
        "ars-readonly" => Some("data-ars-readonly"),
        "ars-scope" => Some("data-ars-scope"),
        "ars-segment" => Some("data-ars-segment"),
        "ars-shape" => Some("data-ars-shape"),
        "ars-size" => Some("data-ars-size"),
        "ars-state" => Some("data-ars-state"),
        "ars-variant" => Some("data-ars-variant"),
        "ars-visually-hidden" => Some("data-ars-visually-hidden"),
        "ars-visually-hidden-focusable" => Some("data-ars-visually-hidden-focusable"),
        _ => None,
    }
}

fn intern_attr_name(attr: &HtmlAttr) -> &'static str {
    // Fast path: if HtmlAttr has a known static name, return it directly.
    if let Some(name) = attr.static_name() {
        return name;
    }
    if let HtmlAttr::Data(suffix) = attr
        && let Some(name) = static_data_attr_name(suffix)
    {
        return name;
    }
    // Fallback: unknown Data(...) attributes need a process-lifetime name for Dioxus.
    let name = attr.to_string(); // e.g., "data-ars-state"
    let mut pool = attr_name_pool();
    if let Some(&existing) = pool.get(name.as_str()) {
        return existing;
    }
    let leaked: &'static str = Box::leak(name.into_boxed_str());
    pool.insert(leaked);
    leaked
}

// `HtmlAttr::static_name()` is defined in `01-architecture.md` §3.2.
// It returns `Some(&'static str)` for all non-Data variants:
//   HtmlAttr::Class => Some("class"),
//   HtmlAttr::Id => Some("id"),
//   HtmlAttr::Role => Some("role"),
//   HtmlAttr::TabIndex => Some("tabindex"),
//   HtmlAttr::Style => Some("style"),
//   HtmlAttr::Disabled => Some("disabled"),
//   HtmlAttr::Aria(AriaAttr::Label) => Some("aria-label"),
//   ... etc. for all ARIA attributes.
//   HtmlAttr::Data(_) => None  // dynamic, requires interning

/// Result of converting an `AttrMap` with strategy awareness.
pub struct DioxusAttrResult {
    /// Dioxus dynamic attributes ready for spreading via `..attrs`.
    pub attrs: Vec<Attribute>,
    /// Styles to apply via CSSOM (`element.style().set_property()`).
    /// Non-empty only when strategy is `Cssom`.
    pub cssom_styles: Vec<(CssProperty, String)>,
    /// Stable key for `nonce_css`; `None` when no nonce rule was generated.
    pub nonce_css_key: Option<String>,
    /// CSS rule text to inject into a `<style nonce="...">` block.
    /// Non-empty only when strategy is `Nonce`.
    pub nonce_css: String,
}

/// Convert an `AttrMap` into Dioxus attributes using the given `StyleStrategy`.
///
/// - `map.styles` are rendered according to the active strategy.
/// - `element_id` is required for `Nonce` strategy (used as CSS selector).
/// - `class` and other space-separated attributes are already merged in the `AttrMap`
///   by `set()` and flow through the main attrs loop naturally.
pub fn attr_map_to_dioxus(
    map: AttrMap,
    strategy: &StyleStrategy,
    element_id: Option<&str>,
) -> DioxusAttrResult {
    let AttrMapParts { attrs, styles } = map.into_parts();

    let mut result: Vec<Attribute> = attrs.into_iter()
        .filter_map(|(key, val)| match val {
            AttrValue::String(s) => Some(Attribute::new(intern_attr_name(&key), AttributeValue::Text(s), None, false)),
            AttrValue::Bool(true) => Some(Attribute::new(intern_attr_name(&key), AttributeValue::Text("".into()), None, false)),
            AttrValue::Bool(false) | AttrValue::None => None,
        })
        .collect();

    let mut cssom_styles = Vec::new();
    let mut nonce_css_key = None;
    let mut nonce_css = String::new();

    match strategy {
        StyleStrategy::Inline => {
            if !styles.is_empty() {
                let style_str: String = styles.into_iter()
                    .map(|(prop, val)| format!("{}: {};", prop, val))
                    .collect::<Vec<_>>()
                    .join(" ");
                result.push(Attribute::new("style", AttributeValue::Text(style_str), None, false));
            }
        }
        StyleStrategy::Cssom => {
            cssom_styles = styles;
        }
        StyleStrategy::Nonce(_) => {
            if !styles.is_empty() {
                let id = element_id.expect("element_id is required for Nonce style strategy");
                result.push(Attribute::new("data-ars-style-id", AttributeValue::Text(id.to_string()), None, false));
                nonce_css_key = Some(id.to_string());
                nonce_css = styles_to_nonce_css(id, &styles);
            }
        }
    }

    DioxusAttrResult { attrs: result, cssom_styles, nonce_css_key, nonce_css }
}

/// Tracks CSS properties applied through CSSOM so stale entries can be removed.
#[cfg(feature = "web")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssomStyleHandle {
    applied: Vec<CssProperty>,
}

#[cfg(feature = "web")]
impl CssomStyleHandle {
    pub const fn new() -> Self {
        Self { applied: Vec::new() }
    }

    pub fn sync(&mut self, el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
        let style = el.style();
        for property in self.applied.drain(..) {
            if !styles.iter().any(|(next, _)| *next == property)
                && let Err(e) = style.remove_property(&property.to_string())
            {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&e);
            }
        }
        for (prop, val) in styles {
            if let Err(e) = style.set_property(&prop.to_string(), val) {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&e);
            }
            self.applied.push(prop.clone());
        }
    }

    pub fn clear(&mut self, el: &web_sys::HtmlElement) {
        let style = el.style();
        for property in self.applied.drain(..) {
            if let Err(e) = style.remove_property(&property.to_string()) {
                #[cfg(debug_assertions)]
                web_sys::console::warn_1(&e);
            }
        }
    }
}

/// Apply styles to a DOM element via the CSSOM API.
/// Prefer `CssomStyleHandle` or `use_cssom_styles()` when styles can change over time.
#[cfg(feature = "web")]
pub fn apply_styles_cssom(el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    CssomStyleHandle::new().sync(el, styles);
}

/// Synchronize CSSOM styles from an attribute conversion result to an event target.
/// Owns a persistent `CssomStyleHandle`, clears owned properties when the
/// target node changes, and clears the last touched node on cleanup.
/// Use `use_cssom_styles()` when the style list itself is reactive.
#[cfg(feature = "web")]
pub fn use_cssom_styles_from_attrs(
    target: Signal<Option<web_sys::EventTarget>>,
    result: &DioxusAttrResult,
) {
    let styles = result.cssom_styles.clone();
    use_cssom_styles(target, move || styles.clone());
}

/// Synchronize reactive CSSOM styles to an event target.
///
/// The closure runs inside a Dioxus effect. Signal reads in the closure
/// resubscribe the hook; stale properties are removed on each sync, previous
/// targets are cleared when the signal changes, and cleanup clears the last
/// touched target.
#[cfg(feature = "web")]
pub fn use_cssom_styles<F>(target: Signal<Option<web_sys::EventTarget>>, styles: F)
where
    F: Fn() -> Vec<(CssProperty, String)> + 'static,
{
    let mut handle = use_hook(|| CopyValue::new(CssomStyleHandle::new()));
    let mut applied_element = use_hook(|| CopyValue::new(None::<web_sys::HtmlElement>));

    use_effect(move || {
        let styles = styles();
        let element = target
            .read()
            .as_ref()
            .and_then(|target| target.clone().dyn_into::<web_sys::HtmlElement>().ok());

        if let Some(previous) = applied_element.write().take() {
            handle.write().clear(&previous);
        }
        let Some(element) = element else { return };

        handle.write().sync(&element, &styles);
        applied_element.set(Some(element));
    });

    use_drop(move || {
        if let Ok(mut applied_element) = applied_element.try_write()
            && let Some(element) = applied_element.take()
            && let Ok(mut handle) = handle.try_write()
        {
            handle.clear(&element);
        }
    });
}

/// Collect nonce CSS generated by `attr_map_to_dioxus`.
///
/// Components using `StyleStrategy::Nonce` MUST call this after converting
/// attributes. The helper no-ops when `nonce_css` is empty and replaces any
/// previous rule with the same `nonce_css_key`.
pub fn collect_nonce_css_from_attrs(result: &DioxusAttrResult) {
    if !result.nonce_css.is_empty() {
        let key = result.nonce_css_key.clone().unwrap_or_else(|| result.nonce_css.clone());
        upsert_nonce_css(key, result.nonce_css.clone());
    }
}

/// Schedule nonce CSS collection after render setup.
/// Component code should prefer this hook over direct render-phase writes.
/// Use `use_nonce_css_rule()` when the nonce rule itself is reactive.
pub fn use_nonce_css_from_attrs(result: &DioxusAttrResult) {
    let entry = nonce_css_entry_from_attrs(result);
    use_nonce_css_rule(move || entry.clone());
}

/// Schedule reactive keyed nonce CSS collection for the current scope.
///
/// `Some((key, css))` inserts or replaces a keyed rule. Returning `None`, key
/// changes, and scope cleanup remove the previously owned key from the
/// collector.
pub fn use_nonce_css_rule<F>(rule: F)
where
    F: Fn() -> Option<(String, String)> + 'static,
{
    let rules = try_use_context::<ArsNonceCssCtx>().map(|ctx| ctx.rules);
    let mut applied_key = use_hook(|| CopyValue::new(None::<String>));

    use_effect(move || {
        let Some(rules) = rules else {
            applied_key.set(None);
            return;
        };

        match rule() {
            Some((key, css)) => {
                if let Some(previous) = applied_key.peek().as_ref()
                    && previous != &key
                {
                    remove_nonce_css_from_rules(rules, previous);
                }
                upsert_nonce_css_in_rules(rules, key.clone(), css);
                applied_key.set(Some(key));
            }
            None => {
                if let Some(previous) = applied_key.write().take() {
                    remove_nonce_css_from_rules(rules, &previous);
                }
            }
        }
    });

    use_drop(move || {
        if let Some(rules) = rules
            && let Ok(mut applied_key) = applied_key.try_write()
            && let Some(previous) = applied_key.take()
        {
            remove_nonce_css_from_rules(rules, &previous);
        }
    });
}

/// Macro for spreading attrs in rsx!
///
/// Usage:
/// ```rust
/// let attrs = api.root_attrs();
/// let strategy = use_style_strategy();
/// rsx! {
///     div { ..attr_map_to_dioxus(attrs, &strategy, Some("my-el")).attrs, {children} }
/// }
/// ```
#[macro_export]
macro_rules! dioxus_attrs {
    ($map:expr, $strategy:expr, $id:expr) => {
        $crate::attr_map_to_dioxus($map, $strategy, $id)
    };
}
````

> **Migration note:** The previous `attr_map_to_dioxus(map: AttrMap) -> Vec<Attribute>` signature is replaced by the strategy-aware version above. Callers must pass a `StyleStrategy` reference (obtained via `use_style_strategy()`) and an optional element ID.

### 3.2 Typed Handler Wiring

Adapters use `derive()` to reactively extract attributes from the connect API,
and wire event handlers through `send.call()`:

```rust
// In a Dioxus component:
let strategy = use_style_strategy();
let root_attrs = machine.derive(move |api| attr_map_to_dioxus(api.root_attrs(), &strategy, Some("checkbox-root")));
let data_state = machine.derive(|api| api.data_state().to_string());

rsx! {
    div {
        ..root_attrs.read().attrs,
        "data-ars-state": data_state(),
        onclick: move |_| machine.send.call(Event::Toggle),
        onkeydown: move |ev| {
            match dioxus_key_to_keyboard_key(&ev.key()).0 {
                KeyboardKey::Enter | KeyboardKey::Space => machine.send.call(Event::Toggle),
                _ => {}
            }
        },
        {props.children}
    }
}
```

When `root_attrs.read().nonce_css` is non-empty, component glue MUST call
`collect_nonce_css_from_attrs(&root_attrs.read())` during render/effect wiring so
the provider-owned nonce style block receives the generated rule.

### 3.3 Event Listener Options

> **Event listener options.** AttrMap handlers may include `EventOptions { passive, capture }`
> metadata. The Dioxus adapter emits these as:
>
> - `passive: true` -> listener registered with `{ passive: true }` option via web_sys
> - `capture: true` -> listener registered with `{ capture: true }` option via web_sys
>
> Dioxus does not natively support event modifiers like Leptos. Use `web_sys::EventTarget::add_event_listener_with_event_listener_and_add_event_listener_options()` for fine-grained control.

```rust
/// Event listener options for passive and capture modes.
pub struct EventOptions {
    /// If true, the event listener will not call preventDefault().
    /// Required for passive scroll/touch listeners.
    pub passive: bool,
    /// If true, the event fires during the capture phase.
    pub capture: bool,
}
```

### 3.4 Practical Pattern: Direct rsx! Attribute Building

Components build attributes inline in `rsx!` via the typed connect API:

```rust
// The connect API for Dioxus provides typed attribute getters:
impl CheckboxDioxusApi {
    pub fn control_attrs(&self) -> Vec<(&'static str, String)> {
        let [(scope_attr, scope_val), (part_attr, part_val)] = checkbox::Part::Control.data_attrs();
        vec![
            ("role", "checkbox".to_string()),
            ("tabindex", "0".to_string()),
            (scope_attr, scope_val.to_string()),
            (part_attr, part_val.to_string()),
            ("aria-checked", self.aria_checked_value().to_string()),
            ("data-ars-state", self.data_state().to_string()),
        ]
    }
}
```

### 3.5 CSP Style Strategy

The adapter provides a context-based `StyleStrategy` configuration. Components read the strategy from context and pass it to `attr_map_to_dioxus()`.

```rust
use dioxus::prelude::*;
use ars_core::StyleStrategy;

/// Read the current style strategy from context.
/// Returns `StyleStrategy::Inline` (the default) if no `ArsProvider` is present.
pub fn use_style_strategy() -> StyleStrategy {
    try_use_context::<ArsContext>()
        .map(|ctx| ctx.style_strategy().clone())
        .unwrap_or_else(|| {
            warn_missing_provider("use_style_strategy");
            StyleStrategy::default()
        })
}
```

`warn_missing_provider()` is adapter-private. It emits `log::warn!` messages
only when the `ars-dioxus/debug` feature is enabled; otherwise these fallback
paths are silent.

> **Note:** `ArsStyleProvider` and `ArsStyleCtx` have been removed. The style strategy is
> now provided by `ArsProvider` (§16) via the `style_strategy` field on `ArsContext`.
> Components continue to call `use_style_strategy()` unchanged.

#### 3.5.1 Nonce CSS Collector

`ArsProvider` always owns a nonce CSS collector context for the whole provider subtree. When `style_strategy` is `StyleStrategy::Nonce(nonce)`, the provider renders the collector automatically as a `<style nonce="...">` before descendant content. Components append scoped CSS rules into that provider-owned collector only when `StyleStrategy::Nonce` produces nonce CSS. `ArsNonceCssProvider` is the standalone public wrapper for advanced/manual collector ownership; `ArsNonceStyle` only renders an already-provided collector.

```rust
/// Stable nonce CSS rule keyed by the styled element or rule owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonceCssRule {
    pub key: String,
    pub css: String,
}

/// Context for collecting nonce CSS rules during rendering.
#[derive(Clone, Debug)]
pub struct ArsNonceCssCtx {
    pub rules: Signal<Vec<NonceCssRule>>,
}

/// Create and publish a nonce CSS collector context for the current scope.
pub fn use_nonce_css_context_provider() -> ArsNonceCssCtx {
    let rules = use_signal(Vec::<NonceCssRule>::new);
    let context = ArsNonceCssCtx { rules };
    use_context_provider(move || context.clone());
    context
}

/// Provide a nonce CSS collector context and render collected rules with `nonce`.
#[component]
pub fn ArsNonceCssProvider(nonce: String, children: Element) -> Element {
    use_nonce_css_context_provider();

    rsx! {
        ArsNonceStyle { nonce }
        {children}
    }
}

/// Renders the current nonce CSS collector as a `<style nonce="...">` block.
/// `ArsProvider` renders this automatically for `StyleStrategy::Nonce`.
#[component]
pub fn ArsNonceStyle(nonce: String) -> Element {
    let context = try_use_context::<ArsNonceCssCtx>();

    let css_text = use_memo(move || {
        context
            .as_ref()
            .map(|ctx| ctx.rules.read().iter().map(|rule| rule.css.as_str()).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default()
    });

    rsx! {
        style { nonce: nonce, {css_text()} }
    }
}

/// Collect a CSS rule using the rule text as its stable key.
/// This is a low-level manual API; component glue should prefer keyed hooks.
pub fn append_nonce_css(css: String) {
    upsert_nonce_css(css.clone(), css);
}

/// Insert or replace a CSS rule in the nonce collector.
/// Called internally by components when `StyleStrategy::Nonce` is active.
pub fn upsert_nonce_css(key: String, css: String) {
    if let Some(mut ctx) = try_use_context::<ArsNonceCssCtx>() {
        let mut rules = ctx.rules.write();
        if let Some(rule) = rules.iter_mut().find(|rule| rule.key == key) {
            rule.css = css;
        } else {
            rules.push(NonceCssRule { key, css });
        }
    }
}

/// Remove a CSS rule from the nonce collector.
pub fn remove_nonce_css(key: &str) {
    if let Some(mut ctx) = try_use_context::<ArsNonceCssCtx>() {
        ctx.rules.write().retain(|rule| rule.key != key);
    }
}
```

---

## 4. Standard Component Pattern

Library adapter code MUST use explicit `Props` structs for every component and sub-part that accepts one or more props. Those props structs MUST implement `Clone` and `PartialEq`; deriving both is preferred, but a manual `PartialEq` implementation is allowed when trait-object fields or other non-derivable members require custom equality semantics. Zero-prop parts (e.g., `fn Backdrop() -> Element`) MAY use a bare function signature. The `#[component]` attribute is still applied to the function itself, but the parameter list is always a single `props: XxxProps` argument.

Rationale: explicit Props structs enable generic type parameters, custom `PartialEq` implementations for re-render control, full control over doc-comment rendering, and `#[props(extends = GlobalAttributes)]` for attribute spreading. This aligns with the Dioxus team recommendation for library code and matches the pattern used in the official `DioxusLabs/components` crate.

### 4.1 Props Derivation

```rust
use dioxus::prelude::*;

/// Dioxus props must implement Clone + PartialEq.
#[derive(Props, Clone, PartialEq)]
pub struct CheckboxProps {
    /// Controlled checked state.
    pub checked: Option<Signal<checkbox::State>>,

    /// Default checked state for uncontrolled mode.
    #[props(default)]
    pub default_checked: checkbox::State,

    /// Whether the checkbox is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the checkbox is required.
    #[props(default = false)]
    pub required: bool,

    /// Name for form submission.
    pub name: Option<String>,

    /// Value for form submission.
    #[props(default = "on".to_string())]
    pub value: String,

    /// Adapter-level callback when checked state changes.
    /// This is NOT passed to core Props — adapter observes state and calls it.
    pub on_checked_change: Option<EventHandler<checkbox::State>>,

    pub children: Element,
}
```

### 4.2 Root Component

```rust
#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let core_props = checkbox::Props {
        // Use .peek() to avoid subscribing the component to the checked signal here.
        // The reactive sync is handled by use_sync_props inside use_machine.
        checked: props.checked.as_ref().map(|s| *s.peek()),
        default_checked: props.default_checked,
        disabled: props.disabled,
        required: props.required,
        name: props.name.clone(),
        value: props.value.clone(),
    };

    let machine = use_machine::<checkbox::Machine>(core_props);

    let UseMachineReturn { state, send, .. } = machine;

    // Fire on_checked_change callback when state changes.
    // NOTE: Hooks are called unconditionally to maintain stable hook ordering
    // across re-renders (Dioxus requirement). The callback invocation is gated
    // on `on_checked_change` being `Some`.
    let mut prev_state: Signal<Option<checkbox::State>> = use_signal(|| None);
    use_effect(move || {
        let current = state.read().clone();
        let prev = prev_state.peek().clone();
        if prev.as_ref() != Some(&current) {
            if prev.is_some() {
                if let Some(on_change) = props.on_checked_change {
                    on_change.call(current);
                }
            }
            *prev_state.write() = Some(current);
        }
    });

    // Dual-path controlled value sync:
    //   1. `use_sync_props` (inside `use_machine`) syncs the full Props struct on re-render,
    //      handling initial state and bulk prop changes at the machine level.
    //   2. `use_controlled_prop_sync` / `_optional` (below) sends individual events for
    //      specific signals, ensuring the machine processes fine-grained state transitions
    //      (e.g., SetChecked) that trigger side effects like onChange callbacks.
    // Both paths are needed: use_sync_props alone cannot fire individual events,
    // and use_controlled_prop_sync alone doesn't handle initial prop reconciliation.
    // NOTE: Called unconditionally to preserve stable hook ordering. When `checked`
    // is None (uncontrolled mode), the internal hook still runs but no event is sent.
    let checked_value = props.checked.map(|sig| *sig.peek());
    use_controlled_prop_sync_optional(send, checked_value, checkbox::Event::SetChecked);
    use_controlled_prop_sync(send, props.disabled, checkbox::Event::SetDisabled);

    use_context_provider(|| CheckboxCtx { state, send, service: machine.service, context_version: machine.context_version });

    rsx! { {props.children} }
}

/// Copy-able context (Signal is Copy).
#[derive(Clone, Copy)]
pub struct CheckboxCtx {
    pub state: ReadSignal<checkbox::State>,
    pub send: Callback<checkbox::Event>,
    pub service: Signal<Service<checkbox::Machine>>,
    pub context_version: ReadSignal<u64>,
}
```

### 4.3 Child Parts

```rust
pub mod checkbox {
    #[derive(Props, Clone, PartialEq)]
    pub struct ControlProps {
        pub class: Option<String>,
        pub children: Element,
    }

    #[component]
    pub fn Control(props: ControlProps) -> Element {
        let ctx = try_use_context::<CheckboxCtx>()
            .expect("checkbox::Control must be used inside Checkbox");

        // Reconstruct UseMachineReturn from context to use derive().
        let machine = UseMachineReturn {
            state: ctx.state,
            send: ctx.send,
            service: ctx.service,
            context_version: ctx.context_version,
        };

        // Derive control attributes reactively from the connect API.
        // This replaces manual state matching and hardcoded ARIA values.
        let strategy = use_style_strategy();
        let control_attrs = machine.derive(move |api| attr_map_to_dioxus(api.control_attrs(), &strategy, Some("checkbox-control")));
        let data_state = machine.derive(|api| api.data_state().to_string());

        rsx! {
            div {
                ..control_attrs.read().attrs,
                "data-ars-state": data_state(),
                if let Some(cls) = &props.class { class: "{cls}" }
                onclick: move |_| ctx.send.call(checkbox::Event::Toggle),
                onkeydown: move |e: KeyboardEvent| {
                    if dioxus_key_to_keyboard_key(&e.key()).0 == KeyboardKey::Space {
                        e.prevent_default();
                        ctx.send.call(checkbox::Event::Toggle);
                    }
                },
                onfocus: move |_| ctx.send.call(checkbox::Event::Focus),
                onblur: move |_| ctx.send.call(checkbox::Event::Blur),
                {props.children}
            }
        }
    }

    #[derive(Props, Clone, PartialEq)]
    pub struct IndicatorProps {
        /// Only render children when checkbox matches this state.
        /// When `None`, children are shown for any non-Unchecked state (default).
        pub match_state: Option<checkbox::State>,
        pub children: Element,
    }

    #[component]
    pub fn Indicator(props: IndicatorProps) -> Element {
        let ctx = try_use_context::<CheckboxCtx>()
            .expect("checkbox::Indicator must be used inside Checkbox");

        let should_show = match props.match_state {
            Some(checkbox::State::Checked) => matches!(*ctx.state.read(), checkbox::State::Checked),
            Some(checkbox::State::Indeterminate) => matches!(*ctx.state.read(), checkbox::State::Indeterminate),
            Some(checkbox::State::Unchecked) => matches!(*ctx.state.read(), checkbox::State::Unchecked),
            None => !matches!(*ctx.state.read(), checkbox::State::Unchecked),
        };

        let [(scope_attr, scope_val), (part_attr, part_val)] = checkbox::Part::Indicator.data_attrs();

        rsx! {
            span {
                "{scope_attr}": scope_val,
                "{part_attr}": part_val,
                if should_show { {props.children} }
            }
        }
    }

    /// Accessible label for the checkbox.
    #[component]
    pub fn Label(children: Element) -> Element {
        let [(scope_attr, scope_val), (part_attr, part_val)] = checkbox::Part::Label.data_attrs();
        rsx! {
            label {
                "{scope_attr}": scope_val,
                "{part_attr}": part_val,
                {children}
            }
        }
    }

    /// Hidden native input for form submission.
    #[component]
    pub fn HiddenInput() -> Element {
        let ctx = try_use_context::<CheckboxCtx>()
            .expect("checkbox::HiddenInput must be used inside Checkbox");

        let machine = UseMachineReturn {
            state: ctx.state,
            send: ctx.send,
            service: ctx.service,
            context_version: ctx.context_version,
        };
        let name = machine.derive(|api| api.props().name.clone());
        let value = machine.derive(|api| api.props().value.clone().unwrap_or_else(|| "on".into()));
        let required = machine.derive(|api| api.props().required);
        let checked = machine.derive(|api| api.is_checked());

        rsx! {
            input {
                r#type: "checkbox",
                name: name,
                value: value,
                checked: checked,
                required: required,
                style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0",
                aria_hidden: "true",
                tabindex: "-1",
            }
        }
    }
}
```

---

## 5. Re-render Optimization

Because Dioxus re-runs components on any Signal read, we split state into granular signals:

### 5.1 Granular State Signals

The Dioxus Select adapter builds a `StaticCollection<select::Item>` from children and converts
`String` values to/from `Key::String` at the boundary. Granular signals ensure minimal
re-renders.

```rust
use ars_core::select;
use ars_collections::{Key, selection::Set, StaticCollection, CollectionBuilder};

/// Instead of one Signal<FullState>, split into concern-specific signals.
///
/// This ensures only components that depend on `open` re-render when open changes,
/// not components that only depend on `highlighted_key`.
#[derive(Clone, Copy)]
pub struct SelectCtx {
    /// Only changes on open/close.
    pub open: ReadSignal<bool>,
    /// Only changes when highlighted item changes.
    pub highlighted_key: ReadSignal<Option<Key>>,
    /// Only changes when selection changes.
    pub selection: ReadSignal<selection::Set>,
    /// Stable send callback — never changes.
    pub send: Callback<select::Event>,
    pub service: Signal<Service<select::Machine>>,
    pub context_version: ReadSignal<u64>,
}

fn setup_select_ctx(machine: &UseMachineReturn<select::Machine>) -> SelectCtx {
    let send = machine.send;
    let service = machine.service;
    let context_version = machine.context_version;

    // Derive granular signals using Memo
    let open = machine.derive(|api| api.is_open());
    let highlighted_key = machine.derive(|api| api.highlighted_key().cloned());
    let selection = machine.derive(|api| api.selection().clone());

    SelectCtx {
        open: open.into(),
        highlighted_key: highlighted_key.into(),
        selection: selection.into(),
        send,
        service,
        context_version,
    }
}
```

### 5.2 use_memo for Derived Data

```rust
pub mod select {
    // In select::Content: only re-renders when open changes
    #[component]
    pub fn Content(children: Element) -> Element {
        let ctx = try_use_context::<SelectCtx>()
            .expect("select::Content must be used inside Select");

        let [(scope_attr, scope_val), (part_attr, part_val)] = select::Part::Content.data_attrs();
        rsx! {
            if *ctx.open.read() {
                div {
                    role: "listbox",
                    "{scope_attr}": scope_val,
                    "{part_attr}": part_val,
                    {children}
                }
            }
        }
    }

    // In select::Item: only re-renders when its selection status changes
    #[component]
    pub fn Item(value: String, children: Element) -> Element {
        let ctx = try_use_context::<SelectCtx>()
            .expect("select::Item must be used inside Select");
        let key = Key::String(value.clone());
        let key_for_selected = key.clone();
        let key_for_highlighted = key.clone();

        // Note: use_memo caches the derived bool, avoiding unnecessary VDOM diffs
        // when the parent re-renders for unrelated reasons. The .read() inside the
        // memo subscribes the memo (not the component) to the signal, so the
        // component only re-renders when the bool result actually changes.
        let is_selected = use_memo(move || {
            ctx.selection.read().contains(&key_for_selected)
        });

        let is_highlighted = use_memo(move || {
            ctx.highlighted_key.read().as_ref() == Some(&key_for_highlighted)
        });

        let key_for_click = key.clone();
        let key_for_hover = key.clone();

        let [(scope_attr, scope_val), (part_attr, part_val)] = select::Part::Item.data_attrs();
        rsx! {
            div {
                role: "option",
                "{scope_attr}": scope_val,
                "{part_attr}": part_val,
                "aria-selected": (*is_selected.read()).to_string(),
                "data-ars-highlighted": if *is_highlighted.read() { "true" } else { "false" },
                onclick: move |_| ctx.send.call(select::Event::SelectItem(key_for_click.clone())),
                onpointerenter: move |_| ctx.send.call(
                    select::Event::HighlightItem(Some(key_for_hover.clone()))
                ),
                {children}
            }
        }
    }
}
```

---

## 6. Multi-Platform Support

### 6.1 Platform Abstraction Trait

#### 6.1.1 Supporting Types

`Rect` comes from `ars_core::Rect` (foundation §1, geometry primitive
shared across adapters). `FileRef` comes from `ars_forms::field::FileRef`.
`DragItem` and `FileHandle` come from `ars_interactions` (foundation §5,
drag-and-drop shared payloads). The adapter does **not** redefine any of
these — re-using them keeps domain types consistent across components.

```rust
/// Options for opening a platform file picker.
///
/// Mirrors the subset of HTML `<input type="file">` configuration the
/// adapter exposes. Web targets ignore these options because the real
/// picker is hosted by the FileUpload component's hidden `<input>`;
/// desktop targets translate them into an `rfd`-style native dialog
/// when that implementation lands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePickerOptions {
    /// Accepted MIME types or extensions. Empty means accept any.
    pub accept: Vec<String>,

    /// Whether the user may select more than one file at once.
    pub multiple: bool,
}

/// Adapter-local drag payload extracted from a platform drag event.
///
/// `items` may be empty on `DragEnter`/`DragOver` if the browser
/// restricts access to item content until drop (security restriction).
/// `types` is always available and carries the MIME types advertised by
/// the drag source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DragData {
    pub items: Vec<DragItem>,
    pub types: Vec<String>,
}

const DEFAULT_DRAG_TYPE_PROBES: &[&str] = &[
    "text/plain",
    "text/html",
    "text/uri-list",
    "application/json",
];

impl DragData {
    /// Builds a `DragData` from Dioxus's unified drag event payload.
    ///
    /// Works on every target Dioxus supports: web, desktop, and SSR.
    /// `types` is populated by probing the known public
    /// `DataTransfer::get_data(format)` formats; `items` is populated
    /// from `DataTransfer::files()`.
    pub fn from_drag_data(event: &dioxus::events::DragData) -> Self {
        let data_transfer = event.data_transfer();

        let mut types = Vec::new();
        for format in DEFAULT_DRAG_TYPE_PROBES {
            if data_transfer.get_data(format).is_some() {
                types.push(String::from(*format));
            }
        }

        let mut items = Vec::new();
        for file in data_transfer.files() {
            let mime_type = file
                .content_type()
                .unwrap_or_else(|| String::from("application/octet-stream"));
            let raw_path = file.path();
            let handle = if raw_path.as_os_str().is_empty() {
                FileHandle::opaque()
            } else {
                FileHandle::from_path(raw_path)
            };

            items.push(DragItem::File {
                name: file.name(),
                mime_type,
                size: file.size(),
                handle,
            });
        }

        if !items.is_empty() {
            types.push(String::from("Files"));
        }

        Self { items, types }
    }
}

/// Platform-agnostic handle to a drag event.
///
/// Adapter glue wraps the framework's drag event in a
/// `PlatformDragEvent` before passing it to
/// `DioxusPlatform::create_drag_data`. The wrapper carries a borrowed
/// `&dioxus::events::DragData` (via `from_dioxus`), the unified event
/// payload Dioxus exposes across web and desktop. On every target an
/// "empty" wrapper is constructible (via `empty`) for tests and for
/// components that compile against the trait but never produce real
/// drag data. The typed wrapper replaces the earlier `&dyn Any`
/// parameter so a type mismatch becomes a compile error instead of a
/// silent `None`. `Copy` lets callers pass the same wrapper to multiple
/// platforms or helpers without clones.
#[derive(Clone, Copy, Debug)]
pub struct PlatformDragEvent<'a> {
    inner: Option<&'a dioxus::events::DragData>,
}

impl<'a> PlatformDragEvent<'a> {
    /// Constructs a payload-less drag event. Available on every target.
    pub const fn empty() -> Self {
        Self { inner: None }
    }

    /// Wraps a Dioxus drag event for `create_drag_data`.
    pub const fn from_dioxus(event: &'a dioxus::events::DragData) -> Self {
        Self { inner: Some(event) }
    }

    /// Returns the underlying Dioxus drag event if the wrapper was
    /// built from one, otherwise `None`.
    pub const fn as_dioxus(&self) -> Option<&'a dioxus::events::DragData> {
        self.inner
    }
}
```

#### 6.1.2 Trait Definition

```rust
use std::rc::Rc;

use dioxus::events::{MountedData, ScrollBehavior};

/// Operations that differ between web and native platforms.
///
/// **Note on `Send` bounds.** The futures returned by async methods
/// (`set_clipboard`, `open_file_picker`) are `!Send` on WASM. On desktop,
/// Dioxus's runtime can run `!Send` futures on the current thread via
/// `dioxus::spawn`, so the trait keeps a single uniform return type
/// across platforms. Callers on desktop runtimes that require `Send`
/// must route the future through `dioxus::spawn` or convert it
/// themselves.
pub trait DioxusPlatform: Send + Sync + 'static {
    /// Focuses a mounted element through Dioxus's renderer-backed
    /// element handle.
    fn focus_mounted_element(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async move {
            element
                .set_focus(true)
                .await
                .map_err(|err| err.to_string())
        })
    }

    /// Returns the viewport-relative bounding rectangle for a mounted
    /// element through Dioxus's renderer-backed element handle.
    fn get_mounted_bounding_rect(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Option<Rect>>>> {
        Box::pin(async move { element.get_client_rect().await.ok().map(rect_from_pixels) })
    }

    /// Scrolls a mounted element into view through Dioxus's
    /// renderer-backed element handle.
    fn scroll_mounted_into_view(
        &self,
        element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async move {
            element
                .scroll_to(ScrollBehavior::Instant)
                .await
                .map_err(|err| err.to_string())
        })
    }

    /// Writes text to the system clipboard.
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>>;

    /// Opens a native file picker.
    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>>;

    /// Returns a monotonically non-decreasing duration since a
    /// platform-defined start point.
    ///
    /// The start point is implementation-defined and intentionally
    /// opaque (web: page-load via `performance.now()`; desktop:
    /// process-local `Instant` start; null: always zero). Callers MUST
    /// use this only for relative measurements — subtract two values
    /// from the same platform instance to get an elapsed `Duration`.
    /// Comparing values across platforms or processes is undefined.
    fn monotonic_now(&self) -> Duration;

    /// Generates a platform-scoped unique ID.
    ///
    /// Web returns a UUIDv4 from `crypto.randomUUID()`; desktop returns
    /// `uuid::Uuid::new_v4()`; the null implementation returns a
    /// sequential counter prefixed with `null-id-`.
    fn new_id(&self) -> String;

    /// Extracts adapter drag data from a platform-specific drag event.
    ///
    /// Returns `None` when the wrapper does not carry a Dioxus drag
    /// event. The typed `PlatformDragEvent` wrapper enforces the
    /// underlying event type at compile time.
    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData>;
}

const fn rect_from_pixels(rect: dioxus::html::geometry::PixelsRect) -> Rect {
    Rect {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}
```

#### 6.1.3 NullPlatform

`NullPlatform` is the no-op implementation used by tests, SSR, and the
`mobile` feature until a dedicated mobile platform lands. It is always
in scope regardless of feature gating.

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct NullPlatform;

impl DioxusPlatform for NullPlatform {
    fn focus_mounted_element(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn get_mounted_bounding_rect(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Option<Rect>>>> {
        Box::pin(async { None })
    }

    fn scroll_mounted_into_view(
        &self,
        _element: Rc<MountedData>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn set_clipboard(&self, _text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        Box::pin(async { Ok(()) })
    }

    fn open_file_picker(
        &self,
        _options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        Box::pin(async { Vec::new() })
    }

    fn monotonic_now(&self) -> Duration { Duration::ZERO }

    fn new_id(&self) -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        format!("null-id-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    fn create_drag_data(&self, _event: PlatformDragEvent<'_>) -> Option<DragData> { None }
}
```

#### 6.1.4 WebPlatform (wasm32-only)

`WebPlatform` exists only when both `feature = "web"` and
`target_arch = "wasm32"` are true — every method invokes browser APIs
that have no meaningful native fallback. On non-wasm hosts (e.g.,
`cargo check --features web` on a developer's Linux box) the type is
not in scope and `default_dioxus_platform()` (§6.1.6) falls through to
`NullPlatform`. This keeps build tooling green without silently
shipping "clipboard writes succeed but do nothing" semantics into a
misconfigured production build.

```rust
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct WebPlatform;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
impl DioxusPlatform for WebPlatform {
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let text = text.to_string();
        Box::pin(async move {
            let window = web_sys::window().ok_or_else(|| String::from("no window available"))?;
            let promise = window.navigator().clipboard().write_text(&text);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map(|_| ())
                .map_err(|err| format!("{err:?}"))
        })
    }

    fn open_file_picker(
        &self,
        _options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        // Web defers to the FileUpload component's hidden <input type="file">.
        Box::pin(async { Vec::new() })
    }

    fn monotonic_now(&self) -> Duration {
        let millis = web_sys::window()
            .and_then(|window| window.performance())
            .map(|performance| performance.now())
            .expect("window.performance must be available on web targets");
        // Convert via `from_secs_f64(millis / 1000.0)` to preserve the
        // sub-millisecond precision `performance.now()` exposes.
        Duration::from_secs_f64(millis / 1000.0)
    }

    fn new_id(&self) -> String {
        web_sys::window()
            .map(|window| {
                window
                    .crypto()
                    .expect("window.crypto must be available on web targets")
                    .random_uuid()
            })
            .expect("window must be available on web targets")
    }

    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData> {
        event.as_dioxus().map(DragData::from_drag_data)
    }
}
```

#### 6.1.5 DesktopPlatform

```rust
#[cfg(feature = "desktop")]
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopPlatform;

#[cfg(feature = "desktop")]
impl DioxusPlatform for DesktopPlatform {
    fn set_clipboard(&self, text: &str) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let text = text.to_string();
        Box::pin(async move {
            let mut clipboard = arboard::Clipboard::new().map_err(|err| err.to_string())?;
            clipboard.set_text(&text).map_err(|err| err.to_string())
        })
    }

    fn open_file_picker(
        &self,
        options: FilePickerOptions,
    ) -> Pin<Box<dyn Future<Output = Vec<FileRef>>>> {
        Box::pin(async move {
            let mut dialog = rfd::AsyncFileDialog::new();
            let extensions = file_picker_filter_extensions(&options.accept);

            if !extensions.is_empty() {
                dialog = dialog.add_filter(file_picker_filter_name(&extensions), &extensions);
            }

            let handles = if options.multiple {
                dialog.pick_files().await.unwrap_or_default()
            } else {
                dialog.pick_file().await.into_iter().collect()
            };

            handles
                .into_iter()
                .map(|file| file_ref_from_path(file.path()))
                .collect()
        })
    }

    fn monotonic_now(&self) -> Duration {
        static START: std::sync::LazyLock<std::time::Instant> =
            std::sync::LazyLock::new(std::time::Instant::now);
        START.elapsed()
    }

    fn new_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn create_drag_data(&self, event: PlatformDragEvent<'_>) -> Option<DragData> {
        event.as_dioxus().map(DragData::from_drag_data)
    }
}
```

#### 6.1.6 default_dioxus_platform

`default_dioxus_platform()` returns the `DioxusPlatform` chosen by
feature gating. It is the canonical out-of-component constructor,
intended for test harnesses, SSR bootstrap, and benchmarks that need a
platform handle outside a Dioxus render scope. Inside components,
prefer `use_platform()` (§6.2).

```rust
/// Resolution order:
///
/// 1. `WebPlatform` when `web` is on **and** the build target is wasm32.
/// 2. `DesktopPlatform` when `desktop` is on (and we did not match step 1).
/// 3. `NullPlatform` otherwise.
///
/// `web` on a non-wasm host falls through to step 2 or 3 — `WebPlatform`
/// only exists on wasm32. The `mobile` feature also currently lands at
/// step 3 until a dedicated mobile platform is added.
#[must_use]
pub fn default_dioxus_platform() -> Arc<dyn DioxusPlatform> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    { Arc::new(WebPlatform) }

    #[cfg(all(
        feature = "desktop",
        not(all(feature = "web", target_arch = "wasm32"))
    ))]
    { Arc::new(DesktopPlatform) }

    #[cfg(all(
        not(all(feature = "web", target_arch = "wasm32")),
        not(feature = "desktop")
    ))]
    { Arc::new(NullPlatform) }
}
```

### 6.2 Platform Hook (Dioxus-Specific)

> **Note:** The `DioxusPlatform` trait object is provided by `ArsProvider`
> (§16) via the `dioxus_platform` field on `ArsContext`. The `DioxusPlatform`
> trait, `WebPlatform`, `DesktopPlatform`, and `NullPlatform` implementations
> remain defined in §6.1 above. The core `PlatformEffects` trait (focus,
> timers, scroll-lock, positioning) is also bundled into `ArsProvider`.
> `DioxusPlatform` is a separate, Dioxus-specific abstraction for platform
> capabilities not covered by `PlatformEffects` (file pickers, clipboard,
> drag data).

```rust
/// Resolves the active `DioxusPlatform` from the surrounding `ArsProvider`
/// context.
///
/// When no `ArsProvider` is mounted, falls back to
/// [`default_dioxus_platform`] using the resolution order
/// **`web` → `desktop` → `NullPlatform`**. The `mobile` feature currently
/// falls through to `NullPlatform`; when a dedicated mobile platform is
/// added, update [`default_dioxus_platform`] with a
/// `#[cfg(feature = "mobile")]` arm.
pub fn use_platform() -> Arc<dyn DioxusPlatform> {
    try_use_context::<ArsContext>()
        .map(|ctx| Arc::clone(&ctx.dioxus_platform))
        .unwrap_or_else(|| {
            warn_missing_provider("use_platform");
            default_dioxus_platform()
        })
}
```

---

### 6.3 Platform Support Matrix

| Platform                      | Status    | Notes                                                                                                        |
| ----------------------------- | --------- | ------------------------------------------------------------------------------------------------------------ |
| Web (WASM)                    | Supported | Full feature set, same as Leptos adapter                                                                     |
| Desktop (macOS/Windows/Linux) | Supported | Via WebView renderer; all web APIs available                                                                 |
| SSR                           | Supported | Server-side rendering with hydration                                                                         |
| Mobile (iOS/Android)          | Planned   | Touch-specific considerations: no hover state, virtual keyboard viewport changes, 44px minimum touch targets |

**Mobile-specific considerations:** On mobile platforms, hover interactions degrade gracefully to focus/press states. Long-press gestures may trigger OS-level context menus, which must be accounted for in interaction handlers. The `inputmode` HTML attribute should be used to control virtual keyboard type (e.g., `numeric`, `email`, `tel`) for appropriate input fields.

---

## 7. Collections Integration

For list-based components (Select, Listbox, Menu, Combobox), Dioxus renders collection items using standard iteration:

````rust
use ars_collections::{Collection, Key};

/// Render a collection in Dioxus using for-loop iteration with key.
#[component]
pub fn CollectionView<T: Clone + PartialEq + 'static>(
    items: ReadSignal<Vec<T>>,
    key: fn(&T) -> String,
    view: fn(T) -> Element,
) -> Element {
    rsx! {
        for item in items.read().iter() {
            // Dioxus uses `key` attribute for efficient VDOM diffing
            div { key: "{key(item)}", {view(item.clone())} }
        }
    }
}

/// Select with async-loaded items using use_resource + Suspense:
///
/// ```rust
/// #[component]
/// fn CountrySelect(on_change: Option<EventHandler<String>>) -> Element {
///     let countries = use_resource(|| async { fetch_countries().await });
///
///     rsx! {
///         Select { on_value_change: on_change,
///             select::Trigger { select::ValueText {} }
///             select::Content {
///                 SuspenseBoundary {
///                     fallback: |_| rsx! { div { "Loading..." } },
///                     match &*countries.read() {
///                         Some(Ok(items)) => rsx! {
///                             for c in items.iter() {
///                                 select::Item { value: c.code.clone(), "{c.name}" }
///                             }
///                         },
///                         _ => rsx! { div { "Loading..." } },
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
````

---

## 8. Animation and Presence

Overlay exit animations are handled by the **Presence** machine (see `spec/components/overlay/presence.md`). Each overlay component (Dialog, Popover, Tooltip) composes Presence internally:

1. When the overlay's `is_open` becomes `true`, send `presence::Event::Mount`.
2. When `is_open` becomes `false`, send `presence::Event::Unmount`.
3. Presence defers unmounting until the CSS exit animation completes.
4. The adapter reads `presence_api.is_mounted()` to decide whether to render the element.

```rust
// Usage in a Dioxus overlay component:
let presence = use_machine::<presence::Machine>(presence::Props::default());
let is_mounted = presence.derive(|api| api.is_mounted());

// When dialog state changes:
// Exception: calling send inside this effect is safe — the dependency
// (is_dialog_open) is an external input, not derived from Presence state,
// so no reactive loop can form.
use_effect(move || {
    if is_dialog_open() {
        presence.send.call(presence::Event::Mount);
    } else {
        presence.send.call(presence::Event::Unmount);
    }
});

rsx! {
    if is_mounted() {
        div {
            "data-ars-state": if is_dialog_open() { "open" } else { "closed" },
            {children}
        }
    }
}
```

No adapter-level `create_presence()` helper is needed — Presence is a standard machine used via `use_machine`.

---

## 9. Named Slots (Element Props)

Leptos uses the `#[slot]` macro for named slot composition. In Dioxus, named slots are modeled as explicit `Element` props:

````rust
#[component]
pub fn Dialog(
    // Simplified example — uses plain bool for brevity.
    // Full Dialog (see spec/dioxus-components/) uses Option<Signal<bool>> for reactive controlled open.
    open: Option<bool>,
    #[props(default)] default_open: bool,
    modal: Option<bool>,
    dialog_title: Option<Element>,
    dialog_description: Option<Element>,
    on_open_change: Option<EventHandler<bool>>,
    children: Element,
) -> Element {
    let props = dialog::Props {
        open,
        default_open,
        modal: modal.unwrap_or(true),
        ..Default::default()
    };

    // NOTE: This is a simplified slots-pattern example. The full Dialog
    // implementation (see spec/dioxus-components/) uses `Context` with additional derived
    // fields (open, title_id, description_id) for granular reactivity.
    let UseMachineReturn { state, send, service, context_version } = use_machine::<dialog::Machine>(props);
    use_context_provider(|| Context { state, send, service, context_version });

    // Wire ARIA IDs from dialog context for accessibility.
    // The full Dialog (see spec/dioxus-components/) derives these from the machine's ID generator.
    // Simplified illustration — the full implementation (see spec/dioxus-components/) uses derive() for reactive IDs.
    // NOTE: peek() avoids subscribing to the service signal, preventing full re-renders on every event.
    let base_id = service.peek().props().id();
    let title_id = format!("{base_id}-title");
    let description_id = format!("{base_id}-description");

    rsx! {
        div {
            "aria-labelledby": title_id,
            "aria-describedby": description_id,
            {children}
        }
    }
}

/// Usage:
/// ```rust
/// Dialog {
///     dialog_title: rsx! { h2 { "My Title" } },
///     dialog_description: rsx! { p { "Description text" } },
///     dialog::Trigger { button { "Open" } }
///     dialog::Content { /* ... */ }
/// }
/// ```
````

## 10. Effect Cleanup and Event Safety

**Problem.** During effect cleanup, removing event listeners can itself trigger synthetic events — `blur` fires when a focused element's listener is removed, `pointerup` may arrive after a transition completes but before new effects are wired. If cleanup and setup overlap, stale callbacks execute against new component state, causing panics or incorrect behavior. In Dioxus, this problem has an additional dimension: the desktop renderer processes events synchronously on the main thread, while the web renderer defers event dispatch to microtasks. Both paths must be handled.

**Rules:**

1. **Cleanup ordering.** All listener removals MUST execute before any new listeners are registered. The adapter enforces this by splitting the hook lifecycle into two phases: a synchronous cleanup phase and a subsequent setup phase.

2. **Idempotent cleanup with `use_drop`.** Use Dioxus `use_drop` for deterministic cleanup instead of relying on `Drop` impls on signals. The `use_drop` callback MUST be safe to call multiple times — guard against double-removal with a `Cell<bool>` flag stored in the hook state.

3. **`Signal::try_write()` for stale-check.** Long-lived callbacks that capture a `Signal` should use `try_write()` (or `try_read()`) before mutating. If the signal's owning scope has been dropped, `try_write()` returns `Err` and the write is silently skipped. This replaces the `WeakSend` pattern used in Leptos.

4. **Desktop vs. web timing.** Desktop Dioxus may deliver events synchronously during the same tick as cleanup. Web Dioxus defers to microtasks. The cleanup logic must not assume either ordering — always check validity before acting.

5. **Batch removals, then batch registrations.** `use_safe_event_listeners()` MUST remove all previously registered listeners synchronously before adding any replacement listeners. `use_safe_event_listener()` is a single-listener wrapper around the batch helper.

```rust
use std::{cell::Cell, fmt, rc::Rc};
use dioxus::prelude::*;
use web_sys::wasm_bindgen::{JsCast, closure::Closure};

type ListenerHandler = Rc<dyn Fn(web_sys::Event)>;

struct RegisteredListener {
    target: web_sys::EventTarget,
    event_name: &'static str,
    capture: bool,
    active: Rc<Cell<bool>>,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

/// DOM event listener definition for batch lifecycle registration.
#[derive(Clone)]
pub struct SafeEventListener {
    event_name: &'static str,
    options: SafeEventListenerOptions,
    handler: ListenerHandler,
}

/// Options passed to DOM event listener registration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SafeEventListenerOptions {
    pub capture: bool,
    pub passive: bool,
    pub once: bool,
}

impl fmt::Debug for SafeEventListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SafeEventListener")
            .field("event_name", &self.event_name)
            .finish_non_exhaustive()
    }
}

impl SafeEventListener {
    pub fn new(event_name: &'static str, handler: impl Fn(web_sys::Event) + 'static) -> Self {
        Self::new_with_options(event_name, SafeEventListenerOptions::default(), handler)
    }

    pub fn new_with_options(
        event_name: &'static str,
        options: SafeEventListenerOptions,
        handler: impl Fn(web_sys::Event) + 'static,
    ) -> Self {
        Self { event_name, options, handler: Rc::new(handler) }
    }
}

// Desktop/mobile targets should use the DioxusPlatform abstraction trait for equivalent functionality.
#[cfg(feature = "web")]
/// Attaches an event listener with framework-managed lifecycle cleanup.
///
/// Uses raw `web_sys::Closure` and `EventTarget::add_event_listener_with_callback`
/// directly because framework-specific cleanup primitives (`use_drop`/`Signal` in
/// Dioxus) require owning the Closure handle. The ars-dom `EventListenerHandle`
/// utility (11-dom-utilities.md §7) does not integrate with framework reactivity
/// systems. See v93 follow-up discussion.
pub fn use_safe_event_listener(
    target: Signal<Option<web_sys::EventTarget>>,
    event_name: &'static str,
    handler: impl Fn(web_sys::Event) + 'static,
) {
    use_safe_event_listeners(target, vec![SafeEventListener::new(event_name, handler)]);
}

/// Attaches multiple event listeners with batched cleanup before registration.
pub fn use_safe_event_listeners(
    target: Signal<Option<web_sys::EventTarget>>,
    listeners: Vec<SafeEventListener>,
) {
    let mut closure_handle: CopyValue<Vec<RegisteredListener>> =
        use_hook(|| CopyValue::new(Vec::new()));
    let mut cleaned_up = use_hook(|| CopyValue::new(false));

    // Track a signal so stale callbacks can bail out.
    let alive = use_signal(|| true);
    let listeners = Rc::new(listeners);

    let listeners_effect = listeners.clone();
    use_effect(move || {
        // Phase 1: Synchronous cleanup of previous listeners from their
        // originally registered targets.
        let previous = core::mem::take(&mut *closure_handle.write());
        for previous in previous {
            previous.active.set(false);
            previous.target.remove_event_listener_with_callback_and_bool(
                previous.event_name,
                previous.closure.as_ref().unchecked_ref(),
                previous.capture,
            )
            .ok();
        }

        let Some(el) = target.read().clone() else { return };

        // Phase 2: Register new listeners with scope and per-registration guards.
        let mut registrations = Vec::with_capacity(listeners_effect.len());
        for listener in listeners_effect.iter() {
            let alive_signal = alive;
            let handler = listener.handler.clone();
            let registration_active = Rc::new(Cell::new(true));
            let registration_active_for_closure = registration_active.clone();
            let closure = Closure::new(move |event: web_sys::Event| {
                // Stale-check: if the signal scope is gone, skip.
                if registration_active_for_closure.get()
                    && alive_signal.try_read().is_ok_and(|alive| *alive)
                {
                    handler(event);
                }
            });

            let listener_options = web_sys::AddEventListenerOptions::new();
            listener_options.set_capture(listener.options.capture);
            listener_options.set_passive(listener.options.passive);
            listener_options.set_once(listener.options.once);

            el.add_event_listener_with_callback_and_add_event_listener_options(
                listener.event_name,
                closure.as_ref().unchecked_ref(),
                &listener_options,
            )
            .expect("addEventListener");

            registrations.push(RegisteredListener {
                target: el.clone(),
                event_name: listener.event_name,
                capture: listener.options.capture,
                active: registration_active,
                closure,
            });
        }

        closure_handle.set(registrations);
        cleaned_up.set(false);
    });

    // use_drop for deterministic cleanup — idempotent.
    use_drop(move || {
        if cleaned_up() {
            return;
        }
        cleaned_up.set(true);

        // Signal the stale-check that this scope is dead.
        if let Ok(mut w) = alive.try_write() {
            *w = false;
        }

        if let Ok(mut registrations) = closure_handle.try_write() {
            let previous = core::mem::take(&mut *registrations);
            for previous in previous {
                previous.active.set(false);
                previous.target.remove_event_listener_with_callback_and_bool(
                    previous.event_name,
                    previous.closure.as_ref().unchecked_ref(),
                    previous.capture,
                )
                .ok();
            }
        }
    });
}
```

---

## 11. API Naming Conventions

All `ars-dioxus` components follow uniform naming conventions for accessors, state, and callbacks. These rules apply across every component in the adapter.

### 11.1 Boolean Accessors — `is_*()` Methods

Boolean state is always accessed through `is_*()` methods, never bare field access:

```rust
api.is_disabled()       // not: api.disabled
api.is_open()           // not: api.open
api.is_checked()        // not: api.checked
api.is_expanded()       // not: api.expanded
api.is_readonly()       // not: api.readonly
api.is_focused()        // not: api.focused
api.is_indeterminate()  // not: api.indeterminate
```

### 11.2 Non-Boolean State — Getter Methods or Field Access

Non-boolean values use getter methods (or direct field access for simple data):

```rust
api.value()             // current value (String, number, etc.)
api.selected_items()    // current selection set
api.highlighted_key()   // currently highlighted item key
api.placeholder()       // placeholder text
api.orientation()       // Orientation enum
```

### 11.3 Event Callbacks — `on_*` Naming

All callback props use the `on_*` prefix:

```rust
on_change               // value changed
on_select               // item selected
on_open_change          // open state toggled
on_checked_change       // checked state toggled
on_press                // press interaction completed
on_focus                // element received focus
on_blur                 // element lost focus
on_dismiss              // overlay dismissed
```

### 11.4 Module-Scoped Compound Component Naming

Adapter component specs that expose compound parts must use module scoping instead of
repeating the component name in every part symbol:

```rust
pub mod dialog {
    #[component]
    pub fn Dialog(props: DialogProps) -> Element

    #[component]
    pub fn Trigger(children: Element) -> Element

    #[component]
    pub fn Content(children: Element) -> Element
}
```

Rules:

- The root component uses the bare component name (`dialog::Dialog`, `tooltip::Tooltip`).
- Child parts drop the redundant component prefix (`dialog::Trigger`, not `DialogTrigger`).
- The primary child-part context inside the module is named `Context`.
- Secondary contexts or helpers use descriptive non-prefixed names (`GroupContext`, `QueueContext`, `Overlay`, `Control`).
- Expect or panic messages must use the module-qualified part name (`dialog::Trigger must be used inside Dialog`).

### 11.5 Summary Table

| Category             | Convention                            | Examples                                                 |
| -------------------- | ------------------------------------- | -------------------------------------------------------- |
| Boolean accessor     | `is_*()` method                       | `is_disabled()`, `is_open()`, `is_checked()`             |
| Non-boolean accessor | `value()` / `selected_items()` method | `value()`, `highlighted_key()`, `orientation()`          |
| Event callback       | `on_*` prop                           | `on_change`, `on_select`, `on_open_change`               |
| Compound parts       | module-scoped symbols                 | `dialog::Trigger`, `tooltip::Content`, `toast::Provider` |

---

## 12. Callback Naming Convention

All public callback props follow a consistent naming convention across `ars-dioxus` components:

| Pattern                | Usage                                                         | Examples                                                     |
| ---------------------- | ------------------------------------------------------------- | ------------------------------------------------------------ |
| `on_<property>_change` | Fires when a **value** changes (controlled component pattern) | `on_value_change`, `on_open_change`, `on_checked_change`     |
| `on_<action>`          | Fires on a **discrete user action** (not a state change)      | `on_press`, `on_submit`, `on_dismiss`, `on_focus`, `on_blur` |

**Rules:**

- Value-change callbacks always receive the **new value** as their argument (e.g., `EventHandler<bool>` for `on_open_change`)
- Action callbacks receive either no argument or an event-specific payload — never the full component state
- Callback props are always `Option<EventHandler<T>>` — omitting a callback is valid and means the consumer does not observe that event
- Use Dioxus `EventHandler` (not `Callback`) for consistency with the Dioxus ecosystem
- Convention: `EventHandler<T>` for user-facing callback props (`onclick`, `on_change`). `Callback<T, R>` for internal machine dispatch (send events to state machine).

---

## 13. Event Handling

### 13.1 Event Mapping

```rust
use dioxus::prelude::*;

/// Dioxus KeyboardEvent -> ars-core KeyboardKey.
pub fn dioxus_key_to_keyboard_key(key: &Key) -> (KeyboardKey, Option<char>) {
    match key {
        Key::Character(c) => {
            if c == " " {
                return (KeyboardKey::Space, Some(' '));
            }
            let mut chars = c.chars();
            let ch = chars.next().filter(|_| chars.next().is_none());
            (KeyboardKey::from_key_str(c), ch)
        }
        Key::Enter => (KeyboardKey::Enter, None),
        Key::Escape => (KeyboardKey::Escape, None),
        Key::Tab => (KeyboardKey::Tab, None),
        Key::ArrowUp => (KeyboardKey::ArrowUp, None),
        Key::ArrowDown => (KeyboardKey::ArrowDown, None),
        Key::ArrowLeft => (KeyboardKey::ArrowLeft, None),
        Key::ArrowRight => (KeyboardKey::ArrowRight, None),
        Key::Home => (KeyboardKey::Home, None),
        Key::End => (KeyboardKey::End, None),
        Key::PageUp => (KeyboardKey::PageUp, None),
        Key::PageDown => (KeyboardKey::PageDown, None),
        Key::Backspace => (KeyboardKey::Backspace, None),
        Key::Delete => (KeyboardKey::Delete, None),
        Key::F1 => (KeyboardKey::F1, None),
        Key::F2 => (KeyboardKey::F2, None),
        Key::F3 => (KeyboardKey::F3, None),
        Key::F4 => (KeyboardKey::F4, None),
        Key::F5 => (KeyboardKey::F5, None),
        Key::F6 => (KeyboardKey::F6, None),
        Key::F7 => (KeyboardKey::F7, None),
        Key::F8 => (KeyboardKey::F8, None),
        Key::F9 => (KeyboardKey::F9, None),
        Key::F10 => (KeyboardKey::F10, None),
        Key::F11 => (KeyboardKey::F11, None),
        Key::F12 => (KeyboardKey::F12, None),
        _ => (KeyboardKey::Unidentified, None),
    }
}
```

### 13.2 Native Element Handler Deduplication

When rendering a machine's keyboard handlers onto a native interactive element (e.g., `<button>`), the adapter MUST strip handlers that duplicate native behavior:

- Native `<button>` fires `click` on Space keyup — the adapter strips the machine's Space key handler to avoid double activation.
- Native `<a>` fires `click` on Enter — the adapter strips the machine's Enter key handler.

The machine always generates the full handler set (it is DOM-element-agnostic). Deduplication is the adapter's responsibility.

> **Parity note:** This mirrors the Leptos adapter §5.3 "Native Element Handler Deduplication". Both adapters apply the same deduplication rules.

---

## 14. Compound Component Pattern: Module-Scoped Parts

Dioxus compound components use `use_context_provider` in the root component and `try_use_context` in child parts. This example shows a generic Popover with the module-scoped naming pattern.

```rust
pub mod popover {
    use std::rc::Rc;
    use dioxus::prelude::*;
    use ars_core::Service;

    // --- Context type shared by all parts ---

    #[derive(Clone, Copy)]
    pub struct Context {
        pub open: ReadSignal<bool>,
        pub send: Callback<popover::Event>,
        pub trigger_id: ReadSignal<String>,
        pub content_id: ReadSignal<String>,
        pub service: Signal<Service<popover::Machine>>,
        pub context_version: ReadSignal<u64>,
    }

    // --- Root: owns the machine, provides context ---

    #[derive(Props, Clone, PartialEq)]
    pub struct PopoverProps {
        pub open: Option<Signal<bool>>,
        #[props(default = false)]
        pub default_open: bool,
        pub children: Element,
    }

    #[component]
    pub fn Popover(props: PopoverProps) -> Element {
        let core_props = popover::Props {
            open: props.open.as_ref().map(|s| *s.peek()),
            default_open: props.default_open,
            ..Default::default()
        };

        let machine = use_machine::<popover::Machine>(core_props);

        let UseMachineReturn { state, send, .. } = machine;

        // Watch controlled open prop. Uses deferred use_effect (not body-level sync)
        // because open/close dispatches Open/Close events, which is an intentional
        // exception to §19's body-level sync rule.
        // NOTE: Hooks are called unconditionally to maintain stable hook ordering
        // across re-renders (Dioxus requirement). The effect body gates on
        // `props.open` being `Some`.
        let send_clone = send;
        let mut prev_open: Signal<Option<bool>> = use_signal(|| None);
        use_effect(move || {
            if let Some(open_sig) = props.open {
                let new_open = *open_sig.read();
                let prev = prev_open.peek().clone();
                if prev.as_ref() != Some(&new_open) {
                    if prev.is_some() {
                        if new_open {
                            send_clone.call(popover::Event::Open);
                        } else {
                            send_clone.call(popover::Event::Close);
                        }
                    }
                    *prev_open.write() = Some(new_open);
                }
            }
        });

        let open = machine.derive(|api| api.is_open());

        // Auto-generated IDs from the machine — avoids collisions with multiple instances.
        let trigger_id: ReadSignal<String> = machine.derive(|api| api.trigger_id().to_string()).into();
        let content_id: ReadSignal<String> = machine.derive(|api| api.content_id().to_string()).into();

        // Provide context to all child parts
        use_context_provider(|| Context {
            open: open.into(),
            send,
            trigger_id,
            content_id,
            service: machine.service,
            context_version: machine.context_version,
        });

        rsx! { {props.children} }
    }

    // --- Trigger: reads context, renders the trigger element ---

    #[derive(Props, Clone, PartialEq)]
    pub struct TriggerProps {
        pub children: Element,
    }

    #[component]
    pub fn Trigger(props: TriggerProps) -> Element {
        let ctx = try_use_context::<Context>()
            .expect("popover::Trigger must be used inside Popover");

        let [(scope_attr, scope_val), (part_attr, part_val)] = popover::Part::Trigger.data_attrs();
        rsx! {
            button {
                r#type: "button",
                id: ctx.trigger_id(),
                "{scope_attr}": scope_val,
                "{part_attr}": part_val,
                "aria-haspopup": "dialog",
                "aria-expanded": (*ctx.open.read()).to_string(),
                "aria-controls": ctx.content_id(),
                onclick: move |_| ctx.send.call(popover::Event::Toggle),
                {props.children}
            }
        }
    }

    // --- Content: reads context, conditionally renders ---

    #[derive(Props, Clone, PartialEq)]
    pub struct ContentProps {
        pub children: Element,
    }

    #[component]
    pub fn Content(props: ContentProps) -> Element {
        let ctx = try_use_context::<Context>()
            .expect("popover::Content must be used inside Popover");

        let [(scope_attr, scope_val), (part_attr, part_val)] = popover::Part::Content.data_attrs();
        rsx! {
            if *ctx.open.read() {
                div {
                    id: ctx.content_id(),
                    role: "dialog",
                    "{scope_attr}": scope_val,
                    "{part_attr}": part_val,
                    "data-ars-state": "open",
                    onkeydown: move |e: KeyboardEvent| {
                        if dioxus_key_to_keyboard_key(&e.key()).0 == KeyboardKey::Escape {
                            ctx.send.call(popover::Event::Close);
                        }
                    },
                    {props.children}
                }
            }
        }
    }

    // --- Usage ---
    // rsx! {
    //     Popover {
    //         popover::Trigger { "Click me" }
    //         popover::Content {
    //             p { "Popover body content" }
    //             button {
    //                 onclick: move |_| { /* close */ },
    //                 "Close"
    //             }
    //         }
    //     }
    // }
}
```

---

> **Per-component adapter examples** have been extracted to individual files under `spec/dioxus-components/{category}/{component}.md`.
> Use `cargo run -p spec-tool -- deps <component>` to find the Dioxus adapter file for any component.

## 15. SSR Support

```rust
// For SSR, Dioxus renders components to a string.
// Components detect the SSR environment via cfg(feature = "ssr").

#[cfg(feature = "ssr")]
pub fn render_to_string(app: fn() -> Element) -> String {
    // dioxus::ssr::render: renders a VirtualDom to string for SSR
    // Dioxus 0.7: use VirtualDom-based rendering for SSR.
    let mut dom = dioxus::prelude::VirtualDom::new(app);
    dom.rebuild_in_place();
    // Dioxus 0.7 re-exports the dioxus_ssr crate as the dioxus::ssr module path.
    // https://docs.rs/dioxus/latest/dioxus/index.html#reexport.ssr
    dioxus::ssr::render(&dom)
}

pub mod tooltip {
    // Components that need SSR-aware behavior:
    #[component]
    pub fn Content(children: Element) -> Element {
        let ctx = try_use_context::<Context>()
            .expect("tooltip::Content must be used inside Tooltip");

        #[cfg(feature = "ssr")]
        {
            // During SSR: render with display:none, full content for SEO
            return rsx! {
                div {
                    role: "tooltip",
                    style: "display: none",
                    {children}
                }
            };
        }

        let [(scope_attr, scope_val), (part_attr, part_val)] = tooltip::Part::Content.data_attrs();

        rsx! {
            if *ctx.is_visible.read() {
                div {
                    role: "tooltip",
                    "{scope_attr}": scope_val,
                    "{part_attr}": part_val,
                    {children}
                }
            }
        }
    }
}
```

---

## 16. ArsProvider Context

`ArsProvider` is the single root provider — the formerly separate `LocaleProvider`,
`PlatformEffectsProvider`, `ArsStyleProvider`, and `PlatformProvider` are all subsumed.
The adapter-level context wraps core `ArsContext` values in reactive signals and
includes the style strategy and the Dioxus-specific platform capabilities trait object.

```rust
use ars_i18n::{Direction, IntlBackend, Locale};
use ars_core::{
    ArsContext as CoreCtx, Arc, ColorMode, I18nRegistries, ModalityContext, PlatformEffects,
    StyleStrategy,
};

/// Reactive environment context published by the Dioxus ArsProvider adapter.
#[derive(Clone)]
pub struct ArsContext {
    pub locale: Signal<Locale>,
    pub direction: Memo<Direction>,
    pub color_mode: Signal<ColorMode>,
    pub disabled: Signal<bool>,
    pub read_only: Signal<bool>,
    pub id_prefix: Signal<Option<String>>,
    pub portal_container_id: Signal<Option<String>>,
    pub root_node_id: Signal<Option<String>>,
    pub platform: Arc<dyn PlatformEffects>,
    pub modality: Arc<dyn ModalityContext>,
    pub intl_backend: Arc<dyn IntlBackend>,
    pub i18n_registries: Arc<I18nRegistries>,
    pub style_strategy: StyleStrategy,
    /// Dioxus adapter-specific: platform services for file pickers, clipboard, drag data.
    pub dioxus_platform: Arc<dyn DioxusPlatform>,
}
```

The `ArsProvider` component, its props, and rendering are specified in
`spec/dioxus-components/utility/ars-provider.md`. The component publishes
`ArsContext` via `use_context_provider` and renders a `<div dir="{dir}">` wrapper.
Although Dioxus context values only require `Clone + 'static`, ars-ui keeps the
Dioxus-local `dioxus_platform` handle in `Arc<dyn DioxusPlatform>` so the adapter
matches the shared `Send + Sync` ownership model used across the rest of the crate
family.
The shared fields above mirror the canonical `ars_core::ArsContext`; only
`dioxus_platform` is adapter-specific.
The Dioxus adapter resolves `platform` via feature flags: `WebPlatformEffects` (web),
`DesktopPlatformEffects` (desktop), `NullPlatformEffects` (SSR/tests/mobile fallback).

### 16.1 use_locale()

All ArsProvider fallback helpers in this section (`use_locale()`,
`use_intl_backend()`, `use_modality_context()`, `t()`, and `use_platform()`) route through
`warn_missing_provider()`. That helper emits `log::warn!` only when the
`ars-dioxus/debug` feature is enabled.

```rust
/// Returns the current locale signal. The returned signal is **read-only in practice**:
/// writing to it does not propagate changes to other components. Use `ArsProvider`
/// to change the locale for a subtree.
pub fn use_locale() -> Signal<Locale> {
    // Intentional: use_signal called unconditionally to satisfy hook ordering rules
    // (Dioxus hooks must be called in the same order on every render).
    let fallback = use_signal(|| Locale::parse("en-US").expect("en-US is always a valid BCP 47 locale"));
    try_use_context::<ArsContext>()
        .map(|c| c.locale)
        .unwrap_or_else(|| {
            warn_missing_provider("use_locale");
            fallback
        })
}
```

### 16.2 use_number_formatter()

```rust
use ars_i18n::number;

/// Resolve a memoized number formatter from ArsProvider locale context.
///
/// `use_number_formatter()` is the public ambient-locale formatting helper for
/// Dioxus components. The `options` closure may read reactive props/signals;
/// the memo rebuilds when either the locale signal or the closure output
/// changes.
pub fn use_number_formatter<F>(options: F) -> Memo<number::Formatter>
where
    F: Fn() -> number::FormatOptions + 'static,
{
    use_resolved_number_formatter(None, options)
}

/// Resolve a memoized formatter from an explicit locale override or the
/// ambient ArsProvider locale.
///
/// This helper is adapter-internal so component implementations with a
/// `locale` prop can preserve the documented resolution chain without pushing
/// formatter state into `ars_core::Env`.
pub(crate) fn use_resolved_number_formatter<F>(
    adapter_props_locale: Option<&Locale>,
    options: F,
) -> Memo<number::Formatter>
where
    F: Fn() -> number::FormatOptions + 'static,
{
    let explicit_locale = adapter_props_locale.cloned();
    let locale = use_locale();

    use_memo(move || {
        let resolved_locale = explicit_locale.clone().unwrap_or_else(|| locale.read().clone());
        number::Formatter::new(&resolved_locale, options())
    })
}
```

**Prelude export:** `pub use crate::use_number_formatter;`

### 16.3 Environment Resolution Utilities

These adapter-only utilities resolve environment values from `ArsProvider` context
before passing them to core code via the `Env` struct and `Messages` parameter.
Core component code never calls these functions directly.

See `04-internationalization.md` §2.3.1 for the three-level resolution chain
(prop override -> ArsProvider -> default) and §2.3.2 for ICU provider resolution.

```rust
use std::sync::Arc;

use ars_core::Env;
use ars_i18n::{Locale, IntlBackend, StubIntlBackend, ComponentMessages, I18nRegistries};

/// Resolve locale from an optional adapter prop override or ArsProvider context.
///
/// Resolution chain:
/// 1. Explicit adapter prop override (if provided)
/// 2. ArsProvider locale signal (via `use_locale()`)
/// 3. Fallback: `en-US`
///
/// This is an **adapter-only** utility — NOT available in core crates. Core code
/// receives a fully-resolved `Locale` via `Env`.
///
/// **Note:** Unlike `use_locale()` which returns `Signal<Locale>`, this function
/// reads the signal and returns a plain `Locale` for use in `Env` construction.
fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| {
            // Read the reactive locale signal to get a plain Locale value.
            // use_locale() returns Signal<Locale>; .read() subscribes the
            // component to locale changes, ensuring re-render on locale change.
            use_locale().read().clone()
        })
}

/// Resolve the ICU provider from ArsProvider context.
///
/// Falls back to `StubIntlBackend` (English-only, zero dependencies) if no
/// `ArsProvider` is present.
///
/// This is an **adapter-only** utility — NOT available in core crates. Core code
/// receives the provider via `Env.intl_backend`.
fn use_intl_backend() -> Arc<dyn IntlBackend> {
    try_use_context::<ArsContext>()
        .map(|ctx| ctx.intl_backend.clone())
        .unwrap_or_else(|| {
            warn_missing_provider("use_intl_backend");
            Arc::new(StubIntlBackend)
        })
}

/// Resolve per-component i18n messages from an optional adapter prop override,
/// ArsProvider i18n registries, or built-in defaults.
///
/// Resolution chain:
/// 1. Explicit adapter prop override (if provided)
/// 2. ArsProvider i18n registries (locale-keyed lookup)
/// 3. Fallback: `M::default()` (built-in English defaults)
///
/// This is an adapter-level hook helper. It reads locale and registries from
/// `ArsProvider` context, then delegates the pure resolution logic to
/// `ars_core::resolve_messages()`.
fn use_messages<M: ComponentMessages + Send + Sync + 'static>(
    adapter_props_messages: Option<&M>,
    adapter_props_locale: Option<&Locale>,
) -> M {
    let locale = resolve_locale(adapter_props_locale);
    let registries = try_use_context::<ArsContext>()
        .map(|ctx| ctx.i18n_registries.clone())
        .unwrap_or_else(|| Arc::new(I18nRegistries::new()));
    ars_core::resolve_messages(adapter_props_messages, registries.as_ref(), &locale)
}
```

### 16.4 t() — Translatable Text Resolver

```rust
use ars_i18n::Translate;

/// Resolve a user-defined `Translate` enum variant into a text string for rendering.
///
/// Reads the current locale and ICU provider from `ArsProvider` context via
/// `Signal::read()`, which subscribes the calling component to locale changes.
/// When locale changes, the component re-renders and `t()` produces the new
/// string (component-level reactivity — standard Dioxus model).
///
/// Included in `ars_dioxus::prelude`.
///
/// See `04-internationalization.md` §7.4 for the `Translate` trait definition
/// and §7.5 for the `t()` function contract.
pub fn t<T: Translate>(msg: T) -> String {
    try_use_context::<ArsContext>()
        .map(|ctx| msg.translate(&ctx.locale.read(), &*ctx.intl_backend))
        .unwrap_or_else(|| {
            warn_missing_provider("t");
            let fallback = Locale::parse("en-US").expect("en-US is always a valid BCP 47 locale");
            msg.translate(&fallback, &StubIntlBackend)
        })
}
```

**Prelude export:** `pub use crate::i18n::t;`

> **Why `t()` doesn't use hooks:** Unlike `use_locale()`, `t()` reads context via
> `try_use_context` (a context lookup, not a hook-slot allocation) and then reads
> signals directly. This makes `t()` safe to call inside conditionals and loops
> within `rsx!` — it does not affect hook ordering.

---

## 17. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Pure unit tests on the machines (no Dioxus dependency)
    #[test]
    fn dialog_opens_and_closes() {
        let props = dialog::Props::default();
        let env = Env::default();
        let messages = dialog::Messages::default();
        let mut svc = Service::<dialog::Machine>::new(props, env, messages);

        assert_eq!(*svc.state(), dialog::State::Closed);
        svc.send(dialog::Event::Open);
        assert_eq!(*svc.state(), dialog::State::Open);
        svc.send(dialog::Event::Close);
        assert_eq!(*svc.state(), dialog::State::Closed);
    }
}

// DOM tests using Dioxus test renderer:
#[test]
fn checkbox_renders_aria_checked() {
    let mut dom = VirtualDom::new(|| rsx! {
        Checkbox { default_checked: checkbox::State::Unchecked,
            checkbox::Control {
                checkbox::Indicator { "✓" }
            }
        }
    });
    dom.rebuild_in_place();
    // render: takes a &VirtualDom, used for component-level test rendering
    let html = dioxus::ssr::render(&dom);

    assert!(html.contains(r#"role="checkbox""#));
    assert!(html.contains(r#"aria-checked="false""#));
    assert!(html.contains(r#"data-ars-scope="checkbox""#));
}
```

Dioxus supports two complementary testing styles:

1. **Raw adapter examples** use `VirtualDom` plus `dioxus::ssr::render(...)` for
   focused adapter/SSR checks like the example above.
2. **Shared harness DOM behavior tests** use the framework-agnostic
   `TestHarness` from [15-test-harness.md](../testing/15-test-harness.md). In
   that setup, the Dioxus backend owns `ArsProvider` wrapping for
   `mount_with_locale(...)`, and reactivity is synchronized through
   a backend-owned browser task boundary behind the harness `flush()` /
   `tick()` helpers. Public `dioxus-web` launch APIs consume the `VirtualDom`,
   so shared-harness tests must not assume access to `wait_for_work()`.
3. **Non-web (Desktop, mobile, SSR) test passes** use
   `ars_test_harness_dioxus::desktop::DesktopHarness`, a headless `VirtualDom`
   wrapper that exercises the `cfg(not(feature = "web"))` graceful-degrade path
   adapter components follow on those platforms. The harness exposes
   `launch(...)` / `launch_with_props(...)` /
   `launch_with_locale(builder, locale)` for mounting plus a `flush()` drain
   that mirrors the wasm-tier `HarnessBackend::flush` contract, and is the
   canonical target for spec checklist items that require validation "on the
   target runtime rather than only in a browser harness" — for example
   [`spec/dioxus-components/utility/dismissable.md`](../dioxus-components/utility/dismissable.md)
   §29-§31. The whole module is gated `cfg(not(target_arch = "wasm32"))`, so it
   builds and runs through `cargo test` on native CI without GUI dependencies
   while leaving the wasm-pack path for the existing `DioxusHarnessBackend`
   untouched. See [`spec/testing/15-test-harness.md`](../testing/15-test-harness.md)
   §5.4 for the full API and rationale.

Shared-harness Dioxus tests should not rely on ad hoc zero-delay timer shims to
observe locale or DOM updates. Interaction helpers already flush the Dioxus
reactivity cycle before returning, and explicit `tick()` / `flush()` calls are
the documented way to request an extra post-event boundary when a test needs one.

---

## 18. Leptos / Dioxus API Mapping

Quick reference for translating between the two adapter APIs:

| Concept              | Leptos (`ars-leptos`)                                                                                                 | Dioxus (`ars-dioxus`)                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Basic hook           | `use_machine(props)`                                                                                                  | `use_machine(props)`                                                                                          |
| Reactive props       | `use_machine_with_reactive_props(signal)`                                                                             | `use_machine(props)` (auto-syncs props internally)                                                            |
| API access           | `machine.derive(\|api\| ...)`                                                                                         | `machine.derive(\|api\| ...)`                                                                                 |
| One-shot API read    | `machine.with_api_snapshot(\|api\| ...)` (returns `T`) or `machine.with_api_ephemeral(\|eref\| ...)` (`EphemeralRef`) | `machine.with_api_snapshot(\|api\| ...)` (returns `T`)                                                        |
| Service wrapper      | `StoredValue<Service<M>>`                                                                                             | `Signal<Service<M>>`                                                                                          |
| Cleanup              | `on_cleanup`                                                                                                          | `use_drop`                                                                                                    |
| Context (required)   | `use_context::<T>().expect(msg)` → `T` (descriptive panic)                                                            | `use_context::<T>()` → `T` (panics); adapter uses `try_use_context::<T>().expect(msg)` for descriptive panics |
| Context (optional)   | `use_context::<T>()` → `Option<T>`                                                                                    | `try_use_context::<T>()` → `Option<T>`                                                                        |
| Context provider     | `provide_context(value)`                                                                                              | `use_context_provider(\|\| value)`                                                                            |
| Callback helper      | `emit(cb, value)`                                                                                                     | `emit(cb, value)`                                                                                             |
| Controlled prop sync | `use_controlled_prop(sig, send, fn)`                                                                                  | `use_controlled_prop_sync(send, val, fn)`                                                                     |
| `derive()` send      | Uses no-op panic closure (same as Dioxus — StoredValue borrow prevents re-entrant write)                              | Uses no-op panic closure (Signal read lock prevents write)                                                    |
| ID generation        | `use_id(scope)` [^1]                                                                                                  | `use_stable_id(prefix)` [^1]                                                                                  |

[^1]: Neither `use_id` (Leptos) nor `use_stable_id` (Dioxus) is hydration-safe. SSR+hydration users must provide explicit `id` props until a deterministic tree-position-based ID scheme is implemented. See §20 for details.

---

## 19. Controlled Value Helper

All controlled prop watchers follow the same pattern: track previous value, skip initial, send event on change. This helper extracts the repeated logic using **body-level synchronous sync** (not `use_effect`) to avoid one-frame stale state:

```rust
/// Watch a prop value and dispatch an event when it changes.
/// Runs synchronously in the component body — NOT deferred via use_effect.
/// Skips the initial mount (machine already has correct initial value from props).
///
/// **IMPORTANT**: All controlled value watchers MUST use body-level sync,
/// not `use_effect`. Deferred watchers cause a one-frame stale state window
/// where `connect()` returns attributes based on the old value.
pub fn use_controlled_prop_sync<T: Clone + PartialEq + 'static, E: 'static>(
    send: Callback<E>,
    current: T,
    event_fn: impl Fn(T) -> E,
) {
    let mut prev: Signal<Option<T>> = use_signal(|| None);
    let p = prev.peek().clone();
    if p.as_ref() != Some(&current) {
        if p.is_some() {
            send.call(event_fn(current.clone()));
        }
        *prev.write() = Some(current);
    }
}

/// Like `use_controlled_prop_sync`, but accepts `Option<T>` for props that may be
/// `None` (uncontrolled mode). The internal `use_signal` hook is **always** called
/// to preserve stable hook ordering. When `current` is `None`, no event is sent.
pub fn use_controlled_prop_sync_optional<T: Clone + PartialEq + 'static, E: 'static>(
    send: Callback<E>,
    current: Option<T>,
    event_fn: impl Fn(T) -> E,
) {
    let mut prev: Signal<Option<T>> = use_signal(|| None);
    if let Some(val) = current {
        let p = prev.peek().clone();
        if p.as_ref() != Some(&val) {
            if p.is_some() {
                send.call(event_fn(val.clone()));
            }
            *prev.write() = Some(val);
        }
    } else {
        // Uncontrolled mode: clear previous value without sending an event.
        if prev.peek().is_some() {
            *prev.write() = None;
        }
    }
}
```

### 19.1 Event Callback Helper

````rust
/// Emit a value through an optional Dioxus EventHandler.
///
/// # Example
/// ```rust
/// emit(props.on_value_change.as_ref(), selected_value);
/// ```
pub fn emit<T: 'static>(handler: Option<&EventHandler<T>>, value: T) {
    if let Some(h) = handler {
        h.call(value);
    }
}

/// Emit a mapped value through an optional callback.
pub fn emit_map<T, U: 'static>(handler: Option<&EventHandler<U>>, value: T, f: impl Fn(T) -> U) {
    if let Some(h) = handler {
        h.call(f(value));
    }
}
````

---

### 19.2 Generated IDs and Hydration

The `dioxus_id_counter()` function (thread-local on WASM, `AtomicU64` on
native) is **NOT hydration-safe**. During SSR, the server increments the counter
in rendering order; on hydration, the client may increment differently due to
lazy loading, code splitting, or Suspense boundaries. This causes ARIA attribute
mismatches (`aria-labelledby`, `aria-describedby`, `aria-controls` pointing to
wrong elements).

**Requirements:**

1. **Explicit IDs for SSR + hydration**: Components rendered through SSR and
   hydrated on the client MUST receive explicit `id` props from application code
   or from a higher-level component API that can serialize and restore the same
   value. Generated IDs are only a fallback for client-only rendering and for
   non-hydrated SSR output.

2. **SSR counter reset**: When the `dioxus/server` feature is active, the ID counter MUST be reset at the start of each SSR request to prevent cross-request counter leakage:

    ```rust
    #[cfg(all(feature = "ssr", target_arch = "wasm32"))]
    pub fn reset_id_counter() {
        DIOXUS_ID_COUNTER.with(|c| c.set(0));
    }
    #[cfg(all(feature = "ssr", not(target_arch = "wasm32")))]
    pub fn reset_id_counter() {
        DIOXUS_ID_COUNTER.store(0, core::sync::atomic::Ordering::Relaxed);
    }
    ```

   Resetting the counter prevents request-to-request leakage, but it does not
   make generated IDs safe if server and client render different hook paths.

3. **Hydration mismatch detection**: In debug builds, the client SHOULD compare
   the mounted DOM element's server-rendered `id` with the client ID it is about
   to use. On mismatch, emit a console warning:
   `"ars-ui hydration ID mismatch: server='ars-dialog-7', client='ars-dialog-9'. Component IDs may be non-deterministic across SSR/client boundaries."`

4. **Generated fallback implementation**: `use_stable_id()` must use Dioxus hook
   storage so the generated value is stable for the lifetime of a mounted
   component and does not change across re-renders:

```rust
fn use_stable_id(prefix: &str) -> String {
    // Hook storage keeps the generated suffix stable across re-renders, but
    // the suffix still comes from the adapter counter and is not hydration-safe.
    let id = use_hook(|| dioxus_id_counter());
    format!("ars-{prefix}-{id}")
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn warn_if_mounted_id_mismatch(element: &web_sys::Element, client_id: &str) {
    let server_id = element.id();
    if server_id.is_empty() || server_id == client_id {
        return;
    }

    #[cfg(debug_assertions)]
    web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
        "ars-ui hydration ID mismatch: server='{server_id}', client='{client_id}'. Component IDs may be non-deterministic across SSR/client boundaries."
    )));
}
```

> **Warning:** `use_stable_id` currently delegates to `dioxus_id_counter()`, which is NOT
> hydration-safe. SSR+hydration users MUST provide explicit `id` props on all components
> unless the component restores the exact server ID from a hydration snapshot.

---

## 20. SSR Hydration Support

### 20.1 FocusScope Hydration Handling

**Problem.** `FocusScope` effects are gated by `#[cfg(not(feature = "ssr"))]`, which means focus restoration logic is skipped entirely during server-side rendering. When the client hydrates, the post-hydration effect calls `document.querySelector()` for a focus target that may no longer exist in the DOM (e.g., a dynamically rendered element the server never produced). Additionally, modal `Dialog` components rendered as open during SSR leave orphaned `inert` attributes on sibling elements because the server never runs the cleanup effect that would remove them.

**Solution.** The following rules govern FocusScope behavior across the SSR-to-hydration boundary:

1. **Emit valid focus targets during SSR.** Server-rendered modal overlays MUST include at least one focusable element in the HTML output. Use a `button` with `autofocus` or an element with `tabindex="-1"` so that the hydration effect has a valid target.

2. **Scan for orphaned `inert` on hydration.** A post-hydration `use_effect` queries all elements with `[inert]` and removes the attribute from any element that is not currently a sibling of an open modal. This prevents "frozen" regions left over from SSR. Cleanup of the modal-open marker uses `use_drop` (Dioxus equivalent of Leptos `on_cleanup`).

3. **Validate focus target existence and visibility.** Before calling `.focus()`, the FocusScope checks that the target element exists and is visible by verifying `element.offset_parent().is_some()`. Elements that are `display: none` or detached return `None` and are skipped.

4. **Gate focus trap activation on hydration completion.** The root `ArsProvider` sets a `data-ars-hydrated` attribute on the document body once hydration is complete. Nested providers MUST NOT repeatedly mark the body; the marker represents the root hydration boundary. FocusScope defers trap activation until this attribute is present, preventing premature focus movement during partial hydration. If the marker is absent when the effect first runs, FocusScope schedules one `request_animation_frame` retry before giving up. In Dioxus, this check is wrapped in a `use_effect` (which only runs on the client), rather than a `#[cfg(not(feature = "ssr"))]` gated `Effect::new` as in the Leptos adapter.

5. **Use `request_animation_frame` for DOM settlement.** After hydration, focus trap activation is wrapped in a `request_animation_frame` callback to ensure the DOM has fully settled before the trap is engaged.

```rust
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

/// Hydration-safe FocusScope setup for modal overlays.
/// Called by FocusScope or Dialog component code after hydration is confirmed.
///
/// Dioxus adapter differences from Leptos:
/// - `use_effect` instead of `#[cfg(not(feature = "ssr"))]` gated `Effect::new`
/// - `use_drop` instead of `on_cleanup` for cleanup registration
/// - `Signal<Option<web_sys::HtmlElement>>` instead of `StoredValue`
/// - Focus operations go through platform abstraction where available
fn setup_focus_scope_hydration_safe(
    scope_id: String,
    mut restore_target: Signal<Option<web_sys::HtmlElement>>,
) {
    let cleanup_scope_id = scope_id.clone();

    // Step 1: Clean up orphaned inert attributes left by SSR.
    // use_effect only runs on the client, so no #[cfg(not(feature = "ssr"))] needed.
    use_effect(move || {
        let document = web_sys::window()
            .expect("window")
            .document()
            .expect("document");

        // Gate on hydration completion, with one frame of tolerance for the
        // root provider marker to settle.
        let body = document.body().expect("document.body");
        if body.get_attribute("data-ars-hydrated").is_none() {
            let scope_id = scope_id.clone();
            let document = document.clone();
            request_animation_frame(move || {
                let body = document.body().expect("document.body");
                if body.get_attribute("data-ars-hydrated").is_some() {
                    activate_focus_scope(&document, &scope_id, restore_target);
                }
            });
        } else {
            activate_focus_scope(&document, &scope_id, restore_target);
        }
    });

    // Step 3: Clean up on unmount using use_drop (Dioxus equivalent of on_cleanup).
    use_drop(move || {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(scope_el) = document.get_element_by_id(&cleanup_scope_id) {
                scope_el.remove_attribute("data-ars-modal-open").ok();
            }
        }

        // Restore focus to the previously focused element when scope deactivates.
        if let Some(el) = restore_target.peek().as_ref() {
            el.focus().ok();
        }
    });
}

fn activate_focus_scope(
    document: &web_sys::Document,
    scope_id: &str,
    restore_target: Signal<Option<web_sys::HtmlElement>>,
) {
    let scope_el = document
        .get_element_by_id(scope_id)
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok());

    if let Some(scope_el) = scope_el.as_ref() {
        scope_el.set_attribute("data-ars-modal-open", "").ok();
    }

    // Remove orphaned inert attributes. An inert element is retained while
    // it remains a sibling of the hydrated modal scope.
    let inert_elements = document
        .query_selector_all("[inert]")
        .expect("querySelectorAll");
    for i in 0..inert_elements.length() {
        if let Some(el) = inert_elements.item(i) {
            let html_el: web_sys::Element = el.unchecked_into();
            // Only remove if no open modal is a sibling.
            if !has_open_modal_sibling(&html_el) {
                html_el.remove_attribute("inert").ok();
            }
        }
    }

    // Step 2: Activate focus trap after DOM settles.
    if let Some(scope_el) = scope_el {
        // Use request_animation_frame to wait for DOM settlement.
        let document_clone = document.clone();
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
            // Validate target exists and is visible.
            let target = scope_el
                .query_selector("[autofocus], [tabindex]")
                .ok()
                .flatten()
                .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                .filter(|el| el.offset_parent().is_some());

            if let Some(el) = target {
                // Save the currently focused element BEFORE moving focus,
                // so it can be restored when the scope is deactivated.
                restore_target.set(
                    document_clone
                        .active_element()
                        .and_then(|ae| ae.dyn_into().ok()),
                );
                el.focus().ok();
            }
        });
        web_sys::window()
            .expect("window must exist")
            .request_animation_frame(cb.as_ref().unchecked_ref())
            .ok();
    }
}

fn has_open_modal_sibling(element: &web_sys::Element) -> bool {
    let Some(parent) = element.parent_element() else {
        return false;
    };

    let children = parent.children();
    for index in 0..children.length() {
        let Some(child) = children.item(index) else {
            continue;
        };

        if child.is_same_node(Some(element)) {
            continue;
        }

        if child.has_attribute("data-ars-modal-open") {
            return true;
        }
    }

    false
}
```

### 20.2 HydrationSnapshot

For stateful components that need to preserve state across SSR → client
hydration, use the canonical `HydrationSnapshot<M>` defined in `ars_core`
(gated behind the `ssr` + `serde` features). The Dioxus adapter re-uses
this shared type rather than redefining its own — the server and client
halves of a round-trip must agree on the wire format, so a single source
of truth lives in the foundation crate.

```rust
use ars_core::{HasId, HydrationSnapshot, Machine, Service};
use dioxus::prelude::*;

/// In SSR mode: embed a snapshot as a JSON script tag.
#[cfg(feature = "ssr")]
fn serialize_snapshot<M: Machine>(svc: &Service<M>) -> String
where
    M::State: Clone + serde::Serialize,
{
    serde_json::to_string(&HydrationSnapshot::<M> {
        state: svc.state().clone(),
        id: svc.props().id().to_string(),
    })
    .expect("HydrationSnapshot must be serializable for SSR — ensure State implements Serialize")
}

/// In hydration mode: read the snapshot and initialize the machine via
/// `Service::new_hydrated(props, snapshot.state, &env, &messages)`.
#[cfg(feature = "ssr")]
fn use_machine_hydrated<M: Machine + 'static>(
    props: M::Props,
    snapshot: HydrationSnapshot<M>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Event: Send + 'static,
    M::Messages: Send + Sync + 'static,
{
    let props = if props.id().is_empty() {
        props.with_id(snapshot.id.clone())
    } else {
        debug_assert_eq!(
            props.id(),
            snapshot.id,
            "HydrationSnapshot id must match Props::id"
        );
        props
    };

    let (result, ..) = use_machine_inner::<M>(props, Some(snapshot.state));
    result
}

#[cfg(feature = "ssr")]
fn use_machine_with_reactive_props_hydrated<M: Machine + 'static>(
    props_signal: Signal<M::Props>,
    snapshot: HydrationSnapshot<M>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
    M::Event: Send + 'static,
    M::Messages: Send + Sync + 'static,
{
    use_machine_hydrated::<M>(props_signal(), snapshot)
}
```

---

## 21. Error Boundary Pattern

Wrap component trees with `ErrorBoundary` to gracefully handle machine panics
or unexpected state transitions:

```rust
#[component]
pub fn ArsErrorBoundary(children: Element) -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: |ctx: ErrorContext| {
                rsx! {
                    div {
                        "data-ars-error": "true",
                        role: "alert",
                        p { "A component encountered an error." }
                        p { {ctx.error().map(|e| format!("{e}")).unwrap_or_default()} }
                    }
                }
            },
            {children}
        }
    }
}
```

---

## 22. Machine Type Parameter Bounds

All `Machine` type parameters in adapter hooks must satisfy `M: Machine + 'static`. This is required because framework reactive primitives (Dioxus `Signal`) require `'static` storage. Consequently, `Machine::Props` must be `'static` — use `Rc<T>` or `Arc<T>` for shared ownership instead of references.

---

## 23. Api Lifetime and Async Event Handlers

`Api` borrows from `Service` and cannot be held across `.await` points. For async event handlers, clone the send callback from the `UseMachineReturn`:

```rust
let send = machine.send;
// then use inside async block (Dioxus auto-spawns async in event handlers):
spawn(async move {
    let result = fetch_data().await;
    send.call(MyEvent::DataLoaded(result));
});
```

Do not hold `Api` references across await boundaries.
