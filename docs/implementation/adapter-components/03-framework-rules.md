# Framework Rules

These rules prevent adapter implementations from compiling but drifting from
Leptos, Dioxus, or target behavior.

## Adapter Semantic Boundary

Adapters are rendering layers, not duplicate component engines. They may own
framework-specific wiring: prop conversion, context reads, hook setup, DOM refs,
event extraction, attr conversion, children/slot rendering, and callback
dispatch.

Move renderer-independent logic into `crates/ars-components` or a shared
foundation crate before using it from adapters. A helper belongs outside the
adapter when it decides component state, derives the next event outcome,
interprets keyboard or pointer meaning, builds ARIA relationships, maps native
form values, merges disabled/readonly/invalid semantics, selects validation
errors, or computes ids/messages in a way both adapters need.

Classify private adapter helpers when adding or reviewing them:

- `renderer-glue`: needs Leptos/Dioxus attrs, views, events, hooks, refs, or
  DOM handles;
- `framework-context-merge`: reads adapter contexts and builds agnostic props;
- `component-semantics`: move to the agnostic component module;
- `foundation-semantics`: move to the relevant shared crate.

Duplicated helper names in the Leptos and Dioxus implementation of the same
component are a warning sign. Either move the logic to the agnostic layer or
add a short marker comment before the helper, for example
`// adapter-rendering-glue: needs framework event conversion`. Do not create
adapter-local extension traits for agnostic APIs such as `Api`, `Props`,
`State`, or `Event`; add the shared method to the agnostic API instead.

## Avoid Repeated Closure Clones

When a per-instance value is captured by multiple closures:

- use `StoredValue<T>` in Leptos;
- use `use_hook(|| CopyValue::new(value))` in Dioxus so the `CopyValue`
  allocation is stable for the component instance across rerenders.

Do this instead of cloning the same value into each handler. The adapter should
make ownership stable and explicit.

## Dioxus Hooks

Do not short-circuit hooks into `unwrap_or_else`, conditionals, loops, or other
closures that can violate Dioxus's Rules of Hooks. Initialize hooks at the
component top level in a stable order.

Bad:

```rust
let id = props.id.unwrap_or_else(|| use_stable_id("field"));
```

Good:

```rust
let generated_id = use_stable_id("field");
let id = props.id.unwrap_or(generated_id);
```

This applies even when the fallback hook looks harmless, deterministic, or only
runs when an optional prop is absent. Dioxus tracks hook calls by position; a
prop changing from `None` to `Some(_)`, or the reverse, must not change which
hooks run during render.

Before handoff, search the Dioxus adapter files changed by the task for hooks
hidden inside fallback closures:

```bash
rg 'unwrap_or_else\(\|\| use_|map_or_else\([^\n]*use_' <changed-dioxus-files>
```

Then manually review those same changed files for hooks inside conditionals,
loops, iterator adapters, nested closures, and early-return branches. Fix the
hook order first, then run the focused Dioxus checks.

## Dioxus Collection Identity

When rendering a stateful Dioxus component from a collection iterator, put the
stable key on the component invocation itself. Keying only an inner DOM node is
too late: Dioxus may preserve the wrong component instance across reorder,
insert, or remove operations before it reaches the keyed child.

Bad:

```rust,no_check
items.iter().map(|item| {
    let key = item.key.to_string();

    rsx! {
        ItemPanel {
            item: item.clone(),
        }
    }
})
```

Also bad when `ItemPanel` owns hooks or component-local state:

```rust,no_check
rsx! {
    ItemPanel {
        item: item.clone(),
        div { key: "{key}" }
    }
}
```

Good:

```rust,no_check
items.iter().map(|item| {
    let key = item.key.to_string();

    rsx! {
        ItemPanel {
            key: "{key}",
            item: item.clone(),
        }
    }
})
```

Add a browser-backed regression test when the component has per-item mounted
state, refs, focus restoration, drag/drop data, lazy panel state, or any other
behavior that must follow the semantic item key after a reorder.

## Dioxus Global Attributes

For Dioxus component props, `#[props(extends = GlobalAttributes)]` is the
consumer escape hatch for root HTML attrs. Do not duplicate root `class`,
`style`, `data-*`, `lang`, `tabindex`, or extra `aria-*` as explicit props when
`GlobalAttributes` already accepts the same call-site syntax.

Use explicit Dioxus props for semantic component inputs, non-root part attrs,
typed HTML vocabularies, or attrs with component-owned validation/precedence.
Otherwise forward the global attrs vector and merge it with agnostic root attrs
using the adapter merge helper. Tests should prove `class:` and `style:` still
work through global attrs rather than through bespoke props.

