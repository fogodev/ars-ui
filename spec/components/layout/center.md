---
component: Center
category: layout
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: [layout-shared-types]
related: [stack, grid]
references: {}
---

# Center

`Center` is a stateless CSS layout primitive that centers content horizontally, vertically, or both. It uses CSS logical properties (`max-inline-size`, `margin-inline`) for RTL-neutral behavior. There is no state machine; there is no interactive behaviour.

## 1. API

### 1.1 Props

```rust
/// Props for `Center`.
///
/// Types `Spacing`, `TextAlign`, and `TokenResolver` are defined in
/// `layout-shared-types`.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// The ID of the center.
    pub id: String,
    /// Maximum width constraint (`max-inline-size`).
    pub max_width: Option<Spacing>,
    /// Center horizontally via `margin-inline: auto`.
    pub horizontal: bool,
    /// Center vertically via flex centering.
    pub vertical: bool,
    /// Text alignment within the container.
    pub text_align: Option<TextAlign>,
}

impl Props {
    /// Apply inline styles for this center to the given attribute map.
    pub fn apply_styles(&self, attrs: &mut AttrMap, is_rtl: bool, resolver: Option<&dyn TokenResolver>) {
        if let Some(ref s) = self.max_width {
            attrs.set_style(CssProperty::MaxInlineSize, s.to_css(resolver));
        }
        if self.horizontal {
            attrs.set_style(CssProperty::MarginInline, "auto");
        }
        if self.vertical {
            attrs.set_style(CssProperty::Display, "flex");
            attrs.set_style(CssProperty::AlignItems, "center");
            attrs.set_style(CssProperty::JustifyContent, "center");
        }
        if let Some(a) = self.text_align {
            attrs.set_style(CssProperty::TextAlign, a.css_value(is_rtl));
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "center"]
pub enum Part {
    Root,
}

pub struct Api {
    props: Props,
    is_rtl: bool,
    resolver: Option<Box<dyn TokenResolver>>,
}

impl Api {
    pub fn new(props: Props, is_rtl: bool, resolver: Option<Box<dyn TokenResolver>>) -> Self {
        Self { props, is_rtl, resolver }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        self.props.apply_styles(&mut attrs, self.is_rtl, self.resolver.as_deref());
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
Center
└── Root  <div>  data-ars-scope="center" data-ars-part="root"
                 style="max-inline-size:...; margin-inline:auto; ..."
```

| Part | Element | Key Attributes                                           |
| ---- | ------- | -------------------------------------------------------- |
| Root | `<div>` | Computed inline style from Props, CSS logical properties |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA roles or attributes added. `Center` is a passive CSS layout container.
- Content accessibility is the consumer's responsibility.

## 4. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
