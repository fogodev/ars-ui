---
component: AsChild
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: asChild
    radix-ui: Slot
---

# AsChild

The `as_child` pattern allows any ars-ui component to render its DOM props onto a consumer-provided child element instead of its own default element. This is the Rust equivalent of Radix UI's `asChild` prop or React Aria's render prop pattern.

Use cases:

- Render a `<Link>` (router-level) component as a [`Button`](button.md), preserving button semantics and button state machine props.
- Render a `<Trigger>` as a custom styled element without wrapping divs.
- Compose ars-ui behavior onto any element without extra DOM nodes.

## 1. API

### 1.1 Props

```rust
/// Include in any component's Props struct that supports the as_child pattern.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Props {
    /// When true, render the component's props onto the single child element
    /// rather than the default element.
    pub as_child: bool,
}
```

The struct field is `pub` so adapter destructure patterns keep working, but the documented
construction path is the inherent builder: `Props::new()` returns the default and
`Props::new().as_child(true)` toggles the flag. `Eq` is implied by `bool: Eq` and is added so
the derived bound matches the underlying field type.

### 1.2 Connect / API

`AsChild` is a pattern — it has no `Part` enum, no `ConnectApi`, and no `AttrMap` output of its own. It defines the `AsChildMerge` trait used by other components when `as_child=true`.

```rust
use ars_core::AttrMap;

/// Merges one set of AttrMap onto another, combining attributes and styles.
/// Event handler composition is an adapter concern (see below).
pub trait AsChildMerge {
    /// Merge `self` (component props) onto `other` (child element props).
    /// - Static attributes: `self` takes precedence over `other` (component attrs win).
    /// - Space-separated attrs (class, aria-labelledby, etc.): merged automatically by `set()`.
    /// - Styles: `self` takes precedence (component styles win).
    fn merge_onto(self, other: AttrMap) -> AttrMap;
}

impl AsChildMerge for AttrMap {
    fn merge_onto(self, other: AttrMap) -> AttrMap {
        let mut result = other;
        // Component attrs take precedence. Space-separated token lists (class,
        // aria-labelledby, etc.) are automatically appended by `merge()` via `set()`.
        result.merge(self);
        // Event handler composition is an adapter concern. The adapter MUST call
        // both the component's typed handler methods (e.g., `api.on_root_click()`)
        // and the child element's existing handlers.
        result
    }
}
```

> **Note:** If the component handler calls `prevent_default()`, the child handler still runs unconditionally. Adapters MAY check `event.default_prevented()` between handler invocations if short-circuit behavior is desired.

## 2. Anatomy

When `as_child=true`, no wrapper element is added to the DOM. The child element itself receives all the component's data attributes (`data-ars-scope`, `data-ars-part`, ARIA attributes, event handlers). The child effectively _becomes_ the component's root.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- When using `as_child` to replace a `<button>` with a `<a href>` (link), the component's ARIA attributes (`role="button"`, keyboard handlers) are merged onto the `<a>`. Consumers are responsible for ensuring the result is semantically correct.
- `as_child` does not add or remove ARIA attributes beyond what the component's `connect()` API normally produces.
- **Development Warning:** When `as_child` merges props with `role="button"` onto an `<a>` element, the resulting element must handle Space key activation (links only respond to Enter by default). Adapters should emit a `cfg(debug_assertions)` warning when detecting this semantic mismatch.

### 3.2 ARIA Attribute Merge Rules

When merging component attributes onto a child element via `as_child`, ARIA attributes follow these rules:

