# Adapter API And Wiring

Adapter crates render framework views and connect events. Component behavior,
semantic state, ARIA, and `data-ars-*` anatomy belong in the framework-agnostic
component API.

## Required Files

Add or update:

- `crates/ars-leptos/src/<category>/<component>/`
- `crates/ars-dioxus/src/<category>/<component>/`
- category `mod.rs` files;
- `crates/ars-leptos/src/lib.rs` and `crates/ars-dioxus/src/lib.rs` when a new
  category module is introduced;
- adapter `Cargo.toml` feature wiring when the component needs an
  `ars-components` feature.

Use directory-backed modules with `mod.rs` when a component module owns child
modules.

## Prelude Exports

For user-facing components, re-export both:

- the component module; and
- the root component entry point.

Example:

```rust
pub use crate::data_display::{grid_list, grid_list::GridList};
```

Also re-export configuration types that consumers pass into props. If the
framework-agnostic name is too generic, use stable aliases such as:

```rust
pub use ars_collections::selection::{
    Behavior as SelectionBehavior,
    Mode as SelectionMode,
    Set as SelectionSet,
};
```

Do not put machine internals, slot output internals, adapter hooks, or
component-author-only helpers in the end-user prelude.

Both adapter preludes must stay symmetric.

## Public API Fidelity

Before implementing any public type, function, prop, slot, or event handler:

1. read the corresponding spec example;
2. implement the exact public shape unless it cannot compile;
3. cross-check after implementation;
4. update the spec in the same PR if the spec is wrong or incomplete.

Do not invent equivalent-looking APIs. Spec drift invalidates downstream code.

## Attribute Ownership

Render semantic and anatomy attributes from the agnostic API only:

- roles;
- ARIA attributes;
- `data-ars-*` flags;
- ids and relationships;
- disabled, readonly, invalid, selected, focused, active, loading, and hidden
  state attrs.

Adapter code may add framework-specific event wiring, consumer styling
forwarding, and view composition. It must not fork the component's semantic
contract.

Render slots that need item state must receive it from the agnostic API. Do not
make Leptos and Dioxus render callbacks recompute selected, disabled, hovered,
pressed, dragging, drop-target, section, or layout-preview state from local
adapter data. If a renderer needs that state, add an agnostic render-state
helper first, then pass that state through both adapters.

Browser-only APIs still belong to adapters, but their inputs must come from the
agnostic API. For example, a drag-preview element id is adapter-owned DOM
configuration; the dragged key set, drop target, and preview order are
core-owned state.

## Consumer Styling

Leptos components must forward consumer styling through a `class` prop where
the adapter convention expects one.

Dioxus props must extend `GlobalAttributes` when the component exposes global
HTML attributes to consumers.

Avoid swallowing consumer classes or replacing them with fixed demo styling.

## Prop Ergonomics

Annotate `String` and `Option<String>` props with `#[prop(into)]` when the
framework supports it and the local adapter convention uses it.

Keep semantic data separate from rendered views. Rich framework views are useful
for customization, but components still need semantic sources for accessible
names, ids, announcements, form serialization, and state-machine wiring.
