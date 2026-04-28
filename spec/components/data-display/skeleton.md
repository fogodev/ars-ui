---
component: Skeleton
category: data-display
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references: {}
---

# Skeleton

A loading placeholder component that renders animated shapes to indicate content
is being loaded. Skeleton is a pure display component with no interactive states
or state machine.

## 1. API

### 1.1 Props

```rust
/// Props for the Skeleton component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Number of skeleton item placeholders to render.
    pub count: NonZero<u32>,
    /// Animation variant.
    pub variant: Variant,
    /// Shape variant for repeated item placeholders.
    pub shape: Shape,
    /// Height of each skeleton item in CSS units.
    pub line_height: String,
    /// Gap between skeleton items in CSS units.
    pub gap: String,
    /// Size of the optional leading circle in CSS units.
    pub leading_circle_size: Option<String>,
}

/// Animation variant for skeleton loading placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Opacity fades in and out.
    Pulse,
    /// Left-to-right sweep highlight.
    Wave,
    /// Diagonal gradient shimmer.
    Shimmer,
    /// Highlight shine animation.
    Shine,
    /// Static placeholder without animation.
    None,
}

impl Variant {
    /// Returns the `data-ars-variant` value for this animation variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str { /* ... */ }

    /// Returns whether this variant has an animation by default.
    #[must_use]
    pub const fn is_animated(self) -> bool { !matches!(self, Self::None) }
}

/// Shape variant for skeleton item placeholders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// Text-line placeholder.
    Text,
    /// Circular placeholder.
    Circle,
    /// Rectangular placeholder.
    Rect,
}

impl Shape {
    /// Returns the `data-ars-shape` value for this shape.
    #[must_use]
    pub const fn as_str(self) -> &'static str { /* ... */ }
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            count: NonZero::<u32>::MIN,
            variant: Variant::Pulse,
            shape: Shape::Text,
            line_height: "1rem".into(),
            gap: "0.5rem".into(),
            leading_circle_size: None,
        }
    }
}

impl Props {
    /// Returns fresh skeleton props with the documented defaults.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the number of skeleton item placeholders to render.
    #[must_use]
    pub const fn count(mut self, count: NonZero<u32>) -> Self {
        self.count = count;
        self
    }

    /// Sets the animation variant.
    #[must_use]
    pub const fn variant(mut self, value: Variant) -> Self {
        self.variant = value;
        self
    }

    /// Sets the shape variant for repeated item placeholders.
    #[must_use]
    pub const fn shape(mut self, value: Shape) -> Self {
        self.shape = value;
        self
    }

    /// Sets the skeleton item height CSS value.
    #[must_use]
    pub fn line_height(mut self, value: impl Into<String>) -> Self {
        self.line_height = value.into();
        self
    }

    /// Sets the gap CSS value.
    #[must_use]
    pub fn gap(mut self, value: impl Into<String>) -> Self {
        self.gap = value.into();
        self
    }

    /// Sets the optional leading circle size CSS value.
    #[must_use]
    pub fn leading_circle_size(mut self, value: impl Into<String>) -> Self {
        self.leading_circle_size = Some(value.into());
        self
    }
}
```

`Variant::None` is the explicit no-animation mode. Adapters must still respect
`prefers-reduced-motion` by suppressing animation styles even when the variant
is animated.

`Shape` applies to repeated item placeholders. The leading circle is a separate,
optional media placeholder controlled by `leading_circle_size`.

### 1.2 Connect / API

```rust
/// Structural parts exposed by the Skeleton connect API.
#[derive(ComponentPart)]
#[scope = "skeleton"]
pub enum Part {
    /// The root loading-status container.
    Root,
    /// The optional leading circle placeholder.
    Circle,
    /// A repeated skeleton item placeholder.
    Item {
        /// Zero-based item index.
        index: usize,
    },
}

/// API for the Skeleton component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Creates a new API for the skeleton.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, locale, messages }
    }

    /// Returns the number of repeated item placeholders.
    #[must_use]
    pub const fn count(&self) -> NonZero<u32> { self.props.count }

    /// Returns the repeated item shape.
    #[must_use]
    pub const fn shape(&self) -> Shape { self.props.shape }

    /// Returns the animation variant.
    #[must_use]
    pub const fn variant(&self) -> Variant { self.props.variant }

    /// Returns the zero-based item indices adapters should render.
    #[must_use]
    pub const fn item_indices(&self) -> Range<usize> {
        0..self.props.count.get() as usize
    }

    /// Returns whether a leading circle placeholder should be rendered.
    #[must_use]
    pub const fn has_leading_circle(&self) -> bool {
        self.props.leading_circle_size.is_some()
    }

    /// Returns root container attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "status");
        attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.loading_label)(&self.locale));
        attrs.set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str());
        attrs.set(HtmlAttr::Data("ars-shape"), self.props.shape.as_str());
        attrs.set_style(CssProperty::Custom("ars-skeleton-line-height"), &self.props.line_height);
        attrs.set_style(CssProperty::Custom("ars-skeleton-gap"), &self.props.gap);
        if self.props.variant.is_animated() {
            attrs.set_bool(HtmlAttr::Data("ars-animated"), true);
        }
        attrs
    }

    /// Returns attributes for the optional leading circle element.
    #[must_use]
    pub fn circle_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Circle.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if let Some(size) = &self.props.leading_circle_size {
            attrs.set_style(CssProperty::Custom("ars-skeleton-circle-size"), size);
        }
        attrs
    }

    /// Returns attributes for a skeleton item element.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Circle => self.circle_attrs(),
            Part::Item { index } => self.item_attrs(index),
        }
    }
}
```

## 2. Anatomy

```text
Skeleton
├── Root      (container; role="status", aria-busy="true")
├── Circle    (optional leading circle; data-ars-part="circle")
└── Item(s)   (one per `count`; data-ars-part="item")
```

| Part     | Element | Key Attributes                                                                                                                  |
| -------- | ------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `Root`   | `<div>` | `role="status"`, `aria-busy="true"`, `aria-label`, `data-ars-variant`, `data-ars-shape`, conditional `data-ars-animated="true"` |
| `Circle` | `<div>` | `aria-hidden="true"`, optional `--ars-skeleton-circle-size`                                                                     |
| `Item`   | `<div>` | `aria-hidden="true"`, `data-ars-index`                                                                                          |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `role="status"` on Root ensures screen readers announce the loading state.
- `aria-busy="true"` signals to assistive technology that content is being loaded.
- `aria-label` on Root provides the localized loading text via `Messages`.
- Item and Circle parts are `aria-hidden="true"` because they are decorative.
- All animation variants MUST respect `prefers-reduced-motion`; adapters suppress
  animation when the media query matches, even when the variant is animated.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the Skeleton component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the localized loading label.
    pub loading_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self { loading_label: MessageFn::static_str("Loading") }
    }
}
impl ComponentMessages for Messages {}
```

- The `loading_label` string comes from `Messages` and defaults to `"Loading"`.
- Host applications provide translated values via the messages provider.
- RTL requires no core attribute changes. Adapters should reverse directional
  sweep animations when `dir="rtl"` is active.

## 5. Library Parity

Skeleton aligns with React Spectrum and Radix loading-wrapper behavior while
also exposing Chakra/Ark-style animation variants (`pulse`, `shine`, `none`).
The core provides shape tokens (`text`, `circle`, `rect`) so adapters can style
common placeholder forms without introducing adapter-specific state.
