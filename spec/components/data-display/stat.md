---
component: Stat
category: data-display
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references: {}
---

# Stat

A metric summary component displaying a key figure alongside a human-readable label, an
optional change delta, and a trend indicator. Commonly used in dashboards and analytics
surfaces.

## 1. API

### 1.1 Props

```rust
/// Trend direction for the Stat component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trend {
    /// The value is increasing.
    Up,
    /// The value is decreasing.
    Down,
    /// The value is neutral.
    Neutral
}

/// Props for the Stat component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The formatted metric value.
    pub value: String,
    /// The metric label.
    pub label: String,
    /// Optional change delta as a percentage (e.g. 12.5 means +12.5%).
    pub change: Option<f64>,
    /// Override trend direction (derived from change.signum() if omitted).
    pub trend: Option<Trend>,
    /// Optional supplementary description.
    pub help_text: Option<String>,
    /// Show skeleton state while data is loading.
    pub loading: bool,
    /// Formatting options passed to ars-i18n NumberFormatter.
    pub format_options: Option<NumberFormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: String::new(),
            label: String::new(),
            change: None,
            trend: None,
            help_text: None,
            loading: false,
            format_options: None,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "stat"]
pub enum Part {
    Root,
    Label,
    Value,
    Change,
    TrendIndicator,
    HelpText,
}

/// API for the Stat component.
pub struct Api<'a> {
    /// Current props of the stat.
    props: &'a Props,
    /// Resolved messages for the stat.
    messages: Messages,
    /// Resolved locale for message formatting.
    locale: Locale,
}

impl<'a> Api<'a> {
    /// Create a new API for the stat.
    pub fn new(props: &'a Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, messages, locale }
    }

    /// Derive trend from change if not explicitly set.
    pub fn resolved_trend(&self) -> Option<Trend> {
        self.props.trend.or_else(|| {
            self.props.change.map(|c| {
                if c > 0.0       { Trend::Up }
                else if c < 0.0  { Trend::Down }
                else             { Trend::Neutral }
            })
        })
    }

    /// Format change delta for display: "+12.5%" or "−3.2%".
    pub fn formatted_change(&self) -> Option<String> {
        let change = self.props.change?;
        let locale = &self.locale;
        let fmt = NumberFormatter::new(locale, self.props.format_options.as_ref());
        let pct = fmt.format_percent(change.abs() / 100.0);
        let msgs = &self.messages;
        let label = match self.resolved_trend() {
            Some(Trend::Up)      => format!("{} {}", (msgs.increase_prefix)(locale), pct),
            Some(Trend::Down)    => format!("{} {}", (msgs.decrease_prefix)(locale), pct),
            Some(Trend::Neutral) | None => pct,
        };
        Some(label)
    }

    /// Screen-reader-friendly change announcement.
    /// Delegates to `messages.change_label` closure for locale-aware formatting.
    pub fn change_aria_label(&self) -> Option<String> {
        let change = self.props.change?;
        let trend = self.resolved_trend().unwrap_or(Trend::Neutral);
        Some((self.messages.change_label)(change.abs(), trend, &self.locale))
    }

    /// Returns the root attributes for the stat.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        // Combine label and value into a cohesive aria-label so screen readers
        // announce the stat as a single unit (e.g., "Total Revenue: $45,231").
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), format!("{}: {}", self.props.label, self.props.value));
        if self.props.loading {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        }
        attrs
    }

    /// Returns the label attributes for the stat.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the value attributes for the stat.
    pub fn value_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Value.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the change attributes for the stat.
    pub fn change_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Change.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(t) = self.resolved_trend() {
            attrs.set(HtmlAttr::Data("ars-trend"), match t {
                Trend::Up      => "up",
                Trend::Down    => "down",
                Trend::Neutral => "neutral",
            });
        }
        // Announce descriptively; do not rely on the raw delta symbol alone
        if let Some(label) = self.change_aria_label() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        attrs
    }

    /// Returns the trend indicator attributes for the stat.
    pub fn trend_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TrendIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Decorative — aria label on change_attrs covers the meaning
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if let Some(t) = self.resolved_trend() {
            attrs.set(HtmlAttr::Data("ars-trend"), match t {
                Trend::Up      => "up",
                Trend::Down    => "down",
                Trend::Neutral => "neutral",
            });
        }
        attrs
    }

    /// Returns the help text attributes for the stat.
    pub fn help_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HelpText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root           => self.root_attrs(),
            Part::Label          => self.label_attrs(),
            Part::Value          => self.value_attrs(),
            Part::Change         => self.change_attrs(),
            Part::TrendIndicator => self.trend_indicator_attrs(),
            Part::HelpText       => self.help_text_attrs(),
        }
    }
}
```

