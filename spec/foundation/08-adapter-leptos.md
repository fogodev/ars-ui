# Leptos Adapter Specification (`ars-leptos`)

## 1. Overview

The `ars-leptos` crate bridges `ars-core` state machines to Leptos's fine-grained reactivity system. Its responsibilities:

1. **Wrap `Service<M>` in Leptos signals** so state changes trigger reactive updates
2. **Convert `AttrMap` to Leptos view attributes** for spreading onto DOM elements
3. **Provide compound component patterns** (context-based root/part composition)
4. **Support SSR and hydration** with deterministic ID generation
5. **Handle controlled values** by watching external signals and syncing to machines

> **Platform scope:** Leptos targets web only via `web_sys`. Platform operations (focus, clipboard, bounding rects) call `web_sys` directly. For multi-platform equivalents, see the Dioxus adapter's `DioxusPlatform` trait (`09-adapter-dioxus.md` §6). DOM utilities (positioning, scroll lock, focus management, z-index) are specified in `11-dom-utilities.md`.

### 1.1 Key Leptos Properties Exploited

- Components run **once** (not on every render) — ideal for creating `Service<M>` once
- Fine-grained reactivity — only the specific DOM attributes that change trigger updates
- `provide_context` / `use_context` for compound component communication
- Owner-based RAII cleanup (signals, effects, and subscriptions are dropped when their Owner is dropped; `on_cleanup` registers additional callbacks on the current Owner)
- `#[slot]` macro for named slot composition

### 1.2 Dependency

```toml
# ars-leptos/Cargo.toml
[dependencies]
ars-core = { workspace = true }
ars-a11y = { workspace = true }
ars-i18n = { workspace = true }
ars-interactions = { workspace = true }
ars-collections = { workspace = true }
ars-forms = { workspace = true }
ars-dom = { workspace = true }
leptos = "0.8"

[features]
default = []
ssr = ["leptos/ssr", "ars-dom/ssr"]
hydrate = ["leptos/hydrate"]
csr = ["leptos/csr"]
```

---

## 2. Platform Support Matrix

| Platform                    | Status    | Notes                                                                                                                                                                                                                   |
| --------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Web (WASM)                  | Supported | Primary target, full feature set                                                                                                                                                                                        |
| SSR (Server-Side Rendering) | Supported | Hydration-compatible, no DOM access during SSR                                                                                                                                                                          |
| Mobile Web                  | Partial   | Works via mobile browser; touch events supported, no native APIs. Hover interactions degrade to focus/press states. Use `inputmode` attribute to control virtual keyboard type. Minimum 44px touch targets recommended. |
| Desktop (Tauri)             | Supported | Via WebView; all web APIs available                                                                                                                                                                                     |
| Native                      | N/A       | Leptos targets web; use Dioxus adapter for native                                                                                                                                                                       |

> **Platform scope:** The Leptos adapter targets web only (via `web_sys`). For Tauri-based Leptos apps, components that access DOM APIs should be gated behind `#[cfg(target_arch = "wasm32")]`.

---

## 3. The `use_machine` Hook

The central primitive. Creates a `Service<M>`, wraps it in a reactive signal, and returns a stable `send` callback.

````rust
use std::rc::Rc;
use leptos::prelude::*;
use ars_core::{Machine, Service, Env, ArsRc};
use ars_i18n::IcuProvider;

/// Return type from `use_machine`.
#[derive(Clone, Copy)]
pub struct UseMachineReturn<M: Machine + 'static>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
{
    /// Reactive signal for the current machine state.
    /// Reading it inside a reactive scope creates a dependency.
    pub state: ReadSignal<M::State>,

    /// Send an event to the machine.
    /// Safe to call from any closure — does not require reactive scope.
    pub send: Callback<M::Event>,

    /// Access the full service (context + state) via a StoredValue.
    /// Use sparingly — prefer `derive()` for reactive data and `with_api_ephemeral()` for imperative access.
    pub service: StoredValue<Service<M>>,

    /// Monotonically increasing counter that increments whenever context changes.
    /// Used by `derive()` to track context mutations even when state remains the same.
    /// Leptos's signal equality check suppresses `state_write.set(same_value)`, so
    /// this counter ensures memos re-run when only context has changed.
    pub context_version: ReadSignal<u64>,
}

impl<M: Machine + 'static> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    /// Get a one-shot snapshot of the connect API.
    /// **Prefer `derive()` for reactive data** — this method does not track dependencies.
    /// Use `with_api_snapshot` only for imperative operations (e.g., reading a value once).
    /// The connect closure uses a no-op/panic callback because `with_value` holds
    /// an immutable borrow on the `StoredValue`, preventing `send` (which calls
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

    /// Create a fine-grained memo that derives a value from the connect API.
    /// Only re-computes when the underlying state changes, and only notifies
    /// dependents when the derived value actually changes.
    ///
    /// **⚠ Important: `Api<'a>` has a non-`'static` lifetime and CANNOT be stored
    /// in Leptos signals, global state, or any `'static` context.** The `&M::Api<'_>`
    /// reference passed to the `derive()` closure is valid only for the duration of
    /// that closure call. Extract the values you need (strings, booleans, `AttrMap`)
    /// and return them — do not attempt to return or store the `Api` itself.
    ///
    /// ```rust
    /// // CORRECT — extract values inside derive:
    /// let is_open = machine.derive(|api| api.is_open());
    ///
    /// // WRONG — cannot store Api in a signal:
    /// // let api_signal = RwSignal::new(machine.with_api_snapshot(|api| api));  // ❌ won't compile
    /// ```
    ///
    /// **Safety**: The closure passed to `derive()` must not call `send()` — it is
    /// a read-only projection of the current state and context.
    ///
    /// # Example
    /// ```rust
    /// let is_open = machine.derive(|api| api.is_open());
    /// let aria_label = machine.derive(|api| api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)).map(str::to_owned));
    /// ```
    pub fn derive<T: Clone + PartialEq + 'static>(
        &self,
        f: impl Fn(&M::Api<'_>) -> T + 'static,
    ) -> Memo<T> {
        let state = self.state;
        let context_version = self.context_version;
        // INVARIANT: `context_version` is a Leptos signal that is tracked (`.track()`)
        // by every component that reads context. It is bumped (`.set()`) on every
        // context mutation (line ~185). This is the ONLY mechanism that causes
        // Leptos to re-render when headless context changes — derive macros alone
        // are insufficient because Leptos requires signal-based reactivity.
        let service = self.service;
        Memo::new(move |_| {
            state.track();
            context_version.track();
            service.with_value(|svc| {
                let api = svc.connect(&|_e| {
                    #[cfg(debug_assertions)]
                    panic!("Cannot send events inside derive() — use event handlers from with_api() instead");
                });
                f(&api)
            })
        })
    }
}
````

### 3.1 EphemeralRef Newtype

To prevent `Api<'a>` from being accidentally stored in signals (which would outlive the borrow), the Leptos adapter MUST wrap derived API access in an `EphemeralRef<'a, T>` newtype that is `!Clone`, `!Copy`, and cannot be coerced to `'static`:

```rust
/// A non-cloneable, non-copyable wrapper that prevents storing borrowed data in signals.
///
/// The `PhantomData<Rc<()>>` marker is chosen because `Rc<()>` is both `!Send` and `!Sync`,
/// which prevents this type from crossing thread boundaries. Unlike `*mut &'a ()`,
/// `Rc<()>` provides both `!Send` and `!Sync` guarantees without relying on raw pointer
/// semantics. The `&'a ()` lifetime parameter still ensures the type is NOT `'static`.
///
/// - NOT `Send` or `Sync` (Rc is neither Send nor Sync)
/// - NOT `'static` (cannot be stored in signals which require `'static`)
/// - NOT `Clone` or `Copy` (cannot be duplicated to circumvent lifetime)
pub struct EphemeralRef<'a, T> {
    value: T,
    _marker: PhantomData<(Rc<()>, &'a ())>,
}

impl<'a, T> EphemeralRef<'a, T> {
    /// Create a new ephemeral reference. Only callable within derive() closures.
    pub fn new(value: T) -> Self {
        Self { value, _marker: PhantomData }
    }

    /// Access the inner value by reference.
    pub fn get(&self) -> &T {
        &self.value
    }
}

// Explicitly NOT implementing Clone, Copy, Send, Sync
// EphemeralRef cannot be stored in Signal<T> because Signal requires T: 'static
```

**Usage in `with_api_ephemeral()`**: The adapter provides `with_api_ephemeral()` for imperative, non-reactive API access (e.g., inside event handlers) using `EphemeralRef`:

```rust
pub fn with_api_ephemeral<R>(&self, f: impl Fn(EphemeralRef<'_, M::Api<'_>>) -> R) -> R {
    let send = self.send;
    self.service.with_value(|svc| {
        let api = svc.connect(&|e| send.run(e));
        f(EphemeralRef::new(api))
    })
}
```

**Compile-time safety**: Attempting to store the result in a signal produces a compile error:

```rust
// COMPILE ERROR: EphemeralRef<'_, Api<'_>> does not satisfy `'static`
let signal = RwSignal::new(ephemeral_ref); // ❌ Won't compile
```

This eliminates the use-after-free class of bugs where `Api<'a>` outlives the `Service` reference.

