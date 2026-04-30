---
component: ErrorBoundary
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: [ars-provider]
---

# ErrorBoundary

ErrorBoundary is the canonical specification for the shared error-boundary wrapper. It owns the
framework-agnostic data model (parts, attribute helpers, message bundle) consumed by the
Dioxus and Leptos adapter components named `error_boundary::Boundary`.

The component wraps a subtree in the framework-native error boundary primitive
(`dioxus_core::ErrorBoundary` / `leptos::error::ErrorBoundary`) and renders a single,
**adapter-symmetric** accessible fallback UI when a descendant fails to render. The fallback
is a `<div role="alert">` containing a localized heading paragraph and a `<ul>` of
`<li>` entries — one per captured error — exposing `data-ars-error-count` so consumers can
branch on the multi-error case from a CSS selector alone.

ErrorBoundary owns the shared `Messages` bundle for the heading wording but does **not**
resolve user-facing wording inside the connect API. Adapter wrappers resolve the locale
through the surrounding `ArsProvider` and pass the final string into the rendered markup.

## 1. API

### 1.1 Props

`ErrorBoundary` is rendered through the per-adapter `Boundary` component. Its props are
declared in adapter-native shapes (see `08-adapter-leptos.md` §17 and `09-adapter-dioxus.md`
§21 for the literal Rust signatures), but the **logical contract** is uniform:

| Prop       | Required | Description                                                                                              |
| ---------- | :------: | -------------------------------------------------------------------------------------------------------- |
| `children` |    ✓     | Subtree to wrap. Adapter-native type (`Element` for Dioxus, `Children` for Leptos).                      |
| `fallback` |          | Override the default fallback renderer. When `None`, the wrapper renders `default_fallback`.             |
| `on_error` |          | Telemetry hook fired with each captured error so apps can ship to monitoring services (Sentry, …).       |
| `messages` |          | Override the default `Messages` bundle. When `None`, the wrapper resolves from `ArsProvider` registries. |

`children` is the only required prop. The other three are optional escape hatches; passing
nothing yields the canonical accessible default with a localized heading. The full Rust
signatures live in the adapter specs because props in Dioxus and Leptos require
framework-native types (`Element` vs `Children`, `Callback` vs `EventHandler`, `Errors`
collection vs `ErrorContext`).

### 1.2 Connect / API

```rust
/// DOM parts of the error-boundary fallback.
#[derive(ComponentPart)]
#[scope = "error-boundary"]
pub enum Part {
    /// The accessible alert container (`<div role="alert">`).
    Root,

    /// The static heading paragraph rendered above the list of errors.
    Message,

    /// The `<ul>` listing every captured error.
    List,

    /// An individual `<li>` entry for one captured error's `Display` text.
    Item,
}

/// Localizable strings rendered inside the default fallback UI.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Static heading rendered above the `<ul>` of error entries.
    pub message: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            message: MessageFn::static_str("A component encountered an error."),
        }
    }
}

impl ComponentMessages for Messages {}

/// Stateless connect API for the canonical fallback container attributes.
pub struct Api {
    error_count: usize,
}

impl Api {
    pub const fn new(error_count: usize) -> Self;

    /// Returns root container attributes:
    /// `role="alert"`, `aria-live="assertive"`, `aria-atomic="true"`,
    /// `data-ars-scope="error-boundary"`, `data-ars-part="root"`,
    /// `data-ars-error="true"`, `data-ars-error-count="{error_count}"`.
    pub fn root_attrs(&self) -> AttrMap;

    /// Returns scope/part attrs for the static heading paragraph.
    pub fn message_attrs(&self) -> AttrMap;

    /// Returns scope/part attrs for the enclosing `<ul>`.
    pub fn list_attrs(&self) -> AttrMap;

    /// Returns scope/part attrs for one `<li>` error entry.
    pub fn item_attrs(&self) -> AttrMap;

    /// Returns the number of errors this fallback is rendering.
    pub const fn error_count(&self) -> usize;
}

impl ConnectApi for Api {
    type Part = Part;
    fn part_attrs(&self, part: Self::Part) -> AttrMap;
}

/// Free-function attribute helpers for adapters that do not need a full Api instance.
pub fn message_attrs() -> AttrMap;
pub fn list_attrs() -> AttrMap;
pub fn item_attrs() -> AttrMap;
```

The connect API is stateless and behaviour-free. Adapters build a fresh `Api` from the
current error count on every render, then merge the part attrs with the framework-native
markup.

## 2. Anatomy

```text
Boundary
└── (no error)  → renders {children}
└── (caught error) → fallback
    ├── Message  <p>           static localized heading
    └── List     <ul>
        └── Item <li>          one per captured error, repeated
```