1. **`role` conflicts:** Component role takes precedence. When the child element already has a different `role`, emit a development warning with message `"as_child: overriding child role '{child_role}' with component role '{component_role}'"`.

    The warning compiles in under `cfg(any(debug_assertions, feature = "debug"))` so it auto-fires in any dev build without requiring an explicit feature flag. Emission routes:
    - `feature = "debug"` enabled → `log::warn!` (structured logging, works on native and wasm whenever the consumer has wired a `log` subscriber). This is the standard diagnostic-build path used elsewhere in the workspace.
    - `debug_assertions` only, with `feature = "std"` → `eprintln!` on native targets, mirroring the stdout branch of `leptos::logging::console_debug_warn`.
    - On wasm dev builds without `feature = "debug"`, `ars-components` itself stays silent (it cannot pull `web_sys`). Framework adapters (`ars-leptos`, `ars-dioxus`) are responsible for re-emitting the warning to the browser console, the same way Leptos surfaces its own internal dev warnings via `web_sys::console::warn_1`.

2. **`aria-describedby` / `aria-labelledby`:** Concatenate values (space-separated) with deduplication rather than overwriting. Both sets of IDs end up in the merged value. Token order is unspecified — assistive technology treats these attributes as unordered ID lists, and the merge is implemented via `AttrMap::merge` (see §1.2), which appends component tokens after existing child tokens.

3. **All other ARIA attributes** (`aria-expanded`, `aria-selected`, `aria-controls`, etc.): Component value takes precedence (standard merge rule).

## 4. Framework Adapter Examples

**Leptos:**

```rust
// A Button component that supports as_child.
// When as_child=true, the component does not render a <button>;
// instead, it clones its single child and spreads button_props onto it.

#[component]
pub fn Button(
    props: Props,
    children: Children,
) -> impl IntoView {
    let api = use_machine::<button::Machine>(props.clone());
    let button_props = api.root_attrs();

    if props.as_child {
        // Render children with merged props via a slot helper.
        view! {
            <AsChildSlot props=button_props>
                {children()}
            </AsChildSlot>
        }
    } else {
        view! {
            <button {..button_props.into_leptos()}>
                {children()}
            </button>
        }
    }
}
```

**Dioxus:**

```rust
// Dioxus pattern using rsx! macro with conditional rendering.

fn Button(props: Props) -> Element {
    let api = use_machine::<button::Machine>(&props);
    let button_props = api.root_attrs();

    if props.as_child {
        rsx! {
            AsChildRenderer {
                props: button_props,
                {props.children}
            }
        }
    } else {
        rsx! {
            button { ..button_props.into_dioxus(), {props.children} }
        }
    }
}
```

## 5. Constraints

- The child element must be exactly one element (not a fragment, not multiple siblings).
- The child must be a DOM element that can accept the merged attributes. For example, merging button props onto an `<input>` is technically valid but semantically wrong.
- `as_child` composes event handlers (both fire). If the child element has its own `onClick`, both the component's click handler and the child's click handler will run.
- Framework adapters must handle the case where the child is a custom component (not a native element). In this case, the custom component must accept and spread AttrMap (or equivalent framework-specific spread props).

## 6. Library Parity

> Compared against: Ark UI (`asChild`), Radix UI (`Slot`).

### 6.1 Props

| Feature       | ars-ui           | Ark UI          | Radix UI        | Notes                                                     |
| ------------- | ---------------- | --------------- | --------------- | --------------------------------------------------------- |
| as_child flag | `as_child: bool` | `asChild: bool` | `asChild: bool` | All libraries                                             |
| Slottable     | --               | --              | `Slottable`     | Radix has a Slottable sub-component for multi-child slots |

**Gaps:** None. Radix's `Slottable` marks which children receive slot props in multi-child scenarios; ars-ui's single-child constraint makes this unnecessary.

### 6.2 Features

| Feature                    | ars-ui           | Ark UI | Radix UI |
| -------------------------- | ---------------- | ------ | -------- |
| Prop merging               | Yes              | Yes    | Yes      |
| Event handler composition  | Yes              | Yes    | Yes      |
| ARIA attribute merge rules | Yes (documented) | Yes    | Yes      |
| Single child constraint    | Yes              | Yes    | Yes      |

**Gaps:** None.

### 6.3 Summary

- **Overall:** Full parity.
- **Divergences:** Radix provides a `Slottable` sub-component for marking the slottable region when multiple children exist; ars-ui enforces a single-child constraint instead.
- **Recommended additions:** None.
