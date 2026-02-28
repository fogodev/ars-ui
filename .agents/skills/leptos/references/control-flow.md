# Leptos 0.8.17 — Control Flow Reference

## `<Show>` — Conditional Rendering

Memoizes the condition; only re-renders when the branch actually changes.

```rust
view! {
    <Show
        when=move || value.get() > 5
        fallback=|| view! { <p>"Small value"</p> }
    >
        <p>"Big value!"</p>
    </Show>
}
```

## `<For>` — Keyed Dynamic Lists

Performs keyed diffing — only items whose keys change are re-rendered.

```rust
view! {
    <For
        each=move || todos.get()
        key=|todo| todo.id               // key must be unique and stable
        children=move |todo| view! {
            <div>{todo.title}</div>
        }
    />
}

// Alternative: let binding syntax
<For
    each=move || todos.get()
    key=|todo| todo.id
    let(todo)
>
    <div>{todo.title}</div>
</For>
```

### `<ForEnumerate>` — With Reactive Index

```rust
<ForEnumerate
    each=move || items.get()
    key=|item| item.id
    children=move |(index, item)| view! {
        <div>{move || index.get()} ": " {item.name}</div>
    }
/>
```

## `<Suspense>` — Async Loading Boundary

Shows fallback while async resources are loading. On SSR, enables out-of-order streaming.

```rust
let data = Resource::new(count, |count| async move { load(count).await });

view! {
    <Suspense fallback=move || view! { <p>"Loading..."</p> }>
        {move || data.get().map(|d| view! { <p>{d.name}</p> })}
    </Suspense>
}
```

With `Suspend::new` for inline async:

```rust
<Suspense fallback=|| "Loading...">
    {move || Suspend::new(async move {
        let result = resource.await;
        view! { <p>{result.name}</p> }
    })}
</Suspense>
```

Multiple resources under one `<Suspense>`:

```rust
<Suspense fallback=|| "Loading...">
    {move || a.get().map(|a| view! { <ShowA a/> })}
    {move || b.get().map(|b| view! { <ShowB b/> })}
</Suspense>
```

## `<Transition>` — Keep Previous Content

Like `<Suspense>`, but keeps the **previous content visible** while new data loads instead of showing the fallback again.

```rust
<Transition fallback=|| view! { <div>"Loading..."</div> }>
    {move || Suspend::new(async move {
        let data = resource.await;
        view! { <p>{data.name}</p> }
    })}
</Transition>
```

The `set_pending` prop provides a signal indicating loading state:

```rust
let (pending, set_pending) = signal(false);

<Transition fallback=|| "Loading..." set_pending>
    // content shows previous data while reloading
    // pending.get() is true during reload
</Transition>
```

## `<ErrorBoundary>` — Catch Errors

Catches `Err` values from child `Result<impl IntoView, E>` signals.

```rust
let (value, set_value) = signal(Ok(0));

view! {
    <ErrorBoundary
        fallback=|errors| view! {
            <ul>
                {move || errors.get().into_iter()
                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                    .collect::<Vec<_>>()
                }
            </ul>
        }
    >
        <p><strong>{value}</strong></p>
    </ErrorBoundary>
}
```

## `<Await>` — One-Shot Async

```rust
<Await future=|| async { fetch_data().await } let:data>
    <p>{data.name.clone()}</p>
</Await>
```

## Inline Conditional Patterns

For simple cases, you can skip `<Show>` and use closures directly:

```rust
view! {
    // Closure returning Option
    {move || count.get().gt(&5).then(|| view! { <p>"Big"</p> })}

    // Either for two branches
    {move || if count.get() > 5 {
        Either::Left(view! { <p>"Big"</p> })
    } else {
        Either::Right(view! { <p>"Small"</p> })
    }}
}
```
