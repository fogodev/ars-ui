---
component: Meter
category: data-display
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  react-aria: Meter
---

# Meter

A gauge for displaying a scalar measurement within a known range. Unlike `Progress`, a Meter
represents a current measurement — not the completion of a task. Typical uses: disk usage,
battery level, password strength, temperature.

## 1. API

### 1.1 Props

```rust
/// Props for the Meter component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Current value.
    pub value: f64,
    /// Lower bound (default 0.0).
    pub min: f64,
    /// Upper bound (default 100.0).
    pub max: f64,
    /// Threshold below which the measurement is considered low.
    pub low: Option<f64>,
    /// Threshold above which the measurement is considered high.
    pub high: Option<f64>,
    /// The value considered optimal.
    pub optimum: Option<f64>,
    /// Format options for the value.
    pub format_options: Option<NumberFormatOptions>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            low: None,
            high: None,
            optimum: None,
            format_options: None,
        }
    }
}
```

### 1.2 Segment Derivation

The segment calculation follows the HTML `<meter>` specification algorithm:

```rust
/// Semantic classification mirroring the HTML <meter> spec.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    /// Value is in the optimal range.
    Optimal,
    /// Value is sub-optimal (low or high zone, but not the worst).
    SubOptimal,
    /// Value is in the worst zone.
    SubSubOptimal,
}

/// Computes the semantic segment for the given meter parameters.
pub fn compute_segment(
    value: f64,
    min: f64, max: f64,
    low: Option<f64>, high: Option<f64>, optimum: Option<f64>,
) -> Segment {
    let low   = low.unwrap_or(min);
    let high  = high.unwrap_or(max);
    let opt   = optimum.unwrap_or((min + max) / 2.0);

    // Which region does the optimum fall in?
    if opt < low {
        // Optimal region is the low zone
        if value < low        { Segment::Optimal }
        else if value <= high { Segment::SubOptimal }
        else                  { Segment::SubSubOptimal }
    } else if opt > high {
        // Optimal region is the high zone
        if value > high       { Segment::Optimal }
        else if value >= low  { Segment::SubOptimal }
        else                  { Segment::SubSubOptimal }
    } else {
        // Optimal region is the middle zone
        if value >= low && value <= high  { Segment::Optimal }
        else                              { Segment::SubOptimal }
    }
}

/// Computes the fill percentage for the given value within [min, max].
pub fn compute_percent(value: f64, min: f64, max: f64) -> f64 {
    ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
}
```

### 1.3 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "meter"]
pub enum Part {
    Root,
    Label,
    Track,
    Range,
    ValueText,
}

/// API for the Meter component.
pub struct Api<'a> {
    /// Current props of the meter.
    props: &'a Props,
    /// Resolved locale for message formatting.
    locale: Locale,
    /// Resolved messages for the meter.
    messages: Messages,
}

impl<'a> Api<'a> {
    /// Create a new API from props.
    pub fn new(props: &'a Props, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, locale, messages }
    }

    /// The semantic segment for the current value.
    pub fn segment(&self) -> Segment {
        compute_segment(
            self.props.value,
            self.props.min, self.props.max,
            self.props.low, self.props.high, self.props.optimum,
        )
    }

    /// The fill percentage for the current value.
    pub fn percent(&self) -> f64 {
        compute_percent(self.props.value, self.props.min, self.props.max)
    }

    /// Root attributes for the meter.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        // Prefer native <meter> element semantics. If rendered as <div>, add ARIA.
        // The adapter chooses the element; we emit all attributes for both cases.
        attrs.set(HtmlAttr::Role, "meter");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.props.value.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.props.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.props.max.to_string());

        let text = (self.messages.value_text)(
            self.props.value, self.props.min, self.props.max, &self.locale,
        );
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), text);

        // Native <meter> attributes (pass-through; no-op on <div>)
        attrs.set(HtmlAttr::Value, self.props.value.to_string());
        attrs.set(HtmlAttr::Min,   self.props.min.to_string());
        attrs.set(HtmlAttr::Max,   self.props.max.to_string());
        if let Some(l) = self.props.low    { attrs.set(HtmlAttr::Low,    l.to_string()); }
        if let Some(h) = self.props.high   { attrs.set(HtmlAttr::High,   h.to_string()); }
        if let Some(o) = self.props.optimum { attrs.set(HtmlAttr::Optimum, o.to_string()); }

        let segment = self.segment();
        attrs.set(HtmlAttr::Data("ars-segment"), match segment {
            Segment::Optimal       => "optimal",
            Segment::SubOptimal    => "sub-optimal",
            Segment::SubSubOptimal => "sub-sub-optimal",
        });
        attrs
    }

    /// Label attributes for the meter.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Track attributes for the meter.
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Range attributes for the meter.
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(CssProperty::Width, format!("{}%", self.percent()));
        attrs
    }

    /// Value text attributes for the meter.
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true"); // redundant with aria-valuetext on root
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::ValueText => self.value_text_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Meter
├── Root        (<meter> or div; role="meter", aria-valuenow/min/max/valuetext)
├── Label       (associated label)
├── Track       (background bar)
├── Range       (filled portion; width = percent%)
└── ValueText   (visible formatted value)
```

| Part        | Element              | Key Attributes                                                                                          |
| ----------- | -------------------- | ------------------------------------------------------------------------------------------------------- |
| `Root`      | `<meter>` or `<div>` | `role="meter"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`, `data-ars-segment` |
| `Label`     | `<label>`            | Associated via `for` or `aria-labelledby`                                                               |
| `Track`     | `<div>`              | Background                                                                                              |
| `Range`     | `<div>`              | `style="width: N%"`                                                                                     |
| `ValueText` | `<span>`             | `aria-hidden="true"`                                                                                    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part   | Role    | Properties                                                          |
| ------ | ------- | ------------------------------------------------------------------- |
| `Root` | `meter` | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext` |