| Part      | Element | Key attributes                                                                                                                                                                      |
| --------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`    | `<div>` | `role="alert"`, `aria-live="assertive"`, `aria-atomic="true"`, `data-ars-scope="error-boundary"`, `data-ars-part="root"`, `data-ars-error="true"`, `data-ars-error-count="{count}"` |
| `Message` | `<p>`   | `data-ars-scope="error-boundary"`, `data-ars-part="message"`                                                                                                                        |
| `List`    | `<ul>`  | `data-ars-scope="error-boundary"`, `data-ars-part="list"`                                                                                                                           |
| `Item`    | `<li>`  | `data-ars-scope="error-boundary"`, `data-ars-part="item"`                                                                                                                           |

The Dioxus framework's `ErrorContext` exposes a single `Option<CapturedError>`, so the
Dioxus adapter renders zero or one `<li>` entries. The Leptos framework's `Errors`
collection is multi-valued, so the Leptos adapter renders one `<li>` per captured error.
Both adapters always wrap the entries in `<ul>` so consumers see the same DOM shape and
the same `data-ars-error-count` semantics regardless of which framework caught the error.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

The fallback container declares itself as an alert region via `role="alert"`. The
explicit `aria-live="assertive"` and `aria-atomic="true"` attributes are redundant with
`role="alert"` for first render, but they are emitted explicitly so:

1. Screen readers re-announce the alert region atomically when a retry-then-fail flow
   inserts a fresh error after `clear_errors()` (the implicit `aria-live` from
   `role="alert"` does not always reannounce on subtree mutation).
2. CSS selectors and tests can target `[aria-live]` independently of the role attribute.

The boundary intentionally omits keyboard-navigation primitives. A custom `fallback`
prop is the right place to add a "Try again" button or a link to a help page; the
default fallback is read-only.

### 3.2 Screen Reader Announcements

Because `role="alert"` is implicitly an `aria-live="assertive"` polite-region, screen
readers announce the heading immediately on insertion. The list of error strings follows
the heading in DOM order so reading-order traversal remains linear.

## 4. Behavior

| Trigger                                      | Action                                                                                                         |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Descendant component returns `Err` / panics  | Framework-native `ErrorBoundary` captures the error; Boundary fires `on_error` (if set), then renders fallback |
| `Errors::clear_errors()` invoked from caller | The framework-native primitive re-renders children; Boundary returns to the no-error path                      |
| Children change identity (router swap)       | Framework-native re-render replays children; Boundary clears its own errors automatically                      |

The wrapper does not introduce additional control flow on top of the framework primitive.
It is an attribute-and-message shim, not a state machine.

## 5. Integration

```rust
// Dioxus
use ars_dioxus::prelude::*;

rsx! {
    error_boundary::Boundary {
        // No props — fallback uses the localized default.
        ChildComponent {}
    }

    error_boundary::Boundary {
        on_error: |err| sentry::capture_error(&err),
        ChildComponent {}
    }

    error_boundary::Boundary {
        fallback: |ctx| rsx! { div { class: "my-error-ui", "Try again" } },
        ChildComponent {}
    }
}
```

```rust
// Leptos
use ars_leptos::prelude::*;

view! {
    <error_boundary::Boundary>
        <ChildComponent/>
    </error_boundary::Boundary>

    <error_boundary::Boundary on_error=move |err| sentry::capture_error(&err)>
        <ChildComponent/>
    </error_boundary::Boundary>
}
```

The `default_fallback` helper is exposed by each adapter so consumers who do not want
the wrapper can still get the canonical accessible markup by passing it directly to the
framework's own `ErrorBoundary`:

```rust,no_check
// Dioxus — usage shown inside an `rsx!` invocation:
ErrorBoundary {
    handle_error: ars_dioxus::error_boundary::default_fallback,
    ChildComponent {}
}

// Leptos — usage shown inside a `view!` invocation:
<ErrorBoundary fallback=ars_leptos::error_boundary::default_fallback>
    <ChildComponent/>
</ErrorBoundary>
```

## 6. Internationalization

### 6.1 Messages

`Messages` carries one localizable string — the static heading rendered above the
error list. The default English text is `"A component encountered an error."`.

Adapter wrappers resolve the bundle in this priority order, matching the
`Dismissable` / `ArsProvider` convention:

1. Explicit `messages` prop on the `Boundary` component.
2. Bundle registered via `ArsProvider`'s `i18n_registries` for the active locale.
3. The framework-agnostic `Messages::default()` (English).

The resolved heading is rendered into the `Message` part as plain text. The error
strings rendered into each `Item` come from each `CapturedError`'s `Display`
implementation and are **not** translated — they are intended for developer
diagnostics, not end-user communication. Apps that need localized end-user error
messages should provide a custom `fallback` that maps domain errors to translated
strings.

## 7. Testing

The component is exercised through the Dioxus and Leptos adapter wrappers. Every
adapter test must cover:

- **Happy path** — children render when no descendant errors; the fallback is absent.
- **Caught error** — fallback renders with `role="alert"`, the static heading from
  `Messages`, and at least one `<li>` per captured error.
- **`data-ars-error-count`** — equals the number of captured errors and survives
  retries (`clear_errors`).
- **`on_error` telemetry** — fires once per captured error episode with the
  `Display`-able error.
- **Custom `fallback`** — the wrapper does not render the default markup when the
  caller supplies its own renderer.
- **Boundary containment** — the error does not propagate past the wrapper; siblings
  outside the boundary continue to render.

SSR string-rendering is the canonical test path for the unit tier:

- **Dioxus** — `dioxus_ssr::render` exercises the renderer's SSR protocol; a
  separate desktop smoke test (`tests/desktop_error_boundary.rs`) drives a
  `VirtualDom` directly through `DesktopHarness` so the runtime path used by
  Desktop, mobile, and SSR builds is also covered, including signal-mutation
  reactivity that single-pass SSR cannot observe.
- **Leptos** — `View::to_html` _is_ the framework primitive's SSR protocol; the
  same `<ErrorBoundary>` / `Owner` machinery runs on every adapter target. The
  internal `run_fallback` dispatch helper has direct unit tests for the
  on-error iteration and custom-fallback delegation paths, since SSR collapses
  multi-error payloads to the most-recent entry and would hide the loop's
  per-error behaviour.
