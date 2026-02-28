---
component: Stack
category: layout
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: [layout-shared-types]
related: [center, grid]
references: {}
---

# Stack

`Stack` is a stateless flex-layout primitive that arranges children in a row or column with consistent spacing. It uses CSS logical properties for RTL-aware spacing. There is no state machine; there is no interactive behaviour.

## 1. API

### 1.1 Props

```rust
/// Props for `Stack`.
///
/// Types `StackDirection`, `FlexAlign`, `FlexJustify`, `Spacing`, and
/// `TokenResolver` are defined in `layout-shared-types`.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// The ID of the stack.
    pub id: String,
    /// The flex direction. `RowLogical` maps to `Row` in LTR and
    /// `RowReverse` in RTL for automatic icon/label ordering.
    pub direction: StackDirection,
    /// Gap between children. Uses CSS `gap` property (direction-neutral).
    pub spacing: Option<Spacing>,
    /// Cross-axis alignment (`align-items`).
    pub align: FlexAlign,
    /// Main-axis distribution (`justify-content`).
    pub justify: FlexJustify,
    /// Whether the flex container wraps.
    pub wrap: bool,
    /// Whether to render visual dividers between children.
    pub divider: bool,
    /// Set `width: 100%`.
    pub full_width: bool,
    /// Set `height: 100%`.
    pub full_height: bool,
}

impl Props {
    /// Apply inline styles for this stack to the given attribute map.
    pub fn apply_styles(&self, attrs: &mut AttrMap, is_rtl: bool, resolver: Option<&dyn TokenResolver>) {
        let direction = self.direction.resolve(is_rtl);
        attrs.set_style(CssProperty::Display, "flex");
        attrs.set_style(CssProperty::FlexDirection, direction.css_value());
        attrs.set_style(CssProperty::AlignItems, self.align.css_value());
        attrs.set_style(CssProperty::JustifyContent, self.justify.css_value());
        if let Some(s) = &self.spacing {
            attrs.set_style(CssProperty::Gap, s.to_css(resolver));
        }
        if self.wrap {
            attrs.set_style(CssProperty::FlexWrap, "wrap");
        }
        if self.full_width {
            attrs.set_style(CssProperty::Width, "100%");
        }
        if self.full_height {
            attrs.set_style(CssProperty::Height, "100%");
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "stack"]
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
Stack
└── Root  <div>  data-ars-scope="stack" data-ars-part="root"
                 style="display:flex; flex-direction:...; gap:...; ..."
```

| Part | Element | Key Attributes                              |
| ---- | ------- | ------------------------------------------- |
| Root | `<div>` | Computed inline style from Props, `gap` CSS |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA roles or attributes added. `Stack` is a passive CSS layout container.
- Content accessibility is the consumer's responsibility.

### 3.2 RTL Considerations

`StackDirection::RowLogical` maps to `Row` in LTR and `RowReverse` in RTL, so an icon-before-label pattern automatically places the icon on the correct side without RTL-specific consumer code. Stack always uses the CSS `gap` property (not directional margin), which is neutral to text direction.

## 4. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
