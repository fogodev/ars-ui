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

## 4. Global Pointer Tracking

FocusRing depends on a global module-level pointer tracking effect maintained by `ars-dom`:

```rust
// ars-dom/src/focus_visible.rs

use std::cell::Cell;

/// Describes why an element received focus. Replaces the previous boolean
/// `had_pointer_interaction` flag with a richer enum that distinguishes
/// keyboard, pointer, and programmatic focus sources.
#[derive(Clone, Debug, PartialEq)]
pub enum FocusCause {
    /// Focus was triggered by keyboard navigation (Tab, Shift+Tab, arrow keys).
    Keyboard,
    /// Focus was triggered by pointer interaction (mouse click, touch).
    Pointer,
    /// Focus was moved programmatically (e.g., dialog open, focus trap).
    Programmatic,
}

thread_local! {
    /// Tracks the cause of the most recent focus event.
    static LAST_FOCUS_CAUSE: Cell<Option<FocusCause>> = Cell::new(None);
}

/// Call once at application startup to install the global listeners.
/// (Called automatically by the ars-leptos / ars-dioxus mount function.)
///
/// **Initialization:** `install_focus_visible_tracker()` from `ars-dom` must
/// be called once at app startup. This is automatically handled by the
/// `ars-leptos` and `ars-dioxus` mount functions. If using a custom setup,
/// call it manually before any FocusRing-enabled components mount.
pub fn install_focus_visible_tracker() {
    // Install `pointerdown` listener on `window` that sets LAST_FOCUS_CAUSE = Some(Pointer).
    // Install `keydown` listener on `window` (for Tab, Shift+Tab, arrow keys)
    //   that sets LAST_FOCUS_CAUSE = Some(Keyboard).
    // Programmatic focus is detected when neither pointer nor keyboard preceded
    //   the focus event — the adapter sets LAST_FOCUS_CAUSE = Some(Programmatic)
    //   when calling element.focus() directly.
}
```

**Cleanup and idempotency:** The function MUST be idempotent — calling it multiple times must not accumulate duplicate listeners. Use a module-level guard:

```rust
thread_local! {
    static TRACKER_INSTALLED: Cell<bool> = Cell::new(false);
}

pub fn install_focus_visible_tracker() {
    TRACKER_INSTALLED.with(|installed| {
        if installed.get() { return; }
        installed.set(true);
        // ... attach listeners ...
    });
}
```

For Dioxus Desktop hot-reload scenarios, the guard prevents listener accumulation across webview recreations. If full cleanup is needed, the function MAY return a cleanup handle that removes the listeners and resets the guard.

```rust
/// Returns the cause of the most recent focus event.
pub fn last_focus_cause() -> Option<FocusCause> {
    LAST_FOCUS_CAUSE.with(|v| v.get())
}

/// Whether focus should show a visible focus ring.
/// Only keyboard-initiated focus triggers the focus ring.
/// When `is_text_input` is true, `data-ars-focus-visible` is set whenever the
/// element is focused, regardless of the `FocusCause`. This matches the
/// convention that text inputs always display a visible focus indicator.
pub fn is_focus_visible() -> bool {
    matches!(last_focus_cause(), Some(FocusCause::Keyboard))
}

/// Legacy compatibility wrapper.
pub fn had_pointer_interaction() -> bool {
    matches!(last_focus_cause(), Some(FocusCause::Pointer))
}
```

Individual component focus handlers call `last_focus_cause()` to determine the focus source:

```rust
// Inside any component's focus handler (now a typed method on the Api struct):
// The adapter calls last_focus_cause() and sends the appropriate event.
// fn on_focus(&self) {
//     let cause = last_focus_cause().unwrap_or(FocusCause::Programmatic);
//     let is_keyboard = matches!(cause, FocusCause::Keyboard);
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