- Prefer the native `<meter>` element for built-in browser semantics and AT support.
- If a custom element is used, set `role="meter"` (ARIA 1.3+). Fall back to
  `role="progressbar"` for broader support where `role="meter"` is unavailable.
- `aria-valuetext` provides a human-readable description: e.g. "72 GB used of 256 GB".

## 4. Internationalization

- `aria-valuetext` and `ValueText` formatted via `NumberFormatter` from `ars-i18n`.
- Segment colors (optimal = green, sub-optimal = yellow, sub-sub-optimal = red) are conveyed
  via `data-ars-segment` for CSS styling; color alone is never the sole indicator — a
  descriptive `aria-valuetext` must also be present.

### 4.1 Locale-Aware Zone Announcements

When the meter value crosses a zone boundary (e.g., from optimal to sub-optimal), the new zone name is announced via `aria-live="polite"`.

```rust
/// Messages for the Meter component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Formats the meter value for display and screen readers.
    /// Receives (value, min, max, locale).
    pub value_text: MessageFn<dyn Fn(f64, f64, f64, &Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            value_text: MessageFn::new(|value, _min, _max, _locale| {
                format!("{:.0}%", value * 100.0)
            }),
        }
    }
}
impl ComponentMessages for Messages {}
```

Zone semantics are value-based, not visual-direction-based — in RTL layouts the zone thresholds and announcements remain identical to LTR. The visual rendering direction is handled by the adapter.

### 4.2 Zone Enum with Locale-Aware Announcements

The `Zone` enum provides a structured representation of the meter's current semantic zone with locale-aware announcement support:

```rust
/// Semantic zone classification for screen reader announcements.
#[derive(Clone, Debug, PartialEq)]
pub enum Zone {
    /// Value is in the optimal range.
    Optimal,
    /// Value is sub-optimal but not critical.
    SubOptimal,
    /// Value is in the critical (worst) zone.
    Critical,
}

impl Zone {
    /// Derives the zone from the current `Segment`.
    pub fn from_segment(segment: &Segment) -> Self {
        match segment {
            Segment::Optimal       => Zone::Optimal,
            Segment::SubOptimal    => Zone::SubOptimal,
            Segment::SubSubOptimal => Zone::Critical,
        }
    }
}
```

The adapter tracks the previous zone and announces the new zone name via `LiveAnnouncer` whenever the value crosses a zone boundary. Zone names are fully translatable via `Messages` -- for example, Japanese locales may use "最適範囲", "注意範囲", "危険範囲" for Optimal, SubOptimal, and Critical respectively. Bipolar meters (where both extremes are bad) are supported by setting `optimum` to the midpoint, which classifies both low and high zones as SubOptimal/Critical via the standard segment derivation algorithm.

## 5. Library Parity

> Compared against: React Aria (`Meter`).

### 5.1 Props

| Feature                    | ars-ui                    | React Aria                 | Notes                                        |
| -------------------------- | ------------------------- | -------------------------- | -------------------------------------------- |
| `value`                    | `f64`                     | `number`                   | Equivalent                                   |
| `min` / `max`              | `f64`                     | `minValue` / `maxValue`    | Equivalent                                   |
| `low` / `high` / `optimum` | `Option<f64>`             | --                         | ars-ui original; follows HTML `<meter>` spec |
| `format_options`           | `NumberFormatOptions`     | `Intl.NumberFormatOptions` | Equivalent                                   |
| `valueLabel`               | Via `Messages.value_text` | `ReactNode`                | ars-ui uses message function pattern         |

**Gaps:** None.

### 5.2 Anatomy

| Part      | ars-ui      | React Aria | Notes                       |
| --------- | ----------- | ---------- | --------------------------- |
| Root      | `Root`      | `Meter`    | --                          |
| Label     | `Label`     | `Label`    | --                          |
| Track     | `Track`     | --         | ars-ui original visual part |
| Range     | `Range`     | --         | ars-ui original visual part |
| ValueText | `ValueText` | --         | ars-ui original visual part |

**Gaps:** None.

### 5.3 Events

No events or callbacks on either side. Meter is a display-only component.

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                            | React Aria                     |
| --------------------------- | --------------------------------- | ------------------------------ |
| Value display               | Yes                               | Yes                            |
| Percentage calculation      | Yes                               | Yes (`percentage` render prop) |
| Segment/zone classification | Yes (Optimal/SubOptimal/Critical) | No                             |
| HTML `<meter>` parity       | Yes (`low`/`high`/`optimum`)      | No                             |
| Zone change announcements   | Yes (live region)                 | No                             |
| Locale-aware formatting     | Yes                               | Yes                            |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity. ars-ui exceeds React Aria with HTML `<meter>` semantic zone support.
- **Divergences:** React Aria provides a minimal meter with value/min/max. ars-ui adds `low`/`high`/`optimum` thresholds from the HTML `<meter>` spec, segment derivation, and zone-crossing live region announcements.
- **Recommended additions:** None.
