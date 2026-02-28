# Dioxus 0.7.3 — Reactive System Reference

Covers: `dioxus-signals`, `dioxus-hooks`, `dioxus-stores`.

## Signals

The primitive reactive value. Reading subscribes; writing triggers re-renders.

### Creation

```rust
let mut count = use_signal(|| 0);              // Signal<i32>, Copy
let mut count = use_signal_sync(|| 0);         // Send + Sync variant
```

### Reading

| Method      | Behavior                                  |
| ----------- | ----------------------------------------- |
| `signal()`  | Clone value + subscribe (callable syntax) |
| `.read()`   | `Ref<T>` borrow + subscribe               |
| `.cloned()` | Clone value + subscribe                   |
| `.peek()`   | Read WITHOUT subscribing                  |

### Writing

| Method              | Behavior                            |
| ------------------- | ----------------------------------- |
| `.set(value)`       | Replace value, notify subscribers   |
| `.write()`          | `RefMut<T>` guard, notifies on drop |
| `+= / -= / *= / /=` | Arithmetic assignment               |
| `.toggle()`         | For booleans                        |
| `.dispose()`        | Manual cleanup                      |

### Async Safety

**Never hold `.read()` or `.write()` across an `.await` — it panics.**

```rust
// BAD — panics
use_future(move || async move {
    let val = signal.read();    // holds borrow
    some_async().await;         // PANIC: borrow held across await
});

// GOOD — clone first
use_future(move || async move {
    let val = signal();         // clone out
    let result = some_async(val).await;
    signal.set(result);
});
```

### Global Signals

```rust
static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

fn IncrementButton() -> Element {
    rsx! {
        button { onclick: move |_| *COUNT.write() += 1, "Increment: {COUNT}" }
    }
}
```

### Signal Type Summary

| Type                | Description                                  |
| ------------------- | -------------------------------------------- |
| `Signal<T>`         | Copy, reactive, auto-tracked                 |
| `ReadSignal<T>`     | Boxed read-only (accepts Signal, Memo, etc.) |
| `WriteSignal<T>`    | Boxed write-only                             |
| `ReadOnlySignal<T>` | **Deprecated** — use `ReadSignal`            |
| `Memo<T>`           | Memoized computed value                      |
| `CopyValue<T>`      | Copy wrapper for any value                   |
| `GlobalSignal<T>`   | `Global<Signal<T>>` — app-wide               |
| `GlobalMemo<T>`     | `Global<Memo<T>>` — app-wide derived         |

### Collection Extensions

Signals have convenience methods for common collection types:

- `ReadableVecExt` / `WritableVecExt` — `.iter()`, `.len()`, etc. on `Signal<Vec<T>>`
- `ReadableOptionExt` / `WritableOptionExt` — on `Signal<Option<T>>`
- `ReadableHashMapExt` — on `Signal<HashMap<K, V>>`

## Memos (Derived State)

Memoized derived state. Reruns only when signal dependencies change; skips downstream if output unchanged.

```rust
let mut count = use_signal(|| 1);
let doubled = use_memo(move || count() * 2);
let tripled = use_memo(move || doubled() + count());  // chains work
```

## Effects

Run side effects when tracked signals change. Runs after render.

```rust
use_effect(move || {
    println!("count = {}", count());
});
```

## Resources (Async Derived State)

Automatically reruns when signal dependencies change.

```rust
let mut breed = use_signal(|| "hound".to_string());

let dogs = use_resource(move || async move {
    reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images"))
        .await?.json::<Response>().await
});

// Reading: Option<Result<T, E>>
if let Some(Ok(urls)) = &*dogs.read() {
    // render urls
}

// Manual restart
dogs.restart();
```

### With Suspense

```rust
#[component]
fn Gallery(breed: ReadSignal<String>) -> Element {
    let response = use_resource(move || async move {
        fetch_breed(breed()).await
    }).suspend()?;    // suspends component; shows SuspenseBoundary fallback

    rsx! {
        for url in response.read().as_ref().unwrap().iter() {
            img { src: "{url}" }
        }
    }
}
```

## Futures (Long-Lived Async)

Spawns once on mount, lives for component lifetime.

```rust
let mut running = use_signal(|| true);

use_future(move || async move {
    loop {
        if running() { count += 1; }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
});
```

## Coroutines (Async with Channel)

Pauseable/resumable future with an `mpsc` channel for receiving messages.

```rust
let task = use_coroutine(|mut rx: UnboundedReceiver<Action>| async move {
    while let Some(action) = rx.next().await {
        match action {
            Action::Start => { /* ... */ }
            Action::Stop => { /* ... */ }
        }
    }
});

// Send messages
task.send(Action::Start);

// Retrieve from child components
let task = use_coroutine_handle::<Action>();
```

## Actions (User-Input Async)

Fire-and-forget async with automatic cancellation of previous invocation.

```rust
let mut message = use_action(get_message);

rsx! {
    pre { "{message:?}" }
    button { onclick: move |_| message.call("world".into(), 30), "Fetch" }
}
```

## Spawning Tasks

```rust
spawn(async { /* background task, returns Task handle */ });
spawn_forever(async { /* persists beyond component unmount */ });

// Event handlers auto-spawn async closures:
rsx! {
    button {
        onclick: move |_| async move {
            let result = fetch_data().await;
            data.set(result);
        },
        "Load"
    }
}
```

