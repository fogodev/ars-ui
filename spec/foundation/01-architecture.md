# Architecture Specification

## 1. Crate Structure

### 1.1 Workspace Layout

```text
ars-ui/
  Cargo.toml                    # Workspace root
  crates/
    ars-core/                   # State machine engine (no_std compatible)
    ars-a11y/                   # ARIA types, focus management, keyboard nav, screen reader utilities
    ars-i18n/                   # Locale, RTL, formatting, calendars
    ars-interactions/           # Press, hover, focus, long press, move, DnD
    ars-collections/            # Collection trait, selection, virtualization, async loading
    ars-forms/                  # Validation, form context, field association, hidden inputs
    ars-components/             # Framework-agnostic component machines and connect APIs
    ars-dom/                    # web-sys DOM utilities, positioning, portal, focus, scroll, URL sanitization, inert
    ars-leptos/                 # Leptos adapter
    ars-dioxus/                 # Dioxus adapter
    ars-derive/                  # Proc-macro: #[derive(HasId)], #[derive(ComponentPart)]
```

### 1.2 Crate Dependency Graph

```text
                      ars-core (no_std)
                     /    |    \
                    /     |     \
             ars-a11y  ars-i18n  ars-interactions        ars-derive (proc-macro, compile-time only → ars-core)
                |  \      |           |
                |   \     |           | (abstract event mapping; DOM normalization in ars-dom)
                |    \    |           |
      ars-collections  ars-forms  ars-components
                |          |          |
                |   ars-forms --> ars-i18n  (date/time types: CalendarDate, Time, DateRange)
                |   ars-forms --> ars-core  (ComponentIds)
                |   ars-components --> ars-core, ars-forms, ars-i18n, ars-interactions
                |          |          |
                |     ars-a11y ──┐    |
                |     ars-i18n ──┼──> ars-dom  (web-sys, wasm-bindgen)
                |     ars-interactions ┘
                |            /       \
                |     ars-leptos    ars-dioxus ·····> ars-dom (optional)
```

> **Note:** `ars-dom` is an optional dependency for `ars-dioxus`. Dioxus Desktop can
> supply its own positioning configuration via the `PositioningOptions` struct passed
> to `compute_position()` without pulling in `web-sys`. The dotted arrow indicates
> this optional relationship.
>
> **Explicit edges:** ars-a11y, ars-i18n → ars-core; ars-interactions → ars-core, ars-a11y, ars-i18n; ars-collections → ars-core, ars-a11y [, ars-i18n (optional, feature = "i18n")]; ars-forms → ars-core, ars-a11y, ars-i18n; ars-components → ars-core, ars-forms, ars-i18n, ars-interactions; ars-dom → ars-core, ars-a11y, ars-i18n, ars-interactions; ars-leptos, ars-dioxus → ars-dom (+ all transitive, and to ars-components when consuming shared machines). Note: `ars-dom` does **not** depend on `ars-forms` or `ars-components` — browser/runtime behavior remains adapter-owned. Note: `ars-interactions → ars-i18n` is required for `DragAnnouncements` which uses `Locale` and `Direction` from `ars-i18n`, and `ars-interactions → ars-a11y` is required for `LiveAnnouncer` / `AnnouncementPriority` integration in keyboard drag-and-drop and other live announcements.
>
> **Note:** The previous `ars-forms → ars-collections` edge has been removed. `ars-forms` defines abstract trait bounds (`SelectionFormExt`, `CheckboxFormExt`) that downstream crates satisfy — the dependency direction is reversed (components depend on forms traits, not vice versa).

### 1.3 Feature Flags Strategy

```toml
# ars-core/Cargo.toml
[features]
default = ["std"]
std = []                          # Enable std library support
serde = ["dep:serde"]             # Serialization for state snapshots
ssr = []                          # Server-side rendering: enables Service::new_hydrated
debug = ["dep:log"]               # Trace-level logging for state transitions and effects
embedded-css = []                 # Opt-in embedded ars-base.css constant for asset pipelines

# ars-i18n/Cargo.toml
[features]
default = ["std", "icu4x"]
std = []
icu4x = ["dep:icu"]            # Rust-native ICU4X backend (default for non-WASM targets)
web-intl = ["dep:js-sys"]      # Browser Intl API via wasm-bindgen (WASM-only)

# Enforce mutual exclusion at compile time:
# #[cfg(all(feature = "icu4x", feature = "web-intl"))]
# compile_error!("features `icu4x` and `web-intl` are mutually exclusive");

# Calendar support is unconditional at the public API layer.
# Binary-size-sensitive builds prune backend data, not public calendar types.

# ars-interactions/Cargo.toml
[features]
default = ["aria-drag-drop-compat"]
aria-drag-drop-compat = []        # Emit deprecated aria-grabbed/aria-dropeffect for legacy AT
debug = ["dep:log", "ars-core/debug"]  # Dev-time warnings for interaction attr conflicts

# ars-dom/Cargo.toml
# Non-feature deps: ars-core, ars-a11y, ars-i18n, ars-interactions (always included)
[features]
default = ["web"]
debug = ["dep:log", "ars-core/debug", "ars-interactions/debug"]  # Dev-time DOM/platform diagnostics
ssr = []                          # Server-side rendering support (no web-sys)
web = ["dep:web-sys", "dep:wasm-bindgen", "dep:js-sys"]  # Browser DOM access

# ars-leptos/Cargo.toml
[features]
default = []
debug = ["dep:log", "ars-core/debug", "ars-dom/debug", "ars-interactions/debug"]
ssr = ["leptos/ssr", "ars-dom/ssr"]
hydrate = ["leptos/hydrate"]
csr = ["leptos/csr"]

# ars-a11y/Cargo.toml
[features]
default = ["aria-drag-drop-compat"]
aria-drag-drop-compat = []    # Emit deprecated aria-grabbed/aria-dropeffect types
                              # NOTE: ars-interactions uses `aria-drag-drop-compat` for the same purpose;
                              # when enabling/disabling, both crates' flags must stay in sync.
axe = ["dep:serde_json"]       # Runtime accessibility validation helpers

# ars-collections/Cargo.toml
[features]
default = ["std"]
std = []                        # Standard library types (enabled by default)
i18n = ["dep:ars-i18n"]        # Locale-aware collation via ars-i18n
serde = ["dep:serde"]          # Serialization for selection state

# ars-forms/Cargo.toml
[features]
default = []
serde = ["dep:serde"]          # Serialization for form data

# ars-dioxus/Cargo.toml
[features]
default = []
debug = ["dep:log", "ars-core/debug", "ars-interactions/debug", "ars-dom?/debug"]
web = ["dioxus/web", "dep:ars-dom", "ars-dom/web"]
desktop = ["dioxus/desktop"]
desktop-dom = ["desktop", "dep:ars-dom"]  # ars-dom declared with default-features = false; no web-sys on desktop
mobile = ["dioxus/mobile"]
ssr = ["dioxus/server", "dep:ars-dom", "ars-dom/ssr"]
```

> **Per-component feature flags:** Per-component feature flags (e.g., `splitter`, `carousel`) live in the adapter crates. See `shared/layout-shared-types.md` §3 and the adapter spec files for per-category feature flag maps.
>
> **Web API guard convention:** All `web_sys` and `wasm_bindgen` calls MUST be behind `#[cfg(target_arch = "wasm32")]` or the appropriate framework feature flag (e.g., `#[cfg(feature = "web")]` for Dioxus, `#[cfg(not(feature = "ssr"))]` for Leptos). This prevents compilation failures on non-WASM targets and ensures multi-platform compatibility.

### 1.4 no_std Considerations

`ars-core` must be `no_std` compatible:

- Use `alloc` for `Vec`, `String`, `Box` where needed
- No filesystem, threading, or I/O in core
- Time-dependent logic (delays, debounce) is handled via `PendingEffect` closures — the adapter schedules the delay and sends a follow-up event when it fires. No separate `Timer` trait is needed.
- Random ID generation uses a trait-based approach

> **Debug logging:** `ars-core` provides an optional `debug` feature flag that enables the `log` crate facade. When enabled, state transitions, guard evaluations, and effect dispatches emit trace-level log events. Adapters must initialize a logger (e.g., `console_log` for WASM, `env_logger` for native). Without the feature flag, all logging is compiled out. This resolves the `no_std` limitation on debug output.
>
> **Design note — single-threaded machine mutation:** `ars-core` machine services are still driven from a single logical thread, even though shared callback handles such as `Callback<T>` and the `send` callback passed to `PendingEffect::setup` use `Arc` on all targets. Desktop adapters (e.g., Dioxus Desktop) must still ensure machine mutation remains on one thread at a time.

#### 1.4.1 Thread Safety for Desktop Adapters

`ars-core` machines are thread-safe for immutable reads (e.g., reading `state()`, `context()`, `props()`) but are NOT safe for concurrent mutation. The `Rc`-based design assumes single-threaded access.

Desktop adapters (Dioxus Desktop, Tauri) MUST ensure that each machine instance stays on a single thread. Strategies:

1. **Scheduler affinity**: Pin all machine operations to the UI thread via the framework's event loop (Dioxus Desktop's `use_coroutine` runs on the main thread by default).
2. **Thread-local storage**: Store `Service<M>` in thread-local state and panic if accessed from a different thread.
3. **`Send` bound opt-in**: For adapters that need `Send` across threads, use the `cfg`-gated `Callback<T>` (which uses `Arc` on non-WASM targets) and wrap `Service<M>` in a `Mutex` at the adapter level. This adds overhead and should only be used when necessary.

> **Warning:** Sharing a `Service<M>` across threads without synchronization is undefined behavior. The `Rc` references will not be dropped correctly, leading to memory leaks or use-after-free.

#### 1.4.2 Per-crate `no_std` Compatibility

| Crate              | `no_std` | `alloc` |  `std`   | Rationale                                                                                                                        |
| ------------------ | :------: | :-----: | :------: | -------------------------------------------------------------------------------------------------------------------------------- |
| `ars-core`         |    Y     |    Y    | optional | Core engine must run in any environment                                                                                          |
| `ars-derive`       |   N/A    |   N/A   |   N/A    | Proc-macro crate, runs at compile-time only                                                                                      |
| `ars-a11y`         |    Y     |    Y    | implicit | ARIA types and ID generation are pure data; depends on `unicode-normalization` (no `std` feature gate needed — `alloc` suffices) |
| `ars-i18n`         |    —     |    Y    |    Y     | ICU4X or `Intl` require std-level features                                                                                       |
| `ars-interactions` |    —     |    Y    |    Y     | Requires std via `ars-i18n` (DragAnnouncements uses `Locale`, `Direction`)                                                       |
| `ars-collections`  |    Y     |    Y    | implicit | Collection trait and selection model (no `std` feature gate needed — `alloc` suffices)                                           |
| `ars-forms`        |    —     |    Y    |    Y     | Depends on ars-core (ComponentIds), ars-i18n (CalendarDate, Time), indexmap 2.x                                                  |
| `ars-dom`          |    —     |    —    |    Y     | Requires `web-sys` / `wasm-bindgen`                                                                                              |
| `ars-leptos`       |    —     |    —    |    Y     | Framework adapter, requires DOM                                                                                                  |
| `ars-dioxus`       |    —     |    —    |    Y     | Framework adapter, requires DOM                                                                                                  |

#### 1.4.3 WASM Memory and Binary Size Considerations

**Memory growth handling:** In WASM targets, `Vec<T>` and other `alloc` collections grow via `memory.grow`. When the WASM linear memory is exhausted, Rust's global allocator calls `abort()` — there is no recoverable `OutOfMemory` error. Machines MUST enforce capacity limits for dynamically-sized collections:

- `TreeView`, `MenuBar`, and other recursive collection machines MUST document a maximum nesting depth (default: 32 levels). The adapter MUST validate depth before constructing nested machine hierarchies. Exceeding the limit returns a `ValidationError` rather than risking stack overflow.
- Recursive `transition()` calls (e.g., cascading selection in nested trees) MUST use iterative processing via the `Service::drain_queue()` loop, never direct recursion. The existing `MAX_DRAIN_ITERATIONS` limit (100) prevents runaway chains.

**Stack depth limits:** WASM's default stack size is typically 1 MB. Deeply nested component trees can exhaust the call stack during `connect()` chains. Adapters SHOULD use iterative traversal for rendering nested collections rather than recursive component mounting.

**Binary size optimization:** For WASM builds:

- Enable `wasm-opt -Oz` in release profiles.
- Use `#[cfg(target_arch = "wasm32")]` to exclude desktop-only code paths.
- Prefer `ars-core`'s `no_std` + `alloc` configuration to avoid pulling in `std`.
- Enable LTO (`lto = true`) and `codegen-units = 1` in the workspace release profile.
- ICU4X data tables are the largest contributor to binary size; WASM builds SHOULD use the `web-intl` feature (browser `Intl` API) instead of bundling ICU4X data.

### 1.5 Shared Ownership and Callback Constraints

`ars-core` standardizes on `Arc`-backed shared ownership and `Send + Sync` callback
bounds across all targets, including WASM. This keeps the public API identical
between web and native adapters, matches Leptos's context/storage requirements,
and avoids per-target trait-bound drift in component props and provider state.

- **All targets:** shared callback/context resources use `Arc<T>` directly.
- **All targets:** callback and message closure trait objects carry `Send + Sync + 'static`.
- **Practical implication:** closures that capture `Rc<RefCell<T>>` are not valid in
  ars-ui public APIs; use `Arc<Mutex<T>>`, atomics, or framework-owned signal types
  that satisfy the stronger contract.

**The problem in practice:**

A closure capturing `Rc<RefCell<T>>` compiles on WASM but fails on desktop:

```rust
use std::rc::Rc;
use std::cell::RefCell;

let local_state = Rc::new(RefCell::new(0u32));

// This closure compiles on WASM but fails on native targets that
// require Send + Sync on callback types:
let callback = move || {
    *local_state.borrow_mut() += 1;
};
// Error on native: `Rc<RefCell<u32>>` cannot be sent between threads safely
```

**Guidance — cross-platform compatibility:**

For code that must compile on both WASM and native targets, use `Arc<Mutex<T>>` instead of `Rc<RefCell<T>>`:

```rust
use std::sync::{Arc, Mutex};

let shared_state = Arc::new(Mutex::new(0u32));

// Compiles on both WASM and native targets:
let callback = move || {
    *shared_state.lock().expect("lock should not be poisoned") += 1;
};
```

On WASM, `Arc` and `Mutex` compile and function correctly — they simply carry unnecessary atomic overhead. The performance difference is negligible for callback-frequency operations.

**Compile-time assertion pattern:**

To catch `Send` bound violations early (before CI tests on native targets), add a compile-time assertion in adapter crates:

```rust
/// Compile-time assertion that a type satisfies Send + Sync.
/// Place in adapter crate tests or a `cfg(test)` module.
fn assert_send_sync<T: Send + Sync>() {}

#[cfg(test)]
mod platform_safety {
    use super::*;

    #[test]
    fn callback_types_are_send_sync() {
        // Fails at compile time if Callback<T> captures non-Send types
        assert_send_sync::<Callback<()>>();
        assert_send_sync::<Callback<String>>();
    }
}
```

This pattern surfaces platform incompatibilities at compile time on any target, rather than discovering them only when building for desktop.

> **Design note:** `Callback<T>` already uses the project-wide `Arc`
> contract. The compile-time assertion above verifies that closures captured by a
> given adapter actually satisfy the required `Send + Sync` bounds.

**`SharedState<T>` — interior-mutable shared state:**

`SharedState<T>` extends the same `cfg`-gated pattern to interior-mutable state containers (interaction result state, live reactive values). On WASM it wraps `Rc<RefCell<T>>`, on native it wraps `Arc<Mutex<T>>`. Key API:

```rust
use ars_core::SharedState;

let state = SharedState::new(HoverState::NotHovered);
let current = state.get();                 // T: Clone — borrow/lock, clone, release
state.set(HoverState::Hovered);            // replace inner value
state.with(|s| s.is_hovered());            // borrow/lock, call closure, release
let clone = state.clone();                 // shares same allocation
```

Use `SharedState<T>` instead of bare `Rc<RefCell<T>>` in all interaction result types to ensure cross-platform compatibility. `SharedFlag` remains the specialized choice for single-boolean coordination flags.

## 2. State Machine Core (`ars-core`)

### 2.1 The Machine Trait

````rust
#![no_std]
extern crate alloc;  // ars-core is no_std + alloc, not bare no_std

use core::{fmt::Debug, hash::Hash};
// SAFETY/THREADING: Machine instances use `Rc` (not `Arc`) and are NOT thread-safe.
// Each machine instance must remain on its originating thread. For multi-threaded
// applications, create separate machine instances per thread.

// Arc is used for shared callback wrappers on every target.
// Note: native targets always enable the `std` feature, making alloc::sync available.
use alloc::sync::{Arc, Weak};

/// Trait for props that carry a component ID.
/// Required by adapters' `use_machine()` which accesses `props.id` for
/// hydration-safe ID assignment.
pub trait HasId: Sized {
    fn id(&self) -> &str;
    fn with_id(self, id: String) -> Self;
    fn set_id(&mut self, id: String);
}

/// **Note:** Components whose `Props` require default values (e.g., for uncontrolled mode)
/// should add `Default` to their Props derive list. The `HasId` trait does not require
/// `Default`; it is an opt-in per-component.

/// Trait for typed component part enums.
///
/// Every component defines a `Part` enum whose variants correspond 1:1
/// to the named DOM parts in the component's anatomy. The `#[derive(ComponentPart)]`
/// macro (from `ars-derive`) generates the implementation automatically.
///
/// Variants may be unit variants or carry instance-identity data (e.g., an item
/// index or ID). Data fields must implement `Default`; `all()` yields one
/// representative instance per variant with `Default::default()` for each field
/// (following the strum `EnumIter` pattern). The `name()` method ignores field
/// data and returns only the variant name.
///
/// This trait subsumes the former `Anatomy` struct — the Part enum is now the
/// single source of truth for a component's DOM structure.
pub trait ComponentPart: Clone + Debug + PartialEq + Eq + Hash + 'static {
    /// The root part. Every component has one.
    /// The derive macro sets this to the first enum variant, which MUST be `Root`
    /// and MUST be a unit variant.
    const ROOT: Self;

    /// The component's scope name (kebab-case), used for `data-ars-scope`.
    /// Set via `#[scope = "..."]` on the enum.
    fn scope() -> &'static str;

    /// The kebab-case name of this part, used for `data-ars-part`.
    /// Derived from the PascalCase variant name (e.g., `HiddenInput` → `"hidden-input"`).
    /// For data-carrying variants, the field data is ignored.
    fn name(&self) -> &'static str;

    /// One representative instance per variant in declaration order.
    /// Data-carrying variants use `Default::default()` for each field.
    fn all() -> Vec<Self>;

    /// Convenience: returns the `data-ars-scope` and `data-ars-part` attributes for this part.
    /// Default implementation — no need to override.
    fn data_attrs(&self) -> [(HtmlAttr, &'static str); 2] {
        [
            (HtmlAttr::Data("ars-scope"), Self::scope()),
            (HtmlAttr::Data("ars-part"), self.name()),
        ]
    }
}

/// Trait bound for `Machine::Api<'a>`. Enables generic adapter code
/// to access part attributes without knowing the concrete component.
///
/// Each component's `Api<'a>` struct implements this trait with a `Part`
/// enum as the associated type. The `part_attrs` method dispatches to the
/// concrete per-part attribute methods on the Api.
///
/// For data-carrying Part variants, `part_attrs` destructures the variant
/// and forwards the data to the concrete `*_attrs()` method. When called
/// with default-valued instances from `Part::all()`, the returned `AttrMap`
/// contains correct structural attributes (`data-ars-scope`, `data-ars-part`,
/// `role`) but ARIA attributes that depend on real data (e.g., `aria-controls`
/// with a derived ID) use default values.
///
/// ```rust
/// // Generic adapter test example:
/// fn assert_all_parts_have_scope<T: ConnectApi>(api: &T) {
///     for part in T::Part::all() {
///         let attrs = api.part_attrs(part);
///         assert!(attrs.contains(&HtmlAttr::Data("ars-scope")),
///             "part '{}' missing data-ars-scope", part.name());
///     }
/// }
/// ```
pub trait ConnectApi {
    /// The typed part enum for this component.
    type Part: ComponentPart;

    /// Return the attribute map for a given part.
    /// Infallible — the typed Part enum guarantees the part is valid.
    fn part_attrs(&self, part: Self::Part) -> AttrMap;
}

/// Derive macro provided by `ars-derive` crate.
/// Automatically implements `HasId` for structs with a `pub id: String` field.
///
/// ```rust
/// #[derive(Clone, Debug, HasId)]
/// pub struct Props {
///     pub id: String,
///     // ...
/// }
/// ```
///
/// #### `ars-derive` Macro Specification
///
/// **Proc-macro crate:** `ars-derive` (re-exported from `ars-core` via `#[doc(inline)]`).
///
/// **Signature:** `#[proc_macro_derive(HasId)]` — no helper attributes.
///
/// **Requirements:**
/// - The target struct MUST have a field named `id` of type `String`.
/// - The `id` field MUST be `pub` (the generated `fn id(&self)` returns `&self.id`).
/// - The struct MUST be `Sized` (required by `HasId` supertrait bound).
///
/// **Generated code example:**
///
/// ```rust
/// // Input:
/// #[derive(HasId)]
/// pub struct Props { pub id: String, pub label: String }
///
/// // Generated:
/// impl HasId for Props {
///     fn id(&self) -> &str { &self.id }
///     fn with_id(self, id: String) -> Self { Self { id, ..self } }
///     fn set_id(&mut self, id: String) { self.id = id; }
/// }
/// ```
///
/// **Compile-time errors:**
/// - Missing `id` field → `"HasId requires a field named `id` of type String"`
/// - Non-`String` `id` field → `"HasId: `id` field must be of type String"`
/// - Applied to enum or union → `"HasId can only be derived for structs"`

/// Adapter-resolved environment context passed to `Machine::init()`.
///
/// The adapter reads these values from `ArsProvider` / `ArsContext` and passes
/// them to framework-agnostic core code. Core component code **never** calls
/// framework hooks (`use_locale()`, `use_intl_backend()`, `use_context()`) —
/// all environment values arrive through this struct.
///
/// Non-date-time components ignore `intl_backend` (it defaults to `StubIntlBackend`).
pub struct Env {
    /// The resolved locale from `ArsProvider`.
    pub locale: Locale,
    /// Calendar/locale data provider for date-time formatting.
    /// Defaults to `StubIntlBackend` (English-only, zero dependencies).
    pub intl_backend: Arc<dyn IntlBackend>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            locale: Locale::parse("en-US").expect("en-US is a valid BCP-47 tag"),
            intl_backend: Arc::new(StubIntlBackend),
        }
    }
}

/// A finite state machine definition for a UI component.
///
/// Bounds are inlined on associated types — no separate marker traits needed.
pub trait Machine: Sized + 'static {
    type State: Clone + PartialEq + Debug;
    type Event: Clone + Debug;          // PartialEq removed — core engine never compares events; avoids O(n) cost on events like UpdateItems(Vec<..>)
    type Context: Clone + Debug;        // Debug added — enables logging/inspection of context
    type Props: Clone + PartialEq + HasId; // PartialEq required for Dioxus memoization; Clone for reactive prop sync; HasId for adapter ID access
    type Messages: ComponentMessages + Clone + Default + 'static; // Per-component i18n messages type
    type Api<'a>: ConnectApi where Self: 'a; // ConnectApi bound enables generic adapter utilities (e.g., part_attrs() access); `where Self: 'a` required for GAT well-formedness

    /// Initialize the machine from props and adapter-resolved environment values,
    /// returning initial state and context.
    ///
    /// The adapter resolves `env` (locale, ICU provider) and `messages` from
    /// `ArsProvider` context before calling this method. Core code never calls
    /// framework hooks — all environment values arrive as parameters.
    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context);

    /// Pure decision function. Reads state and context immutably, returns a transition plan.
    /// Guards see consistent pre-transition state because context is immutable here.
    fn transition(
        state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>>;

    /// Derive the public API from current state, context, and props.
    /// The `send` callback allows the API to dispatch events.
    ///
    /// **Lifetime note:** Lifetimes are explicit because `Api<'a>` bounds the
    /// return lifetime to the borrows of state, context, props, and send.
    /// Standard lifetime elision does not apply here — the compiler cannot
    /// infer which input lifetime should bind to the GAT output.
    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a>;

    /// Called when props are updated via `set_props()`. Returns events to
    /// enqueue in response to prop changes (e.g., `SetValue` for controlled
    /// Bindable fields). Default implementation returns no events.
    fn on_props_changed(_old: &Self::Props, _new: &Self::Props) -> Vec<Self::Event> {
        Vec::new()
    }
}
````

