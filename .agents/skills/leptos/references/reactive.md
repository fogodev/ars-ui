# Leptos 0.8.17 — Reactive System Reference

Covers: `reactive_graph` (signals, memos, effects) and `reactive_stores` (field-level reactive stores).

## Signals

Signals are the atomic unit of reactive state. Arena-allocated and `Copy`.

### Creation

```rust
use leptos::prelude::*;

// Pair: separate read + write handles (both Copy)
let (count, set_count) = signal(0i32);          // requires T: Send + Sync
let (count, set_count) = signal_local(0i32);    // for !Send types (browser JS objects)

// Unified: single handle with read + write
let count = RwSignal::new(0i32);                // requires T: Send + Sync
let count = RwSignal::new_local(0i32);          // for !Send types

// Arc-based: Clone (not Copy), for use outside component trees
let (count, set_count) = arc_signal(0i32);
let count = ArcRwSignal::new(0i32);
```

### Reading

| Method               | Behavior                             |
| -------------------- | ------------------------------------ |
| `.get()`             | Clone value + subscribe to changes   |
| `.get_untracked()`   | Clone value, no subscription         |
| `.read()`            | Returns guard (no clone) + subscribe |
| `.with(\|val\| ...)` | Borrow via closure + subscribe       |

### Writing

| Method                 | Behavior                                |
| ---------------------- | --------------------------------------- |
| `.set(value)`          | Replace value + notify subscribers      |
| `.update(\|val\| ...)` | Mutate in-place via closure + notify    |
| `.write()`             | Returns mutable guard, notifies on drop |

### Signal Type Summary

| Type                | Description                          |
| ------------------- | ------------------------------------ |
| `ReadSignal<T>`     | Arena, read-only (from `signal()`)   |
| `WriteSignal<T>`    | Arena, write-only (from `signal()`)  |
| `RwSignal<T>`       | Arena, read + write combined         |
| `ArcReadSignal<T>`  | Ref-counted, read-only               |
| `ArcWriteSignal<T>` | Ref-counted, write-only              |
| `ArcRwSignal<T>`    | Ref-counted, read + write            |
| `Trigger`           | Arena, data-less notification signal |
| `ArcTrigger`        | Ref-counted, data-less notification  |
| `MappedSignal<T>`   | Arena, derived with mapping function |
| `Signal<T>`         | Type-erased wrapper (see below)      |

## `Signal<T>` (Type-Erased Wrapper)

`Signal<T, S = SyncStorage>` is a **type-erasing wrapper** that can hold any arena-allocated reactive signal: `ReadSignal`, `Memo`, `RwSignal`, a derived signal closure, or a plain static value. It allows APIs to accept `T` or any reactive `T` without generic parameters.

### Construction

```rust
// From any signal type (via Into/From)
let (count, _) = signal(0i32);
let sig: Signal<i32> = count.into();

// Derived: closure re-runs on every read (NOT cached like Memo)
let doubled = Signal::derive(move || count.get() * 2);

// Static: wraps a non-reactive value (backed by ArcStoredValue)
// Use when wrapping constant data that never changes.
let fallback = Signal::stored(Locale::default());

// Local variants (for !Send types)
let local_derived = Signal::derive_local(move || count.get() * 2);
let local_stored = Signal::stored_local(value);
```

### Method Signatures

```rust
// Derived signal — re-evaluates closure on every .get()/.read()/.with()
pub fn derive(f: impl Fn() -> T + Send + Sync + 'static) -> Signal<T>
where T: Send + Sync + 'static;

// Static value — wraps a constant, never re-evaluates
pub fn stored(value: T) -> Signal<T>
where T: Send + Sync + 'static;

// Local variants (no Send + Sync requirement)
pub fn derive_local(f: impl Fn() -> T + 'static) -> Signal<T, LocalStorage>
where T: 'static;

pub fn stored_local(value: T) -> Signal<T, LocalStorage>
where T: 'static;
```

### From Conversions

`Signal<T>` implements `From` for: `ReadSignal<T>`, `RwSignal<T>`, `Memo<T>`, `ArcSignal<T>`, `ArcReadSignal<T>`, `ArcRwSignal<T>`, `ArcMemo<T>`, `MaybeSignal<T>`, `MappedSignal<T>`, and plain `T`.

### derive vs stored vs Memo

