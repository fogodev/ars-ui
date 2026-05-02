---
adapter: dioxus
component: error-boundary
category: utility
source: components/utility/error-boundary.md
source_foundation: foundation/09-adapter-dioxus.md
---

# ErrorBoundary — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core
[`ErrorBoundary`](../../components/utility/error-boundary.md) wrapper to
Dioxus 0.7.x. The adapter wraps Dioxus's framework-native
[`ErrorBoundary`](https://docs.rs/dioxus-core/0.7/dioxus_core/struct.ErrorBoundary.html)
primitive and renders the canonical accessible fallback (`<div role="alert">`
with a localized heading and `<ul>`/`<li>` error list) defined in the core
spec. It owns:

- locale resolution for the heading text via `ArsProvider` and the shared
  `Messages` bundle
- conversion of Dioxus's single `Option<CapturedError>` from
  `ErrorContext::error()` into the canonical `<ul>`-of-`<li>` shape so the
  rendered DOM matches the Leptos adapter byte-for-byte
- emission of `data-ars-error-count` (always `0` or `1` for Dioxus, since
  the framework collapses multiple errors into the most recent one)

## 2. Public Adapter API

The adapter exposes everything through a single `error_boundary` module
(reachable via `use ars_dioxus::prelude::*;`). The module re-exports the
agnostic `ars_components::utility::error_boundary::*` surface (Messages,
Part, Api, attr helpers) alongside the Dioxus-side wrappers, so callers
spell every type with the same prefix:

```rust
#[derive(Props, Clone, PartialEq)]
pub struct BoundaryProps {
    /// Subtree wrapped by the boundary.
    pub children: Element,

    /// Optional override for the entire fallback closure. When `None`,
    /// `default_fallback` is used.
    #[props(optional, into)]
    pub fallback: Option<FallbackHandler>,

    /// Optional telemetry hook fired with each captured error.
    #[props(optional, into)]
    pub on_error: Option<EventHandler<CapturedError>>,

    /// Explicit `Messages` bundle override used when the bundle from the
    /// surrounding `ArsProvider` cannot be resolved or when a wrapper
    /// needs to inject custom wording.
    #[props(optional)]
    pub messages: Option<error_boundary::Messages>,

    /// Explicit locale override used for fallback heading resolution.
    #[props(optional)]
    pub locale: Option<Locale>,
}

#[component]
pub fn Boundary(props: BoundaryProps) -> Element;

/// Renders the canonical accessible fallback markup. Adapters call this
/// from inside `Boundary`'s default branch; consumers may also pass it
/// directly to Dioxus's framework `ErrorBoundary` if they do not want
/// the wrapper at all.
pub fn default_fallback(ctx: ErrorContext) -> Element;

/// Adapter-side fallback handler — a `Callback<ErrorContext, Element>`.
pub type FallbackHandler = Callback<ErrorContext, Element>;

pub use dioxus::CapturedError;
```

`Messages` and the framework-agnostic `error_boundary::Api` are re-exported
through this module so consumers reach every type through a single
namespace.

## 3. Mapping to Core Component Contract

- **Props parity:** the logical props from the core spec plus the adapter
  heading overrides (`children`, `fallback`, `on_error`, `messages`,
  `locale`) map 1:1 to adapter
  props. Dioxus props are declared in an explicit `#[derive(Props)]`
  struct per the
  [`feedback_dioxus_explicit_props.md`](../../../docs/feedback/feedback_dioxus_explicit_props.md)
  rule.
- **Event parity:** the only event surface is `on_error`, fired once per
  captured error episode. Dioxus's `ErrorContext` only exposes a single
  most-recent `Option<CapturedError>`, so the callback fires at most once
  per render pass.
- **Structure parity:** the adapter renders the `Root` / `Message` /
  `List` / `Item` parts with attrs from `error_boundary::Api` whenever
  the framework primitive enters its error state. When the boundary is
  in the no-error state, only `children` are rendered (no `Root` wrapper).

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source           | Notes                                                            |
| --------------------- | --------- | ------------------------ | ------------- | --------------------- | ---------------------------------------------------------------- |
| `Root`                | required  | `<div>` alert region     | adapter-owned | `api.root_attrs()`    | rendered only in the error-fallback branch                       |
| `Message`             | required  | `<p>` heading            | adapter-owned | `api.message_attrs()` | text is the resolved `Messages.message`                          |
| `List`                | required  | `<ul>`                   | adapter-owned | `api.list_attrs()`    | always present in fallback                                       |
| `Item`                | repeated  | `<li>`                   | adapter-owned | `api.item_attrs()`    | always exactly one entry on Dioxus (single-error `ErrorContext`) |

## 5. Locale and Messages Resolution

`Boundary` resolves the heading on every render through the standard ars-ui
priority order:

1. Explicit `messages` prop, if provided.
2. Bundle registered with `ArsProvider`'s `i18n_registries` for the active
   locale.
3. `Messages::default()` (English `"A component encountered an error."`).

The resolved `Locale` comes from the explicit `locale` prop when provided,
otherwise from `use_locale()` (from `ArsContext`). The final string is
rendered into the `Message` part via `rsx!`.

## 6. Error Iteration

Even though `ErrorContext::error()` returns `Option<CapturedError>`, the
adapter always wraps the result in a `<ul>`/`<li>` so the DOM contract
is identical to the Leptos adapter. The error-count attribute on `Root`
is `0` or `1`:

```rust,no_check
let error = ctx.error();
let api = error_boundary::Api::new(usize::from(error.is_some()));
let root_attrs = attr_map_to_dioxus_inline_attrs(api.root_attrs());
let msg_attrs = attr_map_to_dioxus_inline_attrs(api.message_attrs());
let list_attrs = attr_map_to_dioxus_inline_attrs(api.list_attrs());
let item_attrs = attr_map_to_dioxus_inline_attrs(api.item_attrs());

rsx! {
    div { ..root_attrs,
        p { ..msg_attrs, "{heading}" }
        ul { ..list_attrs,
            if let Some(e) = error {
                li { ..item_attrs, "{e}" }
            }
        }
    }
}
```

## 7. Fallback Override

When `fallback` is `Some`, the adapter forwards the framework's
`ErrorContext` to the user callback unchanged. The user's callback
returns an `Element` rendered in place of the default markup; none of the
canonical `data-ars-*` attributes are emitted.

When `fallback` is `None`, the adapter calls `default_fallback`, which is
the same renderer used internally — guaranteeing that consumers who skip
the wrapper (`ErrorBoundary { handle_error: ars_dioxus::utility::error_boundary::default_fallback, … }`)
get byte-identical markup.

## 8. Telemetry (`on_error`)

`Boundary`'s `handle_error` closure first inspects `ctx.error()` and, if
an error is present, invokes `on_error` (if set) before rendering the
fallback. The callback receives the `CapturedError` itself; consumers can
inspect `Display` / `std::error::Error` impls or downcast through the
underlying `anyhow::Error` (`ctx.error().unwrap().0` is `Arc<anyhow::Error>`).

Because Dioxus only retains the most recent error, `on_error` fires at
most once per render pass for the same error episode. Consumers who need
to observe sequences of errors should clear the boundary
(`ctx.clear_errors()`) inside their custom `fallback` and capture errors
on each insertion.
