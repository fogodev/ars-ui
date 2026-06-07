# Framework Rules

These rules prevent adapter implementations from compiling but drifting from
Leptos, Dioxus, or target behavior.

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

## Popup-Anchored Items

For popup-anchored item slots, use the established `mousedown`
`prevent_default` pattern when needed to prevent focus loss before the intended
selection/action event runs.

## Aggregate User-Visible Text

For multi-selection APIs and trigger/value text, aggregate selected item text
from semantic item data. Do not infer the accessible value solely from arbitrary
rendered children.
