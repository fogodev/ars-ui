---
component: Badge
category: data-display
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references: {}
---

# Badge

A small, inline label used to communicate status, a numeric count, a category,
or a lifecycle tag ("New", "Beta"). Badge is a static display component with no
state machine.

## 1. API

### 1.1 Props

```rust
/// Props for the Badge component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Visual style variant.
    pub variant: Variant,
    /// Visual size token.
    pub size: Size,
    /// Assistive-technology exposure mode.
    pub accessibility: Accessibility,
    /// Accessible label describing the badge content when visible text is not
    /// sufficient on its own.
    pub aria_label: Option<String>,
}

/// How the badge is exposed to assistive technology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Accessibility {
    /// Static visible content with an optional `aria-label`.
    Static,
    /// Visual-only badge hidden from assistive technology.
    Decorative,
    /// Polite live-region badge for dynamic, non-urgent updates.
    Status,
    /// Assertive alert badge for urgent status changes.
    Alert,
}

impl Accessibility {
    /// Returns whether this mode hides the badge from assistive technology.
    #[must_use]
    pub const fn is_decorative(self) -> bool { matches!(self, Self::Decorative) }
}

/// Visual style variant of the badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    /// Filled badge with the strongest emphasis.
    Solid,
    /// Soft tinted badge with low emphasis.
    Soft,
    /// Subtle tinted badge with low emphasis.
    Subtle,
    /// Badge with a visible surface/background treatment.
    Surface,
    /// Outlined badge with transparent or minimal fill.
    Outline,
    /// Plain text-like badge with minimal chrome.
    Plain,
}

impl Variant {
    /// Returns the `data-ars-variant` value for this variant.
    #[must_use]
    pub const fn as_str(self) -> &'static str { /* ... */ }
}

/// Visual size token of the badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    /// Extra-small badge size.
    Xs,
    /// Small badge size.
    Sm,
    /// Medium badge size.
    Md,
    /// Large badge size.
    Lg,
    /// Extra-large badge size.
    Xl,
}

impl Size {
    /// Returns the `data-ars-size` value for this size.
    #[must_use]
    pub const fn as_str(self) -> &'static str { /* ... */ }
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            variant: Variant::Subtle,
            size: Size::Sm,
            accessibility: Accessibility::Static,
            aria_label: None,
        }
    }
}

impl Props {
    /// Returns fresh badge props with the documented defaults.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the visual style variant.
    #[must_use]
    pub const fn variant(mut self, value: Variant) -> Self {
        self.variant = value;
        self
    }

    /// Sets the visual size token.
    #[must_use]
    pub const fn size(mut self, value: Size) -> Self {
        self.size = value;
        self
    }

    /// Sets how assistive technology should perceive the badge.
    #[must_use]
    pub const fn accessibility(mut self, value: Accessibility) -> Self {
        self.accessibility = value;
        self
    }

    /// Sets the accessible label for the badge.
    #[must_use]
    pub fn aria_label(mut self, value: impl Into<String>) -> Self {
        self.aria_label = Some(value.into());
        self
    }
}
```

`Variant` is a visual fill/style axis only. Semantic colors such as success,
warning, or destructive/error are styling-layer concerns and should be applied
through adapter props, CSS classes, CSS variables, or utility classes such as
Tailwind. The core only emits `data-ars-variant` and `data-ars-size` tokens.

Rendered badge text is adapter-owned children/content. The agnostic core owns
only visual tokens, accessibility mode, labels, message helpers, and generated
attributes.

Parity mapping:

- React Spectrum `fillStyle="bold"` maps to `Variant::Solid`.
- React Spectrum `fillStyle="subtle"` maps to `Variant::Subtle`.
- React Spectrum `fillStyle="outline"` maps to `Variant::Outline`.
- Radix `solid`, `soft`, `surface`, and `outline` map directly.
- Chakra/Ark-style `solid`, `subtle`, `outline`, `surface`, and `plain` map directly.
- React Spectrum `S`, `M`, `L`, `XL` map to `Sm`, `Md`, `Lg`, `Xl`.
- Radix `1`, `2`, `3` map to `Sm`, `Md`, `Lg`.
- Chakra/Ark-style `xs`, `sm`, `md`, `lg` map directly.

### 1.2 Connect / API

