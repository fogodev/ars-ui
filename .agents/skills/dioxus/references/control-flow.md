# Dioxus 0.7.3 — Control Flow Reference

## Conditionals

Use standard Rust `if` / `if-else` directly in RSX:

```rust
rsx! {
    if show_title {
        h1 { "Title" }
    }

    if count() > 5 {
        div { class: "warning", "Too many!" }
    } else {
        div { "All good" }
    }
}
```

### Optional Rendering

```rust
rsx! {
    {show_title.then(|| rsx! { h1 { "Title" } })}
}
```

## Iteration

### `for` Loops

```rust
rsx! {
    ul {
        for item in 0..5 {
            li { "{item}" }
        }
    }

    // With keys (ALWAYS add keys for dynamic lists)
    for user in users.iter() {
        div {
            key: "{user.id}",
            "{user.name}"
        }
    }
}
```

### Iterator Mapping

```rust
rsx! {
    ul {
        {(0..5).map(|i| rsx! { li { "{i}" } })}
    }
}
```

## `SuspenseBoundary` — Async Loading

Shows fallback while async resources are loading.

```rust
rsx! {
    SuspenseBoundary {
        fallback: |_| rsx! { div { "Loading..." } },
        Gallery { breed: "hound" }
    }
}

#[component]
fn Gallery(breed: ReadSignal<String>) -> Element {
    let response = use_resource(move || async move {
        fetch_breed_images(breed()).await
    }).suspend()?;   // <-- suspends component, shows fallback

    rsx! {
        match &*response.read() {
            Ok(urls) => rsx! {
                for url in urls.iter().take(3) {
                    img { src: "{url}" }
                }
            },
            Err(err) => rsx! { "Error: {err}" },
        }
    }
}
```

Key: `.suspend()?` on a resource pauses the component and returns to the nearest `SuspenseBoundary`.

## `ErrorBoundary` — Error Handling

Catches errors from child components that use `?` or `throw_error()`.

```rust
rsx! {
    ErrorBoundary {
        handle_error: |ctx: ErrorContext| {
            rsx! {
                div { class: "error",
                    "Something went wrong: "
                    // ctx.error() returns Option<CapturedError>; CapturedError implements Display
                    {ctx.error().map(|e| format!("{e}")).unwrap_or_default()}
                }
            }
        },
        ChildComponent {}
    }
}

#[component]
fn ChildComponent() -> Element {
    let data = get_data().context("failed to get data")?;
    rsx! { div { "{data}" } }
}
```

### Error Propagation with `?`

Since `Element = Result<VNode, RenderError>`, you can use `?` freely:

```rust
fn MyComponent() -> Element {
    let config = load_config()?;          // propagates error
    let user = fetch_user()
        .context("user not found")?;      // with context message

    rsx! { div { "User: {user.name}" } }
}
```

Errors bubble up to the nearest `ErrorBoundary`. Event handlers can also return `Result`:

```rust
rsx! {
    button {
        onclick: move |_| -> Result<(), MyError> {
            do_something()?;
            Ok(())
        },
        "Click"
    }
}
```

## Pattern Matching in RSX

```rust
rsx! {
    match status() {
        Status::Loading => rsx! { "Loading..." },
        Status::Error(e) => rsx! { "Error: {e}" },
        Status::Ready(data) => rsx! { "Data: {data}" },
    }
}
```