> **Lifetime note — `Api<'a>` structs are ephemeral.** They borrow `state`, `ctx`, `props`, and `send` from the current scope. Consume the `Api` within the same expression or closure where `connect()` is called. Never store an `Api` across suspension points (`.await`, signal updates). If you need data from an `Api` across frames, extract the values you need into owned types first.
>
> **`ConnectApi` impl requirement.** Every component's `Api<'a>` struct MUST implement the
> `ConnectApi` trait with `type Part` set to the component's Part enum. The `part_attrs()`
> method dispatches to the concrete per-part attribute methods:
>
> ```rust
> impl ConnectApi for Api<'_> {
>     type Part = Part;
>
>     fn part_attrs(&self, part: Self::Part) -> AttrMap {
>         match part {
>             Part::Root => self.root_attrs(),
>             Part::Control => self.control_attrs(),
>             // Data-carrying variants destructure and forward:
>             // Part::Item(ref id) => self.item_attrs(id),
>         }
>     }
> }
> ```
>
> This `impl` block is required because the Machine trait bounds `type Api<'a>: ConnectApi`.
> Without it, the component will not compile.
>
> **Lifetime Safety**: `Api<'a>` borrows from the `send` closure and MUST NOT outlive it. The lifetime `'a` is intentionally non-`'static` to prevent storage in signals or global state.
>
> **Compile-Time Enforcement**: `Api<'a>` does NOT implement `Clone` or `Copy`. Adapters MUST NOT store `Api` in reactive signals, contexts, or any container that outlives the current render cycle.
>
> **Violation**: Storing `Api<'a>` beyond its scope is undefined behavior and will cause use-after-free. If persistent access to machine operations is needed, store the `send: Arc<dyn Fn(Event) + Send + Sync>` closure directly instead.
>
> **Ergonomic persistent access**: For cases where persistent access to machine operations is needed, store `send: Arc<dyn Fn(Event) + Send + Sync>` directly. Do not create ad-hoc wrapper types — the adapter's `use_machine` hook already provides the ergonomic access layer.

Components SHOULD implement `fmt::Display` for their State and Event enums
for debugging, logging, and `data-ars-state` attribute values. The architecture
does not enforce this via trait bound to keep the core minimal. The recommended
format is:

```rust
impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Idle => write!(f, "idle"),
            State::Open => write!(f, "open"),
            // variant name in kebab-case
        }
    }
}
```

This enables `data-ars-state` values and Service debug logging.

> **Naming convention — `Machine` struct vs `ars_core::Machine` trait.** Every component module
> defines a unit struct named `Machine` (e.g., `pub struct Machine;`) and implements the
> `ars_core::Machine` trait on it. Because the local struct and the trait share the same name,
> component modules **do NOT import** `ars_core::Machine` — doing so would shadow the local
> struct. Instead, trait impls use the fully qualified path:
>
> ```rust
> pub mod checkbox {
>     use ars_core::{TransitionPlan, PendingEffect, ConnectApi, AttrMap, /* ... */};
>     //  ^ Note: Machine is deliberately NOT imported from ars_core
>
>     pub struct Machine;  // The component's unit struct
>
>     impl ars_core::Machine for Machine {
>     //   ^^^^^^^^^^^^^^^^^ fully qualified to avoid shadowing `Machine` struct
>         type State = State;
>         type Event = Event;
>         // ...
>     }
> }
> ```
>
> This pattern is used consistently across **all** component specs and foundation machines
> (e.g., `form_submit::Machine`, `fieldset::Machine`, `field::Machine`). Adapter code
> that needs to reference the trait generically (e.g., `use_machine<M: ars_core::Machine>`)
> imports it at the adapter level where no local `Machine` struct exists.
>
> **Adapter import note:** Adapter crates (`ars-leptos`, `ars-dioxus`) use `use ars_core::Machine;` directly, since they do not define a local `Machine` struct. The shadowing concern only applies inside component modules.

### 2.2 TransitionPlan and PendingEffect

> **TransitionPlan Quick Reference**
>
> | Constructor / Method                          | Purpose                                             |
> | --------------------------------------------- | --------------------------------------------------- |
> | `TransitionPlan::to(state)`                   | Transition to a new state                           |
> | `TransitionPlan::new()`                       | Empty plan (no state change); chain builders        |
> | `TransitionPlan::context_only(\|ctx\| {...})` | Mutate context without changing state               |
> | `.apply(\|ctx\| {...})`                       | Add/chain a context mutation                        |
> | `.then(event)`                                | Enqueue a follow-up event                           |
> | `.with_effect(pending_effect)`                | Attach a `PendingEffect`                            |
> | `.with_named_effect(name, setup_fn)`          | Inline `PendingEffect` convenience                  |
> | `.cancel_effect(name)`                        | Cancel a named effect; runs its cleanup immediately |
>
> All methods return `Self` for fluent chaining:
> `TransitionPlan::to(State::Open).apply(|ctx| ctx.count += 1).with_effect(...)`.

````rust
type ApplyFn<M> = dyn FnOnce(&mut <M as Machine>::Context);

/// A transition plan describes what should happen in response to an event.
/// Built using a fluent builder pattern.
pub struct TransitionPlan<M: Machine> {
    /// Target state. `None` means stay in current state (context-only change).
    pub target: Option<M::State>,
    /// Mutation to apply to the context after state change.
    /// Uses `FnOnce` — apply is called exactly once, immediately after transition
    /// selection. Using FnOnce enforces single-invocation semantics.
    /// `pub(crate)` — construct via `.apply()` / `.context_only()` builder methods.
    pub(crate) apply: Option<Box<ApplyFn<M>>>,
    /// Events to enqueue after this transition completes.
    pub then_send: Vec<M::Event>,
    /// Optional human-readable description of the apply closure's purpose.
    /// Used in debug logging and dev-tools inspection. Not evaluated at runtime.
    /// Example: `Some("increment counter")`.
    /// `pub(crate)` — internal diagnostic field, not part of the public API.
    pub(crate) apply_description: Option<&'static str>,
    /// Side effects for the adapter to set up.
    pub effects: Vec<PendingEffect<M>>,
    /// Named effects to cancel (cleanup runs immediately, no replacement).
    pub cancel_effects: Vec<&'static str>,
}

impl<M: Machine> TransitionPlan<M> {
    /// Create a plan that transitions to a new state.
    ///
    /// Note: `Vec::new()` does not allocate until the first element is pushed.
    /// This is a Rust guarantee (`Vec::new()` creates a zero-capacity vector with
    /// no heap allocation), so `then_send` and `effects` are free when unused.
    pub const fn to(state: M::State) -> Self {
        Self {
            target: Some(state),
            apply: None,
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }

    /// Create an empty plan with no state change.
    /// Useful as a builder starting point — chain `.apply()`, `.then_send()`,
    /// and `.with_effect()` to configure. Equivalent to `context_only` but
    /// without requiring an initial closure.
    pub const fn new() -> Self {
        Self {
            target: None,
            apply: None,
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }

    /// Add a context mutation to this plan.
    /// If a previous mutation was already set (e.g., from `context_only()`),
    /// the new closure is chained after it — both run in order.
    ///
    /// **Design note:** `apply` mutates `Context` only — it cannot modify `Props`.
    /// Props are owned by the adapter and flow inward via `set_props()`. This
    /// ensures that the adapter remains the single source of truth for prop values.
    pub fn apply(mut self, f: impl FnOnce(&mut M::Context) + 'static) -> Self {
        self.apply = match self.apply {
            Some(prev) => Some(Box::new(move |ctx: &mut M::Context| {
                prev(ctx);
                f(ctx);
            })),
            None => Some(Box::new(f)),
        };
        self
    }

    /// Enqueue a follow-up event after this transition.
    pub fn then(mut self, event: M::Event) -> Self {
        self.then_send.push(event);
        self
    }

    // **Design note on conditional follow-up events:** `then_send` is intentionally
    // a static `Vec<Event>` to keep the core deterministic and testable. For conditional
    // follow-up logic, use guards in subsequent transitions rather than conditional
    // follow-up events. Complex conditional logic belongs in adapter-level event handlers
    // or in the machine's own `transition()` function responding to intermediate states.

    /// Attach a side effect for the adapter to manage.
    pub fn with_effect(mut self, effect: PendingEffect<M>) -> Self {
        self.effects.push(effect);
        self
    }

    /// Convenience: build a PendingEffect inline from a name and closure.
    pub fn with_named_effect(
        self,
        name: &'static str,
        setup: impl FnOnce(&M::Context, &M::Props, WeakSend<M::Event>) -> CleanupFn + 'static,
    ) -> Self {
        self.with_effect(PendingEffect::new(name, setup))
    }

    /// Cancel a named effect without replacement.
    ///
    /// The effect's cleanup closure runs immediately when the `Service`
    /// processes this plan. No-op if no effect with `name` is currently active.
    ///
    /// Named effects are also auto-cancelled when a new effect with the same
    /// name is registered via `with_named_effect`. Use `cancel_effect` for
    /// explicit cancellation without replacement (e.g., cancelling a timer
    /// when the component transitions to a state that no longer needs it).
    pub fn cancel_effect(mut self, name: &'static str) -> Self {
        self.cancel_effects.push(name);
        self
    }

    /// Create a plan that only mutates context without changing state.
    pub fn context_only(f: impl FnOnce(&mut M::Context) + 'static) -> Self {
        // NOTE: context_only plans are useful for updating internal values
        // (e.g., Bindable fields) without triggering a state transition.
        Self {
            target: None,
            apply: Some(Box::new(f)),
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }
}

// **Closure ownership rule:** Closures in `TransitionPlan` (both `apply` and effect
// `setup`) must not capture references from transition arguments (`state`, `event`,
// `ctx`, `props`). These borrows do not live long enough — `apply` is `'static`.
// Store only owned values: IDs (`String`), cloned enums, indices (`usize`).
// The compiler enforces this via lifetime errors, but the API intent should be
// explicit: transition closures capture *owned snapshots*, never borrows.

// **Chained `apply()` capture constraint:** Each closure's captures must be
// independent. Closures in a chain must not read values captured by a previous
// closure because `prev` is consumed as `FnOnce`. If two closures need the
// same data, clone it before the first `apply` so each closure owns its copy:
//
// **Closure capture optimization guidance for `apply()`:**
//
// 1. **Carry large data via the Event, not via capture.** Move large payloads
//    into the Event variant so they are passed as arguments to `apply()` rather
//    than captured by the closure. This avoids cloning large structs into every
//    chained closure.
//
//    ```rust
//    // PREFER: large data moved into Event
//    Event::UpdateItems(items)  // items: Vec<Item> moved here
//    // then in transition: plan.apply(move |ctx| ctx.items = items)
//
//    // AVOID: large data captured by reference (lifetime issues) or cloned
//    // let items = items.clone();  // expensive clone into closure
//    ```
//
// 2. **Use `Box` for heap allocation when capturing large structs.** If a closure
//    must capture a large value, box it first to avoid copying it onto the stack
//    during closure construction:
//
//    ```rust
//    let big_data = Box::new(large_struct);
//    plan.apply(move |ctx| ctx.data = *big_data)
//    ```
//
// 3. **Prefer cloning small values over capturing references.** Clone cheap values
//    (IDs, indices, small enums) rather than capturing `&ctx` references, which
//    cause lifetime errors because `apply` closures must be `'static`.
//
// ```rust
// let shared = ctx.some_value.clone();       // clone before first apply
// let c1_val = shared.clone();               // owned by closure 1
// let c2_val = shared;                       // owned by closure 2
// plan = plan.apply(move |ctx| { /* use c1_val */ })
//            .apply(move |ctx| { /* use c2_val */ });
// ```
//
// This "clone-before-first-apply" pattern ensures each `FnOnce` closure in the
// chain captures only owned data that is safe to move, avoiding use-after-move
// errors when closures are consumed sequentially.

// `TransitionPlan` provides a custom `Debug` impl that prints closure fields as
// `"<closure>"` since closures do not implement `Debug`:

impl<M: Machine> fmt::Debug for TransitionPlan<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitionPlan")
            .field("target", &self.target)
            .field("apply", &if self.apply.is_some() { "<closure>" } else { "None" })
            .field("apply_description", &self.apply_description)
            .field("then_send", &self.then_send)
            .field("effects", &self.effects.iter().map(|e| e.name).collect::<Vec<_>>())
            .finish()
    }
}

// In debug builds, `apply_description` is included in transition logs to aid
// debugging type-erased closures. When `cfg!(debug_assertions)` is true, the
// adapter's transition logging includes the description string alongside the
// target state and effect names. In release builds the field is still present
// but adapters MAY omit it from logs for compactness.

impl<M: Machine> TransitionPlan<M> {
    /// Returns a short string label for logging/debugging without exposing closures.
    pub const fn debug_summary(&self) -> &'static str {
        match (self.target.is_some(), self.apply.is_some()) {
            (true, _) => "to",
            (false, true) => "context_only",
            (false, false) => "none",
        }
    }
}

/// A side effect to be set up by the adapter after a transition.
/// The setup function receives context, props, and a `WeakSend<M::Event>` callback
/// (the adapter holds the strong `Arc` internally; effects receive the weak handle).
/// Returns a cleanup function invoked when the effect must stop (state change or unmount).
///
/// **Memory retention warning:** The strong `send: Arc<dyn Fn(M::Event) + Send + Sync>` passed to effect
/// setup closures holds a strong reference to the machine's event dispatch pipeline.
/// If an effect captures `send` and lives longer than the component (e.g., a global
/// event listener or a long-lived timer), the `Arc` prevents deallocation.
///
/// **Best practice for long-lived effects:** Downgrade `send` to a `Weak` reference
/// and attempt upgrade on each use:
///
/// ```rust
/// PendingEffect::new("polling_timer", move |_ctx, _props, send| {
///     let handle = set_interval(move || {
///         send.call_if_alive(MyEvent::Poll);
///         // If the component has been unmounted, call_if_alive is a no-op.
///     }, Duration::from_secs(30));
///     Box::new(move || clear_interval(handle))
/// })
/// ```
///
/// Short-lived effects (e.g., focus after transition) that complete within the
/// cleanup lifecycle do not need `Weak` — the cleanup function drops `send` promptly.
///
/// **Cleanup closure limitation:** Cleanup closures MUST NOT depend on context state.
/// Extract IDs and resources during setup; cleanup receives no context argument. If
/// context changes between setup/cleanup, cleanup runs with stale captures.
///
/// **Effect type erasure and follow-up event safety:** Effects use `Box<dyn ...>`,
/// which erases the concrete type. Effects that emit follow-up events via the `send`
/// callback must be exhaustively tested to ensure all possible follow-up events are
/// handled by the machine's `transition()` function. Document all possible follow-up
/// events in the machine's effect protocol (a comment block listing effect name →
/// possible events). Adapters must handle unknown follow-up events gracefully (log a
/// warning, do not panic).
///
/// Type alias for the cleanup function returned by effect setup.
///
/// Two allocations: the outer `Box` erases the closure's concrete type for storage
/// in a heterogeneous effect-cleanup list; the inner `dyn FnOnce()` allows each
/// effect to capture arbitrary owned state for teardown (event listener handles,
/// observer references, timer IDs, etc.).
pub type CleanupFn = Box<dyn FnOnce()>;

/// No-op cleanup for effects that don't need teardown.
#[inline]
#[must_use]
pub fn no_cleanup() -> CleanupFn {
    Box::new(|| {})
}

/// Shared callback wrapper for event handler closures in Props structs.
/// Wraps closures in `Arc<T>` on every target, so
/// `Clone`, `PartialEq`, `Deref`, and `AsRef` need no cfg-gated code.
/// This is distinct from `MessageFn<T>` (used for i18n message closures)
/// and `CleanupFn` (used for effect cleanup).
pub struct Callback<T: ?Sized>(pub(crate) Arc<T>);

impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self { Callback(Arc::clone(&self.0)) }
}

impl<T: ?Sized> core::fmt::Debug for Callback<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Callback(..)")
    }
}

// PartialEq by pointer identity — enables derive(PartialEq) on Props structs.
impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) }
}

// Deref and AsRef impls for ergonomic invocation — avoids verbose `.0` access.
impl<T: ?Sized> core::ops::Deref for Callback<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}

impl<T: ?Sized> AsRef<T> for Callback<T> {
    fn as_ref(&self) -> &T { self.0.as_ref() }
}

/// Callback supports an optional return type via `Callback<dyn Fn(Args) -> Out + Send + Sync>`.
/// When the return type is `()` (the default), you can write `Callback<dyn Fn(Args) + Send + Sync>`
/// as shorthand — `Fn(Args)` is sugar for `Fn(Args) -> ()` in Rust.
///
/// This mirrors the Leptos `Callback<In, Out = ()>` and Dioxus `Callback<Args, Ret = ()>`
/// patterns, enabling callbacks that return values (e.g., adapter-provided async spawners).
///
/// Constructors and `From` impls use raw `Arc` construction for dyn trait object
/// coercion (`Arc` lacks `CoerceUnsized`).
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out + Send + Sync> {
    pub fn new(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Self {
        Self(alloc::sync::Arc::new(f))
    }
}

/// Constructor for zero-argument `Callback<dyn Fn() + Send + Sync>`.
///
/// `dyn Fn()` and `dyn Fn(Args) -> Out` are distinct trait objects in Rust,
/// so the generic `Callback::new` (which requires one `Args` parameter)
/// cannot produce `Callback<dyn Fn() + Send + Sync>`. This fills that gap for
/// void callbacks (e.g. `on_dismiss`, `on_escape_key_down`).
impl Callback<dyn Fn() + Send + Sync> {
    pub fn new_void(f: impl Fn() + Send + Sync + 'static) -> Self {
        Self(alloc::sync::Arc::new(f))
    }
}

// From impls for ergonomic construction
impl<F: Fn(Args) -> Out + Send + Sync + 'static, Args: 'static, Out: 'static> From<F> for Callback<dyn Fn(Args) -> Out + Send + Sync> {
    fn from(f: F) -> Self { Callback(alloc::sync::Arc::new(f)) }
}

// From impls for zero-argument closures (`dyn Fn()`)
impl<F: Fn() + Send + Sync + 'static> From<F> for Callback<dyn Fn() + Send + Sync> {
    fn from(f: F) -> Self { Callback(alloc::sync::Arc::new(f)) }
}

// Usage examples:
//   let cb = callback(|s: String| log::info!("{s}"));  // Preferred: free function
//   let cb2: Callback<dyn Fn(String) + Send + Sync> = Callback::new(|s: String| log::info!("{s}")); // Also valid
//   cb("hello".into());   // Deref makes this work directly
//   (&*cb)("hello");      // Equivalent explicit deref
//   cb.as_ref()("hello"); // AsRef alternative
````

#### 2.2.1 Free Function Constructor

A free function `callback()` is provided for ergonomic construction with better type inference. The compiler can infer `Args` from the closure signature without requiring turbofish syntax:

```rust
/// Ergonomic constructor for `Callback<dyn Fn(Args) -> Out + Send + Sync>`.
/// Avoids the turbofish syntax required by `Callback::new()` in generic contexts.
/// For void-return callbacks, `Out` infers to `()` automatically.
pub fn callback<Args: 'static, Out: 'static>(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Callback<dyn Fn(Args) -> Out + Send + Sync> {
    Callback::new(f)
}
```

**Preferred usage:**

```rust
// Recommended — type inference works naturally:
let cb = callback(|s: String| log::info!("{s}"));

// Also valid for closures with multiple / tuple args:
let cb2 = callback(|(x, y): (f64, f64)| log::info!("{x},{y}"));

// In Props construction:
MyProps {
    id: "field-1".into(),
    on_change: Some(callback(|value: String| update_model(value))),
    on_submit: Some(callback(|_: ()| submit_form())),
}
```

#### 2.2.2 Type Inference Note

When `callback()` is not suitable (e.g., when constructing inside a trait impl or macro), the turbofish syntax on `Callback::new()` remains available:

```rust
// Turbofish resolves ambiguity when the compiler cannot infer the closure signature:
let cb = Callback::<dyn Fn(String)>::new(|s| log::info!("{s}"));
```

#### 2.2.3 Inner Field Privacy

Both `Callback<T>` and `MessageFn<T>` wrap `Arc<T>` internally. The `Arc` field is `pub(crate)` to prevent external code from depending on the concrete smart pointer type (`Arc`). This ensures:

1. **Cross-platform safety**: Generic code depends on a single ownership contract instead of duplicating `Rc`/`Arc` assumptions.
2. **Encapsulation**: The smart pointer type is an implementation detail. External consumers interact through `Deref`-based invocation (`cb(value)`) and `Callback::new()` / `MessageFn::new()` only.
3. **Reduced cfg duplication**: `Clone`, `PartialEq`, `Deref`, `AsRef`, constructors, and `From` impls all delegate to a single `Arc`-based implementation.

**Trait-bound implication**: Code that is generic over `Callback` or `MessageFn` should assume the stronger `Send + Sync + 'static` contract on every target.

````rust
/// ## MessageFn Definition
///
/// `MessageFn<T>` wraps i18n message closures — functions that produce localized strings.
/// Defined here for completeness; see `04-internationalization.md` for full usage.
///
/// ```rust
/// /// Wrapper for i18n message closures. Clones the smart pointer, not the closure.
/// pub struct MessageFn<T: ?Sized>(pub(crate) Arc<T>);
///
/// impl<T: ?Sized> Clone for MessageFn<T> {
///     fn clone(&self) -> Self { Self(self.0.clone()) }
/// }
///```
///
/// **Usage in Messages structs:**
///
/// ```rust
/// /// Button messages.
/// pub struct Messages {
///     pub label: MessageFn<dyn Fn() -> String + Send + Sync>,
///     pub count: MessageFn<dyn Fn(usize) -> String + Send + Sync>,
/// }
///```
````

#### 2.2.4 `Callback` vs `MessageFn` Naming Convention

The naming asymmetry between `Callback<T>` and `MessageFn<T>` is intentional:

- **`Callback<T>`** is the general-purpose event callback type used throughout component APIs for event handlers (e.g., `on_change: Callback<String>`).
- **`MessageFn<T>`** is a specialized closure type used exclusively for i18n message formatting closures, where the ergonomics differ (closures accept locale context and return formatted strings). See `04-internationalization.md` §7.1 for the `MessageFn` definition and its relationship to `ArsProvider`.

New code should prefer `Callback<T>` for all event-handling use cases.

<!-- Removed: SmartCallback<T> was unusable because T is unsized (dyn Fn(...))
     and `call()` was undefined on Callback (which uses Deref to the inner Fn trait).
     Adapters should invoke callbacks directly via Deref: `(cb)(value)` or `cb(value)`.
     Generic code that must accept either Callback or MessageFn should use a simple
     conversion: both types Deref to the inner closure, so `&*cb` yields `&dyn Fn(...)`. -->

Code that previously used `impl SmartCallback<T>` bounds should instead accept `&dyn Fn(Args)` directly, or be generic over `F: Fn(Args)`. Both `Callback<dyn Fn(Args) + Send + Sync>` and `MessageFn<dyn Fn(Args)>` implement `Deref` to their inner closure, so callers can pass `&*callback` or `&*message_fn` to any function expecting a borrowed closure reference.

> **Default English Messages:** Each component module MUST provide a `default()` impl on its `XyzMessages` struct that returns fallback English messages. Adapters override these defaults via Props fields or context injection (e.g., `ArsProvider`). This ensures components render meaningful text even when no i18n configuration is present. See `04-internationalization.md` for the full message override chain.

````rust
/// ## Callback Type Taxonomy
///
/// The library uses four distinct closure/callback wrapper types:
///
/// | Type | Defined In | Purpose | Example |
/// |------|-----------|---------|---------|
/// | `MessageFn<T>` | `ars-core` (callback.rs) | i18n translatable string closures | `MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>` |
/// | `Callback<T>` | here (`01-architecture.md`) | Event/format callbacks in Props | `Callback<dyn Fn(String) + Send + Sync>`, `Callback<dyn Fn(f64) -> String + Send + Sync>` |
/// | `CleanupFn` | here (`01-architecture.md`) | Effect cleanup (returned by setup) | `Box<dyn FnOnce()>` |
/// | `&dyn Fn(Event)` | connect()/Api structs | Borrowed send callback in connect | `send: &'a dyn Fn(M::Event)` |
///
/// **Do NOT use raw `Arc<dyn Fn(...)>` in Props structs** — always use `Callback<dyn Fn(...) + Send + Sync>`.
/// This ensures correct cfg-gating between `Arc` (native) and `Rc` (WASM).
///
/// **Cfg-gated Props example:** When writing generic Props that contain `Callback` fields,
/// the Props trait bounds must respect the cfg-gated inner pointer type:
///
/// ```rust
/// #[derive(Clone, HasId)]
/// pub struct MyProps {
///     pub id: String,
///     pub on_change: Option<Callback<dyn Fn(String) + Send + Sync>>,
///     pub on_submit: Option<Callback<dyn Fn(())>>,
/// }
///
/// // PartialEq can be derived because Callback implements PartialEq via pointer identity.
/// // This manual impl is shown for illustration.
/// impl PartialEq for MyProps {
///     fn eq(&self, other: &Self) -> bool {
///         self.id == other.id
///             // Callback fields are compared by pointer identity:
///             // same Rc/Arc instance = equal. This is sufficient for
///             // Dioxus memoization (avoids re-render when same closure is passed).
///             && self.on_change == other.on_change
///             && self.on_submit == other.on_submit
///     }
/// }
///```
///
/// **Key point for generic callers:** The inner pointer type (`Rc` vs `Arc`) is
/// transparent to consumers. `Callback::new()` and `From` impls handle construction.
/// Never pattern-match on `Callback.0` directly — use `Deref` or `AsRef` to invoke.
///
/// #### Callback PartialEq and Memoization
///
/// `Callback<T>` equality is **identity-based** (pointer comparison). Two `Callback`
/// instances wrapping identical closures but constructed separately are NOT equal —
/// only the same `Rc`/`Arc` instance compares as equal. This has implications for
/// framework memoization:
///
/// - **Dioxus:** Props containing `Callback` fields will fail `PartialEq` on every
/// render if the callback is reconstructed inline. This defeats Dioxus's memoization
/// and triggers unnecessary re-renders.
/// - **Leptos:** Signal-based reactivity is not affected by `PartialEq` on Props, but
/// unnecessary callback reconstruction still creates allocation pressure.
///
/// **Guidance for adapter authors:**
///
/// 1. **Stabilize callback references.** Construct `Callback` instances once and
/// reuse them across renders. In Leptos, store in a `StoredValue`; in Dioxus,
/// use `use_hook` to allocate once.
/// 2. **Extract callbacks outside memo boundaries.** When wrapping components in
/// `memo()` or `#[component(no_memo)]`, pass callbacks as stable references
/// rather than inline closures.
/// 3. **Manual `PartialEq`:** Props structs with `Callback` fields MUST implement
/// `PartialEq` manually (as shown above). Callback fields should compare by
/// pointer identity (`Rc::ptr_eq` / `Arc::ptr_eq`) or be skipped entirely.
///
/// ```rust
/// // WRONG — new Callback allocation every render, breaks memoization
/// let props = MyProps {
///     id: "btn-1".into(),
///     on_change: Some(Callback::new(|s: String| log::info!("{s}"))),
/// };
///
/// // RIGHT — stable reference, same Rc/Arc across renders
/// let on_change = use_hook(|| Callback::new(|s: String| log::info!("{s}")));
/// let props = MyProps {
///     id: "btn-1".into(),
///     on_change: Some(on_change.clone()),
/// };
///```
///
/// #### Callback Usage in Library Crates
///
/// `Callback<T>` always wraps `Arc<T>` and requires `Send + Sync + 'static`
/// captures on every target. That keeps the public API identical across web
/// and native builds.
///
/// **Impact for library authors:** Props containing `Callback` fields can be
/// used in generic code with consistent thread-safety bounds on every target.
///
/// **Recommendation:** Keep the stronger bounds explicit when exposing
/// `Callback`-bearing types across crate boundaries:
///
/// ```rust
/// // In your library crate — define a newtype around the callback.
/// pub struct OnChange(pub Callback<dyn Fn(String) + Send + Sync>);
///
/// // It is valid to require Send + Sync when the captured state satisfies it.
///```
///
/// This preserves a single ownership contract instead of carrying separate
/// wasm/native callback semantics through downstream crates.
````

