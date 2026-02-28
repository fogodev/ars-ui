# Leptos 0.8.17 — Router Reference

**Crate:** `leptos_router`

## Setup

```rust
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::hooks::*;
use leptos_router::path;
```

## Core Components

### Router + Routes

```rust
view! {
    <Router>
        <nav>
            <a href="/">"Home"</a>
            <A href="/contacts">"Contacts"</A>
        </nav>
        <main>
            <Routes fallback=|| "Not found.">
                <Route path=path!("/") view=HomePage/>
                <Route path=path!("/about") view=About/>
            </Routes>
        </main>
    </Router>
}
```

`FlatRoutes` — for flat (non-nested) route lists; slightly more efficient than `Routes`.

### Path Segments

```rust
path!("/")                  // exact root
path!("/static")            // static segment
path!("/user/:id")          // named param
path!("/files/*path")       // wildcard (catches rest)
path!("/maybe/:opt?")       // optional param
```

### Nested Routes (ParentRoute + Outlet)

```rust
<ParentRoute path=path!("/contacts") view=ContactsLayout>
    <Route path=path!("/") view=ContactList/>
    <Route path=path!(":id") view=ContactDetail/>
    <ParentRoute path=path!(":id") view=ContactInfo>
        <Route path=path!("") view=|| "Info"/>
        <Route path=path!("conversations") view=|| "Conversations"/>
    </ParentRoute>
</ParentRoute>
```

The parent component **must** render `<Outlet/>` where children appear:

```rust
#[component]
fn ContactsLayout() -> impl IntoView {
    view! {
        <h1>"Contacts"</h1>
        <Outlet/>
    }
}
```

### ProtectedRoute

```rust
<ProtectedRoute
    path=path!("/admin")
    view=AdminPage
    condition=move || is_logged_in.get()
    redirect_path=path!("/login")
/>

// Nested version
<ProtectedParentRoute
    path=path!("/dashboard")
    view=DashboardLayout
    condition=move || is_admin.get()
    redirect_path=path!("/")
>
    <Route path=path!("/") view=DashboardHome/>
    <Route path=path!("/settings") view=DashboardSettings/>
</ProtectedParentRoute>
```

### Link (`<A>`)

```rust
<A href="/contact/42">"Go to Contact"</A>
<A href="/about" exact=true active_class="active">"About"</A>
```

Plain `<a>` tags also work — the router intercepts them.

### Redirect

```rust
<Redirect path="/login"/>
```

Server-side: HTTP 302. Client-side: client navigation.

### Form

Client-side navigation form. GET navigates to encoded URL. POST sends POST request.

```rust
<Form method="GET" action="/search">
    <input type="text" name="q"/>
    <button type="submit">"Search"</button>
</Form>
```

## Hooks

### use_navigate

```rust
let navigate = use_navigate();
navigate("/dashboard", NavigateOptions::default());
navigate("/login", NavigateOptions { replace: true, ..Default::default() });
```

### use_location

```rust
let location = use_location();
let path = move || location.pathname.get();
let search = move || location.search.get();
let hash = move || location.hash.get();
```

### use_params / use_params_map

```rust
// Typed params
#[derive(Params, PartialEq, Clone)]
struct ContactParams {
    id: Option<usize>,
}

let params = use_params::<ContactParams>();
let id = move || params.read().as_ref().ok().and_then(|p| p.id).unwrap_or(0);

// Untyped params
let params = use_params_map();
let id = move || params.read().get("id").unwrap_or_default();
```

### use_query / use_query_map

Same API shape as params but reads from URL query string `?key=value`.

```rust
#[derive(Params, PartialEq, Clone)]
struct SearchQuery {
    q: Option<String>,
    page: Option<u32>,
}

let query = use_query::<SearchQuery>();
```

## Params Derive

```rust
#[derive(Params, PartialEq, Clone)]
struct MyParams {
    user_id: Option<u64>,
    tab: Option<String>,
}
```

Uses `FromStr` on each field. Use `Option<T>` for optional params.

## Full App Skeleton

```rust
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav>
                <A href="/">"Home"</A>
                <A href="/contacts">"Contacts"</A>
            </nav>
            <main>
                <Routes fallback=|| "Not Found">
                    <Route path=path!("/") view=HomePage/>
                    <ParentRoute path=path!("/contacts") view=ContactsLayout>
                        <Route path=path!("/") view=ContactList/>
                        <Route path=path!(":id") view=ContactDetail/>
                    </ParentRoute>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn ContactsLayout() -> impl IntoView {
    view! {
        <h2>"Contacts"</h2>
        <Outlet/>
    }
}

#[component]
fn ContactDetail() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").unwrap_or_default();
    view! { <p>"Contact ID: " {id}</p> }
}
```