> **Note:** `EphemeralRef` / `with_api_ephemeral()` is for imperative, non-reactive API access (e.g., inside event handlers). For reactive derived values, use `machine.derive(|api| ...)` which returns a `Memo<T>`. These are complementary patterns: derive() for render-time reads, EphemeralRef for event-time reads.

### 3.2 Hydration-Safe Component IDs

The Leptos adapter MUST use `use_id()` (a global monotonic counter) for all component IDs to ensure SSR/hydration consistency. The counter produces the same sequence on server and client when the component tree renders in the same order. The ID resolution order is:

1. **`props.id` takes priority** — If the consumer provides an explicit `id` prop, use it as-is. The consumer is responsible for ensuring uniqueness and hydration stability.
2. **`use_id()` fallback** — If `props.id` is empty, the adapter calls `use_id("component")` to generate a hydration-safe ID. The global `ID_COUNTER` produces deterministic IDs as long as SSR and client render components in the same tree order. `reset_id_counter()` must be called at the start of each SSR request.

```rust
// In use_machine_inner():
let props = {
    let mut p = props;
    if p.id().is_empty() {
        // use_id() returns a deterministic ID that matches between SSR and hydration.
        // This is critical: non-deterministic IDs (e.g., random UUIDs) cause hydration
        // mismatches, breaking ARIA linkage (label-for, describedby, etc.).
        p.set_id(use_id("component"));
    }
    p
};
```

**Invariant:** Once assigned at Service creation, the component ID MUST NOT change for the lifetime of the component instance. Re-renders do not regenerate IDs.

```rust
/// Internal — creates a single Service and returns the public return type plus
/// internal handles needed by `use_machine_with_reactive_props` to process
/// SendResult from set_props (state_write, send_ref, effect_cleanups,
/// context_version_write).
fn use_machine_inner<M: Machine + 'static>(
    props: M::Props,
) -> (
    UseMachineReturn<M>,
    WriteSignal<u64>,
    WriteSignal<M::State>,
    StoredValue<Option<Callback<M::Event>>>,
    StoredValue<std::collections::HashMap<&'static str, Box<dyn FnOnce()>>>,
)
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    // Auto-inject ID if not provided.
    // Convention: M::Props must have a public `id: String` field.
    let props = {
        let mut p = props;
        if p.id().is_empty() {
            p.set_id(use_id("component"));
        }
        p
    };

    // Resolve environment values from ArsProvider context.
    // These are snapshot reads — Leptos components run once, so each component
    // gets the locale/icu_provider/messages at mount time.
    let locale = resolve_locale(None);
    let icu_provider = use_icu_provider();
    let registries = use_context::<ArsContext>()
        .map(|ctx| ctx.i18n_registries.clone())
        .unwrap_or_default();
    let messages = resolve_messages::<M::Messages>(None, &registries, &locale);
    let env = Env { locale, icu_provider };

    // Create the service once — runs only on component initialization.
    // **Safety**: The `init()` function must not call `api.send()` or otherwise
    // produce events. It runs during component initialization and event
    // processing is not yet set up.
    let service = StoredValue::new(Service::<M>::new(props, env, messages));

    // Create a signal tracking the current state
    let initial_state = service.with_value(|s| s.state().clone());
    let (state_read, state_write) = signal(initial_state);

    // Context version counter — incremented on every context change so that
    // derive() memos re-run even when state itself hasn't changed.
    let (context_version_read, context_version_write) = signal(0u64);

    // Track effect cleanups
    // Effect cleanups keyed by effect name. On new effects: only replace effects
    // with matching names, leaving other effects running. On state change: drain ALL.
    use std::collections::HashMap;
    let effect_cleanups: StoredValue<HashMap<&'static str, Box<dyn FnOnce()>>> = StoredValue::new(HashMap::new());

    // INVARIANT: Effect cleanup functions MUST NOT call `send()`. Doing so would
    // re-enter the send callback while the StoredValue is borrowed, causing a panic.
    // If cleanup needs to notify the machine, defer via `queue_microtask`.
    //
    // **Memory leak prevention:** Context/props passed to effect setup closures
    // should extract only the needed fields (e.g., IDs, flags) rather than
    // cloning the entire context or props struct. This prevents retaining large
    // data structures in cleanup closures.
    //
    // **Cleanup ordering:** Cleanup functions run in LIFO order (last effect set
    // up is first to be cleaned up). Since effects are keyed by name in a HashMap,
    // per-name cleanup ordering is naturally LIFO via `pop()` on the
    // values. For the full drain on state change, iteration order of HashMap is
    // irrelevant because each name has exactly one cleanup — the ordering constraint
    // applies only within a single name's lifecycle. If a component re-renders
    // during cleanup (e.g., a signal write triggers a reactive update), defer the
    // new effect setup to the next microtask to avoid interleaving setup and cleanup.

    // Two-phase send callback construction:
    // The send callback needs to pass itself (as an Rc) into PendingEffect::setup,
    // but a closure cannot reference itself during construction. We solve this by:
    // 1. Creating a StoredValue slot for the callback
    // 2. Building the callback that reads from the slot
    // 3. Storing the callback into the slot
    let send_ref: StoredValue<Option<Callback<M::Event>>> = StoredValue::new(None);

    let send: Callback<M::Event> = Callback::new(move |event: M::Event| {
        // Phase 1: Process event — borrow service, send event, clone what we need, release borrow.
        // Note: Leptos 0.8 StoredValue::update_value returns (), so we extract
        // data through a captured Option (side-channel pattern).
        let mut extracted = None;
        service.update_value(|s| {
            let result = s.send(event);

            if result.state_changed {
                state_write.set(s.state().clone());
            }
            if result.context_changed {
                context_version_write.update(|v| *v += 1);
            }

            // Clone ctx and props before releasing borrow (effects need them)
            let ctx_clone = s.context().clone();
            let props_clone = s.props().clone();
            extracted = Some((result, ctx_clone, props_clone));
        });
        let (result, ctx_clone, props_clone) = extracted.expect("update_value closure ran");

        // Phase 2: Set up pending effects OUTSIDE the borrow
        #[cfg(not(feature = "ssr"))]
        {
            if result.state_changed {
                // Full state change: drain ALL effects in LIFO order
                effect_cleanups.update_value(|cleanups| {
                    // HashMap drain order is arbitrary, but each name has exactly
                    // one cleanup so per-name LIFO is trivially satisfied.
                    // For Vec-backed stores, use pop() for natural LIFO.
                    for (_, cleanup) in cleanups.drain() {
                        cleanup();
                    }
                });
            } else if !result.pending_effects.is_empty() {
                // context_only transition: only replace effects with matching names
                effect_cleanups.update_value(|cleanups| {
                    for effect in &result.pending_effects {
                        if let Some(old_cleanup) = cleanups.remove(effect.name) {
                            old_cleanup();
                        }
                    }
                });
            }

            // During the initialization window (between Callback::new and send_ref.set_value),
            // send() is silently dropped. This is safe because no user interaction can occur
            // before the component is mounted, and any framework-triggered events during init
            // are spurious. In debug mode, a debug_assert fires to catch unexpected early sends.
            let send_cb = send_ref.with_value(|opt| opt.clone());
            let send_rc: Rc<dyn Fn(M::Event)> = if let Some(cb) = send_cb {
                Rc::new(move |e| cb.run(e))
            } else {
                debug_assert!(false, "send() called before send_ref initialized — event dropped");
                return; // Silently drop during initialization window
            };

            for effect in result.pending_effects {
                let name = effect.name;
                let cleanup = effect.run(
                    &ctx_clone,
                    &props_clone,
                    send_rc.clone(),
                );
                effect_cleanups.update_value(|cleanups| { cleanups.insert(name, cleanup); });
            }
        }
    });

    // Complete the two-phase pattern: store the send callback so effects can use it
    send_ref.set_value(Some(send));

    // Clean up effects when the component unmounts (LIFO order)
    // Note: on_cleanup registers a callback on the current reactive Owner.
    // In Leptos 0.8, this is the idiomatic way to register cleanup logic
    // that runs when the component's Owner is dropped.
    on_cleanup(move || {
        effect_cleanups.update_value(|cleanups| {
            for (_, cleanup) in cleanups.drain() {
                cleanup();
            }
        });
    });

    let result = UseMachineReturn { state: state_read, send, service, context_version: context_version_read };
    (result, context_version_write, state_write, send_ref, effect_cleanups)
}

pub fn use_machine<M: Machine + 'static>(props: M::Props) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    let (result, _, _, _, _) = use_machine_inner::<M>(props);
    result
}
```

### 3.3 Reactive Props Variant

For components with externally controlled props (e.g., `checked` signal):

