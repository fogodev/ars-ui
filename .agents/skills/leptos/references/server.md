# Leptos 0.8.17 — Server Functions, SSR & Islands Reference

## Server Functions (`#[server]`)

Server functions compile to HTTP endpoints on the server and thin async call wrappers on the client.

### Basic Usage

```rust
use leptos::prelude::*;

#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    // This code runs ONLY on the server
    let pool = expect_context::<PgPool>();
    sqlx::query("INSERT INTO todos (title) VALUES ($1)")
        .bind(title)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(())
}
```

### Full Options

```rust
#[server(
    name = CreateUser,           // struct name (default: PascalCase of fn name)
    prefix = "/api",             // URL prefix (default: "/api")
    endpoint = "user/create",    // path after prefix
    input = Json,                // input encoding
    output = Json,               // output encoding
)]
pub async fn create_user(name: String, email: String) -> Result<User, AppError> { ... }
```

### Codecs

| Codec           | Direction   | HTTP Method | Notes                           |
| --------------- | ----------- | ----------- | ------------------------------- |
| `Json`          | In + Out    | POST        | serde_json; default output      |
| `Cbor`          | In + Out    | POST        | Binary, compact                 |
| `GetUrl`        | Input only  | GET         | URL-encoded query               |
| `PostUrl`       | Input only  | POST        | URL-encoded body; default input |
| `Rkyv`          | In + Out    | POST        | Zero-copy binary                |
| `MultipartData` | Input only  | POST        | File uploads                    |
| `Streaming`     | Output only | POST        | ByteStream                      |
| `TextStream`    | Output only | POST        | UTF-8 text stream               |
| `SseStream`     | Output only | POST        | Server-Sent Events              |

### Custom Error Types

```rust
use server_fn::codec::JsonEncoding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppError {
    Database(String),
    Unauthorized,
    Internal(ServerFnErrorErr),
}

impl FromServerFnError for AppError {
    type Encoder = JsonEncoding;
    fn from_server_fn_error(err: ServerFnErrorErr) -> Self {
        AppError::Internal(err)
    }
}

#[server]
pub async fn get_user(id: i64) -> Result<User, AppError> { ... }
```

### Streaming Output

```rust
#[server(output = Streaming)]
pub async fn live_feed() -> Result<ByteStream, ServerFnError> {
    let stream = futures::stream::iter(vec![Ok(Bytes::from("chunk"))]);
    Ok(ByteStream::new(stream))
}

#[server(output = SseStream)]
pub async fn events() -> Result<SseStream, ServerFnError> { ... }
```

### Calling from Components

```rust
// Direct call
spawn_local(async {
    add_todo("Buy milk".into()).await.unwrap();
});

// ServerAction for forms
let add = ServerAction::<AddTodo>::new();

view! {
    <ActionForm action=add>
        <input type="text" name="title"/>
        <button type="submit">"Add"</button>
    </ActionForm>
}
```

### Important Rules

- Args and return types must be serializable (Serde)
- Avoid `usize`/`isize` — WASM is 32-bit, server is 64-bit
- Feature flags: `json`, `cbor`, `rkyv`, `multipart`, `ssr`, `browser`

---

## Leptos-Axum Integration

```rust
use leptos_axum::*;

let app = Router::new()
    .route("/api/{*fn_name}", post(handle_server_fns))
    .leptos_routes(&leptos_options, routes, App)
    .with_state(leptos_options);
```

### Providing Context to Server Functions

```rust
.route(
    "/api/{*fn_name}",
    post(|State(state): State<AppState>, req: Request| async move {
        handle_server_fns_with_context(
            move || {
                provide_context(state.db_pool.clone());
            },
            req,
        ).await
    }),
)
```

Inside server fn: `let pool = expect_context::<PgPool>();`

### Extractors and Response Helpers

```rust
use leptos_axum::*;

#[server]
pub async fn check_auth() -> Result<User, ServerFnError> {
    let req = extract::<axum::http::request::Parts>().await?;
    // inspect headers, etc.
    Ok(user)
}

#[server]
pub async fn set_cookie() -> Result<(), ServerFnError> {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(SET_COOKIE, cookie_value);
    Ok(())
}
```

---

## SSR Modes

| Mode                       | Behavior                                                     | Trade-off                            |
| -------------------------- | ------------------------------------------------------------ | ------------------------------------ |
| Synchronous                | Render full tree at once                                     | Fastest TTFB for non-async pages     |
| In-order streaming         | Stream HTML, pause at each `<Suspense>`                      | No JS required for ordering; slower  |
| **Out-of-order streaming** | Send shell first, stream `<Suspense>` chunks as they resolve | Best perceived performance (default) |
| Async                      | Wait for all resources, render once                          | Best for meta tags; slowest TTFB     |

---

## Islands Architecture

Selective hydration — only interactive "islands" ship JavaScript.

### Setup

```toml
# Cargo.toml
[features]
islands = ["leptos/islands"]
```

```rust
// lib.rs — hydrate only islands
pub fn hydrate() {
    leptos::mount::hydrate_islands();
}

// In your shell
view! { <HydrationScripts options islands=true/> }
```

### Defining Islands

```rust
#[island]
fn Counter(initial: i32) -> impl IntoView {
    let (count, set_count) = signal(initial);
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            {count}
        </button>
    }
}
```

- `#[island]` replaces `#[component]` for interactive regions
- Non-island `#[component]`s are server-only (can use `std::fs`, database, etc.)
- Islands share state via `provide_context` / `expect_context`
- Significantly smaller WASM bundles (~50% reduction)

---

## Document Metadata (`leptos_meta`)

```rust
use leptos_meta::*;

#[component]
fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text=move || name.get() formatter=|t| format!("{t}'s Profile") />
        <Meta name="description" content="User profile page"/>
        <Link rel="canonical" href="https://example.com"/>
        <Style>"body { background: #333; }"</Style>
        <Stylesheet href="/style.css"/>
    }
}
```

All meta components are reactive — they update the `<head>` when signals change.
