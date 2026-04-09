---
component: QrCode
category: specialized
tier: stateless
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
  ark-ui: QrCode
---

# QrCode

A declarative component that takes an input string and renders a QR code matrix.
There is no user interaction state machine — `QrCode` is stateless.

Supporting types used by Props and the API:

```rust
/// Error correction level for QR encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QrErrorCorrection {
    /// ~7% recovery.
    Low,
    /// ~15% recovery.
    #[default]
    Medium,
    /// ~25% recovery.
    Quartile,
    /// ~30% recovery.
    High,
}

/// The QR code matrix — a 2D grid of modules (black/white cells).
#[derive(Clone, Debug)]
pub struct QrMatrix {
    /// Row-major module data. `true` = dark module.
    pub modules: Vec<Vec<bool>>,
    /// Size (number of modules per side).
    pub size: usize,
}

impl QrMatrix {
    /// Create a QrMatrix from a pre-computed module grid.
    ///
    /// QR matrix generation is delegated to the `qrcode` crate in the adapter
    /// layer (`ars-dom`). The core spec defines the rendering contract only.
    pub fn new(modules: Vec<Vec<bool>>) -> Self {
        let size = modules.len();
        Self { modules, size }
    }

    /// Get the module value at (row, col).
    pub fn get(&self, row: usize, col: usize) -> bool {
        debug_assert!(row < self.size && col < self.size, "QrMatrix::get() out of bounds: ({row}, {col}) for size {}", self.size);
        self.modules.get(row).and_then(|r| r.get(col)).copied().unwrap_or(false)
    }
}
```

## 1. API

### 1.1 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The data to encode.
    pub value: String,
    /// Error correction level.
    pub error_correction: QrErrorCorrection,
    /// Size of each module in pixels (for rendering).
    pub module_size: f64,
    /// Quiet zone (border) in modules. Standard is 4.
    pub quiet_zone: usize,
    /// Foreground color (dark modules).
    pub foreground: String,
    /// Background color (light modules).
    pub background: String,
    /// Optional image overlay (e.g., logo) in the center.
    pub overlay_src: Option<String>,
    /// Overlay size as a fraction of the QR code size [0.0, 0.5].
    pub overlay_size: f64,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: String::new(),
            error_correction: QrErrorCorrection::Medium,
            module_size: 4.0,
            quiet_zone: 4,
            foreground: "#000000".into(),
            background: "#ffffff".into(),
            overlay_src: None,
            overlay_size: 0.2,
        }
    }
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "qr-code"]
pub enum Part {
    Root,
    Frame,
    Pattern,
    Overlay,
    DownloadTrigger,
}

pub struct Api<'a> {
    props: &'a Props,
    matrix: Option<QrMatrix>,
    id: String,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    pub fn new(props: &'a Props, env: &Env, messages: &Messages) -> Self {
        let matrix = QrMatrix::generate(&props.value, props.error_correction);
        let id = if props.id.is_empty() { generate_id() } else { props.id.clone() };
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, matrix, id, locale, messages }
    }

    pub fn matrix(&self) -> Option<&QrMatrix> {
        self.matrix.as_ref()
    }

    /// Total pixel size of the rendered QR code (including quiet zone).
    pub fn pixel_size(&self) -> f64 {
        if let Some(ref m) = self.matrix {
            (m.size + self.props.quiet_zone * 2) as f64 * self.props.module_size
        } else {
            0.0
        }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.id);
        attrs.set(HtmlAttr::Role, "img");
        let label = if self.props.value.starts_with("http://") || self.props.value.starts_with("https://") {
            (self.messages.link_label)(&self.props.value, &self.locale)
        } else {
            (self.messages.label)(&self.props.value, &self.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        let size = self.pixel_size();
        attrs.set_style(CssProperty::Width, format!("{}px", size));
        attrs.set_style(CssProperty::Height, format!("{}px", size));
        attrs
    }

    pub fn frame_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Frame.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn pattern_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Pattern.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(ref src) = self.props.overlay_src {
            attrs.set(HtmlAttr::Src, src);
        }
        attrs
    }

    pub fn download_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DownloadTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.download_label)(&self.locale));
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Frame => self.frame_attrs(),
            Part::Pattern => self.pattern_attrs(),
            Part::Overlay => self.overlay_attrs(),
            Part::DownloadTrigger => self.download_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
