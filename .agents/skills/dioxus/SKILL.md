---
name: dioxus
description: "Dioxus 0.7 reactive UI framework reference (covers dioxus, dioxus-hooks, dioxus-signals, dioxus-stores, dioxus-router, dioxus-fullstack, dioxus-document). Use when writing Dioxus components, signals, effects, resources, server functions, stores, routing, rsx macro code, or fullstack/SSR apps. Also use when the user mentions Dioxus, asks about reactive patterns in Dioxus, or works on any .rs file that imports from dioxus::prelude::*. Even if the question seems simple, consult this skill — Dioxus APIs change frequently between versions and training data is often wrong."
---

# Dioxus 0.7.3 — Framework Reference

**Version:** 0.7.3 (latest stable as of 2026-03-18)
**Docs:** [docs.rs/dioxus](https://docs.rs/dioxus/0.7.3/dioxus/) | [dioxuslabs.com/learn/0.7](https://dioxuslabs.com/learn/0.7/)

> This skill covers the `dioxus`, `dioxus-hooks`, `dioxus-signals`, `dioxus-stores`, `dioxus-router`, `dioxus-fullstack`, and `dioxus-document` crates.

## Standard Import

```rust
use dioxus::prelude::*;
```

## Quick Patterns

### Signal (reactive state)

```rust
let mut count = use_signal(|| 0);

// Read
count();              // clone value (callable syntax)
count.read();         // Ref<T> borrow
count.peek();         // read WITHOUT subscribing

// Write
count.set(5);         // replace
count += 1;           // arithmetic assignment
*count.write() = 10;  // mutable guard, triggers re-render on drop
```

`Signal<T>` is `Copy`. Reading subscribes to changes. Writing triggers re-renders of subscribers.

**Async safety:** Never hold `.read()` or `.write()` across an `.await` — it panics. Clone the value out first.

### Component

```rust
#[component]
fn Counter(initial: i32) -> Element {
    let mut count = use_signal(|| initial);
    rsx! {
        button { onclick: move |_| count += 1, "Count: {count}" }
    }
}
```

`Element` is `Result<VNode, RenderError>` — the `?` operator works in components.

### Memo (derived, cached)

```rust
let doubled = use_memo(move || count() * 2);
```

### Effect (side-effect)

```rust
use_effect(move || {
    println!("count = {}", count());
});
```

## ars-ui Translation Helpers

In `ars-dioxus`, prefer `t(MessageKey)` for ordinary inline translated text.
It is intentionally hookless: it reads `ArsProvider` context without consuming a
Dioxus hook slot, so it is safe inside conditional `rsx!` branches, iterator
closures, and render expressions. It still reads locale and signal-backed
message values reactively during render.

Use `use_t(MessageKey)` only when you need a reusable `Memo<String>`, such as a
translation passed to an API that stores a memo or an expensive parameterized
translation used repeatedly. Because `use_t` is a hook, call it
unconditionally at component top level.

## ars-ui Global Attributes

For ars-ui Dioxus adapter components, prefer
`#[props(extends = GlobalAttributes)] attrs: Vec<Attribute>` as the root HTML
attribute surface. Do not add explicit root `class`, `style`, `data-*`, `lang`,
`tabindex`, or extra `aria-*` props when global attributes already support the
same call-site syntax. Keep explicit props only for semantic component data,
typed vocabularies, non-root part attrs, or documented component-owned
precedence/validation. Tests should prove `class:` and `style:` forwarding
through global attrs when a component supports consumer styling.

For multi-part ars-ui components, prefer public compound part components over
root-level `*_class` / `*_style` prop families. Each stylable Dioxus part should
also use `#[props(extends = GlobalAttributes)]` and merge those attrs with the
agnostic part attrs. Tailwind examples should style those public parts directly
or with Tailwind arbitrary variants over `data-ars-*`; do not inject raw Rust
string CSS for ordinary part styling.
Name low-level primitive roots `Root` inside the component module, matching the
Checkbox standard (`checkbox::Root`, `field::Root`, `fieldset::Root`,
`form::Root`). Reserve semantic component names for future higher-level wrappers
or styled source templates.
Unstyled primitives still need styling hooks: evaluate every core `Part` enum
variant and every adapter-rendered structural node, including hidden inputs,
status regions, live regions, portals, anchors, and measurement wrappers. Expose
a public stylable part when consumers may need to style or position that node;
otherwise document why the node is intentionally private.

For required structural nodes, expose a public part when styling is expected but
keep an adapter fallback when the part is omitted. Suppress the fallback when
the explicit part is present, and keep required text, ids, ARIA, roles, and
relationships owned by the machine or adapter rather than by arbitrary consumer
children. In Dioxus, do not rely on a child component side effect to suppress a
parent fallback during the same SSR render; use an explicit prop-level choice or
inspect the child `Element`/`VNode` tree before rendering the fallback.

For machine-backed Dioxus compound parts that only need agnostic part attrs plus
consumer global attrs, use `UseMachineReturn::part_attrs` instead of writing a
component-local part attr merger. Keep local merge code only when a part adds
adapter-specific dynamic attrs, event handlers, refs, or renderer-only behavior.

Adapter crates (`ars-dioxus`) expose unstyled primitives only; unstyled does
not mean unstylable. Put checked-in
closed-anatomy styled Dioxus source templates in `ars-dioxus-components`, with
CSS and Tailwind variants when both distribution styles are needed. Styled
templates should compose adapter primitives and may expose semantic props plus
root `GlobalAttributes`, but not per-part prop families. Treat these templates
as the source for the future `ars-ui` CLI, which will copy editable component
source into user projects; do not design them as the final customization
boundary.

Organize styled templates category-first under
`src/<category>/<component>/`, for example
`src/input/checkbox/css.rs`, `src/input/checkbox/tailwind.rs`, and an adjacent
`src/input/checkbox/checkbox.css` for CSS variants. Do not add top-level
variant-first module trees like `css::checkbox` or `tailwind::checkbox`.
CSS variant files should include plain comments documenting which component
part or state each selector styles, so copied source remains easy for users to
customize.
Tailwind source templates should keep class strings inline in the rendered
`rsx!` markup rather than hiding them behind `const` identifiers, so copied
source remains editable and Tailwind-aware editor extensions can provide
completion and canonical-class diagnostics at the markup location.
Because styled templates are copied into user applications, template Rust files
must import ars-ui and framework APIs only with `use ars_dioxus::prelude::*;`.
Do not import directly from `dioxus`, `ars_forms`, or deep `ars_dioxus::*`
modules in copied-source templates. If a template needs a helper or type,
export it from the adapter prelude first and consume it from there.
Use `#[props(into)]` for Dioxus callback, text, and view-like props wherever
the macro supports it. Examples and styled templates should pass closures and
elements directly instead of spelling `EventHandler` wrappers or `.into()` at
each call site unless a reusable local or inference boundary requires it.

Plain Dioxus widgets should compose adapter primitives directly. CSS widgets
should import CSS styled source templates, and Tailwind widgets should import
Tailwind styled source templates. Do not use a CSS styled component inside the
plain widget just for visual polish.
Dioxus widget examples should import adapter/framework APIs through
`use ars_dioxus::prelude::*;` as much as possible. Avoid direct `dioxus::*` or
deep `ars_dioxus::*` imports in examples when the item is intentionally
available from the adapter prelude.

## Reference Files

Read the appropriate reference file based on what you're working on:

| Topic                      | File                         | When to read                                                                                             |
| -------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| **Reactive system**        | `references/reactive.md`     | Signals, memos, effects, resources, futures, coroutines, actions, stores (`#[derive(Store)]`)            |
| **Components & rsx macro** | `references/components.md`   | `#[component]`, props, children, context, `rsx!` syntax, event handlers, styling                         |
| **Control flow**           | `references/control-flow.md` | Conditionals, iteration, `SuspenseBoundary`, `ErrorBoundary`, `?` in components                          |
| **Fullstack & SSR**        | `references/fullstack.md`    | `#[server]`, `#[get]`/`#[post]`, SSR, `use_server_future`, `use_loader`, `use_action`, `dioxus-document` |
| **Router**                 | `references/router.md`       | `#[derive(Routable)]`, `Link`, `Outlet`, `#[nest]`, `#[layout]`, navigation                              |

## Key 0.7 Changes (vs 0.6)

- `Element` changed from `Option<VNode>` to `Result<VNode, RenderError>` — `?` now works in components
- `dioxus-lib` removed — use `dioxus` with `features = ["lib"]`
- Default server function codec changed from URL-encoded form to **JSON**
- New HTTP method macros: `#[get]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]`
- `use_action` hook for user-input-driven async with automatic cancellation
- `use_loader` hook for hybrid client/server fetching
- `#[derive(Store)]` for field-level reactive state (new `dioxus-stores` crate)
- Form submission now allowed by default — call `prevent_default()` to block
- Axum 0.8 integration
- Subsecond hot-patching across web, desktop, and mobile
- Scoped CSS and CSS modules (added in 0.7.3)
- Radix-UI component primitives (first-party)

## Platform Targets

```toml
[dependencies]
dioxus = { version = "0.7", features = ["fullstack"] }

[features]
web     = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile  = ["dioxus/mobile"]
server  = ["dioxus/server"]
```

```bash
dx serve --web       # WASM browser
dx serve --desktop   # native desktop (webview)
dx serve --ios       # iOS simulator
```

For ars-ui adapter behavior that depends on browser-owned semantics, classify
the target before coding. `web` can use typed DOM APIs such as `web_sys`.
`desktop`/WebView targets may have a DOM, but Rust usually reaches it through
`dioxus_document::eval` or another bridge, not through typed `web_sys`.
`server`/SSR has no live DOM. Keep public behavior stable where possible, but
document and test which target bucket owns exact native browser behavior such
as constraint validation, focus APIs, selection ranges, layout measurement,
clipboard, drag data, or file inputs.

## Common Pitfalls

1. **Never hold `.read()`/`.write()` across `.await`** — it panics at runtime. Clone first, then await.
2. **Always add `key:` when iterating** — `for item in items { div { key: "{item.id}", ... } }`
3. **`Element` is a `Result`** — use `?` for error propagation, errors bubble to `ErrorBoundary`.
4. **Props need `PartialEq + Clone`** — for `#[derive(Props)]` structs.
5. **Event handlers auto-spawn async** — you can write `onclick: move |_| async move { ... }` directly.
6. **`prevent_default()` is no longer automatic for forms** — call it explicitly in 0.7.
