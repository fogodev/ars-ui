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

## Reactive Context

When reading provider signals during component construction, use an untracked
read if the value is not meant to establish a reactive dependency.

Reactive-context warnings in widgets count as user-visible regressions. A demo
that floods the browser console on mount is not shippable.

## Leptos Children

For Leptos slot components, prefer `TypedChildren<T>` over plain `Children`
when the slot has a typed child-root contract or participates in `add_attr` /
as-child style composition. Plain `Children` can compile while erasing the
typed surface that adapter consumers and tests rely on.

## Dynamic Attributes

Use the adapter's reactive attribute helper for dynamic attrs instead of
building stale one-time maps. Leptos components should use the local
`memo_to_reactive_attrs` pattern where it applies.

Use `UseMachineReturn::part_attrs` for Leptos machine-backed parts that only
need agnostic part attrs plus consumer `class` / `style` props. Keep local
merge code only when the part adds adapter-specific dynamic attrs, event
handlers, refs, or renderer-only behavior.

Use the shared `apply_part_attrs` helper as the final conversion step for
Leptos compound parts that do need local dynamic attrs before rendering. The
component may mutate the `AttrMap` for renderer-specific behavior, then hand it
to `apply_part_attrs` for consumer `class` / `style` merging and Leptos attr
conversion.

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
