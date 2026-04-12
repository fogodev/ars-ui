---
component: FocusRing
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: FocusRing
---

# FocusRing

`FocusRing` is a lightweight utility that tracks whether focus was initiated by keyboard or pointer and exposes a `data-ars-focus-visible` attribute for CSS styling. It is the mechanism behind consistent, accessible focus indicators throughout ars-ui.

## 1. API

### 1.1 Props

```rust
/// Props for the `FocusRing` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Track focus-within rather than direct focus.
    pub within: bool,
    /// Optional CSS class to apply when focused by any means.
    pub focus_class: Option<String>,
    /// Optional CSS class to apply only when focused by keyboard.
    pub focus_visible_class: Option<String>,
    /// When true, the focus ring is shown even on pointer-initiated focus.
    /// Text inputs conventionally show focus indicators regardless of input
    /// method, since users need to know where they are typing.
    /// Default: false.
    pub is_text_input: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            within: false,
            focus_class: None,
            focus_visible_class: None,
            is_text_input: false,
        }
    }
}
```

### 1.2 Connect / API

```rust
/// The context for the `FocusRing` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    /// When true, `focus_visible` is set when any descendant is focused (focus-within mode).
    pub within: bool,
    /// Whether focus-visible is active.
    pub focus_visible: bool,
}

#[derive(ComponentPart)]
#[scope = "focus-ring"]
pub enum Part {
    Root,
}

/// The API for the `FocusRing` component.
pub struct Api {
    ctx: Context,
    props: Props,
}

impl Api {
    pub fn new(ctx: Context, props: Props) -> Self {
        Self { ctx, props }
    }

    /// Attributes applied to the element that should show the focus ring.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        p
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
FocusRing
└── Root  (any element)  data-ars-scope="focus-ring" data-ars-part="root"
                         data-ars-focus-visible (present when keyboard focused)
```

| Part | Element       | Key Attributes                                                                  |
| ---- | ------------- | ------------------------------------------------------------------------------- |
| Root | (any element) | `data-ars-scope="focus-ring"`, `data-ars-part="root"`, `data-ars-focus-visible` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA role — FocusRing is a styling utility, not a semantic component.
- The CSS `:focus-visible` pseudo-class achieves the same goal natively in modern browsers. `data-ars-focus-visible` is provided as a more compatible and controllable alternative.
- FocusRing must NOT suppress focus rings entirely. Only the visual style is modified; the underlying `:focus` state remains active for accessibility tools.
- Components that implement their own focus handling (`Button`, `Toggle`, `Input`) should all emit `data-ars-focus-visible` rather than using `FocusRing` directly.
- Focus indicators MUST meet WCAG 2.4.11 (Focus Appearance) requirements: the focus indicator area must be at least as large as a 2px-thick perimeter of the focused element, and the contrast ratio between focused and unfocused states must be at least 3:1. See foundation/03-accessibility.md for detailed guidance.

> **CSS Selector Performance Note:** The `:has()` pseudo-class (relative selector) has significant performance cost in older Chrome versions (< 105) and can cause style recalculation bottlenecks when used in high-frequency selectors. Prefer attribute selectors like `[data-ars-focus-visible]` over patterns like `:has(:focus-visible)` for focus ring styling. Attribute selectors are O(1) in selector matching, while `:has()` requires ancestor/descendant traversal. The `data-ars-*` attribute system is specifically designed to avoid the need for `:has()` in component styling.

## 4. Shared Modality Tracking

FocusRing no longer depends on a process-global singleton. It consumes the same provider-scoped modality event stream used by `ars_core::ModalityContext`, typically via `ars-dom::ModalityManager`:

```rust
// ars-dom/src/modality.rs

use std::rc::Rc;
use ars_a11y::FocusRing;
use ars_core::{KeyboardKey, KeyModifiers, ModalityContext, PointerType};

pub struct ModalityManager {
    modality: Arc<dyn ModalityContext>,
    focus_ring: FocusRing,
}

impl ModalityManager {
    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        self.modality.on_key_down(key, modifiers);
        self.focus_ring.on_key_down(key, modifiers);
    }

    pub fn on_pointer_down(&self, pointer_type: PointerType) {
        self.modality.on_pointer_down(pointer_type);
        self.focus_ring.on_pointer_down();
    }

    pub fn on_virtual_input(&self) {
        self.modality.on_virtual_input();
        self.focus_ring.on_virtual_input();
    }
}
```

Browser listener installation is also owned by `ars-dom`, not by `FocusRing` itself. The web implementation exposes ref-counted `ensure_listeners()` / `remove_listeners()` methods on `ModalityManager` so adapters can install document listeners without creating duplicate registrations.

Individual component focus handlers consult the shared modality context to determine the focus source:

```rust
// Inside any component's focus handler:
// fn on_focus(&self) {
//     let last_pointer_type = config.modality.last_pointer_type();
//     let is_keyboard = matches!(last_pointer_type, Some(PointerType::Keyboard));
//     (self.send)(Event::Focus { is_keyboard });
// }
```

## 5. CSS Usage

```css
/* Show visible focus ring only for keyboard navigation */
[data-ars-focus-visible]:focus,
[data-ars-focus-visible]:focus-within {
    outline: 2px solid var(--ars-color-focus);
    outline-offset: 2px;
}

/* Suppress focus ring for pointer navigation */
:focus:not([data-ars-focus-visible]) {
    outline: none;
}
```

## 6. Library Parity

> Compared against: React Aria (`FocusRing`).

### 6.1 Props

| Feature             | ars-ui                | React Aria       | Notes                                        |
| ------------------- | --------------------- | ---------------- | -------------------------------------------- |
| Within              | `within`              | `within`         | Both libraries support focus-within tracking |
| Focus class         | `focus_class`         | --               | ars-ui addition                              |
| Focus visible class | `focus_visible_class` | `focusRingClass` | Similar concept                              |
| Auto-focus tracking | `auto_focus`          | `autoFocus`      | Both libraries                               |

**Gaps:** None.

### 6.2 Anatomy

| Part | ars-ui | React Aria | Notes                     |
| ---- | ------ | ---------- | ------------------------- |
| Root | `Root` | (wrapper)  | Both wrap a child element |

**Gaps:** None.

### 6.3 Features

| Feature                            | ars-ui                         | React Aria         |
| ---------------------------------- | ------------------------------ | ------------------ |
| Keyboard vs pointer discrimination | Yes                            | Yes                |
| Focus-within mode                  | Yes                            | Yes                |
| Data attribute output              | Yes (`data-ars-focus-visible`) | Yes (render props) |
| CSS class output                   | Yes                            | Yes                |

**Gaps:** None.

### 6.4 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria's FocusRing uses render props (`isFocused`, `isFocusVisible`); ars-ui uses data attributes and optional CSS classes.
- **Recommended additions:** None.