```rust
pub fn use_machine_with_reactive_props<M: Machine + 'static>(
    props_signal: Signal<M::Props>,
) -> UseMachineReturn<M>
where
    M::State: Clone + PartialEq + 'static,
    M::Context: Clone + 'static,
    M::Props: Clone + PartialEq + 'static,
{
    let initial_props = props_signal.get();

    // Use the internal variant that also returns internal handles (context_version_write,
    // state_write, send_ref, effect_cleanups), so we can process the SendResult from
    // set_props (mirroring the Dioxus use_machine_inner pattern).
    let (result, context_version_write, state_write, send_ref, effect_cleanups) =
        use_machine_inner::<M>(initial_props);

    let service = result.service;

    // Track previous props — skip initial sync (machine already has correct props from init)
    // StoredValue uses interior mutability — `set_value()` does not require `mut`.
    let prev_props: StoredValue<Option<M::Props>> = StoredValue::new(None);

    // Watch for prop changes and sync to machine.
    // Note: This effect writes to `context_version_write` — an intentional exception
    // to the "never write signals inside effects" rule. This is safe because
    // `props_signal` (the dependency) is an external input, not derived from
    // `context_version`, so no reactive loop can form.
    Effect::new(move |_| {
        let new_props = props_signal.get();
        let should_sync = prev_props.with_value(|prev| {
            prev.as_ref() != Some(&new_props)
        });
        if should_sync {
            let is_initial = prev_props.with_value(|prev| prev.is_none());
            if !is_initial {
                // Only sync after first render — init already has correct props.
                // Use side-channel extraction (same pattern as send callback above)
                // to avoid discarding the SendResult from set_props.
                let mut result = None;
                service.update_value(|s| {
                    result = Some(s.set_props(new_props.clone()));
                });
                if let Some(send_result) = result {
                    if send_result.state_changed {
                        state_write.set(service.with_value(|s| s.state().clone()));
                    }
                    if send_result.context_changed {
                        context_version_write.update(|v| *v += 1);
                    }
                    for effect in send_result.pending_effects {
                        let name = effect.name;
                        let send_cb = send_ref.with_value(|opt| opt.clone());
                        if let Some(cb) = send_cb {
                            let send_rc: Rc<dyn Fn(M::Event)> = Rc::new(move |e| cb.run(e));
                            let ctx_clone = service.with_value(|s| s.context().clone());
                            let props_clone = service.with_value(|s| s.props().clone());
                            let cleanup = effect.run(&ctx_clone, &props_clone, send_rc);
                            effect_cleanups.update_value(|cleanups| { cleanups.insert(name, cleanup); });
                        }
                    }
                }
            }
            prev_props.set_value(Some(new_props));
        }
    });

    result
}
```

#### 3.3.1 Usage Example

```rust
#[component]
fn ControlledCheckbox(checked: Signal<checkbox::State>) -> impl IntoView {
    // Prefer `Memo::new` over `Signal::derive` for Props construction:
    // `Signal::derive` re-runs the closure on every read even if inputs haven't changed,
    // while `Memo::new` caches and only recomputes when tracked signals change.
    let props = Memo::new(move |_| checkbox::Props {
        id: String::new(),
        checked: Some(checked.get()),
        ..Default::default()
    });
    // Note: Leptos 0.8's From<Memo<T>> for Signal<T> preserves the Memo's caching
    // semantics. The resulting Signal reads from the Memo's cache, not a re-derived closure.
    let machine = use_machine_with_reactive_props::<checkbox::Machine>(props.into());
    let root_attrs = machine.derive(|api| api.root_attrs());
    // ...
}
```

> **Adapter difference:** Leptos provides `use_machine_with_reactive_props` as a separate hook because Leptos effects are fine-grained and can watch individual signals. Dioxus integrates prop sync into `use_machine` via `use_sync_props` because Dioxus uses component-level re-rendering.

### 3.4 SSR Effect Behavior

During server-side rendering, effects are not executed. The `use_machine` hook gates effect setup with `#[cfg(not(feature = "ssr"))]`, ensuring:

- Timer effects (debounce, delay) are not started on the server
- DOM effects (focus, scroll lock, event listeners) are not attempted
- All ARIA attributes and roles are still computed by `connect()` and included in SSR HTML
- The adapter skips calling `PendingEffect::setup` entirely during SSR

#### 3.4.1 FocusScope Hydration Handling

**Problem.** `FocusScope` effects are gated by `#[cfg(not(feature = "ssr"))]`, which means focus restoration logic is skipped entirely during server-side rendering. When the client hydrates, the post-hydration effect calls `document.querySelector()` for a focus target that may no longer exist in the DOM (e.g., a dynamically rendered element the server never produced). Additionally, modal `Dialog` components rendered as open during SSR leave orphaned `inert` attributes on sibling elements because the server never runs the cleanup effect that would remove them.

**Solution.** The following rules govern FocusScope behavior across the SSR-to-hydration boundary:

1. **Emit valid focus targets during SSR.** Server-rendered modal overlays MUST include at least one focusable element in the HTML output. Use `<button autofocus>` or an element with `tabindex="-1"` so that the hydration effect has a valid target.

2. **Scan for orphaned `inert` on hydration.** A post-hydration effect queries all elements with `[inert]` and removes the attribute from any element that is not currently a sibling of an open modal. This prevents "frozen" regions left over from SSR.

3. **Validate focus target existence and visibility.** Before calling `.focus()`, the FocusScope checks that the target element exists and is visible by verifying `element.offset_parent().is_some()`. Elements that are `display: none` or detached return `None` and are skipped.

4. **Gate focus trap activation on hydration completion.** The adapter sets a `data-ars-hydrated` attribute on the document body once hydration is complete. FocusScope defers trap activation until this attribute is present, preventing premature focus movement during partial hydration.

5. **Use `request_animation_frame` for DOM settlement.** After hydration, focus trap activation is wrapped in a `request_animation_frame` callback to ensure the DOM has fully settled before the trap is engaged.

```rust
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

/// Hydration-safe FocusScope setup for modal overlays.
/// Called from `use_machine` effect setup after hydration is confirmed.
fn setup_focus_scope_hydration_safe(
    scope_ref: NodeRef<html::Div>,
    restore_target: StoredValue<Option<web_sys::HtmlElement>>,
) {
    // Step 1: Clean up orphaned inert attributes left by SSR.
    #[cfg(not(feature = "ssr"))]
    Effect::new(move |_| {
        let document = document();

        // Gate on hydration completion.
        let body = document.body().expect("document.body");
        if body.get_attribute("data-ars-hydrated").is_none() {
            return;
        }

        // Remove orphaned inert attributes.
        let inert_elements = document
            .query_selector_all("[inert]")
            .expect("querySelectorAll");
        for i in 0..inert_elements.length() {
            if let Some(el) = inert_elements.item(i) {
                let html_el: web_sys::HtmlElement = el.unchecked_into();
                // Only remove if no open modal is a sibling.
                if document
                    .query_selector("[data-ars-modal-open]")
                    .ok()
                    .flatten()
                    .is_none()
                {
                    html_el.remove_attribute("inert").ok();
                }
            }
        }

        // Step 2: Activate focus trap after DOM settles.
        if let Some(scope_el) = scope_ref.get() {
            let scope_el: web_sys::HtmlElement = scope_el.into();
            request_animation_frame(move || {
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
                    restore_target.set_value(
                        document
                            .active_element()
                            .and_then(|ae| ae.dyn_into().ok()),
                    );
                    el.focus().ok();
                }
            });
        }
    });
}
```

---

## 4. AttrMap → Leptos Attributes

`AttrMap` is the framework-agnostic attribute map returned by connect functions. Adapting it to Leptos:

### 4.1 AttrMap Conversion

```rust
use ars_core::{AttrMap, HtmlAttr, AttrValue, CssProperty, StyleStrategy};

/// Result of converting an `AttrMap` with strategy awareness.
pub struct LeptosAttrResult {
    /// HTML attribute tuples ready for spreading via `{..attrs}`.
    pub attrs: Vec<(String, String)>,
    /// Styles to apply via CSSOM (`element.style().set_property()`).
    /// Non-empty only when strategy is `Cssom`.
    pub cssom_styles: Vec<(CssProperty, String)>,
    /// CSS rule text to inject into a `<style nonce="...">` block.
    /// Non-empty only when strategy is `Nonce`.
    pub nonce_css: String,
}

/// Convert an `AttrMap` into Leptos attributes using the given `StyleStrategy`.
///
/// - `map.styles` are rendered according to the active strategy.
/// - `element_id` is required for `Nonce` strategy (used as CSS selector).
/// - `class` and other space-separated attributes are already merged in the `AttrMap`
///   by `set()` and flow through the main attrs loop naturally.
pub fn attr_map_to_leptos(
    map: AttrMap,
    strategy: &StyleStrategy,
    element_id: Option<&str>,
) -> LeptosAttrResult {
    let AttrMapParts { attrs, styles } = map.into_parts();

    let mut result: Vec<(String, String)> = attrs.into_iter()
        .filter_map(|(key, val)| match val {
            AttrValue::String(s) => Some((key.to_string(), s)),
            AttrValue::Bool(true) => Some((key.to_string(), String::new())),
            AttrValue::Bool(false) | AttrValue::None => None,
        })
        .collect();

    let mut cssom_styles = Vec::new();
    let mut nonce_css = String::new();

    match strategy {
        StyleStrategy::Inline => {
            if !styles.is_empty() {
                let style_str: String = styles.into_iter()
                    .map(|(prop, val)| format!("{}: {};", prop, val))
                    .collect::<Vec<_>>()
                    .join(" ");
                result.push(("style".to_string(), style_str));
            }
        }
        StyleStrategy::Cssom => {
            cssom_styles = styles;
        }
        StyleStrategy::Nonce(_) => {
            if !styles.is_empty() {
                let id = element_id.expect("element_id is required for Nonce style strategy");
                result.push(("data-ars-style-id".to_string(), id.to_string()));
                nonce_css = styles_to_nonce_css(id, &styles);
            }
        }
    }

    LeptosAttrResult { attrs: result, cssom_styles, nonce_css }
}

/// Apply styles to a DOM element via the CSSOM API.
/// Used when `StyleStrategy::Cssom` is active.
#[cfg(not(feature = "ssr"))]
pub fn apply_styles_cssom(el: &web_sys::HtmlElement, styles: &[(CssProperty, String)]) {
    let style = el.style();
    for (prop, val) in styles {
        let _ = style.set_property(&prop.to_string(), val);
    }
}

/// Convert styles to a CSS rule string for nonce-based injection.
fn styles_to_nonce_css(id: &str, styles: &[(CssProperty, String)]) -> String {
    let decls: Vec<String> = styles.iter()
        .map(|(prop, val)| format!("  {}: {};", prop, val))
        .collect();
    format!("[data-ars-style-id=\"{}\"] {{\n{}\n}}", id, decls.join("\n"))
}
```

