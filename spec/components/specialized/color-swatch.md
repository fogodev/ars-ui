---
component: ColorSwatch
category: specialized
tier: stateless
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-swatch-picker, color-picker]
references:
  ark-ui: ColorPicker
  react-aria: ColorSwatch
---

# ColorSwatch

A non-interactive display element that renders a color preview with an accessible
name. `ColorSwatch` is stateless — no `Machine` is needed. It produces a visual
color sample with a perceptual color description for screen readers, using the
`ColorNameParts` system from `ColorPicker`. Reuses `ColorValue` and `ColorNameParts`
from the shared color types.

## 1. API

### 1.1 Props

```rust
use crate::color_picker::ColorValue;
use crate::machine::{AttrMap, ComponentIds};

#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The color to display.
    pub color: ColorValue,
    /// Optional override for the auto-generated accessible color name.
    /// When `None`, the name is derived from `color.color_name_parts()` + `messages.format_name`.
    pub color_name: Option<String>,
    /// Whether to visually represent the alpha channel (checkerboard pattern).
    pub respect_alpha: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            color: ColorValue::default(),
            color_name: None,
            respect_alpha: true,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-swatch"]
pub enum Part {
    Root,
    Inner,
}

pub struct Api<'a> {
    props: &'a Props,
    id: String,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    pub fn new(props: &'a Props, env: &Env, messages: &Messages) -> Self {
        let id = if props.id.is_empty() { generate_id() } else { props.id.clone() };
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, id, locale, messages }
    }

    /// The resolved accessible color name — either the explicit override or
    /// the auto-generated perceptual name.
    pub fn color_name(&self) -> String {
        if let Some(ref name) = self.props.color_name {
            name.clone()
        } else {
            let parts = self.props.color.color_name_parts();
            (self.messages.format_name)(&parts, &self.locale)
        }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.id);
        attrs.set(HtmlAttr::Role, "img");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.color_name());
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.messages.role_description)(&self.locale));
        attrs.set_style(CssProperty::Custom("--ars-swatch-color"), self.props.color.to_css_hsl());
        attrs
    }

    pub fn inner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Inner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(CssProperty::Background, self.props.color.to_css_hsl());
        if self.props.respect_alpha && self.props.color.alpha < 1.0 {
            attrs.set_bool(HtmlAttr::Data("ars-alpha"), true);
        }
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Inner => self.inner_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ColorSwatch
├── Root   (required — <div>, role="img", aria-label)
└── Inner  (required — <div>, color fill with optional checkerboard for alpha)
```

| Part  | Element | Key Attributes                                                           |
| ----- | ------- | ------------------------------------------------------------------------ |
| Root  | `<div>` | `role="img"`, `aria-label`, `aria-roledescription`, `--ars-swatch-color` |
| Inner | `<div>` | `background` style, `data-ars-alpha` when alpha < 1.0                    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part | Role  | Properties                                                                  |
| ---- | ----- | --------------------------------------------------------------------------- |
| Root | `img` | `aria-label` (perceptual color name), `aria-roledescription="color swatch"` |

- Not focusable — no `tabindex`. ColorSwatch is display-only.
- When embedded in `ColorSwatchPicker`, the interactive behavior (selection, keyboard navigation) is provided by the `role="option"` wrapper element, not the swatch itself.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Formats the color name parts into a localized string.
    /// Default (en): "{lightness} {chroma} {hue}".
    pub format_name: MessageFn<dyn Fn(&ColorNameParts, &Locale) -> String + Send + Sync>,
    /// Role description for the swatch element (default: "color swatch").
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            format_name: MessageFn::new(|parts, _locale| {
                [parts.lightness, parts.chroma, parts.hue]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            role_description: MessageFn::static_str("color swatch"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                             | Default (en-US)                | Purpose                                                      |
| ------------------------------- | ------------------------------ | ------------------------------------------------------------ |
| `color_swatch.format_name`      | `"{lightness} {chroma} {hue}"` | Assembles perceptual color description from `ColorNameParts` |
| `color_swatch.role_description` | `"color swatch"`               | Root element aria-roledescription                            |

- **format_name**: Receives `ColorNameParts` and assembles them in locale-appropriate order. English uses `"{lightness} {chroma} {hue}"`, but other languages may reorder (e.g., Italian: `"{hue} {chroma} {lightness}"`).
- **Color name tokens**: The ~30 lightness/chroma/hue tokens (e.g., "dark", "vibrant", "blue") are translatable via the i18n system. The `color_name_parts()` method returns English keys; `format_name` maps to localized strings.
- **RTL**: No layout implications (non-interactive).

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker.Swatch`), React Aria (`ColorSwatch`).

### 5.1 Props

| Feature           | ars-ui          | Ark UI         | React Aria  | Notes      |
| ----------------- | --------------- | -------------- | ----------- | ---------- |
| `color` / `value` | `color`         | `value`        | `color`     | Equivalent |
| `colorName`       | `color_name`    | --             | `colorName` | Equivalent |
| `respectAlpha`    | `respect_alpha` | `respectAlpha` | --          | Equivalent |

**Gaps:** None.

### 5.2 Anatomy

| Part  | ars-ui  | Ark UI   | React Aria    | Notes                         |
| ----- | ------- | -------- | ------------- | ----------------------------- |
| Root  | `Root`  | `Swatch` | `ColorSwatch` | Equivalent                    |
| Inner | `Inner` | --       | --            | ars-ui has inner fill element |

**Gaps:** None.

### 5.3 Events

No events -- stateless display component in all libraries.

**Gaps:** None.

### 5.4 Features

| Feature                    | ars-ui                 | Ark UI               | React Aria        |
| -------------------------- | ---------------------- | -------------------- | ----------------- |
| Perceptual color naming    | Yes (`ColorNameParts`) | --                   | Yes (`colorName`) |
| Alpha transparency display | Yes (`respect_alpha`)  | Yes (`respectAlpha`) | --                |
| Role="img" with aria-label | Yes                    | --                   | Yes               |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** None significant.
- **Recommended additions:** None.