#### 2.2.5 Standard Debug Impl for Messages Structs

All `XyzMessages` structs contain closures (`MessageFn<dyn Fn(...)>`) that do not implement
`Debug`. Every Messages struct MUST provide a manual `Debug` impl following this pattern:

```rust
impl Debug for XyzMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XyzMessages {{ .. }}")
    }
}
```

For structs with a mix of plain fields and closures, prefer `debug_struct` with closure fields
printed as `"<closure>"` (see `Messages` in `07-forms.md §14.7` for an example).

```rust
pub struct PendingEffect<M: Machine> {
    pub name: &'static str,
    /// The state after the transition that produced this effect. Effects from
    /// intermediate states in a multi-step drain_queue() cycle receive the
    /// FINAL context snapshot, not the intermediate one. Machines must design
    /// effect setup closures to be correct regardless of subsequent context
    /// mutations within the same drain cycle.
    pub target_state: Option<M::State>,
    /// **Internal vs user-facing API:** The `setup` field accepts a strong
    /// `Arc` send handle internally. `PendingEffect::new()` wraps user
    /// closures to receive `WeakSend` instead, preventing retain cycles.
    /// See §2.2 (WeakSend) for the full bridging explanation.
    ///
    /// Consumers MUST use `PendingEffect::new()` to construct
    /// effects — never build this field directly.
    /// Implementation note: these fields are `pub(crate)` to prevent external
    /// struct literal construction. Use `PendingEffect::run()` to invoke the setup closure
    /// from adapter crates.
    pub(crate) setup: Box<dyn FnOnce(&M::Context, &M::Props, Arc<dyn Fn(M::Event) + Send + Sync>) -> CleanupFn>,
}

impl<M: Machine> fmt::Debug for PendingEffect<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingEffect")
            .field("name", &self.name)
            .field("target_state", &self.target_state)
            .field("setup", &"<closure>")
            .finish()
    }
}

impl<M: Machine> PendingEffect<M> {
    /// Run the effect setup closure. This is the public API for adapter crates
    /// to invoke effects, since the `setup` field is `pub(crate)`.
    pub fn run(self, ctx: &M::Context, props: &M::Props, send: Arc<dyn Fn(M::Event) + Send + Sync>) -> CleanupFn {
        (self.setup)(ctx, props, send)
    }
}
```

> **Why `Box<dyn FnOnce>`?** Effects are one-shot — they run exactly once per
> transition. `FnOnce` is intentional: it allows the setup closure to _move_
> captured data out of itself during execution, which is both more efficient
> (no unnecessary cloning) and more correct (the type system prevents
> accidental double-execution). A `Fn` or `FnMut` bound would force captured
> values to remain valid after setup runs, adding overhead and weaker
> guarantees for something that must happen exactly once.
>
> **Arc cycle prevention:** Cleanup closures returned by `PendingEffect::setup` MUST NOT capture the strong `send: Arc<dyn Fn(M::Event) + Send + Sync>` callback. If they do and the adapter stores both the cleanup list and the send callback in mutually-referencing structures, a reference cycle forms and neither is dropped. If cleanup needs to dispatch events, use a `Weak` reference instead.

#### 2.2.6 Closure Capture Guidelines

Rc cycles form when a cleanup closure captures the `send` callback by cloning the `Rc`:

```rust
// ❌ INCORRECT — holding a strong reference in the cleanup prevents cleanup from ever running
PendingEffect::new("auto-dismiss", |_ctx, _props, send| {
    let timer = set_timeout(move || {
        send.call_if_alive(Event::Dismiss); // ✅ send is WeakSend — safe
    });
    // ❌ Problem: timer closure captures `send` (WeakSend) — that's fine,
    // but if it captured a strong Rc instead, the cleanup would never run.
    Box::new(move || clear_timeout(timer))
})
```

The `send` parameter is a `WeakSend<M::Event>` — a weak reference that does not keep the
service alive. Use `send.call_if_alive(event)` to dispatch events safely:

```rust
// ✅ CORRECT — WeakSend is already weak, so no cycle is possible
PendingEffect::new("auto-dismiss", |_ctx, _props, send| {
    let timer = set_timeout(move || {
        send.call_if_alive(Event::Dismiss);
    });
    Box::new(move || clear_timeout(timer))
})
```

**Rule:** The `send` parameter in `PendingEffect::new()` is a `WeakSend<M::Event>`. Always use `send.call_if_alive(event)` — never upgrade or clone into a strong reference.

##### 2.2.6.1 WeakSend Newtype for Safe Effect Cleanup

`WeakSend<T>` must only be constructed from a weak reference (`Weak<T>` or equivalent), never from an owned `Arc<T>`. The `PendingEffect::new` constructor enforces this at the type level by accepting a weak reference instead of the strong `Arc`. This prevents accidental strong-reference cycles where an effect holds a strong reference to the component that owns it, blocking cleanup.

If a consumer needs to capture owned state, they must first wrap it in `Arc<T>`, store the `Arc`, then pass `Arc::downgrade(&arc)` to `PendingEffect::new`.

```rust
/// A weak reference to a send function that avoids Arc cycles in cleanup closures.
/// Cleanup closures MUST capture `WeakSend<M::Event>` instead of `Arc<dyn Fn(M::Event) + Send + Sync>`.
pub struct WeakSend<T>(alloc::sync::Weak<dyn Fn(T) + Send + Sync>);

impl<T> WeakSend<T> {
    /// Attempt to upgrade the weak reference. Returns `None` if the strong reference has been dropped.
    pub fn upgrade(&self) -> Option<Arc<dyn Fn(T) + Send + Sync>> {
        self.0.upgrade()
    }

    /// Call the function if the strong reference is still alive. No-op otherwise.
    pub fn call_if_alive(&self, value: T) {
        if let Some(f) = self.0.upgrade() {
            f(value);
        }
    }
}

impl<T> Clone for WeakSend<T> {
    fn clone(&self) -> Self {
        WeakSend(self.0.clone())
    }
}

impl<T> From<&Arc<dyn Fn(T) + Send + Sync>> for WeakSend<T> {
    fn from(arc: &Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(Arc::downgrade(arc))
    }
}

/// Convenience: create a WeakSend directly from an owned Arc without
/// requiring the caller to manually call Arc::downgrade.
impl<T> WeakSend<T> {
    /// Create a WeakSend by downgrading the given Arc.
    /// Equivalent to `WeakSend::from(&arc)` but more discoverable.
    pub fn from_arc(arc: &Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(Arc::downgrade(arc))
    }

    /// Downgrade an Arc into a WeakSend in one step.
    /// Alias for `from_arc` — mirrors the `Arc::downgrade` naming convention.
    pub fn downgrade(arc: &Arc<dyn Fn(T) + Send + Sync>) -> Self {
        Self::from_arc(arc)
    }
}
```

These convenience constructors complement the `From<&Arc<...>>` impl. `WeakSend::from_arc(&send)` and `WeakSend::downgrade(&send)` are more discoverable than the `From` trait conversion, reducing the likelihood that callers will accidentally capture a strong `Arc` instead of creating a weak reference.

User closures passed to `PendingEffect::new()` MUST accept `send: WeakSend<M::Event>` rather than `Arc<dyn Fn(M::Event) + Send + Sync>`. (The internal `setup` struct field uses a strong `Arc` — this is an implementation detail bridged by `PendingEffect::new()` which downgrades before forwarding.) This ensures cleanup closures cannot hold strong references to the send function, eliminating the most common source of memory leaks in effect lifecycles.

**Mandate**: Adapters pass the strong `Arc` to the `setup` field closure. `PendingEffect::new()` internally downgrades to `WeakSend` before forwarding to the user-authored setup closure. User-authored setup closures receive `WeakSend<M::Event>` and MUST NOT capture the original strong `Arc`. Any cleanup closure that holds a strong reference to the send function is a specification violation.

`PendingEffect::setup` closures that need to dispatch events during cleanup MUST accept `WeakSend<M::Event>` instead of `Arc<dyn Fn(M::Event) + Send + Sync>`:

```rust
// ✅ Type-safe cycle prevention
PendingEffect::new("auto-dismiss", |_ctx, _props, send| {
    // `send` is already `WeakSend<M::Event>` — use directly in cleanup
    let timer = set_timeout(move || { send.call_if_alive(Event::Dismiss); });
    Box::new(move || clear_timeout(timer))
})
```

````rust
impl<M: Machine> PendingEffect<M> {
    /// Create a new pending effect with a name and setup function.
    ///
    /// # Example
    /// ```rust
    /// PendingEffect::new("focus-management", |ctx, _props, _send| {
    ///     let content_id = ctx.content_id.clone();
    ///     focus_first_tabbable(&content_id);
    ///     Box::new(move || restore_focus(&content_id))
    /// })
    /// ```
    pub fn new(
        name: &'static str,
        setup: impl FnOnce(&M::Context, &M::Props, WeakSend<M::Event>) -> CleanupFn + 'static,
    ) -> Self {
        Self { name, target_state: None, setup: Box::new(move |ctx, props, send| {
            let weak_send = WeakSend::from(&send);
            setup(ctx, props, weak_send)
        }) }
    }
}
````

#### 2.2.7 PlatformEffects Trait

Effect closures must be **platform-agnostic** — they MUST NOT call DOM APIs (`web_sys`, `ars_dom`) directly. Instead, all platform-specific operations (focus, timers, announcements, positioning, scroll lock) go through the `PlatformEffects` trait, resolved from the adapter's framework context.

Ambient input modality tracking is a separate concern. It does **not** belong on `PlatformEffects` because it is shared provider state rather than a machine-triggered side-effect surface. The shared `ModalityContext` lives in `ars-core`, is injected through `ArsProvider`, and is fed by platform-specific listener implementations such as `ars-dom::ModalityManager`.

```rust
// ars-core/src/platform.rs

/// Platform-agnostic interface for side effects triggered by PendingEffect closures.
///
/// Each adapter provides an implementation:
/// - `ars-dom` provides `WebPlatformEffects` (web targets via web_sys)
/// - Native adapters provide their own (e.g., AccessKit for accessibility)
///
/// Components resolve this via `use_platform_effects()` inside effect closures.
pub trait PlatformEffects {
    // ── Focus ───────────────────────────────────────────────────────
    /// Focus the element with the given ID. No-op if not found.
    fn focus_element_by_id(&self, id: &str);
    /// Focus the first tabbable element inside a container. No-op if not found.
    fn focus_first_tabbable(&self, container_id: &str);
    /// Focus the last tabbable element inside a container. No-op if not found.
    fn focus_last_tabbable(&self, container_id: &str);
    /// Return IDs of all tabbable elements inside a container in sequential tab order.
    ///
    /// Positive `tabindex` values sort before the natural tab sequence, with DOM
    /// order breaking ties.
    fn tabbable_element_ids(&self, container_id: &str) -> Vec<String>;
    /// Focus the document body (last-resort fallback).
    fn focus_body(&self);

    // ── Timers ──────────────────────────────────────────────────────
    /// Schedule a callback after `delay_ms` milliseconds. Returns a handle for cancellation.
    fn set_timeout(&self, delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle;
    /// Cancel a previously scheduled timeout.
    fn clear_timeout(&self, handle: TimerHandle);

    // ── Announcements ───────────────────────────────────────────────
    /// Announce a message to assistive technology with polite priority.
    fn announce(&self, message: &str);
    /// Announce a message with assertive priority (interrupts current speech).
    fn announce_assertive(&self, message: &str);

    // ── Positioning ─────────────────────────────────────────────────
    /// Position an element at absolute (x, y) coordinates.
    fn position_element_at(&self, id: &str, x: f64, y: f64);
    /// Resolve the computed text direction of an element. Returns `Ltr` or `Rtl`.
    fn resolved_direction(&self, id: &str) -> ResolvedDirection;

    // ── Modal / Inert ───────────────────────────────────────────────
    /// Set `inert` on all siblings of the portal root. Returns a cleanup function
    /// that restores the original state.
    fn set_background_inert(&self, portal_root_id: &str) -> Box<dyn FnOnce()>;
    /// Best-effort direct inert clearing for siblings of the given element.
    /// This is not a substitute for the cleanup closure returned by
    /// `set_background_inert()`, which remains the authoritative path for
    /// restoring any polyfill state such as `tabindex` values or listeners.
    fn remove_inert_from_siblings(&self, portal_id: &str);
    /// Lock body scroll (prevent background scrolling under modals).
    fn scroll_lock_acquire(&self);
    /// Unlock body scroll.
    fn scroll_lock_release(&self);

    // ── DOM queries ─────────────────────────────────────────────────
    /// Check whether an element with the given ID exists in the document.
    fn document_contains_id(&self, id: &str) -> bool;

    // ── Pointer tracking ────────────────────────────────────────────
    /// Track global pointer events during a drag operation (color sliders,
    /// signature pad, image cropper, etc.). Attaches `pointermove` and `pointerup`
    /// listeners at the document/window level so the drag continues even when the
    /// pointer leaves the originating element.
    ///
    /// Returns a cleanup function that removes both listeners.
    fn track_pointer_drag(
        &self,
        on_move: Box<dyn Fn(f64, f64)>,
        on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()>;

    // ── Focus scope / focus management ──────────────────────────────
    /// Return the ID of the currently focused element, or `None` if nothing is focused.
    fn active_element_id(&self) -> Option<String>;
    /// Attach a focus trap to a container element so Tab/Shift+Tab cycles within it.
    /// Returns a cleanup function that removes the trap listeners.
    fn attach_focus_trap(&self, container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()>;
    /// Check whether focus can be safely restored to the element with the given ID
    /// (element exists, is visible, is focusable, and has layout).
    fn can_restore_focus(&self, id: &str) -> bool;
    /// Find the nearest focusable ancestor of the element with the given ID.
    /// Returns the ancestor's ID, or `None`.
    fn nearest_focusable_ancestor_id(&self, id: &str) -> Option<String>;

    // ── Scroll ───────────────────────────────────────────────────────
    /// Set the vertical scroll position of a container element.
    fn set_scroll_top(&self, container_id: &str, scroll_top: f64);

    // ── Element measurement ─────────────────────────────────────────
    /// Resize an element to fit its content (used by textarea auto-resize).
    /// `max_height` is an optional CSS max-height value.
    fn resize_to_content(&self, id: &str, max_height: Option<&str>);

    // ── Platform queries ────────────────────────────────────────────
    /// Listen for `prefers-reduced-motion` media query changes.
    /// Returns a cleanup function that removes the listener.
    fn on_reduced_motion_change(&self, callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()>;
    /// Returns `true` if the platform is macOS (for modifier key mapping).
    fn is_mac_platform(&self) -> bool;
    /// Returns the current monotonic time in milliseconds (e.g., `performance.now()`
    /// on web, `Instant::now()` on native). Used for skip-delay window tracking.
    fn now_ms(&self) -> u64;
    /// Get the bounding rectangle of an element by ID.
    fn get_bounding_rect(&self, id: &str) -> Option<Rect>;

    // ── Animation / Transition ──────────────────────────────────────
    /// Watch an element for CSS animation and/or transition completion.
    /// The callback fires once when all active animations and transitions
    /// on the element have ended. Handles reduced-motion detection,
    /// dual animation+transition support, and a fallback timeout.
    /// Returns a cleanup function that removes all listeners and timers.
    /// See `spec/components/overlay/presence.md` §11 for the full
    /// web implementation specification.
    fn on_animation_end(&self, id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()>;
}

/// Platform-agnostic bounding rectangle.
/// Replaces `web_sys::DomRect` in core machine types.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Opaque timer handle returned by `PlatformEffects::set_timeout()`.
/// The only operation is cancellation via `PlatformEffects::clear_timeout()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerHandle(u64);

impl TimerHandle {
    pub fn new(id: u64) -> Self { Self(id) }
    pub fn id(&self) -> u64 { self.0 }
}
```

**Resolution pattern:**

```rust
/// Resolve platform effects from ArsProvider context.
/// ArsProvider is the single root provider that bundles configuration,
/// platform capabilities, i18n, and style strategy.
/// Falls back to `NullPlatformEffects` with `log::warn!` diagnostics when
/// the `ars-core/debug` feature is enabled and no ArsProvider is found.
fn use_platform_effects() -> Arc<dyn PlatformEffects> {
    use_context::<ArsContext>()
        .map(|ctx| ctx.platform())
        .unwrap_or_else(|| {
            warn_missing_provider("use_platform_effects");
            Arc::new(MissingProviderEffects)
        })
}
```

**NullPlatformEffects** — no-op implementation for unit tests and SSR:

```rust
/// No-op implementation of PlatformEffects for unit tests and SSR.
/// All focus/DOM operations are silent no-ops. Timers fire immediately (no delay).
/// This is the INTENTIONAL no-op — used when tests or SSR explicitly pass
/// `NullPlatformEffects` to `ArsProvider`. No warnings are emitted.
pub struct NullPlatformEffects;

impl PlatformEffects for NullPlatformEffects {
    #[inline]
    fn focus_element_by_id(&self, _id: &str) {}

    #[inline]
    fn set_timeout(&self, _delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        callback(); // fire immediately in tests
        TimerHandle::new(0)
    }

    #[inline]
    fn clear_timeout(&self, _handle: TimerHandle) {}

    #[inline]
    fn announce(&self, _message: &str) {}
    // ... remaining methods are no-ops or return sensible defaults
}
```

**MissingProviderEffects** — fallback when `use_platform_effects()` finds no `ArsProvider`.
Same no-op behavior as `NullPlatformEffects`, but emits a debug-mode warning on every
method call so the developer sees exactly which platform operations are silently failing:

```rust
/// Fallback PlatformEffects used when no ArsProvider is in the component tree.
/// Behaves identically to NullPlatformEffects but emits `log::warn!` diagnostics
/// per call when the `ars-core/debug` feature is enabled.
/// This is NOT used in tests — only in the `use_platform_effects()` fallback path
/// inside adapters.
pub struct MissingProviderEffects;

impl MissingProviderEffects {
    #[cfg(feature = "debug")]
    #[inline]
    fn warn(method: &str) {
        log::warn!(
            "[ars-ui] {method}() called without ArsProvider. \
             Platform effects are disabled. Wrap your app root in <ArsProvider>."
        );
    }

    #[cfg(not(feature = "debug"))]
    #[inline]
    fn warn(_method: &str) {}
}

impl PlatformEffects for MissingProviderEffects {
    #[inline]
    fn focus_element_by_id(&self, _id: &str) {
        Self::warn("focus_element_by_id");
    }

    #[inline]
    fn set_timeout(&self, _delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        Self::warn("set_timeout");
        callback();
        TimerHandle::new(0)
    }

    #[inline]
    fn clear_timeout(&self, _handle: TimerHandle) {
        Self::warn("clear_timeout");
    }

    #[inline]
    fn announce(&self, _message: &str) {
        Self::warn("announce");
    }
    // ... remaining methods follow the same pattern
}
```

**Usage in effect closures:**

```rust
// Platform-agnostic effect — works on web, desktop, and mobile:
PendingEffect::new("focus-content", |ctx, _props, _send| {
    let platform = use_platform_effects();
    let content_id = ctx.ids.part("content");
    platform.focus_element_by_id(&content_id);
    no_cleanup()
})

// Timer effect with cleanup:
PendingEffect::new("typeahead-timeout", |_ctx, _props, send| {
    let platform = use_platform_effects();
    let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
        send.call_if_alive(Event::ClearTypeahead);
    }));
    Box::new(move || { platform.clear_timeout(handle); })
})
```

**Web implementation** (`ars-dom`):

```rust
// ars-dom/src/platform.rs
pub struct WebPlatformEffects;

impl PlatformEffects for WebPlatformEffects {
    fn focus_element_by_id(&self, id: &str) {
        // Delegates to existing ars_dom::focus_element_by_id() implementation
        crate::focus::focus_element_by_id(id);
    }
    fn set_timeout(&self, delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        let id = web_sys::window().unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &Closure::once_into_js(callback).into(), delay_ms as i32,
            ).unwrap();
        TimerHandle::new(id as u64)
    }
    fn announce(&self, message: &str) {
        crate::announcer::announce_polite(message);
    }
    // ... remaining methods delegate to existing ars_dom implementations
}
```

`PlatformEffects::announce()` and `announce_assertive()` define the minimum
cross-platform announcement surface: deliver a best-effort polite or assertive
live-region update. On web, `WebPlatformEffects` may implement this either by
delegating directly to `ars-dom` live-region DOM helpers or by routing through
an adapter-managed `LiveAnnouncer` service. Queueing, repeated-message
deduplication, and assertive preemption remain adapter-level concerns when the
full `LiveAnnouncer` integration is in use.

#### 2.2.8 Context Snapshot Semantics

Effects receive context and props at the moment their `setup` closure is invoked, which occurs after `drain_queue()` completes and returns `SendResult` to the adapter. This creates a subtle ordering hazard: if effect setup is deferred (e.g., scheduled on the next microtask or animation frame), a second event may arrive and call `send()` — mutating context via `apply()` — before the first event's effects have been set up.

**The problem:**

1. Event A arrives. `send(A)` processes the transition, mutates context, and returns `SendResult` containing `PendingEffect` E1.
2. The adapter defers E1's setup to the next microtask.
3. Before the microtask fires, Event B arrives. `send(B)` processes its transition and mutates context again.
4. E1's `setup` closure finally runs — but now `ctx` reflects post-B state, not post-A state.

**Requirements:**

- Effects MUST capture context values from the `setup()` closure's parameters, never by reading from the `Service` at a later point. The `setup` signature `FnOnce(&M::Context, &M::Props, WeakSend<M::Event>)` provides the context snapshot — this is the authoritative source.
- If effect setup is deferred, the adapter MUST clone the relevant context values at deferral time, not at execution time.

**Two valid strategies:**

1. **Snapshot capture (recommended):** Clone needed context fields into the deferred closure at the point where `SendResult` is processed. This is simple and correct regardless of scheduling.

2. **Synchronous effect setup:** Set up all effects synchronously before `send()` returns control to the framework. This eliminates the window for interleaving but may cause layout thrashing if effects touch the DOM.

```rust
// Strategy 1: Snapshot capture (recommended)
//
// The adapter processes SendResult and captures context values
// immediately, even if the actual DOM work is deferred.
fn process_effects<M: Machine>(result: SendResult<M>, service: &Service<M>) {
    for effect in result.pending_effects {
        // Clone context and props NOW, before any future send() can mutate them.
        let ctx_snapshot = service.context().clone();
        let props_snapshot = service.props().clone();
        let send = create_send_callback(service);

        // Even if setup is deferred, it uses the snapshot — not a live reference.
        schedule_microtask(move || {
            let cleanup = effect.run(&ctx_snapshot, &props_snapshot, send);
            active_cleanups.borrow_mut().push(cleanup);
        });
    }
}