QrCode
├── Root             (required — role="img", container for the QR code)
├── Frame            (optional — decorative frame around the code)
├── Pattern          (required — the actual QR module grid, rendered as SVG or Canvas)
├── Overlay          (optional — centered image/logo)
└── DownloadTrigger  (optional — button to download QR code as image)
```

| Part            | Element    | Key Attributes                          |
| --------------- | ---------- | --------------------------------------- |
| Root            | `<div>`    | `role="img"`, `aria-label`, sized to QR |
| Frame           | `<div>`    | decorative border                       |
| Pattern         | `<svg>`    | QR module grid rendering                |
| Overlay         | `<img>`    | `src` from `overlay_src` prop           |
| DownloadTrigger | `<button>` | `aria-label` from messages              |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part | Role  | Properties                                                     |
| ---- | ----- | -------------------------------------------------------------- |
| Root | `img` | `aria-label` — `"QR code: {value}"` (or link variant for URLs) |

The QR code is a purely visual element. The `aria-label` conveys the encoded data.
If the data is a URL, the label indicates that (e.g., `"QR code linking to {url}"`).

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Root aria-label template (default: "QR code: {value}").
    pub label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Label when value is a URL (default: "QR code linking to {url}").
    pub link_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Label for the download trigger button (default: "Download QR code").
    pub download_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::new(|value, _locale| format!("QR code: {value}")),
            link_label: MessageFn::new(|url, _locale| format!("QR code linking to {url}")),
            download_label: MessageFn::static_str("Download QR code"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                      | Default (en-US)              | Purpose                   |
| ------------------------ | ---------------------------- | ------------------------- |
| `qr_code.label`          | `"QR code: {value}"`         | Root aria-label template  |
| `qr_code.link_label`     | `"QR code linking to {url}"` | Label when value is a URL |
| `qr_code.download_label` | `"Download QR code"`         | Download trigger label    |

The `root_attrs()` method formats `aria-label` from `messages.label` (or `messages.link_label` when the value is a URL), replacing `{value}` / `{url}` with the encoded data.

- **RTL**: QR codes are direction-agnostic (fixed matrix). No RTL adjustments needed for the pattern itself. Surrounding layout elements obey the document direction.

## 5. Library Parity

> Compared against: Ark UI (`QrCode`).

### 5.1 Props

| Feature                     | ars-ui                      | Ark UI                   | Notes                                   |
| --------------------------- | --------------------------- | ------------------------ | --------------------------------------- |
| `value` / `defaultValue`    | `value`                     | `value` / `defaultValue` | ars-ui is always controlled (stateless) |
| `errorCorrection`           | `error_correction`          | `encoding`               | Equivalent (different grouping)         |
| `moduleSize`                | `module_size`               | `pixelSize`              | Equivalent (different naming)           |
| `foreground` / `background` | `foreground` / `background` | --                       | ars-ui has color props                  |
| `overlaySrc`                | `overlay_src`               | --                       | ars-ui has overlay support              |
| `quietZone`                 | `quiet_zone`                | --                       | ars-ui has quiet zone                   |

**Gaps:** None.

### 5.2 Anatomy

| Part            | ars-ui            | Ark UI            | Notes      |
| --------------- | ----------------- | ----------------- | ---------- |
| Root            | `Root`            | `Root`            | Equivalent |
| Frame           | `Frame`           | `Frame`           | Equivalent |
| Pattern         | `Pattern`         | `Pattern`         | Equivalent |
| Overlay         | `Overlay`         | `Overlay`         | Equivalent |
| DownloadTrigger | `DownloadTrigger` | `DownloadTrigger` | Equivalent |

**Gaps:** None.

### 5.3 Events

| Callback     | ars-ui                 | Ark UI          | Notes                                |
| ------------ | ---------------------- | --------------- | ------------------------------------ |
| Value change | (stateless, re-render) | `onValueChange` | ars-ui is stateless; value is a prop |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui                  | Ark UI |
| ----------------------- | ----------------------- | ------ |
| QR matrix generation    | Yes (via adapter crate) | Yes    |
| Error correction levels | Yes (4 levels)          | Yes    |
| Center overlay/logo     | Yes                     | Yes    |
| Download as image       | Yes (`DownloadTrigger`) | Yes    |
| Custom colors           | Yes                     | No     |
| Quiet zone control      | Yes                     | No     |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui is fully stateless (QR code is a pure function of `value`). Ark UI supports controlled/uncontrolled value.
- **Recommended additions:** None.
