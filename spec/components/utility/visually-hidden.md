---
component: VisuallyHidden
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  radix-ui: VisuallyHidden
  react-aria: VisuallyHidden
---

# VisuallyHidden

`VisuallyHidden` renders content that is invisible on screen but fully accessible to screen readers. It is one of the most broadly used utilities in ars-ui, appearing wherever text must be provided for accessibility without being visually displayed.

## 1. API

### 1.1 Props

```rust
/// Props for the `VisuallyHidden` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// When true, renders the visually-hidden styles onto the single child element
    /// rather than wrapping it in a <span>. See AsChild section.
    pub as_child: bool,
    /// When `true`, the element becomes visible when it receives focus.
    /// Enables skip-link patterns where hidden navigation aids appear on focus.
    /// Default: `false`.
    pub is_focusable: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self { id: String::new(), as_child: false, is_focusable: false }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "visually-hidden"]
pub enum Part {
    Root,
}

/// The API for the `VisuallyHidden` component.
pub struct Api {
    props: Props,
}

impl Api {
    pub fn new(props: Props) -> Self {
        Self { props }
    }

    /// Props for the root <span> element (or child element when as_child=true).
    /// Applies the ars-visually-hidden class from the companion stylesheet.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Class, "ars-visually-hidden");
        if self.props.is_focusable {
            // Element is visible when focused, hidden otherwise.
            // CSS: [data-ars-visually-hidden-focusable]:not(:focus):not(:focus-within) { /* clip styles */ }
            p.set_bool(HtmlAttr::Data("ars-visually-hidden-focusable"), true);
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
VisuallyHidden
└── Root  <span>  data-ars-scope="visually-hidden" data-ars-part="root"
                  class="ars-visually-hidden"
```

| Part | Element  | Key Attributes                                                                            |
| ---- | -------- | ----------------------------------------------------------------------------------------- |
| Root | `<span>` | `data-ars-scope="visually-hidden"`, `data-ars-part="root"`, `class="ars-visually-hidden"` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- Content inside `VisuallyHidden` is announced by screen readers because it is in the DOM and not `display:none` or `visibility:hidden`.
- No ARIA role required — the component is a passive rendering utility.
- Content remains in the accessibility tree.

### 3.2 Focus Behavior

- By default, VisuallyHidden SHOULD NOT contain interactive elements. When `is_focusable` is `true`, the element becomes visible on focus — enabling skip-link and skip-navigation patterns. Example:

  ```html
  <VisuallyHidden is_focusable="true">
    <a href="#main-content">Skip to main content</a>
  </VisuallyHidden>
  ```

### 3.3 Position Note

`VisuallyHidden` uses `position: absolute` with `clip`/`clip-path`. The parent element does **not** need `position: relative` because the element is clipped to a 1×1px area and does not affect layout. The absolute positioning is solely to remove the element from normal flow; it is never visually positioned relative to a parent.

## 4. CSS Equivalent

For consumers who prefer a class-based approach, the equivalent CSS is:

```css
.ars-visually-hidden {
  position: absolute;
  border: 0;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  word-wrap: normal;
}

/* Focusable variant: visible when focused, hidden otherwise */
[data-ars-visually-hidden-focusable]:not(:focus):not(:focus-within) {
  position: absolute;
  border: 0;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  word-wrap: normal;
}
```

## 5. Usage Patterns

**Icon-only button label:**

```rust
// Leptos example
view! {
    <Button on_click=handle_delete>
        <TrashIcon />
        <VisuallyHidden>"Delete account"</VisuallyHidden>
    </Button>
}
```

**Screen reader status text:**

```rust
// Announce a status without visible text
view! {
    <VisuallyHidden>
        <span role="status">{ upload_status }</span>
    </VisuallyHidden>
}
```

**Hidden form field instructions:**

```rust
view! {
    <TextField>
        <text_field::Label>"Username"</text_field::Label>
        <text_field::Input />
        <VisuallyHidden id="username-hint">
            "Must be 3-20 characters, letters and numbers only"
        </VisuallyHidden>
    </TextField>
}
```

## 6. Library Parity

> Compared against: Radix UI (`VisuallyHidden`), React Aria (`VisuallyHidden`).

### 6.1 Props

| Feature      | ars-ui         | Radix UI  | React Aria    | Notes                                              |
| ------------ | -------------- | --------- | ------------- | -------------------------------------------------- |
| as_child     | `as_child`     | `asChild` | --            | Radix and ars-ui                                   |
| Focusable    | `is_focusable` | --        | `isFocusable` | React Aria and ars-ui support focusable skip links |
| Element type | --             | --        | `elementType` | RA allows `<div>`, `<span>`, etc.                  |

**Gaps:** None. React Aria's `elementType` is handled by adapter element choice.

### 6.2 Anatomy

| Part | ars-ui | Radix UI | React Aria       | Notes                      |
| ---- | ------ | -------- | ---------------- | -------------------------- |
| Root | `Root` | `Root`   | `VisuallyHidden` | All libraries; single-part |

**Gaps:** None.

### 6.3 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui combines Radix's `asChild` and React Aria's `isFocusable` into a single component. React Aria's `elementType` is an adapter concern in ars-ui.
- **Recommended additions:** None.