// Strategy 2: Synchronous effect setup
//
// Set up effects inline before returning from the send() processing.
fn send_and_setup<M: Machine>(service: &mut Service<M>, event: M::Event, active_cleanups: &mut Vec<CleanupFn>) {
    let result = service.send(event);
    let send = create_send_callback(service);

    // Effects are set up synchronously — no window for interleaving.
    for effect in result.pending_effects {
        let cleanup = effect.run(service.context(), service.props(), send.clone());
        active_cleanups.push(cleanup);
    }
}
```

Adapters SHOULD prefer Strategy 1 unless they have a specific reason to require synchronous setup. Strategy 1 is compatible with framework scheduling (e.g., Leptos effect batching, Dioxus virtual DOM diffing) and avoids blocking the render pipeline.

### 2.3 Service Runtime

```rust
use alloc::collections::VecDeque;

/// Maximum number of events processed per `drain_queue` call before
/// breaking to prevent infinite transition loops.
const MAX_DRAIN_ITERATIONS: usize = 100;

/// Result of sending an event to the service.
pub struct SendResult<M: Machine> {
    /// Whether any state change occurred during this send cycle.
    pub state_changed: bool,
    /// Whether any context mutation occurred (via `plan.apply`).
    /// Adapters should trigger re-render when `state_changed || context_changed`.
    pub context_changed: bool,
    /// Effects that the adapter must set up.
    pub pending_effects: Vec<PendingEffect<M>>,
    /// Named effects to cancel. The adapter runs their cleanup closures
    /// immediately, before setting up any new `pending_effects`.
    pub cancel_effects: Vec<&'static str>,
    /// Whether the event queue was truncated due to hitting MAX_DRAIN_ITERATIONS.
    /// When `true`, some enqueued events were dropped — log a warning in debug builds.
    pub truncated: bool,
    /// Number of consecutive iterations at the end of `drain_queue()` where
    /// `target` was `None` (no state change). Resets to zero whenever a state
    /// transition occurs. Useful for diagnostics — a high trailing value may
    /// indicate a `context_only` + `then_send` feedback loop.
    pub context_change_count: usize,
}

pub struct Service<M: Machine> {
    state: M::State,
    context: M::Context,
    props: M::Props,
    event_queue: VecDeque<M::Event>,
    unmounted: bool, // See §2.3 for lifecycle
}

impl<M: Machine> Service<M> {
    pub fn new(props: M::Props, env: &Env, messages: &M::Messages) -> Self {
        debug_assert!(!props.id().is_empty(), "Props::id must not be empty");
        let (state, context) = M::init(&props, env, messages);
        Self {
            state,
            context,
            props,
            event_queue: VecDeque::new(),
            unmounted: false,
        }
    }

    /// Construct a Service from a hydrated state snapshot. Used during SSR hydration
    /// to restore server-rendered state on the client without re-running `Machine::init`.
    /// Calls `Machine::init` to derive a valid context from props, then replaces
    /// the initial state with the hydrated state. This ensures context (which
    /// contains computed IDs, derived flags, etc.) is always correctly derived
    /// from props, while state is restored from the server snapshot.
    #[cfg(feature = "ssr")]
    pub fn new_hydrated(props: M::Props, state: M::State, env: &Env, messages: &M::Messages) -> Self {
        debug_assert!(!props.id().is_empty(), "Props::id must not be empty");
        let (_init_state, context) = M::init(&props, env, messages);
        Self {
            state,
            context,
            props,
            event_queue: VecDeque::new(),
            unmounted: false,
        }
    }

    // Props validation is the caller's responsibility. Adapters should validate
    // props before constructing a `Service` if the machine's API provides validation helpers.

    // Note: Service requires `M::Props: 'static`. For borrowed data from parent
    // components, wrap in `Rc<RefCell<_>>` or clone. Props are consumed at
    // Service creation.

    pub fn state(&self) -> &M::State { &self.state }
    pub fn context(&self) -> &M::Context { &self.context }
    pub fn props(&self) -> &M::Props { &self.props }

    /// Test-only: force the service into a specific state. Re-derives context
    /// from the new state and current props via `Machine::init`, discarding
    /// the init state. Uses default env and messages. Used by keyboard matrix
    /// tests to start from arbitrary states.
    #[cfg(test)]
    pub fn set_state_for_test(&mut self, state: M::State) {
        let (_init_state, context) = M::init(&self.props, &Env::default(), &M::Messages::default());
        self.state = state;
        self.context = context;
    }

    /// Update props atomically. When props change, the machine may need to
    /// re-derive state and context. This method updates props, state, and context
    /// in a single batch so that observers never see mixed old/new values.
    ///
    /// **Atomic update contract:** `set_props()` MUST update `self.props`, then
    /// immediately re-sync any `Bindable` fields in context and process any
    /// resulting `SetValue` events before returning. Callers (adapters) MUST NOT
    /// interleave reads between `set_props()` and the subsequent `drain_queue()`.
    pub fn set_props(&mut self, props: M::Props) -> SendResult<M> {
        let old_props = core::mem::replace(&mut self.props, props);
        let events = M::on_props_changed(&old_props, &self.props);
        for event in events {
            self.event_queue.push_back(event);
        }
        self.drain_queue()
    }

    // NOTE: Service<M> stores owned Props, State, and Context values.
    // All associated types must be 'static (no borrowed data).
    // For external borrowed data, wrap in shared ownership:
    //   - WASM (single-threaded): use `Rc<RefCell<Data>>`
    //   - Native (multi-threaded): use `Arc<RwLock<Data>>`
    // Store the shared pointer in Context and access via borrow in connect().

    /// Send an event, process it and any chained events iteratively.
    /// Returns a SendResult with state change info and pending effects.
    // send() — full implementation with unmount guard in §2.3 below
    pub fn send(&mut self, event: M::Event) -> SendResult<M> {
        debug_assert!(!self.unmounted, "send() called after unmount()");
        if self.unmounted {
            return SendResult {
                state_changed: false,
                context_changed: false,
                pending_effects: Vec::new(),
                cancel_effects: Vec::new(),
                truncated: false,
                context_change_count: 0,
            };
        }
        self.event_queue.push_back(event);
        self.drain_queue()
    }

    fn drain_queue(&mut self) -> SendResult<M> {
        let mut pending_effects = Vec::new();
        let mut cancel_effects = Vec::new();
        let mut state_changed = false;
        let mut context_changed = false;
        let mut truncated = false;
        let mut iterations = 0;
        let mut context_change_count: usize = 0;

        while let Some(event) = self.event_queue.pop_front() {
            iterations += 1;
            if iterations > MAX_DRAIN_ITERATIONS {
                #[cfg(debug_assertions)]
                panic!("Event queue exceeded {MAX_DRAIN_ITERATIONS} iterations — likely an infinite loop in transitions");
                #[cfg(not(debug_assertions))]
                {
                    #[cfg(feature = "debug")]
                    log::warn!(
                        "drain_queue: event queue exceeded {} iterations, truncating. \
                         This likely indicates an infinite loop in state machine transitions.",
                        MAX_DRAIN_ITERATIONS
                    );
                    truncated = true;
                    break;
                }
            }

            if let Some(plan) = M::transition(&self.state, &event, &self.context, &self.props) {
                // Apply context mutation.
                // `FnOnce` closures are consumed on invocation. Moving the
                // `Box<dyn FnOnce>` out of the `Option` via `if let Some(...)`
                // transfers ownership to the caller. The closure is called
                // exactly once and then dropped.
                if let Some(apply) = plan.apply {
                    apply(&mut self.context);
                    context_changed = true;
                }

                // Detect context_only + then_send infinite loops.
                // Reset counter on any state change (with or without apply).
                if plan.target.is_none() {
                    context_change_count += 1;
                    if context_change_count >= MAX_DRAIN_ITERATIONS {
                        #[cfg(debug_assertions)]
                        panic!(
                            "drain_queue: {context_change_count} consecutive context_only \
                             iterations without a state change — likely an infinite \
                             context_only + then_send loop"
                        );
                        #[cfg(not(debug_assertions))]
                        {
                            #[cfg(feature = "debug")]
                            log::warn!(
                                "drain_queue: {} context_only iterations without state \
                                 change — possible infinite loop, truncating.",
                                context_change_count
                            );
                            truncated = true;
                            break;
                        }
                    }
                } else {
                    context_change_count = 0;
                }

                // Enqueue follow-up events
                // If iteration over follow-up events is interrupted (panic, early
                // return), the adapter MUST manually call cleanup for outstanding
                // effects. Consider wrapping the event iterator in a RAII guard
                // that runs remaining cleanups on drop.
                self.event_queue.extend(plan.then_send);

                // Apply state change
                if let Some(next) = plan.target {
                    self.state = next;
                    state_changed = true;
                }

                // Collect effect cancellations.
                cancel_effects.extend(plan.cancel_effects);

                // Collect effects for adapter, tagged with source state.
                // NOTE: Effects receive the FINAL context snapshot when set up
                // by the adapter, not the intermediate context at this point.
                // Machines must design accordingly.
                let target = self.state.clone();
                pending_effects.extend(plan.effects.into_iter().map(|mut e| {
                    e.target_state = Some(target.clone());
                    e
                }));
            }
        }

        SendResult { state_changed, context_changed, pending_effects, cancel_effects, truncated, context_change_count }
    }

    /// Unmount the service, running all active effect cleanups and clearing the event queue.
    ///
    /// After calling `unmount()`, no further `send()` calls are valid. In debug builds,
    /// subsequent `send()` calls will panic; in release builds, they are silently ignored.
    ///
    /// **Contract:**
    /// 1. All active effect cleanup functions (stored by the adapter) are invoked in
    ///    reverse setup order.
    /// 2. The internal event queue is cleared — any pending events are discarded.
    /// 3. The `unmounted` flag is set, preventing further transitions.
    ///
    /// Adapters MUST call `unmount()` when removing the component from the tree.
    /// Failing to call `unmount()` will leak effect resources (timers, event listeners,
    /// DOM mutations, etc.).
    pub fn unmount(&mut self, active_cleanups: Vec<CleanupFn>) {
        // Run all active effect cleanups in reverse order (LIFO).
        for cleanup in active_cleanups.into_iter().rev() {
            cleanup();
        }
        // Discard any pending events — the component is being torn down.
        self.event_queue.clear();
        // Mark as unmounted to reject future send() calls.
        self.unmounted = true;
    }

    /// Returns `true` after `unmount()` has been called.
    pub fn is_unmounted(&self) -> bool {
        self.unmounted
    }
```

The `send()` method (defined in §2.3 above) checks the `unmounted` flag and returns an inert `SendResult` if the service has been unmounted.

### 2.4 Callback Invocation Protocol

1. **Timing:** Callbacks may be queued during a transition (i.e., while
   `transition()` and `apply()` execute), but they are **not** fired
   inline. Instead, they are collected as `PendingEffect` entries and
   returned to the adapter inside `SendResult::pending_effects`. The
   adapter executes them **after** `send()` completes and returns.
2. **Adapter-driven execution:** The `Service` core never invokes
   callbacks directly. All effect execution — including callback
   invocation — is the adapter's responsibility. This keeps the core
   free of platform dependencies and gives adapters control over
   scheduling (synchronous, microtask, animation frame, etc.).
3. **Panic handling:** Because state/context mutations are committed
   _before_ effects are returned, a panic inside a callback (during
   adapter-side execution) leaves those mutations in place. The adapter
   should treat callback panics as application bugs; the machine's
   state remains consistent.
4. **Ordering:** Callbacks fire in the same order as `PendingEffect`
   entries — i.e., the order they were appended to
   `TransitionPlan::effects`.
5. **No re-entrant `send()`:** Calling `send()` from within a callback
   is **forbidden**. The `Service` does not support re-entrant
   transitions. Adapters that need to dispatch follow-up events should
   use `TransitionPlan::then_send` or schedule them asynchronously after
   the current effect batch completes.

```rust
    // NOTE: The Service does NOT internally track active effects or manage
    // their lifecycle. Effects are returned in `SendResult::pending_effects`
    // for the adapter to manage. When `state_changed` is true, the adapter
    // MUST clean up effects from the previous state before setting up new ones.
    // This design keeps the core Service free of DOM/platform dependencies.

    /// Connect the machine to produce its public API.
    ///
    /// The lifetime `'a` on `Api` enforces that it cannot outlive the `send`
    /// callback or the `Service` borrow. Callers must not store the returned
    /// `Api` beyond the scope that borrows `self` and `send`.
    /// IMPORTANT: Api lifetime is tied to &'a self. Api cannot outlive the Service reference.
    pub fn connect<'a>(&'a self, send: &'a dyn Fn(M::Event)) -> M::Api<'a> {
        M::connect(&self.state, &self.context, &self.props, send)
    }

}
```

#### 2.4.1 Follow-up Event Safety

Follow-up events emitted via `TransitionPlan::then_send` are a powerful mechanism for decomposing complex transitions into smaller steps. However, they introduce the risk of infinite event loops if not carefully constrained.

**Requirements for follow-up events:**

- Follow-up events MUST be **pure**: their generation must depend only on the current `State` and `Context`, never on frame timing, animation progress, external I/O, or other non-deterministic inputs.
- All potential cycles in follow-up event chains MUST be prevented by **guards** — boolean conditions in `transition()` that evaluate to `false` once the desired state is reached. Iteration limits are not an acceptable cycle-prevention strategy.
- `MAX_DRAIN_ITERATIONS` (100) is a **safety net only**. Hitting this limit in production indicates a specification or implementation bug. In debug builds, exceeding the limit panics immediately to surface the problem during development.
- Guard conditions MUST be evaluated against post-mutation state. Since `apply` runs before `then_send` events are enqueued, guards in subsequent transitions see the updated `Context`.

**Cycle prevention via guards:**

The correct pattern is to include a guard in `transition()` that makes the follow-up event a no-op once the target condition is met. This guarantees convergence in a bounded number of steps.

```rust
fn transition(
    state: &Self::State,
    event: &Self::Event,
    context: &Self::Context,
    _props: &Self::Props,
) -> Option<TransitionPlan<Self>> {
    match (state, event) {
        // User toggles "select all" — enqueue individual selection events
        (State::Active, Event::SelectAll) => {
            // Guard: only proceed if there are unselected items
            if ctx.all_selected() {
                return None; // No-op — prevents cycle
            }
            Some(TransitionPlan::new()
                .apply(|ctx| ctx.mark_all_selected())
                .then(Event::SyncSelectionIndicator))
        }
        // Follow-up: sync the visual indicator after selection changed
        (State::Active, Event::SyncSelectionIndicator) => {
            // Guard: only update indicator if it doesn't match current state
            let expected = ctx.compute_indicator_state();
            if ctx.indicator_state == expected {
                return None; // Already in sync — chain terminates here
            }
            Some(TransitionPlan::new()
                .apply(move |ctx| ctx.indicator_state = expected))
        }
        _ => None,
    }
}
```

In this example, the `SyncSelectionIndicator` handler checks whether the indicator already reflects the current selection. If it does, the handler returns `None`, terminating the chain. This pattern guarantees that the event loop drains in at most two iterations regardless of how many times `SelectAll` fires.

#### 2.4.2 Concurrent Input Modality Tracking

Multiple input modalities can be active simultaneously within a single frame. A user may hold a mouse button while pressing a keyboard key, or use a stylus while pressing modifier keys. Machines MUST track each input channel independently rather than assuming mutual exclusion.

**Required tracking fields in Context (for interaction machines):**

```rust
pub struct InteractionContext {
    /// Currently held keys — may contain multiple simultaneous keys.
    // KeyboardKey is defined in ars-core; re-exported by 05-interactions.md §13
    pub pressed_keys: BTreeSet<KeyboardKey>,
    /// Whether any pointer (mouse, touch, stylus) is currently down.
    pub pointer_down: bool,
}
```

A component is considered "pressed" if EITHER `pointer_down` is `true` OR `pressed_keys` contains the activation key (e.g., `Space` or `Enter`). Guards that check pressed state MUST use an OR condition:

```rust
fn is_pressed(ctx: &InteractionContext) -> bool {
    ctx.pointer_down || ctx.pressed_keys.contains(&KeyboardKey::Space)
                     || ctx.pressed_keys.contains(&KeyboardKey::Enter)
}
```

Keyboard and pointer events arriving in the same frame are processed in queue order — no special interleaving is required. The `data-ars-pressed` attribute reflects the combined pressed state. See `05-interactions.md` §2-3 for the full Press and Hover interaction specifications.

### 2.5 ScrollLockManager (ars-dom)

> **Canonical location:** `11-dom-utilities.md` §5.2-5.3 — reference-counted scroll locking
> for nested overlay coordination (`ScrollLockManager`, `acquire()`/`release()`).

### 2.6 Controlled vs Uncontrolled: Bindable

```rust
/// Controlled/uncontrolled value container.
/// Change notification is the adapter's responsibility using native callback types.
#[derive(Clone, Debug, PartialEq)]
pub struct Bindable<T: BindableValue> {
    /// Value provided externally (controlled mode).
    controlled: Option<T>,
    /// Internally managed value (uncontrolled mode).
    internal: T,
}

impl<T: BindableValue> Bindable<T> {
    /// Create an uncontrolled bindable with the given default value.
    #[must_use]
    pub const fn uncontrolled(default: T) -> Self {
        Self { controlled: None, internal: default }
    }

    /// Create a controlled bindable with an externally managed value.
    #[must_use]
    pub fn controlled(value: T) -> Self {
        Self { controlled: Some(value.clone()), internal: value }
    }

    /// Get the current effective value.
    #[must_use]
    pub fn get(&self) -> &T {
        self.controlled.as_ref().unwrap_or(&self.internal)
    }

    /// Set the internal value. In controlled mode, this is the "pending" value
    /// until the next sync from the controlling signal.
    pub fn set(&mut self, value: T) {
        self.internal = value;
    }

    /// Sync the controlled value from an external source.
    /// Called by the adapter when the controlling signal changes.
    pub fn sync_controlled(&mut self, value: Option<T>) {
        self.controlled = value;
    }

    /// Whether this bindable is in controlled mode.
    #[must_use]
    pub const fn is_controlled(&self) -> bool {
        self.controlled.is_some()
    }
}

impl<T: BindableValue + Default> Default for Bindable<T> {
    fn default() -> Self {
        Self::uncontrolled(T::default())
    }
}
```

> **Trait bounds on `Bindable<T>`:** All `Bindable<T>` parameters require `T: BindableValue` (i.e., `T: Clone + PartialEq + Debug`). This is enforced at compile time via trait bounds on Props structs. The bounds enable change detection (`PartialEq`), state cloning (`Clone`), and debug output (`Debug`).
>
> **Lifetime constraint:** `T` must additionally satisfy `'static`. `Bindable<T>` is stored inside `Context`, which is owned by `Service<M>` and has no lifetime parameter. Use `Rc<str>` or `Arc<String>` instead of `&str` for shared string data. Borrowed references cannot be used as `T` because `Service` must own all its data across render cycles.

```rust
/// Trait alias for types usable with `Bindable<T>`.
/// All bindable values must satisfy these bounds to support
/// change detection (`PartialEq`), state cloning (`Clone`),
/// and debug output (`Debug`).
pub trait BindableValue: Clone + PartialEq + Debug {}
impl<T: Clone + PartialEq + Debug> BindableValue for T {}
```

#### 2.6.1 Bindable Clone Optimization for Collections

When `Context` contains `Bindable<Vec<T>>` (e.g., selection lists, tag inputs), every state transition clones the entire context — including the collection. For collections with thousands of items, this creates significant allocation pressure.

**`get_mut_owned()` for in-place mutation:**

`Bindable<T>` provides `get_mut_owned()` to access the internal value for mutation without requiring a full clone when the value is uncontrolled:

```rust
impl<T: BindableValue> Bindable<T> {
    /// Returns a mutable reference to the internal value for in-place mutation.
    /// In controlled mode, mutations apply to the internal copy; the controlled
    /// value takes precedence on the next `get()` call until `sync_controlled`
    /// is called.
    #[must_use]
    pub const fn get_mut_owned(&mut self) -> &mut T {
        &mut self.internal
    }
}
```

**Copy-on-Write pattern for collection components:**

For components managing large collections (TreeView, TagGroup, Table selections), use `Rc<Vec<T>>`/`Arc<Vec<T>>` with `make_mut()` instead of `Vec<T>` directly. This avoids cloning when only one reference exists:

```rust
// On WASM: use Rc (single-threaded, no atomic overhead)
#[cfg(target_arch = "wasm32")]
use alloc::rc::Rc;
// On native: use Arc (thread-safe for SSR)
#[cfg(not(target_arch = "wasm32"))]
use alloc::sync::Arc;

// In Context definition (cfg-gated type alias recommended):
pub struct Context {
    pub selected_keys: Bindable<SharedVec<Key>>,  // SharedVec = Rc<Vec<Key>> or Arc<Vec<Key>>
}

// In transition apply closure — clone only if Arc is shared:
TransitionPlan::to(State::Active)
    .apply(|ctx| {
        let items = ctx.selected_keys.get_mut_owned();
        Arc::make_mut(items).push(new_key);
    })
```

When `Arc::make_mut()` is called on an `Arc` with a reference count of 1, no clone occurs — the inner `Vec` is mutated in place. A clone happens only when another reference exists (e.g., an adapter signal still holds the previous value). This reduces per-transition allocations from O(n) to O(1) in the common case. See `06-collections.md` for collection-specific machine patterns.

> **When to use `Bindable` vs adapter-level sync:**
>
> - Use `Bindable<T>` for values that the machine itself must track for state transitions (e.g., `checked`, `pressed`, `value`) — these live in `Context`.
> - For values that only the adapter cares about (e.g., CSS class overrides, render-time formatting), keep them as adapter-local signals. Don't put them in `Context` or wrap them in `Bindable`.
> - Rule of thumb: if the machine's `transition()` function reads it, it belongs in `Bindable<T>`. If only `connect()` reads it for display, a plain `Props` field suffices.

#### 2.6.2 Focus State Management

Focus is intentionally NOT modeled as `Bindable<bool>`. Unlike `value` or `checked`, focus is a side-effectful browser state that cannot be round-tripped through a reactive binding without causing infinite loops (setting focus triggers `onfocus`, which would update the binding, which would re-set focus).

Instead, focus is managed through:

- **`auto_focus: bool`** prop — requests focus on mount.
- **`on_focus: Option<Callback<FocusEvent>>`** / **`on_blur: Option<Callback<FocusEvent>>`** — event callbacks for tracking focus state.
- **Imperative API**: Components expose a `focus()` method on their ref/handle for programmatic focus control.

Consumers who need reactive focus tracking should derive it from `on_focus`/`on_blur` callbacks into their own signal.

#### 2.6.3 Bindable Controlled Sync Timing

When a `Bindable<T>` field is in controlled mode, the adapter synchronizes external prop values into the machine via `Event::SetValue`. The following ordering guarantees prevent race conditions:

1. **User input priority:** User-initiated events (keystrokes, clicks, selections) have **HIGHER priority** than prop-change events. If a user event and a prop sync arrive in the same frame, the user event is processed LAST — its result takes precedence. Adapters MUST enqueue `SetValue` events at the **front** of the event queue (prepend), so that user events queued afterward overwrite the prop value. This ensures that typing into a controlled input always reflects the user's keystrokes, even if the parent re-renders with a stale value in the same frame.
2. **Deduplication:** Adapters MUST deduplicate consecutive `SetValue` events with identical values. If the controlled value hasn't changed, no event is enqueued.
3. **Coalescing:** If props change multiple times before the adapter syncs (e.g., rapid parent re-renders), only the latest value is applied. Intermediate values are dropped.
4. **Queue ordering:** `SetValue` events are prepended to the event queue, not appended. User-input events are appended. This ordering ensures the machine processes the controlled prop sync first, then the user's interaction overwrites it — the user's intent always wins.

> **Rationale:** In controlled mode, the parent component is the source of truth. However, user input must feel immediate — if a controlled input ignores keystrokes until the parent round-trips the value, the UI feels laggy. By processing user events after prop syncs within the same frame, the machine's final state reflects user intent. The adapter then notifies the parent via `on_change`, and the parent's next render confirms or rejects the value. See `components/input/_category.md` for per-component controlled input behavior.

#### 2.6.4 Bindable Controlled Sync and Circular Update Prevention

In controlled mode, a value update follows this cycle: controlled prop changes -> adapter sends `Event::SetValue` -> machine fires `on_change` callback -> parent updates its signal -> signal change triggers another `Event::SetValue`. The machine's `transition()` function includes a guard that skips the transition if the new value equals the current value, which breaks the cycle.

However, the `on_change` callback fires for **both** user-initiated and programmatic value changes. The machine does not distinguish between the two — this is intentional, because the parent component is the source of truth and must be notified of all value changes regardless of origin.

**Consumer responsibility:** Consumers who perform side effects in `on_change` (e.g., network requests, analytics) MUST guard against redundant invocations:

```rust
// In the parent component's on_change handler (Dioxus example; Leptos
// uses equivalent signal APIs — see adapter specs for framework-specific patterns):
let previous = use_signal(|| String::new());

let on_change = Callback::new(move |new_value: String| {
    // Guard: skip side effects if value hasn't actually changed
    if new_value != *previous.read() {
        previous.set(new_value.clone());
        // Safe to perform side effects here
        save_to_server(&new_value);
    }
    // Always update the controlled signal (this is cheap / idempotent)
    value_signal.set(new_value);
});
```