## Callbacks

Stable callback references that don't cause re-renders when passed as props:

```rust
let handler = use_callback(move |value: i32| {
    count.set(value);
});
```

---

## Stores (`dioxus-stores` crate, v0.7.3)

Stores provide field-level reactivity for nested data. Each field gets its own subscription path — writing to one field doesn't notify sibling fields.

### Basic Usage

```rust
use dioxus_stores::*;

#[derive(Store, Default)]
struct AppState {
    count: i32,
    name: String,
}

fn App() -> Element {
    let value = use_store(Default::default);

    let mut count = value.count();     // scoped store for count field
    let mut name = value.name();       // scoped store for name field

    rsx! {
        button { onclick: move |_| count += 1, "Count: {count}" }
        input { oninput: move |e| name.set(e.value()), "Name: {name}" }
    }
}
```

### How Stores Differ from Signals

- **`Signal<T>`**: Single reactive cell. Reading/writing the whole value triggers all subscribers.
- **`Store<T>`**: Reactive cell with a subscription tree. Each field has its own path. Reading `store.count()` only subscribes to that field. Writing to `count` only marks that field dirty.

### `#[derive(Store)]` for Structs

```rust
#[derive(Store)]
struct Todo {
    checked: bool,
    contents: String,
}
// Generates: store.checked() -> Store<bool, ...>, store.contents() -> Store<String, ...>
```

### `#[derive(Store)]` for Enums

Requires `PartialEq + Clone + Debug`.

```rust
#[derive(Store, PartialEq, Clone, Debug)]
enum Choice {
    Foo(String),
    Bar { x: i32, y: i32 },
}

// store.is_foo() -> bool
// store.is_bar() -> bool
// store.foo() -> Option<Store<String, _>>
// store.transpose() -> ChoiceStoreTransposed
```

### Transpose

`.transpose()` gives you a struct/enum with all fields wrapped as stores:

```rust
match store.transpose() {
    ChoiceStoreTransposed::Foo(foo) => rsx! { "Foo: {foo}" },
    ChoiceStoreTransposed::Bar { x, y } => rsx! { "Bar: {x}, {y}" },
}
```

### `#[store]` Attribute — Extension Methods

Add methods to stores for types not in your crate:

```rust
#[store]
impl<Lens> Store<Todo, Lens> {
    fn sum(&self) -> i32 { /* ... */ }         // &self -> requires Readable
    fn toggle(&mut self) { /* ... */ }         // &mut self -> requires Writable
}
```

### Vec Support

```rust
let mut items = store.items();    // Store<Vec<T>, ...>

items.len();                      // tracks shallow (length changes)
items.iter();                     // iterator of Store<T, ...> per element
items.get(0);                     // Option<Store<T, ...>>
items.push(value);                // marks only length dirty
items.remove(idx);                // marks length + items at/after idx dirty
items.clear();                    // marks everything dirty
```

### HashMap / BTreeMap Support

```rust
let mut map = store.data();       // Store<HashMap<K, V>, ...>

map.len();
map.iter();                       // iterator of (K, Store<V, ...>)
map.get(key);                     // Option<Store<V, ...>>
map.get_unchecked(key);           // Store<V, ...> (panics if missing)
map.insert(key, value);
map.remove(&key);
map.values();                     // iterator of Store<V, ...>
```

### Option Support

```rust
let opt = store.opt_field();      // Store<Option<T>, ...>

opt.is_some();                    // tracks shallow
opt.is_none();
opt.transpose();                  // Option<Store<T, ...>>
opt.unwrap();                     // Store<T, ...> (panics if None)
```

### Result Support

```rust
let res = store.result_field();   // Store<Result<T, E>, ...>

res.is_ok();
res.ok();                         // Option<Store<T, ...>>
res.err();                        // Option<Store<E, ...>>
res.unwrap();
```

### GlobalStore

```rust
#[derive(Store)]
struct Counter { count: i32 }

static COUNTER: GlobalStore<Counter> = Global::new(|| Counter { count: 0 });

fn app() -> Element {
    let mut count = COUNTER.resolve().count();
    rsx! { button { onclick: move |_| count += 1, "{count}" } }
}
```

### SyncStore

For `Send + Sync` contexts:

```rust
let store = use_store_sync(|| MyState::default());
```

### Recursive / Tree Structures

```rust
#[derive(Store, Default)]
struct TreeNode {
    value: i32,
    children: Vec<TreeNode>,
}

#[component]
fn Tree(value: Store<TreeNode>) -> Element {
    let mut count = value.count();
    let mut children = value.children();
    rsx! {
        button { onclick: move |_| count += 1, "Value: {count}" }
        button { onclick: move |_| children.push(Default::default()), "Add child" }
        ul {
            for child in children.iter() {
                li { Tree { value: child } }
            }
        }
    }
}
```

### Type Aliases

| Type             | Description                                        |
| ---------------- | -------------------------------------------------- |
| `Store<T>`       | Default read-write store (backed by `WriteSignal`) |
| `ReadStore<T>`   | Read-only store                                    |
| `WriteStore<T>`  | Write-only store                                   |
| `SyncStore<T>`   | `Send + Sync` store                                |
| `GlobalStore<T>` | App-wide singleton store                           |