## 2. Anatomy

The Stat component follows a `Stat` -> `Label` + `Value` + `Trend` (optional sub-component, not the enum)
sub-component pattern:

```text
Stat
├── Root (StatRoot)     (container; role="group", aria-label="{label}: {value}")
├── Label               (metric name; e.g. "Total Revenue")
├── Value               (primary value; e.g. "$45,231")
├── Trend               (optional; change + trend indicator)
│   ├── Change          (delta text; e.g. "↑ 12.5%"; aria-label="12.5% increase")
│   └── TrendIndicator  (icon/arrow; aria-hidden="true")
└── HelpText            (optional description text)
```

| Part             | Element             | Key Attributes                                                             |
| ---------------- | ------------------- | -------------------------------------------------------------------------- |
| `Root`           | `<div>`             | `role="group"`, `aria-label`, `aria-busy` when loading, `data-ars-loading` |
| `Label`          | `<dt>` or `<span>`  | Descriptive text (e.g. "Total Revenue")                                    |
| `Value`          | `<dd>` or `<span>`  | The primary value (e.g. "$45,231")                                         |
| `Change`         | `<span>`            | `aria-label` with descriptive text, `data-ars-trend`                       |
| `TrendIndicator` | `<span>` / SVG icon | `aria-hidden="true"`, `data-ars-trend`                                     |
| `HelpText`       | `<p>`               | —                                                                          |

The `Trend` sub-component wraps `Change` and `TrendIndicator` and is only rendered when
a `trend` or `change` value is provided. It uses `Trend` enum to indicate direction:

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part             | Role/Property  | Value / Condition                                        |
| ---------------- | -------------- | -------------------------------------------------------- |
| `Root`           | `role="group"` | Always                                                   |
| `Root`           | `aria-label`   | `"{label}: {value}"` — cohesive stat announcement        |
| `Root`           | `aria-busy`    | `"true"` when `loading` is set                           |
| `Change`         | `aria-label`   | Descriptive text (e.g. "12.5% increase"), not raw symbol |
| `TrendIndicator` | `aria-hidden`  | `"true"` — purely decorative                             |

- **Grouping**: Root MUST use `role="group"` with an `aria-label` that combines label and
  value (e.g., `"Total Revenue: $45,231"`). This ensures screen readers announce the stat
  as a cohesive unit rather than reading label and value as disconnected text fragments.
- **Label-Value association**: Render as a `<dl>` (description list) with `Label` as
  `<dt>` and `Value` as `<dd>` for native semantic association. Alternatively, use
  `aria-labelledby` pointing from `Value` to `Label`.
- **Loading state**: `aria-busy="true"` on Root signals to AT that content is updating.
  Use a live region (`role="status"` or `aria-live="polite"`) to announce when data arrives.
- **Change announcement**: The `Change` part must not rely on directional symbols (up/down arrows) alone.
  Its `aria-label` must be fully descriptive: "12.5% increase", not "up arrow 12.5%".
- **Trend indicators**: Always `aria-hidden="true"` — they are purely decorative; the
  `Change` part's `aria-label` carries all semantic meaning.
- **Color**: Trend color (green/red) is never the sole indicator — `data-ars-trend` and
  `aria-label` always accompany it.

## 4. Internationalization

- `Value` is formatted by the host application using `NumberFormatter` from `ars-i18n`
  before being passed as the `value` prop; Stat does not format internally.
- `Change` delta uses `NumberFormatter::format_percent()` for locale-aware percentage
  rendering (e.g. "12,5 %" in French).
- `Messages` keys ("increase", "decrease", "no change"; directional prefixes) are
  localizable strings that the host application supplies from a message catalog.
- **RTL**: `Root` receives `dir="rtl"` from the adapter when the active locale is
  right-to-left. The `TrendIndicator` arrow icon must be mirrored — use CSS
  `[dir="rtl"] [data-ars-part="trend-indicator"] { transform: scaleX(-1); }` or supply
  mirrored SVG assets.

### 4.1 Messages