| Constructor           | Caches?      | Re-evaluates on read? | Use case                                  |
| --------------------- | ------------ | --------------------- | ----------------------------------------- |
| `Signal::derive(f)`   | No           | Yes, every time       | Cheap computations, simple wrappers       |
| `Signal::stored(v)`   | N/A (static) | No (constant)         | Fallback/default values that never change |
| `Memo::new(f).into()` | Yes          | Only when deps change | Expensive computations needing caching    |

**Key:** If you want the closure to run minimally, use `Memo::new()` and convert to `Signal` via `.into()`. `Signal::derive()` re-runs the closure on every access.

### Common Pattern: API Parameters

```rust
// Accept any signal-like value as a parameter
#[component]
pub fn ProgressBar(
    #[prop(into)] progress: Signal<f64>,
) -> impl IntoView {
    view! { <div style:width=move || format!("{}%", progress.get() * 100.0) /> }
}

// Callers can pass any reactive source:
let (val, _) = signal(0.5);
view! { <ProgressBar progress=val /> }        // ReadSignal → Signal
view! { <ProgressBar progress=0.75 /> }       // plain f64 → Signal::stored
view! { <ProgressBar progress=move || 0.9 /> } // closure → Signal::derive
```

## Memos (Cached Derived Values)

Recompute only when dependencies change. Only notify downstream if the output actually changed.

```rust
let doubled = Memo::new(move |_| count.get() * 2);
let val = doubled.get(); // subscribe + clone

// Arc version (Clone, not Copy)
let arc_memo = ArcMemo::new(move |_| count.get() * 2);
```

### Selector

Conditional signal that only notifies when a boolean condition changes:

```rust
let selector = Selector::new(move |_| count.get() > 5);
```

## Effects

Side effects that run when dependencies change. **Client-only by default.**

```rust
// Basic effect — runs once immediately, re-runs on dependency change
Effect::new(move |_| {
    log::info!("count = {}", count.get());
});

// Effect with previous value
Effect::new(move |prev: Option<i32>| {
    let current = count.get();
    if let Some(prev) = prev {
        log::info!("changed from {} to {}", prev, current);
    }
    current // becomes prev_value on next run
});

// Isomorphic effect (runs on both server and client)
Effect::new_isomorphic(move |_| { /* ... */ });

// Watch with explicit deps (returns stoppable handle)
let stop = Effect::watch(
    move || count.get(),           // deps
    move |val, prev, _| { /* handler */ },
    false,                         // immediate?
);
stop(); // manually stop watching
```

**Rule: Never write to signals inside effects.** It causes reactive loops.

```rust
// BAD
Effect::new(move |_| {
    if a.get() > 5 { set_b.set(true); }
});

// GOOD — derive instead
let b = move || a.get() > 5;
// or
let b = Memo::new(move |_| a.get() > 5);
```

## Resources (Async Data Fetching)

```rust
// Reactive resource — re-fetches when source signal changes
let user = Resource::new(
    move || user_id.get(),                           // source signal
    move |id| async move { fetch_user(id).await },   // async fetcher
);

// Reading (returns Option<T>)
user.get();  // Option<T>, None while loading

// Client-only resource (not serialized across SSR)
let data = LocalResource::new(move || fetch_data(count.get()));

// Load-once resource
let config = OnceResource::new(async { load_config().await });

// Manual refetch
user.refetch();
```

Inside `<Suspense>`:

```rust
view! {
    <Suspense fallback=|| "Loading...">
        {move || Suspend::new(async move {
            let data = user.await;
            view! { <p>{data.name}</p> }
        })}
    </Suspense>
}
```

## Actions (Async Mutations)

```rust
let save = Action::new(|input: &String| {
    let input = input.clone();
    async move { save_to_db(input).await }
});

save.dispatch("hello".into());

// Reactive state
save.pending();  // ReadSignal<bool>
save.value();    // RwSignal<Option<Result<T, E>>>
save.input();    // RwSignal<Option<String>>

// For !Send futures
let save = Action::new_local(|input: &String| { /* ... */ });
```

### ServerAction

```rust
let add_todo = ServerAction::<AddTodo>::new();

view! {
    <ActionForm action=add_todo>
        <input type="text" name="title"/>
        <input type="submit" value="Add"/>
    </ActionForm>
}
```

## Spawning Async Tasks

```rust
spawn(async { /* platform-aware */ });
spawn_local(async { /* local task */ });
spawn_local_scoped(async { /* under current Owner */ });
```

---

## Reactive Stores (`reactive_stores` crate, v0.4.2)

Stores provide field-level reactivity for nested data structures. Unlike signals, updating one field does NOT notify sibling fields — only the field itself, its ancestors, and its descendants.