Without this guard, the `on_change` -> signal update -> `SetValue` -> `on_change` cycle would cause the side effect to fire twice: once for the user's edit and once for the programmatic sync. The machine's internal deduplication prevents infinite loops, but it does not prevent the callback from being invoked on both legs of the round-trip. See `08-adapter-leptos.md` and `09-adapter-dioxus.md` for adapter-specific controlled value sync patterns.

#### 2.6.5 Developer Guide: Controlled vs Uncontrolled Components

`Bindable<T>` is the mechanism that allows every ars-ui component to operate in either **controlled** mode (parent owns the value) or **uncontrolled** mode (component manages its own value). Understanding this duality is essential for using components correctly.

**Uncontrolled mode** — the component manages its own state. The parent provides a default value and optionally listens for changes:

```rust
// Uncontrolled checkbox — component tracks checked state internally.
// Parent receives notifications but does not drive the value.
let on_change = Callback::new(|checked: bool| {
    log::info!("Checkbox is now: {checked}");
});

view! {
    <Checkbox
        id="terms"
        default_checked=false   // Bindable::uncontrolled(false) under the hood
        on_change=on_change
    />
}
```

**Controlled mode** — the parent owns the value and passes it as a prop. The component reflects the parent's value and reports changes via `on_change`:

```rust
// Controlled input — parent signal is the source of truth.
let value = use_signal(|| "hello".to_string());

let on_change = Callback::new(move |new_value: String| {
    value.set(new_value);
});

view! {
    <Input
        id="name"
        value=value.get()       // Bindable::controlled(value) under the hood
        on_change=on_change
    />
}
```

**Two-way sync** — controlled mode with transformation. The parent can reject or modify the value before confirming it:

```rust
// Controlled input that uppercases all input
let value = use_signal(|| String::new());

let on_change = Callback::new(move |new_value: String| {
    // Transform before accepting — the machine's next render
    // will reflect the uppercased value via prop sync.
    value.set(new_value.to_uppercase());
});

view! {
    <Input
        id="upper"
        value=value.get()
        on_change=on_change
    />
}
```

**What breaks when mixing modes:** A component's mode is determined at construction time by whether `controlled` is `Some` or `None` in the `Bindable`. Switching from uncontrolled to controlled (or vice versa) after mount is not supported and produces a console warning in debug builds. If you need to toggle control, unmount and remount the component with a different `key`.

For implementation details on how adapters wire `Bindable<T>` to framework signals, see `08-adapter-leptos.md` §3 and `09-adapter-dioxus.md` §2. For testing controlled/uncontrolled behavior, see `spec/testing/09-state-machine-correctness.md`.

### 2.7 Machine Definition Example: Toggle

````rust
// Example: A simple toggle machine demonstrating all new patterns.
// All types live in `mod toggle` — external usage: toggle::Machine, toggle::State, toggle::Props, etc.

pub mod toggle {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub enum State {
        Off,
        On,
    }

    #[derive(Clone, Debug)]
    pub enum Event {
        Toggle,
        SetPressed(bool),
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Context {
        pub pressed: Bindable<bool>,
        pub disabled: bool,
    }

    /// Props contain only data — no callback fields.
    /// Change notification is the adapter's responsibility.
    #[derive(Clone, Debug, Default, PartialEq, HasId)]
    pub struct Props {
        /// Base ID provided by the adapter (e.g., from `use_id()` in ars-leptos).
        pub id: String,
        pub pressed: Option<bool>,
        pub default_pressed: bool,
        pub disabled: bool,
    }

    /// Convenience constructor: create Props with just an ID and defaults for the rest.
    /// Recommended pattern for all component Props types.
    impl Props {
        pub fn with_id(id: impl Into<String>) -> Self {
            Self { id: id.into(), ..Default::default() }
        }
    }

    /// #### Optional Builder Pattern for Props
    ///
    /// For components with many Props fields, a builder can reduce boilerplate.
    /// This is **optional ergonomic sugar** — direct struct construction remains
    /// the canonical approach and is always valid.
    ///
    /// ```rust
    /// impl Props {
    ///     pub fn builder(id: impl Into<String>) -> PropsBuilder {
    ///         PropsBuilder {
    ///             id: id.into(),
    ///             pressed: None,
    ///             default_pressed: false,
    ///             disabled: false,
    ///         }
    ///     }
    /// }
    ///
    /// pub struct PropsBuilder {
    ///     id: String,
    ///     pressed: Option<bool>,
    ///     default_pressed: bool,
    ///     disabled: bool,
    /// }
    ///
    /// impl PropsBuilder {
    ///     pub fn pressed(mut self, v: Option<bool>) -> Self { self.pressed = v; self }
    ///     pub fn default_pressed(mut self, v: bool) -> Self { self.default_pressed = v; self }
    ///     pub fn disabled(mut self, v: bool) -> Self { self.disabled = v; self }
    ///     pub fn build(self) -> Props {
    ///         Props {
    ///             id: self.id,
    ///             pressed: self.pressed,
    ///             default_pressed: self.default_pressed,
    ///             disabled: self.disabled,
    ///         }
    ///     }
    /// }
    ///
    /// // Usage:
    /// let props = Props::builder("btn-1").disabled(true).build();
    /// ```
    ///
    /// Components are NOT required to provide a builder. For simple Props with
    /// few fields, `Props { id: ..., ..Default::default() }` or `Props::with_id()`
    /// is sufficient.

    /// Typed part enum for Toggle's DOM structure.
    #[derive(ComponentPart)]
    #[scope = "toggle"]
    pub enum Part {
        Root,
    }

    /// Per-component API. Returned by `connect()`.
    /// Methods returning `AttrMap` provide static attributes.
    /// Typed handler methods wire framework event handlers.
    ///
    /// The lifetime `'a` on `Api` enforces that it cannot outlive the `send`
    /// callback or the `Service` borrow.
    pub struct Api<'a> {
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    }

    impl<'a> Api<'a> {
        pub fn is_pressed(&self) -> bool {
            *self.state == State::On
        }

        /// ARIA attributes, role, data-state, tabindex for the root element.
        // Note: both chained (attrs.set(...).set(...)) and unchained (individual calls)
        // patterns are valid styles for AttrMap construction.
        pub fn root_attrs(&self) -> AttrMap {
            let mut attrs = AttrMap::new();
            attrs.set(HtmlAttr::Id, &self.props.id);
            let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
            attrs.set(scope_attr, scope_val);
            attrs.set(part_attr, part_val);
            attrs.set(HtmlAttr::Data("ars-state"), if self.is_pressed() { "on" } else { "off" });
            attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if self.is_pressed() { "true" } else { "false" });
            attrs.set(HtmlAttr::Role, "button");
            attrs.set(HtmlAttr::TabIndex, "0");
            if self.ctx.disabled {
                attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
                attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            }
            attrs
        }

        /// Typed click handler — adapters wire this to their native click event.
        pub fn on_root_click(&self) {
            (self.send)(Event::Toggle);
        }

        /// Typed keydown handler — receives key name and modifier state.
        pub fn on_root_keydown(&self, key: &str, _shift: bool) {
            match key {
                "Enter" | " " => (self.send)(Event::Toggle),
                _ => {}
            }
        }
    }

    impl ConnectApi for Api<'_> {
        type Part = Part;

        fn part_attrs(&self, part: Self::Part) -> AttrMap {
            match part {
                Part::Root => self.root_attrs(),
            }
        }
    }

    /// The Toggle machine.
    pub struct Machine;

    // Fully-qualified path avoids shadowing the local `Machine` struct name.
    /// Toggle has no translatable strings, so Messages is a unit struct.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Messages;
    impl ComponentMessages for Messages {}

    impl ars_core::Machine for Machine {
        type State = State;
        type Event = Event;
        type Context = Context;
        type Props = Props;
        type Messages = Messages;
        type Api<'a> = Api<'a>;  // Lifetime `'a` is bound to the `send` closure in `connect()` — see §2.4

        fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
            let pressed = match props.pressed {
                Some(v) => Bindable::controlled(v),
                None => Bindable::uncontrolled(props.default_pressed),
            };
            let state = if *pressed.get() { State::On } else { State::Off };
            (state, Context { pressed, disabled: props.disabled })
        }

        fn transition(
            state: &State,
            event: &Event,
            context: &Context,
            _props: &Props,
        ) -> Option<TransitionPlan<Self>> {
            if context.disabled { return None; }

            match event {
                // Toggle delegates to SetPressed via `then()` rather than
                // duplicating the state transition logic. This is preferred
                // over direct recursive delegation (calling Self::transition
                // directly) because `then()` is queue-based and avoids
                // stack growth for multi-level delegation chains.
                Event::Toggle => {
                    let next_pressed = match state {
                        State::Off => true,
                        State::On => false,
                    };
                    Some(TransitionPlan::new()
                        .then(Event::SetPressed(next_pressed)))
                }
                Event::SetPressed(pressed) => {
                    let next = if *pressed { State::On } else { State::Off };
                    let p = *pressed;
                    Some(TransitionPlan::to(next)
                        .apply(move |ctx| ctx.pressed.set(p)))
                }
            }
        }

        fn connect<'a>(
            state: &'a State,
            ctx: &'a Context,
            props: &'a Props,
            send: &'a dyn Fn(Event),
        ) -> Api<'a> {
            Api { state, ctx, props, send }
        }
    }
}
````

### 2.8 SSR Architectural Principles

> **Principle**: All ARIA attributes must be computable from `(State, Context, Props)` and returned by `connect()`. Effects handle DOM-only concerns (focus, scroll lock, event listeners).

This separation ensures that server-side rendering produces complete, accessible HTML without executing any effects. See [§9](#9-ssr-architectural-principles) for full details.

### 2.9 Development and Debugging

`ars-core` provides an optional `debug` feature flag (§1.4) that enables structured logging via the `log` crate facade. This section documents how to activate and use debug output across different targets.

**Enabling debug logging:**

| Target              | Logger Initialization                          | Environment Variable                       |
| ------------------- | ---------------------------------------------- | ------------------------------------------ |
| Native (tests, CLI) | `env_logger::init()` in `main()` or test setup | `RUST_LOG=ars_core=trace`                  |
| WASM (browser)      | `console_log::init_with_level(Level::Trace)`   | N/A (set level in code)                    |
| Dioxus Desktop      | `env_logger::init()` in `main()`               | `RUST_LOG=ars_core=trace,ars_dioxus=debug` |

**Structured log format:**

All transition log lines follow a single structured format for machine-parseable output:

```text
[ars:{component_id}] {state} + {event} → {target_state} (guard: {result}, effects: [{names}])
```

**Field definitions (tracing-compatible span/event fields):**

| Field          | Type     | Description                                                                                    |
| -------------- | -------- | ---------------------------------------------------------------------------------------------- |
| `component_id` | `String` | The `Props::id()` value, e.g. `"toggle#btn-1"`                                                 |
| `state`        | `String` | `Debug` representation of the current state before transition                                  |
| `event`        | `String` | `Debug` representation of the incoming event                                                   |
| `target_state` | `String` | `Debug` representation of the state after transition; `"(same)"` if no state change            |
| `guard`        | `String` | `"pass"` if transition matched, `"reject"` if no transition plan was returned                  |
| `effects`      | `String` | Bracketed list of effect names from `PendingEffect::name`, for example `[focus]` or `[]`       |
| `apply`        | `String` | The `apply_description` from `TransitionPlan`, or `"none"`                                     |
| `then_send`    | `String` | Bracketed list of follow-up events using their `Debug` form, for example `[FocusTrap]` or `[]` |
| `iteration`    | `u32`    | 1-based index within the current `drain_queue()` cycle                                         |
| `queue_depth`  | `u32`    | Number of events remaining in the queue after this iteration                                   |

**Example output for a complete transition cycle:**

When `RUST_LOG=ars_core=trace` is set and the `debug` feature is enabled, a toggle button click produces:

```text
[ars:toggle#btn-1] Off + Toggle → On (guard: pass, effects: [notify_change])
[ars:toggle#btn-1]   apply: "set pressed = true", then_send: [], iteration: 1, queue_depth: 0
```

A multi-step transition (e.g., dialog open with focus trap):

```text
[ars:dialog#dlg-1] Closed + Open → Opening (guard: pass, effects: [scroll_lock, inert_siblings])
[ars:dialog#dlg-1]   apply: "set open = true", then_send: [FocusTrap], iteration: 1, queue_depth: 1
[ars:dialog#dlg-1] Opening + FocusTrap → Open (guard: pass, effects: [focus_first])
[ars:dialog#dlg-1]   apply: "none", then_send: [], iteration: 2, queue_depth: 0
```

A rejected event (guard fails):

```text
[ars:combobox#cb-1] Closed + SelectItem(3) → (same) (guard: reject, effects: [])
```

**`tracing` span integration:**

When using the `tracing` crate instead of `log`, each `drain_queue()` call opens a span:

```rust
let span = tracing::trace_span!("ars_transition",
    component_id = %self.props.id(),
);
let _enter = span.enter();

// Each iteration emits a structured event:
tracing::trace!(
    state = %format!("{:?}", self.state),
    event = %format!("{:?}", event),
    target_state = %format!("{:?}", next_state),
    guard = "pass",
    effects = %effect_names,
    apply = %apply_desc,
    then_send = %then_send_desc,
    iteration = iterations,
    queue_depth = self.event_queue.len(),
);
```

**Test assertion pattern for parseable log lines:**

Tests can capture and assert on structured log output using the `testing` module's log capture:

```rust
#[test]
fn transition_produces_parseable_log() {
    let logs = capture_logs(|| {
        let mut svc = Service::<toggle::Machine>::new(toggle::Props { id: "btn-1".into(), ..Default::default() });
        svc.send(toggle::Event::Toggle);
    });

    // Assert the structured format is parseable
    let line = logs.iter().find(|l| l.contains("[ars:toggle#btn-1]")).expect("toggle transition should produce a log line");
    assert!(line.contains("Off + Toggle"), "expected state + event: {line}");
    assert!(line.contains("→ On"), "expected target state: {line}");
    assert!(line.contains("guard: pass"), "expected guard result: {line}");
    assert!(line.contains("effects: [notify_change]"), "expected effects list: {line}");
}
```

The `apply_description` field on `TransitionPlan` (§2.2) is the source of the human-readable apply description in trace output.

**Browser DevTools integration:**

On WASM targets, `console_log` routes `log` output to the browser console. Filter by `ars_core` in the console filter bar to isolate state machine traces. The `data-ars-state` attribute on component root elements (§3.2) provides a live, inspectable view of each machine's current state in the Elements panel — no additional tooling is required.

**Recommended debugging workflow:**

1. Enable `debug` feature on `ars-core` in dev profile: `ars-core = { features = ["debug"] }`.
2. Set `RUST_LOG=ars_core=trace` (native) or initialize `console_log` at trace level (WASM).
3. Reproduce the issue — trace output shows every event, transition, guard evaluation, and effect dispatch.
4. Inspect `data-ars-state` attributes in the browser to verify the machine reached the expected state.
5. Disable the `debug` feature for release builds — all logging compiles out to zero cost.

See `spec/testing/09-state-machine-correctness.md` for strategies on asserting transition sequences in automated tests.

## 3. The Connect Pattern

### 3.1 Typed Property Enums

String-keyed maps are error-prone: typos compile silently, there is no autocomplete, and no exhaustive matching. All property keys use typed enums for compile-time validation.

> **Interaction AttrMap outputs:** For the complete table of data attributes, ARIA attributes, inline styles, and event handler methods produced by each interaction (Press, Hover, Focus, Move, LongPress, Drag), see `05-interactions.md` §1.5.

#### 3.1.1 HtmlAttr

Keyed by [WHATWG HTML Living Standard — Attributes Index](https://html.spec.whatwg.org/multipage/indices.html#attributes-3). Extensible variants for `data-*` and `aria-*` namespaces. Covers all non-deprecated attributes from the WHATWG spec. The `style` attribute is intentionally omitted — inline styles are handled via `AttrMap::set_style(CssProperty, String)` which decomposes into individual CSS properties for type-safe merging.

```rust
use core::fmt;

/// [WAI-ARIA 1.2 States and Properties](https://www.w3.org/TR/wai-aria-1.2/#state_prop_def)
/// plus `aria-description` from WAI-ARIA 1.3.
/// Complete spec-compliant enumeration of all 49 ARIA attributes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AriaAttr {
    // --- Widget attributes (§6.6.1) ---
    ActiveDescendant,
    AutoComplete,
    Checked,
    Disabled,
    ErrorMessage,
    Expanded,
    HasPopup,
    Hidden,
    Invalid,
    KeyShortcuts,
    Label,
    LabelledBy,
    Level,
    Modal,
    MultiLine,
    MultiSelectable,
    Orientation,
    Placeholder,
    Pressed,
    ReadOnly,
    Required,
    RoleDescription,
    Selected,
    Sort,
    ValueMax,
    ValueMin,
    ValueNow,
    ValueText,

    // --- Live region attributes (§6.6.2) ---
    Atomic,
    Busy,
    Live,
    Relevant,

    // --- Drag-and-drop attributes (§6.6.3) ---
    DropEffect,
    Grabbed,

    // --- Relationship attributes (§6.6.4) ---
    ColCount,
    ColIndex,
    ColSpan,
    Controls,
    Current,
    DescribedBy,
    Description,
    Details,
    FlowTo,
    Owns,
    PosInSet,
    RowCount,
    RowIndex,
    RowSpan,
    SetSize,
}

impl AriaAttr {
    /// Returns the HTML attribute name as a static string slice.
    ///
    /// This is the single source of truth for ARIA attribute name strings.
    /// Both `Display` and `as_str()` return the same string.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ActiveDescendant => "aria-activedescendant",
            Self::AutoComplete => "aria-autocomplete",
            Self::Checked => "aria-checked",
            Self::Disabled => "aria-disabled",
            Self::ErrorMessage => "aria-errormessage",
            Self::Expanded => "aria-expanded",
            Self::HasPopup => "aria-haspopup",
            Self::Hidden => "aria-hidden",
            Self::Invalid => "aria-invalid",
            Self::KeyShortcuts => "aria-keyshortcuts",
            Self::Label => "aria-label",
            Self::LabelledBy => "aria-labelledby",
            Self::Level => "aria-level",
            Self::Modal => "aria-modal",
            Self::MultiLine => "aria-multiline",
            Self::MultiSelectable => "aria-multiselectable",
            Self::Orientation => "aria-orientation",
            Self::Placeholder => "aria-placeholder",
            Self::Pressed => "aria-pressed",
            Self::ReadOnly => "aria-readonly",
            Self::Required => "aria-required",
            Self::RoleDescription => "aria-roledescription",
            Self::Selected => "aria-selected",
            Self::Sort => "aria-sort",
            Self::ValueMax => "aria-valuemax",
            Self::ValueMin => "aria-valuemin",
            Self::ValueNow => "aria-valuenow",
            Self::ValueText => "aria-valuetext",
            Self::Atomic => "aria-atomic",
            Self::Busy => "aria-busy",
            Self::Live => "aria-live",
            Self::Relevant => "aria-relevant",
            Self::DropEffect => "aria-dropeffect",
            Self::Grabbed => "aria-grabbed",
            Self::ColCount => "aria-colcount",
            Self::ColIndex => "aria-colindex",
            Self::ColSpan => "aria-colspan",
            Self::Controls => "aria-controls",
            Self::Current => "aria-current",
            Self::DescribedBy => "aria-describedby",
            Self::Description => "aria-description",
            Self::Details => "aria-details",
            Self::FlowTo => "aria-flowto",
            Self::Owns => "aria-owns",
            Self::PosInSet => "aria-posinset",
            Self::RowCount => "aria-rowcount",
            Self::RowIndex => "aria-rowindex",
            Self::RowSpan => "aria-rowspan",
            Self::SetSize => "aria-setsize",
        }
    }
}

impl fmt::Display for AriaAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HtmlAttr {
    // --- Extensible namespaced attributes ---
    /// `data-*` attributes: `Data("ars-state")` → `data-ars-state`
    ///
    /// Uses `&'static str` because attribute name suffixes (e.g., `"ars-state"`, `"orientation"`)
    /// are compile-time constants defined by component specs — never user-generated at runtime.
    /// This avoids per-instance `String` allocations and enables zero-cost equality comparisons.
    Data(&'static str),
    /// `aria-*` attributes: `Aria(AriaAttr::Label)` → `aria-label`
    ///
    /// Wraps the typed [`AriaAttr`] enum for compile-time validation of all 49 WAI-ARIA 1.2
    /// states and properties. Unlike `Data`, ARIA attributes are a closed, spec-defined set.
    Aria(AriaAttr),

    // --- Global attributes (WHATWG §3.2.6) ---
    AccessKey,
    AutoCapitalize,
    AutoCorrect,
    AutoFocus,
    Class,
    ContentEditable,
    Dir,
    Draggable,
    EnterKeyHint,
    Hidden,
    Id,
    Inert,
    InputMode,
    Is,
    ItemId,
    ItemProp,
    ItemRef,
    ItemScope,
    ItemType,
    Lang,
    Nonce,
    Popover,
    Role,
    Slot,
    SpellCheck,
    TabIndex,
    Title,
    Translate,
    WritingSuggestions,

    // --- Form attributes (WHATWG §4.10) ---
    Accept,
    AcceptCharset,
    Action,
    Alpha,          // input (color)
    AutoComplete,
    Capture,
    Checked,
    Cols,
    ColorSpace,     // input (color)
    Command,        // button (invoker commands)
    CommandFor,     // button (invoker commands)
    Disabled,
    DirName,
    EncType,
    For,
    Form,
    FormAction,
    FormEncType,
    FormMethod,
    FormNoValidate,
    FormTarget,
    High,
    List,
    Low,
    Max,
    MaxLength,
    Method,
    Min,
    MinLength,
    Multiple,
    Name,
    NoValidate,
    Optimum,
    Pattern,
    Placeholder,
    ReadOnly,
    Required,
    Rows,
    Selected,
    Size,
    Step,
    Type,
    Value,
    Wrap,

    // --- Scripting / metadata (WHATWG §4.2, §4.12) ---
    As,             // link (preload)
    Async,          // script
    Blocking,       // link, script, style
    Charset,        // meta
    Color,          // link
    Defer,          // script
    HttpEquiv,      // meta
    ImageSizes,     // link
    ImageSrcSet,    // link

    // --- Embedded content (WHATWG §4.7–4.8) ---
    Allow,
    Alt,
    AutoPlay,
    Controls,
    CrossOrigin,
    Decoding,
    Default,
    Download,
    FetchPriority,
    Height,
    Href,
    HrefLang,
    Integrity,
    IsMap,
    Kind,
    Label,
    Loading,
    Loop,
    Media,
    Muted,
    ObjectData,     // object `data` attr (named to avoid conflict with Data(&str) variant)
    Ping,
    PlaysInline,    // video
    Poster,
    Preload,
    ReferrerPolicy,
    Rel,
    Sandbox,
    Shape,          // area
    Sizes,
    Src,
    SrcDoc,
    SrcLang,
    SrcSet,
    Target,
    UseMap,
    Width,

    // --- Table attributes (WHATWG §4.9) ---
    Abbr,
    ColSpan,
    Headers,
    RowSpan,
    Scope,
    Span,

    // --- Template attributes (WHATWG §4.12.3) ---
    ShadowRootClonable,
    ShadowRootCustomElementRegistry,
    ShadowRootDelegatesFocus,
    ShadowRootMode,
    ShadowRootSerializable,

    // --- Other element-specific (WHATWG §4.3–4.13) ---
    Cite,
    ClosedBy,
    Content,
    Coords,
    DateTime,
    Open,
    Reversed,
    Start,
    Summary,

    // --- Non-standard but widely used ---
    WebkitDirectory,
}

impl fmt::Display for HtmlAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data(s) => write!(f, "data-{s}"),
            Self::Aria(a) => a.fmt(f),
            Self::AccessKey => f.write_str("accesskey"),
            Self::AutoCapitalize => f.write_str("autocapitalize"),
            Self::AutoCorrect => f.write_str("autocorrect"),
            Self::AutoFocus => f.write_str("autofocus"),
            Self::Class => f.write_str("class"),
            Self::ContentEditable => f.write_str("contenteditable"),
            Self::Dir => f.write_str("dir"),
            Self::Draggable => f.write_str("draggable"),
            Self::EnterKeyHint => f.write_str("enterkeyhint"),
            Self::Hidden => f.write_str("hidden"),
            Self::Id => f.write_str("id"),
            Self::Inert => f.write_str("inert"),
            Self::InputMode => f.write_str("inputmode"),
            Self::Is => f.write_str("is"),
            Self::ItemId => f.write_str("itemid"),
            Self::ItemProp => f.write_str("itemprop"),
            Self::ItemRef => f.write_str("itemref"),
            Self::ItemScope => f.write_str("itemscope"),
            Self::ItemType => f.write_str("itemtype"),
            Self::Lang => f.write_str("lang"),
            Self::Nonce => f.write_str("nonce"),
            Self::Popover => f.write_str("popover"),
            Self::Role => f.write_str("role"),
            Self::Slot => f.write_str("slot"),
            Self::SpellCheck => f.write_str("spellcheck"),
            Self::TabIndex => f.write_str("tabindex"),
            Self::Title => f.write_str("title"),
            Self::Translate => f.write_str("translate"),
            Self::WritingSuggestions => f.write_str("writingsuggestions"),
            Self::Accept => f.write_str("accept"),
            Self::AcceptCharset => f.write_str("accept-charset"),
            Self::Action => f.write_str("action"),
            Self::Alpha => f.write_str("alpha"),
            Self::AutoComplete => f.write_str("autocomplete"),
            Self::Capture => f.write_str("capture"),
            Self::Checked => f.write_str("checked"),
            Self::Cols => f.write_str("cols"),
            Self::ColorSpace => f.write_str("colorspace"),
            Self::Command => f.write_str("command"),
            Self::CommandFor => f.write_str("commandfor"),
            Self::Disabled => f.write_str("disabled"),
            Self::DirName => f.write_str("dirname"),
            Self::EncType => f.write_str("enctype"),
            Self::For => f.write_str("for"),
            Self::Form => f.write_str("form"),
            Self::FormAction => f.write_str("formaction"),
            Self::FormEncType => f.write_str("formenctype"),
            Self::FormMethod => f.write_str("formmethod"),
            Self::FormNoValidate => f.write_str("formnovalidate"),
            Self::FormTarget => f.write_str("formtarget"),
            Self::High => f.write_str("high"),
            Self::List => f.write_str("list"),
            Self::Low => f.write_str("low"),
            Self::Max => f.write_str("max"),
            Self::MaxLength => f.write_str("maxlength"),
            Self::Method => f.write_str("method"),
            Self::Min => f.write_str("min"),
            Self::MinLength => f.write_str("minlength"),
            Self::Multiple => f.write_str("multiple"),
            Self::Name => f.write_str("name"),
            Self::NoValidate => f.write_str("novalidate"),
            Self::Optimum => f.write_str("optimum"),
            Self::Pattern => f.write_str("pattern"),
            Self::Placeholder => f.write_str("placeholder"),
            Self::ReadOnly => f.write_str("readonly"),
            Self::Required => f.write_str("required"),
            Self::Rows => f.write_str("rows"),
            Self::Selected => f.write_str("selected"),
            Self::Size => f.write_str("size"),
            Self::Step => f.write_str("step"),
            Self::Type => f.write_str("type"),
            Self::Value => f.write_str("value"),
            Self::Wrap => f.write_str("wrap"),
            Self::As => f.write_str("as"),
            Self::Async => f.write_str("async"),
            Self::Blocking => f.write_str("blocking"),
            Self::Charset => f.write_str("charset"),
            Self::Color => f.write_str("color"),
            Self::Defer => f.write_str("defer"),
            Self::HttpEquiv => f.write_str("http-equiv"),
            Self::ImageSizes => f.write_str("imagesizes"),
            Self::ImageSrcSet => f.write_str("imagesrcset"),
            Self::Allow => f.write_str("allow"),
            Self::Alt => f.write_str("alt"),
            Self::AutoPlay => f.write_str("autoplay"),
            Self::Controls => f.write_str("controls"),
            Self::CrossOrigin => f.write_str("crossorigin"),
            Self::Decoding => f.write_str("decoding"),
            Self::Default => f.write_str("default"),
            Self::Download => f.write_str("download"),
            Self::FetchPriority => f.write_str("fetchpriority"),
            Self::Height => f.write_str("height"),
            Self::Href => f.write_str("href"),
            Self::HrefLang => f.write_str("hreflang"),
            Self::Integrity => f.write_str("integrity"),
            Self::IsMap => f.write_str("ismap"),
            Self::Kind => f.write_str("kind"),
            Self::Label => f.write_str("label"),
            Self::Loading => f.write_str("loading"),
            Self::Loop => f.write_str("loop"),
            Self::Media => f.write_str("media"),
            Self::Muted => f.write_str("muted"),
            Self::ObjectData => f.write_str("data"),
            Self::Ping => f.write_str("ping"),
            Self::PlaysInline => f.write_str("playsinline"),
            Self::Poster => f.write_str("poster"),
            Self::Preload => f.write_str("preload"),
            Self::ReferrerPolicy => f.write_str("referrerpolicy"),
            Self::Rel => f.write_str("rel"),
            Self::Sandbox => f.write_str("sandbox"),
            Self::Shape => f.write_str("shape"),
            Self::Sizes => f.write_str("sizes"),
            Self::Src => f.write_str("src"),
            Self::SrcDoc => f.write_str("srcdoc"),
            Self::SrcLang => f.write_str("srclang"),
            Self::SrcSet => f.write_str("srcset"),
            Self::Target => f.write_str("target"),
            Self::UseMap => f.write_str("usemap"),
            Self::Width => f.write_str("width"),
            Self::Abbr => f.write_str("abbr"),
            Self::ColSpan => f.write_str("colspan"),
            Self::Headers => f.write_str("headers"),
            Self::RowSpan => f.write_str("rowspan"),
            Self::Scope => f.write_str("scope"),
            Self::Span => f.write_str("span"),
            Self::ShadowRootClonable => f.write_str("shadowrootclonable"),
            Self::ShadowRootCustomElementRegistry => f.write_str("shadowrootcustomelementregistry"),
            Self::ShadowRootDelegatesFocus => f.write_str("shadowrootdelegatesfocus"),
            Self::ShadowRootMode => f.write_str("shadowrootmode"),
            Self::ShadowRootSerializable => f.write_str("shadowrootserializable"),
            Self::Cite => f.write_str("cite"),
            Self::ClosedBy => f.write_str("closedby"),
            Self::Content => f.write_str("content"),
            Self::Coords => f.write_str("coords"),
            Self::DateTime => f.write_str("datetime"),
            Self::Open => f.write_str("open"),
            Self::Reversed => f.write_str("reversed"),
            Self::Start => f.write_str("start"),
            Self::Summary => f.write_str("summary"),
            Self::WebkitDirectory => f.write_str("webkitdirectory"),
        }
    }
}