```rust
/// Messages for the Stat component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Prefix for positive change display: "↑" or "+" or locale equivalent.
    pub increase_prefix: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Prefix for negative change display: "↓" or "−".
    pub decrease_prefix: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Locale-aware closure that produces the full accessible label for a change.
    /// Receives the absolute percentage value, trend direction, and locale.
    /// Example (en): `(12.5, Trend::Up, en) → "12.5% increase"`.
    /// Example (ja): `(12.5, Trend::Up, ja) → "12.5%の増加"`.
    pub change_label: MessageFn<dyn Fn(f64, Trend, &Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            increase_prefix: MessageFn::static_str("↑"),
            decrease_prefix: MessageFn::static_str("↓"),
            change_label: MessageFn::new(|pct, trend, _locale| {
                let suffix = match trend {
                    Trend::Up      => "increase",
                    Trend::Down    => "decrease",
                    Trend::Neutral => "no change",
                };
                format!("{pct:.1}% {suffix}")
            }),
        }
    }
}
impl ComponentMessages for Messages {}
```

### 4.2 Currency and Unit Formatting

When `Stat` displays monetary or unit values, the `value` prop MUST be pre-formatted
by the host application using the appropriate formatter. `ars-i18n` provides wrappers
for locale-correct formatting:

```rust
/// Formats a currency value with locale-correct symbol placement and grouping.
///
/// Uses ICU4X `DecimalFormatter` + `CurrencyFormatter` under the hood.
///
/// Symbol placement varies by locale:
/// - `en-US`: "$1,234.56" (symbol before, comma grouping, period decimal)
/// - `de-DE`: "1.234,56 €" (symbol after with space, period grouping, comma decimal)
/// - `ja-JP`: "￥1,234" (symbol before, no decimal for JPY)
/// - `ar-SA`: "١٬٢٣٤٫٥٦ ر.س." (Arabic-Indic numerals, symbol after)
pub struct CurrencyFormatter {
    locale: Locale,
    currency_code: String, // ISO 4217, e.g. "USD", "EUR", "JPY"
}

impl CurrencyFormatter {
    pub fn new(locale: &Locale, currency_code: &str) -> Self { /* ... */ }
    pub fn format(&self, value: f64) -> String { /* ... */ }
}

/// Formats a value with a measurement unit, respecting locale conventions.
///
/// Uses ICU4X `MeasureUnit` for placement rules.
///
/// Placement varies by locale:
/// - `en-US`: "98.6°F", "5 lbs"
/// - `de-DE`: "37 °C", "2,3 kg"
/// - `ja-JP`: "37°C", "5kg"
/// - RTL locales: unit suffix ordering follows bidi algorithm
pub struct UnitFormatter {
    locale: Locale,
    unit: String,          // CLDR unit ID, e.g. "temperature-celsius"
    unit_display: UnitDisplay, // Long, Short, Narrow
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitDisplay { Long, Short, Narrow }

impl UnitFormatter {
    pub fn new(locale: &Locale, unit: &str, display: UnitDisplay) -> Self { /* ... */ }
    pub fn format(&self, value: f64) -> String { /* ... */ }
}
```

The `Messages::change_label` closure receives the locale so that trend descriptions
can use locale-appropriate word order and interpolation. Per-component message templates
use ICU MessageFormat-style interpolation for locale-aware string building.

### 4.3 Currency Symbol Placement

The `CurrencyFormatter` trait defined above handles locale-aware currency symbol placement. To streamline currency display in Stat, Progress, and Meter components:

**Stat `currency` Prop:**

```rust
pub struct Props {
    // ... existing fields ...
    /// Optional ISO 4217 currency code (e.g., "USD", "EUR", "JPY").
    /// When set, the `value` field is interpreted as a raw numeric value
    /// and formatted using `CurrencyFormatter::new(&locale, &currency_code)`.
    pub currency: Option<CurrencyCode>,
}

/// ISO 4217 currency code wrapper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrencyCode(pub String);
```

**Symbol Placement Rules** (determined entirely by locale ICU data):

- Position (prefix vs suffix) is locale-dependent, NOT currency-dependent.
- Spacing between symbol and value is locale-dependent (e.g., `€1.00` in `en-IE` vs `1,00 €` in `de-DE`).
- Narrow symbol variants (e.g., `$` vs `US$`) are selected automatically when the currency is unambiguous in the locale context.

**Progress/Meter**: These components do not directly format currency values. When used to display monetary progress (e.g., fundraising), the application MUST pre-format `value_label` using `CurrencyFormatter` and pass it as a string prop.

## 5. Library Parity

> No counterpart in Ark UI, Radix UI, or React Aria. Original ars-ui component.
