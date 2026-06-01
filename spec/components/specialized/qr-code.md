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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub const fn new(modules: Vec<Vec<bool>>) -> Self {
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

/// Default pixel size of a single QR module; also the fallback when a caller
/// supplies a non-finite or non-positive `module_size`.
const DEFAULT_MODULE_SIZE: f64 = 4.0;

/// Whether `value` is an http(s) URL, comparing the scheme case-insensitively
/// (URL schemes are case-insensitive per RFC 3986).
fn is_url(value: &str) -> bool {
    value.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || value.get(..8).is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
}

pub struct Api<'a> {
    props: &'a Props,
    matrix: Option<QrMatrix>,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    /// QR matrix generation is adapter-owned: the agnostic core defines only the
    /// rendering contract, so the caller (the framework adapter) encodes
    /// `props.value` and injects the resulting `matrix`. It is `None` until the
    /// adapter has produced it; the core renders whatever it is given.
    pub fn new(props: &'a Props, matrix: Option<QrMatrix>, env: &Env, messages: &Messages) -> Self {
        let locale = env.locale.clone();
        let messages = messages.clone();
        Self { props, matrix, locale, messages }
    }

    pub const fn matrix(&self) -> Option<&QrMatrix> {
        self.matrix.as_ref()
    }

    /// The module size used for rendering, normalized to a positive, finite
    /// value. A non-finite or non-positive `module_size` prop falls back to the
    /// default so `width`/`height` can never become `-44px` or `NaNpx`.
    fn effective_module_size(&self) -> f64 {
        if self.props.module_size.is_finite() && self.props.module_size > 0.0 {
            self.props.module_size
        } else {
            DEFAULT_MODULE_SIZE
        }
    }

    /// The accessible name for the QR image, using the link-specific template
    /// when the value is an http(s) URL.
    fn aria_label(&self) -> String {
        if is_url(&self.props.value) {
            (self.messages.link_label)(&self.props.value, &self.locale)
        } else {
            (self.messages.label)(&self.props.value, &self.locale)
        }
    }

    /// Total pixel size of the rendered QR code (including quiet zone).
    /// Returns `0.0` when no matrix has been supplied.
    pub fn pixel_size(&self) -> f64 {
        match &self.matrix {
            Some(matrix) => (matrix.size + self.props.quiet_zone * 2) as f64 * self.effective_module_size(),
            None => 0.0,
        }
    }

    /// The root is a neutral sized container. The image semantics live on
    /// `pattern_attrs` so the optional interactive `DownloadTrigger` is not
    /// pruned from the accessibility tree by a `role="img"` ancestor.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // The id is adapter-supplied; emit it only when set (no in-core id generation).
        if !self.props.id.is_empty() {
            attrs.set(HtmlAttr::Id, self.props.id.clone());
        }
        let size = self.pixel_size();
        attrs.set_style(CssProperty::Width, format!("{size}px"));
        attrs.set_style(CssProperty::Height, format!("{size}px"));
        attrs
    }

    pub fn frame_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Frame.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The pattern (SVG) is the QR code's accessible image, carrying `role="img"`
    /// and the URL-aware `aria-label`.
    pub fn pattern_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Pattern.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "img");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), self.aria_label());
        attrs
    }

    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Decorative overlay: the pattern already exposes the content via its
        // aria-label, so the <img> is marked presentational with an empty alt.
        if let Some(src) = &self.props.overlay_src {
            attrs.set(HtmlAttr::Src, src.clone());
            attrs.set(HtmlAttr::Alt, "");
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
├── Root             (required — neutral sized container for the QR code)
├── Frame            (optional — decorative frame around the code)
├── Pattern          (required — the QR module grid, role="img", rendered as SVG or Canvas)
├── Overlay          (optional — centered decorative image/logo)
└── DownloadTrigger  (optional — button to download QR code as image)
```

| Part            | Element    | Key Attributes                             |
| --------------- | ---------- | ------------------------------------------ |
| Root            | `<div>`    | sized to QR (no role — neutral container)  |
| Frame           | `<div>`    | decorative border                          |
| Pattern         | `<svg>`    | `role="img"`, `aria-label`, QR module grid |
| Overlay         | `<img>`    | `src` from `overlay_src` prop, `alt=""`    |
| DownloadTrigger | `<button>` | `aria-label` from messages                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Role  | Properties                                                     |
| ------- | ----- | -------------------------------------------------------------- |
| Pattern | `img` | `aria-label` — `"QR code: {value}"` (or link variant for URLs) |

The QR code is a purely visual element. The accessible image semantics live on
the `Pattern` (the SVG that visually is the code), not on `Root`: `role="img"`
is a leaf role that prunes its descendants from the accessibility tree, so
putting it on `Root` would hide the interactive `DownloadTrigger`. The
`aria-label` conveys the encoded data; if the data is a URL (scheme matched
case-insensitively), the label indicates that (e.g., `"QR code linking to {url}"`).
The `Overlay` image is decorative (`alt=""`) since the `Pattern` already conveys
the content.

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

The `pattern_attrs()` method formats `aria-label` from `messages.label` (or `messages.link_label` when the value is a URL), replacing `{value}` / `{url}` with the encoded data.

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
