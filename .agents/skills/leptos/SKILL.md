---
name: leptos
description: "Leptos 0.8 reactive web framework reference (covers leptos, leptos_router, reactive_stores, server_fn, leptos_meta). Use when writing Leptos components, signals, effects, resources, actions, server functions, stores, routing, view macro code, or SSR/hydration. Also use when the user mentions Leptos, asks about reactive patterns in Leptos, or works on any .rs file that imports from leptos::*. Even if the question seems simple, consult this skill — Leptos APIs change frequently and training data is often wrong."
---

# Leptos 0.8.17 — Framework Reference

**Version:** 0.8.17 (latest stable as of 2026-03-18)
**Docs:** [docs.rs/leptos](https://docs.rs/leptos/0.8.17/leptos/) | [book.leptos.dev](https://book.leptos.dev)

> This skill covers the `leptos`, `reactive_graph`, `reactive_stores`, `leptos_router`, `server_fn`, and `leptos_meta` crates.

## Standard Import

```rust
use leptos::prelude::*;
```

## Quick Patterns

### Signals (read-only + write-only pair)

```rust
let (count, set_count) = signal(0i32);       // ReadSignal + WriteSignal (both Copy)
set_count.set(5);
set_count.update(|n| *n += 1);
let val = count.get();                        // clone + subscribe
```

`ReadSignal<T>` is **read-only**. `WriteSignal<T>` is **write-only**. They are separate types.

For a combined read+write handle, use `RwSignal`:

```rust
let count = RwSignal::new(0i32);             // single handle, read + write
count.set(1);
let val = count.get();
```

For `!Send` types (browser JS objects), use `signal_local()` / `RwSignal::new_local()`.

### Component

```rust
#[component]
pub fn Counter(initial: i32) -> impl IntoView {
    let (count, set_count) = signal(initial);
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            "Count: " {count}
        </button>
    }
}
```

Component functions run **once** (setup). Reactive closures re-run automatically.

### Memo (derived, cached)

```rust
let doubled = Memo::new(move |_| count.get() * 2);
```

### Effect (side-effect)

```rust
Effect::new(move |_| {
    log::info!("count = {}", count.get());
});
```

Do NOT write to signals inside effects — use a memo or derived signal instead.

## Reference Files

Read the appropriate reference file based on what you're working on:

| Topic                       | File                         | When to read                                                                                                                |
| --------------------------- | ---------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| **Reactive system**         | `references/reactive.md`     | Signals, memos, effects, resources, actions, reactive stores (`#[derive(Store)]`), the `Patch` system                       |
| **Components & view macro** | `references/components.md`   | `#[component]`, props, children, slots, context, `view!` syntax, builder syntax, NodeRef                                    |
| **Control flow**            | `references/control-flow.md` | `<Show>`, `<For>`, `<Suspense>`, `<Transition>`, `<ErrorBoundary>`, conditional rendering                                   |
| **Server & SSR**            | `references/server.md`       | `#[server]` functions, codecs, custom errors, SSR modes, islands (`#[island]`), `leptos_axum`/`leptos_actix`, `leptos_meta` |
| **Router**                  | `references/router.md`       | `<Router>`, `<Routes>`, `path!()`, params, nested routing, `<Form>`, navigation hooks                                       |

## Key 0.8 Changes (vs 0.7 and earlier)

- `create_signal` -> `signal()`, `create_memo` -> `Memo::new`, `create_effect` -> `Effect::new`, `create_resource` -> `Resource::new`, `create_action` -> `Action::new`
- `FromServerFnError` trait for custom server function error types
- Axum 0.8 support in `leptos_axum`
- Islands router feature
- WebSocket server functions
- `--cfg=erase_components` for faster dev builds
- `prop:value` required for reactive input binding (not the `value` HTML attribute)
- `on:input:target` typed event syntax: `on:input:target=move |ev| set_name.set(ev.target().value())`
- `bind:value`, `bind:checked`, `bind:group` two-way binding shorthands

## Common Pitfalls

1. **Signal is NOT read+write.** `signal()` returns `(ReadSignal, WriteSignal)` — two separate types. Use `RwSignal::new()` for a combined handle.
2. **Don't write signals in effects.** This causes reactive loops. Derive instead: `let b = move || a.get() > 5;`
3. **Use `prop:value` for reactive inputs**, not the `value` attribute. The attribute only sets the initial value.
4. **Avoid `usize`/`isize` in server function args** — WASM is 32-bit, server is 64-bit.
5. **Component fns run once.** Put reactive logic in closures, not at the top level.

## ars-ui Part Styling

For multi-part ars-ui Leptos adapter components, prefer public compound part
components over root-level `*_class` / `*_style` prop families. Each stylable
part should expose reactive `class: Option<TextProp>` and
`style: Option<TextProp>` until ars-leptos has a broader global-attributes
surface. Merge those attrs with the agnostic part attrs. Tailwind examples
should style public parts directly or with Tailwind arbitrary variants over
`data-ars-*`; do not inject raw Rust string CSS for ordinary part styling.

For machine-backed Leptos compound parts that only need agnostic part attrs plus
consumer `class` / `style` props, use `UseMachineReturn::part_attrs` instead of
writing a component-local part attr merger. Keep local merge code only when a
part adds adapter-specific dynamic attrs, event handlers, refs, or
renderer-only behavior.

For Leptos compound parts that add local dynamic attrs before rendering, use the
shared `apply_part_attrs` helper as the final step instead of reimplementing the
consumer `class` / `style` merge and AttrMap conversion.

For dynamic Leptos attrs derived from machine part `AttrMap`s, use
`UseMachineReturn::attr_string_memo`,
`UseMachineReturn::attr_optional_string_memo`, and
`UseMachineReturn::attr_presence_memo`. Do not add per-component
`root_attr_*_memo`, `input_attr_*_memo`, or `attr_*_memo` copies unless the
derivation does more than read one `AttrMap` key.

Adapter crates (`ars-leptos`) expose unstyled primitives only. Put checked-in
closed-anatomy styled Leptos source templates in `ars-leptos-components`, with
CSS and Tailwind variants when both distribution styles are needed. Styled
templates should compose adapter primitives and may expose semantic props plus
root `class`/`style`, but not per-part prop families. Treat these templates as
the source for the future `ars-ui` CLI, which will copy editable component
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
`view!` markup rather than hiding them behind `const` identifiers, so copied
source remains editable and Tailwind-aware editor extensions can provide
completion and canonical-class diagnostics at the markup location.
Because styled templates are copied into user applications, template Rust files
must import ars-ui and framework APIs only with `use ars_leptos::prelude::*;`.
Do not import directly from `leptos`, `ars_forms`, or deep `ars_leptos::*`
modules in copied-source templates. If a template needs a helper or type,
export it from the adapter prelude first and consume it from there.
Use `#[prop(into)]` for Leptos callback, signal, text, and view props wherever
the macro supports it. Examples and styled templates should pass closures,
signals, translated memos, and view closures directly instead of spelling
`Callback::new(...)`, `Signal::derive(...)`, `ViewFn::from(...)`, or `.into()`
at each call site. Keep explicit wrapper locals only when reused across more
than one prop or when type inference genuinely needs the help.

Plain Leptos widgets should compose adapter primitives directly. CSS widgets
should import CSS styled source templates, and Tailwind widgets should import
Tailwind styled source templates. Do not use a CSS styled component inside the
plain widget just for visual polish.
Leptos widget examples should import adapter/framework APIs through
`use ars_leptos::prelude::*;` as much as possible. Avoid direct `leptos::*` or
deep `ars_leptos::*` imports in examples when the item is intentionally
available from the adapter prelude.
