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

A loading placeholder component that renders animated shapes to indicate content is being
loaded. Skeleton is a pure display component with no interactive states or state machine.

## 1. API

### 1.1 Props

```rust
/// Props for the Skeleton component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Number of skeleton lines to render.
    pub count: NonZero<u32>,
    /// Animation variant.
    pub variant: Variant,
    /// Height of each skeleton line in CSS units.
    pub line_height: String,
    /// Gap between skeleton lines in CSS units.
    pub gap: String,
    /// Size of the optional leading circle (e.g., avatar placeholder).
    /// `None` means no circle is rendered.
    pub circle_size: Option<String>,
    /// Whether animation is enabled.
    pub animated: bool,
}

/// Animation variant for Skeleton loading placeholders.
///
/// All variants MUST respect `prefers-reduced-motion`: when the user has enabled
/// reduced motion, the adapter MUST use a static placeholder (no animation) or a
/// very subtle opacity change (max 5% opacity delta, ≥2s cycle). The `animated`
/// prop on `Props` provides programmatic control, but `prefers-reduced-motion`
/// takes precedence — even if `animated` is `true`, the adapter suppresses
/// animation when the media query matches.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant {
    /// Opacity fades in and out (default). Least distracting; preferred for
    /// `prefers-reduced-motion` fallback when a subtle animation is acceptable.
    Pulse,
    /// Left-to-right sweep highlight. The highlight bar moves from the leading
    /// edge to the trailing edge of each skeleton item.
    Wave,
    /// Gradient sweep animation. A diagonal gradient band sweeps across the
    /// skeleton surface, producing a metallic shimmer effect.
    Shimmer,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            count: NonZero::new(1).expect("non-zero"),
            variant: Variant::Pulse,
            line_height: "1rem".into(),
            gap: "0.5rem".into(),
            circle_size: None,
            animated: true,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "skeleton"]
pub enum Part {
    Root,
    Circle,
    Item { index: usize },
}

pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, locale, messages }
    }

    /// Root container attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "status");
        p.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        p.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.loading_label)(&self.locale));
        p.set(HtmlAttr::Data("ars-variant"), match self.props.variant {
            Variant::Pulse   => "pulse",
            Variant::Wave    => "wave",
            Variant::Shimmer => "shimmer",
        });
        if self.props.animated {
            p.set_bool(HtmlAttr::Data("ars-animated"), true);
        }
        // CSS custom properties for adapter styling
        p.set_style(CssProperty::Custom("ars-skeleton-line-height"), &self.props.line_height);
        p.set_style(CssProperty::Custom("ars-skeleton-gap"), &self.props.gap);
        p
    }

    /// Attributes for the optional leading circle element.
    pub fn circle_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Circle.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if let Some(size) = &self.props.circle_size {
            p.set_style(CssProperty::Custom("ars-skeleton-circle-size"), size);
        }
        p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        p
    }

    /// Attributes for each skeleton line element.
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-index"), index.to_string());
        p.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        p
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

| Part     | Element | Key Attributes                                                                             |
| -------- | ------- | ------------------------------------------------------------------------------------------ |
| `Root`   | `<div>` | `role="status"`, `aria-busy="true"`, `aria-label`, `data-ars-variant`, `data-ars-animated` |
| `Circle` | `<div>` | `aria-hidden="true"`, `--ars-skeleton-circle-size`                                         |
| `Item`   | `<div>` | `aria-hidden="true"`, `data-ars-index`                                                     |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **`role="status"`** on Root ensures screen readers announce the loading state.
- **`aria-busy="true"`** signals to assistive technology that content is being loaded.
- **`aria-label`** on Root provides the localized "Loading" text via `Messages`.
- **Item and Circle parts** are `aria-hidden="true"` — they are purely decorative placeholders.
- **`prefers-reduced-motion`**: All animation variants (`Pulse`, `Wave`, `Shimmer`) MUST
  respect the `prefers-reduced-motion` media query. When the user has enabled reduced motion:
  1. The adapter MUST suppress all animation and display a static placeholder with a fixed
     background color (no movement, no opacity cycling).
  2. Alternatively, a very subtle `Pulse` (max 5% opacity delta, cycle duration >=2s) is
     acceptable as a minimal loading indicator.
  3. `Wave` and `Shimmer` MUST be fully disabled under reduced motion — their sweeping
     movement is too visually aggressive.
  4. CSS implementation: `@media (prefers-reduced-motion: reduce) { [data-ars-animated] { animation: none; } }`.
  5. The `prefers-reduced-motion` media query takes precedence over the `animated` prop:
     even if `animated` is `true`, the adapter suppresses animation when the query matches.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the Skeleton component.
#[derive(Clone, Debug)]
pub struct Messages {
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
- Host applications provide translated values via the `messages` prop or a `MessagesProvider`.
- **RTL**: Skeleton items are purely visual and require no directional adjustment. However,
  if the `Wave` or `Shimmer` animation sweeps left-to-right, the adapter should reverse the
  animation direction when `dir="rtl"` is active.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
