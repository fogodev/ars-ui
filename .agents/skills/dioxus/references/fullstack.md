# Dioxus 0.7.3 — Fullstack & SSR Reference

Covers: `dioxus-fullstack`, `dioxus-document`.

## Server Functions

### `#[server]`

Define backend logic called transparently from the frontend.

```rust
#[server]
async fn save_item(name: String) -> Result<(), ServerFnError> {
    // Runs only on the server
    db::save(name).await?;
    Ok(())
}
```

### HTTP Method Macros (New in 0.7)

```rust
#[get("/api/:name/?age")]
async fn get_message(name: String, age: i32) -> Result<String> {
    Ok(format!("Hello {name}, age {age}!"))
}

#[post("/api/items")]
async fn create_item(item: Item) -> Result<Item> { /* ... */ }

#[put("/api/items/:id")]
async fn update_item(id: u32, item: Item) -> Result<Item> { /* ... */ }

#[patch("/api/items/:id")]
async fn patch_item(id: u32, patch: Patch) -> Result<Item> { /* ... */ }

#[delete("/api/items/:id")]
async fn delete_item(id: u32) -> Result<()> { /* ... */ }
```

### Default Codec

**JSON** (changed from URL-encoded in 0.6). Server function args and return types must be serializable.

### Calling from Components

```rust
// Direct async call
rsx! {
    button {
        onclick: move |_| async move {
            save_item("New item".into()).await.unwrap();
        },
        "Save"
    }
}
```

### `use_action` — Imperative Server Calls

Automatically cancels previous invocation when called again.

```rust
let mut message = use_action(get_message);

rsx! {
    pre { "{message:?}" }
    button {
        onclick: move |_| message.call("world".into(), 30),
        "Fetch"
    }
}
```

### `use_server_future` — SSR-Safe Async

For server functions during SSR. The resolved value is serialized and sent to the client to avoid hydration mismatches.

```rust
// CORRECT for SSR:
let data = use_server_future(|| my_server_fn())?()
    .unwrap()
    .unwrap_or_default();

// WRONG for SSR (may be pending on client even if resolved on server):
let data = use_resource(|| my_server_fn()).suspend()?;
```

### `use_loader` — Hybrid Client/Server Fetching

New in 0.7. Combines server-side loading with client-side reactivity.

### Extractors

Server-only context via macro arguments:

```rust
#[post("/api/protected", auth: Session)]
async fn protected(auth: Session) -> Result<String> {
    let user = auth.user()?;
    Ok(format!("Hello {}", user.name))
}
```

### Error Handling

```rust
use dioxus::fullstack::prelude::*;

// Convert to HTTP errors
let user = db::find_user(id)
    .await
    .or_not_found()?;

let admin = check_admin(user)
    .or_unauthorized()?;

// Generic HTTP error
let data = fetch()
    .await
    .or_http_error(StatusCode::BAD_GATEWAY, "upstream failed")?;
```

### Streaming and WebSocket

```rust
// Streaming response
#[server(output = Streaming)]
async fn stream_data() -> Result<Streaming, ServerFnError> { /* ... */ }

// WebSocket
#[server]
async fn ws_handler() -> Result<Websocket, ServerFnError> { /* ... */ }
```

---

## Server Entry Point

### Single-Binary Fullstack

```rust
fn main() {
    dioxus::launch(app);
    // Automatically: server build runs serve(), client build runs launch()
}
```

### Custom Server Configuration

```rust
fn main() {
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        let router = dioxus::server::router(app);
        // Add axum layers, custom routes, etc.
        Ok(router)
    });

    #[cfg(not(feature = "server"))]
    dioxus::launch(app);
}
```

### Conditional Compilation

```rust
// Server-only code
#[cfg(feature = "server")]
fn server_only() { /* ... */ }

// Client-only code
#[cfg(feature = "web")]
fn client_only() { /* ... */ }
```

---

## SSR

### Out-of-Order Streaming

```rust
// Enable in your server setup
enable_out_of_order_streaming();
```

`SuspenseBoundary` components stream their content as resources resolve.

---

## Document Metadata (`dioxus-document`)

Manage `<head>` elements reactively.

```rust
use dioxus::prelude::*;

fn App() -> Element {
    rsx! {
        document::Title { "My App" }
        document::Meta { name: "description", content: "My cool app" }
        document::Link { rel: "canonical", href: "https://example.com" }
        document::Stylesheet { href: asset!("/assets/style.css") }
        document::Script { src: asset!("/assets/script.js") }
        document::Style { "body { background: #333; }" }
    }
}
```

### JavaScript Execution

Run JavaScript from Rust using `document::execute_js()`:

```rust
let result = document::execute_js("return document.title");
let title: String = result.await.unwrap();
```

---

## Feature Flags

```toml
[dependencies]
dioxus = { version = "0.7", features = ["fullstack"] }

[features]
default = []
web     = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile  = ["dioxus/mobile"]
server  = ["dioxus/server"]
```

### CLI Commands

```bash
dx serve --web                    # Dev server for web
dx serve --desktop                # Dev with desktop
dx serve --ios                    # Dev with iOS simulator
dx build --release --web          # Production build
dx bundle                         # Package for distribution
```