impl HtmlAttr {
    /// Return the HTML attribute name as a `&'static str` when it is a
    /// compile-time constant, or `None` for variants that require formatting.
    ///
    /// All variants except `Data(_)` return `Some`. `Data(&'static str)` requires
    /// formatting `"data-{suffix}"` which produces a heap `String`; adapters that
    /// need `&'static str` for `Data` attributes must intern the result (see
    /// `09-adapter-dioxus.md` §3.1 `intern_attr_name`).
    pub fn static_name(&self) -> Option<&'static str> {
        match self {
            Self::Data(_) => None,
            Self::Aria(a) => Some(a.as_str()),
            Self::AccessKey => Some("accesskey"),
            Self::AutoCapitalize => Some("autocapitalize"),
            Self::AutoCorrect => Some("autocorrect"),
            Self::AutoFocus => Some("autofocus"),
            Self::Class => Some("class"),
            Self::ContentEditable => Some("contenteditable"),
            Self::Dir => Some("dir"),
            Self::Draggable => Some("draggable"),
            Self::EnterKeyHint => Some("enterkeyhint"),
            Self::Hidden => Some("hidden"),
            Self::Id => Some("id"),
            Self::Inert => Some("inert"),
            Self::InputMode => Some("inputmode"),
            Self::Is => Some("is"),
            Self::ItemId => Some("itemid"),
            Self::ItemProp => Some("itemprop"),
            Self::ItemRef => Some("itemref"),
            Self::ItemScope => Some("itemscope"),
            Self::ItemType => Some("itemtype"),
            Self::Lang => Some("lang"),
            Self::Nonce => Some("nonce"),
            Self::Popover => Some("popover"),
            Self::Role => Some("role"),
            Self::Slot => Some("slot"),
            Self::SpellCheck => Some("spellcheck"),
            Self::TabIndex => Some("tabindex"),
            Self::Title => Some("title"),
            Self::Translate => Some("translate"),
            Self::WritingSuggestions => Some("writingsuggestions"),
            // ... remaining ~80 variants elided for brevity.
            // Each returns `Some("same-string-as-Display-impl")`.
            // The full implementation mirrors the `Display` match above:
            //   Self::Accept => Some("accept"),
            //   Self::Action => Some("action"),
            //   ... etc.
            // Every non-Data, non-Aria variant MUST return `Some`.
            // ... remaining ~80 variants each return Some("their-kebab-name")
            // Full implementation delegates to the Display trait.
        }
    }
}
```

##### 3.1.1.1 URL Sanitization

All URL-valued attributes (`HtmlAttr::Href`, `HtmlAttr::Action`,
`HtmlAttr::FormAction`) must be validated before rendering to prevent URL
injection attacks such as `javascript:`, `data:`, or `vbscript:` schemes.

```rust
/// Check whether a URL is safe for use in `href`, `action`, or `formaction`
/// attributes.
///
/// Allows `http://`, `https://`, `mailto:`, `tel:`, `/`, `./`, `../`, `#`,
/// `?`, and relative paths with no scheme separator. Rejects dangerous or
/// unknown schemes.
pub fn is_safe_url(url: &str) -> bool {
    let trimmed = url.trim_start().as_bytes();

    fn starts_with_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.len() >= needle.len()
            && haystack[..needle.len()]
                .iter()
                .zip(needle)
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
    }

    starts_with_ignore_case(trimmed, b"http://")
        || starts_with_ignore_case(trimmed, b"https://")
        || starts_with_ignore_case(trimmed, b"mailto:")
        || starts_with_ignore_case(trimmed, b"tel:")
        || trimmed.first() == Some(&b'/')
        || trimmed.first() == Some(&b'#')
        || trimmed.first() == Some(&b'?')
        || starts_with_ignore_case(trimmed, b"./")
        || starts_with_ignore_case(trimmed, b"../")
        || !trimmed.contains(&b':')
}

/// Sanitize a URL, returning `"#"` for unsafe URLs.
pub fn sanitize_url(url: &str) -> &str {
    if is_safe_url(url) { url } else { "#" }
}

/// A validated URL newtype.
///
/// Components that store URLs in Props or Context should prefer `SafeUrl` over
/// raw `String` so validation happens once at the boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SafeUrl(String);

impl SafeUrl {
    /// Create a new `SafeUrl`, returning `Err` when the URL uses a disallowed
    /// scheme.
    pub fn new(url: impl Into<String>) -> Result<Self, UnsafeUrlError> {
        let url = url.into();
        if is_safe_url(&url) {
            Ok(Self(url))
        } else {
            Err(UnsafeUrlError(url))
        }
    }

    /// Access the validated URL string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SafeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error returned when `SafeUrl::new()` receives a URL with a disallowed
/// scheme.
#[derive(Clone, Debug)]
pub struct UnsafeUrlError(pub String);

impl fmt::Display for UnsafeUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe URL scheme: {:?}", self.0)
    }
}
```

Components that set URL-valued attributes must call `sanitize_url()`:

- **Link**: `attrs.set(HtmlAttr::Href, sanitize_url(self.ctx.href.as_str()))`
- **Form**: `attrs.set(HtmlAttr::Action, sanitize_url(action))`

#### 3.1.2 HtmlEvent

All standard DOM element events per UI Events, Pointer Events, HTML, CSS Animations/Transitions specs.

```rust
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HtmlEvent {
    // --- Mouse Events (UI Events §5) ---
    AuxClick,
    Click,
    ContextMenu,
    DblClick,
    MouseDown,
    MouseEnter,
    MouseLeave,
    MouseMove,
    MouseOut,
    MouseOver,
    MouseUp,

    // --- Pointer Events (Pointer Events §5) ---
    GotPointerCapture,
    LostPointerCapture,
    PointerCancel,
    PointerDown,
    PointerEnter,
    PointerLeave,
    PointerMove,
    PointerOut,
    PointerOver,
    PointerUp,

    // --- Keyboard Events (UI Events §6) ---
    KeyDown,
    KeyUp,

    // --- Focus Events (UI Events §7) ---
    Blur,
    Focus,
    FocusIn,
    FocusOut,

    // --- Input & Form Events (HTML §4.10) ---
    Change,
    Input,
    BeforeInput,
    Invalid,
    Reset,
    Select,
    Submit,

    // --- Drag & Drop Events (HTML §6.11) ---
    Drag,
    DragEnd,
    DragEnter,
    DragLeave,
    DragOver,
    DragStart,
    Drop,

    // --- Touch Events (Touch Events §4) ---
    TouchCancel,
    TouchEnd,
    TouchMove,
    TouchStart,

    // --- Wheel & Scroll ---
    Scroll,
    /// `scrollend` requires Safari 17.4+. Adapters SHOULD provide a fallback
    /// using a debounced `scroll` listener (~150ms inactivity) for older browsers.
    ScrollEnd,
    Wheel,

    // --- Clipboard Events (Clipboard API §3) ---
    Copy,
    Cut,
    Paste,

    // --- Composition Events (UI Events §8) ---
    CompositionEnd,
    CompositionStart,
    CompositionUpdate,

    // --- Animation Events (CSS Animations §4) ---
    AnimationCancel,
    AnimationEnd,
    AnimationIteration,
    AnimationStart,

    // --- Transition Events (CSS Transitions §4) ---
    TransitionCancel,
    TransitionEnd,
    TransitionRun,
    TransitionStart,

    // --- Resource & Loading Events ---
    Abort,
    Error,
    Load,
    Resize,

    // --- Media Events (HTML §4.7.10.16) ---
    CanPlay,
    CanPlayThrough,
    DurationChange,
    Emptied,
    Ended,
    LoadedData,
    LoadedMetaData,
    LoadStart,
    Pause,
    Play,
    Playing,
    Progress,
    RateChange,
    Seeked,
    Seeking,
    Stalled,
    Suspend,
    TimeUpdate,
    VolumeChange,
    Waiting,

    // --- HTML Element Events ---
    Cancel,
    Close,
    FullscreenChange,
    FullscreenError,
    SelectionChange,
    SlotChange,
    Toggle,
}

impl fmt::Display for HtmlEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuxClick => f.write_str("auxclick"),
            Self::Click => f.write_str("click"),
            Self::ContextMenu => f.write_str("contextmenu"),
            Self::DblClick => f.write_str("dblclick"),
            Self::MouseDown => f.write_str("mousedown"),
            Self::MouseEnter => f.write_str("mouseenter"),
            Self::MouseLeave => f.write_str("mouseleave"),
            Self::MouseMove => f.write_str("mousemove"),
            Self::MouseOut => f.write_str("mouseout"),
            Self::MouseOver => f.write_str("mouseover"),
            Self::MouseUp => f.write_str("mouseup"),
            Self::GotPointerCapture => f.write_str("gotpointercapture"),
            Self::LostPointerCapture => f.write_str("lostpointercapture"),
            Self::PointerCancel => f.write_str("pointercancel"),
            Self::PointerDown => f.write_str("pointerdown"),
            Self::PointerEnter => f.write_str("pointerenter"),
            Self::PointerLeave => f.write_str("pointerleave"),
            Self::PointerMove => f.write_str("pointermove"),
            Self::PointerOut => f.write_str("pointerout"),
            Self::PointerOver => f.write_str("pointerover"),
            Self::PointerUp => f.write_str("pointerup"),
            Self::KeyDown => f.write_str("keydown"),
            Self::KeyUp => f.write_str("keyup"),
            Self::Blur => f.write_str("blur"),
            Self::Focus => f.write_str("focus"),
            Self::FocusIn => f.write_str("focusin"),
            Self::FocusOut => f.write_str("focusout"),
            Self::Change => f.write_str("change"),
            Self::Input => f.write_str("input"),
            Self::BeforeInput => f.write_str("beforeinput"),
            Self::Invalid => f.write_str("invalid"),
            Self::Reset => f.write_str("reset"),
            Self::Select => f.write_str("select"),
            Self::Submit => f.write_str("submit"),
            Self::Drag => f.write_str("drag"),
            Self::DragEnd => f.write_str("dragend"),
            Self::DragEnter => f.write_str("dragenter"),
            Self::DragLeave => f.write_str("dragleave"),
            Self::DragOver => f.write_str("dragover"),
            Self::DragStart => f.write_str("dragstart"),
            Self::Drop => f.write_str("drop"),
            Self::TouchCancel => f.write_str("touchcancel"),
            Self::TouchEnd => f.write_str("touchend"),
            Self::TouchMove => f.write_str("touchmove"),
            Self::TouchStart => f.write_str("touchstart"),
            Self::Scroll => f.write_str("scroll"),
            Self::ScrollEnd => f.write_str("scrollend"),
            Self::Wheel => f.write_str("wheel"),
            Self::Copy => f.write_str("copy"),
            Self::Cut => f.write_str("cut"),
            Self::Paste => f.write_str("paste"),
            Self::CompositionEnd => f.write_str("compositionend"),
            Self::CompositionStart => f.write_str("compositionstart"),
            Self::CompositionUpdate => f.write_str("compositionupdate"),
            Self::AnimationCancel => f.write_str("animationcancel"),
            Self::AnimationEnd => f.write_str("animationend"),
            Self::AnimationIteration => f.write_str("animationiteration"),
            Self::AnimationStart => f.write_str("animationstart"),
            Self::TransitionCancel => f.write_str("transitioncancel"),
            Self::TransitionEnd => f.write_str("transitionend"),
            Self::TransitionRun => f.write_str("transitionrun"),
            Self::TransitionStart => f.write_str("transitionstart"),
            Self::Abort => f.write_str("abort"),
            Self::Error => f.write_str("error"),
            Self::Load => f.write_str("load"),
            Self::Resize => f.write_str("resize"),
            Self::CanPlay => f.write_str("canplay"),
            Self::CanPlayThrough => f.write_str("canplaythrough"),
            Self::DurationChange => f.write_str("durationchange"),
            Self::Emptied => f.write_str("emptied"),
            Self::Ended => f.write_str("ended"),
            Self::LoadedData => f.write_str("loadeddata"),
            Self::LoadedMetaData => f.write_str("loadedmetadata"),
            Self::LoadStart => f.write_str("loadstart"),
            Self::Pause => f.write_str("pause"),
            Self::Play => f.write_str("play"),
            Self::Playing => f.write_str("playing"),
            Self::Progress => f.write_str("progress"),
            Self::RateChange => f.write_str("ratechange"),
            Self::Seeked => f.write_str("seeked"),
            Self::Seeking => f.write_str("seeking"),
            Self::Stalled => f.write_str("stalled"),
            Self::Suspend => f.write_str("suspend"),
            Self::TimeUpdate => f.write_str("timeupdate"),
            Self::VolumeChange => f.write_str("volumechange"),
            Self::Waiting => f.write_str("waiting"),
            Self::Cancel => f.write_str("cancel"),
            Self::Close => f.write_str("close"),
            Self::FullscreenChange => f.write_str("fullscreenchange"),
            Self::FullscreenError => f.write_str("fullscreenerror"),
            Self::SelectionChange => f.write_str("selectionchange"),
            Self::SlotChange => f.write_str("slotchange"),
            Self::Toggle => f.write_str("toggle"),
        }
    }
}
```

#### 3.1.3 CssProperty

Common CSS properties from CSS Box Model, Flexbox, Grid, Positioning, Typography, Transforms, Animations specs. Extensible for custom properties and uncommon standard properties.

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CssProperty {
    /// CSS custom properties: `Custom("ars-timer-progress")` → `--ars-timer-progress`
    Custom(&'static str),

    // --- Box Model (CSS Box Model §4) ---
    BoxSizing,
    Width,
    MinWidth,
    MaxWidth,
    Height,
    MinHeight,
    MaxHeight,
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Border,
    BorderWidth,
    BorderStyle,
    BorderColor,
    BorderRadius,
    BorderCollapse,
    BorderSpacing,

    // --- Logical Properties (CSS Logical Properties §2) ---
    InlineSize,
    BlockSize,
    MinInlineSize,
    MaxInlineSize,
    MinBlockSize,
    MaxBlockSize,
    MarginInline,
    MarginInlineStart,
    MarginInlineEnd,
    MarginBlock,
    MarginBlockStart,
    MarginBlockEnd,
    PaddingInline,
    PaddingInlineStart,
    PaddingInlineEnd,
    PaddingBlock,
    PaddingBlockStart,
    PaddingBlockEnd,
    InsetInlineStart,
    InsetInlineEnd,
    InsetBlockStart,
    InsetBlockEnd,

    // --- Positioning (CSS Position §2) ---
    Position,
    Top,
    Right,
    Bottom,
    Left,
    ZIndex,
    Float,
    Clear,

    // --- Display & Flexbox (CSS Display §2, Flexbox §4) ---
    Display,
    FlexDirection,
    FlexWrap,
    FlexFlow,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Order,
    AlignItems,
    AlignSelf,
    AlignContent,
    JustifyContent,
    JustifyItems,
    JustifySelf,
    PlaceItems,
    PlaceContent,
    Gap,
    RowGap,
    ColumnGap,

    // --- Grid (CSS Grid §7) ---
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridArea,
    GridAutoFlow,
    GridAutoColumns,
    GridAutoRows,

    // --- Typography (CSS Text §3, Fonts §3) ---
    Color,
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    LineHeight,
    TextAlign,
    TextDecoration,
    TextTransform,
    TextOverflow,
    TextIndent,
    TextShadow,
    WhiteSpace,
    WordBreak,
    WordWrap,
    OverflowWrap,
    LetterSpacing,
    WordSpacing,

    // --- Visual (CSS Backgrounds §3, Color §4) ---
    Background,
    BackgroundColor,
    BackgroundImage,
    BackgroundPosition,
    BackgroundSize,
    BackgroundRepeat,
    Opacity,
    Visibility,
    BoxShadow,
    Outline,
    OutlineWidth,
    OutlineStyle,
    OutlineColor,
    OutlineOffset,
    Cursor,
    PointerEvents,
    UserSelect,

    // --- Overflow & Clipping (CSS Overflow §3) ---
    Overflow,
    OverflowX,
    OverflowY,
    Clip,
    ClipPath,
    ScrollBehavior,
    ScrollSnapType,
    ScrollSnapAlign,
    OverscrollBehavior,

    // --- Transforms & Animations (CSS Transforms §2, Animations §3, Transitions §2) ---
    Transform,
    TransformOrigin,
    Transition,
    TransitionProperty,
    TransitionDuration,
    TransitionTimingFunction,
    TransitionDelay,
    Animation,
    AnimationName,
    AnimationDuration,
    AnimationTimingFunction,
    AnimationDelay,
    AnimationIterationCount,
    AnimationDirection,
    AnimationFillMode,
    AnimationPlayState,

    // --- Sizing & Containment ---
    AspectRatio,
    ObjectFit,
    ObjectPosition,
    Contain,
    ContentVisibility,
    WillChange,
    Appearance,
    Resize,
    TouchAction,
    Filter,
    BackdropFilter,

    // --- Content & Lists ---
    Content,
    ListStyle,
    ListStyleType,
    ListStylePosition,
    TableLayout,
    VerticalAlign,
}

impl fmt::Display for CssProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(s) => write!(f, "--{s}"),
            Self::BoxSizing => f.write_str("box-sizing"),
            Self::Width => f.write_str("width"),
            Self::MinWidth => f.write_str("min-width"),
            Self::MaxWidth => f.write_str("max-width"),
            Self::Height => f.write_str("height"),
            Self::MinHeight => f.write_str("min-height"),
            Self::MaxHeight => f.write_str("max-height"),
            Self::Margin => f.write_str("margin"),
            Self::MarginTop => f.write_str("margin-top"),
            Self::MarginRight => f.write_str("margin-right"),
            Self::MarginBottom => f.write_str("margin-bottom"),
            Self::MarginLeft => f.write_str("margin-left"),
            Self::Padding => f.write_str("padding"),
            Self::PaddingTop => f.write_str("padding-top"),
            Self::PaddingRight => f.write_str("padding-right"),
            Self::PaddingBottom => f.write_str("padding-bottom"),
            Self::PaddingLeft => f.write_str("padding-left"),
            Self::Border => f.write_str("border"),
            Self::BorderWidth => f.write_str("border-width"),
            Self::BorderStyle => f.write_str("border-style"),
            Self::BorderColor => f.write_str("border-color"),
            Self::BorderRadius => f.write_str("border-radius"),
            Self::BorderCollapse => f.write_str("border-collapse"),
            Self::BorderSpacing => f.write_str("border-spacing"),
            Self::InlineSize => f.write_str("inline-size"),
            Self::BlockSize => f.write_str("block-size"),
            Self::MinInlineSize => f.write_str("min-inline-size"),
            Self::MaxInlineSize => f.write_str("max-inline-size"),
            Self::MinBlockSize => f.write_str("min-block-size"),
            Self::MaxBlockSize => f.write_str("max-block-size"),
            Self::MarginInline => f.write_str("margin-inline"),
            Self::MarginInlineStart => f.write_str("margin-inline-start"),
            Self::MarginInlineEnd => f.write_str("margin-inline-end"),
            Self::MarginBlock => f.write_str("margin-block"),
            Self::MarginBlockStart => f.write_str("margin-block-start"),
            Self::MarginBlockEnd => f.write_str("margin-block-end"),
            Self::PaddingInline => f.write_str("padding-inline"),
            Self::PaddingInlineStart => f.write_str("padding-inline-start"),
            Self::PaddingInlineEnd => f.write_str("padding-inline-end"),
            Self::PaddingBlock => f.write_str("padding-block"),
            Self::PaddingBlockStart => f.write_str("padding-block-start"),
            Self::PaddingBlockEnd => f.write_str("padding-block-end"),
            Self::InsetInlineStart => f.write_str("inset-inline-start"),
            Self::InsetInlineEnd => f.write_str("inset-inline-end"),
            Self::InsetBlockStart => f.write_str("inset-block-start"),
            Self::InsetBlockEnd => f.write_str("inset-block-end"),
            Self::Position => f.write_str("position"),
            Self::Top => f.write_str("top"),
            Self::Right => f.write_str("right"),
            Self::Bottom => f.write_str("bottom"),
            Self::Left => f.write_str("left"),
            Self::ZIndex => f.write_str("z-index"),
            Self::Float => f.write_str("float"),
            Self::Clear => f.write_str("clear"),
            Self::Display => f.write_str("display"),
            Self::FlexDirection => f.write_str("flex-direction"),
            Self::FlexWrap => f.write_str("flex-wrap"),
            Self::FlexFlow => f.write_str("flex-flow"),
            Self::FlexGrow => f.write_str("flex-grow"),
            Self::FlexShrink => f.write_str("flex-shrink"),
            Self::FlexBasis => f.write_str("flex-basis"),
            Self::Order => f.write_str("order"),
            Self::AlignItems => f.write_str("align-items"),
            Self::AlignSelf => f.write_str("align-self"),
            Self::AlignContent => f.write_str("align-content"),
            Self::JustifyContent => f.write_str("justify-content"),
            Self::JustifyItems => f.write_str("justify-items"),
            Self::JustifySelf => f.write_str("justify-self"),
            Self::PlaceItems => f.write_str("place-items"),
            Self::PlaceContent => f.write_str("place-content"),
            Self::Gap => f.write_str("gap"),
            Self::RowGap => f.write_str("row-gap"),
            Self::ColumnGap => f.write_str("column-gap"),
            Self::GridTemplateColumns => f.write_str("grid-template-columns"),
            Self::GridTemplateRows => f.write_str("grid-template-rows"),
            Self::GridColumn => f.write_str("grid-column"),
            Self::GridRow => f.write_str("grid-row"),
            Self::GridArea => f.write_str("grid-area"),
            Self::GridAutoFlow => f.write_str("grid-auto-flow"),
            Self::GridAutoColumns => f.write_str("grid-auto-columns"),
            Self::GridAutoRows => f.write_str("grid-auto-rows"),
            Self::Color => f.write_str("color"),
            Self::FontFamily => f.write_str("font-family"),
            Self::FontSize => f.write_str("font-size"),
            Self::FontWeight => f.write_str("font-weight"),
            Self::FontStyle => f.write_str("font-style"),
            Self::LineHeight => f.write_str("line-height"),
            Self::TextAlign => f.write_str("text-align"),
            Self::TextDecoration => f.write_str("text-decoration"),
            Self::TextTransform => f.write_str("text-transform"),
            Self::TextOverflow => f.write_str("text-overflow"),
            Self::TextIndent => f.write_str("text-indent"),
            Self::TextShadow => f.write_str("text-shadow"),
            Self::WhiteSpace => f.write_str("white-space"),
            Self::WordBreak => f.write_str("word-break"),
            Self::WordWrap => f.write_str("word-wrap"),
            Self::OverflowWrap => f.write_str("overflow-wrap"),
            Self::LetterSpacing => f.write_str("letter-spacing"),
            Self::WordSpacing => f.write_str("word-spacing"),
            Self::Background => f.write_str("background"),
            Self::BackgroundColor => f.write_str("background-color"),
            Self::BackgroundImage => f.write_str("background-image"),
            Self::BackgroundPosition => f.write_str("background-position"),
            Self::BackgroundSize => f.write_str("background-size"),
            Self::BackgroundRepeat => f.write_str("background-repeat"),
            Self::Opacity => f.write_str("opacity"),
            Self::Visibility => f.write_str("visibility"),
            Self::BoxShadow => f.write_str("box-shadow"),
            Self::Outline => f.write_str("outline"),
            Self::OutlineWidth => f.write_str("outline-width"),
            Self::OutlineStyle => f.write_str("outline-style"),
            Self::OutlineColor => f.write_str("outline-color"),
            Self::OutlineOffset => f.write_str("outline-offset"),
            Self::Cursor => f.write_str("cursor"),
            Self::PointerEvents => f.write_str("pointer-events"),
            Self::UserSelect => f.write_str("user-select"),
            Self::Overflow => f.write_str("overflow"),
            Self::OverflowX => f.write_str("overflow-x"),
            Self::OverflowY => f.write_str("overflow-y"),
            Self::Clip => f.write_str("clip"),
            Self::ClipPath => f.write_str("clip-path"),
            Self::ScrollBehavior => f.write_str("scroll-behavior"),
            Self::ScrollSnapType => f.write_str("scroll-snap-type"),
            Self::ScrollSnapAlign => f.write_str("scroll-snap-align"),
            Self::OverscrollBehavior => f.write_str("overscroll-behavior"),
            Self::Transform => f.write_str("transform"),
            Self::TransformOrigin => f.write_str("transform-origin"),
            Self::Transition => f.write_str("transition"),
            Self::TransitionProperty => f.write_str("transition-property"),
            Self::TransitionDuration => f.write_str("transition-duration"),
            Self::TransitionTimingFunction => f.write_str("transition-timing-function"),
            Self::TransitionDelay => f.write_str("transition-delay"),
            Self::Animation => f.write_str("animation"),
            Self::AnimationName => f.write_str("animation-name"),
            Self::AnimationDuration => f.write_str("animation-duration"),
            Self::AnimationTimingFunction => f.write_str("animation-timing-function"),
            Self::AnimationDelay => f.write_str("animation-delay"),
            Self::AnimationIterationCount => f.write_str("animation-iteration-count"),
            Self::AnimationDirection => f.write_str("animation-direction"),
            Self::AnimationFillMode => f.write_str("animation-fill-mode"),
            Self::AnimationPlayState => f.write_str("animation-play-state"),
            Self::AspectRatio => f.write_str("aspect-ratio"),
            Self::ObjectFit => f.write_str("object-fit"),
            Self::ObjectPosition => f.write_str("object-position"),
            Self::Contain => f.write_str("contain"),
            Self::ContentVisibility => f.write_str("content-visibility"),
            Self::WillChange => f.write_str("will-change"),
            Self::Appearance => f.write_str("appearance"),
            Self::Resize => f.write_str("resize"),
            Self::TouchAction => f.write_str("touch-action"),
            Self::Filter => f.write_str("filter"),
            Self::BackdropFilter => f.write_str("backdrop-filter"),
            Self::Content => f.write_str("content"),
            Self::ListStyle => f.write_str("list-style"),
            Self::ListStyleType => f.write_str("list-style-type"),
            Self::ListStylePosition => f.write_str("list-style-position"),
            Self::TableLayout => f.write_str("table-layout"),
            Self::VerticalAlign => f.write_str("vertical-align"),
        }
    }
}
```

