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
/// Props for the `Heading` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Override the auto-detected level. When `None`, uses the context level.
    pub level: Option<Level>,
}

impl Props {
    pub fn new() -> Self { Self::default() }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
    pub const fn level(mut self, level: Level) -> Self {
        self.level = Some(level);
        self
    }
    pub const fn auto_level(mut self) -> Self {
        self.level = None;
        self
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "heading"]
pub enum Part {
    Root,
}

pub struct Api {
    props: Props,
    ctx: HeadingContext,
}

impl Api {
    pub const fn new(props: Props, ctx: HeadingContext) -> Self {
        Self { props, ctx }
    }

    /// Returns the resolved heading level (1-6).
    pub const fn resolved_level(&self) -> Level {
        match self.props.level {
            Some(level) => level,
            None => self.ctx.level,
        }
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

impl ConnectApi for Api {
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

## 4. Heading Context and Level

```rust
/// Context that tracks the current heading level in the component tree.
/// Starts at level 1 by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadingContext {
    /// The current heading level (1-6, clamped).
    pub level: Level,
}

impl HeadingContext {
    pub const fn new() -> Self { Self { level: Level::One } }
    pub const fn from_level(level: Level) -> Self { Self { level } }
    pub const fn level(&self) -> Level { self.level }
    /// Increments the heading level by 1, capped at Level::Six.
    pub const fn incremented(&self) -> Self {
        Self { level: Level::from_u8(self.level.as_u8() + 1) }
    }
}

impl Default for HeadingContext {
    fn default() -> Self { Self::new() }
}

/// The `Heading` level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 | 1 => Level::One,
            2 => Level::Two,
            3 => Level::Three,
            4 => Level::Four,
            5 => Level::Five,
            _ => Level::Six,
        }
    }

    pub const fn as_u8(self) -> u8 { self as u8 }
}

impl Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_u8().to_string())
    }
}
```

## 5. HeadingLevelProvider

`HeadingLevelProvider` is a context-only component that wraps a subtree and provides the current heading level (1-6) to all descendant `Heading` components. It does not render any DOM element.

```rust
pub mod heading_level_provider {
    /// Props for the `HeadingLevelProvider` context wrapper.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Props {
        /// Starting heading level to provide to descendants.
        pub level: Level,
    }

    impl Default for Props {
        fn default() -> Self {
            Self { level: Level::One }
        }
    }

    impl Props {
        pub fn new() -> Self { Self::default() }
        pub const fn level(mut self, level: Level) -> Self {
            self.level = level;
            self
        }
    }

    pub const fn context_for(props: &Props) -> HeadingContext {
        HeadingContext::from_level(props.level)
    }
}
```

- **HeadingLevelProvider** wraps a subtree and injects a `HeadingContext` into the component tree.
- A nested `Section` increments the inherited heading level by 1, capped at 6.
- The `Heading` component reads from the nearest `HeadingContext` to determine which `<h1>`-`<h6>` element to render, without requiring manual `level` tracking.
- This ensures correct heading hierarchy in deeply nested layouts where components are composed independently and may not know their nesting depth.

## 6. Section

Section is a logical wrapper that increments the heading level context for all descendants.

```rust
pub mod section {
    /// Props for the `Section` component.
    /// No specific props — Section is a pure context modifier.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Props;

    impl Props {
        pub const fn new() -> Self { Self }
    }

    pub const fn context_for(parent: &HeadingContext) -> HeadingContext {
        parent.incremented()
    }
}
```

Rendering: Section provides `HeadingContext::incremented()` to its children. It renders no extra DOM element by default (fragment wrapper), but adapters MAY render a `<section>` element if desired.

Adapters MUST use framework context injection (`provide_context` in Leptos, `use_context_provider` in Dioxus) to provide the incremented level `HeadingContext` to descendants. Prop drilling is not sufficient — `Heading` components at arbitrary nesting depths must read the level via context.

> **Note:** `Section` is a convenience wrapper that provides `parent_level + 1`. `HeadingLevelProvider` allows setting an arbitrary starting level and is the underlying mechanism.

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

### 7.5 Heading Context Pattern Example

```text
HeadingLevelProvider (level=2)       // Sets context to h2
├── Heading "Page Section"           // Renders <h2>
├── Section                          // Increments to h3 (2+1)
│   ├── Heading "Subsection"         // Renders <h3>
│   └── Section                      // Increments to h4 (3+1)
│       └── Heading "Detail"         // Renders <h4>
└── Heading "Another Section"        // Renders <h2> (back at outer level)
```
