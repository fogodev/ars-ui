---
adapter: leptos
component: error-boundary
category: utility
source: components/utility/error-boundary.md
source_foundation: foundation/08-adapter-leptos.md
---

# ErrorBoundary — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core
[`ErrorBoundary`](../../components/utility/error-boundary.md) wrapper to
Leptos 0.8.x. The adapter wraps Leptos's framework-native
[`ErrorBoundary`](https://docs.rs/leptos/0.8/leptos/error/fn.ErrorBoundary.html)
primitive and renders the canonical accessible fallback (`<div role="alert">`
with a localized heading and `<ul>`/`<li>` error list) defined in the core
spec. It owns:

- locale resolution for the heading text via `ArsProvider` and the shared
  `Messages` bundle
- iteration over the multi-error `ArcRwSignal<Errors>` produced by the
  framework primitive
- emission of `data-ars-error-count` so the rendered DOM matches the
  Dioxus adapter byte-for-byte for the same number of caught errors

## 2. Public Adapter API

The adapter exposes everything through a single `error_boundary` module
(reachable via `use ars_leptos::prelude::*;`). The module re-exports the
agnostic `ars_components::utility::error_boundary::*` surface (Messages,
Part, Api, attr helpers) alongside the Leptos-side wrappers, so callers
spell every type with the same prefix:

```rust,no_check
#[component]
pub fn Boundary<T: 'static>(
    /// Optional override for the entire fallback closure. When `None`,
    /// `default_fallback` is used.
    #[prop(optional, into)] fallback: Option<FallbackHandler>,

    /// Optional telemetry hook fired with each captured error.
    #[prop(optional, into)] on_error: Option<Callback<CapturedError>>,

    /// Explicit `Messages` bundle override used when the bundle from the
    /// surrounding `ArsProvider` cannot be resolved or when a wrapper
    /// needs to inject custom wording.
    #[prop(optional)] messages: Option<error_boundary::Messages>,

    /// Explicit locale override used for fallback heading resolution.
    #[prop(optional)] locale: Option<Locale>,

    /// Subtree wrapped by the boundary.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView;

/// Renders the canonical accessible fallback markup. Adapters call this
/// from inside `Boundary`'s default branch; consumers may also pass it
/// directly to Leptos's framework `ErrorBoundary` if they do not want
/// the wrapper at all.
pub fn default_fallback(errors: ArcRwSignal<Errors>) -> impl IntoView;

/// Typed fallback closure used by the `fallback` prop.
pub struct FallbackHandler { /* private fields */ }

impl FallbackHandler {
    /// Create a fallback handler from a typed Leptos view-producing closure.
    pub fn new<F, V>(fallback: F) -> Self
    where
        F: Fn(ArcRwSignal<Errors>) -> V + Send + Sync + 'static,
        V: RenderHtml + Send + 'static;
}

impl<F, V> From<F> for FallbackHandler
where
    F: Fn(ArcRwSignal<Errors>) -> V + Send + Sync + 'static,
    V: RenderHtml + Send + 'static;

/// A single captured error from the Leptos `Errors` collection.
pub use leptos::error::Error as CapturedError;
```

`Messages` and the framework-agnostic `error_boundary::Api` are re-exported
through this module so consumers reach every type through a single
namespace.

## 3. Mapping to Core Component Contract

- **Props parity:** the logical props from the core spec plus the adapter
  heading overrides (`children`, `fallback`, `on_error`, `messages`,
  `locale`) map 1:1 to adapter props.
- **Event parity:** the only event surface is `on_error`, fired once per
  captured error episode. Multi-error `Errors` collections fire `on_error`
  for each `(ErrorId, Error)` pair on insertion.
- **Structure parity:** the adapter renders the `Root` / `Message` /
  `List` / `Item` parts with attrs from `error_boundary::Api` whenever
  the framework primitive enters its error state. When the boundary is
  in the no-error state, only `children` are rendered (no `Root` wrapper).

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source           | Notes                                      |
| --------------------- | --------- | ------------------------ | ------------- | --------------------- | ------------------------------------------ |
| `Root`                | required  | `<div>` alert region     | adapter-owned | `api.root_attrs()`    | rendered only in the error-fallback branch |
| `Message`             | required  | `<p>` heading            | adapter-owned | `api.message_attrs()` | text is the resolved `Messages.message`    |
| `List`                | required  | `<ul>`                   | adapter-owned | `api.list_attrs()`    | always present in fallback                 |
| `Item`                | repeated  | `<li>`                   | adapter-owned | `api.item_attrs()`    | one per `(ErrorId, Error)` entry           |

## 5. Locale and Messages Resolution

`Boundary` resolves the heading on every render through the standard ars-ui
priority order:

1. Explicit `messages` prop, if provided.
2. Bundle registered with `ArsProvider`'s `i18n_registries` for the active
   locale.
3. `Messages::default()` (English `"A component encountered an error."`).

The resolved `Locale` comes from the explicit `locale` prop when provided,
otherwise from `use_locale()` (from `ArsContext`). The final string is
rendered into the `Message` part via `view!`.

## 6. Error Iteration

```rust,no_check
let api = error_boundary::Api::new(errors.get().len());
view! {
    <div {..attr_map_to_leptos_inline_attrs(api.root_attrs())}>
        <p {..attr_map_to_leptos_inline_attrs(api.message_attrs())}>{heading}</p>
        <ul {..attr_map_to_leptos_inline_attrs(api.list_attrs())}>
            {move || errors.get()
                .into_iter()
                .map(|(_, e)| view! {
                    <li {..attr_map_to_leptos_inline_attrs(api.item_attrs())}>
                        {e.to_string()}
                    </li>
                })
                .collect_view()
            }
        </ul>
    </div>
}
```

The adapter relies on the `attr_map_to_leptos_inline_attrs` helper from
`ars_leptos::*` to convert each `AttrMap` into an inline-attribute spread.

## 7. Fallback Override

When `fallback` is `Some`, the adapter forwards the framework's
`ArcRwSignal<Errors>` to the user closure unchanged. The user's closure
returns any typed Leptos view rendered in place of the default markup; none
of the canonical `data-ars-*` attributes are emitted. The adapter performs
the internal view erasure needed by Leptos's native `ErrorBoundary`
primitive after the user closure runs, so consumers do not have to name
Leptos's type-erased view type in their fallback API.

When `fallback` is `None`, the adapter calls `default_fallback`, which is
the same renderer used internally — guaranteeing that consumers who skip
the wrapper (`<ErrorBoundary fallback=ars_leptos::utility::error_boundary::default_fallback>`)
get byte-identical markup.

## 8. Telemetry (`on_error`)

`Boundary` installs a one-shot effect that watches `errors` and fires
`on_error` whenever a new `(ErrorId, Error)` is inserted. The callback
receives the `Error` itself; consumers can inspect `Display` /
`std::error::Error` impls or downcast as needed.

The callback runs inside the active reactive owner; signal mutation
inside the callback is allowed but discouraged. A common pattern is to
forward the error to a non-reactive monitoring crate.