### Basic Usage

```rust
use reactive_stores::Store;

#[derive(Store)]
struct AppState {
    user: String,
    count: i32,
}

let store = Store::new(AppState { user: "Alice".into(), count: 0 });

// Access individual fields — each is independently reactive
store.user().get();                    // subscribe to user only
store.count().update(|n| *n += 1);     // notifies count subscribers, NOT user subscribers
store.count().set(42);
```

### Derive Macros

**`#[derive(Store)]`** generates accessor methods for each field:

```rust
#[derive(Store)]
struct Todo {
    id: usize,
    label: String,
    completed: bool,
}
// Generates: store.id(), store.label(), store.completed()
// Each returns a Subfield<...> that implements reactive Read/Write traits
```

**`#[derive(Patch)]`** enables efficient diffing — only notifies fields that actually changed:

```rust
#[derive(Store, Patch)]
struct Todo {
    id: usize,
    label: String,
    completed: bool,
}

store.patch(Todo { id: 1, label: "Updated".into(), completed: true });
// Only notifies fields whose values differ from current
```

### Keyed Collections

For `Vec`, `HashMap`, `BTreeMap` — use `#[store(key: ...)]` for stable identity across reorders:

```rust
#[derive(Store, Patch)]
struct Todos {
    #[store(key: usize = |todo| todo.id)]
    todos: Vec<Todo>,
}

// Access by key (stable across reorders)
let todo = store.todos().at_key(42);
todo.label().set("Updated".into());    // only notifies this todo's label

// Iterate reactively
for todo in store.todos() {
    let key = todo.key();              // stable key
    let label = todo.label().get();
}
```

**HashMap/BTreeMap:**

```rust
#[derive(Store)]
struct State {
    #[store(key: String = |(k, _)| k.clone())]
    items: HashMap<String, Item>,
}
```

### Unkeyed Collection Access

For simple index-based access without stable keys:

```rust
store.vec_field().at_unkeyed(0).get();      // access by index
for field in store.vec_field().iter_unkeyed() {
    println!("{}", field.get());
}
```

### Option Support

```rust
use reactive_stores::OptionStoreExt;

// Check and unwrap
if store.opt_field().read().is_some() {
    let inner = store.opt_field().unwrap();  // Subfield into the Some value
    inner.get();
}

// Reactive map — re-runs when toggling None/Some
store.name().map(|inner| inner.first().get().len());
```

### Box/Deref Support

```rust
use reactive_stores::DerefField;

#[derive(Store)]
struct Node {
    value: i32,
    #[store]
    child: Option<Box<Node>>,
}

store.child().unwrap().deref_field().value().get();
```

### Type-Erased Fields

Use `Field<T>` to pass store fields into components without specifying full generic types:

```rust
#[component]
fn TodoRow(todo: Field<Todo>) -> impl IntoView {
    view! {
        <li>
            <input type="checkbox"
                prop:checked=move || todo.completed().get()
                on:change=move |_| todo.completed().update(|c| *c = !*c)
            />
            {move || todo.label().get()}
        </li>
    }
}

// Convert any store field to Field<T>
<TodoRow todo=store.todos().at_key(1).into() />
```

### Integration with `<For>`

```rust
view! {
    <For
        each=move || store.todos()       // KeyedSubfield implements IntoIterator
        key=|todo| todo.key()            // AtKeyed has .key()
        let(todo)
    >
        <TodoRow todo=todo.into() />
    </For>
}
```

### Store Field Trait Methods

All store fields (Subfield, AtKeyed, Field, etc.) implement the standard reactive traits:

- `.get()` — track + read + clone
- `.read()` — track + read guard
- `.set(value)` — write the whole value
- `.update(|val| ...)` — mutate in place
- `.write()` — get a write guard
- `.track()` — subscribe without reading
- `.notify()` — trigger reactive updates

### Skip Fields

```rust
#[derive(Store)]
struct State {
    #[store(skip)]
    internal_cache: HashMap<String, Vec<u8>>,  // not reactive
}
```

### Custom Patch

```rust
#[derive(Store, Patch)]
struct Custom {
    #[patch(|this, new| *this = new)]
    data: String,
}
```

### Enum Stores

```rust
#[derive(Store)]
enum Choice {
    First,
    Second(String),
    Third { x: i32, y: i32 },
}

store.first();      // bool — is this variant active?
store.second();     // bool
store.second_0();   // Option<Subfield<..., String>> — access the inner value
store.third_x();    // Option<Subfield<..., i32>>
```