> **Migration note:** The previous `attr_map_to_leptos(map: AttrMap) -> Vec<(String, String)>` signature is replaced by the strategy-aware version above. Callers must pass a `StyleStrategy` reference (obtained via `use_style_strategy()`) and an optional element ID.

### 4.2 Typed Handler Wiring

Adapters wire typed handlers from the Api to Leptos event listeners. Use `derive()` to
reactively obtain attributes and `send.run(checkbox::Event::Toggle)` (or the appropriate event variant) for event handling:

```rust
// In a Leptos component:
// Two attr spread patterns are used:
//   1. `{..machine.derive(|api| api.xxx_attrs()).get()}` — direct spread of Vec<(String, String)>
//      Used when the component does not need StyleStrategy-aware CSS handling.
//   2. `{..attr_map_to_leptos(api.xxx_attrs(), &strategy, id).attrs}` — strategy-aware spread
//      Used when the component has CSS properties requiring CSSOM or nonce injection.
let strategy = use_style_strategy();
let root_attrs = machine.derive(move |api| attr_map_to_leptos(api.root_attrs(), &strategy, Some("checkbox-root")));
let data_state = machine.derive(|api| api.data_state().to_string());

view! {
    <div
        {..root_attrs.get().attrs}
        data-ars-state=data_state
        on:click=move |_| send.run(Event::Toggle)
        on:keydown=move |ev| {
            match KeyboardKey::from_key_str(&ev.key()) {
                KeyboardKey::Enter | KeyboardKey::Space => send.run(Event::Toggle),
                _ => {}
            }
        }
    >
        {children()}
    </div>
}
```

### 4.3 Event Listener Options

```rust
/// Event listener options for passive and capture modes.
pub struct EventOptions {
    /// If true, the event listener will not call preventDefault().
    /// Required for passive scroll/touch listeners.
    pub passive: bool,
    /// If true, the event fires during the capture phase.
    pub capture: bool,
}

// In attr_map_to_leptos conversion:
// When EventOptions are set on a handler in AttrMap, emit:
// - `on:touchstart.passive` for passive listeners
// - `on:focus.capture` for capture phase listeners
// Leptos supports event modifiers via the `.passive` and `.capture` suffixes.
```

### 4.4 Recommended Pattern: Direct Props Building

Components build Leptos props directly via the typed connect API:

```rust
// The connect API exposes typed methods for Leptos:
impl CheckboxLeptosApi {
    /// Returns all attributes for the control element.
    pub fn control_attrs(&self) -> Vec<(&'static str, String)> {
        let [(scope_attr, scope_val), (part_attr, part_val)] = checkbox::Part::Control.data_attrs();
        let mut attrs = Vec::new();
        attrs.push(("role", "checkbox".to_string()));
        attrs.push(("tabindex", "0".to_string()));
        attrs.push((scope_attr, scope_val.to_string()));
        attrs.push((part_attr, part_val.to_string()));
        attrs.push(("aria-checked", self.aria_checked_value().to_string()));
        if self.is_disabled() {
            attrs.push(("aria-disabled", "true".to_string()));
            attrs.push(("data-ars-disabled", "".to_string()));
        }
        if self.is_focus_visible() {
            attrs.push(("data-ars-focus-visible", "".to_string()));
        }
        attrs
    }
}
```

### 4.5 CSP Style Strategy

The adapter provides a context-based `StyleStrategy` configuration. Components read the strategy from context and pass it to `attr_map_to_leptos()`.

```rust
use leptos::prelude::*;
use ars_core::StyleStrategy;

/// Read the current style strategy from context.
/// Returns `StyleStrategy::Inline` if no `ArsProvider` is present.
pub fn use_style_strategy() -> StyleStrategy {
    use_context::<ArsContext>()
        .map(|ctx| ctx.style_strategy().clone())
        .unwrap_or_else(|| {
            warn_missing_provider("use_style_strategy");
            StyleStrategy::default()
        })
}
```

#### 4.5.1 Nonce CSS Collector

For `StyleStrategy::Nonce`, a collector component aggregates CSS rules from all ars components and renders them in a single `<style>` element with the provided nonce.

````rust
/// Context for collecting nonce CSS rules during rendering.
#[derive(Clone, Debug)]
pub struct ArsNonceCssCtx {
    pub rules: RwSignal<Vec<String>>,
}

/// Collects nonce CSS from descendant components and renders a `<style nonce="...">` block.
///
/// Place this component near the document `<head>`:
/// ```rust
/// view! {
///     <ArsProvider strategy=StyleStrategy::Nonce(nonce.clone())>
///         <ArsNonceStyle nonce=nonce.clone() />
///         <App />
///     </ArsProvider>
/// }
/// ```
#[component]
pub fn ArsNonceStyle(nonce: String) -> impl IntoView {
    let rules = RwSignal::new(Vec::<String>::new());
    provide_context(ArsNonceCssCtx { rules });

    view! {
        <style nonce=nonce>
            {move || rules.with(|r| r.join("\n"))}
        </style>
    }
}

/// Append a CSS rule to the nonce collector.
/// Called internally by components when `StyleStrategy::Nonce` is active.
pub fn append_nonce_css(css: String) {
    if let Some(ctx) = use_context::<ArsNonceCssCtx>() {
        ctx.rules.update(|r| r.push(css));
    }
}
````

---

## 5. Standard Component Pattern

All ars-leptos components follow this structure:

### 5.1 Root Component

```rust
use leptos::prelude::*;
use ars_core::Machine;

/// Standard Leptos component for a state machine root.
///
/// 1. Accepts user-facing props (controlled values as Signal)
/// 2. Builds machine Props from component props
/// 3. Calls use_machine to create the service
/// 4. Provides context to child components
/// 5. Renders children
#[component]
pub fn Checkbox(
    /// Controlled checked state. If provided, component is controlled.
    #[prop(optional, into)] checked: Option<Signal<checkbox::State>>,
    /// Default checked state for uncontrolled mode.
    #[prop(optional)] default_checked: Option<checkbox::State>,
    /// Whether the checkbox is disabled.
    #[prop(optional, into)] disabled: Signal<bool>,
    /// Whether the checkbox is required in a form.
    #[prop(optional, into)] required: Signal<bool>,
    /// Name for native form submission.
    #[prop(optional, into)] name: Option<Signal<String>>,
    /// Value for native form submission.
    #[prop(optional, into)] value: Option<Signal<String>>,
    /// Callback when checked state changes.
    #[prop(optional)] on_checked_change: Option<Callback<checkbox::State>>,
    children: Children,
) -> impl IntoView {
    let props = checkbox::Props {
        checked: checked.as_ref().map(|s| s.get()),
        default_checked: default_checked.unwrap_or(checkbox::State::Unchecked),
        disabled: disabled.get(),
        required: required.get(),
        name: name.as_ref().map(|s| s.get()),
        value: value.as_ref().map(|s| s.get()).unwrap_or_else(|| "on".to_string()),
    };

    let machine = use_machine::<checkbox::Machine>(props);
    let UseMachineReturn { state, send, service, context_version } = machine;

    // Controlled value watchers — uses DRY use_controlled_prop helper (see §16)
    if let Some(checked_sig) = checked {
        use_controlled_prop(checked_sig, send, checkbox::Event::SetChecked);
    }
    use_controlled_prop(disabled, send, checkbox::Event::SetDisabled);

    // Fire on_checked_change callback when checked state changes
    if let Some(on_change) = on_checked_change {
        let prev_state: StoredValue<Option<checkbox::State>> = StoredValue::new(None);
        Effect::new(move |_| {
            let current = state.get();
            let prev = prev_state.with_value(|p| p.clone());
            if prev.as_ref() != Some(&current) {
                if prev.is_some() {
                    on_change.run(current.clone());
                }
                prev_state.set_value(Some(current));
            }
        });
    }

    // Provide context to all child parts
    provide_context(CheckboxContext { state, send, service, context_version });

    {children()}
}

/// Context shared between Checkbox compound component parts.
#[derive(Clone, Copy)]
pub struct CheckboxContext {
    pub state: ReadSignal<checkbox::State>,
    pub send: Callback<checkbox::Event>,
    pub service: StoredValue<Service<checkbox::Machine>>,
    pub context_version: ReadSignal<u64>,
}
```