```rust
/// Structural parts exposed by the Badge connect API.
#[derive(ComponentPart)]
#[scope = "badge"]
pub enum Part {
    /// The root inline badge element.
    Root,
}

/// API for the Badge component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Creates a new API for the badge.
    #[must_use]
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, locale, messages }
    }

    /// Returns the overflow label for the given count, such as `"99+"`.
    #[must_use]
    pub fn overflow_label(&self, count: u64) -> String {
        (self.messages.overflow_label)(count, &self.locale)
    }

    /// Returns the badge's accessible label for a count and category.
    #[must_use]
    pub fn badge_label(&self, count: u64, category: &str) -> String {
        (self.messages.badge_label)(count, category, &self.locale)
    }

    /// Returns root attributes for the badge.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-variant"), self.props.variant.as_str());
        attrs.set(HtmlAttr::Data("ars-size"), self.props.size.as_str());

        match self.props.accessibility {
            Accessibility::Decorative => {
                attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
                return attrs;
            }
            Accessibility::Alert => {
                attrs.set(HtmlAttr::Role, "alert");
            }
            Accessibility::Status => {
                attrs.set(HtmlAttr::Role, "status");
                attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
            }
            Accessibility::Static => {}
        }

        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }
        attrs
    }

    /// Returns the badge accessibility mode.
    #[must_use]
    pub const fn accessibility(&self) -> Accessibility { self.props.accessibility }

    /// Returns the badge aria-label override, when present.
    #[must_use]
    pub fn aria_label(&self) -> Option<&str> { self.props.aria_label.as_deref() }
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
Badge
└── Root    (<span>; data-ars-scope="badge" data-ars-part="root")
```

| Part   | Element  | Key Attributes                      |
| ------ | -------- | ----------------------------------- |
| `Root` | `<span>` | `data-ars-variant`, `data-ars-size` |

### 2.1 Semantic HTML Rules

Badges MUST render as `<span>` elements (inline, non-interactive). Never render
badges as `<div>` (block-level breaks inline flow) or `<button>` (implies
interactivity that does not exist).

- `Accessibility::Static` uses a plain `<span>` plus `aria-label` when visible
  content is not sufficient.
- `Accessibility::Decorative` sets `aria-hidden="true"` and suppresses role,
  live-region attributes, and `aria-label`.
- `Accessibility::Status` sets `role="status"` and `aria-live="polite"`.
- `Accessibility::Alert` sets `role="alert"`.
- Interactive badges must be wrapped in a `<button>` parent. The badge `<span>`
  itself remains non-interactive.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Scenario                                      | Recommended pattern                            |
| --------------------------------------------- | ---------------------------------------------- |
| Decorative (visual only, described by parent) | `aria-hidden="true"` on Root                   |
| Static label                                  | Text content plus optional `aria-label`        |
| Dynamic notification count                    | `role="status"` + `aria-live="polite"` on Root |
| Critical state badge                          | `role="alert"` on Root                         |
| Interactive badge                             | Wrap `<span>` badge in a `<button>` parent     |

Numeric badge content should use locale number formatting:

```rust
/// Formats a count value as locale-aware text with a maximum visible value of
/// `99+`.
#[must_use]
pub fn format_count(value: u64, locale: &Locale) -> String {
    let formatter = number::Formatter::new(locale, number::FormatOptions::default());
    if value > 99 {
        format!("{}+", formatter.format(99.0))
    } else {
        formatter.format(value as f64)
    }
}
```

## 4. Internationalization

- Numeric values in badges are formatted with `number::Formatter` from
  `ars-i18n`.
- Textual labels like "New" or "Beta" must come from a localized message
  catalog; do not hard-code English strings in adapter components.
- Overflow display ("99+") is locale-aware; provide a translation slot.

### 4.1 Messages

```rust
type OverflowLabelFn = dyn Fn(u64, &Locale) -> String + Send + Sync;
type BadgeLabelFn = dyn Fn(u64, &str, &Locale) -> String + Send + Sync;

/// Messages for the Badge component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the overflow label, receiving the count and current locale.
    pub overflow_label: MessageFn<OverflowLabelFn>,
    /// Returns the badge's accessible label using the count, semantic category,
    /// and current locale.
    pub badge_label: MessageFn<BadgeLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            overflow_label: MessageFn::new(|count: u64, locale: &Locale| {
                format_count(count, locale)
            }),
            badge_label: MessageFn::new(
                |count: u64, category: &str, _locale: &Locale| match count {
                    0 => format!("No unread {category}s"),
                    1 => format!("1 unread {category}"),
                    n => format!("{n} unread {category}s"),
                },
            ),
        }
    }
}
impl ComponentMessages for Messages {}
```

## 5. Library Parity

Badge aligns its visual `Variant` and `Size` token surface with React Spectrum,
Radix Themes, and Chakra/Ark-style design-system APIs. Semantic color/tone
tokens are deliberately not part of the agnostic core contract.
