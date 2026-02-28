# Dioxus 0.7.3 — Components & RSX Macro Reference

## Components

Components are functions annotated with `#[component]`, returning `Element`.

```rust
use dioxus::prelude::*;

#[component]
fn Greeting(name: String, age: u32) -> Element {
    rsx! { p { "Hello {name}, age {age}" } }
}

// Usage
rsx! { Greeting { name: "Alice", age: 30 } }
```

`Element` is `Result<VNode, RenderError>` — the `?` operator works in components for error propagation.

### Props

Props are the function parameters. The `#[component]` macro generates a typed Props struct.

```rust
#[component]
fn MyButton(
    label: String,                                         // required
    #[props(default)] disabled: bool,                      // optional, uses Default::default()
    #[props(default = "blue".to_string())] color: String,  // explicit default
    #[props(into)] class: String,                          // accepts Into<String>
    #[props(!optional)] value: Option<i32>,                // Option<T> but REQUIRED
) -> Element {
    rsx! { button { class: "{class}", color: "{color}", disabled: disabled, "{label}" } }
}
```

**Prop modifiers:**

| Attribute                              | Effect                              |
| -------------------------------------- | ----------------------------------- |
| `#[props(default)]`                    | Optional, uses `Default::default()` |
| `#[props(default = expr)]`             | Optional, uses given expression     |
| `#[props(!optional)]`                  | Makes `Option<T>` required          |
| `#[props(into)]`                       | Accepts `Into<T>`                   |
| `#[props(extends = GlobalAttributes)]` | Inherits all HTML global attributes |

**Automatic behaviors:**

- `Option<T>` fields are automatically optional (default `None`)
- `String` fields accept format-string expressions
- `ReadSignal<T>` fields auto-convert from `Signal<T>`

### Manual Props Struct

```rust
#[derive(Props, PartialEq, Clone)]
struct ButtonProps {
    #[props(default)]
    text: String,
    #[props(default = "red".to_string())]
    color: String,
}

fn Button(props: ButtonProps) -> Element {
    rsx! { button { color: props.color, "{props.text}" } }
}
```

### Children

Accept child RSX via `children: Element`:

```rust
#[component]
fn Card(title: String, children: Element) -> Element {
    rsx! {
        div { class: "card",
            h2 { "{title}" }
            div { class: "body", {children} }
        }
    }
}

rsx! {
    Card { title: "My Card",
        p { "Card content here" }
    }
}
```

### EventHandler Props

```rust
#[component]
fn MyButton(onclick: EventHandler<MouseEvent>) -> Element {
    rsx! { button { onclick: move |e| onclick(e), "Click" } }
}

// Callback with return value
#[component]
fn Validator(validate: Callback<String, bool>) -> Element {
    rsx! {
        button {
            onclick: move |_| {
                let valid = validate("test".into());
            },
            "Validate"
        }
    }
}
```

## Context (Dependency Injection)

```rust
#[derive(Clone, Copy)]
struct Theme { dark_mode: Signal<bool> }

fn Parent() -> Element {
    use_context_provider(|| Theme { dark_mode: use_signal(|| false) });
    rsx! { Child {} }
}

#[component]
fn Child() -> Element {
    let mut theme = use_context::<Theme>();
    rsx! {
        button {
            onclick: move |_| theme.dark_mode.toggle(),
            if theme.dark_mode() { "Light Mode" } else { "Dark Mode" }
        }
    }
}
```

- `use_context_provider(|| value)` — provide to descendants
- `use_context::<T>()` — consume (panics if missing)
- `try_use_context::<T>()` — consume (returns Option)
- `use_root_context::<T>(|| init)` — provide at root level

---

## The `rsx!` Macro

RSX is Dioxus's HTML-like macro. Returns `Element`.

### Elements and Text

```rust
rsx! {
    div { class: "container",
        h1 { "Hello, world!" }
        p { "Count is {count}" }            // inline formatting
        {"Arbitrary expression"}            // any Display value
    }
}
```

### Attributes

```rust
rsx! {
    div {
        class: "foo",
        id: "main",
        "data-custom": "value",             // quoted name for non-standard attrs

        // Conditional attributes
        class: if is_active { "active" },

        // Multiple class attributes merge
        class: "base",
        class: if highlighted { "highlight" },

        // Inline styles (use underscores for hyphens)
        background_color: "red",
        padding: "10px",
    }
}
```

### Event Handlers

```rust
rsx! {
    button {
        onclick: move |event| {
            println!("clicked: {:?}", event);
            count += 1;
        },
        "Click me"
    }

    input {
        oninput: move |evt| name.set(evt.value()),
    }

    // Async event handlers (auto-spawned)
    button {
        onclick: move |_| async move {
            let data = fetch_data().await;
            result.set(data);
        },
        "Load async"
    }
}
```

### Spreading Props

```rust
rsx! {
    div { ..attrs, "content" }
}
```

### Dangerous Inner HTML

```rust
rsx! {
    div { dangerous_inner_html: "<b>raw HTML</b>" }
}
```

### Web Components

```rust
rsx! {
    my-web-component { "name": "hello" }
}
```

---

## Styling

### CSS File

```rust
use dioxus::prelude::*;

rsx! {
    document::Stylesheet { href: asset!("/assets/style.css") }
    div { class: "my-class", "Styled content" }
}
```

### SCSS

```rust
document::Stylesheet { href: asset!("/assets/style.scss") }
```

### Tailwind

Just create a `tailwind.css` file — Dioxus auto-detects and runs Tailwind.

```rust
rsx! {
    div { class: "flex items-center gap-4 p-4 bg-blue-500 text-white",
        "Tailwind styled"
    }
}
```

### Inline Styles

```rust
rsx! {
    div {
        background_color: "red",
        font_size: "16px",
        padding: "10px",
    }
}
```

### Scoped CSS / CSS Modules (0.7.3+)

Available via the asset system.

## Lifecycle Hooks

```rust
use_drop(|| {
    println!("component unmounting");
});

use_before_render(|| { /* before each render */ });
use_after_render(|| { /* after each render */ });
```
