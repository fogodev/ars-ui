---
component: Heading
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  react-aria: Heading
---

# Heading

`Heading` provides automatic heading level management to prevent heading hierarchy violations (e.g., jumping from `<h1>` to `<h4>`). A `Section` component increments the heading level for its children.

## 1. API

### 1.1 Props

```rust
#[derive(HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Override the auto-detected level. When `None`, uses the context level.
    pub level: Option<Level>,
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "heading"]
pub enum Part {
    Root,
}

pub struct Api<'a> {
    ctx: &'a HeadingContext,
    props: &'a Props,
}

impl<'a> Api<'a> {
    /// Returns the resolved heading level (1-6).
    pub fn resolved_level(&self) -> Level {
        self.props.level.unwrap_or(self.ctx.level)
    }

    /// Returns the root attributes.
    ///
    /// When the adapter renders a native heading element (`<h1>`-`<h6>`),
    /// `role` and `aria-level` are redundant and should be omitted. Only set
    /// them when rendering as a non-semantic element (`<div>`, `<span>`).
    pub fn root_attrs(&self, is_native_heading_element: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, &self.props.id);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if !is_native_heading_element {
            attrs.set(HtmlAttr::Role, "heading");
            attrs.set(HtmlAttr::Aria(AriaAttr::Level), self.resolved_level().to_string());
        }
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(true),
        }
    }
}
```

The adapter renders `<h1>` through `<h6>` based on `resolved_level()`. When using a semantic heading element, the explicit `role="heading"` and `aria-level` are redundant and may be omitted.

## 2. Anatomy

```text
HeadingLevelProvider     (context only, no DOM)
├── Heading              (<h1>-<h6>)
├── Section              (context modifier, increments level)
│   ├── Heading          (level = parent + 1)
│   └── Section
│       └── Heading      (level = parent + 2, capped at 6)
└── ...
```

| Part | Element       | Key Attributes                                     |
| ---- | ------------- | -------------------------------------------------- |
| Root | `<h1>`-`<h6>` | `data-ars-scope="heading"`, `data-ars-part="root"` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property     | Value                                                      |
| ------------ | ---------------------------------------------------------- |
| Role         | `heading` (implicit on `<h1>`-`<h6>`, explicit on `<div>`) |
| `aria-level` | `1`-`6` (only on non-semantic elements)                    |

- Prevents heading hierarchy violations by auto-incrementing levels through nested Sections.
- When heading level exceeds 6, it clamps at `<h6>` (the maximum HTML heading level).
- Screen readers use heading levels for document outline navigation — correct hierarchy is essential.
- The `level` prop override is available as an escape hatch but should be used sparingly.

### 3.2 Bidirectional Text

When heading content may differ in direction from the page (e.g., an English section title inside an Arabic page), the adapter SHOULD set `dir="auto"` on the rendered heading element. This enables the browser's first-strong algorithm to correctly determine the heading's base text direction.

## 4. HeadingLevelProvider

```rust
/// Context that tracks the current heading level in the component tree.
/// Starts at level 1 by default.
#[derive(Clone, Debug)]
pub struct HeadingContext {
    /// The current heading level (1-6, clamped).
    pub level: Level,
}

impl HeadingContext {
    pub fn new() -> Self { Self { level: Level::One } }
    pub fn level(&self) -> Level { self.level }
    /// Increments the heading level by 1, capped at Level::Six.
    pub fn incremented(&self) -> Self {
        Self { level: Level::from_u8((self.level as u8) + 1) }
    }
}

/// The `Heading` level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Level {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

impl Level {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Level::One,
            2 => Level::Two,
            3 => Level::Three,
            4 => Level::Four,
            5 => Level::Five,
            _ => Level::Six,
        }
    }
}
```

## 5. Section

Section is a logical wrapper that increments the heading level context for all descendants.

```rust
pub mod section {
    /// Props for the `Section` component.
    /// No specific props — Section is a pure context modifier.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Props;
}
```

Rendering: Section provides `HeadingContext::incremented()` to its children. It renders no extra DOM element by default (fragment wrapper), but adapters MAY render a `<section>` element if desired.

Adapters MUST use framework context injection (`provide_context` in Leptos, `use_context_provider` in Dioxus) to provide the incremented level `HeadingContext` to descendants. Prop drilling is not sufficient — `Heading` components at arbitrary nesting depths must read the level via context.

> **Note:** `Section` is a convenience wrapper that creates a `HeadingLevelProvider` with `level = parent_level + 1`. `HeadingLevelProvider` allows setting an arbitrary starting level and is the underlying mechanism.

## 7. Library Parity

> Compared against: React Aria (`Heading`).

### 7.1 Props

| Feature        | ars-ui                 | React Aria | Notes          |
| -------------- | ---------------------- | ---------- | -------------- |
| Level override | `level: Option<Level>` | `level`    | Both libraries |

**Gaps:** None.

### 7.2 Anatomy

| Part | ars-ui                 | React Aria | Notes          |
| ---- | ---------------------- | ---------- | -------------- |
| Root | `Root` (`<h1>`-`<h6>`) | `Heading`  | Both libraries |

**Gaps:** None.

### 7.3 Features

| Feature                        | ars-ui                       | React Aria             |
| ------------------------------ | ---------------------------- | ---------------------- |
| Auto heading level via context | Yes (`HeadingLevelProvider`) | Yes (internal context) |
| Section nesting                | Yes (`Section`)              | Yes (internal)         |
| Level clamping (max 6)         | Yes                          | Yes                    |

**Gaps:** None.

### 7.4 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui explicitly exposes `HeadingLevelProvider` and `Section` as public components; React Aria handles level management internally.
- **Recommended additions:** None.

## 6. HeadingLevelProvider Context Pattern

`HeadingLevelProvider` is a context-only component that wraps a subtree and provides the current heading level (1-6) to all descendant `Heading` components:

- **HeadingLevelProvider** wraps a subtree and injects a `HeadingContext` into the component tree. It does not render any DOM element.
- Each nested `HeadingLevelProvider` increments the heading level by 1, capped at 6. This means a `HeadingLevelProvider` inside another `HeadingLevelProvider` provides `parent_level + 1`.
- The `Heading` component reads from the nearest `HeadingContext` to determine which `<h1>`-`<h6>` element to render, without requiring manual `level` tracking.
- This ensures correct heading hierarchy in deeply nested layouts where components are composed independently and may not know their nesting depth.

**Example:**

```text
HeadingLevelProvider (level=2)       // Sets context to h2
├── Heading "Page Section"           // Renders <h2>
├── HeadingLevelProvider             // Increments to h3 (2+1)
│   ├── Heading "Subsection"         // Renders <h3>
│   └── HeadingLevelProvider         // Increments to h4 (3+1)
│       └── Heading "Detail"         // Renders <h4>
└── Heading "Another Section"        // Renders <h2> (back at outer level)
```