### 3.2 AttrMap

The output of every `*_attrs()` method. Framework-agnostic attribute map containing only data — no event handlers. Clone and serializable for SSR.

Event handlers are **not** stored in `AttrMap`. Instead, each component's `Api` struct exposes typed handler methods (e.g., `on_root_click()`, `on_root_keydown(key, shift)`) that adapters wire to their native event system.

**SSR serialization boundary:** “serializable for SSR” means `AttrMap` can be serialized on the
server for SSR-oriented output pipelines (for example, HTML rendering helpers, debug snapshots, or
test fixtures). It is **not** the hydration payload. During hydration, the client reads attributes
from the rendered DOM, not by deserializing `AttrMap` from JSON. JSON round-trip hydration applies
to machine state snapshots (see adapter hydration specs), not to the rendered attribute map itself.

When the `serde` feature is enabled, `AttrMap` derives `Serialize` only. This is intentional:
deserializing `AttrMap` would suggest it participates in the JSON hydration boundary, but the
client reconstructs DOM-facing attributes from rendered HTML and current machine state instead.

````rust
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    /// String attribute value.
    String(String),

    /// Boolean attribute (present or absent).
    Bool(bool),

    /// Attribute should be removed.
    None,
}

impl AttrValue {
    /// Returns the string representation of this value, or `None` for non-string variants.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            Self::Bool(true) => Some("true"),
            Self::Bool(false) => Some("false"),
            Self::None => None,
        }
    }
}

/// HTML attributes whose values are space-separated token lists.
///
/// `AttrMap::set()` automatically appends (with dedup) rather than replaces
/// when the target attribute is in this set, matching the semantics defined
/// by the HTML Living Standard and WAI-ARIA 1.2.
///
/// References:
///   - `class`: HTML Living Standard §3.2.6
///     https://html.spec.whatwg.org/multipage/dom.html#classes
///   - `rel`: HTML Living Standard §4.6.6
///     https://html.spec.whatwg.org/multipage/links.html#linkTypes
///   - ARIA ID reference lists (`aria-labelledby`, `aria-describedby`,
///     `aria-owns`, `aria-controls`, `aria-flowto`, `aria-details`):
///     WAI-ARIA 1.2 §6.2 — https://w3c.github.io/aria/#state_prop_def
const SPACE_SEPARATED: &[HtmlAttr] = &[
    HtmlAttr::Class,
    HtmlAttr::Rel,
    HtmlAttr::Aria(AriaAttr::LabelledBy),
    HtmlAttr::Aria(AriaAttr::DescribedBy),
    HtmlAttr::Aria(AriaAttr::Owns),
    HtmlAttr::Aria(AriaAttr::Controls),
    HtmlAttr::Aria(AriaAttr::FlowTo),
    HtmlAttr::Aria(AriaAttr::Details),
];

/// Framework-agnostic attribute map. Contains only data — no event handlers.
/// Clone and serializable for SSR.
///
/// Uses sorted `Vec` instead of `BTreeMap` for lower overhead — component
/// attr maps are small (typically <15 entries) and built once per render.
///
/// **Why O(n) insertion is acceptable:** `AttrMap` entries are built by `connect()` which
/// adds attributes in a known order. The sorted vec uses `binary_search` for O(log n) lookup
/// and O(n) insertion (shifting elements). For maps with <15 entries, the linear shift is
/// cheaper in practice than `BTreeMap`'s node allocations and pointer chasing, because the
/// entire vec fits in one or two cache lines (each entry is ~40 bytes, so 15 entries ≈ 600
/// bytes). Benchmarks should confirm this if component attr counts grow beyond ~20.
///
/// ## Space-separated token list semantics
///
/// For attributes defined as space-separated token lists by the HTML Living
/// Standard or WAI-ARIA 1.2 (e.g., `class`, `rel`, `aria-labelledby`),
/// `set()` automatically appends new tokens with deduplication rather than
/// replacing the existing value. This means:
///
/// ```rust
/// attrs.set(HtmlAttr::Class, "ars-visually-hidden");
/// attrs.set(HtmlAttr::Class, "ars-touch-none");
/// // Result: class="ars-visually-hidden ars-touch-none"
/// ```
///
/// See `SPACE_SEPARATED` for the complete list and spec references.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttrMap {
    // kept sorted by key for O(log n) lookup via binary search
    attrs: Vec<(HtmlAttr, AttrValue)>,
    // kept sorted by key for O(log n) lookup via binary search
    styles: Vec<(CssProperty, String)>,
}

/// Destructured parts of an `AttrMap`, returned by [`AttrMap::into_parts()`].
///
/// Fields are `pub` for zero-copy consumption by adapter conversion functions
/// (e.g., `attr_map_to_leptos`, `attr_map_to_dioxus`). Unlike `AttrMap` itself,
/// there is no sorted-vec invariant to protect — `AttrMapParts` is a terminal,
/// consumed value.
pub struct AttrMapParts {
    pub attrs: Vec<(HtmlAttr, AttrValue)>,
    pub styles: Vec<(CssProperty, String)>,
}

impl AttrMap {
    pub fn new() -> Self { Self::default() }

    /// Consume this `AttrMap` into its raw parts for adapter-layer conversion.
    ///
    /// Use this when you need to consume the vecs (e.g., `.into_iter()`)
    /// without cloning. The returned `AttrMapParts` has public fields since
    /// it is a terminal value with no invariants to protect.
    pub fn into_parts(self) -> AttrMapParts {
        AttrMapParts { attrs: self.attrs, styles: self.styles }
    }

    /// Read-only access to the attribute entries.
    pub fn attrs(&self) -> &[(HtmlAttr, AttrValue)] {
        &self.attrs
    }

    /// Read-only access to the style entries.
    pub fn styles(&self) -> &[(CssProperty, String)] {
        &self.styles
    }

    /// Set an attribute on the map.
    ///
    /// For most attributes, this replaces any existing value (last-write-wins).
    /// If `value` is `AttrValue::None`, the entry is removed.
    ///
    /// For **space-separated token list attributes** (`class`, `rel`, and ARIA
    /// ID reference lists like `aria-labelledby`), values are automatically
    /// appended with deduplication rather than replaced. This matches the
    /// semantics defined by the HTML Living Standard
    /// (https://html.spec.whatwg.org/multipage/dom.html#classes for `class`,
    /// https://html.spec.whatwg.org/multipage/links.html#linkTypes for `rel`)
    /// and WAI-ARIA 1.2 (https://w3c.github.io/aria/#state_prop_def for ID
    /// reference lists). See `SPACE_SEPARATED` for the complete list.
    pub fn set(&mut self, attr: HtmlAttr, value: impl Into<AttrValue>) -> &mut Self {
        let value = value.into();
        let space_sep = SPACE_SEPARATED.contains(&attr);
        match self.attrs.binary_search_by(|(k, _)| k.cmp(&attr)) {
            Ok(idx) => {
                if matches!(value, AttrValue::None) {
                    self.attrs.remove(idx);
                } else if space_sep {
                    // Append with dedup for space-separated token lists.
                    if let (AttrValue::String(existing), AttrValue::String(new_val)) =
                        (&self.attrs[idx].1, &value)
                    {
                        if !existing.split_whitespace().any(|t| t == new_val.as_str()) {
                            self.attrs[idx].1 =
                                AttrValue::String(format!("{existing} {new_val}"));
                        }
                    } else {
                        // Mixed types (e.g., Bool + String): replace outright.
                        // Space-separated dedup only applies when both are String.
                        self.attrs[idx].1 = value;
                    }
                } else {
                    self.attrs[idx].1 = value;
                }
            }
            Err(idx) => {
                if !matches!(value, AttrValue::None) {
                    self.attrs.insert(idx, (attr, value));
                }
            }
        }
        self
    }

    pub fn set_style(&mut self, prop: CssProperty, value: impl Into<String>) -> &mut Self {
        let value = value.into();
        match self.styles.binary_search_by(|(k, _)| k.cmp(&prop)) {
            Ok(idx) => self.styles[idx].1 = value,
            Err(idx) => self.styles.insert(idx, (prop, value)),
        }
        self
    }

    /// Convenience: sets a boolean attribute.
    pub fn set_bool(&mut self, attr: HtmlAttr, value: bool) -> &mut Self {
        self.set(attr, AttrValue::Bool(value))
    }

    /// Check whether the map contains a given attribute key.
    pub fn contains(&self, attr: &HtmlAttr) -> bool {
        self.attrs.binary_search_by(|(k, _)| k.cmp(attr)).is_ok()
    }

    /// Look up an attribute's string value by key.
    /// Returns `None` if the attribute is absent or is `AttrValue::None`.
    pub fn get(&self, attr: &HtmlAttr) -> Option<&str> {
        self.attrs.binary_search_by(|(k, _)| k.cmp(attr))
            .ok()
            .and_then(|i| self.attrs[i].1.as_str())
    }

    /// Look up an attribute's typed value by key.
    pub fn get_value(&self, attr: &HtmlAttr) -> Option<&AttrValue> {
        self.attrs.binary_search_by(|(k, _)| k.cmp(attr))
            .ok()
            .map(|i| &self.attrs[i].1)
    }

    /// Iterate over all attribute key-value pairs.
    pub fn iter_attrs(&self) -> impl Iterator<Item = &(HtmlAttr, AttrValue)> {
        self.attrs.iter()
    }

    /// Iterate over attribute pairs. Similar to `iter_attrs()` but yields
    /// `(&HtmlAttr, &AttrValue)` instead of `&(HtmlAttr, AttrValue)`.
    pub fn iter(&self) -> impl Iterator<Item = (&HtmlAttr, &AttrValue)> {
        self.attrs.iter().map(|(k, v)| (k, v))
    }

    /// Iterate over attribute keys only.
    pub fn keys(&self) -> impl Iterator<Item = &HtmlAttr> {
        self.attrs.iter().map(|(k, _)| k)
    }

    /// Iterate over all style property-value pairs.
    pub fn iter_styles(&self) -> impl Iterator<Item = &(CssProperty, String)> {
        self.styles.iter()
    }

    /// Merge another (trusted) AttrMap into this one.
    ///
    /// For most attributes, later values override earlier (last-write-wins).
    /// For space-separated token list attributes, values are appended
    /// (handled automatically by `set()`).
    pub fn merge(&mut self, other: AttrMap) {
        for (k, v) in other.attrs {
            self.set(k, v);
        }
        for (k, v) in other.styles {
            self.set_style(k, v);
        }
    }

    /// Merge user-provided attribute extensions.
    ///
    /// No filtering needed — `UserAttrs` enforces the security blocklist at
    /// construction time, so the inner `AttrMap` is guaranteed safe to merge.
    /// Space-separated attributes are handled automatically by `set()`.
    pub fn merge_user(&mut self, user: UserAttrs) {
        self.merge(user.0);
    }
}

/// User-provided attribute extensions.
///
/// Enforces a security blocklist at construction time — blocked attributes
/// are silently rejected by `set()`, so a `UserAttrs` value can never
/// contain attributes that would break component semantics or accessibility.
/// This makes invalid state unrepresentable.
///
/// `UserAttrs` does NOT implement `Into<AttrMap>` or `Deref<Target = AttrMap>`,
/// so it cannot be accidentally merged via the trusted `AttrMap::merge()`.
///
/// ## Blocked attributes
///
/// The following attributes are rejected by `set()` because they are
/// structurally enforced by the component's state machine or accessibility
/// contract:
///
/// - `id` — component identity, used for `aria-owns`/`aria-controls` wiring
/// - `role` — ARIA role determined by component spec
/// - `aria-hidden` — visibility toggling is state-machine-driven
/// - `aria-modal` — modal semantics are structurally enforced
/// - `tabindex` — focus management is component-controlled
/// - `aria-live` — live-region politeness is set by the component
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserAttrs(AttrMap);

/// Attributes that users cannot override. Enforced at `UserAttrs::set()` time
/// so that a `UserAttrs` value can never contain these — invalid state is
/// unrepresentable.
const USER_BLOCKED: &[HtmlAttr] = &[
    HtmlAttr::Id,
    HtmlAttr::Role,
    HtmlAttr::Aria(AriaAttr::Hidden),
    HtmlAttr::Aria(AriaAttr::Modal),
    HtmlAttr::TabIndex,
    HtmlAttr::Aria(AriaAttr::Live),
];

impl UserAttrs {
    pub fn new() -> Self { Self::default() }

    /// Set an attribute. Blocked attributes (see `USER_BLOCKED`) are
    /// silently ignored — they can never enter a `UserAttrs`.
    pub fn set(&mut self, attr: HtmlAttr, value: impl Into<AttrValue>) -> &mut Self {
        if USER_BLOCKED.contains(&attr) {
            return self;
        }
        self.0.set(attr, value);
        self
    }

    pub fn set_style(&mut self, prop: CssProperty, value: impl Into<String>) -> &mut Self {
        self.0.set_style(prop, value);
        self
    }

    pub fn set_bool(&mut self, attr: HtmlAttr, value: bool) -> &mut Self {
        if USER_BLOCKED.contains(&attr) {
            return self;
        }
        self.0.set_bool(attr, value);
        self
    }
}

/// Convenience constructor for data attributes.
pub const fn data(name: &'static str) -> HtmlAttr {
    HtmlAttr::Data(name)
}

/// Options for event listener registration.
/// Used by adapters when wiring typed handler methods to native DOM events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventOptions {
    pub passive: bool,
    pub capture: bool,
}

impl From<&str> for AttrValue {
    fn from(s: &str) -> Self { AttrValue::String(s.to_string()) }
}

impl From<String> for AttrValue {
    fn from(s: String) -> Self { AttrValue::String(s) }
}

impl From<&String> for AttrValue {
    fn from(s: &String) -> Self { AttrValue::String(s.clone()) }
}

impl From<bool> for AttrValue {
    fn from(b: bool) -> Self { AttrValue::Bool(b) }
}
````

#### 3.2.1 StyleStrategy

`AttrMap.styles` produces inline `style` attributes, which are incompatible with strict CSP (`style-src` without `'unsafe-inline'`). To support CSP-strict environments, adapters provide a configurable `StyleStrategy` that controls how `AttrMap.styles` are rendered to the DOM.

```rust
/// Controls how `AttrMap.styles` are applied to elements.
/// Configured via `ArsProvider` context (`ArsContext.style_strategy`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum StyleStrategy {
    /// Render styles as inline `style` attributes (default).
    /// Requires `style-src 'unsafe-inline'` or no CSP.
    #[default]
    Inline,
    /// Apply styles at runtime via `CSSStyleDeclaration` (CSSOM API).
    /// No inline style attributes are emitted; works with any `style-src` policy.
    /// Requires JavaScript; not compatible with SSR initial paint.
    Cssom,
    /// Emit a `<style nonce="...">` block containing scoped CSS rules.
    /// Each element with dynamic styles gets a `data-ars-style-id` attribute;
    /// styles are collected into a single `<style>` element with the provided nonce.
    /// Compatible with `style-src 'nonce-xxx'` CSP and SSR.
    Nonce(String),
}
```

| Strategy | CSP requirement           | SSR support                       | Trade-off                                                   |
| -------- | ------------------------- | --------------------------------- | ----------------------------------------------------------- |
| `Inline` | `'unsafe-inline'` or none | Full                              | Simplest; no extra setup                                    |
| `Cssom`  | Any `style-src`           | No (styles apply after hydration) | Flash of unstyled dynamic positioning until JS runs         |
| `Nonce`  | `'nonce-xxx'`             | Full                              | Requires per-request nonce generation; slightly larger HTML |

`ars-core` owns the nonce rule formatting helpers because they operate only on
framework-agnostic `CssProperty` entries and stable `data-ars-style-id`
selectors. Adapters must reuse these helpers instead of duplicating CSS escaping
logic.

```rust
/// Escape a string for use as a quoted CSS attribute selector value.
///
/// Used for selectors such as `[data-ars-style-id="..."]`.
pub fn escape_css_attribute_value(value: &str) -> String;

/// Convert inline style entries into a nonce-compatible CSS rule targeting
/// `data-ars-style-id`.
pub fn styles_to_nonce_css(id: &str, styles: &[(CssProperty, String)]) -> String;
```

Companion stylesheet classes (e.g., `ars-visually-hidden`, `ars-touch-none`) are stored as `HtmlAttr::Class` in the `attrs` vec via `set(HtmlAttr::Class, "ars-...")`. Because `class` is a space-separated token list, `set()` automatically appends new class names with deduplication. The `class` attribute is always rendered regardless of the active `StyleStrategy`.

#### 3.2.2 Companion Stylesheet: `ars-base.css`

Static, unchanging styles (visually-hidden, screen-reader-only input, touch-action suppression) are defined as utility classes in a companion stylesheet. This avoids inline styles entirely for these patterns, making them CSP-safe in **all** strategies.

```css
/* ars-base.css — companion stylesheet for ars-core
 * Published alongside the ars-core crate. ~500 bytes uncompressed.
 * Include via <link rel="stylesheet" href="ars-base.css"> in the document <head>.
 */

/* Visually hidden but accessible to screen readers.
 * Used by: VisuallyHidden (non-focusable variant), LiveAnnouncer. */
.ars-visually-hidden {
    position: absolute !important;
    border: 0 !important;
    width: 1px !important;
    height: 1px !important;
    padding: 0 !important;
    margin: -1px !important;
    overflow: hidden !important;
    clip: rect(0, 0, 0, 0) !important;
    white-space: nowrap !important;
    word-wrap: normal !important;
}

/* Screen-reader-only native input (hidden but participates in form submission).
 * Used by: RadioGroup hidden input, Switch hidden input, FileUpload hidden input. */
.ars-sr-input {
    position: absolute !important;
    width: 1px !important;
    height: 1px !important;
    overflow: hidden !important;
    clip: rect(0, 0, 0, 0) !important;
}

/* Suppress browser touch gestures (pan, pinch-zoom) on drag targets.
 * Used by: Slider thumb, Splitter handle, use_move targets. */
.ars-touch-none {
    touch-action: none !important;
}
```

**Distribution:** `ars-base.css` is published alongside the `ars-core` crate. Applications MUST include it via a `<link>` element. The file is ~500 bytes uncompressed and contains only the three classes above.

For build systems that prefer programmatic asset collection, `ars-core` may also expose an
opt-in embedded stylesheet constant behind a dedicated feature flag. The sidecar file remains the
default delivery path so consumers do not pay binary-size cost unless they explicitly opt in.

**`!important` rationale:** These classes enforce accessibility and interaction invariants that must not be accidentally overridden by application stylesheets. `.ars-visually-hidden` losing `overflow: hidden` would make the element visible; `.ars-touch-none` losing `touch-action: none` would break drag interactions on touch devices.

#### 3.2.3 Callback Naming Convention

Adapter-level callbacks (not in core Props) follow a consistent naming pattern:

- `on_value_change: Callback<T>` — value changed
- `on_open_change: Callback<bool>` — open/closed state changed
- `on_focus_change: Callback<bool>` — focus state changed
- `on_checked_change: Callback<checkbox::State>` — checked state changed
- `on_selection_change: Callback<Selection>` — selection changed