For Dioxus multi-part components, apply the same rule to each public compound
part. A stylable `Control`, `Indicator`, `Description`, or similar part should
extend `GlobalAttributes` and merge those attrs with its agnostic part attrs.
Do not add repeated explicit `control_class`, `indicator_style`, and similar
props to the root component when a compound part API would let consumers style
the part directly.

Use `UseMachineReturn::part_attrs` for Dioxus machine-backed parts that only
need agnostic part attrs plus consumer global attrs. Keep local merge code only
when the part adds adapter-specific dynamic attrs, event handlers, refs, or
renderer-only behavior.

Treat raw `Vec<Attribute>` props without `#[props(extends = GlobalAttributes)]`
as internal adapter plumbing, not consumer-facing Dioxus part API. Public
stylable Dioxus parts should extend `GlobalAttributes`; otherwise document the
part as private and keep the raw-attr helper out of the public module surface.

Do not recreate component-owned `AttrMap` keys or values in Dioxus adapters.
If a public part needs stable `data-ars-scope`, `data-ars-part`, ARIA,
disabled/readonly/selected state, id relationships, role, or tabindex attrs,
add the smallest `Api::*_attrs()` method or `Part` variant to the agnostic
component and read it from the machine. Dioxus may still add renderer-only
attrs such as event handlers, refs, `draggable`, mounted-node bookkeeping, or
DOM-measurement styles.

When a compound public part owns browser drag/drop, make that larger part the
drag source and expose enough mirrored state for styling the whole row. The
adapter may set renderer-owned attrs such as `draggable` and
`data-ars-dragging`, but cursor styling must account for every visible child:
the idle clickable cursor should be consistent across the shell, trigger, and
secondary affordances, and the dragging cursor must override descendant cursor
rules. CSS variants should place the dragging override after child cursor rules
or use an equal-or-stronger selector such as
`[data-ars-dragging], [data-ars-dragging] * { cursor: grabbing; }`.

When a parent Dioxus component auto-renders a required fallback part, do not
depend on a child component side effect to suppress that fallback on the same
render. On SSR, the parent output can be built before the child registration
updates are visible, producing duplicate structural nodes. Prefer an explicit
prop-level choice or inspect the child `Element`/`VNode` tree before rendering
the fallback. Keep the scan narrow to the specific public part and cover the
single-node guarantee in SSR tests.

Tailwind examples should consume those public parts or use Tailwind arbitrary
variants over `data-ars-*` state/anatomy. Raw `<style>` blocks or Rust string
CSS in Tailwind widget crates are acceptable only for a documented Tailwind
tooling limitation, not for ordinary component part styling.

This low-level primitive-composition rule applies to unstyled adapter examples
and adapter fixtures. CSS and Tailwind widget galleries have a different job:
they must consume the high-level styled component from `ars-*-components` so
the ready-made visual source template is the demo surface. Do not duplicate the
styled component's internal `Root` / `List` / part composition in those styled
widget crates.

If a behavior-critical node is intentionally private, Tailwind examples should
style it from the nearest public ancestor with arbitrary variants targeting its
stable `data-ars-part` attr. The adapter spec must show the selector shape and
must name the non-styling customization hook separately, such as a visual
close-glyph prop that does not transfer close semantics to the user.

## Reactive Context

When reading provider signals during component construction, use an untracked
read if the value is not meant to establish a reactive dependency.

For Dioxus effects or derived render helpers that intentionally subscribe to a
signal or memo only to rerun side effects, make the tracking read explicit with
a named unused binding (`let _revision = revision();`). Dioxus does not expose a
Leptos-style `.track()` method. Avoid bare `signal();` calls because they hide
that the read is only for dependency tracking, and avoid `drop(memo())` unless
the code is deliberately ending a borrow; named unused bindings also avoid
Clippy's `let_underscore_drop` warning for memo/read values with destructors. In
Leptos, prefer `.track()` for the same intent.

Reactive-context warnings in widgets count as user-visible regressions. A demo
that floods the browser console on mount is not shippable.

## Leptos Children

For Leptos slot components, prefer `TypedChildren<T>` over plain `Children`
when the slot has a typed child-root contract or participates in `add_attr` /
as-child style composition. Plain `Children` can compile while erasing the
typed surface that adapter consumers and tests rely on.

Use plain `Children` only when the slot is deliberately heterogeneous and the
component does not consume child values as part of its public contract. For
example, a compound root that only publishes context and then renders arbitrary
public parts/decorations may use `Option<Children>` if the real type-safety
lives in typed collection props and renderer callbacks. Document that exception
in the adapter spec next to the public API signature; otherwise default to
`TypedChildren<T>`.

