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

A small, inline label used to communicate status, a numeric count, a category, or a
lifecycle tag ("New", "Beta"). Badge is a static display component with no state machine.

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
    /// Size token.
    pub size: Size,
    /// Text content of the badge (e.g. "3", "New", "Beta").
    pub content: Option<String>,
    /// When `true`, renders with `role="status"` and `aria-live="polite"` so screen
    /// readers announce value changes. Use for notification counts or status indicators
    /// that update at runtime. Default: `false`.
    pub dynamic: bool,
}

/// Visual style variant of the badge.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant {
    /// Default neutral style.
    Default,
    /// Subdued secondary style.
    Secondary,
    /// Destructive / error style (red).
    Destructive,
    /// Outlined badge; no fill.
    Outline,
    /// Success / positive style (green).
    Success,
    /// Warning / caution style (amber).
    Warning,
}

/// Visual size of the badge.
#[derive(Clone, Debug, PartialEq)]
pub enum Size {
    /// Small size.
    Sm,
    /// Medium size.
    Md,
    /// Large size.
    Lg,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            variant: Variant::Default,
            size: Size::Md,
            content: None,
            dynamic: false,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "badge"]
pub enum Part {
    Root,
}

/// API for the Badge component.
pub struct Api {
    props: Props,
    locale: Locale,
    messages: Messages,
}

impl Api {
    /// Create a new API for the badge.
    pub fn new(props: Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, locale, messages }
    }

    /// Returns the overflow label for the given count (e.g. "99+").
    pub fn overflow_label(&self, count: u64) -> String {
        (self.messages.overflow_label)(count, &self.locale)
    }

    /// Returns the badge's accessible label for the given count and category
    /// (e.g. "3 unread messages").
    pub fn badge_label(&self, count: u64, category: &str) -> String {
        (self.messages.badge_label)(count, category, &self.locale)
    }

    /// Root attributes for the badge.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-variant"), match self.props.variant {
            Variant::Default     => "default",
            Variant::Secondary   => "secondary",
            Variant::Destructive => "destructive",
            Variant::Outline     => "outline",
            Variant::Success     => "success",
            Variant::Warning     => "warning",
        });
        attrs.set(HtmlAttr::Data("ars-size"), match self.props.size {
            Size::Sm => "sm",
            Size::Md => "md",
            Size::Lg => "lg",
        });
        // Dynamic badges (notification counts, status indicators that update) get
        // role="status" + aria-live so screen readers announce changes.
        if self.props.dynamic {
            attrs.set(HtmlAttr::Role, "status");
            attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        }
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
Badge
└── Root    (<span>; data-ars-scope="badge" data-ars-part="root")
```

| Part   | Element  | Key Attributes                      |
| ------ | -------- | ----------------------------------- |
| `Root` | `<span>` | `data-ars-variant`, `data-ars-size` |

### 2.1 Semantic HTML Rules

Badges MUST render as `<span>` elements (inline, non-interactive). Never render badges as
`<div>` (block-level breaks inline flow) or `<button>` (implies interactivity that does not
exist).

- **Static badges** (labels, categories like "New", "Beta"): use a plain `<span>` with no
  special ARIA role.
- **Dynamic badges** (notification counts, status indicators whose value updates at runtime):
  use `<span role="status" aria-live="polite">` so screen readers announce value changes
  without requiring the user to navigate to the badge.
- The adapter determines which pattern to use based on whether the badge value is
  reactive/dynamic. A `dynamic: bool` prop on `Props` controls this.
- If a badge needs to be interactive (clickable or dismissible), it MUST be wrapped in a
  `<button>` parent element. The badge `<span>` itself remains non-interactive.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Scenario                                      | Recommended pattern                                                              |
| --------------------------------------------- | -------------------------------------------------------------------------------- |
| Decorative (visual only, described by parent) | `aria-hidden="true"` on Root                                                     |
| Static label (e.g. "New") in a card           | Include text in card's `aria-label`; hide badge with `aria-hidden`               |
| Dynamic notification count                    | `role="status"` + `aria-live="polite"` on Root; content is the count             |
| Critical state badge                          | `role="alert"` for immediate announcement                                        |
| Interactive badge (clickable/dismissible)     | Wrap `<span>` badge in a `<button>` parent; badge itself remains non-interactive |

- Badge MUST accept an `aria-label` prop describing what the count represents (e.g.,
  `"3 unread messages"`). The numeric content alone is meaningless to screen readers
  without context. When no `aria-label` is provided, the component should log a
  development-mode warning.
- Numeric badge content should use locale number formatting:

```rust
/// Formats a count value as a string with a maximum of 99.
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

- Numeric values in badges (e.g. notification counts) are formatted with
  `number::Formatter` from `ars-i18n`. When locale is inherited from
  `ArsProvider`, adapters should derive the formatter through
  `use_number_formatter(...)`.
- Textual labels like "New" or "Beta" must come from a localized message catalog; do not
  hard-code English strings.
- Overflow display ("99+") is locale-aware: the `+` suffix may need to move to prefix in
  some languages; provide a translation slot:

### 4.1 Messages

```rust
/// Messages for the Badge component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the overflow label. Receives the count and the current locale
    /// so that locale-aware formatting (e.g., prefix vs. suffix `+`) can be applied.
    pub overflow_label: MessageFn<dyn Fn(u64, &Locale) -> String + Send + Sync>,
    /// Returns the badge's accessible label using plural rules.
    /// Receives the count, the semantic category (e.g., "message", "notification"),
    /// and the locale for pluralization.
    /// Example: `(3, "message", en) → "3 unread messages"`.
    pub badge_label: MessageFn<dyn Fn(u64, &str, &Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            overflow_label: MessageFn::new(|count, _locale| format!("{}+", count)),
            badge_label: MessageFn::new(|count, category, _locale| match count {
                0 => format!("No unread {category}s"),
                1 => format!("1 unread {category}"),
                n => format!("{n} unread {category}s"),
            }),
        }
    }
}
impl ComponentMessages for Messages {}
```

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