### 5.2 Child Part Components

```rust
mod checkbox {
    /// The visual checkbox control — the clickable, focusable element.
    ///
    /// Uses `derive()` to reactively obtain attributes from the connect API,
    /// ensuring all ARIA attributes and data-state values stay in sync with
    /// both state and context changes.
    #[component]
    pub fn Control(
        #[prop(optional)] class: Option<Signal<String>>,
        children: Children,
    ) -> impl IntoView {
        // Convention: use_context().expect() over expect_context() for custom panic messages.
        // Leptos provides expect_context::<T>() as a shorthand, but we prefer the explicit
        // form for more descriptive error messages identifying the missing parent component.
        let ctx = use_context::<CheckboxContext>()
            .expect("checkbox::Control must be inside a Checkbox component");
        let send = ctx.send;

        // Build a UseMachineReturn-like wrapper so we can call derive().
        // In practice, CheckboxContext would carry the full UseMachineReturn or
        // expose a derive() helper. Here we reconstruct the needed pieces.
        let machine = UseMachineReturn {
            state: ctx.state,
            send: ctx.send,
            service: ctx.service,
            context_version: ctx.context_version,
        };

        // Derive control attributes reactively from the connect API.
        // This replaces manual state matching and hardcoded ARIA values.
        let control_attrs = machine.derive(|api| api.control_attrs());
        let data_state = machine.derive(|api| api.data_state().to_string());

        // Callback is Copy in Leptos 0.8 (arena-allocated), so `send` can be
        // used directly in multiple closures without rebinding.
        let on_click = move |_: MouseEvent| {
            send.run(checkbox::Event::Toggle);
        };

        let on_keydown = move |e: KeyboardEvent| {
            if KeyboardKey::from_key_str(&e.key()) == KeyboardKey::Space {
                e.prevent_default();
                send.run(checkbox::Event::Toggle);
            }
        };

        let on_focus = move |_: FocusEvent| {
            send.run(checkbox::Event::Focus);
        };

        let on_blur = move |_: FocusEvent| {
            send.run(checkbox::Event::Blur);
        };

        view! {
            <div
                // derive() wraps the control_attrs() return value — Vec<(&str, String)> —
                // in a reactive signal. Leptos can spread Vec<(attr, value)> directly.
                {..control_attrs.get()}
                data-ars-state=move || data_state.get()
                class=class.map(|c| move || c.get())
                on:click=on_click
                on:keydown=on_keydown
                on:focus=on_focus
                on:blur=on_blur
            >
                {children()}
            </div>
        }
    }

    /// The visual indicator inside the checkbox (checkmark, dash for indeterminate).
    #[component]
    pub fn Indicator(
        /// Only render children when checkbox matches this state.
        #[prop(optional)] match_state: Option<checkbox::State>,
        // ChildrenFn (not Children) because <Show> may re-invoke the closure on state change.
        children: ChildrenFn,
    ) -> impl IntoView {
        let CheckboxContext { state, .. } = use_context::<CheckboxContext>()
            .expect("checkbox::Indicator must be inside Checkbox");

        let should_show = move || {
            match match_state {
                Some(checkbox::State::Checked) => matches!(state.get(), checkbox::State::Checked),
                Some(checkbox::State::Indeterminate) => matches!(state.get(), checkbox::State::Indeterminate),
                Some(checkbox::State::Unchecked) => matches!(state.get(), checkbox::State::Unchecked),
                None => !matches!(state.get(), checkbox::State::Unchecked),
            }
        };

        let [(_, scope_val), (_, part_val)] = checkbox::Part::Indicator.data_attrs();
        view! {
            <span data-ars-scope=scope_val data-ars-part=part_val>
                <Show when=should_show>{children()}</Show>
            </span>
        }
    }

    /// Accessible label for the checkbox.
    #[component]
    pub fn Label(children: Children) -> impl IntoView {
        let [(_, scope_val), (_, part_val)] = checkbox::Part::Label.data_attrs();
        view! {
            <label data-ars-scope=scope_val data-ars-part=part_val>
                {children()}
            </label>
        }
    }

    /// Hidden native input for form submission.
    #[component]
    pub fn HiddenInput() -> impl IntoView {
        let ctx = use_context::<CheckboxContext>()
            .expect("checkbox::HiddenInput must be inside Checkbox");
        let machine = UseMachineReturn {
            state: ctx.state,
            send: ctx.send,
            service: ctx.service,
            context_version: ctx.context_version,
        };

        let name = machine.derive(|api| api.props().name.clone());
        let value = machine.derive(|api| api.props().value.clone().unwrap_or_else(|| "on".into()));
        let required = machine.derive(|api| api.props().required);

        // Use api.is_checked() for parity with Dioxus adapter.
        // is_checked() returns false for Indeterminate (per HTML spec, indeterminate checkbox does not submit).
        let checked = machine.derive(|api| api.is_checked());

        view! {
            <input
                type="checkbox"
                name=name
                value=value
                checked=checked
                required=required
                style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0"
                aria-hidden="true"
                tabindex="-1"
            />
        }
    }
}
```

### 5.3 Native Element Handler Deduplication

When rendering a machine's keyboard handlers onto a native interactive element (e.g., `<button>`), the adapter MUST strip handlers that duplicate native behavior:

- Native `<button>` fires `click` on Space keyup — the adapter strips the machine's Space key handler to avoid double activation.
- Native `<a>` fires `click` on Enter — the adapter strips the machine's Enter key handler.

The machine always generates the full handler set (it is DOM-element-agnostic). Deduplication is the adapter's responsibility.

### 5.4 Conditional Rendering with `Either`

When a closure or component returns different view types from branches (`if`/`else`, `match`), use `Either<A, B>` from `leptos::either` (or `EitherOf3`–`EitherOf16` for more branches). Do **not** use `.into_view()` or `.into_any()` for type unification — both erase concrete types and prevent Leptos from optimizing diffing.

```rust
// CORRECT: preserves concrete types
{move || if show_details.get() {
    Either::Left(view! { <DetailPanel machine=machine /> })
} else {
    Either::Right(view! { <SummaryRow machine=machine /> })
}}

// WRONG: type erasure via .into_view()
// {move || if show_details.get() {
//     view! { <DetailPanel machine=machine /> }.into_view()
// } else {
//     view! { <SummaryRow machine=machine /> }.into_view()
// }}
```

This applies to all conditional view returns in adapter components.

---

## 6. Slots Pattern

Leptos supports named slots via the `#[slot]` attribute:

```rust
mod dialog {
    /// Dialog with named slots for title and description.
    #[slot]
    pub struct Title {
        children: Children,
    }

    #[slot]
    pub struct Description {
        children: ChildrenFn,
    }

    #[component]
    pub fn Dialog(
        #[prop(optional, into)] open: Option<Signal<bool>>,
        #[prop(optional)] default_open: bool,
        #[prop(optional)] modal: Option<bool>,
        #[prop(optional)] dialog_title: Option<dialog::Title>,
        #[prop(optional)] dialog_description: Option<dialog::Description>,
        #[prop(optional)] on_open_change: Option<Callback<bool>>,
        children: Children,
    ) -> impl IntoView {
        let props = dialog::Props {
            open: open.as_ref().map(|s| s.get()),
            default_open,
            modal: modal.unwrap_or(true),
            ..Default::default()
        };

        let UseMachineReturn { state, send, service, context_version } = use_machine::<dialog::Machine>(props);
        provide_context(Context { state, send, service, context_version });

        let title_id = use_id("dialog-title");
        let desc_id = use_id("dialog-desc");

        provide_context(Ids { title_id: title_id.clone(), desc_id: desc_id.clone() });

        view! {
            <>
                {children()}
            </>
        }
    }

    /// Context shared between Dialog compound component parts.
    #[derive(Clone, Copy)]
    pub struct Context {
        pub state: ReadSignal<dialog::State>,
        pub send: Callback<dialog::Event>,
        pub service: StoredValue<Service<dialog::Machine>>,
        pub context_version: ReadSignal<u64>,
    }

    /// Close trigger for Dialog — renders a button that dismisses the dialog.
    /// Must be used inside a `Dialog` component.
    #[component]
    pub fn CloseTrigger(children: Children) -> impl IntoView {
        let ctx = use_context::<Context>()
            .expect("dialog::CloseTrigger must be inside Dialog");
        let machine = UseMachineReturn {
            state: ctx.state,
            send: ctx.send,
            service: ctx.service,
            context_version: ctx.context_version,
        };
        let close_label = machine.derive(|api| api.messages().close_label.clone());
        let [(.., scope_val), (.., part_val)] = dialog::Part::CloseTrigger.data_attrs();
        view! {
            <button
                type="button"
                data-ars-scope=scope_val
                data-ars-part=part_val
                aria-label=move || close_label.get()
                on:click=move |_| ctx.send.run(dialog::Event::Close)
            >
                {children()}
            </button>
        }
    }

    /// IDs for linking dialog title and description to the content element.
    #[derive(Clone)]
    pub struct Ids {
        pub title_id: String,
        pub desc_id: String,
    }
}
```

---

## 7. Server-Side Rendering (SSR)

### 7.1 Deterministic IDs

During SSR, IDs must be consistent between server and client for hydration:

```rust
// ID counter: thread_local Cell on WASM (no atomics overhead), AtomicU64 on native.
// Same pattern as the Dioxus adapter (09-adapter-dioxus.md §2, above use_machine_inner).
#[cfg(target_arch = "wasm32")]
thread_local! {
    static ID_COUNTER: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
#[cfg(target_arch = "wasm32")]
fn next_id() -> u64 {
    ID_COUNTER.with(|c| { let v = c.get(); c.set(v + 1); v })
}

#[cfg(not(target_arch = "wasm32"))]
static ID_COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
#[cfg(not(target_arch = "wasm32"))]
fn next_id() -> u64 { ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed) }

/// Generate a consistent ID that works for both SSR and CSR.
///
/// Uses a global monotonic counter that produces the same sequence on server
/// and client as long as the component tree is rendered in the same order.
/// Call `reset_id_counter()` at the start of each SSR request.
///
/// > **Warning:** This counter is NOT inherently hydration-safe. SSR+hydration
/// > users MUST provide explicit `id` props on all components until a
/// > deterministic tree-position-based ID scheme is implemented (same
/// > limitation as the Dioxus adapter — see 09-adapter-dioxus.md §20.2).
pub fn use_id(scope: &'static str) -> String {
    format!("ars-{scope}-{}", next_id())
}

/// Reset the ID counter. MUST be called at the start of each SSR request
/// on the **server** to ensure server and client counters are in sync.
/// Not needed on the hydrate client — only the SSR request handler.
// The wasm32+ssr branch covers WASM-based SSR runtimes (e.g., Cloudflare Workers).
// The native branch covers standard server-side SSR (Linux, macOS).
#[cfg(all(feature = "ssr", target_arch = "wasm32"))]
pub fn reset_id_counter() {
    ID_COUNTER.with(|c| c.set(0));
}
#[cfg(all(feature = "ssr", not(target_arch = "wasm32")))]
pub fn reset_id_counter() {
    ID_COUNTER.store(0, core::sync::atomic::Ordering::Relaxed);
}

/// Generate a related ID (for linking label <-> input, trigger <-> content).
pub fn related_id(base: &str, suffix: &str) -> String {
    format!("{}-{}", base, suffix)
}
```

### 7.2 SSR-Compatible Components

```rust
mod tooltip {
    /* Other tooltip components and structs */

    #[component]
    // ChildrenFn (not Children) because <Show> may re-invoke the closure on toggle.
    pub fn Content(children: ChildrenFn) -> impl IntoView {
        let Context { state, .. } = use_context::<Context>()
            .expect("tooltip::Content must be inside Tooltip");

        let is_visible = move || matches!(state.get(), tooltip::State::Visible);

        // On the server, render content hidden but in DOM for SEO
        // On the client, control visibility reactively
        let [(.., scope_val), (.., part_val)] = tooltip::Part::Content.data_attrs();
        #[cfg(feature = "ssr")]
        return view! {
            <div
                role="tooltip"
                data-ars-scope=scope_val
                data-ars-part=part_val
                style="display:none"
            >
                {children()}
            </div>
        };

        #[cfg(not(feature = "ssr"))]
        return view! {
            <Show when=is_visible>
                <div
                    role="tooltip"
                    data-ars-scope=scope_val
                    data-ars-part=part_val
                >
                    {children()}
                </div>
            </Show>
        };
    }
}
```

### 7.3 Hydration Considerations

```rust
/// Serialize component state for hydration.
/// Used for components that need to hydrate with correct initial state.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HydrationSnapshot<M>
where
    M: Machine,
    M::State: serde::Serialize + serde::de::DeserializeOwned,
{
    pub state: M::State,
    pub id: String,
}

// In SSR mode: embed snapshot in HTML via script tag
// In hydration mode: read snapshot and initialize machine from it
// This ensures the machine starts in the same state as the server rendered
```

### 7.4 SSR Effect Behavior

SSR effect gating is defined in §3.4 — effects are not executed during server-side rendering.

#### 7.4.1 Clipboard "Copied" State and SSR Hydration

The Clipboard component's `feedback_duration_ms` timeout resets the "copied" state back to idle after a delay. During SSR hydration, if a `useEffect` runs before the first paint, the visual "copied" text may flash briefly and disappear (or get stuck if the browser defers cleanup).

**Rules for SSR-safe Clipboard rendering:**

1. **SSR always renders idle state**: The server MUST render Clipboard in the `Idle` state regardless of any prior interaction. The `init()` function always returns `State::Idle`, so this is the default behavior.

2. **Use CSS animation instead of state-based timeout for the "copied" indicator**: Rather than relying on the state machine's `ResetTimeout` event to remove the visual feedback, adapters SHOULD use a CSS animation on the indicator element that auto-hides after the feedback duration. This avoids the hydration race entirely:

   ```css
   [data-ars-scope="clipboard"][data-ars-state="copied"]
     [data-ars-part="indicator"] {
     animation: ars-clipboard-feedback var(--ars-clipboard-duration, 2000ms)
       ease-out forwards;
   }

   @keyframes ars-clipboard-feedback {
     0%,
     90% {
       opacity: 1;
     }
     100% {
       opacity: 0;
     }
   }
   ```

3. **Defer reset-timeout until after first interaction**: The `feedback-timer` effect MUST NOT run during hydration. The adapter gates the timer setup behind a `has_interacted` flag that is only set to `true` after the first `Event::Copy` is sent by user interaction. This prevents orphaned timers from the SSR→hydration transition.

4. **No flash on hydration**: Since SSR renders `data-ars-state="idle"` and no timer is running, the hydrated client starts in idle state with no visual glitch. The CSS animation approach means the "copied" visual is driven by CSS, not by a JavaScript timeout that could race with hydration.

### 7.5 Effect Cleanup and Event Safety

**Problem.** During effect cleanup, removing event listeners can itself trigger synthetic events — `blur` fires when a focused element's listener is removed, `pointerup` may arrive after a transition completes but before new effects are wired. If cleanup and setup overlap, stale callbacks execute against new component state, causing panics or incorrect behavior.

**Rules:**

1. **Cleanup ordering.** All listener removals MUST execute before any new listeners are registered. The adapter enforces this by splitting the effect lifecycle into two phases: a synchronous cleanup phase and a subsequent setup phase.

2. **Idempotent cleanup.** Cleanup functions MUST be safe to call multiple times. Leptos may invoke `on_cleanup` more than once during rapid re-renders or concurrent transitions. Guard against double-removal by checking a `cleaned_up: StoredValue<bool, LocalStorage>` flag.

3. **Weak-guard pattern for stale callbacks.** Store long-lived callbacks (e.g., global `keydown` handlers registered on `document`) as `Weak<Box<dyn Fn(...)>>`. Before invoking, upgrade the weak reference — if the owning scope has been disposed, the upgrade returns `None` and the callback is silently skipped.

4. **Batch removals, then batch registrations.** Never interleave individual remove/add pairs. Collect all pending removals into a `Vec`, execute them synchronously, then collect and execute all registrations.

```rust
use leptos::prelude::*;
use std::rc::{Rc, Weak};
use web_sys::wasm_bindgen::closure::Closure;

/// Attaches an event listener with framework-managed lifecycle cleanup.
///
/// Uses raw `web_sys::Closure` and `EventTarget::add_event_listener_with_callback`
/// directly because framework-specific cleanup primitives (`StoredValue`/`on_cleanup`
/// in Leptos) require owning the Closure handle. The ars-dom `EventListenerHandle`
/// utility (11-dom-utilities.md §7) does not integrate with framework reactivity
/// systems. See v93 follow-up discussion.
fn use_safe_event_listener(
    target: NodeRef<html::Div>,
    event_name: &'static str,
    handler: impl Fn(web_sys::Event) + 'static,
) {
    let handler: Rc<Box<dyn Fn(web_sys::Event)>> = Rc::new(Box::new(handler));
    let weak_handler: Weak<Box<dyn Fn(web_sys::Event)>> = Rc::downgrade(&handler);

    // Store the closure so we can remove it on cleanup.
    // `new_local` accepts `!Send` types — no wrapper needed.
    let closure_handle: StoredValue<Option<Closure<dyn Fn(web_sys::Event)>>, LocalStorage> =
        StoredValue::new_local(None);
    let cleaned_up: StoredValue<bool, LocalStorage> = StoredValue::new_local(false);

    Effect::new(move |_| {
        let Some(el) = target.get() else { return };
        let el: web_sys::HtmlElement = el.into();

        // Phase 1: Synchronous cleanup of previous listener.
        // Closure is !Clone, so we cannot use read_value() (which requires T: Clone).
        // Instead, use update_value + take to extract the Option without cloning.
        let prev_closure = { let mut out = None; closure_handle.update_value(|v| out = v.take()); out };
        if let Some(prev_closure) = prev_closure {
            el.remove_event_listener_with_callback(
                event_name,
                prev_closure.as_ref().unchecked_ref(),
            )
            .ok();
        }

        // Phase 2: Register new listener with weak reference guard.
        let weak = weak_handler.clone();
        let closure = Closure::new(move |event: web_sys::Event| {
            // Note: TOCTOU window exists between alive check and handler execution
            if let Some(strong) = weak.upgrade() {
                (*strong)(event);
            }
            // If upgrade fails, the owning scope is gone — skip silently.
        });

        el.add_event_listener_with_callback(
            event_name,
            closure.as_ref().unchecked_ref(),
        )
        .expect("addEventListener");

        closure_handle.set_value(Some(closure));
        cleaned_up.set_value(false);
    });

    // on_cleanup is idempotent — safe to call multiple times.
    on_cleanup(move || {
        if *cleaned_up.read_value() {
            return;
        }
        cleaned_up.set_value(true);

        if let Some(el) = target.get_untracked() {
            let el: web_sys::HtmlElement = el.into();
            let prev_closure = { let mut out = None; closure_handle.update_value(|v| out = v.take()); out };
            if let Some(prev_closure) = prev_closure {
                el.remove_event_listener_with_callback(
                    event_name,
                    prev_closure.as_ref().unchecked_ref(),
                )
                .ok();
            }
        }
    });
}
```