Pattern: `on_{property}_change`. Always use the property name, not the event name.

#### 3.2.4 `data-ars-state` Kebab-Case Rule

All `data-ars-state` attribute values **must** be kebab-case. This is enforced by
the `Display` impl on each component's `State` enum:

```rust
// CORRECT:  data-ars-state="half-open"
// WRONG:    data-ars-state="HalfOpen" or data-ars-state="half_open"
```

### 3.3 Prop Getter Pattern

Each component's `Api` struct provides two kinds of methods per anatomy part:

1. **`*_attrs()` methods** — return `AttrMap` with ARIA attributes, roles, data attributes, and inline styles. These are pure data, serializable for SSR.
2. **Typed handler methods** — `on_*_click()`, `on_*_keydown(key, shift)`, etc. These dispatch machine events. Adapters wire them to native framework event handlers.

```rust
pub mod checkbox {
    #[derive(ComponentPart)]
    #[scope = "checkbox"]
    pub enum Part {
        Root,
        Control,
    }

    pub struct Api<'a> {
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    }

    impl<'a> Api<'a> {
        pub fn root_attrs(&self) -> AttrMap { /* ... */ }
        pub fn control_attrs(&self) -> AttrMap { /* ... */ }

        pub fn on_control_click(&self) {
            (self.send)(Event::Toggle);
        }

        pub fn on_control_keydown(&self, key: &str, _shift: bool) {
            match key {
                " " => (self.send)(Event::Toggle),
                _ => {}
            }
        }
    }

    impl ConnectApi for Api<'_> {
        type Part = Part;

        fn part_attrs(&self, part: Self::Part) -> AttrMap {
            match part {
                Part::Root => self.root_attrs(),
                Part::Control => self.control_attrs(),
            }
        }
    }
}
```

Example for a dialog component with multiple parts:

```rust
pub mod dialog {
    #[derive(ComponentPart)]
    #[scope = "dialog"]
    pub enum Part {
        Root,
        Trigger,
        Backdrop,
        Positioner,
        Content,
        Title,
        Description,
        CloseTrigger,
    }

    impl<'a> Api<'a> {
        // --- AttrMap getters (data only, SSR-safe) ---
        pub fn root_attrs(&self) -> AttrMap { /* ... */ }
        pub fn trigger_attrs(&self) -> AttrMap { /* ... */ }
        pub fn backdrop_attrs(&self) -> AttrMap { /* ... */ }
        pub fn positioner_attrs(&self) -> AttrMap { /* ... */ }
        pub fn content_attrs(&self) -> AttrMap { /* ... */ }
        pub fn title_attrs(&self) -> AttrMap { /* ... */ }
        pub fn description_attrs(&self) -> AttrMap { /* ... */ }
        pub fn close_trigger_attrs(&self) -> AttrMap { /* ... */ }

        // --- Typed handler methods (adapters wire to native events) ---
        pub fn on_trigger_click(&self) { /* ... */ }
        pub fn on_trigger_keydown(&self, key: &str, _shift: bool) { /* ... */ }
        pub fn on_backdrop_click(&self) { /* ... */ }
        pub fn on_content_keydown(&self, key: &str, _shift: bool) { /* ... */ }
        pub fn on_close_trigger_click(&self) { /* ... */ }

        // --- Computed state accessors ---
        pub fn is_open(&self) -> bool { /* ... */ }

        // --- Event options for handlers that need passive/capture ---
        pub fn on_content_scroll_options(&self) -> EventOptions {
            EventOptions { passive: true, capture: false }
        }
    }

    impl ConnectApi for Api<'_> {
        type Part = Part;

        fn part_attrs(&self, part: Self::Part) -> AttrMap {
            match part {
                Part::Root => self.root_attrs(),
                Part::Trigger => self.trigger_attrs(),
                Part::Backdrop => self.backdrop_attrs(),
                Part::Positioner => self.positioner_attrs(),
                Part::Content => self.content_attrs(),
                Part::Title => self.title_attrs(),
                Part::Description => self.description_attrs(),
                Part::CloseTrigger => self.close_trigger_attrs(),
            }
        }
    }
}
```

When a handler requires non-default listener options (e.g., passive scroll/touch listeners),
the Api exposes a companion `*_options()` method returning `EventOptions`. Adapters use this
when registering the native event listener (e.g., `addEventListener(type, handler, options)`).

## 4. Anatomy System

### 4.1 Anatomy Definition

A component's anatomy is defined by its `Part` enum — a typed enumeration of all named DOM
parts. The `#[derive(ComponentPart)]` macro (from `ars-derive`) generates the `ComponentPart`
trait implementation, which provides the scope name, part names, enumeration, and data attribute
helpers. See the `ComponentPart` trait definition in §2.1.

```rust
/// Example: Accordion anatomy via Part enum (data-carrying variants).
/// Each repeated part carries the IDs needed for ARIA cross-references.
#[derive(ComponentPart)]
#[scope = "accordion"]
pub enum Part {
    Root,
    Item(String),                       // (item_id)
    ItemTrigger(String, String),        // (item_id, content_id) → "item-trigger"
    ItemIndicator(String),              // (item_id) → "item-indicator"
    ItemContent(String, String, String),// (item_id, content_id, trigger_id) → "item-content"
}

/// Example: Dialog anatomy via Part enum (unit variants only).
#[derive(ComponentPart)]
#[scope = "dialog"]
pub enum Part {
    Root,
    Trigger,
    Backdrop,
    Positioner,
    Content,
    Title,
    Description,
    CloseTrigger,   // → "close-trigger"
}

/// Example: Tabs anatomy via Part enum (mixed unit and data variants).
#[derive(ComponentPart)]
#[scope = "tabs"]
pub enum Part {
    Root,
    List,
    Tab(String, String),        // (tab_id, panel_id)
    TabIndicator,               // → "tab-indicator"
    Panel(String, String),      // (panel_id, tab_id)
}
```

#### `#[derive(ComponentPart)]` Macro Specification

**Proc-macro crate:** `ars-derive` (re-exported from `ars-core` via `#[doc(inline)]`).

**Input requirements:**

- The enum must have `#[scope = "kebab-case-name"]` attribute.
- The first variant MUST be `Root` and MUST be a unit variant.
- Variants may be unit variants or carry data fields.
- All field types must implement `Default` (used by `all()` to generate representative instances).

**Generated implementation:**

- `const ROOT: Self = Self::Root` — first variant (always a unit variant).
- `fn scope() -> &'static str` — returns the `#[scope]` value.
- `fn name(&self) -> &'static str` — PascalCase → kebab-case conversion
  (e.g., `ItemGroupLabel` → `"item-group-label"`). For data-carrying variants,
  the match arm uses a wildcard pattern (e.g., `Self::Item(..) => "item"`),
  ignoring field data.
- `fn all() -> Vec<Self>` — one instance per variant in declaration order. Unit
  variants are included directly; data-carrying variants use
  `Default::default()` for each field (strum `EnumIter` pattern).
- `fn data_attrs(&self)` — default method on `ComponentPart`, not generated (inherited).

**Additionally derives:** `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.

**Generated `all()` example:**

```rust
// For Part { Root, Item(String), Tab(String, String) }:
fn all() -> Vec<Self> {
    vec![
        Self::Root,
        Self::Item(Default::default()),
        Self::Tab(Default::default(), Default::default()),
    ]
}
```

### 4.2 Common Anatomy Patterns

**Overlays**: `Root > Trigger > Positioner > Content > [specific parts]`

**Inputs with dropdowns**: `Root > Label > Control > Trigger > Positioner > Content > ItemGroup > Item`

**Form controls**: `Root > Control > Label > Input(hidden) > Description > ErrorMessage`

**Value display**: `Root > Label > ValueText > Control > Track > Range > Thumb`

## 5. Cross-Crate Shared Patterns

> This section documents shared types and contracts that span multiple crates (`ars-a11y`, `ars-core`, `ars-dom`). For the complete `ars-dom` API reference, see [`11-dom-utilities.md`](11-dom-utilities.md).

### 5.1 ID Contract

Adapters provide a hydration-safe base ID via `Props::id: String` (required field). The adapter obtains this from its own ID generation utility (e.g., `use_id()` in ars-leptos — an `AtomicU32` counter, NOT a Leptos built-in; Dioxus uses an equivalent scope/hook-based ID), ensuring deterministic SSR/hydration matching.

Core derives part IDs using `ComponentIds` defined in `ars-core` (see `03-accessibility.md` §2.6 for the full API). This thin wrapper stores a base ID string and derives part/item names dynamically:

- `ids.id()` — returns the base ID (for the root element)
- `ids.part("trigger")` — returns `"{base}-trigger"` (for fixed structural parts)
- `ids.item("item", &key)` — returns `"{base}-item-{key}"` (for per-item IDs in collections; `key` is `impl Display`)
- `ids.item_part("item", &key, "text")` — returns `"{base}-item-{key}-text"` (for sub-elements within a keyed item)

SSR Idempotency: `props.id` MUST be stable across SSR/hydration. Adapters MUST assign deterministic IDs (`ars-leptos`: `use_id()` AtomicU32 counter; `ars-dioxus`: equivalent scope-based ID). IDs derived at Service creation never change. Note: `use_id()` is an ars-ui utility, NOT a Leptos built-in.

```rust
// Convention: part IDs are always `{base_id}-{part_name}`
// Example: base_id = "ars-3" produces:
//   - "ars-3-control"   (ids.part("control"))
//   - "ars-3-label"     (ids.part("label"))
//   - "ars-3-description" (ids.part("description"))

// In a machine's init():
fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
    let ids = ComponentIds::from_id(props.id());
    (State::default(), Context {
        trigger_id: ids.part("trigger"),
        content_id: ids.part("content"),
        label_id: ids.part("label"),
        // Only the IDs this component actually needs
        ..
    })
}

// In connect(), reference the derived IDs from context:
// attrs.set(HtmlAttr::Id, &ctx.control_id);
// attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &ctx.label_id);
```

The adapter provides a hydration-safe base ID via `Props::id`:

```rust
// Adapter side (Leptos example):
#[component]
pub fn Checkbox(/* ... */) -> impl IntoView {
    let id = use_id("checkbox"); // ars-leptos utility (AtomicU32 counter), NOT a Leptos built-in
    let props = checkbox::Props {
        id,
        // ...
    };
    // ...
}
```

### 5.2 Common Shared Types

```rust
/// Layout orientation for components like Slider, Splitter, Tabs, Toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Horizontal => f.write_str("horizontal"),
            Self::Vertical => f.write_str("vertical"),
        }
    }
}

// `Direction` — defined in `04-internationalization.md` (includes `Ltr`, `Rtl`, `Auto`)
// Adapters must resolve `Direction::Auto` to a concrete `Ltr`/`Rtl` via the document's
// computed direction before passing to the positioning engine.
```

### 5.3 Subsystem Summary

See `11-dom-utilities.md` for all details:

- **Positioning Engine** (§2) — `compute_position()`, `auto_update()`, all positioning types
- **Focus Utilities** (§3) — `get_focusable_elements()`, `focus_element()`, `FocusScope`
- **Scroll Management** (§4) — `scroll_into_view_if_needed()`, `nearest_scrollable_ancestor()`
- **Scroll Locking** (§5) — `ScrollLockManager`, `prevent_scroll()`/`restore_scroll()`
- **Z-Index Management** (§6) — `next_z_index()`, `ZIndexAllocator`
- **Portal Root** (§7) — `get_or_create_portal_root()`, `set_background_inert()`
- **Modality Manager** (§8) — `ModalityManager`
- **Media Queries** (§9) — `is_forced_colors_active()`, `prefers_reduced_motion()`
- **URL Sanitization** [§3.1.1.1](#3111-url-sanitization) — `sanitize_url()`, `SafeUrl`

## 6. Adapter Architecture

### 6.1 `use_machine` Pattern

Each adapter provides a `use_machine` function:

```rust
// Pseudocode — the actual signature depends on the framework
fn use_machine<M: Machine>(props: M::Props) -> (MachineApi<M>, ReadSignal<M::State>) {
    // 1. Create the service (once, not on re-render)
    // 2. Wrap service state in reactive signal
    // 3. Set up event sender that updates signal on state change
    // 4. Return the connect API and state signal
}
```

### 6.2 Compound Component Pattern

Both Leptos and Dioxus support context-based compound components:

```text
Root component:
  1. Calls use_machine(props) to create service
  2. Provides machine API via context (provide_context / use_context_provider)
  3. Renders children

Child component (e.g., Trigger):
  1. Consumes machine API from context (use_context)
  2. Calls api.trigger_attrs() to get AttrMap
  3. Spreads AttrMap onto rendered element, wires typed handler methods
  4. Renders children
```

### 6.3 Children and Slots

| Concept         | Leptos                                      | Dioxus                                      |
| --------------- | ------------------------------------------- | ------------------------------------------- |
| Children        | `Children` (`Box<dyn FnOnce() -> AnyView>`) | `Element`                                   |
| Named slots     | `#[slot]` macro                             | Not natively supported — use separate props |
| Render callback | `Box<dyn Fn(ApiState) -> View>`             | `Box<dyn Fn(ApiState) -> Element>`          |
| Context passing | `provide_context` / `use_context`           | `use_context_provider` / `use_context`      |

### 6.4 ArsProvider

`ArsProvider` is the **single root provider** for the ars-ui library. It supplies shared
configuration, platform capabilities, provider-scoped modality state, i18n resources, and style strategy to all descendant
components. It MUST be rendered at (or near) the application root.

`ArsProvider` subsumes the formerly separate `LocaleProvider`, `PlatformEffectsProvider`,
`IntlBackend`, `I18nProvider`, and `ArsStyleProvider`. Components access configuration
via `ArsContext` fields and convenience hooks (e.g., `use_locale()`,
`use_platform_effects()`, `use_modality_context()`, `use_style_strategy()`).

#### 6.4.1 ColorMode

```rust
/// Active color mode for theme-aware rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    Light,
    Dark,
    #[default]
    System,
}
```

#### 6.4.2 Provided Context Values

| Value                 | Type                                        | Description                                                                    |
| --------------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| `locale`              | `ars_i18n::Locale`                          | The active locale for i18n message formatting and text direction inference.    |
| `direction`           | `Direction` (`Ltr` \| `Rtl`)                | Explicit reading direction override. Defaults to locale-inferred direction.    |
| `color_mode`          | `ColorMode` (`Light` \| `Dark` \| `System`) | Active color mode for theme-aware rendering.                                   |
| `disabled`            | `bool`                                      | When `true`, all descendant interactive components render as disabled.         |
| `read_only`           | `bool`                                      | When `true`, all descendant form fields render as read-only.                   |
| `id_prefix`           | `Option<String>`                            | Optional prefix prepended to all generated IDs (for micro-frontend isolation). |
| `portal_container_id` | `Option<String>`                            | ID of the container element for portal mounts. `None` means platform default.  |
| `root_node_id`        | `Option<String>`                            | ID of the root node for focus scope and portal queries. `None` means default.  |
| `platform`            | `Arc<dyn PlatformEffects>`                  | Platform capabilities for side effects. Defaults to `NullPlatformEffects`.     |
| `modality`            | `Arc<dyn ModalityContext>`                  | Shared input-modality state for the current provider root.                     |
| `intl_backend`        | `Arc<dyn IntlBackend>`                      | Calendar/locale data for date-time components. Defaults to `StubIntlBackend`.  |
| `i18n_registries`     | `Arc<I18nRegistries>`                       | Per-component translation registries. Defaults to empty (English fallbacks).   |
| `style_strategy`      | `StyleStrategy`                             | CSS style injection strategy. Defaults to `StyleStrategy::Inline`.             |

Components access configuration via `ctx.locale`, `ctx.direction`, etc. Convenience
hooks read from `ArsContext` with fallback defaults:

Adapter crates publish reactive `ArsContext` wrappers that mirror this canonical
field set and may add framework-specific extras (for example Dioxus-only
platform services).

- `use_locale()` — locale, falls back to `en-US`
- `use_platform_effects()` — platform capabilities
- `use_modality_context()` — shared input-modality state
- `use_intl_backend()` — calendar/locale data
- `use_style_strategy()` — CSS style strategy, falls back to `Inline`
- `resolve_messages::<M>()` — translation registries (pure function, not a hook — takes `&I18nRegistries` explicitly)

#### 6.4.3 Environment Resolution Rule

Locale, messages, and ICU provider are **environment context** provided by `ArsProvider`.
Core component `Props` structs MUST NOT contain `locale`, `messages`, or `intl_backend`
fields — these are resolved by the adapter and passed explicitly via the `Env` struct
and `Messages` parameter.

Adapter component Props (in `ars-leptos` / `ars-dioxus`) MAY accept
`locale: Option<Locale>` and `messages: Option<Messages>` as optional override props.
The three-level resolution chain (prop override → ArsProvider → default) lives entirely
in adapter code. See `04-internationalization.md` §2.3.1 for the adapter-side pattern.

**How locale reaches connect functions:** The adapter resolves locale from `ArsProvider`
context and passes it to `Machine::init()` via the `Env` struct. The `init()` function
stores `env.locale` in the machine's `Context` struct (e.g., `ctx.locale: Locale`).
Connect functions then pass `&self.ctx.locale` to `MessageFn` closures when resolving
translatable strings. For stateless components (no `Machine`), the adapter passes
`&Env` directly to the `Api` struct or individual parameters to standalone functions.
See `04-internationalization.md` §7.1 for the full pattern.

## 7. Error Handling Strategy

- State machine transitions that are invalid for the current state return `None` (silently ignored, like Zag.js) — no `Result` type needed
- Props validation errors are compile-time where possible (builder pattern with required fields)
- DOM errors are logged via `web_sys::console` in debug builds, silently handled in release
- Debug assertions (`debug_assert!`) catch programming errors during development
- The `drain_queue()` loop panics (debug) or logs a warning via `log::warn!` and breaks (release) after 100 iterations to prevent infinite loops

## 8. Testing Strategy

### 8.1 Unit Testing State Machines

Every machine is testable without a DOM:

```rust
use toggle::{self, State, Event, Props};

#[test]
fn toggle_transitions() {
    let props = Props {
        id: "test-toggle".to_string(),
        pressed: None,
        default_pressed: false,
        disabled: false,
    };
    let mut service = Service::<toggle::Machine>::new(props);
    assert_eq!(*service.state(), State::Off);

    let result = service.send(Event::Toggle);
    assert!(result.state_changed);
    assert!(result.pending_effects.is_empty());
    assert_eq!(*service.state(), State::On);

    let result = service.send(Event::Toggle);
    assert!(result.state_changed);
    assert_eq!(*service.state(), State::Off);
}

#[test]
fn toggle_disabled() {
    let props = Props {
        id: "test-toggle-disabled".to_string(),
        pressed: None,
        default_pressed: false,
        disabled: true,
    };
    let mut service = Service::<toggle::Machine>::new(props);
    let result = service.send(Event::Toggle);
    assert!(!result.state_changed); // No transition when disabled
    assert_eq!(*service.state(), State::Off);
}
```

### 8.2 Integration Testing

- Use `wasm-bindgen-test` for DOM-based tests in a headless browser
- Test that connect output produces correct ARIA attributes
- Test keyboard event handling end-to-end
- Snapshot test AttrMap output for regression

### 8.3 Cross-Framework Test Matrix

Each component should be tested in:

1. Pure state machine (no DOM)
2. Leptos integration (with DOM via wasm-bindgen-test)
3. Dioxus integration (with DOM via wasm-bindgen-test)
4. SSR output (Leptos SSR, Dioxus SSR)

## 9. SSR Architectural Principles

Server-side rendering is a first-class concern. These principles apply to all components and adapters.

### 9.1 ARIA Attributes from `connect()`, Never from Effects

All ARIA attributes, roles, and states **must** be computable from `(State, Context, Props)` alone and returned by `connect()` via `AttrMap`. They must appear in the server-rendered HTML.

DOM manipulation effects (focus management, scroll lock, event listeners) are acceptable as client-only operations because they have no impact on the initial accessibility tree.

**Consequence**: If an ARIA attribute (e.g., `aria-hidden`, `aria-expanded`, `role`, `inert`) is set via a client-side `PendingEffect`, screen reader users experience a broken page during the hydration gap (100ms–2s). This is a compliance violation.

### 9.2 IDs from the Framework

Component IDs are provided by the adapter via `Props::id: String` using the framework's hydration-safe ID system:

| Framework | ID Source                                                            |
| --------- | -------------------------------------------------------------------- |
| Leptos    | `use_id()` (ars-leptos) — `AtomicU32` counter, NOT a Leptos built-in |
| Dioxus    | Scope/hook ID from Dioxus runtime                                    |

Core never auto-generates IDs. All `aria-labelledby`, `aria-describedby`, and `aria-controls` references use IDs derived inline via `format!("{id}-{part}")` where `id` comes from `props.id`.

### 9.3 Callbacks Are Client-Only

Props contain only data — no callback fields (e.g., `on_checked_change`, `on_value_change`). Change notification callbacks live in the adapter layer using the `Callback<T>` type (§2.2) in both Leptos and Dioxus.

During SSR, there is no user interaction, so callbacks don't exist. This avoids `!Send + !Sync` issues with `web_sys` types that cannot cross thread boundaries.

### 9.4 Effects Are Skipped During SSR

Adapters gate effect setup with `#[cfg(not(feature = "ssr"))]`:

```rust
// In adapter's use_machine hook:
let result = service.send(event);
if result.state_changed {
    state_write.set(service.state().clone());
}

// Effects are client-only
#[cfg(not(feature = "ssr"))]
{
    // 1. Process explicit effect cancellations FIRST.
    //    cancel_effects contains names of effects to tear down immediately.
    //    The adapter tracks active effects by name in a HashMap.
    for name in &result.cancel_effects {
        if let Some(cleanup) = named_effect_cleanups.remove(name) {
            cleanup();
        }
    }

    // 2. Clean up effects from previous state BEFORE setting up new ones.
    //
    // IMPORTANT: The cleanup condition must be:
    //   state_changed || !result.pending_effects.is_empty()
    //
    // Context-only transitions (where state doesn't change but effects are
    // emitted via `TransitionPlan::context_only().with_effect(...)`) still
    // produce pending_effects that must be processed. Using only
    // `state_changed` as the cleanup condition would cause these effects to
    // accumulate without their predecessors being cleaned up.
    if result.state_changed || !result.pending_effects.is_empty() {
        for (_, cleanup) in named_effect_cleanups.drain() {
            cleanup();
        }
    }

    // 3. Set up effects for the new state.
    // NOTE: This is internal adapter code — the Arc construction is an
    // implementation detail hidden from component authors, who receive
    // WeakSend<M::Event> via PendingEffect::new().
    let send_rc: Arc<dyn Fn(M::Event) + Send + Sync> = Arc::new(move |e| send_callback(e));
    for effect in result.pending_effects {
        // Named effects auto-cancel previous effects with the same name.
        if let Some(old_cleanup) = named_effect_cleanups.remove(effect.name) {
            old_cleanup();
        }
        let name = effect.name;
        let cleanup = effect.run(service.context(), service.props(), send_rc.clone());
        named_effect_cleanups.insert(name, cleanup);
    }
}
```

Timer effects, DOM effects, and focus effects are all no-ops on the server. The adapter simply ignores `pending_effects` during SSR.

### 9.5 Live Region Containers in SSR HTML

Screen readers only track mutations to `aria-live` regions that existed in the accessibility tree when the page loaded. Components that use live regions (Toast, LiveRegion) **must** render the container element with its `aria-live` attribute in the server HTML:

```html
<!-- CORRECT: Container exists in SSR HTML -->
<div aria-live="polite" role="status">
    <!-- Content added dynamically after hydration -->
</div>

<!-- INCORRECT: Container created by client JS after hydration -->
```

## 10. Extensibility

### 10.1 Third-Party Machine Implementations

External crates can implement the `Machine` trait to create custom components that integrate with ars-ui adapters. The public API surface for third-party authors:

- `Machine` trait + `TransitionPlan` + `PendingEffect` (from `ars-core`)
- `AttrMap`, `HtmlAttr`, `CssProperty`, `AttrValue` (from `ars-core`)
- `HasId` trait and `#[derive(HasId)]` (from `ars-core` / `ars-derive`)
- `ComponentPart` trait and `#[derive(ComponentPart)]` for declaring component parts
- Adapter `use_machine` hooks accept any `M: Machine`

### 10.2 `data-ars-scope` Namespacing

Third-party components **must** use a vendor prefix in their `data-ars-scope` values to avoid collisions with built-in components:

```text
data-ars-scope="mylib-fancy-picker"    // third-party — prefixed
data-ars-scope="accordion"             // built-in — no prefix
```

Built-in scopes are reserved. The canonical list is defined by the component catalog.

### 10.3 API Stability

- `Machine` trait and its associated types are **stable** after v1.0.
- `HtmlAttr`, `HtmlEvent`, `CssProperty` enums are `#[non_exhaustive]` — new variants may be added in minor releases.
- `AttrMap`'s fields are private to protect the sorted-vec invariant. Use `attrs()` and `styles()` for read access, `into_parts()` for zero-copy consuming conversion, and `set()`/`set_style()` for mutation.
- `UserAttrs` is the designated type for user-provided attribute extensions. It enforces a security blocklist at construction time and can only be merged via `AttrMap::merge_user()`.
- `TransitionPlan` builder methods are stable; struct fields may become private in future versions.
