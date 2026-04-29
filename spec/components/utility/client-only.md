---
component: ClientOnly
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  ark-ui: ClientOnly
---

# ClientOnly

`ClientOnly` is an SSR utility that renders its children only on the client (after hydration). During server-side rendering, it renders an optional fallback or nothing.

## 1. API

### 1.1 Props

```rust
/// Props for the `ClientOnly` component.
#[derive(Clone, Debug, Default)]
pub struct Props<Fallback = ()> {
    /// Optional fallback content rendered during SSR.
    /// When None, nothing is rendered on the server.
    pub fallback: Option<Fallback>,
}

impl<Fallback> Props<Fallback> {
    /// Create `ClientOnly` props with no fallback content.
    pub const fn new() -> Self {
        Self { fallback: None }
    }

    /// Set the fallback content rendered during SSR and initial hydration.
    pub fn fallback(mut self, fallback: Fallback) -> Self {
        self.fallback = Some(fallback);
        self
    }
}
```

`Fallback` is the framework-specific view or element type. The agnostic core does not name Leptos or Dioxus view types directly, and it does not require `Fallback: PartialEq + Eq`; adapter view types are often closure-backed or virtual-node-backed values where equality is not meaningful.

### 1.2 Connect / API

`ClientOnly` is a logical wrapper — it has no `Part` enum, no `ConnectApi`, and no `AttrMap` output. The adapter implements the SSR/hydration gating logic directly (see §5). Snapshot tests for `connect()` / `Api` `AttrMap` output are not applicable because this utility has no agnostic connect API.

## 2. Anatomy

```text
ClientOnly
└── {children}    (no DOM element rendered)
```

`ClientOnly` renders no wrapper element. It is a logical boundary: children render only after client-side hydration.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

`ClientOnly` has no ARIA semantics. The fallback content should be accessible if it represents meaningful placeholder content (e.g., a loading skeleton with `aria-busy="true"`).

## 4. Behavior

| Phase                                | Renders                                   |
| ------------------------------------ | ----------------------------------------- |
| SSR (server)                         | `fallback` if provided, otherwise nothing |
| Hydration (client, before mount)     | `fallback` if provided, otherwise nothing |
| Post-hydration (client, after mount) | `children`                                |

## 5. Implementation

```rust
// Leptos adapter:
#[component]
pub fn ClientOnly(
    #[prop(optional)] fallback: Option<ChildrenFn>,
    children: ChildrenFn,
) -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        fallback.map(|f| f())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let mounted = RwSignal::new(false);
        Effect::new(move |_| mounted.set(true));
        move || {
            if mounted.get() {
                children()
            } else {
                fallback.as_ref().map(|f| f())
            }
        }
    }
}

// Dioxus adapter:
#[component]
fn ClientOnly(fallback: Option<Element>, children: Element) -> Element {
    let mut mounted = use_signal(|| false);
    use_effect(move || mounted.set(true));
    if mounted() { rsx! { {children} } }
    else { rsx! { {fallback} } }
}
```

> **SSR Behavior:** Leptos uses `#[cfg(feature = "ssr")]` for compile-time gating of client-only content. Dioxus relies on `use_effect` not running during SSR (runtime gating). Both produce equivalent server output (fallback content only), but the mechanism differs.

## 6. Use Cases

- Components that depend on `window`, `document`, or browser APIs (e.g., Canvas, WebGL, `matchMedia`)
- Third-party scripts that must not run during SSR
- Content that differs between server and client (to avoid hydration mismatch warnings)

## 7. SSR Rendering Behavior and Waterfall Prevention

`ClientOnly` is designed to avoid hydration mismatches for browser-only APIs without introducing rendering waterfalls:

- **Server-side**: Renders nothing (empty fragment or HTML comment node `<!-- client-only -->`). If `fallback` is provided, renders the fallback content instead.
- **Client-side**: Renders children on the first client-side render cycle (after hydration completes). The `mounted` signal flips to `true` inside a `Effect::new` / `use_effect`, which runs after the initial hydration pass.
- **No waterfall**: `ClientOnly` does NOT delay rendering of surrounding content. Sibling and parent components render normally on the server and hydrate normally on the client. Only the ClientOnly subtree itself is deferred to the client — the rest of the page is unaffected.
- **Hydration mismatch avoidance**: Because the server renders nothing (or the fallback) and the client initially renders the same thing before the effect flips `mounted`, there is no mismatch between server HTML and initial client HTML.
- **`fallback` prop**: agnostic `Props<Fallback>` stores `fallback: Option<Fallback>`. Leptos maps `Fallback` to `ChildrenFn`; Dioxus maps it to `Element`. Use this for skeleton loaders, placeholder text, or `aria-busy="true"` containers that give users visual feedback while the client-only content loads. When `None`, the server output is empty.
- **Use cases**: `localStorage`, `window.matchMedia`, `navigator`, Canvas/WebGL, third-party browser scripts — any API that does not exist during SSR.

## 8. Library Parity

> Compared against: Ark UI (`ClientOnly`).

### 8.1 Props

| Feature  | ars-ui     | Ark UI                | Notes                                               |
| -------- | ---------- | --------------------- | --------------------------------------------------- |
| Fallback | `fallback` | `fallback` (children) | Both libraries support server-side fallback content |

**Gaps:** None.

### 8.2 Features

| Feature               | ars-ui | Ark UI |
| --------------------- | ------ | ------ |
| SSR fallback          | Yes    | Yes    |
| Client-only rendering | Yes    | Yes    |
| No wrapper element    | Yes    | Yes    |
| Hydration-safe        | Yes    | Yes    |

**Gaps:** None.

### 8.3 Summary

- **Overall:** Full parity.
- **Divergences:** None.
- **Recommended additions:** None.