---

## 8. API Naming Conventions

All `ars-leptos` components follow uniform naming conventions for accessors, state, and callbacks. These rules apply across every component in the adapter.

### 8.1 Boolean Accessors — `is_*()` Methods

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

### 8.2 Non-Boolean State — Getter Methods or Field Access

Non-boolean values use getter methods (or direct field access for simple data):

```rust
api.value()             // current value (String, number, etc.)
api.selected_items()    // current selection set
api.highlighted_key()   // currently highlighted item key
api.placeholder()       // placeholder text
api.orientation()       // Orientation enum
```

### 8.3 Event Callbacks — `on_*` Naming

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

### 8.4 Module-Scoped Compound Component Naming

Adapter component specs that expose compound parts must use module scoping instead of
repeating the component name in every part symbol:

```rust
pub mod dialog {
    #[component]
    pub fn Dialog(...) -> impl IntoView

    #[component]
    pub fn Trigger(...) -> impl IntoView

    #[component]
    pub fn Content(...) -> impl IntoView
}
```

Rules:

- The root component uses the bare component name (`dialog::Dialog`, `tooltip::Tooltip`).
- Child parts drop the redundant component prefix (`dialog::Trigger`, not `DialogTrigger`).
- The primary child-part context inside the module is named `Context`.
- Secondary contexts or helpers use descriptive non-prefixed names (`GroupContext`, `QueueContext`, `Overlay`, `Control`).
- Expect or panic messages must use the module-qualified part name (`dialog::Trigger must be used inside Dialog`).

### 8.5 Summary Table

| Category             | Convention                            | Examples                                                 |
| -------------------- | ------------------------------------- | -------------------------------------------------------- |
| Boolean accessor     | `is_*()` method                       | `is_disabled()`, `is_open()`, `is_checked()`             |
| Non-boolean accessor | `value()` / `selected_items()` method | `value()`, `highlighted_key()`, `orientation()`          |
| Compound parts       | module-scoped symbols                 | `dialog::Trigger`, `tooltip::Content`, `toast::Provider` |
| Event callback       | `on_*` prop                           | `on_change`, `on_select`, `on_open_change`               |

---

## 9. Callback Naming Convention

All public callback props follow a consistent naming convention across `ars-leptos` components:

| Pattern                | Usage                                                         | Examples                                                     |
| ---------------------- | ------------------------------------------------------------- | ------------------------------------------------------------ |
| `on_<property>_change` | Fires when a **value** changes (controlled component pattern) | `on_value_change`, `on_open_change`, `on_checked_change`     |
| `on_<action>`          | Fires on a **discrete user action** (not a state change)      | `on_press`, `on_submit`, `on_dismiss`, `on_focus`, `on_blur` |

**Rules:**

- Value-change callbacks always receive the **new value** as their argument (e.g., `Callback<bool>` for `on_open_change`)
- Action callbacks receive either no argument or an event-specific payload — never the full component state
- Callback props are always `Option<Callback<T>>` — omitting a callback is valid and means the consumer does not observe that event
- The `emit()` helper (below) handles the `Option` check so call sites stay clean

---

## 10. Event Callbacks Pattern

````rust
/// Emit a value through an optional Leptos callback.
///
/// # Example
/// ```rust
/// emit(props.on_value_change.as_ref(), selected_value);
/// ```
pub fn emit<T: Clone>(callback: Option<&Callback<T>>, value: T) {
    if let Some(cb) = callback {
        cb.run(value);
    }
}

/// Map a value before emitting.
pub fn emit_map<T, U: Clone>(callback: Option<&Callback<U>>, value: T, f: impl Fn(T) -> U) {
    if let Some(cb) = callback {
        cb.run(f(value));
    }
}
````

---

## 11. Collections Integration

For list-based components (Select, Listbox, Menu):

````rust
use ars_collections::{Collection, Key};

/// Render a collection with Leptos's `<For>` component.
///
/// `<For>` requires a key function for efficient diffing.
#[component]
pub fn CollectionView<T: Clone + PartialEq + 'static>(
    items: Signal<Vec<T>>,
    key: impl Fn(&T) -> String + 'static,
    view: impl Fn(T) -> AnyView + 'static,
) -> impl IntoView {
    view! {
        <For
            each=move || items.get()
            key=key
            children=view
        />
    }
}

/// Select with async-loaded items using Resource + Suspense:
///
/// ```rust
/// #[component]
/// fn CountrySelect(on_change: Option<Callback<String>>) -> impl IntoView {
///     let countries = Resource::new(
///         || (),
///         |_| async { fetch_countries().await }
///     );
///
///     view! {
///         <Select on_value_change=on_change.map(|cb| move |v: String| cb.run(v))>
///             <select::Trigger><select::ValueText /></select::Trigger>
///             <select::Content>
///                 <Suspense fallback=|| view! { <div>"Loading..."</div> }>
///                     {move || countries.get().map(|items| {
///                         items.into_iter().map(|c| view! {
///                             <select::Item value=c.code.clone()>
///                                 {c.name.clone()}
///                             </select::Item>
///                         }).collect_view()
///                     })}
///                 </Suspense>
///             </select::Content>
///         </Select>
///     }
/// }
/// ```
````

---

## 12. Animation and Presence

Overlay exit animations are handled by the **Presence** machine (see `spec/components/overlay/presence.md`). Each overlay component (Dialog, Popover, Tooltip) composes Presence internally:

1. When the overlay's `is_open` becomes `true`, send `presence::Event::Mount`.
2. When `is_open` becomes `false`, send `presence::Event::Unmount`.
3. Presence defers unmounting until the CSS exit animation completes.
4. The adapter reads `presence_api.is_mounted()` to decide whether to render the element.

```rust
// Usage in a Leptos overlay component:
let presence = use_machine::<presence::Machine>(presence::Props::default());
let is_mounted = presence.derive(|api| api.is_mounted());

// When dialog state changes:
// Exception: calling send inside this effect is safe — the dependency
// (is_dialog_open) is an external input, not derived from Presence state,
// so no reactive loop can form.
Effect::new(move |_| {
    if is_dialog_open() {
        presence.send.run(presence::Event::Mount);
    } else {
        presence.send.run(presence::Event::Unmount);
    }
});

view! {
    <Show when=move || is_mounted.get()>
        <div data-ars-state=move || if is_dialog_open() { "open" } else { "closed" }>
            {children()}
        </div>
    </Show>
}
```

No adapter-level `create_presence()` helper is needed — Presence is a standard machine used via `use_machine`.

---

> **Per-component adapter examples** have been extracted to individual files under `spec/leptos-components/{category}/{component}.md`.
> Use `cargo run -p spec-tool -- deps <component>` to find the Leptos adapter file for any component.

---

## 13. ArsProvider Context

`ArsProvider` is the single root provider — the formerly separate `LocaleProvider`,
`PlatformEffectsProvider`, and `ArsStyleProvider` are subsumed. The adapter-level context
wraps core `ArsContext` values in reactive signals and includes the platform capabilities
trait object.

```rust
use ars_i18n::{Locale, Direction};
use ars_core::{ArsRc, ColorMode, PlatformEffects, StyleStrategy};
use ars_i18n::IcuProvider;

/// Reactive environment context published by the Leptos ArsProvider adapter.
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
    pub platform: ArsRc<dyn PlatformEffects>,
    pub icu_provider: ArsRc<dyn IcuProvider>,
    pub i18n_registries: ArsRc<I18nRegistries>,
    /// Non-reactive style strategy — set once at provider mount time.
    style_strategy: StyleStrategy,
}

impl ArsContext {
    /// Returns the active CSS style injection strategy.
    pub fn style_strategy(&self) -> &StyleStrategy {
        &self.style_strategy
    }
}
```

The `ArsProvider` component, its props, and rendering are specified in
`spec/leptos-components/utility/ars-provider.md`. The component publishes
`ArsContext` via `provide_context` and renders a `<div dir=dir_attr>` wrapper.
The Leptos adapter defaults `platform` to `Rc::new(WebPlatformEffects)` on web targets.

### 13.1 use_locale()

```rust
/// Access the current locale from any component.
/// Falls back to `en-US` if no `ArsProvider` is present.
///
/// **Important:** Do not call `use_locale()` inside reactive closures or effects.
/// Leptos components run once, so setup-time calls are safe (one allocation per component).
/// Calling it inside a closure that re-runs would allocate a new `Signal::stored` on
/// each invocation, causing unbounded allocations.
pub fn use_locale() -> Signal<Locale> {
    use_context::<ArsContext>()
        .map(|ctx| ctx.locale)
        .unwrap_or_else(|| {
            warn_missing_provider("use_locale");
            // Signal::stored wraps a static non-reactive value (backed by ArcStoredValue).
            // Appropriate here because the fallback locale never changes.
            Signal::stored(Locale::parse("en-US").expect("en-US is always a valid BCP 47 locale"))
        })
}
```

