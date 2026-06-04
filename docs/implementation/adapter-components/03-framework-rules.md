# Framework Rules

These rules prevent adapter implementations from compiling but drifting from
Leptos, Dioxus, or target behavior.

## Avoid Repeated Closure Clones

When a per-instance value is captured by multiple closures:

- use `StoredValue<T>` in Leptos;
- use `CopyValue<T>` in Dioxus.

Do this instead of cloning the same value into each handler. The adapter should
make ownership stable and explicit.

## Dioxus Hooks

Do not short-circuit hooks into `unwrap_or_else`, conditionals, loops, or other
closures that can violate Dioxus's Rules of Hooks. Initialize hooks at the
component top level in a stable order.

## Reactive Context

When reading provider signals during component construction, use an untracked
read if the value is not meant to establish a reactive dependency.

Reactive-context warnings in widgets count as user-visible regressions. A demo
that floods the browser console on mount is not shippable.

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