## Leptos Typed Renderer Props

Inline typed closures inside `view!` props are fragile under `leptosfmt` when
the closure type contains generics or lifetimes. Prefer a named local renderer
value for public examples and tests:

```rust,no_check
let render_item = ItemRenderer::from(|item: ItemRenderItem<MyKey>| {
    view! { <items::ItemShell item=item /> }
});

view! {
    <items::Root items render_item=render_item>
        <items::List />
    </items::Root>
}
```

If a component needs public typed renderers, consider a small wrapper type that
implements `From<F>` for cloneable closures instead of exposing a raw closure
prop. This keeps call sites formatter-safe, makes prelude exports explicit,
and avoids teaching examples to rely on `Callback::new(...)` unless the local
API genuinely requires it.

## Dynamic Attributes

Use the adapter's reactive attribute helper for dynamic attrs instead of
building stale one-time maps. Leptos components should use the local
`memo_to_reactive_attrs` pattern where it applies.

Reactive Leptos attrs must still read their keys and values from the agnostic
`Api`. It is fine for the adapter to wrap a core `AttrMap` value in
`AttrValue::reactive(...)`, `reactive_optional(...)`, or
`reactive_bool(...)` so the DOM updates after machine changes; it is drift to
recompute the same semantic boolean or string from adapter copies of props,
tab metadata, field context, or local state. When a wrapper node has public
anatomy, promote that node to a core `Part` plus `Api::*_attrs()` instead of
building `data-ars-*` attrs locally.

Use `UseMachineReturn::part_attrs` for Leptos machine-backed parts that only
need agnostic part attrs plus consumer `class` / `style` props. Keep local
merge code only when the part adds adapter-specific dynamic attrs, event
handlers, refs, or renderer-only behavior.

Use the shared `apply_part_attrs` helper as the final conversion step for
Leptos compound parts that do need local dynamic attrs before rendering. The
component may mutate the `AttrMap` for renderer-specific behavior, then hand it
to `apply_part_attrs` for consumer `class` / `style` merging and Leptos attr
conversion.

Treat public Leptos components that accept `Vec<LeptosAttribute>` as suspect.
That type is the low-level spreadable result of adapter attr conversion; it is
appropriate for private renderer helpers that receive already-built attrs, not
for normal consumer-composed parts. A public Leptos part should compute attrs
from context and expose `class` / `style`, or stay private.

Use the `UseMachineReturn::attr_string_memo`,
`UseMachineReturn::attr_optional_string_memo`, and
`UseMachineReturn::attr_presence_memo` methods when a Leptos component needs
dynamic values from a machine-backed part. Do not add component-local
`root_attr_*_memo`, `input_attr_*_memo`, or `attr_*_memo` copies unless the
derivation includes component-specific renderer behavior beyond reading an
`AttrMap` key.

## Effects

Prefer `Effect::new` over `Effect::watch` in component bodies unless the local
pattern has a specific reason for `watch`.

## Event Dispatch

When using ephemeral API access inside event handlers, defer event dispatch
until after the ephemeral borrow ends. This avoids borrow and reentrancy bugs
that only appear under browser interaction.

## Target-Specific Code

Prefer platform abstractions before target-specific code. If a target-specific
branch is unavoidable:

- prove the blocker;
- keep the branch minimal;
- preserve the same public contract across supported targets;
- cover the target behavior with tests.

For browser-owned behavior, record the platform capability explicitly before
choosing a fallback. Use these buckets in specs and sketches:

- `TypedWebDom`: the adapter can use typed browser APIs, such as `web_sys`;
- `WebViewBridge`: the runtime has a browser engine but Rust must cross an
  eval or bridge boundary to reach the DOM;
- `ServerOrSsr`: no live DOM is available during render;
- `NoDomNative`: the target has no browser DOM semantics.

Native browser behavior such as constraint validation, focus restoration,
selection ranges, layout measurement, clipboard, drag data, and file inputs
must not be described as universally supported unless every target has an
equivalent capability. When the supported behavior differs by target, keep the
adapter API stable, document which bucket owns the exact native behavior, and
test each implemented bucket. Do not reimplement browser algorithms in adapter
Rust just to avoid a target gate unless the spec deliberately chooses that
portable algorithm.

## Popup-Anchored Items

For popup-anchored item slots, use the established `mousedown`
`prevent_default` pattern when needed to prevent focus loss before the intended
selection/action event runs.

## Aggregate User-Visible Text

For multi-selection APIs and trigger/value text, aggregate selected item text
from semantic item data. Do not infer the accessible value solely from arbitrary
rendered children.