> **Re-render optimization note**: Leptos's fine-grained reactivity means that
> reading `locale.get()` inside a component only causes that specific DOM node
> to update — not the entire component subtree. This is why Leptos does NOT need
> the granular signal splitting pattern shown in the Dioxus adapter §5 "Re-render Optimization". Each
> `derive()` call creates an independent memo that only updates its dependents
> when its output value actually changes.

### 13.2 resolve_locale() — Adapter Locale Resolution

An adapter-only utility (not available in core crates) that resolves the effective locale for a component. If the adapter-level component provides a per-instance `locale` prop override, that value is used; otherwise falls back to the `ArsProvider` context locale.

```rust
/// Resolve the effective locale for a component.
///
/// - If `adapter_props_locale` is `Some`, use the per-instance override.
/// - Otherwise, fall back to `use_locale()` from `ArsProvider` context.
///
/// This is a pure adapter utility — core code receives a fully-resolved
/// `Locale` via `Env` and never calls this function.
fn resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale {
    adapter_props_locale
        .cloned()
        .unwrap_or_else(|| use_locale().get())
}
```

### 13.3 t() — Translatable Text Resolver

```rust
use ars_i18n::Translate;

/// Resolve a user-defined `Translate` enum variant into a reactive text node.
///
/// Reads the current locale and ICU provider from `ArsProvider` context,
/// then returns a reactive closure that calls `msg.translate()`. The closure
/// subscribes to the locale signal — when locale changes, only the text node
/// re-evaluates (fine-grained reactivity).
///
/// Included in `ars_leptos::prelude`.
///
/// See `04-internationalization.md` §7.4 for the `Translate` trait definition
/// and §7.5 for the `t()` function contract.
#[inline]
#[must_use]
pub fn t<T: Translate>(msg: T) -> impl IntoView {
    let locale = use_locale();
    let icu = use_icu_provider();
    move || msg.translate(&locale.get(), &*icu)
}
```

**Prelude export:** `pub use crate::i18n::t;`

---

## 14. Event Mapping

Leptos exposes DOM events directly via `web_sys` types. Keyboard events use the DOM `key` property (a string), which `KeyboardKey::from_key_str()` parses:

```rust
use ars_core::KeyboardKey;

/// Leptos KeyboardEvent -> ars-core KeyboardKey.
/// Leptos exposes standard DOM KeyboardEvent — the `key()` method returns the
/// DOM-spec key string (e.g., "Enter", "ArrowUp", " " for Space).
pub fn leptos_key_to_keyboard_key(ev: &web_sys::KeyboardEvent) -> (KeyboardKey, Option<char>) {
    let key_str = ev.key();
    let ch = if key_str.len() == 1 { key_str.chars().next() } else { None };
    (KeyboardKey::from_key_str(&key_str), ch)
}
```

> **Parity note:** The Dioxus adapter (§13.1) provides an exhaustive `match` on `dioxus::Key` enum variants. The Leptos adapter delegates to `KeyboardKey::from_key_str()` because Leptos passes standard DOM `KeyboardEvent` objects whose `.key()` method returns the [DOM UI Events KeyboardEvent key](https://www.w3.org/TR/uievents-key/) string directly.

---

## 15. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::*;

    // Unit test: machine transitions without DOM
    #[test]
    fn checkbox_toggles() {
        let props = checkbox::Props::default();
        let env = Env::default();
        let messages = checkbox::Messages::default();
        let mut svc = Service::<checkbox::Machine>::new(props, env, messages);

        assert_eq!(*svc.state(), checkbox::State::Unchecked);
        svc.send(checkbox::Event::Toggle);
        assert_eq!(*svc.state(), checkbox::State::Checked);
        svc.send(checkbox::Event::Toggle);
        assert_eq!(*svc.state(), checkbox::State::Unchecked);
    }
}

// Integration tests use wasm-bindgen-test:
#[cfg(target_arch = "wasm32")]
mod dom_tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn checkbox_renders_correct_aria() {
        mount_to_body(|| {
            view! {
                <Checkbox>
                    <checkbox::Control>
                        <checkbox::Indicator />
                    </checkbox::Control>
                </Checkbox>
            }
        });
        tick().await;

        let control = document()
            .query_selector("[data-ars-part='control']")
            .expect("query_selector must succeed in test")
            .expect("control element must exist in rendered output");
        assert_eq!(control.get_attribute("role").as_deref(), Some("checkbox"));
        assert_eq!(control.get_attribute("aria-checked").as_deref(), Some("false"));

        // Click to toggle
        control.dyn_ref::<web_sys::HtmlElement>().expect("control must be HtmlElement").click();
        tick().await;
        assert_eq!(control.get_attribute("aria-checked").as_deref(), Some("true"));
    }
}
```

---

## 16. Controlled Value Helper

All controlled prop watchers follow the same pattern: track previous value, skip initial, send event on change. This helper extracts the repeated logic:

````rust
/// Watch a reactive signal and dispatch an event when its value changes.
/// Skips the initial mount (machine already has correct initial value from props).
///
/// # Example
/// ```rust
/// use_controlled_prop(checked_sig, send, |v| checkbox::Event::SetChecked(v));
/// ```
pub fn use_controlled_prop<T: Clone + PartialEq + 'static, E: 'static>(
    signal: Signal<T>,
    send: Callback<E>,
    event_fn: impl Fn(T) -> E + 'static,
) {
    let prev: StoredValue<Option<T>> = StoredValue::new(None);
    Effect::new(move |_| {
        let new_val = signal.get();
        let should_send = prev.with_value(|p| p.as_ref() != Some(&new_val));
        if should_send {
            let is_initial = prev.with_value(|p| p.is_none());
            if !is_initial {
                send.run(event_fn(new_val.clone()));
            }
            prev.set_value(Some(new_val));
        }
    });
}
````

> **Staleness note:** Because Leptos effects are deferred (via `Effect::new`), there is a guaranteed one-frame stale window: the first render after a controlled signal change uses the old value. The Dioxus adapter avoids this by using body-level synchronous sync (`use_controlled_prop_sync`). Body-level sync is not possible in Leptos because component functions execute only once (at mount). Subsequent signal changes are observed only through reactive subscriptions (Effect, Memo), which are inherently deferred. **Mitigation:** For components where one-frame staleness is unacceptable (e.g., rapidly-updated controlled values), consider migrating to `Memo::new` or inline body-level checks before the first render, matching the Dioxus approach.
>
> **Optional controlled props:** For optional controlled props (`Option<T>`), Leptos adapters use conditional effect creation: `if let Some(signal) = controlled_signal { use_controlled_prop(signal, ...) }`. This is safe in Leptos because effects can be conditionally created without violating hook ordering rules (unlike React). See Dioxus adapter section 19 for the Dioxus-specific `use_controlled_prop_sync_optional` helper.

Usage in components (replaces hand-rolled previous-value guards):

```rust
#[component]
pub fn Checkbox(
    #[prop(optional, into)] checked: Option<Signal<checkbox::State>>,
    #[prop(optional, into)] disabled: Signal<bool>,
    // ... other props
) -> impl IntoView {
    let machine = use_machine::<checkbox::Machine>(props);
    let send = machine.send;

    // Controlled value watchers — clean, DRY pattern
    if let Some(checked_sig) = checked {
        use_controlled_prop(checked_sig, send, checkbox::Event::SetChecked);
    }
    use_controlled_prop(disabled, send, checkbox::Event::SetDisabled);

    // ...
}
```

---

## 17. Error Boundary Pattern

Wrap component trees with `ErrorBoundary` to gracefully handle machine panics
or unexpected state transitions:

```rust
#[component]
pub fn ArsErrorBoundary(children: Children) -> impl IntoView {
    view! {
        <ErrorBoundary fallback=|errors: ArcRwSignal<Errors>| view! {
            <div data-ars-error="true" role="alert">
                <p>"A component encountered an error."</p>
                <ul>
                    {move || errors.get()
                        .into_iter()
                        .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                        .collect_view()
                    }
                </ul>
            </div>
        }>
            {children()}
        </ErrorBoundary>
    }
}
```

---

## 18. Machine Type Parameter Bounds

All `Machine` type parameters in adapter hooks must satisfy `M: Machine + 'static`. This is required because framework reactive primitives (Leptos `StoredValue`) require `'static` storage. Consequently, `Machine::Props` must be `'static` — use `Rc<T>` or `Arc<T>` for shared ownership instead of references.

---

## 19. Api Lifetime and Async Event Handlers

`Api` borrows from `Service` and cannot be held across `.await` points. For async event handlers, clone the send callback from the `UseMachineReturn`:

```rust
let send = machine.send; // Callback is Copy (arena-allocated in Leptos 0.8)
// then use inside async block:
spawn_local(async move {
    let result = fetch_data().await;
    send.run(MyEvent::DataLoaded(result));
});
```

Do not hold `Api` references across await boundaries.
