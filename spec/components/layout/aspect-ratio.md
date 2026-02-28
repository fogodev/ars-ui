---
component: AspectRatio
category: layout
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  radix-ui: AspectRatio
---

# AspectRatio

`AspectRatio` is a stateless CSS layout primitive that forces a single child to maintain a given width-to-height ratio. It uses the padding-top intrinsic sizing technique. There is no state machine; there is no interactive behaviour.

## 1. API

### 1.1 Props

```rust
/// The props for the AspectRatio component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Width-to-height ratio. E.g. `16.0 / 9.0` for widescreen.
    /// Must be positive and finite.
    pub ratio: f64,
}

impl Props {
    /// CSS `padding-top` percentage that enforces the ratio.
    /// `padding-top: X%` is relative to element width, so
    /// `X = (1 / ratio) * 100`.
    pub fn padding_top_percent(&self) -> f64 {
        (1.0 / self.ratio) * 100.0
    }
}

impl Default for Props {
    fn default() -> Self {
        Props { id: String::new(), ratio: 1.0 }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "aspect-ratio"]
pub enum Part {
    Root,
}

pub struct Api {
    props: Props,
}

impl Api {
    pub fn new(props: Props) -> Self { Self { props } }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let padding = self.props.padding_top_percent();
        attrs.set_style(CssProperty::Position, "relative");
        attrs.set_style(CssProperty::Width, "100%");
        attrs.set_style(CssProperty::PaddingTop, format!("{:.4}%", padding));
        attrs
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
AspectRatio
└── Root  <div>  data-ars-scope="aspect-ratio" data-ars-part="root"
                 style="position:relative; width:100%; padding-top:N%"
```

| Part | Element | Key Attributes                                  |
| ---- | ------- | ----------------------------------------------- |
| Root | `<div>` | Inline `padding-top` computed from `ratio` prop |

The child element should be styled with `position:absolute; inset:0; width:100%; height:100%` to fill the aspect-ratio container. This is the consumer's responsibility.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA roles or attributes added. `AspectRatio` is a passive layout wrapper.
- The consumer is responsible for all accessibility concerns of the content placed inside (e.g., `alt` text for images, captions for video).

## 4. Library Parity

> Compared against: Radix UI (`AspectRatio`).

### 4.1 Props

| Feature | ars-ui                     | Radix UI                    | Notes |
| ------- | -------------------------- | --------------------------- | ----- |
| Ratio   | `ratio` (f64, default 1.0) | `ratio` (number, default 1) | Same  |

**Gaps:** None.

### 4.2 Anatomy

| Part | ars-ui | Radix UI | Notes |
| ---- | ------ | -------- | ----- |
| Root | `Root` | `Root`   | --    |

**Gaps:** None.

### 4.3 Events

No events in either library.

**Gaps:** None.

### 4.4 Features

| Feature               | ars-ui | Radix UI |
| --------------------- | ------ | -------- |
| Aspect ratio via CSS  | Yes    | Yes      |
| Padding-top technique | Yes    | Yes      |

**Gaps:** None.

### 4.5 Summary

- **Overall:** Full parity.
- **Divergences:** None. Both are minimal wrappers.
- **Recommended additions:** None.
