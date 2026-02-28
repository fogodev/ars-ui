---
component: Grid
category: layout
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: [layout-shared-types]
related: [stack, center]
references: {}
---

# Grid

`Grid` is a stateless CSS grid layout primitive. It provides a declarative API for grid columns, gaps, and alignment. There is no state machine; there is no interactive behaviour.

## 1. API

### 1.1 Props

```rust
/// Props for `Grid`.
///
/// Types `Spacing`, `FlexAlign`, and `TokenResolver` are defined in
/// `layout-shared-types`.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the grid.
    pub id: String,
    /// Number of equal-width columns. Uses `repeat(N, minmax(0, 1fr))`.
    pub columns: Option<u32>,
    /// Minimum column width for auto-fill. Uses `repeat(auto-fill, minmax(N, 1fr))`.
    /// Mutually exclusive with `columns`.
    pub auto_columns: Option<Spacing>,
    /// Row gap.
    pub row_gap: Option<Spacing>,
    /// Column gap.
    pub column_gap: Option<Spacing>,
    /// Uniform gap (overrides `row_gap` and `column_gap`).
    pub gap: Option<Spacing>,
    /// Cross-axis alignment (`align-items`).
    pub align: Option<FlexAlign>,
    /// Whether grid items stretch to fill their cell.
    pub stretch: bool,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            id: String::new(),
            columns: Some(1),
            auto_columns: None,
            row_gap: None,
            column_gap: None,
            gap: None,
            align: None,
            stretch: false,
        }
    }
}

impl Props {
    /// Apply inline styles for this grid to the given attribute map.
    pub fn apply_styles(&self, attrs: &mut AttrMap, resolver: Option<&dyn TokenResolver>) {
        attrs.set_style(CssProperty::Display, "grid");
        if let Some(cols) = self.columns {
            attrs.set_style(
                CssProperty::GridTemplateColumns,
                format!("repeat({cols}, minmax(0, 1fr))"),
            );
        } else if let Some(ref min) = self.auto_columns {
            attrs.set_style(
                CssProperty::GridTemplateColumns,
                format!("repeat(auto-fill, minmax({}, 1fr))", min.to_css(resolver)),
            );
        }
        if let Some(ref g) = self.gap {
            attrs.set_style(CssProperty::Gap, g.to_css(resolver));
        } else {
            if let Some(ref g) = self.row_gap {
                attrs.set_style(CssProperty::RowGap, g.to_css(resolver));
            }
            if let Some(ref g) = self.column_gap {
                attrs.set_style(CssProperty::ColumnGap, g.to_css(resolver));
            }
        }
        if let Some(a) = self.align {
            attrs.set_style(CssProperty::AlignItems, a.css_value());
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "grid"]
pub enum Part {
    Root,
}

pub struct Api {
    props: Props,
    resolver: Option<Box<dyn TokenResolver>>,
}

impl Api {
    pub fn new(props: Props, resolver: Option<Box<dyn TokenResolver>>) -> Self {
        Self { props, resolver }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        self.props.apply_styles(&mut attrs, self.resolver.as_deref());
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
Grid
└── Root  <div>  data-ars-scope="grid" data-ars-part="root"
                 style="display:grid; grid-template-columns:...; gap:...; ..."
```

| Part | Element | Key Attributes                                    |
| ---- | ------- | ------------------------------------------------- |
| Root | `<div>` | Computed inline style from Props, CSS grid layout |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA roles or attributes added. `Grid` is a passive CSS layout container.
- Content accessibility is the consumer's responsibility.

## 4. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
