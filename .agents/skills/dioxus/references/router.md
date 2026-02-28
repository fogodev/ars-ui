# Dioxus 0.7.3 — Router Reference

**Crate:** `dioxus-router`

The router is **enum-driven**. Route variants map to components; parameters map to variant fields.

## Setup

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| rsx! { Router::<Route> {} });
}
```

## Route Enum

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/blog/:id")]
    BlogPost { id: String },

    #[route("/users/:user_id/posts/:post_id")]
    UserPost { user_id: u32, post_id: u32 },
}
```

### Route Segments

| Pattern    | Example     | Description                           |
| ---------- | ----------- | ------------------------------------- |
| `/static`  | `/about`    | Static segment                        |
| `/:name`   | `/:id`      | Dynamic param — maps to variant field |
| `/:..rest` | `/:..path`  | Catch-all (rest of URL)               |
| `?:key`    | `?:search`  | Query parameter                       |
| `#:field`  | `#:section` | Hash fragment                         |

## Route Components

Each route variant gets a component. **Parameters become props automatically:**

```rust
#[component]
fn BlogPost(id: String) -> Element {
    rsx! { h1 { "Post: {id}" } }
}

#[component]
fn Home() -> Element {
    rsx! { h1 { "Home" } }
}
```

## Layouts with `#[layout]`

Layouts wrap nested routes. Use `Outlet` to render the active child.

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(NavLayout)]
        #[route("/")]
        Home {},

        #[route("/about")]
        About {},
    // #[end_layout] is implicit at end
}

#[component]
fn NavLayout() -> Element {
    rsx! {
        nav {
            Link { to: Route::Home {}, "Home" }
            Link { to: Route::About {}, "About" }
        }
        Outlet::<Route> {}    // child route renders here
    }
}
```

## Nesting with `#[nest]`

Group routes under a common path prefix:

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[nest("/blog")]
        #[layout(BlogLayout)]
            #[route("/")]
            BlogList {},

            #[route("/:id")]
            BlogPost { id: String },
        #[end_layout]
    #[end_nest]

    #[route("/")]
    Home {},
}

#[component]
fn BlogLayout() -> Element {
    rsx! {
        h1 { "Blog" }
        Outlet::<Route> {}
    }
}
```

## Link Component

Type-safe navigation using route variants:

```rust
rsx! {
    Link { to: Route::Home {}, "Go home" }
    Link { to: Route::BlogPost { id: "my-post".into() }, "Read post" }
}
```

## Navigation Controls

```rust
rsx! {
    GoBackButton { "Back" }
    GoForwardButton { "Forward" }
}
```

### Programmatic Navigation

```rust
let nav = navigator();
nav.push(Route::BlogPost { id: "new".into() });
nav.replace(Route::Home {});
nav.go_back();
nav.go_forward();
```

## Outlet Context

Pass data from a layout to its child routes:

```rust
#[component]
fn Layout() -> Element {
    use_context_provider(|| SharedData::new());
    rsx! { Outlet::<Route> {} }
}

fn ChildRoute() -> Element {
    let data = use_outlet_context::<SharedData>();
    rsx! { "Data: {data}" }
}
```

## Full App Example

```rust
use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(MainLayout)]
        #[route("/")]
        Home {},

        #[nest("/blog")]
            #[layout(BlogLayout)]
                #[route("/")]
                BlogList {},

                #[route("/:id")]
                BlogPost { id: String },
            #[end_layout]
        #[end_nest]
    // implicit #[end_layout] for MainLayout
}

fn main() {
    dioxus::launch(|| rsx! { Router::<Route> {} });
}

#[component]
fn MainLayout() -> Element {
    rsx! {
        nav {
            Link { to: Route::Home {}, "Home" }
            Link { to: Route::BlogList {}, "Blog" }
        }
        main { Outlet::<Route> {} }
    }
}

#[component]
fn BlogLayout() -> Element {
    rsx! {
        h2 { "Blog" }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! { h1 { "Welcome" } }
}

#[component]
fn BlogList() -> Element {
    rsx! {
        ul {
            li { Link { to: Route::BlogPost { id: "hello".into() }, "Hello World" } }
            li { Link { to: Route::BlogPost { id: "dioxus".into() }, "About Dioxus" } }
        }
    }
}

#[component]
fn BlogPost(id: String) -> Element {
    rsx! { h3 { "Post: {id}" } }
}
```
