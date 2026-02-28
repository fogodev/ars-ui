---
component: Separator
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  radix-ui: Separator
  react-aria: Separator
---

# Separator

A horizontal or vertical dividing line used to group and visually separate content.

## 1. API

### 1.1 Props

```rust
use ars_i18n::Orientation;

/// Props for the `Separator` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The orientation of the separator.
    pub orientation: Orientation,
    /// Whether the separator is purely decorative and hidden from the accessibility tree.
    pub decorative: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: Orientation::Horizontal,
            decorative: false,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "separator"]
pub enum Part {
    Root,
}

/// The API for the `Separator` component.
pub struct Api {
    orientation: Orientation,
    decorative: bool,
}

impl Api {
    /// Creates a new `Api` instance from the given props.
    pub fn new(props: Props) -> Self {
        Self {
            orientation: props.orientation,
            decorative: props.decorative,
        }
    }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);

        let orientation_str = match self.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        };
        p.set(HtmlAttr::Data("ars-orientation"), orientation_str);

        if self.decorative {
            // Decorative separators are hidden from the accessibility tree.
            p.set(HtmlAttr::Role, "presentation");
            p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        } else {
            p.set(HtmlAttr::Role, "separator");
            p.set(HtmlAttr::Aria(AriaAttr::Orientation), orientation_str);
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
Separator
└── Root  <hr> or <div>  data-ars-scope="separator" data-ars-part="root"
                         role="separator" | role="presentation"
```

| Part | Element          | Key Attributes                                                                   |
| ---- | ---------------- | -------------------------------------------------------------------------------- |
| Root | `<hr>` / `<div>` | `data-ars-scope="separator"`, `data-ars-part="root"`, `role`, `aria-orientation` |

### 2.1 Element Choice

The adapter renders `<hr>` for content separators (between paragraphs, sections) and `<div>` for menu, toolbar, or listbox separators where `<hr>` is semantically inappropriate. When `decorative` is true, either element is acceptable.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Value                                                  |
| ------------------ | ------------------------------------------------------ |
| Role               | `separator` (semantic) or `presentation` (decorative)  |
| `aria-orientation` | `"horizontal"` / `"vertical"` (omitted for decorative) |
| `aria-hidden`      | `"true"` when decorative                               |

- Semantic `<hr>` has implicit `role="separator"` in HTML. When using a `<div>`, `role="separator"` must be set explicitly.
- Decorative separators (e.g., in a dropdown menu between groups that are already labeled) should use `role="presentation"` with `aria-hidden="true"` to avoid screen reader noise.
- `aria-orientation` is optional for horizontal separators (the default) but recommended for vertical ones.

## 4. Internationalization

- Separators have no text content and require no localization.
- In RTL layouts, vertical separators remain visually unchanged. The content on either side of the separator reflows according to the document direction.

## 5. Library Parity

> Compared against: Radix UI (`Separator`), React Aria (`Separator`).

### 5.1 Props

| Feature      | ars-ui        | Radix UI      | React Aria    | Notes                                                    |
| ------------ | ------------- | ------------- | ------------- | -------------------------------------------------------- |
| Orientation  | `orientation` | `orientation` | `orientation` | All libraries                                            |
| Decorative   | `decorative`  | `decorative`  | --            | Radix and ars-ui; RA uses role="presentation" implicitly |
| Element type | --            | --            | `elementType` | RA allows overriding the HTML element                    |

**Gaps:** None. React Aria's `elementType` is handled by ars-ui's adapter element choice (section 2.1).

### 5.2 Anatomy

| Part | ars-ui | Radix UI | React Aria  | Notes                      |
| ---- | ------ | -------- | ----------- | -------------------------- |
| Root | `Root` | `Root`   | `Separator` | All libraries; single-part |

**Gaps:** None.

### 5.3 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria exposes `elementType` as a prop; ars-ui handles element choice at the adapter level (section 2.1).
- **Recommended additions:** None.
