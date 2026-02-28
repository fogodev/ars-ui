# Leptos 0.8.17 — Components & View Macro Reference

## Components

Components are plain Rust functions annotated with `#[component]`, returning `impl IntoView`.

```rust
use leptos::prelude::*;

#[component]
pub fn Counter(initial: i32) -> impl IntoView {
    let (count, set_count) = signal(initial);
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Count: " {count}
        </button>
    }
}

// Usage
view! { <Counter initial=5 /> }
```

**Component functions run once** (setup phase). Reactive updates happen through closures and signals.

### Props

```rust
#[component]
pub fn MyButton(
    label: String,                                    // required
    #[prop(optional)] disabled: bool,                 // optional, defaults to Default::default()
    #[prop(default = 42)] count: i32,                 // optional with explicit default
    #[prop(into)] on_click: Callback<MouseEvent>,     // accepts Into<Callback<MouseEvent>>
    children: Children,                               // child elements
) -> impl IntoView {
    view! {
        <button disabled=disabled on:click=move |e| on_click.run(e)>
            {label} {children()}
        </button>
    }
}
```

### Children Types

| Type               | Description                                 |
| ------------------ | ------------------------------------------- |
| `Children`         | `Box<dyn FnOnce() -> AnyView>` — single use |
| `ChildrenFn`       | Implements `Fn` — multi-use                 |
| `ChildrenMut`      | Implements `FnMut`                          |
| `ChildrenFragment` | Returns `Fragment` with `.nodes: Vec<View>` |

```rust
#[component]
fn Wrapper(children: Children) -> impl IntoView {
    view! { <div class="wrapper">{children()}</div> }
}

view! { <Wrapper><p>"Hello"</p></Wrapper> }
```

### Slots

```rust
#[slot]
struct Header { children: ChildrenFn }

#[component]
fn Card(header: Header, children: Children) -> impl IntoView {
    view! {
        <div class="card">
            <div class="header">{(header.children)()}</div>
            <div class="body">{children()}</div>
        </div>
    }
}

view! {
    <Card>
        <Header slot>"My Title"</Header>
        "Card body content"
    </Card>
}
```

## Context (Dependency Injection)

```rust
// Provide to all descendants
provide_context(MyState::new());

// Consume (returns Option)
let state = use_context::<MyState>();

// Consume (panics if missing)
let state = expect_context::<MyState>();
```

Context is scoped to the component subtree where `provide_context` is called.

## NodeRef

```rust
let input_ref = NodeRef::<html::Input>::new();

view! {
    <input node_ref=input_ref />
    <button on:click=move |_| {
        if let Some(input) = input_ref.get() {
            let val = input.value();
        }
    }>"Read"</button>
}
```

---

## The `view!` Macro

JSX-like HTML syntax that compiles to fine-grained DOM operations.

### Text and Dynamic Values

```rust
view! {
    <span>"Static text in quotes"</span>
    <span>{move || count.get()}</span>          // reactive closure
    <span>{count}</span>                        // signal used directly (auto-tracked)
    <span>{"Rust expression"}</span>            // any Display value
}
```

### Attributes

```rust
view! {
    // Static attributes
    <div class="my-class" id="main">"content"</div>

    // Dynamic attributes
    <div class:active=move || is_active.get()/>          // conditional class
    <div class=("my-class", move || is_active.get())/>   // tuple syntax for special chars
    <div style:color="red"/>                             // individual CSS property
    <div style:font-size=move || format!("{}px", size.get())/>

    // DOM properties (not HTML attributes — important for reactive inputs)
    <input prop:value=move || name.get() />

    // Explicit HTML attribute
    <div attr:data-custom="value"/>

    // Two-way binding shorthands
    <input bind:value=name />           // binds to RwSignal<String>
    <input type="checkbox" bind:checked=checked />
    <input type="radio" bind:group=selected />

    // Attribute spreading
    <div {..attrs}>"content"</div>
}
```

### Event Handlers

```rust
view! {
    // Basic
    <button on:click=move |_| set_count.update(|n| *n += 1)>"+"</button>

    // Typed event with target access
    <input on:input:target=move |ev| set_name.set(ev.target().value()) />

    // Event delegation (default for most events)
    // Use on:click:undelegated for non-bubbling behavior
}
```

### Components in View

```rust
view! {
    <Counter initial=5 />
    <MyButton label="Click" on_click=move |_| {} />
}
```

## Builder Syntax (Alternative to view! Macro)

What `view!` expands to — usable directly without macros:

```rust
use leptos::html::*;

div().child((
    button()
        .on(ev::click, move |_| set_count.update(|n| *n += 1))
        .child("Click me"),
    span()
        .class(("active", move || is_active.get()))
        .child(("Count: ", move || count.get())),
))
```

Methods: `.child()`, `.attr()`, `.class()`, `.style()`, `.prop()`, `.on()`, `.id()`.

## Type Erasure for Conditional Rendering

When returning different view types from branches:

```rust
// Using .into_any()
if condition {
    view! { <div>"A"</div> }.into_any()
} else {
    view! { <span>"B"</span> }.into_any()
}

// Using Either enum (more efficient, no allocation)
use leptos::either::Either;
if condition {
    Either::Left(view! { <div>"A"</div> })
} else {
    Either::Right(view! { <span>"B"</span> })
}

// EitherOf3..EitherOf16 for more branches
```

## Mounting

```rust
// Client-side rendering
fn main() {
    mount_to_body(|| view! { <App/> });
}

// Hydration (for SSR)
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_body(App);
}

// Islands hydration
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}
```

## Parent-Child Communication Patterns

1. **Pass `WriteSignal` down** — child mutates parent state directly
2. **Callback prop** — `#[prop(into)] on_change: Callback<String>`
3. **Event listener on component** — `<MyComponent on:click=handler />`
4. **Context** — `provide_context(signal)` / `expect_context::<T>()`
